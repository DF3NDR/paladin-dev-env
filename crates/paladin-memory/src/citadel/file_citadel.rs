//! File System Citadel Adapter
//!
//! This module provides a file system-based implementation of the CitadelPort
//! trait, persisting Paladin and Battalion states as JSON files.
//!
//! # Design
//!
//! - States are stored as individual JSON files in a configured directory
//! - File naming convention: `paladin-{uuid}.json` or `battalion-{uuid}.json`
//! - Pretty-printed JSON for human readability and debugging
//! - Directory is created automatically if it doesn't exist
//! - Atomic writes using tokio::fs async operations
//!
//! # Examples
//!
//! ```rust,no_run
//! use paladin_memory::citadel::file_citadel::FileCitadel;
//! use paladin_ports::output::citadel_port::CitadelPort;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let citadel = FileCitadel::new("./citadel")?;
//!     // Use citadel to save/load states
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use log::{info, warn};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

use paladin_core::platform::container::citadel::{
    BattalionState, PaladinState, StateSummary, StateType,
};
use paladin_core::platform::container::citadel_error::CitadelError;
use paladin_ports::output::citadel_port::CitadelPort;

/// File system-based Citadel adapter
///
/// Persists Paladin and Battalion states as JSON files in a configured directory.
/// The adapter is thread-safe and can be safely shared across async tasks.
///
/// # Examples
///
/// ```rust,no_run
/// use paladin_memory::citadel::file_citadel::FileCitadel;
///
/// let citadel = FileCitadel::new("./my-app/state")
///     .expect("Failed to create citadel");
/// ```
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct FileCitadel {
    state_dir: PathBuf,
}

impl FileCitadel {
    /// Creates a new FileCitadel adapter
    ///
    /// The state directory is created if it doesn't exist. Permissions are validated
    /// to ensure the directory is writable.
    ///
    /// # Arguments
    ///
    /// * `state_dir` - Path to the directory where state files will be stored
    ///
    /// # Returns
    ///
    /// * `Ok(FileCitadel)` - Successfully created adapter
    /// * `Err(CitadelError)` - Directory creation or validation failed
    ///
    /// # Errors
    ///
    /// - `CitadelError::DirectoryCreationFailed` - Cannot create directory
    /// - `CitadelError::PermissionDenied` - Directory is not writable
    /// - `CitadelError::InvalidDirectory` - Path is not a directory
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use paladin_memory::citadel::file_citadel::FileCitadel;
    ///
    /// let citadel = FileCitadel::new("./citadel")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(state_dir: impl Into<PathBuf>) -> Result<Self, CitadelError> {
        let state_dir = state_dir.into();

        // Validate directory
        if state_dir.exists() {
            if !state_dir.is_dir() {
                return Err(CitadelError::invalid_directory(format!(
                    "Path exists but is not a directory: {}",
                    state_dir.display()
                )));
            }

            // Check if directory is writable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&state_dir) {
                    let permissions = metadata.permissions();
                    // Check if owner has write permission (bit 7)
                    if permissions.mode() & 0o200 == 0 {
                        return Err(CitadelError::permission_denied(format!(
                            "Directory is not writable: {}",
                            state_dir.display()
                        )));
                    }
                }
            }
        } else {
            // Create directory if it doesn't exist
            std::fs::create_dir_all(&state_dir).map_err(|e| {
                CitadelError::directory_creation_failed(format!(
                    "Failed to create state directory {}: {}",
                    state_dir.display(),
                    e
                ))
            })?;
            info!("Created state directory: {}", state_dir.display());
        }

        Ok(Self { state_dir })
    }

    /// Generates the file path for a Paladin state
    fn paladin_path(&self, id: Uuid) -> PathBuf {
        self.state_dir.join(format!("paladin-{}.json", id))
    }

    /// Generates the file path for a Battalion state
    fn battalion_path(&self, id: Uuid) -> PathBuf {
        self.state_dir.join(format!("battalion-{}.json", id))
    }

    /// Parses a file name to extract UUID and state type
    fn parse_filename(filename: &str) -> Option<(Uuid, StateType)> {
        if let Some(stripped) = filename.strip_prefix("paladin-") {
            if let Some(uuid_str) = stripped.strip_suffix(".json")
                && let Ok(uuid) = Uuid::parse_str(uuid_str)
            {
                return Some((uuid, StateType::Paladin));
            }
        } else if let Some(stripped) = filename.strip_prefix("battalion-")
            && let Some(uuid_str) = stripped.strip_suffix(".json")
            && let Ok(uuid) = Uuid::parse_str(uuid_str)
        {
            return Some((uuid, StateType::Battalion));
        }
        None
    }
}

