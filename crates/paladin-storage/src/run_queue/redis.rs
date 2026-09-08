/*
Redis Run Queue

A sorted-set visibility lease queue implementing `RunQueuePort` (D-06, D-08),
behind the `redis-queue` feature. Reuses the exact `ConnectionManager`
connection construction and error-mapping style
`crate::node_cache::redis::RedisNodeCache` already established
(27-PATTERNS.md: "the only transferable part") -- no second Redis client
type, no new dependency: the already-enabled `script` feature on the pinned
`redis` dependency (`Cargo.toml:59`) is what this adapter uses, for the
first time in this workspace (D-08's research correction: there is no
in-repo Lua precedent to copy the scripting idiom from -- this module
establishes it).

## Key layout

Three keys under `{key_prefix}` (default `paladin:run_queue`):

- `{prefix}:ready` -- a ZSET whose members are JSON-serialized `QueuedRun`
  payloads and whose score is a **microsecond** Unix timestamp: the time at
  which the member becomes visible to `dequeue`. A freshly enqueued message
  scores at its enqueue time (visible immediately); a claimed message's
  score is bumped forward to its lease expiry (invisible until the lease
  elapses); a nack'd message's score is bumped forward to
  `now + requeue_delay`. `depth()` is `ZCARD` of this one key -- it counts
  ready-and-leased messages together by construction, exactly as the port
  contract requires, with no separate counter to keep in sync.
- `{prefix}:leases` -- a HASH mapping a live lease token to the exact member
  string currently held under it in `ready` (so `extend_lease`/`ack`/`nack`
  know which `ready` entry to touch without re-deriving it).
- `{prefix}:lease_expiry` -- a ZSET mapping every token this queue has ever
  issued to the microsecond timestamp its lease expires (or expired) at.
  Presence with a score in the past is exactly `QueueError::LeaseExpired`;
  absence entirely is exactly `QueueError::UnknownLease` -- the D-07
  distinction the in-memory adapter's own bounded ring approximates with a
  recency window, and this adapter gets for free from Redis's own
  key-retention. Entries here are intentionally never proactively pruned: an
  old, superseded token (one a later claim's redelivery has moved past)
  stays classified `LeaseExpired` forever rather than "unknown", which
  matches the port's contract at the cost of unbounded growth over a
  long-running queue with heavy redelivery -- a known, accepted limitation
  of this plan's scope, not addressed here.

## Atomicity and clock source

Every read-then-write across these three keys (claim, extend, ack, nack) is
one `redis::Script` `EVAL`, run atomically by Redis's single-threaded script
execution -- never a multi-round-trip read/decide/write from this process,
which two concurrent replicas could otherwise interleave (T-27-03-01).
Every script reads the server's own `TIME` command for "now", never this
process's clock, so two `RedisRunQueue` replicas never disagree about
whether a lease has expired (D-08). `enqueue` and `depth` need no script:
`enqueue` is a single `ZADD` (score from one `TIME` read just before it -- a
few microseconds of client-observed staleness has no correctness effect on
when a message becomes visible), and `depth` is a single `ZCARD`.

Scores are microsecond-resolution Unix timestamps, not milliseconds: at
millisecond resolution, several `enqueue` calls issued back-to-back could
tie on score, and a ZSET breaks score ties lexicographically by member
string rather than by arrival order, which could reorder the queue.
Microsecond resolution makes that vanishingly unlikely for calls separated
by a real network round trip, though it does not make it structurally
impossible -- a known, accepted limitation of a timestamp-as-score design,
not solved here with a separate monotonic sequence counter.

## NOSCRIPT reload

None of the four scripts below hand-roll `SCRIPT LOAD`/`EVALSHA`/`NOSCRIPT`
handling: `redis` 0.32.7's `Script::invoke_async` already retries once on
`ErrorKind::NoScriptError` by reloading and re-invoking
(`redis-0.32.7/src/script.rs:180,205`) -- this module relies on that
built-in behavior rather than reimplementing it.
*/

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use redis::{AsyncCommands, Client, Script, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use paladin_ports::output::run_queue_port::{
    LeaseToken, LeasedRun, QueueError, QueuedRun, RunQueuePort,
};

/// Claim script (D-08): atomically finds the oldest visible member of
/// `ready` (score `<=` server-now), re-encodes it with `attempt`
/// incremented, re-scores it to the new lease expiry under a
/// caller-generated token, and records the lease in
/// `leases`/`lease_expiry`. Returns the new member's JSON, or Lua `false`
/// (a Redis Nil reply) if nothing is currently visible.
///
/// - `KEYS[1..3]` = `ready`, `leases`, `lease_expiry`
/// - `ARGV[1]` = lease duration, in microseconds
/// - `ARGV[2]` = the lease token (a UUID v7 string generated in Rust, never
///   in Lua, so the token comes from this process's own `uuid` dependency,
///   not a second source inside the script)
pub const RUN_QUEUE_CLAIM_LUA: &str = r#"
local time = redis.call('TIME')
local now_us = tonumber(time[1]) * 1000000 + tonumber(time[2])

local members = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', now_us, 'LIMIT', 0, 1)
if #members == 0 then
    return false
end

local member = members[1]
local decoded = cjson.decode(member)
decoded.attempt = decoded.attempt + 1
local new_member = cjson.encode(decoded)
local new_expiry = now_us + tonumber(ARGV[1])

redis.call('ZREM', KEYS[1], member)
redis.call('ZADD', KEYS[1], new_expiry, new_member)
redis.call('HSET', KEYS[2], ARGV[2], new_member)
redis.call('ZADD', KEYS[3], new_expiry, ARGV[2])

return new_member
"#;

/// Extend script (D-08): pushes a live lease's expiry to `server-now + by`
/// in both `ready` (the member's own score) and `lease_expiry`. Returns
/// `"ok"`, `"expired"` (the token is known but its lease already elapsed),
/// or `"unknown"` (the token was never issued by this queue).
///
/// - `KEYS[1..3]` = `ready`, `leases`, `lease_expiry`
/// - `ARGV[1]` = the lease token
/// - `ARGV[2]` = extend-by duration, in microseconds
pub const RUN_QUEUE_EXTEND_LUA: &str = r#"
local time = redis.call('TIME')
local now_us = tonumber(time[1]) * 1000000 + tonumber(time[2])

local expiry = redis.call('ZSCORE', KEYS[3], ARGV[1])
if not expiry then
    return 'unknown'
end
if tonumber(expiry) <= now_us then
    return 'expired'
end

local member = redis.call('HGET', KEYS[2], ARGV[1])
if not member then
    return 'unknown'
end

local new_expiry = now_us + tonumber(ARGV[2])
redis.call('ZADD', KEYS[1], new_expiry, member)
redis.call('ZADD', KEYS[3], new_expiry, ARGV[1])
return 'ok'
"#;

/// Ack script (D-08): removes a live lease's member from `ready` and the
/// token from `leases`/`lease_expiry` permanently. Returns `"ok"`,
/// `"expired"`, or `"unknown"` (see [`RUN_QUEUE_EXTEND_LUA`]).
///
/// - `KEYS[1..3]` = `ready`, `leases`, `lease_expiry`
/// - `ARGV[1]` = the lease token
pub const RUN_QUEUE_ACK_LUA: &str = r#"
local time = redis.call('TIME')
local now_us = tonumber(time[1]) * 1000000 + tonumber(time[2])

local expiry = redis.call('ZSCORE', KEYS[3], ARGV[1])
if not expiry then
    return 'unknown'
end
if tonumber(expiry) <= now_us then
    return 'expired'
end

local member = redis.call('HGET', KEYS[2], ARGV[1])
if member then
    redis.call('ZREM', KEYS[1], member)
end
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('ZREM', KEYS[3], ARGV[1])
return 'ok'
"#;

/// Nack script (D-08): re-encodes a live lease's member with `attempt`
/// incremented and re-scores it in `ready` to
/// `server-now + requeue_delay`, then drops the token from
/// `leases`/`lease_expiry`. Returns `"ok"`, `"expired"`, or `"unknown"`
/// (see [`RUN_QUEUE_EXTEND_LUA`]).
///
/// - `KEYS[1..3]` = `ready`, `leases`, `lease_expiry`
/// - `ARGV[1]` = the lease token
/// - `ARGV[2]` = requeue delay, in microseconds
pub const RUN_QUEUE_NACK_LUA: &str = r#"
local time = redis.call('TIME')
local now_us = tonumber(time[1]) * 1000000 + tonumber(time[2])

local expiry = redis.call('ZSCORE', KEYS[3], ARGV[1])
if not expiry then
    return 'unknown'
end
if tonumber(expiry) <= now_us then
    return 'expired'
end

local member = redis.call('HGET', KEYS[2], ARGV[1])
if not member then
    return 'unknown'
end

local decoded = cjson.decode(member)
decoded.attempt = decoded.attempt + 1
local new_member = cjson.encode(decoded)
local new_score = now_us + tonumber(ARGV[2])

redis.call('ZREM', KEYS[1], member)
redis.call('ZADD', KEYS[1], new_score, new_member)
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('ZREM', KEYS[3], ARGV[1])

return 'ok'
"#;

/// Configuration for the Redis-backed run queue (D-08).
///
/// `connection_url` never appears in a `Debug` rendering
/// (security.instructions.md: "no config type carrying a credential is
/// `Debug`-formatted outward"): `Debug` is implemented by hand below,
/// rendering the URL with any embedded password replaced, mirroring
/// `crate::node_cache::redis::RedisNodeCacheConfig`'s own redaction
/// discipline for its host/port/password fields.
#[derive(Clone, Serialize, Deserialize)]
pub struct RedisRunQueueConfig {
    /// The full Redis connection URL (`redis://[:password@]host:port/db`).
    pub connection_url: String,
    /// The key namespace every key this queue reads or writes is prefixed
    /// with.
    pub key_prefix: String,
}

impl Default for RedisRunQueueConfig {
    fn default() -> Self {
        Self {
            connection_url: "redis://127.0.0.1:6379".to_string(),
            key_prefix: "paladin:run_queue".to_string(),
        }
    }
}

impl std::fmt::Debug for RedisRunQueueConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisRunQueueConfig")
            .field(
                "connection_url",
                &redact_connection_url(&self.connection_url),
            )
            .field("key_prefix", &self.key_prefix)
            .finish()
    }
}

