use serde::{Deserialize, Serialize};
use config::{Config, ConfigError, File, Environment};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceConfig {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub prompt: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub llm_type: String,
    pub llm_url: String,
    pub llm_api_key: String,
    pub server: ServerConfig,
    pub sources: Vec<SourceConfig>,
    pub max_file_size: u64,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            .add_source(File::with_name("config").required(true))
            .add_source(Environment::with_prefix("APP"));

        if let Ok(env) = std::env::var("APP_ENV") {
            builder = builder.add_source(File::with_name(&format!("config.{}", env)).required(false));
        }

        builder.build()?.try_deserialize()
    }

    pub fn load_from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(filename)?;
        let config: Settings = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the maximum file size limit in bytes
    /// Returns a default of 10MB if no configuration is available
    pub fn max_file_size() -> u64 {
        // Try to load from environment variable first
        if let Ok(size_str) = std::env::var("APP_MAX_FILE_SIZE") {
            if let Ok(size) = size_str.parse::<u64>() {
                return size;
            }
        }
        
        // Try to load from config file if possible
        if let Ok(settings) = Self::new() {
            return settings.max_file_size;
        }
        
        // Default to 10MB (10 * 1024 * 1024 bytes)
        10 * 1024 * 1024
    }

    /// Get the maximum file size for this specific settings instance
    pub fn get_max_file_size(&self) -> u64 {
        self.max_file_size
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llm_type: "openai".to_string(),
            llm_url: "https://api.openai.com/v1".to_string(),
            llm_api_key: "".to_string(),
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            sources: Vec::new(),
            max_file_size: 10 * 1024 * 1024, // 10MB default
        }
    }
}