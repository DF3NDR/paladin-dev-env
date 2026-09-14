//! Shared E2E-1/E2E-2/E2E-3 graph builders (`.project/v0.10.0/00-program-overview.md` §6,
//! plan 28-16, D-34).
//!
//! These are the SAME graph constructors `tests/integration/e2e_crash_resume_test.rs`,
//! `tests/integration/e2e_approval_gate_test.rs` and
//! `tests/integration/e2e_muster_defer_order_test.rs` used inline before this extraction --
//! moved here verbatim (a move, not a redesign) so both the integration tests AND the eval
//! harness (`tests/evals.rs`, registered via `ScenarioRunner::register_graph`) share one
//! definition (`e2e_fixtures_are_the_only_definition`). Neither consumer needs a graph
//! builder parameterised on ports: every node in all three graphs is either a `Function`
//! node (driving control flow off durable `Battlefield` state, never off in-process memory)
//! or a `Paladin`/worker-template node whose LLM substitution happens entirely at the
//! `PaladinPort` level the ENGINE is constructed with (`FaultyPaladinPort` for the
//! integration tests, `ScenarioPaladinPort` for the eval harness) -- so the graph shape
//! itself never needs to know which port implementation its caller supplies.
//!
//! `paladin_battalion`, `paladin_core`, `paladin_ports` and `async-trait` are unconditional
//! `[dependencies]` of the root `paladin` facade crate (never feature-gated), so this file
//! compiles identically whether pulled in from `tests/` (via the usual
//! `#[path = "../helpers/mod.rs"] mod helpers;` pattern every E2E integration test already
//! uses) or from `src/application/cli/commands/eval.rs` (via an explicit cross-directory
//! `#[path]` inclusion, gated by the `cli` feature at `eval.rs`'s own module declaration) --
//! the one path `paladin-cli eval run` needs to resolve the SAME `registered` targets
//! `tests/evals.rs` registers, without duplicating the graph-building logic a third time.

use std::sync::Arc;
use std::time::Duration;

use paladin_battalion::engine::InputMapping;
use paladin_battalion::engine::graph::GateRequestTemplate;
use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, NodeContext, NodeSpec, StateNode, StateNodeError, WarGraph,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::aegis::{Aegis, RetryPolicy};
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, MusterTask, NextStep};
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::parley::ParleyKind;
use paladin_core::platform::container::waypoint::NodeId;

/// A `Battlefield` field name, panicking only on a malformed literal (test-only helper,
/// mirrors every E2E fixture's own local `field()` before this extraction).
pub fn field(name: &str) -> FieldName {
    FieldName::new(name).expect("valid field name")
}

/// A minimal `Paladin` whose own `PaladinData.name` equals `name` -- the identity both
/// `FaultyPaladinPort` (integration tests) and `ScenarioPaladinPort` (eval harness) route
/// on.
pub fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("{name} prompt"),
        name: name.to_string(),
        user_name: "TestUser".to_string(),
        model: "test-model".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

// ---------------------------------------------------------------------------
// E2E-1: crash-resume (tests/integration/e2e_crash_resume_test.rs)
// ---------------------------------------------------------------------------

/// The loop bound: `loop_gate` must run exactly this many times before its
/// `loop_status` flips from `"continue"` to `"done"`. Deliberately > 3 so dropping "after
/// superstep 3" (E2E-1's own scenario text) lands MID-loop, not after it -- the harder,
/// more interesting crash-resume case.
pub const LOOP_BOUND: i64 = 5;

/// A deterministic `Function` node driving the graph's one cycle entirely off durable
/// `Battlefield` state (never off its own in-process memory) -- the property that makes it
/// safe to resume: a freshly constructed `LoopGateNode` in a brand new graph instance
/// continues counting from wherever the restored `loop_count` field left off.
struct LoopGateNode;

#[async_trait::async_trait]
impl StateNode for LoopGateNode {
    async fn run(
        &self,
        state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let count_field = field("loop_count");
        let status_field = field("loop_status");
        let current = state
            .get::<i64>(&count_field)
            .map_err(|e| StateNodeError(e.to_string()))?
            .unwrap_or(0);
        let next = current + 1;
        let status = if next < LOOP_BOUND {
            "continue"
        } else {
            "done"
        };

        let mut delta = StateDelta::new();
        delta
            .set(count_field, next)
            .map_err(|e| StateNodeError(e.to_string()))?;
        delta
            .set(status_field, status)
            .map_err(|e| StateNodeError(e.to_string()))?;
        Ok(delta.into())
    }
}

