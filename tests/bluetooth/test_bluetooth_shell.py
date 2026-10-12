#!/usr/bin/env python3
"""
End-to-end test of the Bluetooth shell of io-master.

The test runs the real io-master executable and types commands into
its Bluetooth shell, the way a user would. BlueZ is replaced by
fake_bluez.py, so no Bluetooth card and no real devices are needed
and nothing outside of the test is touched:

  * Private bus. The test restarts itself under dbus-run-session
    (see testlib.isolate()), which starts a D-Bus daemon only for
    this test. io-master is pointed at it through
    DBUS_SYSTEM_BUS_ADDRESS, so it never talks to the real BlueZ
    on the system bus.

  * Private configuration. io-master keeps its configuration next
    to the executable. The test copies the executable to a
    temporary directory and runs the copy, so the configuration
    of the real build is left alone.

  * Pseudo-terminal. The shell is driven through a pseudo-terminal,
    so the live scan table and Ctrl+C behave as in a real terminal.

Usage:
    test_bluetooth_shell.py PATH_TO_IO_MASTER

The exit code is 0 when every check passes and 1 otherwise.

Requires dbus-run-session and the python3-dbus and python3-gi
packages.
"""

import os
import re
import signal
import subprocess
import sys
import tempfile

# The shared test helpers live one directory up, in tests/.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from testlib import (  # noqa: E402
    CTRL_C,
    check,
    finish,
    install_binary,
    isolate,
    run_io_master,
    run_shell as run_shell_in_terminal,
)

HERE = os.path.dirname(os.path.abspath(__file__))
FAKE_BLUEZ = os.path.join(HERE, "fake_bluez.py")


class FakeBluez:
    """Runs fake_bluez.py for the duration of a 'with' block."""

    def __init__(self, log_path):
        self.log_path = log_path
        self.process = None

    def __enter__(self):
        self.process = subprocess.Popen(
            [sys.executable, FAKE_BLUEZ, self.log_path],
            stdout=subprocess.PIPE,
            text=True,
        )

        # The fake prints "ready" once it owns the org.bluez name.
        if self.process.stdout.readline().strip() != "ready":
            raise RuntimeError("fake_bluez.py did not start")

        return self

    def __exit__(self, *_exception):
        self.process.send_signal(signal.SIGTERM)
        self.process.wait()

    def log(self):
        """Returns the calls received so far, one per line."""
        with open(self.log_path) as log_file:
            return log_file.read().splitlines()


def run_shell(directory, steps):
    """
    Runs the Bluetooth shell and types steps into it; see
    testlib.run_shell(). io-master is pointed at the private bus
    of the test as its system bus, where the fake BlueZ lives.
    """
    environment = dict(os.environ)

    environment["DBUS_SYSTEM_BUS_ADDRESS"] = environment[
        "DBUS_SESSION_BUS_ADDRESS"
    ]

    return run_shell_in_terminal(directory, steps, environment)


def prepare_project(directory, io_master):
    """
    Copies io-master into directory and creates one Bluetooth
    project in it, with the card hci0 already selected.
    """
    install_binary(io_master, directory)

    run_io_master(
        directory,
        ["project", "add", "test", "--protocol", "bluetooth"],
    )
    run_io_master(directory, ["project", "select", "1"])

    reset_settings(directory)


def settings_path(directory):
    return os.path.join(directory, "im_config", "projects", "1.json")


def reset_settings(directory):
    """Selects hci0 and clears the target device of the project."""
    with open(settings_path(directory), "w") as settings:
        settings.write('{"adapter": "hci0", "device": ""}')


def scan():
    """Steps that run 'scan' for a moment and stop it with Ctrl+C."""
    return [b"scan\n", 1.5, CTRL_C, 1]


def test_scan_and_ids(directory):
    """The scan lists every device once and gives it an id."""
    with FakeBluez(os.path.join(directory, "scan.log")):
        output, exit_code = run_shell(
            directory,
            scan() + [b"devices\n", b"info 2\n", b"info 99\n", b"exit\n"],
        )

    check("scan: shell exits with code 0", exit_code == 0)
    check("scan: reports 7 devices", "7 devices found" in output)

    # The static table printed after the scan.
    table = output[output.rindex("7 devices found"):]

    check(
        "scan: ids are assigned in order",
        re.search(r"^\s+1\s+AA:00:00:00:00:01\b", table, re.M)
        and re.search(r"^\s+7\s+AA:00:00:00:00:07\b", table, re.M),
    )
    check(
        "info: an id is resolved to its device",
        "Address:      AA:00:00:00:00:02" in output,
    )
    check("info: an unknown id is refused", "No device with ID 99" in output)


