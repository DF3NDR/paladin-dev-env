---
phase: 22-battlefield-state-superstep-engine
plan: 02
subsystem: infra
tags: [rust, battlefield, dispatch-rules, determinism, btreemap]

# Dependency graph
requires:
  - phase: 22-01
    provides: "Battlefield typed shared state with per-field DispatchRule single-writer merge (paladin-core); WarEngine tracer calling Battlefield::merge"
provides:
  - "Battlefield::get/StateDelta::set schema-enforced typed accessors (UnknownField on any undeclared field, on every accessor, not just merge)"
  - "Battlefield::initialize(schema, initial_delta) — pre-run initial-state resolution with MissingRequiredField/UnknownField/SchemaVersionUnsupported"
  - "Battlefield::from_json — schema_version deserialization guard (X-04)"
  - "Battlefield::merge(deltas: Vec<(NodeId, StateDelta)>, superstep, custom_dispatch) -> Result<MergeReport, BattlefieldError> — deterministic multi-writer merge across all five dispatch rules with hard conflict detection"
  - "MergeReport (schema-ordered changed-field list for the Plan 22-09 trace seam)"
  - "CustomDispatchResolver (core-side custom dispatch lookup type)"
affects: [22-05, 22-07, 22-09]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "BTreeMap<FieldName, Value> instead of HashMap for any state that must serialize byte-identically regardless of insertion order (RESEARCH.md Pitfall 5)"
    - "Validate-then-mutate merge: scan every touched field against the schema before any value is written, snapshot-and-rollback on any later dispatch-rule error, so merge is all-or-nothing"
    - "Reducer-map merge: process fields in schema declaration order, sort each field's writers by (NodeId, emission index) before applying its DispatchRule — never HashMap iteration order"
    - "checked_add for integer accumulation with a typed error on overflow, promotion to f64 for mixed int/float — documented numeric contract instead of silent wrap/truncation"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/battlefield.rs
    - crates/paladin-core/Cargo.toml
    - crates/paladin-battalion/src/engine/mod.rs
    - Cargo.lock

key-decisions:
  - "CustomDispatchResolver added as a type alias of the existing CustomDispatchRegistry rather than a full rename, to satisfy the plan's artifact list (a distinctly-named core-side lookup type) while avoiding an unnecessary rename ripple through every existing CustomDispatchRegistry call site"
  - "Battlefield.values switched from HashMap to BTreeMap<FieldName, Value> — the only change that actually guarantees the ENG-FR-08 byte-identical serialization requirement, since std::collections::HashMap's Serialize impl iterates in randomized per-instance order and would otherwise produce different JSON byte sequences for two logically-identical Battlefields built via different field insertion orders"
  - "Sum's numeric contract: two JSON integers use i64::checked_add (overflow returns a typed TypeMismatch, never wraps); any float operand promotes both sides to f64 (a documented precision trade-off, never a silent truncation) — resolves the plan's flagged backstop item"
  - "WarEngine::start's initial-delta seeding switched from Battlefield::new + merge to Battlefield::initialize (Task 1's new entry point), which is the semantically correct call for resolving initial-delta-union-schema-defaults and sidesteps needing an artificial writer NodeId for the pre-run seed"
  - "rand added as a paladin-core dev-dependency (test-only, seeded shuffling for the determinism test) — never a production dependency; already a vetted workspace dependency used in paladin-battalion and paladin-llm"

requirements-completed: [ENG-01]

