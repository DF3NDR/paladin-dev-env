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
//! re-execution. Plan 05 expands this into the real superstep engine
//! (`engine::superstep`): the general multi-node loop with cycles, snapshot
//! isolation, bounded concurrency, and both engine limits. Dispatch-conflict
//! surfacing, precise join/defer semantics and full `resume` are later
//! plans' expansion (22-07, 22-08) — this module's types are shaped so that
//! expansion does not require changing these signatures.
//!
//! Submodules:
//! - [`graph`] — `WarGraph`, `NodeSpec`, `EdgeSpec`, `EngineLimits`,
//!   `InputMapping`, and `WarGraph::validate`/`fingerprint`.
//! - [`node`] — `StateNode`, `NodeContext`, `NodeError`.
//! - [`dispatch_registry`] — `DispatchRegistry`, the engine-owned
//!   `DispatchRule::Custom` name -> closure registration (ENG-FR-09).
//! - `superstep` (private) — the superstep loop `start`/`resume` reduce to.
//! - `test_support` (`#[cfg(test)]`) — `RecordingWaypointStore` and
//!   `CountingFunctionNode`, the doubles this and later engine plans assert
//!   against.

pub mod dispatch_registry;
pub mod graph;
pub mod node;
mod superstep;
#[cfg(test)]
pub(crate) mod test_support;

use std::collections::BTreeMap;
use std::sync::Arc;

use thiserror::Error;

#[cfg(test)]
use paladin_core::platform::container::battlefield::CustomDispatchResolver;
use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
#[cfg(test)]
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, NodeId, ParleyRequest, ThreadId, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

pub use dispatch_registry::DispatchRegistry;
pub use graph::{EdgeSpec, EngineLimits, InputMapping, NodeSpec, WarGraph};
pub use node::{NodeContext, NodeError, StateNode};

/// Whether a `WaypointPort::save` failure fails the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaypointDurability {
    /// A `save` failure fails the run with `EngineError::WaypointWrite`
    /// (default; durable-by-default, ENG-FR-11).
    #[default]
    Strict,
    /// A `save` failure is logged as a warning and the run continues. **Do
    /// not** select this in any example, doc snippet, config template or
    /// shared test helper: a failed checkpoint write silently downgrades to
    /// a logged warning, and a whole superstep of work can be lost with no
    /// other signal. Opt in explicitly and locally, only where the
    /// consequence is understood and accepted.
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
    /// The run failed — a bounded-iteration limit was hit, or a node's
    /// execution or the merge it fed returned an error. A Waypoint carrying
    /// `WaypointStatus::Failed` has already been persisted (subject to
    /// `WaypointDurability`) by the time this variant is returned.
    Failed {
        /// The engine error that caused the run to fail.
        error: EngineError,
        /// The waypoint just written recording the failure, if persistence
        /// was attempted.
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

    /// An `EdgeCondition::Regex` pattern failed to compile.
    #[error("invalid edge condition: {reason}")]
    InvalidEdgeCondition {
        /// Why the condition was rejected.
        reason: String,
    },

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

    /// `DispatchRegistry::register` was asked to register a custom
    /// dispatch rule under a name that collides with a built-in
    /// `DispatchRule` variant name (ENG-FR-09). Rejected at registration so
    /// a schema author cannot believe they have overridden e.g.
    /// `LastWrite` when they have not.
    #[error("cannot register custom dispatch rule '{name}': reserved built-in rule name")]
    ReservedDispatchName {
        /// The rejected registration name.
        name: String,
    },
}

/// Executes [`WarGraph`]s: runs nodes, merges their deltas into the shared
/// [`Battlefield`], and automatically checkpoints a [`Waypoint`] after every
/// superstep through `W: WaypointPort` (ENG-FR-11).
pub struct WarEngine<W: WaypointPort> {
    #[allow(dead_code)] // wired for NodeSpec::Paladin execution in Plan 22-08
    paladin_port: Arc<dyn PaladinPort>,
    waypoint_port: Arc<W>,
    durability: WaypointDurability,
    /// In-flight node execution cap per superstep. `None` defaults to the
    /// Vanguard's own size (D-12) — i.e. effectively unbounded unless
    /// explicitly lowered.
    parallelism: Option<usize>,
    /// Engine-owned custom dispatch rule registrations (ENG-FR-09). Never
    /// referenced from `paladin-core` (X-01) -- handed to
    /// `WarGraph::validate` and `Battlefield::merge` as a
    /// `CustomDispatchResolver` at `start`.
    dispatch_registry: DispatchRegistry,
}

