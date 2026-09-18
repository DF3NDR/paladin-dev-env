// examples/observability_tracing.rs
//
// Observability: the Trace Envelope, Configuration and Persisted History
// (EX-100, EX-101, EX-102, EX-108) -- PRD 07 §2.1, D-01/D-02/D-36
//
// This program is fully offline (default features, no LLM provider key
// needed -- the demo graph uses a pure Function node, never a Paladin). It
// demonstrates:
//   1. consuming the trace envelope: running a small graph with a custom
//      in-process TraceSink attached, and printing each captured
//      TraceRecord's envelope fields plus the real TraceEvent variant name
//      it carried (EX-100),
//   2. configuring tracing: constructing a TraceConfig and resolving the
//      facade's own sink-composition function
//      (`paladin::infrastructure::telemetry::build_run_sink`) from it --
//      printing how a single changed field (`log_sink`) visibly changes
//      whether ANY sink is attached at all (EX-101),
//   3. toggling OTLP export from the environment
//      (PALADIN_TRACE_OTEL_ENABLED), and showing that exporting ALSO
//      requires the `otel` Cargo feature -- this build was compiled
//      without it, so the flag alone changes nothing observable here
//      (EX-102), and
//   4. querying the persisted trace history: the same run's records,
//      flushed through a PersistingTraceSink into a local
//      InMemoryRunTraceStore, read back via RunTracePort::read and printed
//      by run id, sequence and event name (EX-108).
//
// To run this example:
// ```bash
// cargo run --example observability_tracing
// ```

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use paladin::config::env_utils::EnvOverridable;
use paladin::config::trace::TraceConfig;
use paladin::infrastructure::telemetry::{PersistingTraceSink, build_run_sink};
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::graph::{EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::trace::TraceEvent;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::trace_sink_port::{
    CompositeSink, TraceRecord, TraceSink, TraceSinkError,
};
use paladin_storage::run_trace::in_memory::InMemoryRunTraceStore;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// `WarEngine::new` requires a `PaladinPort`; the graph below runs a
/// Function node only, so this is never actually invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this program's graph runs a Function node only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this program's graph runs a Function node only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Writes a fixed string to `field` and routes onward -- enough surface to
/// produce a full RunStarted/SuperstepStarted/NodeStarted/NodeFinished/
/// DeltaMerged/WaypointSaved/RunFinished envelope.
struct WriteField {
    field: FieldName,
    value: String,
}

#[async_trait]
impl StateNode for WriteField {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let mut delta = StateDelta::new();
        delta
            .set(self.field.clone(), self.value.clone())
            .map_err(|e| StateNodeError(e.to_string()))?;
        Ok(Directive {
            delta,
            next: NextStep::Edges,
        })
    }
}

/// A `TraceSink` that captures every record it receives in-process, so this
/// program can inspect its own run's envelope directly rather than reading
/// a log line.
struct RecordingSink {
    records: std::sync::Mutex<Vec<TraceRecord>>,
}

impl RecordingSink {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            records: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn records(&self) -> Vec<TraceRecord> {
        self.records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl TraceSink for RecordingSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        self.records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(record);
        Ok(())
    }
}

/// The real, non-paraphrased `TraceEvent` variant name -- `#[non_exhaustive]`
/// means the wildcard arm covers any future variant this build does not
/// know about yet.
fn event_name(event: &TraceEvent) -> &'static str {
    match event {
        TraceEvent::RunStarted { .. } => "RunStarted",
        TraceEvent::SuperstepStarted { .. } => "SuperstepStarted",
        TraceEvent::NodeStarted { .. } => "NodeStarted",
        TraceEvent::NodeProgress { .. } => "NodeProgress",
        TraceEvent::NodeFinished { .. } => "NodeFinished",
        TraceEvent::EdgeEvaluated { .. } => "EdgeEvaluated",
        TraceEvent::DeltaMerged { .. } => "DeltaMerged",
        TraceEvent::WaypointSaved { .. } => "WaypointSaved",
        TraceEvent::ParleyRaised { .. } => "ParleyRaised",
        TraceEvent::RunFinished { .. } => "RunFinished",
        TraceEvent::FallbackHop { .. } => "FallbackHop",
        TraceEvent::MiddlewareEvent { .. } => "MiddlewareEvent",
        _ => "Unknown",
    }
}

