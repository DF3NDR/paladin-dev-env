//! `OtelTraceSink` -- the `otel`-gated OTLP-over-HTTP trace exporter
//! (OBS-02, D-12, D-13).
//!
//! Turns the record stream alone (no engine internals) into a span-per-attempt
//! tree: one `run` root span per `thread_id`, opened on [`TraceEvent::RunStarted`]
//! and closed on [`TraceEvent::RunFinished`], and one child span per
//! `(node_id, attempt)`, opened on [`TraceEvent::NodeStarted`] and closed on
//! [`TraceEvent::NodeFinished`] -- so a retried node produces SIBLING attempt
//! spans, not nested ones (D-12). [`TraceEvent::EdgeEvaluated`] and
//! [`TraceEvent::DeltaMerged`] carry no `node_id` and always become span events
//! on the run span; [`TraceEvent::ParleyRaised`] always carries a `node_id` and
//! becomes an event on that node's currently-open attempt span;
//! [`TraceEvent::FallbackHop`] becomes an attempt-span event when its
//! `node_id` is `Some`, a run-span event otherwise; [`TraceEvent::MiddlewareEvent`]
//! carries no `node_id` and always becomes a run-span event. A record that
//! would need a span this sink never saw opened (a dropped `NodeStarted` or
//! `RunStarted`) opens a synthetic span flagged `paladin.trace.partial = true`
//! instead of being discarded -- a dropped event degrades the trace, never
//! loses the node (D-12). [`TraceEvent::SuperstepStarted`],
//! [`TraceEvent::NodeProgress`] and [`TraceEvent::WaypointSaved`] are outside
//! this span model's scope (not part of D-12's tree) and are no-ops here.
//!
//! # Runtime requirement: a multi-threaded Tokio runtime
//!
//! This sink exports synchronously on `Span::end_with_timestamp` via
//! `opentelemetry_sdk`'s `SimpleSpanProcessor` (its own docs: "ensure this
//! processor is only used from a thread where \[async HTTP clients such as
//! `reqwest-client`\] can run"). Empirically verified (28-09): driving this
//! exact combination -- `SimpleSpanProcessor` + an async `reqwest` client --
//! from a `#[tokio::test]`'s DEFAULT `current_thread` runtime deadlocks the
//! whole test process (`futures_executor::block_on` hijacks the only worker
//! thread, so nothing is left to drive the socket's own I/O readiness).
//! `#[tokio::test(flavor = "multi_thread")]` (this file's own tests) and
//! `#[tokio::main]` (this crate's production binaries -- `multi_thread` is
//! its default, unlike `#[tokio::test]`) both work correctly. Production is
//! therefore safe by construction; only a hand-written `current_thread`
//! Tokio context could reproduce the hang.
//!
//! # Security (T-28-09-01/02, `.github/instructions/security.instructions.md`)
//!
//! The OTLP HTTP client is built with `redirect(Policy::none())` (this
//! file's `Policy::none()` -- see [`build_reqwest_client`]) so a `3xx` from
//! the configured collector can never carry `OtelConfig.headers`'
//! credential-shaped values to a different, attacker-influenced host. No
//! header value is ever logged, `Debug`-printed or otherwise interpolated
//! anywhere in this file.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use opentelemetry::trace::{Span as _, TraceContextExt, Tracer as _, TracerProvider as _};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::{ExporterBuildError, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider, Span as SdkSpan};

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, ThreadId};
use paladin_ports::output::trace_sink_port::{
    RunFinishStatus, TraceEvent, TraceRecord, TraceSink, TraceSinkError,
};

use crate::config::trace::OtelConfig;

