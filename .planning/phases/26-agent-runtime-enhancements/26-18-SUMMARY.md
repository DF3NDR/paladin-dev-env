---
phase: 26-agent-runtime-enhancements
plan: 18
subsystem: engine
tags: [war-engine, structured-output, json-schema, fingerprint, fail-closed-validation, rust]

requires:
  - phase: 26-agent-runtime-enhancements/26-12
    provides: "paladin_core::platform::container::structured (SchemaRef, Structured<T>, StructuredOptions, extract_json, shape_check, render_instruction_block) and paladin_ports::output::structured_executor_port (StructuredExecutorPort, run_structured)"
  - phase: 26-agent-runtime-enhancements/26-13
    provides: "the engine's Vault grant wiring (NodeContext.vault, WarEngine::with_vault, execute_scoped dispatch in superstep.rs) that this plan's ChildEngineResources/run_with_namespace threading mirrors for structured_executor"
  - phase: 26-agent-runtime-enhancements/26-17
    provides: "PaladinExecutionService implements StructuredExecutorPort natively (execute_json_schema/execute_json_schema_observed), which this plan's integration tests drive directly as the engine's structured executor"
provides:
  - "NodeSpec::Paladin.output_schema: Option<SchemaRef> with a fail-closed validation matrix (four distinct EngineError variants) checked before any node runs"
  - "WarEngine::with_structured_executor / with_output_schema builders, StructuredSchema trait + TypedSchema<T> (full typed validation by deserialization, no schemars dependency in paladin-battalion)"
  - "The structured dispatch branch in engine::superstep: a Paladin node with output_schema writes the PARSED JSON VALUE to output_field, never a string"
  - "GRAPH_FINGERPRINT_VERSION v5 -> v6 with output_schema hashed sorted and length-prefixed, golden re-pinned in the same commit"
affects: [26-19, 26-20, 26-21]

tech-stack:
  added: []
  patterns:
    - "Engine-config fail-closed checks (executor presence) live as a separate WarGraph method called by WarEngine after WarGraph::validate, mirroring validate_node_cache_backend; graph-local checks (registered-name lookup, directive-parser conflict, field-type incompatibility) live inside WarGraph::validate_non_recursive, mirroring validate_aegis_cache_fields's 'collect two distinct offender sets in one pass' discipline"
    - "A new engine-owned resource (structured_executor) is threaded through WarEngine -> superstep::run/run_with_namespace -> ChildEngineResources -> execute_vanguard_node as a plain parameter, exactly like node_cache/vault -- never folded into NodeContext, since NodeContext's Debug/Clone/PartialEq derives cannot accommodate an un-comparable Arc<dyn Trait> without a bespoke impl (paladin_port itself is threaded the same plain-parameter way)"

key-files:
  created:
    - tests/integration/structured_engine_node_test.rs
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/registries.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/llm_failure.rs
    - crates/paladin-core/src/platform/container/waypoint.rs
    - tests/integration/mod.rs

