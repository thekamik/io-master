#ifndef COMMANDS_H
#define COMMANDS_H

#include <stdbool.h>

#include "cli/parser.h"
#include "discovery/devices.h"

/*
 * Implementations of the commands selected on the command line.
 *
 * Every function reports its own errors on stderr and returns
 * the exit code of the program:
 *   0  - success
 *   1  - error
 */

int command_scan(device_type_t device_type);

int command_project_list(
    const char *program_name,
    project_filter_t filter
);

int command_project_add(
    const char *program_name,
    const char *name,
    const char *protocol_name
);

int command_project_select(int id);

int command_project_remove(int id);

int command_run(const char *program_name, bool show_help);

#endif
