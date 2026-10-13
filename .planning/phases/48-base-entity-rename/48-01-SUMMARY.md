---
phase: 48-base-entity-rename
plan: 01
subsystem: testing
tags: [serde, persisted-json, compatibility, fixtures, citadel, file-citadel]

requires:
  - phase: 47
    provides: "v0.10 base wrapper, version service and Citadel as the pre-rename tree"
provides:
  - "Five v0.10 persisted-JSON fixtures captured from the unmodified tree"
  - "Tracer: a pre-rename Citadel snapshot loads through the real FileCitadel adapter and saves back to identical JSON"
  - "entity_serde_compat.rs: the persisted-JSON compatibility harness every later rename commit must keep green"
affects: [48-03, 48-04, 48-05, 48-07, 48-08]

tech-stack:
  added: []
  patterns:
    - "Fixture-driven serde compatibility: committed v0.10 bytes, deserialize through public types, lossless re-serialize comparison"
    - "Lossless comparison via to_string then from_str (never to_value) so f32 fields do not widen"

key-files:
  created:
    - crates/paladin-core/tests/fixtures/pre_rename/paladin.json
    - crates/paladin-core/tests/fixtures/pre_rename/citadel.json
    - crates/paladin-core/tests/fixtures/pre_rename/content.json
    - crates/paladin-core/tests/fixtures/pre_rename/prompt.json
    - crates/paladin-core/tests/fixtures/pre_rename/version.json
    - crates/paladin-core/tests/entity_serde_compat.rs
  modified:
    - crates/paladin-memory/src/citadel/file_citadel.rs

key-decisions:
  - "Tracer is the riskiest promise (persisted state across disk, adapter, port, domain), not a thin rename slice, because a rename cannot be sliced thinner than one atomic commit"
  - "Harness tests avoid naming the citadel placeholder PaladinData/PaladinStatus so 48-04's unification needs no edit here"
  - "No dependency or manifest change (D-00d): only serde_json and std"

patterns-established:
  - "Pre-rename wire evidence: fixtures are never hand-edited; a failing compat test is a wire regression to report, not an assertion to relax"

requirements-completed: [ENTITY-01, ENTITY-03]

coverage:
  - id: D1
    description: "Five v0.10 persisted-JSON fixtures (Paladin, Citadel snapshot, ContentItem, PromptItem, version record) captured from the unmodified tree, holding only the old keys"
    requirement: "ENTITY-01"
    verification:
      - kind: other
        ref: "python3 key-shape assertions over crates/paladin-core/tests/fixtures/pre_rename/*.json (re-run by the orchestrator, all pass)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Tracer: pre-rename Citadel snapshot loads through FileCitadel and saves back to an identical JSON value"
    requirement: "ENTITY-01"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel"
        status: pass
    human_judgment: false
  - id: D3
    description: "Compatibility harness: all five fixtures deserialize through the public core types and re-serialize losslessly"
    requirement: "ENTITY-01, ENTITY-03"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-ai-core --test entity_serde_compat"
        status: pass
    human_judgment: false

duration: split across two agents (container restart)
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 01: Pre-rename persisted-JSON compatibility harness Summary

**v0.10 persisted JSON pinned end to end before any rename: five captured fixtures, a Citadel snapshot proven through the real FileCitadel adapter, and a five-test lossless harness over the public core types.**

## Execution note

Execution was split across two agents by a container restart. The first agent ran Task 1 (the tracer),
committed it as 28c9bc5, and was killed at the tracer feedback gate before it could return. The
orchestrator presented the tracer to the user, who approved it (2026-10-13). This continuation agent
verified Task 1 was intact (commit contents present, tracer test green on the current tree) without
redoing it, then executed Task 2.

## Accomplishments

