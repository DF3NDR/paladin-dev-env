use crate::core::platform::container::content::{ContentItem, ContentType, TextContent, VideoContent, AudioContent, ImageContent};

pub struct ContentSummarizer;

#[derive(Debug, Clone)]
pub struct ContentSummary {
    pub content_type: String,
    pub summary: String,
    pub metadata: ContentMetadata,
}

#[derive(Debug, Clone)]
pub struct ContentMetadata {
    pub file_size: u64,
    pub duration: Option<u64>,
    pub resolution: Option<(u32, u32)>,
    pub character_count: Option<usize>,
    pub word_count: Option<usize>,
}

impl ContentSummarizer {
    pub fn new() -> Self {
        Self
    }

    /// Generate a summary of the content based on its type
    pub fn summarize_content(&self, content: &ContentItem, max_length: usize) -> ContentSummary {
        match &content.content {
            ContentType::Text(text_content) => self.summarize_text_content(text_content, max_length),
            ContentType::Video(video_content) => self.summarize_video_content(video_content),
            ContentType::Audio(audio_content) => self.summarize_audio_content(audio_content),
            ContentType::Image(image_content) => self.summarize_image_content(image_content),
        }
    }

    /// Summarize text content by truncating to specified length
    fn summarize_text_content(&self, text_content: &TextContent, max_length: usize) -> ContentSummary {
        let (summary, char_count, word_count) = match &text_content.content {
            Some(content) => {
                let char_count = content.chars().count();
                let word_count = content.split_whitespace().count();
                
                let summary = if char_count <= max_length {
                    content.clone()
                } else {
                    // Truncate at word boundary when possible
                    let truncated: String = content.chars().take(max_length).collect();
                    if let Some(last_space) = truncated.rfind(' ') {
                        format!("{}...", &truncated[..last_space])
                    } else {
                        format!("{}...", truncated)
                    }
                };
                
                (summary, Some(char_count), Some(word_count))
            },
            None => {
                match &text_content.path {
                    Some(path) => (format!("Text file: {}", path), None, None),
                    None => ("Empty text content".to_string(), Some(0), Some(0)),
                }
            }
        };

        ContentSummary {
            content_type: "Text".to_string(),
            summary,
            metadata: ContentMetadata {
                file_size: text_content.filesize,
                duration: None,
                resolution: None,
                character_count: char_count,
                word_count,
            },
        }
    }

    /// Summarize video content with metadata
    fn summarize_video_content(&self, video_content: &VideoContent) -> ContentSummary {
        let summary = match &video_content.path {
            Some(path) => {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown video");
                
                format!(
                    "Video: {} (Duration: {}s, Size: {})",
                    filename,
                    video_content.duration,
                    Self::format_file_size(video_content.filesize)
                )
            },
            None => format!(
                "Video content (Duration: {}s, Size: {})",
                video_content.duration,
                Self::format_file_size(video_content.filesize)
            ),
        };

        ContentSummary {
            content_type: "Video".to_string(),
            summary,
            metadata: ContentMetadata {
                file_size: video_content.filesize,
                duration: Some(video_content.duration),
                resolution: None,
                character_count: None,
                word_count: None,
            },
        }
    }

    /// Summarize audio content with metadata
    fn summarize_audio_content(&self, audio_content: &AudioContent) -> ContentSummary {
        let summary = match &audio_content.path {
            Some(path) => {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown audio");
                
                format!(
                    "Audio: {} (Duration: {}s, Size: {})",
                    filename,
                    audio_content.duration,
                    Self::format_file_size(audio_content.filesize)
                )
            },
            None => format!(
                "Audio content (Duration: {}s, Size: {})",
                audio_content.duration,
                Self::format_file_size(audio_content.filesize)
            ),
        };

        ContentSummary {
            content_type: "Audio".to_string(),
            summary,
            metadata: ContentMetadata {
                file_size: audio_content.filesize,
                duration: Some(audio_content.duration),
                resolution: None,
                character_count: None,
                word_count: None,
            },
        }
    }

