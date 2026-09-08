/*
In-Memory Assistant Repository

An `Arc<tokio::sync::RwLock<Store>>`-backed implementation of
`AssistantRepositoryPort`, for tests and local development (D-03's InMemory
convention). `append_version` and `create` both run under the single write
lock, so this adapter's "retry on VersionConflict" story is trivial: there
is never a real race to retry, because the whole read-then-write sequence
is already serialized -- the SQL adapters (Task 3) are where the retry loop
does real work.
*/

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use paladin_core::platform::container::assistant::{
    ASSISTANT_SCHEMA_VERSION, Assistant, AssistantId, AssistantSource, AssistantVersion,
    NewAssistantVersion,
};
use paladin_ports::output::assistant_repository_port::{
    AssistantPage, AssistantRepositoryError, AssistantRepositoryPort, AssistantVersionPage,
};

#[derive(Default)]
struct Store {
    assistants: HashMap<AssistantId, Assistant>,
    versions: HashMap<AssistantId, Vec<AssistantVersion>>,
}

/// In-memory `AssistantRepositoryPort` implementation.
///
/// Cloning is cheap and shares the same underlying store (the inner `Arc`
/// is cloned).
#[derive(Clone, Default)]
pub struct InMemoryAssistantRepository {
    store: Arc<RwLock<Store>>,
}

impl InMemoryAssistantRepository {
    /// Construct a new, empty repository.
    pub fn new() -> Self {
        Self::default()
    }

    /// The assistant's current `latest` version, or `None` if it does not
    /// exist or is soft-deleted. A thin, read-lock-only convenience used by
    /// `InMemoryRunRepository::insert_with_latest` (D-30) so that caller
    /// does not have to reach into this type's private `Store`.
    pub async fn latest_version_if_active(&self, assistant_id: &AssistantId) -> Option<u32> {
        let store = self.store.read().await;
        store
            .assistants
            .get(assistant_id)
            .filter(|a| a.deleted_at.is_none())
            .map(|a| a.latest)
    }
}

#[async_trait]
impl AssistantRepositoryPort for InMemoryAssistantRepository {
    async fn create(
        &self,
        assistant_id: &AssistantId,
        new: NewAssistantVersion,
    ) -> Result<AssistantVersion, AssistantRepositoryError> {
        let mut store = self.store.write().await;
        if store.assistants.contains_key(assistant_id) {
            return Err(AssistantRepositoryError::AlreadyExists {
                assistant_id: assistant_id.clone(),
            });
        }
        let mut version = AssistantVersion::new(assistant_id.clone(), 1, new.definition);
        version.created_by = new.created_by;
        version.note = new.note;

        store.assistants.insert(
            assistant_id.clone(),
            Assistant::new(assistant_id.clone(), 1, AssistantSource::Stored),
        );
        store
            .versions
            .insert(assistant_id.clone(), vec![version.clone()]);
        Ok(version)
    }

    async fn append_version(
        &self,
        assistant_id: &AssistantId,
        new: NewAssistantVersion,
    ) -> Result<AssistantVersion, AssistantRepositoryError> {
        let mut store = self.store.write().await;
        let assistant = store.assistants.get_mut(assistant_id).ok_or_else(|| {
            AssistantRepositoryError::NotFound {
                assistant_id: assistant_id.clone(),
            }
        })?;
        if assistant.deleted_at.is_some() {
            return Err(AssistantRepositoryError::NotFound {
                assistant_id: assistant_id.clone(),
            });
        }
        let next_version = assistant.latest + 1;
        let mut version = AssistantVersion::new(assistant_id.clone(), next_version, new.definition);
        version.created_by = new.created_by;
        version.note = new.note;

        assistant.latest = next_version;
        store
            .versions
            .entry(assistant_id.clone())
            .or_default()
            .push(version.clone());
        Ok(version)
    }

