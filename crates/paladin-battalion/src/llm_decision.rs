//! `LlmDecisionEvaluator`: an [`EdgeConditionEvaluator`] that routes an edge
//! from a live model's answer against a closed, author-declared choice list
//! (CF-05, D-23/D-24).
//!
//! Registered under `EdgeCondition::Custom("<decision name>")` through
//! [`crate::edge_evaluator::EdgeEvaluatorRegistry`] like any other evaluator
//! (D-23) -- `paladin-core`'s `EdgeCondition` gains no new variant. Nothing
//! in this module is reachable unless a workflow author constructs an
//! evaluator and registers it in code: no `APP_*` environment variable, no
//! cargo feature, and no config-struct field reaches it (D-26).
//!
//! # One call per decision per superstep (D-24)
//!
//! A source node may have several outgoing `LlmDecision`-routed edges in the
//! same superstep. [`LlmDecisionEvaluator`] resolves the model's verdict
//! **once** per `(thread, superstep, source, rendered prompt)` and every
//! edge belonging to that decision consults the same memoized answer -- an
//! edge fires iff the matched choice's mapped target equals the edge's own
//! target. Without this, N outgoing edges would become N independent calls,
//! and a model that answers differently (or an ambiguous answer resolved
//! differently) across those calls could fire both edges or neither --
//! exactly the routing corruption class BUG-01 was.
//!
//! # Security: the rendered prompt is an egress boundary
//!
//! `prompt_template` renders against live `Battlefield` state (the engine
//! path, `EdgeContext::battlefield` is `Some`) or a source node's raw output
//! string (the legacy `CampaignExecutionService` path, `battlefield` is
//! `None`) and the rendered result is sent, verbatim, to a third-party model
//! through `Arc<dyn LlmPort>`. Whatever the template's placeholders resolve
//! to is exactly what leaves this process -- if a workflow author's schema
//! happens to carry secret-like data and the template references that
//! field, it is sent to the model. This is the workflow author's control
//! point, not something this module can filter
//! (`.github/instructions/security.instructions.md`, mirroring
//! `paladin_llm::redaction`'s redact-before-truncate discipline for a
//! different subsystem). Consequently neither this evaluator's error paths
//! nor its memoized state ever interpolate the rendered prompt or the
//! model's raw response body -- an evaluator failure names only this
//! evaluator and a short, fixed failure class (see [`llm_error_class`]).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::llm_port::{LlmError, LlmPort, LlmRequest};
use uuid::Uuid;

use crate::edge_evaluator::{EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorError};
use crate::engine::InputMapping;

/// How an [`LlmDecisionEvaluator`] resolves a model answer that matches
/// none of its declared choices, exact-after-trim and case-insensitively
/// (D-24).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OnAmbiguous {
    /// Fail the run with a typed [`EdgeEvaluatorError`] -- the default.
    /// Silently guessing a routing target for an unrecognized answer is
    /// exactly BUG-01's class of corruption; `Fail` refuses to guess.
    #[default]
    Fail,
    /// Treat the unmatched answer as if the model had returned the named
    /// choice.
    Default(String),
}

/// The memo key one resolved decision is cached under: a decision is
/// resolved once per `(thread, superstep, source, rendered prompt)` (D-24).
/// The rendered prompt is part of the key (rather than assumed constant for
/// a given thread/superstep/source) so a decision whose template resolves
/// differently across two evaluations in the same superstep -- which should
/// not happen in practice, since the source node and Battlefield are fixed
/// for the duration of one superstep, but is not structurally prevented --
/// is never silently answered from a stale cache entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoKey {
    thread: Option<ThreadId>,
    superstep: Option<u64>,
    source: NodeId,
    rendered_prompt: String,
}

/// A memoized decision outcome: either the matched (or defaulted) choice
/// label, or the failure reason to replay for every subsequent edge of the
/// same decision.
#[derive(Debug, Clone)]
enum MemoOutcome {
    Matched(String),
    Failed(String),
}

