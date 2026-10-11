/*
 * Bluetooth protocol.
 *
 * The protocol is built in stages. This file currently holds
 * stage 1: the protocol skeleton, the interactive shell, the
 * project settings and the commands that work with the Bluetooth
 * cards of the computer.
 *
 * The file is organized in this order:
 *
 *     settings      default values, loading and saving of
 *                   im_config/projects/<id>.json
 *     shell state   bt_shell_t
 *     protocol_t    bluetooth_init(), bluetooth_print_help()
 *     commands      one shell_*() function per shell command
 *     shell loop    run_shell(): reads a line, parses it with
 *                   bluetooth_parser.c and executes the command
 *     entry point   bluetooth_run() and the bluetooth_protocol
 *                   structure
 *
 * Access to the Bluetooth cards is in adapter.c. Diagnostic
 * devices such as Ubertooth One belong to sniffer.c.
 *
 * Planned work:
 */

// TODO (stage 2): Reconnaissance. "scan" lists the devices in
//                 range of the selected card and "info ADDRESS"
//                 shows the details of one device.
// TODO (stage 3): Connection. "pair", "unpair", "connect" and
//                 "disconnect". The target device is stored in
//                 the "device" setting of the project.
// TODO (stage 4): Manual communication, similar to a raw socket.
//                 BLE: "services", "read UUID", "write UUID HEX",
//                 "notify UUID". Classic: "rfcomm CHANNEL" and
//                 "l2cap PSM" (needs the libbluetooth-dev package).
// TODO (stage 5): Analysis. "monitor" shows the traffic between
//                 the operating system and the selected card and
//                 can save it to a file for Wireshark.
// TODO (stage 6): Diagnostic devices. Detect Ubertooth One in
//                 "cards" and add a "sniff" command that shows
//                 the traffic between two other devices.
// TODO: The line-reading code of run_shell() is the same as in
//       the UART shell. Move it to a shared helper.

#include "protocol/bluetooth/bluetooth.h"

#include "protocol/bluetooth/adapter.h"
#include "protocol/bluetooth/bluetooth_parser.h"
#include "protocol/bluetooth/sniffer.h"
#include "storage/json_file.h"

#include <errno.h>
#include <limits.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/*
 * Keys used in the settings file of a Bluetooth project
 * (im_config/projects/<id>.json).
 */
#define KEY_ADAPTER "adapter"
#define KEY_DEVICE "device"

/*
 * Interactive shell.
 */
#define SHELL_PROMPT "bluetooth> "
#define SHELL_LINE_MAX 256

/*
 * Set by Ctrl+C (SIGINT).
 *
 * Ctrl+C never closes the shell. Stage 1 has no long-running
 * command, so the flag is only set here.
 */
// TODO (stage 2): "scan" and later "monitor" run until Ctrl+C
//                 and will check this flag, like "read" in the
//                 UART shell.
static volatile sig_atomic_t interrupted = 0;

/*
 * SIGINT handler. It only sets the flag.
 */
static void handle_signal(int signal) {
    (void)signal; // Unused parameter
    interrupted = 1;
}

/*
 * Settings of a Bluetooth project, as stored in its
 * settings file.
 */
typedef struct {
    /*
     * System name of the selected card, for example "hci0".
     * Empty when no card is selected.
     */
    char adapter[BT_ADAPTER_NAME_MAX];

    /*
     * Address of the target device, "AA:BB:CC:DD:EE:FF".
     * Empty when no device is chosen.
     */
    // TODO (stage 3): set by "connect ADDRESS". Until then the
    //                 value is only loaded and kept.
    char device[BT_ADDRESS_MAX];
} bt_settings_t;

/*
 * Writes the settings file with default values: no card and
 * no device, because both depend on the hardware at hand.
 */
static int write_default_settings(const char *path)
{
    json_t *root = json_pack(
        "{s:s, s:s}",
        KEY_ADAPTER, "",
        KEY_DEVICE, ""
    );

    if (root == NULL) {
        errno = ENOMEM;
        return -1;
    }

    int result = json_file_write(path, root);
    int saved_errno = errno;

    json_decref(root);
    errno = saved_errno;

    return result;
}

/*
 * Creates the settings file of a new project.
 * An existing file is left untouched.
 */
static int create_settings(
    const project_info_t *project,
    char *path,
    size_t path_size
)
{
    if (project_create_settings_directory() != 0 ||
        project_get_settings_path(
            project->id,
            path,
            path_size) != 0) {

        perror("project settings");
        return -1;
    }

    if (access(path, F_OK) == 0) {
        return 0;
    }

    if (write_default_settings(path) != 0) {
        perror(path);
        return -1;
    }

    return 0;
}

