#!/usr/bin/env bash
# release-body-fit_test.sh
#
# Committed regression harness for scripts/release-body-fit.sh (CIREL-05,
# D-11, plan 52-02): the shared release-body size helper that
# create-or-reuse-release.sh (and, in plan 52-04, finalize-release-body.sh)
# call before a body is sent to the GitHub API.
#
# Every fixture is generated at test time with python3 into a single
# `mktemp -d` scratch directory removed on exit via a trap; the real tree is
# only ever read. Characters are counted with python3 (code points of strict
# UTF-8), never with `wc -m`, which depends on the locale. The helper is
# driven two ways: sourced with RELEASE_BODY_FIT_LIB_ONLY=1 (each call runs
# in its own `bash -c` so the helper's `set -e` never leaks into this
# harness), and as a CLI.
#
# Every case also proves the caller's input file is byte-identical after the
# run (`cmp` against a saved copy): the helper only ever writes OUT.
#
# Fixtures accumulate into $FAILED rather than exiting on the first
# mismatch, matching the "report everything, don't short-circuit" house
# style the guard scripts in this repo follow.
#
# Usage:  ./tests/scripts/release-body-fit_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HELPER="${WORKSPACE_ROOT}/scripts/release-body-fit.sh"

if [ ! -f "${HELPER}" ]; then
    echo "ERROR: helper script not found at ${HELPER}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/release-body-fit-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0
SENTINEL='<!-- paladin:release-truncated -->'
MARKER='TOPSECRET-BODY-MARKER-9f3a'

# --- Real-tree mutation baseline, captured before any fixture runs. --------
MUTATION_WATCH_PATHS=(scripts .github/workflows)
BEFORE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"

# assert DESC CMD... -> passes when CMD exits 0.
assert() {
    local desc="$1"
    shift
    ASSERTIONS=$((ASSERTIONS + 1))
    if "$@"; then
        echo "PASS: ${desc}"
    else
        echo "FAIL: ${desc}"
        FAILED=$((FAILED + 1))
    fi
}

# refute DESC CMD... -> passes when CMD exits non-zero.
refute() {
    local desc="$1"
    shift
    ASSERTIONS=$((ASSERTIONS + 1))
    if "$@"; then
        echo "FAIL: ${desc}"
        FAILED=$((FAILED + 1))
    else
        echo "PASS: ${desc}"
    fi
}

# pylen FILE -> prints the code-point count of FILE (strict UTF-8).
pylen() {
    python3 -I -c 'import sys; print(len(open(sys.argv[1], "rb").read().decode("utf-8")))' "$1"
}

# gen_text CHARS FILE -> writes exactly CHARS code points of numbered ASCII
# lines (49 characters plus a newline each) into FILE.
gen_text() {
    python3 -I - "$1" "$2" <<'PY'
import sys
n, path = int(sys.argv[1]), sys.argv[2]
out, i, total = [], 0, 0
while total < n:
    out.append(("line %05d " % i).ljust(49, "x") + "\n")
    i += 1
    total += 50
open(path, "w", encoding="utf-8", newline="").write("".join(out)[:n])
PY
}

# call FUNCTION ARGS... -> sources the helper in a fresh bash and runs
# FUNCTION. Sets LAST_OUT, LAST_ERR and LAST_STATUS. Limits come from the
# exported PALADIN_RELEASE_BODY_* variables.
call() {
    local err_file="${SCRATCH}/last_stderr"
    LAST_OUT="$(RELEASE_BODY_FIT_LIB_ONLY=1 bash -c 'source "$1"; shift; "$@"' _ "${HELPER}" "$@" 2>"${err_file}")"
    LAST_STATUS=$?
    LAST_ERR="$(cat "${err_file}")"
}

set_limits() {
    export PALADIN_RELEASE_BODY_LIMIT="$1" PALADIN_RELEASE_BODY_RESERVE="$2"
}
reset_limits() {
    unset PALADIN_RELEASE_BODY_LIMIT PALADIN_RELEASE_BODY_RESERVE
}

# in_unchanged DESC IN SAVED_COPY -> the caller's file must be untouched.
in_unchanged() {
    assert "$1: the caller's input file is byte-identical afterwards" cmp -s "$2" "$3"
}

