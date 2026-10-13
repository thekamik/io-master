/*
 * AI provider: Anthropic (Claude).
 *
 * Uses the Messages API over plain HTTPS; there is no official
 * SDK for C. One request looks like this:
 *
 *     POST {base_url}/v1/messages
 *     content-type: application/json
 *     x-api-key: <API key>
 *     anthropic-version: 2023-06-01
 *
 *     {
 *         "model": "claude-opus-5-5",
 *         "max_tokens": 16000,
 *         "system": "optional instructions",
 *         "messages": [
 *             {"role": "user", "content": "Hello"},
 *             {"role": "assistant", "content": [ ...blocks... ]},
 *             {"role": "user", "content": "And then?"}
 *         ]
 *     }
 *
 * and a successful answer like this:
 *
 *     {
 *         "model": "claude-opus-5-5",
 *         "content": [
 *             {"type": "thinking", ...},
 *             {"type": "text", "text": "Hi!"}
 *         ],
 *         "stop_reason": "end_turn",
 *         "usage": {"input_tokens": 12, "output_tokens": 34}
 *     }
 *
 * "content" is a list of blocks. The text shown to the user is
 * in the blocks of type "text". Other blocks, such as "thinking",
 * are not shown, but the whole list is stored in the conversation
 * and sent back unchanged with the next request, as the API
 * requires.
 *
 * Thinking is not configured in the request: on the default model
 * it is always on, and the model decides how much to think.
 *
 * Refusals
 * --------
 * The service can decline a request. That is not an HTTP error:
 * the answer has status 200 and "stop_reason": "refusal". With
 * ai_config_t.refusal_fallback the request also carries
 * "fallbacks": "default", which lets the service answer with
 * another model instead; a refusal is then reported only when
 * that model declines as well.
 *
 * Errors
 * ------
 * An answer with an error status has this body:
 *
 *     {"type": "error",
 *      "error": {"type": "authentication_error",
 *                "message": "invalid x-api-key"}}
 */

// TODO: Stream the reply (the "stream": true option of the API),
//       so the text appears while it is being written instead
//       of all at once.

#include "ai/provider.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Address of the service and of the Messages API on it.
 */
#define ANTHROPIC_BASE_URL "https://api.anthropic.com"
#define MESSAGES_PATH "/v1/messages"

/*
 * Default model: the current Claude Opus.
 */
#define ANTHROPIC_DEFAULT_MODEL "claude-opus-5-5"

/*
 * Fixed header lines of every request.
 */
#define HEADER_CONTENT_TYPE "content-type: application/json"
#define HEADER_API_VERSION "anthropic-version: 2023-06-01"

/*
 * Beta header that has to accompany "fallbacks": "default".
 */
#define HEADER_FALLBACK_BETA \
    "anthropic-beta: server-side-fallback-2026-07-01"

/*
 * Beginning of the header line that carries the API key.
 */
#define HEADER_API_KEY_PREFIX "x-api-key: "

/*
 * Models suggested to the user. The first one is the default.
 */
static const ai_model_info_t anthropic_models[] = {
    { ANTHROPIC_DEFAULT_MODEL, "most capable" },
    { "claude-sonnet-5-5", "balanced speed and capability" },
    { "claude-haiku-5-5", "fastest and cheapest" },
    { NULL, NULL },
};

/*
 * Beginning of the names of the Haiku models, which have no
 * server-side fallback.
 */
#define HAIKU_MODEL_PREFIX "claude-haiku"

/*
 * Tells whether a request should ask the service to fall back
 * to another model when the requested one declines.
 *
 * That takes the setting of the user and a model that supports
 * it. The Haiku models have no server-side fallback, so the
 * option is left out for them whatever the setting says.
 */
static bool use_fallback(const ai_config_t *config)
{
    return config->refusal_fallback &&
           strncmp(
               config->model,
               HAIKU_MODEL_PREFIX,
               strlen(HAIKU_MODEL_PREFIX)
           ) != 0;
}

