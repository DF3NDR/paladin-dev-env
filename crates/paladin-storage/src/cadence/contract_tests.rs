//! Shared `CadencePort` contract suite (PACE-02, D-04, D-07; mirroring
//! `node_cache::contract_tests` and `waypoint::contract_tests`).
//!
//! One `pub async fn` per contract rule, each taking `&dyn CadencePort` and the
//! [`CadencePolicy`] the port was built with, and asserting inside. Every
//! backend (`InMemoryCadence` today; `RedisCadence` and `ResilientCadence` in
//! plans 43-07 and 43-08) invokes these unchanged from its own tests, so
//! "identical rules across backends" is enforced by construction rather than
//! by convention. Named per clause (not a declarative macro) so a failure names
//! the violated rule rather than a line number.
//!
//! This module is plain (not `#[cfg(test)]`) so the unit tests inside each
//! backend and the Docker-gated Redis tier can both call it.
//!
//! ## How a backend drives the suite
//!
//! * The caller builds the port with its **jitter pinned to a constant** so a
//!   clause never depends on chance. A clause is jitter-agnostic: it asserts
//!   the floor (`wait >= base`, never near-zero thrash) and the ceiling
//!   (`wait <= min(max, base * 2^(streak - 1))`) that hold for every jitter
//!   fraction. A backend runs the suite once with jitter `0.0` (proving the
//!   floor) and once near `1.0` (proving the ceiling).
//! * The caller passes the same [`CadencePolicy`] the port holds. Pick small
//!   values (a base of tens of milliseconds, a maximum of at least eight times
//!   the base, a base of at least 8 ms) so the clauses finish quickly in real
//!   time against a live server.
//! * Waiting is `tokio::time::sleep`, so the same code runs instantly on a
//!   paused clock (in-memory) and in real time (Redis). Clauses whose name ends
//!   in `_paused` assert exact instants and may only run on a paused clock
//!   (`#[tokio::test(start_paused = true)]`; the suite itself needs no `test-util` feature);
//!   `run_all_paused` runs those, `run_all` runs the rest.
//! * Every clause uses a key namespaced by its own name, so clauses never share
//!   state and one port can serve the whole suite.
//!
//! ## Tolerance
//!
//! Against a live server the time between a `record_*` call and the reading it
//! returns is not zero, and a server may keep millisecond resolution. The
//! tolerance is therefore half the base back-off ([`slack`]): wide enough for a
//! round trip, narrow enough that a gate of zero or a gate that has been
//! lowered still fails. Exact values are asserted only by the `_paused` clause.

use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;

use paladin_ports::output::cadence_port::{
    CADENCE_DELAY_CEILING, CadenceKey, CadencePolicy, CadencePort, FencingToken, GateReading,
    LockKey,
};

/// The streak the saturation clause drives a key to.
pub const SATURATION_STREAK: u32 = 10_000;

/// How many callers record concurrently in `concurrent_429s_escalate_at_most_once`.
pub const CONCURRENT_RECORDERS: usize = 16;

/// How many callers contend for one lock in `concurrent_try_lock_has_exactly_one_winner`.
pub const CONCURRENT_LOCKERS: usize = 16;

/// The lock lifetime the real-time lock clauses use: long enough that no clause can see a
/// lock expire under it, short enough that a failed run leaves nothing lasting on a server.
const LOCK_TTL: Duration = Duration::from_secs(30);

/// Extra real-time headroom added when sleeping out a gate, so a server that
/// rounds a deadline up to the next millisecond is still observed as clear.
const SETTLE: Duration = Duration::from_millis(15);

/// The key a clause uses, namespaced by the clause's own name.
fn key(clause: &str) -> CadenceKey {
    CadenceKey::new("contract", clause)
}

/// The lock a clause uses, namespaced by the clause's own name.
fn lock_key(clause: &str) -> LockKey {
    LockKey::new(format!("contract/{clause}"))
}

/// A token of the other source with the same value, for the wrong-source unlock probes.
fn other_source(token: FencingToken) -> FencingToken {
    if token.is_distributed() {
        FencingToken::Local(token.value())
    } else {
        FencingToken::Distributed(token.value())
    }
}

