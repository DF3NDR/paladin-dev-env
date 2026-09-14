//! `EngineRegistries` — the one bundle a [`crate::engine::graph::WarGraph::validate`]
//! call and a [`crate::engine::WarEngine`] carry, rather than three separate
//! positional registry parameters (D-13, D-30).
//!
//! This is a new-in-0.10 type: `WarGraph::validate`'s signature change from
//! `(custom_dispatch, edge_evaluators: &EdgeEvaluatorRegistry)` to
//! `(custom_dispatch, registries: &EngineRegistries)` is a deliberate zero
//! (plan 25-14's own notes record the sentence), not a X-10 semver-break
//! row -- there is no shipped `v0.9` caller of the old two-registry
//! signature to preserve.
//!
//! A `NodeSpec::Battalion` child inherits this bundle WHOLESALE from its
//! parent (Phase 23 D-21): `WarGraph::validate_battalion_children` passes
//! the SAME `&EngineRegistries` down into every child's own
//! `validate_non_recursive` call, so a `Custom` name registered once on the
//! parent `WarEngine` resolves inside every nested child graph with no
//! re-registration.

use std::collections::HashMap;
use std::sync::Arc;

use crate::edge_evaluator::EdgeEvaluatorRegistry;
use crate::engine::StructuredSchema;
use crate::error_handler::ErrorHandlerRegistry;
use crate::retry_predicate::RetryPredicateRegistry;

/// The bundle of every named-registration registry `WarGraph::validate`
/// resolves a graph's `Custom`/`Registered` names against (D-13):
/// `EdgeCondition::Custom` (BUG-01, CF-01), `RetryPredicate::Custom` and
/// `ErrorHandlerSpec::Custom` (plan 25-03), and `SchemaRef::Registered`
/// (D-29, plan 26-18).
#[derive(Default, Clone)]
pub struct EngineRegistries {
    /// Registered `EdgeCondition::Custom` evaluators (BUG-01, CF-01).
    pub edge_evaluators: EdgeEvaluatorRegistry,
    /// Registered `RetryPredicate::Custom` evaluators (D-13).
    pub retry_predicates: RetryPredicateRegistry,
    /// Registered `ErrorHandlerSpec::Custom` handlers (D-13).
    pub error_handlers: ErrorHandlerRegistry,
    /// Registered `SchemaRef::Registered(name)` schemas (D-29, RT-FR-19,
    /// plan 26-18), consulted by `WarGraph::validate` (an unregistered name
    /// is `EngineError::UnregisteredOutputSchema`) and resolved by
    /// `engine::superstep`'s Paladin dispatch (`StructuredSchema::to_json_schema`)
    /// once validation has already proven the name present. A plain
    /// `HashMap` rather than a dedicated registry type (mirroring
    /// `RetryPredicateRegistry`'s own shape): no reserved-name guard is
    /// needed here either, since a `SchemaRef::Registered` name collides
    /// with no other named-registration vocabulary.
    pub output_schemas: HashMap<String, Arc<dyn StructuredSchema>>,
}

impl EngineRegistries {
    /// Construct an empty bundle (every registry starts with no
    /// registrations).
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_evaluator::{EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorError};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct AlwaysTrue;

    #[async_trait]
    impl EdgeConditionEvaluator for AlwaysTrue {
        async fn evaluate(
            &self,
            _output: &str,
            _ctx: &EdgeContext<'_>,
        ) -> Result<bool, EdgeEvaluatorError> {
            Ok(true)
        }
    }

    #[test]
    fn default_bundle_has_empty_registries() {
        let registries = EngineRegistries::new();
        assert!(registries.edge_evaluators.registered_names().is_empty());
        assert!(registries.retry_predicates.registered_names().is_empty());
        assert!(registries.error_handlers.registered_names().is_empty());
    }

    #[test]
    fn clone_is_independent_arc_backed_snapshot() {
        let mut registries = EngineRegistries::new();
        registries
            .edge_evaluators
            .register("x", Arc::new(AlwaysTrue));
        let cloned = registries.clone();
        assert!(cloned.edge_evaluators.contains("x"));
    }
}
