//! # Vault Port — Cross-Thread Namespaced Key/Value Memory (Doc 05 RT-FR-13…16)
//!
//! Port trait defining how the application interacts with the Vault: durable,
//! namespaced key/value storage that outlives any single conversation or run.
//!
//! ## The Vault / Garrison / Waypoint distinction
//!
//! Paladin ships three distinct "memory" concepts, and it is easy to reach
//! for the wrong one:
//!
//! | | Vault | Garrison | Waypoint |
//! |---|---|---|---|
//! | **Scope** | cross-thread namespaced key/value | one conversation's transcript | one run's durable engine state |
//! | **Lifetime** | until explicitly deleted | the conversation | the retention policy |
//! | **Addressed by** | [`Namespace`] + key | a [`GarrisonPort`](crate::output::garrison_port::GarrisonPort) instance | `(ThreadId, WaypointId)` |
//! | **Who writes** | the host, or the agent through a confined tool | the execution service | the `WarEngine` |
//! | **Typical content** | durable facts about a user or a domain | conversation turns | Battlefield snapshots and Frontier state |
//!
//! Reach for the Vault when a fact needs to survive past the end of the
//! conversation or run that produced it and be readable from a different
//! thread later (e.g. "the user's preferred language", set once and read by
//! every future conversation). Reach for the Garrison for the transcript of
//! the current conversation. Reach for a Waypoint only if you are the engine
//! itself checkpointing execution state -- application code almost never
//! writes one directly.
//!
//! ## Hexagonal Architecture
//!
//! This is an **output port** in the application layer. It defines the
//! interface for namespaced key/value operations, allowing Paladin agents
//! and hosts to persist and recall durable facts without depending on a
//! specific storage backend.
//!
//! **Adapter implementations** (`paladin-memory`): `InMemoryVault` (always
//! available), `SqliteVault` (feature `sqlite`), `SemanticVault` (composes a
//! `SanctumPort` + `EmbeddingPort`, ungated).
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync`: a Vault may be read and
//! written concurrently across nodes, threads, and runs.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use paladin_ports::output::vault_port::{Page, VaultPort};
//! use paladin_core::platform::container::vault::Namespace;
//! use serde_json::json;
//!
//! async fn remember_preference(vault: &dyn VaultPort) -> Result<(), Box<dyn std::error::Error>> {
//!     let ns = Namespace::parse("user/alice")?;
//!     vault.put(&ns, "favorite_color", json!("blue")).await?;
//!
//!     let record = vault.get(&ns, "favorite_color").await?;
//!     println!("{:?}", record);
//!
//!     let page = vault.list(&ns, None, Page::default()).await?;
//!     println!("{} records under {}", page.len(), ns);
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

pub use paladin_core::platform::container::vault::{
    Namespace, Page, ScoredVaultRecord, VaultError, VaultRecord,
};

/// Port trait for the Vault: durable, namespaced key/value storage that
/// outlives any single conversation or run.
///
/// # The Vault / Garrison / Waypoint table
///
/// | | Vault | Garrison | Waypoint |
/// |---|---|---|---|
/// | **Scope** | cross-thread namespaced key/value | one conversation's transcript | one run's durable engine state |
/// | **Lifetime** | until explicitly deleted | the conversation | the retention policy |
/// | **Addressed by** | [`Namespace`] + key | a [`GarrisonPort`](crate::output::garrison_port::GarrisonPort) instance | `(ThreadId, WaypointId)` |
/// | **Who writes** | the host, or the agent through a confined tool | the execution service | the `WarEngine` |
/// | **Typical content** | durable facts about a user or a domain | conversation turns | Battlefield snapshots and Frontier state |
///
/// The table's own example -- one snippet naming a [`Namespace`], a
/// [`GarrisonEntry`](paladin_core::platform::container::garrison::GarrisonEntry),
/// and a [`ThreadId`](paladin_core::platform::container::waypoint::ThreadId)
/// -- so the table cannot rot into prose that no longer matches the types:
///
/// ```
/// use paladin_core::platform::container::garrison::{ConversationRole, GarrisonEntry};
/// use paladin_core::platform::container::vault::Namespace;
/// use paladin_core::platform::container::waypoint::ThreadId;
///
/// // Vault: addressed by (Namespace, key).
/// let vault_address = Namespace::parse("user/alice")?;
///
/// // Garrison: addressed by a GarrisonEntry within one conversation instance.
/// let garrison_entry = GarrisonEntry::new(ConversationRole::User, "hi".to_string());
///
/// // Waypoint: addressed by ThreadId (+ WaypointId, not shown here).
/// let waypoint_thread = ThreadId::new("run-42").unwrap();
///
/// assert_eq!(vault_address.to_string(), "user/alice");
/// assert_eq!(garrison_entry.content, "hi");
/// assert_eq!(waypoint_thread.to_string(), "run-42");
/// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
/// ```
///
/// # Object Safety
///
/// `VaultPort` is object-safe (`Arc<dyn VaultPort>`), which the `ConfinedVault`
/// decorator (plan 26-13) and `SemanticVault` composer (plan 26-09) both
/// require.
#[async_trait]
pub trait VaultPort: Send + Sync {
    /// Stores `value` under `(ns, key)`.
    ///
    /// Overwrites and bumps `updated_at` on an existing `(ns, key)`, while
    /// preserving `created_at` -- a second `put` on the same address never
    /// resets when the record was first created.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::InvalidKey`] if `key` fails validation.
    /// Returns [`VaultError::ValueTooLarge`] if `value` exceeds the backend's
    /// configured bound. Returns [`VaultError::Storage`] if the backend
    /// itself fails.
    async fn put(
        &self,
        ns: &Namespace,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), VaultError>;

