//! Assistant Admin Port — validate-then-publish assistant definitions (D-31, D-46)
//!
//! [`AssistantAdminPort`] is the whole surface the `paladin-web` assistant routes
//! (`crates/paladin-web/src/assistant_controller.rs`) drive: create, publish a new
//! version, read, list and soft-delete. There is deliberately **no update method** —
//! immutability holds because the code to violate it does not exist, mirroring
//! [`crate::output::assistant_repository_port::AssistantRepositoryPort`]'s own D-29
//! discipline one layer down.
//!
//! An implementation (`src/application/services/assistant/service.rs`'s
//! `AssistantService`) validates a definition BEFORE any repository call — compile is
//! validation (D-31): `Agent` bodies validate structurally, `Workflow` bodies validate by
//! deserialising to a `WarGraphDoc` and calling `compile()` against the server's live
//! engine registries. A rejected definition is reported as
//! [`AssistantAdminError::Invalid`], carrying a non-empty, machine-readable
//! [`ValidationViolation`] list — nothing is ever persisted on that path.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use paladin_core::platform::container::assistant::{
    Assistant, AssistantDefinition, AssistantId, AssistantVersion,
};

use crate::output::assistant_repository_port::{AssistantPage, AssistantVersionPage};

/// One machine-readable validation failure (D-31): the published `400` `details` shape
/// every client parses, rendered verbatim as one entry of a JSON array under
/// `error.details.violations`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationViolation {
    /// A JSON-Pointer-shaped path into the rejected definition body, e.g.
    /// `"/system_prompt"` or `"/edges/writer->review/condition"`. `"/"` for a
    /// whole-document failure (an empty body, an unparseable shape).
    pub path: String,
    /// A stable, machine-readable violation code, e.g. `"missing_field"`,
    /// `"credential_field_forbidden"`, `"unregistered_edge_evaluator"`.
    pub code: String,
    /// A human-readable explanation of the violation.
    pub message: String,
}

impl ValidationViolation {
    /// Construct a violation from its three fields.
    pub fn new(
        path: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            code: code.into(),
            message: message.into(),
        }
    }
}

/// The fields a caller supplies to create or publish a version through
/// [`AssistantAdminPort`]. The repository assigns `version` and `created_at`.
#[derive(Debug, Clone)]
pub struct PublishAssistant {
    /// The definition to validate and, on success, publish.
    pub definition: AssistantDefinition,
    /// Who is publishing this version, if known (PLAT-FR-10 audit trail).
    pub created_by: Option<String>,
    /// An optional publish-time note (PLAT-FR-10 audit trail).
    pub note: Option<String>,
}

/// Errors returned by [`AssistantAdminPort`] methods (X-06 — structured, never a bare
/// `bool`/`String`). `#[non_exhaustive]`: a future variant must not be a breaking change.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssistantAdminError {
    /// The submitted definition failed validation (D-31) — nothing was persisted.
    #[error("assistant definition failed validation ({} violation(s))", violations.len())]
    Invalid {
        /// Every violation found, never empty.
        violations: Vec<ValidationViolation>,
    },
    /// No assistant exists with the given id.
    #[error("assistant not found: {assistant_id}")]
    NotFound {
        /// The requested assistant id.
        assistant_id: AssistantId,
    },
    /// [`AssistantAdminPort::create`] was called with an id that already exists.
    #[error("assistant already exists: {assistant_id}")]
    AlreadyExists {
        /// The already-existing assistant id.
        assistant_id: AssistantId,
    },
    /// A version-numbering race exhausted the repository's own retry budget.
    #[error("version conflict publishing assistant {assistant_id}")]
    VersionConflict {
        /// The assistant whose version numbering conflicted.
        assistant_id: AssistantId,
    },
    /// The underlying backend failed.
    #[error("assistant admin backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Port trait for validating and publishing [`Assistant`] definitions (D-28, D-31, D-46).
