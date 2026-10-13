/*
 * The AI configuration of the program: loading, validating and
 * saving im_config/ai_config.json.
 */

#include "ai/ai_settings.h"

#include "ai/keyring.h"
#include "storage/config_dir.h"
#include "storage/json_file.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/*
 * Keys used in the configuration file.
 */
#define KEY_PROVIDER "provider"
#define KEY_MODEL "model"
#define KEY_KEY_NAME "key_name"
#define KEY_MAX_TOKENS "max_tokens"
#define KEY_SYSTEM "system"
#define KEY_REFUSAL_FALLBACK "refusal_fallback"
#define KEY_BASE_URL "base_url"
#define KEY_CA_FILE "ca_file"

/*
 * Default length limit of a reply. 16000 tokens leave room for
 * a long reply and still keep a request that is not streamed
 * within the time limit.
 */
#define DEFAULT_MAX_TOKENS 16000

int ai_settings_path(char *buffer, size_t size)
{
    return config_build_path(buffer, size, AI_SETTINGS_FILE_NAME);
}

int ai_settings_set_provider(ai_settings_t *settings, const char *name)
{
    const ai_provider_info_t *provider = ai_provider_find(name);

    if (provider == NULL) {
        return -1;
    }

    ai_config_t *config = &settings->config;

    /*
     * The provider is stored under its canonical name.
     */
    snprintf(
        config->provider,
        sizeof(config->provider),
        "%s",
        provider->name
    );
    snprintf(
        config->model,
        sizeof(config->model),
        "%s",
        provider->default_model
    );
    snprintf(
        settings->key_name,
        sizeof(settings->key_name),
        "%s",
        provider->name
    );

    /*
     * A custom address belongs to the old provider.
     */
    config->base_url[0] = '\0';
    config->ca_file[0] = '\0';

    settings->requires_key = provider->requires_key;

    /*
     * So does the key that may be in memory.
     */
    explicit_bzero(config->api_key, sizeof(config->api_key));

    return 0;
}

/*
 * Fills settings with the default configuration: the first
 * registered provider, its default model and no extras.
 */
static void set_defaults(ai_settings_t *settings)
{
    memset(settings, 0, sizeof(*settings));

    const ai_provider_info_t *provider = ai_provider_get(0);

    if (provider != NULL) {
        ai_settings_set_provider(settings, provider->name);
    }

    settings->config.max_tokens = DEFAULT_MAX_TOKENS;
    settings->config.refusal_fallback = true;
}

/*
 * Checks that a setting is text that fits into a buffer of
 * the given size.
 */
static bool is_text_setting(const json_t *value, size_t size)
{
    return json_is_string(value) &&
           strlen(json_string_value(value)) < size;
}

/*
 * Moves a configuration file written by an earlier version of
 * the program, which kept it directly in im_config, to its
 * place in the AI directory.
 *
 * new_path is the path of the file in the AI directory. Nothing
 * happens when a file is already there or there is no old file.
 * A failure is not reported: the program then simply starts
 * with the default configuration.
 */
static void move_old_file(const char *new_path)
{
    char old_path[PATH_MAX];

    if (access(new_path, F_OK) == 0) {
        return;
    }

    if (config_build_path(
            old_path,
            sizeof(old_path),
            AI_SETTINGS_OLD_FILE_NAME) != 0 ||
        access(old_path, F_OK) != 0) {
        return;
    }

    if (config_create_directories(AI_DIRECTORY_NAME, 0755) == 0) {
        rename(old_path, new_path);
    }
}

