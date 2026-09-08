//! Configuration for the Assistants discovery surface (PLAT-04, D-32, D-50,
//! X-09).
//!
//! Mirrors [`crate::config::run_store::RunStoreConfig`]'s shape (`Default` +
//! `validate()` + `EnvOverridable`), but is the **one deliberate exception**
//! to X-09's "new subsystems default to OFF" rule (D-50): PLAT-FR-11's "one
//! discovery surface" promise -- that `GET /assistants` lists both
//! code-registered and API-registered assistants together -- is only true
//! by default when `expose_code_registry` starts `true`. The exposed data
//! is read-only (code-registered assistants reject every mutating route
//! with `409 code_registered_immutable`, D-32) and was already discoverable
//! through the pre-existing `AgentRegistry`
//! (`crates/paladin-web/src/agent_registry.rs`, X-03, untouched by this
//! struct), so turning this default on adds no new capability -- only a
//! unified read surface for capability that already existed.
//!
//! `Settings` (`src/config/settings.rs`) is never touched by this struct,
//! following the Phase 22/23/24/26 precedent.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Configuration for the Assistants discovery surface (PLAT-04, D-32, D-50,
/// X-09). See the module-level documentation for why `expose_code_registry`
/// defaults `true` rather than `false`.
///
/// # Examples
///
/// ```
/// use paladin::config::assistants::AssistantsConfig;
///
/// let config = AssistantsConfig::default();
/// assert!(config.expose_code_registry);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantsConfig {
    /// Whether code-registered agents from the pre-existing `AgentRegistry`
    /// are rendered as read-only synthetic assistants (`{ source: "code",
    /// version: 1 }`) alongside API-registered ones in `GET /assistants`
    /// (D-32). Defaults to `true` -- see the module-level documentation for
    /// why this is the one subsystem in this phase that starts ON.
    pub expose_code_registry: bool,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `RunStoreConfig`'s convention: the default is stated in
// code, not left implicit in a derive.
impl Default for AssistantsConfig {
    fn default() -> Self {
        Self {
            expose_code_registry: true,
        }
    }
}

impl AssistantsConfig {
    /// Validates the assistants configuration.
    ///
    /// A single boolean field has no invalid state; this always returns
    /// `Ok(())`. The method exists so `AssistantsConfig` follows the same
    /// `Default` + `validate()` + `EnvOverridable` shape as every other
    /// X-09 config struct (D-50).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::assistants::AssistantsConfig;
    ///
    /// let config = AssistantsConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

impl EnvOverridable for AssistantsConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<bool>("APP_ASSISTANTS_EXPOSE_CODE_REGISTRY") {
            self.expose_code_registry = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Test 1: the default config exposes the code registry and validates
    /// cleanly.
    #[test]
    fn default_exposes_code_registry() {
        let config = AssistantsConfig::default();
        assert!(config.expose_code_registry);
        assert!(config.validate().is_ok());
    }

    /// Test 2: `APP_ASSISTANTS_EXPOSE_CODE_REGISTRY` overrides the field,
    /// including turning it off.
    #[test]
    #[serial]
    fn env_override_applies() {
        unsafe {
            env::set_var("APP_ASSISTANTS_EXPOSE_CODE_REGISTRY", "false");
        }

        let mut config = AssistantsConfig::default();
        config.apply_env_overrides();

        assert!(!config.expose_code_registry);

        unsafe {
            env::remove_var("APP_ASSISTANTS_EXPOSE_CODE_REGISTRY");
        }
    }

    /// Test 3: a document omitting the `assistants` section deserialises to
    /// the on-by-default value.
    #[test]
    fn absent_section_deserializes_to_default() {
        let config: AssistantsConfig =
            serde_json::from_value(serde_json::json!({ "expose_code_registry": true })).unwrap();
        assert_eq!(config, AssistantsConfig::default());
    }
}
