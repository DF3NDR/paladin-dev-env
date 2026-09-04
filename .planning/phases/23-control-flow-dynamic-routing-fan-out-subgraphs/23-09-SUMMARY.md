---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 09
subsystem: engine
tags: [rust, superstep-engine, waypoint, subgraph, thread-identity, checkpointing, resume]

# Dependency graph
requires:
  - phase: 23-08
    provides: "NodeSpec::Battalion variant, StateMap, child-engine inheritance, the boxed-future execute_vanguard_node recursive dispatch, and the clearly-marked child-thread seam this plan replaces"
provides:
  - "ThreadId::child(parent, node) -- injective, length-prefixed child thread derivation with an adversarial collision test"
  - "Waypoint.checkpoint_ns: Option<String> -- additive namespace-path record, observability only"
  - "Resume-mid-child: a Battalion dispatch checks latest(child_thread) and resumes the child from there instead of restarting it"
  - "restart_on_resume: true opt-out, abandon-not-delete policy for the old child chain"
  - "checkpoint_ns contract-suite round-trip coverage across in-memory/sqlite/postgres"
  - "CF-FR-17 Formation-inside-branching-Campaign integration test with the kill-after-child-superstep-1 resume assertion"
affects: [24-hitl-parley-resume, 25-per-task-retry-and-aegis]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Text-safe length-prefixed identity derivation: fixed-width lowercase-hex byte-length prefixes ahead of each segment, adapted from graph.rs's push_field (binary) so the encoded result stays a valid whitespace-free UTF-8 ThreadId"
    - "Wrapper/impl split to avoid a required-parameter ripple into files another concurrent plan owns: run() keeps its original public signature and forwards to a new private run_with_namespace(..., checkpoint_ns) that only the Battalion recursive dispatch calls directly"
    - "Seed-from-a-real-run crash simulation extended from a single thread (e2e_crash_resume_test.rs) to a parent thread + its derived child thread simultaneously"

key-files:
  created:
    - tests/integration/subgraph_formation_in_campaign_test.rs
  modified:
    - crates/paladin-core/src/platform/container/waypoint.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-storage/src/waypoint/contract_tests.rs
    - crates/paladin-storage/src/waypoint/in_memory.rs
    - crates/paladin-storage/src/waypoint/sqlite.rs
    - crates/paladin-storage/src/waypoint/postgres.rs
    - Cargo.toml

key-decisions:
  - "Checkpoint auto-selected under GSD auto-mode: option-a (D-20 as written -- length-prefixed injective derivation + additive checkpoint_ns, latest(child_thread) as the child's own resume point). Options b (bare delimiter join) and c (disambiguate by checkpoint_ns alone, keeping the parent's ThreadId) rejected per CONTEXT.md D-20's own reasoning: b is demonstrably collidable given NodeId's unrestricted charset (the exact CR-01 defect class); c would require a WaypointPort method change D-20 explicitly forbids."
  - "ThreadId::child uses a fixed-width 16-hex-digit (64-bit) length prefix per segment, not graph.rs's raw binary push_field prefix -- a raw binary u64 length prefix can itself contain a whitespace byte or an invalid-UTF-8 byte, either of which would corrupt or reject the resulting ThreadId string. The hex encoding keeps the same injectivity property (fixed width means the split point is always deterministic) while staying text-safe."
  - "run()'s public signature is left completely unchanged; a new private run_with_namespace(..., checkpoint_ns) carries the actual implementation, called by run() with None and by the Battalion dispatch's recursive call with the derived child namespace. This was a deviation from the plan's literal 'thread checkpoint_ns through run()' framing, made because graph.rs (owned by the concurrently-running 23-10 worktree) has its own direct test call to superstep::run whose signature this plan must not change."
  - "An already-Completed child found at latest(child_thread) short-circuits straight to the mapped output delta without re-invoking run() -- covers the case where a crash lands between the child's own completion and the parent's own next Waypoint write, without writing a redundant child Waypoint."
  - "restart_on_resume: true abandons (never deletes) the old child chain under the same child ThreadId; the next fresh run starts a new root-shaped Waypoint lineage on that same thread. D-20 requires no retention change, and none was made -- WaypointRetentionService and crates/paladin-storage/src/waypoint/retention.rs are both untouched (confirmed via empty git diff)."

