//! Cross-instance cancellation (D-14, D-15, D-16, PLAT-FR-04): the
//! debounced [`CancellationProbe`] adapter reading the durable cancel flag
//! through [`RunRepositoryPort`], and the per-instance local-token registry
//! [`RunSubmissionService::cancel`] consults to decide `was_local`.
//!
//! ## Two signals, two purposes
//!
//! [`DbCancellationProbe`] is the cross-instance mechanism: any instance's
//! repository handle can durably request cancellation, and every instance
//! running a worker consults the SAME flag at its own superstep boundaries
//! (D-14). [`LocalRunTokens`] is a same-instance fast path: when the
//! cancelling caller happens to be talking to the very instance dispatching
//! the run, a [`tokio_util::sync::CancellationToken`] fires the halt
//! immediately, without waiting out [`DbCancellationProbe`]'s debounce
//! window. Neither is required for correctness on its own to be sufficient
//! -- the durable flag alone is enough for D-16's "never lost to a crash"
//! guarantee; the local token is purely a latency optimization.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::cancellation_probe::CancellationProbe;
use paladin_ports::output::run_repository_port::RunRepositoryPort;

/// A [`CancellationProbe`] adapter reading the durable cancel flag through
/// [`RunRepositoryPort::is_cancel_requested`] (D-14), debounced per thread
/// for `min_interval` (D-15, policy-in-the-adapter): two boundary checks
/// inside `min_interval` produce exactly one repository read.
///
/// Infallible by construction (T-27-07-03): a repository error is caught,
/// logged at `warn`, and answered `false` -- a probe failure must never
/// fail a run.
pub struct DbCancellationProbe {
    repo: Arc<dyn RunRepositoryPort>,
    min_interval: Duration,
    cache: Mutex<HashMap<ThreadId, (Instant, bool)>>,
}

impl DbCancellationProbe {
    /// Construct a probe reading `repo`, caching each thread's answer for
    /// `min_interval` before re-checking the repository.
    pub fn new(repo: Arc<dyn RunRepositoryPort>, min_interval: Duration) -> Self {
        Self {
            repo,
            min_interval,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl CancellationProbe for DbCancellationProbe {
    async fn is_cancelled(&self, thread: &ThreadId) -> bool {
        let now = Instant::now();
        {
            let cache = self.cache.lock().await;
            if let Some((checked_at, answer)) = cache.get(thread)
                && now.duration_since(*checked_at) < self.min_interval
            {
                return *answer;
            }
        }

        let answer = match self.repo.is_cancel_requested(thread).await {
            Ok(answer) => answer,
            Err(err) => {
                log::warn!(
                    "cancellation probe: is_cancel_requested failed for thread {thread}: \
                     {err} -- treated as not-cancelled (a probe failure must never fail a run, \
                     D-14)"
                );
                false
            }
        };

        self.cache
            .lock()
            .await
            .insert(thread.clone(), (now, answer));
        answer
    }
}

/// The per-run [`CancellationProbe`] the worker attaches to every
/// factory-built engine (Phase 42 D-14, G1): it reports a caller cancel as a
/// caller cancel, on this instance as well as on any other.
///
/// ## Why it exists (G1)
///
/// A caller cancel arriving at the instance that dispatches the run fires
/// only that run's in-process child token (via [`LocalRunTokens`]). The child
/// token is a child of the coordinator's shutdown token, so the engine sees
/// the same bare "token cancelled" for a caller cancel and for a worker drain
/// and cannot tell them apart. This probe answers `true` when:
///
/// - the debounced durable-flag probe (`db`, if any) answers `true` -- a
///   cancel written by any instance; or
/// - the run's own `local` token is cancelled while the `shutdown` token is
///   NOT -- a same-instance caller cancel, never a drain.
///
/// While `shutdown` is cancelled the local token's state says nothing about
/// the caller (it is a child of `shutdown`), so only the durable flag counts.
///
/// Infallible like the trait: it never fails a run.
pub struct PerRunCancelProbe {
    db: Option<Arc<dyn CancellationProbe>>,
    local: CancellationToken,
    shutdown: CancellationToken,
}

impl PerRunCancelProbe {
    /// Construct the probe from the optional durable-flag probe, the run's
    /// own child token, and the worker coordinator's shutdown token.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::application::services::run::cancel::PerRunCancelProbe;
    /// use paladin_core::platform::container::waypoint::ThreadId;
    /// use paladin_ports::output::cancellation_probe::CancellationProbe;
    /// use tokio_util::sync::CancellationToken;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let shutdown = CancellationToken::new();
    /// let local = shutdown.child_token();
    /// let probe = PerRunCancelProbe::new(None, local.clone(), shutdown);
    ///
    /// // A same-instance caller cancel fires only the run's own token.
    /// local.cancel();
    /// assert!(probe.is_cancelled(&ThreadId::new("t")?).await);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        db: Option<Arc<dyn CancellationProbe>>,
        local: CancellationToken,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            db,
            local,
            shutdown,
        }
    }
}

#[async_trait]
impl CancellationProbe for PerRunCancelProbe {
    async fn is_cancelled(&self, thread: &ThreadId) -> bool {
        // The local check first: an in-memory flag read, and it lets a
        // same-instance cancel skip the repository entirely.
        if self.local.is_cancelled() && !self.shutdown.is_cancelled() {
            return true;
        }
        match &self.db {
            Some(db) => db.is_cancelled(thread).await,
            None => false,
        }
    }
}

/// The registry a [`RunWorkerPool`](super::worker::RunWorkerPool) shares
/// with [`RunSubmissionService`](super::submission::RunSubmissionService)
/// (via [`RunSubmissionService::with_local_tokens`]) so `cancel` can decide
/// `CancelOutcome::was_local` (D-16): whether THIS process instance is
/// dispatching the target run right now.
///
/// `Clone` is cheap and shares the underlying map (an `Arc<RwLock<..>>`
/// inside) -- every clone observes the same registrations.
#[derive(Debug, Clone, Default)]
pub struct LocalRunTokens {
    tokens: Arc<RwLock<HashMap<RunId, CancellationToken>>>,
}

impl LocalRunTokens {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `token` as `run_id`'s in-process cancellation signal, for
    /// the duration of this instance's dispatch of it. Overwrites any prior
    /// registration for the same id (a redelivery re-registers a fresh
    /// token).
    pub async fn register(&self, run_id: RunId, token: CancellationToken) {
        self.tokens.write().await.insert(run_id, token);
    }

