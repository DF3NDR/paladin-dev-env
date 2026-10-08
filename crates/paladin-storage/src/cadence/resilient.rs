//! # Resilient Cadence -- a shared backend that degrades, never fails (PACE-05, D-05)
//!
//! [`ResilientCadence`] wraps a *primary* [`CadencePort`] (in production the Redis adapter,
//! [`crate::cadence::redis::RedisCadence`]) and an in-process *fallback*
//! ([`InMemoryCadence`], built with a stricter multiplier). While the primary answers, every
//! operation is served by it. The first primary error -- a backend failure or a timeout --
//! latches *degraded mode*: that same call, and every later call, is served by the fallback, so
//! a 429 recorded during the outage still gates its key. A run is never left unpaced and no LLM
//! call fails because pacing state is unreachable.
//!
//! ## Why the latch lives in an adapter
//!
//! The `CadenceLlmAdapter` decorator stays oblivious: it consults one [`CadencePort`] and never
//! sees an error that matters. Putting the outage behaviour in a composite adapter keeps it
//! testable without a Redis server (an injected failing port is enough) and lets any backend
//! reuse it.
//!
//! ## The D-05 rules
//!
//! * **Stricter delays.** The fallback cannot see the 429s other workers observe, so it is built
//!   with `InMemoryCadence::with_multiplier` (default 2.0 from configuration): a delay-less
//!   first 429 gates for `2 * base_backoff`, and a provider minimum is only ever multiplied up.
//! * **One warning per outage.** The healthy-to-degraded edge emits exactly one `warn` under
//!   the `paladin::cadence` log target ([`CADENCE_LOG_TARGET`]) and increments
//!   [`ResilientCadence::degraded_transitions`]; recovery emits exactly one `info`. The warning
//!   interpolates only the [`CadenceError`], whose messages are URL-free by construction.
//! * **Probe back-off.** A dead Redis must not add a timeout to every LLM call (research
//!   Pitfall 10). While degraded, the primary is attempted at most once per probe interval
//!   ([`DEFAULT_PROBE_INTERVAL`], 5 s); a single claim flag means concurrent callers never wait
//!   on a probe -- they are served by the fallback while one caller probes.
//! * **Automatic recovery.** The first probe that succeeds clears the latch and later calls go
//!   to the primary again.
//! * **The max rule on recovery.** A gate recorded in-process during the outage is not copied
//!   back into the primary; instead `gate` returns the larger wait (and streak) of the primary
//!   and the fallback, so a key gated during the outage is not released early.
//!
//! ## Flagged assumptions
//!
//! * A1: during an outage a worker cannot see 429s other workers observe; the multiplier is a
//!   conservative heuristic, not a guarantee that the fleet stays inside the provider's limit.
//! * A2: gates written to the primary before the outage are invisible to the fallback, so the
//!   first call after the latch trips may send without waiting out a pre-outage fleet gate. It
//!   is still paced: any 429 it draws gates the key in-process at once (reactive pacing, D-04).
//! * A3: recovery is detected within one probe interval plus one round trip; state recorded
//!   in-process during the outage is honoured locally through the max rule until it elapses.
//!
//! The lock guarding the latch is never held across an `.await`; a poisoned lock is recovered
//! with `PoisonError::into_inner`, because the latch is a routing hint, not a correctness
//! dependency.

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::Instant;

use paladin_ports::output::cadence_port::{
    CADENCE_LOG_TARGET, CadenceError, CadenceKey, CadencePort, GateReading,
};

use super::in_memory::InMemoryCadence;

/// How often a degraded [`ResilientCadence`] attempts the primary again (5 seconds).
pub const DEFAULT_PROBE_INTERVAL: Duration = Duration::from_secs(5);

/// The outage latch.
#[derive(Debug, Clone, Copy, Default)]
struct Latch {
    /// Whether the primary is currently considered unavailable.
    degraded: bool,
    /// When the primary was last attempted (or last failed) while degraded.
    last_probe: Option<Instant>,
}

