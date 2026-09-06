//! Mock Paladin Port for testing Battalion patterns
//!
//! Provides a mock implementation of PaladinPort that wraps MockLlmAdapter
//! to enable testing of Formation, Phalanx, and other Battalion patterns.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::paladin::Paladin;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_core::platform::container::transience::Transience;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_port::{
    PaladinPort, PaladinResult, PaladinStream, PaladinStreamChunk, StopReason,
};

use super::MockLlmAdapter;

/// Mock implementation of PaladinPort for testing
///
/// This mock wraps a PaladinExecutionService with MockLlmAdapter to enable
/// testing of Battalion patterns (Formation, Phalanx, etc.) without real LLM calls.
pub struct MockPaladinPort {
    execution_service: Arc<PaladinExecutionService>,
}

impl MockPaladinPort {
    /// Create a new MockPaladinPort with the given MockLlmAdapter
    pub fn new(mock_llm: Arc<MockLlmAdapter>, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        let execution_service = Arc::new(PaladinExecutionService::new(
            mock_llm as Arc<dyn LlmPort>,
            circuit_breaker,
            None, // No garrison
            None, // No arsenal
        ));

        Self { execution_service }
    }
}

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        self.execution_service.execute(paladin, input).await
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<PaladinStreamChunk, PaladinError>>, PaladinError>
    {
        // For testing, we don't need streaming support
        Err(PaladinError::ExecutionError(
            "Streaming not supported in MockPaladinPort".to_string(),
        ))
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        // Always validate successfully for testing
        Ok(())
    }
}

/// Configurable failing [`PaladinPort`] mock for exercising Commander error paths.
///
/// Unlike [`MockPaladinPort`], which always succeeds, `FaultyPaladinPort` supports five
/// independently-configurable fault modes — fail always, fail a named Paladin, fail until
/// the Nth attempt (a GLOBAL invocation counter), fail a named Paladin until ITS OWN Nth
/// call (a per-Paladin counter, Phase 25 D-31), and a controllable per-execution delay —
/// so tests can exercise `ErrorStrategy::FailFast`, `ErrorStrategy::ContinueOnError` and
/// `ErrorStrategy::RetryThenContinue` against a Commander without a real LLM, and a real
/// Aegis per-task retry against the `WarEngine`.
///
/// This is the mock D-09/D-10 asked for: a single shared home for Commander error-path
/// testing, built by combining the retry-counter idiom from
/// [`crate::helpers::mock_llm_adapter`]-style interior mutability with the
/// `fail_until_attempt` pattern in `FormationExecutionService`'s in-crate test mock and the
/// `fail_paladin_names` + `delay_ms` pattern in `PhalanxExecutionService`'s.
///
/// # Precedence when several fault modes are configured
///
/// `execute` decides a call in this fixed order, and the first mode that applies wins:
///
/// 1. `fail_until_attempt` (the global counter, shared across every Paladin);
/// 2. `fail_paladin_until_attempt` (this Paladin's own counter);
/// 3. `fail_always`;
/// 4. `fail_paladin` (`fail_paladin_names`).
///
/// Every call advances BOTH the global counter and the executed Paladin's own counter,
/// whether or not an earlier mode already decided that call — so a per-Paladin threshold
/// counts the Paladin's calls, not the calls that reached step 2.
///
/// All interior state uses `Arc<Mutex<_>>`, never `Rc`/`RefCell`, so the type is
/// `Send + Sync` and safe to share across concurrent Paladin executions (Phalanx, Campaign,
/// a mustered `WarGraph` superstep).
#[derive(Clone)]
pub struct FaultyPaladinPort {
    /// Total number of `execute` calls made across every Paladin, in invocation order.
    call_count: Arc<Mutex<usize>>,
    /// One entry per `execute` call, naming the Paladin and the input it received.
    execution_log: Arc<Mutex<Vec<String>>>,
    /// When `true`, every `execute` call fails regardless of which Paladin ran.
    fail_always: bool,
    /// Paladin names that always fail when executed (checked by `paladin.node.name`).
    fail_paladin_names: Arc<Mutex<Vec<String>>>,
    /// When `Some(n)`, `execute` fails while the invocation counter is at or below `n`,
    /// then succeeds on every call after that — the retry-count pattern.
    fail_until_attempt: Option<usize>,
    /// Per-Paladin failure thresholds (Phase 25 D-31), keyed by `paladin.node.name`: a
    /// Paladin listed here fails while ITS OWN call counter is at or below the threshold
    /// and succeeds afterwards, with a Transient-classified `LlmFailure`. Purely
    /// additive beside `fail_until_attempt`, whose global semantics are unchanged.
    fail_paladin_until_attempt: Arc<Mutex<HashMap<String, usize>>>,
    /// Per-Paladin call counters backing `fail_paladin_until_attempt`, keyed by
    /// `paladin.node.name` and advanced on EVERY call for that Paladin.
    per_paladin_calls: Arc<Mutex<HashMap<String, usize>>>,
    /// Milliseconds `execute` sleeps before deciding success or failure.
    delay_ms: u64,
}