    /// Remove `run_id`'s registration -- call once dispatch finishes, one
    /// way or another, so a stale token is never mistaken for a live one.
    pub async fn remove(&self, run_id: &RunId) {
        self.tokens.write().await.remove(run_id);
    }

    /// If this instance holds a token for `run_id`, cancel it and return
    /// `true`; `false` if no local registration exists (the run is not
    /// currently dispatched by this instance).
    pub async fn cancel_if_local(&self, run_id: &RunId) -> bool {
        let tokens = self.tokens.read().await;
        match tokens.get(run_id) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_ports::output::run_repository_port::{
        RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A minimal repository double that only implements
    /// `is_cancel_requested`, counting calls and always answering the
    /// configured value -- every other method is unreachable in these
    /// tests.
    struct CountingRepo {
        calls: AtomicUsize,
        answer: bool,
        fail: bool,
    }

    #[async_trait]
    impl RunRepositoryPort for CountingRepo {
        async fn insert(
            &self,
            _run: &paladin_core::platform::container::run::Run,
        ) -> Result<(), RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }

        async fn get(
            &self,
            _run_id: &RunId,
        ) -> Result<Option<paladin_core::platform::container::run::Run>, RunRepositoryError>
        {
            unreachable!("not exercised by these tests")
        }

        async fn update_status(
            &self,
            _run_id: &RunId,
            _from: paladin_core::platform::container::run::RunStatus,
            _to: paladin_core::platform::container::run::RunStatus,
            _at: chrono::DateTime<chrono::Utc>,
        ) -> Result<(), RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }

        async fn record_outcome(
            &self,
            _run_id: &RunId,
            _outcome: RunOutcomeRecord,
        ) -> Result<(), RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }

        async fn list(&self, _query: RunQuery) -> Result<RunPage, RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }

        async fn active_run_for_thread(
            &self,
            _thread_id: &ThreadId,
        ) -> Result<Option<paladin_core::platform::container::run::Run>, RunRepositoryError>
        {
            unreachable!("not exercised by these tests")
        }

