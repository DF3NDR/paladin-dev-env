use reqwest::Error;
use scraper::{Html, Selector};

pub async fn scrape_web_page(url: &str) -> Result<Vec<String>, Error> {
    let content = reqwest::get(url).await?.text().await?;
    let document = Html::parse_document(&content);
    let selector = Selector::parse("h1").unwrap();
    let titles: Vec<String> = document
        .select(&selector)
        .map(|element| element.inner_html())
        .collect();
    Ok(titles)
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
