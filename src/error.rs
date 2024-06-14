use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Parsing failed: {0}")]
    ParsingError(#[from] serde_json::Error),

    #[error("RSS parse error: {0}")]
    Rss(#[from] rss::Error),

    #[error("Custom error: {0}")]
    Custom(String),
}