- Captured the five Wave 0 fixtures from the unmodified tree; they carry only v0.10 keys (`node`, `node_id`, `node_data`) and hold no secret, token or host-specific path (A1-48-01 confirmed by reading each file; values are `Default` output and the literals `fixture`, `hello`, `hi`, `captured`).
- Tracer (`pre_rename_citadel_snapshot_loads_through_file_citadel`): disk -> `FileCitadel` -> `CitadelPort` -> `PaladinState`, and back out to a JSON value equal to the fixture.
- Harness `entity_serde_compat.rs` (first integration test of `paladin-ai-core`): `pre_rename_paladin_json_still_loads`, `pre_rename_citadel_json_still_loads`, `pre_rename_content_item_json_still_loads`, `pre_rename_prompt_item_json_still_loads`, `pre_rename_version_json_still_loads`. Each asserts captured field values and a lossless round trip.

## Task Commits

1. **Task 1: Pre-rename Citadel snapshot tracer plus five fixtures** - `28c9bc5` (test), executed by the earlier agent; tracer gate approved by the user
2. **Task 2: Compatibility harness over all five persisted shapes** - `5d4e609` (test)

## Files Created/Modified

- `crates/paladin-core/tests/fixtures/pre_rename/{paladin,citadel,content,prompt,version}.json` - v0.10 bytes, never hand-edited
- `crates/paladin-core/tests/entity_serde_compat.rs` - the compatibility harness; helpers `fixture`, `fixture_value`, `reserialized`
- `crates/paladin-memory/src/citadel/file_citadel.rs` - tracer test in the existing `tests` module

## Decisions Made

- The harness reads today's names (`.node` payload and entity fields, `node_id`/`node_data`); 48-03 and 48-05 rewrite them as the renames land.
- The version record is deserialized as `NodeVersion<ContentData>` and its id is compared to the content fixture's `/node/uuid`, tying the fifth fixture to the third.

## Deviations from Plan

None - plan executed as written. One cosmetic adjustment: the `reserialized` doc comment initially named `serde_json::to_value`, which tripped the plan's `grep -c 'serde_json::to_value'` = 0 criterion; it was reworded before the commit. The function itself never calls it.

## Issues Encountered

- Per 48-RESEARCH.md Pitfall 10, two root-crate tests in `infrastructure::web::run_api_wiring::tests` fail identically on unmodified HEAD in this sandbox. They are pre-existing, out of scope, and not part of this plan's verification commands.

## Verification

- `cargo test -p paladin-ai-core --test entity_serde_compat`: 5 passed
- `cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel`: 1 passed
- `cargo test -p paladin-ai-core` (708 lib + doc/integration) and `cargo test -p paladin-memory --lib` (123): all pass
- `cargo clippy -p paladin-ai-core --all-targets --all-features -- -D warnings` and `cargo fmt --all --check`: clean
- `cargo check --workspace --all-targets --all-features`: clean
- `cargo test --test legacy_removal_guard`: 3 passed
- Acceptance greps: 5 `fn pre_rename_`, 0 `serde_json::to_value`, 1 `fn reserialized`, 0 `unwrap()`, 0 `citadel::(PaladinData|PaladinStatus)`; no change to `crates/paladin-core/Cargo.toml` or `crates/paladin-memory/Cargo.toml` (D-00d)
- Manual credential-handling review (D-00a): tests and JSON fixtures only; no API key, response body, log statement or HTTP client touched

## Known Stubs

None.

## Threat Flags

None. T-48-01 mitigated (fixtures read before commit); T-48-02 mitigated (precondition and key-shape acceptance); T-48-03 accepted (parsing path unchanged).

## Next Phase Readiness

The harness passes on the pre-rename tree, so any later failure signals a real wire change. 48-03 (version aliases), 48-04 (citadel unification and type rename) and 48-05 (field rename) can proceed and must keep it green.

## Self-Check: PASSED

- FOUND: crates/paladin-core/tests/entity_serde_compat.rs and the five fixtures
- FOUND commits: 28c9bc5, 5d4e609
