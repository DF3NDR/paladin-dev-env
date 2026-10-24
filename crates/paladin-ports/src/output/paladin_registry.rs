//! Paladin Registry Port - Registry Operations Interface
//!
//! This module defines the port (interface) for Paladin registry operations,
//! following hexagonal architecture principles. The registry provides a mapping
//! from Paladin IDs to actual Paladin instances, enabling multi-agent patterns
//! like Council and Grove to resolve and execute agents dynamically.
//!
//! # Traits
//!
//! - [`PaladinRegistry`]: Basic registry operations (register, get, contains, list)
//!
//! # Example
//!
//! ```rust
//! use paladin_ports::output::paladin_registry::PaladinRegistry;
//! use paladin_core::platform::container::paladin::{PaladinData, Paladin};
//! use paladin_core::base::entity::Entity;
//! use std::sync::Arc;
//!
//! # fn example(registry: impl PaladinRegistry) {
//! // Register a Paladin
//! let paladin_data = PaladinData::default();
//! let paladin = Entity::new(paladin_data, Some("analyzer".to_string()));
//! let result = registry.register("analyzer".to_string(), Arc::new(paladin));
//!
//! // Retrieve a Paladin
//! if let Some(paladin) = registry.get("analyzer") {
//!     println!("Found paladin: {:?}", paladin.name);
//! }
//!
//! // Check if Paladin exists
//! if registry.contains("analyzer") {
//!     println!("Paladin exists in registry");
//! }
//! # }
//! ```

use paladin_core::platform::container::paladin::Paladin;
use std::sync::Arc;

// Re-export pure domain error type from core
pub use paladin_core::platform::container::registry_error::RegistryError;

/// Port for Paladin registry operations
///
/// This trait defines the core interface for storing and retrieving Paladin
/// instances by ID. It enables multi-agent patterns (Council, Grove) to
/// resolve agent IDs to actual executable Paladin instances at runtime.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support concurrent access across
/// async tasks and threads.
///
/// # Example Implementation
///
/// ```rust
/// use paladin_ports::output::paladin_registry::{PaladinRegistry, RegistryError};
/// use paladin_core::platform::container::paladin::Paladin;
/// use std::sync::Arc;
/// use std::collections::HashMap;
/// use std::sync::RwLock;
///
/// pub struct InMemoryRegistry {
///     paladins: RwLock<HashMap<String, Arc<Paladin>>>,
/// }
///
/// impl PaladinRegistry for InMemoryRegistry {
///     fn register(&self, id: String, paladin: Arc<Paladin>) -> Result<(), RegistryError> {
///         let mut map = self.paladins.write().unwrap();
///         if map.contains_key(&id) {
///             return Err(RegistryError::DuplicateId(id));
///         }
///         map.insert(id, paladin);
///         Ok(())
///     }
///
///     fn get(&self, id: &str) -> Option<Arc<Paladin>> {
///         let map = self.paladins.read().unwrap();
///         map.get(id).cloned()
///     }
///
///     fn contains(&self, id: &str) -> bool {
///         let map = self.paladins.read().unwrap();
///         map.contains_key(id)
///     }
///
///     fn list_ids(&self) -> Vec<String> {
///         let map = self.paladins.read().unwrap();
///         map.keys().cloned().collect()
///     }
/// }
/// ```
pub trait PaladinRegistry: Send + Sync {
    /// Register a Paladin with the given ID
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the Paladin
    /// * `paladin` - Arc-wrapped Paladin instance for shared ownership
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Paladin successfully registered
    /// * `Err(RegistryError::DuplicateId)` - ID already exists in registry
    /// * `Err(RegistryError::InvalidId)` - ID is invalid (e.g., empty)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use paladin_ports::output::paladin_registry::PaladinRegistry;
    /// # use paladin_core::platform::container::paladin::{PaladinData, Paladin};
    /// # use paladin_core::base::entity::Entity;
    /// # use std::sync::Arc;
    /// # fn example(registry: impl PaladinRegistry) {
    /// let paladin_data = PaladinData::default();
    /// let paladin = Entity::new(paladin_data, Some("analyzer".to_string()));
    /// let result = registry.register("analyzer".to_string(), Arc::new(paladin));
    /// assert!(result.is_ok());
    /// # }
    /// ```
    fn register(&self, id: String, paladin: Arc<Paladin>) -> Result<(), RegistryError>;

    /// Retrieve a Paladin by ID
    ///
    /// Returns a cloned `Arc<Paladin>` for shared ownership, or `None` if
    /// the Paladin is not found.
    ///
    /// # Arguments
    ///
    /// * `id` - Paladin identifier to lookup
    ///
    /// # Returns
    ///
    /// * `Some(Arc<Paladin>)` - Paladin found in registry
    /// * `None` - Paladin not found
    ///
    /// # Example
    ///
    /// ```rust
    /// # use paladin_ports::output::paladin_registry::PaladinRegistry;
    /// # fn example(registry: impl PaladinRegistry) {
    /// if let Some(paladin) = registry.get("analyzer") {
    ///     println!("Found: {:?}", paladin.name);
    /// } else {
    ///     println!("Paladin not found");
    /// }
    /// # }
    /// ```
    fn get(&self, id: &str) -> Option<Arc<Paladin>>;

    /// Check if a Paladin with the given ID exists
    ///
    /// This is more efficient than calling `get()` if you only need to check
    /// existence without retrieving the Paladin.
    ///
    /// # Arguments
    ///
    /// * `id` - Paladin identifier to check
    ///
    /// # Returns
    ///
    /// * `true` - Paladin exists in registry
    /// * `false` - Paladin not found
    ///
    /// # Example
    ///
    /// ```rust
    /// # use paladin_ports::output::paladin_registry::PaladinRegistry;
    /// # fn example(registry: impl PaladinRegistry) {
    /// if registry.contains("analyzer") {
    ///     println!("Analyzer is registered");
    /// }
    /// # }
    /// ```
    fn contains(&self, id: &str) -> bool;

    /// List all registered Paladin IDs
    ///
    /// Returns a vector of all Paladin IDs currently in the registry.
    /// The order is not guaranteed.
    ///
    /// # Returns
    ///
    /// Vector of Paladin IDs
    ///
    /// # Example
    ///
    /// ```rust
    /// # use paladin_ports::output::paladin_registry::PaladinRegistry;
    /// # fn example(registry: impl PaladinRegistry) {
    /// let ids = registry.list_ids();
    /// println!("Registered Paladins: {:?}", ids);
    /// # }
    /// ```
    fn list_ids(&self) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the trait is object-safe (can be used with `dyn`)
    #[test]
    fn test_trait_is_object_safe() {
        // This test compiles if the trait is object-safe
        fn _assert_object_safe(_registry: &dyn PaladinRegistry) {}
    }

    #[test]
    fn test_registry_error_display() {
        let err = RegistryError::DuplicateId("test_id".to_string());
        assert_eq!(err.to_string(), "Paladin ID already registered: test_id");

        let err = RegistryError::InvalidId("".to_string());
        assert_eq!(err.to_string(), "Invalid Paladin ID: ");

        let err = RegistryError::AccessFailed("lock poisoned".to_string());
        assert_eq!(err.to_string(), "Registry access failed: lock poisoned");
    }
}
