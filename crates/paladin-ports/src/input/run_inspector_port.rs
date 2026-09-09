//! Run Inspector Port — a core-typed view of "what happened on this
//! thread" (D-24, OBS-03 / OBS-FR-10).
//!
//! [`RunInspectorPort`] lets `paladin-web` render the `dev-ui` run
//! inspector page (`GET /v1/dev-ui/threads/{id}`) without ever learning the
//! graph vocabulary -- it never imports the battalion orchestration crate,
//! never sees a `GraphShape`, a `WarGraph`, or a `TraceEvent` (ADR-0031). Every field on
//! [`InspectorView`] is a core value type (ADR-0016), mirroring
//! [`crate::input::run_event_stream_port`]'s own "core-typed only"
//! convention -- one input port, one method, the facade decides everything
//! behind it.
//!
//! # `field_changes` carries names only, always (T-28-14-01)
//!
//! [`SuperstepRow::field_changes`] is `Vec<FieldName>` by TYPE -- there is
//! no configuration, flag, or opt-in that puts a `Battlefield` field VALUE
//! anywhere on this view. A `DeltaMerged` trace record's own opt-in
//! `value`/`value_bytes` (D-05, `trace.state_values`) is never read by the
//! facade implementation of this port. This is a type-level guarantee, not
//! a redaction step: there is nowhere on [`InspectorView`] a value could be
//! placed even by mistake.
//!
//! # `ThreadId` is not an authorization boundary
//!
//! Same caveat as
//! [`WaypointPort`](crate::output::waypoint_port::WaypointPort)'s: a
//! `ThreadId` is a caller-supplied workflow identifier, not a capability
//! token. The `dev-ui` route's `require_auth` + `require_admin` layers
//! (D-25) are the authorization boundary; this port neither performs nor
//! implies one.
//!
//! # Examples
//!
//! ```
//! use async_trait::async_trait;
//! use paladin_core::platform::container::waypoint::ThreadId;
//! use paladin_ports::input::run_inspector_port::{
//!     InspectorError, InspectorSource, InspectorView, RunInspectorPort,
//! };
//! use std::sync::Arc;
//!
//! struct AlwaysUnwired;
//!
//! #[async_trait]
//! impl RunInspectorPort for AlwaysUnwired {
//!     async fn inspect(&self, _thread: &ThreadId) -> Result<InspectorView, InspectorError> {
//!         Err(InspectorError::NotWired)
//!     }
//! }
//!
//! # fn main() {
//! let port: Arc<dyn RunInspectorPort> = Arc::new(AlwaysUnwired);
//! let _ = port; // held as a trait object, exactly as `paladin-web` holds it
//! let _ = InspectorSource::Waypoints; // keeps the import exercised
//! # }
//! ```

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::battlefield::FieldName;
use paladin_core::platform::container::run::{RunId, RunStatus};
use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, ThreadId, WaypointId};

/// One node's execution within one superstep, as rendered for the
/// inspector page. Mirrors
/// [`NodeExecutionRecord`](paladin_core::platform::container::waypoint::NodeExecutionRecord)'s
/// fields but never carries a `Battlefield` value.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletedRow {
    /// The node that ran.
    pub node_id: NodeId,
    /// The 1-indexed attempt that produced this row's `outcome`.
    pub attempt: u32,
    /// This attempt's outcome.
    pub outcome: NodeOutcomeKind,
    /// How long this attempt took, in milliseconds.
    pub duration_ms: u64,
    /// Tokens consumed by this attempt.
    pub token_count: u64,
    /// Whether this attempt's outcome was served from the node cache
    /// (FT-06) rather than by executing the node.
    pub cache_hit: bool,
}

/// One superstep's row on the inspector's superstep table (D-24): the
/// waypoint it produced, what was dispatched into it, what ran, which
/// fields changed (by NAME only), and which edges fired into it.
#[derive(Debug, Clone, PartialEq)]
pub struct SuperstepRow {
    /// The superstep index this row describes.
    pub superstep: u64,
    /// The Waypoint this superstep produced.
    pub waypoint_id: WaypointId,
    /// The nodes dispatched into this superstep.
    pub vanguard: Vec<NodeId>,
    /// Every node execution this superstep, in `Waypoint::completed`
    /// order.
    pub completed: Vec<CompletedRow>,
    /// The Battlefield fields this superstep's merge changed, by NAME
    /// only (T-28-14-01) -- never a value, regardless of whether a
    /// persisted trace carries one.
    pub field_changes: Vec<FieldName>,
    /// The edges that fired FROM a node completed this superstep --
    /// derived (Waypoints source) or exact (Trace source), matching
    /// [`ExecutionOverlay`](https://docs.rs/paladin-ai-battalion)'s own
    /// per-overlay `fired_edges` set, filtered to this row's own
    /// completed nodes.
    pub fired_edges: Vec<(NodeId, NodeId)>,
}

/// A node's aggregate visit history across the whole thread (D-24): answers
/// the OBS-03 acceptance question ("node X ran 3 times: supersteps 2, 4,
/// 6") without the caller re-deriving it from [`SuperstepRow`]s.
#[derive(Debug, Clone, PartialEq)]
pub struct VisitSummary {
    /// The visited node.
    pub node_id: NodeId,
    /// How many times this node ran.
    pub count: u32,
    /// The superstep index of each visit, ascending.
    pub supersteps: Vec<u64>,
}

