//! Chronicle read facade -- the port-only, `paladin-battalion`-independent
//! read surface over a thread's Waypoint history (HITL-03, D-16).
//!
//! Mirrors `waypoint_retention.rs`'s own shape: a struct holding
//! `Arc<dyn WaypointPort>`, a constructor taking it, and no other
//! dependency. `ChronicleService` takes no dependency on `paladin_battalion`
//! (ADR-0031) -- so `paladin-web` can later reuse these same reads through
//! the same port, unchanged (plan 24-11).
//!
//! Performs no authorisation of its own, by design: authorisation is
//! enforced at the HTTP adapter's `route_layer` (T-24-29), not here.

use std::sync::Arc;

use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort, WaypointSummary};

// RED-STATE MARKER (Plan 24-07, Task 3): `ChronicleService` is intentionally
// absent from this commit -- the test module below fails to compile until
// the GREEN commit lands `pub struct ChronicleService` and its three
// methods (`history`, `inspect`, `latest_on_branch`).

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Duration, Utc};
    use paladin_ports::output::waypoint_port::ThreadSummary;
    use paladin_storage::waypoint::contract_tests::sample_waypoint_at;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    #[tokio::test]
    async fn chronicle_history_returns_newest_first_summaries_with_lineage() {
        let port = Arc::new(InMemoryWaypointStore::new());
        let t = thread("chronicle-history-newest-first");
        let now = Utc::now();

        let wp0 = sample_waypoint_at(&t, 0, now);
        let wp0_id = wp0.waypoint_id;
        port.save(&wp0).await.unwrap();

        let mut wp1 = sample_waypoint_at(&t, 1, now + Duration::seconds(1));
        wp1.parent_waypoint_id = Some(wp0_id);
        let wp1_id = wp1.waypoint_id;
        port.save(&wp1).await.unwrap();

        let mut wp2 = sample_waypoint_at(&t, 2, now + Duration::seconds(2));
        wp2.parent_waypoint_id = Some(wp1_id);
        wp2.fork_of = Some(wp0_id);
        let wp2_id = wp2.waypoint_id;
        port.save(&wp2).await.unwrap();

        let chronicle = ChronicleService::new(port);
        let history = chronicle.history(&t, 10, None).await.unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].waypoint_id, wp2_id);
        assert_eq!(history[0].fork_of, Some(wp0_id));
        assert_eq!(history[0].parent_waypoint_id, Some(wp1_id));
        assert_eq!(history[1].waypoint_id, wp1_id);
        assert_eq!(history[2].waypoint_id, wp0_id);
    }

    #[tokio::test]
    async fn chronicle_history_honours_limit_and_before() {
        let port = Arc::new(InMemoryWaypointStore::new());
        let t = thread("chronicle-history-limit-before");
        let now = Utc::now();
        let mut ids = Vec::new();
        for i in 0..3u64 {
            let wp = sample_waypoint_at(&t, i, now + Duration::seconds(i as i64));
            ids.push(wp.waypoint_id);
            port.save(&wp).await.unwrap();
        }

        let chronicle = ChronicleService::new(port);
        let first_page = chronicle.history(&t, 2, None).await.unwrap();
        assert_eq!(first_page.len(), 2);
        assert_eq!(first_page[0].waypoint_id, ids[2]);
        assert_eq!(first_page[1].waypoint_id, ids[1]);

        let cursor = first_page[1].waypoint_id;
        let second_page = chronicle.history(&t, 2, Some(cursor)).await.unwrap();
        assert_eq!(second_page.len(), 1);
        assert_eq!(second_page[0].waypoint_id, ids[0]);
    }

    #[tokio::test]
    async fn chronicle_inspect_returns_the_full_waypoint() {
        let port = Arc::new(InMemoryWaypointStore::new());
        let t = thread("chronicle-inspect");
        let wp = sample_waypoint_at(&t, 0, Utc::now());
        let id = wp.waypoint_id;
        port.save(&wp).await.unwrap();

        let chronicle = ChronicleService::new(port);
        let loaded = chronicle.inspect(&t, id).await.unwrap();
        assert_eq!(loaded, wp);

        let err = chronicle
            .inspect(&t, WaypointId::generate())
            .await
            .unwrap_err();
        assert!(matches!(err, WaypointError::NotFound(_)));
    }

    #[tokio::test]
    async fn chronicle_latest_on_branch_filters_by_fork_of() {
        let port = Arc::new(InMemoryWaypointStore::new());
        let t = thread("chronicle-latest-on-branch");
        let now = Utc::now();

        let mainline_root = sample_waypoint_at(&t, 0, now);
        let root_id = mainline_root.waypoint_id;
        port.save(&mainline_root).await.unwrap();

        let mut branch_a_first = sample_waypoint_at(&t, 1, now + Duration::seconds(1));
        branch_a_first.fork_of = Some(root_id);
        port.save(&branch_a_first).await.unwrap();

        let mut branch_a_second = sample_waypoint_at(&t, 2, now + Duration::seconds(2));
        branch_a_second.fork_of = Some(root_id);
        let branch_a_second_id = branch_a_second.waypoint_id;
        port.save(&branch_a_second).await.unwrap();

        let branch_b_root = WaypointId::generate();
        let mut branch_b_first = sample_waypoint_at(&t, 1, now + Duration::seconds(3));
        branch_b_first.fork_of = Some(branch_b_root);
        let branch_b_first_id = branch_b_first.waypoint_id;
        port.save(&branch_b_first).await.unwrap();

        let chronicle = ChronicleService::new(port);
        let latest_a = chronicle
            .latest_on_branch(&t, root_id)
            .await
            .unwrap()
            .expect("branch a must have a latest summary");
        assert_eq!(latest_a.waypoint_id, branch_a_second_id);

        let latest_b = chronicle
            .latest_on_branch(&t, branch_b_root)
            .await
            .unwrap()
            .expect("branch b must have a latest summary");
        assert_eq!(latest_b.waypoint_id, branch_b_first_id);

        let latest_none = chronicle
            .latest_on_branch(&t, WaypointId::generate())
            .await
            .unwrap();
        assert!(latest_none.is_none());
    }

    /// A `WaypointPort` wrapper whose `get` panics -- proves
    /// `latest_on_branch` reaches only `history`, never `get`, for its
    /// branch-latest resolution (D-14 made this possible: the whole branch
    /// tree reconstructs from `WaypointSummary` alone).
    struct GetPanicsStore {
        inner: InMemoryWaypointStore,
    }

    #[async_trait]
    impl WaypointPort for GetPanicsStore {
        async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
            self.inner.save(wp).await
        }

        async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
            self.inner.latest(thread).await
        }

        async fn get(
            &self,
            _thread: &ThreadId,
            _id: &WaypointId,
        ) -> Result<Option<Waypoint>, WaypointError> {
            panic!("ChronicleService::latest_on_branch must never call WaypointPort::get")
        }

        async fn history(
            &self,
            thread: &ThreadId,
            limit: Option<u32>,
            before: Option<WaypointId>,
        ) -> Result<Vec<WaypointSummary>, WaypointError> {
            self.inner.history(thread, limit, before).await
        }

        async fn list_threads(
            &self,
            limit: Option<u32>,
            before: Option<DateTime<Utc>>,
        ) -> Result<Vec<ThreadSummary>, WaypointError> {
            self.inner.list_threads(limit, before).await
        }

        async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError> {
            self.inner.delete_thread(thread).await
        }

        async fn delete_waypoint(
            &self,
            thread: &ThreadId,
            id: &WaypointId,
        ) -> Result<bool, WaypointError> {
            self.inner.delete_waypoint(thread, id).await
        }
    }

    #[tokio::test]
    async fn chronicle_latest_on_branch_needs_no_full_waypoint_loads() {
        let inner = InMemoryWaypointStore::new();
        let t = thread("chronicle-latest-on-branch-no-get");
        let root = sample_waypoint_at(&t, 0, Utc::now());
        let root_id = root.waypoint_id;
        inner.save(&root).await.unwrap();
        let mut branch = sample_waypoint_at(&t, 1, Utc::now() + Duration::seconds(1));
        branch.fork_of = Some(root_id);
        let branch_id = branch.waypoint_id;
        inner.save(&branch).await.unwrap();

        let port: Arc<dyn WaypointPort> = Arc::new(GetPanicsStore { inner });
        let chronicle = ChronicleService::new(port);

        let result = chronicle
            .latest_on_branch(&t, root_id)
            .await
            .unwrap()
            .expect("branch must have a latest summary");
        assert_eq!(result.waypoint_id, branch_id);
    }

    #[tokio::test]
    async fn chronicle_empty_thread_returns_empty_history() {
        let port = Arc::new(InMemoryWaypointStore::new());
        let chronicle = ChronicleService::new(port);
        let t = thread("chronicle-empty-thread");
        let history = chronicle.history(&t, 10, None).await.unwrap();
        assert!(history.is_empty());
    }
}