/// Render `url` with any embedded password replaced by a fixed placeholder,
/// or a fixed placeholder for the whole value if it does not even parse as
/// a URL (never echo an unparsable value verbatim -- it could still be, or
/// contain, a credential).
fn redact_connection_url(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(mut parsed) => {
            if parsed.password().is_some() {
                let _ = parsed.set_password(Some("REDACTED"));
            }
            parsed.to_string()
        }
        Err(_) => "[REDACTED: unparsable connection url]".to_string(),
    }
}

fn ready_key(prefix: &str) -> String {
    format!("{prefix}:ready")
}

fn leases_key(prefix: &str) -> String {
    format!("{prefix}:leases")
}

fn lease_expiry_key(prefix: &str) -> String {
    format!("{prefix}:lease_expiry")
}

/// Convert a `Duration` to whole microseconds, saturating rather than
/// panicking on an out-of-range value (CLAUDE.md: no panics in library
/// code) -- no realistic lease/delay duration this port is used with comes
/// anywhere near `i64::MAX` microseconds (about 292,471 years).
fn duration_to_micros(d: Duration) -> i64 {
    i64::try_from(d.as_micros()).unwrap_or(i64::MAX)
}

/// Interpret one of the three string results
/// [`RUN_QUEUE_EXTEND_LUA`], [`RUN_QUEUE_ACK_LUA`] and
/// [`RUN_QUEUE_NACK_LUA`] all return (`"ok"`/`"expired"`/`"unknown"`).
fn interpret_lease_op_result(result: &str, token: &LeaseToken) -> Result<(), QueueError> {
    match result {
        "ok" => Ok(()),
        "expired" => Err(QueueError::LeaseExpired {
            token: token.clone(),
        }),
        "unknown" => Err(QueueError::UnknownLease {
            token: token.clone(),
        }),
        other => Err(QueueError::Backend {
            message: format!("unexpected run queue lease script result: {other:?}"),
        }),
    }
}

