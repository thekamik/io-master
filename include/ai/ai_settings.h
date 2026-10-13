/*
 * The AI configuration of the program.
 *
 * There is one AI configuration for the whole program. It is
 * edited with the "ai-config" command and used by everything
 * that talks to an AI model: the chat of the "ai" protocol now,
 * the device protocols later.
 *
 * The configuration is stored in im_config/ai/ai_config.json,
 * in the directory that holds everything related to AI:
 *
 *     {
 *         "provider": "anthropic",
 *         "model": "claude-opus-5-5",
 *         "key_name": "anthropic",
 *         "max_tokens": 16000,
 *         "system": "",
 *         "refusal_fallback": true,
 *         "base_url": "",
 *         "ca_file": ""
 *     }
 *
 * The file holds no secret. "key_name" is only the name of the
 * entry in the keyring of the system that holds the API key
 * (see ai/keyring.h).
 */

#ifndef AI_SETTINGS_H
#define AI_SETTINGS_H

#include <stdbool.h>

#include "ai/ai.h"

/*
 * Directory inside im_config that holds the files related to AI:
 * the configuration here, and the saved chats (ai/chat_store.h).
 */
#define AI_DIRECTORY_NAME "ai"

/*
 * Path of the configuration file inside im_config.
 */
#define AI_SETTINGS_FILE_NAME AI_DIRECTORY_NAME "/ai_config.json"

/*
 * Where the configuration file was kept by earlier versions of
 * the program. A file found there is moved to the new place.
 */
#define AI_SETTINGS_OLD_FILE_NAME "ai_config.json"

/*
 * Longest name of a keyring entry, with the terminating '\0'.
 */
#define AI_KEY_NAME_MAX 64

/*
 * Limits of the "max_tokens" setting.
 */
#define AI_MAX_TOKENS_MIN 1
#define AI_MAX_TOKENS_MAX 128000

/*
 * The AI configuration, as loaded from the file.
 */
typedef struct {
    /*
     * What the AI module needs to send a request. After
     * ai_settings_load() the api_key is empty; it is filled by
     * ai_settings_load_key().
     */
    ai_config_t config;

    /*
     * Name of the keyring entry that holds the API key of
     * the provider.
     */
    char key_name[AI_KEY_NAME_MAX];

    /*
     * true when the provider needs an API key.
     */
    bool requires_key;
} ai_settings_t;

/*
 * Builds the path of the configuration file.
 *
 * Returns 0 on success and -1 on error (errno is set).
 */
int ai_settings_path(char *buffer, size_t size);

/*
 * Loads the AI configuration.
 *
 * When the file does not exist yet, the default configuration
 * is returned: the first provider with its default model. An
 * existing file is validated; problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
int ai_settings_load(ai_settings_t *settings);

/*
 * Stores the AI configuration in the file. The API key is not
 * part of it and is never written.
 *
 * Problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
int ai_settings_save(const ai_settings_t *settings);

/*
 * Switches the configuration to another provider.
 *
 * Besides the provider itself this resets what belongs to the
 * old provider: the model becomes the default model of the new
 * provider, the key name becomes the name of the new provider,
 * a custom address and certificate file are dropped, and the
 * API key in memory is wiped. That way the key of one provider
 * can never be sent to the address of another.
 *
 * Returns 0 on success and -1 when there is no such provider.
 */
int ai_settings_set_provider(ai_settings_t *settings, const char *name);

/*
 * Reads the API key of the provider from the keyring of the
 * system into settings->config.api_key.
 *
 * Does nothing and succeeds when the provider needs no key.
 *
 * error_text is a buffer of KEYRING_ERROR_TEXT_MAX characters
 * (see ai/keyring.h) that receives an explanation on failure.
 * May be NULL.
 *
 * Returns 0 on success and -1 on error; errno is ENOENT when no
 * key is stored, otherwise as set by keyring_load().
 */
int ai_settings_load_key(ai_settings_t *settings, char *error_text);

/*
 * Wipes the API key from memory. Call it when the configuration
 * is no longer needed.
 */
void ai_settings_wipe(ai_settings_t *settings);

#endif
