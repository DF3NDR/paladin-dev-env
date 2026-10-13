use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// The universal base-layer wrapper: an identity, timestamps, an optional display name and a
/// payload of type `T`.
///
/// Platform types such as `Paladin`, `Council`, `Grove`, `User` and `Log` are an
/// `Entity` around their own data struct.
///
/// # Revisioning contract
///
/// `version` declares that this entity wants revision history. It creates none: the flag is
/// intent, not behaviour. It is honoured only by a caller that routes its writes through
/// [`EntityVersionService`](crate::base::service::entity_version_service::EntityVersionService),
/// whose `create_version` returns `VersioningError::VersioningDisabled` when the flag is
/// false.
///
/// Today the only such caller is the root crate's `ContentItemService`, so only
/// `ContentItem` is revisioned. `Paladin`, `Council`, `Grove`, `User` and `Log` carry the
/// flag but nothing reads it, and `Assistant` versions itself with its own
/// `AssistantVersion`. [`Entity::new`] sets the flag to `true`. Platform-wide revisioning is
/// not promised.
///
/// # Serialization
///
/// The payload field `data` was named `node` before v0.11.
///
/// - **Read:** the old key is accepted, so every document written by v0.10 still loads.
/// - **Write:** only the new key is emitted.
/// - **Ambiguity:** a document carrying both keys is rejected (`duplicate field`), and a
///   document carrying neither is rejected (`missing field`); nothing is merged or defaulted.
/// - **Downgrade:** a v0.10 binary cannot read JSON written by v0.11, so back up persisted
///   documents before upgrading and restore them to downgrade.
/// - **Lifetime:** the read-alias is kept for the whole 0.x line; removing it needs its own
///   ADR (ADR-0060).
/// - **Order:** fields serialize in declaration order (`uuid`, `created`, `modified`,
///   `data`, `name`, `version`).
///
/// # Examples
///
/// ```
/// use paladin_core::base::entity::Entity;
///
/// let entity = Entity::new(42_u32, Some("answer".to_string()));
///
/// assert_eq!(entity.data, 42);
/// assert_eq!(entity.name.as_deref(), Some("answer"));
/// assert!(entity.is_versioning_enabled());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entity<T> {
    /// Stable identity of the entity.
    pub uuid: Uuid,
    /// When the entity was created.
    pub created: DateTime<Utc>,
    /// When the entity was last modified.
    pub modified: DateTime<Utc>,
    /// The payload; read under its v0.10 key `node` too.
    #[serde(alias = "node")]
    pub data: T,
    /// Optional display name.
    pub name: Option<String>,
    /// Whether this entity wants revision history; see the revisioning contract above.
    pub version: bool,
}

impl<T> Hash for Entity<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
        self.name.hash(state);
        self.data.hash(state);
    }
}

impl<T> Entity<T> {
    /// Creates an entity with a fresh `uuid`, equal timestamps and `version` set to `true`.
    pub fn new(data: T, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            uuid: Uuid::new_v4(),
            created: now,
            modified: now,
            data,
            name,
            version: true,
        }
    }

    /// Creates an entity with the given `uuid`, equal timestamps and `version` set to `true`.
    pub fn new_with_uuid(uuid: Uuid, data: T, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            uuid,
            created: now,
            modified: now,
            data,
            name,
            version: true,
        }
    }

    /// Replaces the payload and refreshes `modified`.
    pub fn update(&mut self, new_data: T) {
        self.data = new_data;
        self.modified = Utc::now();
    }

    /// Sets the display name and refreshes `modified`.
    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
        self.modified = Utc::now();
    }

    /// Declares that this entity wants revision history and refreshes `modified`.
    pub fn enable_versioning(&mut self) {
        self.version = true;
        self.modified = Utc::now();
    }

    /// Declares that this entity does not want revision history and refreshes `modified`.
    pub fn disable_versioning(&mut self) {
        self.version = false;
        self.modified = Utc::now();
    }

    /// Returns the `version` flag; see the revisioning contract on [`Entity`].
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

        assert_eq!(entity.data, test_data);
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

        assert_eq!(entity.data, new_data);
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

        // Different entities should have different hashes
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

    /// The wire order is the declaration order (`uuid, created, modified, data, name,
    /// version`), so persisted documents keep their shape and the derived `Ord` is stable.
    #[test]
    fn test_entity_serializes_fields_in_declaration_order() -> Result<(), Box<dyn std::error::Error>>
    {
        let entity = Entity::new(7_u32, Some("n".to_string()));

        let serialized = serde_json::to_string(&entity)?;

        let mut previous = None;
        for key in ["uuid", "created", "modified", "data", "name", "version"] {
            let offset = serialized
                .find(&format!("\"{key}\":"))
                .ok_or_else(|| format!("key {key} missing from {serialized}"))?;
            if let Some(before) = previous {
                assert!(offset > before, "key {key} is out of order in {serialized}");
            }
            previous = Some(offset);
        }
        Ok(())
    }
}
