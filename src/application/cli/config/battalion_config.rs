//! Battalion YAML configuration types for multi-agent orchestration
//!
//! This module defines configuration schemas for four Battalion orchestration patterns:
//! - **Formation**: Sequential execution (output N → input N+1)
//! - **Phalanx**: Concurrent execution with result aggregation
//! - **Campaign**: Graph/DAG-based conditional routing
//! - **Chain of Command**: Hierarchical delegation with commander
//!
//! # Example Formation Config
//!
//! ```yaml
//! type: formation
//! name: "document-pipeline"
//! agents:
//!   - name: "analyzer"
//!     system_prompt: "Analyze the input"
//!     model: "gpt-4"
//!     provider:
//!       type: "openai"
//!   - name: "summarizer"
//!     system_prompt: "Summarize the analysis"
//!     model: "gpt-4"
//!     provider:
//!       type: "openai"
//! pass_output_to_next: true
//! ```

use crate::application::cli::error::CliError;
use serde::{Deserialize, Serialize};

/// Battalion YAML configuration with type discriminator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BattalionYamlConfig {
    #[serde(rename = "formation")]
    Formation(FormationConfig),

    #[serde(rename = "phalanx")]
    Phalanx(PhalanxConfig),

    #[serde(rename = "campaign")]
    Campaign(CampaignConfig),

    #[serde(rename = "chain-of-command")]
    ChainOfCommand(ChainOfCommandConfig),

    #[serde(rename = "conclave")]
    Conclave(ConclaveConfig),

    #[serde(rename = "maneuver")]
    Maneuver(ManeuverConfig),
}

impl BattalionYamlConfig {
    /// Get the battalion type as a string
    pub fn battalion_type(&self) -> &str {
        match self {
            BattalionYamlConfig::Formation(_) => "formation",
            BattalionYamlConfig::Phalanx(_) => "phalanx",
            BattalionYamlConfig::Campaign(_) => "campaign",
            BattalionYamlConfig::ChainOfCommand(_) => "chain-of-command",
            BattalionYamlConfig::Conclave(_) => "conclave",
            BattalionYamlConfig::Maneuver(_) => "maneuver",
        }
    }

    /// Validate the battalion configuration
    pub fn validate(&self) -> Result<(), CliError> {
        match self {
            BattalionYamlConfig::Formation(config) => config.validate(),
            BattalionYamlConfig::Phalanx(config) => config.validate(),
            BattalionYamlConfig::Campaign(config) => config.validate(),
            BattalionYamlConfig::ChainOfCommand(config) => config.validate(),
            BattalionYamlConfig::Conclave(config) => config.validate(),
            BattalionYamlConfig::Maneuver(config) => config.validate(),
        }
    }
}

/// Formation configuration - Sequential execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationConfig {
    /// Name of the formation
    pub name: String,

    /// List of Paladins in execution order
    pub paladins: Vec<PaladinReference>,

    /// Whether to pass output from one Paladin to the next
    #[serde(default = "default_true")]
    pub pass_output_to_next: bool,
}

impl FormationConfig {
    fn validate(&self) -> Result<(), CliError> {
        if self.name.is_empty() {
            return Err(CliError::MissingRequiredField {
                field: "name".to_string(),
                message: "Campaign name is required".to_string(),
            });
        }

        if self.paladins.is_empty() {
            return Err(CliError::ValidationError {
                message: "Formation must have at least one Paladin".to_string(),
            });
        }

        Ok(())
    }
}

/// Phalanx configuration - Parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhalanxConfig {
    /// Name of the phalanx
    pub name: String,

    /// List of Paladins to execute in parallel
    pub paladins: Vec<PaladinReference>,

    /// Optional inputs for each Paladin (if different)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
}

impl PhalanxConfig {
    fn validate(&self) -> Result<(), CliError> {
        if self.name.is_empty() {
            return Err(CliError::MissingRequiredField {
                field: "name".to_string(),
                message: "Phalanx name is required".to_string(),
            });
        }

        if self.paladins.is_empty() {
            return Err(CliError::ValidationError {
                message: "Phalanx must have at least one Paladin".to_string(),
            });
        }

        // If inputs provided, must match paladin count
        if let Some(inputs) = &self.inputs
            && inputs.len() != self.paladins.len()
        {
            return Err(CliError::ValidationError {
                message: format!(
                    "Number of inputs ({}) must match number of Paladins ({})",
                    inputs.len(),
                    self.paladins.len()
                ),
            });
        }

        Ok(())
    }
}

