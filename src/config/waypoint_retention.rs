//! Configuration for the Waypoint retention/cleanup routine (ENG-FR-18).

use crate::config::env_utils::{EnvOverridable, read_env};
use serde::{Deserialize, Serialize};

/// Configuration for the Waypoint retention routine, driven through
/// [`crate::application::services::waypoint_retention::WaypointRetentionService`]
/// (the entry point -- it owns the one definition of "protected" and wires
/// this config into `paladin_storage::waypoint::retention::prune`, which
/// this config's fields no longer address directly).
///
/// Mirrors [`crate::config::citadel::CitadelConfig`]'s shape field-for-field
/// (`Default` + `validate()` + `EnvOverridable`). `enabled` defaults to
/// `false` and both bounds default to `None`, so an existing v0.9
/// deployment that never mentions this struct boots v0.10 with identical
/// (no pruning) behavior — X-09's "new subsystems are disabled by default"
/// requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointRetentionConfig {
    /// Whether the retention routine runs at all. `false` out of the box —
    /// no v0.9 deployment gains new deletion behavior on upgrade.
    pub enabled: bool,
    /// Delete Waypoints older than this many days (subject to the hard
    /// exclusions documented on `prune`: never a thread's latest, never
    /// `AwaitingInput`). `None` means no age-based bound.
    pub max_age_days: Option<u32>,
    /// Keep at most this many Waypoints per thread (subject to the same
    /// hard exclusions). `None` means no count-based bound.
    pub max_waypoints_per_thread: Option<u32>,
}

// A manual impl (not #[derive(Default)]) even though every field's value
// below happens to equal its own `Default::default()` -- deliberately
// explicit and colocated with `validate()`'s zero-checks, mirroring
// `CitadelConfig`'s convention field-for-field so the disabled-by-default
// contract (X-09) is stated in code, not left implicit in a derive.
#[allow(clippy::derivable_impls)]
impl Default for WaypointRetentionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_age_days: None,
            max_waypoints_per_thread: None,
        }
    }
}

impl WaypointRetentionConfig {
    /// Validates the retention configuration. A bound of `Some(0)` is
    /// rejected for either field: "keep zero" / "expire after zero days" is
    /// never a sensible retention policy and would make every non-latest
    /// Waypoint a deletion candidate on the very first prune run.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_waypoints_per_thread == Some(0) {
            return Err("max_waypoints_per_thread must be greater than 0 when set".to_string());
        }
        if self.max_age_days == Some(0) {
            return Err("max_age_days must be greater than 0 when set".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for WaypointRetentionConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_WAYPOINT_RETENTION_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u32>("APP_WAYPOINT_RETENTION_MAX_AGE_DAYS") {
            self.max_age_days = Some(v);
        }
        if let Some(v) = read_env::<u32>("APP_WAYPOINT_RETENTION_MAX_WAYPOINTS_PER_THREAD") {
            self.max_waypoints_per_thread = Some(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    fn test_waypoint_retention_config_default() {
        let config = WaypointRetentionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_age_days, None);
        assert_eq!(config.max_waypoints_per_thread, None);
    }

    #[test]
    fn test_waypoint_retention_config_validate_valid() {
        let config = WaypointRetentionConfig {
            enabled: true,
            max_age_days: Some(30),
            max_waypoints_per_thread: Some(50),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_waypoint_retention_config_validate_none_bounds_is_valid() {
        // Both bounds unset is a valid (no-op) configuration.
        let config = WaypointRetentionConfig {
            enabled: true,
            max_age_days: None,
            max_waypoints_per_thread: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_waypoint_retention_config_validate_rejects_zero_max_waypoints_per_thread() {
        let config = WaypointRetentionConfig {
            enabled: true,
            max_age_days: None,
            max_waypoints_per_thread: Some(0),
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("max_waypoints_per_thread must be greater than 0")
        );
    }

    #[test]
    fn test_waypoint_retention_config_validate_rejects_zero_max_age_days() {
        let config = WaypointRetentionConfig {
            enabled: true,
            max_age_days: Some(0),
            max_waypoints_per_thread: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("max_age_days must be greater than 0")
        );
    }

    #[test]
    #[serial]
    fn test_waypoint_retention_config_env_override() {
        unsafe {
            env::set_var("APP_WAYPOINT_RETENTION_ENABLED", "true");
            env::set_var("APP_WAYPOINT_RETENTION_MAX_AGE_DAYS", "45");
            env::set_var("APP_WAYPOINT_RETENTION_MAX_WAYPOINTS_PER_THREAD", "20");
        }

        let mut config = WaypointRetentionConfig::default();
        config.apply_env_overrides();

        assert!(config.enabled);
        assert_eq!(config.max_age_days, Some(45));
        assert_eq!(config.max_waypoints_per_thread, Some(20));

        unsafe {
            env::remove_var("APP_WAYPOINT_RETENTION_ENABLED");
            env::remove_var("APP_WAYPOINT_RETENTION_MAX_AGE_DAYS");
            env::remove_var("APP_WAYPOINT_RETENTION_MAX_WAYPOINTS_PER_THREAD");
        }
    }

    #[test]
    fn test_waypoint_retention_config_deserialization_from_yml() {
        let config = WaypointRetentionConfig {
            enabled: true,
            max_age_days: Some(14),
            max_waypoints_per_thread: Some(10),
        };

        assert!(config.enabled);
        assert_eq!(config.max_age_days, Some(14));
        assert_eq!(config.max_waypoints_per_thread, Some(10));
    }
}
