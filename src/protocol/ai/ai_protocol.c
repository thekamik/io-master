/*
 * AI "protocol": chats with an AI model.
 *
 * The shell of this protocol has two levels:
 *
 *     the menu   Manages the saved chats of the project: list
 *                them, start a new one, open one by its id,
 *                delete one. Its commands are parsed by
 *                ai_parser.c.
 *
 *     a chat     Entered from the menu with "new" or "open".
 *                Every typed line is a message for the model.
 *                Ctrl+C leaves the chat and returns to the menu.
 *
 * What the chat talks to (provider, model, API key) is the AI
 * configuration of the program, edited with the "ai-config"
 * command (cli/ai_config.c). The chats are stored by
 * ai/chat_store.c, one file per chat, and are saved after
 * every reply.
 *
 * The file is organized in this order:
 *
 *     shell state   ai_shell_t
 *     protocol_t    ai_init(), ai_print_help()
 *     output        printing text that came from outside
 *     chat          run_chat(): the conversation itself
 *     menu          one menu_*() function per menu command
 *     menu loop     run_menu(): reads a line, parses it and
 *                   executes the command
 *     entry point   ai_run() and the ai_protocol structure
 *
 * Ctrl+C
 * ------
 * The meaning of Ctrl+C depends on where the user is:
 *
 *     in the menu                  starts a new line; the shell
 *                                  stays open (leave with "exit")
 *     in a chat, at the prompt     leaves the chat
 *     in a chat, while a reply     cancels the request; the user
 *     is being awaited             stays in the chat
 */

// TODO: Let the device protocols use the AI module, for example
//       to explain data read from a UART or the services of
//       a Bluetooth device.
// TODO: Messages of several lines.

#include "protocol/ai/ai_protocol.h"

#include "ai/ai.h"
#include "ai/ai_settings.h"
#include "ai/chat_store.h"
#include "ai/keyring.h"
#include "cli/ai_config.h"
#include "protocol/ai/ai_parser.h"
#include "storage/json_file.h"

#include <errno.h>
#include <limits.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/*
 * Prompts of the two levels of the shell.
 */
#define MENU_PROMPT "ai> "
#define CHAT_PROMPT "you> "

/*
 * Set by Ctrl+C (SIGINT). What it leads to depends on where the
 * user is; see the top of this file.
 */
static volatile sig_atomic_t interrupted = 0;

/*
 * SIGINT handler. It only sets the flag.
 */
static void handle_signal(int signal) {
    (void)signal; // Unused parameter
    interrupted = 1;
}

/*
 * State of the interactive shell.
 */
typedef struct {
    /*
     * The AI configuration of the program, loaded when the shell
     * starts. Its config.api_key is filled from the keyring.
     */
    ai_settings_t settings;

    /*
     * Id of the project. The saved chats belong to it.
     */
    int project_id;

    /*
     * Buffer for typed lines, managed by getline(): it grows to
     * fit the line, so a message can be as long as the user
     * likes. Shared by the menu and the chat.
     */
    char *line;
    size_t line_capacity;
} ai_shell_t;

/*
 * Tells whether the shell can send requests: the provider needs
 * no API key, or the key was loaded.
 */
static bool is_ready(const ai_shell_t *shell)
{
    return !shell->settings.requires_key ||
           shell->settings.config.api_key[0] != '\0';
}

/*
 * Tells the user how to configure the AI integration.
 */
static void print_config_hint(void)
{
    printf(
        "Leave the shell and set up the AI with: "
        "io-master " AI_CONFIG_COMMAND "\n"
    );
}

/*
 * protocol_t.init
 *
 * Called once when an AI project is added. An AI project has no
 * settings of its own, but every project needs a settings file
 * (a project without one counts as damaged), so an empty JSON
 * object is written.
 */
static int ai_init(const project_info_t *project)
{
    char path[PATH_MAX];

    if (project_create_settings_directory() != 0 ||
        project_get_settings_path(
            project->id,
            path,
            sizeof(path)) != 0) {

        perror("project settings");
        return -1;
    }

    /*
     * An existing file is left untouched.
     */
    if (access(path, F_OK) != 0) {
        json_t *root = json_object();

        if (root == NULL || json_file_write(path, root) != 0) {
            perror(path);
            json_decref(root);
            return -1;
        }

        json_decref(root);
    }

    printf(
        "The chats use the AI configuration of the program. "
        "Set it up with: io-master " AI_CONFIG_COMMAND "\n"
    );

    return 0;
}

/*
 * protocol_t.print_help
 *
 * Printed by "run --help" for an AI project.
 */
