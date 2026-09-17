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
