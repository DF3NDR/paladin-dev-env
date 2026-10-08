/*
Redis Cadence

A `redis::aio::ConnectionManager`-backed implementation of `CadencePort` (PACE-03, D-00f,
D-09), behind the `redis-cadence` feature. Every worker pointed at the same Redis server shares
one pacing state per provider and model: a 429 recorded by worker A closes the gate worker B
reads on its next call (ROADMAP success criterion 3). It reuses the `Script`/`TIME` idiom
`crate::run_queue::redis` established and the shared optional `redis` dependency -- no second
client type, no new dependency.

## Atomicity and the server clock

Every operation is exactly one `redis::Script` `EVAL`, run atomically by Redis's
single-threaded script execution: [`CADENCE_RECORD_429_LUA`], [`CADENCE_GATE_LUA`] and
[`CADENCE_RECORD_SUCCESS_LUA`]. The in-flight rule ("a 429 recorded inside an active gate does
not escalate the streak") is a read-decide-write across the key's two fields; evaluated
client-side, two workers could interleave and escalate twice. Inside one script it cannot.

"Now" is always read from the server's own `TIME` inside the script, never from this process's
clock, and the scripts return **relative** waits in microseconds. No worker therefore ever
compares its clock with another host's: clock skew between workers can neither shorten nor
lengthen another worker's wait (D-00f). The only value a client contributes to a deadline is a
*duration* (the provider's retry delay, the policy's base and ceiling, the jitter fraction --
Redis never draws randomness), and all of them travel as `ARGV`; nothing is ever formatted into
script text (T-43-24).

## Key layout

One HASH per paced lane under `{key_prefix}:{provider}:{model}` (default prefix
[`DEFAULT_CADENCE_KEY_PREFIX`], `paladin:cadence`), with two fields: `nb` ("not before", the
server-clock microsecond Unix timestamp at which the gate clears) and `streak`. Both are
written with `string.format('%d', ..)`: a Lua number stringified by the default conversion is
`%.14g`, which renders a 16-digit microsecond timestamp as `1.7e+15` and corrupts the field
(research Pitfall 3).

Provider and model strings are opaque and never parsed. The one thing done to them is making
the concatenation injective: `%` and `:` are percent-escaped in the *provider* half only, so
`("a:b", "c")` and `("a", "b:c")` can never name the same key while an ordinary provider such
as `openai` and any model string are written verbatim. Equality stays exact, case-sensitive
byte equality.

## No residue

State exists only after a 429 (D-04). `gate` is read-only and never creates a key.
`record_success` deletes a key whose gate has also elapsed. Every write sets a TTL of
`max(2 * max_backoff, 60 s)` counted from the gate's own end (`PEXPIRE` to the delay plus that
TTL), so an idle provider leaves nothing behind however the process died (research Pitfall 11).

## A connection that is never unbounded

The `redis` crate configures no timeout by default, so a firewall that silently drops packets
would hang every LLM call that consults the gate (research Pitfall 9). `RedisCadence::new` is
therefore synchronous and does not touch the network (it only parses the URL); the first
operation connects lazily through a `ConnectionManager` built with explicit response and
connection timeouts (500 ms each by default) and a single reconnect retry, and every operation
is additionally wrapped in `tokio::time::timeout(response + connection)`. An elapsed deadline
surfaces as `CadenceError::Timeout`, any other failure as `CadenceError::Backend`; a failed
connect leaves the lazy cell empty so the next call tries again. A worker can therefore boot
while Redis is briefly down, and degraded-mode handling (43-08) sits above this type, not in it.

The manager is cloned per call (it is a multiplexed handle over an `Arc`) rather than kept
behind a shared lock, so concurrent gates and records pipeline on one socket instead of
serialising behind each other.

## Credentials

The connection URL may carry a password. It lives in a private field of
[`RedisCadenceConfig`], whose `Debug` is written by hand and renders it only through the shared
`redact_connection_url`; `RedisCadence` stores only the redacted form, and no error message
contains the URL (T-43-23).

## Assumptions

* A1: every worker in a fleet talks to the same Redis primary. A replica failover can lose a
  gate written just before it; the next 429 re-gates, and the degraded-mode composite keeps
  pacing in-process during the outage.
* A2: Redis 5 or later, where script effects (not commands) are replicated, so `TIME` followed
  by writes is allowed. CI and the compose stack run `redis:7-alpine`; the scripts were verified
  on 7.0.15.
* A3: the key namespace is fixed by the operator-visible prefix default; the configuration
  surface gives the operator no prefix key, so two independent fleets pointed at one Redis share
  pacing state for the same provider and model. That is correct when they share a provider
  account and over-conservative (never unsafe) otherwise.
*/

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use redis::{Client, Script};
use tokio::sync::OnceCell;

