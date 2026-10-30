#!/usr/bin/env bash
# extract-changelog-section_test.sh
#
# Committed regression harness for scripts/extract-changelog-section.sh
# (ARTIFACT-01, plan 21-01). Mirrors
# tests/scripts/create-or-reuse-release_test.sh's fixture-lifecycle pattern:
# every fixture lives under a single `mktemp -d` scratch directory removed
# on exit via a trap, the real tree is only ever read (never mutated), and a
# closing assertion double-checks that `git status --porcelain` over
# `scripts .github/workflows` is unchanged.
#
# Task 1 covers four assertions: extraction of the real in-tree
# CHANGELOG.md's 0.8.1-rc.3 section (non-empty, first content line matches
# the file), extraction of its 0.8.1-rc.4 section (heading-only, empty,
# D-02), a missing version (exit 1 plus the named remedy message), and one
# end-to-end chain proving the extracted text reaches
# create-or-reuse-release.sh's create payload unchanged. Task 2 extends this
# file in place with the boundary/adjacency/escaping/encoding matrix, all
# built from scratch fixtures under $SCRATCH (never the real CHANGELOG.md):
# prefix adjacency (0.8.1 vs 0.8.1-rc.2, and 0.8.1 vs 0.8.10), a heading's
# trailing " - <date>" suffix, regex-metacharacter escaping, EOF-terminated
# sections, no-gap adjacency, "## [Unreleased]" exclusion, and byte-for-byte
# UTF-8 survival.
#
# Fixtures accumulate into $FAILED rather than exiting on the first
# mismatch, matching the "report everything, don't short-circuit" house
# style the guard scripts in this repo follow.
#
# Usage:  ./tests/scripts/extract-changelog-section_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/extract-changelog-section.sh"
CREATE_OR_REUSE_RELEASE="${WORKSPACE_ROOT}/scripts/create-or-reuse-release.sh"
REAL_CHANGELOG="${WORKSPACE_ROOT}/CHANGELOG.md"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/extract-changelog-section-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# --- Real-tree mutation baseline, captured before any fixture runs. --------
MUTATION_WATCH_PATHS=(scripts .github/workflows)
BEFORE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"

# run_guard ARGS... -> sets $LAST_OUTPUT and $LAST_STATUS.
run_guard() {
    LAST_OUTPUT="$("${GUARD}" "$@" 2>&1)"
    LAST_STATUS=$?
}

