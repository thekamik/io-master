/*
 * Bluetooth protocol.
 *
 * The protocol is built in stages. This file currently holds
 * the protocol skeleton, the interactive shell, the project
 * settings, the commands that work with the Bluetooth cards
 * of the computer, the scan for devices and the commands that
 * connect and pair a device.
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
 * Access to the Bluetooth cards is in adapter.c and to the
 * devices and their discovery in device.c. Connecting and
 * pairing is in connection.c, and agent.c receives the questions
 * that a pairing with a code asks. The table shown by "scan" is in
 * scan_table.c. Diagnostic devices such as Ubertooth One belong
 * to sniffer.c.
 *
 * Planned work:
 */

// TODO (stage 2): Reconnaissance. "scan" is done. "info" shows
//                 what the scan knows; it should also read the
//                 services and manufacturer data of the device.
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
#include "protocol/bluetooth/connection.h"
#include "protocol/bluetooth/device.h"
#include "protocol/bluetooth/scan_table.h"
#include "protocol/bluetooth/sniffer.h"
#include "storage/json_file.h"

#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <time.h>
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
 * How often "scan" asks BlueZ for the devices and redraws
 * the table, in seconds.
 */
#define SCAN_REFRESH_SECONDS 1

/*
 * Set by Ctrl+C (SIGINT).
 *
 * Ctrl+C never closes the shell. It stops the command that
 * is running, such as "scan", and returns to the prompt.
 */
// TODO (stage 5): "monitor" will check this flag as well.
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
     * Address of the target device, "AA:BB:CC:DD:EE:FF": the
     * device that was connected or paired last. "connect" and
     * "disconnect" use it when no device is named.
     * Empty when no device was chosen yet.
     */
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
 * Checks that a text has the form of a Bluetooth address:
 * six pairs of hexadecimal digits separated by colons,
 * "AA:BB:CC:DD:EE:FF".
 */
static bool is_bluetooth_address(const char *text)
{
    if (strlen(text) != BT_ADDRESS_MAX - 1) {
        return false;
    }

    for (size_t i = 0; i < BT_ADDRESS_MAX - 1; i++) {
        /*
         * Every third character is a colon, the others
         * are hexadecimal digits.
         */
        if (i % 3 == 2) {

            if (text[i] != ':') {
                return false;
            }

        } else if (!isxdigit((unsigned char)text[i])) {
            return false;
        }
    }

    return true;
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

    /*
     * Devices found during this session. Filled by "scan" and
     * kept until the shell is closed, so a device keeps its id
     * from one scan to the next. The list is not stored on disk:
     * every session starts with an empty list.
     */
    bt_scan_table_t devices;

    /*
     * Address of the device that this session connected to,
     * or empty. The connection is closed when the shell ends.
     * Devices that were already connected before, for example
     * a mouse in use, are never recorded here.
     */
    char connected_here[BT_ADDRESS_MAX];
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
        "               (set by 'connect' and 'pair' in the shell)\n"
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
 * Reports why a scan could not be started.
 *
 * errno has to hold the error set by bt_discovery_start().
 */
static void report_scan_error(const char *adapter)
{
    switch (errno) {

    case ENODEV:
        fprintf(
            stderr,
            "Bluetooth card %s is not present "
            "(list the cards with: cards)\n",
            adapter
        );
        break;

    case ENETDOWN:
        fprintf(
            stderr,
            "Bluetooth card %s is powered off. Turn it on, "
            "for example with: bluetoothctl power on\n",
            adapter
        );
        break;

    case EACCES:
    case EPERM:
        fprintf(
            stderr,
            "The system does not allow this user to scan "
            "with %s\n",
            adapter
        );
        break;

    default:
        fprintf(
            stderr,
            "Cannot start the scan on %s: %s\n",
            adapter,
            strerror(errno)
        );
        break;
    }
}

/*
 * Returns the number of seconds since an arbitrary fixed point.
 * Used to measure how long a scan has been running. The
 * monotonic clock is not affected by changes of the system time.
 */
static long monotonic_seconds(void)
{
    struct timespec now;

    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
        return 0;
    }

    return (long)now.tv_sec;
}

