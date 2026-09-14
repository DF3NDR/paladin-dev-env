#!/usr/bin/env bash
# scripts/sdk-smoke/smoke-http.sh
#
# Generator-free curl round trip proving a run submitted against the smoke
# boot (D-49, PLAT-FR-17) reaches `completed`, end to end, with no network
# egress and no credential. This script exists because the openapi generator
# needs Java/Docker the devcontainer lacks (see 27-18-SUMMARY.md), so it is
# the local proof that the *server side* of the smoke works, while `run.sh`
# remains the CI gate that additionally proves the generated Python and
# TypeScript clients.
#
# Success requires the run's terminal status to be exactly `completed` --
# `failed`, `halted` and `cancelled` are failures that print the run's own
# `error` text, and a run still non-terminal at the poll deadline is a
# failure that prints the last observed status. A terminal status is never
# treated as a pass on its own (PLAT-06 `precision`).
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

POLL_ATTEMPTS=30

echo "--- GET /v1/assistants ---"
assistants_response="$(curl -fsS -H "X-API-Key: ${API_KEY}" "${BASE_URL}/v1/assistants")"
items_type="$(printf '%s' "$assistants_response" | jq -r '.items | type')"
if [ "$items_type" != "array" ]; then
  echo "::error::GET /v1/assistants has no 'items' array: ${assistants_response}" >&2
  exit 1
fi
items_count="$(printf '%s' "$assistants_response" | jq '.items | length')"
if [ "$items_count" -lt 1 ]; then
  echo "::error::GET /v1/assistants returned an empty 'items' array -- an empty catalogue is not a successful round trip" >&2
  exit 1
fi
echo "assistants: ok, ${items_count} item(s)"

echo "--- POST /v1/runs ---"
submit_response="$(curl -fsS -X POST \
  -H "X-API-Key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d "{\"assistant_id\":\"${AGENT_ID}\",\"input\":{}}" \
  "${BASE_URL}/v1/runs")"
run_id="$(printf '%s' "$submit_response" | jq -r '.run_id // empty')"
if [ -z "$run_id" ]; then
  echo "::error::POST /v1/runs has no 'run_id': ${submit_response}" >&2
  exit 1
fi
echo "submit run: ok, run_id=${run_id}"

echo "--- poll GET /v1/runs/${run_id} ---"
status=""
error_text=""
attempt=0
while [ "$attempt" -lt "$POLL_ATTEMPTS" ]; do
  poll_response="$(curl -fsS -H "X-API-Key: ${API_KEY}" "${BASE_URL}/v1/runs/${run_id}")"
  status="$(printf '%s' "$poll_response" | jq -r '.status // empty')"
  error_text="$(printf '%s' "$poll_response" | jq -r '.error // empty')"
  case "$status" in
    completed | failed | halted | cancelled)
      break
      ;;
  esac
  attempt=$((attempt + 1))
  sleep 1
done

# Exact, case-sensitive equality against `completed` -- never a prefix,
# substring or truthiness check (PLAT-06 `precision`). Any other terminal
# status, or none observed by the deadline, is a failure that names the last
# observed status and the run's own error text.
if [ "$status" != "completed" ]; then
  echo "::error::run ${run_id} did not reach 'completed' -- last observed status '${status}', error: ${error_text}" >&2
  exit 1
fi

echo "poll run: terminal status 'completed' reached"
echo "SDK smoke (HTTP, generator-free): completed"
