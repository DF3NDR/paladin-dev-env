#!/usr/bin/env bash
# check-findings-log.sh
#
# Offline, check-only gate that turns DOGF-01 ("no finding reads `open`") into
# a command instead of a read (Phase 56, D-01, D-03). It parses every
# `### DF-nn` entry of the v0.11.1 dogfood findings log, takes each entry's
# LAST disposition bullet, and fails naming every entry that is open, missing,
# malformed or mis-ordered.
#
# Log read (default): .project/v0.11.1/02-dogfood-findings-log.md
#
# Entry: a heading `### DF-<digits>` outside a code fence. Its body runs to the
# next heading of level 1-3 outside a fence, or to the end of the file, so a
# `####` sub-heading does not end an entry. Ids compare as integers (`DF-7`
# equals `DF-07`).
#
# Disposition (D-01): the LAST bullet of the body matching
#     - **Disposition:** <value>
#     - **Disposition (<date>):** <value>
# A value may continue on following lines indented by two or more spaces that
# do not begin a new `- **` bullet. The last bullet wins in both directions: a
# dated closing bullet appended after the pre-seeded `open → recommend **in
# v0.11.1**` bullet makes the entry pass, and a later dated `open` bullet
# re-opens it. The log is append-only (D-01); this gate never edits it.
#
# Accepted final forms (after stripping leading `*`, backticks and spaces):
#     in v0.11.1 — <pointer>         a pointer (commit, phase, requirement, test)
#                                    must follow; a bare `in v0.11.1` is NO_POINTER
#     out (<reason>)                 a non-blank reason in parentheses
#     duplicate of DF-nn             nn must be another entry present in the log
# Anything starting with `open` is OPEN; anything else is UNRECOGNISED.
#
# Fence rule: a line whose stripped form starts with three backticks toggles a
# fence, and everything inside one is ignored. That is how the entry template at
# the top of the log (a literal `### DF-nn` heading and an `open | in v0.11.1 |
# ...` bullet) is skipped. A fence opened and never closed is UNCLOSED_FENCE, so
# an unterminated code block cannot hide the entries after it.
#
# Never vacuous: besides OK and BAD there are named failures. UNREADABLE (the
# file is missing, unreadable or not UTF-8), UNCLOSED_FENCE and ZERO_ENTRIES (no
# entry heading outside a fence). The status line is the first applicable of
# them in that order, then BAD (any per-entry finding), then OK. Per-entry
# findings are OPEN, MISSING, NO_POINTER, BAD_OUT, BAD_DUP_TARGET, UNRECOGNISED,
# DUPLICATE_ID and NOT_INCREASING; every finding and every named failure is
# listed before exit. The last detail line is always `parsed <N> entr(y|ies)
# from <path>`, on OK and on BAD alike.
#
# Milestone token: the accepted keyword is the literal `in v0.11.1`. If a D-00b
# forced break renames the milestone to `v0.12.0`, edit that one token (the
# IN_RE pattern below and this header) in the same commit that records the
# break.
#
# Offline: no network, no cargo; python3 stdlib only. Check-only: this script
# never writes to the log, has no option parsing, no write mode and no
# exemption list. The log text is only ever matched by anchored regexes; it is
# never evaluated or expanded.
#
# Test-only seam: FINDINGS_LOG_PATH relocates the single input file to a fixture
# log (a file path, not a directory). CI and the Makefile target set no
# environment, so the default is the real log.
#
# Run at phase verification and by Phase 58 SHIP-10, never by `make check-gates`
# or any CI workflow (D-03): the real log legitimately holds `open` entries
# mid-phase, and a gate would turn every new finding into a red build for
# unrelated PRs.
#
# Usage:  ./scripts/check-findings-log.sh      (or: make check-findings-log)
# Exit:   0 when every entry's last disposition is a valid final form, 1 otherwise.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${FINDINGS_LOG_PATH:-${WORKSPACE_ROOT}/.project/v0.11.1/02-dogfood-findings-log.md}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for findings-log parsing." >&2
    exit 1
fi

echo "🔍 Checking the findings log for entries without a final disposition ..."