/// Build the E2E-1 fixture: 6 nodes (5 Paladin, 1 Function), one bounded self-loop.
///
/// `loop_gate` (self-loop, bounded, GRAPH ENTRY) `-> researcher -> writer -> reviewer ->
/// finalizer -> archiver`. An uninterrupted run takes exactly `LOOP_BOUND + 5` supersteps.
/// See `tests/integration/e2e_crash_resume_test.rs`'s own (unmoved) module doc comment for
/// why `loop_gate` is the graph's entry rather than fed by an upstream node.
pub fn build_crash_resume_graph() -> WarGraph {
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(field("topic"), DispatchRule::LastWrite, None, true),
        FieldSpec::new(field("research_out"), DispatchRule::LastWrite, None, false),
        FieldSpec::new(field("writer_out"), DispatchRule::LastWrite, None, false),
        FieldSpec::new(
            field("loop_count"),
            DispatchRule::LastWrite,
            Some(serde_json::json!(0)),
            false,
        ),
        FieldSpec::new(
            field("loop_status"),
            DispatchRule::LastWrite,
            Some(serde_json::json!("pending")),
            false,
        ),
        FieldSpec::new(field("reviewer_out"), DispatchRule::LastWrite, None, false),
        FieldSpec::new(field("finalizer_out"), DispatchRule::LastWrite, None, false),
        FieldSpec::new(field("archiver_out"), DispatchRule::LastWrite, None, false),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());

    let researcher = NodeId::new("researcher");
    let writer = NodeId::new("writer");
    let loop_gate = NodeId::new("loop_gate");
    let reviewer = NodeId::new("reviewer");
    let finalizer = NodeId::new("finalizer");
    let archiver = NodeId::new("archiver");

    graph.add_node(
        researcher.clone(),
        NodeSpec::paladin(
            make_paladin("researcher"),
            InputMapping::new("{topic}"),
            field("research_out"),
        ),
    );
    graph.add_node(
        writer.clone(),
        NodeSpec::paladin(
            make_paladin("writer"),
            InputMapping::new("{research_out}"),
            field("writer_out"),
        ),
    );
    graph.add_node(
        loop_gate.clone(),
        NodeSpec::Function(Arc::new(LoopGateNode)),
    );
    graph.add_node(
        reviewer.clone(),
        NodeSpec::paladin(
            make_paladin("reviewer"),
            InputMapping::new("{writer_out}"),
            field("reviewer_out"),
        ),
    );
    graph.add_node(
        finalizer.clone(),
        NodeSpec::paladin(
            make_paladin("finalizer"),
            InputMapping::new("{reviewer_out}"),
            field("finalizer_out"),
        ),
    );
    graph.add_node(
        archiver.clone(),
        NodeSpec::paladin(
            make_paladin("archiver"),
            InputMapping::new("{finalizer_out}"),
            field("archiver_out"),
        ),
    );

    graph.add_edge(EdgeSpec {
        from: loop_gate.clone(),
        to: loop_gate.clone(),
        condition: Some(EdgeCondition::Contains(
            "\"loop_status\":\"continue\"".to_string(),
        )),
    });
    graph.add_edge(EdgeSpec {
        from: loop_gate.clone(),
        to: researcher.clone(),
        condition: Some(EdgeCondition::Contains(
            "\"loop_status\":\"done\"".to_string(),
        )),
    });
    graph.add_edge(EdgeSpec {
        from: researcher.clone(),
        to: writer.clone(),
        condition: None,
    });
    graph.add_edge(EdgeSpec {
        from: writer.clone(),
        to: reviewer.clone(),
        condition: None,
    });
    graph.add_edge(EdgeSpec {
        from: reviewer.clone(),
        to: finalizer.clone(),
        condition: None,
    });
    graph.add_edge(EdgeSpec {
        from: finalizer.clone(),
        to: archiver.clone(),
        condition: None,
    });

    graph.add_entry(loop_gate);
    graph
}

// ---------------------------------------------------------------------------
// E2E-2: approval gate (tests/integration/e2e_approval_gate_test.rs)
// ---------------------------------------------------------------------------

/// A `Function` node that always writes the same fixed value to one field, ignoring the
/// observed `Battlefield` -- the `act`/`cancel` branch effects E2E-2 asserts on.
struct FixedOutputNode {
    field: FieldName,
    value: serde_json::Value,
}

impl FixedOutputNode {
    fn new(field: FieldName, value: serde_json::Value) -> Arc<Self> {
        Arc::new(Self { field, value })
    }
}

