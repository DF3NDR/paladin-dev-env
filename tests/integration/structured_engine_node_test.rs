//! Phase 26 Plan 18 (RT-05, RT-FR-19, D-29): a `NodeSpec::Paladin` node
//! declaring `output_schema` dispatches through the engine's structured
//! executor and writes the PARSED JSON VALUE -- never a string -- into its
//! `output_field`, so a downstream node receives typed structure rather than
//! a string to re-parse. PRD 05 Section 3.5's engine acceptance item.
//!
//! Exercised end-to-end through a REAL two-node `WarGraph`
//! (`worker` -> `reader`) so Test 2 actually demonstrates typed
//! agent-to-agent data flow: the point of RT-FR-19 is not that a value
//! parses, it is that the NEXT node receives structure.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_battalion::engine::graph::WarGraph;
use paladin_battalion::engine::{
    EngineLimits, InputMapping, NodeContext, NodeSpec, RunOutcome, StateNode, StateNodeError,
    StructuredSchema, TypedSchema, WarEngine,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::aegis::{Aegis, RetryPolicy, RetryPredicate};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::structured::SchemaRef;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointStatus};
use paladin_llm::mock::MockLlmAdapter;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use paladin_ports::output::structured_executor_port::StructuredExecutorPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use serde::Deserialize;

/// Adapts a `PaladinExecutionService` to `PaladinPort` so it can back a
/// `WarEngine`'s single, engine-wide Paladin dispatch port -- the same thin
/// wrapper `middleware_under_engine_test.rs` builds (there is no in-tree
/// `PaladinPort` adapter for the service).
struct ServiceAsPaladinPort(Arc<PaladinExecutionService>);

#[async_trait]
impl PaladinPort for ServiceAsPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        self.0.execute(paladin, input).await
    }

    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        <PaladinExecutionService as StreamingExecutorPort>::execute_stream(&self.0, paladin, input)
            .await
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct Weather {
    #[allow(dead_code)]
    city: String,
    #[allow(dead_code)]
    temp_c: f64,
}

fn weather_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["city", "temp_c"],
        "properties": {
            "city": {"type": "string"},
            "temp_c": {"type": "number"}
        }
    })
}

fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: "system".to_string(),
        max_loops: MaxLoops::Fixed(3),
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

fn schema() -> BattlefieldSchema {
    BattlefieldSchema::new(vec![
        FieldSpec::new(
            FieldName::new("weather").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        ),
        FieldSpec::new(
            FieldName::new("city_from_downstream").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        ),
    ])
}

/// Reads `weather` (written by the upstream structured node) as a JSON
/// **object** it can index -- proving typed agent-to-agent data flow rather
/// than string passing (Test 2). Writes `weather.city` into
/// `city_from_downstream` so the test can assert on a plain scalar rather
/// than re-inspecting the object.
struct ReadWeatherObjectNode;

#[async_trait]
impl StateNode for ReadWeatherObjectNode {
    async fn run(
        &self,
        state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let weather = state
            .get_raw(&FieldName::new("weather").unwrap())
            .ok_or_else(|| StateNodeError("weather field is not yet set".to_string()))?;
        let city = weather
            .as_object()
            .ok_or_else(|| StateNodeError("weather is not a JSON object".to_string()))?
            .get("city")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StateNodeError("weather.city is not a string".to_string()))?
            .to_string();

        let mut delta = StateDelta::new();
        delta.set_raw(
            FieldName::new("city_from_downstream").unwrap(),
            serde_json::Value::String(city),
        );
        Ok(delta.into())
    }
}

/// Builds the `worker -> reader` graph: `worker` is a `NodeSpec::Paladin`
/// node with `output_schema` set to `schema` (or `None`, for Test 7's
/// unchanged-behavior case); `reader` is the `ReadWeatherObjectNode` above.
fn build_graph(output_schema: Option<SchemaRef>) -> WarGraph {
    let mut graph = WarGraph::new(schema(), EngineLimits::default());
    let mut worker_spec = NodeSpec::paladin(
        make_paladin("worker"),
        InputMapping::new("what is the weather in Oslo"),
        FieldName::new("weather").unwrap(),
    );
    if let Some(output_schema) = output_schema {
        worker_spec = worker_spec.with_output_schema(output_schema);
    }
    graph.add_node(NodeId::new("worker"), worker_spec);
    graph.add_node(
        NodeId::new("reader"),
        NodeSpec::Function(Arc::new(ReadWeatherObjectNode)),
    );
    graph.add_edge(paladin_battalion::engine::graph::EdgeSpec {
        from: NodeId::new("worker"),
        to: NodeId::new("reader"),
        condition: None,
    });
    graph.add_entry(NodeId::new("worker"));
    graph
}

