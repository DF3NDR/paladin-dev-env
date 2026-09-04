//! Shared `WaypointPort` contract suite (D-09).
//!
//! One generic async function per contract clause, each taking `&dyn
//! WaypointPort` and asserting inside. Every backend (`InMemoryWaypointStore`
//! today; `SqliteWaypointStore` and `PostgresWaypointStore` in Plan 22-06)
//! invokes these unchanged from its own `#[tokio::test]`s, so "identical
//! suite across backends" (ENG-FR-17) is enforced by construction rather than
//! by convention. Named per-clause (not a declarative macro) so a failure
//! names the violated contract clause rather than a line number (D-09).
//!
//! This module is plain (not `#[cfg(test)]`) so both unit tests inside each
//! backend crate and future Docker-gated integration tests can call it.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema, FieldName};
use paladin_core::platform::container::directive::MusterTask;
use paladin_core::platform::container::waypoint::{
    FrontierEdgeState, FrontierSnapshot, GraphFingerprint, MusterProgress, NodeId, ThreadId,
    Waypoint, WaypointId, WaypointStatus, canonical_edge_condition,
};
use paladin_ports::output::waypoint_port::WaypointPort;

/// Build a `Waypoint` fixture for `thread` at `superstep`, stamped with the
/// current time. Every contract function and every backend's test harness
/// should build fixtures through this function (or [`sample_waypoint_at`])
/// so all backends exercise identical inputs.
pub fn sample_waypoint(thread: &ThreadId, superstep: u64) -> Waypoint {
    sample_waypoint_at(thread, superstep, Utc::now())
}

/// Like [`sample_waypoint`], but with an explicit `created_at`, so tests can
/// construct multiple Waypoints that share (or deliberately differ in)
/// timestamp.
pub fn sample_waypoint_at(
    thread: &ThreadId,
    superstep: u64,
    created_at: DateTime<Utc>,
) -> Waypoint {
    Waypoint {
        thread_id: thread.clone(),
        waypoint_id: WaypointId::generate(),
        parent_waypoint_id: None,
        superstep,
        graph_fingerprint: GraphFingerprint::from_canonical_bytes(b"contract-suite-fixture"),
        battlefield: Battlefield::new(BattlefieldSchema::new(vec![])),
        vanguard: vec![],
        completed: vec![],
        status: WaypointStatus::Running,
        created_at,
        schema_version: Waypoint::current_schema_version(),
        visit_counts: std::collections::BTreeMap::new(),
        frontier: FrontierSnapshot::default(),
        muster_progress: None,
    }
}

/// `save` then `latest` returns the saved Waypoint, byte-identical after a
/// serde round trip.
pub async fn save_then_latest_returns_saved_waypoint_round_tripped(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-save-latest").unwrap();
    let wp = sample_waypoint(&thread, 0);
    port.save(&wp).await.unwrap();

    let loaded = port.latest(&thread).await.unwrap().unwrap();

    // Prove the round trip is exact, not just "some fields match": serialize
    // both sides and compare bytes.
    let expected_json = serde_json::to_string(&wp).unwrap();
    let loaded_json = serde_json::to_string(&loaded).unwrap();
    assert_eq!(loaded_json, expected_json);
    assert_eq!(loaded, wp);
}

/// `latest` on an unknown `ThreadId` returns `Ok(None)`, not an error.
pub async fn latest_on_unknown_thread_is_none(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-latest-unknown-thread").unwrap();
    assert_eq!(port.latest(&thread).await.unwrap(), None);
}

/// `get` on a known thread with an unknown id returns `Ok(None)`.
pub async fn get_on_known_thread_unknown_id_is_none(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-get-unknown-id").unwrap();
    let wp = sample_waypoint(&thread, 0);
    port.save(&wp).await.unwrap();

    let unknown_id = WaypointId::generate();
    assert_eq!(port.get(&thread, &unknown_id).await.unwrap(), None);
}

/// `get` on a known `(thread, id)` pair returns that exact Waypoint.
pub async fn get_on_known_thread_known_id_returns_exact_waypoint(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-get-known-id").unwrap();
    let wp = sample_waypoint(&thread, 0);
    port.save(&wp).await.unwrap();

    let loaded = port.get(&thread, &wp.waypoint_id).await.unwrap().unwrap();
    assert_eq!(loaded, wp);
}

