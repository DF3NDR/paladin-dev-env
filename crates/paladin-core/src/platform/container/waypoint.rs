//! Waypoint & Thread Addressing
//!
//! This module defines the identity and checkpoint types the superstep engine
//! persists after every superstep: [`ThreadId`] addresses a run, [`WaypointId`]
//! addresses one checkpoint within it, and [`Waypoint`] is the checkpoint
//! itself — a full `Battlefield` snapshot plus enough bookkeeping to resume
//! the run with zero re-execution of completed work (ENG-FR-11, ENG-FR-12).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::platform::container::battalion::campaign::EdgeCondition;
use crate::platform::container::battlefield::{BATTLEFIELD_SCHEMA_VERSION, Battlefield};

/// Maximum length, in bytes, of a [`ThreadId`].
///
/// This is a **byte** limit (`str::len()`, the UTF-8 encoded length), not a
/// Unicode scalar value count. A multi-byte string is measured by its UTF-8
/// byte length, so e.g. 129 two-byte characters (258 bytes) is rejected even
/// though it is only 129 Unicode scalar values. See
/// `thread_id_multibyte_string_measured_in_bytes_at_boundary` for the
/// boundary proof.
pub const THREAD_ID_MAX_LEN: usize = 256;

/// Caller-supplied identity of a run. Validated non-empty, at most
/// [`THREAD_ID_MAX_LEN`] **bytes** (UTF-8 encoded length, not Unicode scalar
/// count — see that constant's rustdoc), and free of whitespace, so it is
/// safe to use as a storage key (file name component, SQL parameter, HTTP
/// path segment) without further sanitization.
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

    /// Generate a fresh, time-ordered `WaypointId`, wrapping [`Uuid::now_v7`].
    ///
    /// Alias of [`WaypointId::new`], named for call sites (e.g.
    /// [`Waypoint::new_root`], [`Waypoint::new_child`]) that read more clearly
    /// as "generate a new id" than "construct a default-like value".
    pub fn generate() -> Self {
        Self::new()
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

/// The canonical serde form of an edge condition (BUG-04 / ENG-FR-12a) --
/// the SAME bytes [`crate::platform::container::battalion::campaign`]'s
/// `WarGraph::fingerprint` (Phase 22.1 D-15/D-16, `paladin-battalion`) hashes
/// for an edge's condition, so an edge that fingerprints as the same edge is
/// also snapshot-keyed as the same edge by [`FrontierEdgeState`]. Falls back
/// to the empty string on a serialization failure, mirroring
/// `evaluate_edge_condition`'s existing `unwrap_or_default()` convention in
/// `engine::superstep` -- never panics.
pub fn canonical_edge_condition(condition: &Option<EdgeCondition>) -> String {
    serde_json::to_string(condition).unwrap_or_default()
}

/// One incoming edge's resolved state as of a [`Waypoint`] (BUG-04 /
/// ENG-FR-12a), keyed by identity -- `from`, `to`, and the edge's
/// [`canonical_edge_condition`] -- never by edge index, so inserting or
/// removing an edge elsewhere in a graph cannot shift a stored resolution
/// onto a different edge. Absence from a [`FrontierSnapshot`]'s `edges`
/// means `Pending`: only a RESOLVED (`Fired`/`NotFiring`) edge is ever
/// recorded here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontierEdgeState {
    /// The edge's source node.
    pub from: NodeId,
    /// The edge's target node.
    pub to: NodeId,
    /// The edge's condition, in its [`canonical_edge_condition`] form.
    pub condition: String,
    /// Whether this edge fired (`true`) or was proven not-firing (`false`).
    pub fired: bool,
    /// The superstep at which this edge resolved.
    pub resolved_at: u64,
}

