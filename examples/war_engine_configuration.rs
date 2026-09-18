// examples/war_engine_configuration.rs
//
// WarEngine Configuration & Checkpoints (EX-62, EX-63, EX-64, EX-65, EX-66, EX-80)
//
// This example demonstrates the WarEngine's configuration surface and its
// checkpoint (Waypoint) persistence. It shows how to:
// 1. Inject a Waypoint backend -- an explicitly constructed InMemoryWaypointStore,
//    the same WaypointPort trait a SQLite or Postgres backend would satisfy.
// 2. Configure the engine through `EngineConfig`, naming every bounded-iteration
//    and durability field: max_supersteps, max_node_visits, run_timeout_secs,
//    waypoint_durability and max_muster_tasks.
// 3. Override the superstep cap from the environment (APP_ENGINE_MAX_SUPERSTEPS),
//    printing the before and after values so the override is visibly the thing
//    that changed it.
// 4. Run a small cyclic WarGraph to completion and read its checkpoint history
//    back through the WaypointPort, printing each checkpoint's superstep and
//    the node keys that produced it.
// 5. Prune that checkpoint history with the WaypointRetentionService, printing
//    how many rows it removed and how many remain.
// 6. Print the graph fingerprint version and explain what bumping it
//    invalidates.
//
// This example is fully offline: it uses only in-memory, in-process state, reads
// no LLM provider API key from the environment, and needs no external service.
//
// To run this example:
// ```bash
// cargo run --example war_engine_configuration
// ```

use std::env;
use std::sync::Arc;

use async_trait::async_trait;

