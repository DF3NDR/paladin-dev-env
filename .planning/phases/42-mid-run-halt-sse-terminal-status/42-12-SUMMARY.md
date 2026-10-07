---
phase: 42-mid-run-halt-sse-terminal-status
plan: 12
subsystem: treasurer-herald-closeout
tags: [allowance, herald, halt-reason, vocabulary-guard, windows-ledger, adr-conformance, phase-gate]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: HaltReason with its wire_json builder and TraceEvent::RunFinished.halt_reason (42-02, 42-05); the option-b design-gate outcome for a true streamed done (42-01, 42-08); the D-08 row 64 in the windows ledger (42-09); the operator halt notice and mid-run notices (42-10, 42-11)
  - phase: 41-admission-time-allowance-enforcement
    provides: AllowanceWarning::herald_line and the ExecutionMetadata warning helpers whose pattern the halt line copies; HeraldTraceSink's per-run fold
provides:
  - HaltReason::herald_line, HALT_REASON_METADATA_KEY, ExecutionMetadata::with_halt_reason and halt_reason_display
  - HeraldTraceSink folding RunFinished.halt_reason into the run's metadata, and one halt line in each of the markdown, JSON and table heralds
  - tests/treasurer_vocabulary_guard.rs, a std::fs scanner that keeps Treasurer a framework-only word and proves it can fail
  - three open rows in .planning/WINDOWS.md (the D-01 over-admission race, the uncut true streamed call under option-b, unpriced engine nodes), in both representations
  - Phase 42 entries in seven crate changelogs and the root changelog, two MIGRATION.md section 9.2 rows, and ADR-0057's Code Conformance flipped to conforms with every named test located in the tree
  - the full local phase gate, run and recorded
affects: [phase-42-verification, 46-docs-currency]

tech-stack:
  added: []
  patterns:
    - "The halt line copies Phase 41's warning-line pattern exactly: a core rendering method, a metadata key holding the rendered line, a sink fold at RunFinished, one render site per herald"
    - "A vocabulary guard proves it can fail through a planted temporary tree under std::env::temp_dir with a drop-removed scratch directory, never a fixed path"

key-files:
  created:
    - tests/treasurer_vocabulary_guard.rs
  modified:
    - crates/paladin-core/src/platform/container/allowance.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - src/infrastructure/telemetry/herald_sink.rs
    - crates/paladin-herald/src/markdown_herald.rs
    - crates/paladin-herald/src/json_herald.rs
    - crates/paladin-herald/src/table_herald.rs
    - src/application/services/treasurer/tests.rs
    - .planning/WINDOWS.md
    - .planning/decisions/0057-mid-run-halt-contract.md
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - crates/paladin-eval/CHANGELOG.md

key-decisions:
  - "The halt line states the currency once, after the ceiling (25.0000 of 25.0000 USD), as the plan's literal expected line requires: the balance is rendered through format_cost with its trailing currency code stripped when it equals the ceiling's, and a balance in a different currency keeps its own code"
  - "ExecutionMetadata stores the rendered line (a JSON string) under HALT_REASON_METADATA_KEY rather than the typed HaltReason, so no tenant, key name or figure beyond the rendered text can reach a herald through the key"
  - "The JSON herald adds one halt_reason string key, following its existing allowance_warning convention; it is a rendered line, distinct from the structured halt_reason object on the HTTP surface"
  - "The G2 windows row states the option-b scope: a true stream's done carries an informational halt_reason only when its terminal usage crossed the derived figure, so it is not byte-identical in every case"
  - "The vocabulary guard assembles both forbidden strings at run time so the test file never matches its own scan, and allowlists the one in-tree mention in the ledger module docs, itself, and the planning and project trees"

patterns-established:
  - "A phase closeout reads its ADR's conformance section against the tree by grep and names the file holding each test"

requirements-completed: [ALLOW-05, ALLOW-03, PLAT-09]

