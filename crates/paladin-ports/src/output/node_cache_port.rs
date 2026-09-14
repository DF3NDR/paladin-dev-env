//! # Node Cache Port — Per-Node Result Caching (Doc 04 FT-FR-18…20, D-27)
//!
//! This module defines the port trait for caching a node's successful
//! result, so the superstep engine (`WarEngine::with_node_cache`, plan
//! 25-13) can serve a cache hit instead of re-executing a node whose
//! [`paladin_core::platform::container::aegis::CachePolicy`] applies and
//! whose composed key already has a live entry.
//!
//! ## Why this is separate from `WaypointPort`
//!
//! [`WaypointPort`](crate::output::waypoint_port::WaypointPort) persists the
//! WHOLE per-superstep checkpoint, written automatically and read back by
//! thread. A `NodeCachePort` entry is narrower and addressed differently: it
//! is keyed by a composed cache key (graph fingerprint + node id + input,
//! D-28) rather than by `ThreadId`, and it exists purely as a performance
//! optimisation -- unlike a `Waypoint`, losing every `NodeCachePort` entry
//! never loses correctness or resumability, only re-execution cost.
//!
//! ## Best-effort by construction (D-29)
//!
//! A cache read or write failure NEVER fails a run. [`NodeCachePort::get`]
//! returning `Err` is treated by the engine exactly like `Ok(None)` -- a
//! miss, so the node executes. [`NodeCachePort::put`] returning `Err` is
//! logged and never fails the node's run. This trait returns `Result` so a
//! backend CAN report what went wrong (for logging/metrics), not so a
//! caller MUST fail on it -- see each method's own rustdoc.
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync`: a cache may be read and
//! written concurrently across nodes, superstep iterations, and runs.

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::node_cache::CachedDelta;

/// An opaque cache key (D-28).
///
/// Composition (which fields of the graph fingerprint, node id, input, and
/// Paladin config fingerprint feed into it) is the engine's business (plan
/// 25-13's graph-fingerprint-inclusive default) -- this port only stores and
/// retrieves by whatever string a caller hands it, and exposes
/// [`NodeCacheKey::starts_with`] so [`NodeCachePort::invalidate`] callers and
/// backends can select a matching subset without knowing how a key was
/// composed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeCacheKey(String);

impl NodeCacheKey {
    /// Construct a key from an already-composed string.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Borrow the key as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this key starts with `prefix`.
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
}

impl fmt::Display for NodeCacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for NodeCacheKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for NodeCacheKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Errors a [`NodeCachePort`] backend can report.
///
/// A backend reporting an error here is NEVER fatal to a run (D-29): the
/// engine treats a [`NodeCachePort::get`] error exactly like a miss, and
/// logs a [`NodeCachePort::put`] error without failing the node. This type
/// exists so a backend CAN report what went wrong, not so a caller can fail
/// on it.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NodeCacheError {
    /// The underlying cache backend failed (a connection error, a protocol
    /// error, and so on).
    #[error("node cache backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) [`CachedDelta`] could not be
    /// (de)serialized.
    #[error("node cache serialization error: {0}")]
    Serialization(String),
}

/// Port trait for a per-node result cache (Doc 04 FT-FR-18, D-27).
///
/// # Hexagonal Architecture Context
///
/// ```text
/// ┌─────────────────────────────────────────────────────┐
/// │            WarEngine (paladin-battalion)             │
/// │   - looks up before attempt 1 (plan 25-13)           │
/// │   - stores after a successful attempt                │
/// └───────────────────────┬───────────────────────────────┘
///                         │
///                         ▼
/// ┌─────────────────────────────────────────────────────┐
/// │            NodeCachePort (this module)                │
/// └───────────────────────┬───────────────────────────────┘
///                         │
///                         ▼
/// ┌─────────────────────────────────────────────────────┐
/// │   InMemoryNodeCache | RedisNodeCache (paladin-storage) │
/// └─────────────────────────────────────────────────────┘
/// ```
#[async_trait]
pub trait NodeCachePort: Send + Sync {
    /// Look up a cached result for `key`.
    ///
    /// `Ok(None)` is a genuine miss: never written, expired (the TTL
    /// boundary is closed at `expires_at` -- an entry read at exactly that
    /// instant is a miss), or invalidated. An `Err` is ALSO treated as a
    /// miss by the engine (D-29) -- this method returns `Result` so a
    /// backend can report what went wrong, not so a caller must fail on it.
    async fn get(&self, key: &NodeCacheKey) -> Result<Option<CachedDelta>, NodeCacheError>;

    /// Store `delta` under `key`, valid for `ttl`.
    ///
    /// A `put` failure is logged and NEVER fails the node's run (D-29): the
    /// cache is an optimisation, never a correctness or availability
    /// dependency. Overwriting an existing `key` replaces it and resets its
    /// `stored_at`/`expires_at` to this call's values (last write wins).
    async fn put(
        &self,
        key: &NodeCacheKey,
        delta: &StateDelta,
        ttl: Duration,
    ) -> Result<(), NodeCacheError>;

    /// Remove every stored entry whose key starts with `prefix`. Returns the
    /// number of entries removed.
    ///
    /// An empty `prefix` removes everything. A `prefix` matching nothing
    /// returns `Ok(0)`, not an error.
    async fn invalidate(&self, prefix: &str) -> Result<u64, NodeCacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing trait bounds (mirrors waypoint_port.rs's
    // MockWaypointStore fixture).
    struct MockNodeCache;

    #[async_trait]
    impl NodeCachePort for MockNodeCache {
        async fn get(&self, _key: &NodeCacheKey) -> Result<Option<CachedDelta>, NodeCacheError> {
            Ok(None)
        }

        async fn put(
            &self,
            _key: &NodeCacheKey,
            _delta: &StateDelta,
            _ttl: Duration,
        ) -> Result<(), NodeCacheError> {
            Ok(())
        }

        async fn invalidate(&self, _prefix: &str) -> Result<u64, NodeCacheError> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn mock_cache_implements_trait() {
        let cache = MockNodeCache;
        let key = NodeCacheKey::new("k");
        assert!(cache.get(&key).await.unwrap().is_none());
        let delta = StateDelta::new();
        assert!(
            cache
                .put(&key, &delta, Duration::from_secs(1))
                .await
                .is_ok()
        );
        assert_eq!(cache.invalidate("prefix").await.unwrap(), 0);
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Box<dyn NodeCachePort>> = None;
    }

    #[test]
    fn node_cache_key_display_and_starts_with() {
        let key = NodeCacheKey::from("graphA:node1:abc");
        assert_eq!(key.to_string(), "graphA:node1:abc");
        assert_eq!(key.as_str(), "graphA:node1:abc");
        assert!(key.starts_with("graphA:"));
        assert!(!key.starts_with("graphB:"));
    }

    #[test]
    fn node_cache_key_from_string_and_str_are_equal() {
        let from_string = NodeCacheKey::from("k".to_string());
        let from_str = NodeCacheKey::from("k");
        assert_eq!(from_string, from_str);
        let constructed = NodeCacheKey::new("k");
        assert_eq!(constructed, from_str);
    }
}
