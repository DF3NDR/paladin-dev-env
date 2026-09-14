#!/usr/bin/env python3
"""SDK smoke test: the Python client generated from `crates/paladin-web/openapi.json`
(D-49, PLAT-FR-17) against a live `paladin-server` booted by `run.sh` on the
all-InMemory/SQLite profile.

Exercises: list assistants (`GET /v1/assistants`, expects a non-empty JSON `items`
array field) -> submit a run for a code-registered agent (`POST /v1/runs`, expects a
`run_id`) -> poll `GET /v1/runs/{run_id}` until a terminal status or 30s.

A TERMINAL status is not a PASS. Only `SUCCESS_STATUS` ("completed") is success
(PLAT-06 `precision`) -- the comparison is exact and case-sensitive, never a prefix,
substring or truthiness check. Any other terminal status (`failed`, `halted`,
`cancelled`) is a failure that names the status and prints the run's own `error`
text; a run still non-terminal at the poll deadline is a failure that names the
last observed status. Exits non-zero on ANY deviation (a raised API exception, a
missing `items`/`run_id` field, or a run that did not reach `completed`) -- this
script's own exit code is `sdk-clients`'s real Python-side gate (prohibition P1:
the job must not be able to go green on a client that failed to install, list
nothing, submitted nothing, or observed a run that did not actually work).

`--self-test` exercises the success-decision function directly, with no server
and no generated client installed.
"""

from __future__ import annotations

import os
import sys
import time
from typing import Any

# The generated client is installed by `run.sh` via `pip install target/sdk/python`
# before this script runs -- see this file's own module docs. Guarded so
# `--self-test` (which exercises only the pure decision function below) can run
# with no server, and no generated client, installed at all.
try:
    import openapi_client
    from openapi_client.rest import ApiException
except ImportError:  # pragma: no cover -- exercised only by --self-test
    openapi_client = None  # type: ignore[assignment]
    ApiException = Exception  # type: ignore[assignment,misc]

BASE_URL = os.environ.get("PALADIN_SMOKE_BASE_URL", "http://127.0.0.1:18080")
API_KEY = os.environ.get("PALADIN_SMOKE_API_KEY", "sdk-smoke-test-key")
AGENT_ID = os.environ.get("PALADIN_SMOKE_AGENT_ID", "sdk-smoke-agent")
POLL_TIMEOUT_SECS = 30

# The single success status string (Task 3, PLAT-06 `precision`). A terminal
# status is not a pass on its own -- only this exact, case-sensitive value is.
SUCCESS_STATUS = "completed"

# Every status that stops polling -- decides WHEN polling ends, never WHETHER
# the run succeeded. Success/failure is decided exclusively by
# `evaluate_terminal_status` against `SUCCESS_STATUS`.
TERMINAL_STATUSES = {"completed", "failed", "halted", "cancelled"}


def fail(message: str) -> None:
    print(f"SMOKE FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def evaluate_terminal_status(status: Any, error: Any) -> None:
    """The success decision, extracted into a pure function so `--self-test` can
    exercise it directly with no server involved.

    A terminal status of exactly `SUCCESS_STATUS` is success (returns normally).
    Any other terminal status -- or a non-terminal/unknown status observed at the
    poll deadline -- is a failure that names both the observed status and the
    run's own `error` text, via `fail` (exits non-zero).
    """
    if status == SUCCESS_STATUS:
        return
    fail(
        f"run did not reach '{SUCCESS_STATUS}' -- observed status {status!r}, "
        f"error: {error!r}"
    )


def _get(obj: Any, field: str) -> Any:
    """Read `field` off a generated-model instance (attribute access) or a plain
    dict (subscript access) -- see this file's own module docs for why both shapes
    are supported."""
    if isinstance(obj, dict):
        return obj.get(field)
    return getattr(obj, field, None)


def build_api_client() -> "openapi_client.ApiClient":
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


def list_assistants(client: "openapi_client.ApiClient") -> None:
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


def submit_run(client: "openapi_client.ApiClient") -> str:
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


def poll_until_terminal(client: "openapi_client.ApiClient", run_id: str) -> None:
    runs_api = openapi_client.RunsApi(client)
    deadline = time.monotonic() + POLL_TIMEOUT_SECS
    last_status = None
    last_error = None
    while time.monotonic() < deadline:
        try:
            response = runs_api.get_run(run_id=run_id)
        except ApiException as e:
            fail(f"GET /v1/runs/{run_id} raised {e}")
            return
        last_status = _get(response, "status")
        last_error = _get(response, "error")
        if last_status in TERMINAL_STATUSES:
            print(f"poll run: terminal status '{last_status}' reached")
            evaluate_terminal_status(last_status, last_error)
            return
        time.sleep(1)
    # Deadline reached with no terminal status observed -- PLAT-06 `boundary`:
    # the deadline itself is a failure, never a silent pass.
    evaluate_terminal_status(last_status, last_error)


def _self_test() -> None:
    """Exercises `evaluate_terminal_status` directly -- no server, no generated
    client. `SUCCESS_STATUS` passes; every other terminal status, and an
    unknown/`None` status, exits non-zero via `fail`."""
    cases_run = 0

    try:
        evaluate_terminal_status(SUCCESS_STATUS, None)
    except SystemExit:
        print(f"self-test FAIL: '{SUCCESS_STATUS}' must not raise")
        sys.exit(1)
    cases_run += 1

    for bad_status in ("failed", "halted", "cancelled", None, "unknown"):
        try:
            evaluate_terminal_status(bad_status, "boom")
        except SystemExit as e:
            if e.code in (0, None):
                print(f"self-test FAIL: status {bad_status!r} must exit non-zero, exited {e.code!r}")
                sys.exit(1)
            cases_run += 1
            continue
        print(f"self-test FAIL: status {bad_status!r} must have failed but did not raise")
        sys.exit(1)

    print(f"self-test: ok ({cases_run}/6 cases)")


def main() -> None:
    if "--self-test" in sys.argv:
        _self_test()
        return

    if openapi_client is None:
        fail("openapi_client is not installed -- run.sh must pip install target/sdk/python first")
        return

    client = build_api_client()
    list_assistants(client)
    run_id = submit_run(client)
    poll_until_terminal(client, run_id)
    print("SDK smoke (Python): all checks passed.")


if __name__ == "__main__":
    main()
