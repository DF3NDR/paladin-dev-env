---
phase: 48-base-entity-rename
plan: 04
subsystem: core-base
tags: [rename, entity, citadel, d-10, d-13, serde-compat, clean-break, rustdoc]

requires:
  - phase: 48-01
    provides: "entity_serde_compat.rs harness, pre_rename fixtures and the FileCitadel tracer test"
  - phase: 48-02
    provides: "ADR-0060, the licence for the clean break"
  - phase: 48-03
    provides: "EntityVersion family; entity_version_service.rs, which this plan's Stage A sweep carried"
provides:
  - "paladin_core::base::entity::Entity<T> in the private module base/entity/wrapper.rs, re-exported by pub use wrapper::Entity;"
  - "The citadel module declares no PaladinData, PaladinStatus or Paladin alias; PaladinState.paladin is the real platform::container::paladin::Paladin"
  - "Test v0_10_citadel_statuses_load_into_the_unified_status (every v0.10 status loads; Reasoning serializes as itself)"
  - "No Rust comment or rustdoc calls the base wrapper a Node; the base module rustdoc links Entity<T>"
affects: [48-05, 48-06, 48-07, 48-08, 48-09]

tech-stack:
  added: []
  patterns:
    - "Private wrapper module re-exported under the type's name (avoids clippy::module_inception)"
    - "Scripted rename with lookbehind that keeps graph-sense Node tokens, verified by classifying every changed file against a re-run of the script on HEAD blobs"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/base/entity/wrapper.rs
    - crates/paladin-core/src/base/entity/mod.rs
    - crates/paladin-core/src/platform/container/citadel.rs
    - crates/paladin-core/tests/entity_serde_compat.rs
    - crates/paladin-ports/src/output/citadel_port.rs
    - crates/paladin-memory/src/citadel/file_citadel.rs
    - src/application/services/paladin/paladin_builder.rs
    - tests/integration/citadel_integration_test.rs
  removed:
    - crates/paladin-core/src/base/entity/node.rs (git mv to wrapper.rs, history followed)

key-decisions:
  - "Stage A landed as ONE commit: the scripted pass alone compiled, but was not cargo fmt clean (three files), and a commit that fails the project's own fmt gate is not green; the plan allows the split only where the build allows it and every commit stays green"
  - "No compile-guided residual fixes were needed: 48-03 had already moved the version service and content_service onto the new names, so the perl passes produced a building tree"
  - "The status test builds its Reasoning case from the fixture rather than naming the wrapper, so the harness needs no edit when Stage A renames the wrapper (the tests directory is byte-identical across Stage A)"
  - "Intra-doc link in base/mod.rs is crate::base::entity::Entity; the plan's relative entity::Entity does not resolve from a module's own inner doc"

patterns-established:
  - "Compat harness additions are strictly additive (40 added, 0 deleted lines), fixtures untouched"

requirements-completed: [ENTITY-01, ENTITY-02, ENTITY-03]

coverage:
  - id: D1
    description: "D-10: citadel placeholder PaladinData, PaladinStatus and Paladin alias removed; PaladinState.paladin and PaladinState::new use the real Paladin; no re-export left in citadel"
    requirement: "ENTITY-03"
    verification:
      - kind: other
        ref: "grep -cE 'pub (struct PaladinData|enum PaladinStatus|type Paladin\\b)' citadel.rs prints 0; git grep 'pub type Paladin\\s*=' prints exactly paladin.rs:229"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-ai-core --test entity_serde_compat (6 passed, includes pre_rename_citadel_json_still_loads unedited)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-memory --lib citadel (16 passed, includes pre_rename_citadel_snapshot_loads_through_file_citadel unedited)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every v0.10 status encoding loads into the unified PaladinStatus and writes back unchanged; Reasoning (v0.11 only) serializes as Reasoning"
    requirement: "ENTITY-01"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-ai-core --test entity_serde_compat v0_10_citadel_statuses_load_into_the_unified_status"
        status: pass
    human_judgment: false
  - id: D3
    description: "Entity<T> replaces Node<T> at base::entity::Entity with no alias, no deprecated item, no re-export of the old name or path; the wire did not move"
    requirement: "ENTITY-02"
    verification:
      - kind: other
        ref: "git grep 'base::entity::node' and the Node(<|::new|{) boundary grep print nothing; git grep '#[deprecated' counts 0; git diff <PRE_A> HEAD -- crates/paladin-core/tests is empty"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-ai-core (709 lib, 6 compat, 140 doc) and cargo test -p paladin-ai --test legacy_removal_guard"
        status: pass
    human_judgment: false
  - id: D4
    description: "ENTITY-05 prose: no Rust comment calls the wrapper a Node; graph prose untouched; retained base files changed prose only (D-05)"
    requirement: "ENTITY-05"
    verification:
      - kind: other
        ref: "make doc-check; prose phrase grep prints nothing; D-05 item-line diff gate prints 0; Node-cache key composition still present in cache_key.rs and engine/mod.rs"
        status: pass
    human_judgment: false

