#!/usr/bin/env python3
"""
End-to-end test of the AI shell of io-master.

The test runs the real io-master executable and types into its AI
shell, the way a user would. The two outside parties that the
shell talks to are replaced by stand-ins, so the test needs no API
key, costs nothing and touches nothing of the real user:

  * The AI service is replaced by fake_api.py, a local HTTPS
    server with a certificate made for this test. The project
    settings point io-master at it ("base_url", "ca_file").
    Because it is a real HTTPS server, the security rules of
    io-master (HTTPS only, TLS 1.3, certificate checks) are
    tested for real and not bypassed.

  * The keyring of the system is replaced by fake_keyring.py on
    a private D-Bus bus (see testlib.isolate()), so the real
    keyring of the user is never read or changed.

io-master runs as a copy in a temporary directory, so the
configuration of the real build is left alone.

Usage:
    test_ai_shell.py PATH_TO_IO_MASTER

The exit code is 0 when every check passes and 1 otherwise.

Requires dbus-run-session, openssl and the python3-dbus and
python3-gi packages.
"""

import json
import os
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
    run_shell,
)

HERE = os.path.dirname(os.path.abspath(__file__))
FAKE_API = os.path.join(HERE, "fake_api.py")
FAKE_KEYRING = os.path.join(HERE, "fake_keyring.py")

# The key that fake_api.py accepts, and one that it rejects.
VALID_KEY = "sk-test-valid-key"
WRONG_KEY = "sk-test-wrong-key"

# Name of the keyring entry used by the test project.
KEY_NAME = "anthropic"


class Background:
    """
    Runs one of the stand-ins for the duration of a 'with' block.

    The stand-in prints one line when it is ready; that line is
    kept in self.first_line (fake_api.py prints its port there).
    """

    def __init__(self, arguments):
        self.arguments = arguments
        self.process = None
        self.first_line = None

    def __enter__(self):
        self.process = subprocess.Popen(
            [sys.executable] + self.arguments,
            stdout=subprocess.PIPE,
            text=True,
        )
        self.first_line = self.process.stdout.readline().strip()

        if not self.first_line:
            raise RuntimeError("%s did not start" % self.arguments[0])

        return self

    def __exit__(self, *_exception):
        self.process.send_signal(signal.SIGTERM)
        self.process.wait()


def read_lines(path):
    """Returns the lines of a log file."""
    with open(path) as log_file:
        return log_file.read().splitlines()


def read_requests(path):
    """Returns the requests logged by fake_api.py."""
    return [json.loads(line) for line in read_lines(path)]


