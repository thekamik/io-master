#ifndef CONFIG_DIR_H
#define CONFIG_DIR_H

#include <stddef.h>

#define CONFIG_DIRECTORY_NAME "im_config"

/*
 * Builds a path inside the im_config directory.
 *
 * The directory lives next to the executable, so the result does
 * not depend on the current working directory.
 *
 * relative_path may be NULL. In that case the path of the
 * im_config directory itself is returned.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int config_build_path(
    char *buffer,
    size_t size,
    const char *relative_path
);

/*
 * Creates the im_config directory and, when subdirectory is not
 * NULL, a single subdirectory inside it.
 *
 * Already existing directories are not an error.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int config_create_directory(const char *subdirectory);

#endif
