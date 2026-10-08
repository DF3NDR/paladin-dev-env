/*
In-Process Cadence

A `std::sync::Mutex<HashMap<CadenceKey, GateState>>`-backed implementation of
`CadencePort` (PACE-02, D-04, D-07), ungated like `node_cache::in_memory`.

State exists only after a provider 429 (reactive pacing, D-04): `gate` never
inserts, and `record_success` drops an entry once its streak is zero and its
gate has elapsed. Time is `tokio::time::Instant` so paused-clock tests drive
the whole table without sleeping for real. Every deadline is computed with
`checked_add` and every delay is clamped at `CADENCE_DELAY_CEILING`, so a
provider-controlled number can never panic this adapter.

The table is bounded (research Pitfall 11, T-43-16): at most `DEFAULT_KEY_CAPACITY`
keys (adjustable with `with_capacity`). Keys are created only by a 429, but the model half
of a key comes from the request, so a hostile caller could otherwise grow the map without
limit. Inserting a new key at the cap first reclaims entries whose gate has elapsed with
streak zero (they read exactly like an absent key); if every entry is still live, the entry
whose gate expires soonest is evicted -- it is the one that costs the least pacing -- and one
warning is logged, naming the capacity and never a key or model.

The lock is never held across an `.await`; a poisoned lock is recovered with
`PoisonError::into_inner`, because the state is a pacing hint, never a
correctness dependency.
*/

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::Instant;

use paladin_ports::output::cadence_port::{
    CADENCE_DELAY_CEILING, CADENCE_LOG_TARGET, CadenceError, CadenceKey, CadencePolicy,
    CadencePort, GateReading,
};

/// The default bound on distinct keys held in-process (research Pitfall 11).
///
/// A key is created only by a provider 429 and dropped once its gate has elapsed and a
/// success reset its streak, so a real workload holds a handful; the bound exists so a
/// caller-chosen model string cannot grow the table without limit.
pub const DEFAULT_KEY_CAPACITY: usize = 4096;

/// One key's pacing state.
#[derive(Debug, Clone, Copy)]
struct GateState {
    /// The instant the gate clears. `now >= not_before` reads as clear.
    not_before: Instant,
    /// Consecutive escalating 429s since the last success.
    streak: u32,
}

impl GateState {
    fn reading(&self, now: Instant) -> GateReading {
        GateReading::new(self.not_before.saturating_duration_since(now), self.streak)
    }
}

/// In-process `CadencePort` implementation.
pub struct InMemoryCadence {
    policy: CadencePolicy,
    jitter: fn() -> f64,
    capacity: usize,
    /// Set the first time a live entry had to be evicted at the cap; gates the one-time warning.
    cap_announced: AtomicBool,
    state: Mutex<HashMap<CadenceKey, GateState>>,
}

