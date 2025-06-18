/*
Content List Fetcher for File System
This module provides an implementation of the ContentListFetchingService that fetches content lists from a file system directory.
It reads files from the specified directory and creates ContentItem objects for each file.
*/
use std::fs;
use std::path::Path;
use crate::core::platform::container::content::{ContentItem, ContentType, TextContent, ImageContent, VideoContent, AudioContent};
use crate::core::platform::container::content_list::{ContentList, ContentListItem};
use crate::application::use_cases::content::content_list_fetching_service::ContentListFetchingService;
use url::Url;

pub struct FileContentListFetcher;

impl FileContentListFetcher {
    fn determine_content_type(path: &Path) -> Result<ContentType, String> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        let path_str = path.to_string_lossy().to_string();
        
        match extension.as_str() {
            "txt" | "md" | "rst" | "doc" | "docx" | "pdf" => {
                TextContent::new(Some(path_str), None)
                    .map(ContentType::Text)
                    .map_err(|e| format!("Failed to create text content: {}", e))
            },
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => {
                // Default resolution, would need image processing library to get actual dimensions
                ImageContent::new(Some(path_str), (800, 600))
                    .map(ContentType::Image)
                    .map_err(|e| format!("Failed to create image content: {}", e))
            },
            "mp4" | "avi" | "mov" | "wmv" | "flv" | "webm" => {
                // Default duration, would need video processing library to get actual duration
                VideoContent::new(Some(path_str), 0)
                    .map(ContentType::Video)
                    .map_err(|e| format!("Failed to create video content: {}", e))
            },
            "mp3" | "wav" | "flac" | "aac" | "ogg" => {
                // Default duration, would need audio processing library to get actual duration
                AudioContent::new(Some(path_str), 0)
                    .map(ContentType::Audio)
                    .map_err(|e| format!("Failed to create audio content: {}", e))
            },
            _ => {
                // Default to text for unknown types
                TextContent::new(Some(path_str), None)
                    .map(ContentType::Text)
                    .map_err(|e| format!("Failed to create default text content: {}", e))
            }
        }
    }
}

impl ContentListFetchingService for FileContentListFetcher {
    fn fetch_content_list(&self, directory: &str) -> Result<ContentList, String> {
        let mut list: Vec<ContentListItem> = Vec::new();
        
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        // Determine content type based on file extension
                        let content_type = Self::determine_content_type(&path)?;
                        
                        // Create ContentItem using the new method
                        match ContentItem::new(content_type) {
                            Ok(mut content_item) => {
                                // Set additional metadata
                                content_item.url = Url::parse(&format!("file://{}", path.to_string_lossy()))
                                    .ok();
                                content_item.source_url = content_item.url.clone();
                                content_item.title = path.file_name()
                                    .and_then(|name| name.to_str())
                                    .map(|s| s.to_string());
                                content_item.tags = Some(Vec::new());
                                
                                // Convert ContentItem to ContentListItem
                                let content_list_item = ContentListItem::Item(content_item);
                                list.push(content_list_item);
                            },
                            Err(e) => {
                                eprintln!("Failed to create content item for {}: {}", path.display(), e);
                                continue;
                            }
                        }
                    }
                }
            }
        } else {
            return Err(format!("Failed to read directory: {}", directory));
        }
        
        ContentList::new(
            directory.to_string(),
            Url::parse(&format!("file://{}", directory))
                .map_err(|e| format!("Invalid directory URL: {}", e))?,
            list,
        ).map_err(|e| format!("Failed to create content list: {}", e))
    }
}

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
        let result = fetcher.fetch_content_list(dir.path().to_str().unwrap());
        
        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.list_items.len(), 1);
        
        let item = &list.list_items[0];
        if let ContentListItem::Item(content_item) = item {
            assert!(content_item.title.is_some());
            assert_eq!(content_item.title.as_ref().unwrap(), "test.txt");
            assert!(matches!(content_item.content, ContentType::Text(_)));
        } else {
            panic!("Expected ContentListItem::Item");
        }
    }

    #[test]
    fn test_fetch_content_lists_multiple_files() {
        let dir = tempdir().unwrap();
        
        // Create different file types
        let txt_path = dir.path().join("test.txt");
        let mut txt_file = File::create(&txt_path).unwrap();
        writeln!(txt_file, "Text content").unwrap();
        
        let md_path = dir.path().join("readme.md");
        let mut md_file = File::create(&md_path).unwrap();
        writeln!(md_file, "# Markdown content").unwrap();

        let fetcher = FileContentListFetcher;
        let result = fetcher.fetch_content_list(dir.path().to_str().unwrap());
        
        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.list_items.len(), 2);
        
        // Both should be text content types
        for item in &list.list_items {
            if let ContentListItem::Item(content_item) = item {
                assert!(matches!(content_item.content, ContentType::Text(_)));
                assert!(content_item.title.is_some());
            } else {
                panic!("Expected ContentListItem::Item");
            }
        }
    }

    #[test]
    fn test_fetch_content_lists_empty_directory() {
        let dir = tempdir().unwrap();

        let fetcher = FileContentListFetcher;
        let result = fetcher.fetch_content_list(dir.path().to_str().unwrap());
        
        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.list_items.len(), 0);
    }

    #[test]
    fn test_fetch_content_lists_nonexistent_directory() {
        let fetcher = FileContentListFetcher;
        let result = fetcher.fetch_content_list("/nonexistent/directory");
        
        assert!(result.is_err());
    }
}