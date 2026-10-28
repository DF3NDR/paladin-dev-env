#!/usr/bin/env bash
# finalize-release-body_test.sh
#
# Committed regression harness for scripts/finalize-release-body.sh
# (ARTIFACT-03/ARTIFACT-04, plan 21-03). Mirrors
# tests/scripts/create-or-reuse-release_test.sh's fixture-lifecycle pattern:
# every fixture is built under a single `mktemp -d` scratch directory removed
# on exit via a trap, the real tree is only ever read, and no assertion ever
# touches the network.
#
# Two testing strategies, matched to what each behavior actually needs:
#   - Pure composition (curated text + artifact inputs -> a body file) is
#     exercised directly against compose_release_body via the
#     FINALIZE_RELEASE_BODY_LIB_ONLY=1 sourcing seam, inside a `$(...)`
#     subshell (matching tests/scripts/package-release-binaries_test.sh's
#     pattern) so the sourced script's own `set -euo pipefail` never leaks
#     into this test script's shell -- no `gh`, no stub, no network, fast
#     and exact (`cmp`/`grep -F`).
#   - The read-modify-write round trip (does a second run over its own
#     previous output stay byte-identical?) is exercised against the real
#     script, run as a fresh `bash` subprocess, with a scripted `gh` stub on
#     the scratch PATH selected through the GH_BIN seam
#     finalize-release-body.sh reads -- the stub answers
#     `release view --json body -q .body` from a fixture file and records
#     `release edit --notes-file` invocations by copying the notes file back
#     into that same fixture file, so a second run reads exactly what the
#     first run published.
#
# Fixtures accumulate into $FAILED rather than exiting on the first
# mismatch, matching the "report everything, don't short-circuit" house
# style the guard scripts in this repo follow.
#
# Usage:  ./tests/scripts/finalize-release-body_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/finalize-release-body.sh"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/finalize-release-body-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# --- Real-tree mutation baseline, captured before any fixture runs. --------
MUTATION_WATCH_PATHS=(scripts .github/workflows)
BEFORE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"

# compose_to SUBDIR NAME CURATED DIGEST IMAGE_REF SIZE_MB [ASSET_LIST]
#            [SUMS_NAME] [SBOM_ASSET] -> calls compose_release_body with the
# given inputs inside a subshell (so the sourced script's `set -euo
# pipefail` is scoped to that subshell only), writing to
# ${SCRATCH}/${SUBDIR}/${NAME}. Sets $LAST_COMPOSE_FILE. The three trailing
# arguments default to the empty string (plan 21-03's callers never pass
# them), matching plan 21-04's extended compose_release_body signature. A
# non-zero exit from the subshell surfaces as this function's own exit
# status, so a caller wanting to assert on it can check `$?` immediately
# after calling compose_to.
compose_to() {
    local subdir="$1" name="$2" curated="$3" digest="$4" image_ref="$5" size_mb="$6"
    local asset_list="${7:-}" sums_name="${8:-}" sbom_asset="${9:-}"
    mkdir -p "${SCRATCH}/${subdir}"
    LAST_COMPOSE_FILE="${SCRATCH}/${subdir}/${name}"
    (
        # shellcheck source=scripts/finalize-release-body.sh
        FINALIZE_RELEASE_BODY_LIB_ONLY=1 source "${GUARD}"
        compose_release_body "${curated}" "${digest}" "${image_ref}" "${size_mb}" \
            "${asset_list}" "${sums_name}" "${sbom_asset}" "${LAST_COMPOSE_FILE}"
    )
}

# assert_contains DESC FILE NEEDLE -> FILE must contain the literal NEEDLE.
assert_contains() {
    local desc="$1" file="$2" needle="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    if grep -qF -- "${needle}" "${file}"; then
        echo "PASS: ${desc}"
    else
        echo "FAIL: ${desc} -- expected to find '${needle}' in ${file}"
        sed 's/^/  | /' "${file}"
        FAILED=$((FAILED + 1))
    fi
}

# assert_not_contains DESC FILE NEEDLE -> FILE must NOT contain NEEDLE.
assert_not_contains() {
    local desc="$1" file="$2" needle="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    if grep -qF -- "${needle}" "${file}"; then
        echo "FAIL: ${desc} -- did not expect to find '${needle}' in ${file}"
        sed 's/^/  | /' "${file}"
        FAILED=$((FAILED + 1))
    else
        echo "PASS: ${desc}"
    fi
}

# assert_cmp DESC FILE_A FILE_B -> the two files must be byte-identical.
assert_cmp() {
    local desc="$1" file_a="$2" file_b="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    if cmp -s "${file_a}" "${file_b}"; then
        echo "PASS (cmp): ${desc}"
    else
        echo "FAIL: ${desc} -- ${file_a} and ${file_b} differ"
        diff "${file_a}" "${file_b}" 2>&1 | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
    fi
}

CURATED1="## [1.2.3] - 2026-01-01

### Added

- A feature.
"

# --- Case 1: no marker, digest + ref + size present -> curated unchanged,
#     marker, a container-image section with a runnable pull line, and an
#     advisory image-size section. -------------------------------------------
compose_to case1 body1.md "${CURATED1}" "abc123" "ghcr.io/df3ndr/paladin-dev-env:1.2.3" "450"
BODY1="${LAST_COMPOSE_FILE}"
assert_contains "case1: curated text preserved" "${BODY1}" "### Added"
assert_contains "case1: marker present" "${BODY1}" "<!-- paladin:release-artifacts -->"
assert_contains "case1: pull line pinned to digest" "${BODY1}" 'docker pull ghcr.io/df3ndr/paladin-dev-env@sha256:abc123'
assert_contains "case1: image size section present" "${BODY1}" "### Image size"
assert_contains "case1: 450 MB reads as within target" "${BODY1}" "within target"

# --- Case 2: digest already carries the sha256: prefix -> exactly one
#     prefix reaches the pull line, never doubled. ---------------------------
compose_to case2 body2.md "${CURATED1}" "sha256:def456" "ghcr.io/df3ndr/paladin-dev-env:1.2.3" ""
BODY2="${LAST_COMPOSE_FILE}"
assert_contains "case2: pre-prefixed digest used verbatim" "${BODY2}" '@sha256:def456'
assert_not_contains "case2: digest is never double-prefixed" "${BODY2}" 'sha256:sha256:'

# --- Case 3: no digest and no image ref -> no container-image section at
#     all, even though a size was supplied. -----------------------------------
compose_to case3 body3.md "${CURATED1}" "" "" "300"
BODY3="${LAST_COMPOSE_FILE}"
assert_not_contains "case3: no digest/ref -> no container-image section" "${BODY3}" "### Container image"
assert_contains "case3: image size section still present" "${BODY3}" "### Image size"

# --- Case 4: no artifact inputs at all -> states no artifacts were
#     recorded, never an empty heading. ---------------------------------------
compose_to case4 body4.md "${CURATED1}" "" "" ""
BODY4="${LAST_COMPOSE_FILE}"
assert_contains "case4: no inputs -> explicit no-artifacts statement" "${BODY4}" "No artifacts were recorded for this run."
assert_not_contains "case4: no container-image section" "${BODY4}" "### Container image"
assert_not_contains "case4: no image-size section" "${BODY4}" "### Image size"
assert_not_contains "case4: no downloads-and-verification section" "${BODY4}" "### Downloads and verification"
assert_not_contains "case4: no SBOM section when no sbom asset supplied" "${BODY4}" "### SBOM"

# --- Case 5: size exactly 500 reads as within target; 501 reads as over
#     target -- neither changes the composer's own exit status. --------------
compose_to case5a body5a.md "${CURATED1}" "" "" "500"
assert_contains "case5a: 500 MB reads as within target" "${LAST_COMPOSE_FILE}" "within target"
compose_to case5b body5b.md "${CURATED1}" "" "" "501"
assert_contains "case5b: 501 MB reads as over target" "${LAST_COMPOSE_FILE}" "over target"

