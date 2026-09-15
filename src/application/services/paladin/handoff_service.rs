//! HandoffService - Agent Handoff Infrastructure
//!
//! Provides the core infrastructure for delegating tasks between Paladin agents.
//! Supports multiple handoff strategies (Automatic, Explicit, Threshold) and includes
//! safeguards against circular delegation and excessive depth.

use crate::application::errors::handoff_error::HandoffError;
use crate::core::platform::container::autonomous_config::HandoffConfig;
use crate::core::platform::container::handoff::{HandoffContext, HandoffRecord, HandoffStrategy};
use crate::core::platform::container::paladin::Paladin;
use log::{debug, info, warn};
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Service for managing agent handoffs and delegation
///
/// # Features
/// - Multiple handoff strategies (Automatic, Explicit, Threshold-based)
/// - Agent selection based on capabilities
/// - Circular delegation prevention
/// - Max depth enforcement
/// - Context transfer between agents
///
/// # Examples
/// ```
/// use paladin::application::services::paladin::handoff_service::HandoffService;
/// use paladin::core::platform::container::autonomous_config::HandoffConfig;
/// use paladin::core::platform::container::handoff::HandoffStrategy;
/// use std::sync::Arc;
///
/// let config = Arc::new(HandoffConfig {
///     enabled: true,
///     strategy: HandoffStrategy::Automatic,
///     max_depth: 5,
///     retry: Default::default(),
/// });
///
/// let service = HandoffService::new(config);
/// assert!(service.is_some());
/// ```
pub struct HandoffService {
    config: Arc<HandoffConfig>,
}

impl HandoffService {
    /// Creates a new HandoffService with the given configuration
    ///
    /// Returns `None` if handoffs are disabled in the configuration.
    ///
    /// # Arguments
    /// * `config` - Handoff configuration settings
    ///
    /// # Returns
    /// * `Some(HandoffService)` if enabled
    /// * `None` if disabled
    pub fn new(config: Arc<HandoffConfig>) -> Option<Self> {
        if !config.enabled {
            return None;
        }

        Some(Self { config })
    }

    /// Returns the handoff strategy being used
    pub fn strategy(&self) -> &HandoffStrategy {
        &self.config.strategy
    }

    /// Returns the maximum handoff depth allowed
    pub fn max_depth(&self) -> u32 {
        self.config.max_depth
    }

