/*
 * Saved chats: one JSON file per chat in
 * im_config/ai/chats/<project id>/.
 *
 * Besides the chat files, the directory of a project holds
 * a small file, index.json, with the id that the next chat will
 * receive:
 *
 *     {"next_id": 5}
 *
 * It is what keeps ids from being used again: the id of
 * a deleted chat stays behind the counter.
 *
 * The format of a chat file is described in ai/chat_store.h.
 */

#include "ai/chat_store.h"

#include "ai/ai_settings.h"
#include "ai/provider.h"
#include "storage/config_dir.h"
#include "storage/json_file.h"

#include <dirent.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

/*
 * Directory of the chats inside im_config, and the name of the
 * file with the id counter.
 */
#define CHATS_DIRECTORY AI_DIRECTORY_NAME "/chats"
#define INDEX_FILE_NAME "index.json"

/*
 * Permissions of the directories and files that hold chats:
 * the owner only. Chats can contain confidential data.
 */
#define PRIVATE_DIRECTORY_MODE 0700
#define PRIVATE_FILE_MASK 0077

/*
 * Keys used in a chat file.
 */
#define KEY_ID "id"
#define KEY_TITLE "title"
#define KEY_CREATED "created"
#define KEY_UPDATED "updated"
#define KEY_PROVIDER "provider"
#define KEY_MODEL "model"
#define KEY_TRANSCRIPT "transcript"
#define KEY_PROVIDER_STATE "provider_state"

/*
 * Keys of one message of the transcript.
 */
#define KEY_ROLE "role"
#define KEY_TIME "time"
#define KEY_TEXT "text"
#define KEY_INPUT_TOKENS "input_tokens"
#define KEY_OUTPUT_TOKENS "output_tokens"

#define ROLE_USER "user"
#define ROLE_ASSISTANT "assistant"

/*
 * Key of the id counter in index.json.
 */
#define KEY_NEXT_ID "next_id"

/*
 * Longest title taken from the first message of a chat,
 * in characters.
 */
#define AUTO_TITLE_LENGTH 48

/*
 * An open chat.
 */
struct ai_chat {
    /*
     * Project that the chat belongs to.
     */
    int project_id;

    ai_chat_info_t info;

    /*
     * JSON array with the messages as plain text.
     */
    json_t *transcript;

    /*
     * The conversation in the form of the provider. Its message
     * list is what the file stores as "provider_state".
     */
    ai_conversation_t *conversation;
};

/*
 * Builds the path of the chat directory of a project, relative
 * to im_config: "ai/chats/3".
 */
static void project_directory(int project_id, char *buffer, size_t size)
{
    snprintf(buffer, size, "%s/%d", CHATS_DIRECTORY, project_id);
}

/*
 * Builds the full path of a file in the chat directory of
 * a project.
 *
 * Returns 0 on success and -1 on error (errno is set).
 */
static int project_file_path(
    int project_id,
    const char *file_name,
    char *buffer,
    size_t size
)
{
    char relative[128];

    snprintf(
        relative,
        sizeof(relative),
        "%s/%d/%s",
        CHATS_DIRECTORY,
        project_id,
        file_name
    );

    return config_build_path(buffer, size, relative);
}

/*
 * Builds the full path of a chat file: ".../ai/chats/3/2.json".
 */
static int chat_file_path(
    int project_id,
    int id,
    char *buffer,
    size_t size
)
{
    char file_name[32];

    snprintf(file_name, sizeof(file_name), "%d.json", id);

    return project_file_path(project_id, file_name, buffer, size);
}

/*
 * Writes a JSON file that only its owner can read.
 *
 * The permissions of a new file are decided by the umask of the
 * process, so it is tightened for the duration of the write.
 *
 * Returns 0 on success and -1 on error (errno is set).
 */
static int write_private_file(const char *path, const json_t *root)
{
    mode_t original_mask = umask(PRIVATE_FILE_MASK);

    int result = json_file_write(path, root);
    int saved_errno = errno;

    umask(original_mask);

    errno = saved_errno;

    return result;
}

/*
 * Stores the current time in buffer, in UTC, in the form
 * "2026-10-13T14:02:11Z" (ISO 8601). buffer has
 * AI_CHAT_TIME_MAX characters.
 */