/// A snapshot of the superstep engine's `Frontier` (`engine::superstep`,
/// `paladin-battalion`) as of one [`Waypoint`] (BUG-04 / ENG-FR-12a): every
/// RESOLVED incoming edge across the whole run, plus the superstep each
/// declared node last executed at. Restored into a fresh `Frontier` on
/// resume so per-edge resolutions recorded before an interruption are not
/// lost -- without this, a pre-crash fired edge into a join node that was
/// not yet ready is never seen again, and a resumed run can report
/// `Completed` without a node the uninterrupted run executes.
///
/// `edges` is emitted sorted by `(from, to, condition)` and `last_executed`
/// is a `BTreeMap` (never a `HashMap`/`HashSet`), so two byte-identical runs
/// produce byte-identical `Waypoint` payloads (ENG-FR-04/08, RESEARCH.md
/// Pitfall 5) -- the sort itself is `Frontier::snapshot`'s responsibility,
/// the sole runtime producer of this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrontierSnapshot {
    /// Every resolved incoming edge across the run, sorted by
    /// `(from, to, condition)`. An edge absent here is `Pending`.
    pub edges: Vec<FrontierEdgeState>,
    /// The superstep each declared node last executed at, if ever.
    pub last_executed: BTreeMap<NodeId, u64>,
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
    /// Per-node visit counts accumulated so far this run, keyed by `NodeId`.
    ///
    /// A `BTreeMap` (never a `HashMap`) so this field serializes
    /// byte-identically regardless of insertion order (RESEARCH.md Pitfall
    /// 5). Carried on every waypoint so a later `resume` (ENG-FR-12) can
    /// restore each node's visit count without re-deriving it from
    /// `completed` records across the thread's whole history.
    #[serde(default)]
    pub visit_counts: BTreeMap<NodeId, u32>,
    /// A snapshot of the superstep engine's `Frontier` as of this waypoint
    /// (BUG-04 / ENG-FR-12a): every resolved incoming edge across the run,
    /// keyed by edge identity, plus each node's last-executed superstep.
    ///
    /// `#[serde(default)]`, matching `visit_counts`' precedent: a `Waypoint`
    /// payload written before this field existed still loads, with an empty
    /// snapshot -- a resume over such a payload behaves exactly as it did
    /// before this field existed (every edge starts `Pending`), rather than
    /// failing to deserialize.
    #[serde(default)]
    pub frontier: FrontierSnapshot,
}

impl Waypoint {
    /// Stamp the current [`BATTLEFIELD_SCHEMA_VERSION`] as a `Waypoint`'s
    /// `schema_version`. Convenience for constructors elsewhere in the
    /// engine, kept here so the constant has one canonical consumer path.
    pub fn current_schema_version() -> String {
        BATTLEFIELD_SCHEMA_VERSION.to_string()
    }

    /// Construct the first `Waypoint` of a thread.
    ///
    /// `parent_waypoint_id` is always `None`: this is the lineage root. Use
    /// [`Waypoint::new_child`] for every subsequent `Waypoint` in the thread,
    /// so lineage cannot be constructed incorrectly by hand.
    #[allow(clippy::too_many_arguments)]
    pub fn new_root(
        thread_id: ThreadId,
        superstep: u64,
        graph_fingerprint: GraphFingerprint,
        battlefield: Battlefield,
        vanguard: Vec<NodeId>,
        completed: Vec<NodeExecutionRecord>,
        status: WaypointStatus,
        visit_counts: BTreeMap<NodeId, u32>,
        frontier: FrontierSnapshot,
    ) -> Self {
        Self {
            thread_id,
            waypoint_id: WaypointId::generate(),
            parent_waypoint_id: None,
            superstep,
            graph_fingerprint,
            battlefield,
            vanguard,
            completed,
            status,
            created_at: Utc::now(),
            schema_version: Self::current_schema_version(),
            visit_counts,
            frontier,
        }
    }

