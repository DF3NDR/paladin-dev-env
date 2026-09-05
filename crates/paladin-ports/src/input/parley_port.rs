//! Parley Port — Triggering a Resume Without Naming the Engine (HITL-05, D-25)
//!
//! [`ParleyPort`] lets an HTTP surface (`paladin-web`) trigger a suspended
//! thread's resume without ever naming `paladin-battalion`: every type in
//! [`ParleyPort::resume_with`]'s signature is either a `paladin-core` value
//! type or declared in this module (ADR-0031, ADR-0038). The facade adapter
//! (`src/application/services/parley/`, a later plan wave) implements this
//! trait over a real `WarEngine`, so `paladin-web` depends on this port
//! alone — never on `paladin-battalion` — in its default build.
//!
//! # The contract an implementor must honour
//!
//! - **Validation is synchronous and total.** Every way a submission can be
//!   rejected — an unknown thread, an unregistered graph, a thread that
//!   is not suspended, an unknown or already-answered `parley_id`, a
//!   response whose shape does not match its request's `ParleyKind`, or an
//!   expired parley — surfaces as a typed [`ParleyError`] variant from THIS
//!   call, before it returns. An error means nothing was persisted (beyond
//!   the sole documented exception: an expired parley under
//!   `OnExpire::FailRun` persists a `Failed` Waypoint as part of returning
//!   [`ParleyError::ParleyExpired`] — a policy outcome, not a caller-input
//!   rejection) and the thread is still suspended exactly as it was before
//!   the call.
//! - **A success means the continuation is already running in the
//!   background.** Once validation accepts the submission and every
//!   outstanding parley now has a response, the call returns immediately
//!   with a [`ResumeAccepted`] handle; the run itself continues on a task
//!   the caller does not hold a connection open for. A caller polls the
//!   thread's state (a separate read path) to observe the outcome.
//! - **A submission that leaves the thread still suspended (a valid but
//!   partial answer) also returns immediately, still successfully** — the
//!   call never blocks on work that has not started.
//!
//! # Examples
//!
//! ```
//! use async_trait::async_trait;
//! use paladin_core::platform::container::parley::ParleyResponse;
//! use paladin_core::platform::container::waypoint::ThreadId;
//! use paladin_ports::input::parley_port::{ParleyError, ParleyPort, ResumeAccepted};
//! use std::sync::Arc;
//!
//! struct AlwaysSuspended;
//!
//! #[async_trait]
//! impl ParleyPort for AlwaysSuspended {
//!     async fn resume_with(
//!         &self,
//!         thread: &ThreadId,
//!         _responses: Vec<ParleyResponse>,
//!     ) -> Result<ResumeAccepted, ParleyError> {
//!         Err(ParleyError::ThreadNotFound(thread.clone()))
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let port: Arc<dyn ParleyPort> = Arc::new(AlwaysSuspended);
//! let _ = port; // held as a trait object, exactly as `paladin-web` holds it
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use paladin_core::platform::container::parley::{ParleyId, ParleyResponse};
use paladin_core::platform::container::waypoint::{GraphFingerprint, ThreadId};

/// The accepted-and-running handle [`ParleyPort::resume_with`] returns on
/// success (D-25): the thread whose resume was accepted, doubling as the
/// handle a caller polls (a separate read path resolves a `ThreadId` to its
/// current state) since HTTP-layer concerns like a `state_url` string
/// belong to `paladin-web` (ADR-0038), not to this core-typed port.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::waypoint::ThreadId;
/// use paladin_ports::input::parley_port::ResumeAccepted;
///
/// let thread = ThreadId::new("demo-thread").unwrap();
/// let accepted = ResumeAccepted::new(thread.clone());
/// assert_eq!(accepted.thread_id(), &thread);
/// assert_eq!(accepted.state_handle(), &thread);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeAccepted {
    thread_id: ThreadId,
}

impl ResumeAccepted {
    /// Construct a handle for `thread_id`.
    pub fn new(thread_id: ThreadId) -> Self {
        Self { thread_id }
    }

    /// The thread whose resume was accepted.
    pub fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }

    /// The handle a client polls to observe the run's outcome. Identical to
    /// [`Self::thread_id`] today — a `ThreadId` alone is sufficient to
    /// resolve `GET /v1/threads/{id}/state` — kept as its own accessor so a
    /// future opaque handle shape (unrelated to the thread id) is not a
    /// breaking rename of this method.
    pub fn state_handle(&self) -> &ThreadId {
        &self.thread_id
    }
}

