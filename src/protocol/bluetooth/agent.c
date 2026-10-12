/*
 * Pairing agent.
 *
 * The agent is a D-Bus object of this program with the interface
 * org.bluez.Agent1. BlueZ calls its methods during a pairing:
 *
 *     RequestPinCode(device) -> text
 *         Classic devices with a PIN, for example "0000".
 *     RequestPasskey(device) -> number
 *         The device shows a 6-digit number that has to be
 *         typed here.
 *     DisplayPinCode(device, text)
 *     DisplayPasskey(device, number, entered)
 *         This side shows a code that has to be typed on the
 *         device, for example on a Bluetooth keyboard.
 *     RequestConfirmation(device, number)
 *         Both sides show a number; the user confirms that
 *         they are the same.
 *     RequestAuthorization(device)
 *         Pairing without a code that still needs a yes or no.
 *     AuthorizeService(device, uuid)
 *         A device wants to use a service of this computer.
 *     Cancel()
 *         BlueZ gave up on the question it asked.
 *     Release()
 *         BlueZ no longer uses the agent.
 *
 * Each method is implemented by one handler below. A handler
 * forwards the question to the prompts supplied by the caller
 * and sends the answer back. When the user refuses, the handler
 * answers with the error org.bluez.Error.Rejected, which makes
 * BlueZ abort the pairing.
 *
 * The agent is announced to BlueZ with
 * org.bluez.AgentManager1.RegisterAgent(path, capability).
 * The capability "KeyboardDisplay" tells BlueZ that this side
 * can both show a code and accept one, so every pairing method
 * is possible.
 */

#include "protocol/bluetooth/agent.h"

#include "protocol/bluetooth/adapter.h"
#include "protocol/bluetooth/bluez.h"

#include <errno.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/*
 * D-Bus names used by the agent.
 */
#define AGENT_PATH "/io_master/agent"
#define AGENT_INTERFACE "org.bluez.Agent1"
#define AGENT_CAPABILITY "KeyboardDisplay"

#define AGENT_MANAGER_PATH "/org/bluez"
#define AGENT_MANAGER_INTERFACE "org.bluez.AgentManager1"

/*
 * Errors that an agent may answer with.
 */
#define AGENT_ERROR_REJECTED "org.bluez.Error.Rejected"

/*
 * Longest PIN of a classic device, plus the terminating '\0'.
 */
#define PIN_MAX 17

/*
 * A running agent.
 */
struct bt_agent {
    /*
     * Connection on which the agent object lives.
     */
    sd_bus *bus;

    /*
     * Handle of the exported object. Releasing it removes
     * the object from the connection.
     */
    sd_bus_slot *slot;

    /*
     * Functions that ask the user.
     */
    const bt_pairing_prompts_t *prompts;

    /*
     * true when BlueZ was told about the agent and has to be
     * told again when the agent stops.
     */
    bool registered;
};

/*
 * Reads the device argument of a method call, which is a D-Bus
 * object path, and converts it to a Bluetooth address.
 *
 * Returns 0 on success or a negative errno-style value.
 */
static int read_device_address(
    sd_bus_message *call,
    char *address,
    size_t size
)
{
    const char *path = NULL;

    int result = sd_bus_message_read(call, "o", &path);

    if (result < 0) {
        return result;
    }

    return bluez_address_from_path(path, address, size);
}

/*
 * Answers a method call with "the user refused".
 */
static int reject(sd_bus_error *error)
{
    return sd_bus_error_set(
        error,
        AGENT_ERROR_REJECTED,
        "Rejected by the user"
    );
}

/*
 * Agent1.RequestPinCode(device) -> text
 */
static int handle_request_pin_code(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    const bt_agent_t *agent = userdata;
    char address[BT_ADDRESS_MAX];
    char pin[PIN_MAX];

    int result = read_device_address(call, address, sizeof(address));

    if (result < 0) {
        return result;
    }

    if (agent->prompts->request_pin == NULL ||
        agent->prompts->request_pin(address, pin, sizeof(pin)) != 0) {
        return reject(error);
    }

    return sd_bus_reply_method_return(call, "s", pin);
}

