// examples/human_in_the_loop_gate.rs
//
// Human-in-the-Loop Gate: Pause, Resume, Replay (EX-71, EX-72, EX-73)
//
// This example demonstrates the WarEngine's human-in-the-loop capability
// cluster as a single runnable story. It shows how to:
// 1. Pause at a Gate node -- run a small graph whose entry point is a
//    NodeSpec::Gate and let the engine itself raise the pause, rather than
//    simulating one.
// 2. Resume with typed responses -- call WarEngine::resume_with with the
//    responses the gate awaited, and see the total-validation rule reject a
//    resume that withholds the real awaited response in favor of an
//    unrelated parley id, before accepting the correct one.
// 3. Replay the thread -- construct a ChronicleService over the same
//    Waypoint store, read the thread's history back, then replay the run
//    from its pause point onto a NEW branch and resume that branch with the
//    opposite decision, showing the branch's result diverges from the
//    mainline's.
//
// This example is fully offline: its graph runs Function and Gate nodes
// only (no Paladin node, so no LLM call is ever made), it uses only
// in-memory, in-process state, reads no LLM provider API key from the
// environment, and needs no external service.
//
// To run this example:
// ```bash
// cargo run --example human_in_the_loop_gate
// ```

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use paladin::application::services::chronicle::ChronicleService;
use paladin_battalion::engine::graph::{
    EdgeSpec, EngineLimits, GateRequestTemplate, NodeSpec, WarGraph,
};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_battalion::engine::{InputMapping, RunOutcome, WarEngine};
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::{ParleyId, ParleyKind, ParleyResponse};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// `WarEngine::new` requires a `PaladinPort`; this program's graph is built
/// entirely from `Function` and `Gate` nodes, so this is never actually
/// invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this program's WarGraph runs Function and Gate nodes only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this program's WarGraph runs Function and Gate nodes only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A pure `StateNode` that writes one fixed value to one field, then routes
/// via its static outgoing edges. The smallest shape for the two branches
/// (`act`/`cancel`) downstream of the Gate.
struct SetField {
    field: FieldName,
    value: serde_json::Value,
}

#[async_trait]
impl StateNode for SetField {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(Directive {
            delta,
            next: NextStep::Edges,
        })
    }
}

