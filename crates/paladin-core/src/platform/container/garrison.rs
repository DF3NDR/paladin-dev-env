//! Garrison Memory System - Core Domain Layer
//!
//! This module provides the core domain entities for the Garrison memory system,
//! which enables Paladins to maintain conversation context and persist knowledge
//! across sessions.
//!
//! # Domain Entities
//!
//! - [`ConversationRole`]: The role of a participant in a conversation
//! - [`GarrisonEntry`]: A single memory entry in the Garrison
//! - [`GarrisonType`]: Classification of memory types
//! - [`ConversationHistory`]: Windowed conversation history
//! - [`GarrisonConfig`]: Configuration for Garrison behavior
//! - [`EvictionStrategy`]: Strategy for removing old entries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// Role of a participant in a conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversationRole {
    /// System-level instructions or prompts
    System,
    /// User input
    User,
    /// Assistant (Paladin) response
    Assistant,
    /// Tool or function call result
    Tool,
}

/// A single memory entry in the Garrison
///
/// Represents one message or interaction in the conversation history.
/// Each entry is immutable once created and can be serialized for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarrisonEntry {
    /// Unique identifier for this entry
    pub id: Uuid,
    /// Role of the message sender
    pub role: ConversationRole,
    /// The content of the message
    pub content: String,
    /// When this entry was created
    pub timestamp: DateTime<Utc>,
    /// Additional metadata (extensible key-value storage)
    pub metadata: HashMap<String, Value>,
    /// Optional token count for this entry's content
    pub token_count: Option<u32>,
}

impl GarrisonEntry {
    /// Creates a new Garrison entry
    ///
    /// # Arguments
    ///
    /// * `role` - The conversation role
    /// * `content` - The message content
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::garrison::{GarrisonEntry, ConversationRole};
    ///
    /// let entry = GarrisonEntry::new(
    ///     ConversationRole::User,
    ///     "Hello, Paladin!".to_string()
    /// );
    /// assert!(!entry.content.is_empty());
    /// ```
    pub fn new(role: ConversationRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            token_count: None,
        }
    }

    /// Creates a new entry with metadata
    pub fn with_metadata(
        role: ConversationRole,
        content: String,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp: Utc::now(),
            metadata,
            token_count: None,
        }
    }

    /// Creates a new entry with a pre-calculated token count
    pub fn with_token_count(role: ConversationRole, content: String, token_count: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            token_count: Some(token_count),
        }
    }

    /// Validates that all required fields are populated
    ///
    /// # Errors
    ///
    /// Returns an error message if validation fails
    pub fn validate(&self) -> Result<(), String> {
        if self.content.is_empty() {
            return Err("Content cannot be empty".to_string());
        }
        Ok(())
    }

    /// Sets the token count for this entry
    pub fn set_token_count(&mut self, count: u32) {
        self.token_count = Some(count);
    }

    /// Adds metadata to this entry
    pub fn add_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }
}

/// Classification of memory storage types in the Garrison system.
///
/// Different memory types have different persistence and access patterns.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::garrison::GarrisonType;
///
/// let memory_type = GarrisonType::ShortTerm;
/// assert_eq!(memory_type, GarrisonType::ShortTerm);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GarrisonType {
    /// Active conversation context (ephemeral)
    ShortTerm,
    /// Persisted knowledge (long-term storage)
    LongTerm,
    /// Specific event memories
    Episodic,
}

/// Strategy for evicting old entries when storage limits are reached.
///
/// Different strategies prioritize different types of information retention.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::garrison::EvictionStrategy;
///
/// let strategy = EvictionStrategy::ImportanceBased;
/// // This strategy preserves system prompts and recent messages
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvictionStrategy {
    /// First In, First Out - remove oldest entries
    FIFO,
    /// Preserve system prompts and recent messages, evict middle entries
    ImportanceBased,
    /// Always keep most recent N entries
    SlidingWindow,
}

