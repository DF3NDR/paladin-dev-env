//! Live vendor smoke harness -- all nine providers, credential-gated (Phase 17 UAT test 4,
//! widened by Phase 50 PARITY-07).
//!
//! One table-driven harness ([`PROVIDERS`]) covers OpenAI, Anthropic, DeepSeek, Kimi
//! (Moonshot), Qwen (DashScope), Grok (xAI), Gemini, Ollama and the generic
//! OpenAI-compatible adapter. Every provider is behind the same credential gate:
//!
//! | Provider          | Runs only when                                          |
//! |-------------------|---------------------------------------------------------|
//! | OpenAI            | `OPENAI_API_KEY`                                        |
//! | Anthropic         | `ANTHROPIC_API_KEY`                                     |
//! | DeepSeek          | `DEEPSEEK_API_KEY`                                      |
//! | Kimi              | `MOONSHOT_API_KEY`                                      |
//! | Qwen              | `DASHSCOPE_API_KEY`                                     |
//! | Grok              | `XAI_API_KEY`                                           |
//! | Gemini            | `GEMINI_API_KEY`                                        |
//! | Ollama            | `OLLAMA_BASE_URL` (a local daemon has no key)           |
//! | OpenAI-compatible | `OPENAI_COMPATIBLE_API_KEY` and `OPENAI_COMPATIBLE_BASE_URL` |
//!
//! A provider whose credential is absent is printed as `SKIPPED (<VAR> not set)` and is never
//! counted as a pass. A run in which no provider was configured exits non-zero with
//! `no provider was configured -- nothing was tested`: a harness that exits 0 having tested
//! nothing is the vacuous pass this file exists to refuse (the Snyk lesson recorded in
//! `.github/instructions/security.instructions.md`).
//!
//! # Smoke mode (no flag)
//!
//! Two probes per configured provider, kept from the original harness:
//!   1. `get_available_models()` returns from the LIVE-FETCH path, not the curated fallback,
//!      and the adapter's default model ID exists in that live list. `available_models()`
//!      swallows every failure and silently returns the curated `*_FALLBACK_MODELS` constant,
//!      so a non-empty result is NOT evidence the network path worked; this harness
//!      discriminates by comparing the result against that exact constant. Anthropic and
//!      DeepSeek have no live fetch (their adapters return a static list), so that half is
//!      reported `NOT APPLICABLE` and is not counted as a pass.
//!   2. `generate()` completes a real call using the framework's DEFAULT prompt parameters
//!      (`PromptParameters::default()`). A `generate()` that returns `Err`, or `Ok` with empty
//!      content or zero total tokens, is a FAIL -- a 200 carrying nothing is the vacuous pass
//!      this harness refuses.
//!
//! Exits non-zero if any configured provider fails a probe.
//!
//! # Usage mode (`--usage`)
//!
//! One reasoning-eliciting call per candidate model per configured provider, over the raw vendor
//! wire (not the adapters), recording how the vendor's `usage` object relates reasoning tokens
//! to `completion_tokens` and to its own total (Phase 50 gap G-50-5a). Each row carries numbers
//! only -- prompt, completion, reasoning, vendor total and a few extras (cached, cache-write,
//! tool-use prompt, Grok's `cost_in_usd_ticks` and `num_sources_used`) -- plus one verdict derived
//! from that arithmetic:
//!
//! | Verdict                                              | Meaning                                    |
//! |------------------------------------------------------|--------------------------------------------|
//! | `reasoning inside completion`                        | total = prompt + completion, reasoning > 0 |
//! | `reasoning beside completion`                        | total = prompt + completion + reasoning    |
//! | `indeterminate (no reasoning reported)`              | reasoning absent or 0, total agrees        |
//! | `no vendor total`                                    | Anthropic (or a body with no total)        |
//! | `other (total - (prompt + completion) = <n>)`        | anything else, signed                      |
//! | `not measured (<reason>)`                            | no usable response                         |
//!
//! Response content is never printed. `--usage --dry-run` lists the planned calls without network
//! access or credentials.
//!
//! # Effort mode (`--effort`)
//!
//! The Phase 51 reasoning-effort probe (D-01): for each `--models` candidate of each configured
//! provider, calls over the raw vendor wire (not the adapters) that send every
//! `ReasoningEffort` level through each candidate wire form of that vendor, the off and auto
//! forms, one out-of-vocabulary value per form, and one `high` + `temperature: 0.5` call. Every
//! call carries an output cap of [`EFFORT_MAX_TOKENS`] tokens and the [`PROBE_TIMEOUT_SECS`] s
//! client timeout. Each printed row is numbers and a verdict only -- form, value, HTTP status,
//! finish reason, reasoning tokens, completion tokens, elapsed milliseconds -- and a rejected
//! call's body passes the redact-then-bound excerpt first. Response content is never printed.
//! Only the ids passed with `--models` are probed (no curated defaults), so the spend is exactly
//! the confirmed candidate set. `--effort --dry-run` prints the plan with no network access and no
//! credential read.
//!
//! # Effort smoke (`--effort-smoke`)
//!
//! The adapter-path companion of `--effort` (REASON-03): per `--models` candidate, two requests
//! through the shipped adapter (`*Config::from_env()`, `generate`) with
//! `LlmRequest::with_reasoning` at the lowest and the highest level the adapter's own
//! `model_profile(model).effort` admits, honouring optional `--max-tokens N` and
//! `--timeout-seconds S` on the config. A model whose profile admits no level sends nothing and
//! the row says why. Each row prints the outcome (`Ok`, or the `UnsupportedParameter` reason),
//! finish reason, reasoning tokens and the `paladin.reasoning_requested` /
//! `paladin.reasoning_effective` metadata values -- never response content.
//!
//! # Run commands
//!
//! ```text
//! cargo run -p paladin-llm --example live_vendor_smoke \
//!   --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible
//! ```
//!
//! Nothing here runs in CI: it needs real vendor credentials and reaches real vendor hosts.
//! Never prints a credential -- every error printed here is an `LlmError`, which the engine
//! has already routed through its redact-then-bound diagnostic excerpt.

use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::reasoning::{
    REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
};
use paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL;
use paladin_llm::anthropic::{AnthropicAdapter, AnthropicConfig};
use paladin_llm::deepseek::adapter::DEEPSEEK_DEFAULT_MODEL;
use paladin_llm::deepseek::{DeepSeekAdapter, DeepSeekConfig};
use paladin_llm::gemini::adapter::{
    GEMINI_DEFAULT_BASE_URL, GEMINI_DEFAULT_MODEL, GEMINI_FALLBACK_MODELS,
};
use paladin_llm::gemini::{GeminiAdapter, GeminiConfig};
use paladin_llm::grok::adapter::{GROK_DEFAULT_BASE_URL, GROK_DEFAULT_MODEL, GROK_FALLBACK_MODELS};
use paladin_llm::grok::{GrokAdapter, GrokConfig};
use paladin_llm::kimi::adapter::{KIMI_DEFAULT_BASE_URL, KIMI_DEFAULT_MODEL, KIMI_FALLBACK_MODELS};
use paladin_llm::kimi::{KimiAdapter, KimiConfig};
use paladin_llm::ollama::adapter::OLLAMA_PLACEHOLDER_API_KEY;
use paladin_llm::ollama::adapter::{
    OLLAMA_DEFAULT_BASE_URL, OLLAMA_DEFAULT_MODEL, OLLAMA_FALLBACK_MODELS,
};
use paladin_llm::ollama::{OllamaAdapter, OllamaConfig};
use paladin_llm::openai::adapter::{OPENAI_DEFAULT_BASE_URL, OPENAI_DEFAULT_MODEL};
use paladin_llm::openai::{OpenAIAdapter, OpenAIConfig};
use paladin_llm::openai_compatible::{OpenAiCompatibleAdapter, OpenAiCompatibleConfig};
use paladin_llm::qwen::adapter::{QWEN_DEFAULT_BASE_URL, QWEN_DEFAULT_MODEL, QWEN_FALLBACK_MODELS};
use paladin_llm::qwen::{QwenAdapter, QwenConfig};
use paladin_llm::redaction::diagnostic_excerpt;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, ReasoningEffort,
};
use paladin_ports::output::model_profile::{EffortSupport, ModelProfile};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

/// Minimal `log::Log` implementation, no dependency (17-22, G-17-4d).
///
/// With no logger installed, the `log` facade silently discards every record. Fixed at
/// `LevelFilter::Warn` rather than parsing `RUST_LOG`: hand-rolling a level-string parser with
/// no dependency would add complexity this harness does not need.
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

/// The request/response shape a provider speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wire {
    /// OpenAI `POST {base}/chat/completions`.
    OpenAiChat,
    /// Anthropic `POST {base}/messages`.
    AnthropicMessages,
    /// Gemini `POST {base}/models/{model}:generateContent`.
    GeminiGenerate,
}

impl Wire {
    /// Chat endpoint path template, shown in the run output.
    fn chat_path(self) -> &'static str {
        match self {
            Wire::OpenAiChat => "/chat/completions",
            Wire::AnthropicMessages => "/messages",
            Wire::GeminiGenerate => "/models/{model}:generateContent",
        }
    }
}

/// How a provider's credential travels. Always a header, never a URL query string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Auth {
    /// `Authorization: Bearer <key>`.
    Bearer,
    /// `x-api-key: <key>` plus `anthropic-version`.
    XApiKey,
    /// `x-goog-api-key: <key>`.
    GoogApiKey,
}

impl Auth {
    /// The header NAME only; a header value is never printed.
    fn header_name(self) -> &'static str {
        match self {
            Auth::Bearer => "Authorization",
            Auth::XApiKey => "x-api-key",
            Auth::GoogApiKey => "x-goog-api-key",
        }
    }
}

/// An adapter built from the environment, plus what the run needs to report about it.
struct Built {
    port: Box<dyn LlmPort>,
    /// The credential, held only to authenticate probe requests and to redact it from any body
    /// printed. Never printed, never `Debug`-formatted (this type does not derive `Debug`).
    api_key: String,
    /// The base URL the run ACTUALLY used, read back off the resolved config -- NOT the
    /// `*_DEFAULT_BASE_URL` constant. A `*_BASE_URL` override changes what goes on the wire.
    base_url: String,
    model: String,
}

/// One row of the provider table.
struct ProviderSpec {
    name: &'static str,
    /// Every variable that must be set for the provider to run. The first unset one is named in
    /// the `SKIPPED` line.
    cred_vars: &'static [&'static str],
    /// The shipped base URL, `None` when the operator must supply one (no defensible default).
    default_base_url: Option<&'static str>,
    default_model: &'static str,
    /// The curated list: the adapter's `*_FALLBACK_MODELS`, or its static model list.
    curated_models: &'static [&'static str],
    /// True when `get_available_models()` performs a live fetch that can be told apart from the
    /// curated list; false when the adapter returns a static list.
    live_model_list: bool,
    wire: Wire,
    auth: Auth,
    /// The adapter's declared `temperature_range`, `None` when the operator declares it.
    temperature_range: Option<(f32, f32)>,
    /// The output-cap field name on the OpenAI-chat wire (`max_completion_tokens` for OpenAI,
    /// whose newer models reject `max_tokens`). Unused on the other two wires, which have one.
    cap_field: &'static str,
    /// A request-body extra applied to every acceptance probe of this provider (A3-50-04:
    /// Moonshot `thinking` disabled, because the model's default reasoning ran 492 s on a
    /// one-line question). Printed beside every call it was used on.
    acceptance_extras: Option<&'static str>,
    build: fn() -> Result<Built, String>,
}

/// Anthropic's static model list (mirrors `AnthropicAdapter::get_available_models`, which returns
/// its seeded table, in table order). The adapter's default model comes first, built from its own
/// constant rather than restated; the other ids are the seeded rows added by Phase 50.1. The unit
/// test `the_static_model_lists_match_the_adapters` fails if the table and this list drift apart.
const ANTHROPIC_MODELS: &[&str] = &[
    ANTHROPIC_DEFAULT_MODEL,
    "claude-opus-4-8",
    "claude-fable-5-1",
    "claude-fable-5",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-sonnet-5-5",
    "claude-sonnet-5",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-sonnet-4-6",
];

/// DeepSeek's static model list (mirrors `DeepSeekAdapter::get_available_models`, which returns
/// its seeded table, in table order). Default model first, for the same reason as
/// [`ANTHROPIC_MODELS`], and guarded by the same unit test.
const DEEPSEEK_MODELS: &[&str] = &[DEEPSEEK_DEFAULT_MODEL, "deepseek-v4-pro"];

// The harness's OpenAI candidate is `OPENAI_DEFAULT_MODEL`, the adapter's own default (the
// first seeded OpenAI model, and the one whose rejection of the old `max_tokens` key went
// unnoticed while the plain smoke drove `gpt-4o-mini`, G-50-5b). Pass others with
// `--models openai=<id>`; `gpt-4o-mini` stays reachable that way.

/// Build an adapter whose config exposes `base_url` and `model` and whose `new` returns
/// `Result<_, LlmError>`.
macro_rules! build_fn {
    ($name:ident, $config:ty, $adapter:ty) => {
        fn $name() -> Result<Built, String> {
            let cfg = <$config>::from_env()?;
            let (api_key, base_url, model) =
                (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone());
            let adapter = <$adapter>::new(cfg).map_err(|e| e.to_string())?;
            Ok(Built {
                port: Box::new(adapter),
                api_key,
                base_url,
                model,
            })
        }
    };
}

build_fn!(build_anthropic, AnthropicConfig, AnthropicAdapter);
build_fn!(build_deepseek, DeepSeekConfig, DeepSeekAdapter);
build_fn!(build_kimi, KimiConfig, KimiAdapter);
build_fn!(build_qwen, QwenConfig, QwenAdapter);
build_fn!(build_grok, GrokConfig, GrokAdapter);
build_fn!(build_gemini, GeminiConfig, GeminiAdapter);
build_fn!(
    build_openai_compatible,
    OpenAiCompatibleConfig,
    OpenAiCompatibleAdapter
);

fn build_openai() -> Result<Built, String> {
    let cfg = OpenAIConfig::from_env()?;
    let (api_key, base_url) = (cfg.api_key.clone(), cfg.base_url.clone());
    let adapter = OpenAIAdapter::new(cfg)?;
    Ok(Built {
        port: Box::new(adapter),
        api_key,
        base_url,
        model: OPENAI_DEFAULT_MODEL.to_string(),
    })
}

fn build_ollama() -> Result<Built, String> {
    let cfg = OllamaConfig::from_env()?;
    let (base_url, model) = (cfg.base_url.clone(), cfg.model.clone());
    let adapter = OllamaAdapter::new(cfg).map_err(|e| e.to_string())?;
    // A local daemon has no key; the adapter itself sends this fixed placeholder.
    Ok(Built {
        port: Box::new(adapter),
        api_key: OLLAMA_PLACEHOLDER_API_KEY.to_string(),
        base_url,
        model,
    })
}