# prefix_check IN OUT -> OUT carries the sentinel exactly once, and OUT
# minus (the blank line + footer) is a prefix of IN that ends with a newline.
prefix_check() {
    python3 -I - "$1" "$2" <<'PY'
import sys
orig = open(sys.argv[1], "rb").read().decode("utf-8")
out = open(sys.argv[2], "rb").read().decode("utf-8")
S = "<!-- paladin:release-truncated -->"
if out.count(S) != 1:
    sys.exit(1)
idx = out.index(S)
# Footer = "\n" + sentinel ...; the kept text is everything before that "\n".
if idx < 1 or out[idx - 1] != "\n":
    sys.exit(1)
kept = out[: idx - 1]
sys.exit(0 if (kept.endswith("\n") and orig.startswith(kept) and len(kept) > 0) else 1)
PY
}

sentinel_count() {
    python3 -I -c 'import sys; print(open(sys.argv[1], "rb").read().decode("utf-8").count("<!-- paladin:release-truncated -->"))' "$1"
}

# --- Defaults: limit 125000, reserve 5000. ----------------------------------
reset_limits
call release_body_limit
assert "default limit is 125000" test "${LAST_OUT}" = "125000"
call release_body_reserve
assert "default reserve is 5000" test "${LAST_OUT}" = "5000"

# --- Case A: exactly `limit` characters is byte-identical; limit+1 is cut. --
set_limits 2000 500
DIR_A="${SCRATCH}/a"
mkdir -p "${DIR_A}"
gen_text 2000 "${DIR_A}/in2000.md"
cp "${DIR_A}/in2000.md" "${DIR_A}/in2000.saved"
call release_body_fit_for_create "${DIR_A}/in2000.md" "${DIR_A}/out2000.md" v1.2.3 o/r
assert "limit-exact body exits 0" test "${LAST_STATUS}" -eq 0
assert "limit-exact body is sent byte-identical" cmp -s "${DIR_A}/in2000.md" "${DIR_A}/out2000.md"
assert "limit-exact body logs nothing" test -z "${LAST_ERR}"
in_unchanged "limit-exact" "${DIR_A}/in2000.md" "${DIR_A}/in2000.saved"

gen_text 2001 "${DIR_A}/in2001.md"
cp "${DIR_A}/in2001.md" "${DIR_A}/in2001.saved"
call release_body_fit_for_create "${DIR_A}/in2001.md" "${DIR_A}/out2001.md" v1.2.3 o/r
assert "limit+1 body exits 0" test "${LAST_STATUS}" -eq 0
assert "limit+1 body is cut to at most limit-reserve (1500) characters" test "$(pylen "${DIR_A}/out2001.md")" -le 1500
assert "limit+1 body ends with the footer sentinel block" \
    grep -q -F "Full changelog: CHANGELOG.md at v1.2.3 (https://github.com/o/r/blob/v1.2.3/CHANGELOG.md)._" "${DIR_A}/out2001.md"
assert "limit+1 body carries the sentinel exactly once" test "$(sentinel_count "${DIR_A}/out2001.md")" -eq 1
assert "limit+1 body is a newline-ended prefix of the original, then the footer" prefix_check "${DIR_A}/in2001.md" "${DIR_A}/out2001.md"
assert "truncation logs original, target and final sizes" \
    grep -q -E '^release-body-fit: original 2001 chars, target 1500 chars, final [0-9]+ chars$' <<<"${LAST_ERR}"
in_unchanged "limit+1" "${DIR_A}/in2001.md" "${DIR_A}/in2001.saved"

# --- Case B: one 5000-character line, no newline -> non-empty hard cut. -----
DIR_B="${SCRATCH}/b"
mkdir -p "${DIR_B}"
python3 -I -c 'import sys; open(sys.argv[1], "w", newline="").write("y" * 5000)' "${DIR_B}/in.md"
cp "${DIR_B}/in.md" "${DIR_B}/in.saved"
call release_body_fit_for_create "${DIR_B}/in.md" "${DIR_B}/out.md" v1.2.3 o/r
assert "no-newline body exits 0" test "${LAST_STATUS}" -eq 0
assert "no-newline body is hard-cut within the target" test "$(pylen "${DIR_B}/out.md")" -le 1500
assert "no-newline hard cut keeps some of the body (never empty)" grep -q '^yyyyyyyyyy' "${DIR_B}/out.md"
assert "no-newline hard cut still carries the footer once" test "$(sentinel_count "${DIR_B}/out.md")" -eq 1
in_unchanged "no-newline" "${DIR_B}/in.md" "${DIR_B}/in.saved"

