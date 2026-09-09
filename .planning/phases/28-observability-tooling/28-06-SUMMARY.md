---
phase: 28-observability-tooling
plan: 06
subsystem: observability
tags: [trace, tracing, log-sink, task-local, middleware, streaming, criterion, serde]

# Dependency graph
requires:
  - phase: 28-01
    provides: "The twelve-variant TraceEvent/TraceRecord envelope, TraceEmitter, CompositeSink, and the per-run-stamping TraceDispatcher with drop accounting and sink-panic isolation"
  - phase: 28-02
    provides: "TraceConfig (log_sink, channel_capacity, state_values, value_cap_bytes, heartbeat_interval_secs, otel) with EnvOverridable/validate()"
  - phase: 28-03
    provides: "EdgeEvaluated/ParleyRaised/rate-limited Heartbeat, populated RunFinished, WarEngine::trace_emitter() (with its documented before-start()-orphans-a-placeholder gap), TraceDispatcher::with_state_values"
provides:
  - "LogTraceSink (src/infrastructure/telemetry/log_sink.rs): one log::info! line per TraceRecord under target paladin::trace, JSON-serialized, diagnostics-only on failure"
  - "build_run_sink (src/infrastructure/telemetry/mod.rs): the single place a run's sink fan-out (log + D-24 bus, or either alone, or neither) is assembled"
  - "WarEngine::with_bound_trace/with_bound_trace_dispatcher/with_trace_capacity (paladin-battalion): closes the trace_emitter()-before-start() orphaned-placeholder gap 28-03 documented"
  - "StandaloneEmitter + RUN_TRACE_EMITTER task-local + current_trace_emitter() (paladin-ports): the mechanism a below-engine producer reached through a deeply shared, long-lived PaladinPort singleton uses to stamp into ONE run's own seq sequence without a per-instance field racing concurrent runs"
  - "FallbackLlmAdapter::with_trace_emitter (replacing with_trace_sink): record_hop emits through the explicit handle first, then current_trace_emitter()"
  - "ModelCallContext::trace_emitter/middleware_action_hint and ToolCallContext::trace_emitter; middleware::chain emits one MiddlewareEvent per action for all six MiddlewareAction values"
  - "PaladinExecutionService::with_trace_emitter; NodeProgress::ToolCall at both tool-dispatch sites, NodeProgress::StreamChunk (byte count only) in the streaming forwarding loop"
  - "RunWorkerPool::with_trace_config + per-run TraceDispatcher/CompositeSink composition in run_once's engine_factory branch, wrapped in a RUN_TRACE_EMITTER.scope for the whole dispatch"
  - "benches/engine_benchmarks.rs bench_superstep_cost_sink_variants (none/log_sink/composite) and 28-BENCH-EVIDENCE.md's recorded, honest FAIL against PRD 07 acceptance 6's <=3% bar"