/// Configuration for Garrison memory management behavior.
///
/// Controls how the Garrison stores, retrieves, and evicts conversation history.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::garrison::{GarrisonConfig, EvictionStrategy};
///
/// let config = GarrisonConfig::new(50, Some(2000))
///     .with_eviction_strategy(EvictionStrategy::SlidingWindow)
///     .with_preserve_recent(5);
///
/// assert_eq!(config.max_entries, 50);
/// assert_eq!(config.max_tokens, Some(2000));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarrisonConfig {
    /// Maximum number of entries to keep
    pub max_entries: usize,
    /// Maximum total tokens across all entries (None = no limit)
    pub max_tokens: Option<u32>,
    /// Strategy for evicting entries when limits are exceeded
    pub eviction_strategy: EvictionStrategy,
    /// Minimum number of recent entries to always preserve
    pub preserve_recent_count: usize,
}

impl Default for GarrisonConfig {
    /// Creates a GarrisonConfig with sensible defaults.
    ///
    /// # Default Values
    /// - `max_entries`: 100
    /// - `max_tokens`: Some(4000)
    /// - `eviction_strategy`: ImportanceBased
    /// - `preserve_recent_count`: 10
    fn default() -> Self {
        Self {
            max_entries: 100,
            max_tokens: Some(4000),
            eviction_strategy: EvictionStrategy::ImportanceBased,
            preserve_recent_count: 10,
        }
    }
}

impl GarrisonConfig {
    /// Creates a new configuration with specified limits.
    ///
    /// # Arguments
    ///
    /// * `max_entries` - Maximum number of conversation entries to store
    /// * `max_tokens` - Optional maximum token count across all entries
    ///
    /// Uses default values for eviction_strategy and preserve_recent_count.
    pub fn new(max_entries: usize, max_tokens: Option<u32>) -> Self {
        Self {
            max_entries,
            max_tokens,
            ..Default::default()
        }
    }

    /// Sets the eviction strategy for this configuration (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `strategy` - The [`EvictionStrategy`] to use when removing entries
    pub fn with_eviction_strategy(mut self, strategy: EvictionStrategy) -> Self {
        self.eviction_strategy = strategy;
        self
    }

    /// Sets the number of recent entries to preserve (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `count` - Number of recent entries to always keep, regardless of eviction strategy
    pub fn with_preserve_recent(mut self, count: usize) -> Self {
        self.preserve_recent_count = count;
        self
    }
}

/// Conversation history with automatic windowing support
///
/// Manages a collection of [`GarrisonEntry`] items with automatic eviction
/// when configured limits are exceeded.
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    entries: VecDeque<GarrisonEntry>,
    config: GarrisonConfig,
}

impl ConversationHistory {
    /// Creates a new conversation history with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The [`GarrisonConfig`] defining behavior and limits
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::garrison::{ConversationHistory, GarrisonConfig};
    ///
    /// let config = GarrisonConfig::default();
    /// let history = ConversationHistory::new(config);
    /// assert_eq!(history.len(), 0);
    /// ```
    pub fn new(config: GarrisonConfig) -> Self {
        Self {
            entries: VecDeque::new(),
            config,
        }
    }

    /// Adds an entry to the history, applying windowing if necessary
    ///
    /// This method will automatically evict old entries according to the
    /// configured eviction strategy when limits are exceeded.
    pub fn add(&mut self, entry: GarrisonEntry) {
        self.entries.push_back(entry);
        self.apply_windowing();
    }

    /// Retrieves the N most recent entries.
    ///
    /// Returns entries in chronological order (oldest first).
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of recent entries to retrieve
    pub fn get_recent(&self, limit: usize) -> Vec<&GarrisonEntry> {
        let start = self.entries.len().saturating_sub(limit);
        self.entries.range(start..).collect()
    }

    /// Returns all entries in chronological order
    pub fn get_all(&self) -> Vec<&GarrisonEntry> {
        self.entries.iter().collect()
    }

    /// Calculates the total token count across all entries
    pub fn total_tokens(&self) -> u32 {
        self.entries.iter().filter_map(|e| e.token_count).sum()
    }

    /// Returns the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if the history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Applies windowing logic based on configuration
    fn apply_windowing(&mut self) {
        // First check entry count limit
        while self.entries.len() > self.config.max_entries {
            self.evict_entry();
        }

        // Then check token limit if configured
        if let Some(max_tokens) = self.config.max_tokens {
            while self.total_tokens() > max_tokens && !self.entries.is_empty() {
                self.evict_entry();
            }
        }
    }

