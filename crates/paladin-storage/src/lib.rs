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
