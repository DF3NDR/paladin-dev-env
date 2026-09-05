/*
Redis Node Cache

A `redis::aio::ConnectionManager`-backed implementation of `NodeCachePort`
(D-27), behind the `redis-cache` feature. Reuses the exact connection
construction and reconnect handling `crate::redis::RedisQueueAdapter`
already uses (RESEARCH.md `## Don't Hand-Roll`: "Redis connection management
... already solved, already tested, already the house pattern -- copy it,
don't reinvent it") -- no second Redis client type, no new dependency.

Each `CachedDelta` is stored as a serialized JSON string under
`{key_prefix}:{key}`, with a server-side expiry set from the TTL (via
`PSETEX`), so an entry vanishes on its own even if no reader ever checks its
embedded `expires_at`. `invalidate(prefix)` uses a cursor-based `SCAN`
(`scan_match`) -- never a full-keyspace-enumeration or whole-database-wipe
command (T-25-17).

Docker is absent from this devcontainer (RESEARCH.md Environment
Availability): this module's own unit tests below are Docker-free (config
defaults, key-building helpers), mirroring `crate::redis`'s own test-module
convention. The full `NodeCachePort` contract suite against a live server
runs in `crate::node_cache::contract_tests`'s Tier-2 test module -- see that
module for the self-skip convention this crate mirrors from
`waypoint::postgres`. Redis-tier evidence is CI-only (Docker-gated
`docker-integration`/`redis-cache-integration` job), never claimed as
locally verified.
*/

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use redis::{AsyncCommands, Client, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::node_cache::CachedDelta;
use paladin_ports::output::node_cache_port::{NodeCacheError, NodeCacheKey, NodeCachePort};

/// Configuration for the Redis-backed node cache (D-27).
///
/// Mirrors [`crate::redis::RedisQueueConfig`]'s field set exactly, with one
/// deliberate difference: `key_prefix` defaults to a cache-specific
/// namespace (`paladin:node_cache`), NOT the queue's `paladin:queue`, so the
/// two subsystems never collide in the same keyspace even when pointed at
/// the same Redis server (T-25-15).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisNodeCacheConfig {
    /// The Redis server hostname.
    pub redis_host: String,
    /// The Redis server port.
    pub redis_port: u16,
    /// The Redis server password, if authentication is required.
    pub redis_password: Option<String>,
    /// The Redis logical database index.
    pub redis_db: u8,
    /// Connection timeout, in seconds.
    pub connection_timeout: u64,
    /// The key namespace every entry this cache writes is prefixed with.
    pub key_prefix: String,
    /// Maximum connection retry attempts (informational; `ConnectionManager`
    /// itself owns the actual reconnect policy).
    pub max_retries: u32,
}

impl Default for RedisNodeCacheConfig {
    fn default() -> Self {
        Self {
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            redis_password: None,
            redis_db: 0,
            connection_timeout: 30,
            key_prefix: "paladin:node_cache".to_string(),
            max_retries: 3,
        }
    }
}

/// Build the full Redis key for `key` under `config`'s namespace.
fn cache_key(config: &RedisNodeCacheConfig, key: &NodeCacheKey) -> String {
    format!("{}:{}", config.key_prefix, key.as_str())
}

/// Build the `SCAN MATCH` pattern for every key whose `NodeCacheKey` starts
/// with `prefix`, under `config`'s namespace.
fn scan_pattern(config: &RedisNodeCacheConfig, prefix: &str) -> String {
    format!("{}:{}*", config.key_prefix, prefix)
}

/// Redis-backed `NodeCachePort` implementation, behind the `redis-cache`
/// feature (D-27).
pub struct RedisNodeCache {
    #[allow(dead_code)]
    client: Client,
    conn: Arc<RwLock<ConnectionManager>>,
    config: RedisNodeCacheConfig,
}