use paladin::application::services::waypoint_retention::WaypointRetentionService;
use paladin::config::WaypointRetentionConfig;
use paladin::config::engine::EngineConfig;
use paladin::config::env_utils::EnvOverridable;
use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_battalion::engine::{RunOutcome, WarEngine};
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{GRAPH_FINGERPRINT_VERSION, NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// `WarEngine::new` requires a `PaladinPort`; this program's graph is built
/// entirely from `Function` nodes, so this is never actually invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this program's WarGraph runs Function nodes only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this program's WarGraph runs Function nodes only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A pure `StateNode` that increments a `count` field each visit and
/// self-loops -- via `NextStep::Edges` and a `Contains("looping")` condition
/// on its own outgoing edge -- until `count` reaches `target`, then writes
/// `status = "done"` and falls out of the loop. The smallest shape that
/// demonstrates cyclic superstep execution and produces more than one
/// checkpoint for Part 4 to read back.
struct LoopUntil {
    target: u64,
}

#[async_trait]
impl StateNode for LoopUntil {
    async fn run(
        &self,
        state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let count_field = FieldName::new("count").map_err(|e| StateNodeError(e.to_string()))?;
        let status_field = FieldName::new("status").map_err(|e| StateNodeError(e.to_string()))?;
        let current: u64 = state
            .get(&count_field)
            .map_err(|e| StateNodeError(e.to_string()))?
            .unwrap_or(0);
        let next = current + 1;

        let mut delta = StateDelta::new();
        delta
            .set(count_field, next)
            .map_err(|e| StateNodeError(e.to_string()))?;
        delta
            .set(
                status_field,
                if next >= self.target {
                    "done"
                } else {
                    "looping"
                },
            )
            .map_err(|e| StateNodeError(e.to_string()))?;

        Ok(Directive {
            delta,
            next: NextStep::Edges,
        })
    }
}

/// Build the small cyclic `WarGraph` Parts 4-6 run: one `Function` node
/// self-loops over a `(count, status)` `Battlefield` until `status` reads
/// `"done"`.
fn build_graph(limits: EngineLimits) -> Result<WarGraph, Box<dyn std::error::Error>> {
    let count = FieldName::new("count")?;
    let status = FieldName::new("status")?;
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(
            count,
            DispatchRule::LastWrite,
            Some(serde_json::json!(0)),
            false,
        ),
        FieldSpec::new(status, DispatchRule::LastWrite, None, false),
    ]);

    let mut graph = WarGraph::new(schema, limits);
    let looper = NodeId::new("looper");
    graph.add_node(
        looper.clone(),
        NodeSpec::Function(Arc::new(LoopUntil { target: 3 })),
    );
    graph.add_edge(EdgeSpec {
        from: looper.clone(),
        to: looper.clone(),
        condition: Some(EdgeCondition::Contains("looping".to_string())),
    });
    graph.add_entry(looper);

    Ok(graph)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("WarEngine Configuration & Checkpoints\n");

    // ------------------------------------------------------------------------------
    // Part 1 -- inject a Waypoint backend (EX-62).
    // ------------------------------------------------------------------------------
    println!("1. Inject a Waypoint backend (EX-62)\n");

    let store = Arc::new(InMemoryWaypointStore::new());
    println!("   Injected InMemoryWaypointStore as this run's WaypointPort implementor.");
    println!(
        "   A SqliteWaypointStore or PostgresWaypointStore backend satisfies the exact same\n    \
         WaypointPort trait -- this program only swaps the injection point, never the\n    \
         WarEngine's own logic.\n"
    );

    // ------------------------------------------------------------------------------
    // Part 2 -- configure the engine (EX-64).
    // ------------------------------------------------------------------------------
    println!("2. Configure the WarEngine (EX-64)\n");

    let config = EngineConfig {
        max_supersteps: 20,
        max_node_visits: 10,
        run_timeout_secs: Some(60),
        max_muster_tasks: 50,
        ..EngineConfig::default()
    };
    config.validate()?;
    println!("   max_supersteps      = {}", config.max_supersteps);
    println!("   max_node_visits     = {}", config.max_node_visits);
    println!("   run_timeout_secs    = {:?}", config.run_timeout_secs);
    println!("   waypoint_durability = {:?}", config.waypoint_durability);
    println!("   max_muster_tasks    = {}\n", config.max_muster_tasks);

    // ------------------------------------------------------------------------------
    // Part 3 -- override the superstep cap from the environment (EX-65).
    // ------------------------------------------------------------------------------
    println!("3. Override the superstep cap from the environment (EX-65)\n");

    let before_override = config.max_supersteps;
    // SAFETY: this program is single-threaded at this point in `main` -- no
    // other task reads or writes this variable concurrently with the
    // set/apply/remove sequence below.
    unsafe { env::set_var("APP_ENGINE_MAX_SUPERSTEPS", "5") };
    let mut overridden = EngineConfig {
        max_supersteps: before_override,
        ..EngineConfig::default()
    };
    overridden.apply_env_overrides();
    println!("   before APP_ENGINE_MAX_SUPERSTEPS: max_supersteps = {before_override}");
    println!(
        "   after  APP_ENGINE_MAX_SUPERSTEPS: max_supersteps = {}\n",
        overridden.max_supersteps
    );
    unsafe { env::remove_var("APP_ENGINE_MAX_SUPERSTEPS") };

    // ------------------------------------------------------------------------------
    // Part 4 -- run a small graph and inspect the checkpoints (EX-63).
    // ------------------------------------------------------------------------------
    println!("4. Run the graph and inspect the checkpoints (EX-63)\n");

    let limits: EngineLimits = config.clone().into();
    let graph = build_graph(limits)?;
    let engine = WarEngine::new(Arc::new(UnusedPaladinPort), store.clone())
        .with_durability(config.waypoint_durability);
    let thread = ThreadId::new("war-engine-configuration")?;
    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await?;
    match outcome {
        RunOutcome::Completed { waypoint, .. } => {
            println!("   Run completed; final waypoint = {waypoint:?}");
        }
        other => println!("   Unexpected outcome: {other:?}"),
    }

    let history = store.history(&thread, None, None).await?;
    println!(
        "   Checkpoint history ({} waypoint(s), newest first):",
        history.len()
    );
    for summary in &history {
        if let Some(full) = store.get(&thread, &summary.waypoint_id).await? {
            let node_keys: Vec<&str> = full.completed.iter().map(|r| r.node_id.as_str()).collect();
            println!("     superstep {} -> nodes {:?}", full.superstep, node_keys);
        }
    }
    println!();

    // ------------------------------------------------------------------------------
    // Part 5 -- prune with the retention service (EX-66).
    // ------------------------------------------------------------------------------
    println!("5. Prune with the retention service (EX-66)\n");

    let retention_config = WaypointRetentionConfig {
        enabled: true,
        max_age_days: None,
        max_waypoints_per_thread: Some(1),
    };
    retention_config.validate()?;
    let retention_service =
        WaypointRetentionService::new(store.clone() as Arc<dyn WaypointPort>, retention_config);
    let report = retention_service.prune().await?;
    let remaining = store.history(&thread, None, None).await?.len();
    println!(
        "   Pruned {} waypoint(s) for thread {:?}; {} remain (the thread's latest waypoint is\n    \
         always protected, regardless of the configured bounds).\n",
        report.total_removed(),
        thread,
        remaining
    );

    // ------------------------------------------------------------------------------
    // Part 6 -- print and explain the graph fingerprint version (EX-80).
    // ------------------------------------------------------------------------------
    println!("6. Graph fingerprint version (EX-80)\n");

    println!("   GRAPH_FINGERPRINT_VERSION = {GRAPH_FINGERPRINT_VERSION}");
    println!("   graph.fingerprint()       = {}", graph.fingerprint());
    println!(
        "   Bumping GRAPH_FINGERPRINT_VERSION invalidates every previously stored Waypoint:\n    \
         a resumed run whose stored fingerprint no longer matches the running build's\n    \
         fingerprint is rejected with EngineError::GraphMismatch rather than silently\n    \
         resumed under a changed graph.\n"
    );

    println!("Done -- fully offline, no provider API key was read.");
    Ok(())
}
