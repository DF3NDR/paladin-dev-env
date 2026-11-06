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
# The generated client is built in its OWN directory, where its own
# devDependencies (`typescript`) are installed, before it is linked into the
# smoke package. Linking an unbuilt directory made npm run the client's
# `prepare` script (`npm run build` -> `tsc`) with no `tsc` on the PATH, which
# exits 127 on the Ubuntu 26.04 `ubuntu-latest` image (58-CI-EVIDENCE.md,
# F-58-02). `npm install` with no argument installs the devDependencies and
# runs `prepare`, so the build happens here, once, where `tsc` resolves.
(
  cd target/sdk/typescript
  npm install
  main_file="$(node -p "require('./package.json').main || ''")"
  if [ -z "$main_file" ] || [ ! -f "$main_file" ]; then
    echo "::error::generated TypeScript client was not built -- package.json main '${main_file}' is missing under target/sdk/typescript" >&2
    exit 1
  fi
)
(
  cd scripts/sdk-smoke
  npm ci
  # `npm ci` empties `node_modules`, so the generated client -- a local build
  # artefact installed from its build directory, not a resolvable registry
  # dependency the committed lockfile could ever pin -- must be installed
  # AFTER it, never before, and without rewriting the manifest or lockfile
  # (Task 2). It is already built (above), so nothing needs to run at link
  # time and `--ignore-scripts` asks npm not to. npm versions differ on whether
  # a directory dependency's `prepare` honours that flag; if it does run, `tsc`
  # now resolves from the client's own node_modules/.bin, so it cannot fail the
  # way the unbuilt link did.
  npm install --no-save --ignore-scripts ../../target/sdk/typescript
  npx tsc -p tsconfig.json
  PALADIN_SMOKE_BASE_URL="$BASE_URL" \
  PALADIN_SMOKE_API_KEY="$API_KEY" \
  PALADIN_SMOKE_AGENT_ID="$AGENT_ID" \
    node dist/smoke.js
)

echo "SDK smoke: both clients completed successfully."