/// Campaign configuration - DAG-based execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConfig {
    /// Name of the campaign
    pub name: String,

    /// Nodes in the DAG (Paladins)
    pub nodes: Vec<CampaignNode>,

    /// Edges defining dependencies
    pub edges: Vec<CampaignEdge>,

    /// Starting node ID
    pub start_node: String,
}

impl CampaignConfig {
    fn validate(&self) -> Result<(), CliError> {
        if self.name.is_empty() {
            return Err(CliError::MissingRequiredField {
                field: "name".to_string(),
                message: "Campaign name is required".to_string(),
            });
        }

        if self.nodes.is_empty() {
            return Err(CliError::ValidationError {
                message: "Campaign must have at least one node".to_string(),
            });
        }

        // Validate start_node exists
        if !self.nodes.iter().any(|n| n.id == self.start_node) {
            return Err(CliError::ValidationError {
                message: format!("Start node '{}' not found in nodes", self.start_node),
            });
        }

        // Validate all edge references exist
        for edge in &self.edges {
            if !self.nodes.iter().any(|n| n.id == edge.from) {
                return Err(CliError::ValidationError {
                    message: format!("Edge references non-existent node: {}", edge.from),
                });
            }
            if !self.nodes.iter().any(|n| n.id == edge.to) {
                return Err(CliError::ValidationError {
                    message: format!("Edge references non-existent node: {}", edge.to),
                });
            }
        }

        Ok(())
    }
}

/// Campaign node (Paladin in DAG)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignNode {
    /// Unique node ID
    pub id: String,

    /// Paladin reference
    pub paladin: PaladinReference,
}

/// Campaign edge (dependency between nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignEdge {
    /// Source node ID
    pub from: String,

    /// Target node ID
    pub to: String,
}

/// Chain of Command configuration - Hierarchical delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfCommandConfig {
    /// Name of the chain
    pub name: String,

    /// Commander Paladin (top level)
    pub commander: PaladinReference,

    /// Delegate Paladins
    pub delegates: Vec<PaladinReference>,
}

impl ChainOfCommandConfig {
    fn validate(&self) -> Result<(), CliError> {
        if self.name.is_empty() {
            return Err(CliError::MissingRequiredField {
                field: "name".to_string(),
                message: "Chain of Command name is required".to_string(),
            });
        }

        if self.delegates.is_empty() {
            return Err(CliError::ValidationError {
                message: "Chain of Command must have at least one delegate".to_string(),
            });
        }

        Ok(())
    }
}

/// Conclave configuration - Mixture of Agents pattern
///
/// Multiple expert Paladins analyze the task in parallel, then an aggregator
/// Paladin synthesizes their outputs into a comprehensive response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConclaveConfig {
    /// Name of the conclave
    pub name: String,

    /// Expert Paladins (minimum 2 required)
    pub experts: Vec<PaladinReference>,

    /// Aggregator Paladin for synthesis
    pub aggregator: PaladinReference,

    /// Per-attempt timeout in seconds for each expert and the aggregator (optional, defaults to 300)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    /// Number of retry attempts for failed experts (optional, defaults to 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_attempts: Option<u32>,

    /// Custom synthesis prompt for aggregator (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesis_prompt: Option<String>,

    /// Include expert names in aggregator input (optional, defaults to true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_expert_names: Option<bool>,

    /// Max tokens per expert output before truncation (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_expert_output_tokens: Option<usize>,

    /// Observability level: minimal, standard, verbose (optional, defaults to standard)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observability_level: Option<String>,
}

