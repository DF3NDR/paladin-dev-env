// examples/observability_otel_export.rs
//
// OTLP Trace Export (EX-103) -- PRD 07 §2.1, 28-09, D-12/D-13
//
// Wires the `otel`-gated `OtelTraceSink` onto a `WarEngine` run and shows
// the endpoint configuration it exports through.
//
// This program needs a REACHABLE OTLP/HTTP collector at the configured
// endpoint (http://localhost:4318/v1/traces by default -- an OpenTelemetry
// Collector, Jaeger, or similar telemetry backend). None runs in this
// devcontainer or in CI, so this program is build-verified only: never
// executed by any automated check here or in CI (D-16). Enabling the
// `otel` feature pulls in three telemetry crates: `opentelemetry`,
// `opentelemetry_sdk` and `opentelemetry-otlp`.
//
// Build it with:
//   cargo build --example observability_otel_export --features "otel"
//
// Run it (once a collector is reachable at the configured endpoint) with:
//   cargo run --example observability_otel_export --features "otel"
//
// No LLM provider key is read or needed -- the demo graph uses a pure
// Function node, never a Paladin.

use std::sync::Arc;

use async_trait::async_trait;

use paladin::config::trace::OtelConfig;
use paladin::infrastructure::telemetry::OtelTraceSink;
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::graph::{EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::trace_sink_port::TraceSink;
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

/// Writes a fixed string to `field` -- enough surface to produce a full
/// RunStarted/NodeStarted/NodeFinished/RunFinished envelope for the
/// exporter to turn into a span tree.
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== observability_otel_export: EX-103 ===\n");

    let otel_config = OtelConfig {
        enabled: true,
        ..OtelConfig::default()
    };
    println!(
        "OTLP endpoint: {} (service.name={})",
        otel_config.endpoint, otel_config.service_name
    );
    println!(
        "This build was compiled with the `otel` feature, so OtelTraceSink is available. \
         Constructing it against the endpoint above -- needs a reachable OTLP/HTTP \
         collector (see this file's header comment)...\n"
    );

    let sink: Arc<dyn TraceSink> = Arc::new(OtelTraceSink::new(&otel_config)?);

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
            value: "hello from observability_otel_export".to_string(),
        })),
    );
    graph.add_entry(node_id);

    let store = Arc::new(InMemoryWaypointStore::new());
    let thread = ThreadId::new("observability-otel-export-demo")?;
    let engine = WarEngine::new(Arc::new(UnusedPaladinPort), store).with_trace_sink(sink);

    println!(
        "Running the demo graph -- every RunStarted/NodeStarted/NodeFinished/RunFinished \
         record this run produces is turned into a span tree and exported to {}.",
        otel_config.endpoint
    );
    engine.start(&graph, thread, StateDelta::new()).await?;

    println!("\n=== observability_otel_export complete (spans exported, or attempted) ===");
    Ok(())
}
