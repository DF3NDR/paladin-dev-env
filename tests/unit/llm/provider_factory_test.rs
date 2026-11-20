// tests/unit/llm/provider_factory_test.rs
//
// Unit tests for LLM provider factory

use paladin_llm::provider_factory::{LlmProviderFactory, ProviderFactoryError, provider_names};
use std::env;
use std::sync::Mutex;

/// Serializes access to every environment variable the ten-row registry
/// reads (`REGISTRY_ENV_VARS`) across every test in this file that reads or
/// mutates any of them. `cargo test` runs tests in parallel threads within
/// one process by default, and every test below observes or mutates this
/// same process-wide state (T-02-23).
static PROVIDER_ENV_LOCK: Mutex<()> = Mutex::new(());

/// The eleven environment variables the ten-row registry reads
/// (`build_provider_registry`), in registry declaration order, followed by
/// the generic `openai-compatible` provider's two non-credential
/// configuration variables (`OPENAI_COMPATIBLE_BASE_URL`,
/// `OPENAI_COMPATIBLE_MODEL`) last.
///
/// `get_default_provider()` and `list_available_providers()` both scan the
/// *entire* registry, so every variable any row reads must be owned by this
/// one list — a registry row whose variable this list omits is a test that
/// can observe the ambient environment.
const REGISTRY_ENV_VARS: [&str; 11] = [
    "OPENAI_API_KEY",
    "DEEPSEEK_API_KEY",
    "ANTHROPIC_API_KEY",
    "MOONSHOT_API_KEY",
    "DASHSCOPE_API_KEY",
    "XAI_API_KEY",
    "GEMINI_API_KEY",
    "ZAI_API_KEY",
    "OPENAI_COMPATIBLE_API_KEY",
    "OPENAI_COMPATIBLE_BASE_URL",
    "OPENAI_COMPATIBLE_MODEL",
];

/// RAII guard that clears every variable in [`REGISTRY_ENV_VARS`] for its
/// lifetime and restores each to its prior value (present-with-value or
/// absent) on drop — including on panic/unwind, so a failing assertion
/// never leaks state into a sibling test. Holds `PROVIDER_ENV_LOCK` for its
/// entire lifetime so no other test in this file can observe or mutate
/// these vars concurrently.
///
/// This also neutralises ambient values some sandboxes/CI harnesses
/// predefine (even an empty string counts as "set" to `std::env::var`,
/// which would otherwise make assertions below observe a polluted
/// environment rather than the clean one the tests intend).
///
/// **One guard, one lock, over all ten variables — not two guards over two
/// disjoint sets.** An earlier version of this file split the three
/// original providers' vars from the seven newer providers' vars on the
/// theory that the two sets were disjoint and therefore safe to guard
/// independently. That theory is false for the two functions under test:
/// `get_default_provider()` and `list_available_providers()` both scan the
/// *whole* registry, so a test holding only one of two guards could still
/// observe the ambient value of a variable the other guard owned — which is
/// exactly how CR-01's regression hid behind a passing test suite. A second
/// lock would also add an ordering hazard (two mutexes, no declared
/// acquisition order) with no isolation benefit. One guard removes both
/// problems.
struct CleanProviderEnv<'a> {
    _lock: std::sync::MutexGuard<'a, ()>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl CleanProviderEnv<'_> {
    fn acquire() -> Self {
        let lock = PROVIDER_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let saved: Vec<(&'static str, Option<String>)> = REGISTRY_ENV_VARS
            .iter()
            .map(|&var| (var, env::var(var).ok()))
            .collect();

        for &var in &REGISTRY_ENV_VARS {
            // SAFETY: PROVIDER_ENV_LOCK is held for this guard's lifetime
            // (via `_lock`), so no other test in this binary can observe or
            // mutate these process-wide vars while we hold it (T-02-23).
            unsafe {
                env::remove_var(var);
            }
        }

        Self { _lock: lock, saved }
    }

    /// Set one of the ten owned variables for the remainder of this
    /// guard's lifetime.
    fn set(&self, var: &str, value: &str) {
        debug_assert!(
            REGISTRY_ENV_VARS.contains(&var),
            "{var} is not one of the variables this guard owns and restores on drop"
        );
        // SAFETY: `_lock` is held for this guard's entire lifetime, so no
        // other test in this binary can observe or mutate `var`
        // concurrently.
        unsafe {
            env::set_var(var, value);
        }
    }
}

impl Drop for CleanProviderEnv<'_> {
    fn drop(&mut self) {
        // SAFETY: `_lock` (a sibling field) is still held for the duration
        // of this `Drop::drop` call — Rust drops struct fields after the
        // `Drop` impl runs — so this restore cannot race a sibling test.
        for (var, value) in &self.saved {
            unsafe {
                match value.as_deref() {
                    Some(v) => env::set_var(var, v),
                    None => env::remove_var(var),
                }
            }
        }
    }
}

