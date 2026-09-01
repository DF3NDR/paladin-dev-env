//! # Waypoint Port — Superstep Checkpoint Persistence
//!
//! This module defines the port trait for persisting and reading back
//! [`Waypoint`]s, the automatic per-superstep checkpoints written by the
//! `WarEngine` (`paladin-battalion`'s `engine` module).
//!
//! ## Why this is separate from `CitadelPort`
//!
//! [`CitadelPort`](crate::output::citadel_port::CitadelPort) persists coarse,
//! whole-entity snapshots (a Paladin's or Battalion's full state), written
//! occasionally and explicitly. Waypoints are the opposite shape: they are
//! written automatically, **once per superstep** — potentially many times a
//! second in a tight cyclic graph — always addressed by a caller-supplied
//! [`ThreadId`], and read back append-mostly (latest, history, or a specific
//! id) rather than replaced in place. Folding these into `CitadelPort` would
//! force every Citadel backend to also support high-frequency, thread-scoped
//! append semantics it does not otherwise need. Keeping them separate lets
//! each port's backends specialize: an `InMemoryWaypointStore` for tests, a
//! `SqliteWaypointStore` for single-instance durability, and so on.
//!
//! ## Missing is `None`, not an error
//!
//! Following [`CitadelPort`](crate::output::citadel_port::CitadelPort)'s
//! contract: [`WaypointPort::latest`] and [`WaypointPort::get`] return
//! `Ok(None)` when nothing is found. A missing waypoint is the expected,
//! normal case for a brand-new thread — it is never an error on its own.
//! [`resume`](https://github.com/DF3NDR/paladin-dev-env) (in the engine) is
//! what turns "no waypoint for this thread" into `EngineError::ThreadNotFound`,
//! at the layer that knows whether that absence is meaningful.
//!
//! ## `ThreadId` is not an authorization boundary
//!
//! [`ThreadId`] is a caller-supplied workflow identifier, not a capability
//! token or a tenancy key. [`WaypointPort::list_threads`] enumerates **every
//! thread the backend holds**, with no per-caller filtering — any code that
//! exposes these methods over a network (a later HTTP epic) must add its own
//! authorization layer in front of them; this port neither performs nor
//! implies one. Treating a `ThreadId` as if it already gated access would be
//! a security defect in the caller, not a missing feature here.
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync`: Waypoints may be saved and
//! read back concurrently across nodes, superstep iterations, and runs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId, WaypointStatus};

/// Errors that can occur while saving or reading back Waypoints.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WaypointError {
    /// The underlying storage backend failed.
    #[error("waypoint backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A stored (or to-be-stored) Waypoint could not be (de)serialized.
    #[error("waypoint serialization error: {0}")]
    Serialization(String),
    /// A stored Waypoint carries a schema version this build does not know
    /// how to read.
    #[error("unsupported waypoint schema version: found {found}, supported {supported}")]
    SchemaVersionUnsupported {
        /// The schema version found on the stored data.
        found: String,
        /// The schema version(s) this build supports.
        supported: String,
    },
    /// The requested waypoint or thread does not exist.
    #[error("waypoint not found: {0}")]
    NotFound(String),
}

/// A lightweight summary of one `Waypoint`, returned by
/// [`WaypointPort::history`] instead of the full (potentially large)
/// `Battlefield` snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaypointSummary {
    /// The summarized waypoint's identity.
    pub waypoint_id: WaypointId,
    /// The waypoint this one was checkpointed from.
    pub parent_waypoint_id: Option<WaypointId>,
    /// The superstep index that produced this waypoint.
    pub superstep: u64,
    /// The waypoint's status.
    pub status: WaypointStatus,
    /// When this waypoint was created.
    pub created_at: DateTime<Utc>,
}

/// A lightweight summary of one thread, returned by
/// [`WaypointPort::list_threads`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadSummary {
    /// The summarized thread's identity.
    pub thread_id: ThreadId,
    /// The status of the thread's latest waypoint.
    pub latest_status: WaypointStatus,
    /// When the thread's latest waypoint was created.
    pub last_updated_at: DateTime<Utc>,
}

/// Port trait for persisting and reading back superstep checkpoints.
///
/// # Purpose
///
/// Gives the `WarEngine` (and anything reading its history) one storage-
/// agnostic interface for writing a `Waypoint` after every superstep and
/// reading them back — by thread's latest, by specific id, as a paginated
/// history, or as a cross-thread listing — without depending on whether the
/// backend is an in-process `HashMap`, SQLite, or Postgres.
///
/// # Hexagonal Architecture Context
///
/// ```text
/// ┌─────────────────────────────────────────────────────┐
/// │                 WarEngine (paladin-battalion)        │
/// │   - saves one Waypoint per completed superstep       │
/// │   - reads `latest` to resume a thread                │
/// └───────────────────────┬───────────────────────────────┘
///                         │
///                         ▼
/// ┌─────────────────────────────────────────────────────┐
/// │              WaypointPort (this module)              │
/// └───────────────────────┬───────────────────────────────┘
///                         │
///                         ▼
/// ┌─────────────────────────────────────────────────────┐
/// │  InMemoryWaypointStore | SqliteWaypointStore |       │
/// │  PostgresWaypointStore   (all paladin-storage)       │
/// └─────────────────────────────────────────────────────┘
/// ```
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: Waypoints may be saved and read
/// back concurrently across nodes, superstep iterations, and runs.
///
/// # Error Handling
///
/// A missing waypoint or thread is `Ok(None)` / an empty `Vec`, never an
/// error on its own — see the module-level "Missing is `None`" section.
/// [`WaypointError`] is reserved for genuine backend failures: connection
/// errors, (de)serialization failures, and a schema version the running
/// build does not know how to read.
#[async_trait]
pub trait WaypointPort: Send + Sync {
    /// Persist a Waypoint.
    ///
    /// Waypoints are immutable once written and `save` is an append: a
    /// thread's history is the ordered sequence of every `save`d waypoint
    /// for that `thread_id`. Re-saving a `waypoint_id` that already exists
    /// is an **upsert** — the existing row is replaced with the new payload
    /// rather than rejected — and every backend (`InMemoryWaypointStore`,
    /// `SqliteWaypointStore`, `PostgresWaypointStore`) must implement this
    /// identically, since callers must be able to retry a `save` after a
    /// transient failure without first checking whether it partially landed.
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError>;

