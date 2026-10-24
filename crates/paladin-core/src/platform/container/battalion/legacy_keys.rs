//! Load-time refusal of the configuration keys Phase 44 removed (Phase 46, D-23).
//!
//! A `BattalionConfig` document written by v0.10 may still carry `timeout_seconds`,
//! `retry_policy` or `error_strategy`. v0.11 has no field for any of them, and a plain
//! `Deserialize` would skip them silently, loading a run that v0.10 bounded as one that is
//! now unbounded. Instead the document is refused, once, with an error that names every
//! removed key present and the `aegis.*` setting that replaces it, and cites `MIGRATION.md`
//! row `M-B-06`.
//!
//! The mechanism is a private "wire" struct that mirrors the public type and also accepts the
//! removed keys; the public type deserializes through it with
//! `#[serde(try_from = "...")]`. That keeps the public struct shape, its `Serialize` output
//! and the Rust surface exactly as they were. It is a refusal, not a compatibility
//! deserializer: nothing maps a removed key onto its replacement, and the refusal never
//! echoes a value (a value may be a credential-shaped string). Only the removed keys are
//! refused; the wire structs do not use serde's unknown-field denial, so an unrelated
//! unknown key still loads and a v0.11 document is unaffected.
//!
//! The removed key names and the message text live in this file rather than in `mod.rs` or
//! `conclave.rs` because the legacy-removal guard (`tests/legacy_removal_guard.rs`, rule 6)
//! forbids the removed names in the production code of those two files.

use std::path::PathBuf;

use serde::Deserialize;
use serde::de::{Deserializer, IgnoredAny};

use super::BattalionConfig;
use super::conclave::{ConclaveConfig, ObservabilityLevel};
use crate::platform::container::aegis::Aegis;

/// A key removed in v0.11.0 together with the setting that replaces it.
type RemovedKey = (&'static str, &'static str);

/// Marks a key as present for any value, `null` included, without keeping the value.
///
/// Used as `#[serde(default, deserialize_with = "present")]` on an `Option<()>`: an absent
/// key leaves `None`, a present key (whatever it holds) yields `Some(())`.
fn present<'de, D>(deserializer: D) -> Result<Option<()>, D::Error>
where
    D: Deserializer<'de>,
{
    IgnoredAny::deserialize(deserializer)?;
    Ok(Some(()))
}

/// A document carried one or more keys removed in v0.11.0.
///
/// The message lists only key names and fixed replacement text, never a value.
#[derive(Debug, thiserror::Error)]
#[error(
    "{config}: removed in v0.11.0: {}; see MIGRATION.md section 10.1, row M-B-06",
    render_keys(.keys)
)]
pub(super) struct RemovedConfigKeys {
    /// The type whose document was refused.
    config: &'static str,
    /// Every removed key found, with its replacement.
    keys: Vec<RemovedKey>,
}

