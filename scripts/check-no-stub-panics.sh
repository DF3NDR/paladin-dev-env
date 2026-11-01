#!/usr/bin/env bash
# check-no-stub-panics.sh
#
# Offline, check-only gate that holds every published crate free of stub
# panics (Phase 55, FIXES-01, D-01). A `todo!()` or `unimplemented!()` that is
# reachable outside test code takes down the caller's thread the first time
# the unfinished path runs (B2, DF-03: the twenty ContentListRepository stubs).
# This gate turns a new stub into a red `make check-gates` and a red CI job
# instead of a one-off sweep that rots.
#
# The rule: a `todo!` or `unimplemented!` macro token on a code line of any
# `.rs` file under `src/` of a published package is a finding, printed as
# `<path>:<line>: ...`, and the script exits 1. A published package is one whose
# manifest `[package] publish` is not `false`: the root Cargo.toml plus every
# crates/*/Cargo.toml, read with tomllib (a `publish.workspace = true` package
# inherits `[workspace.package] publish`). `crates/doc-examples` (publish =
# false) and every tests/, benches/ and examples/ tree are out of scope. A
# crates/<dir> with no readable Cargo.toml is skipped silently.
#
# Exactly two cfg forms exempt code, and only the item or module block they
# attach to: `#[cfg(test)]` and `#[cfg(all(test, ...))]`. `#[cfg(not(test))]`
# is NOT exempt: it is production code. A file whose own top-level inner
# attribute is `#![cfg(test)]` is wholly test-only.
#
# Out-of-line test modules: `#[cfg(test)] mod name;` resolves to `name.rs` or
# `name/mod.rs` beside the declaring file (the file's own directory for
# lib.rs, main.rs and mod.rs, otherwise the path without `.rs`). Those files
# are test-only, transitively: a test-only file's own `mod child;` children are
# test-only too. So crates/paladin-battalion/src/engine/test_support.rs reports
# nothing. The resolution is by file, not by the inline module nesting, which
# is an over-approximation that can only exempt a file that exists at that
# exact path.
#
# Matching runs on the macro token after line comments, doc comments (so a
# `///` doc example never fires), nested block comments, string, byte-string,
# raw-string and char literals are blanked to spaces with newlines kept (the
# technique of tests/entity_rename_guard.rs). `todo ! (`, `todo![` and
# `unimplemented!{` all match; a lifetime such as `'a` is not read as a char
# literal.
#
# Never vacuous: besides FOUND and OK there are three named failures, in
# precedence order. NO_PACKAGES (no publishable package under the root),
# ZERO_FILES (the publishable packages hold no `.rs` file under `src/`) and
# UNREADABLE (a `.rs` file is not valid UTF-8 or cannot be read: it is named,
# never skipped). The status is the first applicable of NO_PACKAGES,
# ZERO_FILES, UNREADABLE, FOUND, then OK; every named failure and every finding
# is listed before exit, sorted by path and then by line, so two runs on one
# tree print byte-identical output.
#
# There is NO exemption list, no inline allow marker, no environment switch and
# no path filter (D-01). To clear a finding, replace the stub with a typed `Err`
# or delete the dead code.
#
# Offline: no network, no cargo; python3 with tomllib only. Check-only: this
# script never writes to, rewrites or reformats a file it scans, takes no
# argument, and has no write mode, so an interrupted run, or the CI step and
# `make check-gates` running at once, leave the tree unchanged.
#
# Test-only seam: STUB_PANICS_ROOT relocates every input (the manifests and the
# scanned files) to a fixture tree. CI and the Makefile target set no
# environment, so the default is the repository root.
#
# Usage:  ./scripts/check-no-stub-panics.sh
# Exit:   0 when no stub macro is reachable outside test code, 1 otherwise.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${STUB_PANICS_ROOT:-${WORKSPACE_ROOT}}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for Cargo.toml parsing." >&2
    exit 1
fi

echo "🔍 Checking published crates for stub panics outside #[cfg(test)] ..."

