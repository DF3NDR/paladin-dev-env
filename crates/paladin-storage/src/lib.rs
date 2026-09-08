//! # paladin-storage
//!
//! Persistence adapters for the Paladin multi-agent framework: SQL-backed repositories
//! plus MinIO/S3 file storage.
//!
//! ## Feature flags
//!
//! | Flag | Enables |
//! |------|---------|
//! | `sqlite` | [`sqlite_content_repository`], [`sqlite_user_repository`] |
//! | `mysql`  | [`mysql_content_repository`] |
//! | `s3`     | [`minio`] (MinIO / S3 file storage) |
//! | `redis-queue` | [`redis`] (Redis-backed queue) |
//! | `scheduler` | [`scheduler`] (`tokio-cron-scheduler`-backed `SchedulerPort`) |
//!
//! Enable only the backends your deployment actually uses.

#![warn(missing_docs)]
#![allow(rustdoc::broken_intra_doc_links)]

/// `WaypointPort` storage adapters. The in-memory backend is always
/// available (D-01, no feature gate); durable backends are added by later
/// plans behind their own feature flags.
pub mod waypoint;

/// `NodeCachePort` storage adapters (Doc 04 FT-FR-18…20, D-27). The
/// in-memory backend is always available (no feature gate, mirroring
/// `waypoint`'s D-01 precedent); the Redis backend is added behind the
/// `redis-cache` feature.
pub mod node_cache;

/// SQLite implementation of `ContentRepository`, `ContentListRepository`,
/// `MigrationManager`, and `SqlStore`.
#[cfg(feature = "sqlite")]
pub mod sqlite_content_repository;

/// SQLite implementation of `UserRepositoryPort`.
#[cfg(feature = "sqlite")]
pub mod sqlite_user_repository;

/// SQLite implementation of `WorkflowRepositoryPort`.
#[cfg(feature = "sqlite")]
pub mod sqlite_workflow_repository;

/// MySQL implementation of `ContentRepository`, `ContentListRepository`,
/// `MigrationManager`, and `SqlStore`.
#[cfg(feature = "mysql")]
pub mod mysql_content_repository;

/// MinIO / S3 implementation of `FileStoragePort`.
#[cfg(feature = "s3")]
pub mod minio;

/// Redis implementation of `QueuePort`.
#[cfg(feature = "redis-queue")]
pub mod redis;

/// `tokio-cron-scheduler` implementation of `SchedulerPort`.
#[cfg(feature = "scheduler")]
pub mod scheduler;

/// `AssistantRepositoryPort` storage adapters (D-28, D-29). The in-memory
/// backend is always available (no feature gate, mirroring `waypoint`'s
/// D-01 precedent); durable backends are added by plan 27-09 Task 3 behind
/// the existing `sqlite`/`postgres` features.
pub mod assistant;

/// `RunRepositoryPort` storage adapters (D-03). The in-memory backend is
/// always available (no feature gate, mirroring `waypoint`'s D-01
/// precedent); durable backends are added by later plans behind the
/// existing `sqlite`/`postgres` features.
pub mod run;

/// `RunQueuePort` storage adapters (D-06). The in-memory backend is always
/// available (no feature gate); the Redis ZSET+Lua lease backend is added
/// by a later plan behind the existing `redis-queue` feature.
pub mod run_queue;

/// Cron parsing for run schedules (D-38): `parse_run_cron` (5- and 6-field
/// forms via `croner`, IANA timezones via `chrono-tz`) and
/// `cron_field_count`, the counting primitive `scheduler.rs`'s own
/// six-field `validate_cron_field_count` delegates to. Always compiled (no
/// feature gate) -- `run_schedule::in_memory` needs it with no
/// `sqlite`/`postgres` feature enabled.
pub mod cron;

/// `RunScheduleRepositoryPort` storage adapters (D-36, D-37). The in-memory
/// backend is always available (no feature gate, mirroring `run`'s D-03
/// precedent); SQLite and Postgres adapters are added behind the existing
/// `sqlite`/`postgres` features.
pub mod run_schedule;

/// `WebhookDeliveryRepositoryPort` storage adapters (D-40). The in-memory
/// backend is always available (no feature gate, mirroring `run_schedule`'s
/// D-03 precedent); SQLite and Postgres adapters are added behind the
/// existing `sqlite`/`postgres` features.
pub mod webhook;
