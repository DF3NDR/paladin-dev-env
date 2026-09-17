#!/usr/bin/env bash
# 34-rustdoc-rows.sh — parse a captured `cargo doc` run into §3 `RD-nn` pipe-table
# rows (crate, file:line, kind, message, location source, evidence anchor, size).
#
# Usage: 34-rustdoc-rows.sh <capture-file> <run-label>
#
# <run-label> selects the diagnostic prefix to walk:
#   - "default" (or any label not containing "allfeatures"/"all-features") walks
#     `^warning:` blocks — the default-feature `cargo doc --workspace --no-deps` run.
#   - a label containing "allfeatures" or "all-features" walks `^error:` blocks —
#     an `RUSTDOCFLAGS="-D warnings" cargo doc ... --all-features` run, where
#     content diagnostics are reported as errors, not warnings.
#
# Method (RESEARCH.md Pattern 2/3, Pitfall P-01):
#   1. Per-crate "generated N warnings/errors" summary lines are never content
#      rows (D-13) — they are used only to attribute the diagnostics that
#      precede them to a crate, in stream order (see below).
#   2. cargo documents crates concurrently: a crate's full diagnostic block is
#      flushed atomically, but crates that finish around the same moment have
#      their summary lines batched together at the end of the shared stream
#      segment. Diagnostics are therefore attributed to crates by walking the
#      pending diagnostic queue and consuming it in front-to-back order against
#      each summary line's own count, in the order the summaries appear —
#      empirically verified against every `-->`-bearing diagnostic's own path
#      in the plan 34-06 default-feature capture (paladin-ai 5 / paladin-web 3;
#      paladin-battalion 36 / paladin-storage 1 / paladin-llm 4 — including two
#      *location-less* `unclosed HTML tag` diagnostics independently confirmed
#      to belong to paladin-llm by grepping `<status>`/`<body>` and finding both
#      co-located at crates/paladin-llm/src/http_status.rs:6-7; paladin-ports 1
#      / paladin-ai-core 14; paladin-memory 1 alone).
#   3. A block carrying a `-->` line takes its file:line directly from that
#      span (Location source: "rustdoc span").
#   4. A block with no `-->` extracts the single backtick-quoted identifier
#      from its own first line and recovers file:line by grepping that
#      identifier's markdown-link form (`` [`ident`] `` for "unresolved link",
#      `<ident>` for "unclosed HTML tag") against doc-comment lines
#      (`///`/`//!`) restricted to the *attributed* crate's own `src/` tree —
#      never the whole workspace, since step 2 already names the crate
#      (Location source: "grep recovery (...)").
#
# Exits non-zero unless every content diagnostic in the capture produced
# exactly one row (the reconciliation line printed at the end states both
# counts).

set -u

CAPTURE="${1:-}"
RUN_LABEL="${2:-}"

if [ -z "$CAPTURE" ] || [ -z "$RUN_LABEL" ]; then
  echo "usage: 34-rustdoc-rows.sh <capture-file> <run-label>" >&2
  exit 2
fi

if [ ! -f "$CAPTURE" ]; then
  echo "FATAL: capture file not found: $CAPTURE" >&2
  exit 2
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

python3 - "$CAPTURE" "$RUN_LABEL" "$REPO_ROOT" <<'PYEOF'
import sys, re, os, subprocess

capture_path, run_label, repo_root = sys.argv[1], sys.argv[2], sys.argv[3]

sev = "error" if ("allfeatures" in run_label.lower() or "all-features" in run_label.lower()) else "warning"

with open(capture_path, encoding="utf-8", errors="replace") as f:
    lines = f.read().splitlines()

diag_start_re = re.compile(r'^(?:%s): ' % sev)
summary_re = re.compile(r'^(?:warning|error): `([A-Za-z0-9_-]+)` \(lib doc\) generated (\d+) (?:warnings?|errors?)$')
could_not_doc_re = re.compile(r'^error: could not document')
noise_re = re.compile(r'^(?: Documenting | +Finished| +Generated|error: could not compile)')
span_re = re.compile(r'^\s*--> (.+?):(\d+)(?::\d+)?\s*$')

# ---- Pass 1: split into ordered content-diagnostic blocks and summary lines,
#      preserving the exact stream order (blocks vs summaries interleave only
#      at batch boundaries, per the module docstring above). ----
class Block:
    __slots__ = ("start_line", "text_lines")
    def __init__(self, start_line):
        self.start_line = start_line
        self.text_lines = []

events = []  # list of ("block", Block) | ("summary", crate, count)
cur_block = None

