//! Integration tests for Citadel file persistence
//!
//! These tests verify the FileCitadel adapter's ability to save and restore
//! Paladin and Battalion states to/from the file system.

use paladin::application::errors::citadel_error::CitadelError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::core::base::entity::Entity;
use paladin::core::platform::container::citadel::{
    BattalionCheckpointConfig, BattalionState, CheckpointData, ExecutionRecord, ExecutionStatus,
    PaladinState,
};
use paladin::core::platform::container::garrison::{ConversationRole, GarrisonEntry};
use paladin::core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
use paladin::infrastructure::adapters::citadel::file_citadel::FileCitadel;
use paladin_ports::output::citadel_port::CitadelPort;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

/// Helper function to create a temporary test directory
fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Helper to create test PaladinData
fn create_test_paladin_data() -> PaladinData {
    PaladinData {
        system_prompt: "You are a helpful assistant".to_string(),
        name: "TestPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec!["STOP".to_string()],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    }
}

/// Helper to create test Paladin node
fn create_test_paladin() -> Entity<PaladinData> {
    Entity::new(create_test_paladin_data(), Some("TestPaladin".to_string()))
}

/// Helper to create test PaladinState
fn create_test_paladin_state() -> PaladinState {
    let paladin = create_test_paladin();
    let garrison = vec![GarrisonEntry::new(
        ConversationRole::User,
        "Hello, Paladin!".to_string(),
    )];
    let history = vec![ExecutionRecord {
        timestamp: chrono::Utc::now(),
        input: "test input".to_string(),
        output: "test output".to_string(),
        status: ExecutionStatus::Success,
        loops_used: 1,
    }];

    PaladinState::new(paladin, garrison, history)
}

#[tokio::test]
async fn test_save_and_load_paladin_state() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    let state = create_test_paladin_state();
    let state_id = state.paladin.uuid;

    // Save the state
    citadel
        .save_paladin(&state)
        .await
        .expect("Failed to save paladin state");

    // Load it back
    let loaded = citadel
        .load_paladin(state_id)
        .await
        .expect("Failed to load paladin state");

    assert!(loaded.is_some(), "State should be loaded");
    let loaded_state = loaded.unwrap();

    // Verify key fields match
    assert_eq!(loaded_state.paladin.uuid, state.paladin.uuid);
    assert_eq!(
        loaded_state.paladin.data.system_prompt,
        state.paladin.data.system_prompt
    );
    assert_eq!(loaded_state.garrison.len(), 1);
    assert_eq!(loaded_state.execution_history.len(), 1);
}

#[tokio::test]
async fn test_save_overwrites_existing_state() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    let mut state = create_test_paladin_state();
    let state_id = state.paladin.uuid;

    // Save initial state
    citadel
        .save_paladin(&state)
        .await
        .expect("Failed to save initial state");

    // Modify the state
    state.paladin.data.system_prompt = "Updated prompt".to_string();

    // Save again (should overwrite)
    citadel
        .save_paladin(&state)
        .await
        .expect("Failed to save updated state");

    // Load and verify it was overwritten
    let loaded = citadel
        .load_paladin(state_id)
        .await
        .expect("Failed to load state")
        .expect("State should exist");

    assert_eq!(loaded.paladin.data.system_prompt, "Updated prompt");
}

#[tokio::test]
async fn test_load_nonexistent_state_returns_none() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    let nonexistent_id = Uuid::new_v4();

    let result = citadel
        .load_paladin(nonexistent_id)
        .await
        .expect("Load should not error");

    assert!(result.is_none(), "Nonexistent state should return None");
}

#[tokio::test]
async fn test_load_corrupted_json_returns_error() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    // Create a corrupted JSON file
    let uuid = Uuid::new_v4();
    let corrupted_file = temp_dir.path().join(format!("paladin-{}.json", uuid));
    std::fs::write(&corrupted_file, "{ this is not valid json }")
        .expect("Failed to write corrupted file");

    let result = citadel.load_paladin(uuid).await;

    assert!(result.is_err(), "Corrupted JSON should return error");
    assert!(
        matches!(result, Err(CitadelError::CorruptedState(_))),
        "Should be CorruptedState error"
    );
}