impl<W: WaypointPort> WarEngine<W> {
    /// Construct a `WarEngine` over the given Paladin execution port and
    /// Waypoint persistence port, with `WaypointDurability::Strict`, no
    /// explicit parallelism cap and no custom dispatch rules registered.
    pub fn new(paladin_port: Arc<dyn PaladinPort>, waypoint_port: Arc<W>) -> Self {
        Self {
            paladin_port,
            waypoint_port,
            durability: WaypointDurability::Strict,
            parallelism: None,
            dispatch_registry: DispatchRegistry::new(),
        }
    }

    /// Override the default `WaypointDurability::Strict`.
    pub fn with_durability(mut self, durability: WaypointDurability) -> Self {
        self.durability = durability;
        self
    }

    /// Bound the number of nodes executed concurrently within one
    /// superstep. Defaults to the Vanguard's own size (D-12) when not set.
    pub fn with_parallelism(mut self, limit: usize) -> Self {
        self.parallelism = Some(limit);
        self
    }

    /// Register a `(current, delta) -> merged` closure under `name`
    /// (ENG-FR-09), applied when a Battlefield field declares
    /// `DispatchRule::Custom(name)`. Rejects a `name` colliding with a
    /// built-in `DispatchRule` variant name with
    /// `EngineError::ReservedDispatchName` -- registration is where that
    /// collision is caught, not silently ignored later.
    pub fn with_dispatch_rule(
        mut self,
        name: impl Into<String>,
        rule: Arc<paladin_core::platform::container::battlefield::CustomDispatchFn>,
    ) -> Result<Self, EngineError> {
        self.dispatch_registry.register(name, rule)?;
        Ok(self)
    }

    /// Start a new run of `graph` under `thread`, seeded with `initial`.
    ///
    /// Runs the full superstep loop (ENG-FR-01): validates the graph,
    /// resolves the initial Battlefield state, then executes supersteps
    /// until the Vanguard is empty (`RunOutcome::Completed`) or a limit or
    /// node/merge failure intervenes (`RunOutcome::Failed`). `NodeSpec::
    /// Paladin` execution is Plan 22-08's expansion; a graph containing one
    /// fails with a typed `EngineError::Node` before any Paladin runs.
    pub async fn start(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
        initial: StateDelta,
    ) -> Result<RunOutcome, EngineError> {
        let registry = self.dispatch_registry.resolver();
        graph.validate(registry)?;

        let battlefield = Battlefield::initialize(graph.schema().clone(), &initial)?;
        battlefield.validate_required()?;

        superstep::run(
            self.waypoint_port.as_ref(),
            self.durability,
            self.parallelism,
            registry,
            graph,
            thread,
            battlefield,
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            1,
        )
        .await
    }

    /// Resume `thread` from its latest Waypoint.
    ///
    /// This phase implements the case the tracer proves: loads the latest
    /// Waypoint (absent -> `ThreadNotFound`), compares its
    /// `graph_fingerprint` against `graph.fingerprint()` (differing ->
    /// `GraphMismatch`), and when the loaded status is `Completed` returns
    /// `RunOutcome::Completed` immediately without executing anything
    /// (ENG-FR-12). Resuming a `Running`/`Failed`/`AwaitingInput`/`Halted`
    /// waypoint through the same superstep loop `start` uses is Plan
    /// 22-08's expansion.
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
                "WarEngine::resume only supports resuming a Completed waypoint until Plan 22-08"
                    .to_string(),
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
            unimplemented!("not exercised by this plan's Function-node tests")
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unimplemented!("not exercised by this plan's Function-node tests")
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

