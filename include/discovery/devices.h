/*
 * Discovery of serial devices connected to the computer.
 */

#ifndef DEVICES_H
#define DEVICES_H

#include <stddef.h>

/*
 * Which serial devices a scan looks for.
 */
typedef enum {
    DEVICE_TYPE_TTYACM,  /* /dev/ttyACM*, USB CDC ACM devices */
    DEVICE_TYPE_TTYUSB,  /* /dev/ttyUSB*, USB-to-serial adapters */
    DEVICE_TYPE_ALL      /* both of the above */
} device_type_t;

/*
 * Sizes of the buffers in serial_device_info_t and the default
 * size of the device array used by the callers.
 */
#define DEVICE_PATH_MAX 256
#define DEVICE_INFO_MAX 256
#define MAX_SERIAL_DEVICES 32

/*
 * Description of one serial device.
 *
 * A property that the system does not know is stored as "-".
 */
typedef struct {
    char port[DEVICE_PATH_MAX];
    char vendor[DEVICE_INFO_MAX];
    char model[DEVICE_INFO_MAX];
    char serial[DEVICE_INFO_MAX];
} serial_device_info_t;

/*
 * Finds serial devices of the given type.
 *
 * devices is an array of max_devices elements provided by the
 * caller. On success *device_count is the number of elements
 * that were filled.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOSPC when more than max_devices
 *        devices were found)
 */
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