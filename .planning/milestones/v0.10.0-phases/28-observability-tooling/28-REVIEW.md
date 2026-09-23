---
phase: 28-observability-tooling
reviewed: 2026-09-09T12:30:43Z
depth: standard
files_reviewed: 96
files_reviewed_list:
  - .github/workflows/ci.yml
  - .github/workflows/feature-flags.yml
  - benches/engine_benchmarks.rs
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-battalion/src/engine/export/dot.rs
  - crates/paladin-battalion/src/engine/export/mermaid.rs
  - crates/paladin-battalion/src/engine/export/mod.rs
  - crates/paladin-battalion/src/engine/export/overlay.rs
  - crates/paladin-battalion/src/engine/export/shape.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/tests/export_golden.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/run.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-eval/CHANGELOG.md
  - crates/paladin-eval/Cargo.toml
  - crates/paladin-eval/README.md
  - crates/paladin-eval/src/assertion.rs
  - crates/paladin-eval/src/lib.rs
  - crates/paladin-eval/src/runner.rs
  - crates/paladin-eval/src/scenario.rs
  - crates/paladin-eval/src/scripted_llm.rs
  - crates/paladin-eval/tests/assertion_snapshots.rs
  - crates/paladin-eval/tests/schema_golden.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-ports/src/input/mod.rs
  - crates/paladin-ports/src/input/run_inspector_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/run_trace_port.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-storage/migrations/postgres/006_create_run_traces_table.sql
  - crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql
  - crates/paladin-storage/src/lib.rs
  - crates/paladin-storage/src/run_trace/contract_tests.rs
  - crates/paladin-storage/src/run_trace/in_memory.rs
  - crates/paladin-storage/src/run_trace/mod.rs
  - crates/paladin-storage/src/run_trace/postgres.rs
  - crates/paladin-storage/src/run_trace/retention.rs
  - crates/paladin-storage/src/run_trace/sqlite.rs
  - crates/paladin-web/Cargo.toml
  - crates/paladin-web/src/app.rs
  - crates/paladin-web/src/dev_ui/inspector.html
  - crates/paladin-web/src/dev_ui_controller.rs
  - crates/paladin-web/src/lib.rs
  - docs/src/SUMMARY.md
  - docs/src/operations/observability.md
  - docs/src/user-guides/eval-harness.md
  - docs/src/user-guides/graph-visualization.md
  - evals/e2e-1-crash-resume.eval.yaml
  - evals/e2e-2-approval-gate.eval.yaml
  - evals/e2e-3-map-reduce-fault-tolerance.eval.yaml
  - scripts/publish-crates.sh
  - src/application/cli/commands/eval.rs
  - src/application/cli/commands/graph.rs
  - src/application/cli/commands/mod.rs
  - src/application/cli/commands/run.rs
  - src/application/services/paladin/middleware/chain.rs
  - src/application/services/paladin/middleware/context.rs
  - src/application/services/paladin/middleware/guardrail.rs
  - src/application/services/paladin/middleware/resilience.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/run/events.rs
  - src/application/services/run/inspector.rs
  - src/application/services/run/mod.rs
  - src/application/services/run/stream_tests.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - src/application/services/waypoint_retention.rs
  - src/bin/paladin-cli.rs
  - src/config/mod.rs
  - src/config/settings.rs
  - src/config/trace.rs
  - src/config/user_config.rs
  - src/config/web_server.rs
  - src/infrastructure/mod.rs
  - src/infrastructure/telemetry/log_sink.rs
  - src/infrastructure/telemetry/mod.rs
  - src/infrastructure/telemetry/otel_sink.rs
  - src/infrastructure/telemetry/persisting_sink.rs
  - tests/cli/eval_run_test.rs
  - tests/cli/graph_export_test.rs
  - tests/cli/mod.rs
  - tests/cli/run_export_test.rs
  - tests/evals.rs
  - tests/helpers/e2e_fixtures.rs
  - tests/helpers/mod.rs
  - tests/integration/e2e_approval_gate_test.rs
  - tests/integration/e2e_crash_resume_test.rs
  - tests/integration/e2e_muster_defer_order_test.rs
  - tests/integration/mod.rs
  - tests/integration/otel_transport_test.rs
findings:
  critical: 1
  warning: 4
  info: 1
  total: 6
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-09-09T12:30:43Z
**Depth:** standard
**Files Reviewed:** 96
**Status:** issues_found

## Summary