    /// Determines whether a task should be handed off to another agent
    ///
    /// The decision is based on the configured strategy:
    /// - **Automatic**: Hands off if confidence < 0.5 or task appears complex
    /// - **Explicit**: Never hands off automatically (requires explicit call)
    /// - **Threshold**: Hands off if confidence is below the configured threshold
    ///
    /// Always returns `false` if the current depth >= max_depth to prevent infinite chains.
    ///
    /// # Arguments
    /// * `task` - The task description being considered for handoff
    /// * `confidence` - Agent's confidence in handling the task (0.0-1.0)
    /// * `context` - Current handoff context including depth and chain
    ///
    /// # Returns
    /// * `true` if the task should be handed off
    /// * `false` if the current agent should handle it
    ///
    /// # Example
    /// ```
    /// use paladin::application::services::paladin::handoff_service::HandoffService;
    /// use paladin::core::platform::container::autonomous_config::HandoffConfig;
    /// use paladin::core::platform::container::handoff::{HandoffStrategy, HandoffContext};
    /// use std::sync::Arc;
    ///
    /// let config = Arc::new(HandoffConfig {
    ///     enabled: true,
    ///     strategy: HandoffStrategy::threshold(0.7),
    ///     max_depth: 3,
    ///     retry: Default::default(),
    /// });
    /// let service = HandoffService::new(config).unwrap();
    /// let context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    ///
    /// // Low confidence - should handoff
    /// assert!(service.should_handoff("Complex task", 0.6, &context));
    ///
    /// // High confidence - no handoff needed
    /// assert!(!service.should_handoff("Simple task", 0.8, &context));
    /// ```
    pub fn should_handoff(&self, task: &str, confidence: f32, context: &HandoffContext) -> bool {
        // Never handoff if at or beyond max depth
        if context.depth >= self.config.max_depth {
            debug!(
                "Max handoff depth reached (depth={}, max={}), executing locally",
                context.depth, self.config.max_depth
            );
            return false;
        }

        // Decision based on strategy
        match &self.config.strategy {
            HandoffStrategy::Automatic => {
                // Automatic mode: handoff if confidence is low or task appears complex
                if confidence < 0.5 {
                    info!(
                        "Low confidence ({:.2}), considering handoff for task: {}",
                        confidence, task
                    );
                    true
                } else {
                    // Additional heuristic: check if task seems complex
                    let is_complex = self.is_complex_task(task);
                    if is_complex && confidence < 0.8 {
                        info!(
                            "Complex task with moderate confidence ({:.2}), considering handoff: {}",
                            confidence, task
                        );
                        true
                    } else {
                        false
                    }
                }
            }
            HandoffStrategy::Explicit => {
                // Explicit mode: never auto-handoff
                debug!("Explicit strategy: no automatic handoffs");
                false
            }
            HandoffStrategy::Threshold { .. } => {
                // Threshold mode: handoff if confidence below threshold
                if let Some(threshold) = self.config.strategy.get_threshold() {
                    let should_handoff = confidence < threshold;
                    if should_handoff {
                        info!(
                            "Confidence ({:.2}) below threshold ({:.2}), handing off: {}",
                            confidence, threshold, task
                        );
                    }
                    should_handoff
                } else {
                    false
                }
            }
        }
    }

    /// Simple heuristic to detect complex tasks
    ///
    /// Looks for keywords indicating complexity like "implement", "design",
    /// "architecture", "distributed", "algorithm", etc.
    fn is_complex_task(&self, task: &str) -> bool {
        let task_lower = task.to_lowercase();
        let complexity_keywords = [
            "implement",
            "design",
            "architecture",
            "distributed",
            "algorithm",
            "optimize",
            "refactor",
            "migrate",
            "integrate",
            "system",
        ];

        complexity_keywords
            .iter()
            .any(|keyword| task_lower.contains(keyword))
    }

    /// Selects the most appropriate agent for a task from available specialists
    ///
    /// Uses keyword matching to find agents whose descriptions best match
    /// the task requirements. Returns the agent with the highest relevance score.
    ///
    /// # Arguments
    /// * `task` - The task description to match against agent capabilities
    /// * `available_agents` - List of (agent_name, agent_description) tuples
    ///
    /// # Returns
    /// * `Some(agent_name)` if a suitable agent is found
    /// * `None` if no agents are available or no good match exists
    ///
    /// # Example
    /// ```
    /// use paladin::application::services::paladin::handoff_service::HandoffService;
    /// use paladin::core::platform::container::autonomous_config::HandoffConfig;
    /// use paladin::core::platform::container::handoff::HandoffStrategy;
    /// use std::sync::Arc;
    ///
    /// let config = Arc::new(HandoffConfig {
    ///     enabled: true,
    ///     strategy: HandoffStrategy::Automatic,
    ///     max_depth: 3,
    ///     retry: Default::default(),
    /// });
    /// let service = HandoffService::new(config).unwrap();
    ///
    /// let agents = vec![
    ///     ("CodeExpert".to_string(), "Expert in Rust and Python".to_string()),
    ///     ("DataAnalyst".to_string(), "Data analysis specialist".to_string()),
    /// ];
    ///
    /// let selected = service.select_agent("Debug this Rust function", &agents);
    /// assert_eq!(selected, Some("CodeExpert".to_string()));
    /// ```
    pub fn select_agent(
        &self,
        task: &str,
        available_agents: &[(String, String)],
    ) -> Option<String> {
        if available_agents.is_empty() {
            debug!("No available agents for handoff");
            return None;
        }

        let task_lower = task.to_lowercase();
        let task_words: Vec<&str> = task_lower.split_whitespace().collect();

        // Calculate relevance score for each agent
        let mut scored_agents: Vec<(&String, &String, usize)> = available_agents
            .iter()
            .map(|(name, desc)| {
                let desc_lower = desc.to_lowercase();
                let score = self.calculate_relevance_score(&task_words, &desc_lower);
                (name, desc, score)
            })
            .collect();

        // Sort by score (descending)
        scored_agents.sort_by_key(|b| std::cmp::Reverse(b.2));

        // Return the best match if it has a meaningful score
        if let Some((name, desc, score)) = scored_agents.first().filter(|(_, _, s)| *s > 0) {
            info!(
                "Selected agent '{}' with score {} for task: {}",
                name, score, task
            );
            debug!("Agent description: {}", desc);
            return Some((*name).clone());
        }

        // No good match found
        debug!("No suitable specialist found for task: {}", task);
        None
    }

