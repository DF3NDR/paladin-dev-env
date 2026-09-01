//! Waypoint & Thread Addressing
//!
//! This module defines the identity and checkpoint types the superstep engine
//! persists after every superstep: [`ThreadId`] addresses a run, [`WaypointId`]
//! addresses one checkpoint within it, and [`Waypoint`] is the checkpoint
//! itself — a full `Battlefield` snapshot plus enough bookkeeping to resume
//! the run with zero re-execution of completed work (ENG-FR-11, ENG-FR-12).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::platform::container::battlefield::{BATTLEFIELD_SCHEMA_VERSION, Battlefield};

/// Maximum length, in bytes, of a [`ThreadId`].
pub const THREAD_ID_MAX_LEN: usize = 256;

/// Caller-supplied identity of a run. Validated non-empty, at most
/// [`THREAD_ID_MAX_LEN`] characters, and free of whitespace, so it is safe to
/// use as a storage key (file name component, SQL parameter, HTTP path
/// segment) without further sanitization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(String);

/// Error returned by [`ThreadId::new`] when the supplied string is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ThreadIdError {
    /// The supplied thread id was empty.
    #[error("thread id must not be empty")]
    Empty,
    /// The supplied thread id exceeded `THREAD_ID_MAX_LEN` characters.
    #[error("thread id must be at most {THREAD_ID_MAX_LEN} characters, got {len}")]
    TooLong {
        /// The length of the rejected thread id.
        len: usize,
    },
    /// The supplied thread id contained whitespace.
    #[error("thread id must not contain whitespace")]
    ContainsWhitespace,
}

impl ThreadId {
    /// Construct a `ThreadId`, validating non-empty, length, and whitespace.
    pub fn new(id: impl Into<String>) -> Result<Self, ThreadIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ThreadIdError::Empty);
        }
        if id.len() > THREAD_ID_MAX_LEN {
            return Err(ThreadIdError::TooLong { len: id.len() });
        }
        if id.chars().any(char::is_whitespace) {
            return Err(ThreadIdError::ContainsWhitespace);
        }
        Ok(Self(id))
    }

    /// Borrow the thread id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Engine-generated identity of one `Waypoint`, a UUIDv7 (time-ordered) value
/// so waypoints within a thread sort chronologically by id alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WaypointId(Uuid);

impl WaypointId {
    /// Generate a fresh, time-ordered `WaypointId`.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Borrow the underlying `Uuid`.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WaypointId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WaypointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A node identity within a `WarGraph`: a human-readable name (e.g.
/// `"researcher"`), unique within its graph. Replaces raw `Uuid` node
/// identity in new APIs; the existing Campaign pattern keeps `Uuid` and the
/// engine maps between the two where they meet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
    /// Construct a `NodeId` from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the node id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A stable content fingerprint of a `WarGraph`: a hash over node ids, edge
/// specs and schema field names — deliberately NOT over prompts or models,
/// which may be hot-swapped without changing run semantics (ENG-FR-14).
///
/// Encoded as `v1:{blake3_hex}` (Phase 22 Task 1 decision, option-b): the
/// `v1:` tag lets a future algorithm change emit `v2:` and be recognised
/// rather than silently failing every stored thread's `resume` with
/// `GraphMismatch`. See `.planning/phases/22-battlefield-state-superstep-engine/22-01-SUMMARY.md`
/// for the decision record; the `v1:` tag is documented as part of the
/// Waypoint payload format in MIGRATION.md §9.4 when that file lands (Plan
/// 22-04).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphFingerprint(String);

/// Fingerprint algorithm/encoding version tag (Task 1 decision, option-b).
pub const GRAPH_FINGERPRINT_VERSION: &str = "v1";

impl GraphFingerprint {
    /// Compute a `GraphFingerprint` over a caller-supplied canonical byte
    /// stream (deterministically sorted node ids, edge specs, and schema
    /// field names — never a raw `HashMap` iteration order).
    pub fn from_canonical_bytes(bytes: &[u8]) -> Self {
        let hash = blake3::hash(bytes);
        Self(format!("{GRAPH_FINGERPRINT_VERSION}:{}", hash.to_hex()))
    }

    /// Borrow the encoded fingerprint string (`"v1:{hex}"`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GraphFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The outcome of one node's execution within a superstep, recorded on the
/// `Waypoint` that superstep produces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeOutcomeKind {
    /// The node ran and returned a `StateDelta` successfully.
    Succeeded,
    /// The node ran and returned an error.
    Failed,
    /// The node was not executed this superstep (e.g. a `defer`red join
    /// whose dependencies were not all satisfied yet).
    Skipped {
        /// Why the node was skipped.
        reason: String,
    },
}

/// A record of one node's execution within the superstep that produced a
/// given `Waypoint`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeExecutionRecord {
    /// The node that ran.
    pub node_id: NodeId,
    /// The Paladin identity, if this node wraps a `Paladin`.
    pub paladin_id: Option<Uuid>,
    /// When the node started executing.
    pub started_at: DateTime<Utc>,
    /// How long the node ran, in milliseconds.
    pub duration_ms: u64,
    /// Tokens consumed by this node's execution, if applicable.
    pub token_count: u64,
    /// The node's outcome.
    pub outcome: NodeOutcomeKind,
    /// Attempt number for this node this run. Populated meaningfully once
    /// per-node retry lands (Doc 04); `1` until then.
    pub attempt: u32,
}