/*
 * Shell command "scan".
 *
 * Looks for Bluetooth devices with the selected card until
 * Ctrl+C is pressed.
 *
 * Found devices are merged into the device table of the shell.
 * A device that is new to the table receives the next free id;
 * a device that is already there keeps the id it has.
 *
 * On a terminal the devices are shown in a table that is redrawn
 * in place on a separate screen. When the scan ends, the terminal
 * returns to the shell and the devices found by this scan are
 * printed once more as a static table. From that moment the ids
 * no longer change, and they remain visible for the next commands.
 *
 * When the output is not a terminal (a pipe or a file), there is
 * no live table and only the final table is printed.
 */
static void shell_scan(bt_shell_t *shell)
{
    const char *adapter = shell->settings.adapter;

    bt_scan_table_t *table = &shell->devices;

    /*
     * Fresh list of devices read from BlueZ on every refresh.
     * static: the array is too large for the stack. The shell
     * runs one command at a time, so sharing it between calls
     * is safe.
     */
    static bt_device_info_t devices[MAX_BT_DEVICES];

    bt_discovery_t *discovery = NULL;
    sigset_t block_mask;
    sigset_t original_mask;

    if (adapter[0] == '\0') {
        fprintf(
            stderr,
            "No Bluetooth card selected. "
            "Select one with: select CARD\n"
        );
        return;
    }

    /*
     * SIGINT is blocked outside of pselect(), so Ctrl+C cannot
     * slip in between checking the flag and starting to wait.
     * It also keeps the D-Bus calls from being interrupted.
     */
    sigemptyset(&block_mask);
    sigaddset(&block_mask, SIGINT);

    if (sigprocmask(SIG_BLOCK, &block_mask, &original_mask) != 0) {
        perror("sigprocmask");
        return;
    }

    /*
     * Forget a Ctrl+C that was pressed before this scan started.
     */
    interrupted = 0;

    if (bt_discovery_start(adapter, &discovery) != 0) {
        report_scan_error(adapter);
        sigprocmask(SIG_SETMASK, &original_mask, NULL);
        return;
    }

    /*
     * Keep the known devices and their ids, but start with
     * nothing marked as found by this scan.
     */
    bt_scan_table_begin_scan(table);

    /*
     * The live table needs a terminal that understands
     * control sequences.
     */
    bool live = isatty(STDOUT_FILENO) != 0;

    if (live) {
        bt_scan_screen_enter();
    } else {
        printf(
            "Scanning on %s (press Ctrl+C to stop)...\n",
            adapter
        );
        fflush(stdout);
    }

    long started = monotonic_seconds();

    /*
     * Set when reading the devices fails. The message is
     * printed after the live screen is closed.
     */
    int read_error = 0;

    while (!interrupted) {
        size_t device_count = 0;

        /*
         * Take the current list of devices from BlueZ and
         * merge it into the table.
         */
        if (bt_discovery_read(
                discovery,
                devices,
                MAX_BT_DEVICES,
                &device_count) != 0) {

            read_error = errno;
            break;
        }

        bt_scan_table_update(table, devices, device_count);

        if (live) {
            bt_scan_screen_draw(
                table,
                adapter,
                monotonic_seconds() - started
            );
        }

        /*
         * Wait for the next refresh. pselect() is used only as
         * a sleep that Ctrl+C can end early: SIGINT is unblocked
         * while it waits. Any return value is fine here, the
         * loop condition checks the flag.
         */
        struct timespec refresh = {
            .tv_sec = SCAN_REFRESH_SECONDS,
            .tv_nsec = 0,
        };

        pselect(0, NULL, NULL, NULL, &refresh, &original_mask);
    }

    /*
     * Back to the normal screen before anything else is printed.
     */
    if (live) {
        bt_scan_screen_leave();
    }

    bt_discovery_stop(discovery);

    /*
     * Restore the signal mask, so that Ctrl+C works at the
     * prompt again.
     */
    sigprocmask(SIG_SETMASK, &original_mask, NULL);

    if (read_error != 0) {
        fprintf(
            stderr,
            "The scan was interrupted: %s\n",
            strerror(read_error)
        );
    }

    /*
     * The static table, printed as ordinary text in the shell.
     */
    printf(
        "Scan on %s stopped after %lds, %zu devices found:\n\n",
        adapter,
        monotonic_seconds() - started,
        bt_scan_table_found_count(table)
    );

    bt_scan_table_print(table, BT_SCAN_SHOW_FOUND);

    /*
     * Remind the user that the ids can be used right away.
     */
    if (bt_scan_table_found_count(table) > 0) {
        printf(
            "\nA device can be named by its ID or its address, "
            "for example: info 1\n"
        );
    }
}