duration: about 1h
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 04: Citadel unification and the Node to Entity type rename Summary

**The base wrapper is now `paladin_core::base::entity::Entity<T>` everywhere with no trace of the old type or module, the Citadel stores the real `Paladin` instead of its own placeholder, and a captured v0.10 snapshot, including each v0.10 status, still loads byte for byte.**

## Accomplishments

- D-10: deleted the citadel module's placeholder `PaladinData`, `PaladinStatus` and `Paladin` alias. `PaladinState.paladin` and `PaladinState::new` take `super::paladin::Paladin`. Exactly one `pub type Paladin =` remains in the workspace, in `paladin.rs`.
- Stage A: `git mv` of `node.rs` to `wrapper.rs`, `mod.rs` reduced to `pub mod collection; pub mod field; pub mod message; mod wrapper; pub use wrapper::Entity;`, and a scripted type and path rename across 120 files. The payload field is still `node` (48-05).
- Prose: the `base` rustdoc links `Entity<T>`, the crate docs list `Entity<T>`, and every comment, rustdoc line and test string that called the wrapper a Node now says Entity. Graph-sense prose (`Node ids`, `Node-cache`, `Node execution surface`, `BattalionError::Node`, `EngineError::Node`) is untouched.

## Task Commits

1. **Task 1: citadel placeholder unification (D-10)** - `b82bcc8` (refactor)
2. **Task 2: Stage A, Node<T> to Entity<T>** - `c6dfc93` (refactor)
3. **Task 3: Entity wording in Rust prose** - `bd767f7` (docs)

**Plan metadata:** the commit carrying this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md (docs: complete plan)

## Measured placeholder importers (Task 1 step 1)

The multi-line grep and the single-line grep agree. Files that named the citadel placeholder types:

- `crates/paladin-ports/src/output/citadel_port.rs` (doc example and tests module)
- `crates/paladin-memory/src/citadel/file_citadel.rs` (tests module)
- `src/application/services/paladin/paladin_builder.rs` (test mock, full `crate::core::...::citadel::` path)
- `tests/integration/citadel_integration_test.rs`

That is four files plus `citadel.rs` itself, not CONTEXT D-10's five: `examples/battalion_checkpoint_recovery.rs` imports neither type (it uses the builder and `PaladinState` only), confirmed by grep (D-00a, the tree outranks the document). The compat harness `entity_serde_compat.rs` names only `PaladinState`, as 48-01 designed, and gained one test.

## Stage A commit structure and green evidence

Stage A is one commit, `c6dfc93`. The scripted pass (git move, mod.rs wiring, perl passes) compiled on its own, so there was no compile-guided residual work; but `cargo fmt --all --check` reported diffs in `superstep.rs`, `log.rs` and `paladin_port.rs` (the longer `Entity` path changed line wrapping and import order). A scripted-only commit would have failed the project's formatting gate, so formatting and two article fixes ("a `Entity`" to "an `Entity`" in `graph.rs` and `paladin.rs`) went into the same commit. Evidence:

- Task 1 commit `b82bcc8`: `cargo check --workspace --all-targets --all-features` exit 0, compat 6 passed, memory citadel 16 passed, citadel_integration 11 passed, clippy on core/ports/memory clean.
- Stage A `c6dfc93`: `cargo check --workspace --all-targets --all-features` exit 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo test -p paladin-ai-core` 709 lib, 6 compat, 140 doc passed; tracer `pre_rename_citadel_snapshot_loads_through_file_citadel` passed; `legacy_removal_guard` 3 passed; `make check-doc-examples` exit 0.
- Task 3 `bd767f7`: `cargo check --workspace --all-targets --all-features` exit 0, `make doc-check` exit 0, `cargo test -p paladin-ai-core` passed.

Starting scope: `git grep -l 'base::entity::node' -- '*.rs'` listed 117 files before Stage A and 0 after.

## Rewritten files

120 files in `git diff --name-only b82bcc8 c6dfc93`: 112 `scripted` (byte-identical to a re-run of the perl passes on the pre-Stage-A blobs, checked mechanically), 8 `hand-fix`. The hand-fix tag covers the four hand-authored files (`mod.rs`, `wrapper.rs` test renames, `lib.rs` bullet, and `graph.rs`/`paladin.rs` article fixes) and the three files `cargo fmt` reflowed. `crates/paladin-core/tests/` is byte-identical across Stage A.

- `crates/paladin-battalion/src/engine/graph.rs` (hand-fix)
- `crates/paladin-battalion/src/engine/superstep.rs` (hand-fix)
- `crates/paladin-core/src/base/entity/mod.rs` (hand-fix)
- `crates/paladin-core/src/base/entity/wrapper.rs` (hand-fix)
- `crates/paladin-core/src/lib.rs` (hand-fix)
- `crates/paladin-core/src/platform/container/log.rs` (hand-fix)
- `crates/paladin-core/src/platform/container/paladin.rs` (hand-fix)
- `crates/paladin-ports/src/output/paladin_port.rs` (hand-fix)
- `crates/doc-examples/src/support.rs` (scripted)
- `crates/paladin-battalion/benches/battalion_benchmarks.rs` (scripted)
- `crates/paladin-battalion/src/commander.rs` (scripted)
- `crates/paladin-battalion/src/conclave_execution_service.rs` (scripted)
- `crates/paladin-battalion/src/council_service.rs` (scripted)
- `crates/paladin-battalion/src/engine/bridges.rs` (scripted)
- `crates/paladin-battalion/src/engine/cache_key.rs` (scripted)
- `crates/paladin-battalion/src/engine/export/mod.rs` (scripted)
- `crates/paladin-battalion/src/engine/export/shape.rs` (scripted)
- `crates/paladin-battalion/src/engine/graph_doc.rs` (scripted)
- `crates/paladin-battalion/src/engine/mod.rs` (scripted)
- `crates/paladin-battalion/src/formation_service.rs` (scripted)
- `crates/paladin-battalion/src/grove_service.rs` (scripted)
- `crates/paladin-battalion/src/in_memory_registry.rs` (scripted)
- `crates/paladin-battalion/src/phalanx_service.rs` (scripted)
- `crates/paladin-battalion/tests/export_golden.rs` (scripted)
- `crates/paladin-core/src/base/service/entity_version_service.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/campaign.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/chain_of_command.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/conclave.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/council.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/formation.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/grove.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/phalanx.rs` (scripted)
- `crates/paladin-core/src/platform/container/citadel.rs` (scripted)
- `crates/paladin-core/src/platform/container/content.rs` (scripted)
- `crates/paladin-core/src/platform/container/prompt.rs` (scripted)
- `crates/paladin-core/src/platform/container/user.rs` (scripted)
- `crates/paladin-eval/src/runner.rs` (scripted)
- `crates/paladin-memory/src/citadel/file_citadel.rs` (scripted)
- `crates/paladin-ports/src/output/citadel_port.rs` (scripted)
- `crates/paladin-ports/src/output/paladin_registry.rs` (scripted)
- `crates/paladin-storage/src/mysql_content_repository.rs` (scripted)
- `crates/paladin-storage/src/sqlite_content_repository.rs` (scripted)
- `crates/paladin-web/src/assistant_controller.rs` (scripted)
- `examples/campaign_workflow.rs` (scripted)
- `examples/commander_auto.rs` (scripted)
- `examples/commander_basic.rs` (scripted)
- `examples/commander_full_config.rs` (scripted)
- `examples/commander_with_metadata_export.rs` (scripted)
- `examples/conclave_expert_panel.rs` (scripted)
- `examples/council_discussion.rs` (scripted)
- `examples/formation_sequential.rs` (scripted)
- `examples/maneuver_basic.rs` (scripted)
- `examples/maneuver_dynamic_flow.rs` (scripted)
- `examples/maneuver_nested_flow.rs` (scripted)
- `examples/phalanx_parallel.rs` (scripted)
- `src/application/services/assistant/validator.rs` (scripted)
- `src/application/services/log_orchestrator/mod.rs` (scripted)
- `src/application/services/orchestration/processors/battalion_processor.rs` (scripted)
- `src/application/services/paladin/middleware/chain.rs` (scripted)
- `src/application/services/paladin/middleware/context.rs` (scripted)
- `src/application/services/paladin/middleware/guardrail.rs` (scripted)
- `src/application/services/paladin/middleware/history.rs` (scripted)
- `src/application/services/paladin/middleware/limits.rs` (scripted)
- `src/application/services/paladin/middleware/mod.rs` (scripted)
- `src/application/services/paladin/middleware/resilience.rs` (scripted)
- `src/application/services/paladin/middleware/summarization.rs` (scripted)
- `src/application/services/paladin/middleware/tool_protocol.rs` (scripted)
- `src/application/services/paladin/middleware/vault_recall.rs` (scripted)
- `src/application/services/paladin/paladin_builder.rs` (scripted)
- `src/application/services/paladin/paladin_execution_service.rs` (scripted)
- `src/application/services/paladin/structured.rs` (scripted)
- `src/application/services/paladin/temperature_service.rs` (scripted)
- `src/application/services/run/schedule/tests.rs` (scripted)
- `src/application/services/run/submission.rs` (scripted)
- `src/application/services/run/tracer_e2e.rs` (scripted)
- `src/application/services/run/worker.rs` (scripted)
- `src/application/services/run/worker_tests.rs` (scripted)
- `src/config/agent_runtime.rs` (scripted)
- `src/core/mod.rs` (scripted)
- `src/core/platform/manager/content_service.rs` (scripted)
- `src/infrastructure/telemetry/herald_sink.rs` (scripted)
- `src/infrastructure/web/facade_provisioner.rs` (scripted)
- `src/infrastructure/web/run_api_wiring.rs` (scripted)
- `src/presets/mod.rs` (scripted)
- `tests/cli/error_handling_test.rs` (scripted)
- `tests/cli/formation_execution_test.rs` (scripted)
- `tests/cli/paladin_execution_test.rs` (scripted)
- `tests/cli/phalanx_execution_test.rs` (scripted)
- `tests/cli/tool_integration_test.rs` (scripted)
- `tests/cli_isolation_test.rs` (scripted)
- `tests/helpers/e2e_fixtures.rs` (scripted)
- `tests/helpers/mock_llm_adapter.rs` (scripted)
- `tests/helpers/mock_paladin_port.rs` (scripted)
- `tests/integration/aegis_retry_stress_test.rs` (scripted)
- `tests/integration/battalion/campaign_integration_test.rs` (scripted)
- `tests/integration/battalion/council_integration_test.rs` (scripted)
- `tests/integration/battalion/formation_integration_test.rs` (scripted)
- `tests/integration/battalion/grove_integration_test.rs` (scripted)
- `tests/integration/battalion/load_test.rs` (scripted)
- `tests/integration/battalion/phalanx_integration_test.rs` (scripted)
- `tests/integration/battalion_campaign_integration_test.rs` (scripted)
- `tests/integration/battalion_chain_of_command_herald_test.rs` (scripted)
- `tests/integration/battalion_chain_of_command_integration_test.rs` (scripted)
- `tests/integration/citadel_integration_test.rs` (scripted)
- `tests/integration/commander_error_paths_test.rs` (scripted)
- `tests/integration/commander_integration_tests.rs` (scripted)
- `tests/integration/e2e_compensation_chain_test.rs` (scripted)
- `tests/integration/golden_bridge_equivalence_test.rs` (scripted)
- `tests/integration/middleware_under_engine_test.rs` (scripted)
- `tests/integration/structured_engine_node_test.rs` (scripted)
- `tests/integration/subgraph_formation_in_campaign_test.rs` (scripted)
- `tests/repository/mysql_content_repository_test.rs` (scripted)
- `tests/unit/battalion/campaign_service_tests.rs` (scripted)
- `tests/unit/battalion/campaign_tests.rs` (scripted)
- `tests/unit/battalion/chain_of_command_tests.rs` (scripted)
- `tests/unit/battalion/formation_tests.rs` (scripted)
- `tests/unit/battalion/phalanx_tests.rs` (scripted)
- `tests/unit/handoff_service_test.rs` (scripted)
- `tests/unit/mock_llm_adapter_test.rs` (scripted)
- `tests/unit/paladin_entity_test.rs` (scripted)

Total: 120 (112 scripted, 8 hand-fix). The count equals `git diff --name-only b82bcc8 c6dfc93 | wc -l`.

D-07 review of the engine-file hits: the only changed lines in `crates/paladin-battalion/src/engine`, `crates/paladin-eval` and `crates/paladin-storage` are the `use` line, `Node::new(` calls, `Node { .. }` literals, and one doc comment (`graph.rs`: "a `Node<PaladinData>`" now "an `Entity<PaladinData>`", still followed by `NodeSpec`). No line touching `NodeId`, `NodeSpec`, `StateNode`, `NodeContext`, `NodeCache*`, `NodeInterceptor`, `NodeExecutionRecord`, `NodeOutcomeKind`, `NodeProgressKind`, `TraceEvent`, `BattalionError::Node`, `EngineError::Node` or `NodeError` changed, and none of `wargraph-doc.schema.json`, `node_cache` config, `waypoint.rs`, `trace.rs` or `node_error.rs` is in the diff.

## Decisions Made

- One Stage A commit (see above). The scripted-then-fixes split the plan prefers was not taken because the scripted state fails `cargo fmt --check`.
- The Reasoning half of the status test mutates the loaded fixture's status instead of constructing a wrapper, which keeps `crates/paladin-core/tests/` untouched by Stage A.
- Existing `use ...::paladin::Paladin;` in the harness was left as is and `PaladinStatus` added on its own line, so the harness diff is purely additive (40 added, 0 deleted).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Intra-doc link did not resolve**
- **Found during:** Task 3 (`make doc-check`)
- **Issue:** the plan's `[`Entity<T>`](entity::Entity)` in `base/mod.rs` raised `unresolved link to entity::Entity` under the zero-warning rustdoc bar.
- **Fix:** used the crate-rooted `crate::base::entity::Entity`; acceptance (`grep -c 'entity::Entity'` at least 1) still holds.
- **Files modified:** `crates/paladin-core/src/base/mod.rs`
- **Commit:** `bd767f7`

**2. [Rule 1 - Bug] Stray `Node<T>` in the crate doc bullet**
- **Found during:** Task 2 acceptance grep
- **Issue:** `lib.rs` line 11 contains no `base::entity::Entity` import, so the per-file perl pass skipped it and the Stage A boundary grep still matched `Node<T>`.
- **Fix:** hand-edited the bullet to `Entity<T>` inside the Stage A commit (Task 3's lib.rs prose item was satisfied by this).
- **Files modified:** `crates/paladin-core/src/lib.rs`
- **Commit:** `c6dfc93`

**3. [Plan literal, not behaviour] Task 1 commit was amended once** to make the harness diff additive (0 deleted lines) as the acceptance criterion requires; and the Stage A message was amended to say 120 files. Both amendments were local and before any later commit relied on the hashes.

**Total deviations:** 2 auto-fixed, 0 architectural.

## Issues Encountered

- Disk: the filesystem had 1.1 GB free mid-task (A1-48-04). `target/debug/incremental` held 15 GB of stale caches; removing it (a pure cache) freed it, and later builds ran with `CARGO_INCREMENTAL=0`. No gate was skipped.
- Baseline: `cargo test --workspace --lib --bins --no-fail-fast` gives 4,705 passed and exactly two failures, `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set`, identical to the orchestrator's pre-phase baseline. Not touched.
- `cargo test -p paladin-ai-core --test entity_serde_compat` reports 6 passed after every commit.

## Verification

- `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`: all exit 0
- `make doc-check`, `make check-doc-examples`, `make check-api-examples`: exit 0
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: "API surface unchanged" (4274 items); baseline not refreshed
- `cargo test --test legacy_removal_guard` 3 passed; `--test functional` 10 passed; `--test citadel_integration` 11 passed
- Acceptance greps: no `base::entity::node`; no `Node` before `<`, `::new`, `::new_with_uuid`, `::<` or ` {` outside identifiers; `#[deprecated` count 0; `fn test_entity_` 4 and `fn test_node_` 0; `mod.rs` holds exactly the five required items; `git log --follow` on `wrapper.rs` shows 2 commits (history followed); D-05 item-line gate prints 0
- Manual credential-handling review (D-00a): no API key, response body, log statement or HTTP client changed; the only non-comment, non-rename edits are the citadel type unification and one test
- No dependency or manifest change (D-00d)

## Known Stubs

None.

## Threat Flags

None. T-48-10 mitigated (48-01's snapshot and tracer tests unedited and green, plus `v0_10_citadel_statuses_load_into_the_unified_status`); T-48-11 accepted and pinned by the same test (Reasoning is v0.11 only; MIGRATION.md 9.4 wording belongs to 48-07); T-48-12 mitigated (classification of all 120 files against a re-run of the script, graph-vocabulary diff review, compiler, `legacy_removal_guard`).

## Next Phase Readiness

48-05 can rename the payload field `node` to `data` (and `ContentItem.node` / `PromptItem.node` to `entity`) with read-aliases; the harness still reads `.node` and the `/paladin/node/status` pointer and will be updated there. 48-07 owns the semver registration (CI `semver` job stays red from `56213e4` onward) and the MIGRATION.md rows for the removed citadel types and the `PaladinState.paladin` type change.

## Self-Check: PASSED

- FOUND: `crates/paladin-core/src/base/entity/wrapper.rs`; `node.rs` absent
- FOUND commits: b82bcc8, c6dfc93, bd767f7