    /// Calculates relevance score between task and agent description
    ///
    /// Counts how many task words appear in the agent description.
    /// Gives higher weight to longer matching words.
    fn calculate_relevance_score(&self, task_words: &[&str], agent_desc: &str) -> usize {
        task_words
            .iter()
            .filter(|word| word.len() > 3) // Ignore short words like "the", "and"
            .filter(|word| agent_desc.contains(*word))
            .map(|word| word.len()) // Longer words count more
            .sum()
    }

    /// Validates whether a handoff to the target agent is allowed
    ///
    /// Checks for:
    /// - Maximum depth not exceeded
    /// - No circular delegation (target not already in chain)
    ///
    /// # Arguments
    /// * `target_agent` - Name of the agent to potentially handoff to
    /// * `context` - Current handoff context
    ///
    /// # Returns
    /// * `Ok(())` if handoff is allowed
    /// * `Err(HandoffError)` if handoff should be prevented
    ///
    /// # Example
    /// ```
    /// use paladin::application::services::paladin::handoff_service::HandoffService;
    /// use paladin::core::platform::container::autonomous_config::HandoffConfig;
    /// use paladin::core::platform::container::handoff::{HandoffStrategy, HandoffContext};
    /// use std::sync::Arc;
    ///
    /// let config = Arc::new(HandoffConfig {
    ///     enabled: true,
    ///     strategy: HandoffStrategy::Automatic,
    ///     max_depth: 3,
    ///     retry: Default::default(),
    /// });
    /// let service = HandoffService::new(config).unwrap();
    /// let context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    ///
    /// // Valid handoff to new agent
    /// assert!(service.validate_handoff("Agent2", &context).is_ok());
    /// ```
    pub fn validate_handoff(
        &self,
        target_agent: &str,
        context: &HandoffContext,
    ) -> Result<(), HandoffError> {
        // Check max depth
        if context.depth >= self.config.max_depth {
            return Err(HandoffError::MaxDepthExceeded {
                current: context.depth,
                max: self.config.max_depth,
            });
        }

        // Check for circular delegation
        if context.chain.iter().any(|agent| agent == target_agent) {
            return Err(HandoffError::CircularHandoff {
                agent_name: target_agent.to_string(),
                chain: context.chain.join(" -> "),
            });
        }

        Ok(())
    }

    /// Checks if handoff to a specific agent is allowed
    ///
    /// Returns `true` if the agent is not in the current chain and depth allows.
    ///
    /// # Arguments
    /// * `target_agent` - Name of the agent to check
    /// * `context` - Current handoff context
    ///
    /// # Returns
    /// * `true` if handoff is allowed
    /// * `false` if handoff would be circular or depth exceeded
    pub fn can_handoff_to(&self, target_agent: &str, context: &HandoffContext) -> bool {
        // Check depth
        if context.depth >= self.config.max_depth {
            return false;
        }

        // Check if agent is in chain (would be circular)
        !context.chain.iter().any(|agent| agent == target_agent)
    }

