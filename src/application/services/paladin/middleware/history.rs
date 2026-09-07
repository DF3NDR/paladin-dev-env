//! `HistoryTrimmer`: keeps a run's Garrison history within a model's
//! context window, without ever splitting an entry or failing the run
//! (Doc 05 RT-FR-08/10/11/12, D-14, D-15).
//!
//! # Limit resolution is a fixed three-step order (D-14)
//!
//! `config.model_context_limits.get(model)` -> the constructor's own
//! `llm_port.get_capabilities().max_context_tokens` -> `config.default_context_tokens`.
//! The resolved value AND which step produced it are logged at debug --
//! naming the source is how an operator diagnoses "why did my history get
//! trimmed at 8192" rather than guessing.
//!
//! `llm_port` here is the SERVICE's own configured port, read once per
//! `before_model` call -- never a per-run override
//! ([`super::ModelCallContext::llm_override`] is set by port-shaping
//! middleware such as [`super::ModelFallbackMiddleware`] LATER in the
//! documented assembly order, so it is never visible to this middleware's
//! own `before_model` in the same iteration).
//!
//! # Trimming is `KeepSystemAndRecent` (D-15)
//!
//! The system prompt, retrieved context, current input, accumulated output
//! and any middleware-pushed [`super::PromptSection`] are the assembly's
//! *fixed parts* and are never touched. History entries are admitted
//! newest-first while
//! `counted(fixed) + Σ counted(kept) + reserve_for_response <= limit`; the
//! first entry (walking from newest to oldest) that would not fit stops
//! admission entirely -- an entry is kept whole or dropped whole, never
//! truncated. If the fixed parts alone (plus the reserve) already exceed
//! the resolved limit, the history is set empty, a warning is logged
//! naming the resolved limit and its source, and `before_model` still
//! returns `Continue` -- the run proceeds; a budget that cannot be met is
//! never a failure (D-15).
//!
//! Stability (identical inputs produce an identical kept set) falls
//! directly out of the algorithm being a pure function of the fixed parts,
//! the ordered history, the resolved limit and the reserve, over a
//! deterministic [`TokenCounterPort`] -- not an accident that needs its own
//! caching or ordering guard.
//!
//! A [`crate::core::platform::container::garrison::GarrisonEntry`] with
//! `is_summary: true` gets NO special treatment here: this middleware owns
//! size, not summary semantics. `SummarizationMiddleware` (plan 26-15) owns
//! which entry is the newest summary; the two plans do not both claim the
//! same rule.

use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, warn};

use crate::application::services::paladin::error::PaladinError;
use crate::config::agent_runtime::HistoryTrimmerConfig;
use crate::core::platform::container::garrison::GarrisonEntry;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::token_counter_port::TokenCounterPort;

use super::{ExecutionMiddleware, MiddlewareFlow, ModelCallContext};

/// Which of D-14's three resolution steps produced a limit -- named so the
/// debug log can say exactly which one, rather than just the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LimitSource {
    /// `config.model_context_limits.get(model)` had an entry.
    ConfigTable,
    /// The constructor's `llm_port.get_capabilities().max_context_tokens`
    /// had a value.
    ProviderCapabilities,
    /// Neither of the above -- `config.default_context_tokens`.
    Default,
}

impl LimitSource {
    fn as_str(self) -> &'static str {
        match self {
            LimitSource::ConfigTable => "model_context_limits config table",
            LimitSource::ProviderCapabilities => {
                "provider capabilities (get_capabilities().max_context_tokens)"
            }
            LimitSource::Default => "default_context_tokens",
        }
    }
}

/// Keeps a run's conversation history within a model's context window
/// (`KeepSystemAndRecent`, D-15), resolving the context-token limit through
/// the documented three-step order (D-14). See the module docs for the
/// full contract.
pub struct HistoryTrimmer {
    config: HistoryTrimmerConfig,
    counter: Arc<dyn TokenCounterPort>,
    llm_port: Arc<dyn LlmPort>,
}

