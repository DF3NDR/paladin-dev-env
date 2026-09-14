//! `paladin-cli run export` snapshot/behavior tests (D-21, D-22, D-23, plan
//! 28-13 Task 2).
//!
//! Every test seeds a REAL `SqliteWaypointStore`/`SqliteRunTraceStore`/
//! `SqliteRunRepository`/`SqliteAssistantRepository` (never a mock) and
//! points `render_run_export`'s port resolution at it via the SAME
//! `APP_RUN_STORE_*`/`APP_WAYPOINT_STORE_*` env vars `paladin-server` reads
//! (ADR-0023). Every test is `#[serial_test::serial(paladin_cli_run_store_env)]`
//! -- shared with `tests/cli/graph_export_test.rs`, which mutates the
//! identical process-global env vars -- so no two tests in this test binary
//! race each other's backend configuration.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::Utc;
use paladin::application::cli::commands::run::{GraphResolution, render_run_export};
use paladin_battalion::engine::export::OverlaySource;
use paladin_core::platform::container::assistant::{
    AssistantDefinition, AssistantId, AssistantKind, NewAssistantVersion,
};
use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema};
use paladin_core::platform::container::run::{AssistantRef, Run, RunId};
use paladin_core::platform::container::trace::{TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::{
    FrontierSnapshot, GraphFingerprint, NodeExecutionRecord, NodeId, NodeOutcomeKind, ThreadId,
    Waypoint, WaypointStatus,
};
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::waypoint_port::WaypointPort;

fn linear_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("crates/paladin-battalion/tests/fixtures/graph_docs/linear.json")
}

fn new_thread(label: &str) -> ThreadId {
    ThreadId::new(format!("run-export-{label}-{}", uuid::Uuid::new_v4())).expect("valid thread id")
}

fn waypoint(
    thread: &ThreadId,
    superstep: u64,
    vanguard: Vec<&str>,
    completed: Vec<NodeExecutionRecord>,
) -> Waypoint {
    Waypoint::new_root(
        thread.clone(),
        superstep,
        GraphFingerprint::from_canonical_bytes(b"run-export-test-graph"),
        Battlefield::new(BattlefieldSchema::new(Vec::new())),
        vanguard.into_iter().map(NodeId::new).collect(),
        completed,
        WaypointStatus::Running,
        BTreeMap::new(),
        FrontierSnapshot::default(),
    )
}

fn node_record(node_id: &str, attempt: u32, outcome: NodeOutcomeKind) -> NodeExecutionRecord {
    NodeExecutionRecord {
        node_id: NodeId::new(node_id),
        paladin_id: None,
        started_at: Utc::now(),
        duration_ms: 10,
        token_count: 5,
        outcome,
        attempt,
        attempts: Vec::new(),
        cache_hit: false,
    }
}

fn trace_record(thread: &ThreadId, seq: u64, event: TraceEvent) -> TraceRecord {
    TraceRecord {
        thread_id: thread.clone(),
        run_id: None,
        seq,
        at: Utc::now(),
        event,
    }
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "paladin-cli-run-export-{label}-{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    let url = format!("sqlite://{}", path.display());
    (path, url)
}

