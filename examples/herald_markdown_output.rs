//! Example: Using Markdown Herald for human-readable output
//!
//! This example demonstrates how to use the Markdown Herald formatter to produce
//! beautiful, readable output with colors and formatting.

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::herald::Herald;
use paladin::infrastructure::adapters::herald::MarkdownHerald;
use paladin::infrastructure::adapters::herald::markdown_herald::MarkdownHeraldConfig;
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
            usage: TokenUsage::new(30, 80),
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
    println!("=== Markdown Herald Example ===\n");

    // Create mock LLM
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort {
        response: "# Key Points\n\n\
                   - Paris is the capital of France\n\
                   - Population: ~2.2 million in city proper\n\
                   - Famous for: Eiffel Tower, Louvre, Notre-Dame\n\
                   - Known as \"City of Light\"\n\n\
                   Paris has been a major European center of art, culture, and politics for centuries."
            .to_string(),
    });

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

    // Example 1: Default Markdown Herald (with colors, heading level 2)
    println!("--- Example 1: Default Markdown Herald (colored) ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());

        let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt(
                "You are a helpful geography assistant. Format your responses with markdown.",
            )
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

        let result = service.execute(&paladin, "Tell me about Paris").await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
        }
    }

    // Example 2: Without colors (for log files)
    println!("--- Example 2: Markdown without Colors ---\n");
    {
        let config = MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        };
        let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::with_config(config));

        let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt(
                "You are a helpful geography assistant. Format your responses with markdown.",
            )
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

        let result = service.execute(&paladin, "Tell me about London").await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
        }
    }

    // Example 3: Higher heading level (h3)
    println!("--- Example 3: Markdown with Heading Level 3 ---\n");
    {
        let config = MarkdownHeraldConfig {
            include_colors: true,
            heading_level: 3,
        };
        let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::with_config(config));

        let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt(
                "You are a helpful geography assistant. Format your responses with markdown.",
            )
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

        let result = service.execute(&paladin, "Tell me about Berlin").await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
        }
    }

    // Example 4: Multiple Paladins with different topics
    println!("--- Example 4: Multiple Paladins ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());

        let topics = vec![
            ("Rome", "Tell me about Rome"),
            ("Madrid", "Tell me about Madrid"),
            ("Amsterdam", "Tell me about Amsterdam"),
        ];

        for (city, prompt) in topics {
            let paladin = PaladinBuilder::new(Arc::clone(&llm_port))
                .system_prompt(
                    "You are a helpful geography assistant. Format your responses with markdown.",
                )
                .name(format!("{}Guide", city))
                .build()
                .await?;

            let service = PaladinExecutionService::new(
                Arc::clone(&llm_port),
                Arc::clone(&circuit_breaker),
                None,
                None,
            )
            .with_herald(Arc::clone(&herald));

            let result = service.execute(&paladin, prompt).await?;

            if let Some(formatted) = service.format_result(&result, &paladin)? {
                println!("{}", formatted);
                println!("---\n");
            }
        }
    }

    println!("=== End of Markdown Herald Examples ===");

    Ok(())
}
