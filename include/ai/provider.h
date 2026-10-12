/*
 * Interface between the AI module and one AI provider.
 *
 * This is an internal header of the AI module. The rest of the
 * program uses ai/ai.h.
 *
 * Adding a provider:
 *   1. implement it in src/ai/<name>.c and define an
 *      ai_provider_t for it,
 *   2. add that ai_provider_t to the table in src/ai/ai.c.
 *
 * A provider that speaks the OpenAI chat format would cover the
 * OpenAI service as well as most local servers (Ollama,
 * llama.cpp, LM Studio), which differ only in their address.
 */

// TODO: Add an "openai" provider for OpenAI-compatible services
//       and local servers. It needs no API key for local use
//       (requires_key = false), in which case plain HTTP is
//       allowed by ai_post().

#ifndef AI_PROVIDER_H
#define AI_PROVIDER_H

#include <jansson.h>

#include "ai/ai.h"
#include "ai/http.h"

/*
 * A conversation: the list of messages in the JSON form that
 * the provider sends to its service. Because the form belongs
 * to the provider, a conversation should stay with the provider
 * that started it.
 */
struct ai_conversation {
    /*
     * JSON array of messages, oldest first.
     */
    json_t *messages;
};

/*
 * One AI provider.
 */
typedef struct {
    /*
     * Public part: name, description, default model and whether
     * an API key is needed.
     */
    ai_provider_info_t info;

    /*
     * Address of the service, used when ai_config_t.base_url
     * is empty.
     */
    const char *default_base_url;

    /*
     * Sends one message and reads the reply. The rules are those
     * of ai_send() in ai/ai.h; in particular the conversation
     * must be left unchanged on error.
     *
     * base_url is the address to use, already chosen between
     * the configuration and the default.
     */
    ai_status_t (*send)(
        const ai_config_t *config,
        const char *base_url,
        ai_conversation_t *conversation,
        const char *message,
        const volatile sig_atomic_t *cancel,
        ai_reply_t *reply,
        char *error_text
    );
} ai_provider_t;

/*
 * Helper for the providers: sends a JSON request and returns the
 * answer, trying again when that makes sense.
 *
 * A request is repeated up to two times when the service is busy
 * (status 408, 409, 429 or 5xx) or the connection failed. The
 * waits in between are short and can be cancelled.
 *
 * The request is marked as secure, which enforces HTTPS with
 * TLS 1.3, whenever config->api_key is not empty.
 *
 * Returns AI_OK when an answer arrived, whatever its status; the
 * provider then reads response->status and response->body and
 * releases the answer with http_response_free(). Otherwise
 * returns AI_ERROR_CANCELLED or AI_ERROR_NETWORK and fills
 * error_text.
 */
ai_status_t ai_post(
    const ai_config_t *config,
    const char *url,
    const char *const *headers,
    const char *body,
    const volatile sig_atomic_t *cancel,
    http_response_t *response,
    char *error_text
);

/*
 * The providers. Each is defined in its own file.
 */
extern const ai_provider_t anthropic_provider;

#endif