/*
 * Shell command "devices".
 *
 * Prints all devices found during this session, including the
 * ones that the last scan did not see, without scanning again.
 */
static void shell_devices(const bt_shell_t *shell)
{
    if (shell->devices.count == 0) {
        printf("No known devices. Find some with: scan\n");
        return;
    }

    bt_scan_table_print(&shell->devices, BT_SCAN_SHOW_ALL);
}

/*
 * Shell command "forget".
 *
 * Empties the list of known devices. The next scan numbers
 * the devices from 1 again.
 */
static void shell_forget(bt_shell_t *shell)
{
    size_t forgotten = shell->devices.count;

    bt_scan_table_init(&shell->devices);

    printf("Forgot %zu devices. IDs start from 1 again.\n", forgotten);
}

/*
 * A device named by the user, resolved to an address.
 */
typedef struct {
    /*
     * Bluetooth address in upper case, "AA:BB:CC:DD:EE:FF".
     */
    char address[BT_ADDRESS_MAX];

    /*
     * Id of the device, or 0 when the address is not in the
     * table of known devices.
     */
    size_t id;

    /*
     * Row of the device in the table, or NULL when the
     * address is not in the table.
     */
    const bt_scan_entry_t *entry;
} bt_device_reference_t;

/*
 * Resolves the way a user names a device. Every command that
 * takes a device uses this function, so all of them accept
 * the same two forms:
 *
 *     3                    the id shown by "scan" and "devices"
 *     AA:BB:CC:DD:EE:FF    the Bluetooth address
 *
 * The two cannot be confused: an id has only digits and an
 * address always contains colons. An address does not have to
 * be in the table, so a device can be used without scanning.
 *
 * Problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
static int resolve_device(
    const bt_shell_t *shell,
    const char *text,
    bt_device_reference_t *reference
)
{
    memset(reference, 0, sizeof(*reference));

    /*
     * Only digits: an id.
     */
    if (text[strspn(text, "0123456789")] == '\0') {
        char *end = NULL;

        errno = 0;

        unsigned long id = strtoul(text, &end, 10);

        const bt_scan_entry_t *entry = NULL;

        /*
         * ERANGE: a number too large to be an id.
         */
        if (errno == 0) {
            entry = bt_scan_table_find_by_id(
                &shell->devices,
                (size_t)id
            );
        }

        if (entry == NULL) {
            fprintf(
                stderr,
                "No device with ID %s "
                "(list the devices with: devices)\n",
                text
            );
            return -1;
        }

        reference->id = (size_t)id;
        reference->entry = entry;

        snprintf(
            reference->address,
            sizeof(reference->address),
            "%s",
            entry->device.address
        );

        return 0;
    }

    /*
     * Otherwise it has to be an address.
     */
    if (!is_bluetooth_address(text)) {
        fprintf(
            stderr,
            "'%s' is neither a device ID nor a Bluetooth address "
            "(expected for example 3 or AA:BB:CC:DD:EE:FF)\n",
            text
        );
        return -1;
    }

    /*
     * BlueZ writes addresses in upper case.
     */
    for (size_t i = 0; text[i] != '\0'; i++) {
        reference->address[i] =
            (char)toupper((unsigned char)text[i]);
    }

    /*
     * The device may be known; then it also has an id.
     */
    reference->entry = bt_scan_table_find_by_address(
        &shell->devices,
        reference->address,
        &reference->id
    );

    return 0;
}

