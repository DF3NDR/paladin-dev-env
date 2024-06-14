#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub llm_type: String,
    pub url: String,
    pub api_key: String,
}

impl LlmConfig {
    pub fn new(llm_type: String, url: String, api_key: String) -> Self {
        LlmConfig { llm_type, url, api_key }
    }
}
