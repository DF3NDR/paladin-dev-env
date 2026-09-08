/*
In-Memory Run Queue

A lease-aware in-memory `RunQueuePort` implementation (D-08's Tier 1 twin):
a `VecDeque<QueueEntry>` of visible-or-scheduled messages plus a
`HashMap<LeaseToken, LeaseEntry>` of in-flight leases with real expiry
instants. `dequeue` hides a message for the lease duration; an expired
lease becomes visible again (reclaimed lazily on the next queue operation),
with `attempt` incremented on the redelivered message, rather than being
stubbed out -- plan 27-03's queue contract suite runs against this adapter
unchanged.

A bounded ring of recently-reclaimed tokens (`recently_expired`) lets
`ack`/`extend_lease`/`nack` distinguish `QueueError::LeaseExpired` (the
token was issued and has since expired) from `QueueError::UnknownLease`
(the token was never issued by this queue at all) -- the two errors D-07
requires callers be able to tell apart.
*/

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::Mutex;

use paladin_ports::output::run_queue_port::{
    LeaseToken, LeasedRun, QueueError, QueuedRun, RunQueuePort,
};

/// How many recently-expired tokens to remember for the
/// `LeaseExpired`-vs-`UnknownLease` distinction. Bounded so a long-running
/// queue's memory does not grow unboundedly; far larger than any single
/// test or realistic in-flight lease count needs.
const RECENTLY_EXPIRED_CAPACITY: usize = 4096;

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
    /// A bounded FIFO ring of tokens whose lease recently expired (reclaimed
    /// by [`Inner::reclaim_expired_leases`]), oldest first. Membership here
    /// (rather than in `leases`) is what makes a stale token's error
    /// `LeaseExpired` instead of `UnknownLease`.
    recently_expired: VecDeque<LeaseToken>,
    next_token: u64,
}

