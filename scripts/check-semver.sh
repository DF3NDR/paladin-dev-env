#!/usr/bin/env bash
# check-semver.sh
#
# The single implementation of the semver gate (CIREL-01, D-01, Phase 52 plan
# 52-01). CI's `semver` job (display name "Semver Checks (vs v0.11.0)") and
# the local recipe `make check-semver` both run THIS file, so the command a
# maintainer runs on a laptop is literally the command CI runs.
#
# What it does: runs
#
#   cargo semver-checks check-release --package <pkg> --default-features \
#       --baseline-version 0.11.0 --release-type patch
#
# once for every published package, against the 0.11.0 baseline on crates.io.
# Every package runs before the verdict, so one failing package never hides
# the results of the packages after it. The exit status is 1 if any package
# failed, else 0.
#
# Why --release-type patch is fixed by flag, never inferred: the tool derives
# the release type from the version delta between the working tree and the
# baseline unless told otherwise. With every manifest at 0.11.0 (equal to the
# baseline) it infers a patch change today, which is what we want. The moment
# a manifest moves to 0.11.1 or 0.12.0, 0.x inference reads that as a
# major-equivalent change and EVERY lint is skipped: the log line becomes
# `0 checks: 0 pass, 254 skip` and the tool still exits 0. That silent no-op
# is exactly the failure this gate exists to prevent, so the type is pinned.
#
# The zero-check tripwire: a package whose output carries no `N checks:` line
# with N >= 1 is a named failure, never a pass. Three failures are named, in
# this order of precedence:
#   lint failure (exit N)    the tool itself exited non-zero (100 = a lint fired)
#   reported no check count  the tool exited 0 but printed no `N checks:` line
#   evaluated zero checks    the tool printed `0 checks:` (every lint skipped)
#
# Why --default-features: without it the tool's own "enable everything except
# unstable/nightly" heuristic pulls in every optional feature combination at
# once, which trips a PRE-EXISTING, unrelated compile break: paladin-memory's
# `qdrant` adapter fails to build rustdoc against the qdrant-client release
# resolved today, because that release added a required `memory` field to
# `VectorParams`. That defect belongs to whichever phase next touches the
# Sanctum/Qdrant adapter (deferred:
# .planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md).
# Checking under each crate's own default features is also the more correct
# scope: it is the build a crates.io consumer gets by just adding the
# dependency.
#
# Why the packages come from scripts/publish-crates.sh: that file's CRATES
# array is the one dependency-ordered list of published PACKAGE names (not
# `crates/` directory names: `crates/paladin-core` publishes as
# `paladin-ai-core`, and a directory-named --package would match zero
# packages and report a silent, meaningless pass). It is read through the
# PUBLISH_CRATES_LIB_ONLY=1 sourcing seam, exactly as check-publish-order.sh
# reads it, so there is no second hand-maintained list here or in ci.yml. A
# future publishable crate is checked automatically; if it has no published
# baseline it fails by name instead of being skipped. An empty or missing
# CRATES list is the named failure CRATES_EMPTY with zero cargo invocations.
#
# Output: one summary line per package (`semver: <pkg>: <check line>`, with
# ` -- <failure>` appended for a failure), then a closing verdict line. When
# GITHUB_ACTIONS is "true" each package is wrapped in ::group:: / ::endgroup::,
# one ::notice annotation carries every package's check line (so the evidence
# can be read from the public check-run annotations API without log-download
# credentials), and one ::error annotation is emitted per failing package.
#
# This script reads no event data and takes no workflow expression input.
#
# Seams (all for the regression harness or a constrained local disk):
#   SEMVER_CARGO_BIN          cargo binary to run (default: cargo)
#   PUBLISH_CRATES_SCRIPT     file defining CRATES (default: scripts/publish-crates.sh)
#   SEMVER_PRUNE_LOCAL_CACHE  "1" deletes ${CARGO_TARGET_DIR:-<repo>/target}/semver-checks/local-*
#                             after each package (each run adds ~100-200 MB there and
#                             the tool regenerates it; CI never sets this)
#   CHECK_SEMVER_LIB_ONLY     "1" loads the functions without running anything
#
# Usage:  ./scripts/check-semver.sh [--package <name>]
#         --package restricts the run to one CRATES member (a name outside
#         CRATES is a usage error).
# Exit:   0 if every checked package evaluated at least one check and passed;
#         1 if any package failed or CRATES could not be read; 2 for a usage
#         error.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SEMVER_BASELINE_VERSION="0.11.0"
SEMVER_RELEASE_TYPE="patch"

# _cs_check_line LOG -> prints the last `N checks: ...` summary in LOG with
# ANSI colour escapes stripped (CARGO_TERM_COLOR=always paints the count), from
# the count to the end of the line, or prints nothing when there is none. The
# pattern is deliberately tolerant: the clean layout
# (`223 checks: 223 pass, 31 skip`) and the failing layout
# (`217 checks: 216 pass, 1 fail, 0 warn, 37 skip`) differ in their fields.
_cs_check_line() {
    local log="$1"
    # grep exits 1 on no match; that is "no line", not an error.
    sed 's/\x1b\[[0-9;]*m//g' "${log}" | { grep -Eo '[0-9]+ checks:.*' || true; } | tail -n 1
}

# _cs_prune_local_cache -> removes the per-run semver-checks scratch builds
# when SEMVER_PRUNE_LOCAL_CACHE=1. Only `local-*` directories directly under
# semver-checks/ are touched; the tool regenerates them on the next run.
_cs_prune_local_cache() {
    [ "${SEMVER_PRUNE_LOCAL_CACHE:-0}" = "1" ] || return 0
    local base="${CARGO_TARGET_DIR:-${WORKSPACE_ROOT}/target}/semver-checks"
    local dir
    for dir in "${base}"/local-*; do
        if [ -d "${dir}" ]; then
            rm -rf -- "${dir}"
        fi
    done
}