/*
 * Agent1.RequestPasskey(device) -> number
 */
static int handle_request_passkey(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    const bt_agent_t *agent = userdata;
    char address[BT_ADDRESS_MAX];
    uint32_t passkey = 0;

    int result = read_device_address(call, address, sizeof(address));

    if (result < 0) {
        return result;
    }

    if (agent->prompts->request_passkey == NULL ||
        agent->prompts->request_passkey(address, &passkey) != 0) {
        return reject(error);
    }

    return sd_bus_reply_method_return(call, "u", passkey);
}

/*
 * Agent1.DisplayPinCode(device, text)
 */
static int handle_display_pin_code(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    (void)error; // Unused parameter

    const bt_agent_t *agent = userdata;
    char address[BT_ADDRESS_MAX];
    const char *pin = NULL;

    int result = read_device_address(call, address, sizeof(address));

    if (result >= 0) {
        result = sd_bus_message_read(call, "s", &pin);
    }

    if (result < 0) {
        return result;
    }

    if (agent->prompts->display_pin != NULL) {
        agent->prompts->display_pin(address, pin);
    }

    /*
     * An empty reply: the code was shown.
     */
    return sd_bus_reply_method_return(call, "");
}

/*
 * Agent1.DisplayPasskey(device, number, entered)
 */
static int handle_display_passkey(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    (void)error; // Unused parameter

    const bt_agent_t *agent = userdata;
    char address[BT_ADDRESS_MAX];
    uint32_t passkey = 0;
    uint16_t entered = 0;

    int result = read_device_address(call, address, sizeof(address));

    if (result >= 0) {
        result = sd_bus_message_read(call, "uq", &passkey, &entered);
    }

    if (result < 0) {
        return result;
    }

    /*
     * BlueZ repeats this call after every digit typed on the
     * device, with entered counting the digits. Showing the
     * code once, before the first digit, is enough.
     */
    if (entered == 0 && agent->prompts->display_passkey != NULL) {
        agent->prompts->display_passkey(address, passkey);
    }

    return sd_bus_reply_method_return(call, "");
}

/*
 * Agent1.RequestConfirmation(device, number)
 */
static int handle_request_confirmation(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    const bt_agent_t *agent = userdata;
    char address[BT_ADDRESS_MAX];
    uint32_t passkey = 0;

    int result = read_device_address(call, address, sizeof(address));

    if (result >= 0) {
        result = sd_bus_message_read(call, "u", &passkey);
    }

    if (result < 0) {
        return result;
    }

    if (agent->prompts->confirm_passkey == NULL ||
        agent->prompts->confirm_passkey(address, passkey) != 0) {
        return reject(error);
    }

    /*
     * An empty reply: the numbers match.
     */
    return sd_bus_reply_method_return(call, "");
}

/*
 * Agent1.RequestAuthorization(device)
 */
static int handle_request_authorization(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    const bt_agent_t *agent = userdata;
    char address[BT_ADDRESS_MAX];

    int result = read_device_address(call, address, sizeof(address));

    if (result < 0) {
        return result;
    }

    if (agent->prompts->authorize == NULL ||
        agent->prompts->authorize(address) != 0) {
        return reject(error);
    }

    return sd_bus_reply_method_return(call, "");
}

/*
 * Agent1.AuthorizeService(device, uuid)
 *
 * The agent exists only while this program pairs with a device
 * that the user has chosen, so the device is allowed to use
 * the services it asks for.
 */
static int handle_authorize_service(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    (void)userdata; // Unused parameter
    (void)error;    // Unused parameter

    return sd_bus_reply_method_return(call, "");
}

