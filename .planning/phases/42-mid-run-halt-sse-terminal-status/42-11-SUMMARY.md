---
phase: 42-mid-run-halt-sse-terminal-status
plan: 11
subsystem: treasurer-mid-run-operator-notices
tags: [allowance, mid-run-warn, halt-notice, spend-guard, operator-webhook, allowance-halted, trace-emitter, once-per-window]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: NoticeKind, kind on NoticeRecord and AllowanceNotice, the rebuilt once-per-window index, RunEventKind::AllowanceHalted with its operator signing branch and the crate-private operator_event_for mapping (42-10); the shared Treasurer::evaluate and the admit_inner claim ordering (42-07, 42-09); TreasurerSpendGuard with its memoised first halt and the worker's per-run guard attachment (42-02, 42-04); the run's own trace emitter and the TraceEvent::AllowanceWarning precedent (42-05, Phase 41)
  - phase: 41-admission-time-allowance-enforcement
    provides: the once-per-window notice store, the operator webhook queue and its signed delivery service, the warn-path tracer and its operator receiver
provides:
  - a mid-run warn_at crossing detected by the boundary guard, claimed through the Phase 41 notice store, emitted once on the run's own trace emitter and queued once as an operator allowance_warning delivery
  - a spend halt that claims a halt notice and queues one operator allowance_halted delivery per scope, limit, window and ceiling, with the warning's twelve keys
  - no operator notice for a ledger_unavailable halt (logged at error only)
  - a per-run claim memo in the guard (at most one claim write per ceiling, window and notice kind per run) that never changes the guard's decision
  - crate-private Treasurer::spend_guard_with_emitter, Treasurer::claim_halt_notice and Treasurer::notify_operator; Treasurer::spend_guard keeps its 42-02 signature
  - the worker attaching the guard with the run's own trace emitter
  - MIGRATION.md section 9.6 paragraph with the allowance_halted rollout caveat, platform-api.md operator-notice update, CHANGELOG paragraph
affects: [42-12]

tech-stack:
  added: []
  patterns:
    - "One crossing rule and one claim step shared by admission, the mid-run warning and the halt notice (collect_crossings, claim_record), so the three cannot drift"
    - "A guard-side in-run memo keyed like the store's own dedup identity skips repeat writes while the store stays the dedup truth"
    - "Interior-mutability fields on a published public struct are held behind an Arc so its auto traits (Freeze) and the public API surface do not change"

key-files:
  created: []
  modified:
    - src/application/services/treasurer/evaluate.rs
    - src/application/services/treasurer/guard.rs
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/derive.rs
    - src/application/services/treasurer/tests.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/http_surface_tests.rs
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "Evaluation.exhausted carries a small Exhausted struct (the Ceiling and its AllowanceRefusal) rather than a tuple, so the halt notice is keyed on the ceiling's identity and configured warn_at"
  - "The guard's claim memo is keyed on (scope kind, tenant, key name, limit kind, window start, ceiling nanos, notice kind), mirroring the store's identity, and covers the halt claim too, so concurrent child-run boundaries on a shared guard claim a halt at most once"
  - "The worker builds the per-run guard AFTER the run's trace emitter exists and passes it that emitter; a run with no trace sink still claims and queues its notices and only skips the stream event"
  - "The memo is Arc<Mutex<HashSet<ClaimKey>>>, not an inline Mutex, because an inline Mutex made the published TreasurerSpendGuard lose the Freeze auto trait and make api-surface reported drift"
  - "A halt notice emits no trace event: the halt is already on the stream as RunFinished with halt_reason, and a halt is not an AllowanceWarning"

patterns-established:
  - "Operator-visible allowance signals travel one channel only: the Phase 41 notice store plus the signed durable webhook queue"

requirements-completed: [ALLOW-03, ALLOW-05]