#[async_trait::async_trait]
impl StateNode for FixedOutputNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(delta.into())
    }
}

/// Build the E2E-2 fixture: one `NodeSpec::Gate` (`Approval`, `output_field: "approved"`)
/// plus a `Contains("true")` edge to `act` and a `Contains("false")` edge to `cancel` -- the
/// exact "three lines of graph" shape the PRD promises.
pub fn build_approval_gate_graph() -> WarGraph {
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(
            field("approved"),
            DispatchRule::LastWrite,
            Some(serde_json::json!(false)),
            false,
        ),
        FieldSpec::new(field("path"), DispatchRule::LastWrite, None, false),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());

    let request = GateRequestTemplate::new(
        ParleyKind::Approval,
        InputMapping::new("Approve the deploy?"),
    );
    let approve = NodeId::new("approve");
    let act = NodeId::new("act");
    let cancel = NodeId::new("cancel");

    graph.add_node(
        approve.clone(),
        NodeSpec::gate(request, Some(field("approved"))),
    );
    graph.add_node(
        act.clone(),
        NodeSpec::Function(FixedOutputNode::new(
            field("path"),
            serde_json::json!("act"),
        )),
    );
    graph.add_node(
        cancel.clone(),
        NodeSpec::Function(FixedOutputNode::new(
            field("path"),
            serde_json::json!("cancel"),
        )),
    );

    graph.add_edge(EdgeSpec {
        from: approve.clone(),
        to: act,
        condition: Some(EdgeCondition::Contains(r#""approved":true"#.to_string())),
    });
    graph.add_edge(EdgeSpec {
        from: approve.clone(),
        to: cancel,
        condition: Some(EdgeCondition::Contains(r#""approved":false"#.to_string())),
    });
    graph.add_entry(approve);
    graph
}

// ---------------------------------------------------------------------------
// E2E-3: muster/defer/order + recovering worker
// (tests/integration/e2e_muster_defer_order_test.rs)
// ---------------------------------------------------------------------------

/// The five mustered task keys, already in lexicographic (`String` byte) order -- CF-FR-11's
/// ordering guarantee.
pub const TASK_KEYS: [&str; 5] = ["a", "b", "c", "d", "e"];

/// The five worker templates of the recovering-worker fixture, one per task key (`w1` runs
/// `"a"`, .., `w5` runs `"e"`); `w3` is the one that fails its first two attempts (D-31).
pub const WORKER_NAMES: [&str; 5] = ["w1", "w2", "w3", "w4", "w5"];

/// The recovering worker of the E2E-3 scenario.
pub const RECOVERING_WORKER: &str = "w3";

/// Deterministic planner: on its one (and only) execution, musters the configured worker
/// tasks -- each `(worker template, task_key)` pair carrying its own key as a JSON string
/// payload (`{muster.payload}` resolves to the bare key string).
struct PlannerNode {
    tasks: Vec<MusterTask>,
}

impl PlannerNode {
    fn muster(assignments: impl IntoIterator<Item = (NodeId, &'static str)>) -> Self {
        Self {
            tasks: assignments
                .into_iter()
                .map(|(worker, key)| MusterTask {
                    worker,
                    payload: serde_json::json!(key),
                    task_key: key.to_string(),
                })
                .collect(),
        }
    }

    /// Five tasks, keyed `"a"`..`"e"`, all against the single `worker` template -- the
    /// original E2E-3 muster/defer/order fixture.
    fn single_template() -> Self {
        let worker = NodeId::new("worker");
        Self::muster(TASK_KEYS.iter().map(|key| (worker.clone(), *key)))
    }

    /// Five tasks, keyed `"a"`..`"e"`, each against its own `w1`..`w5` template -- the
    /// recovering-worker fixture.
    fn one_template_per_task() -> Self {
        Self::muster(
            WORKER_NAMES
                .iter()
                .zip(TASK_KEYS.iter())
                .map(|(worker, key)| (NodeId::new(*worker), *key)),
        )
    }
}

#[async_trait::async_trait]
impl StateNode for PlannerNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Ok(Directive {
            delta: StateDelta::new(),
            next: NextStep::Muster(self.tasks.clone()),
        })
    }
}

/// Deferred aggregator (`defer: true`): reads the worker template's `Append`-dispatched
/// `worker_out` field -- exactly 5 entries once every mustered task has resolved -- and
/// copies it, unchanged, into `aggregated`, the list-dispatch `Battlefield` field D-17
/// names.
struct AggregatorNode {
    worker_out: FieldName,
    aggregated: FieldName,
}

#[async_trait::async_trait]
impl StateNode for AggregatorNode {
    async fn run(
        &self,
        state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let results = state
            .get::<Vec<String>>(&self.worker_out)
            .map_err(|e| StateNodeError(e.to_string()))?
            .unwrap_or_default();
        let mut delta = StateDelta::new();
        delta.set_raw(self.aggregated.clone(), serde_json::json!(results));
        Ok(delta.into())
    }
}

fn muster_defer_order_schema() -> BattlefieldSchema {
    BattlefieldSchema::new(vec![
        FieldSpec::new(field("worker_out"), DispatchRule::Append, None, false),
        FieldSpec::new(field("aggregated"), DispatchRule::LastWrite, None, false),
    ])
}

/// Build the E2E-3 muster/defer/order fixture: `planner` (Function, entry, one-shot
/// `Muster` of 5 tasks) `-> worker` (Paladin worker template, no static incoming edge --
/// dispatched only when mustered, D-12) `-> aggregator` (Function, `defer: true`, runs once
/// after all 5 resolve).
pub fn build_muster_defer_order_graph() -> WarGraph {
    let worker_out = field("worker_out");
    let aggregated = field("aggregated");
    let mut graph = WarGraph::new(muster_defer_order_schema(), EngineLimits::default());

    let planner = NodeId::new("planner");
    let worker = NodeId::new("worker");
    let aggregator = NodeId::new("aggregator");

    graph.add_node(
        planner.clone(),
        NodeSpec::Function(Arc::new(PlannerNode::single_template())),
    );
    graph.add_worker_template(
        worker.clone(),
        NodeSpec::paladin(
            make_paladin("worker"),
            InputMapping::new("{muster.payload}"),
            worker_out.clone(),
        ),
    );
    graph.add_deferred_node(
        aggregator.clone(),
        NodeSpec::Function(Arc::new(AggregatorNode {
            worker_out: worker_out.clone(),
            aggregated: aggregated.clone(),
        })),
    );
    graph.add_edge(EdgeSpec {
        from: worker.clone(),
        to: aggregator.clone(),
        condition: None,
    });
    graph.add_entry(planner);

    graph
}

/// The per-task retry policy of the recovering-worker fixture (D-31): `max_attempts: 3` so
/// `w3` may fail twice and succeed on its third attempt, and `retry_on` LEFT AT ITS DEFAULT
/// (`TransientOnly`) -- the scenario passes because the failure is Transient by value, never
/// because the predicate was widened. Only the backoff interval is shortened.
pub fn per_task_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 3,
        initial_interval: Duration::from_millis(20),
        ..RetryPolicy::default()
    }
}

