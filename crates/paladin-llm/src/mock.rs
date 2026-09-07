//! Mock LLM adapters for testing.
//!
//! This module provides [`MockLlmAdapter`] and [`MultiStepMockLlmPort`] for
//! use in unit and integration tests that need an in-process LLM adapter
//! without real API calls.

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use paladin_core::platform::container::prompt::PromptType;
use paladin_ports::output::llm_port::{
    FinishReason, FunctionCall, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities,
    StreamingResponse, TokenUsage,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

/// A single mocked response entry — a success string, a tool-call request, or
/// an error.
#[derive(Debug, Clone)]
enum MockEntry {
    Success(String),
    ToolCall { name: String, arguments: String },
    Error(LlmError),
}

/// One scripted entry for [`MockLlmAdapter::with_script`] (Phase 26 D-01/D-02:
/// the `ExecutionMiddleware` chain's `around_tool`/handoff tests need a mock
/// that can emit a [`FunctionCall`], which [`MockLlmAdapter::with_responses`]
/// cannot express).
#[derive(Debug, Clone)]
pub enum MockScriptEntry {
    /// A plain text success response.
    Text(String),
    /// A success response carrying a [`FunctionCall`] (`finish_reason` is
    /// [`FinishReason::FunctionCall`]).
    ToolCall {
        /// The tool/function name the mocked model "calls".
        name: String,
        /// The JSON-encoded arguments string, exactly as a real provider
        /// would return it in [`FunctionCall::arguments`].
        arguments: String,
    },
    /// An error response.
    Error(LlmError),
}

/// Internal state shared between clones of a [`MockLlmAdapter`].
#[derive(Debug)]
struct MockState {
    responses: Vec<MockEntry>,
    response_index: usize,
    delay: Option<Duration>,
    token_usage: TokenUsage,
    finish_reason: FinishReason,
    available_models: Vec<String>,
    call_count: usize,
    provider_name: &'static str,
    stream_script: Option<Vec<Result<String, LlmError>>>,
    model_query_error: Option<LlmError>,
    /// Every request `generate`/`generate_stream` has received, in call
    /// order (Phase 26 D-02: the golden equivalence test inspects the exact
    /// rendered prompt bytes a real run produced).
    requests: Vec<LlmRequest>,
}

impl Default for MockState {
    fn default() -> Self {
        Self {
            responses: vec![MockEntry::Success("Mock LLM response".to_string())],
            response_index: 0,
            delay: None,
            token_usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            finish_reason: FinishReason::Stop,
            available_models: vec!["mock-model".to_string()],
            call_count: 0,
            provider_name: "MockLLM",
            stream_script: None,
            model_query_error: None,
            requests: Vec::new(),
        }
    }
}

/// Configurable mock adapter for the [`LlmPort`] trait.
///
/// Suitable for unit tests and integration tests that need an in-process LLM
/// without making real API calls.
///
/// # Example
///
/// ```rust
/// use paladin_llm::mock::MockLlmAdapter;
///
/// let adapter = MockLlmAdapter::new()
///     .with_responses(vec![
///         "First response".to_string(),
///         "Second response".to_string(),
///     ]);
/// ```
#[derive(Debug, Clone)]
pub struct MockLlmAdapter {
    state: Arc<Mutex<MockState>>,
}

impl MockLlmAdapter {
    /// Create a new adapter with default configuration (single success response).
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
        }
    }

    /// Set the sequence of success responses to return (cycling when exhausted).
    pub fn with_responses(self, responses: Vec<String>) -> Self {
        let mut state = self.state.lock().unwrap();
        state.responses = responses.into_iter().map(MockEntry::Success).collect();
        state.response_index = 0;
        drop(state);
        self
    }

    /// Queue a single success response.
    pub fn with_response(self, response: impl Into<String>) -> Self {
        self.with_responses(vec![response.into()])
    }

    /// Set an error to be returned on the next call.
    pub fn with_error(self, error: LlmError) -> Self {
        let mut state = self.state.lock().unwrap();
        state.responses = vec![MockEntry::Error(error)];
        state.response_index = 0;
        drop(state);
        self
    }

    /// Simulate network latency by adding a delay before each response.
    pub fn with_delay(self, delay: Duration) -> Self {
        self.state.lock().unwrap().delay = Some(delay);
        self
    }

    /// Configure the token usage returned with each response using a `TokenUsage` struct.
    pub fn with_token_usage_struct(self, usage: TokenUsage) -> Self {
        self.state.lock().unwrap().token_usage = usage;
        self
    }

    /// Configure the token usage returned with each response (prompt, completion, total).
    pub fn with_token_usage(
        self,
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    ) -> Self {
        self.state.lock().unwrap().token_usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        };
        self
    }

    /// Configure the `FinishReason` returned with each successful response.
    pub fn with_finish_reason(self, reason: FinishReason) -> Self {
        self.state.lock().unwrap().finish_reason = reason;
        self
    }

    /// Configure the list of models reported as available.
    pub fn with_available_models(self, models: Vec<String>) -> Self {
        self.state.lock().unwrap().available_models = models;
        self
    }

    /// Queue an error response followed by a success response.
    ///
    /// Useful for testing retry / recovery logic.
    pub fn with_error_then_response(self, error: LlmError, response: impl Into<String>) -> Self {
        let mut state = self.state.lock().unwrap();
        state.responses = vec![MockEntry::Error(error), MockEntry::Success(response.into())];
        state.response_index = 0;
        drop(state);
        self
    }

    /// Set the name [`LlmPort::get_provider_name`] reports (default `"MockLLM"`).
    ///
    /// Lets a test compose several distinguishable mocks into one chain —
    /// the `FallbackLlmAdapter` (FT-05) identifies providers by this name.
    pub fn with_provider_name(self, name: &'static str) -> Self {
        self.state.lock().unwrap().provider_name = name;
        self
    }

    /// Script exactly what [`LlmPort::generate_stream`] yields, item by item.
    ///
    /// Each `Ok(text)` becomes one `StreamingResponse` chunk carrying `text`
    /// as its delta and no finish reason; each `Err(e)` is yielded as-is,
    /// in place. The scripted stream counts as one call in
    /// [`call_count`](Self::call_count) and does not consult the
    /// `generate` response queue, so a test can put an error FIRST (before
    /// any chunk) or AFTER a chunk to drive the fallback chain's
    /// first-chunk rule (D-25).
    pub fn with_stream_items(self, items: Vec<Result<String, LlmError>>) -> Self {
        self.state.lock().unwrap().stream_script = Some(items);
        self
    }

    /// Make [`LlmPort::validate_model`] and [`LlmPort::get_available_models`]
    /// answer `Err(error)` instead of consulting the configured model list.
    pub fn with_model_query_error(self, error: LlmError) -> Self {
        self.state.lock().unwrap().model_query_error = Some(error);
        self
    }

    /// Script a sequence of [`MockScriptEntry`] values, returned in order
    /// (cycling when exhausted, same as [`MockLlmAdapter::with_responses`]).
    ///
    /// Unlike `with_responses`, a script can include [`MockScriptEntry::ToolCall`]
    /// entries so a test can drive the [`FunctionCall`]-triggered branches of a
    /// consumer's reasoning loop (handoff detection, Arsenal invocation).
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
    ///
    /// let adapter = MockLlmAdapter::new().with_script(vec![
    ///     MockScriptEntry::ToolCall {
    ///         name: "web_search".to_string(),
    ///         arguments: r#"{"query":"rust"}"#.to_string(),
    ///     },
    ///     MockScriptEntry::Text("done".to_string()),
    /// ]);
    /// ```
    pub fn with_script(self, entries: Vec<MockScriptEntry>) -> Self {
        let mut state = self.state.lock().unwrap();
        state.responses = entries
            .into_iter()
            .map(|entry| match entry {
                MockScriptEntry::Text(text) => MockEntry::Success(text),
                MockScriptEntry::ToolCall { name, arguments } => {
                    MockEntry::ToolCall { name, arguments }
                }
                MockScriptEntry::Error(error) => MockEntry::Error(error),
            })
            .collect();
        state.response_index = 0;
        drop(state);
        self
    }

    /// Every request [`LlmPort::generate`] or [`LlmPort::generate_stream`]
    /// has received so far, in call order.
    pub fn requests(&self) -> Vec<LlmRequest> {
        self.state.lock().unwrap().requests.clone()
    }

    /// The most recent request received, if any.
    pub fn last_request(&self) -> Option<LlmRequest> {
        self.state.lock().unwrap().requests.last().cloned()
    }

    /// The rendered prompt text of the most recent request, extracted from
    /// its [`PromptType`] (`User.query` or `System.instructions`; any other
    /// variant yields an empty string).
    pub fn last_prompt(&self) -> Option<String> {
        self.last_request()
            .map(|request| match request.prompt.prompt_type() {
                PromptType::User(user) => user.query.clone(),
                PromptType::System(system) => system.instructions.clone(),
                _ => String::new(),
            })
    }

    /// Return the number of times [`LlmPort::generate`] or a scripted
    /// [`LlmPort::generate_stream`] has been called.
    pub fn call_count(&self) -> usize {
        self.state.lock().unwrap().call_count
    }

    /// Alias for [`call_count`](Self::call_count) for compatibility.
    pub fn get_call_count(&self) -> usize {
        self.call_count()
    }

    /// Reset the call counter, response index and recorded requests.
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.call_count = 0;
        state.response_index = 0;
        state.requests.clear();
    }

    /// Return `true` if the adapter was called at least once.
    pub fn was_called(&self) -> bool {
        self.call_count() > 0
    }
}

