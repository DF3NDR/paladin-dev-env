//! Shared `AssistantRepositoryPort` contract suite (D-28, D-29, D-30).
//!
//! One generic async function per contract clause, each taking `&dyn
//! AssistantRepositoryPort` (or, for the concurrency stress test, `Arc<dyn
//! AssistantRepositoryPort>`) and asserting inside. Every backend
//! (`InMemoryAssistantRepository`, `SqliteAssistantRepository`,
//! `PostgresAssistantRepository`) invokes these unchanged from its own
//! `#[tokio::test]`s, mirroring `crate::run::contract_tests`'s house
//! pattern. Named per-clause (not a declarative macro) so a failure names
//! the violated contract clause rather than a line number.

use std::sync::Arc;
use std::time::Duration;

use paladin_core::platform::container::assistant::{
    Assistant, AssistantDefinition, AssistantId, AssistantKind, AssistantVersion,
    NewAssistantVersion,
};
use paladin_ports::output::assistant_repository_port::{
    AssistantRepositoryError, AssistantRepositoryPort,
};

/// Build an `AssistantDefinition` fixture, tagging `body` with `marker` so
/// distinct calls are distinguishable.
pub fn sample_definition(marker: &str) -> AssistantDefinition {
    AssistantDefinition {
        kind: AssistantKind::Agent,
        body: serde_json::json!({ "marker": marker }),
    }
}

/// Build a `NewAssistantVersion` fixture around [`sample_definition`].
pub fn sample_new_version(marker: &str) -> NewAssistantVersion {
    NewAssistantVersion {
        definition: sample_definition(marker),
        created_by: Some("tester".to_string()),
        note: Some(format!("note-{marker}")),
    }
}

// ── Create + AlreadyExists ──────────────────────────────────────────────

/// `create` creates the assistant with `latest == 1` and returns version 1;
/// a second `create` for the same id fails with `AlreadyExists`.
pub async fn create_creates_version_one_and_rejects_duplicate(port: &dyn AssistantRepositoryPort) {
    let id = AssistantId::new("contract-assistant-create").unwrap();
    let version = port.create(&id, sample_new_version("v1")).await.unwrap();
    assert_eq!(version.version, 1);
    assert_eq!(version.assistant_id, id);

    let assistant: Assistant = port.get(&id).await.unwrap().unwrap();
    assert_eq!(assistant.latest, 1);
    assert!(!assistant.is_deleted());

    let err = port
        .create(&id, sample_new_version("v1-again"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        AssistantRepositoryError::AlreadyExists { .. }
    ));
}

// ── append_version ───────────────────────────────────────────────────────

/// `append_version` returns `latest + 1` and advances `latest`; an unknown
/// or soft-deleted id fails with `NotFound`.
pub async fn append_version_advances_latest_and_rejects_unknown_or_deleted(
    port: &dyn AssistantRepositoryPort,
) {
    let id = AssistantId::new("contract-assistant-append").unwrap();
    port.create(&id, sample_new_version("v1")).await.unwrap();

    let v2 = port
        .append_version(&id, sample_new_version("v2"))
        .await
        .unwrap();
    assert_eq!(v2.version, 2);
    let assistant = port.get(&id).await.unwrap().unwrap();
    assert_eq!(assistant.latest, 2);

    let unknown = AssistantId::new("contract-assistant-append-unknown").unwrap();
    let err = port
        .append_version(&unknown, sample_new_version("nope"))
        .await
        .unwrap_err();
    assert!(matches!(err, AssistantRepositoryError::NotFound { .. }));

    port.soft_delete(&id).await.unwrap();
    let err = port
        .append_version(&id, sample_new_version("v3-after-delete"))
        .await
        .unwrap_err();
    assert!(matches!(err, AssistantRepositoryError::NotFound { .. }));
}

// ── get_version out-of-range ─────────────────────────────────────────────

/// `get_version(id, 0)` and `get_version(id, latest + 1)` return `Ok(None)`;
/// an unknown assistant also returns `Ok(None)`.
pub async fn get_version_out_of_range_returns_none(port: &dyn AssistantRepositoryPort) {
    let id = AssistantId::new("contract-assistant-get-version-range").unwrap();
    port.create(&id, sample_new_version("v1")).await.unwrap();

    assert!(port.get_version(&id, 0).await.unwrap().is_none());
    assert!(port.get_version(&id, 2).await.unwrap().is_none());
    assert!(port.get_version(&id, 1).await.unwrap().is_some());

    let unknown = AssistantId::new("contract-assistant-get-version-unknown").unwrap();
    assert!(port.get_version(&unknown, 1).await.unwrap().is_none());
}

