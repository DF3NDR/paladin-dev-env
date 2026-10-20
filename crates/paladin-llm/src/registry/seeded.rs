//! The dated per-vendor model tables (ADR-0061 (c), Phase 50 D-05 and D-08).
//!
//! # Single source
//!
//! Every value in this file comes from one of two documents, both operator live seed probes run
//! on the operator's own vendor keys:
//!
//! - `.planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.md`, run at
//!   commit `c33022b9` on **2026-10-15** (the transcript's UTC date line, 22:23:15Z).
//! - `.planning/phases/50.1-consumer-surface-readiness/50.1-SEED-PROBE.md`, two runs at commit
//!   `8ad298dd` on **2026-10-20** (UTC date lines 15:20:29Z and 15:55:58Z). Run 1 probed the
//!   current model range of all seven providers; run 2 re-ran only OpenAI, Qwen and Grok to retry
//!   five candidates, and its Anthropic, DeepSeek, Kimi and Gemini sections read SKIPPED by design.
//!
//! Each entry's `verified_on` date says which transcript its values come from. In both, seven of
//! the nine providers were PROBED (OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini); Ollama
//! and OpenAI-compatible were SKIPPED (no daemon and no endpoint were configured), so the Ollama
//! table is empty and the OpenAI-compatible adapter has none.
//!
//! **A value with no evidence in that transcript is not written here.** Not from a research
//! note, not from a hand-off, not from memory. A field the transcript does not evidence keeps
//! its fail-closed [`ModelProfile::new`] value; a model with no evidence of its own is not
//! listed (it then runs on the adapter-wide fallback, visibly unprofiled, D-07).
//!
//! A seeded entry *replaces* the adapter-wide fallback for its model (it is not laid over it),
//! which is why a model is listed only when it was measured end to end: a half-measured entry
//! would silently turn every unmeasured field into a refusal.
//!
//! # How a probe turns into a value
//!
//! - `temperature`: [`ParamSupport::Range`]`(lo, hi)` when both probed endpoints were accepted
//!   and the next value outside was rejected; [`ParamSupport::Fixed`]`(v)` when only `v` was
//!   accepted; [`ParamSupport::NotSent`] when every probed value was rejected. Where the
//!   transcript cannot place the upper bound by measurement (OpenAI accepted the value *beyond*
//!   its declared end; Grok's high-end probe was unreachable) the entry says so in a comment and
//!   states the bound the transcript does support: the highest accepted endpoint for OpenAI,
//!   the vendor's own observed "less than 2" rejection text for Grok.
//! - `top_p` and the two penalties were probed at `0.5` only: an accepted value is seeded as
//!   the vendor contract's span (`0..=1`, `-2..=2`) and is marked *contract-derived* below; a
//!   rejected one is `NotSent`. A penalty is also `NotSent` when the adapter's own request type
//!   has no field to carry it (the wire cannot send what the profile would admit).
//! - `stop`: unbounded when five sequences were accepted; the documented maximum from the
//!   operator's evidence when five were rejected; `NotSent` when one was rejected.
//! - `reasoning`: the default call returned reasoning text, or the listing says so.
//! - `json_object` / `json_schema`: the matching JSON probe was accepted.
//! - context window and maximum output: the live listing where it exposed them, otherwise the
//!   operator's vendor-documentation evidence, with its URL and access date next to the value.
//!
//! The per-vendor tables are compiled with their adapter's feature, so a build without that
//! adapter carries no unused table.

