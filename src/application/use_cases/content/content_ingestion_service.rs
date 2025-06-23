/*
Content Ingestion Service

This module defines the `ContentIngestionService` trait and its implementation.
This service is responsible for managing the ingestion of content from various sources.

Ingestion differs from content fetching in that it involves processing and storing content
from sources like RSS feeds, web pages, or other content providers, rather than simply fetching
content from a database or cache.
*/

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use url::Url;

use crate::core::platform::container::content::{ContentItem, ContentType, TextContent};
use crate::core::platform::manager::orchestrator::{Orchestrator, OrchestrationContext, ContentAnalysisType};

/// Content repository trait that must be thread-safe
#[async_trait]
pub trait ContentRepository: Send + Sync {
    async fn create(&self, content: ContentItem) -> Result<Uuid, String>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<ContentItem>, String>;
    async fn update(&self, content: ContentItem) -> Result<(), String>;
    async fn delete(&self, id: Uuid) -> Result<(), String>;
    async fn list(&self) -> Result<Vec<ContentItem>, String>;
}

/// Content ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    /// Maximum content size in bytes
    pub max_content_size: usize,
    /// Enable automatic content analysis
    pub auto_analyze: bool,
    /// Analysis types to run automatically
    pub analysis_types: Vec<ContentAnalysisType>,
    /// Batch size for processing multiple items
    pub batch_size: usize,
    /// Maximum concurrent ingestion tasks
    pub max_concurrent: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_content_size: 10 * 1024 * 1024, // 10MB
            auto_analyze: true,
            analysis_types: vec![
                ContentAnalysisType::LanguageDetection,
                ContentAnalysisType::KeywordExtraction,
            ],
            batch_size: 50,
            max_concurrent: 10,
        }
    }
}

/// Content source definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSource {
    pub id: Uuid,
    pub name: String,
    pub source_type: SourceType,
    pub url: Option<Url>,
    pub config: SourceConfig,
    pub enabled: bool,
    pub last_ingested: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    RssFeed,
    WebPage,
    Directory,
    Database,
    Api,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// How frequently to check for new content (in seconds)
    pub check_interval: u64,
    /// Authentication credentials if needed
    pub auth: Option<AuthConfig>,
    /// Custom headers for HTTP requests
    pub headers: HashMap<String, String>,
    /// Source-specific parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub credentials: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    Basic,
    Bearer,
    ApiKey,
    OAuth2,
}

/// Ingestion result for a single content item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    pub content_id: Option<Uuid>,
    pub source_url: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub ingested_at: DateTime<Utc>,
    pub processing_time_ms: u64,
    pub content_size: Option<usize>,
    pub analysis_triggered: bool,
}

/// Batch ingestion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestionResult {
    pub total_items: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<IngestionResult>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_processing_time_ms: u64,
}

/// Content ingestion errors
#[derive(Debug, Error)]
pub enum IngestionError {
    #[error("Source not found: {0}")]
    SourceNotFound(Uuid),
    #[error("Invalid content: {0}")]
    InvalidContent(String),
    #[error("Content too large: {size} bytes (max: {max})")]
    ContentTooLarge { size: usize, max: usize },
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Source disabled: {0}")]
    SourceDisabled(Uuid),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Authentication failed")]
    AuthenticationFailed,
}

/// Content ingestion service trait
#[async_trait]
pub trait ContentIngestionService: Send + Sync {
    /// Register a new content source
    async fn register_source(&self, source: ContentSource) -> Result<Uuid, IngestionError>;
    
    /// Remove a content source
    async fn remove_source(&self, source_id: Uuid) -> Result<(), IngestionError>;
    
    /// Update source configuration
    async fn update_source(&self, source_id: Uuid, config: SourceConfig) -> Result<(), IngestionError>;
    
    /// Enable/disable a source
    async fn set_source_enabled(&self, source_id: Uuid, enabled: bool) -> Result<(), IngestionError>;
    
    /// Ingest content from a single URL
    async fn ingest_from_url(&self, url: Url, source_id: Option<Uuid>) -> Result<IngestionResult, IngestionError>;
    
    /// Ingest raw content directly
    async fn ingest_content(&self, content: String, metadata: HashMap<String, serde_json::Value>) -> Result<IngestionResult, IngestionError>;
    
    /// Ingest content from a registered source
    async fn ingest_from_source(&self, source_id: Uuid) -> Result<BatchIngestionResult, IngestionError>;
    