/*
 * Shell command "info".
 *
 * Shows what is known about one device, named by its id or
 * its address.
 */
// TODO (stage 2): read the device from BlueZ and also show its
//                 services (UUIDs) and manufacturer data. For now
//                 the command shows what the scan has collected.
static void shell_info(const bt_shell_t *shell, const char *text)
{
    bt_device_reference_t reference;

    if (resolve_device(shell, text, &reference) != 0) {
        return;
    }

    /*
     * A valid address that no scan has seen.
     */
    if (reference.entry == NULL) {
        printf(
            "Address: %s\n"
            "This device is not in the list of known devices. "
            "Run 'scan' to find it.\n",
            reference.address
        );
        return;
    }

    const bt_device_info_t *device = &reference.entry->device;

    printf(
        "ID:           %zu\n"
        "Address:      %s\n"
        "Name:         %s\n"
        "Type:         %s\n"
        "Address type: %s\n",
        reference.id,
        device->address,
        device->name[0] != '\0' ? device->name : "-",
        device->classic ? "classic" : "LE",
        device->address_type[0] != '\0' ? device->address_type : "-"
    );

    /*
     * The remaining facts come from the last scan. A device
     * that the last scan did not see has none of them.
     */
    if (!reference.entry->found) {
        printf("State:        not seen by the last scan\n");
        return;
    }

    if (device->has_rssi) {
        printf("RSSI:         %d dBm\n", device->rssi);
    } else {
        printf("RSSI:         -\n");
    }

    const char *state = "lost";

    if (device->connected) {
        state = "connected";
    } else if (reference.entry->in_range) {
        state = "visible";
    }

    printf(
        "Paired:       %s\n"
        "State:        %s\n",
        device->paired ? "yes" : "no",
        state
    );
}

/*
 * Finds the device that a connection command works on.
 *
 * text is the argument typed by the user: an id or an address,
 * resolved by resolve_device(). When text is NULL, the target
 * device of the project is used: the device that was connected
 * or paired last.
 *
 * The command also needs a selected card; its absence is
 * reported here, so every connection command behaves the same.
 *
 * Problems are reported on stderr.
 *
 * Returns 0 on success and -1 on error.
 */
static int resolve_target(
    const bt_shell_t *shell,
    const char *text,
    bt_device_reference_t *reference
)
{
    if (shell->settings.adapter[0] == '\0') {
        fprintf(
            stderr,
            "No Bluetooth card selected. "
            "Select one with: select CARD\n"
        );
        return -1;
    }

    if (text != NULL) {
        return resolve_device(shell, text, reference);
    }

    if (shell->settings.device[0] == '\0') {
        fprintf(
            stderr,
            "No device named and no earlier device to reuse. "
            "Name one by its ID or address, for example: "
            "connect 1\n"
        );
        return -1;
    }

    /*
     * The stored address goes through the same resolver, so
     * the device gets its id when the session knows it.
     */
    return resolve_device(shell, shell->settings.device, reference);
}

/*
 * Describes a device for messages: its id when it has one, its
 * address, and its name when it is known.
 *
 *     "3: AA:BB:CC:DD:EE:FF (Sensor)"
 *     "AA:BB:CC:DD:EE:FF"
 */
static void describe_device(
    const bt_device_reference_t *reference,
    char *buffer,
    size_t size
)
{
    const char *name = "";

    if (reference->entry != NULL) {
        name = reference->entry->device.name;
    }

    if (reference->id != 0 && name[0] != '\0') {
        snprintf(
            buffer,
            size,
            "%zu: %s (%s)",
            reference->id,
            reference->address,
            name
        );
    } else if (reference->id != 0) {
        snprintf(
            buffer,
            size,
            "%zu: %s",
            reference->id,
            reference->address
        );
    } else {
        snprintf(buffer, size, "%s", reference->address);
    }
}

