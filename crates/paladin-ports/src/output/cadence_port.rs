//! # Cadence Port -- Cross-Call Rate Pacing (PACE-02, Phase 43)
//!
//! Cadence is the one who sets the marching pace. This module defines the port
//! trait behind which a process (and, with a shared backend, a fleet of workers)
//! remembers that a provider answered `429 Too Many Requests`, so the *next*
//! call to the same provider and model waits instead of walking into a second
//! refusal.
//!
//! ## Why a port
//!
//! Pacing state is cross-call: every decorator that talks to one provider must
//! see the same gate, and with a shared backend every worker must too. It
//! therefore cannot live in a decorator field -- the same provider wrapped in
//! two decorators would be paced twice, independently. The state lives behind
//! this port; the `CadenceLlmAdapter` decorator in `paladin-llm` is stateless.
//!
//! ## Read-only `gate`, state-changing `record_*`
//!
//! [`CadencePort::gate`] only reads. Only [`CadencePort::record_rate_limited`]
//! and [`CadencePort::record_success`] mutate. A caller cancelled while it is
//! sleeping on a gate therefore leaves no state behind (there is no "reserve a
//! slot, then sleep" step to leak), and a waiter that wakes re-checks the gate
//! because a concurrent 429 may have extended it.
//!
//! ## Reactive only
//!
//! A key is gated only *after* a 429 (D-04). Success headers never open a gate,
//! `gate` on an unknown key is clear and creates no state, and so an unbounded
//! set of model strings cannot grow the store on its own.
//!
//! ## The in-flight rule
//!
//! With N calls in flight on one key, a single provider hiccup produces up to N
//! 429s. A 429 recorded while the key's gate is still active belongs to a
//! request sent *before* the gate, so it does not increase the streak; it can
//! only raise the gate to a later deadline when the provider's own delay asks
//! for one. Only a 429 observed with the gate already clear escalates.
//!
//! ## Relative waits
//!
//! Every adapter reports waits as a [`Duration`] relative to "now", never as an
//! absolute timestamp, so no caller ever compares clocks across hosts.
//!
//! ## The stampede lock (PACE-04)
//!
//! The same port carries a small lock, [`CadencePort::try_lock`] and
//! [`CadencePort::unlock`], so workers about to compute the same costly result can let
//! one of them do it. It is set-if-absent with expiry, hands out strictly increasing
//! [`FencingToken`]s, and releases only for the token that owns it. Tokens carry their source
//! (shared backend or one process) so a resource that checks them never ranks a per-process
//! counter against a fleet-wide one. The lock is an optimisation: any error means "proceed
//! without it".

use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

/// The `log` target every Cadence line is emitted under (D-07), shared by
/// `paladin-storage` and `paladin-llm`.
///
/// Lines under this target carry provider names, model names and durations
/// only -- never request bodies, credentials or header values.
pub const CADENCE_LOG_TARGET: &str = "paladin::cadence";

/// The hard clamp (24 hours) applied to any delay that reaches a port.
///
/// Delays can be provider-controlled numbers (a `Retry-After` header); clamping
/// them before they become deadlines keeps deadline arithmetic from overflowing
/// and keeps one hostile header from gating a key for years.
pub const CADENCE_DELAY_CEILING: Duration = Duration::from_secs(24 * 60 * 60);

/// Identity of one paced lane: a provider and a model.
///
/// Equality is exact, case-sensitive byte equality. Both strings are opaque --
/// they are never parsed, so colons and slashes inside a model string are
/// harmless, and two keys that differ only in the model are independent.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::cadence_port::CadenceKey;
///
/// let key = CadenceKey::new("openai", "gpt-4o");
/// assert_eq!(key.provider(), "openai");
/// assert_eq!(key.model(), "gpt-4o");
/// assert_ne!(key, CadenceKey::new("openai", "gpt-4o-mini"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CadenceKey {
    provider: String,
    model: String,
}