# --- Case 6: read-modify-write round trip via the real script + a stubbed
#     `gh` -- running the composer a second time over its own previous
#     output produces a byte-identical result. --------------------------------
# write_gh_stub DIR -> writes a scripted `gh` stand-in to DIR/gh-stub.sh
# answering four verbs against DIR-scoped fixture files:
#   - `release view TAG --json body -q .body`   -> DIR/current_body
#   - `release view TAG --json assets -q ...`   -> DIR/assets_registry
#     (one asset name per line; empty/absent when not seeded -- most
#     existing fixtures never create this file, so the asset list a caller
#     that doesn't seed it observes is empty, matching pre-21-04 behaviour)
#   - `release edit TAG --notes-file PATH`      -> copies PATH back into
#     DIR/current_body, so a second run reads exactly what the first run
#     published (the read-modify-write round trip)
#   - `release download TAG --pattern ... --dir DEST` -> when
#     DIR/download_source contains at least one file, copies every file from
#     it into DEST, unfiltered -- the pattern-matching semantics under test
#     belong to aggregate_checksums itself, not to this stub, so a stray
#     non-archive fixture file proves the real filtering code, not an
#     assumption baked into the stub. When DIR/download_source is absent or
#     empty, mirrors the real CLI's "no assets match the file pattern"
#     failure: writes nothing to DEST and exits 1 (case 15 depends on this)
#   - `release upload TAG PATH --clobber`       -> records the call, copies
#     PATH into DIR/uploaded_<basename>, and appends <basename> to
#     DIR/assets_registry exactly once (a re-upload of the same name is not
#     duplicated) -- so aggregation's own SHA256SUMS upload becomes visible
#     to a subsequent `release view --json assets` call in the same run,
#     proving aggregation really does run before the asset list is read
write_gh_stub() {
    local dir="$1"
    cat > "${dir}/gh-stub.sh" <<'STUB'
#!/usr/bin/env bash
set -uo pipefail
SCRATCH_DIR="$(cd "$(dirname "$0")" && pwd)"
CALL_LOG="${SCRATCH_DIR}/call_log"

# _stub_flag_value FLAG ARGS... -> prints the value following the first
# occurrence of FLAG among ARGS, or nothing if FLAG is absent.
_stub_flag_value() {
    local flag="$1"
    shift
    local args=("$@")
    local i=0
    while [ "$i" -lt "${#args[@]}" ]; do
        if [ "${args[$i]}" = "${flag}" ]; then
            i=$((i + 1))
            printf '%s' "${args[$i]:-}"
            return 0
        fi
        i=$((i + 1))
    done
}

if [ "${1:-}" = "release" ] && [ "${2:-}" = "view" ]; then
    JSON_FIELD="$(_stub_flag_value --json "$@")"
    if [ "${JSON_FIELD}" = "assets" ]; then
        echo "VIEW_ASSETS" >> "${CALL_LOG}"
        cat "${SCRATCH_DIR}/assets_registry" 2>/dev/null || true
    else
        echo "VIEW" >> "${CALL_LOG}"
        cat "${SCRATCH_DIR}/current_body" 2>/dev/null || true
    fi
    exit 0
elif [ "${1:-}" = "release" ] && [ "${2:-}" = "edit" ]; then
    echo "EDIT" >> "${CALL_LOG}"
    NOTES_FILE="$(_stub_flag_value --notes-file "$@")"
    if [ -n "${NOTES_FILE}" ]; then
        cp "${NOTES_FILE}" "${SCRATCH_DIR}/current_body"
    fi
    exit 0
elif [ "${1:-}" = "release" ] && [ "${2:-}" = "download" ]; then
    echo "DOWNLOAD" >> "${CALL_LOG}"
    DEST_DIR="$(_stub_flag_value --dir "$@")"
    SOURCE_COUNT=0
    if [ -d "${SCRATCH_DIR}/download_source" ]; then
        SOURCE_COUNT="$(find "${SCRATCH_DIR}/download_source" -maxdepth 1 -type f | wc -l | tr -d ' ')"
    fi
    # Mirror the real `gh release download` CLI: when nothing on the release
    # matches the requested pattern, it exits non-zero and writes nothing to
    # DEST_DIR (verified live: `gh release download ... --pattern
    # '*.definitely-not-a-real-extension-xyz'` prints "no assets match the
    # file pattern" and exits 1). Case 15 depends on this to prove
    # aggregate_checksums survives the real failure mode, not a stub that
    # always succeeds.
    if [ "${SOURCE_COUNT}" = "0" ]; then
        echo "no assets match the file pattern" >&2
        exit 1
    fi
    mkdir -p "${DEST_DIR}"
    find "${SCRATCH_DIR}/download_source" -maxdepth 1 -type f -exec cp {} "${DEST_DIR}/" \;
    exit 0
elif [ "${1:-}" = "release" ] && [ "${2:-}" = "upload" ]; then
    # $3 is the release tag; $4 is the first uploaded file path (aggregation
    # always uploads exactly one file per call: `release upload TAG PATH
    # --clobber`).
    UPLOAD_PATH="${4:-}"
    BASE_NAME="$(basename "${UPLOAD_PATH}" 2>/dev/null || true)"
    echo "UPLOAD ${BASE_NAME}" >> "${CALL_LOG}"
    echo "$*" >> "${SCRATCH_DIR}/upload_args_log"
    if [ -n "${UPLOAD_PATH}" ] && [ -f "${UPLOAD_PATH}" ]; then
        cp "${UPLOAD_PATH}" "${SCRATCH_DIR}/uploaded_${BASE_NAME}"
        if ! grep -qxF "${BASE_NAME}" "${SCRATCH_DIR}/assets_registry" 2>/dev/null; then
            echo "${BASE_NAME}" >> "${SCRATCH_DIR}/assets_registry"
        fi
    fi
    exit 0
else
    echo "UNKNOWN gh invocation: $*" >&2
    exit 1
fi
STUB
    chmod +x "${dir}/gh-stub.sh"
}

DIR6="${SCRATCH}/case6"
mkdir -p "${DIR6}"
write_gh_stub "${DIR6}"
printf '%s' "${CURATED1}" > "${DIR6}/current_body"

ASSERTIONS=$((ASSERTIONS + 1))
if FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${DIR6}/gh-stub.sh" bash "${GUARD}" \
    --tag v1.2.3 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.3" --image-size-mb 450 \
    --output "${DIR6}/output_run1.md" >"${DIR6}/run1.log" 2>&1; then
    echo "PASS: case6 first finalize run exits 0"
else
    echo "FAIL: case6 first finalize run did not exit 0"
    sed 's/^/  | /' "${DIR6}/run1.log"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
if FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${DIR6}/gh-stub.sh" bash "${GUARD}" \
    --tag v1.2.3 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.3" --image-size-mb 450 \
    --output "${DIR6}/output_run2.md" >"${DIR6}/run2.log" 2>&1; then
    echo "PASS: case6 second finalize run exits 0"
else
    echo "FAIL: case6 second finalize run did not exit 0"
    sed 's/^/  | /' "${DIR6}/run2.log"
    FAILED=$((FAILED + 1))
fi

assert_cmp "case6: second run over its own previous output is byte-identical to the first" \
    "${DIR6}/output_run1.md" "${DIR6}/output_run2.md"

ASSERTIONS=$((ASSERTIONS + 1))
EDIT_CALLS="$(grep -cx 'EDIT' "${DIR6}/call_log" 2>/dev/null || true)"
EDIT_CALLS="${EDIT_CALLS:-0}"
if [ "${EDIT_CALLS}" = "2" ]; then
    echo "PASS (call-count): case6 makes exactly one release-edit call per run (2 total)"
else
    echo "FAIL: case6 expected 2 release-edit calls, got ${EDIT_CALLS}"
    FAILED=$((FAILED + 1))
fi

# run_finalize DIR OUTPUT_NAME ARGS... -> runs the real script (a fresh
# process, so its own `set -euo pipefail` never touches this test script)
# against DIR's gh stub, writing to ${DIR}/${OUTPUT_NAME}. Returns the
# script's exit status; stdout+stderr land in ${DIR}/${OUTPUT_NAME}.log.
run_finalize() {
    local dir="$1" output_name="$2"
    shift 2
    FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${dir}/gh-stub.sh" bash "${GUARD}" \
        "$@" --output "${dir}/${output_name}" >"${dir}/${output_name}.log" 2>&1
}

# --- Case 7: a curated section containing `&`, `%`, `$(...)`, backticks and
#     a line that is exactly `EOF` survives the truncate-and-rebuild
#     byte-for-byte -- built from a quoted heredoc so the fixture itself is
#     never expanded by this test script's own shell. Proven both by direct
#     substring checks (including an exact whole-line match on the bare
#     `EOF` line) and by a second run over the first run's own output
#     staying byte-identical -- if the metacharacters had perturbed the
#     marker cut or been mangled in transit, the two runs would diverge. ----
CURATED_META="$(cat <<'META_FIXTURE'
## [1.2.4] - 2026-01-02

### Changed

- 100% of requests use the `&`-joined query string; price is $50.
- Running `echo hi` then a subshell like $(rm -rf /tmp/nope) must never execute.
EOF
- The line above is exactly the four characters "EOF" on its own line.
META_FIXTURE
)"
DIR7="${SCRATCH}/case7"
mkdir -p "${DIR7}"
write_gh_stub "${DIR7}"
printf '%s' "${CURATED_META}" > "${DIR7}/current_body"

ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR7}" "run1.md" --tag v1.2.4 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.4" --image-size-mb 100; then
    echo "PASS: case7 (metacharacter fixture) first run exits 0"
else
    echo "FAIL: case7 first run did not exit 0"
    sed 's/^/  | /' "${DIR7}/run1.md.log"
    FAILED=$((FAILED + 1))
