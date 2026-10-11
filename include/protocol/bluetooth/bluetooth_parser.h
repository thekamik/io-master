/*
 * Parser of the interactive Bluetooth shell.
 *
 * It is independent of the command-line parser used for
 * project management (cli/parser.h) and of the UART shell
 * parser (protocol/uart/uart_parser.h).
 */

#ifndef BLUETOOTH_PARSER_H
#define BLUETOOTH_PARSER_H

/*
 * Commands of the Bluetooth shell.
 */
typedef enum {
    BT_COMMAND_EMPTY,   /* empty line */
    BT_COMMAND_HELP,    /* help [COMMAND] */
    BT_COMMAND_CARDS,   /* cards */
    BT_COMMAND_SELECT,  /* select CARD */
    BT_COMMAND_STATUS,  /* status */
    BT_COMMAND_SCAN,    /* scan */
    BT_COMMAND_DEVICES, /* devices */
    BT_COMMAND_INFO,    /* info DEVICE */
    BT_COMMAND_FORGET,  /* forget */
    BT_COMMAND_CONNECT,     /* connect [DEVICE] */
    BT_COMMAND_DISCONNECT,  /* disconnect [DEVICE] */
    BT_COMMAND_PAIR,        /* pair DEVICE */
    BT_COMMAND_UNPAIR,      /* unpair DEVICE */
    BT_COMMAND_EXIT     /* exit */

    // TODO (stage 4): BT_COMMAND_SERVICES, BT_COMMAND_READ,
    //                 BT_COMMAND_WRITE, BT_COMMAND_NOTIFY,
    //                 BT_COMMAND_RFCOMM, BT_COMMAND_L2CAP
    // TODO (stage 5): BT_COMMAND_MONITOR
    // TODO (stage 6): BT_COMMAND_SNIFF
} bt_command_type_t;

/*
 * Result of parsing one line.
 */
typedef struct {
    bt_command_type_t type;

    /*
     * Argument of the command, or NULL when none was given.
     * Points into the parsed line.
     */
    const char *argument;
} bt_command_t;

/*
 * Parses one line typed in the Bluetooth shell.
 *
 * The line is modified during parsing. An empty line is not
 * an error: it yields BT_COMMAND_EMPTY.
 *
 * Errors are reported on stderr.
 *
 * Returns:
 *   0  - success
 *  -1  - error
 */
int bt_parse_line(char *line, bt_command_t *command);

/*
 * Prints help for the Bluetooth shell.
 *
 * command_name == NULL  - the list of all commands
 * command_name != NULL  - the detailed help of that command
 *
 * An unknown command name is reported on stderr.
 */
void bt_print_shell_help(const char *command_name);

#endif