    /// Evicts an entry based on the configured strategy
    fn evict_entry(&mut self) {
        match self.config.eviction_strategy {
            EvictionStrategy::FIFO => {
                self.entries.pop_front();
            }
            EvictionStrategy::SlidingWindow => {
                // Always remove oldest
                self.entries.pop_front();
            }
            EvictionStrategy::ImportanceBased => {
                self.evict_importance_based();
            }
        }
    }

    /// Importance-based eviction: preserve system prompts and recent messages
    fn evict_importance_based(&mut self) {
        let total_entries = self.entries.len();
        if total_entries == 0 {
            return;
        }

        // Identify protected entries (system prompts and recent messages)
        let preserve_count = self.config.preserve_recent_count.min(total_entries);
        let recent_start_idx = total_entries.saturating_sub(preserve_count);

        // Find first non-system, non-recent entry to evict
        for i in 0..recent_start_idx {
            if self.entries[i].role != ConversationRole::System {
                self.entries.remove(i);
                return;
            }
        }

        // If all non-recent entries are system prompts, remove oldest non-system from recent
        for i in recent_start_idx..total_entries {
            if self.entries[i].role != ConversationRole::System {
                self.entries.remove(i);
                return;
            }
        }

        // Last resort: remove oldest entry even if it's a system prompt
        self.entries.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_role_serialization() {
        let role = ConversationRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");

        let deserialized: ConversationRole = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, role);
    }

    #[test]
    fn test_garrison_entry_creation() {
        let entry = GarrisonEntry::new(ConversationRole::User, "Test message".to_string());

        assert_eq!(entry.role, ConversationRole::User);
        assert_eq!(entry.content, "Test message");
        assert!(entry.token_count.is_none());
        assert!(entry.metadata.is_empty());
    }

    #[test]
    fn test_garrison_entry_validation() {
        let valid_entry = GarrisonEntry::new(ConversationRole::User, "Valid content".to_string());
        assert!(valid_entry.validate().is_ok());

        let invalid_entry = GarrisonEntry::new(ConversationRole::User, String::new());
        assert!(invalid_entry.validate().is_err());
    }

    #[test]
    fn test_garrison_entry_with_token_count() {
        let entry = GarrisonEntry::with_token_count(
            ConversationRole::Assistant,
            "Response".to_string(),
            42,
        );

        assert_eq!(entry.token_count, Some(42));
    }