/// The nine providers. Order is the order of the run output.
const PROVIDERS: [ProviderSpec; 9] = [
    ProviderSpec {
        name: "OpenAI",
        cred_vars: &["OPENAI_API_KEY"],
        default_base_url: Some(OPENAI_DEFAULT_BASE_URL),
        default_model: OPENAI_DEFAULT_MODEL,
        curated_models: &[],
        live_model_list: true,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        temperature_range: Some((0.0, 1.0)),
        cap_field: "max_completion_tokens",
        acceptance_extras: None,
        build: build_openai,
    },
    ProviderSpec {
        name: "Anthropic",
        cred_vars: &["ANTHROPIC_API_KEY"],
        default_base_url: Some("https://api.anthropic.com/v1"),
        default_model: ANTHROPIC_DEFAULT_MODEL,
        curated_models: ANTHROPIC_MODELS,
        live_model_list: false,
        wire: Wire::AnthropicMessages,
        auth: Auth::XApiKey,
        temperature_range: Some((0.0, 1.0)),
        cap_field: "max_tokens",
        acceptance_extras: None,
        build: build_anthropic,
    },
    ProviderSpec {
        name: "DeepSeek",
        cred_vars: &["DEEPSEEK_API_KEY"],
        default_base_url: Some("https://api.deepseek.com/v1"),
        default_model: DEEPSEEK_DEFAULT_MODEL,
        curated_models: DEEPSEEK_MODELS,
        live_model_list: false,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        temperature_range: Some((0.0, 2.0)),
        cap_field: "max_tokens",
        acceptance_extras: None,
        build: build_deepseek,
    },
    ProviderSpec {
        name: "Kimi",
        cred_vars: &["MOONSHOT_API_KEY"],
        default_base_url: Some(KIMI_DEFAULT_BASE_URL),
        default_model: KIMI_DEFAULT_MODEL,
        curated_models: KIMI_FALLBACK_MODELS,
        live_model_list: true,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        // Mirrors the adapter: ADR-0004's framework default span, no vendor claim. The next
        // probe measures the span's ends and middle (thinking on and off).
        temperature_range: Some((0.0, 1.0)),
        cap_field: "max_tokens",
        acceptance_extras: Some(r#"{"thinking":{"type":"disabled"}}"#),
        build: build_kimi,
    },
    ProviderSpec {
        name: "Qwen",
        cred_vars: &["DASHSCOPE_API_KEY"],
        default_base_url: Some(QWEN_DEFAULT_BASE_URL),
        default_model: QWEN_DEFAULT_MODEL,
        curated_models: QWEN_FALLBACK_MODELS,
        live_model_list: true,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        temperature_range: Some((0.0, 1.99)),
        cap_field: "max_tokens",
        acceptance_extras: None,
        build: build_qwen,
    },
    ProviderSpec {
        name: "Grok",
        cred_vars: &["XAI_API_KEY"],
        default_base_url: Some(GROK_DEFAULT_BASE_URL),
        default_model: GROK_DEFAULT_MODEL,
        curated_models: GROK_FALLBACK_MODELS,
        live_model_list: true,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        temperature_range: Some((0.0, 2.0)),
        cap_field: "max_tokens",
        acceptance_extras: None,
        build: build_grok,
    },
    ProviderSpec {
        name: "Gemini",
        cred_vars: &["GEMINI_API_KEY"],
        default_base_url: Some(GEMINI_DEFAULT_BASE_URL),
        default_model: GEMINI_DEFAULT_MODEL,
        curated_models: GEMINI_FALLBACK_MODELS,
        live_model_list: true,
        wire: Wire::GeminiGenerate,
        auth: Auth::GoogApiKey,
        temperature_range: Some((0.0, 2.0)),
        cap_field: "max_tokens",
        acceptance_extras: None,
        build: build_gemini,
    },
    ProviderSpec {
        name: "Ollama",
        cred_vars: &["OLLAMA_BASE_URL"],
        default_base_url: Some(OLLAMA_DEFAULT_BASE_URL),
        default_model: OLLAMA_DEFAULT_MODEL,
        curated_models: OLLAMA_FALLBACK_MODELS,
        live_model_list: true,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        temperature_range: Some((0.0, 2.0)),
        cap_field: "max_tokens",
        acceptance_extras: None,
        build: build_ollama,
    },
    ProviderSpec {
        name: "OpenAI-compatible",
        cred_vars: &["OPENAI_COMPATIBLE_API_KEY", "OPENAI_COMPATIBLE_BASE_URL"],
        default_base_url: None,
        default_model: "<OPENAI_COMPATIBLE_MODEL>",
        curated_models: &[],
        live_model_list: true,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        temperature_range: None,
        cap_field: "max_tokens",
        acceptance_extras: None,
        build: build_openai_compatible,
    },
];

/// The first of `spec`'s credential variables that `lookup` reports unset or empty, or `None`
/// when the provider is configured.
fn missing_credential(
    spec: &ProviderSpec,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<&'static str> {
    spec.cred_vars
        .iter()
        .copied()
        .find(|var| lookup(var).is_none_or(|v| v.trim().is_empty()))
}

/// Process exit code for a smoke run: non-zero when nothing was configured (the vacuous-pass
/// refusal) or when any configured provider failed.
fn smoke_exit_code(configured: usize, failures: usize) -> i32 {
    if configured == 0 || failures > 0 {
        1
    } else {
        0
    }
}

/// Outcome of the model-list half of a vendor probe.
struct Live {
    models: Vec<String>,
    /// True when the returned list is byte-identical to the curated fallback, i.e. the live
    /// fetch silently failed.
    is_fallback: bool,
    default_present: bool,
}

fn classify(models: Vec<String>, fallback: &[&str], default_model: &str) -> Live {
    let is_fallback = !fallback.is_empty()
        && models.len() == fallback.len()
        && models.iter().zip(fallback.iter()).all(|(m, f)| m == f);
    let default_present = models.iter().any(|m| m == default_model);
    Live {
        models,
        is_fallback,
        default_present,
    }
}

/// Outcome of the `generate()` half of a vendor probe.
struct GenerateOutcome {
    content_len: usize,
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Call `generate()` on `port` with the framework's DEFAULT prompt parameters -- deliberately
/// built via `PromptItem::new(...)` rather than any explicit parameter override, so this probe
/// carries exactly the configuration `17-UAT.md` proved fails for Grok.
async fn probe_generate(port: &dyn LlmPort, model: &str) -> Result<GenerateOutcome, String> {
    let prompt = PromptItem::new(PromptType::User(UserPrompt {
        query: "ping".to_string(),
        context: None,
    }))
    .map_err(|e| format!("failed to construct prompt: {e}"))?;

    let request = LlmRequest::new(model.to_string(), prompt);

    match port.generate(request).await {
        Ok(response) => {
            if response.content.trim().is_empty() {
                return Err(
                    "generate() returned Ok with empty content -- vacuous pass refused".to_string(),
                );
            }
            if response.usage.total_tokens == 0 {
                return Err(format!(
                    "generate() returned Ok with zero total tokens (content {} chars) -- \
                     vacuous pass refused",
                    response.content.len()
                ));
            }
            Ok(GenerateOutcome {
                content_len: response.content.len(),
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Result of the model-list probe for one provider.
enum ListResult {
    Pass(Live),
    Fail(String),
    /// The adapter has no live fetch to test. Reported, never counted as a pass.
    NotApplicable(&'static str),
}

/// Everything one configured provider's smoke run produced.
struct SmokeRun {
    base_url: String,
    model: String,
    list: ListResult,
    generate: Result<GenerateOutcome, String>,
}

async fn smoke_provider(spec: &ProviderSpec) -> Result<SmokeRun, String> {
    let built = (spec.build)()?;
    let list = if spec.live_model_list {
        match built.port.get_available_models().await {
            Err(e) => ListResult::Fail(format!("get_available_models failed: {e}")),
            Ok(models) => {
                let live = classify(models, spec.curated_models, &built.model);
                if live.models.is_empty() {
                    ListResult::Fail("get_available_models returned an empty list".to_string())
                } else if live.is_fallback {
                    ListResult::Fail(
                        "result is byte-identical to the curated fallback -- the live-fetch \
                         path was not exercised"
                            .to_string(),
                    )
                } else if !live.default_present {
                    ListResult::Fail(format!(
                        "default model `{}` is absent from the live list",
                        built.model
                    ))
                } else {
                    ListResult::Pass(live)
                }
            }
        }
    } else {
        ListResult::NotApplicable("the adapter returns a static list; there is no live fetch")
    };
    let generate = probe_generate(built.port.as_ref(), &built.model).await;
    Ok(SmokeRun {
        base_url: built.base_url,
        model: built.model,
        list,
        generate,
    })
}

/// Parsed options.
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    /// `--probe`: measure what each vendor's models accept and report.
    probe: bool,
    /// `--usage`: send one reasoning-eliciting call per candidate model and record the usage
    /// arithmetic (numbers only).
    usage: bool,
    /// `--effort`: the Phase 51 raw-wire reasoning-effort probe (numbers and verdicts only).
    effort: bool,
    /// `--effort-smoke`: the adapter-path effort smoke (mutually exclusive with the other modes).
    effort_smoke: bool,
    /// `--max-tokens <n>` (`--effort-smoke` only): the adapter config's output budget.
    max_tokens: Option<u32>,
    /// `--timeout-seconds <s>` (`--effort-smoke` only): the adapter config's request timeout.
    timeout_seconds: Option<u64>,
    /// `--dry-run`: print the probe plan, touching no network and reading no credential.
    dry_run: bool,
    /// `--models <provider>=<id>[,<id>...]` (repeatable): extra candidate models, keyed by the
    /// table's canonical provider name.
    models: Vec<(String, Vec<String>)>,
}

/// Parsed command line.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Run(Options),
}

const USAGE: &str = "usage: live_vendor_smoke [--probe | --usage | --effort | --effort-smoke] [--dry-run] [--models <provider>=<id>[,<id>...]]... [--max-tokens <n>] [--timeout-seconds <s>] [-h|--help]\n\
\n\
  (no flag)   credential-gated smoke probes over all nine providers\n\
  --probe     record, per configured provider, the live models listing and what each candidate\n\
              model accepts (sampling, stop sequences, output cap, JSON-object and JSON-schema\n\
              modes, usage fields, quota-shaped errors); prints a markdown transcript\n\
  --usage     send one reasoning-eliciting call per candidate model and record the usage\n\
              arithmetic, numbers only, with a containment verdict per model; excludes --probe\n\
  --effort    send, per --models candidate, the default call, every reasoning-effort level\n\
              through each candidate wire form of its vendor, the off and auto forms, one\n\
              invalid value per form and one high + temperature call, over the raw wire;\n\
              numbers and verdicts only (never response content), every call capped at 2048\n\
              output tokens; probes only the --models ids; excludes --probe and --usage\n\
  --effort-smoke\n\
              send, per --models candidate, two requests through the shipped adapter\n\
              (*Config::from_env(), generate) with LlmRequest::with_reasoning at the lowest and\n\
              the highest level the adapter's own model_profile admits; prints the outcome, finish\n\
              reason, reasoning tokens and the paladin.reasoning_* metadata values, never content;\n\
              excludes --probe, --usage and --effort\n\
  --max-tokens <n>\n\
              with --effort-smoke: the adapter config's output budget (optional)\n\
  --timeout-seconds <s>\n\
              with --effort-smoke: the adapter config's request timeout (optional)\n\
  --dry-run   print the probe (or, with --usage, --effort or --effort-smoke, that mode's) plan for\n\
              all nine providers; no network access, no credential read\n\
  --models    extra candidate model ids for one provider (repeatable); needs --probe, --usage,\n\
              --effort, --effort-smoke or --dry-run\n\
\n\
providers: OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini, Ollama, OpenAI-compatible";

fn add_models(options: &mut Options, value: &str) -> Result<(), String> {
    let (provider, ids) = value
        .split_once('=')
        .ok_or_else(|| format!("`--models {value}` is not <provider>=<id>[,<id>...]"))?;
    let spec = PROVIDERS
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(provider))
        .ok_or_else(|| format!("`--models` names an unknown provider `{provider}`"))?;
    let mut parsed = Vec::new();
    for id in ids.split(',') {
        if !valid_model_id(id) {
            return Err(format!(
                "`--models` for {}: `{id}` is not a plausible model id",
                spec.name
            ));
        }
        parsed.push(id.to_string());
    }
    options.models.push((spec.name.to_string(), parsed));
    Ok(())
}

/// A positive integer flag value; anything else is an error naming the flag.
fn positive<T: std::str::FromStr + PartialEq + Default>(
    flag: &str,
    value: &str,
) -> Result<T, String> {
    value
        .parse::<T>()
        .ok()
        .filter(|n| *n != T::default())
        .ok_or_else(|| format!("`{flag}` needs a positive integer, got `{value}`"))
}

/// Parse the argument list (without the program name). Any argument the harness does not know is
/// an error naming it -- never a silent smoke run.
fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut options = Options::default();
    let mut help = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--models=") {
            add_models(&mut options, value)?;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--max-tokens=") {
            options.max_tokens = Some(positive("--max-tokens", value)?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--timeout-seconds=") {
            options.timeout_seconds = Some(positive("--timeout-seconds", value)?);
            continue;
        }
        match arg.as_str() {
            "-h" | "--help" => help = true,
            "--probe" => options.probe = true,
            "--usage" => options.usage = true,
            "--effort" => options.effort = true,
            "--effort-smoke" => options.effort_smoke = true,
            "--dry-run" => options.dry_run = true,
            "--models" => {
                let value = args.next().ok_or_else(|| {
                    "`--models` needs a value of the form <provider>=<id>[,<id>...]".to_string()
                })?;
                add_models(&mut options, &value)?;
            }
            "--max-tokens" => {
                let value = args
                    .next()
                    .ok_or_else(|| "`--max-tokens` needs a positive integer".to_string())?;
                options.max_tokens = Some(positive("--max-tokens", &value)?);
            }
            "--timeout-seconds" => {
                let value = args
                    .next()
                    .ok_or_else(|| "`--timeout-seconds` needs a positive integer".to_string())?;
                options.timeout_seconds = Some(positive("--timeout-seconds", &value)?);
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if help {
        return Ok(Command::Help);
    }
    let modes: Vec<&str> = [
        (options.probe, "--probe"),
        (options.usage, "--usage"),
        (options.effort, "--effort"),
        (options.effort_smoke, "--effort-smoke"),
    ]
    .into_iter()
    .filter_map(|(set, flag)| set.then_some(flag))
    .collect();
    if let [first, second, ..] = modes.as_slice() {
        return Err(format!("`{first}` and `{second}` are mutually exclusive"));
    }
    for (set, flag) in [
        (options.max_tokens.is_some(), "--max-tokens"),
        (options.timeout_seconds.is_some(), "--timeout-seconds"),
    ] {
        if set && !options.effort_smoke {
            return Err(format!("`{flag}` has no effect without --effort-smoke"));
        }
    }
    if !options.models.is_empty() && modes.is_empty() && !options.dry_run {
        return Err(
            "`--models` has no effect without --probe, --usage, --effort, --effort-smoke or \
             --dry-run"
                .to_string(),
        );
    }
    Ok(Command::Run(options))
}

fn print_header(spec: &ProviderSpec) {
    println!("\n=== {} ===", spec.name);
}

/// Print one configured provider's smoke result. Returns the number of failed probes and the
/// number of passed probes.
fn report_smoke(spec: &ProviderSpec, run: &SmokeRun) -> (usize, usize) {
    let (mut passed, mut failed) = (0usize, 0usize);
    println!(
        "  base_url      : {}{}",
        run.base_url,
        match spec.default_base_url {
            Some(d) if d != run.base_url => {
                format!("\n                  [OVERRIDE -- shipped default is {d}]")
            }
            _ => String::new(),
        }
    );
    println!(
        "  default model : {}{}",
        run.model,
        if run.model != spec.default_model && !spec.default_model.starts_with('<') {
            format!(
                "\n                  [OVERRIDE -- shipped default is {}]",
                spec.default_model
            )
        } else {
            String::new()
        }
    );
    println!(
        "  wire          : {} ({}), auth header {}",
        match spec.wire {
            Wire::OpenAiChat => "OpenAI chat",
            Wire::AnthropicMessages => "Anthropic messages",
            Wire::GeminiGenerate => "Gemini generateContent",
        },
        spec.wire.chat_path(),
        spec.auth.header_name()
    );
    match spec.temperature_range {
        Some((lo, hi)) => println!("  temperature   : declared range {lo}..{hi}"),
        None => println!("  temperature   : no declared range (operator-declared)"),
    }

    println!("  -- model list probe --");
    match &run.list {
        ListResult::Fail(e) => {
            println!("  RESULT        : FAIL -- {e}");
            failed += 1;
        }
        ListResult::NotApplicable(why) => {
            println!("  RESULT        : NOT APPLICABLE -- {why} (not counted as a pass)");
        }
        ListResult::Pass(live) => {
            println!("  models returned: {}", live.models.len());
            println!("  live fetch    : YES -- differs from the curated fallback");
            let mut sample: Vec<&str> = live.models.iter().map(String::as_str).collect();
            sample.sort_unstable();
            let shown: Vec<&str> = sample.iter().take(8).copied().collect();
            println!(
                "  sample        : {}{}",
                shown.join(", "),
                if sample.len() > 8 {
                    format!(", ... (+{} more)", sample.len() - 8)
                } else {
                    String::new()
                }
            );
            println!("  default model in live list: YES");
            println!("  RESULT        : PASS");
            passed += 1;
        }
    }

    println!("  -- generate() probe (default prompt parameters) --");
    match &run.generate {
        Err(e) => {
            println!("  RESULT        : FAIL -- {e}");
            failed += 1;
        }
        Ok(g) => {
            println!(
                "  content       : {} chars; tokens prompt={} completion={} total={}",
                g.content_len, g.prompt_tokens, g.completion_tokens, g.total_tokens
            );
            println!("  RESULT        : PASS");
            passed += 1;
        }
    }
    (passed, failed)
}

async fn run_smoke() -> i32 {
    let lookup = |name: &str| std::env::var(name).ok();
    let (mut configured, mut skipped) = (0usize, 0usize);
    let (mut passed, mut failed) = (0usize, 0usize);

    for spec in PROVIDERS.iter() {
        print_header(spec);
        if let Some(var) = missing_credential(spec, &lookup) {
            println!("  SKIPPED ({var} not set)");
            skipped += 1;
            continue;
        }
        configured += 1;
        match smoke_provider(spec).await {
            Err(e) => {
                println!("  RESULT        : FAIL -- adapter could not be constructed: {e}");
                failed += 1;
            }
            Ok(run) => {
                let (p, f) = report_smoke(spec, &run);
                passed += p;
                failed += f;
            }
        }
    }

    println!("\n------------------------------------------");
    println!(
        "{configured} of {} providers configured ({skipped} not configured); \
         {passed} probes passed, {failed} failed",
        PROVIDERS.len()
    );
    if configured == 0 {
        eprintln!("no provider was configured -- nothing was tested");
    }
    smoke_exit_code(configured, failed)
}

// ---------------------------------------------------------------------------------------------
// Probe mode (`--probe`) and its offline plan (`--dry-run`)
// ---------------------------------------------------------------------------------------------

/// Per-call client timeout (A3-50-04).
const PROBE_TIMEOUT_SECS: u64 = 120;
/// Output cap carried by every acceptance probe (A3-50-04): bounds cost and reasoning time.
const PROBE_MAX_TOKENS: u64 = 256;
const PROBE_PROMPT: &str = "Reply with the single word: ping";
/// Output cap of a `--usage` call (A1-50-22): room for a reasoning model to think and answer,
/// bounded so a long default reasoning (Moonshot) cannot run away on the operator's bill.
const USAGE_MAX_TOKENS: u64 = 1024;
/// The `--usage` prompt: a small multi-step calculation a reasoning model reasons about.
const USAGE_PROMPT: &str =
    "What is 17 multiplied by 23? Work it out, then answer with the number only.";
/// Output cap of an `--effort` call (research Pitfall 15): bounds the spend of a `max` effort on
/// the operator's bill. A `length` finish is recorded as data, never an error.
const EFFORT_MAX_TOKENS: u64 = 2048;
/// The out-of-vocabulary effort value every form is probed with: a vendor that parses the field
/// rejects it, one that does not answers 2xx (research A4).
const EFFORT_INVALID_VALUE: &str = "bogus-effort-51";
/// The JSON-mode probes name JSON in the prompt: OpenAI-chat `json_object` mode refuses a prompt
/// that does not, and that refusal would be a prompt artefact rather than a capability finding.
const PROBE_JSON_PROMPT: &str = "Reply with a JSON object of the form {\"answer\": \"ping\"}.";

/// One call the probe makes per candidate model.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ProbeKind {
    /// Default sampling parameters (plus the provider's acceptance extras).
    Default,
    /// Moonshot only: the model's default thinking, without the `thinking: disabled` extra.
    DefaultThinking,
    Temperature(f64),
    TopP,
    FrequencyPenalty,
    PresencePenalty,
    StopOne,
    StopFive,
    MaxTokens,
    /// OpenAI only: the field newer models require in place of `max_tokens`.
    MaxCompletionTokens,
    JsonObject,
    JsonSchema,
    /// `--usage` only: the reasoning-eliciting arithmetic prompt under [`USAGE_MAX_TOKENS`], with
    /// no body extras and no sampling parameters.
    Usage,
    /// `--effort` only: the same prompt as [`ProbeKind::Usage`] under [`EFFORT_MAX_TOKENS`], with
    /// no acceptance extras; the effort fields are merged in by the caller.
    Effort,
}

/// A probe in the ordered per-model list.
struct PlannedProbe {
    kind: ProbeKind,
    label: String,
    /// Why the probe cannot be made on this provider's wire; such a probe is recorded, never run.
    not_applicable: Option<&'static str>,
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// The four temperatures probed: the low end, the high end, `0.5`, and the next value just outside
/// the high end, so a seeded `Range` endpoint is measured rather than copied (ADR-0004 A4: Qwen's
/// half-open `[0, 2)`). Points that coincide (Kimi's fixed `1.0`) are merged, keeping every role.
fn temperature_points(range: Option<(f32, f32)>) -> Vec<(f64, String)> {
    let (lo, hi, adjective) = match range {
        Some((lo, hi)) => (lo, hi, "declared"),
        None => (0.0, 2.0, "assumed"),
    };
    let (lo, hi) = (round2(f64::from(lo)), round2(f64::from(hi)));
    let raw = [
        (lo, format!("{adjective} low end")),
        (hi, format!("{adjective} high end")),
        (0.5, "mid-range".to_string()),
        (round2(hi + 0.01), "just outside the high end".to_string()),
    ];
    let mut points: Vec<(f64, String)> = Vec::new();
    for (value, role) in raw {
        match points.iter_mut().find(|(p, _)| (*p - value).abs() < 1e-9) {
            Some((_, roles)) => {
                roles.push_str(" = ");
                roles.push_str(&role);
            }
            None => points.push((value, role)),
        }
    }
    points
}

/// The ordered probe list for `spec`, identical for every candidate model.
fn probe_plan(spec: &ProviderSpec) -> Vec<PlannedProbe> {
    let wire = spec.wire;
    let mut plan = vec![PlannedProbe {
        kind: ProbeKind::Default,
        label: "default parameters".to_string(),
        not_applicable: None,
    }];
    if spec.acceptance_extras.is_some() {
        plan.push(PlannedProbe {
            kind: ProbeKind::DefaultThinking,
            label: "default thinking (the thinking body extra is NOT applied)".to_string(),
            not_applicable: None,
        });
    }
    for (value, roles) in temperature_points(spec.temperature_range) {
        plan.push(PlannedProbe {
            kind: ProbeKind::Temperature(value),
            label: format!("temperature={value:.2} ({roles})"),
            not_applicable: None,
        });
    }
    let penalty_na = (wire != Wire::OpenAiChat).then_some("this wire has no such request field");
    plan.push(PlannedProbe {
        kind: ProbeKind::TopP,
        label: "top_p=0.5".to_string(),
        not_applicable: None,
    });
    plan.push(PlannedProbe {
        kind: ProbeKind::FrequencyPenalty,
        label: "frequency_penalty=0.5".to_string(),
        not_applicable: penalty_na,
    });
    plan.push(PlannedProbe {
        kind: ProbeKind::PresencePenalty,
        label: "presence_penalty=0.5".to_string(),
        not_applicable: penalty_na,
    });
    plan.push(PlannedProbe {
        kind: ProbeKind::StopOne,
        label: "stop sequences: 1".to_string(),
        not_applicable: None,
    });
    plan.push(PlannedProbe {
        kind: ProbeKind::StopFive,
        label: "stop sequences: 5".to_string(),
        not_applicable: None,
    });
    plan.push(PlannedProbe {
        kind: ProbeKind::MaxTokens,
        label: "max_tokens=64".to_string(),
        not_applicable: None,
    });
    if wire == Wire::OpenAiChat && spec.cap_field == "max_completion_tokens" {
        plan.push(PlannedProbe {
            kind: ProbeKind::MaxCompletionTokens,
            label: "max_completion_tokens=64".to_string(),
            not_applicable: None,
        });
    }
    let json_na = (wire == Wire::AnthropicMessages).then_some("Anthropic has no JSON mode");
    let (object_field, schema_field) = match wire {
        Wire::GeminiGenerate => (
            "generationConfig.responseMimeType",
            "generationConfig.responseMimeType + responseSchema",
        ),
        _ => ("response_format json_object", "response_format json_schema"),
    };
    plan.push(PlannedProbe {
        kind: ProbeKind::JsonObject,
        label: format!("json_object mode ({object_field}; prompt names JSON)"),
        not_applicable: json_na,
    });
    plan.push(PlannedProbe {
        kind: ProbeKind::JsonSchema,
        label: format!("json_schema mode ({schema_field}; prompt names JSON)"),
        not_applicable: json_na,
    });
    plan
}

/// Set `value` at `path`, creating intermediate objects. A malformed body is left untouched
/// rather than panicking.
fn set_path(body: &mut Value, path: &[&str], value: Value) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut cursor = body;
    for key in parents {
        let Some(map) = cursor.as_object_mut() else {
            return;
        };
        cursor = map
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(Default::default()));
    }
    if let Some(map) = cursor.as_object_mut() {
        map.insert((*last).to_string(), value);
    }
}

fn remove_field(body: &mut Value, key: &str) {
    if let Some(map) = body.as_object_mut() {
        map.remove(key);
    }
}

/// The body extras applied to a probe of `kind`, if any. `DefaultThinking` is the one call made
/// without them.
fn extras_for(spec: &ProviderSpec, kind: ProbeKind) -> Option<&'static str> {
    // `--usage` sends no extras: Kimi's `thinking: disabled` would suppress the very tokens
    // being measured (A2-50-22).
    if matches!(
        kind,
        ProbeKind::DefaultThinking | ProbeKind::Usage | ProbeKind::Effort
    ) {
        None
    } else {
        spec.acceptance_extras
    }
}

/// The JSON body for one probe call. Every probe is bounded by [`PROBE_MAX_TOKENS`] unless it is
/// the probe of the cap itself, and varies exactly one thing from the baseline.
fn request_body(spec: &ProviderSpec, model: &str, kind: ProbeKind) -> Value {
    let prompt = match kind {
        ProbeKind::JsonObject | ProbeKind::JsonSchema => PROBE_JSON_PROMPT,
        ProbeKind::Usage | ProbeKind::Effort => USAGE_PROMPT,
        _ => PROBE_PROMPT,
    };
    let cap = match kind {
        ProbeKind::Usage => USAGE_MAX_TOKENS,
        ProbeKind::Effort => EFFORT_MAX_TOKENS,
        _ => PROBE_MAX_TOKENS,
    };
    let wire = spec.wire;
    let mut body = match wire {
        Wire::OpenAiChat => {
            let mut b = json!({
                "model": model,
                "messages": [{"role": "user", "content": prompt}],
            });
            set_path(&mut b, &[spec.cap_field], json!(cap));
            b
        }
        Wire::AnthropicMessages => json!({
            "model": model,
            "max_tokens": cap,
            "messages": [{"role": "user", "content": prompt}],
        }),
        Wire::GeminiGenerate => json!({
            "contents": [{"role": "user", "parts": [{"text": prompt}]}],
            "generationConfig": {"maxOutputTokens": cap},
        }),
    };
    let (temperature, top_p, stop, max_tokens): (&[&str], &[&str], &[&str], &[&str]) = match wire {
        Wire::OpenAiChat => (&["temperature"], &["top_p"], &["stop"], &["max_tokens"]),
        Wire::AnthropicMessages => (
            &["temperature"],
            &["top_p"],
            &["stop_sequences"],
            &["max_tokens"],
        ),
        Wire::GeminiGenerate => (
            &["generationConfig", "temperature"],
            &["generationConfig", "topP"],
            &["generationConfig", "stopSequences"],
            &["generationConfig", "maxOutputTokens"],
        ),
    };
    match kind {
        ProbeKind::Default | ProbeKind::DefaultThinking | ProbeKind::Usage | ProbeKind::Effort => {}
        ProbeKind::Temperature(value) => set_path(&mut body, temperature, json!(value)),
        ProbeKind::TopP => set_path(&mut body, top_p, json!(0.5)),
        ProbeKind::FrequencyPenalty => set_path(&mut body, &["frequency_penalty"], json!(0.5)),
        ProbeKind::PresencePenalty => set_path(&mut body, &["presence_penalty"], json!(0.5)),
        ProbeKind::StopOne => set_path(&mut body, stop, json!(["END"])),
        ProbeKind::StopFive => set_path(
            &mut body,
            stop,
            json!(["END1", "END2", "END3", "END4", "END5"]),
        ),
        ProbeKind::MaxTokens => {
            if wire == Wire::OpenAiChat {
                remove_field(&mut body, spec.cap_field);
            }
            set_path(&mut body, max_tokens, json!(64));
        }
        ProbeKind::MaxCompletionTokens => {
            set_path(&mut body, &["max_completion_tokens"], json!(64));
        }
        ProbeKind::JsonObject => match wire {
            Wire::OpenAiChat => {
                set_path(
                    &mut body,
                    &["response_format"],
                    json!({"type": "json_object"}),
                );
            }
            Wire::GeminiGenerate => set_path(
                &mut body,
                &["generationConfig", "responseMimeType"],
                json!("application/json"),
            ),
            Wire::AnthropicMessages => {}
        },
        ProbeKind::JsonSchema => match wire {
            Wire::OpenAiChat => set_path(
                &mut body,
                &["response_format"],
                json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": "probe_answer",
                        "strict": true,
                        "schema": {
                            "type": "object",
                            "properties": {"answer": {"type": "string"}},
                            "required": ["answer"],
                            "additionalProperties": false,
                        },
                    },
                }),
            ),
            Wire::GeminiGenerate => {
                set_path(
                    &mut body,
                    &["generationConfig", "responseMimeType"],
                    json!("application/json"),
                );
                set_path(
                    &mut body,
                    &["generationConfig", "responseSchema"],
                    json!({
                        "type": "OBJECT",
                        "properties": {"answer": {"type": "STRING"}},
                        "required": ["answer"],
                    }),
                );
            }
            Wire::AnthropicMessages => {}
        },
    }
    if let Some(extras) = extras_for(spec, kind)
        && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(extras)
        && let Some(object) = body.as_object_mut()
    {
        object.extend(map);
    }
    body
}

