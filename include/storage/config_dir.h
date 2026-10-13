/*
 * Location of the im_config directory, where all configuration
 * files of the program are stored.
 */

#ifndef CONFIG_DIR_H
#define CONFIG_DIR_H

#include <stddef.h>
#include <sys/types.h>

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

/*
 * Creates a directory several levels deep inside im_config, for
 * example "ai/chats/3": im_config itself and then every level of
 * relative_path that does not exist yet.
 *
 * mode is the permission of the directories that are created
 * below im_config, for example 0700 for directories that hold
 * private data. Existing directories are left as they are.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int config_create_directories(const char *relative_path, mode_t mode);

#endif