fi
assert_contains "case7: literal & survives" "${DIR7}/run1.md" '`&`-joined'
assert_contains "case7: literal % survives" "${DIR7}/run1.md" '100%'
assert_contains "case7: literal \$(...) survives unexecuted" "${DIR7}/run1.md" '$(rm -rf /tmp/nope)'
assert_contains "case7: backticked command survives" "${DIR7}/run1.md" '`echo hi`'
ASSERTIONS=$((ASSERTIONS + 1))
if grep -qxF 'EOF' "${DIR7}/run1.md"; then
    echo "PASS: case7 a line that is exactly EOF survives as its own whole line"
else
    echo "FAIL: expected a whole line matching exactly 'EOF' in ${DIR7}/run1.md"
    FAILED=$((FAILED + 1))
fi
if run_finalize "${DIR7}" "run2.md" --tag v1.2.4 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.4" --image-size-mb 100; then
    :
else
    echo "FAIL: case7 second run did not exit 0"
    FAILED=$((FAILED + 1))
fi
assert_cmp "case7: second run over its own previous output is byte-identical to the first" \
    "${DIR7}/run1.md" "${DIR7}/run2.md"

# --- Case 8: a curated section containing multi-byte UTF-8 characters
#     survives byte-for-byte, proven the same way as Case 7. ----------------
CURATED_UTF8="$(cat <<'UTF8_FIXTURE'
## [1.2.5] - 2026-01-03

### Changed

- Endpoint moved Singapore -> US -- then back again, per the DashScope région
  note. Emoji check: caffè, naïve, 日本語, and an em dash — right here.
UTF8_FIXTURE
)"
DIR8="${SCRATCH}/case8"
mkdir -p "${DIR8}"
write_gh_stub "${DIR8}"
printf '%s' "${CURATED_UTF8}" > "${DIR8}/current_body"

ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR8}" "run1.md" --tag v1.2.5; then
    echo "PASS: case8 (UTF-8 fixture) first run exits 0"
else
    echo "FAIL: case8 first run did not exit 0"
    sed 's/^/  | /' "${DIR8}/run1.md.log"
    FAILED=$((FAILED + 1))
fi
assert_contains "case8: accented/multi-byte text survives" "${DIR8}/run1.md" 'caffè, naïve, 日本語'
assert_contains "case8: em dash survives" "${DIR8}/run1.md" 'em dash — right here'
run_finalize "${DIR8}" "run2.md" --tag v1.2.5
assert_cmp "case8: second run over its own previous output is byte-identical to the first" \
    "${DIR8}/run1.md" "${DIR8}/run2.md"

# --- Case 9: an empty curated body (the heading-only-changelog case) still
#     produces a valid body -- marker plus artifact sections, no crash, no
#     phantom content. ---------------------------------------------------------
DIR9="${SCRATCH}/case9"
mkdir -p "${DIR9}"
write_gh_stub "${DIR9}"
printf '' > "${DIR9}/current_body"

ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR9}" "run1.md" --tag v1.2.6 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.6" --image-size-mb 42; then
    echo "PASS: case9 (empty curated body) run exits 0, no crash"
else
    echo "FAIL: case9 run did not exit 0"
    sed 's/^/  | /' "${DIR9}/run1.md.log"
    FAILED=$((FAILED + 1))
fi
assert_contains "case9: marker still present" "${DIR9}/run1.md" "<!-- paladin:release-artifacts -->"
assert_contains "case9: container-image section still present" "${DIR9}/run1.md" "### Container image"
assert_not_contains "case9: no phantom 'null' text" "${DIR9}/run1.md" "null"

# --- Case 10: a body that already contains the marker twice (a hand-edited
#     release) truncates at the first occurrence, so no stale artifact
#     block survives below the rebuilt one. -----------------------------------
DIR10="${SCRATCH}/case10"
mkdir -p "${DIR10}"
write_gh_stub "${DIR10}"
printf '%s' "${CURATED1}" > "${DIR10}/current_body"
run_finalize "${DIR10}" "run1.md" --tag v1.2.3 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.3" --image-size-mb 450
# Hand-insert a second marker plus a stale artifact block below the first
# run's own output -- simulating a maintainer editing the published release
# in the GitHub UI between runs.
{
    cat "${DIR10}/run1.md"
    printf '\n<!-- paladin:release-artifacts -->\n\nSTALE HAND-EDITED CONTENT THAT MUST NOT SURVIVE\n'
} > "${DIR10}/current_body"

ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR10}" "run2.md" --tag v1.2.3 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.3" --image-size-mb 450; then
    echo "PASS: case10 (double-marker fixture) run exits 0"
else
    echo "FAIL: case10 run did not exit 0"
    sed 's/^/  | /' "${DIR10}/run2.md.log"
    FAILED=$((FAILED + 1))
fi
ASSERTIONS=$((ASSERTIONS + 1))
MARKER_COUNT="$(grep -cF '<!-- paladin:release-artifacts -->' "${DIR10}/run2.md")"
if [ "${MARKER_COUNT}" = "1" ]; then
    echo "PASS: case10 exactly one marker occurrence survives truncate-and-rebuild"
else
    echo "FAIL: case10 expected exactly 1 marker occurrence, got ${MARKER_COUNT}"
    FAILED=$((FAILED + 1))
fi
assert_not_contains "case10: stale hand-edited content is discarded, not preserved" \
    "${DIR10}/run2.md" "STALE HAND-EDITED CONTENT"

# --- Case 11: a digest present with no image reference emits no
#     container-image section -- a half-populated section is never
#     advertised (exercised through the full script, not just the pure
#     composer, to prove the CLI-flag path carries the same rule). -----------
compose_to case11 body11.md "${CURATED1}" "abc123" "" "300"
assert_not_contains "case11: digest with no image-ref -> no container-image section" \
    "${LAST_COMPOSE_FILE}" "### Container image"
assert_contains "case11: image size section still present" "${LAST_COMPOSE_FILE}" "### Image size"

# --- Case 12: three consecutive composes over the same inputs are
#     byte-identical to each other -- compose_release_body is a pure
#     function, so this must hold trivially; asserted explicitly as a
#     regression guard. ---------------------------------------------------
compose_to case12 run_a.md "${CURATED1}" "abc123" "ghcr.io/df3ndr/paladin-dev-env:1.2.3" "450"
RUN_A="${LAST_COMPOSE_FILE}"
compose_to case12 run_b.md "${CURATED1}" "abc123" "ghcr.io/df3ndr/paladin-dev-env:1.2.3" "450"
RUN_B="${LAST_COMPOSE_FILE}"
compose_to case12 run_c.md "${CURATED1}" "abc123" "ghcr.io/df3ndr/paladin-dev-env:1.2.3" "450"
RUN_C="${LAST_COMPOSE_FILE}"
assert_cmp "case12: compose #1 and #2 over identical inputs are byte-identical" "${RUN_A}" "${RUN_B}"
assert_cmp "case12: compose #2 and #3 over identical inputs are byte-identical" "${RUN_B}" "${RUN_C}"

# --- Case 13: a composed body's section order is stable when the CLI flags
#     that supply its inputs are given in a different order. -----------------
DIR13="${SCRATCH}/case13"
mkdir -p "${DIR13}"
write_gh_stub "${DIR13}"
printf '%s' "${CURATED1}" > "${DIR13}/current_body"
run_finalize "${DIR13}" "order_a.md" --tag v1.2.3 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.3" --image-size-mb 450
# Reset current_body (run_finalize's own stub overwrites it with the
# previous call's published output) so the second invocation starts from
# the identical pre-marker state as the first.
printf '%s' "${CURATED1}" > "${DIR13}/current_body"
run_finalize "${DIR13}" "order_b.md" --image-size-mb 450 --image-ref \
    "ghcr.io/df3ndr/paladin-dev-env:1.2.3" --tag v1.2.3 --image-digest abc123
assert_cmp "case13: section order is stable regardless of CLI flag order" \
    "${DIR13}/order_a.md" "${DIR13}/order_b.md"

# --- Missing --tag: usage error. ---------------------------------------------
ASSERTIONS=$((ASSERTIONS + 1))
DIR_MISSING_TAG="${SCRATCH}/case-missing-tag"
mkdir -p "${DIR_MISSING_TAG}"
MISSING_TAG_OUTPUT="$(FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${DIR6}/gh-stub.sh" bash "${GUARD}" --image-digest abc123 2>&1)"
MISSING_TAG_STATUS=$?
if [ "${MISSING_TAG_STATUS}" -ne 0 ] && grep -qF -- "--tag is required" <<<"${MISSING_TAG_OUTPUT}"; then
    echo "PASS (fire): missing --tag is a usage error"
else
    echo "FAIL: expected a non-zero exit naming '--tag is required'"
    echo "${MISSING_TAG_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Unknown flag: usage error, no gh call at all. ---------------------------