/// Where an [`InspectorView`]'s data came from (mirrors the battalion
/// crate's own `OverlaySource`'s two variants, restated here core-typed so
/// this port never names the battalion orchestration crate, ADR-0031).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorSource {
    /// Built from Waypoint history alone: `fired_edges` is derived.
    Waypoints,
    /// Built from a persisted trace: `fired_edges` is exact.
    Trace,
}

/// Everything the `dev-ui` inspector page needs for one thread (D-24,
/// OBS-FR-10): the rendered diagram, the superstep table, and the
/// aggregate visit summaries -- one call, no follow-up query, no graph
/// vocabulary leaked to the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectorView {
    /// The inspected thread.
    pub thread_id: ThreadId,
    /// The run this thread belongs to, if one exists. `None` for a thread
    /// with Waypoint history but no associated `Run` row.
    pub run_id: Option<RunId>,
    /// The run's current status, if a run exists.
    pub status: Option<RunStatus>,
    /// The rendered Mermaid execution-overlay diagram
    /// (`to_mermaid_overlay`'s own output) -- ready to embed verbatim.
    pub mermaid: String,
    /// Whether [`Self::mermaid`] was rendered onto the observed-only
    /// fallback shape (D-22) because no static graph document was
    /// available for this thread.
    pub observed_only: bool,
    /// Where this view's edge data came from.
    pub source: InspectorSource,
    /// One row per superstep, ascending.
    pub supersteps: Vec<SuperstepRow>,
    /// One summary per visited node.
    pub visits: Vec<VisitSummary>,
}

/// Errors [`RunInspectorPort::inspect`] can return.
///
/// `#[non_exhaustive]`, mirroring every other port error enum in this
/// crate (`RunStreamError`, `WaypointError`): a future variant can be
/// added without breaking an existing downstream `match`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InspectorError {
    /// No thread exists with the given id -- neither a `Run` row nor any
    /// Waypoint history.
    #[error("thread not found: {thread_id}")]
    ThreadNotFound {
        /// The thread id that was not found.
        thread_id: ThreadId,
    },
    /// The underlying repository, Waypoint or trace backend failed (a
    /// genuine I/O/backend error, never a caller-input rejection).
    #[error("run inspector backend error: {message}")]
    Backend {
        /// A description of the backend failure.
        message: String,
    },
    /// No run inspector backend is configured (mirrors
    /// [`RunStreamError::NotWired`](crate::input::run_event_stream_port::RunStreamError::NotWired)'s
    /// `501` precedent, D-25).
    #[error("no run inspector backend configured")]
    NotWired,
}

/// Port trait for inspecting one thread's execution history (D-24,
/// OBS-03 / OBS-FR-10).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`, mirroring every other port trait
/// in this crate.
#[async_trait]
pub trait RunInspectorPort: Send + Sync {
    /// Inspect `thread`.
    ///
    /// Returns [`InspectorError::ThreadNotFound`] if no `Run` row and no
    /// Waypoint history exist for `thread`. A known thread with no
    /// completed superstep yet returns `Ok` with empty `supersteps` and
    /// `visits` -- the empty case is a valid view, not a failure.
    async fn inspect(&self, thread: &ThreadId) -> Result<InspectorView, InspectorError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Test 1: the trait can be held as `Arc<dyn RunInspectorPort>` -- a
    /// compile-level object-safety assertion.
    #[test]
    fn run_inspector_port_is_object_safe() {
        let _: Option<Arc<dyn RunInspectorPort>> = None;
        let _: Option<Box<dyn RunInspectorPort>> = None;
    }

    /// Test 2: a match over `InspectorError` covers every variant declared
    /// today -- `#[non_exhaustive]` only forces a wildcard arm on a match
    /// written in a DOWNSTREAM crate, not inside this defining crate.
    #[test]
    fn inspector_error_covers_every_variant() {
        fn label(err: &InspectorError) -> &'static str {
            match err {
                InspectorError::ThreadNotFound { .. } => "thread_not_found",
                InspectorError::Backend { .. } => "backend",
                InspectorError::NotWired => "not_wired",
            }
        }

        let thread_id = ThreadId::new("t1").unwrap();
        let cases = vec![
            (
                InspectorError::ThreadNotFound {
                    thread_id: thread_id.clone(),
                },
                "thread_not_found",
            ),
            (
                InspectorError::Backend {
                    message: "boom".to_string(),
                },
                "backend",
            ),
            (InspectorError::NotWired, "not_wired"),
        ];
        for (err, expected) in &cases {
            assert_eq!(label(err), *expected);
        }
        assert_eq!(
            cases.len(),
            3,
            "every InspectorError variant must be covered"
        );
    }

    /// Test 3: `ThreadNotFound`'s `Display` output names the thread id, so
    /// an HTTP layer can surface it without re-deriving it from a message
    /// string.
    #[test]
    fn inspector_error_thread_not_found_displays_the_thread_id() {
        let thread_id = ThreadId::new("t1").unwrap();
        let err = InspectorError::ThreadNotFound {
            thread_id: thread_id.clone(),
        };
        assert!(err.to_string().contains(thread_id.as_str()));
    }
}
