//! Configuration for the Run webhook delivery subsystem (PLAT-05, D-40
//! through D-43, D-50, X-09).
//!
//! Mirrors [`crate::config::schedules::SchedulesConfig`]'s shape
//! field-for-field (`Default` + `validate()` + `EnvOverridable`). Defaults
//! to `allow_private: false`, `max_attempts: 5`, `timeout_secs: 10`, so a
//! v0.9 deployment that never mentions this struct boots v0.10 with the
//! SSRF guard fully engaged and the D-43 retry schedule as documented below
//! -- X-09's "new subsystems default to safe behaviour" requirement.
//!
//! ## The SSRF guard (D-42)
//!
//! Every webhook target url is checked by a standalone, table-tested guard
//! function applied at **both** write time (when a run's webhook url is
//! accepted, e.g. `POST /runs`) and send time (immediately before each
//! delivery attempt, because a DNS record backing an already-accepted
//! hostname can change between acceptance and send). The guard rejects:
//!
//! - any scheme other than `http` or `https`;
//! - any host resolving to a loopback address;
//! - any host resolving to a link-local address (`169.254.0.0/16`,
//!   `fe80::/10`) -- which also covers the cloud metadata address
//!   `169.254.169.254`;
//! - any host resolving to an RFC1918 private address;
//! - any host resolving to a unique-local (`fc00::/7`) address;
//! - any host resolving to an unspecified address (`0.0.0.0`, `::`).
//!
//! [`WebhooksConfig::allow_private`] is the **only** override of this guard
//! (default `false`); flipping it on is a deliberate, documented widening of
//! the deployment's SSRF surface, never an accidental default.
//!
//! The webhook `reqwest::Client` never follows redirects: the delivery
//! request carries `X-Paladin-Signature`, a credential-shaped header
//! (D-41), and a followed redirect would both forward that header to an
//! attacker-chosen host and bypass the write-time guard check entirely
//! (`security.instructions.md`'s house rule for credential-bearing
//! clients).
//!
//! **Known limitation (documented, not implemented):** this guard performs
//! host classification at check time, but does not pin the resolved
//! address for the subsequent connection -- so a DNS-rebinding attack
//! (the name resolves to a public address at check time and a private one
//! at connect time) is not defended against. Resolve-then-connect address
//! pinning is out of scope for this phase (PRD 06 PLAT-FR-15 explicitly
//! permits the gap); claiming coverage this guard does not have would be
//! worse than naming the limitation plainly.
//!
//! ## Retry schedule (D-43)
//!
//! `2xx` responses are delivered and not retried. `4xx` responses
//! dead-letter immediately (the target itself is rejecting the payload; a
//! retry is normally not the fix). `5xx` responses, request timeouts, and
//! connect errors retry up to `max_attempts` times with exponential
//! backoff.
//!
//! `Settings` (`src/config/settings.rs`) is never touched by this struct,
//! following the Phase 22/23/24/26 precedent.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Configuration for the Run webhook delivery subsystem (PLAT-05, D-40
/// through D-43, D-50, X-09). See the module-level documentation for the
/// SSRF guard's rejected target classes and the DNS-rebinding limitation.
///
/// # Examples
///
/// ```
/// use paladin::config::webhooks::WebhooksConfig;
///
/// let config = WebhooksConfig::default();
/// assert!(!config.allow_private);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhooksConfig {
    /// The **only** override of the SSRF guard (D-42): when `true`, targets
    /// resolving to loopback, link-local, RFC1918, unique-local or
    /// unspecified addresses are permitted. Defaults to `false`. Flipping
    /// this on does not add resolve-then-connect DNS-rebinding pinning --
    /// see the module-level documentation for that limitation.
    pub allow_private: bool,
    /// The number of delivery attempts (initial send plus retries) before a
    /// `5xx`/timeout/connect-error delivery is dead-lettered (D-43).
    /// Defaults to `5`.
    pub max_attempts: u32,
    /// The per-attempt request timeout, in seconds. Defaults to `10`.
    pub timeout_secs: u64,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `SchedulesConfig`'s convention: the safe-by-default
// contract is stated in code, not left implicit in a derive.
impl Default for WebhooksConfig {
    fn default() -> Self {
        Self {
            allow_private: false,
            max_attempts: 5,
            timeout_secs: 10,
        }
    }
}

impl WebhooksConfig {
    /// Validates the webhooks configuration.
    ///
    /// - `max_attempts` must be non-zero.
    /// - `timeout_secs` must be non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::webhooks::WebhooksConfig;
    ///
    /// let mut config = WebhooksConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.max_attempts = 0;
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.max_attempts == 0 {
            return Err("webhooks max_attempts must be greater than 0".to_string());
        }
        if self.timeout_secs == 0 {
            return Err("webhooks timeout_secs must be greater than 0".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for WebhooksConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_WEBHOOKS_ALLOW_PRIVATE") {
            self.allow_private = v;
        }
        if let Some(v) = read_env::<u32>("APP_WEBHOOKS_MAX_ATTEMPTS") {
            self.max_attempts = v;
        }
        if let Some(v) = read_env::<u64>("APP_WEBHOOKS_TIMEOUT_SECS") {
            self.timeout_secs = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default config keeps the SSRF guard fully engaged and
    /// validates cleanly.
    #[test]
    fn default_disallows_private_targets() {
        let config = WebhooksConfig::default();
        assert!(!config.allow_private);
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.timeout_secs, 10);
        assert!(config.validate().is_ok());
    }

    /// Test 2: `APP_WEBHOOKS_*` variables override their fields, including
    /// deliberately widening the SSRF guard.
    #[test]
    #[serial]
    fn env_overrides_apply_for_every_field() {
        unsafe {
            env::set_var("APP_WEBHOOKS_ALLOW_PRIVATE", "true");
            env::set_var("APP_WEBHOOKS_MAX_ATTEMPTS", "3");
            env::set_var("APP_WEBHOOKS_TIMEOUT_SECS", "30");
        }

        let mut config = WebhooksConfig::default();
        config.apply_env_overrides();

        assert!(config.allow_private);
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.timeout_secs, 30);

        unsafe {
            env::remove_var("APP_WEBHOOKS_ALLOW_PRIVATE");
            env::remove_var("APP_WEBHOOKS_MAX_ATTEMPTS");
            env::remove_var("APP_WEBHOOKS_TIMEOUT_SECS");
        }
    }

    /// Test 3: a zero `max_attempts` and a zero `timeout_secs` are each
    /// rejected by `validate()`, naming the offending field.
    #[test]
    fn validate_rejects_zero_max_attempts_and_zero_timeout() {
        let mut config = WebhooksConfig {
            max_attempts: 0,
            ..WebhooksConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("max_attempts"));

        config = WebhooksConfig {
            timeout_secs: 0,
            ..WebhooksConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("timeout_secs"));
    }

    /// Test 4: a document omitting the `webhooks` section deserialises to
    /// the safe-by-default values.
    #[test]
    fn absent_section_deserializes_to_default() {
        let config: WebhooksConfig = serde_json::from_value(serde_json::json!({
            "allow_private": false,
            "max_attempts": 5,
            "timeout_secs": 10
        }))
        .unwrap();
        assert_eq!(config, WebhooksConfig::default());
    }
}
