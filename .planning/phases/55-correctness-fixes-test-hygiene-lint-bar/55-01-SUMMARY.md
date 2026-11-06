---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 01
subsystem: infra
tags: [shell-gate, stub-panics, paladin-storage, content-list-repository, check-gates, ci, fixes-01]
status: complete

requires:
  - phase: 54-first-run-usability
    provides: "the check-version-strings gate shape, harness skeleton, Makefile and gate-scripts CI wiring this plan copies"
provides:
  - "scripts/check-no-stub-panics.sh: offline, no-argument, check-only, zero-exemption gate over src/ of every publishable package"
  - "tests/scripts/check-no-stub-panics_test.sh: 34-assertion fixture-tree harness (auto-discovered by make test-shell-guards)"
  - "ten typed ContentListRepository refusals on SqliteStore and ten on MySqlContentRepository, each pinned by a table test"
  - "content_filtering_service.rs (never compiled) deleted"
  - "make check-no-stub-panics, a member of make check-gates, and its own step in the gate-scripts CI job"
affects: [55-12, 55-13, 55-10, phase 58 release gates]

tech-stack:
  added: []
  patterns:
    - "Gate never passes vacuously: NO_PACKAGES, ZERO_FILES and UNREADABLE are named failures, each pinned by an assert_fire case"
    - "Out-of-line #[cfg(test)] mod name; resolves to name.rs / name/mod.rs transitively, so test-only files are exempt by resolution, never by list"
    - "One private fn per adapter file builds the typed refusal, so the ten bodies cannot drift apart"

key-files:
  created:
    - scripts/check-no-stub-panics.sh
    - tests/scripts/check-no-stub-panics_test.sh
  modified:
    - crates/paladin-storage/src/sqlite_content_repository.rs
    - crates/paladin-storage/src/mysql_content_repository.rs
    - crates/paladin-content/src/services/mod.rs
    - docs/src/user-guides/content-processing.md
    - Makefile
    - .github/workflows/ci.yml
  deleted:
    - crates/paladin-content/src/services/content_filtering_service.rs

key-decisions:
  - "PD-01-1: status tokens in precedence order NO_PACKAGES, ZERO_FILES, UNREADABLE, FOUND, OK; every named failure and finding is printed before exit; the only environment read is the test-only STUB_PANICS_ROOT"
  - "PD-01-2: the refusal text is built by one private helper per file with the backend as a literal, on a single physical line so the text is greppable"
  - "A top-level #![cfg(test)] only makes a file test-only when it sits at brace depth 0, and an out-of-line test module is recognised only when the mod declaration is the attributed item itself, so an inline module's inner `mod x;` cannot exempt an unrelated file"

requirements-completed: [FIXES-01]

duration: ~12min
completed: 2026-11-01
---

# Phase 55 Plan 01: Stub-panic guard and typed ContentListRepository refusals Summary

**An offline zero-exemption gate now fails `make check-gates` and the CI gate-scripts job on any `todo!` / `unimplemented!` reachable outside `#[cfg(test)]` in a published crate, and the twenty `ContentListRepository` stubs on SQLite and MySQL return a typed `QueryError` refusal instead of panicking the caller's thread.**

## Performance

- **Plan-start SHA:** `7f2eabe93cb32bccf3da1eb3121172a382193525`
- **Tasks:** 3 (1 tracer, 2 auto)
- **Commits:** `75f3c05a2` (guard and harness), `13941aa4d` (refusals and deletion), `ef062e7dc` (Makefile and CI wiring)
- **Files:** 2 created, 6 modified, 1 deleted

## Accomplishments