/*
 * Reads and validates the settings file.
 *
 * Problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
static int load_settings(const char *path, bt_settings_t *settings)
{
    json_t *root = NULL;

    if (json_file_read(path, &root) != 0) {

        /*
         * Invalid JSON has already been reported.
         */
        if (errno != EINVAL) {
            perror(path);
        }

        return -1;
    }

    /*
     * Fetch every setting. A missing key yields NULL, which
     * fails the type checks below.
     */
    const json_t *adapter = json_object_get(root, KEY_ADAPTER);
    const json_t *device = json_object_get(root, KEY_DEVICE);

    /*
     * Validate the settings one by one and remember the
     * first problem.
     */
    const char *problem = NULL;

    if (!json_is_string(adapter) ||
        strlen(json_string_value(adapter)) >=
            sizeof(settings->adapter)) {

        problem = "'" KEY_ADAPTER "' must be a card name "
                  "such as \"hci0\"";

    } else if (!json_is_string(device) ||
               strlen(json_string_value(device)) >=
                   sizeof(settings->device)) {

        problem = "'" KEY_DEVICE "' must be a Bluetooth address";
    }

    if (problem != NULL) {
        fprintf(stderr, "%s: %s\n", path, problem);
        json_decref(root);
        return -1;
    }

    memset(settings, 0, sizeof(*settings));

    snprintf(
        settings->adapter,
        sizeof(settings->adapter),
        "%s",
        json_string_value(adapter)
    );

    snprintf(
        settings->device,
        sizeof(settings->device),
        "%s",
        json_string_value(device)
    );

    json_decref(root);

    return 0;
}

/*
 * Stores one text setting in the settings file.
 * The other settings are left untouched.
 *
 * Problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
static int save_setting(
    const char *path,
    const char *key,
    const char *value
)
{
    json_t *root = NULL;

    if (json_file_read(path, &root) != 0) {

        /*
         * Invalid JSON has already been reported.
         */
        if (errno != EINVAL) {
            perror(path);
        }

        return -1;
    }

    /*
     * json_string() returns NULL for text that is not valid
     * UTF-8, and json_object_set_new() then fails.
     */
    if (json_object_set_new(root, key, json_string(value)) != 0) {
        fprintf(stderr, "Invalid value for '%s': '%s'\n", key, value);
        json_decref(root);
        return -1;
    }

    if (json_file_write(path, root) != 0) {
        perror(path);
        json_decref(root);
        return -1;
    }

    json_decref(root);

    return 0;
}

/*
 * State of the interactive shell.
 */
typedef struct {
    /*
     * Settings of the project. Kept in step with the settings
     * file: a command that changes a setting saves it first.
     */
    bt_settings_t settings;

    /*
     * Path of the settings file of the project.
     */
    char settings_path[PATH_MAX];

    // TODO (stage 3): connection state of the target device.
} bt_shell_t;

/*
 * Reports that BlueZ could not be asked for the cards.
 *
 * errno has to hold the error set by adapter.c.
 */
static void report_bluez_error(void)
{
    fprintf(
        stderr,
        "Cannot read the Bluetooth cards: %s\n"
        "Check that the Bluetooth service is running: "
        "systemctl status bluetooth\n",
        strerror(errno)
    );
}

/*
 * protocol_t.init
 *
 * Called once when a Bluetooth project is added. Creates the
 * settings file with default values.
 */
static int bluetooth_init(const project_info_t *project)
{
    char path[PATH_MAX];

    if (create_settings(project, path, sizeof(path)) != 0) {
        return -1;
    }

    printf("Bluetooth settings: %s\n", path);

    return 0;
}

/*
 * protocol_t.print_help
 *
 * Printed by "run --help" for a Bluetooth project. Describes the
 * protocol and its settings, followed by the shell commands.
 */
static void bluetooth_print_help(
    const char *program_name,
    const project_info_t *project
)
{
    char path[PATH_MAX];

    if (project_get_settings_path(
            project->id,
            path,
            sizeof(path)) != 0) {

        snprintf(path, sizeof(path), "(unknown)");
    }

    printf(
        "Project %d: %s (bluetooth)\n"
        "\n"
        "Starts an interactive shell for testing Bluetooth\n"
        "devices. The Bluetooth card is chosen in the shell\n"
        "with 'select CARD'.\n"
        "\n"
        "Usage:\n"
        "  %s run\n"
        "\n"
        "Settings file:\n"
        "  %s\n"
        "\n"
        "Settings:\n"
        "  adapter      Bluetooth card, for example hci0\n"
        "               (set by 'select CARD' in the shell)\n"
        "  device       Address of the target device\n"
        "               (not used yet)\n"
        "\n"
        "Shell commands:\n"
        "\n",
        project->id,
        project->name,
        program_name,
        path
    );

    /*
     * The same list that "help" prints inside the shell.
     */
    bt_print_shell_help(NULL);
}

