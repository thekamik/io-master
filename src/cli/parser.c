/*
 * Command-line parser.
 *
 * parse_args() looks at the first argument and passes the rest
 * to one of three parsers:
 *
 *     parse_global()   --help, --scan, --display
 *     parse_project()  project add | select | remove | list
 *     parse_run()      run [--help]
 *
 * All of them read options with parse_options() and then check
 * that only the options valid for the command were used.
 */

#include "cli/parser.h"

#include "protocol/protocol.h"

#include <errno.h>
#include <getopt.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Options found on the command line.
 *
 * All commands share one option table. Each command then rejects
 * the options that have no meaning for it.
 */
typedef struct {
    bool scan;
    bool all;
    bool help;
    const char *type;
    const char *display;
    const char *protocol;
} options_t;

/*
 * Converts a string to an int greater than zero. Used for
 * project ids.
 *
 * Rejects empty strings, trailing characters and values that
 * do not fit in an int.
 */
static int parse_positive_int(const char *str, int *value)
{
    char *endptr;
    long number;

    if (str == NULL || *str == '\0') {
        return -1;
    }

    errno = 0;
    endptr = NULL;

    number = strtol(str, &endptr, 10);

    if (errno == ERANGE ||
        endptr == str ||
        *endptr != '\0' ||
        number < 1 ||
        number > INT_MAX) {
        return -1;
    }

    *value = (int)number;

    return 0;
}

/*
 * Converts the argument of --display to a project filter.
 */
static int parse_project_filter(
    const char *str,
    project_filter_t *filter
)
{
    if (strcmp(str, "active") == 0) {
        *filter = PROJECT_FILTER_ACTIVE;
        return 0;
    }

    if (strcmp(str, "inactive") == 0) {
        *filter = PROJECT_FILTER_INACTIVE;
        return 0;
    }

    if (strcmp(str, "all") == 0) {
        *filter = PROJECT_FILTER_ALL;
        return 0;
    }

    return -1;
}

/*
 * Converts the argument of --type to a device type.
 */
static int parse_device_type(
    const char *str,
    device_type_t *device_type
)
{
    if (strcmp(str, "all") == 0) {
        *device_type = DEVICE_TYPE_ALL;
        return 0;
    }

    if (strcmp(str, "ttyACM") == 0) {
        *device_type = DEVICE_TYPE_TTYACM;
        return 0;
    }

    if (strcmp(str, "ttyUSB") == 0) {
        *device_type = DEVICE_TYPE_TTYUSB;
        return 0;
    }

    return -1;
}

/*
 * Prints the general help.
 *
 * The list of protocols comes from the protocol registry, so
 * a new protocol appears here automatically. Help that is specific
 * to one protocol is printed by "run --help".
 */
void print_usage(const char *program_name)
{
    printf(
        "Usage:\n"
        "  %s --scan [--type TYPE]\n"
        "  %s --display STATUS\n"
        "  %s project add NAME --protocol PROTOCOL\n"
        "  %s project select ID\n"
        "  %s project remove ID\n"
        "  %s run [--help]\n"
        "\n"
        "Device discovery:\n"
        "  --scan            Scan available serial devices\n"
        "  --type TYPE       Device type: all, ttyACM or ttyUSB\n"
        "\n"
        "Project management:\n"
        "  --display STATUS  Show the table of projects;\n"
        "                    STATUS is active, inactive or all\n"
        "  project add       Create a project with the given name\n"
        "  project select    Select the project used by 'run'\n"
        "  project remove    Mark a project as inactive\n"
        "\n"
        "Running a project:\n"
        "  run               Start the selected project\n"
        "  run --help        Show the help of the protocol used\n"
        "                    by the selected project\n"
        "\n"
        "Other options:\n"
        "  -h, --help        Show this help\n"
        "\n"
        "Supported protocols:\n",
        program_name,
        program_name,
        program_name,
        program_name,
        program_name,
        program_name
    );

    for (size_t i = 0; i < protocol_count(); i++) {
        const protocol_t *protocol = protocol_get(i);

        printf(
            "  %-17s %s\n",
            protocol->name,
            protocol->description
        );
    }

    printf(
        "\n"
        "Examples:\n"
        "  %s --scan --type ttyACM\n"
        "  %s --display active\n"
        "  %s project add \"Arduino logger\" --protocol uart\n"
        "  %s project select 1\n"
        "  %s run\n",
        program_name,
        program_name,
        program_name,
        program_name,
        program_name
    );
}

/*
 * Reads the options of one command.
 *
 * argv[0] is skipped, so the caller passes the part of the
 * command line that starts with the command name.
 *
 * After the call optind is the index of the first positional
 * argument in argv.
 */