/// Errors constructing an [`OtelTraceSink`]. Never surfaced to a run: the
/// caller ([`crate::infrastructure::telemetry::build_run_sink`]) logs and
/// treats a construction failure as "OTLP export not attached this run" --
/// diagnostics-only, matching every other `TraceSink`'s own contract
/// (T-28-09-05).
#[derive(Debug, thiserror::Error)]
pub enum OtelSinkError {
    /// The OTLP HTTP client (`reqwest_mcp::Client`) failed to build.
    #[error("failed to build the OTLP HTTP client: {0}")]
    Client(#[from] reqwest_mcp::Error),
    /// The `opentelemetry_otlp` span exporter failed to build.
    #[error("failed to build the OTLP span exporter: {0}")]
    Exporter(#[from] ExporterBuildError),
}

/// Converts a `chrono::DateTime<Utc>` (every [`TraceRecord::at`] and this
/// file's own duration-derived synthetic timestamps) into the
/// `std::time::SystemTime` `SpanBuilder::with_start_time`/
/// `Span::end_with_timestamp` require. `chrono` provides `SystemTime ->
/// DateTime<Utc>` but not the reverse, so this is a small manual
/// conversion rather than a stock `Into` impl. Trace timestamps are always
/// at-or-near "now" in practice (stamped by `TraceDispatcher` at enqueue
/// time), so the `as u64` cast on `timestamp()` never sees a negative
/// (pre-1970) value.
fn to_system_time(at: DateTime<Utc>) -> SystemTime {
    let seconds = at.timestamp().max(0) as u64;
    let nanos = at.timestamp_subsec_nanos();
    UNIX_EPOCH + StdDuration::new(seconds, nanos)
}

/// `OtelTraceSink` implementing [`TraceSink`]: holds the OTel tracer plus
/// the two maps -- `thread_id -> open run span` and
/// `(thread_id, node_id, attempt) -> open attempt span` -- that drive the
/// D-12 span model from the record stream alone.
pub struct OtelTraceSink {
    tracer: SdkTracer,
    /// Kept alive for the sink's own lifetime and explicitly shut down on
    /// `Drop` (flushing/closing the `SimpleSpanProcessor`'s exporter) --
    /// see the `Drop` impl below. Never read otherwise: `self.tracer` holds
    /// its own `Arc`-shared handle into the same processor/exporter chain.
    provider: SdkTracerProvider,
    run_spans: Mutex<HashMap<ThreadId, SdkSpan>>,
    attempt_spans: Mutex<HashMap<(ThreadId, NodeId, u32), SdkSpan>>,
    /// The currently-open attempt number per `(thread_id, node_id)`, so an
    /// event that carries only a `node_id` (no `attempt`) --
    /// `ParleyRaised`, `FallbackHop` -- can resolve to the right attempt
    /// span without scanning `attempt_spans`.
    active_attempt: Mutex<HashMap<(ThreadId, NodeId), u32>>,
}

impl std::fmt::Debug for OtelTraceSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never `#[derive(Debug)]`: `OtelConfig` (the config this sink was
        // built from) is itself deliberately non-`Debug` because
        // `headers` is credential-shaped (security instructions, D-36).
        // Nothing on THIS struct carries header values, but staying
        // manual keeps that guarantee obviously true by inspection rather
        // than by the absence of a field today.
        f.debug_struct("OtelTraceSink").finish_non_exhaustive()
    }
}

impl OtelTraceSink {
    /// Build a production `OtelTraceSink` exporting to `config.endpoint`
    /// over OTLP/HTTP protobuf, with `config.headers` attached to every
    /// export request and `config.service_name` as the exported
    /// `service.name` resource attribute.
    ///
    /// The HTTP client is built with `redirect(Policy::none())` (T-28-09-01):
    /// a `3xx` from the collector can never carry `config.headers`' values
    /// to a different host.
    pub fn new(config: &OtelConfig) -> Result<Self, OtelSinkError> {
        let client = build_reqwest_client()?;
        let headers: HashMap<String, String> = config.headers.clone().into_iter().collect();
        let exporter = SpanExporter::builder()
            .with_http()
            .with_endpoint(config.endpoint.clone())
            .with_headers(headers)
            .with_http_client(client)
            .build()?;
        let resource = Resource::builder()
            .with_service_name(config.service_name.clone())
            .build();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .with_resource(resource)
            .build();
        let tracer = provider.tracer("paladin");
        Ok(Self::from_parts(tracer, provider))
    }

    /// Test-only constructor: builds a tracer/provider pair over a
    /// caller-supplied exporter (`InMemorySpanExporter` in this file's own
    /// tests, D-13a) rather than a real OTLP HTTP client.
    #[cfg(test)]
    fn for_testing<E>(exporter: E, service_name: &str) -> Self
    where
        E: opentelemetry_sdk::trace::SpanExporter + 'static,
    {
        let resource = Resource::builder()
            .with_service_name(service_name.to_string())
            .build();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .with_resource(resource)
            .build();
        let tracer = provider.tracer("paladin");
        Self::from_parts(tracer, provider)
    }

    fn from_parts(tracer: SdkTracer, provider: SdkTracerProvider) -> Self {
        Self {
            tracer,
            provider,
            run_spans: Mutex::new(HashMap::new()),
            attempt_spans: Mutex::new(HashMap::new()),
            active_attempt: Mutex::new(HashMap::new()),
        }
    }

    // ---- run span lifecycle -------------------------------------------

    fn start_run_span(
        &self,
        thread_id: &ThreadId,
        run_id: Option<&RunId>,
        at: DateTime<Utc>,
        graph_fingerprint: &str,
    ) {
        let mut attributes = vec![
            KeyValue::new("paladin.thread_id", thread_id.to_string()),
            KeyValue::new("paladin.graph_fingerprint", graph_fingerprint.to_string()),
        ];
        if let Some(run_id) = run_id {
            attributes.push(KeyValue::new("paladin.run_id", run_id.to_string()));
        }
        let span = self
            .tracer
            .span_builder("run")
            .with_start_time(to_system_time(at))
            .with_attributes(attributes)
            .start_with_context(&self.tracer, &Context::new());
        if let Ok(mut spans) = self.run_spans.lock() {
            spans.insert(thread_id.clone(), span);
        }
    }

    /// Open a synthetic root span flagged `paladin.trace.partial = true` --
    /// this thread's `RunStarted` was dropped or never arrived (D-12).
    fn open_synthetic_run_span(&self, thread_id: &ThreadId, at: DateTime<Utc>) -> SdkSpan {
        let attributes = vec![
            KeyValue::new("paladin.thread_id", thread_id.to_string()),
            KeyValue::new("paladin.trace.partial", true),
        ];
        self.tracer
            .span_builder("run")
            .with_start_time(to_system_time(at))
            .with_attributes(attributes)
            .start_with_context(&self.tracer, &Context::new())
    }

