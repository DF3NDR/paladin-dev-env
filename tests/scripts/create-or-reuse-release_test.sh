#!/usr/bin/env bash
# create-or-reuse-release_test.sh
#
# Committed regression harness for scripts/create-or-reuse-release.sh
# (PUBOPS-03, plan 20-03). Mirrors tests/scripts/check-release-consistency_test.sh's
# fixture-lifecycle pattern: every fixture is built under a single
# `mktemp -d` scratch directory removed on exit via a trap, the real tree is
# only ever read, and no assertion ever touches the network.
#
# Every assertion runs the real script against a stubbed `gh` binary (a
# shell script on a scratch PATH, selected through the GH_BIN seam
# create-or-reuse-release.sh reads) so no HTTP request ever leaves this
# machine. The stub reads a scripted HTTP status/body from files in its own
# scratch directory and appends one line per invocation to a call log, so
# assertions can check both the outcome (exit code / output) and whether a
# create call was actually attempted.
#
# Fixtures accumulate into $FAILED rather than exiting on the first
# mismatch, matching the "report everything, don't short-circuit" house
# style the guard scripts in this repo follow.
#
# Usage:  ./tests/scripts/create-or-reuse-release_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/create-or-reuse-release.sh"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/create-or-reuse-release-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# --- Real-tree mutation baseline, captured before any fixture runs. --------
MUTATION_WATCH_PATHS=(scripts .github/workflows)
BEFORE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"

# write_gh_stub DIR -> writes a scripted stand-in for the `gh` CLI into DIR
# and makes it executable. The stub recognises exactly the two endpoint
# shapes create-or-reuse-release.sh calls -- `.../releases/tags/<tag>`
# (lookup) and `.../releases` (create) -- and responds from status/body
# fixture files an assertion writes into the same DIR beforehand:
#
#   lookup_status / lookup_body        -> single-call lookup fixture
#   lookup_status_N / lookup_body_N    -> the Nth lookup call specifically
#                                          (used for the 422-then-refetch
#                                          case, where the same endpoint is
#                                          hit twice with different answers)
#   create_status / create_body        -> the (at most one) create call
#
# Every invocation appends "LOOKUP" or "CREATE" to DIR/call_log, and a
# create call's stdin (the JSON payload) is captured verbatim to
# DIR/captured_payload -- this is what proves the body-file's literal
# content (including a line that is exactly "EOF", or one with backticks
# and $(...)) reached the request unmangled.
write_gh_stub() {
    local dir="$1"
    cat > "${dir}/gh-stub.sh" <<'STUB'
#!/usr/bin/env bash
set -uo pipefail
SCRATCH_DIR="$(cd "$(dirname "$0")" && pwd)"
CALL_LOG="${SCRATCH_DIR}/call_log"

METHOD="GET"
ENDPOINT=""
HAS_INPUT=0
args=("$@")
i=0
while [ "$i" -lt "${#args[@]}" ]; do
    a="${args[$i]}"
    case "$a" in
        api) ;;
        -i|--include) ;;
        -X) i=$((i + 1)); METHOD="${args[$i]}" ;;
        --input) i=$((i + 1)); HAS_INPUT=1 ;;
        -f|-F) i=$((i + 1)) ;;
        *)
            if [[ "$a" != -* ]]; then
                ENDPOINT="$a"
            fi
            ;;
    esac
    i=$((i + 1))
done
: "${METHOD}"

if [ "${HAS_INPUT}" -eq 1 ]; then
    cat - > "${SCRATCH_DIR}/captured_payload"
fi

if [[ "${ENDPOINT}" == */releases/tags/* ]]; then
    echo "LOOKUP" >> "${CALL_LOG}"
    N_FILE="${SCRATCH_DIR}/lookup_call_n"
    N=0
    [ -f "${N_FILE}" ] && N="$(cat "${N_FILE}")"
    N=$((N + 1))
    echo "${N}" > "${N_FILE}"
    STATUS_FILE="${SCRATCH_DIR}/lookup_status_${N}"
    BODY_FILE="${SCRATCH_DIR}/lookup_body_${N}"
    [ ! -f "${STATUS_FILE}" ] && STATUS_FILE="${SCRATCH_DIR}/lookup_status"
    [ ! -f "${BODY_FILE}" ] && BODY_FILE="${SCRATCH_DIR}/lookup_body"
elif [[ "${ENDPOINT}" == */releases ]]; then
    echo "CREATE" >> "${CALL_LOG}"
    STATUS_FILE="${SCRATCH_DIR}/create_status"
    BODY_FILE="${SCRATCH_DIR}/create_body"
