/*
 * Reading and writing JSON files with the Jansson library.
 */

#include "storage/json_file.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>

#define JSON_TEMP_SUFFIX ".tmp"

int json_file_read(
    const char *path,
    json_t **root
)
{
    if (path == NULL || root == NULL) {
        errno = EINVAL;
        return -1;
    }

    *root = NULL;

    FILE *file = fopen(path, "r");

    if (file == NULL) {
        return -1;
    }

    json_error_t error;

    /*
     * JSON_REJECT_DUPLICATES:
     *   a key repeated inside one object is treated as an error
     *   instead of silently keeping the last value.
     */
    json_t *document = json_loadf(
        file,
        JSON_REJECT_DUPLICATES,
        &error
    );

    fclose(file);

    if (document == NULL) {
        fprintf(
            stderr,
            "%s:%d:%d: %s\n",
            path,
            error.line,
            error.column,
            error.text
        );
        errno = EINVAL;
        return -1;
    }

    *root = document;

    return 0;
}

int json_file_write(
    const char *path,
    const json_t *root
)
{
    char temp_path[PATH_MAX];

    if (path == NULL || root == NULL) {
        errno = EINVAL;
        return -1;
    }

    int length = snprintf(
        temp_path,
        sizeof(temp_path),
        "%s%s",
        path,
        JSON_TEMP_SUFFIX
    );

    if (length < 0 || (size_t)length >= sizeof(temp_path)) {
        errno = ENAMETOOLONG;
        return -1;
    }

    FILE *file = fopen(temp_path, "w");

    if (file == NULL) {
        return -1;
    }

    /*
     * JSON_INDENT(4):
     *   pretty-print the document so that it stays easy to
     *   edit by hand.
     *
     * A trailing newline is added, because json_dumpf()
     * does not write one.
     */
    if (json_dumpf(root, file, JSON_INDENT(4)) != 0 ||
        fputc('\n', file) == EOF ||
        fflush(file) != 0) {

        /*
         * json_dumpf() does not set errno when the document
         * itself cannot be encoded.
         */
        int saved_errno = errno != 0 ? errno : EIO;

        fclose(file);
        remove(temp_path);
        errno = saved_errno;

        return -1;
    }

    if (fclose(file) != 0) {
        int saved_errno = errno;

        remove(temp_path);
        errno = saved_errno;

        return -1;
    }

    /*
     * Replace the target file in a single step.
     */
    if (rename(temp_path, path) != 0) {
        int saved_errno = errno;

        remove(temp_path);
        errno = saved_errno;

        return -1;
    }

    return 0;
}
