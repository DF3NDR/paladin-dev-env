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
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::time::Duration;

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
/// its seeded table). That table holds exactly the adapter's default model today, so the list is
/// built from the adapter's own constant rather than restated; the unit test
/// `the_static_model_lists_match_the_adapters` fails if the table grows.
const ANTHROPIC_MODELS: &[&str] = &[ANTHROPIC_DEFAULT_MODEL];

/// DeepSeek's static model list (mirrors `DeepSeekAdapter::get_available_models`, which returns
/// its seeded table). Built from the adapter's own default-model constant for the same reason as
/// [`ANTHROPIC_MODELS`], and guarded by the same unit test.
const DEEPSEEK_MODELS: &[&str] = &[DEEPSEEK_DEFAULT_MODEL];

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

const USAGE: &str = "usage: live_vendor_smoke [--probe | --usage] [--dry-run] [--models <provider>=<id>[,<id>...]]... [-h|--help]\n\
\n\
  (no flag)   credential-gated smoke probes over all nine providers\n\
  --probe     record, per configured provider, the live models listing and what each candidate\n\
              model accepts (sampling, stop sequences, output cap, JSON-object and JSON-schema\n\
              modes, usage fields, quota-shaped errors); prints a markdown transcript\n\
  --usage     send one reasoning-eliciting call per candidate model and record the usage\n\
              arithmetic, numbers only, with a containment verdict per model; excludes --probe\n\
  --dry-run   print the probe (or, with --usage, the usage) plan for all nine providers; no\n\
              network access, no credential read\n\
  --models    extra candidate model ids for one provider (repeatable); needs --probe, --usage\n\
              or --dry-run\n\
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
        match arg.as_str() {
            "-h" | "--help" => help = true,
            "--probe" => options.probe = true,
            "--usage" => options.usage = true,
            "--dry-run" => options.dry_run = true,
            "--models" => {
                let value = args.next().ok_or_else(|| {
                    "`--models` needs a value of the form <provider>=<id>[,<id>...]".to_string()
                })?;
                add_models(&mut options, &value)?;
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if help {
        return Ok(Command::Help);
    }
    if options.usage && options.probe {
        return Err("`--usage` and `--probe` are mutually exclusive".to_string());
    }
    if !options.models.is_empty() && !options.probe && !options.usage && !options.dry_run {
        return Err("`--models` has no effect without --probe, --usage or --dry-run".to_string());
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
    if matches!(kind, ProbeKind::DefaultThinking | ProbeKind::Usage) {
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
        ProbeKind::Usage => USAGE_PROMPT,
        _ => PROBE_PROMPT,
    };
    let cap = if kind == ProbeKind::Usage {
        USAGE_MAX_TOKENS
    } else {
        PROBE_MAX_TOKENS
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
        ProbeKind::Default | ProbeKind::DefaultThinking | ProbeKind::Usage => {}
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
        Command::Run(options) if options.dry_run && options.usage => {
            print!("{}", render_usage_dry_run(&options.models));
        }
        Command::Run(options) if options.dry_run => {
            print!("{}", render_dry_run(&options.models));
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
                usage: false,
                models: vec![
                    ("Kimi".to_string(), vec!["a".to_string(), "b".to_string()]),
                    ("Kimi".to_string(), vec!["c".to_string()]),
                ],
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
}
