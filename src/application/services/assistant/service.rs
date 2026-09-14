//! `AssistantService` — the facade [`AssistantAdminPort`] implementation (D-31).
//!
//! Validates a definition through [`AssistantValidator`] BEFORE any
//! [`AssistantRepositoryPort`] call: an invalid definition never reaches the repository,
//! so "compile is validation" (D-31) holds all the way down to storage. Version-publish
//! retry-on-conflict is already handled by
//! [`AssistantRepositoryPort::append_version`]'s own implementation (27-09) -- this
//! service surfaces an exhausted retry budget typed, it does not retry a second time
//! itself.

use std::sync::Arc;

use async_trait::async_trait;

use paladin_core::platform::container::assistant::{
    Assistant, AssistantId, AssistantVersion, NewAssistantVersion,
};
use paladin_ports::input::assistant_admin_port::{
    AssistantAdminError, AssistantAdminPort, PublishAssistant,
};
use paladin_ports::output::assistant_repository_port::{
    AssistantPage, AssistantRepositoryError, AssistantRepositoryPort, AssistantVersionPage,
};

use super::validator::AssistantValidator;

/// Implements [`AssistantAdminPort`] over an [`AssistantRepositoryPort`] and an
/// [`AssistantValidator`].
pub struct AssistantService {
    repository: Arc<dyn AssistantRepositoryPort>,
    validator: Arc<AssistantValidator>,
}

impl AssistantService {
    /// Construct a service over `repository`, validating every publish through
    /// `validator` first.
    pub fn new(
        repository: Arc<dyn AssistantRepositoryPort>,
        validator: Arc<AssistantValidator>,
    ) -> Self {
        Self {
            repository,
            validator,
        }
    }
}

fn map_repository_error(err: AssistantRepositoryError) -> AssistantAdminError {
    match err {
        AssistantRepositoryError::NotFound { assistant_id } => {
            AssistantAdminError::NotFound { assistant_id }
        }
        AssistantRepositoryError::AlreadyExists { assistant_id } => {
            AssistantAdminError::AlreadyExists { assistant_id }
        }
        AssistantRepositoryError::VersionConflict { assistant_id, .. } => {
            AssistantAdminError::VersionConflict { assistant_id }
        }
        other => AssistantAdminError::Backend {
            source: Box::new(other),
        },
    }
}

#[async_trait]
impl AssistantAdminPort for AssistantService {
    async fn create(
        &self,
        assistant_id: &AssistantId,
        publish: PublishAssistant,
    ) -> Result<AssistantVersion, AssistantAdminError> {
        if let Err(violations) = self.validator.validate(&publish.definition) {
            return Err(AssistantAdminError::Invalid { violations });
        }
        let new = NewAssistantVersion {
            definition: publish.definition,
            created_by: publish.created_by,
            note: publish.note,
        };
        self.repository
            .create(assistant_id, new)
            .await
            .map_err(map_repository_error)
    }

    async fn publish_version(
        &self,
        assistant_id: &AssistantId,
        publish: PublishAssistant,
    ) -> Result<AssistantVersion, AssistantAdminError> {
        if let Err(violations) = self.validator.validate(&publish.definition) {
            return Err(AssistantAdminError::Invalid { violations });
        }
        let new = NewAssistantVersion {
            definition: publish.definition,
            created_by: publish.created_by,
            note: publish.note,
        };
        self.repository
            .append_version(assistant_id, new)
            .await
            .map_err(map_repository_error)
    }

    async fn get(
        &self,
        assistant_id: &AssistantId,
    ) -> Result<Option<Assistant>, AssistantAdminError> {
        self.repository
            .get(assistant_id)
            .await
            .map_err(map_repository_error)
    }

    async fn get_version(
        &self,
        assistant_id: &AssistantId,
        version: u32,
    ) -> Result<Option<AssistantVersion>, AssistantAdminError> {
        self.repository
            .get_version(assistant_id, version)
            .await
            .map_err(map_repository_error)
    }

    async fn list(
        &self,
        limit: u32,
        cursor: Option<AssistantId>,
        include_deleted: bool,
    ) -> Result<AssistantPage, AssistantAdminError> {
        self.repository
            .list(limit, cursor, include_deleted)
            .await
            .map_err(map_repository_error)
    }

    async fn list_versions(
        &self,
        assistant_id: &AssistantId,
        limit: u32,
        cursor: Option<u32>,
    ) -> Result<AssistantVersionPage, AssistantAdminError> {
        self.repository
            .list_versions(assistant_id, limit, cursor)
            .await
            .map_err(map_repository_error)
    }

    async fn delete(&self, assistant_id: &AssistantId) -> Result<(), AssistantAdminError> {
        self.repository
            .soft_delete(assistant_id)
            .await
            .map_err(map_repository_error)
    }
}
