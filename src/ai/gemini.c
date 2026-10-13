/*
 * AI provider: Google Gemini.
 *
 * Uses the generateContent method of the Gemini API over plain
 * HTTPS. One request looks like this:
 *
 *     POST {base_url}/v1beta/models/{model}:generateContent
 *     content-type: application/json
 *     x-goog-api-key: <API key>
 *
 *     {
 *         "systemInstruction": {"parts": [{"text": "optional"}]},
 *         "contents": [
 *             {"role": "user",  "parts": [{"text": "Hello"}]},
 *             {"role": "model", "parts": [ ...parts... ]},
 *             {"role": "user",  "parts": [{"text": "And then?"}]}
 *         ],
 *         "generationConfig": {"maxOutputTokens": 16000}
 *     }
 *
 * and a successful answer like this:
 *
 *     {
 *         "candidates": [{
 *             "content": {
 *                 "role": "model",
 *                 "parts": [
 *                     {"text": "...", "thought": true},
 *                     {"text": "Hi!", "thoughtSignature": "..."}
 *                 ]
 *             },
 *             "finishReason": "STOP"
 *         }],
 *         "usageMetadata": {"promptTokenCount": 12,
 *                           "candidatesTokenCount": 34},
 *         "modelVersion": "gemini-3.8-flash"
 *     }
 *
 * The API key is sent in a header, not in the address, so it
 * does not end up in logs that record addresses.
 *
 * Differences from the Anthropic provider that matter here:
 *
 *   - The reply of the model has the role "model" and consists
 *     of "parts". Parts marked with "thought": true are notes of
 *     the model and are not shown. The whole "content" object is
 *     stored in the conversation and sent back unchanged, because
 *     its parts can carry signatures that the API expects to
 *     see again.
 *
 *   - A declined request has no dedicated status. Either the
 *     answer has no candidates and "promptFeedback.blockReason"
 *     says why, or the candidate ends with a "finishReason" such
 *     as "SAFETY" and carries no text.
 *
 *   - Errors have this body, with status 400 even for an
 *     invalid key:
 *
 *         {"error": {"code": 400,
 *                    "message": "API key not valid. ...",
 *                    "status": "INVALID_ARGUMENT",
 *                    "details": [{"reason": "API_KEY_INVALID"}]}}
 *
 *   - ai_config_t.refusal_fallback has no meaning for Gemini
 *     and is ignored.
 */

// TODO: Stream the reply (the streamGenerateContent method), so
//       the text appears while it is being written.

#include "ai/provider.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Address of the service.
 */
#define GEMINI_BASE_URL "https://generativelanguage.googleapis.com"

/*
 * Default model: a current Flash model, which the free tier of
 * the Gemini API includes.
 */
#define GEMINI_DEFAULT_MODEL "gemini-3.8-flash"

/*
 * Header lines of a request.
 */
#define HEADER_CONTENT_TYPE "content-type: application/json"
#define HEADER_API_KEY_PREFIX "x-goog-api-key: "

/*
 * Characters that a model name may contain. The name becomes
 * part of the address, so nothing that has a meaning in an
 * address ('/', '?', '#', ':') is allowed.
 */
#define MODEL_NAME_CHARACTERS \
    "abcdefghijklmnopqrstuvwxyz" \
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ" \
    "0123456789.-_"

/*
 * Models suggested to the user. The first one is the default.
 * All three are part of the free tier of the Gemini API.
 */
