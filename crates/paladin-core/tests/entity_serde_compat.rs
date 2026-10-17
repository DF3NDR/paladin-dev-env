//! Persisted-JSON compatibility harness for the base entity wrapper (ADR-0060).
//!
//! The fixtures under `tests/fixtures/pre_rename/` are the bytes v0.10 wrote, captured from
//! the unrenamed tree and never hand-edited. This file proves they keep loading through the
//! public types and re-serialize to the same JSON document with only the renamed keys
//! changed: `node` becomes `data`, `ContentItem` / `PromptItem` write `entity`, and the
//! version record writes `entity_id` / `entity_data` in place of `node_id` / `node_data`.
//! It also pins the alias edge cases (both spellings present, neither present) and that new
//! writes never emit an old key. A failure here means the persisted wire shape changed, which
//! is a regression to report, not an assertion to relax.

use paladin_core::base::entity::Entity;
use paladin_core::base::service::entity_version_service::{ChangeType, EntityVersion};
use paladin_core::platform::container::citadel::PaladinState;
use paladin_core::platform::container::content::{ContentData, ContentItem, ContentType};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin::PaladinStatus;
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
    assert_eq!(loaded.data.model, expected_model);
    // Only the entity-sense key moved: the payload `node` is written as `data` (D-03).
    let mut expected = captured;
    rename_key(&mut expected, "", "node", "data")?;
    assert_eq!(reserialized(&loaded)?, expected);
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
    let mut expected = captured;
    rename_key(&mut expected, "/paladin", "node", "data")?;
    assert_eq!(reserialized(&loaded)?, expected);
    Ok(())
}

#[test]
fn pre_rename_content_item_json_still_loads() -> TestResult {
    let captured = fixture_value("content")?;
    let loaded: ContentItem = serde_json::from_str(&fixture("content")?)?;

    match &loaded.entity.data.content {
        ContentType::Text(text) => assert_eq!(text.content.as_deref(), Some("hello")),
        other => return Err(format!("expected ContentType::Text, found {other:?}").into()),
    }
    assert!(loaded.entity.version);
    // The wrapper field is written as `entity` (D-08), its payload as `data` (D-03).
    let mut expected = captured;
    rename_key(&mut expected, "", "node", "entity")?;
    rename_key(&mut expected, "/entity", "node", "data")?;
    assert_eq!(reserialized(&loaded)?, expected);
    Ok(())
}

#[test]
fn pre_rename_prompt_item_json_still_loads() -> TestResult {
    let captured = fixture_value("prompt")?;
    let loaded: PromptItem = serde_json::from_str(&fixture("prompt")?)?;

    match &loaded.entity.data.prompt_type {
        PromptType::Text(text) => assert_eq!(text.content, "hi"),
        other => return Err(format!("expected PromptType::Text, found {other:?}").into()),
    }
    let expected_max_tokens = captured
        .pointer("/node/node/parameters/max_tokens")
        .and_then(serde_json::Value::as_u64)
        .ok_or("fixture prompt.json has no integer at /node/node/parameters/max_tokens")?;
    assert_eq!(
        loaded.entity.data.parameters.max_tokens.map(u64::from),
        Some(expected_max_tokens)
    );
    let mut expected = captured;
    rename_key(&mut expected, "", "node", "entity")?;
    rename_key(&mut expected, "/entity", "node", "data")?;
    assert_eq!(reserialized(&loaded)?, expected);
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

/// D-10: the Citadel stores the real `PaladinStatus` now. Every status encoding v0.10 wrote
/// (`Idle`, `Executing`, `Completed`, `Failed`) must load into it and write back unchanged,
/// and the v0.11-only `Reasoning` status must serialize under its own name.
#[test]
fn v0_10_citadel_statuses_load_into_the_unified_status() -> TestResult {
    let encodings = [
        serde_json::json!("Idle"),
        serde_json::json!("Executing"),
        serde_json::json!("Completed"),
        serde_json::json!({ "Failed": "boom" }),
    ];
    for encoding in encodings {
        let mut snapshot = fixture_value("citadel")?;
        *snapshot
            .pointer_mut("/paladin/node/status")
            .ok_or("fixture citadel.json has no value at /paladin/node/status")? = encoding.clone();

        let loaded: PaladinState = serde_json::from_str(&snapshot.to_string())?;

        let written = reserialized(&loaded)?;
        assert_eq!(
            written.pointer("/paladin/data/status"),
            Some(&encoding),
            "status {encoding} changed on the way through PaladinState"
        );
    }

    // Reasoning exists only in the real status (v0.11 writes it, a v0.10 binary cannot read it).
    let mut loaded: PaladinState = serde_json::from_str(&fixture("citadel")?)?;
    loaded.paladin.data.status = PaladinStatus::Reasoning;
    let written = reserialized(&loaded)?;
    assert_eq!(
        written.pointer("/paladin/data/status"),
        Some(&serde_json::json!("Reasoning"))
    );
    assert!(serde_json::to_string(&loaded)?.contains("\"Reasoning\""));
    Ok(())
}

/// Parses `value` the way a real file read does (through its text) and returns the error
/// text, or fails the test when the document is accepted.
fn rejection<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    match serde_json::from_str::<T>(&value.to_string()) {
        Ok(_) => Err("the document was accepted but should have been rejected".into()),
        Err(error) => Ok(error.to_string()),
    }
}

