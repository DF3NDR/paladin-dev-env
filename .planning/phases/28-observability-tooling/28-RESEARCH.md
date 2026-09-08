# Phase 28: Observability & Tooling - Research

**Researched:** 2026-09-08
**Domain:** Rust distributed-tracing instrumentation (trace event model, OpenTelemetry OTLP export, Mermaid/DOT graph visualization, a scripted-LLM regression-test harness)
**Confidence:** HIGH — every one of CONTEXT.md's 16 flagged open questions was resolved against direct `git`/`grep`/`cargo info`/docs.rs evidence, not inference. Two corrections to CONTEXT.md's own text are recorded in Metadata.

## Summary

Phase 28 is fully decided in `28-CONTEXT.md` (D-01…D-41) — this research does not re-litigate those decisions. Its job was the 16 explicit open questions CONTEXT.md left to research, each requiring a concrete, cited answer before planning starts. All 16 are answered below with file:line citations or command output. Two are worth flagging up front because they correct CONTEXT.md's own phrasing:

1. **The next free ADR number is `0048`, not `0047`.** `PROMOTION.md:69` reads "Next free ADR number: 0048" (Phase 16 consumed 0047). D-27's ADR must be named `0048-paladin-eval-composition-crate.md`.
2. **`crates/doc-examples`'s package is `paladin-doc-examples`, `publish = false`.** It is valid precedent for a composition crate depending on six workspace crates at once, but it is *not* a publishing precedent — D-27's "published" decision for `paladin-eval` stands on its own reasoning, not on this crate's example.

The OTel dependency triple (`opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp` at `0.32.x`) is current, Apache-2.0, MSRV 1.75 (well under the workspace's 1.88 floor), and passed the package-legitimacy gate clean on crates.io. `libtest-mimic` 0.8.2 and `glob` 0.3.4 are equally clean, MIT OR Apache-2.0, low-risk, mature crates. The workspace has **no existing `harness = false` `[[test]]` target** (only `[[bench]]` ones use it) — the eval-harness `[[test]] harness = false` target D-32 specifies is a first, but `cargo llvm-cov`'s source-based instrumentation and `--test-threads` argument-forwarding both work transparently through it, so the coverage floor and CI's existing invocation need no change.

**Primary recommendation:** Treat CONTEXT.md as the plan's decision spec; use this document only to fill in the sixteen named research gaps and the pitfalls this pass surfaced (`NodeContext`'s derived `PartialEq` vs. an `Arc<dyn TraceEmitter>` field; `WaypointRetentionService`'s single-port constructor needing a second field, not a rewrite; `paladin-web`'s currently feature-less `Cargo.toml`).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| Trace event model, `seq` stamping, `TraceEmitter`/`TraceDispatcher` | Application/Engine (`paladin-battalion`) + Core value types | Ports (`paladin-ports` re-export) | ADR-0016: core owns port value types, ports re-export; the engine is the one `seq` authority (D-03) |
| Log sink, OTel sink, `run_traces` persistence | Infrastructure (facade `src/infrastructure/telemetry/`, `paladin-storage`) | — | Adapters implementing `TraceSink`/`RunTracePort`; never in core or ports (X-01) |
| SSE bridge collapse (`map_trace_event`) | Application (`src/application/services/run/events.rs`) | — | Already the facade's `TraceSink` adapter; this phase completes its match, doesn't relocate it |
| Graph/overlay export (Mermaid/DOT) | Application/Engine (`paladin-battalion::engine::export`) | CLI (`src/application/cli/commands/{graph,run}.rs`) | Pure functions over `GraphShape`, called in-process by the CLI (ADR-0023: no HTTP client in the CLI) |
| `dev-ui` inspector page | Infrastructure/Web (`paladin-web`) | Application (`RunInspectorPort` impl in `src/application/services/run/inspector.rs`) | ADR-0031: `paladin-web` never depends on `paladin-battalion`; a core-typed input port is the only channel in |
| `paladin-eval` scenario runner, assertions, CLI `eval run` | New composition crate (`crates/paladin-eval`) + CLI (`src/application/cli/commands/eval.rs`) | — | Depends downward on `paladin-core`/`ports`/`battalion`/`llm`/`storage`; nothing depends on it except as a dev-dependency (D-27) |

## Package Legitimacy Audit

Ran `gsd-tools query package-legitimacy check --ecosystem crates …` against every new dependency this phase introduces (the tool has no npm/PyPI relevance here — Rust ecosystem = `crates`, confirmed working after an initial `--ecosystem npm` miscall returned false `SLOP`/`SUS` verdicts because it queried the wrong registry).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---|---|---|---|---|---|---|
| `opentelemetry` | crates.io | since 2019-11-08 | 4.34M/wk | github.com/open-telemetry/opentelemetry-rust | OK | Approved |
| `opentelemetry_sdk` | crates.io | since 2019-06-27 | 3.94M/wk | github.com/open-telemetry/opentelemetry-rust | OK | Approved |
| `opentelemetry-otlp` | crates.io | since 2020-08-14 | 2.99M/wk | github.com/open-telemetry/opentelemetry-rust | OK | Approved |
| `libtest-mimic` | crates.io | since 2018-07-23 | 374.7k/wk | github.com/LukasKalbertodt/libtest-mimic | OK | Approved |
| `glob` | crates.io | since 2014-11-11 | 9.56M/wk | github.com/rust-lang/glob | OK | Approved |

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none. (A first pass with `--ecosystem npm` mis-flagged `opentelemetry_sdk`/`opentelemetry-otlp`/`libtest-mimic` as `does-not-exist` and `opentelemetry` as `no-repository` — an npm-registry lookup of Rust crate names, a wrong-ecosystem false positive, not a real finding. Re-run at `--ecosystem crates` is the authoritative result above.) `serde_yaml` is already an optional dependency of the root crate (`Cargo.toml:210`, gated by `cli`) — D-28's new consumer of it is not a new dependency.

All five crates additionally clear `cargo view`/`cargo info` MSRV checks well under the workspace's 1.88 floor (`opentelemetry*` family: 1.75; `libtest-mimic`: 1.65; `glob`: 1.63) and are MIT OR Apache-2.0 / Apache-2.0, both already in `deny.toml`'s `[licenses] allow` list — no `deny.toml` edit needed.

## The 16 Open Questions — Verified Answers

### 1. OpenTelemetry crate versions, `InMemorySpanExporter`, `SpanBuilder::with_start_time`/`end_with_timestamp`, no-redirect reqwest client

Verified via `cargo info` (network-enabled in this environment) and docs.rs (`[CITED: docs.rs]`):

| Crate | Version | License | MSRV | Default features |
|---|---|---|---|---|
| `opentelemetry` | `0.32.0` | Apache-2.0 | 1.75.0 | `trace, metrics, logs, internal-logs, futures` |
| `opentelemetry_sdk` | `0.32.1` | Apache-2.0 | 1.75.0 | `trace, metrics, logs, internal-logs` |
| `opentelemetry-otlp` | `0.32.0` | Apache-2.0 | 1.75.0 | `http-proto, reqwest-blocking-client, trace, metrics, logs, internal-logs` |

