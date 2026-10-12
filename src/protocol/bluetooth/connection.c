/*
 * Operations on one Bluetooth device: connect, disconnect,
 * pair and unpair.
 *
 * BlueZ offers them as D-Bus methods:
 *
 *     operation    object   interface            method
 *     ---------    ------   ---------            ------
 *     connect      device   org.bluez.Device1    Connect
 *     disconnect   device   org.bluez.Device1    Disconnect
 *     pair         device   org.bluez.Device1    Pair
 *     unpair       card     org.bluez.Adapter1   RemoveDevice
 *
 * Why the calls are not simple blocking calls
 * -------------------------------------------
 * Connecting and pairing can take a long time, and the user must
 * be able to abort them with Ctrl+C. A blocking sd-bus call
 * cannot be interrupted, so every method is called
 * asynchronously: the call is sent, and run_call() then waits for
 * the reply in its own loop, looking at the cancel flag several
 * times a second.
 *
 * Pairing with a code
 * -------------------
 * For a pairing, an agent (agent.c) is registered on the same
 * connection before Pair is called. While run_call() waits for
 * the reply to Pair, BlueZ may call the agent to ask for a PIN
 * or a confirmation. Those calls are handled inside
 * sd_bus_process(), in the same loop.
 */

#include "protocol/bluetooth/connection.h"

#include "protocol/bluetooth/agent.h"
#include "protocol/bluetooth/bluez.h"

#include <errno.h>
#include <poll.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/*
 * How long BlueZ may take to answer, in microseconds.
 */
#define USEC_PER_SECOND 1000000ULL
#define CONNECT_TIMEOUT_USEC (30 * USEC_PER_SECOND)
#define PAIR_TIMEOUT_USEC (120 * USEC_PER_SECOND)
#define SHORT_TIMEOUT_USEC (10 * USEC_PER_SECOND)

/*
 * Longest wait between two looks at the cancel flag,
 * in milliseconds.
 */
#define CANCEL_CHECK_INTERVAL_MS 200

/*
 * Outcome of one asynchronous call, filled by on_reply().
 */
typedef struct {
    /*
     * true once BlueZ has answered or the call has timed out.
     */
    bool done;

    /*
     * true when the answer was an error. error_name is then the
     * D-Bus name of the error, such as
     * "org.bluez.Error.AlreadyConnected", and error_message
     * the text that came with it.
     */
    bool failed;
    char error_name[96];
    char error_message[BT_ERROR_TEXT_MAX];
} call_state_t;

/*
 * Called by sd-bus when the reply to an asynchronous call
 * arrives, or when the call times out; a timeout is delivered
 * as an error reply as well.
 */
static int on_reply(
    sd_bus_message *reply,
    void *userdata,
    sd_bus_error *ret_error
)
{
    (void)ret_error; // Unused parameter

    call_state_t *state = userdata;

    state->done = true;

    /*
     * NULL for a normal reply, the error otherwise.
     */
    const sd_bus_error *error = sd_bus_message_get_error(reply);

    if (error != NULL) {
        state->failed = true;

        snprintf(
            state->error_name,
            sizeof(state->error_name),
            "%s",
            error->name != NULL ? error->name : ""
        );

        snprintf(
            state->error_message,
            sizeof(state->error_message),
            "%s",
            error->message != NULL ? error->message : ""
        );
    }

    return 0;
}

/*
 * Translates the D-Bus error of a failed call to one of the
 * errno values documented in connection.h.
 */
static int error_name_to_errno(const char *name)
{
    /*
     * Table of the errors that the callers handle specially.
     */
    static const struct {
        const char *name;
        int error_number;
    } known_errors[] = {
        { "org.freedesktop.DBus.Error.UnknownObject", ENOENT },
        { "org.freedesktop.DBus.Error.UnknownMethod", ENOENT },
        { "org.bluez.Error.DoesNotExist", ENOENT },
        { "org.bluez.Error.NotReady", ENETDOWN },
        { "org.bluez.Error.AlreadyConnected", EISCONN },
        { "org.bluez.Error.NotConnected", ENOTCONN },
        { "org.bluez.Error.AlreadyExists", EEXIST },
        { "org.bluez.Error.AuthenticationFailed", EACCES },
        { "org.bluez.Error.AuthenticationRejected", EACCES },
        { "org.bluez.Error.AuthenticationCanceled", EACCES },
        { "org.bluez.Error.AuthenticationTimeout", ETIMEDOUT },
        { "org.freedesktop.DBus.Error.NoReply", ETIMEDOUT },
        { "org.freedesktop.DBus.Error.Timeout", ETIMEDOUT },
        { "org.freedesktop.DBus.Error.AccessDenied", EACCES },
    };

    for (size_t i = 0;
         i < sizeof(known_errors) / sizeof(known_errors[0]);
         i++) {

        if (strcmp(name, known_errors[i].name) == 0) {
            return known_errors[i].error_number;
        }
    }

    /*
     * Everything else, most often org.bluez.Error.Failed,
     * is explained only by its message.
     */
    return EIO;
}

