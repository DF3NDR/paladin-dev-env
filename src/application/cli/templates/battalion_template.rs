//! Battalion configuration template generator

use crate::application::cli::error::CliError;

/// Generate a Battalion YAML configuration template
///
/// Creates a template for the specified battalion type with inline documentation
/// and example Paladin references.
///
/// # Arguments
///
/// * `name` - Name for the Battalion
/// * `battalion_type` - Type of battalion (formation, phalanx, campaign, chain-of-command)
///
/// # Returns
///
/// Returns a formatted YAML string with comprehensive documentation
pub fn generate_battalion_template(name: &str, battalion_type: &str) -> Result<String, CliError> {
    match battalion_type {
        "formation" => Ok(generate_formation_template(name)),
        "phalanx" => Ok(generate_phalanx_template(name)),
        "campaign" => Ok(generate_campaign_template(name)),
        "chain-of-command" => Ok(generate_chain_of_command_template(name)),
        "conclave" => Ok(generate_conclave_template(name)),
        "maneuver" => Ok(generate_maneuver_template(name)),
        _ => Err(CliError::InvalidFieldValue {
            field: "battalion_type".to_string(),
            message: format!(
                "must be one of: formation, phalanx, campaign, chain-of-command, conclave, maneuver. Got: {}",
                battalion_type
            ),
        }),
    }
}

/// Generate Formation template (sequential execution)
fn generate_formation_template(name: &str) -> String {
    format!(
        r#"# Formation Battalion Configuration
# Sequential execution: Output from one Paladin flows to the next

# Battalion type (required)
type: formation

# Formation name
name: "{}"

# List of Paladins to execute in sequence
# Each Paladin can be defined inline or reference an external config file
paladins:
  # Option 1: Reference external Paladin config file
  - file: "paladin1.yaml"
  
  # Option 2: Inline Paladin definition
  - inline:
      name: "AnalyzerPaladin"
      system_prompt: |
        You are an expert analyzer. Review the input and provide structured analysis.
        Focus on key points, patterns, and insights.
      model: "gpt-4"
      temperature: 0.7
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai
  
  # Option 3: Another external reference
  - file: "paladin2.yaml"

# Pass output from each Paladin to the next in sequence
# Set to false if each Paladin should receive the original input
pass_output_to_next: true

# Example usage:
# paladin battalion run -c formation.yaml -i "Analyze this text..."
"#,
        name
    )
}

/// Generate Phalanx template (parallel execution)
fn generate_phalanx_template(name: &str) -> String {
    format!(
        r#"# Phalanx Battalion Configuration
# Parallel execution: All Paladins execute simultaneously

# Battalion type (required)
type: phalanx

# Phalanx name
name: "{}"

# List of Paladins to execute in parallel
paladins:
  - inline:
      name: "SummarizerPaladin"
      system_prompt: |
        You are an expert summarizer. Create a concise summary of the input.
      model: "gpt-4"
      temperature: 0.5
      max_loops: 2
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai
  
  - inline:
      name: "SentimentAnalyzer"
      system_prompt: |
        You are a sentiment analysis expert. Analyze the sentiment and tone.
      model: "gpt-4"
      temperature: 0.3
      max_loops: 2
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai
  
  - inline:
      name: "KeywordExtractor"
      system_prompt: |
        You are a keyword extraction expert. Extract key terms and concepts.
      model: "gpt-4"
      temperature: 0.3
      max_loops: 2
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai

# Optional: Different inputs for each Paladin
# If omitted, all Paladins receive the same input
# inputs:
#   - "Summarize this text"
#   - "Analyze sentiment"
#   - "Extract keywords"

# Example usage:
# paladin battalion run -c phalanx.yaml -i "Process this document..."
"#,
        name
    )
}

