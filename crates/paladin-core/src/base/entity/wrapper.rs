use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entity<T> {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub node: T,
    pub name: Option<String>,
    pub version: bool,
}

impl<T> Hash for Entity<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
        self.name.hash(state);
        self.node.hash(state);
    }
}

impl<T> Entity<T> {
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
    fn test_entity_creation() {
        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };

        let entity = Entity::new(test_data.clone(), Some("Test Entity".to_string()));

        assert_eq!(entity.node, test_data);
        assert_eq!(entity.name, Some("Test Entity".to_string()));
        assert!(entity.version);
        assert_eq!(entity.created, entity.modified);
    }

    #[test]
    fn test_entity_update() {
        let initial_data = TestData {
            value: "initial".to_string(),
            number: 1,
        };

        let mut entity = Entity::new(initial_data, None);
        let initial_modified = entity.modified;

        std::thread::sleep(std::time::Duration::from_millis(1));

        let new_data = TestData {
            value: "updated".to_string(),
            number: 2,
        };

        entity.update(new_data.clone());

        assert_eq!(entity.node, new_data);
        assert!(entity.modified > initial_modified);
    }

    #[test]
    fn test_versioning_control() {
        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };

        let mut entity = Entity::new(test_data, None);

        assert!(entity.is_versioning_enabled());

        entity.disable_versioning();
        assert!(!entity.is_versioning_enabled());

        entity.enable_versioning();
        assert!(entity.is_versioning_enabled());
    }

    #[test]
    fn test_entity_hash() {
        let test_data1 = TestData {
            value: "test1".to_string(),
            number: 1,
        };

        let test_data2 = TestData {
            value: "test2".to_string(),
            number: 2,
        };

        let entity1 = Entity::new(test_data1, Some("Entity 1".to_string()));
        let entity2 = Entity::new(test_data2, Some("Entity 2".to_string()));

        // Different nodes should have different hashes
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();

        entity1.hash(&mut hasher1);
        entity2.hash(&mut hasher2);

        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_entity_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let test_data = TestData {
            value: "serialize_test".to_string(),
            number: 100,
        };

        let entity = Entity::new(test_data, Some("Serialization Test".to_string()));

        // Test serialization
        let serialized = serde_json::to_string(&entity)?;
        assert!(!serialized.is_empty());

        // Test deserialization
        let deserialized: Entity<TestData> = serde_json::from_str(&serialized)?;
        assert_eq!(entity, deserialized);

        Ok(())
    }
}
