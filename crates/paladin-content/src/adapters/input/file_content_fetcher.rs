/*
File Content Fetcher Adapter

This adapter fetches content from an external file system location and creates a ContentItem object from it.
It implements the ContentIngestionPort trait and handles different content types based on file extensions.
*/
use paladin_core::platform::container::content::{
    AudioContent, ContentItem, ContentType, ImageContent, TextContent, VideoContent,
};
use paladin_ports::input::content_input_port::ContentIngestionPort;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Refusal text returned by `ingest_content` once validation has passed (FIXES-02, D-03).
const INGEST_CONTENT_UNFINISHED: &str = "FileContentFetcher::ingest_content is unfinished: content ingestion is not implemented, nothing was stored";

#[doc(hidden)]
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
            }
            // Video files
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" => {
                self.create_video_content(path)
            }
            // Audio files
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => {
                self.create_audio_content(path)
            }
            // Image files
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "svg" => {
                self.create_image_content(path)
            }
            _ => Err(format!("Unsupported file type: {}", extension)),
        }
    }

    fn create_text_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();

        // Read the file content
        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;

        let text_content =
            TextContent::new(Some(path_str), Some(content)).map_err(|e| e.to_string())?;

        Ok(ContentType::Text(text_content))
    }

    fn create_video_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();

        // Unfinished: `get_video_duration` always answers `None` (no media library is chosen),
        // and `VideoContent` has no "unknown" duration, so an unknown duration is recorded as 0.
        let duration = self.get_video_duration(path).unwrap_or(0);

        let video_content =
            VideoContent::new(Some(path_str), duration).map_err(|e| e.to_string())?;

        Ok(ContentType::Video(video_content))
    }

    fn create_audio_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();

        // Unfinished: `get_audio_duration` always answers `None` (no media library is chosen),
        // and `AudioContent` has no "unknown" duration, so an unknown duration is recorded as 0.
        let duration = self.get_audio_duration(path).unwrap_or(0);

        let audio_content =
            AudioContent::new(Some(path_str), duration).map_err(|e| e.to_string())?;

        Ok(ContentType::Audio(audio_content))
    }

    fn create_image_content(&self, path: &Path) -> Result<ContentType, String> {
        let path_str = path.to_string_lossy().to_string();

        // Unfinished: `get_image_dimensions` always answers `None` (no image library is chosen),
        // and `ImageContent` has no "unknown" resolution, so unknown dimensions are recorded as
        // (0, 0).
        let resolution = self.get_image_dimensions(path).unwrap_or((0, 0));

        let image_content =
            ImageContent::new(Some(path_str), resolution).map_err(|e| e.to_string())?;

        Ok(ContentType::Image(image_content))
    }

    /// Not implemented: always returns `None` (which media library would back it is not decided).
    fn get_video_duration(&self, _path: &Path) -> Option<u64> {
        None
    }

    /// Not implemented: always returns `None` (which media library would back it is not decided).
    fn get_audio_duration(&self, _path: &Path) -> Option<u64> {
        None
    }

    /// Not implemented: always returns `None` (which media library would back it is not decided).
    fn get_image_dimensions(&self, _path: &Path) -> Option<(u32, u32)> {
        None
    }
}

impl ContentIngestionPort for FileContentFetcher {
    fn fetch_content(&self, content: ContentItem) -> Result<ContentItem, String> {
        let url = content.url().cloned().ok_or("URL is None")?;
        let path = url
            .to_file_path()
            .map_err(|_| "Failed to convert URL to file path")?;

        // Determine content type based on file extension
        let content_type = self.determine_content_type(&path)?;

        // Get file name for title
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        // Create new ContentItem using the new API
        // The hash will be automatically generated by the constructor if the content has a path
        let mut content_item = ContentItem::new(content_type)
            .map_err(|e| format!("Failed to create content item: {:?}", e))?;

        // Set optional fields using setter methods
        content_item.set_url(Some(url));
        if let Some(title) = title {
            content_item.set_title(Some(title));
        }
        content_item.set_tags(Some(Vec::new()));

        Ok(content_item)
    }