else
    echo "UNKNOWN endpoint: ${ENDPOINT}" >&2
    exit 1
fi

STATUS="$(cat "${STATUS_FILE}" 2>/dev/null || echo 500)"
BODY="$(cat "${BODY_FILE}" 2>/dev/null || echo '{}')"

case "${STATUS}" in
    2*) REASON="OK" ;;
    4*) REASON="Client Error" ;;
    5*) REASON="Server Error" ;;
    *) REASON="Unknown" ;;
esac
echo "HTTP/2.0 ${STATUS} ${REASON}"
echo "Content-Type: application/json"
echo ""
echo "${BODY}"

case "${STATUS}" in
    2*) exit 0 ;;
    *) exit 1 ;;
esac
STUB
    chmod +x "${dir}/gh-stub.sh"
}

# run_script DIR ARGS... -> runs the guard with GH_BIN pointed at DIR's stub
# and GITHUB_OUTPUT pointed at DIR/github_output (created fresh each call).
# Sets $LAST_OUTPUT and $LAST_STATUS.
run_script() {
    local dir="$1"
    shift
    LAST_OUTPUT="$(GH_BIN="${dir}/gh-stub.sh" GITHUB_OUTPUT="${dir}/github_output" GITHUB_REPOSITORY="" "${GUARD}" "$@" 2>&1)"
    LAST_STATUS=$?
}

# assert_fire DIR DESC NEEDLE ARGS... -> expects non-zero exit AND
# $LAST_OUTPUT to contain NEEDLE (pins which failure fired, not just that
# something did).
assert_fire() {
    local dir="$1" desc="$2" needle="$3"
    shift 3
    ASSERTIONS=$((ASSERTIONS + 1))
    run_script "${dir}" "$@"
    if [ "${LAST_STATUS}" -eq 0 ]; then
        echo "FAIL: expected non-zero exit for: ${desc} (got 0)"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
        echo "FAIL: expected output to contain '${needle}' for: ${desc}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (fire): ${desc}"
}

# assert_silent DIR DESC ARGS... -> expects zero exit AND $LAST_OUTPUT to
# contain the upload_url= line.
assert_silent() {
    local dir="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_script "${dir}" "$@"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected zero exit (silent) for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -qF -- "upload_url=" <<<"${LAST_OUTPUT}"; then
        echo "FAIL: expected output to contain 'upload_url=' for: ${desc}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (silent): ${desc}"
}

# assert_call_count DIR DESC EXPECTED_CREATE_CALLS -> checks DIR/call_log
# (written by the stub) contains exactly EXPECTED_CREATE_CALLS lines equal
# to "CREATE".
assert_call_count() {
    local dir="$1" desc="$2" expected="$3"
    local actual
    ASSERTIONS=$((ASSERTIONS + 1))
    actual="$(grep -cx 'CREATE' "${dir}/call_log" 2>/dev/null || true)"
    actual="${actual:-0}"
    if [ "${actual}" = "${expected}" ]; then
        echo "PASS (call-count): ${desc} (${actual} create call(s))"
    else
        echo "FAIL: ${desc} -- expected ${expected} create call(s), got ${actual}"
        FAILED=$((FAILED + 1))
    fi
}

# --- Case 1: lookup 200 with upload_url -> exit 0, emits it, zero create
#     calls. -------------------------------------------------------------
DIR1="${SCRATCH}/case1"
mkdir -p "${DIR1}"
write_gh_stub "${DIR1}"
echo "200" > "${DIR1}/lookup_status"
echo '{"upload_url":"https://example.com/upload-1"}' > "${DIR1}/lookup_body"
assert_silent "${DIR1}" "lookup 200 with upload_url reuses the release" \
    --tag v1.2.3 --repo test/repo
assert_call_count "${DIR1}" "lookup 200 makes no create call" 0

# --- Case 2: lookup 404, create 201 with upload_url -> exit 0, emits it,
#     exactly one create call. -------------------------------------------
DIR2="${SCRATCH}/case2"
mkdir -p "${DIR2}"
write_gh_stub "${DIR2}"
echo "404" > "${DIR2}/lookup_status"
echo '{}' > "${DIR2}/lookup_body"
echo "201" > "${DIR2}/create_status"
echo '{"upload_url":"https://example.com/upload-2"}' > "${DIR2}/create_body"
assert_silent "${DIR2}" "lookup 404 then create 201 creates the release" \
    --tag v1.2.3 --repo test/repo
