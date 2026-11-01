//! The one wire-time sampling filter every request builder calls (ADR-0061).
//!
//! # The policy in one paragraph
//!
//! A sampling parameter reaches a vendor only when the target model's
//! [`ModelProfile`](paladin_ports::output::model_profile::ModelProfile) admits it. The work is split in two so that no value is ever judged twice and
//! none is ever swapped for another:
//!
//! - **Refuse at build.** `PaladinBuilder` refuses an *explicit* value (an operator's
//!   `.temperature(0.5)`, or more `.add_stop_word(..)` words than the profile's numeric maximum)
//!   the profile does not admit, with an error naming provider, model and parameter. An explicit
//!   request is a statement of intent; silently dropping it would be a divergence nobody can
//!   discover. Stop words on a model that takes no wire `stop` are the one exception: the
//!   execution service enforces them client-side, so that value is omitted below and recorded.
//! - **Filter at the wire.** This module is the other half. A value that is set on the request
//!   but not admitted -- the framework's fabricated `0.7`, an auto-selected temperature, a value a
//!   direct caller put on an `LlmRequest` -- is **omitted** from the outgoing body, and the
//!   omission is made visible. Nothing is clamped, defaulted, snapped or substituted: an
//!   inadmissible `0.85` on a fixed-`1.0` model is left out, never replaced by `1.0`.
//!
//! # Visibility rule
//!
//! Every omission is recorded twice. The first time an adapter instance omits a given
//! (model, parameter) it emits one `log::warn!` (target `paladin::sampling`) -- once, so a hot
//! loop cannot flood the log. Every buffered response from a request that omitted something
//! carries [`PARAMS_OMITTED_METADATA_KEY`](crate::sampling::PARAMS_OMITTED_METADATA_KEY) in `LlmResponse.metadata`; the key is absent when
//! nothing was omitted. The same two-part rule covers a `response_format` the profile cannot
//! honour ([`RESPONSE_FORMAT_ADJUSTED_METADATA_KEY`](crate::sampling::RESPONSE_FORMAT_ADJUSTED_METADATA_KEY)). A `max_tokens` above the model's profiled
//! maximum is different in kind: it is refused before anything is sent (`check_max_tokens`),
//! never clamped, because a truncated answer is not the one the caller asked for.
//!
//! Streamed calls have no metadata channel yet, so a streamed omission is visible through the
//! `warn!` only (owner reading R-1 in ADR-0061 decides whether the keys also reach the stream's
//! terminal chunk).
//!
//! Warn text names the provider, the model, the parameter and a numeric value or a count --
//! never a key, a URL or a body.
// The helpers below are consumed by the compat engine and by the four bespoke adapters (OpenAI,
// DeepSeek, Anthropic, Gemini); without a consuming adapter feature they are legitimately unused.
#![cfg_attr(
    not(any(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible",
        feature = "openai",
        feature = "anthropic",
        feature = "deepseek",
        feature = "gemini"
    )),
    allow(dead_code)
)]

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use paladin_core::platform::container::prompt::PromptParameters;
use paladin_core::platform::container::reasoning::{
    REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
};
use paladin_ports::output::llm_port::{LlmError, ResponseFormat};
use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, SamplingParam};

use crate::registry::UNPROFILED_MODEL_METADATA_KEY;

/// The `LlmResponse.metadata` key under which an adapter records the sampling parameters it did
/// not send, as a comma-separated list of [`SamplingParam::as_str`] names in fixed order
/// (`temperature`, `top_p`, `frequency_penalty`, `presence_penalty`, `stop`).
///
/// Absent -- never empty -- when nothing was omitted.
pub const PARAMS_OMITTED_METADATA_KEY: &str = "paladin.params_omitted";

/// The `LlmResponse.metadata` key under which an adapter records that it changed or dropped the
/// requested `response_format`: `json_object` (a JSON-schema request was downgraded) or
/// `omitted` (the model supports no JSON mode, so none was sent).
///
/// Absent when the requested format reached the wire unchanged or none was requested.
pub const RESPONSE_FORMAT_ADJUSTED_METADATA_KEY: &str = "paladin.response_format_adjusted";