    /// Ingest content from all enabled sources
    async fn ingest_from_all_sources(&self) -> Result<Vec<BatchIngestionResult>, IngestionError>;
    
    /// Get list of registered sources
    async fn list_sources(&self) -> Result<Vec<ContentSource>, IngestionError>;
    
    /// Get ingestion statistics
    async fn get_stats(&self) -> Result<IngestionStats, IngestionError>;
    
    /// Start automatic ingestion scheduler
    async fn start_scheduler(&self) -> Result<(), IngestionError>;
    
    /// Stop automatic ingestion scheduler
    async fn stop_scheduler(&self) -> Result<(), IngestionError>;
}

/// Ingestion statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionStats {
    pub total_sources: usize,
    pub enabled_sources: usize,
    pub total_items_ingested: u64,
    pub items_ingested_today: u64,
    pub failed_ingestions: u64,
    pub average_processing_time_ms: f64,
    pub last_ingestion: Option<DateTime<Utc>>,
    pub scheduler_running: bool,
}

/// Default implementation of ContentIngestionService
pub struct DefaultContentIngestionService {
    config: IngestionConfig,
    sources: Arc<RwLock<HashMap<Uuid, ContentSource>>>,
    orchestrator: Arc<Orchestrator>,
    repository: Arc<dyn ContentRepository>,
    scheduler_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<IngestionStats>>,
}

impl DefaultContentIngestionService {
    pub fn new(
        config: IngestionConfig,
        orchestrator: Arc<Orchestrator>,
        repository: Arc<dyn ContentRepository>,
    ) -> Self {
        Self {
            config,
            sources: Arc::new(RwLock::new(HashMap::new())),
            orchestrator,
            repository,
            scheduler_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(IngestionStats::default())),
        }
    }
    
    /// Parse content from different source types
    async fn parse_content(&self, source: &ContentSource, raw_content: String) -> Result<Vec<ContentItem>, IngestionError> {
        let mut items = Vec::new();
        
        match source.source_type {
            SourceType::RssFeed => {
                // Parse RSS/Atom feed
                items.extend(self.parse_rss_feed(raw_content).await?);
            }
            SourceType::WebPage => {
                // Parse HTML content
                let content_item = self.parse_web_page(raw_content, source.url.clone()).await?;
                items.push(content_item);
            }
            SourceType::Api => {
                // Parse JSON API response
                items.extend(self.parse_api_response(raw_content).await?);
            }
            _ => {
                // Default: treat as plain text
                let url_string = source.url.as_ref().map(|u| u.to_string());
                let text_content = TextContent::new(url_string, Some(raw_content))
                    .map_err(|e| IngestionError::ParseError(e.to_string()))?;
                let content_item = ContentItem::new(ContentType::Text(text_content))
                    .map_err(|e| IngestionError::ParseError(e.to_string()))?;
                items.push(content_item);
            }
        }
        
        Ok(items)
    }
    
    async fn parse_rss_feed(&self, _content: String) -> Result<Vec<ContentItem>, IngestionError> {
        // Implement RSS parsing logic
        // This would use a crate like `rss` or `feed-rs`
        Ok(vec![])
    }
    
    async fn parse_web_page(&self, content: String, url: Option<Url>) -> Result<ContentItem, IngestionError> {
        // Extract text content from HTML
        // This would use a crate like `scraper` or `html2text`
        let url_string = url.map(|u| u.to_string());
        let text_content = TextContent::new(url_string, Some(content))
            .map_err(|e| IngestionError::ParseError(e.to_string()))?;
        ContentItem::new(ContentType::Text(text_content))
            .map_err(|e| IngestionError::ParseError(e.to_string()))
    }
    
    async fn parse_api_response(&self, _content: String) -> Result<Vec<ContentItem>, IngestionError> {
        // Parse structured API response
        Ok(vec![])
    }
    
    /// Fetch content from a URL
    async fn fetch_from_url(&self, url: &Url, _source: &ContentSource) -> Result<String, IngestionError> {
        // This would use reqwest or similar HTTP client
        // For now, return a placeholder
        Ok(format!("Content from {}", url))
    }
    
    /// Get content size for different content types
    fn get_content_size(content_type: &ContentType) -> usize {
        match content_type {
            ContentType::Text(text_content) => {
                text_content.content.as_ref().map(|t| t.len()).unwrap_or(0)
            }
            ContentType::Video(video_content) => video_content.filesize as usize,
            ContentType::Audio(audio_content) => audio_content.filesize as usize,
            ContentType::Image(image_content) => image_content.filesize as usize,
        }
    }
    
    /// Trigger content analysis if enabled
    async fn trigger_analysis(&self, content_item: &ContentItem) -> Result<(), IngestionError> {
        if !self.config.auto_analyze {
            return Ok(());
        }
        
        let context = OrchestrationContext::new(
            "content_ingestion_service".to_string(),
            "production".to_string(),
        );
        
        for analysis_type in &self.config.analysis_types {
            let _ = self.orchestrator.create_content_analysis_workflow(
                vec![content_item.clone()],
                analysis_type.clone(),
                context.clone(),
            ).await;
        }
        
        Ok(())
    }
}

