#!/usr/bin/env bash
# check-version-strings_test.sh
#
# Committed regression harness for scripts/check-version-strings.sh
# (Phase 54, FIRST-03, D-06..D-12). Mirrors the house pattern used by
# tests/scripts/check-dependency-allowlist_test.sh: every fixture tree is
# built under a single `mktemp -d` scratch directory removed on exit via a
# trap, and the guard is pointed at a fixture through the test-only
# VERSION_STRINGS_ROOT seam (the guard itself takes no argument).
#
# Every case starts from `make_tree`, a current fixture tree at workspace
# version 1.2.3 on which the guard passes, and changes exactly one thing.
# Failing cases are asserted FIRST ("prove it fails"), then the passing cases,
# then the real tree, and last a checksum assertion that this run never
# mutated the real scanned files.
#
# Usage:  ./tests/scripts/check-version-strings_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-version-strings.sh"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-version-strings-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# Checksums of the real scanned files, to prove the run never mutates them.
real_sums() {
    (
        cd "${WORKSPACE_ROOT}" || exit 1
        find README.md crates/*/README.md docs/src k8s crates/paladin-web/openapi.json \
            -type f 2>/dev/null | LC_ALL=C sort | xargs cksum
    )
}
BEFORE_SUMS="$(real_sums)"

# make_tree DIR -> a current fixture tree at workspace version 1.2.3.
make_tree() {
    local d="$1"
    mkdir -p "${d}/crates/paladin-core" "${d}/crates/paladin-llm" "${d}/crates/paladin-web" \
        "${d}/docs/src/api-reference" "${d}/docs/src/user-guides" "${d}/k8s/server"
    printf '[package]\nname = "paladin-ai"\nversion = "1.2.3"\n' >"${d}/Cargo.toml"
    printf '[package]\nname = "paladin-ai-core"\nversion.workspace = true\n' \
        >"${d}/crates/paladin-core/Cargo.toml"
    printf '[package]\nname = "paladin-llm"\nversion.workspace = true\n' \
        >"${d}/crates/paladin-llm/Cargo.toml"
    printf '# paladin-ai\n\n```toml\npaladin-ai = "1.2.3"\n```\n\nCurrent version: **1.2.3**.\n' \
        >"${d}/README.md"
    printf '# paladin-llm\n\n```toml\npaladin-llm = "1.2.3"\n```\n' \
        >"${d}/crates/paladin-llm/README.md"
    printf '# Guide\n\n```toml\npaladin-llm = { version = "1.2.3" }\n```\n' \
        >"${d}/docs/src/guide.md"
    printf '# Migration guide\n\n```toml\npaladin-ai = "0.4"\n```\n' \
        >"${d}/docs/src/api-reference/migration-guide.md"
    printf 'apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  labels:\n    version: v1.2.3\nspec:\n  containers:\n    - name: s\n      image: paladin-server:1.2.3\n' \
        >"${d}/k8s/server/deployment.yaml"
    printf '{"openapi": "3.1.0", "info": {"title": "x", "version": "1.2.3"}}\n' \
        >"${d}/crates/paladin-web/openapi.json"
}

# fresh NAME -> builds SCRATCH/NAME and sets $FX to it.
fresh() {
    FX="${SCRATCH}/$1"
    make_tree "${FX}"
}

# plant FILE TEXT -> appends TEXT as a new last line of FILE (relative to $FX)
# and sets $PLANT_LINE to its 1-based line number.
plant() {
    printf '%s\n' "$2" >>"${FX}/$1"
    PLANT_LINE="$(wc -l <"${FX}/$1" | tr -d ' ')"
}

# run_guard ROOT -> sets $LAST_OUTPUT and $LAST_STATUS
run_guard() {
    LAST_OUTPUT="$(VERSION_STRINGS_ROOT="$1" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}

# assert_fire ROOT DESC NEEDLE... -> non-zero exit AND every NEEDLE appears in
# the output.
assert_fire() {
    local root="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${root}"
    if [ "${LAST_STATUS}" -eq 0 ]; then
        echo "FAIL: expected non-zero exit for: ${desc} (got 0)"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    local needle
    for needle in "$@"; do
        if ! grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
            echo "FAIL: output for '${desc}' does not name: ${needle}"
            echo "${LAST_OUTPUT}" | sed 's/^/  | /'
            FAILED=$((FAILED + 1))
            return
        fi
    done
    echo "PASS (fire): ${desc}"
}

# assert_silent ROOT DESC [NEEDLE...] -> zero exit AND every NEEDLE appears.
assert_silent() {
    local root="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${root}"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected zero exit (silent) for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    local needle
    for needle in "$@"; do
        if ! grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
            echo "FAIL: output for '${desc}' does not contain: ${needle}"
            echo "${LAST_OUTPUT}" | sed 's/^/  | /'
            FAILED=$((FAILED + 1))
            return
        fi
    done
    echo "PASS (silent): ${desc}"
}

# --- Failing cases first (class 1: Cargo pins) ------------------------------

fresh stale-short-pin
plant docs/src/guide.md 'paladin-ai = "0.5"'
assert_fire "${FX}" "stale paladin-ai pin in docs/src/guide.md names file:line and both versions" \
    "docs/src/guide.md:${PLANT_LINE}: found '0.5', want '1.2.3'"

fresh stale-bare
plant docs/src/guide.md 'paladin-llm = "1.2.2"'
assert_fire "${FX}" "bare pin paladin-llm = \"1.2.2\"" \
    "docs/src/guide.md:${PLANT_LINE}: found '1.2.2', want '1.2.3'"

fresh stale-inline
plant docs/src/guide.md 'paladin-llm = { version = "1.2.2", features = ["openai"] }'
assert_fire "${FX}" "inline table with a stale version" \
    "docs/src/guide.md:${PLANT_LINE}: found '1.2.2', want '1.2.3'"

fresh stale-alias
plant docs/src/guide.md 'paladin-core = { package = "paladin-ai-core", version = "1.2.2" }'
assert_fire "${FX}" "alias line whose package = value is a workspace name" \
    "docs/src/guide.md:${PLANT_LINE}: found '1.2.2', want '1.2.3'"

fresh stale-multiline
plant docs/src/guide.md 'paladin-ai = { version = "1.2.2", features = ['
plant docs/src/guide.md '    "llm-anthropic",'
plant docs/src/guide.md '] }'
assert_fire "${FX}" "multi-line inline table: the opening line is checked" \
    "docs/src/guide.md:$((PLANT_LINE - 2)): found '1.2.2', want '1.2.3'"

fresh stale-rustdoc
plant docs/src/guide.md '//! paladin-ai = "1.2.2"'
assert_fire "${FX}" "a //! line inside a rust fence" \
    "docs/src/guide.md:${PLANT_LINE}: found '1.2.2', want '1.2.3'"

fresh stale-short-form
plant docs/src/guide.md 'paladin-ai = "1.2"'
assert_fire "${FX}" "the short form \"1.2\" for a 1.2.3 workspace" \
    "docs/src/guide.md:${PLANT_LINE}: found '1.2', want '1.2.3'"

fresh stale-nonexempt
plant docs/src/user-guides/x.md 'paladin-ai = "0.4"'
assert_fire "${FX}" "the exempt file's content in a scanned file fails" \
    "docs/src/user-guides/x.md:${PLANT_LINE}: found '0.4', want '1.2.3'"

# A tree holding only Cargo.toml scanned nothing: a named failure, never a pass.
mkdir -p "${SCRATCH}/zero-files"
printf '[package]\nname = "paladin-ai"\nversion = "1.2.3"\n' >"${SCRATCH}/zero-files/Cargo.toml"
assert_fire "${SCRATCH}/zero-files" "a tree with zero scannable files fails ZERO_FILES" \
    "ZERO_FILES"

# --- Passing cases ----------------------------------------------------------

fresh current
assert_silent "${FX}" "a current fixture tree (exempt file holds a stale 0.4 pin) passes" "OK:"

fresh no-finding
plant docs/src/guide.md 'paladin-custom-llm = "0.1"'
plant docs/src/guide.md 'tokio = { version = "1" }'
plant docs/src/guide.md 'paladin-ai = { path = "../x" }'
plant docs/src/guide.md 'package = "paladin-ai-core"'
assert_silent "${FX}" "non-workspace keys, version-less tables and prose produce no finding" "OK:"

# --- The real tree ----------------------------------------------------------

ASSERTIONS=$((ASSERTIONS + 1))
LAST_OUTPUT="$("${GUARD}" 2>&1)"
LAST_STATUS=$?
if [ "${LAST_STATUS}" -ne 0 ]; then
    echo "FAIL: expected zero exit for the real tree (got ${LAST_STATUS})"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
elif ! grep -qF 'OK:' <<<"${LAST_OUTPUT}"; then
    echo "FAIL: the real-tree output carries no OK: line"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    echo "PASS (silent): the real tree passes"
fi

# --- The real scanned files must never be mutated by this test. -------------
ASSERTIONS=$((ASSERTIONS + 1))
AFTER_SUMS="$(real_sums)"
if [ -n "${BEFORE_SUMS}" ] && [ "${BEFORE_SUMS}" = "${AFTER_SUMS}" ]; then
    echo "PASS (no mutation): the real scanned files are byte-identical before and after"
else
    echo "FAIL: a real scanned file was mutated by this test run (or none were found)"
    FAILED=$((FAILED + 1))
fi

echo ""
if [ "${FAILED}" -eq 0 ]; then
    echo "✅ ${ASSERTIONS} assertion(s) passed."
    exit 0
else
    echo "❌ ${FAILED}/${ASSERTIONS} assertion(s) failed."
    exit 1
fi
