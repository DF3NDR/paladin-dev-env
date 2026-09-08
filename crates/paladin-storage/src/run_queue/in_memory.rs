/*
In-Memory Run Queue

A lease-aware in-memory `RunQueuePort` implementation (D-08's Tier 1 twin):
a `VecDeque<QueueEntry>` of visible-or-scheduled messages plus a
`HashMap<LeaseToken, LeaseEntry>` of in-flight leases with real expiry
instants. `dequeue` hides a message for the lease duration; an expired
lease becomes visible again (reclaimed lazily on the next queue operation)
rather than being stubbed out -- plan 27-03's queue contract suite runs
against this adapter unchanged.
*/

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::Mutex;

use paladin_ports::output::run_queue_port::{
    LeaseToken, LeasedRun, QueueError, QueuedRun, RunQueuePort,
};

struct QueueEntry {
    queued: QueuedRun,
    visible_at: Instant,
}

struct LeaseEntry {
    queued: QueuedRun,
    expires_at: Instant,
}

#[derive(Default)]
struct Inner {
    entries: VecDeque<QueueEntry>,
    leases: HashMap<LeaseToken, LeaseEntry>,
    next_token: u64,
}

impl Inner {
    /// Move every lease whose `expires_at` has passed back onto the visible
    /// queue -- the "an expired lease becomes visible again" half of the
    /// module doc's contract. Called at the start of every operation so a
    /// caller never observes a stale lease as still held.
    fn reclaim_expired_leases(&mut self, now: Instant) {
        let expired: Vec<LeaseToken> = self
            .leases
            .iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(token, _)| token.clone())
            .collect();
        for token in expired {
            if let Some(entry) = self.leases.remove(&token) {
                self.entries.push_back(QueueEntry {
                    queued: entry.queued,
                    visible_at: now,
                });
            }
        }
    }
}

/// In-memory `RunQueuePort` implementation.
///
/// Cloning is cheap and shares the same underlying state (the inner `Arc`
/// is cloned).
#[derive(Clone, Default)]
pub struct InMemoryRunQueue {
    inner: Arc<Mutex<Inner>>,
}