coverage:
  - id: D1
    description: "A run admitted below warn_at that crosses it mid-run claims exactly one warning notice at its first boundary past the threshold, emits exactly one AllowanceWarning on an attached emitter and queues exactly one allowance_warning delivery; later boundaries write nothing and emit nothing"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#guard_claims_a_mid_run_warning_once (red before the guard claimed; the module did not compile without the constructor)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A lost warning claim emits nothing and queues nothing, and a warning the admission already claimed is not claimed again mid-run"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#a_lost_warning_claim_emits_and_enqueues_nothing, #a_warning_the_admission_already_claimed_is_not_claimed_again_mid_run"
        status: pass
    human_judgment: false
  - id: D3
    description: "A spend halt claims a halt-kind notice carrying the exhausted figures and the ceiling's configured warn_at, and queues one allowance_halted delivery with the warning's twelve keys, event set to allowance_halted and run_id the halted run; later boundaries of the sticky halt claim nothing"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#guard_halt_claims_one_halt_notice_and_one_operator_delivery"
        status: pass
    human_judgment: false
  - id: D4
    description: "Edge (concurrency across runs): three guards of one scope halting in one window try one halt claim each, the store answers once, and exactly one operator allowance_halted delivery is queued (D-18, T-42-38)"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#three_guards_halting_in_one_window_enqueue_one_allowance_halted_delivery"
        status: pass
    human_judgment: false
  - id: D5
    description: "Edge (ledger outage): a ledger_unavailable halt claims no notice and queues nothing"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#ledger_unavailable_halt_claims_no_notice_and_enqueues_nothing"
        status: pass
    human_judgment: false
  - id: D6
    description: "A guard built through the published spend_guard (no emitter) still claims and queues but emits nothing; a store error never changes the continue or halt decision; a Treasurer with no notice store claims nothing; every pre-existing guard test passes with no edit to its spend_guard call sites"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#a_guard_built_without_an_emitter_still_claims_and_notifies_but_emits_nothing, #a_notice_store_failure_never_changes_the_guard_decision, #a_guard_without_a_notice_store_neither_claims_nor_notifies, #guard_memoises_its_first_halt"
        status: pass
    human_judgment: false
  - id: D7
    description: "End to end through the real router, worker, SQLite ledger, notices and deliveries and a signed operator receiver: run A has exactly one allowance_warning on its own trace stream, the receiver gets one allowance_warning and one allowance_halted POST each with a valid X-Paladin-Signature over the received bytes and twelve keys, and run B halting in the same window adds no POST"
    requirement: "ALLOW-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#mid_run_warn_and_halt_notices_reach_the_operator_once (passed five consecutive times; red before the worker passed the run's emitter, with the halt correct and the stream warning absent)"
        status: pass
    human_judgment: false
  - id: D8
    description: "The Phase 41 warn path and the 42-02 halt tracer are unchanged"
    requirement: "ALLOW-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#allowance_warn_path_tracer and #engine_spend_halt_tracer (1 passed each)"
        status: pass
    human_judgment: false
  - id: D9
    description: "The notices are registered and documented with no public-surface drift: MIGRATION.md section 9.6 with the rollout caveat, platform-api.md, CHANGELOG, allowlist set-equality, and an unchanged public API surface (4209 items)"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh exit 0; PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface reports the surface unchanged (no baseline refresh)"
        status: pass
    human_judgment: false
  - id: D10
    description: "ALLOW-05 is carried unchanged: nothing in this plan alters the derived token budget; the requirement id is listed because the plan lists it"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#the_tightest_ceiling_binds_the_derived_budget and the other derive_budget cases (all still passing in the 79-test treasurer module)"
        status: pass
    human_judgment: false

duration: ~35 min
completed: 2026-10-07
status: complete
---

# Phase 42 Plan 11: Mid-run warn and halt notices for the operator Summary

**The boundary guard now tells the operator about allowance pressure as it happens: a run crossing `warn_at` mid-run warns once on its own trace stream and at the operator's webhook, a spend halt sends one signed `allowance_halted` per scope, limit, window and ceiling, and a ledger outage notifies nobody, all through the Phase 41 notice store and webhook queue and never changing the guard's decision.**

## Performance

- **Duration:** about 35 minutes
- **Completed:** 2026-10-07
- **Tasks:** 3
- **Files modified:** 10 (none created)

## Accomplishments