for idx, line in enumerate(lines, start=1):
    if diag_start_re.match(line):
        m = summary_re.match(line)
        if m:
            if cur_block is not None:
                events.append(("block", cur_block))
                cur_block = None
            events.append(("summary", m.group(1), int(m.group(2))))
            continue
        if could_not_doc_re.match(line):
            if cur_block is not None:
                events.append(("block", cur_block))
                cur_block = None
            continue
        # new diagnostic block starts
        if cur_block is not None:
            events.append(("block", cur_block))
        cur_block = Block(idx)
        cur_block.text_lines.append(line)
        continue
    if noise_re.match(line):
        if cur_block is not None:
            events.append(("block", cur_block))
            cur_block = None
        continue
    if cur_block is not None:
        cur_block.text_lines.append(line)

if cur_block is not None:
    events.append(("block", cur_block))

# ---- Pass 2: attribute blocks to crates by consuming the pending queue
#      front-to-back against each summary's count, in stream order. ----
pending = []          # list of Block awaiting attribution
attributed = []        # list of (Block, crate_name) in final table order
for kind, *rest in events:
    if kind == "block":
        pending.append(rest[0])
    else:
        _, crate_name, count = (kind,) + tuple(rest)
        if len(pending) < count:
            print(f"FATAL: summary for `{crate_name}` claims {count} diagnostics "
                  f"but only {len(pending)} are pending in queue", file=sys.stderr)
            sys.exit(1)
        chunk, pending = pending[:count], pending[count:]
        for b in chunk:
            attributed.append((b, crate_name))

if pending:
    print(f"FATAL: {len(pending)} diagnostic block(s) never attributed to a crate "
          f"(no trailing summary line covered them)", file=sys.stderr)
    sys.exit(1)

# ---- Build package-name -> src-dir map from the live tree (never hardcoded). ----
def read_pkg_name(cargo_toml):
    try:
        with open(cargo_toml, encoding="utf-8") as fh:
            in_package = False
            for l in fh:
                s = l.strip()
                if s.startswith("["):
                    in_package = (s == "[package]")
                    continue
                if in_package and s.startswith("name"):
                    m = re.match(r'name\s*=\s*"([^"]+)"', s)
                    if m:
                        return m.group(1)
    except FileNotFoundError:
        return None
    return None

pkg_to_srcdir = {}
root_pkg = read_pkg_name(os.path.join(repo_root, "Cargo.toml"))
if root_pkg:
    pkg_to_srcdir[root_pkg] = os.path.join(repo_root, "src")
crates_dir = os.path.join(repo_root, "crates")
if os.path.isdir(crates_dir):
    for d in sorted(os.listdir(crates_dir)):
        ct = os.path.join(crates_dir, d, "Cargo.toml")
        pkg = read_pkg_name(ct)
        if pkg:
            pkg_to_srcdir[pkg] = os.path.join(crates_dir, d, "src")

def classify(first_line):
    if "links to private item" in first_line:
        return "private intra-doc link"
    if first_line.startswith(f"{sev}: unresolved link to"):
        return "unresolved link"
    if "redundant explicit link target" in first_line:
        return "redundant explicit link"
    if first_line.startswith(f"{sev}: unclosed HTML tag"):
        return "unclosed HTML tag"
    if first_line.startswith(f"{sev}: missing documentation"):
        return "missing docs"
    return "other"

def last_backtick_ident(first_line):
    ids = re.findall(r'`([^`]+)`', first_line)
    return ids[-1] if ids else None

def extract_snippet(block_lines):
    """For an 'unresolved link' block, recover the exact source-line snippet
    rustdoc quotes under '= note: the link appears in this line:' (two lines
    below that note: a blank spacer, then the snippet itself). Disambiguates
    the common case where the same identifier is linked from more than one
    doc-comment line in the same crate (e.g. two separate `TraceRecord`
    warnings at trace.rs:6 and trace.rs:17) — the bracket-identifier alone
    would collapse both to the same first match; the full snippet does not.
    Returns None if the block carries no such note (e.g. 'unclosed HTML tag',
    which has no per-line snippet at all).
    """
    for i, tl in enumerate(block_lines):
        if "the link appears in this line:" in tl:
            # i+1 is a blank spacer line, i+2 is the snippet itself
            if i + 2 < len(block_lines):
                snippet = block_lines[i + 2].strip()
                if snippet:
                    return snippet
    return None