key-decisions:
  - "Checkpoint resolutions: auto-selected fingerprint-v6-as-locked (auto-mode) -- Task 1's checkpoint:decision (gate=blocking) was pre-resolved by the orchestrator per D-29: output_schema enters WarGraph::fingerprint() sorted and length-prefixed, GRAPH_FINGERPRINT_VERSION bumps v5 -> v6, and the golden fixture is re-pinned in the same commit."
  - "TypedSchema<T>::new(schema: serde_json::Value) takes an explicit JSON Schema value rather than deriving one via schemars::schema_for! -- paladin-battalion has no schemars dependency (ADR-0015's core/ports allowlist keeps schemars facade-only, D-26) and must not gain one just for this type. A caller with a schemars-derived schema (the facade, or a test) passes the rendered Value in directly."
  - "output_field must accept JSON is enforced as 'the field's DispatchRule is not Sum' -- the only dispatch rule requiring the field to be strictly numeric, which an arbitrary structured JSON object can never satisfy. LastWrite/Append/MergeObject are all treated as JSON-compatible (Battlefield field values are already serde_json::Value under the hood; DispatchRule is the only per-field type signal a structured write's compatibility can be checked against, since FieldSpec carries no separate declared type)."
  - "StructuredSchema::validate() is declared on the trait and unit-tested directly (typed_schema_validates_by_deserialization), but is NOT wired into the per-node dispatch repair loop in this plan -- only StructuredSchema::to_json_schema() is consulted, to resolve a Registered name to the schema value sent to the model. Task 3's own test list (structured_directive_and_output_schema_share_extract_json) asserts there is exactly ONE envelope-extraction implementation in the engine; adding a second, registry-driven typed-repair round at the engine dispatch layer would cut against that same 'no second copy' discipline. Full typed validation via a registered schema stays available at the facade layer through 26-17's StructuredExecutorExt for a direct caller that wants it."
  - "The structured branch never threads a RunScope/Vault confinement (D-21, plan 26-13): StructuredExecutorPort::execute_json_schema_observed's signature has no RunScope parameter, so Vault-scoped structured execution is out of scope for this plan -- documented at the call site, not silently dropped."
  - "The engine's per-node retry-eligible NodeFailure::StructuredOutputInvalid variant hardcodes Transience::Unknown and is kept structurally SEPARATE from the pre-existing NodeFailure::Paladin(PaladinError) path (rather than special-cased inline), because PaladinError::transience() classifies PaladinError::StructuredOutputInvalid as Permanent for a NON-engine caller of execute_structured (26-17) -- a correct answer there, since that caller has no different-graph retry available. The engine's own classification is a deliberate, documented divergence from the general one, not a bug in either."

patterns-established:
  - "Fail-closed engine-config check pattern: WarGraph::validate_<resource>_backend(configured: bool) -> Result<(), EngineError>, called by every WarEngine::start/resume*/fork call site immediately after WarGraph::validate, with a private collect_<resource>_nodes recursive helper bounded by child-graph Arc pointer identity."

requirements-completed: [RT-05]

coverage:
  - id: D1
    description: "NodeSpec::Paladin gains output_schema: Option<SchemaRef>, constructor-preserved via NodeSpec::paladin(..) plus a chainable with_output_schema"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#node_spec_paladin_constructor_is_preserved"
        status: pass
  - id: D2
    description: "Four distinct fail-closed EngineError variants (StructuredExecutorMissing, UnregisteredOutputSchema, OutputSchemaWithStructuredDirective, OutputSchemaFieldNotJson) fire at validation, before any node runs, each listing every offender"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#output_schema_without_a_structured_executor_fails_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#unregistered_schema_name_fails_validation_listing_every_offender"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#output_schema_with_a_structured_directive_parser_fails_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#output_field_must_accept_json"
        status: pass
  - id: D3
    description: "TypedSchema<T>::new(schema) implements StructuredSchema by serde_json::from_value::<T> -- full typed validation, not the partial object-safe shape check"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#typed_schema_validates_by_deserialization"
        status: pass
  - id: D4
    description: "output_schema enters WarGraph::fingerprint() sorted and length-prefixed; GRAPH_FINGERPRINT_VERSION bumps v5 -> v6 with the golden re-pinned and the EngineLimits exclusion intact"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_changes_when_output_schema_changes"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_version_is_v6_and_the_golden_is_repinned"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#fingerprint_golden_hex_v6"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine_limits_are_still_excluded_from_the_hash"
        status: pass
  - id: D5
    description: "A Paladin node with output_schema dispatches through the structured executor and writes the PARSED JSON VALUE to output_field; a downstream node reads it as structure end-to-end through a real two-node WarGraph"
    requirement: "RT-05"
    verification:
      - kind: integration
        ref: "tests/integration/structured_engine_node_test.rs#structured_node_writes_a_parsed_object_to_output_field"
        status: pass
      - kind: integration
        ref: "tests/integration/structured_engine_node_test.rs#a_downstream_node_reads_the_typed_value"
        status: pass
      - kind: integration
        ref: "tests/integration/structured_engine_node_test.rs#registered_schema_by_name_works_end_to_end"
        status: pass
  - id: D6
    description: "Repair happens inside the node (one node-execution, two model calls); exhaustion becomes a NodeError with Paladin{kind: StructuredOutputInvalid} and Unknown transience, writing nothing; a TransientAndUnknown Aegis may still retry the whole node"
    requirement: "RT-05"
    verification:
      - kind: integration
        ref: "tests/integration/structured_engine_node_test.rs#repair_happens_inside_the_node"
        status: pass
      - kind: integration
        ref: "tests/integration/structured_engine_node_test.rs#exhaustion_becomes_a_node_error_with_unknown_transience"
        status: pass
      - kind: integration
        ref: "tests/integration/structured_engine_node_test.rs#a_transient_and_unknown_aegis_may_still_retry_the_node"
        status: pass
  - id: D7
    description: "A node without output_schema is unchanged -- same PaladinPort dispatch, same raw string written -- verified both at the engine-unit level (no structured executor wired at all) and end-to-end (a structured executor IS wired but ignored)"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#a_node_without_output_schema_is_unchanged"
        status: pass
      - kind: integration
        ref: "tests/integration/structured_engine_node_test.rs#a_node_without_output_schema_is_unchanged"
        status: pass
  - id: D8
    description: "StructuredDirective and output_schema share the same extract_json extraction machinery -- no second envelope-extraction implementation in the engine"
    requirement: "RT-05"
    verification:
      - kind: other
        ref: "tests/integration/structured_engine_node_test.rs#structured_directive_and_output_schema_share_extract_json (source-level assertion) plus grep -rc 'fn extract_envelope' crates/paladin-battalion/src == 0"
        status: pass
  human_judgment: false

