---
phase: 22-battlefield-state-superstep-engine
plan: 11
subsystem: infra
tags: [rust, superstep-engine, legacy-bridge, golden-tests, coverage, tdd]

# Dependency graph
requires:
  - phase: 22-08
    provides: "NodeSpec::Paladin execution through engine::input_mapping::InputMapping, and the complete WarEngine::resume this plan's bridges run over unchanged"
  - phase: 22-09
    provides: "engine::dispatch_registry, engine::hooks (TraceDispatcher/NodeInterceptor) -- unused by this plan's bridges but confirming the engine module surface this plan builds on top of"
  - phase: 22-10
    provides: "the three WaypointPort backends (InMemoryWaypointStore used throughout this plan's golden tests) and the shipped coverage-gate tooling this plan's Task 3 measures against"
provides:
  - "WarGraph::from_formation/from_phalanx/from_campaign (paladin_battalion::engine::bridges): additive legacy bridges reproducing FormationExecutionService/PhalanxExecutionService/CampaignExecutionService's data flow as typed WarGraphs, without modifying any legacy service"
  - "campaign_node_ids (D-05 slug/short-uuid NodeId mapping) and dedicated_output_field, both re-exported from paladin_battalion::engine, giving external callers (the golden test, and future consumers) a way to interrogate a bridged Campaign's structural NodeId/field mapping"
  - "tests/integration/golden_bridge_equivalence_test.rs (cargo test target golden_bridge_equivalence): byte-for-byte output-equivalence proof for a 3-node Formation, a 3-node Phalanx, a branching Campaign (diamond fan-out/fan-in), a Campaign false-branch case, and the empty-paladin-list case"
  - "measured coverage figures closing the phase's ADR-0006/PRD-acceptance-7 gate: 86.35% workspace line coverage, 96.17% on this phase's new modules, both well above their respective 82%/85% floors"
affects: [23, 24, 25]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Legacy bridge constructors are associated functions on WarGraph (WarGraph::from_formation/from_phalanx/from_campaign) in a new engine::bridges module, per the plan's literal instruction, rather than free functions"
    - "from_formation and from_phalanx build the ENG-FR-19 default three-field schema (input LastWrite, output LastWrite, history Append) exactly as specified -- Formation's sequential chain reads/overwrites the shared `output` field one node per superstep (never concurrent, so never a LastWrite conflict); Phalanx's concurrent fan-out writes into the shared `history` Append field with zero-padded NodeIds keeping Append's (NodeId, emission index) merge order aligned with the paladins Vec's own order"
    - "from_campaign extends that same three-field baseline with one dedicated LastWrite field per Paladin (documented deviation, see below) -- a general DAG's concurrent fan-out siblings (e.g. a diamond's two branches) run in the SAME superstep, and two distinct writers touching one shared LastWrite field is a hard DispatchConflict; per-node dedicated fields make that structurally impossible while still letting InputMapping's plain string substitution (not a new engine-level aggregation mechanism) reproduce the legacy fan-in concatenation exactly, by naming each parent's own dedicated field in the fan-in node's template, joined by the same separator"
    - "from_campaign's fan-in InputMapping template is built at graph-CONSTRUCTION time by calling the exact same campaign.graph().edges_directed(node_index, Incoming) petgraph call the legacy campaign_service.rs::aggregate_inputs_for_node makes at RUNTIME, over the same Campaign value -- so the two paths' parent order (and therefore the exact fan-in concatenation byte sequence) can never diverge, it is not merely tolerant of either order"
    - "D-05's Uuid -> NodeId slug/short-uuid mapping (campaign_node_ids) is a pure function of the Campaign's own content: slug uniqueness is decided by a full count pass first, independent of HashMap iteration order, so the produced mapping (and therefore WarGraph::fingerprint()) is identical across repeated construction from the same Campaign regardless of HashMap iteration order"
    - "Golden equivalence tests use two independently-constructed FaultyPaladinPort instances (one per path) rather than one shared instance -- FaultyPaladinPort::execute is a pure function of (paladin name, input) with no cross-call state beyond its own counters, so this avoids any need to index-slice a shared call log to separate the two paths' entries"
    - "Golden tests sort per-paladin call logs before comparison only where BOTH paths run Paladins concurrently (Phalanx; Campaign's diamond fan-out) and real completion order is not deterministic on either side; Formation's and Campaign's own toposort-ordered execution are compared in raw insertion order since both paths are genuinely deterministic there"

