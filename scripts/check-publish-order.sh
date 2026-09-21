#!/usr/bin/env bash
# check-publish-order.sh
#
# The publish-order gate (SHIP-06, ROADMAP Phase 37.1 success criterion 3).
# Verifies that scripts/publish-crates.sh's CRATES array -- the order the
# real per-crate `cargo publish` loop actually runs in -- is a valid
# dependency-first ordering of the workspace's publishable crates: every
# versioned workspace dependency (normal, build AND dev) of a crate must
# appear EARLIER in CRATES than that crate itself, and the CRATES array and
# the cargo-metadata-derived publishable set must be exactly equal in both
# directions.
#
# Why this gate exists and what it catches that the existing dry run does
# not: `cargo publish --workspace --dry-run` (the existing `make
# publish-dry-run` target) resolves sibling crates from a LOCAL overlay of
# the workspace, so a crate whose dev-dependency on a sibling has not yet
# been published upstream still resolves fine locally -- the dry run is
# green and blind to publish order. The real per-crate `cargo publish -p
# <name>` loop this repo actually runs resolves against the crates.io
# registry: a versioned dependency (including a dev-dependency, which cargo
# still records -- and validates against the index -- in the published
# manifest) on a crate not yet published fails hard. This is exactly the
# `v0.10.0` failure: paladin-battalion (CRATES position 4) carried versioned
# dev-dependencies on paladin-llm (position 5) and paladin-storage (position
# 10), both published AFTER it, and `cargo package`/`cargo publish` for
# paladin-battalion failed resolving them. This gate exercises the carrier's
# ordering (the real per-crate resolution constraint), not the overlay's.
#
# This script is offline when --metadata-json is supplied, and otherwise
# makes exactly one local `cargo metadata` call -- it never talks to the
# network. It accumulates every violation into one report rather than
# stopping at the first, so a run with several broken edges gets one report
# naming all of them, never a fix-one-rerun-find-the-next loop. It only
# reads: given the same CRATES array and the same metadata, running it
# twice produces byte-identical output and the same exit code.
#
# The publish order is read through scripts/publish-crates.sh's own
# PUBLISH_CRATES_LIB_ONLY=1 sourcing seam -- this file never keeps a second,
# independently-maintained copy of the CRATES array, which would drift
# silently and defeat the entire point of the gate.
#
# A "versioned workspace dependency" is exactly a dependency entry from
# `cargo metadata --format-version 1 --no-deps` with a non-null `path` and a
# `req` other than the match-all requirement "*" -- a path-only dependency
# (no `version =` field in the manifest) always reads `req: "*"` and is
# excluded, matching the shape the path-only battalion fix produces. Kinds
# normal (`kind: null`), dev (`kind: "dev"`) and build (`kind: "build"`) are
# all checked -- normal-only would have missed the actual v0.10.0 defect,
# which was entirely in [dev-dependencies].
#
# Dependencies are resolved to a crate name via the dependency entry's
# `name` field, never its `rename` field: `rename` carries the LOCAL alias a
# manifest uses (for example `paladin-ai`'s `core = { package =
# "paladin-ai-core", ... }` reads `name: "paladin-ai-core", rename:
# "paladin-core"`), while `name` is always the real package name -- the one
# the CRATES array and the crates.io registry both use. Reading `rename`
# here would resolve several of this workspace's core-crate dependents to a
# name that appears nowhere in CRATES and silently break the whole gate.
#
# The publishable set is derived the same way check-release-consistency.sh
# already does: a package is publishable when `cargo metadata` reports its
# `publish` field as `null` (manifest omits `publish` entirely); a package
# carrying `publish = false` (reported as an empty list) is excluded by
# design, never counted.
#
# A crate present in CRATES but absent from the publishable set, or
# publishable but absent from CRATES, is reported by the set-equality check
# and is never ALSO reported as an ordering violation for the same crate --
# the two failure classes are kept distinct so a report never double-counts
# one broken crate as two separate problems.
#
# Sourcing seam: set CHECK_PUBLISH_ORDER_LIB_ONLY=1 before sourcing this
# file to load the check_publish_order_main function without executing it,
# matching the seam check-release-consistency.sh already establishes.
#
# Usage:  ./scripts/check-publish-order.sh [--metadata-json <path>]
#         --metadata-json, when supplied, is read instead of invoking
#         `cargo metadata` -- the fixture seam this script's regression test
#         (tests/scripts/check-publish-order_test.sh) uses so it never
#         invokes cargo or touches the network. A missing or unreadable file
#         named by this flag is a hard, named failure -- never an
#         empty-input silent pass. An unrecognised flag is a usage error.
# Exit:   0 if every publishable crate's versioned workspace dependencies
#         (normal, dev and build) all resolve to an earlier CRATES position,
#         AND the CRATES array and the metadata-derived publishable set are
#         exactly equal; non-zero for VIOLATIONS_FOUND (one or more
#         set-equality or ordering violations -- the report lists every
#         one), METADATA_READ_FAILED, CRATES_EMPTY, a missing
#         --metadata-json file, or a usage error (unknown flag / missing
#         python3).