#[cfg(feature = "anthropic")]
mod anthropic_table {
    use crate::registry::SeededProfile;
    use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};

    /// Anthropic. `claude-haiku-5-5` (2026-10-15, `50-SEED-PROBE.md`) and `claude-opus-4-8`
    /// (2026-10-20, `50.1-SEED-PROBE.md`) were measured end to end. The eight curated legacy ids
    /// (claude-3 family, claude-2.x, claude-instant) all answered 404 `not_found_error` (retired,
    /// not seeded), and the other current ids in the live listing have not been probed, so their
    /// sampling behaviour is unknown and they are not listed.
    pub(crate) static ANTHROPIC: &[SeededProfile] = &[
        SeededProfile {
            model: "claude-haiku-5-5",
            verified_on: "2026-10-15",
            build: claude_haiku_5_5,
        },
        SeededProfile {
            model: "claude-opus-4-8",
            verified_on: "2026-10-20",
            build: claude_opus_4_8,
        },
    ];

    fn claude_haiku_5_5() -> ModelProfile {
        ModelProfile::new()
            // Only temperature 1.00 was accepted; 0.00 and 0.50 answered "`temperature` is
            // deprecated for this model", 1.01 answered "temperature: range: 0..1".
            .with_temperature(ParamSupport::Fixed(1.0))
            // top_p 0.5 answered "`top_p` is deprecated for this model".
            .with_top_p(ParamSupport::NotSent)
            // The Messages API has no such request field (probe: NOT APPLICABLE).
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Five stop sequences were accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: max_input_tokens=1000000, max_tokens=128000.
            .with_context_window(1_000_000)
            .with_max_output_tokens(128_000)
            // The default call carried no reasoning text; the listing capabilities were
            // truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Anthropic has no JSON mode (probe: NOT APPLICABLE).
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-opus-4-8``, probe numbers as in that table, and its listing row.
    fn claude_opus_4_8() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2 and 4 (0.00, 0.50) answered
            // "`temperature` is deprecated for this model", probe 5 (1.01) "temperature: range:
            // 0..1". Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "`top_p` is deprecated for this model".
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: NOT APPLICABLE, the Messages API has no such request field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: one and five stop sequences were accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: max_input_tokens=1000000, max_tokens=128000.
            .with_context_window(1_000_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the listing capabilities
            // were truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }
}
#[cfg(feature = "anthropic")]
pub(crate) use anthropic_table::ANTHROPIC;

#[cfg(feature = "openai")]
mod openai_table {
    use crate::registry::SeededProfile;
    use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};

    /// OpenAI. The two candidates probed. Common to both:
    ///
    /// - temperature `0.00`, `0.50`, `1.00` and `1.01` were all accepted, so the upper bound is
    ///   not located by the transcript (OpenAI accepted the value beyond the declared end). It
    ///   is seeded at the highest probed endpoint, `1.0`, which the probe accepted.
    /// - `top_p` 0.5 was accepted: `0..=1` is contract-derived.
    /// - both penalties were accepted at 0.5, but this adapter's request type has no penalty
    ///   field, so the profile says `NotSent`: admitting a value the wire cannot carry would
    ///   drop it silently.
    pub(crate) static OPENAI: &[SeededProfile] = &[
        SeededProfile {
            model: "gpt-5.4-nano",
            verified_on: "2026-10-15",
            build: gpt_5_4_nano,
        },
        SeededProfile {
            model: "gpt-4o-mini",
            verified_on: "2026-10-15",
            build: gpt_4o_mini,
        },
    ];

    fn gpt_5_4_nano() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // "Unsupported parameter: 'stop' is not supported with this model" for one and
            // for five sequences.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.4-nano (accessed
            // 2026-10-15): 400,000 context window, 128,000 max output.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // No reasoning text in the default call; the `reasoning_tokens` usage key is
            // present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            .with_json_object(true)
            .with_json_schema(true)
    }

    fn gpt_4o_mini() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Five sequences were rejected: "Expected an array with maximum length 4"
            // (`array_above_max_length`), so the maximum is 4.
            .with_stop_sequences(StopSupport::Supported { max: Some(4) })
            // docs https://developers.openai.com/api/docs/models/gpt-4o-mini (accessed
            // 2026-10-15): 128,000 context window, 16,384 max output.
            .with_context_window(128_000)
            .with_max_output_tokens(16_384)
            .with_reasoning(false)
            .with_json_object(true)
            .with_json_schema(true)
    }
}
#[cfg(feature = "openai")]
pub(crate) use openai_table::OPENAI;