/// Where one operation goes.
enum Route<'a> {
    /// The primary is healthy.
    Primary,
    /// Degraded, and this caller won the right to try the primary once.
    Probe(ProbeClaim<'a>),
    /// Degraded; serve from the fallback without touching the primary.
    Fallback,
}

/// The single in-flight probe claim; released on drop so a cancelled probe never wedges
/// recovery.
struct ProbeClaim<'a>(&'a AtomicBool);

impl Drop for ProbeClaim<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// A [`CadencePort`] that serves from a primary while it answers and from a stricter in-process
/// fallback while it does not.
///
/// See the module documentation for the rules. The composite never returns `Err` to its
/// caller.
///
/// # Examples
///
/// A healthy primary serves every call:
///
/// ```
/// use std::sync::Arc;
/// use paladin_ports::output::cadence_port::{CadenceKey, CadencePolicy, CadencePort};
/// use paladin_storage::cadence::{InMemoryCadence, ResilientCadence};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let policy = CadencePolicy::default();
/// let cadence = ResilientCadence::new(
///     Arc::new(InMemoryCadence::new(policy)),
///     InMemoryCadence::new(policy).with_multiplier(2.0),
/// );
/// let key = CadenceKey::new("openai", "gpt-4o");
/// cadence.record_rate_limited(&key, None).await?;
/// assert!(!cadence.gate(&key).await?.is_clear());
/// assert!(!cadence.is_degraded());
/// # Ok(())
/// # }
/// ```
///
/// A primary that fails is absorbed by the fallback, which still gates the key:
///
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
/// use async_trait::async_trait;
/// use paladin_ports::output::cadence_port::{
///     CadenceError, CadenceKey, CadencePolicy, CadencePort, GateReading,
/// };
/// use paladin_storage::cadence::{InMemoryCadence, ResilientCadence};
///
/// /// A shared backend that is down.
/// struct Down;
///
/// #[async_trait]
/// impl CadencePort for Down {
///     async fn gate(&self, _: &CadenceKey) -> Result<GateReading, CadenceError> {
///         Err(CadenceError::Backend { message: "connection refused".into() })
///     }
///     async fn record_rate_limited(
///         &self,
///         _: &CadenceKey,
///         _: Option<Duration>,
///     ) -> Result<GateReading, CadenceError> {
///         Err(CadenceError::Backend { message: "connection refused".into() })
///     }
///     async fn record_success(&self, _: &CadenceKey) -> Result<(), CadenceError> {
///         Err(CadenceError::Backend { message: "connection refused".into() })
///     }
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let policy = CadencePolicy::default();
/// let cadence = ResilientCadence::new(
///     Arc::new(Down),
///     InMemoryCadence::new(policy).with_jitter(|| 0.0).with_multiplier(2.0),
/// );
/// let key = CadenceKey::new("openai", "gpt-4o");
///
/// // The first delay-less 429 gates for 2 x the 500 ms base, from the fallback.
/// let reading = cadence.record_rate_limited(&key, None).await?;
/// assert_eq!(reading.wait(), Duration::from_secs(1));
/// assert!(cadence.is_degraded());
/// assert_eq!(cadence.degraded_transitions(), 1);
/// # Ok(())
/// # }
/// ```
pub struct ResilientCadence {
    primary: Arc<dyn CadencePort>,
    fallback: InMemoryCadence,
    probe_interval: Duration,
    state: Mutex<Latch>,
    probing: AtomicBool,
    transitions: AtomicU64,
}

impl ResilientCadence {
    /// Compose a primary with its in-process fallback, probing every
    /// [`DEFAULT_PROBE_INTERVAL`] while degraded.
    ///
    /// Build the fallback with `InMemoryCadence::with_multiplier` to make degraded delays
    /// stricter (D-05).
    pub fn new(primary: Arc<dyn CadencePort>, fallback: InMemoryCadence) -> Self {
        Self {
            primary,
            fallback,
            probe_interval: DEFAULT_PROBE_INTERVAL,
            state: Mutex::new(Latch::default()),
            probing: AtomicBool::new(false),
            transitions: AtomicU64::new(0),
        }
    }