ASSERTIONS=$((ASSERTIONS + 1))
UNKNOWN_FLAG_OUTPUT="$(FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${DIR6}/gh-stub.sh" bash "${GUARD}" --tag v1.2.3 --bogus-flag foo 2>&1)"
UNKNOWN_FLAG_STATUS=$?
if [ "${UNKNOWN_FLAG_STATUS}" -ne 0 ] && grep -qF -- "unknown flag" <<<"${UNKNOWN_FLAG_OUTPUT}"; then
    echo "PASS (fire): an unknown flag is a usage error"
else
    echo "FAIL: expected a non-zero exit naming the unknown flag"
    echo "${UNKNOWN_FLAG_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Case 14: aggregation writes one SHA256SUMS covering three downloaded
#     archives, sorted by filename under LC_ALL=C, uploads it with
#     --clobber, and the resulting body lists the assets -- including the
#     sums file itself, proving aggregation ran before the asset list was
#     read -- plus a one-command verification block naming both the Linux
#     and macOS forms, and a SBOM section stating its real scope. ------------
DIR14="${SCRATCH}/case14"
mkdir -p "${DIR14}/download_source"
write_gh_stub "${DIR14}"
printf '%s' "${CURATED1}" > "${DIR14}/current_body"
printf 'binary-content-a' > "${DIR14}/download_source/paladin-linux-amd64.tar.gz"
printf 'binary-content-b' > "${DIR14}/download_source/paladin-linux-arm64.tar.gz"
printf 'binary-content-c' > "${DIR14}/download_source/paladin-macos-amd64.tar.gz"
printf 'paladin-linux-amd64.tar.gz\npaladin-linux-arm64.tar.gz\npaladin-macos-amd64.tar.gz\n' \
    > "${DIR14}/assets_registry"
ASSETS_DIR14="${DIR14}/assets"

ASSERTIONS=$((ASSERTIONS + 1))
if FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${DIR14}/gh-stub.sh" bash "${GUARD}" \
    --tag v2.0.0 --aggregate-checksums --assets-dir "${ASSETS_DIR14}" \
    --sbom-asset "paladin-v2.0.0.cdx.json" \
    --output "${DIR14}/run1.md" >"${DIR14}/run1.log" 2>&1; then
    echo "PASS: case14 aggregation run exits 0"
else
    echo "FAIL: case14 aggregation run did not exit 0"
    sed 's/^/  | /' "${DIR14}/run1.log"
    FAILED=$((FAILED + 1))
fi

SUMS14="${ASSETS_DIR14}/SHA256SUMS"
ASSERTIONS=$((ASSERTIONS + 1))
if [ -f "${SUMS14}" ]; then
    LINE_COUNT14="$(wc -l < "${SUMS14}" | tr -d ' ')"
    if [ "${LINE_COUNT14}" = "3" ]; then
        echo "PASS: case14 SHA256SUMS has exactly 3 lines"
    else
        echo "FAIL: case14 expected 3 lines in SHA256SUMS, got ${LINE_COUNT14}"
        FAILED=$((FAILED + 1))
    fi
else
    echo "FAIL: case14 SHA256SUMS was not written"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
NAMES14="$(awk '{print $2}' "${SUMS14}" 2>/dev/null)"
EXPECTED14="$(printf 'paladin-linux-amd64.tar.gz\npaladin-linux-arm64.tar.gz\npaladin-macos-amd64.tar.gz')"
if [ "${NAMES14}" = "${EXPECTED14}" ]; then
    echo "PASS: case14 SHA256SUMS lines are sorted by filename under LC_ALL=C"
else
    echo "FAIL: case14 unexpected SHA256SUMS filename order:"
    echo "${NAMES14}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

assert_contains "case14: SHA256SUMS appears in the published asset list" "${DIR14}/run1.md" '`SHA256SUMS`'
assert_contains "case14: verification block names the Linux form" "${DIR14}/run1.md" 'sha256sum -c SHA256SUMS'
assert_contains "case14: verification block names the macOS form" "${DIR14}/run1.md" 'shasum -a 256 -c SHA256SUMS'
assert_contains "case14: SBOM section states root paladin-ai package scope" "${DIR14}/run1.md" 'root `paladin-ai` package'
assert_not_contains "case14: SBOM section never claims workspace-wide coverage" "${DIR14}/run1.md" 'cover the workspace'
assert_not_contains "case14: per-crate changelogs are never inlined or linked" "${DIR14}/run1.md" 'CHANGELOG'

ASSERTIONS=$((ASSERTIONS + 1))
UPLOAD_SUMS_CALLS14="$(grep -cx 'UPLOAD SHA256SUMS' "${DIR14}/call_log" 2>/dev/null || true)"
UPLOAD_SUMS_CALLS14="${UPLOAD_SUMS_CALLS14:-0}"
if [ "${UPLOAD_SUMS_CALLS14}" = "1" ]; then
    echo "PASS (call-count): case14 uploads SHA256SUMS exactly once"
else
    echo "FAIL: case14 expected exactly 1 SHA256SUMS upload, got ${UPLOAD_SUMS_CALLS14}"
    FAILED=$((FAILED + 1))
fi

assert_contains "case14: SHA256SUMS upload passes --clobber" "${DIR14}/upload_args_log" "SHA256SUMS --clobber"

# --- Case 15: zero downloaded archives -> no SHA256SUMS is written, no
#     upload is attempted, and the composed body carries no verification
#     section (an empty checksum file must never be attached). -------------
DIR15="${SCRATCH}/case15"
mkdir -p "${DIR15}/download_source"
write_gh_stub "${DIR15}"
printf '%s' "${CURATED1}" > "${DIR15}/current_body"
ASSETS_DIR15="${DIR15}/assets"

ASSERTIONS=$((ASSERTIONS + 1))
if FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${DIR15}/gh-stub.sh" bash "${GUARD}" \
    --tag v2.0.1 --aggregate-checksums --assets-dir "${ASSETS_DIR15}" \
    --output "${DIR15}/run1.md" >"${DIR15}/run1.log" 2>&1; then
    echo "PASS: case15 zero-archive aggregation run exits 0 (not a failure)"
else
    echo "FAIL: case15 zero-archive aggregation run did not exit 0"
    sed 's/^/  | /' "${DIR15}/run1.log"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
if [ -f "${ASSETS_DIR15}/SHA256SUMS" ]; then
    echo "FAIL: case15 SHA256SUMS should not exist when zero archives were downloaded"
    FAILED=$((FAILED + 1))
else
    echo "PASS: case15 no SHA256SUMS written for zero downloaded archives"
fi

ASSERTIONS=$((ASSERTIONS + 1))
UPLOAD_SUMS_CALLS15="$(grep -cx 'UPLOAD SHA256SUMS' "${DIR15}/call_log" 2>/dev/null || true)"
UPLOAD_SUMS_CALLS15="${UPLOAD_SUMS_CALLS15:-0}"
if [ "${UPLOAD_SUMS_CALLS15}" = "0" ]; then
    echo "PASS: case15 no SHA256SUMS upload attempted for zero downloaded archives"
else
    echo "FAIL: case15 expected 0 SHA256SUMS uploads, got ${UPLOAD_SUMS_CALLS15}"
    FAILED=$((FAILED + 1))
fi

assert_not_contains "case15: no verification section when no sums file was attached" "${DIR15}/run1.md" "sha256sum -c"

# --- Case 16: the downloads section lists exactly the asset names the
#     release reports, in stable LC_ALL=C sorted order -- even though the
#     underlying registry was seeded out of order. --------------------------
DIR16="${SCRATCH}/case16"
mkdir -p "${DIR16}"
write_gh_stub "${DIR16}"
printf '%s' "${CURATED1}" > "${DIR16}/current_body"
printf 'zeta.tar.gz\nalpha.tar.gz\nbeta.tar.gz\n' > "${DIR16}/assets_registry"

ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR16}" "run1.md" --tag v2.0.2; then
    echo "PASS: case16 finalize run (no aggregation) exits 0"
else
    echo "FAIL: case16 finalize run did not exit 0"
    sed 's/^/  | /' "${DIR16}/run1.md.log"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
DOWNLOAD_LINES16="$(grep -E '^- `' "${DIR16}/run1.md" | sed -E 's/^- `([^`]+)`$/\1/')"
EXPECTED16="$(printf 'alpha.tar.gz\nbeta.tar.gz\nzeta.tar.gz')"
if [ "${DOWNLOAD_LINES16}" = "${EXPECTED16}" ]; then
    echo "PASS: case16 downloads section lists assets in stable LC_ALL=C sorted order"
else
    echo "FAIL: case16 expected sorted asset order, got:"
    echo "${DOWNLOAD_LINES16}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Case 17: the SBOM line names the attached document and states its