patterns-established:
  - "Wrapper/impl split for a required-parameter addition when the public function is called from a file outside this plan's edit scope"
  - "Nested Battalion checkpoint_ns composition test pattern (grandchild derives from child derives from root) as a template for future nesting-depth proofs"

requirements-completed: [CF-04]

coverage:
  - id: D1
    description: "ThreadId::child(parent, node) derives an injective, length-prefixed child thread id, proven adversarially against the exact CR-01 collision shape, composing correctly for nested (grandchild) derivation, and failing typed (never truncating) when the result would exceed ThreadId's own limits"
    requirement: "CF-04"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::child_thread_derivation_is_injective_under_adversarial_names"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::derived_child_thread_id_passes_thread_id_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::nested_child_thread_ids_compose"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::derived_child_thread_id_exceeding_max_len_fails_typed_rather_than_truncating"
        status: pass
    human_judgment: false
  - id: D2
    description: "Waypoint.checkpoint_ns is an additive #[serde(default)] field recording the namespace path, round-tripping through serde and through all three WaypointPort backends"
    requirement: "CF-04"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::waypoint_payload_without_checkpoint_ns_deserializes_as_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/in_memory.rs#waypoint::in_memory::tests::checkpoint_ns_round_trips"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/sqlite.rs#waypoint::sqlite::tests::checkpoint_ns_round_trips"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/postgres.rs#waypoint::postgres::tests::checkpoint_ns_round_trips"
        status: unknown
    human_judgment: true
    rationale: "The Postgres tier compiles and its skip-gracefully path is confirmed (prints SKIP, returns without asserting) since Docker is unavailable in this execution environment. The clause has never actually executed against a live Postgres server; CI's postgres-integration job is the first real run and should be checked there before this deliverable is treated as fully proven on that backend."
  - id: D3
    description: "A parent resumed mid-child resumes the child from latest(child_thread) with zero re-execution of already-completed child work; restart_on_resume: true opts out and runs the child fresh"
    requirement: "CF-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::resume_of_a_parent_mid_child_resumes_the_child_where_it_stopped"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::restart_on_resume_true_runs_the_child_fresh"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::latest_on_the_child_thread_returns_the_childs_own_waypoint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::child_threads_are_ordinary_threads_for_retention"
        status: pass
      - kind: integration
        ref: "tests/integration/subgraph_formation_in_campaign_test.rs#killing_after_the_childs_first_superstep_and_resuming_does_not_repeat_child_work"
        status: pass
    human_judgment: false
  - id: D4
    description: "A Formation subgraph embedded as a node of a BRANCHING parent graph runs correctly -- the untaken branch is proven NotFiring, the Formation's nodes execute in sequential order, and its mapped output reaches the parent"
    requirement: "CF-04"
    verification:
      - kind: integration
        ref: "tests/integration/subgraph_formation_in_campaign_test.rs#formation_subgraph_runs_as_a_node_of_a_branching_parent_graph"
        status: pass
      - kind: integration
        ref: "tests/integration/subgraph_formation_in_campaign_test.rs#phalanx_and_campaign_bridges_also_embed"
        status: pass
    human_judgment: false

duration: ~50min
completed: 2026-09-04
status: complete
---

# Phase 23 Plan 09: Derived Child ThreadId, checkpoint_ns, and Resume-Mid-Child Summary

**Injective, length-prefixed child ThreadId derivation (`ThreadId::child`), additive `Waypoint.checkpoint_ns` observability field, and zero-re-execution resume-mid-child for `NodeSpec::Battalion` nodes, proven end-to-end by a branching Formation-in-Campaign integration test with a real kill-after-child-superstep-1 resume.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 3 (checkpoint auto-selected, Task 1, Task 2)
- **Files modified:** 7 modified, 1 created

## Checkpoint Status

