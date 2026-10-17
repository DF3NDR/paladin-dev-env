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
//! - drop `info.version` -- one sanctioned normalisation; any other inequality is a real,
//!   reported SHIP-02 failure, never something to normalise away
//! - drop the ONE sanctioned `ExecuteResponse` field rename below (Phase 31, D-24 / ADR-0051)
//! - drop the `429` response entries Phase 41 adds to three operations (below)
//! - drop the `422` response entries and the `ExecuteResponse.halt_reason` field Phase 42 adds
//!   (below)
//!
//! This file has no environment-variable-driven regeneration escape hatch (unlike
//! `crates/paladin-web/src/openapi.rs::openapi_matches_committed_baseline`, whose committed
//! baseline the version bump legitimately regenerates): `tests/fixtures/openapi-v0.9.0.json` is a
//! frozen historical record of the `v0.9.0` tag, not a baseline that tracks HEAD. There is nothing
//! to regenerate it from -- see `tests/fixtures/README.md` for the exact provenance.
//!
//! ## Phase 31 exception: `ExecuteResponse.token_count` -> `usage` (D-24, ADR-0051)
//!
//! `.planning/phases/31-lossless-token-accounting/31-CONTEXT.md` declares this phase a
//! "breaking, clean break, no shims" program (ADR-0051, X-03 -- the no-breaking-change-without-a-
//! shim rule this SHIP-02 gate itself enforces -- explicitly superseded for Phases 31-33 only).
//! `ExecuteResponse`'s bare `token_count: u32` becomes `usage: TokenUsageResponse` (a full
//! six-field token split) so the HTTP edge stops re-collapsing the split this phase exists to
//! carry end to end; the one-way door was confirmed at the phase's plan 31-01 consolidated
//! checkpoint. [`strip_known_v0_10_execute_response_divergence`] removes exactly this one
//! field-level divergence (and the new `TokenUsageResponse` schema it introduces) from BOTH
//! documents' `ExecuteResponse` schema before the ref-closure comparison runs, so this gate keeps
//! catching any OTHER, unintentional break to a pre-existing v0.9 path or schema -- this is a
//! second sanctioned, narrowly-scoped, explicitly-documented exception alongside `info.version`,
//! never a loosening of the gate's general power.
//!
//! ## Phase 39 exception: `ExecuteResponse.cost` (D-10, LEDGR-04)
//!
//! `.planning/phases/39-spend-ledger/39-CONTEXT.md` D-10 exposes `PaladinResult.cost` on
//! `ExecuteResponse` -- Phase 38's own regression test (`execute_response_carries_no_cost_field`,
//! now inverted to `execute_response_carries_cost_when_priced`) named this phase by number as the
//! one that would add it. This is purely additive (a new, nullable `cost: Option<CostDto>` field;
//! `output`/`usage`/`execution_time_ms`/`loop_count`/`stop_reason` are all untouched), but it still
//! introduces a schema the frozen `v0.9.0` baseline has no way to have: `cost` is absent from the
//! baseline's `ExecuteResponse` entirely (never `token_count`-like renamed, just new).
//! [`strip_known_v0_10_execute_response_divergence`] removes this field (and the `CostDto` schema
//! it introduces) alongside the Phase 31 exception, for the same reason: so this gate keeps
//! catching any OTHER, unintentional break to a pre-existing v0.9 path or schema.