/// Resolves an `EdgeCondition::Custom(name)` edge from a live LLM's answer
/// against a closed, author-declared choice list (CF-FR-18).
///
/// Register an instance under a decision name through
/// [`crate::edge_evaluator::EdgeEvaluatorRegistry::register`] (legacy
/// `CampaignExecutionService::with_evaluator`, engine-path
/// `WarEngine::with_edge_evaluator`), then reference that name from an edge
/// via `EdgeCondition::Custom("<decision name>")`. See the module-level
/// rustdoc for the memoization rule (D-24) and the security boundary this
/// evaluator's `prompt_template` crosses.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use paladin_battalion::llm_decision::{LlmDecisionEvaluator, OnAmbiguous};
/// use paladin_core::platform::container::waypoint::NodeId;
/// use paladin_llm::mock::MockLlmAdapter;
///
/// let llm = Arc::new(MockLlmAdapter::new().with_response("escalate"));
/// let _evaluator = LlmDecisionEvaluator::new(
///     "route_urgency",
///     llm,
///     "gpt-4",
///     "Is this urgent? Reply escalate or archive.\n\n{output}",
///     vec![
///         ("escalate".to_string(), NodeId::new("urgent_handler")),
///         ("archive".to_string(), NodeId::new("archive_handler")),
///     ],
/// )
/// .on_ambiguous(OnAmbiguous::Default("archive".to_string()));
/// ```
pub struct LlmDecisionEvaluator {
    /// Identifies this evaluator in error messages. Independent of the
    /// registry name it is later registered under (though authors typically
    /// keep the two the same).
    name: String,
    llm: Arc<dyn LlmPort>,
    model: String,
    prompt_template: String,
    /// `(legal model answer, edge target the answer routes to)`.
    choices: Vec<(String, NodeId)>,
    on_ambiguous: OnAmbiguous,
    /// Single-slot memo (D-24): the outgoing edges of one source node are
    /// evaluated consecutively within one `record_execution` pass, so one
    /// slot is sufficient and cannot grow unbounded.
    memo: tokio::sync::Mutex<Option<(MemoKey, MemoOutcome)>>,
}

impl LlmDecisionEvaluator {
    /// Construct an evaluator. `choices` maps each legal model answer to the
    /// `NodeId` it routes to; matching is exact-after-trim, case-insensitive
    /// (D-24). Defaults `on_ambiguous` to [`OnAmbiguous::Fail`] -- override
    /// with [`Self::on_ambiguous`].
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmPort>,
        model: impl Into<String>,
        prompt_template: impl Into<String>,
        choices: Vec<(String, NodeId)>,
    ) -> Self {
        Self {
            name: name.into(),
            llm,
            model: model.into(),
            prompt_template: prompt_template.into(),
            choices,
            on_ambiguous: OnAmbiguous::default(),
            memo: tokio::sync::Mutex::new(None),
        }
    }

    /// Override the model identifier sent with each call.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override how an unmatched model answer is resolved (default:
    /// [`OnAmbiguous::Fail`]).
    pub fn on_ambiguous(mut self, on_ambiguous: OnAmbiguous) -> Self {
        self.on_ambiguous = on_ambiguous;
        self
    }

    fn evaluation_error(&self, reason: impl Into<String>) -> EdgeEvaluatorError {
        EdgeEvaluatorError::Evaluation {
            evaluator: self.name.clone(),
            reason: reason.into(),
        }
    }

    /// Render this decision's prompt template for one edge evaluation.
    ///
    /// Engine path (`ctx.battlefield` is `Some`): rendered through
    /// [`InputMapping::render`] so placeholder syntax matches the rest of
    /// the engine. Legacy path (`ctx.battlefield` is `None`): the single
    /// `{output}` placeholder is replaced, raw, with the source Paladin's
    /// output string.
    fn render_prompt(
        &self,
        output: &str,
        ctx: &EdgeContext<'_>,
    ) -> Result<String, EdgeEvaluatorError> {
        match ctx.battlefield {
            Some(battlefield) => InputMapping::new(self.prompt_template.clone())
                .render(battlefield)
                .map_err(|e| self.evaluation_error(format!("template rendering failed: {e}"))),
            None => Ok(self.prompt_template.replace("{output}", output)),
        }
    }

    /// Resolve the decision for this `(thread, superstep, source)`, reusing
    /// a memoized outcome when this exact decision (same rendered prompt)
    /// was already asked earlier in the same superstep (D-24).
    async fn resolve_choice(
        &self,
        rendered_prompt: &str,
        ctx: &EdgeContext<'_>,
    ) -> Result<String, EdgeEvaluatorError> {
        let key = MemoKey {
            thread: ctx.thread.cloned(),
            superstep: ctx.superstep,
            source: ctx.source.clone(),
            rendered_prompt: rendered_prompt.to_string(),
        };

        let mut guard = self.memo.lock().await;
        if let Some((cached_key, outcome)) = guard.as_ref()
            && *cached_key == key
        {
            return match outcome {
                MemoOutcome::Matched(label) => Ok(label.clone()),
                MemoOutcome::Failed(reason) => Err(self.evaluation_error(reason.clone())),
            };
        }

        let result = self.call_and_match(rendered_prompt).await;
        let outcome = match &result {
            Ok(label) => MemoOutcome::Matched(label.clone()),
            Err(EdgeEvaluatorError::Evaluation { reason, .. }) => {
                MemoOutcome::Failed(reason.clone())
            }
        };
        *guard = Some((key, outcome));
        result
    }

    /// Call the LLM once and match its answer against `choices`,
    /// exact-after-trim, case-insensitive; resolve an unmatched answer
    /// through `on_ambiguous`.
    async fn call_and_match(&self, rendered_prompt: &str) -> Result<String, EdgeEvaluatorError> {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: rendered_prompt.to_string(),
            context: None,
        }))
        .map_err(|e| self.evaluation_error(format!("prompt construction failed: {e}")))?;

        let request = LlmRequest {
            id: Uuid::new_v4(),
            model: self.model.clone(),
            prompt,
            attachments: vec![],
            stream: false,
            metadata: HashMap::new(),
        };

        let response = self
            .llm
            .generate(request)
            .await
            .map_err(|e| self.evaluation_error(llm_error_class(&e)))?;

        let answer = response.content.trim();
        for (label, _target) in &self.choices {
            if label.trim().eq_ignore_ascii_case(answer) {
                return Ok(label.clone());
            }
        }

        match &self.on_ambiguous {
            OnAmbiguous::Fail => {
                Err(self.evaluation_error("model answer matched no declared choice"))
            }
            OnAmbiguous::Default(choice) => Ok(choice.clone()),
        }
    }
}

