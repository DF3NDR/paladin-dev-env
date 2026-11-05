#!/usr/bin/env bash
# check-doc-models_test.sh
#
# Committed regression harness for scripts/check-doc-models.sh (Phase 58,
# SHIP-10, DF-18). Every case is a committed fixture tree under
# tests/scripts/fixtures/check-doc-models/<case>/:
#
#   docs/src/...   pages the gate scans
#   examples/...   example files the gate scans
#   allowlist      optional allowlist (`<path>|<line substring>|<reason>`)
#
# The gate is pointed at a case through its --root and --allowlist flags; a case
# without an `allowlist` file gets /dev/null, so no entry is in force.
#
# Failing cases are asserted first ("prove it fails"), then the passing cases.
# Every run's final output line must match SUMMARY_RE.
#
# Usage:  ./tests/scripts/check-doc-models_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-doc-models.sh"
FIXTURES="${WORKSPACE_ROOT}/tests/scripts/fixtures/check-doc-models"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi
if [ ! -d "${FIXTURES}" ]; then
    echo "ERROR: fixtures not found at ${FIXTURES}" >&2
    exit 1
fi

FAILED=0
ASSERTIONS=0
SUMMARY_RE='^(OK|FAIL): '

# run_case CASE -> sets $LAST_OUTPUT and $LAST_STATUS
run_case() {
    local dir="${FIXTURES}/$1" allow="/dev/null"
    if [ -f "${dir}/allowlist" ]; then
        allow="${dir}/allowlist"
    fi
    LAST_OUTPUT="$("${GUARD}" --root "${dir}" --allowlist "${allow}" 2>&1)"
    LAST_STATUS=$?
}

# check_summary DESC -> the last output line is an OK:/FAIL: summary line.
check_summary() {
    local last
    last="$(tail -n1 <<<"${LAST_OUTPUT}")"
    if ! grep -qE "${SUMMARY_RE}" <<<"${last}"; then
        echo "FAIL: last line for '$1' is not an OK:/FAIL: summary: ${last}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return 1
    fi
    return 0
}

# check_needles DESC NEEDLE... -> every NEEDLE appears in the output.
check_needles() {
    local desc="$1" needle
    shift
    for needle in "$@"; do
        if ! grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
            echo "FAIL: output for '${desc}' does not contain: ${needle}"
            echo "${LAST_OUTPUT}" | sed 's/^/  | /'
            FAILED=$((FAILED + 1))
            return 1
        fi
    done
    return 0
}

# assert_fire CASE WANT_STATUS DESC NEEDLE... -> exit WANT_STATUS, summary
# line last, every NEEDLE present.
assert_fire() {
    local case_name="$1" want="$2" desc="$3"
    shift 3
    ASSERTIONS=$((ASSERTIONS + 1))
    run_case "${case_name}"
    if [ "${LAST_STATUS}" -ne "${want}" ]; then
        echo "FAIL: expected exit ${want} for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    check_summary "${desc}" || return
    check_needles "${desc}" "$@" || return
    echo "PASS (fire): ${desc}"
}

# assert_silent CASE DESC NEEDLE... -> exit 0, summary line last, every NEEDLE
# present.
assert_silent() {
    local case_name="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_case "${case_name}"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected exit 0 for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    check_summary "${desc}" || return
    check_needles "${desc}" "$@" || return
    echo "PASS (silent): ${desc}"
}

# --- Failing cases first -----------------------------------------------------

assert_fire quoted 1 "a double-quoted id in a docs page names file:line" \
    "docs/src/page.md:4: deprecated model id 'gpt-4' (double-quoted)" \
    "FAIL: 1 deprecated model literal(s) in 1 file(s)"

assert_fire single_and_backtick 1 "single-quoted and backticked ids are both found" \
    "docs/src/page.md:3: deprecated model id 'gpt-4' (single-quoted)" \
    "docs/src/page.md:5: deprecated model id 'gpt-4' (backticked)" \
    "FAIL: 2 deprecated model literal(s) in 1 file(s)"

assert_fire yaml_and_flag 1 "a YAML model: value and a --model flag are both found" \
    "examples/config.yaml:2: deprecated model id 'gpt-4' (yaml value)" \
    "examples/run.sh:3: deprecated model id 'gpt-4' (--model flag)" \
    "FAIL: 2 deprecated model literal(s) in 2 file(s)"

assert_fire stale_allow 1 "an allowlist entry that exempts no line is STALE_ALLOW" \
    "STALE_ALLOW" \
    "this text is nowhere"

assert_fire zero_files 2 "an empty tree is a named failure, never a silent pass" \
    "ZERO_FILES" \
    "FAIL: 0 file(s) scanned"

# --- Passing cases -----------------------------------------------------------

assert_silent prefix_lookalikes \
    "gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4-32k, gpt-4-invalid and an output line are not flagged" \
    "OK: 2 file(s) scanned, 0 deprecated model literal(s) (allowlisted: 0)"

assert_silent allowlisted "a hit on a line holding the allowlist substring is exempt and counted" \
    "OK: 1 file(s) scanned, 0 deprecated model literal(s) (allowlisted: 1)"

echo ""
if [ "${FAILED}" -eq 0 ]; then
    echo "✅ ${ASSERTIONS} assertion(s) passed."
    exit 0
else
    echo "❌ ${FAILED}/${ASSERTIONS} assertion(s) failed."
    exit 1
fi
