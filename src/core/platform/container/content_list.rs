use super::content::ContentItem;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use url::Url;
use chrono::prelude::*;
use thiserror::Error;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentList {
    pub uuid: Uuid,
    pub name: String,
    pub url: Url,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub list_items: Vec<ContentListItem>,
}

impl ContentList {
    pub fn new(
        name: String,
        url: Url,
        list_items: Vec<ContentListItem>,
    ) -> Result<Self, CreateContentListError> {
        let mut seen = HashSet::new();
        for item in &list_items {
            let item_url = item.url().map_err(CreateContentListError::from)?;
            if !seen.insert(item_url.clone()) {
                return Err(CreateContentListError::Duplicate {
                    url: item_url,
                });
            }
        }
        Ok(ContentList {
            uuid: Uuid::new_v4(),
            name,
            url,
            created: Utc::now(),
            modified: Utc::now(),
            list_items,
        })
    }

    pub fn add_item(&mut self, item: ContentListItem) -> Result<(), AddContentListItemError> {
        let item_url = item.url().map_err(AddContentListItemError::from)?;
        if self.list_items.iter().any(|i| i.url().unwrap() == item_url) {
            return Err(AddContentListItemError::Duplicate { url: item_url });
        }
        self.list_items.push(item);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContentListItem {
    Fetch(ContentItemToFetch),
    Item(ContentItem),
}

impl ContentListItem {
    pub fn url(&self) -> Result<Url, GetContentListItemUrlError> {
        match self {
            ContentListItem::Fetch(item_to_fetch) => Ok(item_to_fetch.url.0.clone()),
            ContentListItem::Item(content_item) => content_item.url.clone().ok_or(GetContentListItemUrlError::NoUrl),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentItemToFetch {
    pub uuid: Uuid,
    pub url: ContentItemToFetchUrl,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub content_item: ContentItem,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Error, Serialize, Deserialize)]
#[error("Content item to fetch URL cannot be empty")]
pub struct ContentItemToFetchUrl(Url);

impl ContentItemToFetchUrl {
    pub fn new(url: Url) -> Result<Self, ContentItemToFetchEmptyUrlError> {
        if url.as_str().is_empty() {
            Err(ContentItemToFetchEmptyUrlError)
        } else {
            Ok(ContentItemToFetchUrl(url))
        } 
    }
}

#[derive(Debug, Clone, Error)]
#[error("URL cannot be empty")]
pub struct ContentItemToFetchEmptyUrlError;

impl ContentItemToFetch {
    pub fn new(url: Url, content_item: ContentItem) -> Self {
        ContentItemToFetch {
            uuid: Uuid::new_v4(),
            url: ContentItemToFetchUrl(url),
            created: Utc::now(),
            modified: Utc::now(),
            content_item,
        }
    }
}

pub struct CreateContentItemToFetchRequest {
    pub url: Url,
    pub content_item: ContentItem,
}

impl CreateContentItemToFetchRequest {
    pub fn new(url: Url, content_item: ContentItem) -> Result<ContentItemToFetch, CreateContentListItemToFetchError> {
        let url = ContentItemToFetchUrl::new(url).map_err(|_| CreateContentListItemToFetchError::EmptyUrl)?;
        Ok(ContentItemToFetch::new(url.0, content_item))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CreateContentListItemToFetchError {
    #[error("Content item to fetch has duplicate URL")]
    Duplicate {
        url: ContentItemToFetchUrl,
    },
    #[error("URL cannot be empty")]
    EmptyUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CreateContentListError {
    #[error("Content item to fetch has duplicate URL")]
    Duplicate {
        url: Url,
    },
    #[error("Content item has no URL")]
    NoUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AddContentListItemError {
    #[error("Content item to fetch has duplicate URL")]
    Duplicate {
        url: Url,
    },
    #[error("Content item has no URL")]
    NoUrl,
}

#[derive(Debug, Clone, Error)]
pub enum GetContentListItemUrlError {
    #[error("Content item has no URL")]
    NoUrl,
}

// Implement From for error conversions
impl From<GetContentListItemUrlError> for CreateContentListError {
    fn from(err: GetContentListItemUrlError) -> CreateContentListError {
        match err {
            GetContentListItemUrlError::NoUrl => CreateContentListError::NoUrl,
        }
    }
}

impl From<GetContentListItemUrlError> for AddContentListItemError {
    fn from(err: GetContentListItemUrlError) -> AddContentListItemError {
        match err {
            GetContentListItemUrlError::NoUrl => AddContentListItemError::NoUrl,
        }
    }
}