//!
//! ## Phase 41 exception: allowance 429 on the agent routes (D-12, ALLOW-02)
//!
//! `.planning/phases/41-admission-time-allowance-enforcement/41-CONTEXT.md` D-12 documents the
//! new `429 allowance_exhausted` answer in the OpenAPI document, and ALLOW-02 (plan 41-04) makes
//! `POST /v1/agents/{id}/execute`, `.../execute/stream` and `.../jobs` produce it, because those
//! routes settle spend under the calling principal and would otherwise let a refused caller spend
//! the same allowance. A `429` response entry is purely additive for a client -- no field, schema
//! or status a v0.9 client already handles changes -- but it is absent from the frozen `v0.9.0`
//! baseline by construction. [`strip_known_v0_11_allowance_429`] removes exactly the `"429"` key
//! from `responses` on exactly those three operations in BOTH documents before any comparison, so
//! this is a third sanctioned, narrowly-scoped, explicitly-documented exception alongside
//! `info.version` and the `ExecuteResponse` rename/addition above -- never a loosening of the
//! gate's general power: every other response, status, schema and path on all six v0.9 paths is
//! still compared in full, and
//! [`allowance_429_exception_is_narrowly_scoped`] fails if the exception ever widens.
//!
//! ## Phase 42 exception: `ExecuteResponse.halt_reason` and the `422 model_unpriced` response (D-10, D-12)
//!
//! `.planning/decisions/0057-mid-run-halt-contract.md` group (e) lets an agent call stop on the
//! caller's allowance (plan 42-08, ALLOW-05). Two purely additive changes reach the frozen v0.9
//! paths, both absent from the `v0.9.0` baseline by construction: `ExecuteResponse` gains a
//! nullable `halt_reason` object (present only when `stop_reason` is `allowance_halted`; the
//! same object `GET /runs/{id}` reports, and the `done` data of the buffered fallback of
//! `execute/stream`, which serializes this DTO), and `POST /v1/agents/{id}/execute`,
//! `.../execute/stream` and `.../jobs` document a `422` (`model_unpriced`) for a caller with an
//! allowance ceiling whose agent model has no `treasurer.pricing` row.
//! [`strip_known_v0_11_halt_reason`] removes exactly the `"422"` key from `responses` on exactly
//! those three operations and exactly the `halt_reason` key from `ExecuteResponse`'s
//! `properties`/`required`, from BOTH documents, so this is a fourth sanctioned, narrowly-scoped,
//! explicitly-documented exception -- no other response, status, field, schema or path on the
//! six v0.9 paths is touched, and [`phase_42_exception_is_narrowly_scoped`] fails if it widens.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Prefix stripped from a `$ref` string to recover its `components.schemas` key.
const SCHEMA_REF_PREFIX: &str = "#/components/schemas/";

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

/// Load the frozen `v0.9.0` baseline document, with the Phase 42 exception
/// ([`strip_known_v0_11_halt_reason`]) applied (a no-op on the frozen document, which never
/// carried either entry).
fn load_baseline() -> Value {
    let mut doc = load_baseline_unstripped();
    strip_known_v0_11_halt_reason(&mut doc);
    doc
}