/// Generate Campaign template (DAG/graph execution)
fn generate_campaign_template(name: &str) -> String {
    format!(
        r#"# Campaign Battalion Configuration
# Graph-based execution: Directed Acyclic Graph (DAG) of Paladins

# Battalion type (required)
type: campaign

# Campaign name
name: "{}"

# Nodes: Each node represents a Paladin in the graph
nodes:
  - id: "intake"
    paladin:
      inline:
        name: "IntakePaladin"
        system_prompt: |
          You are an intake processor. Validate and structure the input.
        model: "gpt-4"
        temperature: 0.3
        max_loops: 2
        timeout_seconds: 300
        stop_words: []
        provider:
          type: openai
  
  - id: "analyzer"
    paladin:
      inline:
        name: "AnalyzerPaladin"
        system_prompt: |
          You are an analyzer. Process structured data and extract insights.
        model: "gpt-4"
        temperature: 0.5
        max_loops: 3
        timeout_seconds: 300
        stop_words: []
        provider:
          type: openai
  
  - id: "formatter"
    paladin:
      inline:
        name: "FormatterPaladin"
        system_prompt: |
          You are a formatter. Take analysis results and format for final output.
        model: "gpt-4"
        temperature: 0.3
        max_loops: 2
        timeout_seconds: 300
        stop_words: []
        provider:
          type: openai

# Edges: Define dependencies between nodes
# Execution follows topological order respecting these dependencies
edges:
  - from: "intake"
    to: "analyzer"
  - from: "analyzer"
    to: "formatter"

# Start node: Where execution begins
start_node: "intake"

# Example usage:
# paladin battalion run -c campaign.yaml -i "Process this workflow..."
"#,
        name
    )
}

/// Generate Chain of Command template (hierarchical delegation)
fn generate_chain_of_command_template(name: &str) -> String {
    format!(
        r#"# Chain of Command Battalion Configuration
# Hierarchical execution: Commander delegates to specialized Paladins

# Battalion type (required)
type: chain_of_command

# Chain name
name: "{}"

# Commander: Top-level Paladin that orchestrates delegation
commander:
  inline:
    name: "CommanderPaladin"
    system_prompt: |
      You are a commander agent. Analyze the task and decide which specialized
      agents should handle different aspects. Break down complex tasks and
      delegate appropriately.
    model: "gpt-4"
    temperature: 0.7
    max_loops: 5
    timeout_seconds: 600
    stop_words: []
    provider:
      type: openai

# Delegates: Specialized Paladins available for delegation
delegates:
  - inline:
      name: "ResearchDelegate"
      system_prompt: |
        You are a research specialist. Gather information and provide detailed
        research on assigned topics.
      model: "gpt-4"
      temperature: 0.5
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai
  
  - inline:
      name: "WritingDelegate"
      system_prompt: |
        You are a writing specialist. Create well-structured documents and
        content based on research and requirements.
      model: "gpt-4"
      temperature: 0.7
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai
  
  - inline:
      name: "ReviewDelegate"
      system_prompt: |
        You are a review specialist. Evaluate quality, accuracy, and
        completeness of work products.
      model: "gpt-4"
      temperature: 0.3
      max_loops: 2
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai

# Example usage:
# paladin battalion run -c chain-of-command.yaml -i "Create a research report on AI safety"
"#,
        name
    )
}