- `scripts/check-no-stub-panics.sh` reads manifests with `tomllib`, scans every `.rs` under `src/` of each package whose `publish` is not `false`, blanks comments, doc comments, nested block comments, strings, byte strings, raw strings and char literals first, and exempts only `#[cfg(test)]` and `#[cfg(all(test, ...))]` items plus out-of-line test modules resolved transitively. `#[cfg(not(test))]` is not exempt. There is no exemption list, inline marker, environment switch or path filter.
- The harness has 34 assertions: 8 fire cases, 4 named failures, determinism (byte-identical output, sorted by path then line), a fixture no-mutation check, 16 silent cases, 2 real-tree cases and a real-tree checksum no-mutation case.
- Each of the ten methods per backend returns `Err(RepositoryError::QueryError("ContentListRepository::<method> is unfinished for the <sqlite|mysql> backend: content lists are not persisted"))`. Both `impl ContentListRepository` blocks carry rustdoc naming the unfinished G3 scaffolding and `55-BASE-LAYER-REVIEW.md`. No public signature changed; `git diff` against the plan-start SHA is empty for `crates/paladin-ports`, and `make api-surface` reports "API surface unchanged".
- `content_filtering_service.rs` and the two commented `mod` lines naming it are gone; `cargo check -p paladin-content --all-targets` builds with and without `--all-features`.

## Guard red run

Run on the real tree after the guard was committed and before any stub was touched (`./scripts/check-no-stub-panics.sh; echo "exit=$?"`), verbatim:

```
exit=1
🔍 Checking published crates for stub panics outside #[cfg(test)] ...
❌ Stub-panic check failed (FOUND)

crates/paladin-content/src/services/content_filtering_service.rs:21: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:22: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:23: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:24: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:25: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:26: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:627: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:631: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:635: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:639: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:643: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:651: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:659: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:663: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:667: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:671: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:650: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:654: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:658: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:662: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:666: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:674: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:682: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:686: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:690: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:694: todo! outside #[cfg(test)] in a published crate
scanned 624 file(s) (19 test-only), 26 finding(s)

If this failure is unexpected:
  1. Replace each reported stub with a typed Err (or delete the dead code);
     a stub macro outside #[cfg(test)] in a published crate panics a caller.
  2. This gate has no exemption list, no inline marker and no environment
     switch (D-01); do not add one to silence a finding.
```

26 lines: 6 + 10 + 10, matching the planner's re-measurement exactly. `engine/test_support.rs` is not named.

Green run after Task 2 (the deleted file takes one file off the count):

```
✅ OK: scanned 623 file(s) (19 test-only), 0 finding(s)
```

## Watched failing

- Harness: `bash tests/scripts/check-no-stub-panics_test.sh` before the guard existed exited 1 with `ERROR: guard script not found`.
- `sqlite_content_repository::tests::content_list_repository_refuses_every_method_on_sqlite` panicked at `sqlite_content_repository.rs:650` (`not yet implemented: Implement ContentList get_by_id`) before the bodies changed, then passed.
- `mysql_content_repository::tests::content_list_repository_refuses_every_method_on_mysql` panicked at `mysql_content_repository.rs:627` (`not yet implemented: Implement content list retrieval`) before the bodies changed, then passed.

The tracer feedback gate ran in auto mode (`_auto_chain_active: true`): the tracer's `<verify>` was re-run end to end after its commit and passed (harness, shellcheck, mode `100755`, exit 1 with exactly 26 finding lines) before the expansion tasks started.

## Verification

- `bash tests/scripts/check-no-stub-panics_test.sh`: 34 assertions passed.
- `cargo test -p paladin-storage --features sqlite --offline --lib`: 508 passed. `--features mysql`: 302 passed. `cargo test -p paladin-content --offline --lib`: 75 passed.
- `cargo fmt --check`; `cargo clippy -p paladin-storage --features sqlite,mysql --tests --offline -- -D warnings`; `cargo clippy -p paladin-content --tests --offline -- -D warnings`: all clean.
- `make check-gates`, `make test-shell-guards` (85 assertions) and `make lint-shell`: exit 0. `shellcheck --severity=warning` (the pre-commit and `make lint-shell` severity) is clean on both new files.
- `make api-surface`: "API surface unchanged" (4325 items).

## actionlint not run (unrun-verify)

`actionlint` is not installed in this sandbox, so the `ci.yml` edit was checked only by `python3` YAML parsing (`yaml.safe_load` succeeds) and by the existing offline workflow guards inside `make check-gates` (suppressions and trigger-surface policy, both green). The CI "Workflow Lint" job is the proof of record, as in Phase 54-07 (PD-01-3). No `.planning/WINDOWS.md` row was opened, and that file is unchanged by this plan.

