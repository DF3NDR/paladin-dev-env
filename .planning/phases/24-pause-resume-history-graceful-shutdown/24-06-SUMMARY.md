---
phase: 24-pause-resume-history-graceful-shutdown
plan: 06
subsystem: infra
tags: [rust, superstep-engine, chronicle, branch-lineage, contract-suite, waypoint-store]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "Plan 24-01's real Parley suspend/resume spine (AwaitingInput { parleys, responses }, WarEngine::resume_with, ParleyRequest/ParleyResponse); Plan 24-04's complete resume_with validation matrix, partial-answer persistence and lazy expiry, with build_waypoint/persist_waypoint promoted to pub(crate)"
provides:
  - "Waypoint.fork_of: Option<WaypointId> (paladin-core) -- additive, #[serde(default)], marks the branch ROOT and is inherited verbatim by every Waypoint of a run entered from a branch; None for every mainline Waypoint (D-14)"
  - "WaypointSummary.fork_of: Option<WaypointId> (paladin-ports) -- the same value, carried on the lightweight summary so a branch tree is reconstructible from a WaypointSummary list alone, without loading a single full Waypoint"
  - "ThreadId::child_on_branch(parent, branch_root, node) -> Result<Self, ThreadIdError> (paladin-core) -- a branch-scoped child thread id using the SAME length-prefixed injective encoding as ThreadId::child, extended to three components, so a fork's NodeSpec::Battalion subgraph child can never collide with the mainline child's thread (D-18)"
  - "fork_of threaded through the full superstep engine: build_waypoint/run_with_namespace/ChildEngineResources all carry it verbatim (mirroring checkpoint_ns's own threading pattern, but never concatenated); WarEngine::resume_with propagates latest.fork_of onto every Waypoint a resumed run produces, including the partial-answer and FailRun-expiry Waypoints"
  - "Three new WaypointPort contract-suite cases (awaiting_input_payload_round_trips, fork_of_round_trips, latest_prefers_most_recently_created_across_branches), registered for InMemory, SQLite and Postgres (Tier 2, self-skipping locally)"
  - "fork_of read cheaply in SQLite/Postgres's history() via json_extract(payload, '$.fork_of') / payload->>'fork_of' -- no dedicated column, no SQL migration"
  - "retention_protects_awaiting_input_on_any_branch -- the existing AwaitingInput wildcard protection proven unaffected by a branch-resident suspension"
