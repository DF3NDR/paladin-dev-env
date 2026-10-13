//! Paladin Registry - HashMap-based Implementation
//!
//! This module provides a thread-safe, HashMap-based implementation of the
//! `PaladinRegistry` trait for storing and retrieving Paladin instances by ID.
//!
//! Used internally by [`commander`](crate::commander) and
//! [`grove_service`](crate::grove_service) to assemble temporary registries during
//! execution, and exported for use by the application facade and integration tests.
//! Because this type depends only on `paladin-core` and `paladin-ports` it carries
//! zero infrastructure overhead.
//!
//! # Features
//!
//! - Thread-safe concurrent access using `RwLock`
//! - `O(1)` average case lookup performance
//! - Duplicate ID prevention
//! - Shared ownership via `Arc<Paladin>`
//!
//! # Example
//!
//! ```rust
//! use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
//! use paladin_ports::output::paladin_registry::PaladinRegistry;
//! use paladin_core::platform::container::paladin::{PaladinData, Paladin};
//! use paladin_core::base::entity::Entity;
//! use std::sync::Arc;
//!
//! let registry = HashMapPaladinRegistry::new();
//!
//! // Register a Paladin
//! let paladin_data = PaladinData::default();
//! let paladin = Entity::new(paladin_data, Some("analyzer".to_string()));
//! registry.register("analyzer".to_string(), Arc::new(paladin)).unwrap();
//!
//! // Retrieve it
//! let retrieved = registry.get("analyzer").expect("Paladin should exist");
//! assert_eq!(retrieved.name, Some("analyzer".to_string()));
//! ```

use paladin_core::platform::container::paladin::Paladin;
use paladin_ports::output::paladin_registry::{PaladinRegistry, RegistryError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// HashMap-based Paladin registry with thread-safe concurrent access
///
/// Uses a `RwLock<HashMap>` internally to allow multiple concurrent readers
/// or a single writer. This provides good performance for read-heavy workloads
/// (which is typical for registry lookups during Battalion execution).
///
/// # Thread Safety
///
/// - Multiple threads can read simultaneously
/// - Writes are exclusive (block all other access)
/// - Lock poisoning is handled by returning `AccessFailed` errors
///
/// # Performance
///
/// - `register`: O(1) average case, acquires write lock
/// - `get`: O(1) average case, acquires read lock
/// - `contains`: O(1) average case, acquires read lock
/// - `list_ids`: O(n) where n is the number of registered Paladins, acquires read lock
#[derive(Debug, Default)]
pub struct HashMapPaladinRegistry {
    /// Internal storage: Paladin ID -> `Arc<Paladin>`
    paladins: RwLock<HashMap<String, Arc<Paladin>>>,
}

impl HashMapPaladinRegistry {
    /// Create a new empty registry
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
    /// use paladin_ports::output::paladin_registry::PaladinRegistry;
    ///
    /// let registry = HashMapPaladinRegistry::new();
    /// assert!(registry.list_ids().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            paladins: RwLock::new(HashMap::new()),
        }
    }

    /// Create a pre-populated registry from a HashMap
    ///
    /// Useful for testing or when migrating from another registry implementation.
    ///
    /// # Arguments
    ///
    /// * `paladins` - Initial HashMap of Paladin ID to Paladin mappings
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
    /// use paladin_ports::output::paladin_registry::PaladinRegistry;
    /// use paladin_core::platform::container::paladin::{PaladinData, Paladin};
    /// use paladin_core::base::entity::Entity;
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// let mut initial = HashMap::new();
    /// let paladin_data = PaladinData::default();
    /// let paladin = Entity::new(paladin_data, Some("analyzer".to_string()));
    /// initial.insert("analyzer".to_string(), Arc::new(paladin));
    ///
    /// let registry = HashMapPaladinRegistry::from_map(initial);
    /// assert!(registry.contains("analyzer"));
    /// ```
    pub fn from_map(paladins: HashMap<String, Arc<Paladin>>) -> Self {
        Self {
            paladins: RwLock::new(paladins),
        }
    }

    /// Get the current count of registered Paladins
    ///
    /// # Returns
    ///
    /// Number of Paladins currently in the registry
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
    /// use paladin_ports::output::paladin_registry::PaladinRegistry;
    /// use paladin_core::platform::container::paladin::{PaladinData, Paladin};
    /// use paladin_core::base::entity::Entity;
    /// use std::sync::Arc;
    ///
    /// let registry = HashMapPaladinRegistry::new();
    /// assert_eq!(registry.count(), 0);
    ///
    /// let paladin_data = PaladinData::default();
    /// let paladin = Entity::new(paladin_data, Some("test".to_string()));
    /// registry.register("test".to_string(), Arc::new(paladin)).unwrap();
    /// assert_eq!(registry.count(), 1);
    /// ```
    pub fn count(&self) -> usize {
        self.paladins.read().map(|map| map.len()).unwrap_or(0)
    }

    /// Clear all registered Paladins
    ///
    /// This is a destructive operation that removes all Paladins from the registry.
    /// Useful for cleanup or reset scenarios.
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
    /// use paladin_ports::output::paladin_registry::PaladinRegistry;
    /// use paladin_core::platform::container::paladin::{PaladinData, Paladin};
    /// use paladin_core::base::entity::Entity;
    /// use std::sync::Arc;
    ///
    /// let registry = HashMapPaladinRegistry::new();
    /// let paladin_data = PaladinData::default();
    /// let paladin = Entity::new(paladin_data, Some("test".to_string()));
    /// registry.register("test".to_string(), Arc::new(paladin)).unwrap();
    ///
    /// registry.clear();
    /// assert_eq!(registry.count(), 0);
    /// ```
    pub fn clear(&self) {
        if let Ok(mut map) = self.paladins.write() {
            map.clear();
        }
    }
}