/// Redis-backed `RunQueuePort` implementation, behind the `redis-queue`
/// feature (D-08): a sorted-set visibility lease driven by atomic Lua
/// scripts (see the module docs above for the key layout).
pub struct RedisRunQueue {
    #[allow(dead_code)]
    client: Client,
    conn: Arc<RwLock<ConnectionManager>>,
    prefix: String,
    claim_script: Script,
    extend_script: Script,
    ack_script: Script,
    nack_script: Script,
}

impl RedisRunQueue {
    /// Connect to Redis and construct a new `RedisRunQueue`.
    ///
    /// Mirrors `crate::node_cache::redis::RedisNodeCache::new`'s
    /// `ConnectionManager` handshake exactly.
    pub async fn new(config: RedisRunQueueConfig) -> Result<Self, QueueError> {
        let client =
            Client::open(config.connection_url.as_str()).map_err(|_| QueueError::Backend {
                message: format!(
                    "failed to parse redis run queue connection url: {}",
                    redact_connection_url(&config.connection_url)
                ),
            })?;

        let conn =
            ConnectionManager::new(client.clone())
                .await
                .map_err(|e| QueueError::Backend {
                    message: format!("failed to connect to redis run queue: {e}"),
                })?;

        Ok(Self {
            client,
            conn: Arc::new(RwLock::new(conn)),
            prefix: config.key_prefix,
            claim_script: Script::new(RUN_QUEUE_CLAIM_LUA),
            extend_script: Script::new(RUN_QUEUE_EXTEND_LUA),
            ack_script: Script::new(RUN_QUEUE_ACK_LUA),
            nack_script: Script::new(RUN_QUEUE_NACK_LUA),
        })
    }

