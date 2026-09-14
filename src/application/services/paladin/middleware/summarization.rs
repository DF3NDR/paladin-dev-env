//! `SummarizationMiddleware`: compresses an over-long conversation into a
//! compounding Garrison summary, degrading to trimming rather than ever
//! failing the run (Doc 05 RT-FR-08/11/12, D-16).
//!
//! # Compounding without a Garrison delete API (D-16)
//!
//! `GarrisonPort` has no delete-by-id (`garrison_port.rs:380-491`), so a
//! summary this middleware writes is never removed from the store. That is
//! why [`super::super::paladin_execution_service::effective_history`]
//! exists: every reader of a recalled Garrison window -- the trimmer, this
//! middleware, and the service's own prompt renderer -- finds the NEWEST
//! `is_summary` entry and treats it plus every raw entry newer than it as
//! "the history". This middleware never reads the Garrison store directly to
//! decide what to fold; it trusts that `cx.assembly.history` already IS the
//! effective history, because the service applies that rule before this
//! middleware ever runs (D-16's own placement decision).
//!
//! # Compounding, concretely
//!
//! With `threshold_messages: 30` and `keep_recent: 10`, a 30-entry effective
//! history summarizes entries `[0..20)` (the oldest 20, i.e. everything
//! beyond the newest 10) and keeps entries `[20..30)` raw -- producing an
//! 11-entry effective history (1 summary + 10 raw) for the next prompt. Add
//! 20 more raw entries (now 31 effective entries: the summary + 10 raw + 20
//! new raw) and the SECOND summarization folds `[0..21)` -- which starts
//! with the FIRST summary's own content, not the original 20 raw entries it
//! replaced (those no longer exist as separate entries at all). This is what
//! "compounds" means here: each summary's input is whatever the previous
//! round left behind, never a replay of history that already got folded.
//!
//! # Self-sufficient degradation (D-16, RT-FR-12)
//!
//! This middleware is the only built-in that makes its OWN model call, so
//! its failure path cannot depend on anything else being installed. On ANY
//! summarizer error -- transient or permanent, D-31's classification is
//! irrelevant here -- it logs a warning, sets
//! `scratch["summarization.degraded"] = true`, and runs an EMBEDDED
//! [`super::history::HistoryTrimmer`] (built from the same
//! [`crate::config::agent_runtime::HistoryTrimmerConfig`] this middleware
//! was constructed with) directly against `cx`. `HistoryTrimmer::before_model`
//! never returns `MiddlewareFlow::Fail`, so neither does this path -- this
//! middleware's own `before_model` never returns `Fail` either. A
//! `TraceEvent` for this degradation is Phase 28's (OBS-01/02), not this
//! phase's -- its absence here is scope, not omission.
//!
//! # The summarizer's call is invisible to the chain (D-08, D-16)
//!
//! The summarizer port is called directly with `.generate(request).await`,
//! never through [`super::run_before`]/[`super::run_after`] -- so no
//! `before_model`/`after_model` hook fires for it, and a `ModelCallLimit`
//! sharing the same run never counts it.

use std::sync::Arc;

use async_trait::async_trait;
use log::warn;

use crate::application::services::paladin::error::PaladinError;
use crate::config::agent_runtime::{HistoryTrimmerConfig, SummarizationConfig};
use crate::core::platform::container::garrison::{ConversationRole, GarrisonEntry};
use crate::core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_ports::output::garrison_port::GarrisonPort;
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use paladin_ports::output::token_counter_port::TokenCounterPort;

use super::history::HistoryTrimmer;
use super::{ExecutionMiddleware, MiddlewareFlow, ModelCallContext};

/// The key [`ModelCallContext::scratch`] carries when a summarizer call
/// fails and this middleware degrades to its embedded trimmer instead
/// (D-16, RT-FR-12). Written once so the implementation and every test
/// asserting on it read the identical string.
pub const SUMMARIZATION_DEGRADED_KEY: &str = "summarization.degraded";

