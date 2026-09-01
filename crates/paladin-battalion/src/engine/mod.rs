//! War Engine — Superstep Execution Engine
//!
//! This module implements the execution engine for [`WarGraph`]s: typed,
//! potentially-cyclic graphs of [`StateNode`]s whose shared state is a
//! [`Battlefield`] (`paladin-core`), automatically checkpointed as a
//! [`Waypoint`] after every superstep through a [`WaypointPort`]
//! (`paladin-ports`).
//!
//! Phase 22 Plan 01 proved the tracer: a single-entry, single-`Function`-node,
//! zero-edge graph, run through [`WarEngine::start`], checkpointed as exactly
//! one `Waypoint`, and resumed by a freshly constructed `WarEngine` with zero
//! re-execution. This plan (05) starts the general superstep engine's build-
//! out: `WarGraph`/`NodeSpec`/`EdgeSpec`/`EngineLimits` move into their own
//! `graph` module and `StateNode`/`NodeContext`/`NodeError` into `node`, with
//! `WarGraph::validate` gaining the full structural checks (edge/entry
//! endpoints, unregistered custom dispatch) while still never rejecting a
//! cycle. The general multi-node superstep loop is this plan's next task.
//!
//! Submodules:
//! - [`graph`] — `WarGraph`, `NodeSpec`, `EdgeSpec`, `EngineLimits`,
//!   `InputMapping`, and `WarGraph::validate`/`fingerprint`.
//! - [`node`] — `StateNode`, `NodeContext`, `NodeError`.

pub mod graph;
pub mod node;

use std::sync::Arc;

use chrono::Utc;
use log::warn;
use thiserror::Error;

#[cfg(test)]
use paladin_core::platform::container::battlefield::CustomDispatchRegistry;
use paladin_core::platform::container::battlefield::{
    Battlefield, CustomDispatchResolver, StateDelta,
};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
#[cfg(test)]
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, NodeExecutionRecord, NodeId, NodeOutcomeKind, ParleyRequest, ThreadId,
    Waypoint, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

pub use graph::{EdgeSpec, EngineLimits, InputMapping, NodeSpec, WarGraph};
pub use node::{NodeContext, NodeError, StateNode};

/// Whether a `WaypointPort::save` failure fails the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaypointDurability {
    /// A `save` failure fails the run with `EngineError::WaypointWrite`
    /// (default; durable-by-default, ENG-FR-11).
    #[default]
    Strict,
    /// A `save` failure is logged as a warning and the run continues.
    BestEffort,
}

/// The outcome of a `WarEngine::start` or `WarEngine::resume` call.
#[derive(Debug)]
pub enum RunOutcome {
    /// The run finished normally.
    Completed {
        /// The final Battlefield state.
        final_state: Battlefield,
        /// The waypoint written for the run's final superstep.
        waypoint: WaypointId,
    },
    /// The run is paused awaiting external input (Doc 03).
    AwaitingInput {
        /// The outstanding input request.
        parley: ParleyRequest,
        /// The waypoint recording the pause.
        waypoint: WaypointId,
    },
    /// The run was gracefully halted (Doc 03 cancellation).
    Halted {
        /// The waypoint recording the halt.
        waypoint: WaypointId,
    },
    /// The run failed.
    Failed {
        /// A human-readable description of the failure.
        error: String,
        /// The last waypoint written before the failure, if any.
        waypoint: Option<WaypointId>,
    },
}

