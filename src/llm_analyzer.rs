use serde_json::Value;
use crate::error::FetchError;
use crate::configuration::Settings;
use reqwest::Client;

pub async fn analyze_data(data: &str, prompt: &str, config: &Settings) -> Result<Value, FetchError> {
    let client = Client::new();
    let response = client.post(&config.llm_url)
        .header("Authorization", format!("Bearer {}", config.llm_api_key))
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
    use crate::test_utils::common::load_test_config;

    #[tokio::test]
    #[ignore]  // This will make sure the test is ignored by default
    async fn test_analyze_data() {
        let data = "Sample data to analyze";
        let prompt = "Summarize";
        let config = load_test_config();
        let result = analyze_data(data, prompt, &config).await;
        assert!(result.is_ok());
    }
}