/// Renders `` `key` (set `replacement`) `` pairs, comma separated.
fn render_keys(keys: &[RemovedKey]) -> String {
    keys.iter()
        .map(|(key, replacement)| format!("`{key}` (set `{replacement}`)"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `Ok` when no candidate is present; otherwise one error naming every present key.
///
/// Each candidate is `(present, key, replacement)`.
fn refuse_removed(
    config: &'static str,
    candidates: &[(bool, &'static str, &'static str)],
) -> Result<(), RemovedConfigKeys> {
    let keys: Vec<RemovedKey> = candidates
        .iter()
        .filter(|(is_present, ..)| *is_present)
        .map(|&(_, key, replacement)| (key, replacement))
        .collect();
    if keys.is_empty() {
        Ok(())
    } else {
        Err(RemovedConfigKeys { config, keys })
    }
}

/// Wire form of [`BattalionConfig`]: the four current fields plus the three removed keys.
#[derive(Deserialize)]
pub(super) struct BattalionConfigWire {
    name: String,
    description: Option<String>,
    #[serde(default)]
    aegis: Aegis,
    metadata_output_dir: Option<PathBuf>,
    #[serde(default, deserialize_with = "present")]
    timeout_seconds: Option<()>,
    #[serde(default, deserialize_with = "present")]
    retry_policy: Option<()>,
    #[serde(default, deserialize_with = "present")]
    error_strategy: Option<()>,
}

impl TryFrom<BattalionConfigWire> for BattalionConfig {
    type Error = RemovedConfigKeys;

    fn try_from(wire: BattalionConfigWire) -> Result<Self, Self::Error> {
        refuse_removed(
            "BattalionConfig",
            &[
                (
                    wire.timeout_seconds.is_some(),
                    "timeout_seconds",
                    "aegis.timeout.run_timeout",
                ),
                (wire.retry_policy.is_some(), "retry_policy", "aegis.retry"),
                (
                    wire.error_strategy.is_some(),
                    "error_strategy",
                    "aegis.on_error",
                ),
            ],
        )?;
        Ok(Self {
            name: wire.name,
            description: wire.description,
            aegis: wire.aegis,
            metadata_output_dir: wire.metadata_output_dir,
        })
    }
}

/// Wire form of [`ConclaveConfig`]: its seven fields plus the removed `timeout_seconds` key.
///
/// `battalion_config` is a [`BattalionConfig`], so a removed key nested inside it is refused
/// by that type's own conversion.
#[derive(Deserialize)]
pub(super) struct ConclaveConfigWire {
    name: String,
    battalion_config: BattalionConfig,
    retry_attempts: u32,
    synthesis_prompt: Option<String>,
    include_expert_names: bool,
    max_expert_output_tokens: Option<usize>,
    observability_level: ObservabilityLevel,
    #[serde(default, deserialize_with = "present")]
    timeout_seconds: Option<()>,
}

impl TryFrom<ConclaveConfigWire> for ConclaveConfig {
    type Error = RemovedConfigKeys;

    fn try_from(wire: ConclaveConfigWire) -> Result<Self, Self::Error> {
        refuse_removed(
            "ConclaveConfig",
            &[(
                wire.timeout_seconds.is_some(),
                "timeout_seconds",
                "battalion_config.aegis.timeout.run_timeout",
            )],
        )?;
        Ok(Self {
            name: wire.name,
            battalion_config: wire.battalion_config,
            retry_attempts: wire.retry_attempts,
            synthesis_prompt: wire.synthesis_prompt,
            include_expert_names: wire.include_expert_names,
            max_expert_output_tokens: wire.max_expert_output_tokens,
            observability_level: wire.observability_level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::BattalionConfig;
    use super::super::conclave::ConclaveConfig;
    use serde_json::json;

    fn refusal(document: serde_json::Value) -> String {
        serde_json::from_value::<BattalionConfig>(document)
            .expect_err("a removed key must be refused")
            .to_string()
    }

    #[test]
    fn timeout_key_alone_is_refused_naming_its_replacement_and_row() {
        let message = refusal(json!({"name": "a", "timeout_seconds": 300}));
        assert!(message.contains("timeout_seconds"), "{message}");
        assert!(message.contains("aegis.timeout.run_timeout"), "{message}");
        assert!(message.contains("M-B-06"), "{message}");
    }

    #[test]
    fn retry_key_alone_is_refused_naming_its_replacement() {
        let message = refusal(json!({"name": "a", "retry_policy": {"max_attempts": 3}}));
        assert!(message.contains("retry_policy"), "{message}");
        assert!(message.contains("aegis.retry"), "{message}");
        assert!(message.contains("M-B-06"), "{message}");
    }

    #[test]
    fn error_key_alone_is_refused_naming_its_replacement() {
        let message = refusal(json!({"name": "a", "error_strategy": "FailFast"}));
        assert!(message.contains("error_strategy"), "{message}");
        assert!(message.contains("aegis.on_error"), "{message}");
        assert!(message.contains("M-B-06"), "{message}");
    }

    #[test]
    fn all_three_keys_fail_once_naming_every_key_and_replacement() {
        let message = refusal(json!({
            "name": "a",
            "timeout_seconds": 300,
            "retry_policy": {"max_attempts": 3},
            "error_strategy": "FailFast",
        }));
        for needle in [
            "timeout_seconds",
            "retry_policy",
            "error_strategy",
            "aegis.timeout.run_timeout",
            "aegis.retry",
            "aegis.on_error",
            "M-B-06",
        ] {
            assert!(message.contains(needle), "missing {needle}: {message}");
        }
    }

    #[test]
    fn only_the_keys_present_are_listed() {
        let message = refusal(json!({"name": "a", "timeout_seconds": 300}));
        assert!(!message.contains("retry_policy"), "{message}");
        assert!(!message.contains("error_strategy"), "{message}");
    }

    #[test]
    fn a_removed_key_with_a_null_value_is_refused() {
        let message = refusal(json!({"name": "a", "timeout_seconds": null}));
        assert!(message.contains("timeout_seconds"), "{message}");
    }

    #[test]
    fn the_refusal_never_echoes_a_value() {
        let message = refusal(json!({
            "name": "a",
            "retry_policy": {"secret": "sk-planted-value"},
            "timeout_seconds": "sk-planted-value",
            "error_strategy": "sk-planted-value",
        }));
        assert!(!message.contains("sk-planted-value"), "{message}");
    }

    #[test]
    fn documents_without_a_removed_key_load() {
        let bare: BattalionConfig =
            serde_json::from_value(json!({"name": "a"})).expect("bare document loads");
        assert_eq!(bare.name, "a");
        let with_aegis: BattalionConfig =
            serde_json::from_value(json!({"name": "a", "aegis": {}})).expect("empty aegis loads");
        assert_eq!(with_aegis.aegis, Default::default());
    }

    #[test]
    fn an_unrelated_unknown_key_is_ignored() {
        let config: BattalionConfig = serde_json::from_value(json!({"name": "a", "novel": 1}))
            .expect("an unknown key is not a removed key");
        assert_eq!(config.name, "a");
    }

    #[test]
    fn serialized_form_carries_exactly_the_four_current_keys_and_round_trips() {
        let value = serde_json::to_value(BattalionConfig::new("a")).expect("serialize");
        let mut keys: Vec<&str> = value
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["aegis", "description", "metadata_output_dir", "name"]
        );
        let back: BattalionConfig = serde_json::from_value(value).expect("round trip");
        assert_eq!(back.name, "a");
        assert_eq!(back.aegis, Default::default());
    }

    fn conclave_value() -> serde_json::Value {
        serde_json::to_value(ConclaveConfig::new("c", BattalionConfig::new("b")))
            .expect("serialize")
    }

    fn conclave_refusal(document: serde_json::Value) -> String {
        serde_json::from_value::<ConclaveConfig>(document)
            .expect_err("a removed key must be refused")
            .to_string()
    }

    #[test]
    fn conclave_timeout_key_is_refused_naming_its_replacement_and_row() {
        let mut document = conclave_value();
        document["timeout_seconds"] = json!(300);
        let message = conclave_refusal(document);
        assert!(message.contains("timeout_seconds"), "{message}");
        assert!(
            message.contains("battalion_config.aegis.timeout.run_timeout"),
            "{message}"
        );
        assert!(message.contains("M-B-06"), "{message}");
    }

    #[test]
    fn conclave_timeout_key_with_a_null_value_is_refused_without_echoing_values() {
        let mut document = conclave_value();
        document["timeout_seconds"] = json!(null);
        assert!(conclave_refusal(document).contains("timeout_seconds"));
        let mut document = conclave_value();
        document["timeout_seconds"] = json!("sk-planted-value");
        assert!(!conclave_refusal(document).contains("sk-planted-value"));
    }

    #[test]
    fn conclave_with_a_removed_key_in_its_nested_battalion_config_is_refused() {
        let mut document = conclave_value();
        document["battalion_config"]["retry_policy"] = json!({"max_attempts": 3});
        let message = conclave_refusal(document);
        assert!(message.contains("retry_policy"), "{message}");
        assert!(message.contains("aegis.retry"), "{message}");
        assert!(message.contains("M-B-06"), "{message}");
    }

    #[test]
    fn conclave_round_trips_and_ignores_an_unrelated_unknown_key() {
        let value = conclave_value();
        let back: ConclaveConfig = serde_json::from_value(value.clone()).expect("round trip");
        assert_eq!(serde_json::to_value(&back).expect("serialize"), value);
        let mut with_novel = value;
        with_novel["novel"] = json!(1);
        let loaded: ConclaveConfig =
            serde_json::from_value(with_novel).expect("an unknown key is not a removed key");
        assert_eq!(loaded.name, "c");
    }

    #[test]
    fn conclave_serialized_form_carries_no_removed_key() {
        let value = conclave_value();
        let object = value.as_object().expect("object");
        assert!(!object.contains_key("timeout_seconds"));
        assert!(
            !object["battalion_config"]
                .as_object()
                .expect("object")
                .contains_key("timeout_seconds")
        );
    }
}
