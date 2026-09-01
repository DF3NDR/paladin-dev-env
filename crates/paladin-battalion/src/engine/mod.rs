//! War Engine — Superstep Execution Engine
//!
//! This module implements the execution engine for [`WarGraph`]s: typed,
//! potentially-cyclic graphs of [`StateNode`]s whose shared state is a
//! [`Battlefield`] (`paladin-core`), automatically checkpointed as a
//! [`Waypoint`] after every superstep through a [`WaypointPort`]
//! (`paladin-ports`).
//!
//! Phase 22 Plan 01 proves one thin, production-quality end-to-end path:
//! a single-entry, single-`Function`-node, zero-edge graph, run through
//! [`WarEngine::start`], checkpointed as exactly one `Waypoint`, and resumed
//! by a freshly constructed `WarEngine` with zero re-execution. The general
//! multi-node superstep loop (ENG-FR-01), cycles, dispatch-conflict
//! detection, and dynamic routing are later plans' expansion — this module's
//! types are shaped so that expansion does not require changing these
//! signatures.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use log::warn;
use thiserror::Error;

use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, CustomDispatchRegistry, FieldName, StateDelta,
};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, NodeExecutionRecord, NodeId, NodeOutcomeKind, ParleyRequest, ThreadId,
    Waypoint, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

/// Renders a Paladin's string input from the Battlefield: a template string
/// with `{field}` placeholders resolved from state (values JSON-stringified
/// unless the field is a JSON string, in which case it is inserted raw).
///
/// This is the bridge that lets today's string-in/string-out Paladins
/// participate in typed workflows unchanged (X-03). Wiring this into node
/// execution for `NodeSpec::Paladin` is later-plan scope (Plan 22-01 only
/// executes `Function` nodes); this type lands now so `NodeSpec`'s shape is
/// final.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InputMapping {
    template: String,
}

impl InputMapping {
    /// Construct an `InputMapping` from a template string containing
    /// `{field}` placeholders.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }

    /// Render this template against `state`, substituting each `{field}`
    /// placeholder with the field's Battlefield value (raw for JSON
    /// strings, JSON-stringified otherwise). An unresolvable or malformed
    /// placeholder renders as an empty string rather than panicking.
    pub fn render(&self, state: &Battlefield) -> String {
        let mut rendered = self.template.clone();
        let mut idx = 0;
        while idx < rendered.len() {
            let Some(rel_start) = rendered[idx..].find('{') else {
                break;
            };
            let start = idx + rel_start;
            let Some(rel_end) = rendered[start..].find('}') else {
                break;
            };
            let end = start + rel_end;
            let field_name = &rendered[start + 1..end];
            let replacement = match FieldName::new(field_name) {
                Ok(field) => match state.get_raw(&field) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(value) => value.to_string(),
                    None => String::new(),
                },
                Err(_) => {
                    idx = end + 1;
                    continue;
                }
            };
            rendered.replace_range(start..=end, &replacement);
            idx = start + replacement.len();
        }
        rendered
    }
}

/// Error returned by a [`StateNode`]'s execution.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{0}")]
pub struct NodeError(pub String);

/// The read-only context a [`StateNode`] runs with. Carries only what Phase
/// 22 Plan 01 needs; later plans extend this rather than changing its
/// existing fields (attempt counters, cancellation tokens, etc.).
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

/// One node in a [`WarGraph`].
///
/// Declared `#[non_exhaustive]`: Doc 02 adds a `Battalion` (subgraph) variant
/// without this being a breaking change.
#[non_exhaustive]
pub enum NodeSpec {
    /// A node backed by an existing string-in/string-out `Paladin`, bridged
    /// into typed state via `input_template` and `output_field` (X-03).
    Paladin {
        /// The Paladin this node wraps. Boxed: `Paladin` is a large value
        /// type (a `Node<PaladinData>`), and boxing it keeps `NodeSpec`
        /// itself small regardless of how many `Function` nodes a graph
        /// declares alongside it.
        paladin: Box<Paladin>,
        /// Renders the Paladin's string input from the Battlefield.
        input_template: InputMapping,
        /// The field the Paladin's output is written to as a delta.
        output_field: FieldName,
    },
    /// A pure, deterministic state -> delta node.
    Function(Arc<dyn StateNode>),
}

impl std::fmt::Debug for NodeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeSpec::Paladin { output_field, .. } => f
                .debug_struct("NodeSpec::Paladin")
                .field("output_field", output_field)
                .finish(),
            NodeSpec::Function(_) => f.debug_tuple("NodeSpec::Function").field(&"<fn>").finish(),
        }
    }
}