Phase 28 wires a twelve-variant `TraceEvent`/`TraceRecord` model through `paladin-battalion`'s
engine, a fire-and-forget `TraceDispatcher`, an OTel exporter, an SSE bridge, a persisting sink,
`run_traces` storage, an eval harness and a feature-gated `dev-ui` inspector page. The
concurrency design (bounded drop-oldest queue, `catch_unwind` panic isolation, synchronous
`emit`, per-run `seq` counters, the `RUN_TRACE_EMITTER` task-local) is sound and well covered by
tests, including a real multi-thread/16-concurrent-run stress test. The security-sensitive paths
I traced by hand hold up: `OtelTraceSink`'s HTTP client disables redirects, `OtelConfig` never
derives `Debug` and manually redacts header values, `DeltaMerged`'s opt-in value inclusion
redacts before truncating (the documented ordering rule), the `dev-ui` route is admin-gated
under the same middleware the existing admin routes use, and its embedded `InspectorView` JSON
payload is escaped against `</script>`/`<!--` breakout. `paladin-battalion`'s `Cargo.toml`
confirms the `paladin-llm` edge flagged by ADR-0031 stayed `[dev-dependencies]`-only, not a
production leaf-to-leaf edge.

However, one confirmed BLOCKER survived: `TraceEvent::RunStarted.run_id` is hardcoded to `None`
at every `WarEngine::start`/`resume`/`resume_with`/`fork` emission site, even though the
production worker constructs each run's `TraceDispatcher` with the REAL `RunId`. Because
`OtelTraceSink` reads `run_id` from the event payload (not the correctly-populated
`TraceRecord.run_id` envelope), every OTel `run` root span in production is silently missing its
`paladin.run_id` attribute — defeating the ability to correlate an OTel trace back to a Platform
API run, one of this phase's own stated goals. This was masked because every unit and
integration test that constructs a `TraceEvent::RunStarted` also hardcodes `run_id: None`.

Four warnings and one info item are also recorded below, covering a misleading always-on
`MiddlewareEvent{action: Retry|Fallback}` emission, an inconsistency in the `dev-ui` page's
script-breakout escaping, a `NodeId` placeholder that makes concurrent Paladin nodes'
`NodeProgress` records indistinguishable, and the cross-directory `#[path]` inclusion of a
`tests/` fixture file into the shipped `paladin-ai` library/CLI surface.

## Critical Issues

### CR-01: `TraceEvent::RunStarted.run_id` is always `None` in production, breaking OTel run-span correlation

**File:** `crates/paladin-battalion/src/engine/mod.rs:2041,2187,2274,2622,2795`
**Also:** `src/infrastructure/telemetry/otel_sink.rs:190-213,457-462`, `src/application/services/run/worker.rs:861-883`

**Issue:** Every `WarEngine::start`/`resume`/`resume_with`/`fork` call site emits
`TraceEvent::RunStarted { run_id: None, .. }` unconditionally — the `run_id` field is never
populated from anywhere, despite `TraceEvent::RunStarted` documenting it as "The run this start
belongs to, when known" (`crates/paladin-core/src/platform/container/trace.rs:174-180`).

Meanwhile, the production worker (`worker.rs:861-883`) DOES construct each run's
`TraceDispatcher` with the real `RunId` (`Some(run.run_id.clone())`, passed into
`TraceDispatcher::with_capacity` and then `with_bound_trace_dispatcher`), so the *envelope*-level
`TraceRecord.run_id` is correctly populated on every record this dispatcher stamps. But nothing
in `mod.rs`'s `start`/`resume*` methods reads that value back out to populate the *event-level*
`RunStarted.run_id` field — there is no getter on `TraceDispatcher` for its own `run_id`, and the
five emit call sites all write the literal `None`.

`OtelTraceSink::on_event` (`otel_sink.rs:456-462`) destructures `TraceEvent::RunStarted { run_id,
graph_fingerprint }` and passes `run_id.as_ref()` into `start_run_span`, which only sets
`paladin.run_id` on the span when `Some` (`otel_sink.rs:201-203`). It never falls back to
`record.run_id` (the correctly-populated envelope field it already has in scope as `record` at
line 453). The result: **every OTel `run` root span exported to a real collector is silently
missing `paladin.run_id`**, in every build, permanently — regardless of whether the worker wired
a real `RunId` into the dispatcher.

This regression was invisible in CI because every test that constructs a `TraceEvent::RunStarted`
— in `otel_sink.rs`'s own `#[cfg(test)]` module, `tests/integration/otel_transport_test.rs`, and
`trace_sink_port.rs`'s own tests — also hand-writes `run_id: None`, so no test ever exercises the
populated case this bug silently drops.

**Fix:** Read the dispatcher's own `run_id` back out when emitting `RunStarted`. The simplest fix
is a `TraceDispatcher::run_id(&self) -> Option<&RunId>` getter (mirroring the existing
`thread_id()` accessor in `hooks.rs`), used at every emit site:

