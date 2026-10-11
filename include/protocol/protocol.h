#ifndef PROTOCOL_H
#define PROTOCOL_H

#include <stddef.h>

#include "project/project.h"

/*
 * Description of a protocol supported by the program.
 *
 * Adding a protocol:
 *   1. implement it in src/protocol/<name>/ and define
 *      a protocol_t for it,
 *   2. add that protocol_t to the table in src/protocol/protocol.c.
 *
 * The functions report their own errors on stderr.
 */
typedef struct {
    /*
     * Name used on the command line and stored in the project.
     */
    const char *name;

    const char *description;

    /*
     * Prepares a newly added project by creating its settings
     * file (im_config/projects/<id>.json) with default values.
     *
     * A project without that file is treated as damaged.
     *
     * Returns 0 on success and -1 on error.
     */
    int (*init)(const project_info_t *project);

    /*
     * Prints the help that is specific to this protocol.
     */
    void (*print_help)(
        const char *program_name,
        const project_info_t *project
    );

    /*
     * Starts the project.
     *
     * Returns 0 on success and -1 on error.
     */
    int (*run)(const project_info_t *project);
} protocol_t;

/*
 * Finds a protocol by name. Letter case is ignored.
 *
 * Returns NULL when the protocol is not supported.
 */
const protocol_t *protocol_find(const char *name);

/*
 * Access to the list of supported protocols.
 */
size_t protocol_count(void);
const protocol_t *protocol_get(size_t index);

#endif
