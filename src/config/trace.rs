//! Configuration for the runtime trace pipeline: the dispatcher capacity, the
//! log-sink default-on switch, state-value capture/redaction, the heartbeat
//! rate limit, and OTLP export (X-09, D-36).
//!
//! Mirrors [`crate::config::run_stream::RunStreamConfig`]'s shape
//! field-for-field (`Default` + `validate()` + [`EnvOverridable`]). Every
//! field is `#[serde(default)]` at the container level (via `TraceConfig`'s
//! and [`OtelConfig`]'s own `impl Default`), so a v0.9 config file that
//! never mentions a `trace:` key -- and a `trace:` section that overrides
//! only one field -- both boot with the documented defaults below for every
//! other field (X-03 backward compatibility).
//!
//! `otel.headers` holds credential-shaped values (e.g. an OTLP collector's
//! `authorization` bearer token) -- see [`OtelConfig`]'s own docs (Task 2)
//! for the redaction contract.
//!
//! # Env prefix: `PALADIN_TRACE_*`, not `APP_*`
//!
//! Every other config struct under `src/config/` (see
//! `waypoint_retention.rs`'s `APP_WAYPOINT_RETENTION_*` family) uses the
//! workspace's `APP_*` environment-variable prefix. This module
//! deliberately diverges and uses `PALADIN_TRACE_*` instead, per a locked
//! phase decision (D-36) recorded in `MIGRATION.md` §9.5 -- this is NOT a
//! typo or an oversight, and should not be "fixed" to match the `APP_*`
//! convention.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Configuration for the runtime trace pipeline (X-09, D-36). See the
/// module-level documentation for the backward-compatibility and env-prefix
/// contract.
///
/// # Examples
///
/// ```
/// use paladin::config::trace::TraceConfig;
///
/// let config = TraceConfig::default();
/// assert_eq!(config.channel_capacity, 1024);
/// assert!(config.log_sink);
/// assert!(!config.otel.enabled);
/// assert!(config.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TraceConfig {
    /// Whether the built-in log-sink `TraceSink` is wired by default.
    /// Defaults to `true` -- traces reach the process log out of the box.
    pub log_sink: bool,
    /// The bounded channel capacity between the engine's trace emitter and
    /// its sinks. Defaults to `1024`.
    pub channel_capacity: usize,
    /// Whether trace records are persisted to `run_traces`. Defaults to
    /// `false` -- a new subsystem stays off by default (X-09).
    pub persist: bool,
    /// Whether state-diff records carry actual field values (redacted then
    /// capped) rather than only field names. Defaults to `false`: unless
    /// explicitly opted in, no state value ever reaches a trace record, the
    /// SSE wire, or a persisted row.
    pub state_values: bool,
    /// The maximum byte length of a captured state value before it is
    /// truncated. Defaults to `256`.
    pub value_cap_bytes: usize,
    /// How often, in seconds, a running node's heartbeat may emit a trace
    /// record. Defaults to `5`.
    pub heartbeat_interval_secs: u64,
    /// OTLP export configuration. Defaults to disabled (see
    /// [`OtelConfig::default`]).
    pub otel: OtelConfig,
}

// A manual impl (not #[derive(Default)]), colocated with `validate()`'s own
// checks, mirroring `RunStreamConfig`'s and `WaypointRetentionConfig`'s
// convention: the today's-behavior default is stated in code, not left
// implicit in a derive.
impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            log_sink: true,
            channel_capacity: 1024,
            persist: false,
            state_values: false,
            value_cap_bytes: 256,
            heartbeat_interval_secs: 5,
            otel: OtelConfig::default(),
        }
    }
}

impl TraceConfig {
    /// Validates the trace configuration, matching the house
    /// `Result<(), String>` signature every `Settings`-composed `validate()`
    /// uses (X-03 -- `Settings::validate()`'s own signature must not
    /// change).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::trace::TraceConfig;
    ///
    /// let mut config = TraceConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.channel_capacity = 0;
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.channel_capacity == 0 {
            return Err("trace.channel_capacity must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// OTLP export configuration (D-36, X-09). Task 2 replaces the derived
/// `Debug` below with a manual, header-redacting impl and adds
/// `TraceConfigError`/`EnvOverridable`/the full `validate_typed` rule set --
/// this Task-1 shape exists only so [`TraceConfig`] has a concrete `otel`
/// field to carry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OtelConfig {
    /// Whether OTLP export is wired at all. Defaults to `false`.
    pub enabled: bool,
    /// The OTLP/HTTP traces endpoint. Defaults to
    /// `http://localhost:4318/v1/traces`, the standard local-collector
    /// address.
    pub endpoint: String,
    /// Extra headers sent with every OTLP export request (e.g. an
    /// `authorization` bearer token for the collector). Credential-shaped.
    pub headers: BTreeMap<String, String>,
    /// The `service.name` resource attribute attached to every exported
    /// span. Defaults to `"paladin"`.
    pub service_name: String,
}

// A manual impl (not #[derive(Default)]), colocated with `TraceConfig`'s own
// convention: the today's-behavior default (disabled) is stated in code.
impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:4318/v1/traces".to_string(),
            headers: BTreeMap::new(),
            service_name: "paladin".to_string(),
        }
    }
}

