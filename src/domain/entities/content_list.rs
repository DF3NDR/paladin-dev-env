// src/domain/entities/raw_content_list.rs
use serde::{Serialize, Deserialize};
use crate::domain::entities::content_item::ContentItem;
use uuid::Uuid;
use url::Url;
use chrono::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentList {
    pub uuid: Uuid,
    pub name: String,
    pub url: Url,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub list_items: Vec<ContentItem>,
}

impl ContentList {
    pub fn new(
        name: String,
        url: Url,
        list_items: Vec<ContentItem>,
    ) -> Self {
        ContentList {
            uuid: Uuid::new_v4(),
            name,
            url,
            created: Utc::now(),
            modified: Utc::now(),
            list_items,
        }
    }
}

pub struct ContentListMetadata {
    pub uuid: Uuid,
    pub name: String,
    pub url: Url,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

pub struct ContentListConfig {
    pub name: String,
    pub url: Url,
    // Todo The ContentList needs a way to have a unique identifier to eliminate duplicates
    // pub index: 
}