impl Default for MockLlmAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmPort for MockLlmAdapter {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let (response_entry, delay, token_usage, finish_reason) = {
            let mut state = self.state.lock().unwrap();
            state.call_count += 1;
            state.requests.push(request.clone());
            let index = state.response_index;
            let entry = state
                .responses
                .get(index)
                .cloned()
                .unwrap_or(MockEntry::Success("Mock LLM response".to_string()));
            // Advance index, cycling through responses
            state.response_index = (index + 1) % state.responses.len().max(1);
            (
                entry,
                state.delay,
                state.token_usage.clone(),
                state.finish_reason.clone(),
            )
        };

        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }

        match response_entry {
            MockEntry::Error(e) => Err(e),
            MockEntry::Success(content) => Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model.clone(),
                content,
                finish_reason,
                usage: token_usage,
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: None,
            }),
            MockEntry::ToolCall { name, arguments } => Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model.clone(),
                content: format!("Calling tool: {}", name),
                finish_reason: FinishReason::FunctionCall,
                usage: token_usage,
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: Some(FunctionCall { name, arguments }),
            }),
        }
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        let scripted = {
            let mut state = self.state.lock().unwrap();
            let script = state.stream_script.clone();
            if script.is_some() {
                state.call_count += 1;
                state.requests.push(request.clone());
            }
            script
        };
        if let Some(items) = scripted {
            let chunks: Vec<Result<StreamingResponse, LlmError>> = items
                .into_iter()
                .map(|item| {
                    item.map(|delta| StreamingResponse {
                        id: Uuid::new_v4(),
                        delta,
                        finish_reason: None,
                    })
                })
                .collect();
            return Ok(Box::new(stream::iter(chunks)));
        }

        let response = self.generate(request).await?;
        // Emit the full response as a single streaming chunk, then stop.
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

    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let state = self.state.lock().unwrap();
        if let Some(error) = &state.model_query_error {
            return Err(error.clone());
        }
        Ok(state.available_models.iter().any(|m| m == model))
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        let state = self.state.lock().unwrap();
        if let Some(error) = &state.model_query_error {
            return Err(error.clone());
        }
        Ok(state.available_models.clone())
    }

    fn get_provider_name(&self) -> &'static str {
        self.state.lock().unwrap().provider_name
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: false,
            max_context_tokens: Some(4096),
            supports_embeddings: false,
            supports_system_messages: true,
            temperature_range: None,
        }
    }
}

