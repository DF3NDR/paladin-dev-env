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

use chrono::{DateTime, Utc};

use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema};
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, ThreadId, Waypoint, WaypointId, WaypointStatus,
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
    );
    port.save(&child).await.unwrap();

    let loaded_child = port
        .get(&thread, &child.waypoint_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_child.parent_waypoint_id, Some(root.waypoint_id));
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
}
