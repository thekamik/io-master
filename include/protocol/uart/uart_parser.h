#ifndef UART_PARSER_H
#define UART_PARSER_H

/*
 * Parser of the interactive UART shell.
 *
 * It is independent of the command-line parser used for
 * project management (cli/parser.h).
 */

typedef enum {
    UART_COMMAND_EMPTY,
    UART_COMMAND_HELP,
    UART_COMMAND_READ,
    UART_COMMAND_SCAN,
    UART_COMMAND_SELECT,
    UART_COMMAND_PERMISSION,
    UART_COMMAND_EXIT
} uart_command_type_t;

typedef struct {
    uart_command_type_t type;

    /*
     * Argument of the command, or NULL when the command has none.
     * Points into the parsed line.
     */
    const char *argument;
} uart_command_t;

/*
 * Parses one line typed in the UART shell.
 *
 * The line is modified during parsing. An empty line is not
 * an error: it yields UART_COMMAND_EMPTY.
 *
 * Errors are reported on stderr.
 *
 * Returns:
 *   0  - success
 *  -1  - error
 */
int uart_parse_line(char *line, uart_command_t *command);

/*
 * Prints the list of the UART shell commands.
 */
void uart_print_shell_help(void);

#endif