#     real scope via the pure composer directly. -----------------------------
compose_to case17 body17.md "${CURATED1}" "" "" "" "" "" "paladin-v2.0.0.cdx.json"
BODY17="${LAST_COMPOSE_FILE}"
assert_contains "case17: SBOM section names the attached document" "${BODY17}" '`paladin-v2.0.0.cdx.json`'
assert_contains "case17: SBOM section states root paladin-ai package scope" "${BODY17}" 'root `paladin-ai` package'

# --- Case 18: composing twice over the same release state -- with
#     aggregation, an existing asset registry, and a SBOM asset all
#     active at once -- is byte-identical. -----------------------------------
DIR18="${SCRATCH}/case18"
mkdir -p "${DIR18}/download_source"
write_gh_stub "${DIR18}"
printf '%s' "${CURATED1}" > "${DIR18}/current_body"
printf 'binary-content-x' > "${DIR18}/download_source/paladin-linux-amd64.tar.gz"
printf 'binary-content-y' > "${DIR18}/download_source/paladin-linux-arm64.tar.gz"
printf 'paladin-linux-amd64.tar.gz\npaladin-linux-arm64.tar.gz\n' > "${DIR18}/assets_registry"
ASSETS_DIR18="${DIR18}/assets"

ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR18}" "run1.md" --tag v2.0.3 --aggregate-checksums \
    --assets-dir "${ASSETS_DIR18}" --sbom-asset "paladin-v2.0.3.cdx.json"; then
    echo "PASS: case18 first run (aggregation + assets + SBOM) exits 0"
else
    echo "FAIL: case18 first run did not exit 0"
    sed 's/^/  | /' "${DIR18}/run1.md.log"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR18}" "run2.md" --tag v2.0.3 --aggregate-checksums \
    --assets-dir "${ASSETS_DIR18}" --sbom-asset "paladin-v2.0.3.cdx.json"; then
    echo "PASS: case18 second run exits 0"
else
    echo "FAIL: case18 second run did not exit 0"
    sed 's/^/  | /' "${DIR18}/run2.md.log"
    FAILED=$((FAILED + 1))
fi

assert_cmp "case18: composing twice over the same release state is byte-identical" \
    "${DIR18}/run1.md" "${DIR18}/run2.md"

# --- Case 19: two byte-identical archives still produce two SHA256SUMS
#     lines, sharing the same digest, both filenames present in sorted
#     order -- equal digests never collapse to one entry. -------------------
DIR19="${SCRATCH}/case19"
mkdir -p "${DIR19}/download_source"
write_gh_stub "${DIR19}"
printf '%s' "${CURATED1}" > "${DIR19}/current_body"
printf 'identical-bytes' > "${DIR19}/download_source/paladin-linux-amd64.tar.gz"
cp "${DIR19}/download_source/paladin-linux-amd64.tar.gz" "${DIR19}/download_source/paladin-linux-arm64.tar.gz"
ASSETS_DIR19="${DIR19}/assets"
run_finalize "${DIR19}" "run1.md" --tag v3.0.0 --aggregate-checksums --assets-dir "${ASSETS_DIR19}"
SUMS19="${ASSETS_DIR19}/SHA256SUMS"

ASSERTIONS=$((ASSERTIONS + 1))
LINES19="$(wc -l < "${SUMS19}" 2>/dev/null | tr -d ' ')"
if [ "${LINES19}" = "2" ]; then
    echo "PASS: case19 two byte-identical archives still produce two SHA256SUMS lines"
else
    echo "FAIL: case19 expected 2 lines, got ${LINES19:-missing}"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
DIGESTS19="$(awk '{print $1}' "${SUMS19}" 2>/dev/null | LC_ALL=C sort -u | wc -l | tr -d ' ')"
if [ "${DIGESTS19}" = "1" ]; then
    echo "PASS: case19 both lines share the same digest"
else
    echo "FAIL: case19 expected a single shared digest, got ${DIGESTS19} distinct digest(s)"
    FAILED=$((FAILED + 1))
fi

assert_contains "case19: first filename present" "${SUMS19}" "paladin-linux-amd64.tar.gz"
assert_contains "case19: second filename present" "${SUMS19}" "paladin-linux-arm64.tar.gz"

ASSERTIONS=$((ASSERTIONS + 1))
NAMES19="$(awk '{print $2}' "${SUMS19}")"
if [ "${NAMES19}" = "$(printf 'paladin-linux-amd64.tar.gz\npaladin-linux-arm64.tar.gz')" ]; then
    echo "PASS: case19 equal-digest lines stay in sorted filename order"
else
    echo "FAIL: case19 unexpected line order:"
    echo "${NAMES19}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Case 20: a single downloaded archive produces a one-line sums file --
#     not a special-cased empty or header-only file. -------------------------
DIR20="${SCRATCH}/case20"
mkdir -p "${DIR20}/download_source"
write_gh_stub "${DIR20}"
printf '%s' "${CURATED1}" > "${DIR20}/current_body"
printf 'solo' > "${DIR20}/download_source/paladin-linux-amd64.tar.gz"
ASSETS_DIR20="${DIR20}/assets"
run_finalize "${DIR20}" "run1.md" --tag v3.0.1 --aggregate-checksums --assets-dir "${ASSETS_DIR20}"
SUMS20="${ASSETS_DIR20}/SHA256SUMS"

ASSERTIONS=$((ASSERTIONS + 1))
LINES20="$(wc -l < "${SUMS20}" 2>/dev/null | tr -d ' ')"
if [ "${LINES20}" = "1" ]; then
    echo "PASS: case20 a single downloaded archive produces a one-line SHA256SUMS"
else
    echo "FAIL: case20 expected 1 line, got ${LINES20:-missing}"
    FAILED=$((FAILED + 1))
fi
assert_not_contains "case20: no header text precedes the digest line" "${SUMS20}" "SHA256"

# --- Case 21: filenames containing a hyphen and digits sort under
#     LC_ALL=C, and the emitted order is unchanged when the underlying
#     archives are created/downloaded in a different order. -----------------
DIR21A="${SCRATCH}/case21a"
mkdir -p "${DIR21A}/download_source"
write_gh_stub "${DIR21A}"
printf '%s' "${CURATED1}" > "${DIR21A}/current_body"
printf 'a1' > "${DIR21A}/download_source/paladin-linux-amd64-10.tar.gz"
printf 'a2' > "${DIR21A}/download_source/paladin-linux-amd64-2.tar.gz"
printf 'a3' > "${DIR21A}/download_source/paladin-linux-amd64-1.tar.gz"
ASSETS_DIR21A="${DIR21A}/assets"
run_finalize "${DIR21A}" "run1.md" --tag v3.0.2 --aggregate-checksums --assets-dir "${ASSETS_DIR21A}"

DIR21B="${SCRATCH}/case21b"
mkdir -p "${DIR21B}/download_source"
write_gh_stub "${DIR21B}"
printf '%s' "${CURATED1}" > "${DIR21B}/current_body"
# Same three files, created in the opposite order -- directory iteration
# order must never leak into the emitted SHA256SUMS ordering.
printf 'a3' > "${DIR21B}/download_source/paladin-linux-amd64-1.tar.gz"
printf 'a2' > "${DIR21B}/download_source/paladin-linux-amd64-2.tar.gz"
printf 'a1' > "${DIR21B}/download_source/paladin-linux-amd64-10.tar.gz"
ASSETS_DIR21B="${DIR21B}/assets"
run_finalize "${DIR21B}" "run1.md" --tag v3.0.2 --aggregate-checksums --assets-dir "${ASSETS_DIR21B}"

assert_cmp "case21: SHA256SUMS ordering is unaffected by archive creation/download order" \
    "${ASSETS_DIR21A}/SHA256SUMS" "${ASSETS_DIR21B}/SHA256SUMS"

ASSERTIONS=$((ASSERTIONS + 1))
NAMES21="$(awk '{print $2}' "${ASSETS_DIR21A}/SHA256SUMS")"
EXPECTED21="$(printf 'paladin-linux-amd64-1.tar.gz\npaladin-linux-amd64-10.tar.gz\npaladin-linux-amd64-2.tar.gz')"
if [ "${NAMES21}" = "${EXPECTED21}" ]; then
    echo "PASS: case21 hyphen+digit filenames sort under LC_ALL=C byte order"
else
    echo "FAIL: case21 unexpected LC_ALL=C sort order:"
    echo "${NAMES21}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- Case 22: running aggregation twice over the same asset set produces a
#     byte-identical sums file and a second --clobber upload rather than a
#     failure. -----------------------------------------------------------
DIR22="${SCRATCH}/case22"
mkdir -p "${DIR22}/download_source"
write_gh_stub "${DIR22}"
printf '%s' "${CURATED1}" > "${DIR22}/current_body"
printf 'x' > "${DIR22}/download_source/paladin-linux-amd64.tar.gz"
ASSETS_DIR22="${DIR22}/assets"

