//! Per-run value types the [`super::ExecutionMiddleware`] chain reads and
//! mutates: the structured [`PromptAssembly`] that replaces the flat prompt
//! buffer (D-02), and the per-call contexts (D-03, D-04).
//!
//! # `PromptAssembly` replaces a flat `String`, not a message list
//!
//! Today [`crate::application::services::paladin::paladin_execution_service::PaladinExecutionService`]
//! builds one flat `String` per loop iteration and sends it as a
//! `PromptType::User(UserPrompt { query })`. [`PromptAssembly`] carries the
//! same pieces (system prompt, retrieved RAG context, Garrison history,
//! user input, accumulated output) as named fields plus an ordered list of
//! [`PromptSection`]s a middleware can push, and [`PromptAssembly::render`]
//! produces **exactly** the string `build_prompt_with_custom_system` used to
//! build directly. With no pushed sections the rendered bytes are
//! byte-identical to before this phase (the locked D-02 invariant, proven by
//! the `empty_chain_renders_byte_identical_prompt` golden test).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::core::platform::container::arsenal::ArmamentCall;
use crate::core::platform::container::garrison::{ConversationRole, GarrisonEntry};
use crate::core::platform::container::paladin::Paladin;
use paladin_core::platform::container::aegis::RetryPolicy;
use paladin_ports::output::llm_port::{FinishReason, FunctionCall, LlmPort, TokenUsage};
use paladin_ports::output::vault_confined::ConfinedVault;

/// Where a middleware-pushed [`PromptSection`] renders relative to the
/// fixed parts of a [`PromptAssembly`] (D-25's "after retrieved RAG context,
/// before history" placement is the first variant here).
///
/// `#[non_exhaustive]`: a later plan may need a finer-grained position
/// without breaking every existing `match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SectionPlacement {
    /// Immediately after the retrieved-context block, before conversation
    /// history — where `VaultRecallMiddleware` (plan 26-15) frames recalled
    /// memory.
    AfterRetrievedContext,
    /// Immediately before the conversation-history block.
    BeforeHistory,
    /// After everything else (input and accumulated output), at the very
    /// end of the rendered prompt.
    End,
}

/// One block of text a middleware inserts into the rendered prompt, framed
/// under its own heading (T-26-13: structured placement, never
/// concatenation into the system prompt, is what keeps recalled/model text
/// legible as data rather than instructions).
#[derive(Debug, Clone)]
pub struct PromptSection {
    /// Rendered as a Markdown `## heading` line.
    pub heading: String,
    /// The section body, rendered verbatim beneath the heading.
    pub body: String,
    /// Where this section renders relative to the assembly's fixed parts.
    pub placement: SectionPlacement,
}

impl PromptSection {
    /// Construct a new section.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::application::services::paladin::middleware::{PromptSection, SectionPlacement};
    ///
    /// let section = PromptSection::new("Recalled Memory", "the user prefers dark mode", SectionPlacement::AfterRetrievedContext);
    /// assert_eq!(section.heading, "Recalled Memory");
    /// ```
    pub fn new(
        heading: impl Into<String>,
        body: impl Into<String>,
        placement: SectionPlacement,
    ) -> Self {
        Self {
            heading: heading.into(),
            body: body.into(),
            placement,
        }
    }
}

/// The structured stand-in for today's flat prompt `String` (D-02).
///
/// Built once per reasoning-loop iteration from exactly the inputs
/// `build_prompt_with_custom_system` used, then handed to the
/// `before_model` chain (which may push [`PromptSection`]s or edit its
/// fields) and finally rendered via [`PromptAssembly::render`] through the
/// unchanged `PromptType::User(UserPrompt { query })` path. No middleware
/// ever sees or mutates the rendered `String` directly.
#[derive(Debug, Clone)]
pub struct PromptAssembly {
    /// The effective system prompt for this iteration (Layer 1 generated,
    /// or the Paladin's own).
    pub system: String,
    /// Retrieved RAG context, if Sanctum retrieval succeeded and returned
    /// non-empty results.
    pub retrieved_context: Option<String>,
    /// Garrison conversation history for this run.
    pub history: Vec<GarrisonEntry>,
    /// The user's input for this execution.
    pub input: String,
    /// Output accumulated from previous loop iterations (empty on the
    /// first iteration).
    pub accumulated_output: String,
    /// Sections pushed by `before_model` hooks, rendered at their
    /// declared [`SectionPlacement`].
    pub sections: Vec<PromptSection>,
}

