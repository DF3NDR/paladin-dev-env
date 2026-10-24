#!/usr/bin/env bash
# Measure workspace line coverage against the ADR-0006 floor.
#
# Single source of truth for the coverage command: `make coverage` and CI's
# `coverage` job both call this, so the feature list cannot drift between them.
#
# WHY --features integration-tests,llm-all
# ----------------------------------------
# `integration-tests` alone resolves the workspace's default feature set, which is
# `default = ["llm-openai", "llm-anthropic", "llm-deepseek"]` — only the three
# adapters that predate Phase 17. The six added since (kimi, qwen, grok, ollama,
# gemini, openai-compatible) sit behind non-default flags, so they were never
# compiled, never instrumented, and contributed ZERO lines to the measured figure.
# The gate passed at 84.32% over 49209 lines while ignoring 5117 lines of shipped
# adapter code, and could not have caught a regression in any of them.
# With llm-all: 85.01% over 54326 lines, all nine adapters counted.
#
# SERVICE ENDPOINTS
# -----------------
# Resolution is VALIDATE-THEN-FALL-BACK, not "pre-set always wins". A configured
# endpoint is used only if it actually answers:
#   * CI exports TEST_* pointing at the host-mapped ports of
#     docker/docker-compose.test.yml — reachable there, so CI keeps its own values.
#   * The repo .env also exports TEST_* (localhost:6380 / localhost:9010) for host
#     use, and that file is auto-sourced into every devcontainer shell. Those ports
#     are NOT reachable from inside the container, where the compose peers are
#     redis:6379 / rustfs:9000. Honouring an unreachable preset is what made
#     coverage look permanently "unmeasurable" here.
# Credentials follow the endpoint: each stack has its own, so when this script
# picks an endpoint it sets the matching credentials rather than inheriting a
# stale pair aimed at the other stack.
set -euo pipefail
cd "$(dirname "$0")/.."

FLOOR="${COVERAGE_FLOOR:-82}"

# Probes must not assume a client binary exists. GitHub runners do not ship
# redis-cli, and CI's old wait loop only tolerated that because it was best-effort
# (a failed probe there just spun until timeout without failing the step). This
# script HARD-FAILS on an unreachable service, so a missing redis-cli would turn
# into "Redis unreachable" on a runner where Redis is in fact up. Fall back to a
# bare TCP connect via bash's /dev/tcp, which needs no external tool.
probe_tcp() { (exec 3<>"/dev/tcp/$1/$2") 2>/dev/null; }

probe_redis() {
    if command -v redis-cli >/dev/null 2>&1; then
        redis-cli -h "$1" -p "$2" ping >/dev/null 2>&1
    else
        probe_tcp "$1" "$2"
    fi
}

probe_object_store() {
    if command -v curl >/dev/null 2>&1; then
        curl -sf -o /dev/null "http://$1/health/ready" 2>/dev/null
    else
        probe_tcp "${1%%:*}" "${1##*:}"
    fi
}

# --- Redis -----------------------------------------------------------------
if [ -n "${TEST_REDIS_HOST:-}" ] \
   && probe_redis "$TEST_REDIS_HOST" "${TEST_REDIS_PORT:-6379}"; then
    : "${TEST_REDIS_PORT:=6379}"
elif probe_redis redis 6379; then
    TEST_REDIS_HOST=redis TEST_REDIS_PORT=6379
elif probe_redis localhost 6380; then
    TEST_REDIS_HOST=localhost TEST_REDIS_PORT=6380
else
    echo "ERROR: Redis unreachable at ${TEST_REDIS_HOST:-<unset>}:${TEST_REDIS_PORT:-?}," >&2
    echo "       redis:6379, or localhost:6380. Start services with 'make services-up'." >&2
    exit 1
fi
export TEST_REDIS_HOST TEST_REDIS_PORT
export REDIS_HOST="$TEST_REDIS_HOST" REDIS_PORT="$TEST_REDIS_PORT"
export REDIS_URL="redis://${TEST_REDIS_HOST}:${TEST_REDIS_PORT}"

# --- Object store (RustFS) -------------------------------------------------
# The exported names keep the MINIO prefix: they are application-side names (D-04).
if [ -n "${TEST_MINIO_ENDPOINT:-}" ] && probe_object_store "$TEST_MINIO_ENDPOINT"; then
    : "${TEST_MINIO_ACCESS_KEY:=paladin-dev}" "${TEST_MINIO_SECRET_KEY:=paladin-dev-secret}"
elif probe_object_store rustfs:9000; then
    TEST_MINIO_ENDPOINT=rustfs:9000
    TEST_MINIO_ACCESS_KEY=paladin-dev TEST_MINIO_SECRET_KEY=paladin-dev-secret
elif probe_object_store localhost:9010; then
    TEST_MINIO_ENDPOINT=localhost:9010
    TEST_MINIO_ACCESS_KEY=testuser TEST_MINIO_SECRET_KEY=testpass123
else
    echo "ERROR: object store (RustFS) unreachable at ${TEST_MINIO_ENDPOINT:-<unset>}, rustfs:9000," >&2
    echo "       or localhost:9010. Start services with 'make services-up'." >&2
    exit 1
fi
export TEST_MINIO_ENDPOINT TEST_MINIO_ACCESS_KEY TEST_MINIO_SECRET_KEY
export MINIO_ENDPOINT="$TEST_MINIO_ENDPOINT"
export MINIO_ACCESS_KEY="$TEST_MINIO_ACCESS_KEY"
export MINIO_SECRET_KEY="$TEST_MINIO_SECRET_KEY"
export USE_EXTERNAL_TEST_SERVICES=true

echo "coverage: redis=${TEST_REDIS_HOST}:${TEST_REDIS_PORT} minio=${TEST_MINIO_ENDPOINT} floor=${FLOOR}%"

exec cargo llvm-cov --workspace --features integration-tests,llm-all \
    --lcov --output-path lcov.info --fail-under-lines "$FLOOR" -- --test-threads=1
