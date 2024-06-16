// src/fetcher/scraper.rs
use scraper::{Html, Selector};
use crate::error::FetchError;
use crate::data::NormalizedData;

pub async fn scrape_web_page(url: &str) -> Result<Vec<NormalizedData>, FetchError> {
    let content = reqwest::get(url).await?.text().await?;
    let document = Html::parse_document(&content);
    let selector = Selector::parse("css-selector-for-target-elements").unwrap();

    let mut normalized_data = Vec::new();
    for element in document.select(&selector) {
        let title = element.text().collect::<Vec<_>>().concat();
        normalized_data.push(NormalizedData {
            title,
            link: url.to_string(), // Use the URL as a link
            description: String::new(), // Add description if available
        });
    }

    Ok(normalized_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scrape_web_page() {
        let url = "https://example.com";
        let result = scrape_web_page(url).await;
        assert!(result.is_ok());
    }
}