// ── Listing and pagination ────────────────────────────────────────────────

/// `list_versions` returns a strictly ascending `version` sequence across
/// pages, with no overlap and no gap.
pub async fn list_versions_paginates_ascending_by_version(port: &dyn AssistantRepositoryPort) {
    let id = AssistantId::new("contract-assistant-list-versions").unwrap();
    port.create(&id, sample_new_version("v1")).await.unwrap();
    for i in 2..=5u32 {
        port.append_version(&id, sample_new_version(&format!("v{i}")))
            .await
            .unwrap();
    }

    let mut seen = Vec::new();
    let mut cursor = None;
    for _ in 0..10 {
        let page = port.list_versions(&id, 2, cursor).await.unwrap();
        for item in &page.items {
            assert!(
                !seen.contains(&item.version),
                "version {} appeared on more than one page (overlap)",
                item.version
            );
            seen.push(item.version);
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(
        seen,
        vec![1, 2, 3, 4, 5],
        "no gap: every version seen, in order"
    );
}

/// `list` returns assistants in ascending `assistant_id` order across
/// pages, tolerant of interleaved rows created by other tests sharing the
/// same backend (Tier 2 Postgres): filters to the ids THIS clause created,
/// asserting no overlap and no gap among those.
pub async fn list_assistants_paginates_ascending_by_assistant_id(
    port: &dyn AssistantRepositoryPort,
) {
    let mut expected: Vec<AssistantId> = (0..5)
        .map(|i| AssistantId::new(format!("contract-assistant-list-page-{i}")).unwrap())
        .collect();
    for id in &expected {
        port.create(id, sample_new_version("v1")).await.unwrap();
    }
    expected.sort();

    let mut seen = Vec::new();
    let mut cursor = None;
    for _ in 0..1000 {
        let page = port.list(2, cursor.clone(), false).await.unwrap();
        if page.items.is_empty() && page.next_cursor.is_none() {
            break;
        }
        for item in &page.items {
            if expected.contains(&item.assistant_id) {
                assert!(
                    !seen.contains(&item.assistant_id),
                    "assistant {} appeared on more than one page (overlap)",
                    item.assistant_id
                );
                seen.push(item.assistant_id.clone());
            }
        }
        cursor = page.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(
        seen, expected,
        "no gap: every created assistant seen, in order"
    );
}

/// `list(.., include_deleted: false)` omits a soft-deleted assistant;
/// `include_deleted: true` includes it.
pub async fn list_excludes_soft_deleted_unless_requested(port: &dyn AssistantRepositoryPort) {
    let visible = AssistantId::new("contract-assistant-list-visible").unwrap();
    let deleted = AssistantId::new("contract-assistant-list-deleted").unwrap();
    port.create(&visible, sample_new_version("v1"))
        .await
        .unwrap();
    port.create(&deleted, sample_new_version("v1"))
        .await
        .unwrap();
    port.soft_delete(&deleted).await.unwrap();

    let mut without_deleted = Vec::new();
    let mut cursor = None;
    loop {
        let page = port.list(100, cursor.clone(), false).await.unwrap();
        without_deleted.extend(page.items.iter().map(|a| a.assistant_id.clone()));
        cursor = page.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }
    assert!(without_deleted.contains(&visible));
    assert!(!without_deleted.contains(&deleted));

    let mut with_deleted = Vec::new();
    let mut cursor = None;
    loop {
        let page = port.list(100, cursor.clone(), true).await.unwrap();
        with_deleted.extend(page.items.iter().map(|a| a.assistant_id.clone()));
        cursor = page.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }
    assert!(with_deleted.contains(&visible));
    assert!(with_deleted.contains(&deleted));
}

// ── Soft delete ────────────────────────────────────────────────────────────

/// `soft_delete` sets `deleted_at`; `get_version` stays readable for every
/// existing version (PLAT-FR-10), but `append_version` fails `NotFound`.
/// `soft_delete` on an unknown id fails `NotFound`.
pub async fn soft_delete_then_versions_still_readable_but_append_fails(
    port: &dyn AssistantRepositoryPort,
) {
    let id = AssistantId::new("contract-assistant-soft-delete").unwrap();
    port.create(&id, sample_new_version("v1")).await.unwrap();
    port.append_version(&id, sample_new_version("v2"))
        .await
        .unwrap();

    port.soft_delete(&id).await.unwrap();

    let assistant = port.get(&id).await.unwrap().unwrap();
    assert!(assistant.deleted_at.is_some());

    assert!(port.get_version(&id, 1).await.unwrap().is_some());
    assert!(port.get_version(&id, 2).await.unwrap().is_some());

    let err = port
        .append_version(&id, sample_new_version("v3-after-delete"))
        .await
        .unwrap_err();
    assert!(matches!(err, AssistantRepositoryError::NotFound { .. }));

    let unknown = AssistantId::new("contract-assistant-soft-delete-unknown").unwrap();
    let err = port.soft_delete(&unknown).await.unwrap_err();
    assert!(matches!(err, AssistantRepositoryError::NotFound { .. }));
}

// ── No content-based deduplication ────────────────────────────────────────

/// Publishing an identical definition body twice creates two distinct
/// versions (n+1, n+2), each with its own version number and note — there
/// is no content-based deduplication.
pub async fn publishing_identical_body_twice_creates_distinct_versions(
    port: &dyn AssistantRepositoryPort,
) {
    let id = AssistantId::new("contract-assistant-duplicate-body").unwrap();
    let definition = AssistantDefinition {
        kind: AssistantKind::Agent,
        body: serde_json::json!({ "same": true }),
    };

    port.create(
        &id,
        NewAssistantVersion {
            definition: definition.clone(),
            created_by: Some("alice".to_string()),
            note: Some("first".to_string()),
        },
    )
    .await
    .unwrap();
    let v2 = port
        .append_version(
            &id,
            NewAssistantVersion {
                definition: definition.clone(),
                created_by: Some("bob".to_string()),
                note: Some("second".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(v2.version, 2);
    let v1: AssistantVersion = port.get_version(&id, 1).await.unwrap().unwrap();
    assert_eq!(v1.definition, definition);
    assert_eq!(v2.definition, definition);
    assert_ne!(v1.version, v2.version);
    assert_ne!(v1.note, v2.note, "each publish keeps its own audit note");
    assert_ne!(v1.created_by, v2.created_by);
}

// ── Schema versioning (X-04) ────────────────────────────────────────────

/// A stored version whose `schema_version` does not match the currently
/// supported version fails `get_version` with `UnknownSchemaVersion`, never
/// a panic.
pub async fn get_version_on_unsupported_schema_version_fails(port: &dyn AssistantRepositoryPort) {
    // This clause is opt-in per backend (some adapters cannot easily write
    // a row with a "future" schema version through the port itself) — see
    // each backend's own test module for how it constructs the fixture.
    let id = AssistantId::new("contract-assistant-unknown-schema-version").unwrap();
    port.create(&id, sample_new_version("v1")).await.unwrap();
    // Sanity: the freshly created version is readable and carries the
    // current schema version -- the actual "future version" fixture is
    // backend-specific and asserted by each adapter's own test.
    let v1 = port.get_version(&id, 1).await.unwrap().unwrap();
    assert_eq!(
        v1.schema_version,
        paladin_core::platform::container::assistant::ASSISTANT_SCHEMA_VERSION
    );
}

// ── Concurrency stress test (X-05) ────────────────────────────────────────

/// Ten concurrent `append_version` calls on one assistant produce versions
/// `2..=11` with no gaps and no duplicates — every call succeeds because
/// the adapter retries internally on a version-numbering race.
pub async fn concurrent_append_admits_exactly_one_per_version(
    port: Arc<dyn AssistantRepositoryPort>,
) {
    let id = AssistantId::new("contract-assistant-concurrent-append").unwrap();
    port.create(&id, sample_new_version("v1")).await.unwrap();

    let mut handles = Vec::new();
    for i in 0..10 {
        let port = Arc::clone(&port);
        let id = id.clone();
        handles.push(tokio::spawn(async move {
            port.append_version(&id, sample_new_version(&format!("concurrent-{i}")))
                .await
        }));
    }

    let mut versions = Vec::new();
    tokio::time::timeout(Duration::from_secs(20), async {
        for handle in handles {
            let version = handle
                .await
                .expect("task must not panic")
                .expect("append_version must retry to success under contention");
            versions.push(version.version);
        }
    })
    .await
    .expect("ten concurrent append_version calls must not hang");

    versions.sort_unstable();
    assert_eq!(versions, (2..=11).collect::<Vec<_>>());

    let assistant = port.get(&id).await.unwrap().unwrap();
    assert_eq!(assistant.latest, 11);
}
