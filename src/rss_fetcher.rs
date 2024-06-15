use rss::Channel;
use reqwest;
use crate::error::FetchError;
use crate::data::NormalizedData;

pub async fn fetch_rss_feed(url: &str) -> Result<Vec<NormalizedData>, FetchError> {
    let content = reqwest::get(url).await?.text().await?;
    let channel = content.parse::<Channel>()?;
    Ok(normalize_rss_data(&channel))
}

pub fn normalize_rss_data(channel: &Channel) -> Vec<NormalizedData> {
    channel.items().iter().map(|item| {
        NormalizedData {
            title: item.title().unwrap_or_default().to_string(),
            link: item.link().unwrap_or_default().to_string(),
            description: item.description().unwrap_or_default().to_string(),
        }
    }).collect()
}