# --- Case C: characters, not bytes (1500 three-byte characters). -----------
DIR_C="${SCRATCH}/c"
mkdir -p "${DIR_C}"
python3 -I -c 'import sys; open(sys.argv[1], "w", encoding="utf-8", newline="").write("日" * 1500)' "${DIR_C}/in.md"
cp "${DIR_C}/in.md" "${DIR_C}/in.saved"
BYTES_C="$(wc -c < "${DIR_C}/in.md" | tr -d ' ')"
assert "multi-byte fixture has more bytes (${BYTES_C}) than the limit (2000)" test "${BYTES_C}" -gt 2000
call release_body_fit_for_create "${DIR_C}/in.md" "${DIR_C}/out.md" v1.2.3 o/r
assert "multi-byte body under the character limit exits 0" test "${LAST_STATUS}" -eq 0
assert "multi-byte body under the character limit is untouched" cmp -s "${DIR_C}/in.md" "${DIR_C}/out.md"
call release_body_length "${DIR_C}/in.md"
assert "release_body_length counts code points, not bytes" test "${LAST_OUT}" = "1500"
in_unchanged "multi-byte" "${DIR_C}/in.md" "${DIR_C}/in.saved"

# --- Case D: code fences. ---------------------------------------------------
DIR_D="${SCRATCH}/d"
mkdir -p "${DIR_D}"
# Odd: a fence opens near the top and the cut lands inside it.
python3 -I - "${DIR_D}/odd.md" "${DIR_D}/even.md" <<'PY'
import sys
odd = "intro\n```rust\n" + "".join(("code %05d " % i).ljust(49, "z") + "\n" for i in range(80)) + "```\ntail\n"
even = "```\nclosed block\n```\n" + "".join(("line %05d " % i).ljust(49, "x") + "\n" for i in range(80))
open(sys.argv[1], "w", newline="").write(odd)
open(sys.argv[2], "w", newline="").write(even)
PY
cp "${DIR_D}/odd.md" "${DIR_D}/odd.saved"
cp "${DIR_D}/even.md" "${DIR_D}/even.saved"
set_limits 2000 500
call release_body_fit_for_create "${DIR_D}/odd.md" "${DIR_D}/odd.out" v1.2.3 o/r
assert "open-fence body exits 0" test "${LAST_STATUS}" -eq 0
assert "open fence is closed: the line before the blank line and footer is a bare fence" \
    python3 -I -c 'import sys; t=open(sys.argv[1]).read().split("<!-- paladin:release-truncated -->")[0].split("\n"); sys.exit(0 if t[-3]=="```" else 1)' "${DIR_D}/odd.out"
assert "open-fence output keeps an even number of fence lines" \
    test "$(grep -c '^```' "${DIR_D}/odd.out")" -eq 2
assert "open-fence output stays within the target" test "$(pylen "${DIR_D}/odd.out")" -le 1500
call release_body_fit_for_create "${DIR_D}/even.md" "${DIR_D}/even.out" v1.2.3 o/r
assert "balanced-fence body exits 0" test "${LAST_STATUS}" -eq 0
assert "balanced fences get no extra closing fence" test "$(grep -c '^```' "${DIR_D}/even.out")" -eq 2
in_unchanged "open-fence" "${DIR_D}/odd.md" "${DIR_D}/odd.saved"
in_unchanged "balanced-fence" "${DIR_D}/even.md" "${DIR_D}/even.saved"

# --- Case E: re-fitting a body that already carries the footer. -------------
DIR_E="${SCRATCH}/e"
mkdir -p "${DIR_E}"
gen_text 4000 "${DIR_E}/in.md"
set_limits 3000 500
call release_body_fit_for_create "${DIR_E}/in.md" "${DIR_E}/once.md" v1.2.3 o/r
cp "${DIR_E}/once.md" "${DIR_E}/once.saved"
call release_body_fit "${DIR_E}/once.md" "${DIR_E}/twice.md" 1200 v1.2.3 o/r
assert "re-fit of a footered body exits 0" test "${LAST_STATUS}" -eq 0
assert "re-fit strips the old footer: exactly one sentinel remains" test "$(sentinel_count "${DIR_E}/twice.md")" -eq 1
assert "re-fit lands within the smaller target" test "$(pylen "${DIR_E}/twice.md")" -le 1200
assert "re-fit output is still a newline-ended prefix of the original, then one footer" prefix_check "${DIR_E}/in.md" "${DIR_E}/twice.md"
call release_body_fit "${DIR_E}/once.md" "${DIR_E}/same.md" 3000 v1.2.3 o/r
assert "a footered body already within target is copied unchanged" cmp -s "${DIR_E}/once.md" "${DIR_E}/same.md"
in_unchanged "re-fit" "${DIR_E}/once.md" "${DIR_E}/once.saved"

