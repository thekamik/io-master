/*
 * Shared access to BlueZ, the Linux Bluetooth service.
 *
 * One D-Bus call returns all objects that BlueZ knows:
 *
 *     service    org.bluez
 *     path       /
 *     interface  org.freedesktop.DBus.ObjectManager
 *     method     GetManagedObjects
 *
 * The reply has the D-Bus type a{oa{sa{sv}}}:
 *
 *     array of objects
 *         object path                 "/org/bluez/hci0"
 *         array of interfaces
 *             interface name          "org.bluez.Adapter1"
 *             array of properties
 *                 property name       "Address"
 *                 property value      variant, here a string
 *
 * The three visit_*() functions below walk through the three
 * nested arrays, from the innermost (properties) to the outermost
 * (objects). What is done with the data is decided by the caller
 * through a bluez_visitor_t.
 */

#include "protocol/bluetooth/bluez.h"

#include <ctype.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>

#define OBJECT_MANAGER_PATH "/"
#define OBJECT_MANAGER_INTERFACE "org.freedesktop.DBus.ObjectManager"

int bluez_adapter_path(
    const char *adapter,
    char *buffer,
    size_t size
)
{
    if (adapter == NULL || *adapter == '\0') {
        return -EINVAL;
    }

    /*
     * A D-Bus path element may contain only letters, digits
     * and '_'. Card names such as "hci0" always fit that.
     */
    for (size_t i = 0; adapter[i] != '\0'; i++) {

        if (!isalnum((unsigned char)adapter[i]) &&
            adapter[i] != '_') {
            return -EINVAL;
        }
    }

    int length = snprintf(
        buffer,
        size,
        BLUEZ_PATH_PREFIX "%s",
        adapter
    );

    if (length < 0 || (size_t)length >= size) {
        return -EINVAL;
    }

    return 0;
}

int bluez_device_path(
    const char *adapter,
    const char *address,
    char *buffer,
    size_t size
)
{
    /*
     * "AA:BB:CC:DD:EE:FF" has 17 characters.
     */
    enum { ADDRESS_LENGTH = 17 };

    char adapter_path[BLUEZ_PATH_MAX];
    char element[ADDRESS_LENGTH + 1];

    int result = bluez_adapter_path(
        adapter,
        adapter_path,
        sizeof(adapter_path)
    );

    if (result < 0) {
        return result;
    }

    if (address == NULL || strlen(address) != ADDRESS_LENGTH) {
        return -EINVAL;
    }

    /*
     * BlueZ names the device object after the address, in upper
     * case and with '_' in place of ':'.
     */
    for (size_t i = 0; i < ADDRESS_LENGTH; i++) {

        if (i % 3 == 2) {

            if (address[i] != ':') {
                return -EINVAL;
            }

            element[i] = '_';

        } else if (isxdigit((unsigned char)address[i])) {
            element[i] = (char)toupper((unsigned char)address[i]);
        } else {
            return -EINVAL;
        }
    }

    element[ADDRESS_LENGTH] = '\0';

    int length = snprintf(
        buffer,
        size,
        "%s/dev_%s",
        adapter_path,
        element
    );

    if (length < 0 || (size_t)length >= size) {
        return -EINVAL;
    }

    return 0;
}

int bluez_read_string(
    sd_bus_message *value,
    char *destination,
    size_t destination_size
)
{
    const char *text = NULL;

    int result = sd_bus_message_read(value, "v", "s", &text);

    if (result < 0) {
        return result;
    }

    snprintf(destination, destination_size, "%s", text);

    return 1;
}

int bluez_read_bool(sd_bus_message *value, bool *destination)
{
    /*
     * sd-bus stores a D-Bus boolean in an int.
     */
    int flag = 0;

    int result = sd_bus_message_read(value, "v", "b", &flag);

    if (result < 0) {
        return result;
    }

    *destination = flag != 0;

    return 1;
}

int bluez_read_int16(sd_bus_message *value, int16_t *destination)
{
    int result = sd_bus_message_read(value, "v", "n", destination);

    if (result < 0) {
        return result;
    }

    return 1;
}

int bluez_skip_value(sd_bus_message *value)
{
    int result = sd_bus_message_skip(value, "v");

    if (result < 0) {
        return result;
    }

    return 1;
}

/*
 * Innermost level: the properties of one interface.
 *
 * The message is positioned at the array of properties, a{sv}.
 * Every property is offered to the visitor; the ones it does
 * not read are skipped.
 */
