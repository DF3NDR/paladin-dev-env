#!/usr/bin/env bash
# check-findings-log_test.sh
#
# Committed regression harness for scripts/check-findings-log.sh
# (Phase 56, DOGF-01, D-01, D-03). Mirrors the house pattern used by
# tests/scripts/check-version-strings_test.sh: every fixture log is built
# under a single `mktemp -d` scratch directory removed on exit via a trap, and
# the guard is pointed at a fixture through the test-only FINDINGS_LOG_PATH
# seam (the guard itself takes no argument).
#
# Every case starts from `make_log`, a fixture log on which the guard passes,
# and changes exactly one thing. Failing cases are asserted FIRST ("prove it
# fails"), then the passing cases, then the real log.
#
# The real log legitimately holds `open` entries mid-phase and CI runs this
# harness (P12, D-03), so the real-log case asserts only that the guard parsed
# it: a `parsed N entr...` line with N >= 4 and neither ZERO_ENTRIES nor
# UNREADABLE. It never asserts the guard's exit status on the real log.
#
# Usage:  ./tests/scripts/check-findings-log_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-findings-log.sh"
REAL_LOG="${WORKSPACE_ROOT}/.project/v0.11.1/02-dogfood-findings-log.md"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-findings-log-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# The one disposed bullet of the baseline fixture; cases rewrite it in place.
DISPOSED='- **Disposition:** in v0.11.1 — fixed by TEST-01 (Phase 0)'

# make_log FILE -> a minimal findings log on which the guard passes: a title,
# a fenced entry template (whose `### DF-nn` heading and `open | ...` bullet
# the guard must ignore), a `## Findings` heading and one disposed entry.
make_log() {
    cat >"$1" <<'LOG'
# Log

```
### DF-nn — <title>
- **Disposition:** open | in v0.11.1 | out (reason) | duplicate of DF-nn
```

## Findings

### DF-01 — first
- **Severity:** S3
- **Disposition:** in v0.11.1 — fixed by TEST-01 (Phase 0)
LOG
}

# fresh NAME -> builds SCRATCH/NAME.md and sets $FX to it.
fresh() {
    FX="${SCRATCH}/$1.md"
    make_log "${FX}"
}

# plant TEXT -> appends TEXT as a new last line of $FX.
plant() {
    printf '%s\n' "$1" >>"${FX}"
}

# replace_in OLD NEW -> replaces every literal OLD with NEW in $FX.
replace_in() {
    local content
    content="$(<"${FX}")"
    printf '%s\n' "${content//"$1"/"$2"}" >"${FX}"
}

# line_of TEXT -> 1-based number of the first line of $FX containing the
# literal TEXT.
line_of() {
    grep -nF -- "$1" "${FX}" | head -n1 | cut -d: -f1
}

# last_line -> 1-based number of the last line of $FX (in every baseline
# fixture that is the entry's disposition bullet).
last_line() {
    wc -l <"${FX}" | tr -d ' '
}

# run_guard LOG -> sets $LAST_OUTPUT and $LAST_STATUS
run_guard() {
    LAST_OUTPUT="$(FINDINGS_LOG_PATH="$1" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}

# assert_fire LOG DESC NEEDLE... -> non-zero exit AND every NEEDLE appears in
# the output.
assert_fire() {
    local log="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${log}"
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

# assert_silent LOG DESC [NEEDLE...] -> zero exit AND every NEEDLE appears.
assert_silent() {
    local log="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${log}"
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

# --- Failing cases first ------------------------------------------------------

fresh open-plain
replace_in "${DISPOSED}" '- **Disposition:** open'
assert_fire "${FX}" "a bare 'open' disposition fires OPEN" \
    "DF-01" "DF-01: OPEN" ":$(last_line): DF-01: OPEN"

fresh open-preseeded
replace_in "${DISPOSED}" '- **Disposition:** open → recommend **in v0.11.1**'
assert_fire "${FX}" "the pre-seeded 'open → recommend **in v0.11.1**' form fires OPEN" \
    "DF-01: OPEN"

fresh missing
replace_in "${DISPOSED}" '- **Related:** none'
assert_fire "${FX}" "an entry with no Disposition bullet fires MISSING at its heading line" \
    ":$(line_of '### DF-01 — first'): DF-01: MISSING"

# --- Passing cases ------------------------------------------------------------

fresh baseline
assert_silent "${FX}" "the fenced template heading and bullet are ignored; one disposed entry passes" \
    "parsed 1 entry"

fresh dated-after-open
replace_in "${DISPOSED}" '- **Disposition:** open → recommend **in v0.11.1**'
plant '- **Disposition (2026-11-03):** in v0.11.1 — fixed by FIRST-01 (Phase 54)'
assert_silent "${FX}" "a dated 'in v0.11.1 — …' bullet appended after 'open → recommend' passes" \
    "parsed 1 entry"

# --- The real log -------------------------------------------------------------
# Parseability only. The real log legitimately holds open entries mid-phase and
# CI runs this harness, so the guard's exit status is never asserted here
# (D-03, P12).

ASSERTIONS=$((ASSERTIONS + 1))
run_guard "${REAL_LOG}"
REAL_COUNT="$(grep -oE 'parsed [0-9]+ entr' <<<"${LAST_OUTPUT}" | tail -n1 | grep -oE '[0-9]+')"
if [ -z "${REAL_COUNT}" ]; then
    echo "FAIL: the real-log output carries no 'parsed N entr...' line"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
elif [ "${REAL_COUNT}" -lt 4 ]; then
    echo "FAIL: the real log parsed ${REAL_COUNT} entries; expected at least 4"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
elif grep -qE 'ZERO_ENTRIES|UNREADABLE' <<<"${LAST_OUTPUT}"; then
    echo "FAIL: the real-log output names ZERO_ENTRIES or UNREADABLE"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    echo "PASS (parse): the real log parsed ${REAL_COUNT} entries (exit status deliberately not asserted)"
fi

echo ""
if [ "${FAILED}" -eq 0 ]; then
    echo "✅ ${ASSERTIONS} assertion(s) passed."
    exit 0
else
    echo "❌ ${FAILED}/${ASSERTIONS} assertion(s) failed."
    exit 1
fi
