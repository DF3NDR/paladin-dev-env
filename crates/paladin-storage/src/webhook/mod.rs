//! Webhook delivery storage adapters.
//!
//! Implementations of
//! `paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort`,
//! mirroring `crate::run_schedule`'s module layout (D-03, D-40).

/// In-memory implementation, always available (no feature gate): used for
/// tests, local development, and every InMemory-only wiring.
pub mod in_memory;

/// Shared `WebhookDeliveryRepositoryPort` contract suite (D-40, D-43),
/// mirroring `crate::run_schedule::contract_tests`. Plain module (not
/// `#[cfg(test)]`) so both unit tests inside each backend crate and future
/// integration tests can call it.
pub mod contract_tests;

/// SQLite implementation, behind the `sqlite` feature (D-03).
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// PostgreSQL implementation, behind the `postgres` feature (D-03, Tier 2).
#[cfg(feature = "postgres")]
pub mod postgres;
