//! The `evals` custom test harness target (D-32, PRD 07 OBS-FR-13).
//!
//! `harness = false` -- this is NOT compiled against the built-in `libtest`
//! harness; `paladin_eval::eval_scenarios!` expands to its own `fn main()`
//! that globs the pattern below at runtime and hands one
//! `libtest_mimic::Trial` per `(scenario file, case)` pair to `paladin-eval`'s
//! own custom harness, so `cargo test --test evals <filter>` filters exactly
//! like any other Rust test target.
//!
//! `evals/` now holds the E2E-1/2/3 program-acceptance dogfood scenarios
//! (plan 28-16, D-34). Their `target: { registered: ... }` names resolve
//! against the three graph constructors registered below, built from the
//! SAME `tests/helpers/e2e_fixtures.rs` the three E2E integration tests use --
//! one definition, two consumers, proven never to drift apart (D-34's own
//! must-have truth).

#[allow(dead_code, unused_imports)]
#[path = "helpers/mod.rs"]
mod helpers;

use std::sync::Arc;

use helpers::e2e_fixtures;
use paladin_battalion::engine::EngineRegistries;
use paladin_eval::ScenarioRunner;

paladin_eval::eval_scenarios!("evals/**/*.eval.yaml", |runner: &mut ScenarioRunner| {
    runner
        .register_graph(
            "e2e-1-crash-resume",
            Arc::new(|_ports| e2e_fixtures::build_crash_resume_graph()),
        )
        .register_graph(
            "e2e-2-approval-gate",
            Arc::new(|_ports| e2e_fixtures::build_approval_gate_graph()),
        )
        .register_graph(
            "e2e-3-map-reduce-fault-tolerance",
            Arc::new(|_ports| {
                e2e_fixtures::build_muster_defer_order_graph_with_a_template_per_task(Some(
                    e2e_fixtures::per_task_retry_policy(),
                ))
            }),
        )
        // None of the three registered graphs above reference
        // `EdgeCondition::Custom`, `RetryPredicate::Custom` or
        // `SchemaRef::Registered` -- an empty `EngineRegistries` is the
        // structurally correct "no custom registrations needed" answer,
        // registered here (rather than omitted) so the eval harness and
        // the integration tests share one registration point and can
        // never drift apart in what they register (28-RESEARCH.md Open
        // Questions item 1). `ScenarioTarget::Registered` targets do not
        // currently consult a scenario's `registries` field at all (only
        // `ScenarioTarget::GraphDoc` does) -- this registration is
        // therefore inert for these three scenarios today, but keeps the
        // shared-registration seam in place for a future `graph_doc`
        // dogfood scenario.
        .register_registries("e2e", Arc::new(EngineRegistries::new));
});
