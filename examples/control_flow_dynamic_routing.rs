// examples/control_flow_dynamic_routing.rs
//
// Control Flow: Custom Edges, Nested Subgraphs, LLM Routing, Muster Caps
// (EX-67, EX-68, EX-69, EX-70)
//
// This example demonstrates the WarEngine's dynamic-routing surface. It shows
// how to:
// 1. Register a custom `EdgeCondition::Custom` evaluator, and see the
//    fail-closed outcome the engine produces when that name is NOT
//    registered -- the run never silently defaults the edge to always-true.
// 2. Nest a child `WarGraph` inside a parent graph as a `NodeSpec::Battalion`
//    node, and read the mapped result back on both sides of the boundary.
// 3. Drive an edge decision from a live model's answer through
//    `LlmDecisionEvaluator`, using `MockLlmAdapter` so the routing is
//    reproducible offline.
// 4. Override the Muster fan-out cap from the environment
//    (APP_ENGINE_MAX_MUSTER_TASKS) and see it enforced against a running
//    engine.
//
// This example is fully offline: every LLM call goes through `MockLlmAdapter`
// and needs no LLM provider API key or external service.
//
// To run this example:
// ```bash
// cargo run --example control_flow_dynamic_routing
// ```

use std::env;
use std::sync::Arc;

use async_trait::async_trait;

use paladin::MockLlmAdapter;
use paladin::config::engine::EngineConfig;
use paladin::config::env_utils::EnvOverridable;
use paladin_battalion::edge_evaluator::{EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorError};
use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, StateMap, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_battalion::engine::{RunOutcome, WarEngine};
use paladin_battalion::llm_decision::LlmDecisionEvaluator;
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, MusterTask, NextStep};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// `WarEngine::new` requires a `PaladinPort`; every graph below is built
/// entirely from `Function`/`Battalion` nodes, so this is never actually
/// invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("every graph in this program runs Function/Battalion nodes only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("every graph in this program runs Function/Battalion nodes only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A `StateNode` that writes nothing and routes via its static outgoing
/// edges -- used as a router/entry node whose only job is to fan out through
/// conditional edges.
struct NoOp;

#[async_trait]
impl StateNode for NoOp {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Ok(StateDelta::new().into())
    }
}

/// A `StateNode` that writes a fixed string `value` to `field` and routes
/// via its static outgoing edges -- the smallest node that leaves visible
/// evidence of having run.
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

/// A hand-registered `EdgeConditionEvaluator` for `EdgeCondition::Custom`
/// (EX-67, Part 1): always opens the gate once registered. Deliberately not
/// `LlmDecisionEvaluator` -- Part 1 is about the registration mechanism
/// itself, not the LLM-driven variant Part 3 demonstrates separately.
struct AlwaysOpenGate;

#[async_trait]
impl EdgeConditionEvaluator for AlwaysOpenGate {
    async fn evaluate(
        &self,
        _output: &str,
        _ctx: &EdgeContext<'_>,
    ) -> Result<bool, EdgeEvaluatorError> {
        Ok(true)
    }
}

/// A `StateNode` that musters `task_count` tasks onto `worker` in a single
/// `NextStep::Muster` directive (EX-70, Part 4) -- reused verbatim from
/// `src/config/engine.rs`'s own
/// `app_engine_max_muster_tasks_reaches_a_running_engines_limit` test, the
/// smallest shape that demonstrates `EngineLimits::max_muster_tasks` being
/// enforced against a running engine.
struct MusteringPlanner {
    worker: NodeId,
    task_count: usize,
}

#[async_trait]
impl StateNode for MusteringPlanner {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let tasks = (0..self.task_count)
            .map(|i| MusterTask {
                worker: self.worker.clone(),
                payload: serde_json::json!(i),
                task_key: format!("task-{i}"),
            })
            .collect();
        Ok(Directive {
            delta: StateDelta::new(),
            next: NextStep::Muster(tasks),
        })
    }
}

/// The worker template `MusteringPlanner`'s tasks dispatch to -- never
/// actually reached once the cap rejects the request.
struct NoopWorker;