**Task 1 was a `checkpoint:decision` (gate="blocking") on CONTEXT.md D-20.** The orchestrator runs this phase under GSD auto-mode (`_auto_chain_active: true`), where a `decision` checkpoint auto-selects the first (recommended) option unless the gate is `blocking-human`. Per the orchestrator's pre-resolution instruction, this checkpoint was **auto-selected: option-a** — "D-20 as written — length-prefixed (or escaped) injective derivation, plus an additive `#[serde(default)] checkpoint_ns` record, with `latest(child_thread)` as the child's own resume point." Options b (bare delimiter join — demonstrably collidable given `NodeId`'s unrestricted charset, the exact CR-01 defect class) and c (disambiguate by `checkpoint_ns` alone, keeping the parent's `ThreadId` — would require a `WaypointPort` method change D-20 explicitly forbids) were rejected, matching CONTEXT.md D-20's own stated reasoning. No human interactively confirmed this door; it is recorded here as required by the auto-mode carve-out for `blocking` (non-`blocking-human`) decision checkpoints.

## Accomplishments

- `ThreadId::child(parent, node)` derives a provably injective child thread id using a fixed-width (16 lowercase-hex-digit) length-prefix encoding per segment — a text-safe adaptation of `graph.rs`'s `push_field` binary length prefix, needed because a raw binary `u64::to_le_bytes()` prefix can itself contain a whitespace byte or an invalid-UTF-8 byte, either of which would corrupt or outright reject the resulting `ThreadId` string. Proven adversarially against the exact 22.1 CR-01 collision shape, composes correctly across nested (grandchild) derivation, and fails typed (`ThreadIdError::TooLong`) rather than truncating when the encoded result would exceed `THREAD_ID_MAX_LEN`.
- `Waypoint.checkpoint_ns: Option<String>` is an additive `#[serde(default)]` field recording the namespace path (`"outer/inner/"`, nested paths concatenating) for observability only — isolation between a parent's and a child's Waypoints comes entirely from the distinct derived `ThreadId`, never from filtering by `checkpoint_ns`.
- `execute_vanguard_node`'s `NodeSpec::Battalion` dispatch arm now: derives the child thread via `ThreadId::child`; computes and stamps the child's `checkpoint_ns`; checks `latest(child_thread)` through the SAME `WaypointPort` before deciding whether to resume the child from there (zero re-execution of already-completed child work) or seed it fresh; short-circuits an already-`Completed` prior child straight to its mapped output delta without a redundant re-run; and honors `restart_on_resume: true` by always starting fresh, abandoning (never deleting) the old child chain.
- No `WaypointPort` method change and no `WaypointRetentionService`/`crates/paladin-storage/src/waypoint/retention.rs` change — confirmed via empty `git diff` on both, per D-20.
- `checkpoint_ns_round_trips`/`checkpoint_ns_none_round_trips` contract-suite clauses added to `contract_tests.rs`'s `run_all` aggregate and as named `#[tokio::test]`s in all three backend modules (`in_memory.rs`, `sqlite.rs`, `postgres.rs`), matching the `muster_progress` precedent exactly.
- `tests/integration/subgraph_formation_in_campaign_test.rs` (new Tier 1 `[[test]]` target): a branching parent graph (`router -> other_arm | sub`, the untaken branch proven `NotFiring` by asserting its output field stays unset) whose `sub` node embeds `from_formation(3 paladins)` unchanged via `Arc::new(...)`; a kill-after-child-superstep-1 → resume → assert-no-repeat test that seeds a fresh `SqliteWaypointStore` with the REAL first-superstep Waypoints of both the parent thread and the child's own `ThreadId::child`-derived thread (extending `e2e_crash_resume_test.rs`'s re-seeding technique to two threads at once); and a lighter test proving `from_phalanx`/`from_campaign` also construct into a validating `NodeSpec::Battalion`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Derived child threads, namespaced checkpoints, and resume mid-child** — `ea0defd1` (feat)
2. **Task 2: checkpoint_ns contract coverage and the Formation-inside-Campaign integration test** — `a0246489` (test)

_Checkpoint (D-20 decision) resolved via orchestrator auto-mode pre-resolution — no separate commit; recorded in Checkpoint Status above._

**Plan metadata:** this file's own commit (docs), created by the worktree-mode git_commit_metadata step.

## Files Created/Modified