static int parse_options(int argc, char *argv[], options_t *options)
{
    static const struct option long_options[] = {
        { "scan",     no_argument,       NULL, 'c' },
        { "type",     required_argument, NULL, 't' },
        { "display",  required_argument, NULL, 'd' },
        { "all",      no_argument,       NULL, 'a' },
        { "protocol", required_argument, NULL, 'p' },
        { "help",     no_argument,       NULL, 'h' },
        { NULL,       0,                 NULL,  0  }
    };

    int option;
    int option_index = 0;

    memset(options, 0, sizeof(*options));

    /*
     * ':' at the beginning makes getopt_long() distinguish
     * between a missing argument and an unknown option.
     */
    opterr = 0;
    optind = 1;

    while ((option = getopt_long(
                argc,
                argv,
                ":ct:d:ap:h",
                long_options,
                &option_index)) != -1) {

        switch (option) {

        case 'c':
            options->scan = true;
            break;

        case 't':
            options->type = optarg;
            break;

        case 'd':
            options->display = optarg;
            break;

        case 'a':
            options->all = true;
            break;

        case 'p':
            options->protocol = optarg;
            break;

        case 'h':
            options->help = true;
            break;

        case ':':
            fprintf(
                stderr,
                "Option '%s' requires an argument\n",
                argv[optind - 1]
            );
            return -1;

        case '?':
            if (optopt != 0) {
                fprintf(
                    stderr,
                    "Unknown option: '-%c'\n",
                    optopt
                );
            } else {
                fprintf(
                    stderr,
                    "Unknown option: '%s'\n",
                    argv[optind - 1]
                );
            }
            return -1;

        default:
            return -1;
        }
    }

    return 0;
}

/*
 * Reports an option that has no meaning for the given command.
 */
static int reject_option(
    bool present,
    const char *option,
    const char *command
)
{
    if (!present) {
        return 0;
    }

    fprintf(
        stderr,
        "Option '%s' cannot be used with '%s'\n",
        option,
        command
    );

    return -1;
}

/*
 * Checks the number of positional arguments left after
 * parse_options().
 */
static int expect_arguments(
    int argc,
    char *argv[],
    int expected,
    const char *missing_message
)
{
    int count = argc - optind;

    if (count < expected) {
        fprintf(stderr, "%s\n", missing_message);
        return -1;
    }

    if (count > expected) {
        fprintf(
            stderr,
            "Unexpected argument: '%s'\n",
            argv[optind + expected]
        );
        return -1;
    }

    return 0;
}

/*
 * Global options:
 *
 *     --help
 *     --scan
 *     --scan --type ttyACM
 *     --display active
 */
static int parse_global(int argc, char *argv[], command_t *command)
{
    options_t options;

    if (parse_options(argc, argv, &options) != 0) {
        return -1;
    }

    /*
     * --help wins over every other option.
     */
    if (options.help) {
        command->type = COMMAND_HELP;
        return 0;
    }

    if (reject_option(options.all, "--all", "--scan") != 0 ||
        reject_option(
            options.protocol != NULL,
            "--protocol",
            "--scan") != 0 ||
        expect_arguments(argc, argv, 0, "") != 0) {
        return -1;
    }

    /*
     * --display STATUS: show the table of projects.
     */
    if (options.display != NULL) {

        if (reject_option(
                options.scan,
                "--scan",
                "--display") != 0 ||
            reject_option(
                options.type != NULL,
                "--type",
                "--display") != 0) {
            return -1;
        }

        if (parse_project_filter(
                options.display,
                &command->project_filter) != 0) {

            fprintf(
                stderr,
                "Invalid project status: '%s' "
                "(expected active, inactive or all)\n",
                options.display
            );
            return -1;
        }

        command->type = COMMAND_PROJECT_LIST;

        return 0;
    }

    /*
     * Everything else has to be --scan.
     * --type without --scan is a mistake.
     */
    if (!options.scan) {

        if (options.type != NULL) {
            fprintf(
                stderr,
                "Option '--type' can only be used with '--scan'\n"
            );
        } else {
            fprintf(stderr, "Missing command\n");
        }

        return -1;
    }

    /*
     * --scan, optionally narrowed by --type.
     */
    command->type = COMMAND_SCAN;

    if (options.type != NULL &&
        parse_device_type(
            options.type,
            &command->device_type) != 0) {

        fprintf(
            stderr,
            "Invalid device type: '%s' "
            "(expected all, ttyACM or ttyUSB)\n",
            options.type
        );
        return -1;
    }

    return 0;
}

/*
 * Project management:
 *
 *     project list [--all]
 *     project add NAME --protocol PROTOCOL
 *     project select ID
 *     project remove ID
 *
 * argv[0] is the name of the project command ("list", "add", ...).
 */
