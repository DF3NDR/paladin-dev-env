//! Campaign Pattern - Graph-based Paladin Orchestration
//!
//! Campaign executes Paladins according to a Directed Acyclic Graph (DAG) structure.
//! Supports branching, conditional routing, fan-out/fan-in, and parallel execution.
//!
//! # Examples
//!
//! ```no_run
//! use paladin_core::platform::container::battalion::campaign::{Campaign, CampaignEdge, EdgeCondition};
//! use paladin_core::platform::container::battalion::BattalionConfig;
//!
//! // Create a campaign
//! let config = BattalionConfig::new("research_campaign");
//! let mut campaign = Campaign::new(config);
//!
//! // Add Paladins and create a workflow
//! // let analyzer_id = campaign.add_paladin(analyzer);
//! // let summarizer_id = campaign.add_paladin(summarizer);
//!
//! // Connect them with an edge
//! // let edge = CampaignEdge::new(analyzer_id, summarizer_id, EdgeCondition::Always);
//! // campaign.add_edge(edge).unwrap();
//! ```

use crate::platform::container::battalion::{BattalionConfig, BattalionError};
use crate::platform::container::paladin::Paladin;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Edge condition types for conditional routing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeCondition {
    /// Always traverse this edge (unconditional)
    Always,

    /// Traverse if output contains the specified string
    Contains(String),

    /// Traverse if output matches the regex pattern
    Regex(String),

    /// Custom condition logic (user-defined function name)
    Custom(String),
}

/// Edge in the Campaign graph connecting two Paladins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignEdge {
    /// Source Paladin ID
    pub source: Uuid,

    /// Target Paladin ID
    pub target: Uuid,

    /// Condition for traversing this edge
    pub condition: EdgeCondition,

    /// Optional output transformation (prompt template)
    pub transform: Option<String>,
}

impl CampaignEdge {
    /// Create a new campaign edge
    pub fn new(source: Uuid, target: Uuid, condition: EdgeCondition) -> Self {
        Self {
            source,
            target,
            condition,
            transform: None,
        }
    }

    /// Add an output transformation to this edge
    pub fn with_transform(mut self, transform: String) -> Self {
        self.transform = Some(transform);
        self
    }
}

/// Campaign - Graph-based Paladin orchestration using DAG
///
/// Executes Paladins according to a directed acyclic graph structure,
/// enabling complex workflows with branching, conditional routing, and parallel execution.
#[derive(Debug, Clone)]
pub struct Campaign {
    /// Battalion configuration
    config: BattalionConfig,

    /// Graph structure (DAG)
    graph: DiGraph<Uuid, CampaignEdge>,

    /// Paladin instances mapped by UUID
    paladins: HashMap<Uuid, Paladin>,

    /// Mapping from UUID to graph NodeIndex
    node_indices: HashMap<Uuid, NodeIndex>,

    /// Entry points for execution (nodes with no incoming edges or explicitly set)
    entry_points: HashSet<Uuid>,
}

impl Campaign {
    /// Create a new Campaign with the given configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::battalion::campaign::Campaign;
    /// use paladin_core::platform::container::battalion::BattalionConfig;
    ///
    /// let config = BattalionConfig::new("my_campaign");
    /// let campaign = Campaign::new(config);
    /// ```
    pub fn new(config: BattalionConfig) -> Self {
        Self {
            config,
            graph: DiGraph::new(),
            paladins: HashMap::new(),
            node_indices: HashMap::new(),
            entry_points: HashSet::new(),
        }
    }

    /// Add a Paladin to the campaign
    ///
    /// Returns the UUID of the added Paladin for use in edge creation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_core::platform::container::battalion::campaign::Campaign;
    /// # use paladin_core::platform::container::battalion::BattalionConfig;
    /// # let config = BattalionConfig::new("campaign");
    /// # let mut campaign = Campaign::new(config);
    /// # let paladin = todo!();
    /// let paladin_id = campaign.add_paladin(paladin);
    /// ```
    pub fn add_paladin(&mut self, paladin: Paladin) -> Uuid {
        let uuid = paladin.uuid;
        let node_index = self.graph.add_node(uuid);
        self.paladins.insert(uuid, paladin);
        self.node_indices.insert(uuid, node_index);
        uuid
    }

    /// Add an edge between two Paladins
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::InvalidGraph` if:
    /// - Source or target Paladin doesn't exist
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_core::platform::container::battalion::campaign::{Campaign, CampaignEdge, EdgeCondition};
    /// # use paladin_core::platform::container::battalion::BattalionConfig;
    /// # let config = BattalionConfig::new("campaign");
    /// # let mut campaign = Campaign::new(config);
    /// # let id1 = uuid::Uuid::new_v4();
    /// # let id2 = uuid::Uuid::new_v4();
    /// let edge = CampaignEdge::new(id1, id2, EdgeCondition::Always);
    /// campaign.add_edge(edge)?;
    /// # Ok::<(), paladin_core::platform::container::battalion::BattalionError>(())
    /// ```
    pub fn add_edge(&mut self, edge: CampaignEdge) -> Result<(), BattalionError> {
        // Validate source and target exist
        let source_idx = self.node_indices.get(&edge.source).ok_or_else(|| {
            BattalionError::InvalidGraph(format!(
                "Source Paladin {} not found in campaign",
                edge.source
            ))
        })?;

        let target_idx = self.node_indices.get(&edge.target).ok_or_else(|| {
            BattalionError::InvalidGraph(format!(
                "Target Paladin {} not found in campaign",
                edge.target
            ))
        })?;

        // Add edge to graph
        self.graph.add_edge(*source_idx, *target_idx, edge);

        Ok(())
    }