- `crates/paladin-core/src/platform/container/waypoint.rs` — `ThreadId::child`, `Waypoint.checkpoint_ns`, and their tests including the adversarial collision test and the doc test.
- `crates/paladin-battalion/src/engine/superstep.rs` — the `NodeSpec::Battalion` dispatch arm's derived-thread/checkpoint_ns/resume-mid-child rewrite; the `run()`/`run_with_namespace` wrapper split; five new integration-level unit tests.
- `crates/paladin-storage/src/waypoint/contract_tests.rs` — `checkpoint_ns_round_trips`/`checkpoint_ns_none_round_trips` clauses.
- `crates/paladin-storage/src/waypoint/{in_memory,sqlite,postgres}.rs` — named `#[tokio::test]`s invoking the new clauses.
- `tests/integration/subgraph_formation_in_campaign_test.rs` — new file, the CF-FR-17 integration test.
- `Cargo.toml` — the `[[test]]` entry for the new integration test target.

## Decisions Made

- **`run()`'s public signature is unchanged; a new private `run_with_namespace` carries `checkpoint_ns`.** The plan's own text described threading `checkpoint_ns` through `run()` directly. Doing so literally would have required adding a 20th positional parameter to `pub(crate) async fn run`, which `crates/paladin-battalion/src/engine/graph.rs`'s own test module calls directly (`crate::engine::superstep::run(...)` at `graph.rs:1887`) — and `graph.rs` is owned by the concurrently-running 23-10 worktree agent, which this plan is instructed not to touch. Instead, `run()` keeps its original 19-parameter signature and forwards unconditionally to a new private `run_with_namespace(..., checkpoint_ns: Option<String>)`, which carries the actual implementation. `run()` itself passes `None`; only `execute_vanguard_node`'s `NodeSpec::Battalion` recursive dispatch calls `run_with_namespace` directly, with the derived child namespace. No other call site (mod.rs's `start`/`resume_with_options`, or any of superstep.rs's own pre-existing test helpers) needed any signature-shape change at all.
- **Fixed-width hex length-prefix, not `graph.rs`'s raw binary prefix.** `push_field` (the CR-01 fix this plan explicitly reuses the *approach* of) writes a raw `u64::to_le_bytes()` length prefix into an opaque byte buffer that only ever gets hashed — it never needs to survive as a valid `String`. A `ThreadId` DOES need to survive as a valid, whitespace-free UTF-8 `String`, and a raw binary length prefix can trivially contain a byte in the ASCII whitespace range (rejected by `ThreadId::new`) or a byte that is not valid UTF-8 on its own. A fixed-width (16-digit) lowercase-hex encoding of the same length value keeps the identical injectivity guarantee (the split point between segments is always fully determined by the fixed-width prefix, never by scanning for a delimiter) while staying entirely printable ASCII.
- **An already-`Completed` child found at `latest(child_thread)` short-circuits without calling `run()`/`run_with_namespace` again.** Covers the case where a crash lands between the child's own completion and the parent's own next Waypoint write (the recursive dispatch had not yet returned). Avoids writing a redundant Completed Waypoint for a child that already has one.
- **`restart_on_resume: true` abandons, never deletes, the old child chain.** The next fresh run starts a NEW root-shaped Waypoint lineage under the SAME child `ThreadId` (`parent_waypoint_id: None` again). D-20 permits either abandon-or-overwrite as Claude's discretion; abandon was chosen because it requires zero new deletion logic and keeps the old chain available for forensic inspection, at the acceptable cost that `WaypointPort::latest` on that thread now returns the fresh chain's Waypoints rather than the abandoned one's.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `run()`'s public signature could not gain a 20th parameter without breaking a file owned by the concurrent 23-10 worktree**
- **Found during:** Task 1, first `cargo test -p paladin-battalion --lib` after adding `checkpoint_ns` to `run()`'s literal signature.
- **Issue:** `crates/paladin-battalion/src/engine/graph.rs:1887` (a test in `graph.rs`, which this plan is explicitly forbidden from touching — it is 23-10's file) calls `crate::engine::superstep::run(...)` directly with the original 19-argument shape. Adding `checkpoint_ns` as a 20th required parameter to `run()` broke that call site with `E0061: this function takes 20 arguments but 19 arguments were supplied`.
- **Fix:** Reverted all trailing-argument additions at `run()`'s existing call sites (mod.rs's two calls, and superstep.rs's own pre-existing test helpers), restored `run()`'s original 19-parameter signature, and introduced a new private `run_with_namespace(..., checkpoint_ns: Option<String>)` carrying the actual implementation. `run()` now forwards to it with `checkpoint_ns: None`; only the Battalion dispatch's recursive call invokes `run_with_namespace` directly.
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs` (part of the same Task 1 commit; `mod.rs` ended up with a net-zero diff since its edits were fully reverted).
- **Verification:** `cargo check -p paladin-battalion` clean; `graph.rs`'s own test suite untouched and unbroken (verified via full `cargo test -p paladin-battalion --lib`, 464 passed).
- **Committed in:** `ea0defd1` (Task 1 commit).

**2. [Rule 1 - Bug] Two clippy failures surfaced only under `--workspace --all-targets --all-features`**
- **Found during:** the full-workspace clippy pass run before Task 2's final commit (narrower per-crate clippy runs after Task 1 alone did not catch these).
- **Issue:** (a) `crates/paladin-core/src/platform/container/waypoint.rs`'s `ThreadId::child` rustdoc triggered `clippy::doc_lazy_continuation` — a line starting with `+ node ...` inside a doc comment was parsed as an unindented markdown list continuation. (b) `crates/paladin-battalion/src/engine/superstep.rs`'s `build_waypoint` had a duplicated `#[allow(clippy::too_many_arguments)]` attribute (from an editing overlap), and the Battalion dispatch's already-Completed-child short-circuit had a `clippy::collapsible_if` (nested `if let Some(latest) = &existing_latest { if matches!(...) { ... } }`).
- **Fix:** Rephrased the doc comment to avoid a line starting with `+`; removed the duplicate attribute; collapsed the nested `if` into a single `if let ... && matches!(...)` using Rust's let-chains.
- **Files modified:** `crates/paladin-core/src/platform/container/waypoint.rs`, `crates/paladin-battalion/src/engine/superstep.rs`.
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; re-ran the full `paladin-battalion --lib` and `paladin-ai-core --lib waypoint` suites after the fix (unchanged pass counts).
- **Committed in:** `a0246489` (Task 2 commit, alongside the contract/integration-test work already in progress when the full clippy pass was run).

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug/lint).
**Impact on plan:** Both fixes were necessary for correctness (deviation 1) and for the mandated clean-clippy bar (deviation 2). No scope creep — no behavior changed beyond what Task 1/Task 2 already specified.