/// A static edge between two nodes in a [`WarGraph`]. Dynamic routing (Doc
/// 02) is out of scope for this phase; edges here are declared up front.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EdgeSpec {
    /// The source node.
    pub from: NodeId,
    /// The target node.
    pub to: NodeId,
    /// An optional condition gating traversal, reusing the existing
    /// Campaign `EdgeCondition` vocabulary.
    pub condition: Option<EdgeCondition>,
}

/// Bounds on a `WarGraph` run, required so cyclic graphs (ENG-FR-02) always
/// terminate.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLimits {
    /// Maximum number of supersteps before the run fails with
    /// `EngineError::RecursionLimitExceeded`. Must be `>= 1`.
    pub max_supersteps: u64,
    /// Maximum number of times any single node may execute within one run,
    /// before `EngineError::NodeVisitLimitExceeded`. Must be `>= 1`.
    pub max_node_visits: u32,
    /// Optional wall-clock timeout for the whole run.
    pub run_timeout: Option<Duration>,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            max_supersteps: 50,
            max_node_visits: 25,
            run_timeout: None,
        }
    }
}

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

/// The executable graph a [`WarEngine`] runs.
///
/// Deliberately does **not** reject cycles (ENG-FR-02): unlike Campaign's
/// cycle-rejecting graph-order validation, `WarGraph::validate` only
/// enforces that both `EngineLimits` are non-zero, so iterative workflows
/// (retry-and-refine, evaluate-optimize loops) can be expressed here even
/// though Campaign cannot express them.
pub struct WarGraph {
    nodes: HashMap<NodeId, NodeSpec>,
    edges: Vec<EdgeSpec>,
    schema: BattlefieldSchema,
    entry: Vec<NodeId>,
    limits: EngineLimits,
}

