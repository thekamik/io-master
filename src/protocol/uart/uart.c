#include "protocol/uart/uart.h"

#include "discovery/devices.h"
#include "protocol/uart/serial.h"
#include "protocol/uart/uart_parser.h"
#include "storage/json_file.h"

#include <errno.h>
#include <limits.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

#define KEY_DEVICE "device"
#define KEY_BAUDRATE "baudrate"
#define KEY_DATA_BITS "data_bits"
#define KEY_PARITY "parity"
#define KEY_STOP_BITS "stop_bits"

#define SHELL_PROMPT "uart> "
#define SHELL_LINE_MAX 256

/*
 * Set by Ctrl+C (SIGINT).
 *
 * Ctrl+C stops the current operation, such as reading from
 * the serial port. It does not close the shell.
 */
static volatile sig_atomic_t interrupted = 0;

static void handle_signal(int signal) {
    (void)signal; // Unused parameter
    interrupted = 1;
}

static int parse_parity(const char *str, parity_t *parity)
{
    if (strcmp(str, "none") == 0) {
        *parity = PARITY_NONE;
        return 0;
    }

    if (strcmp(str, "even") == 0) {
        *parity = PARITY_EVEN;
        return 0;
    }

    if (strcmp(str, "odd") == 0) {
        *parity = PARITY_ODD;
        return 0;
    }

    return -1;
}

/*
 * Writes the settings file with default values.
 *
 * Defaults corresponding to Arduino:
 *
 * Serial.begin(9600);
 *
 * 9600 8N1
 *
 * The device is left empty, because it depends on the hardware
 * that is connected.
 */
