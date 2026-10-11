/*
 * Parser of the interactive Bluetooth shell.
 *
 * Turns one typed line into a bt_command_t and prints the help.
 * Both use the shell_commands table, so a command added to the
 * table is parsed and documented at once. The shell itself, which
 * executes the commands, is in bluetooth.c.
 */

#include "protocol/bluetooth/bluetooth_parser.h"

#include <stdbool.h>
#include <stdio.h>
#include <string.h>

#define TOKEN_DELIMITERS " \t\r\n"

/*
 * Description of one shell command.
 */
typedef struct {
    /*
     * Word typed by the user.
     */
    const char *name;

    /*
     * Name of the argument shown in the help, or NULL when
     * the command takes no argument.
     */
    const char *argument_name;

    /*
     * true  - the argument has to be given
     * false - the argument may be left out
     * Ignored when argument_name is NULL.
     */
    bool argument_required;

    bt_command_type_t type;

    /*
     * One line shown in the list of commands.
     */
    const char *description;

    /*
     * Longer text shown by "help COMMAND".
     */
    const char *details;
} shell_command_t;

/*
 * Table of the Bluetooth shell commands.
 *
 * A new command is registered by adding one entry here, a value
 * to bt_command_type_t and a case to run_shell() in bluetooth.c.
 */
static const shell_command_t shell_commands[] = {
    {
        "help",
        "COMMAND",
        false,
        BT_COMMAND_HELP,
        "List the commands, or describe one command",
        "Without an argument, lists all commands.\n"
        "With a command name, shows the detailed help of that\n"
        "command, for example: help select\n"
    },
    {
        "cards",
        NULL,
        false,
        BT_COMMAND_CARDS,
        "List the Bluetooth cards of this computer",
        "Shows every Bluetooth card known to the system with its\n"
        "name (hci0, hci1, ...), address, alias and power state.\n"
        "The card selected for this project is marked with '*'.\n"
        "Diagnostic devices such as Ubertooth One will be listed\n"
        "here as well; their detection is not implemented yet.\n"
    },
    {
        "select",
        "CARD",
        true,
        BT_COMMAND_SELECT,
        "Select the Bluetooth card, for example hci0",
        "Chooses the card used by the other commands. The choice\n"
        "is stored in the project settings and remembered the\n"
        "next time the project is run.\n"
        "CARD is a name from the 'cards' command, such as hci0.\n"
    },
    {
        "status",
        NULL,
        false,
        BT_COMMAND_STATUS,
        "Show the selected card and its state",
        "Shows which card is selected for this project, whether\n"
        "it is still present and whether it is powered on.\n"
    },

    // TODO (stage 2): "scan" - list Bluetooth devices in range
    //                 (address, name, type LE/classic, RSSI).
    // TODO (stage 2): "info ADDRESS" - details of one device
    //                 (paired, services, manufacturer data).
    // TODO (stage 3): "pair ADDRESS" / "unpair ADDRESS".
    // TODO (stage 3): "connect ADDRESS" / "disconnect". The
    //                 address is stored in the project settings.
    // TODO (stage 4): BLE manual communication: "services",
    //                 "read UUID", "write UUID HEX", "notify UUID".
    // TODO (stage 4): Classic manual communication: "rfcomm CHANNEL"
    //                 and "l2cap PSM", sending raw bytes like
    //                 a raw socket.
    // TODO (stage 5): "monitor" - show the traffic between the
    //                 operating system and the selected card
    //                 (HCI monitor, btmon), with its own detailed
    //                 help and an option to save a capture file
    //                 for Wireshark.

    // TODO (stage 6): "sniff" - show the traffic between two other
    //                 devices with a diagnostic device such as
    //                 Ubertooth One.

    {
        "exit",
        NULL,
        false,
        BT_COMMAND_EXIT,
        "Leave the shell and close the program",
        "Closes the shell and the program. Ctrl+D does the same.\n"
    },
};

static const size_t shell_command_count =
    sizeof(shell_commands) / sizeof(shell_commands[0]);

/*
 * Finds a command in the table by its name.
 *
 * Returns NULL when there is no such command.
 */
static const shell_command_t *find_command(const char *name)
{
    for (size_t i = 0; i < shell_command_count; i++) {

        if (strcmp(name, shell_commands[i].name) == 0) {
            return &shell_commands[i];
        }
    }

    return NULL;
}

/*
 * Builds the usage text of a command: its name followed by
 * the name of its argument. An optional argument is shown
 * in square brackets.
 *
 *     "cards"
 *     "select CARD"
 *     "help [COMMAND]"
 */
static void format_usage(
    const shell_command_t *command,
    char *buffer,
    size_t size
)
{
    if (command->argument_name == NULL) {
        snprintf(buffer, size, "%s", command->name);
    } else if (command->argument_required) {
        snprintf(
            buffer,
            size,
            "%s %s",
            command->name,
            command->argument_name
        );
    } else {
        snprintf(
            buffer,
            size,
            "%s [%s]",
            command->name,
            command->argument_name
        );
    }
}

void bt_print_shell_help(const char *command_name)
{
    char usage[48];

    /*
     * help COMMAND: the detailed help of one command.
     */
    if (command_name != NULL) {
        const shell_command_t *command = find_command(command_name);

        if (command == NULL) {
            fprintf(
                stderr,
                "Unknown command: '%s' "
                "(type 'help' to list commands)\n",
                command_name
            );
            return;
        }

        format_usage(command, usage, sizeof(usage));

        printf(
            "Usage: %s\n"
            "\n"
            "%s",
            usage,
            command->details
        );

        return;
    }

    /*
     * help: one line per command.
     */
    printf("Commands:\n");

    for (size_t i = 0; i < shell_command_count; i++) {
        format_usage(&shell_commands[i], usage, sizeof(usage));

        printf(
            "  %-16s %s\n",
            usage,
            shell_commands[i].description
        );
    }

    printf("\nType 'help COMMAND' for details of one command.\n");
}

int bt_parse_line(char *line, bt_command_t *command)
{
    char *state = NULL;

    if (line == NULL || command == NULL) {
        fprintf(stderr, "Internal error: invalid shell line\n");
        return -1;
    }

    command->type = BT_COMMAND_EMPTY;
    command->argument = NULL;

    /*
     * The first word is the command name. A line without words
     * is an empty command.
     */
    const char *name = strtok_r(line, TOKEN_DELIMITERS, &state);

    if (name == NULL) {
        return 0;
    }

    const shell_command_t *entry = find_command(name);

    if (entry == NULL) {
        fprintf(
            stderr,
            "Unknown command: '%s' (type 'help' to list commands)\n",
            name
        );
        return -1;
    }

    /*
     * The second word, when present, is the argument.
     */
    const char *argument = strtok_r(NULL, TOKEN_DELIMITERS, &state);

    /*
     * Check the argument against what the command expects.
     */
    if (entry->argument_name == NULL && argument != NULL) {
        fprintf(
            stderr,
            "Command '%s' takes no arguments\n",
            name
        );
        return -1;
    }

    if (entry->argument_name != NULL &&
        entry->argument_required &&
        argument == NULL) {

        fprintf(
            stderr,
            "Command '%s' requires an argument: %s\n",
            name,
            entry->argument_name
        );
        return -1;
    }

    /*
     * No command takes more than one argument.
     */
    if (strtok_r(NULL, TOKEN_DELIMITERS, &state) != NULL) {
        fprintf(
            stderr,
            "Command '%s' takes only one argument\n",
            name
        );
        return -1;
    }

    command->type = entry->type;
    command->argument = argument;

    return 0;
}