    /// Validates the item, then refuses: content ingestion is **unfinished**.
    ///
    /// Each content type still requires a URL, so a URL-less item gets its specific
    /// `<Type> content requires a URL` error. A well-formed item then gets an `Err` naming this
    /// method, because nothing is stored, indexed or thumbnailed; reporting `Ok(())` would tell
    /// the caller its content was ingested when it was not. The decision is recorded in
    /// `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-BASE-LAYER-REVIEW.md`.
    ///
    /// # Errors
    ///
    /// Always returns `Err`: a validation message for an item without a URL, otherwise the
    /// unfinished-ingestion refusal.
    fn ingest_content(&self, content: ContentItem) -> Result<(), String> {
        // Validate that the content is properly formed before refusing.
        match content.content() {
            ContentType::Text(_) => {
                // Validate text content
                if content.url().is_none() {
                    return Err("Text content requires a URL".to_string());
                }
            }
            ContentType::Video(_) => {
                // Validate video content
                if content.url().is_none() {
                    return Err("Video content requires a URL".to_string());
                }
            }
            ContentType::Audio(_) => {
                // Validate audio content
                if content.url().is_none() {
                    return Err("Audio content requires a URL".to_string());
                }
            }
            ContentType::Image(_) => {
                // Validate image content
                if content.url().is_none() {
                    return Err("Image content requires a URL".to_string());
                }
            }
        }

        // Unfinished: refuse rather than report a success that stored nothing (FIXES-02, D-03).
        Err(INGEST_CONTENT_UNFINISHED.to_string())
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

        // Create input content using the new API
        let text_content = TextContent::new(None, Some(String::new()))?;
        let mut input_content = ContentItem::new(ContentType::Text(text_content))?;
        input_content.set_url(Some(file_url));

        let content = fetcher.fetch_content(input_content)?;

        // Verify it's text content
        match content.content() {
            ContentType::Text(text_content) => {
                assert!(text_content.content.is_some());
                assert_eq!(text_content.content.as_ref().unwrap(), "Hello, world!\n");
            }
            _ => panic!("Expected text content"),
        }

        assert_eq!(content.title(), Some(&"test.txt".to_string()));
        assert!(content.hash().is_some());

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

        // Create input content using the new API
        let text_content = TextContent::new(None, Some(String::new()))?;
        let mut input_content = ContentItem::new(ContentType::Text(text_content))?;
        input_content.set_url(Some(file_url));

        let content = fetcher.fetch_content(input_content)?;

        // Verify it's video content
        match content.content() {
            ContentType::Video(_) => {
                // Test passes if we get video content
            }
            _ => panic!("Expected video content"),
        }

        assert_eq!(content.title(), Some(&"test.mp4".to_string()));

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
    fn test_ingest_content_refuses_valid_content_as_unfinished()
    -> Result<(), Box<dyn std::error::Error>> {
        let fetcher = FileContentFetcher;

        // A valid item passes validation and is then refused: nothing is stored.
        let text_content = TextContent::new(None, Some("test".to_string()))?;
        let mut valid_content = ContentItem::new(ContentType::Text(text_content))?;
        valid_content.set_url(Some(Url::parse("file:///test.txt")?));

        let result = fetcher.ingest_content(valid_content);
        assert_eq!(
            result,
            Err("FileContentFetcher::ingest_content is unfinished: content ingestion is not implemented, nothing was stored".to_string())
        );

        // An item without a URL still gets its specific validation error.
        let text_content = TextContent::new(None, Some("test".to_string()))?;
        let invalid_content = ContentItem::new(ContentType::Text(text_content))?;

        let result = fetcher.ingest_content(invalid_content);
        assert_eq!(result, Err("Text content requires a URL".to_string()));

        Ok(())
    }
}
