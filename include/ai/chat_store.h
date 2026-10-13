/*
 * Saved chats.
 *
 * Every chat with an AI model is kept in its own file, so it can
 * be continued later and attached to the documentation of the
 * work that was done:
 *
 *     im_config/ai/chats/<project id>/<chat id>.json
 *
 * The chats belong to a project. A chat is identified by
 * a number that is unique within its project and is never used
 * again after the chat is deleted.
 *
 * Format of a chat file:
 *
 *     {
 *         "id": 2,
 *         "title": "HDLC framing question",
 *         "created": "2026-10-13T14:02:11Z",
 *         "updated": "2026-10-13T14:09:40Z",
 *         "provider": "anthropic",
 *         "model": "claude-haiku-5-5",
 *         "transcript": [
 *             {"role": "user", "time": "2026-10-13T14:02:11Z",
 *              "text": "What does 0x7E mark in HDLC?"},
 *             {"role": "assistant", "time": "2026-10-13T14:02:15Z",
 *              "model": "claude-haiku-5-5",
 *              "input_tokens": 21, "output_tokens": 18,
 *              "text": "It is the flag sequence..."}
 *         ],
 *         "provider_state": [ ... ]
 *     }
 *
 * The conversation is stored twice, for two purposes:
 *
 *   "transcript"      What was said, as plain text with the time
 *                     of every message. This is the record meant
 *                     to be read, and it does not depend on the
 *                     AI provider.
 *
 *   "provider_state"  The same conversation in the JSON form of
 *                     the provider, including parts that are not
 *                     shown, such as the signed notes of a model.
 *                     The provider needs them to continue the
 *                     chat exactly where it stopped.
 *
 * Plain text on purpose
 * ---------------------
 * The files are plain, readable text by design: a chat is
 * a record of work that was carried out and has to be available
 * as evidence, for example in a quality management audit. The
 * files and their directory are readable only by the user who
 * owns them, but they are not encrypted. Whoever uses the program
 * has to treat them like any other record that may contain
 * confidential data.
 */

// TODO: Optional encryption of the chat files at rest.

#ifndef AI_CHAT_STORE_H
#define AI_CHAT_STORE_H

#include <stdbool.h>
#include <stddef.h>

#include "ai/ai.h"

/*
 * Sizes of the buffers in ai_chat_info_t and the default size of
 * the chat array used by the callers.
 */
#define AI_CHAT_TITLE_MAX 64
#define AI_CHAT_TIME_MAX 24      /* "2026-10-13T14:02:11Z" */
#define MAX_AI_CHATS 256

/*
 * Summary of one saved chat.
 */
typedef struct {
    int id;
    char title[AI_CHAT_TITLE_MAX];

    /*
     * When the chat was created and last changed, in UTC, in the
     * form "2026-10-13T14:02:11Z".
     */
    char created[AI_CHAT_TIME_MAX];
    char updated[AI_CHAT_TIME_MAX];

    /*
     * Provider and model that wrote the last reply. Empty for
     * a chat without messages.
     */
    char provider[AI_PROVIDER_NAME_MAX];
    char model[AI_MODEL_MAX];

    /*
     * Number of messages in the transcript. A question and its
     * reply are two messages.
     */
    size_t message_count;
} ai_chat_info_t;

/*
 * An open chat. Created by ai_chat_create() or ai_chat_open()
 * and released by ai_chat_close().
 */
typedef struct ai_chat ai_chat_t;

/*
 * Lists the chats of a project, ordered by id.
 *
 * chats is an array of max_chats elements provided by the
 * caller. On success *chat_count is the number of elements that
 * were filled. A project without chats yields zero. Files that
 * cannot be read as a chat are reported on stderr and left out.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOSPC when the project has more
 *        than max_chats chats)
 */
int ai_chat_list(
    int project_id,
    ai_chat_info_t *chats,
    size_t max_chats,
    size_t *chat_count
);

/*
 * Creates a new, empty chat in a project and saves it.
 *
 * title may be NULL or empty; the chat then takes its title from
 * the first message. The chat receives the next free id.
 *
 * Returns the open chat, or NULL on error (errno is set).
 */
ai_chat_t *ai_chat_create(int project_id, const char *title);

/*
 * Opens a saved chat.
 *
 * Returns the open chat, or NULL on error (errno is set; ENOENT
 * when the project has no chat with that id, EINVAL when the
 * file is not a valid chat, which is also reported on stderr).
 */
ai_chat_t *ai_chat_open(int project_id, int id);

/*
 * Deletes a saved chat. The file is removed for good.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when there is no such chat)
 */
int ai_chat_delete(int project_id, int id);

/*
 * Returns the summary of an open chat.
 */
const ai_chat_info_t *ai_chat_info(const ai_chat_t *chat);

/*
 * Returns the conversation of an open chat, to be passed to
 * ai_send(). It belongs to the chat and is released with it.
 */
ai_conversation_t *ai_chat_conversation(ai_chat_t *chat);

/*
 * Makes an open chat ready to be continued with the given
 * configuration.
 *
 * A chat can only be continued as it is by the provider and
 * model that wrote it. When the configuration names another
 * provider or model, the conversation is built again from the
 * plain text of the transcript; the hidden parts of the old
 * replies are then left behind.
 *
 * Returns:
 *   1  - the conversation was built again from the transcript
 *   0  - the chat continues as it is
 *  -1  - error
 */
int ai_chat_prepare(ai_chat_t *chat, const ai_config_t *config);

/*
 * Records one exchange in an open chat and saves the chat.
 *
 * Call it after a successful ai_send() on the conversation of
 * the chat: message is what the user wrote and reply what the
 * model answered. Both are added to the transcript with the
 * current time, and the file is written.
 *
 * Problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
int ai_chat_record(
    ai_chat_t *chat,
    const ai_config_t *config,
    const char *message,
    const ai_reply_t *reply
);

/*
 * Access to the transcript of an open chat.
 *
 * ai_chat_transcript_get() reads the message at the given
 * position. *from_user tells who wrote it; *text and *time point
 * to texts that belong to the chat and stay valid until the chat
 * is changed or closed.
 *
 * Returns 0 on success and -1 when the index is out of range.
 */
size_t ai_chat_transcript_length(const ai_chat_t *chat);

int ai_chat_transcript_get(
    const ai_chat_t *chat,
    size_t index,
    bool *from_user,
    const char **text,
    const char **time
);

/*
 * Releases an open chat. NULL is accepted. The chat was saved
 * by the last ai_chat_record(), so nothing is written here.
 */
void ai_chat_close(ai_chat_t *chat);

#endif