/// A URL safe to print: userinfo and any credential-looking query value are replaced by
/// `<redacted>`, and a fragment is dropped (T-50-04-03).
fn redact_url(url: &str) -> String {
    let (scheme, rest) = match url.split_once("://") {
        Some((scheme, rest)) => (Some(scheme), rest),
        None => (None, url),
    };
    let rest = rest.split('#').next().unwrap_or(rest);
    let (location, query) = match rest.split_once('?') {
        Some((location, query)) => (location, Some(query)),
        None => (rest, None),
    };
    let (authority, path) = match location.find('/') {
        Some(i) => location.split_at(i),
        None => (location, ""),
    };
    let authority = match authority.rsplit_once('@') {
        Some((_, host)) => format!("<redacted>@{host}"),
        None => authority.to_string(),
    };
    let mut out = String::new();
    if let Some(scheme) = scheme {
        out.push_str(scheme);
        out.push_str("://");
    }
    out.push_str(&authority);
    out.push_str(path);
    if let Some(query) = query {
        let params: Vec<String> = query
            .split('&')
            .map(|param| match param.split_once('=') {
                Some((name, _))
                    if ["key", "token", "secret", "auth", "sig", "password"]
                        .iter()
                        .any(|marker| name.to_ascii_lowercase().contains(marker)) =>
                {
                    format!("{name}=<redacted>")
                }
                _ => param.to_string(),
            })
            .collect();
        out.push('?');
        out.push_str(&params.join("&"));
    }
    out
}

fn trim_base(base_url: &str) -> &str {
    base_url.trim_end_matches('/')
}

fn models_url(wire: Wire, base_url: &str) -> String {
    match wire {
        // Gemini pages its catalogue; the page size is the only query parameter, and the
        // credential travels in a header, never the URL.
        Wire::GeminiGenerate => format!("{}/models?pageSize=1000", trim_base(base_url)),
        _ => format!("{}/models", trim_base(base_url)),
    }
}

fn chat_url(wire: Wire, base_url: &str, model: &str) -> String {
    let base = trim_base(base_url);
    match wire {
        Wire::OpenAiChat => format!("{base}/chat/completions"),
        Wire::AnthropicMessages => format!("{base}/messages"),
        Wire::GeminiGenerate => format!("{base}/models/{model}:generateContent"),
    }
}

/// Make a markdown-table-safe cell from already-redacted text.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace(['\n', '\r'], " ")
}

/// A vendor body (or any vendor-supplied text) as a printable cell: redacted first, then bounded
/// (`diagnostic_excerpt`), then made table-safe. The order is load-bearing -- bounding first can
/// slice a secret across the truncation boundary and leak the tail.
fn excerpt(body: &str, api_key: &str) -> String {
    cell(&diagnostic_excerpt(body, api_key))
}

/// Characters a model id may contain; keeps an operator-supplied id from reshaping a URL path.
fn valid_model_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '/'))
}

/// Candidate models for a provider: its default model, its curated list, then any ids passed with
/// `--models`, without duplicates.
fn candidates(spec: &ProviderSpec, default_model: &str, extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let all = std::iter::once(default_model.to_string())
        .chain(spec.curated_models.iter().map(|m| m.to_string()))
        .chain(extra.iter().cloned());
    for model in all {
        if !out.contains(&model) {
            out.push(model);
        }
    }
    out
}

/// What the probe recorded from one call.
#[derive(Default)]
struct Parsed {
    finish: Option<String>,
    has_content: bool,
    usage_fields: Vec<String>,
    has_reasoning: bool,
    note: Option<String>,
    /// The numeric usage values (`--usage`); every field is an integer or absent.
    /// Boxed so the `Outcome::Accepted` variant stays small (`clippy::large_enum_variant`).
    usage: Box<UsageRow>,
}

enum Outcome {
    Accepted {
        status: u16,
        parsed: Parsed,
    },
    Rejected {
        status: u16,
        detail: String,
    },
    /// No HTTP status was received (connect, TLS or timeout failure).
    Unreachable(String),
    NotApplicable(&'static str),
}

fn non_blank_text(value: &Value) -> bool {
    match value {
        Value::String(s) => !s.trim().is_empty(),
        Value::Array(items) => items
            .iter()
            .any(|i| i.get("text").is_some_and(non_blank_text)),
        _ => false,
    }
}

fn non_empty_value(value: &Value) -> bool {
    match value {
        Value::String(s) => !s.trim().is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => false,
    }
}

/// The dotted names of every leaf field of a usage object (names only -- never values), so the
/// operator can see which of cached, cache-write and reasoning token counts a vendor reports.
fn flatten_field_names(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                if inner.is_object() {
                    flatten_field_names(inner, &path, out);
                } else {
                    out.push(path);
                }
            }
        }
        _ => {
            if !prefix.is_empty() {
                out.push(prefix.to_string());
            }
        }
    }
}

fn parse_success(wire: Wire, body: &str, api_key: &str) -> Parsed {
    let Ok(v) = serde_json::from_str::<Value>(body) else {
        return Parsed {
            note: Some(format!("body is not JSON: {}", excerpt(body, api_key))),
            ..Parsed::default()
        };
    };
    let mut usage_fields = Vec::new();
    let usage_key = if wire == Wire::GeminiGenerate {
        "usageMetadata"
    } else {
        "usage"
    };
    if let Some(usage) = v.get(usage_key) {
        flatten_field_names(usage, "", &mut usage_fields);
    }
    let usage = usage_row(wire, body);
    let text_of = |s: Option<&Value>| s.and_then(Value::as_str).map(str::to_string);
    match wire {
        Wire::OpenAiChat => {
            let choice = v.pointer("/choices/0");
            let message = choice.and_then(|c| c.get("message"));
            Parsed {
                finish: text_of(choice.and_then(|c| c.get("finish_reason"))),
                has_content: message
                    .and_then(|m| m.get("content"))
                    .is_some_and(non_blank_text),
                usage_fields,
                has_reasoning: message.is_some_and(|m| {
                    ["reasoning_content", "reasoning", "reasoning_details"]
                        .iter()
                        .any(|k| m.get(k).is_some_and(non_empty_value))
                }),
                note: None,
                usage: Box::new(usage),
            }
        }
        Wire::AnthropicMessages => {
            let blocks: &[Value] = v
                .get("content")
                .and_then(Value::as_array)
                .map_or(&[], Vec::as_slice);
            let block_type = |b: &Value| b.get("type").and_then(Value::as_str).map(str::to_string);
            Parsed {
                finish: text_of(v.get("stop_reason")),
                has_content: blocks.iter().any(|b| {
                    block_type(b).as_deref() == Some("text")
                        && b.get("text").is_some_and(non_blank_text)
                }),
                usage_fields,
                has_reasoning: blocks.iter().any(|b| {
                    matches!(
                        block_type(b).as_deref(),
                        Some("thinking") | Some("redacted_thinking")
                    )
                }),
                note: None,
                usage: Box::new(usage),
            }
        }
        Wire::GeminiGenerate => {
            let parts: &[Value] = v
                .pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
                .map_or(&[], Vec::as_slice);
            let is_thought = |p: &Value| p.get("thought").and_then(Value::as_bool) == Some(true);
            Parsed {
                finish: text_of(v.pointer("/candidates/0/finishReason")),
                has_content: parts
                    .iter()
                    .any(|p| !is_thought(p) && p.get("text").is_some_and(non_blank_text)),
                usage_fields,
                has_reasoning: parts
                    .iter()
                    .any(|p| is_thought(p) && p.get("text").is_some_and(non_blank_text)),
                note: None,
                usage: Box::new(usage),
            }
        }
    }
}

/// The numeric usage values of one call. Every field is an integer or absent: a field holding a
/// string (or anything else non-integer) is read as absent, so no vendor text can reach a row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct UsageRow {
    /// True when the response carried a usage object at all.
    has_usage: bool,
    /// Prompt tokens. Gemini: `promptTokenCount + toolUsePromptTokenCount`.
    prompt: Option<i64>,
    /// Completion tokens. Gemini: `candidatesTokenCount`.
    completion: Option<i64>,
    /// Reasoning tokens (Gemini: thoughts), as the vendor reports them.
    reasoning: Option<i64>,
    cached: Option<i64>,
    cache_write: Option<i64>,
    /// Gemini only: the tool-use prompt tokens already folded into [`UsageRow::prompt`].
    tool_use_prompt: Option<i64>,
    /// The vendor's own total, when it sends one (Anthropic does not).
    total: Option<i64>,
    /// Grok only: the billed cost in USD ticks.
    cost_in_usd_ticks: Option<i64>,
    /// Grok only: the number of sources a search-grounded call used.
    num_sources_used: Option<i64>,
}

/// Read the usage values out of a response body, integers only. A body that is not JSON, or has
/// no usage object, yields an empty row.
fn usage_row(wire: Wire, body: &str) -> UsageRow {
    let Ok(v) = serde_json::from_str::<Value>(body) else {
        return UsageRow::default();
    };
    let key = if wire == Wire::GeminiGenerate {
        "usageMetadata"
    } else {
        "usage"
    };
    let Some(usage) = v.get(key).filter(|u| u.is_object()) else {
        return UsageRow::default();
    };
    let int = |pointer: &str| usage.pointer(pointer).and_then(Value::as_i64);
    let mut row = UsageRow {
        has_usage: true,
        ..UsageRow::default()
    };
    match wire {
        Wire::OpenAiChat => {
            row.prompt = int("/prompt_tokens");
            row.completion = int("/completion_tokens");
            row.reasoning = int("/completion_tokens_details/reasoning_tokens");
            row.cached = int("/prompt_tokens_details/cached_tokens");
            row.cache_write = int("/prompt_tokens_details/cache_write_tokens");
            row.total = int("/total_tokens");
            row.cost_in_usd_ticks = int("/cost_in_usd_ticks");
            row.num_sources_used = int("/num_sources_used");
        }
        Wire::AnthropicMessages => {
            row.prompt = int("/input_tokens");
            row.completion = int("/output_tokens");
            row.reasoning = int("/output_tokens_details/thinking_tokens");
            row.cached = int("/cache_read_input_tokens");
            row.cache_write = int("/cache_creation_input_tokens");
        }
        Wire::GeminiGenerate => {
            row.tool_use_prompt = int("/toolUsePromptTokenCount");
            row.prompt = int("/promptTokenCount")
                .map(|p| p.saturating_add(row.tool_use_prompt.unwrap_or(0)));
            row.completion = int("/candidatesTokenCount");
            row.reasoning = int("/thoughtsTokenCount");
            row.cached = int("/cachedContentTokenCount");
            row.total = int("/totalTokenCount");
        }
    }
    row
}

/// The containment verdict for one call, derived only from the arithmetic. The difference is
/// computed in `i128` so no input can overflow or underflow it.
fn containment_verdict(wire: Wire, row: &UsageRow) -> String {
    if !row.has_usage {
        return "not measured (no usage object in the response)".to_string();
    }
    let (Some(prompt), Some(completion)) = (row.prompt, row.completion) else {
        return "not measured (usage lacks prompt or completion tokens)".to_string();
    };
    let total = match (wire, row.total) {
        (Wire::AnthropicMessages, _) | (_, None) => return "no vendor total".to_string(),
        (_, Some(total)) => total,
    };
    let difference = i128::from(total) - (i128::from(prompt) + i128::from(completion));
    let reasoning = row.reasoning.filter(|r| *r > 0);
    match reasoning {
        Some(_) if difference == 0 => "reasoning inside completion".to_string(),
        Some(r) if difference == i128::from(r) => "reasoning beside completion".to_string(),
        None if difference == 0 => "indeterminate (no reasoning reported)".to_string(),
        _ => format!("other (total - (prompt + completion) = {difference})"),
    }
}

/// The six usage cells of a table row: prompt, completion, reasoning, vendor total, verdict and
/// extras. Integers, the absent marker `-`, the fixed verdict grammar and fixed extras labels
/// only.
fn usage_cells(wire: Wire, row: &UsageRow) -> [String; 6] {
    let number = |n: Option<i64>| n.map_or_else(|| "-".to_string(), |n| n.to_string());
    let extras: Vec<String> = [
        ("cached", row.cached),
        ("cache_write", row.cache_write),
        ("tool_use_prompt", row.tool_use_prompt),
        ("cost_in_usd_ticks", row.cost_in_usd_ticks),
        ("num_sources_used", row.num_sources_used),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|v| format!("{label}={v}")))
    .collect();
    [
        number(row.prompt),
        number(row.completion),
        number(row.reasoning),
        number(row.total),
        containment_verdict(wire, row),
        if extras.is_empty() {
            "-".to_string()
        } else {
            extras.join("; ")
        },
    ]
}

/// What a non-2xx body says about the error, for the quota-shaped-error ledger.
#[derive(Debug, PartialEq, Eq)]
struct ErrorShape {
    kind: Option<String>,
    code: Option<String>,
    quota_shaped: bool,
}

fn error_shape(status: u16, body: &str) -> ErrorShape {
    let parsed = serde_json::from_str::<Value>(body).ok();
    let error = parsed.as_ref().and_then(|v| v.get("error"));
    let field = |name: &str| {
        error.and_then(|e| e.get(name)).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
    };
    // `error.type` (OpenAI-chat, Anthropic), else `error.status` (Gemini's `RESOURCE_EXHAUSTED`).
    let kind = field("type").or_else(|| field("status"));
    let code = field("code");
    let quota_shaped = status == 429
        || [kind.as_deref(), code.as_deref()]
            .into_iter()
            .flatten()
            .any(|text| {
                let text = text.to_ascii_lowercase();
                [
                    "quota",
                    "insufficient",
                    "exceed",
                    "billing",
                    "balance",
                    "credit",
                    "resource_exhausted",
                    "rate_limit",
                ]
                .iter()
                .any(|marker| text.contains(marker))
            });
    ErrorShape {
        kind,
        code,
        quota_shaped,
    }
}

fn with_auth(
    request: reqwest::RequestBuilder,
    auth: Auth,
    api_key: &str,
) -> reqwest::RequestBuilder {
    match auth {
        Auth::Bearer => request.bearer_auth(api_key),
        Auth::XApiKey => request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01"),
        Auth::GoogApiKey => request.header("x-goog-api-key", api_key),
    }
}

struct Reply {
    status: u16,
    body: String,
}

/// Send one request. A transport failure is returned as already-redacted text (the URL is
/// stripped from the `reqwest` error, which would otherwise print it).
async fn exchange(request: reqwest::RequestBuilder, api_key: &str) -> Result<Reply, String> {
    let response = request
        .send()
        .await
        .map_err(|e| diagnostic_excerpt(&e.without_url().to_string(), api_key))?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| diagnostic_excerpt(&e.without_url().to_string(), api_key))?;
    Ok(Reply { status, body })
}

/// Run one probe call and classify it. Returns the outcome and, for a rejection that looks like a
/// quota error, the verbatim (redacted, bounded) observation.
async fn run_probe_call(
    client: &reqwest::Client,
    spec: &ProviderSpec,
    built: &Built,
    model: &str,
    kind: ProbeKind,
) -> (Outcome, Option<String>) {
    let url = chat_url(spec.wire, &built.base_url, model);
    let body = request_body(spec, model, kind);
    let request = with_auth(client.post(&url).json(&body), spec.auth, &built.api_key);
    match exchange(request, &built.api_key).await {
        Err(e) => (Outcome::Unreachable(e), None),
        Ok(reply) if (200..300).contains(&reply.status) => {
            let parsed = parse_success(spec.wire, &reply.body, &built.api_key);
            (
                Outcome::Accepted {
                    status: reply.status,
                    parsed,
                },
                None,
            )
        }
        Ok(reply) => {
            let shape = error_shape(reply.status, &reply.body);
            let mut detail = excerpt(&reply.body, &built.api_key);
            if (300..400).contains(&reply.status) {
                // The client never follows a redirect, so the credential header cannot be
                // forwarded to the `Location` host.
                detail = format!("redirect not followed {detail}")
                    .trim_end()
                    .to_string();
            }
            let quota = shape.quota_shaped.then(|| {
                format!(
                    "status {}, error.type/status `{}`, error.code `{}`: {}",
                    reply.status,
                    excerpt(shape.kind.as_deref().unwrap_or("-"), &built.api_key),
                    excerpt(shape.code.as_deref().unwrap_or("-"), &built.api_key),
                    detail
                )
            });
            (
                Outcome::Rejected {
                    status: reply.status,
                    detail,
                },
                quota,
            )
        }
    }
}

fn render_outcome_cells(outcome: &Outcome) -> [String; 5] {
    match outcome {
        Outcome::Accepted { status, parsed } => [
            match &parsed.note {
                Some(note) => format!("ACCEPTED ({status}); {note}"),
                None => format!("ACCEPTED ({status})"),
            },
            parsed
                .finish
                .clone()
                .map_or_else(|| "-".to_string(), |f| cell(&f)),
            if parsed.has_content {
                "non-empty"
            } else {
                "EMPTY"
            }
            .to_string(),
            if parsed.usage_fields.is_empty() {
                "none".to_string()
            } else {
                cell(&parsed.usage_fields.join(", "))
            },
            if parsed.has_reasoning {
                "present"
            } else {
                "absent"
            }
            .to_string(),
        ],
        Outcome::Rejected { status, detail } => [
            format!("REJECTED ({status}): {detail}"),
            "-".into(),
            "-".into(),
            "-".into(),
            "-".into(),
        ],
        Outcome::Unreachable(detail) => [
            format!("UNREACHABLE: {detail}"),
            "-".into(),
            "-".into(),
            "-".into(),
            "-".into(),
        ],
        Outcome::NotApplicable(why) => [
            format!("NOT APPLICABLE ({why})"),
            "-".into(),
            "-".into(),
            "-".into(),
            "-".into(),
        ],
    }
}

/// One `limit/capability` cell per model-listing entry: every field the vendor returned, each
/// through the redact-then-bound excerpt.
fn listing_rows(body: &Value, api_key: &str) -> Option<Vec<(String, String)>> {
    let entries = body
        .get("data")
        .or_else(|| body.get("models"))
        .or(Some(body))
        .and_then(Value::as_array)?;
    let mut rows = Vec::new();
    for entry in entries {
        match entry {
            Value::String(id) => rows.push((excerpt(id, api_key), String::new())),
            Value::Object(map) => {
                let id_key = ["id", "name", "model"]
                    .into_iter()
                    .find(|k| map.get(*k).is_some_and(Value::is_string));
                let id = id_key
                    .and_then(|k| map.get(k))
                    .and_then(Value::as_str)
                    .map(|s| s.strip_prefix("models/").unwrap_or(s))
                    .unwrap_or("<no id field>");
                let fields: Vec<String> = map
                    .iter()
                    .filter(|(k, _)| Some(k.as_str()) != id_key)
                    .map(|(k, v)| {
                        let shown = match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        excerpt(&format!("{k}={shown}"), api_key)
                    })
                    .collect();
                rows.push((excerpt(id, api_key), fields.join(" ; ")));
            }
            _ => {}
        }
    }
    Some(rows)
}

/// Header facts shared by the live probe and the dry run.
fn write_section_facts(out: &mut String, spec: &ProviderSpec, base_url: &str, model: &str) {
    let _ = writeln!(out, "\n- base URL: `{}`", redact_url(base_url));
    let _ = writeln!(
        out,
        "- chat endpoint: `POST {}`; credential header `{}` (name only, value never printed)",
        redact_url(&chat_url(spec.wire, base_url, model)),
        spec.auth.header_name()
    );
    let _ = writeln!(
        out,
        "- models endpoint: `GET {}`",
        redact_url(&models_url(spec.wire, base_url))
    );
    match spec.temperature_range {
        Some((lo, hi)) => {
            let _ = writeln!(out, "- declared temperature range: {lo}..{hi}");
        }
        None => {
            let _ = writeln!(
                out,
                "- declared temperature range: none (operator-declared); probing 0.0..2.0"
            );
        }
    }
    let cap = match spec.wire {
        Wire::OpenAiChat => spec.cap_field,
        Wire::AnthropicMessages => "max_tokens",
        Wire::GeminiGenerate => "generationConfig.maxOutputTokens",
    };
    let _ = writeln!(
        out,
        "- every call carries `{cap}` = {PROBE_MAX_TOKENS} and a {PROBE_TIMEOUT_SECS} s timeout; \
         redirects are never followed"
    );
    if let Some(extras) = spec.acceptance_extras {
        let _ = writeln!(
            out,
            "- body extra on every acceptance probe except `default thinking`: `{extras}`"
        );
    }
}

fn write_probe_table_header(out: &mut String, with_outcome_columns: bool) {
    if with_outcome_columns {
        let _ = writeln!(
            out,
            "| # | probe | body extras | outcome | finish | content | usage fields | reasoning text |\n\
             |---|-------|-------------|---------|--------|---------|--------------|----------------|"
        );
    } else {
        let _ = writeln!(
            out,
            "| # | probe | body extras | applicability |\n|---|-------|-------------|---------------|"
        );
    }
}