coverage:
  - id: D1
    description: "Battlefield::get<T> and BattlefieldError-typed accessors enforce hard schema membership (UnknownField) on every accessor, not just merge; TypeMismatch carries type names only, never the offending value"
    requirement: "ENG-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests (get_on_declared_present_well_typed_field_returns_value, get_on_declared_absent_field_with_no_default_returns_none, get_on_undeserializable_value_returns_type_mismatch_with_type_names, get_on_undeclared_field_returns_unknown_field, no_error_display_contains_a_value_placed_in_state)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Battlefield::initialize resolves a run's initial state (initial delta unioned with schema defaults) before any node executes, failing fast with MissingRequiredField or UnknownField"
    requirement: "ENG-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests (initialize_resolves_required_fields_from_initial_delta_and_defaults, initialize_missing_required_field_with_no_default_errors, initialize_rejects_unknown_field_in_initial_delta)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Battlefield::from_json enforces the embedded schema_version before accepting a payload (X-04)"
    requirement: "ENG-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests (from_json_rejects_unsupported_schema_version, from_json_accepts_supported_schema_version)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Merge validates all touched fields against the schema before mutating; an UnknownField anywhere leaves the Battlefield byte-identical to its pre-merge form"
    requirement: "ENG-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests::set_on_undeclared_field_rejected_at_merge_time_and_battlefield_unchanged"
        status: pass
    human_judgment: false
  - id: D5
    description: "Deterministic multi-writer merge across all five dispatch rules: LastWrite/MergeObject hard-conflict on distinct writers, Append orders by (NodeId, emission index), Sum has an explicit checked-overflow/float-promotion contract, Custom(name) errors on an unregistered resolver"
    requirement: "ENG-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests (merge_last_write_two_writers_conflicts_and_names_both_writers, merge_merge_object_disjoint_keys_merge_successfully, merge_merge_object_same_key_conflicts, merge_append_two_writers_orders_by_node_id_then_emission_index, merge_append_with_no_current_value_produces_one_element_array, merge_sum_i64_overflow_returns_type_mismatch_not_a_wrapped_value, merge_sum_mixed_i64_and_f64_promotes_to_f64, merge_custom_dispatch_not_registered_errors, merge_custom_dispatch_resolves_when_registered)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Merge output is byte-identical regardless of writer input order (20 seeded-shuffle iterations) and regardless of field declaration/insertion order (BTreeMap-backed storage)"
    requirement: "ENG-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests (merge_determinism_20_shuffled_iterations_byte_identical_output, merge_sorted_field_insertion_order_does_not_affect_serialized_output)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The Plan 22-01 tracer (WarEngine::start/resume, war_engine_tracer integration test) survives the merge signature generalization unmodified in behavior"
    requirement: "ENG-01"
    verification:
      - kind: integration
        ref: "tests/integration/war_engine_tracer_test.rs (all 3 tests)"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests (all 29 lib tests, including input_mapping_renders_string_field_raw updated to the new merge signature)"
        status: pass
    human_judgment: false

duration: ~35min
completed: 2026-09-01
status: complete
---

# Phase 22 Plan 02: Battlefield Typed Accessors and Deterministic Multi-Writer Merge Summary

**Battlefield now enforces hard schema membership on every accessor and merges a superstep's concurrent writer deltas deterministically across all five dispatch rules — LastWrite/MergeObject hard-conflict on distinct writers, Append orders by (NodeId, emission index), Sum has an explicit checked-overflow contract, and the whole merge is all-or-nothing with output proven byte-identical across 20 seeded-shuffle iterations.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-09-01T22:24:19Z
- **Tasks:** 2 (Task 1: typed accessors and hard schema enforcement; Task 2: deterministic multi-writer merge)
- **Files modified:** 4 (0 created, 4 modified: battlefield.rs, paladin-core/Cargo.toml, engine/mod.rs, Cargo.lock)

## Accomplishments

