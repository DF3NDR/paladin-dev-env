---
phase: 48-base-entity-rename
fixed_at: 2026-10-14T00:00:00Z
review_path: .planning/phases/48-base-entity-rename/48-REVIEW.md
iteration: 1
findings_in_scope: 10
fixed: 10
skipped: 0
status: all_fixed
---

# Phase 48: Code Review Fix Report

**Fixed at:** 2026-10-14
**Source review:** .planning/phases/48-base-entity-rename/48-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 10 (Critical: 0, Warning: 5, Info: 5; `fix_scope: all`)
- Fixed: 10
- Skipped: 0

The five warnings were fixed in a first pass and the five info findings in a second pass, both
on iteration 1.

All five warning findings are in `tests/entity_rename_guard.rs`. After each fix the file passed
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

## Info findings

Second pass. Each finding is one commit. After the pass: `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean, `cargo test --test entity_rename_guard`
(8 tests) still passes, and `tests/scripts/check-migration-allowlist_test.sh` passes (12
assertions).

### IN-01: The version-key coverage claim in the compat test is not enforced

**Files modified:** `crates/paladin-core/tests/entity_serde_compat.rs`
**Commit:** 94724706
**Status:** fixed
**Applied fix:** Added `"data"` to the forbidden keys of the version record check in
`new_writes_never_emit_the_old_keys` and to the matching row of the doc table, so the claim that
the version record holds no `data` key is asserted. `cargo test -p paladin-ai-core --test
entity_serde_compat` passes (11 tests).

### IN-02: Stale plan-narrative comments in committed test and source files

**Files modified:** `crates/paladin-core/tests/entity_serde_compat.rs`,
`crates/paladin-memory/src/citadel/file_citadel.rs`, `crates/paladin-core/src/base/mod.rs`
**Commit:** 647ec9ed
**Status:** fixed
**Applied fix:** Comment-only. The compat test header now describes what the file verifies today
(fixtures load and re-serialize with only the renamed keys changed, alias edge cases, no old key on
new writes) and cites ADR-0060, with no plan numbers. The `FileCitadel` fixture test doc comment
is rewritten the same way. The public `base` module docs no longer carry the
`.planning/todos/...` path and now say the review is "tracked in the repository's planning notes
(ADR-0060 (g))". Verified with `cargo doc -p paladin-ai-core --no-deps`, `cargo check -p
paladin-memory --tests`, the compat test and the rename guard.

### IN-03: Leftover entity-sense `node` names in test code

**Files modified:** `tests/repository/mysql_content_repository_test.rs`,
`crates/paladin-core/src/base/service/entity_version_service.rs`
**Commit:** 4fe88fd8
**Status:** fixed
**Applied fix:** `reconstructed_node` became `reconstructed_entity` (2 sites) and the three
`"test_node"` entity names became `"test_entity"`. `cargo check --all-targets`,
`cargo test -p paladin-ai-core entity_version_service` (4 pass), `cargo test --test
entity_rename_guard` (8 pass) and, with the feature that gates the file,
`cargo test --features storage-mysql --test repository` (9 pass, 1 ignored) all pass.
Not changed, because outside the cited lines: the test fn `test_node_reconstruction_pattern` and
the comment "reconstruct nodes from database" in the same mysql test file, and
`test_node_pattern_accessible_without_cli` / the `"NodeTest"` strings in
`tests/cli_isolation_test.rs`. They are further entity-sense `node` leftovers of the same kind and
could be renamed in a follow-up.

### IN-04: Crate-wide `module_missing` / `trait_missing` allows are not policed by the CI set-equality check

**Files modified:** `scripts/check-migration-allowlist.sh`,
`tests/scripts/check-migration-allowlist_test.sh`
**Commit:** eefc71b3
**Status:** fixed
**Applied fix:** After the existing `crate | type` set-equality comparison, the script now reads
each crate's `[package.metadata.cargo-semver-checks.lints]` table (root `Cargo.toml` and
`crates/*/Cargo.toml`, via `MANIFEST_ROOT`) and requires every lint set to `"allow"` to appear in
at least one `.cargo/semver-checks-allowlist.toml` entry for that same crate. Unmirrored
`crate | lint` pairs are listed and the script exits non-zero; the original diff failure and the
new failure are both reported. The current tree passes with no allowlist change: all 22 crate-wide
allows (`paladin-ai` 1, `paladin-battalion` 2, `paladin-ai-core` 10, `paladin-ports` 5,
`paladin-web` 4) are already mirrored, so there were no unmirrored lint ids to report. The
regression harness now points its fixture cases at an empty manifest root and gains three cases
(unmirrored lint fires and is named, lint mirrored only under another crate fires, fully mirrored
passes). `shellcheck` is not installed here; `bash -n` passes on both scripts. Limits: this is a
local extension only, since the CI step in `ci.yml` is an inline copy and the script is not wired
into any CI job (see `make check-gates`), and the header of the script says so.

### IN-05: Cross-crate fixture path couples `paladin-memory`'s test to a sibling directory

**Files modified:** `crates/paladin-memory/src/citadel/file_citadel.rs`
**Commit:** d0146f7f
**Status:** fixed
**Applied fix:** `pre_rename_citadel_snapshot_loads_through_file_citadel` now checks that
`../paladin-core/tests/fixtures/pre_rename/citadel.json` exists. If it does not, it prints an
`eprintln!` notice naming the test and the missing path and returns `Ok(())`; the fixture is not
copied. Verified with `cargo test -p paladin-memory citadel` (16 pass) and by temporarily moving
the fixture away (the notice printed and the test passed; the fixture was restored, tree clean).

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
- The Info findings IN-01..IN-05 were fixed in a second pass (`fix_scope: all`); see the
  "Info findings" section.
- The second pass also worked directly on `claude/laughing-dirac-e0h2ax` (no worktree, temp
  branch or sentinel), at the caller's instruction. Nothing was pushed and the disk did not fill.
- One pre-existing line in the guard file is 101 characters (the `skip_file` doc comment); rustfmt
  does not rewrap comments and it was left untouched.

---

_Fixed: 2026-10-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
