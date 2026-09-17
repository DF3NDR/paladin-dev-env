// examples/graceful_shutdown.rs
//
// Graceful Shutdown: Drain, Grace Period, Toggle (EX-74, EX-75, EX-76)
//
// This example demonstrates the WarEngine's graceful-shutdown capability
// cluster as a single runnable story. It shows how to:
// 1. Drain in-flight work -- register a run with a ShutdownCoordinator,
//    trigger its shutdown directly, and print which nodes finished, which
//    were cancelled, and whether the run's Halted checkpoint was durably
//    persisted at the cut. In a deployed server the SAME coordinator is
//    driven by the process's SIGTERM/SIGINT handler, not by this program --
//    this example triggers it directly so it terminates on its own rather
//    than waiting on a signal.
// 2. Configure the grace period -- override APP_ENGINE_SHUTDOWN_GRACE_SECS
//    in-process, print the before and after values, then run the drain
//    again so the overridden value's effect is visible on a real run.
// 3. Toggle graceful shutdown off and on -- override
//    APP_ENGINE_GRACEFUL_SHUTDOWN, printing the contrast between exiting
//    immediately (disabled) and waiting up to the grace period for
//    in-flight work to finish (enabled).
//
// This example is fully offline: its graph runs Function nodes only (no
// Paladin node, so no LLM call is ever made), it uses only in-memory,
// in-process state, reads no LLM provider API key from the environment, and
// needs no external service.
//
// To run this example:
// ```bash
// cargo run --example graceful_shutdown
// ```

use std::env;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use paladin::config::engine::EngineConfig;
use paladin::config::env_utils::EnvOverridable;
use paladin_battalion::engine::graph::{EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_battalion::engine::shutdown::{ShutdownCoordinator, ShutdownOutcome};
use paladin_battalion::engine::{RunOutcome, WarEngine};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// `WarEngine::new` requires a `PaladinPort`; this program's graphs are
/// built entirely from `Function` nodes, so this is never actually invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this program's WarGraph runs Function nodes only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this program's WarGraph runs Function nodes only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A pure `StateNode` that sleeps `hold`, then writes one fixed value to one
/// field. A node holding well past the configured shutdown grace is the
/// in-flight work a shutdown drains or aborts.
struct SlowWorker {
    field: FieldName,
    value: serde_json::Value,
    hold: Duration,
}

#[async_trait]
impl StateNode for SlowWorker {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        tokio::time::sleep(self.hold).await;
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(Directive {
            delta,
            next: NextStep::Edges,
        })
    }
}