run_finalize "${DIR22}" "run1.md" --tag v3.0.3 --aggregate-checksums --assets-dir "${ASSETS_DIR22}"
cp "${ASSETS_DIR22}/SHA256SUMS" "${DIR22}/sums_after_run1"
run_finalize "${DIR22}" "run2.md" --tag v3.0.3 --aggregate-checksums --assets-dir "${ASSETS_DIR22}"

assert_cmp "case22: re-running aggregation over the same asset set is byte-identical" \
    "${DIR22}/sums_after_run1" "${ASSETS_DIR22}/SHA256SUMS"

ASSERTIONS=$((ASSERTIONS + 1))
UPLOAD_COUNT22="$(grep -cx 'UPLOAD SHA256SUMS' "${DIR22}/call_log" 2>/dev/null || true)"
UPLOAD_COUNT22="${UPLOAD_COUNT22:-0}"
if [ "${UPLOAD_COUNT22}" = "2" ]; then
    echo "PASS: case22 re-running aggregation uploads via --clobber a second time rather than failing"
else
    echo "FAIL: case22 expected 2 SHA256SUMS uploads, got ${UPLOAD_COUNT22}"
    FAILED=$((FAILED + 1))
fi

# --- Case 23: a download that yields files not matching the archive
#     pattern (a stray .sha256) contributes no line to the sums file. -------
DIR23="${SCRATCH}/case23"
mkdir -p "${DIR23}/download_source"
write_gh_stub "${DIR23}"
printf '%s' "${CURATED1}" > "${DIR23}/current_body"
printf 'archive' > "${DIR23}/download_source/paladin-linux-amd64.tar.gz"
printf 'checksum-only' > "${DIR23}/download_source/paladin-linux-amd64.tar.gz.sha256"
ASSETS_DIR23="${DIR23}/assets"
run_finalize "${DIR23}" "run1.md" --tag v3.0.4 --aggregate-checksums --assets-dir "${ASSETS_DIR23}"
SUMS23="${ASSETS_DIR23}/SHA256SUMS"

ASSERTIONS=$((ASSERTIONS + 1))
LINES23="$(wc -l < "${SUMS23}" 2>/dev/null | tr -d ' ')"
if [ "${LINES23}" = "1" ]; then
    echo "PASS: case23 a stray non-archive file contributes no line to SHA256SUMS"
else
    echo "FAIL: case23 expected 1 line (archive only), got ${LINES23:-missing}"
    FAILED=$((FAILED + 1))
fi
assert_not_contains "case23: the stray .sha256 file itself is never listed" "${SUMS23}" ".sha256"

# --- Case 24: an asset list containing the sums file itself twice lists it
#     once, not twice. -------------------------------------------------------
DIR24="${SCRATCH}/case24"
mkdir -p "${DIR24}"
write_gh_stub "${DIR24}"
printf '%s' "${CURATED1}" > "${DIR24}/current_body"
ASSETS_FILE24="${DIR24}/assets_file.txt"
printf 'SHA256SUMS\nSHA256SUMS\npaladin-linux-amd64.tar.gz\n' > "${ASSETS_FILE24}"
run_finalize "${DIR24}" "run1.md" --tag v3.0.5 --assets-file "${ASSETS_FILE24}"

ASSERTIONS=$((ASSERTIONS + 1))
SHA_COUNT24="$(grep -cF -- '`SHA256SUMS`' "${DIR24}/run1.md" 2>/dev/null || true)"
SHA_COUNT24="${SHA_COUNT24:-0}"
if [ "${SHA_COUNT24}" = "1" ]; then
    echo "PASS: case24 a duplicated asset name is listed exactly once"
else
    echo "FAIL: case24 expected SHA256SUMS to be listed exactly once, got ${SHA_COUNT24}"
    FAILED=$((FAILED + 1))
fi

# --- Case 25: assets exist but no sums file was attached in this
#     invocation (no --aggregate-checksums) -> the downloads section still
#     lists the existing assets, but carries no verification block. --------
DIR25="${SCRATCH}/case25"
mkdir -p "${DIR25}"
write_gh_stub "${DIR25}"
printf '%s' "${CURATED1}" > "${DIR25}/current_body"
printf 'paladin-linux-amd64.tar.gz\n' > "${DIR25}/assets_registry"
run_finalize "${DIR25}" "run1.md" --tag v3.0.6
assert_contains "case25: downloads section still lists existing assets" "${DIR25}/run1.md" '`paladin-linux-amd64.tar.gz`'
assert_not_contains "case25: no verification block when no sums file was attached" "${DIR25}/run1.md" "sha256sum -c"

# --- Size-guard helpers (plan 52-04, CIREL-05, D-11). All measuring goes
#     through python3 -I so the harness counts Unicode code points the same
#     way scripts/release-body-fit.sh does, never bytes or `wc`. -------------

# body_chars FILE -> prints the code-point count of FILE.
body_chars() {
    python3 -I -c 'import sys; print(len(open(sys.argv[1], "rb").read().decode("utf-8")))' "$1"
}

# tail_from_marker FILE OUT -> writes the bytes of FILE from the artifacts
# marker onward (the artifact part of a composed body) to OUT.
tail_from_marker() {
    python3 -I -c '
import sys
data = open(sys.argv[1], "rb").read()
marker = b"<!-- paladin:release-artifacts -->"
at = data.find(marker)
if at < 0:
    sys.exit(1)
open(sys.argv[2], "wb").write(data[at:])
' "$1" "$2"
}

# count_literal FILE NEEDLE -> prints how many times NEEDLE occurs in FILE.
count_literal() {
    python3 -I -c 'import sys; print(open(sys.argv[1], "rb").read().decode("utf-8").count(sys.argv[2]))' "$1" "$2"
}

# make_long_curated OUT LINES -> writes a multi-line changelog-shaped section
# of LINES bullet lines with one fenced block spanning lines 30-70.
make_long_curated() {
    python3 -I -c '
import sys
out, lines = sys.argv[1], int(sys.argv[2])
rows = ["## [9.9.9] - 2026-10-28", ""]
for n in range(1, lines + 1):
    if n == 30:
        rows.append("```text")
    rows.append("- entry %03d: a changelog line padded out to a realistic length for a release" % n)
    if n == 70:
        rows.append("```")
open(out, "w", encoding="utf-8").write("\n".join(rows) + "\n")
' "$1" "$2"
}

SENTINEL_T="<!-- paladin:release-truncated -->"

# --- Case 26: a composed body over PALADIN_RELEASE_BODY_LIMIT is re-fitted --
#     only the curated section is cut (to limit minus the artifact part),
#     the artifact part is byte-identical to the untruncated composition,
#     the published body is within the limit, carries the footer exactly
#     once, and the truncation is logged. -----------------------------------
DIR26="${SCRATCH}/case26"
mkdir -p "${DIR26}"
write_gh_stub "${DIR26}"
make_long_curated "${DIR26}/curated.md" 100
printf '%s' "$(cat "${DIR26}/curated.md")"$'\n' > "${DIR26}/current_body"
printf 'paladin-linux-amd64.tar.gz\npaladin-linux-arm64.tar.gz\n' > "${DIR26}/assets_file.txt"
ASSETS26="$(cat "${DIR26}/assets_file.txt")"
CURATED26="$(cat "${DIR26}/curated.md")"$'\n'

ASSERTIONS=$((ASSERTIONS + 1))
CURATED26_CHARS="$(body_chars "${DIR26}/curated.md")"
if [ "${CURATED26_CHARS}" -gt 5500 ]; then
    echo "PASS: case26 fixture curated section is ${CURATED26_CHARS} chars (> 5500)"
else
    echo "FAIL: case26 fixture curated section is only ${CURATED26_CHARS} chars"
    FAILED=$((FAILED + 1))
fi

# The untruncated composition, for the artifact-part comparison.
compose_to case26 untruncated.md "${CURATED26}" "abc123" "ghcr.io/df3ndr/paladin-dev-env:9.9.9" "450" \
    "${ASSETS26}" "" "paladin-v9.9.9.cdx.json"
tail_from_marker "${LAST_COMPOSE_FILE}" "${DIR26}/untruncated.tail"

export PALADIN_RELEASE_BODY_LIMIT=4000
export GITHUB_REPOSITORY=test/repo
ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR26}" "run1.md" --tag v9.9.9 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:9.9.9" --image-size-mb 450 \
    --assets-file "${DIR26}/assets_file.txt" --sbom-asset "paladin-v9.9.9.cdx.json"; then
    echo "PASS: case26 over-limit finalize run exits 0"
else
    echo "FAIL: case26 over-limit finalize run did not exit 0"
    sed 's/^/  | /' "${DIR26}/run1.md.log"
    FAILED=$((FAILED + 1))
fi
cp "${DIR26}/current_body" "${DIR26}/published1.md"

