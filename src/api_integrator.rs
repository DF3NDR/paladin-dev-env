use crate::error::FetchError;
use crate::data::NormalizedData;
use serde_json::Value;

pub async fn fetch_api_data(url: &str) -> Result<Vec<NormalizedData>, FetchError> {
    let response: Value = reqwest::get(url).await?.json().await?;
    let normalized_data = normalize_api_data(&response);
    Ok(normalized_data)
}

pub fn normalize_api_data(data: &Value) -> Vec<NormalizedData> {
    // Implement normalization logic based on the API data structure
    // Example for a simple case:
    vec![NormalizedData {
        title: data["title"].as_str().unwrap_or_default().to_string(),
        link: data["url"].as_str().unwrap_or_default().to_string(),
        description: data["description"].as_str().unwrap_or_default().to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_api_data() {
        let url = "https://newsapi.org/v2/everything?q=bitcoin&apiKey=";
        let result = fetch_api_data(url).await;
        assert!(result.is_ok());
    }
}