/// Saving five Waypoints then calling `history(thread, None, None)` returns
/// all five, newest-first.
pub async fn history_with_no_pagination_returns_all_newest_first(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-history-all").unwrap();
    let created_at = Utc::now();
    for superstep in 0..5u64 {
        port.save(&sample_waypoint_at(&thread, superstep, created_at))
            .await
            .unwrap();
    }

    let history = port.history(&thread, None, None).await.unwrap();
    let supersteps: Vec<u64> = history.iter().map(|s| s.superstep).collect();
    assert_eq!(supersteps, vec![4, 3, 2, 1, 0]);
}

/// `history(thread, Some(2), None)` returns the two newest; passing the
/// older of those as `before` returns the next two, with no overlap and no
/// gap against the first page.
pub async fn history_limit_and_before_paginate_with_no_overlap_or_gap(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-history-pagination").unwrap();
    let created_at = Utc::now();
    for superstep in 0..5u64 {
        port.save(&sample_waypoint_at(&thread, superstep, created_at))
            .await
            .unwrap();
    }

    let first_page = port.history(&thread, Some(2), None).await.unwrap();
    assert_eq!(first_page.len(), 2);
    assert_eq!(first_page[0].superstep, 4);
    assert_eq!(first_page[1].superstep, 3);

    let cursor = first_page[1].waypoint_id;
    let second_page = port.history(&thread, Some(2), Some(cursor)).await.unwrap();
    assert_eq!(second_page.len(), 2);
    assert_eq!(second_page[0].superstep, 2);
    assert_eq!(second_page[1].superstep, 1);

    // No overlap: no id from the second page appears in the first.
    for summary in &second_page {
        assert!(
            !first_page
                .iter()
                .any(|s| s.waypoint_id == summary.waypoint_id)
        );
    }
}

/// `history(thread, Some(0), None)` returns an empty `Vec`, not the whole
/// thread.
pub async fn history_limit_zero_returns_empty(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-history-limit-zero").unwrap();
    port.save(&sample_waypoint(&thread, 0)).await.unwrap();

    let history = port.history(&thread, Some(0), None).await.unwrap();
    assert_eq!(history, vec![]);
}

/// `history` on an unknown thread returns an empty `Vec`, not an error.
pub async fn history_on_unknown_thread_returns_empty(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-history-unknown-thread").unwrap();
    assert_eq!(port.history(&thread, None, None).await.unwrap(), vec![]);
}

/// Two Waypoints written with the same `created_at` are ordered by
/// descending `superstep`, and repeated calls return the identical order.
pub async fn history_same_created_at_tiebreaks_by_descending_superstep_stably(
    port: &dyn WaypointPort,
) {
    let thread = ThreadId::new("contract-history-tiebreak").unwrap();
    let created_at = Utc::now();
    // Save the lower superstep first, to prove the tiebreak is superstep,
    // not insertion order.
    port.save(&sample_waypoint_at(&thread, 1, created_at))
        .await
        .unwrap();
    port.save(&sample_waypoint_at(&thread, 5, created_at))
        .await
        .unwrap();

    let first_call = port.history(&thread, None, None).await.unwrap();
    let second_call = port.history(&thread, None, None).await.unwrap();

    let supersteps: Vec<u64> = first_call.iter().map(|s| s.superstep).collect();
    assert_eq!(supersteps, vec![5, 1]);
    assert_eq!(first_call, second_call);
}

/// `list_threads` on an empty store returns an empty `Vec`; after writing to
/// three threads it returns three summaries, newest-activity-first.
pub async fn list_threads_empty_then_three_threads_newest_activity_first(port: &dyn WaypointPort) {
    let t1 = ThreadId::new("contract-list-threads-t1").unwrap();
    let t2 = ThreadId::new("contract-list-threads-t2").unwrap();
    let t3 = ThreadId::new("contract-list-threads-t3").unwrap();

    assert_eq!(port.list_threads(None, None).await.unwrap(), vec![]);

    let base = Utc::now();
    port.save(&sample_waypoint_at(&t1, 0, base)).await.unwrap();
    port.save(&sample_waypoint_at(
        &t2,
        0,
        base + chrono::Duration::seconds(1),
    ))
    .await
    .unwrap();
    port.save(&sample_waypoint_at(
        &t3,
        0,
        base + chrono::Duration::seconds(2),
    ))
    .await
    .unwrap();

    let threads = port.list_threads(None, None).await.unwrap();
    assert_eq!(threads.len(), 3);
    let ids: Vec<&ThreadId> = threads.iter().map(|s| &s.thread_id).collect();
    assert_eq!(ids, vec![&t3, &t2, &t1]);
}