impl InMemoryRunQueue {
    /// Construct a new, empty queue.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RunQueuePort for InMemoryRunQueue {
    async fn enqueue(&self, run: QueuedRun) -> Result<(), QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.entries.push_back(QueueEntry {
            queued: run,
            visible_at: now,
        });
        Ok(())
    }

    async fn dequeue(&self, lease: Duration) -> Result<Option<LeasedRun>, QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.reclaim_expired_leases(now);

        let Some(pos) = inner.entries.iter().position(|e| e.visible_at <= now) else {
            return Ok(None);
        };
        // `pos` was just located above, so removal always succeeds.
        let Some(entry) = inner.entries.remove(pos) else {
            return Ok(None);
        };

        inner.next_token += 1;
        let token = LeaseToken::new(format!("lease-{}", inner.next_token));
        inner.leases.insert(
            token.clone(),
            LeaseEntry {
                queued: entry.queued.clone(),
                expires_at: now + lease,
            },
        );
        Ok(Some(LeasedRun {
            queued: entry.queued,
            token,
        }))
    }

    async fn extend_lease(&self, token: &LeaseToken, lease: Duration) -> Result<(), QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.reclaim_expired_leases(now);
        match inner.leases.get_mut(token) {
            Some(entry) => {
                entry.expires_at = now + lease;
                Ok(())
            }
            None => Err(QueueError::UnknownLease {
                token: token.clone(),
            }),
        }
    }

    async fn ack(&self, token: &LeaseToken) -> Result<(), QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.reclaim_expired_leases(now);
        match inner.leases.remove(token) {
            Some(_) => Ok(()),
            None => Err(QueueError::UnknownLease {
                token: token.clone(),
            }),
        }
    }

    async fn nack(&self, token: &LeaseToken, requeue_delay: Duration) -> Result<(), QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.reclaim_expired_leases(now);
        match inner.leases.remove(token) {
            Some(entry) => {
                inner.entries.push_back(QueueEntry {
                    queued: entry.queued,
                    visible_at: now + requeue_delay,
                });
                Ok(())
            }
            None => Err(QueueError::UnknownLease {
                token: token.clone(),
            }),
        }
    }

    async fn depth(&self) -> Result<u64, QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.reclaim_expired_leases(now);
        Ok((inner.entries.len() + inner.leases.len()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::waypoint::ThreadId;

    fn sample_queued() -> QueuedRun {
        QueuedRun {
            run_id: RunId::new_v7(),
            thread_id: ThreadId::new("t1").unwrap(),
            attempt: 1,
            enqueued_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn enqueue_then_dequeue_returns_the_message() {
        let queue = InMemoryRunQueue::new();
        let queued = sample_queued();
        queue.enqueue(queued.clone()).await.unwrap();
        assert_eq!(queue.depth().await.unwrap(), 1);

        let leased = queue
            .dequeue(Duration::from_secs(30))
            .await
            .unwrap()
            .expect("a message was enqueued");
        assert_eq!(leased.queued.run_id, queued.run_id);
    }

    #[tokio::test]
    async fn dequeue_on_empty_queue_returns_none() {
        let queue = InMemoryRunQueue::new();
        assert!(
            queue
                .dequeue(Duration::from_secs(30))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn dequeue_hides_message_for_the_lease_duration() {
        let queue = InMemoryRunQueue::new();
        queue.enqueue(sample_queued()).await.unwrap();
        let _leased = queue.dequeue(Duration::from_secs(30)).await.unwrap();
        // Depth counts leased-but-unacked messages too.
        assert_eq!(queue.depth().await.unwrap(), 1);
        // No second message is visible while the lease is held.
        assert!(
            queue
                .dequeue(Duration::from_secs(30))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn ack_removes_the_message_permanently() {
        let queue = InMemoryRunQueue::new();
        queue.enqueue(sample_queued()).await.unwrap();
        let leased = queue
            .dequeue(Duration::from_secs(30))
            .await
            .unwrap()
            .unwrap();
        queue.ack(&leased.token).await.unwrap();
        assert_eq!(queue.depth().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn ack_on_unknown_token_is_an_error() {
        let queue = InMemoryRunQueue::new();
        let err = queue
            .ack(&LeaseToken::new("does-not-exist"))
            .await
            .unwrap_err();
        assert!(matches!(err, QueueError::UnknownLease { .. }));
    }

    #[tokio::test]
    async fn expired_lease_becomes_visible_again() {
        let queue = InMemoryRunQueue::new();
        queue.enqueue(sample_queued()).await.unwrap();
        let leased = queue
            .dequeue(Duration::from_millis(1))
            .await
            .unwrap()
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        let redelivered = queue
            .dequeue(Duration::from_secs(30))
            .await
            .unwrap()
            .expect("the expired lease's message becomes visible again");
        assert_eq!(redelivered.queued.run_id, leased.queued.run_id);
    }

    #[tokio::test]
    async fn nack_requeues_after_the_delay() {
        let queue = InMemoryRunQueue::new();
        queue.enqueue(sample_queued()).await.unwrap();
        let leased = queue
            .dequeue(Duration::from_secs(30))
            .await
            .unwrap()
            .unwrap();
        queue
            .nack(&leased.token, Duration::from_millis(1))
            .await
            .unwrap();

        // Immediately after nack, the delay has not elapsed -- allow a
        // brief moment then confirm it becomes visible.
        tokio::time::sleep(Duration::from_millis(20)).await;
        let redelivered = queue
            .dequeue(Duration::from_secs(30))
            .await
            .unwrap()
            .expect("nack'd message becomes visible after its delay");
        assert_eq!(redelivered.queued.run_id, leased.queued.run_id);
    }

    #[tokio::test]
    async fn extend_lease_on_unknown_token_is_an_error() {
        let queue = InMemoryRunQueue::new();
        let err = queue
            .extend_lease(&LeaseToken::new("does-not-exist"), Duration::from_secs(30))
            .await
            .unwrap_err();
        assert!(matches!(err, QueueError::UnknownLease { .. }));
    }
}
