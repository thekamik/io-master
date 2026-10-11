/*
 * Reading and writing JSON files.
 *
 * Wraps the Jansson library. The callers work with json_t
 * documents directly, using the Jansson API.
 */

#ifndef JSON_FILE_H
#define JSON_FILE_H

#include <jansson.h>

/*
 * Reads and parses a JSON file.
 *
 * On success *root holds a new reference to the parsed document.
 * The caller releases it with json_decref().
 *
 * A file that is not valid JSON is reported on stderr together
 * with the line and column of the problem.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when the file does not
 *        exist, EINVAL when it is not valid JSON)
 */
int json_file_read(
    const char *path,
    json_t **root
);

/*
 * Writes a JSON document to a file.
 *
 * The document is written to a temporary file first and then
 * renamed, so an existing file is never left half-written.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int json_file_write(
    const char *path,
    const json_t *root
);

#endif