/// Runs the demo graph once with a `CompositeSink` fanning out to `recording`
/// (EX-100's own capture) and a `PersistingTraceSink` over `trace_store`
/// (EX-108's persistence), bound to `run_id` so every record carries it.
async fn run_demo_graph(
    recording: Arc<RecordingSink>,
    trace_store: Arc<InMemoryRunTraceStore>,
    thread: ThreadId,
    run_id: RunId,
) -> Result<(), Box<dyn std::error::Error>> {
    let greeting = FieldName::new("greeting")?;
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        greeting.clone(),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    let node_id = NodeId::new("greeter");
    graph.add_node(
        node_id.clone(),
        NodeSpec::Function(Arc::new(WriteField {
            field: greeting,
            value: "hello".to_string(),
        })),
    );
    graph.add_entry(node_id);

    let persisting: Arc<dyn TraceSink> = Arc::new(PersistingTraceSink::new(trace_store));
    let recording_sink: Arc<dyn TraceSink> = recording.clone();
    let composite: Arc<dyn TraceSink> =
        Arc::new(CompositeSink::new(vec![recording_sink, persisting]));

    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(Arc::new(UnusedPaladinPort), store)
        .with_trace_sink(composite)
        .with_bound_trace(thread.clone(), Some(run_id));

    engine.start(&graph, thread, StateDelta::new()).await?;

    // Trace dispatch is fire-and-forget: rather than a fixed sleep (a race
    // under CPU contention), poll the recording sink's record count,
    // bounded by a generous timeout, until it stops growing across two
    // consecutive checks -- the same "let the background consumer drain"
    // intent paladin-battalion's own trace tests rely on, made robust here
    // via polling instead of a fixed delay.
    let poll_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut previous_len = recording.records().len();
    loop {
        tokio::time::sleep(Duration::from_millis(5)).await;
        let current_len = recording.records().len();
        if current_len == previous_len {
            break;
        }
        previous_len = current_len;
        if tokio::time::Instant::now() >= poll_deadline {
            break;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== observability_tracing: EX-100, EX-101, EX-102, EX-108 ===\n");

    let recording = RecordingSink::new();
    let trace_store = Arc::new(InMemoryRunTraceStore::new());
    let thread = ThreadId::new("observability-tracing-demo")?;
    let run_id = RunId::new_v7();

    run_demo_graph(
        recording.clone(),
        trace_store.clone(),
        thread.clone(),
        run_id.clone(),
    )
    .await?;

    // Part 1 (EX-100): consume the trace envelope.
    println!("--- Part 1: consuming the trace envelope (EX-100) ---");
    let records = recording.records();
    let mut seen_variants: BTreeSet<&'static str> = BTreeSet::new();
    println!(
        "Captured {} TraceRecord(s) for run {} (thread '{}'):",
        records.len(),
        run_id.as_str(),
        thread.as_str()
    );
    for record in &records {
        let name = event_name(&record.event);
        seen_variants.insert(name);
        println!(
            "  seq={} at={} thread_id={} run_id={:?} event={name}",
            record.seq,
            record.at,
            record.thread_id.as_str(),
            record.run_id.as_ref().map(RunId::as_str)
        );
    }
    println!("Distinct event variants observed: {seen_variants:?}\n");

    // Part 2 (EX-101): configure tracing via TraceConfig, and show the
    // effect of changing one field.
    println!("--- Part 2: configuring tracing via TraceConfig (EX-101) ---");
    let mut config = TraceConfig::default();
    println!(
        "TraceConfig defaults: log_sink={}, persist={}, state_values={}, otel.enabled={}",
        config.log_sink, config.persist, config.state_values, config.otel.enabled
    );

    // Start from every sink source off, so build_run_sink resolves to no
    // sink at all -- the untraced fast path.
    config.log_sink = false;
    let no_bus_no_persist: Option<Arc<dyn TraceSink>> = None;
    let before = build_run_sink(&config, no_bus_no_persist.clone(), None);
    println!(
        "log_sink=false, no bus sink, no persist backend -> build_run_sink resolves to: {}",
        if before.is_some() {
            "a sink is attached"
        } else {
            "no sink attached (the untraced fast path)"
        }
    );

    // Flip exactly one field -- log_sink -- and re-resolve: the changed
    // field's effect is directly visible in the resolved sink presence.
    config.log_sink = true;
    let after = build_run_sink(&config, no_bus_no_persist, None);
    println!(
        "log_sink=true (only this field changed) -> build_run_sink resolves to: {}\n",
        if after.is_some() {
            "a sink is attached (the built-in log sink)"
        } else {
            "no sink attached"
        }
    );

    // Part 3 (EX-102): toggle OTLP export from the environment.
    println!("--- Part 3: toggling OTLP export from the environment (EX-102) ---");
    println!(
        "Before: PALADIN_TRACE_OTEL_ENABLED unset -> config.otel.enabled={}",
        config.otel.enabled
    );
    // Safety: single-threaded at this point in `main`.
    unsafe {
        std::env::set_var("PALADIN_TRACE_OTEL_ENABLED", "true");
    }
    config.apply_env_overrides();
    println!(
        "After: PALADIN_TRACE_OTEL_ENABLED=true -> config.otel.enabled={}",
        config.otel.enabled
    );
    println!(
        "Exporting still additionally requires the `otel` Cargo feature -- this build was \
         compiled without it, so build_run_sink's OTel branch is compiled out entirely and \
         no exporter attaches even with the flag on. See the sibling \
         `observability_otel_export` example (built under --features \"otel\") for the \
         export sink itself."
    );
    unsafe {
        std::env::remove_var("PALADIN_TRACE_OTEL_ENABLED");
    }
    println!("PALADIN_TRACE_OTEL_ENABLED restored to unset.\n");

    // Part 4 (EX-108): query the persisted trace history.
    println!("--- Part 4: querying the persisted trace history (EX-108) ---");
    let rows = trace_store.read(&thread, 0, 1000).await?;
    println!(
        "Persisted {} row(s) for thread '{}':",
        rows.len(),
        thread.as_str()
    );
    for row in &rows {
        println!(
            "  run_id={:?} seq={} event={}",
            row.run_id.as_ref().map(RunId::as_str),
            row.seq,
            event_name(&row.event)
        );
    }

    println!("\n=== observability_tracing complete ===");
    Ok(())
}
