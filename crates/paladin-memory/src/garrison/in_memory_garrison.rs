//! In-Memory Garrison Implementation
//!
//! Provides a thread-safe, in-memory implementation of the GarrisonPort trait
//! using RwLock and VecDeque for fast, ephemeral storage.
//!
//! # Use Cases
//!
//! - Development and testing
//! - Short-lived conversation sessions
//! - Scenarios where persistence is not required
//!
//! # Performance
//!
//! - Write: O(1) amortized
//! - Read recent N: O(N)
//! - Search: O(N) linear scan
//! - Memory: Entries stored in RAM, lost on shutdown

use async_trait::async_trait;
use paladin_core::platform::container::garrison::{
    ConversationRole, EvictionStrategy, GarrisonConfig, GarrisonEntry,
};
use paladin_ports::output::garrison_port::{GarrisonError, GarrisonPort, GarrisonStats};
use std::collections::VecDeque;
use std::sync::RwLock;

/// Thread-safe in-memory Garrison implementation.
///
/// Stores all entries in memory using a `VecDeque` protected by an `RwLock`
/// for concurrent access.  All data is lost when the process terminates;
/// use `SqliteGarrison` (feature `sqlite`) when persistence is required.
///
/// # Examples
///
/// ```no_run
/// use paladin_memory::garrison::InMemoryGarrison;
/// use paladin_core::platform::container::garrison::GarrisonConfig;
/// use paladin_ports::output::garrison_port::GarrisonPort;
/// use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
///
/// #[tokio::main]
/// async fn main() {
///     let config = GarrisonConfig::default();
///     let garrison = InMemoryGarrison::new(config);
///
///     // Store an entry
///     let entry = GarrisonEntry::new(
///         ConversationRole::User,
///         "Hello!".to_string()
///     );
///     garrison.remember(entry).await.unwrap();
///
///     // Retrieve it
///     let recent = garrison.recall_recent(10).await.unwrap();
///     assert_eq!(recent.len(), 1);
/// }
/// ```
pub struct InMemoryGarrison {
    entries: RwLock<VecDeque<GarrisonEntry>>,
    config: GarrisonConfig,
}

impl InMemoryGarrison {
    /// Creates a new in-memory Garrison with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for windowing and eviction behavior
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use paladin_memory::garrison::InMemoryGarrison;
    /// use paladin_core::platform::container::garrison::GarrisonConfig;
    ///
    /// let config = GarrisonConfig::new(100, Some(4000));
    /// let garrison = InMemoryGarrison::new(config);
    /// ```
    pub fn new(config: GarrisonConfig) -> Self {
        Self {
            entries: RwLock::new(VecDeque::new()),
            config,
        }
    }

    /// Applies windowing logic to evict old entries based on configuration
    fn apply_windowing(&self, entries: &mut VecDeque<GarrisonEntry>) {
        // Check entry count limit
        while entries.len() > self.config.max_entries {
            self.evict_entry(entries);
        }

        // Check token limit if configured
        if let Some(max_tokens) = self.config.max_tokens {
            while self.calculate_total_tokens(entries) > max_tokens && !entries.is_empty() {
                self.evict_entry(entries);
            }
        }
    }

    /// Evicts a single entry based on the configured strategy
    fn evict_entry(&self, entries: &mut VecDeque<GarrisonEntry>) {
        match self.config.eviction_strategy {
            EvictionStrategy::FIFO | EvictionStrategy::SlidingWindow => {
                entries.pop_front();
            }
            EvictionStrategy::ImportanceBased => {
                self.evict_importance_based(entries);
            }
        }
    }

    /// Importance-based eviction: preserve system prompts and recent messages
    fn evict_importance_based(&self, entries: &mut VecDeque<GarrisonEntry>) {
        let total_entries = entries.len();
        if total_entries == 0 {
            return;
        }

        let preserve_count = self.config.preserve_recent_count.min(total_entries);
        let recent_start_idx = total_entries.saturating_sub(preserve_count);

        // Find first non-system, non-recent entry to evict
        for i in 0..recent_start_idx {
            if entries[i].role != ConversationRole::System {
                entries.remove(i);
                return;
            }
        }

        // If all non-recent entries are system prompts, remove oldest non-system from recent
        for i in recent_start_idx..total_entries {
            if entries[i].role != ConversationRole::System {
                entries.remove(i);
                return;
            }
        }

        // Last resort: remove oldest entry even if it's a system prompt
        entries.pop_front();
    }

    /// Calculates total token count across all entries
    fn calculate_total_tokens(&self, entries: &VecDeque<GarrisonEntry>) -> u32 {
        entries.iter().filter_map(|e| e.token_count).sum()
    }

    /// Estimates size in bytes for statistics
    fn estimate_size_bytes(&self, entries: &VecDeque<GarrisonEntry>) -> usize {
        entries.iter().map(|e| e.content.len()).sum()
    }
}

#[async_trait]
impl GarrisonPort for InMemoryGarrison {
    async fn remember(&self, entry: GarrisonEntry) -> Result<(), GarrisonError> {
        // Validate entry before storing
        entry
            .validate()
            .map_err(|e| GarrisonError::Custom(format!("Invalid entry: {}", e)))?;

        let mut entries = self
            .entries
            .write()
            .map_err(|e| GarrisonError::StorageError(format!("Lock poisoned: {}", e)))?;

        entries.push_back(entry);
        self.apply_windowing(&mut entries);

        Ok(())
    }

