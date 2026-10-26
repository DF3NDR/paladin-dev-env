---
phase: 48-base-entity-rename
plan: 05
subsystem: core-base
tags: [rename, entity, serde-alias, d-03, d-08, d-04, serde-compat, rustdoc]

requires:
  - phase: 48-01
    provides: "entity_serde_compat.rs harness, pre_rename fixtures and the FileCitadel tracer test"
  - phase: 48-03
    provides: "EntityVersion family and EntityVersionService::create_version -> VersioningError::VersioningDisabled, which the revisioning contract cites"
  - phase: 48-04
    provides: "Entity<T> at paladin_core::base::entity::Entity with the payload field still named node"
provides:
  - "Entity<T>.data (was node) with #[serde(alias = \"node\")], same declaration position, no other serde attribute"
  - "ContentItem.entity and PromptItem.entity (were node), each with #[serde(alias = \"node\")]; chains read item.entity.data.<field>"
  - "Entity<T> rustdoc: # Revisioning contract, # Serialization, compiling # Examples"
  - "Six alias contract tests (five in entity_serde_compat.rs, one in wrapper.rs)"
affects: [48-06, 48-07, 48-08, 48-09]

tech-stack:
  added: []
  patterns:
    - "Read-alias only: old key read via #[serde(alias)], new key written, duplicate and missing keys rejected by serde itself, no custom Deserialize"
    - "Compile-guided residual fixes driven by parsing rustc short messages (E0560 / E0609 positions), not by regex over receivers"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/base/entity/wrapper.rs
    - crates/paladin-core/src/platform/container/content.rs
    - crates/paladin-core/src/platform/container/prompt.rs
    - crates/paladin-core/tests/entity_serde_compat.rs
    - crates/paladin-memory/src/citadel/file_citadel.rs
    - crates/paladin-core/src/base/service/entity_version_service.rs
    - src/core/platform/manager/content_service.rs
    - crates/paladin-storage/src/sqlite_content_repository.rs
    - crates/paladin-storage/src/mysql_content_repository.rs
    - crates/paladin-storage/src/sqlite_user_repository.rs
    - tests/functional/content_lifecycle_test.rs
    - tests/repository/mysql_content_repository_test.rs

key-decisions:
  - "Stage B landed as ONE commit (cafc93b): the declarations plus the perl passes do not build on their own (struct literals and single-hop wrapper reads need the compiler), and a commit may not be red"
  - "Residual fixes were made from rustc's own E0560 / E0609 positions: a field named node in an Entity literal becomes data, in a ContentItem / PromptItem literal becomes entity, and a .data read on a ContentItem / PromptItem becomes .entity; no receiver-name regex decided anything"
  - "The two wrapper type files were pre-passed by a receiver-aware rewrite (.node.node -> .entity.data, .node.<uuid|created|modified|name|version> -> .entity.<same>) because every self.node.modified there is an entity-sense read; the global passes then had nothing left to retarget in them"
  - "new_writes_never_emit_the_old_keys does not require or forbid a data key on the version record (payload is under entity_data), matching the plan"

patterns-established:
  - "Alias edge cases are pinned with documents built from fixture_value edits and parsed through their text (serde_json::from_str on value.to_string()), the same input path as a real file read"

requirements-completed: [ENTITY-01, ENTITY-04, ENTITY-05]

coverage:
  - id: D1
    description: "Entity<T> payload field is data with a read-alias for node; ContentItem.entity and PromptItem.entity carry the same alias; writes emit only the new keys"
    requirement: "ENTITY-01"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-ai-core --test entity_serde_compat (11 passed: the five v0.10 fixtures load and re-serialize with only the entity-sense keys renamed)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel (written paladin has data, no node)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Alias edges: both spellings rejected (duplicate field), neither rejected (missing field), generic Entity<T> reads the old key, new writes never emit an old key, field order is declaration order"
    requirement: "ENTITY-01"
    verification:
      - kind: integration
        ref: "pre_rename_json_carrying_both_payload_keys_is_rejected, content_item_carrying_both_entity_keys_is_rejected, json_without_a_payload_key_is_rejected, generic_entity_reads_the_old_payload_key, new_writes_never_emit_the_old_keys"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib test_entity_serializes_fields_in_declaration_order"
        status: pass
    human_judgment: false
  - id: D3
    description: "ENTITY-05: every graph-sense .node line is byte-identical (18 lines), no other .node field access remains in any .rs file"
    requirement: "ENTITY-05"
    verification:
      - kind: other
        ref: "git grep -E 'graph\\.node\\(|(progress|fields)\\.node\\b|\\bp\\.node ==' before/after diff exit 0; residual .node grep prints 0 lines"
        status: pass
    human_judgment: false
  - id: D4
    description: "ENTITY-04 / D-04: Entity<T> rustdoc states the revisioning contract (only ContentItem via ContentItemService is revisioned) and the serialization contract, with a compiling doctest"
    requirement: "ENTITY-04"
    verification:
      - kind: other
        ref: "cargo test -p paladin-ai-core --doc entity (Entity doctest passes); make doc-check exit 0; acceptance greps for EntityVersionService, VersioningDisabled, ContentItemService, AssistantVersion, '# Serialization', '# Examples', 'duplicate field', 'missing field', '0.x'"
        status: pass
    human_judgment: false

