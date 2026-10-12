"""
Helpers shared by the end-to-end tests of io-master.

The tests run the real io-master executable and type into its
shells. This module holds what every such test needs:

    check()            records and prints one PASS/FAIL result
    finish()           prints the summary and ends the test
    isolate()          restarts the test on a private D-Bus bus
                       and with a private home directory
    install_binary()   copies io-master to a temporary directory
    run_shell()        runs a shell in a pseudo-terminal and
                       types into it
"""

import os
import pty
import re
import select
import shutil
import subprocess
import sys
import tempfile
import time

# Set in the environment once the test runs in isolation.
ISOLATION_MARKER = "IO_MASTER_TEST_ISOLATED"

# The byte that a terminal sends for Ctrl+C.
CTRL_C = b"\x03"

# How long to wait for the output of one typed line, in seconds.
STEP_SECONDS = 0.7

# Terminal control sequences that io-master itself prints, such as
# the ones that redraw the Bluetooth scan table. They are removed
# before the output is checked.
CONTROL_SEQUENCE = re.compile(r"\x1b\[[0-9?]*[A-Za-z]")

# Results of all checks: (description, passed).
_results = []


def check(description, condition):
    """Records the result of one check and prints it."""
    _results.append((description, bool(condition)))
    print("%s  %s" % ("PASS" if condition else "FAIL", description))


def finish():
    """Prints the summary and exits with 0 (all passed) or 1."""
    failed = [text for text, passed in _results if not passed]

    print("\n%d checks, %d failed" % (len(_results), len(failed)))

    sys.exit(1 if failed else 0)


def isolate(script, arguments):
    """
    Makes sure the test cannot touch anything of the real user.

    On the first call the test restarts itself:

      * under dbus-run-session, which starts a D-Bus daemon only
        for this test, so the test never uses the bus of the
        desktop session or the system bus;

      * with HOME and the XDG directories pointing to a temporary
        directory, so that a service started on that private bus
        cannot read or write the real files of the user, for
        example the real keyring.

    The restarted test finds ISOLATION_MARKER in its environment
    and simply continues.
    """
    if ISOLATION_MARKER in os.environ:
        return

    home = tempfile.mkdtemp(prefix="io-master-test-home-")

    os.environ[ISOLATION_MARKER] = "1"
    os.environ["HOME"] = home
    os.environ["XDG_DATA_HOME"] = os.path.join(home, "data")
    os.environ["XDG_CONFIG_HOME"] = os.path.join(home, "config")
    os.environ["XDG_CACHE_HOME"] = os.path.join(home, "cache")

    os.execvp(
        "dbus-run-session",
        ["dbus-run-session", "--", sys.executable, script] + arguments,
    )


def install_binary(io_master, directory):
    """
    Copies io-master into directory.

    io-master keeps its configuration next to the executable, so
    running a copy keeps the configuration of the real build
    untouched.
    """
    shutil.copy(io_master, os.path.join(directory, "io-master"))


def run_io_master(directory, arguments):
    """Runs one io-master command in directory and waits for it."""
    subprocess.run(
        ["./io-master"] + arguments,
        cwd=directory,
        check=True,
        stdout=subprocess.DEVNULL,
    )


def run_shell(directory, steps, environment=None):
    """
    Runs "io-master run" in a pseudo-terminal and feeds it steps.

    A step is either bytes, which are typed into the shell, or
    a number, which is a pause in seconds. The last step should
    leave the shell, otherwise the function waits forever.

    The pseudo-terminal makes the shell behave as in a real
    terminal: Ctrl+C is delivered as a signal and typed text is
    echoed, unless the shell turns the echo off.

    Returns (output, exit_code). Terminal control sequences of
    the CONTROL_SEQUENCE kind are replaced with line breaks and
    line ends are normalized.
    """
    if environment is None:
        environment = dict(os.environ)

    process_id, terminal = pty.fork()

    if process_id == 0:
        # Child: become io-master.
        os.chdir(directory)
        os.execve("./io-master", ["./io-master", "run"], environment)

    output = b""

    def collect(seconds):
        """Reads what the shell prints during the given time."""
        nonlocal output
        end = time.time() + seconds

        while time.time() < end:
            readable, _, _ = select.select([terminal], [], [], 0.05)

            if readable:
                try:
                    output += os.read(terminal, 65536)
                except OSError:
                    # The shell has closed the terminal.
                    return

    collect(0.5)

    for step in steps:
        if isinstance(step, (int, float)):
            collect(step)
        else:
            os.write(terminal, step)
            collect(STEP_SECONDS)

    _, status = os.waitpid(process_id, 0)

    text = CONTROL_SEQUENCE.sub("\n", output.decode(errors="replace"))

    return text.replace("\r\n", "\n"), os.waitstatus_to_exitcode(status)
