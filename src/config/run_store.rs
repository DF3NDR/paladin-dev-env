//! Configuration for the durable Run storage backend (PLAT-01, D-50, X-09).
//!
//! Mirrors [`crate::config::waypoint_store::WaypointStoreConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). Defaults
//! to [`RunStoreBackend::Disabled`], so a v0.9 deployment that never
//! mentions this struct boots v0.10 with no run store wired at all --
//! X-09's "new subsystems are disabled by default" requirement. This
//! default is load-bearing beyond configuration hygiene: it is what makes
//! every run route answer `501 not_implemented` (D-44) until an operator
//! deliberately sets a backend, following the same code-configured,
//! off-by-default precedent Phase 24's D-26 established for
//! `WaypointStoreConfig`.
//!
//! `Settings` (`src/config/settings.rs`, all-pub, not `#[non_exhaustive]`)
//! is never touched by this struct, following the Phase 22/23/24/26
//! precedent (`EngineConfig`, `WaypointRetentionConfig`, `WaypointStoreConfig`).

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Which durable Run backend (if any) `paladin-server` wires (D-50).
///
/// The `postgres` variant carries the NAME of the environment variable
/// holding the connection url -- never the url itself -- so a connection
/// string (which may embed a password) never lands in a serialised config
/// payload or a `Debug`/log line of this struct. [`RunStoreConfig::validate`]
/// resolves the named variable at startup without ever storing its value on
/// this type (T-27-06-01).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum RunStoreBackend {
    /// No durable Run backend is wired (the default).
    Disabled,
    /// A SQLite-backed store at `path`.
    Sqlite {
        /// The SQLite database file path (or connection url, e.g.
        /// `sqlite://./data/runs.db`).
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

/// Configuration for the durable Run storage backend (PLAT-01, D-50, X-09).
/// See the module-level documentation for the disabled-by-default contract
/// and why `postgres` never carries a connection string inline.
///
/// # Examples
///
/// ```
/// use paladin::config::run_store::{RunStoreBackend, RunStoreConfig};
///
/// let config = RunStoreConfig::default();
/// assert_eq!(config.backend, RunStoreBackend::Disabled);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunStoreConfig {
    /// The backend to wire, if any. Defaults to
    /// [`RunStoreBackend::Disabled`].
    pub backend: RunStoreBackend,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `WaypointStoreConfig`'s convention: the disabled-by-
// default contract is stated in code, not left implicit in a derive.
impl Default for RunStoreConfig {
    fn default() -> Self {
        Self {
            backend: RunStoreBackend::Disabled,
        }
    }
}

impl RunStoreConfig {
    /// Validates the run store configuration.
    ///
    /// - [`RunStoreBackend::Disabled`] is always valid.
    /// - [`RunStoreBackend::Sqlite`] requires a non-empty `path`.
    /// - [`RunStoreBackend::Postgres`] requires a non-empty `url_env` name,
    ///   AND that the environment variable it names is currently set -- an
    ///   unresolvable env-var name is rejected here, at configuration
    ///   validation time, rather than surfacing later as an opaque
    ///   connection failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::run_store::{RunStoreBackend, RunStoreConfig};
    ///
    /// let mut config = RunStoreConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.backend = RunStoreBackend::Sqlite { path: String::new() };
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        match &self.backend {
            RunStoreBackend::Disabled => Ok(()),
            RunStoreBackend::Sqlite { path } => {
                if path.trim().is_empty() {
                    return Err("run store sqlite backend requires a non-empty path".to_string());
                }
                Ok(())
            }
            RunStoreBackend::Postgres { url_env } => {
                if url_env.trim().is_empty() {
                    return Err(
                        "run store postgres backend requires a non-empty url_env name".to_string(),
                    );
                }
                if std::env::var(url_env).is_err() {
                    return Err(format!(
                        "run store postgres backend names env var '{url_env}', which is not set"
                    ));
                }
                Ok(())
            }
        }
    }
}

