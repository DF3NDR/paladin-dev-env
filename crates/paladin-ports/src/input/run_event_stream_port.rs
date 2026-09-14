//! Run Event Stream Port — a `Stream` of core `RunStreamEvent`s (D-27, PLAT-FR-07)
//!
//! [`RunEventStreamPort`] lets `paladin-web` open `GET /v1/runs/{run_id}/stream`
//! without ever naming the per-run broadcast bus, `paladin-battalion`'s
//! `WarEngine`, or `TraceEvent` (ADR-0031, mirroring [`crate::input::parley_port`]'s
//! own "core-typed only" convention). The facade decides live-versus-degraded
//! (D-26) entirely behind [`RunEventStreamPort::stream`]'s return type; the
//! controller's only job is SSE framing and heartbeats.
//!
//! # Examples
//!
//! ```
//! use async_trait::async_trait;
//! use futures::stream;
//! use paladin_core::platform::container::run::RunId;
//! use paladin_ports::input::run_event_stream_port::{
//!     RunEventStream, RunEventStreamPort, RunStreamError,
//! };
//! use std::sync::Arc;
//!
//! struct AlwaysUnwired;
//!
//! #[async_trait]
//! impl RunEventStreamPort for AlwaysUnwired {
//!     async fn stream(&self, _run_id: &RunId) -> Result<RunEventStream, RunStreamError> {
//!         Err(RunStreamError::NotWired)
//!     }
//! }
//!
//! # fn main() {
//! let port: Arc<dyn RunEventStreamPort> = Arc::new(AlwaysUnwired);
//! let _ = port; // held as a trait object, exactly as `paladin-web` holds it
//! let _ = stream::empty::<()>(); // keeps the `futures` import exercised
//! # }
//! ```

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use thiserror::Error;

use paladin_core::platform::container::run::{RunId, RunStreamEvent};

/// A boxed stream of [`RunStreamEvent`]s. The facade's live/degraded
/// decision (D-26) is entirely behind this type -- `paladin-web` only ever
/// sees a stream of core events, never a `broadcast::Receiver` or a
/// `TraceEvent`.
pub type RunEventStream = Pin<Box<dyn Stream<Item = RunStreamEvent> + Send>>;

/// Errors [`RunEventStreamPort::stream`] can return.
///
/// `#[non_exhaustive]`, mirroring every other port error enum in this crate
/// (`WaypointError`, `ParleyError`): a future variant can be added without
/// breaking an existing downstream `match`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RunStreamError {
    /// No run exists with the given id.
    #[error("run not found: {run_id}")]
    NotFound {
        /// The run id that was not found.
        run_id: RunId,
    },
    /// The underlying repository or waypoint backend failed (a genuine
    /// I/O/backend error, never a caller-input rejection).
    #[error("run event stream backend error: {message}")]
    Backend {
        /// A description of the backend failure.
        message: String,
    },
    /// No run event stream backend is configured (D-44's `501` precedent).
    #[error("no run event stream backend configured")]
    NotWired,
}

/// Port trait for opening a run's event stream (D-27, PLAT-FR-07).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`, mirroring every other port trait
/// in this crate.
#[async_trait]
pub trait RunEventStreamPort: Send + Sync {
    /// Open `run_id`'s event stream.
    ///
    /// Returns [`RunStreamError::NotFound`] if no run exists with that id.
    /// Otherwise the returned stream is live (the run is executing on THIS
    /// instance) or degraded (executing elsewhere, or already terminal) --
    /// the caller cannot tell which from the return type alone, by design
    /// (D-26/D-27): every event's own `mode` field carries that
    /// information instead.
    async fn stream(&self, run_id: &RunId) -> Result<RunEventStream, RunStreamError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Test 1: the trait can be held as `Arc<dyn RunEventStreamPort>` -- a
    /// compile-level object-safety assertion.
    #[test]
    fn run_event_stream_port_is_object_safe() {
        let _: Option<Arc<dyn RunEventStreamPort>> = None;
    }

    /// Test 2: a match over `RunStreamError` covers every variant declared
    /// today -- `#[non_exhaustive]` only forces a wildcard arm on a match
    /// written in a DOWNSTREAM crate, not inside this defining crate.
    #[test]
    fn run_stream_error_covers_every_variant() {
        fn label(err: &RunStreamError) -> &'static str {
            match err {
                RunStreamError::NotFound { .. } => "not_found",
                RunStreamError::Backend { .. } => "backend",
                RunStreamError::NotWired => "not_wired",
            }
        }

        let run_id = RunId::new_v7();
        let cases = vec![
            (RunStreamError::NotFound { run_id }, "not_found"),
            (
                RunStreamError::Backend {
                    message: "boom".to_string(),
                },
                "backend",
            ),
            (RunStreamError::NotWired, "not_wired"),
        ];
        for (err, expected) in &cases {
            assert_eq!(label(err), *expected);
        }
        assert_eq!(
            cases.len(),
            3,
            "every RunStreamError variant must be covered"
        );
    }

    /// Test 3: `NotFound`'s `Display` output names the run id, so an HTTP
    /// layer can surface it without re-deriving it from a message string.
    #[test]
    fn run_stream_error_not_found_displays_the_run_id() {
        let run_id = RunId::new_v7();
        let err = RunStreamError::NotFound {
            run_id: run_id.clone(),
        };
        assert!(err.to_string().contains(run_id.as_str()));
    }
}
