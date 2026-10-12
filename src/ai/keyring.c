/*
 * Storage of API keys in the keyring of the system.
 *
 * The keyring is reached with libsecret, the client library of
 * the Secret Service. An entry of the keyring consists of
 *
 *     a label        text shown in keyring managers
 *     attributes     name/value pairs used to find the entry
 *     the secret     here: the API key
 *
 * The entries of this program have two attributes:
 *
 *     application = "io-master"
 *     key_name    = the name of the key, for example "anthropic"
 *
 * so a key can also be inspected or stored outside of the
 * program:
 *
 *     secret-tool lookup application io-master key_name anthropic
 */

#include "ai/keyring.h"

#include <errno.h>
#include <stdio.h>
#include <string.h>

#include <libsecret/secret.h>

/*
 * Value of the "application" attribute of every entry.
 */
#define KEYRING_APPLICATION "io-master"

/*
 * Describes the entries of this program to libsecret: the name
 * of the schema and the attributes that an entry has.
 */
static const SecretSchema *key_schema(void)
{
    static const SecretSchema schema = {
        "io.master.ApiKey",

        /*
         * Find an entry by the two attributes alone, without
         * also requiring the schema name. This way a key stored
         * with another tool, such as secret-tool, is found too.
         */
        SECRET_SCHEMA_DONT_MATCH_NAME,
        {
            { "application", SECRET_SCHEMA_ATTRIBUTE_STRING },
            { "key_name", SECRET_SCHEMA_ATTRIBUTE_STRING },
            { NULL, 0 },
        },
        /*
         * The remaining fields are reserved by libsecret.
         */
        0, NULL, NULL, NULL, NULL, NULL, NULL, NULL
    };

    return &schema;
}

/*
 * Copies the message of a libsecret error to error_text and
 * releases the error.
 */
static void take_error(GError *error, char *error_text)
{
    if (error_text != NULL) {
        snprintf(
            error_text,
            KEYRING_ERROR_TEXT_MAX,
            "%s",
            error != NULL ? error->message : "unknown keyring error"
        );
    }

    if (error != NULL) {
        g_error_free(error);
    }
}

int keyring_load(
    const char *name,
    char *key,
    size_t size,
    char *error_text
)
{
    GError *error = NULL;

    if (error_text != NULL) {
        error_text[0] = '\0';
    }

    if (name == NULL || *name == '\0' || key == NULL || size == 0) {
        errno = EINVAL;
        return -1;
    }

    key[0] = '\0';

    /*
     * Ask the keyring for the entry with our two attributes.
     * The result is NULL both when there is no such entry and
     * when the lookup failed; error tells the two apart.
     */
    gchar *stored = secret_password_lookup_sync(
        key_schema(),
        NULL,
        &error,
        "application", KEYRING_APPLICATION,
        "key_name", name,
        NULL
    );

    if (error != NULL) {
        take_error(error, error_text);
        errno = EIO;
        return -1;
    }

    if (stored == NULL) {
        errno = ENOENT;
        return -1;
    }

    int result = 0;

    if (strlen(stored) >= size) {
        errno = ENOSPC;
        result = -1;
    } else {
        memcpy(key, stored, strlen(stored) + 1);
    }

    /*
     * secret_password_free() wipes the copy made by libsecret
     * before releasing it. It does not change errno.
     */
    secret_password_free(stored);

    return result;
}

int keyring_store(
    const char *name,
    const char *key,
    char *error_text
)
{
    GError *error = NULL;
    char label[128];

    if (error_text != NULL) {
        error_text[0] = '\0';
    }

    if (name == NULL || *name == '\0' || key == NULL || *key == '\0') {
        errno = EINVAL;
        return -1;
    }

    /*
     * Text shown for the entry in keyring managers such as
     * "Passwords and Keys".
     */
    snprintf(label, sizeof(label), "IO Master API key: %s", name);

    /*
     * SECRET_COLLECTION_DEFAULT is the keyring that the system
     * unlocks at login. An entry with the same attributes is
     * replaced.
     */
    secret_password_store_sync(
        key_schema(),
        SECRET_COLLECTION_DEFAULT,
        label,
        key,
        NULL,
        &error,
        "application", KEYRING_APPLICATION,
        "key_name", name,
        NULL
    );

    if (error != NULL) {
        take_error(error, error_text);
        errno = EIO;
        return -1;
    }

    return 0;
}
