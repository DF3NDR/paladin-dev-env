//! `DocGraphRegistry` — resolves a suspended thread's runnable graph from the run row's
//! frozen `(assistant_id, version)`, replacing [`super::super::parley::registry::GraphRegistry`]'s
//! fingerprint-only lookup behind the SAME [`GraphResolver`] seam (D-33).
//!
//! A `WarGraphDoc`-defined workflow assistant is never registered by fingerprint anywhere:
//! [`DocGraphRegistry::resolve`] instead reads the thread's currently active
//! [`Run`](paladin_core::platform::container::run::Run) row (`RunRepositoryPort::active_run_for_thread`),
//! takes its frozen `assistant` reference, and resolves THAT through an
//! [`AssistantResolver`] -- the same resolver `RunSubmissionService` and
//! `RunWorkerPool` already use, so a resume sees exactly the version the run itself is
//! pinned to, never a re-resolved `latest` (D-30's freeze-at-submit guarantee extends to
//! resume).

use std::sync::Arc;

use async_trait::async_trait;

use paladin_battalion::engine::WarGraph;
use paladin_core::platform::container::waypoint::{GraphFingerprint, ThreadId};
use paladin_ports::output::run_repository_port::RunRepositoryPort;

use crate::application::services::parley::adapter::GraphResolver;
use crate::application::services::run::resolver::{AssistantResolver, Runnable};

/// Implements [`GraphResolver`] by reading a thread's active run's frozen assistant
/// reference and resolving it through an [`AssistantResolver`] -- `fingerprint` is
/// ignored (D-33: a stored/code-registered graph is looked up by the run's OWN
/// `(assistant_id, version)`, not by re-deriving a fingerprint).
pub struct DocGraphRegistry {
    run_repository: Arc<dyn RunRepositoryPort>,
    resolver: Arc<dyn AssistantResolver>,
}

impl DocGraphRegistry {
    /// Construct a registry resolving a thread's active run through `run_repository`, and
    /// the run's own assistant reference through `resolver`.
    pub fn new(
        run_repository: Arc<dyn RunRepositoryPort>,
        resolver: Arc<dyn AssistantResolver>,
    ) -> Self {
        Self {
            run_repository,
            resolver,
        }
    }
}

#[async_trait]
impl GraphResolver for DocGraphRegistry {
    async fn resolve(
        &self,
        thread: &ThreadId,
        _fingerprint: &GraphFingerprint,
    ) -> Option<Arc<WarGraph>> {
        let run = self
            .run_repository
            .active_run_for_thread(thread)
            .await
            .ok()
            .flatten()?;
        let resolved = self
            .resolver
            .resolve(&run.assistant.assistant_id, Some(run.assistant.version))
            .await
            .ok()?;
        match resolved.runnable {
            Runnable::Workflow(graph) => Some(graph),
            // An Agent-kind assistant has no WarGraph; a `ParleyPort` resume is only
            // meaningful for a workflow suspended on a Gate node.
            Runnable::Agent(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::run::resolver::{
        AssistantResolver, ResolveError, ResolvedAssistant,
    };
    use paladin_battalion::engine::{EngineLimits, WarGraph};
    use paladin_core::platform::container::assistant::AssistantSource;
    use paladin_core::platform::container::battlefield::BattlefieldSchema;
    use paladin_core::platform::container::run::{AssistantRef, Run, RunId};
    use paladin_storage::run::in_memory::InMemoryRunRepository;

    fn empty_graph() -> Arc<WarGraph> {
        Arc::new(WarGraph::new(
            BattlefieldSchema::new(vec![]),
            EngineLimits::default(),
        ))
    }

    struct StubResolver {
        graph: Arc<WarGraph>,
    }

    #[async_trait]
    impl AssistantResolver for StubResolver {
        async fn resolve(
            &self,
            assistant_id: &str,
            version: Option<u32>,
        ) -> Result<ResolvedAssistant, ResolveError> {
            Ok(ResolvedAssistant {
                reference: AssistantRef {
                    assistant_id: assistant_id.to_string(),
                    version: version.unwrap_or(1),
                },
                runnable: Runnable::Workflow(Arc::clone(&self.graph)),
                allowed_roles: Vec::new(),
                source: AssistantSource::Stored,
            })
        }
    }

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    #[tokio::test]
    async fn resume_finds_graph_via_doc_registry() {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let t = thread("doc-registry-resume");
        let run = Run::new(
            RunId::new_v7(),
            t.clone(),
            AssistantRef {
                assistant_id: "wf1".to_string(),
                version: 3,
            },
            serde_json::json!({}),
        );
        repository.insert(&run).await.unwrap();

        let graph = empty_graph();
        let resolver: Arc<dyn AssistantResolver> = Arc::new(StubResolver {
            graph: Arc::clone(&graph),
        });
        let doc_registry = DocGraphRegistry::new(Arc::clone(&repository), resolver);

        let unregistered = GraphFingerprint::from_canonical_bytes(b"unused");
        let resolved = doc_registry.resolve(&t, &unregistered).await;
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().fingerprint(), graph.fingerprint());
    }

    #[tokio::test]
    async fn no_active_run_resolves_to_none() {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let graph = empty_graph();
        let resolver: Arc<dyn AssistantResolver> = Arc::new(StubResolver { graph });
        let doc_registry = DocGraphRegistry::new(repository, resolver);

        let t = thread("no-active-run");
        let unregistered = GraphFingerprint::from_canonical_bytes(b"unused");
        assert!(doc_registry.resolve(&t, &unregistered).await.is_none());
    }
}
