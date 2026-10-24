//! Shared OpenAI-compatible protocol engine (D-05).
//!
//! `CompatEngine` owns every part of the OpenAI-compatible chat-completions
//! protocol that is identical across vendors: request shaping, HTTP
//! transport, SSE stream assembly, retry-with-backoff, status-to-`LlmError`
//! mapping, credential redaction and memoized model-list resolution.
//! Extracted and generalized from `deepseek/adapter.rs` (RESEARCH.md finding
//! 1 — that file and `openai/adapter.rs` already implement this same
//! protocol twice).
//!
//! A preset (e.g. `kimi::adapter::KimiAdapter`) supplies only its
//! `base_url`, credential, default model, curated model-list fallback and a
//! [`CompatCapabilities`] block, then delegates every [`LlmPort`] method to
//! an owned `CompatEngine`.
//!
//! [`LlmPort`]: paladin_ports::output::llm_port::LlmPort

use futures::{Stream, StreamExt};
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use std::time::{Duration, SystemTime};
use tokio::sync::OnceCell;

use paladin_core::platform::container::prompt::{PromptItem, PromptType};
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmRequest, LlmResponse, ProviderCapabilities, ReasoningEffort,
    ReasoningFallback, StreamingResponse, TokenUsage,
};
use paladin_ports::output::model_profile::{ModelProfile, ResolvedEffort, StopSupport};
use paladin_ports::output::rate_limit_hints::RateLimitHints;

use super::types::{
    CompatMessage, CompatModelsResponse, CompatRequest, CompatResponse, CompatResponseFormat,
    CompatStreamOptions, CompatThinking, CompatUsage,
};
use crate::backoff::exponential_backoff;
use crate::http_status::map_http_status_with_hints;
use crate::rate_limit_headers::{RateLimitHeaderFamily, hints_from_headers};
use crate::reasoning::{self, ReasoningWire, SubstitutionLog, WireCaps, mask_effort};
use crate::redaction::diagnostic_excerpt as redact_and_bound;
use crate::registry::{ModelRegistry, fallback_from_compat};
use crate::response_guards::{StreamCompletion, bill, classify_completion};
use crate::sampling::{self, FormatLog, OmissionLog, RequestNotes};
use crate::sse::SseLines;
use crate::usage_reconcile::{USAGE_UNRECONCILED_METADATA_KEY, UsageLog};

/// Parse a `429` response's rate-limit headers through the generic family (PACE-01).
///
/// Must run BEFORE `response.text()` consumes the response (research Pitfall 2). Only a 429
/// is parsed, and no credential header is read: the generic family looks at `Retry-After`,
/// `retry-after-ms` and the `x-ratelimit-*` names only, so a preset that is not one of the
/// named providers still surfaces the delay its endpoint sent.
fn snapshot_rate_limit_hints(
    status: reqwest::StatusCode,
    headers: &HeaderMap,
) -> Option<RateLimitHints> {
    if status != reqwest::StatusCode::TOO_MANY_REQUESTS {
        return None;
    }
    hints_from_headers(RateLimitHeaderFamily::Generic, SystemTime::now(), |name| {
        headers.get(name).and_then(|v| v.to_str().ok())
    })
}

/// Capabilities a preset declares for its own request path.
///
/// Per D-04's posture (extended to every preset, not only the generic
/// provider): every flag here must describe what *this adapter's own*
/// `generate()`/`generate_stream()` implements, never what the vendor's API
/// advertises in its own documentation.
#[derive(Debug, Clone)]
pub struct CompatCapabilities {
    /// Whether this preset's `generate_stream()` is implemented.
    pub supports_streaming: bool,
    /// Whether this preset's request path can carry a tool definition.
    /// `LlmRequest` has no tools field today, so every preset built on this
    /// engine must set this `false` (RESEARCH.md Pitfall 4).
    pub supports_tool_calling: bool,
    /// Whether this preset's `generate()` ever returns a populated
    /// `function_call`. No preset built on this engine does today.
    pub supports_function_calling: bool,
    /// Whether this preset's request path can carry image content.
    pub supports_vision: bool,
    /// Whether this preset exposes embeddings generation.
    pub supports_embeddings: bool,
    /// The preset's advertised maximum context window, in tokens.
    pub max_context_tokens: Option<u32>,
    /// Whether this preset's request path supports a system message.
    pub supports_system_messages: bool,
    /// The preset's valid temperature range (ADR-0004).
    pub temperature_range: Option<(f32, f32)>,
}

impl From<CompatCapabilities> for ProviderCapabilities {
    fn from(c: CompatCapabilities) -> Self {
        ProviderCapabilities {
            supports_streaming: c.supports_streaming,
            supports_tool_calling: c.supports_tool_calling,
            supports_function_calling: c.supports_function_calling,
            supports_vision: c.supports_vision,
            supports_embeddings: c.supports_embeddings,
            max_context_tokens: c.max_context_tokens,
            supports_system_messages: c.supports_system_messages,
            temperature_range: c.temperature_range,
        }
    }
}

/// Which of the five optional sampling parameters a preset's own request
/// path puts on the wire (17-18, closing G-17-4a).
///
/// Extends the D-04 posture from [`CompatCapabilities`] into request
/// shaping: each flag describes what **this preset's own request path**
/// carries, established by measurement against the vendor, not by reading
/// the vendor's marketing page. `presence_penalty` and `frequency_penalty`
/// were shipped as unconditionally-carried for every preset before this
/// plan; xAI rejects both **by presence**, for every current model — the
/// measured cause of G-17-4a, not a stale model ID.
///
/// A `false` flag does not omit the caller's value because the caller left
/// it unset — the caller may have set it explicitly, and it is still
/// dropped: the flag is a statement about the *vendor's protocol*, never
/// about what a particular request happened to ask for.
///
/// Deliberately has **no** `Default` impl, matching the same posture on
/// [`CompatEngineConfig`] itself (no `Default`, no struct-update syntax at
/// any construction site): a new preset must be a compile error until its
/// author states a position for every field, one at a time. Silent
/// inheritance of "everything is supported" is exactly how the shipped
/// Grok preset came to send a parameter xAI rejects.
///
/// The same rule now covers reasoning effort (Phase 51, REASON-02): the
/// non-`bool` [`reasoning`](Self::reasoning) field names the one request
/// field this preset's vendor parses an effort in, so a preset cannot be built
/// without stating it, and no wire is ever inherited from another preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatRequestParameters {
    /// Whether this preset's request path carries `temperature`.
    ///
    /// **Option (a) — chosen by the developer 2026-08-22, recorded here
    /// against ADR-0004 (closing G-17-4b, plan 17-19).** When a preset
    /// declares this `false`, `CompatEngine::build_request` omits
    /// `temperature` from the outgoing body entirely; it never substitutes
    /// one legal value for another. ADR-0004's *Considered Options* rejects
    /// adapter-level clamping by name: *"a caller who requested 1.8 and got
    /// 1.0 silently substituted has no way to discover the substitution
    /// happened."* Declaring this field `false` is not that: no value is
    /// swapped in for another, the key is simply absent from the JSON body,
    /// and the vendor's own single legal value applies server-side. The
    /// caller's declared-unsupported value is still discoverable — the
    /// omission is logged as a `warn!` (target `paladin::sampling`, once per
    /// instance per model) and recorded on the response as
    /// `paladin.params_omitted` (ADR-0061; it was a `debug!` before Phase
    /// 50). Since Phase 50 the **per-model profile** decides what is sent;
    /// this flag now feeds the adapter-wide fallback profile derived from it
    /// (`registry::fallback_from_compat`), not a separate gate. First used by
    /// Kimi's fixed-temperature preset (`kimi::adapter::KimiAdapter`, plan
    /// 17-19), whose live-measured constraint made the framework's
    /// fabricated `0.7` default temperature (`PaladinData::default()`)
    /// unconditionally rejected by the vendor.
    pub temperature: bool,
    /// Whether this preset's request path carries `max_tokens`.
    pub max_tokens: bool,
    /// Whether this preset's request path carries `top_p`.
    pub top_p: bool,
    /// Whether this preset's request path carries `frequency_penalty`.
    pub frequency_penalty: bool,
    /// Whether this preset's request path carries `presence_penalty`.
    pub presence_penalty: bool,
    /// Where this preset's vendor parses a reasoning effort in the request body (Phase 51,
    /// REASON-02, D-04).
    ///
    /// The wire is vendor-wide and lives here on the preset; which levels a given model accepts
    /// is per model and lives on that model's profile (`ModelProfile::effort`). It is a closed
    /// enum, never a bool and never a free-form field name: [`ReasoningWire::Unsupported`] makes
    /// every explicit effort fail before any HTTP call, and no wire ever sends a second effort
    /// field beside its own. A preset with no verified form declares `Unsupported` (ADR-0061,
    /// handoff section 5: a shared default of top-level `reasoning_effort` is the silent no-op
    /// this field exists to prevent).
    pub reasoning: ReasoningWire,
}

impl CompatRequestParameters {
    /// Every optional sampling parameter carried — the OpenAI-compatible
    /// baseline every preset except Grok's xAI preset declares in this
    /// plan (T-17-75's behaviour-preservation requirement) — with the
    /// reasoning wire the caller states.
    ///
    /// The wire has no default on purpose (REASON-02): passing it here is how a preset that
    /// takes the baseline still states its reasoning position.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_llm::compat::CompatRequestParameters;
    /// use paladin_llm::reasoning::ReasoningWire;
    ///
    /// let parameters = CompatRequestParameters::all(ReasoningWire::Unsupported);
    /// assert!(parameters.temperature && parameters.max_tokens);
    /// assert_eq!(parameters.reasoning, ReasoningWire::Unsupported);
    /// ```
    pub const fn all(reasoning: ReasoningWire) -> Self {
        Self {
            temperature: true,
            max_tokens: true,
            top_p: true,
            frequency_penalty: true,
            presence_penalty: true,
            reasoning,
        }
    }
}

/// Configuration a preset supplies to construct a [`CompatEngine`].
pub struct CompatEngineConfig {
    /// The provider's API base URL, e.g. `https://api.moonshot.ai/v1`.
    pub base_url: String,
    /// The credential sent as `Authorization: Bearer {api_key}`.
    pub api_key: String,
    /// The default model identifier this preset requests.
    pub model: String,
    /// Request timeout, in seconds.
    pub timeout_seconds: u64,
    /// Maximum retry attempts for a retryable error.
    pub max_retries: u32,
    /// What this preset's own request path implements (D-04 posture).
    pub capabilities: CompatCapabilities,
    /// Which optional sampling parameters this preset's own request path
    /// carries (17-18). Gates `max_tokens` and seeds the adapter-wide fallback
    /// profile `build_request` filters the other four through (ADR-0061) —
    /// see [`CompatRequestParameters`]. Sits beside `error_override` and
    /// `redirect_policy` as a per-vendor protocol knob the engine reads
    /// off its own config rather than branching on provider identity.
    pub request_parameters: CompatRequestParameters,
    /// The curated model list returned when the live `/models` endpoint
    /// fails or returns an empty list (D-13).
    pub fallback_models: Vec<String>,
    /// Preset-specific status-code override, consulted BEFORE the engine's
    /// own mapping. Returning `Some(err)` short-circuits to that error;
    /// returning `None` falls through to the engine's default mapping. This
    /// is how a future preset (e.g. one with its own 402 semantics) adds an
    /// arm without editing the shared engine.
    pub error_override: Option<fn(u16, &str) -> Option<LlmError>>,
    /// Optional override for the underlying HTTP client's redirect policy.
    ///
    /// `None` preserves this engine's original behaviour — no `.redirect()`
    /// call on the client builder, i.e. `reqwest`'s own default policy
    /// (follow up to 10 hops). The field's meaning is **not** inverted by
    /// WR-04 (`17-REVIEW.md`): a future preset that legitimately needs to
    /// follow a redirect can still leave this `None` to get that behaviour.
    ///
    /// `Some(policy)` lets a preset restrict follow behaviour.
    /// `openai_compatible::OpenAiCompatibleAdapter` was the first to set
    /// `Some(Policy::none())` (T-17-18), because its `base_url` is entirely
    /// operator-supplied with no vendor default to fall back on. Every
    /// preset in this crate now sets the identical
    /// `Some(reqwest::redirect::Policy::none())` (T-17-52, plan 17-10):
    /// each one's `*_BASE_URL` is documented and operator-settable too, so
    /// the same reasoning applies uniformly. Setting `Policy::none()` means
    /// a `3xx` response can never cause the `Authorization` header carrying
    /// the operator's API key to be replayed to a different,
    /// attacker-influenced host — see `CompatEngine::map_error`'s
    /// `300..=399` arm for what a refused redirect surfaces to the caller
    /// as.
    pub redirect_policy: Option<reqwest::redirect::Policy>,
}

/// Whether a failed `fetch_live_models` call indicates the operator
/// misconfigured something, or is a state this design already supports and
/// deliberately stays quiet about (G-17-4d, 2026-08-22).
///
/// Read by [`CompatEngine::available_models`]'s `Err(e)` arm to choose a log
/// level. Deliberately just two states, not a description of *why* — the
/// reasoning for each variant's placement lives on
/// [`classify_fetch_failure`]'s match arms, not in this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FetchFailureClass {
    /// The operator most likely configured the wrong credential for the
    /// endpoint they pointed it at, or the wrong endpoint for the credential
    /// they hold. Worth surfacing at `warn`.
    Misconfiguration,
    /// A state this design already supports — offline, rate-limited, out of
    /// quota, or any other failure a model-list fetch has no business
    /// diagnosing on the operator's behalf. Stays at `debug`, unchanged from
    /// before this plan.
    Supported,
}

/// Classify a `fetch_live_models` failure for
/// [`CompatEngine::available_models`]'s log level (G-17-4d, 2026-08-22).
///
/// A pure function of the [`LlmError`] variant alone — no provider name, no
/// base-URL host, no feature flag. `CompatEngine` backs six presets (kimi,
/// qwen, grok, ollama, openai-compatible, and the deepseek-style preset
/// family), and this function reaching every one of them without any preset
/// change is exactly what keeps D-05's shared-engine boundary intact: the
/// engine still learns nothing about which vendor is on the other end.
///
/// Exhaustive over every known `LlmError` variant, each with its own
/// deliberate arm below. `LlmError` is `#[non_exhaustive]` (X-10.2, D-04) as
/// of Phase 25, so a wildcard arm is now REQUIRED for this to compile as a
/// downstream crate — it exists solely to satisfy that compiler requirement,
/// classifying a genuinely unknown future variant the same as the generic
/// non-success catch-all (`FetchFailureClass::Supported`) rather than
/// panicking. A variant added to `paladin-ports::LlmError` should still get
/// its own explicit arm above the wildcard so a person decides which side of
/// the split it falls on, rather than leaning on the wildcard by default.
fn classify_fetch_failure(error: &LlmError) -> FetchFailureClass {
    match error {
        // The one case this plan closes: a rejected credential (mapped from
        // HTTP 401 by `map_error`) is a configuration problem, not a state
        // the D-13/D-14 fallback design was ever meant to describe quietly.
        LlmError::AuthenticationError(_) => FetchFailureClass::Misconfiguration,

        // Being offline is the exact case D-13/D-14 were written for — a
        // self-hosted Ollama that has not started yet is a normal, SUPPORTED
        // condition, not a misconfiguration. Warn-spam here would train
        // operators to ignore the one message this plan makes audible.
        LlmError::NetworkError(_) => FetchFailureClass::Supported,
        // A slow or unresponsive endpoint is transient, for the same reason
        // as the arm above.
        LlmError::Timeout(_) => FetchFailureClass::Supported,

        // A rate limit says "you called too often", not "you configured the
        // wrong thing" — the credential and the endpoint are both fine.
        LlmError::RateLimitExceeded { .. } => FetchFailureClass::Supported,
        // An exhausted quota is an account-billing state, not a
        // credential/endpoint mismatch — nothing about the CONFIGURATION is
        // wrong here either.
        LlmError::UsageLimitExceeded { .. } => FetchFailureClass::Supported,
        // `fetch_live_models` GETs `/models` with no model identifier on the
        // wire, so `map_error`'s 404 arm cannot actually be produced by this
        // call path. Classified anyway because this match must stay
        // exhaustive over the type, not merely over what is reachable
        // today — and an unknown-model 404 is not itself evidence of a
        // wrong credential or endpoint.
        LlmError::ModelNotAvailable(_) => FetchFailureClass::Supported,
        // Never produced by a model-list fetch (no prompt is sent) — quiet
        // for the same exhaustiveness-over-reachability reason as above.
        LlmError::TokenLimitExceeded => FetchFailureClass::Supported,
        // Never produced by a model-list fetch either (no completion is
        // requested) — same reason.
        LlmError::EmptyCompletion { .. } => FetchFailureClass::Supported,
        // A refusal is a policy decision by a model about a prompt; a
        // model-list fetch sends no prompt, so a vendor answering `/models`
        // with one means the endpoint is not the OpenAI-compatible service
        // the configuration claims. That is a configuration problem, not a
        // transient availability one, so it is audible (D-09).
        LlmError::Refusal { .. } => FetchFailureClass::Misconfiguration,
        // A malformed prompt cannot reach a model-list fetch at all — this
        // arm exists only so the match stays exhaustive.
        LlmError::InvalidPrompt(_) => FetchFailureClass::Supported,
        // Body-read / parse failures on an otherwise-2xx model-list response.
        LlmError::ProcessingError(_) => FetchFailureClass::Supported,
        // Every non-2xx status without a dedicated variant (Phase 25 D-03,
        // via `map_http_status`) AND the refused-redirect message
        // (`map_error`'s `300..=399` arm, WR-04). A refused redirect is
        // arguably a misconfiguration too, but it is left quiet
        // deliberately: that message already names the exact setting to
        // check on its own, so raising it again here would only duplicate an
        // already-actionable message rather than surface a silent one. An
        // exhausted fallback chain is classified the same way.
        LlmError::ProviderError { .. } => FetchFailureClass::Supported,
        LlmError::AllProvidersFailed { .. } => FetchFailureClass::Supported,
        // A refused reasoning effort is a statement about the request's configuration, never a
        // fetch condition; a model-list fetch sends no effort, so this is never produced by one.
        LlmError::UnsupportedParameter { .. } => FetchFailureClass::Misconfiguration,
        // X-10.2 (D-04): `LlmError` is `#[non_exhaustive]`; required by the
        // compiler, not reachable by any variant known today. See the
        // rustdoc above for why this stays `Supported` rather than a panic.
        _ => FetchFailureClass::Supported,
    }
}