///
/// # No update method exists, by design (D-29)
///
/// This trait deliberately has no method that rewrites an existing version's
/// `definition` — publishing always means [`Self::create`] (a brand-new assistant) or
/// [`Self::append_version`]... (there is no `append_version` here either: publishing an
/// EXISTING assistant's next version is [`Self::publish_version`]). Neither ever accepts a
/// target `version` to overwrite.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: assistants are published, read and resolved
/// concurrently across HTTP handlers and the run-submission path.
#[async_trait]
pub trait AssistantAdminPort: Send + Sync {
    /// Validate `publish.definition`, then create a brand-new assistant with its first
    /// version (`version == 1`, `latest == 1`).
    ///
    /// # Errors
    ///
    /// Returns [`AssistantAdminError::Invalid`] if the definition fails validation (D-31)
    /// — nothing is persisted. Returns [`AssistantAdminError::AlreadyExists`] if
    /// `assistant_id` already exists.
    async fn create(
        &self,
        assistant_id: &AssistantId,
        publish: PublishAssistant,
    ) -> Result<AssistantVersion, AssistantAdminError>;

    /// Validate `publish.definition`, then publish a new version onto an existing
    /// assistant: the returned version's number is `latest + 1`, and `latest` advances to
    /// match.
    ///
    /// # Errors
    ///
    /// Returns [`AssistantAdminError::Invalid`] if the definition fails validation (D-31)
    /// — nothing is persisted. Returns [`AssistantAdminError::NotFound`] when
    /// `assistant_id` does not exist or is soft-deleted, and
    /// [`AssistantAdminError::VersionConflict`] if the repository's own retry budget for a
    /// concurrent-publish race is exhausted.
    async fn publish_version(
        &self,
        assistant_id: &AssistantId,
        publish: PublishAssistant,
    ) -> Result<AssistantVersion, AssistantAdminError>;

    /// Load an assistant by id, including a soft-deleted one. `Ok(None)` if it does not
    /// exist at all — never an error on its own.
    async fn get(
        &self,
        assistant_id: &AssistantId,
    ) -> Result<Option<Assistant>, AssistantAdminError>;

    /// Load one specific version. `Ok(None)` if `assistant_id` does not exist, `version`
    /// is `0`, or `version` exceeds the assistant's `latest` — all "nothing here" cases,
    /// never an error. A soft-deleted assistant's versions stay readable here
    /// (PLAT-FR-10).
    async fn get_version(
        &self,
        assistant_id: &AssistantId,
        version: u32,
    ) -> Result<Option<AssistantVersion>, AssistantAdminError>;

    /// Page through assistants, ordered ascending by `assistant_id`. `include_deleted`
    /// controls whether soft-deleted assistants appear.
    async fn list(
        &self,
        limit: u32,
        cursor: Option<AssistantId>,
        include_deleted: bool,
    ) -> Result<AssistantPage, AssistantAdminError>;

    /// Page through one assistant's version history, ordered ascending by `version` — the
    /// changelog PLAT-FR-10 asks for.
    async fn list_versions(
        &self,
        assistant_id: &AssistantId,
        limit: u32,
        cursor: Option<u32>,
    ) -> Result<AssistantVersionPage, AssistantAdminError>;

    /// Soft-delete an assistant: after this call `publish_version` fails `NotFound` but
    /// `get`/`get_version` continue to work (PLAT-FR-10 — historical runs stay
    /// reconstructable).
    ///
    /// # Errors
    ///
    /// Returns [`AssistantAdminError::NotFound`] when `assistant_id` does not exist.
    async fn delete(&self, assistant_id: &AssistantId) -> Result<(), AssistantAdminError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn AssistantAdminPort>> = None;
    }

    #[test]
    fn validation_violation_new_sets_fields() {
        let v = ValidationViolation::new("/system_prompt", "empty", "must not be empty");
        assert_eq!(v.path, "/system_prompt");
        assert_eq!(v.code, "empty");
        assert_eq!(v.message, "must not be empty");
    }

    #[test]
    fn validation_violation_round_trips_through_serde_json() {
        let v = ValidationViolation::new("/", "missing_field", "body must not be empty");
        let json = serde_json::to_string(&v).unwrap();
        let restored: ValidationViolation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, v);
    }
}