use crate::redis_url::redact_connection_url;
use paladin_ports::output::cadence_port::{
    CADENCE_DELAY_CEILING, CadenceError, CadenceKey, CadencePolicy, CadencePort, GateReading,
};

/// The default key namespace every cadence key is written under.
pub const DEFAULT_CADENCE_KEY_PREFIX: &str = "paladin:cadence";

/// How long a single response, and separately a single connection attempt, may take by default.
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);

/// The floor for a key's idle time-to-live.
const MIN_KEY_TTL: Duration = Duration::from_secs(60);

/// Reconnect attempts the manager makes before giving up on one command.
const RECONNECT_RETRIES: usize = 1;

/// Record-a-429 script: applies the in-flight rule and the back-off arithmetic atomically.
///
/// Reads `TIME`; if the gate is still active (`now < nb`) the 429 belongs to a request sent
/// before the gate, so the streak is unchanged and the gate is only raised when the provider's
/// delay asks for a later deadline. Otherwise the streak increases and the delay is the
/// provider's delay exactly or, with none, `max(base, floor(min(max, base * 2^(streak - 1)) *
/// fraction))` -- the same arithmetic as `CadencePolicy::delay_for`. Writes `nb = now + delay`
/// and `streak` as integer strings, extends the key's TTL to `delay + ttl`, and returns
/// `{delay_us, streak}`.
///
/// - `KEYS[1]` = the state hash for one provider and model
/// - `ARGV[1]` = the provider's retry delay in microseconds, or `-1` for none
/// - `ARGV[2]` = the base back-off, in microseconds
/// - `ARGV[3]` = the maximum back-off, in microseconds
/// - `ARGV[4]` = the jitter fraction in `[0, 1)` (drawn by the client; Redis draws no randomness)
/// - `ARGV[5]` = the idle key TTL, in milliseconds
pub const CADENCE_RECORD_429_LUA: &str = r#"
local time = redis.call('TIME')
local now = tonumber(time[1]) * 1000000 + tonumber(time[2])

local stored = redis.call('HMGET', KEYS[1], 'nb', 'streak')
local nb = tonumber(stored[1] or '0') or 0
local streak = tonumber(stored[2] or '0') or 0

local retry_after = tonumber(ARGV[1])
local base = tonumber(ARGV[2])
local max_backoff = tonumber(ARGV[3])
local fraction = tonumber(ARGV[4])
local ttl_ms = tonumber(ARGV[5])

local delay
if now < nb then
    delay = nb - now
    if retry_after >= 0 and retry_after > delay then
        delay = retry_after
    end
else
    if streak < 4294967295 then
        streak = streak + 1
    end
    if retry_after >= 0 then
        delay = retry_after
    else
        local ceiling = math.min(max_backoff, base * (2 ^ (streak - 1)))
        delay = math.max(base, math.floor(ceiling * fraction))
    end
end
delay = math.floor(delay)

redis.call('HSET', KEYS[1],
    'nb', string.format('%d', now + delay),
    'streak', string.format('%d', streak))
redis.call('PEXPIRE', KEYS[1], string.format('%d', math.floor(delay / 1000) + ttl_ms))
return {delay, streak}
"#;

/// Gate-read script: read-only, never creates a key.
///
/// Returns `{wait_us, streak}` where `wait_us` is `max(nb - now, 0)` on the server clock, so
/// the boundary is closed (a gate reads clear at exactly `nb`) and an unknown key reads
/// `{0, 0}`.
///
/// - `KEYS[1]` = the state hash for one provider and model
pub const CADENCE_GATE_LUA: &str = r#"
local time = redis.call('TIME')
local now = tonumber(time[1]) * 1000000 + tonumber(time[2])

local stored = redis.call('HMGET', KEYS[1], 'nb', 'streak')
local nb = tonumber(stored[1] or '0') or 0
local wait = nb - now
if wait < 0 then
    wait = 0
end
return {math.floor(wait), tonumber(stored[2] or '0') or 0}
"#;

/// Record-a-success script: resets the streak and drops a key whose gate has also elapsed.
///
/// An unknown key stays unknown (state exists only after a 429); a key whose gate is still
/// active keeps its gate with the streak at zero. Returns `1` when a key existed, else `0`.
///
/// - `KEYS[1]` = the state hash for one provider and model
pub const CADENCE_RECORD_SUCCESS_LUA: &str = r#"
if redis.call('EXISTS', KEYS[1]) == 0 then
    return 0
end

local time = redis.call('TIME')
local now = tonumber(time[1]) * 1000000 + tonumber(time[2])
local nb = tonumber(redis.call('HGET', KEYS[1], 'nb') or '0') or 0

if nb <= now then
    redis.call('DEL', KEYS[1])
else
    redis.call('HSET', KEYS[1], 'streak', '0')
