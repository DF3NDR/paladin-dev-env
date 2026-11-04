#!/usr/bin/env bash
# check-review-dispositions.sh
#
# Offline, check-only gate for REG-05 (Phase 57, D-10..D-13): every warning (and
# critical) finding in the archived v0.11.0 code-review reports must carry a
# checked disposition in the Phase 57 register. The register is the file the
# milestone audit reads by name, so this gate fails on every shape of "looks
# dispositioned but is not": a placeholder, a duplicate row, a dropped heading,
# a missing archive file, an unclosed code fence (Phase 56 REVIEW WR-01/WR-02).
#
# Expected ids are derived from the ARCHIVE, never from the register:
#   <archive>/*/<NN>-REVIEW.md   (NN from the file name, e.g. 40, 42, 50.1)
#   every `### WR-<n>:` and `### CR-<n>:` heading outside a code fence,
#   plus the fixed extras {40: IN-01, IN-04} (D-12).
# The checker never edits the archive or the register.
#
# Register grammar (rows are read only inside the `## Dispositions` section):
#   | Phase | Id | Title | Disposition | Evidence | Owner (if waived) |
# A data row is a table line whose first cell is a phase number (`40`, `50.1`).
# Header and separator rows are skipped. Disposition is one of
#   closed | waived | open
# and the document carries one line, outside any fence:
#   **Totals:** closed: <n>, waived: <n>, open: <n>
#
# Per-row rules (a row that is out of scope under --phases is ignored):
#   closed  Evidence must hold a pointer: a 7-40 hex commit id containing a
#           digit, a snake_case test name with two or more underscores, a
#           requirement id such as TENANT-04 (WR-/CR-/IN- ids do not count:
#           they point back at a finding, not at evidence), a repo path with
#           a `/` (and a file extension or two slashes), or `WINDOWS row <n>`.
#   waived  Evidence is the reason; the Owner cell must be filled.
#   open    A failure (OPEN) in strict mode. With --pending-ok an open row is
#           accepted only when its Evidence starts with `LIVE ` and names a
#           `file:line` (PD-08-2): a finding still live in the tree, visible
#           for triage. 57-11 runs the checker strict.
#   Every row: Evidence must not be empty or a placeholder (TBD, none, todo,
#           n/a, -, see above, ...): NO_EVIDENCE.
#
# Failure classes (each is printed as `FAIL <CLASS> <phase> <id>: <detail>`):
#   MISSING            an archived id has no register row
#   UNKNOWN            a register row names an id not in the archive
#   DUPLICATE          two rows for the same (phase, id)
#   BAD_DISPOSITION    Disposition is not closed | waived | open
#   OPEN               an open row (strict), or an open row without a LIVE pointer
#   NO_EVIDENCE        Evidence empty or a placeholder
#   CLOSED_NO_POINTER  a closed row whose Evidence holds no checkable pointer
#   WAIVED_NO_OWNER    a waived row with no owner
#   TOTALS             the Totals line is absent or disagrees with the rows
#   NO_POINTER_SECTION an in-scope REVIEW.md lacks `## Dispositions (Phase 57)`
#   ZERO_FILES         no in-scope REVIEW.md was found (or a listed phase has none)
#   ZERO_ROWS          the register holds no data row at all
#   UNREADABLE         the register or an archive file is missing or not UTF-8
#   UNCLOSED_FENCE     a code fence opened and never closed (register or archive)
#   MALFORMED_ROW      a phase-numbered table row that does not have six cells
#
# Informational (not a failure): `NOTE MISMATCH` when a review's frontmatter
# critical/warning counts differ from its heading counts (42's iteration-2 file
# legitimately differs).
#
# The last line is always:
#   checked <N> finding(s) across <M> review file(s); <F> failure(s)
#
# Usage:  scripts/check-review-dispositions.sh [--register PATH] [--archive DIR]
#                                              [--phases 40,42,50.1] [--pending-ok]
# Exit:   0 when no failure, 1 on any failure, 2 on a usage error.
#
# Offline: no network, no cargo; python3 stdlib only (3.8-compatible: no match,
# no tomllib). Run at phase verification, never by `make check-gates` or CI
# (Phase 56 D-03 precedent): mid-phase the register legitimately holds rows
# that are not final.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REGISTER="${WORKSPACE_ROOT}/.planning/phases/57-register-hygiene-close-out/57-REVIEW-DISPOSITIONS.md"
ARCHIVE="${WORKSPACE_ROOT}/.planning/milestones/v0.11.0-phases"
PHASES=""
PENDING_OK="0"