REPORT=$(python3 - "${LOG}" "${WORKSPACE_ROOT}" <<'PY'
import os
import re
import sys

log_path = sys.argv[1]
workspace = sys.argv[2]

FENCE = "```"
HEAD_RE = re.compile(r"^### (DF-(\d+))\b")
ANY_HEAD_RE = re.compile(r"^#{1,3} ")
BULLET_RE = re.compile(r"^- \*\*Disposition(?: \(([^)]*)\))?:\*\*\s*(.*)$")
OPEN_RE = re.compile(r"^open\b")
IN_RE = re.compile(r"^in v0\.11\.1\b(.*)$", re.DOTALL)
POINTER_STRIP = " \t*`—–-:;,"


def display(path):
    """Print the path relative to the workspace root when it lies under it."""
    absolute = os.path.abspath(path)
    try:
        relative = os.path.relpath(absolute, workspace)
    except ValueError:
        return path
    if relative == ".." or relative.startswith(".." + os.sep):
        return path
    return relative.replace(os.sep, "/")


shown = display(log_path)


def plural(count):
    return "1 entry" if count == 1 else f"{count} entries"


# --- Read the log. ---
try:
    with open(log_path, "r", encoding="utf-8") as fh:
        text = fh.read()
except (OSError, UnicodeDecodeError) as exc:
    print("UNREADABLE")
    print(f"UNREADABLE: cannot read {shown} as UTF-8 ({type(exc).__name__}); an "
          "unreadable log is a named failure, never a pass.")
    print(f"parsed 0 entries from {shown}")
    sys.exit(0)

# --- Pass 1: entries and their disposition bullets. ---
entries = []  # dicts: id, line, bullets [(line, value)]
current = None
in_fence = False
continuing = False  # the previous line belonged to the last disposition bullet

for lineno, line in enumerate(text.splitlines(), start=1):
    if line.strip().startswith(FENCE):
        in_fence = not in_fence
        continuing = False
        continue
    if in_fence:
        continue
    if ANY_HEAD_RE.match(line):
        continuing = False
        head = HEAD_RE.match(line)
        if head:
            current = {"id": head.group(1), "line": lineno, "bullets": []}
            entries.append(current)
        else:
            current = None
        continue
    if current is None:
        continue
    bullet = BULLET_RE.match(line)
    if bullet:
        current["bullets"].append([lineno, bullet.group(2).strip()])
        continuing = True
        continue
    if (continuing and line[:2] == "  " and line.strip()
            and not line.strip().startswith("- **")):
        current["bullets"][-1][1] = (current["bullets"][-1][1] + " " + line.strip()).strip()
        continue
    continuing = False

# --- Pass 2: classify each entry's last disposition bullet. ---
findings = []  # (line, token, entry id, hint)
for entry in entries:
    if not entry["bullets"]:
        findings.append((entry["line"], "MISSING", entry["id"],
                         "no '- **Disposition:**' bullet in this entry"))
        continue
    line, value = entry["bullets"][-1]
    value = value.lstrip("*` \t")
    if OPEN_RE.match(value):
        findings.append((line, "OPEN", entry["id"],
                         "the last Disposition bullet reads 'open'"))
        continue
    pointed = IN_RE.match(value)
    if pointed:
        if not pointed.group(1).strip(POINTER_STRIP):
            findings.append((line, "NO_POINTER", entry["id"],
                             "'in v0.11.1' must name its evidence"))
        continue
    findings.append((line, "UNRECOGNISED", entry["id"],
                     "not 'in v0.11.1 — <pointer>', 'out (<reason>)' or 'duplicate of DF-nn'"))

findings.sort(key=lambda f: (f[0], f[1]))

# --- Named failures, in status-line priority order. ---
named = []  # (token, detail line)
if not entries:
    named.append(("ZERO_ENTRIES",
                  f"ZERO_ENTRIES: no '### DF-nn' entry found outside a code fence in "
                  f"{shown}; parsing nothing is a named failure, never a pass."))

if named or findings:
    print(named[0][0] if named else "BAD")
    for _, detail in named:
        print(detail)
    for line, token, entry_id, hint in findings:
        print(f"{shown}:{line}: {entry_id}: {token} ({hint})")
else:
    print("OK")
print(f"parsed {plural(len(entries))} from {shown}")
sys.exit(0)
PY
)

STATUS_LINE=$(head -n1 <<<"${REPORT}")
DETAIL=$(tail -n +2 <<<"${REPORT}")

if [ "${STATUS_LINE}" = "OK" ]; then
    echo "✅ OK: every entry's last Disposition is a final form (${DETAIL})"
    exit 0
else
    echo "❌ Findings-log check failed (${STATUS_LINE})"
    echo ""
    echo "If this failure is unexpected:"
    echo "  1. Append a dated '- **Disposition (YYYY-MM-DD):**' bullet under the entry"
    echo "     (in v0.11.1 — <pointer>, out (<reason>) or duplicate of DF-nn). Never"
    echo "     rewrite an earlier line; the log is append-only (D-01)."
    echo "  2. This gate never edits the log. It is run at phase verification and by"
    echo "     SHIP-10, not by check-gates or CI."
    echo ""
    echo "${DETAIL}"
    exit 1
fi