/*
 * Waits until the connection to the bus has something to
 * process, but not longer than CANCEL_CHECK_INTERVAL_MS.
 *
 * Returns 0 on success or a negative errno-style value.
 */
static int wait_for_bus(sd_bus *bus)
{
    struct pollfd descriptor;
    uint64_t timeout_usec = 0;

    int fd = sd_bus_get_fd(bus);

    if (fd < 0) {
        return fd;
    }

    int events = sd_bus_get_events(bus);

    if (events < 0) {
        return events;
    }

    int result = sd_bus_get_timeout(bus, &timeout_usec);

    if (result < 0) {
        return result;
    }

    descriptor.fd = fd;
    descriptor.events = (short)events;
    descriptor.revents = 0;

    /*
     * The wait is deliberately short, so that the caller can
     * look at the cancel flag again. sd-bus keeps track of its
     * own timeouts when sd_bus_process() is called afterwards.
     */
    if (poll(&descriptor, 1, CANCEL_CHECK_INTERVAL_MS) < 0 &&
        errno != EINTR) {
        return -errno;
    }

    return 0;
}

/*
 * Sends a method call to BlueZ and waits for the answer.
 *
 * call is the prepared method call. undo_method, when not NULL,
 * names a method of the same object and interface that is called
 * when the operation is cancelled, to make BlueZ stop what it
 * has started.
 *
 * Returns 0 on success and -1 on error, with errno and
 * error_text set as documented in connection.h.
 */
static int run_call(
    sd_bus *bus,
    sd_bus_message *call,
    uint64_t timeout_usec,
    const char *undo_method,
    const volatile sig_atomic_t *cancel,
    char *error_text
)
{
    sd_bus_slot *slot = NULL;
    call_state_t state;

    memset(&state, 0, sizeof(state));

    /*
     * Send the call. on_reply() will be called with the answer.
     */
    int result = sd_bus_call_async(
        bus,
        &slot,
        call,
        on_reply,
        &state,
        timeout_usec
    );

    bool cancelled = false;

    while (result >= 0 && !state.done) {

        if (cancel != NULL && *cancel != 0) {
            cancelled = true;
            break;
        }

        /*
         * Let sd-bus handle what has arrived. A positive value
         * means that there may be more to do right away.
         */
        result = sd_bus_process(bus, NULL);

        if (result == 0) {
            result = wait_for_bus(bus);
        }
    }

    /*
     * Releasing the slot makes sd-bus forget the call, so
     * on_reply() cannot run after state is gone.
     */
    sd_bus_slot_unref(slot);

    if (cancelled) {
        /*
         * Tell BlueZ to stop. Whether that works does not
         * change what is reported to the caller.
         */
        if (undo_method != NULL) {
            sd_bus_call_method(
                bus,
                sd_bus_message_get_destination(call),
                sd_bus_message_get_path(call),
                sd_bus_message_get_interface(call),
                undo_method,
                NULL,
                NULL,
                ""
            );
        }

        if (error_text != NULL) {
            snprintf(error_text, BT_ERROR_TEXT_MAX, "cancelled");
        }

        errno = ECANCELED;
        return -1;
    }

    /*
     * A problem with the bus itself.
     */
    if (result < 0) {

        if (error_text != NULL) {
            snprintf(
                error_text,
                BT_ERROR_TEXT_MAX,
                "%s",
                strerror(-result)
            );
        }

        errno = -result;
        return -1;
    }

    /*
     * BlueZ answered with an error.
     */
    if (state.failed) {

        if (error_text != NULL) {
            snprintf(
                error_text,
                BT_ERROR_TEXT_MAX,
                "%s",
                state.error_message[0] != '\0'
                    ? state.error_message
                    : state.error_name
            );
        }

        errno = error_name_to_errno(state.error_name);
        return -1;
    }

    return 0;
}

/*
 * Calls a method of org.bluez.Device1 that takes no arguments:
 * Connect, Disconnect or Pair.
 *
 * When prompts is not NULL, a pairing agent that uses them is
 * registered for the duration of the call.
 */