coverage:
  - id: D1
    description: "A halted run renders exactly one halt line per herald naming only the scope kind, the figures and the window end; a window ceiling, a lifetime ceiling and a ledger outage render the three specified lines; a run without a halt reason renders as before"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/allowance.rs#halt_reason_herald_line_for_a_window_ceiling, #halt_reason_herald_line_for_a_lifetime_ceiling, #halt_reason_herald_line_for_a_ledger_outage, #halt_reason_herald_line_names_no_tenant_or_key (red before HaltReason::herald_line existed: the test module did not compile)"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/herald.rs#with_halt_reason_records_one_line_under_the_key, #halt_reason_display_is_none_without_the_key, #halt_reason_display_is_none_for_a_non_string_value"
        status: pass
      - kind: unit
        ref: "crates/paladin-herald/src/markdown_herald.rs#finalize_stream_renders_exactly_one_halt_line_when_present; crates/paladin-herald/src/json_herald.rs#finalize_stream_adds_the_halt_key_only_when_present; crates/paladin-herald/src/table_herald.rs#finalize_stream_renders_exactly_one_halt_row_when_present"
        status: pass
    human_judgment: false
  - id: D2
    description: "HeraldTraceSink folds a RunFinished.halt_reason into the metadata handed to the herald, and a finish without one leaves the metadata and the Phase 41 sink behaviour unchanged"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/infrastructure/telemetry/herald_sink.rs#herald_sink_folds_a_halt_reason_into_one_line (red before the fold: the line was absent), with herald_sink_without_warnings_renders_as_before still passing"
        status: pass
    human_judgment: false
  - id: D3
    description: "Treasurer stays a framework-only word: the guard passes on the tree, reports every planted downstream use, accepts a clean tree, and adds no dependency"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "tests/treasurer_vocabulary_guard.rs#treasurer_is_a_framework_only_word, #scanner_reports_a_planted_downstream_use (red against a stub scanner: 0 violations reported, 4 expected), #scanner_accepts_a_clean_tree"
        status: pass
    human_judgment: false
  - id: D4
    description: "The known limitations are recorded as open windows with owner rationale in both the table and the trailing JSON block, with the counters updated, and the G2 row states the option-b scope"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: ".planning/WINDOWS.md rows 65, 66 and 67: four table rows with phase 42, four objects with phase 42 in the JSON block, total_count 67 equal to the table row count"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every changed crate's changelog names its Phase 42 surface, MIGRATION.md section 9.2 stays consistent with the allowlist, and ADR-0057's Code Conformance names only tests found in the tree"
    requirement: "ALLOW-05"
    verification:
      - kind: other
        ref: "./scripts/check-changelogs.sh exit 0; ./scripts/check-migration-allowlist.sh exit 0; every test name in the ADR section located by git grep in src, crates or tests"
        status: pass
    human_judgment: false
  - id: D6
    description: "The full local phase gate is green apart from the two known sandbox-only cases, with the CI-only legs named"
    requirement: "PLAT-09"
    verification:
      - kind: other
        ref: "cargo fmt --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, make clean-code, make check-gates, make security, make api-surface and both OpenAPI checks all exit 0; cargo test --workspace --no-fail-fast ends with only the two known sandbox-only cases not passing"
        status: pass
    human_judgment: false

duration: ~35 min
completed: 2026-10-07
status: complete
---

# Phase 42 Plan 12: Herald halt line, vocabulary guard and phase closeout Summary

**A Treasurer-halted run now renders one halt line in the markdown, JSON and table heralds (scope kind, figures and window end, never an identity), a root integration test keeps `Treasurer` a framework-only word and proves it can fail, the three known limitations are open rows in `.planning/WINDOWS.md`, ADR-0057 conforms to the tree, and the full local phase gate is green apart from two sandbox-only cases.**

## Performance

- **Duration:** about 35 minutes
- **Completed:** 2026-10-07
- **Tasks:** 3 (all auto; two TDD)
- **Files modified:** 18 (1 created)

## Red phase evidence