affects: [24-07, 24-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A branch marker (fork_of) threaded through the engine exactly like an existing per-run namespace value (checkpoint_ns) -- new trailing parameter on build_waypoint/run_with_namespace, a new ChildEngineResources field, propagated verbatim (never concatenated, unlike checkpoint_ns's per-nesting-level namespace segments) since a branch root is a single value shared by the whole run tree"
    - "A WaypointSummary field the SQL backends cannot cheaply expose via an existing dedicated column is read straight out of the JSON payload column via a database-side JSON-extraction expression (SQLite json_extract, Postgres ->>), keeping history() as cheap as before and requiring no ALTER TABLE"
    - "A three-component injective thread-id derivation (ThreadId::child_on_branch) built by literally extending the existing two-component length-prefixed encoding (ThreadId::child) with one more fixed-width length-prefixed segment, reusing the exact CR-01 fix rather than inventing a new join strategy"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/waypoint.rs
    - crates/paladin-ports/src/output/waypoint_port.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-storage/src/waypoint/in_memory.rs
    - crates/paladin-storage/src/waypoint/sqlite.rs
    - crates/paladin-storage/src/waypoint/postgres.rs
    - crates/paladin-storage/src/waypoint/contract_tests.rs
    - crates/paladin-storage/src/waypoint/retention.rs
    - src/application/services/waypoint_retention.rs

key-decisions:
  - "Task 1 checkpoint (propagate-along-branch) auto-resolved by the orchestrator under auto-mode: fork_of marks the branch root and every Waypoint on the branch inherits it, never set only on the fork's first Waypoint -- reversible cost is one-way after v0.10.0 ships, free now"
  - "fork_of is threaded through the engine as a new trailing parameter on build_waypoint and run_with_namespace, and a new field on ChildEngineResources, mirroring checkpoint_ns's existing threading mechanism exactly -- but propagated VERBATIM to a Battalion child run rather than concatenated per nesting level, since a branch root is one value shared by the whole run tree, not a per-level namespace segment"
  - "No fork/replay entry point exists yet in this codebase (WarEngine::fork is a later plan's responsibility per the objective's own text) -- so every CURRENT call site (run(), the Battalion-child recursion via resources.fork_of, and the AwaitingInput/partial-answer/FailRun-expiry Waypoints resume_with constructs) either passes None or forwards latest.fork_of verbatim. The propagation machinery is fully wired and tested (build_waypoint pass-through, mainline-carries-None, a-fork-of-a-fork-carries-the-newer-root) even though nothing in this plan's own scope produces the first Some(root) value"
  - "resume_with's own three build_waypoint call sites (the FailRun-expiry Failed Waypoint, the D-11 partial-answer Waypoint, and the resumed run's own continuation via run_with_namespace) were all updated to forward latest.fork_of -- not named in Task 2's own <files> list (crates/paladin-battalion/src/engine/mod.rs), but required so a suspended branch's resume never silently reverts to mainline; documented as a Rule 3 mechanical necessity"
  - "WaypointSummary.fork_of is read from the JSON payload column via SQL-side extraction (SQLite json_extract, Postgres ->>'fork_of'), not a new dedicated column -- keeps history()'s summary query as cheap as before (no full-payload deserialization) while satisfying 'no SQL migration' and 'reconstructible from WaypointSummary alone' simultaneously; this is Claude's discretion beyond what the plan's action text specified, since visit_counts/frontier/muster_progress/checkpoint_ns are Waypoint-only fields with no WaypointSummary counterpart to use as a precedent for THIS specific constraint"
  - "retention_protects_awaiting_input_on_any_branch was added to crates/paladin-storage/src/waypoint/retention.rs's own test module, not contract_tests.rs -- Task 4's <files> list names only contract_tests.rs, but the branch-aware retention protection test naturally belongs beside the existing latest_and_awaiting_protected fixture and awaiting_input_waypoint_is_never_deleted_by_either_bound test it extends, mirroring that file's own established structure rather than duplicating its protection-set helper into contract_tests.rs"

patterns-established: []

requirements-completed: []

coverage:
  - id: D1
    description: "Waypoint and WaypointSummary each carry an additive fork_of field with serde default; a pre-D-14 payload missing the key deserialises as None"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#waypoint_payload_without_fork_of_deserializes_as_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/waypoint_port.rs#waypoint_summary_payload_without_fork_of_deserializes_as_none"
        status: pass
    human_judgment: false
  - id: D2
    description: "A branch is queryable: the fork's first Waypoint carries fork_of Some(from), every subsequent Waypoint on the branch inherits the same value, mainline stays None, and a fork of a fork carries the newer root"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#mainline_waypoints_carry_no_fork_of"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#branch_waypoints_inherit_fork_of_from_the_branch_root"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#fork_of_a_fork_carries_the_newer_root"
        status: pass
    human_judgment: false
  - id: D3
    description: "The whole branch tree is reconstructible from WaypointSummary alone, without loading full Waypoints"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/waypoint_port.rs#branch_tree_reconstructs_from_summaries_alone"
        status: pass
    human_judgment: false
  - id: D4
    description: "ThreadId::child_on_branch derives an injective, collision-free branch-scoped child thread id using the same length-prefixed encoding as ThreadId::child, extended to three components, never equal to the mainline child's id"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_injective"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_differs_from_child"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_is_deterministic"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#child_on_branch_rejects_invalid_inputs"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs (doc test) ThreadId::child_on_branch"
        status: pass
    human_judgment: false
  - id: D5
    description: "The three-backend contract suite gains round-trip cases for the AwaitingInput { parleys, responses } payload and the fork_of field, and a latest-across-branches ordering case; InMemory and SQLite run locally, Postgres is Tier 2 and self-skips without Docker"
    requirement: "HITL-03"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/contract_tests.rs#awaiting_input_payload_round_trips (InMemory + SQLite wrappers)"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/contract_tests.rs#fork_of_round_trips (InMemory + SQLite wrappers)"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/contract_tests.rs#latest_prefers_most_recently_created_across_branches (InMemory + SQLite wrappers)"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/retention.rs#retention_protects_awaiting_input_on_any_branch"
        status: pass
    human_judgment: false

duration: ~140min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 06: Chronicle Branch Lineage — fork_of, child_on_branch and the Contract Suite Summary

**`Waypoint`/`WaypointSummary` gain an additive `fork_of: Option<WaypointId>` branch marker threaded through the whole superstep engine, `ThreadId::child_on_branch` gives forked subgraphs collision-free thread ids, and the three-backend `WaypointPort` contract suite pins both the reshaped `AwaitingInput` payload and branch-aware `latest` ordering.**

## Performance

- **Duration:** ~140 min
- **Tasks:** 4 (Task 1 auto-resolved checkpoint, Tasks 2-4 all TDD RED/GREEN — merged into one plan-level RED/GREEN pair, see Deviations)
- **Files modified:** 10 (0 created)

## Accomplishments

- `Waypoint.fork_of: Option<WaypointId>` (paladin-core) and `WaypointSummary.fork_of` (paladin-ports) land additively (`#[serde(default)]`, no SQL migration — both SQL backends already store the whole `Waypoint` as a JSON payload column). `fork_of` marks the branch ROOT and is inherited by every Waypoint of that branch; mainline Waypoints carry `None`; a fork of a fork carries the newer root (D-14, Task 1's checkpoint auto-resolved `propagate-along-branch`).
- `fork_of` is threaded through the whole superstep engine exactly like `checkpoint_ns` already is: a new trailing parameter on `build_waypoint` and `run_with_namespace`, a new field on `ChildEngineResources` — but propagated VERBATIM to a `NodeSpec::Battalion` child run (never concatenated per nesting level, since a branch root is one value shared by the whole run tree). `WarEngine::resume_with` forwards `latest.fork_of` onto every Waypoint a resumed run produces: the resumed continuation's own Waypoints, a D-11 partial-answer Waypoint, and a D-12 `FailRun`-expiry `Failed` Waypoint — so a suspended branch's resume never silently reverts to mainline.
- `ThreadId::child_on_branch(parent, branch_root, node)` (paladin-core) derives a branch-scoped child thread id using the identical length-prefixed injective encoding `ThreadId::child` already uses, extended from two components to three — no delimiter join, reusing the exact Phase 22.1 CR-01 fix rather than reopening that collision class in a second place (D-18). Mainline runs keep calling `ThreadId::child` unchanged; this plan adds the primitive only — wiring it into the Battalion dispatch path is a later plan's responsibility (replay/fork, per this plan's own objective).
- The three-backend `WaypointPort` contract suite (`crates/paladin-storage/src/waypoint/contract_tests.rs`) gains `awaiting_input_payload_round_trips` (D-02), `fork_of_round_trips` (D-14) and `latest_prefers_most_recently_created_across_branches` (D-15), registered through the existing per-backend `#[tokio::test]` wrapper convention for `InMemoryWaypointStore`, `SqliteWaypointStore` and `PostgresWaypointStore`. SQLite and Postgres read `fork_of` cheaply out of the existing `payload` JSON/JSONB column via SQL-side extraction (`json_extract(payload, '$.fork_of')` / `payload->>'fork_of'`) rather than a new dedicated column — `history()`'s summary query stays as cheap as before, and no `migrations/` file is added.
- `retention_protects_awaiting_input_on_any_branch` (added to `crates/paladin-storage/src/waypoint/retention.rs`'s own test module, beside its existing `latest_and_awaiting_protected` fixture) proves the existing `AwaitingInput { .. }` wildcard protection is unaffected by the payload reshape: a branch-resident (`fork_of: Some(..)`) suspended Waypoint survives both the `max_age_days` and `max_waypoints_per_thread` bounds exactly like a mainline one.
- Postgres's three new contract-suite wrappers are Tier 2: `store_or_skip()` self-skips locally (no Docker in this devcontainer, confirmed via `SKIP: postgres-test not reachable at postgres://...` in the actual test run output) and they run only through CI's `postgres-integration` job — never recorded as passed from a local run, per the plan's own prohibition.

## Task Commits

1. **Task 1: Confirm the branch-lineage storage contract** — checkpoint auto-resolved by the orchestrator under auto-mode (`⚡ Auto-selected: propagate-along-branch`); no code commit, recorded here.
2. **Tasks 2-4 (fork_of field/propagation, ThreadId::child_on_branch, contract-suite cases) — one plan-level TDD RED/GREEN pair** (see Deviations for why all three tasks share one RED/GREEN boundary rather than three):
   - `b431cbf2` — `test(24-06): reproduce fork_of lineage and child_on_branch on not-yet-existing API (red)` — the full test surface for Tasks 2-4 added while `Waypoint.fork_of`, `WaypointSummary.fork_of` and `ThreadId::child_on_branch` are deliberately left absent (RED-STATE MARKER comments); 21 compile errors in `paladin-ai-core` alone (`E0560`/`E0609`/`E0599`), which blocks `paladin-ports`, `paladin-battalion` and `paladin-storage` from building at all since they depend on it.
   - `023bbcdc` — `feat(24-06): land fork_of lineage, child_on_branch and contract-suite cases (HITL-03, D-14/D-15/D-18)` — restores the three symbols; all tests green.

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/waypoint.rs` — `Waypoint.fork_of` field + doc; `ThreadId::child_on_branch` + doc test; both `new_root`/`new_child` constructors and every existing test-fixture `Waypoint` literal updated with `fork_of: None`; 6 new tests (2 for `fork_of`, 4 for `child_on_branch`).
- `crates/paladin-ports/src/output/waypoint_port.rs` — `WaypointSummary.fork_of` field + doc; existing round-trip test updated; 2 new tests (strip-key deserialization, branch-tree reconstruction from summaries alone).
- `crates/paladin-battalion/src/engine/superstep.rs` — `build_waypoint`/`run_with_namespace` gain the `fork_of` parameter; `ChildEngineResources` gains a `fork_of` field; all 13 `build_waypoint` call sites, the `run()` wrapper's forwarding call, and the Battalion-child recursion updated; 3 new tests (`mainline_waypoints_carry_no_fork_of`, `branch_waypoints_inherit_fork_of_from_the_branch_root`, `fork_of_a_fork_carries_the_newer_root`).
- `crates/paladin-battalion/src/engine/mod.rs` — `resume_with`'s two direct `build_waypoint` call sites (FailRun-expiry `Failed` Waypoint, D-11 partial-answer Waypoint) and its `run_with_namespace` continuation call all forward `latest.fork_of`.
- `crates/paladin-storage/src/waypoint/in_memory.rs` — `to_summary` copies `wp.fork_of`; 3 new contract-wrapper `#[tokio::test]`s.
- `crates/paladin-storage/src/waypoint/sqlite.rs` — `HISTORY_QUERY_{NO,WITH}_CURSOR` gain `json_extract(payload, '$.fork_of') AS fork_of`; `row_to_summary` extracts it; 3 new contract-wrapper `#[tokio::test]`s.
- `crates/paladin-storage/src/waypoint/postgres.rs` — same shape as `sqlite.rs`, using `payload->>'fork_of'`; 3 new Tier-2 contract-wrapper `#[tokio::test]`s (`store_or_skip`-gated).
- `crates/paladin-storage/src/waypoint/contract_tests.rs` — `sample_parley_request`/`sample_parley_response` fixture builders; `awaiting_input_payload_round_trips`, `fork_of_round_trips`, `latest_prefers_most_recently_created_across_branches`; all three added to `run_all`.
- `crates/paladin-storage/src/waypoint/retention.rs` — `retention_protects_awaiting_input_on_any_branch` test.
- `src/application/services/waypoint_retention.rs` — its test-only `to_summary` fixture builder copies `wp.fork_of`.

## Decisions Made

- **Task 1 checkpoint auto-resolved** (`propagate-along-branch`): `fork_of` marks the branch root and every Waypoint on the branch inherits it, per D-14's own recommendation — a branch becomes a queryable attribute and the tree reconstructs from `WaypointSummary` alone with no full-Waypoint loads. Reversibility: one-way after v0.10.0 ships; free now.
- **`fork_of` threading mirrors `checkpoint_ns`'s existing mechanism exactly** (new trailing parameter on `build_waypoint`/`run_with_namespace`, a new `ChildEngineResources` field) but propagates VERBATIM to a Battalion child run rather than concatenating a namespace segment per nesting level — a branch root is one value shared by the whole run tree, not a per-level path.
- **No fork/replay entry point exists in this plan's own scope** (`WarEngine::fork` is a later plan's responsibility, per this plan's own objective text: "the foundation `replay`/`fork` (plan 24-07) is built on"). Every CURRENT call site therefore either passes `None` (top-level `run()`) or forwards an already-established `fork_of` verbatim (the Battalion-child recursion via `resources.fork_of`, and `resume_with`'s three `build_waypoint` call sites via `latest.fork_of`). The propagation machinery is fully wired and tested even though nothing in this plan's own scope produces the first `Some(root)` value — that is exactly the "foundation" the objective describes.
- **`resume_with`'s own `build_waypoint`/`run_with_namespace` call sites were updated** (`crates/paladin-battalion/src/engine/mod.rs`, not named in Task 2's own `<files>` list) — a Rule 3 mechanical necessity: without forwarding `latest.fork_of`, a suspended branch's `FailRun`-expiry Waypoint, partial-answer Waypoint, or resumed continuation would all silently revert to `fork_of: None`, breaking D-14's "every Waypoint on the branch inherits it" contract the moment a branch thread is ever suspended and resumed.
- **`WaypointSummary.fork_of` is read via SQL-side JSON extraction** (`json_extract`/`->>'fork_of'`), not a new dedicated column — this satisfies both "no SQL migration" and "reconstructible from `WaypointSummary` alone" simultaneously, without forcing `history()` to deserialize the full (potentially large) `payload` column just to read one field. This is a design choice beyond the plan's own action text, since `visit_counts`/`frontier`/`muster_progress`/`checkpoint_ns` are `Waypoint`-only fields with no `WaypointSummary` counterpart to serve as a precedent for this specific constraint.
- **`retention_protects_awaiting_input_on_any_branch` lives in `crates/paladin-storage/src/waypoint/retention.rs`'s own test module**, not `contract_tests.rs` — Task 4's `<files>` list names only `contract_tests.rs`, but the branch-aware retention test naturally extends `retention.rs`'s own existing `latest_and_awaiting_protected` fixture and `awaiting_input_waypoint_is_never_deleted_by_either_bound` test, matching that file's established structure rather than duplicating its protection-set helper into `contract_tests.rs`.

## Deviations from Plan

### Architectural / process note (not a Rule 1-4 auto-fix)

**1. Tasks 2, 3 and 4 share ONE RED/GREEN commit pair instead of three**

- **Found during:** Planning the commit boundaries after drafting all three tasks' tests together.
- **Issue:** Task 2's `fork_of` field, Task 3's `ThreadId::child_on_branch`, and Task 4's contract-suite cases are genuinely independent pieces of surface (unlike 24-04's single-function rewrite), but a genuine per-task RED/GREEN split would still require reverting and reapplying the SAME field declarations across every dependent file for each task boundary, since Task 4's contract cases construct `Waypoint`/`WaypointSummary` values carrying `fork_of` and cannot compile without Task 2's field already present.
- **Resolution:** One RED commit (the full test surface for Tasks 2-4, with `Waypoint.fork_of`, `WaypointSummary.fork_of` and `ThreadId::child_on_branch` deliberately removed via RED-STATE MARKER comments — genuine compile failure, 21 errors in `paladin-ai-core` alone, which transitively blocks `paladin-ports`/`paladin-battalion`/`paladin-storage`) followed by ONE GREEN commit restoring exactly those three symbols. Mirrors 24-01's/24-02's/24-04's own precedent for the identical underlying reason: when later tasks' tests structurally require an earlier task's own production code to even compile, Rust's own compilation model does not allow an intermediate state to be independently green.
- **Verification:** `cargo check -p paladin-ai-core --all-features --tests` — 21 `error[...]` lines before the GREEN commit; `cargo test --workspace` (all green except the documented pre-existing `e2e_crash_resume` flake, see Issues Encountered), `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` (zero warnings) all verified clean after the GREEN commit.
- **Committed in:** `b431cbf2` (RED), `023bbcdc` (GREEN).

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `resume_with`'s own three call sites required editing `crates/paladin-battalion/src/engine/mod.rs`, not listed in Task 2's `<files>`**

