//! Commander error-path integration tests
//!
//! Real bodies for the four Commander error-path tests that previously lived as
//! `#[ignore]`d, empty-bodied stubs in `crates/paladin-battalion/src/commander.rs`
//! (their own TODO comments asked for exactly this relocation). Driven by
//! `FaultyPaladinPort`, the shared configurable-failure mock in `tests/helpers/`, these
//! prove retry counts increment, partial failures are collected and returned separately,
//! and a FailFast error stops sibling Paladins from executing (QUAL-04).

use crate::helpers::FaultyPaladinPort;
use paladin::application::services::battalion::commander::CommanderBuilder;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionStrategy};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::aegis::{
    Aegis, ErrorHandlerSpec, RetryPolicy, RetryPredicate,
};
use paladin_core::platform::container::battlefield::StateDelta;
use paladin_ports::output::paladin_port::PaladinPort;
use std::sync::Arc;
use std::time::Duration;

/// The Aegis equivalent of the v0.10 continue-on-error strategy: absorb a failed
/// Paladin and keep going (`FailFast` is the default, `on_error: None`).
fn absorb() -> Aegis {
    Aegis {
        on_error: Some(ErrorHandlerSpec::Absorb {
            fallback_delta: StateDelta::new(),
        }),
        ..Aegis::default()
    }
}

/// Helper to create test Paladins, matching the `Paladin-N` naming convention already
/// used in `tests/integration/commander_integration_tests.rs`.
fn create_test_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}", name),
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

#[tokio::test]
async fn test_fail_fast_stops_on_first_error() {
    let port = Arc::new(FaultyPaladinPort::new().fail_paladin("Paladin-2"));

    let paladin1 = create_test_paladin("Paladin-1");
    let paladin2 = create_test_paladin("Paladin-2");
    let paladin3 = create_test_paladin("Paladin-3");

    let config = BattalionConfig::new("fail_fast_error_paths");

    let commander = CommanderBuilder::new(port.clone() as Arc<dyn PaladinPort>)
        .strategy(BattalionStrategy::Formation)
        .paladins(vec![paladin1, paladin2, paladin3])
        .config(config)
        .build()
        .expect("Failed to build Commander");

    let result = commander.execute("Test fail fast").await;

    assert!(result.is_err(), "FailFast should propagate the error");

    let log = port.execution_log();
    assert!(
        log.iter().any(|entry| entry.contains("Paladin-1")),
        "Paladin-1 should have executed: {:?}",
        log
    );
    assert!(
        log.iter().any(|entry| entry.contains("Paladin-2")),
        "Paladin-2 should have executed and failed: {:?}",
        log
    );
    assert!(
        !log.iter().any(|entry| entry.contains("Paladin-3")),
        "Paladin-3 should never have executed after FailFast stopped: {:?}",
        log
    );
}

#[tokio::test]
async fn test_continue_on_error_collects_all_errors() {
    let port = Arc::new(
        FaultyPaladinPort::new()
            .fail_paladin("Paladin-1")
            .fail_paladin("Paladin-3"),
    );

    let paladin1 = create_test_paladin("Paladin-1");
    let paladin2 = create_test_paladin("Paladin-2");
    let paladin3 = create_test_paladin("Paladin-3");

    let config = BattalionConfig::new("continue_on_error_error_paths").with_aegis(absorb());

    let commander = CommanderBuilder::new(port.clone() as Arc<dyn PaladinPort>)
        .strategy(BattalionStrategy::Formation)
        .paladins(vec![paladin1, paladin2, paladin3])
        .config(config)
        .build()
        .expect("Failed to build Commander");

    let result = commander
        .execute("Test continue on error")
        .await
        .expect("ContinueOnError should return a result despite failures");

    let log = port.execution_log();
    assert_eq!(
        log.len(),
        3,
        "All Paladins should execute with ContinueOnError: {:?}",
        log
    );

    // Two distinct failures, collected separately and never merged into one aggregate error.
    assert_eq!(
        result.node_errors.len(),
        2,
        "Two distinct error entries expected: {:?}",
        result.node_errors
    );
    let failed_names: Vec<&str> = result
        .node_errors
        .iter()
        .map(|e| e.node_name.as_str())
        .collect();
    assert!(
        failed_names.contains(&"Paladin-1"),
        "Paladin-1's failure should be recorded: {:?}",
        failed_names
    );
    assert!(
        failed_names.contains(&"Paladin-3"),
        "Paladin-3's failure should be recorded: {:?}",
        failed_names
    );

    assert_eq!(result.paladin_success_count, 1);
    assert_eq!(result.paladin_failure_count, 2);
}

