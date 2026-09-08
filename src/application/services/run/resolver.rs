//! Assistant resolver seam (D-33's predecessor, scoped to this tracer
//! slice): resolves an assistant reference to a runnable engine artifact
//! without [`super::submission::RunSubmissionService`] or the web layer
//! needing to know how resolution works.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

use paladin_battalion::engine::WarGraph;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::run::AssistantRef;
use paladin_core::platform::container::user::UserRole;

/// What an assistant resolves to.
///
/// Only `Workflow` is exercised by this plan's tracer; `Agent` is declared
/// now so plan 27-12's stored-assistant resolver does not need a breaking
/// change to this enum (D-28).
#[derive(Clone)]
pub enum Runnable {
    /// A `WarGraph` this run drives through `WarEngine::start`/`resume`.
    Workflow(Arc<WarGraph>),
    /// A single Paladin agent.
    Agent(Arc<Paladin>),
}

// `WarGraph` (`paladin-battalion`) does not implement `Debug`, so this is
// hand-written rather than derived -- deliberately shallow (never prints
// graph/agent internals) since a `Runnable` may end up in a log line.
impl std::fmt::Debug for Runnable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Runnable::Workflow(_) => f.write_str("Runnable::Workflow(..)"),
            Runnable::Agent(_) => f.write_str("Runnable::Agent(..)"),
        }
    }
}

/// The result of resolving an assistant reference.
#[derive(Clone)]
pub struct ResolvedAssistant {
    /// The concrete, frozen assistant reference (`assistant_id` + version).
    pub reference: AssistantRef,
    /// What to run.
    pub runnable: Runnable,
    /// Roles permitted to invoke this assistant; empty means any
    /// authenticated caller (D-46).
    pub allowed_roles: Vec<UserRole>,
}

impl std::fmt::Debug for ResolvedAssistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedAssistant")
            .field("reference", &self.reference)
            .field("runnable", &self.runnable)
            .field("allowed_roles", &self.allowed_roles)
            .finish()
    }
}

/// Errors returned by [`AssistantResolver::resolve`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveError {
    /// No assistant is registered under this id.
    #[error("unknown assistant: {assistant_id}")]
    UnknownAssistant {
        /// The requested assistant id.
        assistant_id: String,
    },
    /// The assistant exists, but not at the requested version.
    #[error("unknown version {version} for assistant {assistant_id}")]
    UnknownVersion {
        /// The requested assistant id.
        assistant_id: String,
        /// The requested (unknown) version.
        version: u32,
    },
}

/// The seam every assistant source plugs into: code-registered today
/// ([`CodeWorkflowResolver`]), a stored, database-backed implementation
/// plus a `ChainedResolver` behind this SAME trait in plan 27-12.
#[async_trait]
pub trait AssistantResolver: Send + Sync {
    /// Resolve `assistant_id` (at `version`, or `latest` when `None`) to a
    /// runnable artifact.
    async fn resolve(
        &self,
        assistant_id: &str,
        version: Option<u32>,
    ) -> Result<ResolvedAssistant, ResolveError>;
}

/// This slice's only [`AssistantResolver`] implementation: a `HashMap` of
/// code-registered [`WarGraph`]s, version fixed at `1` (D-32).
#[derive(Clone, Default)]
pub struct CodeWorkflowResolver {
    graphs: HashMap<String, Arc<WarGraph>>,
}

impl CodeWorkflowResolver {
    /// Construct an empty resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a workflow graph under `assistant_id` (always resolved at
    /// version `1`).
    pub fn register(mut self, assistant_id: impl Into<String>, graph: Arc<WarGraph>) -> Self {
        self.graphs.insert(assistant_id.into(), graph);
        self
    }
}

#[async_trait]
impl AssistantResolver for CodeWorkflowResolver {
    async fn resolve(
        &self,
        assistant_id: &str,
        version: Option<u32>,
    ) -> Result<ResolvedAssistant, ResolveError> {
        if let Some(v) = version
            && v != 1
        {
            return Err(ResolveError::UnknownVersion {
                assistant_id: assistant_id.to_string(),
                version: v,
            });
        }
        let graph = self.graphs.get(assistant_id).cloned().ok_or_else(|| {
            ResolveError::UnknownAssistant {
                assistant_id: assistant_id.to_string(),
            }
        })?;
        Ok(ResolvedAssistant {
            reference: AssistantRef {
                assistant_id: assistant_id.to_string(),
                version: 1,
            },
            runnable: Runnable::Workflow(graph),
            allowed_roles: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_battalion::engine::EngineLimits;
    use paladin_core::platform::container::battlefield::BattlefieldSchema;

    fn empty_graph() -> Arc<WarGraph> {
        Arc::new(WarGraph::new(
            BattlefieldSchema::new(vec![]),
            EngineLimits::default(),
        ))
    }

    #[tokio::test]
    async fn resolve_returns_registered_graph_at_version_one() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let resolved = resolver.resolve("wf1", None).await.unwrap();
        assert_eq!(resolved.reference.assistant_id, "wf1");
        assert_eq!(resolved.reference.version, 1);
        assert!(matches!(resolved.runnable, Runnable::Workflow(_)));
    }

    #[tokio::test]
    async fn resolve_explicit_version_one_succeeds() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let resolved = resolver.resolve("wf1", Some(1)).await.unwrap();
        assert_eq!(resolved.reference.version, 1);
    }

    #[tokio::test]
    async fn resolve_unknown_assistant_errors() {
        let resolver = CodeWorkflowResolver::new();
        let err = resolver.resolve("nope", None).await.unwrap_err();
        assert!(matches!(err, ResolveError::UnknownAssistant { .. }));
    }

    #[tokio::test]
    async fn resolve_unknown_version_errors() {
        let resolver = CodeWorkflowResolver::new().register("wf1", empty_graph());
        let err = resolver.resolve("wf1", Some(2)).await.unwrap_err();
        assert!(matches!(err, ResolveError::UnknownVersion { .. }));
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn AssistantResolver>> = None;
    }
}