impl ConclaveConfig {
    fn validate(&self) -> Result<(), CliError> {
        if self.name.is_empty() {
            return Err(CliError::MissingRequiredField {
                field: "name".to_string(),
                message: "Conclave name is required".to_string(),
            });
        }

        if self.experts.len() < 2 {
            return Err(CliError::ValidationError {
                message: "Conclave requires at least 2 expert Paladins".to_string(),
            });
        }

        // Validate timeout if provided
        if let Some(timeout) = self.timeout_seconds
            && timeout == 0
        {
            return Err(CliError::ValidationError {
                message: "Timeout must be greater than 0".to_string(),
            });
        }

        // Validate observability level if provided
        if let Some(ref level) = self.observability_level {
            let valid_levels = ["minimal", "standard", "verbose"];
            if !valid_levels.contains(&level.as_str()) {
                return Err(CliError::InvalidFieldValue {
                    field: "observability_level".to_string(),
                    message: format!(
                        "must be one of: {}. Got: {}",
                        valid_levels.join(", "),
                        level
                    ),
                });
            }
        }

        Ok(())
    }
}

/// Maneuver configuration - Flow DSL-based orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManeuverConfig {
    /// Name of the maneuver
    pub name: String,

    /// Flow expression (e.g., "agent1 -> agent2", "(a, b) -> c")
    pub flow: String,

    /// List of available Paladins for the maneuver
    pub paladins: Vec<PaladinReference>,

    /// Optional visualization format (ascii or mermaid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visualize: Option<String>,
}

impl ManeuverConfig {
    fn validate(&self) -> Result<(), CliError> {
        if self.name.is_empty() {
            return Err(CliError::MissingRequiredField {
                field: "name".to_string(),
                message: "Maneuver name is required".to_string(),
            });
        }

        if self.flow.is_empty() {
            return Err(CliError::MissingRequiredField {
                field: "flow".to_string(),
                message: "Flow expression is required".to_string(),
            });
        }

        if self.paladins.is_empty() {
            return Err(CliError::ValidationError {
                message: "Maneuver must have at least one Paladin".to_string(),
            });
        }

        // Validate flow expression syntax
        use crate::core::platform::container::battalion::parser::FlowParser;
        FlowParser::parse(&self.flow).map_err(|e| CliError::ValidationError {
            message: format!("Invalid flow expression: {}", e),
        })?;

        Ok(())
    }
}

/// Reference to a Paladin (inline config or external file)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PaladinReference {
    /// Path to external Paladin config file
    File { file: String },

    /// Inline Paladin configuration
    Inline(Box<crate::application::cli::config::paladin_config::PaladinYamlConfig>),
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_valid() {
        let config = FormationConfig {
            name: "test".to_string(),
            paladins: vec![PaladinReference::File {
                file: "paladin1.yaml".to_string(),
            }],
            pass_output_to_next: true,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_formation_empty_name() {
        let config = FormationConfig {
            name: "".to_string(),
            paladins: vec![PaladinReference::File {
                file: "paladin1.yaml".to_string(),
            }],
            pass_output_to_next: true,
        };

        assert!(matches!(
            config.validate(),
            Err(CliError::MissingRequiredField { .. })
        ));
    }

    #[test]
    fn test_phalanx_inputs_mismatch() {
        let config = PhalanxConfig {
            name: "test".to_string(),
            paladins: vec![
                PaladinReference::File {
                    file: "paladin1.yaml".to_string(),
                },
                PaladinReference::File {
                    file: "paladin2.yaml".to_string(),
                },
            ],
            inputs: Some(vec!["input1".to_string()]), // Only 1 input for 2 paladins
        };

        assert!(matches!(
            config.validate(),
            Err(CliError::ValidationError { .. })
        ));
    }

    #[test]
    fn cli_conclave_yaml_timeout_seconds_still_loads() {
        // The CLI's own conclave schema is separate from the core `ConclaveConfig` and keeps its
        // `timeout_seconds` key: `per_attempt_timeout` maps it onto `aegis.timeout` (D-23 refuses
        // the removed key on the core types only).
        let yaml = "\
type: conclave
name: panel
experts:
  - file: expert1.yaml
  - file: expert2.yaml
aggregator:
  file: aggregator.yaml
timeout_seconds: 120
";
        let config: BattalionYamlConfig =
            serde_yaml::from_str(yaml).expect("the CLI conclave YAML still loads");
        let BattalionYamlConfig::Conclave(ref conclave) = config else {
            panic!("expected a conclave config");
        };
        assert_eq!(conclave.timeout_seconds, Some(120));
        assert_eq!(conclave.experts.len(), 2);
        assert!(config.validate().is_ok());
    }
}
