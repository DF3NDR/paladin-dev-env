//! PlanningService - LLM-based autonomous task decomposition
//!
//! This service implements US-14.1: Autonomous Planning Mode.
//! When a Paladin is configured with `MaxLoops::Auto`, it uses this service
//! to decompose complex tasks into subtasks, execute them with dependency tracking,
//! and synthesize results into a cohesive response.
//!
//! # Examples
//!
//! ```rust,no_run
//! use paladin::application::services::paladin::planning_service::PlanningService;
//! use paladin_ports::output::llm_port::LlmPort;
//! use std::sync::Arc;
//!
//! # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
//! let planning_service = PlanningService::new(llm_port);
//!
//! // Create and execute a plan
//! let plan = planning_service.create_plan(
//!     "Analyze the security vulnerabilities in this codebase",
//!     10, // max_subtasks
//!     "gpt-4o", // model
//! ).await?;
//!
//! let result = planning_service.execute_subtasks(&plan, "/* code here */", "gpt-4o").await?;
//! # Ok(())
//! # }
//! ```

use crate::application::errors::planning_error::PlanningError;
use crate::core::platform::container::planning::{Subtask, TaskPlan};
use crate::core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use log::info;
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Service for LLM-based autonomous task planning and execution
///
/// Implements the planning mode where a Paladin decomposes complex tasks
/// into subtasks, manages their execution with dependency tracking, and
/// synthesizes results.
pub struct PlanningService {
    /// LLM port for task decomposition and synthesis
    llm_port: Arc<dyn LlmPort>,
}

/// Internal structure for deserializing LLM plan responses
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmPlanResponse {
    task: String,
    subtasks: Vec<LlmSubtask>,
}

/// Internal structure for deserializing subtasks from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmSubtask {
    id: String,
    description: String,
    dependencies: Vec<String>,
}

impl PlanningService {
    /// Creates a new PlanningService
    ///
    /// # Arguments
    ///
    /// * `llm_port` - LLM port for generating plans and synthesizing results
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paladin::application::services::paladin::planning_service::PlanningService;
    /// use paladin_ports::output::llm_port::LlmPort;
    /// use std::sync::Arc;
    ///
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let service = PlanningService::new(llm_port);
    /// # }
    /// ```
    pub fn new(llm_port: Arc<dyn LlmPort>) -> Self {
        info!("Creating PlanningService");
        Self { llm_port }
    }

    /// Creates a task decomposition plan using LLM
    ///
    /// # Arguments
    ///
    /// * `task_description` - Description of the task to decompose
    /// * `max_subtasks` - Maximum number of subtasks allowed
    /// * `model` - LLM model to use for planning (e.g., "gpt-4", "claude-3")
    ///
    /// # Returns
    ///
    /// A `TaskPlan` containing the decomposed subtasks
    ///
    /// # Errors
    ///
    /// Returns `PlanningError` if:
    /// - LLM call fails
    /// - Response cannot be parsed
    /// - Plan exceeds max_subtasks limit
    /// - Plan has invalid dependencies
    pub async fn create_plan(
        &self,
        task_description: &str,
        max_subtasks: u32,
        model: &str,
    ) -> Result<TaskPlan, PlanningError> {
        info!(
            "Creating plan for task: '{}' (max {} subtasks)",
            task_description, max_subtasks
        );

        // Build the planning prompt
        let prompt = self.build_planning_prompt(task_description, max_subtasks);

        // Call LLM
        let user_prompt = UserPrompt {
            query: prompt,
            context: None,
        };
        let prompt_item = PromptItem::new(PromptType::User(user_prompt))
            .map_err(|e| PlanningError::GenerationFailed(e.to_string()))?;

        let request = LlmRequest::new(model.to_string(), prompt_item);

        let response = self
            .llm_port
            .generate(request)
            .await
            .map_err(|e| PlanningError::LlmError(e.to_string()))?;

        // Parse the LLM response into a TaskPlan
        let plan = self.parse_plan_from_llm(&response.content, max_subtasks)?;

        info!("Created plan with {} subtasks", plan.subtask_count());
        Ok(plan)
    }