/// Remove a userinfo component (`user:pass@`) from a base URL before it is
/// written into a log line (T-17-91). A configured base URL is
/// operator-supplied, and while no shipped preset's documented format
/// includes one, a base URL is exactly the kind of operator-supplied string
/// that could carry a credential this way.
///
/// Looks only at the authority section — everything between the first
/// `://` and the next `/` (or the end of the string, if there is no path).
/// If that section contains an `@`, everything up to and including it is
/// dropped; otherwise the input is returned unchanged. A string with no
/// `://` at all is returned unchanged rather than guessed at.
fn base_url_without_userinfo(base_url: &str) -> String {
    let Some(scheme_end) = base_url.find("://") else {
        return base_url.to_string();
    };
    let authority_start = scheme_end + 3;
    let authority_end = base_url[authority_start..]
        .find('/')
        .map(|i| authority_start + i)
        .unwrap_or(base_url.len());

    let authority = &base_url[authority_start..authority_end];
    // RFC 3986 terminates userinfo at the LAST '@' in the authority, not the
    // first. `find` here leaked a credential fragment whenever a password
    // contained an unescaped '@': for `https://alice:p@ssw0rd@example.com/v1`
    // it kept everything after the first '@', emitting `ssw0rd@example.com`
    // into the warn diagnostic. `rfind` strips the whole userinfo component.
    let Some(at_idx) = authority.rfind('@') else {
        return base_url.to_string();
    };

    let mut out = String::with_capacity(base_url.len());
    out.push_str(&base_url[..authority_start]);
    out.push_str(&authority[at_idx + 1..]);
    out.push_str(&base_url[authority_end..]);
    out
}

/// How a preset's vendor reports reasoning tokens relative to `completion_tokens` (G-50-5a).
///
/// [`TokenUsage`] is inclusive: `completion_tokens` carries every billed output token, reasoning
/// included. Vendors on the OpenAI chat wire do not all report it that way, so each preset
/// declares its vendor's contract through [`CompatEngine::with_usage_semantics`] and the engine
/// folds accordingly -- declared, never branched on provider identity. The vendor's own
/// `total_tokens` is the check on the declaration: a wrong one is recorded under
/// [`USAGE_UNRECONCILED_METADATA_KEY`] on every reasoning call, never silently priced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompatUsageSemantics {
    /// OpenAI's documented contract: `completion_tokens` already includes
    /// `completion_tokens_details.reasoning_tokens`, so `completion_tokens` is copied as is and
    /// the vendor total is `prompt_tokens + completion_tokens`. The engine default; Kimi, Qwen,
    /// Ollama and OpenAI-compatible keep it.
    #[default]
    ReasoningInsideCompletion,
    /// The vendor reports `completion_tokens_details.reasoning_tokens` beside
    /// `completion_tokens` (vendor total = prompt + completion + reasoning), so the engine adds
    /// the reasoning into `completion_tokens` (saturating) and keeps the reasoning sub-count as
    /// reported. Grok declares it (live-measured 2026-10-18, `50-USAGE-PROBE.md`; see
    /// `GrokAdapter::new`).
    // Declared by the Grok preset only; a build without it never constructs this variant.
    #[cfg_attr(not(feature = "grok"), allow(dead_code))]
    ReasoningBesideCompletion,
}

/// The shared OpenAI-compatible engine every preset built on this core
/// delegates to.
///
/// ## A known, recorded asymmetry (D-06, WR-04)
///
/// This engine's [`generate_stream`](Self::generate_stream) and
/// [`GeminiAdapter::generate_stream`](crate::gemini::GeminiAdapter) both
/// retry their connection-opening request, matching their own `generate()`
/// (17-16, closing the new WR-04 recorded in `17-REVIEW.md`). The shipped
/// `openai/`, `anthropic/` and `deepseek/` adapters do **not** — their
/// `generate_stream` implementations still send a single, unretried POST,
/// exactly as `deepseek/adapter.rs` (the file this engine was extracted and
/// generalized from) has always done. This is a **deliberate boundary, not
/// an oversight**: D-06 keeps this phase additive against shipped, covered
/// code, so those three adapters are read but never edited here. A reader
/// comparing this engine's or Gemini's streaming retry behaviour against
/// `openai`/`anthropic`/`deepseek` should read the difference as this
/// recorded decision, not re-file it as a fresh finding. **Trigger to close
/// it:** the next phase that has reason to touch those two adapters anyway
/// (`17-CONTEXT.md` §Deferred Ideas).
pub struct CompatEngine {
    client: Client,
    config: CompatEngineConfig,
    models_cache: OnceCell<Vec<String>>,
    /// The name stamped on every [`LlmError::ProviderError`] this engine
    /// emits (Phase 25 D-03). Set by the preset through
    /// [`CompatEngine::with_provider_name`]; defaults to
    /// [`DEFAULT_PROVIDER_NAME`] so an engine built without a preset still
    /// carries a truthful, if generic, name.
    provider_name: &'static str,
    /// The per-model profile registry the preset attached through
    /// [`CompatEngine::with_model_registry`] (ADR-0061). Private and **not** a field of the
    /// public [`CompatEngineConfig`], which stays constructible exactly as before. `None` for an
    /// engine built without a preset: [`CompatEngine::model_profile`] then derives one profile
    /// from the config's own declarations.
    model_registry: Option<ModelRegistry>,
    /// The output budget sent when a request leaves `max_tokens` unset, from the preset's config
    /// ([`CompatEngine::with_default_max_tokens`]). Private and **not** a field of the public
    /// [`CompatEngineConfig`], which stays constructible exactly as before. `None` sends no
    /// `max_tokens` key (the engine's behaviour before PARITY-03).
    default_max_tokens: Option<u32>,
    /// The provider-level default reasoning effort and fallback from the preset's config
    /// ([`CompatEngine::with_default_reasoning`], Phase 51 D-00e). Private and **not** a field of
    /// the public [`CompatEngineConfig`]. `None` sends nothing for a request that sets none.
    default_reasoning: (Option<ReasoningEffort>, Option<ReasoningFallback>),
    /// Which (model, parameter) omissions this instance has already warned about.
    omissions: OmissionLog,
    /// Which (model, adjustment) `response_format` changes this instance has already warned
    /// about.
    format_log: FormatLog,
    /// How the preset's vendor reports reasoning against `completion_tokens`
    /// ([`CompatEngine::with_usage_semantics`]). Private and **not** a field of the public
    /// [`CompatEngineConfig`], which stays constructible exactly as before.
    usage_semantics: CompatUsageSemantics,
    /// Which (model, check) usage findings this instance has already warned about (G-50-5a).
    usage_log: UsageLog,
    /// Which (model, requested, effective) reasoning substitutions this instance has already
    /// warned about (Phase 51, D-06).
    substitutions: SubstitutionLog,
}

/// What each reasoning wire's request body can carry of a profile's effort data (Phase 51,
/// D-00f). The profile an engine reports is masked by this, and `reasoning::admit` then refuses
/// before HTTP whatever the masked profile does not admit.
///
/// - [`ReasoningWire::TopLevelEffort`]: a level (`reasoning_effort: "L"`) and `off`
///   (`reasoning_effort: "none"`), plus `auto` (nothing sent). No budget field.
/// - [`ReasoningWire::ThinkingObject`]: a level (`thinking.effort`), `off` (`type: disabled`) and
///   `auto` (`type: enabled`). No budget field.
/// - [`ReasoningWire::EnableThinking`]: `off`, `auto` and a budget (`enable_thinking` with
///   `thinking_budget`). It has no level field, so levels and the coerced list are masked away.
/// - anything else, [`ReasoningWire::Unsupported`] included: [`WireCaps::NONE`], so every explicit
///   effort is refused with `WireUnsupported`. `ReasoningWire` is `#[non_exhaustive]`: a wire this
///   engine does not know carries nothing rather than something wrong.
fn wire_caps(wire: ReasoningWire) -> WireCaps {
    match wire {
        ReasoningWire::TopLevelEffort | ReasoningWire::ThinkingObject => WireCaps {
            levels: true,
            off: true,
            auto: true,
            budget: false,
        },
        ReasoningWire::EnableThinking => WireCaps {
            levels: false,
            off: true,
            auto: true,
            budget: true,
        },
        _ => WireCaps::NONE,
    }
}

/// The four request fields an admitted reasoning effort can set; each is absent unless the
/// declared wire names it, and no wire sets a field of another.
#[derive(Debug, Default)]
struct CompatReasoningFields {
    reasoning_effort: Option<&'static str>,
    thinking: Option<CompatThinking>,
    enable_thinking: Option<bool>,
    thinking_budget: Option<u32>,
}

/// Turn an admitted effort into the one wire form `wire` names.
///
/// # Errors
///
/// [`LlmError::UnsupportedParameter`] for a (wire, effort) pair this engine has no mapping for.
/// Masking and `admit` already refuse those, so this is the last line of defence: it fails closed
/// and can never put a field on the wire that the request did not ask for.
fn map_reasoning(
    provider: &'static str,
    model: &str,
    wire: ReasoningWire,
    resolved: &ResolvedEffort,
) -> Result<CompatReasoningFields, LlmError> {
    if !resolved.on_wire {
        return Ok(CompatReasoningFields::default());
    }
    let level = reasoning::level_spelling(resolved.effective);
    let fields = match wire {
        // `reasoning_effort: "L"`; `Off` is the vendor's `"none"`; `Auto` sends nothing (the
        // vendor's own default depth) but is still stamped on the response.
        ReasoningWire::TopLevelEffort => match (level, resolved.effective) {
            (Some(level), _) => Some(CompatReasoningFields {
                reasoning_effort: Some(level),
                ..CompatReasoningFields::default()
            }),
            (None, ReasoningEffort::Off) => Some(CompatReasoningFields {
                reasoning_effort: Some("none"),
                ..CompatReasoningFields::default()
            }),
            (None, ReasoningEffort::Auto) => Some(CompatReasoningFields::default()),
            _ => None,
        },
        // `thinking: {type, effort}`: a level enables thinking at that effort, `Off` disables
        // it, `Auto` enables it with no effort.
        ReasoningWire::ThinkingObject => match (level, resolved.effective) {
            (Some(level), _) => Some(CompatThinking {
                kind: "enabled",
                effort: Some(level),
            }),
            (None, ReasoningEffort::Off) => Some(CompatThinking {
                kind: "disabled",
                effort: None,
            }),
            (None, ReasoningEffort::Auto) => Some(CompatThinking {
                kind: "enabled",
                effort: None,
            }),
            _ => None,
        }
        .map(|thinking| CompatReasoningFields {
            thinking: Some(thinking),
            ..CompatReasoningFields::default()
        }),
        // `enable_thinking` switch plus an optional `thinking_budget`; the wire has no level
        // field, so a level never reaches this arm (masked out of the profile).
        ReasoningWire::EnableThinking => match resolved.effective {
            ReasoningEffort::Off => Some(CompatReasoningFields {
                enable_thinking: Some(false),
                ..CompatReasoningFields::default()
            }),
            ReasoningEffort::Auto => Some(CompatReasoningFields {
                enable_thinking: Some(true),
                ..CompatReasoningFields::default()
            }),
            ReasoningEffort::Budget(budget) => Some(CompatReasoningFields {
                enable_thinking: Some(true),
                thinking_budget: Some(budget),
                ..CompatReasoningFields::default()
            }),
            _ => None,
        },
        _ => None,
    };
    if let Some(fields) = fields {
        return Ok(fields);
    }
    // `Unsupported` (masked away upstream) and any pair without a mapping: fail closed, no field
    // is sent.
    Err(LlmError::unsupported_parameter(
        "reasoning_effort",
        resolved.effective.to_string(),
        provider,
        model,
        format!("the '{wire}' wire has no form for that effort"),
    ))
}

/// The provider name a [`CompatEngine`] reports until a preset names it via
/// [`CompatEngine::with_provider_name`].
pub const DEFAULT_PROVIDER_NAME: &str = "openai-compatible";