/*
 * Prints the table of the Bluetooth cards handled by BlueZ.
 * The card selected for the project is marked with '*'.
 */
static void print_adapters(const bt_shell_t *shell)
{
    bt_adapter_info_t adapters[MAX_BT_ADAPTERS];
    size_t adapter_count = 0;

    if (bt_find_adapters(
            adapters,
            MAX_BT_ADAPTERS,
            &adapter_count) != 0) {

        report_bluez_error();
        return;
    }

    if (adapter_count == 0) {
        printf("No Bluetooth cards found.\n");
        return;
    }

    /*
     * Table header.
     */
    printf(
        "  %-8s %-18s %-24s %-8s\n",
        "CARD",
        "ADDRESS",
        "ALIAS",
        "POWER"
    );

    /*
     * One row per card.
     */
    for (size_t i = 0; i < adapter_count; i++) {
        bool selected =
            strcmp(adapters[i].name, shell->settings.adapter) == 0;

        printf(
            "%c %-8s %-18s %-24s %-8s\n",
            selected ? '*' : ' ',
            adapters[i].name,
            adapters[i].address,
            adapters[i].alias,
            adapters[i].powered ? "on" : "off"
        );
    }
}

/*
 * Prints the table of the Bluetooth diagnostic devices, such as
 * Ubertooth One. Nothing is printed when there are none.
 */
// TODO (stage 6): bt_find_sniffers() does not detect anything
//                 yet, so this table never appears. See sniffer.c.
static void print_sniffers(void)
{
    bt_sniffer_info_t sniffers[MAX_BT_SNIFFERS];
    size_t sniffer_count = 0;

    if (bt_find_sniffers(
            sniffers,
            MAX_BT_SNIFFERS,
            &sniffer_count) != 0) {

        perror("bt_find_sniffers");
        return;
    }

    if (sniffer_count == 0) {
        return;
    }

    /*
     * Table header, separated from the table of cards.
     */
    printf(
        "\n"
        "  %-24s %-32s\n",
        "DIAGNOSTIC DEVICE",
        "SERIAL"
    );

    /*
     * One row per device.
     */
    for (size_t i = 0; i < sniffer_count; i++) {
        printf(
            "  %-24s %-32s\n",
            sniffers[i].model,
            sniffers[i].serial
        );
    }
}

/*
 * Shell command "cards".
 *
 * Lists the Bluetooth hardware of the computer: first the cards
 * handled by BlueZ, then the diagnostic devices. The two lists
 * are independent, so a problem with one does not hide the other.
 */
static void shell_cards(const bt_shell_t *shell)
{
    print_adapters(shell);
    print_sniffers();
}

/*
 * Shell command "select".
 *
 * Remembers the Bluetooth card in the settings file of the
 * project. Only a card that exists can be selected.
 */
static void shell_select(bt_shell_t *shell, const char *name)
{
    bt_adapter_info_t adapter;

    /*
     * Make sure the card exists before storing its name.
     */
    if (bt_get_adapter(name, &adapter) != 0) {

        if (errno == ENOENT) {
            fprintf(
                stderr,
                "No such Bluetooth card: '%s' "
                "(list the cards with: cards)\n",
                name
            );
        } else {
            report_bluez_error();
        }

        return;
    }

    /*
     * Save first. When saving fails, the shell keeps using
     * the card that is still stored in the file.
     */
    if (save_setting(
            shell->settings_path,
            KEY_ADAPTER,
            adapter.name) != 0) {
        return;
    }

    snprintf(
        shell->settings.adapter,
        sizeof(shell->settings.adapter),
        "%s",
        adapter.name
    );

    printf("Selected %s (%s)\n", adapter.name, adapter.address);

    if (!adapter.powered) {
        printf(
            "Warning: %s is powered off. Turn it on before "
            "using it, for example with: bluetoothctl power on\n",
            adapter.name
        );
    }
}

/*
 * Shell command "status".
 *
 * Shows the selected card and its current state.
 */
