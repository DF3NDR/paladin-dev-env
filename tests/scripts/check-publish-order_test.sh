#!/usr/bin/env bash
# check-publish-order_test.sh
#
# Committed regression harness for scripts/check-publish-order.sh (SHIP-06,
# ROADMAP Phase 37.1 success criterion 3, plan 37.1-02). Mirrors
# tests/scripts/check-release-consistency_test.sh's fixture-lifecycle
# pattern: the gate is exercised entirely through its own --metadata-json
# fixture seam, so no assertion here ever invokes a live `cargo metadata`
# call or touches the network. Two of the six required fixtures are
# committed under tests/scripts/fixtures/check-publish-order/ (captured from
# a real worktree of commit 1d4a9724, and a derived fixed-shape variant);
# the remaining two set-equality cases are built at run time in a
# `mktemp -d` scratch directory by copying one of the committed fixtures and
# adding/removing a package entry with jq -- the committed fixtures
# themselves are never edited in place, and this test never leaves a
# tracked file modified (checked at the end via a git-status baseline, the
# same convention check-release-consistency_test.sh and
# create-or-reuse-release_test.sh both follow).
#
# Fixtures accumulate into $FAILED rather than exiting on the first
# mismatch, matching the "report everything, don't short-circuit" house
# style the guard scripts in this repo follow.
#
# Provenance of the two committed fixtures (reproducible, not mysterious):
#   metadata-1d4a9724.json -- captured live via:
#     git worktree add --detach <scratch-dir> 1d4a9724
#     (cd <scratch-dir> && cargo metadata --format-version 1 --no-deps) \
#       > tests/scripts/fixtures/check-publish-order/metadata-1d4a9724.json
#     git worktree remove <scratch-dir>
#   This is the real defective tree, captured -- not a hand-written
#   approximation. It carries paladin-battalion's two forward-pointing
#   dev-dependency edges on paladin-llm and paladin-storage, exactly the
#   shape 37-CI-EVIDENCE.md's "D-16 read-only diagnosis #2" already recorded.
#
#   metadata-fixed.json -- derived from metadata-1d4a9724.json via:
#     jq '.packages |= map(if .name == "paladin-battalion" then
#           .dependencies |= map(if .kind == "dev" and
#             (.name == "paladin-llm" or .name == "paladin-storage") then
#             .req = "*" else . end) else . end)' \
#       metadata-1d4a9724.json > metadata-fixed.json
#   Only the two offending dependency entries' `req` fields change (to the
#   match-all requirement "*"); their `path` fields are left untouched --
#   precisely the shape RESEARCH.md observed after applying the path-only
#   fix (dropping `version = "..."` from a workspace dev-dependency) in a
#   scratch worktree: Cargo strips the version constraint but the path
#   dependency itself is unaffected.
#
# Usage:  ./tests/scripts/check-publish-order_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-publish-order.sh"
FIXTURES="${WORKSPACE_ROOT}/tests/scripts/fixtures/check-publish-order"
FIXTURE_1D4A9724="${FIXTURES}/metadata-1d4a9724.json"
FIXTURE_FIXED="${FIXTURES}/metadata-fixed.json"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

if [ ! -f "${FIXTURE_1D4A9724}" ] || [ ! -f "${FIXTURE_FIXED}" ]; then
    echo "ERROR: committed fixtures not found under ${FIXTURES}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-publish-order-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# --- Real-tree mutation baseline, captured before any fixture runs. --------
MUTATION_WATCH_PATHS=(scripts tests/scripts/fixtures/check-publish-order)
BEFORE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"

# run_guard ARGS... -> sets $LAST_OUTPUT and $LAST_STATUS. No binary is
# stubbed -- the gate's own --metadata-json fixture seam is the only input,
# so this never touches the network or invokes a live `cargo metadata`.
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

# assert_silent DESC ARGS... -> expects zero exit AND $LAST_OUTPUT to
# contain the OK status token this gate prints on success (this gate prints
# a success line and exits zero, unlike create-or-reuse-release.sh's
# upload_url= marker).
assert_silent() {
    local desc="$1"
    shift
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "$@"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected zero exit (silent) for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -qF -- "OK" <<<"${LAST_OUTPUT}"; then
        echo "FAIL: expected output to contain 'OK' for: ${desc}"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (silent): ${desc}"
}

