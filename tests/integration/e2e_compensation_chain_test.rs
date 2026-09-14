//! PRD 04 section 3.5's two end-to-end fault-tolerance scenarios (FT-04, D-21, D-23; plan
//! 25-11), run against a real on-disk `SqliteWaypointStore` rather than an in-memory double:
//!
//! 1. **The compensation chain.** `book` is a Paladin node whose port fails with an
//!    authentication-classified `LlmFailure` (built from `LlmError::AuthenticationError`,
//!    `Transience::Permanent` by the typed table -- a message-only variant, so no HTTP
//!    status is fabricated for it). Under
//!    `Route { to: cancel, error_field: booking_error }` the structured `NodeError` is
//!    written into `booking_error`, `cancel` runs in the next Vanguard, and the run reaches
//!    `RunOutcome::Completed` with `book`'s record reading `outcome: Failed`. The default
//!    retry predicate refuses the permanent error, so `book` executes exactly once despite
//!    a retry policy being present (FT-FR-05).
//! 2. **The loop bound.** `a` routes to `b` on failure, `b` routes to `a`, both always fail,
//!    `max_node_visits = 3`: the run ends `EngineError::NodeVisitLimitExceeded` and never
//!    spins (FT-FR-15). A `tokio::time::timeout` guard turns a regression into a loud CI
//!    failure instead of a hung job, following
//!    `src/application/services/orchestration/listener.rs`'s timeout-guard convention.
//!
//! Plus D-23's "on payment failure, parley a human": a registered `Custom` handler returns
//! `NextStep::Parley`, the run suspends through the HITL-01 path, and -- after a simulated
//! process drop, exactly as `tests/integration/e2e_approval_gate_test.rs` stages it -- a
//! brand new engine over the same database file delivers the approval with `resume_with`
//! and drives the run to completion. The mirror case with the handler removed fails with a
//! `Failed` Waypoint whose `node_error` is `Some(..)` (FT-FR-14).
//!
//! `booking_error` is always compared as PARSED JSON fields, never as string bytes: the
//! serialized `NodeError` is a contract on its field values, not on serde's key order.
//!
//! These tests live under `tests/integration/`, so the repository's default `make test`
//! (`--lib --bins`) does not run them; run `cargo test --test e2e_compensation_chain`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;

use paladin_battalion::engine::{
    EngineError, EngineLimits, InputMapping, NodeContext, NodeSpec, RunOutcome, StateNode,
    StateNodeError, WarEngine, WarGraph,
};
use paladin_battalion::error_handler::ErrorHandler;
use paladin_battalion::llm_failure::to_paladin_error;
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::aegis::{Aegis, ErrorHandlerSpec, RetryPolicy};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::node_error::NodeError;
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::{
    OnExpire, ParleyId, ParleyKind, ParleyRequest, ParleyResponse,
};
use paladin_core::platform::container::transience::Transience;
use paladin_core::platform::container::waypoint::{
    NodeId, NodeOutcomeKind, ThreadId, WaypointStatus,
};
use paladin_ports::output::llm_port::LlmError;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

fn field(name: &str) -> FieldName {
    FieldName::new(name).expect("valid field name")
}

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "e2e_compensation_chain_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