static int parse_project(int argc, char *argv[], command_t *command)
{
    const char *name = argv[0];
    options_t options;

    if (parse_options(argc, argv, &options) != 0) {
        return -1;
    }

    if (options.help) {
        command->type = COMMAND_HELP;
        return 0;
    }

    if (reject_option(options.scan, "--scan", "project") != 0 ||
        reject_option(
            options.type != NULL,
            "--type",
            "project") != 0 ||
        reject_option(
            options.display != NULL,
            "--display",
            "project") != 0) {
        return -1;
    }

    /*
     * project list [--all]
     *
     * Older spelling of --display. It is kept working, but it is
     * not listed in the help.
     */
    if (strcmp(name, "list") == 0) {

        if (reject_option(
                options.protocol != NULL,
                "--protocol",
                "project list") != 0 ||
            expect_arguments(argc, argv, 0, "") != 0) {
            return -1;
        }

        command->type = COMMAND_PROJECT_LIST;
        command->project_filter = options.all
            ? PROJECT_FILTER_ALL
            : PROJECT_FILTER_ACTIVE;

        return 0;
    }

    /*
     * project add NAME --protocol PROTOCOL
     *
     * The protocol name is validated later,
     * in command_project_add().
     */
    if (strcmp(name, "add") == 0) {

        if (reject_option(
                options.all,
                "--all",
                "project add") != 0 ||
            expect_arguments(
                argc,
                argv,
                1,
                "Missing project name") != 0) {
            return -1;
        }

        if (options.protocol == NULL) {
            fprintf(stderr, "Missing option '--protocol'\n");
            return -1;
        }

        command->type = COMMAND_PROJECT_ADD;
        command->project_name = argv[optind];
        command->protocol_name = options.protocol;

        return 0;
    }

    /*
     * project select ID
     * project remove ID
     *
     * Both take exactly one project id.
     */
    if (strcmp(name, "select") == 0 ||
        strcmp(name, "remove") == 0) {

        if (reject_option(options.all, "--all", name) != 0 ||
            reject_option(
                options.protocol != NULL,
                "--protocol",
                name) != 0 ||
            expect_arguments(
                argc,
                argv,
                1,
                "Missing project id") != 0) {
            return -1;
        }

        if (parse_positive_int(
                argv[optind],
                &command->project_id) != 0) {

            fprintf(
                stderr,
                "Invalid project id: '%s'\n",
                argv[optind]
            );
            return -1;
        }

        command->type = strcmp(name, "select") == 0
            ? COMMAND_PROJECT_SELECT
            : COMMAND_PROJECT_REMOVE;

        return 0;
    }

    fprintf(stderr, "Unknown project command: '%s'\n", name);

    return -1;
}

/*
 * Running the selected project:
 *
 *     run
 *     run --help
 */
static int parse_run(int argc, char *argv[], command_t *command)
{
    options_t options;

    if (parse_options(argc, argv, &options) != 0) {
        return -1;
    }

    if (reject_option(options.scan, "--scan", "run") != 0 ||
        reject_option(options.type != NULL, "--type", "run") != 0 ||
        reject_option(options.all, "--all", "run") != 0 ||
        reject_option(
            options.display != NULL,
            "--display",
            "run") != 0 ||
        reject_option(
            options.protocol != NULL,
            "--protocol",
            "run") != 0 ||
        expect_arguments(argc, argv, 0, "") != 0) {
        return -1;
    }

    command->type = options.help ? COMMAND_RUN_HELP : COMMAND_RUN;

    return 0;
}

int parse_args(int argc, char *argv[], command_t *command)
{
    if (command == NULL) {
        fprintf(stderr, "Internal error: command is NULL\n");
        return -1;
    }

    /*
     * Start from a known state, so that fields that the command
     * line does not set have defined values.
     */
    memset(command, 0, sizeof(*command));

    command->type = COMMAND_HELP;
    command->device_type = DEVICE_TYPE_ALL;

    if (argc < 2) {
        fprintf(stderr, "Missing command\n");
        return -1;
    }

    /*
     * The first argument decides which parser handles the rest:
     *
     *     an option  -> parse_global()
     *     "project"  -> parse_project()
     *     "run"      -> parse_run()
     */
    if (argv[1][0] == '-') {
        return parse_global(argc, argv, command);
    }

    if (strcmp(argv[1], "project") == 0) {

        if (argc < 3) {
            fprintf(stderr, "Missing project command\n");
            return -1;
        }

        /*
         * "project --help" has no project command.
         */
        if (strcmp(argv[2], "-h") == 0 ||
            strcmp(argv[2], "--help") == 0) {
            return 0;
        }

        /*
         * Skip the program name and "project", so that the project
         * command ("add", "select", ...) becomes argv[0].
         */
        return parse_project(argc - 2, argv + 2, command);
    }

    if (strcmp(argv[1], "run") == 0) {
        /*
         * Skip the program name, so that "run" becomes argv[0].
         */
        return parse_run(argc - 1, argv + 1, command);
    }

    fprintf(stderr, "Unknown command: '%s'\n", argv[1]);

    return -1;
}
