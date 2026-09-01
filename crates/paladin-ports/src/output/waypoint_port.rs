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

use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
/// Implementations must be `Send + Sync` (Waypoints may be saved and loaded
/// concurrently across nodes and runs) and must treat a missing waypoint or
/// thread as `Ok(None)` / an empty list, never as an error on its own.
#[async_trait]
pub trait WaypointPort: Send + Sync {
    /// Persist a Waypoint. Implementations should treat this as an append
    /// (Waypoints are immutable once written; a thread's history is the
    /// ordered sequence of every `save`d waypoint).
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError>;

    /// Load the most recently saved Waypoint for `thread`.
    ///
    /// Returns `Ok(None)` if the thread has no waypoints yet — not an error.
    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError>;

    /// Load a specific Waypoint by id within `thread`.
    ///
    /// Returns `Ok(None)` if no such waypoint exists — not an error.
    async fn get(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<Option<Waypoint>, WaypointError>;

    /// List a thread's waypoint history, newest-first, paginated.
    ///
    /// `limit` bounds the number of summaries returned; `before` (if given)
    /// returns only waypoints strictly older than that id.
    async fn history(
        &self,
        thread: &ThreadId,
        limit: Option<u32>,
        before: Option<WaypointId>,
    ) -> Result<Vec<WaypointSummary>, WaypointError>;

    /// List known threads, newest-first by last update, paginated.
    async fn list_threads(
        &self,
        limit: Option<u32>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<ThreadSummary>, WaypointError>;

    /// Delete all waypoints for `thread`. Returns the number of waypoints
    /// deleted.
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
}