#[async_trait]
impl ContentIngestionService for DefaultContentIngestionService {
    async fn register_source(&self, mut source: ContentSource) -> Result<Uuid, IngestionError> {
        source.created_at = Utc::now();
        let source_id = source.id;
        
        let mut sources = self.sources.write().await;
        sources.insert(source_id, source);
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_sources = sources.len();
        stats.enabled_sources = sources.values().filter(|s| s.enabled).count();
        
        Ok(source_id)
    }
    
    async fn remove_source(&self, source_id: Uuid) -> Result<(), IngestionError> {
        let mut sources = self.sources.write().await;
        sources.remove(&source_id)
            .ok_or(IngestionError::SourceNotFound(source_id))?;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_sources = sources.len();
        stats.enabled_sources = sources.values().filter(|s| s.enabled).count();
        
        Ok(())
    }
    
    async fn update_source(&self, source_id: Uuid, config: SourceConfig) -> Result<(), IngestionError> {
        let mut sources = self.sources.write().await;
        let source = sources.get_mut(&source_id)
            .ok_or(IngestionError::SourceNotFound(source_id))?;
        
        source.config = config;
        Ok(())
    }
    
    async fn set_source_enabled(&self, source_id: Uuid, enabled: bool) -> Result<(), IngestionError> {
        let mut sources = self.sources.write().await;
        let source = sources.get_mut(&source_id)
            .ok_or(IngestionError::SourceNotFound(source_id))?;
        
        source.enabled = enabled;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.enabled_sources = sources.values().filter(|s| s.enabled).count();
        
        Ok(())
    }
    
    async fn ingest_from_url(&self, url: Url, source_id: Option<Uuid>) -> Result<IngestionResult, IngestionError> {
        let start_time = std::time::Instant::now();
        
        // Create temporary source if none provided
        let temp_source = ContentSource {
            id: source_id.unwrap_or_else(Uuid::new_v4),
            name: format!("Temporary source for {}", url),
            source_type: SourceType::WebPage,
            url: Some(url.clone()),
            config: SourceConfig {
                check_interval: 0,
                auth: None,
                headers: HashMap::new(),
                parameters: HashMap::new(),
            },
            enabled: true,
            last_ingested: None,
            created_at: Utc::now(),
        };
        
        // Fetch content
        let raw_content = self.fetch_from_url(&url, &temp_source).await?;
        
        // Check content size
        if raw_content.len() > self.config.max_content_size {
            return Err(IngestionError::ContentTooLarge {
                size: raw_content.len(),
                max: self.config.max_content_size,
            });
        }
        
        // Parse content
        let content_items = self.parse_content(&temp_source, raw_content).await?;
        
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        if let Some(content_item) = content_items.first() {
            // Store content
            self.repository.create(content_item.clone()).await
                .map_err(|e| IngestionError::StorageError(e.to_string()))?;
            
            // Trigger analysis
            let analysis_triggered = self.trigger_analysis(content_item).await.is_ok();
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_items_ingested += 1;
            stats.last_ingestion = Some(Utc::now());
            
            Ok(IngestionResult {
                content_id: Some(content_item.uuid()),
                source_url: Some(url.to_string()),
                success: true,
                error: None,
                ingested_at: Utc::now(),
                processing_time_ms,
                content_size: Some(Self::get_content_size(content_item.content())),
                analysis_triggered,
            })
        } else {
            Ok(IngestionResult {
                content_id: None,
                source_url: Some(url.to_string()),
                success: false,
                error: Some("No content items parsed".to_string()),
                ingested_at: Utc::now(),
                processing_time_ms,
                content_size: None,
                analysis_triggered: false,
            })
        }
    }
    