fn service_and_ports(
    llm: Arc<MockLlmAdapter>,
) -> (Arc<dyn PaladinPort>, Arc<dyn StructuredExecutorPort>) {
    let service = Arc::new(PaladinExecutionService::new(
        llm,
        Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
        None,
        None,
    ));
    let port: Arc<dyn PaladinPort> = Arc::new(ServiceAsPaladinPort(service.clone()));
    let structured: Arc<dyn StructuredExecutorPort> = service;
    (port, structured)
}

// --- Test 1: `structured_node_writes_a_parsed_object_to_output_field` ---

/// A node with `output_schema: Inline(schema_for!(Weather))` and a
/// conforming mock response writes `{"city":"Oslo","temp_c":4.5}` to
/// `output_field` as a JSON **object** -- asserting the stored value's
/// TYPE, not just its rendered text.
#[tokio::test]
async fn structured_node_writes_a_parsed_object_to_output_field() {
    let llm = Arc::new(MockLlmAdapter::new().with_response(r#"{"city":"Oslo","temp_c":4.5}"#));
    let (port, structured) = service_and_ports(llm);
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, store).with_structured_executor(structured);
    let graph = build_graph(Some(SchemaRef::Inline(weather_schema())));
    let thread = ThreadId::new("structured-writes-object").unwrap();

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    let final_state = match outcome {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    };

    let weather = final_state
        .get_raw(&FieldName::new("weather").unwrap())
        .expect("weather field must be set");
    assert!(
        weather.is_object(),
        "the stored value must be a JSON object, not a string: {weather:?}"
    );
    assert_eq!(weather, &serde_json::json!({"city": "Oslo", "temp_c": 4.5}));
}

// --- Test 2: `a_downstream_node_reads_the_typed_value` ---

/// A second node reads `weather` and receives an object it can INDEX,
/// proving typed agent-to-agent data flow rather than string passing.
#[tokio::test]
async fn a_downstream_node_reads_the_typed_value() {
    let llm = Arc::new(MockLlmAdapter::new().with_response(r#"{"city":"Oslo","temp_c":4.5}"#));
    let (port, structured) = service_and_ports(llm);
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, store).with_structured_executor(structured);
    let graph = build_graph(Some(SchemaRef::Inline(weather_schema())));
    let thread = ThreadId::new("downstream-reads-typed-value").unwrap();

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    let final_state = match outcome {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    };

    assert_eq!(
        final_state.get_raw(&FieldName::new("city_from_downstream").unwrap()),
        Some(&serde_json::Value::String("Oslo".to_string())),
        "the downstream Function node must have indexed weather.city out of the OBJECT -- it \
         cannot do this if `weather` were still a string"
    );
}

// --- Test 3: `registered_schema_by_name_works_end_to_end` ---

/// The same end-to-end flow, but with `SchemaRef::Registered("weather")`
/// and `WarEngine::with_output_schema("weather", Arc::new(TypedSchema::<Weather>::new(..)))`.
#[tokio::test]
async fn registered_schema_by_name_works_end_to_end() {
    let llm = Arc::new(MockLlmAdapter::new().with_response(r#"{"city":"Oslo","temp_c":4.5}"#));
    let (port, structured) = service_and_ports(llm);
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, store)
        .with_structured_executor(structured)
        .with_output_schema(
            "weather",
            Arc::new(TypedSchema::<Weather>::new(weather_schema())) as Arc<dyn StructuredSchema>,
        );
    let graph = build_graph(Some(SchemaRef::Registered("weather".to_string())));
    let thread = ThreadId::new("registered-schema-end-to-end").unwrap();

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    let final_state = match outcome {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    };
    assert_eq!(
        final_state.get_raw(&FieldName::new("weather").unwrap()),
        Some(&serde_json::json!({"city": "Oslo", "temp_c": 4.5}))
    );
}

// --- Test 4: `repair_happens_inside_the_node` ---

/// A mock returning a non-conforming then a conforming response completes
/// the node in ONE node-execution with TWO model calls.
#[tokio::test]
async fn repair_happens_inside_the_node() {
    let llm = Arc::new(MockLlmAdapter::new().with_responses(vec![
        r#"{"city": "Oslo"}"#.to_string(),
        r#"{"city": "Oslo", "temp_c": 4.5}"#.to_string(),
    ]));
    let (port, structured) = service_and_ports(llm.clone());
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, store).with_structured_executor(structured);
    let graph = build_graph(Some(SchemaRef::Inline(weather_schema())));
    let thread = ThreadId::new("repair-happens-inside-the-node").unwrap();

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            assert_eq!(
                final_state.get_raw(&FieldName::new("weather").unwrap()),
                Some(&serde_json::json!({"city": "Oslo", "temp_c": 4.5}))
            );
        }
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    }
    assert_eq!(
        llm.call_count(),
        2,
        "one node-execution, two model calls: the first attempt plus one repair"
    );
}

