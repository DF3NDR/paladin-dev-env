// src/error.rs
use thiserror::Error;
use actix_web::{HttpResponse, ResponseError};

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

impl ResponseError for FetchError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            FetchError::RequestError(ref e) => {
                HttpResponse::BadRequest().body(format!("Request error: {}", e))
            }
            FetchError::ParsingError(ref e) => {
                HttpResponse::InternalServerError().body(format!("Parsing error: {}", e))
            }
            FetchError::Rss(ref e) => {
                HttpResponse::InternalServerError().body(format!("RSS parse error: {}", e))
            }
            FetchError::Custom(ref message) => {
                HttpResponse::BadRequest().body(message.clone())
            }
        }
    }
}