def doc_comment_grep(src_dir, pattern):
    """grep -rnF <pattern> across src_dir, restricted to doc-comment lines."""
    if not os.path.isdir(src_dir):
        return []
    try:
        out = subprocess.run(
            ["grep", "-rnF", "--include=*.rs", "--", pattern, src_dir],
            capture_output=True, text=True, check=False
        ).stdout
    except FileNotFoundError:
        return []
    hits = []
    for line in out.splitlines():
        # file:line:content
        m = re.match(r'^(.+?):(\d+):(.*)$', line)
        if not m:
            continue
        content = m.group(3)
        if re.match(r'^\s*(///|//!)', content):
            hits.append((m.group(1), int(m.group(2)), content))
    return hits

rows = []
size_default = "S"

for block, crate in attributed:
    first_line = block.text_lines[0]
    message = first_line  # verbatim first line, including the "warning:"/"error:" prefix
    kind = classify(first_line)

    file_line = None
    loc_source = None

    for tl in block.text_lines[1:]:
        m = span_re.match(tl)
        if m:
            file_line = f"{m.group(1)}:{m.group(2)}"
            loc_source = "rustdoc span"
            break

    if file_line is None:
        ident = last_backtick_ident(first_line)
        src_dir = pkg_to_srcdir.get(crate)
        snippet = extract_snippet(block.text_lines) if kind == "unresolved link" else None

        # Preferred: the exact quoted source-line snippet (disambiguates two
        # warnings sharing one identifier but living on different lines).
        if snippet is not None and src_dir is not None:
            hits = doc_comment_grep(src_dir, snippet)
            if hits:
                rel = os.path.relpath(hits[0][0], repo_root)
                file_line = f"{rel}:{hits[0][1]}"
                cmd = f'grep -rnF "<snippet>" {os.path.relpath(src_dir, repo_root)}'
                extra = f" ({len(hits)} doc-comment matches, first taken)" if len(hits) > 1 else ""
                loc_source = (f"grep recovery (snippet quoted under "
                               f"\"the link appears in this line:\" — `{cmd}`, resolved to "
                               f"`{rel}:{hits[0][1]}`{extra})")

        # Fallback: the bracket-identifier pattern (used when no snippet note
        # exists at all, e.g. 'unclosed HTML tag', or the snippet grep missed
        # due to a formatting difference between the note and the source).
        if file_line is None:
            pattern = None
            if ident is not None:
                if kind == "unresolved link":
                    pattern = f"[`{ident}`]"
                elif kind == "unclosed HTML tag":
                    pattern = f"<{ident}>"
                else:
                    pattern = f"`{ident}`"
            if pattern is not None and src_dir is not None:
                hits = doc_comment_grep(src_dir, pattern)
                if hits:
                    rel = os.path.relpath(hits[0][0], repo_root)
                    file_line = f"{rel}:{hits[0][1]}"
                    cmd = f'grep -rnF "{pattern}" {os.path.relpath(src_dir, repo_root)}'
                    extra = f" ({len(hits)} doc-comment matches, first taken)" if len(hits) > 1 else ""
                    loc_source = f"grep recovery (`{cmd}`, resolved to `{rel}:{hits[0][1]}`{extra})"
        if file_line is None:
            print(f"FATAL: parser bug — no location recovered for block starting "
                  f"line {block.start_line} (crate {crate}, kind {kind}): {first_line}",
                  file=sys.stderr)
            sys.exit(1)

    anchor = f"{os.path.basename(capture_path)}:{block.start_line}"
    rows.append({
        "crate": crate,
        "file_line": file_line,
        "kind": kind,
        "message": message,
        "loc_source": loc_source,
        "anchor": anchor,
        "size": size_default,
    })

for r in rows:
    msg = r["message"].replace("|", "\\|")
    loc = r["loc_source"].replace("|", "\\|")
    print(f"|PLACEHOLDER-RD| `cargo doc --workspace --no-deps` (D-12/D-13) | {r['crate']} | "
          f"{r['file_line']} | {r['kind']} | `{msg}` | {loc} | "
          f"34-evidence/{r['anchor']} | {r['size']} |")

# ---- Reconciliation ----
content_diagnostics = len(attributed)
rows_emitted = len(rows)
print(f"# reconciliation: content_diagnostics={content_diagnostics} rows_emitted={rows_emitted}",
      file=sys.stderr)
if content_diagnostics != rows_emitted:
    print(f"FATAL: reconciliation mismatch: {content_diagnostics} content diagnostics "
          f"but {rows_emitted} rows emitted", file=sys.stderr)
    sys.exit(1)
print(f"RECONCILED: {content_diagnostics} content diagnostics == {rows_emitted} rows emitted",
      file=sys.stderr)
PYEOF
