//! Unit tests for CircuitBreaker
//! Following TDD - these tests should fail initially

use paladin::application::services::paladin::error::PaladinError;
use paladin::infrastructure::resilience::circuit_breaker::{CircuitBreaker, CircuitState};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

#[test]
fn test_circuit_breaker_closed_state() {
    let circuit_breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));

    // Initial state should be Closed
    assert!(matches!(
        circuit_breaker.get_state(),
        CircuitState::Closed { .. }
    ));

    // Successful calls should keep circuit closed
    let result = circuit_breaker.call(|| Ok::<_, PaladinError>("success"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");

    assert!(matches!(
        circuit_breaker.get_state(),
        CircuitState::Closed { .. }
    ));
}

#[test]
fn test_circuit_breaker_opens_after_threshold_failures() {
    let circuit_breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));

    // First 2 failures should keep circuit closed
    for _ in 0..2 {
        let result = circuit_breaker.call(|| Err::<String, PaladinError>(PaladinError::Timeout(1)));
        assert!(result.is_err());
        assert!(matches!(
            circuit_breaker.get_state(),
            CircuitState::Closed { .. }
        ));
    }

    // 3rd failure should open the circuit
    let result = circuit_breaker.call(|| Err::<String, PaladinError>(PaladinError::Timeout(1)));
    assert!(result.is_err());

    // Circuit should now be Open
    assert!(matches!(
        circuit_breaker.get_state(),
        CircuitState::Open { .. }
    ));

    // Subsequent calls should fail fast with CircuitBreakerOpen error
    let result = circuit_breaker.call(|| Ok::<_, PaladinError>("should not execute"));
    assert!(matches!(result, Err(PaladinError::CircuitBreakerOpen)));
}

#[test]
fn test_circuit_breaker_half_open_state() {
    let circuit_breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));

    // Force circuit to open by exceeding failure threshold
    for _ in 0..3 {
        let _ = circuit_breaker.call(|| Err::<String, PaladinError>(PaladinError::Timeout(1)));
    }

    assert!(matches!(
        circuit_breaker.get_state(),
        CircuitState::Open { .. }
    ));

    // Wait for timeout to expire
    thread::sleep(Duration::from_millis(150));

    // Next call should transition to HalfOpen
    let result = circuit_breaker.call(|| Ok::<_, PaladinError>("test success"));

    // Call should succeed
    assert!(result.is_ok());

    // State should be HalfOpen with 1 success
    let state = circuit_breaker.get_state();
    match state {
        CircuitState::HalfOpen { successes } => {
            assert_eq!(successes, 1);
        }
        _ => panic!("Expected HalfOpen state, got {:?}", state),
    }
}

#[test]
fn test_circuit_breaker_closes_after_success() {
    let circuit_breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));

    // Force circuit to open
    for _ in 0..3 {
        let _ = circuit_breaker.call(|| Err::<String, PaladinError>(PaladinError::Timeout(1)));
    }

    // Wait for timeout
    thread::sleep(Duration::from_millis(150));

    // First success -> HalfOpen with 1 success
    let result1 = circuit_breaker.call(|| Ok::<_, PaladinError>("success 1"));
    assert!(result1.is_ok());

    let state = circuit_breaker.get_state();
    assert!(matches!(state, CircuitState::HalfOpen { successes: 1 }));

    // Second success should close the circuit (threshold is 2)
    let result2 = circuit_breaker.call(|| Ok::<_, PaladinError>("success 2"));
    assert!(result2.is_ok());

    let state = circuit_breaker.get_state();
    assert!(matches!(state, CircuitState::Closed { .. }));

    // Circuit should remain closed for subsequent successful calls
    let result3 = circuit_breaker.call(|| Ok::<_, PaladinError>("success 3"));
    assert!(result3.is_ok());
    assert!(matches!(
        circuit_breaker.get_state(),
        CircuitState::Closed { .. }
    ));
}

#[test]
fn test_circuit_breaker_concurrent_access() {
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_millis(100)));
    let success_count = Arc::new(AtomicU32::new(0));
    let mut handles = vec![];

    // Spawn multiple threads making concurrent calls
    for i in 0..10 {
        let cb = Arc::clone(&circuit_breaker);
        let sc = Arc::clone(&success_count);

        let handle = thread::spawn(move || {
            // Half the threads succeed, half fail
            let result = if i % 2 == 0 {
                cb.call(|| Ok::<String, PaladinError>("success".to_string()))
            } else {
                cb.call(|| Err::<String, PaladinError>(PaladinError::Timeout(1)))
            };

            if result.is_ok() {
                sc.fetch_add(1, Ordering::SeqCst);
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify that concurrent access worked correctly
    let final_count = success_count.load(Ordering::SeqCst);
    assert!(final_count > 0, "Expected some successful calls");

    // Circuit breaker should be in a valid state (not panicked)
    let state = circuit_breaker.get_state();
    assert!(
        matches!(
            state,
            CircuitState::Closed { .. } | CircuitState::Open { .. } | CircuitState::HalfOpen { .. }
        ),
        "Circuit breaker should be in a valid state"
    );
}

#[test]
fn test_circuit_breaker_ignores_unknown_failures() {
    let circuit_breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));

    // `ExecutionError` classifies as Unknown, so it never counts toward
    // tripping (D-14) however many occur.
    for _ in 0..10 {
        let result = circuit_breaker.call(|| {
            Err::<String, PaladinError>(PaladinError::ExecutionError("test failure".to_string()))
        });
        assert!(matches!(result, Err(PaladinError::ExecutionError(_))));
    }

    assert!(matches!(
        circuit_breaker.get_state(),
        CircuitState::Closed { .. }
    ));
}
