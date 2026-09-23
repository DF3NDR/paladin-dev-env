---
phase: 28-observability-tooling
plan: 08
subsystem: testing
tags: [paladin-eval, assertions, insta, snapshot-testing, obs-fr-12]

# Dependency graph
requires:
  - phase: 28-03
    provides: "EdgeEvaluated, ParleyRaised, RunFinished (with total_tokens/total_supersteps/status), NodeStarted/NodeFinished (with attempt) -- the trace record shapes this plan's evaluators read"
  - phase: 28-05
    provides: "The paladin-eval crate skeleton, the Assertion enum (PRD 07 OBS-FR-12 verbatim names) and Times in scenario.rs, and the insta workspace precedent used by tests/cli/*.rs"
provides:
  - "crates/paladin-eval/src/assertion.rs: AssertionContext<'a> built from exactly three inputs (captured TraceRecord stream, final Battlefield, RunOutcome), AssertionOutcome/AssertionFailure with render_failure() producing the four-part actionable message, CustomAssertion (Rust-API-only, no serde derive), and pub fn evaluate dispatching the closed twelve-variant Assertion enum with no wildcard arm"
  - "Twelve OBS-FR-12 assertion evaluators: final_state_field_equals, final_state_field_matches, field_json_path_equals, node_executed, node_not_executed, edge_fired, route_taken, run_status, total_tokens_max, supersteps_max, parley_raised, final_state_snapshot"
  - "crates/paladin-eval/tests/assertion_snapshots.rs and tests/snapshots/: insta snapshots freezing every assertion kind's failure-rendering wording against one fixed branching/retrying/parley-raising fixture"
affects: ["28-12 (the libtest-mimic runner will call evaluate() per case assertion and report AssertionFailure::render_failure() on failure)", "28-16 (E2E dogfood scenarios exercise these assertions end to end)"]

# Tech tracking
tech-stack:
  added: ["insta 1.34 as a paladin-eval dev-dependency (non-workspace-table pin, mirroring the root facade's own [dev-dependencies] insta pin)"]
  patterns:
    - "AssertionContext is the only handle any evaluator receives (D-29): records/battlefield/outcome plus derived per-node NodeStarted views and the route sequence, all borrowed, never cloned"
    - "Every failure carries the same four-part shape (assertion, expected, observed, seq_range, detail) rendered by one render_failure() method, frozen per-kind by insta"
    - "evaluate() dispatches the closed Assertion enum with no wildcard arm -- a future assertion kind is a compile error, never a silent no-op (T-28-08-04)"

key-files:
  created:
    - crates/paladin-eval/src/assertion.rs
    - crates/paladin-eval/tests/assertion_snapshots.rs
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__edge_fired.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__field_json_path_equals.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__final_state_field_equals.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__final_state_field_matches.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__final_state_snapshot.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__node_executed.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__node_not_executed.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__parley_raised.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__route_taken.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__run_status.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__supersteps_max.snap
    - crates/paladin-eval/tests/snapshots/assertion_snapshots__total_tokens_max.snap
  modified:
    - crates/paladin-eval/src/lib.rs
    - crates/paladin-eval/Cargo.toml
    - Cargo.lock

key-decisions:
  - "final_state_snapshot's blessed-file path is supplied via a builder method (AssertionContext::with_snapshot_path), not the three-argument AssertionContext::new -- the constructor stays exactly the three D-29 inputs, and the snapshot path is orthogonal per-case configuration the future runner (28-12) will set."
  - "final_state_snapshot's failure messages never render the snapshot file's own absolute path (only '--bless' and, on a mismatch, the first differing JSON path) -- this keeps the insta-frozen snapshot for this kind machine-independent without needing an insta filter."
  - "route_taken's failure seq_range spans every NodeStarted record in the run (not just the expected-route nodes), since a subsequence miss is a property of the whole observed route."
  - "parley_kind_wire_name maps ParleyKind to the same snake_case wire vocabulary graph_doc.rs's ParleyKindDoc already documents (approval/choice/free_text/state_edit) -- ParleyKind itself has no #[serde(rename_all)], so this mapping lives in assertion.rs rather than being derived."