static void ai_print_help(
    const char *program_name,
    const project_info_t *project
)
{
    printf(
        "Project %d: %s (ai)\n"
        "\n"
        "Starts a shell for chatting with an AI model. The chats\n"
        "of the project are saved, one file per chat, and can be\n"
        "listed, continued and deleted.\n"
        "\n"
        "Usage:\n"
        "  %s run\n"
        "\n"
        "The provider, the model and the API key are not part of\n"
        "the project. They are set once for the whole program with:\n"
        "  %s " AI_CONFIG_COMMAND "\n"
        "\n"
        "Saved chats are plain text files. They are readable only\n"
        "by you, but they are not encrypted.\n"
        "\n",
        project->id,
        project->name,
        program_name,
        program_name
    );

    /*
     * The same text that "help" prints inside the shell.
     */
    ai_print_shell_help();
}

/*
 * Prints text that came from outside (a reply of a model, or
 * a chat file) on the terminal.
 *
 * Control characters other than line breaks and tabs are
 * replaced with '?'. Without this the text could contain
 * terminal control sequences and, for example, hide text or
 * change the title of the window.
 */
static void print_untrusted(const char *text)
{
    for (size_t i = 0; text[i] != '\0'; i++) {
        unsigned char byte = (unsigned char)text[i];

        /*
         * Control characters: below space, and DEL. Bytes of
         * 0x80 and above belong to UTF-8 characters and pass.
         */
        if ((byte < 0x20 && byte != '\n' && byte != '\t') ||
            byte == 0x7F) {
            byte = '?';
        }

        putchar(byte);
    }
}

/*
 * Reports a failed request.
 *
 * status is the result of ai_send() and error_text the
 * explanation that came with it.
 */
static void report_ai_error(ai_status_t status, const char *error_text)
{
    switch (status) {

    case AI_ERROR_CANCELLED:
        fprintf(
            stderr,
            "\nCancelled. The message was not added to the chat.\n"
        );
        break;

    case AI_ERROR_AUTH:
        fprintf(
            stderr,
            "The service rejected the API key: %s\n",
            error_text
        );
        print_config_hint();
        break;

    case AI_ERROR_NETWORK:
        fprintf(
            stderr,
            "Cannot reach the AI service: %s\n",
            error_text
        );
        break;

    case AI_ERROR_RATE_LIMIT:
        fprintf(
            stderr,
            "Too many requests, try again in a moment: %s\n",
            error_text
        );
        break;

    case AI_ERROR_SERVER:
        fprintf(
            stderr,
            "The AI service has a problem, try again later: %s\n",
            error_text
        );
        break;

    case AI_ERROR_REFUSAL:
        fprintf(
            stderr,
            "No reply: %s. The message was not added to the "
            "chat; try to phrase it differently.\n",
            error_text
        );
        break;

    case AI_ERROR_REQUEST:
    case AI_ERROR_CONFIG:
        fprintf(
            stderr,
            "The AI service did not accept the request: %s\n",
            error_text
        );
        print_config_hint();
        break;

    default:
        fprintf(stderr, "The request failed: %s\n", error_text);
        break;
    }
}

/*
 * Sends one message of a chat to the model, prints the reply
 * and records the exchange in the chat file.
 *
 * Ctrl+C while the reply is awaited cancels the request.
 */
static void send_message(
    ai_shell_t *shell,
    ai_chat_t *chat,
    const char *message
)
{
    ai_reply_t reply;
    char error_text[AI_ERROR_TEXT_MAX];

    printf("Waiting for the reply (press Ctrl+C to cancel)...\n");
    fflush(stdout);

    /*
     * Forget a Ctrl+C that was pressed earlier.
     */
    interrupted = 0;

    ai_status_t status = ai_send(
        &shell->settings.config,
        ai_chat_conversation(chat),
        message,
        &interrupted,
        &reply,
        error_text
    );

    if (status != AI_OK) {
        report_ai_error(status, error_text);
        return;
    }

    /*
     * The reply, between empty lines.
     */
    printf("\n");

    if (reply.text[0] != '\0') {
        print_untrusted(reply.text);
    } else {
        printf("(the reply contains no text)");
    }

    printf("\n\n");

    if (reply.truncated) {
        printf(
            "(the reply was cut off by the max_tokens limit of "
            "the AI configuration)\n"
        );
    }

    /*
     * One line of facts about the reply: the model that wrote
     * it, which differs from the configured one when a fallback
     * model answered, and the tokens used.
     */
    printf("[");
    print_untrusted(reply.model);

    if (reply.input_tokens >= 0 && reply.output_tokens >= 0) {
        printf(
            ", %ld tokens in, %ld tokens out",
            reply.input_tokens,
            reply.output_tokens
        );
    }

    printf("]\n");

    /*
     * Write the exchange to the chat file. A failure was
     * reported; the chat goes on in memory.
     */
    if (ai_chat_record(
            chat,
            &shell->settings.config,
            message,
            &reply) != 0) {

        fprintf(
            stderr,
            "Warning: this exchange could not be saved to the "
            "chat file\n"
        );
    }

    ai_reply_free(&reply);
}

