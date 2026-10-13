/*
 * Command-line parser for device discovery and project management.
 *
 * The interactive UART shell has its own, independent parser
 * in protocol/uart/uart_parser.h.
 */

#ifndef PARSER_H
#define PARSER_H

#include "discovery/devices.h"

/*
 * Commands of the program and the command line that selects them.
 */
typedef enum {
    COMMAND_HELP,            /* --help */
    COMMAND_SCAN,            /* --scan [--type TYPE] */
    COMMAND_PROJECT_LIST,    /* --display STATUS */
    COMMAND_PROJECT_ADD,     /* project add NAME --protocol PROTOCOL */
    COMMAND_PROJECT_SELECT,  /* project select ID */
    COMMAND_PROJECT_REMOVE,  /* project remove ID */
    COMMAND_RUN,             /* run */
    COMMAND_RUN_HELP,        /* run --help */
    COMMAND_AI_CONFIG        /* ai-config, or aic for short */
} command_type_t;

/*
 * Which projects the project table shows.
 */
typedef enum {
    PROJECT_FILTER_ACTIVE,
    PROJECT_FILTER_INACTIVE,
    PROJECT_FILTER_ALL
} project_filter_t;

/*
 * Result of parsing: the command and the values it needs.
 *
 * Only the fields that belong to the given type are meaningful.
 */
typedef struct {
    command_type_t type;

    /* COMMAND_SCAN */
    device_type_t device_type;

    /* COMMAND_PROJECT_LIST */
    project_filter_t project_filter;

    /* COMMAND_PROJECT_ADD */
    const char *project_name;
    const char *protocol_name;

    /* COMMAND_PROJECT_SELECT, COMMAND_PROJECT_REMOVE */
    int project_id;
} command_t;

/*
 * Parses the command line.
 *
 * Errors are reported on stderr.
 *
 * Returns:
 *   0  - success
 *  -1  - error
 */
int parse_args(int argc, char *argv[], command_t *command);

void print_usage(const char *program_name);

#endif
