//! Configuration for the Run worker pool (PLAT-01, PLAT-02, D-10, D-15,
//! D-50, X-09).
//!
//! Mirrors [`crate::config::run_store::RunStoreConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`), but has no
//! backend to select -- every field is a today's-behaviour-shaped tuning
//! knob, so a v0.9 deployment that never mentions this struct boots v0.10
//! with the documented defaults below (X-09).
//!
//! There is deliberately **no** `heartbeat` field on this struct (D-10):
//! PLAT-FR-02 asks for a heartbeat at "≤ lease/3", and a quarter
//! (`lease_seconds / 4`) leaves one whole missed heartbeat of slack before
//! expiry. Exposing both `lease_seconds` and a separately configurable
//! heartbeat is a configuration footgun -- two knobs that must be kept in a
//! ratio -- so the worker derives the heartbeat from `lease_seconds` at
//! runtime instead. [`RunWorkerConfig::validate`] enforces `lease_seconds >=
//! 4` so the derived heartbeat is never less than one whole second.
//!
//! `min_probe_interval_ms` (D-15) is the facade cancellation-probe adapter's
//! debounce window, not an engine setting -- the engine itself calls the
//! probe once per superstep boundary, unconditionally.
//!
//! `Settings` (`src/config/settings.rs`) is never touched by this struct,
//! following the Phase 22/23/24/26 precedent.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Configuration for the Run worker pool (PLAT-01, PLAT-02, D-10, D-15,
/// D-50, X-09). See the module-level documentation for why no separate
/// heartbeat field exists.
///
/// # Examples
///
/// ```
/// use paladin::config::run_worker::RunWorkerConfig;
///
/// let config = RunWorkerConfig::default();
/// assert_eq!(config.concurrency, 4);
/// assert_eq!(config.lease_seconds, 60);
/// assert_eq!(config.min_probe_interval_ms, 1000);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunWorkerConfig {
    /// The number of runs this instance dequeues and drives concurrently.
    /// Defaults to `4`.
    pub concurrency: u32,
    /// How long, in seconds, a dequeued run is leased to this worker before
    /// another worker may reclaim it on redelivery. Defaults to `60`. The
    /// worker's heartbeat interval is derived as `lease_seconds / 4` (D-10)
    /// -- there is no separate heartbeat field.
    pub lease_seconds: u64,
    /// The minimum interval, in milliseconds, between calls the facade's
    /// cancellation-probe adapter makes to the durable cancel-flag store for
    /// a single in-flight run (D-15). Defaults to `1000`.
    pub min_probe_interval_ms: u64,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `RunStoreConfig`'s convention: the today's-behaviour
// default is stated in code, not left implicit in a derive.
impl Default for RunWorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            lease_seconds: 60,
            min_probe_interval_ms: 1000,
        }
    }
}

impl RunWorkerConfig {
    /// Validates the run worker configuration.
    ///
    /// - `concurrency` must be non-zero.
    /// - `lease_seconds` must be at least `4`, so the derived
    ///   `lease_seconds / 4` heartbeat is never less than one whole second
    ///   (D-10).
    /// - `min_probe_interval_ms` must be non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::run_worker::RunWorkerConfig;
    ///
    /// let mut config = RunWorkerConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.lease_seconds = 1;
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.concurrency == 0 {
            return Err("run worker concurrency must be greater than 0".to_string());
        }
        if self.lease_seconds < 4 {
            return Err(
                "run worker lease_seconds must be at least 4 (the derived lease/4 heartbeat \
                 must be at least 1 second)"
                    .to_string(),
            );
        }
        if self.min_probe_interval_ms == 0 {
            return Err("run worker min_probe_interval_ms must be greater than 0".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for RunWorkerConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<u32>("APP_RUN_WORKER_CONCURRENCY") {
            self.concurrency = v;
        }
        if let Some(v) = read_env::<u64>("APP_RUN_WORKER_LEASE_SECONDS") {
            self.lease_seconds = v;
        }
        if let Some(v) = read_env::<u64>("APP_RUN_WORKER_MIN_PROBE_INTERVAL_MS") {
            self.min_probe_interval_ms = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default worker config matches PLAT-FR-02's stated
    /// defaults and validates cleanly.
    #[test]
    fn default_matches_documented_values() {
        let config = RunWorkerConfig::default();
        assert_eq!(config.concurrency, 4);
        assert_eq!(config.lease_seconds, 60);
        assert_eq!(config.min_probe_interval_ms, 1000);
        assert!(config.validate().is_ok());
    }

    /// Test 2: each `APP_RUN_WORKER_*` variable overrides its field.
    #[test]
    #[serial]
    fn env_overrides_apply_for_every_field() {
        unsafe {
            env::set_var("APP_RUN_WORKER_CONCURRENCY", "8");
            env::set_var("APP_RUN_WORKER_LEASE_SECONDS", "120");
            env::set_var("APP_RUN_WORKER_MIN_PROBE_INTERVAL_MS", "2500");
        }

        let mut config = RunWorkerConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.concurrency, 8);
        assert_eq!(config.lease_seconds, 120);
        assert_eq!(config.min_probe_interval_ms, 2500);

        unsafe {
            env::remove_var("APP_RUN_WORKER_CONCURRENCY");
            env::remove_var("APP_RUN_WORKER_LEASE_SECONDS");
            env::remove_var("APP_RUN_WORKER_MIN_PROBE_INTERVAL_MS");
        }
    }

    /// Test 3: zero concurrency, a sub-4-second lease, and a zero probe
    /// interval are each rejected by `validate()`, naming the offending
    /// field.
    #[test]
    fn validate_rejects_zero_concurrency_short_lease_and_zero_interval() {
        let mut config = RunWorkerConfig {
            concurrency: 0,
            ..RunWorkerConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("concurrency"));

        config = RunWorkerConfig {
            lease_seconds: 3,
            ..RunWorkerConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("lease_seconds"));

        config = RunWorkerConfig {
            min_probe_interval_ms: 0,
            ..RunWorkerConfig::default()
        };
        assert!(
            config
                .validate()
                .unwrap_err()
                .contains("min_probe_interval_ms")
        );
    }

    /// Test 4: a `lease_seconds` of exactly `4` -- the boundary at which
    /// the derived heartbeat is exactly one second -- is accepted.
    #[test]
    fn validate_accepts_the_four_second_lease_boundary() {
        let config = RunWorkerConfig {
            lease_seconds: 4,
            ..RunWorkerConfig::default()
        };
        assert!(config.validate().is_ok());
    }

    /// Test 5: a document omitting the `run_worker` section deserialises to
    /// the documented defaults.
    #[test]
    fn absent_section_deserializes_to_default() {
        let config: RunWorkerConfig = serde_json::from_value(serde_json::json!({
            "concurrency": 4,
            "lease_seconds": 60,
            "min_probe_interval_ms": 1000
        }))
        .unwrap();
        assert_eq!(config, RunWorkerConfig::default());
    }
}
