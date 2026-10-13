/*
 * Parser of the menu of the AI shell.
 *
 * The AI shell has two levels:
 *
 *     the menu   commands that manage the saved chats
 *                ("list", "new", "open", "delete", ...); they
 *                are parsed here
 *
 *     a chat     entered from the menu; every typed line is
 *                a message for the model, so nothing is parsed
 *
 * The parser is independent of the command-line parser used for
 * project management (cli/parser.h) and of the other shell
 * parsers.
 */

#ifndef AI_PARSER_H
#define AI_PARSER_H

/*
 * Commands of the menu.
 */
typedef enum {
    AI_COMMAND_EMPTY,    /* empty line */
    AI_COMMAND_HELP,     /* help */
    AI_COMMAND_STATUS,   /* status */
    AI_COMMAND_LIST,     /* list */
    AI_COMMAND_NEW,      /* new [TITLE] */
    AI_COMMAND_OPEN,     /* open ID */
    AI_COMMAND_DELETE,   /* delete ID */
    AI_COMMAND_EXIT      /* exit */
} ai_command_type_t;

/*
 * Result of parsing one line.
 */
typedef struct {
    ai_command_type_t type;

    /*
     * For AI_COMMAND_NEW: the title, which may contain spaces,
     * or NULL when none was given. Points into the parsed line.
     */
    const char *text;

    /*
     * For AI_COMMAND_OPEN and AI_COMMAND_DELETE: the id of
     * the chat.
     */
    int id;
} ai_command_t;

/*
 * Parses one line typed in the menu.
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
 * Prints the list of the menu commands.
 */
void ai_print_shell_help(void);

#endif