impl PromptAssembly {
    /// Construct an assembly from exactly the inputs
    /// `build_prompt_with_custom_system` takes today, with an empty
    /// section list.
    pub fn new(
        system: impl Into<String>,
        input: impl Into<String>,
        accumulated_output: impl Into<String>,
        history: Vec<GarrisonEntry>,
        retrieved_context: Option<String>,
    ) -> Self {
        Self {
            system: system.into(),
            retrieved_context,
            history,
            input: input.into(),
            accumulated_output: accumulated_output.into(),
            sections: Vec::new(),
        }
    }

    /// Push a section onto the assembly. Order among sections sharing the
    /// same [`SectionPlacement`] is push order.
    pub fn push_section(&mut self, section: PromptSection) {
        self.sections.push(section);
    }

    fn sections_at(&self, placement: SectionPlacement) -> impl Iterator<Item = &PromptSection> {
        self.sections
            .iter()
            .filter(move |section| section.placement == placement)
    }

    /// Render the assembly to the flat prompt string sent to the model.
    ///
    /// With `sections` empty this is byte-identical to today's
    /// `build_prompt_with_custom_system` output (D-02's locked invariant).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::application::services::paladin::middleware::PromptAssembly;
    ///
    /// let assembly = PromptAssembly::new("You are helpful", "hi", "", vec![], None);
    /// let rendered = assembly.render();
    /// assert!(rendered.starts_with("You are helpful\n\n"));
    /// assert!(rendered.contains("User: hi\n"));
    /// ```
    pub fn render(&self) -> String {
        let mut prompt = format!("{}\n\n", self.system);

        if let Some(context) = self.retrieved_context.as_deref()
            && !context.is_empty()
        {
            prompt.push_str("## Relevant Context from Memory\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }

        for section in self.sections_at(SectionPlacement::AfterRetrievedContext) {
            prompt.push_str(&format!("## {}\n{}\n\n", section.heading, section.body));
        }

        for section in self.sections_at(SectionPlacement::BeforeHistory) {
            prompt.push_str(&format!("## {}\n{}\n\n", section.heading, section.body));
        }

        if !self.history.is_empty() {
            prompt.push_str("Previous conversation:\n");
            for entry in self.history.iter().rev().take(10).rev() {
                let role_str = match entry.role {
                    ConversationRole::System => "System",
                    ConversationRole::User => "User",
                    ConversationRole::Assistant => "Assistant",
                    ConversationRole::Tool => "Tool",
                };
                prompt.push_str(&format!("{}: {}\n", role_str, entry.content));
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!("User: {}\n", self.input));

        if !self.accumulated_output.is_empty() {
            prompt.push_str(&format!("Previous output: {}\n", self.accumulated_output));
        }

        for section in self.sections_at(SectionPlacement::End) {
            prompt.push_str(&format!("## {}\n{}\n\n", section.heading, section.body));
        }

        prompt
    }
}

/// Per-model-call context threaded through `before_model`/`after_model`
/// (D-03, D-04).
///
/// One `ModelCallContext` is constructed per `execute_internal` run (not per
/// loop iteration): `scratch` and the typed state bag live for the whole
/// run, so a built-in like `ModelCallLimit` can keep a running count without
/// a `MiddlewareFactory` (D-03). `loop_index` and `cumulative_tokens` are
/// updated by the service before each hook fires.
///
/// A handoff **specialist's** run is its own run with its own
/// `ModelCallContext` and its own scratch -- it does not share this run's
/// state. Layer-1 planning calls, prompt-generation calls, and a
/// summarizer's own model call are outside any run's middleware chain
/// entirely.
pub struct ModelCallContext<'p> {
    /// Stable identifier for the whole `execute_internal` run.
    pub run_id: Uuid,
    /// The reasoning loop's current iteration, starting at `0`.
    pub loop_index: u32,
    /// Total tokens (prompt + completion) accumulated across every model
    /// call so far in this run, updated by the service immediately after
    /// each call returns and before `after_model` fires.
    pub cumulative_tokens: u32,
    /// The prompt under construction for this iteration.
    pub assembly: PromptAssembly,
    /// Read-only handle to the Paladin being executed.
    paladin: &'p Paladin,
    /// Untyped, PRD-named scratch a middleware can read/write freely.
    pub scratch: HashMap<String, Value>,
    /// A port-shaping override read once at the single model-call site
    /// (`execute_with_retry_and_temperature`): when `Some`, that call uses
    /// this port instead of the service's own (`ModelFallbackMiddleware`,
    /// plan 26-10).
    pub llm_override: Option<Arc<dyn LlmPort>>,
    /// A port-shaping retry policy override read once at the same call
    /// site (`ModelRetryMiddleware`, plan 26-10).
    pub retry_policy: Option<RetryPolicy>,
    /// The confined Vault handle granted to this run, if any (D-21, D-25).
    /// `None` means this run has no Vault grant at all -- read by
    /// `VaultRecallMiddleware`, set exactly once by the service (in
    /// `execute_internal`, right after this context is constructed) from
    /// `PaladinExecutionService::confined_vault(scope)`, and never mutated
    /// by a hook thereafter.
    pub vault: Option<ConfinedVault>,
    typed_state: HashMap<(String, TypeId), Box<dyn Any + Send + Sync>>,
}

impl<'p> ModelCallContext<'p> {
    /// Construct a new context for a run, with an empty scratch and typed
    /// state bag.
    pub fn new(run_id: Uuid, paladin: &'p Paladin, assembly: PromptAssembly) -> Self {
        Self {
            run_id,
            loop_index: 0,
            cumulative_tokens: 0,
            assembly,
            paladin,
            scratch: HashMap::new(),
            llm_override: None,
            retry_policy: None,
            vault: None,
            typed_state: HashMap::new(),
        }
    }

    /// The Paladin being executed.
    pub fn paladin(&self) -> &Paladin {
        self.paladin
    }

    /// The effective [`LlmPort`] for this model call (Doc 05
    /// assumption-delta decision, plan 26-10, D-11).
    ///
    /// Before Phase 26's retry/fallback middleware, a run had exactly one
    /// port: the service's own. This accessor is the ONE place that
    /// resolution happens now: `llm_override` if a port-shaping middleware
    /// (`ModelFallbackMiddleware`) set one in `before_model`, otherwise
    /// `service_default` -- seeded here as the default *source*, not sitting
    /// on the far side of an `else` branch. It is read exactly once, at the
    /// single model-call site
    /// (`PaladinExecutionService::execute_with_retry_and_temperature`); no
    /// other call site chooses a port. A future source of port choice (e.g.
    /// Phase 27's `RunScope`-derived selection) adds a new default source
    /// here, not a second resolution branch elsewhere -- see
    /// `model_call_port_is_resolved_at_exactly_one_point`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use paladin::application::services::paladin::middleware::{ModelCallContext, PromptAssembly};
    /// use paladin_llm::mock::MockLlmAdapter;
    /// use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin::core::base::entity::node::Node;
    /// # use paladin::core::platform::container::paladin::PaladinData;
    ///
    /// let paladin = Node::new(PaladinData::default(), None);
    /// let assembly = PromptAssembly::new("system", "input", "", vec![], None);
    /// let cx = ModelCallContext::new(uuid::Uuid::new_v4(), &paladin, assembly);
    /// let service_default: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
    ///
    /// // With no override, the service default is returned.
    /// assert_eq!(
    ///     cx.effective_llm(&service_default).get_provider_name(),
    ///     service_default.get_provider_name()
    /// );
    /// ```
    pub fn effective_llm(&self, service_default: &Arc<dyn LlmPort>) -> Arc<dyn LlmPort> {
        self.llm_override
            .clone()
            .unwrap_or_else(|| Arc::clone(service_default))
    }

    /// Read (or default-construct) a middleware's typed per-run state,
    /// keyed by `middleware_name` and `T`'s `TypeId` -- two middleware with
    /// different names never collide, and one middleware storing two
    /// different `T`s sees both independently.
    pub fn state<T: Default + Send + Sync + 'static>(&mut self, middleware_name: &str) -> &T {
        self.state_mut::<T>(middleware_name)
    }

    /// Mutably read (or default-construct) a middleware's typed per-run
    /// state. See [`ModelCallContext::state`].
    pub fn state_mut<T: Default + Send + Sync + 'static>(
        &mut self,
        middleware_name: &str,
    ) -> &mut T {
        let key = (middleware_name.to_string(), TypeId::of::<T>());
        let entry = self
            .typed_state
            .entry(key)
            .or_insert_with(|| Box::new(T::default()));
        entry
            .downcast_mut::<T>()
            .expect("ModelCallContext typed state: TypeId collision should be impossible")
    }
}

