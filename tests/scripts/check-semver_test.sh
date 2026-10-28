#!/usr/bin/env bash
# check-semver_test.sh
#
# Committed regression harness for scripts/check-semver.sh (CIREL-01, D-01,
# plan 52-01). Mirrors tests/scripts/create-or-reuse-release_test.sh's
# fixture-lifecycle pattern: every fixture is built under a single `mktemp -d`
# scratch directory removed on exit via a trap, the real tree is only ever
# read, and no assertion ever touches the network or builds anything.
#
# Every assertion runs the real script against a stubbed `cargo` binary (a
# shell script in the case's scratch directory, selected through the
# SEMVER_CARGO_BIN seam check-semver.sh reads). The stub appends its full argv
# to an `invocations` file and replies from `<package>.out` / `<package>.status`
# files, so each assertion can check both the verdict (exit code / output) and
# exactly which cargo-semver-checks calls were made.
#
# Cases that need a different package list point PUBLISH_CRATES_SCRIPT at a
# fixture file defining CRATES=(...). Fixtures accumulate into $FAILED rather
# than exiting on the first mismatch, matching the "report everything, don't
# short-circuit" house style of the guard scripts in this repo.
#
# The script is always run with GITHUB_ACTIONS unset unless a case sets it
# deliberately, because this harness itself runs inside GitHub Actions (the
# License & Dependency Policy job runs `make test-shell-guards`).
#
# Usage:  ./tests/scripts/check-semver_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-semver.sh"
REAL_PUBLISH_CRATES="${WORKSPACE_ROOT}/scripts/publish-crates.sh"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-semver-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# --- Real-tree mutation baseline, captured before any fixture runs. --------
MUTATION_WATCH_PATHS=(scripts .github/workflows)
BEFORE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"

# The expected package order is read from the real publish-crates.sh the same
# way the guard reads it, so this harness never carries a second hand-kept
# package list either.
EXPECTED_CRATES=()
while IFS= read -r name; do
    EXPECTED_CRATES+=("${name}")
done < <(
    # shellcheck source=scripts/publish-crates.sh
    PUBLISH_CRATES_LIB_ONLY=1 source "${REAL_PUBLISH_CRATES}"
    printf '%s\n' "${CRATES[@]}"
)

BASELINE="0.11.0"
RELEASE_TYPE="patch"
CLEAN_LINE="223 checks: 223 pass, 31 skip"

# --- Assertion helpers. ------------------------------------------------------
pass() {
    echo "PASS: $1"
}

fail() {
    echo "FAIL: $1"
    FAILED=$((FAILED + 1))
}

# assert_status DESC EXPECTED -> compares $LAST_STATUS.
assert_status() {
    local desc="$1" expected="$2"
    ASSERTIONS=$((ASSERTIONS + 1))
    if [ "${LAST_STATUS}" -eq "${expected}" ]; then
        pass "${desc} (exit ${LAST_STATUS})"
    else
        fail "${desc}: expected exit ${expected}, got ${LAST_STATUS}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    fi
}

# assert_nonzero_status DESC -> any non-zero exit.
assert_nonzero_status() {
    local desc="$1"
    ASSERTIONS=$((ASSERTIONS + 1))
    if [ "${LAST_STATUS}" -ne 0 ]; then
        pass "${desc} (exit ${LAST_STATUS})"
    else
        fail "${desc}: expected a non-zero exit, got 0"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    fi
}

# assert_contains DESC NEEDLE -> fixed-string match in $LAST_OUTPUT.
assert_contains() {
    local desc="$1" needle="$2"
    ASSERTIONS=$((ASSERTIONS + 1))
    if grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
        pass "${desc}"
    else
        fail "${desc}: output does not contain: ${needle}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    fi
}