/// `delete_thread` on a thread with five Waypoints returns 5 and leaves
/// `latest` at `Ok(None)`; on an unknown thread it returns `Ok(0)`.
pub async fn delete_thread_removes_count_and_unknown_returns_zero(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-delete-thread").unwrap();
    for superstep in 0..5u64 {
        port.save(&sample_waypoint(&thread, superstep))
            .await
            .unwrap();
    }

    let deleted = port.delete_thread(&thread).await.unwrap();
    assert_eq!(deleted, 5);
    assert_eq!(port.latest(&thread).await.unwrap(), None);

    let unknown = ThreadId::new("contract-delete-thread-unknown").unwrap();
    assert_eq!(port.delete_thread(&unknown).await.unwrap(), 0);
}

/// Re-saving a Waypoint with an existing `waypoint_id` behaves exactly as
/// the port rustdoc documents: an upsert, not a rejection or a duplicate
/// entry.
pub async fn resave_existing_waypoint_id_upserts(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-resave-upsert").unwrap();
    let mut wp = sample_waypoint(&thread, 0);
    port.save(&wp).await.unwrap();

    // Mutate content but keep the same waypoint_id, then re-save.
    wp.status = WaypointStatus::Completed;
    port.save(&wp).await.unwrap();

    let loaded = port.get(&thread, &wp.waypoint_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, WaypointStatus::Completed);

    // No duplicate entry: history for this thread still reports exactly one
    // waypoint.
    let history = port.history(&thread, None, None).await.unwrap();
    assert_eq!(history.len(), 1);
}

/// Parent lineage survives a round trip: a child Waypoint's
/// `parent_waypoint_id` reads back equal to the parent's id.
pub async fn child_lineage_survives_round_trip(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-lineage").unwrap();
    let root = Waypoint::new_root(
        thread.clone(),
        0,
        GraphFingerprint::from_canonical_bytes(b"contract-suite-fixture"),
        Battlefield::new(BattlefieldSchema::new(vec![])),
        vec![],
        vec![],
        WaypointStatus::Running,
        std::collections::BTreeMap::new(),
        FrontierSnapshot::default(),
    );
    port.save(&root).await.unwrap();

    let child = Waypoint::new_child(
        &root,
        1,
        GraphFingerprint::from_canonical_bytes(b"contract-suite-fixture"),
        Battlefield::new(BattlefieldSchema::new(vec![])),
        vec![],
        vec![],
        WaypointStatus::Running,
        std::collections::BTreeMap::new(),
        FrontierSnapshot::default(),
    );
    port.save(&child).await.unwrap();

    let loaded_child = port
        .get(&thread, &child.waypoint_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_child.parent_waypoint_id, Some(root.waypoint_id));
}

/// `delete_waypoint` removes exactly the named Waypoint: a thread holding
/// three, after deleting the middle one, holds the other two and reports
/// that a row was removed.
pub async fn delete_waypoint_removes_named_id_and_leaves_others(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-delete-one-removes-named").unwrap();
    let base = Utc::now();
    let mut ids = Vec::new();
    for superstep in 0..3u64 {
        let wp = sample_waypoint_at(
            &thread,
            superstep,
            base + chrono::Duration::seconds(superstep as i64),
        );
        ids.push(wp.waypoint_id);
        port.save(&wp).await.unwrap();
    }

    let middle = ids[1];
    let removed = port.delete_waypoint(&thread, &middle).await.unwrap();
    assert!(removed);

    assert_eq!(port.get(&thread, &middle).await.unwrap(), None);
    assert!(port.get(&thread, &ids[0]).await.unwrap().is_some());
    assert!(port.get(&thread, &ids[2]).await.unwrap().is_some());

    let history = port.history(&thread, None, None).await.unwrap();
    assert_eq!(history.len(), 2);
}

/// `delete_waypoint` on an id the thread does not hold reports that no row
/// was removed, is not an error, and leaves the thread's contents unchanged.
pub async fn delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged(
    port: &dyn WaypointPort,
) {
    let thread = ThreadId::new("contract-delete-one-unknown-id").unwrap();
    let wp = sample_waypoint(&thread, 0);
    port.save(&wp).await.unwrap();

    let unknown_id = WaypointId::generate();
    let removed = port.delete_waypoint(&thread, &unknown_id).await.unwrap();
    assert!(!removed);

    let history = port.history(&thread, None, None).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].waypoint_id, wp.waypoint_id);
}

