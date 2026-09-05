//! Shared `NodeCachePort` contract suite (D-27, mirroring `waypoint::contract_tests`'s
//! D-09 precedent).
//!
//! One generic async function per contract clause, each taking `&dyn
//! NodeCachePort` and asserting inside. Every backend (`InMemoryNodeCache`
//! today; `RedisNodeCache` behind the `redis-cache` feature, Task 2)
//! invokes these unchanged from its own `#[tokio::test]`s, so "identical
//! suite across backends" is enforced by construction rather than by
//! convention. Named per-clause (not a declarative macro) so a failure
//! names the violated contract clause rather than a line number.
//!
//! This module is plain (not `#[cfg(test)]`) so both unit tests inside each
//! backend crate and the Docker-gated Redis integration tier can call it.
//!
//! Four cases (`entry_at_exactly_expires_at_is_expired`,
//! `concurrent_put_of_the_same_key_leaves_exactly_one_entry`,
//! `an_empty_delta_round_trips_as_a_hit_not_a_miss`,
//! `keys_differing_only_by_graph_prefix_do_not_collide`) are FT-06 edge
//! assumptions the deterministic edge probe left unclassified this phase --
//! resolved here by covering each explicitly rather than leaving it
//! implicit. Every timing-sensitive case uses an explicitly injected
//! zero/expired TTL (never a real wall-clock sleep), per this plan's own
//! `<action>` guidance: `Duration::ZERO` makes `expires_at == stored_at`,
//! and any subsequent `Utc::now()` read (which cannot move backward within
//! one test) is provably `>= expires_at`, deterministically proving the
//! closed boundary without a paused clock (which only affects
//! `tokio::time::Instant`, not the wall-clock `DateTime<Utc>` a
//! `CachedDelta` is stamped with).

use std::time::Duration;

use paladin_core::platform::container::battlefield::{FieldName, StateDelta};
use paladin_ports::output::node_cache_port::{NodeCacheKey, NodeCachePort};

/// Build a `StateDelta` carrying a single `count` field set to `value`, for
/// tests that need a delta whose stored value can be distinguished from
/// another sample's.
pub fn sample_delta(value: i64) -> StateDelta {
    let mut delta = StateDelta::new();
    delta
        .set(FieldName::new("count").unwrap(), value)
        .expect("count is a valid field name and i64 serializes");
    delta
}

/// A generous TTL used by every contract case that is not itself testing
/// expiry, so an entry those cases write is never a candidate to expire
/// mid-test.
fn live_ttl() -> Duration {
    Duration::from_secs(3600)
}

/// Contract case 1: a `put` followed by a `get` of the same key returns the
/// stored `CachedDelta` with its `delta` equal to what was stored.
pub async fn put_then_get_returns_the_stored_delta(port: &dyn NodeCachePort) {
    let key = NodeCacheKey::new("contract-put-then-get");
    let delta = sample_delta(42);

    port.put(&key, &delta, live_ttl()).await.unwrap();
    let loaded = port.get(&key).await.unwrap().expect("expected a hit");

    assert_eq!(loaded.delta, delta);
}

/// Contract case 2: `get` of a never-written key returns `None`, not an
/// error.
pub async fn get_of_an_absent_key_is_a_miss(port: &dyn NodeCachePort) {
    let key = NodeCacheKey::new("contract-absent-key");
    assert_eq!(port.get(&key).await.unwrap(), None);
}

/// Contract case 3: an entry written with a short TTL is a miss after the
/// TTL elapses. A `Duration::ZERO` TTL "elapses" the instant it is written
/// (`expires_at == stored_at`), so the very next `get` -- at any strictly
/// later wall-clock instant -- is deterministically a miss. A sibling entry
/// written with a generous TTL in the SAME test proves the cache does not
/// simply reject every write: only the short-TTL entry expires.
pub async fn entry_expires_after_its_ttl(port: &dyn NodeCachePort) {
    let short_lived = NodeCacheKey::new("contract-short-lived");
    let long_lived = NodeCacheKey::new("contract-long-lived");

    port.put(&short_lived, &sample_delta(1), Duration::ZERO)
        .await
        .unwrap();
    port.put(&long_lived, &sample_delta(2), live_ttl())
        .await
        .unwrap();

    assert_eq!(
        port.get(&short_lived).await.unwrap(),
        None,
        "a zero-TTL entry must already be expired"
    );
    assert!(
        port.get(&long_lived).await.unwrap().is_some(),
        "a generously-TTL'd sibling entry must still be a hit"
    );
}

/// Contract case 4 (FT-06 edge assumption): the TTL boundary is CLOSED at
/// `expires_at` -- an entry read at exactly its `expires_at` instant is a
/// miss, not a hit. A `Duration::ZERO` TTL sets `expires_at` to the exact
/// `stored_at` instant; since wall-clock time strictly advances between the
/// `put` and the following `get` within one async test, the `get` executes
/// at (or after) that exact boundary and must observe a miss -- proving the
/// boundary is `now >= expires_at`, never `now > expires_at`.
pub async fn entry_at_exactly_expires_at_is_expired(port: &dyn NodeCachePort) {
    let key = NodeCacheKey::new("contract-boundary");
    port.put(&key, &sample_delta(1), Duration::ZERO)
        .await
        .unwrap();

    assert_eq!(
        port.get(&key).await.unwrap(),
        None,
        "a read at (or after) the exact expires_at instant must be a miss"
    );
}

