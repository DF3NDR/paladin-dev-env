use crate::core::platform::container::content::{ContentItem, ContentType, TextContent};
use crate::application::use_cases::content::content_fetching_service::ContentFetchingService;
use url::Url;

#[derive(Debug, Clone)]
pub struct HttpContentFetcher {
    client: reqwest::blocking::Client,
}

impl HttpContentFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Default for HttpContentFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentFetchingService for HttpContentFetcher {
    fn fetch_content(&self, url: &str) -> Result<ContentItem, String> {
        // Validate URL
        let parsed_url = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        
        // Fetch content from URL
        let response = self.client
            .get(url)
            .send()
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        // Get content body
        let body = response.text().map_err(|e| format!("Failed to read response body: {}", e))?;
        let content_length = body.len() as u64;

        // Create text content with the fetched body
        let text_content = TextContent {
            path: None,
            content: Some(body),
            filesize: content_length,
        };

        let content_type = ContentType::Text(text_content);

        // Create ContentItem
        let mut content_item = ContentItem::new(content_type)
            .map_err(|e| format!("Failed to create content item: {}", e))?;

        // Set URL-specific metadata
        content_item.url = Some(parsed_url.clone());
        content_item.source_url = Some(parsed_url);
        
        // Try to extract title from HTML if it's HTML content
        if let ContentType::Text(ref text_content) = content_item.content {
            if let Some(ref html_content) = text_content.content {
                content_item.title = extract_title_from_html(html_content);
            }
        }

        content_item.source = Some("web".to_string());
        content_item.tags = Some(vec!["web".to_string()]);

        Ok(content_item)
    }
}

// Helper function to extract title from HTML content
fn extract_title_from_html(html: &str) -> Option<String> {
    // Simple regex-based title extraction
    // In a production system, you'd want to use a proper HTML parser like scraper or html5ever
    let title_regex = regex::Regex::new(r"<title[^>]*>([^<]*)</title>").ok()?;
    title_regex
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn test_fetch_content_success() {
        let mut server = Server::new();
        let mock = server.mock("GET", "/test")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("<html><head><title>Test Page</title></head><body>Test content</body></html>")
            .create();

        let fetcher = HttpContentFetcher::new();
        let url = format!("{}/test", server.url());
        let result = fetcher.fetch_content(&url);

        assert!(result.is_ok());
        let content_item = result.unwrap();
        
        assert!(content_item.title.is_some());
        assert_eq!(content_item.title.unwrap(), "Test Page");
        assert!(matches!(content_item.content, ContentType::Text(_)));
        
        if let ContentType::Text(text_content) = content_item.content {
            assert!(text_content.content.is_some());
            assert!(text_content.content.unwrap().contains("Test content"));
        }

        mock.assert();
    }

    #[test]
    fn test_fetch_content_invalid_url() {
        let fetcher = HttpContentFetcher::new();
        let result = fetcher.fetch_content("not-a-valid-url");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid URL"));
    }

    #[test]
    fn test_fetch_content_http_error() {
        let mut server = Server::new();
        let mock = server.mock("GET", "/error")
            .with_status(404)
            .create();

        let fetcher = HttpContentFetcher::new();
        let url = format!("{}/error", server.url());
        let result = fetcher.fetch_content(&url);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HTTP error"));

        mock.assert();
    }

    #[test]
    fn test_extract_title_from_html() {
        let html = "<html><head><title>My Page Title</title></head><body>Content</body></html>";
        let title = extract_title_from_html(html);
        assert_eq!(title, Some("My Page Title".to_string()));

        let html_no_title = "<html><body>Content without title</body></html>";
        let no_title = extract_title_from_html(html_no_title);
        assert_eq!(no_title, None);
    }

    #[test]
    fn test_default_construction() {
        let _fetcher = HttpContentFetcher::default();
        // Just ensure it can be constructed
    }
}