/*
 * Agent1.Cancel() and Agent1.Release()
 *
 * Both only inform the agent; there is nothing to undo, because
 * a question is answered before the next call is handled.
 */
static int handle_notification(
    sd_bus_message *call,
    void *userdata,
    sd_bus_error *error
)
{
    (void)userdata; // Unused parameter
    (void)error;    // Unused parameter

    return sd_bus_reply_method_return(call, "");
}

/*
 * Description of the agent object for sd-bus: the methods of
 * org.bluez.Agent1, the types of their arguments and results
 * ("o" object path, "s" text, "u" 32-bit number, "q" 16-bit
 * number) and the handlers that implement them.
 *
 * SD_BUS_VTABLE_UNPRIVILEGED lets BlueZ call the methods without
 * an additional permission check by sd-bus.
 */
static const sd_bus_vtable agent_vtable[] = {
    SD_BUS_VTABLE_START(0),
    SD_BUS_METHOD(
        "RequestPinCode", "o", "s",
        handle_request_pin_code,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "RequestPasskey", "o", "u",
        handle_request_passkey,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "DisplayPinCode", "os", "",
        handle_display_pin_code,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "DisplayPasskey", "ouq", "",
        handle_display_passkey,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "RequestConfirmation", "ou", "",
        handle_request_confirmation,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "RequestAuthorization", "o", "",
        handle_request_authorization,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "AuthorizeService", "os", "",
        handle_authorize_service,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "Cancel", "", "",
        handle_notification,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_METHOD(
        "Release", "", "",
        handle_notification,
        SD_BUS_VTABLE_UNPRIVILEGED
    ),
    SD_BUS_VTABLE_END
};

int bt_agent_export(
    sd_bus *bus,
    const bt_pairing_prompts_t *prompts,
    bt_agent_t **agent
)
{
    if (bus == NULL || prompts == NULL || agent == NULL) {
        return -EINVAL;
    }

    *agent = NULL;

    bt_agent_t *created = calloc(1, sizeof(*created));

    if (created == NULL) {
        return -ENOMEM;
    }

    created->bus = bus;
    created->prompts = prompts;

    /*
     * Attach the object to the connection. From now on calls
     * to AGENT_PATH are passed to the handlers, with the agent
     * as their userdata.
     */
    int result = sd_bus_add_object_vtable(
        bus,
        &created->slot,
        AGENT_PATH,
        AGENT_INTERFACE,
        agent_vtable,
        created
    );

    if (result < 0) {
        free(created);
        return result;
    }

    *agent = created;

    return 0;
}

int bt_agent_start(
    sd_bus *bus,
    const bt_pairing_prompts_t *prompts,
    bt_agent_t **agent
)
{
    int result = bt_agent_export(bus, prompts, agent);

    if (result < 0) {
        return result;
    }

    /*
     * Tell BlueZ where the agent is and what it can do.
     */
    result = sd_bus_call_method(
        bus,
        BLUEZ_SERVICE,
        AGENT_MANAGER_PATH,
        AGENT_MANAGER_INTERFACE,
        "RegisterAgent",
        NULL,
        NULL,
        "os",
        AGENT_PATH,
        AGENT_CAPABILITY
    );

    if (result < 0) {
        bt_agent_stop(*agent);
        *agent = NULL;
        return result;
    }

    (*agent)->registered = true;

    return 0;
}

void bt_agent_stop(bt_agent_t *agent)
{
    if (agent == NULL) {
        return;
    }

    /*
     * A failure is ignored: BlueZ drops the agent anyway when
     * the connection is closed.
     */
    if (agent->registered) {
        sd_bus_call_method(
            agent->bus,
            BLUEZ_SERVICE,
            AGENT_MANAGER_PATH,
            AGENT_MANAGER_INTERFACE,
            "UnregisterAgent",
            NULL,
            NULL,
            "o",
            AGENT_PATH
        );
    }

    sd_bus_slot_unref(agent->slot);
    free(agent);
}
