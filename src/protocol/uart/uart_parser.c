#include "protocol/uart/uart_parser.h"

#include <stdio.h>
#include <string.h>

#define TOKEN_DELIMITERS " \t\r\n"

typedef struct {
    const char *name;

    /*
     * Name of the required argument, or NULL when the command
     * takes no arguments.
     */
    const char *argument_name;

    uart_command_type_t type;
    const char *description;
} shell_command_t;

/*
 * Table of the UART shell commands.
 *
 * A new command is registered by adding one entry here.
 */
static const shell_command_t shell_commands[] = {
    {
        "help",
        NULL,
        UART_COMMAND_HELP,
        "Show this list of commands"
    },
    {
        "scan",
        NULL,
        UART_COMMAND_SCAN,
        "List available serial devices"
    },
    {
        "select",
        "PORT",
        UART_COMMAND_SELECT,
        "Select the serial port, for example /dev/ttyACM0"
    },
    {
        "permission",
        NULL,
        UART_COMMAND_PERMISSION,
        "Get access to the selected port (asks for sudo password)"
    },
    {
        "read",
        NULL,
        UART_COMMAND_READ,
        "Read from the serial port until Ctrl+C is pressed"
    },
    {
        "exit",
        NULL,
        UART_COMMAND_EXIT,
        "Leave the shell and close the program"
    },
};

static const size_t shell_command_count =
    sizeof(shell_commands) / sizeof(shell_commands[0]);

void uart_print_shell_help(void)
{
    printf("Commands:\n");

    for (size_t i = 0; i < shell_command_count; i++) {
        char usage[32];

        snprintf(
            usage,
            sizeof(usage),
            "%s%s%s",
            shell_commands[i].name,
            shell_commands[i].argument_name != NULL ? " " : "",
            shell_commands[i].argument_name != NULL
                ? shell_commands[i].argument_name
                : ""
        );

        printf(
            "  %-14s %s\n",
            usage,
            shell_commands[i].description
        );
    }
}

int uart_parse_line(char *line, uart_command_t *command)
{
    char *state = NULL;

    if (line == NULL || command == NULL) {
        fprintf(stderr, "Internal error: invalid shell line\n");
        return -1;
    }

    command->type = UART_COMMAND_EMPTY;
    command->argument = NULL;

    const char *name = strtok_r(line, TOKEN_DELIMITERS, &state);

    if (name == NULL) {
        return 0;
    }

    for (size_t i = 0; i < shell_command_count; i++) {

        if (strcmp(name, shell_commands[i].name) != 0) {
            continue;
        }

        const char *argument_name = shell_commands[i].argument_name;

        const char *argument = strtok_r(
            NULL,
            TOKEN_DELIMITERS,
            &state
        );

        if (argument_name == NULL && argument != NULL) {
            fprintf(
                stderr,
                "Command '%s' takes no arguments\n",
                name
            );
            return -1;
        }

        if (argument_name != NULL && argument == NULL) {
            fprintf(
                stderr,
                "Command '%s' requires an argument: %s\n",
                name,
                argument_name
            );
            return -1;
        }

        if (strtok_r(NULL, TOKEN_DELIMITERS, &state) != NULL) {
            fprintf(
                stderr,
                "Command '%s' takes only one argument\n",
                name
            );
            return -1;
        }

        command->type = shell_commands[i].type;
        command->argument = argument;

        return 0;
    }

    fprintf(
        stderr,
        "Unknown command: '%s' (type 'help' to list commands)\n",
        name
    );

    return -1;
}
