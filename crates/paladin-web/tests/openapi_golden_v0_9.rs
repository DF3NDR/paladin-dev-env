//! Golden diff proving the v0.10 HTTP surface did not change under the six paths a v0.9
//! client already calls (SHIP-02, per `.planning/phases/29-program-gates-release/29-CONTEXT.md`
//! D-08).
//!
//! Both documents -- the spec `openapi_spec()` generates today and the frozen `v0.9.0` baseline
//! committed at `tests/fixtures/openapi-v0.9.0.json` -- are restricted to the six pre-existing
//! `/v1/agents...` paths before any comparison runs. A whole-document diff would be useless: v0.10
//! adds seventeen new paths (`/v1/threads/...`, `/v1/runs/...`, ...), so an unrestricted
//! comparison would fail on additions that are not regressions. D-08's restriction rules, in full:
//!
//! - keep only the six v0.9 `paths` entries (this file, `restrict_paths`)
//! - follow the transitive `$ref` closure of those operations into `components.schemas`
//! - keep `components.securitySchemes` in full, unrestricted
//! - drop `info.version` -- the ONLY sanctioned normalisation; any other inequality is a real,
//!   reported SHIP-02 failure, never something to normalise away
//!
//! This file has no environment-variable-driven regeneration escape hatch (unlike
//! `crates/paladin-web/src/openapi.rs::openapi_matches_committed_baseline`, whose committed
//! baseline the version bump legitimately regenerates): `tests/fixtures/openapi-v0.9.0.json` is a
//! frozen historical record of the `v0.9.0` tag, not a baseline that tracks HEAD. There is nothing
//! to regenerate it from -- see `tests/fixtures/README.md` for the exact provenance.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The six paths a v0.9 client could call, in the exact spelling `openapi_spec()` and the
/// frozen `v0.9.0` baseline both use. This is the single source of truth for the restriction --
/// deliberately spelled out here, not derived from the baseline being compared against, so a
/// baseline that silently lost a path would not shrink the restriction along with it.
const V0_9_PATHS: &[&str] = &[
    "/v1/agents",
    "/v1/agents/{id}",
    "/v1/agents/{id}/execute",
    "/v1/agents/{id}/execute/stream",
    "/v1/agents/{id}/jobs",
    "/v1/agents/{id}/jobs/{job_id}",
];

/// Path of the frozen `v0.9.0` OpenAPI baseline (see `tests/fixtures/README.md` for provenance).
fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/openapi-v0.9.0.json")
}

/// Load the frozen `v0.9.0` baseline document.
fn load_baseline() -> Value {
    let path = baseline_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read frozen baseline {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse frozen baseline {}: {e}", path.display()))
}

/// Generate today's OpenAPI document as a `serde_json::Value`, via the same `openapi_spec()`
/// the committed-baseline drift guard (`crates/paladin-web/src/openapi.rs`) uses.
fn generated_spec() -> Value {
    serde_json::to_value(paladin_web::openapi::openapi_spec()).expect("serialize generated spec")
}

/// Restrict a document's `paths` object to the six v0.9 paths named in [`V0_9_PATHS`].
///
/// Returns a `serde_json::Value::Object` so the result is directly comparable via
/// [`assert_deep_eq`] and iteration order is irrelevant to the comparison.
fn restrict_paths(doc: &Value) -> Value {
    let paths = doc
        .get("paths")
        .and_then(Value::as_object)
        .expect("document has a `paths` object");
    let mut restricted = serde_json::Map::new();
    for &key in V0_9_PATHS {
        if let Some(value) = paths.get(key) {
            restricted.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(restricted)
}

/// Walk two `Value`s and report the first differing JSON pointer, so a failure names exactly
/// where two large documents diverge instead of dumping both whole documents.
///
/// Object key order is never significant (compared by key, not serialized text). Array element
/// order IS significant -- a reordered `required`/`tags`/`enum` array is a real, reported
/// difference, not a false failure.
fn first_difference(pointer: &str, a: &Value, b: &Value) -> Option<(String, Value, Value)> {
    if a == b {
        return None;
    }
    match (a, b) {
        (Value::Object(a_map), Value::Object(b_map)) => {
            let mut keys: Vec<&String> = a_map.keys().chain(b_map.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let child_pointer = format!("{pointer}/{key}");
                match (a_map.get(key), b_map.get(key)) {
                    (Some(av), Some(bv)) => {
                        if let Some(diff) = first_difference(&child_pointer, av, bv) {
                            return Some(diff);
                        }
                    }
                    (Some(av), None) => return Some((child_pointer, av.clone(), Value::Null)),
                    (None, Some(bv)) => return Some((child_pointer, Value::Null, bv.clone())),
                    (None, None) => unreachable!("key came from one of the two maps' key sets"),
                }
            }
            None
        }
        (Value::Array(a_arr), Value::Array(b_arr)) => {
            for (index, (av, bv)) in a_arr.iter().zip(b_arr.iter()).enumerate() {
                let child_pointer = format!("{pointer}/{index}");
                if let Some(diff) = first_difference(&child_pointer, av, bv) {
                    return Some(diff);
                }
            }
            if a_arr.len() != b_arr.len() {
                return Some((pointer.to_string(), a.clone(), b.clone()));
            }
            None
        }
        _ => Some((pointer.to_string(), a.clone(), b.clone())),
    }
}

/// Panic with a readable, pointer-scoped diff if `a` and `b` are not deep-equal.
fn assert_deep_eq(a: &Value, b: &Value, context: &str) {
    if let Some((pointer, av, bv)) = first_difference("", a, b) {
        panic!("{context}: documents differ at `{pointer}`:\n  generated: {av}\n  baseline:  {bv}");
    }
}

/// The restriction itself must never be vacuous: an empty or partial restriction must fail
/// loudly here rather than let [`openapi_v0_9_paths_match_the_frozen_baseline`] pass on two
/// empty maps (D-08, threat T-29-02-04).
#[test]
fn v0_9_path_restriction_is_non_empty() {
    let generated = restrict_paths(&generated_spec());
    let baseline = restrict_paths(&load_baseline());

    let generated_keys: Vec<&String> = generated.as_object().unwrap().keys().collect();
    let baseline_keys: Vec<&String> = baseline.as_object().unwrap().keys().collect();

    assert_eq!(
        generated_keys.len(),
        6,
        "expected exactly 6 v0.9 paths in the generated spec, found {}: {:?}",
        generated_keys.len(),
        generated_keys
    );
    assert_eq!(
        baseline_keys.len(),
        6,
        "expected exactly 6 v0.9 paths in the frozen baseline, found {}: {:?}",
        baseline_keys.len(),
        baseline_keys
    );
}

/// The six pre-existing paths' operation objects must be byte-for-byte equivalent (at the
/// `serde_json::Value` level) to what a v0.9 client already calls against.
#[test]
fn openapi_v0_9_paths_match_the_frozen_baseline() {
    let generated = restrict_paths(&generated_spec());
    let baseline = restrict_paths(&load_baseline());

    assert_deep_eq(
        &generated,
        &baseline,
        "the six v0.9 paths' operation objects",
    );
}