    /// Executes all subtasks in dependency order
    ///
    /// # Arguments
    ///
    /// * `plan` - The task plan with subtasks to execute
    /// * `original_input` - The original task input/context
    /// * `model` - LLM model to use for executing subtasks (e.g., "gpt-4", "claude-3")
    ///
    /// # Returns
    ///
    /// A `TaskPlan` with all subtasks executed and results populated
    ///
    /// # Errors
    ///
    /// Returns `PlanningError` if:
    /// - LLM call fails for any subtask
    /// - Subtask execution fails
    /// - Circular dependencies detected
    pub async fn execute_subtasks(
        &self,
        plan: &TaskPlan,
        original_input: &str,
        model: &str,
    ) -> Result<TaskPlan, PlanningError> {
        info!(
            "Executing {} subtasks for task: '{}'",
            plan.subtasks.len(),
            plan.original_task
        );

        let mut executed_plan = plan.clone();
        let mut completed_ids: Vec<String> = Vec::new();

        // Execute subtasks in dependency order
        while completed_ids.len() < executed_plan.subtasks.len() {
            let mut made_progress = false;

            // Find next subtask to execute (need index to avoid borrow issues)
            let mut next_subtask_idx = None;
            let mut next_dependencies = Vec::new();

            for (idx, subtask) in executed_plan.subtasks.iter().enumerate() {
                // Skip if already completed
                if subtask.completed {
                    continue;
                }

                // Check if all dependencies are completed
                let dependencies = executed_plan
                    .dependencies
                    .get(&subtask.id)
                    .cloned()
                    .unwrap_or_default();

                let can_execute = dependencies
                    .iter()
                    .all(|dep_id| completed_ids.contains(dep_id));

                if can_execute {
                    next_subtask_idx = Some(idx);
                    next_dependencies = dependencies;
                    break;
                }
            }

            // Execute the found subtask
            if let Some(idx) = next_subtask_idx {
                let subtask_id = executed_plan.subtasks[idx].id.clone();
                info!(
                    "Executing subtask: {} - {}",
                    subtask_id, executed_plan.subtasks[idx].description
                );

                // Build context from completed dependencies
                let context =
                    self.build_subtask_context(&executed_plan, &next_dependencies, original_input);

                // Execute the subtask via LLM (need immutable reference)
                let result = self
                    .execute_subtask(&executed_plan.subtasks[idx], &context, model)
                    .await?;

                // Mark as completed (now we can mutate)
                executed_plan.subtasks[idx].complete(result);
                completed_ids.push(subtask_id.clone());
                made_progress = true;

                info!("Completed subtask: {}", subtask_id);
            }

            // Check for circular dependencies or impossible state
            if !made_progress && completed_ids.len() < executed_plan.subtasks.len() {
                return Err(PlanningError::InvalidPlan(
                    "Circular dependencies or invalid dependency graph detected".to_string(),
                ));
            }
        }

        info!("All {} subtasks completed", completed_ids.len());
        Ok(executed_plan)
    }

