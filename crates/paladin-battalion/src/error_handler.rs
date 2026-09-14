//! Registered handlers for `ErrorHandlerSpec::Custom` (D-13).
//!
//! `paladin-core`'s [`ErrorHandlerSpec`](paladin_core::platform::container::aegis::ErrorHandlerSpec)
//! enum names a `Custom(String)` variant but -- deliberately -- has no idea
//! what any given name *means*: resolving a name to behavior is
//! application-layer responsibility, owned here in `paladin-battalion`,
//! mirroring [`crate::edge_evaluator`]'s registry byte-for-byte in
//! name-substituted form (D-13, CF-01 precedent).
//!
//! An unregistered `Custom(name)` fails graph validation
//! (`crate::engine::graph::WarGraph::validate`) BEFORE any node executes,
//! naming every offender. This plan (25-03) owns registration and
//! validation only -- dispatching a resolved handler at run time is plan
//! 25-10/11's job.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::battlefield::Battlefield;
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::node_error::NodeError;

/// Resolves a registered `ErrorHandlerSpec::Custom(name)` handler to a
/// compensating [`Directive`] for one failed node execution.
///
/// Async, mirroring [`crate::edge_evaluator::EdgeConditionEvaluator`] and
/// [`crate::retry_predicate::RetryPredicateEvaluator`]: a handler may need
/// to `await` (e.g. write a compensating record to an external system).
/// This trait is new in v0.10 (no `#[non_exhaustive]` register burden,
/// X-10).
#[async_trait]
pub trait ErrorHandler: Send + Sync {
    /// Handle `err`, observing the run's current typed state `state`, and
    /// return the compensating [`Directive`] the engine should treat as this
    /// node's own result. Returning `Err` fails the run, naming this
    /// handler and the underlying error -- a handler failure is never
    /// silently swallowed.
    async fn handle(&self, err: &NodeError, state: &Battlefield) -> Result<Directive, NodeError>;
}

/// The engine-owned registry of named `ErrorHandlerSpec::Custom(name)`
/// handlers (D-13).
///
/// Unlike [`crate::engine::dispatch_registry::DispatchRegistry`], there is
/// NO reserved-name guard here: `ErrorHandlerSpec::Custom` names collide
/// with no built-in `ErrorHandlerSpec` variant name, so there is nothing
/// for a registered name to be confused with. Registering under a name
/// that is already registered REPLACES the prior handler rather than
/// erroring. Name lookup is exact `String` equality: no trimming, no case
/// folding, no Unicode normalization.
///
/// `Clone` (mirroring [`crate::edge_evaluator::EdgeEvaluatorRegistry`]): a
/// `NodeSpec::Battalion` node's child run inherits the PARENT's registries
/// wholesale (Phase 23 D-21) via [`crate::engine::registries::EngineRegistries`],
/// and forwarding it into a `tokio::spawn`'d dispatch task requires an
/// owned, `'static` copy -- cheap, since every value clone is an
/// `Arc::clone`.
#[derive(Default, Clone)]
pub struct ErrorHandlerRegistry {
    inner: HashMap<String, Arc<dyn ErrorHandler>>,
}

impl ErrorHandlerRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `handler` under `name`, exact-byte-equality keyed. A second
    /// registration under the same name replaces the first.
    pub fn register(&mut self, name: impl Into<String>, handler: Arc<dyn ErrorHandler>) {
        self.inner.insert(name.into(), handler);
    }

    /// Look up the handler registered under `name`, if any.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn ErrorHandler>> {
        self.inner.get(name)
    }

    /// Whether a handler is registered under `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }

    /// Every registered name, sorted (byte order, never locale collation).
    pub fn registered_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.inner.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battlefield::{BattlefieldSchema, StateDelta};

    struct NoopHandler;

    #[async_trait]
    impl ErrorHandler for NoopHandler {
        async fn handle(
            &self,
            _err: &NodeError,
            _state: &Battlefield,
        ) -> Result<Directive, NodeError> {
            Ok(StateDelta::new().into())
        }
    }

    fn empty_battlefield() -> Battlefield {
        Battlefield::initialize(BattlefieldSchema::new(vec![]), &StateDelta::new())
            .expect("empty schema initializes")
    }

    #[tokio::test]
    async fn name_lookup_is_exact_byte_equality_case_sensitive() {
        let mut registry = ErrorHandlerRegistry::new();
        registry.register("compensate", Arc::new(NoopHandler));

        assert!(registry.contains("compensate"));
        assert!(
            !registry.contains("Compensate"),
            "lookup must be exact byte equality, not case-insensitive"
        );
        assert!(registry.get("Compensate").is_none());
        assert!(registry.get("compensate").is_some());

        // Prove the resolved handler is actually callable.
        let bf = empty_battlefield();
        let err = NodeError {
            node_id: paladin_core::platform::container::waypoint::NodeId::new("n"),
            attempt: 1,
            transience: paladin_core::platform::container::transience::Transience::Transient,
            source: paladin_core::platform::container::node_error::NodeErrorSource::Function {
                message: "boom".to_string(),
            },
        };
        let handler = registry.get("compensate").unwrap().clone();
        assert!(handler.handle(&err, &bf).await.is_ok());
    }

    #[test]
    fn registered_names_returns_a_byte_sorted_list() {
        let mut registry = ErrorHandlerRegistry::new();
        registry.register("zeta", Arc::new(NoopHandler));
        registry.register("alpha", Arc::new(NoopHandler));
        registry.register("mid", Arc::new(NoopHandler));

        assert_eq!(registry.registered_names(), vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn duplicate_registration_replaces_the_prior_handler() {
        let mut registry = ErrorHandlerRegistry::new();
        registry.register("my_handler", Arc::new(NoopHandler));
        registry.register("my_handler", Arc::new(NoopHandler));

        assert_eq!(registry.registered_names(), vec!["my_handler"]);
    }
}
