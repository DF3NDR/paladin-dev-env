/*
File Content Fetcher Adapter

This adapter fetches content from an external file system location and creates a ContentItem object from it.
It implements the ContentIngestionPort trait.
*/
use std::fs;
use std::io::Read;
use crate::ports::input::content_input_port::ContentIngestionPort;
use crate::core::platform::container::content::ContentItem;

pub struct FileContentFetcher;

impl ContentIngestionPort for FileContentFetcher {
    fn fetch_content(&self, content: ContentItem) -> Result<ContentItem, String> {
        let mut url = content.url.clone().to_path_buf();
        let mut file = fs::File::open(url).map_err(|e| e.to_string())?;
        let mut data = String::new();
        file.read_to_string(&mut data).map_err(|e| e.to_string())?;
        Ok(ContentItem {
            uuid: todo!(),
            created: todo!(),
            modified: todo!(),
            content: todo!(),
            url,
            hash: todo!(),
            source_id: todo!(),
            source_url: todo!(),
            title: todo!(),
            description: todo!(),
            tags: todo!(),
            source: todo!(),
            author: todo!(),
            pub_date: todo!(),
            mod_date: todo!(),
        })
    }
    
    fn ingest_content(&self, content: ContentItem) -> Result<(), String> {
        todo!()
    
    }
}

// src/adapters/primary/file_content_fetcher.rs (unit test)
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_fetch_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello, world!").unwrap();

        let fetcher = FileContentFetcher;
        let content = fetcher.fetch_content(file_path.to_str().unwrap()).unwrap();

        // assert_eq!(content.url, file_path.to_str().unwrap());
        // assert_eq!(content.data, "Hello, world!\n");
    }
}
