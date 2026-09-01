//! Node execution surface for the superstep engine.
//!
//! Defines [`StateNode`], the pure state -> delta node trait `Function`
//! variants of [`crate::engine::graph::NodeSpec`] implement, its execution
//! context [`NodeContext`], and its error type [`NodeError`].

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
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
}

/// A pure state -> delta node: reads the Battlefield snapshot for its
/// superstep and returns the partial update it contributes.
#[async_trait]
pub trait StateNode: Send + Sync {
    /// Execute against `state`, producing a [`StateDelta`] to be merged into
    /// the Battlefield via each touched field's dispatch rule.
    async fn run(&self, state: &Battlefield, ctx: &NodeContext) -> Result<StateDelta, NodeError>;
}
