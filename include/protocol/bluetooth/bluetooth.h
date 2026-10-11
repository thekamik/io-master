/*
 * Bluetooth protocol: tools for testing Bluetooth devices.
 *
 * The protocol is built in stages. See the TODO list at the top
 * of src/protocol/bluetooth/bluetooth.c for what is planned.
 */

#ifndef BLUETOOTH_H
#define BLUETOOTH_H

#include "protocol/protocol.h"

/*
 * Bluetooth protocol, registered in src/protocol/protocol.c.
 *
 * The settings of a Bluetooth project (the selected card and
 * the target device) are stored in its settings file.
 */
extern const protocol_t bluetooth_protocol;

#endif