/// What [`apply`] decided to send.
///
/// Each `Option` is `Some` only when the request set the parameter **and** the profile admits it.
/// `omitted` names every parameter that was set but not admitted, in fixed parameter order; an
/// unset parameter is never listed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Applied {
    /// The `temperature` to send.
    pub temperature: Option<f32>,
    /// The `top_p` to send.
    pub top_p: Option<f32>,
    /// The `frequency_penalty` to send.
    pub frequency_penalty: Option<f32>,
    /// The `presence_penalty` to send.
    pub presence_penalty: Option<f32>,
    /// The stop sequences to send, in the order the caller declared them.
    pub stop: Option<Vec<String>>,
    /// The parameters that were set but not admitted, in fixed parameter order.
    pub omitted: Vec<SamplingParam>,
    /// For each entry of `omitted`, in the same order, the value in log-safe form (a number, or a
    /// count for stop sequences).
    pub omitted_values: Vec<String>,
}

/// Decide what reaches the wire (ADR-0061 (a)): a set parameter is sent only when the profile
/// admits it, every other set parameter is omitted and named in [`Applied::omitted`], and an
/// unset parameter is never sent, defaulted or named.
///
/// Numeric parameters go through [`ParamSupport::admits`]; stop sequences go through
/// [`paladin_ports::output::model_profile::StopSupport::admits`] with their count. An empty stop
/// list counts as unset. Order and duplicates of stop sequences are preserved.
pub(crate) fn apply(profile: &ModelProfile, params: &PromptParameters) -> Applied {
    let mut omitted = Vec::new();
    let mut omitted_values = Vec::new();

    let temperature = admit_numeric(
        SamplingParam::Temperature,
        params.temperature,
        profile.temperature,
        &mut omitted,
        &mut omitted_values,
    );
    let top_p = admit_numeric(
        SamplingParam::TopP,
        params.top_p,
        profile.top_p,
        &mut omitted,
        &mut omitted_values,
    );
    let frequency_penalty = admit_numeric(
        SamplingParam::FrequencyPenalty,
        params.frequency_penalty,
        profile.frequency_penalty,
        &mut omitted,
        &mut omitted_values,
    );
    let presence_penalty = admit_numeric(
        SamplingParam::PresencePenalty,
        params.presence_penalty,
        profile.presence_penalty,
        &mut omitted,
        &mut omitted_values,
    );

    let stop = match params.stop_sequences.as_deref() {
        Some(list) if !list.is_empty() => {
            if profile.stop_sequences.admits(list.len()) {
                Some(list.to_vec())
            } else {
                omitted.push(SamplingParam::StopSequences);
                omitted_values.push(format!("{} sequence(s)", list.len()));
                None
            }
        }
        _ => None,
    };

    Applied {
        temperature,
        top_p,
        frequency_penalty,
        presence_penalty,
        stop,
        omitted,
        omitted_values,
    }
}

/// One numeric parameter: pass it through when set and admitted, record it when set and not.
fn admit_numeric(
    param: SamplingParam,
    value: Option<f32>,
    support: ParamSupport,
    omitted: &mut Vec<SamplingParam>,
    omitted_values: &mut Vec<String>,
) -> Option<f32> {
    let value = value?;
    if support.admits(value) {
        Some(value)
    } else {
        omitted.push(param);
        omitted_values.push(value.to_string());
        None
    }
}

/// Once-per-instance record of which (model, parameter) omissions have already been logged.
///
/// One lives on each adapter instance, so a deployment with two instances warns once per
/// instance, and a hot loop of identical requests warns once in total.
#[derive(Debug, Default)]
pub(crate) struct OmissionLog {
    warned: Mutex<HashSet<(String, SamplingParam)>>,
}

