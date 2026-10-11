/*
 * Bluetooth devices seen by a Bluetooth card, and the discovery
 * (scan) that finds them.
 *
 * The information comes from BlueZ, the Linux Bluetooth service.
 */

#ifndef BLUETOOTH_DEVICE_H
#define BLUETOOTH_DEVICE_H

#include <stdbool.h>
#include <stddef.h>

#include "protocol/bluetooth/adapter.h"

/*
 * Sizes of the buffers in bt_device_info_t and the default size
 * of the device array used by the callers.
 */
#define BT_DEVICE_NAME_MAX 64
#define BT_ADDRESS_TYPE_MAX 8    /* "public" or "random" */
#define MAX_BT_DEVICES 256

/*
 * Description of one Bluetooth device, as BlueZ knows it.
 */
typedef struct {
    /*
     * Bluetooth address, "AA:BB:CC:DD:EE:FF". It identifies
     * the device: no two devices of a card share an address.
     */
    char address[BT_ADDRESS_MAX];

    /*
     * Name announced by the device. Empty when the device
     * does not announce one.
     */
    char name[BT_DEVICE_NAME_MAX];

    /*
     * "public" or "random". Low Energy devices often use
     * random addresses that change over time.
     */
    char address_type[BT_ADDRESS_TYPE_MAX];

    /*
     * true for a classic (BR/EDR) device, false for a Low
     * Energy one. BlueZ does not report this directly: a device
     * is treated as classic when it has a "Class" property,
     * which only classic and dual-mode devices have.
     */
    bool classic;

    /*
     * Signal strength in dBm (closer to 0 is stronger). BlueZ
     * reports it only while the device is being received during
     * a discovery, so has_rssi tells whether rssi is valid.
     */
    bool has_rssi;
    int rssi;

    bool paired;     /* a pairing with this computer exists */
    bool connected;  /* currently connected to this computer */
} bt_device_info_t;

/*
 * Reads the current state of one device of a card, for example
 * to learn whether it is connected or paired right now.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when BlueZ does not know
 *        a device with that address, EINVAL when the card name
 *        or the address is malformed)
 */
int bt_device_get(
    const char *adapter,
    const char *address,
    bt_device_info_t *device
);

/*
 * A running discovery. Created by bt_discovery_start() and
 * released by bt_discovery_stop().
 */
typedef struct bt_discovery bt_discovery_t;

/*
 * Starts a discovery (scan) on the given card, for example
 * "hci0". The card keeps looking for devices until
 * bt_discovery_stop() is called.
 *
 * Returns:
 *   0  - success, *discovery holds the running discovery
 *  -1  - error (errno is set):
 *          ENODEV    there is no such card
 *          ENETDOWN  the card is powered off
 *          other     BlueZ could not be reached or refused
 */
int bt_discovery_start(
    const char *adapter,
    bt_discovery_t **discovery
);

/*
 * Reads the devices that BlueZ currently knows for the card
 * of the discovery.
 *
 * The list also contains devices remembered from the past, such
 * as paired ones, that are not in range now. Those have neither
 * has_rssi nor connected set.
 *
 * devices is an array of max_devices elements provided by the
 * caller. On success *device_count is the number of elements
 * that were filled. Devices that do not fit are left out.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int bt_discovery_read(
    bt_discovery_t *discovery,
    bt_device_info_t *devices,
    size_t max_devices,
    size_t *device_count
);

/*
 * Stops the discovery and releases it. NULL is accepted.
 */
void bt_discovery_stop(bt_discovery_t *discovery);

#endif
