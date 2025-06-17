/*

Content Port

A port that defines how the application fetches content from external sources or 
ingests content from the user. This could be an HTTP API, a database, a file system,
or any other source of content.

*/
use crate::core::platform::container::content::ContentItem;
use url::Url;

pub trait ContentIngestionPort {
    fn fetch_content(&self, content: ContentItem) -> Result<ContentItem, String>;
    fn ingest_content(&self, content: ContentItem) -> Result<(), String>;
}