impl OmissionLog {
    /// An empty log.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record the omissions in `applied` for `provider` and `model`.
    ///
    /// Emits one `log::warn!` (target `paladin::sampling`) the first time this instance omits a
    /// given (model, parameter), naming provider, model, parameter and value. Returns the
    /// comma-joined parameter names for [`PARAMS_OMITTED_METADATA_KEY`] **every** time something
    /// was omitted, and `None` when nothing was.
    pub(crate) fn record(&self, provider: &str, model: &str, applied: &Applied) -> Option<String> {
        if applied.omitted.is_empty() {
            return None;
        }

        // A poisoned lock only means another thread panicked while holding it; the set it guards
        // is still a valid set of "already warned" keys, so recover it rather than panic.
        let mut warned = self
            .warned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        for (index, param) in applied.omitted.iter().enumerate() {
            if warned.insert((model.to_string(), *param)) {
                let value = applied
                    .omitted_values
                    .get(index)
                    .map_or("(unrecorded)", String::as_str);
                log::warn!(
                    target: "paladin::sampling",
                    "provider '{provider}' model '{model}': parameter '{}' = {value} is not \
                     accepted by this model's profile and was omitted from the request \
                     (ADR-0061); it is recorded in the response metadata under '{}'",
                    param.as_str(),
                    PARAMS_OMITTED_METADATA_KEY,
                );
            }
        }

        Some(
            applied
                .omitted
                .iter()
                .map(SamplingParam::as_str)
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

/// Decide which `response_format` reaches the wire (ADR-0061 (i)), given what the caller
/// requested and what the profile says the model supports.
///
/// Returns the effective format and, when it differs from the request, the adjustment name
/// (`json_object` for a downgrade, `omitted` for a drop) used for the `warn!` and the metadata
/// value.
///
/// - no requested format: `(None, None)`
/// - `JsonSchema` and the profile's `json_schema` is true: sent as is
/// - `JsonSchema` and only `json_object` is true: downgraded to `JsonObject`, adjustment
///   `json_object`
/// - `JsonObject` and `json_object` is true: sent as is
/// - anything the profile cannot honour: omitted, adjustment `omitted`
pub(crate) fn apply_response_format(
    profile: &ModelProfile,
    requested: Option<&ResponseFormat>,
) -> (Option<ResponseFormat>, Option<&'static str>) {
    match requested {
        None => (None, None),
        Some(format @ ResponseFormat::JsonSchema { .. }) => {
            if profile.json_schema {
                (Some(format.clone()), None)
            } else if profile.json_object {
                (Some(ResponseFormat::JsonObject), Some("json_object"))
            } else {
                (None, Some("omitted"))
            }
        }
        Some(ResponseFormat::JsonObject) => {
            if profile.json_object {
                (Some(ResponseFormat::JsonObject), None)
            } else {
                (None, Some("omitted"))
            }
        }
        // `ResponseFormat` is `#[non_exhaustive]`: a variant added later is one this profile
        // knows nothing about, so it is not sent.
        Some(_) => (None, Some("omitted")),
    }
}

/// Once-per-instance record of which (model, adjustment) `response_format` changes have already
/// been logged. The `response_format` twin of [`OmissionLog`].
#[derive(Debug, Default)]
pub(crate) struct FormatLog {
    warned: Mutex<HashSet<(String, &'static str)>>,
}

impl FormatLog {
    /// An empty log.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record an adjustment (`json_object` or `omitted`) for `provider` and `model`.
    ///
    /// Emits one `log::warn!` (target `paladin::sampling`) the first time this instance makes a
    /// given adjustment for a model, and returns the value for
    /// [`RESPONSE_FORMAT_ADJUSTED_METADATA_KEY`] every time; `None` when `adjustment` is `None`.
    pub(crate) fn record(
        &self,
        provider: &str,
        model: &str,
        adjustment: Option<&'static str>,
    ) -> Option<String> {
        let adjustment = adjustment?;

        let mut warned = self
            .warned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if warned.insert((model.to_string(), adjustment)) {
            let what = if adjustment == "omitted" {
                "was not sent because the model's profile supports no JSON mode"
            } else {
                "was sent as a plain JSON object because the model's profile supports no JSON \
                 schema mode"
            };
            log::warn!(
                target: "paladin::sampling",
                "provider '{provider}' model '{model}': the requested response_format {what} \
                 (ADR-0061); it is recorded in the response metadata under '{}'",
                RESPONSE_FORMAT_ADJUSTED_METADATA_KEY,
            );
        }

        Some(adjustment.to_string())
    }
}

/// Refuse an effective `max_tokens` above the model's profiled maximum output (ADR-0061 (i)).
///
/// Never clamps: a request for more than the model can produce is an error naming the provider,
/// the model, `max_tokens`, the value and the limit, returned before anything is sent. A profile
/// with no known limit, or a request with no `max_tokens`, passes.
///
/// # Errors
///
/// [`LlmError::InvalidPrompt`] when `max_tokens` exceeds `profile.max_output_tokens`.
pub(crate) fn check_max_tokens(
    provider: &str,
    model: &str,
    profile: &ModelProfile,
    max_tokens: Option<u32>,
) -> Result<(), LlmError> {
    match (max_tokens, profile.max_output_tokens) {
        (Some(requested), Some(limit)) if requested > limit => {
            Err(LlmError::InvalidPrompt(format!(
                "model '{model}' on provider '{provider}' does not accept max_tokens = {requested}: \
             its maximum output is {limit} tokens. Lower max_tokens to {limit} or less, or, if \
             the limit is wrong for your deployment, correct it with a models.{model} override \
             (ADR-0061)"
            )))
        }
        _ => Ok(()),
    }
}

/// What an adapter's request builder decided beyond the body itself, so the buffered path can
/// record it on the response (the ADR-0061 visibility rule) and the streaming path can drop it
/// after the `warn!` has been emitted.
///
/// One type for every adapter -- the compat engine and the four bespoke wires -- so the five
/// metadata keys are stamped identically everywhere. The last two are the reasoning-effort keys
/// (Phase 51, D-11); they are set only when the request set an effort, so an unset effort
/// leaves the metadata exactly as it was.
#[derive(Debug, Default)]
pub(crate) struct RequestNotes {
    /// Comma-joined omitted parameter names, when any ([`PARAMS_OMITTED_METADATA_KEY`]).
    pub params_omitted: Option<String>,
    /// `json_object` or `omitted`, when the requested `response_format` was changed
    /// ([`RESPONSE_FORMAT_ADJUSTED_METADATA_KEY`]).
    pub format_adjusted: Option<String>,
    /// The request's model, when it has neither a seeded entry nor an override
    /// ([`UNPROFILED_MODEL_METADATA_KEY`]).
    pub unprofiled_model: Option<String>,
    /// The reasoning effort the request asked for, in its `Display` spelling, when it set one
    /// ([`REASONING_REQUESTED_METADATA_KEY`]).
    pub reasoning_requested: Option<String>,
    /// The reasoning effort the adapter acted on, in its `Display` spelling, when the request
    /// set one ([`REASONING_EFFECTIVE_METADATA_KEY`]). Differs from the requested value only when
    /// the caller chose a substituting fallback.
    pub reasoning_effective: Option<String>,
}

impl RequestNotes {
    /// The response-metadata entries these notes call for; empty when nothing was adjusted.
    pub(crate) fn into_metadata(self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        if let Some(omitted) = self.params_omitted {
            metadata.insert(PARAMS_OMITTED_METADATA_KEY.to_string(), omitted);
        }
        if let Some(adjusted) = self.format_adjusted {
            metadata.insert(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY.to_string(), adjusted);
        }
        if let Some(model) = self.unprofiled_model {
            metadata.insert(UNPROFILED_MODEL_METADATA_KEY.to_string(), model);
        }
        if let Some(requested) = self.reasoning_requested {
            metadata.insert(REASONING_REQUESTED_METADATA_KEY.to_string(), requested);
        }
        if let Some(effective) = self.reasoning_effective {
            metadata.insert(REASONING_EFFECTIVE_METADATA_KEY.to_string(), effective);
        }
        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_ports::output::model_profile::StopSupport;
    use serde_json::json;

    fn params() -> PromptParameters {
        PromptParameters {
            max_tokens: None,
            temperature: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
        }
    }

    fn open_profile() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_stop_sequences(StopSupport::Supported { max: None })
    }

    #[test]
    fn apply_sends_an_admitted_temperature_and_omits_nothing() {
        let mut p = params();
        p.temperature = Some(0.5);
        let applied = apply(&open_profile(), &p);
        assert_eq!(applied.temperature, Some(0.5));
        assert!(applied.omitted.is_empty());
    }

    #[test]
    fn apply_omits_an_out_of_range_temperature_and_names_it() {
        let mut p = params();
        p.temperature = Some(1.5);
        let applied = apply(&open_profile(), &p);
        assert_eq!(applied.temperature, None);
        assert_eq!(applied.omitted, vec![SamplingParam::Temperature]);
        assert_eq!(applied.omitted_values, vec!["1.5".to_string()]);
    }

    #[test]
    fn apply_never_omits_or_defaults_an_unset_value() {
        let applied = apply(&ModelProfile::new(), &params());
        assert_eq!(applied.temperature, None);
        assert_eq!(applied.top_p, None);
        assert_eq!(applied.frequency_penalty, None);
        assert_eq!(applied.presence_penalty, None);
        assert_eq!(applied.stop, None);
        assert!(
            applied.omitted.is_empty(),
            "nothing was set, nothing was omitted"
        );
    }

    #[test]
    fn apply_fixed_admits_only_the_exact_value() {
        let profile = open_profile().with_temperature(ParamSupport::Fixed(1.0));

        let mut exact = params();
        exact.temperature = Some(1.0);
        assert_eq!(apply(&profile, &exact).temperature, Some(1.0));

        let mut other = params();
        other.temperature = Some(0.85);
        let applied = apply(&profile, &other);
        assert_eq!(
            applied.temperature, None,
            "0.85 must be omitted, never swapped for the fixed 1.0"
        );
        assert_eq!(applied.omitted, vec![SamplingParam::Temperature]);
    }

    #[test]
    fn apply_not_sent_omits_any_set_value() {
        let profile = open_profile().with_top_p(ParamSupport::NotSent);
        let mut p = params();
        p.top_p = Some(0.0);
        let applied = apply(&profile, &p);
        assert_eq!(applied.top_p, None);
        assert_eq!(applied.omitted, vec![SamplingParam::TopP]);
    }

    #[test]
    fn apply_omission_order_is_the_fixed_parameter_order() {
        let profile = ModelProfile::new();
        let mut p = params();
        p.stop_sequences = Some(vec!["END".into()]);
        p.presence_penalty = Some(0.0);
        p.top_p = Some(1.0);
        p.temperature = Some(0.7);
        p.frequency_penalty = Some(0.0);
        let applied = apply(&profile, &p);
        assert_eq!(
            applied.omitted,
            vec![
                SamplingParam::Temperature,
                SamplingParam::TopP,
                SamplingParam::FrequencyPenalty,
                SamplingParam::PresencePenalty,
                SamplingParam::StopSequences,
            ]
        );
    }

    #[test]
    fn apply_stop_within_the_maximum_is_sent_in_order() {
        let profile = open_profile().with_stop_sequences(StopSupport::Supported { max: Some(4) });
        let mut p = params();
        p.stop_sequences = Some(vec!["d".into(), "a".into(), "a".into(), "c".into()]);
        let applied = apply(&profile, &p);
        assert_eq!(
            applied.stop,
            Some(vec!["d".to_string(), "a".into(), "a".into(), "c".into()]),
            "order and duplicates are preserved, nothing is sorted or deduplicated"
        );
        assert!(applied.omitted.is_empty());
    }

    #[test]
    fn apply_stop_above_the_maximum_is_omitted() {
        let profile = open_profile().with_stop_sequences(StopSupport::Supported { max: Some(4) });
        let mut p = params();
        p.stop_sequences = Some((0..5).map(|n| n.to_string()).collect());
        let applied = apply(&profile, &p);
        assert_eq!(applied.stop, None);
        assert_eq!(applied.omitted, vec![SamplingParam::StopSequences]);
    }

    #[test]
    fn apply_stop_not_sent_omits_a_single_sequence() {
        let profile = open_profile().with_stop_sequences(StopSupport::NotSent);
        let mut p = params();
        p.stop_sequences = Some(vec!["END".into()]);
        let applied = apply(&profile, &p);
        assert_eq!(applied.stop, None);
        assert_eq!(applied.omitted, vec![SamplingParam::StopSequences]);
    }

    #[test]
    fn apply_an_empty_stop_list_is_unset() {
        let profile = open_profile().with_stop_sequences(StopSupport::NotSent);
        let mut p = params();
        p.stop_sequences = Some(Vec::new());
        let applied = apply(&profile, &p);
        assert_eq!(applied.stop, None);
        assert!(applied.omitted.is_empty(), "an empty list is not a request");
    }

    #[test]
    fn omission_log_warns_once_per_model_and_param() {
        let log = OmissionLog::new();
        let mut p = params();
        p.temperature = Some(1.5);
        let applied = apply(&open_profile(), &p);

        // The metadata value is returned on every call; the warning is deduplicated.
        for _ in 0..3 {
            assert_eq!(
                log.record("kimi", "m1", &applied),
                Some("temperature".to_string())
            );
        }
        assert_eq!(
            log.warned.lock().unwrap().len(),
            1,
            "three calls for one (model, parameter) are one logged key"
        );

        let _ = log.record("kimi", "m2", &applied);
        assert_eq!(
            log.warned.lock().unwrap().len(),
            2,
            "a second model is its own key"
        );
    }

    #[test]
    fn omission_log_returns_none_when_nothing_was_omitted() {
        let log = OmissionLog::new();
        let applied = apply(&open_profile(), &params());
        assert_eq!(log.record("kimi", "m1", &applied), None);
        assert!(log.warned.lock().unwrap().is_empty());
    }

    #[test]
    fn omission_log_value_joins_names_in_fixed_order() {
        let log = OmissionLog::new();
        let mut p = params();
        p.top_p = Some(0.5);
        p.temperature = Some(0.5);
        p.stop_sequences = Some(vec!["x".into()]);
        let applied = apply(&ModelProfile::new(), &p);
        assert_eq!(
            log.record("grok", "m", &applied),
            Some("temperature,top_p,stop".to_string())
        );
    }

    fn schema() -> ResponseFormat {
        ResponseFormat::JsonSchema {
            name: "answer".into(),
            schema: json!({"type": "object"}),
            strict: true,
        }
    }

    #[test]
    fn apply_response_format_sends_a_schema_the_profile_supports() {
        let profile = ModelProfile::new()
            .with_json_object(true)
            .with_json_schema(true);
        let (format, adjustment) = apply_response_format(&profile, Some(&schema()));
        assert_eq!(format, Some(schema()));
        assert_eq!(adjustment, None);
    }

    #[test]
    fn apply_response_format_downgrades_a_schema_to_an_object() {
        let profile = ModelProfile::new().with_json_object(true);
        let (format, adjustment) = apply_response_format(&profile, Some(&schema()));
        assert_eq!(format, Some(ResponseFormat::JsonObject));
        assert_eq!(adjustment, Some("json_object"));
    }

    #[test]
    fn apply_response_format_omits_a_schema_when_no_json_mode_exists() {
        let (format, adjustment) = apply_response_format(&ModelProfile::new(), Some(&schema()));
        assert_eq!(format, None);
        assert_eq!(adjustment, Some("omitted"));
    }

    #[test]
    fn apply_response_format_sends_an_object_only_when_supported() {
        let supported = ModelProfile::new().with_json_object(true);
        let (format, adjustment) =
            apply_response_format(&supported, Some(&ResponseFormat::JsonObject));
        assert_eq!(format, Some(ResponseFormat::JsonObject));
        assert_eq!(adjustment, None);

        let (format, adjustment) =
            apply_response_format(&ModelProfile::new(), Some(&ResponseFormat::JsonObject));
        assert_eq!(format, None);
        assert_eq!(adjustment, Some("omitted"));
    }

    #[test]
    fn apply_response_format_without_a_request_adjusts_nothing() {
        let (format, adjustment) = apply_response_format(&ModelProfile::new(), None);
        assert_eq!(format, None);
        assert_eq!(adjustment, None);
    }

    #[test]
    fn format_log_warns_once_and_stamps_every_time() {
        let log = FormatLog::new();
        for _ in 0..3 {
            assert_eq!(
                log.record("kimi", "m", Some("json_object")),
                Some("json_object".to_string())
            );
        }
        assert_eq!(log.warned.lock().unwrap().len(), 1);
        assert_eq!(log.record("kimi", "m", None), None);
        assert_eq!(log.warned.lock().unwrap().len(), 1);
    }

    #[test]
    fn check_max_tokens_passes_at_the_limit_and_refuses_above_it() {
        let profile = ModelProfile::new().with_max_output_tokens(8192);
        assert!(check_max_tokens("kimi", "m", &profile, Some(8192)).is_ok());

        let Err(LlmError::InvalidPrompt(message)) =
            check_max_tokens("kimi", "m", &profile, Some(8193))
        else {
            panic!("8193 above a limit of 8192 must be InvalidPrompt");
        };
        for needle in ["kimi", "'m'", "max_tokens", "8193", "8192"] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
    }

    #[test]
    fn check_max_tokens_passes_without_a_limit_or_without_a_request() {
        let limited = ModelProfile::new().with_max_output_tokens(10);
        assert!(check_max_tokens("p", "m", &limited, None).is_ok());
        assert!(check_max_tokens("p", "m", &ModelProfile::new(), Some(u32::MAX)).is_ok());
    }
}
