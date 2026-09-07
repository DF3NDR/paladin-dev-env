//! Circuit Breaker Pattern Implementation
//!
//! This module implements the Circuit Breaker pattern for fault tolerance and resilience.
//! The circuit breaker monitors failures and prevents cascading failures by temporarily
//! blocking operations when a failure threshold is exceeded.
//!
//! # Circuit Breaker States
//!
//! - **Closed**: Normal operation. Requests pass through. Failures are counted.
//! - **Open**: Circuit has detected too many failures. All requests fail fast without execution.
//! - **HalfOpen**: Testing if the underlying service has recovered. Limited requests allowed.
//!
//! # State Transitions
//!
//! ```text
//! Closed --[failure_threshold exceeded]--> Open
//! Open --[timeout expires]--> HalfOpen
//! HalfOpen --[success_threshold met]--> Closed
//! HalfOpen --[failure occurs]--> Open
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
//! use paladin::application::services::paladin::error::PaladinError;
//! use std::time::Duration;
//!
//! let circuit_breaker = CircuitBreaker::new(
//!     3,                            // failure_threshold
//!     2,                            // success_threshold
//!     Duration::from_secs(30),      // timeout
//! );
//!
//! // Wrap potentially failing operations
//! let result = circuit_breaker.call(|| {
//!     // Your operation here
//!     Ok::<_, PaladinError>("success")
//! });
//!
//! match result {
//!     Ok(value) => println!("Success: {}", value),
//!     Err(PaladinError::CircuitBreakerOpen) => {
//!         println!("Circuit breaker is open, failing fast");
//!     }
//!     Err(e) => println!("Operation failed: {}", e),
//! }
//! ```

use crate::application::services::paladin::error::PaladinError;
use log::{debug, info, warn};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Circuit breaker state
///
/// The state determines how the circuit breaker handles incoming requests.
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed - requests pass through normally
    /// Tracks consecutive failures to detect when to open
    Closed {
        /// Number of consecutive failures
        failures: u32,
    },

    /// Circuit is open - all requests fail fast
    /// Records when the circuit opened to implement timeout
    Open {
        /// Timestamp when the circuit opened
        opened_at: Instant,
    },

    /// Circuit is half-open - testing if service recovered
    /// Allows limited requests through and tracks successes
    HalfOpen {
        /// Number of consecutive successes in half-open state
        successes: u32,
    },
}

/// Circuit Breaker for fault tolerance and resilience
///
/// Implements the Circuit Breaker pattern to prevent cascading failures
/// by monitoring operation failures and temporarily blocking requests when
/// a failure threshold is exceeded.
///
/// # Thread Safety
///
/// The circuit breaker is thread-safe and can be shared across threads using `Arc<CircuitBreaker>`.
/// State transitions are protected by a `RwLock` to allow concurrent reads and exclusive writes.
///
/// # Example
///
/// ```rust,no_run
/// use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
/// use paladin::application::services::paladin::error::PaladinError;
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
///
/// // Use in multiple threads
/// let cb = Arc::clone(&circuit_breaker);
/// std::thread::spawn(move || {
///     let result = cb.call(|| Ok::<_, PaladinError>("thread result"));
/// });
/// ```
pub struct CircuitBreaker {
    /// Current state of the circuit breaker
    state: RwLock<CircuitState>,

    /// Number of consecutive failures before opening the circuit
    failure_threshold: u32,

    /// Number of consecutive successes in half-open state before closing
    success_threshold: u32,

    /// Duration to wait before transitioning from Open to HalfOpen
    timeout: Duration,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with the specified configuration
    ///
    /// # Arguments
    ///
    /// * `failure_threshold` - Number of consecutive failures before opening the circuit
    /// * `success_threshold` - Number of consecutive successes in half-open state before closing
    /// * `timeout` - Duration to wait in open state before attempting recovery
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// let cb = CircuitBreaker::new(
    ///     5,                          // Open after 5 failures
    ///     3,                          // Close after 3 successes
    ///     Duration::from_secs(60),    // Wait 60s before retry
    /// );
    /// ```
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        info!(
            "Creating new circuit breaker: failure_threshold={}, success_threshold={}, timeout_ms={}",
            failure_threshold,
            success_threshold,
            timeout.as_millis()
        );