    async fn get(
        &self,
        assistant_id: &AssistantId,
    ) -> Result<Option<Assistant>, AssistantRepositoryError> {
        let store = self.store.read().await;
        match store.assistants.get(assistant_id) {
            // X-04: a row whose schema_version this build does not
            // recognize must fail loudly rather than silently misparse.
            Some(a) if a.schema_version != ASSISTANT_SCHEMA_VERSION => {
                Err(AssistantRepositoryError::UnknownSchemaVersion {
                    found: a.schema_version.clone(),
                })
            }
            other => Ok(other.cloned()),
        }
    }

    async fn get_version(
        &self,
        assistant_id: &AssistantId,
        version: u32,
    ) -> Result<Option<AssistantVersion>, AssistantRepositoryError> {
        if version == 0 {
            return Ok(None);
        }
        let store = self.store.read().await;
        let Some(versions) = store.versions.get(assistant_id) else {
            return Ok(None);
        };
        match versions.iter().find(|v| v.version == version) {
            Some(v) if v.schema_version != ASSISTANT_SCHEMA_VERSION => {
                Err(AssistantRepositoryError::UnknownSchemaVersion {
                    found: v.schema_version.clone(),
                })
            }
            other => Ok(other.cloned()),
        }
    }

    async fn list(
        &self,
        limit: u32,
        cursor: Option<AssistantId>,
        include_deleted: bool,
    ) -> Result<AssistantPage, AssistantRepositoryError> {
        let store = self.store.read().await;
        let mut items: Vec<Assistant> = store
            .assistants
            .values()
            .filter(|a| include_deleted || a.deleted_at.is_none())
            .cloned()
            .collect();
        items.sort_by(|a, b| a.assistant_id.cmp(&b.assistant_id));

        if let Some(cursor) = &cursor {
            let cut = items
                .iter()
                .position(|a| &a.assistant_id == cursor)
                .map(|idx| idx + 1)
                .unwrap_or(0);
            items = items.split_off(cut.min(items.len()));
        }

        let effective_limit = if limit == 0 {
            items.len()
        } else {
            limit as usize
        };
        let next_cursor = if items.len() > effective_limit {
            items
                .get(effective_limit.saturating_sub(1))
                .map(|last| last.assistant_id.clone())
        } else {
            None
        };
        items.truncate(effective_limit);
        Ok(AssistantPage { items, next_cursor })
    }

    async fn list_versions(
        &self,
        assistant_id: &AssistantId,
        limit: u32,
        cursor: Option<u32>,
    ) -> Result<AssistantVersionPage, AssistantRepositoryError> {
        let store = self.store.read().await;
        let mut items: Vec<AssistantVersion> = store
            .versions
            .get(assistant_id)
            .cloned()
            .unwrap_or_default();
        items.sort_by_key(|v| v.version);

        if let Some(cursor) = cursor {
            let cut = items
                .iter()
                .position(|v| v.version == cursor)
                .map(|idx| idx + 1)
                .unwrap_or(0);
            items = items.split_off(cut.min(items.len()));
        }

        let effective_limit = if limit == 0 {
            items.len()
        } else {
            limit as usize
        };
        let next_cursor = if items.len() > effective_limit {
            items
                .get(effective_limit.saturating_sub(1))
                .map(|last| last.version)
        } else {
            None
        };
        items.truncate(effective_limit);
        Ok(AssistantVersionPage { items, next_cursor })
    }

    async fn soft_delete(
        &self,
        assistant_id: &AssistantId,
    ) -> Result<(), AssistantRepositoryError> {
        let mut store = self.store.write().await;
        let assistant = store.assistants.get_mut(assistant_id).ok_or_else(|| {
            AssistantRepositoryError::NotFound {
                assistant_id: assistant_id.clone(),
            }
        })?;
        assistant.deleted_at = Some(Utc::now());
        Ok(())
    }
}