/// `delete_waypoint` on a thread the backend does not hold reports no row
/// removed, not an error.
pub async fn delete_waypoint_unknown_thread_is_false(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-delete-one-unknown-thread").unwrap();
    let id = WaypointId::generate();
    let removed = port.delete_waypoint(&thread, &id).await.unwrap();
    assert!(!removed);
}

/// `prune_thread` with a keep-set naming two of five ids leaves exactly
/// those two, in the same history order, with their payloads
/// byte-identical to what was saved. Also proves the returned count is the
/// number of Waypoints removed.
pub async fn prune_thread_keeps_named_ids_byte_identical_and_ordered(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-prune-keeps-named").unwrap();
    let base = Utc::now();
    let mut saved = Vec::new();
    for superstep in 0..5u64 {
        let wp = sample_waypoint_at(
            &thread,
            superstep,
            base + chrono::Duration::seconds(superstep as i64),
        );
        port.save(&wp).await.unwrap();
        saved.push(wp);
    }

    // Keep the two newest (supersteps 3 and 4).
    let keep_ids = vec![saved[3].waypoint_id, saved[4].waypoint_id];
    let removed = port.prune_thread(&thread, &keep_ids).await.unwrap();
    assert_eq!(removed, 3);

    let history = port.history(&thread, None, None).await.unwrap();
    let ids: Vec<WaypointId> = history.iter().map(|s| s.waypoint_id).collect();
    assert_eq!(ids, vec![saved[4].waypoint_id, saved[3].waypoint_id]);

    for kept in &saved[3..5] {
        let loaded = port.get(&thread, &kept.waypoint_id).await.unwrap().unwrap();
        let expected_json = serde_json::to_string(kept).unwrap();
        let loaded_json = serde_json::to_string(&loaded).unwrap();
        assert_eq!(loaded_json, expected_json);
    }
}

/// `prune_thread` with an empty keep-set removes every Waypoint of the
/// thread.
pub async fn prune_thread_empty_keep_removes_everything(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-prune-empty-keep").unwrap();
    for superstep in 0..4u64 {
        port.save(&sample_waypoint(&thread, superstep))
            .await
            .unwrap();
    }

    let removed = port.prune_thread(&thread, &[]).await.unwrap();
    assert_eq!(removed, 4);
    assert_eq!(port.latest(&thread).await.unwrap(), None);
}

/// `prune_thread` on a thread the backend does not hold returns zero, not
/// an error.
pub async fn prune_thread_unknown_thread_returns_zero(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-prune-unknown-thread").unwrap();
    let removed = port
        .prune_thread(&thread, &[WaypointId::generate()])
        .await
        .unwrap();
    assert_eq!(removed, 0);
}

/// `prune_thread` given ids that are not in the thread ignores them: they
/// are not an error and cause no deletion of anything else.
pub async fn prune_thread_ignores_keep_ids_not_in_thread(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-prune-ignores-foreign-ids").unwrap();
    let base = Utc::now();
    let mut saved = Vec::new();
    for superstep in 0..3u64 {
        let wp = sample_waypoint_at(
            &thread,
            superstep,
            base + chrono::Duration::seconds(superstep as i64),
        );
        port.save(&wp).await.unwrap();
        saved.push(wp);
    }

    let mut keep_ids: Vec<WaypointId> = saved.iter().map(|wp| wp.waypoint_id).collect();
    keep_ids.push(WaypointId::generate()); // not in the thread at all

    let removed = port.prune_thread(&thread, &keep_ids).await.unwrap();
    assert_eq!(removed, 0);
    assert_eq!(port.history(&thread, None, None).await.unwrap().len(), 3);
}

