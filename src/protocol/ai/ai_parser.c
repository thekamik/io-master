/*
 * Parser of the menu of the AI shell.
 *
 * Turns one typed line into an ai_command_t and prints the help.
 * Both use the shell_commands table. The shell itself, which
 * executes the commands, is in ai_protocol.c.
 */

#include "protocol/ai/ai_parser.h"

#include <errno.h>
#include <limits.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define WHITESPACE " \t\r\n"

/*
 * How a command treats what follows its name.
 */
typedef enum {
    ARGUMENT_NONE,  /* nothing may follow */
    ARGUMENT_ID,    /* the id of a chat has to follow */
    ARGUMENT_TEXT   /* the rest of the line may follow, spaces
                       included; it may also be left out */
} argument_kind_t;

/*
 * Description of one menu command.
 */
typedef struct {
    /*
     * Word typed by the user.
     */
    const char *name;

    argument_kind_t argument_kind;

    /*
     * The command with its argument, as shown in the help.
     */
    const char *usage;

    ai_command_type_t type;

    /*
     * One line shown in the list of commands.
     */
    const char *description;
} shell_command_t;

/*
 * Table of the menu commands.
 *
 * A new command is registered by adding one entry here, a value
 * to ai_command_type_t and a case to run_menu() in ai_protocol.c.
 */
static const shell_command_t shell_commands[] = {
    {
        "help", ARGUMENT_NONE, "help",
        AI_COMMAND_HELP,
        "Show this list of commands"
    },
    {
        "status", ARGUMENT_NONE, "status",
        AI_COMMAND_STATUS,
        "Show the AI provider, the model and the state of the API key"
    },
    {
        "list", ARGUMENT_NONE, "list",
        AI_COMMAND_LIST,
        "List the saved chats of this project"
    },
    {
        "new", ARGUMENT_TEXT, "new [TITLE]",
        AI_COMMAND_NEW,
        "Start a new chat"
    },
    {
        "open", ARGUMENT_ID, "open ID",
        AI_COMMAND_OPEN,
        "Show a saved chat and continue it"
    },
    {
        "delete", ARGUMENT_ID, "delete ID",
        AI_COMMAND_DELETE,
        "Delete a saved chat for good"
    },
    {
        "exit", ARGUMENT_NONE, "exit",
        AI_COMMAND_EXIT,
        "Leave the shell and close the program"
    },
};

static const size_t shell_command_count =
    sizeof(shell_commands) / sizeof(shell_commands[0]);

void ai_print_shell_help(void)
{
    printf("Commands:\n");

    for (size_t i = 0; i < shell_command_count; i++) {
        printf(
            "  %-14s %s\n",
            shell_commands[i].usage,
            shell_commands[i].description
        );
    }

    printf(
        "\n"
        "Inside a chat every line you type is sent to the model.\n"
        "Press Ctrl+C to leave the chat and return to this menu;\n"
        "the chat is saved after every reply.\n"
        "\n"
        "The provider, the model and the API key are set with the\n"
        "'ai-config' command of the program.\n"
    );
}

/*
 * Converts the id argument of a command.
 *
 * Returns the id, or 0 when the text is not a number greater
 * than zero.
 */
static int parse_id(const char *text)
{
    char *end = NULL;

    errno = 0;

    long id = strtol(text, &end, 10);

    if (errno != 0 ||
        end == text ||
        *end != '\0' ||
        id < 1 ||
        id > INT_MAX) {
        return 0;
    }

    return (int)id;
}

int ai_parse_line(char *line, ai_command_t *command)
{
    if (line == NULL || command == NULL) {
        fprintf(stderr, "Internal error: invalid shell line\n");
        return -1;
    }

    command->type = AI_COMMAND_EMPTY;
    command->text = NULL;
    command->id = 0;

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
     * Split the line into the command name and the rest:
     * end the name at the first space and skip the spaces
     * after it.
     */
    char *rest = start + strcspn(start, WHITESPACE);

    if (*rest != '\0') {
        *rest = '\0';
        rest++;
        rest += strspn(rest, WHITESPACE);
    }

    /*
     * Look the name up in the command table.
     */
    const shell_command_t *entry = NULL;

    for (size_t i = 0; i < shell_command_count; i++) {

        if (strcmp(start, shell_commands[i].name) == 0) {
            entry = &shell_commands[i];
            break;
        }
    }

    if (entry == NULL) {
        fprintf(
            stderr,
            "Unknown command: '%s' (type 'help' to list commands)\n",
            start
        );
        return -1;
    }

    bool has_rest = *rest != '\0';

    /*
     * Check the rest of the line against what the command
     * expects.
     */
    switch (entry->argument_kind) {

    case ARGUMENT_NONE:
        if (has_rest) {
            fprintf(
                stderr,
                "Command '%s' takes no arguments\n",
                entry->name
            );
            return -1;
        }
        break;

    case ARGUMENT_ID:
        command->id = has_rest ? parse_id(rest) : 0;

        if (command->id == 0) {
            fprintf(
                stderr,
                "Command '%s' needs the ID of a chat, for example: "
                "%s 1 (see the IDs with: list)\n",
                entry->name,
                entry->name
            );
            return -1;
        }
        break;

    case ARGUMENT_TEXT:
        command->text = has_rest ? rest : NULL;
        break;
    }

    command->type = entry->type;

    return 0;
}