/// The provider/credential-variable pairs among the registry's rows that
/// require a credential, in registry declaration order (see
/// `build_provider_registry`). `ollama` is deliberately absent — its row
/// declares `env_var: None` (D-12, no credential required) — and this
/// table's order must track `build_provider_registry`'s.
const CREDENTIALED_PROVIDERS: [(&str, &str); 9] = [
    ("openai", "OPENAI_API_KEY"),
    ("deepseek", "DEEPSEEK_API_KEY"),
    ("anthropic", "ANTHROPIC_API_KEY"),
    ("kimi", "MOONSHOT_API_KEY"),
    ("qwen", "DASHSCOPE_API_KEY"),
    ("grok", "XAI_API_KEY"),
    ("gemini", "GEMINI_API_KEY"),
    ("zai", "ZAI_API_KEY"),
    ("openai-compatible", "OPENAI_COMPATIBLE_API_KEY"),
];

/// The set of provider names `list_available_providers()` reports when
/// every variable in [`REGISTRY_ENV_VARS`] is cleared — i.e. exactly the
/// compiled-in rows that need no credential at all. Must be called while a
/// [`CleanProviderEnv`] guard holds every variable cleared.
///
/// Under the root default feature set (`openai`, `anthropic`, `deepseek`
/// only, D-11) this is empty. Under `--features llm-all` it is
/// `["ollama"]`, because Ollama's row declares `env_var: None` (D-12) and
/// therefore always qualifies, declaration order and nothing else.
fn credential_free_baseline() -> Vec<String> {
    LlmProviderFactory::list_available_providers()
}

/// The first entry of [`CREDENTIALED_PROVIDERS`] whose name is compiled
/// into this build (i.e. appears in [`provider_names`]), preserving
/// `CREDENTIALED_PROVIDERS`' own (registry) declaration order, or `None` if
/// this build compiles in no credentialed provider at all.
fn first_compiled_credentialed_provider() -> Option<(&'static str, &'static str)> {
    let compiled = provider_names();
    CREDENTIALED_PROVIDERS
        .iter()
        .copied()
        .find(|(name, _)| compiled.contains(name))
}

#[test]
fn test_factory_provider_selection() {
    let _env = CleanProviderEnv::acquire();
    let factory = LlmProviderFactory::new();

    // Test unknown provider error
    let result = factory.create("unknown_provider");
    assert!(result.is_err());
    // `Arc<dyn LlmPort>` (the Ok type) is not `Debug`, so `Result::unwrap_err`
    // (which requires `T: Debug`) cannot be used here — go via `err()`
    // (`Option<E>`) then `Option::unwrap`, which has no such bound.
    assert!(matches!(
        result.err().unwrap(),
        ProviderFactoryError::UnknownProvider(_)
    ));

    // Test that provider names are recognized (even if config is missing)
    let result_openai = factory.create("openai");
    let result_deepseek = factory.create("deepseek");
    let result_anthropic = factory.create("anthropic");

    // All should fail with ConfigurationMissing (not UnknownProvider)
    assert!(matches!(
        result_openai.err().unwrap(),
        ProviderFactoryError::ConfigurationMissing(_)
    ));
    assert!(matches!(
        result_deepseek.err().unwrap(),
        ProviderFactoryError::ConfigurationMissing(_)
    ));
    assert!(matches!(
        result_anthropic.err().unwrap(),
        ProviderFactoryError::ConfigurationMissing(_)
    ));
}