int ai_settings_load(ai_settings_t *settings)
{
    char path[PATH_MAX];
    json_t *root = NULL;

    if (settings == NULL) {
        return -1;
    }

    set_defaults(settings);

    if (ai_settings_path(path, sizeof(path)) != 0) {
        perror("AI configuration");
        return -1;
    }

    move_old_file(path);

    if (json_file_read(path, &root) != 0) {

        /*
         * No file yet: the defaults set above apply.
         */
        if (errno == ENOENT) {
            return 0;
        }

        /*
         * Invalid JSON has already been reported.
         */
        if (errno != EINVAL) {
            perror(path);
        }

        return -1;
    }

    ai_config_t *config = &settings->config;

    /*
     * Fetch every setting. A missing key yields NULL, which
     * fails the type checks below.
     */
    const json_t *provider = json_object_get(root, KEY_PROVIDER);
    const json_t *model = json_object_get(root, KEY_MODEL);
    const json_t *key_name = json_object_get(root, KEY_KEY_NAME);
    const json_t *max_tokens = json_object_get(root, KEY_MAX_TOKENS);
    const json_t *system = json_object_get(root, KEY_SYSTEM);
    const json_t *fallback = json_object_get(root, KEY_REFUSAL_FALLBACK);
    const json_t *base_url = json_object_get(root, KEY_BASE_URL);
    const json_t *ca_file = json_object_get(root, KEY_CA_FILE);

    /*
     * Validate the settings one by one and remember the
     * first problem.
     */
    const char *problem = NULL;
    const ai_provider_info_t *provider_info = NULL;

    if (!is_text_setting(provider, sizeof(config->provider)) ||
        (provider_info = ai_provider_find(
            json_string_value(provider))) == NULL) {

        problem = "'" KEY_PROVIDER "' must be a supported AI provider";

    } else if (!is_text_setting(model, sizeof(config->model)) ||
               json_string_length(model) == 0) {

        problem = "'" KEY_MODEL "' must be the name of a model";

    } else if (!is_text_setting(key_name, sizeof(settings->key_name)) ||
               json_string_length(key_name) == 0) {

        problem = "'" KEY_KEY_NAME "' must be the name of the "
                  "keyring entry that holds the API key";

    } else if (!json_is_integer(max_tokens) ||
               json_integer_value(max_tokens) < AI_MAX_TOKENS_MIN ||
               json_integer_value(max_tokens) > AI_MAX_TOKENS_MAX) {

        problem = "'" KEY_MAX_TOKENS "' must be a number "
                  "from 1 to 128000";

    } else if (!is_text_setting(system, sizeof(config->system_prompt))) {

        problem = "'" KEY_SYSTEM "' must be text of at most "
                  "4095 characters";

    } else if (!json_is_boolean(fallback)) {

        problem = "'" KEY_REFUSAL_FALLBACK "' must be true or false";

    } else if (!is_text_setting(base_url, sizeof(config->base_url))) {

        problem = "'" KEY_BASE_URL "' must be an address, "
                  "or \"\" for the address of the provider";

    } else if (!is_text_setting(ca_file, sizeof(config->ca_file))) {

        problem = "'" KEY_CA_FILE "' must be a file path, "
                  "or \"\" for the certificates of the system";
    }

    if (problem != NULL) {
        fprintf(
            stderr,
            "%s: %s\n"
            "Fix the file, or delete it and configure the AI again.\n",
            path,
            problem
        );
        json_decref(root);
        return -1;
    }

    /*
     * The lengths were checked above, so nothing is cut off.
     */
    snprintf(
        config->provider,
        sizeof(config->provider),
        "%s",
        provider_info->name
    );
    snprintf(
        config->model,
        sizeof(config->model),
        "%s",
        json_string_value(model)
    );
    snprintf(
        config->system_prompt,
        sizeof(config->system_prompt),
        "%s",
        json_string_value(system)
    );
    snprintf(
        config->base_url,
        sizeof(config->base_url),
        "%s",
        json_string_value(base_url)
    );
    snprintf(
        config->ca_file,
        sizeof(config->ca_file),
        "%s",
        json_string_value(ca_file)
    );
    snprintf(
        settings->key_name,
        sizeof(settings->key_name),
        "%s",
        json_string_value(key_name)
    );

    config->max_tokens = (int)json_integer_value(max_tokens);
    config->refusal_fallback = json_is_true(fallback);

    settings->requires_key = provider_info->requires_key;

    json_decref(root);

    return 0;
}

int ai_settings_save(const ai_settings_t *settings)
{
    char path[PATH_MAX];

    if (settings == NULL) {
        return -1;
    }

    if (config_create_directories(AI_DIRECTORY_NAME, 0755) != 0 ||
        ai_settings_path(path, sizeof(path)) != 0) {

        perror("AI configuration");
        return -1;
    }

    const ai_config_t *config = &settings->config;

    /*
     * Only these eight values are written. The API key is not
     * among them. json_pack() returns NULL when a text is not
     * valid UTF-8.
     */
    json_t *root = json_pack(
        "{s:s, s:s, s:s, s:i, s:s, s:b, s:s, s:s}",
        KEY_PROVIDER, config->provider,
        KEY_MODEL, config->model,
        KEY_KEY_NAME, settings->key_name,
        KEY_MAX_TOKENS, config->max_tokens,
        KEY_SYSTEM, config->system_prompt,
        KEY_REFUSAL_FALLBACK, config->refusal_fallback ? 1 : 0,
        KEY_BASE_URL, config->base_url,
        KEY_CA_FILE, config->ca_file
    );

    if (root == NULL) {
        fprintf(
            stderr,
            "Cannot store the AI configuration: a value is not "
            "valid UTF-8 text\n"
        );
        return -1;
    }

    if (json_file_write(path, root) != 0) {
        perror(path);
        json_decref(root);
        return -1;
    }

    json_decref(root);

    return 0;
}

int ai_settings_load_key(ai_settings_t *settings, char *error_text)
{
    if (error_text != NULL) {
        error_text[0] = '\0';
    }

    if (!settings->requires_key) {
        return 0;
    }

    return keyring_load(
        settings->key_name,
        settings->config.api_key,
        sizeof(settings->config.api_key),
        error_text
    );
}

void ai_settings_wipe(ai_settings_t *settings)
{
    if (settings != NULL) {
        explicit_bzero(
            settings->config.api_key,
            sizeof(settings->config.api_key)
        );
    }
}
