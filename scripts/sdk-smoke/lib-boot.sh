#!/usr/bin/env bash
# scripts/sdk-smoke/lib-boot.sh
#
# The single shared boot/teardown definition for the sdk-smoke scripts
# (`run.sh`, `smoke-http.sh`) -- D-49, PLAT-FR-17. Source-only: this file
# intentionally carries no `set -e` of its own, so a sourced library never
# silently changes a caller's error-handling mode -- each caller owns
# `set -euo pipefail`.
#
# Provides:
#   smoke_boot_start -- start the loopback mock LLM stub (mock-llm.py), wait
#                        for it to answer, locate and start `paladin-server`
#                        on the all-InMemory/SQLite profile, wait for
#                        `/health`.
#   smoke_boot_stop   -- stop both processes and remove the temp databases.
#
# Exposes after sourcing: BASE_URL, API_KEY, AGENT_ID (the paladin-server
# smoke profile's fixed values). After `smoke_boot_start`: both the stub and
# the server are running and healthy.

LIB_BOOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$LIB_BOOT_DIR/../.." && pwd)"

PORT=18080
BASE_URL="http://127.0.0.1:${PORT}"
# Consumed by callers after sourcing (run.sh, smoke-http.sh), not within this
# file -- exported so shellcheck (and any subshell) sees the intended use.
export API_KEY="sdk-smoke-test-key"
export AGENT_ID="sdk-smoke-agent"

# The loopback mock LLM stub this boot starts itself (D-49, PLAT-FR-17) --
# never a real third-party endpoint. See mock-llm.py's own module docs.
MOCK_LLM_PORT="${PALADIN_SMOKE_MOCK_LLM_PORT:-18081}"
MOCK_LLM_BASE_URL="http://127.0.0.1:${MOCK_LLM_PORT}/v1"

RUN_DB="$(mktemp -u "${TMPDIR:-/tmp}/paladin-sdk-smoke-run-XXXXXX").sqlite"
WAYPOINT_DB="$(mktemp -u "${TMPDIR:-/tmp}/paladin-sdk-smoke-waypoints-XXXXXX").sqlite"

SERVER_PID=""
MOCK_LLM_PID=""

smoke_boot_start() {
  cd "$ROOT_DIR" || return 1

  echo "Starting loopback mock LLM stub on ${MOCK_LLM_BASE_URL} ..."
  python3 "$LIB_BOOT_DIR/mock-llm.py" --port "$MOCK_LLM_PORT" &
  MOCK_LLM_PID=$!

  local mock_ready=0
  local _i
  for _i in $(seq 1 30); do
    if curl -fsS "http://127.0.0.1:${MOCK_LLM_PORT}/v1/models" >/dev/null 2>&1; then
      mock_ready=1
      break
    fi
    sleep 1
  done
  if [ "$mock_ready" -ne 1 ]; then
    echo "::error::mock LLM stub never became ready within 30s" >&2
    return 1
  fi
  echo "mock LLM stub is up."

  export PALADIN_CONFIG="scripts/sdk-smoke/smoke-config.yml"
  # Present-but-dummy, per `run_api_wiring.rs`'s own
  # `sqlite_and_in_memory_wires_three_tasks_and_every_state_field` test
  # precedent: construction only checks the key is PRESENT, never that it is
  # valid.
  export OPENAI_API_KEY="${OPENAI_API_KEY:-sk-sdk-smoke-hermetic}"
  # Hermetic loopback stub (D-49, PLAT-FR-17): the adapter's request path is
  # exactly {base_url}/chat/completions -- no network egress, no real
  # credential needed or accepted.
  export OPENAI_BASE_URL="$MOCK_LLM_BASE_URL"
  export APP_RUN_STORE_BACKEND=sqlite
  export APP_RUN_STORE_PATH="sqlite://${RUN_DB}"
  export APP_RUN_QUEUE_BACKEND=in_memory
  export APP_WAYPOINT_STORE_BACKEND=sqlite
  export APP_WAYPOINT_STORE_SQLITE_PATH="sqlite://${WAYPOINT_DB}"

  local bin="target/release/paladin-server"
  if [ ! -x "$bin" ]; then
    bin="target/debug/paladin-server"
  fi
  if [ ! -x "$bin" ]; then
    echo "::error::paladin-server binary not found -- run 'cargo build --bin paladin-server --features web-server' first" >&2
    return 1
  fi

  echo "Starting $bin on $BASE_URL ..."
  "$bin" &
  SERVER_PID=$!

  echo "Waiting for $BASE_URL/health ..."
  local ready=0
  local _j
  for _j in $(seq 1 60); do
    if curl -fsS "${BASE_URL}/health" >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
  done
  if [ "$ready" -ne 1 ]; then
    echo "::error::paladin-server never became healthy within 60s" >&2
    return 1
  fi
  echo "paladin-server is up."
}

smoke_boot_stop() {
  if [ -n "$SERVER_PID" ]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if [ -n "$MOCK_LLM_PID" ]; then
    kill "$MOCK_LLM_PID" >/dev/null 2>&1 || true
    wait "$MOCK_LLM_PID" 2>/dev/null || true
  fi
  rm -f "$RUN_DB" "${RUN_DB}-wal" "${RUN_DB}-shm" \
        "$WAYPOINT_DB" "${WAYPOINT_DB}-wal" "${WAYPOINT_DB}-shm"
}
