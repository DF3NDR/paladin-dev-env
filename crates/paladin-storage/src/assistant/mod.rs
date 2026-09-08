//! Assistant storage adapters.
//!
//! Implementations of
//! `paladin_ports::output::assistant_repository_port::AssistantRepositoryPort`,
//! mirroring `crate::run`'s module layout (D-03, D-28, D-29).

/// In-memory implementation, always available (no feature gate): used for
/// tests, local development, and every InMemory-only wiring.
pub mod in_memory;

/// Shared `AssistantRepositoryPort` contract suite (D-28, D-29, D-30),
/// mirroring `crate::run::contract_tests`. Plain module (not `#[cfg(test)]`)
/// so both unit tests inside each backend crate and future integration
/// tests can call it.
pub mod contract_tests;

/// SQLite implementation, behind the `sqlite` feature (D-03).
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// PostgreSQL implementation, behind the `postgres` feature (D-03, Tier 2).
#[cfg(feature = "postgres")]
pub mod postgres;
