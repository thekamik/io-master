#ifndef PARSER_H
#define PARSER_H

#include "discovery/devices.h"

typedef enum {
    COMMAND_HELP,
    COMMAND_SCAN,
    COMMAND_PROJECT_LIST,
    COMMAND_PROJECT_ADD,
    COMMAND_PROJECT_SELECT,
    COMMAND_PROJECT_REMOVE,
    COMMAND_RUN,
    COMMAND_RUN_HELP
} command_type_t;

typedef enum {
    PROJECT_FILTER_ACTIVE,
    PROJECT_FILTER_INACTIVE,
    PROJECT_FILTER_ALL
} project_filter_t;

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
