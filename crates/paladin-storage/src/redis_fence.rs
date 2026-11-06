//! The two time-to-live rules that keep the Redis fencing counter and the node-cache fence
//! marker consistent, defined once.
//!
//! The Redis Cadence numbers every stampede-lock acquisition with a monotonically increasing
//! fencing token, kept in a counter key (`fence_counter_ttl_ms` is its TTL). The Redis node cache
//! remembers the highest token that wrote an entry in a *fence marker* key and refuses any write
//! carrying a lower token (`fence_marker_ttl_ms` is its TTL). The two keys are written by
//! different adapters, behind different features, but they only work together while the marker
//! never outlives the counter that numbers it: when the counter has expired and the next lock
//! restarts at token 1, a surviving marker for token `N > 1` makes the node cache silently drop
//! every valid write until the marker expires (43-REVIEW WR-01). Keeping both rules in this one
//! crate-private module is what makes that relation checkable, so neither adapter keeps its own
//! copy of the floor, the factor or either computation.

use std::time::Duration;

/// The floor for a fencing counter's time-to-live (research Pitfall 13): one hour.
pub(crate) const MIN_FENCE_COUNTER_TTL: Duration = Duration::from_secs(60 * 60);

/// How many times longer than its lock a fencing counter must live.
#[cfg(any(feature = "redis-cadence", all(test, feature = "redis-cache")))]
pub(crate) const FENCE_TTL_FACTOR: u32 = 10;

/// A fencing counter's TTL for a lock of `lock_ttl_ms`: `max(10 * lock ttl, 1 h)`, so the
/// counter always outlives the locks it numbers (research Pitfall 13). Saturating, never
/// overflowing.
#[cfg(any(feature = "redis-cadence", all(test, feature = "redis-cache")))]
pub(crate) fn fence_counter_ttl_ms(lock_ttl_ms: i64) -> i64 {
    lock_ttl_ms
        .saturating_mul(i64::from(FENCE_TTL_FACTOR))
        .max(i64::try_from(MIN_FENCE_COUNTER_TTL.as_millis()).unwrap_or(i64::MAX))
}

/// The floor for a fence marker's time-to-live: five minutes, two and a half times the default
/// 120 s stampede-lock window (ADR-0058 "No lock renewal"), so a short entry's marker still
/// outlasts a default holder's lock.
#[cfg(feature = "redis-cache")]
pub(crate) const FENCE_MARKER_FLOOR: Duration = Duration::from_secs(5 * 60);

/// The ceiling for a fence marker's time-to-live: half the counter's one-hour floor.
///
/// This is what makes the marker provably shorter than the counter: for every lock TTL `L`,
/// `L + 30 min <= max(10 * L, 1 h)`. For `L <= 6 min` the right side is the one-hour floor and
/// `L + 30 min <= 36 min`; for `L > 6 min`, `10 * L >= L + 9 * L > L + 30 min`.
#[cfg(feature = "redis-cache")]
pub(crate) const FENCE_MARKER_CEILING: Duration =
    Duration::from_secs(MIN_FENCE_COUNTER_TTL.as_secs() / 2);

/// A fence marker's TTL for an entry valid for `entry_ttl_ms`: twice the entry, clamped to
/// `[FENCE_MARKER_FLOOR, FENCE_MARKER_CEILING]` (5 to 30 minutes). Saturating, never
/// overflowing.
///
/// # Invariant
///
/// For every stampede-lock TTL `L` the Redis Cadence accepts, `L + fence_marker_ttl_ms(e) <=
/// fence_counter_ttl_ms(L)` for every entry TTL `e`, so a marker written by a holder that writes
/// within its lock TTL expires no later than the counter that numbered the token. After the
/// counter expires the next lock restarts at token 1 and finds no marker to refuse it
/// (43-REVIEW WR-01, ADR-0058 (g)).
///
/// # Trade-off
///
/// For an entry TTL above 15 minutes the marker can expire before the entry it guards. A late
/// holder's write of the same node result is then accepted after at most 30 minutes. ADR-0058 (g)
/// already treats a late write of the deterministic result as acceptable (last-write-wins under
/// the in-process Cadence), so the cost is a duplicate write, not a wrong one.
///
/// # Residual
///
/// A holder that writes more than `max(10 * L, 1 h) - L - 30 min` after it took its lock (at
/// least 28 minutes late with the default 120 s lock) can still leave a marker that outlives the
/// counter. The next epoch's writes are then refused until that marker expires, at most 30
/// minutes later, where before this rule the wait was unbounded (ADR-0058 (i)).
#[cfg(feature = "redis-cache")]
pub(crate) fn fence_marker_ttl_ms(entry_ttl_ms: u64) -> u64 {
    let floor = millis_u64(FENCE_MARKER_FLOOR);
    let ceiling = millis_u64(FENCE_MARKER_CEILING);
    entry_ttl_ms.saturating_mul(2).clamp(floor, ceiling)
}

/// Whole milliseconds, saturating.
#[cfg(feature = "redis-cache")]
fn millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
