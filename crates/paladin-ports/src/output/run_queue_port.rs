//! Run Queue Port — Lease-Based Work Queue for Runs (D-06)
//!
//! [`RunQueuePort`] is a NEW port distinct from
//! [`crate::output::queue_port::QueuePort`] (a general queue-lifecycle
//! trait with no lease token). Adding lease semantics to that trait would
//! break every existing implementor (X-10.4); `RunQueuePort` carries PRD
//! 06 §2.2's signature verbatim instead.
//!
//! ## A pointer, not a payload (D-07)
//!
//! [`QueuedRun`] carries only `run_id`/`thread_id`/`attempt`/`enqueued_at`
//! — never the run's `input`. A worker always re-reads the full `Run`
//! through `RunRepositoryPort` on dequeue: the repository is the single
//! source of truth for status, and a message that embedded the input could
//! disagree with the database after a resume or a cancel.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::waypoint::ThreadId;

/// An opaque lease token identifying one in-flight dequeue, shared across
/// backends (the InMemory adapter and the Redis ZSET+Lua lease adapter both
/// express it as a plain string).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeaseToken(String);

impl LeaseToken {
    /// Wrap a caller- or backend-generated token string.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// Borrow the token as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LeaseToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A queued pointer to a run awaiting a worker (D-07): never the run's own
/// `input`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedRun {
    /// The run to dispatch.
    pub run_id: RunId,
    /// The run's thread (carried so a worker can log/trace without a
    /// repository round trip before it needs one).
    pub thread_id: ThreadId,
    /// The run's attempt counter as of enqueue time.
    pub attempt: u32,
    /// When this message was enqueued.
    pub enqueued_at: DateTime<Utc>,
}

/// A [`QueuedRun`] currently leased by one worker, together with the token
/// that must be presented to [`RunQueuePort::extend_lease`],
/// [`RunQueuePort::ack`] or [`RunQueuePort::nack`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeasedRun {
    /// The dequeued pointer.
    pub queued: QueuedRun,
    /// The lease token for this dequeue.
    pub token: LeaseToken,
}

/// Errors returned by [`RunQueuePort`] methods (X-06).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum QueueError {
    /// The presented lease has already expired (visibility timeout passed).
    #[error("lease expired: {token}")]
    LeaseExpired {
        /// The expired token.
        token: LeaseToken,
    },
    /// The presented lease token is not currently held by anyone.
    #[error("unknown lease: {token}")]
    UnknownLease {
        /// The unrecognized token.
        token: LeaseToken,
    },
    /// The underlying queue backend failed.
    #[error("run queue backend error: {message}")]
    Backend {
        /// Description of the backend failure.
        message: String,
    },
    /// A queued message could not be (de)serialized.
    #[error("run queue serialization error: {message}")]
    Serialization {
        /// Description of the serialization failure.
        message: String,
    },
}

/// Port trait for a lease-based work queue of runs (D-06, PRD 06 §2.2).
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: multiple worker tasks dequeue
/// concurrently.
#[async_trait]
pub trait RunQueuePort: Send + Sync {
    /// Enqueue a run pointer for dispatch.
    async fn enqueue(&self, run: QueuedRun) -> Result<(), QueueError>;

    /// Dequeue the next visible message, leasing it for `lease`. `Ok(None)`
    /// if the queue is empty (not an error).
    async fn dequeue(&self, lease: Duration) -> Result<Option<LeasedRun>, QueueError>;

    /// Extend an in-flight lease by `lease` from now.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::LeaseExpired`] or [`QueueError::UnknownLease`]
    /// if `token` no longer identifies a held lease.
    async fn extend_lease(&self, token: &LeaseToken, lease: Duration) -> Result<(), QueueError>;

    /// Acknowledge successful processing, removing the message permanently.
    async fn ack(&self, token: &LeaseToken) -> Result<(), QueueError>;

    /// Negatively acknowledge, making the message visible again after
    /// `requeue_delay`.
    async fn nack(&self, token: &LeaseToken, requeue_delay: Duration) -> Result<(), QueueError>;

    /// The current number of messages not yet acked (queued + leased).
    async fn depth(&self) -> Result<u64, QueueError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Arc<dyn RunQueuePort>> = None;
    }

    #[test]
    fn lease_token_round_trips_display_and_as_str() {
        let token = LeaseToken::new("abc-123");
        assert_eq!(token.as_str(), "abc-123");
        assert_eq!(token.to_string(), "abc-123");
    }

    #[test]
    fn queued_run_round_trips_through_serde_json() {
        let queued = QueuedRun {
            run_id: RunId::new_v7(),
            thread_id: ThreadId::new("t1").unwrap(),
            attempt: 1,
            enqueued_at: Utc::now(),
        };
        let json = serde_json::to_string(&queued).unwrap();
        let restored: QueuedRun = serde_json::from_str(&json).unwrap();
        assert_eq!(queued, restored);
    }

    #[test]
    fn leased_run_round_trips_through_serde_json() {
        let leased = LeasedRun {
            queued: QueuedRun {
                run_id: RunId::new_v7(),
                thread_id: ThreadId::new("t1").unwrap(),
                attempt: 1,
                enqueued_at: Utc::now(),
            },
            token: LeaseToken::new("lease-1"),
        };
        let json = serde_json::to_string(&leased).unwrap();
        let restored: LeasedRun = serde_json::from_str(&json).unwrap();
        assert_eq!(leased, restored);
    }
}