impl PaladinRegistry for HashMapPaladinRegistry {
    fn register(&self, id: String, paladin: Arc<Paladin>) -> Result<(), RegistryError> {
        // Validate ID is not empty
        if id.is_empty() {
            return Err(RegistryError::InvalidId("ID cannot be empty".to_string()));
        }

        // Acquire write lock
        let mut map = self
            .paladins
            .write()
            .map_err(|e| RegistryError::AccessFailed(format!("Write lock poisoned: {}", e)))?;

        // Check for duplicates
        if map.contains_key(&id) {
            return Err(RegistryError::DuplicateId(id));
        }

        // Insert and return success
        map.insert(id, paladin);
        Ok(())
    }

    fn get(&self, id: &str) -> Option<Arc<Paladin>> {
        // Acquire read lock - on error, return None
        let map = self.paladins.read().ok()?;

        // Clone the Arc (cheap operation, just increments reference count)
        map.get(id).cloned()
    }

    fn contains(&self, id: &str) -> bool {
        // Acquire read lock - on error, return false
        let Ok(map) = self.paladins.read() else {
            return false;
        };

        map.contains_key(id)
    }

    fn list_ids(&self) -> Vec<String> {
        // Acquire read lock - on error, return empty vec
        let Ok(map) = self.paladins.read() else {
            return Vec::new();
        };

        // Collect all keys into a vector
        map.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::base::entity::Entity;
    use paladin_core::platform::container::paladin::PaladinData;
    use std::thread;

    fn create_test_paladin(id: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("Test paladin {}", id),
            name: id.to_string(),
            ..Default::default()
        };
        Entity::new(data, Some(id.to_string()))
    }

    #[test]
    fn test_registry_register_and_get() {
        let registry = HashMapPaladinRegistry::new();
        let paladin = create_test_paladin("test_paladin");

        // Register
        let result = registry.register("test_paladin".to_string(), Arc::new(paladin));
        assert!(result.is_ok(), "Registration should succeed");

        // Retrieve
        let retrieved = registry.get("test_paladin");
        assert!(retrieved.is_some(), "Paladin should be retrievable");
        assert_eq!(retrieved.unwrap().name, Some("test_paladin".to_string()));
    }