/// The fixed instruction prefix every summarizer prompt starts with,
/// written once as a private `const` so the compounding test can assert
/// against it without duplicating the string (per this plan's own
/// instruction).
const SUMMARIZATION_PROMPT_PREAMBLE: &str = "Summarize the following conversation history \
concisely, preserving important facts, decisions and context a later turn may need. Output only \
the summary text itself, with no preamble or meta-commentary.";

/// Renders `entries` into one prompt string: the fixed preamble, then each
/// entry as `"{Role}: {content}"`, one per line, in the given order (oldest
/// first, matching `cx.assembly.history`'s own ordering).
fn render_summarizer_prompt(entries: &[GarrisonEntry]) -> String {
    let mut prompt = String::from(SUMMARIZATION_PROMPT_PREAMBLE);
    prompt.push_str("\n\n");
    for entry in entries {
        let role_str = match entry.role {
            ConversationRole::System => "System",
            ConversationRole::User => "User",
            ConversationRole::Assistant => "Assistant",
            ConversationRole::Tool => "Tool",
        };
        prompt.push_str(&format!("{role_str}: {}\n", entry.content));
    }
    prompt
}

/// Compresses an over-long effective history into a compounding Garrison
/// summary, degrading to an embedded [`HistoryTrimmer`] on any summarizer
/// failure. See the module docs for the full contract.
pub struct SummarizationMiddleware {
    config: SummarizationConfig,
    counter: Arc<dyn TokenCounterPort>,
    /// The service's own port -- the default summarizer when
    /// `summarizer_override` is `None`, and the port the embedded
    /// [`HistoryTrimmer`] resolves context limits through (D-14).
    service_llm_port: Arc<dyn LlmPort>,
    /// A distinct summarizer port, if the caller wants summarization served
    /// by a different provider/model than the run's own (D-16: "defaulting
    /// to the service port").
    summarizer_override: Option<Arc<dyn LlmPort>>,
    /// Where the resulting summary is `remember`ed (D-16). A deviation from
    /// this plan's literal 4-argument constructor text -- see the module
    /// docs and this type's `new` doc for why a Garrison reference is
    /// structurally required.
    garrison: Arc<dyn GarrisonPort>,
    /// The self-sufficient degradation path (D-16, RT-FR-12): built once at
    /// construction from the SAME [`HistoryTrimmerConfig`] this middleware
    /// was given, with `enabled` forced to `true` regardless of the passed
    /// value -- this is a guaranteed internal fallback, not a user-facing
    /// toggle, so it must trim on every degradation regardless of whether a
    /// separate, independently-configured `HistoryTrimmer` happens to be
    /// installed or enabled elsewhere in the chain (Test 5's own point).
    embedded_trimmer: HistoryTrimmer,
}

impl SummarizationMiddleware {
    /// Constructs a `SummarizationMiddleware`.
    ///
    /// # Deviation from the plan's literal 4-argument constructor (Rule 2)
    ///
    /// The plan's action text names
    /// `SummarizationMiddleware::new(SummarizationConfig, HistoryTrimmerConfig,
    /// Arc<dyn TokenCounterPort>, Option<Arc<dyn LlmPort>>)`. Two more
    /// parameters are structurally required by D-16's own contract and are
    /// added here, mirroring plan 26-11's identical `HistoryTrimmer::new`
    /// deviation:
    ///
    /// - `service_llm_port: Arc<dyn LlmPort>` -- D-16 says the summarizer
    ///   "defaults to the service port", but nothing reachable from
    ///   `before_model` names which port that is unless the caller provides
    ///   it explicitly; it also feeds the embedded trimmer's own D-14
    ///   context-limit resolution.
    /// - `garrison: Arc<dyn GarrisonPort>` -- D-16 requires `remember`ing the
    ///   resulting summary; no type reachable from `before_model` carries a
    ///   Garrison reference.
    ///
    /// `summarizer_override` is the plan's literal `Option<Arc<dyn
    /// LlmPort>>`: `Some` names a distinct summarizer port/model, `None`
    /// falls back to `service_llm_port`.
    pub fn new(
        config: SummarizationConfig,
        trimmer_config: HistoryTrimmerConfig,
        counter: Arc<dyn TokenCounterPort>,
        service_llm_port: Arc<dyn LlmPort>,
        summarizer_override: Option<Arc<dyn LlmPort>>,
        garrison: Arc<dyn GarrisonPort>,
    ) -> Self {
        let mut embedded_config = trimmer_config;
        embedded_config.enabled = true;
        let embedded_trimmer =
            HistoryTrimmer::new(embedded_config, counter.clone(), service_llm_port.clone());
        Self {
            config,
            counter,
            service_llm_port,
            summarizer_override,
            garrison,
            embedded_trimmer,
        }
    }

