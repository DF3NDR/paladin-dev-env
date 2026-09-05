---
phase: 25-node-level-fault-tolerance
plan: 03
subsystem: infra
tags: [aegis, fault-tolerance, validation, fingerprint, superstep-engine, retry, error-handling]

# Dependency graph
requires:
  - phase: 25-01
    provides: "Aegis/RetryPolicy/RetryPredicate/TimeoutPolicy/ErrorHandlerSpec/CachePolicy value types, the WarGraph aegis/default_aegis sidecar (set_aegis/with_default_aegis/aegis_for), and the retry loop wrapping superstep.rs's per-node dispatch closure"
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "edge_evaluator.rs's EdgeConditionEvaluator/EdgeEvaluatorRegistry fail-closed pattern this plan replicates twice, and WarGraph::validate_battalion_children's recursive registry-inheritance walk"
provides:
  - "paladin_battalion::retry_predicate::{RetryPredicateEvaluator, RetryPredicateError, RetryPredicateRegistry}"
  - "paladin_battalion::error_handler::{ErrorHandler, ErrorHandlerRegistry}"
  - "paladin_battalion::engine::registries::EngineRegistries (edge_evaluators + retry_predicates + error_handlers bundle)"
  - "paladin_battalion::engine::WarEngine::{with_retry_predicate, with_error_handler}"
  - "WarGraph::validate's new EngineRegistries-typed signature, threaded through validate_non_recursive/validate_battalion_children"
  - "WarGraph::validate's Aegis clauses: AegisOnUndeclaredNode, AegisUnsupportedForNodeKind (D-12 node-kind matrix), RetryPolicyInvalid, TimeoutPolicyInvalid, UnregisteredRetryPredicate, UnregisteredErrorHandler"
  - "GRAPH_FINGERPRINT_VERSION v5: on_error/cache hashed (per-node set_aegis sidecar entry + a separate default_aegis sub-section), retry/timeout excluded"
affects: [25-04-node-level-caching, 25-07-waypoint-attempt-history, 25-09-timeouts, 25-10-error-handlers, 25-11-error-handlers-continued, 25-14-migration-doc]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "RetryPredicateRegistry/ErrorHandlerRegistry are byte-for-byte structural mirrors of edge_evaluator.rs's EdgeEvaluatorRegistry -- same HashMap<String, Arc<dyn _>> inner field, same register/get/contains/registered_names shape, same 'replace not error' duplicate-registration policy, same exact-byte-equality name lookup"
    - "EngineRegistries bundles all three registries as public fields so WarGraph::validate/WarEngine hold and pass ONE type instead of three positional parameters; a WarEngine's registries field replaces its old bare edge_evaluators field"
    - "Aegis fingerprint hashing reads the raw per-node set_aegis sidecar (self.aegis), never aegis_for's resolved value -- deliberately, so a graph using default_aegis is distinguishable from one setting the identical policy on every node individually via set_aegis (the two would otherwise hash identically)"
    - "Validation-time Custom-name resolution (UnregisteredRetryPredicate/UnregisteredErrorHandler) reads the RESOLVED aegis_for value instead -- the opposite read from the fingerprint section, and deliberately so: validation cares what will actually run for a node, the fingerprint cares what was explicitly authored"

key-files:
  created:
    - crates/paladin-battalion/src/retry_predicate.rs
    - crates/paladin-battalion/src/error_handler.rs
    - crates/paladin-battalion/src/engine/registries.rs
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/bridges.rs
    - crates/paladin-battalion/src/lib.rs
    - crates/paladin-core/src/platform/container/waypoint.rs
    - tests/integration/subgraph_formation_in_campaign_test.rs

key-decisions:
  - "Task 1 checkpoint auto-resolved (auto mode, gate=blocking not blocking-human): proceed-as-locked -- adopts D-12's node-kind support matrix (Paladin/Function get the full Aegis; Battalion gets timeout/on_error only; Gate gets none) and D-11's fingerprint split (on_error/cache hashed, retry/timeout excluded, v4 -> v5) exactly as CONTEXT.md locked them"
  - "WarGraph::fingerprint()'s new ;aegis: section hashes each node's OWN set_aegis sidecar entry, never aegis_for's resolved value, with default_aegis hashed separately as its own sub-section -- the only way a graph setting a default policy stays distinguishable from one setting the identical policy on every node individually (both would collide under the resolved read)"
  - "RetryPolicyInvalid/TimeoutPolicyInvalid return the FIRST violation per node rather than aggregating every offender, following validate_gates' existing precedent in this same file for 'several distinct rules that don't collapse into one Vec<String>' -- the plan's 'list every offender' instruction is read as applying to the Custom-name and node-kind-matrix clauses, which do aggregate"
  - "validation_lists_every_unregistered_name_not_just_the_first uses THREE nodes (two own-aegis overrides plus one resolving through default_aegis) rather than literally two -- a single Aegis carries exactly one RetryPredicate::Custom value, so three DISTINCT names in one category is structurally unreachable from only two nodes; three is the minimal shape that proves the aggregation claim"