/// A mutable view over an [`paladin_ports::output::llm_port::LlmResponse`]
/// passed to `after_model` (D-04).
///
/// `content` and `function_call` are mutable so a middleware can rewrite the
/// model's answer or synthesize a function call (plan 26-19);
/// `usage`/`finish_reason` are read-only by convention (mutating them has no
/// effect on the service's own token accounting, which reads the original
/// response before this view is built).
#[derive(Debug, Clone)]
pub struct LlmResponseView {
    /// The model's response text. Mutable: an `after_model` hook may
    /// rewrite it (e.g. redaction, guardrail `Redact`).
    pub content: String,
    /// Token usage reported with the response. Read-only by convention.
    pub usage: TokenUsage,
    /// Why generation stopped. Read-only by convention.
    pub finish_reason: FinishReason,
    /// A tool/function call the model requested, if any. Mutable: a
    /// tool-call-protocol middleware (plan 26-19) may synthesize one from a
    /// prompt-level convention no shipped adapter emits natively.
    pub function_call: Option<FunctionCall>,
}

impl LlmResponseView {
    /// Build a view from an [`paladin_ports::output::llm_port::LlmResponse`].
    pub fn from_response(response: &paladin_ports::output::llm_port::LlmResponse) -> Self {
        Self {
            content: response.content.clone(),
            usage: response.usage.clone(),
            finish_reason: response.finish_reason.clone(),
            function_call: response.function_call.clone(),
        }
    }
}