impl WarGraph {
    /// Construct an empty `WarGraph` with the given schema and limits.
    pub fn new(schema: BattlefieldSchema, limits: EngineLimits) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            schema,
            entry: Vec::new(),
            limits,
        }
    }

    /// Register a node under `id`.
    pub fn add_node(&mut self, id: NodeId, spec: NodeSpec) -> &mut Self {
        self.nodes.insert(id, spec);
        self
    }

    /// Add a static edge.
    pub fn add_edge(&mut self, edge: EdgeSpec) -> &mut Self {
        self.edges.push(edge);
        self
    }

    /// Mark `id` as an entry-point node (part of superstep 0's Vanguard).
    pub fn add_entry(&mut self, id: NodeId) -> &mut Self {
        self.entry.push(id);
        self
    }

    /// The graph's Battlefield schema.
    pub fn schema(&self) -> &BattlefieldSchema {
        &self.schema
    }

    /// The graph's entry-point nodes, in registration order.
    pub fn entry(&self) -> &[NodeId] {
        &self.entry
    }

    /// The graph's declared edges.
    pub fn edges(&self) -> &[EdgeSpec] {
        &self.edges
    }

    /// The graph's execution limits.
    pub fn limits(&self) -> &EngineLimits {
        &self.limits
    }

    /// Look up a node's declaration by id.
    pub fn node(&self, id: &NodeId) -> Option<&NodeSpec> {
        self.nodes.get(id)
    }

    /// Validate structural invariants. Does NOT reject cycles (ENG-FR-02):
    /// only rejects a graph whose limits could never terminate.
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.limits.max_supersteps == 0 {
            return Err(EngineError::InvalidLimits {
                reason: "max_supersteps must be at least 1".to_string(),
            });
        }
        if self.limits.max_node_visits == 0 {
            return Err(EngineError::InvalidLimits {
                reason: "max_node_visits must be at least 1".to_string(),
            });
        }
        Ok(())
    }

    /// Compute this graph's stable content fingerprint (ENG-FR-14): a hash
    /// over node ids, edge specs and schema field names — deliberately NOT
    /// over prompts or models. Node ids, edges (by `from`/`to`) and schema
    /// field names are sorted before hashing so the result never depends on
    /// `HashMap` iteration order (RESEARCH.md Pitfall 5).
    pub fn fingerprint(&self) -> GraphFingerprint {
        let mut node_ids: Vec<&NodeId> = self.nodes.keys().collect();
        node_ids.sort();

        let mut edges: Vec<&EdgeSpec> = self.edges.iter().collect();
        edges.sort_by(|a, b| {
            (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str()))
        });

        let mut field_names: Vec<&FieldName> = self.schema.fields.iter().map(|f| &f.name).collect();
        field_names.sort();

        let mut buf = Vec::new();
        buf.extend_from_slice(b"nodes:");
        for id in &node_ids {
            buf.extend_from_slice(id.as_str().as_bytes());
            buf.push(b'|');
        }
        buf.extend_from_slice(b";edges:");
        for edge in &edges {
            buf.extend_from_slice(edge.from.as_str().as_bytes());
            buf.push(b'-');
            buf.extend_from_slice(edge.to.as_str().as_bytes());
            buf.push(b':');
            buf.extend_from_slice(format!("{:?}", edge.condition).as_bytes());
            buf.push(b'|');
        }
        buf.extend_from_slice(b";schema:");
        for name in &field_names {
            buf.extend_from_slice(name.as_str().as_bytes());
            buf.push(b'|');
        }

        GraphFingerprint::from_canonical_bytes(&buf)
    }
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
    /// Phase 22 Plan 01 implements the single-entry, single-`Function`-node,
    /// zero-edge case only: snapshots the Battlefield, runs the entry node,
    /// merges its delta, persists exactly one `Waypoint` with
    /// `status: Completed` and an empty vanguard, and returns
    /// `RunOutcome::Completed`. The general multi-node superstep loop is
    /// later plans' expansion.
    pub async fn start(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
        initial: StateDelta,
    ) -> Result<RunOutcome, EngineError> {
        graph.validate()?;

        let registry = CustomDispatchRegistry::new();
        let mut battlefield = Battlefield::initialize(graph.schema().clone(), &initial)?;
        battlefield.validate_required()?;

        if graph.entry().len() != 1 || !graph.edges().is_empty() {
            return Err(EngineError::Node(NodeError(
                "WarEngine::start (Phase 22 Plan 01) only supports a single-entry, zero-edge graph"
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
                    "WarEngine::start (Phase 22 Plan 01) only supports Function nodes".to_string(),
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
    ///
    /// Phase 22 Plan 01 implements the single case the tracer proves:
    /// loads the latest Waypoint (absent -> `ThreadNotFound`), compares its
    /// `graph_fingerprint` against `graph.fingerprint()` (differing ->
    /// `GraphMismatch`), and when the loaded status is `Completed` returns
    /// `RunOutcome::Completed` immediately without executing anything
    /// (ENG-FR-12). Resuming a `Running`/`Failed`/`AwaitingInput`/`Halted`
    /// waypoint is later plans' expansion.
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
                "WarEngine::resume (Phase 22 Plan 01) only supports resuming a Completed waypoint"
                    .to_string(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battlefield::{DispatchRule, FieldSpec};
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
            unimplemented!("not exercised by Phase 22 Plan 01's Function-node tracer")
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unimplemented!("not exercised by Phase 22 Plan 01's Function-node tracer")
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
    fn fingerprint_is_deterministic_across_calls() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("solo"),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: FieldName::new("result").unwrap(),
                value: serde_json::json!("done"),
            })),
        );
        graph.add_entry(NodeId::new("solo"));

        let a = graph.fingerprint();
        let b = graph.fingerprint();
        assert_eq!(a, b);
        // Hard-coded expected value: this fixture's exact byte stream
        // (one entry node "solo", zero edges, one schema field "result")
        // hashed with blake3 and encoded per the Task 1 decision (option-b,
        // `v1:{blake3_hex}`) always yields this string, in this process or
        // a fresh one.
        assert_eq!(
            a.as_str(),
            "v1:f5532b613066cb2d1972451bad73120abafbf7cbafd8ecf572a043448c31d2d6"
        );
    }

    #[test]
    fn validate_rejects_zero_max_supersteps() {
        let graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_supersteps: 0,
                ..EngineLimits::default()
            },
        );
        assert!(matches!(
            graph.validate(),
            Err(EngineError::InvalidLimits { .. })
        ));
    }

    #[test]
    fn validate_rejects_zero_max_node_visits() {
        let graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_node_visits: 0,
                ..EngineLimits::default()
            },
        );
        assert!(matches!(
            graph.validate(),
            Err(EngineError::InvalidLimits { .. })
        ));
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

    #[test]
    fn input_mapping_renders_missing_field_as_empty() {
        let battlefield = Battlefield::new(BattlefieldSchema::new(vec![]));
        let mapping = InputMapping::new("value=[{missing}]");
        assert_eq!(mapping.render(&battlefield), "value=[]");
    }
}
