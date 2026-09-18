//! Example: Creating custom Herald formatters
//!
//! This example demonstrates how to implement custom Herald formatters
//! for XML and CSV output formats.

use async_trait::async_trait;
use chrono::Utc;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::herald::{
    BattalionResult, ExecutionMetadata, Herald, HeraldError, PaladinResult, StreamChunk,
};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, TokenUsage,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// XML Herald - outputs results in XML format
pub struct XmlHerald;

impl XmlHerald {
    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

impl Herald for XmlHerald {
    fn name(&self) -> &str {
        "xml"
    }

    fn mime_type(&self) -> &str {
        "application/xml"
    }

    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        Ok(format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<paladin_result>
    <output>{}</output>
    <token_count>{}</token_count>
    <execution_time_ms>{}</execution_time_ms>
    <loop_count>{}</loop_count>
    <stop_reason>{:?}</stop_reason>
</paladin_result>"#,
            Self::xml_escape(&result.output),
            result.usage.total_tokens,
            result.execution_time_ms,
            result.loop_count,
            result.stop_reason,
        ))
    }

    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<battalion_result>
    <battalion_id>{}</battalion_id>
    <battalion_name>{}</battalion_name>
    <status>{:?}</status>
    <paladins>"#,
            result.battalion_id,
            Self::xml_escape(&result.battalion_name),
            result.status,
        );

        for paladin in &result.paladin_results {
            xml.push_str(&format!(
                r#"
        <paladin>
            <output>{}</output>
            <token_count>{}</token_count>
        </paladin>"#,
                Self::xml_escape(&paladin.output),
                paladin.usage.total_tokens,
            ));
        }

        xml.push_str("\n    </paladins>\n</battalion_result>");
        Ok(xml)
    }

    fn format_error(&self, error: &PaladinError) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<error>{}</error>"#,
            Self::xml_escape(&error.to_string())
        )
    }

    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        Ok(Some(format!(
            r#"<chunk is_final="{}">{}</chunk>"#,
            chunk.is_final,
            Self::xml_escape(&chunk.content)
        )))
    }

    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
        Ok(format!(
            r#"<metadata model="{}" duration_ms="{}" total_tokens="{}"/>"#,
            metadata.model_used,
            metadata.duration_ms.unwrap_or(0),
            metadata.token_usage.total_tokens
        ))
    }
}

/// CSV Herald - outputs results in CSV format
pub struct CsvHerald;

impl CsvHerald {
    fn csv_escape(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
}

impl Herald for CsvHerald {
    fn name(&self) -> &str {
        "csv"
    }

    fn mime_type(&self) -> &str {
        "text/csv"
    }

    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        let mut csv = String::from("output,token_count,execution_time_ms,loop_count,stop_reason\n");
        csv.push_str(&format!(
            "{},{},{},{},{:?}\n",
            Self::csv_escape(&result.output),
            result.usage.total_tokens,
            result.execution_time_ms,
            result.loop_count,
            result.stop_reason,
        ));
        Ok(csv)
    }

    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
        let mut csv = String::from("battalion_id,battalion_name,paladin_output,token_count\n");

        for paladin in &result.paladin_results {
            csv.push_str(&format!(
                "{},{},{},{}\n",
                result.battalion_id,
                Self::csv_escape(&result.battalion_name),
                Self::csv_escape(&paladin.output),
                paladin.usage.total_tokens,
            ));
        }

