/*
 * Pairing agent.
 *
 * When a device has to be paired with a code, BlueZ does not ask
 * the user itself. It asks an "agent": an object that a program
 * registers with BlueZ and that answers questions such as "what
 * is the PIN?" or "does the device show 123456?".
 *
 * This file provides such an agent. The agent does not talk to
 * the user on its own; it forwards every question to the
 * functions of a bt_pairing_prompts_t (see connection.h), which
 * are supplied by the shell.
 *
 * BlueZ asks the agent only when the pairing needs it. A device
 * that pairs without a code never causes a question.
 *
 * This is an internal header of the Bluetooth protocol. Its
 * functions return 0 on success and a negative errno-style value
 * on error, like sd-bus itself.
 */

#ifndef BLUETOOTH_AGENT_H
#define BLUETOOTH_AGENT_H

#include <systemd/sd-bus.h>

#include "protocol/bluetooth/connection.h"

/*
 * A running agent. Created by bt_agent_start() and released
 * by bt_agent_stop().
 */
typedef struct bt_agent bt_agent_t;

/*
 * Makes the agent object available on the given bus connection,
 * without telling BlueZ about it.
 *
 * bt_agent_start() does this as its first step. The function is
 * public so that the agent can be tested without BlueZ, by
 * calling its methods from another program.
 *
 * prompts has to stay valid until bt_agent_stop().
 */
int bt_agent_export(
    sd_bus *bus,
    const bt_pairing_prompts_t *prompts,
    bt_agent_t **agent
);

/*
 * Creates the agent on the given bus connection and registers
 * it with BlueZ.
 *
 * BlueZ uses the agent of the program that asks for a pairing,
 * so the pairing has to be requested on the same connection.
 * The questions arrive while that connection is being processed
 * (sd_bus_process()).
 *
 * prompts has to stay valid until bt_agent_stop().
 */
int bt_agent_start(
    sd_bus *bus,
    const bt_pairing_prompts_t *prompts,
    bt_agent_t **agent
);

/*
 * Unregisters the agent from BlueZ and releases it.
 * NULL is accepted.
 */
void bt_agent_stop(bt_agent_t *agent);

#endif
