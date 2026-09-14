//! Configuration for the per-node result cache backend (Doc 04 FT-FR-18…20,
//! D-29, X-09).
//!
//! Mirrors [`crate::config::waypoint_store::WaypointStoreConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). Defaults
//! to `enabled: false` with the [`NodeCacheBackend::InMemory`] backend, so a
//! v0.9 deployment that never mentions this struct boots v0.10 with
//! identical behavior -- X-09's "new subsystems are disabled by default"
//! requirement.
//!
//! This config selects the cache **backend** only. Per-node retry, timeout,
//! handler and cache POLICIES (`paladin_core::platform::container::aegis`)
//! are code, like `DirectiveParser`/`on_parse_error` (Phase 23 D-26 / §9.5's
//! rule for per-node enums, D-29) -- no Aegis policy env var or config field
//! is introduced here.
//!
//! `redis_password` never appears in a `Debug` rendering (T-25-18): this
//! struct implements `Debug` by hand rather than deriving it, because no
//! sibling config in this codebase has an existing redaction convention to
//! follow for a raw secret field.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Which node-cache backend (if any) is wired (D-29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCacheBackend {
    /// The ungated, always-available in-memory backend (the default).
    InMemory,
    /// The Redis-backed durable backend, behind the `redis-cache` feature.
    Redis,
}

/// Configuration for the per-node result cache backend (Doc 04 FT-FR-18…20,
/// D-29, X-09). See the module-level documentation for the disabled-by-
/// default contract and why `redis_password` is never `Debug`-printed.
///
/// # Examples
///
/// ```
/// use paladin::config::node_cache::NodeCacheConfig;
///
/// let config = NodeCacheConfig::default();
/// assert!(!config.enabled);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeCacheConfig {
    /// Whether the node cache is wired at all. `false` out of the box -- no
    /// v0.9 deployment gains new caching behavior on upgrade.
    pub enabled: bool,
    /// The backend to wire, if `enabled`. Defaults to
    /// [`NodeCacheBackend::InMemory`].
    pub backend: NodeCacheBackend,
    /// The Redis server hostname (only meaningful when `backend` is
    /// [`NodeCacheBackend::Redis`]).
    pub redis_host: String,
    /// The Redis server port.
    pub redis_port: u16,
    /// The Redis server password, if authentication is required. Never
    /// rendered by this struct's `Debug` implementation.
    pub redis_password: Option<String>,
    /// The Redis logical database index.
    pub redis_db: u8,
    /// The key namespace every entry the cache writes is prefixed with.
    pub key_prefix: String,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `WaypointStoreConfig`'s and `WaypointRetentionConfig`'s
// convention: the disabled-by-default contract is stated in code, not left
// implicit in a derive.
impl Default for NodeCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: NodeCacheBackend::InMemory,
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            redis_password: None,
            redis_db: 0,
            key_prefix: "paladin:node_cache".to_string(),
        }
    }
}

// Manual `Debug`: `redis_password` must never appear in a `Debug` rendering
// (T-25-18) -- rendered as a fixed placeholder when set, following the
// intent of the sibling `WaypointStoreConfig::Postgres`'s "never carry the
// secret inline" precedent, applied here via a redacted `Debug` output
// rather than an env-var-name indirection (this config's `redis_password`
// is itself already sourced from an env var via `EnvOverridable`, not from
// a config file).
impl std::fmt::Debug for NodeCacheConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeCacheConfig")
            .field("enabled", &self.enabled)
            .field("backend", &self.backend)
            .field("redis_host", &self.redis_host)
            .field("redis_port", &self.redis_port)
            .field(
                "redis_password",
                &self.redis_password.as_ref().map(|_| "[REDACTED]"),
            )
            .field("redis_db", &self.redis_db)
            .field("key_prefix", &self.key_prefix)
            .finish()
    }
}

