/*
 * Talking to an AI model.
 *
 * This is the interface that the rest of the program uses. It
 * does not depend on a particular AI service: the service is
 * chosen by name through ai_config_t.provider, and every service
 * ("provider") is implemented in its own file behind the
 * interface in ai/provider.h.
 *
 * Typical use:
 *
 *     ai_conversation_t *conversation = ai_conversation_new();
 *     ai_reply_t reply;
 *
 *     if (ai_send(&config, conversation, "Hello", NULL,
 *                 &reply, error_text) == AI_OK) {
 *         puts(reply.text);
 *         ai_reply_free(&reply);
 *     }
 *
 *     ai_conversation_free(conversation);
 *
 * A conversation remembers what was said, so the model sees the
 * earlier messages with every new one.
 */

#ifndef AI_H
#define AI_H

#include <limits.h>
#include <signal.h>
#include <stdbool.h>
#include <stddef.h>

/*
 * Sizes of the buffers in the structures below.
 */
#define AI_PROVIDER_NAME_MAX 32
#define AI_MODEL_MAX 64
#define AI_URL_MAX 256
#define AI_KEY_MAX 512
#define AI_SYSTEM_PROMPT_MAX 4096
#define AI_STOP_REASON_MAX 32
#define AI_ERROR_TEXT_MAX 320

/*
 * Outcome of ai_send().
 */
typedef enum {
    AI_OK,                /* a reply was received */
    AI_ERROR_CANCELLED,   /* aborted through the cancel flag */
    AI_ERROR_CONFIG,      /* unknown provider, missing key or model */
    AI_ERROR_NETWORK,     /* no answer: connection, TLS or timeout */
    AI_ERROR_AUTH,        /* the service rejected the API key */
    AI_ERROR_REQUEST,     /* the service rejected the request */
    AI_ERROR_RATE_LIMIT,  /* too many requests, try again later */
    AI_ERROR_SERVER,      /* the service failed or is overloaded */
    AI_ERROR_REFUSAL,     /* the model declined to answer */
    AI_ERROR_RESPONSE     /* the answer could not be understood */
} ai_status_t;

/*
 * Everything needed to send a request.
 */
typedef struct {
    /*
     * Name of the AI service, for example "anthropic".
     * See ai_provider_count() and ai_provider_get().
     */
    char provider[AI_PROVIDER_NAME_MAX];

    /*
     * Model to use, in the naming of the provider.
     */
    char model[AI_MODEL_MAX];

    /*
     * Address of the service. Empty uses the address of the
     * provider. Set it for a gateway or a local server.
     */
    char base_url[AI_URL_MAX];

    /*
     * Optional file with the certificate authorities to trust
     * for base_url (PEM format). Empty uses the certificates
     * of the system.
     */
    char ca_file[PATH_MAX];

    /*
     * API key. Empty when the provider needs none. A request
     * that carries a key is sent only over HTTPS with TLS 1.3.
     * The key comes from the keyring of the system (see
     * ai/keyring.h); wipe it when it is no longer needed.
     */
    char api_key[AI_KEY_MAX];

    /*
     * Optional instructions that apply to the whole
     * conversation. Empty for none.
     */
    char system_prompt[AI_SYSTEM_PROMPT_MAX];

    /*
     * Upper limit of the length of one reply, in tokens.
     */
    int max_tokens;

    /*
     * true lets the service answer with another model when the
     * requested model declines a request. Used by providers
     * that offer it; ignored by the others.
     */
    bool refusal_fallback;
} ai_config_t;

/*
 * The messages exchanged so far. Created by
 * ai_conversation_new() and released by ai_conversation_free().
 */
typedef struct ai_conversation ai_conversation_t;

/*
 * One reply of the model.
 */
typedef struct {
    /*
     * Text of the reply. Allocated by ai_send(); release it with
     * ai_reply_free(). May be empty, never NULL after AI_OK.
     */
    char *text;

    /*
     * Model that wrote the reply. It can differ from the
     * requested one when a fallback model answered.
     */
    char model[AI_MODEL_MAX];

    /*
     * Why the model stopped, in the naming of the provider.
     * "max_tokens" means that the reply was cut off.
     */
    char stop_reason[AI_STOP_REASON_MAX];

    /*
     * true when the reply was cut off by max_tokens.
     */
    bool truncated;

    /*
     * Tokens used by the request, as counted by the provider.
     * -1 when the provider did not report them.
     */
    long input_tokens;
    long output_tokens;
} ai_reply_t;

/*
 * Creates an empty conversation.
 *
 * Returns NULL when memory runs out.
 */
ai_conversation_t *ai_conversation_new(void);

/*
 * Forgets all messages of a conversation.
 */
void ai_conversation_clear(ai_conversation_t *conversation);

/*
 * Number of messages in a conversation. A question and its
 * reply are two messages.
 */
size_t ai_conversation_length(const ai_conversation_t *conversation);

/*
 * Releases a conversation. NULL is accepted.
 */
void ai_conversation_free(ai_conversation_t *conversation);

/*
 * Sends a message of the user and waits for the reply.
 *
 * On AI_OK the message and the reply are added to the
 * conversation. On any error the conversation is left as it was,
 * so the same message can simply be sent again.
 *
 * cancel lets the caller abort the request: it is given up soon
 * after *cancel becomes non-zero. May be NULL.
 *
 * error_text is a buffer of AI_ERROR_TEXT_MAX characters that
 * receives an explanation on error. May be NULL.
 */
ai_status_t ai_send(
    const ai_config_t *config,
    ai_conversation_t *conversation,
    const char *message,
    const volatile sig_atomic_t *cancel,
    ai_reply_t *reply,
    char *error_text
);

/*
 * Releases the text of a reply.
 */
void ai_reply_free(ai_reply_t *reply);

/*
 * Public description of an AI provider.
 */
typedef struct {
    const char *name;           /* used in ai_config_t.provider */
    const char *description;
    const char *default_model;
    bool requires_key;          /* false for local servers */
} ai_provider_info_t;

/*
 * Access to the list of supported providers.
 */
size_t ai_provider_count(void);

/*
 * Returns the provider at the given position, or NULL when the
 * index is out of range.
 */
const ai_provider_info_t *ai_provider_get(size_t index);

/*
 * Finds a provider by name. Letter case is ignored.
 *
 * Returns NULL when the provider is not supported.
 */
const ai_provider_info_t *ai_provider_find(const char *name);

#endif