def make_certificate(directory):
    """
    Creates a certificate for 127.0.0.1 that is valid only for
    this test, together with its private key.

    The certificate is self-signed: it is its own certificate
    authority, so the same file is given to io-master as the
    authority to trust ("ca_file").

    Returns (certificate_path, key_path).
    """
    certificate = os.path.join(directory, "certificate.pem")
    key = os.path.join(directory, "key.pem")

    subprocess.run(
        [
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
            "-days", "2",
            "-subj", "/CN=127.0.0.1",
            "-addext", "subjectAltName=IP:127.0.0.1",
            "-keyout", key,
            "-out", certificate,
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    return certificate, key


def write_settings(directory, base_url, ca_file, **overrides):
    """
    Writes the settings of the test project, pointing it at
    a stand-in server. overrides replace individual settings.
    """
    settings = {
        "provider": "anthropic",
        "model": "claude-opus-5-5",
        "key_name": KEY_NAME,
        "max_tokens": 16000,
        "system": "",
        "refusal_fallback": True,
        "base_url": base_url,
        "ca_file": ca_file,
    }
    settings.update(overrides)

    path = os.path.join(directory, "im_config", "projects", "1.json")

    with open(path, "w") as settings_file:
        json.dump(settings, settings_file)


def test_key_and_conversation(directory, certificate, key):
    """Storing the key, then a conversation of several messages."""
    api_log = os.path.join(directory, "api1.log")
    keyring_log = os.path.join(directory, "keyring1.log")

    with Background([FAKE_KEYRING, keyring_log]), \
            Background([FAKE_API, api_log, certificate, key, "1.3"]) as api:

        write_settings(
            directory,
            "https://127.0.0.1:" + api.first_line,
            certificate,
            system="Be brief.",
        )

        output, exit_code = run_shell(
            directory,
            [
                b"hello\n",                       # no key yet
                b"/key\n", VALID_KEY.encode() + b"\n",
                b"hello\n", 1,
                b"and again\n", 1,
                b"/status\n",
                b"/new\n",
                b"fresh start\n", 1,
                b"/bogus\n",
                b"/help\n",
                b"/exit\n",
            ],
        )

        # Second run: the key must come from the keyring now.
        second_output, _ = run_shell(
            directory, [b"/status\n", b"hello\n", 1, b"/exit\n"]
        )

    requests = read_requests(api_log)

    check("key: shell exits with code 0", exit_code == 0)
    check(
        "key: a missing key is reported at start",
        'No API key is stored under the name "anthropic"' in output,
    )
    check(
        "key: no request is sent without a key",
        "No API key. Store one with: /key" in output,
    )
    check(
        "key: the typed key is not shown on the terminal",
        VALID_KEY not in output,
    )
    check(
        "key: the key is stored in the keyring under its name",
        "store key_name=anthropic secret=" + VALID_KEY
        in read_lines(keyring_log),
    )
    check(
        "key: a later run loads the key from the keyring",
        'loaded from the keyring ("anthropic")' in second_output
        and "reply 1 to: hello" in second_output,
    )
    check(
        "key: the key is not written to the project settings",
        VALID_KEY
        not in open(
            os.path.join(directory, "im_config", "projects", "1.json")
        ).read(),
    )
    check(
        "chat: the reply is printed",
        "reply 1 to: hello" in output,
    )
    check(
        "chat: the model and the token counts are shown",
        "[claude-opus-5-5, 10 tokens in, 5 tokens out]" in output,
    )
    check(
        "chat: the conversation is remembered",
        "reply 2 to: and again" in output
        and "Conversation: 4 messages" in output,
    )
    check(
        "chat: /new starts over",
        "Started a new conversation." in output
        and "reply 1 to: fresh start" in output,
    )
    check(
        "chat: an unknown command is refused",
        "Unknown command: '/bogus'" in output,
    )

    first = requests[0]

    check(
        "request: goes to /v1/messages over TLS 1.3 with a valid key",
        first["path"] == "/v1/messages"
        and first["tls"] == "TLSv1.3"
        and first["key_is_valid"],
    )
    check(
        "request: carries the API version and the fallback opt-in",
        first["anthropic-version"] == "2023-06-01"
        and first["anthropic-beta"] == "server-side-fallback-2026-07-01"
        and first["body"]["fallbacks"] == "default",
    )
    check(
        "request: carries model, max_tokens and the system prompt",
        first["body"]["model"] == "claude-opus-5-5"
        and first["body"]["max_tokens"] == 16000
        and first["body"]["system"] == "Be brief.",
    )
    check(
        "request: earlier replies are sent back unchanged",
        requests[1]["body"]["messages"][1]["content"][0]
        == {"type": "thinking", "thinking": "", "signature": "signature-1"},
    )


def test_errors(directory, certificate, key):
    """Refusals, failures of the service and cancelling."""
    api_log = os.path.join(directory, "api2.log")
    keyring_log = os.path.join(directory, "keyring2.log")

    with Background([FAKE_KEYRING, keyring_log, KEY_NAME + "=" + VALID_KEY]), \
            Background([FAKE_API, api_log, certificate, key, "1.3"]) as api:

        write_settings(
            directory,
            "https://127.0.0.1:" + api.first_line,
            certificate,
            refusal_fallback=False,
        )

        output, exit_code = run_shell(
            directory,
            [
                b"refuse\n", 1,
                b"bad request\n", 1,
                b"/status\n",
                b"overloaded once\n", 3,       # retried after 1 s
                b"always overloaded\n", 5,     # two retries: 1 s + 2 s
                b"slow\n", 1, CTRL_C, 2,
                b"long\n", 1,
                b"fallback\n", 1,
                b"escape\n", 1,
                b"/exit\n",
            ],
        )

    requests = read_requests(api_log)

    def tries(text):
        """Number of requests whose last message was text."""
        return sum(
            1 for request in requests
            if request["body"]["messages"][-1]["content"] == text
        )

    check("errors: the shell survives every error", exit_code == 0)
    check(
        "errors: a refusal is reported with its category",
        "the model declined to answer (category: cyber)" in output,
    )
    check(
        "errors: a rejected request shows the reason of the service",
        "max_tokens: too large (HTTP 400, invalid_request_error)" in output,
    )
    check(
        "errors: failed messages are not kept in the conversation",
        "Conversation: 0 messages" in output,
    )
    check(
        "errors: a busy service is tried again",
        "recovered" in output and tries("overloaded once") == 2,
    )
    check(
        "errors: retries stop after two more tries",
        "The AI service has a problem" in output
        and tries("always overloaded") == 3,
    )
    check(
        "errors: Ctrl+C cancels a request",
        "Cancelled" in output and "slow reply arrived" not in output,
    )
    check(
        "reply: a cut-off reply is marked",
        "cut off here" in output
        and "the reply was cut off by the 'max_tokens' setting" in output,
    )
    check(
        "reply: a fallback model is named",
        "[claude-opus-4-8," in output,
    )
    check(
        "reply: terminal control codes are not passed on",
        "plain " in output and "\x1b" not in output and "\x07" not in output,
    )
    check(
        "request: no fallback opt-in when the setting is off",
        requests[0]["anthropic-beta"] is None
        and "fallbacks" not in requests[0]["body"],
    )


def test_wrong_key(directory, certificate, key):
    """A key that the service rejects."""
    api_log = os.path.join(directory, "api3.log")
    keyring_log = os.path.join(directory, "keyring3.log")

    with Background([FAKE_KEYRING, keyring_log, KEY_NAME + "=" + WRONG_KEY]), \
            Background([FAKE_API, api_log, certificate, key, "1.3"]) as api:

        write_settings(
            directory, "https://127.0.0.1:" + api.first_line, certificate
        )

        output, _ = run_shell(directory, [b"hello\n", 1, b"/exit\n"])

    check(
        "key: a rejected key is reported",
        "The service rejected the API key: invalid x-api-key" in output,
    )
    check("key: a rejected key is never printed", WRONG_KEY not in output)


def test_transport_security(directory, certificate, key):
    """The API key is only sent over HTTPS with TLS 1.3."""
    keyring_log = os.path.join(directory, "keyring4.log")
    modern_log = os.path.join(directory, "api4-modern.log")
    old_log = os.path.join(directory, "api4-old.log")

    with Background([FAKE_KEYRING, keyring_log, KEY_NAME + "=" + VALID_KEY]), \
            Background([FAKE_API, modern_log, certificate, key, "1.3"]) as modern, \
            Background([FAKE_API, old_log, certificate, key, "1.2"]) as old:

        # 1. A plain http:// address.
        write_settings(
            directory, "http://127.0.0.1:" + modern.first_line, certificate
        )
        plain_output, _ = run_shell(directory, [b"hello\n", 1, b"/exit\n"])

        # 2. A server that offers nothing newer than TLS 1.2.
        write_settings(
            directory, "https://127.0.0.1:" + old.first_line, certificate
        )
        old_output, _ = run_shell(directory, [b"hello\n", 4, b"/exit\n"])

        # 3. A certificate that nobody vouches for: without
        #    "ca_file" only the certificates of the system count.
        write_settings(
            directory, "https://127.0.0.1:" + modern.first_line, ""
        )
        untrusted_output, _ = run_shell(
            directory, [b"hello\n", 4, b"/exit\n"]
        )

    check(
        "security: the key is not sent to an http:// address",
        "the API key is only sent over HTTPS" in plain_output
        and "reply 1" not in plain_output,
    )
    check(
        "security: a server limited to TLS 1.2 is refused",
        "Cannot reach the AI service" in old_output
        and "reply 1" not in old_output
        and read_lines(old_log) == [],
    )
    check(
        "security: an untrusted certificate is refused",
        "Cannot reach the AI service" in untrusted_output
        and "reply 1" not in untrusted_output,
    )
    check(
        "security: no request reached a server in these cases",
        read_lines(modern_log) == [],
    )


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: test_ai_shell.py PATH_TO_IO_MASTER")

    io_master = os.path.abspath(sys.argv[1])

    # Restart on a private bus and with a private home directory,
    # so the real keyring of the user cannot be reached.
    isolate(__file__, [io_master])

    with tempfile.TemporaryDirectory(prefix="io-master-test-") as directory:
        install_binary(io_master, directory)

        run_io_master(directory, ["project", "add", "test", "--protocol", "ai"])
        run_io_master(directory, ["project", "select", "1"])

        certificate, key = make_certificate(directory)

        for test in (
            test_key_and_conversation,
            test_errors,
            test_wrong_key,
            test_transport_security,
        ):
            test(directory, certificate, key)

    finish()


if __name__ == "__main__":
    main()