// ---------------------------------------------------------------------------
// MultiStepMockLlmPort
// ---------------------------------------------------------------------------

/// A mock adapter that returns different responses for successive calls.
///
/// Unlike [`MockLlmAdapter`] (which cycles), this adapter returns each response
/// exactly once and then panics if called more times than there are responses.
/// This is useful for tests that assert the *exact* sequence of LLM calls.
///
/// # Example
///
/// ```rust
/// use paladin_llm::mock::MultiStepMockLlmPort;
///
/// let adapter = MultiStepMockLlmPort::new(vec![
///     "Step 1 response".to_string(),
///     "Step 2 response".to_string(),
/// ]);
/// ```
#[derive(Debug)]
pub struct MultiStepMockLlmPort {
    responses: Vec<String>,
    call_count: Arc<Mutex<usize>>,
}

impl MultiStepMockLlmPort {
    /// Create a new multi-step mock with the given response sequence.
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Return the number of times [`LlmPort::generate`] has been called.
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait]
impl LlmPort for MultiStepMockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut count = self.call_count.lock().unwrap();
        let index = *count;
        *count += 1;
        drop(count);

        let content = self
            .responses
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Mock step {} response", index));

        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            model: request.model.clone(),
            content,
            finish_reason: FinishReason::Stop,
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        let response = self.generate(request).await?;
        let chunks = vec![Ok(StreamingResponse {
            id: Uuid::new_v4(),
            delta: response.content,
            finish_reason: Some(FinishReason::Stop),
        })];
        Ok(Box::new(stream::iter(chunks)))
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["mock-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "multi-step-mock"
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: false,
            max_context_tokens: Some(4096),
            supports_embeddings: false,
            supports_system_messages: true,
            temperature_range: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::llm_port::LlmPort;
    use uuid::Uuid;

    fn make_request() -> LlmRequest {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "test query".to_string(),
            context: None,
        }))
        .unwrap();
        LlmRequest {
            id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            prompt,
            attachments: vec![],
            stream: false,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_mock_returns_default_response() {
        let adapter = MockLlmAdapter::new();
        let request = make_request();
        let response = adapter.generate(request).await.unwrap();
        assert_eq!(response.content, "Mock LLM response");
    }

    #[tokio::test]
    async fn test_mock_cycles_responses() {
        let adapter =
            MockLlmAdapter::new().with_responses(vec!["First".to_string(), "Second".to_string()]);
        let r1 = adapter.generate(make_request()).await.unwrap();
        let r2 = adapter.generate(make_request()).await.unwrap();
        let r3 = adapter.generate(make_request()).await.unwrap(); // cycles back
        assert_eq!(r1.content, "First");
        assert_eq!(r2.content, "Second");
        assert_eq!(r3.content, "First");
    }

    #[tokio::test]
    async fn test_mock_tracks_call_count() {
        let adapter = MockLlmAdapter::new();
        assert_eq!(adapter.call_count(), 0);
        adapter.generate(make_request()).await.unwrap();
        assert_eq!(adapter.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_returns_error() {
        let adapter = MockLlmAdapter::new().with_error(LlmError::RateLimitExceeded);
        let result = adapter.generate(make_request()).await;
        assert!(matches!(result, Err(LlmError::RateLimitExceeded)));
    }

    #[tokio::test]
    async fn test_multi_step_returns_sequence() {
        let adapter = MultiStepMockLlmPort::new(vec![
            "Step 1".to_string(),
            "Step 2".to_string(),
            "Step 3".to_string(),
        ]);
        let r1 = adapter.generate(make_request()).await.unwrap();
        let r2 = adapter.generate(make_request()).await.unwrap();
        let r3 = adapter.generate(make_request()).await.unwrap();
        assert_eq!(r1.content, "Step 1");
        assert_eq!(r2.content, "Step 2");
        assert_eq!(r3.content, "Step 3");
    }

    #[tokio::test]
    async fn test_multi_step_tracks_call_count() {
        let adapter = MultiStepMockLlmPort::new(vec!["A".to_string()]);
        assert_eq!(adapter.call_count(), 0);
        adapter.generate(make_request()).await.unwrap();
        assert_eq!(adapter.call_count(), 1);
    }
}