# assert_not_contains DESC NEEDLE -> fixed-string absence in $LAST_OUTPUT.
assert_not_contains() {
    local desc="$1" needle="$2"
    ASSERTIONS=$((ASSERTIONS + 1))
    if grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
        fail "${desc}: output unexpectedly contains: ${needle}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    else
        pass "${desc}"
    fi
}

# assert_eq DESC EXPECTED ACTUAL
assert_eq() {
    local desc="$1" expected="$2" actual="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    if [ "${expected}" = "${actual}" ]; then
        pass "${desc}"
    else
        fail "${desc}: expected '${expected}', got '${actual}'"
    fi
}

# --- Fixture helpers. --------------------------------------------------------

# new_case NAME -> creates a scratch dir with a fresh cargo stub, prints its path.
new_case() {
    local dir="${SCRATCH}/case-$1"
    mkdir -p "${dir}"
    cat > "${dir}/cargo-stub.sh" <<'STUB'
#!/usr/bin/env bash
# Stand-in for `cargo`. Records its argv, then replies from <pkg>.out /
# <pkg>.status in its own directory (defaults: the clean 223-check line, 0).
DIR="$(cd "$(dirname "$0")" && pwd)"
echo "$*" >> "${DIR}/invocations"
pkg=""
while [ "$#" -gt 0 ]; do
    if [ "$1" = "--package" ]; then
        pkg="$2"
        shift
    fi
    shift
done
if [ -f "${DIR}/${pkg}.out" ]; then
    cat "${DIR}/${pkg}.out"
else
    printf '     Checked [   1.000s] 223 checks: 223 pass, 31 skip\n'
fi
status=0
[ -f "${DIR}/${pkg}.status" ] && status="$(cat "${DIR}/${pkg}.status")"
exit "${status}"
STUB
    chmod +x "${dir}/cargo-stub.sh"
    echo "${dir}"
}

# set_reply DIR PKG STATUS TEXT -> scripts the stub's reply for one package.
set_reply() {
    local dir="$1" pkg="$2" status="$3" text="$4"
    printf '%s\n' "${text}" > "${dir}/${pkg}.out"
    printf '%s\n' "${status}" > "${dir}/${pkg}.status"
}

# invocation_count DIR -> number of recorded cargo calls (0 if no file).
invocation_count() {
    local dir="$1"
    if [ -f "${dir}/invocations" ]; then
        wc -l < "${dir}/invocations" | tr -d ' '
    else
        echo 0
    fi
}

# expected_argv PKG -> the exact argv line the stub must have recorded.
expected_argv() {
    echo "semver-checks check-release --package $1 --default-features --baseline-version ${BASELINE} --release-type ${RELEASE_TYPE}"
}

# run_script DIR [ENV=VAL ...] -- ARGS... -> runs the guard with the stub
# selected and GITHUB_ACTIONS/prune/target-dir seams cleared unless an
# ENV=VAL pair before the `--` sets them. Sets $LAST_OUTPUT and $LAST_STATUS.
run_script() {
    local dir="$1"
    shift
    local envs=()
    while [ "$#" -gt 0 ] && [ "$1" != "--" ]; do
        envs+=("$1")
        shift
    done
    [ "$#" -gt 0 ] && shift
    LAST_OUTPUT="$(
        env -u GITHUB_ACTIONS -u SEMVER_PRUNE_LOCAL_CACHE -u CARGO_TARGET_DIR \
            SEMVER_CARGO_BIN="${dir}/cargo-stub.sh" \
            PUBLISH_CRATES_SCRIPT="${PUBLISH_OVERRIDE:-${REAL_PUBLISH_CRATES}}" \
            "${envs[@]}" \
            "${GUARD}" "$@" 2>&1
    )"
    LAST_STATUS=$?
}

PKG_COUNT="${#EXPECTED_CRATES[@]}"