/// Stub type for a paused run's outstanding input request.
///
/// Fully defined by Doc 03 (parley/resume-with-payload); this phase only
/// lands the stub so `WaypointStatus::AwaitingInput` has somewhere to point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ParleyRequest {
    /// Free-form prompt describing what input is being awaited.
    pub prompt: String,
}

/// The status of a run as of a given `Waypoint`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WaypointStatus {
    /// More supersteps pending (the Vanguard is non-empty).
    Running,
    /// The run finished normally.
    Completed,
    /// The run failed.
    Failed {
        /// A human-readable description of the failure.
        error: String,
        /// The node whose execution caused the failure.
        failed_node: NodeId,
    },
    /// The run is paused awaiting external input (Doc 03).
    AwaitingInput {
        /// The outstanding input request.
        parley: ParleyRequest,
    },
    /// The run was gracefully halted (Doc 03 cancellation).
    Halted,
}

/// A checkpoint written after one superstep of a `WarGraph` run.
///
/// Embeds a full `Battlefield` snapshot (delta-encoding is a backend storage
/// optimization, never part of this contract) so any `Waypoint` can be read
/// back and resumed from in isolation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waypoint {
    /// The thread (run) this waypoint belongs to.
    pub thread_id: ThreadId,
    /// This waypoint's own identity.
    pub waypoint_id: WaypointId,
    /// The waypoint this one was checkpointed from. `None` only for the
    /// first waypoint of a thread (or a future fork root, Doc 03).
    pub parent_waypoint_id: Option<WaypointId>,
    /// The superstep index that produced this waypoint.
    pub superstep: u64,
    /// The stable content fingerprint of the `WarGraph` this waypoint was
    /// produced under (ENG-FR-14); compared on `resume`.
    pub graph_fingerprint: GraphFingerprint,
    /// A full snapshot of the shared state as of this waypoint.
    pub battlefield: Battlefield,
    /// Nodes ready for the next superstep.
    pub vanguard: Vec<NodeId>,
    /// What ran in the superstep that produced this waypoint.
    pub completed: Vec<NodeExecutionRecord>,
    /// This waypoint's status.
    pub status: WaypointStatus,
    /// When this waypoint was created.
    pub created_at: DateTime<Utc>,
    /// Schema version this waypoint was persisted under (X-04).
    pub schema_version: String,
}

impl Waypoint {
    /// Stamp the current [`BATTLEFIELD_SCHEMA_VERSION`] as a `Waypoint`'s
    /// `schema_version`. Convenience for constructors elsewhere in the
    /// engine, kept here so the constant has one canonical consumer path.
    pub fn current_schema_version() -> String {
        BATTLEFIELD_SCHEMA_VERSION.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };

    #[test]
    fn thread_id_rejects_empty() {
        assert_eq!(ThreadId::new(""), Err(ThreadIdError::Empty));
    }

    #[test]
    fn thread_id_rejects_whitespace() {
        assert_eq!(
            ThreadId::new("has space"),
            Err(ThreadIdError::ContainsWhitespace)
        );
    }

    #[test]
    fn thread_id_rejects_too_long() {
        let long = "a".repeat(THREAD_ID_MAX_LEN + 1);
        assert_eq!(
            ThreadId::new(long.clone()),
            Err(ThreadIdError::TooLong { len: long.len() })
        );
    }

    #[test]
    fn thread_id_accepts_valid_id() {
        assert!(ThreadId::new("thread-abc123").is_ok());
    }

    #[test]
    fn waypoint_id_is_time_ordered() {
        let a = WaypointId::new();
        let b = WaypointId::new();
        assert!(a <= b);
    }

    #[test]
    fn graph_fingerprint_is_deterministic_and_versioned() {
        let a = GraphFingerprint::from_canonical_bytes(b"node:a|edge:none|schema:result");
        let b = GraphFingerprint::from_canonical_bytes(b"node:a|edge:none|schema:result");
        assert_eq!(a, b);
        assert!(a.as_str().starts_with("v1:"));
    }

    #[test]
    fn graph_fingerprint_differs_on_different_input() {
        let a = GraphFingerprint::from_canonical_bytes(b"graph-a");
        let b = GraphFingerprint::from_canonical_bytes(b"graph-b");
        assert_ne!(a, b);
    }

    #[test]
    fn waypoint_round_trips_through_serde_json() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("result").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let waypoint = Waypoint {
            thread_id: ThreadId::new("thread-1").unwrap(),
            waypoint_id: WaypointId::new(),
            parent_waypoint_id: None,
            superstep: 1,
            graph_fingerprint: GraphFingerprint::from_canonical_bytes(b"fixture"),
            battlefield: Battlefield::new(schema),
            vanguard: vec![],
            completed: vec![],
            status: WaypointStatus::Completed,
            created_at: Utc::now(),
            schema_version: Waypoint::current_schema_version(),
        };

        let json = serde_json::to_string(&waypoint).unwrap();
        let restored: Waypoint = serde_json::from_str(&json).unwrap();
        assert_eq!(waypoint, restored);
        assert_eq!(restored.schema_version, BATTLEFIELD_SCHEMA_VERSION);
    }

    #[test]
    fn parley_request_round_trips() {
        let parley = ParleyRequest {
            prompt: "please confirm".to_string(),
        };
        let json = serde_json::to_string(&parley).unwrap();
        let restored: ParleyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parley, restored);
    }
}
