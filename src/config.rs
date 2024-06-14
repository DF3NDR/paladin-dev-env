use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceConfig {
    pub url: String,
    pub source_type: String,
    pub prompt: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPreferences {
    pub summary_format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub sources: Vec<SourceConfig>,
    pub user_preferences: UserPreferences,
}

pub fn load_config(filename: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(filename)?;
    let config: Config = serde_yaml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        // Prepare a sample configuration YAML content
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
        let config = load_config(temp_file_path).expect("Failed to load config");

        // Check the loaded configuration
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].url, "https://example.com/rss");
        assert_eq!(config.sources[0].source_type, "rss");
        assert_eq!(config.sources[0].prompt, "Summarize the main points");
        assert_eq!(config.sources[0].tags, vec!["news", "tech"]);
        assert_eq!(config.user_preferences.summary_format, "brief");
    }
}
