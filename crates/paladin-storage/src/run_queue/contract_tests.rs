//! Shared `RunQueuePort` contract suite (D-06).
//!
//! One generic async function per contract clause, each taking `&dyn
//! RunQueuePort` (the concurrency clause instead takes `Arc<dyn
//! RunQueuePort>`, since it must be cloned into spawned tasks) and asserting
//! inside. Both `InMemoryRunQueue` and `RedisRunQueue` invoke these
//! unchanged from their own `#[tokio::test]`s, so "identical suite across
//! backends" is enforced by construction rather than by convention — the
//! same discipline `waypoint::contract_tests` established for `WaypointPort`
//! (D-06's own reversibility note: "the suite is the contract every future
//! backend must pass").
//!
//! This module is plain (not `#[cfg(test)]`) so both unit tests inside each
//! backend crate and future Docker-gated integration tests can call it.
//!
//! ## Real time, never a paused virtual clock
//!
//! Every clause below uses real, short `tokio::time::sleep` waits, never a
//! paused/mocked async runtime clock. The InMemory adapter measures elapsed
//! time with `std::time::Instant`, and the Redis adapter reads the server's
//! own `TIME` command — a paused virtual clock advances neither of those,
//! so a suite built on one would simply hang against either backend
//! (D-06, D-08).

use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::timeout;

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_queue_port::{LeaseToken, QueueError, QueuedRun, RunQueuePort};

/// Build a `QueuedRun` fixture for `thread`, stamped with the current time
/// and `attempt: 1` (the attempt value of a run's first enqueue, per D-07's
/// `QueuedRun` doc).
pub fn sample_queued_run(thread: &ThreadId) -> QueuedRun {
    QueuedRun {
        run_id: RunId::new_v7(),
        thread_id: thread.clone(),
        attempt: 1,
        enqueued_at: Utc::now(),
    }
}

/// FIFO: three enqueues dequeue in order; each `dequeue` returns a distinct
/// `LeaseToken`.
pub async fn fifo_order_and_distinct_lease_tokens(queue: &dyn RunQueuePort) {
    let thread = ThreadId::new("contract-fifo").unwrap();
    let first = sample_queued_run(&thread);
    let second = sample_queued_run(&thread);
    let third = sample_queued_run(&thread);
    queue.enqueue(first.clone()).await.unwrap();
    queue.enqueue(second.clone()).await.unwrap();
    queue.enqueue(third.clone()).await.unwrap();

    let leased_first = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .expect("first enqueued message must be dequeued first");
    let leased_second = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .expect("second enqueued message must be dequeued second");
    let leased_third = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .expect("third enqueued message must be dequeued third");

    assert_eq!(leased_first.queued.run_id, first.run_id);
    assert_eq!(leased_second.queued.run_id, second.run_id);
    assert_eq!(leased_third.queued.run_id, third.run_id);

    let tokens: HashSet<LeaseToken> = [leased_first.token, leased_second.token, leased_third.token]
        .into_iter()
        .collect();
    assert_eq!(
        tokens.len(),
        3,
        "each dequeue must return a distinct LeaseToken"
    );
}

/// Lease invisibility, then expiry redelivery: after one dequeue, an
/// immediate second dequeue sees nothing; once the lease elapses, the same
/// `run_id` is redelivered with `attempt` incremented by one.
pub async fn lease_expiry_redelivers_with_attempt_incremented(queue: &dyn RunQueuePort) {
    let thread = ThreadId::new("contract-lease-expiry").unwrap();
    let queued = sample_queued_run(&thread);
    queue.enqueue(queued.clone()).await.unwrap();

    let leased = queue
        .dequeue(Duration::from_millis(200))
        .await
        .unwrap()
        .expect("the enqueued message must be dequeued");
    assert_eq!(leased.queued.run_id, queued.run_id);
    assert_eq!(leased.queued.attempt, 1);

    // Immediately after, the lease is still held: nothing else is visible.
    assert!(
        queue
            .dequeue(Duration::from_millis(50))
            .await
            .unwrap()
            .is_none(),
        "a live lease must hide the message from a second dequeue"
    );

    // Wait well past the 200ms lease so expiry is unambiguous under CI jitter.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let redelivered = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .expect("the expired lease's message must become visible again");
    assert_eq!(redelivered.queued.run_id, queued.run_id);
    assert_eq!(
        redelivered.queued.attempt, 2,
        "redelivery after lease expiry must increment attempt"
    );
}

