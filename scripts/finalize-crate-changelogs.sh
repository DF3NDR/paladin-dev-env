#!/usr/bin/env bash
# finalize-crate-changelogs.sh
#
# Stamps a dated "## [X.Y.Z]" section into the changelog of every publishable
# Cargo workspace package -- including the root package's CHANGELOG.md --
# for the version being released (D-09). This is what makes the strict,
# no-exemptions check-release-consistency.sh gate satisfiable in one
# invocation instead of a manual edit of every publishable package's
# changelog, each of which would otherwise lack a section for the release
# version and be refused by the gate.
#
# Per RESEARCH.md Pitfall 5 and release.toml's own comment: cargo-release's
# `pre-release-replacements` is package-scoped, not workspace-scoped, so a
# root-level replacement would run once per crate and duplicate the heading
# -- this script is the carrier instead, extending the same
# perl-substitution idea `make release` already applied to the root
# CHANGELOG.md into a loop over every publishable package's own changelog.
#
# Enumeration: publishable packages come from `cargo metadata --no-deps
# --format-version 1` (never a hardcoded list or a crates/* glob) -- the
# same enumeration scripts/check-release-consistency.sh uses, so the two
# scripts can never disagree about which files matter. A package's
# changelog is resolved as CHANGELOG.md in the directory holding its
# manifest_path, which is what makes the root package resolve to the root
# CHANGELOG.md and each crate to its own crates/<name>/CHANGELOG.md,
# without any special-casing of the root package.
#
# Per-file disposition, in this order:
#   1. If a section for the target version already exists (a line beginning
#      with the literal "## [X.Y.Z]" prefix, regardless of any trailing
#      " - date" text), the file is left untouched and recorded
#      already-finalized. This is what makes a second run of the identical
#      invocation a no-op: byte-identical output, exit 0.
#   2. Else, if the "## [Unreleased]" anchor heading exists, the new dated
#      section is inserted immediately after it (preserving the anchor), so
#      the file stays in reverse-chronological order with the newest
#      release first even when an older versioned section already exists
#      below the anchor. Recorded finalized.
#   3. Else (no anchor, and no existing section for this version), the file
#      is left untouched and the package is recorded as a named failure.
#      This script never guesses a target line to append at.
#
# Every package is processed even if an earlier one failed -- failures are
# accumulated and reported together at the end (collect-then-report), never
# fail-fast. Discovering zero publishable packages is itself a named
# ZERO_PACKAGES failure, never a report of success over an empty set,
# matching the convention check-release-consistency.sh and
# check-changelogs.sh already establish for this repo's gate/tooling
# scripts.
#
# Sourcing seam: set FINALIZE_CRATE_CHANGELOGS_LIB_ONLY=1 before sourcing
# this file to load the finalize_crate_changelogs_main function without
# executing it.
#
# Usage:  ./scripts/finalize-crate-changelogs.sh --version <X.Y.Z>
#             [--date <YYYY-MM-DD>] [--metadata-json <path>]
#             [--workspace-root <path>]
#         --version is required (no leading "v" -- pass the bare semver
#         string, e.g. 0.9.0 or 0.9.0-rc.1; this script does not strip a
#         leading "v" the way check-release-consistency.sh's --tag does,
#         since callers pass $(VERSION) directly, never a tag ref).
#         --date defaults to today (YYYY-MM-DD, UTC-naive `date +%Y-%m-%d`).
#         --metadata-json, when supplied, is read instead of invoking
#         `cargo metadata` -- the fixture seam this script's regression
#         test uses so it never invokes cargo and never touches the real
#         tree (every fixture's manifest_path resolves into a scratch
#         directory, not crates/ or the repo root).
#         --workspace-root overrides the repo root `cargo metadata` is run
#         from (default: derived from BASH_SOURCE); it has no effect when
#         --metadata-json is supplied. An unrecognised flag is a usage
#         error.
# Exit:   0 if every publishable package's changelog carries (or already
#         carried) a section for the target version (status OK); non-zero
#         for FINALIZE_FAILED (one or more packages could not be
#         finalized), ZERO_PACKAGES, MISSING_VERSION, or a usage error
#         (unknown flag / missing python3).

set -euo pipefail

