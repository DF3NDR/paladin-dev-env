//! Directive — Node-Authored Routing (CF-02)
//!
//! [`StateNode::run`] (`paladin-battalion::engine::node`) returns a
//! [`Directive`] instead of a bare `StateDelta`: the partial state update a
//! node contributes, plus how the superstep engine should route control
//! after this node completes ([`NextStep`]).
//!
//! `NextStep::Edges` — the default, produced by [`From<StateDelta>`] for
//! every pre-CF-02 node — preserves the engine's existing behavior exactly:
//! the emitting node's static outgoing edges are evaluated against their
//! declared `EdgeCondition`, as before this phase. Every other variant
//! (`Goto`, `Muster`, `End`, `Parley`) instead resolves the emitting node's
//! static outgoing edges `NotFiring` for that superstep — a node authoring
//! its own routing and a graph author's static edges never both fire for
//! the same execution (`paladin-battalion::engine::superstep`, D-08c).
//!
//! `paladin-core` stays dependency-pure: this module adds no new
//! dependency, it only uses `serde_json` (already a core dependency, ADR-0015).

use serde::{Deserialize, Serialize};

use crate::platform::container::battlefield::StateDelta;
use crate::platform::container::waypoint::{NodeId, ParleyRequest};

/// What a `StateNode` returns from one execution: the [`StateDelta`] it
/// contributes, merged exactly as a bare `StateDelta` return always was,
/// plus a [`NextStep`] telling the superstep engine how to route control
/// next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Directive {
    /// The state delta this execution contributes.
    pub delta: StateDelta,
    /// How the superstep engine should route control after this node
    /// completes.
    pub next: NextStep,
}

/// Routing instruction accompanying a [`Directive`] (CF-FR-05).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NextStep {
    /// Route via the node's static outgoing edges, evaluating each
    /// declared `EdgeCondition` exactly as a pre-CF-02 engine always did.
    /// The default, and the only variant a v0.9-shaped graph ever produces
    /// (`impl From<StateDelta> for Directive`).
    Edges,
    /// Enter the named node(s) directly in the next superstep, bypassing
    /// the normal readiness check (D-08b). Every target must be a declared
    /// node in the graph — an undeclared target fails the run with a typed
    /// error naming both the emitting node and the missing target
    /// (`EngineError::GotoUnknownNode`, D-08a). A target reachable ONLY via
    /// `Goto` (no static incoming edge) must additionally be declared with
    /// the graph's existing dynamic-target marker, the same mechanism
    /// `EngineError::UnreachableNode`'s eligible-set check already uses
    /// (D-08d) — no new marker is introduced for `Goto`. A `Goto` target is
    /// still subject to the engine's per-node visit limit like any other
    /// vanguard entry, so a refine loop (e.g. writer -> reviewer ->
    /// `Goto(writer)` until the reviewer is satisfied) is legal and bounded
    /// rather than an unconditional side channel around the engine's own
    /// bounded-iteration guarantee.
    Goto(Vec<NodeId>),
    /// Fan out `N` worker tasks, intended to execute concurrently in the
    /// next superstep (CF-03). Declared here now, with the dispatch
    /// mechanism itself landing in a later plan, so this shape does not
    /// change when that mechanism is added.
    Muster(Vec<MusterTask>),
    /// Complete the run after this superstep's merge (CF-FR-08). Peers in
    /// the same superstep still merge their own deltas normally; `End`
    /// takes precedence over a `Goto` emitted by another node in the same
    /// superstep.
    End,
    /// Pause the run awaiting external input (Doc 03's suspension
    /// mechanism). This phase does not implement suspension: a node
    /// returning `Parley` fails the run with a typed error
    /// (`EngineError::ParleyNotSupported`) rather than pausing it — it is
    /// never silently treated as `Edges`, and no `WaypointStatus::AwaitingInput`
    /// checkpoint is written for it here.
    Parley(ParleyRequest),
}

/// One runtime-produced worker task from a [`NextStep::Muster`] fan-out
/// (CF-03). Declared here now, ahead of the dispatch mechanism a later plan
/// adds, so that mechanism's arrival does not change this shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusterTask {
    /// The worker-template node this task dispatches to.
    pub worker: NodeId,
    /// This task's isolated payload. Never merged into the Battlefield —
    /// visible only to the worker node itself.
    pub payload: serde_json::Value,
    /// A caller-chosen key identifying this task among its siblings, used
    /// to order worker results deterministically on aggregation and to
    /// reject a duplicate task within one Muster.
    pub task_key: String,
}

/// The per-task context a [`NextStep::Muster`] worker execution carries
/// (CF-03, D-15): its isolated payload and its `task_key`. Reachable ONLY
/// through `paladin_battalion::engine::node::NodeContext`'s `muster` field
/// and its `muster_payload()`/`task_key()` accessors, and through
/// `{muster.payload}`/`{muster.task_key}` in an `InputMapping` template
/// (`paladin_battalion::engine::input_mapping`) — never merged into the
/// Battlefield, and never reachable through a schema field (graph
/// validation rejects a schema field named with the `muster.` prefix).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusterContext {
    /// This task's isolated payload, as given in the [`MusterTask`] that
    /// spawned this execution.
    pub payload: serde_json::Value,
    /// This task's `task_key`, as given in the [`MusterTask`] that spawned
    /// this execution.
    pub task_key: String,
}

impl From<StateDelta> for Directive {
    /// Wrap a bare `StateDelta` as a `Directive` routed via
    /// [`NextStep::Edges`] — the shape every pre-CF-02 `StateNode`
    /// implementor produces, unchanged. Every in-tree node adopts this via
    /// `Ok(delta.into())` in place of the prior `Ok(delta)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::battlefield::StateDelta;
    /// use paladin_core::platform::container::directive::{Directive, NextStep};
    ///
    /// let delta = StateDelta::new();
    /// let directive: Directive = delta.clone().into();
    /// assert_eq!(directive.delta, delta);
    /// assert_eq!(directive.next, NextStep::Edges);
    /// ```
    fn from(delta: StateDelta) -> Self {
        Self {
            delta,
            next: NextStep::Edges,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_delta_converts_to_a_directive_defaulting_to_edges() {
        let delta = StateDelta::new();
        let directive: Directive = delta.clone().into();
        assert_eq!(directive.delta, delta);
        assert_eq!(directive.next, NextStep::Edges);
    }
}
