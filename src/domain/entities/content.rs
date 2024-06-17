use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NormalizedData {
    pub title: String,
    pub link: String,
    pub description: String,
}