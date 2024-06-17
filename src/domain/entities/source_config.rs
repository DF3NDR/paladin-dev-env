use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceConfig {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub prompt: String,
    pub tags: Vec<String>,
}