set -euo pipefail

check_publish_order_main() {
    local WORKSPACE_ROOT
    WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

    local METADATA_JSON=""

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --metadata-json)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --metadata-json requires a value." >&2
                    return 1
                fi
                METADATA_JSON="$2"
                shift 2
                ;;
            *)
                echo "ERROR: unknown flag: $1" >&2
                echo "Usage: check-publish-order.sh [--metadata-json <path>]" >&2
                return 1
                ;;
        esac
    done

    if ! command -v python3 >/dev/null 2>&1; then
        echo "ERROR: python3 is required for Cargo metadata parsing." >&2
        return 1
    fi

    # Read the publish order through the sourcing seam -- never a second
    # hard-coded copy of CRATES anywhere in this file.
    local PUBLISH_CRATES_SCRIPT="${WORKSPACE_ROOT}/scripts/publish-crates.sh"
    if [ ! -f "${PUBLISH_CRATES_SCRIPT}" ]; then
        echo "ERROR: scripts/publish-crates.sh not found at ${PUBLISH_CRATES_SCRIPT} -- cannot read the CRATES publish order." >&2
        return 1
    fi
    # shellcheck source=scripts/publish-crates.sh
    PUBLISH_CRATES_LIB_ONLY=1 source "${PUBLISH_CRATES_SCRIPT}"
    if [ "${#CRATES[@]}" -eq 0 ]; then
        echo "ERROR: CRATES_EMPTY -- scripts/publish-crates.sh's CRATES array is unset or empty after sourcing; refusing to verify a publish order against nothing." >&2
        return 1
    fi

    local METADATA_PATH="" CLEANUP_METADATA=""
    trap '[ -n "${CLEANUP_METADATA}" ] && rm -f "${CLEANUP_METADATA}"' RETURN

    if [ -n "${METADATA_JSON}" ]; then
        if [ ! -f "${METADATA_JSON}" ]; then
            echo "ERROR: --metadata-json file not found: ${METADATA_JSON}" >&2
            return 1
        fi
        METADATA_PATH="${METADATA_JSON}"
    else
        METADATA_PATH="$(mktemp "${TMPDIR:-/tmp}/check-publish-order-metadata.XXXXXX.json")"
        CLEANUP_METADATA="${METADATA_PATH}"
        if ! (cd "${WORKSPACE_ROOT}" && cargo metadata --no-deps --format-version 1) > "${METADATA_PATH}"; then
            echo "ERROR: 'cargo metadata --no-deps --format-version 1' failed in ${WORKSPACE_ROOT}." >&2
            return 1
        fi
    fi

    local REPORT
    REPORT=$(python3 - "${METADATA_PATH}" "${CRATES[@]}" <<'PY'
import json
import sys

metadata_path = sys.argv[1]
crates_order = sys.argv[2:]

try:
    with open(metadata_path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
except (OSError, json.JSONDecodeError) as exc:
    print("METADATA_READ_FAILED")
    print(f"FAIL: could not read/parse metadata JSON at {metadata_path}: {exc}")
    sys.exit(0)

packages = data.get("packages", []) if isinstance(data, dict) else []
publishable = {
    p.get("name"): p
    for p in packages
    if isinstance(p, dict) and p.get("publish") is None
}

order_set = set(crates_order)
publishable_set = set(publishable.keys())

# Set-equality, both directions -- kept as its own failure class so a crate
# missing from one side is never ALSO reported as an ordering violation.
missing_from_order = sorted(publishable_set - order_set)
missing_from_publishable = sorted(order_set - publishable_set)

position = {name: i for i, name in enumerate(crates_order)}

order_violations = []
for name in crates_order:
    pkg = publishable.get(name)
    if pkg is None:
        # Already reported via missing_from_publishable above.
        continue
    deps = pkg.get("dependencies", []) or []
    this_pos = position[name]
    for dep in deps:
        if not isinstance(dep, dict):
            continue
        # A versioned workspace dependency: resolves to a local path AND
        # carries a requirement other than the match-all "*". Resolved via
        # `name` (the real package name), never `rename` (the local alias).
        if dep.get("path") is None:
            continue
        if dep.get("req") == "*":
            continue
        kind = dep.get("kind")
        if kind not in (None, "dev", "build"):
            continue
        dep_name = dep.get("name")
        if dep_name not in position:
            # Already reported via missing_from_order above.
            continue
        dep_pos = position[dep_name]
        if dep_pos > this_pos:
            kind_label = kind if kind else "normal"
            order_violations.append((name, dep_name, kind_label, this_pos + 1, dep_pos + 1))

if missing_from_order or missing_from_publishable or order_violations:
    print("VIOLATIONS_FOUND")
    if missing_from_order:
        print(
            f"FAIL: {len(missing_from_order)} publishable crate(s) exist in the workspace "
            f"but are absent from CRATES in scripts/publish-crates.sh:"
        )
        for name in missing_from_order:
            print(f"  - {name}: publishable per cargo metadata but missing from CRATES")
    if missing_from_publishable:
        print(
            f"FAIL: {len(missing_from_publishable)} crate(s) are listed in CRATES but are not "
            f"in the publishable set per cargo metadata:"
        )
        for name in missing_from_publishable:
            print(f"  - {name}: present in CRATES but not publishable (or not found) per cargo metadata")
    if order_violations:
        print(
            f"FAIL: {len(order_violations)} publish-order violation(s) -- a versioned workspace "
            f"dependency is ordered AFTER its dependent in CRATES:"
        )
        for name, dep_name, kind_label, this_pos, dep_pos in order_violations:
            print(
                f"  - {name} (CRATES position {this_pos}) --{kind_label}--> {dep_name} "
                f"(CRATES position {dep_pos}): {dep_name} must publish before {name} but is "
                f"ordered after it in CRATES"
            )
    sys.exit(0)

print("OK")
print(
    f"{len(crates_order)} crate(s) checked in publish order; every versioned workspace "
    f"dependency (normal, dev and build) resolves to an earlier CRATES position, and CRATES "
    f"exactly matches the cargo-metadata publishable set."
)
sys.exit(0)
PY
)

    local STATUS_LINE DETAIL
    STATUS_LINE="${REPORT%%$'\n'*}"
    DETAIL=$(tail -n +2 <<<"${REPORT}")

    if [ "${STATUS_LINE}" = "OK" ]; then
        echo "✅ OK: ${DETAIL}"
        return 0
    else
        echo "❌ Publish-order check failed (${STATUS_LINE})"
        echo ""
        echo "${DETAIL}"
        echo ""
        echo "If this failure is unexpected:"
        echo "  1. A crate absent from CRATES: add it to scripts/publish-crates.sh's CRATES"
        echo "     array at a position after every crate it depends on."
        echo "  2. A crate in CRATES but not publishable: either it should carry"
        echo "     publish = false and be removed from CRATES, or cargo metadata could not"
        echo "     find it -- check the workspace member list."
        echo "  3. An ordering violation: move the dependency earlier in CRATES, or (as with"
        echo "     the v0.10.0 paladin-battalion defect) drop the version requirement from"
        echo "     the offending workspace dependency so it publishes path-only and imposes"
        echo "     no registry-ordering constraint."
        echo "  4. METADATA_READ_FAILED: the --metadata-json file (or a live cargo metadata"
        echo "     call) did not produce parseable JSON -- see the message above for detail."
        echo "  5. If this guard is wrong about a crate or the order, fix the guard rather"
        echo "     than working around it."
        return 1
    fi
}

if [ "${CHECK_PUBLISH_ORDER_LIB_ONLY:-0}" != "1" ]; then
    check_publish_order_main "$@"
fi
