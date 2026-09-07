//! Phase 26 Plan 01 (RT-01, D-05, RT-FR-03): a real
//! [`PaladinExecutionService`] carrying an `ExecutionMiddleware` chain,
//! dispatched as a `NodeSpec::Paladin` node under a `WarEngine`, applies its
//! chain unchanged -- no engine-side middleware registry exists or is
//! needed.
//!
//! Also proves the two-layer nesting order from
//! `crates/paladin-battalion/src/engine/hooks.rs`'s `NodeInterceptor` doc:
//! the interceptor brackets the WHOLE node (once per Aegis attempt), the
//! `ExecutionMiddleware` chain sits INSIDE it (once per model call).
//!
//! The Phase 23 deferred idea "`NodeInterceptor` visibility of `NextStep`"
//! stays deferred and is not what this test exercises.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use paladin::application::services::paladin::middleware::{
    ExecutionMiddleware, LlmResponseView, MiddlewareFlow, ModelCallContext,
};
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_battalion::engine::{
    EngineLimits, InputMapping, InterceptDecision, NodeContext, NodeInterceptor, NodeSpec,
    RunOutcome, WarEngine, WarGraph,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::heartbeat::HeartbeatHandle;
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_llm::mock::MockLlmAdapter;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// Adapts a `PaladinExecutionService` to `PaladinPort` so it can back a
/// `WarEngine`'s single, engine-wide Paladin dispatch port -- there is no
/// in-tree adapter for this (the facade's own `PaladinExecutionService`
/// implements `PaladinExecutorPort`/`StreamingExecutorPort`, not
/// `PaladinPort` directly), so this test builds the thin wrapper D-05
/// describes: "a `PaladinExecutionService` carrying middleware applies that
/// chain when the engine dispatches it through `execute_observed`".
struct ServiceAsPaladinPort(Arc<PaladinExecutionService>);

#[async_trait]
impl PaladinPort for ServiceAsPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        self.0.execute(paladin, input).await
    }

    async fn execute_observed(
        &self,
        paladin: &Paladin,
        input: &str,
        heartbeat: &HeartbeatHandle,
    ) -> Result<PaladinResult, PaladinError> {
        self.0.execute_observed(paladin, input, heartbeat).await
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

/// Records `"middleware.before_model"` / `"middleware.after_model"` into a
/// shared, global log (shared with a `NodeInterceptor` in some tests) so the
/// interleaving between the two layers can be asserted.
struct RecordingMiddleware {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ExecutionMiddleware for RecordingMiddleware {
    async fn before_model(
        &self,
        _cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        self.log
            .lock()
            .unwrap()
            .push("middleware.before_model".to_string());
        Ok(MiddlewareFlow::Continue)
    }

    async fn after_model(
        &self,
        _cx: &mut ModelCallContext<'_>,
        _resp: &mut LlmResponseView,
    ) -> Result<MiddlewareFlow, PaladinError> {
        self.log
            .lock()
            .unwrap()
            .push("middleware.after_model".to_string());
        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "recording"
    }
}

/// Records `"interceptor.before"` / `"interceptor.after"` into the same
/// shared log as [`RecordingMiddleware`].
struct RecordingInterceptor {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl NodeInterceptor for RecordingInterceptor {
    async fn before(&self, _ctx: &NodeContext, _state: &Battlefield) -> InterceptDecision {
        self.log
            .lock()
            .unwrap()
            .push("interceptor.before".to_string());
        InterceptDecision::Proceed
    }

    async fn after(&self, _ctx: &NodeContext, _delta: &mut StateDelta) {
        self.log
            .lock()
            .unwrap()
            .push("interceptor.after".to_string());
    }
}

fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        name: name.to_string(),
        system_prompt: "system".to_string(),
        max_loops: MaxLoops::Fixed(1),
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

fn one_field_schema() -> BattlefieldSchema {
    BattlefieldSchema::new(vec![FieldSpec::new(
        FieldName::new("output").expect("non-empty field name"),
        DispatchRule::LastWrite,
        None,
        false,
    )])
}

fn build_graph() -> WarGraph {
    let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
    let node_id = NodeId::new("solo");
    graph.add_node(
        node_id.clone(),
        NodeSpec::paladin(
            make_paladin("solo"),
            InputMapping::new("go"),
            FieldName::new("output").expect("non-empty field name"),
        ),
    );
    graph.add_entry(node_id);
    graph
}

fn make_service(
    llm: Arc<MockLlmAdapter>,
    middleware: Arc<dyn ExecutionMiddleware>,
) -> Arc<PaladinExecutionService> {
    Arc::new(
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        )
        .with_middleware(middleware),
    )
}

/// A `NodeSpec::Paladin` node running under a `WarEngine` applies its
/// service's `ExecutionMiddleware` chain -- no engine change is needed
/// (D-05, RT-FR-03).
#[tokio::test]
async fn paladin_node_under_engine_runs_the_service_middleware_chain() {
    let llm = Arc::new(MockLlmAdapter::new().with_response("done"));
    let log = Arc::new(Mutex::new(Vec::new()));
    let service = make_service(
        llm,
        Arc::new(RecordingMiddleware { log: log.clone() }) as Arc<dyn ExecutionMiddleware>,
    );
    let port: Arc<dyn PaladinPort> = Arc::new(ServiceAsPaladinPort(service));
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, store);
    let graph = build_graph();
    let thread = ThreadId::new("mw-under-engine").expect("valid thread id");

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    match outcome {
        RunOutcome::Completed { .. } => {}
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    }

    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "middleware.before_model".to_string(),
            "middleware.after_model".to_string(),
        ]
    );
}

