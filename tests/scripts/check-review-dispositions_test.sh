#!/usr/bin/env bash
# check-review-dispositions_test.sh
#
# Committed regression harness for scripts/check-review-dispositions.sh
# (Phase 57, REG-05, D-10..D-13, P-08-3). Every case is a committed fixture
# directory under tests/scripts/fixtures/check-review-dispositions/<case>/:
#
#   archive/<NN>-<slug>/<NN>-REVIEW.md   a mini archive (read-only input)
#   register.md                          the register under test
#   args                                 optional extra flags, one line
#
# The checker is pointed at a case through its --register and --archive flags.
# Every fixture is a copy of `ok` with exactly one thing changed, so a firing
# case names its own class and nothing else needs to be true for it to fire.
# `ok` carries a heading inside a code fence (WR-99 in the archive, a row in
# the register) that must be ignored: if it were counted, `ok` would fail.
#
# Failing cases are asserted first ("prove it fails"), then the passing cases.
# Every run's last output line must match the `checked ...` summary.
#
# Fixtures are regenerated, not hand-edited: they are plain text and a byte
# file, safe to diff. The real register is never read by this harness; the real
# archive is only counted (it must derive 51 findings across 13 files).
#
# Usage:  ./tests/scripts/check-review-dispositions_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-review-dispositions.sh"
FIXTURES="${WORKSPACE_ROOT}/tests/scripts/fixtures/check-review-dispositions"
REAL_ARCHIVE="${WORKSPACE_ROOT}/.planning/milestones/v0.11.0-phases"

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
SUMMARY_RE='^checked [0-9]+ finding\(s\) across [0-9]+ review file\(s\); [0-9]+ failure\(s\)$'

# run_case CASE -> sets $LAST_OUTPUT and $LAST_STATUS
run_case() {
    local dir="${FIXTURES}/$1" extra=()
    if [ -f "${dir}/args" ]; then
        read -r -a extra <"${dir}/args"
    fi
    LAST_OUTPUT="$("${GUARD}" --register "${dir}/register.md" --archive "${dir}/archive" "${extra[@]+"${extra[@]}"}" 2>&1)"
    LAST_STATUS=$?
}

# check_summary DESC -> the last output line is the summary line.
check_summary() {
    local last
    last="$(tail -n1 <<<"${LAST_OUTPUT}")"
    if ! grep -qE "${SUMMARY_RE}" <<<"${last}"; then
        echo "FAIL: last line for '$1' is not the 'checked ...' summary: ${last}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return 1
    fi
    return 0
}

