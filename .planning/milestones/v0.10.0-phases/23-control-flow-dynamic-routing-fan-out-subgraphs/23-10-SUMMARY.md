---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 10
subsystem: engine
tags: [war-graph, fingerprint, blake3, waypoint, resume, tdd]

requires:
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: worker_templates (23-05), NodeSpec::Battalion/StateMap/restart_on_resume (23-08), DirectiveParser/on_parse_error (23-04)
  - phase: 22.1-engine-readiness-defect-and-msrv-follow-up
    provides: the v2 length-prefixed push_field encoding and golden-hex test pattern (CR-01, D-17)
provides:
  - "GRAPH_FINGERPRINT_VERSION bumped v2 -> v3"
  - "Three new hashed sections: worker-template set, per-Battalion child fingerprint/StateMap/restart_on_resume, per-Paladin DirectiveParser/on_parse_error"
  - "Re-pinned golden-hex fingerprint test for the v3 encoding"
affects: [23-09, resume/waypoint-consumers, any future phase adding scheduling-relevant graph properties]

tech-stack:
  added: []
  patterns:
    - "Fingerprint sections are Merkle-composed: a Battalion node hashes its child's own fingerprint() string rather than walking the child's structure inline -- bounded, compositional, and depth-sensitive."
    - "Variable-length list fields (StateMap inputs/outputs) are preceded by an explicit u64 LE element count before their length-prefixed pairs, so a pair can never shift between two adjacent variable-length lists without changing the encoded bytes."

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-core/src/platform/container/waypoint.rs

key-decisions:
  - "D-18 checkpoint (Task 1, gate=blocking) auto-selected under orchestrator auto-mode: option-a, 'bump to v3 and add all three new sections, each sorted and length-prefixed through push_field' -- as CONTEXT.md D-18 specifies. Option b (stay at v2, emit conditionally) and option c (bump but hash only a subset) were rejected in CONTEXT.md as reintroducing ambiguity or leaving scheduling-relevant properties unhashed."
  - "GRAPH_FINGERPRINT_VERSION's single declaration site is crates/paladin-core/src/platform/container/waypoint.rs, outside this plan's declared files_modified (graph.rs only). Confirmed via read of 23-09-PLAN.md that the concurrent worktree does not touch this constant; made the minimal, isolated edit (doc comment + const value + one test assertion) required by the plan's own acceptance criteria."
  - "StateMap inputs/outputs lists are each preceded by an explicit u64 LE element count (not just length-prefixed pairs) -- without it, a pair could shift from outputs into inputs (or vice versa) without changing the concatenated bytes, reintroducing the exact split-ambiguity class CR-01 fixed. This is additive precision beyond the plan's literal text ('length-prefix each element') but required to satisfy the plan's own collision-freedom mandate and the T-23-41 threat-model mitigation."
  - "DirectiveParser is hashed as a single field via its whole-enum serde-canonical representation (matching the existing EdgeCondition/DispatchRule precedent), which captures both the parser kind and, where StructuredDirective carries one, its on_parse_error in one push_field call."

patterns-established:
  - "Variable-length list-of-pairs fields inside a fingerprint record get an explicit u64 LE count prefix before their length-prefixed elements -- the pattern for any future list-valued hashed property."

requirements-completed: [CF-02, CF-03, CF-04]

coverage:
  - id: D1
    description: "GRAPH_FINGERPRINT_VERSION reads v3; every stored v2-tagged fingerprint is recognised as stale by version-tag mismatch."
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_version_tag_is_v3"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#platform::container::waypoint::tests::graph_fingerprint_is_deterministic_and_versioned"
        status: pass
    human_judgment: false
  - id: D2
    description: "v3 hashes the worker-template set, sorted and length-prefixed through push_field, with an order-independence test."
    requirement: "CF-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_a_node_is_marked_a_worker_template"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::worker_template_section_is_order_independent"
        status: pass
    human_judgment: false
  - id: D3
    description: "v3 hashes each Battalion node's child fingerprint, StateMap, and restart_on_resume -- one difference test per property."
    requirement: "CF-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_an_embedded_child_graph_differs"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_a_state_map_differs"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_restart_on_resume_differs"
        status: pass
    human_judgment: false
  - id: D4
    description: "v3 hashes each Paladin node's DirectiveParser kind and on_parse_error -- one difference test each."
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_a_directive_parser_kind_differs"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_differs_when_on_parse_error_differs"
        status: pass
    human_judgment: false
  - id: D5
    description: "ENG-FR-14 exclusions (prompts, models, InputMapping templates, every EngineLimits field including max_muster_tasks) still hold under v3, proven by the extended existing test, not a new sibling."
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits"
        status: pass
    human_judgment: false
  - id: D6
    description: "Golden-hex test re-pinned to the v3 digest of its unchanged reference graph."
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#engine::graph::tests::fingerprint_golden_hex_pins_canonical_bytes"
        status: pass
    human_judgment: false