/// Adds `to` beside `from` at the object found at `pointer`, as a copy of `from`'s value,
/// so the document carries both spellings of one key.
fn copy_key(
    value: &mut serde_json::Value,
    pointer: &str,
    from: &str,
    to: &str,
) -> Result<(), String> {
    let object = value
        .pointer_mut(pointer)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| format!("no JSON object at pointer {pointer:?}"))?;
    let copied = object
        .get(from)
        .cloned()
        .ok_or_else(|| format!("key {from:?} absent at pointer {pointer:?}"))?;
    object.insert(to.to_owned(), copied);
    Ok(())
}

/// A document that spells the payload key both ways is ambiguous. serde rejects it as a
/// duplicate field instead of picking one or merging (T-48-13).
#[test]
fn pre_rename_json_carrying_both_payload_keys_is_rejected() -> TestResult {
    let mut both = fixture_value("paladin")?;
    copy_key(&mut both, "", "node", "data")?;

    let message = rejection::<Paladin>(&both)?;

    assert!(
        message.contains("duplicate field"),
        "expected a duplicate-field error, got: {message}"
    );
    Ok(())
}

/// Same rule for the wrapper field of `ContentItem` (D-08): `entity` beside the v0.10
/// outer `node`.
#[test]
fn content_item_carrying_both_entity_keys_is_rejected() -> TestResult {
    let mut both = fixture_value("content")?;
    copy_key(&mut both, "", "node", "entity")?;

    let message = rejection::<ContentItem>(&both)?;

    assert!(
        message.contains("duplicate field"),
        "expected a duplicate-field error, got: {message}"
    );
    Ok(())
}

/// A document with no payload key at all is not defaulted into an empty payload.
#[test]
fn json_without_a_payload_key_is_rejected() -> TestResult {
    let mut neither = fixture_value("paladin")?;
    neither
        .as_object_mut()
        .ok_or("fixture paladin.json is not an object")?
        .remove("node")
        .ok_or("fixture paladin.json has no node key")?;

    let message = rejection::<Paladin>(&neither)?;

    assert!(
        message.contains("missing field"),
        "expected a missing-field error, got: {message}"
    );
    Ok(())
}

