use serde_json::Value;
use crate::error::FetchError;
use crate::llm_config::LlmConfig;

pub async fn analyze_data(data: &str, prompt: &str, config: &LlmConfig) -> Result<Value, FetchError> {
    let client = reqwest::Client::new();
    let response = client.post(&config.url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&serde_json::json!({
            "prompt": format!("{}: {}", prompt, data),
            "max_tokens": 150,
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::common::get_llm_config;

    #[tokio::test]
    #[ignore] // Ignoring the test as it requires an external API that costs money
    async fn test_analyze_data() {
        // This could be a news article, blog post, etc. 
        // It can be improved by adding something to actually analyze.
        let data = "Sample data to analyze";
        let prompt = "Summarize"; 
        let config = get_llm_config();
        let result = analyze_data(data, prompt, &config).await;
        assert!(result.is_ok());
    }
}
