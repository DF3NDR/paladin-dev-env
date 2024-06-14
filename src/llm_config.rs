#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub url: String,
    pub api_key: String,
}

impl LlmConfig {
    pub fn new(url: String, api_key: String) -> Self {
        LlmConfig { url, api_key }
    }
}
