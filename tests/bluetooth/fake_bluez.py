#!/usr/bin/env python3
"""
Stand-in for BlueZ, used by the Bluetooth shell tests.

The real BlueZ needs a Bluetooth card and real devices nearby, so
it cannot be used in an automatic test. This script pretends to be
BlueZ instead: it takes the D-Bus name "org.bluez" on a private bus
and offers the same objects and methods that io-master uses:

    /                    ObjectManager.GetManagedObjects
    /org/bluez           AgentManager1.RegisterAgent, UnregisterAgent
    /org/bluez/hci0      Adapter1.StartDiscovery, StopDiscovery,
                         RemoveDevice
    /org/bluez/hci0/dev_...
                         Device1.Connect, Disconnect, Pair,
                         CancelPairing

It publishes one card (hci0) and a fixed set of devices. Each device
has a "kind" that decides how it behaves when it is paired or
connected; see DEVICES below.

Every call that changes something is written to a log file, one
line per call. The tests read that file to check what io-master
asked BlueZ to do.

Usage:
    fake_bluez.py LOG_FILE

The script prints "ready" once it can be used and then runs until
it is terminated. It must be started on a private bus (see
test_bluetooth_shell.py), never on the real system bus.

Requires the python3-dbus and python3-gi packages.
"""

import sys

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

# Values that the fake devices expect during a pairing. The tests
# type these (or deliberately wrong ones).
CORRECT_PIN = "1234"
CORRECT_PASSKEY = 123456

# Number that a "confirm" device asks the user to compare.
CONFIRMATION_NUMBER = 54321

# Code that a "keyboard" device wants typed on its own keys.
KEYBOARD_PASSKEY = 111222

# How long a "slow" device takes to connect, in milliseconds. Long
# enough for a test to cancel the attempt.
SLOW_CONNECT_MS = 20000

# The devices offered by the fake BlueZ: (name, kind). Their
# addresses are AA:00:00:00:00:01, ...:02 and so on, in this order.
#
#   just      pairs without any question
#   pin       asks the agent for a PIN and accepts CORRECT_PIN
#   passkey   asks the agent for a passkey, accepts CORRECT_PASSKEY
#   confirm   asks the agent to confirm CONFIRMATION_NUMBER
#   slow      takes SLOW_CONNECT_MS to connect
#   keyboard  asks the agent to display KEYBOARD_PASSKEY
#   slowpair  never answers a pairing request
DEVICES = [
    ("JustWorks", "just"),
    ("PinDevice", "pin"),
    ("PasskeyDevice", "passkey"),
    ("ConfirmDevice", "confirm"),
    ("SlowDevice", "slow"),
    ("Keyboard", "keyboard"),
    ("SlowPair", "slowpair"),
]

# Timeout for calls made to the agent of io-master, in seconds.
AGENT_TIMEOUT = 100


class BluezError(dbus.DBusException):
    """A D-Bus error with a BlueZ name, such as org.bluez.Error.Failed."""

    def __init__(self, name, message):
        super().__init__(message)
        self._dbus_error_name = "org.bluez.Error." + name


class FakeBluez:
    """State shared by all objects of the fake BlueZ."""

    def __init__(self, bus, log_path):
        self.bus = bus

        # Line-buffered, so the tests can read the log while the
        # fake is still running.
        self.log_file = open(log_path, "w", buffering=1)

        # The agent registered by io-master: bus name of its
        # connection and object path. None when there is no agent.
        self.agent_sender = None
        self.agent_path = None

        # Device objects by their D-Bus path.
        self.devices = {}

    def log(self, text):
        self.log_file.write(text + "\n")

    def agent(self):
        """Returns a proxy for calling the registered agent."""
        proxy = self.bus.get_object(self.agent_sender, self.agent_path)
        return dbus.Interface(proxy, "org.bluez.Agent1")