/*
 * Prints what was said in a chat so far, so the user can pick
 * up where the chat stopped.
 */
static void print_transcript(const ai_chat_t *chat)
{
    for (size_t i = 0; i < ai_chat_transcript_length(chat); i++) {
        bool from_user = false;
        const char *text = NULL;
        const char *time = NULL;

        ai_chat_transcript_get(chat, i, &from_user, &text, &time);

        if (from_user) {
            printf(CHAT_PROMPT);
            print_untrusted(text);
            printf("\n");
        } else {
            printf("\n");
            print_untrusted(text);
            printf("\n\n");
        }
    }
}

/*
 * Removes the line break and the spaces around a typed line.
 *
 * Returns the start of the text inside line; the text may
 * be empty.
 */
static char *trim_line(char *line)
{
    static const char whitespace[] = " \t\r\n";

    char *start = line + strspn(line, whitespace);
    size_t length = strlen(start);

    while (length > 0 && strchr(whitespace, start[length - 1]) != NULL) {
        length--;
    }

    start[length] = '\0';

    return start;
}

/*
 * A chat: every typed line is sent to the model.
 *
 * Takes ownership of chat and closes it when the user leaves
 * with Ctrl+C (or Ctrl+D). Nothing has to be saved at that
 * point, because the chat file is written after every reply.
 */
static void run_chat(ai_shell_t *shell, ai_chat_t *chat)
{
    const ai_chat_info_t *info = ai_chat_info(chat);

    printf("Chat %d", info->id);

    if (info->title[0] != '\0') {
        printf(": ");
        print_untrusted(info->title);
    }

    printf(
        "\n"
        "Type a message and press Enter to send it. "
        "Press Ctrl+C to leave the chat.\n\n"
    );

    /*
     * A saved chat: show what was said, and make it fit the
     * configured provider and model.
     */
    if (ai_chat_transcript_length(chat) > 0) {
        print_transcript(chat);

        int prepared = ai_chat_prepare(chat, &shell->settings.config);

        if (prepared > 0) {
            printf(
                "(This chat was written by %s. It continues with "
                "%s, which receives the text above.)\n\n",
                info->model,
                shell->settings.config.model
            );
        } else if (prepared < 0) {
            fprintf(
                stderr,
                "This chat cannot be continued with the "
                "configured provider.\n"
            );
            ai_chat_close(chat);
            return;
        }
    }

    for (;;) {
        printf(CHAT_PROMPT);
        fflush(stdout);

        if (getline(&shell->line, &shell->line_capacity, stdin) < 0) {
            /*
             * Ctrl+C at the prompt, or the end of input
             * (Ctrl+D): leave the chat.
             */
            clearerr(stdin);
            printf("\n");
            break;
        }

        const char *message = trim_line(shell->line);

        if (message[0] == '\0') {
            continue;
        }

        send_message(shell, chat, message);
    }

    /*
     * info belongs to the chat, so report before closing it.
     */
    printf(
        "Left chat %d. It is saved with %zu messages.\n",
        info->id,
        info->message_count
    );

    ai_chat_close(chat);
}

/*
 * Menu command "status".
 *
 * Shows what the chats talk to and the state of the API key.
 * The key itself is never shown.
 */
static void menu_status(const ai_shell_t *shell)
{
    const ai_settings_t *settings = &shell->settings;
    const ai_config_t *config = &settings->config;

    printf(
        "Provider: %s\n"
        "Model:    %s\n",
        config->provider,
        config->model
    );

    if (config->base_url[0] != '\0') {
        printf("Address:  %s\n", config->base_url);
    }

    if (!settings->requires_key) {
        printf("API key:  not needed\n");
    } else if (is_ready(shell)) {
        printf("API key:  loaded from the keyring\n");
    } else {
        printf("API key:  missing\n");
    }
}

/*
 * Menu command "list".
 *
 * Prints the table of the saved chats of the project.
 */
