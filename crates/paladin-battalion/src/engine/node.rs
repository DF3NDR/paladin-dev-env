//! Node execution surface for the superstep engine.
//!
//! Defines [`StateNode`], the pure state -> delta node trait `Function`
//! variants of [`crate::engine::graph::NodeSpec`] implement, its execution
//! context [`NodeContext`], and its error type [`NodeError`].

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::battlefield::Battlefield;
use paladin_core::platform::container::directive::{Directive, MusterContext};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};

/// Error returned by a [`StateNode`]'s execution.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{0}")]
pub struct NodeError(pub String);

/// The read-only context a [`StateNode`] runs with. Carries only what this
/// phase needs; later plans extend this rather than changing its existing
/// fields (attempt counters, cancellation tokens, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct NodeContext {
    /// The node currently executing.
    pub node_id: NodeId,
    /// The thread (run) this execution belongs to.
    pub thread_id: ThreadId,
    /// The superstep index this execution belongs to.
    pub superstep: u64,
    /// This execution's Muster task context (CF-03, D-15): `Some` only for
    /// a synthetic worker-task dispatch spawned from a returned
    /// `NextStep::Muster(tasks)` Directive, `None` for every ordinary
    /// vanguard execution. Never merged into the Battlefield — reachable
    /// only through this field and its accessors, and through
    /// `{muster.payload}`/`{muster.task_key}` in an `InputMapping` template.
    pub muster: Option<MusterContext>,
}

impl NodeContext {
    /// This execution's Muster task payload (CF-FR-10), or `None` outside a
    /// Muster worker-task dispatch.
    pub fn muster_payload(&self) -> Option<&serde_json::Value> {
        self.muster.as_ref().map(|m| &m.payload)
    }

    /// This execution's Muster `task_key` (CF-FR-10), or `None` outside a
    /// Muster worker-task dispatch.
    pub fn task_key(&self) -> Option<&str> {
        self.muster.as_ref().map(|m| m.task_key.as_str())
    }
}

/// A pure state -> delta node: reads the Battlefield snapshot for its
/// superstep and returns the partial update it contributes, plus how the
/// engine should route control next (CF-02).
#[async_trait]
pub trait StateNode: Send + Sync {
    /// Execute against `state`, producing a [`Directive`] whose `delta` is
    /// merged into the Battlefield via each touched field's dispatch rule,
    /// and whose `next` steers the superstep engine's routing (CF-FR-05).
    ///
    /// Every pre-CF-02 implementor -- which only ever produced a
    /// `StateDelta` -- adopts this via `Ok(delta.into())`
    /// (`impl From<StateDelta> for Directive` defaults `next:
    /// NextStep::Edges`, preserving the prior behavior exactly).
    async fn run(&self, state: &Battlefield, ctx: &NodeContext) -> Result<Directive, NodeError>;
}