finalize_crate_changelogs_main() {
    local WORKSPACE_ROOT
    WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

    local VERSION="" DATE="" METADATA_JSON="" WORKSPACE_ROOT_ARG=""

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --version)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --version requires a value." >&2
                    return 1
                fi
                VERSION="$2"
                shift 2
                ;;
            --date)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --date requires a value." >&2
                    return 1
                fi
                DATE="$2"
                shift 2
                ;;
            --metadata-json)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --metadata-json requires a value." >&2
                    return 1
                fi
                METADATA_JSON="$2"
                shift 2
                ;;
            --workspace-root)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --workspace-root requires a value." >&2
                    return 1
                fi
                WORKSPACE_ROOT_ARG="$2"
                shift 2
                ;;
            *)
                echo "ERROR: unknown flag: $1" >&2
                echo "Usage: finalize-crate-changelogs.sh --version <X.Y.Z> [--date <YYYY-MM-DD>] [--metadata-json <path>] [--workspace-root <path>]" >&2
                return 1
                ;;
        esac
    done

    if [ -n "${WORKSPACE_ROOT_ARG}" ]; then
        WORKSPACE_ROOT="${WORKSPACE_ROOT_ARG}"
    fi

    if ! command -v python3 >/dev/null 2>&1; then
        echo "ERROR: python3 is required for Cargo metadata parsing and changelog editing." >&2
        return 1
    fi

    local REPORT
    if [ -z "${VERSION}" ]; then
        REPORT=$'MISSING_VERSION\nFAIL: --version is required. Usage: finalize-crate-changelogs.sh --version X.Y.Z (or --metadata-json/--workspace-root for local runs).'
    else
        if [ -z "${DATE}" ]; then
            DATE="$(date +%Y-%m-%d)"
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
            METADATA_PATH="$(mktemp "${TMPDIR:-/tmp}/finalize-crate-changelogs-metadata.XXXXXX.json")"
            CLEANUP_METADATA="${METADATA_PATH}"
            if ! (cd "${WORKSPACE_ROOT}" && cargo metadata --no-deps --format-version 1) > "${METADATA_PATH}"; then
                echo "ERROR: 'cargo metadata --no-deps --format-version 1' failed in ${WORKSPACE_ROOT}." >&2
                return 1
            fi
        fi

        REPORT=$(python3 - "${METADATA_PATH}" "${VERSION}" "${DATE}" <<'PY'
import json
import os
import sys

metadata_path = sys.argv[1]
version = sys.argv[2]
date = sys.argv[3]

try:
    with open(metadata_path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
except (OSError, json.JSONDecodeError) as exc:
    print("FINALIZE_FAILED")
    print(f"FAIL: could not read/parse metadata JSON at {metadata_path}: {exc}")
    sys.exit(0)

packages = data.get("packages", []) if isinstance(data, dict) else []
publishable = [
    p for p in packages
    if isinstance(p, dict) and p.get("publish") is None
]

if not publishable:
    print("ZERO_PACKAGES")
    print("FAIL: zero publishable packages discovered from cargo metadata -- a broken "
          "enumeration is a named failure, never a pass over an empty set. A package is "
          "publishable when its manifest omits `publish` (cargo metadata reports this as "
          "`null`); one carrying `publish = false` (reported as an empty list) is excluded "
          "by design, not a bug.")
    sys.exit(0)

publishable = sorted(publishable, key=lambda p: p.get("name", ""))

anchor = "## [Unreleased]"
heading_prefix = f"## [{version}]"
new_heading = f"## [{version}] - {date}"

results = []    # (name, status) for every package, in order
failures = []   # detail strings for packages that could not be finalized

for p in publishable:
    name = p.get("name", "<unknown>")
    manifest_path = p.get("manifest_path") or ""

    if not manifest_path:
        results.append((name, "failed"))
        failures.append(f"{name}: metadata entry has no manifest_path")
        continue

    changelog_path = os.path.join(os.path.dirname(manifest_path), "CHANGELOG.md")

    if not os.path.isfile(changelog_path):
        results.append((name, "failed"))
        failures.append(f"{name}: no CHANGELOG.md found at {changelog_path}")
        continue

    with open(changelog_path, "r", encoding="utf-8") as fh:
        content = fh.read()

    # A line beginning with the exact heading prefix means a section for
    # this version already exists (regardless of the date suffix that
    # follows it) -- already-finalized, write nothing, never duplicate.
    already_finalized = any(
        line.startswith(heading_prefix) for line in content.splitlines()
    )
    if already_finalized:
        results.append((name, "already-finalized"))
        continue

    if anchor not in content:
        results.append((name, "failed"))
        failures.append(
            f"{name}: no '## [Unreleased]' anchor found in {changelog_path} -- "
            "refusing to guess an insertion point"
        )
        continue

    # Insert immediately after the anchor, preserving it. Using count=1
    # guarantees exactly one insertion even if "## [Unreleased]" somehow
    # appears more than once in a malformed file.
    replacement = f"{anchor}\n\n{new_heading}"
    new_content = content.replace(anchor, replacement, 1)
    with open(changelog_path, "w", encoding="utf-8") as fh:
        fh.write(new_content)
    results.append((name, "finalized"))

if failures:
    print("FINALIZE_FAILED")
    print(f"FAIL: {len(failures)} of {len(publishable)} publishable package(s) could not "
          f"be finalized for version {version!r}:")
    for detail in failures:
        print(f"  - {detail}")
    print("")
    print("Per-package results:")
    for name, status in results:
        print(f"  - {name}: {status}")
    sys.exit(0)

print("OK")
print(f"{len(publishable)} publishable package(s) checked for version {version!r} "
      f"(date {date}):")
for name, status in results:
    print(f"  - {name}: {status}")
sys.exit(0)
PY
)
    fi

    local STATUS_LINE DETAIL
    STATUS_LINE=$(head -n1 <<<"${REPORT}")
    DETAIL=$(tail -n +2 <<<"${REPORT}")

    if [ "${STATUS_LINE}" = "OK" ]; then
        echo "✅ OK: ${DETAIL}"
        return 0
    else
        echo "❌ Changelog finalize failed (${STATUS_LINE})"
        echo ""
        echo "${DETAIL}"
        echo ""
        echo "If this failure is unexpected:"
        echo "  1. FINALIZE_FAILED: a publishable package's changelog has no '## "
        echo "     [Unreleased]' anchor to insert after, or has no CHANGELOG.md file at"
        echo "     all -- add the missing anchor/file rather than guessing an insertion"
        echo "     point."
        echo "  2. ZERO_PACKAGES: cargo metadata enumerated no publishable packages --"
        echo "     check for a broken workspace or an accidental publish = false on"
        echo "     every crate."
        echo "  3. MISSING_VERSION: pass --version X.Y.Z."
        echo "  4. If this script is wrong about a crate or the version, fix the script"
        echo "     rather than working around it."
        return 1
    fi
}

if [ "${FINALIZE_CRATE_CHANGELOGS_LIB_ONLY:-0}" != "1" ]; then
    finalize_crate_changelogs_main "$@"
fi