// --- Test 5: `exhaustion_becomes_a_node_error_with_unknown_transience` ---

/// Two non-conforming responses produce a `NodeError` whose source is
/// `Paladin { kind: "StructuredOutputInvalid", .. }` and whose transience
/// is `Unknown`; nothing is written to `output_field`.
#[tokio::test]
async fn exhaustion_becomes_a_node_error_with_unknown_transience() {
    let llm = Arc::new(MockLlmAdapter::new().with_responses(vec![
        r#"{"city": "Oslo"}"#.to_string(),
        r#"{"still": "missing temp_c"}"#.to_string(),
    ]));
    let (port, structured) = service_and_ports(llm);
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, Arc::clone(&store)).with_structured_executor(structured);
    let mut graph = build_graph(Some(SchemaRef::Inline(weather_schema())));
    // --- D-08/D-09: the structured NodeError only travels with the FINAL
    // failed attempt when the node has a RESOLVED Aegis -- a no-Aegis
    // node's failure stays on the byte-identical pre-Phase-25 generic path
    // (`EngineError::Node`, no `node_error()`). An empty `Aegis::default()`
    // (no retry policy) is enough to opt in to the structured path while
    // still producing exactly one attempt.
    graph.set_aegis(NodeId::new("worker"), Aegis::default());
    let thread = ThreadId::new("exhaustion-unknown-transience").unwrap();

    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("start itself must not error -- the FAILURE is the RunOutcome");

    let (error, waypoint_id) = match outcome {
        RunOutcome::Failed { error, waypoint } => (
            error,
            waypoint.expect("a Failed waypoint must be persisted"),
        ),
        other => panic!("expected RunOutcome::Failed, got {other:?}"),
    };

    let node_error = error
        .node_error()
        .expect("a NodeSpec::Paladin failure always carries a structured NodeError");
    assert_eq!(
        node_error.transience,
        paladin_core::platform::container::transience::Transience::Unknown
    );
    match &node_error.source {
        paladin_core::platform::container::node_error::NodeErrorSource::Paladin {
            kind, ..
        } => {
            assert_eq!(kind, "StructuredOutputInvalid");
        }
        other => panic!("expected NodeErrorSource::Paladin, got {other:?}"),
    }

    // Nothing was written to `output_field`: the persisted Failed waypoint's
    // own battlefield snapshot has no `weather` value.
    let waypoint = store
        .get(&thread, &waypoint_id)
        .await
        .unwrap()
        .expect("the Failed waypoint must be retrievable");
    assert!(matches!(waypoint.status, WaypointStatus::Failed { .. }));
    assert_eq!(
        waypoint
            .battlefield
            .get_raw(&FieldName::new("weather").unwrap()),
        None,
        "a node with output_schema must never fall back to writing a raw string, or anything \
         else, on exhaustion"
    );
}

// --- Test 6: `a_transient_and_unknown_aegis_may_still_retry_the_node` ---