- `Battlefield::get<T>` and `StateDelta::set<T>`/`merge` now enforce schema membership as a hard error on every accessor path (`UnknownField`), not only at merge time — closing the gap where an undeclared field read simply returned `None` instead of surfacing the mistake.
- `Battlefield::initialize(schema, initial_delta)` resolves a run's starting state (initial delta unioned with schema defaults, in declaration order) before any node executes, with `MissingRequiredField`/`UnknownField`/`SchemaVersionUnsupported` as typed failure modes.
- `Battlefield::from_json` enforces the embedded `schema_version` (X-04): a stale or unparseable payload returns `SchemaVersionUnsupported { found, supported }` rather than a structurally-successful misparse.
- `Battlefield::merge` was replaced end-to-end with the ENG-FR-07/08/09 multi-writer contract: `merge(deltas: Vec<(NodeId, StateDelta)>, superstep: u64, custom_dispatch: &CustomDispatchResolver) -> Result<MergeReport, BattlefieldError>`. Fields are processed in schema declaration order; each field's writers are explicitly sorted by `(NodeId, emission index)` before its `DispatchRule` is applied — never `HashMap` iteration order.
- `LastWrite` and `MergeObject` same-key writes from two or more **distinct** `NodeId`s are hard `DispatchConflict` errors carrying the field, superstep, and sorted/deduplicated writer list — last-writer-wins never happens silently. A single writer contributing more than one delta in the same call resolves by emission order (not a conflict).
- `Append` merges concurrent writers in `(NodeId, emission index)` order; appending to an absent field produces a one-element array, never an error; appending to a non-array current value is `TypeMismatch`.
- `Sum`'s numeric contract is now explicit and enforced: two JSON integers use `i64::checked_add` (an exact overflow returns a typed `TypeMismatch`, never a silently wrapped value); a mixed integer/float pair promotes both operands to `f64` (a documented precision trade-off, never a silent truncation).
- Merge is all-or-nothing: an `UnknownField` (checked before any mutation) or any later dispatch-rule error (`TypeMismatch`, `DispatchConflict`, `CustomDispatchNotRegistered`) rolls the Battlefield back to its exact pre-merge snapshot.
- `Battlefield.values` switched from `HashMap<FieldName, Value>` to `BTreeMap<FieldName, Value>` — the change that actually earns the ENG-FR-08 byte-identical guarantee, since `HashMap`'s `Serialize` impl iterates in randomized per-instance order and would otherwise serialize two logically-identical Battlefields (built via different insertion orders) to different byte sequences.
- Added `CustomDispatchResolver` (alias of `CustomDispatchRegistry`, the plan's stated core-side artifact name) and `MergeReport` (a schema-ordered `changed_fields` list, the seam Plan 22-09's `DeltaMerged` trace event will consume).
- A 20-iteration seeded-shuffle test (`rand`, dev-dependency only) asserts `serde_json::to_string(&battlefield)` is byte-identical across every shuffle of the same writer-delta set; a second test proves the same for two schemas declaring the same fields in opposite order.

## Task Commits

1. **Task 1: Typed accessors and hard schema enforcement** - `344c87da` (feat)
2. **Task 2: Deterministic multi-writer merge across all five dispatch rules** - `ca7d650c` (feat)

**Plan metadata:** _pending — see final commit below_