impl RedisNodeCache {
    /// Connect to Redis and construct a new `RedisNodeCache`.
    ///
    /// Mirrors `RedisQueueAdapter::new`'s connection-string construction and
    /// `ConnectionManager` handshake exactly.
    pub async fn new(config: RedisNodeCacheConfig) -> Result<Self, NodeCacheError> {
        let connection_url = if let Some(password) = &config.redis_password {
            format!(
                "redis://:{}@{}:{}/{}",
                password, config.redis_host, config.redis_port, config.redis_db
            )
        } else {
            format!(
                "redis://{}:{}/{}",
                config.redis_host, config.redis_port, config.redis_db
            )
        };

        let client = Client::open(connection_url).map_err(|e| NodeCacheError::Backend {
            source: Box::new(e),
        })?;

        let conn =
            ConnectionManager::new(client.clone())
                .await
                .map_err(|e| NodeCacheError::Backend {
                    source: Box::new(e),
                })?;

        Ok(Self {
            client,
            conn: Arc::new(RwLock::new(conn)),
            config,
        })
    }
}

#[async_trait]
impl NodeCachePort for RedisNodeCache {
    async fn get(&self, key: &NodeCacheKey) -> Result<Option<CachedDelta>, NodeCacheError> {
        let full_key = cache_key(&self.config, key);
        let mut conn = self.conn.write().await;
        let raw: Option<String> =
            conn.get(&full_key)
                .await
                .map_err(|e| NodeCacheError::Backend {
                    source: Box::new(e),
                })?;

        let Some(json) = raw else {
            return Ok(None);
        };

        let cached: CachedDelta = serde_json::from_str(&json)
            .map_err(|e| NodeCacheError::Serialization(e.to_string()))?;

        // Defense in depth: the server-side PSETEX expiry (set in `put`)
        // already enforces the TTL, but a `CachedDelta`'s own closed
        // `expires_at` boundary (FT-06 edge assumption) is re-checked here
        // too, so this backend's `get` behaves identically to
        // `InMemoryNodeCache::get` even under clock skew between this
        // process and the Redis server.
        if cached.is_expired_at(Utc::now()) {
            Ok(None)
        } else {
            Ok(Some(cached))
        }
    }

    async fn put(
        &self,
        key: &NodeCacheKey,
        delta: &StateDelta,
        ttl: Duration,
    ) -> Result<(), NodeCacheError> {
        let now = Utc::now();
        let delta_span =
            chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::zero());
        let expires_at = now + delta_span;
        let cached = CachedDelta::new(delta.clone(), now, expires_at);
        let json = serde_json::to_string(&cached)
            .map_err(|e| NodeCacheError::Serialization(e.to_string()))?;

