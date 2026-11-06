#!/usr/bin/env bash
# 58-check-register.sh -- register-completeness verifier for 58-AUDIT-DISPOSITIONS.md (plan 58-05, SHIP-10).
#
# Read-only. Reads the two tables of the disposition register:
#   ## Carried items                       rows R-nn
#   ## v0.11.0 carry-over reconciliation   rows V-nn
# each with seven columns:
#   | ID | Item | Source | Re-proof | Verdict | Pointer / reason / owner + date | Todo |
# Cells never contain a literal pipe character (the parser splits on it).
#
# Rules (a failure is named, printed, and counted; the verdict never depends on row order):
#   BAD_COLUMNS      a row does not have exactly seven cells
#   WRONG_SECTION    an R-nn row under the V-nn table or the reverse
#   DUP_ID           an id appears twice (one row, one verdict)
#   NO_REPROOF       the Re-proof cell is empty or a placeholder
#   BAD_VERDICT      the verdict is not exactly one of fixed, waived, re-homed
#   FIXED_NO_POINTER a `fixed` row without a resolvable pointer: a 7-40 hex commit id that
#                    `git cat-file -e <id>^{commit}` resolves, a 58-CI-EVIDENCE.md reference
#                    (F-58-nn, SHA-<label> or a section name of that file), or a requirement
#                    id together with a NN-VERIFICATION.md that exists
#   WAIVED_NO_REASON a `waived` row whose reason is under 20 characters or a placeholder
#   REHOMED_OWNER    a `re-homed` row whose owner is neither `repo maintainer` nor `v0.12.0 Phase 1`
#   REHOMED_DATE     a `re-homed` row without a YYYY-MM-DD date (in --draft: or the marker
#                    `pending 58-05 decision`)
#   REHOMED_TODO     a `re-homed` row whose Todo path does not exist under .planning/todos/pending/
#                    or completed/, whose frontmatter has no `owner:`, or (with a real date) whose
#                    `recheck_by:` differs from the row's date (in --draft the Todo may read
#                    `to be written in 58-06`)
#
# `re-homed` pointer cell form:  owner: <owner>; date: <YYYY-MM-DD | pending 58-05 decision>; <text>
#
# Usage:  58-check-register.sh [--draft] [register-file]
# Exit:   0 when there are no failures, 1 otherwise.
# Last line:
#   register: checked <N> row(s) (R: <a>, V: <b>); fixed <F>, waived <W>, re-homed <H>; <K> failure(s)

set -euo pipefail

DRAFT=0
FILE=""
for arg in "$@"; do
    case "${arg}" in
        --draft) DRAFT=1 ;;
        -h | --help)
            sed -n '2,36p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        -*)
            echo "ERROR: unknown option ${arg}" >&2
            exit 2
            ;;
        *) FILE="${arg}" ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required." >&2
    exit 2
fi

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
FILE="${FILE:-${ROOT}/.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md}"
EVIDENCE="${ROOT}/.planning/phases/58-crate-release/58-CI-EVIDENCE.md"

if [ ! -f "${FILE}" ]; then
    echo "ERROR: register not found: ${FILE}" >&2
    exit 2
fi

python3 - "${FILE}" "${ROOT}" "${EVIDENCE}" "${DRAFT}" <<'PY'
import glob
import os
import re
import subprocess
import sys

register, root, evidence_path, draft_flag = sys.argv[1:5]
draft = draft_flag == "1"

VERDICTS = ("fixed", "waived", "re-homed")
OWNERS = ("repo maintainer", "v0.12.0 Phase 1")
PLACEHOLDERS = {"tbd", "todo", "n/a", "na", "none", "pending", "placeholder", "...", "-", "tbc", "to be written"}
PENDING_DATE = "pending 58-05 decision"
PENDING_TODO = "to be written in 58-06"

lines = open(register, encoding="utf-8").read().split("\n")
evidence = ""
if os.path.isfile(evidence_path):
    evidence = open(evidence_path, encoding="utf-8").read()
evidence_headings = [
    m.group(1).strip() for m in re.finditer(r"^#{2,4}\s+(.+?)\s*$", evidence, re.M)
]

sections = {"R": "## Carried items", "V": "## v0.11.0 carry-over reconciliation"}
rows = []  # (expected_kind, lineno, cells)
current = None
for lineno, line in enumerate(lines, 1):
    if line.startswith("## "):
        current = None
        for kind, heading in sections.items():
            if line.strip() == heading:
                current = kind
        continue
    if current and re.match(r"^\|\s*[RV]-\d+\s*\|", line):
        body = line.strip()
        body = body[1:-1] if body.endswith("|") else body[1:]
        cells = [c.strip() for c in body.split("|")]
        rows.append((current, lineno, cells))

failures = []


def fail(row_id, code, text):
    failures.append((row_id, code, text))


def unwrap(cell):
    return cell.strip().strip("`").strip()