# assert_fire DESC NEEDLE ARGS... -> expects non-zero exit AND $LAST_OUTPUT
# to contain NEEDLE (pins which failure fired, not just that something did).
assert_fire() {
    local desc="$1" needle="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "$@"
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

# write_gh_stub DIR -> writes a scripted stand-in for the `gh` CLI into DIR
# and makes it executable. Copied from
# tests/scripts/create-or-reuse-release_test.sh's helper of the same name --
# recognises the two endpoint shapes create-or-reuse-release.sh calls
# (`.../releases/tags/<tag>` lookup and `.../releases` create), responds
# from status/body fixture files an assertion writes into DIR beforehand,
# and captures a create call's stdin payload verbatim to
# DIR/captured_payload so an assertion can prove the extracted section
# reached the request unmangled.
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
    STATUS_FILE="${SCRATCH_DIR}/lookup_status"
    BODY_FILE="${SCRATCH_DIR}/lookup_body"
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

# --- Assertion 1: the real in-tree CHANGELOG.md's 0.8.1-rc.3 section is
#     non-empty, and its first content line matches the file. --------------
ASSERTIONS=$((ASSERTIONS + 1))
EXPECTED_FIRST_LINE="$(awk '/^## \[0\.8\.1-rc\.3\]/{f=1;next} f && NF{print;exit} f && /^## \[/{exit}' "${REAL_CHANGELOG}")"
run_guard --changelog "${REAL_CHANGELOG}" --version v0.8.1-rc.3
ACTUAL_FIRST_LINE="$(printf '%s\n' "${LAST_OUTPUT}" | awk 'NF{print;exit}')"
if [ "${LAST_STATUS}" -eq 0 ] && [ -n "${LAST_OUTPUT}" ] && [ "${ACTUAL_FIRST_LINE}" = "${EXPECTED_FIRST_LINE}" ]; then
    echo "PASS: real-tree 0.8.1-rc.3 section is non-empty and its first content line matches the file"
else
    echo "FAIL: real-tree 0.8.1-rc.3 extraction -- status=${LAST_STATUS}, expected first line '${EXPECTED_FIRST_LINE}', got '${ACTUAL_FIRST_LINE}'"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Assertion 2: the real in-tree CHANGELOG.md's 0.8.1-rc.4 section
#     (heading-only) is empty/whitespace-only and still exits 0 (D-02). ----
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${REAL_CHANGELOG}" --version 0.8.1-rc.4
if [ "${LAST_STATUS}" -eq 0 ] && [ -z "$(printf '%s' "${LAST_OUTPUT}" | tr -d '[:space:]')" ]; then
    echo "PASS: real-tree 0.8.1-rc.4 (heading-only) section is empty/whitespace-only, exit 0"
else
    echo "FAIL: real-tree 0.8.1-rc.4 extraction -- status=${LAST_STATUS}, expected empty/whitespace-only output"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Assertion 3: a missing version fails with the named remedy message. ---
assert_fire "missing version 9.9.9 fails with the named remedy message" \
    "no ## [9.9.9] section in CHANGELOG.md -- run make release VERSION=9.9.9 (finalizes changelogs) before tagging" \
    --changelog "${REAL_CHANGELOG}" --version 9.9.9

# --- Assertion 4: end-to-end chain -- extract to $SCRATCH/body.md, then run
#     create-or-reuse-release.sh --tag v9.9.9 against a stubbed gh (404
#     lookup, 201 create), and assert the captured create payload's .body
#     equals the extracted file byte-for-byte. No assertion here touches the
#     network. ------------------------------------------------------------
E2E_DIR="${SCRATCH}/e2e"
mkdir -p "${E2E_DIR}"
BODY_FILE="${E2E_DIR}/body.md"
ASSERTIONS=$((ASSERTIONS + 1))
if "${GUARD}" --changelog "${REAL_CHANGELOG}" --version v0.8.1-rc.3 --output "${BODY_FILE}" >/dev/null 2>&1; then
    echo "PASS: end-to-end extraction to \$SCRATCH/body.md succeeded"
else
    echo "FAIL: end-to-end extraction to \$SCRATCH/body.md did not succeed"
    FAILED=$((FAILED + 1))
fi

write_gh_stub "${E2E_DIR}"
echo "404" > "${E2E_DIR}/lookup_status"
echo '{}' > "${E2E_DIR}/lookup_body"
echo "201" > "${E2E_DIR}/create_status"
echo '{"upload_url":"https://example.com/upload-e2e"}' > "${E2E_DIR}/create_body"

ASSERTIONS=$((ASSERTIONS + 1))
E2E_OUTPUT="$(GH_BIN="${E2E_DIR}/gh-stub.sh" GITHUB_OUTPUT="${E2E_DIR}/github_output" GITHUB_REPOSITORY="" \
    "${CREATE_OR_REUSE_RELEASE}" --tag v9.9.9 --repo owner/name --body-file "${BODY_FILE}" 2>&1)"
E2E_STATUS=$?
if [ "${E2E_STATUS}" -eq 0 ] && grep -qF -- "upload_url=" <<<"${E2E_OUTPUT}"; then
    echo "PASS: create-or-reuse-release.sh accepted the extracted body file and created the release"
else
    echo "FAIL: create-or-reuse-release.sh did not succeed against the extracted body file"
    echo "${E2E_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
# create-or-reuse-release.sh's _cor_build_payload hands the body file to jq
# with `--rawfile` (plan 52-02, D-09), so the file's bytes reach the payload
# unchanged, trailing newlines included. (It used to read the file through
# `$(cat ...)`, which stripped trailing newlines; the byte-for-byte
# comparison below therefore reads the expected side from the raw file.)
EXPECTED_BODY_JSON="$(python3 -c 'import json,sys; print(json.dumps(open(sys.argv[1], encoding="utf-8", newline="").read()))' "${BODY_FILE}")"
ACTUAL_BODY_JSON="$(python3 -c 'import json,sys; print(json.dumps(json.load(open(sys.argv[1], encoding="utf-8"))["body"]))' "${E2E_DIR}/captured_payload" 2>/dev/null)"
if [ -n "${ACTUAL_BODY_JSON}" ] && [ "${EXPECTED_BODY_JSON}" = "${ACTUAL_BODY_JSON}" ]; then
    echo "PASS: captured create payload's .body equals the extracted file byte-for-byte"
else
    echo "FAIL: captured create payload's .body does not equal the extracted file"
    echo "  expected: ${EXPECTED_BODY_JSON}"
    echo "  actual:   ${ACTUAL_BODY_JSON}"
    FAILED=$((FAILED + 1))
fi

# --- Task 2: boundary and version-matching edge matrix ---------------------
# All fixtures below live under $SCRATCH -- never the real tree -- so the
# closing mutation-baseline assertion stays valid.
FIXTURE_DIR="${SCRATCH}/fixtures"
mkdir -p "${FIXTURE_DIR}"

# write_changelog_fixture NAME CONTENT -> writes CONTENT verbatim (no added
# trailing newline beyond what CONTENT itself carries) to
# $FIXTURE_DIR/<NAME>.md and echoes the path. Mirrors
# check-release-consistency_test.sh's write_changelog_fixture naming.
write_changelog_fixture() {
    local name="$1" content="$2"
    local path="${FIXTURE_DIR}/${name}.md"
    printf '%s' "${content}" > "${path}"
    printf '%s' "${path}"
}

# --- 5. Adjacency: 0.8.1 does not match a 0.8.1-rc.2 heading. Headings are
#     ordered 0.8.1-rc.2 then 0.8.1 so a naive prefix match would return the
#     wrong (earlier) section; assert the returned text belongs to the
#     requested 0.8.1 section, not the 0.8.1-rc.2 one. -----------------------
ADJ_RC_FIXTURE="$(write_changelog_fixture adj-rc $'## [0.8.1-rc.2] - 2026-01-01\nRC2 CONTENT LINE\n\n## [0.8.1] - 2026-01-02\nSTABLE CONTENT LINE\n')"
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${ADJ_RC_FIXTURE}" --version 0.8.1
if [ "${LAST_STATUS}" -eq 0 ] && grep -qF "STABLE CONTENT LINE" <<<"${LAST_OUTPUT}" && ! grep -qF "RC2 CONTENT LINE" <<<"${LAST_OUTPUT}"; then
    echo "PASS: 0.8.1 does not match a ## [0.8.1-rc.2] heading (prefix adjacency)"
else
    echo "FAIL: 0.8.1 matched the wrong section against a 0.8.1-rc.2/0.8.1 adjacency fixture -- status=${LAST_STATUS}"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 6. Adjacency: 0.8.1 does not match a 0.8.10 heading, and 0.8.10 does
#     not match a 0.8.1 heading. Headings ordered 0.8.10 then 0.8.1. --------
ADJ_LEN_FIXTURE="$(write_changelog_fixture adj-len $'## [0.8.10] - 2026-01-01\nTEN CONTENT LINE\n\n## [0.8.1] - 2026-01-02\nONE CONTENT LINE\n')"
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${ADJ_LEN_FIXTURE}" --version 0.8.1
if [ "${LAST_STATUS}" -eq 0 ] && grep -qF "ONE CONTENT LINE" <<<"${LAST_OUTPUT}" && ! grep -qF "TEN CONTENT LINE" <<<"${LAST_OUTPUT}"; then
    echo "PASS: 0.8.1 does not match a ## [0.8.10] heading"
else
    echo "FAIL: 0.8.1 matched the wrong section against a 0.8.10/0.8.1 adjacency fixture -- status=${LAST_STATUS}"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${ADJ_LEN_FIXTURE}" --version 0.8.10
if [ "${LAST_STATUS}" -eq 0 ] && grep -qF "TEN CONTENT LINE" <<<"${LAST_OUTPUT}" && ! grep -qF "ONE CONTENT LINE" <<<"${LAST_OUTPUT}"; then
    echo "PASS: 0.8.10 does not match a ## [0.8.1] heading"
else
    echo "FAIL: 0.8.10 matched the wrong section against a 0.8.10/0.8.1 adjacency fixture -- status=${LAST_STATUS}"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 7. A trailing " - <date>" suffix on the heading line still matches on
#     the bracketed version alone. -------------------------------------------
DATE_FIXTURE="$(write_changelog_fixture dated $'## [2.0.0] - 2026-01-01\nDATED CONTENT LINE\n')"
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${DATE_FIXTURE}" --version 2.0.0
if [ "${LAST_STATUS}" -eq 0 ] && grep -qF "DATED CONTENT LINE" <<<"${LAST_OUTPUT}"; then
    echo "PASS: a trailing ' - <date>' suffix on the heading line still matches on the bracketed version alone"
else
    echo "FAIL: dated heading did not match -- status=${LAST_STATUS}"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 8. Regex-metacharacter escaping: a version containing "." and "-" is
#     escaped, not treated as a pattern -- 0.8.1-rc.4 must never match a
#     heading whose literal text would only match if "." were a wildcard. --
ESCAPE_FIXTURE="$(write_changelog_fixture escape $'## [0X8Y1-rcZ4] - 2026-01-01\nWOULD-BE WILDCARD MATCH\n')"
assert_fire "0.8.1-rc.4 never matches 0X8Y1-rcZ4 (dots/hyphen are literal, not regex metacharacters)" \
    "no ## [0.8.1-rc.4] section in CHANGELOG.md -- run make release VERSION=0.8.1-rc.4 (finalizes changelogs) before tagging" \
    --changelog "${ESCAPE_FIXTURE}" --version 0.8.1-rc.4

# --- 9. The last section in the file stops at EOF, not at a truncation --
#     (no trailing heading, no trailing newline on the final content line). -
EOF_FIXTURE="$(write_changelog_fixture eof-stop $'## [3.0.0] - 2026-01-01\nLINE ONE\nLINE TWO\nLINE THREE')"
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${EOF_FIXTURE}" --version 3.0.0
if [ "${LAST_STATUS}" -eq 0 ] && grep -qF "LINE ONE" <<<"${LAST_OUTPUT}" && grep -qF "LINE TWO" <<<"${LAST_OUTPUT}" && grep -qF "LINE THREE" <<<"${LAST_OUTPUT}"; then
    echo "PASS: the last section in the file stops at EOF, not at a truncation"
else
    echo "FAIL: EOF-terminated section was truncated -- status=${LAST_STATUS}"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 10. A section immediately followed by another heading with no blank
#     line between yields an empty body for the first. ----------------------
NOGAP_FIXTURE="$(write_changelog_fixture no-gap $'## [4.0.0] - 2026-01-01\n## [4.0.1] - 2026-01-02\nCONTENT FOR 4.0.1\n')"
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${NOGAP_FIXTURE}" --version 4.0.0
if [ "${LAST_STATUS}" -eq 0 ] && [ -z "$(printf '%s' "${LAST_OUTPUT}" | tr -d '[:space:]')" ]; then
    echo "PASS: a section immediately followed by another heading with no blank line yields an empty body for the first"
else
    echo "FAIL: no-gap adjacency did not yield an empty section -- status=${LAST_STATUS}"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 11. "## [Unreleased]" is never selected for a numeric version --
#     confirms the boundary scan skips straight past Unreleased content to
#     the next heading rather than conflating the two sections. ------------
UNRELEASED_FIXTURE="$(write_changelog_fixture unreleased $'## [Unreleased]\nUNRELEASED CONTENT LINE\n\n## [5.0.0] - 2026-01-01\nFIVE CONTENT LINE\n')"
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --changelog "${UNRELEASED_FIXTURE}" --version 5.0.0
if [ "${LAST_STATUS}" -eq 0 ] && grep -qF "FIVE CONTENT LINE" <<<"${LAST_OUTPUT}" && ! grep -qF "UNRELEASED CONTENT LINE" <<<"${LAST_OUTPUT}"; then
    echo "PASS: ## [Unreleased] is never selected for a numeric version"
else
    echo "FAIL: numeric version extraction leaked Unreleased content -- status=${LAST_STATUS}"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 12. UTF-8 content (non-ASCII characters) survives extraction
#     byte-for-byte -- compared with cmp, not a shell string comparison. ----
UTF8_FIXTURE="$(write_changelog_fixture utf8 $'## [6.0.0] - 2026-01-01\nH\xc3\xa9llo w\xc3\xb6rld \xe2\x80\x94 emoji: \xf0\x9f\x8e\x89\n')"
UTF8_OUT="${FIXTURE_DIR}/utf8-out.md"
UTF8_EXPECTED="${FIXTURE_DIR}/utf8-expected.md"
printf '%s' $'H\xc3\xa9llo w\xc3\xb6rld \xe2\x80\x94 emoji: \xf0\x9f\x8e\x89\n' > "${UTF8_EXPECTED}"
ASSERTIONS=$((ASSERTIONS + 1))
if "${GUARD}" --changelog "${UTF8_FIXTURE}" --version 6.0.0 --output "${UTF8_OUT}" >/dev/null 2>&1 && cmp -s "${UTF8_EXPECTED}" "${UTF8_OUT}"; then
    echo "PASS: UTF-8 content survives extraction byte-for-byte"
else
    echo "FAIL: UTF-8 content did not survive extraction byte-for-byte"
    cmp "${UTF8_EXPECTED}" "${UTF8_OUT}" 2>&1 | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- The real tree must never be mutated by this test. ---------------------
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