    /// Summarize image content with metadata
    fn summarize_image_content(&self, image_content: &ImageContent) -> ContentSummary {
        let (width, height) = image_content.resolution;
        
        let summary = match &image_content.path {
            Some(path) => {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown image");
                
                format!(
                    "Image: {} ({}×{}, Size: {})",
                    filename,
                    width,
                    height,
                    Self::format_file_size(image_content.filesize)
                )
            },
            None => format!(
                "Image content ({}×{}, Size: {})",
                width,
                height,
                Self::format_file_size(image_content.filesize)
            ),
        };

        ContentSummary {
            content_type: "Image".to_string(),
            summary,
            metadata: ContentMetadata {
                file_size: image_content.filesize,
                duration: None,
                resolution: Some(image_content.resolution),
                character_count: None,
                word_count: None,
            },
        }
    }

    /// Generate a brief summary for display purposes
    pub fn brief_summary(&self, content: &ContentItem) -> String {
        let summary = self.summarize_content(content, 100);
        format!("{}: {}", summary.content_type, summary.summary)
    }

    /// Generate a detailed summary with all metadata
    pub fn detailed_summary(&self, content: &ContentItem) -> String {
        let summary = self.summarize_content(content, 500);
        let mut details = vec![
            format!("Type: {}", summary.content_type),
            format!("Summary: {}", summary.summary),
            format!("File Size: {}", Self::format_file_size(summary.metadata.file_size)),
        ];

        if let Some(duration) = summary.metadata.duration {
            details.push(format!("Duration: {}s", duration));
        }

        if let Some((width, height)) = summary.metadata.resolution {
            details.push(format!("Resolution: {}×{}", width, height));
        }

        if let Some(char_count) = summary.metadata.character_count {
            details.push(format!("Characters: {}", char_count));
        }

        if let Some(word_count) = summary.metadata.word_count {
            details.push(format!("Words: {}", word_count));
        }

        details.join("\n")
    }

