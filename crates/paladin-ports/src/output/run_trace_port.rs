//! # Run Trace Port — Durable, Per-Record Trace Persistence (OBS-02, D-17)
//!
//! This module defines the port trait for durably persisting and reading
//! back [`TraceRecord`]s -- the per-event observability envelope a
//! `TraceDispatcher` (`paladin-battalion::engine::hooks`) stamps and fans
//! out through a
//! [`CompositeSink`](crate::output::trace_sink_port::CompositeSink).
//! Persisting through this port is what upgrades the SSE facade's degraded
//! mode to full-fidelity replay and gives the execution overlay exact
//! fired/evaluated edge sets in later Phase 28 plans.
//!
//! ## Missing is empty, not an error
//!
//! Following [`WaypointPort`](crate::output::waypoint_port::WaypointPort)'s
//! contract: [`RunTracePort::read`] on a thread with no persisted rows
//! returns `Ok(vec![])`. A brand-new or never-traced thread is the
//! expected, normal case, never an error on its own.
//!
//! ## Trace persistence is best-effort; the Waypoint is the durability truth
//!
//! A `run_traces` row is a REPLAY convenience -- a durable copy of what a
//! live sink already saw, opt-in via a `TraceConfig.persist` switch. It is
//! never the run's own correctness or resumability boundary: that is
//! [`Waypoint`](paladin_core::platform::container::waypoint::Waypoint),
//! persisted through
//! [`WaypointPort`](crate::output::waypoint_port::WaypointPort) on every
//! superstep regardless of whether tracing is enabled at all. A backend
//! that loses `run_traces` rows degrades replay fidelity; it never loses the
//! ability to resume a thread.
//!
//! ## `ThreadId` is not an authorization boundary
//!
//! Same caveat as `WaypointPort`'s: [`ThreadId`] is a caller-supplied
//! workflow identifier, not a capability token or tenancy key. Any code
//! exposing [`RunTracePort::read`] over a network must add its own
//! authorization layer in front of it; this port neither performs nor
//! implies one.
//!
//! ## The table is append-only (D-17)
//!
//! [`RunTracePort::append`]'s `(thread_id, seq)` primary key makes a
//! retried append of the same record idempotent, never a duplicate row (see
//! `append`'s own rustdoc). There is no `update` or per-record `delete`
//! method on this port -- [`prune_thread`](RunTracePort::prune_thread) is
//! the only removal path, and it removes by age (a superstep boundary),
//! never by record identity.
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync`: records may be appended and
//! read back concurrently across nodes, superstep iterations, and runs.

use async_trait::async_trait;
use thiserror::Error;

pub use paladin_core::platform::container::trace::TraceRecord;
use paladin_core::platform::container::waypoint::ThreadId;

/// Errors that can occur while appending or reading back [`TraceRecord`]s.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RunTraceError {
    /// The underlying storage backend failed.
    #[error("run trace backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) `TraceRecord` could not be (de)serialized.
    #[error("run trace serialization error: {source}")]
    Serialization {
        /// The underlying (de)serialization error.
        #[source]
        source: serde_json::Error,
    },
    /// A stored row carries a schema version this build does not know how
    /// to read (X-04).
    #[error("unsupported run trace schema version: found {found}")]
    UnsupportedSchemaVersion {
        /// The schema version found on the stored row.
        found: String,
    },
}

/// Port trait for durably persisting and reading back [`TraceRecord`]s
/// (OBS-02, D-17).
///
/// # Purpose
///
/// Gives a persisting `TraceSink` and the replay path one storage-agnostic
/// interface for writing a batch of records and reading them back,
/// paginated by `seq`, without depending on whether the backend is an
/// in-process `HashMap`, SQLite, or Postgres.
///
/// # Hexagonal Architecture Context
///
/// ```text
/// ┌─────────────────────────────────────────────────────┐
/// │        A persisting TraceSink (paladin-battalion)    │
/// │   - appends a batch of TraceRecords per flush         │
/// │   - a replay consumer reads them back, paginated      │
/// └───────────────────────┬───────────────────────────────┘
///                         │
///                         ▼
/// ┌─────────────────────────────────────────────────────┐
/// │              RunTracePort (this module)              │
/// └───────────────────────┬───────────────────────────────┘
///                         │
///                         ▼
/// ┌─────────────────────────────────────────────────────┐
/// │  InMemoryRunTraceStore | SqliteRunTraceStore |       │
/// │  PostgresRunTraceStore   (all paladin-storage)       │
/// └─────────────────────────────────────────────────────┘
/// ```
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: records may be appended and read
/// back concurrently across nodes, superstep iterations, and runs.
///
/// # Error Handling
///
/// A missing thread is `Ok(vec![])`, never an error on its own -- see the
/// module-level "Missing is empty" section. [`RunTraceError`] is reserved
/// for genuine backend failures: connection errors, (de)serialization
/// failures, and a schema version the running build does not know how to
/// read.
#[async_trait]
pub trait RunTracePort: Send + Sync {
    /// Append a batch of [`TraceRecord`]s.
    ///
    /// The table is append-only, keyed `(thread_id, seq)`: appending a
    /// record whose `(thread_id, seq)` already exists is a **no-op**, not
    /// an error and not a duplicate row -- so a caller that retries a
    /// partially failed batch (e.g. after a transient backend error) can
    /// safely resend the whole batch, including records that already
    /// landed.
    async fn append(&self, records: &[TraceRecord]) -> Result<(), RunTraceError>;