# --- Sanity: the real CRATES list is the published set (at least twelve). ----
ASSERTIONS=$((ASSERTIONS + 1))
if [ "${PKG_COUNT}" -ge 12 ] && printf '%s\n' "${EXPECTED_CRATES[@]}" | grep -qx 'paladin-eval'; then
    pass "publish-crates.sh CRATES holds ${PKG_COUNT} packages including paladin-eval"
else
    fail "publish-crates.sh CRATES holds ${PKG_COUNT} packages; expected at least 12 including paladin-eval"
fi

# --- Case 1: every package clean -> exit 0, N lines, N exact invocations. ---
DIR="$(new_case clean)"
run_script "${DIR}" --
assert_status "clean run over every published package passes" 0
for pkg in "${EXPECTED_CRATES[@]}"; do
    assert_contains "clean run prints the per-package line for ${pkg}" "semver: ${pkg}: ${CLEAN_LINE}"
done
assert_contains "clean run prints the closing verdict line" \
    "semver: ${PKG_COUNT} package(s) checked against ${BASELINE} at --release-type ${RELEASE_TYPE}; 0 failure(s)"
EXPECTED_ARGV_FILE="${DIR}/expected_invocations"
: > "${EXPECTED_ARGV_FILE}"
for pkg in "${EXPECTED_CRATES[@]}"; do
    expected_argv "${pkg}" >> "${EXPECTED_ARGV_FILE}"
done
ASSERTIONS=$((ASSERTIONS + 1))
if diff -u "${EXPECTED_ARGV_FILE}" "${DIR}/invocations" >"${DIR}/argv.diff" 2>&1; then
    pass "every invocation carries --default-features, --baseline-version ${BASELINE} and --release-type ${RELEASE_TYPE}, in CRATES order"
else
    fail "recorded invocations differ from the expected argv list in CRATES order"
    sed 's/^/  | /' "${DIR}/argv.diff"
fi
assert_eq "clean run makes exactly one cargo call per package" "${PKG_COUNT}" "$(invocation_count "${DIR}")"
assert_not_contains "no ::group:: workflow command outside GitHub Actions" "::group::"
assert_not_contains "no ::notice annotation outside GitHub Actions" "::notice"

# --- Case 2: third package fails -> exit 1, all packages still ran. ---------
DIR="$(new_case third-fails)"
THIRD="${EXPECTED_CRATES[2]}"
set_reply "${DIR}" "${THIRD}" 100 "     Checked [   1.000s] 217 checks: 216 pass, 1 fail, 0 warn, 37 skip"
run_script "${DIR}" --
assert_status "a failing third package fails the run" 1
assert_contains "the failure names the package, its counts and the lint failure" \
    "semver: ${THIRD}: 217 checks: 216 pass, 1 fail, 0 warn, 37 skip -- lint failure (exit 100)"
assert_contains "the closing line counts one failure" \
    "semver: ${PKG_COUNT} package(s) checked against ${BASELINE} at --release-type ${RELEASE_TYPE}; 1 failure(s)"
assert_eq "a failure in the third package still runs every other package" "${PKG_COUNT}" "$(invocation_count "${DIR}")"
assert_contains "the package after the failure still has a recorded result" \
    "semver: ${EXPECTED_CRATES[3]}: ${CLEAN_LINE}"

# --- Case 3: zero checks with tool exit 0 -> named failure (T-52-01). -------
DIR="$(new_case zero-checks)"
ZERO_PKG="${EXPECTED_CRATES[1]}"
set_reply "${DIR}" "${ZERO_PKG}" 0 "     Checked [   0.100s] 0 checks: 0 pass, 254 skip"
run_script "${DIR}" --
assert_status "0 checks with tool exit 0 fails the run" 1
assert_contains "the zero-check failure is named" "semver: ${ZERO_PKG}: 0 checks: 0 pass, 254 skip -- evaluated zero checks"

