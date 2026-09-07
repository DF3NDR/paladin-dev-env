//! Vault adapters -- cross-thread namespaced key/value storage (D-18).
//!
//! Implementations of `paladin_ports::output::vault_port::VaultPort`.
//!
//! - `InMemoryVault` -- always available, ungated.
//! - `SqliteVault` (feature `sqlite`) -- persistent SQLite-backed store,
//!   riding the crate's one shared embedded migrator (D-23).
//! - `SemanticVault` -- composes a `SanctumPort` + `EmbeddingPort`, ungated
//!   (D-24); gives `search` a real implementation.
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

pub mod semantic;
pub use semantic::SemanticVault;

/// Generic, credential-shape redaction for Vault adapter-boundary error
/// text (D-34), shared by `SqliteVault` and `SemanticVault`.
pub(crate) mod redact;

/// Shared `VaultPort` contract suite (mirroring `paladin-storage`'s
/// `node_cache::contract_tests` D-27 precedent): one generic async function
/// per contract clause, run unchanged by every adapter's own tests.
pub mod contract_tests;
