/*
 * Storage of API keys in the keyring of the system.
 *
 * API keys are never written to the configuration files of the
 * program. They are kept by the Secret Service of the desktop
 * (GNOME Keyring, KWallet and compatible services), which stores
 * them encrypted and unlocks them when the user logs in.
 *
 * A key is identified by a name, for example "anthropic". The
 * settings of a project hold only that name.
 *
 * What the keyring protects against: reading the key from the
 * disk, and other users of the computer. It does not protect
 * against other programs of the same user while the keyring
 * is unlocked.
 */

#ifndef AI_KEYRING_H
#define AI_KEYRING_H

#include <stddef.h>

/*
 * Size of the buffer that receives the explanation of
 * a failed keyring operation.
 */
#define KEYRING_ERROR_TEXT_MAX 256

/*
 * Reads the API key stored under the given name.
 *
 * key is a buffer of size characters. The caller should wipe it
 * (explicit_bzero) when the key is no longer needed.
 *
 * When the keyring is locked, the system asks the user to
 * unlock it.
 *
 * error_text is a buffer of KEYRING_ERROR_TEXT_MAX characters
 * that receives an explanation on failure. May be NULL.
 *
 * Returns:
 *   0  - success
 *  -1  - error; errno is one of:
 *          EINVAL  empty name
 *          ENOENT  no key is stored under that name
 *          ENOSPC  the stored key does not fit into the buffer
 *          EIO     the keyring could not be reached
 */
int keyring_load(
    const char *name,
    char *key,
    size_t size,
    char *error_text
);

/*
 * Stores an API key under the given name, replacing a key that
 * is already stored there.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is EINVAL or EIO, as above)
 */
int keyring_store(
    const char *name,
    const char *key,
    char *error_text
);

#endif