fn make_paladin(name: &str) -> Paladin {
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

/// A `PaladinPort` whose every `execute` fails the way a real provider adapter fails on a
/// `401`: the exact `PaladinError::LlmFailure` `llm_failure::to_paladin_error` builds from
/// `LlmError::AuthenticationError` -- classified `Transience::Permanent` by the error's own
/// typed table, never by parsing a message (FT-FR-01, D-05). Counts its calls so a test can
/// prove the default predicate refused to retry it.
struct AuthFailingPaladinPort {
    calls: AtomicUsize,
}

impl AuthFailingPaladinPort {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl PaladinPort for AuthFailingPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(to_paladin_error(&LlmError::AuthenticationError(
            "invalid API key".to_string(),
        )))
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        Err(PaladinError::ExecutionError(
            "AuthFailingPaladinPort only supports execute()".to_string(),
        ))
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A `Function` node that always writes the same fixed value to one field -- the recovery
/// (`cancel`) node's effect this scenario asserts on. Counts its runs.
struct FixedOutputNode {
    field: FieldName,
    value: serde_json::Value,
    runs: AtomicUsize,
}

impl FixedOutputNode {
    fn new(field: FieldName, value: serde_json::Value) -> Arc<Self> {
        Arc::new(Self {
            field,
            value,
            runs: AtomicUsize::new(0),
        })
    }

    fn run_count(&self) -> usize {
        self.runs.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl StateNode for FixedOutputNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(delta.into())
    }
}

/// A `Function` node that always fails -- both halves of the compensation loop.
struct AlwaysFailingNode {
    message: String,
    runs: AtomicUsize,
}

impl AlwaysFailingNode {
    fn new(message: &str) -> Arc<Self> {
        Arc::new(Self {
            message: message.to_string(),
            runs: AtomicUsize::new(0),
        })
    }

    fn run_count(&self) -> usize {
        self.runs.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl StateNode for AlwaysFailingNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        Err(StateNodeError(self.message.clone()))
    }
}

/// The `payment` node: fails on every visit with no answer in scope (the raising visit),
/// and on the post-resume visit writes the delivered approval to `payment_status`. Records
/// the `ctx.attempt` of every run so the fresh-attempt-1 contract (Phase 24 D-07/D-08) is
/// asserted end to end, across the process drop.
struct PaymentNode {
    attempts: Mutex<Vec<u32>>,
}

impl PaymentNode {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            attempts: Mutex::new(Vec::new()),
        })
    }

    fn observed_attempts(&self) -> Vec<u32> {
        self.attempts.lock().expect("attempts lock").clone()
    }
}

#[async_trait]
impl StateNode for PaymentNode {
    async fn run(
        &self,
        _state: &Battlefield,
        ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        self.attempts
            .lock()
            .expect("attempts lock")
            .push(ctx.attempt);
        match ctx.parley_response() {
            None => Err(StateNodeError(
                "payment gateway rejected the charge".to_string(),
            )),
            Some(response) => {
                let approved = response.value.as_bool().unwrap_or(false);
                let mut delta = StateDelta::new();
                delta.set_raw(
                    field("payment_status"),
                    serde_json::json!(if approved { "approved" } else { "denied" }),
                );
                Ok(delta.into())
            }
        }
    }
}

/// The registered `ask-human` handler: asks for a manual approval of the failed payment,
/// carrying the failed node's structured error summary in the payload so the human sees
/// what went wrong (a redacted, bounded summary -- never a provider body, T-25-53).
struct AskHumanHandler {
    parley_id: ParleyId,
    invocations: AtomicUsize,
}

impl AskHumanHandler {
    fn new(parley_id: ParleyId) -> Arc<Self> {
        Arc::new(Self {
            parley_id,
            invocations: AtomicUsize::new(0),
        })
    }

    fn invocation_count(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ErrorHandler for AskHumanHandler {
    async fn handle(&self, err: &NodeError, _state: &Battlefield) -> Result<Directive, NodeError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        let created_at = Utc::now();
        Ok(Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(ParleyRequest {
                parley_id: self.parley_id,
                node_id: err.node_id.clone(),
                kind: ParleyKind::Approval,
                prompt: "payment failed -- approve a manual retry?".to_string(),
                payload: serde_json::json!({
                    "attempt": err.attempt,
                    "transience": err.transience,
                    "error": err.source.to_string(),
                }),
                choices: None,
                expires_at: None,
                created_at,
                on_expire: OnExpire::FailRun,
            }),
        })
    }
}

fn approval(parley_id: ParleyId, value: bool) -> ParleyResponse {
    ParleyResponse {
        parley_id,
        // `kind`/`prompt` are stamped over by `resume_with` regardless of
        // what is submitted here -- never observed.
        kind: ParleyKind::Approval,
        prompt: String::new(),
        value: serde_json::json!(value),
        responded_by: Some("tester".to_string()),
        responded_at: Utc::now(),
        defaulted: false,
    }
}

/// Jitter-free, 1 ms initial interval, `max_attempts` attempts under the DEFAULT
/// (`TransientOnly`) predicate -- so a `Permanent` failure gets exactly one attempt while
/// the policy is demonstrably present (FT-FR-05).
fn retry_policy(max_attempts: u32) -> RetryPolicy {
    RetryPolicy {
        max_attempts,
        jitter: false,
        initial_interval: Duration::from_millis(1),
        ..RetryPolicy::default()
    }
}

