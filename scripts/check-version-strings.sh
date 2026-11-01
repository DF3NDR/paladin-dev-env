#!/usr/bin/env bash
# check-version-strings.sh
#
# Offline, check-only gate that holds every copyable version string in the
# user-facing material to the workspace version (Phase 54, FIRST-03,
# D-06..D-12). A reader who pastes `paladin-ai = "0.5"` from the docs into a
# fresh crate gets a different crate than the one the docs describe; this gate
# turns that drift into a red `make check-gates` and a red required CI job
# instead of a one-off sweep that rots.
#
# The wanted version is the root Cargo.toml [package].version, read with
# tomllib (falling back to [workspace.package].version when the package
# inherits it). The workspace package names are the [package].name of the root
# and of every crates/*/Cargo.toml. Equality is exact string equality on the
# version token: there is no semver normalisation, so `1.2.3` differs from
# `1.2.30`, from `1.2` and from `1.2.3-rc.1`.
#
# Scanned set (every file read as UTF-8; a file that is not valid UTF-8 is
# skipped, never a crash): the root README.md, every crates/*/README.md, every
# file under docs/src and k8s/, and crates/paladin-web/openapi.json.
#
# Pattern classes:
#   1. Cargo pins whose key, or `package =` value, is a workspace package name:
#        name = "X"
#        name = { version = "X", ... }       (including an inline table whose
#                                              closing brace is on a later line)
#        alias = { package = "<ws>", version = "X" }
#      matched inside code fences and `//!` lines too.
#
# Two shapes are deliberately NOT matched (neither exists in the tree; the
# harness pins each choice): a `[dependencies.<name>]` table with `version =`
# on a later line, and a `version =` placed after a multi-line value inside an
# inline table. An inline table is read from its opening brace to the first
# closing brace or the end of that line.
#
# EXEMPT (per file, with one reason each, D-08): the in-script EXEMPT list holds
# exactly one file, the historical migration guide. A stale pin there passes;
# identical content in any other scanned file fails.
#
# Offline: no network, no cargo; python3 with tomllib only. Check-only: this
# script never writes to, rewrites or reformats a file it scans, and it has no
# option parsing and no write mode (D-12). To fix a finding, edit the string to
# the workspace version yourself.
#
# Test-only seam: VERSION_STRINGS_ROOT relocates every input (Cargo.toml, the
# manifests, the scanned files and the EXEMPT lookup, whose paths are
# root-relative) to a fixture tree. CI and the Makefile target set no
# environment, so the default is the repository root.
#
# Usage:  ./scripts/check-version-strings.sh
# Exit:   0 when every matched string equals the workspace version, 1 otherwise.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${VERSION_STRINGS_ROOT:-${WORKSPACE_ROOT}}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for Cargo.toml parsing." >&2
    exit 1
fi

echo "🔍 Checking copyable version strings against the workspace version ..."

REPORT=$(python3 - "${ROOT}" <<'PY'
import glob
import os
import re
import sys
import tomllib

root = sys.argv[1]

# Files exempt from every class, each with the reason it is exempt (D-08).
# Add an entry only with a reason; review any new entry.
EXEMPT = {
    "docs/src/api-reference/migration-guide.md":
        "historical 0.4 upgrade rows; the guide stops at v0.5.0 (FIRST-02)",
}


def rel(path):
    return os.path.relpath(path, root).replace(os.sep, "/")


def load_manifest(path):
    try:
        with open(path, "rb") as fh:
            return tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError):
        return None


# --- The wanted version and the workspace package names. ---
root_manifest = load_manifest(os.path.join(root, "Cargo.toml"))
want = None
names = set()
if root_manifest is not None:
    package = root_manifest.get("package", {})
    want = package.get("version")
    if not isinstance(want, str):
        # `version.workspace = true` inherits from [workspace.package].
        want = root_manifest.get("workspace", {}).get("package", {}).get("version")
    if not isinstance(want, str):
        want = None
    if isinstance(package.get("name"), str):
        names.add(package["name"])
for manifest_path in sorted(glob.glob(os.path.join(root, "crates", "*", "Cargo.toml"))):
    manifest = load_manifest(manifest_path)
    if manifest is None:
        continue
    name = manifest.get("package", {}).get("name")
    if isinstance(name, str):
        names.add(name)

