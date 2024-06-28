/*
Node Service

This service provides an interface for creating, updating, and fetching node items.
 */
use crate::core::entity::node::{NodeItem, NodeItemError, NodeType};

pub trait NodeService {
    // Creates a new node item.
    fn create_node(&self, node: NodeItem) -> Result<NodeItem, NodeItemError>;

    // Updates an existing node item.
    fn update_node(&self, node: NodeItem) -> Result<NodeItem, NodeItemError>;

    // Fetches a node item by its hash.
    fn get_node_by_hash(&self, hash: &str) -> Result<Option<NodeItem>, NodeItemError>;

    // Fetches a node item by its type.
    fn get_node_by_type(&self, node_type: NodeType) -> Result<Vec<NodeItem>, NodeItemError>;
}