# --- Case 4: one check is one step above the threshold -> passes. ----------
DIR="$(new_case one-check)"
ONE_PKG="${EXPECTED_CRATES[1]}"
set_reply "${DIR}" "${ONE_PKG}" 0 "     Checked [   0.100s] 1 checks: 1 pass, 253 skip"
run_script "${DIR}" --
assert_status "1 checks with tool exit 0 passes" 0
assert_contains "the one-check line is reported as a pass" "semver: ${ONE_PKG}: 1 checks: 1 pass, 253 skip"
assert_not_contains "the one-check line is not a failure" "evaluated zero checks"

# --- Case 5: ANSI-coloured check line parses like a plain one. -------------
DIR="$(new_case ansi)"
ANSI_PKG="${EXPECTED_CRATES[0]}"
printf '\033[32;1m     Checked\033[0m [   1.000s] \033[1m223 checks\033[0m: 223 pass, 31 skip\n' > "${DIR}/${ANSI_PKG}.out"
printf '0\n' > "${DIR}/${ANSI_PKG}.status"
run_script "${DIR}" --
assert_status "a coloured check line passes" 0
SUMMARY_LINE="$(grep -F "semver: ${ANSI_PKG}:" <<<"${LAST_OUTPUT}" | tail -1)"
assert_eq "a coloured check line parses to the same plain summary" "semver: ${ANSI_PKG}: ${CLEAN_LINE}" "${SUMMARY_LINE}"

# --- Case 6: no check line at all, tool exit 0 -> named failure. -----------
DIR="$(new_case no-line)"
NOLINE_PKG="${EXPECTED_CRATES[4]}"
set_reply "${DIR}" "${NOLINE_PKG}" 0 "Summary no semver update required"
run_script "${DIR}" --
assert_status "no check count with tool exit 0 fails the run" 1
assert_contains "the missing count is named" "semver: ${NOLINE_PKG}: reported no check count"

# --- Case 7: tool failure outranks a zero count in the classification. -----
DIR="$(new_case fail-and-zero)"
FZ_PKG="${EXPECTED_CRATES[5]}"
set_reply "${DIR}" "${FZ_PKG}" 100 "     Checked [   0.100s] 0 checks: 0 pass, 254 skip"
run_script "${DIR}" --
assert_status "a failing tool with a zero count fails the run" 1
assert_contains "the tool failure is reported first" "semver: ${FZ_PKG}: 0 checks: 0 pass, 254 skip -- lint failure (exit 100)"

# --- Case 8: empty CRATES -> named failure, zero cargo calls. --------------
DIR="$(new_case empty-crates)"
EMPTY_FIXTURE="${DIR}/publish-crates-empty.sh"
printf 'CRATES=()\n' > "${EMPTY_FIXTURE}"
PUBLISH_OVERRIDE="${EMPTY_FIXTURE}" run_script "${DIR}" --
assert_nonzero_status "an empty CRATES array fails"
assert_contains "the empty CRATES failure is named" "CRATES_EMPTY"
assert_eq "an empty CRATES array makes zero cargo calls" 0 "$(invocation_count "${DIR}")"

# --- Case 9: missing publish-crates.sh -> named failure, zero cargo calls. -
DIR="$(new_case missing-publish)"
PUBLISH_OVERRIDE="${DIR}/does-not-exist.sh" run_script "${DIR}" --
assert_nonzero_status "a missing publish-crates.sh fails"
assert_contains "the missing file failure is named" "publish-crates.sh not found"
assert_eq "a missing publish-crates.sh makes zero cargo calls" 0 "$(invocation_count "${DIR}")"

# --- Case 10: --package restricts the run to one member. -------------------
DIR="$(new_case one-package)"
run_script "${DIR}" -- --package paladin-eval
assert_status "--package paladin-eval passes on a clean stub" 0
assert_eq "--package paladin-eval makes exactly one cargo call" 1 "$(invocation_count "${DIR}")"
assert_eq "--package paladin-eval records the exact argv" "$(expected_argv paladin-eval)" "$(cat "${DIR}/invocations")"
assert_contains "--package paladin-eval prints its own per-package line" "semver: paladin-eval: ${CLEAN_LINE}"

