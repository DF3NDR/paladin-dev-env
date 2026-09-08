#!/usr/bin/env bash
# Boots `paladin-server` (via the shared `lib-boot.sh` -- D-49, PLAT-FR-17) on
# the all-InMemory/SQLite profile against a hermetic loopback mock LLM stub:
# no Docker, no Redis, no Postgres, no network egress, no real credential.
# Waits for `/health`, runs the Python and TypeScript generated-client
# smokes against it, then stops both processes on exit regardless of outcome.
#
# Usage (from the repo root, after `cargo build --bin paladin-server --features
# web-server` and generating both clients under `target/sdk/{python,typescript}`):
#   scripts/sdk-smoke/run.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

# shellcheck source=scripts/sdk-smoke/lib-boot.sh
source "$SCRIPT_DIR/lib-boot.sh"

cleanup() {
  local status=$?
  smoke_boot_stop
  exit "$status"
}
trap cleanup EXIT

smoke_boot_start

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
  # `npm ci` empties `node_modules`, so the generated client -- a local build
  # artefact installed from its build directory, not a resolvable registry
  # dependency the committed lockfile could ever pin -- must be installed
  # AFTER it, never before, and without rewriting the manifest or lockfile
  # (Task 2).
  npm install --no-save ../../target/sdk/typescript
  npx tsc -p tsconfig.json
  PALADIN_SMOKE_BASE_URL="$BASE_URL" \
  PALADIN_SMOKE_API_KEY="$API_KEY" \
  PALADIN_SMOKE_AGENT_ID="$AGENT_ID" \
    node dist/smoke.js
)

echo "SDK smoke: both clients completed successfully."
