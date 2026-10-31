//! In-memory async job store for fire-and-poll agent execution (Milestone 12, Epic 3).
//!
//! `POST /agents/{id}/jobs` enqueues a job (a spawned task running the agent) and returns
//! a job id immediately; `GET /agents/{id}/jobs/{job_id}` polls its status/result. A poll
//! is scoped to the `(agent, tenant)` that enqueued the job (ADR-0063, TENANT-07): the
//! owner is kept in the store's private state ([`JobStore::create_owned`]) and checked by
//! [`JobStore::get_visible`], so another tenant's poll is indistinguishable from an
//! unknown job. Jobs are **ephemeral** (lost on restart) and bounded — the store keeps at most `capacity`
//! records, evicting the oldest (and logging the eviction). Durable / distributed jobs
//! are the [queue/worker topology](../../../docs/src/deployment-topologies/queue-worker.md),
//! out of scope here.

use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;

use paladin_core::platform::container::principal::{RunReadScope, TenantId};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::agent_auth::Principal;

/// Lifecycle status of an async job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// The job is executing.
    Running,
    /// The job finished successfully (see `result`).
    Completed,
    /// The job failed (see `error`).
    Failed,
    /// The job exceeded its timeout and was cancelled.
    TimedOut,
}

/// A stored job: its status plus, when finished, a result or error.
///
/// `result` is the serialized success payload (an `ExecuteResponse` as JSON), kept as a
/// [`Value`] so the store stays decoupled from the controller's wire types.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobRecord {
    /// Current status.
    pub status: JobStatus,
    /// Success payload (an `ExecuteResponse` as JSON), present once `status == Completed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object, nullable)]
    pub result: Option<Value>,
    /// Error message, present once `status == Failed`/`TimedOut`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Inner {
    jobs: HashMap<String, JobRecord>,
    order: VecDeque<String>,
    /// Owner `(agent id, tenant)` of each job enqueued through [`JobStore::create_owned`].
    /// Private state (not a `JobRecord` field, so the wire shape and the public record are
    /// unchanged); evicted together with the job. A job created through the legacy
    /// [`JobStore::create`] has no entry here.
    owners: HashMap<String, (String, TenantId)>,
}

/// A thread-safe, bounded, in-memory store of async jobs keyed by job id.
pub struct JobStore {
    inner: RwLock<Inner>,
    capacity: usize,
}

