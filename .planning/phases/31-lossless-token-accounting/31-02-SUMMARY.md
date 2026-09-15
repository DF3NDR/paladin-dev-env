---
phase: 31-lossless-token-accounting
plan: 02
subsystem: core-domain
tags: [rust, serde, token-usage, battalion, trace, herald, tdd]

# Dependency graph
requires:
  - phase: 31-lossless-token-accounting
    provides: "TokenUsage six-field shape (cache/reasoning sub-counts) with saturating Add/AddAssign/Sum, from plan 31-01"
provides:
  - "PaladinResult.usage / NodeExecutionRecord.usage / TraceEvent::NodeFinished.usage / TraceEvent::RunFinished.usage — the full TokenUsage carrier chain, replacing every retired bare-count field"
  - "TraceDispatcher's Mutex<TokenUsage> accumulator (total_usage()), replacing the AtomicU64 token_total that caused the D-30 zeroing bug"
  - "Formation/Phalanx real per-Paladin usage in per_paladin_tokens (D-10), replacing the from_total zeroing bug at the battalion aggregation layer"
  - "RecordingPaladinPort::set_output_with_usage — a TokenUsage-native test seam"
  - "D-25 legacy-JSON contract tests on PaladinResult and NodeExecutionRecord (pre-phase document with the retired key deserialises to TokenUsage::default(), never a reconstructed total)"
  - "D-30 round-trip, cache-hit, and zero-Paladin-node boundary tests on the engine"
  - "TokenUsage::from_total deleted outright (no #[deprecated] replacement) — every in-tree caller migrated to TokenUsage::new"
