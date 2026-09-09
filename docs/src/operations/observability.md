# Observability: Traces, Sinks and Persistence

**Since:** v0.10.0 (Phase 28, PRD 07)

Every `WarEngine` run emits a stream of `TraceRecord`s describing what happened, superstep by
superstep, node attempt by node attempt. This page covers the trace model, where those records
go, how to wire OTLP export, how they are persisted and pruned, and how to correlate one trace
record with the same event's log line, span and stored row.

## The trace model

### The envelope

A `TraceRecord` is the one-flat-object envelope every consumer reads:

```json
{"thread_id":"01a0...","run_id":"01a0...","seq":7,"at":"2026-09-09T00:00:00Z","kind":"node_started","superstep":2,"node_id":"writer","attempt":1,"muster_task_key":null}
```

`thread_id` identifies the run; `run_id` is present when the Platform API wraps the run and
`None` for a bare embedded engine; `seq` is a per-run, 1-based, strictly increasing sequence
number stamped by the run's own `TraceDispatcher` at enqueue time (so `seq` order *is* causal
order); `at` is when the event happened, not when a sink observed it. `#[serde(flatten)]` over a
`#[serde(tag = "kind", rename_all = "snake_case")]` enum keeps the whole record one flat JSON
object — `OBS-FR-04`'s "one line per event."

### The twelve variants

`TraceEvent` (`paladin-core::platform::container::trace`) is `#[non_exhaustive]`, twelve
variants:

| Variant | Fires when | Notable fields |
|---|---|---|
| `RunStarted` | `WarEngine::start`/`resume` begins | `run_id?`, `graph_fingerprint` |
| `SuperstepStarted` | a superstep begins | `superstep`, `vanguard` |
| `NodeStarted` | one node attempt begins | `superstep`, `node_id`, `attempt`, `muster_task_key?` |
| `NodeProgress` | a liveness/progress update from a running node | `node_id`, `progress: Heartbeat \| StreamChunk{bytes} \| ToolCall{tool}` |
| `NodeFinished` | one node attempt ends | `outcome`, `duration_ms`, `token_count`, `cache_hit` |
| `EdgeEvaluated` | an outgoing edge is checked, whether or not it fires | `from`, `to`, `condition_kind`, `fired` |
| `DeltaMerged` | a superstep's `StateDelta` merges into the `Battlefield` | `field_changes: Vec<FieldChange>` |
| `WaypointSaved` | the superstep's Waypoint is persisted | `waypoint_id`, `superstep`, `status` |
| `ParleyRaised` | the engine builds an `AwaitingInput` outcome | `parley_id`, `node_id`, `parley_kind` |
| `RunFinished` | the run reaches a terminal outcome | `status`, `total_supersteps`, `total_tokens`, `duration_ms`, `trace_dropped_total` |
| `FallbackHop` | `FallbackLlmAdapter` switches providers mid-call | `node_id?`, `from_provider`, `to_provider` |
| `MiddlewareEvent` | a middleware chain member finishes/fails/denies/redacts/retries/falls back | `name`, `action` |

