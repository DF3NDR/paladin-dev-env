// src/infrastructure/adapters/llm/config_bridge.rs
//
// The `From<&LlmProviderConfig>` conversions for OpenAI, Anthropic, DeepSeek
// and the vision `From<&VisionConfig>` conversion now live in
// `crates/paladin-llm/src/config/bridge.rs`.  Both source and target types
// are defined in `paladin-llm`, so the impls must live there (orphan rule).
//
// This file is kept for the integration-level tests that exercise those
// conversions from the root-crate perspective.

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use paladin_llm::config::llm::LlmProviderConfig;

    #[cfg(feature = "llm-openai")]
    use paladin_llm::openai::adapter::OpenAIConfig;

    #[cfg(feature = "llm-anthropic")]
    use paladin_llm::anthropic::adapter::AnthropicConfig;

    #[cfg(feature = "llm-deepseek")]
    use paladin_llm::deepseek::adapter::DeepSeekConfig;

    fn typical_provider_config() -> LlmProviderConfig {
        LlmProviderConfig::new("sk-test-key")
            .with_base_url("https://custom.example.com/v1")
            .with_default_model("gpt-4o")
            .with_default_temperature(0.7)
            .with_timeout_seconds(120)
            .with_max_retries(5)
    }

    fn minimal_provider_config() -> LlmProviderConfig {
        LlmProviderConfig::new("sk-minimal")
    }

    // ── OpenAI ──────────────────────────────────────────────────────────────

    #[cfg(feature = "llm-openai")]
    #[test]
    fn test_openai_config_from_provider_config_typical() {
        let provider = typical_provider_config();
        let cfg = OpenAIConfig::from(&provider);
        assert_eq!(cfg.api_key, "sk-test-key");
        assert_eq!(cfg.base_url, "https://custom.example.com/v1");
        assert_eq!(cfg.timeout_seconds, 120);
        assert_eq!(cfg.max_retries, 5);
        assert!(cfg.organization.is_none());
    }

    #[cfg(feature = "llm-openai")]
    #[test]
    fn test_openai_config_from_provider_config_defaults() {
        let provider = minimal_provider_config();
        let cfg = OpenAIConfig::from(&provider);
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
        assert_eq!(cfg.timeout_seconds, 300);
        assert_eq!(cfg.max_retries, 3);
    }

    // ── Anthropic ───────────────────────────────────────────────────────────

    #[cfg(feature = "llm-anthropic")]
    #[test]
    fn test_anthropic_config_from_provider_config_typical() {
        let provider = typical_provider_config();
        let cfg = AnthropicConfig::from(&provider);
        assert_eq!(cfg.api_key, "sk-test-key");
        assert_eq!(cfg.base_url, "https://custom.example.com/v1");
        assert_eq!(cfg.model, "gpt-4o");
        assert_eq!(cfg.timeout_seconds, 120);
    }

    #[cfg(feature = "llm-anthropic")]
    #[test]
    fn test_anthropic_config_from_provider_config_defaults() {
        let provider = minimal_provider_config();
        let cfg = AnthropicConfig::from(&provider);
        assert_eq!(cfg.base_url, "https://api.anthropic.com/v1");
        // Plan 50-09: the bridge's stale defaults (the October-2024 Sonnet id, a 300 s timeout)
        // are the adapter's own compiled ones now.
        assert_eq!(
            cfg.model,
            paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL
        );
        assert_eq!(cfg.timeout_seconds, 60);
    }

    // ── DeepSeek ────────────────────────────────────────────────────────────

    #[cfg(feature = "llm-deepseek")]
    #[test]
    fn test_deepseek_config_from_provider_config_typical() {
        let provider = typical_provider_config();
        let cfg = DeepSeekConfig::from(&provider);
        assert_eq!(cfg.api_key, "sk-test-key");
        assert_eq!(cfg.base_url, "https://custom.example.com/v1");
        assert_eq!(cfg.model, "gpt-4o");
        assert_eq!(cfg.timeout_seconds, 120);
    }

    #[cfg(feature = "llm-deepseek")]
    #[test]
    fn test_deepseek_config_from_provider_config_defaults() {
        let provider = minimal_provider_config();
        let cfg = DeepSeekConfig::from(&provider);
        assert_eq!(cfg.base_url, "https://api.deepseek.com/v1");
        // Plan 50-09: the retired `deepseek-chat` default is the adapter's current default
        // model, a seeded reasoning model, so it gets the 300 s reasoning timeout (D-13).
        assert_eq!(
            cfg.model,
            paladin_llm::deepseek::adapter::DEEPSEEK_DEFAULT_MODEL
        );
        assert_eq!(cfg.timeout_seconds, 300);
    }
}
