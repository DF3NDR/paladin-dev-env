//! Council Discussion Example
//!
//! Demonstrates the Council pattern with multiple expert Paladins engaged in a
//! structured discussion about implementing two-factor authentication.
//!
//! This example showcases:
//! - Creating a Council with multiple expert participants
//! - Round-robin turn-taking strategy
//! - Maximum rounds termination condition
//! - Formatted discussion transcript output
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example council_discussion
//! ```

use async_trait::async_trait;
use paladin::application::services::battalion::council_service::{
    CouncilExecutionService, CouncilResult,
};
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::council::{
    CouncilBuilder, TerminationCondition, TurnStrategy,
};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use std::sync::Arc;

/// Simple mock LLM adapter for demonstration purposes.
///
/// In a production environment, replace this with a real LLM adapter
/// (OpenAI, DeepSeek, Anthropic, etc.)
struct MockLlmAdapter {
    responses: std::collections::HashMap<String, Vec<String>>,
    indices: std::sync::Mutex<std::collections::HashMap<String, usize>>,
}

impl MockLlmAdapter {
    fn new() -> Self {
        let mut responses = std::collections::HashMap::new();

        // Security expert responses (participant_0)
        responses.insert(
            "participant_0".to_string(),
            vec![
                "From a security perspective, two-factor authentication is essential. It significantly reduces the risk of account compromise even if passwords are leaked. I recommend implementing TOTP-based 2FA with backup codes.".to_string(),
                "I agree with the legal requirements. We should use industry-standard protocols like RFC 6238 for TOTP. Additionally, we need to ensure the 2FA secret keys are encrypted at rest.".to_string(),
                "The implementation timeline looks feasible. I'll work on the security audit and penetration testing once the core implementation is complete.".to_string(),
            ],
        );

        // Legal expert responses (participant_1)
        responses.insert(
            "participant_1".to_string(),
            vec![
                "From a legal standpoint, implementing 2FA helps us comply with GDPR Article 32 regarding appropriate security measures. We should make it optional initially to avoid user friction, but strongly recommended.".to_string(),
                "We need to ensure proper consent mechanisms for storing 2FA credentials. I'll draft the updated privacy policy and terms of service amendments.".to_string(),
                "Agreed on the phased rollout. We should notify users 30 days in advance before making it mandatory for sensitive operations.".to_string(),
            ],
        );

        // Technical expert responses (participant_2)
        responses.insert(
            "participant_2".to_string(),
            vec![
                "I propose using a well-tested library like Google Authenticator compatible TOTP implementation. We can integrate it with our existing authentication system in about 2 sprints.".to_string(),
                "For backup codes, I suggest generating 10 single-use codes per user, stored with bcrypt hashing. We'll need to update our database schema and add new API endpoints.".to_string(),
                "I'll create a detailed implementation plan with milestones. We should start with a beta rollout to 5% of users for 2 weeks before full deployment.".to_string(),
            ],
        );

        Self {
            responses,
            indices: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl PaladinPort for MockLlmAdapter {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        let paladin_name = paladin.node.name.clone();

        // Get configured response based on paladin name
        let output = if let Some(responses) = self.responses.get(&paladin_name) {
            let mut indices = self.indices.lock().unwrap();
            let index = *indices.get(&paladin_name).unwrap_or(&0);

            if index < responses.len() {
                let response = responses[index].clone();
                indices.insert(paladin_name.clone(), index + 1);
                format!("[{}]: {}", paladin_name, response)
            } else {
                format!(
                    "[{}]: I've reviewed the discussion so far. Topic: {}",
                    paladin_name, input
                )
            }
        } else {
            format!("[{}]: Analysis of: {}", paladin_name, input)
        };

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(150, 0),
            execution_time_ms: 100,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Create a Paladin with expert configuration
fn create_expert_paladin(name: &str, expertise: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!(
            "You are a {} expert. Provide concise, professional advice based on your expertise. \
             Consider other experts' input and build upon their ideas.",
            expertise
        ),
        name: name.to_string(),
        user_name: "Council".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║           Council Discussion: Two-Factor Authentication         ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Create mock LLM adapter
    let llm_port = Arc::new(MockLlmAdapter::new());

    // Create expert Paladins
    // Council uses participant_N naming convention for participant IDs
    let security_expert = create_expert_paladin("participant_0", "cybersecurity");
    let legal_expert = create_expert_paladin("participant_1", "legal compliance");
    let technical_expert = create_expert_paladin("participant_2", "software engineering");

    // Create paladins array to pass to Council service
    let paladins = [security_expert, legal_expert, technical_expert];

    println!("📋 Council Configuration:");
    println!("   • Participants: Security Expert, Legal Expert, Technical Expert");
    println!("   • Turn Strategy: RoundRobin (each participant speaks in order)");
    println!("   • Max Rounds: 3");
    println!("   • Termination: MaxRounds (stops after 3 complete rounds)");
    println!();

    // Build the Council
    // The Council uses participant IDs that correspond to indices in the paladins vector
    let council = CouncilBuilder::new()
        .name("2FA Implementation Council")
        .add_participant("participant_0") // Security expert
        .add_participant("participant_1") // Legal expert
        .add_participant("participant_2") // Technical expert
        .max_rounds(3) // Maximum 3 rounds of discussion
        .turn_strategy(TurnStrategy::RoundRobin) // Each participant speaks in order
        .termination_condition(TerminationCondition::MaxRounds) // Stop after max rounds
        .build()?;

    // Create Paladin registry from paladins
    use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
    use paladin_ports::output::paladin_registry::PaladinRegistry;
    let registry = HashMapPaladinRegistry::new();
    for (idx, paladin) in paladins.iter().enumerate() {
        registry.register(format!("participant_{}", idx), Arc::new(paladin.clone()))?;
    }

    // Create Council execution service
    let council_service = CouncilExecutionService::new(
        llm_port,
        None, // No Garrison (history storage) for this simple example
        Arc::new(registry),
    );

    println!("🎯 Discussion Topic: \"Should we implement two-factor authentication?\"");
    println!();
    println!("────────────────────────────────────────────────────────────────");
    println!();

    // Execute the Council discussion
    let result: CouncilResult = council_service
        .convene(
            &council,
            "Should we implement two-factor authentication for our application? Please discuss security implications, legal requirements, and implementation timeline.",
        )
        .await?;

    println!("📝 Discussion Transcript:");
    println!();

    // Display formatted transcript
    for (i, message) in result.transcript.iter().enumerate() {
        println!("Round {} | Speaker: {}", message.round, message.speaker);
        println!("         {}", message.content);
        println!();

        // Add separator between speakers
        if i < result.transcript.len() - 1 {
            println!("   ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄");
            println!();
        }
    }

    println!("────────────────────────────────────────────────────────────────");
    println!();
    println!("📊 Discussion Summary:");
    println!("   • Total Rounds Completed: {}", result.rounds_completed);
    println!("   • Total Messages: {}", result.transcript.len());
    println!("   • Termination Reason: {:?}", result.termination_reason);

    if let Some(conclusion) = result.conclusion {
        println!();
        println!("🎯 Conclusion:");
        println!("   {}", conclusion);
    }

    println!();
    println!("✅ Council discussion completed successfully!");
    println!();
    println!("💡 Key Takeaways:");
    println!("   • Council pattern enables structured multi-agent discussions");
    println!("   • RoundRobin ensures each participant contributes equally");
    println!("   • MaxRounds termination prevents infinite discussions");
    println!("   • Transcript provides full audit trail of decision-making");

    Ok(())
}