impl JobStore {
    /// Create a store retaining at most `capacity` jobs (minimum 1).
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(Inner {
                jobs: HashMap::new(),
                order: VecDeque::new(),
                owners: HashMap::new(),
            }),
            capacity: capacity.max(1),
        }
    }

    /// Create a new `Running` job and return its id. Evicts the oldest job if the store
    /// is at capacity.
    ///
    /// The job has **no owner**: it is visible through [`JobStore::get_visible`] to an
    /// `All` (Admin) scope only. Use [`JobStore::create_owned`] to record who enqueued it.
    pub fn create(&self) -> String {
        self.insert(None)
    }

    /// Create a new `Running` job owned by `(agent_id, principal.tenant_id)` and return its
    /// id. Evicts the oldest job (and its owner entry) if the store is at capacity.
    ///
    /// The owner is what [`JobStore::get_visible`] checks a `Tenant` scope against
    /// (ADR-0063, TENANT-07); `agent_id` is the registry id the job was enqueued under.
    pub fn create_owned(&self, agent_id: &str, principal: &Principal) -> String {
        self.insert(Some((agent_id.to_string(), principal.tenant_id.clone())))
    }

    /// The one insert-and-evict body behind [`JobStore::create`] and
    /// [`JobStore::create_owned`], so the job map and the owner map can never drift on
    /// eviction.
    fn insert(&self, owner: Option<(String, TenantId)>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        inner.jobs.insert(
            id.clone(),
            JobRecord {
                status: JobStatus::Running,
                result: None,
                error: None,
            },
        );
        if let Some(owner) = owner {
            inner.owners.insert(id.clone(), owner);
        }
        inner.order.push_back(id.clone());
        while inner.order.len() > self.capacity {
            if let Some(evicted) = inner.order.pop_front() {
                inner.jobs.remove(&evicted);
                inner.owners.remove(&evicted);
                log::debug!("job store at capacity; evicted oldest job {evicted}");
            }
        }
        id
    }

    /// Mark a job completed with its result payload. No-op if the job is gone (evicted).
    pub fn complete(&self, id: &str, result: Value) {
        self.update(id, JobStatus::Completed, Some(result), None);
    }

    /// Mark a job failed with an error message.
    pub fn fail(&self, id: &str, error: impl Into<String>) {
        self.update(id, JobStatus::Failed, None, Some(error.into()));
    }

    /// Mark a job timed out.
    pub fn time_out(&self, id: &str, error: impl Into<String>) {
        self.update(id, JobStatus::TimedOut, None, Some(error.into()));
    }

    fn update(&self, id: &str, status: JobStatus, result: Option<Value>, error: Option<String>) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(record) = inner.jobs.get_mut(id) {
            record.status = status;
            record.result = result;
            record.error = error;
        }
    }

    /// Fetch a cloned job record by id.
    pub fn get(&self, id: &str) -> Option<JobRecord> {
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        inner.jobs.get(id).cloned()
    }

    /// Fetch a cloned job record by id, only if `scope` may see it (ADR-0063, TENANT-07).
    ///
    /// - [`RunReadScope::All`] (Admin) returns the record whatever its owner, under any
    ///   `agent_id`, including a legacy unowned job.
    /// - [`RunReadScope::Tenant`] returns the record only when its stored owner is exactly
    ///   `(agent_id, tenant)`: another tenant's job, the right tenant under a different
    ///   agent id, and an unowned legacy job are all `None`.
    ///
    /// Hidden equals missing: every `None` is the same answer, so a caller cannot tell a
    /// job it may not see from a job that does not exist (or was evicted).
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::principal::{RunReadScope, TenantId};
    /// use paladin_core::platform::container::user::UserRole;
    /// use paladin_web::agent_auth::Principal;
    /// use paladin_web::job_store::JobStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let acme = TenantId::new("acme")?;
    /// let store = JobStore::new(8);
    /// let owner = Principal::new("svc-a", UserRole::User, acme.clone());
    /// let id = store.create_owned("researcher", &owner);
    ///
    /// assert!(store.get_visible(&id, "researcher", &RunReadScope::Tenant(acme.clone())).is_some());
    /// assert!(store.get_visible(&id, "other-agent", &RunReadScope::Tenant(acme)).is_none());
    /// assert!(store.get_visible(&id, "researcher", &RunReadScope::Tenant(TenantId::new("globex")?)).is_none());
    /// assert!(store.get_visible(&id, "any-agent", &RunReadScope::All).is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_visible(
        &self,
        job_id: &str,
        agent_id: &str,
        scope: &RunReadScope,
    ) -> Option<JobRecord> {
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        let record = inner.jobs.get(job_id)?;
        let permitted = match scope {
            RunReadScope::All => true,
            RunReadScope::Tenant(tenant) => inner
                .owners
                .get(job_id)
                .is_some_and(|(owner_agent, owner)| owner_agent == agent_id && owner == tenant),
        };
        permitted.then(|| record.clone())
    }

    /// Number of retained jobs.
    pub fn len(&self) -> usize {
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        inner.jobs.len()
    }

    /// Whether the store holds no jobs.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for JobStore {
    /// A store retaining the most recent 1024 jobs.
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::user::UserRole;
    use serde_json::json;

    #[test]
    fn create_yields_running_job() {
        let store = JobStore::new(8);
        let id = store.create();
        let record = store.get(&id).expect("job present");
        assert_eq!(record.status, JobStatus::Running);
        assert!(record.result.is_none());
        assert!(record.error.is_none());
    }

    #[test]
    fn complete_fail_and_timeout_transitions() {
        let store = JobStore::new(8);

        let a = store.create();
        store.complete(&a, json!({ "output": "ok" }));
        let ra = store.get(&a).unwrap();
        assert_eq!(ra.status, JobStatus::Completed);
        assert_eq!(ra.result, Some(json!({ "output": "ok" })));

        let b = store.create();
        store.fail(&b, "boom");
        assert_eq!(store.get(&b).unwrap().status, JobStatus::Failed);
        assert_eq!(store.get(&b).unwrap().error.as_deref(), Some("boom"));

        let c = store.create();
        store.time_out(&c, "deadline");
        assert_eq!(store.get(&c).unwrap().status, JobStatus::TimedOut);
    }

    #[test]
    fn unknown_job_is_none() {
        let store = JobStore::new(8);
        assert!(store.get("nope").is_none());
    }

    #[test]
    fn bounded_retention_evicts_oldest() {
        let store = JobStore::new(2);
        let a = store.create();
        let b = store.create();
        let c = store.create(); // evicts `a`

        assert!(store.get(&a).is_none(), "oldest job should be evicted");
        assert!(store.get(&b).is_some());
        assert!(store.get(&c).is_some());
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn update_after_eviction_is_noop() {
        let store = JobStore::new(1);
        let a = store.create();
        let _b = store.create(); // evicts `a`
        store.complete(&a, json!({})); // must not panic or resurrect
        assert!(store.get(&a).is_none());
    }

    fn principal(id: &str, tenant: &str) -> Principal {
        Principal::new(
            id,
            UserRole::User,
            TenantId::new(tenant).expect("valid tenant"),
        )
    }

    fn tenant(name: &str) -> RunReadScope {
        RunReadScope::Tenant(TenantId::new(name).expect("valid tenant"))
    }

    #[test]
    fn create_owned_records_the_agent_and_tenant() {
        let store = JobStore::new(8);
        let id = store.create_owned("r", &principal("svc-a", "acme"));
        let record = store.get(&id).expect("job present");
        assert_eq!(record.status, JobStatus::Running);

        let inner = store.inner.read().expect("lock");
        let (agent, owner) = inner.owners.get(&id).expect("owner recorded");
        assert_eq!(agent, "r");
        assert_eq!(owner, &TenantId::new("acme").expect("valid tenant"));
    }

    #[test]
    fn get_visible_scopes_a_job_to_its_agent_and_tenant() {
        let store = JobStore::new(8);
        let id = store.create_owned("r", &principal("svc-a", "acme"));

        assert!(store.get_visible(&id, "r", &tenant("acme")).is_some());
        assert!(store.get_visible(&id, "any", &RunReadScope::All).is_some());
        assert!(store.get_visible(&id, "r", &tenant("globex")).is_none());
        assert!(
            store
                .get_visible(&id, "other-agent", &tenant("acme"))
                .is_none()
        );
        assert!(store.get_visible("nope", "r", &tenant("acme")).is_none());
        assert!(store.get_visible("nope", "r", &RunReadScope::All).is_none());
    }

    #[test]
    fn legacy_unowned_job_is_admin_only() {
        let store = JobStore::new(8);
        let id = store.create();
        assert!(store.get_visible(&id, "r", &tenant("acme")).is_none());
        assert!(store.get_visible(&id, "r", &RunReadScope::All).is_some());
    }

    #[test]
    fn evicted_owned_job_is_gone_for_every_scope() {
        let store = JobStore::new(1);
        let first = store.create_owned("r", &principal("svc-a", "acme"));
        let second = store.create_owned("r", &principal("svc-a", "acme"));

        for scope in [tenant("acme"), RunReadScope::All] {
            assert!(store.get_visible(&first, "r", &scope).is_none());
            assert!(store.get_visible(&second, "r", &scope).is_some());
        }
        let inner = store.inner.read().expect("lock");
        assert_eq!(inner.owners.len(), inner.jobs.len());
        assert!(!inner.owners.contains_key(&first));
    }

    #[test]
    fn legacy_create_eviction_keeps_owner_map_in_step() {
        let store = JobStore::new(1);
        let owned = store.create_owned("r", &principal("svc-a", "acme"));
        let _legacy = store.create(); // evicts `owned`
        let inner = store.inner.read().expect("lock");
        assert!(!inner.owners.contains_key(&owned));
        assert_eq!(inner.owners.len(), 0);
        assert_eq!(inner.jobs.len(), 1);
    }
}