ASSERTIONS=$((ASSERTIONS + 1))
PUBLISHED26_CHARS="$(body_chars "${DIR26}/published1.md")"
if [ "${PUBLISHED26_CHARS}" -le 4000 ] && [ "${PUBLISHED26_CHARS}" -gt 0 ]; then
    echo "PASS: case26 published body is ${PUBLISHED26_CHARS} chars (<= 4000)"
else
    echo "FAIL: case26 published body is ${PUBLISHED26_CHARS} chars, expected 1..4000"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
SENTINEL26="$(count_literal "${DIR26}/published1.md" "${SENTINEL_T}")"
if [ "${SENTINEL26}" = "1" ]; then
    echo "PASS: case26 truncation sentinel appears exactly once"
else
    echo "FAIL: case26 expected the truncation sentinel once, got ${SENTINEL26}"
    FAILED=$((FAILED + 1))
fi

tail_from_marker "${DIR26}/published1.md" "${DIR26}/published1.tail" || true
assert_cmp "case26: the artifact part is byte-identical to the untruncated composition" \
    "${DIR26}/untruncated.tail" "${DIR26}/published1.tail"
assert_contains "case26: the curated heading survives the cut" "${DIR26}/published1.md" "## [9.9.9] - 2026-10-28"
assert_contains "case26: the truncation is logged by size" "${DIR26}/run1.md.log" "release-body-fit: original"
assert_not_contains "case26: the last curated entry was cut" "${DIR26}/published1.md" "entry 100:"

ASSERTIONS=$((ASSERTIONS + 1))
FENCES26="$(grep -c '^```' "${DIR26}/published1.md" || true)"
if [ $((FENCES26 % 2)) -eq 0 ] && [ "${FENCES26}" -ge 2 ]; then
    echo "PASS: case26 the cut inside the fenced block leaves the fences balanced (${FENCES26} fence lines)"
else
    echo "FAIL: case26 expected a balanced, non-zero number of fence lines, got ${FENCES26}"
    FAILED=$((FAILED + 1))
fi

# --- Case 27: finalize is idempotent over its own truncated output -- two
#     more runs publish a body byte-identical to the first, log no new
#     truncation, and the footer stays singular. ----------------------------
for RUN27 in 2 3; do
    ASSERTIONS=$((ASSERTIONS + 1))
    if run_finalize "${DIR26}" "run${RUN27}.md" --tag v9.9.9 --image-digest abc123 \
        --image-ref "ghcr.io/df3ndr/paladin-dev-env:9.9.9" --image-size-mb 450 \
        --assets-file "${DIR26}/assets_file.txt" --sbom-asset "paladin-v9.9.9.cdx.json"; then
        echo "PASS: case27 re-run ${RUN27} exits 0"
    else
        echo "FAIL: case27 re-run ${RUN27} did not exit 0"
        sed 's/^/  | /' "${DIR26}/run${RUN27}.md.log"
        FAILED=$((FAILED + 1))
    fi
    assert_cmp "case27: re-run ${RUN27} publishes a body byte-identical to the first run" \
        "${DIR26}/run1.md" "${DIR26}/run${RUN27}.md"
    assert_not_contains "case27: re-run ${RUN27} logs no new truncation" \
        "${DIR26}/run${RUN27}.md.log" "release-body-fit:"
done
assert_cmp "case27: the stub's round-tripped body equals the first publication" \
    "${DIR26}/published1.md" "${DIR26}/current_body"

ASSERTIONS=$((ASSERTIONS + 1))
SENTINEL27="$(count_literal "${DIR26}/current_body" "${SENTINEL_T}")"
if [ "${SENTINEL27}" = "1" ]; then
    echo "PASS: case27 the footer appears exactly once after three runs"
else
    echo "FAIL: case27 expected one footer after three runs, got ${SENTINEL27}"
    FAILED=$((FAILED + 1))
fi
unset PALADIN_RELEASE_BODY_LIMIT GITHUB_REPOSITORY

# call_count FILE LINE -> prints how many lines of FILE equal LINE exactly.
call_count() {
    local count
    count="$(grep -cx -- "$2" "$1" 2>/dev/null || true)"
    echo "${count:-0}"
}

# --- Case 28a: under the default limit the size guard is a no-op -- the
#     published body is byte-identical to the plain composition and nothing
#     is logged about fitting. ------------------------------------------------
DIR28A="${SCRATCH}/case28a"
mkdir -p "${DIR28A}"
write_gh_stub "${DIR28A}"
printf '%s' "${CURATED1}" > "${DIR28A}/current_body"
printf 'paladin-linux-amd64.tar.gz\npaladin-linux-arm64.tar.gz\n' > "${DIR28A}/assets_file.txt"
unset PALADIN_RELEASE_BODY_LIMIT
export GITHUB_REPOSITORY=test/repo
compose_to case28a expected.md "${CURATED1}" "abc123" "ghcr.io/df3ndr/paladin-dev-env:1.2.3" "450" \
    "$(cat "${DIR28A}/assets_file.txt")" "" "paladin-v1.2.3.cdx.json"
ASSERTIONS=$((ASSERTIONS + 1))
if run_finalize "${DIR28A}" "run1.md" --tag v1.2.3 --image-digest abc123 \
    --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.3" --image-size-mb 450 \
    --assets-file "${DIR28A}/assets_file.txt" --sbom-asset "paladin-v1.2.3.cdx.json"; then
    echo "PASS: case28a under-limit finalize run exits 0"
else
    echo "FAIL: case28a under-limit finalize run did not exit 0"
    sed 's/^/  | /' "${DIR28A}/run1.md.log"
    FAILED=$((FAILED + 1))
fi
assert_cmp "case28a: the published body is byte-identical to the plain composition" \
    "${LAST_COMPOSE_FILE}" "${DIR28A}/current_body"
assert_not_contains "case28a: no fit is logged for a body under the limit" "${DIR28A}/run1.md.log" "release-body-fit:"
unset GITHUB_REPOSITORY

# --- Case 28b: a curated section that already carries a create-time footer
#     (written by the real helper under a small limit) is published unchanged
#     by a finalize run under the limit: one footer, no re-fit. --------------
DIR28B="${SCRATCH}/case28b"
mkdir -p "${DIR28B}"
write_gh_stub "${DIR28B}"
make_long_curated "${DIR28B}/long.md" 100
(
    # shellcheck source=scripts/release-body-fit.sh
    RELEASE_BODY_FIT_LIB_ONLY=1 source "${WORKSPACE_ROOT}/scripts/release-body-fit.sh"
    PALADIN_RELEASE_BODY_LIMIT=1500 PALADIN_RELEASE_BODY_RESERVE=100 \
        release_body_fit_for_create "${DIR28B}/long.md" "${DIR28B}/current_body" v9.9.9 test/repo
) >"${DIR28B}/create-fit.log" 2>&1
ASSERTIONS=$((ASSERTIONS + 1))
FOOTER28B="$(count_literal "${DIR28B}/current_body" "${SENTINEL_T}")"
if [ "${FOOTER28B}" = "1" ]; then
    echo "PASS: case28b fixture is a create-time footered body (one sentinel)"
else
    echo "FAIL: case28b fixture should carry one sentinel, got ${FOOTER28B}"
    sed 's/^/  | /' "${DIR28B}/create-fit.log"
    FAILED=$((FAILED + 1))
fi
CURATED28B="$(cat "${DIR28B}/current_body"; printf x)"
CURATED28B="${CURATED28B%x}"
compose_to case28b expected.md "${CURATED28B}" "" "" "" "paladin-linux-amd64.tar.gz" "" ""
printf 'paladin-linux-amd64.tar.gz\n' > "${DIR28B}/assets_file.txt"
ASSERTIONS=$((ASSERTIONS + 1))
if GITHUB_REPOSITORY=test/repo run_finalize "${DIR28B}" "run1.md" --tag v9.9.9 \
    --assets-file "${DIR28B}/assets_file.txt"; then
    echo "PASS: case28b finalize over a footered curated section exits 0"
else
    echo "FAIL: case28b finalize did not exit 0"
    sed 's/^/  | /' "${DIR28B}/run1.md.log"
    FAILED=$((FAILED + 1))
fi
assert_cmp "case28b: the footered curated section is published unchanged" \
    "${LAST_COMPOSE_FILE}" "${DIR28B}/current_body"
ASSERTIONS=$((ASSERTIONS + 1))
FOOTER28B="$(count_literal "${DIR28B}/current_body" "${SENTINEL_T}")"
if [ "${FOOTER28B}" = "1" ]; then
    echo "PASS: case28b the footer is still singular after finalize"
else
    echo "FAIL: case28b expected one sentinel after finalize, got ${FOOTER28B}"
    FAILED=$((FAILED + 1))
fi
assert_not_contains "case28b: no fit is logged" "${DIR28B}/run1.md.log" "release-body-fit:"