    /// Whether the effective history in `assembly` exceeds either
    /// configured threshold (D-16: token threshold via the counter, or
    /// message-count threshold -- `>=`, not `>`, so a history of EXACTLY
    /// `threshold_messages` entries triggers, matching the PRD's own
    /// 30-message acceptance figure).
    fn exceeds_threshold(&self, assembly: &super::PromptAssembly, model: &str) -> bool {
        if assembly.history.len() as u32 >= self.config.threshold_messages {
            return true;
        }
        if let Some(threshold_tokens) = self.config.threshold_tokens {
            let total: u32 = assembly
                .history
                .iter()
                .map(|entry| self.counter.count(&entry.content, model))
                .sum();
            if total > threshold_tokens {
                return true;
            }
        }
        false
    }

    /// Logs, marks `cx.scratch` degraded, and runs the embedded trimmer.
    /// Never returns `MiddlewareFlow::Fail` -- `HistoryTrimmer::before_model`
    /// never does, and this path propagates its result unchanged.
    async fn degrade(
        &self,
        cx: &mut ModelCallContext<'_>,
        reason: impl std::fmt::Display,
    ) -> Result<MiddlewareFlow, PaladinError> {
        warn!(
            "summarization: {reason} -- degrading to the embedded HistoryTrimmer for this call \
             (scratch[\"{SUMMARIZATION_DEGRADED_KEY}\"] = true)"
        );
        cx.scratch.insert(
            SUMMARIZATION_DEGRADED_KEY.to_string(),
            serde_json::json!(true),
        );
        self.embedded_trimmer.before_model(cx).await
    }
}

#[async_trait]
impl ExecutionMiddleware for SummarizationMiddleware {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        if !self.config.enabled {
            return Ok(MiddlewareFlow::Continue);
        }

        // EDGE(RT-03/empty): an empty history is a no-op -- nothing to
        // summarize, no summarizer call made.
        if cx.assembly.history.is_empty() {
            return Ok(MiddlewareFlow::Continue);
        }

        let model = cx.paladin().node.model.clone();

        if !self.exceeds_threshold(&cx.assembly, &model) {
            return Ok(MiddlewareFlow::Continue);
        }

        let keep_recent = self.config.keep_recent as usize;
        let total_len = cx.assembly.history.len();
        if total_len <= keep_recent {
            // Over threshold by token count alone, but too few entries to
            // fold anything without discarding the whole "keep recent"
            // guarantee -- nothing to summarize.
            return Ok(MiddlewareFlow::Continue);
        }
        let split = total_len - keep_recent;
        let to_summarize = &cx.assembly.history[..split];
        let recent = cx.assembly.history[split..].to_vec();

        let newest_summarized_id = to_summarize
            .last()
            .expect("split > 0 because total_len > keep_recent")
            .id;

        let summarizer_model = self
            .config
            .summarizer_model
            .clone()
            .unwrap_or_else(|| model.clone());
        let summarizer_port = self
            .summarizer_override
            .clone()
            .unwrap_or_else(|| self.service_llm_port.clone());

        let prompt_text = render_summarizer_prompt(to_summarize);
        let prompt_item = match PromptItem::new(PromptType::User(UserPrompt {
            query: prompt_text,
            context: None,
        })) {
            Ok(item) => item,
            Err(err) => {
                return self
                    .degrade(cx, format!("failed to build the summarizer prompt: {err}"))
                    .await;
            }
        };
        let request = LlmRequest::new(summarizer_model.clone(), prompt_item);