impl CadenceKey {
    /// Build a key from a provider name and a model string.
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
        }
    }

    /// The provider name.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The model string, exactly as the request carried it.
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// What a port reports about a key: how long to wait and how many consecutive
/// escalations have been recorded.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_ports::output::cadence_port::GateReading;
///
/// let clear = GateReading::default();
/// assert!(clear.is_clear());
///
/// let gated = GateReading::new(Duration::from_millis(500), 1);
/// assert!(!gated.is_clear());
/// assert_eq!(gated.streak(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct GateReading {
    wait: Duration,
    streak: u32,
}

impl GateReading {
    /// Build a reading.
    pub fn new(wait: Duration, streak: u32) -> Self {
        Self { wait, streak }
    }

    /// How long a caller should wait before sending; zero means clear.
    pub fn wait(&self) -> Duration {
        self.wait
    }

    /// The live count of consecutive escalating 429s for the key.
    pub fn streak(&self) -> u32 {
        self.streak
    }

    /// Whether the key may be sent to right now (the wait is zero).
    pub fn is_clear(&self) -> bool {
        self.wait.is_zero()
    }
}

/// Identity of one stampede lock (PACE-04, D-11): an opaque string naming the work being
/// coalesced, for example the cache key of a node about to be computed.
///
/// Equality is exact, case-sensitive byte equality, and the string is never parsed, so a lock
/// named `a:b` can never alias a lock named `a` plus `b`.
///
/// # Examples
///
/// ```
/// use paladin_ports::output::cadence_port::LockKey;
///
/// let key = LockKey::new("graphA:node1:input-hash");
/// assert_eq!(key.as_str(), "graphA:node1:input-hash");
/// assert_ne!(key, LockKey::new("graphA:node1:INPUT-HASH"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LockKey(String);

impl LockKey {
    /// Build a lock key from any string-like value.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// The key exactly as given.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A fencing token: proof of holding a [`CadencePort`] lock, and the number a protected
/// resource compares to refuse a stale holder's late write (D-14).
///
/// A token carries its **source**. A [`FencingToken::Distributed`] token is issued by a shared
/// backend, so every worker's tokens for one key are ordered against each other. A
/// [`FencingToken::Local`] token is issued by one process's in-memory fallback and means nothing
/// outside it. The two are deliberately not comparable: the type has no `Ord`, and
/// `Local(1) != Distributed(1)`, so a resource can never rank a per-process counter against a
/// fleet-wide one by accident (research Pitfall 13).
///
/// # Examples
///
/// ```
/// use paladin_ports::output::cadence_port::FencingToken;
///
/// let shared = FencingToken::Distributed(7);
/// let local = FencingToken::Local(7);
/// assert_eq!(shared.value(), local.value());
/// assert_ne!(shared, local, "the same number from different sources is a different token");
/// assert!(shared.is_distributed());
/// assert!(!local.is_distributed());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FencingToken {
    /// Issued by a shared backend; comparable across workers for one lock key.
    Distributed(u64),
    /// Issued by one process; only meaningful inside it.
    Local(u64),
}

impl FencingToken {
    /// The counter value, whichever the source. Compare two values only after checking that
    /// both tokens are from the same source.
    pub fn value(&self) -> u64 {
        match self {
            Self::Distributed(value) | Self::Local(value) => *value,
        }
    }

    /// Whether a shared backend issued the token.
    pub fn is_distributed(&self) -> bool {
        matches!(self, Self::Distributed(_))
    }
}

/// Errors a [`CadencePort`] or [`CadencePolicy`] can report.
///
/// Messages never carry a URL or a credential.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CadenceError {
    /// A [`CadencePolicy`] was built from values that cannot pace anything.
    #[error("invalid cadence policy: {0}")]
    InvalidPolicy(String),
    /// The backend failed (a connection error, a protocol error, and so on).
    #[error("cadence backend error: {message}")]
    Backend {
        /// A redacted, human-readable description of the failure.
        message: String,
    },
    /// The backend did not answer within its deadline.
    #[error("cadence backend timed out after {after:?}")]
    Timeout {
        /// How long the call waited before giving up.
        after: Duration,
    },
}