# --- Case 11: --package outside CRATES is a usage error. -------------------
DIR="$(new_case bad-package)"
run_script "${DIR}" -- --package not-a-crate
assert_status "--package not-a-crate is a usage error" 2
assert_contains "the usage error names the package" "not-a-crate"
assert_eq "--package not-a-crate makes zero cargo calls" 0 "$(invocation_count "${DIR}")"

# --- Case 12: unknown argument / missing value are usage errors. -----------
DIR="$(new_case bad-args)"
run_script "${DIR}" -- --bogus-flag
assert_status "an unknown flag is a usage error" 2
assert_eq "an unknown flag makes zero cargo calls" 0 "$(invocation_count "${DIR}")"
run_script "${DIR}" -- --package
assert_status "--package without a value is a usage error" 2
assert_eq "--package without a value makes zero cargo calls" 0 "$(invocation_count "${DIR}")"

# --- Case 13: GitHub annotations (PD-01-5). --------------------------------
DIR="$(new_case annotations)"
A_FAIL="${EXPECTED_CRATES[2]}"
A_ZERO="${EXPECTED_CRATES[6]}"
set_reply "${DIR}" "${A_FAIL}" 100 "     Checked [   1.000s] 217 checks: 216 pass, 1 fail, 0 warn, 37 skip"
set_reply "${DIR}" "${A_ZERO}" 0 "     Checked [   0.100s] 0 checks: 0 pass, 254 skip"
run_script "${DIR}" GITHUB_ACTIONS=true --
assert_status "annotated run with two failing packages fails" 1
assert_contains "GitHub Actions run wraps each package in a ::group::" "::group::"
assert_contains "GitHub Actions run closes each ::group::" "::endgroup::"
NOTICE_COUNT="$(grep -c '^::notice title=Semver check counts' <<<"${LAST_OUTPUT}")"
assert_eq "exactly one ::notice title=Semver check counts annotation" 1 "${NOTICE_COUNT}"
NOTICE_LINE="$(grep '^::notice title=Semver check counts' <<<"${LAST_OUTPUT}" | head -1)"
ASSERTIONS=$((ASSERTIONS + 1))
if [[ "${NOTICE_LINE}" == *"semver: ${EXPECTED_CRATES[0]}: ${CLEAN_LINE}%0Asemver: ${EXPECTED_CRATES[1]}: ${CLEAN_LINE}%0A"* ]]; then
    pass "the ::notice joins every package line with %0A in CRATES order"
else
    fail "the ::notice does not join package lines with %0A: ${NOTICE_LINE}"
fi
ERROR_COUNT="$(grep -c '^::error title=Semver Checks::' <<<"${LAST_OUTPUT}")"
assert_eq "one ::error annotation per failing package" 2 "${ERROR_COUNT}"
assert_contains "the ::error names the lint-failing package" "::error title=Semver Checks::${A_FAIL}: lint failure (exit 100)"
assert_contains "the ::error names the zero-check package" "::error title=Semver Checks::${A_ZERO}: evaluated zero checks"

# --- Case 14: SEMVER_PRUNE_LOCAL_CACHE=1 clears only semver-checks/local-*.
DIR="$(new_case prune)"
FAKE_TARGET="${DIR}/target"
mkdir -p "${FAKE_TARGET}/semver-checks/local-aaa" "${FAKE_TARGET}/semver-checks/local-bbb" \
    "${FAKE_TARGET}/semver-checks/cache" "${FAKE_TARGET}/debug"
run_script "${DIR}" CARGO_TARGET_DIR="${FAKE_TARGET}" -- --package paladin-eval
assert_status "a run without the prune seam passes" 0
ASSERTIONS=$((ASSERTIONS + 1))
if [ -d "${FAKE_TARGET}/semver-checks/local-aaa" ] && [ -d "${FAKE_TARGET}/semver-checks/local-bbb" ]; then
    pass "without SEMVER_PRUNE_LOCAL_CACHE the local-* directories are left alone"