D-12 specifies `http-proto` + `reqwest-client` (the **async** reqwest client), **not** the default `reqwest-blocking-client` — so the dependency line must be `default-features = false, features = ["http-proto", "reqwest-client", "trace"]` (plus whatever `logs`/`metrics` the span-only sink actually needs; D-12's span model needs `trace` only). All three crates clear `cargo deny`'s Apache-2.0 allowlist entry trivially (already present) and the 1.88 MSRV floor with 13 versions of headroom.

- **`InMemorySpanExporter`** — `opentelemetry_sdk::testing::trace::InMemorySpanExporter` (+ `InMemorySpanExporterBuilder`), gated by the `testing` cargo feature `[CITED: docs.rs/opentelemetry_sdk/0.32.1]`. D-13(a)'s shape test needs `opentelemetry_sdk = { features = ["testing"] }` under `[dev-dependencies]` (or a `#[cfg(test)]`-only feature union) — this is a *dev-only* need, so it must not leak into the `otel` production feature's dependency set.
- **`SpanBuilder::with_start_time`** — `pub fn with_start_time<T: Into<SystemTime>>(self, start_time: T) -> Self` on `opentelemetry::trace::SpanBuilder` `[CITED: docs.rs/opentelemetry/0.32.0]`. Confirmed present, matches D-12's span model exactly.
- **`Span::end_with_timestamp`** — `fn end_with_timestamp(&mut self, timestamp: SystemTime)`, a **required** method of the `opentelemetry::trace::Span` trait (alongside a provided `end(&mut self)` using current time) `[CITED: docs.rs/opentelemetry/0.32.0]`. Confirmed present.
- **No-redirect reqwest client** — `opentelemetry-otlp`'s HTTP exporter supports `SpanExporter::builder().with_http().with_http_client(custom_client)`, where `custom_client` implements `opentelemetry_http::HttpClient` (a `reqwest::Client` satisfies this when the `reqwest-client` feature is on) `[CITED: docs.rs/opentelemetry-otlp/0.32.0]`. Build the client with `reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build()` and pass it through `with_http_client` — the same house pattern every LLM adapter's own client and the webhook client use (`.github/instructions/security.instructions.md`'s "no redirects" rule).

### 2. `MockLlmAdapter` / `MockScriptEntry` pre-existing at `v0.9.0`?

```
git cat-file -e v0.9.0:crates/paladin-llm/src/mock.rs   → EXISTS
git show v0.9.0:crates/paladin-llm/src/mock.rs | grep 'pub struct\|MockScriptEntry'
   → pub struct MockLlmAdapter (73), pub struct MultiStepMockLlmPort (299)
   → NO MockScriptEntry, NO with_script
```

**`MockLlmAdapter` is pre-existing at `v0.9.0`.** `MockScriptEntry` and `with_script` are **not** — they are new since `v0.9.0` (current `crates/paladin-llm/src/mock.rs:34,243`), already added by a later phase without an X-10 register row (correctly: additive new-in-0.10 API on a pre-existing type is fine as long as no existing method/field is altered — `with_script` is a new method, not a signature change). This means D-30's "no X-10 register row needed" claim is confirmed for `paladin-eval` leaving `mock.rs` untouched, **and** it confirms `MockLlmAdapter` already has a mid-0.10-cycle precedent for growing new script-oriented methods without a register row — so if a future phase needed to extend it further the bar is "new method, no changed signature," not "never touch it."

### 3. Next free migration numbers; migration-run mechanism

```
ls crates/paladin-storage/migrations/{sqlite,postgres}/
   → 001_create_waypoints_table.sql … 005_create_webhook_deliveries_table.sql (both backends, in lockstep)
```

**Next free number: `006`** for both backends — `006_create_run_traces_table.sql` under `crates/paladin-storage/migrations/sqlite/` and `.../postgres/`. Mechanism confirmed identical across every existing adapter (`waypoint/sqlite.rs:95`, `run/sqlite.rs:91`, `assistant/sqlite.rs:74`, `webhook/sqlite.rs:58`, `run_schedule/sqlite.rs:61`, and their `postgres.rs` siblings): a `static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite")` (or `"migrations/postgres"`), embedded at compile time, `MIGRATOR.run(&pool).await` called automatically on adapter construction. `run_trace`'s adapters follow this pattern verbatim, one new numbered file per backend, no new mechanism.

### 4. `libtest-mimic` version/MSRV/license; existing `harness = false` `[[test]]` precedent; `cargo llvm-cov` behavior on it

- `libtest-mimic` **0.8.2**, MIT/Apache-2.0, MSRV 1.65 `[VERIFIED: cargo info + package-legitimacy check]`.
- **No existing `harness = false` `[[test]]` target anywhere in the workspace.** `grep -n "harness = false" Cargo.toml crates/*/Cargo.toml` returns exactly six hits, and every one is a `[[bench]]` target (`config_benchmarks`, `engine_benchmarks` at the root; similar in `paladin-memory`, `paladin-llm`, `paladin-battalion`). The `[[test]] name = "evals" harness = false` target D-32 specifies is genuinely new shape for this workspace, not a copy of an existing pattern — the planner should budget a small amount of extra care (there is no in-tree example to crib from for the `Cargo.toml` `[[test]]` stanza itself, though it is a well-documented Cargo feature).
- **`cargo llvm-cov` and coverage floor:** `scripts/coverage.sh` (shared by CI's `coverage` job and `make coverage`) runs `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1`. No `--lib`/`--tests`-only restriction — the default `cargo llvm-cov`/`cargo test` behavior builds and runs **every** target in the workspace, including `[[test]]` integration targets regardless of harness, because source-based coverage instrumentation (`-C instrument-coverage`) is a compile-time property, orthogonal to which test harness the resulting binary uses at runtime. The `-- --test-threads=1` trailing arg is forwarded to every test binary's own arg parser; `libtest-mimic`'s `Arguments::from_args()` is deliberately built to accept the same CLI surface as the built-in harness (its own crates.io description: "behaves like the built-in test harness"), so `--test-threads=1` is a no-op it accepts rather than an unrecognized-flag failure. **Conclusion: the `evals` test target counts toward the 82% floor with zero CI changes required**, and every scenario file executed under it is instrumented like any other test.

### 5. `openapi_matches_committed_baseline` feature independence; `sdk-clients` CI job

- `crates/paladin-web/src/openapi.rs:264` — the drift-guard test builds `openapi_spec()` from exactly `agent_controller::agent_router`/`versioned_agent_parts`, `run_controller::versioned_run_parts`, `thread_controller::versioned_thread_parts` (`openapi.rs:1-25`). The `dev_ui_controller` D-25 introduces is not one of these three merged sources and carries no `#[utoipa::path]` annotation by design — so the golden JSON this test blesses is structurally incapable of including the `dev-ui` route regardless of whether the `dev-ui` feature is compiled in. **D-25's "feature-independent golden" claim is confirmed by the assembly function's own source, not just by the annotation's absence.**
- `.github/workflows/ci.yml:1077-1125` — the `sdk-clients` job reads `crates/paladin-web/openapi.json` **directly off disk** (the committed file), feeding it to `openapi-generator-cli` for both Python and TypeScript client generation, then smoke-tests the generated clients. It reacts automatically to any committed change in that file — **D-15's additive `trace_seq` field and D-16's `replay` mode need only a `UPDATE_OPENAPI=1` re-bless commit; the `sdk-clients` job needs no separate edit,** it will pick up the new field/enum value on its next run against the updated baseline.

### 6. `NodeSpec` variants, worker-template/deferred storage, `NodeKindDoc`

`crates/paladin-battalion/src/engine/graph.rs:42-160` — `NodeSpec` (`#[non_exhaustive]`) has exactly four variants: `Paladin { paladin, input_template, output_field, directive_parser, output_schema }`, `Function(Arc<dyn StateNode>)`, `Battalion { graph, state_map, restart_on_resume }` (the "Workflow"/subgraph case), `Gate { request, output_field }`. **There is no separate `WorkerTemplate` `NodeSpec` variant** — a worker template is any `NodeSpec` (in practice `Paladin` or `Function`) whose `NodeId` is separately registered in `WarGraph`'s private `worker_templates: HashSet<NodeId>` field (`graph.rs:547`, populated via `WarGraph::add_worker_template`, queried via `WarGraph::is_worker_template(&NodeId) -> bool`, `graph.rs:660-668`). Similarly, "deferred" is not a `NodeSpec` variant but a second private `HashSet<NodeId>`-backed flag (`WarGraph::add_deferred_node` / `is_deferred`, confirmed by grep at `graph.rs:534-602`). **This directly matches D-18's `ShapeNode { kind, deferred: bool, worker_template: bool, … }` design** — `GraphShape::from_graph` must read `graph.is_worker_template(id)` and `graph.is_deferred(id)` as two independent boolean lookups per node, not decode them from `NodeSpec` itself.

`NodeKindDoc` (`crates/paladin-battalion/src/engine/graph_doc.rs:335-342`) has exactly three variants — `Paladin(PaladinNodeDoc)`, `Gate(GateNodeDoc)`, `Workflow(WorkflowNodeDoc)` — confirming 27-CONTEXT D-33's `{Paladin, Gate, Workflow}` scope exactly, no `Function`, no worker-template kind. `NodeDoc` (`graph_doc.rs:147-168`) additionally carries a **`pub defer: bool`** field directly on each node entry — so `GraphShape::from_doc(&WarGraphDoc)` can set `deferred` straight from `NodeDoc.defer` with no derivation, while `worker_template` is always `false` from a doc (worker templates are not expressible in `WarGraphDoc`, matching D-18's own rationale for why `from_graph` must exist alongside `from_doc`).

`WarGraph`'s edges are `EdgeSpec { from: NodeId, to: NodeId, condition: Option<EdgeCondition> }` (`graph.rs:451`) — `EdgeCondition` reused from the legacy Campaign vocabulary (`Always | Contains | Regex | Custom`), exactly matching D-19's rendering-rule cases.

### 7. `EdgeEvaluated`/`ParleyRaised`/`Heartbeat`/`TraceEvent` emission sites

| Producer | Confirmed site |
|---|---|
| `EdgeEvaluated` | `crates/paladin-battalion/src/engine/superstep.rs:3917` calls `evaluate_edge_condition(...)` (the function itself at `superstep.rs:4293-4316`, matching `EdgeCondition::{Always,Contains,Regex,Custom}`) — the trace emit belongs immediately around this one call site. |
| `ParleyRaised` | `crates/paladin-battalion/src/engine/superstep.rs:3586` — the `return Ok(RunOutcome::AwaitingInput { .. })` construction site (the "Parley arm" `mod.rs`'s own docs at line 636-637 point at). |
| `NodeProgress::Heartbeat` | `crates/paladin-battalion/src/engine/superstep.rs:405` — `async fn idle_or_pending(heartbeat: &HeartbeatHandle, idle: Option<Duration>)`, the idle-timer receiver that already awaits `HeartbeatHandle`'s `watch::Receiver::changed()`. This is the natural rate-limiting point per D-04's "one heartbeat event per `heartbeat_interval` per node." |
| `TraceEvent::RunStarted`/`SuperstepStarted`/etc. — existing emit sites | `crates/paladin-battalion/src/engine/mod.rs` — `self.trace_dispatcher.emit(...)` calls at lines 1831, 1861, 1950, 1953, 2026, 2055, 2066, 2214, 2327, 2358, 2383, 2404, 2517, 2538, 2559 (the last, line 2559, is the existing `RunFinished` emit — D-07's `trace_dropped_total` stamping and D-02's `status` field both attach here). `with_trace_sink` at `mod.rs:1587-1589`; `trace_dispatcher` field at `mod.rs:1422`. |
| `RunOutcome` construction (source for `RunFinished.status`) | `crates/paladin-battalion/src/engine/mod.rs:279-319` — `pub enum RunOutcome { Completed{..}, AwaitingInput{..}, Halted{..}, Failed{..} }` (line numbers drifted ~7 lines from CONTEXT's "272-312" citation, same shape, same order — confirmed current). |

### 8. Worker's direct `parley`/`done`/`error` publish sites; per-run `TraceEmitter`/engine wiring

`/workspace/src/application/services/run/worker.rs:850-903` — inside `run_once`'s dispatch-completion handling, a `match &outcome { RunOutcome::AwaitingInput{..} => bus.publish(..Parley..), RunOutcome::Completed{..} => ..Done.., RunOutcome::Halted{..} => ..Done.., RunOutcome::Failed{..} => ..Error.. }` (four arms, matching D-14's claim of the three-wire-name direct-publish sites the worker owns — `Completed` and `Halted` both map to `done` with a different `status` payload field, so it is three *wire kinds* from four *outcome arms*). `record_engine_failure` (`worker.rs:~1050-1067`) has its own, separate `bus.publish(..Error..)` for an `EngineError` outside normal `RunOutcome` reporting.

Per-run engine/emitter construction: `worker.rs:650-770`, inside `run_once`, when `self.engine_factory` is `Some`, a fresh `WarEngine` is built per run (`let mut engine = factory(child_token)`), then conditionally `.with_cancellation_probe(...)` and, when `self.event_bus` is `Some`, **`engine = engine.with_trace_sink(Arc::new(RunEventBusSink::new(Arc::clone(bus))))`** (`worker.rs:764`) — this is the exact per-run composition point D-03's `TraceEmitter` handle must be pulled from (`WarEngine::trace_emitter()`) and handed to `FallbackLlmAdapter::with_trace_emitter` and the middleware chain for that run.

### 9. Is `RunStreamMode` `#[non_exhaustive]`? External `match`?

`crates/paladin-core/src/platform/container/run.rs:508-517` — **`RunStreamMode` is currently NOT `#[non_exhaustive]`** (only `RunStreamEventKind` at line 465-469 and `RunStreamEvent` at line 524-527 carry that attribute; `RunStreamMode`'s two-line block above it has none). D-16 adding a `Replay` variant is therefore a genuine breaking addition to an existing enum unless the same change also adds `#[non_exhaustive]` in the same commit (X-10.2's rule: "every pre-existing public enum that gains a variant MUST be marked `#[non_exhaustive]` in the same change, unless unreasonable"). Since `RunStreamMode` is itself new-in-0.10 (Phase 27, absent at `v0.9.0`), this is a deliberate-zero `MIGRATION.md` note (D-39), not an X-10 register row — but the `#[non_exhaustive]` marker should still be added defensively per X-10.2's spirit (CONTEXT.md's own text already says so: "marked `#[non_exhaustive]` in the same change"). `grep -rn "match.*RunStreamMode\|RunStreamMode::"` across the tree (outside `run.rs` itself) shows the only consumers are `worker.rs` and `events.rs`, both first-party and both already planned for editing in this phase — no external-crate consumer risk.

### 10. `schemars` version; `graph_doc_round_trip.rs` bless env var

`schemars = "1.2"` — pinned identically in both `Cargo.toml` (root, line 152) and `crates/paladin-battalion/Cargo.toml` (line 45), confirmed already resolved via the `rmcp 2.1.0`/`schemars 1.2.1` chain (Phase 26 D-26's own comment), so D-28's eval-schema `schemars` derive adds zero new dependency versions. Bless idiom: `crates/paladin-battalion/tests/graph_doc_round_trip.rs:104-122` — env var **`UPDATE_WARGRAPH_SCHEMA=1`**, checked via `std::env::var_os("UPDATE_WARGRAPH_SCHEMA").is_some()`, regenerating `docs/schemas/wargraph-doc.schema.json` (repo-root-relative, computed via `CARGO_MANIFEST_DIR` + `/../../docs/schemas/...`). D-28's eval-scenario schema should mirror this exactly: its own env var name (e.g. `UPDATE_EVAL_SCHEMA=1`), same `CARGO_MANIFEST_DIR`-relative path-construction pattern, same golden location convention (`docs/schemas/eval-scenario.schema.json`).

### 11. Mermaid 11 CDN URL and minimal ESM init

`[CITED: WebSearch, cross-checked against jsdelivr.com/mermaid package page]`: ESM bundle URL is `https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs`. Minimal inline init from a `<script type="module">`:

```html
<script type="module">
  import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';
  mermaid.initialize({ startOnLoad: true });
  // or, for explicit control after DOM injection of the diagram text:
  // await mermaid.run({ querySelector: '.mermaid' });
</script>
```
`startOnLoad: true` auto-renders any `<pre class="mermaid">`/`<div class="mermaid">` present at parse time; `mermaid.run()` is the imperative form for content injected after initial load (relevant if the inspector page ever needs to re-render after a client-side update — not required for D-26's static-embed design, but the two-line difference is cheap to note for the discretion item on "live SSE attach," which is deferred). `web_server.dev_ui.mermaid_url` (D-36) should default to the ESM URL above.

### 12. `FallbackLlmAdapter::with_trace_sink` callers; middleware chain files

`crates/paladin-llm/src/fallback.rs:156-157` — `pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self` is the **only** entry point setting `self.trace_sink` (confirmed: `trace_sink: Option<Arc<dyn TraceSink>>` field at line 117, set exactly once at construction, read at line 180 inside the hop-emission path). Its production/test callers (`grep -n "with_trace_sink" fallback.rs`) are all inside its own `#[cfg(test)]` module (`fallback.rs:441,480,599,644,730`) — **no production call site in the facade wires a sink into `FallbackLlmAdapter` today**; that wiring is entirely new work for this phase (the per-run composition point identified in Q8 above is where `.with_trace_emitter(...)` — the D-03 rename — must be added for the first time in production code, not merely renamed from an existing call).

Middleware chain: `src/application/services/paladin/middleware/` contains `chain.rs, context.rs, guardrail.rs, history.rs, limits.rs, mod.rs, resilience.rs, summarization.rs, tool_protocol.rs, vault_recall.rs` — ten files, confirming 26-CONTEXT D-01's placement. `MiddlewareEvent` producers (`Finish`/`Deny`/`Redact`/`Fail`/`Retry`/`Fallback` per D-04's closed-enum action set) map onto this file set; `chain.rs` is the most likely single emission point (the chain-execution loop), with `resilience.rs` (retry/fallback) and `guardrail.rs` (deny) as the other natural candidates — exact file split is Claude's Discretion per CONTEXT.md, this research only confirms the file set exists and matches the D-01 citation.

### 13. `bench_superstep_cost` structure; criterion precedent as phase evidence

`benches/engine_benchmarks.rs:35` — `use criterion::{BatchSize, Criterion, criterion_group, criterion_main};`; `fn build_width_graph(width: usize) -> WarGraph` at line 215 (the fixture-graph builder D-37's `none`/`log_sink`/`composite` sink variants parametrize against); `fn bench_superstep_cost(c: &mut Criterion)` at line 243; `criterion_group!(engine_benches, bench_waypoint_save, bench_superstep_cost); criterion_main!(engine_benches);` at lines 275-276. D-37's three sink variants are new `Criterion::bench_function` calls inside (or parametrized benchmark groups alongside) the existing `bench_superstep_cost`, reusing `build_width_graph` unchanged. **No existing in-tree precedent was found for criterion numbers being recorded as phase evidence in a prior phase's SUMMARY/VALIDATION artifact** (`grep` across `.planning/phases/*/*-VALIDATION.md` for "criterion" turned up nothing phase-specific) — D-37's "the record is the gate, not CI" instruction is itself the first such precedent this phase sets, not a pattern being followed from an earlier one. Plan accordingly: capture the criterion output (e.g. `critcmp`-style before/after numbers, or a plain `cargo bench -p paladin-ai --bench engine_benchmarks -- bench_superstep_cost` transcript) into the phase's own verification evidence file.

### 14. `WaypointRetentionService` / `paladin_storage::waypoint::retention::prune` signatures — where D-17's `prune_thread` attaches

`src/application/services/waypoint_retention.rs:67-98`:
```rust
pub struct WaypointRetentionService {
    port: Arc<dyn WaypointPort>,
    config: WaypointRetentionConfig,
}
impl WaypointRetentionService {
    pub fn new(port: Arc<dyn WaypointPort>, config: WaypointRetentionConfig) -> Self { .. }
    pub async fn prune(&self) -> Result<PruneReport, WaypointError> { .. }
}
```
`crates/paladin-storage/src/waypoint/retention.rs:104-109`:
```rust
pub async fn prune(
    port: &dyn WaypointPort,
    max_age_days: Option<u32>,
    max_waypoints_per_thread: Option<u32>,
    protected: &dyn Fn(&ThreadId, &[WaypointSummary]) -> HashSet<WaypointId>,
) -> Result<PruneReport, WaypointError> { .. }
```
**Pitfall flagged, not just a citation:** `WaypointRetentionService` is hard-typed to exactly one `Arc<dyn WaypointPort>`, and the free function `prune` takes `&dyn WaypointPort` directly — neither can be reused unchanged for `run_traces`, since `RunTracePort` is a distinct trait with a distinct `prune_thread` signature and a distinct error type (`RunTraceError`, not `WaypointError`). D-17's "one config, one routine, two ports" is achievable, but concretely means: (a) add an optional second field `run_trace_port: Option<Arc<dyn RunTracePort>>` to `WaypointRetentionService` via a new `with_run_trace_port` builder method (additive, `new`'s existing two-arg signature is untouched, so no call-site break); (b) `prune()` calls the existing Waypoint `prune` free function *and*, when the second port is set, a new sibling free function (or a direct `RunTracePort::prune_thread` call) in `paladin-storage::run_trace::retention`-equivalent code, passing the *same* `config.max_age_days`/`config.max_waypoints_per_thread` values into both. This is a two-call, one-config change, not a new abstraction over both ports.

### 15. `paladin-web` feature list; umbrella feature-forwarding pattern; CI feature-flags matrix

**`crates/paladin-web/Cargo.toml` currently has no `[features]` section at all** (`grep -n "^\[features\]"` returns nothing) — every capability in that crate compiles unconditionally today. D-25's `dev-ui` feature on `paladin-web` is the **first** cargo feature that crate will ever declare; the planner must add a `[features]` section from scratch, not extend an existing one.

Umbrella feature-forwarding pattern (root `Cargo.toml:406-464`): every optional adapter is `umbrella-flag = ["dep:crate-or-nothing", "leaf-crate/feature-name"]`, explicitly *not* rolled into `default` or `full` for heavy/exotic adapters, with an inline comment citing X-11.4 (e.g. `storage-postgres = ["paladin-storage/postgres"]` at line 447, `redis-cache` at line 454, both explicitly excluded from `full`'s member list at line 460). `otel` and `dev-ui` should follow this identically: `otel = ["dep:opentelemetry", "dep:opentelemetry_sdk", "dep:opentelemetry-otlp"]` and `dev-ui = ["paladin-web/dev-ui"]`, both absent from `default` and from `full`'s explicit member list (line 460's `full = [...]` — currently 12 members, none of which are `storage-postgres`/`redis-cache`; `otel`/`dev-ui` join that same exclusion list).

CI feature-flags matrix (`.github/workflows/feature-flags.yml:36-99`) enumerates 14 legs today (`no-default-features`, `default`, `all-features`, `full`, five `llm-*`, `web-server`, `content-processing`, `notifications`, `vision`, `redis-queue`, `s3-storage`, `cli`) — **`otel` and `dev-ui` are absent** and must be added as two new `- name:` entries (each `--no-default-features --features otel` / `--no-default-features --features dev-ui`), matching D-40's close-out gate list exactly.

### 16. Release/semver/api-surface/sdk crate-list enumeration points for `paladin-eval`

Every place the eleven publishable crates are named, verified by direct read:

| File | What it enumerates | `paladin-eval`'s required change |
|---|---|---|
| `scripts/publish-crates.sh:145-155` | Hardcoded 11-crate **dependency-order** array (`paladin-ai-core` → `paladin-ports` → `paladin-herald` → `paladin-battalion` → `paladin-llm` → `paladin-memory` → `paladin-web` → `paladin-notifications` → `paladin-content` → `paladin-storage` → `paladin-ai`) | Insert `paladin-eval` after `paladin-storage` (its heaviest runtime dependency) and before `paladin-ai` — `paladin-ai` takes it only as a dev-dependency, so ordering relative to `paladin-ai` is not registry-load-bearing the way it is for a real `[dependencies]` edge, but placing it before `paladin-ai` in the list keeps the file's stated invariant ("every dependency already on the registry before its dependent publishes") trivially true either way. |
| `.github/workflows/ci.yml:330-343` (`semver` job) | Hardcoded 11-package list fed to `cargo semver-checks check-release --package "${pkg}" --baseline-version 0.9.0` | **Do NOT add `paladin-eval` here** — D-40 is explicit that the new crate has no `0.9.0` baseline to diff against; `cargo semver-checks` would error on a nonexistent baseline. Record the omission's reason inline in the job (a comment) rather than silently excluding it. |
| `Makefile:547-557` (`publish-dry-run` target) | Ten `cargo publish --dry-run -p <crate>` lines + root `paladin` (note: **`paladin-herald` is itself already missing from this target** — a pre-existing gap, not introduced by this phase, not this phase's to fix) | Add a `cargo publish --dry-run -p paladin-eval \|\| true` line, positioned after `paladin-storage`. |
| `scripts/extract-public-api.sh` / `scripts/check-api-surface.sh` | **No hardcoded per-crate list** — `cargo public-api` is invoked at the workspace level with no `-p` filter, so a new workspace member under `crates/*` (the `[workspace] members` glob already covers it) is picked up automatically the next time `.project/current-exports.txt` is regenerated. | No script edit needed; just re-run `./scripts/extract-public-api.sh .project/current-exports.txt` once `paladin-eval`'s public surface exists, per the existing X-10/X-11 gate-list habit (Phase 25/26 already learned this the hard way — see STATE.md's Phase 25 carried concern). |
| `.github/workflows/ci.yml:1077` (`sdk-clients` job) | Reads `crates/paladin-web/openapi.json` only — no crate enumeration | Not affected by `paladin-eval` at all (it has no HTTP surface). |

## Standard Stack

### Core (new dependencies this phase introduces)

| Library | Version | Purpose | Why Standard |
|---|---|---|---|
| `opentelemetry` | `0.32.0` | OTel API types (`SpanBuilder`, `Span`, `TraceId`) | Official Rust OTel SDK, the only serious option for OTLP export |
| `opentelemetry_sdk` | `0.32.1` | OTel SDK (`TracerProvider`, `InMemorySpanExporter` for tests) | Same family; required companion to `opentelemetry` |
| `opentelemetry-otlp` | `0.32.0`, `default-features = false, features = ["http-proto", "reqwest-client", "trace"]` | OTLP/HTTP protobuf exporter | D-12 explicitly rejects the `tonic`/gRPC transport (doubles dependency graph for an unrequested transport) |
| `libtest-mimic` | `0.8.2` | Custom `harness = false` test runner mimicking built-in `cargo test` CLI/output | D-32's exact requirement: `Trial`-per-case dynamic test registration without a proc macro |
| `glob` | `0.3.4` | `evals/**/*.eval.yaml` runtime pattern matching | Standard, minimal, already the de facto choice for glob patterns in Rust |

### Supporting (already resolved in the workspace, zero new versions)

| Library | Version | Purpose | When to Use |
|---|---|---|---|
| `schemars` | `1.2` | JSON Schema derivation for the eval-scenario format (D-28) | Already pinned identically in root + `paladin-battalion` `Cargo.toml`; zero new dependency graph cost |
| `serde_yaml` | `0.9` (already optional, `cli`-gated) | `.eval.yaml` parsing | D-28's new consumer, not a new dependency |
| `insta` | `1.34` (already a dev-dependency) | Snapshot-testing assertion-failure renderings (D-29) | Already used by `tests/cli/*` |
| `sqlx` | `0.8` (workspace pin) | `run_traces` table migrations | Already the storage layer's SQL toolkit |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|---|---|---|
| `opentelemetry-otlp`'s `reqwest-client` (async) | `reqwest-blocking-client` (the crate's own default) | Blocking client would require a `tokio::task::spawn_blocking` wrapper inside an async engine context — needless friction. D-12 already rejects this implicitly by naming `reqwest-client` explicitly. |
| `opentelemetry-otlp` HTTP/protobuf | `tonic`/gRPC transport | D-12 explicitly rejects: roughly doubles the dependency graph for a transport nobody asked for. |
| `libtest-mimic` custom harness | A `proc_macro` crate reading scenario files at compile time | D-32 explicitly rejects: a second crate to publish, recompiles on every scenario-file edit, fragile glob expansion inside `proc_macro`. |
| `libtest-mimic` custom harness | `macro_rules!` generating one `#[test]` per **file** | D-32 explicitly rejects: fails OBS-FR-13's "one test per case," not per file. |

**Installation:**
```bash
# Root Cargo.toml [dependencies], all `optional = true`, gated by the new `otel` feature:
cargo add opentelemetry@0.32.0 --optional
cargo add opentelemetry_sdk@0.32.1 --optional
cargo add opentelemetry-otlp@0.32.0 --no-default-features --features http-proto,reqwest-client,trace --optional

# crates/paladin-eval/Cargo.toml [dependencies]:
cargo add libtest-mimic@0.8.2 -p paladin-eval
cargo add glob@0.3.4 -p paladin-eval
```

**Version verification (already performed, not deferred to plan-time):** all five versions above confirmed current via `cargo info <pkg>` against the live crates.io index on 2026-09-08; no newer patch was pending at check time. `deny.toml`'s license allowlist needs zero edits (`Apache-2.0`, `MIT`, `MIT OR Apache-2.0` are all already allowed).

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────────────────┐
                    │                     WarEngine (per-run)                  │
                    │                                                          │
  superstep loop ──▶│  TraceDispatcher::emit(TraceEvent)                      │
  (RunStarted,       │        │  stamps seq/at/thread_id at enqueue time       │
   NodeStarted,       │        ▼  (D-03: the ONE seq authority)                │
   EdgeEvaluated,      │  bounded queue (drop-oldest, counted, catch_unwind)   │
   NodeProgress,        │        │                                            │
   DeltaMerged, etc.)    │        ▼                                            │
                    │  CompositeSink.on_event(TraceRecord)  [paladin-ports]   │
                    └────┬──────────────┬──────────────┬──────────────┬──────┘
                         │              │              │              │
                    ┌────▼────┐   ┌─────▼─────┐  ┌─────▼──────┐ ┌────▼──────────┐
                    │ log_sink│   │ otel_sink │  │RunEventBus │ │PersistingSink │
                    │ (log::  │   │ (#[otel]  │  │Sink        │ │(run_traces,   │
                    │ info!,  │   │  feature, │  │(map_trace_ │ │ opt-in,       │
                    │ default-│   │  span-per-│  │event → SSE)│ │ batched per   │
                    │ on)     │   │  attempt) │  │            │ │ superstep)    │
                    └─────────┘   └───────────┘  └─────┬──────┘ └───────┬───────┘
                                                        │                │
                                                  GET /v1/runs/{id}/stream   run_traces table
                                                  (live | replay | degraded)  (sqlite/postgres,
                                                                               ENG-FR-18 retention)

  Producers BELOW the engine, through Arc<dyn TraceEmitter> (never a raw Arc<dyn TraceSink>):
  FallbackLlmAdapter (FallbackHop) ──┐
  Middleware chain (MiddlewareEvent) ├──▶ WarEngine::trace_emitter() ──▶ same TraceDispatcher
  PaladinExecutionService (NodeProgress::StreamChunk/ToolCall) ──┘         (stamped from the one counter)

  Export path (offline, pure functions, no live engine needed):
  WarGraphDoc ──▶ GraphShape::from_doc ─┐
  WarGraph    ──▶ GraphShape::from_graph┴─▶ to_mermaid / to_dot ──▶ paladin-cli graph export
                                          + ExecutionOverlay (Waypoints | run_traces) ──▶ run export / dev-ui page

  Eval harness (offline, scripted, no live LLM):
  evals/*.eval.yaml ──▶ ScenarioRunner (libtest-mimic Trial per case)
     │                        │
     │  WarGraphDoc.compile() │  registered graph constructor (EngineRegistries)
     └───────────┬────────────┘
                  ▼
      WarEngine + ScenarioLlm (LlmPort impl) + CapturingSink (TraceSink)
                  ▼
      assertions evaluated over (Vec<TraceRecord>, final Battlefield, RunOutcome)
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
└── trace.rs                    # TraceEvent, TraceRecord, FieldChange, TRACE_SCHEMA_VERSION (D-01)

crates/paladin-ports/src/output/
├── trace_sink_port.rs          # TraceSink, TraceSinkError, TraceEmitter, CompositeSink; re-exports core types
└── run_trace_port.rs           # RunTracePort (D-17)
crates/paladin-ports/src/input/
└── run_inspector_port.rs       # RunInspectorPort, InspectorView (D-24)

crates/paladin-battalion/src/engine/
├── hooks.rs                    # TraceDispatcher (extend: stamping, catch_unwind, stress test)
└── export/
    ├── mod.rs
    ├── shape.rs                 # GraphShape, ShapeNode, ShapeEdge, from_doc/from_graph
    ├── mermaid.rs                # to_mermaid, to_mermaid_overlay
    ├── dot.rs                    # to_dot
    └── overlay.rs                 # ExecutionOverlay

crates/paladin-storage/src/run_trace/
├── mod.rs
├── in_memory.rs
├── sqlite.rs
├── postgres.rs
└── contract_tests.rs
crates/paladin-storage/migrations/{sqlite,postgres}/006_create_run_traces_table.sql

src/infrastructure/telemetry/
├── log_sink.rs                 # default-on structured-log TraceSink
└── otel_sink.rs                # #[cfg(feature = "otel")] OTLP exporter TraceSink

src/application/services/run/
├── inspector.rs                 # RunInspectorPort impl (D-24)
└── events.rs                     # extend map_trace_event to seven cases (D-14)

crates/paladin-web/src/
├── dev_ui_controller.rs         # GET /v1/dev-ui/threads/{id} (#[cfg(feature = "dev-ui")])
└── dev_ui/inspector.html        # include_str!'d static template (D-26)

src/application/cli/commands/
├── graph.rs                      # paladin-cli graph export
├── run.rs                         # paladin-cli run export
└── eval.rs                         # paladin-cli eval run

crates/paladin-eval/                # NEW composition crate (D-27)
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── scenario.rs                 # serde scenario file format (D-28)
    ├── scripted_llm.rs             # ScenarioLlm: LlmPort impl (D-30)
    ├── assertion.rs                # assertion library (D-29)
    └── runner.rs                    # libtest-mimic eval_scenarios! macro (D-32)

evals/                                # repo-root scenario files (D-34)
├── e2e-1-crash-resume.eval.yaml
├── e2e-2-approval-gate.eval.yaml
└── e2e-3-map-reduce-fault-tolerance.eval.yaml

tests/helpers/e2e_fixtures.rs          # shared graph builders (D-34, refactored out of tests/integration/e2e_*_test.rs)
tests/evals.rs                          # [[test]] name = "evals" harness = false target
```

### Pattern 1: `TraceEmitter` as the sync, never-awaited handoff below the engine

**What:** A second, thinner trait (`TraceEmitter: Send + Sync { fn emit(&self, event: TraceEvent); }`) sits beside `TraceSink`, implemented by `TraceDispatcher` behind a cheap `Arc` clone, and is what every producer *below* the engine (fallback adapter, middleware, execution service) takes instead of `Arc<dyn TraceSink>`.

**When to use:** Any producer that needs to emit trace events but must never construct or own its own `seq` counter — the engine is the sole `seq` authority (D-03).

**Example (the shape, from `fallback.rs`'s existing `with_trace_sink`, to be renamed `with_trace_emitter`):**
```rust
// crates/paladin-llm/src/fallback.rs (existing pattern, sink -> emitter rename)
pub fn with_trace_emitter(mut self, emitter: Arc<dyn TraceEmitter>) -> Self {
    self.trace_emitter = Some(emitter);
    self
}
```

### Pattern 2: Manual `PartialEq`/`Debug` for a non-comparable handle field (precedent: `HeartbeatHandle`)

**What:** `paladin_core::platform::container::heartbeat::HeartbeatHandle` wraps `Arc<watch::Sender<u64>>` and supplies a manual `impl PartialEq for HeartbeatHandle` under which **any two handles compare equal** (`heartbeat.rs:106-115`), plus a manual `Debug` printing an opaque placeholder — because the struct that embeds it, `NodeContext`, derives `PartialEq`/`Debug` and a `watch::Sender` is neither.

**When to use:** If Claude's Discretion resolves to add a `trace: Arc<dyn TraceEmitter>` accessor field on `NodeContext` (CONTEXT.md names this as an open discretion item), the exact same problem recurs — `Arc<dyn TraceEmitter>` is not `PartialEq`. Follow `HeartbeatHandle`'s precedent exactly: a thin newtype wrapper with a manual always-equal `PartialEq` and an opaque `Debug`, not a raw `Arc<dyn TraceEmitter>` field directly on `NodeContext`.

### Anti-Patterns to Avoid

- **A raw `Arc<dyn TraceSink>` passed to a producer below the engine:** bypasses the D-03 `seq` authority — a producer with its own counter (or a shared `Arc<AtomicU64>`) breaks OBS-FR-03's strictly-increasing-per-run guarantee. Always route through `TraceEmitter`.
- **Reusing `paladin-storage::waypoint::retention::prune` for `run_traces`:** the free function is typed to `&dyn WaypointPort` and `WaypointError` — it cannot type-check against `RunTracePort`/`RunTraceError`. Write a parallel, small function; share only the *config values*, not the function itself.
- **Adding `#[utoipa::path]` to the `dev-ui` route:** would pull it into `openapi.json`, breaking D-25's explicit "not API surface" design and forcing an unwanted `sdk-clients` regeneration.
- **`opentelemetry-otlp` with default features:** pulls in `reqwest-blocking-client`, not the async client D-12 specifies — always pass `default-features = false` and the explicit feature list.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| OTLP wire encoding/transport | A hand-rolled protobuf-over-HTTP client for the collector protocol | `opentelemetry-otlp`'s `SpanExporter` | The OTLP protobuf schema is versioned and non-trivial; the official crate tracks spec changes |
| Runtime-discovered, glob-driven test cases | A `build.rs` script generating `#[test]` functions from file contents | `libtest-mimic`'s `Trial`-per-case dynamic registration | D-32 already rejected the `proc_macro` and `macro_rules!` alternatives for the same reason: fragility and recompile cost |
| JSON Schema for the eval file format | Hand-written JSON Schema kept in sync manually | `schemars` derive (already pinned at `1.2`, zero new dependency cost) | The workspace already has this exact golden-bless pattern proven in `graph_doc_round_trip.rs` |
| Glob pattern matching for `evals/**/*.eval.yaml` | A hand-rolled path-walker with wildcard matching | `glob` crate | 9.5M/wk downloads, MIT OR Apache-2.0, the de facto standard, zero reason to reinvent |

**Key insight:** every "don't hand-roll" item in this phase already has either an official upstream crate (OTel, `libtest-mimic`, `glob`) or an in-tree precedent to copy byte-for-byte (`UPDATE_WARGRAPH_SCHEMA=1`'s bless idiom, `sqlx::migrate!`'s migration mechanism, `HeartbeatHandle`'s manual-`PartialEq` pattern). The research risk in this phase is not "does a library exist" — it is "does the existing in-tree pattern generalize cleanly," which is what the sixteen questions above were checking.

## Common Pitfalls

### Pitfall 1: `RunStreamMode` is not yet `#[non_exhaustive]`
**What goes wrong:** D-16 adds a `Replay` variant to an enum that currently has no `#[non_exhaustive]` marker; any external `match` (none exist today, confirmed) would silently be forced to a new match arm, but more importantly the X-10.2 rule is violated if the marker is not added in the same commit.
**Why it happens:** `RunStreamEventKind` and `RunStreamEvent` both got the marker when they were created in Phase 27; `RunStreamMode` was overlooked.
**How to avoid:** Add `#[non_exhaustive]` to `RunStreamMode` in the same commit that adds `Replay`. Record as a deliberate-zero `MIGRATION.md` note (the type is new-in-0.10, so this is not an X-10 register row) but do it anyway per X-10.2's spirit, matching CONTEXT.md's own D-16 text.
**Warning signs:** `cargo semver-checks` would not catch this (adding `#[non_exhaustive]` to a type that has no external consumers yet produces no semver diff) — this is a code-review-time check, not a CI-gate-time check.

### Pitfall 2: `paladin-web` has zero existing cargo features
**What goes wrong:** A planner assuming `dev-ui` is "just another feature flag to add to the existing list" will look for a `[features]` section in `crates/paladin-web/Cargo.toml` and not find one.
**Why it happens:** Every `paladin-web` capability has compiled unconditionally since the crate's creation; feature-gating anything in it is new.
**How to avoid:** Add the `[features]` section from scratch with exactly `dev-ui = []` (or with whatever internal deps it needs), then wire the umbrella `dev-ui = ["paladin-web/dev-ui"]` forwarding at the root. Add the two-line feature-flags matrix entries.

### Pitfall 3: `WaypointRetentionService`/`prune` cannot be generically extended to a second port
**What goes wrong:** A plan that says "extend `WaypointRetentionService::prune()` to also prune `run_traces`" without noticing the free function's hard `&dyn WaypointPort` typing will either try (and fail) to make `prune` generic over a port trait, or accidentally couple `RunTracePort` to `WaypointPort`.
**Why it happens:** The two ports share no supertrait; only the *retention policy values* (age/count bounds) are meant to be shared, not the storage abstraction.
**How to avoid:** Add a second optional field + builder method to `WaypointRetentionService`, and a small sibling `prune`-equivalent function for `RunTracePort` in `paladin-storage`, both driven by the same `WaypointRetentionConfig` values. See Q14 above for the exact shape.
**Warning signs:** A compile error trying to make `prune`'s `port: &dyn WaypointPort` parameter accept a `&dyn RunTracePort` is the first symptom; don't reach for a shared trait object as the fix.

### Pitfall 4: `NodeContext`'s derived `PartialEq`/`Debug` will not compile with a raw `Arc<dyn TraceEmitter>` field
**What goes wrong:** If Claude's Discretion adds a trace accessor directly to `NodeContext` as `pub trace: Arc<dyn TraceEmitter>`, the struct's existing `#[derive(Debug, Clone, PartialEq)]` fails to compile — trait objects are not `PartialEq`.
**Why it happens:** `NodeContext` (`crates/paladin-battalion/src/engine/node.rs:52`) already derives all three; `HeartbeatHandle` already solved exactly this problem for its own `watch::Sender` field.
**How to avoid:** Wrap in a newtype with manual always-equal `PartialEq` and opaque `Debug`, following `HeartbeatHandle`'s pattern verbatim (`heartbeat.rs:106-122`).

### Pitfall 5: Wrong ecosystem for `gsd-tools query package-legitimacy check`
**What goes wrong:** Running the legitimacy gate with `--ecosystem npm` against Rust crate names (`opentelemetry_sdk`, `opentelemetry-otlp`, `libtest-mimic`) returns false `SLOP`/`does-not-exist` verdicts, because it queries the npm registry for crates.io package names.
**Why it happens:** The seam's ecosystem flag is not auto-detected from the target language.
**How to avoid:** Always pass `--ecosystem crates` for this Rust workspace. Verified in this research session — see the Package Legitimacy Audit table above for the correct, re-run result.

## Code Examples

### `sqlx::migrate!` mechanism (verbatim pattern to follow for `run_traces`)
```rust
// Source: crates/paladin-storage/src/waypoint/sqlite.rs:95 (existing pattern)
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

// on adapter construction:
MIGRATOR.run(&pool).await?;
```

### Golden-schema bless idiom (to mirror for the eval-scenario schema, D-28)
```rust
// Source: crates/paladin-battalion/tests/graph_doc_round_trip.rs:104-122 (existing pattern)
fn schema_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/schemas/wargraph-doc.schema.json"))
}
// ...
if std::env::var_os("UPDATE_WARGRAPH_SCHEMA").is_some() {
    std::fs::write(&path, generated_schema).expect("write baseline");
    return;
}
assert_eq!(generated, checked_in_baseline, "regenerate with UPDATE_WARGRAPH_SCHEMA=1 ...");
```

### `HeartbeatHandle`'s manual-`PartialEq` pattern (to mirror for any `Arc<dyn TraceEmitter>`-carrying newtype)
```rust
// Source: crates/paladin-core/src/platform/container/heartbeat.rs:106-122
// Any two handles compare equal (D-18): a handle is never part of identity.
impl PartialEq for HeartbeatHandle {
    fn eq(&self, _other: &Self) -> bool { true }
}
impl Eq for HeartbeatHandle {}
impl fmt::Debug for HeartbeatHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HeartbeatHandle(..)")
    }
}
```

### OTLP HTTP exporter with a no-redirect custom reqwest client (new pattern this phase introduces)
```rust
// Pattern derived from opentelemetry-otlp 0.32.0 docs.rs (with_http_client / HttpClient trait)
let no_redirect_client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .expect("reqwest client");

let exporter = opentelemetry_otlp::SpanExporter::builder()
    .with_http()
    .with_endpoint(&config.endpoint)
    .with_http_client(no_redirect_client)
    .build()?;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| `TraceEvent` per-variant `thread_id` fields (Phase 22 shape) | `TraceRecord` envelope carries `thread_id`/`seq`/`at` once, `#[serde(flatten)]`s the tagged `TraceEvent` | This phase (D-02) | One flat JSON object per line, matching OBS-FR-04 exactly |
| SSE bus's worker-side direct `parley`/`done`/`error` publishes (Phase 27 stopgap) | `map_trace_event` becomes total over all seven wire names once `TraceEvent` carries `ParleyRaised`/`RunFinished.status` | This phase (D-14) | "One implementation, two consumers" (OBS-FR-06) finally holds by construction, not by two producers |
| `opentelemetry-otlp`'s pre-0.30 API (feature name churn across the 0.2x→0.3x line) | `0.32.0` stable feature names (`http-proto`, `reqwest-client`, `reqwest-blocking-client`) | Confirmed current as of 2026-09-08 via `cargo info` | Feature names in CONTEXT.md's own text match the currently-published crate exactly; no drift found |

**Deprecated/outdated:** none found specific to this phase's dependency set — all five new crates are on their current major/minor line with no announced deprecation.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | The exact `MiddlewareEvent` emission site is one of `chain.rs`/`resilience.rs`/`guardrail.rs` (file-split left to Claude's Discretion per CONTEXT.md) | Q12 | Low — CONTEXT.md already scopes this as discretion, not a locked decision; wrong initial guess just means moving one `emit` call during implementation |
| A2 | `mermaid.run({ querySelector: '.mermaid' })` is the correct imperative API for post-load rendering (not directly confirmed against Mermaid 11's own docs page, only WebSearch summary) | Q11 | Low — D-26 defers live SSE attach, and `startOnLoad: true` alone (directly confirmed) is sufficient for the static-embed design this phase actually ships |
| A3 | `paladin-eval` should sit immediately after `paladin-storage` in `publish-crates.sh`'s dependency-order array | Q16 | Low — a dev-dependency edge to `paladin-ai` has no registry-resolution ordering requirement the way a runtime dependency does; any position after its own runtime deps (storage, llm, battalion, ports, core) is technically valid |

**If this table is empty:** N/A — three low-risk items above, none blocking, none touching a locked CONTEXT.md decision.

## Open Questions

None outstanding from CONTEXT.md's explicit research list — all 16 are answered above. One residual item worth flagging for the planner, not a research gap:

1. **Where exactly does `EngineRegistries::default()` get named/registered for the `graph_doc` eval-scenario target (D-31)?**
   - What we know: `EngineRegistries` (`crates/paladin-battalion/src/engine/registries.rs:33-58`) is a plain `#[derive(Default)]`-able bundle of four registries (`edge_evaluators`, `retry_predicates`, `error_handlers`, `output_schemas`), already the exact shape `WarGraphDoc::compile(&EngineRegistries)` consumes.
   - What's unclear: whether the facade's own `ScenarioRunner::register_registries("default", || EngineRegistries::default())` call site (per D-31) lives in `tests/evals.rs`, in a new `src/application/cli/commands/eval.rs` helper, or in a shared `tests/helpers/` module alongside the E2E fixture builders D-34 already relocates there.
   - Recommendation: co-locate it with `tests/helpers/e2e_fixtures.rs` (D-34's own relocation target) since both the `cargo test --test evals` harness and the E2E dogfood scenarios need the same registry population — one registration call, reused by both consumers, avoiding drift between what the integration tests register and what the eval harness registers.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| `cargo`/network access to crates.io | Verifying new dependency versions | ✓ | — | — |
| Docker | Postgres `run_traces` contract suite (Tier 2) | ✗ (confirmed absent in this devcontainer, consistent with 23-CONTEXT/24-CONTEXT's prior findings) | — | CI's `postgres-integration` job only; never claim this tier passed locally (same house rule every prior phase in this milestone has followed) |
| Redis, MinIO | Workspace-wide `coverage` CI job (unrelated to this phase's own new code, but gates the 82% floor measurement) | ✗ locally, ✓ in CI | — | `make services-up` locally, or read the figure from CI as this milestone has done throughout |

**Missing dependencies with no fallback:** none — every missing local dependency above has an established CI-only fallback path already used by six prior phases in this milestone.

**Missing dependencies with fallback:** Docker (Postgres contract suite → CI `postgres-integration` job); Redis/MinIO (coverage measurement → CI `coverage` job or `make services-up`).

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | `cargo test` (built-in) for everything except the new `evals` target, which uses `libtest-mimic 0.8.2` as a `harness = false` custom runner with the same CLI surface |
| Config file | `Cargo.toml` (workspace + per-crate `[[test]]`/`[[bench]]` stanzas); no separate test-framework config file exists or is needed |
| Quick run command | `cargo test -p paladin-battalion --lib engine::hooks::tests` (trace dispatcher unit tests); `cargo test --test evals <filter>` (one eval scenario/case) |
| Full suite command | `cargo test --workspace --features integration-tests,llm-all -- --test-threads=1` (mirrors `scripts/coverage.sh`'s own invocation, minus the `--lcov` flag) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| OBS-01 | `TraceEvent`/`TraceRecord` serde round-trip, `seq` gapless-or-counted-drops | unit | `cargo test -p paladin-core --lib platform::container::trace` | ❌ Wave 1 |
| OBS-01 | Panic isolation: `PanickingTraceSink` sibling to existing three test doubles | unit + property | `cargo test -p paladin-battalion --lib engine::hooks::tests` | ❌ Wave 1 (extends existing `engine/test_support.rs:645-760`) |
| OBS-01 | `CompositeSink` fan-out, one-panicking-child-doesn't-starve-siblings | unit | `cargo test -p paladin-ports --lib output::trace_sink_port` | ❌ Wave 1 |
| OBS-01 | X-05 multi-thread stress: 16 concurrent runs, exact per-run counts, gapless per-run `seq` | integration (stress) | `cargo test -p paladin-battalion --lib engine::hooks::tests -- --test-threads=1` (own `#[tokio::test(flavor = "multi_thread")]`) | ❌ Wave 1 |
| OBS-02 | Log sink: one JSON line per event, target `paladin::trace` | unit | `cargo test -p paladin-ai --lib infrastructure::telemetry::log_sink` | ❌ Wave 2 |
| OBS-02 | OTel span-tree shape (branch + retry + muster fixture) | unit (in-memory) | `cargo test -p paladin-ai --lib --features otel infrastructure::telemetry::otel_sink` | ❌ Wave 3 |
| OBS-02 | OTel transport against axum stub | integration | `cargo test -p paladin-ai --features otel --test otel_transport` (existing `FixtureServer` pattern, Phase 3 precedent) | ❌ Wave 3 |
| OBS-02 | SSE collapse: seven-of-twelve `map_trace_event` total match | unit | `cargo test -p paladin-ai --lib application::services::run::events` (existing "exactly four of eight" test becomes "exactly seven of twelve") | ✅ extends existing (Wave 4) |
| OBS-02 | `run_traces` persistence + replay upgrade | integration + Tier 2 (postgres) | `cargo test -p paladin-storage --lib run_trace::contract_tests` (sqlite local); postgres via CI `postgres-integration` only | ❌ Wave 4 |
| OBS-03 | Golden Mermaid/DOT exports (linear, branch+join, loop, muster, subgraph) | golden | `UPDATE_GOLDEN=1 cargo test -p paladin-battalion --test export_golden` then re-run without the env var | ❌ Wave 5 |
| OBS-03 | Execution overlay golden (branching fixture + scripted run) | golden | same target as above | ❌ Wave 5 |
| OBS-03 | CLI `graph export`/`run export` snapshot tests | snapshot (`insta`) | `cargo test -p paladin-ai --features cli --test cli_graph_export` | ❌ Wave 5 |
| OBS-03 | Inspector page smoke test (DOM-level, embedded-JSON assertion) | integration (`oneshot`) | `cargo test -p paladin-web --features dev-ui --lib dev_ui_controller` | ❌ Wave 6 |
| OBS-04 | Assertion library unit tests + failure-rendering snapshots | unit + snapshot | `cargo test -p paladin-eval --lib assertion` | ❌ Wave 7 |
| OBS-04 | Runner: `--repeat 20` on a scripted scenario is 20/20 | integration | `cargo run --features cli -p paladin-ai -- eval run evals/**.eval.yaml --repeat 20` (manual/CI, not `cargo test`) | ❌ Wave 7 |
| OBS-04 | E2E-1/2/3 dogfooded as eval scenarios, green | integration | `cargo test --test evals` (the new `harness = false` target) | ❌ Wave 8 |
| OBS-04 | E2E-1/2/3 integration tests stay green independently (SHIP-03) | integration | `cargo test --test e2e_crash_resume_test --test e2e_approval_gate_test --test e2e_compensation_chain_test` | ✅ pre-existing, must stay green after the `tests/helpers/e2e_fixtures.rs` refactor |
| Bench acceptance criterion 6 | ≤3% superstep overhead with default sinks vs. sink-disabled | bench (recorded, not CI-gated) | `cargo bench -p paladin-ai --bench engine_benchmarks -- bench_superstep_cost` | ❌ Wave 2 (extends existing `benches/engine_benchmarks.rs`) |

### Sampling Rate
- **Per task commit:** the narrowest `cargo test -p <crate> --lib <module>` command from the table above for the module just touched.
- **Per wave merge:** `cargo test --workspace --features integration-tests,llm-all -- --test-threads=1` (mirrors CI's `scripts/coverage.sh` invocation minus `--lcov`).
- **Phase gate:** full `scripts/coverage.sh` (or CI's `coverage` job) green at ≥82%, plus the D-40 close-out gate list in full (fmt, clippy `-D warnings` with and without `otel`/`dev-ui`, semver-checks against the 11 pre-existing crates, MSRV 1.88 job, `make security`, `check-api-surface.sh` regenerated, `UPDATE_OPENAPI=1` re-bless + `sdk-clients`, `cargo tree -e features` showing no `opentelemetry*`/`libtest-mimic` on a default build, feature-flags job covering `otel`/`dev-ui` individually).

### Wave 0 Gaps
- [ ] `crates/paladin-core/src/platform/container/trace.rs` — the new module itself doesn't exist yet; every OBS-01 test above depends on it.
- [ ] `crates/paladin-battalion/tests/export_golden.rs` (or similarly named) — no export-golden test target exists yet; needs `tests/fixtures/graph_docs/` extended with the branch+join/loop/subgraph fixtures D-20 names (linear may already exist from Phase 27's own fixture corpus — verify at Wave 5 start).
- [ ] `crates/paladin-eval/` — the crate itself does not exist; `Cargo.toml`, `src/lib.rs` and the whole crate skeleton are Wave 7's first task.
- [ ] `tests/evals.rs` — the `[[test]] name = "evals" harness = false` target; needs a `[[test]]` stanza added to root `Cargo.toml` (no existing precedent for a `harness = false` `[[test]]` — see Q4/Pitfall list — so this is genuinely first-of-its-kind work, budget accordingly).
- [ ] `tests/helpers/e2e_fixtures.rs` — does not exist yet; the D-34 refactor extracting graph builders out of `tests/integration/e2e_*_test.rs` is a Wave 8 prerequisite for both the integration tests staying green and the eval harness dogfood scenarios working.

*(Framework install: none needed — `cargo test`/`cargo bench`/`insta` are already in place; only `libtest-mimic`/`glob` need adding to `crates/paladin-eval/Cargo.toml`.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | yes | `dev-ui` route reuses the existing `require_auth` layer (`app.rs`'s established `from_fn_with_state(auth_port, require_auth)` pattern) — no new auth mechanism |
| V3 Session Management | no | No new session concept introduced; opaque-token auth is unchanged from prior phases |
| V4 Access Control | yes | `dev-ui` route additionally reuses `require_admin` (the existing admin-route layering pattern confirmed at `app.rs:51-57`) — it exposes state field names and run structure, operator-only information per D-25 |
| V5 Input Validation | yes | `.eval.yaml` scenario files are untrusted input to a test harness — `serde_yaml`/`schemars`-validated deserialization, never `eval`'d or shelled out; `OtelConfig.endpoint` validated as `http(s)`-only in `TraceConfig::validate()` (D-36) |
| V6 Cryptography | no | No new cryptographic primitive introduced this phase |
| V7 Error Handling & Logging | yes | `FieldChange.value` (when `trace.state_values = true`) must pass through the existing `crates/paladin-llm/src/redaction.rs` helper **before** truncation (D-05) — the security-instructions ordering rule, already a documented pitfall class in this codebase (redact-before-truncate is called out by name in `.github/instructions/security.instructions.md`) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Secret-bearing `OtelConfig.headers` leaking via `Debug`/logs | Information Disclosure | D-36: manual `Debug` impl that redacts header values, no log line interpolates them — mirrors the existing `security.instructions.md` credential-handling rule applied to every other adapter with a header-carrying config |
| OTLP exporter following a redirect to an attacker-controlled host, carrying the configured `Authorization`/other headers | Information Disclosure / SSRF-adjacent | `reqwest::Client::builder().redirect(Policy::none())` passed via `with_http_client()` (confirmed available in Q1 above) — the same house rule the webhook client (`.github/instructions/security.instructions.md`'s named Phase 27 example) and every LLM adapter already follow |
| Truncating a redacted value incorrectly (slicing a secret across the truncation boundary) | Information Disclosure | D-05: redact via `redaction.rs` **before** truncating to `value_cap_bytes` — explicitly the ordering rule `security.instructions.md` calls out by name for a different subsystem (LLM error bodies), now applying identically to trace `FieldChange.value` |
| Untrusted `.eval.yaml` scenario content used as a code-execution vector | Elevation of Privilege | `custom(fn)` assertions are Rust-API-only (never deserialized from YAML/JSON) per OBS-FR-12; the file format itself carries no executable content, only structured assertion parameters |
| Dev-UI route information disclosure (state field names, run structure) to a non-admin | Information Disclosure | `require_admin` + `require_auth` double layer (D-25), route excluded from default build entirely (`dev-ui` feature, default off) |

## Project Constraints (from CLAUDE.md)

- **Hexagonal dependency rule** — `paladin-core` depends on nothing internal; `paladin-ports` depends only on `paladin-core`; infrastructure adapters depend on core + ports. No new port trait may import an SDK/DB driver/HTTP client. This phase's `TraceEmitter`/`CompositeSink`/`RunTracePort`/`RunInspectorPort` all satisfy this by construction per the Architectural Responsibility Map above.
- **TDD, Red-Green-Refactor** — every FR in this phase is written to be testable per the Validation Architecture table; write the failing test first, per D-41's TDD-ordered wave plan (PRD 07 §4).
- **Coverage floor: 82% workspace line coverage** (single number, ADR-0006, superseded 2026-08-13) — gated by `cargo llvm-cov --fail-under-lines` in CI's `coverage` job. The eval crate and its scenarios count toward this figure (D-40).
- **Avoid `unwrap()`/`expect()`/`panic!` in library code** — return `Result`. Every new error type in this phase (`RunTraceError`, `InspectorError`, `CompileError` extensions, eval-assertion failures) must be a `thiserror` enum with structured fields (X-06), never a stringly-typed variant.
- **Ubiquitous language** — no new Medieval Military term is introduced by this phase ("trace", "sink", "eval" are plain terms, per CONTEXT.md's own note); continue using Battlefield/Superstep/Waypoint/Thread/Directive/Muster/Parley/Vanguard/Chronicle/Aegis/Vault consistently in any new code/docs that touches those concepts.
- **Before committing a parent task:** `cargo test` → `cargo fmt --check` → `cargo clippy -- -D warnings`, then a conventional-commit message; stop after each major task and wait for go-ahead (per this repo's stated working agreement).
- **Security:** `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings` on new/modified code, plus the manual credential-handling review for the OTel exporter's header-carrying HTTP client (redirects disabled, no header value logged) — CodeQL remains advisory-only per the documented, version-scoped disqualification; do not reintroduce Snyk.
- **Prefer borrowing over cloning; keep iterators lazy** — applies to the `GraphShape`/`ExecutionOverlay` construction code in particular, which walks potentially large graphs/trace records.

## Sources

### Primary (HIGH confidence — direct repo `git`/`grep`/`cargo info` verification)
- `crates/paladin-ports/src/output/trace_sink_port.rs` — current `TraceEvent`/`TraceSink` shape (Phase 22 baseline this phase extends)
- `crates/paladin-battalion/src/engine/{hooks.rs,mod.rs,superstep.rs,graph.rs,graph_doc.rs,registries.rs}` — `TraceDispatcher`, `RunOutcome`, edge-evaluation/parley/heartbeat sites, `NodeSpec`/`WarGraphDoc` shapes
- `crates/paladin-core/src/platform/container/{run.rs,heartbeat.rs}` — `RunStreamMode`/`RunStreamEventKind` attributes, `HeartbeatHandle`'s manual-`PartialEq` precedent
- `crates/paladin-llm/src/{mock.rs,fallback.rs}` — `MockLlmAdapter`/`MockScriptEntry` v0.9.0 provenance, `FallbackLlmAdapter::with_trace_sink` callers
- `crates/paladin-storage/src/waypoint/{mod.rs,retention.rs,sqlite.rs,postgres.rs}` — migration mechanism, retention-service signature
- `crates/paladin-web/{Cargo.toml,src/openapi.rs,src/app.rs}` — feature-less Cargo.toml, drift-guard assembly function, admin-route layering pattern
- `src/application/services/run/{worker.rs,events.rs}` — direct publish sites, per-run engine composition point, `map_trace_event`
- Root `Cargo.toml`, `deny.toml`, `.github/workflows/{ci.yml,feature-flags.yml,release.yml}`, `Makefile`, `scripts/{publish-crates.sh,extract-public-api.sh,check-api-surface.sh,coverage.sh}` — dependency versions/features, feature-forwarding pattern, CI job composition, crate-list enumeration points
- `.planning/decisions/PROMOTION.md` — next free ADR number (0048, correcting CONTEXT.md's own 0047 example)
- `cargo info opentelemetry@0.32.0 / opentelemetry_sdk@0.32.1 / opentelemetry-otlp@0.32.0 / libtest-mimic@0.8.2 / glob@0.3.4` — live crates.io version/license/MSRV lookups, 2026-09-08
- `gsd-tools query package-legitimacy check --ecosystem crates ...` — clean `OK` verdicts on all five new dependencies

### Secondary (MEDIUM confidence — WebFetch against docs.rs, official documentation)
- `docs.rs/opentelemetry-otlp/0.32.0` — feature list, `with_http_client`/`HttpClient` trait for custom (no-redirect) reqwest client
- `docs.rs/opentelemetry_sdk/0.32.1` — `testing` module/`testing` feature, `InMemorySpanExporter` location
- `docs.rs/opentelemetry/0.32.0` — `SpanBuilder::with_start_time`, `Span::end_with_timestamp` signatures

### Tertiary (LOW confidence — WebSearch only, cross-checked against jsdelivr.com's own package page)
- Mermaid 11 jsDelivr ESM CDN URL and minimal `<script type="module">` init pattern

## Metadata

**Confidence breakdown:**
- Standard stack (OTel/libtest-mimic/glob versions, licenses, MSRV): HIGH — confirmed via live `cargo info` against crates.io, not training-data recall
- The 16 open questions: HIGH — every one resolved against direct `git`/`grep` output or docs.rs citations, none left as inference
- Architecture patterns (`TraceEmitter` split, `HeartbeatHandle` precedent, `WaypointRetentionService` pitfall): HIGH — derived directly from reading the actual current source, not assumed from CONTEXT.md's prose alone
- Mermaid CDN details: MEDIUM — WebSearch-sourced, not fetched directly from mermaid.js.org's own docs page (a `mermaid.run()` follow-up fetch would raise this to HIGH if the planner wants it, but D-26's static-embed design only needs the already-confirmed `startOnLoad: true` path)

**Two corrections to CONTEXT.md's own text, both load-bearing for the plan:**
1. Next free ADR number is **0048**, not 0047 (`.planning/decisions/PROMOTION.md:69`).
2. `crates/doc-examples` is precedent for a composition crate's *dependency shape* only, not for D-27's *publishing* decision — its own package (`paladin-doc-examples`) is `publish = false`.

**Research date:** 2026-09-08
**Valid until:** 30 days for the in-repo findings (stable, code-derived); 7 days for the crates.io version pins specifically (OTel in particular ships frequent point releases — re-run `cargo info` immediately before the OTel wave (Wave 3) starts if more than a few days have elapsed since this research).
