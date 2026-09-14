#!/usr/bin/env python3
"""Loopback, OpenAI-compatible chat-completions stub for the `sdk-clients` smoke
boot (D-49, PLAT-FR-17).

Standard-library only -- the CI runner installs nothing for it. Serves the
exact response shape `crates/paladin-llm/src/openai/adapter.rs` deserialises
(`OpenAIResponse`/`OpenAIChoice`/`OpenAIUsage`, mirrored from that file's own
mockito tests): `POST {prefix}/chat/completions` answers with `id`, `model`
(echoing the request's `model`, defaulting to `gpt-4`), a single `choices`
entry (`index`, `message.role`, `message.content`, `finish_reason`), and a
`usage` object (`prompt_tokens`, `completion_tokens`, `total_tokens`).
`GET {prefix}/models` answers with a one-entry `data` list (the shape
`get_available_models` reads: `data[].id`). Anything else is a 404.

Binds `127.0.0.1` only -- this stub exists so the smoke boot needs no network
egress and no real credential (T-27-21-01): the whole point is that a run
submitted against it can reach `completed` locally, on a fork's PR, with
nothing more than a present-but-fake `OPENAI_API_KEY`.

Usage:
    python3 mock-llm.py [--port 18081] [--prefix /v1]
    python3 mock-llm.py --self-test
"""

from __future__ import annotations

import argparse
import json
import sys
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
from typing import Any


def _chat_completion_body(request_body: dict[str, Any]) -> dict[str, Any]:
    """Build the exact response shape `OpenAIResponse` deserialises."""
    model = request_body.get("model") or "gpt-4"
    return {
        "id": "cmpl-sdk-smoke-mock",
        "model": model,
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "ok",
                },
                "finish_reason": "stop",
            }
        ],
        "usage": {
            "prompt_tokens": 1,
            "completion_tokens": 1,
            "total_tokens": 2,
        },
    }


def _models_body() -> dict[str, Any]:
    return {
        "object": "list",
        "data": [
            {"id": "gpt-4", "object": "model"},
        ],
    }


def make_handler(prefix: str) -> type[BaseHTTPRequestHandler]:
    chat_path = f"{prefix}/chat/completions"
    models_path = f"{prefix}/models"

    class MockLlmHandler(BaseHTTPRequestHandler):
        # Quiet enough not to drown the CI log -- one line per request, no
        # traceback noise, no `BaseHTTPRequestHandler` default access-log format.
        def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
            sys.stderr.write(f"mock-llm: {self.command} {self.path} -> handled\n")

        def _write_json(self, status: int, payload: dict[str, Any]) -> None:
            body = json.dumps(payload).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self) -> None:  # noqa: N802
            if self.path != chat_path:
                self._write_json(404, {"error": f"not found: {self.path}"})
                return
            length = int(self.headers.get("Content-Length", "0") or "0")
            raw = self.rfile.read(length) if length else b"{}"
            try:
                request_body = json.loads(raw or b"{}")
            except json.JSONDecodeError:
                request_body = {}
            self._write_json(200, _chat_completion_body(request_body))

        def do_GET(self) -> None:  # noqa: N802
            if self.path != models_path:
                self._write_json(404, {"error": f"not found: {self.path}"})
                return
            self._write_json(200, _models_body())

    return MockLlmHandler


def run_server(port: int, prefix: str) -> HTTPServer:
    handler_cls = make_handler(prefix)
    server = HTTPServer(("127.0.0.1", port), handler_cls)
    return server


def serve(port: int, prefix: str) -> None:
    server = run_server(port, prefix)
    actual_port = server.server_address[1]
    print(f"mock-llm: listening on http://127.0.0.1:{actual_port}{prefix}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


def _self_test() -> None:
    """Start the handler on an ephemeral loopback port in a background thread,
    post one chat-completion request, validate every field the adapter
    deserialises, and check the `/models` route -- prints `ok` and exits 0 on
    success, or a named failure reason and exits non-zero otherwise."""
    prefix = "/v1"
    server = run_server(0, prefix)
    actual_port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        # Give the background thread a moment to start accepting connections.
        time.sleep(0.2)

        chat_url = f"http://127.0.0.1:{actual_port}{prefix}/chat/completions"
        req_body = json.dumps({"model": "gpt-4o", "messages": []}).encode("utf-8")
        req = urllib.request.Request(
            chat_url,
            data=req_body,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=5) as resp:
                if resp.status != 200:
                    print(f"self-test FAIL: chat/completions returned status {resp.status}")
                    sys.exit(1)
                body = json.loads(resp.read())
        except urllib.error.URLError as e:
            print(f"self-test FAIL: chat/completions request raised {e}")
            sys.exit(1)

        for field in ("id", "model", "choices", "usage"):
            if field not in body:
                print(f"self-test FAIL: response missing required field '{field}': {body}")
                sys.exit(1)
        if not isinstance(body["choices"], list) or not body["choices"]:
            print(f"self-test FAIL: 'choices' is not a non-empty list: {body}")
            sys.exit(1)
        choice = body["choices"][0]
        for field in ("index", "message", "finish_reason"):
            if field not in choice:
                print(f"self-test FAIL: choice missing required field '{field}': {choice}")
                sys.exit(1)
        message = choice["message"]
        for field in ("role", "content"):
            if field not in message or not isinstance(message[field], str):
                print(f"self-test FAIL: message missing/invalid field '{field}': {message}")
                sys.exit(1)
        usage = body["usage"]
        for field in ("prompt_tokens", "completion_tokens", "total_tokens"):
            if field not in usage or not isinstance(usage[field], int):
                print(f"self-test FAIL: usage missing/invalid field '{field}': {usage}")
                sys.exit(1)
        if body["model"] != "gpt-4o":
            print(f"self-test FAIL: model was not echoed back: got {body['model']!r}")
            sys.exit(1)

        models_url = f"http://127.0.0.1:{actual_port}{prefix}/models"
        try:
            with urllib.request.urlopen(models_url, timeout=5) as resp:
                if resp.status != 200:
                    print(f"self-test FAIL: /models returned status {resp.status}")
                    sys.exit(1)
                models_body = json.loads(resp.read())
        except urllib.error.URLError as e:
            print(f"self-test FAIL: /models request raised {e}")
            sys.exit(1)
        data = models_body.get("data")
        if not isinstance(data, list) or not data or "id" not in data[0]:
            print(f"self-test FAIL: /models response malformed: {models_body}")
            sys.exit(1)

        not_found_url = f"http://127.0.0.1:{actual_port}{prefix}/nonexistent"
        try:
            urllib.request.urlopen(not_found_url, timeout=5)
            print("self-test FAIL: unknown path did not 404")
            sys.exit(1)
        except urllib.error.HTTPError as e:
            if e.code != 404:
                print(f"self-test FAIL: unknown path returned {e.code}, expected 404")
                sys.exit(1)

        print("self-test: ok")
    finally:
        server.shutdown()
        thread.join(timeout=5)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=18081)
    parser.add_argument("--prefix", type=str, default="/v1")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        _self_test()
        return

    serve(args.port, args.prefix)


if __name__ == "__main__":
    main()