affects: [31-03, 31-04, 31-05, 31-06, 31-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Mutex<TokenUsage> accumulator recovered with PoisonError::into_inner (never .unwrap()/.expect()) for a multi-field saturating accumulator that can't use a single atomic"
    - "Compiler-driven workspace migration: cargo check --workspace --all-targets --all-features --keep-going to surface every remaining site in one pass rather than one target at a time"
    - "Read-only retarget vs. real semantics: heralds/CLI/telemetry sinks keep their own bare-count shape and just read through usage.total_tokens (DTOs change in 31-06); the execution service's reasoning loop and Formation/Phalanx get the real accumulation fix (D-12/D-10)"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/execution_result.rs
    - crates/paladin-core/src/platform/container/waypoint.rs
    - crates/paladin-core/src/platform/container/trace.rs
    - crates/paladin-core/src/platform/container/token_usage.rs
    - crates/paladin-core/src/platform/container/battalion/mod.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/src/engine/export/overlay.rs
    - crates/paladin-battalion/src/formation_service.rs
    - crates/paladin-battalion/src/phalanx_service.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - (75 further files across crates/, src/, tests/, examples/ — full list in Task Commits below)

key-decisions:
  - "The D-25 legacy-JSON test on PaladinResult and its NodeExecutionRecord mirror, plus the all-six-keys serialization test, were written directly in Task 1's final form (rather than a throwaway intermediate) since Task 1's own rename already required rewriting them — Task 3 confirms/reuses them instead of duplicating"
  - "Herald/CLI/telemetry/inspector read-sites keep their own field shapes this plan (still u32/u64 bare counts) and only retarget the read expression to usage.total_tokens — their own DTO reshape is explicitly plan 31-06's job (D-24), not this plan's"
  - "TraceDispatcher's accumulator uses std::sync::Mutex<TokenUsage> rather than five separate AtomicU32s, recovered via PoisonError::into_inner — simpler to reason about for a multi-field saturating accumulator, and the plan's own text names this as the preferred alternative"
  - "Test literal migrations preserve each fixture's original total by construction (TokenUsage::new(total, 0)) rather than inventing a plausible prompt/completion split — mechanically correct and avoids introducing arbitrary numbers with no basis in the original fixture"

patterns-established:
  - "A cache-hit or non-Paladin dispatch always passes TokenUsage::default() through the (paladin_id, usage, outcome) tuple and every downstream NodeExecutionRecord/NodeFinished construction — never a bespoke zero literal"
  - "PaladinResult::new and TokenUsage::new call sites in tests/examples/docs use TokenUsage::new(total, 0) as the canonical single-total literal shape"

requirements-completed: [ACCT-02]

coverage:
  - id: D1
    description: "PaladinResult.usage, NodeExecutionRecord.usage, TraceEvent::NodeFinished.usage and TraceEvent::RunFinished.usage all carry a full TokenUsage in place of their former bare counts; BattalionResult.per_paladin_tokens values are the Paladin's real split"
    requirement: "ACCT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::d30_round_trip_sums_full_usage_across_two_paladin_nodes"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/formation_service.rs#tests::test_formation_aggregates_per_paladin_times_and_tokens"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/phalanx_service.rs#tests::test_phalanx_per_paladin_tokens"
        status: pass
    human_judgment: false
  - id: D2
    description: "A cache-hit node records TokenUsage::default() on NodeExecutionRecord and NodeFinished and contributes nothing to RunFinished.usage; a zero-Paladin-node run finishes with RunFinished.usage == TokenUsage::default()"
    requirement: "ACCT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::cache_hit_paladin_node_records_default_usage_and_contributes_nothing"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::zero_paladin_node_run_finishes_with_default_usage"
        status: pass
    human_judgment: false
  - id: D3
    description: "RunFinished.usage is the saturating TokenUsage sum over every NodeFinished.usage the TraceDispatcher saw this run, exact the instant TraceDispatcher::emit returns"
    requirement: "ACCT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::d30_round_trip_sums_full_usage_across_two_paladin_nodes (field-by-field RunFinished assertion including both optionals)"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::node_finished_carries_real_outcome_and_cost (trace/Waypoint usage agreement)"
        status: pass
    human_judgment: false
  - id: D4
    description: "TokenUsage::from_total no longer exists anywhere in the tree and no #[deprecated] bare-count accessor replaced it; Formation and Phalanx insert result.usage.clone() into per_paladin_tokens and add u64::from(result.usage.total_tokens) to BattalionResult.total_tokens"
    requirement: "ACCT-02"
    verification:
      - kind: other
        ref: "grep -rn 'from_total' --include=*.rs crates src tests examples benches (0 matches)"
        status: pass
      - kind: other
        ref: "grep -rn '#[deprecated' crates/paladin-core/src/platform/container/token_usage.rs crates/paladin-core/src/platform/container/execution_result.rs (0 matches)"
        status: pass
    human_judgment: false
  - id: D5
    description: "A PaladinResult or NodeExecutionRecord JSON document written before this phase deserialises with usage == TokenUsage::default() — no legacy-shape deserializer maps the retired key into usage.total_tokens"
    requirement: "ACCT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/execution_result.rs#tests::legacy_json_deserialises_with_served_by_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#tests::legacy_json_deserialises_with_default_usage"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/paladin_port.rs#tests::test_paladin_result_deserialization_backward_compatibility"
        status: pass
    human_judgment: false
  - id: D6
    description: "Full workspace compiles, tests, formats and lints clean on the new carrier chain (cargo check/test/fmt/clippy --workspace --all-targets --all-features, plus make clean-code)"
    requirement: "ACCT-02"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features --keep-going (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict logged in deferred-items.md by plan 31-01)"
        status: pass
      - kind: other
        ref: "cargo fmt --all -- --check (exit 0)"
        status: pass
      - kind: other
        ref: "cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: other
        ref: "make clean-code (fmt + clippy + shellcheck + check, exit 0)"
        status: pass
    human_judgment: false

duration: ~2h30m
completed: 2026-09-15
status: complete
---

# Phase 31 Plan 02: Full TokenUsage Carrier Chain (PaladinResult → RunFinished) Summary

**`PaladinResult`, `NodeExecutionRecord`, `TraceEvent::NodeFinished` and `TraceEvent::RunFinished` all carry a full `TokenUsage` split instead of a bare count, with `TokenUsage::from_total` deleted outright and the zeroing bug fixed at Formation/Phalanx aggregation — proven by a D-30 round-trip test with non-zero cache/reasoning figures surviving the whole chain intact.**

## Performance

- **Duration:** ~2h30m
- **Tasks:** 3 (one `tracer` task with TDD, two `auto` tasks)
- **Files modified:** 91 across 3 commits

## Accomplishments

- **Task 1 (tracer, TDD) — the carrier chain itself.** `PaladinResult.usage: TokenUsage` (replacing `token_count: u32`), `NodeExecutionRecord.usage: TokenUsage` with `#[serde(default)]` (replacing `token_count: u64`), and `TraceEvent::NodeFinished`/`RunFinished` both gain `usage: TokenUsage` (replacing `token_count`/`total_tokens: u64`). `superstep.rs`'s per-node dispatch tuple carries a `TokenUsage` through both the structured-executor and plain-`PaladinPort` Paladin arms and every downstream construction site (cache hit, retry, muster, skip, interrupt, failed) — a cache hit or non-Paladin dispatch always passes `TokenUsage::default()`. `TraceDispatcher`'s `token_total: AtomicU64` becomes a `Mutex<TokenUsage>` accumulator recovered via `PoisonError::into_inner`, accumulated through `TokenUsage::AddAssign` inside `emit` so `RunFinished.usage` is exact the instant `emit` returns. Formation and Phalanx insert the Paladin's real `usage.clone()` into `per_paladin_tokens` and sum real totals into `BattalionResult.total_tokens`, fixing the `from_total` zeroing bug at its two production call sites. New tests: a D-30 round-trip test summing two Paladins' distinct non-round `TokenUsage` values (including cache/reasoning optionals) through `NodeExecutionRecord` → `NodeFinished` → summed `RunFinished`; a cache-hit boundary test; a zero-Paladin-node boundary test; and the D-25 legacy-JSON contract tests on both `PaladinResult` and `NodeExecutionRecord`.
- **Task 2 (compiler-driven) — the rest of the workspace.** `paladin_execution_service.rs`'s reasoning loop now accumulates `usage += response.usage.clone()` (via `TokenUsage::AddAssign`, D-06) with `middleware_cx.cumulative_tokens = usage.total_tokens`, so `TokenBudget` enforcement is semantically unchanged while the accumulation itself is real (D-12); the vision site maps `VisionTokenUsage`'s own prompt/completion fields explicitly (D-20, `VisionTokenUsage` itself untouched). `paladin-eval`'s `assertion.rs`/`runner.rs` retarget onto the new fields without changing any snapshot's verdict. Every remaining read-site (heralds including `table_herald.rs`'s exact `(execution_time_ms, usage.total_tokens)` name-matching pool key, CLI formatters/commands, `paladin-web`'s `ExecuteResponse::from`, `run/inspector.rs`'s `CompletedRow` mapping, telemetry sinks) reads `usage.total_tokens` while keeping its own DTO shape unchanged (31-06's job). Every `PaladinResult`/`NodeExecutionRecord`/`TraceEvent` literal across `examples/`, `tests/unit/`, `tests/integration/`, `tests/cli/` and `doc-examples` migrated to a real `TokenUsage` preserving each fixture's original total.
- **Task 3 — delete the total-only constructor.** `TokenUsage::from_total` deleted outright from `token_usage.rs` along with its documenting test; no `#[deprecated]` replacement added. The four remaining call sites (all test fixtures) — `battalion/mod.rs`'s two, and one each in `json_herald.rs`/`markdown_herald.rs`/`table_herald.rs` — migrated to `TokenUsage::new(total, 0)`, preserving every existing total assertion. `partial_eq_compares_all_three_fields_not_only_total` now compares two same-total, different-split values instead of using the deleted constructor.
- Full workspace: `cargo check/test/fmt/clippy --workspace --all-targets --all-features` and `make clean-code` all green, except the pre-existing, unrelated `cli_isolation`/`--all-features` conflict plan 31-01 already logged in `deferred-items.md`.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer, TDD): carrier chain end to end** - `d48ed77c` (feat) — includes Rule 3 auto-fixes for `herald.rs`, `paladin_port.rs`, `trace_sink_port.rs` and `contract_tests.rs` test literals blocking same-crate/workspace compile
2. **Task 2: compiler-driven workspace migration** - `20276641` (refactor)
3. **Task 3: delete `TokenUsage::from_total`, migrate remaining call sites** - `71caaac7` (refactor)

