//! [`BattalionContentProcessor`] — bridges a Battalion of Paladins (Formation or
//! Phalanx) into the content-processing pipeline.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::PromptTemplate;
use crate::application::services::orchestration::types::{
    ContentProcessingResult, ContentProcessor, OrchestratorError,
};
use crate::core::platform::container::battalion::formation::Formation;
use crate::core::platform::container::battalion::phalanx::Phalanx;
use crate::core::platform::container::content::ContentItem;
use crate::core::platform::container::orchestration_context::OrchestrationContext;
use paladin_battalion::formation_service::FormationExecutionService;
use paladin_battalion::phalanx_service::PhalanxExecutionService;
use paladin_ports::output::paladin_port::PaladinPort;

/// The Battalion pattern a [`BattalionContentProcessor`] runs over content.
///
/// - [`BattalionPattern::Formation`] runs Paladins **sequentially**, threading
///   each agent's output into the next (a pipeline, e.g. summarizer → classifier
///   → entity extractor).
/// - [`BattalionPattern::Phalanx`] runs Paladins **in parallel** and merges their
///   independent outputs.
#[derive(Debug, Clone)]
pub enum BattalionPattern {
    /// Sequential pipeline pattern.
    Formation(Formation),
    /// Parallel analysts pattern.
    Phalanx(Phalanx),
}

impl BattalionPattern {
    /// Returns a stable string identifier for this pattern (for metadata).
    fn as_str(&self) -> &'static str {
        match self {
            BattalionPattern::Formation(_) => "formation",
            BattalionPattern::Phalanx(_) => "phalanx",
        }
    }
}

/// A [`ContentProcessor`] that enriches content by invoking a Battalion of
/// Paladins.
///
/// The processor converts an incoming [`ContentItem`] into a prompt via a
/// configurable [`PromptTemplate`], runs the configured [`BattalionPattern`]
/// through the corresponding Battalion execution service, and merges the
/// multi-agent output into a single [`ContentProcessingResult`].
///
/// # Merge strategy
///
/// - **Formation:** the final pipeline output is surfaced under
///   `result_data.final_output`; each stage's output is preserved under
///   `result_data.agent_outputs` keyed by agent name.
/// - **Phalanx:** every analyst's output is merged under
///   `result_data.agent_outputs` keyed by agent name, and the Battalion's
///   aggregated `final_output` is surfaced under `result_data.final_output`.
///
/// In both cases `metadata` records the pattern used and the participating
/// agents. The processor depends only on the [`PaladinPort`] abstraction (via
/// the Battalion services), so it is unit-testable with mock agents.
pub struct BattalionContentProcessor {
    name: String,
    paladin_port: Arc<dyn PaladinPort>,
    pattern: BattalionPattern,
    prompt_template: PromptTemplate,
}

impl BattalionContentProcessor {
    /// Creates a new processor with the default name and prompt template.
    ///
    /// # Arguments
    ///
    /// * `paladin_port` - Port used by the Battalion services to execute Paladins.
    /// * `pattern` - The Battalion pattern (Formation or Phalanx) to run.
    pub fn new(paladin_port: Arc<dyn PaladinPort>, pattern: BattalionPattern) -> Self {
        Self {
            name: "BattalionContentProcessor".to_string(),
            paladin_port,
            pattern,
            prompt_template: PromptTemplate::default(),
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
}

impl Clone for BattalionContentProcessor {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            paladin_port: Arc::clone(&self.paladin_port),
            pattern: self.pattern.clone(),
            prompt_template: self.prompt_template.clone(),
        }
    }
}

#[async_trait]
impl ContentProcessor for BattalionContentProcessor {
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

        let battalion_result = match &self.pattern {
            BattalionPattern::Formation(formation) => {
                let service = FormationExecutionService::new(Arc::clone(&self.paladin_port));
                service
                    .execute(formation, &prompt)
                    .await
                    .map_err(|e| OrchestratorError::ServiceError(e.to_string()))?
            }
            BattalionPattern::Phalanx(phalanx) => {
                let service = PhalanxExecutionService::new(Arc::clone(&self.paladin_port));
                service
                    .execute(phalanx, &prompt)
                    .await
                    .map_err(|e| OrchestratorError::ServiceError(e.to_string()))?
            }
        };

        let processing_time_ms = start.elapsed().as_millis() as u64;