key-files:
  created:
    - crates/paladin-battalion/src/engine/bridges.rs
    - tests/integration/golden_bridge_equivalence_test.rs
  modified:
    - crates/paladin-battalion/src/engine/mod.rs
    - Cargo.toml

key-decisions:
  - "[Deviation from literal plan wording] from_campaign's schema is NOT literally 'exactly the three named fields' the plan's must-have truth states for all three constructors -- it is the same three-field baseline PLUS one dedicated LastWrite field per Paladin. A single shared output field under LastWrite, written by every campaign node, would hard-conflict (BattalionError::DispatchConflict-equivalent EngineError) the instant two siblings in a fan-out (e.g. a diamond's two branches) execute in the same superstep, which the general-DAG case this bridge exists to support requires by construction. from_formation and from_phalanx remain literally exactly three fields, matching the plan text precisely, since neither pattern ever has two distinct writers touching the same field in one superstep. Recorded as a WINDOWS.md deviation entry (id 23) rather than silently reinterpreted; every other must-have truth and acceptance criterion (byte-for-byte golden equivalence, legacy services untouched, NodeId/fingerprint determinism, empty-list validate+immediate-completion) is met exactly as written, including the specific acceptance criterion 'A test asserts the default schema is exactly the three named fields with their stated dispatch rules', proven directly against from_formation's own schema."
  - "Per-node output field naming: dedicated_output_field(node_id) = FieldName::new(format!(\"out__{node_id}\")) -- prefixed rather than bare, so a Paladin literally named \"input\", \"output\" or \"history\" can never collide with the three baseline field names."
  - "campaign_node_ids and dedicated_output_field are exposed as public API (re-exported from paladin_battalion::engine) rather than pub(crate) -- the golden test, an external integration-test crate, needs to independently compute which Battlefield field corresponds to a given bridged Campaign node in order to assert final-output equivalence without inventing a second, potentially-drifting naming scheme."
  - "Coverage measurement (Task 3): scripts/coverage.sh required cargo-llvm-cov, which was not installed in this executor's environment. Installed cargo-llvm-cov@0.8.7 via cargo install --locked -- the EXACT version .github/workflows/ci.yml:846 already pins via taiki-e/install-action, verified before installing (not a new/unvetted dependency, a missing local copy of an existing, already-approved CI tool)."

requirements-completed: [ENG-06]