    /// Extract key information for search/indexing
    pub fn extract_keywords(&self, content: &ContentItem) -> Vec<String> {
        let mut keywords = Vec::new();

        // Add content type as keyword
        match &content.content {
            ContentType::Text(_) => keywords.push("text".to_string()),
            ContentType::Video(_) => keywords.push("video".to_string()),
            ContentType::Audio(_) => keywords.push("audio".to_string()),
            ContentType::Image(_) => keywords.push("image".to_string()),
        }

        // Add file extension if available
        if let Some(path) = content.content.path() {
            if let Some(extension) = std::path::Path::new(path)
                .extension()
                .and_then(|ext| ext.to_str()) {
                keywords.push(extension.to_lowercase());
            }
        }

        // Add tags if available
        if let Some(ref tags) = content.tags {
            keywords.extend(tags.clone());
        }

        // For text content, extract some key words
        if let ContentType::Text(text_content) = &content.content {
            if let Some(ref text) = text_content.content {
                let words: Vec<String> = text
                    .split_whitespace()
                    .filter(|word| word.len() > 3) // Filter out short words
                    .take(10) // Take first 10 significant words
                    .map(|word| word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                    .filter(|word| !word.is_empty())
                    .collect();
                keywords.extend(words);
            }
        }

        keywords.dedup();
        keywords
    }

    /// Format file size in human-readable format
    fn format_file_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

impl Default for ContentSummarizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_text_content() -> Result<ContentItem, Box<dyn std::error::Error>> {
        let text_content = TextContent::new(
            None, 
            Some("This is a test content for summarization. It contains multiple sentences and should be properly summarized.".to_string())
        )?;
        
        Ok(ContentItem::new(ContentType::Text(text_content))?)
    }

    fn create_test_video_content() -> Result<ContentItem, Box<dyn std::error::Error>> {
        // Use None for path to avoid file existence check in tests
        let video_content = VideoContent::new(None, 3600)?;
        Ok(ContentItem::new(ContentType::Video(video_content))?)
    }

    fn create_test_video_content_with_path() -> Result<ContentItem, Box<dyn std::error::Error>> {
        // Create a temporary file for testing file path functionality
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_string_lossy().to_string();
        
        let video_content = VideoContent::new(Some(path), 3600)?;
        Ok(ContentItem::new(ContentType::Video(video_content))?)
    }

    #[test]
    fn test_summarize_text_content() -> Result<(), Box<dyn std::error::Error>> {
        let content = create_test_text_content()?;
        let summarizer = ContentSummarizer::new();
        
        let summary = summarizer.summarize_content(&content, 50);
        
        assert_eq!(summary.content_type, "Text");
        assert!(summary.summary.len() <= 53); // 50 + "..." 
        assert!(summary.metadata.character_count.is_some());
        assert!(summary.metadata.word_count.is_some());
        
        Ok(())
    }

    #[test]
    fn test_summarize_video_content() -> Result<(), Box<dyn std::error::Error>> {
        let content = create_test_video_content()?;
        let summarizer = ContentSummarizer::new();
        
        let summary = summarizer.summarize_content(&content, 100);
        
        assert_eq!(summary.content_type, "Video");
        assert!(summary.summary.contains("Duration: 3600s"));
        assert_eq!(summary.metadata.duration, Some(3600));
        
        Ok(())
    }

    #[test]
    fn test_summarize_video_content_with_path() -> Result<(), Box<dyn std::error::Error>> {
        let content = create_test_video_content_with_path()?;
        let summarizer = ContentSummarizer::new();
        
        let summary = summarizer.summarize_content(&content, 100);
        
        assert_eq!(summary.content_type, "Video");
        assert!(summary.summary.contains("Video:"));
        assert!(summary.summary.contains("Duration: 3600s"));
        assert_eq!(summary.metadata.duration, Some(3600));
        
        Ok(())
    }

    #[test]
    fn test_brief_summary() -> Result<(), Box<dyn std::error::Error>> {
        let content = create_test_text_content()?;
        let summarizer = ContentSummarizer::new();
        
        let brief = summarizer.brief_summary(&content);
        
        assert!(brief.starts_with("Text:"));
        assert!(brief.len() <= 200); // Should be reasonably brief
        
        Ok(())
    }

    #[test]
    fn test_detailed_summary() -> Result<(), Box<dyn std::error::Error>> {
        let content = create_test_text_content()?;
        let summarizer = ContentSummarizer::new();
        
        let detailed = summarizer.detailed_summary(&content);
        
        assert!(detailed.contains("Type: Text"));
        assert!(detailed.contains("File Size:"));
        assert!(detailed.contains("Characters:"));
        assert!(detailed.contains("Words:"));
        
        Ok(())
    }

    #[test]
    fn test_extract_keywords() -> Result<(), Box<dyn std::error::Error>> {
        let content = create_test_text_content()?;
        let summarizer = ContentSummarizer::new();
        
        let keywords = summarizer.extract_keywords(&content);
        
        assert!(keywords.contains(&"text".to_string()));
        assert!(!keywords.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(ContentSummarizer::format_file_size(512), "512 B");
        assert_eq!(ContentSummarizer::format_file_size(1024), "1.0 KB");
        assert_eq!(ContentSummarizer::format_file_size(1536), "1.5 KB");
        assert_eq!(ContentSummarizer::format_file_size(1048576), "1.0 MB");
        assert_eq!(ContentSummarizer::format_file_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_empty_text_content() -> Result<(), Box<dyn std::error::Error>> {
        let text_content = TextContent::new(None, None)?;
        let content = ContentItem::new(ContentType::Text(text_content))?;
        let summarizer = ContentSummarizer::new();
        
        let summary = summarizer.summarize_content(&content, 100);
        
        assert_eq!(summary.content_type, "Text");
        assert_eq!(summary.summary, "Empty text content");
        
        Ok(())
    }

    #[test]
    fn test_all_content_types() -> Result<(), Box<dyn std::error::Error>> {
        let summarizer = ContentSummarizer::new();
        
        // Text
        let text_content = TextContent::new(None, Some("Test".to_string()))?;
        let text_item = ContentItem::new(ContentType::Text(text_content))?;
        let text_summary = summarizer.summarize_content(&text_item, 100);
        assert_eq!(text_summary.content_type, "Text");
        
        // Video - use None for path in tests
        let video_content = VideoContent::new(None, 100)?;
        let video_item = ContentItem::new(ContentType::Video(video_content))?;
        let video_summary = summarizer.summarize_content(&video_item, 100);
        assert_eq!(video_summary.content_type, "Video");
        
        // Audio - use None for path in tests
        let audio_content = AudioContent::new(None, 200)?;
        let audio_item = ContentItem::new(ContentType::Audio(audio_content))?;
        let audio_summary = summarizer.summarize_content(&audio_item, 100);
        assert_eq!(audio_summary.content_type, "Audio");
        
        // Image - use None for path in tests
        let image_content = ImageContent::new(None, (800, 600))?;
        let image_item = ContentItem::new(ContentType::Image(image_content))?;
        let image_summary = summarizer.summarize_content(&image_item, 100);
        assert_eq!(image_summary.content_type, "Image");
        
        Ok(())
    }
}