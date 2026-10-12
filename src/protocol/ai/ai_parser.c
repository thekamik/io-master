/*
 * Parser of the interactive AI shell.
 *
 * Decides whether a typed line is a message for the model or
 * a command, and prints the help. The commands are described by
 * the shell_commands table; the shell itself, which executes
 * them, is in ai_protocol.c.
 */

#include "protocol/ai/ai_parser.h"

#include <stdio.h>
#include <string.h>

#define WHITESPACE " \t\r\n"

/*
 * Character that marks a line as a command.
 */
#define COMMAND_PREFIX '/'

/*
 * Description of one shell command.
 */
typedef struct {
    /*
     * Word typed after the '/'.
     */
    const char *name;

    ai_command_type_t type;

    /*
     * One line shown in the list of commands.
     */
    const char *description;
} shell_command_t;

/*
 * Table of the AI shell commands.
 *
 * A new command is registered by adding one entry here, a value
 * to ai_command_type_t and a case to run_shell() in
 * ai_protocol.c.
 */
static const shell_command_t shell_commands[] = {
    {
        "help",
        AI_COMMAND_HELP,
        "Show this list of commands"
    },
    {
        "new",
        AI_COMMAND_NEW,
        "Start a new conversation; the model forgets the old one"
    },
    {
        "status",
        AI_COMMAND_STATUS,
        "Show the provider, the model and the state of the API key"
    },
    {
        "key",
        AI_COMMAND_KEY,
        "Store the API key in the keyring of the system"
    },
    {
        "exit",
        AI_COMMAND_EXIT,
        "Leave the shell and close the program"
    },
};

static const size_t shell_command_count =
    sizeof(shell_commands) / sizeof(shell_commands[0]);

void ai_print_shell_help(void)
{
    printf(
        "Type a message and press Enter to send it to the model.\n"
        "A line that starts with '%c' is a command:\n"
        "\n",
        COMMAND_PREFIX
    );

    for (size_t i = 0; i < shell_command_count; i++) {
        printf(
            "  %c%-10s %s\n",
            COMMAND_PREFIX,
            shell_commands[i].name,
            shell_commands[i].description
        );
    }

    printf("\nPress Ctrl+C to cancel a request that is running.\n");
}

int ai_parse_line(char *line, ai_command_t *command)
{
    if (line == NULL || command == NULL) {
        fprintf(stderr, "Internal error: invalid shell line\n");
        return -1;
    }

    command->type = AI_COMMAND_EMPTY;
    command->text = NULL;

    /*
     * Remove the spaces and the line break around the text:
     * move the start forward and cut the end off.
     */
    char *start = line + strspn(line, WHITESPACE);
    size_t length = strlen(start);

    while (length > 0 && strchr(WHITESPACE, start[length - 1]) != NULL) {
        length--;
    }

    start[length] = '\0';

    if (length == 0) {
        return 0;
    }

    /*
     * Anything that does not start with '/' is a message.
     */
    if (start[0] != COMMAND_PREFIX) {
        command->type = AI_COMMAND_MESSAGE;
        command->text = start;
        return 0;
    }

    /*
     * A command: the word after the '/'. Commands take no
     * arguments, so the whole rest of the line has to be
     * the name.
     */
    const char *name = start + 1;

    for (size_t i = 0; i < shell_command_count; i++) {

        if (strcmp(name, shell_commands[i].name) == 0) {
            command->type = shell_commands[i].type;
            return 0;
        }
    }

    fprintf(
        stderr,
        "Unknown command: '%s' (type '%chelp' to list commands)\n",
        start,
        COMMAND_PREFIX
    );

    return -1;
}