#[async_trait]
impl StateNode for NoopWorker {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Ok(StateDelta::new().into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Control Flow: Custom Edges, Nested Subgraphs, LLM Routing, Muster Caps\n");

    // ------------------------------------------------------------------------------
    // Part 1 -- a custom edge condition fails closed when unregistered (EX-67).
    // ------------------------------------------------------------------------------
    println!("1. Custom edge condition: fail closed when unregistered, then registered (EX-67)\n");

    let gate_status = FieldName::new("gate_status")?;
    let gate_schema = BattlefieldSchema::new(vec![FieldSpec::new(
        gate_status.clone(),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut gate_graph = WarGraph::new(gate_schema, EngineLimits::default());
    let gatekeeper = NodeId::new("gatekeeper");
    let opened = NodeId::new("opened");
    gate_graph.add_node(gatekeeper.clone(), NodeSpec::Function(Arc::new(NoOp)));
    gate_graph.add_node(
        opened.clone(),
        NodeSpec::Function(Arc::new(WriteField {
            field: gate_status.clone(),
            value: "opened".to_string(),
        })),
    );
    gate_graph.add_edge(EdgeSpec {
        from: gatekeeper.clone(),
        to: opened.clone(),
        condition: Some(EdgeCondition::Custom("gate_check".to_string())),
    });
    gate_graph.add_entry(gatekeeper);

    let gate_store = Arc::new(InMemoryWaypointStore::new());
    let gate_thread = ThreadId::new("control-flow-edge-condition")?;

    let unregistered_engine = WarEngine::new(Arc::new(UnusedPaladinPort), gate_store.clone());
    match unregistered_engine
        .start(&gate_graph, gate_thread.clone(), StateDelta::new())
        .await
    {
        Err(e) => println!("   Unregistered \"gate_check\": engine.start failed closed -> {e}"),
        Ok(outcome) => println!(
            "   UNEXPECTED: engine.start succeeded without a registered evaluator: {outcome:?}"
        ),
    }

    let registered_engine = WarEngine::new(Arc::new(UnusedPaladinPort), gate_store)
        .with_edge_evaluator("gate_check", Arc::new(AlwaysOpenGate));
    let outcome = registered_engine
        .start(&gate_graph, gate_thread, StateDelta::new())
        .await?;
    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let status: Option<String> = final_state.get(&gate_status)?;
            println!("   Registered \"gate_check\": edge taken, gate_status = {status:?}\n");
        }
        other => println!("   Unexpected outcome: {other:?}\n"),
    }

    // ------------------------------------------------------------------------------
    // Part 2 -- nested subgraph composition (EX-68).
    // ------------------------------------------------------------------------------
    println!("2. Nested subgraph composition (EX-68)\n");

    let inner_result = FieldName::new("inner_result")?;
    let inner_schema = BattlefieldSchema::new(vec![FieldSpec::new(
        inner_result.clone(),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut inner_graph = WarGraph::new(inner_schema, EngineLimits::default());
    let inner_worker = NodeId::new("inner_worker");
    inner_graph.add_node(
        inner_worker.clone(),
        NodeSpec::Function(Arc::new(WriteField {
            field: inner_result.clone(),
            value: "child-completed".to_string(),
        })),
    );
    inner_graph.add_entry(inner_worker);

    let outer_result = FieldName::new("outer_result")?;
    let outer_schema = BattlefieldSchema::new(vec![FieldSpec::new(
        outer_result.clone(),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut outer_graph = WarGraph::new(outer_schema, EngineLimits::default());
    let outer_wrapper = NodeId::new("outer_wrapper");
    let state_map = StateMap::new().with_output(inner_result.clone(), outer_result.clone());
    outer_graph.add_node(
        outer_wrapper.clone(),
        NodeSpec::battalion(Arc::new(inner_graph), state_map),
    );
    outer_graph.add_entry(outer_wrapper.clone());

    let nest_store = Arc::new(InMemoryWaypointStore::new());
    let nest_engine = WarEngine::new(Arc::new(UnusedPaladinPort), nest_store.clone());
    let outer_thread = ThreadId::new("control-flow-nested-subgraph")?;
    let outcome = nest_engine
        .start(&outer_graph, outer_thread.clone(), StateDelta::new())
        .await?;
    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let mapped: Option<String> = final_state.get(&outer_result)?;
            println!("   outer.outer_result (mapped from the child via StateMap) = {mapped:?}");
        }
        other => println!("   Unexpected outer outcome: {other:?}"),
    }
    // The child ran under a durably-addressable derived thread id, so its
    // own final checkpoint is independently readable from the SAME store --
    // this is the "inner node result" half of the nesting, distinct from
    // the value StateMap already propagated to the parent above.
    let inner_thread = ThreadId::child(&outer_thread, &outer_wrapper)?;
    if let Some(child_waypoint) = nest_store.latest(&inner_thread).await? {
        let child_value: Option<String> = child_waypoint.battlefield.get(&inner_result)?;
        println!("   inner.inner_result (the child's own final checkpoint) = {child_value:?}\n");
    } else {
        println!("   (no child checkpoint found under the derived thread id)\n");
    }

    // ------------------------------------------------------------------------------
    // Part 3 -- LLM-driven dynamic routing (EX-69).
    // ------------------------------------------------------------------------------
    println!("3. LLM-driven dynamic routing (EX-69)\n");

    let handled_by = FieldName::new("handled_by")?;
    let route_schema = BattlefieldSchema::new(vec![FieldSpec::new(
        handled_by.clone(),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut route_graph = WarGraph::new(route_schema, EngineLimits::default());
    let router = NodeId::new("router");
    let escalate_handler = NodeId::new("escalate_handler");
    let archive_handler = NodeId::new("archive_handler");
    route_graph.add_node(router.clone(), NodeSpec::Function(Arc::new(NoOp)));
    route_graph.add_node(
        escalate_handler.clone(),
        NodeSpec::Function(Arc::new(WriteField {
            field: handled_by.clone(),
            value: "escalate_handler".to_string(),
        })),
    );
    route_graph.add_node(
        archive_handler.clone(),
        NodeSpec::Function(Arc::new(WriteField {
            field: handled_by.clone(),
            value: "archive_handler".to_string(),
        })),
    );
    route_graph.add_edge(EdgeSpec {
        from: router.clone(),
        to: escalate_handler.clone(),
        condition: Some(EdgeCondition::Custom("route_urgency".to_string())),
    });
    route_graph.add_edge(EdgeSpec {
        from: router.clone(),
        to: archive_handler.clone(),
        condition: Some(EdgeCondition::Custom("route_urgency".to_string())),
    });
    route_graph.add_entry(router);

    let decision_evaluator: Arc<dyn EdgeConditionEvaluator> = Arc::new(LlmDecisionEvaluator::new(
        "route_urgency",
        Arc::new(MockLlmAdapter::new().with_response("escalate")) as Arc<dyn LlmPort>,
        "mock-routing-model",
        "Is this urgent? Reply escalate or archive.",
        vec![
            ("escalate".to_string(), escalate_handler.clone()),
            ("archive".to_string(), archive_handler.clone()),
        ],
    ));

    let route_store = Arc::new(InMemoryWaypointStore::new());
    let route_engine = WarEngine::new(Arc::new(UnusedPaladinPort), route_store)
        .with_edge_evaluator("route_urgency", decision_evaluator);
    let route_thread = ThreadId::new("control-flow-llm-routing")?;
    let outcome = route_engine
        .start(&route_graph, route_thread, StateDelta::new())
        .await?;
    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let branch: Option<String> = final_state.get(&handled_by)?;
            println!("   MockLlmAdapter answered \"escalate\" -> handled_by = {branch:?}");
        }
        other => println!("   Unexpected outcome: {other:?}"),
    }
    println!(
        "   (The Commander-level equivalent of this same capability is\n    \
         paladin_battalion::commander::StrategySelection::Semantic -- an LLM picks the\n    \
         Battalion STRATEGY itself, rather than a WarGraph edge target.)\n"
    );

    // ------------------------------------------------------------------------------
    // Part 4 -- cap the Muster fan-out from the environment (EX-70).
    // ------------------------------------------------------------------------------
    println!("4. Cap the Muster fan-out from the environment (EX-70)\n");

    let before_override = EngineConfig::default().max_muster_tasks;
    // SAFETY: this program is single-threaded at this point in `main` -- no
    // other task reads or writes this variable concurrently with the
    // set/apply/remove sequence below.
    unsafe { env::set_var("APP_ENGINE_MAX_MUSTER_TASKS", "2") };
    let mut overridden = EngineConfig::default();
    overridden.apply_env_overrides();
    println!("   before APP_ENGINE_MAX_MUSTER_TASKS: max_muster_tasks = {before_override}");
    println!(
        "   after  APP_ENGINE_MAX_MUSTER_TASKS: max_muster_tasks = {}",
        overridden.max_muster_tasks
    );
    unsafe { env::remove_var("APP_ENGINE_MAX_MUSTER_TASKS") };
    overridden.validate()?;

    let limits: EngineLimits = overridden.into();
    let mut muster_graph = WarGraph::new(BattlefieldSchema::new(vec![]), limits);
    let planner = NodeId::new("planner");
    let worker = NodeId::new("worker");
    muster_graph.add_node(
        planner.clone(),
        NodeSpec::Function(Arc::new(MusteringPlanner {
            worker: worker.clone(),
            task_count: 3,
        })),
    );
    muster_graph.add_worker_template(worker, NodeSpec::Function(Arc::new(NoopWorker)));
    muster_graph.add_entry(planner);

    let muster_store = Arc::new(InMemoryWaypointStore::new());
    let muster_engine = WarEngine::new(Arc::new(UnusedPaladinPort), muster_store);
    let muster_thread = ThreadId::new("control-flow-muster-cap")?;
    let outcome = muster_engine
        .start(&muster_graph, muster_thread, StateDelta::new())
        .await?;
    match outcome {
        RunOutcome::Failed { error, .. } => {
            println!("   Muster cap enforced against a running engine: {error}\n");
        }
        other => println!("   UNEXPECTED: run did not hit the cap: {other:?}\n"),
    }

    println!("Done -- fully offline, no provider API key was read.");
    Ok(())
}
