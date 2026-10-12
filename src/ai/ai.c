/*
 * Talking to an AI model: the part that is the same for every
 * provider.
 *
 *     providers       the table of supported AI services
 *     conversations   creating, clearing and releasing them
 *     ai_post()       sending a request, with retries
 *     ai_send()       checks the configuration and hands the
 *                     request to the chosen provider
 */

#include "ai/ai.h"

#include "ai/provider.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <time.h>

/*
 * Longest time one request may take, in seconds. A long reply
 * of a large model can need several minutes.
 */
#define REQUEST_TIMEOUT_SECONDS 600L

/*
 * How often a failed request is tried again, and how long to
 * wait before the first retry. The wait doubles every time.
 */
#define MAX_RETRIES 2
#define FIRST_RETRY_DELAY_MS 1000L

/*
 * Table of supported providers.
 *
 * A new provider is registered by adding one entry here.
 */
static const ai_provider_t *const providers[] = {
    &anthropic_provider,
};

static const size_t provider_count =
    sizeof(providers) / sizeof(providers[0]);

size_t ai_provider_count(void)
{
    return provider_count;
}

const ai_provider_info_t *ai_provider_get(size_t index)
{
    if (index >= provider_count) {
        return NULL;
    }

    return &providers[index]->info;
}

/*
 * Finds the full description of a provider by name.
 */
static const ai_provider_t *find_provider(const char *name)
{
    if (name == NULL) {
        return NULL;
    }

    for (size_t i = 0; i < provider_count; i++) {

        if (strcasecmp(providers[i]->info.name, name) == 0) {
            return providers[i];
        }
    }

    return NULL;
}

const ai_provider_info_t *ai_provider_find(const char *name)
{
    const ai_provider_t *provider = find_provider(name);

    return provider != NULL ? &provider->info : NULL;
}

ai_conversation_t *ai_conversation_new(void)
{
    ai_conversation_t *conversation = calloc(1, sizeof(*conversation));

    if (conversation == NULL) {
        return NULL;
    }

    conversation->messages = json_array();

    if (conversation->messages == NULL) {
        free(conversation);
        return NULL;
    }

    return conversation;
}

void ai_conversation_clear(ai_conversation_t *conversation)
{
    if (conversation != NULL) {
        json_array_clear(conversation->messages);
    }
}

size_t ai_conversation_length(const ai_conversation_t *conversation)
{
    if (conversation == NULL) {
        return 0;
    }

    return json_array_size(conversation->messages);
}

void ai_conversation_free(ai_conversation_t *conversation)
{
    if (conversation == NULL) {
        return;
    }

    json_decref(conversation->messages);
    free(conversation);
}

void ai_reply_free(ai_reply_t *reply)
{
    if (reply == NULL) {
        return;
    }

    free(reply->text);
    reply->text = NULL;
}

/*
 * Tells whether a request that got this HTTP status is worth
 * repeating: the service was busy or failed for a moment.
 */
static bool is_retryable_status(long status)
{
    return status == 408 ||
           status == 409 ||
           status == 429 ||
           status >= 500;
}

/*
 * Waits before a retry. The wait is cut into short pieces, so
 * that the cancel flag is noticed quickly.
 *
 * Returns true when the wait was cancelled.
 */
static bool wait_cancellable(
    long milliseconds,
    const volatile sig_atomic_t *cancel
)
{
    const long piece_ms = 100;

    for (long waited = 0; waited < milliseconds; waited += piece_ms) {

        if (cancel != NULL && *cancel != 0) {
            return true;
        }

        struct timespec piece = {
            .tv_sec = 0,
            .tv_nsec = piece_ms * 1000000L,
        };

        nanosleep(&piece, NULL);
    }

    return cancel != NULL && *cancel != 0;
}