/// Build the E2E-3 recovering-worker fixture: `planner` (Function, entry, one-shot `Muster`
/// of 5 tasks, one per template) `-> w1..w5` (five Paladin worker templates, each carrying
/// `retry` as its Aegis retry policy when `Some`, no static incoming edges) `-> aggregator`
/// (Function, `defer: true`, runs once after all 5 resolve). `None` attaches no Aegis at
/// all -- the negative control that proves the green scenario depends on the retry.
pub fn build_muster_defer_order_graph_with_a_template_per_task(
    retry: Option<RetryPolicy>,
) -> WarGraph {
    let worker_out = field("worker_out");
    let aggregated = field("aggregated");
    let mut graph = WarGraph::new(muster_defer_order_schema(), EngineLimits::default());

    let planner = NodeId::new("planner");
    let aggregator = NodeId::new("aggregator");

    graph.add_node(
        planner.clone(),
        NodeSpec::Function(Arc::new(PlannerNode::one_template_per_task())),
    );
    graph.add_deferred_node(
        aggregator.clone(),
        NodeSpec::Function(Arc::new(AggregatorNode {
            worker_out: worker_out.clone(),
            aggregated: aggregated.clone(),
        })),
    );
    for name in WORKER_NAMES {
        let worker = NodeId::new(name);
        graph.add_worker_template(
            worker.clone(),
            NodeSpec::paladin(
                make_paladin(name),
                InputMapping::new("{muster.payload}"),
                worker_out.clone(),
            ),
        );
        if let Some(policy) = &retry {
            graph.set_aegis(
                worker.clone(),
                Aegis {
                    retry: Some(policy.clone()),
                    ..Aegis::default()
                },
            );
        }
        graph.add_edge(EdgeSpec {
            from: worker,
            to: aggregator.clone(),
            condition: None,
        });
    }
    graph.add_entry(planner);

    graph
}