requirements-completed: [FT-02, FT-04]

coverage:
  - id: D1
    description: "RetryPredicate::Custom(name) and ErrorHandlerSpec::Custom(name) resolve through registries registered on WarEngine (with_retry_predicate, with_error_handler); an unregistered name fails graph validation before any node executes, listing every offender in one error"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::unregistered_custom_retry_predicate_fails_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::unregistered_custom_error_handler_fails_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::validation_lists_every_unregistered_name_not_just_the_first"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::registered_custom_names_validate_cleanly"
        status: pass
    human_judgment: false
  - id: D2
    description: "The three registries travel as one EngineRegistries { edge_evaluators, retry_predicates, error_handlers } bundle passed to WarGraph::validate, replacing the old two-registry positional signature"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/registries.rs#tests::default_bundle_has_empty_registries"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/registries.rs#tests::clone_is_independent_arc_backed_snapshot"
        status: pass
    human_judgment: false
  - id: D3
    description: "ErrorHandler::handle is async, Send + Sync, taking (&NodeError, &Battlefield) and returning Result<Directive, NodeError> -- the same async trait-object shape EdgeConditionEvaluator already uses"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/error_handler.rs#tests::name_lookup_is_exact_byte_equality_case_sensitive"
        status: pass
    human_judgment: false
  - id: D4
    description: "Paladin and Function nodes accept the full Aegis; a Battalion node accepts timeout/on_error but rejects retry/cache; a Gate node rejects any Aegis"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::battalion_node_rejects_retry_and_cache_but_accepts_timeout_and_on_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::gate_node_rejects_any_aegis"
        status: pass
    human_judgment: false
  - id: D5
    description: "set_aegis on an undeclared node is a typed validation error listing all offenders"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::aegis_on_undeclared_node_is_a_validation_error"
        status: pass
    human_judgment: false
  - id: D6
    description: "on_error and cache enter WarGraph::fingerprint() sorted by node id and length-prefixed; retry and timeout are excluded, so tightening a retry policy or a timeout never invalidates a stored Waypoint's fingerprint comparison on resume"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::changing_on_error_changes_the_fingerprint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::changing_cache_policy_changes_the_fingerprint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::tuning_retry_does_not_change_the_fingerprint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::tuning_timeout_does_not_change_the_fingerprint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::aegis_hashing_is_sorted_by_node_id"
        status: pass
      - kind: integration
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::resume_still_matches_after_a_retry_tuning_edit"
        status: pass
    human_judgment: false
  - id: D7
    description: "GRAPH_FINGERPRINT_VERSION moves from v4 to v5 in the same commit as the hash-input change; the golden hex test and the version test are re-pinned together so a v4-tagged stored fingerprint is recognised as stale"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::fingerprint_golden_hex_v5"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::fingerprint_version_is_v5"
        status: pass
    human_judgment: false
  - id: D8
    description: "A child Battalion inherits the parent's registries wholesale, matching Phase 23 D-21's inherit-the-engine rule"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::child_battalion_inherits_parent_registries"
        status: pass
    human_judgment: false
  - id: D9
    description: "An Aegis whose RetryPolicy has max_attempts == 0, or whose TimeoutPolicy has Some(Duration::ZERO) on either field, is a typed validation error before execution; a TimeoutPolicy with both fields None is a valid no-op"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::retry_policy_with_zero_attempts_fails_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::timeout_policy_with_zero_duration_fails_validation"
        status: pass
    human_judgment: false

duration: ~110min (two session interruptions -- a pre-first-commit crash and a mid-plan disk-full crash -- both recovered from without losing committed work)
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 03: EngineRegistries, the Aegis Validation Matrix, and Fingerprint v5 Summary

**Two new fail-closed registries (`RetryPredicateEvaluator`/`RetryPredicateRegistry`, `ErrorHandler`/`ErrorHandlerRegistry`) bundled with the existing edge-evaluator registry into one `EngineRegistries` type threaded through `WarGraph::validate`'s new signature, six new typed `EngineError` variants enforcing the D-12 node-kind support matrix and D-09's zero-value policy rejection, and `GRAPH_FINGERPRINT_VERSION` bumped `v4` -> `v5` to hash `on_error`/`cache` while permanently excluding `retry`/`timeout` from ever invalidating a resumed thread.**