```rust
// crates/paladin-battalion/src/engine/hooks.rs
impl TraceDispatcher {
    pub fn run_id(&self) -> Option<&RunId> {
        self.run_id.as_ref()
    }
}

// crates/paladin-battalion/src/engine/mod.rs, every RunStarted emit site
trace.emit(TraceEvent::RunStarted {
    run_id: trace.run_id().cloned(),
    graph_fingerprint: graph.fingerprint().to_string(),
});
```

As a defense-in-depth companion fix, `OtelTraceSink::on_event`'s `RunStarted` arm should also
fall back to `record.run_id` when the event's own `run_id` is `None`, so a future producer that
makes the same mistake doesn't reintroduce this gap silently:

```rust
TraceEvent::RunStarted { run_id, graph_fingerprint } => {
    let run_id = run_id.as_ref().or(record.run_id.as_ref());
    self.start_run_span(&thread_id, run_id, at, graph_fingerprint);
}
```

## Warnings

### WR-01: `MiddlewareEvent{action: Retry|Fallback}` fires on every model call, not only when a retry/fallback actually happens

**File:** `src/application/services/paladin/middleware/resilience.rs:110-120,167-171`

**Issue:** `ModelFallbackMiddleware::before_model` unconditionally sets
`cx.middleware_action_hint = Some(MiddlewareAction::Fallback)` every time it runs, and
`ModelRetryMiddleware::before_model` unconditionally sets
`cx.middleware_action_hint = Some(MiddlewareAction::Retry)` every time it runs — regardless of
whether a fallback hop or a retry loop is ever actually exercised for that call. Both middleware
merely *install* a capability (`cx.llm_override` / `cx.retry_policy`) that
`execute_with_retry_and_temperature` (`paladin_execution_service.rs:2429-2578`) may or may not
end up using.

Per `crates/paladin-core/src/platform/container/trace.rs:135-138`, `MiddlewareAction::Retry` is
documented as "The middleware requested a retry" and `MiddlewareAction::Fallback` as "The
middleware requested a model fallback hop" — both read as *an action that happened*, not *a
capability that is present*. In practice, every single successful, first-attempt, no-error model
call on a Paladin configured with retry/fallback middleware now emits a `TraceEvent::
MiddlewareEvent` claiming a retry and/or fallback was requested, even though nothing failed.
There is no separate signal anywhere (I checked the retry loop at
`paladin_execution_service.rs:2544-2578`, which only logs via `warn!`/`error!`, no trace emit) for
"a retry attempt actually fired" vs. "a retry policy is merely configured" — an operator watching
the OTel span/dev-ui trace for a healthy run will see misleading `Retry`/`Fallback` events on
every call, undermining the exact "understand what the run really did" goal this phase exists to
serve.

**Fix:** Only set the hint when the corresponding action is actually taken. For
`ModelRetryMiddleware`, emit the hint from `execute_with_retry_and_temperature`'s own retry arm
(where a failure was observed and a retry is about to happen) rather than unconditionally from
`before_model`. For `ModelFallbackMiddleware`, rely on `FallbackLlmAdapter`'s own
`TraceEvent::FallbackHop` (which already only fires on a real hop,
`crates/paladin-llm/src/fallback.rs:194-217`) as the source of truth, and drop the always-on
`MiddlewareAction::Fallback` hint entirely, or gate it behind the same "a hop actually occurred"
condition.

### WR-02: `dev-ui` inspector page escapes the inspector JSON payload but not the sibling Mermaid-URL payload

**File:** `crates/paladin-web/src/dev_ui_controller.rs:150-160`, `crates/paladin-web/src/dev_ui/inspector.html:364-365`

**Issue:** `dev_ui_inspector_page` runs `escape_for_script` (which neutralizes `</` and `<!--`,
the two sequences that can break out of a `<script>` element or open an HTML comment) over the
`InspectorView` JSON before embedding it in `#inspector-data`. The Mermaid URL payload embedded
in the sibling `#dev-ui-mermaid-config` element, however, is only passed through
`serde_json::to_string` (line 155-156), which does not escape `/` or `<` — so a configured
`web_server.dev_ui.mermaid_url` value containing `</script>` or `<!--` would break out of that
`<script type="application/json">` element unescaped. The practical exploitability is low today
(the value is operator config, not request input), but it is an inconsistent application of the
documented breakout defense (D-26/T-28-15-02) and the one test covering this
(`dev_ui_page_escapes_the_embedded_payload`) only exercises the inspector-data payload, not the
Mermaid-URL one.

**Fix:** Run `escape_for_script` over `mermaid_url_json` too, for defense-in-depth consistency:

```rust
let mermaid_url_json = escape_for_script(
    &serde_json::to_string(&state.mermaid_url).unwrap_or_else(|_| "\"\"".to_string()),
);
```