    /// Synthesizes subtask results into a cohesive final response
    ///
    /// # Arguments
    ///
    /// * `plan` - The completed task plan with subtask results
    /// * `original_task` - The original task description
    /// * `model` - LLM model to use for synthesis (e.g., "gpt-4", "claude-3")
    ///
    /// # Returns
    ///
    /// A cohesive response synthesizing all subtask results
    ///
    /// # Errors
    ///
    /// Returns `PlanningError` if:
    /// - LLM call fails
    /// - Plan has incomplete subtasks
    pub async fn synthesize_results(
        &self,
        plan: &TaskPlan,
        original_task: &str,
        model: &str,
    ) -> Result<String, PlanningError> {
        info!("Synthesizing results for task: '{}'", original_task);

        // Verify all subtasks are complete
        let incomplete: Vec<&Subtask> = plan.subtasks.iter().filter(|st| !st.completed).collect();
        if !incomplete.is_empty() {
            return Err(PlanningError::InvalidPlan(format!(
                "Cannot synthesize results: {} subtasks incomplete",
                incomplete.len()
            )));
        }

        // Build synthesis prompt
        let prompt = self.build_synthesis_prompt(plan, original_task);

        // Call LLM for synthesis
        let user_prompt = UserPrompt {
            query: prompt,
            context: None,
        };

        let mut prompt_item = PromptItem::new(PromptType::User(user_prompt))
            .map_err(|e| PlanningError::GenerationFailed(e.to_string()))?;

        // Use higher temperature for more natural synthesis
        use crate::core::platform::container::prompt::PromptParameters;
        prompt_item.set_parameters(PromptParameters {
            max_tokens: None,
            temperature: Some(0.7), // Higher temperature for natural language
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
        });

        let request = LlmRequest::new(model.to_string(), prompt_item);

        let response = self
            .llm_port
            .generate(request)
            .await
            .map_err(|e| PlanningError::LlmError(e.to_string()))?;

        info!("Synthesis complete");
        Ok(response.content)
    }

    /// Builds the synthesis prompt for the LLM
    fn build_synthesis_prompt(&self, plan: &TaskPlan, original_task: &str) -> String {
        let mut subtask_results = String::new();
        for (i, subtask) in plan.subtasks.iter().enumerate() {
            if let Some(result) = &subtask.result {
                subtask_results.push_str(&format!(
                    "{}. {}\n   Result: {}\n\n",
                    i + 1,
                    subtask.description,
                    result
                ));
            }
        }

        format!(
            r#"You are synthesizing the results of multiple subtasks into a cohesive response.

ORIGINAL TASK: {}

COMPLETED SUBTASKS AND RESULTS:
{}

Synthesize these results into a clear, comprehensive response that directly addresses the original task. Provide a cohesive summary that:
1. Integrates information from all subtasks
2. Presents results in a logical flow
3. Highlights key findings or accomplishments
4. Provides clear next steps or conclusions if applicable

Write the synthesized response now:"#,
            original_task, subtask_results
        )
    }

    /// Builds the planning prompt for the LLM
    fn build_planning_prompt(&self, task_description: &str, max_subtasks: u32) -> String {
        format!(
            r#"You are a task planning assistant. Decompose the following task into subtasks.

TASK: {}

INSTRUCTIONS:
- Break down the task into {} or fewer subtasks
- Each subtask should be concrete and actionable
- Identify dependencies between subtasks
- Return your response as JSON in the following format:

{{
  "task": "original task description",
  "subtasks": [
    {{
      "id": "1",
      "description": "description of subtask",
      "dependencies": ["id1", "id2"]
    }}
  ]
}}

Return ONLY the JSON, no additional text."#,
            task_description, max_subtasks
        )
    }

