//! Persisted-JSON compatibility harness for the base entity wrapper.
//!
//! The fixtures under `tests/fixtures/pre_rename/` are the bytes v0.10 wrote. Plan 48-01
//! captured them from the unmodified tree and they are never hand-edited. This file proves
//! they keep loading through the public types and re-serialize to an identical JSON
//! document (ENTITY-01, D-03; the version record per D-09; the Citadel snapshot per D-10).
//!
//! Later plans update the lossless comparison to the renamed keys and add the alias edge
//! cases (both keys present, neither key present). The version record (plan 48-03) already
//! compares against the fixture with `node_id` / `node_data` renamed to `entity_id` /
//! `entity_data`. A failure here means the persisted wire
//! shape changed, which is a regression to report, not an assertion to relax.

use paladin_core::base::service::entity_version_service::{ChangeType, EntityVersion};
use paladin_core::platform::container::citadel::PaladinState;
use paladin_core::platform::container::content::{ContentData, ContentItem, ContentType};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::prompt::{PromptItem, PromptType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Reads the raw text of a captured fixture.
fn fixture(name: &str) -> std::io::Result<String> {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/pre_rename/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// Parses a captured fixture into a JSON value.
fn fixture_value(name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&fixture(name)?)?)
}

/// Serializes `value` to text and parses that text back, so numbers keep their shortest
/// textual form. Building the value directly from the struct would widen `f32` fields (for
/// example `temperature`) to `f64` artefacts and make an honest round trip compare unequal.
fn reserialized<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(&serde_json::to_string(value)?)
}

/// Moves one key inside the object found at the JSON `pointer`, keeping its value.
///
/// Errors if the pointer does not resolve to an object, if `from` is absent, or if `to` is
/// already present (a rename must never silently overwrite).
fn rename_key(
    value: &mut serde_json::Value,
    pointer: &str,
    from: &str,
    to: &str,
) -> Result<(), String> {
    let object = value
        .pointer_mut(pointer)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| format!("no JSON object at pointer {pointer:?}"))?;
    if object.contains_key(to) {
        return Err(format!("key {to:?} already present at pointer {pointer:?}"));
    }
    let moved = object
        .remove(from)
        .ok_or_else(|| format!("key {from:?} absent at pointer {pointer:?}"))?;
    object.insert(to.to_owned(), moved);
    Ok(())
}

#[test]
fn pre_rename_paladin_json_still_loads() -> TestResult {
    let captured = fixture_value("paladin")?;
    let loaded: Paladin = serde_json::from_str(&fixture("paladin")?)?;

    assert_eq!(loaded.name.as_deref(), Some("fixture"));
    assert!(loaded.version);
    let expected_model = captured
        .pointer("/node/model")
        .and_then(serde_json::Value::as_str)
        .ok_or("fixture paladin.json has no string at /node/model")?;
    assert_eq!(loaded.node.model, expected_model);
    assert_eq!(reserialized(&loaded)?, captured);
    Ok(())
}

#[test]
fn pre_rename_citadel_json_still_loads() -> TestResult {
    let captured = fixture_value("citadel")?;
    let loaded: PaladinState = serde_json::from_str(&fixture("citadel")?)?;

    assert_eq!(loaded.paladin.name.as_deref(), Some("fixture"));
    assert_eq!(loaded.schema_version, "1.0.0");
    assert!(loaded.garrison.is_empty());
    assert!(loaded.execution_history.is_empty());
    assert_eq!(reserialized(&loaded)?, captured);
    Ok(())
}

#[test]
fn pre_rename_content_item_json_still_loads() -> TestResult {
    let captured = fixture_value("content")?;
    let loaded: ContentItem = serde_json::from_str(&fixture("content")?)?;

    match &loaded.node.node.content {
        ContentType::Text(text) => assert_eq!(text.content.as_deref(), Some("hello")),
        other => return Err(format!("expected ContentType::Text, found {other:?}").into()),
    }
    assert!(loaded.node.version);
    assert_eq!(reserialized(&loaded)?, captured);
    Ok(())
}

#[test]
fn pre_rename_prompt_item_json_still_loads() -> TestResult {
    let captured = fixture_value("prompt")?;
    let loaded: PromptItem = serde_json::from_str(&fixture("prompt")?)?;

    match &loaded.node.node.prompt_type {
        PromptType::Text(text) => assert_eq!(text.content, "hi"),
        other => return Err(format!("expected PromptType::Text, found {other:?}").into()),
    }
    let expected_max_tokens = captured
        .pointer("/node/node/parameters/max_tokens")
        .and_then(serde_json::Value::as_u64)
        .ok_or("fixture prompt.json has no integer at /node/node/parameters/max_tokens")?;
    assert_eq!(
        loaded.node.node.parameters.max_tokens.map(u64::from),
        Some(expected_max_tokens)
    );
    assert_eq!(reserialized(&loaded)?, captured);
    Ok(())
}

#[test]
fn pre_rename_version_json_still_loads() -> TestResult {
    let mut expected = fixture_value("version")?;
    rename_key(&mut expected, "", "node_id", "entity_id")?;
    rename_key(&mut expected, "", "node_data", "entity_data")?;
    let loaded: EntityVersion<ContentData> = serde_json::from_str(&fixture("version")?)?;

    assert_eq!(loaded.version_number, 1);
    assert_eq!(loaded.created_by.as_deref(), Some("fixture"));
    assert_eq!(loaded.metadata.change_type, ChangeType::Created);
    let content_uuid = fixture_value("content")?
        .pointer("/node/uuid")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or("fixture content.json has no string at /node/uuid")?;
    assert_eq!(loaded.entity_id.to_string(), content_uuid);
    assert_eq!(reserialized(&loaded)?, expected);
    Ok(())
}