    async fn ingest_content(&self, content: String, _metadata: HashMap<String, serde_json::Value>) -> Result<IngestionResult, IngestionError> {
        let start_time = std::time::Instant::now();
        
        // Check content size
        if content.len() > self.config.max_content_size {
            return Err(IngestionError::ContentTooLarge {
                size: content.len(),
                max: self.config.max_content_size,
            });
        }
        
        // Create content item
        let text_content = TextContent::new(None, Some(content))
            .map_err(|e| IngestionError::ParseError(e.to_string()))?;
        let content_item = ContentItem::new(ContentType::Text(text_content))
            .map_err(|e| IngestionError::ParseError(e.to_string()))?;
        
        // Store content
        self.repository.create(content_item.clone()).await
            .map_err(|e| IngestionError::StorageError(e.to_string()))?;
        
        // Trigger analysis
        let analysis_triggered = self.trigger_analysis(&content_item).await.is_ok();
        
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_items_ingested += 1;
        stats.last_ingestion = Some(Utc::now());
        
        Ok(IngestionResult {
            content_id: Some(content_item.uuid()),
            source_url: None,
            success: true,
            error: None,
            ingested_at: Utc::now(),
            processing_time_ms,
            content_size: Some(Self::get_content_size(content_item.content())),
            analysis_triggered,
        })
    }
    
    async fn ingest_from_source(&self, source_id: Uuid) -> Result<BatchIngestionResult, IngestionError> {
        let start_time = std::time::Instant::now();
        let started_at = Utc::now();
        
        let source = {
            let sources = self.sources.read().await;
            sources.get(&source_id)
                .ok_or(IngestionError::SourceNotFound(source_id))?
                .clone()
        };
        
        if !source.enabled {
            return Err(IngestionError::SourceDisabled(source_id));
        }
        
        let mut results = Vec::new();
        
        if let Some(url) = &source.url {
            match self.ingest_from_url(url.clone(), Some(source_id)).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(IngestionResult {
                        content_id: None,
                        source_url: Some(url.to_string()),
                        success: false,
                        error: Some(e.to_string()),
                        ingested_at: Utc::now(),
                        processing_time_ms: 0,
                        content_size: None,
                        analysis_triggered: false,
                    });
                }
            }
        }
        
        // Update source last_ingested timestamp
        {
            let mut sources = self.sources.write().await;
            if let Some(source) = sources.get_mut(&source_id) {
                source.last_ingested = Some(Utc::now());
            }
        }
        
        let completed_at = Utc::now();
        let total_processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.len() - successful;
        
        Ok(BatchIngestionResult {
            total_items: results.len(),
            successful,
            failed,
            results,
            started_at,
            completed_at,
            total_processing_time_ms,
        })
    }
    
    async fn ingest_from_all_sources(&self) -> Result<Vec<BatchIngestionResult>, IngestionError> {
        let source_ids: Vec<Uuid> = {
            let sources = self.sources.read().await;
            sources.values()
                .filter(|s| s.enabled)
                .map(|s| s.id)
                .collect()
        };
        
        let mut batch_results = Vec::new();
        
        for source_id in source_ids {
            match self.ingest_from_source(source_id).await {
                Ok(result) => batch_results.push(result),
                Err(e) => {
                    println!("Failed to ingest from source {}: {}", source_id, e);
                    // Continue with other sources
                }
            }
        }
        
        Ok(batch_results)
    }
    
    async fn list_sources(&self) -> Result<Vec<ContentSource>, IngestionError> {
        let sources = self.sources.read().await;
        Ok(sources.values().cloned().collect())
    }
    
    async fn get_stats(&self) -> Result<IngestionStats, IngestionError> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    async fn start_scheduler(&self) -> Result<(), IngestionError> {
        let mut scheduler_running = self.scheduler_running.write().await;
        if *scheduler_running {
            return Ok(());
        }
        
        *scheduler_running = true;
        
        // Start background task for periodic ingestion
        let sources = Arc::clone(&self.sources);
        let service = Arc::new(self.clone());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Check every minute
            
            loop {
                interval.tick().await;
                
                let source_ids: Vec<Uuid> = {
                    let sources = sources.read().await;
                    sources.values()
                        .filter(|s| s.enabled)
                        .filter(|s| {
                            // Check if enough time has passed since last ingestion
                            if let Some(last_ingested) = s.last_ingested {
                                let elapsed = (Utc::now() - last_ingested).num_seconds() as u64;
                                elapsed >= s.config.check_interval
                            } else {
                                true // Never ingested, so ingest now
                            }
                        })
                        .map(|s| s.id)
                        .collect()
                };
                
                for source_id in source_ids {
                    let _ = service.ingest_from_source(source_id).await;
                }
            }
        });
        
        println!("Content ingestion scheduler started");
        Ok(())
    }
    
    async fn stop_scheduler(&self) -> Result<(), IngestionError> {
        let mut scheduler_running = self.scheduler_running.write().await;
        *scheduler_running = false;
        
        println!("Content ingestion scheduler stopped");
        Ok(())
    }
}