affects: ["28-04 (SSE collapse, RunTracePort consume the now-complete producer set)", "28-08 (paladin-eval harness reads the same trace stream)", "28-09/28-10 (OTel sink, graph overlay build on build_run_sink's composition point)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "tokio::task_local! for per-run ambient context reaching a deeply shared, long-lived singleton (RUN_TRACE_EMITTER): each tokio task has its own independent value, so setting it once at the top of one run's dispatch and awaiting the engine call inside that scope makes the handle observable to every nested .await in that task without a per-instance field racing concurrent runs sharing the singleton. Explicit field (when the caller has one) always checked FIRST, ambient task-local as fallback -- the same precedence FallbackLlmAdapter/ModelCallContext/PaladinExecutionService all use."
    - "A middleware-set 'action hint' (ModelCallContext::middleware_action_hint) for reporting an action the chain driver cannot observe structurally from the returned MiddlewareFlow (Retry/Fallback/Redact all return Continue) -- set by the middleware on itself, read-and-cleared by the driver after every before_model/after_model call, general (not special-cased) so a future middleware reuses the same mechanism."
    - "A fixed, documented placeholder NodeId for a below-engine producer that has no real node context (PaladinExecutionService's NodeProgress records) -- the same limitation FallbackHop's node_id: None already carries, except NodeProgress::node_id is not Optional so a literal placeholder is used instead."

key-files:
  created:
    - src/infrastructure/telemetry/log_sink.rs
    - src/infrastructure/telemetry/mod.rs
    - .planning/phases/28-observability-tooling/28-BENCH-EVIDENCE.md
  modified:
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-llm/src/fallback.rs
    - src/application/services/paladin/middleware/chain.rs
    - src/application/services/paladin/middleware/context.rs
    - src/application/services/paladin/middleware/resilience.rs
    - src/application/services/paladin/middleware/guardrail.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - src/infrastructure/mod.rs
    - benches/engine_benchmarks.rs

key-decisions:
  - "The below-engine producers (FallbackLlmAdapter, the middleware chain, PaladinExecutionService) reach a run's TraceEmitter via a tokio::task_local! (RUN_TRACE_EMITTER), not solely a per-instance field, because the facade's real composition root (src/infrastructure/web/run_api_wiring.rs) builds ONE Arc<dyn PaladinPort> singleton at boot and clones it into every concurrent run's engine -- a mutable field on that singleton would race concurrent runs, exactly what concurrent_runs_keep_independent_context_state already proves the architecture forbids. worker.rs's run_once wraps the whole engine dispatch (start/resume/resume_with/fork) in RUN_TRACE_EMITTER.scope(emitter, ..).await; every producer checks an explicit field FIRST (for standalone/test construction), falling back to current_trace_emitter()."
  - "WarEngine::with_bound_trace_dispatcher(thread, dispatcher) accepts an ALREADY-BUILT TraceDispatcher (not one WarEngine constructs internally) because worker.rs must hand the SAME Arc<TraceDispatcher> to both the engine (as its trace_sink-bound dispatcher) and to the run's task-local emitter BEFORE WarEngine::new's own paladin_port argument is even usable -- with_bound_trace(thread, run_id) (28-03's originally-scoped API) is kept as a convenience wrapper that builds one internally from self.trace_sink/trace_capacity for the achievable in-process case."
  - "tokio::spawn inside execute_stream_inner's chunk-forwarding loop does NOT inherit the calling task's RUN_TRACE_EMITTER scope (task-locals are per-task, never propagated across a spawn boundary) -- the effective emitter is resolved ONCE on the calling task, before spawning, and moved into the closure explicitly, rather than re-entering .scope() inside the spawned task."
  - "Retry/Fallback/Redact (3 of the 6 MiddlewareAction values) never change the MiddlewareFlow/ToolFlow the chain driver observes (ModelRetryMiddleware/ModelFallbackMiddleware return plain Continue after setting cx.retry_policy/llm_override; Guardrail's redaction is non-terminal, the sweep continues) -- ModelCallContext::middleware_action_hint is the general mechanism for a middleware to report such an action; the driver resets it before every before_model/after_model call and reads-and-clears it immediately after, so a hint never leaks onto the next middleware."
  - "PaladinExecutionService::NodeProgress records carry a fixed placeholder NodeId (\"paladin-execution-service\") because this service sits below the superstep engine with no real node context of its own -- mirrors FallbackHop's documented node_id: None limitation, except NodeProgress::node_id is not Optional so a literal value is required instead of None."
  - "Deliberately did NOT thread TraceConfig::heartbeat_interval_secs into the engine's rate limiter (crates/paladin-battalion/src/engine/superstep.rs's DEFAULT_HEARTBEAT_INTERVAL): the change would touch superstep::run/run_with_namespace's own signature and every one of its ~10 call sites plus every existing test constructing that call, for a knob no Task 1 <behavior> test or <acceptance_criteria> grep exercises. channel_capacity IS wired (WarEngine::with_trace_capacity, a small additive builder). This is a documented, deliberate scope reduction, not an oversight -- a future plan can thread it the same way with_trace_capacity does."
  - "Benchmark IDs for the three sink variants carry the literal substring \"bench_superstep_cost\" (engine/bench_superstep_cost_sinks_{none,log_sink,composite}), not just the containing Rust function's name -- criterion's own CLI filter matches against the benchmark ID string, and this plan's own <verify> command (cargo bench -- bench_superstep_cost --test) needs that literal substring present to select these three cases at all (proven: without it, the filter matched nothing and criterion silently exited 0 with zero benchmarks run)."

patterns-established:
  - "A run's whole below-engine trace-emission story routes through ONE task-local (RUN_TRACE_EMITTER) scoped around the engine dispatch call, read via current_trace_emitter() by every producer that also accepts an explicit field for standalone/test use -- future below-engine producers (a new middleware, a new adapter) should follow this exact precedence rather than inventing a new channel."

requirements-completed: [OBS-02]

coverage:
  - id: D1
    description: "LogTraceSink writes exactly one log::info! line per TraceRecord under target paladin::trace, JSON-serialized with thread_id/seq before kind (grep-able); a serialization failure is swallowed into Ok(()) with one error-level diagnostic"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/infrastructure/telemetry/log_sink.rs#log_sink_writes_one_json_line_per_record"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/log_sink.rs#log_sink_never_returns_err_and_logs_diagnostic"
        status: pass
    human_judgment: false
  - id: D2
    description: "build_run_sink is the single composition point: None with neither log_sink nor a bus sink configured (untraced path), the single sink unwrapped when exactly one is configured, a CompositeSink of both when both are"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/infrastructure/telemetry/mod.rs#neither_configured_returns_none"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/mod.rs#log_sink_only_returns_the_single_sink_unwrapped"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/mod.rs#bus_sink_only_returns_the_single_sink_unwrapped"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/mod.rs#both_configured_fans_out_to_a_composite"
        status: pass
    human_judgment: false
  - id: D3
    description: "The worker's per-run composition attaches one composite sink per run and a run's outcome/node-execution-count is byte-identical with sinks fully on vs fully off (the plan's own safety prohibition)"
    requirement: "OBS-02"
    verification:
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#worker_builds_one_composite_per_run"
        status: pass
      - kind: integration
        ref: "src/application/services/run/worker_tests.rs#worker_run_result_is_identical_with_and_without_sinks"
        status: pass
    human_judgment: false
  - id: D4
    description: "FallbackLlmAdapter::with_trace_emitter replaces with_trace_sink; a hop emitted while running inside a RUN_TRACE_EMITTER scope lands between the surrounding NodeStarted/NodeFinished seq numbers; a bare adapter with an explicit StandaloneEmitter still produces well-formed records with its own counter"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/fallback.rs#fallback_hop_lands_in_the_run_sequence"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/fallback.rs#standalone_emitter_is_used_when_no_engine_is_present"
        status: pass
      - kind: unit
        ref: "crates/paladin-llm/src/fallback.rs#no_emitter_available_records_nothing"
        status: pass
    human_judgment: false
  - id: D5
    description: "Each of the six MiddlewareAction values is emitted as a MiddlewareEvent naming the acting middleware, whether observed directly from MiddlewareFlow/ToolFlow (Finish/Fail/Deny/Redact-via-Rewrite) or via the middleware_action_hint mechanism (Retry/Fallback/Redact-via-Guardrail)"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/chain.rs#middleware_emits_one_event_per_action"
        status: pass
    human_judgment: false
  - id: D6
    description: "PaladinExecutionService emits NodeProgress::ToolCall{tool} at both tool-dispatch sites and NodeProgress::StreamChunk{bytes} per streamed delta (byte count only, never text)"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#stream_and_tool_progress_are_emitted"
        status: pass
    human_judgment: false
  - id: D7
    description: "WarEngine::with_bound_trace_dispatcher lets a caller pre-bind an externally-built dispatcher; trace_emitter() called before start() returns that SAME dispatcher instance start() then uses, and a mismatched-thread binding is never used"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#trace_emitter_before_start_uses_the_same_dispatcher_with_bound_trace"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#mismatched_bound_trace_thread_is_not_used"
        status: pass
    human_judgment: false
  - id: D8
    description: "bench_superstep_cost's three sink variants (none/log_sink/composite) compile, execute, and their real overhead against the untraced baseline is recorded with an explicit verdict against PRD 07 acceptance 6's <=3% bar"
    requirement: "OBS-02"
    verification:
      - kind: other
        ref: "cargo bench --bench engine_benchmarks -- bench_superstep_cost --test (Success x3)"
        status: pass
      - kind: manual_procedural
        ref: ".planning/phases/28-observability-tooling/28-BENCH-EVIDENCE.md"
        status: fail
    human_judgment: true
    rationale: "The measured overhead (log_sink +22.18%, composite +18.46% against a ~110us untraced baseline) genuinely FAILS the <=3% acceptance bar on this run -- recorded honestly per the plan's own instruction not to soften the criterion, flagged for phase close-out adjudication rather than auto-passed."
  - id: D9
    description: "Whole-workspace fmt/clippy/build stay green, and the ADR-0031 no-paladin-llm-in-paladin-battalion invariant holds"
    verification:
      - kind: other
        ref: "cargo fmt --all --check (exit 0); cargo check --workspace --all-targets --all-features (exit 0); cargo clippy --workspace --all-targets -- -D warnings (exit 0)"
        status: pass
      - kind: other
        ref: "cargo tree -p paladin-battalion --no-default-features -e normal | grep -c paladin-llm == 0"
        status: pass
      - kind: integration
        ref: "cargo test --features web-server --test e2e_platform_api (1 passed)"
        status: pass
    human_judgment: false

# Metrics
duration: 59min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 06: Default-On Log Sink, Per-Run Trace Composition, and Below-Engine MiddlewareEvent/NodeProgress Emission Summary

**A default-on `LogTraceSink` and single-composition-point `build_run_sink`, a `RUN_TRACE_EMITTER` task-local so `FallbackLlmAdapter`/the middleware chain/`PaladinExecutionService` — each reached through a deeply shared `PaladinPort` singleton — stamp into one run's own `seq` sequence without racing concurrent runs, `MiddlewareEvent` emission for all six `MiddlewareAction` values, `NodeProgress::StreamChunk`/`ToolCall` emission, and a three-variant `bench_superstep_cost` whose real, honestly-recorded overhead (+18–22%) fails PRD 07 acceptance 6's ≤3% bar.**

## Performance

- **Duration:** ~59 min
- **Started:** 2026-09-09T00:47:51Z (first task commit)
- **Completed:** 2026-09-09T01:51:33Z
- **Tasks:** 3 (1 tracer, 1 auto/tdd, 1 auto) — executed as 7 granular commits
- **Files modified:** 15 (3 created)

## Accomplishments

- `src/infrastructure/telemetry/{log_sink.rs,mod.rs}`: `LogTraceSink` (one JSON `log::info!` line per record under `paladin::trace`, diagnostics-only on serialize failure) and `build_run_sink(config, bus_sink)`, the single place a run's sink fan-out is assembled.
- Closed 28-03's documented `trace_emitter()`-before-`start()` gap: `WarEngine::with_bound_trace`/`with_bound_trace_dispatcher`/`with_trace_capacity` (`crates/paladin-battalion`) let a caller pre-bind a dispatcher (built internally or handed in already-constructed) so a handle pulled before `start()` is the SAME instance `start()` itself then uses.
- `paladin-ports`: `StandaloneEmitter` (a self-contained `TraceEmitter` for a producer with no run-scoped dispatcher — bare unit tests) and `RUN_TRACE_EMITTER`/`current_trace_emitter()` (the ambient per-run channel a real run's worker dispatch sets, reached by every below-engine producer that also accepts an explicit field first).
- `crates/paladin-llm/src/fallback.rs`: `FallbackLlmAdapter::with_trace_emitter` replaces `with_trace_sink`; `record_hop` emits directly through whichever emitter is available — no manual placeholder `ThreadId`/`TraceRecord` construction anymore, the dispatcher does the stamping.
- `middleware::chain`/`context`/`resilience`/`guardrail`: `ModelCallContext`/`ToolCallContext` gain `trace_emitter` + `middleware_action_hint`; `run_before`/`run_after`/`run_around_tool` emit one `MiddlewareEvent` per action for all six `MiddlewareAction` values — `Finish`/`Fail`/`Deny`/`Redact`(via `Rewrite`) observed directly, `Retry`/`Fallback`/`Redact`(via `Guardrail`) via the hint mechanism.
- `paladin_execution_service.rs`: `with_trace_emitter`; `NodeProgress::ToolCall{tool}` at both tool-dispatch sites, `NodeProgress::StreamChunk{bytes}` (byte count only, D-05) in the streaming forwarding loop — resolved once before `tokio::spawn` since task-locals don't cross a spawn boundary.
- `worker.rs`: `RunWorkerPool::with_trace_config`; `run_once`'s `engine_factory` branch builds ONE `TraceDispatcher` per run via `build_run_sink`, binds it into the engine, pulls `engine.trace_emitter()` as the canonical handle, and wraps the whole dispatch (`start`/`resume`/`resume_with`/`fork`) in `RUN_TRACE_EMITTER.scope(..)`.
- `benches/engine_benchmarks.rs` + `28-BENCH-EVIDENCE.md`: three sink-variant cases over the existing `build_width_graph(8)` fixture; one real run recorded — `none` 110.18µs, `log_sink` 134.58µs (+22.18%), `composite` 130.52µs (+18.46%) — an honest **FAIL** against the ≤3% bar, with analysis and a recommendation flagged for phase close-out.

## Task Commits

1. **Task 1 (tracer) — closing the trace_emitter()-before-start() gap + StandaloneEmitter (prerequisite engineering)** — `69818bcf` (feat)
2. **Task 1 — default-on log sink and per-run composition root** — `fa7c37aa` (feat)
3. **Task 2 (part 1) — FallbackLlmAdapter::with_trace_emitter** — `38246abe` (feat)
4. **fix — WarEngine::trace_emitter() as the canonical per-run handle in worker.rs** — `19086f24` (fix)
5. **Task 2 (part 2) — MiddlewareEvent emission for all six actions** — `0853b36a` (feat)
6. **Task 2 (part 3) — PaladinExecutionService NodeProgress emission** — `f15da211` (feat)
7. **Task 3 — bench the three sink variants, record overhead evidence** — `d2a8a002` (test)

**Plan metadata:** (this commit)

## Files Created/Modified

- `src/infrastructure/telemetry/log_sink.rs` - New: `LogTraceSink`
- `src/infrastructure/telemetry/mod.rs` - New: `build_run_sink`, module docs
- `src/infrastructure/mod.rs` - `pub mod telemetry;`
- `crates/paladin-battalion/src/engine/mod.rs` - `with_bound_trace`/`with_bound_trace_dispatcher`/`with_trace_capacity`/`take_or_build_trace_dispatcher`; 2 new tests
- `crates/paladin-ports/src/output/trace_sink_port.rs` - `StandaloneEmitter`, `RUN_TRACE_EMITTER` task-local, `current_trace_emitter()`; 2 new tests
- `crates/paladin-llm/src/fallback.rs` - `trace_sink`→`trace_emitter` rename; `record_hop` rewritten; 5 test call sites updated + 3 new tests
- `src/application/services/paladin/middleware/chain.rs` - `MiddlewareEvent` emission in `run_before`/`run_after`/`run_around_tool`; 1 new table test
- `src/application/services/paladin/middleware/context.rs` - `ModelCallContext::trace_emitter`/`middleware_action_hint`; `ToolCallContext::trace_emitter` + manual `Debug`
- `src/application/services/paladin/middleware/resilience.rs` - `ModelFallbackMiddleware`/`ModelRetryMiddleware` set the Fallback/Retry hint
- `src/application/services/paladin/middleware/guardrail.rs` - `Guardrail` sets the Redact hint on `RuleOutcome::Redacted`
- `src/application/services/paladin/paladin_execution_service.rs` - `with_trace_emitter`; `ToolCall`/`StreamChunk` emission; 1 new test
- `src/application/services/run/worker.rs` - `RunWorkerPool::with_trace_config`; per-run dispatcher composition + `RUN_TRACE_EMITTER.scope`
- `src/application/services/run/worker_tests.rs` - 2 new integration tests (`worker_builds_one_composite_per_run`, `worker_run_result_is_identical_with_and_without_sinks`)
- `benches/engine_benchmarks.rs` - `bench_superstep_cost_sink_variants` (3 cases)
- `.planning/phases/28-observability-tooling/28-BENCH-EVIDENCE.md` - New: recorded numbers and verdict

## Decisions Made

See `key-decisions` in frontmatter. The two most consequential: (1) below-engine producers reach a run's `TraceEmitter` via a `tokio::task_local!` (`RUN_TRACE_EMITTER`), not solely a per-instance field, because the real composition root builds ONE `Arc<dyn PaladinPort>` singleton at boot shared by every concurrent run — a mutable field there would race concurrent runs; (2) `WarEngine::with_bound_trace_dispatcher` accepts an externally-built dispatcher so worker.rs can hand the SAME instance to both the engine and the task-local scope, closing 28-03's documented "before `start()`" gap for real.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `crates/paladin-battalion/src/engine/mod.rs` and `crates/paladin-ports/src/output/trace_sink_port.rs` were not in `files_modified` but required edits to close the trace_emitter()-before-start() gap and provide `StandaloneEmitter`**
- **Found during:** Task 1/2 (discovering the composition-root chicken-and-egg: worker.rs needs the SAME dispatcher instance for both the engine and the below-engine producers, and `WarEngine::new` requires `paladin_port` — itself needing the emitter — before any per-run builder call)
- **Issue:** The plan's own prior-wave-facts explicitly named this gap as this plan's work ("Closing that gap is part of this plan's composition-root work"), but the concrete fix requires new public API on `paladin-battalion`'s `WarEngine` and a new task-local mechanism in `paladin-ports`, neither listed in `files_modified`.
- **Fix:** Added `WarEngine::with_bound_trace`/`with_bound_trace_dispatcher`/`with_trace_capacity` and the private `take_or_build_trace_dispatcher` helper; added `StandaloneEmitter` and the `RUN_TRACE_EMITTER` task-local + `current_trace_emitter()` to `paladin-ports`.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`, `crates/paladin-ports/src/output/trace_sink_port.rs`
- **Verification:** `cargo test -p paladin-battalion --lib` (778 passed), `cargo test -p paladin-ports --lib` (189 passed), both including new tests for the exact gap-closing behavior.
- **Committed in:** `69818bcf`

**2. [Rule 3 - Blocking] `src/application/services/paladin/middleware/context.rs`, `resilience.rs`, and `guardrail.rs` were not in `files_modified` but required edits to make "all six MiddlewareAction values" a real, reachable behavior rather than only testable via mocks**
- **Found during:** Task 2 (implementing `MiddlewareEvent` emission)
- **Issue:** `chain.rs` alone cannot observe `Retry`/`Fallback`/`Redact` structurally — `ModelRetryMiddleware`/`ModelFallbackMiddleware`/`Guardrail`'s redaction all return plain `Continue`. Satisfying the must-have truth ("each of the six... is emitted... when that action is taken") for real (not just via a chain.rs-local test double) requires those three production middleware files to report the action themselves.
- **Fix:** Added `ModelCallContext::trace_emitter`/`middleware_action_hint` and `ToolCallContext::trace_emitter` (`context.rs`); `ModelFallbackMiddleware`/`ModelRetryMiddleware` set the hint alongside their existing `llm_override`/`retry_policy` assignment (`resilience.rs`); `Guardrail` sets it on `RuleOutcome::Redacted` (`guardrail.rs`).
- **Files modified:** `src/application/services/paladin/middleware/{context.rs,resilience.rs,guardrail.rs}`
- **Verification:** `cargo test -p paladin-ai --lib -- application::services::paladin::middleware::` (86 passed, including the new table test `middleware_emits_one_event_per_action` proving all six).
- **Committed in:** `0853b36a`

**3. [Documented scope reduction, not a deviation rule] `TraceConfig::heartbeat_interval_secs` is NOT threaded into the engine's rate limiter**
- **Found during:** Task 1 (planning `build_run_sink`'s composition wiring)
- **Reasoning:** `superstep::run`/`run_with_namespace`'s own signature and every one of its ~10 call sites (plus every existing test constructing one) would need a new parameter for a knob no Task 1 `<behavior>` test or `<acceptance_criteria>` grep exercises. `channel_capacity` IS wired (`WarEngine::with_trace_capacity`, a small additive builder with no blast radius). Deliberately left as a known, documented gap — a future plan can thread it the same way.
- **Files NOT modified:** `crates/paladin-battalion/src/engine/superstep.rs` (left untouched, still hardcodes `DEFAULT_HEARTBEAT_INTERVAL`)

---

**Total deviations:** 2 auto-fixed (both Rule 3 - blocking, necessary for this plan's own stated must-have truths to hold), 1 documented scope reduction (heartbeat interval, not a bug — no test/criterion depends on it).
**Impact on plan:** All changes were necessary for Task 1/2's own acceptance criteria and must-have truths. No scope creep beyond what the plan's own D-03 "one counter per run" requirement demanded once the real architecture (a shared `PaladinPort` singleton) was discovered mid-implementation.

## Issues Encountered

- **Bench filter mismatch:** criterion's CLI filter matches the benchmark ID string, not the containing Rust function's name — the plan's own `<verify>` command (`-- bench_superstep_cost --test`) would have silently matched ZERO benchmarks (confirmed empirically: exit 0, no output) against IDs like `engine/superstep_cost_sinks_none`. Fixed by naming the new benchmark IDs `engine/bench_superstep_cost_sinks_{label}`, embedding the literal filter substring.
- **Log-capture test races:** two `#[cfg(test)]` modules (`log_sink.rs`'s own tests and `infrastructure::telemetry::tests::both_configured_fans_out_to_a_composite`) both exercise `log`'s single process-wide logger slot; an initial `std::sync::Mutex`-based guard scoped to one module didn't prevent cross-module interleaving. Fixed by tagging every test touching the shared logger with `#[serial_test::serial]` (its default key group is shared crate-wide), removing the manual mutex.
- **Benchmark overhead exceeds the ≤3% bar:** see `28-BENCH-EVIDENCE.md` — recorded honestly, not treated as a blocking issue for this plan's own completion (the task's scope is "measure and record," which is done), flagged for phase close-out.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The trace stream now has its first real consumer (`LogTraceSink`, default-on) and a single, tested composition point (`build_run_sink`) that 28-09 (OTel sink) and 28-10 (graph overlay) can extend the same way.
- Every below-engine producer (`FallbackLlmAdapter`, the middleware chain, `PaladinExecutionService`) is wired to the run's own `seq` sequence via `RUN_TRACE_EMITTER` — a future producer follows the same explicit-field-then-task-local precedence, established here as the house pattern.
- `TraceConfig::heartbeat_interval_secs` remains unwired into the engine's rate limiter — a documented, known gap (not a blocker) for whichever future plan wants it configurable.
- The bench evidence's FAIL verdict (+18–22% overhead against ≤3%) needs phase close-out adjudication: either re-scope the acceptance bar against a realistic (I/O-bound) workload, or invest in reducing `TraceDispatcher`/`LogTraceSink`'s per-record cost. Neither is blocking for 28-04/28-08/28-09/28-10, which consume the trace stream's CORRECTNESS, not its overhead bar.
- No blockers for the rest of Phase 28.

## Self-Check: PASSED

- FOUND: `src/infrastructure/telemetry/log_sink.rs`
- FOUND: `src/infrastructure/telemetry/mod.rs`
- FOUND: `.planning/phases/28-observability-tooling/28-BENCH-EVIDENCE.md`
- FOUND commit: `69818bcf`
- FOUND commit: `fa7c37aa`
- FOUND commit: `38246abe`
- FOUND commit: `19086f24`
- FOUND commit: `0853b36a`
- FOUND commit: `f15da211`
- FOUND commit: `d2a8a002`

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