/// Take `key` or fail the clause: the lock must be free when a clause expects to win it.
async fn must_lock(
    port: &dyn CadencePort,
    key: &LockKey,
    ttl: Duration,
    context: &str,
) -> FencingToken {
    port.try_lock(key, ttl)
        .await
        .unwrap_or_else(|e| panic!("{context}: try_lock failed: {e}"))
        .unwrap_or_else(|| panic!("{context}: the lock was expected to be free"))
}

/// Half the base back-off: the tolerance every non-paused comparison allows.
fn slack(policy: CadencePolicy) -> Duration {
    policy.base_backoff() / 2
}

/// `min(max_backoff, base * 2^(streak - 1))`: the largest delay-less gate for
/// `streak`, with saturating arithmetic.
fn ceiling_for(policy: CadencePolicy, streak: u32) -> Duration {
    let exponent = streak.max(1) - 1;
    let doubled = if exponent < 31 {
        policy.base_backoff().checked_mul(1u32 << exponent)
    } else {
        None
    };
    doubled
        .unwrap_or(policy.max_backoff())
        .min(policy.max_backoff())
}

/// Assert a delay-less gate respects the floor and the ceiling for `streak`.
fn assert_delayless_gate(policy: CadencePolicy, reading: GateReading, context: &str) {
    let ceiling = ceiling_for(policy, reading.streak());
    assert!(
        reading.wait() <= ceiling,
        "{context}: gate {:?} exceeds the streak-{} ceiling {ceiling:?}",
        reading.wait(),
        reading.streak()
    );
    assert!(
        reading.wait() + slack(policy) >= policy.base_backoff(),
        "{context}: gate {:?} is below the base back-off {:?} (the floor keeps a gate from \
         reading as near-zero thrash)",
        reading.wait(),
        policy.base_backoff()
    );
}

/// Sleep until a gate of `wait` has cleared.
async fn wait_out(wait: Duration) {
    tokio::time::sleep(wait + SETTLE).await;
}