#[tokio::test]
async fn test_retry_then_continue_retries_failed_paladins() {
    let port = Arc::new(FaultyPaladinPort::new().fail_until_attempt(2));

    let paladin1 = create_test_paladin("Paladin-1");
    let paladin2 = create_test_paladin("Paladin-2");
    let paladin3 = create_test_paladin("Paladin-3");

    // v0.10 `RetryPolicy { max_attempts: 3 }` is `max_attempts: 4` under Aegis (the count now
    // includes the first attempt). `TransientAndUnknown` keeps the v0.10 "retry everything"
    // meaning: `FaultyPaladinPort` fails with an Unknown `ExecutionError`, which the default
    // `TransientOnly` would not retry.
    let retry = RetryPolicy {
        max_attempts: 4,
        initial_interval: Duration::from_millis(1),
        backoff_factor: 1.0,
        max_interval: Duration::from_millis(5),
        jitter: false,
        retry_on: RetryPredicate::TransientAndUnknown,
    };

    let config = BattalionConfig::new("retry_then_continue_error_paths").with_aegis(Aegis {
        retry: Some(retry),
        ..absorb()
    });

    let commander = CommanderBuilder::new(port.clone() as Arc<dyn PaladinPort>)
        .strategy(BattalionStrategy::Formation)
        .paladins(vec![paladin1, paladin2, paladin3])
        .config(config)
        .build()
        .expect("Failed to build Commander");

    let result = commander
        .execute("Test retry then continue")
        .await
        .expect("RetryThenContinue should succeed once retries exhaust the configured failures");

    // FaultyPaladinPort's invocation counter is shared across every Paladin, not scoped
    // per Paladin. `fail_until_attempt(2)` fails the first two calls globally: Paladin-1's
    // first two attempts (global calls 1 and 2), then succeeds on its third attempt
    // (global call 3) — proving a retry occurred. Paladin-2 and Paladin-3 then each
    // succeed on their first attempt (global calls 4 and 5), for an exact total of 5.
    assert_eq!(
        port.call_count(),
        5,
        "call_count reads the exact number of executions, not a range"
    );
    assert_eq!(result.paladin_success_count, 3);
    assert_eq!(result.paladin_failure_count, 0);

    // Formation chains each Paladin's output into the next Paladin's input, so a plain
    // substring search would also match "Paladin-1" inside Paladin-2/3's carried-over
    // input text. Match on the log entry's leading "{name}: " prefix instead, which
    // names the Paladin that actually executed.
    let log = port.execution_log();
    let paladin1_attempts = log.iter().filter(|e| e.starts_with("Paladin-1:")).count();
    assert_eq!(
        paladin1_attempts, 3,
        "Paladin-1 should have been retried until it succeeded, within the configured \
         3-attempt retry budget: {:?}",
        log
    );
}

#[tokio::test]
async fn test_partial_results_returned_with_errors() {
    let port = Arc::new(FaultyPaladinPort::new().fail_paladin("Paladin-2"));

    let paladin1 = create_test_paladin("Paladin-1");
    let paladin2 = create_test_paladin("Paladin-2");
    let paladin3 = create_test_paladin("Paladin-3");

    let config = BattalionConfig::new("partial_results_error_paths").with_aegis(absorb());

    let commander = CommanderBuilder::new(port.clone() as Arc<dyn PaladinPort>)
        .strategy(BattalionStrategy::Formation)
        .paladins(vec![paladin1, paladin2, paladin3])
        .config(config)
        .build()
        .expect("Failed to build Commander");

    let result = commander
        .execute("Test partial results")
        .await
        .expect("ContinueOnError should return partial results despite one failure");

    assert_eq!(
        result.paladin_results.len(),
        2,
        "The two successful Paladin outputs should be preserved: {:?}",
        result.paladin_results
    );
    let succeeded_outputs: Vec<&str> = result
        .paladin_results
        .iter()
        .map(|r| r.output.as_str())
        .collect();
    assert!(
        succeeded_outputs.iter().any(|o| o.contains("Paladin-1")),
        "Paladin-1's successful output should be present: {:?}",
        succeeded_outputs
    );
    assert!(
        succeeded_outputs.iter().any(|o| o.contains("Paladin-3")),
        "Paladin-3's successful output should be present: {:?}",
        succeeded_outputs
    );

    assert_eq!(result.node_errors.len(), 1);
    assert_eq!(result.node_errors[0].node_name, "Paladin-2");

    assert_eq!(result.paladin_success_count, 2);
    assert_eq!(result.paladin_failure_count, 1);
}
