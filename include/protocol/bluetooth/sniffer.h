/*
 * Bluetooth diagnostic devices (sniffers), such as Ubertooth One.
 *
 * A sniffer is not a Bluetooth card. It is not handled by BlueZ,
 * it has no hciX name and it cannot pair or connect. It is a radio
 * receiver that passively listens to the traffic between other
 * devices, so it is detected and used separately from the cards
 * in adapter.h.
 *
 * Only the detection interface exists so far. See the TODO notes
 * in src/protocol/bluetooth/sniffer.c.
 */

#ifndef BLUETOOTH_SNIFFER_H
#define BLUETOOTH_SNIFFER_H

#include <stddef.h>

/*
 * Sizes of the buffers in bt_sniffer_info_t and the default size
 * of the sniffer array used by the callers.
 */
#define BT_SNIFFER_MODEL_MAX 32
#define BT_SNIFFER_SERIAL_MAX 64
#define MAX_BT_SNIFFERS 8

/*
 * Description of one diagnostic device.
 */
typedef struct {
    char model[BT_SNIFFER_MODEL_MAX];    /* for example "Ubertooth One" */
    char serial[BT_SNIFFER_SERIAL_MAX];  /* USB serial number */
} bt_sniffer_info_t;

/*
 * Finds the Bluetooth diagnostic devices connected to
 * the computer.
 *
 * sniffers is an array of max_sniffers elements provided by the
 * caller. On success *sniffer_count is the number of elements
 * that were filled.
 *
 * Detection is not implemented yet: the function always succeeds
 * and reports no devices.
 *
 * Returns:
 *   0  - success (also when there are no devices)
 *  -1  - error (errno is set)
 */
int bt_find_sniffers(
    bt_sniffer_info_t *sniffers,
    size_t max_sniffers,
    size_t *sniffer_count
);

#endif