assert_call_count "${DIR2}" "lookup 404 + create 201 makes exactly one create call" 1

# --- Case 3: lookup 500 -> exit non-zero, names the status, zero create
#     calls. -------------------------------------------------------------
DIR3="${SCRATCH}/case3"
mkdir -p "${DIR3}"
write_gh_stub "${DIR3}"
echo "500" > "${DIR3}/lookup_status"
echo '{}' > "${DIR3}/lookup_body"
assert_fire "${DIR3}" "lookup 500 is a hard failure naming the status" "500" \
    --tag v1.2.3 --repo test/repo
assert_call_count "${DIR3}" "lookup 500 makes no create call" 0

# --- Case 4: lookup 401 -> exit non-zero, zero create calls. -------------
DIR4="${SCRATCH}/case4"
mkdir -p "${DIR4}"
write_gh_stub "${DIR4}"
echo "401" > "${DIR4}/lookup_status"
echo '{}' > "${DIR4}/lookup_body"
assert_fire "${DIR4}" "lookup 401 is a hard failure naming the status" "401" \
    --tag v1.2.3 --repo test/repo
assert_call_count "${DIR4}" "lookup 401 makes no create call" 0

# --- Case 5a: lookup 404, create 422, re-fetch 200 -> exit 0, reuses the
#     concurrently-created release. ---------------------------------------
DIR5A="${SCRATCH}/case5a"
mkdir -p "${DIR5A}"
write_gh_stub "${DIR5A}"
echo "404" > "${DIR5A}/lookup_status_1"
echo '{}' > "${DIR5A}/lookup_body_1"
echo "200" > "${DIR5A}/lookup_status_2"
echo '{"upload_url":"https://example.com/upload-5a"}' > "${DIR5A}/lookup_body_2"
echo "422" > "${DIR5A}/create_status"
echo '{}' > "${DIR5A}/create_body"
assert_silent "${DIR5A}" "create 422 then re-fetch 200 reuses the concurrent release" \
    --tag v1.2.3 --repo test/repo
assert_call_count "${DIR5A}" "422-then-200 recovery makes exactly one create call" 1

# --- Case 5b: lookup 404, create 422, re-fetch 404 -> exit non-zero
#     (cannot resolve). ----------------------------------------------------
DIR5B="${SCRATCH}/case5b"
mkdir -p "${DIR5B}"
write_gh_stub "${DIR5B}"
echo "404" > "${DIR5B}/lookup_status_1"
echo '{}' > "${DIR5B}/lookup_body_1"
echo "404" > "${DIR5B}/lookup_status_2"
echo '{}' > "${DIR5B}/lookup_body_2"
echo "422" > "${DIR5B}/create_status"
echo '{}' > "${DIR5B}/create_body"
assert_fire "${DIR5B}" "create 422 then re-fetch 404 is a hard failure" "cannot resolve" \
    --tag v1.2.3 --repo test/repo

# --- Case 6: lookup 200 with a body lacking upload_url -> exit non-zero
#     (malformed response is a loud failure). ------------------------------
DIR6="${SCRATCH}/case6"
mkdir -p "${DIR6}"
write_gh_stub "${DIR6}"
echo "200" > "${DIR6}/lookup_status"
echo '{"no_upload_url_field": true}' > "${DIR6}/lookup_body"
assert_fire "${DIR6}" "200 with no upload_url field is a loud failure, not empty output" "upload_url" \
    --tag v1.2.3 --repo test/repo

# --- Case 7: GITHUB_OUTPUT is populated with both upload_url= and
#     version= lines after a successful run. -------------------------------
DIR7="${SCRATCH}/case7"
mkdir -p "${DIR7}"
write_gh_stub "${DIR7}"
echo "200" > "${DIR7}/lookup_status"
echo '{"upload_url":"https://example.com/upload-7"}' > "${DIR7}/lookup_body"
run_script "${DIR7}" --tag v1.2.3 --repo test/repo
ASSERTIONS=$((ASSERTIONS + 1))
if [ "${LAST_STATUS}" -eq 0 ] \
    && grep -qF -- "upload_url=https://example.com/upload-7" "${DIR7}/github_output" \
    && grep -qF -- "version=v1.2.3" "${DIR7}/github_output"; then
    echo "PASS: GITHUB_OUTPUT carries both upload_url= and version= lines"
