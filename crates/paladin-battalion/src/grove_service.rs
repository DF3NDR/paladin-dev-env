//! Grove Execution Service
//!
//! Provides intelligent routing and execution logic for the Grove pattern,
//! directing tasks to specialized Paladins based on expertise matching.

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use paladin_core::platform::container::battalion::BattalionError;
use paladin_core::platform::container::battalion::grove::{
    Grove, RoutingStrategy, Tree, TreeAgent,
};
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_ports::output::embedding_port::EmbeddingPort;
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::paladin_registry::PaladinRegistry;

/// Decision made by the Grove routing system
///
/// Contains information about which agent was selected and why,
/// along with confidence metrics.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Name of the selected Tree
    pub selected_tree: String,

    /// ID of the selected agent (Paladin)
    pub selected_agent: String,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Human-readable reasoning for the selection
    pub reasoning: String,
}

/// Result of Grove task execution
///
/// Contains the routing decision and the execution result from
/// the selected Paladin.
#[derive(Debug, Clone)]
pub struct GroveResult {
    /// Routing decision that was made
    pub routing_decision: RoutingDecision,

    /// Result from executing the selected Paladin
    pub execution_result: String,

    /// Additional metadata about the execution
    pub metadata: HashMap<String, String>,
}

/// Error message returned when `RoutingStrategy::LlmRouting` is selected but no
/// operator-configured `routing_model` is present (D-02).
///
/// This is the single definition of the message text: both `route_task`'s pre-dispatch
/// check and `route_by_llm`'s in-strategy guard call [`GroveExecutionService::resolve_routing_model`],
/// which returns this exact string, so the dispatch-layer check and the strategy-layer guard
/// cannot drift apart. ADR-0013, `CHANGELOG.md` and the pre-existing `route_by_llm` guard
/// tests all key on this wording — do not change it.
const MISSING_ROUTING_MODEL_ERROR: &str = "routing_model not configured for LLM-based routing";

/// LLM routing response structure
///
/// Expected JSON structure from LLM when performing routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoutingResponse {
    /// Name of the selected tree
    tree_name: String,

    /// ID of the selected agent
    agent_id: String,

    /// Confidence score (0.0 to 1.0)
    confidence: f32,

    /// Reasoning for the selection
    reasoning: String,
}

/// Service for executing Grove patterns
///
/// Routes incoming tasks to specialized Paladins using various routing strategies
/// (keyword matching, semantic similarity, LLM-based routing).
///
/// # Examples
///
/// ```ignore
/// use paladin_battalion::grove_service::GroveExecutionService;
/// use std::sync::Arc;
///
/// let service = GroveExecutionService::new(
///     paladin_port,
///     Some(embedding_port),
///     Some(llm_port)
/// );
/// let result = service.execute(&grove, "Fix the authentication bug").await?;
/// println!("Routed to: {}", result.routing_decision.selected_agent);
/// ```
pub struct GroveExecutionService {
    /// Paladin execution port
    paladin_port: Arc<dyn PaladinPort>,

    /// Optional embedding port for semantic similarity routing
    embedding_port: Option<Arc<dyn EmbeddingPort>>,

    /// Optional LLM port for LLM-based routing (will be used in Task 5.0)
    #[allow(dead_code)]
    llm_port: Option<Arc<dyn LlmPort>>,

    /// Paladin registry for resolving routed agents
    registry: Arc<dyn PaladinRegistry>,
}