# Case E2: the old footer sits INSIDE the kept budget (short body, footer, then
# more text after it), so only an explicit strip keeps the sentinel unique.
gen_text 4000 "${DIR_E}/long.md"
set_limits 3000 500
call release_body_fit "${DIR_E}/long.md" "${DIR_E}/short_footered.md" 600 v1.2.3 o/r
{ cat "${DIR_E}/short_footered.md"; gen_text 2000 "${DIR_E}/tail.md"; cat "${DIR_E}/tail.md"; } > "${DIR_E}/composed.md"
assert "composed fixture carries the old footer before more text" test "$(grep -c -F "${SENTINEL}" "${DIR_E}/composed.md")" -eq 1
cp "${DIR_E}/composed.md" "${DIR_E}/composed.saved"
call release_body_fit "${DIR_E}/composed.md" "${DIR_E}/composed.out" 1200 v1.2.3 o/r
assert "re-fit with the old footer inside the budget exits 0" test "${LAST_STATUS}" -eq 0
assert "re-fit with the old footer inside the budget leaves exactly one sentinel" test "$(grep -c -F "${SENTINEL}" "${DIR_E}/composed.out")" -eq 1
assert "re-fit with the old footer inside the budget stays within the target" test "$(pylen "${DIR_E}/composed.out")" -le 1200
in_unchanged "re-fit (footer inside budget)" "${DIR_E}/composed.md" "${DIR_E}/composed.saved"

# --- Case F: named failures write no OUT. -----------------------------------
DIR_F="${SCRATCH}/f"
mkdir -p "${DIR_F}"
gen_text 3000 "${DIR_F}/in.md"
cp "${DIR_F}/in.md" "${DIR_F}/in.saved"
for bad in abc 0 -5 "" 1.5; do
    export PALADIN_RELEASE_BODY_LIMIT="${bad}"
    unset PALADIN_RELEASE_BODY_RESERVE
    rm -f "${DIR_F}/out.md"
    call release_body_fit_for_create "${DIR_F}/in.md" "${DIR_F}/out.md" v1.2.3 o/r
    if [ -z "${bad}" ]; then
        # Empty means "unset": the default limit applies and a 3000-char body is simply copied.
        assert "empty limit falls back to the default and copies the body" test "${LAST_STATUS}" -eq 0
        continue
    fi
    assert "limit '${bad}' is a named failure" test "${LAST_STATUS}" -ne 0
    assert "limit '${bad}' names the variable and the rule" \
        grep -q -F "ERROR: PALADIN_RELEASE_BODY_LIMIT must be a positive integer" <<<"${LAST_ERR}"
    assert "limit '${bad}' writes no output file" test ! -e "${DIR_F}/out.md"
done
reset_limits
set_limits 2000 2000
call release_body_fit_for_create "${DIR_F}/in.md" "${DIR_F}/out.md" v1.2.3 o/r
assert "reserve >= limit is a named failure" test "${LAST_STATUS}" -ne 0
assert "reserve >= limit names PALADIN_RELEASE_BODY_RESERVE" grep -q -F "PALADIN_RELEASE_BODY_RESERVE" <<<"${LAST_ERR}"
assert "reserve >= limit writes no output file" test ! -e "${DIR_F}/out.md"
set_limits 2000 abc
call release_body_fit_for_create "${DIR_F}/in.md" "${DIR_F}/out.md" v1.2.3 o/r
assert "a non-numeric reserve is a named failure" test "${LAST_STATUS}" -ne 0
reset_limits
call release_body_fit "${DIR_F}/in.md" "${DIR_F}/out.md" 50 v1.2.3 o/r
assert "a target too small for the footer is a named failure" test "${LAST_STATUS}" -ne 0
assert "a too-small target is named in the message" grep -q -F "too small" <<<"${LAST_ERR}"
assert "a too-small target writes no output file" test ! -e "${DIR_F}/out.md"
in_unchanged "named failures" "${DIR_F}/in.md" "${DIR_F}/in.saved"

# --- Case G: invalid UTF-8 and empty input. ---------------------------------
DIR_G="${SCRATCH}/g"
mkdir -p "${DIR_G}"
printf 'valid start \377\376 invalid tail\n' > "${DIR_G}/bad.md"
cp "${DIR_G}/bad.md" "${DIR_G}/bad.saved"
set_limits 2000 500
call release_body_length "${DIR_G}/bad.md"
assert "release_body_length rejects invalid UTF-8" test "${LAST_STATUS}" -ne 0
call release_body_fit_for_create "${DIR_G}/bad.md" "${DIR_G}/out.md" v1.2.3 o/r
assert "invalid UTF-8 is a named failure" test "${LAST_STATUS}" -ne 0
assert "invalid UTF-8 names the cause" grep -q -F "ERROR: release body is not valid UTF-8" <<<"${LAST_ERR}"
assert "invalid UTF-8 writes no output file" test ! -e "${DIR_G}/out.md"
in_unchanged "invalid UTF-8" "${DIR_G}/bad.md" "${DIR_G}/bad.saved"
: > "${DIR_G}/empty.md"
call release_body_fit_for_create "${DIR_G}/empty.md" "${DIR_G}/empty.out" v1.2.3 o/r
assert "an empty body exits 0" test "${LAST_STATUS}" -eq 0
assert "an empty body gives an empty, identical output" cmp -s "${DIR_G}/empty.md" "${DIR_G}/empty.out"