usage() {
    echo "Usage: $0 [--register PATH] [--archive DIR] [--phases 40,42,50.1] [--pending-ok]" >&2
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --register)
            [ "$#" -ge 2 ] || { usage; exit 2; }
            REGISTER="$2"
            shift 2
            ;;
        --archive)
            [ "$#" -ge 2 ] || { usage; exit 2; }
            ARCHIVE="$2"
            shift 2
            ;;
        --phases)
            [ "$#" -ge 2 ] || { usage; exit 2; }
            PHASES="$2"
            shift 2
            ;;
        --pending-ok)
            PENDING_OK="1"
            shift
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "ERROR: unknown argument: $1" >&2
            usage
            exit 2
            ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for review-disposition parsing." >&2
    exit 2
fi

python3 - "${REGISTER}" "${ARCHIVE}" "${PHASES}" "${PENDING_OK}" "${WORKSPACE_ROOT}" <<'PY'
import os
import re
import sys

register_path, archive_dir, phases_arg, pending_ok_arg, workspace = sys.argv[1:6]
pending_ok = pending_ok_arg == "1"

FENCE = "```"
POINTER_HEADING = "## Dispositions (Phase 57)"
REGISTER_SECTION = "## Dispositions"
EXTRA_IDS = {"40": ["IN-01", "IN-04"]}  # D-12: Phase 40's two info items

REVIEW_NAME_RE = re.compile(r"^(\d+(?:\.\d+)?)-REVIEW\.md$")
FINDING_RE = re.compile(r"^### ((?:WR|CR)-\d+):")
PHASE_CELL_RE = re.compile(r"^\d+(?:\.\d+)?$")
TOTALS_RE = re.compile(
    r"^\*\*Totals:\*\*\s*closed:\s*(\d+),\s*waived:\s*(\d+),\s*open:\s*(\d+)\s*$")
DISPOSITIONS = ("closed", "waived", "open")

COMMIT_RE = re.compile(r"\b(?=[0-9a-f]*[0-9])[0-9a-f]{7,40}\b")
TEST_RE = re.compile(r"\b[a-z][a-z0-9]*(?:_[a-z0-9]+){2,}\b")
REQ_RE = re.compile(r"\b([A-Z]{2,})-[0-9]+\b")
FINDING_PREFIXES = ("WR", "CR", "IN")  # a pointer back at a finding is not evidence
PATH_RE = re.compile(r"[A-Za-z0-9_.\-]+(?:/[A-Za-z0-9_.\-]+)+")
WINDOWS_RE = re.compile(r"\bWINDOWS row \d+\b")
LIVE_RE = re.compile(r"^LIVE\s+[^\s:]+:\d+")
PLACEHOLDERS = {
    "", "tbd", "tba", "todo", "none", "n/a", "na", "see above", "same as above",
    "ditto", "fixme", "?", "pending", "unknown", "later", "wip",
}
PLACEHOLDER_STRIP = " \t*`_.;:,!-–—"
PLACEHOLDER_START_RE = re.compile(r"^(tbd|todo|tba|fixme|see above)\b")


def display(path):
    absolute = os.path.abspath(path)
    try:
        relative = os.path.relpath(absolute, workspace)
    except ValueError:
        return path
    if relative == ".." or relative.startswith(".." + os.sep):
        return path
    return relative.replace(os.sep, "/")


def is_placeholder(text):
    norm = text.strip(PLACEHOLDER_STRIP).lower()
    return norm in PLACEHOLDERS or bool(PLACEHOLDER_START_RE.match(norm))


def has_pointer(text):
    if COMMIT_RE.search(text) or TEST_RE.search(text) or WINDOWS_RE.search(text):
        return True
    for match in REQ_RE.finditer(text):
        if match.group(1) not in FINDING_PREFIXES:
            return True
    for match in PATH_RE.finditer(text):
        candidate = match.group(0)
        last = candidate.rsplit("/", 1)[-1]
        if "." in last.strip(".") or candidate.count("/") >= 2:
            return True
    return False