static int write_default_settings(const char *path)
{
    json_t *root = json_pack(
        "{s:s, s:i, s:i, s:s, s:i}",
        KEY_DEVICE, "",
        KEY_BAUDRATE, 9600,
        KEY_DATA_BITS, 8,
        KEY_PARITY, "none",
        KEY_STOP_BITS, 1
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
 */
static int load_settings(const char *path, serial_config_t *config)
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

    const json_t *device = json_object_get(root, KEY_DEVICE);
    const json_t *baudrate = json_object_get(root, KEY_BAUDRATE);
    const json_t *data_bits = json_object_get(root, KEY_DATA_BITS);
    const json_t *parity = json_object_get(root, KEY_PARITY);
    const json_t *stop_bits = json_object_get(root, KEY_STOP_BITS);

    const char *problem = NULL;

    memset(config, 0, sizeof(*config));

    if (!json_is_string(device) ||
        strlen(json_string_value(device)) >= sizeof(config->device)) {

        problem = "'" KEY_DEVICE "' must be a device path";

    } else if (!json_is_integer(baudrate) ||
               json_integer_value(baudrate) < 1 ||
               json_integer_value(baudrate) > INT_MAX) {

        problem = "'" KEY_BAUDRATE "' must be a positive number";

    } else if (!json_is_integer(data_bits) ||
               json_integer_value(data_bits) < 5 ||
               json_integer_value(data_bits) > 8) {

        problem = "'" KEY_DATA_BITS "' must be 5, 6, 7 or 8";

    } else if (!json_is_string(parity) ||
               parse_parity(
                   json_string_value(parity),
                   &config->parity) != 0) {

        problem = "'" KEY_PARITY "' must be \"none\", \"even\" "
                  "or \"odd\"";

    } else if (!json_is_integer(stop_bits) ||
               (json_integer_value(stop_bits) != 1 &&
                json_integer_value(stop_bits) != 2)) {

        problem = "'" KEY_STOP_BITS "' must be 1 or 2";
    }

    if (problem != NULL) {
        fprintf(stderr, "%s: %s\n", path, problem);
        json_decref(root);
        return -1;
    }

    snprintf(
        config->device,
        sizeof(config->device),
        "%s",
        json_string_value(device)
    );

    config->baudrate = (int)json_integer_value(baudrate);
    config->data_bits = (int)json_integer_value(data_bits);
    config->stop_bits = (int)json_integer_value(stop_bits);

    json_decref(root);

    return 0;
}

/*
 * State of the interactive shell.
 *
 * The shell can run without a connection: port.fd is -1 until
 * a serial port is selected and opened.
 */
typedef struct {
    serial_port_t port;
    serial_config_t config;
    char settings_path[PATH_MAX];
} uart_shell_t;

static bool is_connected(const uart_shell_t *shell)
{
    return shell->port.fd >= 0;
}

/*
 * Stores the selected device in the settings file.
 * The other settings are left untouched.
 */
static int save_device(const char *path, const char *device)
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

    if (json_object_set_new(
            root,
            KEY_DEVICE,
            json_string(device)) != 0) {

        fprintf(stderr, "Invalid device path: '%s'\n", device);
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
 * Checks that the path points to a character device, which is
 * what every serial port is.
 */
static int check_device(const char *device)
{
    struct stat info;

    if (stat(device, &info) != 0) {
        fprintf(
            stderr,
            "Cannot use %s: %s\n",
            device,
            strerror(errno)
        );
        return -1;
    }

    if (!S_ISCHR(info.st_mode)) {
        fprintf(stderr, "%s is not a serial device\n", device);
        return -1;
    }

    return 0;
}

static int uart_init(const project_info_t *project)
{
    char path[PATH_MAX];

    if (create_settings(project, path, sizeof(path)) != 0) {
        return -1;
    }

    printf("UART settings: %s\n", path);

    return 0;
}

static void uart_print_help(
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
        "Project %d: %s (uart)\n"
        "\n"
        "Starts an interactive shell for the serial port of the\n"
        "project. Type 'help' in the shell to list its commands.\n"
        "The serial port is chosen in the shell with 'select PORT'.\n"
        "\n"
        "Usage:\n"
        "  %s run\n"
        "\n"
        "Settings file:\n"
        "  %s\n"
        "\n"
        "Settings:\n"
        "  device       Serial device path, for example /dev/ttyACM0\n"
        "               (set by 'select PORT' in the shell)\n"
        "  baudrate     9600, 19200, 38400, 57600 or 115200\n"
        "  data_bits    5, 6, 7 or 8\n"
        "  parity       \"none\", \"even\" or \"odd\"\n"
        "  stop_bits    1 or 2\n",
        project->id,
        project->name,
        program_name,
        path
    );
}

/*
 * Shell command "read".
 *
 * Prints the bytes received from the serial port until Ctrl+C
 * is pressed.
 *
 * Returns:
 *   0  - reading was stopped by Ctrl+C
 *  -1  - error
 */
static int read_until_interrupted(serial_port_t *port)
{
    char buffer[256];
    sigset_t block_mask;
    sigset_t original_mask;
    int status = 0;

    /*
     * SIGINT is blocked outside of pselect(), so Ctrl+C cannot
     * slip in between checking the flag and starting to wait.
     */
    sigemptyset(&block_mask);
    sigaddset(&block_mask, SIGINT);

    if (sigprocmask(SIG_BLOCK, &block_mask, &original_mask) != 0) {
        perror("sigprocmask");
        return -1;
    }

    interrupted = 0;

    printf(
        "Reading from %s (press Ctrl+C to stop)\n",
        port->config.device
    );

    while (!interrupted) {
        fd_set read_fds;
        FD_ZERO(&read_fds);
        FD_SET(port->fd, &read_fds);

        int result = pselect(
            port->fd + 1,
            &read_fds,
            NULL,
            NULL,
            NULL,
            &original_mask
        );

        if (result < 0) {
            if (errno == EINTR) {
                // Interrupted by signal, check the flag again
                continue;
            }
            perror("select");
            status = -1;
            break;
        }

        if (FD_ISSET(port->fd, &read_fds)) {
            ssize_t bytes_read = serial_read(
                port,
                buffer,
                sizeof(buffer)
            );

            if (bytes_read > 0) {
                fwrite(buffer, 1, bytes_read, stdout);
                fflush(stdout);
            } else if (bytes_read == 0) {
                /*
                 * The port is reported as readable, but there
                 * is no data: the device is gone.
                 */
                fprintf(stderr, "\nSerial device disconnected\n");
                status = -1;
                break;
            } else if (errno != EAGAIN &&
                    errno != EWOULDBLOCK) {
                perror("serial_read");
                status = -1;
                break;
            }
        }
    }

    sigprocmask(SIG_SETMASK, &original_mask, NULL);

    if (status == 0) {
        printf("\nReading stopped\n");
    }

    return status;
}

/*
 * Opens the selected serial port.
 */
static int connect_port(uart_shell_t *shell)
{
    const char *device = shell->config.device;

    if (serial_start_connection(
            &shell->port,
            &shell->config) != 0) {

        if (errno == EACCES) {
            fprintf(
                stderr,
                "Permission denied: %s\n"
                "Use the 'permission' command to get access\n",
                device
            );
        } else {
            fprintf(
                stderr,
                "Cannot open %s: %s\n",
                device,
                strerror(errno)
            );
        }

        return -1;
    }

    printf("Connected to %s\n", device);

    return 0;
}

/*
 * Shell command "scan".
 */
static void shell_scan(void)
{
    printf("Available serial devices:\n\n");

    if (print_serial_devices(DEVICE_TYPE_ALL) != 0) {
        perror("find_serial_devices");
        return;
    }

    printf("\nSelect a device with: select PORT\n");
}

/*
 * Shell command "select".
 *
 * Remembers the serial port in the settings file of the project
 * and connects to it.
 */
static void shell_select(uart_shell_t *shell, const char *device)
{
    if (strlen(device) >= sizeof(shell->config.device)) {
        fprintf(stderr, "Device path is too long\n");
        return;
    }

    if (check_device(device) != 0 ||
        save_device(shell->settings_path, device) != 0) {
        return;
    }

    if (is_connected(shell) &&
        serial_end_connection(&shell->port) != 0) {
        perror("serial_end_connection");
    }

    snprintf(
        shell->config.device,
        sizeof(shell->config.device),
        "%s",
        device
    );

    printf("Selected %s\n", device);

    connect_port(shell);
}

/*
 * Shell command "permission".
 *
 * Runs "sudo chmod a+rw -- DEVICE" in the terminal of the user.
 * sudo asks for the password itself.
 *
 * The command is started directly, without a system shell, so
 * the device path is never interpreted as shell syntax.
 */
static void shell_permission(uart_shell_t *shell)
{
    char *device = shell->config.device;

    if (device[0] == '\0') {
        fprintf(
            stderr,
            "No serial device selected. "
            "Select one with: select PORT\n"
        );
        return;
    }

    if (check_device(device) != 0) {
        return;
    }

    if (access(device, R_OK | W_OK) == 0) {
        printf("You already have access to %s\n", device);

        if (!is_connected(shell)) {
            connect_port(shell);
        }

        return;
    }

    printf(
        "Running: sudo chmod a+rw -- %s\n"
        "The permission lasts until the device is reconnected.\n",
        device
    );
    fflush(stdout);

    pid_t child = fork();

    if (child < 0) {
        perror("fork");
        return;
    }

    if (child == 0) {
        char *const arguments[] = {
            "sudo",
            "chmod",
            "a+rw",
            "--",
            device,
            NULL
        };

        /*
         * Ctrl+C at the password prompt has to stop sudo.
         */
        signal(SIGINT, SIG_DFL);

        execvp(arguments[0], arguments);

        perror("sudo");
        _exit(127);
    }

    int status = 0;

    while (waitpid(child, &status, 0) < 0) {

        if (errno != EINTR) {
            perror("waitpid");
            return;
        }
    }

    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        fprintf(stderr, "\nThe permission was not changed\n");
        return;
    }

    printf("Access to %s granted\n", device);

    if (!is_connected(shell)) {
        connect_port(shell);
    }
}