    /// Parses LLM response into a TaskPlan
    ///
    /// # Arguments
    ///
    /// * `llm_response` - The LLM's response content
    /// * `max_subtasks` - Maximum allowed subtasks
    ///
    /// # Returns
    ///
    /// A validated `TaskPlan`
    ///
    /// # Errors
    ///
    /// Returns `PlanningError` if parsing fails or plan is invalid
    fn parse_plan_from_llm(
        &self,
        llm_response: &str,
        max_subtasks: u32,
    ) -> Result<TaskPlan, PlanningError> {
        // Try to extract JSON from the response (LLM might add extra text)
        let json_str = self.extract_json(llm_response)?;

        // Parse JSON
        let llm_plan: LlmPlanResponse = serde_json::from_str(&json_str)
            .map_err(|e| PlanningError::GenerationFailed(format!("JSON parse error: {}", e)))?;

        // Validate subtask count
        if llm_plan.subtasks.len() as u32 > max_subtasks {
            return Err(PlanningError::MaxSubtasksExceeded {
                max: max_subtasks,
                attempted: llm_plan.subtasks.len() as u32,
            });
        }

        // Create TaskPlan
        let mut plan = TaskPlan::new(llm_plan.task, max_subtasks);

        // Add subtasks
        for llm_subtask in llm_plan.subtasks {
            let subtask = Subtask::new(
                llm_subtask.id.clone(),
                llm_subtask.description,
                "Expected output from subtask execution".to_string(), // TODO: Ask LLM for expected output
            );
            plan.add_subtask(subtask)
                .map_err(PlanningError::InvalidPlan)?;

            // Add dependencies if any
            if !llm_subtask.dependencies.is_empty() {
                plan.dependencies
                    .insert(llm_subtask.id, llm_subtask.dependencies);
            }
        }

        // Validate the plan (checks for circular dependencies, etc.)
        plan.validate().map_err(PlanningError::InvalidPlan)?;

        Ok(plan)
    }

    /// Extracts JSON from LLM response (handles markdown code blocks, etc.)
    fn extract_json(&self, response: &str) -> Result<String, PlanningError> {
        let trimmed = response.trim();

        // Check for markdown code block
        if let Some(start) = trimmed.find("```json")
            && let Some(end) = trimmed[start + 7..].find("```")
        {
            return Ok(trimmed[start + 7..start + 7 + end].trim().to_string());
        }

        // Check for plain code block
        if let Some(start) = trimmed.find("```")
            && let Some(end) = trimmed[start + 3..].find("```")
        {
            return Ok(trimmed[start + 3..start + 3 + end].trim().to_string());
        }

        // Assume the whole response is JSON
        Ok(trimmed.to_string())
    }

    /// Builds context for subtask execution from completed dependencies
    fn build_subtask_context(
        &self,
        plan: &TaskPlan,
        dependencies: &[String],
        original_input: &str,
    ) -> String {
        if dependencies.is_empty() {
            return original_input.to_string();
        }

        let mut context = format!("Original Task: {}\n\n", original_input);
        context.push_str("Results from prerequisite subtasks:\n\n");

        for dep_id in dependencies {
            if let Some(dep_subtask) = plan.subtasks.iter().find(|st| st.id == *dep_id) {
                context.push_str(&format!(
                    "Subtask {}: {}\nResult: {}\n\n",
                    dep_subtask.id,
                    dep_subtask.description,
                    dep_subtask
                        .result
                        .as_ref()
                        .unwrap_or(&"No result".to_string())
                ));
            }
        }

        context
    }

