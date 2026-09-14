//! `ScenarioLlm` -- the scripted [`LlmPort`] a running scenario substitutes for a real
//! provider (D-30).
//!
//! `paladin-eval` owns its scripted LLM outright rather than extending
//! `paladin-llm`'s [`MockLlmAdapter`](paladin_llm::mock::MockLlmAdapter): that type is
//! public and may be pre-existing at `v0.9.0`, and touching it would be an X-10
//! register event for a capability this crate can own instead. `ScenarioLlm` provides
//! per-node routing (one instance per Paladin node, via [`ScenarioLlm::for_node`]),
//! sequence scripts, prompt-substring `match` rules checked before either sequence, and
//! a captured request log every failure renderer can read.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use thiserror::Error;
use uuid::Uuid;

use paladin_core::platform::container::prompt::PromptType;
use paladin_ports::output::llm_port::{
    FinishReason, FunctionCall, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities,
    StreamingResponse, TokenUsage,
};

use crate::scenario::{LlmScript, ScriptEntry};

/// One request [`ScenarioLlm`] received, captured for failure rendering (D-29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRequest {
    /// The node this call was attributed to (via [`ScenarioLlm::for_node`]).
    pub node_id: String,
    /// This call's position among every call [`ScenarioLlm::requests`] has captured for
    /// `node_id` (0-based) -- the same index a [`ScenarioLlmError::SequenceExhausted`]
    /// names.
    pub call_index: usize,
    /// The rendered prompt text (`PromptType::User.query` or `PromptType::System.instructions`;
    /// any other prompt variant captures as an empty string, mirroring
    /// `MockLlmAdapter::last_prompt`'s own convention).
    pub prompt: String,
}

/// Errors [`ScenarioLlm`] itself can raise, distinct from a scripted
/// [`crate::scenario::LlmErrorKind`] (which becomes a real [`LlmError`] via
/// [`crate::scenario::LlmErrorKind::to_llm_error`], never this type).
///
/// `#[non_exhaustive]`: a future variant (e.g. a malformed script caught at
/// construction) can be added without a semver-major bump.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ScenarioLlmError {
    /// Node `node`'s resolved sequence (its own per-node script, or the scenario's
    /// global script when it has none) had no entry left for call `call_index`, and no
    /// `match` rule matched the incoming prompt either. Never a panic (D-30's
    /// behaviour contract) -- always this typed, node- and call-index-naming error.
    #[error(
        "ScenarioLlm: node {node:?}'s scripted sequence is exhausted at call {call_index} \
         (no match rule matched, and no script entry remained)"
    )]
    SequenceExhausted {
        /// The node whose sequence was exhausted.
        node: String,
        /// The 0-based call index (this node's position in [`ScenarioLlm::requests`])
        /// at which exhaustion was detected.
        call_index: usize,
    },
}

impl From<ScenarioLlmError> for LlmError {
    /// `LlmPort::generate` returns `Result<_, LlmError>` -- [`LlmError`] is
    /// `#[non_exhaustive]` and owned by `paladin-ports`, so this crate cannot add a
    /// dedicated variant to it. The typed [`ScenarioLlmError`] is still constructed and
    /// available to a caller matching on it directly; this conversion folds its
    /// `Display` rendering (node + call index intact) into
    /// [`LlmError::ProcessingError`] for the trait boundary.
    fn from(error: ScenarioLlmError) -> Self {
        LlmError::ProcessingError(error.to_string())
    }
}

/// State shared across every [`ScenarioLlm::for_node`] clone of the same scenario's
/// script (D-30): the resolved script is immutable, but sequence cursors, per-node call
/// counters and the captured request log are consulted and advanced by every node's
/// clone.
struct SharedState {
    script: LlmScript,
    global_cursor: Mutex<usize>,
    per_node_cursors: Mutex<BTreeMap<String, usize>>,
    per_node_call_counts: Mutex<BTreeMap<String, usize>>,
    requests: Mutex<Vec<CapturedRequest>>,
}