/// Poll every future in `futures` to completion on the current task, so their
/// interleaving happens at the port's own await points. No new dependency: the
/// contract needs "join" and nothing else from a futures crate.
async fn join_all<'a, T>(futures: Vec<Pin<Box<dyn Future<Output = T> + Send + 'a>>>) -> Vec<T> {
    let mut slots: Vec<Option<Pin<Box<dyn Future<Output = T> + Send + 'a>>>> =
        futures.into_iter().map(Some).collect();
    let mut results: Vec<Option<T>> = slots.iter().map(|_| None).collect();
    poll_fn(|cx| {
        let mut pending = false;
        for (slot, result) in slots.iter_mut().zip(results.iter_mut()) {
            if let Some(future) = slot {
                match future.as_mut().poll(cx) {
                    Poll::Ready(value) => {
                        *result = Some(value);
                        *slot = None;
                    }
                    Poll::Pending => pending = true,
                }
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
    results.into_iter().flatten().collect()
}

/// Edge (empty): `gate` on an unknown key is clear (`wait == 0`, `streak ==
/// 0`), and `record_success` on an unknown key is `Ok` and leaves it clear
/// with no phantom streak -- the next 429 is the FIRST (streak 1). State
/// exists only after a 429 (D-04).
pub async fn unknown_key_is_clear_and_creates_no_state(
    port: &dyn CadencePort,
    _policy: CadencePolicy,
) {
    let k = key("unknown_key_is_clear_and_creates_no_state");

    let first = port.gate(&k).await.expect("gate on an unknown key");
    assert!(
        first.is_clear(),
        "an unknown key must read clear: {first:?}"
    );
    assert_eq!(first.streak(), 0);

    port.record_success(&k)
        .await
        .expect("record_success on an unknown key is Ok");
    let after_success = port.gate(&k).await.expect("gate after success");
    assert!(after_success.is_clear());
    assert_eq!(
        after_success.streak(),
        0,
        "a success must not invent a streak"
    );

    let first_429 = port
        .record_rate_limited(&k, None)
        .await
        .expect("record_rate_limited");
    assert_eq!(
        first_429.streak(),
        1,
        "the first 429 after reads and a success is streak 1"
    );
}

/// Edge (boundary): the first delay-less 429 gates for exactly the base
/// back-off ceiling (streak 1), so the floor and the ceiling coincide: the gate
/// is the base back-off whatever the jitter, and never zero.
pub async fn first_delayless_429_gates_for_base(port: &dyn CadencePort, policy: CadencePolicy) {
    let k = key("first_delayless_429_gates_for_base");
    let reading = port
        .record_rate_limited(&k, None)
        .await
        .expect("record_rate_limited");
    assert_eq!(reading.streak(), 1);
    assert!(!reading.is_clear(), "a fresh 429 must open a gate");
    assert!(
        reading.wait() <= policy.base_backoff(),
        "streak-1 gate {:?} exceeds the base back-off {:?}",
        reading.wait(),
        policy.base_backoff()
    );
    assert!(
        reading.wait() + slack(policy) >= policy.base_backoff(),
        "streak-1 gate {:?} is not the base back-off {:?}",
        reading.wait(),
        policy.base_backoff()
    );

    let read = port.gate(&k).await.expect("gate");
    assert_eq!(read.streak(), 1);
    assert!(!read.is_clear(), "gate reads must see the gate just opened");
}

/// Edge (adjacency, paused clock only): one tick (1 ms) before `not_before` the
/// gate reads a non-zero wait; at exactly `not_before` it reads zero (the
/// boundary is closed); a 429 recorded at exactly `not_before` escalates the
/// streak, because the gate is already clear. Also asserts the exact values a
/// real-time run cannot: a streak-1 gate is exactly the base back-off and an
/// explicit provider delay is exactly that delay.
pub async fn gate_boundary_is_closed_at_not_before_paused(
    port: &dyn CadencePort,
    policy: CadencePolicy,
) {
    let tick = Duration::from_millis(1);
    let base = policy.base_backoff();
    assert!(
        base > tick,
        "the paused clause needs a base back-off above 1 ms"
    );

    let k = key("gate_boundary_is_closed_at_not_before_paused");
    let opened = port
        .record_rate_limited(&k, None)
        .await
        .expect("record_rate_limited");
    assert_eq!(opened.streak(), 1);
    assert_eq!(
        opened.wait(),
        base,
        "a streak-1 delay-less gate is exactly the base"
    );

    // `sleep`, not `advance`: `advance` needs tokio's `test-util` feature, which a plain (non-test)
    // module cannot assume. On a paused clock an idle `sleep` jumps the clock to exactly its
    // deadline, and every duration here is a whole number of milliseconds.
    tokio::time::sleep(base - tick).await;
    let before = port.gate(&k).await.expect("gate one tick early");
    assert_eq!(
        before.wait(),
        tick,
        "one tick before not_before the gate is not clear"
    );
    assert!(!before.is_clear());

    tokio::time::sleep(tick).await;
    let at = port.gate(&k).await.expect("gate at not_before");
    assert_eq!(
        at.wait(),
        Duration::ZERO,
        "closed boundary: clear at exactly not_before"
    );
    assert_eq!(at.streak(), 1, "the streak survives until a success");

    let escalated = port
        .record_rate_limited(&k, None)
        .await
        .expect("record at not_before");
    assert_eq!(
        escalated.streak(),
        2,
        "a 429 at exactly not_before finds the gate clear, so it escalates"
    );
    assert_delayless_gate(policy, escalated, "escalation at not_before");

    // Precision: an explicit provider delay is the exact gate, never jittered.
    let explicit = Duration::from_millis(7);
    let e = key("gate_boundary_is_closed_at_not_before_paused/explicit");
    let reading = port
        .record_rate_limited(&e, Some(explicit))
        .await
        .expect("explicit delay");
    assert_eq!(
        reading.wait(),
        explicit,
        "an explicit delay is exact, even below the base"
    );
}

/// Each delay-less 429 recorded after the gate cleared escalates the streak by
/// one and its gate stays within `[base, min(max, base * 2^(streak - 1))]`.
pub async fn escalation_doubles_the_ceiling_after_the_gate_clears(
    port: &dyn CadencePort,
    policy: CadencePolicy,
) {
    let k = key("escalation_doubles_the_ceiling_after_the_gate_clears");
    for streak in 1..=4u32 {
        let reading = port
            .record_rate_limited(&k, None)
            .await
            .expect("record_rate_limited");
        assert_eq!(
            reading.streak(),
            streak,
            "each post-clear 429 escalates by one"
        );
        assert_delayless_gate(policy, reading, "escalation");

        wait_out(reading.wait()).await;
        let after = port.gate(&k).await.expect("gate");
        assert!(
            after.is_clear(),
            "gate {:?} must have cleared",
            after.wait()
        );
        assert_eq!(
            after.streak(),
            streak,
            "the streak survives until a success"
        );
    }
}

/// Edge (idempotency, in-flight): a 429 recorded while the gate is active
/// answers a request sent before the gate, so it does NOT escalate. Recording
/// the same 429 twice (or many times) while its gate is active leaves the
/// streak unchanged.
pub async fn in_flight_429_does_not_escalate(port: &dyn CadencePort, policy: CadencePolicy) {
    let k = key("in_flight_429_does_not_escalate");
    let first = port.record_rate_limited(&k, None).await.expect("first 429");
    assert_eq!(first.streak(), 1);

    for attempt in 0..5 {
        let again = port
            .record_rate_limited(&k, None)
            .await
            .expect("in-flight 429");
        assert_eq!(
            again.streak(),
            1,
            "in-flight 429 #{attempt} must not escalate the streak"
        );
        assert!(
            again.wait() <= policy.base_backoff(),
            "an in-flight 429 must not raise the gate: {:?}",
            again.wait()
        );
    }
    assert_eq!(port.gate(&k).await.expect("gate").streak(), 1);
}

/// A provider delay can only RAISE an active gate: a smaller explicit delay
/// never lowers it, a larger one raises it, and neither escalates the streak.
pub async fn explicit_retry_after_raises_but_never_lowers_an_active_gate(
    port: &dyn CadencePort,
    policy: CadencePolicy,
) {
    let base = policy.base_backoff();
    let k = key("explicit_retry_after_raises_but_never_lowers_an_active_gate");

    let opened = port
        .record_rate_limited(&k, Some(base * 4))
        .await
        .expect("open with 4 x base");
    assert_eq!(opened.streak(), 1);

    let lower = port
        .record_rate_limited(&k, Some(base))
        .await
        .expect("a smaller delay");
    assert_eq!(lower.streak(), 1, "an in-flight 429 never escalates");
    assert!(
        lower.wait() + slack(policy) >= base * 4,
        "a smaller delay must not lower an active gate: {:?}",
        lower.wait()
    );

    let none = port
        .record_rate_limited(&k, None)
        .await
        .expect("a delay-less 429");
    assert!(
        none.wait() + slack(policy) >= base * 4,
        "a delay-less in-flight 429 must not lower the gate: {:?}",
        none.wait()
    );

    let higher = port
        .record_rate_limited(&k, Some(base * 8))
        .await
        .expect("a larger delay");
    assert_eq!(higher.streak(), 1, "raising the gate does not escalate");
    assert!(
        higher.wait() + slack(policy) >= base * 8,
        "a larger delay must raise the gate: {:?}",
        higher.wait()
    );
    assert!(higher.wait() <= base * 8);
}

/// Edge (precision): the provider's delay is the exact gate on a clear key --
/// it is never jittered down and never raised to the base back-off -- and a
/// hostile delay is clamped to `CADENCE_DELAY_CEILING` without panicking.
pub async fn explicit_retry_after_is_the_exact_gate(port: &dyn CadencePort, policy: CadencePolicy) {
    let base = policy.base_backoff();

    let three = key("explicit_retry_after_is_the_exact_gate/three");
    let reading = port
        .record_rate_limited(&three, Some(base * 3))
        .await
        .expect("explicit delay");
    assert_eq!(reading.streak(), 1);
    assert!(
        reading.wait() <= base * 3,
        "gate {:?} exceeds the delay",
        reading.wait()
    );
    assert!(
        reading.wait() + slack(policy) >= base * 3,
        "gate {:?} is below the provider's delay",
        reading.wait()
    );

    let short = key("explicit_retry_after_is_the_exact_gate/short");
    let tiny = base / 4;
    let short_reading = port
        .record_rate_limited(&short, Some(tiny))
        .await
        .expect("a delay below the base");
    assert!(
        short_reading.wait() <= tiny,
        "a provider delay below the base is not raised to the base: {:?}",
        short_reading.wait()
    );

    let hostile = key("explicit_retry_after_is_the_exact_gate/hostile");
    let clamped = port
        .record_rate_limited(&hostile, Some(Duration::MAX))
        .await
        .expect("a hostile delay must not panic or error");
    assert!(
        clamped.wait() <= CADENCE_DELAY_CEILING,
        "gate {:?} exceeds the 24 h ceiling",
        clamped.wait()
    );
    assert!(!clamped.is_clear());
}

/// `record_success` returns the streak to zero, a second `record_success` is a
/// no-op, and the next 429 starts again from the base back-off.
pub async fn success_resets_the_streak_and_is_idempotent(
    port: &dyn CadencePort,
    policy: CadencePolicy,
) {
    let k = key("success_resets_the_streak_and_is_idempotent");
    let one = port.record_rate_limited(&k, None).await.expect("first");
    wait_out(one.wait()).await;
    let two = port.record_rate_limited(&k, None).await.expect("second");
    assert_eq!(two.streak(), 2);
    wait_out(two.wait()).await;

    port.record_success(&k).await.expect("first success");
    let after_first = port.gate(&k).await.expect("gate");
    assert_eq!(after_first.streak(), 0, "a success resets the streak");

    port.record_success(&k).await.expect("second success");
    let after_second = port.gate(&k).await.expect("gate");
    assert_eq!(after_second.streak(), 0, "a repeated success is a no-op");
    assert!(
        after_second.wait() <= after_first.wait(),
        "a repeated success must not move the gate"
    );

    let fresh = port.record_rate_limited(&k, None).await.expect("fresh 429");
    assert_eq!(
        fresh.streak(),
        1,
        "after a success the next 429 starts over"
    );
    assert!(
        fresh.wait() <= policy.base_backoff(),
        "and gates for the base back-off again: {:?}",
        fresh.wait()
    );
}

/// Edge (boundary): the streak counts to [`SATURATION_STREAK`] without
/// overflow or panic and the gate never exceeds `max_backoff`, however long the
/// streak. Each record uses a zero provider delay, so every one finds the gate
/// already clear and escalates (this drives the streak without sleeping).
pub async fn streak_saturates_without_overflow(port: &dyn CadencePort, policy: CadencePolicy) {
    let k = key("streak_saturates_without_overflow");
    let mut last = GateReading::default();
    for _ in 0..SATURATION_STREAK {
        last = port
            .record_rate_limited(&k, Some(Duration::ZERO))
            .await
            .expect("record with a zero delay");
    }
    assert_eq!(last.streak(), SATURATION_STREAK);

    let long = port
        .record_rate_limited(&k, None)
        .await
        .expect("delay-less record at a very long streak");
    assert_eq!(long.streak(), SATURATION_STREAK + 1);
    assert!(
        long.wait() <= policy.max_backoff(),
        "gate {:?} exceeds max_backoff {:?} at streak {}",
        long.wait(),
        policy.max_backoff(),
        long.streak()
    );
    assert!(
        long.wait() + slack(policy) >= policy.base_backoff(),
        "the floor holds at a very long streak: {:?}",
        long.wait()
    );
}

/// Edge (empty / identity): keys are compared by exact, case-sensitive bytes.
/// `gpt-4` and `GPT-4` are independent, an empty model string is a valid and
/// distinct key, and neither the provider/model boundary nor a Unicode
/// normalization form is folded.
pub async fn keys_are_exact_byte_equal(port: &dyn CadencePort, _policy: CadencePolicy) {
    let opened = CadenceKey::new("contract-keys", "gpt-4");
    port.record_rate_limited(&opened, None)
        .await
        .expect("record");
    assert!(!port.gate(&opened).await.expect("gate").is_clear());

    let others = [
        CadenceKey::new("contract-keys", "GPT-4"),
        CadenceKey::new("Contract-Keys", "gpt-4"),
        CadenceKey::new("contract-keys", "gpt-4 "),
        CadenceKey::new("contract-keys", ""),
        CadenceKey::new("contract-keys", "gpt-"),
        CadenceKey::new("contract-keys:gpt", "-4"),
        CadenceKey::new("contract", "keys:gpt-4"),
    ];
    for other in &others {
        let reading = port.gate(other).await.expect("gate");
        assert!(
            reading.is_clear() && reading.streak() == 0,
            "{other:?} must be independent of {opened:?}"
        );
    }

    // An empty model is a valid key in its own right.
    let empty = CadenceKey::new("contract-keys", "");
    let reading = port
        .record_rate_limited(&empty, None)
        .await
        .expect("an empty model string is a valid key");
    assert_eq!(reading.streak(), 1);
    assert!(!port.gate(&empty).await.expect("gate").is_clear());
    assert_eq!(
        port.gate(&opened).await.expect("gate").streak(),
        1,
        "the empty-model key did not touch the other"
    );

    // Composed vs decomposed 'e' with acute accent are different bytes.
    let composed = CadenceKey::new("contract-keys", "mod\u{e9}l");
    let decomposed = CadenceKey::new("contract-keys", "mode\u{301}l");
    port.record_rate_limited(&composed, None)
        .await
        .expect("record");
    assert!(
        port.gate(&decomposed).await.expect("gate").is_clear(),
        "no Unicode normalization: the keys differ at byte level"
    );
}

/// Edge (concurrency): [`CONCURRENT_RECORDERS`] callers recording a delay-less
/// 429 on one clear key at once leave the streak at exactly 1 -- the in-flight
/// rule applies atomically, so a burst of simultaneous refusals is one
/// escalation, not sixteen.
pub async fn concurrent_429s_escalate_at_most_once(port: &dyn CadencePort, _policy: CadencePolicy) {
    let k = key("concurrent_429s_escalate_at_most_once");
    let futures: Vec<Pin<Box<dyn Future<Output = GateReading> + Send + '_>>> = (0
        ..CONCURRENT_RECORDERS)
        .map(|_| {
            let k = k.clone();
            Box::pin(async move {
                port.record_rate_limited(&k, None)
                    .await
                    .expect("concurrent record")
            }) as Pin<Box<dyn Future<Output = GateReading> + Send + '_>>
        })
        .collect();

    let readings = join_all(futures).await;
    assert_eq!(readings.len(), CONCURRENT_RECORDERS);
    for reading in &readings {
        assert_eq!(
            reading.streak(),
            1,
            "no concurrent record may escalate twice"
        );
    }
    assert_eq!(port.gate(&k).await.expect("gate").streak(), 1);
}

/// Edge (empty + exclusivity): `try_lock` on a fresh key returns `Some`; while that lock is
/// held every other `try_lock` on the key returns `None`, whatever the contender; a different
/// key is independent; the winner can release it.
pub async fn try_lock_is_exclusive(port: &dyn CadencePort, _policy: CadencePolicy) {
    let k = lock_key("try_lock_is_exclusive");
    let other = lock_key("try_lock_is_exclusive/other");

    let token = must_lock(port, &k, LOCK_TTL, "fresh key").await;
    for _ in 0..3 {
        assert_eq!(
            port.try_lock(&k, LOCK_TTL).await.expect("contender"),
            None,
            "a held lock must refuse every other contender"
        );
    }
    let independent = must_lock(port, &other, LOCK_TTL, "an unrelated key").await;
    assert!(
        port.unlock(&k, &token).await.expect("unlock"),
        "the winner releases its lock"
    );
    assert!(port.unlock(&other, &independent).await.expect("unlock"));
}

/// Edge (ordering): tokens for one key are strictly increasing in acquisition order --
/// acquire, unlock, acquire again yields a larger token -- and one source throughout.
pub async fn tokens_increase_per_key(port: &dyn CadencePort, _policy: CadencePolicy) {
    let k = lock_key("tokens_increase_per_key");
    let mut last: Option<FencingToken> = None;
    for round in 0..4 {
        let token = must_lock(port, &k, LOCK_TTL, &format!("round {round}")).await;
        if let Some(previous) = last {
            assert_eq!(
                token.is_distributed(),
                previous.is_distributed(),
                "a port issues tokens from one source"
            );
            assert!(
                token.value() > previous.value(),
                "round {round}: token {token:?} is not above the previous {previous:?}"
            );
        }
        assert!(port.unlock(&k, &token).await.expect("unlock"));
        last = Some(token);
    }
}

/// Edge (ownership): only the token that owns the lock releases it. A token from the other
/// source, and a same-source token with a different value, both return `Ok(false)` and leave
/// the lock held; the owner's token then releases it.
pub async fn unlock_only_by_the_owner(port: &dyn CadencePort, _policy: CadencePolicy) {
    let k = lock_key("unlock_only_by_the_owner");
    let token = must_lock(port, &k, LOCK_TTL, "owner").await;

    let foreign = other_source(token);
    assert!(
        !port.unlock(&k, &foreign).await.expect("wrong source"),
        "a token from the other source never releases the lock"
    );
    let same_source_other_value = match token {
        FencingToken::Distributed(v) => FencingToken::Distributed(v + 1000),
        _ => FencingToken::Local(token.value() + 1000),
    };
    assert!(
        !port
            .unlock(&k, &same_source_other_value)
            .await
            .expect("wrong value"),
        "a token with another value never releases the lock"
    );
    assert_eq!(
        port.try_lock(&k, LOCK_TTL).await.expect("still held"),
        None,
        "the failed unlocks left the lock held"
    );
    assert!(port.unlock(&k, &token).await.expect("owner unlock"));
}

/// Edge (idempotency): a second `unlock` with the same token returns `Ok(false)`, and it must
/// not release a later holder's lock.
pub async fn unlock_is_idempotent(port: &dyn CadencePort, _policy: CadencePolicy) {
    let k = lock_key("unlock_is_idempotent");
    let first = must_lock(port, &k, LOCK_TTL, "first holder").await;
    assert!(port.unlock(&k, &first).await.expect("unlock"));
    assert!(
        !port.unlock(&k, &first).await.expect("second unlock"),
        "a used token releases nothing"
    );

    let second = must_lock(port, &k, LOCK_TTL, "second holder").await;
    assert!(
        !port.unlock(&k, &first).await.expect("stale unlock"),
        "the previous holder's token must not release the new holder's lock"
    );
    assert_eq!(port.try_lock(&k, LOCK_TTL).await.expect("held"), None);
    assert!(port.unlock(&k, &second).await.expect("unlock"));
}

/// Edge (empty): `unlock` of a key never locked is `Ok(false)` for either token source.
pub async fn unlock_of_a_never_locked_key_is_false(port: &dyn CadencePort, _policy: CadencePolicy) {
    let k = lock_key("unlock_of_a_never_locked_key_is_false");
    for token in [FencingToken::Local(1), FencingToken::Distributed(1)] {
        assert!(
            !port.unlock(&k, &token).await.expect("unlock"),
            "{token:?} on a key never locked"
        );
    }
    // And it left nothing behind: the key is still free.
    let token = must_lock(port, &k, LOCK_TTL, "after the probes").await;
    assert!(port.unlock(&k, &token).await.expect("unlock"));
}

/// Edge (idempotency): the current holder calling `try_lock` again gets `None` -- the lock is
/// not re-entrant -- and the original token still owns it.
pub async fn lock_is_not_reentrant(port: &dyn CadencePort, _policy: CadencePolicy) {
    let k = lock_key("lock_is_not_reentrant");
    let token = must_lock(port, &k, LOCK_TTL, "holder").await;
    assert_eq!(
        port.try_lock(&k, LOCK_TTL).await.expect("again"),
        None,
        "the holder is not given the lock a second time"
    );
    assert!(
        port.unlock(&k, &token).await.expect("unlock"),
        "the original token is unaffected by the refused re-entry"
    );
}

/// Edge (adjacency, paused clock only): one tick (1 ms) before the TTL elapses the lock is
/// still held; at exactly the TTL it is free (closed boundary) and the next holder's token is
/// larger. The expired holder's late `unlock` returns `Ok(false)` and does not release the new
/// holder's lock.
pub async fn lock_expires_at_its_ttl_paused(port: &dyn CadencePort, _policy: CadencePolicy) {
    let tick = Duration::from_millis(1);
    let ttl = Duration::from_millis(200);
    let k = lock_key("lock_expires_at_its_ttl_paused");

    let first = must_lock(port, &k, ttl, "first holder").await;
    tokio::time::sleep(ttl - tick).await;
    assert_eq!(
        port.try_lock(&k, ttl).await.expect("one tick early"),
        None,
        "one tick before the TTL the lock is still held"
    );

    tokio::time::sleep(tick).await;
    let second = must_lock(port, &k, ttl, "exactly at the TTL").await;
    assert_eq!(first.is_distributed(), second.is_distributed());
    assert!(
        second.value() > first.value(),
        "the next holder's token {second:?} must exceed the expired one {first:?}"
    );

    assert!(
        !port.unlock(&k, &first).await.expect("late unlock"),
        "an expired holder releases nothing"
    );
    assert_eq!(
        port.try_lock(&k, ttl).await.expect("still held"),
        None,
        "the new holder's lock survived the stale unlock"
    );
    assert!(port.unlock(&k, &second).await.expect("unlock"));
}

/// Edge (concurrency): [`CONCURRENT_LOCKERS`] callers racing for one key at once yield exactly
/// one `Some`. After the winner unlocks, the next caller acquires with a larger token.
pub async fn concurrent_try_lock_has_exactly_one_winner(
    port: &dyn CadencePort,
    _policy: CadencePolicy,
) {
    let k = lock_key("concurrent_try_lock_has_exactly_one_winner");
    let futures: Vec<Pin<Box<dyn Future<Output = Option<FencingToken>> + Send + '_>>> = (0
        ..CONCURRENT_LOCKERS)
        .map(|_| {
            let k = k.clone();
            Box::pin(async move { port.try_lock(&k, LOCK_TTL).await.expect("racer") })
                as Pin<Box<dyn Future<Output = Option<FencingToken>> + Send + '_>>
        })
        .collect();

    let outcomes = join_all(futures).await;
    assert_eq!(outcomes.len(), CONCURRENT_LOCKERS);
    let winners: Vec<FencingToken> = outcomes.into_iter().flatten().collect();
    assert_eq!(
        winners.len(),
        1,
        "exactly one contender may win the lock, got {winners:?}"
    );
    let winner = winners[0];

    assert!(port.unlock(&k, &winner).await.expect("winner unlocks"));
    let next = must_lock(port, &k, LOCK_TTL, "after the winner unlocked").await;
    assert!(
        next.value() > winner.value(),
        "the next acquisition's token {next:?} must exceed the winner's {winner:?}"
    );
    assert!(port.unlock(&k, &next).await.expect("unlock"));
}

/// Every lock clause that is valid on any clock, against a single port. The paused TTL clause
/// ([`lock_expires_at_its_ttl_paused`]) is separate: only a paused-clock adapter may run it.
pub async fn run_all_locks(port: &dyn CadencePort, policy: CadencePolicy) {
    try_lock_is_exclusive(port, policy).await;
    tokens_increase_per_key(port, policy).await;
    unlock_only_by_the_owner(port, policy).await;
    unlock_is_idempotent(port, policy).await;
    unlock_of_a_never_locked_key_is_false(port, policy).await;
    lock_is_not_reentrant(port, policy).await;
    concurrent_try_lock_has_exactly_one_winner(port, policy).await;
}

/// Smoke aggregate: runs every clause that is valid on any clock against a
/// single port. Backends should still invoke each clause from its own named
/// test so a failure names the violated rule; this is a single-call
/// convenience (mirroring `node_cache::contract_tests::run_all`).
pub async fn run_all(port: &dyn CadencePort, policy: CadencePolicy) {
    unknown_key_is_clear_and_creates_no_state(port, policy).await;
    first_delayless_429_gates_for_base(port, policy).await;
    escalation_doubles_the_ceiling_after_the_gate_clears(port, policy).await;
    in_flight_429_does_not_escalate(port, policy).await;
    explicit_retry_after_raises_but_never_lowers_an_active_gate(port, policy).await;
    explicit_retry_after_is_the_exact_gate(port, policy).await;
    success_resets_the_streak_and_is_idempotent(port, policy).await;
    streak_saturates_without_overflow(port, policy).await;
    keys_are_exact_byte_equal(port, policy).await;
    concurrent_429s_escalate_at_most_once(port, policy).await;
}

/// The clauses that assert exact instants and so need a paused tokio clock
/// (`#[tokio::test(start_paused = true)]`). A real-time backend does not call
/// this.
pub async fn run_all_paused(port: &dyn CadencePort, policy: CadencePolicy) {
    gate_boundary_is_closed_at_not_before_paused(port, policy).await;
}
