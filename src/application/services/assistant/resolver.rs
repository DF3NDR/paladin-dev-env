//! Stored-assistant resolution behind the SAME [`AssistantResolver`] seam 27-01's
//! [`CodeWorkflowResolver`] already implements (D-28, D-32).
//!
//! [`StoredAssistantResolver`] resolves a stored `(assistant_id, version)` by loading the
//! version's `AssistantDefinition` and re-validating it through [`AssistantValidator`]
//! (compile is validation, D-31) -- a stored version's `AssistantDefinition` is a static,
//! serialized document; the runnable artifact it compiles to is not itself persisted, so
//! resolution recompiles it. Since a published version is immutable (D-29), the result is
//! cached forever once computed (`RwLock<HashMap<(assistant_id, version), ResolvedAssistant>>`).
//!
//! [`ChainedResolver`] tries the stored resolver first, then falls back to a code-
//! registered one -- correct because a stored id and a code-registered id can never
//! collide (`AssistantService::create` never checks the code registry itself; that 409 is
//! enforced one layer up, at the HTTP surface, `crates/paladin-web/src/assistant_controller.rs`).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use paladin_core::platform::container::assistant::{AssistantId, AssistantSource};
use paladin_core::platform::container::run::AssistantRef;
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;

use crate::application::services::run::resolver::{
    AssistantResolver, ResolveError, ResolvedAssistant, Runnable,
};

use super::validator::{AssistantValidator, Validated};

/// Cache key: an immutable published version never needs re-resolving once computed.
type CacheKey = (String, u32);

/// Resolves a stored assistant's definition, re-validating it at resolve time (D-31) and
/// caching the immutable result forever per `(assistant_id, version)`.
pub struct StoredAssistantResolver {
    repository: Arc<dyn AssistantRepositoryPort>,
    validator: Arc<AssistantValidator>,
    cache: RwLock<HashMap<CacheKey, ResolvedAssistant>>,
}

