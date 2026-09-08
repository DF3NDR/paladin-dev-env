//! Run Submission Port — Triggering a Run Without Naming the Engine (D-12)
//!
//! [`RunSubmissionPort`] lets `paladin-web` submit a run without ever
//! naming `paladin-battalion`, mirroring [`crate::input::parley_port::ParleyPort`]'s
//! "facade implements, `paladin-web` calls" direction exactly (D-12: "exactly
//! Phase 24's split"). Every type in [`RunSubmissionPort::submit`]'s
//! signature is either a `paladin-core` value type or declared in this
//! module (ADR-0031, ADR-0038).

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::run::{RunId, RunStatus, WebhookSpec};
use paladin_core::platform::container::user::UserRole;
use paladin_core::platform::container::waypoint::ThreadId;

/// A run submission request (D-12).
#[derive(Debug, Clone)]
pub struct SubmitRun {
    /// The assistant to run.
    pub assistant_id: String,
    /// A specific version, or `None` to resolve `latest` at submit time
    /// (D-30).
    pub version: Option<u32>,
    /// An existing thread to run against, or `None` to start a fresh one.
    pub thread_id: Option<ThreadId>,
    /// The caller-supplied input.
    pub input: serde_json::Value,
    /// An optional webhook delivery target for this run's lifecycle events.
    pub webhook: Option<WebhookSpec>,
    /// The identity and role of the submitting principal, if known.
    pub requested_by: Option<(String, UserRole)>,
}

/// The accepted handle [`RunSubmissionPort::submit`] returns on success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunAccepted {
    /// The newly created run's identity.
    pub run_id: RunId,
    /// The thread the run executes against (freshly generated when the
    /// request did not supply one).
    pub thread_id: ThreadId,
}

/// The result of a successful [`RunSubmissionPort::cancel`] call (D-16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelOutcome {
    /// The cancelled run's identity.
    pub run_id: RunId,
    /// The run's status immediately after the durable flag was written --
    /// always a non-terminal, "busy" status (`Queued`/`Running`/
    /// `AwaitingInput`), since a terminal run is rejected with
    /// [`RunSubmissionError::AlreadyTerminal`] before this is constructed.
    pub status: RunStatus,
    /// Whether THIS call site's own process instance held an in-process
    /// cancellation signal for the run and fired it directly (a same-
    /// instance fast path). `false` when the run is executing on a
    /// different instance, or not currently dispatched by any worker at
    /// all -- the durable flag written first is what reaches it either way.
    pub was_local: bool,
}

/// Every way [`RunSubmissionPort::submit`] can reject a request (D-12).
///
/// `#[non_exhaustive]`: 27-07 adds `cancel`-shaped variants, 27-13 adds
/// `WebhookRejected`, 27-15 adds fork-shaped variants.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RunSubmissionError {
    /// No assistant is registered under this id.
    #[error("unknown assistant: {assistant_id}")]
    UnknownAssistant {
        /// The requested assistant id.
        assistant_id: String,
    },
    /// The assistant exists, but not at the requested version.
    #[error("unknown version {version} for assistant {assistant_id}")]
    UnknownVersion {
        /// The requested assistant id.
        assistant_id: String,
        /// The requested (unknown) version.
        version: u32,
    },
    /// The target thread already has an active run (D-17/D-18).
    #[error("thread busy: {thread_id}")]
    ThreadBusy {
        /// The busy thread.
        thread_id: ThreadId,
    },
    /// The principal is not permitted to invoke this assistant.
    #[error("forbidden: {reason}")]
    Forbidden {
        /// Why the request was rejected.
        reason: String,
    },
    /// The request body failed submit-time shape validation (the assistant
    /// reference or the request shape itself — never a schema violation on
    /// `input`, which surfaces later as a `Failed` run).
    #[error("invalid input: {message}")]
    InvalidInput {
        /// Description of the validation failure.
        message: String,
    },
    /// The referenced run does not exist.
    #[error("run not found: {run_id}")]
    NotFound {
        /// The requested run id.
        run_id: RunId,
    },
    /// The referenced run is already terminal.
    #[error("run {run_id} is already terminal ({status})")]
    AlreadyTerminal {
        /// The terminal run.
        run_id: RunId,
        /// Its current (terminal) status.
        status: RunStatus,
    },
    /// A genuine backend failure (repository or queue), never a caller-input
    /// rejection.
    #[error("run submission backend error: {message}")]
    Backend {
        /// Description of the backend failure.
        message: String,
    },
    /// No run store/queue is configured (the D-24 501 precedent).
    #[error("run submission is not wired: configure run_store and run_queue")]
    NotWired,
}

/// Port trait for submitting a run (D-12).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`, mirroring every other port trait
/// in this crate.
#[async_trait]
pub trait RunSubmissionPort: Send + Sync {
    /// Submit `request`, resolving its assistant reference, persisting and
    /// enqueuing a new [`Run`](paladin_core::platform::container::run::Run),
    /// and returning immediately with a [`RunAccepted`] handle.
    async fn submit(&self, request: SubmitRun) -> Result<RunAccepted, RunSubmissionError>;

    /// Idempotently request cancellation of `run_id` (D-16): persists the
    /// durable flag through the repository FIRST -- so a cancel is never
    /// lost to a crash between the durable write and any local signal --
    /// then best-effort signals this instance's own in-process
    /// cancellation token if it is the one currently dispatching the run.
    ///
    /// Calling this twice on the same non-terminal run is `Ok` both times
    /// (idempotent). Returns [`RunSubmissionError::NotFound`] when `run_id`
    /// does not exist, and [`RunSubmissionError::AlreadyTerminal`] when the
    /// run has already reached a terminal status.
    async fn cancel(&self, run_id: &RunId) -> Result<CancelOutcome, RunSubmissionError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct AlwaysUnwired;

    #[async_trait]
    impl RunSubmissionPort for AlwaysUnwired {
        async fn submit(&self, _request: SubmitRun) -> Result<RunAccepted, RunSubmissionError> {
            Err(RunSubmissionError::NotWired)
        }

        async fn cancel(&self, _run_id: &RunId) -> Result<CancelOutcome, RunSubmissionError> {
            Err(RunSubmissionError::NotWired)
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn RunSubmissionPort>> = None;
    }

    #[tokio::test]
    async fn always_unwired_returns_not_wired() {
        let port = AlwaysUnwired;
        let request = SubmitRun {
            assistant_id: "a1".to_string(),
            version: None,
            thread_id: None,
            input: serde_json::json!({}),
            webhook: None,
            requested_by: None,
        };
        let err = port.submit(request).await.unwrap_err();
        assert!(matches!(err, RunSubmissionError::NotWired));
    }

    #[test]
    fn run_accepted_carries_run_and_thread_id() {
        let run_id = RunId::new_v7();
        let thread_id = ThreadId::new("t1").unwrap();
        let accepted = RunAccepted {
            run_id: run_id.clone(),
            thread_id: thread_id.clone(),
        };
        assert_eq!(accepted.run_id, run_id);
        assert_eq!(accepted.thread_id, thread_id);
    }
}
