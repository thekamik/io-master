#!/usr/bin/env python3
"""
End-to-end test of the AI integration of io-master: the
configuration questions ("io-master aic") and the chat of the
"ai" protocol ("io-master run"), including the saved chats.

The test runs the real io-master executable and types into those
shells, the way a user would. The two outside parties that the
shell talks to are replaced by stand-ins, so the test needs no API
key, costs nothing and touches nothing of the real user:

  * The AI service is replaced by fake_api.py, a local HTTPS
    server with a certificate made for this test. The AI
    configuration points io-master at it ("base_url", "ca_file").
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
import re
import shutil
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


def settings_path(directory):
    """Path of the AI configuration file of the test."""
    return os.path.join(directory, "im_config", "ai", "ai_config.json")


def write_settings(directory, base_url, ca_file, **overrides):
    """
    Writes the AI configuration, pointing io-master at a stand-in
    server. overrides replace individual settings.
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

    os.makedirs(os.path.dirname(settings_path(directory)), exist_ok=True)

    with open(settings_path(directory), "w") as settings_file:
        json.dump(settings, settings_file)


def read_settings(directory):
    """Returns the AI configuration as io-master saved it."""
    with open(settings_path(directory)) as settings_file:
        return json.load(settings_file)


def run_config(directory, steps):
    """Runs "io-master aic" and types the answers in steps."""
    return run_shell(directory, steps, arguments=("aic",))


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

        # The chat before any key is stored.
        early_output, _ = run_shell(directory, [b"new\n", b"exit\n"])

        # Store the key with "io-master aic": keep the provider
        # and the model, type the key, skip the test.
        config_output, config_exit = run_config(
            directory,
            [b"\n", b"\n", VALID_KEY.encode() + b"\n", b"n\n"],
        )

        # The chat now finds the key in the keyring.
        output, exit_code = run_shell(
            directory,
            [
                b"new\n",
                b"hello\n", 1,
                b"and again\n", 1,
                CTRL_C,                  # leave the chat
                b"new\n",
                b"fresh start\n", 1,
                CTRL_C,
                b"bogus\n",
                b"key\n",
                b"help\n",
                b"exit\n",
            ],
        )

    requests = read_requests(api_log)

    check(
        "key: both shells exit with code 0",
        exit_code == 0 and config_exit == 0,
    )
    check(
        "key: the chat reports a missing key and points to ai-config",
        "No API key is stored for provider anthropic." in early_output
        and "io-master ai-config" in early_output,
    )
    check(
        "key: no request is sent without a key",
        "Chat 1" not in early_output and "reply 1" not in early_output,
    )
    check(
        "key: the typed key is not shown on the terminal",
        VALID_KEY not in config_output,
    )
    check(
        "key: the configuration confirms that the key was stored",
        "The key was stored in the keyring." in config_output,
    )
    check(
        "key: the key is stored in the keyring under its name",
        "store key_name=anthropic secret=" + VALID_KEY
        in read_lines(keyring_log),
    )
    check(
        "key: the chat loads the key from the keyring",
        "API key:  loaded from the keyring" in output,
    )
    check(
        "key: the key is not written to any configuration file",
        not any(
            VALID_KEY in open(os.path.join(folder, name)).read()
            for folder, _, names in os.walk(
                os.path.join(directory, "im_config")
            )
            for name in names
        ),
    )
    check(
        "chat: configuration commands are not part of the shell",
        "Unknown command: 'key'" in output,
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
        and "Left chat 1. It is saved with 4 messages." in output,
    )
    check(
        "chat: a new chat starts over",
        "Chat 2" in output and "reply 1 to: fresh start" in output,
    )
    check(
        "chat: an unknown command is refused",
        "Unknown command: 'bogus'" in output,
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
                b"new\n",
                b"refuse\n", 1,
                b"bad request\n", 1,
                b"overloaded once\n", 3,       # retried after 1 s
                b"always overloaded\n", 5,     # two retries: 1 s + 2 s
                b"slow\n", 1, CTRL_C, 2,
                b"long\n", 1,
                b"fallback\n", 1,
                b"escape\n", 1,
                CTRL_C,
                b"exit\n",
            ],
        )

        # What ended up in the chat file.
        chat_path = os.path.join(
            directory, "im_config", "ai", "chats", "1"
        )
        newest = max(
            int(name.split(".")[0])
            for name in os.listdir(chat_path)
            if name[0].isdigit()
        )

        with open(os.path.join(chat_path, "%d.json" % newest)) as chat:
            recorded = [
                entry["text"]
                for entry in json.load(chat)["transcript"]
                if entry["role"] == "user"
            ]

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
        "errors: only answered messages are recorded in the chat",
        recorded == ["overloaded once", "long", "fallback", "escape"],
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
        "errors: Ctrl+C cancels a request and stays in the chat",
        "Cancelled" in output
        and "slow reply arrived" not in output
        and "cut off here" in output,
    )
    check(
        "reply: a cut-off reply is marked",
        "cut off here" in output
        and "the reply was cut off by the max_tokens limit" in output,
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


def test_gemini(directory, certificate, key):
    """A conversation with the Gemini provider and its errors."""
    api_log = os.path.join(directory, "api5.log")
    keyring_log = os.path.join(directory, "keyring5.log")

    with Background([FAKE_KEYRING, keyring_log, "gemini=" + VALID_KEY]), \
            Background([FAKE_API, api_log, certificate, key, "1.3"]) as api:

        write_settings(
            directory,
            "https://127.0.0.1:" + api.first_line,
            certificate,
            provider="gemini",
            model="gemini-3.8-flash",
            key_name="gemini",
            system="Be brief.",
            max_tokens=2000,
        )

        output, exit_code = run_shell(
            directory,
            [
                b"new\n",
                b"hello\n", 1,
                b"and again\n", 1,
                b"refuse\n", 1,
                b"unsafe\n", 1,
                b"long\n", 1,
                b"quota\n", 5,              # two retries: 1 s + 2 s
                CTRL_C,
                b"exit\n",
            ],
        )

    requests = read_requests(api_log)
    first = requests[0]

    check("gemini: shell exits with code 0", exit_code == 0)
    check(
        "gemini: the reply is printed with model and token counts",
        "gemini reply 1 to: hello" in output
        and "[gemini-3.8-flash, 10 tokens in, 7 tokens out]" in output,
    )
    check(
        "gemini: thoughts of the model are not shown",
        "SECRET THOUGHT" not in output,
    )
    check(
        "gemini: the conversation is remembered",
        "gemini reply 2 to: and again" in output
        and "It is saved with 6 messages." in output,
    )
    check(
        "gemini: a blocked request is reported",
        "the request was blocked (reason: SAFETY)" in output,
    )
    check(
        "gemini: a reply stopped for safety is reported",
        "the model gave no answer (reason: SAFETY)" in output,
    )
    check(
        "gemini: a cut-off reply is marked",
        "the reply was cut off by the max_tokens limit" in output,
    )
    check(
        "gemini: an exhausted quota is reported after retries",
        "Too many requests" in output
        and "Quota exceeded. (HTTP 429, RESOURCE_EXHAUSTED)" in output,
    )
    check(
        "gemini request: right address, TLS 1.3, key in its header",
        first["path"] == "/v1beta/models/gemini-3.8-flash:generateContent"
        and first["tls"] == "TLSv1.3"
        and first["key_is_valid"],
    )
    check(
        "gemini request: carries the token limit and the system prompt",
        first["body"]["generationConfig"]["maxOutputTokens"] == 2000
        and first["body"]["systemInstruction"]
        == {"parts": [{"text": "Be brief."}]},
    )
    check(
        "gemini request: earlier replies are sent back unchanged",
        requests[1]["body"]["contents"][1]["parts"][-1]["thoughtSignature"]
        == "signature-1",
    )


def test_gemini_wrong_key(directory, certificate, key):
    """Gemini reports an invalid key as a plain 400."""
    api_log = os.path.join(directory, "api6.log")
    keyring_log = os.path.join(directory, "keyring6.log")

    with Background([FAKE_KEYRING, keyring_log, "gemini=" + WRONG_KEY]), \
            Background([FAKE_API, api_log, certificate, key, "1.3"]) as api:

        write_settings(
            directory,
            "https://127.0.0.1:" + api.first_line,
            certificate,
            provider="gemini",
            model="gemini-3.8-flash",
            key_name="gemini",
        )

        output, _ = run_shell(
            directory, [b"new\n", b"hello\n", 1, CTRL_C, b"exit\n"]
        )

    check(
        "gemini: a rejected key is recognized as a key problem",
        "The service rejected the API key: API key not valid." in output
        and "io-master ai-config" in output,
    )


def test_configuration(directory, certificate, key):
    """The questions of "io-master aic" and what they store."""
    keyring_log = os.path.join(directory, "keyring7.log")
    api_log = os.path.join(directory, "api7.log")

    # Start without a configuration file: the defaults apply.
    if os.path.exists(settings_path(directory)):
        os.remove(settings_path(directory))

    with Background([FAKE_KEYRING, keyring_log]), \
            Background([FAKE_API, api_log, certificate, key, "1.3"]) as api:

        # 1. Cancelling and giving no key change nothing.
        cancel_output, cancel_exit = run_config(directory, [CTRL_C])
        no_key_output, _ = run_config(
            directory, [b"\n", b"\n", b"\n"]
        )
        nothing_saved = not os.path.exists(settings_path(directory))

        # 2. A first configuration, with answers that have to be
        #    asked again, and without the test.
        first_output, first_exit = run_config(
            directory,
            [
                b"9\n",              # not on the list
                b"gemini\n",         # a provider by name
                b"7\n",              # no such model number
                b"2\n",              # the second suggested model
                b"gemini-secret\n",  # the key
                b"maybe\n",          # neither yes nor no
                b"n\n",              # no test
            ],
        )
        first = read_settings(directory)

        # 3. Claude, pointed at the stand-in server, with the
        #    test. "claude" is the name users know Anthropic by.
        write_settings(
            directory, "https://127.0.0.1:" + api.first_line, certificate
        )
        test_output, test_exit = run_config(
            directory,
            [
                b"claude\n",
                b"3\n",                          # claude-haiku-5-5
                VALID_KEY.encode() + b"\n",
                b"\n", 1,                        # Enter: run the test
            ],
        )
        tested = read_settings(directory)

        # 4. Running it again and accepting every default keeps
        #    the configuration and the stored key.
        again_output, again_exit = run_config(
            directory, [b"\n", b"\n", b"\n", b"y\n", 1]
        )
        again = read_settings(directory)

        # 5. A model name that the provider does not know.
        typo_output, typo_exit = run_config(
            directory, [b"\n", b"claude-typo\n", b"\n", b"y\n", 1]
        )

        # 6. A key that the provider rejects.
        write_settings(
            directory, "https://127.0.0.1:" + api.first_line, certificate
        )
        wrong_output, wrong_exit = run_config(
            directory, [b"\n", b"\n", WRONG_KEY.encode() + b"\n", b"y\n", 1]
        )

    stored = [
        line for line in read_lines(keyring_log) if line.startswith("store")
    ]
    requests = read_requests(api_log)

    check(
        "wizard: Ctrl+C cancels without changing anything",
        "Cancelled. Nothing was changed." in cancel_output
        and cancel_exit == 0,
    )
    check(
        "wizard: no key given changes nothing",
        "No key given. Nothing was changed." in no_key_output
        and nothing_saved,
    )
    check(
        "wizard: the providers are offered as a numbered list",
        "1) claude" in cancel_output and "2) gemini" in cancel_output,
    )
    check(
        "wizard: an answer that is not on the list is asked again",
        "'9' is not on the list" in first_output
        and "There is no model number 7" in first_output
        and "Type y for yes or n for no." in first_output,
    )
    check(
        "wizard: the answers are saved to the AI configuration file",
        first_exit == 0
        and first["provider"] == "gemini"
        and first["model"] == "gemini-3.5-flash-lite"
        and first["key_name"] == "gemini",
    )
    check(
        "wizard: without a test the user is told nothing was tested",
        "The configuration was saved. No test was run" in first_output
        and requests != []
        and all(request["service"] == "anthropic" for request in requests),
    )
    check(
        "wizard: the key goes to the keyring under the provider's name",
        stored[0] == "store key_name=gemini secret=gemini-secret",
    )
    check(
        "wizard: typed keys are never shown on the terminal",
        "gemini-secret" not in first_output
        and VALID_KEY not in test_output
        and WRONG_KEY not in wrong_output,
    )
    check(
        "wizard: a successful test reports a working configuration",
        test_exit == 0
        and "The configuration works: claude-haiku-5-5 answered."
        in test_output,
    )
    check(
        "wizard: the chosen model is saved; other settings are kept",
        tested["provider"] == "anthropic"
        and tested["model"] == "claude-haiku-5-5"
        and tested["max_tokens"] == 16000
        and tested["base_url"].startswith("https://127.0.0.1:"),
    )
    check(
        "wizard: the test is one short, cheap request",
        requests[0]["body"]["model"] == "claude-haiku-5-5"
        and requests[0]["body"]["max_tokens"] == 512
        and len(requests[0]["body"]["messages"]) == 1,
    )
    check(
        "request: no fallback opt-in for Haiku, which has none",
        requests[0]["anthropic-beta"] is None
        and "fallbacks" not in requests[0]["body"],
    )
    check(
        "wizard: accepting every default keeps model and stored key",
        again_exit == 0
        and again == tested
        and "A key is already stored." in again_output
        and "The configuration works" in again_output
        and len(stored) == 3,
    )
    check(
        "wizard: an unknown model is explained",
        typo_exit == 1
        and "the model name is wrong: \"claude-typo\"" in typo_output
        and "saved, but the test did not" in typo_output,
    )
    check(
        "wizard: a rejected key is explained",
        wrong_exit == 1
        and "The provider did not accept the API key." in wrong_output,
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

        output, _ = run_shell(
            directory, [b"new\n", b"hello\n", 1, CTRL_C, b"exit\n"]
        )

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
        plain_output, _ = run_shell(
            directory, [b"new\n", b"hello\n", 1, CTRL_C, b"exit\n"]
        )

        # 2. A server that offers nothing newer than TLS 1.2.
        write_settings(
            directory, "https://127.0.0.1:" + old.first_line, certificate
        )
        old_output, _ = run_shell(
            directory, [b"new\n", b"hello\n", 4, CTRL_C, b"exit\n"]
        )

        # 3. A certificate that nobody vouches for: without
        #    "ca_file" only the certificates of the system count.
        write_settings(
            directory, "https://127.0.0.1:" + modern.first_line, ""
        )
        untrusted_output, _ = run_shell(
            directory, [b"new\n", b"hello\n", 4, CTRL_C, b"exit\n"]
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


def chat_file(directory, chat_id):
    """Path of a saved chat of the test project (project 1)."""
    return os.path.join(
        directory, "im_config", "ai", "chats", "1", "%d.json" % chat_id
    )


def test_chat_history(directory, certificate, key):
    """Saving chats, the chat menu, and continuing a saved chat."""
    api_log = os.path.join(directory, "api8.log")
    keyring_log = os.path.join(directory, "keyring8.log")

    # The earlier tests left chats behind. Remove them, so this
    # test starts with an empty list and the first chat gets ID 1.
    shutil.rmtree(
        os.path.join(directory, "im_config", "ai", "chats"),
        ignore_errors=True,
    )

    with Background([FAKE_KEYRING, keyring_log, KEY_NAME + "=" + VALID_KEY]), \
            Background([FAKE_API, api_log, certificate, key, "1.3"]) as api:

        address = "https://127.0.0.1:" + api.first_line

        write_settings(directory, address, certificate)

        # 1. Two chats in one run of the shell.
        first_output, first_exit = run_shell(
            directory,
            [
                b"new\n",
                b"first question\n", 1,
                CTRL_C,                      # leave the chat
                CTRL_C,                      # in the menu: stay
                b"new Named chat\n",
                b"second question\n", 1,
                CTRL_C,
                b"list\n",
                b"exit\n",
            ],
        )

        with open(chat_file(directory, 1)) as saved_file:
            saved = json.load(saved_file)

        file_mode = os.stat(chat_file(directory, 1)).st_mode & 0o777
        folder_mode = os.stat(
            os.path.dirname(chat_file(directory, 1))
        ).st_mode & 0o777

        # 2. A new run: the chats are still there and one of them
        #    is continued, deleted or kept.
        second_output, _ = run_shell(
            directory,
            [
                b"open 1\n",
                b"follow up\n", 1,
                CTRL_C,
                b"delete 2\n", b"n\n",       # asked, kept
                b"delete 2\n", b"y\n",       # asked, deleted
                b"delete 2\n",               # gone
                b"open 2\n",
                b"open\n",                   # no id
                b"new\n",
                b"third question\n", 1,
                CTRL_C,
                b"list\n",
                b"exit\n",
            ],
        )

        remaining = sorted(
            name
            for name in os.listdir(os.path.dirname(chat_file(directory, 1)))
            if name[0].isdigit()
        )

        # 3. Another model is configured: the chat continues from
        #    its plain text.
        write_settings(
            directory, address, certificate, model="claude-sonnet-5-5"
        )
        third_output, _ = run_shell(
            directory,
            [b"open 1\n", b"after the switch\n", 1, CTRL_C, b"exit\n"],
        )

    requests = read_requests(api_log)

    check("history: shell exits with code 0", first_exit == 0)
    check(
        "history: Ctrl+C leaves a chat and reports that it is saved",
        "Left chat 1. It is saved with 2 messages." in first_output,
    )
    check(
        "history: Ctrl+C in the menu does not close the shell",
        "Chat 2: Named chat" in first_output,
    )
    check(
        "history: a chat is named after its first message or as asked",
        re.search(r"^\s+1\s.*first question$", first_output, re.M)
        and re.search(r"^\s+2\s.*Named chat$", first_output, re.M),
    )
    check(
        "history: the chat file records who said what, when, with what",
        saved["id"] == 1
        and saved["provider"] == "anthropic"
        and saved["model"] == "claude-opus-5-5"
        and [entry["role"] for entry in saved["transcript"]]
        == ["user", "assistant"]
        and saved["transcript"][0]["text"] == "first question"
        and saved["transcript"][1]["text"] == "reply 1 to: first question"
        and saved["transcript"][1]["model"] == "claude-opus-5-5"
        and saved["transcript"][1]["output_tokens"] == 5
        and re.fullmatch(
            r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", saved["transcript"][0]["time"]
        ),
    )
    check(
        "history: chat files are readable only by their owner",
        file_mode == 0o600 and folder_mode == 0o700,
    )
    check(
        "history: an opened chat shows what was said",
        "you> first question" in second_output
        and "reply 1 to: first question" in second_output,
    )
    check(
        "history: an opened chat continues where it stopped",
        "reply 2 to: follow up" in second_output
        and "Left chat 1. It is saved with 4 messages." in second_output,
    )
    check(
        "history: deleting asks first and keeps the chat on 'n'",
        'Delete chat 2 "Named chat" with 2 messages?' in second_output
        and "Chat 2 was kept." in second_output,
    )
    check(
        "history: a confirmed delete removes the chat for good",
        "Chat 2 was deleted." in second_output
        and second_output.count("There is no chat with ID 2") == 2,
    )
    check(
        "history: a command without its ID is explained",
        "Command 'open' needs the ID of a chat" in second_output,
    )
    check(
        "history: the ID of a deleted chat is not used again",
        "Chat 3" in second_output and remaining == ["1.json", "3.json"],
    )
    check(
        "history: another model continues a chat from its text",
        "This chat was written by claude-opus-5-5. It continues with "
        "claude-sonnet-5-5" in third_output
        and "reply 3 to: after the switch" in third_output,
    )
    check(
        "history: the new model receives the earlier messages as text",
        requests[-1]["body"]["model"] == "claude-sonnet-5-5"
        and [m["content"] for m in requests[-1]["body"]["messages"][:4]]
        == [
            "first question",
            "reply 1 to: first question",
            "follow up",
            "reply 2 to: follow up",
        ],
    )


def test_old_configuration_file(directory, certificate, key):
    """A configuration file of an earlier version is moved."""
    keyring_log = os.path.join(directory, "keyring9.log")
    old_path = os.path.join(directory, "im_config", "ai_config.json")

    # Put the configuration where earlier versions kept it.
    write_settings(directory, "", "", model="claude-haiku-5-5")
    os.replace(settings_path(directory), old_path)

    with Background([FAKE_KEYRING, keyring_log]):
        output, _ = run_shell(directory, [b"status\n", b"exit\n"])

    check(
        "config: a file from the old location is moved and used",
        "Model:    claude-haiku-5-5" in output
        and os.path.exists(settings_path(directory))
        and not os.path.exists(old_path),
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
            test_gemini,
            test_gemini_wrong_key,
            test_configuration,
            test_chat_history,
            test_old_configuration_file,
        ):
            test(directory, certificate, key)

    finish()


if __name__ == "__main__":
    main()
