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
        let config = load_config("config.yaml").unwrap();
        assert!(!config.sources.is_empty());
        assert!(!config.user_preferences.summary_format.is_empty());
    }
}