impl Clone for DefaultContentIngestionService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            sources: Arc::clone(&self.sources),
            orchestrator: Arc::clone(&self.orchestrator),
            repository: Arc::clone(&self.repository),
            scheduler_running: Arc::clone(&self.scheduler_running),
            stats: Arc::clone(&self.stats),
        }
    }
}

impl Default for IngestionStats {
    fn default() -> Self {
        Self {
            total_sources: 0,
            enabled_sources: 0,
            total_items_ingested: 0,
            items_ingested_today: 0,
            failed_ingestions: 0,
            average_processing_time_ms: 0.0,
            last_ingestion: None,
            scheduler_running: false,
        }
    }
}

/// In-memory implementation of ContentRepository for testing
#[derive(Debug, Clone)]
pub struct InMemoryContentRepository {
    items: Arc<RwLock<HashMap<Uuid, ContentItem>>>,
}

impl InMemoryContentRepository {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ContentRepository for InMemoryContentRepository {
    async fn create(&self, content: ContentItem) -> Result<Uuid, String> {
        let id = content.uuid();
        let mut items = self.items.write().await;
        items.insert(id, content);
        Ok(id)
    }
    
    async fn get_by_id(&self, id: Uuid) -> Result<Option<ContentItem>, String> {
        let items = self.items.read().await;
        Ok(items.get(&id).cloned())
    }
    
    async fn update(&self, content: ContentItem) -> Result<(), String> {
        let id = content.uuid();
        let mut items = self.items.write().await;
        items.insert(id, content);
        Ok(())
    }
    
    async fn delete(&self, id: Uuid) -> Result<(), String> {
        let mut items = self.items.write().await;
        items.remove(&id);
        Ok(())
    }
    
    async fn list(&self) -> Result<Vec<ContentItem>, String> {
        let items = self.items.read().await;
        Ok(items.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_content_ingestion_service_creation() {
        let config = IngestionConfig::default();
        let orchestrator = Arc::new(Orchestrator::new());
        let repository = Arc::new(InMemoryContentRepository::new());
        
        let service = DefaultContentIngestionService::new(config, orchestrator, repository);
        
        let stats = service.get_stats().await.unwrap();
        assert_eq!(stats.total_sources, 0);
        assert_eq!(stats.enabled_sources, 0);
    }
    
    #[tokio::test]
    async fn test_source_registration() {
        let config = IngestionConfig::default();
        let orchestrator = Arc::new(Orchestrator::new());
        let repository = Arc::new(InMemoryContentRepository::new());
        
        let service = DefaultContentIngestionService::new(config, orchestrator, repository);
        
        let source = ContentSource {
            id: Uuid::new_v4(),
            name: "Test Source".to_string(),
            source_type: SourceType::WebPage,
            url: Some("https://example.com".parse().unwrap()),
            config: SourceConfig {
                check_interval: 3600,
                auth: None,
                headers: HashMap::new(),
                parameters: HashMap::new(),
            },
            enabled: true,
            last_ingested: None,
            created_at: Utc::now(),
        };
        
        let source_id = service.register_source(source).await.unwrap();
        
        let sources = service.list_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, source_id);
    }
    
    #[tokio::test]
    async fn test_content_ingestion() {
        let config = IngestionConfig::default();
        let orchestrator = Arc::new(Orchestrator::new());
        let repository = Arc::new(InMemoryContentRepository::new());
        
        let service = DefaultContentIngestionService::new(config, orchestrator, repository);
        
        let content = "This is test content for ingestion".to_string();
        let metadata = HashMap::new();
        
        let result = service.ingest_content(content, metadata).await.unwrap();
        
        assert!(result.success);
        assert!(result.content_id.is_some());
        assert!(result.content_size.is_some());
        assert!(result.processing_time_ms > 0);
    }
}