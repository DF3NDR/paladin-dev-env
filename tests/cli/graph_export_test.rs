//! `paladin-cli graph export` snapshot/behavior tests (D-23, plan 28-13 Task 1).
//!
//! `graph_export_file_to_mermaid`/`graph_export_file_to_dot` byte-compare
//! against the committed golden files under
//! `crates/paladin-battalion/tests/golden/export/` (28-07/D-20) -- proving
//! `render_graph_export` produces EXACTLY what `to_mermaid`/`to_dot` produce
//! for the resolved shape, not merely "close enough".
//!
//! `graph_export_assistant_resolves_through_the_store` seeds a real
//! `SqliteAssistantRepository` (never a mock) and points `--assistant`'s
//! store resolution at it via the SAME `APP_RUN_STORE_*` env vars
//! `paladin-server` reads (ADR-0023) -- serialized with
//! `#[serial_test::serial]` under the shared `paladin_cli_run_store_env` key
//! (shared with `tests/cli/run_export_test.rs`, which reads the identical
//! process-global env vars) so no two tests race each other's backend
//! configuration.

use std::path::PathBuf;

use paladin::application::cli::commands::graph::{
    ExportFormat, render_graph_export, run_graph_export,
};
use paladin_battalion::engine::graph_doc::WarGraphDoc;
use paladin_core::platform::container::assistant::{
    AssistantDefinition, AssistantId, AssistantKind, NewAssistantVersion,
};
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("crates/paladin-battalion/tests/fixtures/graph_docs")
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates/paladin-battalion/tests/golden/export")
}

fn linear_fixture_path() -> PathBuf {
    fixtures_dir().join("linear.json")
}

fn read_golden(name: &str) -> String {
    std::fs::read_to_string(golden_dir().join(name))
        .unwrap_or_else(|e| panic!("read golden {name}: {e}"))
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "paladin-cli-graph-export-{label}-{}.sqlite",
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

/// Shared with `tests/cli/run_export_test.rs`: both files mutate the SAME
/// process-global `APP_RUN_STORE_*` env vars `RunStoreConfig::
/// apply_env_overrides` reads, so every test that touches them must be
/// serialized under this one key.
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

#[tokio::test]
async fn graph_export_file_to_mermaid() {
    let path = linear_fixture_path();
    let rendered = render_graph_export(ExportFormat::Mermaid, Some(path), None)
        .await
        .expect("render succeeds");
    let expected = read_golden("linear.mermaid");
    assert_eq!(
        rendered, expected,
        "rendered output must byte-match the committed golden"
    );
    insta::assert_snapshot!("graph_export_file_to_mermaid", rendered);
}

#[tokio::test]
async fn graph_export_file_to_dot() {
    let path = linear_fixture_path();
    let rendered = render_graph_export(ExportFormat::Dot, Some(path), None)
        .await
        .expect("render succeeds");
    let expected = read_golden("linear.dot");
    assert_eq!(
        rendered, expected,
        "rendered output must byte-match the committed golden"
    );
    insta::assert_snapshot!("graph_export_file_to_dot", rendered);
}

#[tokio::test]
async fn graph_export_to_out_file() {
    let path = linear_fixture_path();
    let temp_dir = std::env::temp_dir().join(format!(
        "paladin-cli-graph-export-out-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&temp_dir).expect("create temp dir");
    let out_path = temp_dir.join("linear.mermaid");

    run_graph_export(
        ExportFormat::Mermaid,
        Some(path),
        None,
        Some(out_path.clone()),
    )
    .await
    .expect("export to --out succeeds");

    let written = std::fs::read_to_string(&out_path).expect("read written diagram");
    let expected = read_golden("linear.mermaid");
    assert_eq!(
        written, expected,
        "the file written by --out must byte-match the golden"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn graph_export_unknown_format_is_an_error() {
    use clap::Parser;
    use paladin::application::cli::commands::graph::GraphExportArgs;

    #[derive(Debug, clap::Parser)]
    struct Wrapper {
        #[command(flatten)]
        args: GraphExportArgs,
    }

    let result = Wrapper::try_parse_from(["paladin-cli", "some-file.json", "--format", "bogus"]);
    let error = result.expect_err("an unrecognised --format value must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("mermaid") && message.contains("dot"),
        "the error should list the accepted values: {message}"
    );
}

#[tokio::test]
async fn graph_export_unreadable_document_is_an_error() {
    // A missing file names the path and the reason.
    let missing = PathBuf::from("/tmp/paladin-cli-graph-export-does-not-exist.json");
    let missing_err = render_graph_export(ExportFormat::Mermaid, Some(missing.clone()), None)
        .await
        .expect_err("a missing file must error");
    let missing_message = missing_err.to_string();
    assert!(
        missing_message.contains(missing.to_str().unwrap()),
        "must name the missing path: {missing_message}"
    );

    // A malformed document names the path and the parse reason, and is a
    // DISTINCT failure from the missing-file case.
    let temp_dir = std::env::temp_dir().join(format!(
        "paladin-cli-graph-export-malformed-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&temp_dir).expect("create temp dir");
    let malformed_path = temp_dir.join("malformed.json");
    std::fs::write(&malformed_path, "{ not valid json").expect("write malformed fixture");

    let malformed_err =
        render_graph_export(ExportFormat::Mermaid, Some(malformed_path.clone()), None)
            .await
            .expect_err("a malformed document must error");
    let malformed_message = malformed_err.to_string();
    assert!(
        malformed_message.contains(malformed_path.to_str().unwrap())
            || malformed_message.to_lowercase().contains("json"),
        "must name the path or the parse reason: {malformed_message}"
    );
    assert_ne!(
        missing_message, malformed_message,
        "the two failure modes must produce distinct errors"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
#[serial_test::serial(paladin_cli_run_store_env)]
async fn graph_export_assistant_resolves_through_the_store() {
    let (db_path, db_url) = temp_sqlite_url("assistant");
    set_run_store_env(&db_url);

    let store = paladin_storage::assistant::sqlite::SqliteAssistantRepository::new(&db_url)
        .await
        .expect("open sqlite assistant store");

    let assistant_id = AssistantId::new("linear-workflow").expect("valid assistant id");
    let doc_contents = std::fs::read_to_string(linear_fixture_path()).expect("read fixture");
    let doc: WarGraphDoc = serde_json::from_str(&doc_contents).expect("parse fixture");
    let body = serde_json::to_value(&doc).expect("serialize graph doc");

    store
        .create(
            &assistant_id,
            NewAssistantVersion {
                definition: AssistantDefinition {
                    kind: AssistantKind::Workflow,
                    body,
                },
                created_by: None,
                note: None,
            },
        )
        .await
        .expect("create assistant version");

    let rendered = render_graph_export(
        ExportFormat::Mermaid,
        None,
        Some("linear-workflow".to_string()),
    )
    .await
    .expect("assistant-resolved render succeeds");
    let expected = read_golden("linear.mermaid");
    assert_eq!(
        rendered, expected,
        "the assistant-resolved shape must render identically to the file-resolved one"
    );

    // An unknown assistant id is a distinct, naming error.
    let unknown_err = render_graph_export(
        ExportFormat::Mermaid,
        None,
        Some("totally-unknown-assistant".to_string()),
    )
    .await
    .expect_err("an unknown assistant id must error");
    assert!(
        unknown_err
            .to_string()
            .contains("totally-unknown-assistant"),
        "must name the unresolved assistant id: {unknown_err}"
    );

    clear_run_store_env();
    cleanup_sqlite(&db_path);
}