/*
 * Reports a failed connection command.
 *
 * action says what was attempted ("connect to", "pair with").
 * errno has to hold the error set by connection.c, and
 * error_text the explanation that came with it.
 */
static void report_device_error(
    const char *action,
    const char *device,
    const char *adapter,
    const char *error_text
)
{
    switch (errno) {

    case ENOENT:
        fprintf(
            stderr,
            "Cannot %s %s: the system does not know this "
            "device. Devices found by a scan are forgotten "
            "after about half a minute; run 'scan' again and "
            "repeat the command.\n",
            action,
            device
        );
        break;

    case ENETDOWN:
        fprintf(
            stderr,
            "Cannot %s %s: Bluetooth card %s is powered off\n",
            action,
            device,
            adapter
        );
        break;

    case ETIMEDOUT:
        fprintf(
            stderr,
            "Cannot %s %s: the device did not answer in time\n",
            action,
            device
        );
        break;

    case ECANCELED:
        fprintf(stderr, "\nCancelled\n");
        break;

    default:
        /*
         * Show the explanation given by BlueZ.
         */
        fprintf(
            stderr,
            "Cannot %s %s: %s\n",
            action,
            device,
            error_text[0] != '\0' ? error_text : strerror(errno)
        );
        break;
    }
}

/*
 * Reads the current state of a device from BlueZ and copies it
 * into the table of known devices, so "devices" and "info" show
 * the result of the last connection command.
 *
 * Does nothing when the device is not known to BlueZ.
 */
static void refresh_device(bt_shell_t *shell, const char *address)
{
    bt_device_info_t device;

    if (bt_device_get(
            shell->settings.adapter,
            address,
            &device) == 0) {

        bt_scan_table_refresh(&shell->devices, &device);
    }
}

/*
 * Makes a device the target device of the project and stores
 * its address in the settings file. A failure to store it is
 * reported, but the device is still used for this session.
 */
static void remember_target(bt_shell_t *shell, const char *address)
{
    if (strcmp(shell->settings.device, address) == 0) {
        return;
    }

    save_setting(shell->settings_path, KEY_DEVICE, address);

    snprintf(
        shell->settings.device,
        sizeof(shell->settings.device),
        "%s",
        address
    );
}

/*
 * Shell command "connect".
 *
 * Connects to a device named by its id or address, or to the
 * target device of the project when no device is named.
 * Ctrl+C aborts the attempt.
 */
static void shell_connect(bt_shell_t *shell, const char *text)
{
    const char *adapter = shell->settings.adapter;

    bt_device_reference_t reference;
    bt_device_info_t device;
    char description[BT_DEVICE_NAME_MAX + 48];
    char error_text[BT_ERROR_TEXT_MAX];

    if (resolve_target(shell, text, &reference) != 0) {
        return;
    }

    describe_device(&reference, description, sizeof(description));

    /*
     * A device that is connected already, perhaps by another
     * program, is left alone. It is not recorded as connected
     * by this session, so leaving the shell will not close it.
     */
    if (bt_device_get(adapter, reference.address, &device) == 0 &&
        device.connected) {

        printf("Already connected to %s\n", description);

        remember_target(shell, reference.address);
        refresh_device(shell, reference.address);

        return;
    }

    printf(
        "Connecting to %s (press Ctrl+C to cancel)...\n",
        description
    );
    fflush(stdout);

    /*
     * Forget a Ctrl+C that was pressed earlier.
     */
    interrupted = 0;

    if (bt_device_connect(
            adapter,
            reference.address,
            &interrupted,
            error_text) != 0 &&
        errno != EISCONN) {

        report_device_error(
            "connect to",
            description,
            adapter,
            error_text
        );
        return;
    }

    printf("Connected to %s\n", description);

    /*
     * Remember the connection, so that it can be closed when
     * the shell ends, and make the device the target device.
     */
    snprintf(
        shell->connected_here,
        sizeof(shell->connected_here),
        "%s",
        reference.address
    );

    remember_target(shell, reference.address);
    refresh_device(shell, reference.address);
}

