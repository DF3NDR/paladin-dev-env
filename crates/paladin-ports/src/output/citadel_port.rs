//! # Citadel Port - State Persistence and Recovery Interface
//!
//! This module defines the port trait for the Citadel state persistence system,
//! enabling Paladins and Battalions to save and restore their complete execution
//! state across sessions, crashes, and interruptions.
//!
//! ## Purpose
//!
//! The Citadel port provides a standardized interface for:
//! - **State Persistence**: Saving Paladin and Battalion state to durable storage
//! - **State Recovery**: Loading previously saved states by ID
//! - **State Discovery**: Listing all saved states with summary information
//! - **Checkpoint Management**: Supporting resumable Battalion orchestrations
//!
//! Following hexagonal architecture, this trait abstracts state operations
//! from their implementations (file system, database, cloud storage).
//!
//! ## Hexagonal Architecture Context
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │               Application Layer                      │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  PaladinExecutionService                      │  │
//! │  │    - Saves state after execution              │  │
//! │  │    - Loads state for resumption               │  │
//! │  │  BattalionServices                            │  │
//! │  │    - Checkpoint management                    │  │
//! │  │    - Recovery from failures                   │  │
//! │  └──────────────────────────────────────────────┘  │
//! │                         │                            │
//! │                         ▼                            │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  CitadelPort (this module)                    │  │
//! │  │    - State persistence interface              │  │
//! │  └──────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//!                          │
//!                          ▼
//! ┌─────────────────────────────────────────────────────┐
//! │            Infrastructure Layer                      │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  FileCitadel (Local JSON files)              │  │
//! │  │  SqliteCitadel (SQLite database)             │  │
//! │  │  S3Citadel (AWS S3 bucket)                   │  │
//! │  │  RedisCitadel (Redis cache)                  │  │
//! │  └──────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync`:
//! - **Send**: States may be saved/loaded from different threads
//! - **Sync**: Multiple Paladins may access the Citadel concurrently
//! - Implementations must handle concurrent save/load operations safely
//!
//! ## Error Handling
//!
//! State persistence can fail for several reasons:
//! - **State Not Found**: Requested state ID doesn't exist
//! - **Corrupted State**: File/data contains invalid JSON or structure
//! - **Incompatible Version**: State schema version doesn't match
//! - **I/O Error**: File system or network error
//! - **Permission Denied**: Insufficient permissions for storage access
//!
//! All errors are represented via [`CitadelError`](paladin_core::platform::container::citadel_error::CitadelError)
//! with detailed context for debugging and recovery.
//!
//! ## Common Use Cases
//!
//! ### 1. Automatic State Saving
//!
//! ```rust,no_run
//! use paladin_ports::output::citadel_port::CitadelPort;
//! use paladin_core::platform::container::citadel::PaladinState;
//!
//! async fn autosave_paladin_state(
//!     citadel: &dyn CitadelPort,
//!     state: &PaladinState,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Save state after execution
//!     citadel.save_paladin(state).await?;
//!
//!     println!("Paladin state saved: {}", state.paladin.uuid);
//!     println!("  Garrison entries: {}", state.garrison.len());
//!     println!("  Executions: {}", state.execution_history.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 2. State Recovery and Resumption
//!
//! ```rust,no_run
//! use paladin_ports::output::citadel_port::CitadelPort;
//! use uuid::Uuid;
//!
//! async fn restore_paladin(
//!     citadel: &dyn CitadelPort,
//!     paladin_id: Uuid,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Attempt to load saved state
//!     if let Some(state) = citadel.load_paladin(paladin_id).await? {
//!         println!("Restored Paladin: {}", state.paladin.data.name);
//!         println!("  Created: {}", state.created_at);
//!         println!("  Last updated: {}", state.updated_at);
//!         println!("  Garrison size: {}", state.garrison.len());
//!
//!         // Resume from saved state...
//!     } else {
//!         println!("No saved state found for ID: {}", paladin_id);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 3. Battalion Checkpoint Recovery
//!
//! ```rust,no_run
//! use paladin_ports::output::citadel_port::CitadelPort;
//! use uuid::Uuid;
//!
//! async fn resume_battalion_execution(
//!     citadel: &dyn CitadelPort,
//!     battalion_id: Uuid,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Load Battalion checkpoint
//!     if let Some(state) = citadel.load_battalion(battalion_id).await? {
//!         println!("Resuming Battalion: {}", state.battalion_type);
//!         println!("  Paladins: {}", state.paladin_states.len());
//!
//!         if let Some(checkpoint) = &state.checkpoint {
//!             println!("  Last completed: {:?}", checkpoint.last_completed_index);
//!             println!("  Completed Paladins: {:?}", checkpoint.completed_paladins);
//!             // Resume from checkpoint...
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ```rust,no_run
//! use paladin_ports::output::citadel_port::CitadelPort;
//!
//! async fn list_all_saved_states(
//!     citadel: &dyn CitadelPort,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     let states = citadel.list_saved().await?;
//!
//!     println!("Found {} saved states:", states.len());
//!     for summary in states {
//!         println!("  • {:?} at {}", summary.state_type, summary.file_path.display());
//!         println!("    ID: {}", summary.id);
//!         println!("    Created: {}", summary.created_at);
//!         println!("    Updated: {}", summary.updated_at);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Implementation Notes
///
/// ### Storage Backend Selection
///
/// Choose based on deployment requirements:
///
/// - **FileCitadel** (Local JSON): Best for development, single-instance deployments
/// - **SqliteCitadel** (SQLite DB): Better query capabilities, atomic operations
/// - **S3Citadel** (AWS S3): Cloud-native, scalable, cross-region replication
/// - **RedisCitadel** (Redis): Fast, distributed, expirable states
///
/// ```rust,ignore
/// // Local file storage
/// let citadel = FileCitadel::new("./paladin-states")?;
///
/// // SQLite database
/// let citadel = SqliteCitadel::new("paladin-states.db").await?;
///
/// // AWS S3 bucket
/// let citadel = S3Citadel::new("my-bucket", "paladin-states/").await?;
///
/// // Redis cache
/// let citadel = RedisCitadel::new("redis://localhost").await?;
/// ```
///
/// ### State Schema Versioning
///
/// Handle schema evolution gracefully:
///
/// ```rust,ignore
/// // Check schema version on load
/// if let Some(state) = citadel.load_paladin(id).await? {
///     match state.schema_version.as_str() {
///         "1.0.0" => { /* current version */ },
///         "0.9.0" => { /* migrate from old version */ },
///         _ => {
///             return Err(CitadelError::incompatible("1.0.0", &state.schema_version));
///         }
///     }
/// }
/// ```
///
/// ### Autosave Strategy
///
/// Configure when states are saved:
///
/// 1. **After Each Execution**: Most durable, higher I/O cost
/// 2. **Periodic Checkpoints**: Balanced, configurable interval
/// 3. **Manual Triggers**: Fine-grained control, requires discipline
/// 4. **On Shutdown**: Least durable, minimal overhead
///
/// ```rust,ignore
/// // After each execution (autosave)
/// paladin_builder.enable_autosave();
///
/// // Periodic (every N seconds)
/// let interval = Duration::from_secs(30);
/// tokio::spawn(async move {
///     loop {
///         tokio::time::sleep(interval).await;
///         citadel.save_paladin(&state).await?;
///     }
/// });
///
/// // Manual trigger
/// if important_milestone {
///     citadel.save_paladin(&state).await?;
/// }
/// ```
///
/// ### Performance Considerations
///
/// 1. **Async I/O**: Use async file/DB operations to avoid blocking
/// 2. **Batch Operations**: Save multiple states in one transaction
/// 3. **Compression**: Compress JSON for large states (gzip, zstd)
/// 4. **Incremental Saves**: Only save changed data (delta encoding)
/// 5. **Background Saving**: Don't block execution on save
///
/// ### Best Practices
///
/// 1. **Validate Before Save**: Ensure state is serializable
/// 2. **Handle Schema Changes**: Version states, support migration
/// 3. **Cleanup Old States**: Implement retention policies
/// 4. **Monitor Storage**: Alert on disk/quota exhaustion
/// 5. **Test Recovery**: Regularly verify saved states load correctly
///
/// ## Common Pitfalls
///
/// - Not handling missing states gracefully (assume fresh start)
/// - Blocking execution thread during save (use async)
/// - Missing schema version compatibility checks
/// - Not cleaning up old states (disk space exhaustion)
/// - Saving circular references (serialization errors)
/// - Not validating restored states (corrupted data propagation)
///
/// ## Related Modules
///
/// - [`PaladinState`](paladin_core::platform::container::citadel::PaladinState) - Paladin state structure
/// - [`BattalionState`](paladin_core::platform::container::citadel::BattalionState) - Battalion state structure
/// - [`StateSummary`](paladin_core::platform::container::citadel::StateSummary) - State metadata
/// - [`CitadelError`](paladin_core::platform::container::citadel_error::CitadelError) - Error types
/// - [`GarrisonPort`](crate::output::garrison_port::GarrisonPort) - Memory storage
///
/// ## See Also
///
/// - [CITADEL.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/docs/CITADEL.md) - Comprehensive Citadel guide
/// - `examples/citadel_autosave.rs` - Automatic state saving example
/// - `examples/citadel_restore.rs` - State restoration example
/// - `examples/battalion_checkpoint_recovery.rs` - Battalion recovery example
use async_trait::async_trait;
use uuid::Uuid;