patterns-established:
  - "A closed AssertionOutcome (Passed | Failed(AssertionFailure)) with rendering computed only on the Failed arm -- the happy path never allocates a message."

requirements-completed: [OBS-04]

coverage:
  - id: D1
    description: "AssertionContext is built from exactly three inputs (TraceRecord stream, final Battlefield, RunOutcome) and no evaluator reaches into engine internals beyond them"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::assertion_context_uses_only_records_battlefield_and_outcome"
        status: pass
    human_judgment: false
  - id: D2
    description: "node_executed counts NodeStarted records per node (a retried node counts once per attempt) and supports exact/min/max bounds, with a failure rendering a visit table (seq, superstep, attempt, outcome)"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::node_executed_counts_attempts_not_nodes"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::node_executed_min_and_max"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::node_executed_failure_renders_a_visit_table"
        status: pass
      - kind: integration
        ref: "crates/paladin-eval/tests/assertion_snapshots.rs#node_executed_failure_renders_the_visit_table"
        status: pass
    human_judgment: false
  - id: D3
    description: "All twelve OBS-FR-12 assertion kinds are implemented with PRD-verbatim names and evaluate() dispatches the closed enum with no wildcard arm"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests (34 tests: one passing and one failing case per evaluator, plus edge cases)"
        status: pass
      - kind: other
        ref: "grep -c 'pub fn evaluate' crates/paladin-eval/src/assertion.rs (1); the twelve Assertion variant names all present"
        status: pass
    human_judgment: false
  - id: D4
    description: "edge_fired distinguishes evaluated-but-not-fired from never-evaluated; route_taken checks a subsequence not exact order; run_status/total_tokens_max/supersteps_max fail explicitly (never a default) when the trace carries no RunFinished record"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::edge_fired_fails_when_evaluated_but_not_fired"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::edge_fired_fails_when_never_evaluated"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::route_taken_passes"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::run_status_fails_when_run_never_finished"
        status: pass
    human_judgment: false
  - id: D5
    description: "final_state_field_matches compiles its regex through the regex crate (linear-time, no backtracking) and returns a typed failure -- never a panic -- on an invalid pattern (T-28-08-01)"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/assertion.rs#assertion::tests::final_state_field_matches_fails_on_invalid_regex"
        status: pass
    human_judgment: false
  - id: D6
    description: "CustomAssertion has no serde representation and cannot be deserialized from a scenario file (T-28-08-02)"
    requirement: "OBS-04"
    verification:
      - kind: other
        ref: "grep -c 'CustomAssertion' crates/paladin-eval/src/scenario.rs (0)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Every assertion kind's failure rendering is frozen by an insta snapshot against one fixed, branching, retrying, parley-raising fixture; node_executed shows the visit table, edge_fired shows the evaluated-but-not-fired wording, route_taken shows the observed route, final_state_snapshot shows the first differing path; no snapshot embeds an absolute path"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "crates/paladin-eval/tests/assertion_snapshots.rs (12 tests, one per assertion kind)"
        status: pass
      - kind: other
        ref: "grep -rl '/workspace' crates/paladin-eval/tests/snapshots/ | wc -l (0)"
        status: pass
    human_judgment: false
  - id: D8
    description: "Library code (outside #[cfg(test)]) never calls unwrap()/expect()/panic!; rustdoc builds with zero warnings; clippy and fmt pass"
    requirement: "OBS-04"
    verification:
      - kind: other
        ref: "grep count of unwrap()/expect(/panic! outside #[cfg(test)] in assertion.rs (0); cargo clippy -p paladin-eval --all-targets --all-features -- -D warnings (0 warnings); cargo doc -p paladin-eval --no-deps (0 warnings); cargo fmt --all --check (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: ~28min