impl CompatEngine {
    /// Construct a new engine from preset-supplied configuration.
    ///
    /// # Errors
    /// Returns [`LlmError::AuthenticationError`] if the API key cannot be
    /// encoded as a header value, or [`LlmError::NetworkError`] if the
    /// underlying HTTP client cannot be built.
    pub fn new(mut config: CompatEngineConfig) -> Result<Self, LlmError> {
        let timeout = Duration::from_secs(config.timeout_seconds);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key)).map_err(|e| {
                LlmError::AuthenticationError(format!("Invalid API key format: {}", e))
            })?,
        );

        let mut client_builder = Client::builder().timeout(timeout).default_headers(headers);
        if let Some(policy) = config.redirect_policy.take() {
            client_builder = client_builder.redirect(policy);
        }

        let client = client_builder
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            config,
            models_cache: OnceCell::new(),
            provider_name: DEFAULT_PROVIDER_NAME,
            model_registry: None,
            default_max_tokens: None,
            default_reasoning: (None, None),
            omissions: OmissionLog::new(),
            format_log: FormatLog::new(),
            usage_semantics: CompatUsageSemantics::default(),
            usage_log: UsageLog::new(),
            substitutions: SubstitutionLog::new(),
        })
    }

    /// Name the preset this engine serves, so every
    /// [`LlmError::ProviderError`] it emits carries that name in its
    /// `provider` field (Phase 25 D-03) rather than the generic
    /// [`DEFAULT_PROVIDER_NAME`]. Each thin preset calls this with the same
    /// literal its `LlmPort::get_provider_name` returns.
    #[must_use]
    pub fn with_provider_name(mut self, provider_name: &'static str) -> Self {
        self.provider_name = provider_name;
        self
    }

    /// Attach the per-model profile registry this engine resolves every request's model through
    /// (ADR-0061, D-05). Each thin preset calls this with a registry over its adapter-wide
    /// fallback profile, in the [`with_provider_name`](Self::with_provider_name) style.
    #[must_use]
    pub(crate) fn with_model_registry(mut self, registry: ModelRegistry) -> Self {
        self.model_registry = Some(registry);
        self
    }

    /// Set the output budget sent when a request leaves `max_tokens` unset (PARITY-03, D-12).
    ///
    /// It is the effective `max_tokens` from then on: it is gated by the preset's
    /// `request_parameters.max_tokens` declaration and checked against the request model's
    /// profiled maximum output by [`sampling::check_max_tokens`], exactly as a request's own
    /// value is. A request's own `max_tokens` always wins over it.
    #[must_use]
    pub(crate) fn with_default_max_tokens(mut self, max_tokens: Option<u32>) -> Self {
        self.default_max_tokens = max_tokens;
        self
    }

    /// Set the provider-level default reasoning effort and fallback (Phase 51, D-00e), from the
    /// preset config's `reasoning_effort` / `reasoning_fallback`.
    ///
    /// Applied per request, below the request's own value and against the request's own model: a
    /// request that sets an effort (or a fallback) always wins field by field, and with both
    /// unset here and on the request nothing changes. A refusal of the default names
    /// `llm.<provider>.reasoning_effort`.
    #[must_use]
    pub(crate) fn with_default_reasoning(
        mut self,
        effort: Option<ReasoningEffort>,
        fallback: Option<ReasoningFallback>,
    ) -> Self {
        self.default_reasoning = (effort, fallback);
        self
    }

    /// Declare how this preset's vendor reports reasoning tokens against `completion_tokens`
    /// (G-50-5a), in the [`with_model_registry`](Self::with_model_registry) style. Without it the
    /// engine reads OpenAI's inclusive contract
    /// ([`CompatUsageSemantics::ReasoningInsideCompletion`]).
    #[must_use]
    // Called by the Grok preset only today; the other presets keep the default.
    #[cfg_attr(not(feature = "grok"), allow(dead_code))]
    pub(crate) fn with_usage_semantics(mut self, semantics: CompatUsageSemantics) -> Self {
        self.usage_semantics = semantics;
        self
    }

    /// The profile that decides what reaches the wire for `model`: the registry's (override over
    /// seeded entry over adapter-wide fallback), or, when no registry was attached, one derived
    /// from this config's own capabilities and `request_parameters` with stop sequences not sent.
    ///
    /// Its `effort` data is masked by this preset's declared [`ReasoningWire`] (Phase 51, D-00f):
    /// whatever the wire has no field for is removed, so a caller reading this profile (the
    /// builder, the fallback chain) judges exactly what [`CompatEngine::build_request`] will send.
    pub(crate) fn model_profile(&self, model: &str) -> ModelProfile {
        let mut profile = match &self.model_registry {
            Some(registry) => registry.profile(model),
            None => fallback_from_compat(
                &self.capabilities(),
                &self.config.request_parameters,
                StopSupport::NotSent,
            ),
        };
        profile.effort = mask_effort(
            std::mem::take(&mut profile.effort),
            wire_caps(self.config.request_parameters.reasoning),
        );
        profile
    }

    /// The pre-judging verdict for a reasoning `effort` / `fallback` on `model` (Phase 51, D-09):
    /// whether [`CompatEngine::build_request`] would admit it, without building a request.
    ///
    /// The send-time twin of `build_request`'s `in_force` + `admit`: the same
    /// [`model_profile`](Self::model_profile) (masked by the preset's declared [`ReasoningWire`]),
    /// the same provider defaults ([`with_default_reasoning`](Self::with_default_reasoning)) and
    /// the same `judge`, so a preset's build-time check cannot read a different profile, wire
    /// mask or default than its send path. The per-request budget-versus-`max_tokens` rule is
    /// left to send time. No [`SubstitutionLog`] is touched
    /// and no network call is made.
    ///
    /// # Errors
    ///
    /// [`LlmError::UnsupportedParameter`], with the reason text `build_request` would produce.
    pub(crate) fn check_reasoning(
        &self,
        model: &str,
        effort: Option<ReasoningEffort>,
        fallback: Option<ReasoningFallback>,
    ) -> Result<(), LlmError> {
        reasoning::check(
            self.provider_name,
            model,
            || self.model_profile(model),
            effort,
            fallback,
            self.default_reasoning.0,
            self.default_reasoning.1,
        )
    }

    /// The preset's declared request parameters, for a preset's own position test.
    #[cfg(test)]
    pub(crate) fn request_parameters(&self) -> CompatRequestParameters {
        self.config.request_parameters
    }

    /// The name this engine stamps on its [`LlmError::ProviderError`]s.
    pub fn provider_name(&self) -> &'static str {
        self.provider_name
    }

    /// The preset's declared capabilities, converted to the port's type.
    pub fn capabilities(&self) -> ProviderCapabilities {
        self.config.capabilities.clone().into()
    }

    /// Build the outgoing request body from a port-level [`LlmRequest`], plus the notes the
    /// buffered path records on its response.
    ///
    /// What reaches the wire is decided by the request model's [`ModelProfile`] (ADR-0061),
    /// resolved through [`CompatEngine::model_profile`]: [`sampling::apply`] sends a sampling
    /// parameter only when the profile admits it, and a set parameter it does not admit is
    /// **omitted** -- the key is absent from the JSON body (`CompatRequest`'s
    /// `skip_serializing_if = "Option::is_none"`), never `null`, never a substituted legal value
    /// -- and recorded: one `warn!` per instance per (model, parameter), and
    /// `paladin.params_omitted` on the response. Stop sequences ride the same filter and reach
    /// the wire in the order the caller declared them.
    ///
    /// `max_tokens` is the one parameter still gated by this engine's own
    /// [`CompatRequestParameters`] declaration; an effective `max_tokens` above the profile's
    /// `max_output_tokens` is refused by [`sampling::check_max_tokens`] before anything is built.
    ///
    /// The gate reads only the profile and `self.config.request_parameters`, never the provider
    /// name or base URL, so the same mechanism serves the next vendor without editing this method.
    ///
    /// This method backs **five presets** sharing this one engine: Kimi, Qwen, Grok, Ollama and
    /// the generic OpenAI-compatible preset -- a change here (e.g. the `response_format` mapping
    /// below, D-28) is shared by all five, not per-vendor.
    fn build_request(
        &self,
        request: &LlmRequest,
    ) -> Result<(CompatRequest, RequestNotes), LlmError> {
        let messages = Self::convert_prompt_to_messages(&request.prompt)?;
        let params = &request.prompt.entity.data.parameters;
        let profile = self.model_profile(&request.model);

        // The configured default output budget (PARITY-03) stands in for a request that sets
        // none; either way the effective value is gated by this preset's declaration and checked
        // against the profile below, so a default is held to the same limit as a caller's value.
        let max_tokens = params
            .max_tokens
            .or(self.default_max_tokens)
            .filter(|_| self.config.request_parameters.max_tokens);
        sampling::check_max_tokens(self.provider_name, &request.model, &profile, max_tokens)?;
        if params.max_tokens.is_some() && max_tokens.is_none() {
            log::warn!(
                target: "paladin::sampling",
                "provider '{}' model '{}': this preset's request path does not carry \
                 'max_tokens'; the caller's value was dropped from the outgoing body",
                self.provider_name,
                request.model
            );
        }

        // Phase 51 (D-00f): an explicit effort is sent exactly or refused here, before any HTTP
        // call and before any retry loop -- never routed through the omit-and-log path below.
        // `profile` is already masked by this preset's declared wire.
        let in_force = reasoning::in_force(
            self.provider_name,
            request.reasoning,
            request.reasoning_fallback,
            self.default_reasoning.0,
            self.default_reasoning.1,
        );
        let resolved = reasoning::admit(
            self.provider_name,
            &request.model,
            &profile,
            in_force.effort,
            in_force.fallback,
            max_tokens,
            in_force.source_key.as_deref(),
            &self.substitutions,
        )?;
        let fields = match &resolved {
            Some(resolved) => map_reasoning(
                self.provider_name,
                &request.model,
                self.config.request_parameters.reasoning,
                resolved,
            )?,
            None => CompatReasoningFields::default(),
        };

        let applied = sampling::apply(&profile, params);
        let params_omitted = self
            .omissions
            .record(self.provider_name, &request.model, &applied);

        // D-28 / ADR-0061 (i): the effective `response_format` follows the profile. These five
        // presets speak the de-facto chat-completions dialect without OpenAI's schema-carrying
        // extension, so an effective format -- object or schema -- maps to this engine's plain
        // JSON-object wire form; the compat fallback profile (`json_object` true, `json_schema`
        // false) makes a schema request a recorded downgrade. The field is present whenever an
        // effective format exists (never `null`, EDGE(RT-05/wire shape)); correctness never
        // depends on it because the caller also appends the schema-conformance instruction block
        // (D-27).
        let (effective_format, adjustment) =
            sampling::apply_response_format(&profile, request.response_format.as_ref());
        let format_adjusted =
            self.format_log
                .record(self.provider_name, &request.model, adjustment);
        let response_format = effective_format.map(|_| CompatResponseFormat {
            kind: "json_object",
        });

        let unprofiled_model = self
            .model_registry
            .as_ref()
            .is_some_and(|registry| registry.note_unprofiled(&request.model))
            .then(|| request.model.clone());

        let body = CompatRequest {
            model: request.model.clone(),
            messages,
            temperature: applied.temperature,
            max_tokens,
            top_p: applied.top_p,
            frequency_penalty: applied.frequency_penalty,
            presence_penalty: applied.presence_penalty,
            stop: applied.stop,
            stream: request.stream,
            response_format,
            // Set unconditionally to `None` here; `generate_stream` forces
            // it to `Some` right after building this request, mirroring the
            // existing defensive `api_request.stream = true` override
            // (D-13/D-15) -- `request.stream` is not a reliable signal at
            // this point since callers such as the execution service build
            // the `LlmRequest` and only decide to call `generate_stream`
            // afterward.
            stream_options: None,
            reasoning_effort: fields.reasoning_effort,
            thinking: fields.thinking,
            enable_thinking: fields.enable_thinking,
            thinking_budget: fields.thinking_budget,
        };
        let notes = RequestNotes {
            params_omitted,
            format_adjusted,
            unprofiled_model,
            reasoning_requested: resolved.map(|r| r.requested.to_string()),
            reasoning_effective: resolved.map(|r| r.effective.to_string()),
        };
        Ok((body, notes))
    }

    /// Map a wire-reported [`CompatUsage`] into [`TokenUsage`], applying the
    /// D-20 cache/reasoning sub-count builders only when the provider's
    /// payload actually carried the figure (D-03: an absent figure is
    /// `None`, never a fabricated `Some(0)`). Shared by both the
    /// non-streaming usage-construction site and the streaming terminal
    /// -chunk usage, so the two paths cannot drift.
    ///
    /// Cache-read, cache-write and reasoning counts are copied from the vendor's fields exactly
    /// as reported. `completion_tokens` follows the preset's declared `semantics`: copied as is
    /// when the vendor reports reasoning inside it, and with the reported reasoning added
    /// (saturating) when the vendor reports it beside it, so the mapped `prompt + completion`
    /// equals what the vendor bills. A reasoning figure of `None` or `Some(0)` leaves
    /// completion unchanged.
    ///
    /// Returns the mapped usage together with the vendor's own `total_tokens` (when it reported
    /// one). The vendor total is parsed and handed to the reconciliation, never copied:
    /// [`TokenUsage`] guarantees `total_tokens == prompt_tokens + completion_tokens` (D-02), so
    /// a disagreement is recorded under [`USAGE_UNRECONCILED_METADATA_KEY`], not written over
    /// the mapped total.
    fn map_compat_usage(
        usage: CompatUsage,
        semantics: CompatUsageSemantics,
    ) -> (TokenUsage, Option<u32>) {
        let reasoning = usage
            .completion_tokens_details
            .and_then(|details| details.reasoning_tokens);
        let completion = match semantics {
            CompatUsageSemantics::ReasoningInsideCompletion => usage.completion_tokens,
            CompatUsageSemantics::ReasoningBesideCompletion => usage
                .completion_tokens
                .saturating_add(reasoning.unwrap_or(0)),
        };
        let mut mapped = TokenUsage::new(usage.prompt_tokens, completion);
        if let Some(details) = usage.prompt_tokens_details {
            if let Some(cached) = details.cached_tokens {
                mapped = mapped.with_cache_read(cached);
            }
            if let Some(written) = details.cache_write_tokens {
                mapped = mapped.with_cache_write(written);
            }
        }
        if let Some(reasoning) = reasoning {
            mapped = mapped.with_reasoning(reasoning);
        }
        (mapped, usage.total_tokens)
    }

    /// Convert a `PromptItem` into the wire message list. Provider-agnostic
    /// — every `PromptType` arm the DeepSeek adapter covers is handled here.
    fn convert_prompt_to_messages(prompt: &PromptItem) -> Result<Vec<CompatMessage>, LlmError> {
        let mut messages = Vec::new();

        match &prompt.entity.data.prompt_type {
            PromptType::System(system_prompt) => {
                messages.push(CompatMessage {
                    role: "system".to_string(),
                    content: system_prompt.instructions.clone(),
                    reasoning_content: None,
                    refusal: None,
                });
            }
            PromptType::User(user_prompt) => {
                messages.push(CompatMessage {
                    role: "user".to_string(),
                    content: crate::turn::user_turn_text(user_prompt),
                    reasoning_content: None,
                    refusal: None,
                });
            }
            PromptType::Text(text_prompt) => {
                messages.push(CompatMessage {
                    role: match text_prompt.role {
                        paladin_core::platform::container::prompt::PromptRole::System => "system",
                        paladin_core::platform::container::prompt::PromptRole::User => "user",
                        paladin_core::platform::container::prompt::PromptRole::Assistant => {
                            "assistant"
                        }
                        paladin_core::platform::container::prompt::PromptRole::Function => {
                            "function"
                        }
                    }
                    .to_string(),
                    content: text_prompt.content.clone(),
                    reasoning_content: None,
                    refusal: None,
                });
            }
            PromptType::Assistant(assistant_prompt) => {
                messages.push(CompatMessage {
                    role: "assistant".to_string(),
                    content: assistant_prompt.response.clone(),
                    reasoning_content: None,
                    refusal: None,
                });
            }
            PromptType::Function(function_prompt) => {
                messages.push(CompatMessage {
                    role: "function".to_string(),
                    content: function_prompt.function_name.clone(),
                    reasoning_content: None,
                    refusal: None,
                });
            }
        }

        if messages.is_empty() {
            return Err(LlmError::InvalidPrompt(
                "Prompt must contain at least one message".to_string(),
            ));
        }

        Ok(messages)
    }

    /// Map a compatible-protocol finish reason string to [`FinishReason`].
    ///
    /// Any string this engine does not recognise maps to
    /// [`FinishReason::Error`] rather than silently to `Stop` — an unknown
    /// finish reason is a signal worth surfacing, not swallowing (PROV-02
    /// adjacency edge). An absent/`null` finish reason maps to `Stop`.
    fn map_finish_reason(reason: Option<String>) -> FinishReason {
        match reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            Some("function_call") => FinishReason::FunctionCall,
            Some(other) => FinishReason::Error(format!("Unknown finish reason: {}", other)),
            None => FinishReason::Stop,
        }
    }

    /// Render untrusted provider text as a log-safe diagnostic excerpt.
    /// Redaction runs before truncation — see [`crate::redaction`].
    fn diagnostic_excerpt(&self, body: &str) -> String {
        redact_and_bound(body, &self.config.api_key)
    }

    /// Map a non-2xx HTTP status + RAW response body to [`LlmError`].
    ///
    /// Checks the preset's `error_override` first (handed the redacted,
    /// bounded excerpt, as before) so a preset-specific status can be added
    /// without editing this engine. Everything else is the crate-wide
    /// [`map_http_status_with_hints`] (Phase 25 D-03, FT-FR-01): it redacts `body`
    /// before bounding it and emits a typed `ProviderError { status }` for
    /// every status without a dedicated variant — which is why callers pass
    /// the raw body here rather than a pre-built excerpt (bounding twice
    /// would truncate the first elision marker).
    ///
    /// A thin wrapper over [`Self::map_error_with_hints`], kept so the status-mapping unit
    /// tests read the same as before PACE-01. Production paths call the hints variant.
    #[cfg(test)]
    fn map_error(&self, status: u16, body: &str) -> LlmError {
        self.map_error_with_hints(status, body, None)
    }

    /// Map a non-2xx HTTP status + RAW response body to [`LlmError`], carrying the parsed
    /// rate-limit headers of a `429` (PACE-01). See `map_error` for the rules; the
    /// preset's `error_override` still runs first and is handed no hints.
    fn map_error_with_hints(
        &self,
        status: u16,
        body: &str,
        hints: Option<RateLimitHints>,
    ) -> LlmError {
        if let Some(override_fn) = self.config.error_override
            && let Some(err) = override_fn(status, &self.diagnostic_excerpt(body))
        {
            return err;
        }

        match status {
            // WR-04 (`17-REVIEW.md`, T-17-52): every preset builds its
            // client with `redirect_policy: Some(Policy::none())`, so a
            // `3xx` response is never followed — it arrives here as an
            // ordinary non-success status instead. Named explicitly rather
            // than left to the helper's generic arm, so the operator whose
            // previously-working endpoint now fails gets an actionable
            // message: which setting to check, not an opaque status.
            //
            // The VARIANT is the same one the helper would choose for a
            // `3xx` (`ProviderError`, classified Permanent by value —
            // FT-FR-01); only the message is enriched. This engine's retry
            // loop (`call_api_with_retry`) still retries it up to
            // `max_retries` before it surfaces, the accepted cost recorded
            // as T-17-54; the "no new variant" rationale that once forced
            // this onto `ProcessingError` is superseded by plan 25-02's
            // `ProviderError`.
            300..=399 => LlmError::ProviderError {
                provider: self.provider_name.to_string(),
                status,
                message: format!(
                    "the configured base URL responded with a redirect (HTTP {status}), which \
                     this client refuses to follow because doing so would forward the \
                     credential header to a different, potentially attacker-influenced host. \
                     Correct the configured base-URL setting to point directly at the intended \
                     endpoint. Response excerpt: {}",
                    self.diagnostic_excerpt(body)
                ),
            },
            _ => map_http_status_with_hints(
                self.provider_name,
                status,
                body,
                &self.config.api_key,
                hints,
            ),
        }
    }

    /// Perform an API call with exponential-backoff-plus-jitter retry.
    ///
    /// Non-retryable set: `AuthenticationError | InvalidPrompt |
    /// EmptyCompletion | Refusal | UsageLimitExceeded | RateLimitExceeded` — matching
    /// `deepseek/adapter.rs::call_api_with_retry`'s rationale: these need
    /// operator intervention or will not clear on backoff, so retrying
    /// burns attempts for no benefit.
    ///
    /// `RateLimitExceeded` is in the set for a different reason (Phase 43
    /// D-02): a 429 is the one error whose cure is *waiting*, and the
    /// Cadence decorator above this engine owns waiting, with a gate shared
    /// by every caller of the same provider and model. Retrying it here would
    /// re-send into a limit the provider just announced and hide the thrash
    /// below the decorator, so the first 429 is surfaced carrying the
    /// provider's own `Retry-After`. Network, timeout and 5xx retries are
    /// unchanged.
    async fn call_api_with_retry<F, Fut, T>(
        &self,
        operation: F,
        max_retries: u32,
    ) -> Result<T, LlmError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, LlmError>>,
    {
        let mut last_error: Option<LlmError> = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if matches!(
                        e,
                        LlmError::AuthenticationError(_)
                            | LlmError::InvalidPrompt(_)
                            | LlmError::EmptyCompletion { .. }
                            | LlmError::Refusal { .. }
                            | LlmError::UsageLimitExceeded { .. }
                            // D-02: surface the first 429; the Cadence decorator waits.
                            | LlmError::RateLimitExceeded { .. }
                    ) {
                        return Err(e);
                    }

                    if attempt >= max_retries {
                        return Err(e);
                    }

                    let backoff = exponential_backoff(
                        Duration::from_millis(100),
                        attempt,
                        Duration::from_secs(10),
                    );
                    let jitter = Duration::from_millis(rand::random::<u64>() % 100);
                    tokio::time::sleep(backoff + jitter).await;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            LlmError::ProcessingError("Retry logic failed unexpectedly".to_string())
        }))
    }

    /// Generate a completion. POSTs to `{base_url}/chat/completions`.
    pub async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let (api_request, notes) = self.build_request(&request)?;
        // ADR-0061 visibility rule: whatever the filter omitted or adjusted is stamped on every
        // response this request yields, built once and cloned per retry attempt.
        let metadata = notes.into_metadata();
        let url = format!("{}/chat/completions", self.config.base_url);

        let operation = || async {
            let response = self
                .client
                .post(&url)
                .json(&api_request)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        LlmError::Timeout(format!(
                            "API request timed out after {} seconds",
                            self.config.timeout_seconds
                        ))
                    } else {
                        LlmError::NetworkError(format!("Failed to send request: {}", e))
                    }
                })?;

            let status = response.status();

            if !status.is_success() {
                // Snapshot the 429 headers BEFORE `.text()` consumes the response
                // (research Pitfall 2).
                let hints = snapshot_rate_limit_hints(status, response.headers());
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error_with_hints(status.as_u16(), &error_text, hints));
            }

            // Read the body to text FIRST, then deserialize it separately —
            // never `Response::json()` (RESEARCH.md Pitfall 3). Splitting the
            // two steps distinguishes a transport failure from a schema
            // mismatch, both in the error type and in the message.
            let body = response.text().await.map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(format!(
                        "Response body did not finish streaming within the {}s client timeout: {e}",
                        self.config.timeout_seconds
                    ))
                } else {
                    LlmError::NetworkError(format!("Failed to read response body: {e}"))
                }
            })?;

            let api_response: CompatResponse = serde_json::from_str(&body).map_err(|e| {
                LlmError::ProcessingError(format!(
                    "Failed to parse response (schema mismatch — field: {e}) — body excerpt: {}",
                    self.diagnostic_excerpt(&body)
                ))
            })?;

            // G-50-5a: usage is mapped BEFORE the guard (and before the no-choices check), so a
            // call the vendor billed but that is classified as empty or refused still carries
            // its bill on the error. The billed model is the served model the body names, else
            // the request model (A3-50-26). No usage object means no bill, never a fabricated
            // zero (D-00c).
            let mapped = api_response
                .usage
                .map(|usage| Self::map_compat_usage(usage, self.usage_semantics));
            let billed_model = if api_response.model.is_empty() {
                request.model.as_str()
            } else {
                api_response.model.as_str()
            };

            let Some(choice) = api_response.choices.first() else {
                return Err(bill(
                    LlmError::empty_completion("Response contained no choices".to_string()),
                    billed_model,
                    mapped.map(|(usage, _)| usage),
                ));
            };

            let finish_reason = Self::map_finish_reason(choice.finish_reason.clone());

            // One rule for refusal, content filter, function call and empty text on any
            // finish reason (PARITY-05, D-09/D-11a).
            classify_completion(
                self.provider_name(),
                &request.model,
                &choice.message.content,
                &finish_reason,
                choice.message.refusal.as_deref(),
                &self.config.api_key,
            )
            .map_err(|err| {
                bill(
                    err,
                    billed_model,
                    mapped.as_ref().map(|(usage, _)| usage.clone()),
                )
            })?;

            // A successful response must still report its usage: the field was required
            // before G-50-5a made it optional for billed failures, and the same schema-mismatch
            // error is kept for a body without it.
            let Some((usage, vendor_total)) = mapped else {
                return Err(LlmError::ProcessingError(format!(
                    "Failed to parse response (schema mismatch — field: missing field `usage`) — \
                     body excerpt: {}",
                    self.diagnostic_excerpt(&body)
                )));
            };
            // G-50-5a: the vendor's own total is the check on the mapping -- a disagreement is
            // warned once and recorded, never written over the mapped total (D-02).
            let unreconciled =
                self.usage_log
                    .record(self.provider_name(), &request.model, &usage, vendor_total);

            let mut llm_response = LlmResponse::new(
                request.id,
                api_response.model,
                choice.message.content.clone(),
                finish_reason,
                usage,
            );
            llm_response.metadata = metadata.clone();
            if let Some(value) = unreconciled {
                llm_response
                    .metadata
                    .insert(USAGE_UNRECONCILED_METADATA_KEY.to_string(), value);
            }
            if let Some(reasoning) = &choice.message.reasoning_content {
                llm_response = llm_response.with_reasoning(reasoning.as_str());
            }
            Ok(llm_response)
        };

        self.call_api_with_retry(operation, self.config.max_retries)
            .await
    }

    /// Generate a streaming completion. POSTs with `stream: true` and
    /// assembles SSE `data: {...}` chunks, terminating on the literal
    /// `[DONE]` sentinel.
    ///
    /// ## Retry (WR-04, `17-REVIEW.md`)
    ///
    /// Only the connection-opening POST is retried — through the same
    /// `call_api_with_retry` helper and the same `self.config.max_retries`
    /// [`generate`](Self::generate) passes — so a transient failure opening
    /// the stream is retried exactly as many times as `generate()` retries
    /// the identical failure. The same non-retryable set applies: an
    /// authentication failure, an invalid prompt, an already-empty
    /// completion or a usage-limit rejection is attempted exactly once,
    /// never replaying a live `Authorization` credential to an endpoint
    /// that has already rejected it.
    ///
    /// Once the response is opened, `.bytes_stream()` is consumed exactly
    /// once, **outside** the retry loop: the byte stream itself is never
    /// re-read or re-opened, so a caller can never observe a duplicated or
    /// reordered delta as a result of this retry.
    ///
    /// **Cost:** each retried open re-sends the caller's entire prompt to
    /// the provider, so a transient failure can bill the prompt more than
    /// once. The non-retryable set above exists precisely so a failure the
    /// provider has already answered definitively is never retried.
    ///
    /// ## Terminal item (D-09, D-11a)
    ///
    /// Each delta carries the vendor's reasoning text for that chunk on
    /// [`StreamingResponse::reasoning`], in wire order. At the `[DONE]`
    /// sentinel the stream is classified by the same rule as
    /// [`generate`](Self::generate): a refusal ends the stream with
    /// `Err(LlmError::Refusal)`, and a stream that carried no text that is
    /// non-empty after trimming ends with `Err(LlmError::EmptyCompletion)`
    /// instead of the `Ok` terminal chunk (a `content_filter` or function-call
    /// finish stays `Ok`). Deltas already yielded, reasoning deltas included,
    /// stay yielded: only the terminal item changes.
    ///
    /// The `Ok` terminal chunk carries the usage frame mapped with the preset's
    /// usage semantics (G-50-5a), and, when that usage does not reconcile with
    /// the vendor's own total or breaks containment, the
    /// [`USAGE_UNRECONCILED_METADATA_KEY`] finding in its
    /// [`StreamingResponse::metadata`] -- beside any ADR-0061 omission keys.
    /// Content chunks never carry it, and an `Err` terminal item carries no
    /// metadata at all. An `Err(EmptyCompletion)` or `Err(Refusal)` terminal
    /// item carries the usage frame's figures as its billed usage
    /// ([`LlmError::billed_usage`], G-50-5a) on the request model, or `None`
    /// when the stream reported no usage.
    pub async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        // ADR-0061 (h), owner reading R-1 (option-b): the omissions, adjustments and unprofiled
        // model `build_request` already `warn!`ed about are also stamped on the stream's
        // terminal chunk (and only that chunk), exactly as the buffered path stamps them on
        // `LlmResponse.metadata`.
        let (mut api_request, notes) = self.build_request(&request)?;
        let stream_metadata = notes.into_metadata();
        api_request.stream = true;
        api_request.stream_options = Some(CompatStreamOptions {
            include_usage: true,
        });

        let url = format!("{}/chat/completions", self.config.base_url);

        // Only the connection-opening POST is retried — the byte stream is
        // deliberately consumed OUTSIDE this closure, once, after the retry
        // loop returns below. Re-opening a stream whose first chunks a
        // caller has already consumed would deliver the same tokens twice
        // with no marker (WR-04, `17-REVIEW.md`; T-17-71).
        let operation = || async {
            let response = self
                .client
                .post(&url)
                .json(&api_request)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        LlmError::Timeout(format!(
                            "API request timed out after {} seconds",
                            self.config.timeout_seconds
                        ))
                    } else {
                        LlmError::NetworkError(format!("Failed to send streaming request: {}", e))
                    }
                })?;

            let status = response.status();
            if !status.is_success() {
                // Same header snapshot as the buffered path, taken before the body read.
                let hints = snapshot_rate_limit_hints(status, response.headers());
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(self.map_error_with_hints(status.as_u16(), &error_text, hints));
            }

            Ok(response)
        };

        let response = self
            .call_api_with_retry(operation, self.config.max_retries)
            .await?;

        let stream = response.bytes_stream();

        // `flat_map` rather than `map`: a single network chunk can carry more
        // than one complete SSE `data: {...}` event (this is common when a
        // mock transport — or a provider whose TCP framing does not align to
        // event boundaries — writes the whole body at once). Returning after
        // the FIRST matching line per chunk would silently drop every
        // subsequent delta in that chunk; every `data:` line found is
        // therefore emitted as its own stream item.
        //
        // D-14 hold-and-emit terminal-chunk contract: a `choices[].finish_reason`
        // frame and the trailing empty-`choices` usage frame can arrive on
        // SEPARATE SSE frames, both strictly before `[DONE]`. Rather than
        // emitting `finish_reason`/`usage` on whichever frame happens to
        // carry them, this holds both in state captured by the `move`
        // closure (a `Stream::flat_map` closure is `FnMut`, so ordinary
        // mutable locals persist correctly across calls) and emits them
        // together on the ONE `[DONE]` terminal chunk -- delta frames are
        // still emitted immediately, in wire order, carrying neither.
        let mut held_finish_reason: Option<FinishReason> = None;
        let mut held_usage: Option<TokenUsage> = None;
        // The vendor's own total from the held usage frame, reconciled at the terminal chunk
        // (G-50-5a) against the usage mapped with this preset's semantics.
        let mut held_vendor_total: Option<u32> = None;
        let usage_semantics = self.usage_semantics;
        let usage_log = self.usage_log.clone();

        // D-11a / D-09 on the streaming path: the terminal chunk is decided by the same
        // `classify_completion` the buffered path uses, over what the stream accumulated.
        let provider = self.provider_name();
        let model = request.model.clone();
        let api_key = self.config.api_key.clone();
        let mut completion = StreamCompletion::default();

        // A network read can end inside a `data:` line (or inside a multi-byte character), so the
        // bytes are reassembled into whole lines before any line is parsed (CR-02).
        let mut sse = SseLines::default();
        let llm_stream = stream
            .map(Some)
            .chain(futures::stream::iter([None]))
            .flat_map(move |chunk_option| {
                // `None` is the end of the body: it releases a final line that had no newline.
                let chunk_result = match chunk_option {
                    Some(read) => read.map(|bytes| sse.push(&bytes)),
                    None => Ok(sse.finish()),
                };
                let items: Vec<Result<StreamingResponse, LlmError>> = match chunk_result {
                    Ok(lines) => {
                        let mut items = Vec::new();

                        for line in &lines {
                            let Some(json_str) = line.strip_prefix("data: ") else {
                                continue;
                            };

                            if json_str.trim() == "[DONE]" {
                                let finish_reason =
                                    held_finish_reason.take().unwrap_or(FinishReason::Stop);
                                match completion.finish(provider, &model, &finish_reason, &api_key)
                                {
                                    Ok(()) => {
                                        let mut terminal =
                                            StreamingResponse::terminal(finish_reason);
                                        if let Some(usage) = held_usage.take() {
                                            // G-50-5a: only the `Ok` terminal chunk carries
                                            // a finding; an `Err` terminal carries no
                                            // metadata (50-18 rule).
                                            if let Some(value) = usage_log.record(
                                                provider,
                                                &model,
                                                &usage,
                                                held_vendor_total.take(),
                                            ) {
                                                terminal = terminal.with_metadata(
                                                    USAGE_UNRECONCILED_METADATA_KEY,
                                                    value,
                                                );
                                            }
                                            terminal = terminal.with_usage(usage);
                                        }
                                        for (key, value) in &stream_metadata {
                                            terminal = terminal
                                                .with_metadata(key.as_str(), value.as_str());
                                        }
                                        items.push(Ok(terminal));
                                    }
                                    // A refusal or an empty answer replaces the terminal chunk;
                                    // deltas already yielded (reasoning included) stay yielded.
                                    // G-50-5a: the error carries the usage frame's bill on the
                                    // request model, `None` when the stream reported none.
                                    Err(error) => {
                                        items.push(Err(bill(error, &model, held_usage.take())))
                                    }
                                }
                                continue;
                            }

                            match serde_json::from_str::<super::types::CompatStreamResponse>(
                                json_str,
                            ) {
                                Ok(response) => {
                                    if let Some(usage) = response.usage {
                                        let (mapped, vendor_total) =
                                            Self::map_compat_usage(usage, usage_semantics);
                                        held_usage = Some(mapped);
                                        held_vendor_total = vendor_total;
                                    }
                                    if let Some(choice) = response.choices.first() {
                                        if let Some(reason) = &choice.finish_reason {
                                            held_finish_reason =
                                                Some(Self::map_finish_reason(Some(reason.clone())));
                                        }
                                        let content =
                                            choice.delta.content.clone().unwrap_or_default();
                                        completion.observe_content(&content);
                                        if let Some(refusal) = &choice.delta.refusal {
                                            completion.observe_refusal(refusal);
                                        }
                                        let mut delta = StreamingResponse::delta(content);
                                        if let Some(reasoning) = &choice.delta.reasoning_content {
                                            delta = delta.with_reasoning(reasoning.as_str());
                                        }
                                        items.push(Ok(delta));
                                    }
                                }
                                Err(e) => {
                                    items.push(Err(LlmError::ProcessingError(format!(
                                        "Failed to parse streaming response: {}",
                                        e
                                    ))));
                                }
                            }
                        }

                        items
                    }
                    Err(e) => vec![Err(LlmError::NetworkError(format!("Stream error: {}", e)))],
                };

                futures::stream::iter(items)
            });

        Ok(Box::new(llm_stream))
    }

    /// Fetch the live model list from `GET {base_url}/models`.
    async fn fetch_live_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.config.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout(format!(
                    "Model list request timed out after {} seconds",
                    self.config.timeout_seconds
                ))
            } else {
                LlmError::NetworkError(format!("Failed to fetch model list: {}", e))
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let hints = snapshot_rate_limit_hints(status, response.headers());
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.map_error_with_hints(status.as_u16(), &error_text, hints));
        }

        let body = response
            .text()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Failed to read model list body: {e}")))?;

        let parsed: CompatModelsResponse = serde_json::from_str(&body).map_err(|e| {
            LlmError::ProcessingError(format!(
                "Failed to parse model list (schema mismatch: {e}) — body excerpt: {}",
                self.diagnostic_excerpt(&body)
            ))
        })?;

        Ok(parsed.data.into_iter().map(|entry| entry.id).collect())
    }

    /// Resolve the model list: live on first call, memoized for this
    /// engine's lifetime (D-13/D-14). Falls back to the preset's curated
    /// list on any failure or an empty live response — **the fallback
    /// contract itself is deliberately unchanged by G-17-4d (2026-08-22,
    /// plan 17-22)**: this method still returns `Vec<String>`, still never
    /// errors, and still returns the curated list on every failure path. See
    /// the tests below (`available_models_returns_curated_fallback_on_*`)
    /// for the pin.
    ///
    /// What DID change (G-17-4d, 2026-08-22): the `Err(e)` arm used to log
    /// every failure at `debug` with the same sentence, which is precisely
    /// why a region/credential mismatch looked identical to an offline
    /// vendor and G-17-4c was misdiagnosed for five days. It now reads
    /// `classify_fetch_failure`'s verdict on `e` and only a
    /// misconfiguration (currently: `AuthenticationError`) is raised to
    /// `warn`; every other failure — including the offline/timeout states
    /// D-13/D-14 were written for — keeps its original `debug` wording
    /// unchanged. The empty-live-list arm above is untouched; an endpoint
    /// answering with an empty catalogue is neither an auth failure nor an
    /// offline one. `tokio::sync::OnceCell` still ensures exactly one fetch
    /// even under concurrent callers, so a rejected credential produces at
    /// most one warning per engine, not one per caller.
    pub async fn available_models(&self) -> Vec<String> {
        self.models_cache
            .get_or_init(|| async {
                match self.fetch_live_models().await {
                    Ok(models) if !models.is_empty() => models,
                    Ok(_) => {
                        log::debug!(
                            "live model list was empty; falling back to curated list (not authoritative)"
                        );
                        self.config.fallback_models.clone()
                    }
                    Err(e) => {
                        match classify_fetch_failure(&e) {
                            FetchFailureClass::Misconfiguration => {
                                log::warn!(
                                    "configured endpoint {} rejected the request while listing \
                                     models ({e}); the returned model list is the curated \
                                     fallback, not this vendor's own catalog — a credential \
                                     scoped to a different account or region is the usual cause",
                                    base_url_without_userinfo(&self.config.base_url)
                                );
                            }
                            FetchFailureClass::Supported => {
                                log::debug!(
                                    "live model list fetch failed ({e}); falling back to curated list (not authoritative)"
                                );
                            }
                        }
                        self.config.fallback_models.clone()
                    }
                }
            })
            .await
            .clone()
    }

    /// `true` when `model` appears in the resolved list (live or fallback).
    pub async fn validate_model(&self, model: &str) -> bool {
        self.available_models().await.iter().any(|m| m == model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::UNPROFILED_MODEL_METADATA_KEY;
    use crate::sampling::{PARAMS_OMITTED_METADATA_KEY, RESPONSE_FORMAT_ADJUSTED_METADATA_KEY};
    use mockito::Server;
    use paladin_core::platform::container::prompt::UserPrompt;
    use paladin_ports::output::llm_port::ResponseFormat;
    use std::collections::HashMap;

    fn test_config() -> CompatEngineConfig {
        CompatEngineConfig {
            base_url: "https://example.invalid/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "test-model".to_string(),
            timeout_seconds: 60,
            max_retries: 3,
            capabilities: CompatCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                max_context_tokens: Some(1000),
                supports_system_messages: true,
                temperature_range: Some((0.0, 1.0)),
            },
            // Unchanged pre-existing behaviour: no vendor-specific
            // restriction has been measured for this synthetic test
            // preset, so it declares full support (17-18).
            request_parameters: CompatRequestParameters::all(ReasoningWire::Unsupported),
            fallback_models: vec!["fallback-model".to_string()],
            error_override: None,
            redirect_policy: None,
        }
    }

    #[tokio::test]
    async fn compat_engine_honours_configured_max_retries() {
        // `CompatEngineConfig.max_retries` is the number of retries after the first attempt:
        // 0 sends once, 2 sends three times (a retryable 5xx each time).
        for retries in [0_u32, 2] {
            let server = crate::compat::test_support::server().await;
            let mut config = test_config();
            config.base_url = server.url();
            config.max_retries = retries;
            let engine = CompatEngine::new(config).unwrap();
            crate::compat::test_support::attempts_against_a_failing_endpoint(
                server,
                &engine_port::EnginePort(engine),
                "test-model",
                retries as usize + 1,
            )
            .await;
        }
    }

    /// An `LlmPort` over a bare engine, so the shared retry helper can drive it.
    mod engine_port {
        use super::super::CompatEngine;
        use async_trait::async_trait;
        use futures::Stream;
        use paladin_ports::output::llm_port::{
            LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
        };

        pub(super) struct EnginePort(pub(super) CompatEngine);

        #[async_trait]
        impl LlmPort for EnginePort {
            async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
                self.0.generate(request).await
            }
            async fn generate_stream(
                &self,
                request: LlmRequest,
            ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
            {
                self.0.generate_stream(request).await
            }
            async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
                Ok(self.0.validate_model(model).await)
            }
            async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
                Ok(self.0.available_models().await)
            }
            fn get_provider_name(&self) -> &'static str {
                self.0.provider_name()
            }
            fn get_capabilities(&self) -> ProviderCapabilities {
                self.0.capabilities()
            }
        }
    }

    #[test]
    fn engine_constructs_from_valid_config() {
        let engine = CompatEngine::new(test_config());
        assert!(engine.is_ok());
    }

    #[test]
    fn map_finish_reason_stop_length_content_filter_and_function_call() {
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("stop".to_string())),
            FinishReason::Stop
        ));
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("length".to_string())),
            FinishReason::Length
        ));
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("content_filter".to_string())),
            FinishReason::ContentFilter
        ));
        assert!(matches!(
            CompatEngine::map_finish_reason(Some("function_call".to_string())),
            FinishReason::FunctionCall
        ));
    }

    #[test]
    fn map_finish_reason_unknown_string_maps_to_error_never_silently_to_stop() {
        let result = CompatEngine::map_finish_reason(Some("bogus".to_string()));
        assert!(matches!(result, FinishReason::Error(_)));
    }

    #[test]
    fn map_finish_reason_absent_maps_to_stop() {
        assert!(matches!(
            CompatEngine::map_finish_reason(None),
            FinishReason::Stop
        ));
    }

    #[test]
    fn map_error_status_codes() {
        let engine = CompatEngine::new(test_config()).unwrap();
        assert!(matches!(
            engine.map_error(401, "bad key"),
            LlmError::AuthenticationError(_)
        ));
        assert!(matches!(
            engine.map_error(429, "slow down"),
            LlmError::RateLimitExceeded { .. }
        ));
        assert!(matches!(
            engine.map_error(404, "no model"),
            LlmError::ModelNotAvailable(_)
        ));
        assert!(matches!(
            engine.map_error(400, "bad prompt"),
            LlmError::InvalidPrompt(_)
        ));
        // Phase 25 (FT-FR-01, D-03): a status with no dedicated variant
        // carries its code as a typed field, never as text to parse.
        match engine.map_error(500, "server error") {
            LlmError::ProviderError {
                provider, status, ..
            } => {
                assert_eq!(provider, DEFAULT_PROVIDER_NAME);
                assert_eq!(status, 500);
            }
            other => panic!("expected ProviderError {{ status: 500 }}, got {other:?}"),
        }
    }

    #[test]
    fn map_error_maps_a_redirect_status_to_an_actionable_provider_error() {
        let engine = CompatEngine::new(test_config()).unwrap();

        for expected in [302u16, 307u16] {
            match engine.map_error(expected, "moved") {
                LlmError::ProviderError {
                    status, message, ..
                } => {
                    assert_eq!(status, expected, "typed status field must carry the code");
                    assert!(
                        message.contains("redirect"),
                        "status {expected}: message must name the refused redirect, got: {message}"
                    );
                }
                other => panic!("status {expected}: expected ProviderError, got {other:?}"),
            }
        }
    }

    #[test]
    fn map_error_stamps_the_preset_provider_name_on_provider_error() {
        let engine = CompatEngine::new(test_config())
            .unwrap()
            .with_provider_name("test-preset");
        assert_eq!(engine.provider_name(), "test-preset");
        match engine.map_error(503, "overloaded") {
            LlmError::ProviderError { provider, .. } => assert_eq!(provider, "test-preset"),
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[test]
    fn map_error_redacts_the_raw_body_before_bounding_it() {
        // Callers now hand `map_error` the RAW body; the helper must still
        // scrub the configured key (test_config's key is "test-key").
        let engine = CompatEngine::new(test_config()).unwrap();
        let body = r#"{"error":"echo","authorization":"Bearer test-key"}"#;
        match engine.map_error(500, body) {
            LlmError::ProviderError { message, .. } => {
                assert!(!message.contains("test-key"), "key leaked: {message}");
            }
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[test]
    fn map_error_consults_override_before_default_mapping() {
        fn override_fn(status: u16, _message: &str) -> Option<LlmError> {
            if status == 402 {
                Some(LlmError::UsageLimitExceeded {
                    provider: "test".to_string(),
                    regain_hint: None,
                })
            } else {
                None
            }
        }

        let mut config = test_config();
        config.error_override = Some(override_fn);
        let engine = CompatEngine::new(config).unwrap();

        assert!(matches!(
            engine.map_error(402, "insufficient balance"),
            LlmError::UsageLimitExceeded { .. }
        ));
        // Falls through to default mapping when the override returns None.
        assert!(matches!(
            engine.map_error(401, "bad key"),
            LlmError::AuthenticationError(_)
        ));
    }

    #[test]
    fn capabilities_reports_exactly_what_was_configured() {
        let engine = CompatEngine::new(test_config()).unwrap();
        let caps = engine.capabilities();
        assert!(caps.supports_streaming);
        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_function_calling);
        assert_eq!(caps.max_context_tokens, Some(1000));
        assert_eq!(caps.temperature_range, Some((0.0, 1.0)));
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_invokes_operation_exactly_once_on_empty_completion() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let engine = CompatEngine::new(test_config()).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = engine
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::empty_completion("no text".to_string()))
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_retries_network_error_up_to_max_retries_plus_one() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let engine = CompatEngine::new(test_config()).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = engine
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::NetworkError("connection reset".to_string()))
                    }
                },
                3,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    // ── WR-04 (new): a transient stream-open failure retries like generate() does ──
    //
    // Resolved facts (recorded in 17-16-SUMMARY.md per D-00e), read before
    // writing these tests:
    //   1. `test_config()`'s `max_retries` (this file, above) is `3`,
    //      matching every shipped preset (e.g. `kimi/adapter.rs:157`), and
    //      `call_api_with_retry`'s `for attempt in 0..=max_retries` loop
    //      (compat/engine.rs:357) means `max_retries + 1` = 4 total
    //      attempts on a retryable error — the `+1` semantics this file's
    //      own `call_api_with_retry_retries_network_error_up_to_max_retries_plus_one`
    //      test already pins.
    //   2. HTTP 500 is retryable: it falls through `map_error`'s only
    //      engine-local arm (300..=399) into the crate-wide
    //      `map_http_status`, which returns `LlmError::ProviderError
    //      { status: 500 }` — outside `call_api_with_retry`'s
    //      non-retryable set, so it is retried (Phase 25 D-03; this
    //      comment previously named the pre-25-05 `ProcessingError`
    //      catch-all).
    //   3. These tests exercise `CompatEngine` directly (not through the
    //      `KimiAdapter` preset) — Kimi already has `generate_stream` mock
    //      -transport scaffolding (`kimi/adapter.rs`
    //      `generate_stream_assembles_deltas_in_wire_order_with_terminal_stop`),
    //      whose shape (endpoint, SSE body, `[DONE]` sentinel) is reused
    //      here directly against the shared engine, per the plan's "reuse
    //      that scaffolding" instruction. `kimi/adapter.rs` itself is
    //      untouched — this plan's `files_modified` names only this file
    //      and `gemini/adapter.rs`.
    //
    // The "transient" test below derives the expected attempt count by
    // running the same mock failure through `generate()` first, rather than
    // hardcoding `4` — the assertion is "streaming retries like
    // non-streaming", not "streaming retries four times", and does not
    // silently rot if `call_api_with_retry`'s cap ever changes.

    fn test_config_at(base_url: &str) -> CompatEngineConfig {
        let mut config = test_config();
        config.base_url = base_url.to_string();
        config
    }

    fn build_request(model: &str) -> LlmRequest {
        LlmRequest::new(
            model,
            PromptItem::new(PromptType::User(UserPrompt {
                query: "Hello".to_string(),
                context: None,
            }))
            .unwrap(),
        )
    }

    // Not `start_paused = true`: this test exercises real network round
    // -trips against a `mockito` server, and pausing the tokio clock races
    // the retry backoff timer against `reqwest`'s own request timeout —
    // confirmed directly against the sibling Gemini test in
    // `gemini/adapter.rs`, whose third attempt spuriously timed out under
    // a paused clock instead of exercising all `max_retries` attempts. Real
    // (unpaused) time makes the ~700ms of total backoff actually elapse,
    // which is what the synthetic-closure retry tests above use
    // `start_paused = true` to avoid paying.
    #[tokio::test]
    async fn generate_stream_retries_a_transient_open_failure_as_many_times_as_generate() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let mut generate_server = Server::new_async().await;
        let generate_calls = Arc::new(AtomicU32::new(0));
        let generate_calls_clone = Arc::clone(&generate_calls);
        generate_server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body_from_request(move |_req| {
                generate_calls_clone.fetch_add(1, Ordering::SeqCst);
                br#"{"error":"transient failure"}"#.to_vec()
            })
            .create_async()
            .await;

        let generate_engine = CompatEngine::new(test_config_at(&generate_server.url())).unwrap();
        let generate_result = generate_engine.generate(build_request("test-model")).await;
        assert!(generate_result.is_err());
        let generate_attempt_count = generate_calls.load(Ordering::SeqCst);

        let mut stream_server = Server::new_async().await;
        let stream_calls = Arc::new(AtomicU32::new(0));
        let stream_calls_clone = Arc::clone(&stream_calls);
        let stream_mock = stream_server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body_from_request(move |_req| {
                stream_calls_clone.fetch_add(1, Ordering::SeqCst);
                br#"{"error":"transient failure"}"#.to_vec()
            })
            // Load-bearing (D-00e): today `generate_stream` makes exactly
            // ONE request on a transient failure, not
            // `generate_attempt_count` — this `.expect()` is what fails in
            // the RED state, not the `Result` assertion below.
            .expect(generate_attempt_count as usize)
            .create_async()
            .await;

        let stream_engine = CompatEngine::new(test_config_at(&stream_server.url())).unwrap();
        let stream_result = stream_engine
            .generate_stream(build_request("test-model"))
            .await;

        stream_mock.assert_async().await;
        assert_eq!(
            stream_calls.load(Ordering::SeqCst),
            generate_attempt_count,
            "generate_stream must retry the connection-opening POST exactly \
             as many times as generate() does"
        );
        assert!(stream_result.is_err());
    }

    // Not `start_paused = true` — see the sibling transient test's comment
    // above: a paused clock is unreliable against a real `mockito` network
    // round trip in this suite.
    #[tokio::test]
    async fn generate_stream_does_not_retry_an_authentication_failure_on_open() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body(r#"{"error":"invalid key"}"#)
            // Load-bearing (D-00e): passes today (there is no retry at all)
            // and must keep passing after the fix — this is the
            // credential-replay guard, proving an authentication failure on
            // stream open is attempted exactly once.
            .expect(1)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let result = engine.generate_stream(build_request("test-model")).await;

        mock.assert_async().await;
        // `result`'s `Ok` payload is a boxed `dyn Stream` with no `Debug`
        // impl, so this is matched by hand rather than via a formatted
        // `matches!` assertion.
        match &result {
            Err(LlmError::AuthenticationError(_)) => {}
            Ok(_) => panic!("expected Err(AuthenticationError), got Ok(<stream>)"),
            Err(other) => panic!("expected AuthenticationError, got: {other:?}"),
        }
    }

    // ── PACE-01 / D-02: a 429 is surfaced on the FIRST attempt, with its delay ──
    //
    // The decorator above this engine (the Cadence) owns waiting. A private
    // retry loop that re-sent a 429 would hide thrash below it, so a 429 is
    // never retried here; network, timeout and 5xx retries are unchanged
    // (`call_api_with_retry_retries_network_error_up_to_max_retries_plus_one`).

    #[tokio::test(start_paused = true)]
    async fn call_api_with_retry_surfaces_rate_limit_exceeded_on_the_first_attempt() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let engine = CompatEngine::new(test_config()).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = Arc::clone(&calls);

        let result: Result<(), LlmError> = engine
            .call_api_with_retry(
                move || {
                    let calls = Arc::clone(&calls_clone);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(LlmError::rate_limited(None))
                    }
                },
                3,
            )
            .await;

        assert!(matches!(result, Err(LlmError::RateLimitExceeded { .. })));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn compat_429_is_surfaced_on_the_first_attempt_with_its_retry_delay() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_header("retry-after", "7")
            .with_body(r#"{"error":{"message":"slow down"}}"#)
            .expect(1)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let err = engine
            .generate(build_request("test-model"))
            .await
            .expect_err("a 429 must surface as an error");

        mock.assert_async().await;
        assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
    }

    #[tokio::test]
    async fn compat_stream_open_429_is_surfaced_once_with_its_retry_delay() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_header("retry-after", "7")
            .with_body(r#"{"error":{"message":"slow down"}}"#)
            .expect(1)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let result = engine.generate_stream(build_request("test-model")).await;

        mock.assert_async().await;
        match &result {
            Err(err @ LlmError::RateLimitExceeded { .. }) => {
                assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
            }
            Ok(_) => panic!("expected a rate-limit error, got Ok(<stream>)"),
            Err(other) => panic!("expected a rate-limit error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn compat_429_without_retry_after_carries_no_delay() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_body("{}")
            .expect(1)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let err = engine
            .generate(build_request("test-model"))
            .await
            .expect_err("a 429 must surface as an error");

        mock.assert_async().await;
        assert!(matches!(err, LlmError::RateLimitExceeded { .. }));
        assert_eq!(err.retry_after(), None);
    }

    #[tokio::test]
    async fn generate_stream_opens_exactly_once_and_yields_its_deltas_in_order_on_success() {
        let sse_body = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"lo \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body)
            // Load-bearing (D-00e): proves the retried-open shape does not
            // double-open a working connection.
            .expect(1)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let stream = engine
            .generate_stream(build_request("test-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut deltas = Vec::new();
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            if !chunk.delta.is_empty() {
                deltas.push(chunk.delta);
            }
        }

        mock.assert_async().await;
        // Exactly the three wire deltas, in order, none duplicated — a
        // future change that retried the byte stream would double this.
        assert_eq!(
            deltas,
            vec!["Hel".to_string(), "lo ".to_string(), "world".to_string()]
        );
    }

    // ── CompatRequestParameters (17-18, closing G-17-4a) ──
    //
    // A preset declares which of the five optional sampling parameters its
    // own request path carries; the engine reads only that declaration —
    // never a provider name, base URL, or other vendor identity — to decide
    // what reaches the wire. These five tests exercise the mechanism
    // directly against a synthetic preset carrying no vendor identity at
    // all, per the plan's own edge case: "a synthetic preset carrying no
    // vendor identity at all, declaring one parameter unsupported, produces
    // a body omitting exactly that parameter."
    //
    // All five values below (0.5, 0.25, 0.75, 0.125) are exact sums of
    // negative powers of two, so the `f32` caller value and the `f64`
    // parsed from the captured wire JSON compare exactly — no floating
    // -point tolerance is needed or used.

    use paladin_core::platform::container::prompt::PromptParameters;
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    fn test_config_with_request_parameters(
        base_url: &str,
        request_parameters: CompatRequestParameters,
    ) -> CompatEngineConfig {
        let mut config = test_config_at(base_url);
        config.request_parameters = request_parameters;
        config
    }

    /// Every one of the five optional sampling parameters set to a distinct,
    /// exactly-representable value, so a captured wire body can be checked
    /// against the caller's actual value, not merely "is present".
    fn fully_specified_parameters() -> PromptParameters {
        PromptParameters {
            max_tokens: Some(123),
            temperature: Some(0.5),
            top_p: Some(0.25),
            frequency_penalty: Some(0.75),
            presence_penalty: Some(0.125),
            stop_sequences: None,
        }
    }

    fn build_request_with_parameters(model: &str, parameters: PromptParameters) -> LlmRequest {
        let mut request = build_request(model);
        request.prompt.entity.data.parameters = parameters;
        request
    }

    /// Sets up a mock `/chat/completions` endpoint that captures the raw
    /// outgoing request body (as both parsed JSON and the original text) and
    /// answers with a minimal well-formed completion, and runs `generate()`
    /// against it. Returns the captured body — this is the shared plumbing
    /// under all five tests below; each test differs only in the engine
    /// config and request it builds.
    async fn generate_and_capture_body(
        config: CompatEngineConfig,
        request: LlmRequest,
    ) -> (Value, String) {
        let (body, text, _metadata) = generate_and_capture_all(config, request).await;
        (body, text)
    }

    /// [`generate_and_capture_body`] plus the response's metadata map, for the tests that pin
    /// what an omission recorded (ADR-0061).
    async fn generate_and_capture_all(
        config: CompatEngineConfig,
        request: LlmRequest,
    ) -> (Value, String, HashMap<String, String>) {
        // `server` must outlive the call to `generate()` below (it is
        // dropped at the end of this function's scope, after `.await`
        // returns), so it cannot be constructed inside a nested block.
        let mut server = Server::new_async().await;
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured);

        // `config`'s `base_url` is overwritten to point at this mock server
        // — every caller below builds its config via `test_config_at`-style
        // helpers whose `base_url` is a placeholder replaced here, so the
        // mock is guaranteed to be the one actually hit.
        let mut config = config;
        config.base_url = server.url();

        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body_from_request(move |req| {
                let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                *captured_clone.lock().unwrap() = Some(body_text);
                json!({
                    "id": "cmpl-1",
                    "model": "test-model",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "ok"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string()
                .into_bytes()
            })
            .create_async()
            .await;

        let engine = CompatEngine::new(config).unwrap();
        let result = engine.generate(request).await;
        assert!(
            result.is_ok(),
            "mock server returned a well-formed response: {result:?}"
        );
        let metadata = result.map(|response| response.metadata).unwrap_or_default();

        let body_text = captured
            .lock()
            .unwrap()
            .take()
            .expect("mock must have been called exactly once");
        let body_json: Value =
            serde_json::from_str(&body_text).expect("captured body must be valid JSON");
        (body_json, body_text, metadata)
    }

    /// Phase 50 (PARITY-08): one test for the engine covers Kimi, Qwen, Grok, Ollama and the
    /// OpenAI-compatible provider, which all build their request here. A present context is
    /// appended to the query by the shared `user_turn_text` helper; an absent one leaves the
    /// query unchanged.
    #[tokio::test]
    async fn compat_engine_user_turn_appends_context() {
        async fn user_content(context: Option<&str>) -> Value {
            let request = LlmRequest::new(
                "test-model",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "What is 2+2?".to_string(),
                    context: context.map(str::to_string),
                }))
                .unwrap(),
            );
            let (body, _) =
                generate_and_capture_body(test_config_at("https://example.invalid/v1"), request)
                    .await;
            body["messages"][0]["content"].clone()
        }

        assert_eq!(user_content(None).await, "What is 2+2?");
        assert_eq!(
            user_content(Some("Extra context.")).await,
            "What is 2+2?\n\nExtra context."
        );
    }

    // Test 1 — mechanism is data-driven: a preset declaring
    // `presence_penalty: false` and everything else `true` omits exactly
    // that key while carrying the caller's values for the rest.
    #[tokio::test]
    async fn build_request_omits_exactly_the_declared_unsupported_parameter() {
        let mut request_parameters = CompatRequestParameters::all(ReasoningWire::Unsupported);
        request_parameters.presence_penalty = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (body, _, metadata) = generate_and_capture_all(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("presence_penalty"),
            "declared-unsupported parameter must be absent from the wire body, got: {obj:?}"
        );
        assert_eq!(
            metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("presence_penalty"),
            "the omission must be recorded, not silent (ADR-0061)"
        );
        assert_eq!(obj.get("temperature").and_then(Value::as_f64), Some(0.5));
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
    }

    // Test 2 — behaviour preservation: a preset declaring every parameter
    // carried produces a body byte-for-byte identical, in substance, to the
    // pre-17-18 engine — all five keys present with exactly the caller's
    // values.
    #[tokio::test]
    async fn build_request_with_all_declared_carries_every_caller_value_unchanged() {
        let config = test_config_with_request_parameters(
            "https://example.invalid/v1",
            CompatRequestParameters::all(ReasoningWire::Unsupported),
        );
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (body, _) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert_eq!(obj.get("temperature").and_then(Value::as_f64), Some(0.5));
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
        assert_eq!(
            obj.get("presence_penalty").and_then(Value::as_f64),
            Some(0.125)
        );
    }

    // Test 3 — omission is absence, not `null`: the raw wire text contains
    // no occurrence of the dropped key's name at all, so a future change
    // that serialised `"presence_penalty":null` instead of dropping the key
    // would fail this test even though Test 1's `contains_key` check on the
    // parsed object could not tell the difference between "absent" and
    // "present with a null value that happened to parse oddly".
    #[tokio::test]
    async fn build_request_omission_is_absence_never_a_null_value() {
        let mut request_parameters = CompatRequestParameters::all(ReasoningWire::Unsupported);
        request_parameters.presence_penalty = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (_, body_text) = generate_and_capture_body(config, request).await;

        assert!(
            !body_text.contains("presence_penalty"),
            "the key name itself must not appear on the wire — got body: {body_text}"
        );
    }

    // Test 4 — a caller-supplied value cannot resurrect a declared
    // -unsupported parameter: the declaration is about the vendor's
    // protocol, not about whether the caller happened to supply a value.
    #[tokio::test]
    async fn build_request_declared_unsupported_wins_even_when_caller_sets_a_value() {
        let mut request_parameters = CompatRequestParameters::all(ReasoningWire::Unsupported);
        request_parameters.presence_penalty = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let mut parameters = fully_specified_parameters();
        parameters.presence_penalty = Some(1.5);
        let request = build_request_with_parameters("test-model", parameters);

        let (body, body_text, metadata) = generate_and_capture_all(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("presence_penalty"),
            "an explicit caller value must not resurrect a declared-unsupported \
             parameter, got: {obj:?}"
        );
        assert!(!body_text.contains("presence_penalty"));
        assert_eq!(
            metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("presence_penalty")
        );
    }

    // Test 5 — unset stays unset: a preset declaring a parameter carried,
    // whose caller left it `None`, still omits the key. This is the
    // pre-existing `skip_serializing_if` behaviour, and this test proves the
    // new gating logic does not disturb it.
    #[tokio::test]
    async fn build_request_carried_but_caller_unset_parameter_stays_omitted() {
        let config = test_config_with_request_parameters(
            "https://example.invalid/v1",
            CompatRequestParameters::all(ReasoningWire::Unsupported),
        );
        let mut parameters = fully_specified_parameters();
        parameters.presence_penalty = None;
        let request = build_request_with_parameters("test-model", parameters);

        let (body, _) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("presence_penalty"),
            "a parameter the caller left unset must stay omitted, got: {obj:?}"
        );
        // The other four, which the caller DID set, must still be present —
        // proving this is "unset stays unset", not an accidental drop of
        // everything.
        assert_eq!(obj.get("temperature").and_then(Value::as_f64), Some(0.5));
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
    }

    // Test 6 (17-19, closing G-17-4b) — option (a) pinned specifically for
    // `temperature`: a preset declaring `temperature: false` produces a body
    // with no `temperature` key at all, while the other four sampling
    // parameters still carry the caller's values unchanged. Tests 1-5 above
    // exercise the mechanism generically through `presence_penalty`; this
    // test exercises the exact field plan 17-19's Kimi preset declares
    // absent, so a future change that special-cased `presence_penalty`
    // without generalizing to `temperature` would be caught here.
    #[tokio::test]
    async fn build_request_omits_exactly_a_declared_absent_temperature() {
        let mut request_parameters = CompatRequestParameters::all(ReasoningWire::Unsupported);
        request_parameters.temperature = false;

        let config =
            test_config_with_request_parameters("https://example.invalid/v1", request_parameters);
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (body, body_text, metadata) = generate_and_capture_all(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("temperature"),
            "declared-absent temperature must not appear on the wire, got: {obj:?}"
        );
        assert_eq!(
            metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("temperature"),
            "the omission must be recorded, not silent (ADR-0061)"
        );
        assert!(
            !body_text.contains("temperature"),
            "the key name itself must not appear on the wire — got body: {body_text}"
        );
        // The other four parameters, all declared carried, must still be
        // present with exactly the caller's values — proving this is a
        // targeted omission of one field, not an accidental drop of all.
        assert_eq!(obj.get("max_tokens").and_then(Value::as_u64), Some(123));
        assert_eq!(obj.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            obj.get("frequency_penalty").and_then(Value::as_f64),
            Some(0.75)
        );
        assert_eq!(
            obj.get("presence_penalty").and_then(Value::as_f64),
            Some(0.125)
        );
    }

    // ── classify_fetch_failure (17-22, closing G-17-4d's diagnosability half) ──
    //
    // One test per `LlmError` variant, matching the plan's own requirement
    // ("Each arm is asserted by its own test") — this is what makes the
    // match's exhaustiveness meaningful: a variant added later without a
    // corresponding test here is still caught by the compiler (no wildcard
    // arm in `classify_fetch_failure` itself), but a variant classified on
    // the WRONG side of the split would only be caught by these.

    #[test]
    fn classify_fetch_failure_authentication_error_is_misconfiguration() {
        assert_eq!(
            classify_fetch_failure(&LlmError::AuthenticationError("bad key".to_string())),
            FetchFailureClass::Misconfiguration
        );
    }

    #[test]
    fn classify_fetch_failure_network_error_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::NetworkError("connection refused".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_timeout_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::Timeout("timed out".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_rate_limit_exceeded_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::rate_limited(None)),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_usage_limit_exceeded_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::UsageLimitExceeded {
                provider: "test".to_string(),
                regain_hint: None,
            }),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_model_not_available_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::ModelNotAvailable("no such model".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_token_limit_exceeded_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::TokenLimitExceeded),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_empty_completion_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::empty_completion("no text".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_refusal_is_misconfiguration() {
        assert_eq!(
            classify_fetch_failure(&LlmError::refusal("kimi", "m", "no")),
            FetchFailureClass::Misconfiguration
        );
    }

    #[test]
    fn classify_fetch_failure_invalid_prompt_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::InvalidPrompt("empty prompt".to_string())),
            FetchFailureClass::Supported
        );
    }

    #[test]
    fn classify_fetch_failure_processing_error_is_supported() {
        assert_eq!(
            classify_fetch_failure(&LlmError::ProcessingError("server error".to_string())),
            FetchFailureClass::Supported
        );
    }

    // ── base_url_without_userinfo (T-17-91) ──

    #[test]
    fn base_url_without_userinfo_strips_a_credential_from_the_authority() {
        assert_eq!(
            base_url_without_userinfo("https://user:secret@example.invalid/v1"),
            "https://example.invalid/v1"
        );
    }

    #[test]
    fn base_url_without_userinfo_strips_a_password_containing_an_unescaped_at() {
        // WR-01: userinfo ends at the LAST '@' per RFC 3986. Stripping to the
        // first one left `ssw0rd@example.invalid` in the warn line.
        assert_eq!(
            base_url_without_userinfo("https://alice:p@ssw0rd@example.invalid/v1"),
            "https://example.invalid/v1"
        );
        assert_eq!(
            base_url_without_userinfo("https://a@b@c@example.invalid:8443/v1"),
            "https://example.invalid:8443/v1"
        );
    }

    #[test]
    fn base_url_without_userinfo_leaves_a_plain_url_unchanged() {
        let url = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1";
        assert_eq!(base_url_without_userinfo(url), url);
    }

    #[test]
    fn base_url_without_userinfo_preserves_port_and_path_after_stripping() {
        assert_eq!(
            base_url_without_userinfo("https://user:secret@example.invalid:8443/v1/models"),
            "https://example.invalid:8443/v1/models"
        );
    }

    #[test]
    fn base_url_without_userinfo_handles_a_string_with_no_scheme_separator() {
        let malformed = "not-a-url";
        assert_eq!(base_url_without_userinfo(malformed), malformed);
    }

    // ── available_models' D-13/D-14 fallback contract, unchanged by this
    //    plan (17-22) ──
    //
    // These pin the exact must_have the plan prohibits touching: the method
    // still returns `Vec<String>`, never errors, and returns the curated
    // list on both a misconfiguration (401) and a transport failure —
    // proven against the RETURNED VALUE, not merely against the log, since
    // no log-capture crate may be added (T-17-SC-22).

    #[tokio::test]
    async fn available_models_returns_curated_fallback_on_authentication_failure() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/models")
            .with_status(401)
            .with_body(r#"{"error":"invalid_api_key"}"#)
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let models = engine.available_models().await;

        assert_eq!(models, vec!["fallback-model".to_string()]);
    }

    #[tokio::test]
    async fn available_models_returns_curated_fallback_on_connection_failure() {
        // A bound-then-immediately-dropped TCP listener yields a port no
        // process is listening on, so connecting to it fails fast with a
        // real `NetworkError` — no mock server involved, because this test
        // exercises the transport-failure path `fetch_live_models` produces
        // before any HTTP status exists to mock.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let engine = CompatEngine::new(test_config_at(&format!("http://{addr}"))).unwrap();
        let models = engine.available_models().await;

        assert_eq!(models, vec!["fallback-model".to_string()]);
    }

    // ── Phase 26 (RT-05, D-28): response_format reaches the wire ──────────
    //
    // These three tests cover all five presets that share this engine
    // (Kimi, Qwen, Grok, Ollama, the generic OpenAI-compatible preset) with
    // one set of cases, per the plan's Test 4/Test 5 split.

    #[tokio::test]
    async fn compat_engine_request_carries_response_format() {
        let config = test_config_at("https://example.invalid/v1");
        let request = build_request("test-model").with_response_format(ResponseFormat::JsonObject);

        let (body, _) = generate_and_capture_body(config, request).await;

        assert_eq!(
            body.get("response_format"),
            Some(&json!({"type": "json_object"}))
        );
    }

    #[tokio::test]
    async fn compat_engine_json_schema_degrades_to_json_object() {
        let config = test_config_at("https://example.invalid/v1");
        let request =
            build_request("test-model").with_response_format(ResponseFormat::JsonSchema {
                name: "answer".to_string(),
                schema: json!({"type": "object"}),
                strict: true,
            });

        let (body, _) = generate_and_capture_body(config, request).await;

        assert_eq!(
            body.get("response_format"),
            Some(&json!({"type": "json_object"})),
            "the compat engine degrades every ResponseFormat variant to the \
             plain JSON-object form (D-28) -- never to an omitted field"
        );
    }

    #[tokio::test]
    async fn compat_engine_request_without_response_format_is_byte_identical_to_today() {
        let config = test_config_at("https://example.invalid/v1");
        let request = build_request("test-model");

        let (body, _) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("response_format"),
            "absent response_format must not appear on the wire, got: {obj:?}"
        );
    }

    // ── Phase 50 (ADR-0061): the per-model profile decides what reaches the wire ────────────
    //
    // One set of cases for all five presets that share this engine. The registry is attached
    // the way a preset attaches it, over a fallback profile derived from the config.

    /// A registry over the test config's own declaration, with stop sequences supported.
    fn test_registry(overrides: Vec<(&str, crate::registry::ModelOverride)>) -> ModelRegistry {
        let config = test_config();
        let fallback = fallback_from_compat(
            &config.capabilities.clone().into(),
            &config.request_parameters,
            StopSupport::Supported { max: None },
        );
        ModelRegistry::new("test-preset", &[], fallback)
            .with_overrides(
                overrides
                    .into_iter()
                    .map(|(model, over)| (model.to_string(), over))
                    .collect(),
            )
            .expect("the test overrides are valid")
    }

    /// Run `generate` through a mock endpoint, returning the outcome, the captured body (if any
    /// request arrived) and how many requests arrived.
    async fn generate_through(
        engine_for: impl FnOnce(String) -> CompatEngine,
        request: LlmRequest,
    ) -> (Result<LlmResponse, LlmError>, Option<Value>, usize) {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let mut server = Server::new_async().await;
        let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
        let hits = Arc::new(AtomicUsize::new(0));
        let (captured_clone, hits_clone) = (Arc::clone(&captured), Arc::clone(&hits));
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body_from_request(move |req| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
                let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                *captured_clone.lock().unwrap() = serde_json::from_str(&text).ok();
                json!({
                    "id": "cmpl-1",
                    "model": "test-model",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "ok"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string()
                .into_bytes()
            })
            .expect_at_least(0)
            .create_async()
            .await;

        let engine = engine_for(server.url());
        let outcome = engine.generate(request).await;
        let body = captured.lock().unwrap().take();
        (outcome, body, hits.load(Ordering::SeqCst))
    }

    fn engine_with(registry: ModelRegistry) -> impl FnOnce(String) -> CompatEngine {
        move |url| {
            CompatEngine::new(test_config_at(&url))
                .unwrap()
                .with_provider_name("test-preset")
                .with_model_registry(registry)
        }
    }

    /// Drain a compat stream against a mock SSE endpoint and return its chunks.
    async fn stream_through(
        engine_for: impl FnOnce(String) -> CompatEngine,
        request: LlmRequest,
    ) -> Vec<StreamingResponse> {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body_with_usage_frame())
            .create_async()
            .await;
        let engine = engine_for(server.url());
        let stream = engine.generate_stream(request).await.unwrap();
        let mut stream = Box::into_pin(stream);
        let mut chunks = Vec::new();
        while let Some(item) = stream.next().await {
            chunks.push(item.expect("every item of the scripted stream is Ok"));
        }
        chunks
    }

    /// Phase 50, owner reading R-1 (option-b): a streamed call stamps the visibility keys the
    /// buffered path would, on the terminal chunk only.
    #[tokio::test]
    async fn compat_stream_terminal_chunk_carries_metadata() {
        let mut parameters = fully_specified_parameters();
        parameters.temperature = Some(1.5); // above the declared (0.0, 1.0)
        let request = build_request_with_parameters("some-new-model", parameters);

        let chunks = stream_through(engine_with(test_registry(vec![])), request).await;

        let terminal = chunks.last().expect("a terminal chunk");
        assert!(terminal.finish_reason.is_some());
        assert_eq!(
            terminal
                .metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("temperature")
        );
        assert_eq!(
            terminal
                .metadata
                .get(UNPROFILED_MODEL_METADATA_KEY)
                .map(String::as_str),
            Some("some-new-model")
        );
        for content in &chunks[..chunks.len() - 1] {
            assert!(content.metadata.is_empty(), "content chunk: {content:?}");
        }
    }

    /// A streamed call that adjusted nothing ends with an empty metadata map -- the terminal
    /// chunk is byte-identical to before the field existed.
    #[tokio::test]
    async fn compat_stream_terminal_chunk_has_no_metadata_when_nothing_was_adjusted() {
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let chunks = stream_through(
            |url| CompatEngine::new(test_config_at(&url)).unwrap(),
            request,
        )
        .await;

        assert!(chunks.iter().all(|c| c.metadata.is_empty()), "{chunks:?}");
    }

    #[tokio::test]
    async fn compat_omission_is_recorded_in_metadata() {
        let mut parameters = fully_specified_parameters();
        parameters.temperature = Some(1.5); // above the declared (0.0, 1.0)
        parameters.top_p = Some(0.25); // admitted
        let request = build_request_with_parameters("test-model", parameters);

        let (outcome, body, _) =
            generate_through(engine_with(test_registry(vec![])), request).await;
        let response = outcome.expect("the request is well-formed");
        let body = body.expect("a request was sent");

        assert!(
            body.get("temperature").is_none(),
            "an inadmissible temperature is omitted -- absent, never null or swapped: {body}"
        );
        assert_eq!(body.get("top_p").and_then(Value::as_f64), Some(0.25));
        assert_eq!(
            response
                .metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("temperature")
        );
    }

    #[tokio::test]
    async fn compat_nothing_omitted_leaves_the_metadata_key_absent() {
        let request = build_request_with_parameters("test-model", fully_specified_parameters());

        let (outcome, _, _) = generate_through(engine_with(test_registry(vec![])), request).await;
        let response = outcome.expect("the request is well-formed");

        assert!(
            !response.metadata.contains_key(PARAMS_OMITTED_METADATA_KEY),
            "nothing was omitted, so the key must be absent: {:?}",
            response.metadata
        );
    }

    #[tokio::test]
    async fn compat_stop_reaches_the_wire_in_order() {
        let mut parameters = fully_specified_parameters();
        parameters.stop_sequences = Some(vec!["END".into(), "STOP".into(), "END".into()]);
        let request = build_request_with_parameters("test-model", parameters);

        let (outcome, body, _) =
            generate_through(engine_with(test_registry(vec![])), request).await;
        let response = outcome.expect("the request is well-formed");

        assert_eq!(
            body.expect("a request was sent").get("stop"),
            Some(&json!(["END", "STOP", "END"])),
            "stop sequences keep the caller's order and duplicates"
        );
        assert!(!response.metadata.contains_key(PARAMS_OMITTED_METADATA_KEY));
    }

    #[tokio::test]
    async fn compat_stop_is_omitted_and_recorded_when_the_profile_does_not_take_it() {
        // No registry attached: the profile derives from the config with stop NotSent.
        let mut parameters = fully_specified_parameters();
        parameters.stop_sequences = Some(vec!["END".into()]);
        let request = build_request_with_parameters("test-model", parameters);

        let (outcome, body, _) = generate_through(
            |url| CompatEngine::new(test_config_at(&url)).unwrap(),
            request,
        )
        .await;
        let response = outcome.expect("the request is well-formed");

        let body = body.expect("a request was sent");
        assert!(body.get("stop").is_none(), "no stop key at all: {body}");
        assert_eq!(
            response
                .metadata
                .get(PARAMS_OMITTED_METADATA_KEY)
                .map(String::as_str),
            Some("stop")
        );
    }

    #[tokio::test]
    async fn compat_unprofiled_model_is_recorded() {
        let request = build_request("some-new-model");

        let (outcome, _, _) = generate_through(engine_with(test_registry(vec![])), request).await;
        let response = outcome.expect("an unknown model is never a refusal (D-07)");

        assert_eq!(
            response
                .metadata
                .get(UNPROFILED_MODEL_METADATA_KEY)
                .map(String::as_str),
            Some("some-new-model")
        );
    }

    #[tokio::test]
    async fn compat_an_overridden_model_is_not_reported_unprofiled() {
        let request = build_request("tuned-model");
        let registry = test_registry(vec![(
            "tuned-model",
            crate::registry::ModelOverride {
                max_output_tokens: Some(1000),
                ..crate::registry::ModelOverride::default()
            },
        )]);

        let (outcome, _, _) = generate_through(engine_with(registry), request).await;
        let response = outcome.expect("the request is well-formed");

        assert!(
            !response
                .metadata
                .contains_key(UNPROFILED_MODEL_METADATA_KEY)
        );
    }

    #[tokio::test]
    async fn compat_response_format_follows_the_profile() {
        let schema = || ResponseFormat::JsonSchema {
            name: "answer".to_string(),
            schema: json!({"type": "object"}),
            strict: true,
        };

        // JsonObject still reaches the wire as json_object, unrecorded.
        let request = build_request("test-model").with_response_format(ResponseFormat::JsonObject);
        let (outcome, body, _) =
            generate_through(engine_with(test_registry(vec![])), request).await;
        let response = outcome.expect("well-formed");
        assert_eq!(
            body.expect("sent").get("response_format"),
            Some(&json!({"type": "json_object"}))
        );
        assert!(
            !response
                .metadata
                .contains_key(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY)
        );

        // JsonSchema is still sent as json_object, and the downgrade is now recorded.
        let request = build_request("test-model").with_response_format(schema());
        let (outcome, body, _) =
            generate_through(engine_with(test_registry(vec![])), request).await;
        let response = outcome.expect("well-formed");
        assert_eq!(
            body.expect("sent").get("response_format"),
            Some(&json!({"type": "json_object"}))
        );
        assert_eq!(
            response
                .metadata
                .get(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY)
                .map(String::as_str),
            Some("json_object")
        );

        // An override that says the model has no JSON mode drops the field and records it.
        let registry = test_registry(vec![(
            "no-json-model",
            crate::registry::ModelOverride {
                json_object: Some(false),
                ..crate::registry::ModelOverride::default()
            },
        )]);
        let request =
            build_request("no-json-model").with_response_format(ResponseFormat::JsonObject);
        let (outcome, body, _) = generate_through(engine_with(registry), request).await;
        let response = outcome.expect("well-formed");
        let body = body.expect("sent");
        assert!(
            body.get("response_format").is_none(),
            "no response_format key at all: {body}"
        );
        assert_eq!(
            response
                .metadata
                .get(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY)
                .map(String::as_str),
            Some("omitted")
        );
    }

    #[tokio::test]
    async fn compat_max_tokens_above_the_profile_is_refused() {
        let registry = test_registry(vec![(
            "small-output-model",
            crate::registry::ModelOverride {
                max_output_tokens: Some(1000),
                ..crate::registry::ModelOverride::default()
            },
        )]);
        let mut parameters = fully_specified_parameters();
        parameters.max_tokens = Some(1001);
        let request = build_request_with_parameters("small-output-model", parameters);

        let (outcome, _, hits) = generate_through(engine_with(registry), request).await;

        let Err(LlmError::InvalidPrompt(message)) = outcome else {
            panic!("max_tokens 1001 above a limit of 1000 must be InvalidPrompt: {outcome:?}");
        };
        for needle in [
            "test-preset",
            "small-output-model",
            "max_tokens",
            "1001",
            "1000",
        ] {
            assert!(
                message.contains(needle),
                "'{needle}' missing from: {message}"
            );
        }
        assert_eq!(hits, 0, "nothing may be sent for a refused request");
    }

    #[tokio::test]
    async fn compat_max_tokens_at_the_profile_limit_is_sent() {
        let registry = test_registry(vec![(
            "small-output-model",
            crate::registry::ModelOverride {
                max_output_tokens: Some(1000),
                ..crate::registry::ModelOverride::default()
            },
        )]);
        let mut parameters = fully_specified_parameters();
        parameters.max_tokens = Some(1000);
        let request = build_request_with_parameters("small-output-model", parameters);

        let (outcome, body, hits) = generate_through(engine_with(registry), request).await;

        outcome.expect("exactly the limit is allowed");
        assert_eq!(hits, 1);
        assert_eq!(
            body.expect("sent")
                .get("max_tokens")
                .and_then(Value::as_u64),
            Some(1000)
        );
    }

    // ── D-13/D-14/D-15/D-20: streaming usage terminal-chunk contract ──────

    /// The exact `usage` object every test below both streams and returns
    /// non-streaming, so the two paths can be compared field-for-field.
    /// Values are distinct and non-round so a swapped or dropped field
    /// cannot pass by coincidence (house style, `table_herald.rs`/
    /// `json_herald.rs`).
    fn detail_bearing_usage_json() -> Value {
        json!({
            "prompt_tokens": 37,
            "completion_tokens": 19,
            "total_tokens": 56,
            "prompt_tokens_details": {"cached_tokens": 11},
            "completion_tokens_details": {"reasoning_tokens": 5}
        })
    }

    fn expected_detail_bearing_usage() -> TokenUsage {
        TokenUsage::new(37, 19)
            .with_cache_read(11)
            .with_reasoning(5)
    }

    /// Two content frames, then a frame whose `choices[0].finish_reason` is
    /// `"stop"` (itself carrying no content), then a frame with empty
    /// `choices` and a full `usage` object, then `[DONE]`.
    fn sse_body_with_usage_frame() -> String {
        format!(
            "data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"Hel\"}},\"finish_reason\":null}}]}}\n\n\
             data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"lo\"}},\"finish_reason\":null}}]}}\n\n\
             data: {{\"id\":\"1\",\"choices\":[{{\"delta\":{{\"content\":\"\"}},\"finish_reason\":\"stop\"}}]}}\n\n\
             data: {{\"choices\":[],\"usage\":{}}}\n\n\
             data: [DONE]\n\n",
            detail_bearing_usage_json()
        )
    }

    /// The same body with the usage frame removed entirely.
    fn sse_body_without_usage_frame() -> String {
        concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        )
        .to_string()
    }

    #[tokio::test]
    async fn generate_stream_holds_finish_reason_and_usage_for_the_one_terminal_chunk() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body_with_usage_frame())
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let stream = engine
            .generate_stream(build_request("test-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut chunks = Vec::new();
        while let Some(item) = stream.next().await {
            chunks.push(item.unwrap());
        }

        let finish_indices: Vec<usize> = chunks
            .iter()
            .enumerate()
            .filter(|(_, c)| c.finish_reason.is_some())
            .map(|(i, _)| i)
            .collect();
        let usage_indices: Vec<usize> = chunks
            .iter()
            .enumerate()
            .filter(|(_, c)| c.usage.is_some())
            .map(|(i, _)| i)
            .collect();

        assert_eq!(
            finish_indices.len(),
            1,
            "exactly one chunk must carry a finish reason"
        );
        assert_eq!(usage_indices.len(), 1, "exactly one chunk must carry usage");
        assert_eq!(
            finish_indices, usage_indices,
            "the finish-reason chunk and the usage chunk must be the SAME chunk"
        );

        let terminal = &chunks[finish_indices[0]];
        assert!(matches!(terminal.finish_reason, Some(FinishReason::Stop)));
        assert_eq!(terminal.usage, Some(expected_detail_bearing_usage()));

        let assembled: String = chunks.iter().map(|c| c.delta.as_str()).collect();
        assert_eq!(
            assembled, "Hello",
            "wire order must be unaffected by the usage contract"
        );
    }

    #[tokio::test]
    async fn generate_stream_terminal_usage_equals_the_non_streaming_usage_field_for_field() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-1",
                    "model": "test-model",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hello"},
                        "finish_reason": "stop"
                    }],
                    "usage": detail_bearing_usage_json()
                })
                .to_string(),
            )
            .create_async()
            .await;
        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let non_streaming = engine.generate(build_request("test-model")).await.unwrap();

        let mut stream_server = Server::new_async().await;
        stream_server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body_with_usage_frame())
            .create_async()
            .await;
        let stream_engine = CompatEngine::new(test_config_at(&stream_server.url())).unwrap();
        let stream = stream_engine
            .generate_stream(build_request("test-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut terminal_usage = None;
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            if chunk.usage.is_some() {
                terminal_usage = chunk.usage;
            }
        }

        assert_eq!(terminal_usage, Some(non_streaming.usage));
    }

    #[tokio::test]
    async fn generate_stream_without_a_usage_frame_yields_terminal_chunk_with_usage_none() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_body_without_usage_frame())
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let stream = engine
            .generate_stream(build_request("test-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);

        let mut chunks = Vec::new();
        while let Some(item) = stream.next().await {
            chunks.push(item.unwrap());
        }

        let terminal = chunks
            .iter()
            .find(|c| c.finish_reason.is_some())
            .expect("a terminal chunk with a finish reason must exist");
        assert!(matches!(terminal.finish_reason, Some(FinishReason::Stop)));
        assert!(terminal.usage.is_none());
        assert!(
            chunks.iter().all(|c| c.usage.is_none()),
            "no chunk may carry usage when the provider never reported one"
        );
    }

    #[tokio::test]
    async fn compat_engine_streaming_request_carries_stream_options_include_usage() {
        let mut server = Server::new_async().await;
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured);

        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body_from_request(move |req| {
                let body_text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                *captured_clone.lock().unwrap() = Some(body_text);
                b"data: [DONE]\n\n".to_vec()
            })
            .create_async()
            .await;

        let engine = CompatEngine::new(test_config_at(&server.url())).unwrap();
        let stream = engine
            .generate_stream(build_request("test-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);
        while stream.next().await.is_some() {}

        let body_text = captured
            .lock()
            .unwrap()
            .clone()
            .expect("request must have been captured");
        let body: Value = serde_json::from_str(&body_text).unwrap();
        assert_eq!(
            body.get("stream_options"),
            Some(&json!({"include_usage": true}))
        );
    }

    #[tokio::test]
    async fn compat_engine_non_streaming_request_carries_no_stream_options_key() {
        let config = test_config_at("https://example.invalid/v1");
        let request = build_request("test-model");

        let (body, _) = generate_and_capture_body(config, request).await;
        let obj = body.as_object().expect("body must be a JSON object");

        assert!(
            !obj.contains_key("stream_options"),
            "a non-streaming request must never carry stream_options, got: {obj:?}"
        );
    }

    // ---- PARITY-05: refusal, empty output, reasoning text and cache-write tokens ----

    /// POST a canned non-streaming body to a mock endpoint and return what `generate` made of it.
    async fn generate_from_body(body: Value) -> Result<LlmResponse, LlmError> {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;
        CompatEngine::new(test_config_at(&server.url()))
            .unwrap()
            .with_provider_name("test-preset")
            .generate(build_request("test-model"))
            .await
    }

    fn buffered_body(message: Value, finish_reason: &str) -> Value {
        json!({
            "id": "cmpl-1",
            "model": "test-model",
            "choices": [{"index": 0, "message": message, "finish_reason": finish_reason}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
        })
    }

    /// Drain `generate_stream` over a canned SSE body into its ordered items.
    async fn stream_from_sse(sse: &str) -> Vec<Result<StreamingResponse, LlmError>> {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse)
            .create_async()
            .await;
        let engine = CompatEngine::new(test_config_at(&server.url()))
            .unwrap()
            .with_provider_name("test-preset");
        let stream = engine
            .generate_stream(build_request("test-model"))
            .await
            .unwrap();
        let mut stream = Box::into_pin(stream);
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }
        items
    }

    /// G-50-5a (plan 50-26 tracer): a call the vendor answered and billed but that returned no
    /// text -- a reasoning model that spent its whole budget thinking -- reaches the caller with
    /// its usage on the error: the body's served model, the mapped usage and no cost (pricing it
    /// is plan 50-27). With no usage reported there is no bill, never a fabricated zero (D-00c).
    #[tokio::test]
    async fn compat_billed_empty_completion_carries_its_usage() {
        let mut body = json!({
            "id": "cmpl-1",
            "model": "kimi-served-test",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": ""},
                "finish_reason": "length"
            }],
            "usage": {"prompt_tokens": 1_000, "completion_tokens": 62_000, "total_tokens": 63_000}
        });

        let err = generate_from_body(body.clone())
            .await
            .expect_err("an empty length completion is an error");
        assert!(
            matches!(err, LlmError::EmptyCompletion { .. }),
            "expected EmptyCompletion, got {err:?}"
        );
        let billed = err
            .billed_usage()
            .expect("a billed empty completion carries its usage");
        assert_eq!(billed.model, "kimi-served-test");
        assert_eq!(billed.usage.prompt_tokens, 1_000);
        assert_eq!(billed.usage.completion_tokens, 62_000);
        assert_eq!(billed.usage.total_tokens, 63_000);
        assert_eq!(billed.cost, None);

        body.as_object_mut()
            .expect("the body is a JSON object")
            .remove("usage");
        let err = generate_from_body(body)
            .await
            .expect_err("an empty length completion is an error");
        assert!(
            matches!(err, LlmError::EmptyCompletion { .. }),
            "expected EmptyCompletion, got {err:?}"
        );
        assert_eq!(err.billed_usage(), None);
    }

    /// D-11a: empty text on a `stop` finish is an `EmptyCompletion`, no longer `Ok("")`.
    #[tokio::test]
    async fn empty_stop_completion_is_empty_completion() {
        let result = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": ""}),
            "stop",
        ))
        .await;
        match result {
            Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                assert!(msg.contains("finish_reason=stop"), "{msg}");
                assert!(!msg.contains("larger max_tokens"), "{msg}");
            }
            other => panic!("expected EmptyCompletion, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn whitespace_only_completion_is_empty_and_null_content_too() {
        for message in [
            json!({"role": "assistant", "content": "  \n\t"}),
            json!({"role": "assistant", "content": null}),
        ] {
            let result = generate_from_body(buffered_body(message, "stop")).await;
            assert!(
                matches!(result, Err(LlmError::EmptyCompletion { .. })),
                "got {result:?}"
            );
        }
    }

    #[tokio::test]
    async fn empty_length_completion_keeps_the_larger_max_tokens_advice() {
        let result = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": ""}),
            "length",
        ))
        .await;
        match result {
            Err(LlmError::EmptyCompletion { detail: msg, .. }) => {
                assert!(msg.contains("finish_reason=length"), "{msg}");
                assert!(msg.contains("larger max_tokens"), "{msg}");
            }
            other => panic!("expected EmptyCompletion, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn non_empty_content_is_never_an_empty_completion() {
        for finish in ["stop", "length"] {
            let response = generate_from_body(buffered_body(
                json!({"role": "assistant", "content": "some answer"}),
                finish,
            ))
            .await
            .expect("text is a complete answer on any finish reason");
            assert_eq!(response.content, "some answer");
        }
    }

    /// D-09 / adjacency: a `content_filter` finish with no text stays `Ok`.
    #[tokio::test]
    async fn content_filter_with_empty_text_stays_ok() {
        let response = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": ""}),
            "content_filter",
        ))
        .await
        .expect("a content filter is a finish reason, not an error");
        assert!(matches!(
            response.finish_reason,
            FinishReason::ContentFilter
        ));
    }

    #[tokio::test]
    async fn compat_buffered_refusal_is_typed_and_names_provider_and_model() {
        let result = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": "", "refusal": "I can't help with that."}),
            "stop",
        ))
        .await;
        match result {
            Err(LlmError::Refusal {
                provider,
                model,
                detail,
                ..
            }) => {
                assert_eq!(provider, "test-preset");
                assert_eq!(model, "test-model");
                assert_eq!(detail, "I can't help with that.");
            }
            other => panic!("expected Refusal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn compat_buffered_refusal_redacts_the_configured_key() {
        // `test_config` carries the key "test-key".
        let result = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": null, "refusal": "no, test-key is mine"}),
            "stop",
        ))
        .await;
        match result {
            Err(LlmError::Refusal { detail, .. }) => {
                assert!(!detail.contains("test-key"), "{detail}");
                assert!(detail.contains("[REDACTED]"), "{detail}");
            }
            other => panic!("expected Refusal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn compat_buffered_reasoning_text_is_surfaced_and_empty_is_none() {
        let with_reasoning = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": "ok", "reasoning_content": "think"}),
            "stop",
        ))
        .await
        .unwrap();
        assert_eq!(with_reasoning.reasoning.as_deref(), Some("think"));

        for blank in ["", "   "] {
            let response = generate_from_body(buffered_body(
                json!({"role": "assistant", "content": "ok", "reasoning_content": blank}),
                "stop",
            ))
            .await
            .unwrap();
            assert_eq!(response.reasoning, None, "blank reasoning {blank:?}");
        }

        let absent = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": "ok"}),
            "stop",
        ))
        .await
        .unwrap();
        assert_eq!(absent.reasoning, None);
        assert!(
            !absent.metadata.contains_key("reasoning_content"),
            "reasoning text never travels through metadata"
        );
    }

    #[tokio::test]
    async fn compat_buffered_cache_write_tokens_are_mapped() {
        let mut body = buffered_body(json!({"role": "assistant", "content": "ok"}), "stop");
        body["usage"] = json!({
            "prompt_tokens": 30,
            "completion_tokens": 2,
            "total_tokens": 32,
            "prompt_tokens_details": {"cached_tokens": 4, "cache_write_tokens": 7}
        });
        let response = generate_from_body(body).await.unwrap();
        assert_eq!(response.usage.cache_write_tokens, Some(7));
        assert_eq!(response.usage.cache_read_tokens, Some(4));
    }

    /// Edge PARITY-05/precision: sub-counts are copied exactly as the vendor reported them, and
    /// a vendor total that disagrees is recorded, not discarded (G-50-5a, owner decision
    /// 2026-10-17; this test previously pinned the discard). `total_tokens` stays
    /// `prompt_tokens + completion_tokens` (D-02): the disagreement is surfaced under
    /// `paladin.usage_unreconciled`, never written over the mapped total.
    #[tokio::test]
    async fn compat_vendor_total_disagreement_is_recorded() {
        let mut body = buffered_body(json!({"role": "assistant", "content": "ok"}), "stop");
        body["usage"] = json!({
            "prompt_tokens": 1234,
            "completion_tokens": 567,
            "total_tokens": 9999,
            "prompt_tokens_details": {"cached_tokens": 1000, "cache_write_tokens": 200},
            "completion_tokens_details": {"reasoning_tokens": 321}
        });
        let response = generate_from_body(body).await.unwrap();
        assert_eq!(
            response
                .metadata
                .get(USAGE_UNRECONCILED_METADATA_KEY)
                .map(String::as_str),
            Some("total:vendor=9999,mapped=1801")
        );
        let usage = response.usage;
        assert_eq!(usage.prompt_tokens, 1234);
        assert_eq!(usage.completion_tokens, 567);
        assert_eq!(usage.cache_read_tokens, Some(1000));
        assert_eq!(usage.cache_write_tokens, Some(200));
        assert_eq!(usage.reasoning_tokens, Some(321));
        assert_eq!(usage.total_tokens, 1234 + 567);

        // An absent sub-count stays `None`, never a fabricated `Some(0)` (D-03).
        let bare = generate_from_body(buffered_body(
            json!({"role": "assistant", "content": "ok"}),
            "stop",
        ))
        .await
        .unwrap()
        .usage;
        assert_eq!(bare.cache_write_tokens, None);
        assert_eq!(bare.cache_read_tokens, None);
        assert_eq!(bare.reasoning_tokens, None);
    }

    /// Edge PARITY-05/ordering: reasoning deltas arrive in wire order, interleaved with content
    /// exactly as sent, and only the terminal chunk carries the finish reason and usage.
    #[tokio::test]
    async fn compat_stream_reasoning_keeps_wire_order() {
        let sse = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\"think \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\"harder\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"An\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\" more\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"swer\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":6,\"total_tokens\":10}}\n\n",
            "data: [DONE]\n\n",
        );
        let chunks: Vec<StreamingResponse> = stream_from_sse(sse)
            .await
            .into_iter()
            .map(|item| item.expect("a stream with text has no error item"))
            .collect();

        let reasoning: Vec<Option<&str>> = chunks.iter().map(|c| c.reasoning.as_deref()).collect();
        assert_eq!(
            reasoning,
            vec![
                Some("think "),
                Some("harder"),
                None,
                Some(" more"),
                None,
                None
            ],
            "reasoning deltas stay on the chunk that carried them, in wire order"
        );
        let content: Vec<&str> = chunks.iter().map(|c| c.delta.as_str()).collect();
        assert_eq!(content, vec!["", "", "An", "", "swer", ""]);

        let terminal_indices: Vec<usize> = chunks
            .iter()
            .enumerate()
            .filter(|(_, c)| c.finish_reason.is_some())
            .map(|(i, _)| i)
            .collect();
        assert_eq!(terminal_indices, vec![chunks.len() - 1]);
        let terminal = chunks.last().unwrap();
        assert!(matches!(terminal.finish_reason, Some(FinishReason::Stop)));
        assert_eq!(terminal.usage, Some(TokenUsage::new(4, 6)));
        assert!(chunks[..chunks.len() - 1].iter().all(|c| c.usage.is_none()));
    }

    /// D-11a on the streaming path: reasoning, then a `stop` with no content, ends in
    /// `Err(EmptyCompletion)` instead of the `Ok` terminal chunk; the reasoning stays yielded.
    #[tokio::test]
    async fn compat_empty_stream_is_empty_completion() {
        let sse = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\"hmm\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"  \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        let items = stream_from_sse(sse).await;

        let (last, yielded) = items.split_last().expect("the stream yields items");
        assert!(
            matches!(last, Err(LlmError::EmptyCompletion { .. })),
            "the last item must be EmptyCompletion, got {last:?}"
        );
        assert!(
            yielded.iter().all(|item| item.is_ok()),
            "everything before the terminal item stays Ok"
        );
        assert_eq!(
            yielded[0].as_ref().unwrap().reasoning.as_deref(),
            Some("hmm"),
            "reasoning already yielded stays yielded"
        );
        assert!(
            items
                .iter()
                .all(|item| !matches!(item, Ok(chunk) if chunk.finish_reason.is_some())),
            "no Ok terminal chunk is emitted"
        );
    }

    #[tokio::test]
    async fn compat_content_filter_stream_with_no_text_stays_an_ok_terminal_chunk() {
        let sse = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{},\"finish_reason\":\"content_filter\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        let items = stream_from_sse(sse).await;
        let terminal = items.last().unwrap().as_ref().expect("no error item");
        assert!(matches!(
            terminal.finish_reason,
            Some(FinishReason::ContentFilter)
        ));
    }

    #[tokio::test]
    async fn compat_stream_refusal_is_the_last_item_and_wins_over_partial_text() {
        let sse = concat!(
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Sure, \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"refusal\":\"I can't \"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"refusal\":\"help.\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        let items = stream_from_sse(sse).await;
        match items.last().unwrap() {
            Err(LlmError::Refusal {
                provider,
                model,
                detail,
                ..
            }) => {
                assert_eq!(provider, "test-preset");
                assert_eq!(model, "test-model");
                assert_eq!(detail, "I can't help.");
            }
            other => panic!("expected Refusal as the last item, got {other:?}"),
        }
        assert_eq!(
            items[0].as_ref().unwrap().delta,
            "Sure, ",
            "partial content already yielded stays yielded"
        );
    }
    // ── Phase 51 (REASON-02, REASON-03, D-00f): the reasoning wires, exactly or refused ──
    //
    // One set of cases for every preset that shares this engine, against a synthetic preset that
    // declares each wire in turn. The wire is the only difference between the three accepted
    // cases, so a body that carries another wire's field fails them.

    mod reasoning_wires {
        use super::*;
        use crate::reasoning::{
            REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
        };
        use crate::registry::ModelOverride;
        use paladin_ports::output::llm_port::ReasoningEffort::{
            Auto, Budget, High, Low, Max, Medium, Minimal, Off, XHigh,
        };
        use paladin_ports::output::model_profile::{BudgetBounds, EffortLevels, EffortSupport};
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// The four request fields an effort can set; every other key is outside this module's
        /// business.
        const REASONING_KEYS: [&str; 4] = [
            "reasoning_effort",
            "thinking",
            "enable_thinking",
            "thinking_budget",
        ];

        fn engine_for(
            wire: ReasoningWire,
            effort: EffortSupport,
        ) -> impl FnOnce(String) -> CompatEngine {
            move |url| {
                let mut config = test_config_at(&url);
                config.request_parameters = CompatRequestParameters::all(wire);
                let registry = test_registry(vec![(
                    "test-model",
                    ModelOverride {
                        effort: Some(effort),
                        ..ModelOverride::default()
                    },
                )]);
                CompatEngine::new(config)
                    .unwrap()
                    .with_provider_name("test-preset")
                    .with_model_registry(registry)
            }
        }

        fn levels() -> EffortSupport {
            EffortSupport::Supported(
                EffortLevels::new([Low, Medium, High])
                    .with_off(true)
                    .with_auto(true)
                    .with_budget(BudgetBounds::new(1, 32_768)),
            )
        }

        fn budget_only() -> EffortSupport {
            EffortSupport::Supported(
                EffortLevels::new([])
                    .with_off(true)
                    .with_auto(true)
                    .with_budget(BudgetBounds::new(1, 32_768)),
            )
        }

        async fn send(
            wire: ReasoningWire,
            effort: EffortSupport,
            requested: paladin_ports::output::llm_port::ReasoningEffort,
        ) -> (Result<LlmResponse, LlmError>, Option<Value>, usize) {
            generate_through(
                engine_for(wire, effort),
                build_request("test-model").with_reasoning(requested),
            )
            .await
        }

        fn reasoning_keys(body: &Value) -> Vec<&'static str> {
            REASONING_KEYS
                .into_iter()
                .filter(|key| body.get(key).is_some())
                .collect()
        }

        #[tokio::test]
        async fn top_level_effort_sends_the_level_and_nothing_else() {
            let (result, body, _) = send(ReasoningWire::TopLevelEffort, levels(), Medium).await;
            let metadata = result.expect("a listed level is accepted").metadata;
            let body = body.unwrap();
            assert_eq!(body["reasoning_effort"], "medium", "{body}");
            assert_eq!(reasoning_keys(&body), ["reasoning_effort"], "{body}");
            assert_eq!(metadata[REASONING_REQUESTED_METADATA_KEY], "medium");
            assert_eq!(metadata[REASONING_EFFECTIVE_METADATA_KEY], "medium");
        }

        #[tokio::test]
        async fn top_level_effort_off_is_none_and_auto_sends_nothing() {
            let (result, body, _) = send(ReasoningWire::TopLevelEffort, levels(), Off).await;
            result.expect("off is accepted");
            assert_eq!(body.unwrap()["reasoning_effort"], "none");

            let (result, body, hits) = send(ReasoningWire::TopLevelEffort, levels(), Auto).await;
            let metadata = result.expect("auto is accepted").metadata;
            assert_eq!(hits, 1);
            assert_eq!(reasoning_keys(&body.unwrap()), Vec::<&str>::new());
            assert_eq!(metadata[REASONING_REQUESTED_METADATA_KEY], "auto");
            assert_eq!(metadata[REASONING_EFFECTIVE_METADATA_KEY], "auto");
        }

        #[tokio::test]
        async fn top_level_effort_has_no_budget_field_so_a_budget_is_refused() {
            let (result, body, hits) =
                send(ReasoningWire::TopLevelEffort, levels(), Budget(100)).await;
            assert!(matches!(result, Err(LlmError::UnsupportedParameter { .. })));
            assert_eq!((hits, body), (0, None));
        }

        #[tokio::test]
        async fn thinking_object_sends_level_off_and_auto_in_its_own_field() {
            for (requested, expected) in [
                (High, json!({"type": "enabled", "effort": "high"})),
                (Off, json!({"type": "disabled"})),
                (Auto, json!({"type": "enabled"})),
            ] {
                let (result, body, _) =
                    send(ReasoningWire::ThinkingObject, levels(), requested).await;
                result.expect("accepted");
                let body = body.unwrap();
                assert_eq!(body["thinking"], expected, "{requested}: {body}");
                assert_eq!(reasoning_keys(&body), ["thinking"], "{requested}: {body}");
            }
            let (result, _, hits) =
                send(ReasoningWire::ThinkingObject, levels(), Budget(100)).await;
            assert!(matches!(result, Err(LlmError::UnsupportedParameter { .. })));
            assert_eq!(hits, 0);
        }

        #[tokio::test]
        async fn enable_thinking_sends_the_switch_and_a_bounded_budget() {
            let (result, body, _) =
                send(ReasoningWire::EnableThinking, budget_only(), Budget(512)).await;
            result.expect("an in-bounds budget is accepted");
            let body = body.unwrap();
            assert_eq!(body["enable_thinking"], true, "{body}");
            assert_eq!(body["thinking_budget"], 512, "{body}");
            assert_eq!(
                reasoning_keys(&body),
                ["enable_thinking", "thinking_budget"],
                "{body}"
            );

            let (result, body, _) = send(ReasoningWire::EnableThinking, budget_only(), Off).await;
            result.expect("off is accepted");
            let body = body.unwrap();
            assert_eq!(body["enable_thinking"], false, "{body}");
            assert_eq!(reasoning_keys(&body), ["enable_thinking"], "{body}");

            let (result, body, _) = send(ReasoningWire::EnableThinking, budget_only(), Auto).await;
            result.expect("auto is accepted");
            let body = body.unwrap();
            assert_eq!(body["enable_thinking"], true, "{body}");
            assert_eq!(reasoning_keys(&body), ["enable_thinking"], "{body}");
        }

        #[tokio::test]
        async fn enable_thinking_refuses_an_out_of_bounds_budget_and_every_level() {
            let (result, _, hits) =
                send(ReasoningWire::EnableThinking, budget_only(), Budget(32_769)).await;
            assert!(matches!(result, Err(LlmError::UnsupportedParameter { .. })));
            assert_eq!(hits, 0);

            // The wire has no level field, so even an override listing levels cannot unlock one.
            let (result, _, hits) = send(ReasoningWire::EnableThinking, levels(), Low).await;
            assert!(matches!(result, Err(LlmError::UnsupportedParameter { .. })));
            assert_eq!(hits, 0);
        }

        #[tokio::test]
        async fn a_budget_not_below_max_tokens_is_refused_before_http() {
            // The synthetic request carries a 1000-token output cap; a 2048-token budget leaves
            // the model no room to answer.
            let (result, _, hits) =
                send(ReasoningWire::EnableThinking, budget_only(), Budget(2048)).await;
            match result {
                Err(LlmError::UnsupportedParameter { reason, .. }) => {
                    assert!(reason.contains("below max_tokens"), "{reason}");
                }
                other => panic!("expected UnsupportedParameter, got {other:?}"),
            }
            assert_eq!(hits, 0);
        }

        #[tokio::test]
        async fn the_budget_bound_comes_from_the_profile_not_a_constant() {
            let narrow = EffortSupport::Supported(
                EffortLevels::new([]).with_budget(BudgetBounds::new(10, 20)),
            );
            let (result, _, _) =
                send(ReasoningWire::EnableThinking, narrow.clone(), Budget(20)).await;
            result.expect("the profile's upper bound is inclusive");
            let (result, _, hits) = send(ReasoningWire::EnableThinking, narrow, Budget(21)).await;
            assert!(matches!(result, Err(LlmError::UnsupportedParameter { .. })));
            assert_eq!(hits, 0);
        }

        #[tokio::test]
        async fn an_unsupported_wire_refuses_all_nine_values_with_zero_http_hits() {
            for requested in [
                Off,
                Auto,
                Minimal,
                Low,
                Medium,
                High,
                XHigh,
                Max,
                Budget(1024),
            ] {
                // An override that lists everything still cannot unlock a wire with no form.
                let (result, body, hits) =
                    send(ReasoningWire::Unsupported, levels(), requested).await;
                match result {
                    Err(LlmError::UnsupportedParameter { reason, .. }) => {
                        assert!(reason.contains("not verified"), "{requested}: {reason}");
                    }
                    other => panic!("{requested}: expected UnsupportedParameter, got {other:?}"),
                }
                assert_eq!((hits, body), (0, None), "{requested}");
            }
        }

        #[tokio::test]
        async fn an_unsupported_wire_with_no_effort_set_sends_and_stamps_nothing() {
            let (result, body, hits) = generate_through(
                engine_for(ReasoningWire::Unsupported, levels()),
                build_request("test-model"),
            )
            .await;
            let metadata = result.expect("an unset effort is untouched").metadata;
            assert_eq!(hits, 1);
            assert_eq!(reasoning_keys(&body.unwrap()), Vec::<&str>::new());
            assert!(!metadata.contains_key(REASONING_REQUESTED_METADATA_KEY));
            assert!(!metadata.contains_key(REASONING_EFFECTIVE_METADATA_KEY));
        }

        /// REASON-03 / T-51-13: no body ever carries a field of a wire the preset did not
        /// declare, for any effort that wire admits.
        #[tokio::test]
        async fn no_wire_ever_sends_a_second_wires_field() {
            let cases: [(ReasoningWire, EffortSupport, &[&str]); 3] = [
                (
                    ReasoningWire::TopLevelEffort,
                    levels(),
                    &["reasoning_effort"],
                ),
                (ReasoningWire::ThinkingObject, levels(), &["thinking"]),
                (
                    ReasoningWire::EnableThinking,
                    budget_only(),
                    &["enable_thinking", "thinking_budget"],
                ),
            ];
            for (wire, effort, own) in cases {
                for requested in [Off, Auto, Low, Medium, High, Budget(512)] {
                    let (result, body, _) = send(wire, effort.clone(), requested).await;
                    if result.is_err() {
                        continue; // refused: nothing was sent at all
                    }
                    for key in reasoning_keys(&body.unwrap()) {
                        assert!(
                            own.contains(&key),
                            "{wire} sent '{key}' for {requested}, which is not its own field"
                        );
                    }
                }
            }
        }

        /// Run `generate_stream` against a mock SSE endpoint, capturing the request body.
        async fn stream_capture(
            wire: ReasoningWire,
            effort: EffortSupport,
            requested: paladin_ports::output::llm_port::ReasoningEffort,
        ) -> (
            Option<Value>,
            usize,
            Result<Vec<StreamingResponse>, LlmError>,
        ) {
            let mut server = Server::new_async().await;
            let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
            let hits = Arc::new(AtomicUsize::new(0));
            let (slot, counter) = (Arc::clone(&captured), Arc::clone(&hits));
            server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_header("content-type", "text/event-stream")
                .with_body_from_request(move |req| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                    *slot.lock().unwrap() = serde_json::from_str(&text).ok();
                    sse_body_with_usage_frame().into_bytes()
                })
                .expect_at_least(0)
                .create_async()
                .await;
            let engine = engine_for(wire, effort)(server.url());
            let outcome = match engine
                .generate_stream(build_request("test-model").with_reasoning(requested))
                .await
            {
                Ok(stream) => {
                    let mut stream = Box::into_pin(stream);
                    let mut chunks = Vec::new();
                    while let Some(item) = stream.next().await {
                        chunks.push(item.expect("every scripted item is Ok"));
                    }
                    Ok(chunks)
                }
                Err(error) => Err(error),
            };
            let body = captured.lock().unwrap().take();
            (body, hits.load(Ordering::SeqCst), outcome)
        }

        #[tokio::test]
        async fn the_stream_sends_the_same_fields_as_generate_for_each_wire() {
            let cases: [(ReasoningWire, EffortSupport, _, &str); 3] = [
                (
                    ReasoningWire::TopLevelEffort,
                    levels(),
                    High,
                    "reasoning_effort",
                ),
                (ReasoningWire::ThinkingObject, levels(), High, "thinking"),
                (
                    ReasoningWire::EnableThinking,
                    budget_only(),
                    Budget(512),
                    "thinking_budget",
                ),
            ];
            for (wire, effort, requested, key) in cases {
                let (buffered, buffered_body, _) = send(wire, effort.clone(), requested).await;
                buffered.expect("accepted");
                let buffered_body = buffered_body.unwrap();

                let (body, hits, outcome) = stream_capture(wire, effort, requested).await;
                let body = body.expect("the stream request reached the wire");
                assert_eq!(hits, 1, "{wire}");
                assert_eq!(body[key], buffered_body[key], "{wire}: {body}");
                assert_eq!(
                    reasoning_keys(&body),
                    reasoning_keys(&buffered_body),
                    "{wire}"
                );

                let chunks = outcome.expect("the stream opens");
                let (terminal, earlier) = chunks.split_last().expect("a terminal chunk");
                assert_eq!(
                    terminal.metadata[REASONING_REQUESTED_METADATA_KEY],
                    requested.to_string(),
                    "{wire}"
                );
                assert_eq!(
                    terminal.metadata[REASONING_EFFECTIVE_METADATA_KEY],
                    requested.to_string(),
                    "{wire}"
                );
                for chunk in earlier {
                    assert!(
                        !chunk
                            .metadata
                            .contains_key(REASONING_REQUESTED_METADATA_KEY),
                        "{wire}: only the terminal chunk is stamped: {chunk:?}"
                    );
                }
            }
        }

        #[tokio::test]
        async fn a_refused_effort_fails_the_stream_before_any_http_call() {
            let (body, hits, outcome) =
                stream_capture(ReasoningWire::Unsupported, levels(), High).await;
            assert!(matches!(
                outcome,
                Err(LlmError::UnsupportedParameter { .. })
            ));
            assert_eq!((hits, body), (0, None));
        }

        #[test]
        fn the_masked_profile_carries_only_what_each_wire_can_send() {
            let profile_for = |wire: ReasoningWire, effort: EffortSupport| {
                let engine = engine_for(wire, effort)("https://example.invalid/v1".to_string());
                engine.model_profile("test-model").effort
            };
            match profile_for(ReasoningWire::TopLevelEffort, levels()) {
                EffortSupport::Supported(l) => {
                    assert_eq!(l.levels, vec![Low, Medium, High]);
                    assert_eq!(l.budget, None);
                }
                other => panic!("expected Supported, got {other:?}"),
            }
            match profile_for(ReasoningWire::EnableThinking, levels()) {
                EffortSupport::Supported(l) => {
                    assert!(l.levels.is_empty() && l.coerced.is_empty());
                    assert!(l.budget.is_some() && l.off && l.auto);
                }
                other => panic!("expected Supported, got {other:?}"),
            }
            assert_eq!(
                profile_for(ReasoningWire::Unsupported, levels()),
                EffortSupport::WireUnsupported
            );
        }

        #[test]
        fn a_refused_effort_is_a_configuration_statement_not_a_fetch_condition() {
            let error = LlmError::unsupported_parameter(
                "reasoning_effort",
                "high",
                "test-preset",
                "test-model",
                "reason",
            );
            assert_eq!(
                classify_fetch_failure(&error),
                FetchFailureClass::Misconfiguration
            );
        }
    }
}
