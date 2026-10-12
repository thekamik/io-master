/*
 * AI "protocol": a shell for talking to an AI model.
 *
 * Unlike uart and bluetooth, this is not a communication
 * protocol of a device. It is a working tool for trying out and
 * debugging the AI integration (src/ai/), which the device
 * protocols are meant to use later for analysing their data.
 */

#ifndef AI_PROTOCOL_H
#define AI_PROTOCOL_H

#include "protocol/protocol.h"

/*
 * AI protocol, registered in src/protocol/protocol.c.
 *
 * The settings of an AI project (provider, model, name of the
 * API key and so on) are stored in its settings file. The API
 * key itself is kept in the keyring of the system.
 */
extern const protocol_t ai_protocol;

#endif