/// Contract case 5: `invalidate("graphA:")` removes every `graphA:`-prefixed
/// key and leaves `graphB:` keys intact.
pub async fn invalidate_prefix_removes_only_matching_keys(port: &dyn NodeCachePort) {
    let a1 = NodeCacheKey::new("graphA:node1:x");
    let a2 = NodeCacheKey::new("graphA:node2:y");
    let b1 = NodeCacheKey::new("graphB:node1:z");

    port.put(&a1, &sample_delta(1), live_ttl()).await.unwrap();
    port.put(&a2, &sample_delta(2), live_ttl()).await.unwrap();
    port.put(&b1, &sample_delta(3), live_ttl()).await.unwrap();

    let removed = port.invalidate("graphA:").await.unwrap();
    assert_eq!(removed, 2, "exactly the two graphA: keys must be removed");

    assert_eq!(port.get(&a1).await.unwrap(), None);
    assert_eq!(port.get(&a2).await.unwrap(), None);
    assert!(
        port.get(&b1).await.unwrap().is_some(),
        "a graphB: key must survive a graphA: invalidation"
    );
}

/// Contract case 6: a second `put` on the same key wins and resets
/// `stored_at`/`expires_at`.
pub async fn overwriting_a_key_replaces_the_previous_entry(port: &dyn NodeCachePort) {
    let key = NodeCacheKey::new("contract-overwrite");

    port.put(&key, &sample_delta(1), live_ttl()).await.unwrap();
    let first = port.get(&key).await.unwrap().expect("first put must hit");

    port.put(&key, &sample_delta(2), live_ttl()).await.unwrap();
    let second = port.get(&key).await.unwrap().expect("second put must hit");

    assert_eq!(second.delta, sample_delta(2));
    assert_ne!(second.delta, first.delta);
    assert!(
        second.stored_at >= first.stored_at,
        "the second put's stored_at must not precede the first's"
    );
}

/// Contract case 7 (FT-06 edge assumption): two concurrent `put` calls for
/// one key leave one readable entry equal to one of the two written deltas,
/// never a merged or partial value.
pub async fn concurrent_put_of_the_same_key_leaves_exactly_one_entry(port: &dyn NodeCachePort) {
    let key = NodeCacheKey::new("contract-concurrent");
    let delta_a = sample_delta(101);
    let delta_b = sample_delta(202);

    let (result_a, result_b) = tokio::join!(
        port.put(&key, &delta_a, live_ttl()),
        port.put(&key, &delta_b, live_ttl()),
    );
    result_a.unwrap();
    result_b.unwrap();

    let loaded = port.get(&key).await.unwrap().expect("expected a hit");
    assert!(
        loaded.delta == delta_a || loaded.delta == delta_b,
        "the surviving entry must equal exactly one of the two concurrently-written deltas, \
         got {:?}",
        loaded.delta
    );
}

/// Contract case 8 (FT-06 edge assumption): a `StateDelta` with no fields
/// stores and reads back as a hit whose `delta` is empty -- an empty delta
/// is a legitimate cached value, never conflated with absence.
pub async fn an_empty_delta_round_trips_as_a_hit_not_a_miss(port: &dyn NodeCachePort) {
    let key = NodeCacheKey::new("contract-empty-delta");
    let empty = StateDelta::new();
    assert!(empty.values.is_empty());

    port.put(&key, &empty, live_ttl()).await.unwrap();
    let loaded = port
        .get(&key)
        .await
        .unwrap()
        .expect("an empty delta must still be a hit, not a miss");

    assert!(loaded.delta.values.is_empty());
    assert_eq!(loaded.delta, empty);
}

/// Contract case 9 (FT-06 edge assumption): two keys identical except for
/// their graph-fingerprint component read back independently.
pub async fn keys_differing_only_by_graph_prefix_do_not_collide(port: &dyn NodeCachePort) {
    let key_graph_a = NodeCacheKey::new("graphA:node1:sameinput");
    let key_graph_b = NodeCacheKey::new("graphB:node1:sameinput");

    port.put(&key_graph_a, &sample_delta(1), live_ttl())
        .await
        .unwrap();
    port.put(&key_graph_b, &sample_delta(2), live_ttl())
        .await
        .unwrap();

    let loaded_a = port
        .get(&key_graph_a)
        .await
        .unwrap()
        .expect("graphA key must be a hit");
    let loaded_b = port
        .get(&key_graph_b)
        .await
        .unwrap()
        .expect("graphB key must be a hit");

    assert_eq!(loaded_a.delta, sample_delta(1));
    assert_eq!(loaded_b.delta, sample_delta(2));
    assert_ne!(loaded_a.delta, loaded_b.delta);
}

/// Smoke aggregate: runs every contract function in sequence against a
/// single fresh port (mirroring `waypoint::contract_tests::run_all`).
/// Individual backends should still invoke each function from its own named
/// `#[tokio::test]` (so a failure names the violated clause) -- this exists
/// as a single-call convenience, not a replacement for the per-clause tests.
pub async fn run_all(port: &dyn NodeCachePort) {
    put_then_get_returns_the_stored_delta(port).await;
    get_of_an_absent_key_is_a_miss(port).await;
    entry_expires_after_its_ttl(port).await;
    entry_at_exactly_expires_at_is_expired(port).await;
    invalidate_prefix_removes_only_matching_keys(port).await;
    overwriting_a_key_replaces_the_previous_entry(port).await;
    concurrent_put_of_the_same_key_leaves_exactly_one_entry(port).await;
    an_empty_delta_round_trips_as_a_hit_not_a_miss(port).await;
    keys_differing_only_by_graph_prefix_do_not_collide(port).await;
}