        // D-08/D-16: called directly, never through `run_before`/`run_after`
        // -- no hook fires for this call, and a `ModelCallLimit` sharing the
        // run never counts it.
        match summarizer_port.generate(request).await {
            Ok(response) => {
                let mut summary_entry = GarrisonEntry::summary(response.content);
                summary_entry.metadata.insert(
                    "summarized_through".to_string(),
                    serde_json::json!(newest_summarized_id.to_string()),
                );

                if let Err(err) = self.garrison.remember(summary_entry.clone()).await {
                    // The model call succeeded and the summary is still
                    // applied to THIS run's prompt below; only the future
                    // compounding benefit (a later run seeing this summary
                    // via `recall_recent`) is lost. Not the summarizer
                    // failure path D-16 names -- the summarizer worked.
                    warn!(
                        "summarization: produced a summary but failed to persist it to \
                         Garrison (it will not compound on a later run): {err}"
                    );
                }

                let mut new_history = Vec::with_capacity(1 + recent.len());
                new_history.push(summary_entry);
                new_history.extend(recent);
                cx.assembly.history = new_history;

                Ok(MiddlewareFlow::Continue)
            }
            Err(err) => {
                self.degrade(
                    cx,
                    format!("summarizer call to model '{summarizer_model}' failed: {err}"),
                )
                .await
            }
        }
    }

    fn name(&self) -> &str {
        "summarization"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::middleware::{ModelCallContext, PromptAssembly};
    use crate::application::services::paladin::paladin_execution_service::{
        PaladinExecutionService, effective_history,
    };
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_llm::mock::MockLlmAdapter;
    use paladin_memory::garrison::in_memory_garrison::InMemoryGarrison;
    use paladin_ports::output::llm_port::LlmError;
    use std::collections::HashMap;
    use std::time::Duration;

    fn make_paladin(model: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: "system".to_string(),
            model: model.to_string(),
            max_loops: MaxLoops::Fixed(1),
            ..Default::default()
        };
        Node::new(data, None)
    }

    fn make_counter() -> Arc<dyn TokenCounterPort> {
        Arc::new(paladin_memory::token_counter::HeuristicTokenCounter)
    }

    fn make_garrison() -> Arc<dyn GarrisonPort> {
        Arc::new(InMemoryGarrison::new(
            paladin_core::platform::container::garrison::GarrisonConfig::default(),
        ))
    }

    fn base_summarization_config() -> SummarizationConfig {
        SummarizationConfig {
            enabled: true,
            threshold_tokens: None,
            threshold_messages: 30,
            keep_recent: 10,
            summarizer_model: None,
        }
    }

    /// A `HistoryTrimmerConfig` with a huge budget -- the embedded trimmer
    /// built from it would never trim anything, used where the DEGRADE path
    /// is not under test.
    fn permissive_trimmer_config() -> HistoryTrimmerConfig {
        HistoryTrimmerConfig {
            enabled: false,
            reserve_for_response: 0,
            default_context_tokens: 1_000_000,
            model_context_limits: HashMap::new(),
            recall_limit: 20,
        }
    }

    fn make_cx<'p>(paladin: &'p Paladin, history: Vec<GarrisonEntry>) -> ModelCallContext<'p> {
        let assembly = PromptAssembly::new("system", "input", "", history, None);
        ModelCallContext::new(uuid::Uuid::new_v4(), paladin, assembly)
    }

    /// A distinct, padded, collision-free entry identifier: `"raw-1"` is a
    /// substring of `"raw-19"`, but `"[MSG-01]"` is never a substring of
    /// `"[MSG-19]"`.
    fn entry(i: usize) -> GarrisonEntry {
        GarrisonEntry::new(ConversationRole::User, format!("[MSG-{i:02}]"))
    }

    fn make_middleware(
        config: SummarizationConfig,
        trimmer_config: HistoryTrimmerConfig,
        service_llm_port: Arc<dyn LlmPort>,
        summarizer_override: Option<Arc<dyn LlmPort>>,
        garrison: Arc<dyn GarrisonPort>,
    ) -> SummarizationMiddleware {
        SummarizationMiddleware::new(
            config,
            trimmer_config,
            make_counter(),
            service_llm_port,
            summarizer_override,
            garrison,
        )
    }

    /// Test 2: 30 messages, `threshold_messages: 30`, `keep_recent: 10` ->
    /// one summarization; the resulting Garrison holds one new entry with
    /// `is_summary: true`, `ConversationRole::System` and
    /// `metadata["summarized_through"]` set to the newest summarized
    /// entry's id; the next rendered prompt (simulated here by re-applying
    /// `effective_history` to the recalled window) shows the summary plus
    /// the 10 most recent raw entries.
    #[tokio::test]
    async fn thirty_messages_produce_one_summary_and_ten_raw() {
        let garrison = make_garrison();
        let raw: Vec<GarrisonEntry> = (0..30).map(entry).collect();
        for e in &raw {
            garrison.remember(e.clone()).await.unwrap();
        }
        let newest_summarized_expected_id = raw[19].id;

        let summarizer: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_response("Summary of the earliest twenty turns."));
        let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let middleware = make_middleware(
            base_summarization_config(),
            permissive_trimmer_config(),
            service_llm,
            Some(summarizer),
            garrison.clone(),
        );

        let paladin = make_paladin("gpt-4");
        let mut cx = make_cx(&paladin, raw.clone());
        middleware.before_model(&mut cx).await.unwrap();

        assert_eq!(cx.assembly.history.len(), 11, "1 summary + 10 raw");
        assert!(cx.assembly.history[0].is_summary);
        assert_eq!(cx.assembly.history[0].role, ConversationRole::System);
        assert_eq!(
            cx.assembly.history[0]
                .metadata
                .get("summarized_through")
                .and_then(|v| v.as_str()),
            Some(newest_summarized_expected_id.to_string().as_str())
        );
        let raw_contents: Vec<&str> = cx.assembly.history[1..]
            .iter()
            .map(|e| e.content.as_str())
            .collect();
        assert_eq!(
            raw_contents,
            vec![
                "[MSG-20]", "[MSG-21]", "[MSG-22]", "[MSG-23]", "[MSG-24]", "[MSG-25]", "[MSG-26]",
                "[MSG-27]", "[MSG-28]", "[MSG-29]"
            ]
        );

        // The next run's recall would see this via `effective_history`.
        let recalled = garrison.recall_recent(100).await.unwrap();
        let effective = effective_history(recalled);
        assert_eq!(effective.len(), 11);
        assert!(effective[0].is_summary);
    }

    /// Test 3: adding 20 more messages triggers a second summarization
    /// whose input is [summary #1 + the oldest raw entries beyond
    /// `keep_recent`] and NOT the original 30 -- asserted against the
    /// summarizer mock's recorded prompt.
    #[tokio::test]
    async fn compounding_builds_the_second_summary_from_the_first() {
        let garrison = make_garrison();
        let raw: Vec<GarrisonEntry> = (0..30).map(entry).collect();
        for e in &raw {
            garrison.remember(e.clone()).await.unwrap();
        }

        let summarizer = Arc::new(
            MockLlmAdapter::new()
                .with_responses(vec!["Summary one.".to_string(), "Summary two.".to_string()]),
        );
        let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let middleware = make_middleware(
            base_summarization_config(),
            permissive_trimmer_config(),
            service_llm,
            Some(summarizer.clone() as Arc<dyn LlmPort>),
            garrison.clone(),
        );

        let paladin = make_paladin("gpt-4");

        // Round 1.
        let mut cx1 = make_cx(&paladin, raw.clone());
        middleware.before_model(&mut cx1).await.unwrap();
        assert_eq!(summarizer.call_count(), 1);

        // Simulate the next run: recall the window and apply the SAME
        // effective-history rule the service applies.
        let more: Vec<GarrisonEntry> = (30..50).map(entry).collect();
        for e in &more {
            garrison.remember(e.clone()).await.unwrap();
        }
        let recalled = garrison.recall_recent(100).await.unwrap();
        let effective = effective_history(recalled);
        assert_eq!(effective.len(), 31, "1 summary + 10 raw + 20 new raw");

        // Round 2.
        let mut cx2 = make_cx(&paladin, effective);
        middleware.before_model(&mut cx2).await.unwrap();
        assert_eq!(summarizer.call_count(), 2);

        let second_prompt = summarizer.last_prompt().expect("a second call was made");
        assert!(
            second_prompt.contains("Summary one."),
            "the second summarizer input must include the first summary's own content"
        );
        assert!(
            !second_prompt.contains("[MSG-00]") && !second_prompt.contains("[MSG-19]"),
            "the second summarizer input must NOT replay the 20 raw entries the first summary \
             already replaced -- they no longer exist as separate entries"
        );
        assert!(
            second_prompt.contains("[MSG-20]") && second_prompt.contains("[MSG-39]"),
            "the second summarizer input DOES include the raw entries kept after round 1 plus \
             the new raw entries beyond keep_recent"
        );
    }

    /// Test 4: a summarizer mock returning a 503-class error causes
    /// `scratch["summarization.degraded"] == true`, a `log::warn!`, an
    /// embedded-trimmer-shaped history that fits the limit, and a
    /// completed (never failed) run.
    #[tokio::test]
    async fn summarizer_failure_degrades_to_trimming() {
        let garrison = make_garrison();
        let summarizer: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_error(LlmError::ProviderError {
                provider: "mock".to_string(),
                status: 503,
                message: "temporarily unavailable".to_string(),
            }));
        let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());

        let mut tight_trimmer_config = permissive_trimmer_config();
        tight_trimmer_config.default_context_tokens = 3; // room for ~1 tiny entry
        // `MockLlmAdapter::get_capabilities()` reports `max_context_tokens:
        // Some(4096)`, which would otherwise win D-14's step 2 over this
        // test's tiny `default_context_tokens` (step 3) -- force step 1
        // (the config table) so the embedded trimmer's budget is genuinely
        // tiny for "gpt-4".
        tight_trimmer_config
            .model_context_limits
            .insert("gpt-4".to_string(), 3);

        let mut config = base_summarization_config();
        config.threshold_messages = 3;
        config.keep_recent = 1;

        let middleware = make_middleware(
            config,
            tight_trimmer_config,
            service_llm,
            Some(summarizer),
            garrison,
        );

        let paladin = make_paladin("gpt-4");
        let history: Vec<GarrisonEntry> = (0..5).map(entry).collect();
        let mut cx = make_cx(&paladin, history);

        let outcome = middleware.before_model(&mut cx).await;

        assert!(outcome.is_ok(), "the run must complete, never fail");
        assert_eq!(
            cx.scratch
                .get(SUMMARIZATION_DEGRADED_KEY)
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(
            cx.assembly.history.len() <= 1,
            "the embedded trimmer's tiny budget must have cut the history down"
        );
    }

    /// Test 5: Test 4's outcome is identical whether or not a separate
    /// `HistoryTrimmer` is installed, and identical whether that trimmer
    /// sits before or after the summarizer -- degradation is self-sufficient
    /// and does not depend on chain order.
    #[tokio::test]
    async fn degradation_does_not_depend_on_chain_order() {
        fn failing_middleware() -> SummarizationMiddleware {
            let garrison = make_garrison();
            let summarizer: Arc<dyn LlmPort> =
                Arc::new(MockLlmAdapter::new().with_error(LlmError::ProviderError {
                    provider: "mock".to_string(),
                    status: 503,
                    message: "temporarily unavailable".to_string(),
                }));
            let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
            let mut tight_trimmer_config = permissive_trimmer_config();
            tight_trimmer_config.default_context_tokens = 3;
            // See `summarizer_failure_degrades_to_trimming`'s identical
            // comment: forces D-14 step 1 over the mock's hardcoded
            // provider-capabilities step 2.
            tight_trimmer_config
                .model_context_limits
                .insert("gpt-4".to_string(), 3);
            let mut config = base_summarization_config();
            config.threshold_messages = 3;
            config.keep_recent = 1;
            make_middleware(
                config,
                tight_trimmer_config,
                service_llm,
                Some(summarizer),
                garrison,
            )
        }

        fn contents(cx: &ModelCallContext<'_>) -> Vec<String> {
            cx.assembly
                .history
                .iter()
                .map(|e| e.content.clone())
                .collect()
        }

        let paladin = make_paladin("gpt-4");
        let history: Vec<GarrisonEntry> = (0..5).map(entry).collect();

        // (a) No separate trimmer installed at all.
        let middleware_a = failing_middleware();
        let mut cx_a = make_cx(&paladin, history.clone());
        middleware_a.before_model(&mut cx_a).await.unwrap();

        // (b) A separate, generously-bounded (effectively no-op) trimmer
        // runs BEFORE summarization.
        let middleware_b = failing_middleware();
        let separate_trimmer_b = HistoryTrimmer::new(
            permissive_trimmer_config(),
            make_counter(),
            Arc::new(MockLlmAdapter::new()),
        );
        let mut cx_b = make_cx(&paladin, history.clone());
        separate_trimmer_b.before_model(&mut cx_b).await.unwrap();
        middleware_b.before_model(&mut cx_b).await.unwrap();

        // (c) The same separate trimmer runs AFTER summarization.
        let middleware_c = failing_middleware();
        let separate_trimmer_c = HistoryTrimmer::new(
            permissive_trimmer_config(),
            make_counter(),
            Arc::new(MockLlmAdapter::new()),
        );
        let mut cx_c = make_cx(&paladin, history.clone());
        middleware_c.before_model(&mut cx_c).await.unwrap();
        separate_trimmer_c.before_model(&mut cx_c).await.unwrap();

        assert_eq!(contents(&cx_a), contents(&cx_b));
        assert_eq!(contents(&cx_a), contents(&cx_c));
        for cx in [&cx_a, &cx_b, &cx_c] {
            assert_eq!(
                cx.scratch
                    .get(SUMMARIZATION_DEGRADED_KEY)
                    .and_then(|v| v.as_bool()),
                Some(true)
            );
        }
    }

    /// Test 6: with a `ModelCallLimit { max_calls: 1 }` also installed, a
    /// run that summarizes still gets its one main model call, and a
    /// recording middleware observes no extra `before_model`/`after_model`
    /// for the summarizer's call.
    #[tokio::test]
    async fn the_summarizers_own_call_is_not_counted_and_fires_no_hooks() {
        struct RecordingMiddleware {
            before_count: std::sync::atomic::AtomicU32,
            after_count: std::sync::atomic::AtomicU32,
        }

        #[async_trait]
        impl ExecutionMiddleware for RecordingMiddleware {
            async fn before_model(
                &self,
                _cx: &mut ModelCallContext<'_>,
            ) -> Result<MiddlewareFlow, PaladinError> {
                self.before_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(MiddlewareFlow::Continue)
            }

            async fn after_model(
                &self,
                _cx: &mut ModelCallContext<'_>,
                _resp: &mut crate::application::services::paladin::middleware::LlmResponseView,
            ) -> Result<MiddlewareFlow, PaladinError> {
                self.after_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(MiddlewareFlow::Continue)
            }

            fn name(&self) -> &str {
                "recording"
            }
        }

        let garrison = make_garrison();
        let raw: Vec<GarrisonEntry> = (0..30).map(entry).collect();
        for e in &raw {
            garrison.remember(e.clone()).await.unwrap();
        }

        let main_llm = Arc::new(MockLlmAdapter::new().with_response("the final answer"));
        let summarizer: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_response("a compact summary"));
        let summarization = Arc::new(make_middleware(
            base_summarization_config(),
            permissive_trimmer_config(),
            main_llm.clone(),
            Some(summarizer),
            garrison.clone(),
        ));
        let recorder = Arc::new(RecordingMiddleware {
            before_count: std::sync::atomic::AtomicU32::new(0),
            after_count: std::sync::atomic::AtomicU32::new(0),
        });
        let limit = Arc::new(
            crate::application::services::paladin::middleware::ModelCallLimit::new(
                crate::config::agent_runtime::ModelCallLimitConfig {
                    enabled: true,
                    max_calls: 1,
                },
            ),
        );

        let service = PaladinExecutionService::new(
            main_llm.clone(),
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            Some(garrison),
            None,
        )
        .with_recall_limit(100)
        .with_middleware(limit)
        .with_middleware(summarization)
        .with_middleware(recorder.clone() as Arc<dyn ExecutionMiddleware>);

        let paladin = make_paladin("gpt-4");
        let result = service.execute(&paladin, "one more turn").await.unwrap();

        assert_eq!(main_llm.call_count(), 1, "exactly one MAIN model call");
        assert_eq!(
            recorder
                .before_count
                .load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the recorder must observe exactly one before_model -- none extra for the \
             summarizer's own call"
        );
        assert_eq!(
            recorder
                .after_count
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(result.output, "the final answer");
    }

    /// Test 7: a history of 5 messages with `threshold_messages: 30` leaves
    /// the assembly byte-identical and makes no summarizer call.
    #[tokio::test]
    async fn below_threshold_is_a_no_op() {
        let garrison = make_garrison();
        let summarizer = Arc::new(MockLlmAdapter::new());
        let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let middleware = make_middleware(
            base_summarization_config(),
            permissive_trimmer_config(),
            service_llm,
            Some(summarizer.clone() as Arc<dyn LlmPort>),
            garrison,
        );

        let paladin = make_paladin("gpt-4");
        let history: Vec<GarrisonEntry> = (0..5).map(entry).collect();
        let mut cx = make_cx(&paladin, history.clone());
        let before_render = cx.assembly.render();

        middleware.before_model(&mut cx).await.unwrap();

        assert_eq!(cx.assembly.render(), before_render);
        assert_eq!(summarizer.call_count(), 0);
    }

    /// Test 8: an empty history makes no summarizer call and does not
    /// panic.
    #[tokio::test]
    async fn empty_history_is_a_no_op() {
        let garrison = make_garrison();
        let summarizer = Arc::new(MockLlmAdapter::new());
        let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let middleware = make_middleware(
            base_summarization_config(),
            permissive_trimmer_config(),
            service_llm,
            Some(summarizer.clone() as Arc<dyn LlmPort>),
            garrison,
        );

        let paladin = make_paladin("gpt-4");
        let mut cx = make_cx(&paladin, Vec::new());

        let outcome = middleware.before_model(&mut cx).await;

        assert!(outcome.is_ok());
        assert!(cx.assembly.history.is_empty());
        assert_eq!(summarizer.call_count(), 0);
    }

    /// A disabled middleware changes nothing and calls no summarizer.
    #[tokio::test]
    async fn disabled_middleware_changes_nothing() {
        let garrison = make_garrison();
        let summarizer = Arc::new(MockLlmAdapter::new());
        let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let mut config = base_summarization_config();
        config.enabled = false;
        let middleware = make_middleware(
            config,
            permissive_trimmer_config(),
            service_llm,
            Some(summarizer.clone() as Arc<dyn LlmPort>),
            garrison,
        );

        let paladin = make_paladin("gpt-4");
        let history: Vec<GarrisonEntry> = (0..40).map(entry).collect();
        let mut cx = make_cx(&paladin, history.clone());

        middleware.before_model(&mut cx).await.unwrap();

        assert_eq!(cx.assembly.history.len(), 40);
        assert_eq!(summarizer.call_count(), 0);
    }

    /// `before_model` never returns `MiddlewareFlow::Fail`, even on
    /// summarizer failure.
    #[tokio::test]
    async fn summarization_never_fails_the_run() {
        let garrison = make_garrison();
        let summarizer: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new().with_error(LlmError::NetworkError("connection reset".into())),
        );
        let service_llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let middleware = make_middleware(
            base_summarization_config(),
            permissive_trimmer_config(),
            service_llm,
            Some(summarizer),
            garrison,
        );

        let paladin = make_paladin("gpt-4");
        let history: Vec<GarrisonEntry> = (0..30).map(entry).collect();
        let mut cx = make_cx(&paladin, history);

        let outcome = middleware.before_model(&mut cx).await;
        assert!(matches!(outcome, Ok(MiddlewareFlow::Continue)));
    }
}