end
return 1
"#;

/// Connection settings for [`RedisCadence`].
///
/// The connection URL (`redis://[:password@]host:port/db`) may carry a password, so it is held
/// privately and `Debug` is written by hand to render it only through the shared redaction
/// helper (security.instructions.md: no config type carrying a credential is `Debug`-formatted
/// outward).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_storage::cadence::redis::RedisCadenceConfig;
///
/// let config = RedisCadenceConfig::new("redis://:s3cret@cache.internal:6379/2")
///     .with_key_prefix("paladin:cadence")
///     .with_response_timeout(Duration::from_millis(250));
/// assert!(!format!("{config:?}").contains("s3cret"));
/// ```
#[derive(Clone)]
pub struct RedisCadenceConfig {
    connection_url: String,
    key_prefix: String,
    response_timeout: Duration,
    connection_timeout: Duration,
}

impl RedisCadenceConfig {
    /// Settings for the server at `connection_url`, with the default key prefix and 500 ms
    /// response and connection timeouts.
    pub fn new(connection_url: impl Into<String>) -> Self {
        Self {
            connection_url: connection_url.into(),
            key_prefix: DEFAULT_CADENCE_KEY_PREFIX.to_string(),
            response_timeout: DEFAULT_TIMEOUT,
            connection_timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Replace the key namespace (default [`DEFAULT_CADENCE_KEY_PREFIX`]).
    #[must_use]
    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = key_prefix.into();
        self
    }

    /// Replace how long one response may take (default 500 ms).
    #[must_use]
    pub fn with_response_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = timeout;
        self
    }

    /// Replace how long one connection attempt may take (default 500 ms).
    #[must_use]
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }
}

impl fmt::Debug for RedisCadenceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisCadenceConfig")
            .field(
                "connection_url",
                &redact_connection_url(&self.connection_url),
            )
            .field("key_prefix", &self.key_prefix)
            .field("response_timeout", &self.response_timeout)
            .field("connection_timeout", &self.connection_timeout)
            .finish()
    }
}

/// Redis-backed [`CadencePort`]: pacing state shared by every worker on one server.
///
/// See the module documentation for the atomicity, clock, key-layout and timeout rules.
///
/// # Examples
///
/// ```no_run
/// use paladin_storage::cadence::redis::{RedisCadence, RedisCadenceConfig};
/// use paladin_ports::output::cadence_port::CadencePolicy;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Synchronous and offline: this only parses the URL. The first operation connects.
/// let cadence = RedisCadence::new(
///     RedisCadenceConfig::new("redis://127.0.0.1:6379/2"),
///     CadencePolicy::default(),
/// )?;
/// # let _ = cadence;
/// # Ok(())
/// # }
/// ```
pub struct RedisCadence {
    client: Client,
    connection: OnceCell<ConnectionManager>,
    policy: CadencePolicy,
    jitter: fn() -> f64,
    key_prefix: String,
    redacted_url: String,
    response_timeout: Duration,
    connection_timeout: Duration,
    record_429_script: Script,
    gate_script: Script,
    record_success_script: Script,
}

impl RedisCadence {
    /// Build an adapter whose delay-less gates draw jitter from `rand::random::<f64>`.
    ///
    /// Synchronous and offline: it only parses the connection URL. The connection is made
    /// lazily by the first operation.
    ///
    /// # Errors
    ///
    /// [`CadenceError::Backend`] when the URL does not parse; the message carries the redacted
    /// URL, never the password.
    pub fn new(config: RedisCadenceConfig, policy: CadencePolicy) -> Result<Self, CadenceError> {
        let client =
            Client::open(config.connection_url.as_str()).map_err(|_| CadenceError::Backend {
                message: format!(
                    "invalid redis cadence connection url: {}",
                    redact_connection_url(&config.connection_url)
                ),
            })?;

        Ok(Self {
            client,
            connection: OnceCell::new(),
            policy,
            jitter: rand::random::<f64>,
            key_prefix: config.key_prefix,
            redacted_url: redact_connection_url(&config.connection_url),
            response_timeout: config.response_timeout,
            connection_timeout: config.connection_timeout,
            record_429_script: Script::new(CADENCE_RECORD_429_LUA),
            gate_script: Script::new(CADENCE_GATE_LUA),
            record_success_script: Script::new(CADENCE_RECORD_SUCCESS_LUA),
        })
    }

    /// Replace the jitter source -- the deterministic seam for tests. The function must return
    /// a fraction in `[0, 1)`; anything else is clamped into range before it reaches the
    /// script.
    #[must_use]
    pub fn with_jitter(mut self, jitter: fn() -> f64) -> Self {
        self.jitter = jitter;
        self
    }

