/*
 * Parser of the interactive AI shell.
 *
 * It is independent of the command-line parser used for project
 * management (cli/parser.h) and of the other shell parsers.
 *
 * In this shell a line is a message for the AI model, unless it
 * starts with '/', which marks a command:
 *
 *     what does 0x7E mean in HDLC?     a message
 *     /new                             a command
 */

#ifndef AI_PARSER_H
#define AI_PARSER_H

/*
 * What a typed line asks for.
 */
typedef enum {
    AI_COMMAND_EMPTY,    /* empty line */
    AI_COMMAND_MESSAGE,  /* text to send to the model */
    AI_COMMAND_HELP,     /* /help */
    AI_COMMAND_NEW,      /* /new */
    AI_COMMAND_STATUS,   /* /status */
    AI_COMMAND_KEY,      /* /key */
    AI_COMMAND_EXIT      /* /exit */
} ai_command_type_t;

/*
 * Result of parsing one line.
 */
typedef struct {
    ai_command_type_t type;

    /*
     * For AI_COMMAND_MESSAGE: the message, without surrounding
     * spaces. Points into the parsed line. NULL otherwise.
     */
    const char *text;
} ai_command_t;

/*
 * Parses one line typed in the AI shell.
 *
 * The line is modified during parsing. An empty line is not
 * an error: it yields AI_COMMAND_EMPTY.
 *
 * Errors are reported on stderr.
 *
 * Returns:
 *   0  - success
 *  -1  - error
 */
int ai_parse_line(char *line, ai_command_t *command);

/*
 * Prints the list of the AI shell commands.
 */
void ai_print_shell_help(void);

#endif
