//! `Scenario`'s golden-schema bless test (D-28), mirroring
//! `paladin-battalion/tests/graph_doc_round_trip.rs`'s `wargraph_doc_schema_matches_golden`
//! bless idiom exactly: `UPDATE_EVAL_SCHEMA=1` regenerates the committed golden file.

use std::path::PathBuf;

/// The checked-in golden schema, one level up from the crate at the repo's
/// `docs/schemas/` (mirroring `graph_doc_round_trip.rs`'s own repo-root-relative path
/// helper).
fn schema_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/schemas/eval-scenario.schema.json"
    ))
}

/// `schema_matches_golden`: `schemars::schema_for!(Scenario)` serialised as pretty JSON
/// must equal the committed golden file byte-for-byte. `UPDATE_EVAL_SCHEMA=1`
/// regenerates it.
#[test]
fn schema_matches_golden() {
    let schema_value = schemars::schema_for!(paladin_eval::scenario::Scenario).to_value();
    let generated =
        serde_json::to_string_pretty(&schema_value).expect("serialize derived schema") + "\n";
    let path = schema_path();

    if std::env::var_os("UPDATE_EVAL_SCHEMA").is_some() {
        std::fs::write(&path, &generated).expect("write golden schema");
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        generated.trim(),
        committed.trim(),
        "Scenario's derived JSON Schema drifted from {}. If intentional, regenerate with: \
         UPDATE_EVAL_SCHEMA=1 cargo test -p paladin-eval --test schema_golden schema_matches_golden",
        path.display()
    );
}