/// Structured validation errors for [`TraceConfig`] (X-06). `#[non_exhaustive]`
/// so a future variant is not a breaking change for downstream matchers.
///
/// RED-phase stub (Task 2): variants exist so the new tests compile;
/// `TraceConfig::validate_typed` does not raise any of them yet.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TraceConfigError {
    /// `channel_capacity` was `0`.
    #[error("trace.channel_capacity must be greater than 0")]
    ZeroChannelCapacity,
    /// `value_cap_bytes` was `0`.
    #[error("trace.value_cap_bytes must be greater than 0")]
    ZeroValueCapBytes,
    /// `otel.enabled` was `true` but `otel.endpoint`'s scheme was neither
    /// `http` nor `https`.
    #[error("trace.otel.endpoint must use the http or https scheme, got `{scheme}`")]
    EndpointNotHttp {
        /// The rejected scheme.
        scheme: String,
    },
    /// `otel.enabled` was `true` on a build compiled without the named
    /// Cargo feature.
    #[error("trace.otel.enabled requires the `{feature}` feature to be compiled in")]
    FeatureNotCompiled {
        /// The Cargo feature name required.
        feature: &'static str,
    },
}

impl TraceConfig {
    /// RED-phase stub (Task 2): always succeeds. The GREEN commit
    /// implements the four rules documented on [`TraceConfigError`].
    pub fn validate_typed(&self) -> Result<(), TraceConfigError> {
        Ok(())
    }
}

// RED-phase stub (Task 2): a no-op `EnvOverridable` impl so
// `trace_env_overrides_apply` compiles and fails on its assertions. The
// GREEN commit reads the nine `PALADIN_TRACE_*` variables.
impl crate::config::env_utils::EnvOverridable for TraceConfig {
    fn apply_env_overrides(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::env_utils::EnvOverridable;
    use config::{Config, File, FileFormat};
    use serial_test::serial;
    use std::env;

    /// A minimal `Settings`-shaped wrapper used to exercise `trace`'s
    /// deserialization behavior in isolation, mirroring
    /// `agent_runtime.rs`'s test-module `Wrapper` pattern.
    #[derive(Debug, Deserialize)]
    struct Wrapper {
        #[serde(default)]
        trace: TraceConfig,
    }

    fn deserialize_wrapper(yaml: &str) -> Wrapper {
        Config::builder()
            .add_source(File::from_str(yaml, FileFormat::Yaml))
            .build()
            .expect("config should build from the inline yaml fixture")
            .try_deserialize()
            .expect("wrapper should deserialize")
    }

    // ── Task 1 behaviors ─────────────────────────────────────────────────

    /// Behavior 1: `Settings::load_from_file` on the tracked
    /// `config.test.yml`, which carries a `trace:` section overriding only
    /// `channel_capacity`, yields that override on `settings.trace` and
    /// house defaults for every other field.
    #[test]
    #[serial]
    fn trace_section_partial_override_applies_and_defaults_rest() {
        let settings = crate::config::settings::Settings::load_from_file("config.test.yml")
            .expect("config.test.yml should load");

        assert_eq!(settings.trace.channel_capacity, 32);
        assert!(settings.trace.log_sink);
        assert!(!settings.trace.persist);
        assert!(!settings.trace.state_values);
        assert_eq!(settings.trace.value_cap_bytes, 256);
        assert_eq!(settings.trace.heartbeat_interval_secs, 5);
        assert_eq!(settings.trace.otel, OtelConfig::default());
    }

    /// Behavior 2: a document with no `trace:` key at all deserializes to
    /// `TraceConfig::default()` -- the section is fully optional (X-03).
    #[test]
    fn absent_trace_section_deserializes_to_default() {
        let wrapper = deserialize_wrapper("other_key: 1\n");
        assert_eq!(wrapper.trace, TraceConfig::default());
    }

    /// Behavior 3: `Settings::validate()` rejects `trace.channel_capacity ==
    /// 0`, naming the field in the error message.
    #[test]
    fn validate_rejects_zero_channel_capacity() {
        let mut settings = crate::config::settings::Settings::default();
        settings.trace.channel_capacity = 0;

        let err = settings
            .trace
            .validate()
            .expect_err("zero channel_capacity should be rejected");
        assert!(err.contains("channel_capacity"));
    }

    // ── Task 2 behaviors ─────────────────────────────────────────────────

    /// Behavior: `OtelConfig::default()` matches D-36's documented values.
    #[test]
    fn otel_config_default_matches_documented_values() {
        let config = OtelConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.endpoint, "http://localhost:4318/v1/traces");
        assert!(config.headers.is_empty());
        assert_eq!(config.service_name, "paladin");
    }