/// Generate Conclave template (expert synthesis pattern)
fn generate_conclave_template(name: &str) -> String {
    format!(
        r#"# Conclave Battalion Configuration
# Mixture-of-Agents: Multiple expert Paladins analyze in parallel, then an aggregator synthesizes

# Battalion type (required)
type: conclave

# Conclave name
name: "{}"

# Expert Paladins: Execute in parallel to provide diverse perspectives
# Minimum 2 experts required, 3-5 recommended for quality synthesis
experts:
  - inline:
      name: "TechnicalExpert"
      system_prompt: |
        You are a technical expert with deep knowledge of software architecture,
        algorithms, and system design. Analyze the input from a technical perspective,
        focusing on implementation details, performance, scalability, and best practices.
        Be specific and cite technical considerations.
      model: "gpt-4o"
      temperature: 0.7
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai
  
  - inline:
      name: "BusinessExpert"
      system_prompt: |
        You are a business strategy expert with expertise in product management,
        market analysis, and organizational dynamics. Analyze the input from a business
        perspective, focusing on value proposition, market fit, ROI, and strategic
        alignment. Consider stakeholder needs and business impact.
      model: "gpt-4o"
      temperature: 0.7
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai
  
  - inline:
      name: "SecurityExpert"
      system_prompt: |
        You are a security and risk management expert. Analyze the input from a
        security perspective, identifying potential vulnerabilities, compliance
        requirements, threat vectors, and risk mitigation strategies. Consider
        both technical and organizational security aspects.
      model: "gpt-4o"
      temperature: 0.7
      max_loops: 3
      timeout_seconds: 300
      stop_words: []
      provider:
        type: openai

# Aggregator: Synthesizes expert outputs into a coherent final response
aggregator:
  inline:
    name: "SynthesisAggregator"
    system_prompt: |
      You are a synthesis expert who combines multiple perspectives into a coherent,
      comprehensive analysis. You will receive outputs from multiple expert agents,
      each providing their specialized perspective on the same input.
      
      Your role is to:
      1. Identify common themes and agreements across expert analyses
      2. Highlight valuable unique insights from each expert
      3. Resolve any contradictions by weighing evidence and reasoning
      4. Synthesize a balanced, comprehensive final response
      5. Ensure all critical points from experts are represented
      
      Create a well-structured synthesis that is greater than the sum of its parts.
      Do not simply concatenate expert outputs - integrate them thoughtfully.
    model: "gpt-4o"
    temperature: 0.5
    max_loops: 3
    timeout_seconds: 300
    stop_words: []
    provider:
      type: openai

# Optional Configuration:

# Per-attempt timeout for each expert and the aggregator (seconds)
# timeout_seconds: 300

# Number of retry attempts for failed expert executions
# retry_attempts: 2

# Custom synthesis prompt (overrides aggregator's system prompt for this execution)
# synthesis_prompt: |
#   Combine the following expert analyses into a unified recommendation:
#   Focus on actionable insights and prioritize technical feasibility.

# Include expert names in aggregator input for attribution
# include_expert_names: true

# Truncate individual expert outputs to this token limit before aggregation
# max_expert_output_tokens: 2000

# Observability level: minimal, standard, or verbose
# Controls logging detail during execution
# observability_level: "standard"

# Example usage:
# paladin battalion run -c conclave.yaml -i "Should we migrate our monolith to microservices?"
# paladin battalion run -c conclave.yaml -i "Evaluate this proposed API design" -o result.json
"#,
        name
    )
}