impl EnvOverridable for RunStoreConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<String>("APP_RUN_STORE_BACKEND") {
            match v.to_ascii_lowercase().as_str() {
                "disabled" => self.backend = RunStoreBackend::Disabled,
                "sqlite" => {
                    let path = match &self.backend {
                        RunStoreBackend::Sqlite { path } => path.clone(),
                        _ => String::new(),
                    };
                    self.backend = RunStoreBackend::Sqlite { path };
                }
                "postgres" => {
                    let url_env = match &self.backend {
                        RunStoreBackend::Postgres { url_env } => url_env.clone(),
                        _ => String::new(),
                    };
                    self.backend = RunStoreBackend::Postgres { url_env };
                }
                // Unparseable: leave the field at its prior value, matching
                // read_env's own silently-swallowed-parse-error contract.
                _ => {}
            }
        }
        if let Some(v) = read_env::<String>("APP_RUN_STORE_PATH")
            && let RunStoreBackend::Sqlite { path } = &mut self.backend
        {
            *path = v;
        }
        if let Some(v) = read_env::<String>("APP_RUN_STORE_URL_ENV")
            && let RunStoreBackend::Postgres { url_env } = &mut self.backend
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
    fn default_is_disabled() {
        let config = RunStoreConfig::default();
        assert_eq!(config.backend, RunStoreBackend::Disabled);
        assert!(config.validate().is_ok());
    }

    /// Test 2: the `APP_`-prefixed env vars select the sqlite and postgres
    /// backends and supply their parameters.
    #[test]
    #[serial]
    fn env_overrides_select_sqlite_and_postgres() {
        unsafe {
            env::set_var("APP_RUN_STORE_BACKEND", "sqlite");
            env::set_var("APP_RUN_STORE_PATH", "sqlite://./data/runs.db");
        }
        let mut config = RunStoreConfig::default();
        config.apply_env_overrides();
        assert_eq!(
            config.backend,
            RunStoreBackend::Sqlite {
                path: "sqlite://./data/runs.db".to_string()
            }
        );
        unsafe {
            env::remove_var("APP_RUN_STORE_BACKEND");
            env::remove_var("APP_RUN_STORE_PATH");
        }

        unsafe {
            env::set_var("APP_RUN_STORE_BACKEND", "postgres");
            env::set_var("APP_RUN_STORE_URL_ENV", "RUN_DATABASE_URL");
        }
        let mut config = RunStoreConfig::default();
        config.apply_env_overrides();
        assert_eq!(
            config.backend,
            RunStoreBackend::Postgres {
                url_env: "RUN_DATABASE_URL".to_string()
            }
        );
        unsafe {
            env::remove_var("APP_RUN_STORE_BACKEND");
            env::remove_var("APP_RUN_STORE_URL_ENV");
        }

        unsafe {
            env::set_var("APP_RUN_STORE_BACKEND", "disabled");
        }
        let mut config = RunStoreConfig {
            backend: RunStoreBackend::Sqlite {
                path: "x".to_string(),
            },
        };
        config.apply_env_overrides();
        assert_eq!(config.backend, RunStoreBackend::Disabled);
        unsafe {
            env::remove_var("APP_RUN_STORE_BACKEND");
        }
    }

    /// Test 3: a sqlite backend with an empty path and a postgres backend
    /// with an unset url env name are rejected by `validate()`.
    #[test]
    #[serial]
    fn validate_rejects_empty_path_and_unset_url_env() {
        let config = RunStoreConfig {
            backend: RunStoreBackend::Sqlite {
                path: String::new(),
            },
        };
        assert!(config.validate().unwrap_err().contains("path"));

        let config = RunStoreConfig {
            backend: RunStoreBackend::Postgres {
                url_env: String::new(),
            },
        };
        assert!(config.validate().unwrap_err().contains("url_env"));

        // A NAMED but unset env var is also rejected.
        let unset_var = "APP_RUN_STORE_TEST_UNSET_VAR";
        unsafe {
            env::remove_var(unset_var);
        }
        let config = RunStoreConfig {
            backend: RunStoreBackend::Postgres {
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

    /// Test 4: a document omitting the `run_store` section deserialises to
    /// the disabled default (`#[serde(default)]` on the containing field in
    /// `Settings`; here we assert the struct's own `Default` matches what
    /// such a document would produce).
    #[test]
    fn absent_section_deserializes_to_default() {
        let config: RunStoreConfig =
            serde_json::from_value(serde_json::json!({ "backend": { "backend": "disabled" } }))
                .unwrap();
        assert_eq!(config, RunStoreConfig::default());
    }

    /// Test 5: the postgres variant names the env var holding the
    /// connection url rather than carrying the url itself.
    #[test]
    fn postgres_reads_url_from_env_name_not_inline() {
        let config = RunStoreConfig {
            backend: RunStoreBackend::Postgres {
                url_env: "RUN_DATABASE_URL".to_string(),
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("RUN_DATABASE_URL"));
        // The struct has no field a connection string (containing a
        // password, e.g. postgres://user:pass@host/db) could ever be
        // written into -- only the referring env var's NAME is stored.
        assert!(!json.contains("postgres://"));
    }
}