/// Build the graph every part of this program runs: one Approval Gate
/// ("approve"), then two branches ("act"/"cancel") selected by the gate's
/// own `approved` output field.
fn build_graph(limits: EngineLimits) -> Result<WarGraph, Box<dyn std::error::Error>> {
    let approved = FieldName::new("approved")?;
    let path = FieldName::new("path")?;
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(
            approved.clone(),
            DispatchRule::LastWrite,
            Some(serde_json::json!(false)),
            false,
        ),
        FieldSpec::new(path.clone(), DispatchRule::LastWrite, None, false),
    ]);

    let mut graph = WarGraph::new(schema, limits);
    let request = GateRequestTemplate::new(
        ParleyKind::Approval,
        InputMapping::new("Deploy the release to production?"),
    );
    graph.add_node(
        NodeId::new("approve"),
        NodeSpec::gate(request, Some(approved)),
    );
    graph.add_node(
        NodeId::new("act"),
        NodeSpec::Function(Arc::new(SetField {
            field: path.clone(),
            value: serde_json::json!("act: release deployed"),
        })),
    );
    graph.add_node(
        NodeId::new("cancel"),
        NodeSpec::Function(Arc::new(SetField {
            field: path.clone(),
            value: serde_json::json!("cancel: release withheld"),
        })),
    );
    graph.add_edge(EdgeSpec {
        from: NodeId::new("approve"),
        to: NodeId::new("act"),
        condition: Some(EdgeCondition::Contains(r#""approved":true"#.to_string())),
    });
    graph.add_edge(EdgeSpec {
        from: NodeId::new("approve"),
        to: NodeId::new("cancel"),
        condition: Some(EdgeCondition::Contains(r#""approved":false"#.to_string())),
    });
    graph.add_entry(NodeId::new("approve"));

    Ok(graph)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Human-in-the-Loop Gate: Pause, Resume, Replay\n");

    let path_field = FieldName::new("path")?;
    let store = Arc::new(InMemoryWaypointStore::new());
    let engine = WarEngine::new(Arc::new(UnusedPaladinPort), store.clone());
    let graph = build_graph(EngineLimits::default())?;
    let thread = ThreadId::new("human-in-the-loop-gate")?;

    // ------------------------------------------------------------------------------
    // Part 1 -- pause at a Gate (EX-71).
    // ------------------------------------------------------------------------------
    println!("1. Pause at a Gate (EX-71)\n");

    let outcome = engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await?;
    let (parley, pause_waypoint) = match outcome {
        RunOutcome::AwaitingInput { parleys, waypoint } => {
            let parley = parleys
                .into_iter()
                .next()
                .ok_or("the gate must raise exactly one parley")?;
            (parley, waypoint)
        }
        other => return Err(format!("expected AwaitingInput, got {other:?}").into()),
    };
    println!("   thread         = {thread}");
    println!("   paused at node = {}", parley.node_id);
    println!(
        "   awaiting       = {:?} -> \"{}\"",
        parley.kind, parley.prompt
    );
    println!("   parley id      = {}\n", parley.parley_id);

    // ------------------------------------------------------------------------------
    // Part 2 -- resume with typed responses (EX-72).
    // ------------------------------------------------------------------------------
    println!("2. Resume with typed responses (EX-72)\n");

    // Withhold the real awaited response and submit one naming an unrelated
    // parley id instead -- total validation (D-10) rejects this BEFORE any
    // state changes, and no Waypoint is written for the attempt.
    let bogus_response = ParleyResponse {
        parley_id: ParleyId::new(),
        kind: ParleyKind::Approval,
        prompt: String::new(),
        value: serde_json::json!(true),
        responded_by: Some("demo-operator".to_string()),
        responded_at: Utc::now(),
        defaulted: false,
    };
    match engine
        .resume_with(&graph, thread.clone(), vec![bogus_response])
        .await
    {
        Err(err) => println!(
            "   withheld the real response, submitted an unknown parley id instead:\n     -> typed rejection: {err}\n"
        ),
        Ok(unexpected) => println!(
            "   UNEXPECTED: resume with an unknown parley id was accepted: {unexpected:?}\n"
        ),
    }

    // Now submit the real response for the real parley id -- the engine
    // accepts it, writes a new Waypoint, and the run completes.
    let approve_response = ParleyResponse {
        parley_id: parley.parley_id,
        kind: ParleyKind::Approval,
        prompt: String::new(),
        value: serde_json::json!(true),
        responded_by: Some("demo-operator".to_string()),
        responded_at: Utc::now(),
        defaulted: false,
    };
    let completed = engine
        .resume_with(&graph, thread.clone(), vec![approve_response])
        .await?;
    let mainline_path = match completed {
        RunOutcome::Completed {
            final_state,
            waypoint,
        } => {
            let path: Option<String> = final_state.get(&path_field)?;
            println!("   accepted the correct response -> thread resumed and completed");
            println!("   final waypoint = {waypoint}");
            println!("   path           = {path:?}\n");
            path.ok_or("path field must be set on completion")?
        }
        other => return Err(format!("expected Completed, got {other:?}").into()),
    };

    // ------------------------------------------------------------------------------
    // Part 3 -- replay the thread (EX-73).
    // ------------------------------------------------------------------------------
    println!("3. Replay the thread (EX-73)\n");

    let chronicle = ChronicleService::new(store.clone() as Arc<dyn WaypointPort>);
    let history = chronicle.history(&thread, 10, None).await?;
    println!(
        "   chronicle history ({} waypoint(s), newest first):",
        history.len()
    );
    for summary in &history {
        println!(
            "     superstep {} -> {:?} (fork_of {:?})",
            summary.superstep, summary.status, summary.fork_of
        );
    }
    println!();

    // Replay from the ORIGINAL pause point onto a NEW branch. A Gate has no
    // `run` body of its own and carries no caller response across a
    // replay/fork boundary (only `WarEngine::resume_with` does that) -- so
    // the replayed branch pauses again, raising a fresh parley id.
    let replay_outcome = engine.replay(&graph, &thread, pause_waypoint).await?;
    let branch_parley = match replay_outcome {
        RunOutcome::AwaitingInput { parleys, .. } => parleys
            .into_iter()
            .next()
            .ok_or("the replayed gate must raise exactly one parley")?,
        other => {
            return Err(
                format!("expected the replayed branch to pause again, got {other:?}").into(),
            );
        }
    };
    println!(
        "   replayed from pause waypoint {pause_waypoint} -> new branch, new parley id {}",
        branch_parley.parley_id
    );

    // Resume the NEW branch with the OPPOSITE decision, proving the branch
    // is independent of the (already-completed) mainline.
    let deny_response = ParleyResponse {
        parley_id: branch_parley.parley_id,
        kind: ParleyKind::Approval,
        prompt: String::new(),
        value: serde_json::json!(false),
        responded_by: Some("demo-operator".to_string()),
        responded_at: Utc::now(),
        defaulted: false,
    };
    let branch_completed = engine
        .resume_with(&graph, thread.clone(), vec![deny_response])
        .await?;
    let branch_path = match branch_completed {
        RunOutcome::Completed {
            final_state,
            waypoint,
        } => {
            let path: Option<String> = final_state.get(&path_field)?;
            println!("   branch resumed with the opposite decision -> waypoint {waypoint}");
            println!("   branch path    = {path:?}\n");
            path.ok_or("path field must be set on the replayed branch")?
        }
        other => return Err(format!("expected the branch to complete, got {other:?}").into()),
    };

    println!("   mainline path (approved) = {mainline_path:?}");
    println!("   branch   path (denied)   = {branch_path:?}");
    if mainline_path != branch_path {
        println!("   the replayed branch diverged from the mainline's own result.\n");
    } else {
        println!("   UNEXPECTED: the replayed branch did not diverge from the mainline.\n");
    }

    println!("Done -- fully offline, no provider API key was read.");
    Ok(())
}