# --- Case H: logging names sizes only, never body text. ---------------------
DIR_H="${SCRATCH}/h"
mkdir -p "${DIR_H}"
{
    echo "${MARKER}"
    python3 -I -c 'import sys
for i in range(60): sys.stdout.write(("filler %05d " % i).ljust(49, "q") + "\n")'
    echo "${MARKER}"
} > "${DIR_H}/in.md"
cp "${DIR_H}/in.md" "${DIR_H}/in.saved"
set_limits 2000 500
call release_body_fit_for_create "${DIR_H}/in.md" "${DIR_H}/out.md" v1.2.3 o/r
assert "marker body is truncated (exit 0)" test "${LAST_STATUS}" -eq 0
assert "stderr reports sizes" grep -q -F "release-body-fit: original" <<<"${LAST_ERR}"
refute "stderr never contains body text" grep -q -F "${MARKER}" <<<"${LAST_ERR}"
refute "stdout never contains body text" grep -q -F "${MARKER}" <<<"${LAST_OUT}"
in_unchanged "logging" "${DIR_H}/in.md" "${DIR_H}/in.saved"

# --- Case I: GitHub Actions annotation. -------------------------------------
GITHUB_ACTIONS=true call release_body_fit_for_create "${DIR_A}/in2001.md" "${DIR_A}/ann.md" v1.2.3 o/r
assert "Actions run prints the truncation warning annotation" \
    grep -q -E '^::warning title=Release body truncated::original 2001 chars, final [0-9]+ chars, limit 2000$' <<<"${LAST_OUT}"
GITHUB_ACTIONS="" call release_body_fit_for_create "${DIR_A}/in2001.md" "${DIR_A}/ann2.md" v1.2.3 o/r
refute "a non-Actions run prints no annotation" grep -q -F "::warning" <<<"${LAST_OUT}"

# --- Case J: footer without a repo omits the URL. ---------------------------
call release_body_fit_for_create "${DIR_A}/in2001.md" "${DIR_A}/norepo.md" v1.2.3 ""
assert "an empty repo omits the URL from the footer" grep -q -F "CHANGELOG.md at v1.2.3._" "${DIR_A}/norepo.md"
refute "an empty repo writes no github.com link" grep -q -F "https://github.com" "${DIR_A}/norepo.md"

# --- Case K: the CLI form applies the create-time policy. -------------------
set_limits 2000 500
"${HELPER}" --in "${DIR_A}/in2001.md" --out "${DIR_A}/cli.md" --tag v1.2.3 --repo o/r 2>"${SCRATCH}/cli_err"
CLI_STATUS=$?
assert "CLI form exits 0" test "${CLI_STATUS}" -eq 0
assert "CLI form truncates like release_body_fit_for_create" test "$(pylen "${DIR_A}/cli.md")" -le 1500
assert "CLI form logs sizes" grep -q -F "release-body-fit: original 2001 chars" "${SCRATCH}/cli_err"
"${HELPER}" --bogus x >/dev/null 2>"${SCRATCH}/cli_err"
CLI_STATUS=$?
assert "an unknown CLI flag exits 2" test "${CLI_STATUS}" -eq 2
"${HELPER}" --out "${DIR_A}/x.md" --tag v1 >/dev/null 2>"${SCRATCH}/cli_err"
CLI_STATUS=$?
assert "a missing --in exits 2" test "${CLI_STATUS}" -eq 2
reset_limits

# --- The real tree must never be mutated by this test. ----------------------
ASSERTIONS=$((ASSERTIONS + 1))
AFTER_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"
if [ "${BEFORE_STATUS}" = "${AFTER_STATUS}" ]; then
    echo "PASS (no mutation): git status --porcelain -- scripts .github/workflows is unchanged"
else
    echo "FAIL: scripts/ or .github/workflows/ was mutated by this test run:"
    echo "before: ${BEFORE_STATUS}" | sed 's/^/  | /'
    echo "after:  ${AFTER_STATUS}" | sed 's/^/  | /'
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