/*
 * Shell command "disconnect".
 *
 * Closes the connection to a device named by its id or address,
 * or to the target device when no device is named.
 */
static void shell_disconnect(bt_shell_t *shell, const char *text)
{
    const char *adapter = shell->settings.adapter;

    bt_device_reference_t reference;
    char description[BT_DEVICE_NAME_MAX + 48];
    char error_text[BT_ERROR_TEXT_MAX];

    if (resolve_target(shell, text, &reference) != 0) {
        return;
    }

    describe_device(&reference, description, sizeof(description));

    interrupted = 0;

    if (bt_device_disconnect(
            adapter,
            reference.address,
            &interrupted,
            error_text) != 0) {

        if (errno == ENOTCONN) {
            printf("Not connected to %s\n", description);
        } else {
            report_device_error(
                "disconnect from",
                description,
                adapter,
                error_text
            );
            return;
        }
    } else {
        printf("Disconnected from %s\n", description);
    }

    /*
     * The connection made by this session is gone.
     */
    if (strcmp(shell->connected_here, reference.address) == 0) {
        shell->connected_here[0] = '\0';
    }

    refresh_device(shell, reference.address);
}

/*
 * Reads one line typed by the user as the answer to a pairing
 * question. question is printed first, without a line break,
 * so the answer is typed next to it.
 *
 * The line break and surrounding spaces are removed.
 *
 * Returns 0 when an answer was typed and -1 when there is none:
 * an empty line, Ctrl+C or the end of input.
 */
static int read_answer(
    const char *question,
    char *answer,
    size_t size
)
{
    char line[SHELL_LINE_MAX];

    printf("%s", question);
    fflush(stdout);

    if (fgets(line, sizeof(line), stdin) == NULL) {
        /*
         * Ctrl+C interrupts the read, because the SIGINT
         * handler is installed without SA_RESTART. Clear the
         * error, so the shell can read commands again.
         */
        clearerr(stdin);
        printf("\n");
        return -1;
    }

    /*
     * Cut the answer out of the line: skip leading spaces and
     * stop at the first space or line break after it.
     */
    const char *start = line + strspn(line, " \t");
    size_t length = strcspn(start, " \t\r\n");

    if (length == 0 || length >= size) {
        return -1;
    }

    memcpy(answer, start, length);
    answer[length] = '\0';

    return 0;
}

/*
 * Asks a yes/no question. Only an answer that starts with
 * 'y' or 'Y' counts as yes; anything else, including no answer,
 * is a no.
 *
 * Returns 0 for yes and -1 for no.
 */
static int read_yes(const char *question)
{
    char answer[8];

    if (read_answer(question, answer, sizeof(answer)) != 0) {
        return -1;
    }

    return (answer[0] == 'y' || answer[0] == 'Y') ? 0 : -1;
}

/*
 * bt_pairing_prompts_t.request_pin
 *
 * The device needs a PIN. Called only for devices that
 * require one.
 */
static int prompt_request_pin(
    const char *address,
    char *pin,
    size_t size
)
{
    printf(
        "Device %s requires a PIN "
        "(press Enter without a PIN to cancel).\n",
        address
    );

    if (read_answer("PIN: ", pin, size) != 0) {
        printf("No PIN given, the pairing is cancelled.\n");
        return -1;
    }

    return 0;
}

/*
 * bt_pairing_prompts_t.request_passkey
 *
 * The device shows a number that has to be typed here.
 */
