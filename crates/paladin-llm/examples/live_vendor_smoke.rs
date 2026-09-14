//! Live vendor smoke test — Kimi, Qwen, Grok, Gemini (Phase 17 UAT test 4).
//!
//! Proves, against the REAL vendor endpoints, that for each provider:
//!   1. the documented `base_url` resolves and authenticates,
//!   2. `get_available_models()` returns from the LIVE-FETCH path, not the
//!      curated fallback, and the adapter's default model ID actually
//!      exists in that live list, and
//!   3. `generate()` completes a real call using the framework's DEFAULT
//!      prompt parameters (`PromptParameters::default()`) — the exact
//!      configuration `17-UAT.md` proved fails for Grok.
//!
//! Point 2 is the reason this cannot be a plain "did it return a non-empty
//! list" check: `available_models()` swallows every failure and silently
//! returns the curated `*_FALLBACK_MODELS` constant, so a non-empty result
//! is NOT evidence the network path worked. This binary discriminates by
//! comparing the result against that exact constant.
//!
//! Point 3 is a SEPARATE probe from point 2: a vendor can pass the
//! model-list probe (proving auth and the default model ID are both good)
//! while still failing every `generate()` call, which is exactly what
//! happened to Grok before the `CompatRequestParameters` fix landed
//! (17-18). A generate probe that returns `Err`, or `Ok` with empty content
//! or zero total tokens, is a FAIL — a 200 carrying nothing is the vacuous
//! pass this harness exists to refuse.
//!
//! Run with real credentials in the environment:
//!   cargo run -p paladin-llm --example live_vendor_smoke \
//!     --features kimi,qwen,grok,gemini
//!
//! Exits non-zero if any provider fails either probe. Never prints a
//! credential — every error printed here is an `LlmError`, which the engine
//! has already routed through its redact-then-bound diagnostic excerpt.

use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_llm::gemini::adapter::{
    GEMINI_DEFAULT_BASE_URL, GEMINI_DEFAULT_MODEL, GEMINI_FALLBACK_MODELS,
};
use paladin_llm::gemini::{GeminiAdapter, GeminiConfig};
use paladin_llm::grok::adapter::{GROK_DEFAULT_BASE_URL, GROK_DEFAULT_MODEL, GROK_FALLBACK_MODELS};
use paladin_llm::grok::{GrokAdapter, GrokConfig};
use paladin_llm::kimi::adapter::{KIMI_DEFAULT_BASE_URL, KIMI_DEFAULT_MODEL, KIMI_FALLBACK_MODELS};
use paladin_llm::kimi::{KimiAdapter, KimiConfig};
use paladin_llm::qwen::adapter::{QWEN_DEFAULT_BASE_URL, QWEN_DEFAULT_MODEL, QWEN_FALLBACK_MODELS};
use paladin_llm::qwen::{QwenAdapter, QwenConfig};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};

/// Minimal `log::Log` implementation, no dependency (17-22, G-17-4d).
///
/// With no logger installed, the `log` facade silently discards every
/// record — so before this plan's engine-level `log::warn!` diagnostic
/// existed to demonstrate, this harness had nothing to show it with even if
/// it fired. Fixed at `LevelFilter::Warn` rather than parsing `RUST_LOG`:
/// hand-rolling a level-string parser with no dependency would add
/// complexity this harness does not need — `Warn` is exactly the level the
/// new diagnostic is emitted at, so a fixed filter is sufficient to prove
/// both halves of task 2: the warning fires on a deliberately mismatched
/// endpoint (run A) and stays completely silent on the shipped defaults
/// (run B).
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

/// Outcome of the model-list half of a vendor probe.
struct Live {
    models: Vec<String>,
    /// True when the returned list is byte-identical to the curated fallback,
    /// i.e. the live fetch silently failed.
    is_fallback: bool,
    default_present: bool,
}