duration: about 1h 15m
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 05: Stage B payload and wrapper field rename Summary

**`Entity<T>.node` is now `data` and `ContentItem.node` / `PromptItem.node` are now `entity`, each read-aliased to the v0.10 key so every captured pre-rename document still loads, new writes emit only the new keys, and `Entity<T>` documents what its `version` flag promises and how it serializes.**

## Accomplishments

- D-03: the payload field is `#[serde(alias = "node")] pub data: T` in its old position; no `rename`, `skip` or `default` attribute, no custom `Deserialize`, no `schema_version`.
- D-08: `ContentItem.entity` and `PromptItem.entity` carry the same alias; the hand-written `Hash` impls and the accessors use `self.entity`; chains read `item.entity.data.<field>`.
- Six tests pin the alias contract (all in the table below); the existing six compat tests now compare against the fixtures with only the entity-sense keys renamed (paladin root, citadel `/paladin`, content and prompt root then `/entity`); fixtures are untouched.
- D-04 / ENTITY-04: `Entity<T>` rustdoc has `# Revisioning contract`, `# Serialization` and a compiling `# Examples`; `ContentItem.entity` / `PromptItem.entity` carry one-sentence field docs naming the key read and the key written.
- D-07 / ENTITY-05: the 18 graph-sense `.node` lines are byte-identical to the pre-change snapshot.

## Task Commits

1. **Task 1: Stage B, payload field to data and wrapper fields to entity** - `cafc93b` (refactor)
2. **Task 2: alias contract tests and Entity rustdoc** - `e7ffc40` (test)

**Plan metadata:** the commit carrying this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md (docs: complete plan)

## Stage B commit structure and green evidence

Stage B is one commit, `cafc93b`. The declarations plus the three perl passes plus the harness edits do not build (the E0560 / E0609 sites below), so the scripted-pass / compile-guided split the plan allows was not available, and no red tree was committed. Evidence on `cafc93b`:

- `cargo check --workspace --all-targets --all-features` exit 0; `cargo fmt --all --check` exit 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0
- `cargo test -p paladin-ai-core --test entity_serde_compat` 6 passed; `pre_rename_citadel_snapshot_loads_through_file_citadel` 1 passed; `paladin-ai-core --lib` 709, `paladin-battalion --lib` 875, `paladin-eval --lib` 56 passed
- `cargo test --workspace --lib --bins --no-fail-fast`: 4,705 passed, exactly the two sandbox-baseline failures (`build_run_api_persists_no_run_traces_by_default`, `..._persists_run_traces_when_trace_persist_is_set`)

Residual compile-guided sites (all fixed from rustc positions): 3 `Entity { node: .. }` literals in `entity_version_service.rs`; `Entity` / `ContentItem` literals and shorthand `ContentItem { node }` in `sqlite_content_repository.rs`, `mysql_content_repository.rs`, `sqlite_user_repository.rs`, `tests/repository/mysql_content_repository_test.rs` (feature-gated, only visible with `--all-features`); ~20 single-hop `content.data.<uuid|created|modified|name|version>` reads in the two content repositories, `content_service.rs` and `tests/functional/content_lifecycle_test.rs`, which became `.entity.<field>`.

## Graph snapshot (Task 1 step 7)

