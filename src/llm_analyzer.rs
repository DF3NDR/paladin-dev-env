use serde_json::Value;
use crate::error::FetchError;

pub async fn analyze_data(data: &str, prompt: &str) -> Result<Value, FetchError> {
    let client = reqwest::Client::new();
    let response = client.post("https://api.openai.com/v1/engines/davinci-codex/completions")
        .header("Authorization", "Bearer YOUR_API_KEY")
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

    #[tokio::test]
    async fn test_analyze_data() {
        let data = "Sample data to analyze";
        let prompt = "Summarize";
        let result = analyze_data(data, prompt).await;
        assert!(result.is_ok());
    }
}
