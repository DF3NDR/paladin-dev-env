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

The lock is never held across an `.await`; a poisoned lock is recovered with
`PoisonError::into_inner`, because the state is a pacing hint, never a
correctness dependency.
*/

use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::Instant;

use paladin_ports::output::cadence_port::{
    CADENCE_DELAY_CEILING, CadenceError, CadenceKey, CadencePolicy, CadencePort, GateReading,
};

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
    state: Mutex<HashMap<CadenceKey, GateState>>,
}

impl InMemoryCadence {
    /// Build an adapter whose delay-less gates draw jitter from
    /// `rand::random::<f64>`.
    pub fn new(policy: CadencePolicy) -> Self {
        Self {
            policy,
            jitter: rand::random::<f64>,
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

    const BASE: Duration = Duration::from_millis(500);
    const MAX: Duration = Duration::from_secs(30);

    fn adapter(jitter: fn() -> f64) -> InMemoryCadence {
        let policy = CadencePolicy::new(BASE, MAX).expect("valid policy");
        InMemoryCadence::new(policy).with_jitter(jitter)
    }

    fn key(model: &str) -> CadenceKey {
        CadenceKey::new("openai", model)
    }

    fn entries(cadence: &InMemoryCadence) -> usize {
        cadence.lock().len()
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_gate_on_unknown_key_is_clear_and_inserts_nothing() {
        let cadence = adapter(|| 0.999);
        let reading = cadence.gate(&key("m")).await.expect("gate");
        assert_eq!(reading.wait(), Duration::ZERO);
        assert_eq!(reading.streak(), 0);
        assert!(reading.is_clear());
        assert_eq!(entries(&cadence), 0, "a read must not create state");
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_first_429_gates_for_base_then_clears() {
        let cadence = adapter(|| 0.999);
        let k = key("m");
        let after = cadence.record_rate_limited(&k, None).await.expect("record");
        assert_eq!(after.wait(), BASE);
        assert_eq!(after.streak(), 1);
        assert_eq!(cadence.gate(&k).await.expect("gate").wait(), BASE);

        tokio::time::advance(BASE).await;
        let cleared = cadence.gate(&k).await.expect("gate");
        assert_eq!(
            cleared.wait(),
            Duration::ZERO,
            "closed boundary at not_before"
        );
        assert_eq!(cleared.streak(), 1, "the streak survives until a success");
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_second_429_after_the_gate_clears_escalates_the_streak() {
        let cadence = adapter(|| 0.999);
        let k = key("m");
        cadence.record_rate_limited(&k, None).await.expect("first");
        tokio::time::advance(BASE).await;

        let second = cadence.record_rate_limited(&k, None).await.expect("second");
        assert_eq!(second.streak(), 2);
        assert!(second.wait() >= BASE, "gate {:?} below base", second.wait());
        assert!(
            second.wait() <= MAX.min(BASE * 2),
            "gate {:?} above the streak-2 ceiling",
            second.wait()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_429_inside_an_active_gate_does_not_escalate_and_raises_to_a_larger_retry_after()
     {
        let cadence = adapter(|| 0.999);
        let k = key("m");
        cadence.record_rate_limited(&k, None).await.expect("first");

        // A second 429 while the gate is active: same streak, same gate.
        let inflight = cadence
            .record_rate_limited(&k, None)
            .await
            .expect("inflight");
        assert_eq!(inflight.streak(), 1, "in-flight 429 must not escalate");
        assert_eq!(inflight.wait(), BASE);

        // A smaller provider delay never shortens the gate.
        let smaller = cadence
            .record_rate_limited(&k, Some(Duration::from_millis(100)))
            .await
            .expect("smaller");
        assert_eq!(smaller.wait(), BASE);

        // A larger provider delay raises it, still without escalating.
        let seven = Duration::from_secs(7);
        let raised = cadence
            .record_rate_limited(&k, Some(seven))
            .await
            .expect("raised");
        assert_eq!(raised.streak(), 1);
        assert_eq!(raised.wait(), seven);
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_success_resets_the_streak() {
        let cadence = adapter(|| 0.999);
        let k = key("m");
        cadence.record_rate_limited(&k, None).await.expect("first");
        tokio::time::advance(BASE).await;
        cadence.record_rate_limited(&k, None).await.expect("second");
        tokio::time::advance(MAX).await;

        cadence.record_success(&k).await.expect("success");
        assert_eq!(cadence.gate(&k).await.expect("gate").streak(), 0);
        assert_eq!(entries(&cadence), 0, "an elapsed, reset entry is dropped");

        // The next 429 starts from the base again.
        let fresh = cadence.record_rate_limited(&k, None).await.expect("fresh");
        assert_eq!(fresh.streak(), 1);
        assert_eq!(fresh.wait(), BASE);
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_success_keeps_an_entry_whose_gate_is_still_active() {
        let cadence = adapter(|| 0.999);
        let k = key("m");
        cadence.record_rate_limited(&k, None).await.expect("first");
        cadence.record_success(&k).await.expect("success");
        let reading = cadence.gate(&k).await.expect("gate");
        assert_eq!(reading.streak(), 0);
        assert_eq!(reading.wait(), BASE, "the gate itself still holds");
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_keys_differing_only_by_model_are_independent() {
        let cadence = adapter(|| 0.999);
        cadence
            .record_rate_limited(&key("gpt-a"), None)
            .await
            .expect("record");
        assert!(!cadence.gate(&key("gpt-a")).await.expect("a").is_clear());
        assert!(cadence.gate(&key("gpt-b")).await.expect("b").is_clear());
        assert!(
            cadence
                .gate(&CadenceKey::new("anthropic", "gpt-a"))
                .await
                .expect("other provider")
                .is_clear()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn in_memory_hostile_retry_after_is_clamped_and_never_panics() {
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

    #[tokio::test(start_paused = true)]
    async fn in_memory_streak_saturates_without_overflow() {
        let cadence = adapter(|| 0.999);
        let k = key("m");
        for _ in 0..70 {
            cadence.record_rate_limited(&k, None).await.expect("record");
            tokio::time::advance(MAX).await;
        }
        let reading = cadence.gate(&k).await.expect("gate");
        assert_eq!(reading.streak(), 70);
        assert!(reading.is_clear());
    }

    #[test]
    fn in_memory_debug_reports_entry_count_only() {
        let cadence = adapter(|| 0.0);
        let rendered = format!("{cadence:?}");
        assert!(rendered.contains("entries: 0"), "{rendered}");
    }
}