    #[test]
    fn test_garrison_entry_serialization() {
        let entry =
            GarrisonEntry::with_token_count(ConversationRole::User, "Test message".to_string(), 10);

        // Serialize to JSON
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Test message\""));
        assert!(json.contains("\"token_count\":10"));

        // Deserialize back
        let deserialized: GarrisonEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role, entry.role);
        assert_eq!(deserialized.content, entry.content);
        assert_eq!(deserialized.token_count, entry.token_count);
        assert_eq!(deserialized.id, entry.id);
    }

    #[test]
    fn test_conversation_history_add_and_get() {
        let config = GarrisonConfig::default();
        let mut history = ConversationHistory::new(config);

        history.add(GarrisonEntry::new(
            ConversationRole::User,
            "First".to_string(),
        ));
        history.add(GarrisonEntry::new(
            ConversationRole::Assistant,
            "Second".to_string(),
        ));

        assert_eq!(history.len(), 2);
        let recent = history.get_recent(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_conversation_history_windowing_by_count() {
        let config = GarrisonConfig::new(3, None);
        let mut history = ConversationHistory::new(config);

        for i in 0..5 {
            history.add(GarrisonEntry::new(
                ConversationRole::User,
                format!("Message {}", i),
            ));
        }

        assert_eq!(history.len(), 3);
        let entries = history.get_all();
        assert_eq!(entries[0].content, "Message 2");
    }

    #[test]
    fn test_conversation_history_token_counting() {
        let config = GarrisonConfig::default();
        let mut history = ConversationHistory::new(config);

        history.add(GarrisonEntry::with_token_count(
            ConversationRole::User,
            "First".to_string(),
            10,
        ));
        history.add(GarrisonEntry::with_token_count(
            ConversationRole::Assistant,
            "Second".to_string(),
            20,
        ));

        assert_eq!(history.total_tokens(), 30);
    }

    #[test]
    fn test_importance_based_eviction_preserves_system() {
        let config = GarrisonConfig::new(3, None)
            .with_eviction_strategy(EvictionStrategy::ImportanceBased)
            .with_preserve_recent(1);

        let mut history = ConversationHistory::new(config);

        history.add(GarrisonEntry::new(
            ConversationRole::System,
            "System prompt".to_string(),
        ));
        history.add(GarrisonEntry::new(
            ConversationRole::User,
            "User 1".to_string(),
        ));
        history.add(GarrisonEntry::new(
            ConversationRole::User,
            "User 2".to_string(),
        ));
        history.add(GarrisonEntry::new(
            ConversationRole::User,
            "User 3".to_string(),
        ));

        // Should have 3 entries: System, User 2, User 3
        assert_eq!(history.len(), 3);
        let entries = history.get_all();
        assert_eq!(entries[0].role, ConversationRole::System);
        assert_eq!(entries[1].content, "User 2");
    }

    #[test]
    fn test_fifo_eviction() {
        let config = GarrisonConfig::new(3, None).with_eviction_strategy(EvictionStrategy::FIFO);

        let mut history = ConversationHistory::new(config);

        // Add 5 entries - FIFO should remove oldest first
        for i in 0..5 {
            history.add(GarrisonEntry::new(
                ConversationRole::User,
                format!("Message {}", i),
            ));
        }

        // Should have 3 entries: Message 2, 3, 4 (oldest 0, 1 evicted)
        assert_eq!(history.len(), 3);
        let entries = history.get_all();
        assert_eq!(entries[0].content, "Message 2");
        assert_eq!(entries[1].content, "Message 3");
        assert_eq!(entries[2].content, "Message 4");
    }

    #[test]
    fn test_empty_history_operations() {
        let config = GarrisonConfig::default();
        let history = ConversationHistory::new(config);

        // Empty history operations should not panic
        assert_eq!(history.len(), 0);
        assert_eq!(history.total_tokens(), 0);

        let recent = history.get_recent(10);
        assert!(recent.is_empty());

        let all = history.get_all();
        assert!(all.is_empty());
    }

    // RT-03 / D-17: `GarrisonEntry.is_summary` under the X-10 treatment.

    #[test]
    fn existing_constructors_default_is_summary_to_false() {
        let new_entry = GarrisonEntry::new(ConversationRole::User, "hi".to_string());
        assert!(!new_entry.is_summary);

        let with_metadata_entry = GarrisonEntry::with_metadata(
            ConversationRole::User,
            "hi".to_string(),
            HashMap::new(),
        );
        assert!(!with_metadata_entry.is_summary);

        let with_token_count_entry =
            GarrisonEntry::with_token_count(ConversationRole::User, "hi".to_string(), 5);
        assert!(!with_token_count_entry.is_summary);
    }

    #[test]
    fn summary_constructor_sets_role_and_flag() {
        let entry = GarrisonEntry::summary("condensed history".to_string());
        assert_eq!(entry.role, ConversationRole::System);
        assert!(entry.is_summary);
        assert_eq!(entry.content, "condensed history");
    }

    #[test]
    fn is_summary_defaults_on_deserialize() {
        // A JSON document written before this change had no `is_summary` key.
        let legacy_json = r#"{
            "id": "5c1a1f0a-6b7b-4f8b-9b0e-6f2b8a2f9e11",
            "role": "user",
            "content": "legacy entry",
            "timestamp": "2026-01-01T00:00:00Z",
            "metadata": {},
            "token_count": null
        }"#;
        let entry: GarrisonEntry = serde_json::from_str(legacy_json).unwrap();
        assert!(!entry.is_summary);
    }

    #[test]
    fn garrison_entry_round_trips_with_the_new_field() {
        let entry = GarrisonEntry::summary("compressed".to_string());
        assert!(entry.is_summary);

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: GarrisonEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.is_summary, entry.is_summary);
        assert_eq!(deserialized.content, entry.content);
        assert_eq!(deserialized.role, entry.role);
        assert_eq!(deserialized.id, entry.id);
    }
}
