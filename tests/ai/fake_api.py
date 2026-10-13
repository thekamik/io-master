#!/usr/bin/env python3
"""
Stand-in for the AI services, used by the AI shell tests.

The real services need an API key, may cost money and cannot be
made to fail on demand. This script is a small HTTPS server that
answers the way the real APIs do. It plays two services at once,
told apart by the address of the request:

    POST /v1/messages                              Anthropic
    POST /v1beta/models/<model>:generateContent    Gemini

Anthropic
---------
The behaviour is selected through the text of the message:

    message of the user      answer of the server
    -------------------      --------------------
    refuse                   200, the model declines
    overloaded once          529 for the first try, then a reply
    always overloaded        529 for every try
    bad request              400 invalid_request_error
    slow                     a reply after SLOW_SECONDS seconds
    long                     a reply cut off by max_tokens
    fallback                 a reply written by another model
    escape                   a reply with terminal control codes
    anything else            "reply N to: <message>", where N is
                             the number of user messages so far

Requests with an API key other than VALID_KEY get 401, and
requests for the model "claude-typo" get 404.

Every reply contains a "thinking" block with a signature. The real
API requires such blocks to be sent back unchanged with the next
request; the server checks that and answers 400 when a block is
missing, so a test notices when io-master forgets the history.

Gemini
------
    message of the user      answer of the server
    -------------------      --------------------
    refuse                   200 without candidates, blocked
    unsafe                   200, a candidate stopped for SAFETY
    quota                    429 RESOURCE_EXHAUSTED
    long                     a reply cut off by MAX_TOKENS
    anything else            "gemini reply N to: <message>"

The key is expected in the "x-goog-api-key" header; a wrong key
gets the 400 "API_KEY_INVALID" answer of the real service. Every
reply carries a thought part, which must not be shown, and
a "thoughtSignature", which must come back with the next request.

Log
---
Every request that reaches the server is written to a log file,
one JSON object per line, so the tests can check what was sent.
The API key itself is not logged, only whether it was valid.

Usage:
    fake_api.py LOG_FILE CERT_FILE KEY_FILE TLS_VERSION

TLS_VERSION is "1.3" for a server like the real one, or "1.2" for
a server that offers nothing newer than TLS 1.2, which io-master
must refuse to talk to. The server listens on a free port of
127.0.0.1, prints that port and then runs until it is terminated.
"""

import json
import re
import ssl
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# The only API key that the server accepts.
VALID_KEY = "sk-test-valid-key"

# Delay of the "slow" answer. Long enough to cancel the request.
SLOW_SECONDS = 15

# Model name that the Anthropic side answers with 404, the way
# the real service answers a model it does not know.
UNKNOWN_MODEL = "claude-typo"

# Model named in the reply of the "fallback" behaviour.
FALLBACK_MODEL = "claude-opus-4-8"

# Address of the Gemini method; the group is the model name.
GEMINI_PATH = re.compile(r"^/v1beta/models/([^/:]+):generateContent$")

# Number of tries of an "overloaded once" message seen so far.
overloaded_tries = 0
counter_lock = threading.Lock()


def error_body(error_type, message):
    """Body of an error answer, in the format of the real API."""
    return {"type": "error", "error": {"type": error_type, "message": message}}


def reply_body(model, text, user_turns, stop_reason="end_turn"):
    """Body of a successful answer."""
    return {
        "id": "msg_test",
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": [
            # Must come back unchanged with the next request.
            {
                "type": "thinking",
                "thinking": "",
                "signature": "signature-%d" % user_turns,
            },
            {"type": "text", "text": text},
        ],
        "stop_reason": stop_reason,
        "usage": {"input_tokens": 10 * user_turns, "output_tokens": 5},
    }


def history_is_intact(messages):
    """
    Checks that every earlier reply was sent back as it was given:
    the n-th assistant message must still start with the thinking
    block that carries the n-th signature.
    """
    assistant_turns = 0

    for message in messages:
        if message.get("role") != "assistant":
            continue

        assistant_turns += 1
        content = message.get("content")

        # A reply given as plain text: the chat was written by
        # another model and is being continued from its
        # transcript, which carries no thinking blocks.
        if isinstance(content, str):
            continue

        if (
            not isinstance(content, list)
            or not content
            or content[0].get("type") != "thinking"
            or content[0].get("signature")
            != "signature-%d" % assistant_turns
        ):
            return False

    return True


