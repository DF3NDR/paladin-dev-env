---
phase: 44-legacy-clean-break-removal
plan: 13
subsystem: source-tree-guard
tags: [guard, clean-break, legacy-removal, tdd, adr-0059]
status: complete

requires:
  - phase: 44-10
    provides: the deletions the guard pins (legacy RetryPolicy / ErrorStrategy / NodeError, timeout variants and field, retry module)
  - phase: 44-11
    provides: MIGRATION.md / allowlist / manifest allow-line comments written by concept (no guard-forbidden path forms)
provides:
  - tests/legacy_removal_guard.rs, a std-only six-rule source-tree guard with two positive controls and the repository scan
  - FaultyPaladinPort doc comment reworded by concept (comment-only)
affects: [44-12]

tech-stack:
  added: []
  patterns:
    - "House guard shape (tests/treasurer_vocabulary_guard.rs): std::fs walk, forbidden strings assembled with concat, ScratchTree temp trees, planted + clean controls before the repository scan"
    - "File-scoped rules read their targets directly and report a missing target as a violation"

key-files:
  created:
    - tests/legacy_removal_guard.rs
  modified:
    - tests/helpers/mock_paladin_port.rs

key-decisions:
  - "Scoped-variant coverage (every forbidden form of rules 5 and 6 in every file those rules cover) and missing-target coverage live inside scanner_reports_planted_legacy_uses, so the file has exactly the three tests the plan names"
  - "Skip set is exactly the declared history set: target, node_modules, every dot-directory, every CHANGELOG.md, root MIGRATION.md, the two upgrade guides, the guard itself; no allowed look-alike was added to silence a hit"

patterns-established:
  - "Rules 1-4 are generic line scans (comments included); rules 5-6 are limited to non-comment lines before the first `mod tests` of named files"

requirements-completed: []
requirements-advanced: [LEGACY-01, LEGACY-02, LEGACY-03]  # already marked complete by 44-10; this plan adds the regression guard. LEGACY-04 stays pending until 44-12 lands.

duration: ~35min
completed: 2026-10-11
---

# Phase 44 Plan 13: Legacy-removal source-tree guard Summary

**`tests/legacy_removal_guard.rs` pins every Phase 44 removal on each `cargo test` run: six rules over `*.rs`/`*.md`/`*.toml`/`*.yaml`/`*.yml` outside the history set, two positive controls proven able to fail and to stay quiet, and a repository scan that is clean.**

## Task Commits

1. **Task 1 RED:** `bc1a97a8` test(44-13): add failing positive controls for the legacy-removal guard
2. **Task 1 GREEN:** `8eba8e00` feat(44-13): implement the legacy-removal scanner behind its positive controls
3. **Task 2:** `466df43d` test(44-13): pin every Phase 44 removal with a repository scan (third test + comment-only reword of `tests/helpers/mock_paladin_port.rs`)

## RED run (Task 1)

With `scan` replaced by an empty stub, `cargo test --test legacy_removal_guard -- scanner_` gave:

```
assertion `left == right` failed: []
  left: 0
 right: 22
test result: FAILED. 1 passed; 1 failed
failures: scanner_reports_planted_legacy_uses
```

(0 violations reported against 22 planted; the clean-tree control passes vacuously against a stub.) The GREEN run then reported `2 passed; 0 failed`.

## Rules implemented

1. Whole-word `ContinueOnError` / `RetryThenContinue` (so `SequentialContinueOnError` passes).
2. Path forms ending at an identifier boundary: `battalion::` + the three removed types, `BattalionError::Timeout`, `ConclaveError::Timeout`, the removed `PaladinError` LLM variant and the two removed predicates.
3. A `battalion::{ ... }` list (up to the first `}` on the line) naming a removed type as a whole word.
4. `.is_retryable()` call, `.with_retry_policy(` call, the legacy delay helper (whole word), `paladin_battalion::retry` and `services::battalion::retry` ending at a boundary (`retry_predicate`, `engine::retry` pass).
5. Non-comment lines before `mod tests` of the five legacy-pattern files: a `timeout(` call not preceded by an identifier character, and the `timeout_seconds` token. The runner (`aegis_attempt.rs`) is not scoped, so it stays the one allowed site.
6. Non-comment lines before `mod tests` of the core battalion `mod.rs` and `conclave.rs`: the removed field names, `ErrorStrategy`, and `struct` / `type` / `as` declarations or aliases of `RetryPolicy` or `NodeError`.

Rules 5 and 6 each read their targets directly and report a missing or unreadable target as a violation.

## Positive controls