/// Build a two-entry-node graph: `fast` finishes well inside any reasonable
/// grace window, `slow` never does -- so a single superstep dispatches both
/// concurrently and a shutdown mid-superstep drains one and cancels the
/// other.
fn fan_out_graph(
    fast_hold: Duration,
    slow_hold: Duration,
) -> Result<(WarGraph, FieldName, FieldName), Box<dyn std::error::Error>> {
    let fast_field = FieldName::new("fast_result")?;
    let slow_field = FieldName::new("slow_result")?;
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(fast_field.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(slow_field.clone(), DispatchRule::LastWrite, None, false),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    graph.add_node(
        NodeId::new("fast"),
        NodeSpec::Function(Arc::new(SlowWorker {
            field: fast_field.clone(),
            value: serde_json::json!("fast finished"),
            hold: fast_hold,
        })),
    );
    graph.add_node(
        NodeId::new("slow"),
        NodeSpec::Function(Arc::new(SlowWorker {
            field: slow_field.clone(),
            value: serde_json::json!("slow finished"),
            hold: slow_hold,
        })),
    );
    graph.add_entry(NodeId::new("fast"));
    graph.add_entry(NodeId::new("slow"));
    Ok((graph, fast_field, slow_field))
}

/// Register a run with a fresh [`ShutdownCoordinator`], start `graph` under
/// `engine_shutdown_grace`, wait a short fixed delay so the in-flight work
/// is genuinely underway, then trigger the coordinator's shutdown directly
/// (never a real signal handler) with `coordinator_grace`. Returns the
/// coordinator's drain outcome alongside the run's own outcome.
async fn drain_run(
    store: Arc<InMemoryWaypointStore>,
    graph: WarGraph,
    thread: ThreadId,
    engine_shutdown_grace: Duration,
    coordinator_grace: Duration,
) -> Result<(ShutdownOutcome, RunOutcome), Box<dyn std::error::Error>> {
    let coordinator = ShutdownCoordinator::new();
    let (child_token, guard) = coordinator.register();
    let engine = WarEngine::new(Arc::new(UnusedPaladinPort), store)
        .with_cancellation_token(child_token)
        .with_shutdown_grace(engine_shutdown_grace);

    let run_handle = tokio::spawn(async move {
        let outcome = engine.start(&graph, thread, StateDelta::new()).await;
        drop(guard);
        outcome
    });

    // Give the in-flight work a moment to genuinely start before triggering
    // shutdown -- deterministic enough for a demo: the fast node's own hold
    // is always far shorter than this delay, and the slow node's hold is
    // always far longer.
    tokio::time::sleep(Duration::from_millis(30)).await;

    let shutdown_outcome = coordinator.cancel_and_wait(coordinator_grace).await;
    let run_outcome = run_handle.await??;
    Ok((shutdown_outcome, run_outcome))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Graceful Shutdown: Drain, Grace Period, Toggle\n");

    // ------------------------------------------------------------------------------
    // Part 1 -- drain in-flight work (EX-74).
    // ------------------------------------------------------------------------------
    println!("1. Drain in-flight work (EX-74)\n");
    println!(
        "   (In a deployed server, ShutdownCoordinator::cancel_and_wait is driven by the\n    \
         process's SIGTERM/SIGINT handler -- this program triggers it directly so it\n    \
         terminates on its own, never waiting on a real signal.)\n"
    );

    let store = Arc::new(InMemoryWaypointStore::new());
    let (graph, fast_field, slow_field) =
        fan_out_graph(Duration::from_millis(10), Duration::from_secs(2))?;
    let thread = ThreadId::new("graceful-shutdown-drain")?;
    let (shutdown_outcome, run_outcome) = drain_run(
        store.clone(),
        graph,
        thread.clone(),
        Duration::from_millis(150),
        Duration::from_secs(5),
    )
    .await?;

    println!("   coordinator drain outcome = {shutdown_outcome:?}");
    let halted_waypoint = match run_outcome {
        RunOutcome::Halted { waypoint } => waypoint,
        other => return Err(format!("expected Halted, got {other:?}").into()),
    };
    let waypoint = store
        .get(&thread, &halted_waypoint)
        .await?
        .ok_or("the Halted waypoint must be readable back from the store")?;
    println!("   run outcome               = Halted, waypoint {halted_waypoint}");
    println!(
        "   checkpoint durable at the cut: {} completed record(s), vanguard re-lists {:?} for resume",
        waypoint.completed.len(),
        waypoint.vanguard
    );
    for record in &waypoint.completed {
        println!("     node {} -> {:?}", record.node_id, record.outcome);
    }
    let fast_result: Option<String> = waypoint.battlefield.get(&fast_field)?;
    let slow_result: Option<String> = waypoint.battlefield.get(&slow_field)?;
    println!("   fast_result = {fast_result:?} (finished before the grace deadline)");
    println!("   slow_result = {slow_result:?} (still None -- aborted, never merged)\n");

    // ------------------------------------------------------------------------------
    // Part 2 -- configure the grace period (EX-75).
    // ------------------------------------------------------------------------------
    println!("2. Configure the grace period (EX-75)\n");

    let before_grace = EngineConfig::default().shutdown_grace_secs;
    // SAFETY: this program is single-threaded at this point in `main` -- no
    // other task reads or writes this variable concurrently with the
    // set/apply/remove sequence below.
    unsafe { env::set_var("APP_ENGINE_SHUTDOWN_GRACE_SECS", "1") };
    let mut overridden_config = EngineConfig::default();
    overridden_config.apply_env_overrides();
    unsafe { env::remove_var("APP_ENGINE_SHUTDOWN_GRACE_SECS") };
    println!("   before APP_ENGINE_SHUTDOWN_GRACE_SECS: shutdown_grace_secs = {before_grace}");
    println!(
        "   after  APP_ENGINE_SHUTDOWN_GRACE_SECS: shutdown_grace_secs = {}\n",
        overridden_config.shutdown_grace_secs
    );

    println!("   running the drain again with the overridden grace wired into the engine:\n");
    let (single_graph, _fast_unused, slow_field_2) =
        fan_out_graph(Duration::from_millis(5), Duration::from_secs(5))?;
    let thread2 = ThreadId::new("graceful-shutdown-grace-override")?;
    let started_at = std::time::Instant::now();
    let (shutdown_outcome_2, run_outcome_2) = drain_run(
        store.clone(),
        single_graph,
        thread2.clone(),
        Duration::from_secs(overridden_config.shutdown_grace_secs),
        Duration::from_secs(30),
    )
    .await?;
    let elapsed = started_at.elapsed();

    println!("   coordinator drain outcome = {shutdown_outcome_2:?}");
    match run_outcome_2 {
        RunOutcome::Halted { waypoint: wp_id } => {
            let wp = store
                .get(&thread2, &wp_id)
                .await?
                .ok_or("the Halted waypoint must be readable back from the store")?;
            let slow_result_2: Option<String> = wp.battlefield.get(&slow_field_2)?;
            println!(
                "   run outcome               = Halted after {elapsed:.2?} (bounded by the \
                 overridden {}s grace, not the default {before_grace}s)",
                overridden_config.shutdown_grace_secs
            );
            println!(
                "   slow_result = {slow_result_2:?} (still None -- aborted at the new, shorter grace)\n"
            );
        }
        other => return Err(format!("expected Halted, got {other:?}").into()),
    }

    // ------------------------------------------------------------------------------
    // Part 3 -- toggle graceful shutdown off and on (EX-76).
    // ------------------------------------------------------------------------------
    println!("3. Toggle graceful shutdown off and on (EX-76)\n");

    unsafe { env::set_var("APP_ENGINE_GRACEFUL_SHUTDOWN", "false") };
    let mut disabled_config = EngineConfig::default();
    disabled_config.apply_env_overrides();
    unsafe { env::remove_var("APP_ENGINE_GRACEFUL_SHUTDOWN") };
    println!(
        "   APP_ENGINE_GRACEFUL_SHUTDOWN=false -> graceful_shutdown = {}",
        disabled_config.graceful_shutdown
    );

    let coordinator_disabled = ShutdownCoordinator::new();
    let (_token, guard_disabled) = coordinator_disabled.register();
    let hold_disabled = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        drop(guard_disabled);
    });
    let effective_grace_disabled = if disabled_config.graceful_shutdown {
        Duration::from_secs(disabled_config.shutdown_grace_secs)
    } else {
        Duration::ZERO
    };
    let outcome_disabled = coordinator_disabled
        .cancel_and_wait(effective_grace_disabled)
        .await;
    println!(
        "   disabled -> exits immediately without waiting: {outcome_disabled:?} (in_flight = {})\n",
        coordinator_disabled.in_flight()
    );
    hold_disabled.await?;

    unsafe { env::set_var("APP_ENGINE_GRACEFUL_SHUTDOWN", "true") };
    let mut enabled_config = EngineConfig::default();
    enabled_config.apply_env_overrides();
    unsafe { env::remove_var("APP_ENGINE_GRACEFUL_SHUTDOWN") };
    println!(
        "   APP_ENGINE_GRACEFUL_SHUTDOWN=true  -> graceful_shutdown = {}",
        enabled_config.graceful_shutdown
    );

    let coordinator_enabled = ShutdownCoordinator::new();
    let (_token, guard_enabled) = coordinator_enabled.register();
    let hold_enabled = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        drop(guard_enabled);
    });
    let effective_grace_enabled = if enabled_config.graceful_shutdown {
        Duration::from_secs(enabled_config.shutdown_grace_secs)
    } else {
        Duration::ZERO
    };
    let outcome_enabled = coordinator_enabled
        .cancel_and_wait(effective_grace_enabled)
        .await;
    println!(
        "   enabled  -> waits for in-flight work to drain: {outcome_enabled:?} (in_flight = {})\n",
        coordinator_enabled.in_flight()
    );
    hold_enabled.await?;

    println!("Done -- fully offline, no provider API key was read.");
    Ok(())
}
