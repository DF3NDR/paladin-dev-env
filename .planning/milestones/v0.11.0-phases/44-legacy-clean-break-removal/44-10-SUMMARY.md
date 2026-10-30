---
phase: 44-legacy-clean-break-removal
plan: 10
subsystem: battalion-core
tags: [clean-break, battalion, aegis, api-surface, removal]
status: complete

requires:
  - phase: 44-06
    provides: BattalionResult.node_errors as structured NodeError (left the legacy summary struct unused)
  - phase: 44-07
    provides: Commander and Conclave on Aegis (left ConclaveConfig.timeout_seconds / with_timeout / ConclaveError::Timeout unread)
  - phase: 44-08
    provides: CLI, examples and doc-examples on with_aegis (no consumer of the legacy builders left)
  - phase: 44-09
    provides: mdBook pages describing the Aegis form
provides:
  - legacy Battalion RetryPolicy, ErrorStrategy and the name-plus-text NodeError deleted from paladin-core
  - BattalionConfig reduced to name, description, aegis, metadata_output_dir
  - BattalionError::Timeout, ConclaveError::Timeout, ConclaveConfig.timeout_seconds and with_timeout deleted
  - legacy paladin_battalion::retry module and the services::battalion::retry facade re-export deleted
  - regenerated .project/current-exports.txt
affects: [44-11, 44-12, 44-13]

tech-stack:
  added: []
  patterns:
    - "Consumers first, definitions last: tests migrated and committed, then a trial deletion proved only the two core files broke, then the real deletion"
    - "v0.10 documents stay readable by plain serde ignoring of unknown keys (no deny_unknown_fields, alias or untagged shim)"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/battalion/mod.rs
    - crates/paladin-core/src/platform/container/battalion/conclave.rs
    - crates/paladin-core/src/platform/container/aegis.rs
    - crates/paladin-battalion/src/lib.rs
    - crates/paladin-battalion/src/error_aggregation.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - src/application/services/battalion/mod.rs
    - .project/current-exports.txt
    - tests/integration/battalion/load_test.rs
    - tests/integration/battalion/campaign_integration_test.rs
    - tests/integration/battalion_campaign_integration_test.rs
    - tests/integration/battalion_chain_of_command_integration_test.rs
    - tests/integration/commander_error_paths_test.rs
    - tests/integration/commander_integration_tests.rs
    - tests/unit/battalion/formation_tests.rs
    - tests/unit/battalion/campaign_service_tests.rs
  deleted:
    - crates/paladin-battalion/src/retry.rs

key-decisions:
  - "Removed names are described by concept in every comment and rustdoc, so the 44-13 source-tree scan starts clean"
  - "The v0.10 fixture test carries the three removed keys as JSON strings with strategy value FailFast (no guard-forbidden variant name)"
  - "The chain-of-command test drops its dead timeout assignment and is renamed test_chain_with_default_config_completes (ChainOfCommand is not bounded by aegis, ADR-0059 (c))"

patterns-established:
  - "Campaign test bounds each attempt with with_aegis and a 1 s TimeoutPolicy instead of a whole-run field"

requirements-completed: [LEGACY-01, LEGACY-02, LEGACY-03]

duration: ~55min
completed: 2026-10-11
---

# Phase 44 Plan 10: Delete the legacy Battalion definitions Summary

**The legacy Battalion `RetryPolicy` / `ErrorStrategy` / `NodeError`, the three `BattalionConfig` fields and builders, both timeout variants, the `ConclaveConfig` timeout and the third backoff module (`paladin_battalion::retry`) are gone; the same-named Aegis and Flow types are untouched and a v0.10 config document still deserializes with `aegis == Aegis::default()`.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 3 of 3
- **Files modified:** 16 (+1 deleted)

## Accomplishments

- Task 1: deleted `crates/paladin-battalion/src/retry.rs`, its `pub mod` and crate-doc bullet, and the facade `pub use paladin_battalion::retry;`. `engine::retry` and `retry_predicate` untouched. `error_aggregation.rs` docs now describe patterns that continue past a failed Paladin under `aegis.on_error = Absorb`; its three `BattalionError::Timeout(300)` literals became `ExecutionError("Paladin never answered")`.
- Task 2: migrated the last test writers (see Task 2 trial deletion below).
- Task 3: removed the definitions from `battalion/mod.rs` and `conclave.rs`, reworded the `aegis.rs` module doc and the `BattalionError::Node` rustdoc by concept, and added `battalion_config_v0_10_document_deserializes_with_default_aegis`.

## Task Commits

1. Task 1: `9a9b1d3f` refactor(44-10): delete the legacy battalion retry module and its facade re-export
2. Task 2: `c6adc336` test(44-10): sweep the last test writers of the legacy battalion builders
3. Task 3: `84f8e902` refactor(44-10): delete the legacy battalion definitions from paladin-core

(Task 3 is `tdd="true"`: the new v0.10 deserialization test could not be written failing first, because the removed keys were already tolerated by plain serde; it pins behaviour that must survive the deletion rather than adding behaviour, so a single refactor commit carries it with the deletion. No `test(...)` RED commit exists for it.)

## API baseline (`.project/current-exports.txt`)

`PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update` changed exactly: the removed line `pub use paladin::application::services::battalion::retry`, the item count 4275 -> 4274, and the generated-at header timestamp. Nothing else. `make api-surface` then reports "API surface unchanged". The baseline only lists the root `paladin` crate's facade, so the `paladin-ai-core` removals (types, variants, fields) do not appear in it; they are covered by the 9.2 rows and semver allowlist in 44-11 and the CHANGELOG in 44-12 (not written here).

## Task 2 trial deletion