static const ai_model_info_t gemini_models[] = {
    { GEMINI_DEFAULT_MODEL, "most capable Flash model" },
    { "gemini-3.5-flash-lite", "fastest and cheapest" },
    { "gemini-2.5-pro", "older Pro model" },
    { NULL, NULL },
};

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
        "{s:O, s:{s:i}}",
        "contents", conversation->messages,
        "generationConfig",
            "maxOutputTokens", config->max_tokens
    );

    if (root == NULL) {
        return NULL;
    }

    int failed = 0;

    if (config->system_prompt[0] != '\0') {
        /*
         * {"parts": [{"text": "..."}]}
         */
        failed |= json_object_set_new(
            root,
            "systemInstruction",
            json_pack("{s:[{s:s}]}", "parts", "text", config->system_prompt)
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
 * Reads an error answer: stores its explanation in error_text
 * and returns the kind of error.
 *
 * body is the body of the answer and status its HTTP status.
 */
static ai_status_t read_error(
    const char *body,
    long status,
    char *error_text
)
{
    json_t *root = json_loads(body, 0, NULL);

    const json_t *error = json_object_get(root, "error");
    const char *message = json_string_value(
        json_object_get(error, "message")
    );
    const char *status_name = json_string_value(
        json_object_get(error, "status")
    );

    if (error_text != NULL) {

        if (message != NULL) {
            snprintf(
                error_text,
                AI_ERROR_TEXT_MAX,
                "%s (HTTP %ld, %s)",
                message,
                status,
                status_name != NULL ? status_name : "error"
            );
        } else {
            snprintf(error_text, AI_ERROR_TEXT_MAX, "HTTP %ld", status);
        }
    }

    /*
     * An invalid key is reported as a plain 400. It is told
     * apart by the reason "API_KEY_INVALID" in the details.
     */
    bool key_invalid = false;

    size_t index;
    const json_t *detail;

    json_array_foreach(json_object_get(error, "details"), index, detail) {
        const char *reason = json_string_value(
            json_object_get(detail, "reason")
        );

        if (reason != NULL && strcmp(reason, "API_KEY_INVALID") == 0) {
            key_invalid = true;
        }
    }

    /*
     * json_decref() accepts NULL, which root is when the body
     * was not JSON. The strings read above belong to root and
     * are not used after this point.
     */
    json_decref(root);

    if (key_invalid || status == 401 || status == 403) {
        return AI_ERROR_AUTH;
    }

    /*
     * 429 is also what the free tier answers when its quota
     * is used up.
     */
    if (status == 429) {
        return AI_ERROR_RATE_LIMIT;
    }

    return status >= 500 ? AI_ERROR_SERVER : AI_ERROR_REQUEST;
}

/*
 * Tells whether a part of a reply is text meant for the user:
 * it has a "text" and is not marked as a thought of the model.
 */
static bool is_visible_text(const json_t *part)
{
    return json_is_string(json_object_get(part, "text")) &&
           !json_is_true(json_object_get(part, "thought"));
}

/*
 * Joins the visible text parts of a reply into one text.
 *
 * parts is the "parts" array of the reply; NULL is accepted.
 *
 * Returns the text, to be released with free(), or NULL when
 * memory runs out. A reply without visible text yields "".
 */
static char *collect_text(const json_t *parts)
{
    size_t total = 0;
    size_t index;
    const json_t *part;

    /*
     * First pass: add up the lengths.
     */
    json_array_foreach(parts, index, part) {

        if (is_visible_text(part)) {
            total += json_string_length(json_object_get(part, "text"));
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

    json_array_foreach(parts, index, part) {

        if (is_visible_text(part)) {
            const json_t *text = json_object_get(part, "text");

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
 * Returns AI_OK, AI_ERROR_REFUSAL when the request was blocked,
 * or AI_ERROR_RESPONSE when the answer is not what the API
 * promises.
 */
static ai_status_t read_reply(
    const char *body,
    const ai_config_t *config,
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

    /*
     * Only one candidate is requested, so only the first one
     * is read. json_array_get() returns NULL when there is none.
     */
    const json_t *candidate = json_array_get(
        json_object_get(root, "candidates"),
        0
    );

    /*
     * No candidate at all: the request itself was blocked.
     */
    if (candidate == NULL) {

        if (error_text != NULL) {
            const char *reason = json_string_value(
                json_object_get(
                    json_object_get(root, "promptFeedback"),
                    "blockReason"
                )
            );

            snprintf(
                error_text,
                AI_ERROR_TEXT_MAX,
                "the request was blocked (reason: %s)",
                reason != NULL ? reason : "not given"
            );
        }

        json_decref(root);
        return AI_ERROR_REFUSAL;
    }

    json_t *content = json_object_get(candidate, "content");
    const char *finish_reason = json_string_value(
        json_object_get(candidate, "finishReason")
    );

    if (finish_reason == NULL) {
        finish_reason = "";
    }

    reply->text = collect_text(json_object_get(content, "parts"));

    if (reply->text == NULL) {
        json_decref(root);
        set_error(error_text, "out of memory");
        return AI_ERROR_RESPONSE;
    }

    bool finished_normally =
        strcmp(finish_reason, "STOP") == 0 ||
        strcmp(finish_reason, "MAX_TOKENS") == 0;

    /*
     * No text and an unusual end, such as "SAFETY": the model
     * was stopped before it could answer.
     */
    if (reply->text[0] == '\0' && !finished_normally) {

        if (error_text != NULL) {
            snprintf(
                error_text,
                AI_ERROR_TEXT_MAX,
                "the model gave no answer (reason: %s)",
                finish_reason[0] != '\0' ? finish_reason : "not given"
            );
        }

        ai_reply_free(reply);
        json_decref(root);
        return AI_ERROR_REFUSAL;
    }

    /*
     * The answer names the exact model version that replied.
     */
    const char *model = json_string_value(
        json_object_get(root, "modelVersion")
    );

    snprintf(
        reply->model,
        sizeof(reply->model),
        "%s",
        model != NULL ? model : config->model
    );

    snprintf(
        reply->stop_reason,
        sizeof(reply->stop_reason),
        "%s",
        finish_reason
    );

    reply->truncated = strcmp(finish_reason, "MAX_TOKENS") == 0;

    /*
     * Token counts are optional information. The tokens that
     * the model spent on thinking count as output.
     */
    const json_t *usage = json_object_get(root, "usageMetadata");
    const json_t *prompt_tokens = json_object_get(
        usage,
        "promptTokenCount"
    );
    const json_t *answer_tokens = json_object_get(
        usage,
        "candidatesTokenCount"
    );
    const json_t *thought_tokens = json_object_get(
        usage,
        "thoughtsTokenCount"
    );

    if (json_is_integer(prompt_tokens)) {
        reply->input_tokens = (long)json_integer_value(prompt_tokens);
    }

    if (json_is_integer(answer_tokens)) {
        reply->output_tokens = (long)json_integer_value(answer_tokens);

        if (json_is_integer(thought_tokens)) {
            reply->output_tokens +=
                (long)json_integer_value(thought_tokens);
        }
    }

    /*
     * Remember the reply: the whole "content" object, with all
     * of its parts, as the API expects it back. A reply without
     * content (possible when it was cut off at once) is stored
     * as an empty text, so that user and model messages keep
     * alternating.
     */
    json_t *message = NULL;

    if (json_is_object(content)) {
        message = json_incref(content);

        /*
         * Make sure the role is there; the API requires it
         * in a request.
         */
        if (json_object_get(message, "role") == NULL &&
            json_object_set_new(
                message,
                "role",
                json_string("model")) != 0) {

            json_decref(message);
            message = NULL;
        }
    } else {
        message = json_pack(
            "{s:s, s:[{s:s}]}",
            "role", "model",
            "parts", "text", ""
        );
    }

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
static ai_status_t gemini_send(
    const ai_config_t *config,
    const char *base_url,
    ai_conversation_t *conversation,
    const char *message,
    const volatile sig_atomic_t *cancel,
    ai_reply_t *reply,
    char *error_text
)
{
    char url[AI_URL_MAX + AI_MODEL_MAX + 48];
    char key_header[sizeof(HEADER_API_KEY_PREFIX) + AI_KEY_MAX];
    http_response_t response = { 0, NULL, 0 };

    /*
     * The model name is put into the address below. Refuse
     * a name that could change what the address means.
     */
    if (config->model[strspn(config->model, MODEL_NAME_CHARACTERS)]
            != '\0') {

        set_error(
            error_text,
            "the model name may only contain letters, digits, "
            "'.', '-' and '_'"
        );
        return AI_ERROR_CONFIG;
    }

    /*
     * Add the message of the user to the conversation. It is
     * taken out again below when the request fails, so the
     * conversation only ever holds complete exchanges.
     *
     * {"role": "user", "parts": [{"text": "..."}]}
     */
    json_t *user_message = json_pack(
        "{s:s, s:[{s:s}]}",
        "role", "user",
        "parts", "text", message
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

    snprintf(
        url,
        sizeof(url),
        "%s/v1beta/models/%s:generateContent",
        base_url,
        config->model
    );

    snprintf(
        key_header,
        sizeof(key_header),
        "%s%s",
        HEADER_API_KEY_PREFIX,
        config->api_key
    );

    /*
     * Header lines, ended by NULL.
     */
    const char *headers[] = {
        HEADER_CONTENT_TYPE,
        key_header,
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
                config,
                conversation,
                reply,
                error_text
            );
        } else {
            status = read_error(
                response.body,
                response.status,
                error_text
            );
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
 * {"role": "user" or "model", "parts": [{"text": "..."}]}
 */
static int gemini_add_text(
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
        "{s:s, s:[{s:s}]}",
        "role", from_user ? "user" : "model",
        "parts", "text", text
    );

    return json_array_append_new(conversation->messages, message);
}

/*
 * Registration of the provider. The table in ai.c refers to
 * this structure.
 */
const ai_provider_t gemini_provider = {
    .info = {
        .name = "gemini",
        .alias = NULL,
        .description = "Gemini models by Google",
        .default_model = GEMINI_DEFAULT_MODEL,
        .models = gemini_models,
        .requires_key = true,
    },
    .default_base_url = GEMINI_BASE_URL,
    .send = gemini_send,
    .add_text = gemini_add_text,
};