#[test]
fn test_factory_config_validation() {
    let _env = CleanProviderEnv::acquire();
    // Test that factory properly validates configurations
    let factory = LlmProviderFactory::new();

    // Test with missing environment variables
    let result = factory.create("deepseek");
    assert!(result.is_err());

    let error = result.err().unwrap();
    match error {
        ProviderFactoryError::ConfigurationMissing(msg) => {
            assert!(msg.contains("DEEPSEEK_API_KEY"));
        }
        _ => panic!("Expected ConfigurationMissing error"),
    }
}

#[test]
fn test_factory_case_insensitive() {
    let _env = CleanProviderEnv::acquire();
    let factory = LlmProviderFactory::new();

    // All case variations should be recognized
    let result1 = factory.create("OpenAI");
    let result2 = factory.create("OPENAI");
    let result3 = factory.create("openai");

    // All should fail with ConfigurationMissing (not UnknownProvider)
    for result in [result1, result2, result3] {
        assert!(matches!(
            result.err().unwrap(),
            ProviderFactoryError::ConfigurationMissing(_)
        ));
    }
}

#[test]
fn test_factory_error_messages() {
    let factory = LlmProviderFactory::new();

    // Test unknown provider error message
    let result = factory.create("invalid_provider");
    assert!(result.is_err());

    let error_msg = result.err().unwrap().to_string();
    assert!(error_msg.contains("Unknown provider"));
    assert!(error_msg.contains("invalid_provider"));
    assert!(error_msg.contains("Supported providers"));
}

#[test]
fn test_get_default_provider() {
    // CleanProviderEnv::acquire() takes PROVIDER_ENV_LOCK, saves the current
    // value of every registry-read var, clears them, and restores the
    // saved values on drop (including on panic) — see its doc comment.
    let _env = CleanProviderEnv::acquire();

    // No providers configured — the answer is whatever credential-free
    // baseline row is compiled in (empty under the root default feature
    // set, `Some("ollama")` under `--features llm-all`, since Ollama's row
    // needs no credential, D-12). A hardcoded `None` here would be wrong
    // under any build that compiles Ollama in.
    let baseline = credential_free_baseline();
    assert_eq!(
        LlmProviderFactory::get_default_provider(),
        baseline.first().cloned(),
        "with no credentials configured, get_default_provider() must return the first \
         credential-free baseline row (compiled providers: {:?}, baseline: {baseline:?})",
        provider_names(),
    );

    // Only Anthropic configured - should be selected. This assertion (and
    // the two that follow) are feature-set-independent: openai, deepseek
    // and anthropic are all curated presets declared before every
    // credential-free row in `build_provider_registry` (placement
    // rationale documented there), so their expected values hold under any
    // feature set this test binary happens to be compiled with.
    // SAFETY: test-owned registry var, restored at the end of this test.
    unsafe {
        env::set_var("ANTHROPIC_API_KEY", "test-key");
    }
    assert_eq!(
        LlmProviderFactory::get_default_provider(),
        Some("anthropic".to_string())
    );

    // Add DeepSeek - should be selected over Anthropic
    // SAFETY: test-owned registry var, restored at the end of this test.
    unsafe {
        env::set_var("DEEPSEEK_API_KEY", "test-key");
    }
    assert_eq!(
        LlmProviderFactory::get_default_provider(),
        Some("deepseek".to_string())
    );

    // Add OpenAI - should be selected as highest priority
    // SAFETY: test-owned registry var, restored at the end of this test.
    unsafe {
        env::set_var("OPENAI_API_KEY", "test-key");
    }
    assert_eq!(
        LlmProviderFactory::get_default_provider(),
        Some("openai".to_string())
    );

    // `_env`'s Drop impl restores the pre-test environment when it goes out
    // of scope at the end of this function.
}