    /// Set an explicit entry point for execution
    ///
    /// Entry points are Paladins that will be executed first, even if they have incoming edges.
    /// Useful for workflows with multiple independent starting points.
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::InvalidGraph` if the Paladin doesn't exist.
    pub fn set_entry_point(&mut self, paladin_id: Uuid) -> Result<(), BattalionError> {
        if !self.paladins.contains_key(&paladin_id) {
            return Err(BattalionError::InvalidGraph(format!(
                "Paladin {} not found in campaign, cannot set as entry point",
                paladin_id
            )));
        }

        self.entry_points.insert(paladin_id);
        Ok(())
    }

    /// Get the entry points for this campaign
    ///
    /// Returns explicitly set entry points, or if none are set, returns all nodes with no incoming edges.
    pub fn entry_points(&self) -> HashSet<Uuid> {
        if !self.entry_points.is_empty() {
            return self.entry_points.clone();
        }

        // Find nodes with no incoming edges
        let mut entries = HashSet::new();
        for (uuid, &node_idx) in &self.node_indices {
            // Check if this node has any incoming edges
            let has_incoming = self
                .graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .next()
                .is_some();

            if !has_incoming {
                entries.insert(*uuid);
            }
        }

        entries
    }

    /// Validate the campaign structure
    ///
    /// Checks:
    /// - At least one Paladin exists
    /// - Graph is acyclic (no cycles)
    /// - All edges reference valid Paladins
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::InvalidGraph` if validation fails.
    pub fn validate(&self) -> Result<(), BattalionError> {
        // Check minimum Paladins
        if self.paladins.is_empty() {
            return Err(BattalionError::InvalidGraph(
                "Campaign must have at least one Paladin".to_string(),
            ));
        }

        // Check for cycles using petgraph
        if petgraph::algo::toposort(&self.graph, None).is_err() {
            return Err(BattalionError::InvalidGraph(
                "Campaign graph contains a cycle, must be a DAG".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if a Paladin exists in the campaign
    pub fn has_paladin(&self, paladin_id: &Uuid) -> bool {
        self.paladins.contains_key(paladin_id)
    }

    /// Get the number of Paladins in the campaign
    pub fn paladin_count(&self) -> usize {
        self.paladins.len()
    }

    /// Get the number of edges in the campaign
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get the battalion configuration
    pub fn config(&self) -> &BattalionConfig {
        &self.config
    }

    /// Get a reference to a Paladin by UUID
    pub fn get_paladin(&self, paladin_id: &Uuid) -> Option<&Paladin> {
        self.paladins.get(paladin_id)
    }

    /// Get all Paladins in the campaign
    pub fn paladins(&self) -> &HashMap<Uuid, Paladin> {
        &self.paladins
    }

    /// Get the graph structure (read-only)
    pub fn graph(&self) -> &DiGraph<Uuid, CampaignEdge> {
        &self.graph
    }

    /// Get the node indices mapping
    pub fn node_indices(&self) -> &HashMap<Uuid, NodeIndex> {
        &self.node_indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::entity::Entity;
    use crate::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};

    fn create_test_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("You are {}", name),
            name: name.to_string(),
            user_name: "User".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(1),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };
        Entity::new(data, Some(name.to_string()))
    }

    #[test]
    fn test_campaign_creation() {
        let config = BattalionConfig::new("test");
        let campaign = Campaign::new(config);

        assert_eq!(campaign.paladin_count(), 0);
        assert_eq!(campaign.edge_count(), 0);
    }

    #[test]
    fn test_add_paladin_returns_uuid() {
        let config = BattalionConfig::new("test");
        let mut campaign = Campaign::new(config);
        let paladin = create_test_paladin("Test");

        let uuid = campaign.add_paladin(paladin);

        assert!(campaign.has_paladin(&uuid));
        assert_eq!(campaign.paladin_count(), 1);
    }

    #[test]
    fn test_add_edge_success() {
        let config = BattalionConfig::new("test");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("P1"));
        let id2 = campaign.add_paladin(create_test_paladin("P2"));

        let edge = CampaignEdge::new(id1, id2, EdgeCondition::Always);
        let result = campaign.add_edge(edge);

        assert!(result.is_ok());
        assert_eq!(campaign.edge_count(), 1);
    }

    #[test]
    fn test_validate_empty_campaign_fails() {
        let config = BattalionConfig::new("test");
        let campaign = Campaign::new(config);

        let result = campaign.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_single_paladin_succeeds() {
        let config = BattalionConfig::new("test");
        let mut campaign = Campaign::new(config);
        campaign.add_paladin(create_test_paladin("P1"));

        let result = campaign.validate();
        assert!(result.is_ok());
    }
}