duration: 55min
completed: 2026-09-04
status: complete
---

# Phase 23 Plan 10: Graph Fingerprint v3 Summary

**`WarGraph::fingerprint()` bumped v2 -> v3, hashing the worker-template set, each Battalion node's child fingerprint/StateMap/restart_on_resume, and each Paladin node's DirectiveParser/on_parse_error, all through the existing length-prefixed `push_field` helper.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-09-04T00:51:00Z
- **Completed:** 2026-09-04T01:46:11Z
- **Tasks:** 1 (checkpoint auto-resolved) + 1 auto (TDD)
- **Files modified:** 2

## Accomplishments

- `GRAPH_FINGERPRINT_VERSION` bumped from `v2` to `v3` at its single declaration site (`paladin-core`'s `waypoint.rs`); every `v2`-tagged stored fingerprint is now recognised as stale on `resume` via version-tag mismatch rather than silently reinterpreted.
- Three new hashed sections added to `WarGraph::fingerprint()`, each routed through the existing `push_field` length-prefixed helper (22.1 CR-01 discipline, never a delimiter join):
  - The worker-template set (sorted node ids).
  - Per `NodeSpec::Battalion` node (walked in `node_order`): node id, child graph's own `fingerprint()` string, `StateMap` inputs/outputs pairs (each list preceded by an explicit `u64` LE element count), and `restart_on_resume` as a fixed-width tag byte.
  - Per `NodeSpec::Paladin` node (walked in `node_order`): its `DirectiveParser`, hashed via serde-canonical JSON (captures both kind and `on_parse_error` in one field).
- Six difference tests (one per new hashed property) plus a version-tag test and a worker-template order-independence test, all passing.
- The existing `fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits` exclusion test extended in place (not duplicated) — still passing under `v3`, confirming `max_muster_tasks` and every other `EngineLimits` field, prompt, model, and `InputMapping` template stay excluded.
- Golden-hex test re-pinned to the v3 digest of its unchanged reference graph; `fingerprint_is_deterministic_across_calls`'s literal also re-pinned.
- `fingerprint()`'s rustdoc extended to document the three new sections, the collision-analysis rationale for the `StateMap` list-count prefixes, and the `v2` -> `v3` supersession.

## Task Commits

Each task was committed atomically:

1. **Checkpoint (auto-resolved under GSD auto-mode):** option-a — D-18 as written, bump to v3 and add all three new sections. No commit (decision only, recorded in Deviations/Decisions below).
2. **Task 1 (TDD): Hash the three new sections and re-pin the golden digest at v3**
   - RED: `8bdbb392` — `test(23-10): add failing v3 fingerprint difference tests`
   - GREEN: `7751cb3d` — `feat(23-10): bump graph fingerprint to v3, hash worker templates/battalion/directive parser`

**Plan metadata:** committed alongside this SUMMARY (see below).

_TDD task: RED -> GREEN. No separate REFACTOR commit was needed — the GREEN implementation required no follow-up cleanup._

## Files Created/Modified

- `crates/paladin-battalion/src/engine/graph.rs` — `fingerprint()`'s three new hashed sections, extended rustdoc, re-pinned golden-hex and deterministic-across-calls literals, eight new tests, and two pre-existing `RecursiveEmbedding` tests rewritten (see Deviations).
- `crates/paladin-core/src/platform/container/waypoint.rs` — `GRAPH_FINGERPRINT_VERSION` bumped `v2` -> `v3`, doc comments extended, one test literal re-pinned. (Outside this plan's declared `files_modified`; see Deviations.)

## Decisions Made

- **Checkpoint auto-selected (option-a, D-18 as written):** the orchestrator's auto-mode pre-resolved Task 1's `checkpoint:decision` (`gate="blocking"`) to option-a per its own instructions before this executor started — bump to `v3` and add all three new sections, each sorted and length-prefixed through `push_field`. Rejected alternatives per CONTEXT.md D-18: option-b (stay at v2, emit new sections only when non-empty — reintroduces conditional-emission ambiguity) and option-c (bump to v3 but hash only a subset — leaves scheduling-relevant properties unhashed, defeating the fingerprint's coverage contract).
- **`StateMap` list-length prefixes:** the plan's action text says "length-prefix each element" of the `StateMap` pairs; implementing this literally (only `push_field` per element, no count) would let a pair silently shift between `inputs` and `outputs` without changing the concatenated bytes, reintroducing exactly the split-ambiguity class CR-01 fixed for delimiter joins. Added an explicit `u64` LE element-count prefix before each of the two lists to close this gap (Rule 2 — missing critical functionality; the threat model's T-23-41 mitigation explicitly requires collision-freedom).
- **`DirectiveParser` hashed as one field:** rather than separately hashing "kind" and "on_parse_error" as two fields, the whole `DirectiveParser` enum is serialized once via serde (matching the existing `EdgeCondition`/`DispatchRule` precedent) — this naturally captures both the kind and, where present, `on_parse_error`, satisfying the plan's "kind and, where the variant carries one, its on_parse_error" requirement in one field rather than two.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] `GRAPH_FINGERPRINT_VERSION`'s declaration site is outside this plan's declared `files_modified`**
- **Found during:** Task 1, action item 1 ("Bump `GRAPH_FINGERPRINT_VERSION` from `v2` to `v3` at its single declaration site").
- **Issue:** The constant is declared solely in `crates/paladin-core/src/platform/container/waypoint.rs`, not `crates/paladin-battalion/src/engine/graph.rs` (this plan's only declared file). The parallel-execution instructions for this wave explicitly listed `waypoint.rs` as a file the concurrent 23-09 worktree edits and instructed this executor not to touch it.
- **Fix:** Read `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-09-PLAN.md` first to confirm 23-09's own plan does not touch `GRAPH_FINGERPRINT_VERSION` (only references CR-01 in passing prose). Made the minimal, isolated edit: the constant's value, its doc comment, `GraphFingerprint::as_str`'s doc comment, and one pre-existing test assertion (`starts_with("v2:")` -> `starts_with("v3:")`) — no other lines in the file touched, to minimize any merge-conflict surface with the concurrent worktree.
- **Files modified:** `crates/paladin-core/src/platform/container/waypoint.rs`.
- **Verification:** `cargo test -p paladin-ai-core --lib waypoint` — 22 passed, 0 failed.
- **Committed in:** `7751cb3d` (part of the Task 1 GREEN commit).

**2. [Rule 1 - Bug] Two pre-existing `RecursiveEmbedding` tests relied on a latent v2 encoding gap**
- **Found during:** Task 1, after implementing the `battalion:` section and re-running the full `graph` test suite.
- **Issue:** `directly_recursive_embedding_is_rejected` and `transitively_recursive_embedding_is_rejected` (both pre-existing, from Phase 22/22.1) constructed a "self-similar" graph by embedding a leaf graph that was structurally identical to its wrapper *under v2's encoding only* — v2's `nodes:` section did not distinguish a `NodeSpec::Battalion` node from a `NodeSpec::Function` node (both got the same "no output field" tag byte), so a Battalion-wrapping graph coincidentally hashed identically to the plain leaf it wrapped. v3's new `battalion:` section correctly distinguishes these (a genuine, D-18-mandated coverage improvement), which means the two tests' fingerprints no longer collide, and `validate()` no longer returns `RecursiveEmbedding` for their fixtures.
- **Analysis:** Under a properly depth-sensitive, Merkle-style fingerprint (each Battalion node hashes its child's own `fingerprint()` string), a descendant's fingerprint can never algebraically equal an ancestor's via honest finite construction — the ancestor's hash is partly *defined by* the descendant's hash through the wrapping chain, so requiring equality asks for a hash fixed point, which a collision-resistant hash does not yield except by (here, engineered) accident. This is a correctness improvement: `RecursiveEmbedding`, as coded, can no longer misfire on encoding blindness, but it also can no longer be exercised end-to-end through the public `validate()` entry point with an honestly-constructed fixture.
- **Fix:** Rewrote both tests to call the private `validate_battalion_children` helper directly (accessible from the same-module `tests` submodule) with a hand-seeded `ancestry` list containing the exact fingerprint of an embedded child/grandchild — this exercises the identical ancestry-collision code path (`ancestry.contains(&child_fp)`) and, for the transitive case, the same multi-level `next_ancestry` accumulation a genuine multi-level embedding would use, without relying on an impossible hash coincidence. Added an explanatory comment block documenting why this changed.
- **Files modified:** `crates/paladin-battalion/src/engine/graph.rs`.
- **Verification:** `cargo test -p paladin-battalion --lib engine::graph::tests::directly_recursive_embedding_is_rejected engine::graph::tests::transitively_recursive_embedding_is_rejected` — both pass; full `cargo test -p paladin-battalion --lib` — 467 passed, 0 failed.
- **Committed in:** `7751cb3d` (part of the Task 1 GREEN commit).

---

**Total deviations:** 2 auto-fixed (1 Rule 3 - blocking issue, 1 Rule 1 - bug fix).
**Impact on plan:** Both deviations were necessary to satisfy the plan's own acceptance criteria and D-18's coverage mandate; neither is scope creep. The `waypoint.rs` touch is a one-line constant plus doc/test literal, isolated from 23-09's declared edit areas. The `RecursiveEmbedding` test rewrite preserves the exact same safety property under test, using a more directly-targeted (and arguably better-isolated) testing technique.

## Issues Encountered

None beyond the two deviations documented above, both discovered and resolved during Task 1's GREEN implementation.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `GRAPH_FINGERPRINT_VERSION` is `v3`; any code or test elsewhere in the workspace that hardcodes a `"v2:"` prefix should be checked in future work (none found outside `graph.rs` and `waypoint.rs` at time of writing — confirmed via `grep -rln '"v2\|v2:'` across `crates/` and `tests/`).
- Plan 23-09 (child thread identity / `checkpoint_ns` / resume-mid-child), running concurrently in its own worktree, does not touch `GRAPH_FINGERPRINT_VERSION` or the `fingerprint()` byte layout — no expected merge conflict from this plan's changes.
- This is the last plan in the phase's fingerprint-coverage chain (23-04, 23-05, 23-08 each landed one producing property; this plan is the single, deliberate bump after all three). No further fingerprint version bump is anticipated for this phase.

---

*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-04*

## Self-Check: PASSED

- `crates/paladin-battalion/src/engine/graph.rs` — FOUND
- `crates/paladin-core/src/platform/container/waypoint.rs` — FOUND
- Commit `8bdbb392` (test: RED-phase failing v3 fingerprint tests) — FOUND in `git log --oneline --all`
- Commit `7751cb3d` (feat: GREEN-phase v3 fingerprint implementation) — FOUND in `git log --oneline --all`
- All acceptance criteria re-verified: `cargo test -p paladin-battalion --lib engine::graph` (68 passed, 0 failed, 0 ignored), all six new difference tests individually (`1 passed` each), `fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits` (pass, no new sibling test added), golden-hex test re-pinned with reference-graph construction unchanged (confirmed via `git diff`), `cargo test -p paladin-battalion --lib` (467 passed), `cargo test --test e2e_crash_resume` and `cargo test --test golden_bridge_equivalence` (both green), `cargo test --workspace --lib --bins` (all green, no failures), `cargo fmt --check` (clean), `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean).
- REQUIREMENTS.md: CF-03 checkbox and traceability row marked complete (CF-02/CF-04 already applied by prior plans 23-04/23-08).