#[test]
fn test_list_available_providers() {
    // See test_get_default_provider's comment: this guard clears every
    // registry-read var for the duration of this test and restores their
    // pre-test values (including on panic) when it is dropped.
    let _env = CleanProviderEnv::acquire();

    let baseline = credential_free_baseline();

    // No providers configured beyond whatever credential-free baseline rows
    // are compiled in.
    let providers = LlmProviderFactory::list_available_providers();
    assert_eq!(providers.len(), baseline.len());
    for name in &baseline {
        assert!(providers.contains(name));
    }

    // Add one provider
    // SAFETY: test-owned registry var, restored at the end of this test.
    unsafe {
        env::set_var("DEEPSEEK_API_KEY", "test-key");
    }
    let providers = LlmProviderFactory::list_available_providers();
    assert_eq!(providers.len(), baseline.len() + 1);
    assert!(providers.contains(&"deepseek".to_string()));
    for name in &baseline {
        assert!(providers.contains(name));
    }

    // Add all providers
    // SAFETY: test-owned registry var, restored at the end of this test.
    unsafe {
        env::set_var("OPENAI_API_KEY", "test-key");
    }
    // SAFETY: test-owned registry var, restored at the end of this test.
    unsafe {
        env::set_var("ANTHROPIC_API_KEY", "test-key");
    }
    let providers = LlmProviderFactory::list_available_providers();
    assert_eq!(providers.len(), baseline.len() + 3);
    assert!(providers.contains(&"openai".to_string()));
    assert!(providers.contains(&"deepseek".to_string()));
    assert!(providers.contains(&"anthropic".to_string()));
    for name in &baseline {
        assert!(providers.contains(name));
    }

    // The returned order is stable: the sequence of non-baseline names
    // appears in CREDENTIALED_PROVIDERS order (which mirrors registry
    // declaration order).
    let non_baseline: Vec<&String> = providers.iter().filter(|n| !baseline.contains(n)).collect();
    let expected_order: Vec<&str> = CREDENTIALED_PROVIDERS
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| non_baseline.iter().any(|n| n.as_str() == *name))
        .collect();
    assert_eq!(
        non_baseline.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        expected_order,
        "list_available_providers() must preserve registry declaration order"
    );

    // `_env`'s Drop impl restores the pre-test environment when it goes out
    // of scope at the end of this function.
}

/// D-11 (amended 2026-08-17, option-b) / ADR-0046 regression guard: the
/// facade's `llm-*` feature flags are now real gates on `paladin-llm`'s own
/// features rather than inert stubs, but `default = ["llm-openai",
/// "llm-anthropic", "llm-deepseek"]` must still compile — and therefore
/// still resolve — exactly the same three providers a default build did
/// before this plan. A `cargo tree` assertion alone proves the feature
/// graph is wired; this test proves the runtime consequence (`create(..)`
/// still finding a registered row) actually holds under whatever features
/// this test binary was compiled with.
#[test]
fn default_features_still_resolve_openai_anthropic_and_deepseek() {
    let _env = CleanProviderEnv::acquire();
    let factory = LlmProviderFactory::new();

    for name in ["openai", "anthropic", "deepseek"] {
        // `Arc<dyn LlmPort>` (the `Ok` type) is not `Debug`, so the whole
        // `Result` cannot be formatted with `{:?}` — match on a reference
        // and report only the `Err` variant when the assertion trips.
        let result = factory.create(name);
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "provider {name:?} must still resolve under default features (D-11/ADR-0046); got {:?}",
            result.as_ref().err()
        );
    }
}