// --- The compensation chain (CONTEXT.md specifics, PRD 04 section 3.5) -----

/// `result` (book's own output), `booking_error` (the Route's `error_field`, `LastWrite` --
/// a serialized error object can never be `Sum`med) and `recovered` (what `cancel` writes).
fn compensation_schema() -> BattlefieldSchema {
    BattlefieldSchema::new(
        ["result", "booking_error", "recovered"]
            .into_iter()
            .map(|name| FieldSpec::new(field(name), DispatchRule::LastWrite, None, false))
            .collect(),
    )
}

/// `book` (Paladin, entry, `Aegis { retry: 3 attempts, on_error: Route { to: cancel,
/// error_field: booking_error } }`) with `cancel` declared but NOT statically wired --
/// reachable only by routing, exactly the CONTEXT.md shape.
fn compensation_graph(cancel: Arc<FixedOutputNode>) -> WarGraph {
    let mut graph = WarGraph::new(compensation_schema(), EngineLimits::default());
    graph.add_node(
        NodeId::new("book"),
        NodeSpec::paladin(
            make_paladin("book"),
            InputMapping::new("book the flight"),
            field("result"),
        ),
    );
    graph.add_node(NodeId::new("cancel"), NodeSpec::Function(cancel));
    graph.add_entry(NodeId::new("book"));
    graph.set_aegis(
        NodeId::new("book"),
        Aegis {
            retry: Some(retry_policy(3)),
            on_error: Some(ErrorHandlerSpec::Route {
                to: NodeId::new("cancel"),
                error_field: field("booking_error"),
            }),
            ..Aegis::default()
        },
    );
    graph
}

/// Runs the compensation chain over a fresh on-disk store; returns the outcome, the port's
/// call count, the recovery node's run count and the store (for record assertions).
async fn run_compensation_chain(
    label: &str,
) -> (
    RunOutcome,
    Arc<AuthFailingPaladinPort>,
    Arc<FixedOutputNode>,
    Arc<SqliteWaypointStore>,
    ThreadId,
) {
    let cancel = FixedOutputNode::new(field("recovered"), serde_json::json!("cancelled"));
    let graph = compensation_graph(cancel.clone());
    let port = AuthFailingPaladinPort::new();
    let store = Arc::new(
        SqliteWaypointStore::new(&temp_db_url(label))
            .await
            .expect("store should connect"),
    );
    let engine = WarEngine::new(port.clone(), store.clone());
    let thread = ThreadId::new(format!("e2e-compensation-{label}")).expect("valid thread id");
    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("a routed failure never errors out of start");
    (outcome, port, cancel, store, thread)
}

/// The `NodeExecutionRecord`s for `node` across every persisted Waypoint of `thread`, oldest
/// superstep first.
async fn records_for(
    store: &SqliteWaypointStore,
    thread: &ThreadId,
    node: &NodeId,
) -> Vec<paladin_core::platform::container::waypoint::NodeExecutionRecord> {
    let summaries = store
        .history(thread, None, None)
        .await
        .expect("history should succeed");
    let mut out = Vec::new();
    for summary in summaries.iter().rev() {
        let wp = store
            .get(thread, &summary.waypoint_id)
            .await
            .expect("get should succeed")
            .expect("listed waypoint exists");
        out.extend(wp.completed.iter().filter(|r| &r.node_id == node).cloned());
    }
    out
}

