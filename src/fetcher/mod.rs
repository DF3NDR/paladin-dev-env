// src/fetcher/mod.rs
pub mod api_fetcher;
pub mod rss_fetcher;
pub mod scraper;

use crate::data::NormalizedData;
use crate::error::FetchError;

pub async fn fetch_data(source_type: &str, url: &str) -> Result<Vec<NormalizedData>, FetchError> {
    match source_type {
        "rss" => rss_fetcher::fetch_rss_feed(url).await,
        "api" => api_fetcher::fetch_api_data(url).await,
        "web" => scraper::scrape_web_page(url).await,
        _ => Err(FetchError::Custom("Unknown source type".into())),
    }
}
