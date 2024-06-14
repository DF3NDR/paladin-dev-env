use serde_json::Value;
use crate::error::FetchError;
use crate::llm_config::LlmConfig;

pub async fn analyze_data(data: &str, prompt: &str, config: &LlmConfig) -> Result<Value, FetchError> {
    let client = reqwest::Client::new();
    let response = match config.llm_type.as_str() {
        "openai" => {
            client.post(&config.url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .json(&serde_json::json!({
                    "prompt": format!("{}: {}", prompt, data),
                    "max_tokens": 150,
                }))
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        "other_llm" => {
            // Example for another LLM provider
            client.post(&config.url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .json(&serde_json::json!({
                    "prompt": format!("{}: {}", prompt, data),
                    "max_tokens": 150,
                }))
                .send()
                .await?
                .json::<Value>()
                .await?
        }
        _ => return Err(FetchError::Custom("Unsupported LLM type".into())),
    };
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::common::get_llm_config;

    #[tokio::test]
    #[ignore]  // This will make sure the test is ignored by default
    async fn test_analyze_data() {
        let data = "Sample data to analyze";
        let prompt = "Summarize";
        let config = get_llm_config();
        let result = analyze_data(data, prompt, &config).await;
        assert!(result.is_ok());
    }
}
