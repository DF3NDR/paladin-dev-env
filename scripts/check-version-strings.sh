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
#   2. `Current version: X` lines (markdown emphasis and backticks around the
#      token are stripped, then one trailing `.`).
#   3. Under k8s/ only: `version:` labels (an optional leading `v` is accepted;
#      `apiVersion:` is never a label) and the `image:` tags of `paladin-server`
#      and `paladin` (a tag that starts with an optional `v`, digits, a dot and
#      digits; compared exactly, so `paladin-server:v1.2.3` is reported).
#   4. `info.version` of crates/paladin-web/openapi.json.
#
# Two Cargo shapes are deliberately NOT matched (neither exists in the tree; the
# harness pins each choice): a `[dependencies.<name>]` table with `version =`
# on a later line, and a `version =` placed after a multi-line value inside an
# inline table. An inline table is read from its opening brace to the first
# closing brace or the end of that line.
#
# Never vacuous: besides MISMATCH and OK there are four named failures.
# NO_VERSION (the root Cargo.toml carries no version string), STALE_EXEMPTION
# (an EXEMPT path is not a file), ZERO_FILES (the scan set is empty) and
# CLASS_UNMATCHED (a class matched nothing anywhere, so a rotted pattern cannot
# make the gate pass). The status line is the first applicable of them in that
# order, then MISMATCH, then OK; every finding and every named failure is listed
# before exit, so nothing is hidden behind the first.
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
import json
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

OPENAPI = "crates/paladin-web/openapi.json"

CLASS_NAMES = {
    1: "Cargo pins",
    2: "Current version lines",
    3: "k8s labels and image tags",
    4: "openapi.json info.version",
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
counts = {n: 0 for n in CLASS_NAMES}


def record(klass, path, lineno, found):
    """Count one candidate string; report it when it is not the workspace version."""
    counts[klass] += 1
    if want is not None and found != want:
        findings.append((path, lineno, found))


# --- Class 1: Cargo pins. ---
# A key (no word or `-` character before it), `=`, then a quoted string or `{`
# and the rest of the line up to the first `}` (or the end of the line).
PIN_RE = re.compile(
    r'(?<![\w\-])([A-Za-z0-9_][A-Za-z0-9_\-]*)\s*=\s*'
    r'(?:"([^"\n]*)"|\{(.*?)(?:\}|$))'
)
PACKAGE_RE = re.compile(r'(?<![\w\-])package\s*=\s*"([^"]*)"')
VERSION_RE = re.compile(r'(?<![\w\-])version\s*=\s*"([^"]*)"')

# --- Class 2: `Current version: **X**.` lines. ---
CURRENT_RE = re.compile(r'Current version:\s*[*`_]*v?([^*`_\s]+)')

# --- Class 3 (k8s/ only): version labels and first-party image tags. ---
# The first non-blank text must be `version:` / `image:`, so `apiVersion:` and
# prose never match.
LABEL_RE = re.compile(r"""^\s*version:\s*["']?v?([^\s"'#]+)["']?\s*(?:#.*)?$""")
IMAGE_RE = re.compile(
    r"""^\s*(?:-\s+)?image:\s*["']?(?:[\w.\-]+(?::\d+)?/)*"""
    r"""(?:paladin-server|paladin):([^\s"'#]+)"""
)
CANDIDATE_TAG_RE = re.compile(r'^v?\d+\.\d+')

for path in sorted(texts):
    if path == OPENAPI:
        continue
    in_k8s = path.startswith("k8s/")
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
            if found is not None:
                record(1, path, lineno, found)
        for match in CURRENT_RE.finditer(line):
            token = match.group(1)
            if token.endswith("."):
                token = token[:-1]
            record(2, path, lineno, token)
        if in_k8s:
            label = LABEL_RE.match(line)
            if label:
                record(3, path, lineno, label.group(1))
            image = IMAGE_RE.match(line)
            if image and CANDIDATE_TAG_RE.match(image.group(1)):
                record(3, path, lineno, image.group(1))

# --- Class 4: openapi.json info.version. ---
if OPENAPI in texts:
    openapi_lines = texts[OPENAPI].splitlines()
    try:
        info_version = json.loads(texts[OPENAPI]).get("info", {}).get("version")
    except (ValueError, AttributeError):
        info_version = None
    if not isinstance(info_version, str):
        info_version = "<missing>"
    # Report the first line at or after the one holding "info" that names "version".
    info_at = next((i for i, text in enumerate(openapi_lines) if '"info"' in text), 0)
    version_at = next(
        (i for i in range(info_at, len(openapi_lines)) if '"version"' in openapi_lines[i]),
        info_at,
    )
    counts[4] += 1
    if want is not None and info_version != want:
        findings.append((OPENAPI, version_at + 1, info_version))

findings.sort(key=lambda f: (f[0], f[1]))

# --- Named failures, in status-line priority order. ---
named = []  # (token, detail line)
if want is None:
    named.append(("NO_VERSION",
                  "NO_VERSION: the root Cargo.toml has no [package].version or "
                  "[workspace.package].version string, so there is no version to "
                  "hold the copyable strings to."))
for exempt_path in sorted(EXEMPT):
    if not os.path.isfile(os.path.join(root, exempt_path)):
        named.append(("STALE_EXEMPTION",
                      f"STALE_EXEMPTION: {exempt_path} ({EXEMPT[exempt_path]}) names "
                      "a file that does not exist; remove the entry or fix the path."))
if not texts:
    named.append(("ZERO_FILES",
                  f"ZERO_FILES: scanned 0 file(s) under {root}; a broken scan set "
                  "is a named failure, never a pass."))
for klass in sorted(CLASS_NAMES):
    if counts[klass] == 0:
        named.append(("CLASS_UNMATCHED",
                      f"CLASS_UNMATCHED: class {klass} ({CLASS_NAMES[klass]}) "
                      f"matched nothing in {len(texts)} scanned file(s)"))

if named or findings:
    print(named[0][0] if named else "MISMATCH")
    for _, detail in named:
        print(detail)
    for path, lineno, found in findings:
        print(f"{path}:{lineno}: found '{found}', want '{want}'")
    sys.exit(0)

print("OK")
per_class = "; ".join(f"class {n} {CLASS_NAMES[n]}: {counts[n]}" for n in sorted(CLASS_NAMES))
print(f"OK: {len(texts)} file(s) scanned, every copyable version string equals "
      f"'{want}' ({per_class}; exempt files: {len(EXEMPT)})")
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
