/*
 * Bluetooth cards (adapters) of the computer.
 *
 * A computer can have more than one card, for example a built-in
 * one and a USB dongle. Linux names them hci0, hci1 and so on.
 *
 * The information comes from BlueZ, the Linux Bluetooth service.
 */

#ifndef BLUETOOTH_ADAPTER_H
#define BLUETOOTH_ADAPTER_H

#include <stdbool.h>
#include <stddef.h>

/*
 * Sizes of the buffers in bt_adapter_info_t and the default size
 * of the adapter array used by the callers.
 */
#define BT_ADAPTER_NAME_MAX 16   /* "hci0" */
#define BT_ADDRESS_MAX 18        /* "AA:BB:CC:DD:EE:FF" + '\0' */
#define BT_ALIAS_MAX 64
#define MAX_BT_ADAPTERS 16

/*
 * Description of one Bluetooth card.
 */
typedef struct {
    char name[BT_ADAPTER_NAME_MAX];  /* system name, for example hci0 */
    char address[BT_ADDRESS_MAX];    /* Bluetooth address of the card */
    char alias[BT_ALIAS_MAX];        /* name shown to other devices */
    bool powered;                    /* false when the card is off */
    bool discovering;                /* true while a scan is running */
} bt_adapter_info_t;

/*
 * Finds the Bluetooth cards of the computer.
 *
 * adapters is an array of max_adapters elements provided by the
 * caller. On success *adapter_count is the number of elements
 * that were filled. The cards are sorted by name.
 *
 * Returns:
 *   0  - success (also when there are no cards)
 *  -1  - error (errno is set; ENOSPC when more than max_adapters
 *        cards were found; other values mean that BlueZ could
 *        not be reached)
 */
int bt_find_adapters(
    bt_adapter_info_t *adapters,
    size_t max_adapters,
    size_t *adapter_count
);

/*
 * Reads the card with the given name, for example "hci0".
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when there is no such card)
 */
int bt_get_adapter(
    const char *name,
    bt_adapter_info_t *adapter
);

#endif