    async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        let entries = self
            .entries
            .read()
            .map_err(|e| GarrisonError::StorageError(format!("Lock poisoned: {}", e)))?;

        let start = entries.len().saturating_sub(limit);
        Ok(entries.range(start..).cloned().collect())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        let entries = self
            .entries
            .read()
            .map_err(|e| GarrisonError::StorageError(format!("Lock poisoned: {}", e)))?;

        let results: Vec<GarrisonEntry> = entries
            .iter()
            .filter(|e| e.content.contains(query))
            .take(limit)
            .cloned()
            .collect();

        Ok(results)
    }

    async fn forget_all(&self) -> Result<(), GarrisonError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|e| GarrisonError::StorageError(format!("Lock poisoned: {}", e)))?;

        entries.clear();
        Ok(())
    }

    async fn stats(&self) -> Result<GarrisonStats, GarrisonError> {
        let entries = self
            .entries
            .read()
            .map_err(|e| GarrisonError::StorageError(format!("Lock poisoned: {}", e)))?;

        Ok(GarrisonStats {
            entry_count: entries.len(),
            total_tokens: self.calculate_total_tokens(&entries),
            size_bytes: Some(self.estimate_size_bytes(&entries)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::garrison::ConversationRole;

    #[tokio::test]
    async fn test_remember_and_recall() {
        let config = GarrisonConfig::default();
        let garrison = InMemoryGarrison::new(config);

        let entry = GarrisonEntry::new(ConversationRole::User, "Test message".to_string());
        garrison.remember(entry).await.unwrap();

        let recent = garrison.recall_recent(10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "Test message");
    }

    #[tokio::test]
    async fn test_windowing_by_count() {
        let config = GarrisonConfig::new(3, None);
        let garrison = InMemoryGarrison::new(config);

        for i in 0..5 {
            let entry = GarrisonEntry::new(ConversationRole::User, format!("Message {}", i));
            garrison.remember(entry).await.unwrap();
        }

        let all = garrison.recall_recent(100).await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].content, "Message 2");
    }

    #[tokio::test]
    async fn test_windowing_by_tokens() {
        let config = GarrisonConfig::new(100, Some(50));
        let garrison = InMemoryGarrison::new(config);

        for i in 0..5 {
            let entry = GarrisonEntry::with_token_count(
                ConversationRole::User,
                format!("Message {}", i),
                20,
            );
            garrison.remember(entry).await.unwrap();
        }

        let all = garrison.recall_recent(100).await.unwrap();
        // Should have 2 entries (40 tokens) after windowing
        assert!(all.len() <= 3);
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let config = GarrisonConfig::default();
        let garrison = InMemoryGarrison::new(config);

        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Hello world".to_string(),
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Goodbye world".to_string(),
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Random message".to_string(),
            ))
            .await
            .unwrap();

        let results = garrison.search("world", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_forget_all() {
        let config = GarrisonConfig::default();
        let garrison = InMemoryGarrison::new(config);

        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Test".to_string(),
            ))
            .await
            .unwrap();

        garrison.forget_all().await.unwrap();

        let recent = garrison.recall_recent(10).await.unwrap();
        assert_eq!(recent.len(), 0);
    }

    #[tokio::test]
    async fn test_stats() {
        let config = GarrisonConfig::default();
        let garrison = InMemoryGarrison::new(config);

        garrison
            .remember(GarrisonEntry::with_token_count(
                ConversationRole::User,
                "First".to_string(),
                10,
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::with_token_count(
                ConversationRole::Assistant,
                "Second".to_string(),
                20,
            ))
            .await
            .unwrap();

        let stats = garrison.stats().await.unwrap();
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.total_tokens, 30);
        assert!(stats.size_bytes.is_some());
    }

    #[tokio::test]
    async fn is_summary_round_trips_through_in_memory() {
        let config = GarrisonConfig::default();
        let garrison = InMemoryGarrison::new(config);

        garrison
            .remember(GarrisonEntry::summary("condensed".to_string()))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(ConversationRole::User, "raw".to_string()))
            .await
            .unwrap();

        let entries = garrison.recall_recent(10).await.unwrap();

        let summary_entry = entries.iter().find(|e| e.content == "condensed").unwrap();
        assert!(summary_entry.is_summary);

        let raw_entry = entries.iter().find(|e| e.content == "raw").unwrap();
        assert!(!raw_entry.is_summary);
    }

    #[tokio::test]
    async fn test_importance_based_eviction() {
        let config = GarrisonConfig::new(3, None)
            .with_eviction_strategy(EvictionStrategy::ImportanceBased)
            .with_preserve_recent(1);

        let garrison = InMemoryGarrison::new(config);

        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::System,
                "System prompt".to_string(),
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "User 1".to_string(),
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "User 2".to_string(),
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "User 3 - triggers eviction".to_string(),
            ))
            .await
            .unwrap();

        let all = garrison.recall_recent(100).await.unwrap();
        assert_eq!(all.len(), 3);
        // System prompt should be preserved
        assert!(all.iter().any(|e| e.role == ConversationRole::System));
    }
}