/// `prune_thread` run twice with the same keep-set removes nothing the
/// second time and leaves the keep-set intact -- idempotence.
pub async fn prune_thread_idempotent_second_run_removes_nothing(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-prune-idempotent").unwrap();
    let base = Utc::now();
    let mut saved = Vec::new();
    for superstep in 0..5u64 {
        let wp = sample_waypoint_at(
            &thread,
            superstep,
            base + chrono::Duration::seconds(superstep as i64),
        );
        port.save(&wp).await.unwrap();
        saved.push(wp);
    }

    let keep_ids = vec![saved[3].waypoint_id, saved[4].waypoint_id];
    let first = port.prune_thread(&thread, &keep_ids).await.unwrap();
    assert_eq!(first, 3);

    let second = port.prune_thread(&thread, &keep_ids).await.unwrap();
    assert_eq!(second, 0);

    let history = port.history(&thread, None, None).await.unwrap();
    let ids: Vec<WaypointId> = history.iter().map(|s| s.waypoint_id).collect();
    assert_eq!(ids, vec![saved[4].waypoint_id, saved[3].waypoint_id]);
}

/// Convergence: after a `prune_thread` that leaves a superset of the
/// keep-set (simulate by first pruning to a larger keep-set, then pruning
/// again to the target), a second run with the target keep-set reaches
/// exactly the target.
pub async fn prune_thread_converges_from_superset_to_target(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-prune-converges").unwrap();
    let base = Utc::now();
    let mut saved = Vec::new();
    for superstep in 0..6u64 {
        let wp = sample_waypoint_at(
            &thread,
            superstep,
            base + chrono::Duration::seconds(superstep as i64),
        );
        port.save(&wp).await.unwrap();
        saved.push(wp);
    }

    // First prune to a superset of the eventual target.
    let superset_keep: Vec<WaypointId> = saved[2..6].iter().map(|wp| wp.waypoint_id).collect();
    port.prune_thread(&thread, &superset_keep).await.unwrap();

    // Now prune to the actual target, a subset of what survived.
    let target_keep: Vec<WaypointId> = saved[4..6].iter().map(|wp| wp.waypoint_id).collect();
    let removed = port.prune_thread(&thread, &target_keep).await.unwrap();
    assert_eq!(removed, 2);

    let history = port.history(&thread, None, None).await.unwrap();
    let ids: Vec<WaypointId> = history.iter().map(|s| s.waypoint_id).collect();
    assert_eq!(ids, vec![saved[5].waypoint_id, saved[4].waypoint_id]);
}

/// Large keep-set: a thread holding 1,200 Waypoints pruned to a keep-set of
/// 1,100 ids leaves exactly those 1,100. This is the parameter-limit guard
/// -- it lives in the shared suite so every backend proves it, not in a
/// backend-specific test.
pub async fn prune_thread_large_keep_set_1200_to_1100(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-prune-large-keep-set").unwrap();
    let base = Utc::now();
    let mut saved = Vec::with_capacity(1200);
    for superstep in 0..1200u64 {
        let wp = sample_waypoint_at(
            &thread,
            superstep,
            base + chrono::Duration::seconds(superstep as i64),
        );
        port.save(&wp).await.unwrap();
        saved.push(wp);
    }

    // Keep the 1,100 newest (supersteps 100..1200).
    let keep_ids: Vec<WaypointId> = saved[100..1200].iter().map(|wp| wp.waypoint_id).collect();
    let removed = port.prune_thread(&thread, &keep_ids).await.unwrap();
    assert_eq!(removed, 100);

    let history = port.history(&thread, None, None).await.unwrap();
    assert_eq!(history.len(), 1100);
    for kept in &saved[100..1200] {
        assert!(
            port.get(&thread, &kept.waypoint_id)
                .await
                .unwrap()
                .is_some()
        );
    }
}

// ── BUG-04 / ENG-FR-12a: the FrontierSnapshot round-trips ────────────────

/// A `FrontierSnapshot` fixture carrying two resolved edges (one fired, one
/// not) and a non-empty `last_executed`, shared by both frontier contract
/// clauses so every backend exercises byte-identical inputs, exactly as
/// [`sample_waypoint`] does today.
fn frontier_fixture() -> FrontierSnapshot {
    FrontierSnapshot {
        edges: vec![
            FrontierEdgeState {
                from: NodeId::new("a"),
                to: NodeId::new("b"),
                condition: canonical_edge_condition(&None),
                fired: true,
                resolved_at: 2,
            },
            FrontierEdgeState {
                from: NodeId::new("b"),
                to: NodeId::new("c"),
                condition: canonical_edge_condition(&None),
                fired: false,
                resolved_at: 3,
            },
        ],
        last_executed: BTreeMap::from([(NodeId::new("a"), 2), (NodeId::new("b"), 3)]),
    }
}