class Device(dbus.service.Object):
    """One fake Bluetooth device (org.bluez.Device1)."""

    def __init__(self, bluez, address, name, kind):
        self.bluez = bluez
        self.address = address
        self.name = name
        self.kind = kind
        self.connected = False
        self.paired = False

        # BlueZ names the object after the address.
        self.path = "/org/bluez/hci0/dev_" + address.replace(":", "_")

        super().__init__(bluez.bus, self.path)
        bluez.devices[self.path] = self

    def properties(self):
        """The properties reported by GetManagedObjects."""
        return dbus.Dictionary(
            {
                "Address": self.address,
                "Name": self.name,
                "AddressType": "public",
                "RSSI": dbus.Int16(-50),
                "Paired": dbus.Boolean(self.paired),
                "Connected": dbus.Boolean(self.connected),
            },
            signature="sv",
        )

    @dbus.service.method("org.bluez.Device1", async_callbacks=("ok", "error"))
    def Connect(self, ok, error):
        self.bluez.log("Connect " + self.name)

        if self.kind == "slow":
            # Answer much later, so the caller can cancel.
            GLib.timeout_add(SLOW_CONNECT_MS, lambda: ok() and False)
            return

        self.connected = True
        ok()

    @dbus.service.method("org.bluez.Device1")
    def Disconnect(self):
        self.bluez.log("Disconnect " + self.name)

        # A slow device is never connected, but Disconnect is also
        # how a pending connection attempt is aborted.
        if not self.connected and self.kind != "slow":
            raise BluezError("NotConnected", "Not Connected")

        self.connected = False

    @dbus.service.method("org.bluez.Device1")
    def CancelPairing(self):
        self.bluez.log("CancelPairing " + self.name)

    @dbus.service.method("org.bluez.Device1", async_callbacks=("ok", "error"))
    def Pair(self, ok, error):
        has_agent = self.bluez.agent_path is not None

        self.bluez.log("Pair %s agent=%s" % (self.name, has_agent))

        if self.paired:
            error(BluezError("AlreadyExists", "Already Exists"))
            return

        def paired(*_ignored):
            self.paired = True
            ok()

        def refused():
            error(
                BluezError("AuthenticationFailed", "Authentication Failed")
            )

        def agent_failed(agent_error):
            # The agent answered with an error: the user refused
            # or gave no code.
            self.bluez.log("agent error: " + agent_error.get_dbus_name())
            refused()

        if self.kind == "just":
            paired()
            return

        if self.kind == "slowpair":
            # Never answer; the caller has to cancel.
            return

        if not has_agent:
            error(BluezError("AuthenticationFailed", "No agent available"))
            return

        agent = self.bluez.agent()

        if self.kind == "pin":
            agent.RequestPinCode(
                self.path,
                timeout=AGENT_TIMEOUT,
                reply_handler=lambda pin: (
                    paired() if pin == CORRECT_PIN else refused()
                ),
                error_handler=agent_failed,
            )
        elif self.kind == "passkey":
            agent.RequestPasskey(
                self.path,
                timeout=AGENT_TIMEOUT,
                reply_handler=lambda passkey: (
                    paired() if int(passkey) == CORRECT_PASSKEY else refused()
                ),
                error_handler=agent_failed,
            )
        elif self.kind == "confirm":
            agent.RequestConfirmation(
                self.path,
                dbus.UInt32(CONFIRMATION_NUMBER),
                timeout=AGENT_TIMEOUT,
                reply_handler=paired,
                error_handler=agent_failed,
            )
        elif self.kind == "keyboard":
            # Like the real BlueZ: show the code, then report
            # progress as digits are typed on the device.
            def digits_typed():
                agent.DisplayPasskey(
                    self.path,
                    dbus.UInt32(KEYBOARD_PASSKEY),
                    dbus.UInt16(3),
                    reply_handler=paired,
                    error_handler=agent_failed,
                )

            agent.DisplayPasskey(
                self.path,
                dbus.UInt32(KEYBOARD_PASSKEY),
                dbus.UInt16(0),
                reply_handler=digits_typed,
                error_handler=agent_failed,
            )


class Adapter(dbus.service.Object):
    """The fake Bluetooth card hci0 (org.bluez.Adapter1)."""

    def __init__(self, bluez):
        self.bluez = bluez
        super().__init__(bluez.bus, "/org/bluez/hci0")

    @dbus.service.method("org.bluez.Adapter1")
    def StartDiscovery(self):
        # All fake devices are always "in range".
        pass

    @dbus.service.method("org.bluez.Adapter1")
    def StopDiscovery(self):
        pass

    @dbus.service.method("org.bluez.Adapter1", in_signature="o")
    def RemoveDevice(self, path):
        self.bluez.log("RemoveDevice " + path)

        if path not in self.bluez.devices:
            raise BluezError("DoesNotExist", "Does Not Exist")

        # The device disappears from the bus, as in the real BlueZ.
        self.bluez.devices.pop(path).remove_from_connection()


class AgentManager(dbus.service.Object):
    """Registration of pairing agents (org.bluez.AgentManager1)."""

    def __init__(self, bluez):
        self.bluez = bluez
        super().__init__(bluez.bus, "/org/bluez")

    @dbus.service.method(
        "org.bluez.AgentManager1",
        in_signature="os",
        sender_keyword="sender",
    )
    def RegisterAgent(self, path, capability, sender=None):
        self.bluez.log("RegisterAgent %s %s" % (path, capability))
        self.bluez.agent_sender = sender
        self.bluez.agent_path = path

    @dbus.service.method("org.bluez.AgentManager1", in_signature="o")
    def UnregisterAgent(self, path):
        self.bluez.log("UnregisterAgent")
        self.bluez.agent_sender = None
        self.bluez.agent_path = None


class ObjectManager(dbus.service.Object):
    """The list of all objects (org.freedesktop.DBus.ObjectManager)."""

    def __init__(self, bluez):
        self.bluez = bluez
        super().__init__(bluez.bus, "/")

    @dbus.service.method(
        "org.freedesktop.DBus.ObjectManager",
        out_signature="a{oa{sa{sv}}}",
    )
    def GetManagedObjects(self):
        objects = {
            "/org/bluez/hci0": {
                "org.bluez.Adapter1": dbus.Dictionary(
                    {
                        "Address": "00:11:22:33:44:55",
                        "Alias": "fake",
                        "Powered": dbus.Boolean(True),
                        "Discovering": dbus.Boolean(False),
                    },
                    signature="sv",
                )
            }
        }

        for path, device in self.bluez.devices.items():
            objects[path] = {"org.bluez.Device1": device.properties()}

        return objects


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: fake_bluez.py LOG_FILE")

    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

    # The private bus of the test is reached as the "session" bus.
    bus = dbus.SessionBus()

    # Keep a reference: the name is released when it is collected.
    bus_name = dbus.service.BusName("org.bluez", bus)

    bluez = FakeBluez(bus, sys.argv[1])

    ObjectManager(bluez)
    AgentManager(bluez)
    Adapter(bluez)

    for number, (name, kind) in enumerate(DEVICES, start=1):
        Device(bluez, "AA:00:00:00:00:%02X" % number, name, kind)

    # Tell the test that the fake can be used now.
    print("ready", flush=True)

    GLib.MainLoop().run()

    del bus_name


if __name__ == "__main__":
    main()
