use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("RSS parse error: {0}")]
    Rss(#[from] rss::Error),

    #[error("Custom error: {0}")]
    Custom(String),
}