/// D-10 regression guard (RESEARCH.md Pitfall 1/4, plan 17-04's own remit):
/// a provider whose feature is not compiled into this test binary must be
/// absent from `list_available_providers()` even when its credential env
/// var is set. `paladin-llm` denies `unsafe_code`, so this env-mutating
/// variant of the regression lives in this workspace-level test crate
/// instead (Task 1's own action text).
///
/// Written against whichever of the four credentialed new-provider
/// candidates this workspace's `unit` test target's *currently-active*
/// feature set genuinely does not compile in — determined at runtime via
/// [`provider_names`], never assumed. Under this root crate's default
/// feature set (`default = ["llm-openai", "llm-anthropic", "llm-deepseek"]`,
/// D-11) all four are compiled out, but the lookup stays correct under any
/// combination this test binary might be built with. If every candidate
/// happens to be compiled in (e.g. a `--features llm-all` run), the plan's
/// own fallback applies: assert the structural invariant that makes the
/// defect impossible — every reported-available name is in the compiled-in
/// registry — instead of the env-mutating single-name form.
#[test]
fn test_compiled_out_provider_absent_from_list_available_providers() {
    let _env = CleanProviderEnv::acquire();
    let compiled = provider_names();

    let candidates: &[(&str, &str)] = &[
        ("kimi", "MOONSHOT_API_KEY"),
        ("qwen", "DASHSCOPE_API_KEY"),
        ("grok", "XAI_API_KEY"),
        ("gemini", "GEMINI_API_KEY"),
        ("zai", "ZAI_API_KEY"),
    ];

    let Some(&(name, env_var)) = candidates.iter().find(|(name, _)| !compiled.contains(name))
    else {
        assert!(
            LlmProviderFactory::list_available_providers()
                .iter()
                .all(|available| compiled.contains(&available.as_str())),
            "every name list_available_providers() reports must be in the compiled-in registry \
             — the env-mutating compiled-out-provider form was not expressible under this \
             build's feature set, which compiles every one of kimi/qwen/grok/gemini/zai"
        );
        return;
    };

    _env.set(env_var, "test-key-set-despite-feature-being-compiled-out");

    assert!(
        !LlmProviderFactory::list_available_providers()
            .iter()
            .any(|available| available == name),
        "{name} must be absent from list_available_providers() even with {env_var} set, because \
         its feature is not compiled into this build (D-10 regression)"
    );
}

/// D-10 regression: every new provider name this build's compiled feature
/// set includes resolves through `create()` once its credential is set
/// (Ollama needs none, D-12), and a name that is not compiled in at all
/// always returns `UnknownProvider` whose message lists only the compiled-in
/// names.
#[test]
fn test_new_provider_names_resolve_through_create() {
    let _env = CleanProviderEnv::acquire();
    let compiled = provider_names();
    let factory = LlmProviderFactory::new();

    if compiled.contains(&"kimi") {
        _env.set("MOONSHOT_API_KEY", "test-key");
        let result = factory.create("kimi");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "kimi is compiled in and its credential is set; create() must not return \
             UnknownProvider: {:?}",
            result.as_ref().err()
        );
    }
    if compiled.contains(&"qwen") {
        _env.set("DASHSCOPE_API_KEY", "test-key");
        let result = factory.create("qwen");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "qwen is compiled in and its credential is set; create() must not return \
             UnknownProvider: {:?}",
            result.as_ref().err()
        );
    }
    if compiled.contains(&"grok") {
        _env.set("XAI_API_KEY", "test-key");
        let result = factory.create("grok");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "grok is compiled in and its credential is set; create() must not return \
             UnknownProvider: {:?}",
            result.as_ref().err()
        );
    }
    if compiled.contains(&"gemini") {
        _env.set("GEMINI_API_KEY", "test-key");
        let result = factory.create("gemini");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "gemini is compiled in and its credential is set; create() must not return \
             UnknownProvider: {:?}",
            result.as_ref().err()
        );
    }
    if compiled.contains(&"zai") {
        _env.set("ZAI_API_KEY", "test-key");
        let result = factory.create("zai");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "zai is compiled in and its credential is set; create() must not return \
             UnknownProvider: {:?}",
            result.as_ref().err()
        );
    }
    if compiled.contains(&"ollama") {
        // No credential env var to set — Ollama requires none (D-12).
        let result = factory.create("ollama");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "ollama is compiled in and requires no credential; create() must not return \
             UnknownProvider: {:?}",
            result.as_ref().err()
        );
    }
    if compiled.contains(&"openai-compatible") {
        _env.set("OPENAI_COMPATIBLE_API_KEY", "test-key");
        _env.set("OPENAI_COMPATIBLE_BASE_URL", "http://localhost:8080");
        _env.set("OPENAI_COMPATIBLE_MODEL", "test-model");
        let result = factory.create("openai-compatible");
        assert!(
            !matches!(result, Err(ProviderFactoryError::UnknownProvider(_))),
            "openai-compatible is compiled in and its three required vars are set; create() \
             must not return UnknownProvider: {:?}",
            result.as_ref().err()
        );
    }

    // A name that is not compiled in — regardless of which feature set this
    // binary was built with — must always return UnknownProvider, with a
    // message that lists every compiled-in name and never the bogus one.
    let bogus_name = "definitely-not-a-registered-provider";
    let result = factory.create(bogus_name);
    match result {
        Err(ProviderFactoryError::UnknownProvider(name)) => {
            assert_eq!(name, bogus_name);
        }
        Err(other) => panic!("expected UnknownProvider, got Err({other})"),
        Ok(_) => panic!("expected UnknownProvider, got Ok(_)"),
    }

    // The message's Display impl necessarily echoes the *requested* name back
    // (`"Unknown provider: {bogus_name}. Supported providers: ..."`), so the
    // "does not list the bogus name" half of this assertion must look only at
    // the supported-providers segment, not the whole message.
    let message = ProviderFactoryError::UnknownProvider(bogus_name.to_string()).to_string();
    let supported_segment = message
        .split_once("Supported providers: ")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| {
            panic!("UnknownProvider message missing 'Supported providers: ' segment: {message}")
        });
    for name in &compiled {
        assert!(
            supported_segment.contains(*name),
            "UnknownProvider message's supported-providers segment must list every compiled-in \
             provider name; missing {name:?}: {message}"
        );
    }
    assert!(
        !supported_segment.contains(bogus_name),
        "UnknownProvider message's supported-providers segment must not list the bogus name: \
         {message}"
    );
}

