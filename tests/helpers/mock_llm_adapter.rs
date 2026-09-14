//! Mock LLM adapter for testing Battalion and Commander
//!
//! Provides a configurable mock implementation of LlmPort that returns
//! pre-configured responses in sequence, tracks invocation counts, and
//! supports both success and error scenarios.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream;
use paladin_ports::output::llm_port::{
    FinishReason, FunctionCall, LlmError, LlmPort, LlmRequest, LlmResponse, StreamingResponse,
    TokenUsage,
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Invocation record for testing
///
/// Records details about each LLM call for test assertions
#[derive(Debug, Clone)]
pub struct Invocation {
    /// The prompt text sent to the LLM
    pub prompt: String,
    /// The model requested
    pub model: String,
    /// When the invocation occurred
    pub timestamp: DateTime<Utc>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Mock response types for flexible testing
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Simple text response
    Text(String),
    /// Tool/function call request
    ToolCall {
        tool_name: String,
        arguments: String,
    },
    /// Streaming response (list of chunks)
    Streaming(Vec<String>),
    /// Error response
    Error(LlmError),
}

/// Mock LLM adapter with configurable responses for testing
///
/// Responses are queued and returned in FIFO order. When the queue is empty,
/// returns a default success response.
///
/// # Examples
///
/// ```rust
/// use std::sync::Arc;
/// let mock = Arc::new(MockLlmAdapter::new());
/// mock.add_success("First response");
/// mock.add_success("Second response");
///
/// // First call returns "First response", second returns "Second response"
/// // Third call returns default response
/// ```
#[derive(Clone)]
pub struct MockLlmAdapter {
    responses: Arc<Mutex<VecDeque<MockResponse>>>,
    invocations: Arc<Mutex<Vec<Invocation>>>,
}

impl MockLlmAdapter {
    /// Create a new MockLlmAdapter with empty response queue
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            invocations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a response to the queue
    pub fn add_response(&self, response: MockResponse) {
        self.responses.lock().unwrap().push_back(response);
    }

    /// Add a success response to the queue
    pub fn add_success(&self, content: impl Into<String>) {
        self.add_response(MockResponse::Text(content.into()));
    }

    /// Add a failure response to the queue
    pub fn add_failure(&self, error: LlmError) {
        self.add_response(MockResponse::Error(error));
    }

    /// Add a tool call response to the queue
    pub fn add_tool_call(&self, tool_name: impl Into<String>, arguments: impl Into<String>) {
        self.add_response(MockResponse::ToolCall {
            tool_name: tool_name.into(),
            arguments: arguments.into(),
        });
    }

    /// Add a streaming response to the queue
    pub fn add_streaming(&self, chunks: Vec<impl Into<String>>) {
        let string_chunks: Vec<String> = chunks.into_iter().map(|c| c.into()).collect();
        self.add_response(MockResponse::Streaming(string_chunks));
    }

    /// Get the number of times generate() was called
    pub fn call_count(&self) -> usize {
        self.invocations.lock().unwrap().len()
    }

    /// Get all invocation records
    pub fn invocations(&self) -> Vec<Invocation> {
        self.invocations.lock().unwrap().clone()
    }

    /// Get the last prompt sent to the LLM, if any
    pub fn last_prompt(&self) -> Option<String> {
        self.invocations
            .lock()
            .unwrap()
            .last()
            .map(|inv| inv.prompt.clone())
    }

    /// Get the last invocation record, if any
    pub fn last_invocation(&self) -> Option<Invocation> {
        self.invocations.lock().unwrap().last().cloned()
    }

    /// Reset the mock: clear responses and invocations
    pub fn reset(&self) {
        self.responses.lock().unwrap().clear();
        self.invocations.lock().unwrap().clear();
    }

    /// Record an invocation for testing
    fn record_invocation(&self, request: &LlmRequest) {
        let prompt = match &request.prompt.prompt_type() {
            paladin::core::platform::container::prompt::PromptType::User(user) => {
                user.query.clone()
            }
            paladin::core::platform::container::prompt::PromptType::System(system) => {
                system.instructions.clone()
            }
            _ => "unknown".to_string(),
        };

        self.invocations.lock().unwrap().push(Invocation {
            prompt,
            model: request.model.clone(),
            timestamp: Utc::now(),
            metadata: request.metadata.clone(),
        });
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
        // Record invocation
        self.record_invocation(&request);

        // Pop next response from queue, or return default if empty
        let mock_response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(MockResponse::Text("Mock LLM response".to_string()));

        // Convert MockResponse to LlmResponse
        match mock_response {
            MockResponse::Text(content) => Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model,
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
            }),
            MockResponse::ToolCall {
                tool_name,
                arguments,
            } => Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model,
                content: format!("Calling tool: {}", tool_name),
                finish_reason: FinishReason::FunctionCall,
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 15,
                    total_tokens: 25,
                },
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: Some(FunctionCall {
                    name: tool_name,
                    arguments,
                }),
            }),
            MockResponse::Streaming(chunks) => {
                // For non-streaming generate(), concatenate all chunks
                let content = chunks.join("");
                Ok(LlmResponse {
                    id: Uuid::new_v4(),
                    request_id: request.id,
                    model: request.model,
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
            MockResponse::Error(error) => Err(error),
        }
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    {
        // Record invocation
        self.record_invocation(&request);

        // Get response
        let mock_response =
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(MockResponse::Text(
                    "Mock LLM streaming response".to_string(),
                ));

        match mock_response {
            MockResponse::Text(content) => {
                // Create single chunk stream
                let response = StreamingResponse {
                    id: request.id,
                    delta: content,
                    finish_reason: Some(FinishReason::Stop),
                };
                Ok(Box::new(stream::once(async move { Ok(response) })))
            }
            MockResponse::Streaming(chunks) => {
                // Create multi-chunk stream
                let request_id = request.id;
                let num_chunks = chunks.len();
                let responses: Vec<Result<StreamingResponse, LlmError>> = chunks
                    .into_iter()
                    .enumerate()
                    .map(|(i, chunk)| {
                        Ok(StreamingResponse {
                            id: request_id,
                            delta: chunk,
                            finish_reason: if i == num_chunks - 1 {
                                Some(FinishReason::Stop)
                            } else {
                                None
                            },
                        })
                    })
                    .collect();

                Ok(Box::new(stream::iter(responses)))
            }
            MockResponse::ToolCall { .. } => {
                // For streaming, tool calls return as text
                let response = StreamingResponse {
                    id: request.id,
                    delta: "Tool call not supported in streaming".to_string(),
                    finish_reason: Some(FinishReason::Stop),
                };
                Ok(Box::new(stream::once(async move { Ok(response) })))
            }
            MockResponse::Error(error) => Err(error),
        }
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["mock-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "mock"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_function_calling: true,
            supports_vision: false,
            supports_embeddings: false,
            max_context_tokens: Some(128000),
            supports_system_messages: true,
            temperature_range: None,
        }
    }
}

/// Helper function to create a test Paladin with a mock LLM adapter
pub fn create_test_paladin_with_mock(
    name: impl Into<String>,
    _mock: Arc<MockLlmAdapter>,
) -> paladin::core::platform::container::paladin::Paladin {
    use paladin::core::base::entity::node::Node;
    use paladin::core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};

    let data = PaladinData {
        system_prompt: "Test prompt".to_string(),
        name: name.into(),
        user_name: "test".to_string(),
        model: "mock-model".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, None)
}