#[async_trait]
impl CitadelPort for FileCitadel {
    async fn save_paladin(&self, state: &PaladinState) -> Result<(), CitadelError> {
        let path = self.paladin_path(state.paladin.uuid);

        // Serialize to pretty JSON for human readability
        let json = serde_json::to_string_pretty(state)?;

        // Write to file atomically
        fs::write(&path, json).await.map_err(|e| {
            CitadelError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to write Paladin state to {}: {}", path.display(), e),
            ))
        })?;

        info!(
            "Saved Paladin state: {} to {}",
            state.paladin.uuid,
            path.display()
        );

        Ok(())
    }

    async fn load_paladin(&self, id: Uuid) -> Result<Option<PaladinState>, CitadelError> {
        let path = self.paladin_path(id);

        // Check if file exists
        if !path.exists() {
            return Ok(None);
        }

        // Read file
        let json = fs::read_to_string(&path).await.map_err(|e| {
            CitadelError::IoError(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to read Paladin state from {}: {}",
                    path.display(),
                    e
                ),
            ))
        })?;

        // Deserialize
        let state: PaladinState = serde_json::from_str(&json).map_err(|e| {
            CitadelError::corrupted(format!(
                "Failed to parse Paladin state from {}: {}",
                path.display(),
                e
            ))
        })?;

        // Validate schema version (currently just log warning for MVP)
        if state.schema_version != "1.0.0" {
            warn!(
                "Loading Paladin state with schema version {} (expected 1.0.0)",
                state.schema_version
            );
        }

        info!("Loaded Paladin state: {} from {}", id, path.display());

        Ok(Some(state))
    }

    async fn save_battalion(&self, state: &BattalionState) -> Result<(), CitadelError> {
        let path = self.battalion_path(state.id);

        // Serialize to pretty JSON
        let json = serde_json::to_string_pretty(state)?;

        // Write to file
        fs::write(&path, json).await.map_err(|e| {
            CitadelError::IoError(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to write Battalion state to {}: {}",
                    path.display(),
                    e
                ),
            ))
        })?;

        info!("Saved Battalion state: {} to {}", state.id, path.display());

        Ok(())
    }

    async fn load_battalion(&self, id: Uuid) -> Result<Option<BattalionState>, CitadelError> {
        let path = self.battalion_path(id);

        // Check if file exists
        if !path.exists() {
            return Ok(None);
        }

        // Read file
        let json = fs::read_to_string(&path).await.map_err(|e| {
            CitadelError::IoError(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to read Battalion state from {}: {}",
                    path.display(),
                    e
                ),
            ))
        })?;

        // Deserialize
        let state: BattalionState = serde_json::from_str(&json).map_err(|e| {
            CitadelError::corrupted(format!(
                "Failed to parse Battalion state from {}: {}",
                path.display(),
                e
            ))
        })?;

        // Validate schema version
        if state.schema_version != "1.0.0" {
            warn!(
                "Loading Battalion state with schema version {} (expected 1.0.0)",
                state.schema_version
            );
        }

        info!("Loaded Battalion state: {} from {}", id, path.display());

        Ok(Some(state))
    }

    async fn list_saved(&self) -> Result<Vec<StateSummary>, CitadelError> {
        let mut summaries = Vec::new();

        // Read directory entries
        let mut entries = fs::read_dir(&self.state_dir).await.map_err(|e| {
            CitadelError::IoError(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to read state directory {}: {}",
                    self.state_dir.display(),
                    e
                ),
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            CitadelError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read directory entry: {}", e),
            ))
        })? {
            let path = entry.path();

            // Skip non-files
            if !path.is_file() {
                continue;
            }

            // Parse filename
            if let Some(filename) = path.file_name().and_then(|n| n.to_str())
                && let Some((id, state_type)) = Self::parse_filename(filename)
            {
                // Get file metadata for timestamps
                if let Ok(metadata) = fs::metadata(&path).await
                    && let Ok(created) = metadata.created()
                    && let Ok(modified) = metadata.modified()
                {
                    summaries.push(StateSummary {
                        id,
                        state_type,
                        created_at: created.into(),
                        updated_at: modified.into(),
                        file_path: path.clone(),
                    });
                }
            }
        }

        info!("Listed {} saved states", summaries.len());

        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::base::entity::Entity;
    use paladin_core::platform::container::citadel::{BattalionCheckpointConfig, CheckpointData};
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
    use tempfile::TempDir;

    fn create_test_paladin_state() -> PaladinState {
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
        PaladinState::new(paladin, vec![], vec![])
    }

    #[test]
    fn test_file_citadel_creation() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path());
        assert!(citadel.is_ok());
    }

    #[test]
    fn test_file_citadel_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join("new_dir");
        assert!(!state_dir.exists());

        let citadel = FileCitadel::new(&state_dir);
        assert!(citadel.is_ok());
        assert!(state_dir.exists());
        assert!(state_dir.is_dir());
    }

    #[test]
    fn test_file_citadel_rejects_file_as_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("notadir");
        std::fs::write(&file_path, "test").unwrap();

        let citadel = FileCitadel::new(&file_path);
        assert!(citadel.is_err());
        assert!(matches!(
            citadel.unwrap_err(),
            CitadelError::InvalidDirectory(_)
        ));
    }

    #[test]
    fn test_paladin_path_generation() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();
        let id = Uuid::new_v4();
        let path = citadel.paladin_path(id);
        assert_eq!(path, temp_dir.path().join(format!("paladin-{}.json", id)));
    }

    #[test]
    fn test_battalion_path_generation() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();
        let id = Uuid::new_v4();
        let path = citadel.battalion_path(id);
        assert_eq!(path, temp_dir.path().join(format!("battalion-{}.json", id)));
    }

    #[test]
    fn test_parse_filename_paladin() {
        let id = Uuid::new_v4();
        let filename = format!("paladin-{}.json", id);
        let result = FileCitadel::parse_filename(&filename);
        assert!(result.is_some());
        let (parsed_id, state_type) = result.unwrap();
        assert_eq!(parsed_id, id);
        assert_eq!(state_type, StateType::Paladin);
    }

    #[test]
    fn test_parse_filename_battalion() {
        let id = Uuid::new_v4();
        let filename = format!("battalion-{}.json", id);
        let result = FileCitadel::parse_filename(&filename);
        assert!(result.is_some());
        let (parsed_id, state_type) = result.unwrap();
        assert_eq!(parsed_id, id);
        assert_eq!(state_type, StateType::Battalion);
    }

    #[test]
    fn test_parse_filename_invalid() {
        assert!(FileCitadel::parse_filename("invalid.json").is_none());
        assert!(FileCitadel::parse_filename("paladin-notauuid.json").is_none());
        assert!(FileCitadel::parse_filename("paladin-123.txt").is_none());
    }

    /// Compatibility check for the base-entity rename (ADR-0060): the bytes a
    /// v0.10 Citadel wrote -- captured from the unrenamed tree and never
    /// hand-edited -- load through the real `FileCitadel` adapter into a
    /// `PaladinState` and save back to the same JSON document, apart from the
    /// renamed entity key (`paladin.node` is written as `paladin.data`).
    #[tokio::test]
    async fn pre_rename_citadel_snapshot_loads_through_file_citadel()
    -> Result<(), Box<dyn std::error::Error>> {
        // Runtime read (never `include_str!`) so crate packaging is unaffected.
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../paladin-core/tests/fixtures/pre_rename/citadel.json");
        // The fixture lives in the sibling `paladin-core` crate. A packaged
        // `paladin-memory` tarball or a build context that copies only this
        // crate does not have it, so skip loudly instead of failing on I/O.
        if !fixture_path.exists() {
            eprintln!(
                "skipping pre_rename_citadel_snapshot_loads_through_file_citadel: \
                 fixture {} is absent (sibling paladin-core crate not present)",
                fixture_path.display()
            );
            return Ok(());
        }
        let fixture_bytes = std::fs::read(&fixture_path)?;
        let fixture_value: serde_json::Value = serde_json::from_slice(&fixture_bytes)?;

        let id_text = fixture_value["paladin"]["uuid"]
            .as_str()
            .ok_or("fixture has no paladin.uuid string")?;
        let id = Uuid::parse_str(id_text)?;

        // The fixture bytes are copied unchanged into the Citadel's state dir.
        let temp_dir = TempDir::new()?;
        std::fs::write(
            temp_dir.path().join(format!("paladin-{id}.json")),
            &fixture_bytes,
        )?;

        let citadel = FileCitadel::new(temp_dir.path())?;
        let state = citadel
            .load_paladin(id)
            .await?
            .ok_or("pre-rename snapshot was not found by load_paladin")?;

        assert_eq!(state.paladin.uuid, id);
        assert_eq!(state.paladin.name.as_deref(), Some("fixture"));
        assert_eq!(state.schema_version, "1.0.0");

        // Saving overwrites the same path; the written document must equal the
        // fixture (`serde_json::Value` equality ignores key order).
        citadel.save_paladin(&state).await?;
        let written = std::fs::read_to_string(temp_dir.path().join(format!("paladin-{id}.json")))?;
        let written_value: serde_json::Value = serde_json::from_str(&written)?;

        // The only wire change is the entity-sense key: the fixture's v0.10 `node` key under `paladin` is
        // written as `paladin.data` (D-03), and no old key is emitted.
        let mut expected = fixture_value;
        let paladin = expected
            .pointer_mut("/paladin")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("fixture has no paladin object")?;
        let payload = paladin
            .remove("node")
            .ok_or("fixture paladin has no v0.10 node key")?;
        paladin.insert("data".to_owned(), payload);
        assert_eq!(written_value, expected);
        let written_paladin = written_value
            .pointer("/paladin")
            .and_then(serde_json::Value::as_object)
            .ok_or("written snapshot has no paladin object")?;
        assert!(written_paladin.contains_key("data"));
        assert!(!written_paladin.contains_key("node"));
        Ok(())
    }

    #[tokio::test]
    async fn test_save_and_load_paladin() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();

        let state = create_test_paladin_state();
        let id = state.paladin.uuid;

        // Save
        citadel.save_paladin(&state).await.unwrap();

        // Verify file exists
        let path = citadel.paladin_path(id);
        assert!(path.exists());

        // Load
        let loaded = citadel.load_paladin(id).await.unwrap();
        assert!(loaded.is_some());
        let loaded_state = loaded.unwrap();
        assert_eq!(loaded_state.paladin.uuid, id);
        assert_eq!(loaded_state.schema_version, "1.0.0");
    }

    #[tokio::test]
    async fn test_load_nonexistent_paladin() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();

        let result = citadel.load_paladin(Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_save_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();

        let state = create_test_paladin_state();

        // Save twice
        citadel.save_paladin(&state).await.unwrap();
        citadel.save_paladin(&state).await.unwrap();

        // Should only be one file
        let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_save_and_load_battalion() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();

        let state = BattalionState::new(
            "Formation",
            BattalionCheckpointConfig::default(),
            vec![],
            Some(CheckpointData::new()),
        );
        let id = state.id;

        // Save
        citadel.save_battalion(&state).await.unwrap();

        // Load
        let loaded = citadel.load_battalion(id).await.unwrap();
        assert!(loaded.is_some());
        let loaded_state = loaded.unwrap();
        assert_eq!(loaded_state.id, id);
        assert_eq!(loaded_state.battalion_type, "Formation");
    }

    #[tokio::test]
    async fn test_list_saved_empty() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();

        let summaries = citadel.list_saved().await.unwrap();
        assert_eq!(summaries.len(), 0);
    }

    #[tokio::test]
    async fn test_list_saved_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();

        // Save multiple states
        let state1 = create_test_paladin_state();
        let state2 = create_test_paladin_state();
        let battalion = BattalionState::new(
            "Phalanx",
            BattalionCheckpointConfig::default(),
            vec![],
            None,
        );

        citadel.save_paladin(&state1).await.unwrap();
        citadel.save_paladin(&state2).await.unwrap();
        citadel.save_battalion(&battalion).await.unwrap();

        // List
        let summaries = citadel.list_saved().await.unwrap();
        assert_eq!(summaries.len(), 3);

        // Check types
        let paladin_count = summaries
            .iter()
            .filter(|s| s.state_type == StateType::Paladin)
            .count();
        let battalion_count = summaries
            .iter()
            .filter(|s| s.state_type == StateType::Battalion)
            .count();
        assert_eq!(paladin_count, 2);
        assert_eq!(battalion_count, 1);
    }

    #[tokio::test]
    async fn test_json_is_human_readable() {
        let temp_dir = TempDir::new().unwrap();
        let citadel = FileCitadel::new(temp_dir.path()).unwrap();

        let state = create_test_paladin_state();
        let id = state.paladin.uuid;

        citadel.save_paladin(&state).await.unwrap();

        // Read file and verify it's pretty-printed
        let path = citadel.paladin_path(id);
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains('\n')); // Multi-line
        assert!(contents.contains("  ")); // Indented
        assert!(contents.contains("\"schema_version\""));
    }
}