/// [`load_baseline`] without the Phase 42 exception -- used only to prove the exception's own
/// scope.
fn load_baseline_unstripped() -> Value {
    let path = baseline_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read frozen baseline {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse frozen baseline {}: {e}", path.display()))
}

/// Generate today's OpenAPI document as a `serde_json::Value`, via the same `openapi_spec()`
/// the committed-baseline drift guard (`crates/paladin-web/src/openapi.rs`) uses, with the
/// Phase 42 exception ([`strip_known_v0_11_halt_reason`]) applied.
fn generated_spec() -> Value {
    let mut doc = generated_spec_unstripped();
    strip_known_v0_11_halt_reason(&mut doc);
    doc
}

/// [`generated_spec`] without the Phase 42 exception -- used only to prove the exception's own
/// scope.
fn generated_spec_unstripped() -> Value {
    serde_json::to_value(paladin_web::openapi::openapi_spec()).expect("serialize generated spec")
}

/// The three v0.9 operations (`POST`) that gained a `429 allowance_exhausted` response in
/// Phase 41 (D-12, ALLOW-02) -- spelled out here, never derived, so the exception can only ever
/// remove an entry from exactly these three operations.
const ALLOWANCE_429_PATHS: &[&str] = &[
    "/v1/agents/{id}/execute",
    "/v1/agents/{id}/execute/stream",
    "/v1/agents/{id}/jobs",
];

/// The three v0.9 operations (`POST`) that gained a `422 model_unpriced` response in Phase 42
/// (D-10, plan 42-08) -- spelled out here, never derived, so the exception can only ever remove
/// an entry from exactly these three operations.
const MODEL_UNPRICED_422_PATHS: &[&str] = &[
    "/v1/agents/{id}/execute",
    "/v1/agents/{id}/execute/stream",
    "/v1/agents/{id}/jobs",
];

/// Restrict a document's `paths` object to the six v0.9 paths named in [`V0_9_PATHS`], with the
/// Phase 41 `429` exception ([`strip_known_v0_11_allowance_429`]) already applied.
///
/// Returns a `serde_json::Value::Object` so the result is directly comparable via
/// [`assert_deep_eq`] and iteration order is irrelevant to the comparison.
fn restrict_paths(doc: &Value) -> Value {
    let mut restricted = restrict_paths_unstripped(doc);
    strip_known_v0_11_allowance_429(&mut restricted);
    restricted
}

/// [`restrict_paths`] without the Phase 41 exception -- used only to prove the exception's own
/// scope.
fn restrict_paths_unstripped(doc: &Value) -> Value {
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

/// Remove the ONE sanctioned Phase 41 divergence (see this file's module docs): the `"429"`
/// entry of `responses` on the `post` operation of the three [`ALLOWANCE_429_PATHS`], from a
/// restricted `paths` object in place. Nothing else -- not a sibling status, not another
/// operation, not a schema -- is touched.
fn strip_known_v0_11_allowance_429(restricted_paths: &mut Value) {
    for &path in ALLOWANCE_429_PATHS {
        if let Some(responses) = restricted_paths
            .get_mut(path)
            .and_then(|item| item.get_mut("post"))
            .and_then(|operation| operation.get_mut("responses"))
            .and_then(Value::as_object_mut)
        {
            responses.remove("429");
        }
    }
}

/// Remove the ONE sanctioned Phase 42 divergence (see this file's module docs) from a whole
/// OpenAPI document in place:
///
/// - the `"422"` entry of `responses` on the `post` operation of the three
///   [`MODEL_UNPRICED_422_PATHS`] (`code = "model_unpriced"`, plan 42-08, D-10);
/// - the `halt_reason` key of `components.schemas.ExecuteResponse`'s `properties` and `required`
///   (D-12).
///
/// Nothing else -- not a sibling status, not another operation, not another field or schema --
/// is touched.
fn strip_known_v0_11_halt_reason(doc: &mut Value) {
    for &path in MODEL_UNPRICED_422_PATHS {
        if let Some(responses) = doc
            .get_mut("paths")
            .and_then(|paths| paths.get_mut(path))
            .and_then(|item| item.get_mut("post"))
            .and_then(|operation| operation.get_mut("responses"))
            .and_then(Value::as_object_mut)
        {
            responses.remove("422");
        }
    }
    if let Some(object) = doc
        .get_mut("components")
        .and_then(|components| components.get_mut("schemas"))
        .and_then(|schemas| schemas.get_mut("ExecuteResponse"))
        .and_then(Value::as_object_mut)
    {
        if let Some(required) = object.get_mut("required").and_then(Value::as_array_mut) {
            required.retain(|v| v != "halt_reason");
        }
        if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
            properties.remove("halt_reason");
        }
    }
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

/// The `components.schemas` object of a document.
fn schemas_of(doc: &Value) -> &serde_json::Map<String, Value> {
    doc.get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(Value::as_object)
        .expect("document has components.schemas")
}

/// The `components.securitySchemes` object of a document.
fn security_schemes_of(doc: &Value) -> &Value {
    doc.get("components")
        .and_then(|c| c.get("securitySchemes"))
        .expect("document has components.securitySchemes")
}

/// The `info` object of a document, with the `version` key removed -- the ONLY sanctioned
/// normalisation (D-08). Any other field difference between the generated document and the
/// frozen baseline is a real SHIP-02 failure and must never be normalised away.
fn info_sans_version(doc: &Value) -> Value {
    let mut info = doc
        .get("info")
        .cloned()
        .expect("document has an `info` object");
    if let Some(map) = info.as_object_mut() {
        map.remove("version");
    }
    info
}

/// Recursively collect every string found under a `$ref` key within `value`.
fn collect_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if key == "$ref" {
                    if let Value::String(s) = v {
                        refs.push(s.clone());
                    }
                } else {
                    collect_refs(v, refs);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_refs(v, refs);
            }
        }
        _ => {}
    }
}