`git grep -E 'graph\.node\(|(progress|fields)\.node\b|\bp\.node ==' -- '*.rs'` before (snapshot, 18 lines) versus after: `diff` exit 0. The residual `git grep -nE '\.node\b'` minus graph-sense and `..node` prints 0 lines (a first draft of a comment in `file_citadel.rs` tripped it and was reworded). Every removed `.node` line under `crates/paladin-battalion`, `crates/paladin-eval`, `src/infrastructure/web`, `crates/paladin-web` and the waypoint, trace and run containers was reviewed by hand: all are `paladin.node.*`, `grove.node.*`, `council.node.*`, `specialist.node.*`, `entry.paladin.node.*` -- entity-sense. `progress.node`, `fields.node`, `p.node ==` and every `graph.node(` are untouched. `cache_key.rs` is entity-sense and converted.

## `new_writes_never_emit_the_old_keys` assertions

| Fixture | JSON pointer | Key required | Keys forbidden |
|---------|--------------|--------------|----------------|
| paladin | root | `data` | `node` |
| citadel | `/paladin` | `data` | `node` |
| content | root | `entity` | `node` |
| content | `/entity` | `data` | `node` |
| prompt | root | `entity` | `node` |
| prompt | `/entity` | `data` | `node` |
| version record | root | `entity_id`, `entity_data` | `node_id`, `node_data` (no `data` key is required) |

For all five, the serialized text also contains none of `"node":`, `"node_id":`, `"node_data":`. A1-48-05 held: the five fixtures carry `node` only at the entity-sense positions above (and `node_id` / `node_data` only on the version record); no payload holds such a key, so the whole-string check is used.

## TDD note

The six tests pin behaviour Task 1 produced, so each was green on its first run; none was weakened. The first full run of `entity_serde_compat` was 11 passed. The duplicate-field and missing-field messages are serde's own (`duplicate field` / `missing field`).

## Rewritten files

108 files in `git diff --name-only 1211b56 cafc93b`: 96 `scripted` (byte-identical to a mechanical re-run of the three perl passes on the pre-Stage-B blobs, which also covers files `cargo fmt` did not reflow) and 12 `hand-fix` (declarations, struct literals, single-hop reads, harness, tracer test). `files_modified` in the plan lists 13 paths; this list is the authoritative set (A2-48-05). `crates/paladin-core/tests/fixtures/` is untouched across the range.

