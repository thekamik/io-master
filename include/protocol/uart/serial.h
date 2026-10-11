/*
 * Low-level access to a serial port: open and configure,
 * read, write and close.
 */

#ifndef SERIAL_H
#define SERIAL_H

#include <stddef.h>
#include <sys/types.h>

#define SERIAL_DEVICE_MAX 256

typedef enum {
    PARITY_NONE,
    PARITY_EVEN,
    PARITY_ODD
} parity_t;

/*
 * Configuration of a serial port.
 */
typedef struct {
    char device[SERIAL_DEVICE_MAX];  /* for example /dev/ttyACM0 */
    int baudrate;                    /* for example 9600 */
    int data_bits;                   /* 5, 6, 7 or 8 */
    parity_t parity;
    int stop_bits;                   /* 1 or 2 */
} serial_config_t;

/*
 * An open serial port.
 *
 * fd is -1 when the port is not open.
 */
typedef struct {
    int fd;
    serial_config_t config;
} serial_port_t;

/*
 * Opens the port and configures it according to config.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int serial_start_connection(
    serial_port_t *port,
    const serial_config_t *config
);

/*
 * Reads at most size bytes. Does not wait for data.
 *
 * Returns:
 *   >0 - number of bytes read
 *    0 - no data
 *   -1 - error (errno is set)
 */
ssize_t serial_read(
    serial_port_t *port,
    void *buffer,
    size_t size
);

/*
 * Writes at most size bytes.
 *
 * Returns:
 *   >=0 - number of bytes written
 *   -1  - error (errno is set)
 */
ssize_t serial_write(
    serial_port_t *port,
    const void *buffer,
    size_t size
);

/*
 * Closes the port. Closing a port that is not open is not
 * an error.
 *
 * Returns:
 *    0 - success
 *   -1 - error (errno is set)
 */
int serial_end_connection(serial_port_t *port);

#endif