# _cs_check_package PKG -> runs the tool for one package and records the result
# in the globals CS_RESULT_LINES (one summary line, always) and CS_FAIL_PKGS /
# CS_FAIL_MSGS (only on a failure). Never aborts the caller's loop: the tool's
# exit status is captured from PIPESTATUS with `set -e` suspended.
_cs_check_package() {
    local pkg="$1"
    local log status check_line count failure=""

    log="$(mktemp "${TMPDIR:-/tmp}/check-semver.XXXXXX")"

    if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
        echo "::group::cargo semver-checks -- ${pkg} vs ${SEMVER_BASELINE_VERSION} (${SEMVER_RELEASE_TYPE})"
    fi

    set +e
    "${SEMVER_CARGO_BIN}" semver-checks check-release \
        --package "${pkg}" \
        --default-features \
        --baseline-version "${SEMVER_BASELINE_VERSION}" \
        --release-type "${SEMVER_RELEASE_TYPE}" 2>&1 | tee "${log}"
    status="${PIPESTATUS[0]}"
    set -e

    if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
        echo "::endgroup::"
    fi

    check_line="$(_cs_check_line "${log}")"
    rm -f "${log}"
    _cs_prune_local_cache

    if [ "${status}" -ne 0 ]; then
        failure="lint failure (exit ${status})"
    elif [ -z "${check_line}" ]; then
        failure="reported no check count"
    else
        count="${check_line%% checks:*}"
        # 10# forces base ten so a count like 08 can never read as octal.
        if [ "$((10#${count}))" -eq 0 ]; then
            failure="evaluated zero checks"
        fi
    fi

    if [ -z "${failure}" ]; then
        CS_RESULT_LINES+=("semver: ${pkg}: ${check_line}")
    elif [ -n "${check_line}" ]; then
        CS_RESULT_LINES+=("semver: ${pkg}: ${check_line} -- ${failure}")
        CS_FAIL_PKGS+=("${pkg}")
        CS_FAIL_MSGS+=("${failure}")
    else
        CS_RESULT_LINES+=("semver: ${pkg}: ${failure}")
        CS_FAIL_PKGS+=("${pkg}")
        CS_FAIL_MSGS+=("${failure}")
    fi
}

check_semver_main() {
    local only_pkg=""

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --package)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --package requires a value." >&2
                    echo "Usage: check-semver.sh [--package <name>]" >&2
                    return 2
                fi
                only_pkg="$2"
                shift 2
                ;;
            *)
                echo "ERROR: unknown argument: $1" >&2
                echo "Usage: check-semver.sh [--package <name>]" >&2
                return 2
                ;;
        esac
    done

    SEMVER_CARGO_BIN="${SEMVER_CARGO_BIN:-cargo}"
    local publish_script="${PUBLISH_CRATES_SCRIPT:-${WORKSPACE_ROOT}/scripts/publish-crates.sh}"

    # Read the published package names through the sourcing seam: never a
    # second hard-coded copy of CRATES anywhere in this file.
    if [ ! -f "${publish_script}" ]; then
        echo "ERROR: scripts/publish-crates.sh not found at ${publish_script} -- cannot read the CRATES package list." >&2
        return 1
    fi
    CRATES=()
    # shellcheck source=scripts/publish-crates.sh
    PUBLISH_CRATES_LIB_ONLY=1 source "${publish_script}"
    if [ "${#CRATES[@]}" -eq 0 ]; then
        echo "ERROR: CRATES_EMPTY -- ${publish_script} defines no CRATES entries after sourcing; refusing to report a pass over no packages." >&2
        return 1
    fi

    local -a packages=("${CRATES[@]}")
    if [ -n "${only_pkg}" ]; then
        local found=0 name
        for name in "${packages[@]}"; do
            if [ "${name}" = "${only_pkg}" ]; then
                found=1
                break
            fi
        done
        if [ "${found}" -eq 0 ]; then
            echo "ERROR: --package ${only_pkg} is not a member of CRATES in ${publish_script}." >&2
            echo "Usage: check-semver.sh [--package <name>]" >&2
            return 2
        fi
        packages=("${only_pkg}")
    fi

    CS_RESULT_LINES=()
    CS_FAIL_PKGS=()
    CS_FAIL_MSGS=()

    local pkg
    for pkg in "${packages[@]}"; do
        _cs_check_package "${pkg}"
    done

    echo ""
    printf '%s\n' "${CS_RESULT_LINES[@]}"
    echo "semver: ${#packages[@]} package(s) checked against ${SEMVER_BASELINE_VERSION} at --release-type ${SEMVER_RELEASE_TYPE}; ${#CS_FAIL_PKGS[@]} failure(s)"

    if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
        # Workflow-command escaping: a property value escapes `,` as %2C; a
        # message joins lines with %0A.
        local joined="" line
        for line in "${CS_RESULT_LINES[@]}"; do
            joined="${joined:+${joined}%0A}${line}"
        done
        echo "::notice title=Semver check counts (vs ${SEMVER_BASELINE_VERSION}%2C ${SEMVER_RELEASE_TYPE})::${joined}"
        local i
        for i in "${!CS_FAIL_PKGS[@]}"; do
            echo "::error title=Semver Checks::${CS_FAIL_PKGS[$i]}: ${CS_FAIL_MSGS[$i]}"
        done
    fi

    if [ "${#CS_FAIL_PKGS[@]}" -gt 0 ]; then
        return 1
    fi
    return 0
}

if [ "${CHECK_SEMVER_LIB_ONLY:-0}" != "1" ]; then
    check_semver_main "$@"
fi
