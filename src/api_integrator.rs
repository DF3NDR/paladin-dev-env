use reqwest::Error;
use serde_json::Value;

pub async fn fetch_api_data(url: &str) -> Result<Value, Error> {
    let response = reqwest::get(url).await?.json::<Value>().await?;
    Ok(response)
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