// ── CR-01: blank credentials are not configured credentials ──
//
// `get_default_provider()` and `list_available_providers()` currently
// decide a provider's credential is "configured" with a bare presence
// check (`std::env::var(var).is_ok()`), which is `true` for an environment
// variable set to the empty string or to whitespace only. Every adapter's
// own `*Config::validate()` rejects such a value, so `create()` then fails
// with `ConfigurationMissing` for a provider these two functions just
// reported as available. These four tests isolate that defect: the first
// three fail against the current production code (RED); the fourth is a
// positive control that passes before and after the fix.

#[test]
fn an_empty_credential_env_var_is_not_a_configured_provider() {
    let _env = CleanProviderEnv::acquire();
    let Some((name, var)) = first_compiled_credentialed_provider() else {
        eprintln!(
            "skipping an_empty_credential_env_var_is_not_a_configured_provider — no \
             credentialed provider is compiled into this build"
        );
        return;
    };
    let baseline = credential_free_baseline();

    _env.set(var, "");

    assert!(
        !LlmProviderFactory::list_available_providers()
            .iter()
            .any(|p| p == name),
        "{name} must be absent from list_available_providers() when its credential variable \
         ({var}) is set to the empty string"
    );
    assert_eq!(
        LlmProviderFactory::list_available_providers(),
        baseline,
        "list_available_providers() must equal the credential-free baseline when the only \
         configured credential is blank"
    );
    assert_ne!(
        LlmProviderFactory::get_default_provider(),
        Some(name.to_string()),
        "get_default_provider() must not name {name} when its credential variable ({var}) is \
         blank"
    );
}

#[test]
fn a_whitespace_only_credential_env_var_is_not_a_configured_provider() {
    let _env = CleanProviderEnv::acquire();
    let Some((name, var)) = first_compiled_credentialed_provider() else {
        eprintln!(
            "skipping a_whitespace_only_credential_env_var_is_not_a_configured_provider — no \
             credentialed provider is compiled into this build"
        );
        return;
    };
    let baseline = credential_free_baseline();

    _env.set(var, "   ");

    assert!(
        !LlmProviderFactory::list_available_providers()
            .iter()
            .any(|p| p == name),
        "{name} must be absent from list_available_providers() when its credential variable \
         ({var}) is set to ASCII whitespace only"
    );
    assert_eq!(
        LlmProviderFactory::list_available_providers(),
        baseline,
        "list_available_providers() must equal the credential-free baseline when the only \
         configured credential is whitespace-only"
    );
    assert_ne!(
        LlmProviderFactory::get_default_provider(),
        Some(name.to_string()),
        "get_default_provider() must not name {name} when its credential variable ({var}) is \
         whitespace-only"
    );
}