- `scanner_reports_planted_legacy_uses`: 22 planted files, one violation each, each asserted to name its rule (rules 1-6 all covered, with `.rs`, `.md`, `.toml`, `.yaml`, `.yml`). It then plants every forbidden form of rule 5 (3 forms) in each of the 5 files and every form of rule 6 (10 forms) in each of the 2 files, requiring exactly one violation naming file and rule each time, and finally deletes each of the 7 scoped files in turn from a clean tree, requiring exactly one "target file is missing" violation.
- `scanner_accepts_a_clean_tree`: Aegis `RetryPolicy`, structured `NodeError`, Flow `ErrorStrategy`, `SequentialContinueOnError`, `HandoffError::LlmError(String)`, `PaladinError::LlmFailure`, `retry_predicate`, `engine::retry`, boundary look-alikes (`battalion::RetryPolicyConfig`, `battalion::NodeErrorKind`), a Maneuver `with_timeout` and the Conclave's `is_retryable_error` inside files named like scoped files, an `aegis_attempt.rs` with `tokio::time::timeout(`, test modules in scoped files using the removed key strings, and history files (every `CHANGELOG.md`, `MIGRATION.md`, `.planning/`, `.project/`, both upgrade guides, `target/`, `node_modules/`, a `.cast`, a `.txt`) naming every removed item: zero violations.

## Repository scan

First-run output (before the reword), exactly the two expected hits:

```
tests/helpers/mock_paladin_port.rs:74: rule 1: removed Phase 44 name `ContinueOnError`
tests/helpers/mock_paladin_port.rs:75: rule 1: removed Phase 44 name `RetryThenContinue`
```

After the comment-only reword (fail-fast with no `on_error` handler, `ErrorHandlerSpec::Absorb`, retry-then-Absorb) the scan reports nothing. `git diff b362d6cb -- tests/helpers/mock_paladin_port.rs` shows only comment lines changed; `grep -cw 'ContinueOnError\|RetryThenContinue'` on that file prints 0.

Real-failure demonstration (temporary, reverted, never committed): a `"timeout("` string constant planted before the test module of `formation_service.rs` and a markdown file naming a removed variant and a removed predicate made `legacy_surfaces_are_gone_from_the_tree` fail with `formation_service.rs:317: rule 5`, `zz_planted.md:1: rule 1` and `zz_planted.md:1: rule 2`. Both plants were removed (`git checkout --` of that one file, `rm` of the other); `git status` afterwards showed only the two intended files.

Skip list, reviewed by reading `skip_directory` / `skip_file`: `target`, `node_modules`, every dot-directory, every `CHANGELOG.md`, `MIGRATION.md`, `docs/src/api-reference/migration-guide.md`, `docs/src/api-reference/upgrading.md`, and `tests/legacy_removal_guard.rs`. Exactly the declared history set. No look-alike was added to silence a hit.

## Verification (real results)

- `cargo test --test legacy_removal_guard`: 3 passed, 0 failed
- `cargo test --test treasurer_vocabulary_guard`: 3 passed
- `cargo clippy -p paladin-ai --test legacy_removal_guard --test lib --all-features -- -D warnings`: ok
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: ok
- `cargo fmt --check`: ok
- `grep -c concat tests/legacy_removal_guard.rs`: 24

Not run: a full `cargo test --workspace` (only a doc comment and a new std-only test file changed; the other test targets are unaffected, and the two known `paladin-ai` baseline failures `run_api_wiring::tests::build_run_api_persists_*` were therefore not exercised), `make api-surface` (no `pub` item changed), `make security` (no dependency change).

## Deviations from Plan

None - plan executed as written, with these notes:

- The scoped-variant and missing-target controls are helper functions called from `scanner_reports_planted_legacy_uses` rather than a third `scanner_` test, so that `-- scanner_` reports 2 passed and the whole file reports 3, as the acceptance criteria state.
- My first demonstration plant (a bare statement at the top of `formation_service.rs`) broke compilation of the crate the guard test builds; I reverted it and re-planted a compile-safe string constant. No effect on the committed tree.
- No disk-space failure; `target/debug/examples` and `target/debug/incremental` were pruned before the workspace clippy run (3.6 GB free throughout).
- No file owned by 44-11 or 44-12 was edited, no CHANGELOG entry written, LEGACY-04 not ticked. No other file in the tree needed rewording: the scan's only hits were the two in the faulty-port comment.

## Known Stubs

None.

## Threat Flags

None. The plan adds a test and edits a comment; no network, auth, file-access or schema surface. T-44-33 is mitigated by the six rules plus the planted control, T-44-34 by the fixed skip set and the first-run output recorded above.

## Self-Check: PASSED

- `tests/legacy_removal_guard.rs` exists and contains `fn legacy_surfaces_are_gone_from_the_tree`.
- Commits `bc1a97a8`, `8eba8e00`, `466df43d` are on `claude/laughing-dirac-e0h2ax`.