    /// A `Context` carrying `thread_id`'s run span as parent, for building
    /// an attempt span as its child. Synthesizes (and stores) a
    /// flagged-partial root span first when none is open yet (D-12).
    fn parent_context_for_run(&self, thread_id: &ThreadId, at: DateTime<Utc>) -> Context {
        if let Ok(spans) = self.run_spans.lock()
            && let Some(span) = spans.get(thread_id)
        {
            return Context::new().with_remote_span_context(span.span_context().clone());
        }
        let span = self.open_synthetic_run_span(thread_id, at);
        let ctx = Context::new().with_remote_span_context(span.span_context().clone());
        if let Ok(mut spans) = self.run_spans.lock() {
            spans.insert(thread_id.clone(), span);
        }
        ctx
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_run_span(
        &self,
        thread_id: &ThreadId,
        status: &RunFinishStatus,
        total_supersteps: u64,
        total_tokens: u64,
        duration_ms: u64,
        trace_dropped_total: u64,
        at: DateTime<Utc>,
    ) {
        let mut span = match self
            .run_spans
            .lock()
            .ok()
            .and_then(|mut m| m.remove(thread_id))
        {
            Some(span) => span,
            None => {
                let synthetic_start = at - ChronoDuration::milliseconds(duration_ms as i64);
                self.tracer
                    .span_builder("run")
                    .with_start_time(to_system_time(synthetic_start))
                    .with_attributes(vec![
                        KeyValue::new("paladin.thread_id", thread_id.to_string()),
                        KeyValue::new("paladin.trace.partial", true),
                    ])
                    .start_with_context(&self.tracer, &Context::new())
            }
        };
        span.set_attributes(vec![
            KeyValue::new("paladin.status", format!("{status:?}")),
            KeyValue::new("paladin.total_supersteps", total_supersteps as i64),
            KeyValue::new("paladin.total_tokens", total_tokens as i64),
            KeyValue::new("paladin.duration_ms", duration_ms as i64),
            KeyValue::new("paladin.trace_dropped_total", trace_dropped_total as i64),
        ]);
        span.end_with_timestamp(to_system_time(at));
    }

    // ---- attempt span lifecycle -----------------------------------------

    fn start_attempt_span(
        &self,
        thread_id: &ThreadId,
        node_id: &NodeId,
        superstep: u64,
        attempt: u32,
        muster_task_key: Option<&str>,
        at: DateTime<Utc>,
    ) {
        let parent_ctx = self.parent_context_for_run(thread_id, at);
        let mut attributes = vec![
            KeyValue::new("paladin.node_id", node_id.to_string()),
            KeyValue::new("paladin.superstep", superstep as i64),
            KeyValue::new("paladin.attempt", attempt as i64),
        ];
        if let Some(key) = muster_task_key {
            attributes.push(KeyValue::new("paladin.muster_task_key", key.to_string()));
        }
        let span = self
            .tracer
            .span_builder(node_id.to_string())
            .with_start_time(to_system_time(at))
            .with_attributes(attributes)
            .start_with_context(&self.tracer, &parent_ctx);
        if let Ok(mut spans) = self.attempt_spans.lock() {
            spans.insert((thread_id.clone(), node_id.clone(), attempt), span);
        }
        if let Ok(mut active) = self.active_attempt.lock() {
            active.insert((thread_id.clone(), node_id.clone()), attempt);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_attempt_span(
        &self,
        thread_id: &ThreadId,
        node_id: &NodeId,
        superstep: u64,
        attempt: u32,
        outcome: &NodeOutcomeKind,
        duration_ms: u64,
        token_count: u64,
        cache_hit: bool,
        at: DateTime<Utc>,
    ) {
        let key = (thread_id.clone(), node_id.clone(), attempt);
        let mut span = match self
            .attempt_spans
            .lock()
            .ok()
            .and_then(|mut m| m.remove(&key))
        {
            Some(span) => span,
            None => {
                // Orphan NodeFinished: NodeStarted was dropped. Open a
                // synthetic, flagged-partial attempt span rather than
                // discarding this node entirely (D-12).
                let synthetic_start = at - ChronoDuration::milliseconds(duration_ms as i64);
                let parent_ctx = self.parent_context_for_run(thread_id, synthetic_start);
                self.tracer
                    .span_builder(node_id.to_string())
                    .with_start_time(to_system_time(synthetic_start))
                    .with_attributes(vec![
                        KeyValue::new("paladin.node_id", node_id.to_string()),
                        KeyValue::new("paladin.superstep", superstep as i64),
                        KeyValue::new("paladin.attempt", attempt as i64),
                        KeyValue::new("paladin.trace.partial", true),
                    ])
                    .start_with_context(&self.tracer, &parent_ctx)
            }
        };
        span.set_attributes(vec![
            KeyValue::new("paladin.outcome", format!("{outcome:?}")),
            KeyValue::new("paladin.duration_ms", duration_ms as i64),
            KeyValue::new("paladin.tokens", token_count as i64),
            KeyValue::new("paladin.cache_hit", cache_hit),
        ]);
        span.end_with_timestamp(to_system_time(at));

        if let Ok(mut active) = self.active_attempt.lock() {
            let active_key = (thread_id.clone(), node_id.clone());
            if active.get(&active_key) == Some(&attempt) {
                active.remove(&active_key);
            }
        }
    }

    // ---- span events ------------------------------------------------------

    /// Record `name`/`attributes` as an event on `thread_id`'s run span,
    /// synthesizing a flagged-partial root span first if none is open yet
    /// (D-12).
    fn add_run_event(
        &self,
        thread_id: &ThreadId,
        name: &'static str,
        attributes: Vec<KeyValue>,
        at: DateTime<Utc>,
    ) {
        if let Ok(mut spans) = self.run_spans.lock()
            && let Some(span) = spans.get_mut(thread_id)
        {
            span.add_event_with_timestamp(name, to_system_time(at), attributes);
            return;
        }
        let mut span = self.open_synthetic_run_span(thread_id, at);
        span.add_event_with_timestamp(name, to_system_time(at), attributes);
        if let Ok(mut spans) = self.run_spans.lock() {
            spans.insert(thread_id.clone(), span);
        }
    }

    /// Record `name`/`attributes` as an event on `node_id`'s currently-open
    /// attempt span, falling back to the run span (D-12) if this node has
    /// no open attempt right now (a dropped `NodeStarted`, or the event
    /// arrived after `NodeFinished` already closed it).
    fn add_node_event(
        &self,
        thread_id: &ThreadId,
        node_id: &NodeId,
        name: &'static str,
        attributes: Vec<KeyValue>,
        at: DateTime<Utc>,
    ) {
        let attempt = self
            .active_attempt
            .lock()
            .ok()
            .and_then(|m| m.get(&(thread_id.clone(), node_id.clone())).copied());
        if let Some(attempt) = attempt {
            let key = (thread_id.clone(), node_id.clone(), attempt);
            if let Ok(mut spans) = self.attempt_spans.lock()
                && let Some(span) = spans.get_mut(&key)
            {
                span.add_event_with_timestamp(name, to_system_time(at), attributes);
                return;
            }
        }
        self.add_run_event(thread_id, name, attributes, at);
    }
}

impl Drop for OtelTraceSink {
    fn drop(&mut self) {
        // Flush/close the SimpleSpanProcessor's exporter. Diagnostics-only:
        // a shutdown failure here is this sink's own teardown malfunction,
        // never something that should panic on drop.
        let _ = self.provider.shutdown();
    }
}

/// Build the `reqwest_mcp::Client` every OTLP export request is sent
/// through: no redirects (T-28-09-01, mirrors the house pattern in
/// `crates/paladin-llm/src/openai/adapter.rs` and
/// `src/application/services/run/webhook/client.rs`) -- a `3xx` from the
/// configured collector can never carry `OtelConfig.headers`' credential-shaped
/// values to a different, attacker-influenced host.
fn build_reqwest_client() -> Result<reqwest_mcp::Client, reqwest_mcp::Error> {
    reqwest_mcp::Client::builder()
        .redirect(reqwest_mcp::redirect::Policy::none())
        .build()
}

#[async_trait]
impl TraceSink for OtelTraceSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        let thread_id = record.thread_id.clone();
        let at = record.at;
        match &record.event {
            TraceEvent::RunStarted {
                run_id,
                graph_fingerprint,
            } => {
                // CR-01 (28-REVIEW): fall back to the envelope's own
                // `record.run_id` when the event-level field is `None` --
                // defense-in-depth so a future producer that repeats the
                // hardcoded-`None` mistake doesn't silently reintroduce a
                // missing `paladin.run_id` on the OTel root span.
                let run_id = run_id.as_ref().or(record.run_id.as_ref());
                self.start_run_span(&thread_id, run_id, at, graph_fingerprint);
            }
            TraceEvent::NodeStarted {
                superstep,
                node_id,
                attempt,
                muster_task_key,
            } => {
                self.start_attempt_span(
                    &thread_id,
                    node_id,
                    *superstep,
                    *attempt,
                    muster_task_key.as_deref(),
                    at,
                );
            }
            TraceEvent::NodeFinished {
                superstep,
                node_id,
                attempt,
                outcome,
                duration_ms,
                token_count,
                cache_hit,
            } => {
                self.finish_attempt_span(
                    &thread_id,
                    node_id,
                    *superstep,
                    *attempt,
                    outcome,
                    *duration_ms,
                    *token_count,
                    *cache_hit,
                    at,
                );
            }
            TraceEvent::EdgeEvaluated {
                from,
                to,
                condition_kind,
                fired,
            } => {
                self.add_run_event(
                    &thread_id,
                    "edge_evaluated",
                    vec![
                        KeyValue::new("from", from.to_string()),
                        KeyValue::new("to", to.to_string()),
                        KeyValue::new("condition_kind", condition_kind.clone()),
                        KeyValue::new("fired", *fired),
                    ],
                    at,
                );
            }
            TraceEvent::DeltaMerged {
                superstep,
                field_changes,
            } => {
                self.add_run_event(
                    &thread_id,
                    "delta_merged",
                    vec![
                        KeyValue::new("superstep", *superstep as i64),
                        KeyValue::new("field_count", field_changes.len() as i64),
                    ],
                    at,
                );
            }
            TraceEvent::ParleyRaised {
                parley_id,
                node_id,
                parley_kind,
            } => {
                self.add_node_event(
                    &thread_id,
                    node_id,
                    "parley_raised",
                    vec![
                        KeyValue::new("parley_id", parley_id.to_string()),
                        KeyValue::new("parley_kind", format!("{parley_kind:?}")),
                    ],
                    at,
                );
            }
            TraceEvent::RunFinished {
                status,
                total_supersteps,
                total_tokens,
                duration_ms,
                trace_dropped_total,
            } => {
                self.finish_run_span(
                    &thread_id,
                    status,
                    *total_supersteps,
                    *total_tokens,
                    *duration_ms,
                    *trace_dropped_total,
                    at,
                );
            }
            TraceEvent::FallbackHop {
                node_id,
                from_provider,
                to_provider,
            } => {
                let attributes = vec![
                    KeyValue::new("from_provider", from_provider.clone()),
                    KeyValue::new("to_provider", to_provider.clone()),
                ];
                match node_id {
                    Some(node_id) => {
                        self.add_node_event(&thread_id, node_id, "fallback_hop", attributes, at)
                    }
                    None => self.add_run_event(&thread_id, "fallback_hop", attributes, at),
                }
            }
            TraceEvent::MiddlewareEvent { name, action } => {
                self.add_run_event(
                    &thread_id,
                    "middleware_event",
                    vec![
                        KeyValue::new("name", name.clone()),
                        KeyValue::new("action", format!("{action:?}")),
                    ],
                    at,
                );
            }
            // SuperstepStarted, NodeProgress, WaypointSaved: outside D-12's
            // span-per-attempt model (not tested by this plan's <behavior>
            // list). `TraceEvent` is `#[non_exhaustive]`, so this wildcard
            // arm is also what keeps a future variant from breaking this
            // downstream crate's build.
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_sdk::trace::InMemorySpanExporter;
    use paladin_core::platform::container::parley::{ParleyId, ParleyKind};
    use paladin_ports::output::trace_sink_port::FieldChange;

    fn thread(id: &str) -> ThreadId {
        ThreadId::new(id).unwrap()
    }

    fn record(thread_id: ThreadId, seq: u64, at: DateTime<Utc>, event: TraceEvent) -> TraceRecord {
        TraceRecord {
            thread_id,
            run_id: None,
            seq,
            at,
            event,
        }
    }

    fn sink_and_exporter(service_name: &str) -> (OtelTraceSink, InMemorySpanExporter) {
        let exporter = InMemorySpanExporter::default();
        let sink = OtelTraceSink::for_testing(exporter.clone(), service_name);
        (sink, exporter)
    }

    /// Behavior: `run_start_and_finish_produce_one_root_span` -- a record
    /// stream of `RunStarted` ... `RunFinished` produces exactly one root
    /// span named for the run, whose start/end timestamps equal the
    /// records' `at` values rather than wall-clock-at-export.
    #[tokio::test(flavor = "multi_thread")]
    async fn run_start_and_finish_produce_one_root_span() {
        let (sink, exporter) = sink_and_exporter("run-root-test");
        let t = thread("t-root");
        let start = Utc::now();
        let end = start + ChronoDuration::milliseconds(500);

        sink.on_event(record(
            t.clone(),
            1,
            start,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".into(),
            },
        ))
        .await
        .unwrap();
        sink.on_event(record(
            t,
            2,
            end,
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 1,
                total_tokens: 0,
                duration_ms: 500,
                trace_dropped_total: 0,
            },
        ))
        .await
        .unwrap();

        let spans = exporter.get_finished_spans().unwrap();
        assert_eq!(spans.len(), 1, "exactly one root span: {spans:?}");
        assert_eq!(spans[0].name, "run");
        assert_eq!(spans[0].start_time, to_system_time(start));
        assert_eq!(spans[0].end_time, to_system_time(end));
        assert!(
            !spans[0]
                .attributes
                .iter()
                .any(|kv| kv.key.as_str() == "paladin.trace.partial")
        );
    }

    /// CR-01 (28-REVIEW): when `TraceEvent::RunStarted.run_id` is `None`
    /// but the envelope's own `TraceRecord.run_id` is populated (the shape
    /// every hardcoded-`None` emit site produced before the fix), the root
    /// span must still carry `paladin.run_id` from the envelope fallback --
    /// defense-in-depth against a future producer repeating the mistake.
    #[tokio::test(flavor = "multi_thread")]
    async fn run_started_falls_back_to_envelope_run_id_when_event_field_is_none() {
        let (sink, exporter) = sink_and_exporter("run-id-fallback-test");
        let t = thread("t-run-id-fallback");
        let run_id = RunId::new_v7();
        let start = Utc::now();
        let end = start + ChronoDuration::milliseconds(10);

        sink.on_event(TraceRecord {
            thread_id: t.clone(),
            run_id: Some(run_id.clone()),
            seq: 1,
            at: start,
            event: TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".into(),
            },
        })
        .await
        .unwrap();
        sink.on_event(TraceRecord {
            thread_id: t,
            run_id: Some(run_id.clone()),
            seq: 2,
            at: end,
            event: TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 0,
                total_tokens: 0,
                duration_ms: 10,
                trace_dropped_total: 0,
            },
        })
        .await
        .unwrap();