/// Compute the transitive `$ref` closure of `subtree` into `schemas`.
///
/// Collects every `$ref` string found anywhere in `subtree`, resolves each to its
/// `components.schemas` entry, and recurses into that entry's own `$ref`s until the name set
/// stops growing. Panics naming the offending `$ref` if it names a key absent from `schemas` --
/// a silently-renamed schema must not slip through as "not in the closure".
fn ref_closure(
    subtree: &Value,
    schemas: &serde_json::Map<String, Value>,
) -> BTreeMap<String, Value> {
    let mut closure = BTreeMap::new();
    let mut frontier: Vec<String> = Vec::new();
    collect_refs(subtree, &mut frontier);

    while let Some(reference) = frontier.pop() {
        let name = reference
            .strip_prefix(SCHEMA_REF_PREFIX)
            .unwrap_or_else(|| {
                panic!("unexpected $ref shape (not a components/schemas ref): `{reference}`")
            });
        if closure.contains_key(name) {
            continue;
        }
        let schema = schemas.get(name).unwrap_or_else(|| {
            panic!("$ref `{reference}` names a schema absent from components.schemas")
        });
        closure.insert(name.to_string(), schema.clone());
        collect_refs(schema, &mut frontier);
    }

    closure
}

/// Remove the two sanctioned `ExecuteResponse` schema divergences (see this file's module docs)
/// from a `components.schemas` closure map in place:
///
/// - Phase 31 (D-24, ADR-0051): `token_count` (present only in the frozen `v0.9.0` baseline) and
///   `usage` (present only in the generated v0.10 document) are both dropped from `properties`
///   and `required`, and the `TokenUsageResponse` schema `usage` introduces is dropped entirely.
/// - Phase 39 (D-10, LEDGR-04): `cost` (present only in the generated document -- absent from
///   the baseline entirely, never renamed) is dropped from `properties` and `required`, and the
///   `CostDto` schema it introduces is dropped entirely.
///
/// Every other schema, and every other field of `ExecuteResponse` itself, is left untouched -- a
/// regression in `output`, `execution_time_ms`, `loop_count` or `stop_reason` still fails this
/// gate.
fn strip_known_v0_10_execute_response_divergence(schemas: &mut BTreeMap<String, Value>) {
    if let Some(object) = schemas
        .get_mut("ExecuteResponse")
        .and_then(Value::as_object_mut)
    {
        if let Some(required) = object.get_mut("required").and_then(Value::as_array_mut) {
            required.retain(|v| v != "token_count" && v != "usage" && v != "cost");
        }
        if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
            properties.remove("token_count");
            properties.remove("usage");
            properties.remove("cost");
        }
    }
    schemas.remove("TokenUsageResponse");
    schemas.remove("CostDto");
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

/// Every schema transitively reachable from the six v0.9 paths' operations must be deep-equal
/// between the generated document and the frozen baseline -- a changed request/response schema
/// reachable only through a pre-existing path is a SHIP-02 failure even though the path key
/// itself did not change (T-29-02-02).
#[test]
fn ref_closure_schemas_match_the_frozen_baseline() {
    let generated_doc = generated_spec();
    let baseline_doc = load_baseline();

    let mut generated_closure =
        ref_closure(&restrict_paths(&generated_doc), schemas_of(&generated_doc));
    let mut baseline_closure =
        ref_closure(&restrict_paths(&baseline_doc), schemas_of(&baseline_doc));

    // The ONE sanctioned Phase 31 schema divergence (D-24, ADR-0051) -- see this file's module
    // docs and `strip_known_v0_10_execute_response_divergence`'s own docs.
    strip_known_v0_10_execute_response_divergence(&mut generated_closure);
    strip_known_v0_10_execute_response_divergence(&mut baseline_closure);

    let generated_value = serde_json::to_value(&generated_closure).expect("serialize closure");
    let baseline_value = serde_json::to_value(&baseline_closure).expect("serialize closure");

    assert_deep_eq(
        &generated_value,
        &baseline_value,
        "the $ref closure of the six v0.9 paths into components.schemas",
    );
}