    /// Construct a `Waypoint` chained from `parent`.
    ///
    /// `thread_id` is copied from `parent` (a child always belongs to its
    /// parent's thread) and `parent_waypoint_id` is set to
    /// `Some(parent.waypoint_id)`, so lineage is exactly the previous
    /// `Waypoint` of that thread by construction.
    #[allow(clippy::too_many_arguments)]
    pub fn new_child(
        parent: &Waypoint,
        superstep: u64,
        graph_fingerprint: GraphFingerprint,
        battlefield: Battlefield,
        vanguard: Vec<NodeId>,
        completed: Vec<NodeExecutionRecord>,
        status: WaypointStatus,
        visit_counts: BTreeMap<NodeId, u32>,
        frontier: FrontierSnapshot,
    ) -> Self {
        Self {
            thread_id: parent.thread_id.clone(),
            waypoint_id: WaypointId::generate(),
            parent_waypoint_id: Some(parent.waypoint_id),
            superstep,
            graph_fingerprint,
            battlefield,
            vanguard,
            completed,
            status,
            created_at: Utc::now(),
            schema_version: Self::current_schema_version(),
            visit_counts,
            frontier,
        }
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
    fn thread_id_rejects_tab() {
        assert_eq!(
            ThreadId::new("has\ttab"),
            Err(ThreadIdError::ContainsWhitespace)
        );
    }

    #[test]
    fn thread_id_rejects_newline() {
        assert_eq!(
            ThreadId::new("has\nnewline"),
            Err(ThreadIdError::ContainsWhitespace)
        );
    }

    #[test]
    fn thread_id_rejects_non_breaking_space() {
        // U+00A0 NO-BREAK SPACE carries Unicode White_Space=Yes, so
        // char::is_whitespace() rejects it the same as an ordinary space.
        assert_eq!(
            ThreadId::new("has\u{00A0}nbsp"),
            Err(ThreadIdError::ContainsWhitespace)
        );
    }

    #[test]
    fn thread_id_accepts_exactly_max_len() {
        let at_max = "a".repeat(THREAD_ID_MAX_LEN);
        assert!(ThreadId::new(at_max).is_ok());
    }

    #[test]
    fn thread_id_multibyte_string_measured_in_bytes_at_boundary() {
        // "é" (U+00E9) encodes as 2 UTF-8 bytes. 128 of them is exactly
        // THREAD_ID_MAX_LEN (256) bytes -- accepted -- while 128 Unicode
        // scalar values alone would be well under any reasonable char-count
        // limit, proving the boundary is enforced in bytes, not chars.
        let at_boundary: String = "é".repeat(THREAD_ID_MAX_LEN / 2);
        assert_eq!(at_boundary.len(), THREAD_ID_MAX_LEN);
        assert!(ThreadId::new(at_boundary).is_ok());

        // One "é" over the boundary is 258 bytes (129 scalar values), and
        // must be rejected with the byte length in the error, not 129.
        let over_boundary: String = "é".repeat((THREAD_ID_MAX_LEN / 2) + 1);
        assert_eq!(over_boundary.len(), THREAD_ID_MAX_LEN + 2);
        assert_eq!(
            ThreadId::new(over_boundary.clone()),
            Err(ThreadIdError::TooLong {
                len: over_boundary.len()
            })
        );
    }

    #[test]
    fn waypoint_id_is_time_ordered() {
        let a = WaypointId::new();
        let b = WaypointId::new();
        assert!(a <= b);
    }

    #[test]
    fn waypoint_id_generate_never_collides_and_sorts_in_creation_order() {
        let a = WaypointId::generate();
        let b = WaypointId::generate();
        assert_ne!(a, b);
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
            visit_counts: BTreeMap::new(),
            frontier: FrontierSnapshot::default(),
        };

        let json = serde_json::to_string(&waypoint).unwrap();
        let restored: Waypoint = serde_json::from_str(&json).unwrap();
        assert_eq!(waypoint, restored);
        assert_eq!(restored.schema_version, BATTLEFIELD_SCHEMA_VERSION);
    }

    #[test]
    fn new_root_has_no_parent() {
        let schema = BattlefieldSchema::new(vec![]);
        let root = Waypoint::new_root(
            ThreadId::new("thread-1").unwrap(),
            0,
            GraphFingerprint::from_canonical_bytes(b"fixture"),
            Battlefield::new(schema),
            vec![],
            vec![],
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
        );
        assert_eq!(root.parent_waypoint_id, None);
        assert_eq!(root.superstep, 0);
    }