static int call_device_method(
    const char *adapter,
    const char *address,
    const char *method,
    uint64_t timeout_usec,
    const char *undo_method,
    const bt_pairing_prompts_t *prompts,
    const volatile sig_atomic_t *cancel,
    char *error_text
)
{
    char path[BLUEZ_PATH_MAX];
    sd_bus *bus = NULL;
    sd_bus_message *call = NULL;
    bt_agent_t *agent = NULL;

    if (error_text != NULL) {
        error_text[0] = '\0';
    }

    if (bluez_device_path(adapter, address, path, sizeof(path)) < 0) {

        if (error_text != NULL) {
            snprintf(
                error_text,
                BT_ERROR_TEXT_MAX,
                "invalid card name or address"
            );
        }

        errno = EINVAL;
        return -1;
    }

    int result = sd_bus_open_system(&bus);

    /*
     * The agent has to be on the connection that sends Pair:
     * BlueZ asks the agent of the program that wants the pairing.
     */
    if (result >= 0 && prompts != NULL) {
        result = bt_agent_start(bus, prompts, &agent);
    }

    if (result >= 0) {
        result = sd_bus_message_new_method_call(
            bus,
            &call,
            BLUEZ_SERVICE,
            path,
            BLUEZ_DEVICE_INTERFACE,
            method
        );
    }

    /*
     * The bus could not be opened, the agent not be registered
     * or the call not be built.
     */
    if (result < 0) {

        if (error_text != NULL) {
            snprintf(
                error_text,
                BT_ERROR_TEXT_MAX,
                "%s",
                strerror(-result)
            );
        }

        bt_agent_stop(agent);
        sd_bus_message_unref(call);
        sd_bus_unref(bus);

        errno = -result;
        return -1;
    }

    int status = run_call(
        bus,
        call,
        timeout_usec,
        undo_method,
        cancel,
        error_text
    );

    /*
     * Keep the errno of the operation across the cleanup.
     */
    int saved_errno = errno;

    bt_agent_stop(agent);
    sd_bus_message_unref(call);
    sd_bus_unref(bus);

    errno = saved_errno;

    return status;
}

int bt_device_connect(
    const char *adapter,
    const char *address,
    const volatile sig_atomic_t *cancel,
    char *error_text
)
{
    /*
     * Disconnect also aborts a connection attempt that is
     * still in progress.
     */
    return call_device_method(
        adapter,
        address,
        "Connect",
        CONNECT_TIMEOUT_USEC,
        "Disconnect",
        NULL,
        cancel,
        error_text
    );
}

int bt_device_disconnect(
    const char *adapter,
    const char *address,
    const volatile sig_atomic_t *cancel,
    char *error_text
)
{
    return call_device_method(
        adapter,
        address,
        "Disconnect",
        SHORT_TIMEOUT_USEC,
        NULL,
        NULL,
        cancel,
        error_text
    );
}

int bt_device_pair(
    const char *adapter,
    const char *address,
    const bt_pairing_prompts_t *prompts,
    const volatile sig_atomic_t *cancel,
    char *error_text
)
{
    return call_device_method(
        adapter,
        address,
        "Pair",
        PAIR_TIMEOUT_USEC,
        "CancelPairing",
        prompts,
        cancel,
        error_text
    );
}

int bt_device_unpair(
    const char *adapter,
    const char *address,
    const volatile sig_atomic_t *cancel,
    char *error_text
)
{
    char adapter_path[BLUEZ_PATH_MAX];
    char device_path[BLUEZ_PATH_MAX];
    sd_bus *bus = NULL;
    sd_bus_message *call = NULL;

    if (error_text != NULL) {
        error_text[0] = '\0';
    }

    if (bluez_adapter_path(
            adapter,
            adapter_path,
            sizeof(adapter_path)) < 0 ||
        bluez_device_path(
            adapter,
            address,
            device_path,
            sizeof(device_path)) < 0) {

        if (error_text != NULL) {
            snprintf(
                error_text,
                BT_ERROR_TEXT_MAX,
                "invalid card name or address"
            );
        }

        errno = EINVAL;
        return -1;
    }

    /*
     * Unlike the other operations, this one is a method of the
     * card: Adapter1.RemoveDevice(device path). It removes the
     * device object together with its pairing.
     */
    int result = sd_bus_open_system(&bus);

    if (result >= 0) {
        result = sd_bus_message_new_method_call(
            bus,
            &call,
            BLUEZ_SERVICE,
            adapter_path,
            BLUEZ_ADAPTER_INTERFACE,
            "RemoveDevice"
        );
    }

    if (result >= 0) {
        result = sd_bus_message_append(call, "o", device_path);
    }

    if (result < 0) {

        if (error_text != NULL) {
            snprintf(
                error_text,
                BT_ERROR_TEXT_MAX,
                "%s",
                strerror(-result)
            );
        }

        sd_bus_message_unref(call);
        sd_bus_unref(bus);

        errno = -result;
        return -1;
    }

    int status = run_call(
        bus,
        call,
        SHORT_TIMEOUT_USEC,
        NULL,
        cancel,
        error_text
    );

    int saved_errno = errno;

    sd_bus_message_unref(call);
    sd_bus_unref(bus);

    errno = saved_errno;

    return status;
}