static void current_time(char *buffer)
{
    time_t now = time(NULL);
    struct tm utc;

    if (gmtime_r(&now, &utc) == NULL ||
        strftime(buffer, AI_CHAT_TIME_MAX, "%Y-%m-%dT%H:%M:%SZ", &utc)
            == 0) {

        buffer[0] = '\0';
    }
}

/*
 * Copies a text setting of a chat file into a buffer.
 *
 * Returns 0 on success and -1 when the value is not text or
 * does not fit.
 */
static int copy_text(const json_t *value, char *buffer, size_t size)
{
    if (!json_is_string(value) || json_string_length(value) >= size) {
        return -1;
    }

    memcpy(buffer, json_string_value(value), json_string_length(value) + 1);

    return 0;
}

/*
 * Reads the summary of a chat from the content of its file.
 *
 * root is the parsed file and expected_id the id that the file
 * name promises.
 *
 * Returns 0 on success and -1 when the content is not
 * a valid chat.
 */
static int read_info(
    const json_t *root,
    int expected_id,
    ai_chat_info_t *info
)
{
    const json_t *id = json_object_get(root, KEY_ID);
    const json_t *transcript = json_object_get(root, KEY_TRANSCRIPT);

    memset(info, 0, sizeof(*info));

    if (!json_is_integer(id) ||
        json_integer_value(id) != expected_id ||
        !json_is_array(transcript) ||
        copy_text(
            json_object_get(root, KEY_TITLE),
            info->title,
            sizeof(info->title)) != 0 ||
        copy_text(
            json_object_get(root, KEY_CREATED),
            info->created,
            sizeof(info->created)) != 0 ||
        copy_text(
            json_object_get(root, KEY_UPDATED),
            info->updated,
            sizeof(info->updated)) != 0 ||
        copy_text(
            json_object_get(root, KEY_PROVIDER),
            info->provider,
            sizeof(info->provider)) != 0 ||
        copy_text(
            json_object_get(root, KEY_MODEL),
            info->model,
            sizeof(info->model)) != 0) {

        return -1;
    }

    info->id = expected_id;
    info->message_count = json_array_size(transcript);

    return 0;
}

/*
 * Writes an open chat to its file.
 *
 * Problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
static int save_chat(const ai_chat_t *chat)
{
    char path[PATH_MAX];

    if (chat_file_path(
            chat->project_id,
            chat->info.id,
            path,
            sizeof(path)) != 0) {

        perror("chat file");
        return -1;
    }

    /*
     * "O" stores the two arrays themselves, without copying
     * them: the document shares them with the open chat.
     */
    json_t *root = json_pack(
        "{s:i, s:s, s:s, s:s, s:s, s:s, s:O, s:O}",
        KEY_ID, chat->info.id,
        KEY_TITLE, chat->info.title,
        KEY_CREATED, chat->info.created,
        KEY_UPDATED, chat->info.updated,
        KEY_PROVIDER, chat->info.provider,
        KEY_MODEL, chat->info.model,
        KEY_TRANSCRIPT, chat->transcript,
        KEY_PROVIDER_STATE, chat->conversation->messages
    );

    if (root == NULL) {
        fprintf(stderr, "Cannot save chat %d\n", chat->info.id);
        return -1;
    }

    if (write_private_file(path, root) != 0) {
        perror(path);
        json_decref(root);
        return -1;
    }

    json_decref(root);

    return 0;
}

/*
 * Compares two chat summaries by id. Used with qsort().
 */
static int compare_chats(const void *left, const void *right)
{
    const ai_chat_info_t *left_chat = left;
    const ai_chat_info_t *right_chat = right;

    return left_chat->id - right_chat->id;
}

/*
 * Reads the id of a chat from the name of its file: "12.json"
 * yields 12.
 *
 * Returns 0 when the name is not the name of a chat file, such
 * as "index.json" or a temporary file.
 */
static int id_from_file_name(const char *name)
{
    char *end = NULL;

    errno = 0;

    long id = strtol(name, &end, 10);

    if (errno != 0 ||
        end == name ||
        strcmp(end, ".json") != 0 ||
        id < 1 ||
        id > INT_MAX) {
        return 0;
    }

    return (int)id;
}

