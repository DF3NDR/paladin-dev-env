//! Configuration for a durable Waypoint storage backend (HITL-05, D-26,
//! X-09).
//!
//! Mirrors [`crate::config::waypoint_retention::WaypointRetentionConfig`]'s
//! shape field-for-field (`Default` + `validate()` + `EnvOverridable`).
//! Defaults to [`WaypointStoreBackend::Disabled`], so a v0.9 deployment
//! that never mentions this struct boots v0.10 with no waypoint backend
//! wired at all -- X-09's "new subsystems are disabled by default"
//! requirement. This default is load-bearing beyond configuration hygiene:
//! it is what makes every thread route in a later plan answer `501
//! not_implemented` (D-24) until an operator deliberately sets a backend,
//! following the same code-configured, off-by-default precedent Phase 23's
//! D-26 established.
//!
//! `Settings` (`src/config/settings.rs`, all-pub, not `#[non_exhaustive]`)
//! is never touched by this struct, following the Phase 22/23/24 precedent
//! (`EngineConfig`, `WaypointRetentionConfig`).

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Which durable Waypoint backend (if any) `paladin-server` wires (D-26).
///
/// The `postgres` variant carries the NAME of the environment variable
/// holding the connection url -- never the url itself -- so a connection
/// string (which may embed a password) never lands in a serialised config
/// payload or a `Debug`/log line of this struct. [`WaypointStoreConfig::validate`]
/// resolves the named variable at startup without ever storing its value on
/// this type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum WaypointStoreBackend {
    /// No durable Waypoint backend is wired (the default).
    Disabled,
    /// A SQLite-backed store at `path`.
    Sqlite {
        /// The SQLite database file path (or connection url, e.g.
        /// `sqlite://./data/waypoints.db`).
        path: String,
    },
    /// A Postgres-backed store, whose connection url is read at startup
    /// from the environment variable named `url_env`.
    Postgres {
        /// The NAME of the environment variable holding the Postgres
        /// connection url -- not the url itself.
        url_env: String,
    },
}

/// Configuration for a durable Waypoint storage backend (HITL-05, D-26,
/// X-09). See the module-level documentation for the disabled-by-default
/// contract and why `postgres` never carries a connection string inline.
///
/// # Examples
///
/// ```
/// use paladin::config::waypoint_store::{WaypointStoreBackend, WaypointStoreConfig};
///
/// let config = WaypointStoreConfig::default();
/// assert_eq!(config.backend, WaypointStoreBackend::Disabled);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointStoreConfig {
    /// The backend to wire, if any. Defaults to
    /// [`WaypointStoreBackend::Disabled`].
    pub backend: WaypointStoreBackend,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `WaypointRetentionConfig`'s and `EngineConfig`'s
// convention: the disabled-by-default contract is stated in code, not left
// implicit in a derive.
impl Default for WaypointStoreConfig {
    fn default() -> Self {
        Self {
            backend: WaypointStoreBackend::Disabled,
        }
    }
}

impl WaypointStoreConfig {
    /// Validates the waypoint store configuration.
    ///
    /// - [`WaypointStoreBackend::Disabled`] is always valid.
    /// - [`WaypointStoreBackend::Sqlite`] requires a non-empty `path`.
    /// - [`WaypointStoreBackend::Postgres`] requires a non-empty `url_env`
    ///   name, AND that the environment variable it names is currently set
    ///   -- an unresolvable env-var name is rejected here, at configuration
    ///   validation time, rather than surfacing later as an opaque
    ///   connection failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::waypoint_store::{WaypointStoreBackend, WaypointStoreConfig};
    ///
    /// let mut config = WaypointStoreConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.backend = WaypointStoreBackend::Sqlite { path: String::new() };
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        match &self.backend {
            WaypointStoreBackend::Disabled => Ok(()),
            WaypointStoreBackend::Sqlite { path } => {
                if path.trim().is_empty() {
                    return Err(
                        "waypoint store sqlite backend requires a non-empty path".to_string()
                    );
                }
                Ok(())
            }
            WaypointStoreBackend::Postgres { url_env } => {
                if url_env.trim().is_empty() {
                    return Err(
                        "waypoint store postgres backend requires a non-empty url_env name"
                            .to_string(),
                    );
                }
                if std::env::var(url_env).is_err() {
                    return Err(format!(
                        "waypoint store postgres backend names env var '{url_env}', which is not \
                         set"
                    ));
                }
                Ok(())
            }
        }
    }
}