#[tokio::test]
async fn test_directory_created_automatically() {
    let temp_dir = create_temp_dir();
    let nested_path = temp_dir.path().join("nested").join("citadel");

    // Directory should not exist yet
    assert!(
        !nested_path.exists(),
        "Directory should not exist before creation"
    );

    // Creating FileCitadel should create the directory
    let citadel = FileCitadel::new(nested_path.clone()).expect("Failed to create FileCitadel");

    assert!(
        nested_path.exists(),
        "Directory should be created automatically"
    );
    assert!(nested_path.is_dir(), "Path should be a directory");

    // Should be able to save to the newly created directory
    let state = create_test_paladin_state();
    citadel
        .save_paladin(&state)
        .await
        .expect("Should be able to save to new directory");
}

#[tokio::test]
async fn test_list_saved_states() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    // Initially should be empty
    let initial_list = citadel.list_saved().await.expect("Failed to list states");
    assert_eq!(initial_list.len(), 0, "Should start with no states");

    // Save several states
    let state1 = create_test_paladin_state();
    let state2 = create_test_paladin_state();

    citadel
        .save_paladin(&state1)
        .await
        .expect("Failed to save state1");
    citadel
        .save_paladin(&state2)
        .await
        .expect("Failed to save state2");

    // List should now contain 2 entries
    let list = citadel.list_saved().await.expect("Failed to list states");
    assert_eq!(list.len(), 2, "Should have 2 saved states");

    // Verify both states are in the list
    let ids: Vec<Uuid> = list.iter().map(|s| s.id).collect();
    assert!(ids.contains(&state1.paladin.uuid));
    assert!(ids.contains(&state2.paladin.uuid));
}

#[tokio::test]
async fn test_save_and_load_battalion_state() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    let paladin_state = create_test_paladin_state();
    let config = BattalionCheckpointConfig {
        max_concurrency: Some(4),
        timeout_seconds: Some(300),
        continue_on_error: true,
    };
    let checkpoint = CheckpointData::new();

    let battalion_state =
        BattalionState::new("Formation", config, vec![paladin_state], Some(checkpoint));
    let battalion_id = battalion_state.id;

    // Save battalion state
    citadel
        .save_battalion(&battalion_state)
        .await
        .expect("Failed to save battalion state");

    // Load it back
    let loaded = citadel
        .load_battalion(battalion_id)
        .await
        .expect("Failed to load battalion state");

    assert!(loaded.is_some(), "Battalion state should be loaded");
    let loaded_state = loaded.unwrap();

    assert_eq!(loaded_state.id, battalion_id);
    assert_eq!(loaded_state.battalion_type, "Formation");
    assert_eq!(loaded_state.paladin_states.len(), 1);
    assert!(loaded_state.checkpoint.is_some());
}

#[tokio::test]
async fn test_file_naming_convention() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    let paladin_state = create_test_paladin_state();
    let paladin_id = paladin_state.paladin.uuid;

    citadel
        .save_paladin(&paladin_state)
        .await
        .expect("Failed to save paladin");

    // Check file exists with correct naming convention
    let expected_file = temp_dir.path().join(format!("paladin-{}.json", paladin_id));
    assert!(
        expected_file.exists(),
        "File should follow paladin-{{uuid}}.json convention"
    );

    // Test battalion naming
    let battalion_state = BattalionState::new(
        "Phalanx",
        BattalionCheckpointConfig::default(),
        vec![],
        None,
    );
    let battalion_id = battalion_state.id;

    citadel
        .save_battalion(&battalion_state)
        .await
        .expect("Failed to save battalion");

    let expected_battalion_file = temp_dir
        .path()
        .join(format!("battalion-{}.json", battalion_id));
    assert!(
        expected_battalion_file.exists(),
        "File should follow battalion-{{uuid}}.json convention"
    );
}