/*
 * Stores an explanation in error_text, when the caller wants one.
 */
static void set_error(char *error_text, const char *text)
{
    if (error_text != NULL) {
        snprintf(error_text, AI_ERROR_TEXT_MAX, "%s", text);
    }
}

/*
 * Builds the JSON body of a request from the configuration and
 * the conversation.
 *
 * Returns the body as text, to be released with free(), or NULL
 * when it could not be built.
 */
static char *build_request_body(
    const ai_config_t *config,
    const ai_conversation_t *conversation
)
{
    /*
     * "O" stores the messages array itself, without copying it:
     * the request shares it with the conversation.
     */
    json_t *root = json_pack(
        "{s:s, s:i, s:O}",
        "model", config->model,
        "max_tokens", config->max_tokens,
        "messages", conversation->messages
    );

    if (root == NULL) {
        return NULL;
    }

    int failed = 0;

    if (config->system_prompt[0] != '\0') {
        failed |= json_object_set_new(
            root,
            "system",
            json_string(config->system_prompt)
        );
    }

    /*
     * Let the service answer with another model when the
     * requested one declines.
     */
    if (use_fallback(config)) {
        failed |= json_object_set_new(
            root,
            "fallbacks",
            json_string("default")
        );
    }

    char *body = NULL;

    if (!failed) {
        body = json_dumps(root, JSON_COMPACT);
    }

    json_decref(root);

    return body;
}

/*
 * Reads the explanation of an error answer into error_text.
 *
 * body is the body of the answer and status its HTTP status.
 * When the body is not the expected JSON, only the status
 * is reported.
 */
static void read_error_body(
    const char *body,
    long status,
    char *error_text
)
{
    if (error_text == NULL) {
        return;
    }

    json_t *root = json_loads(body, 0, NULL);

    const json_t *error = json_object_get(root, "error");
    const char *type = json_string_value(json_object_get(error, "type"));
    const char *message = json_string_value(
        json_object_get(error, "message")
    );

    if (message != NULL) {
        snprintf(
            error_text,
            AI_ERROR_TEXT_MAX,
            "%s (HTTP %ld, %s)",
            message,
            status,
            type != NULL ? type : "error"
        );
    } else {
        snprintf(error_text, AI_ERROR_TEXT_MAX, "HTTP %ld", status);
    }

    /*
     * json_decref() accepts NULL, which root is when the body
     * was not JSON.
     */
    json_decref(root);
}

/*
 * Translates the HTTP status of an error answer to the kind of
 * error reported to the caller.
 */
static ai_status_t status_to_error(long status)
{
    switch (status) {

    /*
     * 401: invalid key. 403: the key may not do this.
     */
    case 401:
    case 403:
        return AI_ERROR_AUTH;

    case 429:
        return AI_ERROR_RATE_LIMIT;

    default:
        /*
         * 500 and above (including 529 "overloaded") are
         * problems of the service; the rest are requests that
         * the service does not accept.
         */
        return status >= 500 ? AI_ERROR_SERVER : AI_ERROR_REQUEST;
    }
}

/*
 * Joins the text blocks of a reply into one text.
 *
 * content is the "content" array of the answer.
 *
 * Returns the text, to be released with free(), or NULL when
 * memory runs out. A reply without text blocks yields "".
 */
static char *collect_text(const json_t *content)
{
    size_t total = 0;
    size_t index;
    const json_t *block;

    /*
     * First pass: add up the lengths.
     */
    json_array_foreach(content, index, block) {
        const char *type = json_string_value(
            json_object_get(block, "type")
        );
        const json_t *text = json_object_get(block, "text");

        if (type != NULL &&
            strcmp(type, "text") == 0 &&
            json_is_string(text)) {

            total += json_string_length(text);
        }
    }

    char *result = malloc(total + 1);

    if (result == NULL) {
        return NULL;
    }

    /*
     * Second pass: copy the texts one after another.
     */
    size_t used = 0;

    json_array_foreach(content, index, block) {
        const char *type = json_string_value(
            json_object_get(block, "type")
        );
        const json_t *text = json_object_get(block, "text");

        if (type != NULL &&
            strcmp(type, "text") == 0 &&
            json_is_string(text)) {

            memcpy(
                result + used,
                json_string_value(text),
                json_string_length(text)
            );
            used += json_string_length(text);
        }
    }

    result[used] = '\0';

    return result;
}