static int prompt_request_passkey(
    const char *address,
    uint32_t *passkey
)
{
    char answer[16];
    char *end = NULL;

    printf(
        "Device %s shows a passkey of up to 6 digits. Type it "
        "here (press Enter without a passkey to cancel).\n",
        address
    );

    if (read_answer("Passkey: ", answer, sizeof(answer)) != 0) {
        printf("No passkey given, the pairing is cancelled.\n");
        return -1;
    }

    /*
     * A passkey is a number from 0 to 999999.
     */
    errno = 0;

    unsigned long value = strtoul(answer, &end, 10);

    if (errno != 0 ||
        answer[strspn(answer, "0123456789")] != '\0' ||
        *end != '\0' ||
        value > 999999UL) {

        printf(
            "'%s' is not a passkey, the pairing is cancelled.\n",
            answer
        );
        return -1;
    }

    *passkey = (uint32_t)value;

    return 0;
}

/*
 * bt_pairing_prompts_t.confirm_passkey
 *
 * Both sides show a number and the user compares them.
 */
static int prompt_confirm_passkey(
    const char *address,
    uint32_t passkey
)
{
    printf(
        "Device %s should show this number: %06u\n",
        address,
        (unsigned int)passkey
    );

    return read_yes("Does the device show the same number? [y/N]: ");
}

/*
 * bt_pairing_prompts_t.authorize
 *
 * A pairing without a code that still needs consent.
 */
static int prompt_authorize(const char *address)
{
    printf("Device %s asks to be paired.\n", address);

    return read_yes("Accept the pairing? [y/N]: ");
}

/*
 * bt_pairing_prompts_t.display_pin
 *
 * The code has to be typed on the device.
 */
static void prompt_display_pin(const char *address, const char *pin)
{
    printf(
        "Type this PIN on device %s and confirm it there: %s\n",
        address,
        pin
    );
    fflush(stdout);
}

/*
 * bt_pairing_prompts_t.display_passkey
 */
static void prompt_display_passkey(
    const char *address,
    uint32_t passkey
)
{
    printf(
        "Type this passkey on device %s and confirm it there: "
        "%06u\n",
        address,
        (unsigned int)passkey
    );
    fflush(stdout);
}

/*
 * The questions of a pairing, answered on the terminal.
 * Passed to bt_device_pair() by the "pair" command.
 */
static const bt_pairing_prompts_t pairing_prompts = {
    .request_pin = prompt_request_pin,
    .request_passkey = prompt_request_passkey,
    .confirm_passkey = prompt_confirm_passkey,
    .authorize = prompt_authorize,
    .display_pin = prompt_display_pin,
    .display_passkey = prompt_display_passkey,
};

/*
 * Shell command "pair".
 *
 * Pairs with a device named by its id or address. Ctrl+C aborts
 * the attempt.
 *
 * A device that pairs without a code is paired without any
 * question. When the device requires a PIN, a passkey or
 * a confirmation, the matching prompt_*() function above asks
 * the user for it in the middle of the pairing.
 */
static void shell_pair(bt_shell_t *shell, const char *text)
{
    const char *adapter = shell->settings.adapter;

    bt_device_reference_t reference;
    char description[BT_DEVICE_NAME_MAX + 48];
    char error_text[BT_ERROR_TEXT_MAX];

    if (resolve_target(shell, text, &reference) != 0) {
        return;
    }

    describe_device(&reference, description, sizeof(description));

    printf(
        "Pairing with %s (press Ctrl+C to cancel)...\n",
        description
    );
    fflush(stdout);

    interrupted = 0;

    if (bt_device_pair(
            adapter,
            reference.address,
            &pairing_prompts,
            &interrupted,
            error_text) != 0) {

        if (errno == EEXIST) {
            printf("Already paired with %s\n", description);
        } else if (errno == EACCES) {
            /*
             * A wrong PIN, a refused confirmation, or the
             * device said no.
             */
            fprintf(
                stderr,
                "Pairing with %s failed: %s\n",
                description,
                error_text
            );
            return;
        } else {
            report_device_error(
                "pair with",
                description,
                adapter,
                error_text
            );
            return;
        }
    } else {
        printf("Paired with %s\n", description);
    }

    remember_target(shell, reference.address);
    refresh_device(shell, reference.address);
}

/*
 * Shell command "unpair".
 *
 * Removes the pairing with a device named by its id or address.
 * BlueZ forgets the device entirely, which also closes
 * a connection to it.
 */