    /// Read the server's own "now", in microseconds, via `TIME` -- the same
    /// clock source every claim/extend/ack/nack script uses internally, so
    /// `enqueue`'s visibility score is never measured against a different
    /// clock than the one that later decides whether it is due.
    async fn server_now_us(&self, conn: &mut ConnectionManager) -> Result<i64, QueueError> {
        let (seconds, micros): (i64, i64) =
            redis::cmd("TIME")
                .query_async(conn)
                .await
                .map_err(|e| QueueError::Backend {
                    message: format!("redis run queue TIME command failed: {e}"),
                })?;
        Ok(seconds * 1_000_000 + micros)
    }
}

#[async_trait]
impl RunQueuePort for RedisRunQueue {
    async fn enqueue(&self, run: QueuedRun) -> Result<(), QueueError> {
        let member = serde_json::to_string(&run).map_err(|e| QueueError::Serialization {
            message: e.to_string(),
        })?;

        let mut conn = self.conn.write().await;
        let now_us = self.server_now_us(&mut conn).await?;
        let _: () = conn
            .zadd(ready_key(&self.prefix), member, now_us)
            .await
            .map_err(|e| QueueError::Backend {
                message: format!("redis run queue enqueue failed: {e}"),
            })?;
        Ok(())
    }

    async fn dequeue(&self, lease: Duration) -> Result<Option<LeasedRun>, QueueError> {
        let token = LeaseToken::new(Uuid::now_v7().to_string());
        let mut conn = self.conn.write().await;

        let result: Option<String> = self
            .claim_script
            .key(ready_key(&self.prefix))
            .key(leases_key(&self.prefix))
            .key(lease_expiry_key(&self.prefix))
            .arg(duration_to_micros(lease))
            .arg(token.as_str())
            .invoke_async(&mut *conn)
            .await
            .map_err(|e| QueueError::Backend {
                message: format!("redis run queue claim failed: {e}"),
            })?;

        match result {
            None => Ok(None),
            Some(member_json) => {
                let queued: QueuedRun =
                    serde_json::from_str(&member_json).map_err(|e| QueueError::Serialization {
                        message: e.to_string(),
                    })?;
                Ok(Some(LeasedRun { queued, token }))
            }
        }
    }

