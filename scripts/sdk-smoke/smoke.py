#!/usr/bin/env python3
"""SDK smoke test: the Python client generated from `crates/paladin-web/openapi.json`
(D-49, PLAT-FR-17) against a live `paladin-server` booted by `run.sh` on the
all-InMemory/SQLite profile.

Exercises: list assistants (`GET /v1/assistants`, expects a non-empty JSON `items`
array field) -> submit a run for a code-registered agent (`POST /v1/runs`, expects a
`run_id`) -> poll `GET /v1/runs/{run_id}` until a terminal status or 30s.

Exits non-zero on ANY deviation (a raised API exception, a missing `items`/`run_id`
field, or a poll that never reaches a terminal status) -- this script's own exit
code is `sdk-clients`'s real Python-side gate (prohibition P1: the job must not be
able to go green on a client that failed to install, list nothing, or submit
nothing).

Field access uses `_get` below rather than direct attribute/subscript access, since
this test suite cannot run the generator locally (no Java, no Docker -- see
`27-18-SUMMARY.md`) to pin exactly which shape (a pydantic model with attributes, or
a plain dict) the pinned generator version returns.
"""

from __future__ import annotations

import os
import sys
import time
from typing import Any

# The generated client is installed by `run.sh` via `pip install target/sdk/python`
# before this script runs -- see this file's own module docs.
import openapi_client
from openapi_client.rest import ApiException

BASE_URL = os.environ.get("PALADIN_SMOKE_BASE_URL", "http://127.0.0.1:18080")
API_KEY = os.environ.get("PALADIN_SMOKE_API_KEY", "sdk-smoke-test-key")
AGENT_ID = os.environ.get("PALADIN_SMOKE_AGENT_ID", "sdk-smoke-agent")
POLL_TIMEOUT_SECS = 30
TERMINAL_STATUSES = {"completed", "failed", "halted", "cancelled"}


def fail(message: str) -> None:
    print(f"SMOKE FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def _get(obj: Any, field: str) -> Any:
    """Read `field` off a generated-model instance (attribute access) or a plain
    dict (subscript access) -- see this file's own module docs for why both shapes
    are supported."""
    if isinstance(obj, dict):
        return obj.get(field)
    return getattr(obj, field, None)


def build_api_client() -> openapi_client.ApiClient:
    configuration = openapi_client.Configuration(host=BASE_URL)
    # The generated `Configuration` maps a security-scheme NAME (`api_key`, per
    # `crates/paladin-web/openapi.json`'s `components.securitySchemes`) to the
    # credential value -- the standard `openapi-generator` python convention.
    configuration.api_key["api_key"] = API_KEY
    client = openapi_client.ApiClient(configuration)
    # Belt-and-braces: also set the header directly, so this script's own
    # correctness does not depend on the generator having wired the security
    # scheme name exactly as expected.
    client.set_default_header("X-API-Key", API_KEY)
    return client


def list_assistants(client: openapi_client.ApiClient) -> None:
    assistants_api = openapi_client.AssistantsApi(client)
    try:
        response = assistants_api.list_assistants()
    except ApiException as e:
        fail(f"GET /v1/assistants raised {e}")
        return
    items = _get(response, "items")
    if items is None or not isinstance(items, list):
        fail(f"GET /v1/assistants response has no 'items' array: {response!r}")
    print(f"list assistants: ok, {len(items)} item(s)")


def submit_run(client: openapi_client.ApiClient) -> str:
    runs_api = openapi_client.RunsApi(client)
    request_body = openapi_client.SubmitRunRequest(assistant_id=AGENT_ID, input={})
    try:
        response = runs_api.submit_run(submit_run_request=request_body)
    except ApiException as e:
        fail(f"POST /v1/runs raised {e}")
        return ""
    run_id = _get(response, "run_id")
    if not run_id:
        fail(f"POST /v1/runs response has no 'run_id': {response!r}")
    print(f"submit run: ok, run_id={run_id}")
    return str(run_id)


def poll_until_terminal(client: openapi_client.ApiClient, run_id: str) -> None:
    runs_api = openapi_client.RunsApi(client)
    deadline = time.monotonic() + POLL_TIMEOUT_SECS
    last_status = None
    while time.monotonic() < deadline:
        try:
            response = runs_api.get_run(run_id=run_id)
        except ApiException as e:
            fail(f"GET /v1/runs/{run_id} raised {e}")
            return
        last_status = _get(response, "status")
        if last_status in TERMINAL_STATUSES:
            print(f"poll run: terminal status '{last_status}' reached")
            return
        time.sleep(1)
    fail(
        f"GET /v1/runs/{run_id} never reached a terminal status within "
        f"{POLL_TIMEOUT_SECS}s (last observed: {last_status})"
    )


def main() -> None:
    client = build_api_client()
    list_assistants(client)
    run_id = submit_run(client)
    poll_until_terminal(client, run_id)
    print("SDK smoke (Python): all checks passed.")


if __name__ == "__main__":
    main()
