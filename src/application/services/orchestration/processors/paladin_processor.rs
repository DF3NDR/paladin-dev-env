//! [`PaladinContentProcessor`] — bridges a single Paladin agent into the
//! content-processing pipeline.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{OutputParsing, PromptTemplate};
use crate::application::services::orchestration::types::{
    ContentProcessingResult, ContentProcessor, OrchestratorError,
};
use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use crate::core::platform::container::content::ContentItem;
use crate::core::platform::container::orchestration_context::OrchestrationContext;
use crate::core::platform::container::paladin::Paladin;

/// A [`ContentProcessor`] that enriches content by invoking a single Paladin
/// agent.
///
/// The processor converts an incoming [`ContentItem`] into a prompt via a
/// configurable [`PromptTemplate`], executes the prompt through a
/// [`PaladinExecutionService`], and parses the agent's response into a
/// [`ContentProcessingResult`] according to the configured [`OutputParsing`]
/// strategy.
///
/// It depends only on the `PaladinExecutionService` / `LlmPort` abstraction, so
/// it can be unit-tested with a mock LLM adapter and never reaches a concrete
/// provider directly.
///
/// # Degraded results
///
/// Under [`OutputParsing::Json`], a response that is not valid JSON does **not**
/// fail the pipeline. Instead the result is marked unsuccessful, the parse error
/// is recorded in `error`, and the raw agent text is preserved under
/// `result_data` so no data is lost.
#[derive(Clone)]
pub struct PaladinContentProcessor {
    name: String,
    execution_service: Arc<PaladinExecutionService>,
    paladin: Arc<Paladin>,
    prompt_template: PromptTemplate,
    parsing: OutputParsing,
}

impl PaladinContentProcessor {
    /// Creates a new processor with the default name, prompt template, and
    /// (raw-text) parsing strategy.
    ///
    /// # Arguments
    ///
    /// * `execution_service` - The Paladin execution service used to run the agent.
    /// * `paladin` - The Paladin configuration to execute.
    pub fn new(execution_service: Arc<PaladinExecutionService>, paladin: Arc<Paladin>) -> Self {
        Self {
            name: "PaladinContentProcessor".to_string(),
            execution_service,
            paladin,
            prompt_template: PromptTemplate::default(),
            parsing: OutputParsing::default(),
        }
    }

    /// Overrides the processor name (used for registration/lookup).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the prompt template used to convert content into a prompt.
    pub fn with_prompt_template(mut self, template: PromptTemplate) -> Self {
        self.prompt_template = template;
        self
    }

    /// Sets the output-parsing strategy applied to the agent's response.
    pub fn with_output_parsing(mut self, parsing: OutputParsing) -> Self {
        self.parsing = parsing;
        self
    }

    /// Builds the enrichment metadata attached to every result.
    fn base_metadata(&self, output: &str, token_count: u32) -> HashMap<String, Value> {
        let mut metadata = HashMap::new();
        metadata.insert("agent_name".to_string(), json!(self.paladin.node.name));
        metadata.insert("model".to_string(), json!(self.paladin.node.model));
        metadata.insert("parsing_strategy".to_string(), json!(self.parsing.as_str()));
        metadata.insert("token_count".to_string(), json!(token_count));
        metadata.insert("output_chars".to_string(), json!(output.len()));
        metadata
    }
}