    async fn extend_lease(&self, token: &LeaseToken, lease: Duration) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let result: String = self
            .extend_script
            .key(ready_key(&self.prefix))
            .key(leases_key(&self.prefix))
            .key(lease_expiry_key(&self.prefix))
            .arg(token.as_str())
            .arg(duration_to_micros(lease))
            .invoke_async(&mut *conn)
            .await
            .map_err(|e| QueueError::Backend {
                message: format!("redis run queue extend_lease failed: {e}"),
            })?;
        interpret_lease_op_result(&result, token)
    }

    async fn ack(&self, token: &LeaseToken) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let result: String = self
            .ack_script
            .key(ready_key(&self.prefix))
            .key(leases_key(&self.prefix))
            .key(lease_expiry_key(&self.prefix))
            .arg(token.as_str())
            .invoke_async(&mut *conn)
            .await
            .map_err(|e| QueueError::Backend {
                message: format!("redis run queue ack failed: {e}"),
            })?;
        interpret_lease_op_result(&result, token)
    }

    async fn nack(&self, token: &LeaseToken, requeue_delay: Duration) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;
        let result: String = self
            .nack_script
            .key(ready_key(&self.prefix))
            .key(leases_key(&self.prefix))
            .key(lease_expiry_key(&self.prefix))
            .arg(token.as_str())
            .arg(duration_to_micros(requeue_delay))
            .invoke_async(&mut *conn)
            .await
            .map_err(|e| QueueError::Backend {
                message: format!("redis run queue nack failed: {e}"),
            })?;
        interpret_lease_op_result(&result, token)
    }

    async fn depth(&self) -> Result<u64, QueueError> {
        let mut conn = self.conn.write().await;
        let count: u64 =
            conn.zcard(ready_key(&self.prefix))
                .await
                .map_err(|e| QueueError::Backend {
                    message: format!("redis run queue depth failed: {e}"),
                })?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_queue::contract_tests;
    use std::collections::HashSet;
    use std::sync::Arc as StdArc;

    /// Build a `RedisRunQueueConfig` for tests, overriding only
    /// `key_prefix` with a synthetic literal. No field here is a real
    /// hostname, credential, or connection string -- this module's own
    /// pure tests never open a connection (mirroring
    /// `crate::node_cache::redis`'s Docker-free unit-test convention).
    fn test_config() -> RedisRunQueueConfig {
        RedisRunQueueConfig {
            key_prefix: "test-run-queue".to_string(),
            ..RedisRunQueueConfig::default()
        }
    }

    #[test]
    fn redis_run_queue_config_default_matches_documented_values() {
        let config = RedisRunQueueConfig::default();
        assert_eq!(config.connection_url, "redis://127.0.0.1:6379");
        assert_eq!(config.key_prefix, "paladin:run_queue");
    }

    /// A `Debug` rendering of the config must never carry the password,
    /// while every other part of the URL stays readable.
    #[test]
    fn debug_rendering_never_prints_the_password() {
        let config = RedisRunQueueConfig {
            connection_url: "redis://:s3cr3t-queue-password@queue.example.internal:6379/1"
                .to_string(),
            ..test_config()
        };
        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("s3cr3t-queue-password"),
            "password leaked into Debug output: {rendered}"
        );
        assert!(
            rendered.contains("REDACTED"),
            "placeholder missing: {rendered}"
        );
        assert!(rendered.contains("queue.example.internal"));

        let unset = format!("{:?}", test_config());
        assert!(unset.contains("redis://127.0.0.1:6379"), "{unset}");
    }

    /// An unparsable connection string must never be echoed verbatim into a
    /// `Debug` rendering -- it could itself be, or contain, a credential.
    #[test]
    fn debug_rendering_of_an_unparsable_url_never_echoes_it_verbatim() {
        let config = RedisRunQueueConfig {
            connection_url: "not a valid url :: with a secret=s3cr3t".to_string(),
            ..test_config()
        };
        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("s3cr3t"),
            "unparsable connection url leaked into Debug output: {rendered}"
        );
    }

    // ── Tier 2: live-server contract suite (D-08) ────────────────────────
    //
    // Docker is absent from this devcontainer (RESEARCH.md Environment
    // Availability), so every test below detects an unreachable server and
    // self-skips -- mirroring `node_cache::redis`'s `cache_or_skip`
    // convention -- rather than failing or hanging when no `redis-test`
    // server is present. This whole module is ALSO compile-time gated
    // behind the `redis-queue` feature (in no default feature set; see
    // `paladin-storage/Cargo.toml`), so a plain `cargo test -p
    // paladin-storage` never attempts to build it, let alone run it.
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d redis-test
    // RUN_QUEUE_REDIS_TEST_URL=redis://127.0.0.1:6380/1 \
    //   cargo test -p paladin-storage --features redis-queue --lib run_queue::redis
    // ```
    //
    // Every test constructs its `RedisRunQueue`(s) with a per-test
    // randomized `key_prefix` (a fresh UUID), so concurrently-running tests
    // never observe or interfere with each other's keys even though they
    // all share the one live `redis-test` server.
    //
    // NEVER mark this suite as passed from a local run: D-51's own
    // prohibition (WINDOWS-tracked) is that Tier-2 evidence comes only from
    // the CI `redis-queue` job or a UAT run against a live server.

    fn redis_test_url() -> String {
        std::env::var("RUN_QUEUE_REDIS_TEST_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6380/1".to_string())
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing
    /// `url` to `redis::Client` -- mirrors
    /// `node_cache::redis::redis_reachable`'s rationale exactly: a
    /// connection refusal can otherwise absorb a connect-timeout budget
    /// before surfacing an error, making every test in this suite slow
    /// (rather than a clean, fast skip) when `redis-test` is simply not
    /// running.
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

    /// Returns a connected `RedisRunQueue` under a fresh, randomized
    /// `key_prefix`, or `None` (after printing a named `SKIP:` reason) if
    /// `redis-test` is not reachable.
    async fn queue_or_skip() -> Option<RedisRunQueue> {
        let url = redis_test_url();
        if !redis_reachable(&url) {
            println!(
                "SKIP: redis-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d redis-test` \
                 (or point RUN_QUEUE_REDIS_TEST_URL at a reachable server)"
            );
            return None;
        }

        let config = RedisRunQueueConfig {
            connection_url: url.clone(),
            key_prefix: format!("test-run-queue-{}", Uuid::new_v4()),
        };

        match RedisRunQueue::new(config).await {
            Ok(queue) => Some(queue),
            Err(e) => {
                println!("SKIP: redis-test connection failed at {url} ({e})");
                None
            }
        }
    }

    /// With no reachable Redis, the reachability probe reports unreachable
    /// and the suite takes the SKIP path (asserted directly against a
    /// deliberately-unreachable address -- NOT this suite's own configured
    /// `redis-test` server -- so this test is deterministic regardless of
    /// whether `redis-test` happens to be running).
    #[tokio::test]
    async fn redis_run_queue_self_skips_without_a_server() {
        assert!(!redis_reachable("redis://127.0.0.1:1/0"));
    }

    #[tokio::test]
    async fn redis_run_queue_fifo_order_and_distinct_lease_tokens() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::fifo_order_and_distinct_lease_tokens(&queue).await;
    }

    #[tokio::test]
    async fn redis_run_queue_lease_expiry_redelivers_with_attempt_incremented() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::lease_expiry_redelivers_with_attempt_incremented(&queue).await;
    }

    #[tokio::test]
    async fn redis_run_queue_extend_lease_keeps_message_hidden_until_new_expiry() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::extend_lease_keeps_message_hidden_until_new_expiry(&queue).await;
    }

    #[tokio::test]
    async fn redis_run_queue_ack_removes_message_permanently() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::ack_removes_message_permanently(&queue).await;
    }

    #[tokio::test]
    async fn redis_run_queue_nack_requeues_after_delay_with_attempt_incremented() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::nack_requeues_after_delay_with_attempt_incremented(&queue).await;
    }

    #[tokio::test]
    async fn redis_run_queue_expired_token_operations_return_lease_expired_and_touch_nothing() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::expired_token_operations_return_lease_expired_and_touch_nothing(&queue)
            .await;
    }

    #[tokio::test]
    async fn redis_run_queue_unknown_token_operations_return_unknown_lease() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::unknown_token_operations_return_unknown_lease(&queue).await;
    }

    #[tokio::test]
    async fn redis_run_queue_depth_counts_ready_plus_leased() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::depth_counts_ready_plus_leased(&queue).await;
    }

    /// Full aggregate run of every shared clause in one go, in addition to
    /// the per-clause tests above -- mirrors `waypoint::contract_tests`'
    /// `run_all` convenience, exercised here at least once per backend.
    #[tokio::test]
    async fn redis_run_queue_full_contract_suite_via_run_all() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        contract_tests::run_all(&queue).await;
    }

    /// Two `RedisRunQueue` instances with different `key_prefix` values,
    /// pointed at the same server, do not see each other's entries.
    #[tokio::test]
    async fn redis_run_queue_keys_are_namespaced_by_the_configured_prefix() {
        let Some(queue_a) = queue_or_skip().await else {
            return;
        };
        let Some(queue_b) = queue_or_skip().await else {
            return;
        };

        let thread =
            paladin_core::platform::container::waypoint::ThreadId::new("contract-redis-namespace")
                .unwrap();
        queue_a
            .enqueue(contract_tests::sample_queued_run(&thread))
            .await
            .unwrap();

        assert_eq!(
            queue_a.depth().await.unwrap(),
            1,
            "queue_a must see its own enqueue"
        );
        assert_eq!(
            queue_b.depth().await.unwrap(),
            0,
            "queue_b, under a different key_prefix, must not see queue_a's write"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn redis_run_queue_concurrent_workers_each_message_exactly_once() {
        let Some(queue) = queue_or_skip().await else {
            return;
        };
        let queue: StdArc<dyn RunQueuePort> = StdArc::new(queue);
        contract_tests::concurrent_workers_each_message_exactly_once(queue).await;
    }

    /// T-27-03-01: two independent `RedisRunQueue` connections/instances
    /// pointed at the SAME key prefix must never both claim the same
    /// message inside one lease window -- the atomicity guarantee comes
    /// from Redis's own single-threaded script execution, not from
    /// anything in this process, so this is provable only with two
    /// genuinely separate adapter instances (not two clones of one), unlike
    /// [`redis_run_queue_concurrent_workers_each_message_exactly_once`]
    /// above.
    #[tokio::test(flavor = "multi_thread")]
    async fn redis_run_queue_two_instances_sharing_one_prefix_never_double_claim() {
        let Some(seed) = queue_or_skip().await else {
            return;
        };
        let shared_prefix = seed.prefix.clone();
        let url = redis_test_url();

        let instance_b = RedisRunQueue::new(RedisRunQueueConfig {
            connection_url: url,
            key_prefix: shared_prefix,
        })
        .await
        .expect("second instance must connect to the same reachable server the first did");

        const MESSAGE_COUNT: usize = 40;
        const WORKERS_PER_INSTANCE: usize = 4;
        let thread = paladin_core::platform::container::waypoint::ThreadId::new(
            "contract-redis-two-instance",
        )
        .unwrap();
        for _ in 0..MESSAGE_COUNT {
            seed.enqueue(contract_tests::sample_queued_run(&thread))
                .await
                .unwrap();
        }

        let delivered: StdArc<tokio::sync::Mutex<HashSet<_>>> =
            StdArc::new(tokio::sync::Mutex::new(HashSet::new()));

        let mut workers: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
        for instance in [StdArc::new(seed), StdArc::new(instance_b)] {
            for _ in 0..WORKERS_PER_INSTANCE {
                let instance = StdArc::clone(&instance);
                let delivered = StdArc::clone(&delivered);
                workers.spawn(async move {
                    loop {
                        match instance.dequeue(Duration::from_secs(5)).await.unwrap() {
                            Some(leased) => {
                                {
                                    let mut delivered = delivered.lock().await;
                                    let first_delivery =
                                        delivered.insert(leased.queued.run_id.clone());
                                    assert!(
                                        first_delivery,
                                        "run_id {:?} claimed by more than one of the two instances",
                                        leased.queued.run_id
                                    );
                                }
                                instance.ack(&leased.token).await.unwrap();
                            }
                            None => {
                                if instance.depth().await.unwrap() == 0 {
                                    break;
                                }
                                tokio::time::sleep(Duration::from_millis(5)).await;
                            }
                        }
                    }
                });
            }
        }

        tokio::time::timeout(Duration::from_secs(30), async {
            while let Some(outcome) = workers.join_next().await {
                outcome.expect("a worker task panicked");
            }
        })
        .await
        .expect("two-instance concurrency test exceeded its 30s timeout guard");

        assert_eq!(
            delivered.lock().await.len(),
            MESSAGE_COUNT,
            "every run_id must be claimed exactly once across both instances"
        );
    }
}