/// The pure back-off arithmetic of the Cadence: how long a key is gated.
///
/// No randomness is drawn here; callers inject the jitter fraction, so the
/// whole table is deterministic and testable.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_ports::output::cadence_port::{CadenceError, CadencePolicy};
///
/// # fn main() -> Result<(), CadenceError> {
/// let policy = CadencePolicy::new(Duration::from_millis(500), Duration::from_secs(30))?;
///
/// // The first delay-less 429 gates for exactly the base back-off.
/// assert_eq!(policy.delay_for(1, None, 0.9), Duration::from_millis(500));
///
/// // A provider-supplied delay is a minimum: returned exactly, never jittered down.
/// let seven = Duration::from_secs(7);
/// assert_eq!(policy.delay_for(3, Some(seven), 0.1), seven);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CadencePolicy {
    base_backoff: Duration,
    max_backoff: Duration,
}

impl CadencePolicy {
    /// Build a policy.
    ///
    /// # Errors
    ///
    /// [`CadenceError::InvalidPolicy`] when `base` is zero or `base > max`.
    pub fn new(base: Duration, max: Duration) -> Result<Self, CadenceError> {
        if base.is_zero() {
            return Err(CadenceError::InvalidPolicy(
                "base back-off must be greater than zero".to_string(),
            ));
        }
        if base > max {
            return Err(CadenceError::InvalidPolicy(
                "base back-off must not exceed the maximum back-off".to_string(),
            ));
        }
        Ok(Self {
            base_backoff: base,
            max_backoff: max,
        })
    }

    /// The first gate length for a delay-less 429.
    pub fn base_backoff(&self) -> Duration {
        self.base_backoff
    }

    /// The largest delay-less gate.
    pub fn max_backoff(&self) -> Duration {
        self.max_backoff
    }

    /// How long a key is gated after a 429.
    ///
    /// * `Some(d)` -- the provider's own delay is a minimum, so it is returned
    ///   exactly, clamped only to [`CADENCE_DELAY_CEILING`]. It is never
    ///   jittered downwards; spread between waiters is the caller's business.
    /// * `None` -- `ceiling = min(max_backoff, base * 2^(max(streak, 1) - 1))`
    ///   with saturating arithmetic, and the result is
    ///   `max(base, floor(ceiling * u))` where `u` is `jitter_fraction`
    ///   clamped into `[0, 1)` (NaN and out-of-range values map into range).
    ///   The floor at `base` keeps a gate from reading as near-zero thrash.
    ///
    /// Never panics, whatever the inputs.
    pub fn delay_for(
        &self,
        streak: u32,
        retry_after: Option<Duration>,
        jitter_fraction: f64,
    ) -> Duration {
        if let Some(provided) = retry_after {
            return provided.min(CADENCE_DELAY_CEILING);
        }

        let exponent = streak.max(1) - 1;
        // `1u32 << 31` is the largest shift that fits; anything past that is
        // far beyond every sane `max_backoff`, so saturate to the cap.
        let doubled = if exponent >= 31 {
            None
        } else {
            self.base_backoff.checked_mul(1u32 << exponent)
        };
        let ceiling = doubled
            .unwrap_or(self.max_backoff)
            .min(self.max_backoff)
            .min(CADENCE_DELAY_CEILING);

        let fraction = sanitize_fraction(jitter_fraction);
        let jittered = Duration::try_from_secs_f64(ceiling.as_secs_f64() * fraction)
            .unwrap_or(ceiling)
            .min(ceiling);
        jittered.max(self.base_backoff).min(CADENCE_DELAY_CEILING)
    }
}

impl Default for CadencePolicy {
    /// 500 ms base, 30 s cap (D-10).
    fn default() -> Self {
        Self {
            base_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
        }
    }
}

/// Map any `f64` into `[0, 1)`: NaN and negatives to zero, one and above to
/// just below one.
fn sanitize_fraction(fraction: f64) -> f64 {
    if fraction.is_nan() || fraction < 0.0 {
        0.0
    } else if fraction >= 1.0 {
        1.0 - f64::EPSILON
    } else {
        fraction
    }
}

