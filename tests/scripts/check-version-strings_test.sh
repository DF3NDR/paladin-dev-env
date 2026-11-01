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
    printf '{\n  "openapi": "3.1.0",\n  "info": {\n    "title": "x",\n    "version": "1.2.3"\n  }\n}\n' \
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

# replace_in FILE OLD NEW -> replaces every literal OLD with NEW in FILE
# (relative to $FX).
replace_in() {
    local content
    content="$(<"${FX}/$1")"
    printf '%s\n' "${content//"$2"/"$3"}" >"${FX}/$1"
}

# line_of FILE TEXT -> 1-based number of the first line of FILE (relative to
# $FX) containing the literal TEXT.
line_of() {
    grep -nF -- "$2" "${FX}/$1" | head -n1 | cut -d: -f1
}

# tree_sums DIR -> checksums of every file under DIR, for the no-mutation case.
tree_sums() {
    (cd "$1" && find . -type f | LC_ALL=C sort | xargs cksum)
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

# --- Failing cases (classes 2-4, named failures, edges) ---------------------

fresh stale-current-version
replace_in README.md 'Current version: **1.2.3**.' 'Current version: **1.2.2**.'
assert_fire "${FX}" "class 2: a stale Current version line in README.md" \
    "README.md:$(line_of README.md 'Current version:'): found '1.2.2', want '1.2.3'"

fresh stale-k8s-label
replace_in k8s/server/deployment.yaml 'version: v1.2.3' 'version: v1.2.2'
assert_fire "${FX}" "class 3: a stale k8s version label" \
    "k8s/server/deployment.yaml:$(line_of k8s/server/deployment.yaml 'version: v1.2.2'): found '1.2.2', want '1.2.3'"

fresh stale-k8s-image
replace_in k8s/server/deployment.yaml 'image: paladin-server:1.2.3' 'image: paladin-server:1.2.2'
assert_fire "${FX}" "class 3: a stale paladin-server image tag" \
    "k8s/server/deployment.yaml:$(line_of k8s/server/deployment.yaml 'image: paladin-server'): found '1.2.2', want '1.2.3'"

fresh stale-k8s-registry-image
plant k8s/server/deployment.yaml '    - image: example.com/team/paladin:1.2.2'
assert_fire "${FX}" "class 3: a stale paladin tag behind a registry path" \
    "k8s/server/deployment.yaml:${PLANT_LINE}: found '1.2.2', want '1.2.3'"

fresh stale-openapi
replace_in crates/paladin-web/openapi.json '"version": "1.2.3"' '"version": "1.2.2"'
assert_fire "${FX}" "class 4: a stale openapi.json info.version" \
    "crates/paladin-web/openapi.json:$(line_of crates/paladin-web/openapi.json '"version"'): found '1.2.2', want '1.2.3'"

fresh stale-crate-readme
plant crates/paladin-llm/README.md 'paladin-llm = "1.2.2"'
assert_fire "${FX}" "a stale pin in a crates/*/README.md" \
    "crates/paladin-llm/README.md:${PLANT_LINE}: found '1.2.2', want '1.2.3'"

fresh image-v-prefixed
replace_in k8s/server/deployment.yaml 'image: paladin-server:1.2.3' 'image: paladin-server:v1.2.3'
assert_fire "${FX}" "an image tag must equal the version exactly (v1.2.3 fails)" \
    "found 'v1.2.3', want '1.2.3'"

fresh label-longer
replace_in k8s/server/deployment.yaml 'version: v1.2.3' 'version: v1.2.30'
assert_fire "${FX}" "a label v1.2.30 differs from 1.2.3 (no semver normalisation)" \
    "found '1.2.30', want '1.2.3'"

fresh label-prerelease
replace_in k8s/server/deployment.yaml 'version: v1.2.3' 'version: v1.2.3-rc.1'
assert_fire "${FX}" "a label v1.2.3-rc.1 differs from 1.2.3" \
    "found '1.2.3-rc.1', want '1.2.3'"

# Named, never-vacuous failures.
fresh no-current-version
printf '# paladin-ai\n\n```toml\npaladin-ai = "1.2.3"\n```\n' >"${FX}/README.md"
assert_fire "${FX}" "no Current version line anywhere gives CLASS_UNMATCHED for class 2" \
    "CLASS_UNMATCHED" "class 2"

fresh no-k8s-strings
printf 'apiVersion: apps/v1\nkind: Deployment\n' >"${FX}/k8s/server/deployment.yaml"
assert_fire "${FX}" "no k8s label or first-party tag gives CLASS_UNMATCHED for class 3" \
    "CLASS_UNMATCHED" "class 3"

fresh no-openapi
rm "${FX}/crates/paladin-web/openapi.json"
assert_fire "${FX}" "a missing openapi.json gives CLASS_UNMATCHED for class 4" \
    "CLASS_UNMATCHED" "class 4"

fresh no-pins
printf '# paladin-ai\n\nCurrent version: **1.2.3**.\n' >"${FX}/README.md"
printf '# Guide\n' >"${FX}/docs/src/guide.md"
printf '# paladin-llm\n' >"${FX}/crates/paladin-llm/README.md"
assert_fire "${FX}" "no Cargo pin anywhere gives CLASS_UNMATCHED for class 1" \
    "CLASS_UNMATCHED" "class 1"

fresh stale-exemption
rm "${FX}/docs/src/api-reference/migration-guide.md"
assert_fire "${FX}" "deleting the exempt file gives STALE_EXEMPTION" \
    "STALE_EXEMPTION" "docs/src/api-reference/migration-guide.md"

fresh no-version
printf '[package]\nname = "paladin-ai"\n' >"${FX}/Cargo.toml"
assert_fire "${FX}" "a root Cargo.toml with no [package].version gives NO_VERSION" \
    "NO_VERSION"

# Ordering and determinism: three stale strings in two files, all reported,
# sorted by path then line.
fresh ordering
plant docs/src/guide.md 'paladin-ai = "1.2.0"'
GUIDE_LINE_1="${PLANT_LINE}"
plant docs/src/guide.md 'paladin-llm = "1.2.1"'
GUIDE_LINE_2="${PLANT_LINE}"
plant README.md 'paladin-ai = "1.2.2"'
README_LINE="${PLANT_LINE}"
ASSERTIONS=$((ASSERTIONS + 1))
run_guard "${FX}"
first_output="${LAST_OUTPUT}"
n_readme="$(grep -nF "README.md:${README_LINE}: found '1.2.2'" <<<"${LAST_OUTPUT}" | head -n1 | cut -d: -f1)"
n_guide_1="$(grep -nF "docs/src/guide.md:${GUIDE_LINE_1}: found '1.2.0'" <<<"${LAST_OUTPUT}" | head -n1 | cut -d: -f1)"
n_guide_2="$(grep -nF "docs/src/guide.md:${GUIDE_LINE_2}: found '1.2.1'" <<<"${LAST_OUTPUT}" | head -n1 | cut -d: -f1)"
# Byte order: README.md sorts before docs/src/guide.md ('R' < 'd'), and guide lines ascend.
if [ "${LAST_STATUS}" -ne 0 ] && [ -n "${n_readme}" ] && [ -n "${n_guide_1}" ] && [ -n "${n_guide_2}" ] \
    && [ "${n_readme}" -lt "${n_guide_1}" ] && [ "${n_guide_1}" -lt "${n_guide_2}" ]; then
    echo "PASS (fire): all three findings are printed, sorted by path then line"
else
    echo "FAIL: expected all three findings sorted by path then line (status ${LAST_STATUS}, lines '${n_readme}' '${n_guide_1}' '${n_guide_2}')"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
BEFORE_FX_SUMS="$(tree_sums "${FX}")"
run_guard "${FX}"
if cmp -s <(printf '%s' "${first_output}") <(printf '%s' "${LAST_OUTPUT}"); then
    echo "PASS (fire): two runs over the same tree print byte-identical output"
else
    echo "FAIL: two runs over the same tree printed different output"
    FAILED=$((FAILED + 1))
fi

ASSERTIONS=$((ASSERTIONS + 1))
if [ "$(tree_sums "${FX}")" = "${BEFORE_FX_SUMS}" ] && [ "${LAST_STATUS}" -ne 0 ]; then
    echo "PASS (no mutation): the fixture tree is byte-identical after a failing run"
else
    echo "FAIL: the fixture tree changed across a failing run (or the run did not fail)"
    FAILED=$((FAILED + 1))
fi

# --- Passing cases ----------------------------------------------------------

fresh current
assert_silent "${FX}" "a current fixture tree (exempt file holds a stale 0.4 pin) passes" "OK:"

fresh no-finding
plant docs/src/guide.md 'paladin-custom-llm = "0.1"'
plant docs/src/guide.md 'tokio = { version = "1" }'
plant docs/src/guide.md 'paladin-ai = { path = "../x" }'
plant docs/src/guide.md 'package = "paladin-ai-core"'
assert_silent "${FX}" "non-workspace keys, version-less tables and prose produce no finding" "OK:"

fresh label-forms
plant k8s/server/deployment.yaml '    version: "v1.2.3"'
plant k8s/server/deployment.yaml '    version: 1.2.3'
plant k8s/server/deployment.yaml '    version: v1.2.3 # trailing comment'
plant k8s/server/deployment.yaml 'apiVersion: v1'
assert_silent "${FX}" "label forms v1.2.3, \"v1.2.3\" and 1.2.3 pass; apiVersion is never a label" "OK:"

fresh other-images
plant k8s/server/deployment.yaml '          image: paladin:test'
plant k8s/server/deployment.yaml '          image: paladin-server:latest'
plant k8s/server/deployment.yaml '          image: redis:7-alpine'
plant k8s/server/deployment.yaml '          image: rustfs/rustfs:1.0.0'
plant k8s/server/deployment.yaml '          image: example/my-paladin:1.0.0'
assert_silent "${FX}" "non-candidate images (test, latest, redis, rustfs, my-paladin) are not matched" "OK:"

fresh version-outside-k8s
plant docs/src/guide.md 'version: 9.9.9'
assert_silent "${FX}" "a version: line in markdown outside k8s/ is not a class 3 label" "OK:"

fresh not-utf8
printf '\377\376\000paladin-ai = "0.0.1"\377\n' >"${FX}/docs/src/binary.dat"
assert_silent "${FX}" "a file that is not valid UTF-8 under docs/src is skipped, never a crash" "OK:"

# Documented limits (PD-07-3): two shapes are deliberately not matched.
fresh limit-table-header
plant docs/src/guide.md '[dependencies.paladin-ai]'
plant docs/src/guide.md 'version = "0.5"'
assert_silent "${FX}" "a [dependencies.<name>] table with version = on the next line is not matched" "OK:"

fresh limit-late-version
plant docs/src/guide.md 'paladin-ai = { features = ['
plant docs/src/guide.md '    "llm-anthropic",'
plant docs/src/guide.md '], version = "0.5" }'
assert_silent "${FX}" "an inline table whose version = follows a multi-line array is not matched" "OK:"

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
elif grep -qE 'class [1-4][^;)]*: 0[;)]' <<<"${LAST_OUTPUT}"; then
    echo "FAIL: the real-tree OK line reports a zero count for one of the four classes"
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
