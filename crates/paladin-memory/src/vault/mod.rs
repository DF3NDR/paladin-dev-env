//! Vault adapters -- cross-thread namespaced key/value storage (D-18).
//!
//! Implementations of `paladin_ports::output::vault_port::VaultPort`.
//!
//! - `InMemoryVault` -- always available, ungated.
//! - `SqliteVault` (feature `sqlite`) -- persistent SQLite-backed store,
//!   riding the crate's one shared embedded migrator (D-23).
//! - `semantic` (plan 26-09) -- `SemanticVault`, composing a `SanctumPort` +
//!   `EmbeddingPort`, ungated; not yet implemented.
//!
//! `contract_tests` is the shared specification every adapter above is
//! instantiated against -- a new adapter is not "done" until it appears
//! there too.

pub mod in_memory;
pub use in_memory::InMemoryVault;

#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteVault;

/// Generic, credential-shape redaction for Vault adapter-boundary error
/// text (D-34), shared by `SqliteVault` and (plan 26-09's) `SemanticVault`.
pub(crate) mod redact;

/// Shared `VaultPort` contract suite (mirroring `paladin-storage`'s
/// `node_cache::contract_tests` D-27 precedent): one generic async function
/// per contract clause, run unchanged by every adapter's own tests.
pub mod contract_tests;