/// The offline plan: the same section skeleton as a live probe, for all nine providers, with no
/// network access and no credential read.
fn render_dry_run(models: &[(String, Vec<String>)]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Live vendor probe plan (--dry-run)\n");
    let _ = writeln!(
        out,
        "- no network access was made and no credential was read"
    );
    for spec in PROVIDERS.iter() {
        let _ = writeln!(out, "\n## {}", spec.name);
        let base_url = spec.default_base_url.map_or_else(
            || "<OPENAI_COMPATIBLE_BASE_URL>".to_string(),
            str::to_string,
        );
        write_section_facts(&mut out, spec, &base_url, spec.default_model);
        let _ = writeln!(
            out,
            "- credential variables required: {}",
            spec.cred_vars
                .iter()
                .map(|v| format!("`{v}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let extra = extra_models_for(models, spec);
        let _ = writeln!(out, "\n### Candidate models\n");
        for model in candidates(spec, spec.default_model, &extra) {
            let _ = writeln!(out, "- `{model}`");
        }
        let _ = writeln!(
            out,
            "\n### Ordered probe list (run once per candidate model)\n"
        );
        write_probe_table_header(&mut out, false);
        for (i, probe) in probe_plan(spec).iter().enumerate() {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} |",
                i + 1,
                cell(&probe.label),
                extras_for(spec, probe.kind).map_or_else(|| "-".to_string(), |e| format!("`{e}`")),
                probe.not_applicable.map_or_else(
                    || "planned".to_string(),
                    |why| format!("NOT APPLICABLE ({why})")
                ),
            );
        }
    }
    out
}

fn extra_models_for(models: &[(String, Vec<String>)], spec: &ProviderSpec) -> Vec<String> {
    models
        .iter()
        .filter(|(provider, _)| provider == spec.name)
        .flat_map(|(_, ids)| ids.iter().cloned())
        .collect()
}

/// How a provider's probe ended, for the exit code.
enum ProbeEnd {
    /// At least one HTTP exchange completed.
    Reached,
    /// Not one HTTP exchange completed -- never reported as a pass.
    Unreachable,
    /// The adapter could not be built from the environment.
    NotBuilt,
}

async fn probe_provider(
    client: &reqwest::Client,
    spec: &ProviderSpec,
    options: &Options,
) -> ProbeEnd {
    let built = match (spec.build)() {
        Ok(b) => b,
        Err(e) => {
            println!(
                "\nNOT PROBED: the adapter could not be built from the environment: {}",
                excerpt(&e, "")
            );
            return ProbeEnd::NotBuilt;
        }
    };
    let key = built.api_key.as_str();
    let mut reached = false;
    let mut quota_observations: Vec<String> = Vec::new();

    let mut facts = String::new();
    write_section_facts(&mut facts, spec, &built.base_url, &built.model);
    print!("{facts}");

    // (1) the live models listing, every limit or capability field as received.
    println!("\n### Models listing\n");
    let listing_request = with_auth(
        client.get(models_url(spec.wire, &built.base_url)),
        spec.auth,
        key,
    );
    match exchange(listing_request, key).await {
        Err(e) => println!("UNREACHABLE: {e}"),
        Ok(reply) if (200..300).contains(&reply.status) => {
            reached = true;
            match serde_json::from_str::<Value>(&reply.body)
                .ok()
                .and_then(|v| listing_rows(&v, key))
            {
                Some(rows) => {
                    println!("| model id | limit / capability fields (as received) |\n|---|---|");
                    for (id, fields) in rows {
                        println!("| {id} | {fields} |");
                    }
                }
                None => println!(
                    "ACCEPTED ({}) but the body is not a recognisable model listing: {}",
                    reply.status,
                    excerpt(&reply.body, key)
                ),
            }
        }
        Ok(reply) => {
            reached = true;
            println!("REJECTED ({}): {}", reply.status, excerpt(&reply.body, key));
            let shape = error_shape(reply.status, &reply.body);
            if shape.quota_shaped {
                quota_observations.push(format!(
                    "models listing: status {}: {}",
                    reply.status,
                    excerpt(&reply.body, key)
                ));
            }
        }
    }

    // (2)-(3) candidates and their ordered probes.
    let extra = extra_models_for(&options.models, spec);
    let plan = probe_plan(spec);
    for model in candidates(spec, &built.model, &extra) {
        println!("\n### Probes: `{model}`\n");
        let mut table = String::new();
        write_probe_table_header(&mut table, true);
        print!("{table}");
        for (i, probe) in plan.iter().enumerate() {
            let (outcome, quota) = match probe.not_applicable {
                Some(why) => (Outcome::NotApplicable(why), None),
                None => run_probe_call(client, spec, &built, &model, probe.kind).await,
            };
            if !matches!(outcome, Outcome::Unreachable(_) | Outcome::NotApplicable(_)) {
                reached = true;
            }
            if let Some(observation) = quota {
                quota_observations.push(format!("`{model}` / {}: {observation}", probe.label));
            }
            let [outcome_cell, finish, content, usage, reasoning] = render_outcome_cells(&outcome);
            println!(
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                i + 1,
                cell(&probe.label),
                extras_for(spec, probe.kind).map_or_else(|| "-".to_string(), |e| format!("`{e}`")),
                outcome_cell,
                finish,
                content,
                usage,
                reasoning
            );
        }
    }

    // (4) quota-shaped errors observed, verbatim through the excerpt.
    println!("\n### Quota-shaped errors observed\n");
    if quota_observations.is_empty() {
        println!("none observed (a quota error cannot be provoked on demand)");
    } else {
        for observation in &quota_observations {
            println!("- {observation}");
        }
    }

    if reached {
        ProbeEnd::Reached
    } else {
        ProbeEnd::Unreachable
    }
}

/// `git rev-parse --short HEAD` when available.
fn git_short_head() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// The credentialed HTTP client shared by `--probe` and `--usage`: a per-call timeout, and no
/// redirect is ever followed, so the credential header cannot be forwarded to a host the vendor
/// does not control (T-50-04-02).
fn build_probe_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("could not build the HTTP client: {}", e.without_url()))
}

/// The transcript header shared by `--probe` and `--usage`: title, UTC date, commit and the
/// credential variable NAMES found (never values).
fn print_run_header(title: &str, lookup: &dyn Fn(&str) -> Option<String>) {
    let found: Vec<&str> = PROVIDERS
        .iter()
        .flat_map(|p| p.cred_vars.iter().copied())
        .filter(|var| lookup(var).is_some_and(|v| !v.trim().is_empty()))
        .collect();

    println!("# {title}\n");
    println!(
        "- date (UTC): {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%SZ")
    );
    if let Some(head) = git_short_head() {
        println!("- commit: {head}");
    }
    println!(
        "- credential variables found (names only, never values): {}",
        if found.is_empty() {
            "none".to_string()
        } else {
            found.join(", ")
        }
    );
}

