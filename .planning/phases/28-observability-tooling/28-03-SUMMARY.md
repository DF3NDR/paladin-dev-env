---
phase: 28-observability-tooling
plan: 03
subsystem: observability
tags: [trace, tracing, engine, war-engine, redaction, heartbeat, edge-condition, parley]

# Dependency graph
requires:
  - phase: 28-01
    provides: "The twelve-variant TraceEvent/TraceRecord envelope, TraceEmitter, CompositeSink, and the per-run-stamping TraceDispatcher with drop accounting and sink-panic isolation"
provides:
  - "EdgeEvaluated emission at the edge-condition call site (Frontier::record_execution), one per evaluated outgoing edge, whether or not it fired"
  - "ParleyRaised emission the moment the engine builds an AwaitingInput outcome, one per raised ParleyRequest"
  - "Rate-limited NodeProgress::Heartbeat (at most one per node per 5s default interval), keyed per-node across the whole run"
  - "Populated RunFinished: status derived from the RunOutcome/Err the engine matches on, total_supersteps/total_tokens tallied synchronously on TraceDispatcher, duration_ms from the run's own start instant"
  - "WarEngine::trace_emitter() -> Arc<dyn TraceEmitter>, the seam 28-06 hands to below-engine producers"
  - "TraceDispatcher::with_state_values(enabled, cap_bytes): DeltaMerged.field_changes[].value is redacted (paladin-llm::redaction::redact_secret_patterns) THEN truncated (bounded_excerpt), never the reverse; value_bytes is now the real serialized byte length always"