impl InMemoryCadence {
    /// Build an adapter whose delay-less gates draw jitter from
    /// `rand::random::<f64>`.
    pub fn new(policy: CadencePolicy) -> Self {
        Self {
            policy,
            jitter: rand::random::<f64>,
            capacity: DEFAULT_KEY_CAPACITY,
            cap_announced: AtomicBool::new(false),
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Replace the jitter source -- the deterministic seam for tests. The
    /// function must return a fraction in `[0, 1)`; anything else is clamped
    /// by [`CadencePolicy::delay_for`].
    #[must_use]
    pub fn with_jitter(mut self, jitter: fn() -> f64) -> Self {
        self.jitter = jitter;
        self
    }

    /// Bound the number of distinct keys held (default [`DEFAULT_KEY_CAPACITY`]). A capacity of
    /// zero is treated as one: the table must be able to hold the key being recorded.
    #[must_use]
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity.max(1);
        self
    }

    /// Make room for one new key: reclaim entries that read as absent (gate elapsed, streak
    /// zero), then, if the table is still full, evict the live entry whose gate expires soonest
    /// and announce the cap once.
    fn make_room(&self, state: &mut HashMap<CadenceKey, GateState>, now: Instant) {
        state.retain(|_, entry| !(entry.streak == 0 && now >= entry.not_before));
        while state.len() >= self.capacity {
            let soonest = state
                .iter()
                .min_by_key(|(_, entry)| entry.not_before)
                .map(|(key, _)| key.clone());
            let Some(soonest) = soonest else { break };
            state.remove(&soonest);
            if !self.cap_announced.swap(true, Ordering::SeqCst) {
                log::warn!(
                    target: CADENCE_LOG_TARGET,
                    "in-process cadence table reached its capacity of {} keys; evicting the \
                     entry whose gate expires soonest to make room",
                    self.capacity
                );
            }
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<CadenceKey, GateState>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for InMemoryCadence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Entry count only: keys carry model strings, kept out of Debug output.
        f.debug_struct("InMemoryCadence")
            .field("policy", &self.policy)
            .field("entries", &self.lock().len())
            .finish()
    }
}

/// `now + delay`, saturating at the delay ceiling and never panicking.
fn deadline_after(now: Instant, delay: Duration) -> Instant {
    now.checked_add(delay.min(CADENCE_DELAY_CEILING))
        .or_else(|| now.checked_add(Duration::from_secs(1)))
        .unwrap_or(now)
}

#[async_trait]
impl CadencePort for InMemoryCadence {
    async fn gate(&self, key: &CadenceKey) -> Result<GateReading, CadenceError> {
        let now = Instant::now();
        let state = self.lock();
        Ok(state
            .get(key)
            .map_or_else(GateReading::default, |entry| entry.reading(now)))
    }

    async fn record_rate_limited(
        &self,
        key: &CadenceKey,
        retry_after: Option<Duration>,
    ) -> Result<GateReading, CadenceError> {
        let now = Instant::now();
        let mut state = self.lock();
        if self.capacity <= state.len() && !state.contains_key(key) {
            self.make_room(&mut state, now);
        }
        let entry = state.entry(key.clone()).or_insert(GateState {
            not_before: now,
            streak: 0,
        });

        if now < entry.not_before {
            // In-flight rule (research Pattern 3): this 429 answers a request
            // sent before the gate opened. No escalation; only a later
            // provider-supplied deadline can extend the gate.
            if let Some(provided) = retry_after {
                let candidate = deadline_after(now, provided);
                if candidate > entry.not_before {
                    entry.not_before = candidate;
                }
            }
        } else {
            entry.streak = entry.streak.saturating_add(1);
            let delay = self
                .policy
                .delay_for(entry.streak, retry_after, (self.jitter)());
            entry.not_before = deadline_after(now, delay);
        }
        Ok(entry.reading(now))
    }

    async fn record_success(&self, key: &CadenceKey) -> Result<(), CadenceError> {
        let now = Instant::now();
        let mut state = self.lock();
        if let Some(entry) = state.get_mut(key) {
            entry.streak = 0;
            if now >= entry.not_before {
                // Gate also elapsed: state exists only after a 429 (Pitfall 11).
                state.remove(key);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cadence::contract_tests;

    /// Tens of milliseconds, per the contract suite's guidance; a maximum of
    /// at least eight times the base so the escalation clause sees growth.
    const BASE: Duration = Duration::from_millis(40);
    const MAX: Duration = Duration::from_millis(640);

    fn policy() -> CadencePolicy {
        CadencePolicy::new(BASE, MAX).expect("valid policy")
    }

    fn adapter(jitter: fn() -> f64) -> InMemoryCadence {
        InMemoryCadence::new(policy()).with_jitter(jitter)
    }

    fn key(model: &str) -> CadenceKey {
        CadenceKey::new("openai", model)
    }

    fn entries(cadence: &InMemoryCadence) -> usize {
        cadence.lock().len()
    }

    // --- The shared contract suite, one named test per clause so a failure
    // names the violated rule. Jitter is pinned to the floor (0.0) for the
    // per-clause tests; `run_all` below runs the whole suite at both ends.

    #[tokio::test(start_paused = true)]
    async fn contract_unknown_key_is_clear_and_creates_no_state() {
        contract_tests::unknown_key_is_clear_and_creates_no_state(&adapter(|| 0.0), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_first_delayless_429_gates_for_base() {
        contract_tests::first_delayless_429_gates_for_base(&adapter(|| 0.0), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_gate_boundary_is_closed_at_not_before_paused() {
        contract_tests::gate_boundary_is_closed_at_not_before_paused(&adapter(|| 0.0), policy())
            .await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_escalation_doubles_the_ceiling_after_the_gate_clears() {
        contract_tests::escalation_doubles_the_ceiling_after_the_gate_clears(
            &adapter(|| 0.999),
            policy(),
        )
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_in_flight_429_does_not_escalate() {
        contract_tests::in_flight_429_does_not_escalate(&adapter(|| 0.0), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_explicit_retry_after_raises_but_never_lowers_an_active_gate() {
        contract_tests::explicit_retry_after_raises_but_never_lowers_an_active_gate(
            &adapter(|| 0.0),
            policy(),
        )
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_explicit_retry_after_is_the_exact_gate() {
        contract_tests::explicit_retry_after_is_the_exact_gate(&adapter(|| 0.0), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_success_resets_the_streak_and_is_idempotent() {
        contract_tests::success_resets_the_streak_and_is_idempotent(&adapter(|| 0.0), policy())
            .await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_streak_saturates_without_overflow() {
        contract_tests::streak_saturates_without_overflow(&adapter(|| 0.999), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_keys_are_exact_byte_equal() {
        contract_tests::keys_are_exact_byte_equal(&adapter(|| 0.0), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_concurrent_429s_escalate_at_most_once() {
        contract_tests::concurrent_429s_escalate_at_most_once(&adapter(|| 0.0), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_run_all_at_the_jitter_floor() {
        // Jitter 0.0: a delay-less gate must still be the base, never zero.
        contract_tests::run_all(&adapter(|| 0.0), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_run_all_at_the_jitter_ceiling() {
        contract_tests::run_all(&adapter(|| 0.999), policy()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn contract_run_all_paused_at_both_jitter_ends() {
        contract_tests::run_all_paused(&adapter(|| 0.0), policy()).await;
        contract_tests::run_all_paused(&adapter(|| 0.999), policy()).await;
    }

    // --- In-memory specifics the port contract cannot observe.

    #[tokio::test(start_paused = true)]
    async fn in_memory_reads_and_a_success_on_an_unknown_key_insert_nothing() {
        let cadence = adapter(|| 0.999);
        let reading = cadence.gate(&key("m")).await.expect("gate");
        assert!(reading.is_clear());
        cadence.record_success(&key("m")).await.expect("success");
        assert_eq!(entries(&cadence), 0, "a read must not create state");
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_success_drops_an_elapsed_entry_and_keeps_an_active_one() {
        let cadence = adapter(|| 0.999);
        let k = key("m");
        cadence.record_rate_limited(&k, None).await.expect("first");
        tokio::time::advance(BASE).await;
        cadence.record_rate_limited(&k, None).await.expect("second");
        tokio::time::advance(MAX).await;

        cadence.record_success(&k).await.expect("success");
        assert_eq!(entries(&cadence), 0, "an elapsed, reset entry is dropped");

        // An entry whose gate is still active is kept, with its gate intact.
        cadence.record_rate_limited(&k, None).await.expect("third");
        cadence.record_success(&k).await.expect("success");
        let reading = cadence.gate(&k).await.expect("gate");
        assert_eq!(reading.streak(), 0);
        assert_eq!(reading.wait(), BASE, "the gate itself still holds");
        assert_eq!(entries(&cadence), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_hostile_retry_after_is_clamped_to_the_ceiling_and_never_panics() {
        let cadence = adapter(|| 0.5);
        let k = key("m");
        let reading = cadence
            .record_rate_limited(&k, Some(Duration::MAX))
            .await
            .expect("record");
        assert_eq!(reading.wait(), CADENCE_DELAY_CEILING);

        let inflight = cadence
            .record_rate_limited(&k, Some(Duration::MAX))
            .await
            .expect("in-flight");
        assert_eq!(inflight.wait(), CADENCE_DELAY_CEILING);
    }

    #[test]
    fn in_memory_debug_reports_entry_count_only() {
        let cadence = adapter(|| 0.0);
        let rendered = format!("{cadence:?}");
        assert!(rendered.contains("entries: 0"), "{rendered}");
    }

    // --- Bounded in-process state (T-43-16, research Pitfall 11).

    #[test]
    fn default_capacity_is_4096_and_zero_is_treated_as_one() {
        assert_eq!(DEFAULT_KEY_CAPACITY, 4096);
        assert_eq!(adapter(|| 0.0).capacity, DEFAULT_KEY_CAPACITY);
        assert_eq!(adapter(|| 0.0).with_capacity(0).capacity, 1);
        assert_eq!(adapter(|| 0.0).with_capacity(7).capacity, 7);
    }

    #[tokio::test(start_paused = true)]
    async fn capacity_evicts_elapsed_idle_entries_first() {
        let cadence = adapter(|| 0.0).with_capacity(3);
        let idle_a = key("idle-a");
        let idle_b = key("idle-b");
        let live = key("live");

        // Two entries whose gates elapse and whose streak is then reset to
        // zero by a success that arrived while the gate was still active.
        for k in [&idle_a, &idle_b] {
            cadence.record_rate_limited(k, None).await.expect("record");
            cadence.record_success(k).await.expect("success");
        }
        cadence
            .record_rate_limited(&live, Some(Duration::from_secs(600)))
            .await
            .expect("live");
        assert_eq!(entries(&cadence), 3);

        // The idle entries' gates elapse; the live one's is far off.
        tokio::time::advance(BASE).await;

        cadence
            .record_rate_limited(&key("fourth"), None)
            .await
            .expect("fourth");

        let state = cadence.lock();
        assert!(!state.contains_key(&idle_a), "elapsed idle entry evicted");
        assert!(!state.contains_key(&idle_b), "elapsed idle entry evicted");
        assert!(
            state.contains_key(&live),
            "a live entry survives idle eviction"
        );
        assert!(state.contains_key(&key("fourth")));
        assert!(
            !cadence.cap_announced.load(Ordering::SeqCst),
            "reclaiming idle entries is not hitting the cap"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn capacity_at_the_cap_with_live_entries_evicts_the_soonest_expiring_and_warns_once() {
        install_capturing_logger();
        // A capacity no other test in this binary uses, so the capture below
        // (scoped to this OS thread as well) can only be this test's line.
        let cadence = adapter(|| 0.0).with_capacity(3);
        let soonest = key("m-early");
        let middle = key("m-mid");
        let latest = key("m-late");
        for (k, secs) in [(&latest, 900), (&soonest, 100), (&middle, 500)] {
            cadence
                .record_rate_limited(k, Some(Duration::from_secs(secs)))
                .await
                .expect("record");
        }
        assert_eq!(entries(&cadence), 3);

        cadence
            .record_rate_limited(&key("m-new-1"), Some(Duration::from_secs(700)))
            .await
            .expect("newcomer 1");
        {
            let state = cadence.lock();
            assert_eq!(state.len(), 3, "the table never exceeds its capacity");
            assert!(
                !state.contains_key(&soonest),
                "the soonest-expiring is evicted"
            );
            assert!(state.contains_key(&middle) && state.contains_key(&latest));
            assert!(state.contains_key(&key("m-new-1")));
        }

        cadence
            .record_rate_limited(&key("m-new-2"), Some(Duration::from_secs(800)))
            .await
            .expect("newcomer 2");
        {
            let state = cadence.lock();
            assert_eq!(state.len(), 3);
            assert!(
                !state.contains_key(&middle),
                "then the next soonest is evicted"
            );
        }

        let warnings = captured_warnings_for_this_thread();
        assert_eq!(
            warnings.len(),
            1,
            "exactly one warning, the first time the cap is hit: {warnings:?}"
        );
        assert!(
            warnings[0].contains('3'),
            "names the capacity: {}",
            warnings[0]
        );
        for secret in ["m-early", "m-mid", "m-late", "m-new", "openai"] {
            assert!(
                !warnings[0].contains(secret),
                "the warning must never name a key or model: {}",
                warnings[0]
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn capacity_does_not_evict_when_the_key_already_exists() {
        let cadence = adapter(|| 0.0).with_capacity(2);
        let a = key("a");
        let b = key("b");
        cadence
            .record_rate_limited(&a, Some(Duration::from_secs(60)))
            .await
            .expect("a");
        cadence
            .record_rate_limited(&b, Some(Duration::from_secs(60)))
            .await
            .expect("b");
        cadence
            .record_rate_limited(&a, Some(Duration::from_secs(90)))
            .await
            .expect("a again");
        assert_eq!(entries(&cadence), 2);
        assert!(!cadence.cap_announced.load(Ordering::SeqCst));
    }

    // A minimal in-process `log::Log` capturer, scoped by OS thread id so
    // concurrently running `#[tokio::test]`s (each its own OS thread) never
    // observe each other's lines. Installed at most once per process.
    struct CapturingLogger;

    static LOGGER_INIT: std::sync::Once = std::sync::Once::new();
    static CAPTURED: Mutex<Option<CapturedLines>> = Mutex::new(None);
    type CapturedLines = HashMap<std::thread::ThreadId, Vec<String>>;

    impl log::Log for CapturingLogger {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            if record.target() == CADENCE_LOG_TARGET {
                let mut guard = CAPTURED.lock().unwrap_or_else(PoisonError::into_inner);
                guard
                    .get_or_insert_with(HashMap::new)
                    .entry(std::thread::current().id())
                    .or_default()
                    .push(record.args().to_string());
            }
        }
        fn flush(&self) {}
    }

    fn install_capturing_logger() {
        static LOGGER: CapturingLogger = CapturingLogger;
        LOGGER_INIT.call_once(|| {
            log::set_logger(&LOGGER).expect("install the capturing test logger");
            log::set_max_level(log::LevelFilter::Warn);
        });
    }

    fn captured_warnings_for_this_thread() -> Vec<String> {
        CAPTURED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .and_then(|lines| lines.get(&std::thread::current().id()).cloned())
            .unwrap_or_default()
    }
}