impl FaultyPaladinPort {
    /// The provider name carried by every per-Paladin `LlmFailure` this mock produces.
    pub const PROVIDER: &'static str = "faulty-paladin-port";

    /// Creates a `FaultyPaladinPort` with no configured failures: every `execute` call
    /// succeeds and is recorded in the execution log.
    pub fn new() -> Self {
        Self {
            call_count: Arc::new(Mutex::new(0)),
            execution_log: Arc::new(Mutex::new(Vec::new())),
            fail_always: false,
            fail_paladin_names: Arc::new(Mutex::new(Vec::new())),
            fail_until_attempt: None,
            fail_paladin_until_attempt: Arc::new(Mutex::new(HashMap::new())),
            per_paladin_calls: Arc::new(Mutex::new(HashMap::new())),
            delay_ms: 0,
        }
    }

    /// Makes every `execute` call fail, regardless of which Paladin ran.
    pub fn fail_always(mut self) -> Self {
        self.fail_always = true;
        self
    }

    /// Adds a Paladin name that should fail whenever it is executed. Chainable — call
    /// multiple times to fail more than one Paladin.
    pub fn fail_paladin(self, name: impl Into<String>) -> Self {
        self.fail_paladin_names.lock().unwrap().push(name.into());
        self
    }

    /// Fails every `execute` call while the invocation counter is at or below `n`, then
    /// succeeds from the `n + 1`th call onward. The counter is shared across every
    /// Paladin executed through this port, not scoped per Paladin — for a counter scoped
    /// to ONE named Paladin, see [`FaultyPaladinPort::fail_paladin_until_attempt`].
    pub fn fail_until_attempt(mut self, n: usize) -> Self {
        self.fail_until_attempt = Some(n);
        self
    }

    /// Fails the Paladin named `name` while ITS OWN call counter is at or below `n`, then
    /// succeeds from that Paladin's `n + 1`th call onward (Phase 25 D-31). Chainable —
    /// call once per Paladin; each named Paladin's counter is independent of every
    /// other's and of the global [`FaultyPaladinPort::fail_until_attempt`] counter,
    /// which keeps its documented cross-Paladin semantics unchanged.
    ///
    /// The failure produced is `PaladinError::LlmFailure { transience: Transient,
    /// status: Some(503), provider: Some(PROVIDER), .. }` — classified Transient BY VALUE,
    /// so the default `RetryPredicate::TransientOnly` retries it without any test
    /// widening the predicate (FT-FR-01, FT-FR-05, FT-FR-06 proven together).
    ///
    /// Precedence relative to the other fault modes (see the type-level rustdoc): the
    /// global `fail_until_attempt` is consulted FIRST, this per-Paladin counter SECOND,
    /// then `fail_always`, then `fail_paladin`. The Paladin's own counter advances on
    /// every one of its calls, including calls the global counter already failed.
    pub fn fail_paladin_until_attempt(self, name: impl Into<String>, n: usize) -> Self {
        self.fail_paladin_until_attempt
            .lock()
            .unwrap()
            .insert(name.into(), n);
        self
    }

    /// Sets a delay, in milliseconds, that `execute` sleeps before deciding success or
    /// failure — used to prove timeout enforcement stops sibling agents.
    pub fn with_delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    /// Returns the exact number of `execute` calls made so far, across every Paladin.
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    /// Returns a clone of the execution log, in invocation order — one entry per
    /// `execute` call, naming the Paladin executed.
    pub fn execution_log(&self) -> Vec<String> {
        self.execution_log.lock().unwrap().clone()
    }
}

