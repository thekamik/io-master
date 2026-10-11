/*
 * Registry of supported protocols.
 *
 * The rest of the program never refers to a protocol directly.
 * It asks this registry for a protocol_t by name and calls
 * the functions stored in it.
 */

#include "protocol/protocol.h"

#include "protocol/uart/uart.h"

#include <strings.h>

/*
 * Table of supported protocols.
 *
 * A new protocol is registered by adding one entry here.
 */
static const protocol_t *const protocols[] = {
    &uart_protocol,
};

/*
 * Number of registered protocols.
 */
size_t protocol_count(void)
{
    return sizeof(protocols) / sizeof(protocols[0]);
}

/*
 * Returns the protocol at the given position of the table, or NULL
 * when the index is out of range. Used to list all protocols.
 */
const protocol_t *protocol_get(size_t index)
{
    if (index >= protocol_count()) {
        return NULL;
    }

    return protocols[index];
}

/*
 * Looks a protocol up by name. Letter case is ignored, so "UART"
 * and "uart" name the same protocol.
 */
const protocol_t *protocol_find(const char *name)
{
    if (name == NULL) {
        return NULL;
    }

    for (size_t i = 0; i < protocol_count(); i++) {

        if (strcasecmp(protocols[i]->name, name) == 0) {
            return protocols[i];
        }
    }

    return NULL;
}