use paladin_core::platform::container::citadel::{BattalionState, PaladinState, StateSummary};
use paladin_core::platform::container::citadel_error::CitadelError;

/// Port trait for Citadel state persistence operations.
///
/// Provides the interface for saving, loading, and discovering Paladin and
/// Battalion execution states. Implementations handle storage backend specifics
/// (file system, database, cloud storage) while providing consistent semantics.
///
/// # Capabilities
///
/// - **Paladin Persistence**: Save/load individual Paladin states with [`save_paladin`](Self::save_paladin) / [`load_paladin`](Self::load_paladin)
/// - **Battalion Persistence**: Save/load Battalion orchestration states with [`save_battalion`](Self::save_battalion) / [`load_battalion`](Self::load_battalion)
/// - **State Discovery**: List all saved states with metadata via [`list_saved`](Self::list_saved)
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support:
/// - Concurrent state saves from multiple Paladins
/// - Safe loading during async execution
/// - Background autosave operations
///
/// # Implementation Requirements
///
/// Implementations should:
/// 1. Use atomic operations for save (no partial writes)
/// 2. Validate schema version on load (reject incompatible states)
/// 3. Create storage location if it doesn't exist
/// 4. Handle concurrent access safely (file locking, DB transactions)
/// 5. Return `None` for missing states (not an error)
///
/// # Examples
///
/// ## Save and Load Paladin State
///
/// ```rust,no_run
/// use paladin_ports::output::citadel_port::CitadelPort;
/// use paladin_core::platform::container::citadel::PaladinState;
/// use paladin_core::base::entity::Entity;
/// use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
///
/// async fn save_and_load(
///     citadel: &dyn CitadelPort,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     // Create a Paladin state
///     let paladin_data = PaladinData {
///         system_prompt: "Test prompt".to_string(),
///         name: "TestPaladin".to_string(),
///         user_name: "user".to_string(),
///         model: "gpt-4".to_string(),
///         temperature: 0.7,
///         max_loops: MaxLoops::Fixed(3),
///         stop_words: vec![],
///         status: PaladinStatus::Idle,
///         vision_enabled: false,
///         ..Default::default()
///     };
///
///     let paladin = Entity::new(paladin_data, Some("test".to_string()));
///     let state = PaladinState::new(paladin.clone(), vec![], vec![]);
///
///     // Save the state
///     citadel.save_paladin(&state).await?;
///     println!("Saved Paladin state: {}", paladin.uuid);
///
///     // Load it back
///     if let Some(loaded) = citadel.load_paladin(paladin.uuid).await? {
///         println!("Loaded: {}", loaded.paladin.data.name);
///         println!("  Created: {}", loaded.created_at);
///         println!("  Schema: v{}", loaded.schema_version);
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Battalion Checkpoint Management
///
/// ```rust,no_run
/// use paladin_ports::output::citadel_port::CitadelPort;
/// use paladin_core::platform::container::citadel::{BattalionState, BattalionCheckpointConfig, CheckpointData};
/// use uuid::Uuid;
///
/// async fn battalion_checkpoint(
///     citadel: &dyn CitadelPort,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     // Create Battalion state with checkpoint
///     let mut checkpoint = CheckpointData::new();
///     checkpoint.last_completed_index = Some(4);
///     checkpoint.completed_paladins = vec![Uuid::new_v4(), Uuid::new_v4()];
///
///     let battalion_state = BattalionState::new(
///         "Formation",
///         BattalionCheckpointConfig::default(),
///         vec![],  // paladin_states would be here
///         Some(checkpoint),
///     );
///
///     // Save checkpoint
///     citadel.save_battalion(&battalion_state).await?;
///     println!("Battalion checkpoint saved: {}", battalion_state.id);
///
///     // Resume from checkpoint
///     if let Some(loaded) = citadel.load_battalion(battalion_state.id).await? {
///         if let Some(cp) = loaded.checkpoint {
///             println!("Resuming from index {:?}", cp.last_completed_index);
///             println!("Completed: {} Paladins", cp.completed_paladins.len());
///         }
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Listing Saved States
///
/// ```rust,no_run
/// use paladin_ports::output::citadel_port::CitadelPort;
///
/// async fn inspect_states(
///     citadel: &dyn CitadelPort,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     let states = citadel.list_saved().await?;
///
///     println!("Found {} saved states:", states.len());
///
///     for summary in states {
///         println!("\\n{:?} ({})", summary.state_type, summary.id);
///         println!("  Created: {}", summary.created_at);
///         println!("  Updated: {}", summary.updated_at);
///         println!("  Path: {}", summary.file_path.display());
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Custom Implementation Example
///
/// ```rust
/// use paladin_ports::output::citadel_port::CitadelPort;
/// use paladin_core::platform::container::citadel_error::CitadelError;
/// use paladin_core::platform::container::citadel::{PaladinState, BattalionState, StateSummary};
/// use async_trait::async_trait;
/// use uuid::Uuid;
/// use std::collections::HashMap;
/// use std::sync::{Arc, RwLock};
///
/// struct InMemoryCitadel {
///     paladin_states: Arc<RwLock<HashMap<Uuid, PaladinState>>>,
///     battalion_states: Arc<RwLock<HashMap<Uuid, BattalionState>>>,
/// }
///
/// impl InMemoryCitadel {
///     fn new() -> Self {
///         Self {
///             paladin_states: Arc::new(RwLock::new(HashMap::new())),
///             battalion_states: Arc::new(RwLock::new(HashMap::new())),
///         }
///     }
/// }
///
/// #[async_trait]
/// impl CitadelPort for InMemoryCitadel {
///     async fn save_paladin(&self, state: &PaladinState) -> Result<(), CitadelError> {
///         let mut states = self.paladin_states.write().unwrap();
///         states.insert(state.paladin.uuid, state.clone());
///         Ok(())
///     }
///
///     async fn load_paladin(&self, id: Uuid) -> Result<Option<PaladinState>, CitadelError> {
///         let states = self.paladin_states.read().unwrap();
///         Ok(states.get(&id).cloned())
///     }
///
///     async fn save_battalion(&self, state: &BattalionState) -> Result<(), CitadelError> {
///         let mut states = self.battalion_states.write().unwrap();
///         states.insert(state.id, state.clone());
///         Ok(())
///     }
///
///     async fn load_battalion(&self, id: Uuid) -> Result<Option<BattalionState>, CitadelError> {
///         let states = self.battalion_states.read().unwrap();
///         Ok(states.get(&id).cloned())
///     }
///
///     async fn list_saved(&self) -> Result<Vec<StateSummary>, CitadelError> {
///         // Return empty for this simple example
///         Ok(vec![])
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Storage Strategies
///
/// Different backends suit different requirements:
///
/// ```rust,ignore
/// // File-based (simple, human-readable)
/// let citadel = FileCitadel::new("./states")?;
/// citadel.save_paladin(&state).await?;
/// // Result: ./states/<uuid>.json
///
/// // SQLite (transactional, queryable)
/// let citadel = SqliteCitadel::new("states.db").await?;
/// citadel.save_paladin(&state).await?;
/// // Result: INSERT INTO paladin_states...
///
/// // S3 (cloud-native, scalable)
/// let citadel = S3Citadel::new("my-bucket", "states/").await?;
/// citadel.save_paladin(&state).await?;
/// // Result: s3://my-bucket/states/<uuid>.json
/// ```
///
/// ## Atomicity Guarantees
///
/// Ensure no partial writes:
///
/// ```rust,ignore
/// // File-based: Write to temp file, then atomic rename
/// let temp_file = format!("{}.tmp", state_file);
/// fs::write(&temp_file, json)?;
/// fs::rename(&temp_file, &state_file)?;  // Atomic on POSIX
///
/// // Database: Use transactions
/// let mut tx = db.begin().await?;
/// tx.execute("INSERT OR REPLACE INTO...", &[state])?;
/// tx.commit().await?;  // All-or-nothing
/// ```
///
/// ## Schema Evolution
///
/// Handle version compatibility:
///
/// ```rust,ignore
/// match state.schema_version.as_str() {
///     "1.0.0" => { /* current version */ },
///     "0.9.0" => {
///         // Migrate old schema
///         state_v1 = migrate_from_v09(state_v09)?;
///     },
///     _ => {
///         return Err(CitadelError::incompatible("1.0.0", &state.schema_version));
///     }
/// }
/// ```
///
/// ## Performance Optimization
///
/// 1. **Compression**: Compress JSON to reduce storage (gzip, zstd)
/// 2. **Background Saves**: Don't block execution on save
/// 3. **Batch Operations**: Save multiple states in one transaction
/// 4. **Caching**: Cache recently loaded states in memory
/// 5. **Lazy Loading**: Load full state only when needed
///
/// ```rust,ignore
/// // Compression example
/// use flate2::write::GzEncoder;
/// let json = serde_json::to_string(&state)?;
/// let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
/// encoder.write_all(json.as_bytes())?;
/// let compressed = encoder.finish()?;
/// ```
///
/// ## Error Handling Strategies
///
/// - **State Not Found**: Normal case, return `None` (not error)
/// - **Corrupted State**: Log warning, return error with context
/// - **I/O Error**: Retry with exponential backoff (transient failures)
/// - **Permission Denied**: Fail fast, alert operator
///
/// # Common Pitfalls
///
/// - Blocking execution thread during save (use spawn for async save)
/// - Not validating schema version on load (compatibility issues)
/// - Saving sensitive data without encryption (security risk)
/// - No retention policy (disk space exhaustion)
/// - Missing error context (hard to debug failures)
/// - Not testing state restoration (find issues in production)
///
/// # See Also
///
/// - [`PaladinState`](paladin_core::platform::container::citadel::PaladinState) - Paladin state structure
/// - [`BattalionState`](paladin_core::platform::container::citadel::BattalionState) - Battalion state structure
/// - [`StateSummary`](paladin_core::platform::container::citadel::StateSummary) - State metadata
/// - [`CitadelError`](paladin_core::platform::container::citadel_error::CitadelError) - Error types
#[async_trait]
pub trait CitadelPort: Send + Sync {
    /// Saves a Paladin state to persistent storage
    ///
    /// Overwrites any existing state with the same Paladin ID.
    async fn save_paladin(&self, state: &PaladinState) -> Result<(), CitadelError>;