/// Every way [`ParleyPort::resume_with`] can reject a submission (D-25):
/// mirrors `paladin-battalion`'s `EngineError` D-10 validation variants
/// (`ThreadNotAwaitingInput`, `UnknownParleyId`, `ParleyAlreadyAnswered`,
/// `ResponseShapeInvalid`, `ParleyExpired`) plus the two failure modes only
/// the facade adapter can produce: `ThreadNotFound` (no Waypoint at all for
/// this thread) and `GraphNotRegistered` (the thread's own graph fingerprint
/// names a graph no code has registered, D-26).
///
/// `#[non_exhaustive]`: a future validation case can be added without
/// breaking an existing match, mirroring every other persisted/port error
/// enum in this codebase (`WaypointError`, `EngineError`).
///
/// Every per-parley variant carries its [`ParleyId`] so the HTTP layer
/// (`paladin-web`, a later plan) can place it in the error envelope's
/// `details` without re-deriving it from a message string.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParleyError {
    /// No Waypoint exists at all for the requested thread.
    #[error("thread not found: {0}")]
    ThreadNotFound(ThreadId),

    /// The thread's latest Waypoint names a [`GraphFingerprint`] no
    /// [`GraphRegistry`](https://github.com/DF3NDR/paladin-dev-env) has a
    /// registered `WarGraph` for (D-26). Never resolved by falling back to
    /// a default or "nearest" graph — an unregistered fingerprint is
    /// always this error.
    #[error("no graph registered for fingerprint {fingerprint}")]
    GraphNotRegistered {
        /// The fingerprint the thread's latest Waypoint carries.
        fingerprint: GraphFingerprint,
    },

    /// The thread's latest Waypoint is not suspended awaiting input: only
    /// an `AwaitingInput` thread can be advanced by delivering parley
    /// responses.
    #[error("thread {thread} is not awaiting input (status: {status})")]
    ThreadNotAwaitingInput {
        /// The thread `resume_with` was called against.
        thread: ThreadId,
        /// The loaded Waypoint's actual status, `Debug`-formatted.
        status: String,
    },

    /// A submitted response's `parley_id` does not match any request on the
    /// thread's own outstanding parleys.
    #[error("unknown parley id: {parley_id}")]
    UnknownParleyId {
        /// The response's `parley_id`, absent from the thread's outstanding
        /// parleys.
        parley_id: ParleyId,
    },

    /// A submitted response names a `parley_id` that already has an
    /// accepted answer — either from a prior `resume_with` call, or a
    /// second response for the same `parley_id` within this SAME
    /// submission (both are rejected; see `EngineError::ParleyAlreadyAnswered`'s
    /// own rustdoc for the exact "both rejected" resolution this mirrors).
    #[error("parley already answered: {parley_id}")]
    ParleyAlreadyAnswered {
        /// The `parley_id` a response was submitted for that already has an
        /// accepted answer.
        parley_id: ParleyId,
    },

    /// A submitted response's value does not satisfy its own request's
    /// `ParleyKind` (Approval/Choice/FreeText/StateEdit).
    #[error("parley {parley_id} response shape invalid: {reason}")]
    ResponseShapeInvalid {
        /// The parley whose submitted value failed shape validation.
        parley_id: ParleyId,
        /// Why the value was rejected.
        reason: String,
    },

    /// An outstanding parley's `expires_at` has passed, evaluated lazily
    /// against `Utc::now()` at resume time. Under `on_expire: FailRun` this
    /// error is returned AFTER a `Failed` Waypoint naming the expired
    /// parley is persisted — the one documented exception to "an error
    /// leaves the thread suspended with nothing persisted."
    #[error("parley {parley_id} expired at {expires_at}")]
    ParleyExpired {
        /// The expired parley.
        parley_id: ParleyId,
        /// When it expired.
        expires_at: DateTime<Utc>,
    },

    /// The underlying `WaypointPort` backend failed (a genuine I/O/backend
    /// error, never a caller-input rejection) while loading the thread's
    /// latest Waypoint. Added while implementing the facade adapter
    /// (`src/application/services/parley/adapter.rs`, Task 2, Rule 2): a
    /// real backend failure must surface as something other than
    /// `ThreadNotFound`, which would misleadingly read as a 404 rather
    /// than a 5xx-class failure to an HTTP caller (a later plan).
    #[error("waypoint backend error: {source}")]
    Backend {
        /// The underlying backend error.
        #[source]
        source: crate::output::waypoint_port::WaypointError,
    },

    /// A future validation case a mapping from `paladin-battalion`'s
    /// `EngineError` does not yet name explicitly (D-25's facade adapter
    /// maps every named `EngineError` variant onto its own `ParleyError`
    /// counterpart; this is the fail-closed catch-all `EngineError`'s own
    /// `#[non_exhaustive]` status requires every downstream match to
    /// carry). Added alongside [`Self::Backend`] (Task 2, Rule 2) rather
    /// than silently dropping or panicking on an unmapped variant.
    #[error("resume rejected: {reason}")]
    Rejected {
        /// The underlying, unmapped failure's own message.
        reason: String,
    },
}