impl GroveExecutionService {
    /// Creates a new GroveExecutionService
    ///
    /// # Arguments
    ///
    /// * `paladin_port` - Port for executing Paladins
    /// * `embedding_port` - Optional port for generating embeddings
    /// * `llm_port` - Optional port for LLM calls (used in LLM routing)
    /// * `registry` - Paladin registry for resolving agent IDs to Paladin instances
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = GroveExecutionService::new(
    ///     paladin_port,
    ///     Some(embedding_port),
    ///     Some(llm_port),
    ///     registry
    /// );
    /// ```
    pub fn new(
        paladin_port: Arc<dyn PaladinPort>,
        embedding_port: Option<Arc<dyn EmbeddingPort>>,
        llm_port: Option<Arc<dyn LlmPort>>,
        registry: Arc<dyn PaladinRegistry>,
    ) -> Self {
        Self {
            paladin_port,
            embedding_port,
            llm_port,
            registry,
        }
    }

    /// Executes a task using the Grove pattern
    ///
    /// Routes the task to the most appropriate agent based on the Grove's
    /// routing strategy, then executes that agent with the task.
    ///
    /// # Arguments
    ///
    /// * `grove` - The Grove configuration with trees and agents
    /// * `task` - The task description to route and execute
    ///
    /// # Returns
    ///
    /// Returns a `GroveResult` containing the routing decision and execution result.
    ///
    /// # Errors
    ///
    /// Returns `BattalionError` if:
    /// - Routing fails and no fallback is available
    /// - `routing_strategy` is `RoutingStrategy::LlmRouting` and `routing_model` is absent or
    ///   blank (`BattalionError::RoutingError`, naming `routing_model`) — this specific
    ///   configuration error is excluded from all fallback handling; it never consults
    ///   `fallback_tree` or default agent selection (D-02)
    /// - Required embedding port is missing for SemanticSimilarity strategy
    /// - Agent not found in registry (`PaladinNotFound`)
    /// - Paladin execution fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = service.execute(&grove, "Analyze customer feedback").await?;
    /// println!("Agent: {}", result.routing_decision.selected_agent);
    /// println!("Result: {}", result.execution_result);
    /// ```
    pub async fn execute(&self, grove: &Grove, task: &str) -> Result<GroveResult, BattalionError> {
        info!("Grove '{}' routing task: {}", grove.node.name, task);

        // Route the task to the appropriate agent
        let routing_decision = self.route_task(grove, task).await?;

        debug!(
            "Routed to agent '{}' in tree '{}' (confidence: {:.2})",
            routing_decision.selected_agent,
            routing_decision.selected_tree,
            routing_decision.confidence
        );

        // Resolve the selected Paladin from registry
        let paladin = self
            .registry
            .get(&routing_decision.selected_agent)
            .ok_or_else(|| {
                BattalionError::PaladinNotFound(format!(
                    "Agent '{}' not found in registry",
                    routing_decision.selected_agent
                ))
            })?;

        // Execute the selected Paladin
        let execution_result = self.execute_agent(&paladin, task).await.map_err(|e| {
            BattalionError::ExecutionError(format!(
                "Failed to execute agent '{}': {}",
                routing_decision.selected_agent, e
            ))
        })?;

        // Build metadata
        let mut metadata = HashMap::new();
        metadata.insert("grove_name".to_string(), grove.node.name.clone());
        metadata.insert(
            "routing_strategy".to_string(),
            format!("{:?}", grove.node.config.routing_strategy),
        );
        metadata.insert(
            "confidence".to_string(),
            routing_decision.confidence.to_string(),
        );

        Ok(GroveResult {
            routing_decision,
            execution_result,
            metadata,
        })
    }

    /// Resolves and validates the operator-configured `routing_model` for LLM-based routing.
    ///
    /// This is the single definition of the D-02 configuration check: both `route_task`'s
    /// pre-dispatch early return and `route_by_llm`'s in-strategy guard call this function,
    /// so the dispatch-layer check and the strategy-layer guard cannot drift apart.
    ///
    /// A `None` value and a string that is empty after trimming are treated identically as
    /// unconfigured. There is no fallback of any kind here: this function must not consult
    /// `routing_fallback`, must not call `handle_routing_failure`, and must not query the LLM
    /// port for its available models — a misconfigured Grove fails loudly rather than silently
    /// substituting a model the operator did not choose.
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::RoutingError` naming `routing_model` when `grove`'s
    /// `routing_model` is absent or blank.
    fn resolve_routing_model(grove: &Grove) -> Result<&str, BattalionError> {
        grove
            .node
            .config
            .routing_model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .ok_or_else(|| BattalionError::RoutingError(MISSING_ROUTING_MODEL_ERROR.to_string()))
    }

    /// Routes a task to the most appropriate agent
    ///
    /// Uses the Grove's configured routing strategy to select the best agent
    /// for the given task. Falls back to alternative strategies or fallback
    /// trees if the primary routing fails.
    ///
    /// This fallback behaviour applies to routing *failures* only — a transient LLM call
    /// failure, unparseable JSON, a below-threshold confidence, or an absent `llm_port`. It
    /// explicitly excludes the missing-`routing_model` configuration error (D-02), which is
    /// resolved before dispatch below and propagated to the caller with no fallback consulted.
    ///
    /// # Arguments
    ///
    /// * `grove` - The Grove configuration
    /// * `task` - The task to route
    ///
    /// # Returns
    ///
    /// Returns a `RoutingDecision` containing the selected agent and reasoning.
    ///
    /// # Errors
    ///
    /// Returns `BattalionError` if routing fails and no fallback is available.
    async fn route_task(
        &self,
        grove: &Grove,
        task: &str,
    ) -> Result<RoutingDecision, BattalionError> {
        let strategy = &grove.node.config.routing_strategy;

        // D-02: a Grove selecting LLM routing with no configured routing_model is a
        // configuration error, not a routing failure. Resolve it here, above the dispatch
        // below, so the `?` propagates it directly to `execute()`'s caller before it can ever
        // enter the `match result { .. Err(e) => .. }` fallback arm. Neither `fallback_tree`
        // nor the `grove.node.trees.first()` terminal fallback may absorb this case — every
        // *other* routing failure (transient LLM call failure, unparseable JSON,
        // below-threshold confidence, an absent `llm_port`) deliberately keeps its existing
        // fallback behaviour and is unaffected by this check.
        if matches!(strategy, RoutingStrategy::LlmRouting) {
            Self::resolve_routing_model(grove)?;
        }

        // Try the configured strategy
        let result = match strategy {
            RoutingStrategy::KeywordMatch => self.route_by_keywords(grove, task),
            RoutingStrategy::SemanticSimilarity => {
                self.route_by_semantic_similarity(grove, task).await
            }
            RoutingStrategy::LlmRouting => self.route_by_llm(grove, task).await,
        };

        match result {
            Ok(decision) => Ok(decision),
            Err(e) => {
                warn!(
                    "Primary routing strategy failed: {}. Attempting fallback.",
                    e
                );

                // Try fallback tree if configured
                if let Some(ref fallback_tree_name) = grove.node.config.fallback_tree
                    && let Some(fallback_tree) = grove
                        .node
                        .trees
                        .iter()
                        .find(|t| &t.name == fallback_tree_name)
                {
                    info!("Using fallback tree: {}", fallback_tree_name);
                    return self.select_from_tree(fallback_tree, task, "fallback");
                }

                // Final fallback: use first agent in first tree
                if let Some(first_tree) = grove.node.trees.first() {
                    warn!(
                        "Using default fallback: first agent in tree '{}'",
                        first_tree.name
                    );
                    return self.select_from_tree(first_tree, task, "default_fallback");
                }

                Err(BattalionError::RoutingError(
                    "No agents available for routing".to_string(),
                ))
            }
        }
    }

    /// Routes by keyword matching
    ///
    /// Tokenizes the task, counts matching keywords for each agent,
    /// and selects the agent with the highest overlap.
    fn route_by_keywords(
        &self,
        grove: &Grove,
        task: &str,
    ) -> Result<RoutingDecision, BattalionError> {
        debug!("Routing by keyword matching");

        // Normalize and tokenize the task
        let task_lower = task.to_lowercase();
        let task_tokens: Vec<String> = task_lower
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let mut best_score = 0;
        let mut best_tree: Option<&Tree> = None;
        let mut best_agent: Option<&TreeAgent> = None;

        // Score each agent in each tree
        for tree in &grove.node.trees {
            for agent in &tree.agents {
                let score = self.calculate_keyword_score(&task_tokens, agent);

                // Track best score, but also track first agent as fallback
                if best_agent.is_none() {
                    best_tree = Some(tree);
                    best_agent = Some(agent);
                }

                if score > best_score {
                    best_score = score;
                    best_tree = Some(tree);
                    best_agent = Some(agent);
                }
            }
        }

        // Check if we found a match
        if let (Some(tree), Some(agent)) = (best_tree, best_agent) {
            let confidence = if task_tokens.is_empty() {
                0.5 // No keywords in task
            } else {
                (best_score as f32 / task_tokens.len() as f32).min(1.0)
            };

            Ok(RoutingDecision {
                selected_tree: tree.name.clone(),
                selected_agent: agent.paladin_id.clone(),
                confidence,
                reasoning: format!(
                    "Matched {} keywords from agent expertise: {:?}",
                    best_score, agent.expertise_keywords
                ),
            })
        } else {
            Err(BattalionError::RoutingError(
                "No agents found for keyword matching".to_string(),
            ))
        }
    }

    /// Calculates keyword overlap score for an agent
    ///
    /// Counts how many task tokens match the agent's expertise keywords
    /// using case-insensitive comparison.
    fn calculate_keyword_score(&self, task_tokens: &[String], agent: &TreeAgent) -> usize {
        let agent_keywords: Vec<String> = agent
            .expertise_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        task_tokens
            .iter()
            .filter(|token| agent_keywords.contains(token))
            .count()
    }

    /// Routes by semantic similarity using embeddings
    ///
    /// Embeds the task and compares it with agent expertise embeddings
    /// using cosine similarity. Selects the agent with highest similarity
    /// above the configured threshold.
    async fn route_by_semantic_similarity(
        &self,
        grove: &Grove,
        task: &str,
    ) -> Result<RoutingDecision, BattalionError> {
        debug!("Routing by semantic similarity");

        // Check if embedding port is available
        let embedding_port = self.embedding_port.as_ref().ok_or_else(|| {
            BattalionError::RoutingError(
                "EmbeddingPort required for SemanticSimilarity routing".to_string(),
            )
        })?;

        // Embed the task
        let task_embedding = embedding_port
            .embed_text(task)
            .await
            .map_err(|e| BattalionError::RoutingError(format!("Failed to embed task: {}", e)))?;

        let threshold = grove.node.config.similarity_threshold;
        let mut best_similarity = threshold; // Must meet minimum threshold
        let mut best_tree: Option<&Tree> = None;
        let mut best_agent: Option<&TreeAgent> = None;

        // Calculate similarity with each agent
        for tree in &grove.node.trees {
            for agent in &tree.agents {
                if let Some(ref agent_embedding) = agent.expertise_embedding {
                    let similarity =
                        self.cosine_similarity(&task_embedding.vector, agent_embedding);

                    if similarity > best_similarity {
                        best_similarity = similarity;
                        best_tree = Some(tree);
                        best_agent = Some(agent);
                    }
                }
            }
        }

        // Check if we found a match above threshold
        if let (Some(tree), Some(agent)) = (best_tree, best_agent) {
            Ok(RoutingDecision {
                selected_tree: tree.name.clone(),
                selected_agent: agent.paladin_id.clone(),
                confidence: best_similarity,
                reasoning: format!(
                    "Semantic similarity score: {:.3} (threshold: {:.3})",
                    best_similarity, threshold
                ),
            })
        } else {
            Err(BattalionError::RoutingError(format!(
                "No agents found with similarity above threshold {:.3}",
                threshold
            )))
        }
    }

    /// Calculates cosine similarity between two vectors
    ///
    /// Returns a value between 0.0 and 1.0, where 1.0 means identical direction.
    fn cosine_similarity(&self, vec_a: &[f32], vec_b: &[f32]) -> f32 {
        if vec_a.len() != vec_b.len() {
            warn!(
                "Dimension mismatch in cosine similarity: {} vs {}",
                vec_a.len(),
                vec_b.len()
            );
            return 0.0;
        }

        let dot_product: f32 = vec_a.iter().zip(vec_b.iter()).map(|(a, b)| a * b).sum();

        let magnitude_a: f32 = vec_a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = vec_b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }

        (dot_product / (magnitude_a * magnitude_b)).clamp(0.0, 1.0)
    }

    /// Routes using LLM analysis
    ///
    /// Provides the LLM with task description and agent expertise,
    /// asking it to select the best agent with reasoning.
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::RoutingError` if:
    /// - LLM is not configured (None)
    /// - LLM call fails
    /// - JSON parsing fails
    /// - Confidence is below threshold and routing_fallback is "error"
    /// - Unknown agent_id in response
    async fn route_by_llm(
        &self,
        grove: &Grove,
        task: &str,
    ) -> Result<RoutingDecision, BattalionError> {
        debug!("Routing by LLM analysis");

        // Check if LLM port is available
        let llm_port = self.llm_port.as_ref().ok_or_else(|| {
            BattalionError::RoutingError(
                "LLM port not configured for LLM-based routing".to_string(),
            )
        })?;

        // Check that an operator-configured routing model is present (D-01/D-02). Delegates to
        // the single shared resolver (also called by `route_task`'s pre-dispatch check) so this
        // guard and the dispatch-layer check cannot drift apart. There is no fallback of any
        // kind here: this guard must not consult `routing_fallback`, must not call
        // `handle_routing_failure`, and must not query the LLM port for its available models —
        // a misconfigured Grove fails loudly rather than silently substituting a model the
        // operator did not choose.
        let routing_model = Self::resolve_routing_model(grove)?;

        // Build prompt with agent information
        let mut prompt = format!(
            "You are a task routing system. Given the following task and available specialized agents, \
            select the most appropriate agent and explain your reasoning.\n\n\
            Task: {}\n\n\
            Available Agents:\n",
            task
        );

        for tree in &grove.node.trees {
            prompt.push_str(&format!("\nTree: {}\n", tree.name));
            for agent in &tree.agents {
                prompt.push_str(&format!(
                    "  - Agent ID: {}\n    Expertise: {}\n",
                    agent.paladin_id,
                    agent.expertise_keywords.join(", ")
                ));
            }
        }

        prompt.push_str(
            "\n\nRespond in JSON format with the following structure:\n\
            {\n  \
              \"tree_name\": \"selected tree name\",\n  \
              \"agent_id\": \"selected agent ID\",\n  \
              \"confidence\": 0.85,\n  \
              \"reasoning\": \"explanation for selection\"\n\
            }\n",
        );

        debug!("LLM routing prompt prepared: {} characters", prompt.len());

        // Create prompt item
        let user_prompt = UserPrompt {
            query: prompt,
            context: None,
        };

        let prompt_item = PromptItem::new(PromptType::User(user_prompt))
            .map_err(|e| BattalionError::RoutingError(format!("Failed to create prompt: {}", e)))?;

        // Call LLM
        let llm_request = LlmRequest::new(routing_model.to_string(), prompt_item);

        let llm_response = llm_port.generate(llm_request).await.map_err(|e| {
            let msg = format!("LLM call failed: {}", e);
            warn!("{}", msg);
            e // Return the LLM error, will be handled below
        });

        let llm_response = match llm_response {
            Ok(resp) => resp,
            Err(_) => {
                return self.handle_routing_failure(grove, task, "LLM call failed");
            }
        };

        debug!(
            "LLM response received: {} characters",
            llm_response.content.len()
        );

        // Parse JSON response
        let routing_response: RoutingResponse = match serde_json::from_str(&llm_response.content) {
            Ok(resp) => resp,
            Err(e) => {
                warn!("Failed to parse LLM JSON response: {}", e);
                return self.handle_routing_failure(
                    grove,
                    task,
                    &format!("Failed to parse JSON: {}", e),
                );
            }
        };

        debug!(
            "Parsed routing response: agent={}, confidence={}",
            routing_response.agent_id, routing_response.confidence
        );

        // Validate confidence threshold
        if routing_response.confidence < grove.node.config.min_confidence {
            warn!(
                "Confidence {} below threshold {}",
                routing_response.confidence, grove.node.config.min_confidence
            );
            return self.handle_routing_failure(
                grove,
                task,
                &format!(
                    "Confidence {} below threshold {}",
                    routing_response.confidence, grove.node.config.min_confidence
                ),
            );
        }

        // Validate agent_id exists in Grove
        let agent_exists = grove.node.trees.iter().any(|tree| {
            tree.agents
                .iter()
                .any(|agent| agent.paladin_id == routing_response.agent_id)
        });

        if !agent_exists {
            warn!(
                "Unknown agent_id in LLM response: {}",
                routing_response.agent_id
            );
            return self.handle_routing_failure(
                grove,
                task,
                &format!("Unknown agent_id: {}", routing_response.agent_id),
            );
        }

        info!(
            "LLM routing successful: {} (confidence: {})",
            routing_response.agent_id, routing_response.confidence
        );

        Ok(RoutingDecision {
            selected_tree: routing_response.tree_name,
            selected_agent: routing_response.agent_id,
            confidence: routing_response.confidence,
            reasoning: routing_response.reasoning,
        })
    }

    /// Handles routing failures based on the configured fallback strategy
    ///
    /// If `routing_fallback` is "keyword", attempts keyword matching.
    /// If "error", returns an error.
    fn handle_routing_failure(
        &self,
        grove: &Grove,
        task: &str,
        reason: &str,
    ) -> Result<RoutingDecision, BattalionError> {
        match grove.node.config.routing_fallback.as_str() {
            "keyword" => {
                warn!("Falling back to keyword matching: {}", reason);
                self.route_by_keywords(grove, task).map(|mut decision| {
                    decision.reasoning = format!(
                        "LLM routing failed ({}), fell back to keyword matching: {}",
                        reason, decision.reasoning
                    );
                    decision
                })
            }
            "error" => Err(BattalionError::RoutingError(format!(
                "LLM routing failed: {}",
                reason
            ))),
            other => {
                warn!(
                    "Unknown routing_fallback value '{}', treating as 'error'",
                    other
                );
                Err(BattalionError::RoutingError(format!(
                    "LLM routing failed: {}",
                    reason
                )))
            }
        }
    }

    /// Selects the first agent from a tree
    ///
    /// Used for fallback scenarios when routing strategies fail.
    fn select_from_tree(
        &self,
        tree: &Tree,
        task: &str,
        reason: &str,
    ) -> Result<RoutingDecision, BattalionError> {
        tree.agents.first().map_or_else(
            || {
                Err(BattalionError::RoutingError(format!(
                    "Tree '{}' has no agents",
                    tree.name
                )))
            },
            |agent| {
                Ok(RoutingDecision {
                    selected_tree: tree.name.clone(),
                    selected_agent: agent.paladin_id.clone(),
                    confidence: 0.5,
                    reasoning: format!("Using {} strategy for task: {}", reason, task),
                })
            },
        )
    }

    /// Executes an agent with the given task
    ///
    /// Looks up the Paladin by agent_id (TreeAgent.paladin_id) and executes it.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent to execute (matches index in paladins vec, e.g., "agent_0")
    /// * `paladins` - Slice of available Paladins
    /// * `task` - Task input string
    ///
    /// # Returns
    ///
    /// The output string from the Paladin execution
    async fn execute_agent(
        &self,
        paladin: &paladin_core::platform::container::paladin::Paladin,
        task: &str,
    ) -> Result<String, BattalionError> {
        debug!(
            "Executing agent '{}' with task: {}",
            paladin.node.name, task
        );

        // Execute the Paladin
        let result = self
            .paladin_port
            .execute(paladin, task)
            .await
            .map_err(|e| {
                BattalionError::ExecutionError(format!(
                    "Paladin '{}' execution failed: {}",
                    paladin.node.name, e
                ))
            })?;

        Ok(result.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battalion::grove::{GroveBuilder, Tree, TreeAgent};
    use paladin_ports::output::embedding_port::Embedding;

    /// Mock embedding port for testing
    struct MockEmbeddingPort;

    #[async_trait::async_trait]
    impl EmbeddingPort for MockEmbeddingPort {
        async fn embed_text(
            &self,
            text: &str,
        ) -> Result<Embedding, paladin_ports::output::embedding_port::EmbeddingError> {
            // Simple mock: return embedding based on text length
            let vector = vec![text.len() as f32 / 100.0; 128];
            Ok(Embedding {
                vector,
                model: "mock-model".to_string(),
                dimension: 128,
                token_count: Some(text.split_whitespace().count() as u32),
            })
        }

        async fn embed_batch(
            &self,
            texts: &[&str],
        ) -> Result<Vec<Embedding>, paladin_ports::output::embedding_port::EmbeddingError> {
            let mut embeddings = Vec::new();
            for text in texts {
                embeddings.push(self.embed_text(text).await?);
            }
            Ok(embeddings)
        }

        fn dimension(&self) -> usize {
            128
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }
    }

    #[test]
    fn test_calculate_keyword_score() {
        let service = create_test_service();
        let agent =
            TreeAgent::new("test_agent").with_keywords(vec!["rust", "backend", "api", "database"]);

        let task_tokens = vec!["rust".to_string(), "api".to_string(), "testing".to_string()];

        let score = service.calculate_keyword_score(&task_tokens, &agent);
        assert_eq!(score, 2); // "rust" and "api" match
    }

    #[test]
    fn test_calculate_keyword_score_case_insensitive() {
        let service = create_test_service();
        let agent = TreeAgent::new("test_agent").with_keywords(vec!["Rust", "Backend", "API"]);

        let task_tokens = vec!["rust".to_string(), "api".to_string()];

        let score = service.calculate_keyword_score(&task_tokens, &agent);
        assert_eq!(score, 2);
    }

    #[test]
    fn test_calculate_keyword_score_no_match() {
        let service = create_test_service();
        let agent = TreeAgent::new("test_agent").with_keywords(vec!["python", "django"]);

        let task_tokens = vec!["rust".to_string(), "api".to_string()];

        let score = service.calculate_keyword_score(&task_tokens, &agent);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let service = create_test_service();
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![1.0, 2.0, 3.0];

        let similarity = service.cosine_similarity(&vec_a, &vec_b);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let service = create_test_service();
        let vec_a = vec![1.0, 0.0, 0.0];
        let vec_b = vec![0.0, 1.0, 0.0];

        let similarity = service.cosine_similarity(&vec_a, &vec_b);
        assert!((similarity - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let service = create_test_service();
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![-1.0, -2.0, -3.0];

        let similarity = service.cosine_similarity(&vec_a, &vec_b);
        // Cosine similarity is clamped to [0, 1], so opposite vectors give 0
        assert!((similarity - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let service = create_test_service();
        let vec_a = vec![1.0, 2.0];
        let vec_b = vec![1.0, 2.0, 3.0];

        let similarity = service.cosine_similarity(&vec_a, &vec_b);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_route_by_keywords_basic() {
        let service = create_test_service();
        let grove = create_test_grove();

        let result = service.route_by_keywords(&grove, "rust backend api development");

        assert!(result.is_ok());
        let decision = result.unwrap();
        assert_eq!(decision.selected_agent, "backend_expert");
        assert!(decision.confidence > 0.0);
    }

    #[test]
    fn test_route_by_keywords_no_match() {
        let service = create_test_service();
        let grove = create_test_grove();

        let result = service.route_by_keywords(&grove, "quantum physics simulation");

        // Should still route, but with low confidence
        assert!(result.is_ok());
        let decision = result.unwrap();
        assert!(decision.confidence <= 0.5);
    }

    #[tokio::test]
    async fn test_grove_resolves_routed_agent() {
        // Task 4.1: Grove resolves routed agent from registry
        use crate::in_memory_registry::HashMapPaladinRegistry;
        use paladin_core::base::entity::node::Node;
        use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
        use paladin_ports::output::paladin_port::PaladinResult;
        use paladin_ports::output::paladin_registry::PaladinRegistry;

        // Create test Paladins with names matching TreeAgent paladin_ids
        let backend_paladin = Node::new(
            PaladinData {
                system_prompt: "Backend expert".to_string(),
                name: "backend_expert".to_string(),
                user_name: "User".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_loops: MaxLoops::Fixed(3),
                stop_words: vec![],
                status: PaladinStatus::Idle,
                vision_enabled: false,
                ..Default::default()
            },
            Some("backend_expert".to_string()),
        );

        // Create registry and register paladins
        let registry = HashMapPaladinRegistry::new();
        registry
            .register("backend_expert".to_string(), Arc::new(backend_paladin))
            .expect("Should register backend_expert");

        // Create mock that returns a result
        struct ExecutingMockPort;
        #[async_trait::async_trait]
        impl PaladinPort for ExecutingMockPort {
            async fn execute(
                &self,
                paladin: &paladin_core::platform::container::paladin::Paladin,
                input: &str,
            ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError>
            {
                Ok(PaladinResult {
                    output: format!("[{}] Analyzed: {}", paladin.node.name, input),
                    token_count: 100,
                    execution_time_ms: 10,
                    loop_count: 1,
                    ..Default::default()
                })
            }

            async fn execute_stream(
                &self,
                _paladin: &paladin_core::platform::container::paladin::Paladin,
                _input: &str,
            ) -> Result<
                paladin_ports::output::paladin_port::PaladinStream,
                paladin_core::platform::container::paladin_error::PaladinError,
            > {
                let (_tx, rx) = tokio::sync::mpsc::channel(1);
                Ok(rx)
            }

            fn validate(
                &self,
                _paladin: &paladin_core::platform::container::paladin::Paladin,
            ) -> Result<(), paladin_core::platform::container::paladin_error::PaladinError>
            {
                Ok(())
            }
        }

        let service =
            GroveExecutionService::new(Arc::new(ExecutingMockPort), None, None, Arc::new(registry));

        let grove = create_test_grove();

        // Execute - should resolve "backend_expert" from registry
        let result = service.execute(&grove, "rust backend task").await;

        assert!(result.is_ok(), "Grove should resolve and execute agent");
        let grove_result = result.unwrap();
        assert_eq!(
            grove_result.routing_decision.selected_agent,
            "backend_expert"
        );
        assert!(grove_result.execution_result.contains("backend_expert"));
    }

    #[tokio::test]
    async fn test_grove_paladin_not_found_error() {
        // Task 4.2: Grove returns PaladinNotFound when agent missing from registry
        use crate::in_memory_registry::HashMapPaladinRegistry;

        // Create empty registry (no paladins registered)
        let registry = HashMapPaladinRegistry::new();

        let service =
            GroveExecutionService::new(Arc::new(MockPaladinPort), None, None, Arc::new(registry));

        let grove = create_test_grove();

        // Execute - routing will select "backend_expert" but it won't be in registry
        let result = service.execute(&grove, "rust backend task").await;

        assert!(result.is_err(), "Should return error when agent not found");
        match result {
            Err(BattalionError::PaladinNotFound(msg)) => {
                assert!(msg.contains("backend_expert"));
            }
            _ => panic!("Expected PaladinNotFound error"),
        }
    }

    // Helper functions
    fn create_test_service() -> GroveExecutionService {
        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();
        GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            Some(Arc::new(MockEmbeddingPort)),
            Some(Arc::new(MockLlmPort)),
            Arc::new(registry),
        )
    }

    fn create_test_grove() -> Grove {
        GroveBuilder::new()
            .name("Test Grove")
            .add_tree(
                Tree::new("engineering")
                    .add_agent(
                        TreeAgent::new("backend_expert")
                            .with_keywords(vec!["rust", "backend", "api", "database"]),
                    )
                    .add_agent(TreeAgent::new("frontend_expert").with_keywords(vec![
                        "react",
                        "ui",
                        "css",
                        "javascript",
                    ])),
            )
            .build()
            .unwrap()
    }

    // Mock ports for testing
    struct MockPaladinPort;
    struct MockLlmPort;

    #[async_trait::async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            _paladin: &paladin_core::platform::container::paladin::Paladin,
            _input: &str,
        ) -> Result<
            paladin_ports::output::paladin_port::PaladinResult,
            paladin_core::platform::container::paladin_error::PaladinError,
        > {
            unimplemented!("Mock not needed for these tests")
        }

        async fn execute_stream(
            &self,
            _paladin: &paladin_core::platform::container::paladin::Paladin,
            _input: &str,
        ) -> Result<
            paladin_ports::output::paladin_port::PaladinStream,
            paladin_core::platform::container::paladin_error::PaladinError,
        > {
            unimplemented!("Mock not needed for these tests")
        }

        fn validate(
            &self,
            _paladin: &paladin_core::platform::container::paladin::Paladin,
        ) -> Result<(), paladin_core::platform::container::paladin_error::PaladinError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(
            &self,
            _request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            paladin_ports::output::llm_port::LlmResponse,
            paladin_ports::output::llm_port::LlmError,
        > {
            unimplemented!("Mock not needed for these tests")
        }

        async fn generate_stream(
            &self,
            _request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            Box<
                dyn futures::Stream<
                        Item = Result<
                            paladin_ports::output::llm_port::StreamingResponse,
                            paladin_ports::output::llm_port::LlmError,
                        >,
                    > + Send,
            >,
            paladin_ports::output::llm_port::LlmError,
        > {
            unimplemented!("Mock not needed for these tests")
        }

        async fn validate_model(
            &self,
            _model: &str,
        ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
            Ok(true)
        }

        async fn get_available_models(
            &self,
        ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
            Ok(vec!["mock-model".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock"
        }

        fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
            use paladin_ports::output::llm_port::ProviderCapabilities;
            ProviderCapabilities {
                supports_streaming: false,
                supports_tool_calling: false,
                supports_function_calling: false,
                supports_vision: false,
                supports_embeddings: false,
                max_context_tokens: Some(4096),
                supports_system_messages: true,
                temperature_range: None,
            }
        }
    }

    // Task 5.0: LLM-based routing tests (TDD - these should fail until implementation)

    #[tokio::test]
    async fn test_route_with_llm_successful() {
        // Mock LLM that returns valid high-confidence routing decision
        struct SuccessfulLlmMock;

        #[async_trait::async_trait]
        impl LlmPort for SuccessfulLlmMock {
            async fn generate(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                paladin_ports::output::llm_port::LlmResponse,
                paladin_ports::output::llm_port::LlmError,
            > {
                let response_json = r#"{
                    "tree_name": "engineering",
                    "agent_id": "backend_expert",
                    "confidence": 0.85,
                    "reasoning": "Task mentions rust and backend, which are backend expert's core skills"
                }"#;

                Ok(paladin_ports::output::llm_port::LlmResponse {
                    id: uuid::Uuid::new_v4(),
                    request_id: uuid::Uuid::new_v4(),
                    model: "mock-model".to_string(),
                    content: response_json.to_string(),
                    finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                    usage: paladin_ports::output::llm_port::TokenUsage {
                        prompt_tokens: 100,
                        completion_tokens: 50,
                        total_tokens: 150,
                    },
                    created_at: chrono::Utc::now(),
                    metadata: std::collections::HashMap::new(),
                    function_call: None,
                })
            }

            async fn generate_stream(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                Box<
                    dyn futures::Stream<
                            Item = Result<
                                paladin_ports::output::llm_port::StreamingResponse,
                                paladin_ports::output::llm_port::LlmError,
                            >,
                        > + Send,
                >,
                paladin_ports::output::llm_port::LlmError,
            > {
                unimplemented!()
            }

            async fn validate_model(
                &self,
                _model: &str,
            ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
                Ok(true)
            }

            async fn get_available_models(
                &self,
            ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
                Ok(vec!["mock-model".to_string()])
            }

            fn get_provider_name(&self) -> &'static str {
                "mock"
            }

            fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
                paladin_ports::output::llm_port::ProviderCapabilities::default()
            }
        }

        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();
        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(Arc::new(SuccessfulLlmMock)),
            Arc::new(registry),
        );

        let mut grove = create_test_grove();
        grove.node.config.routing_model = Some("mock-model".to_string());

        let result = service
            .route_by_llm(&grove, "rust backend development task")
            .await;

        assert!(
            result.is_ok(),
            "Should route successfully with high confidence"
        );
        let decision = result.unwrap();
        assert_eq!(decision.selected_tree, "engineering");
        assert_eq!(decision.selected_agent, "backend_expert");
        assert!(decision.confidence >= 0.85);
        assert!(decision.reasoning.contains("rust"));
    }

    #[tokio::test]
    async fn test_route_with_llm_low_confidence() {
        // Mock LLM that returns valid but low-confidence routing decision
        struct LowConfidenceLlmMock;

        #[async_trait::async_trait]
        impl LlmPort for LowConfidenceLlmMock {
            async fn generate(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                paladin_ports::output::llm_port::LlmResponse,
                paladin_ports::output::llm_port::LlmError,
            > {
                let response_json = r#"{
                    "tree_name": "engineering",
                    "agent_id": "backend_expert",
                    "confidence": 0.3,
                    "reasoning": "Unclear task, best guess is backend"
                }"#;

                Ok(paladin_ports::output::llm_port::LlmResponse {
                    id: uuid::Uuid::new_v4(),
                    request_id: uuid::Uuid::new_v4(),
                    model: "mock-model".to_string(),
                    content: response_json.to_string(),
                    finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                    usage: paladin_ports::output::llm_port::TokenUsage {
                        prompt_tokens: 100,
                        completion_tokens: 50,
                        total_tokens: 150,
                    },
                    created_at: chrono::Utc::now(),
                    metadata: std::collections::HashMap::new(),
                    function_call: None,
                })
            }

            async fn generate_stream(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                Box<
                    dyn futures::Stream<
                            Item = Result<
                                paladin_ports::output::llm_port::StreamingResponse,
                                paladin_ports::output::llm_port::LlmError,
                            >,
                        > + Send,
                >,
                paladin_ports::output::llm_port::LlmError,
            > {
                unimplemented!()
            }

            async fn validate_model(
                &self,
                _model: &str,
            ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
                Ok(true)
            }

            async fn get_available_models(
                &self,
            ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
                Ok(vec!["mock-model".to_string()])
            }

            fn get_provider_name(&self) -> &'static str {
                "mock"
            }

            fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
                paladin_ports::output::llm_port::ProviderCapabilities::default()
            }
        }

        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();
        let mut grove = create_test_grove();
        // Set routing_fallback to "error" to test that low confidence triggers fallback logic
        grove.node.config.routing_fallback = "error".to_string();
        grove.node.config.min_confidence = 0.5;
        grove.node.config.routing_model = Some("mock-model".to_string());

        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(Arc::new(LowConfidenceLlmMock)),
            Arc::new(registry),
        );

        let result = service.route_by_llm(&grove, "ambiguous task").await;

        // With routing_fallback="error" and confidence below threshold, should return error
        assert!(
            result.is_err(),
            "Should return error when confidence below threshold and fallback is 'error'"
        );
        match result {
            Err(BattalionError::RoutingError(msg)) => {
                assert!(msg.contains("confidence") || msg.contains("threshold"));
            }
            _ => panic!("Expected RoutingError for low confidence with error fallback"),
        }
    }

    #[tokio::test]
    async fn test_route_with_llm_invalid_json() {
        // Mock LLM that returns invalid JSON
        struct InvalidJsonLlmMock;

        #[async_trait::async_trait]
        impl LlmPort for InvalidJsonLlmMock {
            async fn generate(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                paladin_ports::output::llm_port::LlmResponse,
                paladin_ports::output::llm_port::LlmError,
            > {
                let response_json = "This is not JSON at all!";

                Ok(paladin_ports::output::llm_port::LlmResponse {
                    id: uuid::Uuid::new_v4(),
                    request_id: uuid::Uuid::new_v4(),
                    model: "mock-model".to_string(),
                    content: response_json.to_string(),
                    finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                    usage: paladin_ports::output::llm_port::TokenUsage {
                        prompt_tokens: 100,
                        completion_tokens: 50,
                        total_tokens: 150,
                    },
                    created_at: chrono::Utc::now(),
                    metadata: std::collections::HashMap::new(),
                    function_call: None,
                })
            }

            async fn generate_stream(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                Box<
                    dyn futures::Stream<
                            Item = Result<
                                paladin_ports::output::llm_port::StreamingResponse,
                                paladin_ports::output::llm_port::LlmError,
                            >,
                        > + Send,
                >,
                paladin_ports::output::llm_port::LlmError,
            > {
                unimplemented!()
            }

            async fn validate_model(
                &self,
                _model: &str,
            ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
                Ok(true)
            }

            async fn get_available_models(
                &self,
            ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
                Ok(vec!["mock-model".to_string()])
            }

            fn get_provider_name(&self) -> &'static str {
                "mock"
            }

            fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
                paladin_ports::output::llm_port::ProviderCapabilities::default()
            }
        }

        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();
        let mut grove = create_test_grove();
        grove.node.config.routing_fallback = "error".to_string();
        grove.node.config.routing_model = Some("mock-model".to_string());

        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(Arc::new(InvalidJsonLlmMock)),
            Arc::new(registry),
        );

        let result = service.route_by_llm(&grove, "test task").await;

        // Invalid JSON should trigger error when routing_fallback="error"
        assert!(result.is_err(), "Should return error for invalid JSON");
        match result {
            Err(BattalionError::RoutingError(_)) => {} // Expected
            _ => panic!("Expected RoutingError for invalid JSON"),
        }
    }

    #[tokio::test]
    async fn test_route_with_llm_fallback_to_keyword() {
        // Mock LLM that returns low confidence
        struct LowConfidenceFallbackMock;

        #[async_trait::async_trait]
        impl LlmPort for LowConfidenceFallbackMock {
            async fn generate(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                paladin_ports::output::llm_port::LlmResponse,
                paladin_ports::output::llm_port::LlmError,
            > {
                let response_json = r#"{
                    "tree_name": "engineering",
                    "agent_id": "backend_expert",
                    "confidence": 0.2,
                    "reasoning": "Very uncertain"
                }"#;

                Ok(paladin_ports::output::llm_port::LlmResponse {
                    id: uuid::Uuid::new_v4(),
                    request_id: uuid::Uuid::new_v4(),
                    model: "mock-model".to_string(),
                    content: response_json.to_string(),
                    finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                    usage: paladin_ports::output::llm_port::TokenUsage {
                        prompt_tokens: 100,
                        completion_tokens: 50,
                        total_tokens: 150,
                    },
                    created_at: chrono::Utc::now(),
                    metadata: std::collections::HashMap::new(),
                    function_call: None,
                })
            }

            async fn generate_stream(
                &self,
                _request: paladin_ports::output::llm_port::LlmRequest,
            ) -> Result<
                Box<
                    dyn futures::Stream<
                            Item = Result<
                                paladin_ports::output::llm_port::StreamingResponse,
                                paladin_ports::output::llm_port::LlmError,
                            >,
                        > + Send,
                >,
                paladin_ports::output::llm_port::LlmError,
            > {
                unimplemented!()
            }

            async fn validate_model(
                &self,
                _model: &str,
            ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
                Ok(true)
            }

            async fn get_available_models(
                &self,
            ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
                Ok(vec!["mock-model".to_string()])
            }

            fn get_provider_name(&self) -> &'static str {
                "mock"
            }

            fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
                paladin_ports::output::llm_port::ProviderCapabilities::default()
            }
        }

        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();
        let mut grove = create_test_grove();
        // Set routing_fallback to "keyword" to test fallback to keyword matching
        grove.node.config.routing_fallback = "keyword".to_string();
        grove.node.config.min_confidence = 0.5;
        grove.node.config.routing_model = Some("mock-model".to_string());

        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(Arc::new(LowConfidenceFallbackMock)),
            Arc::new(registry),
        );

        let result = service
            .route_by_llm(&grove, "rust backend development")
            .await;

        // With routing_fallback="keyword", should fallback and succeed
        assert!(
            result.is_ok(),
            "Should fallback to keyword matching when confidence below threshold"
        );
        let decision = result.unwrap();
        // Should have been routed by keyword matching (backend_expert has "rust" keyword)
        assert_eq!(decision.selected_agent, "backend_expert");
        // The reasoning should indicate fallback occurred
        assert!(decision.reasoning.contains("keyword") || decision.reasoning.contains("fallback"));
    }

    // CLOSE-01 (D-01/D-02/D-04): Grove LLM routing must use the operator-configured
    // `routing_model` rather than a hardcoded literal, and must hard-error with no fallback
    // of any kind when it is absent.

    /// Recording mock `LlmPort` that captures every `LlmRequest.model` it is handed.
    ///
    /// Used to prove Grove LLM routing sources its model from `GroveConfig.routing_model`
    /// rather than a hardcoded provider literal (D-04). Extends the established in-file mock
    /// convention (see `SuccessfulLlmMock` above) rather than introducing a parallel harness.
    struct RecordingLlmMock {
        recorded_models: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl RecordingLlmMock {
        fn new() -> Self {
            Self {
                recorded_models: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        /// Returns every `LlmRequest.model` this mock has recorded, in call order.
        fn recorded_models(&self) -> Vec<String> {
            self.recorded_models
                .lock()
                .expect("recorded_models mutex poisoned")
                .clone()
        }
    }

    #[async_trait::async_trait]
    impl LlmPort for RecordingLlmMock {
        async fn generate(
            &self,
            request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            paladin_ports::output::llm_port::LlmResponse,
            paladin_ports::output::llm_port::LlmError,
        > {
            self.recorded_models
                .lock()
                .expect("recorded_models mutex poisoned")
                .push(request.model.clone());

            let response_json = r#"{
                "tree_name": "engineering",
                "agent_id": "backend_expert",
                "confidence": 0.85,
                "reasoning": "Task mentions rust and backend, which are backend expert's core skills"
            }"#;

            Ok(paladin_ports::output::llm_port::LlmResponse {
                id: uuid::Uuid::new_v4(),
                request_id: uuid::Uuid::new_v4(),
                model: request.model,
                content: response_json.to_string(),
                finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                usage: paladin_ports::output::llm_port::TokenUsage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                    total_tokens: 150,
                },
                created_at: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
                function_call: None,
            })
        }

        async fn generate_stream(
            &self,
            _request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            Box<
                dyn futures::Stream<
                        Item = Result<
                            paladin_ports::output::llm_port::StreamingResponse,
                            paladin_ports::output::llm_port::LlmError,
                        >,
                    > + Send,
            >,
            paladin_ports::output::llm_port::LlmError,
        > {
            unimplemented!("Mock not needed for these tests")
        }

        async fn validate_model(
            &self,
            _model: &str,
        ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
            Ok(true)
        }

        async fn get_available_models(
            &self,
        ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
            Ok(vec!["mock-model".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock"
        }

        fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
            paladin_ports::output::llm_port::ProviderCapabilities::default()
        }
    }

    #[tokio::test]
    async fn test_llm_routing_uses_configured_routing_model() {
        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();

        let mut grove = create_test_grove();
        grove.node.config.routing_strategy = RoutingStrategy::LlmRouting;
        grove.node.config.routing_model = Some("deepseek-chat".to_string());

        let mock = Arc::new(RecordingLlmMock::new());
        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(mock.clone()),
            Arc::new(registry),
        );

        let result = service
            .route_by_llm(&grove, "rust backend development task")
            .await;

        assert!(
            result.is_ok(),
            "Expected successful routing, got {:?}",
            result
        );
        assert_eq!(mock.recorded_models(), vec!["deepseek-chat".to_string()]);
    }

    #[tokio::test]
    async fn test_llm_routing_errors_when_routing_model_absent() {
        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();

        let mut grove = create_test_grove();
        grove.node.config.routing_strategy = RoutingStrategy::LlmRouting;
        // routing_model left unset (default None)

        let mock = Arc::new(RecordingLlmMock::new());
        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(mock.clone()),
            Arc::new(registry),
        );

        let result = service.route_by_llm(&grove, "any task").await;

        match result {
            Err(BattalionError::RoutingError(msg)) => {
                assert!(
                    msg.contains("routing_model"),
                    "error message should name routing_model, got: {}",
                    msg
                );
            }
            other => panic!(
                "Expected RoutingError naming routing_model, got {:?}",
                other
            ),
        }
        assert!(
            mock.recorded_models().is_empty(),
            "no LLM call should have been made"
        );
    }

    // CLOSE-01 edge rows: empty, adjacency, concurrency (04-CONTEXT.md D-02, <specifics>).

    #[tokio::test]
    async fn test_llm_routing_errors_when_routing_model_empty() {
        use crate::in_memory_registry::HashMapPaladinRegistry;

        for blank in ["", "   "] {
            let registry = HashMapPaladinRegistry::new();
            let mut grove = create_test_grove();
            grove.node.config.routing_strategy = RoutingStrategy::LlmRouting;
            grove.node.config.routing_model = Some(blank.to_string());

            let mock = Arc::new(RecordingLlmMock::new());
            let service = GroveExecutionService::new(
                Arc::new(MockPaladinPort),
                None,
                Some(mock.clone()),
                Arc::new(registry),
            );

            let result = service.route_by_llm(&grove, "any task").await;

            match result {
                Err(BattalionError::RoutingError(msg)) => {
                    assert!(
                        msg.contains("routing_model"),
                        "error message should name routing_model for input {:?}, got: {}",
                        blank,
                        msg
                    );
                }
                other => panic!(
                    "Expected RoutingError naming routing_model for input {:?}, got {:?}",
                    blank, other
                ),
            }
            assert!(
                mock.recorded_models().is_empty(),
                "no LLM call should have been made for input {:?}",
                blank
            );
        }
    }

    #[tokio::test]
    async fn test_llm_routing_missing_model_error_precedes_keyword_fallback() {
        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();

        // create_test_grove()'s "backend_expert" agent carries keywords ("rust", "backend",
        // "api", "database") that would match this task under keyword routing, so a
        // successful keyword-fallback result would be a false pass for this test.
        let mut grove = create_test_grove();
        grove.node.config.routing_strategy = RoutingStrategy::LlmRouting;
        grove.node.config.routing_fallback = "keyword".to_string();
        // routing_model left unset (default None)

        let mock = Arc::new(RecordingLlmMock::new());
        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(mock.clone()),
            Arc::new(registry),
        );

        let result = service
            .route_by_llm(&grove, "rust backend api development")
            .await;

        match result {
            Err(BattalionError::RoutingError(msg)) => {
                assert!(
                    msg.contains("routing_model"),
                    "error should name routing_model, not a keyword-fallback failure, got: {}",
                    msg
                );
            }
            other => panic!(
                "Expected the missing-routing_model RoutingError to precede keyword fallback, got: {:?}",
                other
            ),
        }
        assert!(
            mock.recorded_models().is_empty(),
            "no LLM call should have been made"
        );
    }

    #[tokio::test]
    async fn test_concurrent_groves_use_their_own_routing_model() {
        use crate::in_memory_registry::HashMapPaladinRegistry;

        let mock = Arc::new(RecordingLlmMock::new());

        let mut grove_a = create_test_grove();
        grove_a.node.config.routing_strategy = RoutingStrategy::LlmRouting;
        grove_a.node.config.routing_model = Some("deepseek-chat".to_string());

        let mut grove_b = create_test_grove();
        grove_b.node.config.routing_strategy = RoutingStrategy::LlmRouting;
        grove_b.node.config.routing_model = Some("claude-3-5-sonnet-20241022".to_string());

        let service_a = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(mock.clone()),
            Arc::new(HashMapPaladinRegistry::new()),
        );
        let service_b = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(mock.clone()),
            Arc::new(HashMapPaladinRegistry::new()),
        );

        let (result_a, result_b) = tokio::join!(
            service_a.route_by_llm(&grove_a, "rust backend development task"),
            service_b.route_by_llm(&grove_b, "rust backend development task")
        );

        assert!(result_a.is_ok(), "Grove A should route successfully");
        assert!(result_b.is_ok(), "Grove B should route successfully");

        let mut recorded = mock.recorded_models();
        recorded.sort();
        assert_eq!(
            recorded,
            vec![
                "claude-3-5-sonnet-20241022".to_string(),
                "deepseek-chat".to_string(),
            ]
        );
    }