/// With an Aegis whose predicate is `TransientAndUnknown`, the node is
/// retried after the exhaustion -- the documented consequence of choosing
/// `Unknown` (D-29, Phase 25 D-05).
#[tokio::test]
async fn a_transient_and_unknown_aegis_may_still_retry_the_node() {
    // Node attempt 1: two non-conforming responses -> exhaustion.
    // Node attempt 2 (the Aegis retry): one conforming response -> success.
    let llm = Arc::new(MockLlmAdapter::new().with_responses(vec![
        r#"{"city": "Oslo"}"#.to_string(),
        r#"{"still": "missing temp_c"}"#.to_string(),
        r#"{"city": "Oslo", "temp_c": 4.5}"#.to_string(),
    ]));
    let (port, structured) = service_and_ports(llm.clone());
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, store).with_structured_executor(structured);
    let mut graph = build_graph(Some(SchemaRef::Inline(weather_schema())));
    graph.set_aegis(
        NodeId::new("worker"),
        Aegis {
            retry: Some(RetryPolicy {
                max_attempts: 2,
                initial_interval: Duration::from_millis(1),
                backoff_factor: 1.0,
                max_interval: Duration::from_millis(1),
                jitter: false,
                retry_on: RetryPredicate::TransientAndUnknown,
            }),
            ..Aegis::default()
        },
    );
    let thread = ThreadId::new("transient-and-unknown-retries").unwrap();

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed after the Aegis retry");

    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            assert_eq!(
                final_state.get_raw(&FieldName::new("weather").unwrap()),
                Some(&serde_json::json!({"city": "Oslo", "temp_c": 4.5}))
            );
        }
        other => panic!(
            "expected RunOutcome::Completed after the TransientAndUnknown Aegis retried the \
             whole node, got {other:?}"
        ),
    }
    assert_eq!(
        llm.call_count(),
        3,
        "node attempt 1's exhausted repair loop (2 calls) plus node attempt 2's single \
         successful call (1 call) = 3 total model calls"
    );
}

// --- Test 7: `a_node_without_output_schema_is_unchanged` ---

/// A plain Paladin node (no `output_schema`) behaves EXACTLY as before --
/// same dispatch (through the plain `PaladinPort`), same string written to
/// `output_field`.
#[tokio::test]
async fn a_node_without_output_schema_is_unchanged() {
    let llm = Arc::new(MockLlmAdapter::new().with_response("just a plain string, not JSON"));
    let (port, structured) = service_and_ports(llm);
    let store = Arc::new(InMemoryWaypointStore::new());
    // A structured executor IS wired (so an engine hosting both kinds of
    // node works), but this graph's only Paladin node has no
    // `output_schema` -- it must ignore the structured executor entirely.
    let engine = WarEngine::new(port, store).with_structured_executor(structured);
    // A solo `worker` node (no `reader`, unlike `build_graph`'s shared
    // fixture): a plain string is not a JSON object, and this test's whole
    // point is that a schema-less node is UNCHANGED -- it must not need a
    // downstream consumer that tolerates one shape or the other.
    let mut graph = WarGraph::new(schema(), EngineLimits::default());
    graph.add_node(
        NodeId::new("worker"),
        NodeSpec::paladin(
            make_paladin("worker"),
            InputMapping::new("what is the weather in Oslo"),
            FieldName::new("weather").unwrap(),
        ),
    );
    graph.add_entry(NodeId::new("worker"));
    let thread = ThreadId::new("no-output-schema-unchanged").unwrap();

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    let final_state = match outcome {
        RunOutcome::Completed { final_state, .. } => final_state,
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    };

    assert_eq!(
        final_state.get_raw(&FieldName::new("weather").unwrap()),
        Some(&serde_json::Value::String(
            "just a plain string, not JSON".to_string()
        )),
        "a node with no output_schema must write the raw string verbatim, exactly as before \
         this phase"
    );
}

// --- Test 8: `structured_directive_and_output_schema_share_extract_json` ---

/// A source-level assertion that the engine has ONE envelope-extraction
/// implementation (CF-FR-06, D-26): `DirectiveParser::StructuredDirective`
/// calls the SAME `paladin_core::platform::container::structured::extract_json`
/// the structured-output repair loop uses, rather than maintaining a second,
/// private `extract_envelope` copy.
#[test]
fn structured_directive_and_output_schema_share_extract_json() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/crates/paladin-battalion/src/engine/directive_parser.rs"
    ));
    assert!(
        !source.contains("fn extract_envelope"),
        "directive_parser.rs must not carry its own private extract_envelope copy -- it must \
         call the shared structured::extract_json instead (D-26, CF-FR-06)"
    );
    assert!(
        source.contains("extract_json("),
        "directive_parser.rs must call the shared structured::extract_json function"
    );
}
