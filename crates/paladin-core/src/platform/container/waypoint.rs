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
use crate::platform::container::battlefield::{
    BATTLEFIELD_SCHEMA_VERSION, Battlefield, StateDelta,
};
use crate::platform::container::directive::MusterTask;
pub use crate::platform::container::parley::{
    OnExpire, ParleyId, ParleyKind, ParleyRequest, ParleyResponse,
};

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

    /// Derive a child `ThreadId` from a parent thread and a Battalion node's
    /// id (CF-FR-15, D-20): the durable identity under which a
    /// `NodeSpec::Battalion` node's embedded child run addresses its own
    /// Waypoints through the SAME `WaypointPort` the parent uses.
    ///
    /// # Injectivity
    ///
    /// Encodes as a FIXED-WIDTH (16 lowercase-hex-digit, i.e. 64-bit),
    /// length-prefixed segment for `parent`, immediately followed by
    /// `parent`'s own bytes, then the same shape for `node`:
    /// `format!("{:016x}{parent}{:016x}{node}", parent.len(), node.len())`.
    /// Mirrors `engine::graph::push_field`'s length-prefixed byte encoding
    /// (`paladin-battalion`, introduced by Phase 22.1 CR-01) adapted to
    /// TEXT rather than an opaque hash input: a raw binary length prefix
    /// (`push_field`'s own `u64::to_le_bytes()`) can contain a byte in the
    /// ASCII whitespace range or a byte that is not valid UTF-8 on its own,
    /// either of which would corrupt or outright fail construction of the
    /// `String` a `ThreadId` wraps -- a fixed-width HEX-encoded length
    /// avoids both hazards while keeping the same injectivity property.
    ///
    /// Because the prefix width is FIXED (never variable-width or
    /// delimiter-terminated), the byte offset at which `parent`'s own bytes
    /// start and end -- and where `node`'s begin -- is always fully
    /// determined by the two length values alone. No byte sequence
    /// occurring INSIDE `parent` or `node` (including one that happens to
    /// look like a length prefix, or a `/`, `:`, or any other delimiter
    /// character) can ever be reinterpreted as a different split between
    /// the two segments. This is deliberately NOT a bare delimiter join
    /// (`format!("{parent}/{node}")`-style): [`NodeId::new`] validates
    /// nothing beyond non-emptiness, so a bare join is collidable by
    /// construction: parent `"t"` paired with node `"a/b"`, and parent
    /// `"t/a"` paired with node `"b"`, would join to the identical string.
    /// This is the exact defect class Phase 22.1's CR-01 found and fixed
    /// once already, in `WarGraph::fingerprint()`'s canonical byte encoding
    /// (`paladin-battalion/src/engine/graph.rs`); this method deliberately
    /// reuses that fix's length-prefixed approach rather than reintroducing
    /// the same hazard in a second place.
    ///
    /// # Errors
    ///
    /// Returns the same [`ThreadIdError`] [`ThreadId::new`] would: `Empty`
    /// is unreachable here (the encoded result always contains at least the
    /// two 16-character length prefixes, so it is never empty);
    /// `ContainsWhitespace` is reachable if `node`'s own bytes contain
    /// whitespace ([`NodeId::new`] does not reject it, unlike
    /// `ThreadId::new`); `TooLong` is reachable for a sufficiently long
    /// `parent`/`node` pair, including one produced by nesting several
    /// derivations deep. The derivation FAILS TYPED in every such case
    /// rather than silently truncating the encoded result -- a truncated
    /// encoding would reopen exactly the collision hazard this method
    /// exists to close.
    ///
    /// ```
    /// # use paladin_core::platform::container::waypoint::{ThreadId, NodeId};
    /// let parent = ThreadId::new("run-1").unwrap();
    /// let node = NodeId::new("subgraph");
    /// let child = ThreadId::child(&parent, &node).unwrap();
    /// assert_ne!(child.as_str(), parent.as_str());
    ///
    /// // Derivation composes: a grandchild derives from the child exactly
    /// // as the child derived from the root.
    /// let grandchild_node = NodeId::new("nested-subgraph");
    /// let grandchild = ThreadId::child(&child, &grandchild_node).unwrap();
    /// assert_ne!(grandchild.as_str(), child.as_str());
    /// ```
    pub fn child(parent: &ThreadId, node: &NodeId) -> Result<Self, ThreadIdError> {
        let encoded = format!(
            "{:016x}{}{:016x}{}",
            parent.as_str().len(),
            parent.as_str(),
            node.as_str().len(),
            node.as_str(),
        );
        Self::new(encoded)
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
/// Encoded as `{GRAPH_FINGERPRINT_VERSION}:{blake3_hex}` (Phase 22 Task 1
/// decision, option-b): the version tag lets a future algorithm change emit
/// a new tag and be recognised rather than silently failing every stored
/// thread's `resume` with `GraphMismatch`. See
/// `.planning/phases/22-battlefield-state-superstep-engine/22-01-SUMMARY.md`
/// for the decision record; the tag is documented as part of the Waypoint
/// payload format in MIGRATION.md §9.4 when that file lands (Plan 22-04).
///
/// Bumped to `v2` (Phase 22.1 CR-01, D-17): `v1`'s canonical byte encoding
/// (`WarGraph::fingerprint`, `paladin-battalion`) was delimiter-separated
/// and unescaped, so two structurally different graphs could be crafted to
/// hash identically whenever a `NodeId`/`FieldName` contained one of the
/// separator bytes. `v2` uses a length-prefixed encoding with no delimiter
/// collision; every `v1`-tagged fingerprint is now recognised as stale
/// rather than silently reinterpreted.
///
/// Bumped to `v3` (Phase 23, D-18): three new scheduling/merge-relevant
/// sections were added to `WarGraph::fingerprint`'s hashed bytes -- the
/// worker-template set (CF-03), each `NodeSpec::Battalion` node's child
/// fingerprint plus `StateMap` plus `restart_on_resume` (CF-04), and each
/// `NodeSpec::Paladin` node's `DirectiveParser` kind plus `on_parse_error`
/// (CF-02) -- each written through the same length-prefixed `push_field`
/// helper `v2` established, never a delimiter join. Every `v2`-tagged
/// fingerprint is now recognised as stale on `resume` rather than silently
/// reinterpreted under the new layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphFingerprint(String);

/// Fingerprint algorithm/encoding version tag (Task 1 decision, option-b;
/// bumped to `v2` by Phase 22.1 CR-01 / D-17's collision-free re-encoding;
/// bumped to `v3` by Phase 23 D-18's three new hashed sections).
pub const GRAPH_FINGERPRINT_VERSION: &str = "v3";

impl GraphFingerprint {
    /// Compute a `GraphFingerprint` over a caller-supplied canonical byte
    /// stream (deterministically sorted node ids, edge specs, and schema
    /// field names — never a raw `HashMap` iteration order).
    pub fn from_canonical_bytes(bytes: &[u8]) -> Self {
        let hash = blake3::hash(bytes);
        Self(format!("{GRAPH_FINGERPRINT_VERSION}:{}", hash.to_hex()))
    }

    /// Borrow the encoded fingerprint string (`"v3:{hex}"`).
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

/// A Muster's in-progress state as of one progress [`Waypoint`] (CF-FR-12,
/// D-14): the mustering node, the FULL validated (sorted by `task_key`)
/// task list -- the same shape `validate_muster_tasks`
/// (`engine::superstep`, `paladin-battalion`) accepted -- and every
/// completed task's UNMERGED [`StateDelta`], keyed by `task_key`. Carried
/// on a `Waypoint` whose `battlefield` is still the superstep-START
/// snapshot (unchanged): the deltas recorded here are merged into the real
/// Battlefield exactly once, after every task in `tasks` has resolved,
/// through the engine's existing end-of-superstep merge path in
/// `task_key` order -- never incrementally, which would break snapshot
/// isolation for the still-running siblings and make a resumed run
/// double-merge.
///
/// `completed` is a `BTreeMap` (never a `HashMap`), matching
/// `visit_counts`'/`frontier`'s precedent, so this field serializes
/// byte-identically regardless of insertion order (RESEARCH.md Pitfall 5).
///
/// Resume reads [`MusterProgress::unfinished_tasks`] to decide which tasks
/// still need to run -- it does NOT reconstruct that set from the
/// Battlefield, which by design has not changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusterProgress {
    /// The node whose `NextStep::Muster` produced this record.
    pub node: NodeId,
    /// The full, validated (sorted by `task_key`) muster task list, carried
    /// here so a resumed run does not need the original `Directive` to
    /// re-enter the muster.
    pub tasks: Vec<MusterTask>,
    /// Every completed task's UNMERGED `StateDelta`, keyed by `task_key`.
    pub completed: BTreeMap<String, StateDelta>,
}

impl MusterProgress {
    /// The tasks in `self.tasks` whose `task_key` is absent from
    /// `self.completed` -- the set a resumed run must still execute, in
    /// `self.tasks`' own (`task_key`-sorted) order.
    pub fn unfinished_tasks(&self) -> Vec<MusterTask> {
        self.tasks
            .iter()
            .filter(|task| !self.completed.contains_key(&task.task_key))
            .cloned()
            .collect()
    }
}

impl Default for MusterProgress {
    /// An empty, no-op `MusterProgress` -- never produced by the engine
    /// itself (a real record always carries a real mustering `node` and a
    /// non-empty `tasks` list), provided so callers needing a placeholder
    /// value (e.g. test fixtures) do not need to invent one.
    fn default() -> Self {
        Self {
            node: NodeId::new(String::new()),
            tasks: Vec::new(),
            completed: BTreeMap::new(),
        }
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
    /// The node ran and returned a `Directive` whose `next` was
    /// `NextStep::End` (CF-FR-08): this node's `StateDelta` merged
    /// normally, and its return also completed the run after this
    /// superstep. Distinguishes the run-ending node from an ordinary
    /// `Succeeded` node so which node ended the run is observable from a
    /// persisted `Waypoint`'s `completed` records without re-running the
    /// graph (D-09).
    Ended,
    /// The node ran and its `Directive.next` was `NextStep::Parley`
    /// (HITL-01, D-03): its own `StateDelta` merged normally (it emitted
    /// it), and its return also suspended the run after this superstep.
    /// Distinguishes the parleying node from an ordinary `Succeeded` node
    /// so which node(s) raised the pause is observable from a persisted
    /// `Waypoint`'s `completed` records without re-running the graph,
    /// mirroring `Ended`'s precedent.
    Parleyed,
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
    /// The run is paused awaiting external input (HITL-01, D-02): every
    /// `ParleyRequest` raised in the suspending superstep, plus every
    /// `ParleyResponse` accepted so far -- so "partially answered" is a
    /// property of this persisted `Waypoint`, not of process memory
    /// (HITL-FR-02's survive-termination rule applies to partial answers
    /// too). Never persisted with an empty `parleys` list.
    AwaitingInput {
        /// Every parley request raised in the suspending superstep, ordered
        /// by `node_id` (mirrors `completed`'s own `node_id` sort).
        parleys: Vec<ParleyRequest>,
        /// The accepted subset of responses so far -- empty on the initial
        /// suspension, growing as partial answers are accepted (a later
        /// plan; this phase's `resume_with` only accepts a response set
        /// that answers the happy path).
        responses: Vec<ParleyResponse>,
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
    /// A Muster's in-progress state as of this waypoint (CF-FR-12, D-14):
    /// `Some` only for a progress Waypoint written mid-muster (`status`
    /// `Running`, `superstep` equal to the muster's own dispatch
    /// superstep); `None` for every ordinary superstep-complete Waypoint,
    /// exactly as before this field existed.
    ///
    /// `#[serde(default)]`, matching `visit_counts`'/`frontier`'s
    /// precedent: a `Waypoint` payload written before this field existed
    /// still loads, with `None` -- a resume over such a payload behaves
    /// exactly as it did before this field existed, rather than failing to
    /// deserialize.
    #[serde(default)]
    pub muster_progress: Option<MusterProgress>,
    /// The namespace path of a `NodeSpec::Battalion` child run's Waypoints
    /// (CF-FR-15, D-20): `None` for a top-level (non-child) run's own
    /// Waypoints, `Some("parent_node_id/")` for a child run's, with nested
    /// paths concatenating one segment per nesting level (e.g.
    /// `"outer_node/inner_node/"` for a grandchild).
    ///
    /// A RECORD for observability and debugging ONLY -- NOT the isolation
    /// mechanism (RESEARCH.md Pitfall 6). Isolation between a parent's and
    /// a child's Waypoints comes entirely from [`ThreadId::child`]'s
    /// distinct derived `ThreadId`: `thread_id` above is already the
    /// child's own thread by the time a Waypoint carrying a `Some`
    /// `checkpoint_ns` is constructed, so `WaypointPort::latest` on that
    /// thread already returns only that child's own history. No lookup
    /// path in this codebase derives a child's Waypoints by filtering a
    /// parent thread's history by `checkpoint_ns` -- building one would
    /// contradict this field's documented role.
    ///
    /// `#[serde(default)]`, matching `visit_counts`'/`frontier`'s/
    /// `muster_progress`'s precedent: a `Waypoint` payload written before
    /// this field existed still loads, with `None` -- a resume over such a
    /// payload behaves exactly as it did before this field existed, rather
    /// than failing to deserialize.
    #[serde(default)]
    pub checkpoint_ns: Option<String>,
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
    ///
    /// `muster_progress` always starts `None` here: a fresh root is never
    /// itself a mid-muster progress checkpoint (`engine::superstep`'s
    /// `build_waypoint`, the sole production writer of a `Some`
    /// `muster_progress` value, constructs every `Waypoint` -- progress or
    /// not -- directly rather than through this constructor).
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
            muster_progress: None,
            checkpoint_ns: None,
        }
    }

    /// Construct a `Waypoint` chained from `parent`.
    ///
    /// `thread_id` is copied from `parent` (a child always belongs to its
    /// parent's thread) and `parent_waypoint_id` is set to
    /// `Some(parent.waypoint_id)`, so lineage is exactly the previous
    /// `Waypoint` of that thread by construction.
    ///
    /// `muster_progress` always starts `None` here -- see
    /// [`Waypoint::new_root`]'s rustdoc for why.
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
            muster_progress: None,
            checkpoint_ns: None,
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

    // --- CF-FR-15 / D-20: ThreadId::child ------------------------------

    #[test]
    fn child_thread_derivation_is_injective_under_adversarial_names() {
        // The exact CR-01 regression shape: a bare `format!("{parent}/{node}")`
        // join would produce the IDENTICAL string for both pairs below,
        // because `NodeId::new` validates nothing beyond non-emptiness --
        // node `"a/b"` under parent thread `"t"` joins to `"t/a/b"`, and
        // node `"b"` under parent thread `"t/a"` also joins to `"t/a/b"`.
        // The length-prefixed encoding must NOT collide here.
        let pair_a = ThreadId::child(&ThreadId::new("t").unwrap(), &NodeId::new("a/b")).unwrap();
        let pair_b = ThreadId::child(&ThreadId::new("t/a").unwrap(), &NodeId::new("b")).unwrap();
        assert_ne!(
            pair_a, pair_b,
            "length-prefixed derivation must not collide on the CR-01 shape"
        );

        // Repeat for a colon delimiter, in case a future encoding change
        // reintroduces a different bare-delimiter join.
        let colon_a = ThreadId::child(&ThreadId::new("t").unwrap(), &NodeId::new("a:b")).unwrap();
        let colon_b = ThreadId::child(&ThreadId::new("t:a").unwrap(), &NodeId::new("b")).unwrap();
        assert_ne!(colon_a, colon_b);
    }

    #[test]
    fn derived_child_thread_id_passes_thread_id_validation() {
        let parent = ThreadId::new("run-1").unwrap();
        let node = NodeId::new("subgraph-node");
        let child = ThreadId::child(&parent, &node).unwrap();

        // Re-validating the already-constructed child's own string through
        // `ThreadId::new` must succeed identically -- proving the derived
        // value is itself a fully valid `ThreadId` by the type's own rules,
        // not merely by having been wrapped in one already.
        assert!(ThreadId::new(child.as_str().to_string()).is_ok());
        assert!(!child.as_str().is_empty());
        assert!(!child.as_str().chars().any(char::is_whitespace));
        assert!(child.as_str().len() <= THREAD_ID_MAX_LEN);
    }

    #[test]
    fn derived_child_thread_id_exceeding_max_len_fails_typed_rather_than_truncating() {
        let parent = ThreadId::new("a".repeat(200)).unwrap();
        let node = NodeId::new("b".repeat(200));
        let result = ThreadId::child(&parent, &node);
        assert!(
            matches!(result, Err(ThreadIdError::TooLong { .. })),
            "an over-long derivation must fail typed, not silently truncate: {result:?}"
        );
    }

    #[test]
    fn nested_child_thread_ids_compose() {
        let root = ThreadId::new("root").unwrap();
        let child = ThreadId::child(&root, &NodeId::new("outer")).unwrap();
        let grandchild = ThreadId::child(&child, &NodeId::new("inner")).unwrap();

        assert_ne!(root, child);
        assert_ne!(child, grandchild);
        assert_ne!(root, grandchild);
        assert!(ThreadId::new(grandchild.as_str().to_string()).is_ok());
    }

    #[test]
    fn child_derivation_propagates_a_node_ids_whitespace_as_a_typed_error() {
        // `NodeId::new` validates nothing beyond wrapping the string (unlike
        // `ThreadId::new`, which rejects whitespace) -- a whitespace-carrying
        // `NodeId` must surface as a typed `ThreadIdError`, not silently
        // produce an invalid `ThreadId`.
        let parent = ThreadId::new("t").unwrap();
        let node = NodeId::new("has space");
        assert_eq!(
            ThreadId::child(&parent, &node),
            Err(ThreadIdError::ContainsWhitespace)
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
        assert!(a.as_str().starts_with("v4:"));
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
            muster_progress: None,
            checkpoint_ns: None,
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
            parley_id: ParleyId::new(),
            node_id: NodeId::new("asker"),
            kind: ParleyKind::FreeText,
            prompt: "please confirm".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: None,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        };
        let json = serde_json::to_string(&parley).unwrap();
        let restored: ParleyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parley, restored);
    }

    /// Test 5 (Phase 24 Plan 01): the reshaped `AwaitingInput` status --
    /// `{ parleys: Vec<ParleyRequest>, responses: Vec<ParleyResponse> }`
    /// (D-02) -- serialises and deserialises with both fields preserved,
    /// including a non-empty `responses` list (a partially-answered
    /// suspension).
    #[test]
    fn awaiting_input_status_round_trips_through_serde() {
        let request = ParleyRequest {
            parley_id: ParleyId::new(),
            node_id: NodeId::new("asker"),
            kind: ParleyKind::Approval,
            prompt: "proceed?".to_string(),
            payload: serde_json::json!({"amount": 42}),
            choices: None,
            expires_at: None,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        };
        let response = ParleyResponse {
            parley_id: request.parley_id,
            value: serde_json::json!(true),
            responded_by: Some("alice".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let status = WaypointStatus::AwaitingInput {
            parleys: vec![request.clone()],
            responses: vec![response.clone()],
        };

        let json = serde_json::to_string(&status).unwrap();
        let restored: WaypointStatus = serde_json::from_str(&json).unwrap();

        match restored {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(parleys, vec![request]);
                assert_eq!(responses, vec![response]);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
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
            muster_progress: None,
            checkpoint_ns: None,
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

    // --- CF-FR-12 / D-14: MusterProgress ------------------------------------

    #[test]
    fn waypoint_payload_without_muster_progress_field_deserializes_as_none() {
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
            superstep: 2,
            graph_fingerprint: GraphFingerprint::from_canonical_bytes(b"fixture"),
            battlefield: Battlefield::new(schema),
            vanguard: vec![],
            completed: vec![],
            status: WaypointStatus::Running,
            created_at: Utc::now(),
            schema_version: Waypoint::current_schema_version(),
            visit_counts: BTreeMap::new(),
            frontier: FrontierSnapshot::default(),
            muster_progress: Some(MusterProgress {
                node: NodeId::new("planner"),
                tasks: vec![],
                completed: BTreeMap::new(),
            }),
            checkpoint_ns: None,
        };

        // Simulate a pre-CF-FR-12 payload: serialize, then strip the
        // `muster_progress` key entirely before deserializing back, rather
        // than merely round-tripping the value already present.
        let mut value = serde_json::to_value(&waypoint).unwrap();
        value
            .as_object_mut()
            .expect("Waypoint serializes to a JSON object")
            .remove("muster_progress");
        assert!(
            !value.to_string().contains("muster_progress"),
            "the muster_progress key must be genuinely absent from the fixture payload"
        );

        let restored: Waypoint = serde_json::from_value(value).unwrap();
        assert_eq!(restored.muster_progress, None);
        // Every other field is untouched by the missing key.
        assert_eq!(restored.thread_id, waypoint.thread_id);
        assert_eq!(restored.superstep, waypoint.superstep);
    }

    #[test]
    fn muster_progress_round_trips_through_serde_json() {
        let mut completed = BTreeMap::new();
        let mut delta = StateDelta::new();
        delta.set_raw(FieldName::new("x").unwrap(), serde_json::json!("a-result"));
        completed.insert("a".to_string(), delta);

        let progress = MusterProgress {
            node: NodeId::new("planner"),
            tasks: vec![
                MusterTask {
                    worker: NodeId::new("worker"),
                    payload: serde_json::json!("payload-a"),
                    task_key: "a".to_string(),
                },
                MusterTask {
                    worker: NodeId::new("worker"),
                    payload: serde_json::json!("payload-b"),
                    task_key: "b".to_string(),
                },
            ],
            completed,
        };

        let json = serde_json::to_string(&progress).unwrap();
        let restored: MusterProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(progress, restored);
    }

    #[test]
    fn muster_progress_unfinished_tasks_excludes_completed_keys() {
        let mut completed = BTreeMap::new();
        completed.insert("a".to_string(), StateDelta::new());
        let progress = MusterProgress {
            node: NodeId::new("planner"),
            tasks: vec![
                MusterTask {
                    worker: NodeId::new("worker"),
                    payload: serde_json::json!("a"),
                    task_key: "a".to_string(),
                },
                MusterTask {
                    worker: NodeId::new("worker"),
                    payload: serde_json::json!("b"),
                    task_key: "b".to_string(),
                },
                MusterTask {
                    worker: NodeId::new("worker"),
                    payload: serde_json::json!("c"),
                    task_key: "c".to_string(),
                },
            ],
            completed,
        };

        let unfinished: Vec<String> = progress
            .unfinished_tasks()
            .into_iter()
            .map(|t| t.task_key)
            .collect();
        assert_eq!(unfinished, vec!["b".to_string(), "c".to_string()]);
    }

    // --- CF-FR-15 / D-20: checkpoint_ns -------------------------------------

    #[test]
    fn waypoint_payload_without_checkpoint_ns_deserializes_as_none() {
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
            superstep: 4,
            graph_fingerprint: GraphFingerprint::from_canonical_bytes(b"fixture"),
            battlefield: Battlefield::new(schema),
            vanguard: vec![],
            completed: vec![],
            status: WaypointStatus::Running,
            created_at: Utc::now(),
            schema_version: Waypoint::current_schema_version(),
            visit_counts: BTreeMap::new(),
            frontier: FrontierSnapshot::default(),
            muster_progress: None,
            checkpoint_ns: Some("outer/inner/".to_string()),
        };

        // Simulate a pre-CF-FR-15 payload: serialize, then strip the
        // `checkpoint_ns` key entirely before deserializing back, rather
        // than merely round-tripping the value already present.
        let mut value = serde_json::to_value(&waypoint).unwrap();
        value
            .as_object_mut()
            .expect("Waypoint serializes to a JSON object")
            .remove("checkpoint_ns");
        assert!(
            !value.to_string().contains("checkpoint_ns"),
            "the checkpoint_ns key must be genuinely absent from the fixture payload"
        );

        let restored: Waypoint = serde_json::from_value(value).unwrap();
        assert_eq!(restored.checkpoint_ns, None);
        // Every other field is untouched by the missing key.
        assert_eq!(restored.thread_id, waypoint.thread_id);
        assert_eq!(restored.superstep, waypoint.superstep);
    }

    #[test]
    fn checkpoint_ns_round_trips_through_serde_json() {
        let schema = BattlefieldSchema::new(vec![]);
        let waypoint = Waypoint {
            thread_id: ThreadId::new("thread-1").unwrap(),
            waypoint_id: WaypointId::new(),
            parent_waypoint_id: None,
            superstep: 1,
            graph_fingerprint: GraphFingerprint::from_canonical_bytes(b"fixture"),
            battlefield: Battlefield::new(schema),
            vanguard: vec![],
            completed: vec![],
            status: WaypointStatus::Running,
            created_at: Utc::now(),
            schema_version: Waypoint::current_schema_version(),
            visit_counts: BTreeMap::new(),
            frontier: FrontierSnapshot::default(),
            muster_progress: None,
            checkpoint_ns: Some("outer_node/inner_node/".to_string()),
        };

        let json = serde_json::to_string(&waypoint).unwrap();
        let restored: Waypoint = serde_json::from_str(&json).unwrap();
        assert_eq!(waypoint, restored);
        assert_eq!(
            restored.checkpoint_ns,
            Some("outer_node/inner_node/".to_string())
        );
    }
}