    /// Transfers context for a handoff to a new agent
    ///
    /// Creates a new HandoffContext with:
    /// - Updated chain including the new agent
    /// - Incremented depth
    /// - Preserved history and metadata
    /// - New task description
    ///
    /// # Arguments
    /// * `new_task` - Task description for the target agent
    /// * `current_context` - Current handoff context
    /// * `target_agent` - Name of the agent receiving the handoff
    ///
    /// # Returns
    /// A new HandoffContext configured for the target agent
    ///
    /// # Example
    /// ```
    /// use paladin::application::services::paladin::handoff_service::HandoffService;
    /// use paladin::core::platform::container::autonomous_config::HandoffConfig;
    /// use paladin::core::platform::container::handoff::{HandoffStrategy, HandoffContext};
    /// use std::sync::Arc;
    ///
    /// let config = Arc::new(HandoffConfig {
    ///     enabled: true,
    ///     strategy: HandoffStrategy::Automatic,
    ///     max_depth: 3,
    ///     retry: Default::default(),
    /// });
    /// let service = HandoffService::new(config).unwrap();
    /// let context = HandoffContext::new("Original task".to_string(), "Agent1".to_string());
    ///
    /// let new_context = service.transfer_context("Subtask", &context, "Agent2");
    /// assert_eq!(new_context.depth, 2);
    /// assert_eq!(new_context.chain.len(), 2);
    /// ```
    pub fn transfer_context(
        &self,
        new_task: &str,
        current_context: &HandoffContext,
        target_agent: &str,
    ) -> HandoffContext {
        let mut new_chain = current_context.chain.clone();
        new_chain.push(target_agent.to_string());

        HandoffContext {
            task: new_task.to_string(),
            chain: new_chain,
            history: current_context.history.clone(),
            metadata: current_context.metadata.clone(),
            depth: current_context.depth + 1,
        }
    }

    /// Classifies whether an error is transient (retriable) or permanent (fail-fast)
    ///
    /// Transient errors include:
    /// - Timeouts
    /// - Network-related errors (temp unavailable)
    /// - Rate limits (implicit in network errors)
    ///
    /// Permanent errors include:
    /// - Invalid agent (not found)
    /// - Circular handoff
    /// - Max depth exceeded
    /// - Configuration errors
    fn is_transient_error(error: &HandoffError) -> bool {
        match error {
            HandoffError::Timeout(_) => true,
            HandoffError::ExecutionFailed { reason, .. } => {
                let reason_lower = reason.to_lowercase();
                reason_lower.contains("network")
                    || reason_lower.contains("temporary")
                    || reason_lower.contains("unavailable")
                    || reason_lower.contains("timeout")
            }
            _ => false,
        }
    }