#[cfg(feature = "deepseek")]
mod deepseek_table {
    use crate::registry::SeededProfile;
    use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};

    /// DeepSeek. `deepseek-flash` is the live listing's general-purpose model (2026-10-15,
    /// `50-SEED-PROBE.md`); `deepseek-v4-pro` is its larger sibling, listed and probed on
    /// 2026-10-20 (`50.1-SEED-PROBE.md`, run 1). `deepseek-chat` and `deepseek-coder` answered
    /// 200 but are absent from the listing and from the vendor's pricing page (the operator
    /// classes them as undocumented aliases): not seeded.
    pub(crate) static DEEPSEEK: &[SeededProfile] = &[
        SeededProfile {
            model: "deepseek-flash",
            verified_on: "2026-10-15",
            build: deepseek_flash,
        },
        SeededProfile {
            model: "deepseek-v4-pro",
            verified_on: "2026-10-20",
            build: deepseek_v4_pro,
        },
    ];

    fn deepseek_flash() -> ModelProfile {
        ModelProfile::new()
            // 0.00 and 2.00 accepted, 2.01 answered "the valid range of temperature is [0, 2]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Both penalties accepted at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: context_window=1048576, max_output_tokens=393216.
            .with_context_window(1_048_576)
            .with_max_output_tokens(393_216)
            // The default call returned reasoning text.
            .with_reasoning(true)
            .with_json_object(true)
            // "This response_format type is unavailable now".
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `deepseek-v4-pro``, probe numbers as in that table, and its listing row.
    fn deepseek_v4_pro() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered "the valid range of
            // temperature is [0, 2]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties accepted at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: context_window=1048576, max_output_tokens=393216.
            .with_context_window(1_048_576)
            .with_max_output_tokens(393_216)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probe 12: json_object accepted.
            .with_json_object(true)
            // Probe 13: "This response_format type is unavailable now".
            .with_json_schema(false)
    }
}
#[cfg(feature = "deepseek")]
pub(crate) use deepseek_table::DEEPSEEK;

#[cfg(feature = "kimi")]
mod kimi_table {
    use crate::registry::SeededProfile;
    use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};

    /// Kimi. `kimi-k3` and `kimi-k2.6` were measured. `kimi-k2.7-code` and
    /// `kimi-k2.7-code-highspeed` (listing: 262,144-token window, reasoning) rejected every
    /// probe that disables thinking ("only type=enabled is allowed for this model"), so no
    /// sampling, stop or JSON fact was measured for them and they are not seeded.
    ///
    /// Every probed `temperature` (1.00, 0.50, 1.01) was rejected ("only 0.6 is allowed for this
    /// model"), so temperature is `NotSent`. The allowed value is stated for the thinking-disabled
    /// request the probes sent, and an earlier measurement named a different single value, so the
    /// profile does not pin one: the vendor applies its own legal value to an omitted field. The
    /// `top_p` probe (0.5) and both penalty probes were rejected too ("only 0.95", "only 0 is
    /// allowed"), hence `NotSent`.
    pub(crate) static KIMI: &[SeededProfile] = &[
        SeededProfile {
            model: "kimi-k2.6",
            verified_on: "2026-10-15",
            build: kimi_k2_6,
        },
        SeededProfile {
            model: "kimi-k3",
            verified_on: "2026-10-15",
            build: kimi_k3,
        },
    ];

    fn kimi_k2_6() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::NotSent)
            .with_top_p(ParamSupport::NotSent)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing context_length=262144 (docs "256K"); no maximum output is documented
            // (https://platform.kimi.ai/docs/guide/choosing-an-appropriate-kimi-model-for-your-tasks,
            // accessed 2026-10-15), so none is seeded.
            .with_context_window(262_144)
            // The default (thinking) call returned reasoning text.
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(true)
    }

    fn kimi_k3() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::NotSent)
            .with_top_p(ParamSupport::NotSent)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing context_length=1048576 (docs "1M-token context window", same page as
            // above, accessed 2026-10-15); no maximum output is documented.
            .with_context_window(1_048_576)
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(true)
    }
}
#[cfg(feature = "kimi")]
pub(crate) use kimi_table::KIMI;

#[cfg(feature = "qwen")]
mod qwen_table {
    use crate::registry::SeededProfile;
    use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};

    /// Qwen. `qwen3.8-flash` and `qwen-plus` were measured. `qwen-turbo` answered 429
    /// `limit_requests` on every probe (a rate limit, nothing measured) and `qwen3-max` answered
    /// 404 "Model not exist." on the International endpoint: neither is seeded.
    pub(crate) static QWEN: &[SeededProfile] = &[
        SeededProfile {
            model: "qwen3.8-flash",
            verified_on: "2026-10-15",
            build: qwen3_8_flash,
        },
        SeededProfile {
            model: "qwen-plus",
            verified_on: "2026-10-15",
            build: qwen_plus,
        },
    ];

    fn qwen3_8_flash() -> ModelProfile {
        ModelProfile::new()
            // 0.00 and 1.99 accepted, 2.00 answered "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Both penalties accepted at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://help.aliyun.com/en/model-studio/qwen3-8-flash (China/Beijing page,
            // accessed 2026-10-15): 1,000,000 context window, 131,072 max output (thinking
            // mode). The International pages state no limits.
            .with_context_window(1_000_000)
            .with_max_output_tokens(131_072)
            // The default call returned reasoning text.
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(true)
    }

    fn qwen_plus() -> ModelProfile {
        // No context window and no maximum output: the listing exposes neither and the vendor
        // pricing page lists only input price tiers (docs
        // https://www.alibabacloud.com/help/en/model-studio/model-pricing, accessed 2026-10-15).
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_stop_sequences(StopSupport::Supported { max: None })
            // No reasoning text in the default call.
            .with_reasoning(false)
            .with_json_object(true)
            .with_json_schema(true)
    }
}
#[cfg(feature = "qwen")]
pub(crate) use qwen_table::QWEN;

#[cfg(feature = "grok")]
mod grok_table {
    use crate::registry::SeededProfile;
    use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};

    /// Grok. `grok-4.3`, `grok-4.5` and `grok-4.6` were measured. The other listed chat ids
    /// (the dated `grok-4.20-*` family, `grok-4.7`, `grok-build-0.1`) were not probed, and the
    /// image and video ids are not chat models: none is seeded.
    ///
    /// Common to all three:
    ///
    /// - temperature: `0.00` and `0.50` were accepted and `2.01` was rejected ("Temperature must
    ///   be less than 2 but temperature = 2.01"), but the high-end probe (`2.00`) was UNREACHABLE
    ///   (a transport error, three times), so the upper bound itself was never measured. The
    ///   vendor's own rejection text states the bound is strictly below 2, so the range is
    ///   seeded as `0.0..=1.99`, the largest value under it, the same half-open shape the Qwen
    ///   probe measured at 1.99. Leaving temperature `NotSent` instead would drop a parameter the
    ///   probe showed Grok accepts. Re-running the Grok probe turns the stated bound into a
    ///   measured one.
    /// - `top_p` 0.5 was accepted: `0..=1` is contract-derived.
    /// - both penalties and `stop` (one and five sequences) were rejected ("does not support
    ///   parameter ...").
    pub(crate) static GROK: &[SeededProfile] = &[
        SeededProfile {
            model: "grok-4.3",
            verified_on: "2026-10-15",
            build: grok_4_3,
        },
        SeededProfile {
            model: "grok-4.6",
            verified_on: "2026-10-15",
            build: grok_4_6,
        },
        SeededProfile {
            model: "grok-4.5",
            verified_on: "2026-10-15",
            build: grok_4_5,
        },
    ];

    fn grok_4_3() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            .with_stop_sequences(StopSupport::NotSent)
            // Listing context_length=1000000 (docs "1M", https://docs.x.ai/docs/models,
            // accessed 2026-10-15); no maximum output is documented.
            .with_context_window(1_000_000)
            // The default call returned reasoning text.
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(true)
    }

    fn grok_4_6() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            .with_stop_sequences(StopSupport::NotSent)
            // Listing context_length=500000 (docs "500k", https://docs.x.ai/docs/models,
            // accessed 2026-10-15); no maximum output is documented.
            .with_context_window(500_000)
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(true)
    }

    fn grok_4_5() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            .with_stop_sequences(StopSupport::NotSent)
            // Listing context_length=500000 (docs "500k", same source and date).
            .with_context_window(500_000)
            // The default call carried no reasoning text, but the listing declares
            // capabilities.reasoning_effort (default "high"), which says so.
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(true)
    }
}
#[cfg(feature = "grok")]
pub(crate) use grok_table::GROK;

#[cfg(feature = "gemini")]
mod gemini_table {
    use crate::registry::SeededProfile;
    use paladin_ports::output::model_profile::{ModelProfile, ParamSupport, StopSupport};

    /// Gemini. `gemini-3.5-flash-lite`, `gemini-3.5-flash` and `gemini-3.6-flash` were
    /// measured. Common to all three: temperature `0.00` and `2.00` accepted and `2.01`
    /// rejected ("temperature must be in the range [0.0, 2.0]"); `top_p` 0.5 accepted
    /// (`0..=1` is contract-derived); the `generateContent` wire has no penalty field (probe: NOT
    /// APPLICABLE), so both penalties are `NotSent`; five stop sequences accepted; JSON object
    /// (`responseMimeType`) and JSON schema (`responseSchema`) both accepted; the listing
    /// reports `inputTokenLimit=1048576`, `outputTokenLimit=65536` and `thinking=true`.
    pub(crate) static GEMINI: &[SeededProfile] = &[
        SeededProfile {
            model: "gemini-3.5-flash-lite",
            verified_on: "2026-10-15",
            build: gemini_3_5_flash_lite,
        },
        SeededProfile {
            model: "gemini-3.6-flash",
            verified_on: "2026-10-15",
            build: gemini_3_6_flash,
        },
        SeededProfile {
            model: "gemini-3.5-flash",
            verified_on: "2026-10-15",
            build: gemini_3_5_flash,
        },
    ];

    fn measured() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            .with_stop_sequences(StopSupport::Supported { max: None })
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            .with_reasoning(true)
            .with_json_object(true)
            .with_json_schema(true)
    }

    fn gemini_3_5_flash_lite() -> ModelProfile {
        measured()
    }

    fn gemini_3_6_flash() -> ModelProfile {
        measured()
    }

    fn gemini_3_5_flash() -> ModelProfile {
        measured()
    }
}
#[cfg(feature = "gemini")]
pub(crate) use gemini_table::GEMINI;

#[cfg(feature = "ollama")]
mod ollama_table {
    use crate::registry::SeededProfile;

    /// Ollama: empty. The 2026-10-15 probe SKIPPED it (`OLLAMA_BASE_URL` was not set, so no local
    /// daemon was configured), and Ollama serves whatever the operator pulled, so there is no
    /// model to seed. Every Ollama model runs on the adapter-wide fallback, visibly unprofiled.
    pub(crate) static OLLAMA: &[SeededProfile] = &[];
}
#[cfg(feature = "ollama")]
pub(crate) use ollama_table::OLLAMA;