/// `frontier` survives `save` -> `latest` -> `get`, byte-identical after a
/// serde round trip (ENG-FR-12a): a `FrontierSnapshot` carrying at least one
/// `fired: true` and one `fired: false` entry with distinct `resolved_at`
/// values, plus a non-empty `last_executed`, round-trips exactly through
/// `save`/`latest`/`get`, and `history` still returns the matching summary
/// for that waypoint id (`history` carries no payload, so `get` is the
/// payload-bearing third call D-23 asks for).
pub async fn frontier_survives_save_latest_and_get_round_trip(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-frontier-round-trip").unwrap();
    let mut wp = sample_waypoint(&thread, 0);
    wp.frontier = frontier_fixture();
    port.save(&wp).await.unwrap();

    let expected_json = serde_json::to_string(&wp).unwrap();

    let latest = port.latest(&thread).await.unwrap().unwrap();
    let latest_json = serde_json::to_string(&latest).unwrap();
    assert_eq!(latest_json, expected_json);
    assert_eq!(latest.frontier, wp.frontier);

    let fetched = port.get(&thread, &wp.waypoint_id).await.unwrap().unwrap();
    let fetched_json = serde_json::to_string(&fetched).unwrap();
    assert_eq!(fetched_json, expected_json);
    assert_eq!(fetched.frontier, wp.frontier);

    let history = port.history(&thread, None, None).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].waypoint_id, wp.waypoint_id);
}

/// A `Waypoint` payload written before `frontier` existed (D-23) still
/// loads, with an empty snapshot: serialize a `Waypoint`, remove the
/// `frontier` key entirely (simulating a pre-BUG-04 payload), deserialize
/// back, confirm `#[serde(default)]` produced an empty `FrontierSnapshot`,
/// then round-trip that value through the backend and confirm `latest`
/// returns it unchanged with the same empty snapshot -- proving both the
/// deserialization default AND the backend's write/read path handle a value
/// that originated from a pre-BUG-04 payload. Backend-agnostic by
/// construction, so all three backends run the identical assertion.
pub async fn pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot(
    port: &dyn WaypointPort,
) {
    let thread = ThreadId::new("contract-frontier-pre-bug-04-payload").unwrap();
    let wp = sample_waypoint(&thread, 0);

    let mut value = serde_json::to_value(&wp).unwrap();
    value
        .as_object_mut()
        .expect("Waypoint serializes to a JSON object")
        .remove("frontier");
    let restored: Waypoint = serde_json::from_value(value).unwrap();
    assert_eq!(restored.frontier, FrontierSnapshot::default());

    port.save(&restored).await.unwrap();
    let loaded = port.latest(&thread).await.unwrap().unwrap();
    assert_eq!(loaded.frontier, FrontierSnapshot::default());
    assert_eq!(loaded, restored);
}

// ── CF-FR-12 / D-14: MusterProgress round-trips ───────────────────────────

/// A fully populated `MusterProgress` fixture: five tasks (`a`..`e`), two
/// completed with distinct, non-trivial deltas (`a`, `c`) and three still
/// pending -- shared by both `MusterProgress` contract clauses so every
/// backend exercises byte-identical inputs, matching [`frontier_fixture`]'s
/// precedent.
fn muster_progress_fixture() -> MusterProgress {
    let worker = NodeId::new("worker");
    let tasks = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|key| MusterTask {
            worker: worker.clone(),
            payload: serde_json::json!({ "task_key": key }),
            task_key: key.to_string(),
        })
        .collect();

    let mut completed = BTreeMap::new();
    let mut delta_a = paladin_core::platform::container::battlefield::StateDelta::new();
    delta_a.set_raw(
        FieldName::new("result").unwrap(),
        serde_json::json!("result-a"),
    );
    completed.insert("a".to_string(), delta_a);
    let mut delta_c = paladin_core::platform::container::battlefield::StateDelta::new();
    delta_c.set_raw(
        FieldName::new("result").unwrap(),
        serde_json::json!("result-c"),
    );
    completed.insert("c".to_string(), delta_c);

    MusterProgress {
        node: NodeId::new("planner"),
        tasks,
        completed,
    }
}