affects: ["28-04 (SSE collapse, RunTracePort consume the now-complete engine-owned event set)", "28-06 (worker.rs/FallbackLlmAdapter/middleware wire WarEngine::trace_emitter() and TraceConfig into TraceDispatcher::with_state_values, replacing this plan's hardcoded defaults)", "28-08 (eval harness's edge_fired/parley_raised/total_tokens_max assertions)", "28-10 (graph overlay's fired/evaluated edge sets)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-run counters tallied SYNCHRONOUSLY inside TraceDispatcher::emit (superstep_count/token_total), never by the async consumer task -- mirrors the existing dropped/sink_panics pattern so a caller reading them back immediately after a run never races the background drain"
    - "A dispatcher-level opt-in flag (TraceDispatcher::with_state_values) instead of a new parameter threaded through superstep::run's already-large signature -- the flag travels with the SAME object already threaded everywhere"
    - "redact-then-truncate ordering for any value placed on a trace record, reusing paladin-llm's existing redact_secret_patterns/bounded_excerpt rather than a second implementation"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/Cargo.toml

key-decisions:
  - "record_execution's force_notfiring branch (a node that authored its own routing via Goto/Muster/End/Parley) never emits EdgeEvaluated -- those edges are never evaluated at all (skips evaluate_edge_condition entirely, per the function's own pre-existing doc comment), so 'every evaluated edge' correctly excludes them."
  - "Heartbeat rate-limiter state (last-emitted-per-node map, Arc<Mutex<HashMap<NodeId, Instant>>>) and the 5s DEFAULT_HEARTBEAT_INTERVAL constant are constructed once per run_with_namespace call and cloned into each dispatched node's spawned task -- NOT added to EngineLimits (23-CONTEXT D-18 keeps that struct out of the graph fingerprint) and NOT threaded as a new run() parameter (would touch ~150 existing test call sites for a value 28-06 will source from TraceConfig anyway)."
  - "total_supersteps/total_tokens: rather than adding fields to RunOutcome's variants (would break exhaustive-match call sites across ~23 files outside paladin-battalion, including src/application/services and every integration test) or threading a new accumulator parameter through run/run_with_namespace, TraceDispatcher itself gained superstep_count()/token_total() -- two atomics updated inside emit() when it sees SuperstepStarted/NodeFinished pass through, read back by WarEngine's own entry points right before constructing RunFinished. Deliberately mirrors dropped_count()/sink_panics()'s existing shape on the same struct."
  - "WarEngine::trace_emitter() is bound to 'this engine's most recently constructed dispatcher' (a Mutex<Option<Arc<TraceDispatcher>>> cell every start/resume* populates), not a dispatcher pre-bound to a future run's thread -- WarEngine has no stored ThreadId to construct one with before a caller supplies it via start()/resume*(). A caller that calls trace_emitter() BEFORE any run gets a lazily-constructed placeholder-thread dispatcher that is REPLACED (orphaned) the moment the next real start/resume* call runs, since every entry point unconditionally builds its own fresh dispatcher (D-03's per-run seq-restart contract, unchanged) and only ALSO records it into the cell. This is a known, documented gap left for 28-06's own wiring work (production's worker.rs already knows run.thread_id before constructing the per-run engine -- 28-06 will need to thread it through so trace_emitter(), called before start(), returns the SAME thread-bound dispatcher start() itself then uses)."
  - "TraceDispatcher::with_state_values(enabled, cap_bytes) is a post-construction builder method (not a new TraceDispatcher::new/with_capacity parameter) -- avoids touching all 19 existing constructor call sites for a knob only one new test and 28-06's future wiring need. Field/method naming (state_value_cap_bytes, default 256) matches TraceConfig::value_cap_bytes's own name and default from plan 28-02, even though the value is handed to paladin_llm::redaction::bounded_excerpt as a CHARACTER budget (that helper's own contract) -- an exact approximation for ASCII-dominant JSON, conservative for multi-byte UTF-8, documented on the method."
  - "paladin-llm promoted from paladin-battalion's [dev-dependencies] to a real [dependencies] entry (Cargo.toml) -- production engine code (DeltaMerged's value path), not just tests, now calls paladin_llm::redaction::redact_secret_patterns/bounded_excerpt. No cycle: paladin-llm depends only on paladin-core + paladin-ports."
  - "dispatch/writers on FieldChange stay placeholder defaults (empty string/empty vec) -- 28-01's own documented scope boundary (Battlefield::merge's MergeReport today only tracks changed field NAMES, not per-field dispatch rule or writer list) is unchanged by this plan; this plan's own <behavior> tests and acceptance criteria test value/value_bytes only, never dispatch/writers, confirming that boundary still holds for this plan."

patterns-established:
  - "A per-node, per-run rate limiter for a high-frequency trace signal: an Arc<Mutex<HashMap<NodeId, Instant>>> constructed once per run, cloned into each spawned dispatch task, checked-and-updated inside the SAME poll cycle that observes the triggering event -- future high-frequency signals (e.g. StreamChunk) can reuse this exact shape."

requirements-completed: [OBS-01]

coverage:
  - id: D1
    description: "EdgeEvaluated: one record per evaluated outgoing edge (fired or not), condition_kind matching the EdgeCondition discriminant, emitted at Frontier::record_execution's own call site"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#branch_emits_one_edge_evaluated_per_edge"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#always_edge_reports_always"
        status: pass
    human_judgment: false
  - id: D2
    description: "ParleyRaised: one record per raised ParleyRequest, emitted the moment the engine builds an AwaitingInput outcome"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#awaiting_input_emits_one_parley_raised_per_request"
        status: pass
    human_judgment: false
  - id: D3
    description: "NodeProgress::Heartbeat is rate-limited to at most one emission per node per 5s default interval, keyed independently per node across the whole run"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#heartbeat_is_rate_limited_per_node"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#heartbeat_rate_limit_is_per_node"
        status: pass
    human_judgment: false
  - id: D4
    description: "RunFinished carries a real status (Completed/Failed/Halted/AwaitingInput derived from the RunOutcome/Err), total_supersteps, total_tokens (summed across every NodeFinished the run's own dispatcher stamped) and duration_ms > 0"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#run_finished_reports_completed_with_totals"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#run_finished_reports_failed_and_halted_and_awaiting_input"
        status: pass
    human_judgment: false
  - id: D5
    description: "NodeStarted carries the Muster task key when the node is a muster worker (None otherwise); a retried NodeFinished's duration_ms/token_count match the Waypoint's own NodeExecutionRecord for the same attempt"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#node_started_carries_muster_task_key"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#node_finished_carries_real_outcome_and_cost"
        status: pass
    human_judgment: false
  - id: D6
    description: "DeltaMerged.field_changes carries names/sizes only by default; with value inclusion enabled the value is redacted THEN truncated, never the reverse"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#delta_merged_carries_names_not_values_by_default"
        status: pass
    human_judgment: false
  - id: D7
    description: "WarEngine::trace_emitter() returns a handle sharing the run's own seq counter -- no repeats, no gaps"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#trace_emitter_shares_the_run_counter"
        status: pass
    human_judgment: false
  - id: D8
    description: "The whole workspace stays green: cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test -p paladin-battalion --lib (776 passed), and the five engine integration test binaries (war_engine_tracer, e2e_crash_resume, e2e_approval_gate, e2e_compensation_chain, e2e_muster_defer_order)"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib (776 passed, 0 failed)"
        status: pass
      - kind: unit
        ref: "cargo test --test war_engine_tracer --test e2e_crash_resume --test e2e_approval_gate --test e2e_compensation_chain --test e2e_muster_defer_order (112 passed, 0 failed)"
        status: pass
      - kind: unit
        ref: "cargo clippy --workspace --all-targets -- -D warnings (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: 56min
completed: 2026-09-08
status: complete
---

# Phase 28 Plan 03: Engine-Owned Trace Producers Summary

**`EdgeEvaluated`/`ParleyRaised`/rate-limited `NodeProgress::Heartbeat` emission, a fully-populated `RunFinished` (status/totals/duration), `WarEngine::trace_emitter()`, and redact-then-truncate `DeltaMerged` value inclusion — the engine's own share of PRD 07's twelve-variant trace contract, closing 27-CONTEXT D-25's status-vocabulary correction along the way.**

## Performance

- **Duration:** ~56 min
- **Started:** 2026-09-08T22:46:11Z (base commit)
- **Completed:** 2026-09-08T23:42:21Z
- **Tasks:** 3 (1 tracer, 2 auto/tdd)
- **Files modified:** 4

## Accomplishments

- `crates/paladin-battalion/src/engine/superstep.rs`: `Frontier::record_execution` emits one `TraceEvent::EdgeEvaluated { from, to, condition_kind, fired }` per evaluated outgoing edge (both the `None`/"always" case and every `Some(EdgeCondition)` case), via a wildcard-free `edge_condition_kind` helper; the `AwaitingInput` construction site emits one `ParleyRaised` per raised request before persisting the Waypoint; `idle_or_pending` now also rate-limits `NodeProgress::Heartbeat` (at most one per node per 5s default interval, via a per-run `Arc<Mutex<HashMap<NodeId, Instant>>>`); `DeltaMerged.field_changes` computes a real `value_bytes` and, when the dispatcher's `state_values` flag is set, a redacted-then-truncated `value`.
- `crates/paladin-battalion/src/engine/hooks.rs`: `TraceDispatcher` gained `superstep_count()`/`token_total()` (two atomics tallied synchronously inside `emit()`) and the `state_values`/`state_value_cap_bytes` fields with `with_state_values()`/`state_values_enabled()`/`state_value_cap_bytes()`.
- `crates/paladin-battalion/src/engine/mod.rs`: all five `RunStarted`/`RunFinished` bracket sites (`start`, `resume_with_options` ×2, `resume_with`, `replay_or_fork`) now populate `RunFinished` from a new `run_finish_status()` helper plus the dispatcher's own counters and a captured `run_started_at`; `WarEngine` gained `trace_emitter()` and the `trace_dispatcher_cell`/`remember_trace_dispatcher` machinery backing it.
- `crates/paladin-battalion/Cargo.toml`: `paladin-llm` promoted from `[dev-dependencies]` to `[dependencies]` so the DeltaMerged redaction path can call `paladin_llm::redaction::{redact_secret_patterns, bounded_excerpt}` from production code.

## Task Commits

1. **Task 1 (tracer): `EdgeEvaluated` end to end** — bundled with Task 2 in the same file (see commit 2)
2. **Task 1+2: `EdgeEvaluated`, `ParleyRaised`, rate-limited `Heartbeat`, `DeltaMerged` value inclusion** — `a416ba5a` (feat) — `crates/paladin-battalion/src/engine/superstep.rs`
3. **Task 3: populated `RunFinished`, `WarEngine::trace_emitter()`, `TraceDispatcher` state-values opt-in** — `12d909c9` (feat) — `crates/paladin-battalion/src/engine/mod.rs`, `hooks.rs`, `Cargo.toml`

**Commit granularity note:** all three tasks touch `superstep.rs` (Task 3's `DeltaMerged` value/redaction work lives there too, alongside Task 1/2's edge/parley/heartbeat emission), so a strict one-commit-per-task split was not possible without fragile hunk-level staging. Grouped by FILE instead: commit 1 is everything in `superstep.rs` (Tasks 1, 2, and Task 3's `DeltaMerged` piece); commit 2 is everything in `mod.rs`/`hooks.rs`/`Cargo.toml` (the rest of Task 3). Both commits are individually buildable, clippy-clean and fully tested (verified independently before and after each commit).

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/superstep.rs` — `edge_condition_kind` helper; `Frontier::record_execution` gains a `trace: &Arc<TraceDispatcher>` param and emits `EdgeEvaluated`; `DEFAULT_HEARTBEAT_INTERVAL` const; `idle_or_pending`/`race_attempt` gain `node_id`/`trace`/`heartbeat_last_emitted`/`heartbeat_interval` params and rate-limit `NodeProgress::Heartbeat`; `AwaitingInput` construction emits `ParleyRaised`; `DeltaMerged` construction computes `value_bytes`/opt-in `value`; 7 new tests
- `crates/paladin-battalion/src/engine/hooks.rs` — `TraceQueue` gains `superstep_count`/`token_total` atomics, tallied inside `emit()`; `TraceDispatcher` gains `state_values`/`state_value_cap_bytes` fields, `superstep_count()`/`token_total()`/`with_state_values()`/`state_values_enabled()`/`state_value_cap_bytes()`
- `crates/paladin-battalion/src/engine/mod.rs` — `run_finish_status()` helper; all five `RunStarted`/`RunFinished` sites populated; `WarEngine` gains `trace_dispatcher_cell`, `remember_trace_dispatcher()`, `trace_emitter()`; 6 new tests; existing `trace_sink_receives_exact_ordered_event_sequence_for_two_superstep_run` updated for the new `EdgeEvaluated` record
- `crates/paladin-battalion/Cargo.toml` — `paladin-llm` moved from `[dev-dependencies]` to `[dependencies]`

## Decisions Made

See `key-decisions` in frontmatter. The two most consequential: (1) `total_supersteps`/`total_tokens` are tallied via two new atomics ON `TraceDispatcher` itself (mirroring the existing `dropped`/`sink_panics` shape) rather than by adding fields to `RunOutcome`'s variants, which would have broken exhaustive-match call sites across ~23 files well outside this plan's scope; (2) `WarEngine::trace_emitter()`'s "bound to this engine's dispatcher" is honored for the achievable case (a caller reading it back during/after a run shares that run's own `seq` counter, proven by `trace_emitter_shares_the_run_counter`) but the "call it before `start()`" case production's own `worker.rs` composition point will need (28-06) is left as a documented, known gap — `WarEngine` has no stored `ThreadId` to construct a REAL dispatcher with before a caller supplies one via `start`/`resume*`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `paladin-llm` was a dev-dependency only, but the plan requires production code to call its redaction helpers**
- **Found during:** Task 3 (implementing `DeltaMerged`'s redact-then-truncate value path)
- **Issue:** `crates/paladin-battalion/Cargo.toml` only listed `paladin-llm` under `[dev-dependencies]` (for `MockLlmAdapter` in tests); `cargo check` failed with `cannot find module or crate paladin_llm` when `superstep.rs`'s (non-test) `DeltaMerged` construction called `paladin_llm::redaction::redact_secret_patterns`/`bounded_excerpt`.
- **Fix:** Promoted the existing `paladin-llm` entry from `[dev-dependencies]` to `[dependencies]` (removing the now-redundant dev-dependency line). No cycle: `paladin-llm` depends only on `paladin-core` + `paladin-ports`.
- **Files modified:** `crates/paladin-battalion/Cargo.toml`
- **Verification:** `cargo check --workspace --all-targets --all-features` succeeds; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `12d909c9` (Task 3 commit)

**2. [Rule 1 - Bug] A pre-existing test's expected trace sequence didn't account for the new `EdgeEvaluated` record**
- **Found during:** Task 3 (running the full `paladin-battalion` lib test suite after Task 1/2's changes)
- **Issue:** `engine::tests::trace_sink_receives_exact_ordered_event_sequence_for_two_superstep_run` asserted an EXACT, hardcoded trace event sequence for a two-node chain graph; Task 1's new `EdgeEvaluated` emission correctly adds one record (for the first node's one outgoing edge) between the first superstep's `DeltaMerged` and `WaypointSaved` — the second node has no outgoing edge, so no second `EdgeEvaluated` follows.
- **Fix:** Updated the test's expected `Vec<&str>` to insert `"EdgeEvaluated"` at the correct position, with a comment explaining why only the first superstep gets one.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`
- **Verification:** `cargo test -p paladin-battalion --lib engine::tests::trace_sink_receives_exact_ordered_event_sequence_for_two_superstep_run` passes.
- **Committed in:** `12d909c9` (Task 3 commit)

**3. [Rule 1 - Bug] A heartbeat rate-limit test's boundary-crossing beat raced against the attempt's own completion and was intermittently lost**
- **Found during:** Task 2 (writing `heartbeat_is_rate_limited_per_node`)
- **Issue:** `race_attempt`'s `tokio::select! { biased; result = attempt => ..., _ = idle_or_pending(..) => .. }` polls `attempt` FIRST every cycle. When a node's LAST heartbeat is also its LAST action before returning (no more `.await` points after), `attempt` resolves `Ready` on that same poll cycle, and `biased` short-circuits `select!` before `idle_or_pending` is ever polled again to observe that final `changed()` notification — so the rate-limit check-and-emit logic for that specific beat never runs. My first test design (6 beats, 1s apart, the 6th also being the node's final action) hit this exactly: only 1 of the expected 2 `NodeProgress::Heartbeat` records appeared.
- **Fix:** Extended the test's hold duration by one more beat interval (6 beats → 7 beats) so the boundary-crossing beat (the 6th) is followed by at least one more pending `sleep` inside the node's own loop — `attempt` then returns `Pending` on that poll cycle instead of `Ready`, forcing `select!` to also poll `idle_or_pending`, which reliably observes and processes the beat. Documented the race mechanism in the test's own doc comment as a genuine, minor characteristic of heartbeats-as-liveness-signals (a heartbeat that coincides exactly with task completion may be lost — acceptable, since the task has already finished by the time it would have mattered), not something worth restructuring `race_attempt` to eliminate.
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs` (test only — no production code changed)
- **Verification:** `cargo test -p paladin-battalion --lib engine::superstep::tests::heartbeat_is_rate_limited_per_node` passes repeatably.
- **Committed in:** `a416ba5a` (Task 1+2 commit)

**4. [Rule 1 - Bug] My own first draft of the redaction test asserted the wrong thing**
- **Found during:** Task 3 (writing `delta_merged_carries_names_not_values_by_default`)
- **Issue:** `redact_secret_patterns`'s `redact_token_after` helper keeps the MARKER text (e.g. `"sk-"`) visible in the output and replaces only the SECRET TOKEN that follows it (by design — readability: you can tell a value WAS an `sk-`-shaped key without seeing it). My first assertion (`!value.contains("sk-")`) was wrong given this correct, by-design behavior and failed.
- **Fix:** Changed the assertion to check the SECRET portion (the 40-character token after `sk-`) is absent, not the marker itself.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs` (test only)
- **Verification:** `cargo test -p paladin-battalion --lib engine::tests::delta_merged_carries_names_not_values_by_default` passes.
- **Committed in:** `12d909c9` (Task 3 commit)

---

**Total deviations:** 4 auto-fixed (1 blocking dependency promotion, 3 test-correctness fixes — 1 in a pre-existing test, 2 in this plan's own new tests). No production-code deviation beyond the necessary `Cargo.toml` promotion; no scope creep.
**Impact on plan:** All four were necessary for the plan's own stated acceptance criteria (`cargo test -p paladin-battalion --lib engine` green, `cargo clippy --workspace --all-targets -- -D warnings` clean) to hold.

## Issues Encountered

None beyond the deviations documented above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Every `TraceEvent` variant the ENGINE itself is responsible for producing (per PRD 07 §2.1, this plan's own scope) now has a real, tested emission site: `EdgeEvaluated`, `ParleyRaised`, rate-limited `NodeProgress::Heartbeat`, a fully-populated `RunFinished`, and `DeltaMerged`'s complete `value_bytes`/opt-in-`value` shape.
- `WarEngine::trace_emitter()` exists and is proven to share the run's own `seq` sequence for the achievable "call during/after a run" case; 28-06's wiring work (worker.rs → `FallbackLlmAdapter::with_trace_emitter` / the middleware chain) will need to resolve the "call `trace_emitter()` BEFORE `start()`" ordering this plan left as a documented, known gap (see key-decisions).
- `TraceDispatcher::with_state_values(enabled, cap_bytes)` exists and is proven correct (redact-then-truncate, names-only by default); 28-06 wires `TraceConfig::state_values`/`value_cap_bytes` through it in place of this plan's own direct-call test path.
- `FieldChange.dispatch`/`writers` remain placeholder defaults — unchanged scope boundary from 28-01, not addressed by this plan (its own tests never exercise those two fields); a future plan enriching `Battlefield::merge`'s `MergeReport` is still required before they carry real data.
- No blockers for 28-04/28-06/28-08/28-10.

## Self-Check: PASSED

- FOUND: `crates/paladin-battalion/src/engine/superstep.rs`
- FOUND: `crates/paladin-battalion/src/engine/mod.rs`
- FOUND: `crates/paladin-battalion/src/engine/hooks.rs`
- FOUND: `crates/paladin-battalion/Cargo.toml`
- FOUND commit: `a416ba5a`
- FOUND commit: `12d909c9`

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-08*