int ai_chat_list(
    int project_id,
    ai_chat_info_t *chats,
    size_t max_chats,
    size_t *chat_count
)
{
    char directory_path[PATH_MAX];
    char relative[64];

    if (chats == NULL || chat_count == NULL || max_chats == 0) {
        errno = EINVAL;
        return -1;
    }

    *chat_count = 0;

    project_directory(project_id, relative, sizeof(relative));

    if (config_build_path(
            directory_path,
            sizeof(directory_path),
            relative) != 0) {
        return -1;
    }

    DIR *directory = opendir(directory_path);

    if (directory == NULL) {
        /*
         * No directory yet: the project has no chats.
         */
        return errno == ENOENT ? 0 : -1;
    }

    int result = 0;
    struct dirent *entry;

    while ((entry = readdir(directory)) != NULL) {
        int id = id_from_file_name(entry->d_name);

        if (id == 0) {
            continue;
        }

        if (*chat_count >= max_chats) {
            errno = ENOSPC;
            result = -1;
            break;
        }

        char path[PATH_MAX];
        json_t *root = NULL;

        /*
         * A file that is not valid JSON was already reported
         * by json_file_read(). Either way it is left out.
         */
        if (chat_file_path(project_id, id, path, sizeof(path)) != 0 ||
            json_file_read(path, &root) != 0) {
            continue;
        }

        if (read_info(root, id, &chats[*chat_count]) == 0) {
            (*chat_count)++;
        } else {
            fprintf(stderr, "%s: not a valid chat file\n", path);
        }

        json_decref(root);
    }

    int saved_errno = errno;

    closedir(directory);

    errno = saved_errno;

    if (result != 0) {
        return -1;
    }

    /*
     * readdir() returns the files in no particular order.
     */
    qsort(chats, *chat_count, sizeof(chats[0]), compare_chats);

    return 0;
}

/*
 * Takes the next free chat id of a project and moves the
 * counter in index.json forward.
 *
 * When the index file is missing or unreadable, the counter
 * starts after the highest id among the existing chat files.
 *
 * Returns the id, or -1 on error (errno is set).
 */
static int take_next_id(int project_id)
{
    char path[PATH_MAX];
    json_t *root = NULL;
    int next_id = 0;

    if (project_file_path(
            project_id,
            INDEX_FILE_NAME,
            path,
            sizeof(path)) != 0) {
        return -1;
    }

    if (json_file_read(path, &root) == 0) {
        const json_t *value = json_object_get(root, KEY_NEXT_ID);

        if (json_is_integer(value) &&
            json_integer_value(value) >= 1 &&
            json_integer_value(value) < INT_MAX) {

            next_id = (int)json_integer_value(value);
        }

        json_decref(root);
    }

    /*
     * No usable counter: continue after the existing chats.
     */
    if (next_id == 0) {
        static ai_chat_info_t chats[MAX_AI_CHATS];
        size_t chat_count = 0;

        if (ai_chat_list(project_id, chats, MAX_AI_CHATS, &chat_count)
                != 0) {
            return -1;
        }

        /*
         * The list is ordered by id, so the last one is
         * the highest.
         */
        next_id = chat_count > 0 ? chats[chat_count - 1].id + 1 : 1;
    }

    /*
     * Store the counter for the chat after this one.
     */
    root = json_pack("{s:i}", KEY_NEXT_ID, next_id + 1);

    if (root == NULL) {
        errno = ENOMEM;
        return -1;
    }

    int result = write_private_file(path, root);
    int saved_errno = errno;

    json_decref(root);

    if (result != 0) {
        errno = saved_errno;
        return -1;
    }

    return next_id;
}

/*
 * Allocates an open chat with an empty transcript and an empty
 * conversation.
 *
 * Returns NULL when memory runs out (errno is ENOMEM).
 */
static ai_chat_t *allocate_chat(int project_id)
{
    ai_chat_t *chat = calloc(1, sizeof(*chat));

    if (chat == NULL) {
        errno = ENOMEM;
        return NULL;
    }

    chat->project_id = project_id;
    chat->transcript = json_array();
    chat->conversation = ai_conversation_new();

    if (chat->transcript == NULL || chat->conversation == NULL) {
        ai_chat_close(chat);
        errno = ENOMEM;
        return NULL;
    }

    return chat;
}