Every payload is bounded by construction: `DeltaMerged.field_changes` carries the changed field's
**name** and `value_bytes` (its serialized size), never the value itself, unless
`trace.state_values` is explicitly turned on — see [Drop accounting and payload
bounds](#values-are-opt-in-redacted-then-capped) below. `NodeProgress::StreamChunk` carries a
byte count, never streamed text.

### `seq` ordering and drops

Every producer reaches the trace stream through a `TraceEmitter` handle
(`WarEngine::trace_emitter()`), never a raw sink. `TraceDispatcher::emit` stamps `seq`/`at` at
enqueue time on a bounded, drop-oldest channel (`trace.channel_capacity`, default `1024`). If the
channel fills, the *oldest* queued record is dropped, the drop is counted, and the first drop of a
run logs one `warn!` line under target `paladin::trace`. `RunFinished` is never itself the dropped
event and always carries the run's final `trace_dropped_total`, so a consumer can reconcile
"observed `seq` gaps" against "the run's own drop count" without a second source of truth.

## The sinks

A run's sink fan-out is assembled once, in `src/infrastructure/telemetry/mod.rs::build_run_sink`
— the single composition point every new sink joins. Zero, one, or several of the following are
attached per run, based on configuration:

1. **Log sink** (`LogTraceSink`, default **on** via `trace.log_sink`) — one `log::info!` line per
   record, JSON-serialized, under target `paladin::trace`. Silence it with
   `RUST_LOG=paladin::trace=off` or `trace.log_sink: false`.
2. **OTel sink** (`OtelTraceSink`, `otel` Cargo feature, off by default) — see [OTLP
   export](#otlp-export) below.
3. **SSE bus sink** (`RunEventBusSink`) — the *only* producer onto a run's live SSE stream
   (`GET /v1/runs/{id}/stream`); `map_trace_event` maps seven of the twelve wire names onto the
   frozen SSE event vocabulary. See [Known limitations](#known-limitations) for what a live SSE
   event does and does not carry compared to the full trace record.
4. **Persisting sink** (`PersistingTraceSink`, `trace.persist`, off by default) — see
   [`run_traces` persistence](#run_traces-persistence-and-retention) below.

A construction failure in any sink (a malformed OTLP endpoint, for example) is diagnostics-only:
logged, that one sink is skipped for the run, and the run itself never fails.

## OTLP export

Behind the `otel` Cargo feature (absent from `default` and `full`), `OtelTraceSink` turns the
record stream alone into a span-per-attempt tree: one root `run` span per `thread_id`, opened on
`RunStarted` and closed on `RunFinished`; one child span per `(node_id, attempt)`, opened on
`NodeStarted` and closed on `NodeFinished` — **a retried node produces sibling spans, never nested
ones**. `EdgeEvaluated`/`DeltaMerged`/`MiddlewareEvent`/`ParleyRaised`/`FallbackHop` become span
*events* on the enclosing span. A record that would need a span this sink never saw opened (a
dropped `NodeStarted`) opens a synthetic span flagged `paladin.trace.partial = true` rather than
losing the node entirely.

Enable it:

```yaml
trace:
  otel:
    enabled: true
    endpoint: "http://localhost:4318/v1/traces"
    headers:
      Authorization: "Bearer <redacted>"
    service_name: "paladin"
```

```bash
cargo build --features otel
```

`OtelConfig.headers` values are secrets by assumption: the struct has a **manual** `Debug` impl
that redacts every value (never derived), and no log line interpolates them. The exporter's HTTP
client never follows a redirect, so a `3xx` from the configured endpoint can never carry a
configured header to a different, attacker-influenced host. Setting `otel.enabled: true` on a
build without the `otel` feature is a typed `TraceConfigError::FeatureNotCompiled`, never a silent
no-op.

Example collector configuration (OpenTelemetry Collector, OTLP/HTTP receiver):

```yaml
receivers:
  otlp:
    protocols:
      http:
        endpoint: 0.0.0.0:4318
exporters:
  logging:
    verbosity: detailed
service:
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [logging]
```

Only OTLP over HTTP/protobuf is supported — no gRPC/`tonic` transport, which would roughly double
the dependency graph for a transport nobody has asked for.

## `run_traces` persistence and retention

Set `trace.persist: true` to attach `PersistingTraceSink`, which buffers records and flushes them
as one batch through `RunTracePort::append` — on `WaypointSaved` (the superstep boundary), on
`RunFinished`, or whenever the buffer reaches 256 records. A crash loses at most the un-flushed
tail of one superstep; **the Waypoint remains the durability truth, the trace is best-effort.**

Records land in the `run_traces` table (migration `006`, both SQLite and Postgres backends),
append-only, `PRIMARY KEY (thread_id, seq)`. Reading a row whose `schema_version` this build does
not recognize is a typed `RunTraceError::UnsupportedSchemaVersion`, never a silent misparse.

Retention shares `ENG-FR-18`'s Waypoint policy: `WaypointRetentionService` prunes `run_traces`
rows with the *same* age/count bounds it applies to Waypoints — one config
(`WaypointRetentionConfig`), one routine, two ports. There is no `run_traces`-specific tunable. A
failing trace prune is logged and never aborts Waypoint pruning.

## Replay: upgrading a finished run's stream to full fidelity

When `trace.persist` is on and a requested run's `GET /v1/runs/{id}/stream` call finds persisted
rows, `RunStreamMode::Replay` streams them back through the same `map_trace_event` mapping, with
the original `at` and `trace_seq`, then terminates with `done`/`error` exactly as a live run does.
No rows (persistence off, or a run outside retention) falls back to today's degraded
Waypoint-polling path, unchanged.

## Drop accounting and payload bounds

`DeltaMerged.field_changes[]` carries `field`, `dispatch`, `writers` and `value_bytes` (the
changed value's serialized size) by default — never the value itself. Setting
`trace.state_values: true` (default `false`) additionally attaches a `value`, but only after it
passes through the existing secret-redaction helper and *then* gets truncated to
`trace.value_cap_bytes` (default 256 bytes) — redact **then** truncate, never the reverse, so a
secret cannot be sliced across the truncation boundary and leaked in the tail. A value opted into
this way still never reaches the SSE wire — `map_trace_event` strips it unconditionally — it can
only land in a log line, an OTel span attribute, or a persisted `run_traces` row.

## Correlating one event across logs, spans and storage

`trace_seq` is the thread that ties one trace record to the same event everywhere it appears: the
`seq` on the record itself, an additive field on every live/replayed SSE event's payload, an
attribute value on the corresponding OTel span/event, and a column on the persisted `run_traces`
row. Given a `trace_seq` from any one of those four surfaces, you can find the exact same record
in the other three.

## Known limitations

- **Superstep-cost overhead is measured, not a passed gate.** PRD 07 acceptance 6's bar is ≤3%
  superstep overhead versus an untraced run. Measured on this build (`28-BENCH-EVIDENCE.md`):
  `log_sink` +22.18%, `composite` (log + a no-op sink) +18.46% against a ~110µs untraced baseline
  — both genuinely **fail** the ≤3% bar. This is recorded here honestly rather than softened; see
  `.planning/phases/28-observability-tooling/28-BENCH-EVIDENCE.md` for the full measurement and
  analysis. It is not gated in CI (criterion numbers on shared runners are noise) — the record is
  the gate.
- **`trace.heartbeat_interval_secs` is not yet wired into the engine's own rate limiter.** The
  engine hardcodes a 5-second default heartbeat interval per node; threading the configured value
  through is a documented, deliberate scope reduction (28-06), not a bug.
- **`DeltaMerged.field_changes[].dispatch`/`.writers` are placeholder defaults** (empty
  string/empty vec) — `Battlefield::merge`'s own `MergeReport` today tracks only changed field
  *names*, not per-field dispatch rule or writer list. `field` and `value_bytes` are always real.
- **A live/replayed `parley`/`error` SSE event carries a reduced payload compared to the full
  trace record.** `TraceEvent::ParleyRaised` carries only `parley_id`/`node_id`/`kind` (no
  `prompt`/`choices`/`expires_at`); `TraceEvent::RunFinished` carries no `waypoint_id` or error
  message. The published wire's top-level field *names* are unchanged, but those specific fields
  are `null` on this path — a deliberate consequence of the trace model excluding free-form/PII-shaped
  content by design, not an oversight (`.planning/WINDOWS.md` id 33). A client wanting the full
  detail reads `GET /threads/{id}/state`, which is unaffected.
- **The `run_traces`-backed replay mode and the `trace_seq` field are not reflected in the
  committed OpenAPI schema.** `RunStreamMode`/`RunStreamEvent` are not reachable from the router
  sources `openapi.rs`'s drift-guard assembles — the SSE endpoint's wire-event shape is static
  prose in the route's description, not a `#[derive(ToSchema)]`-derived schema. Re-running the
  `UPDATE_OPENAPI=1` bless produces an empty diff; this is a known gap in the generated schema's
  coverage of the streaming endpoint, not a missed update.