- **Task 1.** The new tests in `crates/paladin-core/src/platform/container/allowance.rs` and `crates/paladin-core/src/platform/container/herald.rs` were added first; `cargo test -p paladin-ai-core --lib allowance` stopped with 10 compile errors on the missing `herald_line`, `with_halt_reason` and `halt_reason_display`. After the implementation the window-ceiling test stated its balance as `2.5000 USD of 2.5000 USD`; the plan's literal line puts the currency once (`25.0000 of 25.0000 USD`), so the rendering strips the duplicate code. The sink test `herald_sink_folds_a_halt_reason_into_one_line` in `src/infrastructure/telemetry/herald_sink.rs` ran to completion and failed on the absent line before the fold was added; the three herald tests failed the same way before their render sites existed.
- **Task 2.** `scanner_reports_a_planted_downstream_use` failed before the scanner was implemented: with `scan` stubbed to return an empty report, the planted tree (three downstream officer-word files and one fixture-term file) gave `left: 0, right: 4` while the other two tests passed against the stub. The real scanner was restored and all three tests pass.

## Accomplishments

- **The halt line.** `HaltReason::herald_line` in `crates/paladin-core/src/platform/container/allowance.rs` renders `⛔ halted: allowance exhausted — 25.0000 of 25.0000 USD (api_key, window resets 2026-10-06T00:00:00Z)`, the `(tenant, lifetime cap)` form, and `⛔ halted: allowance could not be evaluated (ledger unavailable)`, with the same horizon wording as `AllowanceWarning::herald_line` and doctests. `HALT_REASON_METADATA_KEY`, `ExecutionMetadata::with_halt_reason` and `ExecutionMetadata::halt_reason_display` in `crates/paladin-core/src/platform/container/herald.rs` mirror the warning helpers.
- **The sink and the heralds.** `src/infrastructure/telemetry/herald_sink.rs` reads the `RunFinished.halt_reason` of the record it is handling and folds it in beside the warning fold; its module docs list the new line. `crates/paladin-herald/src/markdown_herald.rs` adds a `Halt` field, `crates/paladin-herald/src/json_herald.rs` a `halt_reason` string key and `crates/paladin-herald/src/table_herald.rs` a `Halt` row, each directly after the allowance line and each absent for a run without a reason.
- **The vocabulary guard.** `tests/treasurer_vocabulary_guard.rs` walks the repository with `std::fs` (no new crate), skips `.git`, `target`, `node_modules` and dot-directories other than `.planning` and `.project`, and reports the officer word as a whole word in `examples/`, `benches/`, `fixtures/`, any `tests/fixtures/` and `crates/*/examples|benches/`, and the downstream fixture term outside its allowlist. Both forbidden strings are assembled at run time. The two controls run the same scanner over scratch trees under `std::env::temp_dir()` that a drop guard removes.
- **Windows.** `.planning/WINDOWS.md` rows 65 (D-01 race, `src/application/services/treasurer/guard.rs`), 66 (G2, `src/application/services/paladin/paladin_execution_service.rs`; states that under option-b the true stream's `done` carries an informational `halt_reason` only when terminal usage crossed the derived figure, so it is not byte-identical in every case) and 67 (G15, `crates/paladin-battalion/src/engine/settlement.rs`), added through `gsd-tools windows append` so the table row, the JSON object and the frontmatter counters (`open_count` 8, `total_count` 67) come from one writer.
- **Registers.** Phase 42 bullets in the changelogs of core, ports, battalion, storage, web and herald and a pattern-update note in `crates/paladin-eval/CHANGELOG.md`; the root `CHANGELOG.md` gains the herald and guard paragraph inside the single Phase 42 entry and a Known limitations bullet for the three windows; two `MIGRATION.md` section 9.2 rows (`HaltReason::herald_line`, the `ExecutionMetadata` halt helpers) with Req `ALLOW-03`.
- **ADR-0057 conformance.** `.planning/decisions/0057-mid-run-halt-contract.md` Code Conformance now reads `conforms`; every test it names was located by `git grep` and the section names the file holding each one, including the option-b true-stream tests.
- **Stale doc.** The module doc of `src/application/services/treasurer/tests.rs` no longer says Phase 42 closes the over-admission race with a reservation; it records the check-only boundary and points at ADR-0057 and `.planning/WINDOWS.md` row 65, the item `.planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md` assigned to this plan.

## Task Commits

1. **Task 1: halt line in core, sink and three heralds** -- `5c346b8f` (feat)
2. **Task 2: ALLOW-05 vocabulary guard** -- `9840e7b5` (test)
3. **Task 3: windows, changelogs, migration rows, ADR conformance, test-module doc** -- `2d057c6c` (docs)

Plan metadata (this SUMMARY, STATE, ROADMAP) is committed after this file; its hashes are in the orchestrator report.

## Decisions Made

- **Currency stated once.** The plan's literal line (`25.0000 of 25.0000 USD`) governs; the balance drops its code only when it matches the ceiling's currency.
- **Store the rendered line, not the typed reason.** The metadata key holds a string, so a herald cannot leak more than the line.
- **JSON key `halt_reason` as a string.** It follows the herald's own `allowance_warning` convention and is a rendered line; the HTTP surface's structured object is a different type on a different surface.
- **G2 row worded for option-b.** Not worded as byte-identical in all cases.

## Deviations from Plan

None of Rules 1 to 4 required a design change. Departures from the letter of the plan:

1. **[Rule 1 - Bug] Duplicate currency in the first rendering.** The first implementation produced `2.5000 USD of 2.5000 USD`; the plan's expected line has the currency once. Fixed in the same task before its commit (`crates/paladin-core/src/platform/container/allowance.rs`), covered by the four `halt_reason_herald_line_*` tests.
2. **Table herald tests need a feature.** `crates/paladin-herald/src/table_herald.rs` is compiled only with the `table` feature, so the plan's `cargo test -p paladin-herald --lib` does not run the table test; it was run with `--all-features` (89 tests) as well as without (57 tests), and `cargo test --workspace` and clippy `--all-features` cover it.
3. **Closeout-directed file outside `files_modified`.** `src/application/services/treasurer/tests.rs` was edited (module doc only) because the dispatch notes and the phase's deferred items name this plan for it.
4. **Windows rows written through the tool.** The plan describes hand-editing both representations; `gsd-tools windows append` produced the identical shape (row format, JSON object, counters) from one writer, and the diff was checked against the 42-09 row's shape.
5. **TDD commit shape.** Tests and implementation are committed together per task, as in earlier Phase 42 plans; red was observed before each implementation (see Red phase evidence). The Task 2 red used a temporary stub for `scan`, restored before the commit.
6. **No public-surface baseline refresh.** `make api-surface` reports the surface unchanged (4209 items) because the baseline lists only the `paladin` facade's own items, so `.project/current-exports.txt` is untouched although the plan listed it conditionally.
7. **Commit trailer model name.** The dispatch notes asked for `Claude Fable 5.1` and the three task commits carry it. The session's attribution reminder (which replaces earlier guidance) names `Claude Sonnet 5.5`, the model that ran this plan, so the SUMMARY and tracking commits carry that name, the same split plan 42-11 recorded. The task commits were not rewritten (no history rewriting).

## Known Stubs

None. Every new function has a caller: `HaltReason::herald_line` from `ExecutionMetadata::with_halt_reason`, which `HeraldTraceSink` calls; `halt_reason_display` from the three heralds.

## Threat Flags

None beyond the plan's register. T-42-44 (halt line disclosure) is mitigated and pinned by `halt_reason_herald_line_names_no_tenant_or_key`; T-42-45 by the three ledger rows and the ADR re-read; T-42-46 by the planted-tree control, which was seen to report 0 of 4 against a stub; T-42-47 is transferred to CI and named below.

## Manual credential-handling review

Performed by reading every Phase 42 log line, error body, trace event, webhook payload and herald line against `.github/instructions/security.instructions.md`:

- **Herald line:** scope kind, two `format_cost` figures and the window end only; test-pinned to contain no tenant or key name. The metadata key stores the rendered text only.
- **Log lines** added in Phase 42 (`log::warn!` at the halt and refusal sites in `src/application/services/treasurer/guard.rs`, `log::error!` at the fail-closed and dispatch-derivation sites in `src/application/services/run/worker.rs`, the unpriced-schedule skip): they carry run id, scope kind, limit kind, tenant id and `format_cost` figures or a backend error text. None interpolates an API key name or value; a tenant id is an operator-side identifier and follows the Phase 41 refusal log's existing shape. Backend error text comes from the storage adapters' own `Backend(String)` messages, as in Phase 41.
- **Error bodies and wire objects:** `halt_reason` is `HaltReason::wire_json` (scope kind, limit kind, `format_cost` balance and ceiling, window bounds, `reason`); `422 model_unpriced` carries the registered agent's model name, taken from the agent entry and never from the request.
- **Trace events and webhooks:** `RunFinished.halt_reason` holds the same figures; the caller's `halted` webhook carries `halt_reason` and the signature unchanged; the operator `allowance_halted` delivery reuses the Phase 41 operator target and secret and is not caller-subscribable.
- **HTTP clients and redirects:** no new outbound client was added in Phase 42; the webhook client's no-redirect and SSRF guard from Phase 27 are untouched.
- **Result:** no credential leak found and nothing to remediate. `make security` and clippy `-D warnings` are green; CodeQL stays advisory-only, so this manual review is the primary control, as the project instructions state.

## Gates and tests run

- `cargo test --workspace --no-fail-fast`: exit 101 because one target, `paladin-ai --lib`, ended with 1295 passed and 2 not passing: the two known sandbox-only cases `build_run_api_persists_run_traces_when_trace_persist_is_set` and `build_run_api_persists_no_run_traces_by_default` (no outbound network; logged in `.planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md`). Every other target passed: 58 test suites, 6565 tests passed in total, 229 ignored, and all doctest groups; `cancel_tests::local_cancel_signals_token` passed in this run. `tests/treasurer_vocabulary_guard.rs` ran 3 of 3 passing.
- Targeted runs: `cargo test -p paladin-ai-core --lib allowance` (41 passed), `cargo test -p paladin-ai-core --lib herald` (31 passed), `cargo test -p paladin-ai-core --doc allowance` (20 passed) and `--doc herald` (16 passed), `cargo test -p paladin-herald --lib` (57 passed) and `--all-features` (89 passed), `cargo test -p paladin-ai --lib infrastructure::telemetry::herald_sink` (7 passed), `cargo test --test treasurer_vocabulary_guard` (3 passed).
- `cargo fmt --check`: exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0.
- `make clean-code` (fmt, lint, shell lint, check, doc-check, public API examples gate): exit 0.
- `make check-gates`: exit 0.
- `make security` (cargo-audit and cargo-deny): exit 0; cargo-deny reported advisories, bans, licenses and sources ok.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: exit 0, surface unchanged (4209 items), no baseline refresh.
- `cargo test -p paladin-web --test openapi_golden_v0_9`: exit 0 (9 passed). `cargo test -p paladin-web --lib openapi_matches_committed_baseline`: exit 0 (1 passed); `crates/paladin-web/openapi.json` is unchanged.
- `./scripts/check-changelogs.sh`: exit 0 (11 publishable crates). `./scripts/check-migration-allowlist.sh`: exit 0 (set-equal).
- ADR-0057 conformance: every test named in the section was found by `git grep` in `src`, `crates` or `tests`.
- Not run locally, named as CI-only: the PostgreSQL contract legs (PostgreSQL is not running here, so those legs self-skip and are not claimed), the Redis integration tests (no Redis here), and the 82 % workspace coverage floor (`cargo llvm-cov`, the `coverage` CI job).

## Self-Check: PASSED

- FOUND: commits `5c346b8f`, `9840e7b5` and `2d057c6c` in `git log`
- FOUND: `crates/paladin-core/src/platform/container/herald.rs` contains `pub const HALT_REASON_METADATA_KEY`, `pub fn with_halt_reason` and `pub fn halt_reason_display`
- FOUND: `crates/paladin-core/src/platform/container/allowance.rs` contains `pub fn herald_line` on `HaltReason`
- FOUND: `src/infrastructure/telemetry/herald_sink.rs` contains `with_halt_reason(`
- FOUND: `halt_reason_display()` in `crates/paladin-herald/src/markdown_herald.rs`, `crates/paladin-herald/src/json_herald.rs` and `crates/paladin-herald/src/table_herald.rs`
- FOUND: `tests/treasurer_vocabulary_guard.rs` contains `fn treasurer_is_a_framework_only_word`, `fn scanner_reports_a_planted_downstream_use` and `fn scanner_accepts_a_clean_tree`
- FOUND: `.planning/WINDOWS.md` has 4 table rows and 4 JSON objects for phase 42, `total_count` 67
- FOUND: `Cargo.toml` and `Cargo.lock` untouched by this plan
