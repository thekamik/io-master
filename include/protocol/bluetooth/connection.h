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
#include <stdint.h>

/*
 * Size of the buffer that receives the explanation of a failed
 * operation.
 */
#define BT_ERROR_TEXT_MAX 160

/*
 * Questions that a pairing may have to ask the user.
 *
 * Which of them is used, if any, is decided by the two devices:
 * a device that pairs without a code asks nothing. The functions
 * are supplied by the caller of bt_device_pair(); the shell
 * implements them with prompts on the terminal.
 *
 * address is the Bluetooth address of the device that is being
 * paired. A function that is NULL counts as a refusal (for the
 * questions) or is skipped (for the display functions).
 */
typedef struct {
    /*
     * The device needs a PIN, as many classic devices do
     * ("0000", "1234"). Stores the PIN typed by the user in pin,
     * a buffer of size characters; a PIN has 1 to 16 characters.
     *
     * Returns 0 on success and -1 when the user gave none.
     */
    int (*request_pin)(const char *address, char *pin, size_t size);

    /*
     * The device shows a number of up to 6 digits that the user
     * has to type here. Stores it in *passkey (0 - 999999).
     *
     * Returns 0 on success and -1 when the user gave none.
     */
    int (*request_passkey)(const char *address, uint32_t *passkey);

    /*
     * Both sides show a number. Asks the user whether the device
     * shows passkey as well.
     *
     * Returns 0 when the user confirms and -1 otherwise.
     */
    int (*confirm_passkey)(const char *address, uint32_t passkey);

    /*
     * A pairing without a code that still needs consent.
     *
     * Returns 0 when the user agrees and -1 otherwise.
     */
    int (*authorize)(const char *address);

    /*
     * The user has to type a code on the device, for example
     * on a Bluetooth keyboard. Show it.
     */
    void (*display_pin)(const char *address, const char *pin);
    void (*display_passkey)(const char *address, uint32_t passkey);
} bt_pairing_prompts_t;

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
 * Pairs with the device. Can take up to two minutes, because
 * the user may have to read or type a code.
 *
 * prompts supplies the questions for a pairing with a code; see
 * bt_pairing_prompts_t. It is used only when the device asks for
 * it. With NULL only devices that need no code can be paired.
 */
int bt_device_pair(
    const char *adapter,
    const char *address,
    const bt_pairing_prompts_t *prompts,
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
