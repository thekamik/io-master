/*
 * Entry point of io-master.
 *
 * main() only parses the command line and dispatches the command.
 * The commands themselves are implemented in cli/commands.c.
 */

#include "cli/ai_config.h"
#include "cli/commands.h"
#include "cli/parser.h"
#include "project/project.h"

#include <stdio.h>

int main(int argc, char *argv[]) {
    command_t command;

    /*
     * Translate the command line into a command_t.
     * On error parse_args() has already printed what is wrong.
     */
    if (parse_args(argc, argv, &command) != 0) {
        fprintf(stderr, "Try '%s --help' for more information\n", argv[0]);
        return 1;
    }

    /*
     * Make sure the configuration directory and the project
     * file exist before any command uses them.
     */
    if (command.type != COMMAND_HELP &&
        project_initialize_storage() != 0) {

        perror("project_initialize_storage");
        return 1;
    }

    /*
     * Run the requested command. Every handler returns the exit
     * code of the program.
     */
    switch (command.type) {

    case COMMAND_HELP:
        print_usage(argv[0]);
        return 0;

    case COMMAND_SCAN:
        return command_scan(command.device_type);

    case COMMAND_PROJECT_LIST:
        return command_project_list(argv[0], command.project_filter);

    case COMMAND_PROJECT_ADD:
        return command_project_add(
            argv[0],
            command.project_name,
            command.protocol_name
        );

    case COMMAND_PROJECT_SELECT:
        return command_project_select(command.project_id);

    case COMMAND_PROJECT_REMOVE:
        return command_project_remove(command.project_id);

    case COMMAND_RUN:
        return command_run(argv[0], false);

    case COMMAND_RUN_HELP:
        return command_run(argv[0], true);

    case COMMAND_AI_CONFIG:
        return command_ai_config();
    }

    return 1;
}