/*
 * Shell command "read".
 */
static void shell_read(uart_shell_t *shell)
{
    if (shell->config.device[0] == '\0') {
        fprintf(
            stderr,
            "No serial device selected. "
            "Select one with: select PORT\n"
        );
        return;
    }

    if (!is_connected(shell) && connect_port(shell) != 0) {
        return;
    }

    read_until_interrupted(&shell->port);
}

/*
 * Interactive shell of a UART project.
 *
 * Runs until the "exit" command or the end of input (Ctrl+D).
 */
static void run_shell(uart_shell_t *shell)
{
    char line[SHELL_LINE_MAX];

    printf("Type 'help' to list commands\n");

    for (;;) {
        uart_command_t command;

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

        if (uart_parse_line(line, &command) != 0) {
            continue;
        }

        switch (command.type) {

        case UART_COMMAND_EMPTY:
            break;

        case UART_COMMAND_HELP:
            uart_print_shell_help();
            break;

        case UART_COMMAND_SCAN:
            shell_scan();
            break;

        case UART_COMMAND_SELECT:
            shell_select(shell, command.argument);
            break;

        case UART_COMMAND_PERMISSION:
            shell_permission(shell);
            break;

        case UART_COMMAND_READ:
            shell_read(shell);
            break;

        case UART_COMMAND_EXIT:
            return;
        }
    }
}

static int uart_run(const project_info_t *project)
{
    uart_shell_t shell;

    memset(&shell, 0, sizeof(shell));

    shell.port.fd = -1;

    if (project_get_settings_path(
            project->id,
            shell.settings_path,
            sizeof(shell.settings_path)) != 0) {

        perror("project_get_settings_path");
        return -1;
    }

    if (load_settings(shell.settings_path, &shell.config) != 0) {
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
     * Without a selected device the shell starts by showing
     * what can be selected.
     */
    if (shell.config.device[0] == '\0') {
        printf("No serial device selected.\n\n");
        shell_scan();
        printf("\n");
    } else {
        connect_port(&shell);
    }

    run_shell(&shell);

    if (is_connected(&shell)) {
        printf("Closing connection...\n");

        if (serial_end_connection(&shell.port) != 0) {
            perror("serial_end_connection");
            return -1;
        }
    }

    return 0;
}

const protocol_t uart_protocol = {
    .name = "uart",
    .description = "Serial port (UART) reader",
    .init = uart_init,
    .print_help = uart_print_help,
    .run = uart_run,
};