    #[test]
    fn test_registry_duplicate_id_error() {
        let registry = HashMapPaladinRegistry::new();
        let paladin1 = create_test_paladin("duplicate");
        let paladin2 = create_test_paladin("duplicate");

        // First registration succeeds
        assert!(
            registry
                .register("duplicate".to_string(), Arc::new(paladin1))
                .is_ok()
        );

        // Second registration fails
        let result = registry.register("duplicate".to_string(), Arc::new(paladin2));
        assert!(result.is_err(), "Duplicate registration should fail");

        match result.unwrap_err() {
            RegistryError::DuplicateId(id) => assert_eq!(id, "duplicate"),
            _ => panic!("Expected DuplicateId error"),
        }
    }

    #[test]
    fn test_registry_contains() {
        let registry = HashMapPaladinRegistry::new();
        let paladin = create_test_paladin("exists");

        // Before registration
        assert!(!registry.contains("exists"), "Should not exist initially");

        // After registration
        registry
            .register("exists".to_string(), Arc::new(paladin))
            .unwrap();
        assert!(
            registry.contains("exists"),
            "Should exist after registration"
        );
        assert!(
            !registry.contains("nonexistent"),
            "Non-existent should return false"
        );
    }

    #[test]
    fn test_registry_list_ids() {
        let registry = HashMapPaladinRegistry::new();

        // Empty initially
        assert_eq!(registry.list_ids().len(), 0);

        // Add three paladins
        registry
            .register(
                "paladin1".to_string(),
                Arc::new(create_test_paladin("paladin1")),
            )
            .unwrap();
        registry
            .register(
                "paladin2".to_string(),
                Arc::new(create_test_paladin("paladin2")),
            )
            .unwrap();
        registry
            .register(
                "paladin3".to_string(),
                Arc::new(create_test_paladin("paladin3")),
            )
            .unwrap();

        let ids = registry.list_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"paladin1".to_string()));
        assert!(ids.contains(&"paladin2".to_string()));
        assert!(ids.contains(&"paladin3".to_string()));
    }

    #[test]
    fn test_registry_thread_safety() {
        let registry = Arc::new(HashMapPaladinRegistry::new());
        let mut handles = vec![];

        // Spawn 10 threads that concurrently register paladins
        for i in 0..10 {
            let registry_clone = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                let id = format!("paladin_{}", i);
                let paladin = create_test_paladin(&id);
                registry_clone.register(id, Arc::new(paladin)).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all paladins were registered
        assert_eq!(registry.count(), 10);
        for i in 0..10 {
            let id = format!("paladin_{}", i);
            assert!(registry.contains(&id), "Paladin {} should exist", id);
        }
    }

    #[test]
    fn test_registry_invalid_id() {
        let registry = HashMapPaladinRegistry::new();
        let paladin = create_test_paladin("test");

        let result = registry.register("".to_string(), Arc::new(paladin));
        assert!(result.is_err(), "Empty ID should fail");

        match result.unwrap_err() {
            RegistryError::InvalidId(_) => {}
            _ => panic!("Expected InvalidId error"),
        }
    }

    #[test]
    fn test_registry_count() {
        let registry = HashMapPaladinRegistry::new();
        assert_eq!(registry.count(), 0);

        registry
            .register("p1".to_string(), Arc::new(create_test_paladin("p1")))
            .unwrap();
        assert_eq!(registry.count(), 1);

        registry
            .register("p2".to_string(), Arc::new(create_test_paladin("p2")))
            .unwrap();
        assert_eq!(registry.count(), 2);
    }

    #[test]
    fn test_registry_clear() {
        let registry = HashMapPaladinRegistry::new();
        registry
            .register("p1".to_string(), Arc::new(create_test_paladin("p1")))
            .unwrap();
        registry
            .register("p2".to_string(), Arc::new(create_test_paladin("p2")))
            .unwrap();

        assert_eq!(registry.count(), 2);

        registry.clear();
        assert_eq!(registry.count(), 0);
        assert!(!registry.contains("p1"));
        assert!(!registry.contains("p2"));
    }

    #[test]
    fn test_registry_from_map() {
        let mut initial = HashMap::new();
        initial.insert("p1".to_string(), Arc::new(create_test_paladin("p1")));
        initial.insert("p2".to_string(), Arc::new(create_test_paladin("p2")));

        let registry = HashMapPaladinRegistry::from_map(initial);
        assert_eq!(registry.count(), 2);
        assert!(registry.contains("p1"));
        assert!(registry.contains("p2"));
    }
}
