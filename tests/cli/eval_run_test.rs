//! `paladin-cli eval run` snapshot/behavior tests (D-33, plan 28-12 Task 2).
//!
//! Every scenario file these tests write targets a real `graph_doc` --
//! `crates/paladin-battalion/tests/fixtures/graph_docs/linear.json` -- via an
//! absolute path, so `paladin_eval::Scenario`'s relative-path resolution never
//! needs a fixture co-located with the temp scenario file these tests write.
//!
//! `repeat_divergence_exits_nonzero_and_names_the_seq_range` exercises
//! `eval::first_divergence` directly with a synthetically "injected" divergent
//! pair of record streams, rather than driving a genuinely nondeterministic
//! engine run: `ScenarioLlm` is a fully deterministic scripted mock by
//! construction (D-30), so there is no way to make a real engine run diverge
//! from itself without registering custom Rust graph code -- an injection
//! point `paladin-cli eval run`'s own public surface (`glob` string in,
//! rendered report out) does not expose. The divergence-DETECTION algorithm
//! itself is a pure function over two `Vec<TraceRecord>`, and that is exactly
//! what this test proves.

use std::path::PathBuf;

use paladin::application::cli::commands::eval::{first_divergence, run_eval_report};
use paladin_core::platform::container::trace::{RunFinishStatus, TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};

fn linear_graph_doc_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("crates/paladin-battalion/tests/fixtures/graph_docs/linear.json")
}

fn temp_scenario_dir(label: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("paladin-cli-eval-{label}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create temp scenario dir");
    dir
}

fn write_passing_scenario(dir: &std::path::Path, file_name: &str, case_name: &str) -> PathBuf {
    let path = dir.join(file_name);
    let graph_doc = linear_graph_doc_path();
    std::fs::write(
        &path,
        format!(
            r#"
schema_version: "1"
target:
  graph_doc: "{graph_doc}"
llm:
  global:
    - text: "s1"
    - text: "s2"
    - text: "s3"
cases:
  - name: {case_name}
    input:
      topic: "widgets"
    assertions:
      - run_status: completed
      - node_executed:
          node: "start"
          times:
            exact: 1
"#,
            graph_doc = graph_doc.display(),
            case_name = case_name,
        ),
    )
    .expect("write passing scenario fixture");
    path
}

fn write_failing_scenario(dir: &std::path::Path, file_name: &str, case_name: &str) -> PathBuf {
    let path = dir.join(file_name);
    let graph_doc = linear_graph_doc_path();
    std::fs::write(
        &path,
        format!(
            r#"
schema_version: "1"
target:
  graph_doc: "{graph_doc}"
llm:
  global:
    - text: "s1"
    - text: "s2"
    - text: "s3"
cases:
  - name: {case_name}
    input:
      topic: "widgets"
    assertions:
      - node_executed:
          node: "start"
          times:
            exact: 5
"#,
            graph_doc = graph_doc.display(),
            case_name = case_name,
        ),
    )
    .expect("write failing scenario fixture");
    path
}