/// Helper function to create a mock pre-configured with success responses
pub fn create_mock_with_responses(responses: Vec<&str>) -> Arc<MockLlmAdapter> {
    let mock = Arc::new(MockLlmAdapter::new());
    for response in responses {
        mock.add_success(response);
    }
    mock
}

/// Helper function to create a mock with tool call responses
pub fn create_mock_with_tool_calls(tool_calls: Vec<(&str, &str)>) -> Arc<MockLlmAdapter> {
    let mock = Arc::new(MockLlmAdapter::new());
    for (tool_name, arguments) in tool_calls {
        mock.add_tool_call(tool_name, arguments);
    }
    mock
}

/// Helper function to create a mock with mixed responses
pub fn create_mock_with_mixed_responses(
    text_responses: Vec<&str>,
    tool_calls: Vec<(&str, &str)>,
) -> Arc<MockLlmAdapter> {
    let mock = Arc::new(MockLlmAdapter::new());
    for text in text_responses {
        mock.add_success(text);
    }
    for (tool_name, arguments) in tool_calls {
        mock.add_tool_call(tool_name, arguments);
    }
    mock
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin::core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

    fn create_test_prompt() -> PromptItem {
        PromptItem::new(PromptType::User(UserPrompt {
            query: "test prompt".to_string(),
            context: None,
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn test_mock_llm_adapter_returns_configured_responses() {
        let mock = MockLlmAdapter::new();
        mock.add_success("First response");
        mock.add_success("Second response");
        mock.add_success("Third response");

        let request = LlmRequest::new("mock", create_test_prompt());

        let response1 = mock.generate(request.clone()).await.unwrap();
        assert_eq!(response1.content, "First response");

        let response2 = mock.generate(request.clone()).await.unwrap();
        assert_eq!(response2.content, "Second response");

        let response3 = mock.generate(request.clone()).await.unwrap();
        assert_eq!(response3.content, "Third response");

        // Fourth call returns default
        let response4 = mock.generate(request).await.unwrap();
        assert_eq!(response4.content, "Mock LLM response");
    }

    #[tokio::test]
    async fn test_mock_llm_adapter_tracks_call_count() {
        let mock = MockLlmAdapter::new();
        assert_eq!(mock.call_count(), 0);

        let request = LlmRequest::new("mock", create_test_prompt());

        mock.generate(request.clone()).await.unwrap();
        assert_eq!(mock.call_count(), 1);

        mock.generate(request.clone()).await.unwrap();
        assert_eq!(mock.call_count(), 2);

        mock.generate(request).await.unwrap();
        assert_eq!(mock.call_count(), 3);
    }

    #[tokio::test]
    async fn test_mock_llm_adapter_handles_failures() {
        let mock = MockLlmAdapter::new();
        mock.add_success("Success response");
        mock.add_failure(LlmError::ProcessingError("Simulated LLM error".to_string()));
        mock.add_success("Another success");

        let request = LlmRequest::new("mock", create_test_prompt());

        let response1 = mock.generate(request.clone()).await;
        assert!(response1.is_ok());
        assert_eq!(response1.unwrap().content, "Success response");

        let response2 = mock.generate(request.clone()).await;
        assert!(response2.is_err());
        if let Err(LlmError::ProcessingError(msg)) = response2 {
            assert_eq!(msg, "Simulated LLM error");
        } else {
            panic!("Expected ProcessingError");
        }

        let response3 = mock.generate(request).await;
        assert!(response3.is_ok());
        assert_eq!(response3.unwrap().content, "Another success");
    }

    #[tokio::test]
    async fn test_mock_llm_adapter_reset() {
        let mock = MockLlmAdapter::new();
        mock.add_success("Response 1");
        mock.add_success("Response 2");

        let request = LlmRequest::new("mock", create_test_prompt());

        mock.generate(request.clone()).await.unwrap();
        mock.generate(request.clone()).await.unwrap();
        assert_eq!(mock.call_count(), 2);

        mock.reset();
        assert_eq!(mock.call_count(), 0);

        // After reset, should return default response
        let response = mock.generate(request).await.unwrap();
        assert_eq!(response.content, "Mock LLM response");
    }

    #[tokio::test]
    async fn test_mock_llm_adapter_tool_call() {
        let mock = MockLlmAdapter::new();
        mock.add_tool_call("web_search", r#"{"query": "Rust programming"}"#);

        let request = LlmRequest::new("mock", create_test_prompt());

        let response = mock.generate(request).await.unwrap();
        assert!(response.function_call.is_some());

        let function_call = response.function_call.unwrap();
        assert_eq!(function_call.name, "web_search");
        assert_eq!(function_call.arguments, r#"{"query": "Rust programming"}"#);
        assert!(matches!(response.finish_reason, FinishReason::FunctionCall));
    }

    #[tokio::test]
    async fn test_mock_llm_adapter_invocation_tracking() {
        let mock = MockLlmAdapter::new();
        mock.add_success("Response 1");
        mock.add_success("Response 2");

        let request1 = LlmRequest::new("gpt-4", create_test_prompt());

        let request2 = LlmRequest::new("gpt-3.5-turbo", create_test_prompt());

        mock.generate(request1).await.unwrap();
        mock.generate(request2).await.unwrap();

        let invocations = mock.invocations();
        assert_eq!(invocations.len(), 2);
        assert_eq!(invocations[0].model, "gpt-4");
        assert_eq!(invocations[1].model, "gpt-3.5-turbo");

        let last_prompt = mock.last_prompt();
        assert!(last_prompt.is_some());
        assert_eq!(last_prompt.unwrap(), "test prompt");
    }

    #[tokio::test]
    async fn test_mock_llm_adapter_streaming_chunks() {
        let mock = MockLlmAdapter::new();
        mock.add_streaming(vec!["Hello", " ", "World", "!"]);

        let request = LlmRequest::new("mock", create_test_prompt()).with_stream(true);

        // For non-streaming generate, it concatenates chunks
        let response = mock.generate(request).await.unwrap();
        assert_eq!(response.content, "Hello World!");
    }

    #[test]
    fn test_create_mock_with_responses() {
        let mock = create_mock_with_responses(vec!["First", "Second", "Third"]);
        // Verify responses are queued
        assert_eq!(mock.responses.lock().unwrap().len(), 3);
    }

    #[test]
    fn test_create_mock_with_tool_calls() {
        let mock = create_mock_with_tool_calls(vec![
            ("search", r#"{"q":"test"}"#),
            ("calculate", r#"{"expr":"2+2"}"#),
        ]);
        assert_eq!(mock.responses.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_create_test_paladin_with_mock() {
        let mock = Arc::new(MockLlmAdapter::new());
        let paladin = create_test_paladin_with_mock("test_paladin", mock);
        // Access inner data via .node field
        assert_eq!(paladin.node.name, "test_paladin");
        assert_eq!(paladin.node.model, "mock-model");
    }
}