static void shell_status(const bt_shell_t *shell)
{
    const char *name = shell->settings.adapter;
    bt_adapter_info_t adapter;

    if (name[0] == '\0') {
        printf(
            "Card:    not selected "
            "(select one with: select CARD)\n"
        );
    } else if (bt_get_adapter(name, &adapter) == 0) {
        printf(
            "Card:    %s\n"
            "Address: %s\n"
            "Alias:   %s\n"
            "Power:   %s\n",
            adapter.name,
            adapter.address,
            adapter.alias,
            adapter.powered ? "on" : "off"
        );
    } else if (errno == ENOENT) {
        /*
         * The card was selected earlier, but it is gone now,
         * for example an unplugged USB dongle.
         */
        printf("Card:    %s (not present)\n", name);
    } else {
        report_bluez_error();
    }

    // TODO (stage 3): show the target device and whether
    //                 it is paired and connected.
}

/*
 * Interactive shell of a Bluetooth project.
 *
 * Runs until the "exit" command or the end of input (Ctrl+D).
 */
static void run_shell(bt_shell_t *shell)
{
    char line[SHELL_LINE_MAX];

    printf("Type 'help' to list commands\n");

    for (;;) {
        bt_command_t command;

        /*
         * Show the prompt and wait for one line.
         */
        printf(SHELL_PROMPT);
        fflush(stdout);

        errno = 0;

        if (fgets(line, sizeof(line), stdin) == NULL) {

            /*
             * Ctrl+C at the prompt: start a new line
             * and keep the shell open.
             */
            if (ferror(stdin) && errno == EINTR) {
                clearerr(stdin);
                printf("\n");
                continue;
            }

            /*
             * End of input (Ctrl+D).
             */
            printf("\n");
            return;
        }

        /*
         * A line without '\n' did not fit into the buffer.
         * Skip the rest of it.
         */
        if (strchr(line, '\n') == NULL && !feof(stdin)) {
            int character;

            while ((character = getchar()) != '\n' &&
                   character != EOF) {
            }

            fprintf(stderr, "Command is too long\n");
            continue;
        }

        /*
         * Turn the line into a command. Errors were already
         * reported by the parser, so just show the prompt again.
         */
        if (bt_parse_line(line, &command) != 0) {
            continue;
        }

        /*
         * Execute the command.
         */
        switch (command.type) {

        case BT_COMMAND_EMPTY:
            break;

        case BT_COMMAND_HELP:
            /*
             * argument is NULL for plain "help".
             */
            bt_print_shell_help(command.argument);
            break;

        case BT_COMMAND_CARDS:
            shell_cards(shell);
            break;

        case BT_COMMAND_SELECT:
            shell_select(shell, command.argument);
            break;

        case BT_COMMAND_STATUS:
            shell_status(shell);
            break;

        // TODO (stage 2-5): cases for the commands listed at
        //                   the top of this file.

        case BT_COMMAND_EXIT:
            return;
        }
    }
}

/*
 * protocol_t.run
 *
 * Called by the "run" command for a Bluetooth project. Loads the
 * settings and runs the shell.
 */
static int bluetooth_run(const project_info_t *project)
{
    bt_shell_t shell;

    memset(&shell, 0, sizeof(shell));

    if (project_get_settings_path(
            project->id,
            shell.settings_path,
            sizeof(shell.settings_path)) != 0) {

        perror("project_get_settings_path");
        return -1;
    }

    if (load_settings(shell.settings_path, &shell.settings) != 0) {
        return -1;
    }

    /*
     * Handle Ctrl+C (SIGINT).
     *
     * sa_flags has no SA_RESTART, so Ctrl+C also interrupts
     * waiting for a command at the prompt.
     */
    struct sigaction sa = {0};

    sa.sa_handler = handle_signal;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;

    if (sigaction(SIGINT, &sa, NULL) != 0) {
        perror("sigaction");
        return -1;
    }

    /*
     * Start by showing where the project stands: without
     * a selected card, list what can be selected; otherwise
     * show the state of the selected card.
     */
    if (shell.settings.adapter[0] == '\0') {
        printf("No Bluetooth card selected.\n\n");
        shell_cards(&shell);
        printf("\nSelect a card with: select CARD\n\n");
    } else {
        shell_status(&shell);
        printf("\n");
    }

    /*
     * Hand control to the shell until "exit".
     */
    run_shell(&shell);

    // TODO (stage 3): disconnect from the target device here.

    return 0;
}

/*
 * Registration of the Bluetooth protocol. The table
 * in protocol/protocol.c refers to this structure.
 */
const protocol_t bluetooth_protocol = {
    .name = "bluetooth",
    .description = "Bluetooth device testing (in development)",
    .init = bluetooth_init,
    .print_help = bluetooth_print_help,
    .run = bluetooth_run,
};