## Performance

- **Duration:** ~110 min across two interrupted sessions (a pre-first-commit crash recovered per the resume brief, then a mid-plan disk-full crash recovered per the coordinator's second interrupt message; no committed work was lost either time)
- **Tasks:** 3 (1 auto-resolved checkpoint, 2 auto)
- **Files modified:** 10 (3 created, 7 modified)

## Accomplishments

- `crates/paladin-battalion/src/retry_predicate.rs` and `crates/paladin-battalion/src/error_handler.rs` land as byte-for-byte structural mirrors of `edge_evaluator.rs`'s fail-closed registry pattern (same registry shape, same exact-byte-equality lookup, same "replace not error" duplicate policy), each with their own trait (`RetryPredicateEvaluator::allows`, `ErrorHandler::handle`) and registry type.
- `crates/paladin-battalion/src/engine/registries.rs` bundles all three registries into one `EngineRegistries { edge_evaluators, retry_predicates, error_handlers }` — the single type `WarGraph::validate`, `validate_non_recursive` and `validate_battalion_children` now carry instead of a bare `&EdgeEvaluatorRegistry`, and the single field `WarEngine` now holds instead of its old bare `edge_evaluators` field. `WarEngine::with_retry_predicate`/`with_error_handler` join `with_edge_evaluator` as the three registration methods.
- `WarGraph::validate` gained four new clauses, ordered shallowest-structural-error-first: `AegisOnUndeclaredNode` (a `set_aegis` id that isn't a declared node), `AegisUnsupportedForNodeKind` (D-12's matrix — Battalion rejects `retry`/`cache`, Gate rejects any `Aegis`), `RetryPolicyInvalid`/`TimeoutPolicyInvalid` (zero-value self-validation, first-violation reporting like the existing `validate_gates`), and `UnregisteredRetryPredicate`/`UnregisteredErrorHandler` (D-13's fail-closed Custom-name resolution, each aggregating every offender in one error like the existing `UnregisteredEdgeCondition`).
- `WarGraph::fingerprint()` gained a `;aegis:` section (each node's own `set_aegis` sidecar entry's `on_error`/`cache`, sorted by node id) and a separate `;default_aegis:` sub-section for the graph-wide fallback — reading the RAW sidecar rather than the resolved `aegis_for` value is what keeps a graph using `default_aegis` distinguishable from one setting the identical policy on every node individually. `retry`/`timeout` contribute zero bytes. `GRAPH_FINGERPRINT_VERSION` moved `v4` → `v5` in the same commit; both the golden-hex and version tests were renamed and re-pinned against the real computed output (read from failing-test output, never hand-computed).
- 17 new tests across `retry_predicate.rs`, `error_handler.rs`, `registries.rs` and `engine/graph.rs` prove: unregistered-name rejection (single, multi-offender, and clean-when-registered); undeclared-node rejection; the full node-kind matrix; zero-value policy rejection with the `None`/`None` no-op accepted; child-Battalion registry inheritance; `on_error`/`cache` changing the fingerprint while `retry`/`timeout` tuning (every field of each, one at a time) never does; aegis-section order-independence; and a real `WarEngine::start`→`resume` round trip proving a retry-tuning edit never trips `GraphMismatch`.

## Task Commits

1. **Task 1: Confirm the graph contract — node-kind matrix and fingerprint hashing split** — auto-resolved (`proceed-as-locked`, per checkpoint pre-resolution; gate was `blocking`, not `blocking-human`). No commit (decision only).
2. **Task 2 (auto): EngineRegistries, the two fail-closed registries, and the Aegis validation matrix**:
   - `bd17b76a` (feat) — `retry_predicate.rs`, `error_handler.rs`, `engine/registries.rs`, `WarGraph::validate`'s `EngineRegistries` signature and its four new Aegis clauses, `WarEngine::with_retry_predicate`/`with_error_handler`, and every mechanical call-site update the signature change forced (test code in `graph.rs`/`superstep.rs`/`bridges.rs`, and the root crate's `tests/integration/subgraph_formation_in_campaign_test.rs`)
3. **Task 3 (auto): Fingerprint v5 — hash on_error and cache, exclude retry and timeout**:
   - `2f9223e0` (feat) — `WarGraph::fingerprint()`'s new `;aegis:`/`;default_aegis:` sections, `GRAPH_FINGERPRINT_VERSION` `v4` → `v5`, the renamed and re-pinned golden/version tests, the two mechanically-broken `"v4:..."` literal re-pins, and the 8 new fingerprint-behavior tests

**Plan metadata:** (this commit, `docs(25-03): ...`)

## Files Created/Modified

- `crates/paladin-battalion/src/retry_predicate.rs` (new) — `RetryPredicateEvaluator`, `RetryPredicateError`, `RetryPredicateRegistry`
- `crates/paladin-battalion/src/error_handler.rs` (new) — `ErrorHandler`, `ErrorHandlerRegistry`
- `crates/paladin-battalion/src/engine/registries.rs` (new) — `EngineRegistries` bundle
- `crates/paladin-battalion/src/engine/graph.rs` — `validate`/`validate_non_recursive`/`validate_battalion_children`'s `EngineRegistries` signature, the four new Aegis validation clauses, the `;aegis:`/`;default_aegis:` fingerprint sections, `push_aegis_hashed_fields`, 18 new tests (10 Task 2 + 8 Task 3), renamed/re-pinned golden and version fingerprint tests
- `crates/paladin-battalion/src/engine/mod.rs` — `EngineError`'s 6 new variants, `WarEngine`'s `registries: EngineRegistries` field (replacing `edge_evaluators`), `with_retry_predicate`/`with_error_handler`, all 4 `graph.validate(...)` call sites and 4 `superstep::run(...)` edge-evaluator arguments updated
- `crates/paladin-battalion/src/engine/superstep.rs` — 5 test-only `.validate(...)` call sites updated to `&EngineRegistries::default()`
- `crates/paladin-battalion/src/engine/bridges.rs` — 3 test-only `.validate(...)` call sites updated
- `crates/paladin-battalion/src/lib.rs` — `pub mod error_handler; pub mod retry_predicate;` plus their re-exports
- `crates/paladin-core/src/platform/container/waypoint.rs` — `GRAPH_FINGERPRINT_VERSION` `v4` → `v5`, rustdoc for the `v5` bump, one mechanically-broken `"v4:"` literal re-pinned
- `tests/integration/subgraph_formation_in_campaign_test.rs` — 2 `.validate(...)` call sites updated to `EngineRegistries::default()` (mechanical call-site fix the signature change forced, in scope per the plan's own carve-out)

## Decisions Made

- **Task 1 checkpoint auto-resolved to `proceed-as-locked`** per the orchestrator's pre-resolution (auto mode, gate `blocking` not `blocking-human`): D-12's node-kind matrix and D-11's fingerprint split apply exactly as CONTEXT.md locked them, with no override.
- **Fingerprint hashing reads the raw `set_aegis` sidecar, not the resolved `aegis_for` value** — the only design that keeps a graph using `default_aegis` distinguishable from one setting the identical policy on every node individually via `set_aegis` (both would otherwise produce byte-identical hashes). This is the opposite read from the Custom-name validation clause, which deliberately DOES resolve through `aegis_for` (validation cares what a node will actually run; the fingerprint cares what was explicitly authored).
- **`RetryPolicyInvalid`/`TimeoutPolicyInvalid` report the first violation per node, not every offender aggregated** — following `validate_gates`' own established precedent in this file for "several distinct rules [that] produce distinctly-shaped, differently-typed errors [that] do not collapse into one `Vec<String>`". The plan's "list every offender" instruction is read as applying specifically to the Custom-name and node-kind-matrix clauses, which do aggregate (`UnregisteredRetryPredicate`/`UnregisteredErrorHandler`/`AegisUnsupportedForNodeKind`/`AegisOnUndeclaredNode` all collect every offender).
- **`validation_lists_every_unregistered_name_not_just_the_first` uses three nodes, not the plan prose's literal "two"** — a single `Aegis` carries exactly one `RetryPredicate::Custom` value, so three DISTINCT unregistered names within one category is structurally unreachable from only two nodes' resolved policies; three (two own overrides plus one via `default_aegis`) is the minimal shape that actually proves "lists every offender, not just the first."

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Two hardcoded `"v4:..."` fingerprint literals broke mechanically from the version bump, neither named in the plan's own Task 3 test list**
- **Found during:** Task 3, first `cargo test -p paladin-battalion --lib` run after the `v5` byte-layout change
- **Issue:** `fingerprint_is_deterministic_across_calls` (`graph.rs`) and `graph_fingerprint_is_deterministic_and_versioned` (`waypoint.rs`) both assert exact `"v4:{hex}"` fingerprint strings unrelated to the golden-hex/version tests the plan names — both fail the moment `GRAPH_FINGERPRINT_VERSION` moves, since the tag is part of every literal.
- **Fix:** Re-pinned both to their actual `v5` output, read from the failing-test assertion output (never hand-computed), matching the plan's own discipline for the two named golden tests.
- **Files modified:** `crates/paladin-battalion/src/engine/graph.rs`, `crates/paladin-core/src/platform/container/waypoint.rs`
- **Committed in:** `2f9223e0` (Task 3 commit)

**2. [Rule 3 - Blocking] A residual `fingerprint_golden_hex_v4` substring survived in a rustdoc comment after the test itself was renamed, which would have broken the acceptance criteria's literal grep check**
- **Found during:** Task 3, self-check pass over the acceptance criteria's `! grep -qE 'fingerprint_golden_hex_v4|fingerprint_version_is_v4' ...` command
- **Issue:** The golden test's own historical rustdoc ("Renamed from ... to `fingerprint_golden_hex_v4`") still named the old identifier literally even after the `#[test] fn` itself was renamed to `fingerprint_golden_hex_v5`.
- **Fix:** Reworded the historical note to describe the naming convention without repeating the literal old identifier.
- **Files modified:** `crates/paladin-battalion/src/engine/graph.rs`
- **Committed in:** `2f9223e0` (Task 3 commit)

**3. [Rule 3 - Blocking] `cargo fmt --check` found a stray formatting diff in `tests/integration/subgraph_formation_in_campaign_test.rs` after Task 2's mechanical call-site fix**
- **Found during:** the final workspace-wide `cargo fmt --check` pass
- **Issue:** `EngineRegistries::default()` is shorter than the `EdgeEvaluatorRegistry::new()` it replaced, so rustfmt's line-width rule now collapses two previously-multi-line `.validate(...)` calls to one line each.
- **Fix:** Ran `rustfmt` on just that file.
- **Files modified:** `tests/integration/subgraph_formation_in_campaign_test.rs`
- **Committed in:** `2f9223e0` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (2 bugs mechanically forced by the version bump, 1 blocking acceptance-criteria/formatting fix). No architectural changes, no scope creep — every fix is a direct, unavoidable mechanical consequence of the plan's own required changes.

## Issues Encountered

- **A pre-first-commit session crash** occurred before this executor's work began (per the resume brief handed to this session); no work was lost since nothing had been committed yet, and this plan was executed fresh from the `25-02`-topped base.
- **A mid-plan disk-full crash** interrupted the session between Task 2's commit and Task 3's commit, caused by `/workspace`'s shared filesystem filling to 100% (independent worktrees building concurrently). Recovered per the coordinator's interrupt message: `git status`/`git log` confirmed Task 2's commit (`bd17b76a`) was intact and Task 3's uncommitted edits (`graph.rs`, `waypoint.rs`, the integration test) survived on disk unchanged. All Task 2/3 tests were re-run from scratch after the interrupt to confirm nothing was silently corrupted before committing Task 3. No work was lost either time.
- Workspace disk pressure recurred once more during this session's own full `cargo test --workspace --lib` verification pass (unrelated crates failed to compile with `No space left on device`); resolved by clearing this worktree's own regenerable `target/debug/incremental` cache (5.7 GB → freed), which is safe to remove and owned entirely by this worktree — no shared or sibling-owned files were touched.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Every `Custom` name, every aegis annotation and every unsupported node-kind/policy pairing is now rejected by `WarGraph::validate` before any node executes — plans 25-09 (timeouts), 25-10/11 (error handlers) and 25-04/13 (caching) can now assume that check already exists rather than building it themselves.
- `GRAPH_FINGERPRINT_VERSION` is `v5`; `on_error`/`cache` are part of graph identity, `retry`/`timeout` are not — later plans adding a new `Aegis` field know which side of that rule it belongs on from `fingerprint()`'s own rustdoc, without re-deriving the rationale.
- Handler DISPATCH (actually invoking a resolved `ErrorHandler`/`RetryPredicateEvaluator` at run time) is explicitly NOT built here, as scoped — `error_handler.rs`'s own module doc states plainly that plan 25-10/11 owns dispatch; this plan owns registration and validation only. Nothing here needs reshaping once dispatch lands: the trait signatures (`ErrorHandler::handle(&NodeError, &Battlefield) -> Result<Directive, NodeError>`; `RetryPredicateEvaluator::allows(&NodeError, attempt: u32) -> Result<bool, RetryPredicateError>`) are already in their PRD-final shape.
- No blockers for the next wave.

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-05*
