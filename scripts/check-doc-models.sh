#!/usr/bin/env bash
# check-doc-models.sh
#
# Offline, read-only gate that keeps deprecated model ids out of the material a
# new user copies (Phase 58, SHIP-10, DF-18). The maintainer's keyed first-run
# walk on the published 0.11.0 crates found the Quickstart's `.model("gpt-4")`
# failing against a real OpenAI key with `model_not_found` (the id is retired),
# and the same id sat at about 110 more copyable sites. A one-off sweep goes
# stale, so this gate turns a returning deprecated id into a red
# `make check-gates` and a red CI job.
#
# Deprecated ids live in the single DEPRECATED_IDS array below, so a later
# deprecation is a one-line change. Only ids measured dead belong there: the
# serving status of the other ids in the docs (gpt-4o, gpt-3.5-turbo, gpt-4-turbo)
# was never measured, and a registry-membership check for them is deliberately
# not built here.
#
# Scanned set (each file read as UTF-8; a file that is not valid UTF-8 is
# skipped, never a crash): README.md, every *.md under docs/src, and every file
# under examples/ with the suffix .rs, .md, .yaml, .yml, .toml, .json, .sh or .py.
#
# A deprecated id is reported only in a copyable form:
#   - the exact id between matching " ... ", ' ... ' or ` ... ` delimiters;
#   - a YAML line whose key contains `model` and whose value is exactly the id
#     (optionally followed by a `#` comment);
#   - `--model` followed by a space or `=` and exactly the id.
# Ids that merely share the prefix (gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4-32k,
# gpt-4-invalid, mock-gpt-4) and illustrative output lines such as
# `Models: gpt-4, gpt-3.5-turbo` are not reported.
#
# Allowlist (default scripts/check-doc-models.allowlist): one entry per line,
# `<path>|<line substring>|<reason>`, path relative to the root. A hit is
# exempt only when it sits in <path> on a line containing the substring. Every
# entry needs a reason (ALLOW_NO_REASON otherwise), a malformed line is
# ALLOW_MALFORMED, and an entry that exempts no hit is STALE_ALLOW: an
# allowlist that outlives its line would hide the next real site.
#
# Output: one `<file>:<line>: deprecated model id '<id>' (<form>)` line per
# hit, then a last line:
#   OK: <N> file(s) scanned, 0 deprecated model literal(s) (allowlisted: <K>)
#   FAIL: <M> deprecated model literal(s) in <F> file(s)[; <S> STALE_ALLOW ...]
# Zero files scanned is the named failure ZERO_FILES, never a silent pass.
#
# Usage:  ./scripts/check-doc-models.sh [--root <dir>] [--allowlist <file>]
# Exit:   0 clean, 1 findings or a bad/stale allowlist entry, 2 usage error or
#         zero files scanned.

set -euo pipefail

# The one place a deprecated id is named.
DEPRECATED_IDS=("gpt-4")

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${WORKSPACE_ROOT}"
ALLOWLIST="${WORKSPACE_ROOT}/scripts/check-doc-models.allowlist"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --root)
            [ "$#" -ge 2 ] || { echo "ERROR: --root needs a directory" >&2; exit 2; }
            ROOT="$2"
            shift 2
            ;;
        --allowlist)
            [ "$#" -ge 2 ] || { echo "ERROR: --allowlist needs a file" >&2; exit 2; }
            ALLOWLIST="$2"
            shift 2
            ;;
        *)
            echo "ERROR: unknown argument: $1" >&2
            echo "Usage: $0 [--root <dir>] [--allowlist <file>]" >&2
            exit 2
            ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for the scanner." >&2
    exit 2
fi
if [ ! -d "${ROOT}" ]; then
    echo "ERROR: root is not a directory: ${ROOT}" >&2
    exit 2
fi
if [ ! -r "${ALLOWLIST}" ]; then
    echo "ERROR: allowlist is not readable: ${ALLOWLIST}" >&2
    exit 2
fi

exec python3 - "${ROOT}" "${ALLOWLIST}" "${DEPRECATED_IDS[@]}" <<'PY'
import os
import re
import sys

root, allowlist_path, *deprecated_ids = sys.argv[1:]

