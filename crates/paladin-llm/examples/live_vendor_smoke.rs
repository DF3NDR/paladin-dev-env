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
use paladin_llm::anthropic::{AnthropicAdapter, AnthropicConfig};
use paladin_llm::deepseek::{DeepSeekAdapter, DeepSeekConfig};
use paladin_llm::gemini::adapter::{
    GEMINI_DEFAULT_BASE_URL, GEMINI_DEFAULT_MODEL, GEMINI_FALLBACK_MODELS,
};
use paladin_llm::gemini::{GeminiAdapter, GeminiConfig};
use paladin_llm::grok::adapter::{GROK_DEFAULT_BASE_URL, GROK_DEFAULT_MODEL, GROK_FALLBACK_MODELS};
use paladin_llm::grok::{GrokAdapter, GrokConfig};
use paladin_llm::kimi::adapter::{KIMI_DEFAULT_BASE_URL, KIMI_DEFAULT_MODEL, KIMI_FALLBACK_MODELS};
use paladin_llm::kimi::{KimiAdapter, KimiConfig};
use paladin_llm::ollama::adapter::{
    OLLAMA_DEFAULT_BASE_URL, OLLAMA_DEFAULT_MODEL, OLLAMA_FALLBACK_MODELS,
};
use paladin_llm::ollama::{OllamaAdapter, OllamaConfig};
use paladin_llm::openai::{OpenAIAdapter, OpenAIConfig};
use paladin_llm::openai_compatible::{OpenAiCompatibleAdapter, OpenAiCompatibleConfig};
use paladin_llm::qwen::adapter::{QWEN_DEFAULT_BASE_URL, QWEN_DEFAULT_MODEL, QWEN_FALLBACK_MODELS};
use paladin_llm::qwen::{QwenAdapter, QwenConfig};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};

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
    build: fn() -> Result<Built, String>,
}

/// Anthropic's static model list (mirrors `AnthropicAdapter::get_available_models`; a unit test
/// below keeps the two in step).
const ANTHROPIC_MODELS: &[&str] = &[
    "claude-3-5-sonnet-20241022",
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    "claude-2.1",
    "claude-2.0",
    "claude-instant-1.2",
];

/// DeepSeek's static model list (mirrors `DeepSeekAdapter::get_available_models`).
const DEEPSEEK_MODELS: &[&str] = &["deepseek-chat", "deepseek-coder"];

/// `OpenAIConfig` carries no model (the request does), so the harness names one candidate. It is
/// a probe candidate only; pass others with `--models openai=<id>`.
const OPENAI_PROBE_DEFAULT_MODEL: &str = "gpt-4o-mini";

/// `AnthropicConfig::from_env` default model, restated for the dry run (no credential read).
const ANTHROPIC_DEFAULT_MODEL: &str = "claude-3-5-sonnet-20241022";

/// `DeepSeekConfig::from_env` default model, restated for the dry run.
const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-chat";

/// Build an adapter whose config exposes `base_url` and `model` and whose `new` returns
/// `Result<_, LlmError>`.
macro_rules! build_fn {
    ($name:ident, $config:ty, $adapter:ty) => {
        fn $name() -> Result<Built, String> {
            let cfg = <$config>::from_env()?;
            let (base_url, model) = (cfg.base_url.clone(), cfg.model.clone());
            let adapter = <$adapter>::new(cfg).map_err(|e| e.to_string())?;
            Ok(Built {
                port: Box::new(adapter),
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
    let base_url = cfg.base_url.clone();
    let adapter = OpenAIAdapter::new(cfg)?;
    Ok(Built {
        port: Box::new(adapter),
        base_url,
        model: OPENAI_PROBE_DEFAULT_MODEL.to_string(),
    })
}

fn build_ollama() -> Result<Built, String> {
    let cfg = OllamaConfig::from_env()?;
    let (base_url, model) = (cfg.base_url.clone(), cfg.model.clone());
    let adapter = OllamaAdapter::new(cfg).map_err(|e| e.to_string())?;
    Ok(Built {
        port: Box::new(adapter),
        base_url,
        model,
    })
}

/// The nine providers. Order is the order of the run output.
const PROVIDERS: [ProviderSpec; 9] = [
    ProviderSpec {
        name: "OpenAI",
        cred_vars: &["OPENAI_API_KEY"],
        default_base_url: Some("https://api.openai.com/v1"),
        default_model: OPENAI_PROBE_DEFAULT_MODEL,
        curated_models: &[],
        live_model_list: true,
        wire: Wire::OpenAiChat,
        auth: Auth::Bearer,
        temperature_range: Some((0.0, 1.0)),
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
        temperature_range: Some((1.0, 1.0)),
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

/// Parsed command line.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Smoke,
    Help,
}

const USAGE: &str = "usage: live_vendor_smoke [-h|--help]\n\
\n\
With no flag, runs the credential-gated smoke probes over all nine providers.";

/// Parse the argument list (without the program name). Any argument the harness does not know is
/// an error naming it -- never a silent smoke run.
fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut command = Command::Smoke;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => command = Command::Help,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(command)
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
        Command::Smoke => std::process::exit(run_smoke().await),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
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
        assert_eq!(parse_args(args(&[])), Ok(Command::Smoke));
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
        let anthropic = AnthropicAdapter::new(AnthropicConfig::new(
            "k".to_string(),
            "https://example.invalid/v1".to_string(),
            ANTHROPIC_DEFAULT_MODEL.to_string(),
            16,
        ))
        .expect("anthropic adapter");
        let deepseek = DeepSeekAdapter::new(DeepSeekConfig::new(
            "k".to_string(),
            "https://example.invalid/v1".to_string(),
            DEEPSEEK_DEFAULT_MODEL.to_string(),
        ))
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
}