if want is None:
    print("NO_VERSION")
    print("NO_VERSION: the root Cargo.toml has no [package].version or "
          "[workspace.package].version string, so there is no version to hold "
          "the copyable strings to.")
    sys.exit(0)

# --- The scan set: sorted, deterministic, minus EXEMPT. ---
scan = []
if os.path.isfile(os.path.join(root, "README.md")):
    scan.append("README.md")
scan += sorted(rel(p) for p in glob.glob(os.path.join(root, "crates", "*", "README.md")))
for top in ("docs/src", "k8s"):
    for dirpath, dirnames, filenames in os.walk(os.path.join(root, top)):
        dirnames.sort()
        for filename in sorted(filenames):
            scan.append(rel(os.path.join(dirpath, filename)))
OPENAPI = "crates/paladin-web/openapi.json"
if os.path.isfile(os.path.join(root, OPENAPI)):
    scan.append(OPENAPI)
scan = sorted(set(scan) - set(EXEMPT))

texts = {}
for path in scan:
    try:
        with open(os.path.join(root, path), "r", encoding="utf-8") as fh:
            texts[path] = fh.read()
    except (UnicodeDecodeError, OSError):
        continue  # not valid UTF-8 (or unreadable): skipped, never a crash

findings = []  # (path, line, found)
counts = {1: 0}

# --- Class 1: Cargo pins. ---
# A key (no word or `-` character before it), `=`, then a quoted string or `{`
# and the rest of the line up to the first `}` (or the end of the line).
PIN_RE = re.compile(
    r'(?<![\w\-])([A-Za-z0-9_][A-Za-z0-9_\-]*)\s*=\s*'
    r'(?:"([^"\n]*)"|\{(.*?)(?:\}|$))'
)
PACKAGE_RE = re.compile(r'(?<![\w\-])package\s*=\s*"([^"]*)"')
VERSION_RE = re.compile(r'(?<![\w\-])version\s*=\s*"([^"]*)"')

for path in sorted(texts):
    if path == OPENAPI:
        continue
    for lineno, line in enumerate(texts[path].splitlines(), start=1):
        for match in PIN_RE.finditer(line):
            key, bare, table = match.group(1), match.group(2), match.group(3)
            found = None
            if bare is not None:
                if key in names:
                    found = bare
            else:
                package_match = PACKAGE_RE.search(table)
                target = package_match.group(1) if package_match else key
                if target in names:
                    version_match = VERSION_RE.search(table)
                    if version_match:
                        found = version_match.group(1)
            if found is None:
                continue
            counts[1] += 1
            if found != want:
                findings.append((path, lineno, found))

findings.sort(key=lambda f: (f[0], f[1]))

if not texts:
    print("ZERO_FILES")
    print(f"ZERO_FILES: scanned 0 file(s) under {root}; a broken scan set is a "
          "named failure, never a pass.")
    sys.exit(0)

if findings:
    print("MISMATCH")
    for path, lineno, found in findings:
        print(f"{path}:{lineno}: found '{found}', want '{want}'")
    sys.exit(0)

print("OK")
print(f"OK: {len(texts)} file(s) scanned, every copyable version string equals "
      f"'{want}' (class 1 Cargo pins: {counts[1]}; exempt files: {len(EXEMPT)})")
sys.exit(0)
PY
)

STATUS_LINE=$(head -n1 <<<"${REPORT}")
DETAIL=$(tail -n +2 <<<"${REPORT}")

if [ "${STATUS_LINE}" = "OK" ]; then
    echo "✅ ${DETAIL}"
    exit 0
else
    echo "❌ Version-string check failed (${STATUS_LINE})"
    echo ""
    echo "${DETAIL}"
    echo ""
    echo "If this failure is unexpected:"
    echo "  1. Edit each reported string to the workspace version. This gate never"
    echo "     edits a file; the checklist above is one file:line per finding."
    echo "  2. Exempt a whole file only by adding it to EXEMPT in this script with a"
    echo "     one-line reason (D-08); do not exempt a file to silence a stale pin."
    exit 1
fi