**Plan metadata:** this SUMMARY.md commit (docs, worktree mode — orchestrator handles the final metadata commit after merge)

## Files Created/Modified

**Task 1 (23 files):**
- `crates/paladin-core/src/platform/container/{execution_result,waypoint,trace,herald}.rs` — the four carrier types plus one Rule-3 same-crate fix
- `crates/paladin-battalion/src/{engine/{superstep,hooks,mod,test_support,export/overlay},formation_service,phalanx_service,{campaign,chain_of_command,conclave_execution,council,grove}_service,commander,maneuver/service}.rs`, `benches/battalion_benchmarks.rs`, `tests/export_golden.rs`
- `crates/paladin-ports/src/output/{paladin_port,trace_sink_port}.rs`
- `crates/paladin-storage/src/waypoint/contract_tests.rs`

**Task 2 (66 files; see `git show 20276641 --stat` for the exhaustive list):**
- `src/application/services/paladin/paladin_execution_service.rs` (real D-12 semantics), `paladin-eval`'s `assertion.rs`/`runner.rs`/`assertion_snapshots.rs`
- Heralds (`json_herald.rs`, `markdown_herald.rs`, `table_herald.rs`), `paladin-web`'s `agent_controller.rs`/`agent_registry.rs`, `paladin-llm`'s `fallback.rs`
- CLI (`cli/commands/{agent,battalion,eval}.rs`, `cli/formatters/output.rs`), `application/services/run/{events,inspector,stream_tests}.rs`, `orchestration/processors/paladin_processor.rs`, `paladin/{handoff_service,middleware/limits}.rs`
- `infrastructure/telemetry/{otel_sink,persisting_sink}.rs`
- 29 files under `examples/`, 21 under `tests/` (unit, integration, cli), `crates/doc-examples/src/{bridge,support}.rs`