/// `extend_lease(token, by)` at some time `t` pushes expiry to `t + by`
/// (measured from the extension call, not the original dequeue): the
/// message stays hidden just before the new expiry and becomes visible
/// just after it.
pub async fn extend_lease_keeps_message_hidden_until_new_expiry(queue: &dyn RunQueuePort) {
    let thread = ThreadId::new("contract-extend-lease").unwrap();
    let queued = sample_queued_run(&thread);
    queue.enqueue(queued.clone()).await.unwrap();

    // The original lease must still be live when `extend_lease` is called
    // below (extending an already-expired lease is the separate
    // `LeaseExpired` clause's concern, not this one) -- generous relative
    // to the 100ms wait before the extension call.
    let leased = queue
        .dequeue(Duration::from_millis(300))
        .await
        .unwrap()
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    queue
        .extend_lease(&leased.token, Duration::from_millis(400))
        .await
        .unwrap();
    // New expiry is ~500ms after the original dequeue (100ms elapsed + 400ms
    // extension).

    tokio::time::sleep(Duration::from_millis(150)).await;
    // ~250ms elapsed since dequeue: comfortably before the ~500ms extended
    // expiry.
    assert!(
        queue
            .dequeue(Duration::from_millis(50))
            .await
            .unwrap()
            .is_none(),
        "extend_lease must keep the message hidden until the new expiry"
    );

    tokio::time::sleep(Duration::from_millis(400)).await;
    // ~650ms elapsed since dequeue: comfortably past the ~500ms extended
    // expiry.
    let redelivered = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .expect("the message must become visible once the extended lease elapses");
    assert_eq!(redelivered.queued.run_id, queued.run_id);
}

/// `ack(token)` removes the message permanently: a later dequeue sees
/// nothing and `depth()` is 0.
pub async fn ack_removes_message_permanently(queue: &dyn RunQueuePort) {
    let thread = ThreadId::new("contract-ack").unwrap();
    let queued = sample_queued_run(&thread);
    queue.enqueue(queued).await.unwrap();
    assert_eq!(queue.depth().await.unwrap(), 1);

    let leased = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    queue.ack(&leased.token).await.unwrap();

    assert_eq!(queue.depth().await.unwrap(), 0);
    assert!(
        queue
            .dequeue(Duration::from_millis(50))
            .await
            .unwrap()
            .is_none(),
        "an acked message must never be redelivered"
    );
}

/// `nack(token, delay)` re-queues the message visible only after `delay`,
/// with `attempt` incremented.
pub async fn nack_requeues_after_delay_with_attempt_incremented(queue: &dyn RunQueuePort) {
    let thread = ThreadId::new("contract-nack").unwrap();
    let queued = sample_queued_run(&thread);
    queue.enqueue(queued.clone()).await.unwrap();

    let leased = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(leased.queued.attempt, 1);
    queue
        .nack(&leased.token, Duration::from_millis(150))
        .await
        .unwrap();

    // Hidden immediately after nack, before the requeue delay elapses.
    assert!(
        queue
            .dequeue(Duration::from_millis(50))
            .await
            .unwrap()
            .is_none(),
        "a nack'd message must stay hidden until its requeue_delay elapses"
    );

    tokio::time::sleep(Duration::from_millis(300)).await;

    let redelivered = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .expect("a nack'd message must become visible after its delay");
    assert_eq!(redelivered.queued.run_id, queued.run_id);
    assert_eq!(
        redelivered.queued.attempt, 2,
        "nack must increment attempt on requeue"
    );
}