coverage:
  - id: D1
    description: "WarGraph::from_formation and WarGraph::from_phalanx build the ENG-FR-19 default three-field schema (input LastWrite, output LastWrite, history Append) and reproduce Formation's sequential chain / Phalanx's concurrent fan-out via InputMapping template substitution, including the empty-paladin-list case validating and completing immediately"
    requirement: "ENG-06"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::default_schema_is_exactly_three_named_fields_with_stated_dispatch_rules"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::from_formation_chains_output_into_next_input"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::from_phalanx_all_write_history_in_vec_order"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::from_formation_empty_list_validates_and_completes_immediately"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::from_phalanx_empty_list_validates_and_completes_immediately"
        status: pass
    human_judgment: false
  - id: D2
    description: "WarGraph::from_campaign reproduces a branching Campaign's edges, entry points and D-05 NodeId mapping, with the legacy fan-in separator reproduced exactly for two parents and no separator for one parent"
    requirement: "ENG-06"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::two_parent_fan_in_uses_the_legacy_separator_read_from_its_source"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::one_parent_fan_in_inserts_no_separator"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::unique_slugs_produce_bare_slug_node_ids"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::colliding_slugs_get_distinct_short_uuid_suffixes"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::same_campaign_built_twice_yields_identical_node_ids_and_fingerprint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/bridges.rs#tests::from_campaign_reproduces_diamond_fan_out_and_fan_in"
        status: pass
    human_judgment: false
  - id: D3
    description: "Legacy formation_service.rs, phalanx_service.rs, campaign_service.rs and commander.rs are byte-identical in behavior -- untouched by this plan, existing tests pass unchanged"
    requirement: "ENG-06"
    verification:
      - kind: other
        ref: "git diff --stat crates/paladin-battalion/src/{formation_service,phalanx_service,campaign_service,commander}.rs (empty output)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib (316 passed, 0 failed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Golden output-equivalence tests: a 3-node Formation, a 3-node Phalanx, a branching Campaign (diamond fan-out/fan-in), a Campaign false-branch case and an empty-paladin-list case, each run once through its legacy service and once through the matching bridge over the SAME mock port, comparing final output and ordered per-paladin inputs as raw, unnormalized strings"
    requirement: "ENG-06"
    verification:
      - kind: integration
        ref: "tests/integration/golden_bridge_equivalence_test.rs#formation_3_node_matches_legacy_final_output_and_per_paladin_inputs"
        status: pass
      - kind: integration
        ref: "tests/integration/golden_bridge_equivalence_test.rs#phalanx_3_node_matches_legacy_collected_results_and_per_paladin_inputs"
        status: pass
      - kind: integration
        ref: "tests/integration/golden_bridge_equivalence_test.rs#campaign_diamond_matches_legacy_final_output_including_fan_in_concatenation"
        status: pass
      - kind: integration
        ref: "tests/integration/golden_bridge_equivalence_test.rs#campaign_false_branch_condition_produces_the_same_outcome_from_both_paths"
        status: pass
      - kind: integration
        ref: "tests/integration/golden_bridge_equivalence_test.rs#empty_paladin_list_produces_the_same_outcome_from_both_paths"
        status: pass
    human_judgment: false
  - id: D5
    description: "Phase coverage gate closed: workspace line coverage and this phase's new-module line coverage both measured with the project's own scripts/coverage.sh, both above their acceptance-7 floors (82% workspace, 85% new modules)"
    requirement: "ENG-06"
    verification:
      - kind: other
        ref: "bash scripts/coverage.sh (exit 0); lcov.info summed per-file LF/LH across all SF: records (workspace) and across the phase's new-module paths"
        status: pass
    human_judgment: false

# Metrics
duration: ~2h (dominated by ~35min of instrumented full-workspace coverage compilation+run)
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 11: Legacy Bridges, Golden Equivalence Tests & Coverage Gate Summary

**`WarGraph::from_formation`/`from_phalanx`/`from_campaign` reproduce the three legacy Battalion patterns byte-for-byte over untouched legacy services, proven by golden equivalence tests, with the phase's coverage gate closing at 86.35% workspace / 96.17% new-modules.**

## Performance