**Task 3 (5 files):**
- `crates/paladin-core/src/platform/container/{token_usage,battalion/mod}.rs`, `crates/paladin-herald/src/{json_herald,markdown_herald,table_herald}.rs`

## Decisions Made

- Wrote the D-25 legacy-JSON tests (both `PaladinResult` and `NodeExecutionRecord`) and the all-six-keys serialization test in Task 1's final form directly, since Task 1's own field rename already required rewriting the pre-existing `served_by` legacy-JSON test — Task 3 confirmed and reused them rather than duplicating.
- Kept every herald/CLI/telemetry/inspector DTO's own field shape unchanged this plan (still `u32`/`u64` bare counts on the wire), retargeting only the read expression to `usage.total_tokens` — the DTO reshape itself is explicitly plan 31-06's job (D-24), not this plan's.
- `TraceDispatcher`'s accumulator uses `std::sync::Mutex<TokenUsage>` rather than five separate atomics, recovered via `PoisonError::into_inner` (never `.unwrap()`/`.expect()`, per CLAUDE.md) — simpler to reason about for a multi-field saturating accumulator, and the plan's own text named this as the preferred alternative.
- Test-literal migrations always preserve the original fixture's total via `TokenUsage::new(total, 0)` rather than inventing a plausible prompt/completion split — mechanically correct, and avoids introducing arbitrary numbers with no basis in the original fixture.
- Used `cargo check --workspace --all-targets --all-features --keep-going` (rather than the plan's plain `cargo check`) partway through Task 2 to surface every remaining compile site across every example/test target in one pass instead of one target at a time — purely a within-task efficiency choice, not a deviation from the plan's own compiler-driven instruction.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Fixed same-crate/same-workspace test literals blocking Task 1's own verify command**
- **Found during:** Task 1
- **Issue:** `herald.rs` (same crate as the retyped `PaladinResult`), and `paladin-ports`' `paladin_port.rs`/`trace_sink_port.rs` (dependents of the retyped core types) each had `PaladinResult`/`TraceEvent` literals using the retired `token_count`/`total_tokens` field names, which no longer compile once those fields are renamed to `usage`. `paladin-storage`'s `contract_tests.rs` had the same issue for `NodeExecutionRecord`. Task 1's own verify commands (`cargo test -p paladin-ai-core --lib platform::container`, `cargo test -p paladin-battalion --lib engine`) require these crates/dependents to compile.
- **Fix:** Migrated each literal to the new `usage: TokenUsage` field, and (for `paladin_port.rs`) rewrote the existing `test_paladin_result_deserialization_backward_compatibility` test to assert the D-25 contract (`usage == TokenUsage::default()` on a legacy document) rather than the old total-preserving assertion, since that test's own JSON fixture carries the exact retired key this phase retires.
- **Files modified:** `crates/paladin-core/src/platform/container/herald.rs`, `crates/paladin-ports/src/output/{paladin_port,trace_sink_port}.rs`, `crates/paladin-storage/src/waypoint/contract_tests.rs`
- **Verification:** `cargo test -p paladin-ai-core --lib platform::container` and `cargo test -p paladin-battalion --lib engine` both pass; `cargo check -p paladin-ports --all-targets --all-features` exits 0
- **Committed in:** `d48ed77c` (Task 1 commit)

**2. [Rule 1 - Bug] Reverted two accidental false-positive substitutions from bulk scripted fixes**
- **Found during:** Task 2
- **Issue:** A regex-driven bulk fix pass (matching `.token_count` field-access patterns to retarget onto `.usage.total_tokens`) incorrectly matched two unrelated occurrences in `tests/unit/herald_consolidation_test.rs`: `BattalionResult.total_tokens` (a legitimate bare `u64` that stays per D-08, not part of the four-carrier migration) and `StreamChunk.token_count` (an unrelated type never touched by this phase).
- **Fix:** Reverted both lines to their original field names (`total_tokens: 0,` and `chunk.token_count`/`deserialized.token_count`) before re-running `cargo check`.
- **Files modified:** `tests/unit/herald_consolidation_test.rs`
- **Verification:** `cargo check --workspace --all-targets --all-features --keep-going` exits 0; the two reverted assertions still exercise their original, correct fields
- **Committed in:** `20276641` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 — blocking compilation issues directly caused by this plan's own field renames; 1 Rule 1 — self-caught bug in my own scripted bulk-fix pass, corrected before the compile-check that would have caught it anyway).
**Impact on plan:** Both were necessary to keep the workspace compiling/testing/linting clean on the new four-carrier shape, exactly as the plan's own acceptance criteria require. No scope creep — no carrier type outside the four named in this plan (`PaladinResult`, `NodeExecutionRecord`, `TraceEvent::NodeFinished`/`RunFinished`) was touched, and `BattalionResult.total_tokens`/`StreamChunk.token_count` were correctly left alone after the Rule 1 catch.

## Issues Encountered

**Pre-existing, unrelated test/feature-flag conflict (not an issue with this plan's changes), already logged by plan 31-01:**
`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under `cargo test --workspace --all-features` because the test asserts the `cli` feature is NOT active, while `--all-features` (this plan's own verify command) necessarily activates it. Confirmed pre-existing and already documented in `.planning/phases/31-lossless-token-accounting/deferred-items.md`; left unfixed as out of scope for this plan too.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

The full `TokenUsage` carrier chain (`PaladinResult` → `NodeExecutionRecord`/`NodeFinished` → `RunFinished`) is now load-bearing infrastructure every later plan in this phase builds on. `TokenUsage::from_total` is gone from the tree entirely, so plan 31-03 (streaming usage parity, D-13..D-19) and plan 31-06 (the HTTP/DTO reshape, D-24) can proceed without any remaining total-only construction path to work around. The heralds/CLI/telemetry sinks whose own DTO shape stayed a bare count this plan (by design, D-24 deferred to 31-06) are the explicit target list for that later plan. No blockers.

## Self-Check: PASSED

- FOUND: `crates/paladin-core/src/platform/container/execution_result.rs`
- FOUND: `crates/paladin-core/src/platform/container/waypoint.rs`
- FOUND: `crates/paladin-core/src/platform/container/trace.rs`
- FOUND: `crates/paladin-core/src/platform/container/token_usage.rs`
- FOUND: `crates/paladin-battalion/src/engine/hooks.rs`
- FOUND: `.planning/phases/31-lossless-token-accounting/31-02-SUMMARY.md`
- FOUND commit `d48ed77c` (feat: Task 1 carrier chain)
- FOUND commit `20276641` (refactor: Task 2 workspace migration)
- FOUND commit `71caaac7` (refactor: Task 3 delete from_total)

---
*Phase: 31-lossless-token-accounting*
*Completed: 2026-09-15*
