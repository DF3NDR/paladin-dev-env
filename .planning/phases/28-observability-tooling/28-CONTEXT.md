# Phase 28: Observability & Tooling - Context

**Gathered:** 2026-09-08
**Status:** Ready for planning
**Mode:** `--auto` — every decision below is the recommended default, selected without a human
in the loop and logged in `28-DISCUSSION-LOG.md`. A human reviewing before planning should look
first at D-03 (seq authority), D-18 (`GraphShape`), D-27 (`paladin-eval` publishing posture) and
D-31/D-34 (registered-constructor scenario targets) — the four choices with the widest blast radius.

<domain>
## Phase Boundary

Phase 28 delivers **OBS-01 … OBS-04** (`.planning/REQUIREMENTS.md` lines 259-282; PRD 07
OBS-FR-01…15) — the observability and tooling epic of the v0.10.0 "Durable Agent Execution
Runtime" milestone, and the last feature phase before the Phase 29 SHIP gates:

1. **The authoritative trace stream (OBS-01).** The `TraceEvent` enum Phase 22 seeded (eight
   variants, `#[non_exhaustive]`, `crates/paladin-ports/src/output/trace_sink_port.rs:67`) becomes
   PRD 07 §2.1's twelve-variant authoritative list, every event carrying `thread_id` + a per-run
   monotonic `seq` + `at`, payloads bounded (field names and byte sizes, never values by default),
   with a gapless-or-counted-drops guarantee, `catch_unwind` panic isolation of sinks, and a
   `CompositeSink` fan-out. The Phase 22 fire-and-forget `TraceDispatcher`
   (`crates/paladin-battalion/src/engine/hooks.rs`, bounded 1024, drop-oldest, counted) is the
   base this phase extends, not replaces.