- **Duration:** ~2h (includes ~35min of instrumented full-workspace `cargo llvm-cov` compilation and sequential test run, and installing `cargo-llvm-cov@0.8.7`)
- **Tasks:** 3 completed
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- `WarGraph::from_formation` and `WarGraph::from_phalanx` build the ENG-FR-19 default three-field schema (`input` `LastWrite`, `output` `LastWrite`, `history` `Append`) exactly as specified, reproducing `FormationExecutionService`'s sequential output-chaining and `PhalanxExecutionService`'s concurrent fan-out purely through `InputMapping` string templates — no new engine mechanism needed for either.
- `WarGraph::from_campaign` reproduces a branching `Campaign`'s edges, entry points, D-05 deterministic `NodeId` mapping (`campaign_node_ids`), and the legacy `"\n\n---\n\n"` fan-in separator exactly, by giving each Paladin its own dedicated `LastWrite` field and building each fan-in node's `InputMapping` template from the SAME `petgraph` incoming-edge iteration the legacy service's own `aggregate_inputs_for_node` uses at runtime — so the two paths' concatenation order can never diverge.
- `tests/integration/golden_bridge_equivalence_test.rs` (new `golden_bridge_equivalence` cargo test target) proves byte-for-byte output equivalence for a 3-node Formation, a 3-node Phalanx, a branching diamond Campaign (fan-out/fan-in), a Campaign false-branch case, and the empty-paladin-list case — every comparison is a raw `assert_eq!`, verified by the plan's own zero-hit grep for `.trim()`/`.to_lowercase()`/`.replace(`.
- Legacy `formation_service.rs`, `phalanx_service.rs`, `campaign_service.rs` and `commander.rs` are provably untouched (`git diff --stat` empty for all four), and their existing test suites (316 tests in `paladin-battalion --lib`) pass unchanged.
- The phase's coverage acceptance gate (criterion 7) is closed with measured, reproducible figures: **86.35% workspace line coverage** (53,222/61,637 lines) and **96.17%** on this phase's new modules (6,233/6,481 lines) — both comfortably above the 82%/85% floors, at commit `1d3d03ff`. No additional tests were needed to clear either floor.
- `cargo test --test golden_bridge_equivalence`, `--test e2e_crash_resume`, `--test war_engine_tracer`, `--workspace --lib`, `--workspace`, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` are all green.

## Task Commits

Each task was committed atomically:

1. **Task 1: Bridge constructors from_formation, from_phalanx and from_campaign** - `26ab4cb5` (feat)
2. **Task 2: Golden output-equivalence tests against the legacy services** - `1d3d03ff` (test)
3. **Task 3: Close the phase coverage gate on the new modules** - no commit (coverage already cleared both floors; no source changes required — see Deviations)

**Plan metadata:** committed alongside this SUMMARY (worktree mode; STATE.md/ROADMAP.md excluded, orchestrator owns those after wave merge)

## Files Created/Modified

- `crates/paladin-battalion/src/engine/bridges.rs` - New: `WarGraph::from_formation`/`from_phalanx`/`from_campaign`, `campaign_node_ids`, `dedicated_output_field`, `CAMPAIGN_FAN_IN_SEPARATOR`, 14 unit tests
- `crates/paladin-battalion/src/engine/mod.rs` - Registers `pub mod bridges;`, re-exports `campaign_node_ids`/`dedicated_output_field`/`CAMPAIGN_FAN_IN_SEPARATOR`, module-doc entry
- `tests/integration/golden_bridge_equivalence_test.rs` - New: 5 golden equivalence tests (Formation, Phalanx, Campaign diamond, Campaign false-branch, empty list)
- `Cargo.toml` - New `[[test]] name = "golden_bridge_equivalence"` target

## Decisions Made

See `key-decisions` in frontmatter. The most consequential: `from_campaign`'s schema is NOT literally the plan's stated "exactly three fields" for every constructor — it extends the three-field baseline with one dedicated `LastWrite` field per Paladin, because a single shared `output` field under `LastWrite` would hard-conflict the instant two Campaign siblings execute in the same superstep (a structural certainty for any real fan-out, not an edge case). `from_formation` and `from_phalanx` remain literally exactly three fields, matching the plan text precisely, since neither pattern ever produces two distinct same-superstep writers to one field. This is recorded as WINDOWS.md deviation entry id 23 rather than silently reinterpreted, and every other must-have truth and acceptance criterion — including the specific "schema is exactly the three named fields" test — is satisfied exactly as written (proven directly against `from_formation`'s own schema, which genuinely has no other fields).

A secondary, environment-level decision: `cargo-llvm-cov` was not present in this executor's environment. Installed `cargo-llvm-cov@0.8.7` via `cargo install --locked` — verified first against `.github/workflows/ci.yml:846`, which already pins that exact version via `taiki-e/install-action`, so this was restoring an already-approved CI tool locally, not introducing a new or unvetted dependency (the deviation-rules package-install exclusion is about unverified/potentially-slopsquatted names; this one was cross-checked against the project's own CI configuration before installing).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo-llvm-cov` not installed in the executor environment**
- **Found during:** Task 3, first invocation of `bash scripts/coverage.sh`
- **Issue:** The script requires `cargo llvm-cov`, which is not a cargo built-in and was not present (`error: no such command: llvm-cov`).
- **Fix:** Verified the exact pinned version against `.github/workflows/ci.yml:846` (`tool: cargo-llvm-cov@0.8.7`, installed there via `taiki-e/install-action`), then ran `cargo install cargo-llvm-cov@0.8.7 --locked` to match it exactly.
- **Files modified:** None (local toolchain only; no `Cargo.toml`/`Cargo.lock` change, since this is a standalone cargo subcommand binary, not a project dependency)
- **Verification:** `bash scripts/coverage.sh` subsequently ran to completion and exited 0.
- **Committed in:** N/A (no repository change; environment setup only)