impl NodeCacheConfig {
    /// Validates the node cache configuration.
    ///
    /// - `key_prefix` must be non-empty.
    /// - `redis_port` must be non-zero.
    /// - [`NodeCacheBackend::Redis`] requires a non-empty `redis_host`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::node_cache::NodeCacheConfig;
    ///
    /// let mut config = NodeCacheConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.key_prefix = String::new();
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.key_prefix.trim().is_empty() {
            return Err("node cache key_prefix must not be empty".to_string());
        }
        if self.redis_port == 0 {
            return Err("node cache redis_port must be greater than 0".to_string());
        }
        if self.backend == NodeCacheBackend::Redis && self.redis_host.trim().is_empty() {
            return Err("node cache redis backend requires a non-empty redis_host".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for NodeCacheConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_NODE_CACHE_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<String>("APP_NODE_CACHE_BACKEND") {
            match v.to_ascii_lowercase().as_str() {
                "in_memory" | "inmemory" => self.backend = NodeCacheBackend::InMemory,
                "redis" => self.backend = NodeCacheBackend::Redis,
                // Unparseable: leave the field at its prior value, matching
                // read_env's own silently-swallowed-parse-error contract.
                _ => {}
            }
        }
        if let Some(v) = read_env::<String>("APP_NODE_CACHE_REDIS_HOST") {
            self.redis_host = v;
        }
        if let Some(v) = read_env::<u16>("APP_NODE_CACHE_REDIS_PORT") {
            self.redis_port = v;
        }
        if let Some(v) = read_env::<String>("APP_NODE_CACHE_REDIS_PASSWORD") {
            self.redis_password = Some(v);
        }
        if let Some(v) = read_env::<u8>("APP_NODE_CACHE_REDIS_DB") {
            self.redis_db = v;
        }
        if let Some(v) = read_env::<String>("APP_NODE_CACHE_KEY_PREFIX") {
            self.key_prefix = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default node cache config is disabled, with the
    /// in-memory backend, and validates cleanly.
    #[test]
    fn default_node_cache_config_is_disabled() {
        let config = NodeCacheConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.backend, NodeCacheBackend::InMemory);
        assert!(config.validate().is_ok());
    }

    /// Test 2: each `APP_NODE_CACHE_*` variable overrides its field.
    #[test]
    #[serial]
    fn env_overrides_apply_for_every_field() {
        unsafe {
            env::set_var("APP_NODE_CACHE_ENABLED", "true");
            env::set_var("APP_NODE_CACHE_BACKEND", "redis");
            env::set_var("APP_NODE_CACHE_REDIS_HOST", "cache.example.internal");
            env::set_var("APP_NODE_CACHE_REDIS_PORT", "6390");
            env::set_var("APP_NODE_CACHE_REDIS_PASSWORD", "s3cr3t");
            env::set_var("APP_NODE_CACHE_REDIS_DB", "2");
            env::set_var("APP_NODE_CACHE_KEY_PREFIX", "custom:node_cache");
        }

        let mut config = NodeCacheConfig::default();
        config.apply_env_overrides();

        assert!(config.enabled);
        assert_eq!(config.backend, NodeCacheBackend::Redis);
        assert_eq!(config.redis_host, "cache.example.internal");
        assert_eq!(config.redis_port, 6390);
        assert_eq!(config.redis_password, Some("s3cr3t".to_string()));
        assert_eq!(config.redis_db, 2);
        assert_eq!(config.key_prefix, "custom:node_cache");

        unsafe {
            env::remove_var("APP_NODE_CACHE_ENABLED");
            env::remove_var("APP_NODE_CACHE_BACKEND");
            env::remove_var("APP_NODE_CACHE_REDIS_HOST");
            env::remove_var("APP_NODE_CACHE_REDIS_PORT");
            env::remove_var("APP_NODE_CACHE_REDIS_PASSWORD");
            env::remove_var("APP_NODE_CACHE_REDIS_DB");
            env::remove_var("APP_NODE_CACHE_KEY_PREFIX");
        }
    }

    /// Test 3: an empty `key_prefix` and a zero `redis_port` are both
    /// rejected by `validate()`, each naming the offending field.
    #[test]
    fn validate_rejects_an_empty_key_prefix_and_a_zero_port() {
        let mut config = NodeCacheConfig {
            key_prefix: String::new(),
            ..NodeCacheConfig::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("key_prefix"));

        config = NodeCacheConfig {
            redis_port: 0,
            ..NodeCacheConfig::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("redis_port"));
    }

    /// Test 4: selecting the Redis backend with no host is a validation
    /// error rather than a runtime surprise.
    #[test]
    fn redis_backend_without_a_host_fails_validation() {
        let config = NodeCacheConfig {
            backend: NodeCacheBackend::Redis,
            redis_host: String::new(),
            ..NodeCacheConfig::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("redis_host"));

        let config = NodeCacheConfig {
            backend: NodeCacheBackend::Redis,
            redis_host: "cache.example.internal".to_string(),
            ..NodeCacheConfig::default()
        };
        assert!(config.validate().is_ok());
    }

    /// `redis_password`, when set, must never appear in this struct's
    /// `Debug` rendering (T-25-18).
    #[test]
    fn debug_rendering_never_prints_the_password() {
        let config = NodeCacheConfig {
            redis_password: Some("super-secret-value".to_string()),
            ..NodeCacheConfig::default()
        };
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("super-secret-value"));
        assert!(rendered.contains("REDACTED"));
    }
}