        Ok(csv)
    }

    fn format_error(&self, error: &PaladinError) -> String {
        format!(
            "error_type,error_message\nerror,{}\n",
            Self::csv_escape(&error.to_string())
        )
    }

    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        // CSV doesn't stream well, buffer until final
        if chunk.is_final {
            Ok(Some(format!(
                "content,is_final\n{},true\n",
                Self::csv_escape(&chunk.content)
            )))
        } else {
            Ok(None)
        }
    }

    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
        Ok(format!(
            "model,duration_ms,total_tokens\n{},{},{}\n",
            metadata.model_used,
            metadata.duration_ms.unwrap_or(0),
            metadata.token_usage.total_tokens
        ))
    }
}

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
            usage: TokenUsage::new(20, 60),
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
    println!("=== Custom Herald Formatters Example ===\n");

    // Create mock LLM
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort {
        response:
            "Paris is the capital and largest city of France. It is situated on the Seine River."
                .to_string(),
    });

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

    // Example 1: XML Herald
    println!("--- Example 1: XML Herald ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(XmlHerald);

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

    // Example 2: CSV Herald
    println!("--- Example 2: CSV Herald ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(CsvHerald);

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

    // Example 3: XML with special characters (demonstrates escaping)
    println!("--- Example 3: XML with Special Characters ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(XmlHerald);

        let special_llm: Arc<dyn LlmPort> = Arc::new(MockLlmPort {
            response: r#"The answer contains "quotes" & <special> characters!"#.to_string(),
        });

        let paladin = PaladinBuilder::new(Arc::clone(&special_llm))
            .system_prompt("Test system")
            .name("TestPaladin")
            .build()
            .await?;

        let service = PaladinExecutionService::new(
            Arc::clone(&special_llm),
            Arc::clone(&circuit_breaker),
            None,
            None,
        )
        .with_herald(Arc::clone(&herald));

        let result = service.execute(&paladin, "Test input").await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
            println!("Note: Special characters are properly escaped!\n");
        }
    }

    // Example 4: CSV with commas and quotes (demonstrates escaping)
    println!("--- Example 4: CSV with Special Characters ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(CsvHerald);

        let special_llm: Arc<dyn LlmPort> = Arc::new(MockLlmPort {
            response: r#"Paris, the "City of Light", is amazing!"#.to_string(),
        });

        let paladin = PaladinBuilder::new(Arc::clone(&special_llm))
            .system_prompt("Test system")
            .name("TestPaladin")
            .build()
            .await?;

        let service = PaladinExecutionService::new(
            Arc::clone(&special_llm),
            Arc::clone(&circuit_breaker),
            None,
            None,
        )
        .with_herald(Arc::clone(&herald));

        let result = service.execute(&paladin, "Test input").await?;

        if let Some(formatted) = service.format_result(&result, &paladin)? {
            println!("{}\n", formatted);
            println!("Note: Commas and quotes are properly escaped!\n");
        }
    }

    // Example 5: Streaming with XML
    println!("--- Example 5: XML Streaming ---\n");
    {
        let herald: Arc<dyn Herald> = Arc::new(XmlHerald);

        println!("Simulating streaming chunks:\n");

        use chrono::Utc;
        use uuid::Uuid;
        let chunks = vec![
            StreamChunk::builder()
                .chunk_id(Uuid::new_v4())
                .sequence_number(0)
                .timestamp(Utc::now())
                .content("First part ".to_string())
                .token_count(3)
                .is_final(false)
                .build()
                .unwrap(),
            StreamChunk::builder()
                .chunk_id(Uuid::new_v4())
                .sequence_number(1)
                .timestamp(Utc::now())
                .content("Second part ".to_string())
                .token_count(3)
                .is_final(false)
                .build()
                .unwrap(),
            StreamChunk::builder()
                .chunk_id(Uuid::new_v4())
                .sequence_number(2)
                .timestamp(Utc::now())
                .content("Final part".to_string())
                .token_count(3)
                .is_final(true)
                .build()
                .unwrap(),
        ];

        for chunk in &chunks {
            if let Some(formatted) = herald.format_stream_chunk(chunk)? {
                print!("{}", formatted);
            }
        }

        use paladin_ports::output::llm_port::TokenUsage;
        let metadata = ExecutionMetadata::builder()
            .execution_id(Uuid::new_v4())
            .start_time(Utc::now())
            .end_time(Utc::now())
            .duration_ms(1500)
            .model_used("gpt-4".to_string())
            .token_usage(TokenUsage::new(42, 43))
            .build()
            .unwrap();
        println!("{}\n", herald.finalize_stream(&metadata)?);
    }

    println!("=== End of Custom Herald Examples ===");
    println!("\nKey Takeaways:");
    println!("- XML Herald properly escapes special characters (&, <, >, \", ')");
    println!("- CSV Herald handles commas and quotes correctly");
    println!("- Both formatters implement the full Herald trait");
    println!("- Custom formatters can define their own streaming behavior");

    Ok(())
}