    #[test]
    fn new_child_points_at_parent_and_inherits_thread() {
        let schema = BattlefieldSchema::new(vec![]);
        let root = Waypoint::new_root(
            ThreadId::new("thread-1").unwrap(),
            0,
            GraphFingerprint::from_canonical_bytes(b"fixture"),
            Battlefield::new(schema.clone()),
            vec![],
            vec![],
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
        );
        let child = Waypoint::new_child(
            &root,
            1,
            GraphFingerprint::from_canonical_bytes(b"fixture"),
            Battlefield::new(schema),
            vec![],
            vec![],
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
        );
        assert_eq!(child.parent_waypoint_id, Some(root.waypoint_id));
        assert_eq!(child.thread_id, root.thread_id);
        assert_ne!(child.waypoint_id, root.waypoint_id);
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

    // --- BUG-04 / ENG-FR-12a: FrontierSnapshot ------------------------------

    #[test]
    fn waypoint_payload_without_frontier_field_deserializes_with_an_empty_snapshot() {
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
            superstep: 3,
            graph_fingerprint: GraphFingerprint::from_canonical_bytes(b"fixture"),
            battlefield: Battlefield::new(schema),
            vanguard: vec![],
            completed: vec![],
            status: WaypointStatus::Completed,
            created_at: Utc::now(),
            schema_version: Waypoint::current_schema_version(),
            visit_counts: BTreeMap::new(),
            frontier: FrontierSnapshot {
                edges: vec![FrontierEdgeState {
                    from: NodeId::new("a"),
                    to: NodeId::new("b"),
                    condition: canonical_edge_condition(&None),
                    fired: true,
                    resolved_at: 1,
                }],
                last_executed: BTreeMap::from([(NodeId::new("a"), 1)]),
            },
        };

        // Simulate a pre-BUG-04 payload: serialize, then strip the
        // `frontier` key entirely before deserializing back, rather than
        // merely round-tripping the value already present.
        let mut value = serde_json::to_value(&waypoint).unwrap();
        value
            .as_object_mut()
            .expect("Waypoint serializes to a JSON object")
            .remove("frontier");
        assert!(
            !value.to_string().contains("frontier"),
            "the frontier key must be genuinely absent from the fixture payload"
        );

        let restored: Waypoint = serde_json::from_value(value).unwrap();
        assert_eq!(restored.frontier, FrontierSnapshot::default());
        // Every other field is untouched by the missing key.
        assert_eq!(restored.thread_id, waypoint.thread_id);
        assert_eq!(restored.superstep, waypoint.superstep);
    }

    #[test]
    fn frontier_snapshot_edges_serialize_in_sorted_identity_order() {
        // `Frontier::snapshot` (`engine::superstep`, `paladin-battalion`) is
        // the sole runtime producer of a `FrontierSnapshot` and owns sorting
        // `edges` by `(from, to, condition)` before construction
        // (ENG-FR-04/08). This test proves the TYPE preserves that order
        // faithfully through a serde round trip -- `edges` is a plain `Vec`,
        // never re-sorted or re-ordered by serialization -- and that
        // `last_executed`, a `BTreeMap`, serializes with its keys in sorted
        // order regardless of insertion order, so two byte-identical runs
        // produce byte-identical Waypoint payloads.
        let already_sorted_edges = vec![
            FrontierEdgeState {
                from: NodeId::new("a"),
                to: NodeId::new("b"),
                condition: canonical_edge_condition(&None),
                fired: true,
                resolved_at: 1,
            },
            FrontierEdgeState {
                from: NodeId::new("a"),
                to: NodeId::new("c"),
                condition: canonical_edge_condition(&None),
                fired: false,
                resolved_at: 2,
            },
            FrontierEdgeState {
                from: NodeId::new("b"),
                to: NodeId::new("c"),
                condition: canonical_edge_condition(&None),
                fired: true,
                resolved_at: 3,
            },
        ];

        let mut last_executed = BTreeMap::new();
        // Insert out of alphabetical order -- a `BTreeMap` always iterates
        // (and therefore serializes) in key order regardless of insertion
        // order.
        last_executed.insert(NodeId::new("zulu"), 5u64);
        last_executed.insert(NodeId::new("alpha"), 1u64);
        last_executed.insert(NodeId::new("mike"), 3u64);

        let snapshot = FrontierSnapshot {
            edges: already_sorted_edges.clone(),
            last_executed,
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: FrontierSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.edges, already_sorted_edges);

        // Confirm the ORDER as written in the JSON text itself (not merely
        // after round-tripping back into a BTreeMap, which would sort on
        // load regardless of what was written) -- isolate the
        // `last_executed` section so an edge's own "a"/"b"/"c" node names
        // cannot produce a false match.
        let last_executed_section = json
            .split("\"last_executed\":")
            .nth(1)
            .expect("last_executed key must be present in the serialized JSON");
        let alpha_pos = last_executed_section.find("alpha").unwrap();
        let mike_pos = last_executed_section.find("mike").unwrap();
        let zulu_pos = last_executed_section.find("zulu").unwrap();
        assert!(
            alpha_pos < mike_pos && mike_pos < zulu_pos,
            "last_executed keys must serialize in sorted order: {json}"
        );
    }
}