fn cleanup_sqlite(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

/// Shared with `tests/cli/graph_export_test.rs`: both files mutate the SAME
/// process-global `APP_RUN_STORE_*` env vars.
fn set_run_store_env(url: &str) {
    unsafe {
        std::env::set_var("APP_RUN_STORE_BACKEND", "sqlite");
        std::env::set_var("APP_RUN_STORE_PATH", url);
    }
}

fn clear_run_store_env() {
    unsafe {
        std::env::remove_var("APP_RUN_STORE_BACKEND");
        std::env::remove_var("APP_RUN_STORE_PATH");
    }
}

fn set_waypoint_store_env(url: &str) {
    unsafe {
        std::env::set_var("APP_WAYPOINT_STORE_BACKEND", "sqlite");
        std::env::set_var("APP_WAYPOINT_STORE_SQLITE_PATH", url);
    }
}

fn clear_waypoint_store_env() {
    unsafe {
        std::env::remove_var("APP_WAYPOINT_STORE_BACKEND");
        std::env::remove_var("APP_WAYPOINT_STORE_SQLITE_PATH");
    }
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn run_export_thread_with_waypoints() {
    clear_run_store_env();
    let (wp_path, wp_url) = temp_sqlite_url("waypoints-only");
    set_waypoint_store_env(&wp_url);

    let store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
        .await
        .expect("open sqlite waypoint store");
    let thread = new_thread("waypoints-only");
    store
        .save(&waypoint(
            &thread,
            1,
            vec!["middle"],
            vec![node_record("start", 1, NodeOutcomeKind::Succeeded)],
        ))
        .await
        .expect("save waypoint 1");
    store
        .save(&waypoint(
            &thread,
            2,
            vec![],
            vec![node_record("middle", 1, NodeOutcomeKind::Succeeded)],
        ))
        .await
        .expect("save waypoint 2");

    let report = render_run_export(Some(thread), None, None, None)
        .await
        .expect("render succeeds");
    assert_eq!(report.overlay_source, OverlaySource::Waypoints);
    assert_eq!(report.resolution, GraphResolution::ObservedOnly);
    let rendered = report.render();
    assert!(
        rendered.contains("waypoints"),
        "the output must note the source: {rendered}"
    );
    assert!(rendered.contains("start") && rendered.contains("middle"));
    insta::assert_snapshot!("run_export_thread_with_waypoints", rendered);

    clear_waypoint_store_env();
    cleanup_sqlite(&wp_path);
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn run_export_prefers_trace_rows_when_present() {
    let (run_path, run_url) = temp_sqlite_url("trace");
    let (wp_path, wp_url) = temp_sqlite_url("trace-waypoints");
    set_run_store_env(&run_url);
    set_waypoint_store_env(&wp_url);

    let trace_store = paladin_storage::run_trace::sqlite::SqliteRunTraceStore::new(&run_url)
        .await
        .expect("open sqlite trace store");
    let waypoint_store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
        .await
        .expect("open sqlite waypoint store");

    let thread = new_thread("trace-preferred");

    // Waypoint history exists too, proving trace WINS rather than merely
    // being the only available source.
    waypoint_store
        .save(&waypoint(
            &thread,
            1,
            vec![],
            vec![node_record("check", 1, NodeOutcomeKind::Succeeded)],
        ))
        .await
        .expect("save waypoint");

    let records = vec![
        trace_record(
            &thread,
            1,
            TraceEvent::NodeFinished {
                superstep: 1,
                node_id: NodeId::new("check"),
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 10,
                token_count: 5,
                cache_hit: false,
            },
        ),
        trace_record(
            &thread,
            2,
            TraceEvent::EdgeEvaluated {
                from: NodeId::new("check"),
                to: NodeId::new("retry"),
                condition_kind: "contains".to_string(),
                fired: true,
            },
        ),
        trace_record(
            &thread,
            3,
            TraceEvent::EdgeEvaluated {
                from: NodeId::new("check"),
                to: NodeId::new("done"),
                condition_kind: "contains".to_string(),
                fired: false,
            },
        ),
    ];
    trace_store
        .append(&records)
        .await
        .expect("append trace records");

    let report = render_run_export(Some(thread), None, None, None)
        .await
        .expect("render succeeds");
    assert_eq!(report.overlay_source, OverlaySource::Trace);
    let rendered = report.render();
    assert!(
        rendered.contains("-.->"),
        "evaluated-but-not-fired must render dotted: {rendered}"
    );
    insta::assert_snapshot!("run_export_prefers_trace_rows_when_present", rendered);

    clear_run_store_env();
    clear_waypoint_store_env();
    cleanup_sqlite(&run_path);
    cleanup_sqlite(&wp_path);
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn run_export_graph_resolution_order() {
    // Bucket 1: an explicit --graph wins outright.
    {
        clear_run_store_env();
        let (wp_path, wp_url) = temp_sqlite_url("resolution-explicit");
        set_waypoint_store_env(&wp_url);
        let store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
            .await
            .expect("open sqlite waypoint store");
        let thread = new_thread("resolution-explicit");
        store
            .save(&waypoint(
                &thread,
                1,
                vec![],
                vec![node_record("start", 1, NodeOutcomeKind::Succeeded)],
            ))
            .await
            .expect("save waypoint");

        let path = linear_fixture_path();
        let report = render_run_export(Some(thread), None, None, Some(path.clone()))
            .await
            .expect("render succeeds");
        assert_eq!(report.resolution, GraphResolution::Explicit(path));

        clear_waypoint_store_env();
        cleanup_sqlite(&wp_path);
    }

    // Bucket 2: no --graph, but --run resolves the assistant version.
    {
        let (run_path, run_url) = temp_sqlite_url("resolution-run");
        let (wp_path, wp_url) = temp_sqlite_url("resolution-run-waypoints");
        set_run_store_env(&run_url);
        set_waypoint_store_env(&wp_url);

        let assistant_store =
            paladin_storage::assistant::sqlite::SqliteAssistantRepository::new(&run_url)
                .await
                .expect("open sqlite assistant store");
        let run_store = paladin_storage::run::sqlite::SqliteRunRepository::new(&run_url)
            .await
            .expect("open sqlite run store");
        let waypoint_store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
            .await
            .expect("open sqlite waypoint store");

        let assistant_id = AssistantId::new("resolution-assistant").expect("valid id");
        let doc_contents = std::fs::read_to_string(linear_fixture_path()).expect("read fixture");
        let doc_value: serde_json::Value =
            serde_json::from_str(&doc_contents).expect("parse fixture as JSON");
        assistant_store
            .create(
                &assistant_id,
                NewAssistantVersion {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Workflow,
                        body: doc_value,
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .expect("create assistant version");

        let thread = new_thread("resolution-run");
        let run_id = RunId::new_v7();
        let run = Run::new(
            run_id.clone(),
            thread.clone(),
            AssistantRef {
                assistant_id: "resolution-assistant".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        );
        run_store.insert(&run).await.expect("insert run row");
        waypoint_store
            .save(&waypoint(
                &thread,
                1,
                vec![],
                vec![node_record("start", 1, NodeOutcomeKind::Succeeded)],
            ))
            .await
            .expect("save waypoint");

        let report = render_run_export(None, None, Some(run_id), None)
            .await
            .expect("render succeeds");
        assert_eq!(
            report.resolution,
            GraphResolution::AssistantVersion {
                assistant_id: "resolution-assistant".to_string(),
                version: 1,
            }
        );

        clear_run_store_env();
        clear_waypoint_store_env();
        cleanup_sqlite(&run_path);
        cleanup_sqlite(&wp_path);
    }

    // Bucket 3: neither resolvable -- observed-only, with the locked title.
    {
        clear_run_store_env();
        let (wp_path, wp_url) = temp_sqlite_url("resolution-observed");
        set_waypoint_store_env(&wp_url);
        let store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
            .await
            .expect("open sqlite waypoint store");
        let thread = new_thread("resolution-observed");
        store
            .save(&waypoint(
                &thread,
                1,
                vec![],
                vec![node_record("start", 1, NodeOutcomeKind::Succeeded)],
            ))
            .await
            .expect("save waypoint");

        let report = render_run_export(Some(thread), None, None, None)
            .await
            .expect("render succeeds");
        assert_eq!(report.resolution, GraphResolution::ObservedOnly);
        assert!(
            report
                .diagram
                .contains("observed nodes only — no graph document available"),
            "the diagram must carry the locked observed-only title: {}",
            report.diagram
        );

        clear_waypoint_store_env();
        cleanup_sqlite(&wp_path);
    }
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn run_export_run_flag_resolves_thread_and_graph() {
    let (run_path, run_url) = temp_sqlite_url("run-flag");
    let (wp_path, wp_url) = temp_sqlite_url("run-flag-waypoints");
    set_run_store_env(&run_url);
    set_waypoint_store_env(&wp_url);

    let assistant_store =
        paladin_storage::assistant::sqlite::SqliteAssistantRepository::new(&run_url)
            .await
            .expect("open sqlite assistant store");
    let run_store = paladin_storage::run::sqlite::SqliteRunRepository::new(&run_url)
        .await
        .expect("open sqlite run store");
    let waypoint_store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
        .await
        .expect("open sqlite waypoint store");

    let assistant_id = AssistantId::new("run-flag-assistant").expect("valid id");
    let doc_contents = std::fs::read_to_string(linear_fixture_path()).expect("read fixture");
    let doc_value: serde_json::Value =
        serde_json::from_str(&doc_contents).expect("parse fixture as JSON");
    assistant_store
        .create(
            &assistant_id,
            NewAssistantVersion {
                definition: AssistantDefinition {
                    kind: AssistantKind::Workflow,
                    body: doc_value,
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .expect("create assistant version");

    let thread = new_thread("run-flag-derived-thread");
    let run_id = RunId::new_v7();
    let run = Run::new(
        run_id.clone(),
        thread.clone(),
        AssistantRef {
            assistant_id: "run-flag-assistant".to_string(),
            version: 1,
        },
        serde_json::json!({}),
    );
    run_store.insert(&run).await.expect("insert run row");
    // Seeded ONLY under the run-derived thread's own id, marked `Failed` (a
    // distinctive `outcomeFailed` class, unlike every other test's
    // `Succeeded` records) -- if `--run` failed to derive the thread, this
    // Waypoint would never be found (the export would error "no history"
    // instead of succeeding), and if the wrong node id were visited this
    // specific class assertion would not hold.
    waypoint_store
        .save(&waypoint(
            &thread,
            1,
            vec!["middle"],
            vec![node_record("start", 1, NodeOutcomeKind::Failed)],
        ))
        .await
        .expect("save waypoint");

    let report = render_run_export(None, None, Some(run_id), None)
        .await
        .expect("--run alone must resolve both the thread and the graph");
    assert_eq!(
        report.resolution,
        GraphResolution::AssistantVersion {
            assistant_id: "run-flag-assistant".to_string(),
            version: 1,
        }
    );
    assert_eq!(report.overlay_source, OverlaySource::Waypoints);
    assert!(
        report.diagram.contains("outcomeFailed"),
        "the run-derived thread's own seeded Waypoint (node 'start', Failed) must be the one \
         rendered: {}",
        report.diagram
    );

    clear_run_store_env();
    clear_waypoint_store_env();
    cleanup_sqlite(&run_path);
    cleanup_sqlite(&wp_path);
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn run_export_unknown_thread_is_an_error() {
    clear_run_store_env();
    let (wp_path, wp_url) = temp_sqlite_url("unknown-thread");
    set_waypoint_store_env(&wp_url);
    // Open (and thereby migrate) the store, but seed nothing for this
    // thread -- exactly what an unknown thread id looks like to the store.
    let _store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
        .await
        .expect("open sqlite waypoint store");

    let unknown = ThreadId::new("totally-unknown-thread").expect("valid thread id");
    let err = render_run_export(Some(unknown.clone()), None, None, None)
        .await
        .expect_err("an unknown thread must error, never render an empty diagram");
    assert!(
        err.to_string().contains("totally-unknown-thread"),
        "the error must name the unknown thread: {err}"
    );

    clear_waypoint_store_env();
    cleanup_sqlite(&wp_path);
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn run_export_thread_with_no_history_is_a_clear_message() {
    clear_run_store_env();
    let (wp_path, wp_url) = temp_sqlite_url("no-history");
    set_waypoint_store_env(&wp_url);
    let _store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
        .await
        .expect("open sqlite waypoint store");

    let thread = new_thread("no-history");
    let err = render_run_export(Some(thread), None, None, None)
        .await
        .expect_err("a thread with no Waypoints and no trace must produce a clear message");
    let message = err.to_string();
    assert!(
        message.contains("no Waypoint history") && message.contains("no persisted trace"),
        "the message must be clear, not an empty diagram: {message}"
    );

    clear_waypoint_store_env();
    cleanup_sqlite(&wp_path);
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn run_export_waypoint_flag_limits_history() {
    clear_run_store_env();
    let (wp_path, wp_url) = temp_sqlite_url("waypoint-cap");
    set_waypoint_store_env(&wp_url);
    let store = paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&wp_url)
        .await
        .expect("open sqlite waypoint store");

    let thread = new_thread("waypoint-cap");
    let wp1 = waypoint(
        &thread,
        1,
        vec!["middle"],
        vec![node_record("start", 1, NodeOutcomeKind::Succeeded)],
    );
    let cap_id = wp1.waypoint_id;
    store.save(&wp1).await.expect("save waypoint 1");
    store
        .save(&waypoint(
            &thread,
            2,
            vec!["late"],
            vec![node_record("middle", 1, NodeOutcomeKind::Succeeded)],
        ))
        .await
        .expect("save waypoint 2");
    store
        .save(&waypoint(
            &thread,
            3,
            vec![],
            vec![node_record("late", 1, NodeOutcomeKind::Succeeded)],
        ))
        .await
        .expect("save waypoint 3");

    // Capped at waypoint 1: only "start" was visited as of that Waypoint.
    let capped_report = render_run_export(Some(thread.clone()), Some(cap_id), None, None)
        .await
        .expect("render succeeds");
    assert!(capped_report.diagram.contains("start"));
    assert!(
        !capped_report.diagram.contains("middle") && !capped_report.diagram.contains("late"),
        "the capped overlay must not include later visits: {}",
        capped_report.diagram
    );

    // The whole history (no cap) includes every visit.
    let full_report = render_run_export(Some(thread), None, None, None)
        .await
        .expect("render succeeds");
    assert!(full_report.diagram.contains("start"));
    assert!(full_report.diagram.contains("middle"));
    assert!(full_report.diagram.contains("late"));

    clear_waypoint_store_env();
    cleanup_sqlite(&wp_path);
}
