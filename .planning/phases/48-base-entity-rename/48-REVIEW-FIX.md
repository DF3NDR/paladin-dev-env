---
phase: 48-base-entity-rename
fixed_at: 2026-10-14T00:00:00Z
review_path: .planning/phases/48-base-entity-rename/48-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 48: Code Review Fix Report

**Fixed at:** 2026-10-14
**Source review:** .planning/phases/48-base-entity-rename/48-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (Critical: 0, Warning: 5; Info excluded by `fix_scope: critical_warning`)
- Fixed: 5
- Skipped: 0

All five findings are in `tests/entity_rename_guard.rs`. After each fix the file passed
`cargo test --test entity_rename_guard`, `cargo fmt --check` and
`cargo clippy --tests --test entity_rename_guard -- -D warnings`. The guard went from 5 tests to 8:
three new positive-control tests were added, and every existing positive control still passes.
The real-tree test `entity_surfaces_are_renamed_in_the_tree` passes after every commit.

Because these are guard-logic changes (matching conditions), they are flagged for a human look at
the matching rules, although the positive controls exercise each new behaviour.

## Fixed Issues

Commits are listed in the order applied. WR-04 was applied first because the WR-01 fix reuses
`production_lines`.

### WR-04: `production_lines` truncates at any line starting with `mod tests`

**Files modified:** `tests/entity_rename_guard.rs`
**Commit:** ceeb148e
**Status:** fixed: requires human verification (matching logic)
**Applied fix:** Added `opens_test_module`, which recognises only the whole inline item
`mod tests {` or `mod test {` (optional `pub` / `pub(crate)` / `pub(super)`). `mod tests_support {`,
`mod testsuite {` and the out-of-line `mod tests;` no longer end the scan early. Every real target
file declares `#[cfg(test)]` then `mod tests {`, so real behaviour is unchanged. New control
`production_scan_stops_only_at_an_inline_test_module` plants a violation after each look-alike
opener and asserts rule 3 reports it, and asserts qualified openers still stop the scan.

### WR-01: Rule 5 (graph freeze) satisfied by unrelated occurrences

**Files modified:** `tests/entity_rename_guard.rs`
**Commit:** cd6e95c0
**Status:** fixed: requires human verification (matching logic)
**Applied fix:**
- Rust needles are now matched by `declares`: a production line (via `production_lines`, so
  comments and test modules are excluded) must begin with the needle after indentation, with an
  identifier boundary when the needle ends in an identifier character. Match arms such as
  `TraceEvent::NodeStarted {` and test-module repeats no longer count.
- The two schema keys moved out of the substring table (`GRAPH_DECLARATIONS` is now 11 entries)
  into `SCHEMA_KEYS`, JSON pointers `/properties/nodes` and
  `/$defs/LimitsDoc/properties/max_node_visits`, checked against the `serde_json`-parsed document
  in the new `scan_schema`. The `required` list and the `limits.default` example no longer
  satisfy them. Unparsable JSON is itself a violation.
- `serde_json` is a regular `[dependencies]` entry of the root crate, so integration tests can use
  it without adding a dependency.
- New control `scanner_reports_graph_declarations_that_only_appear_elsewhere` covers a removed
  `NodeStarted {` whose text survives in a match arm, a comment and a test module; each schema key
  removed while its name survives in `required` and an example; and a non-JSON schema.
- A doc-comment line-length tidy for WR-04 (`opens_test_module`) was included in this commit.

### WR-03: The "no `docs/` directory" carve-out disables the schema freeze in any docs-less tree

**Files modified:** `tests/entity_rename_guard.rs`
**Commit:** cdefd67c
**Status:** fixed
**Applied fix:** The schema is now skipped only when the tree holds none of `DOCKER_EXCLUDED_DIRS`
(`docs`, `notes`, `k8s`, `.github`), the directories `.dockerignore` keeps out of the Docker image.
A sparse checkout or packaged tree that merely lacks `docs/` is still reported. The skip prints
`rule 5: schema check skipped, ...` via `eprintln!`. Limit: libtest only shows that notice with
`--nocapture` or when a test fails, so it is visible but not in a default passing run. The
docs-less Docker case still passes (`scanner_accepts_a_build_context_without_docs` now removes
all four directories), and the new `scanner_reports_a_missing_schema_when_only_docs_is_absent`
pins the narrowing. I kept the schema check inside `scan` rather than a separate `#[test]`, since
the real-tree and scratch-tree tests share that entry point.

### WR-02: The guard silently skips every dot-directory

**Files modified:** `tests/entity_rename_guard.rs`
**Commit:** 6a48b71e
**Status:** fixed
**Applied fix:** `skip_directory` now skips `target`, `node_modules` and the explicit
`SKIPPED_DOT_DIRECTORIES` (`.git`, `.planning`, `.project`). `.github/` and `.claude/` are
scanned; both are clean today, and the vendored GSD files under `.claude/` did not trip the guard,
so `.claude` did not need to stay skipped. Letting every other dot-directory through exposed one
legitimate hit: `.cargo/semver-checks-allowlist.toml`, which mirrors `MIGRATION.md` 9.2 and names
the removed items by necessity. It is recorded as an exact-path history file in `HISTORY_FILES`
(now 4 entries), not as a directory skip, and the header was updated to say so. New controls: planted
violations in `.github/copilot-instructions.md`, `.github/workflows/ci.yml` and
`.claude/commands/note.md` must each be reported (rule named); the clean tree now carries
look-alike files in `.github/` and `.claude/` and old-name files in `.git/` and the semver
allowlist, which must pass.

### WR-05: Rule 4 declaration needles are formatting- and visibility-sensitive

**Files modified:** `tests/entity_rename_guard.rs`
**Commit:** 41837b56
**Status:** fixed: requires human verification (matching logic, includes a small lexer)
**Applied fix:** Each line is run through `normalize_declaration` before rule 4 matching
(whitespace collapsed, `pub(crate)` / `pub(super)` / `pub(in ..)` read as `pub`, no space before
`<`, one space before `=`). Rule 4 now reads code only: `blank_non_code` removes `//` and nested
`/* */` comments and empties ordinary, raw and character literals (keeping newlines and handling
lifetimes), so a declaration inside a block comment or a raw string no longer counts. I used this
small lexer rather than `syn` to avoid a new dependency. The existing "declared 0 times in the
home file" check backstops a lexer that swallows a real declaration. New controls: respelled
duplicates (restricted visibility, extra spaces, spaced `<`, tight `=`) are each reported;
look-alikes in block comments, raw strings, plain strings and lifetime/quote literals pass; and a
real duplicate placed after quote-and-lifetime literals is still found at its own line.

## Skipped Issues

None. All in-scope findings were fixed.

## Notes for the orchestrator

- Isolation: I worked directly on branch `claude/laughing-dirac-e0h2ax` in
  `/home/user/paladin-dev-env` instead of the temp worktree, as the caller named that branch and
  was blocked waiting. No worktree, temp branch or recovery sentinel was created, so nothing needs
  cleanup. Nothing was pushed.
- Disk space: the first test run failed with `No space left on device`. I deleted the
  git-ignored, regenerable `target/debug/incremental` and `target/debug/examples` to recover
  about 8.6 GB. No tracked or source file was touched.
- Info findings IN-01..IN-05 were out of scope (`fix_scope: critical_warning`) and were not
  attempted.
- One pre-existing line in the guard file is 101 characters (the `skip_file` doc comment); rustfmt
  does not rewrap comments and it was left untouched.

---

_Fixed: 2026-10-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