# --- Case 28c: a composed body of exactly PALADIN_RELEASE_BODY_LIMIT chars is
#     published untouched (the boundary is inclusive); one character less
#     and it is fitted, still inside the limit with its artifact tail intact.
UNTRUNCATED_CHARS="$(body_chars "${DIR26}/untruncated.md")"
for LIMIT28C in "${UNTRUNCATED_CHARS}" "$((UNTRUNCATED_CHARS - 1))"; do
    DIR28C="${SCRATCH}/case28c-${LIMIT28C}"
    mkdir -p "${DIR28C}"
    write_gh_stub "${DIR28C}"
    cp "${DIR26}/curated.md" "${DIR28C}/current_body"
    ASSERTIONS=$((ASSERTIONS + 1))
    if PALADIN_RELEASE_BODY_LIMIT="${LIMIT28C}" GITHUB_REPOSITORY=test/repo run_finalize "${DIR28C}" "run1.md" \
        --tag v9.9.9 --image-digest abc123 --image-ref "ghcr.io/df3ndr/paladin-dev-env:9.9.9" \
        --image-size-mb 450 --assets-file "${DIR26}/assets_file.txt" --sbom-asset "paladin-v9.9.9.cdx.json"; then
        echo "PASS: case28c finalize at limit ${LIMIT28C} exits 0"
    else
        echo "FAIL: case28c finalize at limit ${LIMIT28C} did not exit 0"
        sed 's/^/  | /' "${DIR28C}/run1.md.log"
        FAILED=$((FAILED + 1))
    fi
    if [ "${LIMIT28C}" = "${UNTRUNCATED_CHARS}" ]; then
        assert_cmp "case28c: a body of exactly the limit is published byte-identical" \
            "${DIR26}/untruncated.md" "${DIR28C}/current_body"
        assert_not_contains "case28c: no fit is logged at exactly the limit" "${DIR28C}/run1.md.log" "release-body-fit:"
    else
        ASSERTIONS=$((ASSERTIONS + 1))
        PUBLISHED28C="$(body_chars "${DIR28C}/current_body")"
        if [ "${PUBLISHED28C}" -le "${LIMIT28C}" ]; then
            echo "PASS: case28c a body one over the limit is fitted to ${PUBLISHED28C} chars (<= ${LIMIT28C})"
        else
            echo "FAIL: case28c published ${PUBLISHED28C} chars at limit ${LIMIT28C}"
            FAILED=$((FAILED + 1))
        fi
        tail_from_marker "${DIR28C}/current_body" "${DIR28C}/published.tail" || true
        assert_cmp "case28c: the artifact tail is intact when the curated part is fitted" \
            "${DIR26}/untruncated.tail" "${DIR28C}/published.tail"
        assert_contains "case28c: the fit is logged" "${DIR28C}/run1.md.log" "release-body-fit: original"
    fi
done

# --- Case 29a: the artifact sections alone exceed the limit -> a named
#     failure before `gh release edit`; nothing is published and no artifact
#     section is dropped to make room. ---------------------------------------
DIR29A="${SCRATCH}/case29a"
mkdir -p "${DIR29A}"
write_gh_stub "${DIR29A}"
printf '%s' "${CURATED1}" > "${DIR29A}/current_body"
printf 'paladin-linux-amd64.tar.gz\npaladin-linux-arm64.tar.gz\n' > "${DIR29A}/assets_file.txt"
ASSERTIONS=$((ASSERTIONS + 1))
if PALADIN_RELEASE_BODY_LIMIT=200 GITHUB_REPOSITORY=test/repo run_finalize "${DIR29A}" "run1.md" \
    --tag v1.2.3 --image-digest abc123 --image-ref "ghcr.io/df3ndr/paladin-dev-env:1.2.3" \
    --image-size-mb 450 --assets-file "${DIR29A}/assets_file.txt" --sbom-asset "paladin-v1.2.3.cdx.json"; then
    echo "FAIL: case29a expected a non-zero exit when the artifact sections exceed the limit"
    FAILED=$((FAILED + 1))
else
    echo "PASS: case29a artifact sections over the limit exit non-zero"
fi
assert_contains "case29a: the failure is named" "${DIR29A}/run1.md.log" "leaves no room"
ASSERTIONS=$((ASSERTIONS + 1))
if [ "$(call_count "${DIR29A}/call_log" EDIT)" = "0" ]; then
    echo "PASS: case29a no release edit was attempted"
else
    echo "FAIL: case29a gh release edit was called despite the failure"
    FAILED=$((FAILED + 1))
fi
assert_cmp "case29a: the stored release body is untouched" \
    <(printf '%s' "${CURATED1}") "${DIR29A}/current_body"

# --- Case 29b: an invalid limit is a named failure before `gh release edit`.
DIR29B="${SCRATCH}/case29b"
mkdir -p "${DIR29B}"
write_gh_stub "${DIR29B}"
printf '%s' "${CURATED1}" > "${DIR29B}/current_body"
ASSERTIONS=$((ASSERTIONS + 1))
if PALADIN_RELEASE_BODY_LIMIT=abc run_finalize "${DIR29B}" "run1.md" --tag v1.2.3; then
    echo "FAIL: case29b expected a non-zero exit for a non-numeric limit"
    FAILED=$((FAILED + 1))
else
    echo "PASS: case29b a non-numeric limit exits non-zero"
fi
assert_contains "case29b: the failure is named" "${DIR29B}/run1.md.log" "must be a positive integer"
ASSERTIONS=$((ASSERTIONS + 1))
if [ "$(call_count "${DIR29B}/call_log" EDIT)" = "0" ]; then
    echo "PASS: case29b no release edit was attempted"
else
    echo "FAIL: case29b gh release edit was called despite the failure"
    FAILED=$((FAILED + 1))
fi

# --- Case 29c: the artifact part leaves a budget too small for the footer
#     (positive, but only a few characters) -> the same named failure, and no
#     release edit. ----------------------------------------------------------
DIR29C="${SCRATCH}/case29c"
mkdir -p "${DIR29C}"
write_gh_stub "${DIR29C}"
cp "${DIR26}/curated.md" "${DIR29C}/current_body"
ARTIFACT29C="$(body_chars "${DIR26}/untruncated.tail")"
ASSERTIONS=$((ASSERTIONS + 1))
if PALADIN_RELEASE_BODY_LIMIT="$((ARTIFACT29C + 30))" GITHUB_REPOSITORY=test/repo run_finalize "${DIR29C}" "run1.md" \
    --tag v9.9.9 --image-digest abc123 --image-ref "ghcr.io/df3ndr/paladin-dev-env:9.9.9" \
    --image-size-mb 450 --assets-file "${DIR26}/assets_file.txt" --sbom-asset "paladin-v9.9.9.cdx.json"; then
    echo "FAIL: case29c expected a non-zero exit when the budget cannot hold the footer"
    FAILED=$((FAILED + 1))
else
    echo "PASS: case29c a budget too small for the footer exits non-zero"
fi
assert_contains "case29c: the failure is named" "${DIR29C}/run1.md.log" "leaves no room"
ASSERTIONS=$((ASSERTIONS + 1))
if [ "$(call_count "${DIR29C}/call_log" EDIT)" = "0" ]; then
    echo "PASS: case29c no release edit was attempted"
else
    echo "FAIL: case29c gh release edit was called despite the failure"
    FAILED=$((FAILED + 1))
fi

# --- Case 29d: a missing size helper is a named failure before any gh call
#     (the script copied somewhere its sibling helper does not exist). -------
DIR29D="${SCRATCH}/case29d"
mkdir -p "${DIR29D}/scripts"
write_gh_stub "${DIR29D}"
cp "${GUARD}" "${DIR29D}/scripts/finalize-release-body.sh"
printf '%s' "${CURATED1}" > "${DIR29D}/current_body"
ASSERTIONS=$((ASSERTIONS + 1))
if FINALIZE_RELEASE_BODY_LIB_ONLY=0 GH_BIN="${DIR29D}/gh-stub.sh" bash "${DIR29D}/scripts/finalize-release-body.sh" \
    --tag v1.2.3 --output "${DIR29D}/run1.md" >"${DIR29D}/run1.md.log" 2>&1; then
    echo "FAIL: case29d expected a non-zero exit without the size helper"
    FAILED=$((FAILED + 1))
else
    echo "PASS: case29d a missing size helper exits non-zero"
fi
assert_contains "case29d: the failure names the helper" "${DIR29D}/run1.md.log" "release-body-fit.sh not found"
ASSERTIONS=$((ASSERTIONS + 1))
if [ ! -e "${DIR29D}/call_log" ]; then
    echo "PASS: case29d no gh call was made"
else
    echo "FAIL: case29d gh was called before the helper check"
    FAILED=$((FAILED + 1))
fi

# --- The real tree must never be mutated by this test: scripts/ and
#     .github/workflows/ (this guard only invokes the stubbed GH_BIN, never
#     the real `gh`, and never writes into the repo tree). -------------------
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
