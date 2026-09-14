//! `WarGraphDoc` golden-schema, fixture round-trip and two-process
//! fingerprint-stability tests (Doc 06 plan 27-05).
//!
//! An integration test target (`tests/graph_doc_round_trip.rs`), so it
//! counts toward `cargo llvm-cov`'s workspace coverage floor exactly like
//! `crates/paladin-web/tests/auth_rbac.rs` already does for that crate.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_battalion::edge_evaluator::{EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorError};
use paladin_battalion::engine::registries::EngineRegistries;
use paladin_battalion::engine::{CompileError, TypedSchema, WarGraphDoc};
use paladin_battalion::error_handler::ErrorHandler;
use paladin_battalion::retry_predicate::{RetryPredicateError, RetryPredicateEvaluator};
use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::node_error::NodeError;

/// `tests/fixtures/graph_docs/` -- the fixture corpus this file round-trips.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/graph_docs"
    ))
}

/// The checked-in golden schema, one level up from the crate at the repo's
/// `docs/schemas/` (mirroring `crates/paladin-web/openapi.rs`'s own
/// `UPDATE_OPENAPI=1` bless idiom, but for a repo-root-relative doc asset
/// rather than a crate-relative one).
fn schema_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/schemas/wargraph-doc.schema.json"
    ))
}

/// Always resolves `EdgeCondition::Custom("custom_edge")` to `true` --
/// exercising the resolution path, not the evaluator's own logic (already
/// covered by `edge_evaluator`'s own unit tests).
struct AlwaysTrueEdge;
#[async_trait]
impl EdgeConditionEvaluator for AlwaysTrueEdge {
    async fn evaluate(
        &self,
        _output: &str,
        _ctx: &EdgeContext<'_>,
    ) -> Result<bool, EdgeEvaluatorError> {
        Ok(true)
    }
}

/// Always allows a retry under `RetryPredicate::Custom("retry_pred")`.
struct AlwaysRetry;
#[async_trait]
impl RetryPredicateEvaluator for AlwaysRetry {
    async fn allows(&self, _err: &NodeError, _attempt: u32) -> Result<bool, RetryPredicateError> {
        Ok(true)
    }
}

/// Always absorbs the failure under `ErrorHandlerSpec::Custom("on_error_handler")`.
struct AlwaysAbsorb;
#[async_trait]
impl ErrorHandler for AlwaysAbsorb {
    async fn handle(&self, _err: &NodeError, _state: &Battlefield) -> Result<Directive, NodeError> {
        Ok(Directive {
            delta: StateDelta::new(),
            next: NextStep::Edges,
        })
    }
}

/// The `EngineRegistries` bundle every fixture in the corpus resolves
/// against: `custom_edge`, `retry_pred`, `on_error_handler` (the exact
/// names `two_paladins_custom_edge.json` uses) and the `answer` output
/// schema `two_paladins_custom_edge.json`'s `finalizer` node registers
/// against.
fn fixture_registries() -> EngineRegistries {
    let mut registries = EngineRegistries::new();
    registries
        .edge_evaluators
        .register("custom_edge", Arc::new(AlwaysTrueEdge));
    registries
        .retry_predicates
        .register("retry_pred", Arc::new(AlwaysRetry));
    registries
        .error_handlers
        .register("on_error_handler", Arc::new(AlwaysAbsorb));
    registries.output_schemas.insert(
        "answer".to_string(),
        Arc::new(TypedSchema::<serde_json::Value>::new(serde_json::json!({
            "type": "object"
        }))),
    );
    registries
}

/// `wargraph_doc_schema_matches_golden`: `schemars::schema_for!(WarGraphDoc)`
/// serialised as pretty JSON must equal the committed golden file
/// byte-for-byte. `UPDATE_WARGRAPH_SCHEMA=1` regenerates it.
#[test]
fn wargraph_doc_schema_matches_golden() {
    let schema_value = schemars::schema_for!(WarGraphDoc).to_value();
    let generated =
        serde_json::to_string_pretty(&schema_value).expect("serialize derived schema") + "\n";
    let path = schema_path();

    if std::env::var_os("UPDATE_WARGRAPH_SCHEMA").is_some() {
        std::fs::write(&path, &generated).expect("write golden schema");
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        generated.trim(),
        committed.trim(),
        "WarGraphDoc's derived JSON Schema drifted from {}. If intentional, regenerate with: \
         UPDATE_WARGRAPH_SCHEMA=1 cargo test -p paladin-battalion --test graph_doc_round_trip \
         wargraph_doc_schema_matches_golden",
        path.display()
    );
}