/// A scripted [`LlmPort`] implementation a running scenario substitutes for a real
/// provider (D-30).
///
/// # Resolution order (per call, D-30)
///
/// 1. [`crate::scenario::MatchRule`]s, in declaration order -- the first whose
///    `prompt_contains` is a substring of the incoming prompt wins. Matching does
///    **not** advance any sequence cursor.
/// 2. The per-node sequence for [`ScenarioLlm`]'s own node id
///    ([`crate::scenario::LlmScript::per_node`]), if the scenario's script declares
///    one for this node at all -- a node with a declared (even if since-exhausted)
///    per-node sequence never falls through to the global sequence.
/// 3. The scenario's global sequence ([`crate::scenario::LlmScript::global`]),
///    consumed one entry per call.
///
/// Exhausting whichever sequence applies is a typed [`ScenarioLlmError::SequenceExhausted`],
/// never a panic.
///
/// # Example
///
/// ```rust
/// use paladin_eval::scenario::{LlmScript, ScriptEntry};
/// use paladin_eval::scripted_llm::ScenarioLlm;
///
/// let script = LlmScript {
///     global: vec![ScriptEntry::Text("hello".to_string())],
///     ..Default::default()
/// };
/// let llm = ScenarioLlm::new(script).for_node("planner");
/// assert_eq!(llm.requests().len(), 0);
/// ```
#[derive(Clone)]
pub struct ScenarioLlm {
    shared: Arc<SharedState>,
    node_id: String,
}

impl ScenarioLlm {
    /// Build a [`ScenarioLlm`] over `script`, attributed to no node in particular
    /// (`node_id` is empty). Call [`ScenarioLlm::for_node`] once per Paladin node
    /// before wiring an instance into that node's `LlmPort` slot -- the returned clone
    /// shares this instance's cursors, call counters and captured request log.
    pub fn new(script: LlmScript) -> Self {
        Self {
            shared: Arc::new(SharedState {
                script,
                global_cursor: Mutex::new(0),
                per_node_cursors: Mutex::new(BTreeMap::new()),
                per_node_call_counts: Mutex::new(BTreeMap::new()),
                requests: Mutex::new(Vec::new()),
            }),
            node_id: String::new(),
        }
    }

    /// Return a clone of this [`ScenarioLlm`] attributed to `node_id`, sharing the same
    /// underlying script, sequence cursors, call counters and captured request log.
    /// The runner hands one such clone to each Paladin node's `LlmPort` slot (D-31).
    #[must_use]
    pub fn for_node(&self, node_id: impl Into<String>) -> ScenarioLlm {
        ScenarioLlm {
            shared: Arc::clone(&self.shared),
            node_id: node_id.into(),
        }
    }

    /// Every request this [`ScenarioLlm`] (across every [`ScenarioLlm::for_node`] clone
    /// sharing its state) has received so far, in call order.
    ///
    /// # Threat accepted here (28-05-PLAN.md `<threat_model>`, T-28-05-03)
    ///
    /// Each [`CapturedRequest`] carries the rendered prompt text, and failure renderers
    /// echo it into the developer console or CI log. This is accepted: the log is
    /// test-harness-local and exists precisely to make a failure actionable, and
    /// scenarios are scripted with fixture prompts, never production data.
    pub fn requests(&self) -> Vec<CapturedRequest> {
        self.shared
            .requests
            .lock()
            .expect("requests lock poisoned")
            .clone()
    }

    /// Extract the rendered prompt text from an [`LlmRequest`], mirroring
    /// `MockLlmAdapter::last_prompt`'s own convention: `PromptType::User.query` or
    /// `PromptType::System.instructions`; any other prompt variant yields an empty
    /// string.
    fn prompt_text(request: &LlmRequest) -> String {
        match request.prompt.prompt_type() {
            PromptType::User(user) => user.query.clone(),
            PromptType::System(system) => system.instructions.clone(),
            _ => String::new(),
        }
    }

    /// Record `prompt` as this node's next call, returning the 0-based call index just
    /// assigned.
    fn record_request(&self, prompt: &str) -> usize {
        let call_index = {
            let mut counts = self
                .shared
                .per_node_call_counts
                .lock()
                .expect("per_node_call_counts lock poisoned");
            let entry = counts.entry(self.node_id.clone()).or_insert(0);
            let index = *entry;
            *entry += 1;
            index
        };
        self.shared
            .requests
            .lock()
            .expect("requests lock poisoned")
            .push(CapturedRequest {
                node_id: self.node_id.clone(),
                call_index,
                prompt: prompt.to_string(),
            });
        call_index
    }