impl HistoryTrimmer {
    /// Construct a `HistoryTrimmer`.
    ///
    /// `llm_port` is the SERVICE's own port -- D-14's "the service port's
    /// `get_capabilities()`" -- consulted only when `model` has no entry in
    /// `config.model_context_limits`.
    pub fn new(
        config: HistoryTrimmerConfig,
        counter: Arc<dyn TokenCounterPort>,
        llm_port: Arc<dyn LlmPort>,
    ) -> Self {
        Self {
            config,
            counter,
            llm_port,
        }
    }

    /// D-14's three-step resolution order, returning both the resolved
    /// limit and which step produced it.
    fn resolve_limit(&self, model: &str) -> (u32, LimitSource) {
        if let Some(&limit) = self.config.model_context_limits.get(model) {
            return (limit, LimitSource::ConfigTable);
        }
        if let Some(max_context_tokens) = self.llm_port.get_capabilities().max_context_tokens {
            return (max_context_tokens, LimitSource::ProviderCapabilities);
        }
        (self.config.default_context_tokens, LimitSource::Default)
    }

    /// Sum of `counter.count(_, model)` over every fixed (non-history) part
    /// of `assembly`: the system prompt, the retrieved context (if any),
    /// the current input, the accumulated output, and every pushed
    /// [`super::PromptSection`]'s heading and body.
    fn count_fixed_parts(&self, assembly: &super::PromptAssembly, model: &str) -> u32 {
        let mut total = self.counter.count(&assembly.system, model);
        if let Some(context) = assembly.retrieved_context.as_deref() {
            total += self.counter.count(context, model);
        }
        total += self.counter.count(&assembly.input, model);
        total += self.counter.count(&assembly.accumulated_output, model);
        for section in &assembly.sections {
            total += self.counter.count(&section.heading, model);
            total += self.counter.count(&section.body, model);
        }
        total
    }
}

#[async_trait]
impl ExecutionMiddleware for HistoryTrimmer {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        if !self.config.enabled {
            return Ok(MiddlewareFlow::Continue);
        }

        let model = cx.paladin().node.model.clone();
        let (limit, source) = self.resolve_limit(&model);
        let budget = limit.saturating_sub(self.config.reserve_for_response);
        debug!(
            "history_trimmer: resolved context limit {limit} tokens for model '{model}' \
             (source: {}); reserve_for_response={}, budget for fixed+history={budget}",
            source.as_str(),
            self.config.reserve_for_response
        );

        let fixed_tokens = self.count_fixed_parts(&cx.assembly, &model);

        if fixed_tokens > budget {
            warn!(
                "history_trimmer: fixed prompt parts ({fixed_tokens} tokens) already exceed the \
                 resolved budget ({budget} tokens = limit {limit} minus \
                 reserve_for_response {}) -- history for this iteration is empty, run proceeds",
                self.config.reserve_for_response
            );
            cx.assembly.history.clear();
            return Ok(MiddlewareFlow::Continue);
        }