completed: 2026-09-09
status: complete
---

# Phase 28: Observability & Tooling — Plan 08 Summary

**The `paladin-eval` assertion library: twelve OBS-FR-12 evaluators over the captured trace record stream and final `Battlefield`, each with an actionable, `insta`-frozen failure rendering, proving the twelve-variant `TraceEvent` is sufficient for the harness's own acceptance bar.**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-09-09T00:58:49Z (worktree base)
- **Completed:** 2026-09-09T01:21:30Z (last commit)
- **Tasks:** 3/3
- **Files modified:** 17 (14 created, 3 modified)

## Accomplishments

- `AssertionContext<'a>` (`crates/paladin-eval/src/assertion.rs`) built from exactly the three D-29 inputs — the captured `TraceRecord` stream, the final `Battlefield`, and the `RunOutcome` — with derived per-node `NodeStarted` views and the executed-route sequence, borrowing rather than cloning the record stream.
- `AssertionOutcome` / `AssertionFailure` with `render_failure()` producing the four-part actionable message (assertion, expected, observed, `seq` range, detail); a passing evaluation carries no rendered text.
- `CustomAssertion`: a Rust-API-only closure type with no `serde` derive and no variant in `scenario.rs`'s `Assertion` enum — it can never be deserialized from a scenario file (T-28-08-02).
- All twelve OBS-FR-12 assertion kinds implemented and dispatched by `pub fn evaluate` over the closed `Assertion` enum with no wildcard arm (T-28-08-04): `final_state_field_equals`, `final_state_field_matches`, `field_json_path_equals`, `node_executed`, `node_not_executed`, `edge_fired`, `route_taken`, `run_status`, `total_tokens_max`, `supersteps_max`, `parley_raised`, `final_state_snapshot`.
- `edge_fired` distinguishes "evaluated but did not fire" from "never evaluated"; `route_taken` checks a subsequence, not exact order; `run_status`/`total_tokens_max`/`supersteps_max` fail explicitly, never with a default, when the trace has no `RunFinished` record; `final_state_field_matches` compiles its regex through the `regex` crate (linear-time, no backtracking) and returns a typed failure rather than panicking on an invalid pattern (T-28-08-01); `final_state_snapshot` fails naming `--bless` when no snapshot file is configured or none exists yet, and reports the first differing JSON path on a mismatch, without ever rendering an absolute path.
- `crates/paladin-eval/tests/assertion_snapshots.rs`: twelve `insta` snapshots (one per assertion kind) freezing `render_failure()`'s exact wording against one fixed, branching, retrying, parley-raising `TraceRecord` fixture — `node_executed`'s snapshot shows the visit table, `edge_fired`'s shows the evaluated-but-not-fired wording, `route_taken`'s shows the observed route, `final_state_snapshot`'s shows the first differing path.

## Task Commits

1. **Task 1 (tracer): AssertionContext, AssertionOutcome/Failure, and `node_executed`** — `d2d72bb6` (feat)
2. **Task 2: the remaining eleven assertion kinds and `evaluate()` dispatch** — `0eb41333` (feat)
3. **Task 3: freeze every failure rendering with `insta` snapshots** — `82ed3010` (test)

**Tracer feedback gate:** Task 1's `<verify>` (`cargo test -p paladin-eval --lib assertion` → `test result: ok. 7 passed`) was re-run immediately after the Task 1 commit and passed before Task 2 began.

## Files Created/Modified