/// Port trait for triggering a suspended thread's resume (HITL-05, D-25).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow sharing across async
/// tasks, mirroring every other port trait in this crate.
///
/// See the module-level documentation for the full validate-then-background
/// contract every implementor must honour.
#[async_trait]
pub trait ParleyPort: Send + Sync {
    /// Deliver `responses` to `thread`'s outstanding parleys.
    ///
    /// Every parameter and the success type name only `paladin-core` types
    /// or types declared in this module — no `paladin-battalion` type
    /// appears in this signature, so `paladin-web` can depend on this port
    /// without a default-build edge to `paladin-battalion` (ADR-0031).
    async fn resume_with(
        &self,
        thread: &ThreadId,
        responses: Vec<ParleyResponse>,
    ) -> Result<ResumeAccepted, ParleyError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Test 1: the trait can be held as `Arc<dyn ParleyPort>` -- a
    /// compile-level object-safety assertion.
    #[test]
    fn parley_port_is_object_safe() {
        let _: Option<Arc<dyn ParleyPort>> = None;
    }

    fn sample_parley_id() -> ParleyId {
        ParleyId::new()
    }

    /// Test 2: a match over `ParleyError` covers every validation case.
    /// `ParleyError` is `#[non_exhaustive]`, but a match inside its OWN
    /// defining crate may still be exhaustive with no wildcard arm --
    /// `#[non_exhaustive]` only forces a wildcard on a match written in a
    /// DOWNSTREAM crate.
    #[test]
    fn parley_error_covers_every_validation_case() {
        fn label(err: &ParleyError) -> &'static str {
            match err {
                ParleyError::ThreadNotFound(_) => "thread_not_found",
                ParleyError::GraphNotRegistered { .. } => "graph_not_registered",
                ParleyError::ThreadNotAwaitingInput { .. } => "thread_not_awaiting_input",
                ParleyError::UnknownParleyId { .. } => "unknown_parley_id",
                ParleyError::ParleyAlreadyAnswered { .. } => "parley_already_answered",
                ParleyError::ResponseShapeInvalid { .. } => "response_shape_invalid",
                ParleyError::ParleyExpired { .. } => "parley_expired",
                ParleyError::Backend { .. } => "backend",
                ParleyError::Rejected { .. } => "rejected",
            }
        }

        let thread = ThreadId::new("t1").unwrap();
        let parley_id = sample_parley_id();
        let cases = vec![
            (
                ParleyError::ThreadNotFound(thread.clone()),
                "thread_not_found",
            ),
            (
                ParleyError::GraphNotRegistered {
                    fingerprint: GraphFingerprint::from_canonical_bytes(b"g1"),
                },
                "graph_not_registered",
            ),
            (
                ParleyError::ThreadNotAwaitingInput {
                    thread: thread.clone(),
                    status: "Running".to_string(),
                },
                "thread_not_awaiting_input",
            ),
            (
                ParleyError::UnknownParleyId { parley_id },
                "unknown_parley_id",
            ),
            (
                ParleyError::ParleyAlreadyAnswered { parley_id },
                "parley_already_answered",
            ),
            (
                ParleyError::ResponseShapeInvalid {
                    parley_id,
                    reason: "bad shape".to_string(),
                },
                "response_shape_invalid",
            ),
            (
                ParleyError::ParleyExpired {
                    parley_id,
                    expires_at: Utc::now(),
                },
                "parley_expired",
            ),
            (
                ParleyError::Backend {
                    source: crate::output::waypoint_port::WaypointError::Serialization(
                        "boom".to_string(),
                    ),
                },
                "backend",
            ),
            (
                ParleyError::Rejected {
                    reason: "unmapped engine error".to_string(),
                },
                "rejected",
            ),
        ];

        for (err, expected) in &cases {
            assert_eq!(label(err), *expected);
        }
        assert_eq!(cases.len(), 9, "every ParleyError variant must be covered");
    }

    /// Test 3: the per-parley variants' `Display` output names the
    /// `ParleyId` so the HTTP layer can place it in the error envelope's
    /// details.
    #[test]
    fn parley_error_display_names_the_parley_id() {
        let parley_id = sample_parley_id();

        let unknown = ParleyError::UnknownParleyId { parley_id };
        assert!(unknown.to_string().contains(&parley_id.to_string()));

        let already = ParleyError::ParleyAlreadyAnswered { parley_id };
        assert!(already.to_string().contains(&parley_id.to_string()));

        let shape_invalid = ParleyError::ResponseShapeInvalid {
            parley_id,
            reason: "must be a string".to_string(),
        };
        assert!(shape_invalid.to_string().contains(&parley_id.to_string()));
        assert!(shape_invalid.to_string().contains("must be a string"));

        let expired = ParleyError::ParleyExpired {
            parley_id,
            expires_at: Utc::now(),
        };
        assert!(expired.to_string().contains(&parley_id.to_string()));
    }

    /// Test 4: `ResumeAccepted` exposes the thread id and the handle a
    /// client polls.
    #[test]
    fn resume_accepted_carries_thread_and_state_handle() {
        let thread = ThreadId::new("resume-accepted-thread").unwrap();
        let accepted = ResumeAccepted::new(thread.clone());
        assert_eq!(accepted.thread_id(), &thread);
        assert_eq!(accepted.state_handle(), &thread);
    }
}
