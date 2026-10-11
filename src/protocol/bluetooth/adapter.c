/*
 * Bluetooth cards (adapters) of the computer.
 *
 * The cards are read from BlueZ over D-Bus, using the sd-bus API
 * of libsystemd. BlueZ publishes every object it knows (cards,
 * devices, GATT services) through the standard ObjectManager
 * interface. One call returns all of them:
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
 * An object that has the org.bluez.Adapter1 interface is a card.
 * The functions below walk through the reply level by level.
 */

#include "protocol/bluetooth/adapter.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <systemd/sd-bus.h>

#define BLUEZ_SERVICE "org.bluez"
#define BLUEZ_ADAPTER_INTERFACE "org.bluez.Adapter1"

#define OBJECT_MANAGER_PATH "/"
#define OBJECT_MANAGER_INTERFACE "org.freedesktop.DBus.ObjectManager"

/*
 * Reads a property value that is a string, such as "Address".
 *
 * The message is positioned at the variant that holds the value.
 *
 * Returns 0 on success or a negative errno-style value.
 */
static int read_string_property(
    sd_bus_message *message,
    char *destination,
    size_t destination_size
)
{
    const char *value = NULL;

    int result = sd_bus_message_read(message, "v", "s", &value);

    if (result < 0) {
        return result;
    }

    snprintf(destination, destination_size, "%s", value);

    return 0;
}

/*
 * Reads a property value that is a boolean, such as "Powered".
 *
 * Returns 0 on success or a negative errno-style value.
 */
static int read_bool_property(
    sd_bus_message *message,
    bool *destination
)
{
    /*
     * sd-bus stores a D-Bus boolean in an int.
     */
    int value = 0;

    int result = sd_bus_message_read(message, "v", "b", &value);

    if (result < 0) {
        return result;
    }

    *destination = value != 0;

    return 0;
}

/*
 * Reads the properties of one card.
 *
 * The message is positioned at the array of properties, a{sv}.
 * Properties that the program does not use are skipped.
 *
 * Returns 0 on success or a negative errno-style value.
 */
static int parse_adapter_properties(
    sd_bus_message *message,
    bt_adapter_info_t *adapter
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

        const char *property = NULL;

        result = sd_bus_message_read(message, "s", &property);

        if (result < 0) {
            return result;
        }

        if (strcmp(property, "Address") == 0) {
            result = read_string_property(
                message,
                adapter->address,
                sizeof(adapter->address)
            );
        } else if (strcmp(property, "Alias") == 0) {
            result = read_string_property(
                message,
                adapter->alias,
                sizeof(adapter->alias)
            );
        } else if (strcmp(property, "Powered") == 0) {
            result = read_bool_property(
                message,
                &adapter->powered
            );
        } else if (strcmp(property, "Discovering") == 0) {
            result = read_bool_property(
                message,
                &adapter->discovering
            );
        } else {
            /*
             * A property we are not interested in.
             */
            result = sd_bus_message_skip(message, "v");
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
 * Reads the interfaces of one BlueZ object and stores the object
 * in the adapters array when it is a card.
 *
 * The message is positioned at the array of interfaces,
 * a{sa{sv}}. path is the D-Bus path of the object.
 *
 * Returns 0 on success or a negative errno-style value
 * (-ENOSPC when the adapters array is full).
 */
static int parse_object_interfaces(
    sd_bus_message *message,
    const char *path,
    bt_adapter_info_t *adapters,
    size_t max_adapters,
    size_t *adapter_count
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

        if (strcmp(interface, BLUEZ_ADAPTER_INTERFACE) == 0) {

            if (*adapter_count >= max_adapters) {
                return -ENOSPC;
            }

            bt_adapter_info_t *adapter = &adapters[*adapter_count];

            memset(adapter, 0, sizeof(*adapter));

            /*
             * The system name of the card is the last element
             * of the object path: "/org/bluez/hci0" -> "hci0".
             */
            const char *name = strrchr(path, '/');

            snprintf(
                adapter->name,
                sizeof(adapter->name),
                "%s",
                name != NULL ? name + 1 : path
            );

            result = parse_adapter_properties(message, adapter);

            (*adapter_count)++;

        } else {
            /*
             * Not a card interface: skip its properties.
             */
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
 * Reads the whole GetManagedObjects reply.
 *
 * Returns 0 on success or a negative errno-style value.
 */
static int parse_managed_objects(
    sd_bus_message *reply,
    bt_adapter_info_t *adapters,
    size_t max_adapters,
    size_t *adapter_count
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

        result = parse_object_interfaces(
            reply,
            path,
            adapters,
            max_adapters,
            adapter_count
        );

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

/*
 * Orders cards by their system name. Used with qsort().
 */
static int compare_adapters(const void *left, const void *right)
{
    const bt_adapter_info_t *left_adapter = left;
    const bt_adapter_info_t *right_adapter = right;

    return strcmp(left_adapter->name, right_adapter->name);
}

int bt_find_adapters(
    bt_adapter_info_t *adapters,
    size_t max_adapters,
    size_t *adapter_count
)
{
    sd_bus *bus = NULL;
    sd_bus_message *reply = NULL;
    sd_bus_error error = SD_BUS_ERROR_NULL;

    if (adapters == NULL ||
        adapter_count == NULL ||
        max_adapters == 0) {

        errno = EINVAL;
        return -1;
    }

    *adapter_count = 0;

    /*
     * BlueZ is a system service, so it lives on the system bus.
     */
    int result = sd_bus_open_system(&bus);

    if (result >= 0) {
        /*
         * Ask BlueZ for all of its objects.
         */
        result = sd_bus_call_method(
            bus,
            BLUEZ_SERVICE,
            OBJECT_MANAGER_PATH,
            OBJECT_MANAGER_INTERFACE,
            "GetManagedObjects",
            &error,
            &reply,
            ""
        );
    }

    if (result >= 0) {
        result = parse_managed_objects(
            reply,
            adapters,
            max_adapters,
            adapter_count
        );
    }

    /*
     * Release the D-Bus resources. All three functions accept
     * NULL, so this is safe after a failure at any step.
     */
    sd_bus_error_free(&error);
    sd_bus_message_unref(reply);
    sd_bus_unref(bus);

    if (result < 0) {
        *adapter_count = 0;
        errno = -result;
        return -1;
    }

    /*
     * BlueZ returns the objects in no particular order.
     */
    qsort(
        adapters,
        *adapter_count,
        sizeof(adapters[0]),
        compare_adapters
    );

    return 0;
}

int bt_get_adapter(
    const char *name,
    bt_adapter_info_t *adapter
)
{
    bt_adapter_info_t adapters[MAX_BT_ADAPTERS];
    size_t adapter_count = 0;

    if (name == NULL || adapter == NULL) {
        errno = EINVAL;
        return -1;
    }

    if (bt_find_adapters(
            adapters,
            MAX_BT_ADAPTERS,
            &adapter_count) != 0) {
        return -1;
    }

    for (size_t i = 0; i < adapter_count; i++) {

        if (strcmp(adapters[i].name, name) == 0) {
            *adapter = adapters[i];
            return 0;
        }
    }

    errno = ENOENT;
    return -1;
}