/// `fixture_corpus_round_trips`: every fixture under
/// `tests/fixtures/graph_docs/` parses, round-trips its JSON `Value`
/// byte-for-byte, compiles, and compiles again to the SAME fingerprint
/// after a re-serialise/re-parse cycle.
#[test]
fn fixture_corpus_round_trips() {
    let dir = fixtures_dir();
    let mut count = 0usize;
    let registries = fixture_registries();

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read fixtures dir {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    entries.sort();

    for path in entries {
        count += 1;
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let raw_value: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {} as Value: {e}", path.display()));
        let doc: WarGraphDoc = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {} as WarGraphDoc: {e}", path.display()));

        let round_tripped = serde_json::to_value(&doc)
            .unwrap_or_else(|e| panic!("serialize {}: {e}", path.display()));
        assert_eq!(
            round_tripped,
            raw_value,
            "{} did not round-trip its JSON Value byte-for-byte",
            path.display()
        );

        let graph_a = doc
            .compile(&registries)
            .unwrap_or_else(|e| panic!("compile {}: {e}", path.display()));
        let fingerprint_a = graph_a.fingerprint();

        // Re-serialise -> re-parse -> compile again: the fingerprint must
        // be identical, proving compilation is deterministic across a
        // round trip through the wire format.
        let value2 = serde_json::to_value(&doc).expect("serialize for second round");
        let doc2: WarGraphDoc = serde_json::from_value(value2).expect("re-parse for second round");
        let graph_b = doc2
            .compile(&registries)
            .unwrap_or_else(|e| panic!("re-compile {}: {e}", path.display()));
        let fingerprint_b = graph_b.fingerprint();

        assert_eq!(
            fingerprint_a,
            fingerprint_b,
            "{} fingerprint changed across a re-serialise/re-parse cycle",
            path.display()
        );
    }

    assert!(
        count >= 3,
        "expected at least 3 fixtures under {}, found {count}",
        dir.display()
    );
}

/// `unsupported_kind_is_typed` (named `wargraph_doc_unsupported_node_kind`
/// for 27-VALIDATION.md): a document naming an unsupported `kind` string
/// parses (v0.10's document format accepts any `kind` string at the wire
/// level) but fails `compile` with a typed `CompileError::UnsupportedNodeKind`
/// naming the rejected string -- never a silent drop, never a bare
/// `serde_json` deserialize failure.
#[test]
fn wargraph_doc_unsupported_node_kind() {
    let raw = serde_json::json!({
        "schema_version": "1",
        "entry": ["n"],
        "nodes": [
            {"id": "n", "kind": "function"}
        ],
        "edges": [],
        "schema": {"fields": []}
    });
    let doc: WarGraphDoc =
        serde_json::from_value(raw).expect("a document with an unsupported kind still parses");
    // `WarGraph` carries no `Debug` derive, so `Result::expect_err` (which
    // bounds `T: Debug`) cannot be used directly on `Result<WarGraph, _>`.
    let err = match doc.compile(&EngineRegistries::new()) {
        Ok(_) => panic!("expected a CompileError, got a compiled WarGraph"),
        Err(e) => e,
    };
    assert!(matches!(
        err,
        CompileError::UnsupportedNodeKind { kind } if kind == "function"
    ));
}

/// `wargraph_doc_fingerprint_two_process` (D-35): the parent computes
/// `approval_gate.json`'s fingerprint in-process, then spawns
/// `std::env::current_exe()` (this SAME test binary) as a genuinely
/// separate OS process to recompute it, and asserts equality. Rust's
/// `HashMap` uses a per-process random `RandomState` seed, so a map-order
/// leak into the canonical bytes would fail this test -- and would NOT be
/// caught by a same-process round trip.
#[test]
fn wargraph_doc_fingerprint_two_process() {
    let path = fixtures_dir().join("approval_gate.json");
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc: WarGraphDoc =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let graph = doc
        .compile(&fixture_registries())
        .unwrap_or_else(|e| panic!("compile {}: {e}", path.display()));
    let in_process_fingerprint = graph.fingerprint();

    let exe = std::env::current_exe().expect("current_exe");
    let output = Command::new(exe)
        .args(["--exact", "fingerprint_child", "--nocapture"])
        .env("PALADIN_FINGERPRINT_CHILD", &path)
        .output()
        .expect("spawn child process");
    assert!(
        output.status.success(),
        "child process exited non-zero: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.starts_with("FINGERPRINT="))
        .unwrap_or_else(|| panic!("child stdout carried no FINGERPRINT= line:\n{stdout}"));
    let child_fingerprint = line.trim_start_matches("FINGERPRINT=");

    assert_eq!(
        in_process_fingerprint.as_str(),
        child_fingerprint,
        "fingerprint differs across a real OS process boundary"
    );
}

/// The child half of [`wargraph_doc_fingerprint_two_process`]'s two-process
/// proof (D-35). A no-op `Ok` when `PALADIN_FINGERPRINT_CHILD` is absent --
/// which it always is under a plain `cargo test` run -- so this test never
/// spawns anything on its own; when present (only ever set by the parent
/// test's own `Command::env` call), it prints `FINGERPRINT=<value>` and
/// nothing else to stdout.
#[test]
fn fingerprint_child() {
    let Some(path) = std::env::var_os("PALADIN_FINGERPRINT_CHILD") else {
        return;
    };
    let raw = std::fs::read_to_string(&path).expect("read fixture in child process");
    let doc: WarGraphDoc = serde_json::from_str(&raw).expect("parse fixture in child process");
    let graph = doc
        .compile(&fixture_registries())
        .expect("compile fixture in child process");
    println!("FINGERPRINT={}", graph.fingerprint().as_str());
}