static void shell_unpair(bt_shell_t *shell, const char *text)
{
    const char *adapter = shell->settings.adapter;

    bt_device_reference_t reference;
    char description[BT_DEVICE_NAME_MAX + 48];
    char error_text[BT_ERROR_TEXT_MAX];

    if (resolve_target(shell, text, &reference) != 0) {
        return;
    }

    describe_device(&reference, description, sizeof(description));

    interrupted = 0;

    if (bt_device_unpair(
            adapter,
            reference.address,
            &interrupted,
            error_text) != 0) {

        if (errno == ENOENT) {
            /*
             * The system has nothing stored for this device.
             */
            printf("Not paired with %s\n", description);
        } else {
            report_device_error(
                "unpair from",
                description,
                adapter,
                error_text
            );
        }

        return;
    }

    printf(
        "Removed the pairing with %s. The system has forgotten "
        "the device; run 'scan' before using it again.\n",
        description
    );

    /*
     * Removing the device also closed its connection.
     */
    if (strcmp(shell->connected_here, reference.address) == 0) {
        shell->connected_here[0] = '\0';
    }

    // TODO: The row of the device in the table of known devices
    //       still shows the state from the last scan. Mark it as
    //       not seen here.
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

    /*
     * The target device: the one connected or paired last.
     */
    const char *address = shell->settings.device;
    bt_device_info_t device;

    if (address[0] == '\0') {
        printf("Device:  none (name one with: connect DEVICE)\n");
    } else if (name[0] != '\0' &&
               bt_device_get(name, address, &device) == 0) {
        /*
         * Fresh state from BlueZ.
         */
        printf(
            "Device:  %s%s%s%s\n"
            "         %s, %s\n",
            device.address,
            device.name[0] != '\0' ? " (" : "",
            device.name,
            device.name[0] != '\0' ? ")" : "",
            device.connected ? "connected" : "not connected",
            device.paired ? "paired" : "not paired"
        );
    } else {
        /*
         * BlueZ knows nothing about the device right now:
         * it is neither connected nor paired, and no recent
         * scan has seen it.
         */
        printf(
            "Device:  %s\n"
            "         not connected, not known to the system "
            "(run 'scan' to find it)\n",
            address
        );
    }
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

        case BT_COMMAND_SCAN:
            shell_scan(shell);
            break;

        case BT_COMMAND_DEVICES:
            shell_devices(shell);
            break;

        case BT_COMMAND_INFO:
            shell_info(shell, command.argument);
            break;

        case BT_COMMAND_FORGET:
            shell_forget(shell);
            break;

        /*
         * For "connect" and "disconnect" the argument is NULL
         * when no device was named.
         */
        case BT_COMMAND_CONNECT:
            shell_connect(shell, command.argument);
            break;

        case BT_COMMAND_DISCONNECT:
            shell_disconnect(shell, command.argument);
            break;

        case BT_COMMAND_PAIR:
            shell_pair(shell, command.argument);
            break;

        case BT_COMMAND_UNPAIR:
            shell_unpair(shell, command.argument);
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
    /*
     * static: the device table makes the structure too large
     * for the stack.
     */
    static bt_shell_t shell;

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
     * Every session starts without known devices.
     */
    bt_scan_table_init(&shell.devices);

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

    /*
     * The shell was closed with "exit" (or Ctrl+D): the session
     * is over, so the list of found devices and their ids are
     * discarded. Nothing of it was written to disk.
     */
    bt_scan_table_init(&shell.devices);

    /*
     * Close the connection that this session opened. Devices
     * that were connected before the session are not touched.
     */
    if (shell.connected_here[0] != '\0') {
        printf("Disconnecting from %s...\n", shell.connected_here);

        if (bt_device_disconnect(
                shell.settings.adapter,
                shell.connected_here,
                NULL,
                NULL) != 0 &&
            errno != ENOTCONN) {

            perror("bt_device_disconnect");
        }
    }

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