/// `ack`/`extend_lease`/`nack` against a token whose lease has already
/// expired return `QueueError::LeaseExpired`, distinct from a token that was
/// never issued, and touch no other in-flight message.
pub async fn expired_token_operations_return_lease_expired_and_touch_nothing(
    queue: &dyn RunQueuePort,
) {
    let thread = ThreadId::new("contract-expired-token-errors").unwrap();
    let survivor = sample_queued_run(&thread);
    let victim = sample_queued_run(&thread);
    queue.enqueue(survivor.clone()).await.unwrap();
    queue.enqueue(victim.clone()).await.unwrap();

    let leased_survivor = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    let leased_victim = queue
        .dequeue(Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(leased_survivor.queued.run_id, survivor.run_id);
    assert_eq!(leased_victim.queued.run_id, victim.run_id);

    tokio::time::sleep(Duration::from_millis(300)).await;
    // Trigger a reclaim pass (backends reclaim lazily on the next
    // operation) without touching the survivor's still-live lease.
    let _ = queue.depth().await.unwrap();

    let ack_err = queue.ack(&leased_victim.token).await.unwrap_err();
    assert!(
        matches!(ack_err, QueueError::LeaseExpired { .. }),
        "expected LeaseExpired for an expired token on ack, got {ack_err:?}"
    );

    let extend_err = queue
        .extend_lease(&leased_victim.token, Duration::from_secs(30))
        .await
        .unwrap_err();
    assert!(
        matches!(extend_err, QueueError::LeaseExpired { .. }),
        "expected LeaseExpired for an expired token on extend_lease, got {extend_err:?}"
    );

    let nack_err = queue
        .nack(&leased_victim.token, Duration::from_millis(50))
        .await
        .unwrap_err();
    assert!(
        matches!(nack_err, QueueError::LeaseExpired { .. }),
        "expected LeaseExpired for an expired token on nack, got {nack_err:?}"
    );

    // None of the three failed calls above may have touched the survivor's
    // still-live lease.
    queue
        .ack(&leased_survivor.token)
        .await
        .expect("the survivor's lease must be untouched by the victim's expired-token errors");
}

/// `ack`/`extend_lease`/`nack` against a token that was never issued by this
/// queue return `QueueError::UnknownLease`.
pub async fn unknown_token_operations_return_unknown_lease(queue: &dyn RunQueuePort) {
    let never_issued = LeaseToken::new("contract-suite-never-issued-token");

    let ack_err = queue.ack(&never_issued).await.unwrap_err();
    assert!(matches!(ack_err, QueueError::UnknownLease { .. }));

    let extend_err = queue
        .extend_lease(&never_issued, Duration::from_secs(30))
        .await
        .unwrap_err();
    assert!(matches!(extend_err, QueueError::UnknownLease { .. }));

    let nack_err = queue
        .nack(&never_issued, Duration::from_millis(50))
        .await
        .unwrap_err();
    assert!(matches!(nack_err, QueueError::UnknownLease { .. }));
}

/// `depth()` counts ready-plus-leased messages, not just ready ones, and
/// returns to the expected count as messages are acked.
pub async fn depth_counts_ready_plus_leased(queue: &dyn RunQueuePort) {
    let thread = ThreadId::new("contract-depth").unwrap();
    assert_eq!(queue.depth().await.unwrap(), 0);

    queue.enqueue(sample_queued_run(&thread)).await.unwrap();
    queue.enqueue(sample_queued_run(&thread)).await.unwrap();
    assert_eq!(queue.depth().await.unwrap(), 2);

    let leased = queue
        .dequeue(Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        queue.depth().await.unwrap(),
        2,
        "depth must count the leased message too, not only the still-ready one"
    );

    queue.ack(&leased.token).await.unwrap();
    assert_eq!(queue.depth().await.unwrap(), 1);
}

/// Concurrency (D-52): eight workers each looping `dequeue(5s)` -> `ack`
/// over 200 enqueued messages deliver every `run_id` exactly once, and the
/// queue is empty at the end. Guarded by an overall timeout so a broken
/// implementation that deadlocks fails fast rather than hanging the suite.
pub async fn concurrent_workers_each_message_exactly_once(queue: Arc<dyn RunQueuePort>) {
    const MESSAGE_COUNT: usize = 200;
    const WORKER_COUNT: usize = 8;

    let thread = ThreadId::new("contract-concurrency").unwrap();
    for _ in 0..MESSAGE_COUNT {
        queue.enqueue(sample_queued_run(&thread)).await.unwrap();
    }

    let delivered: Arc<Mutex<HashSet<RunId>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut workers: JoinSet<()> = JoinSet::new();
    for _ in 0..WORKER_COUNT {
        let queue = Arc::clone(&queue);
        let delivered = Arc::clone(&delivered);
        workers.spawn(async move {
            loop {
                match queue.dequeue(Duration::from_secs(5)).await.unwrap() {
                    Some(leased) => {
                        {
                            let mut delivered = delivered.lock().await;
                            let first_delivery = delivered.insert(leased.queued.run_id.clone());
                            assert!(
                                first_delivery,
                                "run_id {:?} delivered more than once under concurrent workers",
                                leased.queued.run_id
                            );
                        }
                        queue.ack(&leased.token).await.unwrap();
                    }
                    None => {
                        // Nothing currently visible: if the queue is fully
                        // drained (nothing ready or leased anywhere), this
                        // worker is done; otherwise a peer still holds a
                        // lease, so wait briefly and check again.
                        if queue.depth().await.unwrap() == 0 {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
            }
        });
    }

    timeout(Duration::from_secs(30), async {
        while let Some(outcome) = workers.join_next().await {
            outcome.expect("a worker task panicked");
        }
    })
    .await
    .expect("concurrency stress test exceeded its 30s timeout guard");

    let delivered = delivered.lock().await;
    assert_eq!(
        delivered.len(),
        MESSAGE_COUNT,
        "every run_id must be delivered exactly once across all workers"
    );
    assert_eq!(
        queue.depth().await.unwrap(),
        0,
        "the queue must be empty once every message has been acked"
    );
}

/// Runs every `&dyn RunQueuePort` contract function above, each against its
/// OWN freshly constructed, still-empty queue built by calling
/// `fresh_queue()` once per clause.
///
/// **Fresh-queue-per-clause, not one shared queue.** A single queue instance
/// run through all eight clauses back-to-back leaks state forward: the fifo,
/// lease-expiry and extend-lease clauses each leave behind leased (unacked)
/// messages that are not that clause's cleanup responsibility, and a later
/// clause -- e.g. `ack_removes_message_permanently`'s `depth() == 1`
/// assertion -- then observes them as if they were its own fixture. `run_all`
/// takes a factory precisely so no clause can ever see another clause's
/// leftover leases, tokens or depth (the defect CI run 34238527001 exposed:
/// `ack_removes_message_permanently` saw `depth() == 6`, the five leases the
/// fifo, lease-expiry and extend-lease clauses left behind, instead of `1`).
///
/// The concurrency clause is not included here (it needs an
/// `Arc<dyn RunQueuePort>` and is heavier); call
/// [`concurrent_workers_each_message_exactly_once`] separately. Prefer
/// invoking each function from its own named `#[tokio::test]` for per-clause
/// failure diagnostics (mirroring `waypoint::contract_tests`'s own
/// convention); this aggregator is a convenience, not a replacement.
pub async fn run_all<F, Fut, Q>(fresh_queue: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = Q>,
    Q: RunQueuePort,
{
    fifo_order_and_distinct_lease_tokens(&fresh_queue().await).await;
    lease_expiry_redelivers_with_attempt_incremented(&fresh_queue().await).await;
    extend_lease_keeps_message_hidden_until_new_expiry(&fresh_queue().await).await;
    ack_removes_message_permanently(&fresh_queue().await).await;
    nack_requeues_after_delay_with_attempt_incremented(&fresh_queue().await).await;
    expired_token_operations_return_lease_expired_and_touch_nothing(&fresh_queue().await).await;
    unknown_token_operations_return_unknown_lease(&fresh_queue().await).await;
    depth_counts_ready_plus_leased(&fresh_queue().await).await;
}
