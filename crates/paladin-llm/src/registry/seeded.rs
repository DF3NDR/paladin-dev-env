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

    /// Anthropic. `claude-haiku-5-5` (2026-10-15, `50-SEED-PROBE.md`) and the ten current
    /// candidates of `50.1-SEED-PROBE.md` (2026-10-20, run 1: `claude-opus-4-8`,
    /// `claude-fable-5-1`, `claude-fable-5`, `claude-opus-5-5`, `claude-opus-5`,
    /// `claude-sonnet-5-5`, `claude-sonnet-5`, `claude-opus-4-7`, `claude-opus-4-6`,
    /// `claude-sonnet-4-6`) were each measured end to end. The eight curated legacy ids (claude-3
    /// family, claude-2.x, claude-instant) all answered 404 `not_found_error` (retired, not
    /// seeded), and the 4.5-generation ids in the live listing were not candidates and were not
    /// probed, so their sampling behaviour is unknown and they are not listed.
    ///
    /// Two shapes were measured. The 5.x ids, `claude-opus-4-8` and `claude-opus-4-7` accept only
    /// `temperature` 1.00 and refuse `top_p` ("is deprecated for this model"); `claude-opus-4-6`
    /// and `claude-sonnet-4-6` accept `temperature` 0..1 and `top_p`. Every row's window and
    /// maximum output are its listing row's `max_input_tokens` and `max_tokens`.
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
        SeededProfile {
            model: "claude-fable-5-1",
            verified_on: "2026-10-20",
            build: claude_fable_5_1,
        },
        SeededProfile {
            model: "claude-fable-5",
            verified_on: "2026-10-20",
            build: claude_fable_5,
        },
        SeededProfile {
            model: "claude-opus-5-5",
            verified_on: "2026-10-20",
            build: claude_opus_5_5,
        },
        SeededProfile {
            model: "claude-opus-5",
            verified_on: "2026-10-20",
            build: claude_opus_5,
        },
        SeededProfile {
            model: "claude-sonnet-5-5",
            verified_on: "2026-10-20",
            build: claude_sonnet_5_5,
        },
        SeededProfile {
            model: "claude-sonnet-5",
            verified_on: "2026-10-20",
            build: claude_sonnet_5,
        },
        SeededProfile {
            model: "claude-opus-4-7",
            verified_on: "2026-10-20",
            build: claude_opus_4_7,
        },
        SeededProfile {
            model: "claude-opus-4-6",
            verified_on: "2026-10-20",
            build: claude_opus_4_6,
        },
        SeededProfile {
            model: "claude-sonnet-4-6",
            verified_on: "2026-10-20",
            build: claude_sonnet_4_6,
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

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-fable-5-1``, probe numbers as in that table, and its listing row.
    fn claude_fable_5_1() -> ModelProfile {
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
            // Probe 1 (the default call) carried no reasoning text; the listing
            // capabilities were truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-fable-5``, probe numbers as in that table, and its listing row.
    fn claude_fable_5() -> ModelProfile {
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
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-opus-5-5``, probe numbers as in that table, and its listing row.
    fn claude_opus_5_5() -> ModelProfile {
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
            // Probe 1 (the default call) carried no reasoning text; the listing
            // capabilities were truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-opus-5``, probe numbers as in that table, and its listing row.
    fn claude_opus_5() -> ModelProfile {
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
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-sonnet-5-5``, probe numbers as in that table, and its listing row.
    fn claude_sonnet_5_5() -> ModelProfile {
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
            // Probe 1 (the default call) carried no reasoning text; the listing
            // capabilities were truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-sonnet-5``, probe numbers as in that table, and its listing row.
    fn claude_sonnet_5() -> ModelProfile {
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
            // Probe 1 (the default call) carried no reasoning text; the listing
            // capabilities were truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-opus-4-7``, probe numbers as in that table, and its listing row.
    fn claude_opus_4_7() -> ModelProfile {
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
            // Probe 1 (the default call) carried no reasoning text; the listing
            // capabilities were truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-opus-4-6``, probe numbers as in that table, and its listing row.
    fn claude_opus_4_6() -> ModelProfile {
        ModelProfile::new()
            // Probes 2, 3 and 4: temperature 0.00, 1.00 and 0.50 ACCEPTED; probe 5: 1.01
            // answered "temperature: range: 0..1".
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the Messages API has no such request field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: one and five stop sequences were accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: max_input_tokens=1000000, max_tokens=128000.
            .with_context_window(1_000_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the listing
            // capabilities were truncated in the transcript, so reasoning is not evidenced.
            .with_reasoning(false)
            // Probes 12 and 13: NOT APPLICABLE, Anthropic has no JSON mode.
            .with_json_object(false)
            .with_json_schema(false)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `claude-sonnet-4-6``, probe numbers as in that table, and its listing row.
    fn claude_sonnet_4_6() -> ModelProfile {
        ModelProfile::new()
            // Probes 2, 3 and 4: temperature 0.00, 1.00 and 0.50 ACCEPTED; probe 5: 1.01
            // answered "temperature: range: 0..1".
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the Messages API has no such request field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: one and five stop sequences were accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: max_input_tokens=1000000, max_tokens=128000.
            .with_context_window(1_000_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the listing
            // capabilities were truncated in the transcript, so reasoning is not evidenced.
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

    /// OpenAI. `gpt-5.4-nano` and `gpt-4o-mini` were measured on 2026-10-15
    /// (`50-SEED-PROBE.md`); 17 further ids were measured end to end on 2026-10-20
    /// (`50.1-SEED-PROBE.md`; run 1 except `gpt-6-sol` and `gpt-5`, taken from run 2, which
    /// supersedes run 1 for those two): `gpt-6.1-sol`, `gpt-6-sol`, `gpt-6-luna`, `gpt-6-astra`,
    /// `gpt-5.6-sol`, `gpt-5.6-luna`, `gpt-5.6-terra`, `gpt-5.5`, `gpt-5.4`, `gpt-5.4-mini`,
    /// `gpt-5.2`, `gpt-5.1`, `gpt-5`, `gpt-5-mini`, `gpt-5-nano`, `chat-latest` and `o3`. The
    /// four pro-tier ids (`gpt-5.5-pro`, `gpt-5.4-pro`, `gpt-5.2-pro`, `gpt-5-pro`) answered the
    /// default call 404 on `/v1/chat/completions` (they need the Responses API, a pending todo),
    /// so nothing was measured for them and they are not seeded. `chat-latest` is a moving alias:
    /// its row is true on its `verified_on` date only.
    ///
    /// Two sampling shapes were measured. The *fixed* group (`gpt-6.1-sol`, `gpt-6-sol`,
    /// `gpt-6-luna`, `gpt-6-astra`, `gpt-5.6-*`, `gpt-5.5`, `gpt-5`, `gpt-5-mini`, `gpt-5-nano`,
    /// `chat-latest`, `o3`) accepts `temperature` 1.00 only and refuses `top_p`; the *span* group
    /// (`gpt-5.4`, `gpt-5.4-mini`, `gpt-5.2`, `gpt-5.1`, and the earlier `gpt-5.4-nano` and
    /// `gpt-4o-mini`) accepts the whole probed span. Common to all:
    ///
    /// - temperature `0.00`, `0.50`, `1.00` and `1.01` were all accepted by the span group, so
    ///   the upper bound is not located by the transcript (OpenAI accepted the value beyond the
    ///   declared end). It is seeded at the highest probed endpoint, `1.0`, which the probe
    ///   accepted. The fixed group accepted `1.00` only ("Only the default (1) value is
    ///   supported"), so it is `Fixed(1.0)`.
    /// - `top_p` 0.5 was accepted by the span group: `0..=1` is contract-derived. The fixed
    ///   group refused it.
    /// - both penalties were accepted at 0.5 by the span group, but this adapter's request type
    ///   has no penalty field, so the profile says `NotSent`: admitting a value the wire cannot
    ///   carry would drop it silently. The fixed group refused them.
    /// - `stop` was refused for one and for five sequences by every id, so it is `NotSent`; the
    ///   documented maximum of 4 (operator evidence (c)) is only used for a model that rejected
    ///   five but took one, as `gpt-4o-mini` did.
    /// - the live listing exposes no window and no maximum output, so every window and maximum
    ///   output comes from the operator's docs evidence (b), URL and access date in a comment.
    /// - no default call returned reasoning text (Chat Completions never does), so `reasoning`
    ///   is `false` for every OpenAI row; a model that hides its reasoning can still return an
    ///   empty answer under a small output cap (MIGRATION section 9.8).
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
        SeededProfile {
            model: "gpt-6.1-sol",
            verified_on: "2026-10-20",
            build: gpt_6_1_sol,
        },
        SeededProfile {
            model: "gpt-6-sol",
            verified_on: "2026-10-20",
            build: gpt_6_sol,
        },
        SeededProfile {
            model: "gpt-6-luna",
            verified_on: "2026-10-20",
            build: gpt_6_luna,
        },
        SeededProfile {
            model: "gpt-6-astra",
            verified_on: "2026-10-20",
            build: gpt_6_astra,
        },
        SeededProfile {
            model: "gpt-5.6-sol",
            verified_on: "2026-10-20",
            build: gpt_5_6_sol,
        },
        SeededProfile {
            model: "gpt-5.6-luna",
            verified_on: "2026-10-20",
            build: gpt_5_6_luna,
        },
        SeededProfile {
            model: "gpt-5.6-terra",
            verified_on: "2026-10-20",
            build: gpt_5_6_terra,
        },
        SeededProfile {
            model: "gpt-5.5",
            verified_on: "2026-10-20",
            build: gpt_5_5,
        },
        SeededProfile {
            model: "gpt-5.4",
            verified_on: "2026-10-20",
            build: gpt_5_4,
        },
        SeededProfile {
            model: "gpt-5.4-mini",
            verified_on: "2026-10-20",
            build: gpt_5_4_mini,
        },
        SeededProfile {
            model: "gpt-5.2",
            verified_on: "2026-10-20",
            build: gpt_5_2,
        },
        SeededProfile {
            model: "gpt-5.1",
            verified_on: "2026-10-20",
            build: gpt_5_1,
        },
        SeededProfile {
            model: "gpt-5",
            verified_on: "2026-10-20",
            build: gpt_5,
        },
        SeededProfile {
            model: "gpt-5-mini",
            verified_on: "2026-10-20",
            build: gpt_5_mini,
        },
        SeededProfile {
            model: "gpt-5-nano",
            verified_on: "2026-10-20",
            build: gpt_5_nano,
        },
        SeededProfile {
            model: "chat-latest",
            verified_on: "2026-10-20",
            build: chat_latest,
        },
        SeededProfile {
            model: "o3",
            verified_on: "2026-10-20",
            build: o3,
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

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-6.1-sol``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_6_1_sol() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-6.1-sol (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 2 (2026-10-20), `### Probes:
    // `gpt-6-sol``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_6_sol() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-6-sol (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-6-luna``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_6_luna() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-6-luna (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-6-astra``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_6_astra() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-6-astra (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.6-sol``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_5_6_sol() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.6-sol (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.6-luna``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_5_6_luna() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.6-luna (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.6-terra``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_5_6_terra() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.6-terra (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.5``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_5_5() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.5 (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.4``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_5_4() -> ModelProfile {
        ModelProfile::new()
            // Probes 2-5: 0.00, 1.00, 0.50 and 1.01 all ACCEPTED, so the transcript does not locate
            // the upper bound; seeded at the highest probed endpoint, 1.0 (module docs).
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: 0.5 ACCEPTED, but this adapter's request type has no penalty field,
            // so both are `NotSent`: admitting a value the wire cannot carry would drop it.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.4 (accessed
            // 2026-10-20): 1,050,000 context window, 128,000 max output tokens.
            .with_context_window(1_050_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.4-mini``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_5_4_mini() -> ModelProfile {
        ModelProfile::new()
            // Probes 2-5: 0.00, 1.00, 0.50 and 1.01 all ACCEPTED, so the transcript does not locate
            // the upper bound; seeded at the highest probed endpoint, 1.0 (module docs).
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: 0.5 ACCEPTED, but this adapter's request type has no penalty field,
            // so both are `NotSent`: admitting a value the wire cannot carry would drop it.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.4-mini (accessed
            // 2026-10-20): 400,000 context window, 128,000 max output tokens.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.2``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn gpt_5_2() -> ModelProfile {
        ModelProfile::new()
            // Probes 2-5: 0.00, 1.00, 0.50 and 1.01 all ACCEPTED, so the transcript does not locate
            // the upper bound; seeded at the highest probed endpoint, 1.0 (module docs).
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: 0.5 ACCEPTED, but this adapter's request type has no penalty field,
            // so both are `NotSent`: admitting a value the wire cannot carry would drop it.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.2 (accessed
            // 2026-10-20): 400,000 context window, 128,000 max output tokens.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5.1``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Docs badge "Deprecated"; the live listing shows `shutdown_date=2027-04-01`, after the
    // D-15 cutoff, and the owner decided on 2026-10-20 that a measured deprecated id is seeded
    // (`50.1-PROBE-CANDIDATES.md`, "Owner decision for plan 50.1-09").
    fn gpt_5_1() -> ModelProfile {
        ModelProfile::new()
            // Probes 2-5: 0.00, 1.00, 0.50 and 1.01 all ACCEPTED, so the transcript does not locate
            // the upper bound; seeded at the highest probed endpoint, 1.0 (module docs).
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: 0.5 ACCEPTED, but this adapter's request type has no penalty field,
            // so both are `NotSent`: admitting a value the wire cannot carry would drop it.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5.1 (accessed
            // 2026-10-20): 400,000 context window, 128,000 max output tokens.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 2 (2026-10-20), `### Probes:
    // `gpt-5``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Docs badge "Deprecated"; the live listing shows `shutdown_date=null`, and the owner decided
    // on 2026-10-20 that a measured deprecated id is seeded (`50.1-PROBE-CANDIDATES.md`).
    // Probes 12 and 14 were ACCEPTED but finished `length` with EMPTY content: hidden reasoning
    // consumed the whole small output budget (MIGRATION section 9.8 names this).
    fn gpt_5() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5 (accessed
            // 2026-10-20): 400,000 context window, 128,000 max output tokens.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5-mini``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Docs badge "Deprecated"; the live listing shows `shutdown_date=null`, and the owner decided
    // on 2026-10-20 that a measured deprecated id is seeded (`50.1-PROBE-CANDIDATES.md`).
    // Probe 12 answered 400 "Could not finish the message because max_tokens or model output
    // limit was reached": the `max_completion_tokens` key itself was understood (probe 11
    // names it as the replacement), the budget was spent before any text.
    fn gpt_5_mini() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5-mini (accessed
            // 2026-10-20): 400,000 context window, 128,000 max output tokens.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gpt-5-nano``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Docs badge "Deprecated"; the live listing shows `shutdown_date=null`, and the owner decided
    // on 2026-10-20 that a measured deprecated id is seeded (`50.1-PROBE-CANDIDATES.md`).
    // Probes 12 and 14 were ACCEPTED but finished `length` with EMPTY content: hidden reasoning
    // consumed the whole small output budget (MIGRATION section 9.8 names this).
    fn gpt_5_nano() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/gpt-5-nano (accessed
            // 2026-10-20): 400,000 context window, 128,000 max output tokens.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `chat-latest``, probe numbers as in that table, and operator evidence (b) for the limits.
    // A moving alias ("points to the latest Instant model currently used in ChatGPT", docs): the
    // row is true on its `verified_on` date only.
    fn chat_latest() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/chat-latest (accessed
            // 2026-10-20): 400,000 context window, 128,000 max output tokens.
            .with_context_window(400_000)
            .with_max_output_tokens(128_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `o3``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Docs badge "Deprecated"; the live listing shows `shutdown_date=null`, and the owner decided
    // on 2026-10-20 that a measured deprecated id is seeded (`50.1-PROBE-CANDIDATES.md`).
    fn o3() -> ModelProfile {
        ModelProfile::new()
            // Probe 3: temperature 1.00 ACCEPTED; probes 2, 4 and 5 (0.00, 0.50, 1.01) answered
            // "Unsupported value: 'temperature' does not support <v> with this model. Only the
            // default (1) value is supported." Only 1.00 was accepted.
            .with_temperature(ParamSupport::Fixed(1.0))
            // Probe 6: top_p 0.5 answered "Unsupported parameter: 'top_p' is not supported with
            // this model."
            .with_top_p(ParamSupport::NotSent)
            // Probes 7 and 8: both penalties answered "Unsupported parameter: ... is not
            // supported with this model." (This adapter's request type has no penalty field.)
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: "Unsupported parameter: 'stop' is not supported with this model."
            // for one and for five sequences. The documented maximum of 4 (operator evidence (c))
            // does not apply: a model that rejects one sequence takes none.
            .with_stop_sequences(StopSupport::NotSent)
            // docs https://developers.openai.com/api/docs/models/o3 (accessed
            // 2026-10-20): 200,000 context window, 100,000 max output tokens.
            .with_context_window(200_000)
            .with_max_output_tokens(100_000)
            // Probe 1 (the default call) carried no reasoning text; the `reasoning_tokens`
            // usage key is present on every OpenAI model, so it is not evidence.
            .with_reasoning(false)
            // Probes 13 and 14: json_object and json_schema ACCEPTED.
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

    /// Qwen. `qwen3.8-flash` and `qwen-plus` were measured on 2026-10-15 (`50-SEED-PROBE.md`);
    /// 11 further ids were measured end to end on 2026-10-20 (`50.1-SEED-PROBE.md`; run 1
    /// except `qwen3.7-flash` and `qwen3.5-flash`, taken from run 2, which supersedes run 1 for
    /// those two): `qwen3.8-max`, `qwen3.7-plus`, `qwen3.7-max`, `qwen3.7-flash`, `qwen3.6-plus`,
    /// `qwen3.6-flash`, `qwen3.5-plus`, `qwen3.5-flash`, `qwen-max`, `qwen-flash` and
    /// `qwen-plus-latest` (a moving alias: true on its `verified_on` date only). They share one
    /// sampling shape, measured identically: temperature `0.00` and `1.99` accepted and `2.00`
    /// refused ("Temperature should be in [0.0, 2.0)"), `top_p` and both penalties accepted at
    /// 0.5 (contract-derived spans), five stop sequences accepted, both JSON modes accepted. The
    /// listing exposes no limit, so each window and maximum output is the operator's docs
    /// evidence (b) from the Alibaba Cloud International Model Info page (URL and access date in
    /// a comment). `qwen-turbo` answered 429 `limit_requests` on every probe (a rate limit,
    /// nothing measured) and `qwen3-max` answered 404 "Model not exist." on the International
    /// endpoint: neither is seeded.
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
        SeededProfile {
            model: "qwen3.8-max",
            verified_on: "2026-10-20",
            build: qwen3_8_max,
        },
        SeededProfile {
            model: "qwen3.7-plus",
            verified_on: "2026-10-20",
            build: qwen3_7_plus,
        },
        SeededProfile {
            model: "qwen3.7-max",
            verified_on: "2026-10-20",
            build: qwen3_7_max,
        },
        SeededProfile {
            model: "qwen3.7-flash",
            verified_on: "2026-10-20",
            build: qwen3_7_flash,
        },
        SeededProfile {
            model: "qwen3.6-plus",
            verified_on: "2026-10-20",
            build: qwen3_6_plus,
        },
        SeededProfile {
            model: "qwen3.6-flash",
            verified_on: "2026-10-20",
            build: qwen3_6_flash,
        },
        SeededProfile {
            model: "qwen3.5-plus",
            verified_on: "2026-10-20",
            build: qwen3_5_plus,
        },
        SeededProfile {
            model: "qwen3.5-flash",
            verified_on: "2026-10-20",
            build: qwen3_5_flash,
        },
        SeededProfile {
            model: "qwen-max",
            verified_on: "2026-10-20",
            build: qwen_max,
        },
        SeededProfile {
            model: "qwen-flash",
            verified_on: "2026-10-20",
            build: qwen_flash,
        },
        SeededProfile {
            model: "qwen-plus-latest",
            verified_on: "2026-10-20",
            build: qwen_plus_latest,
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

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen3.8-max``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn qwen3_8_max() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-8-max
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 131,072.
            // The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(131_072)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen3.7-plus``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Docs conflict: the vision-model page gives "64k" for the Qwen3.7 maximum output; the
    // operator resolved it on 2026-10-20 to the Model Info value, 131072 (`50.1-SEED-PROBE.md`).
    fn qwen3_7_plus() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-7-plus
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 131,072.
            // The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(131_072)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen3.7-max``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn qwen3_7_max() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-7-max
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 131,072.
            // The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(131_072)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 2 (2026-10-20), `### Probes:
    // `qwen3.7-flash``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Docs conflict: the vision-model page gives "64k" for the Qwen3.7 maximum output; the
    // operator resolved it on 2026-10-20 to the Model Info value, 131072 (`50.1-SEED-PROBE.md`).
    fn qwen3_7_flash() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-7-flash
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 131,072.
            // The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(131_072)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen3.6-plus``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn qwen3_6_plus() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-6-plus
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 65,536. The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(65_536)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen3.6-flash``, probe numbers as in that table, and operator evidence (b) for the limits.
    // Probe 3 (temperature 1.99) was ACCEPTED but returned EMPTY content (finish stop); the
    // acceptance, not the answer, is what the temperature range rests on.
    fn qwen3_6_flash() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-6-flash
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 65,536. The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(65_536)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen3.5-plus``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn qwen3_5_plus() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-5-plus
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 65,536. The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(65_536)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 2 (2026-10-20), `### Probes:
    // `qwen3.5-flash``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn qwen3_5_flash() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen3-5-flash
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 991808 (thinking: 983616), max output 65,536. The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(65_536)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen-max``, probe numbers as in that table, and operator evidence (b) for the limits.
    // The docs call it functionally equivalent to the snapshot `qwen-max-2025-01-25`; it is not
    // `qwen3-max`, which answered 404 and is not seeded.
    fn qwen_max() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen-max
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 32,768,
            // max input 30720, max output 8,192. The listing exposes no limit.
            .with_context_window(32_768)
            .with_max_output_tokens(8_192)
            // Probe 1 (the default call) carried no reasoning text.
            .with_reasoning(false)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen-flash``, probe numbers as in that table, and operator evidence (b) for the limits.
    fn qwen_flash() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen-flash
            // (Model Info, "Context Limits", accessed 2026-10-20): context window 1,000,000,
            // max input 997952, max output 32,768. The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(32_768)
            // Probe 1 (the default call) carried no reasoning text.
            .with_reasoning(false)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `qwen-plus-latest``, probe numbers as in that table,
    // and operator evidence (b) for the limits.
    // A moving alias ("This dynamically updated version implements changes without prior
    // notice", docs): the row is true on its `verified_on` date only.
    fn qwen_plus_latest() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 1.99 ACCEPTED; probe 5: 2.00 answered
            // "Temperature should be in [0.0, 2.0)".
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties ACCEPTED at 0.5: `-2..=2` is contract-derived.
            .with_frequency_penalty(ParamSupport::Range(-2.0, 2.0))
            .with_presence_penalty(ParamSupport::Range(-2.0, 2.0))
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // docs https://www.alibabacloud.com/help/en/model-studio/qwen-plus
            // (section "Dynamic Updates"; Model Info, "Context Limits", accessed 2026-10-20):
            // context window 1,000,000, max input 995904, max output 32,768.
            // The listing exposes no limit.
            .with_context_window(1_000_000)
            .with_max_output_tokens(32_768)
            // Probe 1 (the default call) carried no reasoning text.
            .with_reasoning(false)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
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

    /// Grok. `grok-4.3`, `grok-4.5` and `grok-4.6` were measured on 2026-10-15
    /// (`50-SEED-PROBE.md`). On 2026-10-20 (`50.1-SEED-PROBE.md`) `grok-4.20-reasoning` and
    /// `grok-4.20-non-reasoning` were measured end to end (run 1), and `grok-4.7` was seeded
    /// under a recorded exception (below). Each id is seeded under the exact id the probe used,
    /// and the lookup is exact-match: the dated `grok-4.20-0309-*` ids that the listing shows and
    /// every other alias of the same model (`grok-4.20`, `grok-4.20-beta`, ...) are NOT covered
    /// and run on the adapter-wide fallback. `grok-build-0.1` and the multi-agent id were not
    /// probed, and the image and video ids are not chat models: none is seeded.
    ///
    /// **`grok-4.7` exception (owner ruling, 2026-10-20).** Its `temperature=2.00` probe failed
    /// at the vendor identically in both runs (`UNREACHABLE: error sending request`); `0.00` and
    /// `0.50` were accepted and `2.01` was a clean 400, so the vendor's 0..2 range is
    /// established. The owner ruled that a reproducible vendor-side failure at the top endpoint,
    /// observed identically in two independent runs, counts as a measured outcome for that one
    /// row, so `grok-4.7` is seeded with a range that stops below 2.00 (`0.0..=1.99`, like the
    /// three rows above). It applies to `grok-4.7` only and changes no other row.
    ///
    /// Common to `grok-4.3`, `grok-4.5` and `grok-4.6`:
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
    ///
    /// The two `grok-4.20-*` aliases differ in one respect from the rows above: their
    /// `temperature=2.00` probe was ACCEPTED, so their range is the measured `0.0..=2.0`.
    /// `grok-4.20-non-reasoning` also accepted one and five stop sequences; the reasoning alias
    /// rejected `stop`. The listing and the docs expose no maximum output for either.
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
        SeededProfile {
            model: "grok-4.20-reasoning",
            verified_on: "2026-10-20",
            build: grok_4_20_reasoning,
        },
        SeededProfile {
            model: "grok-4.20-non-reasoning",
            verified_on: "2026-10-20",
            build: grok_4_20_non_reasoning,
        },
        SeededProfile {
            model: "grok-4.7",
            verified_on: "2026-10-20",
            build: grok_4_7,
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
            // The 2026-10-15 basis (`temperature=2.00` UNREACHABLE) is what this range rests on.
            // The 2026-10-20 re-probe differs: run 1 ACCEPTED 2.00 (200), run 2 was UNREACHABLE
            // (`50.1-SEED-PROBE.md`, "Differences found"). The range is left as seeded.
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
            // The 2026-10-15 basis (`temperature=2.00` UNREACHABLE) is what this range rests on.
            // The 2026-10-20 re-probe differs: run 1 answered REJECTED (500) "Internal error
            // during token parsing", run 2 was UNREACHABLE (`50.1-SEED-PROBE.md`, "Differences
            // found"). The range is left as seeded.
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

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `grok-4.20-reasoning``, probe numbers as in that table. The probe used the alias; the
    // listing row is `grok-4.20-0309-reasoning`, whose `aliases` field names it. Only the alias
    // is seeded: the dated id is a distinct exact-match key and is not covered.
    fn grok_4_20_reasoning() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "Temperature must be less than 2 but temperature = 2.01".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties answered "Model grok-4.20-reasoning does not
            // support parameter ...".
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: `stop` answered "does not support parameter stop" for one and
            // for five sequences. (Operator evidence (c) documents up to 4 and "not supported
            // by reasoning models"; the measured rejection decides.)
            .with_stop_sequences(StopSupport::NotSent)
            // Listing context_length=1000000. No maximum output is documented (operator
            // evidence (b), https://docs.x.ai/developers/models/grok-4.20-reasoning, accessed
            // 2026-10-20), so none is seeded.
            .with_context_window(1_000_000)
            // Probe 1 (the default call) returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `grok-4.20-non-reasoning``, probe numbers as in that table. The probe used the alias; the
    // listing row is `grok-4.20-0309-non-reasoning`. Only the alias is seeded.
    fn grok_4_20_non_reasoning() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "Temperature must be less than 2 but temperature = 2.01".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties answered "Model grok-4.20-non-reasoning does not
            // support parameter ...".
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: one and five stop sequences ACCEPTED.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing context_length=1000000. No maximum output is documented (operator
            // evidence (b), https://docs.x.ai/developers/models/grok-4.20-non-reasoning,
            // accessed 2026-10-20), so none is seeded.
            .with_context_window(1_000_000)
            // Probe 1 (the default call) carried no reasoning text.
            .with_reasoning(false)
            // Probes 12 and 13: json_object and json_schema ACCEPTED.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, `### Probes: `grok-4.7`` in run 1 and in run
    // 2 (both 2026-10-20, commit 8ad298dd), probe numbers as in those tables, and its listing row.
    //
    // RECORDED EXCEPTION (owner ruling, 2026-10-20, `50.1-PROBE-CANDIDATES.md`, "Owner
    // decisions for plan 50.1-09"): probe 3, `temperature=2.00`, was UNREACHABLE ("error sending
    // request") in BOTH runs, so the temperature upper bound was never measured here. The
    // `temperature=2.00` probe of the sibling rows did NOT fail identically on 2026-10-20:
    // `grok-4.3` was UNREACHABLE in both runs, `grok-4.6` was ACCEPTED (200) in run 1 and
    // UNREACHABLE in run 2, and `grok-4.5` answered REJECTED (500) "Internal error during token
    // parsing" in run 1 and was UNREACHABLE in run 2. The owner ruled that a reproducible
    // vendor-side failure at the top endpoint, observed identically in two independent runs,
    // counts as a measured outcome for that one row, and that this row is seeded anyway. It
    // applies to `grok-4.7` only and changes no other row.
    fn grok_4_7() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 4: temperature 0.00 and 0.50 ACCEPTED in both runs; probe 5: 2.01
            // answered a clean 400 "Temperature must be less than 2 but temperature = 2.01",
            // which establishes the vendor's 0..2 range. 2.00 fails at the vendor, so the range
            // stops at 1.99, the largest value under the stated bound (the shape the Qwen probe
            // measured, and the one the `grok-4.3`, `grok-4.5` and `grok-4.6` rows use). The
            // registry's range type is inclusive, so this is how "below 2.00" is expressed;
            // 1.99 itself was not probed for `grok-4.7`. A caller cannot send 2.00.
            .with_temperature(ParamSupport::Range(0.0, 1.99))
            // Probe 6: top_p 0.5 ACCEPTED: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: both penalties answered "Model grok-4.7 does not support parameter
            // ...".
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probes 9 and 10: `stop` answered "does not support parameter stop" for one and for
            // five sequences.
            .with_stop_sequences(StopSupport::NotSent)
            // Listing context_length=500000. No maximum output is seeded: the docs state "No text
            // output limit" (owner ruling, 2026-10-20), which is no number.
            .with_context_window(500_000)
            // The listing declares capabilities.reasoning_effort (default "high") and the run 2
            // default call returned reasoning text.
            .with_reasoning(true)
            // Probes 12 and 13: json_object and json_schema ACCEPTED in both runs.
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
    /// measured on 2026-10-15 (`50-SEED-PROBE.md`); nine further ids were measured end to end on
    /// 2026-10-20 (`50.1-SEED-PROBE.md`, run 1): `gemini-4-argon`, `gemini-3.8-flash`,
    /// `gemini-3.7-flash`, `gemini-3.1-pro-preview`, `gemini-pro-latest`, `gemini-flash-latest`,
    /// `gemini-flash-lite-latest`, `gemini-3.1-flash-lite` and `gemini-3-flash-preview`. A
    /// `*-latest` id is a moving alias and a `*-preview` id a preview: such a row is true on its
    /// `verified_on` date only. `gemini-2.5-pro`, `gemini-2.5-flash` and `gemini-2.5-flash-lite`
    /// answered the default call 404 ("no longer available to new users"), so nothing was
    /// measured for them and they are not seeded.
    ///
    /// Common to all twelve: temperature `0.00` and `2.00` accepted and `2.01` rejected
    /// ("temperature must be in the range [0.0, 2.0]"); `top_p` 0.5 accepted (`0..=1` is
    /// contract-derived); the `generateContent` wire has no penalty field (probe: NOT
    /// APPLICABLE), so both penalties are `NotSent`; five stop sequences accepted; JSON object
    /// (`responseMimeType`) and JSON schema (`responseSchema`) both accepted; the listing
    /// reports `thinking=true` and an `inputTokenLimit` of 1048576. The `outputTokenLimit` is
    /// 65536 for every id except `gemini-4-argon`, which reports 32768.
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
        SeededProfile {
            model: "gemini-4-argon",
            verified_on: "2026-10-20",
            build: gemini_4_argon,
        },
        SeededProfile {
            model: "gemini-3.8-flash",
            verified_on: "2026-10-20",
            build: gemini_3_8_flash,
        },
        SeededProfile {
            model: "gemini-3.7-flash",
            verified_on: "2026-10-20",
            build: gemini_3_7_flash,
        },
        SeededProfile {
            model: "gemini-3.1-pro-preview",
            verified_on: "2026-10-20",
            build: gemini_3_1_pro_preview,
        },
        SeededProfile {
            model: "gemini-pro-latest",
            verified_on: "2026-10-20",
            build: gemini_pro_latest,
        },
        SeededProfile {
            model: "gemini-flash-latest",
            verified_on: "2026-10-20",
            build: gemini_flash_latest,
        },
        SeededProfile {
            model: "gemini-flash-lite-latest",
            verified_on: "2026-10-20",
            build: gemini_flash_lite_latest,
        },
        SeededProfile {
            model: "gemini-3.1-flash-lite",
            verified_on: "2026-10-20",
            build: gemini_3_1_flash_lite,
        },
        SeededProfile {
            model: "gemini-3-flash-preview",
            verified_on: "2026-10-20",
            build: gemini_3_flash_preview,
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

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-4-argon``, probe numbers as in that table, and its listing row.
    fn gemini_4_argon() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=32768.
            .with_context_window(1_048_576)
            .with_max_output_tokens(32_768)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-3.8-flash``, probe numbers as in that table, and its listing row.
    fn gemini_3_8_flash() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-3.7-flash``, probe numbers as in that table, and its listing row.
    fn gemini_3_7_flash() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-3.1-pro-preview``, probe numbers as in that table, and its listing row.
    // A preview id: its row is true on its `verified_on` date only, because the vendor
    // may change or retire a preview without notice.
    fn gemini_3_1_pro_preview() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-pro-latest``, probe numbers as in that table, and its listing row.
    // A moving alias: its row is true on its `verified_on` date only, because the
    // vendor re-points the alias at a newer model without changing the id.
    fn gemini_pro_latest() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-flash-latest``, probe numbers as in that table, and its listing row.
    // A moving alias: its row is true on its `verified_on` date only, because the
    // vendor re-points the alias at a newer model without changing the id.
    fn gemini_flash_latest() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-flash-lite-latest``, probe numbers as in that table, and its listing row.
    // A moving alias: its row is true on its `verified_on` date only, because the
    // vendor re-points the alias at a newer model without changing the id.
    fn gemini_flash_lite_latest() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-3.1-flash-lite``, probe numbers as in that table, and its listing row.
    fn gemini_3_1_flash_lite() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
    }

    // Source for the row below: `50.1-SEED-PROBE.md`, run 1 (2026-10-20), `### Probes:
    // `gemini-3-flash-preview``, probe numbers as in that table, and its listing row.
    // A preview id: its row is true on its `verified_on` date only, because the vendor
    // may change or retire a preview without notice.
    fn gemini_3_flash_preview() -> ModelProfile {
        ModelProfile::new()
            // Probes 2 and 3: temperature 0.00 and 2.00 ACCEPTED; probe 5: 2.01 answered
            // "temperature must be in the range [0.0, 2.0]".
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            // Probe 6: top_p 0.5 accepted: `0..=1` is contract-derived.
            .with_top_p(ParamSupport::Range(0.0, 1.0))
            // Probes 7 and 8: NOT APPLICABLE, the generateContent wire has no penalty field.
            .with_frequency_penalty(ParamSupport::NotSent)
            .with_presence_penalty(ParamSupport::NotSent)
            // Probe 10: five stop sequences accepted.
            .with_stop_sequences(StopSupport::Supported { max: None })
            // Listing: inputTokenLimit=1048576, outputTokenLimit=65536.
            .with_context_window(1_048_576)
            .with_max_output_tokens(65_536)
            // Listing: thinking=true.
            .with_reasoning(true)
            // Probes 12 and 13: responseMimeType and responseSchema both accepted.
            .with_json_object(true)
            .with_json_schema(true)
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