ai_status_t ai_post(
    const ai_config_t *config,
    const char *url,
    const char *const *headers,
    const char *body,
    const volatile sig_atomic_t *cancel,
    http_response_t *response,
    char *error_text
)
{
    char http_error[HTTP_ERROR_TEXT_MAX];

    http_request_t request = {
        .url = url,
        .headers = headers,
        .body = body,

        /*
         * A request that carries an API key must be protected.
         */
        .secure = config->api_key[0] != '\0',

        .ca_file = config->ca_file,
        .timeout_seconds = REQUEST_TIMEOUT_SECONDS,
        .cancel = cancel,
    };

    long delay_ms = FIRST_RETRY_DELAY_MS;

    for (int attempt = 0; ; attempt++) {
        bool last_attempt = attempt >= MAX_RETRIES;

        int result = http_post(&request, response, http_error);

        if (result == 0) {
            /*
             * An answer arrived. Hand it over, unless the
             * service asked to try again and tries are left.
             */
            if (!is_retryable_status(response->status) || last_attempt) {
                return AI_OK;
            }

            http_response_free(response);

        } else {

            if (errno == ECANCELED) {
                if (error_text != NULL) {
                    snprintf(error_text, AI_ERROR_TEXT_MAX, "cancelled");
                }
                return AI_ERROR_CANCELLED;
            }

            /*
             * Problems that another try cannot fix: a wrong
             * address or an unacceptable TLS setup. The same
             * goes for the last try.
             */
            if (errno == EINVAL ||
                errno == EPROTOTYPE ||
                errno == EPROTO ||
                last_attempt) {

                if (error_text != NULL) {
                    snprintf(
                        error_text,
                        AI_ERROR_TEXT_MAX,
                        "%s",
                        http_error
                    );
                }
                return AI_ERROR_NETWORK;
            }
        }

        if (wait_cancellable(delay_ms, cancel)) {
            if (error_text != NULL) {
                snprintf(error_text, AI_ERROR_TEXT_MAX, "cancelled");
            }
            return AI_ERROR_CANCELLED;
        }

        delay_ms *= 2;
    }
}

ai_status_t ai_send(
    const ai_config_t *config,
    ai_conversation_t *conversation,
    const char *message,
    const volatile sig_atomic_t *cancel,
    ai_reply_t *reply,
    char *error_text
)
{
    if (error_text != NULL) {
        error_text[0] = '\0';
    }

    if (config == NULL ||
        conversation == NULL ||
        message == NULL ||
        reply == NULL) {

        if (error_text != NULL) {
            snprintf(error_text, AI_ERROR_TEXT_MAX, "internal error");
        }
        return AI_ERROR_CONFIG;
    }

    /*
     * Start from an empty reply, so ai_reply_free() is safe
     * whatever happens.
     */
    memset(reply, 0, sizeof(*reply));

    reply->input_tokens = -1;
    reply->output_tokens = -1;

    const ai_provider_t *provider = find_provider(config->provider);

    if (provider == NULL) {
        if (error_text != NULL) {
            snprintf(
                error_text,
                AI_ERROR_TEXT_MAX,
                "unknown AI provider '%s'",
                config->provider
            );
        }
        return AI_ERROR_CONFIG;
    }

    if (provider->info.requires_key && config->api_key[0] == '\0') {
        if (error_text != NULL) {
            snprintf(
                error_text,
                AI_ERROR_TEXT_MAX,
                "provider '%s' needs an API key",
                provider->info.name
            );
        }
        return AI_ERROR_CONFIG;
    }

    if (config->model[0] == '\0') {
        if (error_text != NULL) {
            snprintf(error_text, AI_ERROR_TEXT_MAX, "no model chosen");
        }
        return AI_ERROR_CONFIG;
    }

    /*
     * The address from the configuration wins over the address
     * of the provider.
     */
    const char *base_url = config->base_url[0] != '\0'
        ? config->base_url
        : provider->default_base_url;

    return provider->send(
        config,
        base_url,
        conversation,
        message,
        cancel,
        reply,
        error_text
    );
}