static void menu_list(const ai_shell_t *shell)
{
    /*
     * static: the array is too large for the stack. The shell
     * runs one command at a time, so sharing it is safe.
     */
    static ai_chat_info_t chats[MAX_AI_CHATS];
    size_t chat_count = 0;

    if (ai_chat_list(
            shell->project_id,
            chats,
            MAX_AI_CHATS,
            &chat_count) != 0) {

        perror("Cannot list the chats");
        return;
    }

    if (chat_count == 0) {
        printf("No saved chats. Start one with: new\n");
        return;
    }

    /*
     * Table header.
     */
    printf(
        "%4s  %-16s  %4s  %-20s  %s\n",
        "ID",
        "UPDATED (UTC)",
        "MSGS",
        "MODEL",
        "TITLE"
    );

    /*
     * One row per chat.
     */
    for (size_t i = 0; i < chat_count; i++) {
        const ai_chat_info_t *chat = &chats[i];

        /*
         * Show the time as "2026-10-13 14:09": the first 16
         * characters of "2026-10-13T14:09:40Z", with a space
         * in place of the 'T'.
         */
        char updated[17];

        snprintf(updated, sizeof(updated), "%.16s", chat->updated);

        if (strlen(updated) > 10) {
            updated[10] = ' ';
        }

        printf(
            "%4d  %-16s  %4zu  %-20.20s  ",
            chat->id,
            updated,
            chat->message_count,
            chat->model[0] != '\0' ? chat->model : "-"
        );

        /*
         * Titles come from files and from typed text.
         */
        print_untrusted(chat->title[0] != '\0' ? chat->title : "(empty)");
        printf("\n");
    }
}

/*
 * Checks that a chat can be started: without an API key no
 * message could be sent.
 *
 * Returns true when the shell is ready; otherwise explains what
 * to do and returns false.
 */
static bool check_ready(const ai_shell_t *shell)
{
    if (is_ready(shell)) {
        return true;
    }

    fprintf(
        stderr,
        "No API key is stored for provider %s.\n",
        shell->settings.config.provider
    );
    print_config_hint();

    return false;
}

/*
 * Menu command "new".
 *
 * Creates a chat and enters it. title may be NULL; the chat is
 * then named after its first message.
 */
static void menu_new(ai_shell_t *shell, const char *title)
{
    if (!check_ready(shell)) {
        return;
    }

    if (title != NULL && strlen(title) >= AI_CHAT_TITLE_MAX) {
        fprintf(
            stderr,
            "The title is too long (at most %d characters)\n",
            AI_CHAT_TITLE_MAX - 1
        );
        return;
    }

    /*
     * title points into the line buffer of the shell. The chat
     * copies it here, before the buffer is used again.
     */
    ai_chat_t *chat = ai_chat_create(shell->project_id, title);

    if (chat == NULL) {
        perror("Cannot create the chat");
        return;
    }

    run_chat(shell, chat);
}

/*
 * Menu command "open".
 *
 * Opens a saved chat, shows it and enters it.
 */
static void menu_open(ai_shell_t *shell, int id)
{
    if (!check_ready(shell)) {
        return;
    }

    ai_chat_t *chat = ai_chat_open(shell->project_id, id);

    if (chat == NULL) {

        if (errno == ENOENT) {
            fprintf(
                stderr,
                "There is no chat with ID %d (see the IDs with: "
                "list)\n",
                id
            );
        } else if (errno != EINVAL) {
            /*
             * EINVAL: a damaged file, already reported.
             */
            perror("Cannot open the chat");
        }

        return;
    }

    run_chat(shell, chat);
}

/*
 * Menu command "delete".
 *
 * Deletes a saved chat after the user confirms it. The file is
 * removed for good; there is no way to bring the chat back.
 */
static void menu_delete(ai_shell_t *shell, int id)
{
    /*
     * Open the chat first: to check that it exists and to show
     * its title in the question.
     */
    ai_chat_t *chat = ai_chat_open(shell->project_id, id);

    if (chat == NULL && errno == ENOENT) {
        fprintf(
            stderr,
            "There is no chat with ID %d (see the IDs with: list)\n",
            id
        );
        return;
    }

    /*
     * A damaged chat file (chat is NULL here) can still
     * be deleted.
     */
    printf("Delete chat %d", id);

    if (chat != NULL) {
        const ai_chat_info_t *info = ai_chat_info(chat);

        printf(" \"");
        print_untrusted(info->title);
        printf("\" with %zu messages", info->message_count);
    }

    printf("? This cannot be undone. [y/N]: ");
    fflush(stdout);

    ai_chat_close(chat);

    /*
     * Only an answer that starts with 'y' or 'Y' deletes.
     * Anything else, including Ctrl+C, keeps the chat.
     */
    bool confirmed = false;

    if (getline(&shell->line, &shell->line_capacity, stdin) < 0) {
        clearerr(stdin);
        printf("\n");
    } else {
        const char *answer = trim_line(shell->line);

        confirmed = answer[0] == 'y' || answer[0] == 'Y';
    }

    if (!confirmed) {
        printf("Chat %d was kept.\n", id);
        return;
    }

    if (ai_chat_delete(shell->project_id, id) != 0) {
        perror("Cannot delete the chat");
        return;
    }

    printf("Chat %d was deleted.\n", id);
}