Uncommitted scratch deletion of the three fields and builders, the three legacy types, `BattalionError::Timeout`, `ConclaveError::Timeout`, `ConclaveConfig.timeout_seconds` and `with_timeout`, then `cargo check --workspace --all-targets --all-features --keep-going`. Result: 29 errors, all in `paladin-ai-core`'s lib-test target: 28 in `crates/paladin-core/src/platform/container/battalion/mod.rs` and 1 in `battalion/conclave.rs` (their own tests; the removed items' doctests were deleted with them). No error anywhere else, so no consumer outside the two core files needed fixing. Both files were restored with `git checkout --` afterwards, then the real deletion was done.

## Same-name survivors

- `git diff b362d6cb -- crates/paladin-core/src/platform/container/node_error.rs crates/paladin-battalion/src/maneuver/` is empty.
- `aegis.rs` diff against `b362d6cb` touches only `//!` lines (the module-doc paragraph that described the legacy pair as an untouched sibling family).
- `cargo test -p paladin-ai-core --lib aegis` (15 passed), `node_error` (9 passed), `cargo test -p paladin-battalion --lib maneuver` (75 passed).
- `BattalionConfig` has exactly four public fields (acceptance awk prints 4); no `deprecated` / `serde(alias` / `serde(untagged` / `deny_unknown_fields` string in `battalion/mod.rs` or `conclave.rs`.

## Verification run (real results)

- `cargo check --workspace --all-targets --all-features`: ok
- `cargo fmt --check`: ok
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: ok
- `make doc-check` (rustdoc zero-warning, all-features `-D warnings`, `cargo test --workspace --doc`): passed
- `make check-doc-examples`: 0 failed; `make check-examples`: 62 of 62 example binaries present
- `cargo test --workspace --no-fail-fast`: the ONLY failures are the two known pre-existing `paladin-ai` lib tests `infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set` and `...build_run_api_persists_no_run_traces_by_default` (panic at `run_api_wiring.rs:2369`, recorded in deferred-items.md; 1348 passed / 2 failed in that target). Per the brief a run whose only failures are these two counts as passing. Every other test target passed.
- `cargo test -p paladin-ai --test lib -- integration::battalion::load_test integration::battalion::campaign_integration_test integration::battalion_campaign_integration_test integration::battalion_chain_of_command_integration_test`: 35 passed; `cargo test -p paladin-ai --test unit -- battalion`: 94 passed
- `cargo test --test treasurer_vocabulary_guard`: 3 passed
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: unchanged (exit 0)
- Not run: `make security` (no dependency change in this plan), `make test-integration-docker`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Stale intra-doc link in `engine/mod.rs`**
- **Found during:** Task 3 (`make doc-check`, step 1 zero-warning bar)
- **Issue:** the engine module docs linked `` [`retry`] `` by bare name, which had resolved to the crate-level module deleted in Task 1; rustdoc warned "unresolved link to `retry`".
- **Fix:** point the link at `crate::engine::retry` (same target the text already described); text unchanged.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs` (outside the plan's file list)
- **Commit:** `84f8e902`

**2. [Rule 2 - Missing critical] Residual legacy strategy names in two commander test files**
- **Found during:** Task 3 residual grep ("leave every removed name absent from tests so 44-13's scan starts clean")
- **Issue:** `tests/integration/commander_error_paths_test.rs` and `commander_integration_tests.rs` still named the two removed v0.10 strategy variants in `expect(..)` / `assert!` messages and comments. Plan 44-13 names only `tests/helpers/mock_paladin_port.rs` as the last residual, so its repository scan (rule 1, whole-word, `.rs`) would have failed on these.
- **Fix:** reworded the messages and comments by concept ("an Absorb handler ...", "retry then Absorb ..."); no code or assertion logic changed.
- **Files modified:** the two test files above (outside the plan's file list)
- **Commit:** `84f8e902`

**3. [formatting] `cargo fmt` touched `error_aggregation.rs`**
- Rustfmt rewrapped the three new `ExecutionError(..)` test literals from Task 1; committed with Task 2 (`c6adc336`) rather than amending Task 1.

No other deviations. No disk-space failures occurred (target subdirectories were pruned before each full build; free space ended at 1.6 GB).

## Residual mentions left for later plans

- `tests/helpers/mock_paladin_port.rs` lines 74-75 doc comment still names the two removed strategy variants: owned by 44-13 (comment-only reword).
- `docs/assets/recordings/battalion-formation.cast` (a recording, line 11) and `.project/**` history files still name them; the `.cast` extension is not scanned by the 44-13 guard and `.project` is history.
- Generated non-gate files (`lcov.info`, `final-api.txt`, `api_surface_current.txt`) not edited, per the plan.

## Requirements

LEGACY-01, LEGACY-02 and LEGACY-03 success criteria are now fully met in code and marked complete in REQUIREMENTS.md: the three legacy Battalion types are gone (LEGACY-01); no non-test whole-run `timeout(` or `timeout_seconds` remains in Formation, Phalanx, Campaign (LEGACY-02, with the legacy field and builders now also deleted); `PaladinError::LlmError(String)` and the message-matching retry check are gone (LEGACY-03, verified by grep; the remaining `LlmError(String)` hits are other types: `HandoffError`, `PlanningError`, `PromptError`). LEGACY-04 stays pending (44-11, 44-12, 44-13).

## Threat Flags

None. No new network endpoint, auth path, file access or schema surface; this plan only deletes public items. T-44-25 (no shim) is confirmed by the greps above, T-44-26 by `battalion_config_v0_10_document_deserializes_with_default_aegis`, T-44-27 by the regenerated baseline (9.2 rows and allowlist are 44-11's).

## Known Stubs

None.

## Self-Check: PASSED

- `crates/paladin-battalion/src/retry.rs` absent; commits `9a9b1d3f`, `c6adc336`, `84f8e902` present in `git log`.
