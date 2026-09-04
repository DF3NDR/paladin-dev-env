/*
In-Memory Waypoint Store

An `Arc<tokio::sync::RwLock<HashMap<ThreadId, Vec<Waypoint>>>>`-backed
implementation of `WaypointPort`, for tests and local development (ENG-FR-15,
D-01: always available, no feature gate). `save` appends a new waypoint_id or
upserts (replaces in place) an existing one, per the WaypointPort::save
contract; ordering for `history`/`latest` is never inferred from storage
position -- both are computed by explicit sort/max over `created_at` and
`superstep`, per the documented tiebreak.
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
        let entries = threads.entry(wp.thread_id.clone()).or_default();
        // Upsert: re-saving an existing waypoint_id replaces its entry
        // in place rather than appending a duplicate (WaypointPort::save
        // rustdoc contract, contract_tests::resave_existing_waypoint_id_upserts).
        match entries
            .iter_mut()
            .find(|existing| existing.waypoint_id == wp.waypoint_id)
        {
            Some(existing) => *existing = wp.clone(),
            None => entries.push(wp.clone()),
        }
        Ok(())
    }

    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
        let threads = self.threads.read().await;
        Ok(threads.get(thread).and_then(|wps| {
            wps.iter()
                .max_by(|a, b| {
                    a.created_at
                        .cmp(&b.created_at)
                        .then_with(|| a.superstep.cmp(&b.superstep))
                })
                .cloned()
        }))
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

        // Sort explicitly by created_at descending, superstep descending as
        // the documented tiebreak (WaypointPort::history rustdoc) -- do not
        // rely on insertion order, which `save`'s upsert can disturb.
        let mut newest_first: Vec<&Waypoint> = wps.iter().collect();
        newest_first.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.superstep.cmp(&a.superstep))
        });

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

    async fn delete_waypoint(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<bool, WaypointError> {
        let mut threads = self.threads.write().await;
        let Some(wps) = threads.get_mut(thread) else {
            return Ok(false);
        };
        let before_len = wps.len();
        wps.retain(|wp| &wp.waypoint_id != id);
        Ok(wps.len() < before_len)
    }

    async fn prune_thread(
        &self,
        thread: &ThreadId,
        keep: &[WaypointId],
    ) -> Result<u64, WaypointError> {
        // The whole operation happens under a single write-lock acquisition
        // -- no intermediate state is ever observable to a concurrent
        // reader, so this is stronger than the trait's default provided
        // implementation needs to be, at no extra cost for an in-process
        // store.
        let keep_set: std::collections::HashSet<WaypointId> = keep.iter().copied().collect();
        let mut threads = self.threads.write().await;
        let Some(wps) = threads.get_mut(thread) else {
            return Ok(0);
        };
        let before_len = wps.len();
        wps.retain(|wp| keep_set.contains(&wp.waypoint_id));
        Ok((before_len - wps.len()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waypoint::contract_tests;

    // One #[tokio::test] per shared contract function (D-09), each against a
    // fresh store, so a failure names the violated contract clause. See
    // `contract_tests` for the assertions themselves -- this file only wires
    // InMemoryWaypointStore into them.

    #[tokio::test]
    async fn save_then_latest_returns_saved_waypoint_round_tripped() {
        contract_tests::save_then_latest_returns_saved_waypoint_round_tripped(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn latest_on_unknown_thread_is_none() {
        contract_tests::latest_on_unknown_thread_is_none(&InMemoryWaypointStore::new()).await;
    }

    #[tokio::test]
    async fn get_on_known_thread_unknown_id_is_none() {
        contract_tests::get_on_known_thread_unknown_id_is_none(&InMemoryWaypointStore::new()).await;
    }

    #[tokio::test]
    async fn get_on_known_thread_known_id_returns_exact_waypoint() {
        contract_tests::get_on_known_thread_known_id_returns_exact_waypoint(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn history_with_no_pagination_returns_all_newest_first() {
        contract_tests::history_with_no_pagination_returns_all_newest_first(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn history_limit_and_before_paginate_with_no_overlap_or_gap() {
        contract_tests::history_limit_and_before_paginate_with_no_overlap_or_gap(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn history_limit_zero_returns_empty() {
        contract_tests::history_limit_zero_returns_empty(&InMemoryWaypointStore::new()).await;
    }

    #[tokio::test]
    async fn history_on_unknown_thread_returns_empty() {
        contract_tests::history_on_unknown_thread_returns_empty(&InMemoryWaypointStore::new())
            .await;
    }

    #[tokio::test]
    async fn history_same_created_at_tiebreaks_by_descending_superstep_stably() {
        contract_tests::history_same_created_at_tiebreaks_by_descending_superstep_stably(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn list_threads_empty_then_three_threads_newest_activity_first() {
        contract_tests::list_threads_empty_then_three_threads_newest_activity_first(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn delete_thread_removes_count_and_unknown_returns_zero() {
        contract_tests::delete_thread_removes_count_and_unknown_returns_zero(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn resave_existing_waypoint_id_upserts() {
        contract_tests::resave_existing_waypoint_id_upserts(&InMemoryWaypointStore::new()).await;
    }

    #[tokio::test]
    async fn child_lineage_survives_round_trip() {
        contract_tests::child_lineage_survives_round_trip(&InMemoryWaypointStore::new()).await;
    }

    #[tokio::test]
    async fn run_all_contract_functions_smoke_aggregate() {
        contract_tests::run_all(&InMemoryWaypointStore::new()).await;
    }

    // ── delete_waypoint / prune_thread (Plan 22-13, G-22-2) ──────────────

    #[tokio::test]
    async fn delete_waypoint_removes_named_id_and_leaves_others() {
        contract_tests::delete_waypoint_removes_named_id_and_leaves_others(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged() {
        contract_tests::delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn delete_waypoint_unknown_thread_is_false() {
        contract_tests::delete_waypoint_unknown_thread_is_false(&InMemoryWaypointStore::new())
            .await;
    }

    #[tokio::test]
    async fn prune_thread_keeps_named_ids_byte_identical_and_ordered() {
        contract_tests::prune_thread_keeps_named_ids_byte_identical_and_ordered(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn prune_thread_empty_keep_removes_everything() {
        contract_tests::prune_thread_empty_keep_removes_everything(&InMemoryWaypointStore::new())
            .await;
    }

    #[tokio::test]
    async fn prune_thread_unknown_thread_returns_zero() {
        contract_tests::prune_thread_unknown_thread_returns_zero(&InMemoryWaypointStore::new())
            .await;
    }

    #[tokio::test]
    async fn prune_thread_ignores_keep_ids_not_in_thread() {
        contract_tests::prune_thread_ignores_keep_ids_not_in_thread(&InMemoryWaypointStore::new())
            .await;
    }

    #[tokio::test]
    async fn prune_thread_idempotent_second_run_removes_nothing() {
        contract_tests::prune_thread_idempotent_second_run_removes_nothing(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn prune_thread_converges_from_superset_to_target() {
        contract_tests::prune_thread_converges_from_superset_to_target(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn prune_thread_large_keep_set_1200_to_1100() {
        contract_tests::prune_thread_large_keep_set_1200_to_1100(&InMemoryWaypointStore::new())
            .await;
    }

    // ── BUG-04 / ENG-FR-12a: FrontierSnapshot (Plan 22.1-06) ─────────────

    #[tokio::test]
    async fn frontier_survives_save_latest_and_get_round_trip() {
        contract_tests::frontier_survives_save_latest_and_get_round_trip(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    #[tokio::test]
    async fn pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot() {
        contract_tests::pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot(
            &InMemoryWaypointStore::new(),
        )
        .await;
    }

    // ── CF-FR-12 / D-14: MusterProgress (Plan 23-06) ──────────────────────

    #[tokio::test]
    async fn muster_progress_round_trips() {
        contract_tests::muster_progress_round_trips(&InMemoryWaypointStore::new()).await;
    }

    #[tokio::test]
    async fn muster_progress_none_round_trips_as_none() {
        contract_tests::muster_progress_none_round_trips_as_none(&InMemoryWaypointStore::new())
            .await;
    }
}