/// The Phase 31 + Phase 39 `ExecuteResponse` exceptions strip exactly the three known-divergent
/// keys and the `TokenUsageResponse`/`CostDto` schemas -- proven directly against the real
/// generated/baseline closures, so a future edit that widens the exception (e.g. also dropping an
/// unrelated field) breaks this test instead of silently passing.
#[test]
fn execute_response_exception_is_narrowly_scoped() {
    let generated_doc = generated_spec();
    let baseline_doc = load_baseline();

    let mut generated_closure =
        ref_closure(&restrict_paths(&generated_doc), schemas_of(&generated_doc));
    let mut baseline_closure =
        ref_closure(&restrict_paths(&baseline_doc), schemas_of(&baseline_doc));

    // Before stripping: the two closures must actually differ under `ExecuteResponse`, and the
    // generated closure must actually contain `TokenUsageResponse`/`CostDto` -- otherwise this
    // test would exercise nothing.
    assert_ne!(
        generated_closure.get("ExecuteResponse"),
        baseline_closure.get("ExecuteResponse"),
        "fixture must actually diverge on ExecuteResponse before stripping"
    );
    assert!(
        generated_closure.contains_key("TokenUsageResponse"),
        "generated closure must contain the new TokenUsageResponse schema before stripping"
    );
    assert!(
        generated_closure.contains_key("CostDto"),
        "generated closure must contain the new CostDto schema before stripping"
    );
    assert!(
        !baseline_closure.contains_key("CostDto"),
        "the frozen v0.9.0 baseline must never contain CostDto"
    );

    strip_known_v0_10_execute_response_divergence(&mut generated_closure);
    strip_known_v0_10_execute_response_divergence(&mut baseline_closure);

    assert!(!generated_closure.contains_key("TokenUsageResponse"));
    assert!(!baseline_closure.contains_key("TokenUsageResponse"));
    assert!(!generated_closure.contains_key("CostDto"));
    assert!(!baseline_closure.contains_key("CostDto"));

    for (label, closure) in [
        ("generated", &generated_closure),
        ("baseline", &baseline_closure),
    ] {
        let execute_response = closure
            .get("ExecuteResponse")
            .expect("ExecuteResponse present in both closures");
        let properties = execute_response
            .get("properties")
            .and_then(Value::as_object)
            .expect("ExecuteResponse has properties");
        assert!(
            !properties.contains_key("token_count"),
            "{label}: token_count must be stripped"
        );
        assert!(
            !properties.contains_key("usage"),
            "{label}: usage must be stripped"
        );
        assert!(
            !properties.contains_key("cost"),
            "{label}: cost must be stripped"
        );
        // Every other field must survive the exception untouched.
        for field in ["output", "execution_time_ms", "loop_count", "stop_reason"] {
            assert!(
                properties.contains_key(field),
                "{label}: {field} must survive the exception"
            );
        }
    }
}