    /// Retrieves the record at `(ns, key)`, if any.
    ///
    /// `Ok(None)` is the expected, normal result for a key that was never
    /// written -- never treated as an error.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Storage`] if the backend itself fails.
    async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError>;

    /// Deletes the record at `(ns, key)`, returning whether a record was
    /// actually removed.
    ///
    /// A second `delete` of the same key is `Ok(false)`, never an error --
    /// deleting something already absent is not a failure.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Storage`] if the backend itself fails.
    async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError>;

    /// Lists records in **exactly** `ns` -- never its descendants -- ordered
    /// by key, optionally filtered to keys starting with `prefix`, and
    /// paginated by `page`.
    ///
    /// Stating "exactly `ns`, not descendants" here means every adapter
    /// inherits this rule rather than each backend choosing its own
    /// recursion behavior. A namespace with no records returns an empty
    /// page, never an error.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Storage`] if the backend itself fails.
    async fn list(
        &self,
        ns: &Namespace,
        prefix: Option<&str>,
        page: Page,
    ) -> Result<Vec<VaultRecord>, VaultError>;

    /// Searches `ns` for records relevant to `query`, returning up to
    /// `limit` results ranked by relevance.
    ///
    /// The default implementation returns
    /// [`VaultError::Unsupported`] naming `"search"` -- a *correct* default
    /// (it claims no capability it does not have), so a backend without
    /// embeddings (e.g. `InMemoryVault`, `SqliteVault`) does not have to
    /// write boilerplate to reject a call it cannot serve.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Unsupported`] if the backend has no search
    /// capability. Returns [`VaultError::Storage`] if a backend that does
    /// support search fails.
    async fn search(
        &self,
        ns: &Namespace,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ScoredVaultRecord>, VaultError> {
        // RED (deliberate, temporary): returns an empty success instead of
        // the correct Unsupported default, confirmed failing against
        // `search_defaults_to_unsupported` before being replaced with the
        // real default in the GREEN commit.
        let _ = (ns, query, limit);
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock implementation for testing trait bounds (mirrors
    // node_cache_port.rs's MockNodeCache fixture).
    struct MockVault;

    #[async_trait]
    impl VaultPort for MockVault {
        async fn put(
            &self,
            _ns: &Namespace,
            _key: &str,
            _value: serde_json::Value,
        ) -> Result<(), VaultError> {
            Ok(())
        }

        async fn get(
            &self,
            _ns: &Namespace,
            _key: &str,
        ) -> Result<Option<VaultRecord>, VaultError> {
            Ok(None)
        }

        async fn delete(&self, _ns: &Namespace, _key: &str) -> Result<bool, VaultError> {
            Ok(false)
        }

        async fn list(
            &self,
            _ns: &Namespace,
            _prefix: Option<&str>,
            _page: Page,
        ) -> Result<Vec<VaultRecord>, VaultError> {
            Ok(vec![])
        }
    }

    /// Test 1: a `fn takes_dyn(_: Arc<dyn VaultPort>) {}` compiles --
    /// object safety is what `ConfinedVault` (plan 26-13) and
    /// `SemanticVault` (plan 26-09) both require.
    fn takes_dyn(_: Arc<dyn VaultPort>) {}

    #[test]
    fn vault_port_is_object_safe() {
        let vault: Arc<dyn VaultPort> = Arc::new(MockVault);
        takes_dyn(vault);
    }

    /// Test 2: a static assertion that `dyn VaultPort: Send + Sync` (X-05).
    #[test]
    fn vault_port_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn VaultPort>();
    }

    #[tokio::test]
    async fn search_defaults_to_unsupported() {
        let vault = MockVault;
        let ns = Namespace::parse("user/alice").unwrap();
        let err = vault.search(&ns, "query", 5).await.unwrap_err();
        assert!(matches!(
            err,
            VaultError::Unsupported {
                operation: "search"
            }
        ));
    }

    #[tokio::test]
    async fn mock_vault_implements_trait() {
        let vault = MockVault;
        let ns = Namespace::parse("user/alice").unwrap();
        assert!(vault.get(&ns, "k").await.unwrap().is_none());
        assert!(vault.put(&ns, "k", serde_json::json!(1)).await.is_ok());
        assert!(!vault.delete(&ns, "k").await.unwrap());
        assert!(
            vault
                .list(&ns, None, Page::default())
                .await
                .unwrap()
                .is_empty()
        );
    }
}
