//! Vault adapters -- cross-thread namespaced key/value storage (D-18).
//!
//! Implementations of `paladin_ports::output::vault_port::VaultPort`.
//!
//! - [`in_memory::InMemoryVault`] -- always available, ungated (this plan).
//! - `sqlite` (plan 26-09) -- persistent SQLite-backed store behind the
//!   existing `sqlite` feature; not yet implemented.
//! - `semantic` (plan 26-09) -- `SemanticVault`, composing a `SanctumPort` +
//!   `EmbeddingPort`, ungated; not yet implemented.
//!
//! [`contract_tests`] is the shared specification every adapter above is
//! instantiated against -- a new adapter is not "done" until it appears
//! there too.

pub mod in_memory;
pub use in_memory::InMemoryVault;

/// Shared `VaultPort` contract suite (mirroring `paladin-storage`'s
/// `node_cache::contract_tests` D-27 precedent): one generic async function
/// per contract clause, run unchanged by every adapter's own tests.
pub mod contract_tests;