- `crates/doc-examples/src/support.rs` (scripted)
- `crates/paladin-battalion/benches/battalion_benchmarks.rs` (scripted)
- `crates/paladin-battalion/src/aegis_attempt.rs` (scripted)
- `crates/paladin-battalion/src/campaign_service.rs` (scripted)
- `crates/paladin-battalion/src/chain_of_command_service.rs` (scripted)
- `crates/paladin-battalion/src/commander.rs` (scripted)
- `crates/paladin-battalion/src/conclave_execution_service.rs` (scripted)
- `crates/paladin-battalion/src/council_service.rs` (scripted)
- `crates/paladin-battalion/src/engine/bridges.rs` (scripted)
- `crates/paladin-battalion/src/engine/cache_key.rs` (scripted)
- `crates/paladin-battalion/src/engine/mod.rs` (scripted)
- `crates/paladin-battalion/src/engine/settlement.rs` (scripted)
- `crates/paladin-battalion/src/engine/superstep.rs` (scripted)
- `crates/paladin-battalion/src/engine/test_support.rs` (scripted)
- `crates/paladin-battalion/src/formation_service.rs` (scripted)
- `crates/paladin-battalion/src/grove_service.rs` (scripted)
- `crates/paladin-battalion/src/maneuver/service.rs` (scripted)
- `crates/paladin-battalion/src/phalanx_service.rs` (scripted)
- `crates/paladin-content/src/services/content_fetching_service.rs` (scripted)
- `crates/paladin-core/src/base/entity/wrapper.rs` (hand-fix)
- `crates/paladin-core/src/base/service/entity_version_service.rs` (hand-fix)
- `crates/paladin-core/src/platform/container/arsenal/handoff_tool.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/chain_of_command.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/conclave.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/council.rs` (scripted)
- `crates/paladin-core/src/platform/container/battalion/grove.rs` (scripted)
- `crates/paladin-core/src/platform/container/content.rs` (hand-fix)
- `crates/paladin-core/src/platform/container/log.rs` (scripted)
- `crates/paladin-core/src/platform/container/paladin.rs` (scripted)
- `crates/paladin-core/src/platform/container/prompt.rs` (hand-fix)
- `crates/paladin-core/src/platform/container/user.rs` (scripted)
- `crates/paladin-core/tests/entity_serde_compat.rs` (hand-fix)
- `crates/paladin-eval/src/runner.rs` (scripted)
- `crates/paladin-llm/src/anthropic/adapter.rs` (scripted)
- `crates/paladin-llm/src/compat/engine.rs` (scripted)
- `crates/paladin-llm/src/deepseek/adapter.rs` (scripted)
- `crates/paladin-llm/src/gemini/adapter.rs` (scripted)
- `crates/paladin-llm/src/grok/adapter.rs` (scripted)
- `crates/paladin-llm/src/openai/adapter.rs` (scripted)
- `crates/paladin-memory/src/citadel/file_citadel.rs` (hand-fix)
- `crates/paladin-ports/src/output/citadel_port.rs` (scripted)
- `crates/paladin-storage/src/mysql_content_repository.rs` (hand-fix)
- `crates/paladin-storage/src/sqlite_content_repository.rs` (hand-fix)
- `crates/paladin-storage/src/sqlite_user_repository.rs` (hand-fix)
- `crates/paladin-web/src/agent_controller.rs` (scripted)
- `crates/paladin-web/src/agent_registry.rs` (scripted)
- `examples/autonomous_planning.rs` (scripted)
- `examples/autonomous_prompt_generation.rs` (scripted)
- `examples/basic_paladin.rs` (scripted)
- `examples/campaign_workflow.rs` (scripted)
- `examples/chain_of_command_delegation.rs` (scripted)
- `examples/citadel_autosave.rs` (scripted)
- `examples/citadel_restore.rs` (scripted)
- `examples/commander_auto.rs` (scripted)
- `examples/commander_basic.rs` (scripted)
- `examples/commander_full_config.rs` (scripted)
- `examples/commander_with_metadata_export.rs` (scripted)
- `examples/conclave_expert_panel.rs` (scripted)
- `examples/council_discussion.rs` (scripted)
- `examples/formation_sequential.rs` (scripted)
- `examples/grove_routing.rs` (scripted)
- `examples/maneuver_basic.rs` (scripted)
- `examples/maneuver_dynamic_flow.rs` (scripted)
- `examples/maneuver_nested_flow.rs` (scripted)
- `examples/paladin_with_config.rs` (scripted)
- `examples/phalanx_parallel.rs` (scripted)
- `examples/vision_analysis.rs` (scripted)
- `src/application/cli/commands/agent.rs` (scripted)
- `src/application/cli/commands/battalion.rs` (scripted)
- `src/application/services/log_orchestrator/mod.rs` (scripted)
- `src/application/services/log_orchestrator/types.rs` (scripted)
- `src/application/services/orchestration/processors/paladin_processor.rs` (scripted)
- `src/application/services/paladin/middleware/history.rs` (scripted)
- `src/application/services/paladin/middleware/resilience.rs` (scripted)
- `src/application/services/paladin/middleware/summarization.rs` (scripted)
- `src/application/services/paladin/paladin_builder.rs` (scripted)
- `src/application/services/paladin/paladin_execution_service.rs` (scripted)
- `src/application/services/paladin/temperature_service.rs` (scripted)
- `src/application/services/run/submission.rs` (scripted)
- `src/application/services/run/worker.rs` (scripted)
- `src/core/platform/manager/content_service.rs` (hand-fix)
- `src/infrastructure/web/agent_host.rs` (scripted)
- `tests/cli_isolation_test.rs` (scripted)
- `tests/functional/content_lifecycle_test.rs` (hand-fix)
- `tests/helpers/mock_llm_adapter.rs` (scripted)
- `tests/helpers/mock_paladin_port.rs` (scripted)
- `tests/integration/battalion/campaign_integration_test.rs` (scripted)
- `tests/integration/battalion/chain_of_command_integration_test.rs` (scripted)
- `tests/integration/battalion/council_integration_test.rs` (scripted)
- `tests/integration/battalion/formation_integration_test.rs` (scripted)
- `tests/integration/battalion/grove_integration_test.rs` (scripted)
- `tests/integration/battalion/load_test.rs` (scripted)
- `tests/integration/battalion/phalanx_integration_test.rs` (scripted)
- `tests/integration/battalion_campaign_integration_test.rs` (scripted)
- `tests/integration/battalion_chain_of_command_herald_test.rs` (scripted)
- `tests/integration/battalion_chain_of_command_integration_test.rs` (scripted)
- `tests/integration/battalion_herald_end_to_end_test.rs` (scripted)
- `tests/integration/citadel_integration_test.rs` (scripted)
- `tests/integration/commander_integration_tests.rs` (scripted)
- `tests/integration/herald_integration_test.rs` (scripted)
- `tests/integration/paladin_integration_test.rs` (scripted)
- `tests/repository/mysql_content_repository_test.rs` (hand-fix)
- `tests/unit/battalion/campaign_service_tests.rs` (scripted)
- `tests/unit/battalion/chain_of_command_tests.rs` (scripted)
- `tests/unit/mock_llm_adapter_test.rs` (scripted)
- `tests/unit/paladin_builder_test.rs` (scripted)
- `tests/unit/paladin_entity_test.rs` (scripted)
- `tests/unit/paladin_execution_service_test.rs` (scripted)

