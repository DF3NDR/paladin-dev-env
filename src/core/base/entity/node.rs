use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Node<T> {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub node: T,
    pub name: Option<String>,
    pub version: bool,
}

impl<T> Hash for Node<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
        self.name.hash(state);
        self.node.hash(state);
    }
}

impl<T> Node<T> {
    pub fn new(node: T, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            uuid: Uuid::new_v4(),
            created: now,
            modified: now,
            node,
            name,
            version: true,
        }
    }

    pub fn new_with_uuid(uuid: Uuid, node: T, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            uuid,
            created: now,
            modified: now,
            node,
            name,
            version: true,
        }
    }

    pub fn update(&mut self, new_data: T) {
        self.node = new_data;
        self.modified = Utc::now();
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
        self.modified = Utc::now();
    }

    pub fn enable_versioning(&mut self) {
        self.version = true;
        self.modified = Utc::now();
    }

    pub fn disable_versioning(&mut self) {
        self.version = false;
        self.modified = Utc::now();
    }

    pub fn is_versioning_enabled(&self) -> bool {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    struct TestData {
        value: String,
        number: u32,
    }

    #[test]
    fn test_node_creation() {
        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };
        
        let node = Node::new(test_data.clone(), Some("Test Node".to_string()));
        
        assert_eq!(node.node, test_data);
        assert_eq!(node.name, Some("Test Node".to_string()));
        assert!(node.version);
        assert_eq!(node.created, node.modified);
    }

    #[test]
    fn test_node_update() {
        let initial_data = TestData {
            value: "initial".to_string(),
            number: 1,
        };
        
        let mut node = Node::new(initial_data, None);
        let initial_modified = node.modified;
        
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        let new_data = TestData {
            value: "updated".to_string(),
            number: 2,
        };
        
        node.update(new_data.clone());
        
        assert_eq!(node.node, new_data);
        assert!(node.modified > initial_modified);
    }

    #[test]
    fn test_versioning_control() {
        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };
        
        let mut node = Node::new(test_data, None);
        
        assert!(node.is_versioning_enabled());
        
        node.disable_versioning();
        assert!(!node.is_versioning_enabled());
        
        node.enable_versioning();
        assert!(node.is_versioning_enabled());
    }

    #[test]
    fn test_node_hash() {
        let test_data1 = TestData {
            value: "test1".to_string(),
            number: 1,
        };
        
        let test_data2 = TestData {
            value: "test2".to_string(),
            number: 2,
        };
        
        let node1 = Node::new(test_data1, Some("Node 1".to_string()));
        let node2 = Node::new(test_data2, Some("Node 2".to_string()));
        
        // Different nodes should have different hashes
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        
        node1.hash(&mut hasher1);
        node2.hash(&mut hasher2);
        
        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_node_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let test_data = TestData {
            value: "serialize_test".to_string(),
            number: 100,
        };
        
        let node = Node::new(test_data, Some("Serialization Test".to_string()));
        
        // Test serialization
        let serialized = serde_json::to_string(&node)?;
        assert!(!serialized.is_empty());
        
        // Test deserialization
        let deserialized: Node<TestData> = serde_json::from_str(&serialized)?;
        assert_eq!(node, deserialized);
        
        Ok(())
    }
}