impl EnvOverridable for WaypointStoreConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<String>("APP_WAYPOINT_STORE_BACKEND") {
            match v.to_ascii_lowercase().as_str() {
                "disabled" => self.backend = WaypointStoreBackend::Disabled,
                "sqlite" => {
                    let path = match &self.backend {
                        WaypointStoreBackend::Sqlite { path } => path.clone(),
                        _ => String::new(),
                    };
                    self.backend = WaypointStoreBackend::Sqlite { path };
                }
                "postgres" => {
                    let url_env = match &self.backend {
                        WaypointStoreBackend::Postgres { url_env } => url_env.clone(),
                        _ => String::new(),
                    };
                    self.backend = WaypointStoreBackend::Postgres { url_env };
                }
                // Unparseable: leave the field at its prior value, matching
                // read_env's own silently-swallowed-parse-error contract.
                _ => {}
            }
        }
        if let Some(v) = read_env::<String>("APP_WAYPOINT_STORE_SQLITE_PATH")
            && let WaypointStoreBackend::Sqlite { path } = &mut self.backend
        {
            *path = v;
        }
        if let Some(v) = read_env::<String>("APP_WAYPOINT_STORE_POSTGRES_URL_ENV")
            && let WaypointStoreBackend::Postgres { url_env } = &mut self.backend
        {
            *url_env = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default backend is disabled.
    #[test]
    fn waypoint_store_config_defaults_to_disabled() {
        let config = WaypointStoreConfig::default();
        assert_eq!(config.backend, WaypointStoreBackend::Disabled);
        assert!(config.validate().is_ok());
    }

    /// Test 2: the `APP_`-prefixed env vars select the sqlite and postgres
    /// backends and supply their parameters.
    #[test]
    #[serial]
    fn waypoint_store_config_reads_env_overrides() {
        unsafe {
            env::set_var("APP_WAYPOINT_STORE_BACKEND", "sqlite");
            env::set_var("APP_WAYPOINT_STORE_SQLITE_PATH", "sqlite://./data/wp.db");
        }
        let mut config = WaypointStoreConfig::default();
        config.apply_env_overrides();
        assert_eq!(
            config.backend,
            WaypointStoreBackend::Sqlite {
                path: "sqlite://./data/wp.db".to_string()
            }
        );
        unsafe {
            env::remove_var("APP_WAYPOINT_STORE_BACKEND");
            env::remove_var("APP_WAYPOINT_STORE_SQLITE_PATH");
        }

        unsafe {
            env::set_var("APP_WAYPOINT_STORE_BACKEND", "postgres");
            env::set_var(
                "APP_WAYPOINT_STORE_POSTGRES_URL_ENV",
                "WAYPOINT_DATABASE_URL",
            );
        }
        let mut config = WaypointStoreConfig::default();
        config.apply_env_overrides();
        assert_eq!(
            config.backend,
            WaypointStoreBackend::Postgres {
                url_env: "WAYPOINT_DATABASE_URL".to_string()
            }
        );
        unsafe {
            env::remove_var("APP_WAYPOINT_STORE_BACKEND");
            env::remove_var("APP_WAYPOINT_STORE_POSTGRES_URL_ENV");
        }

        unsafe {
            env::set_var("APP_WAYPOINT_STORE_BACKEND", "disabled");
        }
        let mut config = WaypointStoreConfig {
            backend: WaypointStoreBackend::Sqlite {
                path: "x".to_string(),
            },
        };
        config.apply_env_overrides();
        assert_eq!(config.backend, WaypointStoreBackend::Disabled);
        unsafe {
            env::remove_var("APP_WAYPOINT_STORE_BACKEND");
        }
    }

    /// Test 3: a sqlite backend with an empty path and a postgres backend
    /// with an unset url env name are rejected by `validate()`.
    #[test]
    #[serial]
    fn waypoint_store_config_validates_backend_parameters() {
        let config = WaypointStoreConfig {
            backend: WaypointStoreBackend::Sqlite {
                path: String::new(),
            },
        };
        assert!(config.validate().unwrap_err().contains("path"));

        let config = WaypointStoreConfig {
            backend: WaypointStoreBackend::Postgres {
                url_env: String::new(),
            },
        };
        assert!(config.validate().unwrap_err().contains("url_env"));

        // A NAMED but unset env var is also rejected.
        let unset_var = "APP_WAYPOINT_STORE_TEST_UNSET_VAR";
        unsafe {
            env::remove_var(unset_var);
        }
        let config = WaypointStoreConfig {
            backend: WaypointStoreBackend::Postgres {
                url_env: unset_var.to_string(),
            },
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains(unset_var));

        // Once the named var is actually set, validation passes.
        unsafe {
            env::set_var(unset_var, "postgres://example/db");
        }
        assert!(config.validate().is_ok());
        unsafe {
            env::remove_var(unset_var);
        }
    }

    /// Test 4: the postgres variant names the env var holding the
    /// connection url rather than carrying the url itself.
    #[test]
    fn waypoint_store_config_postgres_reads_url_from_env_name_not_inline() {
        let config = WaypointStoreConfig {
            backend: WaypointStoreBackend::Postgres {
                url_env: "WAYPOINT_DATABASE_URL".to_string(),
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("WAYPOINT_DATABASE_URL"));
        // The struct has no field a connection string (containing a
        // password, e.g. postgres://user:pass@host/db) could ever be
        // written into -- only the referring env var's NAME is stored.
        assert!(!json.contains("postgres://"));
    }
}
