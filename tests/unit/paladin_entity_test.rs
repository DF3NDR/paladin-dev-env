use paladin::core::base::entity::Entity;
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};

#[test]
fn test_paladin_status_transitions() {
    // Test state transitions
    let idle = PaladinStatus::Idle;
    let reasoning = PaladinStatus::Reasoning;
    let executing = PaladinStatus::Executing;
    let completed = PaladinStatus::Completed;
    let failed = PaladinStatus::Failed("error".to_string());

    // Verify states
    assert_eq!(idle, PaladinStatus::Idle);
    assert_eq!(reasoning, PaladinStatus::Reasoning);
    assert_eq!(executing, PaladinStatus::Executing);
    assert_eq!(completed, PaladinStatus::Completed);
    assert_eq!(failed, PaladinStatus::Failed("error".to_string()));

    // Test terminal states
    assert!(!idle.is_terminal());
    assert!(!reasoning.is_terminal());
    assert!(!executing.is_terminal());
    assert!(completed.is_terminal());
    assert!(failed.is_terminal());

    // Test active states
    assert!(!idle.is_active());
    assert!(reasoning.is_active());
    assert!(executing.is_active());
    assert!(!completed.is_active());
    assert!(!failed.is_active());
}

#[test]
fn test_paladin_data_serialization_roundtrip() {
    let data = PaladinData {
        system_prompt: "You are a helpful assistant".to_string(),
        name: "TestPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(5),
        stop_words: vec!["STOP".to_string(), "END".to_string()],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&data).expect("Failed to serialize");

    // Deserialize back
    let deserialized: PaladinData = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify all fields match
    assert_eq!(data.system_prompt, deserialized.system_prompt);
    assert_eq!(data.name, deserialized.name);
    assert_eq!(data.user_name, deserialized.user_name);
    assert_eq!(data.model, deserialized.model);
    assert_eq!(data.temperature, deserialized.temperature);
    assert_eq!(data.max_loops, deserialized.max_loops);
    assert_eq!(data.stop_words, deserialized.stop_words);
    assert_eq!(data.status, deserialized.status);
}

#[test]
fn test_paladin_data_clone() {
    let original = PaladinData {
        system_prompt: "Original prompt".to_string(),
        name: "Original".to_string(),
        user_name: "User".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.5,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec!["STOP".to_string()],
        status: PaladinStatus::Reasoning,
        vision_enabled: false,
        ..Default::default()
    };

    let cloned = original.clone();

    // Verify clone matches original
    assert_eq!(original.system_prompt, cloned.system_prompt);
    assert_eq!(original.name, cloned.name);
    assert_eq!(original.status, cloned.status);
}

#[test]
fn test_paladin_node_creation() {
    let data = PaladinData {
        system_prompt: "You are a code reviewer".to_string(),
        name: "CodeReviewer".to_string(),
        user_name: "Developer".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.3,
        max_loops: MaxLoops::Fixed(5),
        stop_words: vec!["DONE".to_string()],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    let paladin: Paladin = Entity::new(data.clone(), Some("CodeReviewer".to_string()));

    // Verify Entity properties
    assert_eq!(paladin.name, Some("CodeReviewer".to_string()));
    assert_eq!(paladin.data.name, "CodeReviewer");
    assert_eq!(paladin.data.system_prompt, "You are a code reviewer");
    assert!(paladin.version);
}

#[test]
fn test_paladin_default() {
    let data = PaladinData::default();

    assert_eq!(data.name, "Paladin");
    assert_eq!(data.user_name, "User");
    assert_eq!(data.model, "gpt-4");
    assert_eq!(data.temperature, 0.7);
    assert_eq!(data.max_loops, MaxLoops::Fixed(3));
    assert_eq!(data.status, PaladinStatus::Idle);
    assert!(data.stop_words.is_empty());
    assert!(data.system_prompt.is_empty());
}