/// Generate Maneuver template (Flow DSL-based orchestration)
fn generate_maneuver_template(name: &str) -> String {
    format!(
        r#"# Maneuver Battalion Configuration
# Flow DSL-based orchestration with dynamic execution patterns

# Battalion type (required)
type: maneuver

# Maneuver name
name: "{}"

# Flow expression using Flow DSL syntax
# Syntax:
#   - Sequential: agent1 -> agent2 -> agent3
#   - Parallel: (agent1, agent2)
#   - Nested: agent1 -> (agent2, agent3) -> agent4
# Note: Use commas (,) for parallel, not pipes (|)
flow: "analyzer -> (summarizer, translator) -> reviewer"

# List of available Paladins (referenced by name in flow expression)
paladins:
  - inline:
      name: "analyzer"
      system_prompt: |
        You are an expert analyzer. Review the input and extract key insights.
      model: "gpt-4"
      temperature: 0.7
      max_loops: 1
      timeout_seconds: 300
      provider:
        type: openai

  - inline:
      name: "summarizer"
      system_prompt: |
        You are an expert summarizer. Create a concise summary from the analysis.
      model: "gpt-4"
      temperature: 0.5
      max_loops: 1
      timeout_seconds: 300
      provider:
        type: openai

  - inline:
      name: "translator"
      system_prompt: |
        You are an expert translator. Translate the analysis to simple language.
      model: "gpt-4"
      temperature: 0.5
      max_loops: 1
      timeout_seconds: 300
      provider:
        type: openai

  - inline:
      name: "reviewer"
      system_prompt: |
        You are a final reviewer. Combine and synthesize all previous outputs.
      model: "gpt-4"
      temperature: 0.6
      max_loops: 1
      timeout_seconds: 300
      provider:
        type: openai

# Optional: Visualize the flow before execution (ascii or mermaid)
# visualize: "ascii"

# Example usage:
# paladin battalion run -c maneuver.yaml -t maneuver -i "Analyze this document..."
# paladin maneuver visualize -c maneuver.yaml --format ascii
# paladin maneuver validate -c maneuver.yaml
"#,
        name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_formation_template() {
        let template = generate_battalion_template("TestFormation", "formation");
        assert!(template.is_ok());
        let yaml = template.unwrap();
        assert!(yaml.contains("type: formation"));
        assert!(yaml.contains("name: \"TestFormation\""));
        assert!(yaml.contains("pass_output_to_next: true"));
    }

    #[test]
    fn test_generate_phalanx_template() {
        let template = generate_battalion_template("TestPhalanx", "phalanx");
        assert!(template.is_ok());
        let yaml = template.unwrap();
        assert!(yaml.contains("type: phalanx"));
        assert!(yaml.contains("name: \"TestPhalanx\""));
        assert!(yaml.contains("SummarizerPaladin"));
    }

    #[test]
    fn test_generate_campaign_template() {
        let template = generate_battalion_template("TestCampaign", "campaign");
        assert!(template.is_ok());
        let yaml = template.unwrap();
        assert!(yaml.contains("type: campaign"));
        assert!(yaml.contains("name: \"TestCampaign\""));
        assert!(yaml.contains("nodes:"));
        assert!(yaml.contains("edges:"));
        assert!(yaml.contains("start_node: \"intake\""));
    }

    #[test]
    fn test_generate_chain_of_command_template() {
        let template = generate_battalion_template("TestChain", "chain-of-command");
        assert!(template.is_ok());
        let yaml = template.unwrap();
        assert!(yaml.contains("type: chain_of_command"));
        assert!(yaml.contains("name: \"TestChain\""));
        assert!(yaml.contains("commander:"));
        assert!(yaml.contains("delegates:"));
    }

    #[test]
    fn test_generate_conclave_template() {
        let template = generate_battalion_template("TestConclave", "conclave");
        assert!(template.is_ok());
        let yaml = template.unwrap();
        assert!(yaml.contains("type: conclave"));
        assert!(yaml.contains("name: \"TestConclave\""));
        assert!(yaml.contains("experts:"));
        assert!(yaml.contains("aggregator:"));
        assert!(yaml.contains("TechnicalExpert"));
        assert!(yaml.contains("BusinessExpert"));
        assert!(yaml.contains("SecurityExpert"));
        assert!(yaml.contains("SynthesisAggregator"));
    }

    #[test]
    fn test_invalid_battalion_type() {
        let result = generate_battalion_template("Test", "invalid");
        assert!(result.is_err());
        match result {
            Err(CliError::InvalidFieldValue { field, .. }) => {
                assert_eq!(field, "battalion_type");
            }
            _ => panic!("Expected InvalidFieldValue error"),
        }
    }

    #[test]
    fn test_all_templates_are_valid_yaml() {
        let types = vec![
            "formation",
            "phalanx",
            "campaign",
            "chain-of-command",
            "conclave",
            "maneuver",
        ];
        for battalion_type in types {
            let template = generate_battalion_template("Test", battalion_type).unwrap();
            let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(&template);
            assert!(
                parsed.is_ok(),
                "Template for {} should be valid YAML",
                battalion_type
            );
        }
    }
}