        // Merge multi-agent output into a single coherent result.
        let mut agent_outputs = serde_json::Map::new();
        let agents: Vec<String> = battalion_result
            .paladin_results
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                // Paladin results don't carry their own name, so key by position.
                let key = format!("agent_{idx}");
                agent_outputs.insert(key.clone(), json!(r.output));
                key
            })
            .collect();

        let result_data = json!({
            "final_output": battalion_result.final_output,
            "agent_outputs": Value::Object(agent_outputs),
        });

        let mut metadata: HashMap<String, Value> = HashMap::new();
        metadata.insert("pattern".to_string(), json!(self.pattern.as_str()));
        metadata.insert(
            "battalion_name".to_string(),
            json!(battalion_result.battalion_name),
        );
        metadata.insert("agents".to_string(), json!(agents));
        metadata.insert(
            "paladin_success_count".to_string(),
            json!(battalion_result.paladin_success_count),
        );
        metadata.insert(
            "paladin_failure_count".to_string(),
            json!(battalion_result.paladin_failure_count),
        );
        metadata.insert(
            "total_tokens".to_string(),
            json!(battalion_result.total_tokens),
        );

        let success = battalion_result.paladin_failure_count == 0;

        Ok(ContentProcessingResult {
            content_id,
            processor_name: self.name.clone(),
            processing_time_ms,
            success,
            result_data: Some(result_data),
            error: None,
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
    use crate::core::base::entity::Entity;
    use crate::core::platform::container::battalion::BattalionConfig;
    use crate::core::platform::container::content::{ContentItem, ContentType, TextContent};
    use crate::core::platform::container::paladin::{Paladin, PaladinData};
    use paladin_ports::output::paladin_port::{PaladinResult, PaladinStream};
    use std::sync::Mutex;

    /// A mock PaladinPort that echoes a per-call response and records the inputs
    /// it received, so we can assert sequential threading vs. parallel fan-out.
    struct MockPaladinPort {
        responses: Mutex<Vec<String>>,
        inputs: Arc<Mutex<Vec<String>>>,
    }

    impl MockPaladinPort {
        fn new(responses: Vec<&str>, inputs: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().map(String::from).collect()),
                inputs,
            }
        }
    }

    #[async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, crate::application::services::paladin::error::PaladinError>
        {
            self.inputs.lock().unwrap().push(input.to_string());
            let mut responses = self.responses.lock().unwrap();
            let output = if responses.is_empty() {
                "default".to_string()
            } else {
                responses.remove(0)
            };
            Ok(PaladinResult {
                output,
                ..Default::default()
            })
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, crate::application::services::paladin::error::PaladinError>
        {
            unimplemented!()
        }

        fn validate(
            &self,
            _paladin: &Paladin,
        ) -> Result<(), crate::application::services::paladin::error::PaladinError> {
            Ok(())
        }
    }

    fn test_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: "test".to_string(),
            name: name.to_string(),
            model: "mock".to_string(),
            ..Default::default()
        };
        Entity::new(data, Some(name.to_string()))
    }

    fn text_item(body: &str) -> ContentItem {
        let content = TextContent::new(None, Some(body.to_string())).expect("text content");
        ContentItem::new_with_title(ContentType::Text(content), "Doc".to_string())
            .expect("content item")
    }

    fn context() -> OrchestrationContext {
        OrchestrationContext::new("test".to_string(), "test".to_string())
    }

    #[tokio::test]
    async fn formation_runs_agents_sequentially_into_one_result() {
        let inputs = Arc::new(Mutex::new(Vec::new()));
        let port = Arc::new(MockPaladinPort::new(
            vec!["summary", "classification"],
            Arc::clone(&inputs),
        ));
        let formation = Formation::new(
            vec![test_paladin("summarizer"), test_paladin("classifier")],
            BattalionConfig::new("pipeline"),
        )
        .expect("formation");

        let processor =
            BattalionContentProcessor::new(port, BattalionPattern::Formation(formation));
        let item = text_item("Original article text.");
        let id = item.uuid();

        let result = processor
            .process_content(item, context())
            .await
            .expect("processing succeeds");

        assert!(result.success);
        assert_eq!(result.content_id, id);
        assert_eq!(result.metadata["pattern"], json!("formation"));
        // The pipeline threads output: agent 2 received agent 1's output.
        let recorded = inputs.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert!(recorded[1].contains("summary"));
        // Final output is the last stage's result.
        let data = result.result_data.unwrap();
        assert_eq!(data["final_output"], json!("classification"));
    }

    #[tokio::test]
    async fn phalanx_runs_agents_in_parallel_and_merges_outputs() {
        let inputs = Arc::new(Mutex::new(Vec::new()));
        let port = Arc::new(MockPaladinPort::new(
            vec!["analyst_a", "analyst_b"],
            Arc::clone(&inputs),
        ));
        let phalanx = Phalanx::new(
            vec![test_paladin("analyst_a"), test_paladin("analyst_b")],
            BattalionConfig::new("analysts"),
        )
        .expect("phalanx");

        let processor = BattalionContentProcessor::new(port, BattalionPattern::Phalanx(phalanx));
        let item = text_item("Some content to analyze.");

        let result = processor
            .process_content(item, context())
            .await
            .expect("processing succeeds");

        assert!(result.success);
        assert_eq!(result.metadata["pattern"], json!("phalanx"));
        // Both analysts received the same original prompt (parallel fan-out).
        let recorded = inputs.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], recorded[1]);
        // Both outputs are merged into the result.
        let data = result.result_data.unwrap();
        let outputs = data["agent_outputs"].as_object().unwrap();
        assert_eq!(outputs.len(), 2);
    }
}