#[tokio::test]
async fn test_permission_error_handling() {
    // Create a read-only directory to test permission errors
    let temp_dir = create_temp_dir();
    let readonly_dir = temp_dir.path().join("readonly");
    std::fs::create_dir(&readonly_dir).expect("Failed to create directory");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&readonly_dir)
            .expect("Failed to get metadata")
            .permissions();
        perms.set_mode(0o444); // Read-only
        std::fs::set_permissions(&readonly_dir, perms).expect("Failed to set permissions");

        // Attempting to create FileCitadel in read-only directory should fail
        let result = FileCitadel::new(readonly_dir.clone());
        assert!(
            result.is_err(),
            "Should fail to create FileCitadel in read-only directory"
        );

        // Clean up - restore write permissions
        let mut perms = std::fs::metadata(&readonly_dir)
            .expect("Failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&readonly_dir, perms).expect("Failed to restore permissions");
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, just verify the directory exists
        assert!(readonly_dir.exists());
    }
}

#[tokio::test]
async fn test_paladin_restoration_via_builder() {
    let temp_dir = create_temp_dir();
    let citadel = Arc::new(
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel"),
    );

    // Create and save a state
    let state = create_test_paladin_state();
    let state_id = state.paladin.uuid;

    citadel
        .save_paladin(&state)
        .await
        .expect("Failed to save state");

    // Use builder to restore
    let mock_llm = Arc::new(MockLlmPort);
    let builder = PaladinBuilder::new(mock_llm)
        .with_citadel(citadel.clone())
        .restore_from(state_id)
        .await
        .expect("Failed to restore from state");

    // Verify the builder has the restored configuration
    let paladin = builder.build().await.expect("Failed to build paladin");
    assert_eq!(paladin.data.system_prompt, "You are a helpful assistant");
    assert_eq!(paladin.data.name, "TestPaladin");
    assert_eq!(paladin.data.model, "gpt-4");
}

#[tokio::test]
async fn test_garrison_context_restored_correctly() {
    let temp_dir = create_temp_dir();
    let citadel =
        FileCitadel::new(temp_dir.path().to_path_buf()).expect("Failed to create FileCitadel");

    // Create state with garrison entries
    let paladin = create_test_paladin();
    let garrison = vec![
        GarrisonEntry::new(ConversationRole::User, "First message".to_string()),
        GarrisonEntry::new(ConversationRole::Assistant, "First response".to_string()),
        GarrisonEntry::new(ConversationRole::User, "Second message".to_string()),
    ];
    let history = vec![];

    let state = PaladinState::new(paladin, garrison, history);
    let state_id = state.paladin.uuid;

    // Save state
    citadel
        .save_paladin(&state)
        .await
        .expect("Failed to save state");

    // Load and verify garrison is intact
    let loaded = citadel
        .load_paladin(state_id)
        .await
        .expect("Failed to load state")
        .expect("State should exist");

    assert_eq!(loaded.garrison.len(), 3, "Should have 3 garrison entries");
    assert_eq!(loaded.garrison[0].content, "First message");
    assert_eq!(loaded.garrison[1].content, "First response");
    assert_eq!(loaded.garrison[2].content, "Second message");

    // Verify roles are correct
    assert_eq!(loaded.garrison[0].role, ConversationRole::User);
    assert_eq!(loaded.garrison[1].role, ConversationRole::Assistant);
    assert_eq!(loaded.garrison[2].role, ConversationRole::User);
}

// Mock LLM Port for testing
struct MockLlmPort;

#[async_trait::async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(
        &self,
        _request: paladin_ports::output::llm_port::LlmRequest,
    ) -> Result<
        paladin_ports::output::llm_port::LlmResponse,
        paladin_ports::output::llm_port::LlmError,
    > {
        unimplemented!("Not needed for this test")
    }

    async fn generate_stream(
        &self,
        _request: paladin_ports::output::llm_port::LlmRequest,
    ) -> Result<
        Box<
            dyn futures::Stream<
                    Item = Result<
                        paladin_ports::output::llm_port::StreamingResponse,
                        paladin_ports::output::llm_port::LlmError,
                    >,
                > + Send,
        >,
        paladin_ports::output::llm_port::LlmError,
    > {
        unimplemented!("Not needed for this test")
    }

    async fn validate_model(
        &self,
        _model: &str,
    ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
        Ok(true)
    }

    async fn get_available_models(
        &self,
    ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
        Ok(vec![])
    }

    fn get_provider_name(&self) -> &'static str {
        "Mock"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}
