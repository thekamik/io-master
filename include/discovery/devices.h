#ifndef DEVICES_H
#define DEVICES_H

#include <stddef.h>

typedef enum {
    DEVICE_TYPE_TTYACM,
    DEVICE_TYPE_TTYUSB,
    DEVICE_TYPE_ALL
} device_type_t;

#define DEVICE_PATH_MAX 256
#define DEVICE_INFO_MAX 256
#define MAX_SERIAL_DEVICES 32

typedef struct {
    char port[DEVICE_PATH_MAX];
    char vendor[DEVICE_INFO_MAX];
    char model[DEVICE_INFO_MAX];
    char serial[DEVICE_INFO_MAX];
} serial_device_info_t;

int find_serial_devices(
    device_type_t device_type,
    serial_device_info_t *devices,
    size_t max_devices,
    size_t *device_count
);

/*
 * Finds serial devices and prints them as a table on stdout.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int print_serial_devices(device_type_t device_type);

#endif