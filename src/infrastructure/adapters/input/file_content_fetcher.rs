/*
File Content Fetcher Adapter

This adapter fetches content from an external file system location and creates a ContentItem object from it.
It implements the ContentIngestionPort trait and handles different content types based on file extensions.
*/
use std::fs;
use std::io::Read;
use std::path::Path;
use crate::ports::input::content_input_port::ContentIngestionPort;
use crate::core::platform::container::content::{
    ContentItem, ContentType, TextContent, VideoContent, AudioContent, ImageContent};

pub struct FileContentFetcher;

impl FileContentFetcher {
    /// Determine content type based on file extension
    fn determine_content_type(&self, path: &Path) -> Result<ContentType, String> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .ok_or("No file extension found")?;

        match extension.as_str() {
            // Text files
            "txt" | "md" | "rst" | "json" | "xml" | "yaml" | "yml" | "toml" | "csv" | "log" => {
                self.create_text_content(path)
            },
            // Video files
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" => {
                self.create_video_content(path)
            },
            // Audio files
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => {
                self.create_audio_content(path)
            },
            // Image files
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "svg" => {
                self.create_image_content(path)
            },
            _ => Err(format!("Unsupported file type: {}", extension))
        }
    }

    fn create_text_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();
        
        // Read the file content
        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;

        let text_content = TextContent::new(Some(path_str), Some(content))
            .map_err(|e| e.to_string())?;
        
        Ok(ContentType::Text(text_content))
    }

    fn create_video_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();
        
        // For now, we'll use a placeholder duration
        // In a real implementation, you'd use a library like ffmpeg-rust to get actual duration
        let duration = self.get_video_duration(path).unwrap_or(0);
        
        let video_content = VideoContent::new(Some(path_str), duration)
            .map_err(|e| e.to_string())?;
        
        Ok(ContentType::Video(video_content))
    }

    fn create_audio_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();
        
        // For now, we'll use a placeholder duration
        // In a real implementation, you'd use a library like symphonia or rodio to get actual duration
        let duration = self.get_audio_duration(path).unwrap_or(0);
        
        let audio_content = AudioContent::new(Some(path_str), duration)
            .map_err(|e| e.to_string())?;
        
        Ok(ContentType::Audio(audio_content))
    }

    fn create_image_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();
        
        // For now, we'll use placeholder dimensions
        // In a real implementation, you'd use a library like image-rs to get actual dimensions
        let resolution = self.get_image_dimensions(path).unwrap_or((0, 0));
        
        let image_content = ImageContent::new(Some(path_str), resolution)
            .map_err(|e| e.to_string())?;
        
        Ok(ContentType::Image(image_content))
    }

    // Placeholder methods for media metadata extraction
    // These would be implemented using appropriate media libraries
    fn get_video_duration(&self, _path: &Path) -> Option<u64> {
        // TODO: Implement using ffmpeg-rust or similar
        None
    }

    fn get_audio_duration(&self, _path: &Path) -> Option<u64> {
        // TODO: Implement using symphonia, rodio, or similar
        None
    }

    fn get_image_dimensions(&self, _path: &Path) -> Option<(u32, u32)> {
        // TODO: Implement using image crate
        // You'll need to add `image = "0.24"` to Cargo.toml
        None
    }

    fn generate_content_hash(&self, content_type: &ContentType) -> Result<String, String> {
        match content_type.path() {
            Some(path) => {
                let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
                
                // Use a simple hash for now - in production you might want SHA-256 or Blake3
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                buffer.hash(&mut hasher);
                Ok(format!("{:x}", hasher.finish()))
            },
            None => Err("No file path available for hashing".to_string())
        }
    }
}

impl ContentIngestionPort for FileContentFetcher {
    fn fetch_content(&self, content: ContentItem) -> Result<ContentItem, String> {
        let url = content.url.clone().ok_or("URL is None")?;
        let path = url.to_file_path().map_err(|_| "Failed to convert URL to file path")?;
        
        // Determine content type based on file extension
        let content_type = self.determine_content_type(&path)?;
        
        // Generate hash for the content
        let hash = self.generate_content_hash(&content_type).ok();
        
        // Get file name for title
        let title = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        Ok(ContentItem {
            uuid: uuid::Uuid::new_v4(),
            created: chrono::Utc::now(),
            modified: chrono::Utc::now(),
            content: content_type,
            url: Some(url),
            hash,
            source_id: None,
            source_url: None,
            title,
            description: None,
            tags: Some(Vec::new()),
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        })
    }
    
