/*
 * Bluetooth cards (adapters) of the computer.
 *
 * The cards are read from BlueZ over D-Bus. A BlueZ object that
 * has the org.bluez.Adapter1 interface is a card. Walking through
 * the list of BlueZ objects is done by bluez.c; this file only
 * says which objects and which properties it wants.
 */

#include "protocol/bluetooth/adapter.h"

#include "protocol/bluetooth/bluez.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * State kept while the BlueZ objects are being read.
 */
typedef struct {
    bt_adapter_info_t *adapters;  /* array provided by the caller */
    size_t max_adapters;          /* size of that array */
    size_t count;                 /* number of cards found so far */
    bt_adapter_info_t *current;   /* card whose properties are read */
} adapter_search_t;

/*
 * bluez_visitor_t.begin_object
 *
 * Called for every card. Reserves the next element of the array
 * and stores the system name of the card in it.
 */
static int begin_adapter(const char *path, void *userdata)
{
    adapter_search_t *search = userdata;

    if (search->count >= search->max_adapters) {
        return -ENOSPC;
    }

    search->current = &search->adapters[search->count];
    search->count++;

    memset(search->current, 0, sizeof(*search->current));

    /*
     * The system name of the card is the last element
     * of the object path: "/org/bluez/hci0" -> "hci0".
     */
    const char *name = strrchr(path, '/');

    snprintf(
        search->current->name,
        sizeof(search->current->name),
        "%s",
        name != NULL ? name + 1 : path
    );

    return 1;
}

/*
 * bluez_visitor_t.property
 *
 * Called for every property of a card. Stores the properties
 * that the program uses and leaves the others to be skipped.
 */
static int read_adapter_property(
    const char *name,
    sd_bus_message *value,
    void *userdata
)
{
    adapter_search_t *search = userdata;
    bt_adapter_info_t *adapter = search->current;

    if (strcmp(name, "Address") == 0) {
        return bluez_read_string(
            value,
            adapter->address,
            sizeof(adapter->address)
        );
    }

    if (strcmp(name, "Alias") == 0) {
        return bluez_read_string(
            value,
            adapter->alias,
            sizeof(adapter->alias)
        );
    }

    if (strcmp(name, "Powered") == 0) {
        return bluez_read_bool(value, &adapter->powered);
    }

    if (strcmp(name, "Discovering") == 0) {
        return bluez_read_bool(value, &adapter->discovering);
    }

    /*
     * A property we are not interested in.
     */
    return 0;
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
    static const bluez_visitor_t visitor = {
        .interface = BLUEZ_ADAPTER_INTERFACE,
        .begin_object = begin_adapter,
        .property = read_adapter_property,
    };

    sd_bus *bus = NULL;

    if (adapters == NULL ||
        adapter_count == NULL ||
        max_adapters == 0) {

        errno = EINVAL;
        return -1;
    }

    *adapter_count = 0;

    adapter_search_t search = {
        .adapters = adapters,
        .max_adapters = max_adapters,
        .count = 0,
        .current = NULL,
    };

    /*
     * BlueZ is a system service, so it lives on the system bus.
     */
    int result = sd_bus_open_system(&bus);

    if (result >= 0) {
        result = bluez_visit_objects(bus, &visitor, &search);
    }

    /*
     * sd_bus_unref() accepts NULL, so this is safe when
     * opening the bus failed.
     */
    sd_bus_unref(bus);

    if (result < 0) {
        errno = -result;
        return -1;
    }

    *adapter_count = search.count;

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
