/*
In-Memory Node Cache

An `Arc<tokio::sync::RwLock<HashMap<NodeCacheKey, CachedDelta>>>`-backed
implementation of `NodeCachePort` (D-27), ungated (no feature flag,
mirroring `waypoint::in_memory`'s D-01 precedent): used for tests, local
development, and any deployment that has not opted into a durable cache
backend.

Expiry is lazy-on-read plus swept-on-write: a `get` never removes an expired
entry itself (an expired entry is simply treated as absent, at no extra
lock-upgrade cost), while every `put` opportunistically sweeps every expired
entry out of the map before inserting -- so a cache that is never written to
again does not grow unbounded with dead entries, without needing a
background task.
*/

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::node_cache::CachedDelta;
use paladin_ports::output::node_cache_port::{NodeCacheError, NodeCacheKey, NodeCachePort};

/// In-memory `NodeCachePort` implementation.
///
/// Cloning an `InMemoryNodeCache` is cheap and shares the same underlying
/// store (the inner `Arc` is cloned), matching `InMemoryWaypointStore`'s own
/// convention.
#[derive(Clone, Default)]
pub struct InMemoryNodeCache {
    entries: Arc<RwLock<HashMap<NodeCacheKey, CachedDelta>>>,
}

impl InMemoryNodeCache {
    /// Construct a new, empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove every entry expired as of `now` from an already-locked map.
    fn sweep_expired_locked(entries: &mut HashMap<NodeCacheKey, CachedDelta>, now: DateTime<Utc>) {
        entries.retain(|_, cached| !cached.is_expired_at(now));
    }
}

#[async_trait]
impl NodeCachePort for InMemoryNodeCache {
    async fn get(&self, key: &NodeCacheKey) -> Result<Option<CachedDelta>, NodeCacheError> {
        let entries = self.entries.read().await;
        let now = Utc::now();
        Ok(entries
            .get(key)
            .filter(|cached| !cached.is_expired_at(now))
            .cloned())
    }

    async fn put(
        &self,
        key: &NodeCacheKey,
        delta: &StateDelta,
        ttl: Duration,
    ) -> Result<(), NodeCacheError> {
        let now = Utc::now();
        // `chrono::Duration::from_std` only fails for a `std::time::Duration`
        // too large to fit an `i64` millisecond count -- practically
        // unreachable for a cache TTL. Fall back to a zero delta (an
        // immediately-expiring entry) rather than panicking or silently
        // caching forever.
        let delta_span =
            chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::zero());
        let expires_at = now + delta_span;
        let cached = CachedDelta::new(delta.clone(), now, expires_at);

        let mut entries = self.entries.write().await;
        Self::sweep_expired_locked(&mut entries, now);
        // Overwrite semantics: re-`put`ting an existing key replaces it and
        // resets `stored_at`/`expires_at` to this call's values (last write
        // wins, contract case `overwriting_a_key_replaces_the_previous_entry`).
        entries.insert(key.clone(), cached);
        Ok(())
    }

    async fn invalidate(&self, prefix: &str) -> Result<u64, NodeCacheError> {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|key, _| !key.starts_with(prefix));
        Ok((before - entries.len()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_cache::contract_tests;

    // One #[tokio::test] per shared contract function, each against a fresh
    // cache, mirroring `waypoint::in_memory`'s own test module exactly.

    #[tokio::test]
    async fn put_then_get_returns_the_stored_delta() {
        contract_tests::put_then_get_returns_the_stored_delta(&InMemoryNodeCache::new()).await;
    }

    #[tokio::test]
    async fn get_of_an_absent_key_is_a_miss() {
        contract_tests::get_of_an_absent_key_is_a_miss(&InMemoryNodeCache::new()).await;
    }

    #[tokio::test]
    async fn entry_expires_after_its_ttl() {
        contract_tests::entry_expires_after_its_ttl(&InMemoryNodeCache::new()).await;
    }

    #[tokio::test]
    async fn entry_at_exactly_expires_at_is_expired() {
        contract_tests::entry_at_exactly_expires_at_is_expired(&InMemoryNodeCache::new()).await;
    }

    #[tokio::test]
    async fn invalidate_prefix_removes_only_matching_keys() {
        contract_tests::invalidate_prefix_removes_only_matching_keys(&InMemoryNodeCache::new())
            .await;
    }

    #[tokio::test]
    async fn overwriting_a_key_replaces_the_previous_entry() {
        contract_tests::overwriting_a_key_replaces_the_previous_entry(&InMemoryNodeCache::new())
            .await;
    }

    #[tokio::test]
    async fn concurrent_put_of_the_same_key_leaves_exactly_one_entry() {
        contract_tests::concurrent_put_of_the_same_key_leaves_exactly_one_entry(
            &InMemoryNodeCache::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn an_empty_delta_round_trips_as_a_hit_not_a_miss() {
        contract_tests::an_empty_delta_round_trips_as_a_hit_not_a_miss(&InMemoryNodeCache::new())
            .await;
    }

    #[tokio::test]
    async fn keys_differing_only_by_graph_prefix_do_not_collide() {
        contract_tests::keys_differing_only_by_graph_prefix_do_not_collide(
            &InMemoryNodeCache::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn run_all_contract_functions_smoke_aggregate() {
        contract_tests::run_all(&InMemoryNodeCache::new()).await;
    }

    /// Not part of the shared contract suite (backend-internal behavior,
    /// not a `NodeCachePort` contract clause): `put` opportunistically
    /// sweeps expired entries out of the map, so a cache that is never read
    /// again does not grow unbounded with dead entries.
    #[tokio::test]
    async fn put_sweeps_expired_entries() {
        let cache = InMemoryNodeCache::new();
        let expired_key = NodeCacheKey::new("expired");
        cache
            .put(&expired_key, &StateDelta::new(), Duration::ZERO)
            .await
            .unwrap();
        assert_eq!(cache.entries.read().await.len(), 1);

        let live_key = NodeCacheKey::new("live");
        cache
            .put(&live_key, &StateDelta::new(), Duration::from_secs(60))
            .await
            .unwrap();

        // The expired entry was swept out by the second `put`; only the
        // live one remains.
        let entries = cache.entries.read().await;
        assert_eq!(entries.len(), 1);
        assert!(entries.contains_key(&live_key));
        assert!(!entries.contains_key(&expired_key));
    }
}