/// `NodeInterceptor` and `ExecutionMiddleware` are independent layers: the
/// interceptor brackets the WHOLE node (once per Aegis attempt), and the
/// middleware chain sits INSIDE it (once per model call) -- D-05's
/// two-layer contract, observed as one global interleaving order.
#[tokio::test]
async fn node_interceptor_and_execution_middleware_are_independent_layers() {
    let llm = Arc::new(MockLlmAdapter::new().with_response("done"));
    let log = Arc::new(Mutex::new(Vec::new()));
    let service = make_service(
        llm,
        Arc::new(RecordingMiddleware { log: log.clone() }) as Arc<dyn ExecutionMiddleware>,
    );
    let port: Arc<dyn PaladinPort> = Arc::new(ServiceAsPaladinPort(service));
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine =
        WarEngine::new(port, store).with_interceptors(vec![Arc::new(RecordingInterceptor {
            log: log.clone(),
        }) as Arc<dyn NodeInterceptor>]);
    let graph = build_graph();
    let thread = ThreadId::new("mw-and-interceptor").expect("valid thread id");

    engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "interceptor.before".to_string(),
            "middleware.before_model".to_string(),
            "middleware.after_model".to_string(),
            "interceptor.after".to_string(),
        ],
        "the interceptor must bracket the whole node; the middleware chain runs inside it"
    );
}

/// D-05: `WarEngine` exposes no middleware registration API of its own.
/// This test constructs the engine using ONLY pre-existing builders
/// (`WarEngine::new`, `with_interceptors`) -- there is no
/// `with_middleware`/`ExecutionMiddleware`-shaped builder on `WarEngine` to
/// call, and there never needs to be, since middleware lives on the
/// `PaladinExecutionService` a `PaladinPort` wraps.
#[tokio::test]
async fn engine_needs_no_middleware_registry() {
    let llm = Arc::new(MockLlmAdapter::new().with_response("done"));
    let service = Arc::new(PaladinExecutionService::new(
        llm,
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
        None,
        None,
    ));
    let port: Arc<dyn PaladinPort> = Arc::new(ServiceAsPaladinPort(service));
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(port, store);
    let graph = build_graph();
    let thread = ThreadId::new("no-middleware-registry").expect("valid thread id");

    let outcome = engine
        .start(&graph, thread, StateDelta::new())
        .await
        .expect("run should succeed");

    match outcome {
        RunOutcome::Completed { .. } => {}
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    }
}
