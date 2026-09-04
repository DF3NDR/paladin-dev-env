# PRD 07 — Observability: Trace Stream, OpenTelemetry Export, Visualization Export, Eval Harness (Epic `OBS`)

**Depends on:** PRD 01 (TraceSink seam ENG-FR-21, WarGraphDoc from PLAT-FR-12 for visualization — coordinate if Doc 06 is scheduled later: WarGraphDoc may land here first). Eval harness (§2.4) is standalone.
**Primary crates:** `paladin-ports` (TraceSink types), `paladin-core` (event types), facade `infrastructure/telemetry`, new dev-tool surface in `paladin-web`, new `paladin-eval` crate (dev-dependency-oriented).

---

## 1. Problem Statement

Paladin emits structured logs and has an internal event system, but there is no standardized, machine-consumable account of *what a workflow run did*: which nodes ran in which supersteps, what each attempt cost, which edges fired and why, how state changed. Without it: production debugging is log archaeology; no external tracing tool can render a run; no visualization of a graph or an execution is possible; and there is no harness to regression-test agent *behavior* (as opposed to code), which an LLM-dependent system needs as much as unit tests.

## 2. Functional Requirements

### 2.1 Trace event model (the contract everything else consumes)

- **OBS-FR-01 (TraceEvent).** In `paladin-core` (types only), a serde-serializable tagged enum — the authoritative list (extend, never repurpose):

  ```text
  RunStarted   { thread_id, run_id?, graph_fingerprint, at }
  SuperstepStarted { superstep, vanguard: Vec<NodeId>, at }
  NodeStarted  { superstep, node_id, attempt, muster_task_key?, at }
  NodeProgress { node_id, kind: Heartbeat|StreamChunk{bytes}|ToolCall{tool}, at }
  NodeFinished { superstep, node_id, attempt, outcome, duration_ms, token_count, cache_hit, at }
  EdgeEvaluated{ from, to, condition_kind, fired: bool, at }
  DeltaMerged  { superstep, field_changes: Vec<FieldChange{field, dispatch, writers}>, at }
  WaypointSaved{ waypoint_id, superstep, status, at }
  ParleyRaised { parley_id, node_id, kind, at }
  RunFinished  { status, total_supersteps, total_tokens, duration_ms, at }
  FallbackHop  { node_id?, from_provider, to_provider, at }          // from FT-FR-17
  MiddlewareEvent { name, action, at }                                // Finish/Deny/Redact etc. (RT)
  ```

  Every event carries `thread_id` + monotonic `seq: u64` (per-run). Payload sizes bounded: `FieldChange` carries field names and value-size bytes, NEVER full state values (privacy + volume); a `trace_state_values: bool` debug config (default false) may include values truncated to a configured byte cap.
- **OBS-FR-02 (TraceSink port; non-interference).** `TraceSinkPort { fn emit(&self, ev: TraceEvent) }` — synchronous handoff into a bounded channel (capacity configurable, default 1024); on overflow drop-oldest and increment a `trace_dropped_total` counter surfaced in `RunFinished` metadata and logs. A panicking/blocking sink implementation MUST NOT affect the run (engine wraps emission in catch_unwind + never awaits the sink). Multi-sink fan-out via a `CompositeSink`.
- **OBS-FR-03 (Ordering guarantee).** Events for one run are emitted in causal order (seq strictly increasing); consumers may rely on it. Cross-run interleaving is unordered. Test: 20-superstep run, assert seq gapless per emitted set (gaps only from documented drops).

### 2.2 Sinks

- **OBS-FR-04 (Log sink).** Default-on structured-log sink (existing `log`/telemetry stack), one line per event, JSON payload.
- **OBS-FR-05 (OpenTelemetry sink).** Feature `otel`: adapter mapping the event stream to OTel spans — `run` root span; child span per node ATTEMPT (span-per-attempt, retries visible as siblings) with attributes `{node_id, superstep, attempt, outcome, tokens, cache_hit, muster_task_key}`; span events for `EdgeEvaluated`/`DeltaMerged`/`ParleyRaised`; OTLP exporter config (endpoint, headers, service.name) per X-09. LLM calls inside a node MAY be child spans of the attempt span where the adapter has instrumentation, but this is not required in this epic. Verified against an in-test OTLP collector stub asserting span tree shape for a branching + retrying run.
- **OBS-FR-06 (SSE bridge).** The live sink backing `GET /runs/{id}/stream` (PLAT-FR-07) is a TraceSink adapter — one implementation, two consumers; no second event pathway may be introduced.
- **OBS-FR-07 (Waypoint trace persistence, opt-in).** Config `trace_persist: bool` (default false): batch events per superstep into an append-only `run_traces` table (same backend family as Waypoints) enabling post-hoc replay of the SSE stream for finished runs (the PLAT-FR-07 degraded mode upgrades to full fidelity when enabled). Retention shares ENG-FR-18 policy.

### 2.3 Visualization export