        let mut remaining = budget - fixed_tokens;
        let mut kept_newest_first: Vec<GarrisonEntry> = Vec::new();
        for entry in cx.assembly.history.iter().rev() {
            let entry_tokens = self.counter.count(&entry.content, &model);
            if entry_tokens > remaining {
                // Newest-first admission stops at the first entry that does
                // not fit -- kept whole or dropped whole, never truncated
                // (D-15).
                break;
            }
            remaining -= entry_tokens;
            kept_newest_first.push(entry.clone());
        }
        kept_newest_first.reverse();
        cx.assembly.history = kept_newest_first;

        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "history_trimmer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::middleware::{
        ModelCallContext, PromptAssembly, PromptSection, SectionPlacement,
    };
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::garrison::{ConversationRole, GarrisonEntry};
    use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use paladin_ports::output::llm_port::{
        LlmError, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
    };
    use std::collections::HashMap;

    /// A minimal `LlmPort` test fixture exposing only a configured
    /// `get_capabilities().max_context_tokens` -- every other method is
    /// never called by these tests and returns a `ProcessingError`.
    struct CapabilityOnlyLlmPort {
        max_context_tokens: Option<u32>,
    }

    #[async_trait::async_trait]
    impl LlmPort for CapabilityOnlyLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::ProcessingError(
                "CapabilityOnlyLlmPort: generate is not implemented in this test fixture"
                    .to_string(),
            ))
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            Err(LlmError::ProcessingError(
                "CapabilityOnlyLlmPort: generate_stream is not implemented in this test fixture"
                    .to_string(),
            ))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(Vec::new())
        }

        fn get_provider_name(&self) -> &'static str {
            "capability-only-test-fixture"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                max_context_tokens: self.max_context_tokens,
                ..ProviderCapabilities::default()
            }
        }
    }

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

    fn make_llm_port(max_context_tokens: Option<u32>) -> Arc<dyn LlmPort> {
        Arc::new(CapabilityOnlyLlmPort { max_context_tokens })
    }

    fn base_config() -> HistoryTrimmerConfig {
        HistoryTrimmerConfig {
            enabled: true,
            reserve_for_response: 0,
            default_context_tokens: 500,
            model_context_limits: HashMap::new(),
            recall_limit: 20,
        }
    }

    fn entry(role: ConversationRole, content: &str) -> GarrisonEntry {
        GarrisonEntry::new(role, content.to_string())
    }

    /// Test 1: with `model_context_limits: {"gpt-4": 1000}` and a port
    /// reporting `max_context_tokens: 8000`, the resolved limit is 1000.
    #[test]
    fn limit_resolution_prefers_the_config_table() {
        let mut config = base_config();
        config
            .model_context_limits
            .insert("gpt-4".to_string(), 1000);
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(Some(8000)));

        let (limit, source) = trimmer.resolve_limit("gpt-4");
        assert_eq!(limit, 1000);
        assert!(source.as_str().contains("config"));
    }

    /// Test 2: with an empty table and a port reporting 8000, the resolved
    /// limit is 8000.
    #[test]
    fn limit_resolution_falls_back_to_provider_capabilities() {
        let config = base_config();
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(Some(8000)));

        let (limit, source) = trimmer.resolve_limit("gpt-4");
        assert_eq!(limit, 8000);
        assert!(source.as_str().contains("provider") || source.as_str().contains("capabilities"));
    }

    /// Test 3: with an empty table and a port reporting nothing, the
    /// resolved limit is `default_context_tokens`.
    #[test]
    fn limit_resolution_falls_back_to_the_default() {
        let config = base_config();
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(None));

        let (limit, source) = trimmer.resolve_limit("gpt-4");
        assert_eq!(limit, 500);
        assert!(source.as_str().contains("default"));
    }

    /// Test 4: system prompt, retrieved context, current input and
    /// accumulated output survive trimming even when the history does not.
    #[tokio::test]
    async fn fixed_parts_are_always_kept() {
        let mut config = base_config();
        config.default_context_tokens = 1; // budget cannot fit anything but rounds to at least 0
        config.reserve_for_response = 0;
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history = vec![entry(ConversationRole::User, "some prior message")];
        let assembly = PromptAssembly::new(
            "system prompt text",
            "current input text",
            "accumulated output text",
            history,
            Some("retrieved context text".to_string()),
        );
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        trimmer.before_model(&mut cx).await.unwrap();

        assert_eq!(cx.assembly.system, "system prompt text");
        assert_eq!(
            cx.assembly.retrieved_context.as_deref(),
            Some("retrieved context text")
        );
        assert_eq!(cx.assembly.input, "current input text");
        assert_eq!(cx.assembly.accumulated_output, "accumulated output text");
    }

    /// Test 5: with a limit that fits exactly three of ten equally-sized
    /// entries, the three newest are kept, in original order.
    #[tokio::test]
    async fn history_is_admitted_newest_first_until_the_budget() {
        let counter = make_counter();
        let one_entry_tokens = counter.count("entry-0", "gpt-4");

        let mut config = base_config();
        config.reserve_for_response = 0;
        config.default_context_tokens = one_entry_tokens * 3; // room for exactly 3, no fixed parts
        let trimmer = HistoryTrimmer::new(config, counter, make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history: Vec<GarrisonEntry> = (0..10)
            .map(|i| entry(ConversationRole::User, &format!("entry-{i}")))
            .collect();
        let assembly = PromptAssembly::new("", "", "", history, None);
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        trimmer.before_model(&mut cx).await.unwrap();

        let kept: Vec<&str> = cx
            .assembly
            .history
            .iter()
            .map(|e| e.content.as_str())
            .collect();
        assert_eq!(kept, vec!["entry-7", "entry-8", "entry-9"]);
    }

    /// Test 6: with a limit that would fit half of the next entry, that
    /// entry is dropped entirely -- no entry's content is ever truncated.
    #[tokio::test]
    async fn an_entry_is_kept_whole_or_dropped_whole() {
        let counter = make_counter();
        let small_tokens = counter.count("s", "gpt-4");
        let big_tokens = counter.count("a much bigger entry than the others here", "gpt-4");

        let mut config = base_config();
        config.reserve_for_response = 0;
        // Room for the newest small entry plus HALF the next (big) one --
        // not enough for the big one whole.
        config.default_context_tokens = small_tokens + (big_tokens / 2);
        let trimmer = HistoryTrimmer::new(config, counter, make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history = vec![
            entry(
                ConversationRole::User,
                "a much bigger entry than the others here",
            ),
            entry(ConversationRole::User, "s"),
        ];
        let assembly = PromptAssembly::new("", "", "", history, None);
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        trimmer.before_model(&mut cx).await.unwrap();

        let kept: Vec<&str> = cx
            .assembly
            .history
            .iter()
            .map(|e| e.content.as_str())
            .collect();
        assert_eq!(
            kept,
            vec!["s"],
            "the big entry must be dropped whole, never truncated"
        );
    }

    /// Test 7: raising `reserve_for_response` by exactly one entry's token
    /// count drops exactly one more entry.
    #[tokio::test]
    async fn reserve_for_response_is_subtracted() {
        let counter = make_counter();
        let one_entry_tokens = counter.count("entry-0", "gpt-4");

        let history: Vec<GarrisonEntry> = (0..5)
            .map(|i| entry(ConversationRole::User, &format!("entry-{i}")))
            .collect();

        let mut config_a = base_config();
        config_a.reserve_for_response = 0;
        config_a.default_context_tokens = one_entry_tokens * 3;
        let trimmer_a = HistoryTrimmer::new(config_a, counter.clone(), make_llm_port(None));
        let paladin = make_paladin("gpt-4");
        let assembly_a = PromptAssembly::new("", "", "", history.clone(), None);
        let mut cx_a = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly_a);
        trimmer_a.before_model(&mut cx_a).await.unwrap();
        assert_eq!(cx_a.assembly.history.len(), 3);

        let mut config_b = base_config();
        config_b.reserve_for_response = one_entry_tokens;
        config_b.default_context_tokens = one_entry_tokens * 3;
        let trimmer_b = HistoryTrimmer::new(config_b, counter, make_llm_port(None));
        let assembly_b = PromptAssembly::new("", "", "", history, None);
        let mut cx_b = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly_b);
        trimmer_b.before_model(&mut cx_b).await.unwrap();
        assert_eq!(
            cx_b.assembly.history.len(),
            2,
            "reserving one more entry's worth of tokens drops exactly one more entry"
        );
    }

    /// Test 8: when the fixed parts alone exceed the effective budget, the
    /// history is empty and the run proceeds (never fails).
    #[tokio::test]
    async fn oversized_fixed_parts_yield_an_empty_history_and_the_run_proceeds() {
        let mut config = base_config();
        config.reserve_for_response = 0;
        config.default_context_tokens = 1; // far smaller than the fixed system prompt below
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history = vec![entry(ConversationRole::User, "some history")];
        let assembly = PromptAssembly::new(
            "a system prompt long enough to blow the budget all on its own",
            "input",
            "",
            history,
            None,
        );
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        let outcome = trimmer.before_model(&mut cx).await;

        assert!(outcome.is_ok(), "the run must proceed, never fail");
        assert!(cx.assembly.history.is_empty());
    }

    /// Test 9: the same inputs produce the identical kept set across 20
    /// runs and across a freshly constructed counter/trimmer instance.
    #[tokio::test]
    async fn trimming_is_stable_across_repetitions_and_instances() {
        let history: Vec<GarrisonEntry> = (0..10)
            .map(|i| entry(ConversationRole::User, &format!("entry-{i}")))
            .collect();
        let paladin = make_paladin("gpt-4");

        let run_once = || async {
            let counter = make_counter();
            let one_entry_tokens = counter.count("entry-0", "gpt-4");
            let mut config = base_config();
            config.reserve_for_response = 0;
            config.default_context_tokens = one_entry_tokens * 4;
            let trimmer = HistoryTrimmer::new(config, counter, make_llm_port(None));
            let assembly = PromptAssembly::new("", "", "", history.clone(), None);
            let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);
            trimmer.before_model(&mut cx).await.unwrap();
            cx.assembly
                .history
                .iter()
                .map(|e| e.content.clone())
                .collect::<Vec<_>>()
        };

        let first = run_once().await;
        for _ in 0..20 {
            assert_eq!(run_once().await, first);
        }
    }

    /// Test 10: an entry with `is_summary: true` is admitted or dropped by
    /// the same size rule as any other, with no special case.
    #[tokio::test]
    async fn a_summary_entry_is_an_ordinary_entry_to_the_trimmer() {
        let counter = make_counter();
        let one_entry_tokens = counter.count("entry-0", "gpt-4");

        let mut config = base_config();
        config.reserve_for_response = 0;
        config.default_context_tokens = one_entry_tokens * 2;
        let trimmer = HistoryTrimmer::new(config, counter, make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history = vec![
            entry(ConversationRole::User, "entry-0"),
            GarrisonEntry::summary("entry-1".to_string()),
            entry(ConversationRole::User, "entry-2"),
        ];
        let assembly = PromptAssembly::new("", "", "", history, None);
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        trimmer.before_model(&mut cx).await.unwrap();

        let kept: Vec<&str> = cx
            .assembly
            .history
            .iter()
            .map(|e| e.content.as_str())
            .collect();
        assert_eq!(
            kept,
            vec!["entry-1", "entry-2"],
            "the summary entry is kept purely by newest-first size admission"
        );
    }

    /// A disabled trimmer changes nothing.
    #[tokio::test]
    async fn disabled_trimmer_changes_nothing() {
        let mut config = base_config();
        config.enabled = false;
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history = vec![entry(ConversationRole::User, "untouched")];
        let assembly = PromptAssembly::new("system", "input", "", history, None);
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        trimmer.before_model(&mut cx).await.unwrap();

        assert_eq!(cx.assembly.history.len(), 1);
        assert_eq!(cx.assembly.history[0].content, "untouched");
    }

    /// The `before_model` implementation never returns `MiddlewareFlow::Fail`.
    #[tokio::test]
    async fn history_trimmer_never_fails_the_run() {
        let mut config = base_config();
        config.default_context_tokens = 0;
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history = vec![entry(ConversationRole::User, "x")];
        let assembly = PromptAssembly::new("system", "input", "", history, None);
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        assert!(trimmer.before_model(&mut cx).await.is_ok());
    }

    /// A `PromptSection` pushed onto the assembly is counted as part of the
    /// fixed parts.
    #[tokio::test]
    async fn pushed_sections_count_toward_the_fixed_parts() {
        let mut config = base_config();
        config.reserve_for_response = 0;
        config.default_context_tokens = 1;
        let trimmer = HistoryTrimmer::new(config, make_counter(), make_llm_port(None));

        let paladin = make_paladin("gpt-4");
        let history = vec![entry(ConversationRole::User, "history entry")];
        let mut assembly = PromptAssembly::new("s", "i", "", history, None);
        assembly.push_section(PromptSection::new(
            "Heading",
            "a section body long enough to matter",
            SectionPlacement::End,
        ));
        let mut cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);

        trimmer.before_model(&mut cx).await.unwrap();

        assert!(cx.assembly.history.is_empty());
    }
}