def commit_resolves(token):
    try:
        subprocess.run(
            ["git", "-C", root, "cat-file", "-e", token + "^{commit}"],
            check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def fixed_pointer_ok(pointer):
    for token in re.findall(r"\b[0-9a-f]{7,40}\b", pointer):
        if re.search(r"[0-9]", token) and re.search(r"[a-f]", token) and commit_resolves(token):
            return True
    for m in re.finditer(r"\bF-58-\d\d\b", pointer):
        if re.search(r"\b" + re.escape(m.group(0)) + r"\b", evidence):
            return True
    for m in re.finditer(r"\bSHA-[A-Z0-9]+\b", pointer):
        if re.search(r"^- " + re.escape(m.group(0)) + r": [0-9a-f]{40}\s*$", evidence, re.M):
            return True
    if "58-CI-EVIDENCE" in pointer:
        for heading in evidence_headings:
            if len(heading) >= 6 and heading in pointer:
                return True
    reqs = re.findall(r"\b[A-Z]{3,6}-\d{2}\b", pointer)
    for v in re.findall(r"\b(\d{2}(?:\.\d)?)-VERIFICATION\.md\b", pointer):
        if reqs and glob.glob(os.path.join(root, ".planning", "phases", v + "-*", v + "-VERIFICATION.md")):
            return True
    return False


def frontmatter(path):
    text = open(path, encoding="utf-8").read()
    m = re.match(r"^---\n(.*?)\n---", text, re.S)
    return m.group(1) if m else ""


def todo_path(cell):
    for m in re.finditer(r"\.planning/todos/(?:pending|completed)/[^\s`;,)]+\.md", cell):
        return m.group(0)
    return None


seen = {}
counts = {"R": 0, "V": 0}
verdict_counts = {v: 0 for v in VERDICTS}

for kind, lineno, cells in rows:
    row_id = cells[0].strip() if cells else "?"
    if len(cells) != 7:
        fail(row_id, "BAD_COLUMNS", "line %d has %d cells, expected 7" % (lineno, len(cells)))
        continue
    rid, item, source, reproof, verdict, pointer, todo = cells
    rid = rid.strip()
    if not rid.startswith(kind + "-"):
        fail(rid, "WRONG_SECTION", "line %d: %s row under the %s table" % (lineno, rid, sections[kind]))
    if rid in seen:
        fail(rid, "DUP_ID", "lines %d and %d" % (seen[rid], lineno))
    seen[rid] = lineno
    counts[rid[0] if rid[0] in counts else kind] += 1

    if len(reproof.strip()) < 10 or unwrap(reproof).lower() in PLACEHOLDERS:
        fail(rid, "NO_REPROOF", "line %d: empty or placeholder Re-proof" % lineno)

    v = unwrap(verdict)
    if v not in VERDICTS:
        fail(rid, "BAD_VERDICT", "line %d: verdict %r is not one of %s" % (lineno, verdict, ", ".join(VERDICTS)))
        continue
    verdict_counts[v] += 1

    if v == "fixed":
        if not fixed_pointer_ok(pointer):
            fail(rid, "FIXED_NO_POINTER", "line %d: no resolvable commit, CI-evidence reference or requirement + VERIFICATION" % lineno)
    elif v == "waived":
        reason = pointer.strip()
        if len(reason) < 20 or unwrap(reason).lower() in PLACEHOLDERS:
            fail(rid, "WAIVED_NO_REASON", "line %d: reason under 20 characters or a placeholder" % lineno)
    else:  # re-homed
        om = re.search(r"owner:\s*(repo maintainer|v0\.12\.0 Phase 1)\b", pointer)
        if not om:
            fail(rid, "REHOMED_OWNER", "line %d: pointer cell needs `owner: repo maintainer` or `owner: v0.12.0 Phase 1`" % lineno)
        dm = re.search(r"date:\s*(\d{4}-\d{2}-\d{2}|" + re.escape(PENDING_DATE) + r")", pointer)
        date = dm.group(1) if dm else None
        if date is None or (date == PENDING_DATE and not draft):
            fail(rid, "REHOMED_DATE", "line %d: needs `date: YYYY-MM-DD`%s" % (lineno, " (or the pending marker in --draft)" if draft else ""))
        tpath = todo_path(todo)
        if tpath is None:
            if not (draft and PENDING_TODO in todo):
                fail(rid, "REHOMED_TODO", "line %d: Todo cell has no .planning/todos path%s" % (lineno, " (--draft allows `%s`)" % PENDING_TODO if draft else ""))
        else:
            full = os.path.join(root, tpath)
            if not os.path.isfile(full):
                fail(rid, "REHOMED_TODO", "line %d: %s does not exist" % (lineno, tpath))
            else:
                fm = frontmatter(full)
                if not re.search(r"^owner:\s*\S", fm, re.M):
                    fail(rid, "REHOMED_TODO", "line %d: %s has no owner: in its frontmatter" % (lineno, tpath))
                rb = re.search(r"^recheck_by:\s*(\S+)", fm, re.M)
                if date and date != PENDING_DATE:
                    if not rb:
                        if not draft:
                            fail(rid, "REHOMED_TODO", "line %d: %s has no recheck_by:" % (lineno, tpath))
                    elif rb.group(1).strip("\"'") != date:
                        fail(rid, "REHOMED_TODO", "line %d: %s recheck_by %s differs from the row date %s" % (lineno, tpath, rb.group(1), date))

for row_id, code, text in failures:
    print("FAIL %s %s: %s" % (code, row_id, text))

total = counts["R"] + counts["V"]
print(
    "register: checked %d row(s) (R: %d, V: %d); fixed %d, waived %d, re-homed %d; %d failure(s)"
    % (total, counts["R"], counts["V"], verdict_counts["fixed"], verdict_counts["waived"], verdict_counts["re-homed"], len(failures))
)
sys.exit(1 if failures else 0)
PY