#[test]
fn a_non_ascii_whitespace_credential_env_var_is_not_a_configured_provider() {
    let _env = CleanProviderEnv::acquire();
    let Some((name, var)) = first_compiled_credentialed_provider() else {
        eprintln!(
            "skipping a_non_ascii_whitespace_credential_env_var_is_not_a_configured_provider — \
             no credentialed provider is compiled into this build"
        );
        return;
    };
    let baseline = credential_free_baseline();

    // A single U+00A0 NO-BREAK SPACE — non-ASCII Unicode whitespace. Rust's
    // `str::trim` strips the full Unicode `White_Space` set, not just ASCII
    // blanks, so this must behave identically to the ASCII-whitespace case.
    _env.set(var, "\u{00A0}");

    assert!(
        !LlmProviderFactory::list_available_providers()
            .iter()
            .any(|p| p == name),
        "{name} must be absent from list_available_providers() when its credential variable \
         ({var}) is a single U+00A0 NO-BREAK SPACE"
    );
    assert_eq!(
        LlmProviderFactory::list_available_providers(),
        baseline,
        "list_available_providers() must equal the credential-free baseline when the only \
         configured credential is non-ASCII-whitespace-only"
    );
    assert_ne!(
        LlmProviderFactory::get_default_provider(),
        Some(name.to_string()),
        "get_default_provider() must not name {name} when its credential variable ({var}) is \
         non-ASCII-whitespace-only"
    );
}

/// Positive control — passes before and after the fix. Proves the stricter
/// check does not become a rejection of everything.
#[test]
fn a_real_credential_env_var_is_a_configured_provider() {
    let _env = CleanProviderEnv::acquire();
    let Some((name, var)) = first_compiled_credentialed_provider() else {
        eprintln!(
            "skipping a_real_credential_env_var_is_a_configured_provider — no credentialed \
             provider is compiled into this build"
        );
        return;
    };

    _env.set(var, "a-real-non-blank-credential-value");

    assert!(
        LlmProviderFactory::list_available_providers()
            .iter()
            .any(|p| p == name),
        "{name} must be present in list_available_providers() when its credential variable \
         ({var}) holds a real, non-blank value"
    );
    assert_eq!(
        LlmProviderFactory::get_default_provider(),
        Some(name.to_string()),
        "get_default_provider() must name {name} when it is the first row (in \
         CREDENTIALED_PROVIDERS / registry declaration order) with a real credential and no \
         earlier row qualifies"
    );
}

// ── declaration-order stability ──
//
// `list_available_providers()`'s output must be a subsequence of
// `provider_names()` in the same relative registry declaration order —
// never re-sorted, never grouped by credential state — and
// `get_default_provider()`'s tie-break, when two or more rows qualify
// simultaneously, must be exactly that declaration order and nothing else.
// `build_provider_registry`'s three placement-rationale comments already
// specify this; these two tests pin it as an executable contract so a
// future reordering shows up as a failing assertion rather than a silent
// behaviour change.