    /// Replace how often a degraded instance attempts the primary again (default
    /// [`DEFAULT_PROBE_INTERVAL`]).
    #[must_use]
    pub fn with_probe_interval(mut self, interval: Duration) -> Self {
        self.probe_interval = interval;
        self
    }

    /// Whether the primary is currently considered unavailable.
    pub fn is_degraded(&self) -> bool {
        self.lock().degraded
    }

    /// How many healthy-to-degraded transitions this instance has made: one per outage.
    pub fn degraded_transitions(&self) -> u64 {
        self.transitions.load(Ordering::SeqCst)
    }

    fn lock(&self) -> MutexGuard<'_, Latch> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Decide where one operation goes, claiming the probe when it is due.
    fn route(&self) -> Route<'_> {
        let mut latch = self.lock();
        if !latch.degraded {
            return Route::Primary;
        }
        let now = Instant::now();
        let due = latch
            .last_probe
            .is_none_or(|at| now.saturating_duration_since(at) >= self.probe_interval);
        if due
            && self
                .probing
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            // Set before the attempt so a slow probe cannot make the next one "due" early.
            latch.last_probe = Some(now);
            return Route::Probe(ProbeClaim(&self.probing));
        }
        Route::Fallback
    }

    /// Record a primary error: latch on the healthy-to-degraded edge only (one warning per
    /// outage), and restart the probe back-off.
    fn latch(&self, error: &CadenceError) {
        let edge = {
            let mut latch = self.lock();
            latch.last_probe = Some(Instant::now());
            let edge = !latch.degraded;
            latch.degraded = true;
            edge
        };
        if edge {
            self.transitions.fetch_add(1, Ordering::SeqCst);
            log::warn!(
                target: CADENCE_LOG_TARGET,
                "pacing degraded to per-process: shared pacing backend unavailable ({error}); \
                 delays x{:.1} until it recovers",
                self.fallback.multiplier()
            );
        }
    }

    /// Clear the latch after a successful probe: one `info` line per recovery.
    fn recover(&self) {
        let edge = {
            let mut latch = self.lock();
            let edge = latch.degraded;
            latch.degraded = false;
            edge
        };
        if edge {
            log::info!(
                target: CADENCE_LOG_TARGET,
                "pacing recovered: shared pacing backend reachable again"
            );
        }
    }

    /// The larger wait and streak of the primary's reading and the fallback's for `key`: a gate
    /// recorded in-process during an outage must outlive recovery (the max rule).
    async fn merged(&self, primary: GateReading, key: &CadenceKey) -> GateReading {
        match self.fallback.gate(key).await {
            Ok(local) => larger(primary, local),
            Err(_) => primary,
        }
    }
}

/// The reading with the larger wait and the larger streak of the two.
fn larger(a: GateReading, b: GateReading) -> GateReading {
    GateReading::new(a.wait().max(b.wait()), a.streak().max(b.streak()))
}

impl fmt::Debug for ResilientCadence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The degraded flag and the transition count only: nothing here can carry a key or URL.
        f.debug_struct("ResilientCadence")
            .field("degraded", &self.is_degraded())
            .field("degraded_transitions", &self.degraded_transitions())
            .finish()
    }
}

#[async_trait]
impl CadencePort for ResilientCadence {
    async fn gate(&self, key: &CadenceKey) -> Result<GateReading, CadenceError> {
        match self.route() {
            Route::Primary => match self.primary.gate(key).await {
                Ok(reading) => Ok(self.merged(reading, key).await),
                Err(error) => {
                    self.latch(&error);
                    self.fallback.gate(key).await
                }
            },
            Route::Probe(_claim) => match self.primary.gate(key).await {
                Ok(reading) => {
                    self.recover();
                    Ok(self.merged(reading, key).await)
                }
                // Still down: keep the latch, stay silent (the outage was already announced).
                Err(_) => self.fallback.gate(key).await,
            },
            Route::Fallback => self.fallback.gate(key).await,
        }
    }

