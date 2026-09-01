/*
In-Memory Waypoint Store

An `Arc<tokio::sync::RwLock<HashMap<ThreadId, Vec<Waypoint>>>>`-backed
implementation of `WaypointPort`, for tests and local development (ENG-FR-15,
D-01: always available, no feature gate). Waypoints are stored append-only,
oldest-first, per thread.
*/

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId};
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};

/// In-memory `WaypointPort` implementation.
///
/// Cloning an `InMemoryWaypointStore` is cheap and shares the same
/// underlying store (the inner `Arc` is cloned), so a single store can be
/// handed to multiple `WarEngine` instances that must observe each other's
/// writes (e.g. the "resume from a freshly constructed engine" tracer case).
#[derive(Clone, Default)]
pub struct InMemoryWaypointStore {
    threads: Arc<RwLock<HashMap<ThreadId, Vec<Waypoint>>>>,
}

impl InMemoryWaypointStore {
    /// Construct a new, empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn to_summary(wp: &Waypoint) -> WaypointSummary {
        WaypointSummary {
            waypoint_id: wp.waypoint_id,
            parent_waypoint_id: wp.parent_waypoint_id,
            superstep: wp.superstep,
            status: wp.status.clone(),
            created_at: wp.created_at,
        }
    }
}

#[async_trait]
impl WaypointPort for InMemoryWaypointStore {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
        let mut threads = self.threads.write().await;
        threads
            .entry(wp.thread_id.clone())
            .or_default()
            .push(wp.clone());
        Ok(())
    }

    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
        let threads = self.threads.read().await;
        Ok(threads.get(thread).and_then(|wps| wps.last().cloned()))
    }

    async fn get(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<Option<Waypoint>, WaypointError> {
        let threads = self.threads.read().await;
        Ok(threads
            .get(thread)
            .and_then(|wps| wps.iter().find(|wp| &wp.waypoint_id == id).cloned()))
    }

    async fn history(
        &self,
        thread: &ThreadId,
        limit: Option<u32>,
        before: Option<WaypointId>,
    ) -> Result<Vec<WaypointSummary>, WaypointError> {
        let threads = self.threads.read().await;
        let Some(wps) = threads.get(thread) else {
            return Ok(vec![]);
        };

        // Stored oldest-first; present newest-first.
        let mut newest_first: Vec<&Waypoint> = wps.iter().rev().collect();

        if let Some(before_id) = before {
            let cut = newest_first
                .iter()
                .position(|wp| wp.waypoint_id == before_id)
                .map(|idx| idx + 1)
                .unwrap_or(newest_first.len());
            newest_first = newest_first.split_off(cut.min(newest_first.len()));
        }

        if let Some(limit) = limit {
            newest_first.truncate(limit as usize);
        }

        Ok(newest_first.into_iter().map(Self::to_summary).collect())
    }

    async fn list_threads(
        &self,
        limit: Option<u32>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<ThreadSummary>, WaypointError> {
        let threads = self.threads.read().await;
        let mut summaries: Vec<ThreadSummary> = threads
            .iter()
            .filter_map(|(thread_id, wps)| {
                wps.last().map(|latest| ThreadSummary {
                    thread_id: thread_id.clone(),
                    latest_status: latest.status.clone(),
                    last_updated_at: latest.created_at,
                })
            })
            .collect();

        summaries.sort_by_key(|s| std::cmp::Reverse(s.last_updated_at));

        if let Some(before) = before {
            summaries.retain(|s| s.last_updated_at < before);
        }

        if let Some(limit) = limit {
            summaries.truncate(limit as usize);
        }

        Ok(summaries)
    }

    async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError> {
        let mut threads = self.threads.write().await;
        Ok(threads
            .remove(thread)
            .map(|wps| wps.len() as u64)
            .unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema};
    use paladin_core::platform::container::waypoint::{GraphFingerprint, WaypointStatus};

    fn fixture_waypoint(thread: &ThreadId, superstep: u64) -> Waypoint {
        Waypoint {
            thread_id: thread.clone(),
            waypoint_id: WaypointId::new(),
            parent_waypoint_id: None,
            superstep,
            graph_fingerprint: GraphFingerprint::from_canonical_bytes(b"fixture"),
            battlefield: Battlefield::new(BattlefieldSchema::new(vec![])),
            vanguard: vec![],
            completed: vec![],
            status: WaypointStatus::Completed,
            created_at: Utc::now(),
            schema_version: Waypoint::current_schema_version(),
        }
    }

    #[tokio::test]
    async fn save_then_latest_round_trips() {
        let store = InMemoryWaypointStore::new();
        let thread = ThreadId::new("t1").unwrap();
        let wp = fixture_waypoint(&thread, 0);
        store.save(&wp).await.unwrap();

        let loaded = store.latest(&thread).await.unwrap().unwrap();
        assert_eq!(loaded, wp);
    }

    #[tokio::test]
    async fn latest_on_unknown_thread_is_none_not_error() {
        let store = InMemoryWaypointStore::new();
        let thread = ThreadId::new("unknown").unwrap();
        assert!(store.latest(&thread).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn history_is_newest_first_and_paginated() {
        let store = InMemoryWaypointStore::new();
        let thread = ThreadId::new("t1").unwrap();
        for i in 0..3 {
            store.save(&fixture_waypoint(&thread, i)).await.unwrap();
        }

        let history = store.history(&thread, None, None).await.unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].superstep, 2);
        assert_eq!(history[1].superstep, 1);
        assert_eq!(history[2].superstep, 0);

        let limited = store.history(&thread, Some(2), None).await.unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].superstep, 2);
    }

    #[tokio::test]
    async fn delete_thread_removes_all_waypoints_and_counts_them() {
        let store = InMemoryWaypointStore::new();
        let thread = ThreadId::new("t1").unwrap();
        store.save(&fixture_waypoint(&thread, 0)).await.unwrap();
        store.save(&fixture_waypoint(&thread, 1)).await.unwrap();

        let deleted = store.delete_thread(&thread).await.unwrap();
        assert_eq!(deleted, 2);
        assert!(store.latest(&thread).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_threads_reflects_latest_status_per_thread() {
        let store = InMemoryWaypointStore::new();
        let t1 = ThreadId::new("t1").unwrap();
        let t2 = ThreadId::new("t2").unwrap();
        store.save(&fixture_waypoint(&t1, 0)).await.unwrap();
        store.save(&fixture_waypoint(&t2, 0)).await.unwrap();

        let threads = store.list_threads(None, None).await.unwrap();
        assert_eq!(threads.len(), 2);
    }
}
