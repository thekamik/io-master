/*
 * Low-level access to a serial port, built on POSIX termios.
 */

#include "protocol/uart/serial.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <termios.h>
#include <unistd.h>

/*
 * Converts a numeric baud rate to the termios constant.
 * Only the rates listed here are supported.
 */
static int baudrate_to_speed(int baudrate, speed_t *speed)
{
    switch (baudrate) {
        case 9600:
            *speed = B9600;
            return 0;

        case 19200:
            *speed = B19200;
            return 0;

        case 38400:
            *speed = B38400;
            return 0;

        case 57600:
            *speed = B57600;
            return 0;

        case 115200:
            *speed = B115200;
            return 0;

        default:
            return -1;
    }
}

/*
 * Applies the serial configuration (baud rate, data bits, parity,
 * stop bits) to an open port and switches it to raw mode with
 * non-blocking reads.
 */
static int configure_terminal(
    int fd,
    const serial_config_t *config
)
{
    struct termios tty;
    speed_t speed;

    if (baudrate_to_speed(config->baudrate, &speed) != 0) {
        fprintf(
            stderr,
            "Unsupported baud rate: %d\n",
            config->baudrate
        );
        errno = EINVAL;
        return -1;
    }

    if (tcgetattr(fd, &tty) != 0) {
        return -1;
    }

    /*
     * Start with raw mode.
     *
     * This disables processing such as:
     * - canonical input
     * - echo
     * - signal characters
     * - CR/LF translation
     */
    cfmakeraw(&tty);

    /*
     * Baud rate.
     */
    if (cfsetispeed(&tty, speed) != 0 ||
        cfsetospeed(&tty, speed) != 0) {
        return -1;
    }

    /*
     * Data bits.
     */
    tty.c_cflag &= ~CSIZE;

    switch (config->data_bits) {
        case 5:
            tty.c_cflag |= CS5;
            break;

        case 6:
            tty.c_cflag |= CS6;
            break;

        case 7:
            tty.c_cflag |= CS7;
            break;

        case 8:
            tty.c_cflag |= CS8;
            break;

        default:
            errno = EINVAL;
            return -1;
    }

    /*
     * Parity.
     */
    tty.c_cflag &= ~(PARENB | PARODD);

    switch (config->parity) {
        case PARITY_NONE:
            break;

        case PARITY_EVEN:
            tty.c_cflag |= PARENB;
            break;

        case PARITY_ODD:
            tty.c_cflag |= PARENB | PARODD;
            break;

        default:
            errno = EINVAL;
            return -1;
    }

    /*
     * Stop bits.
     */
    if (config->stop_bits == 1) {
        tty.c_cflag &= ~CSTOPB;
    } else if (config->stop_bits == 2) {
        tty.c_cflag |= CSTOPB;
    } else {
        errno = EINVAL;
        return -1;
    }

    /*
     * Enable receiver.
     * Ignore modem control lines.
     */
    tty.c_cflag |= CREAD | CLOCAL;

    /*
    * Read behavior:
    *
    * VMIN  = minimum number of bytes required
    * VTIME = timeout in units of 100 ms
    *
    * With VMIN=0 and VTIME=0:
    * read() returns immediately.
    *
    * If data is available, read() returns the number of bytes
    * available (up to the requested size).
    *
    * If no data is available, read() returns 0.
    *
    * O_NONBLOCK is also set on the file descriptor, so the
    * descriptor itself operates in non-blocking mode.
    */
    tty.c_cc[VMIN] = 0; // Don't wait for at least one byte
    tty.c_cc[VTIME] = 0;    // No timeout

    /*
     * Apply configuration immediately.
     */
    if (tcsetattr(fd, TCSANOW, &tty) != 0) {
        return -1;
    }

    /*
     * Discard data that may have been waiting before
     * we configured the port.
     */
    if (tcflush(fd, TCIOFLUSH) != 0) {
        return -1;
    }

    return 0;
}

int serial_start_connection(
    serial_port_t *port,
    const serial_config_t *config
)
{
    if (port == NULL || config == NULL) {
        errno = EINVAL;
        return -1;
    }

    /*
     * Initialize the structure so that it never contains
     * an accidental file descriptor.
     */
    port->fd = -1;

    /*
     * Open the serial device.
     *
     * O_NOCTTY:
     *   do not make the serial device our controlling terminal.
     *
     * O_CLOEXEC:
     *   automatically close the descriptor if the process
     *   executes another program.
     */
    int fd = open(
        config->device,
        // O_RDWR | O_NOCTTY | O_CLOEXEC    // Blocking mode
        O_RDWR | O_NOCTTY | O_CLOEXEC | O_NONBLOCK // Non-blocking mode
    );

    if (fd == -1) {
        return -1;
    }

    if (configure_terminal(fd, config) != 0) {
        int saved_errno = errno;

        close(fd);
        errno = saved_errno;

        return -1;
    }

    port->fd = fd;
    port->config = *config;

    return 0;
}

/*
 * Thin wrapper around read() that validates its arguments.
 */
ssize_t serial_read(
    serial_port_t *port,
    void *buffer,
    size_t size
)
{
    if (port == NULL || buffer == NULL || size == 0) {
        errno = EINVAL;
        return -1;
    }

    if (port->fd == -1) {
        errno = EBADF;
        return -1;
    }

    return read(port->fd, buffer, size);
}

/*
 * Thin wrapper around write() that validates its arguments.
 */
ssize_t serial_write(
    serial_port_t *port,
    const void *buffer,
    size_t size
)
{
    if (port == NULL || buffer == NULL || size == 0) {
        errno = EINVAL;
        return -1;
    }

    if (port->fd < 0) {
        errno = EBADF;
        return -1;
    }

    return write(port->fd, buffer, size);
}

int serial_end_connection(serial_port_t *port)
{
    if (port == NULL) {
        errno = EINVAL;
        return -1;
    }

    if (port->fd < 0) {
        return 0;
    }

    int result = close(port->fd);

    port->fd = -1;

    return result;
}