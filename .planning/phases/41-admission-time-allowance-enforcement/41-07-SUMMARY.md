---
phase: 41-admission-time-allowance-enforcement
plan: 07
subsystem: treasurer
tags: [allowance, trace-event, herald, worker, run-scope, warn-threshold, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 01
    provides: AllowanceWarning, Admission::warnings(), the recorded design checkpoint (option-b, item 5 = C6 worker-side trace emission)
  - phase: 41-admission-time-allowance-enforcement
    plan: 04
    provides: the three agent handlers' admission binding (left as _admission for this plan)
  - phase: 41-admission-time-allowance-enforcement
    plan: 06
    provides: TreasuryNoticePort::notices_for_run, NoticeRecord, build_treasury_notices
provides:
  - TraceEvent::AllowanceWarning (13th variant) with From<AllowanceWarning> and AllowanceWarning::from_trace_event
  - AllowanceWarning::percent_of_ceiling and AllowanceWarning::herald_line (the one shared rendering)
  - ALLOWANCE_WARNING_METADATA_KEY, ExecutionMetadata::with_allowance_warnings and allowance_warning_display
  - one allowance line in the markdown, JSON and table heralds; a stateful HeraldTraceSink fold
  - RunWorkerPool::with_treasury_notices and first-dispatch readback for graph runs and agent-kind runs
  - RunScope.allowance_warnings / with_allowance_warnings; PaladinExecutionService emission and streamed final-chunk metadata
  - build_run_api opens the notice store once and shares it between the worker pool and the Treasurer
affects: [41-08, 41-09]

tech-stack:
  added: []
  patterns:
    - "A notice observes the run and never gates it: the worker's notice read failure is logged and the run proceeds"
    - "Emit from the run's own dispatcher on the Queued arm only, so seq never collides (C6)"
    - "One display helper (ExecutionMetadata::allowance_warning_display) feeds every herald; a run without a warning renders byte-identically"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/trace.rs
    - crates/paladin-core/src/platform/container/allowance.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - crates/paladin-core/src/platform/container/run_scope.rs
    - crates/paladin-herald/src/markdown_herald.rs
    - crates/paladin-herald/src/json_herald.rs
    - crates/paladin-herald/src/table_herald.rs
    - src/infrastructure/telemetry/herald_sink.rs
    - src/application/services/run/events.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - crates/paladin-web/src/agent_controller.rs
    - src/infrastructure/web/run_api_wiring.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "Followed the 41-01 checkpoint (option-b, item 5) exactly: the worker reads notices_for_run on the Queued arm only and emits one AllowanceWarning per row through the run's own dispatcher before RunStarted; the submitting process never emits"
  - "The warning is emitted through the run's per-run emitter, so on a graph run it takes seq 1 and the engine's RunStarted takes seq 2; the persisted stream stays gapless (asserted)"
  - "The notice store is opened once in build_run_api and shared by the worker pool and the Treasurer instead of a second adapter on the same URL"
  - "Emission on the HTTP path lives in the PaladinExecutorPort::execute_scoped and StreamingExecutorPort::execute_stream_scoped trait impls (as the plan said), not in the inherent execute_scoped, so no other caller of the inherent method can double-emit"
  - "The streamed warnings are folded into all three final-chunk exits (no-model-call Finish, terminal chunk, stream-ended-without-marker) through one stream_execution_metadata parameter"

patterns-established:
  - "HeraldTraceSink is stateful but per-run: the worker builds one sink per dispatch, so the recorded warnings are always the run's own and are drained at RunFinished"

requirements-completed: []

duration: ~65min
completed: 2026-10-04
status: complete
---

# Phase 41 Plan 07: Allowance trace event, herald line and RunScope warning Summary

**Each warn-threshold notice an admission won now surfaces as exactly one `TraceEvent::AllowanceWarning` on the admitted run's own trace stream (emitted by the worker on first dispatch, before `RunStarted`, graph and agent-kind runs alike) and as one `allowance:` line in the markdown, JSON and table heralds through a single shared helper; on the HTTP agent routes the warning rides `RunScope.allowance_warnings`, is emitted once by `PaladinExecutionService` and lands on the streamed final chunk's `ExecutionMetadata`.**

## Performance

- **Duration:** ~65 min (warm workspace, no cold build)
- **Completed:** 2026-10-04
- **Tasks:** 2 (both `tdd="true"`)
- **Files:** 17 modified, none created

## Accomplishments

- **Event and renderings (Task 1).** `TraceEvent::AllowanceWarning { scope_kind, limit_kind, balance, ceiling, window_start, window_end, warn_at }` as the thirteenth variant (`window_*` serde-defaulted), `From<AllowanceWarning> for TraceEvent`, `AllowanceWarning::from_trace_event`, `percent_of_ceiling` (i128 floor, `0` for a non-positive ceiling or negative balance) and `herald_line` (`⚠ allowance: 82% of 25.0000 USD (api_key, window resets 2026-10-03T00:00:00Z)` or `... (tenant, lifetime cap)`; names no tenant id or key name). `ExecutionMetadata::with_allowance_warnings` / `allowance_warning_display` and `ALLOWANCE_WARNING_METADATA_KEY` beside `cost_display`; markdown (`Allowance` field after `Cost`), table (`Allowance` row) and JSON (`allowance_warning` key, only when present) call the one helper. `HeraldTraceSink` records warnings in a poison-tolerant `Mutex<Vec<_>>` and drains them into the `RunFinished` metadata. The SSE bridge maps the new variant to no wire event (explicit arm in `events.rs`), so the seven wire events are unchanged.
- **Worker readback (Task 2).** `RunWorkerPool::with_treasury_notices`; one private helper `emit_allowance_warnings` called from `run_once` (after the per-run emitter exists and the bus is bound, before dispatch, `first_dispatch && emitter`) and from `run_agent` (immediately before its own `RunStarted`, guarded by the pre-update row's `Queued` status). A read error is `log::warn!`ed with the run id and error only. `run_agent`'s `RunScope` deliberately does not carry the warnings.
- **HTTP agent path.** `RunScope.allowance_warnings` (serde-defaulted, skipped when empty) and `with_allowance_warnings`; the three agent handlers thread `admission.warnings()` into the scope (the `_admission` underscore from 41-04 is gone); `PaladinExecutionService` emits each warning once through `self.trace_emitter` or the ambient emitter (a no-op with neither) and folds them into the streamed final chunk's metadata on all three exit paths.
- **Wiring.** `build_run_api` opens `build_treasury_notices` once whenever a run store is configured and attaches it to the pool; the Treasurer reuses the same handle.
- **Registers.** MIGRATION.md 9.2 rows for `TraceEvent` (N/A, new-in-0.10), `RunScope` (N) and `ExecutionMetadata` (N), a 9.4 rollout note (older readers meet an unknown `run_traces` kind only for runs carrying a notice; upgrade every replica before setting `warn_at`), a CHANGELOG paragraph, and the public-API baseline (4159 to 4161 items).

## Task Commits

1. **Task 1: the event, the shared herald line, three heralds, stateful HeraldTraceSink** - `c5c573e` (feat)
2. **Task 2: worker readback, RunScope and execution-service path, wiring, registers** - `01ccec4` (feat)

## TDD / red evidence

- **Task 1:** honest note, the pure rendering functions, the herald branches and the sink fold were written together with their tests and passed on first run; one earlier run failed four herald tests because a test helper built `ExecutionMetadata` without its required builder fields (a test error fixed before commit). There was no separate red step for the pure functions.
- **Task 2, worker:** mutation evidence. Replacing the first-dispatch guard with `(first_dispatch || true)` failed `running_redelivery_does_not_reemit_the_allowance_warning` and `awaiting_input_resume_does_not_reemit_the_allowance_warning` (`2 failed; 4 passed`); the mutation was reverted. The emit-path, agent-path, no-store and read-failure tests were written with the implementation.
- **Task 2, execution service and controller:** written alongside the code; the controller and service tests assert the scope warnings, the single emission, the streamed metadata line and the no-warning no-op.

## Verification

- `cargo test -p paladin-ai-core --lib` 675 passed (incl. `all_thirteen_event_variants_construct`, `allowance_warning_event_round_trips_with_kind_allowance_warning`, floor, line-shape, join and absence tests, `run_scope_allowance_warnings_default_empty_omitted_and_round_trip`); `--doc` 126 passed incl. the new doctests; `cargo test -p paladin-herald --all-features` 86 passed.
- `cargo test -p paladin-ai --lib infrastructure::telemetry::herald_sink` 6 passed (incl. `herald_sink_folds_allowance_warnings_into_run_finished_metadata`, `herald_sink_without_warnings_renders_as_before`).
- `cargo test -p paladin-ai --lib application::services::run` 198 passed incl. the six `allowance_warnings` worker tests (`first_dispatch_emits_one_allowance_warning_before_run_started` also asserts a gapless persisted `seq` and no tenant id or key name in the persisted row, `agent_run_first_dispatch_emits_the_allowance_warning_once` asserts seq 1 and an empty scope); `--lib application::services::paladin::paladin_execution_service` 80 passed; `cargo test -p paladin-web --lib agent_controller` 56 passed (four new); `--features web-server --lib infrastructure::web::run_api_wiring` 19 passed.
- `cargo test --workspace` (default features): **6317 passed, 0 failed** (6278 after 41-06).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p paladin-ai-core -p paladin-ai`: clean.
- `./scripts/check-migration-allowlist.sh` exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exits 0 after `make api-surface-update`.
- Acceptance greps: `AllowanceWarning {` in `TraceEvent`, `assert_eq!(events.len(), 13)` and `=> "allowance_warning"` in `trace.rs`; `allowance_warning_display` in all three heralds; the metadata-key constant in `herald.rs`; `awk` over `herald_line` prints 0 for `tenant_id|api_key_id`; `pub fn with_treasury_notices`, `notices_for_run(` and `matches!(run.status, RunStatus::Queued)` in `worker.rs`; `pub allowance_warnings: Vec<AllowanceWarning>` in `run_scope.rs`; the pre-`#[cfg(test)]` part of `agent_controller.rs` has exactly 3 `with_allowance_warnings`; `with_treasury_notices` in `run_api_wiring.rs` and `.project/current-exports.txt`.
- Not run: `make security` (cargo-audit and cargo-deny): no dependency changed in this plan. No PostgreSQL leg was touched.
- Manual credential-handling review: the event and `herald_line` carry figures, window and threshold only (no tenant id, key name or key value; a test asserts the persisted row contains neither); the worker's read-failure log line carries the run id and the adapter error only; no HTTP client or redirect behaviour changed; the notice SQL is untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in own test] Test helper missing required builder fields**
- **Found during:** Task 1 verification
- **Issue:** the first `ExecutionMetadata` helper (and two doctests) set only `model_used`; the builder also requires `execution_id`, `start_time` and `token_usage`, so four herald tests failed.
- **Fix:** helpers and doctests now set all four. **Commit:** `c5c573e`

**2. [Rule 3 - Blocking] Exhaustive `TraceEvent` match sites**
- **Found during:** Task 1
- **Issue:** adding a variant needed a decision at every `match` over `TraceEvent`; all downstream sites already carry wildcard arms, so none broke. The SSE bridge (`map_trace_event`) gained an explicit `AllowanceWarning => None` arm so the "no wire event" intent is visible rather than implicit. **Files:** `src/application/services/run/events.rs`. **Commit:** `c5c573e`

**3. [Note - scope] Notice store opened once, shared with the Treasurer**
- The plan said to attach `build_treasury_notices(..)` to the pool; the Treasurer branch already opened its own copy. `build_run_api` now opens it once before the pool is assembled and the Treasurer reuses the same handle (one adapter instead of two on the same URL). Behaviour is unchanged; the existing `wired_treasurer_records_a_warn_notice_for_the_admitted_run` test still passes.

**4. [Note - tooling] A working-tree checkout nearly dropped uncommitted worker changes**
- While proving the first-dispatch guard by mutation I restored the mutated file with `git checkout`, which also discarded my uncommitted `worker.rs` edits. I had taken a copy immediately before the mutation, restored from it, and re-ran the worker tests green; nothing was lost and nothing outside `worker.rs` was affected.

**5. [Note - attribution] Commit trailers**
- Both commits carry `Co-Authored-By: Claude Fable 5.1` plus the session line, as the dispatch note instructs (earlier plans used the session reminder's model name; the orchestrator may amend if repo policy differs).

**Total deviations:** 1 own-test bug, 1 blocking-decision note, 3 notes; no scope change.

## Authentication Gates

None.

## Known Stubs

None. `Treasurer::confirm` is intentionally still the no-op 41-06 left it: the operator webhook delivery lands in 41-08, as the 41-03, 41-04 and 41-06 docs already state.

## Threat Flags

None beyond the plan's register. T-41-34 (disclosure through shared sinks) is mitigated by the variant and `herald_line` carrying figures only, the `awk` proof, and the persisted-row assertion. T-41-35 (seq collision) by emitting only from the run's own dispatcher on the `Queued` arm, with the redelivery and resume tests and the gapless-seq assertion. T-41-36 (a notice read failing a run) by `notice_read_failure_never_fails_the_run`. T-41-37 stays accepted and is documented in the MIGRATION 9.4 rollout note; the upgrade-every-replica guidance for the webhook lands in 41-08.

## Notes for later plans

- `requirements-completed` is intentionally empty (and ALLOW-04 left unchecked in REQUIREMENTS.md): ALLOW-04 also needs the operator webhook, which lands in 41-08; the trace event and herald legs it names are done here.
- 41-08 attaches the operator webhook in `Treasurer::confirm`; the `AllowanceNotice` values it receives already carry `tenant_id`, `api_key_id` (names only) and `run_id`. The trace event and herald line deliberately do not carry those names, only the webhook payload does (D-17 amended).
- The `HeraldTraceSink` is only correct while the worker keeps building one sink per run dispatch; do not share one instance across runs.
- `make api-surface` also records `HeraldTraceSink` flipping from `Freeze` to `!Freeze` (the new `Mutex` field); it is a nightly auto-trait artifact, not a stable-API change.

## Self-Check: PASSED

Commits `c5c573e` and `01ccec4` are present in `git log`; all 17 modified files are tracked in those commits; every acceptance grep above returned the expected value.