        Self {
            state: RwLock::new(CircuitState::Closed { failures: 0 }),
            failure_threshold,
            success_threshold,
            timeout,
        }
    }

    /// Executes the given operation through the circuit breaker
    ///
    /// This method wraps an operation with circuit breaker logic. If the circuit is open,
    /// the operation fails fast without execution. Otherwise, the operation is executed
    /// and the result affects the circuit state.
    ///
    /// # Type Parameters
    ///
    /// * `F` - Closure type that performs the operation
    /// * `T` - Return type of the operation
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that performs the potentially failing operation
    ///
    /// # Returns
    ///
    /// - `Ok(T)` - Operation succeeded
    /// - `Err(PaladinError::CircuitBreakerOpen)` - Circuit is open, operation not executed
    /// - `Err(PaladinError)` - Operation was executed but failed
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// use paladin::application::services::paladin::error::PaladinError;
    /// use std::time::Duration;
    ///
    /// let cb = CircuitBreaker::new(3, 2, Duration::from_secs(30));
    ///
    /// // Simulate an operation that succeeds
    /// let result = cb.call(|| {
    ///     Ok("Success!")
    /// });
    ///
    /// assert!(result.is_ok());
    /// ```
    pub fn call<F, T>(&self, f: F) -> Result<T, PaladinError>
    where
        F: FnOnce() -> Result<T, PaladinError>,
    {
        // Check if we should transition from Open to HalfOpen
        self.check_and_transition_to_half_open();

        // Read current state
        let current_state = {
            let state = self.state.read().unwrap();
            state.clone()
        };

        match current_state {
            CircuitState::Open { .. } => {
                debug!("Circuit breaker is open, failing fast");
                Err(PaladinError::CircuitBreakerOpen)
            }
            CircuitState::Closed { .. } | CircuitState::HalfOpen { .. } => {
                // Execute the operation
                match f() {
                    Ok(result) => {
                        self.on_success();
                        Ok(result)
                    }
                    Err(e) => {
                        // Only count retryable errors as failures for circuit breaker
                        if e.is_retryable() {
                            self.on_failure();
                        }
                        Err(e)
                    }
                }
            }
        }
    }

    /// Executes the given async operation through the circuit breaker
    ///
    /// This is the async version of `call` for async operations. If the circuit is open,
    /// the operation fails fast without execution. Otherwise, the operation is executed
    /// and the result affects the circuit state.
    ///
    /// # Type Parameters
    ///
    /// * `F` - Future type that performs the async operation
    /// * `T` - Return type of the operation
    ///
    /// # Arguments
    ///
    /// * `f` - Future that performs the potentially failing async operation
    ///
    /// # Returns
    ///
    /// - `Ok(T)` - Operation succeeded
    /// - `Err(PaladinError::CircuitBreakerOpen)` - Circuit is open, operation not executed
    /// - `Err(PaladinError)` - Operation was executed but failed
    pub async fn call_async<F, T>(&self, f: F) -> Result<T, PaladinError>
    where
        F: std::future::Future<Output = Result<T, PaladinError>>,
    {
        // Check if we should transition from Open to HalfOpen
        self.check_and_transition_to_half_open();

        // Read current state
        let current_state = {
            let state = self.state.read().unwrap();
            state.clone()
        };

        match current_state {
            CircuitState::Open { .. } => {
                debug!("Circuit breaker is open, failing fast");
                Err(PaladinError::CircuitBreakerOpen)
            }
            CircuitState::Closed { .. } | CircuitState::HalfOpen { .. } => {
                // Execute the async operation
                match f.await {
                    Ok(result) => {
                        self.on_success();
                        Ok(result)
                    }
                    Err(e) => {
                        // Only count retryable errors as failures for circuit breaker
                        if e.is_retryable() {
                            self.on_failure();
                        }
                        Err(e)
                    }
                }
            }
        }
    }

    /// Gets the current state of the circuit breaker
    ///
    /// This is useful for monitoring and testing purposes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin::infrastructure::resilience::circuit_breaker::{CircuitBreaker, CircuitState};
    /// use std::time::Duration;
    ///
    /// let cb = CircuitBreaker::new(3, 2, Duration::from_secs(30));
    /// let state = cb.get_state();
    ///
    /// match state {
    ///     CircuitState::Closed { .. } => println!("Circuit is closed"),
    ///     CircuitState::Open { .. } => println!("Circuit is open"),
    ///     CircuitState::HalfOpen { .. } => println!("Circuit is half-open"),
    /// }
    /// ```
    pub fn get_state(&self) -> CircuitState {
        let state = self.state.read().unwrap();
        state.clone()
    }

    /// The number of consecutive failures configured to open the circuit
    /// (the `failure_threshold` constructor argument), so a caller can pin
    /// what figures a `CircuitBreaker` was actually built with -- e.g.
    /// `paladin::presets::reasoning_agent`'s documented default (Doc 05
    /// D-35).
    pub fn failure_threshold(&self) -> u32 {
        self.failure_threshold
    }

    /// The number of consecutive half-open successes configured to close
    /// the circuit (the `success_threshold` constructor argument). See
    /// [`Self::failure_threshold`].
    pub fn success_threshold(&self) -> u32 {
        self.success_threshold
    }

    /// The configured open-state recovery wait (the `timeout` constructor
    /// argument). See [`Self::failure_threshold`].
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Handles successful operation
    ///
    /// Updates the circuit state based on success:
    /// - Closed: Reset failure counter
    /// - HalfOpen: Increment success counter, potentially close circuit
    fn on_success(&self) {
        let mut state = self.state.write().unwrap();

        match *state {
            CircuitState::Closed { .. } => {
                // Reset failure counter on success
                *state = CircuitState::Closed { failures: 0 };
                debug!("Success in Closed state, reset failure counter");
            }
            CircuitState::HalfOpen { successes } => {
                let new_successes = successes + 1;

                if new_successes >= self.success_threshold {
                    // Close the circuit after threshold successes
                    *state = CircuitState::Closed { failures: 0 };
                    info!(
                        "Circuit breaker transitioned from HalfOpen to Closed (threshold={})",
                        self.success_threshold
                    );
                } else {
                    *state = CircuitState::HalfOpen {
                        successes: new_successes,
                    };
                    debug!(
                        "Success in HalfOpen state: successes={}, threshold={}",
                        new_successes, self.success_threshold
                    );
                }
            }
            CircuitState::Open { .. } => {
                // Should not happen - open state blocks execution
                warn!("Unexpected success in Open state");
            }
        }
    }

    /// Handles failed operation
    ///
    /// Updates the circuit state based on failure:
    /// - Closed: Increment failure counter, potentially open circuit
    /// - HalfOpen: Immediately reopen circuit
    fn on_failure(&self) {
        let mut state = self.state.write().unwrap();

        match *state {
            CircuitState::Closed { failures } => {
                let new_failures = failures + 1;

                if new_failures >= self.failure_threshold {
                    // Open the circuit
                    *state = CircuitState::Open {
                        opened_at: Instant::now(),
                    };
                    warn!(
                        "Circuit breaker opened due to failures (threshold={})",
                        self.failure_threshold
                    );
                } else {
                    *state = CircuitState::Closed {
                        failures: new_failures,
                    };
                    debug!(
                        "Failure recorded in Closed state: failures={}, threshold={}",
                        new_failures, self.failure_threshold
                    );
                }
            }
            CircuitState::HalfOpen { .. } => {
                // Reopen circuit on any failure in half-open state
                *state = CircuitState::Open {
                    opened_at: Instant::now(),
                };
                warn!("Circuit breaker reopened after failure in HalfOpen state");
            }
            CircuitState::Open { .. } => {
                // Should not happen - open state blocks execution
                warn!("Unexpected failure in Open state");
            }
        }
    }

    /// Checks if timeout has expired and transitions from Open to HalfOpen
    ///
    /// This is called before each operation to potentially allow retry attempts
    fn check_and_transition_to_half_open(&self) {
        let should_transition = {
            let state = self.state.read().unwrap();

            match *state {
                CircuitState::Open { opened_at } => {
                    let elapsed = opened_at.elapsed();
                    elapsed >= self.timeout
                }
                _ => false,
            }
        };

        if should_transition {
            let mut state = self.state.write().unwrap();

            // Double-check after acquiring write lock
            if let CircuitState::Open { opened_at } = *state
                && opened_at.elapsed() >= self.timeout
            {
                *state = CircuitState::HalfOpen { successes: 0 };
                info!(
                    "Circuit breaker transitioned from Open to HalfOpen (timeout_ms={})",
                    self.timeout.as_millis()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(30));
        assert!(matches!(
            cb.get_state(),
            CircuitState::Closed { failures: 0 }
        ));
    }

    #[test]
    fn test_state_transitions() {
        let cb = CircuitBreaker::new(2, 1, Duration::from_millis(10));

        // Initial state is Closed
        assert!(matches!(cb.get_state(), CircuitState::Closed { .. }));

        // Two failures should open the circuit
        let _ = cb.call(|| Err::<(), _>(PaladinError::ExecutionError("fail".into())));
        let _ = cb.call(|| Err::<(), _>(PaladinError::ExecutionError("fail".into())));

        assert!(matches!(cb.get_state(), CircuitState::Open { .. }));
    }
}