    /// The state hash for `key`: `{prefix}:{provider}:{model}`, with `%` and `:` escaped in the
    /// provider half so the concatenation is injective (see the module docs).
    fn state_key(&self, key: &CadenceKey) -> String {
        let provider = key.provider().replace('%', "%25").replace(':', "%3A");
        format!("{}:{provider}:{}", self.key_prefix, key.model())
    }

    /// The idle TTL every write refreshes: `max(2 * max_backoff, 60 s)`, in milliseconds.
    fn key_ttl_ms(&self) -> i64 {
        let doubled = self
            .policy
            .max_backoff()
            .min(CADENCE_DELAY_CEILING)
            .saturating_mul(2);
        millis(doubled.max(MIN_KEY_TTL))
    }

    /// The deadline one operation, connection included, must finish within.
    fn operation_budget(&self) -> Duration {
        self.response_timeout
            .saturating_add(self.connection_timeout)
    }

    /// The shared connection, established on first use. A failed attempt leaves the cell empty,
    /// so the next call tries again.
    async fn manager(&self) -> Result<ConnectionManager, CadenceError> {
        let manager = self
            .connection
            .get_or_try_init(|| async {
                let config = ConnectionManagerConfig::new()
                    .set_response_timeout(self.response_timeout)
                    .set_connection_timeout(self.connection_timeout)
                    .set_number_of_retries(RECONNECT_RETRIES);
                ConnectionManager::new_with_config(self.client.clone(), config).await
            })
            .await
            .map_err(|e| backend_error("connect", &e))?;
        Ok(manager.clone())
    }

    /// Run `script` against `key`'s state hash with `args`, inside the operation deadline.
    async fn evaluate<T: redis::FromRedisValue>(
        &self,
        script: &Script,
        key: &CadenceKey,
        args: &[String],
    ) -> Result<T, CadenceError> {
        let budget = self.operation_budget();
        let outcome = tokio::time::timeout(budget, async {
            let mut connection = self.manager().await?;
            let mut invocation = script.key(self.state_key(key));
            for arg in args {
                invocation.arg(arg.as_str());
            }
            invocation
                .invoke_async::<T>(&mut connection)
                .await
                .map_err(|e| backend_error("script", &e))
        })
        .await;
        match outcome {
            Ok(result) => result,
            Err(_elapsed) => Err(CadenceError::Timeout { after: budget }),
        }
    }
}

impl fmt::Debug for RedisCadence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisCadence")
            .field("connection_url", &self.redacted_url)
            .field("key_prefix", &self.key_prefix)
            .field("policy", &self.policy)
            .field("response_timeout", &self.response_timeout)
            .field("connection_timeout", &self.connection_timeout)
            .finish()
    }
}