#[test]
fn list_available_providers_preserves_registry_declaration_order() {
    let _env = CleanProviderEnv::acquire();
    let compiled = provider_names();

    let compiled_credentialed: Vec<(&str, &str)> = CREDENTIALED_PROVIDERS
        .iter()
        .copied()
        .filter(|(name, _)| compiled.contains(name))
        .collect();

    if compiled_credentialed.len() < 2 {
        eprintln!(
            "skipping list_available_providers_preserves_registry_declaration_order — fewer \
             than two credentialed providers are compiled into this build"
        );
        return;
    }

    for &(_, var) in &compiled_credentialed {
        _env.set(var, "test-credential-value");
    }
    if compiled.contains(&"openai-compatible") {
        _env.set("OPENAI_COMPATIBLE_BASE_URL", "http://localhost:8080");
        _env.set("OPENAI_COMPATIBLE_MODEL", "test-model");
    }

    // Every credentialed row now holds a real credential and Ollama (if
    // compiled in) needs none, so every compiled-in row qualifies — the
    // returned list must equal provider_names() itself, element-for-element,
    // in registry declaration order.
    let expected: Vec<String> = compiled.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        LlmProviderFactory::list_available_providers(),
        expected,
        "list_available_providers() must return exactly provider_names() filtered to \
         compiled-in rows, element-for-element, in registry declaration order — this is what \
         makes CR-01's fix a predicate change rather than a scan change"
    );
}

#[test]
fn get_default_provider_breaks_ties_by_declaration_order() {
    let _env = CleanProviderEnv::acquire();
    let compiled = provider_names();

    let compiled_credentialed: Vec<(&str, &str)> = CREDENTIALED_PROVIDERS
        .iter()
        .copied()
        .filter(|(name, _)| compiled.contains(name))
        .collect();

    if compiled_credentialed.len() < 2 {
        eprintln!(
            "skipping get_default_provider_breaks_ties_by_declaration_order — fewer than two \
             credentialed providers are compiled into this build"
        );
        return;
    }

    for &(_, var) in &compiled_credentialed {
        _env.set(var, "test-credential-value");
    }
    if compiled.contains(&"openai-compatible") {
        _env.set("OPENAI_COMPATIBLE_BASE_URL", "http://localhost:8080");
        _env.set("OPENAI_COMPATIBLE_MODEL", "test-model");
    }

    // With every credentialed row configured, the answer is the very first
    // name in provider_names() — registry declaration order, and nothing
    // else. Ollama's `env_var: None` row is declared last precisely so it
    // can never pre-empt a configured named provider here (see the
    // placement rationale on `build_provider_registry`).
    assert_eq!(
        LlmProviderFactory::get_default_provider(),
        compiled.first().map(|s| s.to_string())
    );
    assert_eq!(
        compiled.first().copied(),
        compiled_credentialed.first().map(|&(name, _)| name),
        "the first row in registry declaration order must be a credentialed row whenever at \
         least one is compiled in — Ollama's credential-free row is always declared last"
    );

    // Clear the first compiled-in credentialed provider's variable; the
    // answer must advance to the next compiled-in credentialed name in
    // declaration order — never to Ollama's credential-free baseline row.
    _env.set(compiled_credentialed[0].1, "");
    assert_eq!(
        LlmProviderFactory::get_default_provider(),
        Some(compiled_credentialed[1].0.to_string()),
        "clearing {}'s credential must advance get_default_provider() to the next compiled-in \
         credentialed name in declaration order ({}), not to a credential-free baseline row",
        compiled_credentialed[0].0,
        compiled_credentialed[1].0
    );

    // Repeat the advance once more, proving a scan rather than a two-case
    // lookup — only when a third credentialed row is compiled in.
    if compiled_credentialed.len() >= 3 {
        _env.set(compiled_credentialed[1].1, "");
        assert_eq!(
            LlmProviderFactory::get_default_provider(),
            Some(compiled_credentialed[2].0.to_string()),
            "clearing {}'s credential too must advance get_default_provider() again, to {}, \
             proving a scan and not a two-case lookup",
            compiled_credentialed[1].0,
            compiled_credentialed[2].0
        );
    }
}

#[test]
fn test_factory_zero_sized() {
    let factory = LlmProviderFactory::new();
    // Factory should be zero-sized (no runtime cost)
    assert_eq!(std::mem::size_of_val(&factory), 0);
}

#[test]
// The whole point of this test is exercising the `Default` trait impl itself
// (not just constructing the unit struct), so the "just write the struct
// literal" suggestion would defeat what is being tested.
#[allow(clippy::default_constructed_unit_structs)]
fn test_factory_default() {
    let factory = LlmProviderFactory::default();
    assert_eq!(std::mem::size_of_val(&factory), 0);
}