/// The Phase 41 `429` exception strips exactly the `"429"` response key from exactly the three
/// agent operations: proven against the real generated and baseline documents, so a future edit
/// that widens it (another status, another operation, a schema) breaks this test.
#[test]
fn allowance_429_exception_is_narrowly_scoped() {
    let generated = restrict_paths_unstripped(&generated_spec());
    let baseline = restrict_paths_unstripped(&load_baseline());

    let status_keys = |doc: &Value, path: &str| -> Vec<String> {
        doc[path]["post"]["responses"]
            .as_object()
            .expect("responses object")
            .keys()
            .cloned()
            .collect()
    };

    for &path in ALLOWANCE_429_PATHS {
        // Before stripping: the generated document documents the 429, the frozen one cannot.
        assert!(
            generated[path]["post"]["responses"]
                .get("429")
                .is_some_and(|response| !response.is_null()),
            "{path}: the generated document must carry the Phase 41 429 response"
        );
        assert!(
            baseline[path]["post"]["responses"].get("429").is_none(),
            "{path}: the frozen v0.9.0 baseline must never carry a 429"
        );
    }

    let mut generated_stripped = generated.clone();
    strip_known_v0_11_allowance_429(&mut generated_stripped);
    let mut baseline_stripped = baseline.clone();
    strip_known_v0_11_allowance_429(&mut baseline_stripped);

    for &path in ALLOWANCE_429_PATHS {
        let mut stripped_keys = status_keys(&generated_stripped, path);
        let mut frozen_keys = status_keys(&baseline, path);
        stripped_keys.sort();
        frozen_keys.sort();
        assert_eq!(
            stripped_keys, frozen_keys,
            "{path}: after stripping, only the 429 key may differ from the frozen baseline"
        );
    }

    // The exception reaches nothing else: every other path is identical before and after.
    for &path in V0_9_PATHS {
        if ALLOWANCE_429_PATHS.contains(&path) {
            continue;
        }
        assert_eq!(
            generated[path], generated_stripped[path],
            "{path}: the 429 exception must not touch any other path"
        );
    }
    // ...and on the three operations, every other response is unchanged by it.
    for &path in ALLOWANCE_429_PATHS {
        let mut expected = generated[path].clone();
        expected["post"]["responses"]
            .as_object_mut()
            .expect("responses object")
            .remove("429");
        assert_eq!(
            generated_stripped[path], expected,
            "{path}: the exception must remove the 429 entry and nothing else"
        );
    }
}

/// The closure must never be vacuous, and every `$ref` it encounters must resolve to a present
/// `components.schemas` key -- `ref_closure` itself panics naming the offending pointer on an
/// unresolvable `$ref`, so a silently-renamed schema cannot slip through as "not in the closure"
/// (T-29-02-04).
#[test]
fn ref_closure_is_non_empty_and_fully_resolved() {
    for (label, doc) in [
        ("generated", generated_spec()),
        ("baseline", load_baseline()),
    ] {
        let closure = ref_closure(&restrict_paths(&doc), schemas_of(&doc));

        assert!(
            !closure.is_empty(),
            "{label}: the $ref closure of the six v0.9 paths must be non-empty"
        );
        for name in closure.keys() {
            assert!(
                schemas_of(&doc).contains_key(name),
                "{label}: closure member `{name}` must exist in components.schemas"
            );
        }
    }
}

/// `components.securitySchemes` must be deep-equal between the two documents in full, with no
/// path restriction applied -- an auth-scheme rename on a pre-existing path is a SHIP-02 failure
/// (T-29-02-03).
#[test]
fn security_schemes_match_the_frozen_baseline() {
    let generated = generated_spec();
    let baseline = load_baseline();

    assert_deep_eq(
        security_schemes_of(&generated),
        security_schemes_of(&baseline),
        "components.securitySchemes",
    );
}

/// `info.version` is the ONLY sanctioned normalisation (D-08). This is asserted explicitly,
/// rather than implicitly relied on, so a future edit that quietly widens the normalisation set
/// breaks this test instead of silently passing.
#[test]
fn info_version_is_the_only_normalisation() {
    let generated = generated_spec();
    let baseline = load_baseline();

    // Every `info` field other than `version` must already agree with no normalisation applied
    // -- proving `version` really is the one sanctioned exception, not a stand-in for "the info
    // block might differ in several ways that all get waved through".
    assert_deep_eq(
        &info_sans_version(&generated),
        &info_sans_version(&baseline),
        "info fields other than `version`",
    );

    // Prove the normalisation is scoped to exactly the `version` key: a synthetic document whose
    // `info.version` differs from the baseline's, but is otherwise identical, becomes equal
    // after `info_sans_version`.
    let mut version_only_diff = baseline.clone();
    version_only_diff["info"]["version"] = Value::String("9.9.9-synthetic".to_string());
    assert_ne!(
        version_only_diff["info"]["version"], baseline["info"]["version"],
        "synthetic fixture must actually differ in version to exercise the normalisation"
    );
    assert_deep_eq(
        &info_sans_version(&version_only_diff),
        &info_sans_version(&baseline),
        "a version-only difference must normalise away",
    );

    // ...and a document that differs in ANY OTHER info field must stay unequal after the same
    // normalisation -- so a future edit that quietly widens the normalisation set (e.g. also
    // dropping `info.title`) breaks this test.
    let mut other_field_diff = baseline.clone();
    other_field_diff["info"]["title"] = Value::String("A Different Title".to_string());
    let diff = first_difference(
        "",
        &info_sans_version(&other_field_diff),
        &info_sans_version(&baseline),
    );
    assert!(
        diff.is_some(),
        "a non-version info field difference must NOT be normalised away"
    );
}