#[async_trait]
impl ContentProcessor for PaladinContentProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process_content(
        &self,
        content: ContentItem,
        _context: OrchestrationContext,
    ) -> Result<ContentProcessingResult, OrchestratorError> {
        let content_id = content.uuid();
        let start = Instant::now();

        let prompt = self.prompt_template.render(&content);

        let result = self
            .execution_service
            .execute(&self.paladin, &prompt)
            .await
            .map_err(|e| OrchestratorError::ServiceError(e.to_string()))?;

        let processing_time_ms = start.elapsed().as_millis() as u64;
        let output = result.output;
        let mut metadata = self.base_metadata(&output, result.usage.total_tokens);

        let (success, result_data, error) = match self.parsing {
            OutputParsing::RawText => (true, Some(json!({ "enrichment": output })), None),
            OutputParsing::Json => match serde_json::from_str::<Value>(&output) {
                Ok(value) => (true, Some(value), None),
                Err(parse_err) => {
                    // Degraded result: preserve the raw text, flag the failure.
                    metadata.insert("parse_failed".to_string(), json!(true));
                    (
                        false,
                        Some(json!({ "raw": output })),
                        Some(format!("JSON parse failed: {parse_err}")),
                    )
                }
            },
        };

        Ok(ContentProcessingResult {
            content_id,
            processor_name: self.name.clone(),
            processing_time_ms,
            success,
            result_data,
            error,
            metadata,
        })
    }

    fn clone_box(&self) -> Result<Box<dyn ContentProcessor>, OrchestratorError> {
        Ok(Box::new(self.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockLlmAdapter;
    use crate::application::services::paladin::paladin_builder::PaladinBuilder;
    use crate::core::platform::container::content::{ContentItem, ContentType, TextContent};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_ports::output::llm_port::LlmPort;
    use std::time::Duration;

    async fn build_processor(response: &str, parsing: OutputParsing) -> PaladinContentProcessor {
        let llm_port: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_response(response.to_string()));
        let paladin = PaladinBuilder::new(llm_port.clone())
            .system_prompt("You are a content analyst")
            .name("Analyst")
            .model("gpt-4")
            .build()
            .await
            .expect("failed to build paladin");

        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = Arc::new(PaladinExecutionService::new(
            llm_port,
            circuit_breaker,
            None,
            None,
        ));

        PaladinContentProcessor::new(service, Arc::new(paladin)).with_output_parsing(parsing)
    }

    fn text_item(body: &str) -> ContentItem {
        let content = TextContent::new(None, Some(body.to_string())).expect("text content");
        ContentItem::new_with_title(ContentType::Text(content), "Test Article".to_string())
            .expect("content item")
    }

    fn context() -> OrchestrationContext {
        OrchestrationContext::new("test".to_string(), "test".to_string())
    }

    #[tokio::test]
    async fn raw_text_strategy_stores_response_verbatim() {
        let processor = build_processor("The article is about Rust.", OutputParsing::RawText).await;
        let item = text_item("Rust is a systems programming language.");
        let id = item.uuid();

        let result = processor
            .process_content(item, context())
            .await
            .expect("processing succeeds");

        assert!(result.success);
        assert_eq!(result.content_id, id);
        assert_eq!(result.processor_name, "PaladinContentProcessor");
        assert_eq!(
            result.result_data.unwrap()["enrichment"],
            json!("The article is about Rust.")
        );
        // Enrichment metadata is attached.
        assert_eq!(result.metadata["agent_name"], json!("Analyst"));
        assert_eq!(result.metadata["parsing_strategy"], json!("raw_text"));
    }

    #[tokio::test]
    async fn json_strategy_parses_well_formed_response() {
        let processor = build_processor(
            r#"{"sentiment": "positive", "entities": ["Rust"]}"#,
            OutputParsing::Json,
        )
        .await;
        let item = text_item("Rust is loved by developers.");

        let result = processor
            .process_content(item, context())
            .await
            .expect("processing succeeds");

        assert!(result.success);
        let data = result.result_data.unwrap();
        assert_eq!(data["sentiment"], json!("positive"));
        assert_eq!(data["entities"], json!(["Rust"]));
    }

    #[tokio::test]
    async fn json_strategy_malformed_response_yields_degraded_result() {
        let processor = build_processor("not valid json at all", OutputParsing::Json).await;
        let item = text_item("Some content.");

        let result = processor
            .process_content(item, context())
            .await
            .expect("processing returns a degraded result, not an error");

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("JSON parse failed"));
        // Raw text is preserved so no data is lost.
        assert_eq!(
            result.result_data.unwrap()["raw"],
            json!("not valid json at all")
        );
        assert_eq!(result.metadata["parse_failed"], json!(true));
    }

    #[tokio::test]
    async fn clone_box_produces_equivalent_processor() {
        let processor = build_processor("hello", OutputParsing::RawText).await;
        let cloned = processor.clone_box().expect("clone_box succeeds");
        assert_eq!(cloned.name(), "PaladinContentProcessor");
    }
}