### WR-03: `PaladinExecutionService`'s `NodeProgress` records use a single fixed placeholder `NodeId`, making concurrent nodes indistinguishable

**File:** `src/application/services/paladin/paladin_execution_service.rs:2795-2833`

**Issue:** `emit_tool_call_progress`/`emit_stream_chunk_progress` both stamp every
`TraceEvent::NodeProgress` with the same hardcoded `NodeId::new("paladin-execution-service")`
(`placeholder_node_id()`, lines 2797-2833), regardless of which graph node's execution actually
produced the tool call or stream chunk. Since one `PaladinExecutionService` singleton is shared
by every concurrent Paladin node in a Phalanx/parallel dispatch across a whole process, two nodes
streaming or calling tools concurrently in the SAME run produce `NodeProgress` records that are
byte-identical on `node_id` — a consumer has no way to attribute a `ToolCall`/`StreamChunk`
record back to the node that produced it. No current consumer reads this field (OTel's
`on_event` explicitly no-ops `NodeProgress`, and the SSE bus drops it via `map_trace_event`), so
this is latent rather than actively user-visible today, but it will misattribute data the moment
either consumer is extended to read `NodeProgress`.

**Fix:** Thread the real `NodeId` through to `PaladinExecutionService` (e.g. via
`ModelCallContext`, alongside `trace_emitter`) so `emit_tool_call_progress`/
`emit_stream_chunk_progress` can stamp the actual node rather than a fixed placeholder. If that
plumbing genuinely isn't available at this layer yet, at minimum document the placeholder's
false-precision risk more strongly than "no real node context" (which currently reads as merely
"there is no node," not "every node's progress looks identical").

### WR-04: `tests/helpers/e2e_fixtures.rs` is pulled into the shipped `src/` library/CLI surface via a cross-directory `#[path]`

**File:** `src/application/cli/commands/eval.rs:33-35`

**Issue:** `eval.rs` declares `#[path = "../../../../tests/helpers/e2e_fixtures.rs"] mod
e2e_fixtures;`, compiling a file that lives under the conventionally test-only `tests/` directory
directly into the `cli`-feature-gated portion of the `paladin` library crate (and, transitively,
the `paladin-cli` binary). The module doc justifies this as avoiding "a second,
independently-maintained copy of the graph-building logic," and it is a deliberate, documented
choice (X-10) rather than an oversight — `tests/` is intentionally included in the published
package (`Cargo.toml:74-82`) for exactly this reason.

That said, it is still a real architectural smell worth flagging: (1) it makes `tests/helpers/`
— normally free to refactor without touching `src/` — a de facto part of the library's
compilation unit, so a `cargo test`-motivated change to `e2e_fixtures.rs` (e.g. adding a
`#[cfg(test)]`-only helper, or a `dev-dependencies`-only import) can silently break the `cli`
feature build with no obvious signal that `src/` was affected; and (2) it embeds test-fixture
graph-building logic (crash-resume/approval-gate/map-reduce fixtures) into the shipped CLI
binary's dependency closure, which is unusual for production code and easy to miss in a future
audit of "what does the CLI actually ship."

**Fix:** No urgent action required given the explicit design rationale, but consider either (a)
extracting the shared graph builders into a proper library module (e.g.
`paladin_eval::fixtures` or a new small crate) that both `tests/` and `src/application/cli`
depend on normally, removing the `#[path]` reach-across entirely, or (b) adding a CI check that
fails if `tests/helpers/e2e_fixtures.rs` is edited without also touching
`src/application/cli/commands/eval.rs` in the same commit, so the coupling stays visible.

## Info

### IN-01: `run_before`/`run_after` can double-emit a `MiddlewareEvent` if a future middleware sets a hint AND returns `Finish`/`Fail`

**File:** `src/application/services/paladin/middleware/chain.rs:65-107`

**Issue:** `emit_pending_hint(cx, middleware.name())` is called unconditionally right after every
`before_model`/`after_model` invocation (lines 69, 94), regardless of the returned
`MiddlewareFlow`. If that same call's flow is `Finish` or `Fail`, a second, explicit
`cx.emit_middleware_event(middleware.name(), MiddlewareAction::Finish|Fail)` is then also emitted
(lines 73, 81, 100, 104). No current middleware sets `middleware_action_hint` in the same call
where it also returns `Finish`/`Fail`, so this is not triggered today, but nothing in the type
system or a runtime assertion prevents a future middleware from doing both and silently producing
two `MiddlewareEvent` records for one decision.

**Fix:** Either assert `cx.middleware_action_hint.is_none()` when the flow is `Finish`/`Fail` (to
catch the double-emission at the point a future middleware introduces it), or have
`emit_pending_hint` skip emission when the driver is about to emit its own structural event for
the same call.

---

_Reviewed: 2026-09-09T12:30:43Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