    /// Load the most recently saved Waypoint for `thread`.
    ///
    /// Returns `Ok(None)` if the thread has no waypoints yet — a brand-new
    /// thread is the expected, normal case, never an error. `NotFound` is
    /// reserved for a backend that has lost a row it previously
    /// acknowledged, which this method never signals.
    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError>;

    /// Load a specific Waypoint by id within `thread`.
    ///
    /// Returns `Ok(None)` if `thread` is unknown, or if `thread` is known but
    /// `id` does not identify one of its waypoints — both are the normal
    /// "nothing here" case, not an error.
    async fn get(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<Option<Waypoint>, WaypointError>;

    /// List a thread's waypoint history, newest-first, paginated.
    ///
    /// Ordering is **descending `created_at`**, with **descending
    /// `superstep` as the tiebreak** when two waypoints of the same thread
    /// share a `created_at` timestamp — so the ordering is total and stable:
    /// repeated calls against unchanged data return the identical sequence.
    ///
    /// `limit` caps the number of summaries returned; `Some(0)` returns an
    /// empty `Vec` (not the whole thread — zero means zero), and `None`
    /// means unbounded. `before` is an **exclusive** cursor: passing the
    /// `waypoint_id` of the oldest summary from a previous page returns the
    /// next older page with no overlap and no gap. An unknown `thread`
    /// returns an empty `Vec`, not an error.
    async fn history(
        &self,
        thread: &ThreadId,
        limit: Option<u32>,
        before: Option<WaypointId>,
    ) -> Result<Vec<WaypointSummary>, WaypointError>;

    /// List known threads, newest-activity-first, paginated.
    ///
    /// Ordering is descending `last_updated_at` (the `created_at` of each
    /// thread's latest waypoint). `limit` and `before` behave as in
    /// [`history`](Self::history) — `Some(0)` returns an empty `Vec`,
    /// `before` is an exclusive `DateTime<Utc>` cursor. An empty store
    /// returns an empty `Vec`, not an error.
    ///
    /// **This enumerates every thread the backend holds, with no per-caller
    /// filtering** — see the module-level "`ThreadId` is not an
    /// authorization boundary" section before exposing this over a network.
    async fn list_threads(
        &self,
        limit: Option<u32>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<ThreadSummary>, WaypointError>;

    /// Delete all waypoints for `thread`. Returns the number of waypoints
    /// deleted.
    ///
    /// An unknown `thread` returns `Ok(0)`, not an error — deleting a thread
    /// that never existed is a no-op, not a failure.
    async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing trait bounds (mirrors citadel_port.rs's
    // MockCitadel fixture).
    struct MockWaypointStore;

    #[async_trait]
    impl WaypointPort for MockWaypointStore {
        async fn save(&self, _wp: &Waypoint) -> Result<(), WaypointError> {
            Ok(())
        }

        async fn latest(&self, _thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
            Ok(None)
        }

        async fn get(
            &self,
            _thread: &ThreadId,
            _id: &WaypointId,
        ) -> Result<Option<Waypoint>, WaypointError> {
            Ok(None)
        }

        async fn history(
            &self,
            _thread: &ThreadId,
            _limit: Option<u32>,
            _before: Option<WaypointId>,
        ) -> Result<Vec<WaypointSummary>, WaypointError> {
            Ok(vec![])
        }

        async fn list_threads(
            &self,
            _limit: Option<u32>,
            _before: Option<DateTime<Utc>>,
        ) -> Result<Vec<ThreadSummary>, WaypointError> {
            Ok(vec![])
        }

        async fn delete_thread(&self, _thread: &ThreadId) -> Result<u64, WaypointError> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn mock_store_implements_trait() {
        let store = MockWaypointStore;
        let thread = ThreadId::new("t1").unwrap();
        assert!(store.latest(&thread).await.unwrap().is_none());
        assert!(store.list_threads(None, None).await.unwrap().is_empty());
        assert_eq!(store.delete_thread(&thread).await.unwrap(), 0);
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Box<dyn WaypointPort>> = None;
    }

    #[test]
    fn waypoint_summary_round_trips_through_serde_json() {
        let summary = WaypointSummary {
            waypoint_id: WaypointId::generate(),
            parent_waypoint_id: None,
            superstep: 3,
            status: WaypointStatus::Running,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        let restored: WaypointSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary, restored);
    }

    #[test]
    fn thread_summary_round_trips_through_serde_json() {
        let summary = ThreadSummary {
            thread_id: ThreadId::new("t1").unwrap(),
            latest_status: WaypointStatus::Completed,
            last_updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        let restored: ThreadSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary, restored);
    }
}