EXAMPLE_SUFFIXES = (".rs", ".md", ".yaml", ".yml", ".toml", ".json", ".sh", ".py")
TOKEN_TAIL = r"(?![A-Za-z0-9._-])"


def rel(path):
    return os.path.relpath(path, root).replace(os.sep, "/")


def discover():
    found = []
    readme = os.path.join(root, "README.md")
    if os.path.isfile(readme):
        found.append(readme)
    for top, suffixes in (("docs/src", (".md",)), ("examples", EXAMPLE_SUFFIXES)):
        base = os.path.join(root, top)
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames.sort()
            for name in sorted(filenames):
                if name.endswith(suffixes):
                    found.append(os.path.join(dirpath, name))
    return found


def patterns(model_id):
    esc = re.escape(model_id)
    return [
        ("double-quoted", re.compile('"' + esc + '"')),
        ("single-quoted", re.compile("'" + esc + "'")),
        ("backticked", re.compile("`" + esc + "`")),
        (
            "yaml value",
            re.compile(
                r"^\s*(?:-\s+)?[A-Za-z0-9_.-]*model[A-Za-z0-9_.-]*\s*:\s*"
                + esc
                + r"\s*(?:#.*)?$"
            ),
        ),
        ("--model flag", re.compile(r"--model[ =]" + esc + TOKEN_TAIL)),
    ]


PATTERNS = {model_id: patterns(model_id) for model_id in deprecated_ids}


def load_allowlist():
    entries = []
    problems = []
    with open(allowlist_path, encoding="utf-8") as fh:
        for number, raw in enumerate(fh, start=1):
            line = raw.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            parts = line.split("|", 2)
            if len(parts) != 3 or not parts[0].strip() or not parts[1]:
                problems.append(
                    f"ALLOW_MALFORMED: {allowlist_path}:{number}: "
                    "want '<path>|<line substring>|<reason>'"
                )
                continue
            if not parts[2].strip():
                problems.append(
                    f"ALLOW_NO_REASON: {allowlist_path}:{number}: "
                    f"entry '{parts[0].strip()}|{parts[1]}' carries no reason"
                )
                continue
            entries.append(
                {"path": parts[0].strip(), "needle": parts[1], "line": number, "used": 0}
            )
    return entries, problems


files = discover()
entries, problems = load_allowlist()

if not files:
    print(f"ZERO_FILES: no file to scan under {root} (README.md, docs/src, examples)")
    print("FAIL: 0 file(s) scanned (ZERO_FILES); a gate that reads nothing proves nothing")
    sys.exit(2)

hits = []
allowlisted = 0
for path in files:
    try:
        with open(path, encoding="utf-8") as fh:
            lines = fh.read().splitlines()
    except (OSError, UnicodeDecodeError):
        continue
    name = rel(path)
    for number, text in enumerate(lines, start=1):
        for model_id in deprecated_ids:
            form = next((f for f, rx in PATTERNS[model_id] if rx.search(text)), None)
            if form is None:
                continue
            exempt = [e for e in entries if e["path"] == name and e["needle"] in text]
            if exempt:
                for entry in exempt:
                    entry["used"] += 1
                allowlisted += 1
                continue
            hits.append((name, number, model_id, form))

stale = [e for e in entries if e["used"] == 0]
for hit in hits:
    print(f"{hit[0]}:{hit[1]}: deprecated model id '{hit[2]}' ({hit[3]})")
for problem in problems:
    print(problem)
for entry in stale:
    print(
        f"STALE_ALLOW: {allowlist_path}:{entry['line']}: entry "
        f"'{entry['path']}|{entry['needle']}' exempts no deprecated-id line"
    )

if hits or problems or stale:
    extra = ""
    if stale:
        extra += f"; {len(stale)} STALE_ALLOW entr{'y' if len(stale) == 1 else 'ies'}"
    if problems:
        extra += f"; {len(problems)} bad allowlist line(s)"
    print(
        f"FAIL: {len(hits)} deprecated model literal(s) in "
        f"{len({h[0] for h in hits})} file(s){extra}"
    )
    sys.exit(1)

print(
    f"OK: {len(files)} file(s) scanned, 0 deprecated model literal(s) "
    f"(allowlisted: {allowlisted})"
)
PY
