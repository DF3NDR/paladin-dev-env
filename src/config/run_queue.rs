//! Configuration for the Run dispatch queue backend (PLAT-01, D-50, X-09).
//!
//! Mirrors [`crate::config::run_store::RunStoreConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). Defaults
//! to [`RunQueueBackend::InMemory`], so a v0.9 deployment that never
//! mentions this struct boots v0.10 with an in-process queue only --
//! X-09's "new subsystems default to today's behaviour" requirement.
//!
//! `Settings` (`src/config/settings.rs`) is never touched by this struct,
//! following the Phase 22/23/24/26 precedent.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// The default key namespace every entry the run queue writes is prefixed
/// with, when the [`RunQueueBackend::Redis`] variant is selected without an
/// explicit `key_prefix`.
const DEFAULT_KEY_PREFIX: &str = "paladin:run_queue";

/// Which Run dispatch queue backend is wired (D-50).
///
/// The `redis` variant carries the NAME of the environment variable holding
/// the connection url -- never the url itself -- so a connection string
/// (which may embed a password) never lands in a serialised config payload
/// or a `Debug`/log line of this struct. [`RunQueueConfig::validate`]
/// resolves the named variable at startup without ever storing its value on
/// this type (T-27-06-01), the same pattern as
/// [`crate::config::run_store::RunStoreBackend::Postgres`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum RunQueueBackend {
    /// The ungated, always-available in-process queue (the default).
    InMemory,
    /// A Redis-backed durable queue, whose connection url is read at
    /// startup from the environment variable named `url_env`.
    Redis {
        /// The NAME of the environment variable holding the Redis
        /// connection url -- not the url itself.
        url_env: String,
        /// The key namespace every entry the queue writes is prefixed
        /// with. Defaults to `paladin:run_queue`.
        key_prefix: String,
    },
}

/// Configuration for the Run dispatch queue backend (PLAT-01, D-50, X-09).
/// See the module-level documentation for the today's-behaviour-by-default
/// contract and why `redis` never carries a connection string inline.
///
/// # Examples
///
/// ```
/// use paladin::config::run_queue::{RunQueueBackend, RunQueueConfig};
///
/// let config = RunQueueConfig::default();
/// assert_eq!(config.backend, RunQueueBackend::InMemory);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunQueueConfig {
    /// The backend to wire. Defaults to [`RunQueueBackend::InMemory`].
    pub backend: RunQueueBackend,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `RunStoreConfig`'s convention: the today's-behaviour
// default is stated in code, not left implicit in a derive.
impl Default for RunQueueConfig {
    fn default() -> Self {
        Self {
            backend: RunQueueBackend::InMemory,
        }
    }
}

impl RunQueueConfig {
    /// Validates the run queue configuration.
    ///
    /// - [`RunQueueBackend::InMemory`] is always valid.
    /// - [`RunQueueBackend::Redis`] requires a non-empty `url_env` name,
    ///   AND that the environment variable it names is currently set -- an
    ///   unresolvable env-var name is rejected here, at configuration
    ///   validation time, rather than surfacing later as an opaque
    ///   connection failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::run_queue::{RunQueueBackend, RunQueueConfig};
    ///
    /// let mut config = RunQueueConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.backend = RunQueueBackend::Redis {
    ///     url_env: String::new(),
    ///     key_prefix: "paladin:run_queue".to_string(),
    /// };
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        match &self.backend {
            RunQueueBackend::InMemory => Ok(()),
            RunQueueBackend::Redis { url_env, .. } => {
                if url_env.trim().is_empty() {
                    return Err(
                        "run queue redis backend requires a non-empty url_env name".to_string()
                    );
                }
                if std::env::var(url_env).is_err() {
                    return Err(format!(
                        "run queue redis backend names env var '{url_env}', which is not set"
                    ));
                }
                Ok(())
            }
        }
    }
}