def phase_key(value):
    return tuple(int(part) for part in value.split("."))


failures = []  # (class, phase, id, detail)


def fail(cls, phase, ident, detail):
    failures.append((cls, phase or "-", ident or "-", detail))


# --- Scope. ---
scope = None
if phases_arg.strip():
    scope = []
    for part in phases_arg.split(","):
        part = part.strip()
        if not PHASE_CELL_RE.match(part):
            print("ERROR: --phases wants a comma list of phase numbers, got %r" % phases_arg,
                  file=sys.stderr)
            sys.exit(2)
        scope.append(part)
elif phases_arg != "":
    print("ERROR: --phases is blank", file=sys.stderr)
    sys.exit(2)


def in_scope(phase):
    return scope is None or phase in scope


def scan_fenced(text):
    """Yield (lineno, line) for lines outside code fences; return unclosed-at lineno."""
    outside = []
    in_fence = False
    opened = 0
    for lineno, line in enumerate(text.splitlines(), start=1):
        if line.strip().startswith(FENCE):
            in_fence = not in_fence
            if in_fence:
                opened = lineno
            continue
        if not in_fence:
            outside.append((lineno, line))
    return outside, (opened if in_fence else 0)


def read_text(path):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            return fh.read()
    except (OSError, UnicodeDecodeError) as exc:
        fail("UNREADABLE", None, None,
             "cannot read %s as UTF-8 (%s)" % (display(path), type(exc).__name__))
        return None


# --- Archive: expected ids per phase, derived from the REVIEW.md headings. ---
expected = {}      # phase -> ordered unique list of ids
review_files = {}  # phase -> path
pointer_ok = {}    # phase -> bool
notes = []

if not os.path.isdir(archive_dir):
    fail("UNREADABLE", None, None, "archive %s is not a directory" % display(archive_dir))
else:
    names = []
    for sub in sorted(os.listdir(archive_dir)):
        subdir = os.path.join(archive_dir, sub)
        if not os.path.isdir(subdir):
            continue
        for fname in sorted(os.listdir(subdir)):
            match = REVIEW_NAME_RE.match(fname)
            if match and in_scope(match.group(1)):
                names.append((match.group(1), os.path.join(subdir, fname)))
    names.sort(key=lambda item: phase_key(item[0]))
    for phase, path in names:
        text = read_text(path)
        if text is None:
            review_files[phase] = path
            continue
        review_files[phase] = path
        outside, unclosed = scan_fenced(text)
        if unclosed:
            fail("UNCLOSED_FENCE", phase, None,
                 "the code fence opened at %s:%d is never closed" % (display(path), unclosed))
        ids = []
        for _, line in outside:
            found = FINDING_RE.match(line)
            if found and found.group(1) not in ids:
                ids.append(found.group(1))
        for extra in EXTRA_IDS.get(phase, []):
            if extra not in ids:
                ids.append(extra)
        expected[phase] = ids
        pointer_ok[phase] = any(line.rstrip() == POINTER_HEADING for _, line in outside)
        # Informational cross-check against the frontmatter counts.
        head = text.split("\n---", 1)[0] if text.startswith("---") else ""
        crit = re.search(r"^\s+critical:\s*(\d+)", head, re.M)
        warn = re.search(r"^\s+warning:\s*(\d+)", head, re.M)
        if crit and warn:
            n_cr = sum(1 for i in ids if i.startswith("CR-"))
            n_wr = sum(1 for i in ids if i.startswith("WR-"))
            if int(crit.group(1)) != n_cr or int(warn.group(1)) != n_wr:
                notes.append("NOTE MISMATCH %s: frontmatter critical=%s warning=%s, headings CR=%d WR=%d"
                             % (phase, crit.group(1), warn.group(1), n_cr, n_wr))

if scope is not None:
    for listed in scope:
        if listed not in review_files:
            fail("ZERO_FILES", listed, None, "no <NN>-REVIEW.md found for listed phase %s under %s"
                 % (listed, display(archive_dir)))
if not review_files and os.path.isdir(archive_dir):
    fail("ZERO_FILES", None, None, "no <NN>-REVIEW.md found under %s" % display(archive_dir))