    /// Read a thread's persisted records with `seq` strictly greater than
    /// `after_seq`, ascending by `seq`, capped at `limit`.
    ///
    /// Returns `Ok(vec![])` for a thread with no persisted rows -- a
    /// brand-new or never-traced thread is the expected, normal case, never
    /// an error. Calling this repeatedly with the last returned record's
    /// `seq` as the next call's `after_seq` walks the whole run exactly
    /// once, with no repeats and no gaps.
    async fn read(
        &self,
        thread: &ThreadId,
        after_seq: u64,
        limit: u32,
    ) -> Result<Vec<TraceRecord>, RunTraceError>;

    /// Remove every persisted record of `thread` whose superstep is
    /// strictly less than `before_superstep`. Returns the number of
    /// records removed.
    ///
    /// An unknown `thread` returns `Ok(0)`, not an error -- pruning a
    /// thread that never persisted anything is a no-op, not a failure.
    async fn prune_thread(
        &self,
        thread: &ThreadId,
        before_superstep: u64,
    ) -> Result<u64, RunTraceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing trait bounds (mirrors waypoint_port.rs's
    // MockWaypointStore fixture).
    struct MockRunTraceStore;

    #[async_trait]
    impl RunTracePort for MockRunTraceStore {
        async fn append(&self, _records: &[TraceRecord]) -> Result<(), RunTraceError> {
            Ok(())
        }

        async fn read(
            &self,
            _thread: &ThreadId,
            _after_seq: u64,
            _limit: u32,
        ) -> Result<Vec<TraceRecord>, RunTraceError> {
            Ok(vec![])
        }

        async fn prune_thread(
            &self,
            _thread: &ThreadId,
            _before_superstep: u64,
        ) -> Result<u64, RunTraceError> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn mock_store_implements_trait() {
        let store = MockRunTraceStore;
        let thread = ThreadId::new("t1").unwrap();
        assert_eq!(store.read(&thread, 0, 100).await.unwrap(), vec![]);
        assert_eq!(store.prune_thread(&thread, 5).await.unwrap(), 0);
        assert!(store.append(&[]).await.is_ok());
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Box<dyn RunTracePort>> = None;
    }

    #[test]
    fn unsupported_schema_version_error_carries_found() {
        let err = RunTraceError::UnsupportedSchemaVersion {
            found: "999".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "unsupported run trace schema version: found 999"
        );
    }

    #[test]
    fn backend_error_display_includes_source() {
        let err = RunTraceError::Backend {
            source: "boom".into(),
        };
        assert!(err.to_string().contains("boom"));
    }
}