#[async_trait]
impl EdgeConditionEvaluator for LlmDecisionEvaluator {
    async fn evaluate(
        &self,
        output: &str,
        ctx: &EdgeContext<'_>,
    ) -> Result<bool, EdgeEvaluatorError> {
        let rendered = self.render_prompt(output, ctx)?;
        let matched_label = self.resolve_choice(&rendered, ctx).await?;
        let fires = self
            .choices
            .iter()
            .any(|(label, target)| *label == matched_label && target == ctx.target);
        Ok(fires)
    }
}

/// A short, fixed classification of an [`LlmError`], safe to interpolate
/// into an evaluator (or Commander strategy-selection) failure message --
/// never the provider's raw error text, which may carry response-body
/// content (the egress-boundary privacy rule this module's rustdoc states).
pub(crate) fn llm_error_class(error: &LlmError) -> &'static str {
    match error {
        LlmError::NetworkError(_) => "network error",
        LlmError::AuthenticationError(_) => "authentication error",
        LlmError::InvalidPrompt(_) => "invalid prompt",
        LlmError::RateLimitExceeded => "rate limit exceeded",
        LlmError::UsageLimitExceeded { .. } => "usage limit exceeded",
        LlmError::ModelNotAvailable(_) => "model not available",
        LlmError::TokenLimitExceeded => "token limit exceeded",
        LlmError::EmptyCompletion(_) => "empty completion",
        LlmError::ProcessingError(_) => "processing error",
        LlmError::Timeout(_) => "timeout",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_llm::mock::MockLlmAdapter;

    fn make_evaluator(llm: Arc<dyn LlmPort>) -> LlmDecisionEvaluator {
        LlmDecisionEvaluator::new(
            "route_urgency",
            llm,
            "mock-model",
            "escalate or archive: {output}",
            vec![
                ("escalate".to_string(), NodeId::new("urgent")),
                ("archive".to_string(), NodeId::new("cold")),
            ],
        )
    }

    fn ctx<'a>(source: &'a NodeId, target: &'a NodeId) -> EdgeContext<'a> {
        EdgeContext {
            source,
            target,
            battlefield: None,
            thread: None,
            superstep: None,
        }
    }

    #[tokio::test]
    async fn matching_choice_fires_only_the_mapped_edge() {
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("escalate"));
        let evaluator = make_evaluator(llm);
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");
        let cold = NodeId::new("cold");

        assert!(
            evaluator
                .evaluate("case", &ctx(&source, &urgent))
                .await
                .unwrap()
        );
        assert!(
            !evaluator
                .evaluate("case", &ctx(&source, &cold))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn one_llm_call_per_decision_per_superstep() {
        let mock = MockLlmAdapter::new().with_response("escalate");
        let llm: Arc<dyn LlmPort> = Arc::new(mock.clone());
        let evaluator = make_evaluator(llm);
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");
        let cold = NodeId::new("cold");

        let fired_urgent = evaluator
            .evaluate("case", &ctx(&source, &urgent))
            .await
            .unwrap();
        let fired_cold = evaluator
            .evaluate("case", &ctx(&source, &cold))
            .await
            .unwrap();

        assert_eq!(mock.call_count(), 1);
        assert!(fired_urgent);
        assert!(!fired_cold);
    }

    #[tokio::test]
    async fn a_different_superstep_re_asks() {
        let mock = MockLlmAdapter::new().with_response("escalate");
        let llm: Arc<dyn LlmPort> = Arc::new(mock.clone());
        let evaluator = make_evaluator(llm);
        let source = NodeId::new("triage");
        let target = NodeId::new("urgent");
        let thread = ThreadId::new("t1").unwrap();

        let ctx0 = EdgeContext {
            source: &source,
            target: &target,
            battlefield: None,
            thread: Some(&thread),
            superstep: Some(0),
        };
        let ctx1 = EdgeContext {
            source: &source,
            target: &target,
            battlefield: None,
            thread: Some(&thread),
            superstep: Some(1),
        };

        evaluator.evaluate("case", &ctx0).await.unwrap();
        evaluator.evaluate("case", &ctx1).await.unwrap();

        assert_eq!(mock.call_count(), 2);
    }

    #[tokio::test]
    async fn matching_is_exact_after_trim_and_case_insensitive() {
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("  ESCALATE\n"));
        let evaluator = make_evaluator(llm);
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");

        assert!(
            evaluator
                .evaluate("case", &ctx(&source, &urgent))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn unmatched_answer_with_on_ambiguous_fail_errors() {
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("maybe"));
        let evaluator = make_evaluator(llm);
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");

        let err = evaluator
            .evaluate("case", &ctx(&source, &urgent))
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("maybe"));
    }

    #[tokio::test]
    async fn unmatched_answer_with_on_ambiguous_default_routes_to_the_default_choice() {
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("maybe"));
        let evaluator =
            make_evaluator(llm).on_ambiguous(OnAmbiguous::Default("archive".to_string()));
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");
        let cold = NodeId::new("cold");

        assert!(
            !evaluator
                .evaluate("case", &ctx(&source, &urgent))
                .await
                .unwrap()
        );
        assert!(
            evaluator
                .evaluate("case", &ctx(&source, &cold))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn llm_error_surfaces_as_a_typed_evaluator_error() {
        let llm: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_error(LlmError::RateLimitExceeded));
        let evaluator = make_evaluator(llm);
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");

        let err = evaluator
            .evaluate("case", &ctx(&source, &urgent))
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("route_urgency"));
        assert!(!msg.contains("escalate or archive"));
    }

    #[test]
    fn legacy_path_renders_the_template_from_the_source_output() {
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("escalate"));
        let evaluator = LlmDecisionEvaluator::new(
            "route_urgency",
            llm,
            "mock-model",
            "decide: {output}",
            vec![("escalate".to_string(), NodeId::new("urgent"))],
        );
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");
        let c = ctx(&source, &urgent);

        let rendered = evaluator.render_prompt("angry customer", &c).unwrap();
        assert_eq!(rendered, "decide: angry customer");
    }

    #[test]
    fn engine_path_renders_the_template_from_the_battlefield() {
        use paladin_core::platform::container::battlefield::{
            Battlefield, BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldName,
            FieldSpec, StateDelta,
        };

        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("topic").unwrap(),
            DispatchRule::LastWrite,
            Some(serde_json::json!("billing")),
            false,
        )]);
        let mut battlefield = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set_raw(
            FieldName::new("topic").unwrap(),
            serde_json::json!("shipping"),
        );
        battlefield
            .merge(
                vec![(NodeId::new("writer"), delta)],
                0,
                &CustomDispatchResolver::new(),
            )
            .unwrap();

        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("escalate"));
        let evaluator = LlmDecisionEvaluator::new(
            "route_topic",
            llm,
            "mock-model",
            "decide for {topic}",
            vec![("escalate".to_string(), NodeId::new("urgent"))],
        );
        let source = NodeId::new("triage");
        let urgent = NodeId::new("urgent");
        let c = EdgeContext {
            source: &source,
            target: &urgent,
            battlefield: Some(&battlefield),
            thread: None,
            superstep: None,
        };

        let rendered = evaluator.render_prompt("ignored on this path", &c).unwrap();
        assert_eq!(rendered, "decide for shipping");
    }
}