else
    echo "FAIL: GITHUB_OUTPUT missing expected upload_url=/version= lines"
    cat "${DIR7}/github_output" 2>&1 | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Case 8: a body-file line that is exactly "EOF", and one containing
#     backticks and $(...): the value reaches the create payload literally,
#     and the run still succeeds. ------------------------------------------
DIR8="${SCRATCH}/case8"
mkdir -p "${DIR8}"
write_gh_stub "${DIR8}"
echo "404" > "${DIR8}/lookup_status"
echo '{}' > "${DIR8}/lookup_body"
echo "201" > "${DIR8}/create_status"
echo '{"upload_url":"https://example.com/upload-8"}' > "${DIR8}/create_body"
BODY_FILE8="${DIR8}/body.txt"
printf 'Some changelog line\nEOF\na line with `backticks` and $(command substitution)\nanother line\n' > "${BODY_FILE8}"
assert_silent "${DIR8}" "body-file containing a literal EOF line and shell metacharacters still succeeds" \
    --tag v1.2.3 --repo test/repo --body-file "${BODY_FILE8}"
ASSERTIONS=$((ASSERTIONS + 1))
if grep -qxF 'EOF' "${DIR8}/captured_payload" 2>/dev/null; then
    echo "FAIL: captured_payload has a raw unescaped 'EOF' line -- the value must reach the API as a JSON string field, not a raw line"
    FAILED=$((FAILED + 1))
else
    echo "PASS: captured_payload does not contain a raw bare 'EOF' line (it is JSON-escaped inside the body field)"
fi
ASSERTIONS=$((ASSERTIONS + 1))
if grep -qF '\nEOF\n' "${DIR8}/captured_payload" 2>/dev/null \
    && grep -qF 'backticks' "${DIR8}/captured_payload" 2>/dev/null \
    && grep -qF 'command substitution' "${DIR8}/captured_payload" 2>/dev/null; then
    echo "PASS: captured_payload's body field contains the literal EOF line and the backtick/\$(...) line"
else
    echo "FAIL: captured_payload does not contain the expected literal content"
    cat "${DIR8}/captured_payload" 2>&1 | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Case 9: lookup 200 with a response body larger than the pipe buffer --
#     (SC2 regression -- `printf | head -n1` under `pipefail` can abort with
#     the signal-driven exit status on a large-enough response; the fixed
#     extraction must handle this identically to a small body). The padding
#     size is computed from a named variable, not pasted inline, and sized
#     well above the ~70 KB non-trigger threshold measured in RESEARCH.md so
#     the assertion cannot pass against the broken script by chance. --------
DIR9="${SCRATCH}/case9"
mkdir -p "${DIR9}"
write_gh_stub "${DIR9}"
PADDING_SIZE=200000
PADDING="$(head -c "${PADDING_SIZE}" /dev/zero | tr '\0' 'a')"
echo "200" > "${DIR9}/lookup_status"
printf '{"upload_url":"https://example.com/upload-9","padding":"%s"}\n' "${PADDING}" > "${DIR9}/lookup_body"
assert_silent "${DIR9}" "lookup 200 with a response body larger than the pipe buffer (${PADDING_SIZE}+ chars) reuses the release" \
    --tag v1.2.3 --repo test/repo
assert_call_count "${DIR9}" "large-body lookup 200 makes no create call" 0

# --- Missing --tag: usage error. -------------------------------------------
DIR_MISSING_TAG="${SCRATCH}/case-missing-tag"
mkdir -p "${DIR_MISSING_TAG}"
write_gh_stub "${DIR_MISSING_TAG}"
assert_fire "${DIR_MISSING_TAG}" "missing --tag is a usage error" "--tag is required" \
    --repo test/repo

# --- Unknown flag: usage error, no gh call at all. --------------------------
DIR_UNKNOWN_FLAG="${SCRATCH}/case-unknown-flag"
mkdir -p "${DIR_UNKNOWN_FLAG}"
write_gh_stub "${DIR_UNKNOWN_FLAG}"
assert_fire "${DIR_UNKNOWN_FLAG}" "an unknown flag is a usage error" "unknown flag" \
    --bogus-flag foo --tag v1.2.3 --repo test/repo

# --- The real tree must never be mutated by this test: scripts/ and
#     .github/workflows/ (this guard only invokes the stubbed GH_BIN, never
#     the real `gh`, and never writes into the repo tree). ------------------
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
