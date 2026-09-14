//! Registered evaluators for `RetryPredicate::Custom` (D-13).
//!
//! `paladin-core`'s [`RetryPredicate`](paladin_core::platform::container::aegis::RetryPredicate)
//! enum names a `Custom(String)` variant but -- deliberately -- has no idea
//! what any given name *means*: resolving a name to behavior is
//! application-layer responsibility, owned here in `paladin-battalion`,
//! mirroring [`crate::edge_evaluator`]'s registry byte-for-byte in
//! name-substituted form (D-13, CF-01 precedent).
//!
//! An unregistered `Custom(name)` fails graph validation
//! (`crate::engine::graph::WarGraph::validate`) BEFORE any node executes,
//! naming every offender -- never silently degrading to "do not retry" or
//! any other default at runtime (FT-FR-13).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::node_error::NodeError;
use thiserror::Error;

/// Resolves a registered `RetryPredicate::Custom(name)` policy to a verdict
/// for one failed attempt.
///
/// Async by the same deliberate deviation from a sync sketch
/// [`crate::edge_evaluator::EdgeConditionEvaluator`] already makes: a future
/// predicate may need to `await` (e.g. consult an external circuit-breaker
/// service), and blocking a Tokio worker thread to do so is a house
/// anti-pattern. This trait is new in v0.10 (no `#[non_exhaustive]` register
/// burden, X-10).
#[async_trait]
pub trait RetryPredicateEvaluator: Send + Sync {
    /// Resolve whether `err` should be retried on the upcoming `attempt`
    /// (1-indexed, the attempt ABOUT to run if this returns `Ok(true)`).
    ///
    /// An `Err` fails the run naming this predicate -- it is never treated
    /// as `false`, and never defaulted to `true`.
    async fn allows(&self, err: &NodeError, attempt: u32) -> Result<bool, RetryPredicateError>;
}

/// Errors a [`RetryPredicateEvaluator`] can return.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RetryPredicateError {
    /// The predicate could not resolve a verdict.
    #[error("retry predicate '{predicate}' failed: {reason}")]
    Evaluation {
        /// The predicate's registered name.
        predicate: String,
        /// Why the predicate failed.
        reason: String,
    },
}

/// The engine-owned registry of named `RetryPredicate::Custom(name)`
/// evaluators (D-13).
///
/// Unlike [`crate::engine::dispatch_registry::DispatchRegistry`], there is
/// NO reserved-name guard here: `RetryPredicate::Custom` names collide with
/// no built-in `RetryPredicate` variant name, so there is nothing for a
/// registered name to be confused with. Registering under a name that is
/// already registered REPLACES the prior evaluator rather than erroring.
/// Name lookup is exact `String` equality: no trimming, no case folding, no
/// Unicode normalization.
///
/// `Clone` (mirroring [`crate::edge_evaluator::EdgeEvaluatorRegistry`]): a
/// `NodeSpec::Battalion` node's child run inherits the PARENT's registries
/// wholesale (Phase 23 D-21) via [`crate::engine::registries::EngineRegistries`],
/// and forwarding it into a `tokio::spawn`'d dispatch task requires an
/// owned, `'static` copy -- cheap, since every value clone is an
/// `Arc::clone`.
#[derive(Default, Clone)]
pub struct RetryPredicateRegistry {
    inner: HashMap<String, Arc<dyn RetryPredicateEvaluator>>,
}

impl RetryPredicateRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `evaluator` under `name`, exact-byte-equality keyed. A
    /// second registration under the same name replaces the first.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        evaluator: Arc<dyn RetryPredicateEvaluator>,
    ) {
        self.inner.insert(name.into(), evaluator);
    }

    /// Look up the evaluator registered under `name`, if any.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn RetryPredicateEvaluator>> {
        self.inner.get(name)
    }

    /// Whether an evaluator is registered under `name`.
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

    struct AlwaysAllow;

    #[async_trait]
    impl RetryPredicateEvaluator for AlwaysAllow {
        async fn allows(
            &self,
            _err: &NodeError,
            _attempt: u32,
        ) -> Result<bool, RetryPredicateError> {
            Ok(true)
        }
    }

    #[test]
    fn name_lookup_is_exact_byte_equality_case_sensitive() {
        let mut registry = RetryPredicateRegistry::new();
        registry.register("backoffAware", Arc::new(AlwaysAllow));

        assert!(registry.contains("backoffAware"));
        assert!(
            !registry.contains("backoffaware"),
            "lookup must be exact byte equality, not case-insensitive"
        );
        assert!(registry.get("backoffaware").is_none());
        assert!(registry.get("backoffAware").is_some());
    }

    #[test]
    fn registered_names_returns_a_byte_sorted_list() {
        let mut registry = RetryPredicateRegistry::new();
        registry.register("zeta", Arc::new(AlwaysAllow));
        registry.register("alpha", Arc::new(AlwaysAllow));
        registry.register("mid", Arc::new(AlwaysAllow));

        assert_eq!(registry.registered_names(), vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn duplicate_registration_replaces_the_prior_evaluator() {
        let mut registry = RetryPredicateRegistry::new();
        registry.register("my_predicate", Arc::new(AlwaysAllow));
        registry.register("my_predicate", Arc::new(AlwaysAllow));

        assert_eq!(registry.registered_names(), vec!["my_predicate"]);
    }
}