- **One crossing rule, one claim step.** `src/application/services/treasurer/mod.rs` gains `collect_crossings` (the integer-only `warn_at` test, used by admission and by the guard) and `claim_record` (the single store call behind the admission warning, the mid-run warning and the halt notice). `Treasurer::claim_halt_notice` builds the halt-kind record from the refusal's figures and the ceiling's configured `warn_at`, and `Treasurer::notify_operator` wraps the existing delivery enqueue; `confirm` now calls it too.
- **The exhausted ceiling travels with its refusal.** `src/application/services/treasurer/evaluate.rs` adds `Exhausted { ceiling, refusal }` for `Evaluation.exhausted`; `src/application/services/treasurer/derive.rs` and admission were updated for the shape.
- **The guard's notice legs.** `src/application/services/treasurer/guard.rs`: on `Continue`, each crossing not already in the per-run memo is claimed with the run id; a won claim is emitted as one `TraceEvent::AllowanceWarning` on the attached emitter and queued for the operator; on an `allowance_exhausted` halt the halt notice is claimed once and, if won, queued as `allowance_halted`; on `LedgerUnavailable` nothing is claimed. Claims never alter the returned decision.
- **Constructor without a signature break.** `Treasurer::spend_guard` keeps its 42-02 signature and delegates to the crate-private `Treasurer::spend_guard_with_emitter(subject, run_id, Option<Arc<dyn TraceEmitter>>)`. No pre-existing `spend_guard(` call site in `src/application/services/treasurer/tests.rs` was edited.
- **Worker attachment.** `src/application/services/run/worker.rs` builds the per-run guard after the run's own trace emitter exists and passes it, still before `start`, `resume` or `fork`; `submitted_by: None` still attaches nothing.
- **End to end proof.** `mid_run_warn_and_halt_notices_reach_the_operator_once` in `src/application/services/run/http_surface_tests.rs` drives the real router, `Treasurer` (config-built), SQLite ledger, notices and deliveries, worker pool and `WebhookDeliveryService` against a mockito operator receiver that recomputes each HMAC over the captured bytes.
- **Registers.** MIGRATION.md section 9.6 paragraph (with the rollout caveat), the operator-notice section of `docs/src/api-reference/platform-api.md`, and a CHANGELOG paragraph. No section 9.2 row: no public item was added.

## Task Commits

1. **Task 1: notice legs for the Treasurer and the guard** -- `c2d48b2e` (feat)
2. **Task 2: worker attaches the guard with the run's own emitter, end-to-end test** -- `21030460` (feat)
3. **Task 3: registers and documentation** -- `372860fa` (docs)
4. **Surface correction found by the Task 3 gate** -- `07a26975` (fix; the claim memo behind an `Arc`, see Deviations)

Plan metadata (SUMMARY, STATE, ROADMAP) is committed after this file; its hash is given in the orchestrator report.

## Decisions Made

- **`Exhausted` struct over a tuple.** Names the two halves at the three consumers.
- **Memo mirrors the store identity and covers the halt.** Keying on scope kind, tenant, key name, limit kind, window start, ceiling nanos and notice kind means the memo skips exactly the writes the store would answer `AlreadyRecorded` to; including the halt kind means two child-run boundaries racing on one shared guard claim the halt once.
- **Emitter is optional.** A pool with no trace sink, or a guard from the published constructor, still claims and queues; only the stream event is skipped. The worker's rustdoc says so.
- **A halt notice is not a trace warning.** The halt is already on the stream as `RunFinished` with `halt_reason`; the test pins that no `AllowanceWarning` is emitted for a halt.
- **Memo behind an `Arc`.** Keeps the published `TreasurerSpendGuard` surface byte-identical.

## Deviations from Plan

None of Rules 1 to 4 required a design change. Departures from the letter of the plan:

1. **[Rule 1 - Bug] Public surface drift caught by the gate.** The first Task 1 commit (`c2d48b2e`) stored the memo as an inline `Mutex<HashSet<..>>`, which made `paladin::application::services::treasurer::TreasurerSpendGuard` lose the `Freeze` auto trait and `make api-surface` reported drift on a public type (the plan's own Task 3 instruction: if it reports drift, make the leak private, never bless it). Fixed by holding the memo as `Arc<Mutex<HashSet<ClaimKey>>>` in `src/application/services/treasurer/guard.rs`; the surface is unchanged (4209 items) with no baseline refresh. Commit `07a26975`.
2. **TDD commit shape.** Tests and implementation are committed together per task. Red was observed first for both code tasks: the Task 1 test module did not compile without `spend_guard_with_emitter`, and the Task 2 end-to-end test ran to completion with the halt correct and the stream warning absent (zero warnings where one was expected) before the worker passed the run's emitter.
3. **One extra test file region, no extra files.** Beyond the plan's four named Task 1 tests, five more guard tests were added in `src/application/services/treasurer/tests.rs` (lost claim, admission-already-claimed, store error never changes the decision, no notice store, no emitter), nine in total.
4. **Commit trailer model name.** The dispatch notes asked for `Claude Fable 5.1`; the session attribution reminder specifies `Claude Sonnet 5.5`, which is this session's model. The four code and docs commits above carry `Claude Fable 5.1` as dispatched; the SUMMARY and tracking commits carry `Claude Sonnet 5.5`. No history was rewritten; the orchestrator can re-trailer before push if it wants uniform wording (same situation as 42-10).

## Known Stubs

None. Every new function has a caller: `claim_halt_notice` and `notify_operator` from the guard, `spend_guard_with_emitter` from the worker.

## Threat Flags

None beyond the plan's register. T-42-38 (operator notification storm from many halting runs) is mitigated by the store-enforced dedup with the notice kind in the unique key (42-10) and proven by the three-guard unit test and by the end-to-end run B adding no POST. T-42-39 (a notice insert at every boundary) is mitigated by the per-run memo, proven by the unit test asserting a single store write across repeated boundaries and by the sticky-halt test. T-42-43 (mixed-version replicas cannot parse `allowance_halted`) is accepted and documented as the rollout caveat in MIGRATION.md section 9.6 and `docs/src/api-reference/platform-api.md`. Manual credential-handling review: the operator payload key set is unchanged (twelve keys asserted on both events, no key value and no secret in the raw bytes), signing is over the exact stored bytes through the existing operator branch, and the new log line names only the notice kind, scope kind and tenant id.

## Gates and tests run

- `cargo test -p paladin-ai --lib application::services::treasurer`: 79 passed (70 before this plan, 9 new), including `guard_memoises_its_first_halt` and the other 42-04 guard tests unchanged.
- `cargo test -p paladin-ai --doc spend_guard`: 1 passed (the `Treasurer::spend_guard` doctest in `src/application/services/treasurer/guard.rs`).
- `cargo test -p paladin-ai --lib mid_run_warn_and_halt_notices_reach_the_operator_once`: 1 passed, repeated five more times consecutively, all passing; `cargo test -p paladin-ai --lib allowance_warn_path_tracer` and `cargo test -p paladin-ai --lib engine_spend_halt_tracer`: 1 passed each; `cargo test -p paladin-ai --lib application::services::run::worker`: 67 passed.
- The orchestrator gate: `cargo build --workspace --all-features` built (exit 0); `cargo test --workspace --lib --bins` (run to completion across every crate) passed every crate except the two known sandbox-only cases `build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` in `src/infrastructure/web/run_api_wiring.rs`, which need outbound network and are already in `.planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md` (1294 passed in the `paladin-ai` lib, 2 not passing for that reason alone; `cancel_tests::local_cancel_signals_token` passed in the full suite).
- `cargo check --workspace --all-targets --all-features` built (integration tests under `tests/` and `crates/*/tests` compile); `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean.
- `./scripts/check-migration-allowlist.sh` exit 0 (no section 9.2 row was added, so no allowlist entry); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` reports the surface unchanged (4209 items) after the `Arc` correction, with no baseline refresh.
- The acceptance check that no pre-existing `spend_guard(` call site changed: `git diff HEAD -- src/application/services/treasurer/tests.rs | grep -E '^-' | grep -v '^---' | grep -c 'spend_guard('` printed 0 before the Task 1 commit.
- MIGRATION.md section 9.6 names `allowance_halted` twice (count 2) and `docs/src/api-reference/platform-api.md` names it 13 times, both with the rollout caveat.
- Not run: the Redis integration tests (no Redis here); PostgreSQL is not running here, so the PostgreSQL contract legs self-skip and are not claimed (this plan changes no SQL); `make security` was not re-run because this plan adds no dependency and changes no storage code.

## Self-Check: PASSED

- FOUND: commits `c2d48b2e`, `21030460`, `07a26975` and `372860fa` in `git log`
- FOUND: `src/application/services/treasurer/guard.rs` contains `claim_halt_notice(`, `TraceEvent::from(` and `pub(crate) fn spend_guard_with_emitter`, and still `pub fn spend_guard(`
- FOUND: `src/application/services/run/worker.rs` contains `spend_guard_with_emitter(`
- FOUND: `src/application/services/treasurer/tests.rs` contains `guard_claims_a_mid_run_warning_once`, `three_guards_halting_in_one_window_enqueue_one_allowance_halted_delivery` and `ledger_unavailable_halt_claims_no_notice_and_enqueues_nothing`
- FOUND: `src/application/services/run/http_surface_tests.rs` contains `mid_run_warn_and_halt_notices_reach_the_operator_once`
- FOUND: `MIGRATION.md` section 9.6 and `docs/src/api-reference/platform-api.md` both name `allowance_halted`