for phase in sorted(pointer_ok, key=phase_key):
    if not pointer_ok[phase]:
        fail("NO_POINTER_SECTION", phase, None,
             "%s lacks the '%s' section" % (display(review_files[phase]), POINTER_HEADING))

# --- Register. ---
rows = []  # (phase, id, title, disposition, evidence, owner)
totals = None
register_text = read_text(register_path)
if register_text is not None:
    outside, unclosed = scan_fenced(register_text)
    if unclosed:
        fail("UNCLOSED_FENCE", None, None,
             "the code fence opened at %s:%d is never closed" % (display(register_path), unclosed))
    in_section = False
    for lineno, line in outside:
        stripped = line.strip()
        totals_match = TOTALS_RE.match(stripped)
        if totals_match:
            totals = tuple(int(g) for g in totals_match.groups())
            continue
        if stripped.startswith("## "):
            in_section = stripped == REGISTER_SECTION
            continue
        if not in_section or not stripped.startswith("|"):
            continue
        cells = [c.strip() for c in stripped.strip("|").split("|")]
        if not PHASE_CELL_RE.match(cells[0]):
            continue
        if len(cells) != 6:
            fail("MALFORMED_ROW", cells[0], cells[1] if len(cells) > 1 else None,
                 "%s:%d has %d cells, expected 6" % (display(register_path), lineno, len(cells)))
            continue
        rows.append(tuple(cells))

    if not rows:
        fail("ZERO_ROWS", None, None,
             "no data row found in the '%s' section of %s; parsing nothing is a named "
             "failure, never a pass" % (REGISTER_SECTION, display(register_path)))

# --- Row checks. ---
seen = {}
counts = {"closed": 0, "waived": 0, "open": 0}
for phase, ident, _title, disposition, evidence, owner in rows:
    disp = disposition.strip("*` ")
    if disp in counts:
        counts[disp] += 1
    if not in_scope(phase):
        continue
    key = (phase, ident)
    if key in seen:
        fail("DUPLICATE", phase, ident, "a second row for the same finding")
        continue
    seen[key] = True
    if ident not in expected.get(phase, []):
        fail("UNKNOWN", phase, ident, "not a finding of the archived %s-REVIEW.md" % phase)
        continue
    if disp not in DISPOSITIONS:
        fail("BAD_DISPOSITION", phase, ident,
             "Disposition %r is not one of closed | waived | open" % disposition)
        continue
    if is_placeholder(evidence):
        fail("NO_EVIDENCE", phase, ident, "Evidence is empty or a placeholder: %r" % evidence)
        continue
    if disp == "closed" and not has_pointer(evidence):
        fail("CLOSED_NO_POINTER", phase, ident,
             "no commit, test name, requirement id, repo path or WINDOWS row in Evidence")
    elif disp == "waived" and is_placeholder(owner):
        fail("WAIVED_NO_OWNER", phase, ident, "a waived finding needs an owner")
    elif disp == "open":
        if not pending_ok:
            fail("OPEN", phase, ident, "the finding is still open")
        elif not LIVE_RE.match(evidence.replace("`", "")):
            fail("OPEN", phase, ident,
                 "an open row needs 'LIVE <file:line> — <class>: ...' under --pending-ok")

for phase in sorted(expected, key=phase_key):
    for ident in expected[phase]:
        if (phase, ident) not in seen:
            fail("MISSING", phase, ident, "archived finding has no register row")

if register_text is not None:
    if totals is None:
        fail("TOTALS", None, None, "no '**Totals:** closed: <n>, waived: <n>, open: <n>' line")
    elif totals != (counts["closed"], counts["waived"], counts["open"]):
        fail("TOTALS", None, None,
             "Totals line says closed=%d waived=%d open=%d but the rows count closed=%d "
             "waived=%d open=%d" % (totals + (counts["closed"], counts["waived"], counts["open"])))

n_findings = sum(len(ids) for ids in expected.values())
n_files = len(review_files)
for note in notes:
    print(note)
for cls, phase, ident, detail in failures:
    print("FAIL %s %s %s: %s" % (cls, phase, ident, detail))
print("checked %d finding(s) across %d review file(s); %d failure(s)"
      % (n_findings, n_files, len(failures)))
sys.exit(1 if failures else 0)
PY