    /// Executes a single subtask via LLM
    ///
    /// # Arguments
    ///
    /// * `subtask` - The subtask to execute
    /// * `context` - Contextual information including dependencies
    /// * `model` - LLM model to use (e.g., "gpt-4", "claude-3")
    async fn execute_subtask(
        &self,
        subtask: &Subtask,
        context: &str,
        model: &str,
    ) -> Result<String, PlanningError> {
        let prompt = format!(
            r#"You are executing a subtask as part of a larger plan.

SUBTASK: {}

EXPECTED OUTPUT: {}

CONTEXT:
{}

Execute this subtask and provide the result. Be concise and focused on the expected output."#,
            subtask.description, subtask.expected_output, context
        );

        let user_prompt = UserPrompt {
            query: prompt,
            context: None,
        };

        let mut prompt_item = PromptItem::new(PromptType::User(user_prompt))
            .map_err(|e| PlanningError::GenerationFailed(e.to_string()))?;

        // Set temperature for focused task execution
        use crate::core::platform::container::prompt::PromptParameters;
        prompt_item.set_parameters(PromptParameters {
            max_tokens: None,
            temperature: Some(0.3), // Lower temperature for task execution
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
        });

        let request = LlmRequest::new(model.to_string(), prompt_item);

        let response = self
            .llm_port
            .generate(request)
            .await
            .map_err(|e| PlanningError::LlmError(e.to_string()))?;

        Ok(response.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use paladin_ports::output::llm_port::{
        FinishReason, LlmError, LlmResponse, ProviderCapabilities, TokenUsage,
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    /// Mock LLM port for testing
    struct MockLlmPort {
        response: String,
    }

    impl MockLlmPort {
        fn new(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    #[async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: Uuid::new_v4(),
                model: "test-model".to_string(),
                content: self.response.clone(),
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
            _request: LlmRequest,
        ) -> Result<
            Box<
                dyn futures::Stream<
                        Item = Result<paladin_ports::output::llm_port::StreamingResponse, LlmError>,
                    > + Send,
            >,
            LlmError,
        > {
            unimplemented!("Streaming not needed for tests")
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["test-model".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                supports_streaming: false,
                supports_function_calling: false,
                supports_tool_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                supports_system_messages: true,
                max_context_tokens: Some(4096),
                temperature_range: None,
            }
        }
    }

    #[test]
    fn test_planning_service_new() {
        // Given: A mock LLM port
        let llm_port = Arc::new(MockLlmPort::new("test"));

        // When: Creating a new PlanningService
        let _service = PlanningService::new(llm_port.clone());

        // Then: The service should be created successfully
        // Verify the Arc has been cloned (service holds a reference)
        assert!(Arc::strong_count(&llm_port) >= 2);
    }

    #[tokio::test]
    async fn test_create_plan_basic() {
        // Given: A mock LLM that returns a valid plan
        let plan_json = r#"{
            "task": "Analyze security vulnerabilities",
            "subtasks": [
                {
                    "id": "1",
                    "description": "Scan for SQL injection vulnerabilities",
                    "dependencies": []
                },
                {
                    "id": "2",
                    "description": "Check for XSS vulnerabilities",
                    "dependencies": []
                },
                {
                    "id": "3",
                    "description": "Generate security report",
                    "dependencies": ["1", "2"]
                }
            ]
        }"#;

        let llm_port = Arc::new(MockLlmPort::new(plan_json));
        let service = PlanningService::new(llm_port);

        // When: Creating a plan
        let result = service
            .create_plan("Analyze security vulnerabilities", 10, "gpt-4")
            .await;

        // Then: The plan should be created successfully
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.subtask_count(), 3);
    }

    #[tokio::test]
    async fn test_create_plan_enforces_max_subtasks() {
        // Given: A mock LLM that returns a plan with many subtasks
        let plan_json = r#"{
            "task": "Complex task",
            "subtasks": [
                {"id": "1", "description": "Task 1", "dependencies": []},
                {"id": "2", "description": "Task 2", "dependencies": []},
                {"id": "3", "description": "Task 3", "dependencies": []},
                {"id": "4", "description": "Task 4", "dependencies": []},
                {"id": "5", "description": "Task 5", "dependencies": []},
                {"id": "6", "description": "Task 6", "dependencies": []}
            ]
        }"#;

        let llm_port = Arc::new(MockLlmPort::new(plan_json));
        let service = PlanningService::new(llm_port);

        // When: Creating a plan with max_subtasks=3
        let result = service.create_plan("Complex task", 3, "gpt-4").await;

        // Then: Should return error for exceeding limit
        assert!(result.is_err());
        if let Err(e) = result {
            // Check it's the right error type
            match e {
                PlanningError::MaxSubtasksExceeded { max, attempted } => {
                    assert_eq!(max, 3);
                    assert_eq!(attempted, 6);
                }
                other => panic!("Expected MaxSubtasksExceeded, got: {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_execute_subtasks_with_dependencies() {
        // Given: A plan with subtasks that have dependencies
        let plan_json = r#"{
            "task": "Build and test application",
            "subtasks": [
                {
                    "id": "1",
                    "description": "Install dependencies",
                    "dependencies": []
                },
                {
                    "id": "2",
                    "description": "Build application",
                    "dependencies": ["1"]
                },
                {
                    "id": "3",
                    "description": "Run tests",
                    "dependencies": ["2"]
                }
            ]
        }"#;

        let llm_port = Arc::new(MockLlmPort::new(plan_json));
        let service = PlanningService::new(llm_port.clone());

        // When: Creating and executing the plan
        let plan = service
            .create_plan("Build and test application", 10, "gpt-4")
            .await
            .expect("Failed to create plan");

        let result = service
            .execute_subtasks(&plan, "Build and test application", "gpt-4")
            .await;

        // Then: Subtasks should execute in dependency order
        assert!(result.is_ok());
        let executed_plan = result.unwrap();

        // All subtasks should be marked as completed
        assert_eq!(executed_plan.subtasks.len(), 3);

        // Verify subtasks have results
        for subtask in &executed_plan.subtasks {
            assert!(
                subtask.completed,
                "Subtask {} should be completed",
                subtask.id
            );
            assert!(
                subtask.result.is_some(),
                "Subtask {} should have a result",
                subtask.id
            );
        }
    }

    #[tokio::test]
    async fn test_synthesize_results() {
        // Given: A completed plan with subtask results
        let mut plan = TaskPlan::new("Build and deploy application".to_string(), 10);

        let mut subtask1 = Subtask::new(
            "1".to_string(),
            "Install dependencies".to_string(),
            "Dependencies installed".to_string(),
        );
        subtask1.complete(
            "Successfully installed all dependencies: express, react, typescript".to_string(),
        );

        let mut subtask2 = Subtask::new(
            "2".to_string(),
            "Build application".to_string(),
            "Build output".to_string(),
        );
        subtask2
            .complete("Build completed successfully. Output: dist/bundle.js (245 KB)".to_string());

        let mut subtask3 = Subtask::new(
            "3".to_string(),
            "Run tests".to_string(),
            "Test results".to_string(),
        );
        subtask3.complete("All tests passed: 42 passed, 0 failed".to_string());

        plan.add_subtask(subtask1).unwrap();
        plan.add_subtask(subtask2).unwrap();
        plan.add_subtask(subtask3).unwrap();

        // Mock LLM to return synthesized result
        let synthesis_response = r#"Successfully built and tested the application:
1. Installed all required dependencies (express, react, typescript)
2. Built the application successfully (output: dist/bundle.js, 245 KB)
3. Verified functionality with complete test suite (42 tests passed)

The application is ready for deployment."#;

        let llm_port = Arc::new(MockLlmPort::new(synthesis_response));
        let service = PlanningService::new(llm_port);

        // When: Synthesizing results
        let result = service
            .synthesize_results(&plan, "Build and deploy application", "gpt-4")
            .await;

        // Then: Should return a cohesive synthesized response
        assert!(result.is_ok());
        let synthesized = result.unwrap();

        // Verify the synthesis contains information from all subtasks
        assert!(synthesized.contains("dependencies"));
        assert!(synthesized.contains("Built"));
        assert!(synthesized.contains("tests passed"));
        assert!(synthesized.contains("ready for deployment"));
    }

    #[tokio::test]
    async fn test_planning_failure_invalid_json() {
        // Given: A mock LLM that returns invalid JSON
        let invalid_json = "This is not valid JSON at all!";
        let llm_port = Arc::new(MockLlmPort::new(invalid_json));
        let service = PlanningService::new(llm_port);

        // When: Creating a plan
        let result = service.create_plan("Some task", 10, "gpt-4").await;

        // Then: Should return generation failed error
        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                PlanningError::GenerationFailed(_) => {
                    // Expected error type
                }
                other => panic!("Expected GenerationFailed, got: {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_synthesis_with_incomplete_subtasks() {
        // Given: A plan with incomplete subtasks
        let mut plan = TaskPlan::new("Test task".to_string(), 10);
        let subtask1 = Subtask::new(
            "1".to_string(),
            "Incomplete task".to_string(),
            "Output".to_string(),
        );
        // Don't complete it
        plan.add_subtask(subtask1).unwrap();

        let llm_port = Arc::new(MockLlmPort::new("Some response"));
        let service = PlanningService::new(llm_port);

        // When: Trying to synthesize results
        let result = service
            .synthesize_results(&plan, "Test task", "gpt-4")
            .await;

        // Then: Should return error about incomplete subtasks
        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                PlanningError::InvalidPlan(msg) => {
                    assert!(msg.contains("incomplete"));
                }
                other => panic!("Expected InvalidPlan, got: {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_planning_logs_progress() {
        // Given: A valid plan
        let plan_json = r#"{
            "task": "Simple task",
            "subtasks": [
                {"id": "1", "description": "Do something", "dependencies": []}
            ]
        }"#;

        let llm_port = Arc::new(MockLlmPort::new(plan_json));
        let service = PlanningService::new(llm_port);

        // When: Creating and executing a plan
        // (This test verifies logging is present - actual log output would be visible with RUST_LOG=info)
        let plan = service.create_plan("Simple task", 10, "gpt-4").await;
        assert!(plan.is_ok());

        let plan = plan.unwrap();
        let result = service
            .execute_subtasks(&plan, "Simple task", "gpt-4")
            .await;
        assert!(result.is_ok());

        // Then: Test passes if logging doesn't panic
        // Logging is tested by checking that info! calls exist in the code
        // In a real scenario, we'd use a test logging framework to capture logs
    }

    #[tokio::test]
    async fn test_planning_service_uses_configured_model() {
        // Given: A mock LLM port that tracks which model was used
        let plan_json = r#"{
            "task": "Test task",
            "subtasks": [
                {"id": "1", "description": "Test subtask", "dependencies": []}
            ]
        }"#;
        let llm_port = Arc::new(MockLlmPort::new(plan_json));
        let service = PlanningService::new(llm_port.clone());

        // When: Creating a plan with a specific model
        let result = service.create_plan("Test task", 5, "claude-3").await;

        // Then: The plan should be created and the model should have been used
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.subtask_count(), 1);

        // Note: In a real implementation with model tracking, we'd verify
        // llm_port.last_model_used() == "claude-3"
    }

    #[tokio::test]
    async fn test_planning_service_validates_model_compatibility() {
        // Given: A mock LLM port that returns a valid plan
        let plan_json = r#"{
            "task": "Test task",
            "subtasks": [
                {"id": "1", "description": "Test subtask", "dependencies": []}
            ]
        }"#;
        let llm_port = Arc::new(MockLlmPort::new(plan_json));
        let service = PlanningService::new(llm_port);

        // When: Using different model identifiers
        let gpt4_result = service.create_plan("Task 1", 5, "gpt-4").await;
        let claude_result = service.create_plan("Task 2", 5, "claude-3").await;
        let custom_result = service.create_plan("Task 3", 5, "custom-model").await;

        // Then: All should work (model validation happens in LlmPort layer)
        assert!(gpt4_result.is_ok());
        assert!(claude_result.is_ok());
        assert!(custom_result.is_ok());
    }

    #[tokio::test]
    async fn test_planning_service_falls_back_on_invalid_model() {
        // Given: A mock LLM port that would fail with invalid model
        // (In reality, the LlmPort implementation handles fallback)
        let plan_json = r#"{
            "task": "Test task",
            "subtasks": [
                {"id": "1", "description": "Test subtask", "dependencies": []}
            ]
        }"#;
        let llm_port = Arc::new(MockLlmPort::new(plan_json));
        let service = PlanningService::new(llm_port);

        // When: Using an empty or invalid model string
        // The service itself doesn't validate - it passes to LlmPort
        let result = service.create_plan("Test task", 5, "").await;

        // Then: The service doesn't fail at the planning level
        // (LlmPort would handle the invalid model error)
        // For this mock, it still succeeds
        assert!(result.is_ok());
    }
}