    /// Executes a handoff to a specialist Paladin with retry logic
    ///
    /// This method delegates a task to a specialist agent, handling:
    /// - Validation (circular delegation, max depth)
    /// - Context transfer to the specialist
    /// - Execution via `PaladinExecutorPort` (dependency-inverted)
    /// - Retry with exponential backoff for transient errors
    /// - Fail-fast for permanent errors
    /// - `HandoffRecord` creation for traceability
    ///
    /// # Arguments
    ///
    /// * `specialist_name` - Name of the specialist agent
    /// * `task` - Task description to delegate
    /// * `context` - Current handoff context (depth, chain, history)
    /// * `specialist` - The specialist Paladin to execute
    /// * `executor` - Execution port for running the specialist
    ///
    /// # Returns
    ///
    /// * `Ok((String, HandoffRecord))` - The specialist's output and a record of the handoff
    /// * `Err(HandoffError)` - If validation fails or execution exhausts retries
    pub async fn execute_handoff(
        &self,
        specialist_name: &str,
        task: &str,
        context: &HandoffContext,
        specialist: &Paladin,
        executor: &dyn PaladinExecutorPort,
    ) -> Result<(String, HandoffRecord), HandoffError> {
        info!(
            "Executing handoff: from_chain={:?}, to={}, task_len={}, depth={}",
            context.chain,
            specialist_name,
            task.len(),
            context.depth
        );

        // Step 1: Validate the handoff (permanent errors → fail immediately)
        self.validate_handoff(specialist_name, context)?;

        // Step 2: Transfer context to the specialist
        let new_context = self.transfer_context(task, context, specialist_name);
        debug!(
            "Context transferred: new_depth={}, chain={:?}",
            new_context.depth, new_context.chain
        );

        // Step 3: Create handoff record (result will be filled after execution)
        let from_agent = context
            .chain
            .last()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let mut record = HandoffRecord::new(
            from_agent.clone(),
            specialist_name.to_string(),
            task.to_string(),
            new_context.depth,
        );

        // Step 4: Execute with retry logic
        let max_retries = self.config.retry.max_retries;
        let mut last_error: Option<HandoffError> = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                let backoff_ms = self.config.retry.calculate_backoff(attempt - 1);
                info!(
                    "Handoff retry: attempt={}/{}, backoff={}ms, specialist={}",
                    attempt, max_retries, backoff_ms, specialist_name
                );
                sleep(Duration::from_millis(backoff_ms)).await;
            }

            match executor.execute(specialist, task).await {
                Ok(result) => {
                    info!(
                        "Handoff succeeded: specialist={}, tokens={}, loops={}, time={}ms",
                        specialist_name,
                        result.usage.total_tokens,
                        result.loop_count,
                        result.execution_time_ms
                    );
                    record.set_result(result.output.clone());
                    return Ok((result.output, record));
                }
                Err(paladin_err) => {
                    // Convert PaladinError → HandoffError for classification
                    let handoff_err = HandoffError::ExecutionFailed {
                        from_agent: from_agent.clone(),
                        to_agent: specialist_name.to_string(),
                        reason: paladin_err.to_string(),
                    };

                    // Check if this is a permanent error → fail immediately
                    if !Self::is_transient_error(&handoff_err) {
                        warn!(
                            "Handoff permanent failure: specialist={}, error={}",
                            specialist_name, handoff_err
                        );
                        return Err(handoff_err);
                    }

                    // Transient error → retry (if attempts remain)
                    warn!(
                        "Handoff transient failure: specialist={}, attempt={}/{}, error={}",
                        specialist_name, attempt, max_retries, handoff_err
                    );
                    last_error = Some(handoff_err);
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| HandoffError::ExecutionFailed {
            from_agent,
            to_agent: specialist_name.to_string(),
            reason: "All retry attempts exhausted".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_getter() {
        let config = Arc::new(HandoffConfig {
            enabled: true,
            strategy: HandoffStrategy::Explicit,
            max_depth: 3,
            retry: Default::default(),
        });

        let service = HandoffService::new(config).unwrap();
        assert!(matches!(service.strategy(), HandoffStrategy::Explicit));
    }

    #[test]
    fn test_max_depth_getter() {
        let config = Arc::new(HandoffConfig {
            enabled: true,
            strategy: HandoffStrategy::Automatic,
            max_depth: 7,
            retry: Default::default(),
        });

        let service = HandoffService::new(config).unwrap();
        assert_eq!(service.max_depth(), 7);
    }

    #[test]
    fn test_threshold_strategy() {
        let config = Arc::new(HandoffConfig {
            enabled: true,
            strategy: HandoffStrategy::threshold(0.85),
            max_depth: 5,
            retry: Default::default(),
        });

        let service = HandoffService::new(config).unwrap();
        assert!(service.strategy().get_threshold().is_some());
        assert!((service.strategy().get_threshold().unwrap() - 0.85).abs() < 0.01);
    }
}