    async fn record_rate_limited(
        &self,
        key: &CadenceKey,
        retry_after: Option<Duration>,
    ) -> Result<GateReading, CadenceError> {
        match self.route() {
            Route::Primary => match self.primary.record_rate_limited(key, retry_after).await {
                Ok(reading) => Ok(reading),
                Err(error) => {
                    self.latch(&error);
                    self.fallback.record_rate_limited(key, retry_after).await
                }
            },
            Route::Probe(_claim) => {
                match self.primary.record_rate_limited(key, retry_after).await {
                    Ok(reading) => {
                        self.recover();
                        Ok(self.merged(reading, key).await)
                    }
                    Err(_) => self.fallback.record_rate_limited(key, retry_after).await,
                }
            }
            Route::Fallback => self.fallback.record_rate_limited(key, retry_after).await,
        }
    }

    async fn record_success(&self, key: &CadenceKey) -> Result<(), CadenceError> {
        match self.route() {
            Route::Primary => match self.primary.record_success(key).await {
                Ok(()) => self.fallback.record_success(key).await,
                Err(error) => {
                    self.latch(&error);
                    self.fallback.record_success(key).await
                }
            },
            Route::Probe(_claim) => match self.primary.record_success(key).await {
                Ok(()) => {
                    self.recover();
                    self.fallback.record_success(key).await
                }
                Err(_) => self.fallback.record_success(key).await,
            },
            Route::Fallback => self.fallback.record_success(key).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cadence::{contract_tests, test_logger};
    use paladin_ports::output::cadence_port::CadencePolicy;
    use std::sync::atomic::AtomicUsize;
    use tokio::sync::Notify;

    const BASE: Duration = Duration::from_millis(40);
    const MAX: Duration = Duration::from_millis(640);
    /// A string no real error message contains, so a captured line can be tied to this stub.
    const MARKER: &str = "flaky-primary-marker-7f3a";

    fn policy() -> CadencePolicy {
        CadencePolicy::new(BASE, MAX).expect("valid policy")
    }

    fn key(model: &str) -> CadenceKey {
        CadenceKey::new("openai", model)
    }

    fn floor() -> f64 {
        0.0
    }

    fn near_one() -> f64 {
        0.999
    }

    /// A primary with a failure switch, a call counter and an optional hold, answering from a
    /// real in-memory table while it is "up".
    struct FlakyCadence {
        failing: AtomicBool,
        fail_with_timeout: AtomicBool,
        calls: AtomicUsize,
        hold: Mutex<Option<Arc<Notify>>>,
        inner: InMemoryCadence,
    }

    impl FlakyCadence {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                failing: AtomicBool::new(false),
                fail_with_timeout: AtomicBool::new(false),
                calls: AtomicUsize::new(0),
                hold: Mutex::new(None),
                inner: InMemoryCadence::new(policy()).with_jitter(floor),
            })
        }

        fn failing() -> Arc<Self> {
            let flaky = Self::new();
            flaky.set_failing(true);
            flaky
        }

        fn set_failing(&self, failing: bool) {
            self.failing.store(failing, Ordering::SeqCst);
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }

        fn hold_calls(&self) -> Arc<Notify> {
            let notify = Arc::new(Notify::new());
            *self.hold.lock().expect("hold lock") = Some(Arc::clone(&notify));
            notify
        }

        fn release_hold(&self) {
            *self.hold.lock().expect("hold lock") = None;
        }