### Architectural Note (not a Rule 1-3 auto-fix; recorded per the plan's own X-03 stop-and-flag guidance, resolved without needing a checkpoint since no legacy service required modification)

**2. `from_campaign`'s schema extends beyond the plan's literal "exactly three fields" wording** — see `key-decisions` above and WINDOWS.md deviation entry id 23 for the full rationale. This was resolved without a checkpoint because: (a) it required no change to any legacy service, staying fully within ENG-FR-20's constraint; (b) every must-have truth this plan is actually graded against — byte-for-byte golden equivalence, legacy services untouched, NodeId/fingerprint determinism, empty-list handling, and the specific "schema is exactly three fields" test (satisfied against `from_formation`) — is met; and (c) the alternative (a single shared `output` field for all Campaign nodes) is not a stricter reading of the spec, it is a design that provably breaks on the exact "branching Campaign fixture" this plan's own Task 2 requires testing.

---

**Total deviations:** 1 auto-fixed (Rule 3, environment tooling) + 1 architectural note (schema design, recorded in WINDOWS.md, no checkpoint needed). No scope creep — no legacy service touched, no new public API beyond the plan's three bridge constructors plus the two small helper functions (`campaign_node_ids`, `dedicated_output_field`) needed for the golden test to verify `from_campaign`'s output.

## Issues Encountered

Two authoring bugs were caught and fixed during Task 1/Task 2 development, before any commit:
- An early unit test compared `InputMapping`'s `{:?}` debug-formatted output (which re-escapes real newlines as literal `\n` two-character sequences) against a string built from the real separator constant — fixed by comparing via `InputMapping`'s own `PartialEq` instead of a debug-formatted string.
- An early golden-test assertion read a Campaign sink node's OWN dedicated output field expecting to find its fan-in-concatenated INPUT there — that field holds the node's own OUTPUT, not its input. Fixed by reading the expected concatenation from the recording port's own call log instead.

Both were found by running the tests locally (not discovered later), fixed before commit, and are not separately itemized as Deviations since they never reached a committed state.

## User Setup Required

None — no external service configuration required. Coverage measurement used the already-reachable `redis`/`minio` hostnames in this execution environment; `cargo-llvm-cov` was installed as a local dev-tool (see Deviations).

## Coverage Measurement Record (Task 3)