class QuietServer(ThreadingHTTPServer):
    """
    A server that does not print a traceback when a client goes
    away. In these tests clients do that on purpose: io-master
    cancels requests and refuses servers it does not trust.
    """

    def handle_error(self, request, client_address):
        pass


class Handler(BaseHTTPRequestHandler):
    """Answers one request."""

    # Set by main().
    log_file = None

    def log_message(self, *_arguments):
        # Keep the output of the test clean.
        pass

    def send_json(self, status, body):
        data = json.dumps(body).encode()

        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()

        try:
            self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError):
            # The client cancelled the request.
            pass

    def do_POST(self):
        global overloaded_tries

        length = int(self.headers.get("content-length", "0"))
        raw_body = self.rfile.read(length)

        try:
            body = json.loads(raw_body)
        except ValueError:
            body = None

        # Which service the request is meant for.
        gemini = GEMINI_PATH.match(self.path)

        key_header = "x-goog-api-key" if gemini else "x-api-key"
        key_is_valid = self.headers.get(key_header) == VALID_KEY

        # What the tests want to know about the request.
        self.log_file.write(
            json.dumps(
                {
                    "path": self.path,
                    "service": "gemini" if gemini else "anthropic",
                    "key_is_valid": key_is_valid,
                    "anthropic-version": self.headers.get("anthropic-version"),
                    "anthropic-beta": self.headers.get("anthropic-beta"),
                    "tls": self.connection.version(),
                    "body": body,
                }
            )
            + "\n"
        )

        if gemini:
            self.answer_gemini(gemini.group(1), key_is_valid, body)
            return

        if self.path != "/v1/messages":
            self.send_json(404, error_body("not_found_error", "Not found"))
            return

        if not key_is_valid:
            self.send_json(
                401, error_body("authentication_error", "invalid x-api-key")
            )
            return

        if not isinstance(body, dict) or not body.get("messages"):
            self.send_json(
                400, error_body("invalid_request_error", "messages: required")
            )
            return

        messages = body["messages"]
        model = body.get("model", "")
        text = messages[-1].get("content", "")
        user_turns = sum(1 for m in messages if m.get("role") == "user")

        if not history_is_intact(messages):
            self.send_json(
                400,
                error_body(
                    "invalid_request_error",
                    "earlier replies were not sent back unchanged",
                ),
            )
            return

        # A model that does not exist, as after a typing mistake.
        if model == UNKNOWN_MODEL:
            self.send_json(
                404, error_body("not_found_error", "model: " + model)
            )
            return

        if text == "refuse":
            self.send_json(
                200,
                {
                    "id": "msg_test",
                    "type": "message",
                    "role": "assistant",
                    "model": model,
                    "content": [],
                    "stop_reason": "refusal",
                    "stop_details": {"type": "refusal", "category": "cyber"},
                    "usage": {"input_tokens": 10, "output_tokens": 0},
                },
            )
        elif text == "overloaded once":
            with counter_lock:
                overloaded_tries += 1
                first_try = overloaded_tries == 1

            if first_try:
                self.send_json(
                    529, error_body("overloaded_error", "Overloaded")
                )
            else:
                self.send_json(
                    200, reply_body(model, "recovered", user_turns)
                )
        elif text == "always overloaded":
            self.send_json(529, error_body("overloaded_error", "Overloaded"))
        elif text == "bad request":
            self.send_json(
                400,
                error_body("invalid_request_error", "max_tokens: too large"),
            )
        elif text == "slow":
            time.sleep(SLOW_SECONDS)
            self.send_json(
                200, reply_body(model, "slow reply arrived", user_turns)
            )
        elif text == "long":
            self.send_json(
                200,
                reply_body(model, "cut off here", user_turns, "max_tokens"),
            )
        elif text == "fallback":
            self.send_json(
                200, reply_body(FALLBACK_MODEL, "from a fallback", user_turns)
            )
        elif text == "escape":
            # A window title change and a colour change, which
            # io-master must not pass on to the terminal.
            self.send_json(
                200,
                reply_body(
                    model, "\x1b]0;evil\x07plain \x1b[31mtext", user_turns
                ),
            )
        else:
            self.send_json(
                200,
                reply_body(
                    model, "reply %d to: %s" % (user_turns, text), user_turns
                ),
            )