#[tokio::test]
async fn eval_run_reports_per_case_results() {
    let dir = temp_scenario_dir("per-case");
    write_passing_scenario(&dir, "alpha.eval.yaml", "case_a");
    write_passing_scenario(&dir, "beta.eval.yaml", "case_b");

    let pattern = dir.join("*.eval.yaml");
    let report = run_eval_report(
        pattern.to_str().expect("valid utf8 path").to_string(),
        None,
        false,
        false,
        None,
    )
    .await
    .expect("run_eval_report succeeds");

    assert_eq!(report.total_cases, 2);
    assert_eq!(report.passed_cases, 2);
    assert!(report.exit_ok);
    let rendered = report.render();
    assert!(rendered.contains("PASS"), "rendered: {rendered:?}");
    assert!(
        rendered.contains("2/2 cases passed"),
        "rendered: {rendered:?}"
    );
    // Pipe-friendly, no colour codes (matches every other command's output).
    assert!(
        !rendered.contains('\u{1b}'),
        "no ANSI escape codes: {rendered:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn eval_run_exits_nonzero_on_failure() {
    let dir = temp_scenario_dir("failure");
    write_failing_scenario(&dir, "fails.eval.yaml", "always_fails");

    let pattern = dir.join("*.eval.yaml");
    let report = run_eval_report(
        pattern.to_str().expect("valid utf8 path").to_string(),
        None,
        false,
        false,
        None,
    )
    .await
    .expect("run_eval_report succeeds even when a case fails");

    assert!(!report.exit_ok);
    let rendered = report.render();
    assert!(rendered.contains("FAIL"), "rendered: {rendered:?}");
    assert!(
        rendered.contains("expected: exactly 5"),
        "the rendered failure must be the assertion's own render_failure() output: {rendered:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn repeat_twenty_is_twenty_of_twenty() {
    let dir = temp_scenario_dir("repeat-pass");
    write_passing_scenario(&dir, "stable.eval.yaml", "deterministic");

    let pattern = dir.join("*.eval.yaml");
    let report = run_eval_report(
        pattern.to_str().expect("valid utf8 path").to_string(),
        Some(20),
        false,
        false,
        None,
    )
    .await
    .expect("run_eval_report succeeds");

    assert!(report.exit_ok, "rendered: {}", report.render());
    assert!(
        report.render().contains("20/20 passed"),
        "rendered: {}",
        report.render()
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// See this module's own top-level doc comment for why this exercises
/// `first_divergence` directly with an injected divergent pair rather than a
/// genuinely nondeterministic engine run.
#[test]
fn repeat_divergence_exits_nonzero_and_names_the_seq_range() {
    fn record(seq: u64, event: TraceEvent) -> TraceRecord {
        TraceRecord {
            thread_id: ThreadId::new("divergence-test").expect("valid thread id"),
            run_id: None,
            seq,
            at: chrono::Utc::now(),
            event,
        }
    }

    fn node_started(seq: u64, node: &str) -> TraceRecord {
        record(
            seq,
            TraceEvent::NodeStarted {
                superstep: 1,
                node_id: NodeId::new(node),
                attempt: 1,
                muster_task_key: None,
            },
        )
    }

    fn run_finished(seq: u64) -> TraceRecord {
        record(
            seq,
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 1,
                total_tokens: 0,
                duration_ms: 1,
                trace_dropped_total: 0,
            },
        )
    }

    // Every "run" agrees through seq 1 (`node_started` on the same node),
    // then run 2 (index 2) takes a DIFFERENT node at seq 2 -- the injected
    // divergence -- before both converge on an identical RunFinished at
    // seq 3.
    let run_0 = vec![node_started(1, "a"), node_started(2, "b"), run_finished(3)];
    let run_1 = vec![node_started(1, "a"), node_started(2, "b"), run_finished(3)];
    let run_2_diverged = vec![
        node_started(1, "a"),
        node_started(2, "c"), // <-- injected divergence at seq 2
        run_finished(3),
    ];

    let divergence = first_divergence(&[run_0, run_1, run_2_diverged]);
    let (run_index, seq_from, seq_to) =
        divergence.expect("a genuinely divergent run must be reported, not silently averaged away");
    assert_eq!(run_index, 2, "run 2 is the one that diverged from run 0");
    assert_eq!(seq_from, 2, "the divergence first appears at seq 2");
    assert_eq!(
        seq_to, 3,
        "the reported range spans through the run's own final seq"
    );

    // Sanity: three byte-for-byte-identical (modulo wall-clock/id noise)
    // runs report no divergence at all.
    let identical_a = vec![node_started(1, "a"), run_finished(2)];
    let identical_b = vec![node_started(1, "a"), run_finished(2)];
    assert!(first_divergence(&[identical_a, identical_b]).is_none());
}

#[tokio::test]
async fn bless_writes_a_snapshot_beside_the_scenario() {
    let dir = temp_scenario_dir("bless");
    let graph_doc = linear_graph_doc_path();
    let scenario_path = dir.join("snap.eval.yaml");
    std::fs::write(
        &scenario_path,
        format!(
            r#"
schema_version: "1"
target:
  graph_doc: "{graph_doc}"
llm:
  global:
    - text: "s1"
    - text: "s2"
    - text: "s3"
cases:
  - name: snapshot_case
    input:
      topic: "widgets"
    assertions:
      - final_state_snapshot
"#,
            graph_doc = graph_doc.display(),
        ),
    )
    .expect("write bless scenario fixture");

    let pattern = dir.join("*.eval.yaml");
    // `Path::file_stem` strips only the LAST extension, so
    // `snap.eval.yaml`'s own stem is `snap.eval`, not `snap`.
    let snapshot_path = dir.join("snap.eval.snapshot_case.snap.json");
    assert!(!snapshot_path.exists(), "sanity: no pre-existing snapshot");

    // --bless: writes the blessed file and passes against it in the same run.
    let blessed_report = run_eval_report(
        pattern.to_str().expect("valid utf8 path").to_string(),
        None,
        true,
        false,
        None,
    )
    .await
    .expect("run_eval_report succeeds");
    assert!(
        blessed_report.exit_ok,
        "rendered: {}",
        blessed_report.render()
    );
    assert!(
        snapshot_path.exists(),
        "expected {snapshot_path:?} to exist beside the scenario after --bless"
    );

    // A following run without --bless passes against the just-written file.
    let unblessed_report = run_eval_report(
        pattern.to_str().expect("valid utf8 path").to_string(),
        None,
        false,
        false,
        None,
    )
    .await
    .expect("run_eval_report succeeds");
    assert!(
        unblessed_report.exit_ok,
        "rendered: {}",
        unblessed_report.render()
    );

    // Deleting the snapshot file makes the same run fail -- never an
    // implicit pass.
    std::fs::remove_file(&snapshot_path).expect("remove blessed snapshot");
    let missing_report = run_eval_report(
        pattern.to_str().expect("valid utf8 path").to_string(),
        None,
        false,
        false,
        None,
    )
    .await
    .expect("run_eval_report succeeds");
    assert!(
        !missing_report.exit_ok,
        "rendered: {}",
        missing_report.render()
    );
    assert!(
        missing_report.render().contains("--bless"),
        "a missing snapshot's failure must name --bless: {}",
        missing_report.render()
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn unresolvable_registered_target_is_a_clear_error() {
    let dir = temp_scenario_dir("unresolvable");
    let scenario_path = dir.join("ghost.eval.yaml");
    std::fs::write(
        &scenario_path,
        r#"
schema_version: "1"
target:
  registered: "totally-unregistered-name"
cases:
  - name: never_runs
    assertions: []
"#,
    )
    .expect("write unresolvable-target scenario fixture");

    let pattern = dir.join("*.eval.yaml");
    let report = run_eval_report(
        pattern.to_str().expect("valid utf8 path").to_string(),
        None,
        false,
        false,
        None,
    )
    .await
    .expect("run_eval_report succeeds even when a target cannot be resolved");

    assert!(!report.exit_ok);
    let rendered = report.render();
    assert!(
        rendered.contains("totally-unregistered-name"),
        "the error must name the unresolved target: {rendered:?}"
    );
    assert!(
        rendered.contains("registered:"),
        "the error must list the (empty) registered set: {rendered:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