else
    fail "local-* directories were removed without SEMVER_PRUNE_LOCAL_CACHE=1"
fi
run_script "${DIR}" CARGO_TARGET_DIR="${FAKE_TARGET}" SEMVER_PRUNE_LOCAL_CACHE=1 -- --package paladin-eval
assert_status "a run with the prune seam passes" 0
ASSERTIONS=$((ASSERTIONS + 1))
if [ ! -e "${FAKE_TARGET}/semver-checks/local-aaa" ] && [ ! -e "${FAKE_TARGET}/semver-checks/local-bbb" ] \
    && [ -d "${FAKE_TARGET}/semver-checks/cache" ] && [ -d "${FAKE_TARGET}/debug" ]; then
    pass "SEMVER_PRUNE_LOCAL_CACHE=1 removes local-* and nothing else"
else
    fail "SEMVER_PRUNE_LOCAL_CACHE=1 did not remove exactly the local-* directories"
    find "${FAKE_TARGET}" | sed 's/^/  | /'
fi

# --- Case 15: library seam and the check-line parser. ----------------------
ASSERTIONS=$((ASSERTIONS + 1))
LIB_OUT="$(CHECK_SEMVER_LIB_ONLY=1 bash -c 'source "$1" && declare -F check_semver_main _cs_check_package _cs_check_line' _ "${GUARD}" 2>&1)"
if [ "$(grep -c . <<<"${LIB_OUT}")" -eq 3 ]; then
    pass "CHECK_SEMVER_LIB_ONLY=1 loads the three functions without running anything"
else
    fail "CHECK_SEMVER_LIB_ONLY=1 did not expose check_semver_main, _cs_check_package and _cs_check_line: ${LIB_OUT}"
fi
PARSE_LOG="${SCRATCH}/parse.log"
{
    printf 'noise 5 checks: not the summary\n'
    printf '\033[32;1m     Checked\033[0m [   1.0s] \033[1m217 checks\033[0m: 216 pass, 1 fail, 0 warn, 37 skip\n'
} > "${PARSE_LOG}"
PARSED="$(CHECK_SEMVER_LIB_ONLY=1 bash -c 'source "$1" && _cs_check_line "$2"' _ "${GUARD}" "${PARSE_LOG}" 2>&1)"
assert_eq "_cs_check_line returns the last check line, ANSI stripped" \
    "217 checks: 216 pass, 1 fail, 0 warn, 37 skip" "${PARSED}"
printf 'nothing here\n' > "${PARSE_LOG}"
PARSED="$(CHECK_SEMVER_LIB_ONLY=1 bash -c 'source "$1" && _cs_check_line "$2"' _ "${GUARD}" "${PARSE_LOG}" 2>&1)"
assert_eq "_cs_check_line prints nothing when no check line exists" "" "${PARSED}"

# --- Case 16: static properties of the script itself. ----------------------
ASSERTIONS=$((ASSERTIONS + 1))
HARD_CODED="$(grep -v '^[[:space:]]*#' "${GUARD}" | grep -c 'paladin-[a-z]' || true)"
if [ "${HARD_CODED}" -eq 0 ]; then
    pass "check-semver.sh carries no hard-coded package name outside comments"
else
    fail "check-semver.sh hard-codes ${HARD_CODED} line(s) naming a paladin- package"
fi
ASSERTIONS=$((ASSERTIONS + 1))
if grep -q 'PUBLISH_CRATES_LIB_ONLY=1' "${GUARD}"; then
    pass "check-semver.sh reads the package list through PUBLISH_CRATES_LIB_ONLY=1"
else
    fail "check-semver.sh does not source publish-crates.sh through PUBLISH_CRATES_LIB_ONLY=1"
fi

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
