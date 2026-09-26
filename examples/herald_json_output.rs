//! Example: Using JSON Herald for structured output
//!
//! This example demonstrates how to use the JSON Herald formatter to produce
//! structured JSON output from Paladin execution results.

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::herald::Herald;
use paladin::infrastructure::adapters::herald::JsonHerald;
use paladin::infrastructure::adapters::herald::json_herald::JsonHeraldConfig;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, TokenUsage,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Simple mock LLM for demonstration
struct MockLlmPort {
    response: String,
}

#[async_trait]
impl LlmPort for MockLlmPort {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            model: request.model,
            content: self.response.clone(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::new(25, 75),
            cost: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<
        Box<
            dyn futures::Stream<
                    Item = Result<paladin_ports::output::llm_port::StreamingResponse, LlmError>,
                > + Send,
        >,
        LlmError,
    > {
        unimplemented!("Streaming not needed for this example")
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        Ok(true)
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(vec!["mock-gpt-4".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "mock"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JSON Herald Example ===\n");

    // Create mock LLM
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort {
        response: "The capital of France is Paris. It is known for the Eiffel Tower, \
                   the Louvre Museum, and its rich cultural heritage."
            .to_string(),
    });

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

    // Example 1: Default JSON Herald (pretty-printed)
    println!("--- Example 1: Default JSON Herald ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(JsonHerald::new());

        let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("You are a helpful geography assistant")
            .name("GeographyExpert")
            .build()
            .await?;

        let service = PaladinExecutionService::new(
            Arc::clone(&llm_port),
            Arc::clone(&circuit_breaker),
            None,
            None,
        )
        .with_herald(Arc::clone(&herald));

        let result = service
            .execute(&paladin, "What is the capital of France?")
            .await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
        }
    }

    // Example 2: Compact JSON (no pretty-print)
    println!("--- Example 2: Compact JSON ---\n");
    {
        let config = JsonHeraldConfig {
            pretty: false,
            include_metadata: false,
        };
        let herald: Arc<dyn Herald> = Arc::new(JsonHerald::with_config(config));

        let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("You are a helpful geography assistant")
            .name("GeographyExpert")
            .build()
            .await?;

        let service = PaladinExecutionService::new(
            Arc::clone(&llm_port),
            Arc::clone(&circuit_breaker),
            None,
            None,
        )
        .with_herald(Arc::clone(&herald));

        let result = service
            .execute(&paladin, "What is the capital of Spain?")
            .await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
        }
    }

    // Example 3: With metadata
    println!("--- Example 3: JSON with Metadata ---\n");
    {
        let config = JsonHeraldConfig {
            pretty: true,
            include_metadata: true,
        };
        let herald: Arc<dyn Herald> = Arc::new(JsonHerald::with_config(config));

        let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("You are a helpful geography assistant")
            .name("GeographyExpert")
            .build()
            .await?;

        let service = PaladinExecutionService::new(
            Arc::clone(&llm_port),
            Arc::clone(&circuit_breaker),
            None,
            None,
        )
        .with_herald(Arc::clone(&herald));

        let result = service
            .execute(&paladin, "What is the capital of Italy?")
            .await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
        }
    }

    // Example 4: Parse and use JSON output
    println!("--- Example 4: Parsing JSON Output ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(JsonHerald::new());

        let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("You are a helpful geography assistant")
            .name("GeographyExpert")
            .build()
            .await?;

        let service = PaladinExecutionService::new(
            Arc::clone(&llm_port),
            Arc::clone(&circuit_breaker),
            None,
            None,
        )
        .with_herald(Arc::clone(&herald));

        let result = service
            .execute(&paladin, "What is the capital of Germany?")
            .await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            // Parse the JSON
            let parsed: serde_json::Value = serde_json::from_str(&formatted)?;

            // Extract specific fields
            println!("Extracted fields:");
            println!("  Paladin: {}", parsed["paladin_name"]);
            println!("  Status: {}", parsed["status"]);
            println!(
                "  Output length: {} chars",
                parsed["output"].as_str().unwrap().len()
            );
            println!();
        }
    }

    println!("=== End of JSON Herald Examples ===");

    Ok(())
}
