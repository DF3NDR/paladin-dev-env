//! Node execution surface for the superstep engine.
//!
//! Defines [`StateNode`], the pure state -> delta node trait `Function`
//! variants of [`crate::engine::graph::NodeSpec`] implement, its execution
//! context [`NodeContext`], and its error type [`StateNodeError`].
//!
//! `StateNodeError` (renamed from `NodeError`, D-06) is deliberately a
//! bare-`String` newtype -- the same shape it has always had -- rather than
//! the structured `paladin_core::platform::container::node_error::NodeError`
//! Doc 04 introduces: a `StateNode` author still just returns a message, and
//! `impl From<StateNodeError> for NodeErrorSource` is where that message
//! crosses into the structured family, at the engine boundary where the
//! node identity and attempt number are in scope (D-07).

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::battlefield::Battlefield;
use paladin_core::platform::container::directive::{Directive, MusterContext};
use paladin_core::platform::container::node_error::NodeErrorSource;
use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};

use crate::engine::heartbeat::HeartbeatHandle;

/// Error returned by a [`StateNode`]'s execution.
///
/// Renamed from `NodeError` (D-06): the PRD's own `NodeError` name is taken
/// by the structured
/// `paladin_core::platform::container::node_error::NodeError` this phase
/// lands, so this pre-existing, new-in-`v0.10.0` engine newtype (absent at
/// `v0.9.0`, so this rename breaks no published contract) moves aside.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{0}")]
pub struct StateNodeError(pub String);

impl From<StateNodeError> for NodeErrorSource {
    /// A `StateNode`'s own error becomes a `NodeErrorSource::Function`
    /// (D-07): its message is already first-party text (never a
    /// provider-sourced excerpt), so no redaction step applies here -- D-34
    /// governs `Paladin`/`Llm` variant construction, not this one.
    fn from(err: StateNodeError) -> Self {
        NodeErrorSource::Function { message: err.0 }
    }
}

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
    /// The answer to this node's own outstanding `ParleyRequest`, populated
    /// only on the post-resume re-run of a parleying node (HITL-01, D-07,
    /// D-08): `Some` when `WarEngine::resume_with` seeded this superstep's
    /// vanguard with a matching `ParleyResponse`, `None` for every ordinary
    /// execution -- including a node's own FIRST run, the one that raises
    /// the parley in the first place. Never merged into the Battlefield --
    /// reachable only through this field and its accessor, and through the
    /// `parley.` `InputMapping` namespace (a later plan).
    pub parley_response: Option<ParleyResponse>,
    /// The 1-indexed attempt this execution is (Doc 04 D-18): `1` for a
    /// node's first run, `2` for its first RETRY under an Aegis retry
    /// policy, and so on. A node with no retry policy always sees `1`. The
    /// context is rebuilt per attempt, so a `StateNode` never observes a
    /// stale value from a previous attempt.
    pub attempt: u32,
    /// This attempt's progress channel (D-18, D-19): fresh per attempt,
    /// beaten by [`NodeContext::heartbeat`], by `PaladinPort::execute_observed`
    /// for a Paladin node, and once per child superstep for a Battalion
    /// node. Only an `idle_timeout` on this node's resolved
    /// `TimeoutPolicy` ever subscribes to it; without one the handle exists
    /// but nothing is watching, so beating it is a cheap no-op. Never
    /// merged into the Battlefield and never part of a context's identity
    /// (any two handles compare equal).
    pub heartbeat: HeartbeatHandle,
}

impl NodeContext {
    /// The 1-indexed attempt this execution is (D-18); `1` for a node with
    /// no retry policy.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Report progress (D-18, FT-FR-09): resets this attempt's
    /// `TimeoutPolicy::idle_timeout` timer, if the node has one. On a node
    /// with no `idle_timeout` this is a cheap no-op -- the handle exists
    /// but nothing subscribes to it, so a `StateNode` author can call this
    /// unconditionally inside a long loop without checking the policy.
    ///
    /// A `StateNode` that never calls this under an `idle_timeout` is
    /// bounded by that timeout exactly as a stalled node would be: the idle
    /// bound degrades to a per-attempt wall clock (D-19).
    pub fn heartbeat(&self) {
        self.heartbeat.beat();
    }

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

    /// The answer to this node's own outstanding `ParleyRequest` (HITL-01,
    /// D-07), or `None` outside a post-resume re-run of a parleying node.
    pub fn parley_response(&self) -> Option<&ParleyResponse> {
        self.parley_response.as_ref()
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
    async fn run(
        &self,
        state: &Battlefield,
        ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::heartbeat::HeartbeatHandle;

    fn ctx(heartbeat: HeartbeatHandle) -> NodeContext {
        NodeContext {
            node_id: NodeId::new("n"),
            thread_id: ThreadId::new("t").unwrap(),
            superstep: 1,
            muster: None,
            parley_response: None,
            attempt: 1,
            heartbeat,
        }
    }

    /// D-18: the `Debug, Clone, PartialEq` derive set on `NodeContext` is
    /// load-bearing (interceptor tests and `Directive` plumbing compare
    /// contexts by value), so `HeartbeatHandle` must satisfy it -- two
    /// contexts differing ONLY in their handle compare equal, cloning
    /// works, and `Debug` renders without exposing the handle's internals.
    #[test]
    fn node_context_keeps_its_derives_with_a_heartbeat_handle() {
        fn assert_derives<T: Clone + PartialEq + std::fmt::Debug>() {}
        assert_derives::<NodeContext>();

        let a = ctx(HeartbeatHandle::new());
        let b = ctx(HeartbeatHandle::new());
        b.heartbeat.beat();
        assert_eq!(a, b, "handles compare equal regardless of beat state");

        let c = a.clone();
        assert_eq!(a, c);
        assert_eq!(c.attempt(), 1);
        assert!(
            format!("{a:?}").contains("HeartbeatHandle"),
            "Debug renders an opaque placeholder for the handle"
        );
    }
}