REPORT=$(python3 - "${ROOT}" <<'PY'
import glob
import os
import re
import sys
import tomllib

root = os.path.abspath(sys.argv[1])

# `todo!` / `unimplemented!` followed by an opening delimiter. `\b` keeps
# `my_todo!` and `unimplemented_x!` out.
STUB = re.compile(r'\b(todo|unimplemented)\s*!\s*[\(\[{]')
# The two exempt attribute forms: cfg(test) and cfg(all(test, ...)).
CFG_TEST_ATTR = re.compile(
    r'#\s*\[\s*cfg\s*\(\s*(?:test|all\s*\(\s*test\s*,[^\]]*\))\s*\)\s*\]'
)
INNER_CFG_TEST = re.compile(r'#!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]')
ATTR_START = re.compile(r'\s*#\s*\[')
# A module declaration header: optional visibility, `mod name ;`.
MOD_ITEM = re.compile(r'\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(\w+)\s*;')
MOD_DECL = re.compile(r'\bmod\s+(\w+)\s*;')
RAW_OPEN = re.compile(r'(?:br|r)(#*)"')
CHAR_LIT = re.compile(r"'(?:\\.[^']*|[^\\'])'")


def rel(path):
    return os.path.relpath(path, root).replace(os.sep, "/")


def load_manifest(path):
    try:
        with open(path, "rb") as fh:
            return tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError):
        return None


def blank(src):
    """Replace comments and string/char literals with spaces, keeping newlines.

    Handles nested block comments, raw strings (`r#"..."#`, `br"..."`), byte
    strings, and tells a char literal from a lifetime.
    """
    out, n, i = list(src), len(src), 0

    def wipe(a, b):
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            wipe(i, j)
            i = j
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            wipe(i, j)
            i = j
        elif (
            c in "rb"
            and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_"))
            and RAW_OPEN.match(src, i)
        ):
            m = RAW_OPEN.match(src, i)
            end = src.find('"' + m.group(1), m.end())
            j = n if end < 0 else end + 1 + len(m.group(1))
            wipe(i, j)
            i = j
        elif c == '"' or (c == "b" and src.startswith('b"', i)):
            j = i + (2 if c == "b" else 1)
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            wipe(i, j + 1)
            i = j + 1
        elif c == "'":
            m = CHAR_LIT.match(src, i)
            if m:
                wipe(i, m.end())
                i = m.end()
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return "".join(out)


def skip_attrs(b, j):
    """Index just past any run of `#[...]` attributes starting at j."""
    while True:
        m = ATTR_START.match(b, j)
        if not m:
            return j
        depth, k = 0, m.end() - 1
        while k < len(b):
            depth += (b[k] == "[") - (b[k] == "]")
            if depth == 0:
                break
            k += 1
        j = k + 1


def item_end(b, j):
    """Index of the `;` or matching `}` that ends the item whose header starts at j."""
    k = skip_attrs(b, j)
    while k < len(b) and b[k] not in "{;":
        k += 1
    if k >= len(b) or b[k] == ";":
        return k
    depth = 0
    while k < len(b):
        depth += (b[k] == "{") - (b[k] == "}")
        if depth == 0:
            break
        k += 1
    return k


def analyse(blanked):
    """Return (hits, test_mods); hits is None when the whole file is test-only.

    hits: (line, macro) of each stub macro outside an exempt item.
    test_mods: names of out-of-line `#[cfg(test)] mod name;` declarations.
    """
    for m in INNER_CFG_TEST.finditer(blanked):
        before = blanked[: m.start()]
        if before.count("{") == before.count("}"):
            return None, []  # a top-level #![cfg(test)]: the whole file is test-only
    spans, test_mods = [], []
    for m in CFG_TEST_ATTR.finditer(blanked):
        end = item_end(blanked, m.end())
        spans.append((m.start(), end))
        decl = MOD_ITEM.match(blanked, skip_attrs(blanked, m.end()))
        if decl:
            test_mods.append(decl.group(1))
    hits = [
        (blanked.count("\n", 0, m.start()) + 1, m.group(1))
        for m in STUB.finditer(blanked)
        if not any(a <= m.start() <= z for a, z in spans)
    ]
    return hits, test_mods


def module_dir(path):
    base = os.path.basename(path)
    if base in ("lib.rs", "main.rs", "mod.rs"):
        return os.path.dirname(path)
    return path[: -len(".rs")]


# --- The published packages. ---
root_manifest = load_manifest(os.path.join(root, "Cargo.toml"))
ws_publish = None
if root_manifest is not None:
    ws_publish = root_manifest.get("workspace", {}).get("package", {}).get("publish")