## Known Stubs

None. `killing_after_the_childs_first_superstep_and_resuming_does_not_repeat_child_work` and `formation_subgraph_runs_as_a_node_of_a_branching_parent_graph` are both real, fully-asserting tests against real engine behavior, not scaffolding.

## Issues Encountered

- **Postgres contract-suite tier not run against a live server.** Per the parallel-execution instructions, Docker is not available in this environment, so `checkpoint_ns_round_trips`/`checkpoint_ns_none_round_trips` on `PostgresWaypointStore` compile and exercise their `store_or_skip()` skip-gracefully path (confirmed via `--nocapture` that the SKIP message prints and the test returns without asserting), but have never executed against a real Postgres instance locally. `make test-integration-docker`'s `postgres-integration` CI job is the first real run against a live server for this specific clause and should be checked there. Recorded as `unknown`/`human_judgment: true` in the coverage block above (deliverable D2) rather than claimed as proven.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- CF-FR-15/CF-FR-17 (subgraph identity, namespacing, resume-mid-child, legacy-bridge embedding) are fully implemented and tested; `NodeSpec::Battalion` subgraph composition (Plan 23-08 + this plan) is complete pending the sibling 23-10 plan's fingerprint `v3` work landing independently in `graph.rs`.
- No blockers for Phase 23's remaining waves. The Postgres tier's live-server confirmation is a CI-only follow-up, not a code blocker — the contract clause is written, compiles, and is invoked identically to every other backend clause.

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-04*

## Self-Check: PASSED

- FOUND: `crates/paladin-core/src/platform/container/waypoint.rs`
- FOUND: `crates/paladin-battalion/src/engine/superstep.rs`
- FOUND: `crates/paladin-battalion/src/engine/mod.rs`
- FOUND: `crates/paladin-storage/src/waypoint/contract_tests.rs`
- FOUND: `tests/integration/subgraph_formation_in_campaign_test.rs`
- FOUND: `Cargo.toml`
- FOUND: commit `ea0defd1` (Task 1)
- FOUND: commit `a0246489` (Task 2)
- All named tests referenced in the coverage block re-verified passing at time of writing (see command transcript in execution).