    /// Resolve this call's [`ScriptEntry`], per the resolution order documented on
    /// [`ScenarioLlm`], or a typed [`ScenarioLlmError::SequenceExhausted`].
    fn resolve_entry(&self, prompt: &str) -> Result<ScriptEntry, ScenarioLlmError> {
        // 1. Match rules, in declaration order, first match wins -- no cursor advance.
        for rule in &self.shared.script.match_rules {
            if prompt.contains(&rule.prompt_contains) {
                return Ok(rule.response.clone());
            }
        }

        // 2. A declared per-node sequence for this node is authoritative -- it never
        //    falls through to the global sequence, even once exhausted.
        if let Some(sequence) = self.shared.script.per_node.get(&self.node_id) {
            let mut cursors = self
                .shared
                .per_node_cursors
                .lock()
                .expect("per_node_cursors lock poisoned");
            let cursor = cursors.entry(self.node_id.clone()).or_insert(0);
            return match sequence.get(*cursor) {
                Some(entry) => {
                    *cursor += 1;
                    Ok(entry.clone())
                }
                None => Err(ScenarioLlmError::SequenceExhausted {
                    node: self.node_id.clone(),
                    call_index: 0, // overwritten by the caller with the real call index
                }),
            };
        }

        // 3. The scenario's global sequence, consumed one entry per call.
        let mut global_cursor = self
            .shared
            .global_cursor
            .lock()
            .expect("global_cursor lock poisoned");
        match self.shared.script.global.get(*global_cursor) {
            Some(entry) => {
                *global_cursor += 1;
                Ok(entry.clone())
            }
            None => Err(ScenarioLlmError::SequenceExhausted {
                node: self.node_id.clone(),
                call_index: 0, // overwritten by the caller with the real call index
            }),
        }
    }

    /// Turn a resolved [`ScriptEntry`] into the `LlmPort::generate` result it scripts.
    fn build_response(
        &self,
        entry: &ScriptEntry,
        request: &LlmRequest,
        call_index: usize,
    ) -> Result<LlmResponse, LlmError> {
        match entry {
            ScriptEntry::Text(text) => Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model.clone(),
                content: text.clone(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: None,
            }),
            ScriptEntry::ToolCall { name, arguments } => Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model.clone(),
                content: format!("Calling tool: {name}"),
                finish_reason: FinishReason::FunctionCall,
                usage: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: Some(FunctionCall {
                    name: name.clone(),
                    arguments: arguments.to_string(),
                }),
            }),
            ScriptEntry::Error(kind) => Err(kind.to_llm_error(&self.node_id, call_index)),
        }
    }
}

#[async_trait]
impl LlmPort for ScenarioLlm {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let prompt = Self::prompt_text(&request);
        let call_index = self.record_request(&prompt);

        let entry = self.resolve_entry(&prompt).map_err(|error| match error {
            ScenarioLlmError::SequenceExhausted { node, .. } => {
                ScenarioLlmError::SequenceExhausted { node, call_index }
            }
        })?;