async fn run_probe(options: &Options) -> i32 {
    let client = match build_probe_client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let lookup = |name: &str| std::env::var(name).ok();
    print_run_header("Live vendor probe", &lookup);

    let (mut configured, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    for spec in PROVIDERS.iter() {
        println!("\n## {}", spec.name);
        if let Some(var) = missing_credential(spec, &lookup) {
            println!("\nSKIPPED ({var} not set)");
            skipped += 1;
            continue;
        }
        configured += 1;
        match probe_provider(&client, spec, options).await {
            ProbeEnd::Reached => {}
            ProbeEnd::Unreachable => {
                println!("\nUNREACHABLE: no HTTP exchange with this provider completed");
                failed += 1;
            }
            ProbeEnd::NotBuilt => failed += 1,
        }
    }

    println!("\n---\n");
    println!(
        "{configured} of {} providers probed ({skipped} not configured); {failed} unreachable or \
         not buildable",
        PROVIDERS.len()
    );
    if configured == 0 {
        eprintln!("no provider was configured -- nothing was tested");
    }
    smoke_exit_code(configured, failed)
}

// ---------------------------------------------------------------------------------------------
// Usage mode (`--usage`): the recorded containment measurement (G-50-5a)
// ---------------------------------------------------------------------------------------------

/// Candidates of a `--usage` run: the provider's default model plus the `--models` ids, without
/// duplicates. The curated lists are not added -- every call costs the operator money.
fn usage_candidates(_spec: &ProviderSpec, default_model: &str, extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for model in std::iter::once(default_model.to_string()).chain(extra.iter().cloned()) {
        if !out.contains(&model) {
            out.push(model);
        }
    }
    out
}

/// Header facts of a provider's `--usage` section.
fn write_usage_section_facts(out: &mut String, spec: &ProviderSpec, base_url: &str, model: &str) {
    let cap = match spec.wire {
        Wire::OpenAiChat => spec.cap_field,
        Wire::AnthropicMessages => "max_tokens",
        Wire::GeminiGenerate => "generationConfig.maxOutputTokens",
    };
    let _ = writeln!(out, "\n- base URL: `{}`", redact_url(base_url));
    let _ = writeln!(
        out,
        "- chat endpoint: `POST {}`; credential header `{}` (name only, value never printed)",
        redact_url(&chat_url(spec.wire, base_url, model)),
        spec.auth.header_name()
    );
    let _ = writeln!(
        out,
        "- every call carries `{cap}` = {USAGE_MAX_TOKENS}, a {PROBE_TIMEOUT_SECS} s timeout, no \
         body extras and no sampling parameters; redirects are never followed"
    );
}

/// The offline `--usage` plan: every planned call for all nine providers, with no network access
/// and no credential read.
fn render_usage_dry_run(models: &[(String, Vec<String>)]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Live vendor usage probe plan (--usage --dry-run)\n");
    let _ = writeln!(
        out,
        "- no network access was made and no credential was read"
    );
    let _ = writeln!(out, "- prompt: \"{USAGE_PROMPT}\"");
    for spec in PROVIDERS.iter() {
        let _ = writeln!(out, "\n## {}", spec.name);
        let base_url = spec.default_base_url.map_or_else(
            || "<OPENAI_COMPATIBLE_BASE_URL>".to_string(),
            str::to_string,
        );
        write_usage_section_facts(&mut out, spec, &base_url, spec.default_model);
        let _ = writeln!(
            out,
            "- credential variables required: {}",
            spec.cred_vars
                .iter()
                .map(|v| format!("`{v}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let extra = extra_models_for(models, spec);
        let _ = writeln!(out, "\n### Planned usage calls\n");
        for model in usage_candidates(spec, spec.default_model, &extra) {
            let _ = writeln!(out, "- `{model}`: 1 call");
        }
    }
    out
}

/// The seven cells of a `--usage` table row after the model: the outcome, then [`usage_cells`].
/// A call without a usable response is `not measured (<reason>)`, never a guess.
fn usage_table_cells(wire: Wire, outcome: &Outcome) -> [String; 7] {
    let blank = |outcome_cell: String, verdict: String| -> [String; 7] {
        [
            outcome_cell,
            "-".into(),
            "-".into(),
            "-".into(),
            "-".into(),
            verdict,
            "-".into(),
        ]
    };
    match outcome {
        Outcome::Accepted { status, parsed } => {
            let [prompt, completion, reasoning, total, verdict, extras] =
                usage_cells(wire, &parsed.usage);
            [
                format!("ACCEPTED ({status})"),
                prompt,
                completion,
                reasoning,
                total,
                verdict,
                extras,
            ]
        }
        Outcome::Rejected { status, detail } => blank(
            format!("REJECTED ({status}): {detail}"),
            format!("not measured (HTTP {status})"),
        ),
        Outcome::Unreachable(detail) => blank(
            format!("UNREACHABLE: {detail}"),
            "not measured (unreachable)".to_string(),
        ),
        Outcome::NotApplicable(why) => blank(
            format!("NOT APPLICABLE ({why})"),
            format!("not measured ({why})"),
        ),
    }
}

async fn usage_provider(
    client: &reqwest::Client,
    spec: &ProviderSpec,
    options: &Options,
) -> ProbeEnd {
    let built = match (spec.build)() {
        Ok(b) => b,
        Err(e) => {
            println!(
                "\nNOT MEASURED: the adapter could not be built from the environment: {}",
                excerpt(&e, "")
            );
            return ProbeEnd::NotBuilt;
        }
    };
    let mut facts = String::new();
    write_usage_section_facts(&mut facts, spec, &built.base_url, &built.model);
    print!("{facts}");
    println!(
        "\n| model | outcome | prompt | completion | reasoning | vendor total | verdict | extras |\n\
         |---|---|---|---|---|---|---|---|"
    );
    let extra = extra_models_for(&options.models, spec);
    let mut reached = false;
    for model in usage_candidates(spec, &built.model, &extra) {
        let (outcome, _) = run_probe_call(client, spec, &built, &model, ProbeKind::Usage).await;
        if !matches!(outcome, Outcome::Unreachable(_) | Outcome::NotApplicable(_)) {
            reached = true;
        }
        let [
            outcome_cell,
            prompt,
            completion,
            reasoning,
            total,
            verdict,
            extras,
        ] = usage_table_cells(spec.wire, &outcome);
        println!(
            "| {} | {outcome_cell} | {prompt} | {completion} | {reasoning} | {total} | {verdict} \
             | {extras} |",
            cell(&model)
        );
    }
    if reached {
        ProbeEnd::Reached
    } else {
        ProbeEnd::Unreachable
    }
}

async fn run_usage(options: &Options) -> i32 {
    let client = match build_probe_client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let lookup = |name: &str| std::env::var(name).ok();
    print_run_header("Live vendor usage probe", &lookup);
    println!(
        "- prompt: \"{USAGE_PROMPT}\" (numbers are recorded; response content is never printed)"
    );

    let (mut configured, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    for spec in PROVIDERS.iter() {
        println!("\n## {}", spec.name);
        if let Some(var) = missing_credential(spec, &lookup) {
            println!("\nSKIPPED ({var} not set)");
            skipped += 1;
            continue;
        }
        configured += 1;
        match usage_provider(&client, spec, options).await {
            ProbeEnd::Reached => {}
            ProbeEnd::Unreachable => {
                println!("\nUNREACHABLE: no HTTP exchange with this provider completed");
                failed += 1;
            }
            ProbeEnd::NotBuilt => failed += 1,
        }
    }

    println!("\n---\n");
    println!(
        "{configured} of {} providers measured ({skipped} not configured); {failed} unreachable \
         or not buildable",
        PROVIDERS.len()
    );
    if configured == 0 {
        eprintln!("no provider was configured -- nothing was tested");
    }
    smoke_exit_code(configured, failed)
}

// ---------------------------------------------------------------------------------------------
// Effort mode (`--effort`): the Phase 51 raw-wire reasoning-effort probe (D-01)
// ---------------------------------------------------------------------------------------------

/// The effort levels probed through every level-taking form, lowest first.
const EFFORT_LEVELS: [&str; 6] = ["minimal", "low", "medium", "high", "xhigh", "max"];

/// What an effort call is for; the summary lines are computed from the roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EffortRole {
    /// The baseline: no effort field at all.
    Default,
    /// A valid level through a form.
    Level,
    /// The off form.
    Off,
    /// The auto form (thinking on, vendor's default depth).
    Auto,
    /// An out-of-vocabulary value through a form: a parsed field rejects it.
    Invalid,
    /// `high` plus `temperature: 0.5`, detecting a sampling-with-effort rejection (Pitfall 13).
    Sampling,
    /// An explicit reasoning-token budget (at a bound, or one step outside it).
    Budget,
    /// A top-level `reasoning_effort: bogus-effort-51` on a vendor whose candidate form is not
    /// that field: a 2xx answer means the field is silently ignored (research A4).
    Detector,
}

/// One planned effort call: the form name and value label shown in the transcript, and the JSON
/// object deep-merged into the base chat body.
#[derive(Clone, Debug, PartialEq)]
struct EffortCall {
    /// The wire form: `default`, `top_level_effort`, `thinking_object`, ...
    form: &'static str,
    /// The value label: a level, `off`, `auto`, the invalid value, or `-`.
    value: String,
    role: EffortRole,
    /// Merged into the base body; `Value::Null` for the default call.
    extra: Value,
}

impl EffortCall {
    fn new(form: &'static str, value: &str, role: EffortRole, extra: Value) -> Self {
        Self {
            form,
            value: value.to_string(),
            role,
            extra,
        }
    }

    /// Relabel the call (an invalid numeric budget is shown as its number, not as the string).
    fn with_value(mut self, value: &str) -> Self {
        self.value = value.to_string();
        self
    }
}

/// Merge `extra` into `target`: objects merge key by key, anything else replaces.
fn deep_merge(target: &mut Value, extra: &Value) {
    match (target, extra) {
        (Value::Object(into), Value::Object(from)) => {
            for (key, value) in from {
                if value.is_object()
                    && let Some(existing) = into.get_mut(key)
                    && existing.is_object()
                {
                    deep_merge(existing, value);
                } else {
                    into.insert(key.clone(), value.clone());
                }
            }
        }
        (_, Value::Null) => {}
        (target, extra) => *target = extra.clone(),
    }
}

/// OpenAI-style top-level `reasoning_effort`.
fn top_level_effort(value: &str) -> Value {
    json!({"reasoning_effort": value})
}

/// Kimi-style `thinking: {type: enabled, effort}`.
fn thinking_object_effort(value: &str) -> Value {
    json!({"thinking": {"type": "enabled", "effort": value}})
}

/// The call that carries no effort field: the vendor's own default.
fn default_call() -> EffortCall {
    EffortCall::new("default", "-", EffortRole::Default, Value::Null)
}

/// One valid-level call per entry of `levels`, through `form`, each body built by `extra`.
fn level_calls(
    form: &'static str,
    levels: &[&str],
    extra: impl Fn(&str) -> Value,
) -> Vec<EffortCall> {
    levels
        .iter()
        .map(|level| EffortCall::new(form, level, EffortRole::Level, extra(level)))
        .collect()
}

/// The `high` + `temperature: 0.5` row: `extra` plus a top-level `temperature`.
fn sampling_call(form: &'static str, value: &str, mut extra: Value) -> EffortCall {
    deep_merge(&mut extra, &json!({"temperature": 0.5}));
    EffortCall::new(form, value, EffortRole::Sampling, extra)
}

/// A call whose label is the invalid value itself.
fn invalid_call(form: &'static str, extra: Value) -> EffortCall {
    EffortCall::new(form, EFFORT_INVALID_VALUE, EffortRole::Invalid, extra)
}

/// The top-level `reasoning_effort: bogus-effort-51` detector for a vendor whose candidate form
/// is not that field (research A4): a 2xx answer means the field is silently ignored.
fn detector_call() -> EffortCall {
    EffortCall::new(
        "detector",
        EFFORT_INVALID_VALUE,
        EffortRole::Detector,
        top_level_effort(EFFORT_INVALID_VALUE),
    )
}

/// The Kimi plan (research Open Question 1 and Pitfall 2): both candidate forms for every level,
/// so the transcript shows which form changes `reasoning_tokens`, then off, auto, one invalid
/// value per form and the sampling row. The sampling row uses the `thinking` object, the form the
/// vendor documents.
fn kimi_effort_plan() -> Vec<EffortCall> {
    let mut plan = vec![default_call()];
    for level in EFFORT_LEVELS {
        plan.push(EffortCall::new(
            "top_level_effort",
            level,
            EffortRole::Level,
            top_level_effort(level),
        ));
        plan.push(EffortCall::new(
            "thinking_object",
            level,
            EffortRole::Level,
            thinking_object_effort(level),
        ));
    }
    plan.push(EffortCall::new(
        "thinking_object",
        "off",
        EffortRole::Off,
        json!({"thinking": {"type": "disabled"}}),
    ));
    plan.push(EffortCall::new(
        "thinking_object",
        "auto",
        EffortRole::Auto,
        json!({"thinking": {"type": "enabled"}}),
    ));
    plan.push(invalid_call(
        "top_level_effort",
        top_level_effort(EFFORT_INVALID_VALUE),
    ));
    plan.push(invalid_call(
        "thinking_object",
        thinking_object_effort(EFFORT_INVALID_VALUE),
    ));
    plan.push(sampling_call(
        "thinking_object",
        "high+temperature=0.5",
        thinking_object_effort("high"),
    ));
    plan
}

/// The OpenAI plan: `none` is the off form, then every level, one invalid value (which is also
/// the silent-field detector) and the sampling row. `Auto` is "send nothing", so it has no call.
fn openai_effort_plan() -> Vec<EffortCall> {
    let mut plan = vec![default_call()];
    plan.push(EffortCall::new(
        "top_level_effort",
        "none",
        EffortRole::Off,
        top_level_effort("none"),
    ));
    plan.extend(level_calls(
        "top_level_effort",
        &EFFORT_LEVELS,
        top_level_effort,
    ));
    plan.push(invalid_call(
        "top_level_effort",
        top_level_effort(EFFORT_INVALID_VALUE),
    ));
    plan.push(sampling_call(
        "top_level_effort",
        "high+temperature=0.5",
        top_level_effort("high"),
    ));
    plan
}

/// The DeepSeek plan: every level through `reasoning_effort`, the `thinking` toggle for off and
/// auto, an invalid value through `reasoning_effort` (also the detector) and through
/// `thinking.type`, and the sampling row. The valid low and high pair is the effect check the
/// invalid call alone cannot give (DeepSeek documents coercion of some unknown names, A1-51-02).
fn deepseek_effort_plan() -> Vec<EffortCall> {
    let mut plan = vec![default_call()];
    plan.extend(level_calls(
        "top_level_effort",
        &EFFORT_LEVELS,
        top_level_effort,
    ));
    plan.push(EffortCall::new(
        "thinking_object",
        "off",
        EffortRole::Off,
        json!({"thinking": {"type": "disabled"}}),
    ));
    plan.push(EffortCall::new(
        "thinking_object",
        "auto",
        EffortRole::Auto,
        json!({"thinking": {"type": "enabled"}}),
    ));
    plan.push(invalid_call(
        "top_level_effort",
        top_level_effort(EFFORT_INVALID_VALUE),
    ));
    plan.push(invalid_call(
        "thinking_object",
        json!({"thinking": {"type": EFFORT_INVALID_VALUE}}),
    ));
    plan.push(sampling_call(
        "top_level_effort",
        "high+temperature=0.5",
        top_level_effort("high"),
    ));
    plan
}

/// The Grok plan: every level through `reasoning_effort`, one invalid value (also the detector)
/// and the sampling row. The chat-completions field is undocumented, so the plan only asks
/// whether the OpenAI-style field is parsed and has an effect.
fn grok_effort_plan() -> Vec<EffortCall> {
    let mut plan = vec![default_call()];
    plan.extend(level_calls(
        "top_level_effort",
        &EFFORT_LEVELS,
        top_level_effort,
    ));
    plan.push(invalid_call(
        "top_level_effort",
        top_level_effort(EFFORT_INVALID_VALUE),
    ));
    plan.push(sampling_call(
        "top_level_effort",
        "high+temperature=0.5",
        top_level_effort("high"),
    ));
    plan
}

/// `enable_thinking: true` with an explicit `thinking_budget`.
fn qwen_budget(budget: i64) -> Value {
    json!({"enable_thinking": true, "thinking_budget": budget})
}

/// The Qwen plan: the thinking toggle, the budget at its lower bound (1) and one step outside it
/// (0), an invalid budget (-1), the silent-field detector (Qwen has no levels) and a sampling row
/// with thinking on.
fn qwen_effort_plan() -> Vec<EffortCall> {
    vec![
        default_call(),
        EffortCall::new(
            "enable_thinking",
            "off",
            EffortRole::Off,
            json!({"enable_thinking": false}),
        ),
        EffortCall::new(
            "enable_thinking",
            "auto",
            EffortRole::Auto,
            json!({"enable_thinking": true}),
        ),
        EffortCall::new("thinking_budget", "1", EffortRole::Budget, qwen_budget(1)),
        EffortCall::new("thinking_budget", "0", EffortRole::Budget, qwen_budget(0)),
        invalid_call("thinking_budget", qwen_budget(-1)).with_value("-1"),
        detector_call(),
        sampling_call(
            "enable_thinking",
            "auto+temperature=0.5",
            json!({"enable_thinking": true}),
        ),
    ]
}

/// `output_config: {effort}`.
fn output_config_effort(value: &str) -> Value {
    json!({"output_config": {"effort": value}})
}

/// `output_config: {effort}` paired with `thinking: {type: adaptive}`.
fn output_config_adaptive(value: &str) -> Value {
    let mut body = output_config_effort(value);
    deep_merge(&mut body, &json!({"thinking": {"type": "adaptive"}}));
    body
}

/// `thinking: {type: enabled, budget_tokens}`.
fn anthropic_budget(tokens: u32) -> Value {
    json!({"thinking": {"type": "enabled", "budget_tokens": tokens}})
}

/// The Anthropic plan (research Open Question 3 and Pitfalls 3-4): every level twice, alone and
/// paired with adaptive thinking, so the transcript shows whether a default-off model needs the
/// pairing; the thinking toggles; an invalid effort; `budget_tokens` at its documented floor
/// (1024) and one step below it (1023); and the sampling row.
fn anthropic_effort_plan() -> Vec<EffortCall> {
    let levels = ["low", "medium", "high", "xhigh", "max"];
    let mut plan = vec![default_call()];
    plan.extend(level_calls("output_config", &levels, output_config_effort));
    plan.extend(level_calls(
        "output_config+adaptive",
        &levels,
        output_config_adaptive,
    ));
    plan.push(EffortCall::new(
        "thinking_object",
        "off",
        EffortRole::Off,
        json!({"thinking": {"type": "disabled"}}),
    ));
    plan.push(EffortCall::new(
        "thinking_object",
        "auto",
        EffortRole::Auto,
        json!({"thinking": {"type": "adaptive"}}),
    ));
    plan.push(invalid_call(
        "output_config",
        output_config_effort(EFFORT_INVALID_VALUE),
    ));
    plan.push(EffortCall::new(
        "thinking_budget",
        "1024",
        EffortRole::Budget,
        anthropic_budget(1024),
    ));
    plan.push(EffortCall::new(
        "thinking_budget",
        "1023",
        EffortRole::Budget,
        anthropic_budget(1023),
    ));
    plan.push(sampling_call(
        "output_config",
        "high+temperature=0.5",
        output_config_effort("high"),
    ));
    plan
}

/// `generationConfig.thinkingConfig.thinkingLevel`.
fn gemini_level(value: &str) -> Value {
    json!({"generationConfig": {"thinkingConfig": {"thinkingLevel": value}}})
}

/// `generationConfig.thinkingConfig.thinkingBudget`.
fn gemini_budget(value: i64) -> Value {
    json!({"generationConfig": {"thinkingConfig": {"thinkingBudget": value}}})
}

/// The Gemini plan (research A2): the documented lowercase `thinkingLevel` values plus one
/// uppercase variant, `thinkingBudget: 0` as the off form, an invalid level and an invalid
/// budget, the budget at 128 and 1, and the sampling row. The temperature lives inside
/// `generationConfig`, which the deep merge keeps beside the output cap.
fn gemini_effort_plan() -> Vec<EffortCall> {
    let mut plan = vec![default_call()];
    plan.extend(level_calls(
        "thinking_level",
        &["minimal", "low", "medium", "high"],
        gemini_level,
    ));
    plan.push(EffortCall::new(
        "thinking_level",
        "HIGH",
        EffortRole::Level,
        gemini_level("HIGH"),
    ));
    plan.push(EffortCall::new(
        "thinking_budget",
        "off",
        EffortRole::Off,
        gemini_budget(0),
    ));
    plan.push(invalid_call(
        "thinking_level",
        gemini_level(EFFORT_INVALID_VALUE),
    ));
    plan.push(invalid_call("thinking_budget", gemini_budget(-5)).with_value("-5"));
    plan.push(EffortCall::new(
        "thinking_budget",
        "128",
        EffortRole::Budget,
        gemini_budget(128),
    ));
    plan.push(EffortCall::new(
        "thinking_budget",
        "1",
        EffortRole::Budget,
        gemini_budget(1),
    ));
    let mut sampling = gemini_level("high");
    deep_merge(
        &mut sampling,
        &json!({"generationConfig": {"temperature": 0.5}}),
    );
    plan.push(EffortCall::new(
        "thinking_level",
        "high+temperature=0.5",
        EffortRole::Sampling,
        sampling,
    ));
    plan
}

/// The plan for an `ollama` or `openai_compatible` target (D-10): all four `ReasoningWire`
/// candidate forms -- the top-level field, the `thinking` object and the `enable_thinking` pair --
/// each with an invalid value, so an operator can prove which field their endpoint parses. The
/// top-level invalid value doubles as the silent-field detector.
fn self_hosted_effort_plan() -> Vec<EffortCall> {
    let mut plan = kimi_effort_plan();
    // Replace Kimi's sampling row (it rides on the `thinking` object) with the three
    // `enable_thinking` forms and a sampling row on the most widely served field.
    plan.pop();
    plan.push(EffortCall::new(
        "enable_thinking",
        "off",
        EffortRole::Off,
        json!({"enable_thinking": false}),
    ));
    plan.push(EffortCall::new(
        "enable_thinking",
        "auto",
        EffortRole::Auto,
        json!({"enable_thinking": true}),
    ));
    plan.push(EffortCall::new(
        "thinking_budget",
        "1024",
        EffortRole::Budget,
        qwen_budget(1024),
    ));
    plan.push(invalid_call("thinking_budget", qwen_budget(-1)).with_value("-1"));
    plan.push(sampling_call(
        "top_level_effort",
        "high+temperature=0.5",
        top_level_effort("high"),
    ));
    plan
}

/// The ordered effort plan for `spec`, identical for every candidate model of the provider.
fn effort_plan(spec: &ProviderSpec) -> Vec<EffortCall> {
    match spec.name {
        "OpenAI" => openai_effort_plan(),
        "Anthropic" => anthropic_effort_plan(),
        "DeepSeek" => deepseek_effort_plan(),
        "Kimi" => kimi_effort_plan(),
        "Qwen" => qwen_effort_plan(),
        "Grok" => grok_effort_plan(),
        "Gemini" => gemini_effort_plan(),
        "Ollama" | "OpenAI-compatible" => self_hosted_effort_plan(),
        _ => Vec::new(),
    }
}

/// Candidates of an `--effort` run: the `--models` ids only, without duplicates. No curated or
/// default model is added -- every call costs the operator money, so the spend is exactly the
/// confirmed candidate set.
fn effort_candidates(extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for model in extra {
        if !out.contains(model) {
            out.push(model.clone());
        }
    }
    out
}

/// The JSON body for one effort call: the baseline chat body under [`EFFORT_MAX_TOKENS`] with no
/// acceptance extra and no sampling parameter, plus the call's effort fields.
fn effort_body(spec: &ProviderSpec, model: &str, call: &EffortCall) -> Value {
    let mut body = request_body(spec, model, ProbeKind::Effort);
    deep_merge(&mut body, &call.extra);
    body
}

/// How an effort call ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EffortVerdict {
    /// A 2xx answer.
    Accepted,
    /// The vendor refused the request body (HTTP 400 or 422): a parsed field with a bad value.
    Rejected,
    /// Anything else: a transport failure, a rate limit, an auth failure, a 5xx, a redirect.
    Error,
}

impl EffortVerdict {
    fn label(self) -> &'static str {
        match self {
            EffortVerdict::Accepted => "accepted",
            EffortVerdict::Rejected => "rejected",
            EffortVerdict::Error => "error",
        }
    }
}

/// One measured effort call: numbers, a verdict and a bounded excerpt of a refusal body. Never the
/// response content.
#[derive(Clone, Debug, PartialEq)]
struct EffortRow {
    form: &'static str,
    value: String,
    role: EffortRole,
    /// `None` when no HTTP status was received.
    status: Option<u16>,
    finish: Option<String>,
    reasoning_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    elapsed_ms: u128,
    verdict: EffortVerdict,
    /// A redacted, bounded, table-safe excerpt of a non-2xx body or transport error; `-` otherwise.
    detail: String,
}

/// Turn one exchange into a row. A transport error and every vendor-supplied string go through the
/// redact-then-bound excerpt first; a 2xx answer contributes its finish reason and usage numbers
/// only.
fn effort_row_from_reply(
    call: &EffortCall,
    wire: Wire,
    reply: Result<Reply, String>,
    api_key: &str,
    elapsed_ms: u128,
) -> EffortRow {
    let row = |status: Option<u16>, verdict: EffortVerdict, detail: String| EffortRow {
        form: call.form,
        value: call.value.clone(),
        role: call.role,
        status,
        finish: None,
        reasoning_tokens: None,
        completion_tokens: None,
        elapsed_ms,
        verdict,
        detail,
    };
    match reply {
        Err(e) => row(None, EffortVerdict::Error, excerpt(&e, api_key)),
        Ok(reply) if (200..300).contains(&reply.status) => {
            let parsed = parse_success(wire, &reply.body, api_key);
            EffortRow {
                finish: parsed.finish.as_deref().map(|f| excerpt(f, api_key)),
                reasoning_tokens: parsed.usage.reasoning,
                completion_tokens: parsed.usage.completion,
                ..row(
                    Some(reply.status),
                    EffortVerdict::Accepted,
                    parsed.note.unwrap_or_else(|| "-".to_string()),
                )
            }
        }
        Ok(reply) => {
            let verdict = if matches!(reply.status, 400 | 422) {
                EffortVerdict::Rejected
            } else {
                EffortVerdict::Error
            };
            let mut detail = excerpt(&reply.body, api_key);
            if (300..400).contains(&reply.status) {
                detail = format!("redirect not followed {detail}")
                    .trim_end()
                    .to_string();
            }
            row(Some(reply.status), verdict, detail)
        }
    }
}

/// Send one effort call and time it.
async fn run_effort_call(
    client: &reqwest::Client,
    spec: &ProviderSpec,
    built: &Built,
    model: &str,
    call: &EffortCall,
) -> EffortRow {
    let url = chat_url(spec.wire, &built.base_url, model);
    let body = effort_body(spec, model, call);
    let request = with_auth(client.post(&url).json(&body), spec.auth, &built.api_key);
    let started = Instant::now();
    let reply = exchange(request, &built.api_key).await;
    effort_row_from_reply(
        call,
        spec.wire,
        reply,
        &built.api_key,
        started.elapsed().as_millis(),
    )
}

/// A count cell: the integer or the absent marker.
fn count_cell(n: Option<i64>) -> String {
    n.map_or_else(|| "-".to_string(), |n| n.to_string())
}

/// The markdown row of one measured call.
fn effort_row_line(index: usize, row: &EffortRow) -> String {
    format!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        index,
        row.form,
        cell(&row.value),
        row.status
            .map_or_else(|| "-".to_string(), |s| s.to_string()),
        row.finish.as_deref().map_or_else(|| "-".to_string(), cell),
        count_cell(row.reasoning_tokens),
        count_cell(row.completion_tokens),
        row.elapsed_ms,
        row.verdict.label(),
        row.detail
    )
}

const EFFORT_TABLE_HEADER: &str = "| # | form | value | status | finish | reasoning tokens | completion tokens | ms | verdict | detail |\n\
     |---|------|-------|--------|--------|------------------|-------------------|----|---------|--------|";

/// The per-form summary lines of one model's rows: whether the vendor parses the form's field
/// (its invalid value was refused), the effect check (reasoning tokens at `low` against the
/// highest level probed, or at each budget), and the detector verdict. Computed from the rows
/// only. A 2xx answer to the detector's out-of-vocabulary value is `field not parsed (silent)`.
fn form_summary_lines(rows: &[EffortRow]) -> Vec<String> {
    let mut forms: Vec<&'static str> = Vec::new();
    for row in rows {
        if row.form != "default" && row.form != "detector" && !forms.contains(&row.form) {
            forms.push(row.form);
        }
    }
    let mut lines: Vec<String> = forms
        .into_iter()
        .map(|form| {
            let in_form = || rows.iter().filter(|r| r.form == form);
            let parsed = match in_form().find(|r| r.role == EffortRole::Invalid) {
                Some(r) => match r.verdict {
                    EffortVerdict::Rejected => "yes",
                    EffortVerdict::Accepted => "no",
                    EffortVerdict::Error => "unknown",
                },
                None => "unknown",
            };
            let tokens_at = |value: &str| {
                in_form()
                    .find(|r| r.role == EffortRole::Level && r.value == value)
                    .and_then(|r| r.reasoning_tokens)
            };
            let has_levels = in_form().any(|r| r.role == EffortRole::Level);
            let budgets: Vec<String> = in_form()
                .filter(|r| r.role == EffortRole::Budget)
                .map(|r| format!("{}={}", r.value, count_cell(r.reasoning_tokens)))
                .collect();
            let effect = if has_levels {
                let top = ["max", "xhigh", "high"]
                    .into_iter()
                    .find(|v| in_form().any(|r| r.role == EffortRole::Level && r.value == *v))
                    .unwrap_or("max");
                format!(
                    "; effect: low={} {top}={}",
                    count_cell(tokens_at("low")),
                    count_cell(tokens_at(top))
                )
            } else if budgets.is_empty() {
                String::new()
            } else {
                format!("; budget: {}", budgets.join(" "))
            };
            format!("- `{form}`: field parsed: {parsed}{effect}")
        })
        .collect();
    // The detector: its own call where the vendor's candidate form is not the top-level field,
    // otherwise the top-level form's invalid value.
    let detector = rows
        .iter()
        .find(|r| r.role == EffortRole::Detector)
        .or_else(|| {
            rows.iter()
                .find(|r| r.form == "top_level_effort" && r.role == EffortRole::Invalid)
        });
    if let Some(row) = detector {
        let verdict = match row.verdict {
            EffortVerdict::Accepted => "field not parsed (silent)",
            EffortVerdict::Rejected => "field parsed (rejected)",
            EffortVerdict::Error => "unknown (the call errored)",
        };
        lines.push(format!("- detector: {verdict}"));
    }
    lines
}

/// Header facts of a provider's `--effort` section.
fn write_effort_section_facts(out: &mut String, spec: &ProviderSpec, base_url: &str, model: &str) {
    let cap = match spec.wire {
        Wire::OpenAiChat => spec.cap_field,
        Wire::AnthropicMessages => "max_tokens",
        Wire::GeminiGenerate => "generationConfig.maxOutputTokens",
    };
    let _ = writeln!(out, "\n- base URL: `{}`", redact_url(base_url));
    let _ = writeln!(
        out,
        "- chat endpoint: `POST {}`; credential header `{}` (name only, value never printed)",
        redact_url(&chat_url(spec.wire, base_url, model)),
        spec.auth.header_name()
    );
    let _ = writeln!(
        out,
        "- every call carries `{cap}` = {EFFORT_MAX_TOKENS}, a {PROBE_TIMEOUT_SECS} s timeout, \
         no acceptance extra; redirects are never followed"
    );
}

/// The offline `--effort` plan: every planned call for all nine providers, with no network access
/// and no credential read. Only the `--models` ids are candidates.
fn render_effort_dry_run(models: &[(String, Vec<String>)]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Live vendor effort probe plan (--effort --dry-run)\n"
    );
    let _ = writeln!(
        out,
        "- no network access was made and no credential was read"
    );
    let _ = writeln!(out, "- prompt: \"{USAGE_PROMPT}\"");
    let _ = writeln!(
        out,
        "- invalid value: `{EFFORT_INVALID_VALUE}`; output cap {EFFORT_MAX_TOKENS} tokens per call"
    );
    let mut total_calls = 0usize;
    for spec in PROVIDERS.iter() {
        let _ = writeln!(out, "\n## {}", spec.name);
        let base_url = spec.default_base_url.map_or_else(
            || "<OPENAI_COMPATIBLE_BASE_URL>".to_string(),
            str::to_string,
        );
        write_effort_section_facts(&mut out, spec, &base_url, spec.default_model);
        let _ = writeln!(
            out,
            "- credential variables required: {}",
            spec.cred_vars
                .iter()
                .map(|v| format!("`{v}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let plan = effort_plan(spec);
        let candidates = effort_candidates(&extra_models_for(models, spec));
        let _ = writeln!(out, "\n### Candidate models\n");
        if candidates.is_empty() {
            let _ = writeln!(
                out,
                "- none (pass `--models {}=<id>[,<id>...]`; the effort modes probe no default)",
                spec.name.to_ascii_lowercase()
            );
        }
        for model in &candidates {
            let _ = writeln!(out, "- `{model}`: {} calls", plan.len());
            total_calls += plan.len();
        }
        let _ = writeln!(out, "\n### Planned calls (run once per candidate model)\n");
        if plan.is_empty() {
            let _ = writeln!(out, "- no effort plan is defined for this provider");
            continue;
        }
        let _ = writeln!(
            out,
            "| # | form | value | role | body extras |\n|---|------|-------|------|-------------|"
        );
        for (i, call) in plan.iter().enumerate() {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {:?} | {} |",
                i + 1,
                call.form,
                cell(&call.value),
                call.role,
                if call.extra.is_null() {
                    "-".to_string()
                } else {
                    format!("`{}`", cell(&call.extra.to_string()))
                }
            );
        }
    }
    let _ = writeln!(out, "\n- total planned calls: {total_calls}");
    out
}

async fn effort_provider(
    client: &reqwest::Client,
    spec: &ProviderSpec,
    options: &Options,
) -> ProbeEnd {
    let candidates = effort_candidates(&extra_models_for(&options.models, spec));
    let plan = effort_plan(spec);
    if plan.is_empty() {
        println!("\nNOT MEASURED: no effort plan is defined for this provider");
        return ProbeEnd::NotBuilt;
    }
    let built = match (spec.build)() {
        Ok(b) => b,
        Err(e) => {
            println!(
                "\nNOT MEASURED: the adapter could not be built from the environment: {}",
                excerpt(&e, "")
            );
            return ProbeEnd::NotBuilt;
        }
    };
    let mut facts = String::new();
    write_effort_section_facts(&mut facts, spec, &built.base_url, "{model}");
    print!("{facts}");
    let mut reached = false;
    for model in candidates {
        println!("\n### {model}\n");
        println!("{EFFORT_TABLE_HEADER}");
        let mut rows: Vec<EffortRow> = Vec::new();
        for (i, call) in plan.iter().enumerate() {
            let row = run_effort_call(client, spec, &built, &model, call).await;
            if row.status.is_some() {
                reached = true;
            }
            println!("{}", effort_row_line(i + 1, &row));
            rows.push(row);
        }
        println!();
        for line in form_summary_lines(&rows) {
            println!("{line}");
        }
    }
    if reached {
        ProbeEnd::Reached
    } else {
        ProbeEnd::Unreachable
    }
}

async fn run_effort(options: &Options) -> i32 {
    let client = match build_probe_client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let lookup = |name: &str| std::env::var(name).ok();
    print_run_header("Live vendor effort probe", &lookup);
    println!(
        "- prompt: \"{USAGE_PROMPT}\"; every call capped at {EFFORT_MAX_TOKENS} output tokens \
         (numbers and verdicts are recorded; response content is never printed)"
    );

    let (mut configured, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    for spec in PROVIDERS.iter() {
        println!("\n## {}", spec.name);
        if let Some(var) = missing_credential(spec, &lookup) {
            println!("\nSKIPPED ({var} not set)");
            skipped += 1;
            continue;
        }
        if effort_candidates(&extra_models_for(&options.models, spec)).is_empty() {
            println!("\nNOT PROBED: no `--models` candidate for this provider");
            skipped += 1;
            continue;
        }
        configured += 1;
        match effort_provider(&client, spec, options).await {
            ProbeEnd::Reached => {}
            ProbeEnd::Unreachable => {
                println!("\nUNREACHABLE: no HTTP exchange with this provider completed");
                failed += 1;
            }
            ProbeEnd::NotBuilt => failed += 1,
        }
    }

    println!("\n---\n");
    println!(
        "{configured} of {} providers probed ({skipped} not configured or without candidates); \
         {failed} unreachable or not buildable",
        PROVIDERS.len()
    );
    if configured == 0 {
        eprintln!("no provider was configured with a candidate -- nothing was tested");
    }
    smoke_exit_code(configured, failed)
}

// ---------------------------------------------------------------------------------------------
// Effort smoke (`--effort-smoke`): the shipped adapters' effort path (REASON-03)
// ---------------------------------------------------------------------------------------------

/// The optional caps an `--effort-smoke` run applies to each adapter's config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SmokeKnobs {
    /// `--max-tokens N`: the config's output budget.
    max_tokens: Option<u32>,
    /// `--timeout-seconds S`: the config's request timeout.
    timeout_seconds: Option<u64>,
}

/// Build one provider's adapter exactly as a consumer would -- `*Config::from_env()` -- with the
/// optional caps applied through the shipped `with_max_tokens` / `with_timeout_seconds` builders.
fn build_smoke_port(name: &str, knobs: SmokeKnobs) -> Result<Box<dyn LlmPort>, String> {
    macro_rules! smoke_port {
        ($config:ty, $adapter:ty) => {{
            let mut config = <$config>::from_env()?;
            if let Some(max_tokens) = knobs.max_tokens {
                config = config.with_max_tokens(max_tokens);
            }
            if let Some(timeout_seconds) = knobs.timeout_seconds {
                config = config.with_timeout_seconds(timeout_seconds);
            }
            let adapter = <$adapter>::new(config).map_err(|e| e.to_string())?;
            Ok(Box::new(adapter) as Box<dyn LlmPort>)
        }};
    }
    match name {
        "OpenAI" => smoke_port!(OpenAIConfig, OpenAIAdapter),
        "Anthropic" => smoke_port!(AnthropicConfig, AnthropicAdapter),
        "DeepSeek" => smoke_port!(DeepSeekConfig, DeepSeekAdapter),
        "Kimi" => smoke_port!(KimiConfig, KimiAdapter),
        "Qwen" => smoke_port!(QwenConfig, QwenAdapter),
        "Grok" => smoke_port!(GrokConfig, GrokAdapter),
        "Gemini" => smoke_port!(GeminiConfig, GeminiAdapter),
        "Ollama" => smoke_port!(OllamaConfig, OllamaAdapter),
        "OpenAI-compatible" => smoke_port!(OpenAiCompatibleConfig, OpenAiCompatibleAdapter),
        other => Err(format!("no adapter builder for provider `{other}`")),
    }
}

/// Which levels an adapter's own profile lets the smoke send.
#[derive(Debug, PartialEq, Eq)]
enum SmokeSelection {
    /// The lowest and highest admitted level (one entry when only one level is listed).
    Send(Vec<ReasoningEffort>),
    /// Nothing is sent; the text says which state the profile is in.
    Nothing(String),
}

/// Pick the levels to send from `profile.effort`: the lowest and the highest level the model
/// lists. Any other state sends nothing and says so.
fn smoke_selection(profile: Option<&ModelProfile>) -> SmokeSelection {
    let Some(profile) = profile else {
        return SmokeSelection::Nothing("no model profile".to_string());
    };
    match &profile.effort {
        EffortSupport::Supported(levels) => match (levels.levels.first(), levels.levels.last()) {
            (Some(lowest), Some(highest)) if lowest == highest => {
                SmokeSelection::Send(vec![*lowest])
            }
            (Some(lowest), Some(highest)) => SmokeSelection::Send(vec![*lowest, *highest]),
            _ => SmokeSelection::Nothing("the profile lists no levels".to_string()),
        },
        EffortSupport::Unmeasured => SmokeSelection::Nothing("effort unmeasured".to_string()),
        EffortSupport::WireUnsupported => {
            SmokeSelection::Nothing("effort wire unsupported".to_string())
        }
        EffortSupport::NoReasoning => SmokeSelection::Nothing("model has no reasoning".to_string()),
        _ => SmokeSelection::Nothing("effort support state not recognised".to_string()),
    }
}

/// One adapter-path request, or the statement that nothing was sent. Numbers, labels and the two
/// metadata values only -- never the response content.
#[derive(Clone, Debug, PartialEq)]
struct SmokeRow {
    model: String,
    /// The level sent, or `-` when nothing was sent.
    requested: String,
    /// `Ok`, `UnsupportedParameter: <reason>`, `error: <excerpt>` or `not sent: <why>`.
    outcome: String,
    finish: Option<String>,
    reasoning_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    /// The `paladin.reasoning_requested` metadata value, `-` when absent.
    meta_requested: String,
    /// The `paladin.reasoning_effective` metadata value, `-` when absent.
    meta_effective: String,
    elapsed_ms: u128,
}

/// A finish reason as a fixed label; `Error(text)` never exposes its text.
fn finish_label(finish: &FinishReason) -> &'static str {
    match finish {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::ContentFilter => "content_filter",
        FinishReason::FunctionCall => "function_call",
        FinishReason::Error(_) => "error",
    }
}

/// Send the lowest and highest admitted level through `port` for one model.
async fn smoke_model(port: &dyn LlmPort, model: &str, api_key: &str) -> Vec<SmokeRow> {
    let blank = |requested: &str, outcome: String| SmokeRow {
        model: model.to_string(),
        requested: requested.to_string(),
        outcome,
        finish: None,
        reasoning_tokens: None,
        completion_tokens: None,
        meta_requested: "-".to_string(),
        meta_effective: "-".to_string(),
        elapsed_ms: 0,
    };
    let profile = port.model_profile(model);
    let levels = match smoke_selection(profile.as_ref()) {
        SmokeSelection::Send(levels) => levels,
        SmokeSelection::Nothing(why) => return vec![blank("-", format!("not sent: {why}"))],
    };
    let mut rows = Vec::new();
    for level in levels {
        let prompt = match PromptItem::new(PromptType::User(UserPrompt {
            query: USAGE_PROMPT.to_string(),
            context: None,
        })) {
            Ok(prompt) => prompt,
            Err(e) => {
                rows.push(blank(
                    &level.to_string(),
                    format!(
                        "error: could not construct the prompt: {}",
                        excerpt(&e.to_string(), api_key)
                    ),
                ));
                continue;
            }
        };
        let request = LlmRequest::new(model.to_string(), prompt).with_reasoning(level);
        let started = Instant::now();
        let result = port.generate(request).await;
        let elapsed_ms = started.elapsed().as_millis();
        let mut row = blank(&level.to_string(), String::new());
        row.elapsed_ms = elapsed_ms;
        match result {
            Ok(response) => {
                row.outcome = "Ok".to_string();
                row.finish = Some(finish_label(&response.finish_reason).to_string());
                row.reasoning_tokens = response.usage.reasoning_tokens;
                row.completion_tokens = Some(response.usage.completion_tokens);
                let meta = |key: &str| {
                    response
                        .metadata
                        .get(key)
                        .map_or_else(|| "-".to_string(), |v| excerpt(v, api_key))
                };
                row.meta_requested = meta(REASONING_REQUESTED_METADATA_KEY);
                row.meta_effective = meta(REASONING_EFFECTIVE_METADATA_KEY);
            }
            Err(LlmError::UnsupportedParameter { reason, .. }) => {
                row.outcome = format!("UnsupportedParameter: {}", excerpt(&reason, api_key));
            }
            Err(other) => {
                row.outcome = format!("error: {}", excerpt(&other.to_string(), api_key));
            }
        }
        rows.push(row);
    }
    rows
}

const SMOKE_TABLE_HEADER: &str = "| model | requested | outcome | finish | reasoning tokens | completion tokens | paladin.reasoning_requested | paladin.reasoning_effective | ms |\n\
     |-------|-----------|---------|--------|------------------|-------------------|-----------------------------|-----------------------------|----|";

/// The markdown row of one smoke request.
fn smoke_row_line(model: &str, row: &SmokeRow) -> String {
    format!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        cell(model),
        cell(&row.requested),
        cell(&row.outcome),
        row.finish.as_deref().map_or_else(|| "-".to_string(), cell),
        count_cell(row.reasoning_tokens.map(i64::from)),
        count_cell(row.completion_tokens.map(i64::from)),
        cell(&row.meta_requested),
        cell(&row.meta_effective),
        row.elapsed_ms
    )
}

