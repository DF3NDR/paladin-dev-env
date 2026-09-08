#!/usr/bin/env bash
# Boots `paladin-server` on a free port with a static test API key and the
# all-InMemory/SQLite profile (D-49, PLAT-FR-17): no Docker, no Redis, no
# Postgres -- every store this smoke boot wires is either a temp-file SQLite
# database or the in-memory run queue. Waits for `/health`, runs the Python and
# TypeScript generated-client smokes against it, then stops the server on exit
# regardless of outcome.
#
# Usage (from the repo root, after `cargo build --bin paladin-server --features
# web-server` and generating both clients under `target/sdk/{python,typescript}`):
#   scripts/sdk-smoke/run.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

PORT=18080
BASE_URL="http://127.0.0.1:${PORT}"
API_KEY="sdk-smoke-test-key"
AGENT_ID="sdk-smoke-agent"

RUN_DB="$(mktemp -u "${TMPDIR:-/tmp}/paladin-sdk-smoke-run-XXXXXX").sqlite"
WAYPOINT_DB="$(mktemp -u "${TMPDIR:-/tmp}/paladin-sdk-smoke-waypoints-XXXXXX").sqlite"

export PALADIN_CONFIG="scripts/sdk-smoke/smoke-config.yml"
# Present-but-dummy, per `run_api_wiring.rs`'s own
# `sqlite_and_in_memory_wires_three_tasks_and_every_state_field` test precedent:
# construction only checks the key is PRESENT, never that it is valid.
export OPENAI_API_KEY="${OPENAI_API_KEY:-sk-sdk-smoke-hermetic}"
export APP_RUN_STORE_BACKEND=sqlite
export APP_RUN_STORE_PATH="sqlite://${RUN_DB}"
export APP_RUN_QUEUE_BACKEND=in_memory
export APP_WAYPOINT_STORE_BACKEND=sqlite
export APP_WAYPOINT_STORE_SQLITE_PATH="sqlite://${WAYPOINT_DB}"

BIN="target/release/paladin-server"
if [ ! -x "$BIN" ]; then
  BIN="target/debug/paladin-server"
fi
if [ ! -x "$BIN" ]; then
  echo "::error::paladin-server binary not found -- run 'cargo build --bin paladin-server --features web-server' first" >&2
  exit 1
fi

echo "Starting $BIN on $BASE_URL ..."
"$BIN" &
SERVER_PID=$!

cleanup() {
  local status=$?
  kill "$SERVER_PID" >/dev/null 2>&1 || true
  wait "$SERVER_PID" 2>/dev/null || true
  rm -f "$RUN_DB" "${RUN_DB}-wal" "${RUN_DB}-shm" \
        "$WAYPOINT_DB" "${WAYPOINT_DB}-wal" "${WAYPOINT_DB}-shm"
  exit "$status"
}
trap cleanup EXIT

echo "Waiting for $BASE_URL/health ..."
ready=0
for _ in $(seq 1 60); do
  if curl -fsS "${BASE_URL}/health" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [ "$ready" -ne 1 ]; then
  echo "::error::paladin-server never became healthy within 60s" >&2
  exit 1
fi
echo "paladin-server is up."

echo "--- Python generated-client smoke ---"
if [ ! -d target/sdk/python ]; then
  echo "::error::target/sdk/python is missing -- generate the Python client first" >&2
  exit 1
fi
python3 -m pip install --quiet target/sdk/python
PALADIN_SMOKE_BASE_URL="$BASE_URL" \
PALADIN_SMOKE_API_KEY="$API_KEY" \
PALADIN_SMOKE_AGENT_ID="$AGENT_ID" \
  python3 scripts/sdk-smoke/smoke.py

echo "--- TypeScript generated-client smoke ---"
if [ ! -d target/sdk/typescript ]; then
  echo "::error::target/sdk/typescript is missing -- generate the TypeScript client first" >&2
  exit 1
fi
(
  cd scripts/sdk-smoke
  npm ci
  npx tsc -p tsconfig.json
  PALADIN_SMOKE_BASE_URL="$BASE_URL" \
  PALADIN_SMOKE_API_KEY="$API_KEY" \
  PALADIN_SMOKE_AGENT_ID="$AGENT_ID" \
    node dist/smoke.js
)

echo "SDK smoke: both clients completed successfully."