Total: 108 (96 scripted, 12 hand-fix).

## Deviations from Plan

None to the plan's intent. Process notes:

- **[Rule 3 - Blocking] Local variables named `node`.** Shorthand `ContentItem { node }` / `PromptItem { node }` literals and their `let node = Entity {..}` locals (content and prompt type files, both content repositories, the mysql repository test) were renamed to `entity` so the shorthand compiles. Compile-guided, in `cafc93b`.
- **Test fn names kept.** `tests/repository/mysql_content_repository_test.rs` still has fn names `test_content_item_node_structure` and `test_node_reconstruction_pattern` and a comment saying "nodes"; renaming prose and test names is outside this plan's acceptance and is left to the ENTITY-05 sweep.

## Issues Encountered

- Disk: free space dropped to 7.3 GB by the end of the full workspace test run (82% used); `CARGO_INCREMENTAL=0` was exported for every cargo command and nothing outside `target/` was removed. No gate was skipped.
- Scratch file: a shell loop used an unset `$TMPDIR` and wrote `/chk.txt` at the filesystem root (cargo check output, harmless). The sandbox safety check refused my attempt to delete it, so it remains and can be removed by hand.
- Flaky under load: `application::services::run::worker::tests::lease_heartbeat_extends_at_lease_over_four_and_stops_after_drop` failed once during the full `cargo test --workspace --no-fail-fast` run (a timing assertion; the file's only change is `.node` to `.data` on entity reads), passed in the preceding `--lib --bins` run and 3 of 3 in isolation. Not a Stage B regression.
- Baseline: the two `run_api_wiring` failures are the sandbox baseline; the full run also shows them identically.

## Verification

- `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`: all exit 0
- `cargo test --workspace --no-fail-fast`: 6,902 passed; the only failures are the two baseline tests plus the one load-sensitive heartbeat test above (passes alone)
- `cargo test -p paladin-ai-core --test entity_serde_compat` 11 passed; `rename_key(` count 9 (needs at least 8); tracer 1 passed; `--doc entity` passed
- `make doc-check`, `make check-doc-examples`, `make check-api-examples`: exit 0
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: "API surface unchanged" (4274 items), baseline not refreshed
- `cargo test --test legacy_removal_guard` 3 passed
- Acceptance greps: one `#[serde(alias = "node")]` in each of `wrapper.rs`, `content.rs`, `prompt.rs`; `pub data: T,` 1; `pub entity: Entity<ContentData>` 1; `pub entity: Entity<PromptData>` 1; `serde((default|rename|skip)` in `wrapper.rs` 0; payload-field prose grep prints nothing
- Manual credential-handling review (D-00a): no API key, response body, log statement or HTTP client changed
- No dependency or manifest change (D-00d)

## Known Stubs

None.

## Threat Flags

None. T-48-13 mitigated (serde rejects both spellings, pinned by two tests, no custom deserializer); T-48-14 mitigated (18-line snapshot diff exit 0, residual `.node` grep empty); T-48-15 mitigated at this layer by the `# Serialization` rustdoc (MIGRATION.md 9.1 M-B-12 and 9.4 wording belongs to 48-07).

## Next Phase Readiness

The fields are renamed and the contracts are documented; 48-06 can sweep the prose (CLAUDE.md, copilot-instructions naming table, domain-model.md, the SVG), 48-07 owns the MIGRATION.md rows, the semver allow lines and the CHANGELOG, and 48-08's guard rule 3 (no `.node` / `node:` payload access in the eight container files) can now pass.

## Self-Check: PASSED

- FOUND: `crates/paladin-core/src/base/entity/wrapper.rs` with `#[serde(alias = "node")]`, `# Revisioning contract`, `# Serialization`, `# Examples`
- FOUND: `fn new_writes_never_emit_the_old_keys` in `crates/paladin-core/tests/entity_serde_compat.rs`
- FOUND commits: cafc93b, e7ffc40
