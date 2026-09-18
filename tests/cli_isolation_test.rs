//! Library-only CLI isolation regression tests.
//!
//! These tests verify that core library functionality compiles and operates
//! correctly without the `cli` feature enabled. They act as a guardrail:
//! if CLI dependencies leak back into the library compilation path, these
//! tests will fail to compile in a `--no-default-features` build.
//!
//! Run these tests in library-only mode:
//! ```bash
//! cargo test --test cli_isolation
//! cargo test --test cli_isolation --no-default-features
//! ```
//!
//! The CLI-feature-absence guard below reads the root manifest's declared `[features]
//! default` array rather than the current build's own compile-time feature
//! configuration, because a compile-time check cannot distinguish `cli` being
//! genuinely listed in `default` from `cli` merely having been switched on by a
//! whole-workspace feature-union invocation such as `--all-features`. That
//! indistinguishability is what made the same `--all-features` false failure get
//! re-logged by three consecutive phases.

// Deliberately do NOT import anything from `paladin::application::cli`.
// Any such import would mean the test only compiles with `--features cli`,
// which would defeat the purpose of this suite.

use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::paladin::MaxLoops;
use paladin::{BattalionConfig, BattalionError, PaladinConfig, PaladinData, PaladinStatus};

// ============================================================================
// Paladin domain entity tests (no CLI dependency)
// ============================================================================

#[test]
fn test_paladin_data_constructs_without_cli() {
    let data = PaladinData {
        name: "TestPaladin".to_string(),
        user_name: "test_user".to_string(),
        system_prompt: "You are a test assistant.".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec!["DONE".to_string()],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        autonomous_planning: false,
        autonomous_prompts: false,
        agent_description: String::new(),
        dynamic_temperature: false,
    };

    assert_eq!(data.name, "TestPaladin");
    assert!(!data.system_prompt.is_empty());
}

#[test]
fn test_paladin_config_default_without_cli() {
    // PaladinConfig must be constructible without any CLI dependency.
    let _config = PaladinConfig::default();
}

#[test]
fn test_paladin_status_variants_without_cli() {
    // All status variants must be reachable from library code without cli.
    let _idle = PaladinStatus::Idle;
    let _completed = PaladinStatus::Completed;
    let _failed = PaladinStatus::Failed("reason".to_string());
    let _reasoning = PaladinStatus::Reasoning;
    let _executing = PaladinStatus::Executing;
}

// ============================================================================
// Battalion domain entity tests (no CLI dependency)
// ============================================================================

#[test]
fn test_battalion_config_default_without_cli() {
    // BattalionConfig must be constructible without any CLI dependency.
    let _config = BattalionConfig::default();
}

#[test]
fn test_battalion_error_variants_without_cli() {
    let err = BattalionError::FormationError("test error".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("test error"),
        "BattalionError must format without CLI"
    );
}

#[test]
fn test_battalion_error_configuration_variant_without_cli() {
    let err = BattalionError::ConfigurationError("bad config".to_string());
    let msg = err.to_string();
    assert!(msg.contains("bad config"));
}

// ============================================================================
// Core base type tests (no CLI dependency)
// ============================================================================

#[test]
fn test_node_pattern_accessible_without_cli() {
    // Node<T> is the core entity wrapper used throughout the framework.
    // It must remain available without cli.
    let data = PaladinData {
        name: "NodeTest".to_string(),
        user_name: "u".to_string(),
        system_prompt: "s".to_string(),
        model: "m".to_string(),
        temperature: 0.5,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        autonomous_planning: false,
        autonomous_prompts: false,
        agent_description: String::new(),
        dynamic_temperature: false,
    };
    // Paladin is Node<PaladinData>; constructing it exercises the Node pattern.
    let paladin = Node::new(data, Some("NodeTest".to_string()));
    assert_eq!(paladin.node.name, "NodeTest");
}

// ============================================================================
// MaxLoops type test (no CLI dependency)
// ============================================================================

#[test]
fn test_max_loops_variants_without_cli() {
    let fixed = MaxLoops::Fixed(5);
    let auto = MaxLoops::Auto { max_subtasks: 10 };

    assert!(fixed.is_fixed());
    assert!(auto.is_auto());
    assert_eq!(fixed.as_u32(), 5);
    assert_eq!(auto.as_u32(), 10);
}

// ============================================================================
// CLI feature absence guard
// ============================================================================

/// Verifies that the `cli` feature is NOT included in the default feature set.
///
/// If this test panics, it means `cli` was inadvertently added to `[features]
/// default` in Cargo.toml. Remove it to keep the library compilation path
/// free of CLI-specific dependencies.
///
/// This asserts the root manifest's declared `[features] default` array directly
/// (via the `toml` crate, already a regular dependency of this package) rather than
/// the current build's own compile-time feature configuration, so the guard gives
/// the same answer under every invocation, including `cargo test --all-features`
/// (D-15).
#[test]
fn test_cli_feature_is_not_default() {
    let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let manifest_text =
        std::fs::read_to_string(manifest_path).expect("root Cargo.toml must be readable");
    let manifest: toml::Value = manifest_text
        .parse()
        .expect("root Cargo.toml must be valid TOML");
    let default_features = manifest
        .get("features")
        .and_then(|f| f.get("default"))
        .and_then(|d| d.as_array())
        .expect("[features] default must be an array in Cargo.toml");
    let names: Vec<&str> = default_features.iter().filter_map(|v| v.as_str()).collect();

    assert!(
        !names.contains(&"cli"),
        "The `cli` feature must not be listed in the `default` feature set in Cargo.toml. \
         Check that `cli` is NOT listed in the `default` feature set in Cargo.toml. Saw: {names:?}"
    );
}