        async fn enter(&self) -> Result<(), CadenceError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let hold = self.hold.lock().expect("hold lock").clone();
            if let Some(notify) = hold {
                notify.notified().await;
            }
            if self.failing.load(Ordering::SeqCst) {
                return Err(if self.fail_with_timeout.load(Ordering::SeqCst) {
                    CadenceError::Timeout {
                        after: Duration::from_millis(500),
                    }
                } else {
                    CadenceError::Backend {
                        message: MARKER.to_string(),
                    }
                });
            }
            Ok(())
        }
    }

    #[async_trait]
    impl CadencePort for FlakyCadence {
        async fn gate(&self, key: &CadenceKey) -> Result<GateReading, CadenceError> {
            self.enter().await?;
            self.inner.gate(key).await
        }

        async fn record_rate_limited(
            &self,
            key: &CadenceKey,
            retry_after: Option<Duration>,
        ) -> Result<GateReading, CadenceError> {
            self.enter().await?;
            self.inner.record_rate_limited(key, retry_after).await
        }

        async fn record_success(&self, key: &CadenceKey) -> Result<(), CadenceError> {
            self.enter().await?;
            self.inner.record_success(key).await
        }
    }

    fn fallback(multiplier: f64) -> InMemoryCadence {
        InMemoryCadence::new(policy())
            .with_jitter(floor)
            .with_multiplier(multiplier)
    }

    fn resilient(primary: &Arc<FlakyCadence>, multiplier: f64) -> ResilientCadence {
        ResilientCadence::new(
            Arc::clone(primary) as Arc<dyn CadencePort>,
            fallback(multiplier),
        )
    }

    #[tokio::test(start_paused = true)]
    async fn healthy_primary_serves_every_call() {
        let primary = FlakyCadence::new();
        let cadence = resilient(&primary, 2.0);
        let k = key("m");

        let recorded = cadence.record_rate_limited(&k, None).await.expect("record");
        assert_eq!(
            recorded.wait(),
            BASE,
            "the primary's own (unmultiplied) gate"
        );
        let gated = cadence.gate(&k).await.expect("gate");
        assert_eq!(gated.wait(), BASE);
        cadence.record_success(&k).await.expect("success");

        assert_eq!(primary.calls(), 3, "every operation reached the primary");
        assert!(!cadence.is_degraded());
        assert_eq!(cadence.degraded_transitions(), 0);
        assert!(
            cadence
                .fallback
                .gate(&k)
                .await
                .expect("fallback")
                .is_clear(),
            "the fallback was never written while the primary answered"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn first_primary_error_latches_and_serves_the_same_call_from_the_fallback() {
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 2.0);
        let k = key("m");

        let reading = cadence
            .record_rate_limited(&k, None)
            .await
            .expect("the composite never errors");
        assert_eq!(
            reading.wait(),
            BASE * 2,
            "served by the fallback on the failing call"
        );
        assert!(cadence.is_degraded());
        assert_eq!(cadence.degraded_transitions(), 1);

        // The 429 recorded during the outage gates the key: a run is never unpaced.
        let gated = cadence.gate(&k).await.expect("gate");
        assert_eq!(gated.wait(), BASE * 2);
        assert_eq!(
            primary.calls(),
            1,
            "no further primary attempt inside the probe interval"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_timeout_latches_exactly_like_a_backend_error() {
        let primary = FlakyCadence::failing();
        primary.fail_with_timeout.store(true, Ordering::SeqCst);
        let cadence = resilient(&primary, 1.0);

        let reading = cadence
            .record_rate_limited(&key("m"), None)
            .await
            .expect("record");
        assert_eq!(reading.wait(), BASE);
        assert!(cadence.is_degraded());
    }

    #[tokio::test(start_paused = true)]
    async fn degraded_delays_use_the_multiplier() {
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 2.0);

        // The latching call and a later fallback-only call.
        let delayless = cadence
            .record_rate_limited(&key("a"), None)
            .await
            .expect("a");
        assert_eq!(
            delayless.wait(),
            BASE * 2,
            "2 x base for a delay-less first 429"
        );

        let explicit = cadence
            .record_rate_limited(&key("b"), Some(Duration::from_secs(3)))
            .await
            .expect("b");
        assert_eq!(
            explicit.wait(),
            Duration::from_secs(6),
            "a provider minimum is multiplied up"
        );
        assert!(explicit.wait() >= Duration::from_secs(3), "never reduced");
    }

    #[tokio::test(start_paused = true)]
    async fn one_warning_per_outage_and_one_recovery_line() {
        test_logger::install();
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 2.0);
        let k = key("m");

        // 100 calls in one outage, spread over the three operations.
        for i in 0..100 {
            match i % 3 {
                0 => drop(cadence.gate(&k).await.expect("gate")),
                1 => drop(cadence.record_rate_limited(&k, None).await.expect("record")),
                _ => cadence.record_success(&k).await.expect("success"),
            }
        }
        assert_eq!(cadence.degraded_transitions(), 1);
        let lines = test_logger::lines_for_this_thread();
        let warnings: Vec<_> = lines
            .iter()
            .filter(|(level, _)| *level == log::Level::Warn)
            .collect();
        assert_eq!(
            warnings.len(),
            1,
            "exactly one warning for the outage: {lines:?}"
        );
        let warning = &warnings[0].1;
        assert!(
            warning.contains(MARKER),
            "the warning carries the error: {warning}"
        );
        assert!(
            warning.contains("x2.0"),
            "and the degraded multiplier: {warning}"
        );
        assert!(
            !warning.contains("redis://"),
            "T-43-32: no URL in the line: {warning}"
        );
        assert_eq!(
            lines.len(),
            1,
            "nothing else was logged during the outage: {lines:?}"
        );

        // Recovery: the next successful round trip after the probe interval.
        primary.set_failing(false);
        tokio::time::advance(DEFAULT_PROBE_INTERVAL).await;
        cadence.gate(&k).await.expect("probe");
        assert!(!cadence.is_degraded());
        let infos: Vec<_> = test_logger::lines_for_this_thread()
            .into_iter()
            .filter(|(level, _)| *level == log::Level::Info)
            .collect();
        assert_eq!(infos.len(), 1, "exactly one recovery line: {infos:?}");

        // Healthy again, no more lines; then a second outage warns a second time.
        for _ in 0..10 {
            cadence.gate(&k).await.expect("healthy gate");
        }
        assert_eq!(test_logger::lines_for_this_thread().len(), 2);
        primary.set_failing(true);
        for _ in 0..10 {
            cadence.gate(&k).await.expect("second outage gate");
        }
        assert_eq!(cadence.degraded_transitions(), 2);
        let warnings = test_logger::warnings_for_this_thread();
        assert_eq!(
            warnings.len(),
            2,
            "a second outage warns a second time: {warnings:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_failures_in_one_outage_warn_once() {
        test_logger::install();
        // Two callers both routed to a healthy-looking primary, both then failing: only the
        // first observation is the healthy-to-degraded edge.
        let primary = FlakyCadence::new();
        let cadence = Arc::new(resilient(&primary, 1.0));
        let release = primary.hold_calls();
        let callers: Vec<_> = (0..2)
            .map(|_| {
                let cadence = Arc::clone(&cadence);
                tokio::spawn(async move { cadence.gate(&key("m")).await })
            })
            .collect();
        tokio::task::yield_now().await;
        assert_eq!(primary.calls(), 2, "both callers are inside the primary");

        primary.set_failing(true);
        release.notify_one();
        release.notify_one();
        for caller in callers {
            caller
                .await
                .expect("join")
                .expect("the composite never errors");
        }
        assert_eq!(cadence.degraded_transitions(), 1);
        assert_eq!(test_logger::warnings_for_this_thread().len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn primary_is_probed_at_most_once_per_interval_while_degraded() {
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 1.0);
        let k = key("m");

        cadence.gate(&k).await.expect("latching call");
        assert_eq!(primary.calls(), 1);
        for _ in 0..50 {
            cadence.gate(&k).await.expect("degraded gate");
        }
        tokio::time::advance(DEFAULT_PROBE_INTERVAL - Duration::from_millis(1)).await;
        for _ in 0..50 {
            cadence.gate(&k).await.expect("degraded gate");
        }
        assert_eq!(
            primary.calls(),
            1,
            "no probe before the interval has elapsed"
        );

        tokio::time::advance(Duration::from_millis(1)).await;
        for _ in 0..50 {
            cadence.gate(&k).await.expect("degraded gate");
        }
        assert_eq!(primary.calls(), 2, "exactly one probe per elapsed interval");
        assert!(cadence.is_degraded(), "a failed probe keeps the latch");

        tokio::time::advance(DEFAULT_PROBE_INTERVAL).await;
        primary.set_failing(false);
        cadence.gate(&k).await.expect("recovering probe");
        assert_eq!(primary.calls(), 3);
        assert!(
            !cadence.is_degraded(),
            "the first successful probe clears the latch"
        );
        cadence.gate(&k).await.expect("healthy gate");
        assert_eq!(primary.calls(), 4, "later calls go to the primary again");
    }

    #[tokio::test(start_paused = true)]
    async fn the_probe_interval_is_configurable() {
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 1.0).with_probe_interval(Duration::from_secs(1));
        cadence.gate(&key("m")).await.expect("latch");
        tokio::time::advance(Duration::from_secs(1)).await;
        cadence.gate(&key("m")).await.expect("probe");
        assert_eq!(primary.calls(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_callers_do_not_wait_on_a_probe() {
        let primary = FlakyCadence::failing();
        let cadence = Arc::new(resilient(&primary, 1.0));
        let k = key("m");
        cadence.gate(&k).await.expect("latch");

        // The next probe is due, the primary is back up but slow: the probe blocks.
        tokio::time::advance(DEFAULT_PROBE_INTERVAL).await;
        primary.set_failing(false);
        let release = primary.hold_calls();
        let prober = {
            let cadence = Arc::clone(&cadence);
            let k = k.clone();
            tokio::spawn(async move { cadence.gate(&k).await })
        };
        tokio::task::yield_now().await;
        assert_eq!(primary.calls(), 2, "the probe is in flight");

        // Meanwhile another caller is served by the fallback at once. Were it to wait on the
        // probe, the paused clock would auto-advance and the timeout would fire.
        let served = tokio::time::timeout(Duration::from_millis(10), cadence.gate(&k))
            .await
            .expect("a concurrent caller must not wait on the probe");
        assert!(served.expect("gate").is_clear());
        assert_eq!(primary.calls(), 2, "and it did not touch the primary");

        release.notify_one();
        let probed = prober.await.expect("join").expect("probe result");
        assert!(probed.is_clear());
        assert!(!cadence.is_degraded());
    }

    #[tokio::test(start_paused = true)]
    async fn a_cancelled_probe_releases_its_claim() {
        let primary = FlakyCadence::failing();
        let cadence = Arc::new(resilient(&primary, 1.0));
        let k = key("m");
        cadence.gate(&k).await.expect("latch");

        tokio::time::advance(DEFAULT_PROBE_INTERVAL).await;
        primary.set_failing(false);
        let _release = primary.hold_calls();
        let prober = {
            let cadence = Arc::clone(&cadence);
            let k = k.clone();
            tokio::spawn(async move { cadence.gate(&k).await })
        };
        tokio::task::yield_now().await;
        prober.abort();
        assert!(prober.await.expect_err("aborted").is_cancelled());

        // The next interval can probe again: the abandoned claim did not wedge recovery.
        primary.release_hold();
        tokio::time::advance(DEFAULT_PROBE_INTERVAL).await;
        cadence.gate(&k).await.expect("probe");
        assert!(!cadence.is_degraded());
    }

    #[tokio::test(start_paused = true)]
    async fn gate_recorded_during_the_outage_survives_recovery() {
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 1.0);
        let k = key("m");

        let gated = cadence
            .record_rate_limited(&k, Some(Duration::from_secs(10)))
            .await
            .expect("record during the outage");
        assert_eq!(gated.wait(), Duration::from_secs(10));

        tokio::time::advance(DEFAULT_PROBE_INTERVAL).await;
        primary.set_failing(false);
        // The primary knows nothing of this key, but the key must not be released early.
        let after_recovery = cadence.gate(&k).await.expect("recovering gate");
        assert!(!cadence.is_degraded());
        assert_eq!(
            after_recovery.wait(),
            Duration::from_secs(5),
            "the fallback's remaining wait"
        );
        assert_eq!(after_recovery.streak(), 1);

        let later = cadence.gate(&k).await.expect("healthy gate");
        assert_eq!(
            later.wait(),
            Duration::from_secs(5),
            "still honoured on the healthy path"
        );
        tokio::time::advance(Duration::from_secs(5)).await;
        assert!(cadence.gate(&k).await.expect("elapsed").is_clear());
    }

    #[tokio::test(start_paused = true)]
    async fn a_success_through_a_recovering_probe_clears_the_fallback_streak() {
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 1.0);
        let k = key("m");
        cadence.record_rate_limited(&k, None).await.expect("record");
        assert_eq!(
            cadence.fallback.gate(&k).await.expect("fallback").streak(),
            1
        );

        tokio::time::advance(DEFAULT_PROBE_INTERVAL).await;
        primary.set_failing(false);
        cadence
            .record_success(&k)
            .await
            .expect("recovering success");
        assert!(!cadence.is_degraded());
        assert_eq!(
            cadence.fallback.gate(&k).await.expect("fallback").streak(),
            0
        );
    }

    #[tokio::test(start_paused = true)]
    async fn debug_output_is_the_latch_state_only() {
        let primary = FlakyCadence::failing();
        let cadence = resilient(&primary, 1.0);
        cadence.gate(&key("secret-model")).await.expect("gate");
        let rendered = format!("{cadence:?}");
        assert!(rendered.contains("degraded: true"), "{rendered}");
        assert!(rendered.contains("degraded_transitions: 1"), "{rendered}");
        assert!(!rendered.contains("secret-model"), "{rendered}");
    }

    // --- The shared contract suite, unchanged, against the composite.

    #[tokio::test(start_paused = true)]
    async fn resilient_cadence_passes_the_contract_healthy() {
        for jitter in [floor as fn() -> f64, near_one as fn() -> f64] {
            let cadence = ResilientCadence::new(
                Arc::new(InMemoryCadence::new(policy()).with_jitter(jitter)),
                InMemoryCadence::new(policy()).with_jitter(jitter),
            );
            contract_tests::run_all(&cadence, policy()).await;
            contract_tests::run_all_paused(&cadence, policy()).await;
            assert!(!cadence.is_degraded());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn resilient_cadence_passes_the_contract_degraded() {
        // An always-failing primary, a fallback at multiplier 1.0: the composite must hold the
        // same pacing contract as any adapter even when only the fallback is serving.
        for jitter in [floor as fn() -> f64, near_one as fn() -> f64] {
            let primary = FlakyCadence::failing();
            let cadence = ResilientCadence::new(
                Arc::clone(&primary) as Arc<dyn CadencePort>,
                InMemoryCadence::new(policy())
                    .with_jitter(jitter)
                    .with_multiplier(1.0),
            );
            contract_tests::run_all(&cadence, policy()).await;
            contract_tests::run_all_paused(&cadence, policy()).await;
            assert!(cadence.is_degraded());
            assert_eq!(cadence.degraded_transitions(), 1);
        }
    }
}