duration: ~2h
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 18: Structured Output Wired Into the Engine — output_schema, Fail-Closed Validation, Fingerprint v6 Summary

**A `NodeSpec::Paladin` node can now declare `output_schema`, dispatching through the engine's structured executor to write a parsed JSON object — not a string — into its `output_field`, gated by four fail-closed `EngineError` variants and a `GRAPH_FINGERPRINT_VERSION` bump to `v6` re-pinned in the same commit.**

## Performance

- **Duration:** ~2h
- **Tasks:** 3 (Task 1 checkpoint auto-resolved; Task 2 and Task 3 executed)
- **Files modified:** 7 modified, 1 created

## Checkpoint resolutions

Task 1 (`checkpoint:decision`, `gate="blocking"`) was pre-resolved by the orchestrator under auto-mode: **auto-selected `fingerprint-v6-as-locked`** — proceed exactly as D-29 locks it (add `output_schema` to `NodeSpec::Paladin`, include it in `WarGraph::fingerprint()` sorted and length-prefixed, bump `GRAPH_FINGERPRINT_VERSION` from `v5` to `v6`, re-pin the golden fixture in the same commit). No further decision was required from this execution.

## Accomplishments

- `NodeSpec::Paladin.output_schema: Option<SchemaRef>` (new in 0.10, deliberate-zero note per D-29/D-37), constructor-preserved via `NodeSpec::paladin(..)` plus a chainable `with_output_schema`.
- Four distinct, offender-listing `EngineError` variants — `StructuredExecutorMissing`, `UnregisteredOutputSchema`, `OutputSchemaWithStructuredDirective`, `OutputSchemaFieldNotJson` — fire at validation, before any node runs. The executor-presence check (`validate_structured_executor_backend`) mirrors `validate_node_cache_backend`'s engine-config split; the other three live inside `WarGraph::validate` itself, consulting the new `EngineRegistries.output_schemas` registry.
- `WarEngine::with_structured_executor`/`with_output_schema`, `StructuredSchema` trait, and `TypedSchema<T>` (full typed validation by `serde_json::from_value::<T>`, carrying its JSON Schema as a plain value rather than deriving one via `schemars` — this crate has no `schemars` dependency, by design, D-26/ADR-0015).
- The structured dispatch branch in `engine::superstep`'s Paladin arm: when `output_schema` is set, the node calls `StructuredExecutorPort::execute_json_schema_observed` instead of the plain `PaladinPort` path, and the **parsed JSON value** is written to `output_field`. Exhaustion becomes a dedicated `NodeFailure::StructuredOutputInvalid` classified `Transience::Unknown` — deliberately never delegated to `PaladinError::transience()`'s general `Permanent` verdict for the same error variant, which is correct for 26-17's non-engine caller but wrong here (a `TransientAndUnknown` Aegis may still retry the whole node with a different prompt/model).
- `GRAPH_FINGERPRINT_VERSION` `v5` → `v6`: a new `;output_schemas:` fingerprint section, golden fixture and version-tag test re-pinned in the same commit as the new hash input.
- `tests/integration/structured_engine_node_test.rs`: a real `worker -> reader` two-node `WarGraph` proving typed agent-to-agent data flow end-to-end (Inline and Registered `SchemaRef`, in-node repair, exhaustion, Aegis retry, unchanged-path, and the single-envelope-extraction source assertion).