/// The offline `--effort-smoke` plan: for every `--models` candidate, the two adapter calls a run
/// would make. No adapter is built, so no credential is read; the levels themselves come from the
/// adapter's own profile at run time.
fn render_effort_smoke_dry_run(
    models: &[(String, Vec<String>)],
    max_tokens: Option<u32>,
    timeout_seconds: Option<u64>,
) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Live vendor effort smoke plan (--effort-smoke --dry-run)\n"
    );
    let _ = writeln!(
        out,
        "- no network access was made and no credential was read"
    );
    let _ = writeln!(out, "- prompt: \"{USAGE_PROMPT}\"");
    let _ = writeln!(
        out,
        "- output budget: {}; timeout: {}",
        max_tokens.map_or_else(
            || "the adapter's own default".to_string(),
            |n| format!("{n} tokens")
        ),
        timeout_seconds.map_or_else(
            || "the adapter's own default".to_string(),
            |s| format!("{s} s")
        )
    );
    let mut total_calls = 0usize;
    for spec in PROVIDERS.iter() {
        let _ = writeln!(out, "\n## {}", spec.name);
        let _ = writeln!(
            out,
            "- credential variables required: {}",
            spec.cred_vars
                .iter()
                .map(|v| format!("`{v}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let candidates = effort_candidates(&extra_models_for(models, spec));
        let _ = writeln!(out, "\n### Planned adapter calls\n");
        if candidates.is_empty() {
            let _ = writeln!(
                out,
                "- none (pass `--models {}=<id>[,<id>...]`; the effort modes probe no default)",
                spec.name.to_ascii_lowercase()
            );
        }
        for model in &candidates {
            let _ = writeln!(
                out,
                "- `{model}`: 2 adapter calls (`generate` through the shipped adapter with \
                 `LlmRequest::with_reasoning` at the lowest and the highest level \
                 `model_profile(model).effort` admits, read at run time; a model whose profile \
                 admits no level sends nothing)"
            );
            total_calls += 2;
        }
    }
    let _ = writeln!(out, "\n- total planned adapter calls: {total_calls}");
    out
}

async fn effort_smoke_provider(spec: &ProviderSpec, options: &Options) -> ProbeEnd {
    let knobs = SmokeKnobs {
        max_tokens: options.max_tokens,
        timeout_seconds: options.timeout_seconds,
    };
    let port = match build_smoke_port(spec.name, knobs) {
        Ok(port) => port,
        Err(e) => {
            println!(
                "\nNOT MEASURED: the adapter could not be built from the environment: {}",
                excerpt(&e, "")
            );
            return ProbeEnd::NotBuilt;
        }
    };
    // The credential, held only to redact it from any printed text.
    let api_key = std::env::var(spec.cred_vars[0]).unwrap_or_default();
    println!("\n{SMOKE_TABLE_HEADER}");
    let mut reached = false;
    for model in effort_candidates(&extra_models_for(&options.models, spec)) {
        for row in smoke_model(port.as_ref(), &model, &api_key).await {
            if row.outcome == "Ok" || row.outcome.starts_with("UnsupportedParameter") {
                reached = true;
            }
            println!("{}", smoke_row_line(&model, &row));
        }
    }
    if reached {
        ProbeEnd::Reached
    } else {
        ProbeEnd::Unreachable
    }
}

async fn run_effort_smoke(options: &Options) -> i32 {
    let lookup = |name: &str| std::env::var(name).ok();
    print_run_header("Live vendor effort smoke (adapter path)", &lookup);
    println!(
        "- prompt: \"{USAGE_PROMPT}\"; two requests per candidate through the shipped adapter \
         (numbers and metadata values are recorded; response content is never printed)"
    );

    let (mut configured, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    for spec in PROVIDERS.iter() {
        println!("\n## {}", spec.name);
        if let Some(var) = missing_credential(spec, &lookup) {
            println!("\nSKIPPED ({var} not set)");
            skipped += 1;
            continue;
        }
        if effort_candidates(&extra_models_for(&options.models, spec)).is_empty() {
            println!("\nNOT PROBED: no `--models` candidate for this provider");
            skipped += 1;
            continue;
        }
        configured += 1;
        match effort_smoke_provider(spec, options).await {
            ProbeEnd::Reached => {}
            ProbeEnd::Unreachable => {
                println!("\nNOTHING SENT OR ANSWERED: no request produced an `Ok` or a refusal");
                failed += 1;
            }
            ProbeEnd::NotBuilt => failed += 1,
        }
    }

    println!("\n---\n");
    println!(
        "{configured} of {} providers smoked ({skipped} not configured or without candidates); \
         {failed} without an answer or not buildable",
        PROVIDERS.len()
    );
    if configured == 0 {
        eprintln!("no provider was configured with a candidate -- nothing was tested");
    }
    smoke_exit_code(configured, failed)
}

#[tokio::main]
async fn main() {
    // `set_logger` (not `set_boxed_logger`) is used deliberately: the latter requires `log`'s
    // `std` feature, which this workspace does not enable for `log = { workspace = true }`.
    static LOGGER: StderrLogger = StderrLogger;
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Warn);
    }

    let command = match parse_args(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            std::process::exit(2);
        }
    };
    match command {
        Command::Help => println!("{USAGE}"),
        Command::Run(options) if options.dry_run && options.effort_smoke => {
            print!(
                "{}",
                render_effort_smoke_dry_run(
                    &options.models,
                    options.max_tokens,
                    options.timeout_seconds
                )
            );
        }
        Command::Run(options) if options.dry_run && options.effort => {
            print!("{}", render_effort_dry_run(&options.models));
        }
        Command::Run(options) if options.dry_run && options.usage => {
            print!("{}", render_usage_dry_run(&options.models));
        }
        Command::Run(options) if options.dry_run => {
            print!("{}", render_dry_run(&options.models));
        }
        Command::Run(options) if options.effort => std::process::exit(run_effort(&options).await),
        Command::Run(options) if options.effort_smoke => {
            std::process::exit(run_effort_smoke(&options).await)
        }
        Command::Run(options) if options.usage => std::process::exit(run_usage(&options).await),
        Command::Run(options) if options.probe => std::process::exit(run_probe(&options).await),
        Command::Run(_) => std::process::exit(run_smoke().await),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_openai_default_candidate_is_the_first_seeded_model() {
        // The plain smoke must drive the model whose rejection of the old output-cap key it
        // once missed (G-50-5b); `build_openai` and the table both read this one constant.
        assert_eq!(OPENAI_DEFAULT_MODEL, "gpt-5.4-nano");
        let spec = PROVIDERS.iter().find(|p| p.name == "OpenAI").unwrap();
        assert_eq!(spec.default_model, OPENAI_DEFAULT_MODEL);
    }

    #[test]
    fn the_table_has_nine_distinctly_named_providers() {
        let mut names: Vec<&str> = PROVIDERS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 9);
        assert!(PROVIDERS.iter().all(|p| !p.cred_vars.is_empty()));
    }

    #[test]
    fn an_absent_credential_is_named_and_never_a_pass() {
        let none = |_: &str| None;
        for spec in PROVIDERS.iter() {
            assert_eq!(missing_credential(spec, &none), Some(spec.cred_vars[0]));
        }
    }

    #[test]
    fn an_empty_credential_counts_as_absent() {
        let blank = |_: &str| Some("  ".to_string());
        assert_eq!(
            missing_credential(&PROVIDERS[0], &blank),
            Some("OPENAI_API_KEY")
        );
    }

    #[test]
    fn openai_compatible_needs_both_its_key_and_base_url() {
        let spec = PROVIDERS
            .iter()
            .find(|p| p.name == "OpenAI-compatible")
            .expect("table row present");
        let only_key = |n: &str| (n == "OPENAI_COMPATIBLE_API_KEY").then(|| "k".to_string());
        assert_eq!(
            missing_credential(spec, &only_key),
            Some("OPENAI_COMPATIBLE_BASE_URL")
        );
        let both = |_: &str| Some("v".to_string());
        assert_eq!(missing_credential(spec, &both), None);
    }

    #[test]
    fn zero_configured_providers_is_a_failure_not_a_vacuous_pass() {
        assert_eq!(smoke_exit_code(0, 0), 1);
        assert_eq!(smoke_exit_code(3, 1), 1);
        assert_eq!(smoke_exit_code(3, 0), 0);
    }

    #[test]
    fn an_unknown_flag_is_an_error_naming_it() {
        let err = parse_args(args(&["--bogus"])).unwrap_err();
        assert!(err.contains("--bogus"), "{err}");
    }

    #[test]
    fn no_flag_is_a_smoke_run_and_help_is_recognised() {
        assert_eq!(parse_args(args(&[])), Ok(Command::Run(Options::default())));
        assert_eq!(parse_args(args(&["--help"])), Ok(Command::Help));
    }

    #[test]
    fn classify_flags_a_list_identical_to_the_curated_fallback() {
        let live = classify(vec!["a".to_string(), "b".to_string()], &["a", "b"], "a");
        assert!(live.is_fallback);
        let live = classify(vec!["a".to_string(), "c".to_string()], &["a", "b"], "a");
        assert!(!live.is_fallback);
        assert!(live.default_present);
    }

    #[test]
    fn the_static_model_lists_match_the_adapters() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");
        let anthropic = AnthropicAdapter::new(
            AnthropicConfig::new("k".to_string(), ANTHROPIC_DEFAULT_MODEL.to_string())
                .with_base_url("https://example.invalid/v1".to_string())
                .with_max_tokens(16),
        )
        .expect("anthropic adapter");
        let deepseek = DeepSeekAdapter::new(
            DeepSeekConfig::new("k".to_string(), DEEPSEEK_DEFAULT_MODEL.to_string())
                .with_base_url("https://example.invalid/v1"),
        )
        .expect("deepseek adapter");
        let a = rt
            .block_on(anthropic.get_available_models())
            .expect("anthropic list");
        let d = rt
            .block_on(deepseek.get_available_models())
            .expect("deepseek list");
        assert_eq!(a, ANTHROPIC_MODELS);
        assert_eq!(d, DEEPSEEK_MODELS);
    }

    fn spec(name: &str) -> &'static ProviderSpec {
        PROVIDERS
            .iter()
            .find(|p| p.name == name)
            .expect("table row present")
    }

    #[test]
    fn probe_flags_parse_and_models_is_repeatable() {
        let parsed = parse_args(args(&[
            "--probe",
            "--models",
            "kimi=a,b",
            "--models=Kimi=c",
            "--dry-run",
        ]));
        assert_eq!(
            parsed,
            Ok(Command::Run(Options {
                probe: true,
                dry_run: true,
                models: vec![
                    ("Kimi".to_string(), vec!["a".to_string(), "b".to_string()]),
                    ("Kimi".to_string(), vec!["c".to_string()]),
                ],
                ..Options::default()
            }))
        );
    }

    #[test]
    fn models_flag_rejects_an_unknown_provider_a_bad_id_and_a_missing_probe() {
        let err = parse_args(args(&["--probe", "--models", "nope=a"])).unwrap_err();
        assert!(err.contains("nope"), "{err}");
        let err = parse_args(args(&["--probe", "--models", "kimi=../x"])).unwrap_err();
        assert!(err.contains("../x"), "{err}");
        let err = parse_args(args(&["--models", "kimi=a"])).unwrap_err();
        assert!(err.contains("--probe"), "{err}");
        let err = parse_args(args(&["--probe", "--models"])).unwrap_err();
        assert!(err.contains("--models"), "{err}");
    }

    #[test]
    fn temperature_points_measure_the_range_endpoints_and_the_next_value_outside() {
        let values = |range| -> Vec<f64> {
            temperature_points(range)
                .into_iter()
                .map(|(v, _)| v)
                .collect()
        };
        // ADR-0004 A4: Qwen's half-open [0, 2) is probed at 0, 1.99, 0.5 and the excluded 2.0.
        assert_eq!(values(Some((0.0, 1.99))), vec![0.0, 1.99, 0.5, 2.0]);
        assert_eq!(values(Some((0.0, 2.0))), vec![0.0, 2.0, 0.5, 2.01]);
        // Kimi's fixed 1.0 merges the coinciding ends and keeps both roles.
        let kimi = temperature_points(Some((1.0, 1.0)));
        assert_eq!(kimi.len(), 3);
        assert!(kimi[0].1.contains("low end") && kimi[0].1.contains("high end"));
        assert_eq!(values(None), vec![0.0, 2.0, 0.5, 2.01]);
    }

    #[test]
    fn every_provider_plan_lists_the_json_schema_probe_and_anthropic_marks_it_na() {
        for p in PROVIDERS.iter() {
            let plan = probe_plan(p);
            let schema = plan
                .iter()
                .find(|x| x.kind == ProbeKind::JsonSchema)
                .expect("json_schema probe planned");
            assert!(schema.label.contains("json_schema"));
            assert_eq!(
                schema.not_applicable.is_some(),
                p.wire == Wire::AnthropicMessages,
                "{}",
                p.name
            );
        }
    }

    #[test]
    fn penalty_probes_are_openai_chat_only_and_kimi_gets_a_default_thinking_probe() {
        let plan = probe_plan(spec("Gemini"));
        let fp = plan
            .iter()
            .find(|x| x.kind == ProbeKind::FrequencyPenalty)
            .expect("planned");
        assert!(fp.not_applicable.is_some());
        let kimi = probe_plan(spec("Kimi"));
        assert!(kimi.iter().any(|x| x.kind == ProbeKind::DefaultThinking));
        assert!(
            !probe_plan(spec("Qwen"))
                .iter()
                .any(|x| x.kind == ProbeKind::DefaultThinking)
        );
    }

    #[test]
    fn request_bodies_put_each_parameter_on_its_own_wire_field() {
        let openai = request_body(spec("Qwen"), "m", ProbeKind::StopFive);
        assert_eq!(openai["stop"].as_array().map(Vec::len), Some(5));
        assert_eq!(openai["max_tokens"], json!(PROBE_MAX_TOKENS));
        let anthropic = request_body(spec("Anthropic"), "m", ProbeKind::StopOne);
        assert_eq!(anthropic["stop_sequences"], json!(["END"]));
        let gemini = request_body(spec("Gemini"), "m", ProbeKind::Temperature(1.5));
        assert_eq!(gemini["generationConfig"]["temperature"], json!(1.5));
        assert_eq!(
            gemini["generationConfig"]["maxOutputTokens"],
            json!(PROBE_MAX_TOKENS)
        );
        let schema = request_body(spec("Gemini"), "m", ProbeKind::JsonSchema);
        assert_eq!(
            schema["generationConfig"]["responseMimeType"],
            json!("application/json")
        );
        assert!(schema["generationConfig"]["responseSchema"].is_object());
        let chat_schema = request_body(spec("Grok"), "m", ProbeKind::JsonSchema);
        assert_eq!(chat_schema["response_format"]["type"], json!("json_schema"));
        let object = request_body(spec("Grok"), "m", ProbeKind::JsonObject);
        assert_eq!(object["response_format"]["type"], json!("json_object"));
        assert!(
            object["messages"][0]["content"]
                .as_str()
                .is_some_and(|c| c.contains("JSON"))
        );
    }

    #[test]
    fn the_max_tokens_probe_replaces_the_baseline_cap_instead_of_doubling_it() {
        let body = request_body(spec("OpenAI"), "m", ProbeKind::MaxTokens);
        assert_eq!(body["max_tokens"], json!(64));
        assert!(body.get("max_completion_tokens").is_none());
        let body = request_body(spec("OpenAI"), "m", ProbeKind::MaxCompletionTokens);
        assert_eq!(body["max_completion_tokens"], json!(64));
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn moonshot_thinking_is_disabled_everywhere_except_the_default_thinking_probe() {
        let kimi = spec("Kimi");
        let disabled = request_body(kimi, "m", ProbeKind::Default);
        assert_eq!(disabled["thinking"]["type"], json!("disabled"));
        let top_p = request_body(kimi, "m", ProbeKind::TopP);
        assert_eq!(top_p["thinking"]["type"], json!("disabled"));
        let thinking = request_body(kimi, "m", ProbeKind::DefaultThinking);
        assert!(thinking.get("thinking").is_none());
        assert!(
            request_body(spec("Qwen"), "m", ProbeKind::Default)
                .get("thinking")
                .is_none()
        );
    }

    #[test]
    fn usage_field_names_are_flattened_without_values() {
        let usage = json!({
            "prompt_tokens": 3,
            "prompt_tokens_details": {"cached_tokens": 0},
            "completion_tokens_details": {"reasoning_tokens": 7},
        });
        let mut names = Vec::new();
        flatten_field_names(&usage, "", &mut names);
        names.sort();
        assert_eq!(
            names,
            vec![
                "completion_tokens_details.reasoning_tokens",
                "prompt_tokens",
                "prompt_tokens_details.cached_tokens",
            ]
        );
    }

    #[test]
    fn responses_are_parsed_per_wire() {
        let chat = r#"{"choices":[{"finish_reason":"length","message":{"content":"","reasoning_content":"thinking"}}],"usage":{"total_tokens":1}}"#;
        let p = parse_success(Wire::OpenAiChat, chat, "k");
        assert_eq!(p.finish.as_deref(), Some("length"));
        assert!(!p.has_content && p.has_reasoning);
        let anthropic = r#"{"stop_reason":"end_turn","content":[{"type":"thinking","thinking":"x"},{"type":"text","text":"ping"}],"usage":{"cache_creation_input_tokens":0}}"#;
        let p = parse_success(Wire::AnthropicMessages, anthropic, "k");
        assert!(p.has_content && p.has_reasoning);
        assert_eq!(p.usage_fields, vec!["cache_creation_input_tokens"]);
        let gemini = r#"{"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"hm","thought":true},{"text":"ping"}]}}],"usageMetadata":{"thoughtsTokenCount":4}}"#;
        let p = parse_success(Wire::GeminiGenerate, gemini, "k");
        assert!(p.has_content && p.has_reasoning);
        assert_eq!(p.usage_fields, vec!["thoughtsTokenCount"]);
    }

    #[test]
    fn quota_shaped_errors_are_recognised_by_status_type_or_code() {
        let quota = error_shape(
            429,
            r#"{"error":{"type":"exceeded_current_quota_error","code":"x"}}"#,
        );
        assert!(quota.quota_shaped);
        assert_eq!(quota.kind.as_deref(), Some("exceeded_current_quota_error"));
        assert!(error_shape(400, r#"{"error":{"status":"RESOURCE_EXHAUSTED"}}"#).quota_shaped);
        assert!(error_shape(403, r#"{"error":{"code":"insufficient_balance"}}"#).quota_shaped);
        assert!(!error_shape(400, r#"{"error":{"type":"invalid_request_error"}}"#).quota_shaped);
        assert!(!error_shape(500, "not json").quota_shaped);
    }

    #[test]
    fn printed_urls_carry_no_userinfo_and_no_credential_query_value() {
        assert_eq!(
            redact_url("https://user:pw@host.example/v1/models?key=abc&pageSize=3#frag"),
            "https://<redacted>@host.example/v1/models?key=<redacted>&pageSize=3"
        );
        assert_eq!(
            redact_url("http://localhost:11434/v1"),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            redact_url("https://h/x?api_key=s&access_token=t&q=1"),
            "https://h/x?api_key=<redacted>&access_token=<redacted>&q=1"
        );
    }

    #[test]
    fn a_printed_body_is_redacted_before_it_is_bounded() {
        // The key straddles the 512-char truncation boundary: bounding first would leave its
        // prefix in the output.
        let key = "zzSECRETKEYVALUE123456";
        let body = format!("{}{key} tail", "x".repeat(500));
        let shown = excerpt(&body, key);
        assert!(!shown.contains("zzSECRET"), "{shown}");
        assert!(shown.contains("[REDACTED]"));
        assert_eq!(cell("a|b\nc"), "a\\|b c");
    }

    #[test]
    fn the_dry_run_lists_all_nine_providers_and_names_no_credential() {
        let plan = render_dry_run(&[("Kimi".to_string(), vec!["extra-model".to_string()])]);
        assert_eq!(plan.lines().filter(|l| l.starts_with("## ")).count(), 9);
        assert!(plan.contains("json_schema"));
        assert!(plan.contains("`extra-model`"));
        assert!(plan.contains("{\"thinking\":{\"type\":\"disabled\"}}"));
    }

    #[test]
    fn candidates_deduplicate_default_curated_and_extra_models() {
        let kimi = spec("Kimi");
        let list = candidates(
            kimi,
            KIMI_DEFAULT_MODEL,
            &["kimi-k2.6".into(), "new".into()],
        );
        assert_eq!(list.iter().filter(|m| *m == "kimi-k2.6").count(), 1);
        assert_eq!(list.last().map(String::as_str), Some("new"));
        assert_eq!(list[0], KIMI_DEFAULT_MODEL);
    }

    /// The verdict for a body on a wire, through the same two functions the run uses.
    fn verdict_of(wire: Wire, body: &str) -> String {
        containment_verdict(wire, &usage_row(wire, body))
    }

    #[test]
    fn containment_verdicts_follow_the_arithmetic() {
        // OpenAI-shaped: total = prompt + completion, reasoning inside completion.
        let inside = r#"{"usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":140,
            "completion_tokens_details":{"reasoning_tokens":15}}}"#;
        assert_eq!(
            verdict_of(Wire::OpenAiChat, inside),
            "reasoning inside completion"
        );
        // The Vertex AI Grok example (debug Evidence 00:35): 663 + 50 + 124 = 837.
        let beside = r#"{"usage":{"prompt_tokens":663,"completion_tokens":50,"total_tokens":837,
            "completion_tokens_details":{"reasoning_tokens":124}}}"#;
        assert_eq!(
            verdict_of(Wire::OpenAiChat, beside),
            "reasoning beside completion"
        );
        let none = r#"{"usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":140}}"#;
        assert_eq!(
            verdict_of(Wire::OpenAiChat, none),
            "indeterminate (no reasoning reported)"
        );
        let zero = r#"{"usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":140,
            "completion_tokens_details":{"reasoning_tokens":0}}}"#;
        assert_eq!(
            verdict_of(Wire::OpenAiChat, zero),
            "indeterminate (no reasoning reported)"
        );
        let over = r#"{"usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":9999,
            "completion_tokens_details":{"reasoning_tokens":15}}}"#;
        assert_eq!(
            verdict_of(Wire::OpenAiChat, over),
            "other (total - (prompt + completion) = 9859)"
        );
        // A total below prompt + completion is a signed difference, not an underflow.
        let under = r#"{"usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":120,
            "completion_tokens_details":{"reasoning_tokens":15}}}"#;
        assert_eq!(
            verdict_of(Wire::OpenAiChat, under),
            "other (total - (prompt + completion) = -20)"
        );
        // Gemini: prompt counts promptTokenCount + toolUsePromptTokenCount, completion counts
        // candidatesTokenCount; 100 + 300 + 20 + 10 thoughts = 430.
        let gemini = r#"{"usageMetadata":{"promptTokenCount":100,"candidatesTokenCount":20,
            "thoughtsTokenCount":10,"toolUsePromptTokenCount":300,"totalTokenCount":430}}"#;
        assert_eq!(
            verdict_of(Wire::GeminiGenerate, gemini),
            "reasoning beside completion"
        );
        let gemini_row = usage_row(Wire::GeminiGenerate, gemini);
        assert_eq!(gemini_row.prompt, Some(400));
        assert_eq!(gemini_row.tool_use_prompt, Some(300));
        // Anthropic sends no vendor total, whatever else it reports.
        let anthropic = r#"{"usage":{"input_tokens":12,"output_tokens":30,
            "cache_read_input_tokens":2,"cache_creation_input_tokens":0,
            "output_tokens_details":{"thinking_tokens":9}}}"#;
        assert_eq!(
            verdict_of(Wire::AnthropicMessages, anthropic),
            "no vendor total"
        );
        let row = usage_row(Wire::AnthropicMessages, anthropic);
        assert_eq!(
            (row.prompt, row.completion, row.reasoning, row.cached),
            (Some(12), Some(30), Some(9), Some(2))
        );
        // No usage object at all, and a usage object missing its counts, are not measured.
        assert_eq!(
            verdict_of(Wire::OpenAiChat, r#"{"choices":[]}"#),
            "not measured (no usage object in the response)"
        );
        assert_eq!(
            verdict_of(Wire::OpenAiChat, "not json"),
            "not measured (no usage object in the response)"
        );
        assert_eq!(
            verdict_of(Wire::OpenAiChat, r#"{"usage":{"total_tokens":5}}"#),
            "not measured (usage lacks prompt or completion tokens)"
        );
        // An OpenAI-shaped body with counts but no total has no vendor total to compare.
        assert_eq!(
            verdict_of(
                Wire::OpenAiChat,
                r#"{"usage":{"prompt_tokens":1,"completion_tokens":2}}"#
            ),
            "no vendor total"
        );
        // Extremes never panic.
        let huge = r#"{"usage":{"prompt_tokens":9223372036854775807,"completion_tokens":9223372036854775807,"total_tokens":1}}"#;
        assert!(verdict_of(Wire::OpenAiChat, huge).starts_with("other ("));
    }

    #[test]
    fn grok_cost_and_sources_are_read_as_numbers() {
        let body = r#"{"usage":{"prompt_tokens":663,"completion_tokens":50,"total_tokens":837,
            "prompt_tokens_details":{"cached_tokens":8,"cache_write_tokens":0},
            "completion_tokens_details":{"reasoning_tokens":124},
            "cost_in_usd_ticks":123456,"num_sources_used":3}}"#;
        let row = usage_row(Wire::OpenAiChat, body);
        assert_eq!(row.cached, Some(8));
        assert_eq!(row.cache_write, Some(0));
        assert_eq!(row.cost_in_usd_ticks, Some(123_456));
        assert_eq!(row.num_sources_used, Some(3));
    }

    #[test]
    fn usage_values_are_numbers_only() {
        // Content, a text-valued "count" and an echoed credential-shaped string are all present
        // in the body; none may reach the rendered row.
        let body = r#"{"choices":[{"message":{"content":"SECRET-ANSWER-TEXT 391"}}],
            "echo":"sk-should-never-print",
            "usage":{"prompt_tokens":"sk-leaked-as-a-count","completion_tokens":40,
                     "total_tokens":140,"cost_in_usd_ticks":"also-text",
                     "completion_tokens_details":{"reasoning_tokens":15}}}"#;
        let row = usage_row(Wire::OpenAiChat, body);
        // A string where a number belongs is absent, never copied.
        assert_eq!(row.prompt, None);
        assert_eq!(row.cost_in_usd_ticks, None);
        assert_eq!(row.completion, Some(40));
        let cells = usage_cells(Wire::OpenAiChat, &row);
        let rendered = cells.join(" | ");
        for forbidden in ["SECRET", "sk-", "also-text", "391"] {
            assert!(!rendered.contains(forbidden), "{rendered}");
        }
        // Every cell but the verdict is an integer or the absent marker.
        for cell in &cells[..cells.len() - 2] {
            assert!(
                cell == "-" || cell.parse::<i64>().is_ok(),
                "non-numeric cell `{cell}`"
            );
        }
    }

    #[test]
    fn usage_flag_is_exclusive_with_probe_and_accepts_models_and_dry_run() {
        let parsed = parse_args(args(&["--usage", "--models", "grok=grok-4.3"]));
        assert_eq!(
            parsed,
            Ok(Command::Run(Options {
                usage: true,
                models: vec![("Grok".to_string(), vec!["grok-4.3".to_string()])],
                ..Options::default()
            }))
        );
        let Ok(Command::Run(options)) = parse_args(args(&["--usage", "--dry-run"])) else {
            panic!("--usage --dry-run must parse");
        };
        assert!(options.usage && options.dry_run && !options.probe);
        let err = parse_args(args(&["--usage", "--probe"])).unwrap_err();
        assert!(err.contains("--usage") && err.contains("--probe"), "{err}");
        let err = parse_args(args(&["--probe", "--usage"])).unwrap_err();
        assert!(err.contains("--usage") && err.contains("--probe"), "{err}");
    }

    #[test]
    fn the_usage_body_sends_no_extras_and_no_sampling_under_a_larger_cap() {
        let kimi = request_body(spec("Kimi"), "kimi-k2.6", ProbeKind::Usage);
        assert!(kimi.get("thinking").is_none(), "{kimi}");
        assert!(kimi.get("temperature").is_none() && kimi.get("top_p").is_none());
        assert_eq!(kimi["max_tokens"], json!(USAGE_MAX_TOKENS));
        assert!(kimi.to_string().contains("17 multiplied by 23"));
        let openai = request_body(spec("OpenAI"), "m", ProbeKind::Usage);
        assert_eq!(openai["max_completion_tokens"], json!(USAGE_MAX_TOKENS));
        assert!(openai.get("max_tokens").is_none());
        let gemini = request_body(spec("Gemini"), "m", ProbeKind::Usage);
        assert_eq!(
            gemini["generationConfig"],
            json!({"maxOutputTokens": USAGE_MAX_TOKENS})
        );
        let anthropic = request_body(spec("Anthropic"), "m", ProbeKind::Usage);
        assert_eq!(anthropic["max_tokens"], json!(USAGE_MAX_TOKENS));
        assert_eq!(USAGE_MAX_TOKENS, 1024);
    }

    #[test]
    fn the_usage_dry_run_lists_all_nine_providers_without_credentials() {
        let plan = render_usage_dry_run(&[("Grok".to_string(), vec!["grok-4.5".to_string()])]);
        assert_eq!(plan.lines().filter(|l| l.starts_with("## ")).count(), 9);
        assert!(plan.contains("`grok-4.5`"));
        assert!(plan.contains("no network access"));
        // The extras that would suppress reasoning are not planned.
        assert!(!plan.contains("thinking"), "{plan}");
    }

    #[test]
    fn usage_candidates_are_the_default_plus_extras_without_duplicates() {
        let grok = spec("Grok");
        let list = usage_candidates(
            grok,
            "grok-4.3",
            &["grok-4.3".into(), "grok-4.5".into(), "grok-4.5".into()],
        );
        assert_eq!(list, vec!["grok-4.3".to_string(), "grok-4.5".to_string()]);
    }

    #[test]
    fn a_call_without_a_usable_response_is_not_measured() {
        let rejected = Outcome::Rejected {
            status: 429,
            detail: "[REDACTED] quota".to_string(),
        };
        let cells = usage_table_cells(Wire::OpenAiChat, &rejected);
        assert!(cells[0].starts_with("REJECTED (429)"), "{cells:?}");
        assert_eq!(cells[5], "not measured (HTTP 429)");
        let down = Outcome::Unreachable("timed out".to_string());
        assert_eq!(
            usage_table_cells(Wire::OpenAiChat, &down)[5],
            "not measured (unreachable)"
        );
        let body = r#"{"usage":{"prompt_tokens":663,"completion_tokens":50,"total_tokens":837,
            "completion_tokens_details":{"reasoning_tokens":124}}}"#;
        let accepted = Outcome::Accepted {
            status: 200,
            parsed: parse_success(Wire::OpenAiChat, body, "k"),
        };
        let cells = usage_table_cells(Wire::OpenAiChat, &accepted);
        assert_eq!(
            cells[1..].to_vec(),
            [
                "663",
                "50",
                "124",
                "837",
                "reasoning beside completion",
                "-"
            ]
        );
    }

    // ----- `--effort` (Phase 51, D-01) -----

    fn effort_options(list: &[&str]) -> Options {
        match parse_args(args(list)) {
            Ok(Command::Run(options)) => options,
            other => panic!("expected a run, got {other:?}"),
        }
    }

    #[test]
    fn effort_flag_parses_with_models_and_dry_run() {
        let options = effort_options(&["--effort"]);
        assert!(options.effort && !options.probe && !options.usage && !options.effort_smoke);
        let options = effort_options(&["--effort", "--dry-run", "--models", "kimi=kimi-k3"]);
        assert!(options.effort && options.dry_run);
        assert_eq!(
            options.models,
            vec![("Kimi".to_string(), vec!["kimi-k3".to_string()])]
        );
    }

    #[test]
    fn effort_is_mutually_exclusive_with_every_other_mode_and_names_both_flags() {
        for (list, other) in [
            (["--effort", "--probe"], "--probe"),
            (["--probe", "--effort"], "--probe"),
            (["--effort", "--usage"], "--usage"),
            (["--usage", "--effort"], "--usage"),
            (["--effort", "--effort-smoke"], "--effort-smoke"),
            (["--effort-smoke", "--effort"], "--effort-smoke"),
        ] {
            let err = parse_args(args(&list)).unwrap_err();
            assert!(err.contains("--effort"), "{err}");
            assert!(err.contains(other), "{err}");
            assert!(err.contains("mutually exclusive"), "{err}");
        }
    }

    #[test]
    fn models_still_needs_a_mode_or_a_dry_run() {
        let err = parse_args(args(&["--models", "kimi=a"])).unwrap_err();
        assert!(err.contains("--effort"), "{err}");
        assert!(parse_args(args(&["--effort", "--models", "kimi=a"])).is_ok());
    }

    /// The `(form, value)` pairs of a plan, in order.
    fn plan_pairs(plan: &[EffortCall]) -> Vec<(&'static str, String)> {
        plan.iter().map(|c| (c.form, c.value.clone())).collect()
    }

    #[test]
    fn the_kimi_effort_plan_probes_both_forms_in_the_documented_order() {
        let plan = effort_plan(spec("Kimi"));
        let mut expected: Vec<(&'static str, String)> = vec![("default", "-".to_string())];
        for level in ["minimal", "low", "medium", "high", "xhigh", "max"] {
            expected.push(("top_level_effort", level.to_string()));
            expected.push(("thinking_object", level.to_string()));
        }
        expected.push(("thinking_object", "off".to_string()));
        expected.push(("thinking_object", "auto".to_string()));
        expected.push(("top_level_effort", EFFORT_INVALID_VALUE.to_string()));
        expected.push(("thinking_object", EFFORT_INVALID_VALUE.to_string()));
        expected.push(("thinking_object", "high+temperature=0.5".to_string()));
        assert_eq!(plan_pairs(&plan), expected);
    }

    #[test]
    fn the_kimi_plan_bodies_have_the_documented_shapes() {
        let kimi = spec("Kimi");
        let plan = effort_plan(kimi);
        let body_of = |form: &str, value: &str| {
            let call = plan
                .iter()
                .find(|c| c.form == form && c.value == value)
                .unwrap_or_else(|| panic!("no {form} {value} call"));
            effort_body(kimi, "kimi-k3", call)
        };
        for level in ["minimal", "low", "medium", "high", "xhigh", "max"] {
            assert_eq!(
                body_of("top_level_effort", level)["reasoning_effort"],
                json!(level)
            );
            assert_eq!(
                body_of("thinking_object", level)["thinking"],
                json!({"type": "enabled", "effort": level})
            );
        }
        assert_eq!(
            body_of("thinking_object", "off")["thinking"],
            json!({"type": "disabled"})
        );
        assert_eq!(
            body_of("thinking_object", "auto")["thinking"],
            json!({"type": "enabled"})
        );
        assert_eq!(
            body_of("top_level_effort", EFFORT_INVALID_VALUE)["reasoning_effort"],
            json!("bogus-effort-51")
        );
        assert_eq!(
            body_of("thinking_object", EFFORT_INVALID_VALUE)["thinking"]["effort"],
            json!("bogus-effort-51")
        );
        let sampling = body_of("thinking_object", "high+temperature=0.5");
        assert_eq!(sampling["thinking"]["effort"], json!("high"));
        assert_eq!(sampling["temperature"], json!(0.5));
    }

    #[test]
    fn an_effort_body_carries_the_cap_and_the_prompt_and_never_the_acceptance_extra() {
        let kimi = spec("Kimi");
        assert!(kimi.acceptance_extras.is_some(), "the premise of this test");
        for call in effort_plan(kimi) {
            let body = effort_body(kimi, "kimi-k3", &call);
            assert_eq!(
                body["max_tokens"],
                json!(EFFORT_MAX_TOKENS),
                "{}",
                call.value
            );
            assert!(
                body.to_string().contains("17 multiplied by 23"),
                "{}",
                call.value
            );
            // The `--probe` acceptance extra disables thinking; only the planned `off` call may
            // carry a `disabled` thinking object.
            let disabled = body["thinking"]["type"] == json!("disabled");
            assert_eq!(disabled, call.role == EffortRole::Off, "{}", call.value);
        }
        assert_eq!(EFFORT_MAX_TOKENS, 2048);
        let default = effort_body(kimi, "kimi-k3", &effort_plan(kimi)[0]);
        assert!(default.get("thinking").is_none() && default.get("reasoning_effort").is_none());
    }

    #[test]
    fn deep_merge_merges_objects_and_replaces_scalars() {
        let mut body = json!({"a": {"x": 1, "y": 2}, "b": 1});
        deep_merge(&mut body, &json!({"a": {"y": 3, "z": 4}, "b": {"c": 1}}));
        assert_eq!(body, json!({"a": {"x": 1, "y": 3, "z": 4}, "b": {"c": 1}}));
        deep_merge(&mut body, &Value::Null);
        assert_eq!(body["a"]["x"], json!(1));
    }

    fn kimi_call(form: &str, value: &str) -> EffortCall {
        effort_plan(spec("Kimi"))
            .into_iter()
            .find(|c| c.form == form && c.value == value)
            .expect("planned call")
    }

    #[test]
    fn a_canned_kimi_200_yields_reasoning_tokens_and_the_finish_reason() {
        let body = r#"{"choices":[{"finish_reason":"length","message":{"content":"","reasoning_content":"hidden chain of thought"}}],
            "usage":{"prompt_tokens":20,"completion_tokens":2048,"total_tokens":2068,
                     "completion_tokens_details":{"reasoning_tokens":2040}}}"#;
        let row = effort_row_from_reply(
            &kimi_call("thinking_object", "max"),
            Wire::OpenAiChat,
            Ok(Reply {
                status: 200,
                body: body.to_string(),
            }),
            "k",
            1234,
        );
        assert_eq!(row.status, Some(200));
        assert_eq!(row.verdict, EffortVerdict::Accepted);
        assert_eq!(row.finish.as_deref(), Some("length"));
        assert_eq!(row.reasoning_tokens, Some(2040));
        assert_eq!(row.completion_tokens, Some(2048));
        assert_eq!(row.elapsed_ms, 1234);
        let line = effort_row_line(3, &row);
        assert!(!line.contains("hidden chain of thought"), "{line}");
        assert!(line.contains("accepted") && line.contains("2040"), "{line}");
    }

    #[test]
    fn a_canned_kimi_400_is_rejected_with_an_excerpt_that_has_passed_the_redactor() {
        let key = "zzSECRETKEYVALUE123456";
        let body = format!(
            "{{\"message\":\"Invalid request: invalid value for thinking effort level: \
             bogus-effort-51 (key {key})\",\"type\":\"invalid_request_error\"}}"
        );
        let row = effort_row_from_reply(
            &kimi_call("thinking_object", EFFORT_INVALID_VALUE),
            Wire::OpenAiChat,
            Ok(Reply { status: 400, body }),
            key,
            7,
        );
        assert_eq!(row.verdict, EffortVerdict::Rejected);
        assert_eq!(row.status, Some(400));
        assert!(row.detail.contains("invalid value for thinking effort"));
        assert!(!row.detail.contains(key), "{}", row.detail);
        assert!(row.detail.contains("[REDACTED]"), "{}", row.detail);
        assert_eq!(row.reasoning_tokens, None);
    }

    #[test]
    fn only_a_400_or_422_is_a_rejection_other_failures_are_errors() {
        let call = kimi_call("thinking_object", "low");
        for (status, verdict) in [
            (400, EffortVerdict::Rejected),
            (422, EffortVerdict::Rejected),
            (401, EffortVerdict::Error),
            (429, EffortVerdict::Error),
            (500, EffortVerdict::Error),
            (302, EffortVerdict::Error),
        ] {
            let row = effort_row_from_reply(
                &call,
                Wire::OpenAiChat,
                Ok(Reply {
                    status,
                    body: "x".to_string(),
                }),
                "k",
                0,
            );
            assert_eq!(row.verdict, verdict, "{status}");
        }
        let down = effort_row_from_reply(
            &call,
            Wire::OpenAiChat,
            Err("timed out".to_string()),
            "k",
            0,
        );
        assert_eq!((down.verdict, down.status), (EffortVerdict::Error, None));
    }

    #[test]
    fn the_form_summary_reports_parsed_and_effect_from_the_rows() {
        let row = |form: &'static str, value: &str, role, verdict, reasoning| EffortRow {
            form,
            value: value.to_string(),
            role,
            status: Some(200),
            finish: None,
            reasoning_tokens: reasoning,
            completion_tokens: None,
            elapsed_ms: 1,
            verdict,
            detail: "-".to_string(),
        };
        let rows = vec![
            row(
                "default",
                "-",
                EffortRole::Default,
                EffortVerdict::Accepted,
                Some(9),
            ),
            row(
                "top_level_effort",
                "low",
                EffortRole::Level,
                EffortVerdict::Accepted,
                Some(40),
            ),
            row(
                "top_level_effort",
                "max",
                EffortRole::Level,
                EffortVerdict::Accepted,
                Some(41),
            ),
            row(
                "top_level_effort",
                EFFORT_INVALID_VALUE,
                EffortRole::Invalid,
                EffortVerdict::Accepted,
                Some(40),
            ),
            row(
                "thinking_object",
                "low",
                EffortRole::Level,
                EffortVerdict::Accepted,
                Some(60),
            ),
            row(
                "thinking_object",
                "max",
                EffortRole::Level,
                EffortVerdict::Accepted,
                Some(1900),
            ),
            row(
                "thinking_object",
                EFFORT_INVALID_VALUE,
                EffortRole::Invalid,
                EffortVerdict::Rejected,
                None,
            ),
        ];
        let lines = form_summary_lines(&rows);
        assert_eq!(
            lines,
            vec![
                "- `top_level_effort`: field parsed: no; effect: low=40 max=41".to_string(),
                "- `thinking_object`: field parsed: yes; effect: low=60 max=1900".to_string(),
                "- detector: field not parsed (silent)".to_string(),
            ]
        );
    }

    #[test]
    fn the_effort_dry_run_lists_every_planned_kimi_call_with_no_credential() {
        let plan = render_effort_dry_run(&[("Kimi".to_string(), vec!["kimi-k3".to_string()])]);
        assert_eq!(plan.lines().filter(|l| l.starts_with("## ")).count(), 9);
        assert!(plan.contains("no network access"));
        assert!(plan.contains("`kimi-k3`: 18 calls"), "{plan}");
        for planned in effort_plan(spec("Kimi")) {
            assert!(
                plan.contains(&format!("| {} | {} |", planned.form, planned.value)),
                "{} {}",
                planned.form,
                planned.value
            );
        }
        for level in ["low", "max"] {
            assert!(plan.contains(&thinking_object_effort(level).to_string()));
            assert!(plan.contains(&top_level_effort(level).to_string()));
        }
        assert!(plan.contains("bogus-effort-51"));
        // No candidate means no probe: nothing is added from the curated lists.
        let none = render_effort_dry_run(&[]);
        assert!(none.contains("probe no default"), "{none}");
        assert!(!none.contains("kimi-k2.6"), "{none}");
    }

    #[test]
    fn effort_candidates_are_the_models_ids_only_without_duplicates() {
        assert_eq!(
            effort_candidates(&["a".into(), "b".into(), "a".into()]),
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(effort_candidates(&[]).is_empty());
    }

    // ----- every vendor's effort plan (Phase 51 Task 2) -----

    /// Plan entries keyed by `(form, value)`, panicking with a readable message when absent.
    fn planned<'a>(plan: &'a [EffortCall], form: &str, value: &str) -> &'a EffortCall {
        plan.iter()
            .find(|c| c.form == form && c.value == value)
            .unwrap_or_else(|| panic!("no planned call {form} / {value}"))
    }

    fn forms_of(plan: &[EffortCall]) -> Vec<&'static str> {
        let mut forms: Vec<&'static str> = Vec::new();
        for call in plan {
            if !forms.contains(&call.form) {
                forms.push(call.form);
            }
        }
        forms
    }

    #[test]
    fn the_openai_plan_sends_none_through_max_an_invalid_value_and_the_sampling_row() {
        let openai = spec("OpenAI");
        let plan = effort_plan(openai);
        assert_eq!(plan[0].form, "default");
        for level in ["none", "minimal", "low", "medium", "high", "xhigh", "max"] {
            assert_eq!(
                planned(&plan, "top_level_effort", level).extra,
                json!({"reasoning_effort": level})
            );
        }
        assert_eq!(
            planned(&plan, "top_level_effort", "none").role,
            EffortRole::Off
        );
        let invalid = planned(&plan, "top_level_effort", EFFORT_INVALID_VALUE);
        assert_eq!(invalid.role, EffortRole::Invalid);
        let sampling = planned(&plan, "top_level_effort", "high+temperature=0.5");
        assert_eq!(
            sampling.extra,
            json!({"reasoning_effort": "high", "temperature": 0.5})
        );
        assert_eq!(plan.len(), 10);
        // The cap field is the one OpenAI requires, never `max_tokens`.
        let body = effort_body(openai, "gpt-5.4-nano", &plan[0]);
        assert_eq!(body["max_completion_tokens"], json!(EFFORT_MAX_TOKENS));
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn the_deepseek_plan_probes_levels_off_auto_and_both_invalid_forms() {
        let plan = effort_plan(spec("DeepSeek"));
        for level in ["minimal", "low", "medium", "high", "xhigh", "max"] {
            assert_eq!(
                planned(&plan, "top_level_effort", level).extra,
                json!({"reasoning_effort": level})
            );
        }
        assert_eq!(
            planned(&plan, "thinking_object", "off").extra,
            json!({"thinking": {"type": "disabled"}})
        );
        assert_eq!(
            planned(&plan, "thinking_object", "auto").extra,
            json!({"thinking": {"type": "enabled"}})
        );
        assert_eq!(
            planned(&plan, "top_level_effort", EFFORT_INVALID_VALUE).extra,
            json!({"reasoning_effort": "bogus-effort-51"})
        );
        assert_eq!(
            planned(&plan, "thinking_object", EFFORT_INVALID_VALUE).extra,
            json!({"thinking": {"type": "bogus-effort-51"}})
        );
        assert!(plan.iter().any(|c| c.role == EffortRole::Sampling));
        assert_eq!(plan.len(), 12);
    }

    #[test]
    fn the_grok_plan_probes_each_level_an_invalid_value_and_the_sampling_row() {
        let plan = effort_plan(spec("Grok"));
        assert_eq!(forms_of(&plan), vec!["default", "top_level_effort"]);
        for level in ["minimal", "low", "medium", "high", "xhigh", "max"] {
            assert_eq!(
                planned(&plan, "top_level_effort", level).extra,
                json!({"reasoning_effort": level})
            );
        }
        assert!(plan.iter().any(|c| c.role == EffortRole::Invalid));
        assert!(plan.iter().any(|c| c.role == EffortRole::Sampling));
        assert_eq!(plan.len(), 9);
    }

    #[test]
    fn the_qwen_plan_probes_the_toggle_the_budget_bounds_and_a_detector() {
        let plan = effort_plan(spec("Qwen"));
        assert_eq!(
            planned(&plan, "enable_thinking", "off").extra,
            json!({"enable_thinking": false})
        );
        assert_eq!(
            planned(&plan, "enable_thinking", "auto").extra,
            json!({"enable_thinking": true})
        );
        assert_eq!(
            planned(&plan, "thinking_budget", "1").extra,
            json!({"enable_thinking": true, "thinking_budget": 1})
        );
        assert_eq!(
            planned(&plan, "thinking_budget", "0").extra,
            json!({"enable_thinking": true, "thinking_budget": 0})
        );
        let invalid = planned(&plan, "thinking_budget", "-1");
        assert_eq!(invalid.role, EffortRole::Invalid);
        assert_eq!(
            invalid.extra,
            json!({"enable_thinking": true, "thinking_budget": -1})
        );
        // Qwen has no levels, so the silent-field detector is its own call.
        let detector = planned(&plan, "detector", EFFORT_INVALID_VALUE);
        assert_eq!(detector.role, EffortRole::Detector);
        assert_eq!(
            detector.extra,
            json!({"reasoning_effort": "bogus-effort-51"})
        );
        assert!(plan.iter().any(|c| c.role == EffortRole::Sampling));
    }

    #[test]
    fn the_anthropic_plan_probes_each_level_alone_and_paired_with_adaptive_thinking() {
        let anthropic = spec("Anthropic");
        let plan = effort_plan(anthropic);
        for level in ["low", "medium", "high", "xhigh", "max"] {
            assert_eq!(
                planned(&plan, "output_config", level).extra,
                json!({"output_config": {"effort": level}})
            );
            assert_eq!(
                planned(&plan, "output_config+adaptive", level).extra,
                json!({"output_config": {"effort": level}, "thinking": {"type": "adaptive"}})
            );
        }
        assert_eq!(
            planned(&plan, "thinking_object", "off").extra,
            json!({"thinking": {"type": "disabled"}})
        );
        assert_eq!(
            planned(&plan, "thinking_object", "auto").extra,
            json!({"thinking": {"type": "adaptive"}})
        );
        assert_eq!(
            planned(&plan, "output_config", EFFORT_INVALID_VALUE).extra,
            json!({"output_config": {"effort": "bogus-effort-51"}})
        );
        assert_eq!(
            planned(&plan, "thinking_budget", "1024").extra,
            json!({"thinking": {"type": "enabled", "budget_tokens": 1024}})
        );
        assert_eq!(
            planned(&plan, "thinking_budget", "1023").extra,
            json!({"thinking": {"type": "enabled", "budget_tokens": 1023}})
        );
        assert!(plan.iter().any(|c| c.role == EffortRole::Sampling));
        // No detector: the Anthropic wire is not the OpenAI chat wire.
        assert!(!plan.iter().any(|c| c.role == EffortRole::Detector));
        let body = effort_body(anthropic, "claude-opus-4-8", &plan[0]);
        assert_eq!(body["max_tokens"], json!(EFFORT_MAX_TOKENS));
    }

    #[test]
    fn the_gemini_plan_probes_lowercase_levels_one_uppercase_variant_and_budgets() {
        let gemini = spec("Gemini");
        let plan = effort_plan(gemini);
        let level =
            |value: &str| json!({"generationConfig": {"thinkingConfig": {"thinkingLevel": value}}});
        for value in ["minimal", "low", "medium", "high", "HIGH"] {
            assert_eq!(planned(&plan, "thinking_level", value).extra, level(value));
        }
        assert_eq!(
            planned(&plan, "thinking_level", EFFORT_INVALID_VALUE).extra,
            level("bogus-effort-51")
        );
        let budget =
            |value: i64| json!({"generationConfig": {"thinkingConfig": {"thinkingBudget": value}}});
        assert_eq!(planned(&plan, "thinking_budget", "off").extra, budget(0));
        assert_eq!(planned(&plan, "thinking_budget", "128").extra, budget(128));
        assert_eq!(planned(&plan, "thinking_budget", "1").extra, budget(1));
        assert_eq!(planned(&plan, "thinking_budget", "-5").extra, budget(-5));
        assert_eq!(
            planned(&plan, "thinking_budget", "-5").role,
            EffortRole::Invalid
        );
        // The merged body keeps the output cap beside the thinking config.
        let body = effort_body(
            gemini,
            "gemini-3.6-flash",
            planned(&plan, "thinking_level", "low"),
        );
        assert_eq!(
            body["generationConfig"]["maxOutputTokens"],
            json!(EFFORT_MAX_TOKENS)
        );
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            json!("low")
        );
        let sampling = plan
            .iter()
            .find(|c| c.role == EffortRole::Sampling)
            .expect("sampling row");
        let body = effort_body(gemini, "gemini-3.6-flash", sampling);
        assert_eq!(body["generationConfig"]["temperature"], json!(0.5));
        assert_eq!(
            body["generationConfig"]["maxOutputTokens"],
            json!(EFFORT_MAX_TOKENS)
        );
    }

    #[test]
    fn a_self_hosted_target_is_probed_through_all_four_forms() {
        for name in ["Ollama", "OpenAI-compatible"] {
            let plan = effort_plan(spec(name));
            let forms = forms_of(&plan);
            for form in ["top_level_effort", "thinking_object", "enable_thinking"] {
                assert!(forms.contains(&form), "{name}: {forms:?}");
            }
            assert!(
                plan.iter()
                    .any(|c| c.extra.get("reasoning_effort") == Some(&json!(EFFORT_INVALID_VALUE))),
                "{name}: no detector"
            );
            assert!(
                plan.iter().any(|c| c.role == EffortRole::Sampling),
                "{name}"
            );
        }
    }

    #[test]
    fn every_openai_chat_vendor_gets_the_silent_field_detector() {
        for p in PROVIDERS.iter().filter(|p| p.wire == Wire::OpenAiChat) {
            let plan = effort_plan(p);
            assert!(
                plan.iter()
                    .any(|c| c.extra.get("reasoning_effort") == Some(&json!(EFFORT_INVALID_VALUE))),
                "{} has no top-level reasoning_effort detector",
                p.name
            );
        }
    }

    #[test]
    fn every_provider_has_a_plan_and_every_body_is_capped_and_free_of_the_acceptance_extra() {
        for p in PROVIDERS.iter() {
            let plan = effort_plan(p);
            assert!(!plan.is_empty(), "{} has no plan", p.name);
            assert_eq!(plan[0].role, EffortRole::Default, "{}", p.name);
            for call in &plan {
                let body = effort_body(p, "m", call);
                let cap = match p.wire {
                    Wire::OpenAiChat => body[p.cap_field].clone(),
                    Wire::AnthropicMessages => body["max_tokens"].clone(),
                    Wire::GeminiGenerate => body["generationConfig"]["maxOutputTokens"].clone(),
                };
                assert_eq!(cap, json!(EFFORT_MAX_TOKENS), "{} {}", p.name, call.value);
                if p.name == "Kimi" && call.role != EffortRole::Off {
                    assert_ne!(
                        body["thinking"]["type"],
                        json!("disabled"),
                        "{}",
                        call.value
                    );
                }
            }
        }
    }

    #[test]
    fn the_summary_adds_a_detector_line_and_prints_the_silent_phrase_on_a_2xx() {
        let row = |form: &'static str, role, verdict| EffortRow {
            form,
            value: EFFORT_INVALID_VALUE.to_string(),
            role,
            status: Some(200),
            finish: None,
            reasoning_tokens: None,
            completion_tokens: None,
            elapsed_ms: 1,
            verdict,
            detail: "-".to_string(),
        };
        let silent = form_summary_lines(&[row(
            "detector",
            EffortRole::Detector,
            EffortVerdict::Accepted,
        )]);
        assert_eq!(
            silent,
            vec!["- detector: field not parsed (silent)".to_string()]
        );
        let parsed = form_summary_lines(&[row(
            "detector",
            EffortRole::Detector,
            EffortVerdict::Rejected,
        )]);
        assert_eq!(
            parsed,
            vec!["- detector: field parsed (rejected)".to_string()]
        );
        // A top-level invalid value doubles as the detector.
        let lines = form_summary_lines(&[row(
            "top_level_effort",
            EffortRole::Invalid,
            EffortVerdict::Accepted,
        )]);
        assert!(
            lines.contains(&"- detector: field not parsed (silent)".to_string()),
            "{lines:?}"
        );
    }

    #[test]
    fn budget_forms_report_their_reasoning_tokens_in_the_summary() {
        let row = |value: &str, role, verdict, reasoning| EffortRow {
            form: "thinking_budget",
            value: value.to_string(),
            role,
            status: Some(200),
            finish: None,
            reasoning_tokens: reasoning,
            completion_tokens: None,
            elapsed_ms: 1,
            verdict,
            detail: "-".to_string(),
        };
        let lines = form_summary_lines(&[
            row(
                "1024",
                EffortRole::Budget,
                EffortVerdict::Accepted,
                Some(900),
            ),
            row("1023", EffortRole::Budget, EffortVerdict::Rejected, None),
        ]);
        assert_eq!(
            lines,
            vec!["- `thinking_budget`: field parsed: unknown; budget: 1024=900 1023=-".to_string()]
        );
    }

    // ----- `--effort-smoke`: the adapter path (Phase 51 Task 2, REASON-03) -----

    use paladin_ports::output::llm_port::{
        FinishReason, LlmError, LlmResponse, ProviderCapabilities, ReasoningEffort,
        StreamingResponse, TokenUsage,
    };
    use paladin_ports::output::model_profile::{EffortLevels, EffortSupport, ModelProfile};
    use std::sync::Mutex;

    /// An adapter double: reports the given effort support and answers every `generate` with the
    /// canned result, recording the effort each request carried.
    struct MockPort {
        effort: Option<EffortSupport>,
        answer: fn(&LlmRequest) -> Result<LlmResponse, LlmError>,
        seen: Mutex<Vec<Option<ReasoningEffort>>>,
    }

    impl MockPort {
        fn new(
            effort: Option<EffortSupport>,
            answer: fn(&LlmRequest) -> Result<LlmResponse, LlmError>,
        ) -> Self {
            Self {
                effort,
                answer,
                seen: Mutex::new(Vec::new()),
            }
        }

        fn efforts_sent(&self) -> Vec<Option<ReasoningEffort>> {
            self.seen.lock().expect("lock").clone()
        }
    }

    #[async_trait::async_trait]
    impl LlmPort for MockPort {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            self.seen.lock().expect("lock").push(request.reasoning);
            (self.answer)(&request)
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            Err(LlmError::ProcessingError("unused".to_string()))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(Vec::new())
        }

        fn get_provider_name(&self) -> &'static str {
            "mock"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn model_profile(&self, _model: &str) -> Option<ModelProfile> {
            self.effort
                .clone()
                .map(|effort| ModelProfile::new().with_effort(effort))
        }
    }

    fn supported(levels: &[ReasoningEffort]) -> Option<EffortSupport> {
        Some(EffortSupport::Supported(EffortLevels::new(
            levels.iter().copied(),
        )))
    }

    fn run_smoke_model(port: &MockPort, key: &str) -> Vec<SmokeRow> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
            .block_on(smoke_model(port, "some-model", key))
    }

    fn canned_ok(request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut response = LlmResponse::new(
            request.id,
            "some-model",
            "SECRET-ANSWER-TEXT 391",
            FinishReason::Length,
            TokenUsage::new(20, 2048).with_reasoning(2040),
        );
        if let Some(effort) = request.reasoning {
            response.metadata.insert(
                REASONING_REQUESTED_METADATA_KEY.to_string(),
                effort.to_string(),
            );
            response.metadata.insert(
                REASONING_EFFECTIVE_METADATA_KEY.to_string(),
                "low".to_string(),
            );
        }
        Ok(response)
    }

    fn canned_refusal(request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        Err(LlmError::unsupported_parameter(
            "reasoning_effort",
            request
                .reasoning
                .map_or_else(|| "-".to_string(), |e| e.to_string()),
            "mock",
            "some-model",
            "the model does not offer that effort level",
        ))
    }

    #[test]
    fn effort_smoke_flags_parse_with_the_optional_caps() {
        let options = effort_options(&[
            "--effort-smoke",
            "--max-tokens",
            "512",
            "--timeout-seconds",
            "30",
            "--models",
            "deepseek=deepseek-v4-pro",
        ]);
        assert!(options.effort_smoke && !options.effort && !options.probe && !options.usage);
        assert_eq!(options.max_tokens, Some(512));
        assert_eq!(options.timeout_seconds, Some(30));
        let plain = effort_options(&["--effort-smoke", "--dry-run"]);
        assert_eq!((plain.max_tokens, plain.timeout_seconds), (None, None));
    }

    #[test]
    fn the_caps_are_parsed_only_with_effort_smoke_and_must_be_positive_integers() {
        for list in [
            ["--max-tokens", "10", "--effort"],
            ["--timeout-seconds", "10", "--probe"],
            ["--max-tokens", "10", "--dry-run"],
        ] {
            let err = parse_args(args(&list)).unwrap_err();
            assert!(err.contains("--effort-smoke"), "{err}");
        }
        for list in [
            ["--effort-smoke", "--max-tokens", "0"],
            ["--effort-smoke", "--max-tokens", "lots"],
            ["--effort-smoke", "--timeout-seconds", "-3"],
            ["--effort-smoke", "--timeout-seconds", "0"],
        ] {
            let err = parse_args(args(&list)).unwrap_err();
            assert!(err.contains("positive integer"), "{err}");
        }
        assert!(parse_args(args(&["--effort-smoke", "--max-tokens"])).is_err());
    }

    #[test]
    fn the_help_text_names_every_effort_flag() {
        for flag in [
            "--effort",
            "--effort-smoke",
            "--max-tokens",
            "--timeout-seconds",
        ] {
            assert!(USAGE.contains(flag), "USAGE does not name {flag}");
        }
    }

    #[test]
    fn the_effort_smoke_dry_run_lists_two_adapter_calls_per_candidate_and_reads_no_credential() {
        let plan = render_effort_smoke_dry_run(
            &[("DeepSeek".to_string(), vec!["deepseek-v4-pro".to_string()])],
            Some(512),
            Some(30),
        );
        assert_eq!(plan.lines().filter(|l| l.starts_with("## ")).count(), 9);
        assert!(plan.contains("no network access"));
        assert!(
            plan.contains("`deepseek-v4-pro`: 2 adapter calls"),
            "{plan}"
        );
        assert!(plan.contains("512") && plan.contains("30 s"), "{plan}");
        // Nothing is planned for a provider with no `--models` candidate.
        assert!(plan.contains("none (pass `--models openai="), "{plan}");
    }

    #[test]
    fn the_smoke_sends_the_lowest_and_the_highest_admitted_level() {
        use ReasoningEffort::{High, Low, Max, Minimal};
        let port = MockPort::new(supported(&[Low, Minimal, Max, High]), canned_ok);
        let rows = run_smoke_model(&port, "k");
        assert_eq!(port.efforts_sent(), vec![Some(Minimal), Some(Max)]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].requested, "minimal");
        assert_eq!(rows[1].requested, "max");
        // One admitted level is one request, not the same level twice.
        let single = MockPort::new(supported(&[High]), canned_ok);
        assert_eq!(run_smoke_model(&single, "k").len(), 1);
        assert_eq!(single.efforts_sent(), vec![Some(High)]);
    }

    #[test]
    fn an_ok_row_shows_tokens_finish_and_the_two_metadata_values_never_content() {
        use ReasoningEffort::{High, Low};
        let port = MockPort::new(supported(&[Low, High]), canned_ok);
        let rows = run_smoke_model(&port, "k");
        let row = &rows[1];
        assert_eq!(row.outcome, "Ok");
        assert_eq!(row.finish.as_deref(), Some("length"));
        assert_eq!(row.reasoning_tokens, Some(2040));
        assert_eq!(row.completion_tokens, Some(2048));
        assert_eq!(row.meta_requested, "high");
        assert_eq!(row.meta_effective, "low");
        let line = smoke_row_line(&row.model, row);
        for forbidden in ["SECRET", "391"] {
            assert!(!line.contains(forbidden), "{line}");
        }
        assert!(line.contains("2040") && line.contains("high"), "{line}");
    }

    #[test]
    fn a_refusal_row_shows_the_unsupported_parameter_reason() {
        use ReasoningEffort::{High, Low};
        let port = MockPort::new(supported(&[Low, High]), canned_refusal);
        let rows = run_smoke_model(&port, "k");
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert!(
                row.outcome.starts_with("UnsupportedParameter"),
                "{}",
                row.outcome
            );
            assert!(
                row.outcome
                    .contains("the model does not offer that effort level"),
                "{}",
                row.outcome
            );
            assert_eq!(row.reasoning_tokens, None);
            assert_eq!(row.meta_requested, "-");
        }
    }

    #[test]
    fn a_state_that_is_not_supported_sends_nothing_and_says_why() {
        for (effort, why) in [
            (Some(EffortSupport::Unmeasured), "unmeasured"),
            (Some(EffortSupport::WireUnsupported), "wire unsupported"),
            (Some(EffortSupport::NoReasoning), "no reasoning"),
            (None, "no model profile"),
        ] {
            let port = MockPort::new(effort, canned_ok);
            let rows = run_smoke_model(&port, "k");
            assert!(port.efforts_sent().is_empty(), "{why}: a request was sent");
            assert_eq!(rows.len(), 1, "{why}");
            assert!(
                rows[0].outcome.starts_with("not sent"),
                "{}",
                rows[0].outcome
            );
            assert!(rows[0].outcome.contains(why), "{}", rows[0].outcome);
            assert_eq!(rows[0].requested, "-");
        }
        // A Supported profile that lists no level has nothing to send either.
        let empty = MockPort::new(supported(&[]), canned_ok);
        let rows = run_smoke_model(&empty, "k");
        assert!(empty.efforts_sent().is_empty());
        assert!(rows[0].outcome.contains("no levels"), "{}", rows[0].outcome);
    }

    #[test]
    fn a_key_shaped_string_in_an_error_never_reaches_a_printed_smoke_row() {
        fn leaky(_request: &LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::ProcessingError(
                "upstream said: bad request for key zzSECRETKEYVALUE123456".to_string(),
            ))
        }
        use ReasoningEffort::{High, Low};
        let port = MockPort::new(supported(&[Low, High]), leaky);
        let key = "zzSECRETKEYVALUE123456";
        let rows = run_smoke_model(&port, key);
        for row in &rows {
            let line = smoke_row_line(&row.model, row);
            assert!(!line.contains(key), "{line}");
            assert!(!line.contains("zzSECRET"), "{line}");
            assert!(line.contains("[REDACTED]"), "{line}");
            assert!(row.outcome.starts_with("error"), "{}", row.outcome);
        }
    }

    #[test]
    fn every_provider_has_a_smoke_port_builder_and_an_unknown_one_is_an_error() {
        // The environment decides whether a build succeeds (Ollama needs no key), so only the
        // contract is pinned: no builder panics, and a name outside the table is an error.
        let knobs = SmokeKnobs {
            max_tokens: Some(64),
            timeout_seconds: Some(5),
        };
        for p in PROVIDERS.iter() {
            let _ = build_smoke_port(p.name, knobs);
        }
        assert!(build_smoke_port("Nope", knobs).is_err());
    }
}
