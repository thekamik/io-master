/*
 * Bluetooth diagnostic devices (sniffers), such as Ubertooth One.
 *
 * This file is a placeholder. It defines where the detection of
 * diagnostic devices belongs, so that the "cards" command of the
 * Bluetooth shell already has a place to show them. The detection
 * itself has to be written and tested with the real hardware.
 */

// TODO (stage 6): Detect Ubertooth One. It is a plain USB device,
//                 so it can be found the same way as the serial
//                 devices in src/discovery/devices.c: enumerate
//                 the "usb" subsystem with sd-device and match
//                 the USB ids of the device (vendor 1d50,
//                 product 6002 - verify them with the hardware).
//                 Fill model from ID_MODEL and serial from
//                 ID_SERIAL_SHORT.
// TODO (stage 6): Use the device. Planned as a separate "sniff"
//                 command that runs the Ubertooth tools
//                 (ubertooth-btle) and shows the traffic between
//                 two other devices. It complements "monitor",
//                 which shows only the traffic of this computer.
// TODO (stage 6): Store the chosen sniffer in its own project
//                 setting. "select CARD" stays reserved for the
//                 Bluetooth cards handled by BlueZ.

#include "protocol/bluetooth/sniffer.h"

#include <errno.h>

int bt_find_sniffers(
    bt_sniffer_info_t *sniffers,
    size_t max_sniffers,
    size_t *sniffer_count
)
{
    if (sniffers == NULL ||
        sniffer_count == NULL ||
        max_sniffers == 0) {

        errno = EINVAL;
        return -1;
    }

    /*
     * No detection yet: report that nothing was found.
     */
    *sniffer_count = 0;

    return 0;
}