## Task Commits

1. **Task 1 (tracer): the guard and its harness** - `75f3c05a2` (feat)
2. **Task 2: refusals, table tests, dead file removed** - `13941aa4d` (fix)
3. **Task 3: Makefile and CI wiring** - `ef062e7dc` (ci)

## Decisions Made

- Status tokens, finding format and the `STUB_PANICS_ROOT` seam follow PD-01-1; the refusal helper follows PD-01-2; the actionlint handling follows PD-01-3.
- The refusal string is one physical line (no `\` continuation) in both the helper and the test, so the plan's `grep -c` acceptance checks, and any future drift check, can match it.
- Table tests call the trait through `ContentListRepository::<method>(&store, ..)`, because both stores also implement `ContentRepository` with identically named methods and a method-call spelling is ambiguous.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stale user guide row for the deleted module**
- **Found during:** Task 2, step 4 (the delete-safety grep)
- **Issue:** `docs/src/user-guides/content-processing.md` (lines 175 and 179) said `content_filtering_service` was "present but disabled (not compiled)" and told readers to wait for it to be "re-enabled". The plan's grep scope included `docs` and the plan said to stop and record any other reference. The reference is prose only (no code, no link target), and the deletion would have made it false.
- **Fix:** the row now reads "Not implemented: no filtering module exists" and the paragraph says the framework ships no filtering service.
- **Files modified:** `docs/src/user-guides/content-processing.md`
- **Commit:** `13941aa4d`

**2. [Rule 3 - Blocking] rustfmt reordered `services/mod.rs` after the deletion**
- **Found during:** Task 2, step 7
- **Issue:** the two commented `mod content_filtering_service;` lines had acted as group separators; with them gone `cargo fmt --check` wanted `content_summarizer_service` moved below the feature-gated `content_llm_analysis_service`.
- **Fix:** ran `cargo fmt -p paladin-content`; the change is a one-line reorder in the same file.
- **Files modified:** `crates/paladin-content/src/services/mod.rs`
- **Commit:** `13941aa4d`

**3. [Rule 1 - Bug] Harness expectation off by one**
- **Found during:** Task 1, first harness run
- **Issue:** the sort-order assertion expected line numbers that assumed a pre-existing header line; planted files are new, so their lines start at 1.
- **Fix:** corrected the expected string to `a.rs:1 b.rs:1 b.rs:2`.
- **Commit:** `75f3c05a2`

### Scanner refinements beyond the prototype (documented in the script header)

- A top-level `#![cfg(test)]` is honoured only at brace depth 0, and an out-of-line test module is recognised only when the `mod name;` declaration is itself the attributed item; the research prototype matched either anywhere in a file, which could let an inline module's inner `mod x;` exempt an unrelated file. The real-tree figures are unchanged (624 files, 19 test-only, 26 findings).

### Notes

- `make test-shell-guards` fails on an uncommitted `.github/workflows/ci.yml` because sibling harnesses assert `git status` is clean for that directory. It passed (85 assertions) once Task 3 was committed. This is existing harness behaviour, not a regression.
- The `docs/src/user-guides/content-processing.md` edit is outside the plan's `files_modified` list; it is recorded here so plan 55-12's changelog work can mention it if wanted.

## Known Stubs

None. The twenty refusing methods are intentional, typed, documented and tested; content-list persistence is out of scope (D-00d) and the keep / fence decision is plan 55-12's `55-BASE-LAYER-REVIEW.md`.

## Threat Flags

None. The guard is read-only and offline; the storage change removes a panic path and adds no endpoint, auth path, file access or schema change. The refusal text names only the method and the backend (T-55-03).

## Self-Check: PASSED

- FOUND: `scripts/check-no-stub-panics.sh` (mode `100755` in the index)
- FOUND: `tests/scripts/check-no-stub-panics_test.sh`
- MISSING (intentionally deleted): `crates/paladin-content/src/services/content_filtering_service.rs`
- FOUND commits: `75f3c05a2`, `13941aa4d`, `ef062e7dc`