ai_chat_t *ai_chat_create(int project_id, const char *title)
{
    char relative[64];

    /*
     * Make sure the directory of the project exists.
     */
    project_directory(project_id, relative, sizeof(relative));

    if (config_create_directories(relative, PRIVATE_DIRECTORY_MODE) != 0) {
        return NULL;
    }

    ai_chat_t *chat = allocate_chat(project_id);

    if (chat == NULL) {
        return NULL;
    }

    int id = take_next_id(project_id);

    if (id < 0) {
        int saved_errno = errno;

        ai_chat_close(chat);
        errno = saved_errno;

        return NULL;
    }

    chat->info.id = id;

    if (title != NULL) {
        snprintf(chat->info.title, sizeof(chat->info.title), "%s", title);
    }

    current_time(chat->info.created);

    memcpy(chat->info.updated, chat->info.created, AI_CHAT_TIME_MAX);

    /*
     * Save at once, so the chat exists even when the program
     * ends before the first message.
     */
    if (save_chat(chat) != 0) {
        ai_chat_close(chat);
        errno = EIO;
        return NULL;
    }

    return chat;
}

ai_chat_t *ai_chat_open(int project_id, int id)
{
    char path[PATH_MAX];
    json_t *root = NULL;

    if (chat_file_path(project_id, id, path, sizeof(path)) != 0) {
        return NULL;
    }

    /*
     * A missing file sets errno to ENOENT; invalid JSON is
     * reported and sets it to EINVAL.
     */
    if (json_file_read(path, &root) != 0) {
        return NULL;
    }

    ai_chat_t *chat = allocate_chat(project_id);

    if (chat == NULL) {
        json_decref(root);
        errno = ENOMEM;
        return NULL;
    }

    json_t *transcript = json_object_get(root, KEY_TRANSCRIPT);
    json_t *state = json_object_get(root, KEY_PROVIDER_STATE);

    if (read_info(root, id, &chat->info) != 0 || !json_is_array(state)) {
        fprintf(stderr, "%s: not a valid chat file\n", path);
        ai_chat_close(chat);
        json_decref(root);
        errno = EINVAL;
        return NULL;
    }

    /*
     * Take over the two arrays of the file in place of the
     * empty ones made by allocate_chat(). json_incref() keeps
     * them alive after root is released.
     */
    json_decref(chat->transcript);
    chat->transcript = json_incref(transcript);

    json_decref(chat->conversation->messages);
    chat->conversation->messages = json_incref(state);

    json_decref(root);

    return chat;
}

int ai_chat_delete(int project_id, int id)
{
    char path[PATH_MAX];

    if (chat_file_path(project_id, id, path, sizeof(path)) != 0) {
        return -1;
    }

    /*
     * unlink() sets errno to ENOENT when there is no such file.
     */
    return unlink(path);
}

const ai_chat_info_t *ai_chat_info(const ai_chat_t *chat)
{
    return &chat->info;
}

ai_conversation_t *ai_chat_conversation(ai_chat_t *chat)
{
    return chat->conversation;
}

size_t ai_chat_transcript_length(const ai_chat_t *chat)
{
    return json_array_size(chat->transcript);
}

int ai_chat_transcript_get(
    const ai_chat_t *chat,
    size_t index,
    bool *from_user,
    const char **text,
    const char **time
)
{
    const json_t *message = json_array_get(chat->transcript, index);

    if (message == NULL) {
        return -1;
    }

    const char *role = json_string_value(
        json_object_get(message, KEY_ROLE)
    );
    const char *message_text = json_string_value(
        json_object_get(message, KEY_TEXT)
    );
    const char *message_time = json_string_value(
        json_object_get(message, KEY_TIME)
    );

    /*
     * Tolerate a transcript that was edited by hand: missing
     * values are shown as empty texts.
     */
    *from_user = role != NULL && strcmp(role, ROLE_USER) == 0;
    *text = message_text != NULL ? message_text : "";
    *time = message_time != NULL ? message_time : "";

    return 0;
}