        self.build_response(&entry, &request, call_index)
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        let response = self.generate(request).await?;
        let chunks = vec![
            Ok(StreamingResponse {
                id: Uuid::new_v4(),
                delta: response.content.clone(),
                finish_reason: None,
            }),
            Ok(StreamingResponse {
                id: Uuid::new_v4(),
                delta: String::new(),
                finish_reason: Some(response.finish_reason),
            }),
        ];
        Ok(Box::new(stream::iter(chunks)))
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["scenario-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "ScenarioLlm"
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_function_calling: true,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: None,
            supports_system_messages: true,
            temperature_range: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{LlmErrorKind, MatchRule};
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

    fn request(query: &str) -> LlmRequest {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: query.to_string(),
            context: None,
        }))
        .unwrap();
        LlmRequest::new("scenario-model", prompt)
    }

    #[tokio::test]
    async fn sequence_is_consumed_one_per_call() {
        let script = LlmScript {
            global: vec![
                ScriptEntry::Text("first".to_string()),
                ScriptEntry::Text("second".to_string()),
                ScriptEntry::Text("third".to_string()),
            ],
            ..Default::default()
        };
        let llm = ScenarioLlm::new(script).for_node("solo");

        let r1 = llm.generate(request("go")).await.unwrap();
        let r2 = llm.generate(request("go")).await.unwrap();
        let r3 = llm.generate(request("go")).await.unwrap();
        assert_eq!(r1.content, "first");
        assert_eq!(r2.content, "second");
        assert_eq!(r3.content, "third");

        let err = llm.generate(request("go")).await.unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("solo"),
            "message {message:?} must name the node"
        );
        assert!(
            message.contains('3'),
            "message {message:?} must name the call index (3)"
        );
    }

    #[tokio::test]
    async fn per_node_script_overrides_global() {
        let mut per_node = BTreeMap::new();
        per_node.insert(
            "planner".to_string(),
            vec![ScriptEntry::Text("planner-only".to_string())],
        );
        let script = LlmScript {
            global: vec![ScriptEntry::Text("global-fallback".to_string())],
            per_node,
            ..Default::default()
        };
        let root = ScenarioLlm::new(script);
        let planner = root.for_node("planner");
        let other = root.for_node("other");

        let planner_response = planner.generate(request("go")).await.unwrap();
        assert_eq!(planner_response.content, "planner-only");

        // `other` has no per-node sequence, so it consumes the global sequence --
        // still at its first (and only) entry, proving `planner`'s call left it
        // untouched.
        let other_response = other.generate(request("go")).await.unwrap();
        assert_eq!(other_response.content, "global-fallback");
    }

    #[tokio::test]
    async fn match_rule_wins_over_sequence() {
        let script = LlmScript {
            global: vec![ScriptEntry::Text("sequence-response".to_string())],
            match_rules: vec![MatchRule {
                prompt_contains: "special".to_string(),
                response: ScriptEntry::Text("matched-response".to_string()),
            }],
            ..Default::default()
        };
        let llm = ScenarioLlm::new(script).for_node("solo");

        let matched = llm
            .generate(request("this is a special prompt"))
            .await
            .unwrap();
        assert_eq!(matched.content, "matched-response");

        // The match did not advance the global cursor -- the very next call (a
        // non-matching prompt) still sees the sequence's first (and only) entry.
        let sequenced = llm.generate(request("plain prompt")).await.unwrap();
        assert_eq!(sequenced.content, "sequence-response");
    }

    #[tokio::test]
    async fn error_entry_returns_the_llm_error_kind() {
        let script = LlmScript {
            global: vec![ScriptEntry::Error(LlmErrorKind::Transient)],
            ..Default::default()
        };
        let llm = ScenarioLlm::new(script).for_node("solo");

        let err = llm.generate(request("go")).await.unwrap_err();
        assert!(matches!(err, LlmError::NetworkError(_)));
        assert_eq!(
            err.transience(),
            paladin_core::platform::container::transience::Transience::Transient
        );
    }

    #[tokio::test]
    async fn requests_are_captured_for_failure_rendering() {
        let script = LlmScript {
            global: vec![
                ScriptEntry::Text("first".to_string()),
                ScriptEntry::Text("second".to_string()),
            ],
            ..Default::default()
        };
        let llm = ScenarioLlm::new(script).for_node("planner");

        llm.generate(request("prompt one")).await.unwrap();
        llm.generate(request("prompt two")).await.unwrap();

        let requests = llm.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].node_id, "planner");
        assert_eq!(requests[0].call_index, 0);
        assert_eq!(requests[0].prompt, "prompt one");
        assert_eq!(requests[1].node_id, "planner");
        assert_eq!(requests[1].call_index, 1);
        assert_eq!(requests[1].prompt, "prompt two");
    }

    #[tokio::test]
    async fn tool_call_entry_carries_a_function_call() {
        let script = LlmScript {
            global: vec![ScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({"query": "rust"}),
            }],
            ..Default::default()
        };
        let llm = ScenarioLlm::new(script).for_node("solo");

        let response = llm.generate(request("go")).await.unwrap();
        assert!(matches!(response.finish_reason, FinishReason::FunctionCall));
        let call = response.function_call.expect("tool call must be present");
        assert_eq!(call.name, "search");
        assert!(call.arguments.contains("rust"));
    }
}
