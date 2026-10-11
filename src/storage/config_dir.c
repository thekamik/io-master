#include "storage/config_dir.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

int config_build_path(
    char *buffer,
    size_t size,
    const char *relative_path
)
{
    char executable[PATH_MAX];

    if (buffer == NULL || size == 0) {
        errno = EINVAL;
        return -1;
    }

    /*
     * Resolve the location of the executable instead of using
     * the current working directory.
     */
    ssize_t length = readlink(
        "/proc/self/exe",
        executable,
        sizeof(executable) - 1
    );

    if (length < 0) {
        return -1;
    }

    executable[length] = '\0';

    /*
     * Cut off the executable name and keep its directory.
     */
    char *separator = strrchr(executable, '/');

    if (separator == NULL) {
        errno = ENOENT;
        return -1;
    }

    *separator = '\0';

    int written;

    if (relative_path == NULL) {
        written = snprintf(
            buffer,
            size,
            "%s/%s",
            executable,
            CONFIG_DIRECTORY_NAME
        );
    } else {
        written = snprintf(
            buffer,
            size,
            "%s/%s/%s",
            executable,
            CONFIG_DIRECTORY_NAME,
            relative_path
        );
    }

    if (written < 0 || (size_t)written >= size) {
        errno = ENAMETOOLONG;
        return -1;
    }

    return 0;
}

int config_create_directory(const char *subdirectory)
{
    char path[PATH_MAX];

    if (config_build_path(path, sizeof(path), NULL) != 0) {
        return -1;
    }

    if (mkdir(path, 0755) != 0 && errno != EEXIST) {
        return -1;
    }

    if (subdirectory == NULL) {
        return 0;
    }

    if (config_build_path(path, sizeof(path), subdirectory) != 0) {
        return -1;
    }

    if (mkdir(path, 0755) != 0 && errno != EEXIST) {
        return -1;
    }

    return 0;
}
