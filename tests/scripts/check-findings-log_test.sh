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

# Checksum of the real log, to prove this run never mutates it.
REAL_SUM_BEFORE="$(cksum <"${REAL_LOG}" 2>/dev/null || true)"

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

# plant_entry ID VALUE -> appends a blank line, a "### ID — extra" heading and,
# when VALUE is non-empty, a "- **Disposition:** VALUE" bullet to $FX. Sets
# $HEADING_LINE to the heading's 1-based line number.
plant_entry() {
    plant ""
    plant "### $1 — extra"
    HEADING_LINE="$(last_line)"
    if [ -n "$2" ]; then
        plant "- **Disposition:** $2"
    fi
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

# assert_output_lacks DESC NEEDLE -> NEEDLE is absent from the output of the
# most recent assert_fire / assert_silent / run_guard.
assert_output_lacks() {
    ASSERTIONS=$((ASSERTIONS + 1))
    if grep -qF -- "$2" <<<"${LAST_OUTPUT}"; then
        echo "FAIL: output for '$1' unexpectedly contains: $2"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
    else
        echo "PASS (absent): $1"
    fi
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

fresh reopened
plant '- **Disposition (2026-11-04):** open'
assert_fire "${FX}" "the last bullet wins: closed then re-opened reads OPEN" \
    ":$(last_line): DF-01: OPEN"

fresh missing
replace_in "${DISPOSED}" '- **Related:** none'
assert_fire "${FX}" "an entry with no Disposition bullet fires MISSING at its heading line" \
    ":$(line_of '### DF-01 — first'): DF-01: MISSING"

fresh fenced-disposition
replace_in "${DISPOSED}" $'```\n- **Disposition:** in v0.11.1 — x\n```'
assert_fire "${FX}" "a Disposition bullet that sits inside a code fence does not count: MISSING" \
    ":$(line_of '### DF-01 — first'): DF-01: MISSING"

fresh unrecognised
replace_in "${DISPOSED}" '- **Disposition:** fixed'
assert_fire "${FX}" "an unknown keyword fires UNRECOGNISED" \
    ":$(last_line): DF-01: UNRECOGNISED"

NO_POINTER_FORMS=('in v0.11.1' 'in v0.11.1 —' 'in v0.11.1 — **' 'in v0.11.1: `')
n=0
for form in "${NO_POINTER_FORMS[@]}"; do
    n=$((n + 1))
    fresh "no-pointer-${n}"
    replace_in "${DISPOSED}" "- **Disposition:** ${form}"
    assert_fire "${FX}" "disposition '${form}' (no evidence after the keyword) fires NO_POINTER" \
        ":$(last_line): DF-01: NO_POINTER"
done

BAD_OUT_FORMS=('out' 'out ()' 'out (  )' 'out — no parens' 'out (unterminated')
n=0
for form in "${BAD_OUT_FORMS[@]}"; do
    n=$((n + 1))
    fresh "bad-out-${n}"
    replace_in "${DISPOSED}" "- **Disposition:** ${form}"
    assert_fire "${FX}" "disposition '${form}' fires BAD_OUT" \
        ":$(last_line): DF-01: BAD_OUT"
done

fresh dup-target-absent
plant_entry DF-02 'duplicate of DF-09'
assert_fire "${FX}" "'duplicate of' an id no heading carries fires BAD_DUP_TARGET" \
    ":$(last_line): DF-02: BAD_DUP_TARGET"

fresh dup-target-self
replace_in "${DISPOSED}" '- **Disposition:** duplicate of DF-01'
assert_fire "${FX}" "'duplicate of' the entry itself fires BAD_DUP_TARGET" \
    ":$(last_line): DF-01: BAD_DUP_TARGET"

fresh dup-id
plant_entry DF-02 'in v0.11.1 — a'
plant_entry DF-02 'in v0.11.1 — b'
assert_fire "${FX}" "two headings with the id DF-02 fire DUPLICATE_ID at the second heading" \
    ":${HEADING_LINE}: DF-02: DUPLICATE_ID"

fresh dup-id-integer
plant_entry DF-02 'in v0.11.1 — a'
plant_entry DF-2 'in v0.11.1 — b'
assert_fire "${FX}" "DF-2 after DF-02 is the same integer id: DUPLICATE_ID" \
    ":${HEADING_LINE}: DF-2: DUPLICATE_ID"

fresh not-increasing
plant_entry DF-03 'in v0.11.1 — a'
plant_entry DF-02 'in v0.11.1 — b'
assert_fire "${FX}" "DF-02 after DF-03 fires NOT_INCREASING at the DF-02 heading" \
    ":${HEADING_LINE}: DF-02: NOT_INCREASING"

fresh dup-beats-order
plant_entry DF-03 'in v0.11.1 — a'
plant_entry DF-01 'in v0.11.1 — b'
assert_fire "${FX}" "a repeated id that is also lower than the previous heading is DUPLICATE_ID" \
    ":${HEADING_LINE}: DF-01: DUPLICATE_ID"
assert_output_lacks "DUPLICATE_ID takes precedence over NOT_INCREASING" "NOT_INCREASING"

# --- Named failures: the checker never passes vacuously ----------------------

FX="${SCRATCH}/empty.md"
: >"${FX}"
assert_fire "${FX}" "an empty file fires ZERO_ENTRIES" "ZERO_ENTRIES"

fresh template-only
head -n "$(($(line_of '### DF-01 — first') - 1))" "${FX}" >"${FX}.cut" && mv "${FX}.cut" "${FX}"
assert_fire "${FX}" "a file whose only '### DF-nn' heading sits in the fenced template fires ZERO_ENTRIES" \
    "ZERO_ENTRIES"

assert_fire "${SCRATCH}/does-not-exist.md" "a path that does not exist fires UNREADABLE" "UNREADABLE"

FX="${SCRATCH}/not-utf8.md"
printf '\377\376\000### DF-01 \377\n' >"${FX}"
assert_fire "${FX}" "a file holding the bytes 0xff 0xfe fires UNREADABLE" "UNREADABLE"

fresh unclosed-fence
plant '```'
FENCE_LINE="$(last_line)"
plant '### DF-02 — hidden'
plant '- **Disposition:** open'
assert_fire "${FX}" "a fence opened and never closed fires UNCLOSED_FENCE at its opening line" \
    "UNCLOSED_FENCE" "${FX}:${FENCE_LINE}" "parsed 1 entry"

# --- Every finding listed, deterministic order, no mutation ------------------

fresh multi
plant_entry DF-02 ''
plant_entry DF-03 'out'
plant_entry DF-04 'open'
plant_entry DF-05 'in v0.11.1 — fine'
MULTI_SUM_BEFORE="$(cksum <"${FX}")"
assert_fire "${FX}" "every finding is listed, not just the first" \
    "DF-02: MISSING" "DF-03: BAD_OUT" "DF-04: OPEN" "parsed 5 entries"
FIRST_OUTPUT="${LAST_OUTPUT}"
run_guard "${FX}"
ASSERTIONS=$((ASSERTIONS + 1))
if [ "${LAST_OUTPUT}" = "${FIRST_OUTPUT}" ]; then
    echo "PASS (determinism): two runs on one multi-finding log print byte-identical output"
else
    echo "FAIL: two runs on one log printed different output"
    FAILED=$((FAILED + 1))
fi
ASSERTIONS=$((ASSERTIONS + 1))
FINDING_LINES="$(grep -oE ':[0-9]+: DF-' <<<"${LAST_OUTPUT}" | grep -oE '[0-9]+')"
if [ -n "${FINDING_LINES}" ] && [ "${FINDING_LINES}" = "$(sort -n <<<"${FINDING_LINES}")" ]; then
    echo "PASS (order): per-entry findings are printed sorted by line number"
else
    echo "FAIL: per-entry findings are not sorted by line number"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi
ASSERTIONS=$((ASSERTIONS + 1))
if [ "${MULTI_SUM_BEFORE}" = "$(cksum <"${FX}")" ]; then
    echo "PASS (no mutation): the fixture log is byte-identical after two runs"
else
    echo "FAIL: the guard modified its input log"
    FAILED=$((FAILED + 1))
fi

# --- Passing cases ------------------------------------------------------------

fresh baseline
assert_silent "${FX}" "the fenced template heading and bullet are ignored; one disposed entry passes" \
    "parsed 1 entry"

fresh dated-after-open
replace_in "${DISPOSED}" '- **Disposition:** open → recommend **in v0.11.1**'
plant '- **Disposition (2026-11-03):** in v0.11.1 — fixed by FIRST-01 (Phase 54)'
assert_silent "${FX}" "a dated 'in v0.11.1 — …' bullet appended after 'open → recommend' passes" \
    "parsed 1 entry"

fresh emphasised-in
replace_in "${DISPOSED}" '- **Disposition:** **in v0.11.1** — fixed by X'
assert_silent "${FX}" "emphasis around 'in v0.11.1' is stripped" "parsed 1 entry"

fresh continuation
replace_in "${DISPOSED}" $'- **Disposition (2026-11-03):**\n  in v0.11.1 — fixed by X'
assert_silent "${FX}" "a value carried on an indented continuation line is read" "parsed 1 entry"

fresh out-nested
replace_in "${DISPOSED}" '- **Disposition:** out (fixed by Phase 54 — FIRST-02 (proof: make check-version-strings))'
assert_silent "${FX}" "'out (…)' with nested parentheses passes (greedy to the last paren)" \
    "parsed 1 entry"

fresh out-simple
replace_in "${DISPOSED}" '- **Disposition:** **out** (not reproducible on HEAD)'
assert_silent "${FX}" "an emphasised 'out (reason)' passes" "parsed 1 entry"

fresh dup-of-disposed
plant_entry DF-02 'duplicate of DF-01'
assert_silent "${FX}" "'duplicate of DF-01' passes while DF-01 is disposed" "parsed 2 entries"

fresh dup-of-integer
plant_entry DF-02 'duplicate of DF-1'
assert_silent "${FX}" "'duplicate of DF-1' resolves DF-01 by integer id" "parsed 2 entries"

fresh heading-after-fence
plant ''
plant '```'
plant 'some fenced text'
plant '```'
plant '### DF-02 — straight after a closing fence'
plant '- **Disposition:** in v0.11.1 — x'
assert_silent "${FX}" "a '### DF-nn' heading on the line straight after a fence close is an entry" \
    "parsed 2 entries"

fresh subheading
plant '#### Notes'
plant '- **Related:** none'
plant '- **Disposition (2026-11-05):** out (superseded)'
assert_silent "${FX}" "a '####' sub-heading does not end an entry; the later bullet is its last" \
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

# --- The real log must never be mutated by this test. ----------------------

ASSERTIONS=$((ASSERTIONS + 1))
if [ -n "${REAL_SUM_BEFORE}" ] && [ "${REAL_SUM_BEFORE}" = "$(cksum <"${REAL_LOG}" 2>/dev/null || true)" ]; then
    echo "PASS (no mutation): the real log is byte-identical before and after"
else
    echo "FAIL: the real log was mutated by this test run (or could not be read)"
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
