# Phase 28: Observability & Tooling - Pattern Map

**Mapped:** 2026-09-08
**Files analyzed:** ~40 new/modified files across 8 crates + facade + CLI + web + eval crate
**Analogs found:** 34 / 40 (remainder listed under "No Analog Found" — first-of-kind in this repo, use RESEARCH.md's Code Examples / external docs instead)

Note: `28-CONTEXT.md` and `28-RESEARCH.md` already carry exhaustive file:line citations for nearly
every analog (RESEARCH.md's "16 Open Questions" section in particular). This document distills
those into concrete copy-from excerpts; where CONTEXT/RESEARCH already gives the exact citation,
this file points there rather than re-quoting at length.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-core/src/platform/container/trace.rs` | model (core value types) | event-driven | `crates/paladin-ports/src/output/trace_sink_port.rs` (current `TraceEvent`, to be moved+reshaped) + `crates/paladin-core/src/platform/container/waypoint.rs` (`NodeExecutionRecord`) | exact (relocation + extension of existing type) |
| `crates/paladin-ports/src/output/trace_sink_port.rs` (extend: `TraceEmitter`, `CompositeSink`) | port (output) | event-driven | itself (current file, Phase 22 baseline) | exact |
| `crates/paladin-ports/src/output/run_trace_port.rs` | port (output) | CRUD (append/read/prune) | `crates/paladin-ports/src/output/waypoint_port.rs` | exact (near-identical shape: `Backend` error variant, `Ok(None)`-is-not-error convention) |
| `crates/paladin-ports/src/input/run_inspector_port.rs` | port (input) | request-response | `crates/paladin-ports/src/input/run_event_stream_port.rs` | exact (core-typed input port for `paladin-web`) |
| `crates/paladin-battalion/src/engine/hooks.rs` (extend `TraceDispatcher`) | service (dispatcher) | event-driven | itself (current file) | exact |
| `crates/paladin-battalion/src/engine/test_support.rs` (add `PanickingTraceSink`) | test double | event-driven | itself — `RecordingTraceSink`/`BlockingTraceSink`/`AlwaysErroringTraceSink`/`GatedTraceSink` (`engine/test_support.rs:645-760`) | exact |
| `crates/paladin-battalion/src/engine/export/{mod,shape,mermaid,dot,overlay}.rs` | service (pure transform) | transform | `crates/paladin-battalion/src/maneuver/visualizer.rs` (`FlowVisualizer::to_mermaid`/`to_ascii`) | role-match (style reference for Mermaid string-building/escaping, different domain) |
| `crates/paladin-battalion/tests/export_golden.rs` | test (golden) | batch | `crates/paladin-battalion/tests/graph_doc_round_trip.rs` (`UPDATE_WARGRAPH_SCHEMA=1` bless idiom, lines 104-122) | exact |
| `crates/paladin-storage/src/run_trace/{mod,in_memory,sqlite,postgres,contract_tests}.rs` | model+service (storage adapter trio) | CRUD | `crates/paladin-storage/src/waypoint/{mod,in_memory,sqlite,postgres,contract_tests}.rs` | exact |
| `crates/paladin-storage/migrations/{sqlite,postgres}/006_create_run_traces_table.sql` | migration | batch | `crates/paladin-storage/migrations/{sqlite,postgres}/005_create_webhook_deliveries_table.sql` | exact (numbering/header/index convention) |
| `src/infrastructure/telemetry/log_sink.rs` | service (adapter, `TraceSink` impl) | event-driven | `src/application/services/run/events.rs` (`RunEventBusSink`, a `TraceSink` impl, lines 267-295) | role-match (same trait impl shape, different transport: `log::info!` vs SSE bus) |
| `src/infrastructure/telemetry/otel_sink.rs` | service (adapter, `TraceSink` impl, feature-gated) | event-driven | `RunEventBusSink` (structure) + RESEARCH.md Code Examples §"OTLP HTTP exporter with no-redirect client" | role-match / new-pattern (first OTel consumer in tree; redirect-disabled reqwest client pattern is the webhook client's own house rule) |
| `src/infrastructure/telemetry/persisting_sink.rs` | service (adapter, `TraceSink` impl, buffered) | batch + event-driven | `RunEventBusSink` (trait shape) + `WaypointRetentionService` (buffering/flush-on-boundary concept) | role-match |
| `src/config/trace.rs` | config | request-response (static) | `src/config/run_stream.rs` | exact (`Default` + `validate()` + `EnvOverridable`, doc-test pattern) |
| `crates/paladin-battalion/src/engine/export/shape.rs`+overlay | model (pure) | transform | `crates/paladin-battalion/src/engine/graph.rs` (`NodeSpec`, `WarGraph::is_worker_template`/`is_deferred`) + `graph_doc.rs` (`NodeKindDoc`, `NodeDoc.defer`) | exact (source-of-truth types to read from) |
| `src/application/cli/commands/graph.rs`, `run.rs`, `eval.rs` | controller (CLI) | request-response | `src/application/cli/commands/muster.rs` | role-match (clap args → `CliError` → `OutputFormatter`; muster is LLM-generation, graph/run/eval are export/inspect, but the command skeleton, error handling and output formatter usage transfer directly) |
| `tests/cli/*.rs` (new snapshot tests) | test (snapshot) | request-response | existing `tests/cli/*.rs` (insta snapshots for other commands) | exact |
| `crates/paladin-web/src/dev_ui_controller.rs` | controller (web) | request-response | `crates/paladin-web/src/thread_controller.rs` | exact (router/state/auth-layer/error-envelope conventions) |
| `crates/paladin-web/src/dev_ui/inspector.html` | template (static asset) | request-response | none in-tree — new pattern (`include_str!`'d template); use RESEARCH.md's Mermaid CDN + JSON-embed pattern | no analog |
| `crates/paladin-web/tests/*` (dev-ui smoke test) | test (integration) | request-response | existing `crates/paladin-web/tests/*` `oneshot` harness tests | exact |
| `crates/paladin-eval/Cargo.toml` | config (crate manifest) | — | `crates/doc-examples/Cargo.toml` | exact (composition-crate manifest shape — six+ workspace-crate deps) |
| `crates/paladin-eval/src/lib.rs`, `scenario.rs`, `scripted_llm.rs`, `assertions.rs`, `runner.rs` | model/service (new crate) | transform + event-driven | `crates/paladin-llm/src/mock.rs` (`MockLlmAdapter`, `MockScriptEntry`, `LlmPort` impl) for `scripted_llm.rs`; `tests/integration/e2e_crash_resume_test.rs` for `runner.rs`'s engine+temp-store setup | role-match / exact |
| `tests/helpers/e2e_fixtures.rs` | test helper | transform | graph-builder functions inside `tests/integration/e2e_{crash_resume,approval_gate,compensation_chain}_test.rs` | exact (behavior-preserving extraction) |
| `benches/engine_benchmarks.rs` (extend) | test (bench) | batch | itself — `bench_superstep_cost` (line 243), `build_width_graph` (line 215) | exact |
| `src/application/services/run/events.rs` (extend `map_trace_event`) | service | event-driven | itself (current file, lines 87, 267-295, 500-560) | exact |
| `src/application/services/run/inspector.rs` | service (port impl) | request-response | `src/application/services/run/events.rs` (`RunEventStreamService`, degraded-mode `DegradedState` unfold reading Waypoints) | role-match |
| `src/application/services/waypoint_retention.rs` (extend) | service | batch | itself (current file, lines 67-98) — see Pitfall 3 below | exact |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/trace.rs` (model, event-driven)

**Analog:** `crates/paladin-ports/src/output/trace_sink_port.rs` (current `TraceEvent`, to move here) + `crates/paladin-core/src/platform/container/waypoint.rs::NodeExecutionRecord`

**Enum shape to copy** (`trace_sink_port.rs:65-152`, current 8-variant form — extend to 12 per CONTEXT D-02, keep the `#[non_exhaustive]` + doc-comment-per-variant style verbatim):
```rust
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TraceEvent {
    RunStarted { thread_id: ThreadId },
    SuperstepStarted { thread_id: ThreadId, superstep: u64 },
    NodeStarted { thread_id: ThreadId, superstep: u64, node_id: NodeId, attempt: u32 },
    NodeFinished { thread_id: ThreadId, superstep: u64, node_id: NodeId, attempt: u32, cache_hit: bool },
    // ... D-02's twelve variants; envelope now carries thread_id once (TraceRecord), not per-variant
}
```
Per D-02, remove per-variant `thread_id` and wrap in `TraceRecord { thread_id, run_id, seq, at, #[serde(flatten)] event: TraceEvent }` with `#[serde(tag = "kind", rename_all = "snake_case")]` on the enum — this is new shape, no direct in-tree precedent for the flatten+tag combo; base the serde attributes directly on D-02's prose, not on an analog.

**`#[non_exhaustive]` + wildcard-arm test discipline** (`trace_sink_port.rs:210-271`, `hooks.rs:281-292`): every `match` over `TraceEvent` anywhere in the codebase carries a wildcard arm; a construction-smoke test builds one of every variant (`all_seven_event_variants_construct` → becomes `all_twelve_...`).

---

### `crates/paladin-ports/src/output/trace_sink_port.rs` (extend: `TraceEmitter`, `CompositeSink`)

**Analog:** itself, current file (`trace_sink_port.rs:1-272`)

**Module-doc style to preserve** (lines 1-38): the "why a dedicated port, not a re-used one" / "carries field NAMES not VALUES" / "errors are diagnostics only" framing — write the same three-part rationale for `TraceEmitter` and `CompositeSink`.

**`TraceSink` trait shape to keep unchanged** (lines 176-186):
```rust
#[async_trait]
pub trait TraceSink: Send + Sync {
    async fn on_event(&self, event: TraceEvent) -> Result<(), TraceSinkError>;
}
```

**New `TraceEmitter` (sync, object-safe, per D-03)** — no direct in-tree trait to copy verbatim (it is deliberately NOT async, unlike `TraceSink`); model its doc structure on `TraceSink`'s but the signature is `fn emit(&self, event: TraceEvent);` with no `async_trait`. Object-safety test to copy (`trace_sink_port.rs:210-213`):
```rust
#[test]
fn trait_is_object_safe() {
    let _: Option<Box<dyn TraceSink>> = None;
}
```

**`TraceSinkError` — keep `#[non_exhaustive]` `thiserror` enum shape** (lines 51-57), same pattern for any new `RunTraceError`/`InspectorError`.

**`CompositeSink`** — new type; sequential fan-out with `catch_unwind` per child (D-08). No in-tree fan-out precedent to copy directly; write against `futures::FutureExt::catch_unwind(AssertUnwindSafe(..))` as named in CONTEXT D-08, applying the same "Errors are diagnostics only" doc framing from this file's own module docs.

---

### `crates/paladin-ports/src/output/run_trace_port.rs`

**Analog:** `crates/paladin-ports/src/output/waypoint_port.rs` (lines 1-60+)

**Module-doc "why separate" framing to copy** (`waypoint_port.rs:1-38`): the "Missing is `None`, not an error" and "`ThreadId` is not an authorization boundary" sections are directly reusable framing for `RunTracePort`'s own docs (its `read` should likewise return `Ok(vec![])`/`Ok(None)` for "nothing yet", never an error).

**Error enum shape** (`waypoint_port.rs` `WaypointError`, `#[non_exhaustive]` `thiserror::Error` with a `Backend { source }` variant) — copy directly for `RunTraceError`, adding `UnsupportedSchemaVersion { found }` per D-17/X-04.

**Trait signature to mirror the shape of** (per D-17, not from an existing method but same async_trait + Send+Sync convention as `WaypointPort`):
```rust
#[async_trait]
pub trait RunTracePort: Send + Sync {
    async fn append(&self, records: &[TraceRecord]) -> Result<(), RunTraceError>;
    async fn read(&self, thread: &ThreadId, after_seq: u64, limit: u32) -> Result<Vec<TraceRecord>, RunTraceError>;
    async fn prune_thread(&self, thread: &ThreadId, before_superstep: u64) -> Result<u64, RunTraceError>;
}
```

---

### `crates/paladin-ports/src/input/run_inspector_port.rs`

**Analog:** `crates/paladin-ports/src/input/run_event_stream_port.rs`

Copy the input-port pattern verbatim: a core-typed request/response port `paladin-web` calls without ever importing `paladin-battalion` (ADR-0031). `RunInspectorPort::inspect(&self, thread: &ThreadId) -> Result<InspectorView, InspectorError>` follows the same one-method, core-value-in/core-value-out shape as `RunEventStreamPort`. Read `run_event_stream_port.rs` directly for the exact trait-doc conventions (not excerpted here — CONTEXT.md D-24 cites this file as the pattern to copy).

---

### `crates/paladin-battalion/src/engine/hooks.rs` (extend `TraceDispatcher`)

**Analog:** itself, current file (`hooks.rs:1-420`)

**Fire-and-forget doc framing to extend, not replace** (lines 8-19): keep the "Why fire-and-forget" module section, add a subsection on panic isolation (D-08) using the same tone.

**`TraceQueue`/capacity/drop-oldest core to preserve exactly** (lines 41-90, 141-157) — `emit()`'s body (mutex lock, `pop_front` on overflow, `dropped.fetch_add`, doorbell `try_send`) stays as-is; the only change is where `seq`/`at`/`thread_id` get stamped (inside `emit`, before push, per D-03).

**Consumer task change for D-08 panic isolation** — wrap the existing `let _ = sink.on_event(event).await;` (line 116) in `catch_unwind`:
```rust
// current (hooks.rs:108-119):
match event {
    Some(event) => {
        let _ = sink.on_event(event).await;
    }
    None => break,
}
// D-08 target shape: wrap the await in catch_unwind(AssertUnwindSafe(..)),
// increment a sink_panics counter, log::error!, and keep looping (never break).
```

**Test-double reuse** (lines 277-420): `RecordingTraceSink`, `BlockingTraceSink`, `AlwaysErroringTraceSink`, `GatedTraceSink` (imported from `engine::test_support`) are the four doubles to add a `PanickingTraceSink` sibling to, following the exact same `Arc<Mutex<...>>`-backed pattern each of those already uses (see `engine/test_support.rs:645-760`).

**Ordering/overflow test style to copy for the new `seq`-gapless test** (lines 294-420): `full_queue_drops_the_oldest_event_not_the_newest` is the template for the D-06 ordering property test — gate a sink, overflow the queue by a known amount, assert `dropped_count()` and the surviving event order.

**X-05 stress-test pattern** — the module has no existing multi-thread stress test; base the new `#[tokio::test(flavor = "multi_thread")]` 16-concurrent-runs test on this file's own `tokio::test` style plus the `listener.rs` timeout-guard pattern named in D-37 (`src/application/services/orchestration/listener.rs`).

---

### `crates/paladin-battalion/src/engine/export/{shape,mermaid,dot,overlay}.rs`

**Analog:** `crates/paladin-battalion/src/maneuver/visualizer.rs` (style reference only, per D-19 — a different surface, stays untouched)

**Mermaid string-building style to mirror** (`visualizer.rs:1-50`, `to_mermaid`/`to_ascii`): doc-comment-driven public API with an `ignore`-tagged doctest showing sample output; build the diagram as owned `String` via `write!`/`push_str` into a buffer, never `HashMap`-iteration order (byte-determinism, D-19). Do **not** reuse `FlowVisualizer`'s `flowchart LR` — D-19 specifies `flowchart TD` for `GraphShape`, a different rendering convention for a different domain object.

**Source-of-truth types to read `GraphShape::from_graph`/`from_doc` from** (per RESEARCH.md Q6, verbatim):
```rust
// crates/paladin-battalion/src/engine/graph.rs:42-160
pub enum NodeSpec {           // #[non_exhaustive]
    Paladin { paladin, input_template, output_field, directive_parser, output_schema },
    Function(Arc<dyn StateNode>),
    Battalion { graph, state_map, restart_on_resume },  // = "Workflow"
    Gate { request, output_field },
}
// worker_template / deferred are NOT NodeSpec variants — separate HashSet<NodeId>
// fields on WarGraph, queried via:
graph.is_worker_template(id)   // graph.rs:660-668
graph.is_deferred(id)          // graph.rs:534-602

// crates/paladin-battalion/src/engine/graph_doc.rs:335-342
pub enum NodeKindDoc { Paladin(PaladinNodeDoc), Gate(GateNodeDoc), Workflow(WorkflowNodeDoc) }
// NodeDoc (graph_doc.rs:147-168) carries `pub defer: bool` directly.
```

---

### `crates/paladin-battalion/tests/export_golden.rs`

**Analog:** `crates/paladin-battalion/tests/graph_doc_round_trip.rs:104-122`

**Bless idiom to copy verbatim** (env-var name changes only):
```rust
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
For export goldens use `UPDATE_GOLDEN=1` per D-20, same three-line shape (compute → check env var → write-or-assert), golden files at `crates/paladin-battalion/tests/golden/export/`.

---

### `crates/paladin-storage/src/run_trace/{mod,in_memory,sqlite,postgres,contract_tests}.rs`

**Analog:** `crates/paladin-storage/src/waypoint/{mod,in_memory,sqlite,postgres,contract_tests}.rs`

**Module-doc framing to copy** (`waypoint/sqlite.rs:1-13`):
```
/*
SQLite <X> Store

Concrete `<X>Port` implementation over SQLite. `payload` is stored as TEXT
holding serialized JSON; every statement uses bound parameters (T-22-17); no
query string is ever built by formatting a caller-supplied value into it.
Migrations follow the versioned-file convention at
`crates/paladin-storage/migrations/sqlite/`, embedded at compile time via
`sqlx::migrate!` and applied automatically on construction.
*/
```

**`sqlx::migrate!` mechanism, verbatim** (`waypoint/sqlite.rs:95`, confirmed identical across all five existing adapters):
```rust
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");
// on adapter construction:
MIGRATOR.run(&pool).await?;
```
Same for `postgres.rs` with `"migrations/postgres"`.

**Bound-parameter query style** (`waypoint/sqlite.rs:14-52`): named `const ... : &str = r#"..."#;` SQL blocks with placeholder `?`, `json_extract` used for reading one field out of a JSON payload column without full deserialization — directly applicable to `run_trace`'s `record TEXT/JSONB` column if any summary-only read is ever needed.

**Contract-test-suite convention**: mirror `waypoint/contract_tests.rs`'s shared-fn-run-by-both-in_memory-and-sqlite-and-postgres pattern (file not excerpted here — read it directly at implementation time; same trait, three call sites).

**Retention integration — see Pitfall/Anti-Pattern below** (`src/application/services/waypoint_retention.rs:67-98`, `crates/paladin-storage/src/waypoint/retention.rs:104-109`).

---

### `crates/paladin-storage/migrations/{sqlite,postgres}/006_create_run_traces_table.sql`

**Analog:** `crates/paladin-storage/migrations/sqlite/005_create_webhook_deliveries_table.sql` (verbatim structure)

```sql
-- Migration: Create Run Traces Table (SQLite)
-- Purpose: <one line, per D-17>
-- Version: 006
-- Date: <today>

CREATE TABLE IF NOT EXISTS run_traces (
    thread_id       TEXT NOT NULL,
    seq             BIGINT NOT NULL,
    run_id          TEXT NULL,
    superstep       BIGINT NOT NULL,
    at              TEXT NOT NULL,
    schema_version  TEXT NOT NULL,
    record          TEXT NOT NULL,
    PRIMARY KEY (thread_id, seq)
);

-- Serves RunTracePort::read's (thread_id, seq > after_seq) filter/ordering.
CREATE INDEX IF NOT EXISTS idx_run_traces_thread_seq
ON run_traces(thread_id, seq);
```
(Postgres sibling: `TIMESTAMPTZ` for `at`, `JSONB` for `record`, same index names — follow `005_create_webhook_deliveries_table.sql`'s postgres twin for the type-mapping convention.)

---

### `src/infrastructure/telemetry/log_sink.rs`

**Analog:** `src/application/services/run/events.rs::RunEventBusSink` (lines 267-295) — same `TraceSink` impl shape, different transport.

Copy the `impl TraceSink for X { async fn on_event(&self, event: TraceEvent) -> Result<(), TraceSinkError> { ... Ok(()) } }` structure directly; replace the SSE-bus-publish body with:
```rust
log::info!(target: "paladin::trace", "{}", serde_json::to_string(&record).unwrap_or_default());
Ok(())
```
per D-11 — `record` here is the `TraceRecord` envelope (once `on_event` is retargeted to receive records via the `CompositeSink`/dispatcher plumbing), keyed so `thread_id`/`seq`/`kind` are the first three JSON keys (D-05 acceptance note: "grep-able ... because they are the first three keys of the flat JSON object").

---

### `src/infrastructure/telemetry/otel_sink.rs`

**Analog:** structure from `RunEventBusSink`; wire pattern from RESEARCH.md's own Code Example (verbatim, already vetted against docs.rs):
```rust
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
This mirrors the webhook client's and every LLM adapter's own no-redirect house rule (`.github/instructions/security.instructions.md`).

**In-memory span-tree test pattern** — `opentelemetry_sdk::testing::trace::InMemorySpanExporter` (`testing` feature, dev-only) per RESEARCH.md Q1; no in-tree precedent, first OTel consumer.

---

### `src/infrastructure/telemetry/persisting_sink.rs`

**Analog:** `RunEventBusSink` (trait shape) + `WaypointRetentionService`'s buffering concept (no direct buffered-sink precedent in tree — new pattern per D-17: buffer records, flush on `WaypointSaved`/`RunFinished`/256-record threshold).

---

### `src/config/trace.rs`

**Analog:** `src/config/run_stream.rs` (full file, 80+ lines read)

**Copy the whole shape verbatim** — struct + manual `Default` + `validate()` + `EnvOverridable`, doc-tested:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStreamConfig {
    pub poll_interval_ms: u64,
}

impl Default for RunStreamConfig {
    fn default() -> Self {
        Self { poll_interval_ms: 1000 }
    }
}

impl RunStreamConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.poll_interval_ms == 0 {
            return Err("run stream poll_interval_ms must be greater than 0".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for RunStreamConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<u64>("APP_RUN_STREAM_POLL_INTERVAL_MS") {
            self.poll_interval_ms = v;
        }
    }
}
```
`TraceConfig` follows this pattern exactly, with nested `OtelConfig` (own `Default`/manual `Debug` that redacts `headers`, per D-36 — no direct manual-`Debug`-redaction precedent found in a config struct; base it on `.github/instructions/security.instructions.md`'s credential rules plus the `HeartbeatHandle` manual-impl style below). Doc-test convention (module doc, `# Examples` with a runnable ` ``` ` block) — copy exactly from `run_stream.rs`'s own doc comments (not excerpted further here; read the file directly, it is short).

**Env-var prefix convention**: `run_stream.rs` uses `APP_RUN_STREAM_*`; note CONTEXT D-36 specifies `PALADIN_TRACE_*` for this phase — different prefix family, confirm against `src/config/waypoint_retention.rs`'s own prefix (`EnvOverridable` impl) before finalizing, since the codebase appears to have more than one active prefix convention.

---

### `HeartbeatHandle`'s manual-`PartialEq`/`Debug` pattern (shared pattern for `NodeContext.trace` discretion item)

**Analog:** `crates/paladin-core/src/platform/container/heartbeat.rs:106-122` (verbatim, per RESEARCH.md):
```rust
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
Apply identically to any newtype wrapping `Arc<dyn TraceEmitter>` if `NodeContext` gains a trace accessor field (Claude's Discretion item) — `NodeContext` derives `Debug`/`Clone`/`PartialEq` and a raw trait-object field will not compile against that derive (see Pitfall 4 in RESEARCH.md).

---

### `src/application/cli/commands/{graph,run,eval}.rs`

**Analog:** `src/application/cli/commands/muster.rs` (full file read, lines 1-90+)

**Command function signature + error/formatter conventions to copy**:
```rust
use crate::application::cli::error::CliError;
use crate::application::cli::formatters::output::OutputFormatter;

pub async fn run_graph_export(
    format: String,
    file: Option<String>,
    assistant: Option<String>,
    out: Option<String>,
) -> Result<(), CliError> {
    let formatter = OutputFormatter::new();
    // ... resolve input, call the pure export function, write to stdout or `out`
    Ok(())
}
```
Enum-with-`as_str`/`parse` pattern (`muster.rs:14-49`, `BattalionPattern`) is the template for any small closed CLI-facing enum (e.g. an export-format enum), though `clap::ValueEnum` derive may be more idiomatic for new code — check other recent CLI commands for whichever convention is more current before copying `muster.rs`'s hand-rolled `parse`.

**Insta snapshot test convention**: existing `tests/cli/*.rs` files (not excerpted; read one directly, e.g. whichever covers `muster` or another recent command) for the harness invocation and `insta::assert_snapshot!` usage pattern.

---

### `crates/paladin-web/src/dev_ui_controller.rs`

**Analog:** `crates/paladin-web/src/thread_controller.rs` (per CONTEXT D-25's own citation — router/state/auth-layer/error-envelope conventions; also `app.rs:53-57`'s `require_auth`+`require_admin` layering and `error.rs`'s `{ error: { code, message, details } }` envelope). Read these two files directly at implementation time (not excerpted here to control length) — CONTEXT.md D-25 already gives the exact route shape (`GET /v1/dev-ui/threads/{id}`, no `#[utoipa::path]`, `text/html` response, 404/501 error codes matching the existing `NotWired` precedent).

---

### `crates/doc-examples/Cargo.toml` → `crates/paladin-eval/Cargo.toml`

**Analog:** `crates/doc-examples/Cargo.toml` (full file read above) — composition-crate manifest shape, six+ workspace-crate dependencies via `path = "../X"`. **Caveat** (RESEARCH.md correction #2): `doc-examples`'s package is `publish = false` — it is precedent for the *dependency shape* only, not for D-27's *publishing* decision. `paladin-eval`'s manifest must set `publish = true` and register in the version-with-workspace / semver / sdk / api-surface crate lists per RESEARCH.md Q16, unlike its analog.

```toml
[package]
name = "paladin-eval"
version = "0.10.0"  # tracks workspace version, publish = true
edition = "2024"
publish = true
description = "Scenario-based regression testing harness for Paladin agent graphs"

[dependencies]
paladin-core = { package = "paladin-ai-core", version = "0.10.0", path = "../paladin-core" }
paladin-ports = { version = "0.10.0", path = "../paladin-ports" }
paladin-battalion = { version = "0.10.0", path = "../paladin-battalion" }
paladin-llm = { version = "0.10.0", path = "../paladin-llm", features = ["mock"] }
paladin-storage = { version = "0.10.0", path = "../paladin-storage", features = ["sqlite"] }
serde = { workspace = true }
serde_yaml = { workspace = true }
serde_json = { workspace = true }
regex = { workspace = true }
glob = "0.3.4"
libtest-mimic = "0.8.2"
```

---

### `crates/paladin-eval/src/scripted_llm.rs`

**Analog:** `crates/paladin-llm/src/mock.rs` (`MockLlmAdapter`, `MockScriptEntry`, both confirmed pre-existing/new-since-v0.9.0 per RESEARCH.md Q2 — read directly for the exact `LlmPort` impl and sequence-consumption logic). Per D-30, `ScenarioLlm` may delegate to `MockLlmAdapter` internally but is a distinct type in `paladin-eval` — `mock.rs` itself stays untouched.

---

### `crates/paladin-eval/src/runner.rs` + `tests/evals.rs`

**Analog:** `tests/integration/e2e_crash_resume_test.rs` (engine + SQLite temp-store setup, the "simulated crash" technique: drop `Arc`s, build a fresh engine over the same temp SQLite file, resume) — read directly; D-34 says this exact technique is what `interrupt_after_superstep` reuses.

**No in-tree `harness = false` `[[test]]` precedent exists** (RESEARCH.md Q4, confirmed by grep — only `[[bench]]` targets use `harness = false`). This is genuinely first-of-kind; base the `Cargo.toml` `[[test]]` stanza and `libtest-mimic::Arguments::from_args()`/`Trial` usage directly on `libtest-mimic`'s own crate docs, not an in-tree analog.

---

### `tests/helpers/e2e_fixtures.rs`

**Analog:** the graph-builder functions currently inside `tests/integration/e2e_{crash_resume,approval_gate,compensation_chain}_test.rs` — behavior-preserving extraction (D-34); read each file's builder function directly, move verbatim, re-export, update both the integration tests' and `paladin-eval`'s import paths.

---

### `benches/engine_benchmarks.rs` (extend)

**Analog:** itself — `bench_superstep_cost` (line 243), `build_width_graph` (line 215), `criterion_group!`/`criterion_main!` (lines 275-276). Add new `Criterion::bench_function` calls parametrized by sink variant (`none`/`log_sink`/`composite`) reusing `build_width_graph` unchanged; do not write a new fixture builder.

## Shared Patterns

### `#[non_exhaustive]` + wildcard-match discipline
**Source:** `crates/paladin-ports/src/output/trace_sink_port.rs:52,66` (`TraceSinkError`, `TraceEvent`)
**Apply to:** `TraceEvent`, `TraceRecord`-adjacent enums, `RunTraceError`, `InspectorError`, `RunStreamMode` (existing — must gain the marker per Pitfall 1), any new closed enum this phase introduces (`NodeProgressKind`, `MiddlewareEvent.action`, assertion tags).

### `thiserror` structured error convention
**Source:** `crates/paladin-ports/src/output/waypoint_port.rs` (`WaypointError::Backend { source }`)
**Apply to:** every new port error type (`RunTraceError`, `InspectorError`) — never a stringly-typed variant (X-06).

### Config struct: `Default` + `validate()` + `EnvOverridable` + doc-tested
**Source:** `src/config/run_stream.rs` (full file)
**Apply to:** `src/config/trace.rs::TraceConfig` and its nested `OtelConfig`.

### `sqlx::migrate!` embedded-migration mechanism
**Source:** `crates/paladin-storage/src/waypoint/sqlite.rs:95` (and every other adapter's `sqlite.rs`/`postgres.rs`)
**Apply to:** `crates/paladin-storage/src/run_trace/{sqlite,postgres}.rs`.

### `TraceSink` fire-and-forget / diagnostics-only error contract
**Source:** `crates/paladin-ports/src/output/trace_sink_port.rs:32-38`, `crates/paladin-battalion/src/engine/hooks.rs:8-19`
**Apply to:** `log_sink.rs`, `otel_sink.rs`, `persisting_sink.rs`, `CompositeSink` — a sink's own `Result` is never inspected by anything deciding a run's outcome.

### No-redirect HTTP client for a header-carrying outbound request
**Source:** `.github/instructions/security.instructions.md`'s webhook-client rule (pattern already applied at `src/application/services/run/webhook/` per the security instructions doc)
**Apply to:** `otel_sink.rs`'s OTLP HTTP exporter (`reqwest::Client::builder().redirect(Policy::none())`).

### Golden/bless-file idiom (`UPDATE_X=1` env var)
**Source:** `crates/paladin-battalion/tests/graph_doc_round_trip.rs:104-122` (`UPDATE_WARGRAPH_SCHEMA=1`), `Makefile:370` (`UPDATE_OPENAPI=1`)
**Apply to:** `crates/paladin-battalion/tests/export_golden.rs` (`UPDATE_GOLDEN=1`), any eval-scenario JSON Schema golden (`UPDATE_EVAL_SCHEMA=1`).

### CLI command skeleton: `CliError` + `OutputFormatter`
**Source:** `src/application/cli/commands/muster.rs`
**Apply to:** `graph.rs`, `run.rs`, `eval.rs`.

### Manual `PartialEq`/`Debug` for a non-comparable handle field
**Source:** `crates/paladin-core/src/platform/container/heartbeat.rs:106-122` (`HeartbeatHandle`)
**Apply to:** any newtype wrapping `Arc<dyn TraceEmitter>` embedded in a struct that derives `PartialEq`/`Debug` (e.g. a possible `NodeContext.trace` field).

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/paladin-web/src/dev_ui/inspector.html` | template | request-response | First static HTML/JS template in the tree (`include_str!`'d); use RESEARCH.md's Mermaid CDN example and D-26's JSON-embed spec directly |
| `crates/paladin-eval/` crate skeleton (`lib.rs`, `Cargo.toml` `publish=true` posture) | new crate | — | First *published* composition crate; `doc-examples` is dependency-shape precedent only (`publish=false`), not a publishing precedent — see RESEARCH.md correction #2 |
| `tests/evals.rs` `[[test]] harness = false` target + `libtest-mimic` `Trial`-per-case runner | test harness | event-driven | No existing `harness = false` `[[test]]` target anywhere in the workspace (only `[[bench]]` uses it) — confirmed by grep in RESEARCH.md Q4 |
| `src/infrastructure/telemetry/otel_sink.rs`'s OTLP wiring itself | service | event-driven | First OpenTelemetry consumer in the tree; no in-tree adapter precedent, only the docs.rs-sourced pattern in RESEARCH.md |
| `crates/paladin-web/Cargo.toml` `[features] dev-ui = []` | config | — | `paladin-web` currently has **no** `[features]` section at all (RESEARCH.md Q15, Pitfall 2) — this is the crate's first feature flag, not an extension of an existing list |
| `TraceRecord`'s `#[serde(flatten)] event: TraceEvent` + `#[serde(tag = "kind")]` combo | model | transform | New serde shape; no existing type in the tree combines flatten + internally-tagged enum this way — build directly from D-02's spec |

## Metadata

**Analog search scope:** `crates/paladin-{core,ports,battalion,storage,llm,web}/src`, `src/{config,application,infrastructure}`, `tests/`, `benches/`, `crates/doc-examples/`
**Files scanned/read directly:** `trace_sink_port.rs`, `hooks.rs` (full), `waypoint_port.rs`, `waypoint/sqlite.rs`, migration `005_create_webhook_deliveries_table.sql`, `run_stream.rs` (full), `muster.rs`, `doc-examples/Cargo.toml` (full), `maneuver/visualizer.rs`, plus CONTEXT.md/RESEARCH.md's own exhaustive file:line citation set (16 verified research questions covering every remaining analog)
**Pattern extraction date:** 2026-09-08
