use std::env;
use std::fs;
use crate::configuration::Settings;
use crate::llm_config::LlmConfig;

pub fn setup_test_config() -> Settings {
    let yaml_content = r#"
    server:
      host: "127.0.0.1"
      port: 8080
    rust_log: "info"
    llm_type: "openai"
    llm_url: "https://api.openai.com/v1/engines/davinci-codex/completions"
    llm_api_key: "your_test_api_key"
    sources:
      - url: "https://example.com/rss"
        source_type: "rss"
        prompt: "Summarize the main points"
        tags: ["news", "tech"]
    user_preferences:
      summary_format: "brief"
    "#;

    // Write the content to a temporary file
    let temp_file_path = "/tmp/test_config.yaml";
    fs::write(temp_file_path, yaml_content).expect("Unable to write test config file");

    // Load the configuration
    Settings::load_from_file(temp_file_path).expect("Failed to load config")
}

pub fn get_env_value(name: &str) -> String {
    env::var(name).expect(&format!("{} must be set", name))
}

pub fn get_llm_config() -> LlmConfig {
    let llm_type = get_env_value("LLM_TYPE");
    let url = get_env_value("LLM_URL");
    let api_key = get_env_value("LLM_API_KEY");
    LlmConfig::new(llm_type, url, api_key)
}
