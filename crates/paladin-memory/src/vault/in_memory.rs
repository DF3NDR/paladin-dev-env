//! In-memory `VaultPort` implementation (D-18).
//!
//! Always available -- no feature gate, mirroring `garrison::InMemoryGarrison`'s
//! and `paladin-storage`'s `node_cache::InMemoryNodeCache`'s ungated
//! precedent. Used for tests, local development, and any deployment that
//! has not opted into a durable Vault backend (`SqliteVault`, plan 26-09).

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use paladin_core::platform::container::vault::{DEFAULT_MAX_VALUE_BYTES, Namespace, VaultRecord};
use paladin_ports::output::vault_port::{Page, VaultError, VaultPort};
use tokio::sync::RwLock;

/// In-memory `VaultPort` implementation.
///
/// Storage is `RwLock<HashMap<String, BTreeMap<String, VaultRecord>>>`: the
/// outer key is the `/`-joined namespace (safe because a `Namespace`
/// segment can never contain `/`, so two different namespaces can never
/// collide on their joined form), and the inner map is a `BTreeMap` so
/// `list`'s key-ascending ordering is structural -- a property of the data
/// structure itself -- rather than a sort performed at read time.
///
/// # Examples
///
/// ```no_run
/// use paladin_memory::vault::InMemoryVault;
/// use paladin_core::platform::container::vault::Namespace;
/// use paladin_ports::output::vault_port::VaultPort;
/// use serde_json::json;
///
/// #[tokio::main]
/// async fn main() {
///     let vault = InMemoryVault::new();
///     let ns = Namespace::parse("user/alice").unwrap();
///     vault.put(&ns, "favorite_color", json!("blue")).await.unwrap();
///     let record = vault.get(&ns, "favorite_color").await.unwrap();
///     assert!(record.is_some());
/// }
/// ```
pub struct InMemoryVault {
    entries: RwLock<HashMap<String, BTreeMap<String, VaultRecord>>>,
    max_value_bytes: usize,
}

impl InMemoryVault {
    /// Constructs a new, empty `InMemoryVault` with the default
    /// [`DEFAULT_MAX_VALUE_BYTES`] value-size bound.
    pub fn new() -> Self {
        Self::with_max_value_bytes(DEFAULT_MAX_VALUE_BYTES)
    }

    /// Constructs a new, empty `InMemoryVault` with an explicit
    /// `max_value_bytes` bound, overriding [`DEFAULT_MAX_VALUE_BYTES`].
    pub fn with_max_value_bytes(max_value_bytes: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_value_bytes,
        }
    }
}

impl Default for InMemoryVault {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VaultPort for InMemoryVault {
    async fn put(
        &self,
        ns: &Namespace,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), VaultError> {
        let ns_key = ns.to_string();
        let mut entries = self.entries.write().await;
        let namespace_map = entries.entry(ns_key).or_default();

        if let Some(existing) = namespace_map.get_mut(key) {
            existing.overwrite_value_with_bound(value, self.max_value_bytes)?;
        } else {
            let record = VaultRecord::new_with_bound(ns.clone(), key, value, self.max_value_bytes)?;
            namespace_map.insert(key.to_string(), record);
        }
        Ok(())
    }

    async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError> {
        let entries = self.entries.read().await;
        Ok(entries
            .get(&ns.to_string())
            .and_then(|namespace_map| namespace_map.get(key))
            .cloned())
    }

    async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError> {
        let mut entries = self.entries.write().await;
        Ok(entries
            .get_mut(&ns.to_string())
            .map(|namespace_map| namespace_map.remove(key).is_some())
            .unwrap_or(false))
    }

    async fn list(
        &self,
        ns: &Namespace,
        prefix: Option<&str>,
        page: Page,
    ) -> Result<Vec<VaultRecord>, VaultError> {
        let entries = self.entries.read().await;
        let Some(namespace_map) = entries.get(&ns.to_string()) else {
            return Ok(vec![]);
        };

        let filtered = namespace_map
            .iter()
            .filter(|(key, _)| prefix.is_none_or(|p| key.starts_with(p)))
            // Skip up to and including the opaque `after` cursor -- the
            // last key returned by the previous page.
            .skip_while(|(key, _)| page.after().is_some_and(|after| key.as_str() <= after));

        let records: Vec<VaultRecord> = filtered
            .take(page.limit() as usize)
            .map(|(_, record)| record.clone())
            .collect();

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::contract_tests;

    // One #[tokio::test] per shared contract function, each against a
    // fresh vault, mirroring `node_cache::in_memory`'s own test module
    // shape exactly.

    #[tokio::test]
    async fn put_then_get_returns_the_record() {
        contract_tests::put_then_get_returns_the_record(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn put_overwrites_and_preserves_created_at() {
        contract_tests::put_overwrites_and_preserves_created_at(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn delete_returns_true_then_false() {
        contract_tests::delete_returns_true_then_false(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn list_returns_only_this_namespace_ordered_by_key() {
        contract_tests::list_returns_only_this_namespace_ordered_by_key(&InMemoryVault::new())
            .await;
    }

    #[tokio::test]
    async fn list_filters_by_key_prefix() {
        contract_tests::list_filters_by_key_prefix(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn list_paginates_by_opaque_after_cursor() {
        contract_tests::list_paginates_by_opaque_after_cursor(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn list_on_an_empty_namespace_is_an_empty_page_not_an_error() {
        contract_tests::list_on_an_empty_namespace_is_an_empty_page_not_an_error(
            &InMemoryVault::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn namespaces_are_isolated() {
        contract_tests::namespaces_are_isolated(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn value_larger_than_the_bound_is_rejected() {
        contract_tests::value_larger_than_the_bound_is_rejected(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn search_is_unsupported_on_in_memory() {
        contract_tests::assert_search_unsupported(&InMemoryVault::new()).await;
    }

    #[tokio::test]
    async fn run_all_shared_clauses_smoke_aggregate() {
        contract_tests::run_all_shared_clauses(&InMemoryVault::new()).await;
    }
}