int ai_chat_prepare(ai_chat_t *chat, const ai_config_t *config)
{
    /*
     * A chat without messages has nothing to carry over, and
     * a chat written by the configured provider and model
     * continues as it is.
     */
    if (json_array_size(chat->transcript) == 0 ||
        (strcmp(chat->info.provider, config->provider) == 0 &&
         strcmp(chat->info.model, config->model) == 0)) {
        return 0;
    }

    /*
     * Another provider or model: build the conversation again
     * from the plain text, in the form of the new provider.
     */
    ai_conversation_clear(chat->conversation);

    for (size_t i = 0; i < json_array_size(chat->transcript); i++) {
        bool from_user = false;
        const char *text = NULL;
        const char *time = NULL;

        ai_chat_transcript_get(chat, i, &from_user, &text, &time);

        if (ai_conversation_add_text(
                config->provider,
                chat->conversation,
                from_user,
                text) != 0) {
            return -1;
        }
    }

    return 1;
}

/*
 * Makes a title for a chat from its first message: the
 * beginning of the message, on one line.
 *
 * The message is UTF-8, so a character can take several bytes;
 * the title never ends in the middle of one.
 */
static void make_title(const char *message, char *title, size_t size)
{
    size_t length = 0;
    size_t characters = 0;

    for (size_t i = 0; message[i] != '\0'; i++) {
        unsigned char byte = (unsigned char)message[i];

        /*
         * In UTF-8 the bytes 10xxxxxx continue a character
         * that started earlier.
         */
        bool starts_character = (byte & 0xC0) != 0x80;

        if (starts_character) {

            if (characters >= AUTO_TITLE_LENGTH) {
                break;
            }

            characters++;
        }

        /*
         * Room for this byte and the terminating '\0'.
         */
        if (length + 1 >= size) {
            break;
        }

        /*
         * Line breaks and other control characters become
         * spaces, so the title stays on one line.
         */
        if (byte < 0x20 || byte == 0x7F) {
            byte = ' ';
        }

        title[length] = (char)byte;
        length++;
    }

    title[length] = '\0';
}

int ai_chat_record(
    ai_chat_t *chat,
    const ai_config_t *config,
    const char *message,
    const ai_reply_t *reply
)
{
    char now[AI_CHAT_TIME_MAX];

    current_time(now);

    /*
     * The message of the user. Its time is not known exactly;
     * the time of recording is close enough for a record.
     */
    json_t *user_entry = json_pack(
        "{s:s, s:s, s:s}",
        KEY_ROLE, ROLE_USER,
        KEY_TIME, now,
        KEY_TEXT, message
    );

    /*
     * The reply, with the model that wrote it and the tokens
     * that the exchange used.
     */
    json_t *reply_entry = json_pack(
        "{s:s, s:s, s:s, s:I, s:I, s:s}",
        KEY_ROLE, ROLE_ASSISTANT,
        KEY_TIME, now,
        KEY_MODEL, reply->model[0] != '\0' ? reply->model : config->model,
        KEY_INPUT_TOKENS, (json_int_t)reply->input_tokens,
        KEY_OUTPUT_TOKENS, (json_int_t)reply->output_tokens,
        KEY_TEXT, reply->text
    );

    /*
     * json_array_append_new() takes ownership of the entry and
     * fails for NULL, which json_pack() returns for text that
     * is not valid UTF-8.
     */
    if (json_array_append_new(chat->transcript, user_entry) != 0) {
        json_decref(reply_entry);
        fprintf(stderr, "Cannot record the message in the chat\n");
        return -1;
    }

    if (json_array_append_new(chat->transcript, reply_entry) != 0) {
        fprintf(stderr, "Cannot record the reply in the chat\n");
        return -1;
    }

    /*
     * A chat without a title is named after its first message.
     */
    if (chat->info.title[0] == '\0') {
        make_title(message, chat->info.title, sizeof(chat->info.title));
    }

    /*
     * Remember who wrote the last reply. ai_chat_prepare()
     * compares these with the configuration when the chat is
     * opened again.
     */
    snprintf(
        chat->info.provider,
        sizeof(chat->info.provider),
        "%s",
        config->provider
    );
    snprintf(
        chat->info.model,
        sizeof(chat->info.model),
        "%s",
        config->model
    );

    memcpy(chat->info.updated, now, AI_CHAT_TIME_MAX);

    chat->info.message_count = json_array_size(chat->transcript);

    return save_chat(chat);
}

void ai_chat_close(ai_chat_t *chat)
{
    if (chat == NULL) {
        return;
    }

    /*
     * Both functions accept NULL, which the members are when
     * allocate_chat() failed half-way.
     */
    json_decref(chat->transcript);
    ai_conversation_free(chat->conversation);

    free(chat);
}