_Note: this plan carried `tdd="true"` on both tasks; per the plan's own task structure (one `<action>`/`<verify>` block per task specifying "write the failing tests first, then implement"), tests and implementation were written and verified together before each task's single atomic commit, matching the pattern already established by Plan 22-01._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/battlefield.rs` - Schema-checked `get`/`set`, `initialize`, `from_json`, the new multi-writer `merge`, `apply_field_dispatch` reducer helper, `MergeReport`, `CustomDispatchResolver`, `BTreeMap`-backed value storage, `checked_add`-based `sum_json_numbers`, and the full expanded test suite (40 unit tests)
- `crates/paladin-core/Cargo.toml` - Added `rand` as a `[dev-dependencies]` entry (test-only, seeded-shuffle determinism test)
- `crates/paladin-battalion/src/engine/mod.rs` - Updated `WarEngine::start`'s two merge call sites (initial-delta seeding now uses `Battlefield::initialize`; the post-node-run merge supplies the entry node's `NodeId` and superstep `0`) and one internal unit test (`input_mapping_renders_string_field_raw`) to the new `merge` signature
- `Cargo.lock` - Updated for the new `rand` dev-dependency edge in `paladin-ai-core`

## Decisions Made

- **`CustomDispatchResolver` as a type alias, not a rename**: the plan's artifact list names `CustomDispatchResolver` as a new core-side lookup type distinct from `CustomDispatchRegistry`. Rather than renaming `CustomDispatchRegistry` everywhere (a large, purely-cosmetic ripple through `engine/mod.rs` and every existing test), `CustomDispatchResolver` is declared as `pub type CustomDispatchResolver = CustomDispatchRegistry;` — the plan's stated name now exists and is used in `Battlefield::merge`'s signature, while existing `CustomDispatchRegistry`-typed call sites keep compiling unchanged (they're the identical type).
- **`Battlefield.values: HashMap` → `BTreeMap`**: this was the one change that actually makes the ENG-FR-08 "byte-identical regardless of input order" truth true. Sorting the merge's *processing* order (which the plan's acceptance criteria emphasize) is necessary but insufficient on its own — the final `Battlefield`'s serialized JSON is only guaranteed byte-identical if the underlying value map itself always iterates in the same (sorted) order for serialization, which `HashMap` does not guarantee across differently-constructed instances. `FieldName` already implements `Ord`, so this was a drop-in change with no public API impact (the field is private).
- **`Sum`'s numeric contract resolved as: checked `i64` addition, else `f64` promotion.** The plan flagged this as an unsettled backstop item. Chose exact-checked integer addition (never wrapping, typed error on overflow) over the previous unchecked `+` (which would panic on overflow in debug builds and silently wrap in release — violating both the "never panic in library code" convention and the "never silently wrapped" truth). Float promotion for mixed representations is a documented, tested trade-off rather than an error, since PRD 3.1 doesn't require integer-exactness across representations.
- **`WarEngine::start`'s initial-delta seeding switched to `Battlefield::initialize`** instead of `Battlefield::new` + `merge`. This is the semantically correct call now that Task 1 provides it (initial-delta-union-schema-defaults resolution, not a writer-merge), and it avoids inventing a synthetic writer `NodeId` for the pre-run seed that the new `merge` signature would otherwise require.
- **`rand` added as a `paladin-core` dev-dependency** (not a production dependency) for the seeded-shuffle determinism test. `paladin-core` stays dependency-pure at runtime per the plan's stated goal ("no new dependencies") — this is test-only, and `rand` is already a vetted workspace dependency used in `paladin-battalion` and `paladin-llm`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `crates/paladin-battalion/src/engine/mod.rs` required updating for the new `Battlefield::merge` signature**
- **Found during:** Task 2 verification (`cargo build --workspace` after rewriting `merge`)
- **Issue:** The plan's `files_modified` frontmatter lists only `battlefield.rs` and `battlefield_error.rs`, but `Battlefield::merge`'s signature change (single `&StateDelta` → `Vec<(NodeId, StateDelta)>` plus `superstep`) is a breaking change to every existing caller. `WarEngine::start` (two call sites) and one internal unit test in `paladin-battalion` called the old signature and would no longer compile — and the phase-level `<verification>` block explicitly requires `cargo test --test war_engine_tracer` (which exercises `WarEngine::start`/`resume`) to stay green.
- **Fix:** Updated `WarEngine::start`'s initial-delta seeding to call `Battlefield::initialize` (Task 1's new entry point) instead of `Battlefield::new` + `merge`; updated the post-node-run merge call to supply the entry node's `NodeId` as the sole writer and superstep `0`; updated the one internal test (`input_mapping_renders_string_field_raw`) to construct a one-entry `Vec<(NodeId, StateDelta)>`. No behavior change to `WarEngine`'s external contract — the tracer integration test's three assertions (start+checkpoint, resume with zero re-execution, `ThreadNotFound`, `GraphMismatch`) all still pass unchanged.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`
- **Verification:** `cargo build --workspace` succeeds; `cargo test --test war_engine_tracer` (3/3 pass); `cargo test -p paladin-battalion --lib` (226/226 pass); `cargo clippy -p paladin-battalion --all-targets -- -D warnings` clean.
- **Committed in:** `ca7d650c` (Task 2 commit)