- `crates/paladin-eval/src/assertion.rs` — `AssertionContext`, `AssertionOutcome`, `AssertionFailure`, `render_failure`, `CustomAssertion`, `evaluate`, and the twelve evaluators, plus 34 unit tests
- `crates/paladin-eval/tests/assertion_snapshots.rs` — the fixed run fixture and 12 snapshot tests
- `crates/paladin-eval/tests/snapshots/*.snap` — the 12 committed blessed snapshots
- `crates/paladin-eval/src/lib.rs` — `pub mod assertion;` and re-exports (`AssertionContext`, `AssertionFailure`, `AssertionOutcome`, `CustomAssertion`, `evaluate`)
- `crates/paladin-eval/Cargo.toml` — `insta = "1.34"` added to `[dev-dependencies]` (non-workspace-table pin, mirroring the root manifest's own `[dev-dependencies]` pin)
- `Cargo.lock` — `insta` and its transitive dependencies resolved for `paladin-eval`

## Decisions Made

See `key-decisions` in the frontmatter. Most consequential: `final_state_snapshot`'s blessed-file path is supplied via a builder method (`AssertionContext::with_snapshot_path`) rather than a fourth constructor argument, keeping `AssertionContext::new`'s signature exactly the three D-29 inputs; and its failure messages never render the snapshot file's own absolute path, keeping the frozen snapshot machine-independent without an `insta` filter.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Two `if let` chains collapsed per clippy's `collapsible_if` lint**
- **Found during:** Task 2, first `cargo clippy -p paladin-eval --all-targets --all-features -- -D warnings` run
- **Issue:** `edge_fired` and `parley_raised` each nested an `if let TraceEvent::... = &r.event { if <condition> { ... } }`, which clippy's `collapsible_if` lint (deny-by-default under `-D warnings`) flags as collapsible into a single `if let ... && ... { }` (Rust 2024 let-chains).
- **Fix:** Rewrote both as `if let TraceEvent::Variant { .. } = &r.event && <condition1> && <condition2> { ... }`.
- **Files modified:** `crates/paladin-eval/src/assertion.rs`
- **Verification:** `cargo clippy -p paladin-eval --all-targets --all-features -- -D warnings` exits clean; all 34 lib tests still pass.
- **Committed in:** `0eb41333` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 lint fix). **Impact on plan:** None on scope — a mechanical clippy-driven cleanup, no behavior change.

## Issues Encountered

None beyond the deviation above. `cargo tree -e features -p paladin-ai | grep -c paladin-eval` remains `0`, confirming this plan added no edge into the facade's dependency graph (X-11.4, unchanged from 28-05).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- 28-12 can wire the `libtest-mimic` runner to call `paladin_eval::evaluate` per case assertion and report `AssertionFailure::render_failure()` on a failing case, plus `--bless` for `final_state_snapshot` (the builder hook, `AssertionContext::with_snapshot_path`, is already in place for it to call).
- 28-16's E2E dogfood scenarios have a complete, tested assertion vocabulary to assert against.
- `crates/paladin-eval/src/assertion.rs`'s module doc records the T-28-08-01..04 threat-mitigation rationale inline for future readers.

## Self-Check: PASSED

- `cargo fmt --all --check` — exit 0
- `cargo check --workspace --all-targets --all-features` — exit 0 (4m14s, cold worktree target/)
- `cargo clippy --all-targets --all-features -p paladin-eval -- -D warnings` — exit 0, 0 warnings
- `cargo doc -p paladin-eval --no-deps` — exit 0, 0 warnings
- `cargo test -p paladin-eval` — 46 lib + 12 snapshot + 1 golden + 4 doc tests passed, 0 failed
- `grep -c 'unwrap()\|expect(\|panic!' crates/paladin-eval/src/assertion.rs` outside `#[cfg(test)]` — `0`
- `cargo tree -e features -p paladin-ai | grep -c paladin-eval` — `0`
- `grep -rl '/workspace' crates/paladin-eval/tests/snapshots/ | wc -l` — `0`
- No `.snap.new` files present after the final `cargo test -p paladin-eval --test assertion_snapshots` run
- `git log --oneline -3` shows `82ed3010`, `0eb41333`, `d2d72bb6` — all three task commits present

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
