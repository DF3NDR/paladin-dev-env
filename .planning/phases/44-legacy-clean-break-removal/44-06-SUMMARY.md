---
phase: 44-legacy-clean-break-removal
plan: 06
subsystem: battalion-orchestration
tags: [aegis, node-errors, heralds, one-way-door, serde, clean-break]
status: complete

requires:
  - phase: 44-04
    provides: Formation on the runner; AttemptFailure.node_error
  - phase: 44-05
    provides: Phalanx on the runner; typed failures in declaration order
provides:
  - BattalionResult.node_errors retyped to the structured node_error::NodeError (D-06, one-way)
  - Formation (Absorb) and Phalanx record the runner's AttemptFailure.node_error unchanged
  - JSON, Markdown and Table heralds render the structured element (D-08)
  - tests pinning the persisted shape, the strict v0.10 rejection and the exact herald lines
  - ADR-0059 (g) records the checkpoint outcome
affects: [44-07, 44-10, 44-11, 44-12, 44-13]

tech-stack:
  added: []
  patterns:
    - "Persisted shape pinned with a raw serde_json::to_string comparison (a Value would alphabetise keys)"
    - "Herald line = node id + attempt + transience around the source's own display; no provider or status"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/battalion/mod.rs
    - crates/paladin-battalion/src/formation_service.rs
    - crates/paladin-battalion/src/phalanx_service.rs
    - crates/paladin-herald/src/json_herald.rs
    - crates/paladin-herald/src/markdown_herald.rs
    - crates/paladin-herald/src/table_herald.rs
    - tests/integration/battalion_herald_end_to_end_test.rs
    - tests/integration/commander_error_paths_test.rs
    - .planning/decisions/0059-legacy-clean-break-removal.md

key-decisions:
  - "Task 1 checkpoint outcome: option-a, answered 2026-10-11 -- node_id is the Paladin's name (PaladinData.name, the paladin_name the runner already passes); no runner change"
  - "No compatibility deserializer, untagged enum or alias for the v0.10 element (D-00c); #[serde(default)] kept so empty or absent node_errors still reads"
  - "Operator context (recorded, not acted on): a later phase 44.1 is under consideration that renames the Node<T> entity envelope to Entity<T> to remove the overload with graph-vertex NodeId; it does not change this decision and is out of scope here"

patterns-established:
  - "Information minimisation in herald lines (T-44-15), proved by herald_node_error_line_adds_no_provider_detail"

requirements-completed: []
requirements-advanced: [LEGACY-01]  # REQUIREMENTS.md left unchecked; the removal plans (up to 44-10) finish it

duration: ~1h
completed: 2026-10-11
---

# Phase 44 Plan 06: node_errors retype Summary

**`BattalionResult.node_errors` is now `Vec<node_error::NodeError>`: Formation and Phalanx push the runner's typed failure unchanged, the three heralds render node id, attempt and transience beside the source's own display, and the one-way wire shape and the strict v0.10 rejection are pinned by tests.**

## Task 1: persisted wire shape checkpoint (decision, blocking)

- **Operator response:** `option-a` -- approve the proposed shape as written; `node_id` is the Paladin's name (`PaladinData.name`, i.e. what the runner already passes as `paladin_name`). No runner change.
- **Answer date:** 2026-10-11
- Approved element: `{"node_id": "<Paladin name>", "attempt": <1-indexed attempt that produced the final failure>, "transience": "Transient"|"Permanent"|"Unknown", "source": {"Llm": {...}} | {"Paladin": {...}} | {"Timeout": "Run"|"Idle"}}`.
- Pre-retype re-check of assumption A1-44-06: `git grep -n '"node_name"'` found only the two test assertions (`json_herald.rs:461`, `battalion_herald_end_to_end_test.rs:357`, both retyped here) and planning documents; no stored v0.10 JSON in the repository.

## Accomplishments

- **Task 2 (D-06, D-07, D-08), commit `ad7e28f3`:** the core field is retyped (rustdoc rewritten; `BattalionError::Node` rustdoc no longer contrasts itself with a result-carried summary). Formation's Absorb branch pushes `failure.node_error.clone()` (the original still feeds `BattalionError::Node` for the aggregate); Phalanx maps `failures` to their `node_error`s in declaration order. Markdown renders `**<node id>:** attempt <n>, <Transience>: <source>`, Table renders `  <node id> (attempt <n>, <Transience>): <source>` (the `HeraldError::SerializationError` mapping is kept), JSON needed no serializer change. In-file and root integration tests now assert `node_id`, `attempt` and the `source` variant. `src/application/cli/formatters/output.rs` is unchanged (`git diff --quiet b362d6cb` exits 0) and still compiles.
- **Task 3 (D-06, D-08, T-44-15, T-44-16), commit `b0cdd815`:** seven named tests added (see below) and the ADR-0059 (g) sentence `Checkpoint outcome (44-06, 2026-10-11): option-a ...` appended to the D-06 paragraph.

