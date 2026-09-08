//! Configuration for the Run schedules subsystem (PLAT-05, D-36, D-37,
//! D-50, X-09).
//!
//! Mirrors [`crate::config::run_worker::RunWorkerConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). Defaults
//! to `enabled: false`, so a v0.9 deployment that never mentions this
//! struct boots v0.10 with the `ScheduleService` tick loop never started at
//! all -- X-09's "new subsystems are disabled by default" requirement. The
//! persisted `ScheduleRepositoryPort` schema and rows are unaffected by this
//! flag; only the facade's periodic claim-and-fire loop (D-37) is gated.
//!
//! `Settings` (`src/config/settings.rs`) is never touched by this struct,
//! following the Phase 22/23/24/26 precedent.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Configuration for the Run schedules subsystem (PLAT-05, D-36, D-37,
/// D-50, X-09). See the module-level documentation for the disabled-by-
/// default contract.
///
/// # Examples
///
/// ```
/// use paladin::config::schedules::SchedulesConfig;
///
/// let config = SchedulesConfig::default();
/// assert!(!config.enabled);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulesConfig {
    /// Whether the `ScheduleService` tick loop runs at all. `false` out of
    /// the box -- no v0.9 deployment gains scheduled runs on upgrade.
    pub enabled: bool,
    /// How often, in milliseconds, `ScheduleService` polls for schedules
    /// whose `next_tick` has elapsed (D-37). Defaults to `1000`.
    pub tick_interval_ms: u64,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `RunWorkerConfig`'s convention: the disabled-by-default
// contract is stated in code, not left implicit in a derive.
impl Default for SchedulesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tick_interval_ms: 1000,
        }
    }
}

impl SchedulesConfig {
    /// Validates the schedules configuration.
    ///
    /// - `tick_interval_ms` must be non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::schedules::SchedulesConfig;
    ///
    /// let mut config = SchedulesConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.tick_interval_ms = 0;
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.tick_interval_ms == 0 {
            return Err("schedules tick_interval_ms must be greater than 0".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for SchedulesConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_SCHEDULES_ENABLED") {
            self.enabled = v;
        }
        if let Some(v) = read_env::<u64>("APP_SCHEDULES_TICK_INTERVAL_MS") {
            self.tick_interval_ms = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default config is disabled with a 1 s tick interval and
    /// validates cleanly.
    #[test]
    fn default_is_disabled() {
        let config = SchedulesConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.tick_interval_ms, 1000);
        assert!(config.validate().is_ok());
    }

    /// Test 2: `APP_SCHEDULES_*` variables override their fields.
    #[test]
    #[serial]
    fn env_overrides_apply_for_every_field() {
        unsafe {
            env::set_var("APP_SCHEDULES_ENABLED", "true");
            env::set_var("APP_SCHEDULES_TICK_INTERVAL_MS", "2500");
        }

        let mut config = SchedulesConfig::default();
        config.apply_env_overrides();

        assert!(config.enabled);
        assert_eq!(config.tick_interval_ms, 2500);

        unsafe {
            env::remove_var("APP_SCHEDULES_ENABLED");
            env::remove_var("APP_SCHEDULES_TICK_INTERVAL_MS");
        }
    }

    /// Test 3: a zero tick interval is rejected by `validate()`.
    #[test]
    fn validate_rejects_zero_tick_interval() {
        let config = SchedulesConfig {
            tick_interval_ms: 0,
            ..SchedulesConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("tick_interval_ms"));
    }

    /// Test 4: a document omitting the `schedules` section deserialises to
    /// the disabled default.
    #[test]
    fn absent_section_deserializes_to_default() {
        let config: SchedulesConfig = serde_json::from_value(serde_json::json!({
            "enabled": false,
            "tick_interval_ms": 1000
        }))
        .unwrap();
        assert_eq!(config, SchedulesConfig::default());
    }
}