#[cfg(test)]
mod contract_suite {
    use super::*;
    use crate::assistant::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function (D-09 precedent, per
    // `crate::run::in_memory`'s own convention), each against a fresh
    // `InMemoryAssistantRepository`, so a failure names the violated
    // contract clause.

    #[tokio::test]
    async fn create_creates_version_one_and_rejects_duplicate() {
        contract_tests::create_creates_version_one_and_rejects_duplicate(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn append_version_advances_latest_and_rejects_unknown_or_deleted() {
        contract_tests::append_version_advances_latest_and_rejects_unknown_or_deleted(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn get_version_out_of_range_returns_none() {
        contract_tests::get_version_out_of_range_returns_none(&InMemoryAssistantRepository::new())
            .await;
    }

    #[tokio::test]
    async fn list_versions_paginates_ascending_by_version() {
        contract_tests::list_versions_paginates_ascending_by_version(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn list_assistants_paginates_ascending_by_assistant_id() {
        contract_tests::list_assistants_paginates_ascending_by_assistant_id(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn list_excludes_soft_deleted_unless_requested() {
        contract_tests::list_excludes_soft_deleted_unless_requested(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn soft_delete_then_versions_still_readable_but_append_fails() {
        contract_tests::soft_delete_then_versions_still_readable_but_append_fails(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn publishing_identical_body_twice_creates_distinct_versions() {
        contract_tests::publishing_identical_body_twice_creates_distinct_versions(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn get_version_on_unsupported_schema_version_fails() {
        contract_tests::get_version_on_unsupported_schema_version_fails(
            &InMemoryAssistantRepository::new(),
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_append_admits_exactly_one_per_version() {
        let repo: Arc<dyn AssistantRepositoryPort> = Arc::new(InMemoryAssistantRepository::new());
        contract_tests::concurrent_append_admits_exactly_one_per_version(repo).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn latest_version_if_active_reflects_create_append_and_soft_delete() {
        let repo = InMemoryAssistantRepository::new();
        let id = AssistantId::new("a1").unwrap();
        assert!(repo.latest_version_if_active(&id).await.is_none());

        repo.create(
            &id,
            NewAssistantVersion {
                definition: paladin_core::platform::container::assistant::AssistantDefinition {
                    kind: paladin_core::platform::container::assistant::AssistantKind::Agent,
                    body: serde_json::json!({}),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(repo.latest_version_if_active(&id).await, Some(1));

        repo.append_version(
            &id,
            NewAssistantVersion {
                definition: paladin_core::platform::container::assistant::AssistantDefinition {
                    kind: paladin_core::platform::container::assistant::AssistantKind::Agent,
                    body: serde_json::json!({}),
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(repo.latest_version_if_active(&id).await, Some(2));

        repo.soft_delete(&id).await.unwrap();
        assert!(repo.latest_version_if_active(&id).await.is_none());
    }

    #[tokio::test]
    async fn get_and_get_version_return_none_for_unknown_assistant() {
        let repo = InMemoryAssistantRepository::new();
        let unknown = AssistantId::new("does-not-exist").unwrap();
        assert!(repo.get(&unknown).await.unwrap().is_none());
        assert!(repo.get_version(&unknown, 1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_versions_returns_empty_page_for_unknown_assistant() {
        let repo = InMemoryAssistantRepository::new();
        let unknown = AssistantId::new("does-not-exist").unwrap();
        let page = repo.list_versions(&unknown, 10, None).await.unwrap();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_with_zero_limit_returns_everything_on_one_page() {
        let repo = InMemoryAssistantRepository::new();
        for i in 0..3 {
            let id = AssistantId::new(format!("zero-limit-{i}")).unwrap();
            repo.create(
                &id,
                NewAssistantVersion {
                    definition: paladin_core::platform::container::assistant::AssistantDefinition {
                        kind: paladin_core::platform::container::assistant::AssistantKind::Agent,
                        body: serde_json::json!({}),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();
        }
        let page = repo.list(0, None, false).await.unwrap();
        assert!(page.items.len() >= 3);
        assert!(page.next_cursor.is_none());
    }
}