impl Default for FaultyPaladinPort {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PaladinPort for FaultyPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Record the invocation before any await, in invocation order.
        {
            let mut log = self.execution_log.lock().unwrap();
            log.push(format!("{}: {}", paladin.node.name, input));
        }

        // Increment and read the shared counter, dropping the guard before the sleep.
        let current_attempt = {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            *count
        };

        // Advance THIS Paladin's own counter on every call (D-31), independently of the
        // global counter above and of whichever fault mode decides the call below.
        let paladin_attempt = {
            let mut per_paladin = self.per_paladin_calls.lock().unwrap();
            let count = per_paladin.entry(paladin.node.name.clone()).or_insert(0);
            *count += 1;
            *count
        };

        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }

        // Precedence (documented on the type): fail_until_attempt (global), then
        // fail_paladin_until_attempt (per-Paladin), then fail_always, then
        // fail_paladin_names.
        if let Some(threshold) = self.fail_until_attempt
            && current_attempt <= threshold
        {
            return Err(PaladinError::ExecutionError(format!(
                "FaultyPaladinPort: {} failed on attempt {} (fail_until_attempt={})",
                paladin.node.name, current_attempt, threshold
            )));
        }

        let per_paladin_threshold = self
            .fail_paladin_until_attempt
            .lock()
            .unwrap()
            .get(&paladin.node.name)
            .copied();
        if let Some(threshold) = per_paladin_threshold
            && paladin_attempt <= threshold
        {
            // A Transient-classified, status-carrying failure -- the shape a real
            // provider adapter produces for a 503 -- so the DEFAULT TransientOnly
            // retry predicate retries it by value.
            return Err(PaladinError::LlmFailure {
                transience: Transience::Transient,
                status: Some(503),
                provider: Some(Self::PROVIDER.to_string()),
                message: format!(
                    "FaultyPaladinPort: {} failed on its own attempt {} \
                     (fail_paladin_until_attempt={})",
                    paladin.node.name, paladin_attempt, threshold
                ),
            });
        }

        if self.fail_always {
            return Err(PaladinError::ExecutionError(format!(
                "FaultyPaladinPort: {} failed on attempt {} (fail_always)",
                paladin.node.name, current_attempt
            )));
        }

        if self
            .fail_paladin_names
            .lock()
            .unwrap()
            .contains(&paladin.node.name)
        {
            return Err(PaladinError::ExecutionError(format!(
                "FaultyPaladinPort: {} failed on attempt {} (fail_paladin)",
                paladin.node.name, current_attempt
            )));
        }

        Ok(PaladinResult {
            output: format!(
                "FaultyPaladinPort: {} processed {}",
                paladin.node.name, input
            ),
            token_count: 10,
            execution_time_ms: self.delay_ms,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        // Streaming is not supported in this mock, matching every existing mock in this
        // workspace (MockPaladinPort above, and the in-crate mocks in formation_service.rs
        // and phalanx_service.rs).
        Err(PaladinError::ExecutionError(
            "Streaming not supported in FaultyPaladinPort".to_string(),
        ))
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin::core::base::entity::node::Node;
    use paladin::core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};

    fn assert_send_sync<T: Send + Sync>() {}

    fn make_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("{} prompt", name),
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

    #[test]
    fn faulty_paladin_port_is_send_and_sync() {
        assert_send_sync::<FaultyPaladinPort>();
    }

    #[tokio::test]
    async fn faulty_paladin_port_new_succeeds_and_logs_execution() {
        let port = FaultyPaladinPort::new();
        let paladin = make_paladin("Paladin-1");

        let result = port.execute(&paladin, "hello").await;

        assert!(result.is_ok(), "New FaultyPaladinPort should succeed");
        assert_eq!(port.call_count(), 1);
        let log = port.execution_log();
        assert_eq!(log.len(), 1);
        assert!(log[0].contains("Paladin-1"));
        assert!(log[0].contains("hello"));
    }

    #[tokio::test]
    async fn faulty_paladin_port_fail_always_fails_every_execution() {
        let port = FaultyPaladinPort::new().fail_always();
        let paladin = make_paladin("Paladin-1");

        assert!(port.execute(&paladin, "first").await.is_err());
        assert!(port.execute(&paladin, "second").await.is_err());
        assert_eq!(port.call_count(), 2);
    }

    #[tokio::test]
    async fn faulty_paladin_port_fail_paladin_fails_only_named_paladin() {
        let port = FaultyPaladinPort::new().fail_paladin("Paladin-2");
        let paladin1 = make_paladin("Paladin-1");
        let paladin2 = make_paladin("Paladin-2");

        assert!(
            port.execute(&paladin1, "x").await.is_ok(),
            "Paladin-1 was not configured to fail"
        );
        assert!(
            port.execute(&paladin2, "x").await.is_err(),
            "Paladin-2 was configured to fail"
        );
    }

    #[tokio::test]
    async fn faulty_paladin_port_fail_until_attempt_then_succeeds() {
        let port = FaultyPaladinPort::new().fail_until_attempt(2);
        let paladin = make_paladin("Paladin-1");

        assert!(
            port.execute(&paladin, "x").await.is_err(),
            "attempt 1 fails"
        );
        assert!(
            port.execute(&paladin, "x").await.is_err(),
            "attempt 2 fails"
        );
        assert!(
            port.execute(&paladin, "x").await.is_ok(),
            "attempt 3 succeeds"
        );
        assert_eq!(
            port.call_count(),
            3,
            "call_count reads the exact number of attempts, not a range"
        );
    }

    #[tokio::test]
    async fn faulty_paladin_port_with_delay_ms_sleeps_before_deciding() {
        let port = FaultyPaladinPort::new().with_delay_ms(20);
        let paladin = make_paladin("Paladin-1");

        let start = std::time::Instant::now();
        let _ = port.execute(&paladin, "x").await;

        assert!(
            start.elapsed() >= std::time::Duration::from_millis(20),
            "execute should sleep for the configured delay before deciding"
        );
    }

    // --- Phase 25 D-31: the additive per-Paladin counter ----------------

    /// D-31: `fail_paladin_until_attempt("w3", 2)` fails `w3`'s OWN first
    /// two calls and succeeds on its third, while a sibling Paladin never
    /// fails no matter how many `w3` calls preceded it -- the counter is
    /// scoped per Paladin name, unlike the global `fail_until_attempt`.
    #[tokio::test]
    async fn fail_paladin_until_attempt_is_scoped_to_one_paladin() {
        let port = FaultyPaladinPort::new().fail_paladin_until_attempt("w3", 2);
        let w1 = make_paladin("w1");
        let w3 = make_paladin("w3");

        assert!(port.execute(&w3, "x").await.is_err(), "w3 call 1 fails");
        assert!(
            port.execute(&w1, "x").await.is_ok(),
            "w1 is not configured to fail, whatever w3's counter reads"
        );
        assert!(port.execute(&w3, "x").await.is_err(), "w3 call 2 fails");
        assert!(
            port.execute(&w1, "x").await.is_ok(),
            "w1 still succeeds after two w3 failures"
        );
        assert!(
            port.execute(&w3, "x").await.is_ok(),
            "w3 call 3 succeeds: its own counter is past the threshold"
        );
        assert_eq!(port.call_count(), 5, "exact total across both Paladins");
    }

    /// D-31: the per-Paladin failure is `PaladinError::LlmFailure { status:
    /// Some(503), .. }` whose typed `transience()` is `Transient`, so the
    /// DEFAULT `TransientOnly` retry predicate retries it by value -- never
    /// by parsing a message.
    #[tokio::test]
    async fn fail_paladin_until_attempt_returns_a_transient_llm_failure() {
        use paladin_core::platform::container::transience::Transience;

        let port = FaultyPaladinPort::new().fail_paladin_until_attempt("w3", 1);
        let w3 = make_paladin("w3");

        let err = port
            .execute(&w3, "x")
            .await
            .expect_err("the first w3 call fails");
        match &err {
            PaladinError::LlmFailure {
                transience,
                status,
                provider,
                ..
            } => {
                assert_eq!(*transience, Transience::Transient);
                assert_eq!(*status, Some(503));
                assert!(provider.is_some(), "a provider name is carried");
            }
            other => panic!("expected LlmFailure {{ status: Some(503), .. }}, got {other:?}"),
        }
        assert_eq!(err.transience(), Transience::Transient);
    }

    /// D-31: the global `fail_until_attempt` counter keeps exactly the
    /// semantics its rustdoc states -- shared across EVERY Paladin, never
    /// scoped per name -- and its failure is still the legacy
    /// `ExecutionError`, so the pre-existing global-counter test above is
    /// unedited and this one pins the cross-Paladin sharing explicitly.
    #[tokio::test]
    async fn the_global_fail_until_attempt_semantics_are_unchanged() {
        let port = FaultyPaladinPort::new().fail_until_attempt(2);
        let p1 = make_paladin("Paladin-1");
        let p2 = make_paladin("Paladin-2");

        let first = port.execute(&p1, "x").await;
        assert!(
            matches!(first, Err(PaladinError::ExecutionError(_))),
            "the global counter's failure is the legacy ExecutionError: {first:?}"
        );
        assert!(
            port.execute(&p2, "x").await.is_err(),
            "call 2 (a DIFFERENT Paladin) still fails: the counter is global"
        );
        assert!(
            port.execute(&p1, "x").await.is_ok(),
            "call 3 succeeds regardless of which Paladin makes it"
        );
        assert!(port.execute(&p2, "x").await.is_ok());
        assert_eq!(port.call_count(), 4);
    }

    /// D-31: a port configured with BOTH a global counter and a per-Paladin
    /// counter follows the documented precedence -- the global counter is
    /// consulted first, then the per-Paladin counter -- and every call for
    /// a named Paladin advances that Paladin's own counter whether or not
    /// the global counter already decided the call.
    #[tokio::test]
    async fn the_two_mechanisms_compose() {
        let port = FaultyPaladinPort::new()
            .fail_until_attempt(1)
            .fail_paladin_until_attempt("w3", 2);
        let w1 = make_paladin("w1");
        let w3 = make_paladin("w3");

        // Call 1 (w3): the GLOBAL counter fires first -> ExecutionError.
        let first = port.execute(&w3, "x").await;
        assert!(
            matches!(first, Err(PaladinError::ExecutionError(_))),
            "global counter takes precedence on call 1: {first:?}"
        );
        // Call 2 (w3): the global counter is past its threshold; w3's own
        // counter (advanced by call 1 too) reads 2 <= 2 -> LlmFailure.
        let second = port.execute(&w3, "x").await;
        assert!(
            matches!(second, Err(PaladinError::LlmFailure { .. })),
            "per-Paladin counter fires on w3's second call: {second:?}"
        );
        // Call 3 (w1): neither mechanism applies.
        assert!(port.execute(&w1, "x").await.is_ok());
        // Call 4 (w3): w3's counter reads 3 > 2 -> success.
        assert!(port.execute(&w3, "x").await.is_ok());
        assert_eq!(port.call_count(), 4);
    }

    /// D-31: two named Paladins each configured with their own threshold
    /// count independently -- neither's calls advance the other's counter.
    #[tokio::test]
    async fn per_paladin_counters_are_independent() {
        let port = FaultyPaladinPort::new()
            .fail_paladin_until_attempt("w2", 1)
            .fail_paladin_until_attempt("w4", 3);
        let w2 = make_paladin("w2");
        let w4 = make_paladin("w4");

        assert!(port.execute(&w4, "x").await.is_err(), "w4 call 1 fails");
        assert!(port.execute(&w4, "x").await.is_err(), "w4 call 2 fails");
        assert!(
            port.execute(&w2, "x").await.is_err(),
            "w2 call 1 fails: w4's two calls did not consume w2's threshold"
        );
        assert!(
            port.execute(&w2, "x").await.is_ok(),
            "w2 call 2 succeeds: w2's own counter is past 1"
        );
        assert!(
            port.execute(&w4, "x").await.is_err(),
            "w4 call 3 still fails: w2's calls did not advance w4's counter"
        );
        assert!(port.execute(&w4, "x").await.is_ok(), "w4 call 4 succeeds");
        assert_eq!(port.call_count(), 6);
    }

    #[tokio::test]
    async fn faulty_paladin_port_execution_log_records_invocation_order() {
        let port = FaultyPaladinPort::new();
        let paladin1 = make_paladin("Paladin-1");
        let paladin2 = make_paladin("Paladin-2");

        port.execute(&paladin1, "first").await.unwrap();
        port.execute(&paladin2, "second").await.unwrap();

        let log = port.execution_log();
        assert_eq!(log.len(), 2);
        assert!(log[0].contains("Paladin-1"), "first entry: {:?}", log);
        assert!(log[1].contains("Paladin-2"), "second entry: {:?}", log);
    }
}