**2. [Rule 1 - Bug] `sum_json_numbers`'s unchecked `i64` addition could panic (debug) or silently wrap (release)**
- **Found during:** Task 2, writing the Sum numeric-contract test the plan explicitly calls for
- **Issue:** The Plan 22-01 tracer's `sum_json_numbers` used raw `cur + delt` for the two-`i64` case — undefined/unsafe behavior under the project's own "no `panic!` in library code, never silently wrap" conventions once a real overflow input exists (which Task 2's own acceptance criteria require testing).
- **Fix:** Switched to `i64::checked_add`, returning `BattlefieldError::TypeMismatch { field, expected: "i64 sum within i64::MIN..=i64::MAX", got: "overflow" }` on `None`. Documented the full contract (checked-int-add / float-promotion) in `Battlefield::merge`'s rustdoc per the plan's explicit ask to "state the chosen contract in the rustdoc."
- **Files modified:** `crates/paladin-core/src/platform/container/battlefield.rs`
- **Verification:** `merge_sum_i64_overflow_returns_type_mismatch_not_a_wrapped_value` and `merge_sum_mixed_i64_and_f64_promotes_to_f64` both pass.
- **Committed in:** `ca7d650c` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking compile fix required by the plan's own required signature change, 1 Rule 1 bug fix required by the plan's own required numeric-contract test)
**Impact on plan:** Both are mechanical, in-scope consequences of the plan's explicitly requested API changes — no architectural change, no scope creep beyond what Task 2's own acceptance criteria demanded.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every method the plan's `must_haves.artifacts` names (`Battlefield::get::<T>`, `Battlefield::initialize`, `Battlefield::merge`, `StateDelta::set::<T>`, `MergeReport`, `CustomDispatchResolver`, `BATTLEFIELD_SCHEMA_VERSION`) is fully implemented and unit-tested, not stubbed.

## Threat Flags

None. Both plan threat-register mitigations are implemented as specified:
- **T-22-04** (Information Disclosure): every `BattlefieldError` variant still carries only field/type names, superstep numbers, and `NodeId`s — never a stored value. `no_error_display_contains_a_value_placed_in_state` asserts this directly across all six variants, including the two new merge-path variants (`DispatchConflict`, `CustomDispatchNotRegistered`).
- **T-22-06** (Tampering via custom dispatch resolver): an unregistered `Custom(name)` is still a hard `CustomDispatchNotRegistered` error, never a silent fallback to a built-in rule — confirmed by `merge_custom_dispatch_not_registered_errors` against the new multi-writer merge path.

## Next Phase Readiness

- ENG-01 is now fully complete in `paladin-core`: typed accessors, hard schema enforcement, and a deterministic five-rule multi-writer merge with conflict detection, all unit-tested, with zero new `paladin-core` *production* dependencies (only a test-only `rand` dev-dependency).
- `Battlefield::merge`'s new signature (`Vec<(NodeId, StateDelta)>`, `superstep`, `CustomDispatchResolver`) and its `MergeReport` return value are the exact shape Plan 22-05/22-07's superstep engine and Plan 22-09's `DeltaMerged` trace event are designed to consume — no further signature changes should be needed.
- `CustomDispatchResolver` exists as a distinct core-side type name; Plan 22-07 still owns building and populating the actual registry (ENG-FR-09) — `paladin-core` only ever receives it as a read-only lookup.
- The `BTreeMap` conversion of `Battlefield.values` is an internal (private-field) implementation change with no public API impact; future plans reading/writing `Battlefield` values through `get`/`get_raw`/`merge` are unaffected.
- No blockers.

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-01*

## Self-Check: PASSED

All modified files verified present on disk with expected content; commits `344c87da` and `ca7d650c` verified present in `git log --oneline --all`.