/// The payload type stays generic: `Entity<T>` reads the v0.10 key for any `T`, and the
/// old and new spellings of the same document load to equal values.
#[test]
fn generic_entity_reads_the_old_payload_key() -> TestResult {
    #[derive(
        Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
    )]
    struct Label {
        label: String,
    }

    let document = |payload_key: &str| {
        format!(
            r#"{{"uuid":"67e55044-10b1-426f-9247-bb680e5fe0c8",
                "created":"2026-01-02T03:04:05Z",
                "modified":"2026-01-02T03:04:06Z",
                "{payload_key}":{{"label":"vanguard"}},
                "name":"unit",
                "version":false}}"#
        )
    };

    let old: Entity<Label> = serde_json::from_str(&document("node"))?;
    let new: Entity<Label> = serde_json::from_str(&document("data"))?;

    assert_eq!(old, new);
    assert_eq!(old.data.label, "vanguard");
    assert_eq!(old.name.as_deref(), Some("unit"));
    assert!(!old.version);
    Ok(())
}

/// D-03 / D-08 / D-09: new writes emit only the new keys. For each of the five captured
/// documents, the key each one must hold at its own entity-sense position (and the old key
/// it must not hold there), then none of the old spellings anywhere in the text.
///
/// | fixture | JSON pointer | key required | keys forbidden |
/// |---------|--------------|--------------|----------------|
/// | paladin | `` (root) | `data` | `node` |
/// | citadel | `/paladin` | `data` | `node` |
/// | content | `` (root) | `entity` | `node` |
/// | content | `/entity` | `data` | `node` |
/// | prompt | `` (root) | `entity` | `node` |
/// | prompt | `/entity` | `data` | `node` |
/// | version | `` (root) | `entity_id`, `entity_data` | `node_id`, `node_data`, `data` |
///
/// The version record is required to hold no `data` key: its payload sits under
/// `entity_data`, and `ContentData`, `TextContent` and `EntityVersionMetadata` declare no
/// `data` field of their own.
#[test]
fn new_writes_never_emit_the_old_keys() -> TestResult {
    fn check(
        written: &serde_json::Value,
        pointer: &str,
        required: &[&str],
        forbidden: &[&str],
    ) -> TestResult {
        let object = written
            .pointer(pointer)
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| format!("no JSON object at pointer {pointer:?}"))?;
        for key in required {
            assert!(object.contains_key(*key), "{pointer:?} should hold {key:?}");
        }
        for key in forbidden {
            assert!(
                !object.contains_key(*key),
                "{pointer:?} should not hold {key:?}"
            );
        }
        Ok(())
    }

    fn assert_no_old_spelling(text: &str) {
        for old in ["\"node\":", "\"node_id\":", "\"node_data\":"] {
            assert!(!text.contains(old), "serialized text still holds {old}");
        }
    }

    let paladin: Paladin = serde_json::from_str(&fixture("paladin")?)?;
    let text = serde_json::to_string(&paladin)?;
    check(&reserialized(&paladin)?, "", &["data"], &["node"])?;
    assert_no_old_spelling(&text);

    let citadel: PaladinState = serde_json::from_str(&fixture("citadel")?)?;
    let text = serde_json::to_string(&citadel)?;
    check(&reserialized(&citadel)?, "/paladin", &["data"], &["node"])?;
    assert_no_old_spelling(&text);

    let content: ContentItem = serde_json::from_str(&fixture("content")?)?;
    let text = serde_json::to_string(&content)?;
    let written = reserialized(&content)?;
    check(&written, "", &["entity"], &["node"])?;
    check(&written, "/entity", &["data"], &["node"])?;
    assert_no_old_spelling(&text);

    let prompt: PromptItem = serde_json::from_str(&fixture("prompt")?)?;
    let text = serde_json::to_string(&prompt)?;
    let written = reserialized(&prompt)?;
    check(&written, "", &["entity"], &["node"])?;
    check(&written, "/entity", &["data"], &["node"])?;
    assert_no_old_spelling(&text);

    let version: EntityVersion<ContentData> = serde_json::from_str(&fixture("version")?)?;
    let text = serde_json::to_string(&version)?;
    check(
        &reserialized(&version)?,
        "",
        &["entity_id", "entity_data"],
        &["node_id", "node_data", "data"],
    )?;
    assert_no_old_spelling(&text);
    Ok(())
}