fn classify(models: Vec<String>, fallback: &[&str], default_model: &str) -> Live {
    let is_fallback =
        models.len() == fallback.len() && models.iter().zip(fallback.iter()).all(|(m, f)| m == f);
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

/// Call `generate()` on `port` with the framework's DEFAULT prompt
/// parameters — deliberately built via `PromptItem::new(...)` rather than
/// any explicit parameter override, so this probe carries exactly the
/// configuration `17-UAT.md` proved fails for Grok. Setting parameters
/// explicitly here would hide the defect this probe exists to catch.
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
                    "generate() returned Ok with empty content — vacuous pass refused".to_string(),
                );
            }
            if response.usage.total_tokens == 0 {
                return Err(format!(
                    "generate() returned Ok with zero total tokens (content {} chars) — \
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

/// Outcome of probing one provider: two independent results under one
/// vendor heading — the model-list probe and the generate probe.
struct Probe {
    vendor: &'static str,
    key_var: &'static str,
    /// The base URL the run ACTUALLY used, read back off the resolved config —
    /// NOT the `*_DEFAULT_BASE_URL` constant. A `*_BASE_URL` environment
    /// override changes what goes on the wire, and a record that printed the
    /// constant regardless would attribute a result to the wrong endpoint.
    base_url: String,
    default_model: String,
    /// `Err` = the adapter could not even be constructed (missing key), or
    /// the live model-list fetch/comparison failed.
    models_result: Result<Live, String>,
    /// `Err` = adapter construction failed, `generate()` returned `Err`, or
    /// `generate()` returned a vacuous `Ok` (empty content / zero tokens).
    generate_result: Result<GenerateOutcome, String>,
}

#[tokio::main]
async fn main() {
    // `set_logger` (not `set_boxed_logger`) is used deliberately: the latter
    // requires `log`'s `std` feature, which this workspace does not enable
    // for `log = { workspace = true }` — adding it would be exactly the
    // dependency-surface change T-17-SC-22's prohibition rules out. A
    // `'static` reference to a zero-sized logger needs no allocation and no
    // additional feature.
    static LOGGER: StderrLogger = StderrLogger;
    log::set_logger(&LOGGER).expect("logger installed exactly once at harness startup");
    log::set_max_level(log::LevelFilter::Warn);

    let mut probes: Vec<Probe> = Vec::new();

    // ── Kimi (Moonshot) ─────────────────────────────────────────────
    probes.push(match KimiConfig::from_env() {
        Err(e) => Probe {
            vendor: "Kimi",
            key_var: "MOONSHOT_API_KEY",
            base_url: KIMI_DEFAULT_BASE_URL.to_string(),
            default_model: KIMI_DEFAULT_MODEL.to_string(),
            models_result: Err(e.clone()),
            generate_result: Err(e),
        },
        Ok(cfg) => {
            let (base_url, model) = (cfg.base_url.clone(), cfg.model.clone());
            match KimiAdapter::new(cfg) {
                Err(e) => Probe {
                    vendor: "Kimi",
                    key_var: "MOONSHOT_API_KEY",
                    base_url,
                    default_model: model,
                    models_result: Err(e.to_string()),
                    generate_result: Err("adapter construction failed".to_string()),
                },
                Ok(a) => {
                    let models_result = match a.get_available_models().await {
                        Ok(m) => Ok(classify(m, KIMI_FALLBACK_MODELS, &model)),
                        Err(e) => Err(format!("get_available_models failed: {e}")),
                    };
                    let generate_result = probe_generate(&a, &model).await;
                    Probe {
                        vendor: "Kimi",
                        key_var: "MOONSHOT_API_KEY",
                        base_url,
                        default_model: model,
                        models_result,
                        generate_result,
                    }
                }
            }
        }
    });

    // ── Qwen (Alibaba DashScope) ────────────────────────────────────
    probes.push(match QwenConfig::from_env() {
        Err(e) => Probe {
            vendor: "Qwen",
            key_var: "DASHSCOPE_API_KEY",
            base_url: QWEN_DEFAULT_BASE_URL.to_string(),
            default_model: QWEN_DEFAULT_MODEL.to_string(),
            models_result: Err(e.clone()),
            generate_result: Err(e),
        },
        Ok(cfg) => {
            let (base_url, model) = (cfg.base_url.clone(), cfg.model.clone());
            match QwenAdapter::new(cfg) {
                Err(e) => Probe {
                    vendor: "Qwen",
                    key_var: "DASHSCOPE_API_KEY",
                    base_url,
                    default_model: model,
                    models_result: Err(e.to_string()),
                    generate_result: Err("adapter construction failed".to_string()),
                },
                Ok(a) => {
                    let models_result = match a.get_available_models().await {
                        Ok(m) => Ok(classify(m, QWEN_FALLBACK_MODELS, &model)),
                        Err(e) => Err(format!("get_available_models failed: {e}")),
                    };
                    let generate_result = probe_generate(&a, &model).await;
                    Probe {
                        vendor: "Qwen",
                        key_var: "DASHSCOPE_API_KEY",
                        base_url,
                        default_model: model,
                        models_result,
                        generate_result,
                    }
                }
            }
        }
    });

    // ── Grok (xAI) ──────────────────────────────────────────────────
    probes.push(match GrokConfig::from_env() {
        Err(e) => Probe {
            vendor: "Grok",
            key_var: "XAI_API_KEY",
            base_url: GROK_DEFAULT_BASE_URL.to_string(),
            default_model: GROK_DEFAULT_MODEL.to_string(),
            models_result: Err(e.clone()),
            generate_result: Err(e),
        },
        Ok(cfg) => {
            let (base_url, model) = (cfg.base_url.clone(), cfg.model.clone());
            match GrokAdapter::new(cfg) {
                Err(e) => Probe {
                    vendor: "Grok",
                    key_var: "XAI_API_KEY",
                    base_url,
                    default_model: model,
                    models_result: Err(e.to_string()),
                    generate_result: Err("adapter construction failed".to_string()),
                },
                Ok(a) => {
                    let models_result = match a.get_available_models().await {
                        Ok(m) => Ok(classify(m, GROK_FALLBACK_MODELS, &model)),
                        Err(e) => Err(format!("get_available_models failed: {e}")),
                    };
                    let generate_result = probe_generate(&a, &model).await;
                    Probe {
                        vendor: "Grok",
                        key_var: "XAI_API_KEY",
                        base_url,
                        default_model: model,
                        models_result,
                        generate_result,
                    }
                }
            }
        }
    });

    // ── Gemini (Google) ─────────────────────────────────────────────
    probes.push(match GeminiConfig::from_env() {
        Err(e) => Probe {
            vendor: "Gemini",
            key_var: "GEMINI_API_KEY",
            base_url: GEMINI_DEFAULT_BASE_URL.to_string(),
            default_model: GEMINI_DEFAULT_MODEL.to_string(),
            models_result: Err(e.clone()),
            generate_result: Err(e),
        },
        Ok(cfg) => {
            let (base_url, model) = (cfg.base_url.clone(), cfg.model.clone());
            match GeminiAdapter::new(cfg) {
                Err(e) => Probe {
                    vendor: "Gemini",
                    key_var: "GEMINI_API_KEY",
                    base_url,
                    default_model: model,
                    models_result: Err(e.to_string()),
                    generate_result: Err("adapter construction failed".to_string()),
                },
                Ok(a) => {
                    let models_result = match a.get_available_models().await {
                        Ok(m) => Ok(classify(m, GEMINI_FALLBACK_MODELS, &model)),
                        Err(e) => Err(format!("get_available_models failed: {e}")),
                    };
                    let generate_result = probe_generate(&a, &model).await;
                    Probe {
                        vendor: "Gemini",
                        key_var: "GEMINI_API_KEY",
                        base_url,
                        default_model: model,
                        models_result,
                        generate_result,
                    }
                }
            }
        }
    });

    let mut model_failures = 0usize;
    let mut generate_failures = 0usize;

    for p in &probes {
        println!("\n=== {} ({}) ===", p.vendor, p.key_var);
        let shipped_default = match p.vendor {
            "Kimi" => KIMI_DEFAULT_BASE_URL,
            "Qwen" => QWEN_DEFAULT_BASE_URL,
            "Grok" => GROK_DEFAULT_BASE_URL,
            _ => GEMINI_DEFAULT_BASE_URL,
        };
        println!(
            "  base_url      : {}{}",
            p.base_url,
            if p.base_url == shipped_default {
                String::new()
            } else {
                format!(
                    "\n                  [OVERRIDE via *_BASE_URL — shipped default is {shipped_default}]"
                )
            }
        );
        println!("  default model : {}", p.default_model);

        println!("  -- model list probe --");
        match &p.models_result {
            Err(e) => {
                println!("  RESULT        : FAIL — {e}");
                model_failures += 1;
            }
            Ok(live) => {
                println!("  models returned: {}", live.models.len());
                if live.is_fallback {
                    println!(
                        "  live fetch    : NO — result is byte-identical to the curated fallback"
                    );
                    println!("  RESULT        : FAIL (live-fetch path not exercised)");
                    model_failures += 1;
                } else {
                    println!("  live fetch    : YES — differs from curated fallback");
                    let mut sample: Vec<&str> = live.models.iter().map(|s| s.as_str()).collect();
                    sample.sort_unstable();
                    let shown: Vec<&str> = sample.iter().take(8).copied().collect();
                    println!(
                        "  sample        : {}{}",
                        shown.join(", "),
                        if sample.len() > 8 {
                            format!(", … (+{} more)", sample.len() - 8)
                        } else {
                            String::new()
                        }
                    );
                    if live.default_present {
                        println!("  default model in live list: YES");
                        println!("  RESULT        : PASS");
                    } else {
                        println!("  default model in live list: NO  <-- default model ID is wrong");
                        println!("  RESULT        : FAIL (default model absent from live list)");
                        model_failures += 1;
                    }
                }
            }
        }

        println!("  -- generate() probe (default prompt parameters) --");
        match &p.generate_result {
            Err(e) => {
                println!("  RESULT        : FAIL — {e}");
                generate_failures += 1;
            }
            Ok(g) => {
                println!(
                    "  content       : {} chars; tokens prompt={} completion={} total={}",
                    g.content_len, g.prompt_tokens, g.completion_tokens, g.total_tokens
                );
                println!("  RESULT        : PASS");
            }
        }
    }

    let total_probes = probes.len() * 2;
    let total_failures = model_failures + generate_failures;

    println!("\n──────────────────────────────────────────");
    println!(
        "{} of {} probes passed ({} vendors × 2 probes each; {} model-list failures, \
         {} generate failures)",
        total_probes - total_failures,
        total_probes,
        probes.len(),
        model_failures,
        generate_failures
    );
    if total_failures > 0 {
        std::process::exit(1);
    }
}