- **OBS-FR-08 (Static graph export).** `WarGraphDoc → Mermaid flowchart` and `→ Graphviz DOT` exporters (pure functions, golden-file tested): nodes labeled with id + kind badge (paladin/function/gate/subgraph/worker-template), conditional edges labeled with condition kind, subgraphs rendered as clusters, worker templates visually distinct (dashed). CLI: `paladin-cli graph export --format mermaid|dot <assistant|file>`.
- **OBS-FR-09 (Execution overlay export).** Given a thread's persisted trace (OBS-FR-07) or Waypoint history, export an annotated Mermaid diagram: executed nodes colored by outcome, visit counts on loop nodes, fired edges bold, per-node duration+tokens in labels. CLI: `paladin-cli run export --thread <id> [--waypoint <id>]`. Golden tests on a fixture run.
- **OBS-FR-10 (Minimal run inspector page).** `paladin-web` (feature `dev-ui`, default off, auth-gated): a single served HTML page per thread rendering (client-side, no build pipeline — inline JS + the Mermaid library vendored or CDN-configurable) the OBS-FR-09 diagram plus a superstep-by-superstep table (from the history endpoint) with per-superstep field-change lists. Explicitly minimal: no editing, no live mode required (nice-to-have if SSE trivially attaches). Acceptance = a human can answer "which branch fired and why did node X run 3 times" from this page alone on the fixture run.

### 2.4 Eval harness (standalone; new crate `paladin-eval`)

- **OBS-FR-11 (Scenario definition).** A test-oriented API + serde file format (`.eval.yaml`/`.json`) defining: target (assistant doc / graph constructor), inputs (one or many cases), scripted LLM behavior (per-node or global `MockLlmAdapter` response scripts, supporting sequence-per-call and match-on-prompt-substring), and assertions.
- **OBS-FR-12 (Assertion library).** Built-in assertions, each with a clear failure rendering: `final_state_field_equals`, `final_state_field_matches(regex)`, `field_json_path_equals`, `node_executed(times: exact|min|max)`, `node_not_executed`, `edge_fired(from,to)`, `route_taken([nodes...] as subsequence)`, `run_status`, `total_tokens_max`, `supersteps_max`, `parley_raised(kind, node)`, plus `custom(fn)` in the Rust API. Assertions evaluate against the TraceEvent record + final Battlefield — the harness runs with a capturing sink; no reaching into engine internals.
- **OBS-FR-13 (Runner & CI shape).** `cargo test`-integrable runner macro (`eval_scenarios!("evals/**.eval.yaml")`) generating one test per case, and a CLI `paladin-cli eval run <glob> [--repeat N]` with `--repeat` for flakiness detection (report per-case pass rate; nondeterminism with scripted mocks = a bug to surface, exit non-zero if any repeat diverges). Baseline snapshots: `--bless` writes expected final-state snapshots for `*_snapshot` assertions.
- **OBS-FR-14 (Live-model mode, gated).** The same scenario files runnable against real providers behind `--live` + env-var keys, with assertions restricted to structural kinds (route/status/limits) unless `allow_content_assertions: true`. Never run in default CI. Documented as the promotion path: script-mocked in CI, live smoke pre-release.
- **OBS-FR-15 (Dogfood).** The program's E2E-1/2/3 fixtures MUST also exist as eval scenarios, proving the harness expresses the program's own acceptance bar.

## 3. Acceptance Criteria

1. Event-order + drop-accounting tests (OBS-FR-03, overflow with a deliberately slow sink → counted drops, run unaffected, timing-asserted non-blocking).
2. OTel span-tree shape test against collector stub (branch + retry + muster fixture).
3. Golden Mermaid/DOT exports for: linear, branch+join, loop, muster, subgraph fixtures; execution overlay golden for the branching fixture.
4. Inspector page smoke test (served, renders fixture data — DOM-level assertion via the existing HTTP test harness on embedded data payload, not a browser test).
5. Eval harness: a failing assertion renders actionable output (snapshot-tested message); `--repeat 20` on a scripted scenario is 20/20; E2E-1/2/3 expressed as scenarios and green.
6. Coverage per X-02; no measurable engine slowdown with default sinks (bench: ≤ 3% superstep overhead vs. sink-disabled, added to `benches/`).
7. **Versioning gate (X-10/X-11):** any pre-existing public type touched by this epic is recorded in `MIGRATION.md` §9.2 with its mitigation; `cargo semver-checks` and the MSRV job pass; new dependencies listed in §9.3; new migrations in §9.4; new config/env in §9.5.

## 4. Test Plan (TDD ordering)

1. TraceEvent serde + seq/order units.
2. Channel/overflow/panic-isolation tests.
3. Log sink, composite sink.
4. OTel mapping units → collector-stub integration.
5. Exporters (golden files) → CLI wiring.
6. Trace persistence + SSE-replay upgrade path.
7. Eval: format parsing → assertion units → runner macro → repeat/bless → dogfood scenarios.
8. Inspector page.

## 5. Out of Scope

Full graphical IDE / live-editing studio; hosted trace product; browser-automation tests for the inspector page; LLM-as-judge eval scoring (assertion `custom` leaves the door open; no built-in judge ships).