manifests = [os.path.join(root, "Cargo.toml")]
manifests += sorted(glob.glob(os.path.join(root, "crates", "*", "Cargo.toml")))
packages = []
for manifest_path in manifests:
    manifest = (
        root_manifest if manifest_path == os.path.join(root, "Cargo.toml")
        else load_manifest(manifest_path)
    )
    if manifest is None:
        continue
    package = manifest.get("package")
    if not isinstance(package, dict):
        continue
    publish = package.get("publish", True)
    if isinstance(publish, dict) and publish.get("workspace") is True:
        publish = True if ws_publish is None else ws_publish
    if publish is False:
        continue
    packages.append(os.path.dirname(manifest_path))

# --- The scan set: sorted, deterministic. ---
files = sorted({
    os.path.join(dirpath, filename)
    for pkg in packages
    for dirpath, _dirnames, filenames in os.walk(os.path.join(pkg, "src"))
    for filename in filenames
    if filename.endswith(".rs")
})

blanked = {}
unreadable = []
for path in files:
    try:
        with open(path, "r", encoding="utf-8") as fh:
            blanked[path] = blank(fh.read())
    except (UnicodeDecodeError, OSError):
        unreadable.append(path)

parsed = {path: analyse(text) for path, text in blanked.items()}
test_only = {path for path, (hits, _mods) in parsed.items() if hits is None}


def resolve_children(path, names):
    """Mark every `name.rs` / `name/mod.rs` declared beside `path` as test-only."""
    added = []
    for name in names:
        for cand in (
            os.path.join(module_dir(path), name + ".rs"),
            os.path.join(module_dir(path), name, "mod.rs"),
        ):
            if cand in parsed and cand not in test_only:
                test_only.add(cand)
                added.append(cand)
    return added


# `#[cfg(test)] mod x;` in any file makes x test-only; then, transitively, every
# `mod y;` declared inside a test-only file.
queue = []
for path, (_hits, mods) in parsed.items():
    queue += resolve_children(path, mods)
queue += [p for p in test_only]
seen = set()
while queue:
    path = queue.pop()
    if path in seen:
        continue
    seen.add(path)
    queue += resolve_children(path, MOD_DECL.findall(blanked[path]))

findings = sorted(
    (rel(path), line, macro)
    for path in files
    if path in parsed and path not in test_only
    for line, macro in parsed[path][0]
)

# --- Named failures, in status-line priority order. ---
named = []  # (token, detail line)
if not packages:
    named.append(("NO_PACKAGES",
                  f"NO_PACKAGES: no publishable package (a [package] table whose "
                  f"publish is not false) under {root}; a broken tree is a named "
                  "failure, never a pass."))
elif not files:
    named.append(("ZERO_FILES",
                  f"ZERO_FILES: scanned 0 .rs file(s) under src/ of "
                  f"{len(packages)} publishable package(s); a broken scan set is a "
                  "named failure, never a pass."))
for path in unreadable:
    named.append(("UNREADABLE",
                  f"UNREADABLE: {rel(path)} is not valid UTF-8 or cannot be read; "
                  "it is never skipped."))
named.sort(key=lambda item: ("NO_PACKAGES", "ZERO_FILES", "UNREADABLE").index(item[0]))

summary = (f"scanned {len(files)} file(s) ({len(test_only)} test-only), "
           f"{len(findings)} finding(s)")

if named or findings:
    print(named[0][0] if named else "FOUND")
    for _, detail in named:
        print(detail)
    for path, line, macro in findings:
        print(f"{path}:{line}: {macro}! outside #[cfg(test)] in a published crate")
    if files:
        print(summary)
    sys.exit(0)

print("OK")
print(summary)
sys.exit(0)
PY
)

STATUS_LINE=$(head -n1 <<<"${REPORT}")
DETAIL=$(tail -n +2 <<<"${REPORT}")

if [ "${STATUS_LINE}" = "OK" ]; then
    echo "✅ OK: ${DETAIL}"
    exit 0
else
    echo "❌ Stub-panic check failed (${STATUS_LINE})"
    echo ""
    echo "${DETAIL}"
    echo ""
    echo "If this failure is unexpected:"
    echo "  1. Replace each reported stub with a typed Err (or delete the dead code);"
    echo "     a stub macro outside #[cfg(test)] in a published crate panics a caller."
    echo "  2. This gate has no exemption list, no inline marker and no environment"
    echo "     switch (D-01); do not add one to silence a finding."
    exit 1
fi