| Test | Where | Pins |
| ---- | ----- | ---- |
| `battalion_result_node_errors_serialize_as_structured_node_errors` | core `battalion/mod.rs` | exact JSON string, key order `node_id, attempt, transience, source`, Paladin and Timeout sources, round trip |
| `battalion_result_rejects_v0_10_node_errors_shape` | core | `{"node_name","error"}` element fails to deserialize |
| `battalion_result_accepts_empty_or_absent_node_errors` | core | `[]` and a missing key both read as empty |
| `json_herald_serialises_structured_node_errors` | json_herald | `node_id`, `attempt`, `transience`, object-valued `source`; v0.10 keys absent |
| `markdown_herald_renders_structured_node_error_line` | markdown_herald | exact line `**Scout:** attempt 2, Transient: run timeout` |
| `table_herald_renders_structured_node_error_line` | table_herald | exact line `  Scout (attempt 2, Transient): run timeout` |
| `herald_node_error_line_adds_no_provider_detail` | markdown_herald (table half under `feature = "table"`) | `Llm` source with provider `openai`, status 401: line has the message, neither `openai` nor `401` |

## Deviations from Plan

**1. [Rule 3 - Blocking] Table herald is feature-gated.** `paladin-herald` builds `table_herald` only with `--features table` (default is empty), so the plan's `cargo test -p paladin-herald` does not run the Table tests. `herald_node_error_line_adds_no_provider_detail` therefore covers Table through a `#[cfg(feature = "table")]` block, and the Table tests were verified with `cargo test -p paladin-herald --all-features` (93 passed, 0 failed) in addition to the default run (60 passed). No production code is affected.

**2. [Ordering note] Task 3 tests were written after the Task 2 retype**, not before it, because Task 2 had to change the type for the workspace to compile in one commit (the plan's own split). The RED-before-GREEN step is therefore not separately demonstrable; the tests were run green against the implemented shape and the rejection test asserts `is_err()` on the v0.10 element.

**3. [Attribution] The first Task 2 commit carried the wrong attribution trailer** (the model name from the task brief rather than the harness attribution reminder); it was amended to the harness-specified trailer before any further work (local, unpushed). Hash `ad7e28f3` is the amended commit.

No Rule 1/2/4 deviations. No auth gates.

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo test -p paladin-ai-core --lib battalion` / `battalion_result` | 111 passed / 9 passed, 0 failed |
| `cargo test -p paladin-battalion --lib -- formation_service phalanx_service` | 35 passed, 0 failed |
| `cargo test -p paladin-herald` / `--all-features` | 60 passed / 93 passed, 0 failed |
| `cargo test -p paladin-ai --features cli --test lib -- integration::battalion_herald_end_to_end_test integration::commander_error_paths_test` | 6 passed |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `make doc-check` | passed |
| `make api-surface` | "API surface unchanged" |
| `cargo test --workspace --no-fail-fast` | all targets pass except `paladin-ai` lib: 1348 passed, **2 failed** -- only `run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` (known pre-existing, `deferred-items.md`, fail on the pre-phase baseline); by the plan brief a run whose only failures are those counts as passing |

Acceptance greps: the `pub node_errors: Vec<crate::platform::container::node_error::NodeError>` line is present once; `git grep node_name` over the herald crate, both services and both root tests prints nothing; `serde(untagged)|serde(alias` count in `battalion/mod.rs` is 0; the ADR marker grep prints 1.

## Known Stubs

None. (The legacy summary struct `battalion::NodeError` and its test `legacy_battalion_node_error_summary_is_unchanged` remain defined and now unused on purpose; 44-10 deletes them. Its rustdoc still says it is "carried on `BattalionResult`", which is stale until then.)

## Threat Flags

None. The richer herald line is the T-44-15 surface, covered by the information-minimisation test; T-44-16 by the strict-rejection test; T-44-17 by the checkpoint, this summary and ADR-0059 (g).

## Commits

1. `ad7e28f3` refactor(44-06): retype BattalionResult.node_errors to the structured NodeError
2. `b0cdd815` test(44-06): pin the node_errors wire shape and the herald lines

## Self-Check: PASSED

All nine listed files exist and are modified; both commits are present in `git log`; the ADR marker line is present.