/// Which dispatch path produced a [`ToolCallContext`] (D-04: a handoff is a
/// tool call the model made, so both branches route through the same
/// `around_tool` hook).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallKind {
    /// A regular Arsenal tool invocation.
    Armament,
    /// A `handoff_to_specialist` call, dispatched to `HandoffService`.
    Handoff,
}

/// Per-tool-call context passed to `around_tool` (D-04).
#[derive(Debug, Clone)]
pub struct ToolCallContext {
    /// The call the model requested (or, for a handoff, a synthesized
    /// `ArmamentCall` carrying the same name/arguments).
    pub call: ArmamentCall,
    /// Which dispatch path this call is on.
    pub kind: ToolCallKind,
    /// The reasoning loop's current iteration.
    pub loop_index: u32,
    /// Stable identifier for the run this call belongs to.
    pub run_id: Uuid,
    /// A working copy of the run's scratch, seeded from
    /// [`ModelCallContext::scratch`] at dispatch time. `around_tool`
    /// implementations may read and write it freely (Doc 05 D-08); the
    /// service copies it back into the run's own `ModelCallContext::scratch`
    /// after the `around_tool` chain returns, so a value written on one
    /// tool/handoff dispatch is visible on the next -- without ever storing
    /// counter state on the middleware struct itself (D-03).
    pub scratch: HashMap<String, Value>,
}