- **Measured at commit:** `1d3d03ff94550b692e8b51b3be73bb3250f8694f`
- **Command:** `bash scripts/coverage.sh` (== `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1`) — exit 0.
- **Workspace line coverage: 86.35%** (53,222 / 61,637 lines), summed directly from `lcov.info`'s `LF:`/`LH:` records across every `SF:` entry (not the `cargo llvm-cov report --summary-only` command, per ADR-0006's Phase 15 amendment finding that command reports a narrower, rosier scope than the gate). PASS against the 82% floor by 4.35pp.
- **New-module line coverage: 96.17%** (6,233 / 6,481 lines), summed the same way, restricted to this phase's new files: `crates/paladin-core/src/platform/container/{battlefield,waypoint}.rs`, `crates/paladin-ports/src/output/{waypoint_port,trace_sink_port}.rs`, `crates/paladin-storage/src/waypoint/{in_memory,redact,retention,sqlite}.rs`, and every file under `crates/paladin-battalion/src/engine/` (`bridges`, `dispatch_registry`, `graph`, `hooks`, `input_mapping`, `mod`, `node`, `superstep`, `test_support`). PASS against the 85% floor by 11.17pp.
- **Deliberately unmeasured path:** `crates/paladin-storage/src/waypoint/postgres.rs` produces no `SF:` record in this run because the `postgres` feature is not part of `integration-tests,llm-all` (D-10: Postgres contract tests run via the Docker-gated `make test-integration-docker` target, not this script). This mirrors the pre-existing WINDOWS.md entry id 22 (Postgres Tier 2 suite unverified against a live server in this sandbox — no Docker daemon available). `crates/paladin-storage/src/waypoint/mod.rs` also produces no `SF:` record, but for an unrelated reason: it contains only doc comments and `pub mod` declarations with zero executable lines, so `llvm-cov` has nothing to instrument there.
- **Ratchet-trigger note:** the last recorded workspace figure in `.planning/decisions/0006-coverage-gate.md` (Phase 15 amendment, 2026-08-13) was **82.39%**. This measurement (86.35%) is **+3.96 percentage points**, which meets this project's own ≥2-point ratchet-trigger convention. Recorded here per this task's own instruction to "note it" rather than silently absorb it; no ADR-0006 amendment was made by this plan (out of this plan's file scope), and no new tests were added since both floors were already cleared — a future phase or an explicit ADR-0006 amendment pass can decide whether to raise the floor. The increase is consistent with, not surprising given: Phase 22 alone added ~6,481 lines at 96.17% coverage (the whole Battlefield/Waypoint/superstep-engine subsystem, built under strict TDD per the PRD's §7 ordering), which alone would move a workspace average of this size by several points, on top of whatever incremental coverage phases 18-21 (the v0.9.0 milestone) added since the last measurement.
- **No tests were added in this task** — both floors were already cleared by the code Task 1/Task 2 (and prior plans in this phase) delivered, so the "no assertion-free test added to pad the number" acceptance criterion is satisfied vacuously.

## Next Phase Readiness

- ENG-06 is fully proven: all three bridge constructors reproduce legacy data flow byte for byte over real fixtures (including the branching Campaign case ENG-FR-19 specifically names), every legacy execution service file is unmodified, `NodeId`/fingerprint determinism is proven across repeated construction, and the phase's coverage acceptance criterion (7) closes with measured, recorded figures.
- `WarGraph::from_formation`/`from_phalanx`/`from_campaign`, `campaign_node_ids` and `dedicated_output_field` are stable public surfaces (`paladin_battalion::engine::{from_formation, from_phalanx, from_campaign, campaign_node_ids, dedicated_output_field}` via `WarGraph`'s inherent impls and the module re-exports) a later phase can build a real migration path on top of, without further signature changes.
- This is the final plan of Phase 22 (wave 7 of 7). No blockers for Phase 23's dynamic control-flow work (Directive routing, Muster fan-out, subgraphs) or the CF-01 BUG-01 fix, which this phase's ENG-FR-20 constraint explicitly reserved as the program's sole sanctioned legacy-service change.
- Carried-forward, pre-existing WINDOWS.md items this plan did not touch: entry id 22 (Postgres Tier 2 suite unverified against a live server — needs `make test-integration-docker` in an environment with Docker).

## Self-Check: PASSED

- FOUND: crates/paladin-battalion/src/engine/bridges.rs
- FOUND: tests/integration/golden_bridge_equivalence_test.rs
- FOUND: .planning/phases/22-battlefield-state-superstep-engine/22-11-SUMMARY.md
- FOUND: commit 26ab4cb5 (Task 1)
- FOUND: commit 1d3d03ff (Task 2)

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
