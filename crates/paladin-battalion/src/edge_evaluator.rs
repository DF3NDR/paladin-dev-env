//! Registered evaluators for `EdgeCondition::Custom` (BUG-01, CF-01).
//!
//! `paladin-core`'s [`EdgeCondition`] enum names a `Custom(String)` variant
//! but -- deliberately -- has no idea what any given name *means*: resolving
//! a name to behavior is application-layer responsibility, owned here in
//! `paladin-battalion`, mirroring the house pattern
//! [`crate::engine::dispatch_registry::DispatchRegistry`] already
//! established for `DispatchRule::Custom` (D-01).
//!
//! Before this module existed, BOTH consumers of `EdgeCondition::Custom` --
//! `campaign_service.rs`'s `CampaignExecutionService` and
//! `engine/superstep.rs`'s `WarEngine` -- warned and then silently evaluated
//! every `Custom` condition as `Ok(true)`, corrupting conditional routing
//! with no signal to the caller (BUG-01). This module's mechanism replaces
//! both placeholder sites with a fail-closed contract: an unregistered
//! `Custom(name)` fails graph/campaign validation, BEFORE any node executes,
//! naming every offender; a registered evaluator's verdict routes the edge;
//! and a registered evaluator's `Err` fails the run, never defaulting either
//! branch (CF-FR-02/03/04).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::battlefield::Battlefield;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use thiserror::Error;

/// The context a registered [`EdgeConditionEvaluator`] sees when it is
/// asked to resolve an `EdgeCondition::Custom` edge (D-02).
///
/// `battlefield` is `Some` on the `WarEngine` path and `None` on the legacy
/// `CampaignExecutionService` path -- the legacy path has no typed state to
/// offer, only the source Paladin's raw output string (passed separately as
/// `evaluate`'s `output` argument).
///
/// `thread` and `superstep` are carried so a future evaluator (CF-05's
/// `LlmDecisionEvaluator`, D-24) can memoize one call per decision per
/// superstep without a further change to `superstep.rs`; both are `None` on
/// the legacy path, which has no thread or superstep concept.
#[derive(Debug)]
pub struct EdgeContext<'a> {
    /// The edge's source node.
    pub source: &'a NodeId,
    /// The edge's target node.
    pub target: &'a NodeId,
    /// The engine's typed state, when evaluated on the `WarEngine` path.
    pub battlefield: Option<&'a Battlefield>,
    /// The run's thread id, when evaluated on the `WarEngine` path.
    pub thread: Option<&'a ThreadId>,
    /// The superstep at which the source node completed, when evaluated on
    /// the `WarEngine` path.
    pub superstep: Option<u64>,
}

/// Resolves a registered `EdgeCondition::Custom(name)` edge to a verdict.
///
/// Async by deliberate deviation from PRD 02's sync sketch (D-02): CF-05's
/// `LlmDecisionEvaluator` must `await` an `LlmPort` call, and blocking a
/// Tokio worker thread to do so is a house anti-pattern. This trait is new
/// in v0.10 (no `#[non_exhaustive]` register burden, X-10).
#[async_trait]
pub trait EdgeConditionEvaluator: Send + Sync {
    /// Resolve this edge's verdict. `output` is the source Paladin's output
    /// string on the legacy `CampaignExecutionService` path; on the
    /// `WarEngine` path it is the source node's `output_field` value (empty
    /// string if unset yet) when the source is a `NodeSpec::Paladin` node,
    /// else the canonical Battlefield JSON the engine's built-in
    /// `Contains`/`Regex` conditions already render.
    ///
    /// An `Err` fails the run naming the edge and this evaluator (CF-FR-03)
    /// -- it is never treated as `false`, and never defaulted to `true`.
    async fn evaluate(
        &self,
        output: &str,
        ctx: &EdgeContext<'_>,
    ) -> Result<bool, EdgeEvaluatorError>;
}

/// Errors an [`EdgeConditionEvaluator`] can return.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EdgeEvaluatorError {
    /// The evaluator could not resolve a verdict.
    #[error("evaluator '{evaluator}' failed: {reason}")]
    Evaluation {
        /// The evaluator's registered name.
        evaluator: String,
        /// Why the evaluator failed.
        reason: String,
    },
}

/// The engine- and campaign-owned registry of named
/// `EdgeCondition::Custom(name)` evaluators (BUG-01, CF-01).
///
/// Unlike [`crate::engine::dispatch_registry::DispatchRegistry`], there is
/// NO reserved-name guard here: `EdgeCondition::Custom` names collide with
/// no built-in `EdgeCondition` variant name (`EdgeCondition` has no
/// "Custom"-shaped sibling the way `DispatchRule` does), so there is nothing
/// for a registered name to be confused with. Registering under a name that
/// is already registered REPLACES the prior evaluator rather than erroring
/// -- mirroring the same discretion `DispatchRegistry` exercises for its own
/// reserved-name stance, just resolved the other way since there is no
/// collision class to reject here. Name lookup is exact `String` equality:
/// no trimming, no case folding, no Unicode normalization -- `"isUrgent"`
/// and `"isurgent"` are two distinct registrations.
/// `Clone` (CF-FR-16, D-21): a `NodeSpec::Battalion` node's child run
/// inherits the PARENT's edge-evaluator registry wholesale (D-19), and
/// forwarding it into a `tokio::spawn`'d dispatch task requires an owned,
/// `'static` copy -- cheap, since every value clone is an `Arc::clone`.
#[derive(Default, Clone)]
pub struct EdgeEvaluatorRegistry {
    inner: HashMap<String, Arc<dyn EdgeConditionEvaluator>>,
}

impl EdgeEvaluatorRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `evaluator` under `name`, exact-byte-equality keyed. A
    /// second registration under the same name replaces the first.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        evaluator: Arc<dyn EdgeConditionEvaluator>,
    ) {
        self.inner.insert(name.into(), evaluator);
    }

    /// Look up the evaluator registered under `name`, if any.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn EdgeConditionEvaluator>> {
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
    fn name_lookup_is_exact_byte_equality_case_sensitive() {
        let mut registry = EdgeEvaluatorRegistry::new();
        registry.register("isUrgent", Arc::new(AlwaysTrue));

        assert!(registry.contains("isUrgent"));
        assert!(
            !registry.contains("isurgent"),
            "lookup must be exact byte equality, not case-insensitive"
        );
        assert!(registry.get("isurgent").is_none());
        assert!(registry.get("isUrgent").is_some());
    }

    #[test]
    fn registered_names_returns_a_byte_sorted_list() {
        let mut registry = EdgeEvaluatorRegistry::new();
        registry.register("zeta", Arc::new(AlwaysTrue));
        registry.register("alpha", Arc::new(AlwaysTrue));
        registry.register("mid", Arc::new(AlwaysTrue));

        assert_eq!(registry.registered_names(), vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn duplicate_registration_replaces_the_prior_evaluator() {
        let mut registry = EdgeEvaluatorRegistry::new();
        registry.register("is_urgent", Arc::new(AlwaysTrue));
        registry.register("is_urgent", Arc::new(AlwaysTrue));

        assert_eq!(registry.registered_names(), vec!["is_urgent"]);
    }
}