/// PRD 04 section 3.5, D-21: `book` fails permanently, routes to `cancel` with the structured
/// error in `booking_error`, `cancel` succeeds, and the run Completes with `book`'s record
/// reading `outcome: Failed`. Every `booking_error` assertion is on a parsed JSON field.
#[tokio::test]
async fn compensation_chain_routes_a_permanent_failure_to_a_recovery_node() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (outcome, _port, cancel, store, thread) = run_compensation_chain("chain").await;

        let final_state = match outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected Completed, got {other:?}"),
        };
        assert!(
            cancel.run_count() == 1,
            "the recovery node ran exactly once"
        );
        assert_eq!(
            final_state.get_raw(&field("recovered")),
            Some(&serde_json::json!("cancelled"))
        );

        let written = final_state
            .get_raw(&field("booking_error"))
            .expect("booking_error holds the routed NodeError");
        assert_eq!(written["node_id"], serde_json::json!("book"));
        assert_eq!(written["attempt"], serde_json::json!(1));
        assert_eq!(
            written["transience"],
            serde_json::to_value(Transience::Permanent).expect("transience serializes"),
            "the permanent value, compared as the serde representation of the typed enum"
        );
        let source = written["source"]
            .as_object()
            .expect("source is an externally tagged object");
        assert_eq!(source.len(), 1, "exactly one source arm: {source:?}");
        let (arm, inner) = source.iter().next().expect("one arm");
        assert_eq!(arm, "Llm", "an LlmFailure surfaces as NodeErrorSource::Llm");
        assert_eq!(inner["kind"], serde_json::json!("LlmFailure"));
        // `LlmError::AuthenticationError` is a message-only variant: the
        // authentication classification travels as `transience: Permanent`
        // (asserted above, by value), and no HTTP status is fabricated for
        // it -- only a `ProviderError` carries one (`llm_failure::typed_origin`).
        assert!(
            inner["status"].is_null(),
            "no status is invented for a message-only variant: {inner}"
        );
        assert!(
            inner["message"]
                .as_str()
                .expect("message is a string")
                .contains("invalid API key"),
            "the message chain reaches the provider's own text: {inner}"
        );
        // And it round-trips into the typed value.
        let parsed: NodeError =
            serde_json::from_value(written.clone()).expect("booking_error parses as a NodeError");
        assert_eq!(parsed.node_id, NodeId::new("book"));
        assert_eq!(parsed.attempt, 1);
        assert_eq!(parsed.transience, Transience::Permanent);

        let book_records = records_for(&store, &thread, &NodeId::new("book")).await;
        assert_eq!(book_records.len(), 1, "book ran in exactly one superstep");
        assert_eq!(book_records[0].outcome, NodeOutcomeKind::Failed);
        assert_eq!(book_records[0].attempt, 1);
        let cancel_records = records_for(&store, &thread, &NodeId::new("cancel")).await;
        assert_eq!(cancel_records.len(), 1);
        assert_eq!(cancel_records[0].outcome, NodeOutcomeKind::Succeeded);

        // The persisted trail ends Completed, never Failed.
        let latest = store
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("a waypoint exists");
        assert_eq!(latest.status, WaypointStatus::Completed);
    })
    .await
    .expect("compensation chain must complete within its 30s timeout guard");
}

/// FT-FR-05: the retry policy is present (3 attempts) but the DEFAULT predicate refuses a
/// `Permanent` error, so `book` executes exactly once before routing.
#[tokio::test]
async fn a_permanent_failure_is_not_retried_before_routing() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (outcome, port, _cancel, store, thread) = run_compensation_chain("no-retry").await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(
            port.call_count(),
            1,
            "a Permanent failure gets exactly one attempt under TransientOnly"
        );
        let book_records = records_for(&store, &thread, &NodeId::new("book")).await;
        assert_eq!(book_records[0].attempt, 1);
        assert!(
            book_records[0].attempts.is_empty(),
            "no retried attempt preceded the final one"
        );
    })
    .await
    .expect("no-retry scenario must complete within its 30s timeout guard");
}

// --- The loop bound (CONTEXT.md specifics, FT-FR-15) --------------------------