static int visit_properties(
    sd_bus_message *message,
    const bluez_visitor_t *visitor,
    void *userdata
)
{
    int result = sd_bus_message_enter_container(
        message,
        SD_BUS_TYPE_ARRAY,
        "{sv}"
    );

    if (result < 0) {
        return result;
    }

    /*
     * One dictionary entry per property: name and value.
     * enter_container() returns 0 at the end of the array.
     */
    while ((result = sd_bus_message_enter_container(
                message,
                SD_BUS_TYPE_DICT_ENTRY,
                "sv")) > 0) {

        const char *name = NULL;

        result = sd_bus_message_read(message, "s", &name);

        if (result < 0) {
            return result;
        }

        result = visitor->property(name, message, userdata);

        if (result < 0) {
            return result;
        }

        /*
         * 0 means that the visitor left the value in place.
         */
        if (result == 0) {
            result = sd_bus_message_skip(message, "v");

            if (result < 0) {
                return result;
            }
        }

        result = sd_bus_message_exit_container(message);

        if (result < 0) {
            return result;
        }
    }

    if (result < 0) {
        return result;
    }

    return sd_bus_message_exit_container(message);
}

/*
 * Middle level: the interfaces of one object.
 *
 * The message is positioned at the array of interfaces,
 * a{sa{sv}}. Only the interface named by the visitor is read.
 */
static int visit_interfaces(
    sd_bus_message *message,
    const char *path,
    const bluez_visitor_t *visitor,
    void *userdata
)
{
    int result = sd_bus_message_enter_container(
        message,
        SD_BUS_TYPE_ARRAY,
        "{sa{sv}}"
    );

    if (result < 0) {
        return result;
    }

    /*
     * One dictionary entry per interface: its name and
     * its properties.
     */
    while ((result = sd_bus_message_enter_container(
                message,
                SD_BUS_TYPE_DICT_ENTRY,
                "sa{sv}")) > 0) {

        const char *interface = NULL;

        result = sd_bus_message_read(message, "s", &interface);

        if (result < 0) {
            return result;
        }

        /*
         * Ask the visitor whether it wants this object. The
         * question is asked only for the right interface.
         */
        int wanted = 0;

        if (strcmp(interface, visitor->interface) == 0) {
            wanted = visitor->begin_object(path, userdata);

            if (wanted < 0) {
                return wanted;
            }
        }

        if (wanted > 0) {
            result = visit_properties(message, visitor, userdata);
        } else {
            result = sd_bus_message_skip(message, "a{sv}");
        }

        if (result < 0) {
            return result;
        }

        result = sd_bus_message_exit_container(message);

        if (result < 0) {
            return result;
        }
    }

    if (result < 0) {
        return result;
    }

    return sd_bus_message_exit_container(message);
}

/*
 * Outermost level: all objects of the GetManagedObjects reply.
 */
static int visit_objects(
    sd_bus_message *reply,
    const bluez_visitor_t *visitor,
    void *userdata
)
{
    int result = sd_bus_message_enter_container(
        reply,
        SD_BUS_TYPE_ARRAY,
        "{oa{sa{sv}}}"
    );

    if (result < 0) {
        return result;
    }

    /*
     * One dictionary entry per object: its path and
     * its interfaces.
     */
    while ((result = sd_bus_message_enter_container(
                reply,
                SD_BUS_TYPE_DICT_ENTRY,
                "oa{sa{sv}}")) > 0) {

        const char *path = NULL;

        result = sd_bus_message_read(reply, "o", &path);

        if (result < 0) {
            return result;
        }

        result = visit_interfaces(reply, path, visitor, userdata);

        if (result < 0) {
            return result;
        }

        result = sd_bus_message_exit_container(reply);

        if (result < 0) {
            return result;
        }
    }

    if (result < 0) {
        return result;
    }

    return sd_bus_message_exit_container(reply);
}

int bluez_visit_objects(
    sd_bus *bus,
    const bluez_visitor_t *visitor,
    void *userdata
)
{
    sd_bus_message *reply = NULL;
    sd_bus_error error = SD_BUS_ERROR_NULL;

    /*
     * Ask BlueZ for all of its objects.
     */
    int result = sd_bus_call_method(
        bus,
        BLUEZ_SERVICE,
        OBJECT_MANAGER_PATH,
        OBJECT_MANAGER_INTERFACE,
        "GetManagedObjects",
        &error,
        &reply,
        ""
    );

    if (result >= 0) {
        result = visit_objects(reply, visitor, userdata);
    }

    /*
     * Both functions accept an unused error and a NULL message.
     */
    sd_bus_error_free(&error);
    sd_bus_message_unref(reply);

    return result < 0 ? result : 0;
}
