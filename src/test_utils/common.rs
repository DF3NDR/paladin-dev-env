use dotenv::dotenv;
use std::env;
use std::fs;
use crate::config::{Config, load_config};
use crate::llm_config::LlmConfig;

pub fn setup_test_config() -> Config {
    dotenv().ok();

    let yaml_content = r#"
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
    load_config(temp_file_path).expect("Failed to load config")
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
