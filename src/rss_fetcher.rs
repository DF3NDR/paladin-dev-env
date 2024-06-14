use rss::Channel;
use crate::error::FetchError;

pub async fn fetch_rss_feed(url: &str) -> Result<Channel, FetchError> {
    let content = reqwest::get(url).await?.text().await?;
    let channel = content.parse::<Channel>()?;
    Ok(channel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_rss_feed() {
        let url = "https://caitlin-long.com/feed/";
        let result = fetch_rss_feed(url).await;
        assert!(result.is_ok());
    }
}