    fn ingest_content(&self, content: ContentItem) -> Result<(), String> {
        // This would implement the logic to store the content in your system
        // For now, we'll just validate that the content is properly formed
        match &content.content {
            ContentType::Text(_) => {
                // Validate text content
                if content.url.is_none() {
                    return Err("Text content requires a URL".to_string());
                }
            },
            ContentType::Video(_) => {
                // Validate video content
                if content.url.is_none() {
                    return Err("Video content requires a URL".to_string());
                }
            },
            ContentType::Audio(_) => {
                // Validate audio content
                if content.url.is_none() {
                    return Err("Audio content requires a URL".to_string());
                }
            },
            ContentType::Image(_) => {
                // Validate image content
                if content.url.is_none() {
                    return Err("Image content requires a URL".to_string());
                }
            },
        }
        
        // TODO: Implement actual ingestion logic
        // This might involve:
        // - Storing content in a repository
        // - Creating search indices
        // - Generating thumbnails for images/videos
        // - Extracting metadata
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use url::Url;

    #[test]
    fn test_fetch_text_content() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "Hello, world!")?;

        let fetcher = FileContentFetcher;
        let file_url = Url::from_file_path(&file_path).unwrap();
        let input_content = ContentItem {
            uuid: uuid::Uuid::new_v4(),
            created: chrono::Utc::now(),
            modified: chrono::Utc::now(),
            content: ContentType::Text(TextContent::new(None, Some(String::new()))?),
            url: Some(file_url),
            hash: None,
            source_id: None,
            source_url: None,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        let content = fetcher.fetch_content(input_content)?;

        // Verify it's text content
        match &content.content {
            ContentType::Text(text_content) => {
                assert!(text_content.content.is_some());
                assert_eq!(text_content.content.as_ref().unwrap(), "Hello, world!\n");
            },
            _ => panic!("Expected text content"),
        }

        assert_eq!(content.title, Some("test.txt".to_string()));
        assert!(content.hash.is_some());

        Ok(())
    }

    #[test]
    fn test_fetch_video_content() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.mp4");
        let mut file = File::create(&file_path)?;
        writeln!(file, "fake video data")?; // Just create a dummy file

        let fetcher = FileContentFetcher;
        let file_url = Url::from_file_path(&file_path).unwrap();
        let input_content = ContentItem {
            uuid: uuid::Uuid::new_v4(),
            created: chrono::Utc::now(),
            modified: chrono::Utc::now(),
            content: ContentType::Text(TextContent::new(None, Some(String::new()))?),
            url: Some(file_url),
            hash: None,
            source_id: None,
            source_url: None,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        let content = fetcher.fetch_content(input_content)?;

        // Verify it's video content
        match &content.content {
            ContentType::Video(_) => {
                // Test passes if we get video content
            },
            _ => panic!("Expected video content"),
        }

        assert_eq!(content.title, Some("test.mp4".to_string()));

        Ok(())
    }

    #[test]
    fn test_determine_content_type() -> Result<(), Box<dyn std::error::Error>> {
        let fetcher = FileContentFetcher;
        
        // Create temporary files for different types
        let dir = tempdir()?;
        
        let txt_path = dir.path().join("test.txt");
        File::create(&txt_path)?;
        let content_type = fetcher.determine_content_type(&txt_path)?;
        assert!(matches!(content_type, ContentType::Text(_)));

        let mp4_path = dir.path().join("test.mp4");
        File::create(&mp4_path)?;
        let content_type = fetcher.determine_content_type(&mp4_path)?;
        assert!(matches!(content_type, ContentType::Video(_)));

        let mp3_path = dir.path().join("test.mp3");
        File::create(&mp3_path)?;
        let content_type = fetcher.determine_content_type(&mp3_path)?;
        assert!(matches!(content_type, ContentType::Audio(_)));

        let jpg_path = dir.path().join("test.jpg");
        File::create(&jpg_path)?;
        let content_type = fetcher.determine_content_type(&jpg_path)?;
        assert!(matches!(content_type, ContentType::Image(_)));

        Ok(())
    }

    #[test]
    fn test_unsupported_file_type() -> Result<(), Box<dyn std::error::Error>> {
        let fetcher = FileContentFetcher;
        let dir = tempdir()?;
        let unsupported_path = dir.path().join("test.xyz");
        File::create(&unsupported_path)?;
        
        let result = fetcher.determine_content_type(&unsupported_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported file type"));

        Ok(())
    }

    #[test]
    fn test_ingest_content_validation() -> Result<(), Box<dyn std::error::Error>> {
        let fetcher = FileContentFetcher;
        
        // Test with valid content
        let valid_content = ContentItem {
            uuid: uuid::Uuid::new_v4(),
            created: chrono::Utc::now(),
            modified: chrono::Utc::now(),
            content: ContentType::Text(TextContent::new(None, Some("test".to_string()))?),
            url: Some(Url::parse("file:///test.txt")?),
            hash: None,
            source_id: None,
            source_url: None,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        let result = fetcher.ingest_content(valid_content);
        assert!(result.is_ok());

        // Test with invalid content (no URL)
        let invalid_content = ContentItem {
            uuid: uuid::Uuid::new_v4(),
            created: chrono::Utc::now(),
            modified: chrono::Utc::now(),
            content: ContentType::Text(TextContent::new(None, Some("test".to_string()))?),
            url: None,
            hash: None,
            source_id: None,
            source_url: None,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        let result = fetcher.ingest_content(invalid_content);
        assert!(result.is_err());

        Ok(())
    }
}