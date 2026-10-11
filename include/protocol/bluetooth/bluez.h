/*
 * Shared access to BlueZ, the Linux Bluetooth service.
 *
 * BlueZ is used over D-Bus with the sd-bus API of libsystemd.
 * It publishes every object it knows (cards, devices, GATT
 * services) through the standard ObjectManager interface, and
 * this file holds the code that walks through that list. It is
 * used by adapter.c (cards) and device.c (devices).
 *
 * This is an internal header of the Bluetooth protocol. Its
 * functions return 0 or a positive value on success and
 * a negative errno-style value on error, like sd-bus itself.
 */

#ifndef BLUETOOTH_BLUEZ_H
#define BLUETOOTH_BLUEZ_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include <systemd/sd-bus.h>

/*
 * D-Bus names of BlueZ.
 */
#define BLUEZ_SERVICE "org.bluez"
#define BLUEZ_ADAPTER_INTERFACE "org.bluez.Adapter1"
#define BLUEZ_DEVICE_INTERFACE "org.bluez.Device1"

/*
 * Every BlueZ object path starts with this prefix:
 * "/org/bluez/hci0", "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF".
 */
#define BLUEZ_PATH_PREFIX "/org/bluez/"

/*
 * Size of a buffer that holds the D-Bus path of a BlueZ card
 * or device.
 */
#define BLUEZ_PATH_MAX 64

/*
 * Builds the D-Bus path of a card: "hci0" -> "/org/bluez/hci0".
 *
 * Returns 0 on success or -EINVAL when the name is empty,
 * too long or contains characters that a path cannot have.
 */
int bluez_adapter_path(
    const char *adapter,
    char *buffer,
    size_t size
);

/*
 * Builds the D-Bus path of a device of a card:
 *
 *     "hci0", "aa:bb:cc:dd:ee:ff"
 *         -> "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"
 *
 * Returns 0 on success or -EINVAL when the card name or the
 * address is not valid.
 */
int bluez_device_path(
    const char *adapter,
    const char *address,
    char *buffer,
    size_t size
);

/*
 * Describes what the caller wants to read from the BlueZ objects.
 *
 * bluez_visit_objects() goes through all objects and calls these
 * functions for the objects that have the given interface.
 */
typedef struct {
    /*
     * Interface that selects the objects, for example
     * BLUEZ_ADAPTER_INTERFACE for the cards.
     */
    const char *interface;

    /*
     * Called once for every object that has the interface,
     * before its properties are read. path is the D-Bus path
     * of the object.
     *
     * Returns:
     *    1  - read the properties of this object
     *    0  - ignore this object
     *   <0  - error, stop the whole walk
     */
    int (*begin_object)(const char *path, void *userdata);

    /*
     * Called for every property of an accepted object. value is
     * positioned at the variant that holds the property value;
     * read it with the bluez_read_*() functions below.
     *
     * Returns:
     *    1  - the value was read
     *    0  - the property is not interesting; the value was
     *         left untouched and the caller skips it
     *   <0  - error, stop the whole walk
     */
    int (*property)(
        const char *name,
        sd_bus_message *value,
        void *userdata
    );
} bluez_visitor_t;

/*
 * Asks BlueZ for all of its objects and reports the ones that
 * have visitor->interface to the visitor.
 *
 * bus is an open connection to the system bus. userdata is
 * passed to the visitor functions unchanged.
 *
 * Returns 0 on success or a negative errno-style value.
 */
int bluez_visit_objects(
    sd_bus *bus,
    const bluez_visitor_t *visitor,
    void *userdata
);

/*
 * Readers of property values. Each of them reads the variant
 * the message is positioned at and returns 1 on success
 * (the value a visitor's property function has to return)
 * or a negative errno-style value.
 */

/* Text property, such as "Address". */
int bluez_read_string(
    sd_bus_message *value,
    char *destination,
    size_t destination_size
);

/* Boolean property, such as "Powered". */
int bluez_read_bool(sd_bus_message *value, bool *destination);

/* Signed 16-bit property, such as "RSSI". */
int bluez_read_int16(sd_bus_message *value, int16_t *destination);

/*
 * Skips a value whose content does not matter. Used for
 * properties where only the presence is interesting.
 */
int bluez_skip_value(sd_bus_message *value);

#endif