# =============================================================================
# Behavior 1: the 1d4a9724 fixture fires, naming paladin-battalion,
# paladin-llm and paladin-storage.
# =============================================================================
assert_fire "1d4a9724 fixture fires, naming paladin-battalion" "paladin-battalion" \
    --metadata-json "${FIXTURE_1D4A9724}"
assert_fire "1d4a9724 fixture fires, naming paladin-llm" "paladin-llm" \
    --metadata-json "${FIXTURE_1D4A9724}"
assert_fire "1d4a9724 fixture fires, naming paladin-storage" "paladin-storage" \
    --metadata-json "${FIXTURE_1D4A9724}"

# =============================================================================
# Behavior 2: the fixed-shape fixture is silent (exit 0).
# =============================================================================
assert_silent "fixed-shape fixture (path-only dev-deps) is silent" \
    --metadata-json "${FIXTURE_FIXED}"

# =============================================================================
# Behavior 3: a publishable crate absent from CRATES fires, naming that
# crate. Built at run time from the fixed fixture (which has zero ordering
# violations of its own, isolating this assertion to the set-equality
# clause) by adding one new publishable package with no dependencies.
# =============================================================================
EXTRA_CRATE_FIXTURE="${SCRATCH}/extra-crate.json"
jq '.packages += [{
        "name": "paladin-not-in-publish-order",
        "version": "0.10.1",
        "publish": null,
        "manifest_path": "/dev/null/Cargo.toml",
        "dependencies": []
    }]' "${FIXTURE_FIXED}" > "${EXTRA_CRATE_FIXTURE}"
assert_fire "a publishable crate absent from CRATES fires, naming it" \
    "paladin-not-in-publish-order" \
    --metadata-json "${EXTRA_CRATE_FIXTURE}"

# =============================================================================
# Behavior 4: a crate present in CRATES but absent from the publishable set
# fires, naming that crate. Built at run time from the fixed fixture by
# removing one publishable package (paladin-content) that CRATES still
# names.
# =============================================================================
MISSING_CRATE_FIXTURE="${SCRATCH}/missing-crate.json"
jq '.packages |= map(select(.name != "paladin-content"))' "${FIXTURE_FIXED}" > "${MISSING_CRATE_FIXTURE}"
assert_fire "a CRATES-listed crate absent from the publishable set fires, naming it" \
    "paladin-content" \
    --metadata-json "${MISSING_CRATE_FIXTURE}"

# =============================================================================
# Behavior 5: two independent forward-pointing edges are both named in one
# report -- the gate does not stop at the first. The committed 1d4a9724
# fixture already carries exactly this shape (paladin-battalion --dev-->
# paladin-llm and paladin-battalion --dev--> paladin-storage); this
# assertion pins that both names appear together in the SAME single run's
# output, not merely across two separate invocations.
# =============================================================================
ASSERTIONS=$((ASSERTIONS + 1))
run_guard --metadata-json "${FIXTURE_1D4A9724}"
if [ "${LAST_STATUS}" -ne 0 ] \
    && grep -qF -- "paladin-llm" <<<"${LAST_OUTPUT}" \
    && grep -qF -- "paladin-storage" <<<"${LAST_OUTPUT}"; then
    echo "PASS (accumulate): a single run names both forward-pointing edges, not just the first"
else
    echo "FAIL: a single run did not name both paladin-llm and paladin-storage together"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# =============================================================================
# Behavior 6: a non-existent fixture path is a hard, named failure -- never
# a silent empty-input pass.
# =============================================================================
assert_fire "a non-existent --metadata-json path is a hard failure, not a silent pass" \
    "not found" \
    --metadata-json "${SCRATCH}/does-not-exist.json"

# --- The real tree must never be mutated by this test: scripts/ (the gate
#     itself is only ever read) and the fixtures directory (committed
#     fixtures are only ever read; derived fixtures live under $SCRATCH). --
ASSERTIONS=$((ASSERTIONS + 1))
AFTER_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- "${MUTATION_WATCH_PATHS[@]}")"
if [ "${BEFORE_STATUS}" = "${AFTER_STATUS}" ]; then
    echo "PASS (no mutation): git status --porcelain -- scripts tests/scripts/fixtures/check-publish-order is unchanged"
else
    echo "FAIL: scripts/ or the committed fixtures were mutated by this test run:"
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
