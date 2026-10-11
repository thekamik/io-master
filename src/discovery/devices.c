/*
 * Discovery of serial devices.
 *
 * Devices are enumerated with the sd-device API of libsystemd,
 * which reads the udev database: the "tty" subsystem is searched
 * for /dev/ttyACM* and /dev/ttyUSB* nodes, and the vendor, model
 * and serial number come from the udev properties of each device.
 */

#include "discovery/devices.h"

#include <errno.h>
#include <stdio.h>
#include <string.h>

#include <systemd/sd-device.h>

/*
 * Copies one udev property of a device (for example ID_VENDOR)
 * into a fixed-size buffer.
 *
 * Returns 0 on success, or a negative errno-style value
 * reported by sd-device.
 */
static int copy_device_property(
    sd_device *device,
    const char *property,
    char *destination,
    size_t destination_size
)
{
    const char *value = NULL;

    int result = sd_device_get_property_value(
        device,
        property,
        &value
    );

    if (result == -ENOENT) {
        /*
         * Property does not exist.
         * This corresponds to the "-" used by the
         * original Bash script.
         */
        snprintf(destination, destination_size, "-");
        return 0;
    }

    if (result < 0) {
        return result;
    }

    if (value == NULL) {
        snprintf(destination, destination_size, "-");
        return 0;
    }

    snprintf(
        destination,
        destination_size,
        "%s",
        value
    );

    return 0;
}

/*
 * Checks whether a device node belongs to the requested type:
 *
 *     /dev/ttyACM*  USB CDC ACM devices, for example Arduino
 *     /dev/ttyUSB*  USB-to-serial adapters
 *
 * Returns 1 when the device is supported and 0 otherwise.
 */
static int is_supported_serial_device(
    const char *devname,
    device_type_t device_type
)
{
    if (devname == NULL) {
        return 0;
    }

    switch (device_type) {

    case DEVICE_TYPE_TTYACM:
        return strncmp(
            devname,
            "/dev/ttyACM",
            strlen("/dev/ttyACM")
        ) == 0;

    case DEVICE_TYPE_TTYUSB:
        return strncmp(
            devname,
            "/dev/ttyUSB",
            strlen("/dev/ttyUSB")
        ) == 0;

    case DEVICE_TYPE_ALL:
        return (
            strncmp(
                devname,
                "/dev/ttyACM",
                strlen("/dev/ttyACM")
            ) == 0
        ) || (
            strncmp(
                devname,
                "/dev/ttyUSB",
                strlen("/dev/ttyUSB")
            ) == 0
        );

    default:
        return 0;
    }
}

int find_serial_devices(
    device_type_t device_type,
    serial_device_info_t *devices,
    size_t max_devices,
    size_t *device_count
)
{
    sd_device_enumerator *enumerator = NULL;
    sd_device *device = NULL;

    int result;

    if (devices == NULL ||
        device_count == NULL ||
        max_devices == 0) {

        errno = EINVAL;
        return -1;
    }

    *device_count = 0;

    /*
     * Create device enumerator.
     */
    result = sd_device_enumerator_new(&enumerator);

    if (result < 0) {
        errno = -result;
        return -1;
    }

    /*
     * We are interested only in devices belonging
     * to the "tty" subsystem.
     */
    result = sd_device_enumerator_add_match_subsystem(
        enumerator,
        "tty",
        1
    );

    if (result < 0) {
        errno = -result;
        sd_device_enumerator_unref(enumerator);
        return -1;
    }

    /*
     * For a specific device type we can let sd-device
     * perform the filtering for us.
     *
     * For DEVICE_TYPE_ALL we enumerate all tty devices
     * and perform the ttyACM/ttyUSB filtering below.
     */
    if (device_type == DEVICE_TYPE_TTYACM) {

        result = sd_device_enumerator_add_match_sysname(
            enumerator,
            "ttyACM*"
        );

        if (result < 0) {
            errno = -result;
            sd_device_enumerator_unref(enumerator);
            return -1;
        }

    } else if (device_type == DEVICE_TYPE_TTYUSB) {

        result = sd_device_enumerator_add_match_sysname(
            enumerator,
            "ttyUSB*"
        );

        if (result < 0) {
            errno = -result;
            sd_device_enumerator_unref(enumerator);
            return -1;
        }
    }

    /*
     * Start enumeration.
     */
    device = sd_device_enumerator_get_device_first(
        enumerator
    );

    while (device != NULL) {

        const char *devname = NULL;

        /*
         * Get /dev/... device node.
         */
        result = sd_device_get_devname(
            device,
            &devname
        );

        if (result < 0) {
            errno = -result;
            sd_device_enumerator_unref(enumerator);
            return -1;
        }

        /*
         * DEVICE_TYPE_ALL requires additional filtering,
         * because the "tty" subsystem contains many devices
         * other than ttyACM* and ttyUSB*.
         */
        if (is_supported_serial_device(
                devname,
                device_type)) {

            if (*device_count >= max_devices) {
                errno = ENOSPC;
                sd_device_enumerator_unref(enumerator);
                return -1;
            }

            serial_device_info_t *info =
                &devices[*device_count];

            /*
             * Initialize the structure.
             */
            memset(
                info,
                0,
                sizeof(*info)
            );

            /*
             * Device path.
             */
            snprintf(
                info->port,
                sizeof(info->port),
                "%s",
                devname
            );

            /*
             * udev properties.
             */
            result = copy_device_property(
                device,
                "ID_VENDOR",
                info->vendor,
                sizeof(info->vendor)
            );

            if (result < 0) {
                errno = -result;
                sd_device_enumerator_unref(enumerator);
                return -1;
            }

            result = copy_device_property(
                device,
                "ID_MODEL",
                info->model,
                sizeof(info->model)
            );

            if (result < 0) {
                errno = -result;
                sd_device_enumerator_unref(enumerator);
                return -1;
            }

            result = copy_device_property(
                device,
                "ID_SERIAL_SHORT",
                info->serial,
                sizeof(info->serial)
            );

            if (result < 0) {
                errno = -result;
                sd_device_enumerator_unref(enumerator);
                return -1;
            }

            (*device_count)++;
        }

        /*
         * Get next device.
         */
        device = sd_device_enumerator_get_device_next(
            enumerator
        );
    }

    /*
     * Release enumerator.
     */
    sd_device_enumerator_unref(enumerator);

    return 0;
}

int print_serial_devices(device_type_t device_type)
{
    serial_device_info_t devices[MAX_SERIAL_DEVICES];
    size_t device_count = 0;

    if (find_serial_devices(
            device_type,
            devices,
            MAX_SERIAL_DEVICES,
            &device_count
        ) != 0) {
        return -1;
    }

    /*
     * Table header.
     */
    printf(
        "%-15s %-20s %-30s %-20s\n",
        "PORT",
        "PRODUCENT",
        "MODEL",
        "SERIAL"
    );

    /*
     * Separator line. Its columns have the widths of the header.
     */
    printf(
        "%-15s %-20s %-30s %-20s\n",
        "---------------",
        "--------------------",
        "------------------------------",
        "--------------------"
    );

    /*
     * One row per device.
     */
    for (size_t i = 0; i < device_count; i++) {
        printf(
            "%-15s %-20s %-30s %-20s\n",
            devices[i].port,
            devices[i].vendor,
            devices[i].model,
            devices[i].serial
        );
    }

    return 0;
}
