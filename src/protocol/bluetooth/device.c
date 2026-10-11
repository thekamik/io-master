/*
 * Bluetooth devices and their discovery (scan).
 *
 * Discovery is controlled through the org.bluez.Adapter1
 * interface of the card:
 *
 *     StartDiscovery   the card starts looking for devices
 *     StopDiscovery    the card stops
 *
 * BlueZ ties a discovery to the D-Bus connection that started
 * it and ends it when that connection is closed. For that reason
 * a bt_discovery_t keeps one connection open for its whole life.
 *
 * Found devices appear in BlueZ as objects with the
 * org.bluez.Device1 interface, placed below the card:
 * "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF". They are read with
 * the object walker from bluez.c.
 */

// TODO: Let the user limit the discovery to Low Energy or to
//       classic devices (Adapter1.SetDiscoveryFilter, "Transport").

#include "protocol/bluetooth/device.h"

#include "protocol/bluetooth/bluez.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * A running discovery.
 */
struct bt_discovery {
    /*
     * Connection to the system bus that owns the discovery.
     */
    sd_bus *bus;

    /*
     * D-Bus path of the card, "/org/bluez/hci0".
     */
    char adapter_path[BLUEZ_PATH_MAX];

    /*
     * Beginning of the D-Bus paths of the devices of that card,
     * "/org/bluez/hci0/dev_". Used to ignore the devices
     * of other cards.
     */
    char device_prefix[BLUEZ_PATH_MAX + 8];
};

/*
 * State kept while the BlueZ objects are being read.
 */
typedef struct {
    /*
     * Which devices are wanted:
     *
     *   exact == false  every device whose D-Bus path starts
     *                   with path (all devices of one card)
     *   exact == true   only the device whose D-Bus path
     *                   is path (one device)
     */
    const char *path;
    bool exact;

    bt_device_info_t *devices;   /* array provided by the caller */
    size_t max_devices;          /* size of that array */
    size_t count;                /* number of devices stored so far */
    bt_device_info_t *current;   /* device whose properties are read */
} device_search_t;

/*
 * bluez_visitor_t.begin_object
 *
 * Called for every device known to BlueZ. Accepts the devices
 * that the search asks for and reserves the next element of the
 * array for each of them.
 */
static int begin_device(const char *path, void *userdata)
{
    device_search_t *search = userdata;

    /*
     * Compare the whole path for one device, or only its
     * beginning for all devices of a card.
     */
    bool wanted = search->exact
        ? strcmp(path, search->path) == 0
        : strncmp(path, search->path, strlen(search->path)) == 0;

    if (!wanted) {
        return 0;
    }

    /*
     * The array is full: leave the remaining devices out.
     */
    if (search->count >= search->max_devices) {
        return 0;
    }

    search->current = &search->devices[search->count];
    search->count++;

    memset(search->current, 0, sizeof(*search->current));

    return 1;
}

/*
 * bluez_visitor_t.property
 *
 * Called for every property of an accepted device.
 */
static int read_device_property(
    const char *name,
    sd_bus_message *value,
    void *userdata
)
{
    device_search_t *search = userdata;
    bt_device_info_t *device = search->current;

    if (strcmp(name, "Address") == 0) {
        return bluez_read_string(
            value,
            device->address,
            sizeof(device->address)
        );
    }

    if (strcmp(name, "Name") == 0) {
        return bluez_read_string(
            value,
            device->name,
            sizeof(device->name)
        );
    }

    if (strcmp(name, "AddressType") == 0) {
        return bluez_read_string(
            value,
            device->address_type,
            sizeof(device->address_type)
        );
    }

    if (strcmp(name, "RSSI") == 0) {
        int16_t rssi = 0;

        int result = bluez_read_int16(value, &rssi);

        if (result > 0) {
            device->has_rssi = true;
            device->rssi = rssi;
        }

        return result;
    }

    if (strcmp(name, "Paired") == 0) {
        return bluez_read_bool(value, &device->paired);
    }

    if (strcmp(name, "Connected") == 0) {
        return bluez_read_bool(value, &device->connected);
    }

    /*
     * Only the presence of "Class" matters: classic and
     * dual-mode devices have it, Low Energy devices do not.
     */
    if (strcmp(name, "Class") == 0) {
        device->classic = true;
        return bluez_skip_value(value);
    }

    /*
     * A property we are not interested in.
     */
    return 0;
}

/*
 * Tells bluez_visit_objects() to report devices to the two
 * functions above.
 */
static const bluez_visitor_t device_visitor = {
    .interface = BLUEZ_DEVICE_INTERFACE,
    .begin_object = begin_device,
    .property = read_device_property,
};

