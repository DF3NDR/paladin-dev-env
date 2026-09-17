//! Examples for `docs/src/user-guides/superstep-engine.md` (Phase 35, MB-30).
//!
//! Every `// ANCHOR:` region below is pulled into the WarEngine
//! superstep-engine guide via mdBook `{{#include}}`, so a sample in the
//! guide cannot drift from the landed API: `cargo check -p
//! paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;

use async_trait::async_trait;

use crate::support::mock_paladin_port;

// ANCHOR: build_graph
use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::waypoint::NodeId;

/// A pure `StateNode` that increments a `count` field each visit and
/// self-loops -- via `NextStep::Edges` and a `Contains("looping")` condition
/// on its own outgoing edge -- until `count` reaches `target`, then writes
/// `status = "done"` and falls out of the loop. The smallest shape that
/// demonstrates cyclic superstep execution, not a straight-line DAG.
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

/// Build a small cyclic `WarGraph`: one `Function` node self-loops over a
/// `(count, status)` `Battlefield` a few times, then falls out of the loop
/// once `status` reads `"done"` -- a shape `WarGraph::validate` accepts
/// precisely because cycles, including self-loops, are legal (ENG-FR-02),
/// unlike the legacy Campaign graph's cycle-rejecting validation.
pub fn build_graph() -> Result<WarGraph, Box<dyn std::error::Error>> {
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

    let mut graph = WarGraph::new(schema, EngineLimits::default());
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
// ANCHOR_END: build_graph

// ANCHOR: configure_limits
use paladin::config::engine::EngineConfig;
use paladin_battalion::engine::WaypointDurability;

/// Configure the engine's bounded-iteration limits and Waypoint durability
/// through the app-facing `EngineConfig` -- the same struct the
/// `APP_ENGINE_MAX_SUPERSTEPS`, `APP_ENGINE_MAX_NODE_VISITS`,
/// `APP_ENGINE_RUN_TIMEOUT_SECS`, `APP_ENGINE_WAYPOINT_DURABILITY` and
/// `APP_ENGINE_MAX_MUSTER_TASKS` environment overrides populate at boot --
/// then convert it into the `EngineLimits` a `WarGraph` is constructed with
/// (`waypoint_durability` stays on the source `EngineConfig` value itself;
/// it is not part of `EngineLimits` and is passed to
/// `WarEngine::with_durability` separately).
pub fn configure_limits() -> Result<(EngineLimits, WaypointDurability), Box<dyn std::error::Error>>
{
    let config = EngineConfig {
        max_supersteps: 20,
        max_node_visits: 10,
        run_timeout_secs: Some(60),
        waypoint_durability: WaypointDurability::Strict,
        max_muster_tasks: 50,
        ..EngineConfig::default()
    };
    config.validate()?;

    let durability = config.waypoint_durability;
    let limits: EngineLimits = config.into();
    Ok((limits, durability))
}
// ANCHOR_END: configure_limits

// ANCHOR: run_engine
use paladin_battalion::engine::{RunOutcome, WarEngine};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// Build the graph, run it to completion over a fresh
/// `InMemoryWaypointStore`, and return the outcome plus the store and
/// thread so [`inspect_waypoints`] can read back the Waypoint history the
/// run left behind -- a `RunOutcome::Failed` carrying
/// `EngineError::RecursionLimitExceeded` is what a graph that never falls
/// out of its loop would produce once `EngineLimits::max_supersteps` is
/// exhausted.
pub async fn run_engine()
-> Result<(RunOutcome, Arc<InMemoryWaypointStore>, ThreadId), Box<dyn std::error::Error>> {
    let graph = build_graph()?;
    let (_limits, durability) = configure_limits()?;
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(mock_paladin_port(), store.clone()).with_durability(durability);
    let thread = ThreadId::new("superstep-engine-guide")?;
    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await?;
    Ok((outcome, store, thread))
}
// ANCHOR_END: run_engine

// ANCHOR: inspect_waypoints
use paladin_core::platform::container::waypoint::WaypointId;
use paladin_ports::output::waypoint_port::WaypointPort;

/// Given the store and thread [`run_engine`] just used, read back the
/// persisted Waypoint via `WaypointPort::latest`, addressed by `(ThreadId,
/// WaypointId)`, and return its `vanguard` -- the nodes ready for the next
/// superstep the engine checkpointed after the run's final superstep.
pub async fn inspect_waypoints(
    store: &InMemoryWaypointStore,
    thread: &ThreadId,
) -> Result<Option<(ThreadId, WaypointId, Vec<NodeId>)>, Box<dyn std::error::Error>> {
    let waypoint = store.latest(thread).await?;
    Ok(waypoint.map(|wp| (wp.thread_id, wp.waypoint_id, wp.vanguard)))
}
// ANCHOR_END: inspect_waypoints