/// Errors returned by [`WarEngine::start`] and [`WarEngine::resume`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// The run's superstep count reached `EngineLimits::max_supersteps`.
    #[error("recursion limit exceeded: {limit} supersteps for thread {thread_id}")]
    RecursionLimitExceeded {
        /// The configured limit that was hit.
        limit: u64,
        /// The thread whose run hit the limit.
        thread_id: ThreadId,
    },

    /// A single node exceeded `EngineLimits::max_node_visits` within one run.
    #[error("node visit limit exceeded: node {node} exceeded {limit} visits")]
    NodeVisitLimitExceeded {
        /// The node that exceeded its visit limit.
        node: NodeId,
        /// The configured limit that was hit.
        limit: u32,
    },

    /// `WarGraph::validate` rejected the graph's limits.
    #[error("invalid engine limits: {reason}")]
    InvalidLimits {
        /// Why the limits were rejected.
        reason: String,
    },

    /// `WarGraph::validate` found an edge or entry point naming a `NodeId`
    /// not present in the graph's node map.
    #[error("unknown node referenced in graph: {0}")]
    UnknownNode(NodeId),

    /// Persisting a Waypoint failed under `WaypointDurability::Strict`.
    #[error("failed to persist waypoint: {source}")]
    WaypointWrite {
        /// The underlying port error.
        #[source]
        source: WaypointError,
    },

    /// Reading a Waypoint back from the port failed.
    #[error("failed to read waypoint: {source}")]
    WaypointRead {
        /// The underlying port error.
        #[source]
        source: WaypointError,
    },

    /// `resume` found a stored Waypoint whose `graph_fingerprint` does not
    /// match the graph passed to `resume` (ENG-FR-14).
    #[error("graph fingerprint mismatch: expected {expected}, got {got}")]
    GraphMismatch {
        /// The fingerprint of the graph passed to `resume`.
        expected: GraphFingerprint,
        /// The fingerprint stored on the latest Waypoint.
        got: GraphFingerprint,
    },

    /// `resume` was called for a thread with no stored Waypoint.
    #[error("thread not found: {0}")]
    ThreadNotFound(ThreadId),

    /// A Battlefield operation (merge, typed accessor, required-field
    /// check) failed.
    #[error("battlefield error: {0}")]
    Battlefield(#[from] BattlefieldError),

    /// A node's execution returned an error.
    #[error("node execution error: {0}")]
    Node(#[from] NodeError),
}

/// Executes [`WarGraph`]s: runs nodes, merges their deltas into the shared
/// [`Battlefield`], and automatically checkpoints a [`Waypoint`] after every
/// superstep through `W: WaypointPort` (ENG-FR-11).
pub struct WarEngine<W: WaypointPort> {
    #[allow(dead_code)] // wired for NodeSpec::Paladin execution in a later plan
    paladin_port: Arc<dyn PaladinPort>,
    waypoint_port: Arc<W>,
    durability: WaypointDurability,
}

impl<W: WaypointPort> WarEngine<W> {
    /// Construct a `WarEngine` over the given Paladin execution port and
    /// Waypoint persistence port, with `WaypointDurability::Strict`.
    pub fn new(paladin_port: Arc<dyn PaladinPort>, waypoint_port: Arc<W>) -> Self {
        Self {
            paladin_port,
            waypoint_port,
            durability: WaypointDurability::Strict,
        }
    }

    /// Override the default `WaypointDurability::Strict`.
    pub fn with_durability(mut self, durability: WaypointDurability) -> Self {
        self.durability = durability;
        self
    }

    /// Start a new run of `graph` under `thread`, seeded with `initial`.
    ///
    /// This plan's Task 1 only lands `WarGraph::validate`'s full structural
    /// checks; `start` still implements the single-entry, single-`Function`-
    /// node, zero-edge case proven by the tracer. The general multi-node
    /// superstep loop is this plan's Task 2.
    pub async fn start(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
        initial: StateDelta,
    ) -> Result<RunOutcome, EngineError> {
        let registry = CustomDispatchResolver::new();
        graph.validate(&registry)?;

        let mut battlefield = Battlefield::initialize(graph.schema().clone(), &initial)?;
        battlefield.validate_required()?;

        if graph.entry().len() != 1 || !graph.edges().is_empty() {
            return Err(EngineError::Node(NodeError(
                "WarEngine::start (Phase 22 Plan 05 Task 1) only supports a single-entry, \
                 zero-edge graph; the general superstep loop is this plan's Task 2"
                    .to_string(),
            )));
        }

        let node_id = graph.entry()[0].clone();
        let spec = graph.node(&node_id).ok_or_else(|| {
            EngineError::Node(NodeError(format!(
                "entry node {node_id} not found in graph"
            )))
        })?;

        let node = match spec {
            NodeSpec::Function(node) => Arc::clone(node),
            NodeSpec::Paladin { .. } => {
                return Err(EngineError::Node(NodeError(
                    "WarEngine::start only supports Function nodes this phase".to_string(),
                )));
            }
        };

        let ctx = NodeContext {
            node_id: node_id.clone(),
            thread_id: thread.clone(),
            superstep: 0,
        };

        let started_at = Utc::now();
        let snapshot = Arc::new(battlefield.clone());
        let delta = node.run(&snapshot, &ctx).await?;
        let duration_ms = (Utc::now() - started_at).num_milliseconds().max(0) as u64;

        battlefield.merge(vec![(node_id.clone(), delta)], 0, &registry)?;

        let waypoint_id = WaypointId::new();
        let waypoint = Waypoint {
            thread_id: thread.clone(),
            waypoint_id,
            parent_waypoint_id: None,
            superstep: 0,
            graph_fingerprint: graph.fingerprint(),
            battlefield: battlefield.clone(),
            vanguard: Vec::new(),
            completed: vec![NodeExecutionRecord {
                node_id,
                paladin_id: None,
                started_at,
                duration_ms,
                token_count: 0,
                outcome: NodeOutcomeKind::Succeeded,
                attempt: 1,
            }],
            status: WaypointStatus::Completed,
            created_at: Utc::now(),
            schema_version: Waypoint::current_schema_version(),
            visit_counts: Default::default(),
        };

        if let Err(source) = self.waypoint_port.save(&waypoint).await {
            match self.durability {
                WaypointDurability::Strict => return Err(EngineError::WaypointWrite { source }),
                WaypointDurability::BestEffort => {
                    warn!(
                        "waypoint save failed under BestEffort durability for thread {thread}: {source}"
                    );
                }
            }
        }

        Ok(RunOutcome::Completed {
            final_state: battlefield,
            waypoint: waypoint_id,
        })
    }

    /// Resume `thread` from its latest Waypoint.
    pub async fn resume(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
    ) -> Result<RunOutcome, EngineError> {
        let latest = self
            .waypoint_port
            .latest(&thread)
            .await
            .map_err(|source| EngineError::WaypointRead { source })?
            .ok_or_else(|| EngineError::ThreadNotFound(thread.clone()))?;

        let expected = graph.fingerprint();
        if latest.graph_fingerprint != expected {
            return Err(EngineError::GraphMismatch {
                expected,
                got: latest.graph_fingerprint,
            });
        }

        match latest.status {
            WaypointStatus::Completed => Ok(RunOutcome::Completed {
                final_state: latest.battlefield,
                waypoint: latest.waypoint_id,
            }),
            _ => Err(EngineError::Node(NodeError(
                "WarEngine::resume only supports resuming a Completed waypoint".to_string(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_ports::output::paladin_port::{PaladinResult, PaladinStream};
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    struct UnimplementedPaladinPort;

    #[async_trait]
    impl PaladinPort for UnimplementedPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            unimplemented!("not exercised by this task's Function-node tracer")
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unimplemented!("not exercised by this task's Function-node tracer")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    struct FixedDeltaNode {
        field: FieldName,
        value: serde_json::Value,
    }

    #[async_trait]
    impl StateNode for FixedDeltaNode {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<StateDelta, NodeError> {
            let mut delta = StateDelta::new();
            delta.set_raw(self.field.clone(), self.value.clone());
            Ok(delta)
        }
    }

    fn one_field_schema() -> BattlefieldSchema {
        BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("result").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )])
    }

    fn engine() -> WarEngine<InMemoryWaypointStore> {
        WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
    }

    #[tokio::test]
    async fn start_runs_one_node_and_persists_one_completed_waypoint() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("solo");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: FieldName::new("result").unwrap(),
                value: serde_json::json!("done"),
            })),
        );
        graph.add_entry(node_id);

        let engine = engine();
        let thread = ThreadId::new("thread-1").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&FieldName::new("result").unwrap())
                        .unwrap(),
                    Some("done".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resume_on_unknown_thread_errors() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_entry(NodeId::new("solo"));

        let engine = engine();
        let thread = ThreadId::new("never-started").unwrap();
        let err = engine.resume(&graph, thread).await.unwrap_err();
        assert!(matches!(err, EngineError::ThreadNotFound(_)));
    }

    #[test]
    fn input_mapping_renders_string_field_raw() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("name").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut battlefield = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(FieldName::new("name").unwrap(), "world").unwrap();
        battlefield
            .merge(
                vec![(NodeId::new("writer"), delta)],
                0,
                &CustomDispatchRegistry::new(),
            )
            .unwrap();

        let mapping = InputMapping::new("hello {name}!");
        assert_eq!(mapping.render(&battlefield), "hello world!");
    }
}
