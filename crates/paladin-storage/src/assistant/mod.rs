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

// `sqlite`/`postgres` adapter modules are declared by Task 3, once those
// files exist -- declaring a feature-gated `mod` before its file exists
// makes `rustfmt`/`cargo check` fail to resolve it regardless of `cfg`
// gating (the identical Task-boundary lesson `crate::run::mod`'s own
// history records for plan 27-02).