/// The Phase 42 exception strips exactly the `"422"` response key from exactly the three agent
/// operations and exactly `halt_reason` from `ExecuteResponse`: proven against the real
/// generated and baseline documents, so a future edit that widens it (another status, another
/// operation, another field or schema) breaks this test.
#[test]
fn phase_42_exception_is_narrowly_scoped() {
    let generated = generated_spec_unstripped();
    let baseline = load_baseline_unstripped();

    // Before stripping: the generated document documents both additions, the frozen one cannot.
    for &path in MODEL_UNPRICED_422_PATHS {
        assert!(
            generated["paths"][path]["post"]["responses"]
                .get("422")
                .is_some_and(|response| !response.is_null()),
            "{path}: the generated document must carry the Phase 42 422 response"
        );
        assert!(
            baseline["paths"][path]["post"]["responses"]
                .get("422")
                .is_none(),
            "{path}: the frozen v0.9.0 baseline must never carry a 422"
        );
    }
    assert!(
        generated["components"]["schemas"]["ExecuteResponse"]["properties"]
            .get("halt_reason")
            .is_some(),
        "the generated ExecuteResponse must carry halt_reason before stripping"
    );
    assert!(
        baseline["components"]["schemas"]["ExecuteResponse"]["properties"]
            .get("halt_reason")
            .is_none(),
        "the frozen v0.9.0 baseline must never carry halt_reason"
    );
    // `halt_reason` is optional on the wire: it must never become a required property.
    assert!(
        !generated["components"]["schemas"]["ExecuteResponse"]["required"]
            .as_array()
            .expect("ExecuteResponse has a required array")
            .iter()
            .any(|v| v == "halt_reason"),
        "halt_reason is absent on every non-halt response, so it must not be required"
    );

    let mut stripped = generated.clone();
    strip_known_v0_11_halt_reason(&mut stripped);

    // Exactly the three 422 entries are gone; every other response on those operations stays.
    for &path in MODEL_UNPRICED_422_PATHS {
        let mut expected = generated["paths"][path].clone();
        expected["post"]["responses"]
            .as_object_mut()
            .expect("responses object")
            .remove("422");
        assert_eq!(
            stripped["paths"][path], expected,
            "{path}: the exception must remove the 422 entry and nothing else"
        );
    }
    // No other path is touched.
    for (path, item) in generated["paths"].as_object().expect("paths object") {
        if MODEL_UNPRICED_422_PATHS.contains(&path.as_str()) {
            continue;
        }
        assert_eq!(
            &stripped["paths"][path], item,
            "{path}: the Phase 42 exception must not touch any other path"
        );
    }
    // Exactly `halt_reason` is gone from `ExecuteResponse`; every other schema is untouched.
    let mut expected_schema = generated["components"]["schemas"]["ExecuteResponse"].clone();
    expected_schema["properties"]
        .as_object_mut()
        .expect("properties object")
        .remove("halt_reason");
    assert_eq!(
        stripped["components"]["schemas"]["ExecuteResponse"], expected_schema,
        "the exception must remove halt_reason from ExecuteResponse and nothing else"
    );
    for (name, schema) in generated["components"]["schemas"]
        .as_object()
        .expect("schemas object")
    {
        if name == "ExecuteResponse" {
            continue;
        }
        assert_eq!(
            &stripped["components"]["schemas"][name], schema,
            "schema `{name}` must not be touched by the Phase 42 exception"
        );
    }
}