/// A `Waypoint` whose `muster_progress` is `Some` with a fully populated
/// [`muster_progress_fixture`] (two completed tasks with distinct keys and
/// non-trivial deltas, plus a pending task) round-trips through
/// `save` -> `latest` -> `get`, byte-identical after a serde round trip and
/// equal field-for-field (CF-FR-12, D-14).
pub async fn muster_progress_round_trips(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-muster-progress-round-trip").unwrap();
    let mut wp = sample_waypoint(&thread, 2);
    wp.status = WaypointStatus::Running;
    wp.muster_progress = Some(muster_progress_fixture());
    port.save(&wp).await.unwrap();

    let expected_json = serde_json::to_string(&wp).unwrap();

    let latest = port.latest(&thread).await.unwrap().unwrap();
    let latest_json = serde_json::to_string(&latest).unwrap();
    assert_eq!(latest_json, expected_json);
    assert_eq!(latest.muster_progress, wp.muster_progress);

    let fetched = port.get(&thread, &wp.waypoint_id).await.unwrap().unwrap();
    let fetched_json = serde_json::to_string(&fetched).unwrap();
    assert_eq!(fetched_json, expected_json);
    assert_eq!(fetched.muster_progress, wp.muster_progress);

    let history = port.history(&thread, None, None).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].waypoint_id, wp.waypoint_id);
}

/// A `Waypoint` whose `muster_progress` is `None` (the ordinary,
/// non-muster case, unchanged by this field's addition) round-trips as
/// `None` (CF-FR-12, D-14) -- the additive-field precedent
/// [`pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot`]
/// established for `frontier`, applied here to `muster_progress`.
pub async fn muster_progress_none_round_trips_as_none(port: &dyn WaypointPort) {
    let thread = ThreadId::new("contract-muster-progress-none").unwrap();
    let wp = sample_waypoint(&thread, 0);
    assert_eq!(wp.muster_progress, None);
    port.save(&wp).await.unwrap();

    let loaded = port.latest(&thread).await.unwrap().unwrap();
    assert_eq!(loaded.muster_progress, None);
    assert_eq!(loaded, wp);
}

/// Runs every contract function above against `port`.
///
/// **Requires a freshly constructed, still-empty `port`** — call this once,
/// before any other operation touches the store, as a single backend's
/// smoke aggregate. `list_threads_empty_then_three_threads_newest_activity_first`
/// runs first specifically because it is the one clause that asserts a
/// literally empty store; every other function only ever inspects
/// thread-scoped state under its own uniquely named `ThreadId`, so their
/// relative order does not matter. Prefer invoking each function from its
/// own named `#[tokio::test]` for per-clause failure diagnostics (D-09);
/// this aggregator is a convenience, not a replacement.
pub async fn run_all(port: &dyn WaypointPort) {
    list_threads_empty_then_three_threads_newest_activity_first(port).await;
    save_then_latest_returns_saved_waypoint_round_tripped(port).await;
    latest_on_unknown_thread_is_none(port).await;
    get_on_known_thread_unknown_id_is_none(port).await;
    get_on_known_thread_known_id_returns_exact_waypoint(port).await;
    history_with_no_pagination_returns_all_newest_first(port).await;
    history_limit_and_before_paginate_with_no_overlap_or_gap(port).await;
    history_limit_zero_returns_empty(port).await;
    history_on_unknown_thread_returns_empty(port).await;
    history_same_created_at_tiebreaks_by_descending_superstep_stably(port).await;
    delete_thread_removes_count_and_unknown_returns_zero(port).await;
    resave_existing_waypoint_id_upserts(port).await;
    child_lineage_survives_round_trip(port).await;
    delete_waypoint_removes_named_id_and_leaves_others(port).await;
    delete_waypoint_unknown_id_is_false_and_leaves_thread_unchanged(port).await;
    delete_waypoint_unknown_thread_is_false(port).await;
    prune_thread_keeps_named_ids_byte_identical_and_ordered(port).await;
    prune_thread_empty_keep_removes_everything(port).await;
    prune_thread_unknown_thread_returns_zero(port).await;
    prune_thread_ignores_keep_ids_not_in_thread(port).await;
    prune_thread_idempotent_second_run_removes_nothing(port).await;
    prune_thread_converges_from_superset_to_target(port).await;
    prune_thread_large_keep_set_1200_to_1100(port).await;
    frontier_survives_save_latest_and_get_round_trip(port).await;
    pre_bug_04_payload_without_frontier_loads_with_an_empty_snapshot(port).await;
    muster_progress_round_trips(port).await;
    muster_progress_none_round_trips_as_none(port).await;
}