    // Task 2 (06-08-PLAN.md): prove the D-02 no-fallback guarantee at the execute() level —
    // the public entry point — rather than only at route_by_llm. `MockPaladinPort`'s
    // `unimplemented!` body is safe in every test below precisely because a passing test never
    // reaches Paladin execution: if any of these tests ever panics inside the mock instead of
    // returning the expected `Err`, that panic is itself the regression signal that the routing
    // error was swallowed again by route_task's fallback arm.

    #[tokio::test]
    async fn test_execute_errors_when_routing_model_absent() {
        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();

        let mut grove = create_test_grove();
        grove.node.config.routing_strategy = RoutingStrategy::LlmRouting;
        // routing_model left unset (default None)

        let mock = Arc::new(RecordingLlmMock::new());
        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(mock.clone()),
            Arc::new(registry),
        );

        let result = service.execute(&grove, "any task").await;

        match result {
            Err(BattalionError::RoutingError(msg)) => {
                assert!(
                    msg.contains("routing_model"),
                    "error message should name routing_model, got: {}",
                    msg
                );
            }
            other => panic!(
                "Expected RoutingError naming routing_model, got {:?}",
                other
            ),
        }
        assert!(
            mock.recorded_models().is_empty(),
            "no LLM call should have been made"
        );
    }

    #[tokio::test]
    async fn test_execute_errors_when_routing_model_blank() {
        use crate::in_memory_registry::HashMapPaladinRegistry;

        for blank in ["", "   "] {
            let registry = HashMapPaladinRegistry::new();
            let mut grove = create_test_grove();
            grove.node.config.routing_strategy = RoutingStrategy::LlmRouting;
            grove.node.config.routing_model = Some(blank.to_string());

            let mock = Arc::new(RecordingLlmMock::new());
            let service = GroveExecutionService::new(
                Arc::new(MockPaladinPort),
                None,
                Some(mock.clone()),
                Arc::new(registry),
            );

            let result = service.execute(&grove, "any task").await;

            match result {
                Err(BattalionError::RoutingError(msg)) => {
                    assert!(
                        msg.contains("routing_model"),
                        "error message should name routing_model for input {:?}, got: {}",
                        blank,
                        msg
                    );
                }
                other => panic!(
                    "Expected RoutingError naming routing_model for input {:?}, got {:?}",
                    blank, other
                ),
            }
            assert!(
                mock.recorded_models().is_empty(),
                "no LLM call should have been made for input {:?}",
                blank
            );
        }
    }

    #[tokio::test]
    async fn test_execute_errors_despite_fallback_tree_when_routing_model_absent() {
        use crate::in_memory_registry::HashMapPaladinRegistry;
        let registry = HashMapPaladinRegistry::new();

        // create_test_grove() builds exactly one tree, named "engineering" — read from the
        // source rather than assumed, so this test proves a *configured and resolvable*
        // fallback tree is declined, not one that was never found.
        let mut grove = create_test_grove();
        grove.node.config.routing_strategy = RoutingStrategy::LlmRouting;
        grove.node.config.fallback_tree = Some("engineering".to_string());
        // routing_model left unset (default None)

        let mock = Arc::new(RecordingLlmMock::new());
        let service = GroveExecutionService::new(
            Arc::new(MockPaladinPort),
            None,
            Some(mock.clone()),
            Arc::new(registry),
        );

        let result = service.execute(&grove, "any task").await;

        match result {
            Err(BattalionError::RoutingError(msg)) => {
                assert!(
                    msg.contains("routing_model"),
                    "error message should name routing_model despite a configured, resolvable \
                     fallback_tree, got: {}",
                    msg
                );
            }
            other => panic!(
                "Expected RoutingError naming routing_model (fallback_tree must not be \
                 consulted for this configuration error), got {:?}",
                other
            ),
        }
        assert!(
            mock.recorded_models().is_empty(),
            "no LLM call should have been made"
        );
    }
}