        async fn request_cancel(
            &self,
            _run_id: &RunId,
        ) -> Result<paladin_core::platform::container::run::RunStatus, RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }

        async fn is_cancel_requested(
            &self,
            _thread_id: &ThreadId,
        ) -> Result<bool, RunRepositoryError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(RunRepositoryError::Backend {
                    source: "boom".into(),
                })
            } else {
                Ok(self.answer)
            }
        }

        async fn bump_attempt(&self, _run_id: &RunId) -> Result<u32, RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }

        async fn record_resume(
            &self,
            _run_id: &RunId,
            _responses: Vec<paladin_core::platform::container::parley::ParleyResponse>,
        ) -> Result<u32, RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }

        async fn clear_pending_responses(&self, _run_id: &RunId) -> Result<(), RunRepositoryError> {
            unreachable!("not exercised by these tests")
        }
    }

    fn thread() -> ThreadId {
        ThreadId::new("t1").unwrap()
    }

    #[tokio::test]
    async fn two_calls_within_the_interval_hit_the_repository_once() {
        let repo = Arc::new(CountingRepo {
            calls: AtomicUsize::new(0),
            answer: false,
            fail: false,
        });
        let probe = DbCancellationProbe::new(repo.clone(), Duration::from_millis(500));

        assert!(!probe.is_cancelled(&thread()).await);
        assert!(!probe.is_cancelled(&thread()).await);
        assert_eq!(
            repo.calls.load(Ordering::SeqCst),
            1,
            "two calls inside the debounce interval must hit the repository once"
        );
    }

    #[tokio::test]
    async fn a_fresh_read_happens_after_the_interval_elapses() {
        let repo = Arc::new(CountingRepo {
            calls: AtomicUsize::new(0),
            answer: false,
            fail: false,
        });
        let probe = DbCancellationProbe::new(repo.clone(), Duration::from_millis(20));

        assert!(!probe.is_cancelled(&thread()).await);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(!probe.is_cancelled(&thread()).await);
        assert_eq!(
            repo.calls.load(Ordering::SeqCst),
            2,
            "a call after the interval elapses must re-read the repository"
        );
    }

    #[tokio::test]
    async fn a_repository_error_yields_false_and_never_panics() {
        let repo = Arc::new(CountingRepo {
            calls: AtomicUsize::new(0),
            answer: true,
            fail: true,
        });
        let probe = DbCancellationProbe::new(repo, Duration::from_millis(500));

        // Must not panic, and must answer `false` on a backend failure.
        assert!(!probe.is_cancelled(&thread()).await);
    }

    #[tokio::test]
    async fn local_run_tokens_register_cancel_and_remove() {
        let tokens = LocalRunTokens::new();
        let run_id = RunId::new_v7();

        assert!(
            !tokens.cancel_if_local(&run_id).await,
            "no registration yet -- must answer false"
        );

        let token = CancellationToken::new();
        tokens.register(run_id.clone(), token.clone()).await;
        assert!(tokens.cancel_if_local(&run_id).await);
        assert!(token.is_cancelled());

        tokens.remove(&run_id).await;
        assert!(
            !tokens.cancel_if_local(&run_id).await,
            "removed registration must answer false again"
        );
    }

    #[tokio::test]
    async fn local_run_tokens_clones_share_the_same_map() {
        let tokens = LocalRunTokens::new();
        let clone = tokens.clone();
        let run_id = RunId::new_v7();
        let token = CancellationToken::new();

        tokens.register(run_id.clone(), token.clone()).await;
        assert!(
            clone.cancel_if_local(&run_id).await,
            "a clone must observe registrations made through the original"
        );
        assert!(token.is_cancelled());
    }

    /// A probe double answering a fixed value.
    struct FixedProbe(bool);

    #[async_trait]
    impl CancellationProbe for FixedProbe {
        async fn is_cancelled(&self, _thread: &ThreadId) -> bool {
            self.0
        }
    }

    #[tokio::test]
    async fn per_run_probe_reports_a_local_cancel_while_not_shutting_down() {
        let shutdown = CancellationToken::new();
        let local = shutdown.child_token();
        let probe = PerRunCancelProbe::new(None, local.clone(), shutdown.clone());

        assert!(!probe.is_cancelled(&thread()).await, "nothing cancelled");
        local.cancel();
        assert!(
            probe.is_cancelled(&thread()).await,
            "local cancelled, shutdown not -> a caller cancel"
        );
    }

    #[tokio::test]
    async fn per_run_probe_treats_a_drain_as_not_a_caller_cancel() {
        let shutdown = CancellationToken::new();
        let local = shutdown.child_token();
        let probe = PerRunCancelProbe::new(None, local, shutdown.clone());

        // Cancelling the parent cancels the child too: a drain.
        shutdown.cancel();
        assert!(
            !probe.is_cancelled(&thread()).await,
            "both tokens cancelled and no durable flag -> a drain, not a cancel"
        );
    }

    #[tokio::test]
    async fn per_run_probe_reports_the_durable_flag_even_while_shutting_down() {
        let shutdown = CancellationToken::new();
        let local = shutdown.child_token();
        let probe = PerRunCancelProbe::new(
            Some(Arc::new(FixedProbe(true))),
            local.clone(),
            shutdown.clone(),
        );
        assert!(probe.is_cancelled(&thread()).await, "db probe true");
        shutdown.cancel();
        assert!(
            probe.is_cancelled(&thread()).await,
            "a durable cancel still counts during a drain"
        );

        let quiet = PerRunCancelProbe::new(Some(Arc::new(FixedProbe(false))), local, shutdown);
        assert!(!quiet.is_cancelled(&thread()).await);
    }
}