def gemini_error(code, status, message, reason=None):
    """Body of a Gemini error answer, in the format of the real API."""
    error = {"code": code, "message": message, "status": status}

    if reason is not None:
        error["details"] = [
            {
                "@type": "type.googleapis.com/google.rpc.ErrorInfo",
                "reason": reason,
            }
        ]

    return {"error": error}


def gemini_reply(model, text, user_turns, finish_reason="STOP"):
    """Body of a successful Gemini answer."""
    return {
        "candidates": [
            {
                "content": {
                    "role": "model",
                    "parts": [
                        # A note of the model: must not be shown.
                        {"text": "SECRET THOUGHT", "thought": True},
                        # Must come back unchanged with the
                        # next request.
                        {
                            "text": text,
                            "thoughtSignature": "signature-%d" % user_turns,
                        },
                    ],
                },
                "finishReason": finish_reason,
            }
        ],
        "usageMetadata": {
            "promptTokenCount": 10 * user_turns,
            "candidatesTokenCount": 5,
            "thoughtsTokenCount": 2,
        },
        "modelVersion": model,
    }


def gemini_history_is_intact(contents):
    """
    Checks that every earlier reply was sent back as it was given:
    the last part of the n-th model message must still carry the
    n-th signature.
    """
    model_turns = 0

    for message in contents:
        if message.get("role") != "model":
            continue

        model_turns += 1
        parts = message.get("parts")

        if (
            not isinstance(parts, list)
            or not parts
            or parts[-1].get("thoughtSignature")
            != "signature-%d" % model_turns
        ):
            return False

    return True


def answer_gemini(self, model, key_is_valid, body):
    """Answers a request to the Gemini generateContent method."""
    if not key_is_valid:
        self.send_json(
            400,
            gemini_error(
                400,
                "INVALID_ARGUMENT",
                "API key not valid. Please pass a valid API key.",
                "API_KEY_INVALID",
            ),
        )
        return

    if not isinstance(body, dict) or not body.get("contents"):
        self.send_json(
            400, gemini_error(400, "INVALID_ARGUMENT", "contents: required")
        )
        return

    contents = body["contents"]
    text = contents[-1]["parts"][0].get("text", "")
    user_turns = sum(1 for m in contents if m.get("role") == "user")

    if not gemini_history_is_intact(contents):
        self.send_json(
            400,
            gemini_error(
                400,
                "INVALID_ARGUMENT",
                "earlier replies were not sent back unchanged",
            ),
        )
        return

    if text == "refuse":
        # The request itself is blocked: no candidates at all.
        self.send_json(200, {"promptFeedback": {"blockReason": "SAFETY"}})
    elif text == "unsafe":
        # The model was stopped before it wrote anything.
        self.send_json(
            200,
            {
                "candidates": [
                    {"content": {"role": "model"}, "finishReason": "SAFETY"}
                ]
            },
        )
    elif text == "quota":
        self.send_json(
            429,
            gemini_error(429, "RESOURCE_EXHAUSTED", "Quota exceeded."),
        )
    elif text == "long":
        self.send_json(
            200, gemini_reply(model, "cut off here", user_turns, "MAX_TOKENS")
        )
    else:
        self.send_json(
            200,
            gemini_reply(
                model,
                "gemini reply %d to: %s" % (user_turns, text),
                user_turns,
            ),
        )


# The function above is written at module level for readability
# and attached to the handler here.
Handler.answer_gemini = answer_gemini


def main():
    if len(sys.argv) != 5 or sys.argv[4] not in ("1.2", "1.3"):
        sys.exit("usage: fake_api.py LOG_FILE CERT_FILE KEY_FILE 1.2|1.3")

    log_path, cert_file, key_file, tls_version = sys.argv[1:]

    # Line-buffered, so the tests can read the log while the
    # server is still running.
    Handler.log_file = open(log_path, "w", buffering=1)

    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert_file, key_file)

    if tls_version == "1.3":
        context.minimum_version = ssl.TLSVersion.TLSv1_3
    else:
        # An outdated server: nothing newer than TLS 1.2.
        context.maximum_version = ssl.TLSVersion.TLSv1_2

    # Port 0 lets the system pick a free port.
    server = QuietServer(("127.0.0.1", 0), Handler)
    server.socket = context.wrap_socket(server.socket, server_side=True)

    # Tell the test where the server listens.
    print(server.server_address[1], flush=True)

    server.serve_forever()


if __name__ == "__main__":
    main()