impl Inner {
    /// Move every lease whose `expires_at` has passed back onto the visible
    /// queue with `attempt` incremented -- the "an expired lease becomes
    /// visible again, redelivered with attempt+1" half of the module doc's
    /// contract -- and record the token in `recently_expired`. Called at the
    /// start of every operation so a caller never observes a stale lease as
    /// still held.
    fn reclaim_expired_leases(&mut self, now: Instant) {
        let expired: Vec<LeaseToken> = self
            .leases
            .iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(token, _)| token.clone())
            .collect();
        for token in expired {
            if let Some(mut entry) = self.leases.remove(&token) {
                entry.queued.attempt += 1;
                self.entries.push_back(QueueEntry {
                    queued: entry.queued,
                    visible_at: now,
                });
                self.remember_expired(token);
            }
        }
    }

    /// Record `token` in the bounded `recently_expired` ring, evicting the
    /// oldest entry first if at capacity.
    fn remember_expired(&mut self, token: LeaseToken) {
        if self.recently_expired.len() >= RECENTLY_EXPIRED_CAPACITY {
            self.recently_expired.pop_front();
        }
        self.recently_expired.push_back(token);
    }

    /// Classify a token this queue does not currently hold a live lease
    /// for: `LeaseExpired` if it was issued and has since expired,
    /// `UnknownLease` if this queue never issued it at all.
    fn error_for_missing_token(&self, token: &LeaseToken) -> QueueError {
        if self.recently_expired.contains(token) {
            QueueError::LeaseExpired {
                token: token.clone(),
            }
        } else {
            QueueError::UnknownLease {
                token: token.clone(),
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
            None => Err(inner.error_for_missing_token(token)),
        }
    }

    async fn ack(&self, token: &LeaseToken) -> Result<(), QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.reclaim_expired_leases(now);
        match inner.leases.remove(token) {
            Some(_) => Ok(()),
            None => Err(inner.error_for_missing_token(token)),
        }
    }

    async fn nack(&self, token: &LeaseToken, requeue_delay: Duration) -> Result<(), QueueError> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.reclaim_expired_leases(now);
        match inner.leases.remove(token) {
            Some(mut entry) => {
                entry.queued.attempt += 1;
                inner.entries.push_back(QueueEntry {
                    queued: entry.queued,
                    visible_at: now + requeue_delay,
                });
                Ok(())
            }
            None => Err(inner.error_for_missing_token(token)),
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
    use crate::run_queue::contract_tests;
    use std::sync::Arc as StdArc;

    // ── D-06 shared contract suite, one #[tokio::test] per clause ────────
    //
    // Each test constructs its own fresh, empty `InMemoryRunQueue` (the
    // contract suite's own precondition) and delegates entirely to
    // `contract_tests`, so a failure names the violated contract clause
    // directly rather than a line number in this file.

    #[tokio::test]
    async fn fifo_order_and_distinct_lease_tokens() {
        contract_tests::fifo_order_and_distinct_lease_tokens(&InMemoryRunQueue::new()).await;
    }

    #[tokio::test]
    async fn lease_expiry_redelivers_with_attempt_incremented() {
        contract_tests::lease_expiry_redelivers_with_attempt_incremented(&InMemoryRunQueue::new())
            .await;
    }

    #[tokio::test]
    async fn extend_lease_keeps_message_hidden_until_new_expiry() {
        contract_tests::extend_lease_keeps_message_hidden_until_new_expiry(
            &InMemoryRunQueue::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn ack_removes_message_permanently() {
        contract_tests::ack_removes_message_permanently(&InMemoryRunQueue::new()).await;
    }

    #[tokio::test]
    async fn nack_requeues_after_delay_with_attempt_incremented() {
        contract_tests::nack_requeues_after_delay_with_attempt_incremented(
            &InMemoryRunQueue::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn expired_token_operations_return_lease_expired_and_touch_nothing() {
        contract_tests::expired_token_operations_return_lease_expired_and_touch_nothing(
            &InMemoryRunQueue::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn unknown_token_operations_return_unknown_lease() {
        contract_tests::unknown_token_operations_return_unknown_lease(&InMemoryRunQueue::new())
            .await;
    }

    #[tokio::test]
    async fn depth_counts_ready_plus_leased() {
        contract_tests::depth_counts_ready_plus_leased(&InMemoryRunQueue::new()).await;
    }

    // RED: `run_all` runs every clause back-to-back on ONE
    // `InMemoryRunQueue`, so leases left behind by the fifo, lease-expiry
    // and extend-lease clauses are still visible when
    // `ack_removes_message_permanently` asserts `depth() == 1` -- the same
    // suite-isolation defect CI's `redis_run_queue_full_contract_suite_via_run_all`
    // hit at `contract_tests.rs:188` (`left: 6 right: 1`), reproduced here
    // without Redis. This proves the defect is not Redis-specific before
    // `run_all` is changed to take a fresh-queue factory.
    #[tokio::test]
    async fn in_memory_run_queue_full_contract_suite_via_run_all() {
        contract_tests::run_all(&InMemoryRunQueue::new()).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_workers_each_message_exactly_once() {
        let queue: StdArc<dyn RunQueuePort> = StdArc::new(InMemoryRunQueue::new());
        contract_tests::concurrent_workers_each_message_exactly_once(queue).await;
    }

    // ── Adapter-specific regression coverage beyond the shared contract ──

    #[tokio::test]
    async fn recently_expired_ring_evicts_the_oldest_token_at_capacity() {
        // A capacity-scoped regression for `remember_expired`'s eviction
        // policy, exercised directly against `Inner` (bypassing the queue's
        // own dequeue/reclaim timing) so the test stays fast regardless of
        // how large `RECENTLY_EXPIRED_CAPACITY` is: fill the ring to
        // capacity with synthetic tokens, then push one more and confirm
        // the oldest was evicted while the newest is remembered. This is
        // deliberately adapter-internal (not part of the cross-backend
        // contract, since Redis has no equivalent in-memory ring) so it
        // lives here rather than in `contract_tests`.
        let mut inner = Inner::default();
        for i in 0..RECENTLY_EXPIRED_CAPACITY {
            inner.remember_expired(LeaseToken::new(format!("token-{i}")));
        }
        let oldest = LeaseToken::new("token-0");
        assert!(inner.recently_expired.contains(&oldest));

        let overflow_token = LeaseToken::new("token-overflow");
        inner.remember_expired(overflow_token.clone());

        assert_eq!(inner.recently_expired.len(), RECENTLY_EXPIRED_CAPACITY);
        assert!(
            !inner.recently_expired.contains(&oldest),
            "the oldest token must be evicted once the ring is at capacity"
        );
        assert!(
            inner.recently_expired.contains(&overflow_token),
            "the newest token must still be remembered"
        );
    }
}