/*
 * Translates a failed StartDiscovery call to an errno value.
 *
 * result is the negative value returned by sd-bus and error
 * holds the D-Bus error sent by BlueZ, when there was one.
 */
static int discovery_error_to_errno(
    int result,
    const sd_bus_error *error
)
{
    /*
     * The card is powered off.
     */
    if (sd_bus_error_has_name(error, "org.bluez.Error.NotReady")) {
        return ENETDOWN;
    }

    /*
     * BlueZ has no object or no Adapter1 interface at the
     * path: there is no card with that name.
     */
    if (sd_bus_error_has_name(
            error,
            "org.freedesktop.DBus.Error.UnknownObject") ||
        sd_bus_error_has_name(
            error,
            "org.freedesktop.DBus.Error.UnknownMethod")) {
        return ENODEV;
    }

    return -result;
}

int bt_discovery_start(
    const char *adapter,
    bt_discovery_t **discovery
)
{
    sd_bus_error error = SD_BUS_ERROR_NULL;

    if (adapter == NULL || *adapter == '\0' || discovery == NULL) {
        errno = EINVAL;
        return -1;
    }

    *discovery = NULL;

    bt_discovery_t *session = calloc(1, sizeof(*session));

    if (session == NULL) {
        errno = ENOMEM;
        return -1;
    }

    /*
     * Build the D-Bus paths of the card and of its devices.
     */
    if (bluez_adapter_path(
            adapter,
            session->adapter_path,
            sizeof(session->adapter_path)) < 0) {

        free(session);
        errno = EINVAL;
        return -1;
    }

    snprintf(
        session->device_prefix,
        sizeof(session->device_prefix),
        "%s/dev_",
        session->adapter_path
    );

    /*
     * Open the connection that will own the discovery
     * and start it.
     */
    int result = sd_bus_open_system(&session->bus);

    if (result >= 0) {
        result = sd_bus_call_method(
            session->bus,
            BLUEZ_SERVICE,
            session->adapter_path,
            BLUEZ_ADAPTER_INTERFACE,
            "StartDiscovery",
            &error,
            NULL,
            ""
        );
    }

    if (result < 0) {
        int error_number = discovery_error_to_errno(result, &error);

        sd_bus_error_free(&error);
        sd_bus_unref(session->bus);
        free(session);

        errno = error_number;
        return -1;
    }

    sd_bus_error_free(&error);

    *discovery = session;

    return 0;
}

int bt_discovery_read(
    bt_discovery_t *discovery,
    bt_device_info_t *devices,
    size_t max_devices,
    size_t *device_count
)
{
    if (discovery == NULL ||
        devices == NULL ||
        device_count == NULL ||
        max_devices == 0) {

        errno = EINVAL;
        return -1;
    }

    *device_count = 0;

    device_search_t search = {
        .path = discovery->device_prefix,
        .exact = false,
        .devices = devices,
        .max_devices = max_devices,
        .count = 0,
        .current = NULL,
    };

    int result = bluez_visit_objects(
        discovery->bus,
        &device_visitor,
        &search
    );

    if (result < 0) {
        errno = -result;
        return -1;
    }

    *device_count = search.count;

    return 0;
}

void bt_discovery_stop(bt_discovery_t *discovery)
{
    if (discovery == NULL) {
        return;
    }

    /*
     * Ask the card to stop. A failure is ignored: closing the
     * connection below ends the discovery in any case.
     */
    sd_bus_call_method(
        discovery->bus,
        BLUEZ_SERVICE,
        discovery->adapter_path,
        BLUEZ_ADAPTER_INTERFACE,
        "StopDiscovery",
        NULL,
        NULL,
        ""
    );

    sd_bus_unref(discovery->bus);
    free(discovery);
}

int bt_device_get(
    const char *adapter,
    const char *address,
    bt_device_info_t *device
)
{
    char path[BLUEZ_PATH_MAX];
    sd_bus *bus = NULL;

    if (device == NULL ||
        bluez_device_path(adapter, address, path, sizeof(path)) < 0) {

        errno = EINVAL;
        return -1;
    }

    /*
     * Search for exactly one device; the array has one element.
     */
    device_search_t search = {
        .path = path,
        .exact = true,
        .devices = device,
        .max_devices = 1,
        .count = 0,
        .current = NULL,
    };

    int result = sd_bus_open_system(&bus);

    if (result >= 0) {
        result = bluez_visit_objects(bus, &device_visitor, &search);
    }

    sd_bus_unref(bus);

    if (result < 0) {
        errno = -result;
        return -1;
    }

    /*
     * BlueZ has no object for this address.
     */
    if (search.count == 0) {
        errno = ENOENT;
        return -1;
    }

    return 0;
}
