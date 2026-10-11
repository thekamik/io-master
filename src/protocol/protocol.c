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

size_t protocol_count(void)
{
    return sizeof(protocols) / sizeof(protocols[0]);
}

const protocol_t *protocol_get(size_t index)
{
    if (index >= protocol_count()) {
        return NULL;
    }

    return protocols[index];
}

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
