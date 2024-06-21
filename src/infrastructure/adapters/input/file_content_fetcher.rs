// src/adapters/primary/file_content_fetcher.rs
use std::fs;
use std::io::Read;
use crate::domain::entities::content_item::ContentItem;
use crate::domain::services::content_fetching_service::ContentFetchingService;

pub struct FileContentFetcher;

impl ContentFetchingService for FileContentFetcher {
    fn fetch_content(&self, location: &str) -> Result<ContentItem, String> {
        let mut file = fs::File::open(location).map_err(|e| e.to_string())?;
        let mut data = String::new();
        file.read_to_string(&mut data).map_err(|e| e.to_string())?;
        Ok(ContentItem {
            uuid: todo!(),
            created: todo!(),
            modified: todo!(),
            source_data: todo!(),
            content: todo!(),
        })
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
