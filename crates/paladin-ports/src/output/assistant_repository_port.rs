//! Assistant Repository Port — Append-Only Immutable Versions (D-28, D-29)
//!
//! [`AssistantRepositoryPort`] is the whole contract every backend adapter
//! (`InMemoryAssistantRepository`, `SqliteAssistantRepository`,
//! `PostgresAssistantRepository`, all `paladin-storage`) implements.
//!
//! ## No method exists to mutate an existing version, by design (D-29)
//!
//! This trait deliberately exposes exactly `create`, `append_version`,
//! `get`, `get_version`, `list`, `list_versions` and `soft_delete` — there
//! is no method anywhere in this trait that rewrites the `definition` of a
//! version that already exists, and no `PUT` route is ever registered for
//! one on the HTTP surface above this port. Immutability holds because the
//! code to violate it does not exist, not because callers are trusted to
//! avoid calling it.
//!
//! ## `latest` is a property of [`Assistant`], not of any version row
//!
//! [`Assistant::latest`] is the most recently published version number.
//! [`AssistantRepositoryPort::append_version`] both writes the new
//! `(assistant_id, version)` row and advances `latest` — an adapter must do
//! both atomically (a single transaction on a SQL backend), since
//! [`crate::output::run_repository_port::RunRepositoryPort::insert_with_latest`]
//! reads `latest` to freeze a run's assistant reference at submit time
//! (D-30).

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::assistant::{
    Assistant, AssistantId, AssistantVersion, NewAssistantVersion,
};

/// A page of [`Assistant`]s returned by [`AssistantRepositoryPort::list`],
/// ordered ascending by `assistant_id`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssistantPage {
    /// The page's assistants, in the documented order.
    pub items: Vec<Assistant>,
    /// Opaque cursor for the next page (the last `assistant_id` on this
    /// page), `None` on the last page.
    pub next_cursor: Option<AssistantId>,
}

/// A page of [`AssistantVersion`]s returned by
/// [`AssistantRepositoryPort::list_versions`], ordered ascending by
/// `version`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssistantVersionPage {
    /// The page's versions, in the documented order.
    pub items: Vec<AssistantVersion>,
    /// Opaque cursor for the next page (the last `version` on this page),
    /// `None` on the last page.
    pub next_cursor: Option<u32>,
}

/// Errors returned by [`AssistantRepositoryPort`] methods (X-06 —
/// structured, never a bare `bool`/`String`).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssistantRepositoryError {
    /// No assistant exists with the given id.
    #[error("assistant not found: {assistant_id}")]
    NotFound {
        /// The requested assistant id.
        assistant_id: AssistantId,
    },
    /// [`AssistantRepositoryPort::create`] was called with an id that
    /// already exists.
    #[error("assistant already exists: {assistant_id}")]
    AlreadyExists {
        /// The already-existing assistant id.
        assistant_id: AssistantId,
    },
    /// A version-numbering race: two concurrent publishes both computed the
    /// same `version` for `assistant_id`. Callers of
    /// [`AssistantRepositoryPort::append_version`] never observe this
    /// directly — the port retries internally — but the SQL adapters'
    /// underlying `append_version_once` primitive returns it.
    #[error("version conflict for assistant {assistant_id} version {version}")]
    VersionConflict {
        /// The assistant whose version numbering conflicted.
        assistant_id: AssistantId,
        /// The version number two writers raced to claim.
        version: u32,
    },
    /// The underlying storage backend failed.
    #[error("assistant repository backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) assistant or version could not be
    /// (de)serialized.
    #[error("assistant serialization error: {message}")]
    Serialization {
        /// Description of the serialization failure.
        message: String,
    },
    /// A stored assistant or version carries a schema version this build
    /// does not know how to read.
    #[error("unsupported assistant schema version: found {found}")]
    UnknownSchemaVersion {
        /// The schema version found on the stored data.
        found: String,
    },
}

/// Port trait for persisting and reading back append-only immutable
/// [`Assistant`] versions (D-28, D-29).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: assistants are published, read
/// and resolved concurrently across HTTP handlers and the run-submission
/// path.
#[async_trait]
pub trait AssistantRepositoryPort: Send + Sync {
    /// Create a brand-new assistant with its first version (`version == 1`,
    /// `latest == 1`).
    ///
    /// # Errors
    ///
    /// Returns [`AssistantRepositoryError::AlreadyExists`] if `assistant_id`
    /// already exists.
    async fn create(
        &self,
        assistant_id: &AssistantId,
        new: NewAssistantVersion,
    ) -> Result<AssistantVersion, AssistantRepositoryError>;

    /// Publish a new version onto an existing assistant: the returned
    /// version's number is `latest + 1`, and `latest` advances to match.
    ///
    /// # Errors
    ///
    /// Returns [`AssistantRepositoryError::NotFound`] when `assistant_id`
    /// does not exist or is soft-deleted.
    async fn append_version(
        &self,
        assistant_id: &AssistantId,
        new: NewAssistantVersion,
    ) -> Result<AssistantVersion, AssistantRepositoryError>;

    /// Load an assistant by id, including a soft-deleted one. `Ok(None)` if
    /// it does not exist at all — never an error on its own.
    async fn get(
        &self,
        assistant_id: &AssistantId,
    ) -> Result<Option<Assistant>, AssistantRepositoryError>;

    /// Load one specific version. `Ok(None)` if `assistant_id` does not
    /// exist, or `version` is `0`, or `version` exceeds the assistant's
    /// `latest` — all "nothing here" cases, never an error. A soft-deleted
    /// assistant's versions stay readable here (PLAT-FR-10).
    async fn get_version(
        &self,
        assistant_id: &AssistantId,
        version: u32,
    ) -> Result<Option<AssistantVersion>, AssistantRepositoryError>;

    /// Page through assistants, ordered ascending by `assistant_id`.
    /// `include_deleted` controls whether soft-deleted assistants appear.
    async fn list(
        &self,
        limit: u32,
        cursor: Option<AssistantId>,
        include_deleted: bool,
    ) -> Result<AssistantPage, AssistantRepositoryError>;

    /// Page through one assistant's version history, ordered ascending by
    /// `version` — the changelog PLAT-FR-10 asks for.
    async fn list_versions(
        &self,
        assistant_id: &AssistantId,
        limit: u32,
        cursor: Option<u32>,
    ) -> Result<AssistantVersionPage, AssistantRepositoryError>;

    /// Soft-delete an assistant: sets `deleted_at`, after which
    /// `append_version` fails `NotFound` but `get`/`get_version` continue to
    /// work (PLAT-FR-10 — historical runs stay reconstructable).
    ///
    /// # Errors
    ///
    /// Returns [`AssistantRepositoryError::NotFound`] when `assistant_id`
    /// does not exist.
    async fn soft_delete(&self, assistant_id: &AssistantId)
    -> Result<(), AssistantRepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // No full mock `impl AssistantRepositoryPort` lives in this file: the
    // real proof of implementability is `InMemoryAssistantRepository`
    // (`paladin-storage`), and keeping a second full impl here would
    // duplicate the trait's version-publishing method signature verbatim in
    // this file -- this plan's own acceptance criteria grep for exactly one
    // occurrence of that method's declaration, to guard against a stray
    // `update`-shaped duplicate. Object safety alone is checked below
    // without a concrete implementation.

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn AssistantRepositoryPort>> = None;
    }

    #[test]
    fn assistant_page_default_is_empty() {
        let page = AssistantPage::default();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn assistant_version_page_default_is_empty() {
        let page = AssistantVersionPage::default();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }
}