        let ttl_ms = u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX);
        if ttl_ms == 0 {
            // A zero (or already-elapsed) TTL is immediately expired, and
            // Redis's PSETEX rejects a non-positive expiry outright -- skip
            // the write entirely rather than attempt an invalid command.
            // The contract is satisfied regardless: a `get` of a
            // never-written key is already a miss, so this is not even
            // observably different from writing-then-instantly-expiring.
            return Ok(());
        }

        let full_key = cache_key(&self.config, key);
        let mut conn = self.conn.write().await;
        let _: () =
            conn.pset_ex(&full_key, json, ttl_ms)
                .await
                .map_err(|e| NodeCacheError::Backend {
                    source: Box::new(e),
                })?;
        Ok(())
    }

    async fn invalidate(&self, prefix: &str) -> Result<u64, NodeCacheError> {
        let pattern = scan_pattern(&self.config, prefix);
        let mut conn = self.conn.write().await;

        // Cursor-based SCAN, never a full-keyspace-enumeration or
        // whole-database-wipe command (T-25-17): `scan_match` iterates the
        // keyspace incrementally, yielding control between batches rather
        // than blocking the server for the whole keyspace in one call.
        let mut matched: Vec<String> = Vec::new();
        {
            let mut iter: redis::AsyncIter<'_, String> =
                conn.scan_match(&pattern)
                    .await
                    .map_err(|e| NodeCacheError::Backend {
                        source: Box::new(e),
                    })?;
            // `safe_iterators` (enabled on the `redis` dependency) makes
            // `next_item` surface a per-item conversion error instead of
            // silently stopping the scan early.
            while let Some(matched_key) = iter.next_item().await {
                let matched_key = matched_key.map_err(|e| NodeCacheError::Backend {
                    source: Box::new(e),
                })?;
                matched.push(matched_key);
            }
        }

        if matched.is_empty() {
            return Ok(0);
        }

        let removed: usize = conn
            .del(&matched)
            .await
            .map_err(|e| NodeCacheError::Backend {
                source: Box::new(e),
            })?;
        Ok(removed as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_cache::contract_tests;

    /// Build a `RedisNodeCacheConfig` for tests, overriding only
    /// `key_prefix` with a synthetic literal. No field here is a real
    /// hostname, credential, or connection string -- this module's own
    /// tests never open a connection (mirroring `crate::redis`'s
    /// Docker-free unit-test convention).
    fn test_config() -> RedisNodeCacheConfig {
        RedisNodeCacheConfig {
            key_prefix: "test-node-cache".to_string(),
            ..RedisNodeCacheConfig::default()
        }
    }

    #[test]
    fn redis_node_cache_config_default_matches_documented_values() {
        let config = RedisNodeCacheConfig::default();
        assert_eq!(config.redis_host, "localhost");
        assert_eq!(config.redis_port, 6379);
        assert_eq!(config.redis_password, None);
        assert_eq!(config.redis_db, 0);
        assert_eq!(config.connection_timeout, 30);
        assert_eq!(config.key_prefix, "paladin:node_cache");
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn default_key_prefix_differs_from_the_queue_adapters_prefix() {
        // T-25-15: the two subsystems must never collide in the same
        // keyspace even when pointed at the same Redis server. Compared
        // against the literal `crate::redis::RedisQueueConfig::default()`
        // value directly (rather than importing that type) so this test
        // compiles under `--features redis-cache` alone, without also
        // requiring `redis-queue`.
        assert_ne!(RedisNodeCacheConfig::default().key_prefix, "paladin:queue");
    }

    #[test]
    fn cache_key_builds_expected_literal() {
        let config = test_config();
        let key = NodeCacheKey::new("graphA:node1:abc");
        assert_eq!(cache_key(&config, &key), "test-node-cache:graphA:node1:abc");
    }

    #[test]
    fn scan_pattern_builds_expected_literal() {
        let config = test_config();
        assert_eq!(scan_pattern(&config, "graphA:"), "test-node-cache:graphA:*");
    }

    // ── Tier 2: live-server contract suite (D-27) ────────────────────────
    //
    // Docker is absent from this devcontainer (RESEARCH.md Environment
    // Availability), so every test below detects an unreachable server and
    // self-skips -- mirroring `waypoint::postgres`'s `store_or_skip`
    // convention -- rather than failing or hanging when no `redis-test`
    // server is present. This whole module is ALSO compile-time gated
    // behind the `redis-cache` feature, which is in no default feature set
    // (see `paladin-storage/Cargo.toml`), so a plain `cargo test -p
    // paladin-storage` never attempts to build it, let alone run it.
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d redis-test
    // cargo test -p paladin-storage --features redis-cache --lib node_cache::redis
    // ```
    //
    // Every test below constructs its `RedisNodeCache` with a per-test
    // randomized `key_prefix` (a fresh UUID), so concurrently-running tests
    // never observe or `invalidate` each other's keys even though they all
    // share the one live `redis-test` server -- unlike the Postgres Tier-2
    // suite, this suite does NOT need `--test-threads=1`.

    fn redis_test_url() -> String {
        std::env::var("NODE_CACHE_REDIS_TEST_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6380/0".to_string())
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing
    /// `url` to `redis::Client` -- mirrors `waypoint::postgres::postgres_reachable`'s
    /// rationale exactly: a connection refusal can otherwise absorb a
    /// connect-timeout budget before surfacing an error, making every test
    /// in this suite slow (rather than a clean, fast skip) when
    /// `redis-test` is simply not running.
    fn redis_reachable(url: &str) -> bool {
        use std::net::ToSocketAddrs;

        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let port = parsed.port().unwrap_or(6379);

        (host, port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .is_some_and(|addr| {
                std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(750))
                    .is_ok()
            })
    }

    /// Returns a connected `RedisNodeCache` under a fresh, randomized
    /// `key_prefix`, or `None` (after printing a named SKIP reason) if
    /// `redis-test` is not reachable.
    async fn cache_or_skip() -> Option<RedisNodeCache> {
        let url = redis_test_url();
        if !redis_reachable(&url) {
            println!(
                "SKIP: redis-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d redis-test` \
                 (or point NODE_CACHE_REDIS_TEST_URL at a reachable server)"
            );
            return None;
        }

        let Ok(parsed) = url::Url::parse(&url) else {
            println!("SKIP: NODE_CACHE_REDIS_TEST_URL is not a valid url ({url})");
            return None;
        };
        let host = parsed.host_str().unwrap_or("127.0.0.1").to_string();
        let port = parsed.port().unwrap_or(6379);
        let db: u8 = parsed.path().trim_start_matches('/').parse().unwrap_or(0);

        let config = RedisNodeCacheConfig {
            redis_host: host,
            redis_port: port,
            redis_password: parsed.password().map(|p| p.to_string()),
            redis_db: db,
            key_prefix: format!("test-node-cache-{}", uuid::Uuid::new_v4()),
            ..RedisNodeCacheConfig::default()
        };

        match RedisNodeCache::new(config).await {
            Ok(cache) => Some(cache),
            Err(e) => {
                println!("SKIP: redis-test connection failed at {url} ({e})");
                None
            }
        }
    }

    /// Test 1: under a reachable `redis-test` server, all nine shared
    /// contract cases pass against `RedisNodeCache` exactly as they do
    /// against `InMemoryNodeCache`.
    #[tokio::test]
    async fn redis_node_cache_runs_the_full_contract_suite() {
        let Some(cache) = cache_or_skip().await else {
            return;
        };
        contract_tests::run_all(&cache).await;
    }

    /// Test 2: with no reachable Redis, the reachability probe reports
    /// unreachable and the suite takes the SKIP path (asserted directly
    /// against a deliberately-unreachable address -- NOT this suite's own
    /// configured `redis-test` server -- so this test is deterministic
    /// regardless of whether `redis-test` happens to be running).
    #[tokio::test]
    async fn redis_node_cache_self_skips_without_a_server() {
        assert!(!redis_reachable("redis://127.0.0.1:1/0"));
    }

    /// Test 3: two `RedisNodeCache` instances with different `key_prefix`
    /// values, pointed at the same server, do not see each other's entries
    /// (T-25-15).
    #[tokio::test]
    async fn redis_keys_are_namespaced_by_the_configured_prefix() {
        let Some(cache_a) = cache_or_skip().await else {
            return;
        };
        let Some(cache_b) = cache_or_skip().await else {
            return;
        };

        let key = NodeCacheKey::new("shared-name");
        cache_a
            .put(
                &key,
                &contract_tests::sample_delta(1),
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        assert!(
            cache_a.get(&key).await.unwrap().is_some(),
            "cache_a must see its own write"
        );
        assert_eq!(
            cache_b.get(&key).await.unwrap(),
            None,
            "cache_b, under a different key_prefix, must not see cache_a's write"
        );
    }

    /// Test 4: an entry's server-side expiry is set from the TTL, so the key
    /// vanishes from Redis itself without any reader-side `expires_at`
    /// check -- proven by reading the raw key directly with a plain `GET`
    /// after the TTL elapses.
    #[tokio::test]
    async fn redis_ttl_is_set_on_the_server_not_only_in_the_payload() {
        let Some(cache) = cache_or_skip().await else {
            return;
        };

        let key = NodeCacheKey::new("short-ttl");
        cache
            .put(
                &key,
                &contract_tests::sample_delta(1),
                Duration::from_millis(200),
            )
            .await
            .unwrap();

        // The entry is present immediately after the write.
        assert!(cache.get(&key).await.unwrap().is_some());

        tokio::time::sleep(Duration::from_millis(500)).await;

        // Read the RAW server key directly (bypassing `NodeCachePort::get`'s
        // own `is_expired_at` re-check) to prove the SERVER, not just this
        // process's re-check, has already dropped the key.
        let full_key = cache_key(&cache.config, &key);
        let mut conn = cache.conn.write().await;
        let raw: Option<String> = conn.get(&full_key).await.unwrap();
        assert_eq!(
            raw, None,
            "the server itself must have expired the key via PSETEX, independent of any \
             client-side expires_at check"
        );
    }
}