        let spans = exporter.get_finished_spans().unwrap();
        assert_eq!(spans.len(), 1, "exactly one root span: {spans:?}");
        let run_id_attr = spans[0]
            .attributes
            .iter()
            .find(|kv| kv.key.as_str() == "paladin.run_id")
            .expect("paladin.run_id must be set from the envelope's run_id fallback");
        assert_eq!(run_id_attr.value.as_str(), run_id.to_string());
    }

    /// Behavior: `retried_node_produces_sibling_attempt_spans` -- a node
    /// with two attempts produces two child spans of the root, siblings
    /// (not nested), carrying `attempt` 1 and 2 and their own outcomes.
    #[tokio::test(flavor = "multi_thread")]
    async fn retried_node_produces_sibling_attempt_spans() {
        let (sink, exporter) = sink_and_exporter("retry-test");
        let t = thread("t-retry");
        let node = NodeId::new("writer");
        let base = Utc::now();

        sink.on_event(record(
            t.clone(),
            1,
            base,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".into(),
            },
        ))
        .await
        .unwrap();
        for attempt in [1u32, 2u32] {
            let started = base + ChronoDuration::milliseconds(10 * attempt as i64);
            let finished = started + ChronoDuration::milliseconds(5);
            let outcome = if attempt == 1 {
                NodeOutcomeKind::Failed
            } else {
                NodeOutcomeKind::Succeeded
            };
            sink.on_event(record(
                t.clone(),
                10 + attempt as u64,
                started,
                TraceEvent::NodeStarted {
                    superstep: 1,
                    node_id: node.clone(),
                    attempt,
                    muster_task_key: None,
                },
            ))
            .await
            .unwrap();
            sink.on_event(record(
                t.clone(),
                20 + attempt as u64,
                finished,
                TraceEvent::NodeFinished {
                    superstep: 1,
                    node_id: node.clone(),
                    attempt,
                    outcome,
                    duration_ms: 5,
                    token_count: 3,
                    cache_hit: false,
                },
            ))
            .await
            .unwrap();
        }
        sink.on_event(record(
            t,
            99,
            base + ChronoDuration::milliseconds(100),
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 1,
                total_tokens: 3,
                duration_ms: 100,
                trace_dropped_total: 0,
            },
        ))
        .await
        .unwrap();

        let spans = exporter.get_finished_spans().unwrap();
        let root = spans.iter().find(|s| s.name == "run").expect("root span");
        let attempts: Vec<_> = spans.iter().filter(|s| s.name == "writer").collect();
        assert_eq!(attempts.len(), 2, "two sibling attempt spans: {spans:?}");
        for span in &attempts {
            assert_eq!(
                span.parent_span_id,
                root.span_context.span_id(),
                "each attempt is a direct child of root, not nested under the other attempt"
            );
        }
        let attempt_numbers: Vec<i64> = attempts
            .iter()
            .map(|s| {
                s.attributes
                    .iter()
                    .find(|kv| kv.key.as_str() == "paladin.attempt")
                    .map(|kv| match &kv.value {
                        opentelemetry::Value::I64(v) => *v,
                        other => panic!("unexpected attempt value type: {other:?}"),
                    })
                    .unwrap()
            })
            .collect();
        assert!(attempt_numbers.contains(&1) && attempt_numbers.contains(&2));
    }

    /// Behavior: `attempt_span_carries_every_attribute` -- an attempt span
    /// has `node_id`, `superstep`, `attempt`, `outcome`, `tokens`,
    /// `cache_hit` and `muster_task_key`.
    #[tokio::test(flavor = "multi_thread")]
    async fn attempt_span_carries_every_attribute() {
        let (sink, exporter) = sink_and_exporter("attrs-test");
        let t = thread("t-attrs");
        let node = NodeId::new("muster-worker");
        let base = Utc::now();

        sink.on_event(record(
            t.clone(),
            1,
            base,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".into(),
            },
        ))
        .await
        .unwrap();
        sink.on_event(record(
            t.clone(),
            2,
            base,
            TraceEvent::NodeStarted {
                superstep: 3,
                node_id: node.clone(),
                attempt: 1,
                muster_task_key: Some("task-key-1".into()),
            },
        ))
        .await
        .unwrap();
        sink.on_event(record(
            t,
            3,
            base + ChronoDuration::milliseconds(5),
            TraceEvent::NodeFinished {
                superstep: 3,
                node_id: node,
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 5,
                token_count: 42,
                cache_hit: true,
            },
        ))
        .await
        .unwrap();

        let spans = exporter.get_finished_spans().unwrap();
        let attempt = spans
            .iter()
            .find(|s| s.name == "muster-worker")
            .expect("attempt span");
        let names: Vec<&str> = attempt
            .attributes
            .iter()
            .map(|kv| kv.key.as_str())
            .collect();
        for expected in [
            "paladin.node_id",
            "paladin.superstep",
            "paladin.attempt",
            "paladin.outcome",
            "paladin.tokens",
            "paladin.cache_hit",
            "paladin.muster_task_key",
        ] {
            assert!(
                names.contains(&expected),
                "missing attribute {expected}, got {names:?}"
            );
        }
    }

    /// Behavior: `branch_retry_muster_fixture_tree_shape` -- a branching,
    /// retrying, mustering fixture yields one root, the expected number of
    /// attempt children, edge/delta events on the run span, and the parley
    /// event on the attempt span.
    #[tokio::test(flavor = "multi_thread")]
    async fn branch_retry_muster_fixture_tree_shape() {
        let (sink, exporter) = sink_and_exporter("fixture-test");
        let t = thread("t-fixture");
        let base = Utc::now();
        let reviewer = NodeId::new("reviewer");
        let writer = NodeId::new("writer");

        sink.on_event(record(
            t.clone(),
            1,
            base,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".into(),
            },
        ))
        .await
        .unwrap();

        // Branch: two edges evaluated on the run span.
        sink.on_event(record(
            t.clone(),
            2,
            base,
            TraceEvent::EdgeEvaluated {
                from: writer.clone(),
                to: reviewer.clone(),
                condition_kind: "always".into(),
                fired: true,
            },
        ))
        .await
        .unwrap();
        sink.on_event(record(
            t.clone(),
            3,
            base,
            TraceEvent::EdgeEvaluated {
                from: writer.clone(),
                to: writer.clone(),
                condition_kind: "contains".into(),
                fired: false,
            },
        ))
        .await
        .unwrap();

        // Retry: writer attempts 1 (fails) and 2 (succeeds).
        for attempt in [1u32, 2u32] {
            let started = base + ChronoDuration::milliseconds(10 * attempt as i64);
            let finished = started + ChronoDuration::milliseconds(5);
            let outcome = if attempt == 1 {
                NodeOutcomeKind::Failed
            } else {
                NodeOutcomeKind::Succeeded
            };
            sink.on_event(record(
                t.clone(),
                10 + attempt as u64,
                started,
                TraceEvent::NodeStarted {
                    superstep: 1,
                    node_id: writer.clone(),
                    attempt,
                    muster_task_key: None,
                },
            ))
            .await
            .unwrap();
            if attempt == 1 {
                // Parley raised mid-attempt: must land on the writer's
                // OPEN attempt-1 span, not the run span.
                sink.on_event(record(
                    t.clone(),
                    15,
                    started + ChronoDuration::milliseconds(2),
                    TraceEvent::ParleyRaised {
                        parley_id: ParleyId::new(),
                        node_id: writer.clone(),
                        parley_kind: ParleyKind::Approval,
                    },
                ))
                .await
                .unwrap();
            }
            sink.on_event(record(
                t.clone(),
                20 + attempt as u64,
                finished,
                TraceEvent::NodeFinished {
                    superstep: 1,
                    node_id: writer.clone(),
                    attempt,
                    outcome,
                    duration_ms: 5,
                    token_count: 1,
                    cache_hit: false,
                },
            ))
            .await
            .unwrap();
        }

        // Muster: reviewer runs once as a synthetic worker-task dispatch.
        sink.on_event(record(
            t.clone(),
            30,
            base + ChronoDuration::milliseconds(30),
            TraceEvent::NodeStarted {
                superstep: 2,
                node_id: reviewer.clone(),
                attempt: 1,
                muster_task_key: Some("mk-1".into()),
            },
        ))
        .await
        .unwrap();
        sink.on_event(record(
            t.clone(),
            31,
            base + ChronoDuration::milliseconds(35),
            TraceEvent::NodeFinished {
                superstep: 2,
                node_id: reviewer,
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 5,
                token_count: 2,
                cache_hit: false,
            },
        ))
        .await
        .unwrap();

        sink.on_event(record(
            t.clone(),
            40,
            base,
            TraceEvent::DeltaMerged {
                superstep: 1,
                field_changes: vec![FieldChange {
                    field: paladin_core::platform::container::battlefield::FieldName::new("draft")
                        .unwrap(),
                    dispatch: "last_write".into(),
                    writers: vec![writer.clone()],
                    value_bytes: 12,
                    value: None,
                }],
            },
        ))
        .await
        .unwrap();

        sink.on_event(record(
            t,
            99,
            base + ChronoDuration::milliseconds(50),
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 2,
                total_tokens: 3,
                duration_ms: 50,
                trace_dropped_total: 0,
            },
        ))
        .await
        .unwrap();

        let spans = exporter.get_finished_spans().unwrap();
        let root = spans
            .iter()
            .find(|s| s.name == "run")
            .expect("one root span");
        let writer_spans: Vec<_> = spans.iter().filter(|s| s.name == "writer").collect();
        let reviewer_spans: Vec<_> = spans.iter().filter(|s| s.name == "reviewer").collect();
        assert_eq!(
            writer_spans.len(),
            2,
            "two writer attempt children: {spans:?}"
        );
        assert_eq!(
            reviewer_spans.len(),
            1,
            "one reviewer (muster) attempt child"
        );

        let root_event_names: Vec<&str> =
            root.events.events.iter().map(|e| e.name.as_ref()).collect();
        assert_eq!(
            root_event_names
                .iter()
                .filter(|&&n| n == "edge_evaluated")
                .count(),
            2,
            "both edges land on the run span"
        );
        assert!(
            root_event_names.contains(&"delta_merged"),
            "delta_merged lands on the run span"
        );

        let attempt1 = writer_spans
            .iter()
            .find(|s| {
                s.attributes.iter().any(|kv| {
                    kv.key.as_str() == "paladin.attempt"
                        && matches!(kv.value, opentelemetry::Value::I64(1))
                })
            })
            .expect("attempt-1 writer span");
        let attempt1_event_names: Vec<&str> = attempt1
            .events
            .events
            .iter()
            .map(|e| e.name.as_ref())
            .collect();
        assert!(
            attempt1_event_names.contains(&"parley_raised"),
            "parley event lands on the writer's own open attempt span, not the run span: {attempt1_event_names:?}"
        );
    }

    /// Behavior: `orphan_record_opens_a_partial_span` -- a `NodeFinished`
    /// whose `NodeStarted` was dropped produces a span flagged
    /// `paladin.trace.partial = true` rather than being discarded.
    #[tokio::test(flavor = "multi_thread")]
    async fn orphan_record_opens_a_partial_span() {
        let (sink, exporter) = sink_and_exporter("orphan-test");
        let t = thread("t-orphan");
        let node = NodeId::new("orphaned-node");
        let finished = Utc::now();

        // No RunStarted, no NodeStarted -- straight to NodeFinished. This
        // also synthesizes an OPEN (not yet exported -- SimpleSpanProcessor
        // only exports on end) partial root span as the attempt span's
        // parent, since RunStarted was never seen either.
        sink.on_event(record(
            t.clone(),
            1,
            finished,
            TraceEvent::NodeFinished {
                superstep: 1,
                node_id: node,
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 5,
                token_count: 0,
                cache_hit: false,
            },
        ))
        .await
        .unwrap();

        let spans = exporter.get_finished_spans().unwrap();
        let attempt = spans
            .iter()
            .find(|s| s.name == "orphaned-node")
            .expect("synthetic attempt span");
        assert!(
            attempt
                .attributes
                .iter()
                .any(|kv| kv.key.as_str() == "paladin.trace.partial"
                    && matches!(kv.value, opentelemetry::Value::Bool(true))),
            "orphan attempt span must be flagged partial: {:?}",
            attempt.attributes
        );
        assert!(
            spans.iter().find(|s| s.name == "run").is_none(),
            "the synthesized root span is still OPEN (no RunFinished yet) -- SimpleSpanProcessor only \
             exports on end, so it must not appear yet: {spans:?}"
        );

        // Now close the run: the synthesized root span (created above,
        // still open) must be the SAME span RunFinished closes -- also
        // flagged partial, and it must be the attempt span's real parent.
        sink.on_event(record(
            t,
            2,
            finished + ChronoDuration::milliseconds(10),
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 1,
                total_tokens: 0,
                duration_ms: 15,
                trace_dropped_total: 0,
            },
        ))
        .await
        .unwrap();

        let spans = exporter.get_finished_spans().unwrap();
        let root = spans
            .iter()
            .find(|s| s.name == "run")
            .expect("the synthesized root span, now closed by RunFinished");
        assert!(
            root.attributes
                .iter()
                .any(|kv| kv.key.as_str() == "paladin.trace.partial"
                    && matches!(kv.value, opentelemetry::Value::Bool(true))),
            "the synthesized root span must also be flagged partial: {:?}",
            root.attributes
        );
        assert_eq!(
            attempt.parent_span_id,
            root.span_context.span_id(),
            "the orphan attempt span's parent is the SAME synthesized root span RunFinished later closes, not a second, disconnected one"
        );
    }

    /// Behavior: `otel_sink_absent_without_the_feature` -- a witness that
    /// this whole module (and therefore `OtelTraceSink`) only exists when
    /// compiled with `--features otel`: this file's own presence in the
    /// compiled test binary IS the proof (it is declared
    /// `#[cfg(feature = "otel")]` in `mod.rs`), so `build_run_sink` never
    /// references the exporter on a default build. No runtime assertion is
    /// possible from inside a file that only compiles when the feature is
    /// on; the acceptance criterion (`cargo build` with default features
    /// succeeding, with no `opentelemetry*` in `cargo tree`) is verified at
    /// the plan level instead.
    #[test]
    fn otel_sink_absent_without_the_feature() {
        // Compiling this test AT ALL already proves the `otel` feature is
        // active for this build -- see the module doc above.
    }
}
