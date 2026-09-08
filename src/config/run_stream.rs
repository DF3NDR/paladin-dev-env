//! Configuration for the Run event stream's degraded-mode polling interval
//! (PLAT-03, D-26, D-50, X-09).
//!
//! Mirrors [`crate::config::run_worker::RunWorkerConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). There is
//! no backend to select -- `poll_interval_ms` only governs how often the
//! degraded path (D-26: the run is executing on another instance, or is
//! already terminal) polls `WaypointPort::history` to synthesize stream
//! events, so a v0.9 deployment that never mentions this struct boots
//! v0.10 with the documented default below (X-09).
//!
//! `Settings` (`src/config/settings.rs`) is never touched by this struct,
//! following the Phase 22/23/24/26 precedent.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Configuration for the Run event stream's degraded-mode polling interval
/// (PLAT-03, D-26, D-50, X-09). See the module-level documentation for the
/// degraded-path polling contract.
///
/// # Examples
///
/// ```
/// use paladin::config::run_stream::RunStreamConfig;
///
/// let config = RunStreamConfig::default();
/// assert_eq!(config.poll_interval_ms, 1000);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStreamConfig {
    /// How often, in milliseconds, the degraded stream path polls
    /// `WaypointPort::history` for newly persisted Waypoints (D-26).
    /// Defaults to `1000`.
    pub poll_interval_ms: u64,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `RunWorkerConfig`'s convention: the today's-behaviour
// default is stated in code, not left implicit in a derive.
impl Default for RunStreamConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
        }
    }
}

impl RunStreamConfig {
    /// Validates the run stream configuration.
    ///
    /// - `poll_interval_ms` must be non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::run_stream::RunStreamConfig;
    ///
    /// let mut config = RunStreamConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.poll_interval_ms = 0;
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.poll_interval_ms == 0 {
            return Err("run stream poll_interval_ms must be greater than 0".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for RunStreamConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<u64>("APP_RUN_STREAM_POLL_INTERVAL_MS") {
            self.poll_interval_ms = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default poll interval matches D-26's documented value
    /// and validates cleanly.
    #[test]
    fn default_matches_documented_value() {
        let config = RunStreamConfig::default();
        assert_eq!(config.poll_interval_ms, 1000);
        assert!(config.validate().is_ok());
    }

    /// Test 2: `APP_RUN_STREAM_POLL_INTERVAL_MS` overrides the field.
    #[test]
    #[serial]
    fn env_override_applies() {
        unsafe {
            env::set_var("APP_RUN_STREAM_POLL_INTERVAL_MS", "2500");
        }

        let mut config = RunStreamConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.poll_interval_ms, 2500);

        unsafe {
            env::remove_var("APP_RUN_STREAM_POLL_INTERVAL_MS");
        }
    }

    /// Test 3: a zero poll interval is rejected by `validate()`.
    #[test]
    fn validate_rejects_zero_poll_interval() {
        let config = RunStreamConfig {
            poll_interval_ms: 0,
        };
        assert!(config.validate().unwrap_err().contains("poll_interval_ms"));
    }

    /// Test 4: a document omitting the `run_stream` section deserialises to
    /// the documented default.
    #[test]
    fn absent_section_deserializes_to_default() {
        let config: RunStreamConfig =
            serde_json::from_value(serde_json::json!({ "poll_interval_ms": 1000 })).unwrap();
        assert_eq!(config, RunStreamConfig::default());
    }
}