## Task Commits

1. **Task 2: output_schema on NodeSpec, the engine registries, and the fail-closed validation matrix** — `d17a505f` (feat)
2. **Task 3: The structured branch in superstep, and the end-to-end typed data-flow test** — `b093e8ef` (test)

Task 1's `checkpoint:decision` was auto-resolved by the orchestrator and produced no commit of its own.

## Files Created/Modified

- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec::Paladin.output_schema`, `NodeSpec::with_output_schema`, `validate_output_schemas`, `validate_structured_executor_backend` + `collect_output_schema_nodes`, the `;output_schemas:` fingerprint section, nine new/renamed tests (Tasks 2's Tests 1–9)
- `crates/paladin-battalion/src/engine/mod.rs` — `StructuredSchema` trait, `TypedSchema<T>`, `WarEngine::with_structured_executor`/`with_output_schema`, the `structured_executor` field threaded through `start`/`resume`/`resume_with_options`/`resume_with`/`fork`, four new `EngineError` variants
- `crates/paladin-battalion/src/engine/registries.rs` — `EngineRegistries.output_schemas: HashMap<String, Arc<dyn StructuredSchema>>`
- `crates/paladin-battalion/src/engine/superstep.rs` — `NodeDispatch::Paladin.output_schema` (resolved once before spawn), `ChildEngineResources.structured_executor`, `run`/`run_with_namespace` signature threading, `execute_vanguard_node`'s structured branch, `NodeFailure::StructuredOutputInvalid`, the `a_node_without_output_schema_is_unchanged` unit test
- `crates/paladin-battalion/src/llm_failure.rs` — `paladin_error_kind` names `PaladinError::StructuredOutputInvalid` as `"StructuredOutputInvalid"`
- `crates/paladin-core/src/platform/container/waypoint.rs` — `GRAPH_FINGERPRINT_VERSION` `v5` → `v6`, version-history rustdoc, two test literal updates
- `tests/integration/structured_engine_node_test.rs` (new) — the end-to-end acceptance suite
- `tests/integration/mod.rs` — registers the new test module

## Decisions Made

See `key-decisions` in the frontmatter for the full reasoning. In short: `TypedSchema<T>` takes an explicit JSON Schema value (no `schemars` dependency in this crate); JSON-compatibility for `output_field` is "not `DispatchRule::Sum`"; `StructuredSchema::validate()` is unit-tested but not wired into the engine's own repair loop (only `to_json_schema()` is consulted, keeping exactly one envelope-extraction/repair implementation per Task 3's own acceptance criterion); the structured branch does not thread `RunScope`/Vault confinement (the port trait has no such parameter); and the engine's `NodeFailure::StructuredOutputInvalid` deliberately diverges from `PaladinError::transience()`'s general `Permanent` verdict, hardcoding `Unknown`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Two `run(...)` call sites in `graph.rs`/`superstep.rs` test helpers used a variable (`std::time::Duration::from_secs(30)` / `shutdown_grace`) instead of the `default_shutdown_grace()` helper my bulk `perl` regex targeted**
- **Found during:** Task 2 verification (`cargo check --workspace`)
- **Issue:** Adding the new `structured_executor` trailing parameter to `superstep::run`/`run_with_namespace` broke every existing call site; a blanket regex substitution missed three call sites using a differently-named shutdown-grace expression
- **Fix:** Added the missing `None,` argument at each of the three sites individually
- **Files modified:** `crates/paladin-battalion/src/engine/graph.rs`, `crates/paladin-battalion/src/engine/superstep.rs`
- **Verification:** `cargo check --workspace --all-targets --all-features` clean
- **Committed in:** `d17a505f` (Task 2 commit)

**2. [Rule 1 - Bug] Two structured integration tests were written with an incorrect expectation of engine behavior, caught by their own first run**
- **Found during:** Task 3, first `cargo test --test lib structured_engine_node` run
- **Issue:** (a) `a_node_without_output_schema_is_unchanged` chained the plain-string-writing node into the shared `reader` `StateNode`, which fails closed on a non-object value — the run correctly failed, contradicting the test's own `RunOutcome::Completed` expectation. (b) `exhaustion_becomes_a_node_error_with_unknown_transience` asserted `error.node_error()` is `Some`, but per D-08/D-09 a NO-AEGIS node's failure deliberately stays on the byte-identical pre-Phase-25 generic path (`EngineError::Node`, no structured `NodeError`) — only a node WITH a resolved Aegis gets the structured error.
- **Fix:** (a) built a dedicated solo-`worker` graph for that test instead of reusing `build_graph`'s `reader`-chained fixture. (b) added `graph.set_aegis(NodeId::new("worker"), Aegis::default())` so the structured `NodeError` surfaces via the resolved-Aegis path, matching the documented D-08/D-09 discipline rather than fighting it.
- **Files modified:** `tests/integration/structured_engine_node_test.rs`
- **Verification:** all 8 tests in the file pass
- **Committed in:** `b093e8ef` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking compile-error, 1 test-correctness bug caught by the test's own first run against real engine semantics).
**Impact on plan:** Neither touched the plan's intended behavior; both are exactly the kind of "found during execution, fixed inline" issue the deviation rules exist for. No scope creep.

## Issues Encountered

None beyond the two documented deviations above. The Task 2/Task 3 split is sequential rather than fully independent: `NodeDispatch::Paladin`'s new `output_schema` field (added in Task 2, since it required updating `NodeSpec::Paladin`'s own struct shape in the same file/commit) needed a real dispatch consumer to remain meaningful, so the structured branch in `execute_vanguard_node` was implemented as part of Task 2's commit rather than deferred to a strict Task-3-only RED-then-GREEN pair. Task 3's own commit therefore reads as pure `test` (acceptance evidence for behavior that already existed), which is disclosed here rather than left implicit.

**House-convention note (not a deviation):** the plan's own `<verify>` blocks write `cargo test --test integration structured_engine_node`. This workspace has no `integration` test binary — `tests/integration/*.rs` are modules registered under the single `tests/lib.rs` binary (per this project's own execution rules) — so the equivalent, and actually-runnable, command is `cargo test --test lib structured_engine_node`. Both this SUMMARY's coverage block and the actual runs above use `--test lib`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `output_schema` (Inline and Registered `SchemaRef`) is now fully wired end-to-end: declare on a `NodeSpec::Paladin` node, register a `TypedSchema<T>` or rely on `Inline`, wire `WarEngine::with_structured_executor`, and a downstream node receives real structure.
- `GRAPH_FINGERPRINT_VERSION` is `v6`; no `v5:`-tagged fingerprint literal remains anywhere in the workspace (verified by grep across all `.rs` files).
- No blockers for later plans in this wave. `.project/v0.10.0/08-traceability-matrix.md`/`MIGRATION.md` deliberate-zero notes for the new `output_schema` field and the fingerprint bump are plan 26-21's job, per this plan's own D-29/D-37 framing, and are not addressed here.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

- All 8 files listed under `key-files` (created + modified) verified present on disk.
- Both task commits (`d17a505f`, `b093e8ef`) verified present in `git log --oneline --all`.
- `cargo check --workspace --all-targets --all-features` — clean.
- `cargo test -p paladin-battalion --lib` — 736 passed, 0 failed.
- `cargo test --test lib structured_engine_node` — 8 passed, 0 failed.
- `cargo fmt --all --check` — clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
- `grep -rn '"v5:'` across all `.rs` files in the workspace — 0 matches.
- `grep -c 'GRAPH_FINGERPRINT_VERSION: &str = "v6"' waypoint.rs` — 1; non-comment `"v5"` occurrences — 0.
