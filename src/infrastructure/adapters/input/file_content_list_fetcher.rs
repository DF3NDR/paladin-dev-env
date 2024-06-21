// src/adapters/primary/file_content_list_fetcher.rs
use std::fs;
use crate::domain::entities::content_item::ContentItem;
use crate::domain::entities::content_list::ContentList;
use crate::domain::services::content_fetching_service::ContentListFetchingService;
use url::Url;

pub struct FileContentListFetcher;

impl ContentListFetchingService for FileContentListFetcher {
    fn fetch_content_list(&self, directory: &str) -> Result<ContentList, String> {
        let mut list = Vec::new();
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        let metadata = ContentItemMetadata::new(
                            "".to_string(), 
                            ContentType::new(ContentTypeItem::TextDocument), 
                            url::Url::parse(&format!("file://{}", path.to_str().unwrap())).unwrap(),
                            path.file_name().unwrap().to_str().unwrap().to_string(), 
                            None,
                        );
                        let content_item = ContentItem {
                            uuid: todo!(),
                            created: todo!(),
                            modified: todo!(),
                            source_data: todo!(),
                            content: todo!(),
                        };
                        list.push(content_item);
                    }
                }
            }
        }
        Ok(ContentList::new(
            directory.to_string(),
            Url::parse(&format!("file://{}", directory)).unwrap(),            
            list,
        ))
    }
}

// src/adapters/primary/file_content_list_fetcher.rs (unit test)
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_fetch_content_lists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello, world!").unwrap();

        let fetcher = FileContentListFetcher;
        let list = fetcher.fetch_content_list(dir.path().to_str().unwrap());
        
        // assert that the list contains the file
        assert_eq!(list.unwrap().list_items.len(), 1);  
        // This test probably doesn't work
        // assert_eq!(list.unwrap().list_items, dir.path().to_str().unwrap());
    }
}