    /// Loads a Paladin state from persistent storage
    ///
    /// Returns `None` if no state exists for the given ID.
    async fn load_paladin(&self, id: Uuid) -> Result<Option<PaladinState>, CitadelError>;

    /// Saves a Battalion state to persistent storage
    async fn save_battalion(&self, state: &BattalionState) -> Result<(), CitadelError>;

    /// Loads a Battalion state from persistent storage
    async fn load_battalion(&self, id: Uuid) -> Result<Option<BattalionState>, CitadelError>;

    /// Lists all saved states with summary information
    async fn list_saved(&self) -> Result<Vec<StateSummary>, CitadelError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::base::entity::Entity;
    use paladin_core::platform::container::citadel::{BattalionCheckpointConfig, CheckpointData};
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};

    // Mock implementation for testing trait bounds
    struct MockCitadel;

    #[async_trait]
    impl CitadelPort for MockCitadel {
        async fn save_paladin(&self, _state: &PaladinState) -> Result<(), CitadelError> {
            Ok(())
        }

        async fn load_paladin(&self, _id: Uuid) -> Result<Option<PaladinState>, CitadelError> {
            Ok(None)
        }

        async fn save_battalion(&self, _state: &BattalionState) -> Result<(), CitadelError> {
            Ok(())
        }

        async fn load_battalion(&self, _id: Uuid) -> Result<Option<BattalionState>, CitadelError> {
            Ok(None)
        }

        async fn list_saved(&self) -> Result<Vec<StateSummary>, CitadelError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_mock_citadel_implements_trait() {
        let citadel = MockCitadel;

        let paladin_data = PaladinData {
            system_prompt: "test".to_string(),
            name: "test".to_string(),
            user_name: "test".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(3),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };
        let paladin = Entity::new(paladin_data, Some("test".to_string()));
        let state = PaladinState::new(paladin, vec![], vec![]);

        assert!(citadel.save_paladin(&state).await.is_ok());
        assert!(citadel.load_paladin(Uuid::new_v4()).await.is_ok());

        let battalion_state = BattalionState::new(
            "Formation",
            BattalionCheckpointConfig::default(),
            vec![],
            Some(CheckpointData::new()),
        );

        assert!(citadel.save_battalion(&battalion_state).await.is_ok());
        assert!(citadel.load_battalion(Uuid::new_v4()).await.is_ok());
        assert!(citadel.list_saved().await.is_ok());
    }

    #[test]
    fn test_trait_is_object_safe() {
        // This test verifies that CitadelPort can be used as a trait object
        let _: Option<Box<dyn CitadelPort>> = None;
    }
}