/// Port trait for the shared pacing state (PACE-02, D-07).
///
/// # Hexagonal Architecture Context
///
/// ```text
/// ┌──────────────────────────────────────────────────────┐
/// │   CadenceLlmAdapter (paladin-llm), one per provider    │
/// │   - waits on gate(key) before delegating               │
/// │   - records a 429 / a success after the call           │
/// └───────────────────────┬────────────────────────────────┘
///                         │
///                         ▼
/// ┌──────────────────────────────────────────────────────┐
/// │              CadencePort (this module)                 │
/// └───────────────────────┬────────────────────────────────┘
///                         │
///                         ▼
/// ┌──────────────────────────────────────────────────────┐
/// │        InMemoryCadence (paladin-storage)               │
/// └──────────────────────────────────────────────────────┘
/// ```
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use std::sync::Mutex;
/// use std::time::Duration;
///
/// use async_trait::async_trait;
/// use paladin_ports::output::cadence_port::{
///     CadenceError, CadenceKey, CadencePort, FencingToken, GateReading, LockKey,
/// };
///
/// /// A toy port: a fixed one-second gate after any 429, until a success.
/// struct ToyCadence {
///     streaks: Mutex<HashMap<CadenceKey, u32>>,
/// }
///
/// impl ToyCadence {
///     fn streaks(&self) -> Result<std::sync::MutexGuard<'_, HashMap<CadenceKey, u32>>, CadenceError> {
///         self.streaks.lock().map_err(|_| CadenceError::Backend {
///             message: "state lock poisoned".to_string(),
///         })
///     }
/// }
///
/// #[async_trait]
/// impl CadencePort for ToyCadence {
///     async fn gate(&self, key: &CadenceKey) -> Result<GateReading, CadenceError> {
///         let streak = self.streaks()?.get(key).copied().unwrap_or(0);
///         let wait = if streak > 0 { Duration::from_secs(1) } else { Duration::ZERO };
///         Ok(GateReading::new(wait, streak))
///     }
///
///     async fn record_rate_limited(
///         &self,
///         key: &CadenceKey,
///         _retry_after: Option<Duration>,
///     ) -> Result<GateReading, CadenceError> {
///         let mut streaks = self.streaks()?;
///         let streak = streaks.entry(key.clone()).or_insert(0);
///         *streak += 1;
///         Ok(GateReading::new(Duration::from_secs(1), *streak))
///     }
///
///     async fn record_success(&self, key: &CadenceKey) -> Result<(), CadenceError> {
///         self.streaks()?.remove(key);
///         Ok(())
///     }
///
///     // A toy never coordinates anything: every caller "wins" the lock, which a caller
///     // must tolerate (the lock is an optimisation, never a correctness dependency).
///     async fn try_lock(
///         &self,
///         _key: &LockKey,
///         _ttl: Duration,
///     ) -> Result<Option<FencingToken>, CadenceError> {
///         Ok(Some(FencingToken::Local(1)))
///     }
///
///     async fn unlock(&self, _key: &LockKey, _token: &FencingToken) -> Result<bool, CadenceError> {
///         Ok(false)
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let cadence = ToyCadence { streaks: Mutex::new(HashMap::new()) };
///     let key = CadenceKey::new("openai", "gpt-4o");
///
///     assert!(cadence.gate(&key).await?.is_clear(), "an unknown key is clear");
///
///     cadence.record_rate_limited(&key, None).await?;
///     assert!(!cadence.gate(&key).await?.is_clear());
///
///     cadence.record_success(&key).await?;
///     assert!(cadence.gate(&key).await?.is_clear());
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait CadencePort: Send + Sync {
    /// How long to wait before sending to `key`, and the live streak.
    ///
    /// Read-only: an unknown key is clear (`wait == 0`, `streak == 0`) and no
    /// state is created for it. The boundary is closed -- at the instant the
    /// gate expires the key reads as clear.
    async fn gate(&self, key: &CadenceKey) -> Result<GateReading, CadenceError>;

    /// Record a provider 429 for `key`.
    ///
    /// * Gate active (the 429 belongs to a request sent before the gate):
    ///   the streak is unchanged and the gate is raised to at least
    ///   `now + retry_after` when that is later.
    /// * Gate clear: the streak increases by one and the gate becomes
    ///   `now + policy.delay_for(streak, retry_after, ..)`.
    ///
    /// Returns the reading after the update. `retry_after` is the provider's
    /// own minimum delay when it sent one.
    async fn record_rate_limited(
        &self,
        key: &CadenceKey,
        retry_after: Option<Duration>,
    ) -> Result<GateReading, CadenceError>;

    /// Record a success for `key`: the streak returns to zero.
    ///
    /// State exists only after a 429, so an adapter may drop the entry once
    /// its gate has also elapsed.
    async fn record_success(&self, key: &CadenceKey) -> Result<(), CadenceError>;

    /// Try to take the stampede lock `key` for `ttl` (PACE-04, D-11).
    ///
    /// Set-if-absent with expiry: exactly one caller at a time receives `Some(token)`; every
    /// other caller receives `None` without waiting. The lock is **not re-entrant** -- the
    /// current holder calling again also gets `None`. It is free again the moment `ttl` has
    /// elapsed (closed boundary: free at exactly `acquired + ttl`, still held one tick before),
    /// so a crashed holder never blocks anyone for longer than `ttl`. A `ttl` of zero is raised
    /// to one millisecond, and an excessive one is clamped at [`CADENCE_DELAY_CEILING`].
    ///
    /// Tokens for one key are strictly increasing in acquisition order. Among concurrent
    /// contenders the winner is unspecified. The token's source tells a resource how to use it:
    /// see [`FencingToken`].
    ///
    /// The lock is an optimisation, never a correctness dependency (D-13, D-29): callers must
    /// treat any `Err` as "proceed without the lock".
    ///
    /// # Errors
    ///
    /// [`CadenceError`] when the backend is unavailable.
    async fn try_lock(
        &self,
        key: &LockKey,
        ttl: Duration,
    ) -> Result<Option<FencingToken>, CadenceError>;

    /// Release the lock `key`, but only if `token` still owns it.
    ///
    /// Returns `Ok(true)` when the lock was held by `token` and has been released. Returns
    /// `Ok(false)` -- never an error -- when the key was never locked, the lock expired or was
    /// taken over by a later holder, the token was already used to unlock, or the token comes
    /// from another source than this port issues. A second `unlock` with the same token is
    /// therefore `Ok(false)`, and a stale holder can never release a newer holder's lock.
    ///
    /// # Errors
    ///
    /// [`CadenceError`] when the backend is unavailable; the lock then simply expires by its
    /// `ttl`.
    async fn unlock(&self, key: &LockKey, token: &FencingToken) -> Result<bool, CadenceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: Duration = Duration::from_millis(500);
    const MAX: Duration = Duration::from_secs(30);

    fn policy() -> CadencePolicy {
        CadencePolicy::new(BASE, MAX).expect("valid policy")
    }

    #[test]
    fn policy_first_delayless_429_gates_for_exactly_base() {
        for u in [0.0, 0.5, 0.999] {
            assert_eq!(policy().delay_for(1, None, u), BASE, "u = {u}");
        }
        // Streak zero is treated as the first escalation, never a panic.
        assert_eq!(policy().delay_for(0, None, 0.7), BASE);
    }

    #[test]
    fn policy_ceiling_doubles_per_streak_and_caps_at_max() {
        for n in 1..=10u32 {
            let ceiling = MAX.min(BASE * 2u32.pow(n - 1));
            for u in [0.0, 0.25, 0.999] {
                let d = policy().delay_for(n, None, u);
                assert!(d >= BASE, "streak {n} u {u}: {d:?} below base");
                assert!(d <= ceiling, "streak {n} u {u}: {d:?} above {ceiling:?}");
            }
        }
        assert!(policy().delay_for(64, None, 0.999) <= MAX);
        assert!(policy().delay_for(u32::MAX, None, 0.999) <= MAX);
        assert!(policy().delay_for(u32::MAX, None, 0.999) >= BASE);
    }

    #[test]
    fn policy_retry_after_is_returned_exactly_and_clamped_to_the_ceiling() {
        let seven = Duration::from_secs(7);
        for u in [0.0, 0.3, 0.999] {
            assert_eq!(policy().delay_for(3, Some(seven), u), seven);
        }
        // The provider's delay is a minimum: even below base it is not raised.
        let tiny = Duration::from_millis(10);
        assert_eq!(policy().delay_for(1, Some(tiny), 0.5), tiny);
        let huge = Duration::from_secs(48 * 60 * 60);
        assert_eq!(
            policy().delay_for(1, Some(huge), 0.5),
            CADENCE_DELAY_CEILING
        );
        assert_eq!(
            policy().delay_for(1, Some(Duration::MAX), 0.5),
            CADENCE_DELAY_CEILING
        );
    }

    #[test]
    fn policy_rejects_nan_and_out_of_range_jitter_fractions_without_panicking() {
        for u in [f64::NAN, -1.0, 1.0, 7.0, f64::INFINITY, f64::NEG_INFINITY] {
            let d = policy().delay_for(4, None, u);
            let ceiling = BASE * 8;
            assert!(d >= BASE && d <= ceiling, "u = {u}: {d:?}");
        }
    }

    #[test]
    fn policy_new_rejects_zero_base_and_base_above_max() {
        assert!(matches!(
            CadencePolicy::new(Duration::ZERO, MAX),
            Err(CadenceError::InvalidPolicy(_))
        ));
        assert!(matches!(
            CadencePolicy::new(Duration::from_secs(31), MAX),
            Err(CadenceError::InvalidPolicy(_))
        ));
        assert!(CadencePolicy::new(MAX, MAX).is_ok(), "base == max is valid");
    }

    #[test]
    fn policy_default_is_500ms_to_30s() {
        let p = CadencePolicy::default();
        assert_eq!(p.base_backoff(), BASE);
        assert_eq!(p.max_backoff(), MAX);
    }

    #[test]
    fn key_equality_is_exact_and_case_sensitive() {
        assert_eq!(CadenceKey::new("a:b", "m/x"), CadenceKey::new("a:b", "m/x"));
        assert_ne!(
            CadenceKey::new("openai", "m"),
            CadenceKey::new("OpenAI", "m")
        );
        assert_ne!(
            CadenceKey::new("openai", "m"),
            CadenceKey::new("openai", "n")
        );
    }

    #[test]
    fn fencing_tokens_from_different_sources_are_never_equal() {
        assert_ne!(FencingToken::Local(1), FencingToken::Distributed(1));
        assert_eq!(FencingToken::Local(1), FencingToken::Local(1));
        assert_eq!(FencingToken::Distributed(9), FencingToken::Distributed(9));
        assert_eq!(FencingToken::Local(3).value(), 3);
        assert_eq!(FencingToken::Distributed(4).value(), 4);
        assert!(FencingToken::Distributed(1).is_distributed());
        assert!(!FencingToken::Local(1).is_distributed());
    }

    #[test]
    fn lock_key_equality_is_exact_and_the_string_is_opaque() {
        assert_eq!(LockKey::new("a:b"), LockKey::new(String::from("a:b")));
        assert_ne!(LockKey::new("a:b"), LockKey::new("A:b"));
        assert_ne!(LockKey::new("a:b"), LockKey::new("a"));
        assert_eq!(LockKey::new("x y").as_str(), "x y");
    }

    #[test]
    fn gate_reading_reports_clear_only_for_a_zero_wait() {
        assert!(GateReading::default().is_clear());
        assert!(!GateReading::new(Duration::from_nanos(1), 0).is_clear());
        assert_eq!(GateReading::new(Duration::ZERO, 3).streak(), 3);
    }
}