impl EnvOverridable for RunQueueConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<String>("APP_RUN_QUEUE_BACKEND") {
            match v.to_ascii_lowercase().as_str() {
                "in_memory" | "inmemory" => self.backend = RunQueueBackend::InMemory,
                "redis" => {
                    let (url_env, key_prefix) = match &self.backend {
                        RunQueueBackend::Redis {
                            url_env,
                            key_prefix,
                        } => (url_env.clone(), key_prefix.clone()),
                        _ => (String::new(), DEFAULT_KEY_PREFIX.to_string()),
                    };
                    self.backend = RunQueueBackend::Redis {
                        url_env,
                        key_prefix,
                    };
                }
                // Unparseable: leave the field at its prior value, matching
                // read_env's own silently-swallowed-parse-error contract.
                _ => {}
            }
        }
        if let Some(v) = read_env::<String>("APP_RUN_QUEUE_URL_ENV")
            && let RunQueueBackend::Redis { url_env, .. } = &mut self.backend
        {
            *url_env = v;
        }
        if let Some(v) = read_env::<String>("APP_RUN_QUEUE_KEY_PREFIX")
            && let RunQueueBackend::Redis { key_prefix, .. } = &mut self.backend
        {
            *key_prefix = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default backend is in-memory.
    #[test]
    fn default_is_in_memory() {
        let config = RunQueueConfig::default();
        assert_eq!(config.backend, RunQueueBackend::InMemory);
        assert!(config.validate().is_ok());
    }

    /// Test 2: the `APP_`-prefixed env vars select the redis backend and
    /// supply its parameters, including the default key prefix.
    #[test]
    #[serial]
    fn env_overrides_select_redis_with_default_key_prefix() {
        unsafe {
            env::set_var("APP_RUN_QUEUE_BACKEND", "redis");
            env::set_var("APP_RUN_QUEUE_URL_ENV", "RUN_QUEUE_REDIS_URL");
        }
        let mut config = RunQueueConfig::default();
        config.apply_env_overrides();
        assert_eq!(
            config.backend,
            RunQueueBackend::Redis {
                url_env: "RUN_QUEUE_REDIS_URL".to_string(),
                key_prefix: DEFAULT_KEY_PREFIX.to_string(),
            }
        );
        unsafe {
            env::remove_var("APP_RUN_QUEUE_BACKEND");
            env::remove_var("APP_RUN_QUEUE_URL_ENV");
        }
    }

    /// Test 3: `APP_RUN_QUEUE_KEY_PREFIX` overrides the prefix once the
    /// redis backend is selected.
    #[test]
    #[serial]
    fn env_override_sets_key_prefix() {
        unsafe {
            env::set_var("APP_RUN_QUEUE_BACKEND", "redis");
            env::set_var("APP_RUN_QUEUE_URL_ENV", "RUN_QUEUE_REDIS_URL");
            env::set_var("APP_RUN_QUEUE_KEY_PREFIX", "custom:run_queue");
        }
        let mut config = RunQueueConfig::default();
        config.apply_env_overrides();
        assert_eq!(
            config.backend,
            RunQueueBackend::Redis {
                url_env: "RUN_QUEUE_REDIS_URL".to_string(),
                key_prefix: "custom:run_queue".to_string(),
            }
        );
        unsafe {
            env::remove_var("APP_RUN_QUEUE_BACKEND");
            env::remove_var("APP_RUN_QUEUE_URL_ENV");
            env::remove_var("APP_RUN_QUEUE_KEY_PREFIX");
        }
    }

    /// Test 4: a redis backend with an unset `url_env` name is rejected by
    /// `validate()`, and passes once the named var is actually set.
    #[test]
    #[serial]
    fn validate_rejects_unset_url_env() {
        let config = RunQueueConfig {
            backend: RunQueueBackend::Redis {
                url_env: String::new(),
                key_prefix: DEFAULT_KEY_PREFIX.to_string(),
            },
        };
        assert!(config.validate().unwrap_err().contains("url_env"));

        let unset_var = "APP_RUN_QUEUE_TEST_UNSET_VAR";
        unsafe {
            env::remove_var(unset_var);
        }
        let config = RunQueueConfig {
            backend: RunQueueBackend::Redis {
                url_env: unset_var.to_string(),
                key_prefix: DEFAULT_KEY_PREFIX.to_string(),
            },
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains(unset_var));

        unsafe {
            env::set_var(unset_var, "redis://example:6379");
        }
        assert!(config.validate().is_ok());
        unsafe {
            env::remove_var(unset_var);
        }
    }

    /// Test 5: a document omitting the `run_queue` section deserialises to
    /// the in-memory default.
    #[test]
    fn absent_section_deserializes_to_default() {
        let config: RunQueueConfig =
            serde_json::from_value(serde_json::json!({ "backend": { "backend": "in_memory" } }))
                .unwrap();
        assert_eq!(config, RunQueueConfig::default());
    }

    /// Test 6: the redis variant names the env var holding the connection
    /// url rather than carrying the url itself.
    #[test]
    fn redis_reads_url_from_env_name_not_inline() {
        let config = RunQueueConfig {
            backend: RunQueueBackend::Redis {
                url_env: "RUN_QUEUE_REDIS_URL".to_string(),
                key_prefix: DEFAULT_KEY_PREFIX.to_string(),
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("RUN_QUEUE_REDIS_URL"));
        assert!(!json.contains("redis://"));
    }
}