/// FT-FR-15, D-21: `a` routes to `b` on failure, `b` routes to `a`, both always fail,
/// `max_node_visits = 3` -- the run ends `EngineError::NodeVisitLimitExceeded` within a
/// bounded number of supersteps. The outer `tokio::time::timeout` guard is the point: a
/// regression that spins fails loudly here instead of hanging CI.
#[tokio::test]
async fn compensation_loop_terminates_at_the_visit_limit() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("booking_error"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(
            schema,
            EngineLimits {
                max_node_visits: 3,
                ..EngineLimits::default()
            },
        );
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let a_node = AlwaysFailingNode::new("a always fails");
        let b_node = AlwaysFailingNode::new("b always fails");
        graph.add_node(a.clone(), NodeSpec::Function(a_node.clone()));
        graph.add_node(b.clone(), NodeSpec::Function(b_node.clone()));
        graph.add_entry(a.clone());
        for (from, to) in [(&a, &b), (&b, &a)] {
            graph.set_aegis(
                from.clone(),
                Aegis {
                    on_error: Some(ErrorHandlerSpec::Route {
                        to: to.clone(),
                        error_field: field("booking_error"),
                    }),
                    ..Aegis::default()
                },
            );
        }

        let store = Arc::new(
            SqliteWaypointStore::new(&temp_db_url("loop"))
                .await
                .expect("store should connect"),
        );
        let engine = WarEngine::new(AuthFailingPaladinPort::new(), store.clone());
        let thread = ThreadId::new("e2e-compensation-loop").expect("valid thread id");

        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("a visit-limit failure is a RunOutcome, not an Err");
        match &outcome {
            RunOutcome::Failed {
                error: EngineError::NodeVisitLimitExceeded { limit, .. },
                ..
            } => assert_eq!(*limit, 3),
            other => panic!("expected Failed(NodeVisitLimitExceeded), got {other:?}"),
        }
        // a(1) b(1) a(2) b(2) a(3 -> trips): bounded, never a spin.
        assert!(
            a_node.run_count() + b_node.run_count() <= 5,
            "bounded: a ran {} times, b ran {} times",
            a_node.run_count(),
            b_node.run_count()
        );
        let history = store
            .history(&thread, None, None)
            .await
            .expect("history should succeed");
        assert!(history.len() <= 6, "bounded: {} waypoints", history.len());
        let latest = store
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("a waypoint exists");
        assert!(matches!(latest.status, WaypointStatus::Failed { .. }));
    })
    .await
    .expect("a compensation loop must terminate at the visit limit, never spin");
}

// --- "On payment failure, parley a human" (D-23) --------------------------------

fn payment_schema() -> BattlefieldSchema {
    BattlefieldSchema::new(vec![FieldSpec::new(
        field("payment_status"),
        DispatchRule::LastWrite,
        None,
        false,
    )])
}

/// `payment` (entry) under `aegis` -- with `on_error: Custom("ask-human")` for the round
/// trip, or with the handler removed for the mirror case.
fn payment_graph(payment: Arc<PaymentNode>, aegis: Aegis) -> WarGraph {
    let mut graph = WarGraph::new(payment_schema(), EngineLimits::default());
    graph.add_node(NodeId::new("payment"), NodeSpec::Function(payment));
    graph.add_entry(NodeId::new("payment"));
    graph.set_aegis(NodeId::new("payment"), aegis);
    graph
}