- **Found during:** Task 2 (threading `fork_of` through the engine)
- **Issue:** Task 2's `<files>` list names `superstep.rs` but not `mod.rs`. Adding a `fork_of` parameter to `run_with_namespace`/`build_waypoint` is a Rust-structural requirement affecting every call site, including the three in `mod.rs`'s `resume_with` (the `FailRun`-expiry `Failed` Waypoint, the D-11 partial-answer Waypoint, and the resumed continuation's own `run_with_namespace` call) — omitting them would leave `resume_with` silently resetting `fork_of` to `None` on every resumed Waypoint, defeating D-14's inheritance contract for the exact code path (suspend/resume) this phase's own HITL-01/02 machinery exercises constantly.
- **Fix:** Each of the three call sites now forwards `latest.fork_of` (the just-loaded `AwaitingInput` Waypoint's own branch marker) instead of a bare `None`.
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`
- **Verification:** `cargo test -p paladin-battalion --lib` (536 passed, including all pre-existing `resume_with`/partial-answer/expiry tests from plan 24-04, unaffected).
- **Committed in:** `023bbcdc` (GREEN commit)

**2. [Rule 3 - Blocking] `useless_vec` clippy lint on the new `branch_tree_reconstructs_from_summaries_alone` fixture**

- **Found during:** Post-implementation `cargo clippy --workspace --all-targets -- -D warnings`
- **Issue:** The new test built a `Vec<WaypointSummary>` fixture via `vec![...]` that is only ever iterated (`.iter()`), never mutated or grown — clippy's `useless_vec` lint (deny-by-default under `-D warnings`) flagged it.
- **Fix:** Changed `vec![...]` to a plain array literal `[...]`.
- **Files modified:** `crates/paladin-ports/src/output/waypoint_port.rs` (test only)
- **Verification:** `cargo clippy -p paladin-ports --all-targets -- -D warnings` clean.
- **Committed in:** `023bbcdc` (GREEN commit)

---

**Total deviations:** 1 architectural/process note (commit-boundary merge, mirrors prior plans' own precedent) + 2 auto-fixed (1 blocking mod.rs edit necessary for D-14's own inheritance contract, 1 clippy lint fix in test-only code). **Impact on plan:** No scope creep — the `mod.rs` edit is a direct, minimal consequence of `fork_of`'s own propagation requirement; the clippy fix is mechanical.

## Issues Encountered

- **`e2e_1_crash_resume_matches_control_run_with_no_reexecution` is flaky under full-workspace parallel test contention** — the exact pre-existing flake documented in `24-02-SUMMARY.md`'s, `24-03-SUMMARY.md`'s and `24-04-SUMMARY.md`'s own "Issues Encountered" sections (a 30-second timeout guard in `tests/integration/e2e_crash_resume_test.rs`, unrelated to `fork_of`/`child_on_branch`/the contract suite). Observed failing once under `cargo test --workspace` (CPU contention from the full parallel suite); confirmed unrelated to this plan's changes by running it in isolation (`cargo test -p paladin-ai --test e2e_crash_resume`), which passed in 1.49s, well under its 30s guard.
- **Postgres contract-suite wrappers confirmed self-skipping, not silently passing.** Ran `cargo test -p paladin-storage --lib --all-features -- --nocapture fork_of_round_trips` explicitly to confirm the actual skip message (`SKIP: postgres-test not reachable at postgres://paladin:paladin@localhost:5433/paladin_waypoint_test`) appears rather than trusting a bare "ok" result — this devcontainer has no Docker, so the three new Postgres wrappers are Tier 2 and were never run against a real Postgres instance; they are recorded here as routed to CI's `postgres-integration` job only.
- **No pre-commit hook timeout encountered this plan** — both commits used `--no-verify` per the orchestrator's `workflow.worktree_skip_hooks=true` allowance for this run (the cold `cargo clippy --workspace --all-targets --all-features` pre-commit hook would exceed the 2-minute command timeout); `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` (zero warnings across the full workspace) were both run and verified clean before this SUMMARY was written, in addition to `cargo test --workspace --no-fail-fast`.

## User Setup Required

None — no external service configuration required.

## Note on REQUIREMENTS.md

`requirements-completed` in this SUMMARY's frontmatter is deliberately empty, and `.planning/REQUIREMENTS.md`'s `HITL-03` checkbox was **not** marked complete. This plan's own frontmatter lists `requirements: [HITL-03]`, but per this plan's own objective text, `replay`/`fork`, `ChronicleService` (`history`/`inspect`/`latest_on_branch`) and the immutability/subgraph-fork proof tests are explicitly a LATER plan's responsibility ("the foundation `replay`/`fork` (plan 24-07) is built on"). This plan lands only the storage-contract half of HITL-03 (D-14's `fork_of` field and its propagation, D-18's `child_on_branch` primitive, and the D-02/D-14/D-15 contract-suite cases) — not the engine-level `replay`/`fork` operations or the read-facade HITL-03 itself promises. Whichever later plan lands `WarEngine::replay`/`WarEngine::fork`/`ChronicleService` should be the one to run `gsd_run query requirements.mark-complete HITL-03`.

## Next Phase Readiness

- The branch-lineage storage contract is fully wired and proven at the plumbing level: `fork_of` propagates correctly through every current engine code path (mainline runs carry `None`, a value passed in propagates verbatim across supersteps and through `resume_with`, a fork of a fork carries the newer root), `ThreadId::child_on_branch` is a proven-injective primitive ready to be wired into the Battalion dispatch path, and the three-backend contract suite pins both the reshaped `AwaitingInput` payload and branch-aware `latest` ordering as load-bearing guarantees rather than incidental properties.
- Plan 24-07 (or whichever plan lands `replay`/`fork`) can call `Waypoint`'s existing construction path with a real `Some(root)` `fork_of` value and expect every downstream consumer (WaypointSummary construction, the three storage backends, retention's protection logic) to already handle it correctly — proven by this plan's own tests, not merely assumed.
- No blockers. The `e2e_crash_resume` timing flake (Issues Encountered) is pre-existing, unrelated to this plan, and already documented across three prior plans' own SUMMARYs; it does not gate this plan's completion. The Postgres Tier-2 contract-suite concern carried forward from Phase 22/23/24 plans is unaffected — this plan's three new Postgres wrappers self-skip identically to the pre-existing ones.

## Self-Check: PASSED

All 10 modified files verified present on disk; both commit hashes (`b431cbf2` RED, `023bbcdc` GREEN) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
