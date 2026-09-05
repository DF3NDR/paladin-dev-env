//! RED state (Phase 24 Plan 10, Task 3): `WaypointStoreConfig` and
//! `WaypointStoreBackend` do not exist yet. See the GREEN commit that
//! follows for the full implementation and rustdoc.

use crate::config::env_utils::{EnvOverridable, read_env};

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
            env::set_var("APP_WAYPOINT_STORE_POSTGRES_URL_ENV", "WAYPOINT_DATABASE_URL");
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