/*
 * Reads a successful answer (HTTP 200) into reply and adds the
 * reply of the model to the conversation.
 *
 * Returns AI_OK, AI_ERROR_REFUSAL when the model declined, or
 * AI_ERROR_RESPONSE when the answer is not what the API promises.
 */
static ai_status_t read_reply(
    const char *body,
    ai_conversation_t *conversation,
    ai_reply_t *reply,
    char *error_text
)
{
    json_t *root = json_loads(body, 0, NULL);

    if (!json_is_object(root)) {
        json_decref(root);
        set_error(error_text, "the answer is not valid JSON");
        return AI_ERROR_RESPONSE;
    }

    const json_t *content = json_object_get(root, "content");
    const char *stop_reason = json_string_value(
        json_object_get(root, "stop_reason")
    );
    const char *model = json_string_value(json_object_get(root, "model"));
    const json_t *usage = json_object_get(root, "usage");

    /*
     * A refusal is checked before the content is read: a refused
     * answer can have no content at all.
     */
    if (stop_reason != NULL && strcmp(stop_reason, "refusal") == 0) {

        if (error_text != NULL) {
            /*
             * The category says which kind of safeguard declined.
             * It is informational and may be missing.
             */
            const char *category = json_string_value(
                json_object_get(
                    json_object_get(root, "stop_details"),
                    "category"
                )
            );

            snprintf(
                error_text,
                AI_ERROR_TEXT_MAX,
                "the model declined to answer (category: %s)",
                category != NULL ? category : "not given"
            );
        }

        json_decref(root);
        return AI_ERROR_REFUSAL;
    }

    if (!json_is_array(content)) {
        json_decref(root);
        set_error(error_text, "the answer has no content");
        return AI_ERROR_RESPONSE;
    }

    reply->text = collect_text(content);

    if (reply->text == NULL) {
        json_decref(root);
        set_error(error_text, "out of memory");
        return AI_ERROR_RESPONSE;
    }

    snprintf(
        reply->model,
        sizeof(reply->model),
        "%s",
        model != NULL ? model : ""
    );

    snprintf(
        reply->stop_reason,
        sizeof(reply->stop_reason),
        "%s",
        stop_reason != NULL ? stop_reason : ""
    );

    reply->truncated =
        stop_reason != NULL && strcmp(stop_reason, "max_tokens") == 0;

    /*
     * Token counts are optional information.
     */
    const json_t *input_tokens = json_object_get(usage, "input_tokens");
    const json_t *output_tokens = json_object_get(usage, "output_tokens");

    if (json_is_integer(input_tokens)) {
        reply->input_tokens = (long)json_integer_value(input_tokens);
    }

    if (json_is_integer(output_tokens)) {
        reply->output_tokens = (long)json_integer_value(output_tokens);
    }

    /*
     * Remember the reply. The whole content list is stored, not
     * only its text: the API expects blocks such as "thinking"
     * to come back unchanged with the next request.
     */
    json_t *message = json_pack(
        "{s:s, s:O}",
        "role", "assistant",
        "content", content
    );

    if (message == NULL ||
        json_array_append_new(conversation->messages, message) != 0) {

        ai_reply_free(reply);
        json_decref(root);
        set_error(error_text, "out of memory");
        return AI_ERROR_RESPONSE;
    }

    json_decref(root);

    return AI_OK;
}

/*
 * ai_provider_t.send
 */