# assert_fire CASE DESC NEEDLE... -> exit 1, summary line last, every NEEDLE present.
assert_fire() {
    local case_name="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_case "${case_name}"
    if [ "${LAST_STATUS}" -ne 1 ]; then
        echo "FAIL: expected exit 1 for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    check_summary "${desc}" || return
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

# assert_silent CASE DESC SUMMARY -> exit 0 and the exact summary line.
assert_silent() {
    local case_name="$1" desc="$2" summary="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    run_case "${case_name}"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected exit 0 (silent) for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    check_summary "${desc}" || return
    if [ "$(tail -n1 <<<"${LAST_OUTPUT}")" != "${summary}" ]; then
        echo "FAIL: summary for '${desc}' is not: ${summary}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (silent): ${desc}"
}

# assert_lacks DESC NEEDLE -> NEEDLE absent from the most recent run's output.
assert_lacks() {
    ASSERTIONS=$((ASSERTIONS + 1))
    if grep -qF -- "$2" <<<"${LAST_OUTPUT}"; then
        echo "FAIL: output for '$1' unexpectedly contains: $2"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
    else
        echo "PASS (absent): $1"
    fi
}

# --- Failing cases first: one per named failure class ------------------------

assert_fire missing "MISSING: an archived id has no register row" \
    "FAIL MISSING 40 IN-04"
assert_fire unknown "UNKNOWN: a register row names an id the archive lacks" \
    "FAIL UNKNOWN 10 WR-02"
assert_fire duplicate "DUPLICATE: two rows for one finding" \
    "FAIL DUPLICATE 10 WR-01"
assert_fire bad_disposition "BAD_DISPOSITION: 'done' is not closed | waived | open" \
    "FAIL BAD_DISPOSITION 10 WR-01"
assert_fire open "OPEN: an open row fails in strict mode" \
    "FAIL OPEN 10 WR-01"
assert_fire open_no_live "OPEN: under --pending-ok an open row without LIVE <file:line> still fails" \
    "FAIL OPEN 10 WR-01" "LIVE"
assert_fire no_evidence "NO_EVIDENCE: TBD, none and 'see above' are placeholders" \
    "FAIL NO_EVIDENCE 10 WR-01" "FAIL NO_EVIDENCE 10 CR-01" "FAIL NO_EVIDENCE 40 WR-01"
assert_fire closed_no_pointer "CLOSED_NO_POINTER: prose with no commit, test, requirement or path" \
    "FAIL CLOSED_NO_POINTER 10 WR-01"
assert_fire waived_no_owner "WAIVED_NO_OWNER: a waived row with an empty owner" \
    "FAIL WAIVED_NO_OWNER 10 CR-01"
assert_fire totals "TOTALS: the Totals line disagrees with the rows" \
    "FAIL TOTALS"
assert_fire totals_absent "TOTALS: no Totals line at all" \
    "FAIL TOTALS" "no '**Totals:**"
assert_fire no_pointer_section "NO_POINTER_SECTION: a review file lacks the appended section" \
    "FAIL NO_POINTER_SECTION 40"
assert_fire zero_files "ZERO_FILES: an archive with no <NN>-REVIEW.md" \
    "FAIL ZERO_FILES"
assert_fire zero_rows "ZERO_ROWS: a register with a section but no data row" \
    "FAIL ZERO_ROWS"
assert_fire unreadable "UNREADABLE: a non-UTF-8 register" \
    "FAIL UNREADABLE"
assert_fire unclosed_fence "UNCLOSED_FENCE: an archived review with a never-closed fence" \
    "FAIL UNCLOSED_FENCE 10"
assert_fire unclosed_fence_register "UNCLOSED_FENCE: a register with a never-closed fence" \
    "FAIL UNCLOSED_FENCE"
assert_fire malformed_row "MALFORMED_ROW: a phase-numbered row with five cells" \
    "FAIL MALFORMED_ROW 40 IN-04"
assert_fire zero_files_listed_phase "ZERO_FILES: --phases names a phase with no review file" \
    "FAIL ZERO_FILES 99"

# --- Passing cases ------------------------------------------------------------

assert_silent ok "ok: closed and waived rows, fenced headings ignored" \
    "checked 5 finding(s) across 2 review file(s); 0 failure(s)"
assert_lacks "ok: the fenced WR-99 heading is never an expected id" "WR-99"

assert_silent open_pending_ok "--pending-ok: an open row with 'LIVE <file:line>' passes" \
    "checked 5 finding(s) across 2 review file(s); 0 failure(s)"

assert_silent scoped "--phases 10: a missing phase-40 row and section are not reported" \
    "checked 2 finding(s) across 1 review file(s); 0 failure(s)"

# --- The real archive: derive, never vacuous --------------------------------

ASSERTIONS=$((ASSERTIONS + 1))
REAL_OUTPUT="$("${GUARD}" --register "${FIXTURES}/ok/register.md" --archive "${REAL_ARCHIVE}" 2>&1 || true)"
if [ "$(tail -n1 <<<"${REAL_OUTPUT}" | sed -E 's/; [0-9]+ failure\(s\)$//')" = "checked 51 finding(s) across 13 review file(s)" ]; then
    echo "PASS (derive): the real archive yields 51 findings across 13 review files"
else
    echo "FAIL: the real archive did not yield 51 findings across 13 review files"
    echo "${REAL_OUTPUT}" | tail -n3 | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- The archive and fixtures must never be mutated by a run. ----------------

ASSERTIONS=$((ASSERTIONS + 1))
if [ -z "$(git -C "${WORKSPACE_ROOT}" status --porcelain -- tests/scripts/fixtures/check-review-dispositions 2>/dev/null | grep '^ M' || true)" ]; then
    echo "PASS (no mutation): no fixture was modified by this run"
else
    echo "FAIL: a fixture was modified by this run"
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