/// Whole milliseconds, saturating.
fn millis(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

/// Whole microseconds, clamped to the 24 h delay ceiling first so the script never sees a
/// provider-controlled number large enough to overflow its arithmetic.
fn clamped_micros(duration: Duration) -> i64 {
    i64::try_from(duration.min(CADENCE_DELAY_CEILING).as_micros()).unwrap_or(i64::MAX)
}

/// Map a jitter fraction into `[0, 1)`: NaN and negatives to zero, one and above to just below
/// one (the same mapping `CadencePolicy::delay_for` applies).
fn sanitized_fraction(fraction: f64) -> f64 {
    if fraction.is_nan() || fraction < 0.0 {
        0.0
    } else if fraction >= 1.0 {
        1.0 - f64::EPSILON
    } else {
        fraction
    }
}

/// A backend error that carries the redis crate's description but never the connection URL.
fn backend_error(context: &str, error: &redis::RedisError) -> CadenceError {
    CadenceError::Backend {
        message: format!("redis cadence {context} failed: {error}"),
    }
}

/// Decode the `{wait_us, streak}` reply of the record and gate scripts.
fn reading_from(values: &[i64]) -> Result<GateReading, CadenceError> {
    let [wait_us, streak] = values else {
        return Err(CadenceError::Backend {
            message: format!(
                "unexpected redis cadence script reply: {} values",
                values.len()
            ),
        });
    };
    // A forged or corrupted `nb` must not translate into an unbounded wait.
    let wait =
        Duration::from_micros(u64::try_from(*wait_us).unwrap_or(0)).min(CADENCE_DELAY_CEILING);
    let streak = u32::try_from((*streak).max(0)).unwrap_or(u32::MAX);
    Ok(GateReading::new(wait, streak))
}

#[async_trait]
impl CadencePort for RedisCadence {
    async fn gate(&self, key: &CadenceKey) -> Result<GateReading, CadenceError> {
        let values: Vec<i64> = self.evaluate(&self.gate_script, key, &[]).await?;
        reading_from(&values)
    }

    async fn record_rate_limited(
        &self,
        key: &CadenceKey,
        retry_after: Option<Duration>,
    ) -> Result<GateReading, CadenceError> {
        let args = [
            retry_after.map_or(-1, clamped_micros).to_string(),
            clamped_micros(self.policy.base_backoff())
                .max(1)
                .to_string(),
            clamped_micros(self.policy.max_backoff()).max(1).to_string(),
            sanitized_fraction((self.jitter)()).to_string(),
            self.key_ttl_ms().to_string(),
        ];
        let values: Vec<i64> = self.evaluate(&self.record_429_script, key, &args).await?;
        reading_from(&values)
    }

    async fn record_success(&self, key: &CadenceKey) -> Result<(), CadenceError> {
        self.evaluate::<i64>(&self.record_success_script, key, &[])
            .await
            .map(|_existed| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cadence::contract_tests;
    use std::time::Instant;
    use uuid::Uuid;

    // ---- Live-server tests -------------------------------------------------------------------
    //
    // These tests need a reachable Redis. They self-skip (printing a `SKIP:` line) when none is,
    // exactly like `run_queue::redis`: a bare `cargo test` on a machine without Redis stays
    // green, and the `redis-cadence-integration` CI job turns any SKIP into a failure so the
    // live run cannot be silently vacuous. Point `CADENCE_REDIS_TEST_URL` at another server, or
    // start one with `docker compose -f docker/docker-compose.test.yml up -d redis-test`.
    //
    // Every test uses a fresh `key_prefix`, so concurrently running tests never see each other's
    // keys on the one shared server.

    /// A tens-of-milliseconds base keeps the real-time contract suite fast; the maximum is at
    /// least eight times the base so the escalation clause sees growth.
    const BASE: Duration = Duration::from_millis(50);
    const MAX: Duration = Duration::from_millis(800);

    fn policy() -> CadencePolicy {
        CadencePolicy::new(BASE, MAX).expect("valid policy")
    }

    fn cadence_redis_test_url() -> String {
        std::env::var("CADENCE_REDIS_TEST_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6380/2".to_string())
    }

    /// A cheap, short-timeout TCP probe tried before handing `url` to the redis client, so a
    /// missing server is a fast, clean skip rather than a connect-timeout per test.
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
                std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(750)).is_ok()
            })
    }

    fn unique_prefix() -> String {
        format!("test-cadence-{}", Uuid::new_v4())
    }

    /// A worker: its own adapter, hence its own connection, under `prefix`.
    fn worker(prefix: &str, jitter: fn() -> f64) -> RedisCadence {
        RedisCadence::new(
            RedisCadenceConfig::new(cadence_redis_test_url()).with_key_prefix(prefix),
            policy(),
        )
        .expect("a parsable test url")
        .with_jitter(jitter)
    }

    /// A connected adapter under a fresh prefix, or `None` after printing a named `SKIP:`.
    fn cadence_or_skip(jitter: fn() -> f64) -> Option<(RedisCadence, String)> {
        let url = cadence_redis_test_url();
        if !redis_reachable(&url) {
            println!(
                "SKIP: redis-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d redis-test` \
                 (or point CADENCE_REDIS_TEST_URL at a reachable server)"
            );
            return None;
        }
        let prefix = unique_prefix();
        Some((worker(&prefix, jitter), prefix))
    }

    async fn raw_connection() -> redis::aio::MultiplexedConnection {
        redis::Client::open(cadence_redis_test_url())
            .expect("a parsable test url")
            .get_multiplexed_async_connection()
            .await
            .expect("a raw connection to the test server")
    }

    fn floor() -> f64 {
        0.0
    }

    fn near_one() -> f64 {
        0.999
    }

    #[tokio::test]
    async fn redis_cadence_self_skips_without_a_server() {
        assert!(!redis_reachable("redis://127.0.0.1:1/0"));
    }

    #[tokio::test]
    async fn redis_cadence_passes_the_shared_contract() {
        // The suite runs at both jitter ends: the floor proves `wait >= base`, near one proves
        // the exponential ceiling. A separate prefix per run keeps the two independent.
        for jitter in [floor as fn() -> f64, near_one as fn() -> f64] {
            let Some((cadence, _prefix)) = cadence_or_skip(jitter) else {
                return;
            };
            contract_tests::run_all(&cadence, policy()).await;
        }
    }

    #[tokio::test]
    async fn two_workers_share_one_gate() {
        let Some((worker_a, prefix)) = cadence_or_skip(floor) else {
            return;
        };
        let worker_b = worker(&prefix, floor);
        let key = CadenceKey::new("openai", "gpt-4o");
        let other = CadenceKey::new("openai", "gpt-4o-mini");

        let recorded = worker_a
            .record_rate_limited(&key, Some(Duration::from_secs(2)))
            .await
            .expect("worker A records the 429");
        assert_eq!(recorded.streak(), 1);

        let seen = worker_b.gate(&key).await.expect("worker B reads the gate");
        assert!(
            seen.wait() > Duration::from_millis(1500) && seen.wait() <= Duration::from_secs(2),
            "worker B must see worker A's gate, got {:?}",
            seen.wait()
        );
        assert_eq!(seen.streak(), 1);

        let unrelated = worker_b
            .gate(&other)
            .await
            .expect("worker B reads another model");
        assert!(
            unrelated.is_clear(),
            "another model is not gated: {unrelated:?}"
        );
    }

    #[tokio::test]
    async fn a_success_on_one_worker_clears_the_streak_for_the_other() {
        let Some((worker_a, prefix)) = cadence_or_skip(floor) else {
            return;
        };
        let worker_b = worker(&prefix, floor);
        let key = CadenceKey::new("anthropic", "claude");

        worker_a
            .record_rate_limited(&key, Some(Duration::from_millis(30)))
            .await
            .expect("record");
        tokio::time::sleep(Duration::from_millis(60)).await;
        worker_b.record_success(&key).await.expect("success");

        let after = worker_a.gate(&key).await.expect("gate");
        assert!(after.is_clear());
        assert_eq!(
            after.streak(),
            0,
            "worker B's success reset the shared streak"
        );
    }

    #[tokio::test]
    async fn keys_that_differ_across_the_provider_model_boundary_stay_independent() {
        let Some((cadence, _prefix)) = cadence_or_skip(floor) else {
            return;
        };
        cadence
            .record_rate_limited(&CadenceKey::new("a:b", "c"), None)
            .await
            .expect("record");
        for other in [
            CadenceKey::new("a", "b:c"),
            CadenceKey::new("a:b:c", ""),
            CadenceKey::new("a%3Ab", "c"),
        ] {
            let reading = cadence.gate(&other).await.expect("gate");
            assert!(
                reading.is_clear() && reading.streak() == 0,
                "{other:?} must not share the key of (\"a:b\", \"c\")"
            );
        }
    }

    #[tokio::test]
    async fn stored_not_before_is_an_integer_string_and_every_key_has_a_ttl() {
        let Some((cadence, prefix)) = cadence_or_skip(near_one) else {
            return;
        };
        let mut raw = raw_connection().await;

        let first = CadenceKey::new("openai", "gpt-4o");
        let explicit = CadenceKey::new("openai", "gpt-4o-mini");
        let hostile = CadenceKey::new("openai", "hostile");
        cadence
            .record_rate_limited(&first, None)
            .await
            .expect("record");
        cadence
            .record_rate_limited(&explicit, Some(Duration::from_secs(2)))
            .await
            .expect("record");
        // An in-flight 429 rewrites the same fields; the format must survive that too.
        cadence
            .record_rate_limited(&explicit, Some(Duration::from_secs(3)))
            .await
            .expect("record");
        cadence
            .record_rate_limited(&hostile, Some(Duration::MAX))
            .await
            .expect("record");

        for key in [&first, &explicit, &hostile] {
            let redis_key = cadence.state_key(key);
            let nb: String = redis::cmd("HGET")
                .arg(&redis_key)
                .arg("nb")
                .query_async(&mut raw)
                .await
                .expect("HGET nb");
            assert!(
                !nb.is_empty() && nb.chars().all(|c| c.is_ascii_digit()),
                "nb must be a plain integer string (no exponent, no fraction): {nb:?}"
            );
            let streak: String = redis::cmd("HGET")
                .arg(&redis_key)
                .arg("streak")
                .query_async(&mut raw)
                .await
                .expect("HGET streak");
            assert!(
                !streak.is_empty() && streak.chars().all(|c| c.is_ascii_digit()),
                "streak must be a plain integer string: {streak:?}"
            );
            let pttl: i64 = redis::cmd("PTTL")
                .arg(&redis_key)
                .query_async(&mut raw)
                .await
                .expect("PTTL");
            assert!(pttl > 0, "{key:?}: every key carries a TTL (PTTL = {pttl})");
        }

        // The TTL outlives the gate it protects: a 2 s gate on a key idle-TTL'd at 60 s.
        let explicit_pttl: i64 = redis::cmd("PTTL")
            .arg(cadence.state_key(&explicit))
            .query_async(&mut raw)
            .await
            .expect("PTTL");
        assert!(
            explicit_pttl > 60_000,
            "TTL is gate + idle floor: {explicit_pttl}"
        );

        // And the whole namespace holds exactly the three keys written.
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(format!("{prefix}:*"))
            .query_async(&mut raw)
            .await
            .expect("KEYS");
        assert_eq!(keys.len(), 3, "{keys:?}");
    }

    #[tokio::test]
    async fn gate_on_an_unknown_key_creates_no_key() {
        let Some((cadence, prefix)) = cadence_or_skip(floor) else {
            return;
        };
        let mut raw = raw_connection().await;
        let key = CadenceKey::new("openai", "never-limited");

        let reading = cadence.gate(&key).await.expect("gate");
        assert!(reading.is_clear());
        assert_eq!(reading.streak(), 0);
        cadence
            .record_success(&key)
            .await
            .expect("success on an unknown key");
        cadence.gate(&key).await.expect("gate again");

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(format!("{prefix}:*"))
            .query_async(&mut raw)
            .await
            .expect("KEYS");
        assert!(
            keys.is_empty(),
            "reads and a success created keys: {keys:?}"
        );
    }

    #[tokio::test]
    async fn success_drops_an_elapsed_gate_and_keeps_a_live_one() {
        let Some((cadence, _prefix)) = cadence_or_skip(floor) else {
            return;
        };
        let mut raw = raw_connection().await;
        let live = CadenceKey::new("openai", "live");
        let elapsed = CadenceKey::new("openai", "elapsed");

        cadence
            .record_rate_limited(&live, Some(Duration::from_secs(5)))
            .await
            .expect("record");
        cadence
            .record_rate_limited(&elapsed, Some(Duration::from_millis(20)))
            .await
            .expect("record");
        tokio::time::sleep(Duration::from_millis(60)).await;

        cadence.record_success(&live).await.expect("success");
        cadence.record_success(&elapsed).await.expect("success");

        let live_exists: i64 = redis::cmd("EXISTS")
            .arg(cadence.state_key(&live))
            .query_async(&mut raw)
            .await
            .expect("EXISTS");
        let elapsed_exists: i64 = redis::cmd("EXISTS")
            .arg(cadence.state_key(&elapsed))
            .query_async(&mut raw)
            .await
            .expect("EXISTS");
        assert_eq!(live_exists, 1, "a success must not lift a live gate");
        assert_eq!(elapsed_exists, 0, "an elapsed gate is dropped by a success");

        let reading = cadence.gate(&live).await.expect("gate");
        assert!(!reading.is_clear());
        assert_eq!(reading.streak(), 0, "the streak was reset");
    }

    // ---- Tests that need no server -----------------------------------------------------------

    #[test]
    fn scripts_read_the_server_clock_and_format_integers() {
        for (name, script) in [
            ("record_429", CADENCE_RECORD_429_LUA),
            ("gate", CADENCE_GATE_LUA),
            ("record_success", CADENCE_RECORD_SUCCESS_LUA),
        ] {
            assert!(
                script.contains("redis.call('TIME')"),
                "{name} must read the server clock"
            );
        }
        assert!(CADENCE_RECORD_429_LUA.contains("string.format('%d'"));
        // No script takes "now" from the caller: record_429 has five ARGV and none is a clock.
        assert!(!CADENCE_RECORD_429_LUA.contains("ARGV[6]"));
        // The gate script only reads.
        for write in ["HSET", "PEXPIRE", "DEL", "SET"] {
            assert!(
                !CADENCE_GATE_LUA.contains(write),
                "the gate script must be read-only, found {write}"
            );
        }
    }

    #[test]
    fn new_is_synchronous_and_does_not_connect() {
        // No runtime exists on this thread: construction against an unreachable address must
        // succeed without touching the network.
        let started = Instant::now();
        let cadence = RedisCadence::new(
            RedisCadenceConfig::new("redis://127.0.0.1:1/0"),
            CadencePolicy::default(),
        );
        assert!(cadence.is_ok());
        assert!(started.elapsed() < Duration::from_millis(250));
    }

    #[test]
    fn an_unparsable_url_is_rejected_without_echoing_it() {
        let error = RedisCadence::new(
            RedisCadenceConfig::new("not a url :: password=hunter2"),
            CadencePolicy::default(),
        )
        .expect_err("an unparsable url");
        let rendered = format!("{error} / {error:?}");
        assert!(!rendered.contains("hunter2"), "{rendered}");
    }

    #[test]
    fn config_defaults_match_the_documented_values() {
        let config = RedisCadenceConfig::new("redis://127.0.0.1:6379/0");
        assert_eq!(config.key_prefix, DEFAULT_CADENCE_KEY_PREFIX);
        assert_eq!(DEFAULT_CADENCE_KEY_PREFIX, "paladin:cadence");
        assert_eq!(config.response_timeout, Duration::from_millis(500));
        assert_eq!(config.connection_timeout, Duration::from_millis(500));
    }

    #[test]
    fn key_ttl_is_twice_the_maximum_back_off_with_a_sixty_second_floor() {
        let small = RedisCadence::new(RedisCadenceConfig::new("redis://127.0.0.1:1/0"), policy())
            .expect("valid");
        assert_eq!(
            small.key_ttl_ms(),
            60_000,
            "the floor applies to a short maximum"
        );

        let large = RedisCadence::new(
            RedisCadenceConfig::new("redis://127.0.0.1:1/0"),
            CadencePolicy::new(Duration::from_secs(1), Duration::from_secs(120)).expect("valid"),
        )
        .expect("valid");
        assert_eq!(large.key_ttl_ms(), 240_000);
    }

    #[test]
    fn state_key_is_injective_across_the_provider_model_boundary() {
        let cadence = RedisCadence::new(
            RedisCadenceConfig::new("redis://127.0.0.1:1/0").with_key_prefix("p"),
            policy(),
        )
        .expect("valid");
        assert_eq!(
            cadence.state_key(&CadenceKey::new("openai", "gpt-4o")),
            "p:openai:gpt-4o",
            "an ordinary key is written verbatim"
        );
        assert_ne!(
            cadence.state_key(&CadenceKey::new("a:b", "c")),
            cadence.state_key(&CadenceKey::new("a", "b:c"))
        );
        assert_ne!(
            cadence.state_key(&CadenceKey::new("a%3Ab", "c")),
            cadence.state_key(&CadenceKey::new("a:b", "c"))
        );
    }

    #[test]
    fn fractions_and_delays_are_clamped_before_they_reach_the_script() {
        assert_eq!(sanitized_fraction(f64::NAN), 0.0);
        assert_eq!(sanitized_fraction(-3.0), 0.0);
        assert!(sanitized_fraction(7.0) < 1.0);
        assert_eq!(sanitized_fraction(0.25), 0.25);
        assert_eq!(
            clamped_micros(Duration::MAX),
            i64::try_from(CADENCE_DELAY_CEILING.as_micros()).expect("fits")
        );
        assert_eq!(clamped_micros(Duration::from_millis(7)), 7_000);
    }

    #[test]
    fn a_malformed_reply_is_a_backend_error_and_a_forged_wait_is_clamped() {
        assert!(matches!(
            reading_from(&[1]),
            Err(CadenceError::Backend { .. })
        ));
        let forged = reading_from(&[i64::MAX, -4]).expect("two values");
        assert_eq!(forged.wait(), CADENCE_DELAY_CEILING);
        assert_eq!(forged.streak(), 0);
    }

    #[tokio::test]
    async fn debug_and_errors_never_render_the_password() {
        let url = "redis://:hunter2@127.0.0.1:1/0";
        let config = RedisCadenceConfig::new(url)
            .with_response_timeout(Duration::from_millis(100))
            .with_connection_timeout(Duration::from_millis(100));
        let rendered_config = format!("{config:?}");
        assert!(!rendered_config.contains("hunter2"), "{rendered_config}");
        assert!(rendered_config.contains("REDACTED"), "{rendered_config}");

        let cadence = RedisCadence::new(config, CadencePolicy::default()).expect("parsable");
        let rendered = format!("{cadence:?}");
        assert!(!rendered.contains("hunter2"), "{rendered}");

        // Nothing listens on port 1: the first operation fails, and neither rendering of the
        // error carries the credential.
        let key = CadenceKey::new("openai", "gpt-4o");
        let error = cadence
            .gate(&key)
            .await
            .expect_err("nothing listens on port 1");
        let rendered_error = format!("{error} / {error:?}");
        assert!(!rendered_error.contains("hunter2"), "{rendered_error}");
        let record_error = cadence
            .record_rate_limited(&key, None)
            .await
            .expect_err("nothing listens on port 1");
        let rendered_error = format!("{record_error} / {record_error:?}");
        assert!(!rendered_error.contains("hunter2"), "{rendered_error}");
    }

    #[tokio::test]
    async fn a_black_holed_server_errors_within_the_deadline_instead_of_hanging() {
        // A listener that completes the TCP handshake (the kernel does that from the backlog)
        // but never reads or answers: the shape of a firewall that drops the conversation.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        let port = listener.local_addr().expect("local addr").port();
        let cadence = RedisCadence::new(
            RedisCadenceConfig::new(format!("redis://127.0.0.1:{port}/0"))
                .with_response_timeout(Duration::from_millis(150))
                .with_connection_timeout(Duration::from_millis(150)),
            CadencePolicy::default(),
        )
        .expect("parsable");

        let started = Instant::now();
        let outcome = cadence.gate(&CadenceKey::new("openai", "gpt-4o")).await;
        assert!(
            matches!(
                outcome,
                Err(CadenceError::Timeout { .. } | CadenceError::Backend { .. })
            ),
            "a dead server must be an error, got {outcome:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "the call hung for {:?}",
            started.elapsed()
        );
        drop(listener);
    }
}