/*
 * The menu.
 *
 * Runs until the "exit" command or the end of input (Ctrl+D).
 */
static void run_menu(ai_shell_t *shell)
{
    printf("Type 'help' to list commands\n");

    for (;;) {
        ai_command_t command;

        /*
         * Show the prompt and wait for one line.
         */
        printf(MENU_PROMPT);
        fflush(stdout);

        errno = 0;

        if (getline(&shell->line, &shell->line_capacity, stdin) < 0) {

            /*
             * Ctrl+C in the menu: start a new line and keep
             * the shell open.
             */
            if (ferror(stdin) && errno == EINTR) {
                clearerr(stdin);
                printf("\n");
                continue;
            }

            /*
             * End of input (Ctrl+D).
             */
            printf("\n");
            return;
        }

        /*
         * Turn the line into a command. Errors were already
         * reported by the parser, so just show the prompt again.
         */
        if (ai_parse_line(shell->line, &command) != 0) {
            continue;
        }

        /*
         * Execute the command.
         */
        switch (command.type) {

        case AI_COMMAND_EMPTY:
            break;

        case AI_COMMAND_HELP:
            ai_print_shell_help();
            break;

        case AI_COMMAND_STATUS:
            menu_status(shell);
            break;

        case AI_COMMAND_LIST:
            menu_list(shell);
            break;

        case AI_COMMAND_NEW:
            menu_new(shell, command.text);
            break;

        case AI_COMMAND_OPEN:
            menu_open(shell, command.id);
            break;

        case AI_COMMAND_DELETE:
            menu_delete(shell, command.id);
            break;

        case AI_COMMAND_EXIT:
            return;
        }
    }
}

/*
 * Reads the API key of the configured provider from the keyring.
 *
 * A missing key does not stop the shell from starting: saved
 * chats can still be listed and deleted. Other problems are
 * reported on stderr.
 */
static void load_key(ai_shell_t *shell)
{
    char error_text[KEYRING_ERROR_TEXT_MAX];

    if (ai_settings_load_key(&shell->settings, error_text) == 0) {
        return;
    }

    if (errno == ENOENT) {
        printf(
            "No API key is stored for provider %s.\n",
            shell->settings.config.provider
        );
        print_config_hint();
    } else if (errno == ENOSPC) {
        fprintf(
            stderr,
            "The key stored in the keyring is too long to be "
            "an API key\n"
        );
    } else {
        fprintf(
            stderr,
            "Cannot read the API key from the keyring: %s\n",
            error_text
        );
    }
}

/*
 * protocol_t.run
 *
 * Called by the "run" command for an AI project. Loads the AI
 * configuration and the API key, runs the menu, and wipes the
 * key when the shell ends.
 */
static int ai_run(const project_info_t *project)
{
    /*
     * static: the configuration with its buffers is large.
     */
    static ai_shell_t shell;

    memset(&shell, 0, sizeof(shell));

    shell.project_id = project->id;

    if (ai_settings_load(&shell.settings) != 0) {
        return -1;
    }

    /*
     * Handle Ctrl+C (SIGINT).
     *
     * sa_flags has no SA_RESTART, so Ctrl+C also interrupts
     * waiting for a line at a prompt.
     */
    struct sigaction sa = {0};

    sa.sa_handler = handle_signal;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;

    if (sigaction(SIGINT, &sa, NULL) != 0) {
        perror("sigaction");
        return -1;
    }

    /*
     * Fetch the API key from the keyring and show where the
     * project stands: what the chats talk to and which chats
     * are saved.
     */
    load_key(&shell);
    menu_status(&shell);
    printf("\n");
    menu_list(&shell);
    printf("\n");

    /*
     * Hand control to the menu until "exit".
     */
    run_menu(&shell);

    /*
     * The session is over: wipe the API key from memory.
     */
    free(shell.line);
    explicit_bzero(&shell, sizeof(shell));

    return 0;
}

/*
 * Registration of the AI protocol. The table
 * in protocol/protocol.c refers to this structure.
 */
const protocol_t ai_protocol = {
    .name = "ai",
    .description = "Chats with an AI model (set up with 'ai-config')",
    .init = ai_init,
    .print_help = ai_print_help,
    .run = ai_run,
};
