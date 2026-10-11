/*
 * Operations on one Bluetooth device: connect, disconnect,
 * pair and unpair.
 *
 * The operations are carried out by BlueZ, the Linux Bluetooth
 * service. BlueZ has to know the device already, which means
 * that the device was found by a scan a short time ago, or that
 * it is paired or connected.
 */

#ifndef BLUETOOTH_CONNECTION_H
#define BLUETOOTH_CONNECTION_H

#include <signal.h>
#include <stddef.h>

/*
 * Size of the buffer that receives the explanation of a failed
 * operation.
 */
#define BT_ERROR_TEXT_MAX 160

/*
 * Common rules of the four operations below
 * ------------------------------------------
 *
 * adapter      Card that performs the operation, for example
 *              "hci0".
 *
 * address      Bluetooth address of the device,
 *              "AA:BB:CC:DD:EE:FF". Letter case is ignored.
 *
 * cancel       Lets the caller abort a running operation. The
 *              operation checks *cancel several times a second
 *              and stops as soon as it is not 0. The shell
 *              passes the flag set by Ctrl+C. May be NULL.
 *
 * error_text   Buffer of BT_ERROR_TEXT_MAX characters that
 *              receives an explanation when the operation
 *              fails, usually the message sent by BlueZ.
 *              May be NULL.
 *
 * Returns:
 *   0  - success
 *  -1  - error; errno is one of:
 *          EINVAL     malformed card name or address
 *          ENOENT     BlueZ does not know the device (or, for
 *                     unpair, the card)
 *          ENETDOWN   the card is powered off
 *          EISCONN    the device is already connected
 *          ENOTCONN   the device is not connected
 *          EEXIST     the device is already paired
 *          EACCES     the device or the user refused the
 *                     pairing, or the authentication failed
 *          ETIMEDOUT  the device did not answer in time
 *          ECANCELED  aborted through cancel
 *          other      see error_text
 */

/*
 * Connects to the device. Can take up to half a minute.
 */
int bt_device_connect(
    const char *adapter,
    const char *address,
    const volatile sig_atomic_t *cancel,
    char *error_text
);

/*
 * Closes the connection to the device.
 */
int bt_device_disconnect(
    const char *adapter,
    const char *address,
    const volatile sig_atomic_t *cancel,
    char *error_text
);

/*
 * Pairs with the device. Can take up to a minute, because the
 * device or the user may have to confirm the pairing.
 */
int bt_device_pair(
    const char *adapter,
    const char *address,
    const volatile sig_atomic_t *cancel,
    char *error_text
);

/*
 * Removes the pairing. BlueZ then forgets the device entirely,
 * so a scan is needed before it can be used again.
 */
int bt_device_unpair(
    const char *adapter,
    const char *address,
    const volatile sig_atomic_t *cancel,
    char *error_text
);

#endif