def test_pairing(directory):
    """Pairing without a code and with every kind of code."""
    with FakeBluez(os.path.join(directory, "pair.log")) as bluez:
        output, _ = run_shell(
            directory,
            scan()
            + [
                b"pair 1\n",                 # no code needed
                b"pair 1\n",                 # already paired
                b"pair 2\n", b"1234\n",      # PIN
                b"pair 3\n", b"123456\n",    # passkey
                b"pair 4\n", b"y\n",         # numeric comparison
                b"pair 6\n", 1,              # code shown for the device
                b"unpair 2\n",
                b"exit\n",
            ],
        )
        log = bluez.log()

    # Only the part after the scan matters here.
    output = output[output.index("bluetooth> pair 1"):]

    check(
        "pair: a device without a code pairs without a question",
        "Paired with 1: AA:00:00:00:00:01 (JustWorks)" in output
        and "PIN:" not in output.split("bluetooth> pair 2")[0],
    )
    check("pair: pairing twice is reported", "Already paired with 1:" in output)
    check(
        "pair: a PIN is asked for and accepted",
        "requires a PIN" in output and "Paired with 2:" in output,
    )
    check(
        "pair: a passkey is asked for and accepted",
        "Passkey:" in output and "Paired with 3:" in output,
    )
    check(
        "pair: a number is confirmed",
        "should show this number: 054321" in output
        and "Paired with 4:" in output,
    )
    check(
        "pair: a code for the device is displayed",
        "Type this passkey on device AA:00:00:00:00:06" in output
        and "111222" in output
        and "Paired with 6:" in output,
    )
    check(
        "pair: the agent is registered and unregistered",
        log.count("RegisterAgent /io_master/agent KeyboardDisplay") == 6
        and log.count("UnregisterAgent") == 6,
    )
    check(
        "unpair: the device is removed from the system",
        "Removed the pairing with 2:" in output
        and "RemoveDevice /org/bluez/hci0/dev_AA_00_00_00_00_02" in log,
    )


def test_pairing_refusals(directory):
    """Wrong codes, missing codes and Ctrl+C abort the pairing."""
    with FakeBluez(os.path.join(directory, "refuse.log")) as bluez:
        output, exit_code = run_shell(
            directory,
            scan()
            + [
                b"pair 2\n", b"9999\n",      # wrong PIN
                b"pair 2\n", b"\n",          # no PIN
                b"pair 2\n", CTRL_C, 1,      # Ctrl+C at the PIN prompt
                b"pair 3\n", b"abc\n",       # not a passkey
                b"pair 4\n", b"n\n",         # numbers differ
                b"pair 7\n", 1, CTRL_C, 1,   # Ctrl+C while waiting
                b"exit\n",
            ],
        )
        log = bluez.log()

    output = output[output.index("bluetooth> pair 2"):]

    check(
        "pair: the shell survives every refusal",
        exit_code == 0 and "bluetooth> exit" in output,
    )
    check("pair: no device was paired", "Paired with" not in output)
    check(
        "pair: a wrong PIN fails",
        "failed: Authentication Failed" in output,
    )
    check(
        "pair: an empty PIN cancels",
        "No PIN given, the pairing is cancelled." in output,
    )
    check(
        "pair: an invalid passkey cancels",
        "'abc' is not a passkey" in output,
    )
    check(
        "pair: the agent rejects when the user refuses",
        log.count("agent error: org.bluez.Error.Rejected") == 4,
    )
    check(
        "pair: Ctrl+C tells the system to cancel the pairing",
        "CancelPairing PinDevice" in log
        and "CancelPairing SlowPair" in log
        and "Cancelled" in output,
    )


def test_connections(directory):
    """Connecting, disconnecting, cancelling and leaving the shell."""
    with FakeBluez(os.path.join(directory, "connect.log")) as bluez:
        output, exit_code = run_shell(
            directory,
            scan()
            + [
                b"connect 5\n", 1, CTRL_C, 1,   # Ctrl+C while connecting
                b"connect 1\n",
                b"status\n",
                b"disconnect\n",                 # target device
                b"disconnect\n",                 # not connected any more
                b"connect\n",                    # target device again
                b"connect aa:00:00:00:00:01\n",  # by address, lower case
                b"exit\n",
            ],
        )
        log = bluez.log()

    output = output[output.index("bluetooth> connect 5"):]

    check("connect: shell exits with code 0", exit_code == 0)
    check(
        "connect: Ctrl+C aborts the attempt",
        "Cancelled" in output
        and log[:2] == ["Connect SlowDevice", "Disconnect SlowDevice"],
    )
    check(
        "connect: an id connects its device",
        "Connected to 1: AA:00:00:00:00:01 (JustWorks)" in output,
    )
    check(
        "status: the target device is shown as connected",
        "Device:  AA:00:00:00:00:01 (JustWorks)" in output
        and "connected, not paired" in output,
    )
    check(
        "disconnect: without a device uses the target device",
        "Disconnected from 1:" in output and "Not connected to 1:" in output,
    )
    check(
        "connect: an address in lower case is accepted",
        "Already connected to 1: AA:00:00:00:00:01" in output,
    )
    check(
        "exit: the connection made by the session is closed",
        "Disconnecting from AA:00:00:00:00:01" in output
        and log[-1] == "Disconnect JustWorks",
    )

    with open(settings_path(directory)) as settings:
        check(
            "connect: the target device is stored in the project",
            '"device": "AA:00:00:00:00:01"' in settings.read(),
        )


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: test_bluetooth_shell.py PATH_TO_IO_MASTER")

    io_master = os.path.abspath(sys.argv[1])

    # Restart on a private bus, so the test never uses the bus of
    # the desktop session or the system bus.
    isolate(__file__, [io_master])

    with tempfile.TemporaryDirectory(prefix="io-master-test-") as directory:
        prepare_project(directory, io_master)

        for test in (
            test_scan_and_ids,
            test_pairing,
            test_pairing_refusals,
            test_connections,
        ):
            # Every test starts from the same project settings.
            reset_settings(directory)
            test(directory)

    finish()


if __name__ == "__main__":
    main()