impl StoredAssistantResolver {
    /// Construct a resolver over `repository`, re-validating every resolved version
    /// through `validator`.
    pub fn new(
        repository: Arc<dyn AssistantRepositoryPort>,
        validator: Arc<AssistantValidator>,
    ) -> Self {
        Self {
            repository,
            validator,
            cache: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl AssistantResolver for StoredAssistantResolver {
    async fn resolve(
        &self,
        assistant_id: &str,
        version: Option<u32>,
    ) -> Result<ResolvedAssistant, ResolveError> {
        let unknown_assistant = || ResolveError::UnknownAssistant {
            assistant_id: assistant_id.to_string(),
        };
        let unknown_version = |v: u32| ResolveError::UnknownVersion {
            assistant_id: assistant_id.to_string(),
            version: v,
        };

        let id = AssistantId::new(assistant_id).map_err(|_| unknown_assistant())?;

        let assistant = self
            .repository
            .get(&id)
            .await
            .map_err(|_| unknown_assistant())?
            .ok_or_else(unknown_assistant)?;
        if assistant.is_deleted() {
            return Err(unknown_assistant());
        }

        let effective_version = version.unwrap_or(assistant.latest);
        if effective_version == 0 || effective_version > assistant.latest {
            return Err(unknown_version(effective_version));
        }

        let key = (assistant_id.to_string(), effective_version);
        if let Some(cached) = self
            .cache
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
        {
            return Ok(cached.clone());
        }

        let stored_version = self
            .repository
            .get_version(&id, effective_version)
            .await
            .map_err(|_| unknown_version(effective_version))?
            .ok_or_else(|| unknown_version(effective_version))?;

        // A stored version was already validated at publish time
        // (`AssistantService::create`/`publish_version`); a re-validation failure here
        // means the process's live registries changed since publish (a `Custom`
        // evaluator/predicate/handler that used to be registered no longer is) -- a real,
        // if rare, anomaly. Surfaced as `UnknownVersion` (the resolver's only vocabulary
        // for "this version cannot be run right now") rather than panicking.
        let validated = self
            .validator
            .validate(&stored_version.definition)
            .map_err(|_violations| unknown_version(effective_version))?;

        let (runnable, allowed_roles) = match validated {
            Validated::Agent(paladin, roles) => (Runnable::Agent(paladin), roles),
            Validated::Workflow(graph) => (Runnable::Workflow(graph), Vec::new()),
        };

        let resolved = ResolvedAssistant {
            reference: AssistantRef {
                assistant_id: assistant_id.to_string(),
                version: effective_version,
            },
            runnable,
            allowed_roles,
            source: AssistantSource::Stored,
        };

        self.cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, resolved.clone());
        Ok(resolved)
    }
}

/// Tries a stored resolver first, then falls back to a code-registered one (D-32).
/// Correct because ids are disjoint by construction: creating a stored assistant under a
/// code-registered id is rejected with a 409 one layer up (the HTTP surface), never here.
pub struct ChainedResolver {
    stored: Arc<dyn AssistantResolver>,
    code: Arc<dyn AssistantResolver>,
}

impl ChainedResolver {
    /// Construct a resolver trying `stored` first, then `code`.
    pub fn new(stored: Arc<dyn AssistantResolver>, code: Arc<dyn AssistantResolver>) -> Self {
        Self { stored, code }
    }
}

#[async_trait]
impl AssistantResolver for ChainedResolver {
    async fn resolve(
        &self,
        assistant_id: &str,
        version: Option<u32>,
    ) -> Result<ResolvedAssistant, ResolveError> {
        match self.stored.resolve(assistant_id, version).await {
            Ok(resolved) => Ok(resolved),
            Err(ResolveError::UnknownAssistant { .. }) => {
                self.code.resolve(assistant_id, version).await
            }
            Err(other) => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::run::resolver::CodeWorkflowResolver;
    use paladin_battalion::engine::registries::EngineRegistries;
    use paladin_battalion::engine::{EngineLimits, WarGraph};
    use paladin_core::platform::container::assistant::{
        AssistantDefinition, AssistantKind, NewAssistantVersion,
    };
    use paladin_core::platform::container::battlefield::BattlefieldSchema;
    use paladin_storage::assistant::in_memory::InMemoryAssistantRepository;

    fn empty_graph() -> Arc<WarGraph> {
        Arc::new(WarGraph::new(
            BattlefieldSchema::new(vec![]),
            EngineLimits::default(),
        ))
    }

    fn valid_workflow_body() -> serde_json::Value {
        serde_json::json!({
            "schema_version": "1",
            "entry": ["review"],
            "nodes": [{
                "id": "review",
                "kind": "gate",
                "gate": {
                    "parley": "approval",
                    "prompt_template": "Approve?",
                    "on_expire": { "type": "fail_run" },
                    "output_field": "approved"
                },
                "defer": false
            }],
            "edges": [{ "from": "review", "to": "review" }],
            "schema": {
                "fields": [{
                    "name": "approved",
                    "kind": "boolean",
                    "reducer": "last_write",
                    "default": false,
                    "required": false
                }]
            }
        })
    }

    async fn seeded(id: &str) -> (Arc<InMemoryAssistantRepository>, AssistantId) {
        let repository = Arc::new(InMemoryAssistantRepository::new());
        let assistant_id = AssistantId::new(id).unwrap();
        repository
            .create(
                &assistant_id,
                NewAssistantVersion {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Workflow,
                        body: valid_workflow_body(),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();
        (repository, assistant_id)
    }

    #[tokio::test]
    async fn resolves_latest_when_no_version_given() {
        let (repository, _id) = seeded("wf-a").await;
        let validator = Arc::new(AssistantValidator::new(Arc::new(EngineRegistries::new())));
        let resolver = StoredAssistantResolver::new(repository, validator);

        let resolved = resolver.resolve("wf-a", None).await.unwrap();
        assert_eq!(resolved.reference.version, 1);
        assert_eq!(resolved.source, AssistantSource::Stored);
        assert!(matches!(resolved.runnable, Runnable::Workflow(_)));
    }

    #[tokio::test]
    async fn unknown_assistant_id_is_unknown_assistant() {
        let repository: Arc<dyn AssistantRepositoryPort> =
            Arc::new(InMemoryAssistantRepository::new());
        let validator = Arc::new(AssistantValidator::new(Arc::new(EngineRegistries::new())));
        let resolver = StoredAssistantResolver::new(repository, validator);

        let err = resolver.resolve("nope", None).await.unwrap_err();
        assert!(matches!(err, ResolveError::UnknownAssistant { .. }));
    }

    #[tokio::test]
    async fn version_zero_and_above_latest_are_unknown_version() {
        let (repository, _id) = seeded("wf-b").await;
        let validator = Arc::new(AssistantValidator::new(Arc::new(EngineRegistries::new())));
        let resolver = StoredAssistantResolver::new(repository, validator);

        assert!(matches!(
            resolver.resolve("wf-b", Some(0)).await.unwrap_err(),
            ResolveError::UnknownVersion { version: 0, .. }
        ));
        assert!(matches!(
            resolver.resolve("wf-b", Some(2)).await.unwrap_err(),
            ResolveError::UnknownVersion { version: 2, .. }
        ));
    }

    #[tokio::test]
    async fn soft_deleted_assistant_is_unknown_assistant() {
        let (repository, id) = seeded("wf-c").await;
        repository.soft_delete(&id).await.unwrap();
        let validator = Arc::new(AssistantValidator::new(Arc::new(EngineRegistries::new())));
        let resolver = StoredAssistantResolver::new(repository, validator);

        let err = resolver.resolve("wf-c", None).await.unwrap_err();
        assert!(matches!(err, ResolveError::UnknownAssistant { .. }));
    }

    #[tokio::test]
    async fn resolve_result_is_cached_across_calls() {
        let (repository, _id) = seeded("wf-d").await;
        let validator = Arc::new(AssistantValidator::new(Arc::new(EngineRegistries::new())));
        let resolver = StoredAssistantResolver::new(repository, validator);

        let first = resolver.resolve("wf-d", Some(1)).await.unwrap();
        let second = resolver.resolve("wf-d", Some(1)).await.unwrap();
        assert_eq!(first.reference, second.reference);
    }

    #[tokio::test]
    async fn code_and_stored_ids_are_disjoint() {
        let (repository, _id) = seeded("stored-id").await;
        let validator = Arc::new(AssistantValidator::new(Arc::new(EngineRegistries::new())));
        let stored: Arc<dyn AssistantResolver> =
            Arc::new(StoredAssistantResolver::new(repository, validator));
        let code: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("code-id", empty_graph()));
        let chained = ChainedResolver::new(stored, code);

        let stored_resolved = chained.resolve("stored-id", None).await.unwrap();
        assert_eq!(stored_resolved.source, AssistantSource::Stored);
        let code_resolved = chained.resolve("code-id", None).await.unwrap();
        assert_eq!(code_resolved.source, AssistantSource::Code);

        let err = chained.resolve("neither-id", None).await.unwrap_err();
        assert!(matches!(err, ResolveError::UnknownAssistant { .. }));
    }

    #[tokio::test]
    async fn chained_resolver_falls_through_to_code_on_unknown_stored_id() {
        let repository: Arc<dyn AssistantRepositoryPort> =
            Arc::new(InMemoryAssistantRepository::new());
        let validator = Arc::new(AssistantValidator::new(Arc::new(EngineRegistries::new())));
        let stored: Arc<dyn AssistantResolver> =
            Arc::new(StoredAssistantResolver::new(repository, validator));
        let code: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf1", empty_graph()));
        let chained = ChainedResolver::new(stored, code);

        let resolved = chained.resolve("wf1", None).await.unwrap();
        assert_eq!(resolved.source, AssistantSource::Code);
    }
}