/// D-23, HITL-01/02: a failing payment node's registered handler returns `NextStep::Parley`;
/// the run suspends (`RunOutcome::AwaitingInput`, one persisted `AwaitingInput` Waypoint),
/// engine instance A is dropped, and a brand new instance B over the SAME database file
/// delivers the approval through `resume_with` -- the payment node re-runs as a fresh
/// attempt 1 with the answer in scope and the run completes.
#[tokio::test]
async fn on_payment_failure_parley_a_human() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let db_url = temp_db_url("payment-parley");
        let parley_id = ParleyId::new();
        let payment = PaymentNode::new();
        let graph = payment_graph(
            payment.clone(),
            Aegis {
                on_error: Some(ErrorHandlerSpec::Custom("ask-human".to_string())),
                ..Aegis::default()
            },
        );
        let thread = ThreadId::new("e2e-payment-parley").expect("valid thread id");

        // --- Instance A: start, suspend at the handler's parley, drop. --
        {
            let handler_a = AskHumanHandler::new(parley_id);
            let store_a = Arc::new(
                SqliteWaypointStore::new(&db_url)
                    .await
                    .expect("store A should connect"),
            );
            let engine_a = WarEngine::new(AuthFailingPaladinPort::new(), store_a.clone())
                .with_error_handler("ask-human", handler_a.clone());

            let outcome = engine_a
                .start(&graph, thread.clone(), StateDelta::new())
                .await
                .expect("start should suspend at the handler's parley");
            match outcome {
                RunOutcome::AwaitingInput { parleys, .. } => {
                    assert_eq!(parleys.len(), 1, "exactly one handler-raised parley");
                    assert_eq!(parleys[0].parley_id, parley_id);
                    assert_eq!(parleys[0].node_id, NodeId::new("payment"));
                    assert_eq!(
                        parleys[0].payload["error"],
                        serde_json::json!("function error: payment gateway rejected the charge")
                    );
                }
                other => panic!("expected AwaitingInput, got {other:?}"),
            }
            assert_eq!(handler_a.invocation_count(), 1);
            assert_eq!(payment.observed_attempts(), vec![1]);

            let history = store_a
                .history(&thread, None, None)
                .await
                .expect("history should succeed");
            assert_eq!(
                history.len(),
                1,
                "exactly one Waypoint persisted at suspension"
            );
            let latest = store_a
                .latest(&thread)
                .await
                .expect("latest should succeed")
                .expect("latest waypoint must exist");
            match &latest.status {
                WaypointStatus::AwaitingInput { parleys, responses } => {
                    assert_eq!(parleys.len(), 1);
                    assert_eq!(parleys[0].parley_id, parley_id);
                    assert!(responses.is_empty());
                }
                other => panic!("expected AwaitingInput, got {other:?}"),
            }
            assert_eq!(latest.vanguard, vec![NodeId::new("payment")]);
            // engine_a, handler_a and store_a are dropped here -- instance
            // A's process is simulated as gone before B is constructed.
        }

        // --- Instance B: a brand new engine, handler AND store, same file. --
        let handler_b = AskHumanHandler::new(parley_id);
        let store_b = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("store B should reconnect to the same file"),
        );
        let engine_b = WarEngine::new(AuthFailingPaladinPort::new(), store_b.clone())
            .with_error_handler("ask-human", handler_b.clone());

        let resumed = engine_b
            .resume_with(&graph, thread.clone(), vec![approval(parley_id, true)])
            .await
            .expect("resume_with should complete the run");
        let final_state = match resumed {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected Completed, got {other:?}"),
        };
        assert_eq!(
            final_state.get_raw(&field("payment_status")),
            Some(&serde_json::json!("approved"))
        );
        assert_eq!(
            payment.observed_attempts(),
            vec![1, 1],
            "the post-resume re-run is a fresh attempt 1 (Phase 24 D-07/D-08)"
        );
        assert_eq!(
            handler_b.invocation_count(),
            0,
            "the answered re-run succeeded: the handler was not asked again"
        );
        let latest = store_b
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("latest waypoint must exist");
        assert_eq!(latest.status, WaypointStatus::Completed);
    })
    .await
    .expect("payment-parley scenario must complete within its 30s timeout guard");
}

/// FT-FR-14, D-08: the same payment graph with the handler REMOVED (the Aegis stays, so the
/// failure is Aegis-governed) fails the run with a `Failed` Waypoint whose `node_error` is
/// `Some(..)` -- the structured error, never only a string -- and `RunOutcome::Failed`
/// carries the same value.
#[tokio::test]
async fn the_failed_waypoint_of_a_handler_less_run_carries_the_structured_error() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let payment = PaymentNode::new();
        let graph = payment_graph(
            payment.clone(),
            Aegis {
                retry: Some(retry_policy(2)),
                on_error: None,
                ..Aegis::default()
            },
        );
        let store = Arc::new(
            SqliteWaypointStore::new(&temp_db_url("payment-no-handler"))
                .await
                .expect("store should connect"),
        );
        let engine = WarEngine::new(AuthFailingPaladinPort::new(), store.clone());
        let thread = ThreadId::new("e2e-payment-no-handler").expect("valid thread id");

        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("a node failure is a RunOutcome, not an Err");
        let run_error = match &outcome {
            RunOutcome::Failed {
                error: EngineError::NodeFailed(node_error),
                ..
            } => node_error.clone(),
            other => panic!("expected Failed(NodeFailed), got {other:?}"),
        };
        assert_eq!(run_error.node_id, NodeId::new("payment"));
        assert_eq!(
            run_error.attempt, 1,
            "a Function failure classifies Unknown, which TransientOnly refuses"
        );

        let latest = store
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("latest waypoint must exist");
        match &latest.status {
            WaypointStatus::Failed {
                failed_node,
                node_error,
                ..
            } => {
                assert_eq!(failed_node, &NodeId::new("payment"));
                let persisted = node_error
                    .as_ref()
                    .expect("the Failed Waypoint carries the structured error");
                assert_eq!(persisted, &run_error, "one value on both surfaces");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        assert_eq!(
            store
                .history(&thread, None, None)
                .await
                .expect("history should succeed")
                .len(),
            1,
            "no suspension, no routing: exactly one Waypoint"
        );
    })
    .await
    .expect("handler-less scenario must complete within its 30s timeout guard");
}