    /// Behavior: `OtelConfig`'s `Debug` output names header keys but never
    /// prints a header value (security instructions, D-36).
    #[test]
    fn otel_debug_redacts_header_values() {
        let mut config = OtelConfig::default();
        config
            .headers
            .insert("authorization".to_string(), "Bearer sk-secret".to_string());

        let rendered = format!("{config:?}");

        assert!(rendered.contains("authorization"));
        assert!(!rendered.contains("sk-secret"));
    }

    /// Behavior: `validate_typed` rejects a zero `channel_capacity`, a zero
    /// `value_cap_bytes`, and (this build has no `otel` feature declared)
    /// `otel.enabled` at all, each with its own distinct variant.
    #[test]
    fn validate_typed_rejects_each_invalid_case_distinctly() {
        let mut config = TraceConfig {
            channel_capacity: 0,
            ..TraceConfig::default()
        };
        assert!(matches!(
            config.validate_typed(),
            Err(TraceConfigError::ZeroChannelCapacity)
        ));

        config = TraceConfig {
            value_cap_bytes: 0,
            ..TraceConfig::default()
        };
        assert!(matches!(
            config.validate_typed(),
            Err(TraceConfigError::ZeroValueCapBytes)
        ));

        // This workspace has no `otel` Cargo feature declared yet (28-09's
        // scope), so `otel.enabled` alone is always rejected as
        // `FeatureNotCompiled` -- exercised directly below rather than via
        // a non-http(s) scheme, since that branch is unreachable until the
        // feature exists.
        config = TraceConfig::default();
        config.otel.enabled = true;
        assert!(matches!(
            config.validate_typed(),
            Err(TraceConfigError::FeatureNotCompiled { feature: "otel" })
        ));
    }

    /// Behavior: setting all nine `PALADIN_TRACE_*` variables and calling
    /// `apply_env_overrides` changes exactly those fields.
    #[test]
    #[serial]
    fn trace_env_overrides_apply() {
        unsafe {
            env::set_var("PALADIN_TRACE_LOG_SINK", "false");
            env::set_var("PALADIN_TRACE_CHANNEL_CAPACITY", "64");
            env::set_var("PALADIN_TRACE_PERSIST", "true");
            env::set_var("PALADIN_TRACE_STATE_VALUES", "true");
            env::set_var("PALADIN_TRACE_VALUE_CAP_BYTES", "64");
            env::set_var("PALADIN_TRACE_HEARTBEAT_INTERVAL_SECS", "1");
            env::set_var("PALADIN_TRACE_OTEL_ENABLED", "true");
            env::set_var(
                "PALADIN_TRACE_OTEL_ENDPOINT",
                "https://collector.example.com/v1/traces",
            );
            env::set_var("PALADIN_TRACE_OTEL_SERVICE_NAME", "paladin-test");
        }

        let mut config = TraceConfig::default();
        config.apply_env_overrides();

        assert!(!config.log_sink);
        assert_eq!(config.channel_capacity, 64);
        assert!(config.persist);
        assert!(config.state_values);
        assert_eq!(config.value_cap_bytes, 64);
        assert_eq!(config.heartbeat_interval_secs, 1);
        assert!(config.otel.enabled);
        assert_eq!(
            config.otel.endpoint,
            "https://collector.example.com/v1/traces"
        );
        assert_eq!(config.otel.service_name, "paladin-test");

        unsafe {
            env::remove_var("PALADIN_TRACE_LOG_SINK");
            env::remove_var("PALADIN_TRACE_CHANNEL_CAPACITY");
            env::remove_var("PALADIN_TRACE_PERSIST");
            env::remove_var("PALADIN_TRACE_STATE_VALUES");
            env::remove_var("PALADIN_TRACE_VALUE_CAP_BYTES");
            env::remove_var("PALADIN_TRACE_HEARTBEAT_INTERVAL_SECS");
            env::remove_var("PALADIN_TRACE_OTEL_ENABLED");
            env::remove_var("PALADIN_TRACE_OTEL_ENDPOINT");
            env::remove_var("PALADIN_TRACE_OTEL_SERVICE_NAME");
        }
    }
}