2. **Real consumers (OBS-02).** A default-on structured-log sink; an `otel`-feature-gated
   OpenTelemetry exporter with span-per-attempt trees verified against a collector stub; the
   Phase 27 SSE bridge (`src/application/services/run/events.rs`) collapsed onto the single
   `TraceSink` pathway (27-CONTEXT D-25's "single edit point"); and opt-in `run_traces`
   persistence that upgrades post-hoc stream replay to full fidelity, pruned under the ENG-FR-18
   Waypoint retention policy.
3. **Visualization (OBS-03).** Golden-tested Mermaid and DOT exporters, an execution-overlay
   export (outcomes, visit counts, fired edges, durations/tokens), `paladin-cli graph export` and
   `paladin-cli run export`, and a minimal, auth-gated, feature-gated `dev-ui` inspector page in
   `paladin-web` from which a human can answer "which branch fired and why did node X run 3
   times" on the fixture run.
4. **The eval harness (OBS-04).** The new `crates/paladin-eval` crate: a serde scenario file
   format (scripted mock-LLM behavior + assertions), an assertion library evaluated over the
   captured trace record and final Battlefield, a `cargo test`-integrable runner,
   `paladin-cli eval run <glob> [--repeat N] [--bless] [--live]`, and the three program E2E
   fixtures (`tests/integration/e2e_{crash_resume,approval_gate,compensation_chain}_test.rs`)
   dogfooded as eval scenarios.

Plus the program-wide obligations every phase carries: X-01…X-11 (hexagonal rule, TDD + 82%
floor, backward compatibility, `schema_version`, `Send + Sync` + multi-thread stress test,
structured errors, feature flags, docs, config structs, semver register, MSRV 1.88 + dependency
discipline), the `MIGRATION.md` §9 rows this phase owes, and the release gates
(`cargo semver-checks`, `msrv`, `make security`, coverage, `api-surface`, `openapi` golden,
`sdk-clients`).

**Not this phase.** `MIGRATION.md` §9 finalisation, the v0.9-config boot test, the openapi golden
diff and the program acceptance audit (SHIP-01…04, Phase 29); a graphical IDE or live-editing
studio, a hosted trace product, browser-automation tests of the inspector, and LLM-as-judge eval
scoring (PRD 07 §5); LLM-call child spans under attempt spans (PRD 07 OBS-FR-05 "MAY, not
required"); `27-SECURITY.md` (`/gsd-secure-phase 27` is a separate, still-open step — see
STATE.md). Any behavioral change to an existing public surface discovered mid-implementation is
an X-03 stop-and-flag event, not a judgment call.

</domain>

<decisions>
## Implementation Decisions

Decision numbering continues the house style (D-01 …). Each carries the PRD 07 FR it serves.

### Trace event model & the `seq` authority (OBS-01; OBS-FR-01, FR-03)

- **D-01: The authoritative types move to `paladin-core`; `paladin-ports` re-exports them.**
  PRD 07 says "in `paladin-core` (types only)" and ADR-0016 says core owns port value types with
  ports re-exporting. New module `crates/paladin-core/src/platform/container/trace.rs` holds
  `TraceEvent`, `TraceRecord`, `FieldChange`, `NodeProgressKind`, `TraceOutcome`/status enums and
  `TRACE_SCHEMA_VERSION`; `crates/paladin-ports/src/output/trace_sink_port.rs` keeps `TraceSink`,
  `TraceSinkError`, the new `TraceEmitter` (D-03) and `CompositeSink` (D-08), and `pub use`s the
  core types so every existing `use paladin_ports::output::trace_sink_port::TraceEvent` keeps
  compiling. `TraceEvent` is new-in-0.10 (absent at `v0.9.0`), so X-10 does not apply; the move
  is recorded as a deliberate-zero note in `MIGRATION.md` §9.2 (D-39).
  — **Reversibility:** costly — a published type's home crate; moving it back touches every
  `use` in three crates and the facade.
- **D-02: `TraceRecord` is the envelope; `TraceEvent` stays the tagged enum.**
  `TraceRecord { thread_id: ThreadId, run_id: Option<RunId>, seq: u64, at: DateTime<Utc>,
  #[serde(flatten)] event: TraceEvent }` with `#[serde(tag = "kind", rename_all =
  "snake_case")]` on the enum, so one record serializes to a single flat JSON object
  (`{"thread_id":…,"seq":7,"at":…,"kind":"node_started","node_id":…}`) — exactly OBS-FR-04's
  "one line per event". The per-variant `thread_id` fields the Phase 22 enum carries are
  removed (the envelope carries it once); `superstep`, `node_id`, `attempt` stay on the variants
  that have them. The twelve variants and their payloads follow PRD 07 §2.1 verbatim, with these
  concrete resolutions: `RunStarted { run_id?, graph_fingerprint }`; `NodeStarted` gains
  `muster_task_key: Option<String>` (from `NodeContext`'s Muster task key, CF-FR-10);
  `NodeFinished { outcome: NodeOutcomeKind, duration_ms, token_count, cache_hit }` — the
  same `NodeOutcomeKind` the Waypoint's `NodeExecutionRecord` uses, so a trace and a Waypoint
  never disagree on vocabulary; `EdgeEvaluated { from, to, condition_kind, fired }` with
  `condition_kind` the `EdgeCondition` discriminant name (`always`/`contains`/`regex`/`custom`);
  `DeltaMerged { field_changes: Vec<FieldChange { field, dispatch, writers: Vec<NodeId>,
  value_bytes: u64, value: Option<String> }> }` (D-05); `WaypointSaved { waypoint_id, superstep,
  status }`; `ParleyRaised { parley_id, node_id, kind }`; `RunFinished { status, total_supersteps,
  total_tokens, duration_ms, trace_dropped_total }` (D-07); `FallbackHop` unchanged from Phase 25
  D-25; `MiddlewareEvent { name, action }` (D-04); `NodeProgress { node_id, kind: Heartbeat |
  StreamChunk { bytes } | ToolCall { tool } }`. `TraceEvent` stays `#[non_exhaustive]`; every
  `match` in the tree carries a wildcard arm. The Phase 27 D-25 correction ("no parley variant,
  no error variant, `RunFinished` does not distinguish success from failure") is closed by
  `ParleyRaised` and `RunFinished.status`.
  — **Reversibility:** one-way once persisted (D-17) — `run_traces` rows carry this shape under
  `TRACE_SCHEMA_VERSION = "1"`; reshaping later needs a data migration, exactly as 23-CONTEXT D-14
  records for `MusterProgress`.
- **D-03: One per-run `seq` authority — the engine's `TraceDispatcher` — reached by every producer
  through a `TraceEmitter` handle, never a raw sink.** `seq` starts at 1 per run and is stamped,
  together with `at` and `thread_id`/`run_id`, inside `TraceDispatcher::emit` at enqueue time
  (so `seq` order *is* causal order, and `at` is when the event happened, not when a sink saw it).
  A new object-safe `TraceEmitter: Send + Sync { fn emit(&self, event: TraceEvent) }` (sync,
  never awaits) lives beside `TraceSink` in `paladin-ports`; `TraceDispatcher` implements it
  behind a cheap clonable handle (`WarEngine::trace_emitter() -> Arc<dyn TraceEmitter>`). The
  producers that sit *below* the engine — `FallbackLlmAdapter` (Phase 25 D-25 `FallbackHop`),
  the Phase 26 middleware chain (`MiddlewareEvent`) and `PaladinExecutionService`'s stream/tool
  paths (`NodeProgress::StreamChunk`/`ToolCall`) — take an `Arc<dyn TraceEmitter>` instead of an
  `Arc<dyn TraceSink>`: `FallbackLlmAdapter::with_trace_sink` becomes `with_trace_emitter`
  (new-in-0.10, D-39 deliberate-zero; the only caller is the facade). The Phase 27 run worker,
  which already builds one engine per run, wires the same handle into everything it composes for
  that run, so in the served path every producer stamps from the one counter. A producer used
  with no engine at all (a bare `FallbackLlmAdapter` in a unit test) gets a `StandaloneEmitter`
  with its own counter and a caller-supplied `ThreadId` — documented as test-only. Rejected:
  producer-local counters merged downstream (breaks OBS-FR-03's strictly-increasing guarantee);
  a shared `Arc<AtomicU64>` passed around by value (the same thing with a worse API).
  — **Reversibility:** costly — the emitter/sink split is the shape every sink and producer is
  written against.
- **D-04: Producers of the four new variants, fixed here so the researcher does not re-open it.**
  `EdgeEvaluated` — the engine's edge-resolution step (`crates/paladin-battalion/src/engine/
  superstep.rs` / the `EdgeConditionEvaluator` call site), one event per evaluated edge whether
  or not it fired; `ParleyRaised` — the engine at the moment it builds the `AwaitingInput`
  outcome; `NodeProgress::Heartbeat` — the engine's `HeartbeatHandle` receiver (Phase 25 D-18),
  rate-limited to at most one heartbeat event per `trace.heartbeat_interval` (default 5 s) per
  node so a chatty node cannot flood the queue; `NodeProgress::StreamChunk`/`ToolCall` and
  `MiddlewareEvent` — the facade's `PaladinExecutionService` and middleware chain (26-CONTEXT
  D-01 puts them there) through the D-03 emitter; `RunFinished` — the engine, from the
  `RunOutcome` it already matches on (`engine/mod.rs:272-312`), with `status` one of
  `completed | failed | halted | awaiting_input`. `MiddlewareEvent.action` is a closed enum
  `{ finish, deny, redact, fail, retry, fallback }` (X-06: no stringly actions).
- **D-05: Payloads are bounded by construction; values are opt-in, redacted, then capped.**
  `FieldChange` carries `field`, `dispatch`, `writers` and `value_bytes` (serialized size of the
  new value) — never the value — unless `trace.state_values = true` (D-36, default `false`), in
  which case `value: Some(..)` is the serialized value passed through the existing
  `crates/paladin-llm/src/redaction.rs` helper **before** truncation to
  `trace.value_cap_bytes` (default 256) — the security-instructions ordering rule. The SSE wire
  never carries values regardless of that flag (`map_trace_event` strips `value`; Phase 27
  T-27-10-01 stands). `NodeProgress::StreamChunk` carries `bytes`, not text. The log and OTel
  sinks emit whatever the record carries; the config flag is the only switch.
- **D-06: The ordering test is a property of the dispatcher, exercised end to end.** The
  OBS-FR-03 test runs a 20-superstep graph with a `RecordingTraceSink`, asserts `seq` is
  strictly increasing and gapless when nothing dropped, then reruns with capacity 4 and a gated
  sink and asserts `gaps == RunFinished.trace_dropped_total == dispatcher.dropped_count()`.
  Cross-run interleaving is unordered by contract; the X-05 stress test (D-37) runs 16 concurrent
  runs on a multi-thread runtime through one `CompositeSink` and asserts exact per-run counts.

### Sink non-interference & fan-out (OBS-FR-02)

- **D-07: `RunFinished` is never the dropped event, and it carries the final drop count.**
  Drop-oldest already guarantees the newest event is enqueued; `RunFinished` is the last event a
  run emits, so `TraceDispatcher::emit` stamps `trace_dropped_total = dropped_count()` onto it
  at enqueue time. The first drop of a run also logs one `warn!` line (target `paladin::trace`)
  naming the thread and the configured capacity; subsequent drops are counted silently.
- **D-08: Panic isolation is applied twice, and `CompositeSink` lives in `paladin-ports`.**
  The dispatcher's consumer task wraps every `sink.on_event(record).await` in
  `futures::FutureExt::catch_unwind(AssertUnwindSafe(..))`, increments `sink_panics`, logs at
  `error` and **keeps the task alive** (today a panic would kill the consumer for the rest of the
  run silently). `CompositeSink { sinks: Vec<Arc<dyn TraceSink>> }` — in
  `crates/paladin-ports/src/output/trace_sink_port.rs`, zero new dependencies, so ADR-0015's
  allowlist is untouched — forwards each record to every child sequentially, each child under its
  own `catch_unwind`, so one panicking or erroring child neither starves nor fails its siblings;
  it returns `Ok(())` unless *every* child errored (diagnostic only, as before). The blocking
  test (`BlockingTraceSink`) and erroring test (`AlwaysErroringTraceSink`) from Phase 22 gain a
  `PanickingTraceSink` sibling in `engine/test_support.rs`. Timing assertion: a run with a
  sink that sleeps 500 ms per event completes within the sink-disabled time + 50 ms.
- **D-09: Keep the name `TraceSink`; the PRD's `TraceSinkPort` is satisfied by the module.**
  Seven implementors, four prior CONTEXT files and `MIGRATION.md` already say `TraceSink`; the
  house convention names the *module* `*_port.rs` and the trait without the suffix
  (`WaypointPort` is the exception, not the rule — cf. `TraceSink`, `LlmPort`). Recorded as a
  deliberate naming deviation from PRD text, not a gap.
- **D-10: Capacity is configurable from `TraceConfig.channel_capacity` (default 1024, D-36);
  the engine API is unchanged.** `WarEngine::with_trace_sink` stays; the facade passes capacity
  through the existing `TraceDispatcher::with_capacity`. The untraced path (no sink) stays
  zero-cost — Phase 22's must-have — which is why "default-on" for the log sink is a facade
  composition decision (D-11), not an engine default.

### Log & OTel sinks (OBS-02; OBS-FR-04, FR-05)

- **D-11: The log sink uses the `log` crate under target `paladin::trace`, one JSON line per
  record, `info` level, default-on in the facade's composition root only.**
  `src/infrastructure/telemetry/log_sink.rs` (PRD 07 names `infrastructure/telemetry` as the
  facade home) — `log::info!(target: "paladin::trace", "{}", serde_json::to_string(&record))`.
  The house stack is `log` + `env_logger` (57 files use `log::`, zero use `tracing::`; the
  `tracing-subscriber` line in the root manifest has no consumer in `src/` and is left alone
  under X-03). "Default-on" means: `paladin-server`'s run worker and `paladin-cli muster`
  attach it whenever `trace.log_sink` (default `true`) is set; operators silence it with
  `RUST_LOG=paladin::trace=off` or the config flag. `WarEngine` itself keeps no default sink
  (D-10). Rejected: `tracing` — it would be the first `tracing` consumer in the tree and a second
  logging stack for one sink.
- **D-12: The OTel exporter is `src/infrastructure/telemetry/otel_sink.rs` behind the umbrella
  feature `otel`, OTLP over HTTP/protobuf only.** Dependencies `opentelemetry`,
  `opentelemetry_sdk` and `opentelemetry-otlp` (`http-proto` + `reqwest-client` features; **no**
  `tonic`/gRPC — it roughly doubles the dependency graph for a transport nobody has asked for),
  all `optional = true`, gated by `otel`, absent from `default` and from `full`'s implicit set
  per X-11.4 (mirroring `storage-postgres`'s explicit note in `Cargo.toml`). Versions are the
  researcher's to pin against MSRV 1.88, `cargo deny` and `cargo audit`; the OTel family is the
  program's named MSRV-bump risk (X-11) and any bump is a stop-and-flag item recorded in §9.3.
  Span model, from the record stream alone (no engine internals): a `run` root span per
  `thread_id` opened on `RunStarted` with `start_time = at` and closed on `RunFinished`
  (`end_with_timestamp`); one child span per `(node_id, attempt)` opened on `NodeStarted` and
  closed on `NodeFinished` with attributes `{node_id, superstep, attempt, outcome, tokens,
  cache_hit, muster_task_key}` — retries are sibling spans; `EdgeEvaluated`, `DeltaMerged`,
  `ParleyRaised`, `FallbackHop` and `MiddlewareEvent` are span **events** on the enclosing span
  (the attempt span when a `node_id` is present, else the run span); resource
  `service.name` and OTLP `endpoint`/`headers` come from `OtelConfig` (D-36, X-09). A record
  arriving for a span that was never opened (a dropped `NodeStarted`) opens a synthetic span
  flagged `paladin.trace.partial = true` rather than being discarded.
- **D-13: Two-layer verification: tree shape in-process, transport against an axum stub.**
  (a) Shape — the sink is constructed over `opentelemetry_sdk`'s `InMemorySpanExporter`
  (its `testing` feature) and the branch + retry + muster fixture run is asserted as a span
  tree: one root, N attempt children, retried node has sibling spans with `attempt = 1, 2`,
  edge/delta events on the right spans. (b) Transport — an `#[cfg(feature = "otel")]`
  integration test starts an axum `POST /v1/traces` stub (the hermetic `FixtureServer` pattern
  from Phase 3), points `OtelConfig.endpoint` at it, and asserts a request arrived with
  `Content-Type: application/x-protobuf`, the configured headers and a body decoding to a
  resource carrying the configured `service.name`. CI runs (b) in the existing feature-flags job
  with `--features otel`. Rejected: a real collector container — Docker is unavailable in the
  devcontainer (24-CONTEXT D-28) and adds nothing (a) + (b) do not already prove.

### SSE collapse & trace persistence (OBS-FR-06, FR-07)

- **D-14: The bus has one producer again.** The worker (`src/application/services/run/
  worker.rs:856-903, 1067`) stops publishing `parley`/`done`/`error` directly;
  `map_trace_event` (`events.rs:87`) becomes total over the seven wire names —
  `SuperstepStarted → superstep`, `NodeStarted → node_started`, `NodeFinished →
  node_finished`, `DeltaMerged → state_delta`, `ParleyRaised → parley`, `RunFinished{status:
  completed|halted|awaiting_input} → done`, `RunFinished{status: failed} → error`, everything
  else `None` — and the existing "exactly four of eight" unit test becomes "exactly seven of
  twelve". The seven wire names, their payload fields and the OpenAPI schema are the published
  contract (27-CONTEXT D-25, X-03) and do not change. `RunEventBusSink` stays a `TraceSink` (it
  receives stamped `TraceRecord`s now), so OBS-FR-06's "one implementation, two consumers; no
  second pathway" holds by construction. — **Reversibility:** reversible — the worker's
  publish sites are the only edit.
- **D-15: The wire `seq` stays the bus's own dense counter; the payload gains `trace_seq`.**
  `RunStreamEvent.seq` is documented as "monotonically increasing per-run, starting at 1" and
  the generated Python/TypeScript clients consume it; only the mapped subset of trace records
  reaches the wire, so adopting the trace `seq` verbatim would introduce gaps into a published
  field. Instead every live/replayed event's `payload` carries `trace_seq` (additive, documented
  in OpenAPI, golden re-blessed with `UPDATE_OPENAPI=1`), which is what correlates an SSE event
  with the same record in logs, OTel and `run_traces`. Degraded-mode events synthesized from
  Waypoints carry no `trace_seq`.
- **D-16: `RunStreamMode` gains `Replay`.** When `trace.persist` is on and a requested run is
  terminal (or executing elsewhere) with rows in `run_traces`, `RunEventStreamService` replays
  them through the same `map_trace_event` with `mode: replay`, the original `at` and
  `trace_seq`, then terminates with `done`/`error` as today — PRD 07's "degraded mode upgrades to
  full fidelity". No rows (persistence off, or a run older than retention) → today's degraded
  Waypoint-polling path, unchanged. `RunStreamMode` is new-in-0.10 (Phase 27) so the variant is
  additive, and the enum is marked `#[non_exhaustive]` in the same change (X-10.2 spirit). The
  OpenAPI enum, the mdBook run-streaming page and the `sdk-clients` smoke test are updated
  together.
- **D-17: A new `RunTracePort` in `paladin-ports`, three adapters in `paladin-storage`
  mirroring `waypoint/`, batched per superstep, pruned with Waypoint retention.**
  `RunTracePort: Send + Sync { async fn append(&self, records: &[TraceRecord]) -> Result<(),
  RunTraceError>; async fn read(&self, thread: &ThreadId, after_seq: u64, limit: u32) ->
  Result<Vec<TraceRecord>, RunTraceError>; async fn prune_thread(&self, thread: &ThreadId,
  before_superstep: u64) -> Result<u64, RunTraceError> }` (`#[non_exhaustive]` error enum,
  ADR-0015 imports only). Adapters `crates/paladin-storage/src/run_trace/{mod,in_memory,sqlite,
  postgres,contract_tests}.rs` under the existing `sqlite` (always-on) and `postgres` features —
  **no new feature**; migrations `crates/paladin-storage/migrations/{sqlite,postgres}/NNN_create_run_traces_table.sql`
  (next free numbers, researcher confirms) with table `run_traces (thread_id TEXT NOT NULL,
  seq BIGINT NOT NULL, run_id TEXT NULL, superstep BIGINT NOT NULL, at TIMESTAMPTZ/TEXT NOT
  NULL, schema_version TEXT NOT NULL, record TEXT/JSONB NOT NULL, PRIMARY KEY (thread_id,
  seq))`, append-only. X-04: the persisted row carries `schema_version` (`TRACE_SCHEMA_VERSION`);
  reading an unknown newer version is `RunTraceError::UnsupportedSchemaVersion { found }`.
  `PersistingTraceSink` (facade telemetry) buffers records and flushes on `WaypointSaved`
  (the superstep boundary), on `RunFinished`, and whenever the buffer reaches 256 records (bounds
  memory for a long superstep); a crash loses at most the un-flushed tail of one superstep —
  documented: the Waypoint is the durability truth, the trace is best-effort. Retention:
  `WaypointRetentionService` (`src/application/services/waypoint_retention.rs`) calls
  `prune_thread` with the same age/count bounds it applies to Waypoints — "shares ENG-FR-18
  policy" means one config, one routine, two ports; no `run_traces`-specific tunable. Postgres
  contract suite is Tier 2 (CI `postgres-integration` job only, 24-CONTEXT D-28). Rejected:
  extending `WaypointPort` with trace methods — different lifetime, different write pattern, and
  it would make every `WaypointPort` implementor carry trace storage.
  — **Reversibility:** one-way — a persisted table with a schema version.

### Graph export & execution overlay (OBS-03; OBS-FR-08, FR-09)

- **D-18: Exporters live in `paladin-battalion` as pure functions over a `GraphShape`.**
  New `crates/paladin-battalion/src/engine/export/{mod,shape,mermaid,dot,overlay}.rs`.
  `GraphShape { nodes: Vec<ShapeNode { id, kind: ShapeKind, deferred: bool, worker_template:
  bool, subgraph: Option<Box<GraphShape>> }>, edges: Vec<ShapeEdge { from, to, condition:
  Option<String> }>, entry: Vec<NodeId> }` with `ShapeKind = Paladin | Function | Gate |
  Workflow | WorkerTemplate`, built by `GraphShape::from_doc(&WarGraphDoc)` **and**
  `GraphShape::from_graph(&WarGraph)`; `to_mermaid(&GraphShape) -> String` and `to_dot(&GraphShape)
  -> String`. PRD 07 says `WarGraphDoc →`, but 27-CONTEXT D-33 scoped the document to
  `{Paladin, Gate, Workflow}` — no Function nodes, no worker templates — so a doc-only exporter
  could render neither the PRD's own "muster" golden fixture nor any of the three E2E graphs,
  which are code-built. The compiled `WarGraph` carries everything the badges need. Rejected: a
  Function-node registry in `WarGraphDoc` (a new capability, deferred in 27-CONTEXT).
  — **Reversibility:** costly — `GraphShape` is the exporter, overlay and inspector contract.
- **D-19: Rendering rules, fixed for the golden files.** Mermaid: `flowchart TD`; node ids
  sanitized to `n{i}` in declaration order with the real id in the label; labels
  `"<id> «paladin»"` (badge in guillemets), Gate `{ }` diamond, Workflow as a `subgraph
  <id>` cluster rendering the nested shape, worker templates and deferred nodes in dashed style
  (`classDef`), conditional edges labeled `-- contains("x") -->` / `-- regex --> ` / `-- custom(name)
  -->`, `Always` unlabeled; a `classDef` block for the five kinds. DOT: `digraph`, quoted ids,
  `subgraph cluster_<id>` for Workflow, `shape=diamond` Gate, `style=dashed` for worker
  templates/deferred, edge `label` for conditions. Output is byte-deterministic (declaration
  order, no `HashMap` iteration). The existing Maneuver `FlowVisualizer` (`flowchart LR`,
  `crates/paladin-battalion/src/maneuver/visualizer.rs`) is a different surface and stays
  untouched (X-03).
- **D-20: Goldens under `crates/paladin-battalion/tests/golden/export/` with an
  `UPDATE_GOLDEN=1` bless idiom** mirroring `UPDATE_OPENAPI=1` (`crates/paladin-web/openapi.rs`,
  `Makefile:370`) and the schema bless in `graph_doc_round_trip.rs`; a `make bless-golden`
  target. Fixtures: linear, branch+join, loop, muster, subgraph — the first three and the
  subgraph as `WarGraphDoc` files under the existing `tests/fixtures/graph_docs/` (both
  `from_doc` and `from_graph` of the compiled doc must produce the same shape — asserted), muster
  as a code-built `WarGraph`. The overlay golden is the branching fixture with a scripted run.
- **D-21: The overlay is `GraphShape` + `ExecutionOverlay`, Mermaid only, with Waypoint history
  as the always-available source and the persisted trace as the upgrade.** `ExecutionOverlay {
  visits: BTreeMap<NodeId, Vec<Visit { superstep, attempt, outcome, duration_ms, tokens,
  cache_hit }>>, fired_edges: BTreeSet<(NodeId, NodeId)>, evaluated_edges: BTreeSet<(NodeId,
  NodeId)>, source: Waypoints | Trace, observed_only: bool }`. From Waypoints
  (`WaypointPort::history` + each `Waypoint.completed: Vec<NodeExecutionRecord>` and
  `vanguard`): visits from the records, fired edges **derived** from consecutive
  completed→vanguard transitions (a completed `A` in superstep *n* and `C` in the vanguard of
  *n+1* with an `A→C` edge in the shape fires it), `evaluated_edges` empty. From `run_traces`:
  visits from `NodeStarted`/`NodeFinished`, fired and evaluated edges exact from
  `EdgeEvaluated`, rendered bold (fired) and dotted (evaluated, not fired). `to_mermaid_overlay(
  &GraphShape, &ExecutionOverlay)` colors nodes by last outcome via `classDef` (`success`,
  `failed`, `parleyed`, `skipped`, `cache_hit`), appends `×N` visit counts to loop nodes and
  `<duration>ms · <tokens>tok` to labels. A DOT overlay is deferred (PRD names Mermaid only).
- **D-22: When no static graph is available for a thread, `run export` renders the observed
  subgraph and says so.** Waypoints carry only a `graph_fingerprint`, not the graph. Resolution
  order: `--graph <doc file>` on the CLI → the run's assistant version's `WarGraphDoc` when the
  thread belongs to a run (via `RunRepositoryPort` + the assistant store) → `observed_only =
  true`, building the shape from the nodes and derived edges actually seen, with the diagram
  title `(observed nodes only — no graph document available)`. The three E2E threads fall in
  the third bucket, which is acceptable: the acceptance question is answered by the overlay,
  not by unexecuted nodes.
- **D-23: CLI commands are in-process, under the existing `cli` feature and command module
  layout.** `src/application/cli/commands/graph.rs` — `paladin-cli graph export --format
  mermaid|dot (<FILE> | --assistant <id>[@<version>]) [--out <path>]`, `<FILE>` a `WarGraphDoc`
  JSON/YAML; `--assistant` resolves through the store the loaded config names (SQLite locally,
  Postgres by URL) exactly as the server would. `src/application/cli/commands/run.rs` —
  `paladin-cli run export --thread <id> [--waypoint <id>] [--run <run_id>] [--graph <FILE>]
  [--out <path>]`; `--run` is sugar that resolves the thread and graph from the run row. Both
  write to stdout by default (pipe-friendly, no colour), use the existing `CliError` and
  formatter conventions, and are covered by `insta` snapshot tests in `tests/cli/` like the
  other commands. `--assistant` and `--run` reach storage through ports, never through HTTP
  (the CLI has no HTTP client and ADR-0023 keeps it that way).

### The `dev-ui` inspector (OBS-FR-10)

- **D-24: `paladin-web` renders what a new input port hands it; it never learns the graph
  vocabulary.** ADR-0031 forbids `paladin-web → paladin-battalion` in the default build and
  27-CONTEXT D-27 set the pattern: a core-typed input port. New
  `crates/paladin-ports/src/input/run_inspector_port.rs` — `RunInspectorPort: Send + Sync {
  async fn inspect(&self, thread: &ThreadId) -> Result<InspectorView, InspectorError> }` with
  `InspectorView { thread_id, run_id: Option<RunId>, status, mermaid: String, observed_only:
  bool, source: Waypoints | Trace, supersteps: Vec<SuperstepRow { superstep, waypoint_id,
  vanguard: Vec<NodeId>, completed: Vec<CompletedRow { node_id, attempt, outcome, duration_ms,
  token_count, cache_hit }>, field_changes: Vec<FieldName>, fired_edges: Vec<(NodeId, NodeId)>
  }>, visits: Vec<VisitSummary { node_id, count, supersteps: Vec<u64> }> }` (core value types,
  ADR-0016). The facade implements it in `src/application/services/run/inspector.rs` on top of
  D-21/D-22, deriving `field_changes` by diffing consecutive Waypoint Battlefields by field name
  (names only) when no trace is persisted, and from `DeltaMerged` when it is.
- **D-25: One route, admin-scoped, feature-gated, outside the OpenAPI document.**
  `GET /v1/dev-ui/threads/{id}` in `crates/paladin-web/src/dev_ui_controller.rs`, mounted on the
  `/v1` router (ADR-0037) under the same `require_auth` + `require_admin` layers the admin
  routes use (`app.rs:53-57`) — it exposes state field names and the run's structure, which is
  operator information. Compiled only with `paladin-web` feature `dev-ui`, forwarded from a new
  umbrella feature `dev-ui` (default off, not in `full`, X-07/X-10.7). The route carries **no**
  `#[utoipa::path]` and is documented in mdBook instead, so `openapi.json` and the `sdk-clients`
  gate stay feature-independent — a dev tool page is not API surface. Response `text/html`;
  errors reuse the `{ error: { code, message, details } }` envelope (`404` unknown thread, `501`
  when no `RunInspectorPort` is wired, matching the `NotWired` precedent).
- **D-26: The page is one static HTML template with the data embedded, no fetches, no build
  pipeline.** `crates/paladin-web/src/dev_ui/inspector.html` via `include_str!`, the
  `InspectorView` JSON injected into `<script id="inspector-data" type="application/json">`
  (HTML-escaped `</` and `<!--`), inline vanilla JS rendering: the Mermaid overlay diagram, a
  per-node visits panel ("node X ran 3 times: supersteps 2, 4, 6 — entered via `loop → X`
  each time; outcomes ✓ ✓ ✓"), the fired-edge list per superstep ("superstep 3: `check →
  retry` fired; `check → done` evaluated, not fired" when the trace source is available), and
  the superstep table with field-change lists. Mermaid is loaded from
  `web_server.dev_ui.mermaid_url` (default the jsDelivr `mermaid@11` ESM bundle) — CDN-
  configurable, **not vendored**: a multi-megabyte JS asset in a published crate's `include`
  list is a cost nobody asked for, and an air-gapped operator points the URL at a local copy
  (documented). The PRD's acceptance test is DOM-level on the embedded payload: the smoke test
  requests the page through the existing `oneshot` harness with an in-test `RunInspectorPort`
  returning the branching fixture view and asserts the embedded JSON carries the fired branch
  and the ×3 visit summary — no browser. Live SSE attach is deferred (see Deferred Ideas).

### The `paladin-eval` crate (OBS-04; OBS-FR-11…15)

- **D-27: `crates/paladin-eval` is a facade-tier tool crate — it depends downward on the leaf
  crates, nothing depends on it except as a dev-dependency.** Dependencies: `paladin-core`,
  `paladin-ports`, `paladin-battalion`, `paladin-llm` (`mock` feature, already default),
  `paladin-storage` (`sqlite`, for the E2E fixtures' temp-file stores), `serde`, `serde_yaml`,
  `serde_json`, `regex`, `glob`, `libtest-mimic` (D-32). It never depends on the facade
  `paladin-ai` (the facade takes it as a dev-dependency for the dogfood scenarios, and a
  dependency cycle through dev-deps is legal but a maintenance trap). ADR-0031's invariant
  ("no extracted crate depends on another extracted crate in its default build") is scoped to
  the *extracted* leaf crates; `paladin-eval` is a composition crate like `doc-examples`
  (`crates/doc-examples/Cargo.toml` depends on six workspace crates). It is **published**
  (`publish = true`, version with the workspace, added to the `semver`/`sdk`/`api-surface`
  crate lists and the release pipeline's eleven → twelve crates) because a downstream team's
  `[dev-dependencies]` is the whole point of "dev-dependency-oriented". A one-page ADR
  (`.planning/decisions/0047-paladin-eval-composition-crate.md`, next free number per
  `PROMOTION.md`) records the classification so a future reader of ADR-0031 does not read the
  dependency edges as a violation. — **Reversibility:** one-way once published to crates.io
  under that name and dependency shape.
- **D-28: Scenario file = YAML (`.eval.yaml`, JSON accepted), `schema_version: "1"`, one
  target, many cases.** Shape:
  `target: { graph_doc: <path> } | { registered: <name> }` (D-33); `store: in_memory |
  sqlite_temp` (default `in_memory`); `llm:` a global script and/or per-node scripts keyed by
  the Paladin node id, each a list of `{ text | tool_call: {name, arguments} | error: <LlmError
  kind> }` consumed sequence-per-call, plus `match: [{ prompt_contains: "...", response: {...}
  }]` rules checked before the sequence; `cases: [{ name, input: { <field>: <json> },
  interrupt_after_superstep: N?, parley_responses: [{ kind, value }]?, llm: <case overrides>?,
  assertions: [ ... ] }]`; `live: { allow_content_assertions: bool }` (D-35). Assertions are a
  tagged list using PRD 07 OBS-FR-12's names verbatim (`final_state_field_equals`,
  `final_state_field_matches`, `field_json_path_equals`, `node_executed: { node, times: {exact|
  min|max} }`, `node_not_executed`, `edge_fired: {from, to}`, `route_taken: [nodes]` as a
  subsequence of the `NodeStarted` order, `run_status`, `total_tokens_max`, `supersteps_max`,
  `parley_raised: {kind, node}`, `final_state_snapshot`), plus `custom(fn)` in the Rust API only.
  `interrupt_after_superstep` + `parley_responses` are what E2E-1 and E2E-2 need (D-34) and are
  general runtime controls, not fixture hacks. A JSON Schema for the format is derived with
  `schemars` and golden-checked exactly like `wargraph-doc.schema.json` (27-CONTEXT D-34).
  — **Reversibility:** costly — a file format users write by hand; additive evolution under
  `schema_version` only.
- **D-29: Assertions read the captured `TraceRecord`s and the final Battlefield, nothing
  else.** The runner installs a `CapturingSink` (a `TraceSink` collecting `Vec<TraceRecord>`)
  through `WarEngine::with_trace_sink`, runs the case, then evaluates every assertion against
  `(records, final_battlefield, outcome)`. `edge_fired` needs `EdgeEvaluated` (D-04), `parley_
  raised` needs `ParleyRaised`, `total_tokens_max` reads `RunFinished.total_tokens` — the
  harness is the first consumer that proves the twelve-variant enum is sufficient, which is
  PRD 07's intent ("no reaching into engine internals"). Every assertion has a `render_failure()`
  producing an actionable message (expected vs observed, the relevant `seq` range, the node
  visit table for `node_executed`), snapshot-tested with `insta` (already a dev-dependency,
  used by `tests/cli/`).
- **D-30: `paladin-eval` owns its scripted LLM; `paladin-llm`'s mock is not extended.**
  `ScenarioLlm` (implements `LlmPort`, in `crates/paladin-eval/src/scripted_llm.rs`) provides
  per-node routing (one instance per Paladin node, built from the doc/constructor), sequence
  scripts, prompt-substring `match` rules, and records every request for failure rendering.
  `MockLlmAdapter`/`MockScriptEntry` (`crates/paladin-llm/src/mock.rs`) are public and may be
  pre-existing at `v0.9.0`; touching them would be an X-10 register event for a capability the
  harness can own outright. The researcher confirms whether they are pre-existing; either way
  they stay untouched.
- **D-31: `graph_doc` targets compile through `EngineRegistries` the scenario declares by
  name; `registered` targets are Rust closures.** A file target names the doc path and an
  optional `registries: <name>` the host test binary registered
  (`ScenarioRunner::register_registries("default", || EngineRegistries::default())`); the
  Paladin nodes' `LlmPort`s are replaced by `ScenarioLlm` instances at compile time via a
  `PaladinPort` shim the runner owns. Registered targets: `ScenarioRunner::register_graph(
  "e2e1", |scripted: &ScriptedPorts| -> WarGraph)` receives the per-node scripted ports so
  code-built graphs (Function nodes, worker templates, custom edge evaluators, Aegis handlers)
  are fully expressible.
- **D-32: The runner is a `libtest-mimic` custom harness — one runtime-discovered test per
  case, no proc macro.** `paladin_eval::eval_scenarios!("evals/**/*.eval.yaml")` expands to a
  `fn main()` for a `[[test]] name = "evals" harness = false` target that globs the pattern at
  runtime, registers the host's graph constructors (D-31) and hands one `Trial` per
  `(file, case)` to `libtest-mimic` — so `cargo test --test evals e2e1::approve` filters, the
  output looks like `cargo test`, CI needs nothing new, and there is no proc-macro crate to
  publish. Rejected: a proc macro reading files at compile time (a second crate, recompiles on
  every scenario edit, and glob expansion in `proc_macro` is fragile); `macro_rules!` with
  explicit paths (one test per *file*, not per case — fails the FR).
- **D-33: `paladin-cli eval run <glob> [--repeat N] [--bless] [--live] [--registries <name>]`
  in `src/application/cli/commands/eval.rs`, behind `cli`, with `paladin-eval` an optional
  dependency the `cli` feature enables.** `--repeat N` runs every case N times sequentially and
  prints a per-case pass rate; **any divergence across repeats** (not N/N identical verdicts)
  exits non-zero with the differing `seq` ranges — nondeterminism with scripted mocks is a bug
  to surface, per the PRD. `--bless` writes `<scenario>.<case>.snap.json` beside the scenario for
  `final_state_snapshot` assertions (the `UPDATE_*=1` env idiom's CLI cousin). The CLI can only
  run file targets and `registered` targets whose constructors ship in the facade (the E2E
  three, registered in the facade's eval registry), which is documented.
- **D-34: E2E-1/2/3 become `evals/e2e-1-crash-resume.eval.yaml`, `evals/e2e-2-approval-gate.
  eval.yaml`, `evals/e2e-3-map-reduce-fault-tolerance.eval.yaml` at the repo root, with
  `registered` targets whose constructors are shared with the integration tests.** The graph
  builders in the three `tests/integration/e2e_*_test.rs` files are refactored (behavior-
  preserving) into `tests/helpers/e2e_fixtures.rs` and used by both the integration tests (which
  must stay green as integration tests — SHIP-03) and the facade's `tests/evals.rs` harness.
  E2E-1 uses `store: sqlite_temp` + `interrupt_after_superstep: 3` (the runner drops the engine,
  builds a fresh one over the same temp store and resumes — the same simulated-crash technique
  the integration test documents) with assertions `node_executed` exact counts, `final_state_
  snapshot` against the control run and `supersteps_max`; E2E-2 has two cases (`approve`,
  `deny`) using `parley_responses` and `route_taken`; E2E-3 scripts the failing worker with
  `[error: transient, error: transient, text: ...]` and asserts `node_executed: { node: worker,
  times: { min: 7 } }` (5 workers + 2 retries as attempts are visible per `NodeStarted`),
  `node_executed: { node: aggregate, times: { exact: 1 } }` and the 5-element list field.
  Both the harness and the integration tests run in the default `cargo test --workspace`.
- **D-35: Live mode is `--live` + `PALADIN_EVAL_LIVE=1` + provider env keys, structural
  assertions only unless `allow_content_assertions: true`, never in default CI.** Providers come
  from `paladin-llm`'s `provider_factory` with the keys ADR-0012 already governs for live-API
  tests; content assertions (`final_state_field_equals`, `_matches`, `field_json_path_equals`,
  `final_state_snapshot`) are skipped with a visible `SKIPPED (content assertion, live mode)`
  unless the scenario opts in. Documented as the promotion path: script-mocked in CI, live smoke
  pre-release, in the eval-harness mdBook page.

### Config, docs & program bookkeeping (X-07 … X-11)

- **D-36: `src/config/trace.rs` — `TraceConfig` with `Default`, `validate()`, `EnvOverridable`
  (`PALADIN_TRACE_*`), mirroring `RunStreamConfig` and `WaypointRetentionConfig`.** Fields:
  `log_sink: bool = true`, `channel_capacity: usize = 1024`, `persist: bool = false`,
  `state_values: bool = false`, `value_cap_bytes: usize = 256`, `heartbeat_interval_secs: u64 =
  5`, `otel: OtelConfig { enabled: bool = false, endpoint: String = "http://localhost:4318/v1/
  traces", headers: BTreeMap<String, String> = {}, service_name: String = "paladin" }`.
  `validate()` rejects `channel_capacity == 0`, `value_cap_bytes == 0`, a non-`http(s)` endpoint
  when `otel.enabled`, and `otel.enabled` on a build without the `otel` feature (a typed
  `ConfigError::FeatureNotCompiled { feature: "otel" }`, never a silent no-op). `otel.headers`
  values are secrets by assumption: `OtelConfig` has a manual `Debug` that redacts them, and
  no log line interpolates them (security instructions). `web_server.dev_ui.mermaid_url:
  String` (default the jsDelivr URL) joins `WebServerConfig`. `config.example.yml` and
  `config.test.yml` gain the sections with every default spelled out.
- **D-37: The bench and the stress test are deliverables, not nice-to-haves.** `benches/
  engine_benchmarks.rs::bench_superstep_cost` gains sink variants — `none`, `log_sink`,
  `composite(log + noop)` — and PRD 07 acceptance 6's "≤ 3 % superstep overhead vs
  sink-disabled" is measured and recorded in the phase's verification evidence (not gated in CI
  — criterion numbers on shared runners are noise; the record is the gate). X-05: one
  `#[tokio::test(flavor = "multi_thread")]` stress test in `crates/paladin-battalion/src/
  engine/hooks.rs` runs 16 concurrent traced runs through one `CompositeSink` of two recording
  children, asserts exact per-run record counts and gapless per-run `seq` on both children, with
  the `listener.rs` timeout-guard pattern.
- **D-38: Docs (X-08).** Three new mdBook pages wired into `docs/src/SUMMARY.md`:
  `docs/src/operations/observability.md` (trace model with the twelve variants and the envelope,
  sinks, OTel setup with a collector example, `run_traces` persistence and retention, drop
  accounting, the `trace_seq` correlation story), `docs/src/user-guides/graph-visualization.md`
  (`graph export`/`run export`, the badge/colour legend, the inspector page and its admin
  gating, `mermaid_url` for air-gapped hosts), `docs/src/user-guides/eval-harness.md` (file
  format with the JSON Schema link, every assertion with an example, the `evals` harness setup,
  `--repeat`/`--bless`/`--live`, the promotion path). Crate-level rustdoc for `paladin-eval`
  with doc tests on every public item; `cargo doc` adds no broken intra-doc links (the ~60
  pre-existing rustdoc warnings carried from Phase 26 are not this phase's, but none may be
  added).
- **D-39: `MIGRATION.md` §9 rows this phase owes (SHIP-01 fails on any "TBD").** §9.2: a
  deliberate-zero note in the Phase 24/25 form for every new-in-0.10 type touched (`TraceEvent`,
  `TraceSink`, `FallbackLlmAdapter::with_trace_sink → with_trace_emitter`, `RunStreamMode`,
  `RunStreamEvent` payload, `NodeContext` if it gains a trace handle, `WarGraphDoc`
  untouched) and an explicit row for **any** pre-existing-at-`v0.9.0` public type touched —
  expected count zero (`MockLlmAdapter` deliberately untouched, D-30); the planner inventories
  first (X-10.1). §9.3: `opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp` (behind
  `otel`), `libtest-mimic`, `glob` (and `serde_yaml`'s new consumer), each with the MSRV
  verification result; any MSRV bump proposal is a stop-and-flag item. §9.4: `run_traces`
  (sqlite + postgres), Citadel files unchanged. §9.5: `trace.*`, `web_server.dev_ui.*`,
  `PALADIN_TRACE_*`, `PALADIN_EVAL_LIVE`. §9.6: `trace_seq` payload field and `replay` mode on
  `GET /v1/runs/{id}/stream`; the `dev-ui` page listed as a non-API route. §9.7: none.
- **D-40: Close-out gates on the phase's final commit, in this order:** `cargo test --workspace`
  (incl. `--test evals`), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings` (with and without `otel`/`dev-ui`), `cargo semver-checks` against the published
  `0.9.0` crates for all twelve crates (the new crate has no baseline — recorded), the `msrv`
  job at 1.88 with `--all-features`, `make security`, coverage ≥ 82 % (ADR-0006; the eval crate
  and its scenarios count toward the workspace figure), `scripts/check-api-surface.sh` with
  `.project/current-exports.txt` regenerated (the Phase 25/26 carried concern — do not leave it
  red), `UPDATE_OPENAPI=1` re-bless and the `sdk-clients` CI job, `cargo tree -e features` on a
  default build showing no `opentelemetry*`/`libtest-mimic` (X-11.4), and the feature-flags CI
  job covering `otel` and `dev-ui` individually. Docker-dependent tiers (Postgres `run_traces`
  contract suite) are read green from CI runs and recorded in the phase's CI-evidence file, as
  Phase 27 did — never marked passed locally.
- **D-41: Plan decomposition follows PRD 07 §4's TDD order.** Suggested waves: (1) core trace
  types + envelope + `TraceEmitter` + dispatcher stamping/panic isolation/`CompositeSink` +
  ordering/overflow/stress tests + config; (2) the four new producers (edge, parley, heartbeat,
  middleware/progress) + log sink + facade composition + bench; (3) OTel sink + in-memory shape
  test + axum stub; (4) SSE collapse + `trace_seq` + `RunTracePort` + adapters + migrations +
  `PersistingTraceSink` + replay mode + retention; (5) `GraphShape` + Mermaid/DOT + goldens +
  overlay + CLI `graph export`/`run export`; (6) `RunInspectorPort` + facade impl + `dev-ui`
  page + smoke test; (7) `paladin-eval` format + `ScenarioLlm` + assertions + runner + CLI
  `eval run` + `--repeat`/`--bless`/`--live`; (8) E2E-1/2/3 dogfood + docs + `MIGRATION.md` +
  ADR-0047 + gates. Waves 3, 5 and 7 are independent of each other after wave 1 and may run in
  parallel executors.

### Claude's Discretion

- Exact file splits inside `engine/export/`, `infrastructure/telemetry/` and `paladin-eval/src/`.
- Whether `NodeContext` gains a `trace: Arc<dyn TraceEmitter>` accessor (so a `StateNode` body
  can emit `NodeProgress::ToolCall` itself) — cheap and additive on a new-in-0.10 type; recommended
  if any producer in D-04 turns out to need the node id the engine already knows.
- The `heartbeat_interval` rate-limit mechanism (token bucket vs last-emitted timestamp).
- Mermaid `classDef` colours and the exact badge glyphs — golden files freeze whatever is chosen.
- The inspector page's visual layout beyond the four panels D-26 names (no design system
  applies; it is a dev tool). A `UI hint: yes` phase, but PRD 07 says "explicitly minimal";
  `/gsd-ui-phase` is not required for a single admin-gated HTML page.
- `libtest-mimic` `Trial` naming (`<file-stem>::<case>` recommended) and how `--repeat` output
  is tabulated in the CLI.
- Whether the three E2E `.eval.yaml` files also carry a `graph_doc` twin where expressible
  (E2E-2's approval gate is Paladin + Gate and *is* doc-expressible) as a format demonstration.
- Whether `run export` gains `--format dot` for the static shape when `--overlay` is off (cheap,
  but PRD names Mermaid for the overlay only).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase source of truth (behavior)

- `.project/v0.10.0/07-observability-tooling.md` — **the** specification for this phase. §1
  problem statement; §2.1 OBS-FR-01…03 (the twelve-variant `TraceEvent` list, `seq`, bounded
  payloads, the sink port and non-interference, ordering); §2.2 OBS-FR-04…07 (log, OTel,
  SSE bridge, Waypoint trace persistence); §2.3 OBS-FR-08…10 (static export, overlay, inspector);
  §2.4 OBS-FR-11…15 (scenario format, assertions, runner/CLI, live mode, dogfood); §3 the seven
  acceptance criteria (incl. the ≤ 3 % bench and the X-10/X-11 gate); §4 the TDD ordering D-41
  follows; §5 out of scope.
- `.planning/REQUIREMENTS.md` lines 259-282 — OBS-01…04 as written for this milestone (the
  wording the roadmap's success criteria were derived from); lines 296-310 SHIP-01…04 (what
  Phase 29 will check this phase left complete).
- `.planning/ROADMAP.md` lines 659-670 — Phase 28's goal, dependencies (Phase 22 trace seam,
  Phase 27 `WarGraphDoc`) and the four success criteria that must be TRUE at verification.
- `.project/v0.10.0/00-program-overview.md` §3 — **X-01…X-11, non-negotiable.** X-01 hexagonal
  rule (D-01, D-17, D-24, D-27); X-02 TDD + 82 %; X-03 backward compatibility and stop-and-flag;
  X-04 `schema_version` (D-17); X-05 `Send + Sync` + multi-thread stress (D-37); X-06 structured
  errors (D-04, D-17); X-07 feature gates (D-12, D-25); X-08 docs (D-38); X-09 config structs
  (D-36); X-10 the §9.2 register (D-39); X-11 MSRV **1.88** (the text says 1.85; superseded
  since Phase 22.1) and dependency discipline (D-12, D-27, D-40). §6 E2E-1/2/3 — the scenarios
  D-34 dogfoods. §9 the `MIGRATION.md` structure D-39 fills.
- `.project/v0.10.0/08-traceability-matrix.md` — the OBS rows that gain test anchors.

### Program deliverable this phase appends to

- `MIGRATION.md` — §9.2 (the deliberate-zero note form used by the Phase 24 and Phase 25 notes
  at lines 196 and 198 is the form D-39 follows), §9.3, §9.4, §9.5, §9.6.
- `.cargo/semver-checks-allowlist.toml` — entry schema; expected untouched (no deliberate-
  breaking entries from this phase).
- `.planning/decisions/PROMOTION.md` — the next free ADR number for D-27's ADR.

### Prior-phase decisions that constrain this phase

- `.planning/phases/27-platform-api/27-CONTEXT.md` — **D-24…D-27** (the bus is a `TraceSink`;
  the seven frozen wire names and the one-function mapping that is this phase's single edit
  point, with the research correction that today's enum cannot produce all seven; degraded mode;
  `RunEventStreamPort`), **D-33…D-35** (`WarGraphDoc` home, the `{Paladin, Gate, Workflow}`
  scope correction that motivates D-18, the `schemars` golden idiom D-28 reuses, two-process
  fingerprint proof), D-44…D-47 (route/scope/pagination/error-envelope conventions D-25 reuses),
  D-53/D-54 (what a phase owes `MIGRATION.md` and the gates). Its Deferred Ideas name OBS-01/02
  and OBS-03 as this phase's.
- `.planning/phases/26-agent-runtime-enhancements/26-CONTEXT.md` — D-01 (the middleware chain
  lives in the facade beside `PaladinExecutionService` — the `MiddlewareEvent` producer site,
  D-04); its deferred "a `TraceEvent` for summarization degradation, middleware `Finish`/`Fail`,
  vault recall and structured repair attempts — OBS-01/02" is the `MiddlewareEvent` variant.
- `.planning/phases/25-node-level-fault-tolerance/25-CONTEXT.md` — D-16 (`attempt`, `cache_hit`
  added to `NodeStarted`/`NodeFinished` "so Phase 28 renames nothing"), D-18 (`HeartbeatHandle`
  on `NodeContext` — the `NodeProgress::Heartbeat` source), D-25 (`FallbackHop` emitted below the
  engine with `node_id: None`, "Phase 28 may enrich" — D-03 changes the handle type, Deferred Ideas defers
  the enrichment); its deferred "full PRD 07 `NodeFinished` payload, `EdgeEvaluated`, the
  authoritative enum, OTel span-per-attempt" is this phase.
- `.planning/phases/24-pause-resume-history-graceful-shutdown/24-CONTEXT.md` — the
  Claude's-discretion resolution that `TraceEvent` gained no `ParleyRaised`/`RunHalted` there
  ("Phase 28 owns the authoritative enum"), D-28 (test tiers: Docker unavailable locally,
  Postgres is Tier 2/CI-only).
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-CONTEXT.md` — D-14
  (a stored payload contract needs a data migration to change — the precedent D-02/D-17 cite),
  D-18 (`EngineLimits` excluded from the fingerprint).
- `.planning/phases/22-battlefield-state-superstep-engine/22-CONTEXT.md` — item 7 "seams only:
  `TraceSink` hook (bounded channel, drop-oldest, counted drops) … implement the hooks, not
  their consumers" — this phase is the consumer.

### Standing decisions and governance

- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — the single 82 % floor.
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` (ADR-0015) — what
  `paladin-ports` may import; `CompositeSink`, `TraceEmitter`, `RunTracePort` and
  `RunInspectorPort` add nothing to the allowlist (D-08, D-17, D-24).
- `.planning/decisions/0016-port-value-type-ownership.md` (ADR-0016) — core owns port value
  types, ports re-export (D-01, D-24).
- `.planning/decisions/0031-extracted-crate-dependency-rule.md` (ADR-0031) — `paladin-web`
  never depends on `paladin-battalion` in its default build (D-24); the invariant's scope
  ("extracted crates") is what D-27's ADR-0047 clarifies for `paladin-eval`.
- `.planning/decisions/0023-cli-dependency-isolation.md` (ADR-0023) — the CLI is `cli`-gated
  and in-process; `paladin-eval` joins as an optional dependency of that feature (D-23, D-33).
- `.planning/decisions/0037-agent-route-surface-v1.md` (ADR-0037) — `/v1` prefix,
  `openapi.json` is the drift-guard baseline (D-15, D-16, D-25).
- `.planning/decisions/0038-agent-provisioner-placement.md` (ADR-0038) — port parameter types
  are core types (D-24).
- `.planning/decisions/0012-live-api-test-key-behaviour.md` (ADR-0012) — how live-API keys are
  handled; D-35's `--live` mode inherits it.
- `.planning/decisions/0046-facade-llm-feature-flag-wiring.md` (ADR-0046) — the umbrella
  feature-forwarding convention D-12/D-25 follow for `otel` and `dev-ui`.
- `.github/instructions/security.instructions.md` — redact before truncate (D-05); no API key or
  header secret in a trace record, log line, `Debug` output or error body (D-36 `OtelConfig`);
  the manual credential-handling review is the primary control for the OTel exporter (it sends
  configured headers to a configured endpoint — the client must not follow redirects, the same
  house rule every LLM adapter and the webhook client obey).
- `CLAUDE.md` / `.github/copilot-instructions.md` — ubiquitous language (no new military term is
  introduced by this phase; "trace", "sink", "eval" are plain), TDD, no `unwrap()` in library
  code, doc tests on all public APIs.

### Existing implementation this phase extends

- `crates/paladin-ports/src/output/trace_sink_port.rs` — the Phase 22 `TraceSink` +
  eight-variant `TraceEvent` (moved and reshaped by D-01/D-02; module docs' "field NAMES, not
  VALUES" and "errors are diagnostics only" sections stay true).
- `crates/paladin-battalion/src/engine/hooks.rs` — `TraceDispatcher` (`DEFAULT_CAPACITY` 1024,
  `with_capacity`, `emit`, `dropped_count`; the consumer task D-08 hardens; the `seq` authority
  D-03 adds); `engine/test_support.rs:645-760` (`RecordingTraceSink`, `BlockingTraceSink`,
  `AlwaysErroringTraceSink`, `GatedTraceSink` — reuse, add `PanickingTraceSink`).
- `crates/paladin-battalion/src/engine/mod.rs` — `with_trace_sink` (1587), `RunOutcome`
  (272-312, the `RunFinished.status` source), `start`/`resume`; `engine/superstep.rs` (edge
  resolution, the `EdgeEvaluated` site); `engine/graph.rs:527` `WarGraph` (`from_graph`
  source); `engine/graph_doc.rs:124` `WarGraphDoc` + `NodeKindDoc` (`from_doc` source);
  `engine/registries.rs` `EngineRegistries` (D-31).
- `crates/paladin-llm/src/fallback.rs:117-160` — `FallbackLlmAdapter.trace_sink` /
  `with_trace_sink` (D-03 → `with_trace_emitter`); `crates/paladin-llm/src/redaction.rs` (D-05).
- `src/application/services/run/events.rs` — `RunEventBus`, `RUN_EVENT_CHANNEL_CAPACITY` (64),
  `map_trace_event` (87), `RunEventBusSink` (267-295), `RunEventStreamService` (500-560) and the
  degraded `DegradedState` unfold (D-14…D-16); `src/application/services/run/worker.rs:856-903,
  1067` — the publish sites D-14 removes; `src/application/services/run/stream_tests.rs`.
- `crates/paladin-core/src/platform/container/run.rs:468-570` — `RunStreamEventKind`,
  `RunStreamMode`, `RunStreamEvent` (D-15, D-16); `waypoint.rs:562-585` `NodeExecutionRecord`
  (`outcome`, `duration_ms`, `token_count`, `attempt`), `Waypoint` (655-677: `vanguard`,
  `completed`, `status`, `graph_fingerprint`) — the D-21 overlay source;
  `heartbeat.rs:63` `HeartbeatHandle`.
- `crates/paladin-ports/src/output/waypoint_port.rs:212` `history(thread, limit, before)`;
  `crates/paladin-storage/src/waypoint/{mod,in_memory,sqlite,postgres,contract_tests,retention}.rs`
  and `crates/paladin-storage/migrations/{sqlite,postgres}/` — the layout D-17 mirrors;
  `src/application/services/waypoint_retention.rs` — the routine D-17 extends.
- `crates/paladin-ports/src/input/run_event_stream_port.rs` — the input-port pattern D-24 copies.
- `crates/paladin-web/src/{app.rs,agent_auth.rs,error.rs,thread_controller.rs,openapi.rs}` —
  router/auth/envelope conventions for the `dev-ui` route (D-25); `crates/paladin-web/openapi.json`
  + the `UPDATE_OPENAPI=1` bless (`Makefile:370`) — D-15/D-16/D-20.
- `crates/paladin-battalion/src/maneuver/visualizer.rs` — the existing Mermaid renderer for the
  Maneuver DSL (untouched; a style reference only).
- `src/application/cli/{commands/mod.rs,commands/muster.rs,error.rs,formatters/}`,
  `src/bin/paladin-cli.rs`, `tests/cli/*.rs` (insta snapshots) — the CLI conventions D-23/D-33
  follow.
- `crates/paladin-llm/src/mock.rs` — `MockLlmAdapter`, `MockScriptEntry` (untouched, D-30;
  `ScenarioLlm` may delegate to it internally).
- `tests/integration/e2e_crash_resume_test.rs`, `e2e_approval_gate_test.rs`,
  `e2e_compensation_chain_test.rs` (+ `e2e_muster_defer_order_test.rs` for the muster shape) —
  the fixture builders D-34 shares; `tests/helpers/` — where they move.
- `crates/doc-examples/Cargo.toml` — the composition-crate precedent D-27 cites.
- `src/config/{run_stream.rs,waypoint_retention.rs,web_server.rs,mod.rs}` — the config-struct
  pattern D-36 mirrors; `config.example.yml`, `config.test.yml`.
- `benches/engine_benchmarks.rs:243` `bench_superstep_cost` (D-37);
  `src/application/services/orchestration/listener.rs` — the timeout-guard house pattern.
- `docs/src/SUMMARY.md`, `docs/schemas/wargraph-doc.schema.json` (D-28's golden idiom).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`TraceDispatcher` + its four test doubles** — the bounded, drop-oldest, counted, never-awaited
  handoff PRD 07 OBS-FR-02 asks for already exists and is tested; this phase adds stamping,
  `catch_unwind` and a stress test to it rather than writing a channel.
- **`RunEventBusSink` / `map_trace_event`** — already a `TraceSink`; the collapse is deleting the
  worker's direct publishes and completing one match.
- **`NodeExecutionRecord` / `Waypoint.completed` / `vanguard`** — carry outcome, duration, tokens,
  attempt and the vanguard per superstep, enough to build the overlay without persisted traces.
- **`WaypointRetentionService` + `paladin-storage::waypoint::retention`** — the routine the
  `run_traces` prune joins.
- **`MockLlmAdapter` with `with_script`/`requests()`** — sequence scripting and request capture
  `ScenarioLlm` can delegate to.
- **`schemars` derive + golden idiom (`graph_doc_round_trip.rs`), `UPDATE_OPENAPI=1`, `insta`** —
  every golden/snapshot mechanism this phase needs already has a house form.
- **Hermetic axum `FixtureServer` (Phase 3) and `oneshot` web tests** — the OTLP stub and the
  inspector smoke test reuse them.
- **`FlowVisualizer::to_mermaid`** — a style reference for Mermaid escaping/ids (not reused).
- **`benches/engine_benchmarks.rs::build_width_graph` + `bench_superstep_cost`** — the bench
  harness D-37 extends.

### Established Patterns

- **Ports see only core (ADR-0015/0016); `paladin-web` sees only ports/core (ADR-0031)** — every
  new capability the web layer needs arrives as a core-typed input port the facade implements
  (`RunEventStreamPort` precedent → `RunInspectorPort`).
- **Feature forwarding** — umbrella feature → leaf-crate feature, optional deps, absent from
  `default`/`full` for heavy adapters (`storage-postgres`, `redis-cache` comments in `Cargo.toml`).
- **Config structs** — `Default` + `validate()` + `EnvOverridable`, a `#[serde(default)]`
  section that deserializes when absent, `config.example.yml` mirrors every field.
- **Storage adapter trio + contract suite** — `in_memory`/`sqlite`/`postgres` + shared
  `contract_tests.rs`, migrations numbered per backend, Postgres proven only in CI.
- **Deliberate-zero `MIGRATION.md` notes** for new-in-0.10 types; explicit rows for pre-existing
  ones; `cargo semver-checks` + allowlist set-equality.
- **`log` crate everywhere** — no `tracing` consumer in the tree.
- **CLI is in-process, `cli`-gated, snapshot-tested with `insta`**.
- **Simulated process drop** in the E2E tests (drop `Arc`s, fresh engine over the same SQLite
  file) — the runner's `interrupt_after_superstep` reuses the technique.

### Integration Points

- `WarEngine::with_trace_sink` (unchanged) + new `WarEngine::trace_emitter()`; the Phase 27 run
  worker's per-run engine construction is where the composite (log ⊕ bus ⊕ persist ⊕ otel) is
  assembled and the emitter handed to the fallback adapter and middleware.
- `engine/superstep.rs` edge resolution → `EdgeEvaluated`; `engine/mod.rs` `RunOutcome`
  construction → `ParleyRaised`/`RunFinished`; `HeartbeatHandle` receiver → `NodeProgress`.
- `src/application/services/paladin/{paladin_execution_service.rs,middleware/}` →
  `MiddlewareEvent`, `NodeProgress::{StreamChunk,ToolCall}`.
- `RunEventStreamService::stream` → live / replay / degraded branch (D-16).
- `paladin-web/src/app.rs` `/v1` router → `dev_ui_controller` under `require_admin` (feature).
- `src/bin/paladin-cli.rs` `Commands` enum → `Graph`, `Run`, `Eval` subcommand groups.
- Root `Cargo.toml` — `[features] otel`, `dev-ui`; `cli` gains `dep:paladin-eval`;
  `[dev-dependencies] paladin-eval`; `[[test]] name = "evals" harness = false`.
- CI: feature-flags job matrix gains `otel`, `dev-ui`; `postgres-integration` gains the
  `run_traces` contract suite; release/`semver`/`api-surface`/`sdk` crate lists gain
  `paladin-eval`.

</code_context>

<specifics>
## Specific Ideas

- The acceptance question for the inspector is literal: on the branching fixture run a reader
  must see, without any other tool, **which** of two conditional edges fired at the gate
  (with the condition kind and the value that satisfied it, when the trace source is present)
  and **why** a loop node ran three times (three visits, the supersteps, the edge that
  re-entered it each time, and the outcome of each attempt).
- `--repeat 20` on every scripted scenario in `evals/` is 20/20 — a divergence is a bug in the
  engine's determinism, and the CLI output must make the divergent `seq` visible, not just "1
  of 20 failed".
- A failing eval assertion reads like a test failure: expected, observed, the `seq` window, and
  for `node_executed` the visit table — the `insta` snapshot of that message is part of the
  contract.
- The log sink line is grep-able by `thread_id`, `seq` and `kind` because they are the first
  three keys of the flat JSON object (serde field order is declaration order).
- One dependency graph for a default `cargo build`: `cargo tree -e features` before/after the
  phase differs only by `paladin-eval` as a **dev**-dependency.

</specifics>

<deferred>
## Deferred Ideas

- **LLM-call child spans under the attempt span** — PRD 07 OBS-FR-05 "MAY … not required";
  needs adapter-side instrumentation through the emitter (a Phase 29+ or FUT item).
- **`FallbackHop.node_id` enrichment** (25-CONTEXT D-25 "Phase 28 may enrich") — the emitter
  change (D-03) makes it possible; wiring the node id from `NodeContext` into the adapter is
  left for when a consumer needs it. Claude's discretion may pick it up if `NodeContext` gains
  the trace accessor.
- **Live SSE attach in the inspector page** — PRD 07 OBS-FR-10 "nice-to-have if SSE trivially
  attaches"; the page is static-first (D-26).
- **A DOT execution overlay** — PRD names Mermaid for the overlay.
- **A Function-node / worker-template registry for `WarGraphDoc`** — 27-CONTEXT deferral,
  unchanged; D-18's `GraphShape::from_graph` sidesteps the need for export.
- **LLM-as-judge eval scoring** — PRD 07 §5; `custom(fn)` leaves the door open.
- **A `tracing`-crate migration** of the logging stack — the unused `tracing-subscriber` line is
  a pre-existing oddity, not this phase's.
- **`trace_state_values` on the SSE wire** — never (T-27-10-01); a consumer wanting values reads
  Waypoints.
- **Per-`run_traces` retention tunables** — D-17 shares the Waypoint policy by design.
- **Vendoring Mermaid** — D-26's rejected alternative; revisit only if a CDN-free build is
  demanded.
- **`27-SECURITY.md`** (`/gsd-secure-phase 27`), WR-27-01 (`Ok(None)` signing-key arm),
  IN-27-01, WINDOWS.md rows 31/32 — Phase 27's open items, not this phase's.
- **`MIGRATION.md` §9 finalisation, the v0.9-config boot test, openapi golden diff, program
  acceptance audit, v0.10.0 version bump** — SHIP-01…04 (Phase 29). This phase fills its rows
  (D-39); Phase 29 proves they are complete.
- **22-REVIEW.md WR-01/WR-02, 22-deferred-items.md item 1 (`qdrant` `--all-features` rustdoc
  break), 24/25/26-REVIEW.md advisory warnings, the ~60 pre-existing rustdoc warnings** —
  unchanged, not this phase's.

### Reviewed Todos (not folded)

- "Verify local make coverage reproduces CI's 82.39% figure"
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`) — `todo.match-phase`
  returned no match for Phase 28 (score below the 0.4 `--auto` fold threshold); a maintainer's
  local-tooling check, unrelated to observability. Left in the todo list, as in Phases 24-27.

</deferred>

---

*Phase: 28-observability-tooling*
*Context gathered: 2026-09-08 (`--auto`)*