    // --- Task 2: engine-level custom dispatch registry -------------------

    #[tokio::test]
    async fn engine_with_dispatch_rule_applies_custom_merge_end_to_end() {
        let field_name = FieldName::new("score").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::Custom("max".to_string()),
            Some(serde_json::json!(0)),
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("scorer");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: field_name.clone(),
                value: serde_json::json!(7),
            })),
        );
        graph.add_entry(node_id);

        let engine = engine()
            .with_dispatch_rule(
                "max",
                Arc::new(|current: &serde_json::Value, delta: &serde_json::Value| {
                    let c = current.as_i64().unwrap_or(i64::MIN);
                    let d = delta.as_i64().unwrap_or(i64::MIN);
                    Ok(serde_json::json!(c.max(d)))
                }),
            )
            .unwrap();
        let thread = ThreadId::new("custom-dispatch").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(final_state.get::<i64>(&field_name).unwrap(), Some(7));
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn engine_start_fails_before_execution_for_unregistered_custom_dispatch() {
        let field_name = FieldName::new("score").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::Custom("missing".to_string()),
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node = crate::engine::test_support::CountingFunctionNode::fixed(
            field_name,
            serde_json::json!(1),
        );
        let node_id = NodeId::new("n");
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id);

        let engine = engine();
        let thread = ThreadId::new("unregistered-custom").unwrap();
        let err = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap_err();
        match err {
            EngineError::Battlefield(BattlefieldError::CustomDispatchNotRegistered { name }) => {
                assert_eq!(name, "missing");
            }
            other => panic!("expected CustomDispatchNotRegistered, got {other:?}"),
        }
        assert_eq!(
            node.run_count(),
            0,
            "no node executes before graph validation passes"
        );
    }

    #[tokio::test]
    async fn engine_two_writer_last_write_conflict_surfaces_field_superstep_and_writers() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let n1 = NodeId::new("n1");
        let n2 = NodeId::new("n2");
        graph.add_node(
            n1.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: FieldName::new("result").unwrap(),
                value: serde_json::json!("a"),
            })),
        );
        graph.add_node(
            n2.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: FieldName::new("result").unwrap(),
                value: serde_json::json!("b"),
            })),
        );
        graph.add_entry(n1.clone());
        graph.add_entry(n2.clone());

        let engine = engine();
        let thread = ThreadId::new("dispatch-conflict").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Failed { error, .. } => match error {
                EngineError::Battlefield(BattlefieldError::DispatchConflict {
                    field,
                    superstep,
                    writers,
                }) => {
                    assert_eq!(field, FieldName::new("result").unwrap());
                    assert_eq!(superstep, 1);
                    let mut sorted = writers.clone();
                    sorted.sort();
                    assert_eq!(sorted, vec![n1.clone(), n2.clone()]);
                }
                other => panic!("expected DispatchConflict, got {other:?}"),
            },
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn engine_custom_dispatch_closure_error_fails_the_run_not_swallowed() {
        let field_name = FieldName::new("score").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::Custom("boom".to_string()),
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("n");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: field_name.clone(),
                value: serde_json::json!(1),
            })),
        );
        graph.add_entry(node_id);

        let engine = engine()
            .with_dispatch_rule(
                "boom",
                Arc::new(|_c: &serde_json::Value, _d: &serde_json::Value| {
                    Err(BattlefieldError::TypeMismatch {
                        field: FieldName::new("score").unwrap(),
                        expected: "never".to_string(),
                        got: "boom".to_string(),
                    })
                }),
            )
            .unwrap();
        let thread = ThreadId::new("custom-dispatch-error").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Failed { error, .. } => {
                assert!(matches!(
                    error,
                    EngineError::Battlefield(BattlefieldError::TypeMismatch { .. })
                ));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
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
                &CustomDispatchResolver::new(),
            )
            .unwrap();

        let mapping = InputMapping::new("hello {name}!");
        assert_eq!(mapping.render(&battlefield), "hello world!");
    }
}