static ai_status_t anthropic_send(
    const ai_config_t *config,
    const char *base_url,
    ai_conversation_t *conversation,
    const char *message,
    const volatile sig_atomic_t *cancel,
    ai_reply_t *reply,
    char *error_text
)
{
    char url[AI_URL_MAX + sizeof(MESSAGES_PATH)];
    char key_header[sizeof(HEADER_API_KEY_PREFIX) + AI_KEY_MAX];
    http_response_t response = { 0, NULL, 0 };

    /*
     * Add the message of the user to the conversation. It is
     * taken out again below when the request fails, so the
     * conversation only ever holds complete exchanges.
     */
    json_t *user_message = json_pack(
        "{s:s, s:s}",
        "role", "user",
        "content", message
    );

    /*
     * json_pack() returns NULL for text that is not valid UTF-8.
     */
    if (user_message == NULL) {
        set_error(error_text, "the message is not valid UTF-8 text");
        return AI_ERROR_REQUEST;
    }

    if (json_array_append_new(conversation->messages, user_message) != 0) {
        set_error(error_text, "out of memory");
        return AI_ERROR_RESPONSE;
    }

    size_t user_index = json_array_size(conversation->messages) - 1;

    char *body = build_request_body(config, conversation);

    if (body == NULL) {
        json_array_remove(conversation->messages, user_index);
        set_error(error_text, "cannot build the request");
        return AI_ERROR_REQUEST;
    }

    snprintf(url, sizeof(url), "%s%s", base_url, MESSAGES_PATH);

    snprintf(
        key_header,
        sizeof(key_header),
        "%s%s",
        HEADER_API_KEY_PREFIX,
        config->api_key
    );

    /*
     * Header lines, ended by NULL. The last line is used only
     * together with "fallbacks" in the body.
     */
    const char *headers[] = {
        HEADER_CONTENT_TYPE,
        HEADER_API_VERSION,
        key_header,
        use_fallback(config) ? HEADER_FALLBACK_BETA : NULL,
        NULL,
    };

    ai_status_t status = ai_post(
        config,
        url,
        headers,
        body,
        cancel,
        &response,
        error_text
    );

    /*
     * The key must not stay in memory longer than needed.
     */
    explicit_bzero(key_header, sizeof(key_header));

    free(body);

    if (status == AI_OK) {

        if (response.status == 200) {
            status = read_reply(
                response.body,
                conversation,
                reply,
                error_text
            );
        } else {
            read_error_body(response.body, response.status, error_text);
            status = status_to_error(response.status);
        }

        http_response_free(&response);
    }

    /*
     * No reply was stored: take the message of the user out
     * again. read_reply() appends the reply only on AI_OK, so
     * the message is still at user_index.
     */
    if (status != AI_OK) {
        json_array_remove(conversation->messages, user_index);
    }

    return status;
}

/*
 * ai_provider_t.add_text
 *
 * {"role": "user" or "assistant", "content": "..."}
 */
static int anthropic_add_text(
    ai_conversation_t *conversation,
    bool from_user,
    const char *text
)
{
    /*
     * json_pack() returns NULL for text that is not valid UTF-8,
     * and json_array_append_new() fails for NULL.
     */
    json_t *message = json_pack(
        "{s:s, s:s}",
        "role", from_user ? "user" : "assistant",
        "content", text
    );

    return json_array_append_new(conversation->messages, message);
}

/*
 * Registration of the provider. The table in ai.c refers to
 * this structure.
 */
const ai_provider_t anthropic_provider = {
    .info = {
        .name = "anthropic",
        .alias = "claude",
        .description = "Claude models by Anthropic",
        .default_model = ANTHROPIC_DEFAULT_MODEL,
        .models = anthropic_models,
        .requires_key = true,
    },
    .default_base_url = ANTHROPIC_BASE_URL,
    .send = anthropic_send,
    .add_text = anthropic_add_text,
};
