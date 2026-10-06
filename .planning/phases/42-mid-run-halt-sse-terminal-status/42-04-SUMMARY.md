---
phase: 42-mid-run-halt-sse-terminal-status
plan: 04
subsystem: treasurer
tags: [allowance, mid-run-halt, spend-guard, fail-closed, fork-resume, tests, docs]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: TreasurerSpendGuard with its memoised first halt, HaltCause/HaltReason, the shared Treasurer::evaluate (plan 42-02); persisted runs.halt_reason and GET /runs halt_reason + final_waypoint_id (plan 42-03); ADR-0057
  - phase: 41-admission-time-allowance-enforcement
    provides: admission through RunSubmissionService::fork, the 429 allowance_exhausted mapping with Retry-After
provides:
  - guard contract tests (exact-ceiling boundary, a read at every boundary, inert without a ceiling, three fail-closed kinds, memoised first halt, shared scope)
  - child_battalion_halt_on_spend_halts_the_parent (a spend halt in a Battalion child halts the parent through the shared guard)
  - unattributed_run_gets_no_guard_and_reads_no_ledger (with an attributed control on the same pool)
  - halted_run_resumes_by_fork_after_window_reset and ledger_unavailable_halt_resumes_after_recovery (end to end through the real run and thread routers over on-disk SQLite)
  - platform-api.md "Resuming a halted run" recipe, MIGRATION.md 9.6 and CHANGELOG entries
  - settlement.rs module doc naming the SpendGuard as the authoritative read
affects: [42-05, 42-06, 42-07, 42-11, 42-12]

tech-stack:
  added: []
  patterns:
    - "Scripted ledger wrapper: a delegating TreasuryLedgerPort that offsets the store clock and fails reads on demand, shared by the Treasurer, the submission service and the pool, so admission and the boundary see one scripted clock"
    - "Resume is a new forked run that re-runs admission; the original halted run stays terminal"
    - "Pure message builder for a security-relevant log line (fail_closed_message) so the content is unit-testable without a process-wide logger"

key-files:
  created: []
  modified:
    - src/application/services/treasurer/guard.rs
    - src/application/services/treasurer/tests.rs
    - src/application/services/run/worker_tests.rs
    - src/application/services/run/http_surface_tests.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/settlement.rs
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "No new public symbol: the boundary contract was already implemented by 42-02, so every behaviour test passed on its first run against it; the only production change is the fail-closed log line gaining the scope kinds (a pub(super) helper, public API surface unchanged)"
  - "The fail-closed log line is asserted through its pure builder (fail_closed_message) rather than a captured log record: the paladin-ai lib test binary already shares one process-wide logger slot with the log_sink tests, which would make a second capturing logger order-dependent"
  - "While reads still fail, a fork's own admission fails closed with 500 (the documented allowance-check failure), asserted in the ledger-outage resume test"

patterns-established:
  - "ResumeRig: one helper builds the merged run and thread routers, the Treasurer-attached pool and the scripted ledger for any halt-and-resume scenario"

requirements-completed: [ALLOW-03]

coverage:
  - id: D1
    description: "The boundary check halts at a balance exactly equal to a ceiling and one nano above it, continues one nano below, reads every applicable ceiling at every boundary without caching a Continue, and never reads the ledger for a principal with no ceiling even when the ledger fails"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/treasurer/tests.rs#guard_halts_at_exactly_the_ceiling_and_continues_one_nano_below, #guard_reads_every_applicable_ceiling_on_every_check_without_caching, #guard_for_a_principal_without_a_ceiling_never_reads_the_ledger_even_when_the_ledger_fails"
        status: pass
    human_judgment: false
  - id: D2
    description: "A balance error, a store-clock error and a currency mismatch each halt with ledger_unavailable (fail closed), the first halt is memoised (no further ledger read after the balance drops and the window rolls, also for a ledger_unavailable halt), and two guards on one scope both halt once the shared balance crosses; the error log line names run, scope kinds, tenant and error and no key value"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "treasurer/tests.rs#guard_fails_closed_with_ledger_unavailable_when_balance_errs, #..._when_the_store_clock_errs, #..._on_a_currency_mismatch, #guard_memoises_its_first_halt, #guard_memoises_a_ledger_unavailable_halt_too, #two_guards_on_one_scope_both_halt_once_the_shared_balance_crosses, #fail_closed_log_line_names_run_scope_tenant_and_error_only (cargo test -p paladin-ai --lib application::services::treasurer: 53 passed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "A Battalion child run that halts on spend at its first boundary through the shared guard contributes an empty delta, persists its own Halted Waypoint with no completed node, and the parent halts at its next boundary with the same reason without running its post-Battalion node, the memoised halt answering without a re-read"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#child_battalion_halt_on_spend_halts_the_parent"
        status: pass
    human_judgment: false
  - id: D4
    description: "A run whose row records no submitter gets no guard and completes with zero ledger reads on a pool with a Treasurer attached, while an attributed run on the same pool reads the ledger"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#unattributed_run_gets_no_guard_and_reads_no_ledger"
        status: pass
    human_judgment: false
  - id: D5
    description: "A spend-halted engine run resumes by fork from its final_waypoint_id: refused 429 allowance_exhausted with an integer Retry-After in 1..=86400 while the 1h window is exhausted, admitted (202) after the store clock passes the window end, the forked run Completed with fork_from recording the Halted Waypoint, n0 not dispatched again and n1 and n2 run exactly once, the original run still halted; a real hour boundary mid-test triggers one re-run on a fresh tenant and key"
    requirement: "ALLOW-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#halted_run_resumes_by_fork_after_window_reset (cargo test -p paladin-ai --lib: 1 passed, with and without --features web-server)"
        status: pass
    human_judgment: false
  - id: D6
    description: "A ledger_unavailable halt (reads failing at the first boundary: halted, error null, halt_reason exactly {reason: ledger_unavailable}, no node ran) is refused by the fork with 500 while reads still fail and resumes by fork once reads recover, the forked run completing"
    requirement: "ALLOW-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#ledger_unavailable_halt_resumes_after_recovery"
        status: pass
    human_judgment: false
  - id: D7
    description: "The resume recipe is documented and registered, and the settlement.rs change is documentation only"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: "platform-api.md contains 'Resuming a halted run', final_waypoint_id, from_waypoint_id, Retry-After; MIGRATION.md 9.6 mentions fork (5 lines); ./scripts/check-migration-allowlist.sh exit 0; PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface unchanged (4195 items); git diff of settlement.rs shows only comment lines"
        status: pass
    human_judgment: false
  - id: D8
    description: "With several runs of one scope in flight at once, total spend past the ceiling is bounded by at most one superstep's spend per run (the accepted 41 D-05 over-admission race)"
    requirement: "ALLOW-03"
    verification: []
    human_judgment: true
    rationale: "Backstop truth by design (D-01, ADR-0057): the overshoot bound cannot be proven exactly by a test; two_guards_on_one_scope_both_halt_once_the_shared_balance_crosses proves each run's own guard halts at its first boundary after exhaustion, and plan 42-12 records the race in WINDOWS.md"

duration: ~25min
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 04: Boundary hardening and resume by fork Summary

**The mid-run spend guard's edge contract is now pinned by tests (exact-ceiling boundary, a read at every boundary, three fail-closed kinds, a sticky first halt shared with child battalion runs, no guard for an unattributed run), and a spend-halted or ledger-unavailable engine run is proven to resume end to end by forking from its Halted Waypoint once admission passes, with the recipe documented.**

## Performance

- **Duration:** ~25 min (two task commits, `169cf78e` and `5a7b993d`, plus reading, registers and this summary)
- **Started:** 2026-10-06T18:42Z
- **Completed:** 2026-10-06T19:00Z (code); summary and state updates follow
- **Tasks:** 2 (both auto/TDD)
- **Files modified:** 9, none created

## Accomplishments

- **Guard contract (Task 1).** Eleven new treasurer tests over the scripted `FakeLedger`: `guard_halts_at_exactly_the_ceiling_and_continues_one_nano_below` (999 continues, 1000 and 1001 halt with the exact figures), the every-boundary read count (three checks, three store-clock reads, six balance reads), no ledger read for a principal with no ceiling even while the ledger fails, one fail-closed test each for a balance error, a store-clock error and a currency mismatch, the memoised first halt (identical reason and zero extra ledger calls after the balance drops and the window rolls, and the same for a `ledger_unavailable` halt after the ledger recovers), and two guards on one scope halting together once the shared balance crosses.
- **Child stickiness.** `child_battalion_halt_on_spend_halts_the_parent` drives a real `NodeSpec::Battalion` through a new `run_with_children_and_guard` helper (`run_with_children` delegates to it, so its 15 call sites are untouched). A test-local guard that mirrors `TreasurerSpendGuard`'s memo continues at the parent's first boundary, halts at the child's first boundary and is sticky afterwards: the child persists its own Halted Waypoint with no completed node, the parent halts with `HaltCause::Spend(LedgerUnavailable)` and never runs its post-Battalion node, and the underlying guard evaluated exactly twice (the parent's second boundary was the memo).
- **Unattributed run.** `unattributed_run_gets_no_guard_and_reads_no_ledger` counts every store-clock and balance read through a delegating ledger: the unattributed run completes with zero reads and an attributed run on the same pool completes with more than zero, so the zero is not vacuous.
- **Resume end to end (Task 2).** A `ScriptedLedger` (inner SQLite ledger, an adjustable store-clock offset, a read-failure switch) is shared by the Treasurer, the submission service and the pool, and a `ResumeRig` merges the real `run_router` and `thread_router` over one auth map. `halted_run_resumes_by_fork_after_window_reset` halts, reads `halted` plus `final_waypoint_id`, sees the fork refused `429 allowance_exhausted` with an integer `Retry-After`, moves the clock past the halted window's end, sees the same fork answered `202`, runs it to `Completed` with `fork_from` recording the Halted Waypoint, and asserts the node counters `[1, 1, 1]` (n0 not re-dispatched) and the original run still `halted`. `ledger_unavailable_halt_resumes_after_recovery` halts at the first boundary with exactly `{"reason":"ledger_unavailable"}` and a null error, sees the fork refused `500` (fail closed) while reads still fail, then admits and completes it after recovery.
- **Docs.** `platform-api.md` gains "Resuming a halted run" (and the fork route row names its `429`), `MIGRATION.md` 9.6 and the CHANGELOG Phase 42 entry carry the two-sentence recipe, and the `settlement.rs` module doc now says the check-only `SpendGuard` is the authoritative read and `settle_boundary` is unchanged.

## Task Commits

1. **Task 1: prove the spend guard's boundary, fail-closed and sticky contract** - `169cf78e` (test)
2. **Task 2: prove a halted run resumes by fork and document the recipe** - `5a7b993d` (test)

## Verification

- `cargo test -p paladin-ai --lib application::services::treasurer`: 53 passed. `cargo test -p paladin-battalion --lib child_battalion_halt_on_spend_halts_the_parent`: 1 passed; `--lib spend_guard`: 4 passed; the whole `paladin-battalion` lib: 822 passed. `cargo test -p paladin-ai --lib unattributed_run_gets_no_guard_and_reads_no_ledger`, `halted_run_resumes_by_fork_after_window_reset`, `ledger_unavailable_halt_resumes_after_recovery` and `engine_spend_halt_tracer`: 1 passed each.
- `cargo test -p paladin-ai --lib --features web-server --no-fail-fast`: 1216 passed, 2 did not pass (the known `run_api_wiring` pair, see Issues Encountered).
- `cargo clippy -p paladin-ai -p paladin-battalion --all-targets --all-features -- -D warnings` clean (re-run after Task 2 for `paladin-ai`); `cargo fmt --check` clean; `./scripts/check-migration-allowlist.sh` exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` unchanged (4195 items, so no baseline refresh or MIGRATION 9.2 row was needed).
- Acceptance greps: `grep -c UserRole guard.rs` prints 0; the `settlement.rs` diff has no non-comment line; `git diff --name-only HEAD~1 HEAD -- src crates` for the Task 2 commit lists only `http_surface_tests.rs`.
- Not run here: the PostgreSQL contract legs (no server in this sandbox; this plan adds none) and the Redis integration tests (no server).
- Manual credential-handling review: the guard still holds identity only (tenant and key NAME), the fail-closed line takes the run id, scope-kind labels, tenant id and the error text and no key value, and the resume tests assert no response body contains the key value.

## Decisions Made

- No guard behaviour had to change to make the boundary tests pass: 42-02's implementation already met every clause. The one production change closes a gap the plan's own must-have named: the `error`-level fail-closed line named run, tenant and error but not the scope kind. It now lists the scope kinds of the ceilings that apply to the principal (a label like `api_key`, never a value), built by `fail_closed_message`.
- The log line is tested through that pure builder instead of a captured record, because the paladin-ai lib test binary already shares one process-wide logger slot with the `log_sink` tests (which install theirs with a tolerated "already set" error), so a second capturing logger would be order-dependent.
- The ledger-outage resume test also asserts the fork is refused `500` while reads still fail: it shows admission fails closed on the resume path too, and costs nothing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Fail-closed log line lacked the scope kind**
- **Found during:** Task 1
- **Issue:** The must-have requires one `error` line naming the run, scope kind, tenant id and backend error; `guard.rs` logged run, tenant and error only.
- **Fix:** Added `fail_closed_message`, listing the deduplicated scope kinds of `AllowancePolicy::ceilings_for(subject)`; a unit test pins its content.
- **Files modified:** `src/application/services/treasurer/guard.rs`, `src/application/services/treasurer/tests.rs`
- **Commit:** `169cf78e`

### Plan wording interpreted

- **Child-halt test guard.** The plan says the test-local guard halts "from its first call". The parent consults the same guard first (its own boundary 1), so a guard halting from call 1 would halt the parent before any Battalion node dispatched and prove nothing about the child. The guard instead continues at the parent's first boundary and halts at the second (the child's first), then memoises, which is exactly the shape that distinguishes a shared guard from an unshared one.
- **Field name.** The plan says `requested_by: None` for the unattributed run; the run row field is `submitted_by` (`requested_by` is the submission request's principal). The test leaves `submitted_by` unset.

**Total deviations:** 1 auto-fixed, 2 wording interpretations; none change the design, a decision or the public surface.

## Auth Gates

None.

## Issues Encountered

- Pre-existing and out of scope: `run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` do not pass in this sandbox (they need outbound network), already in `deferred-items.md` from 42-02. They are the only two failures in the paladin-ai lib run.
- Disk stayed above 7 GB, so no cache deletion was needed; every cargo command ran with `CARGO_INCREMENTAL=0`.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file-access pattern or schema change; the threat model's T-42-14 through T-42-16 and T-42-18 mitigations are each now pinned by a named test, and T-42-17 stays an accepted race (backstop truth, recorded in WINDOWS.md by plan 42-12).

## Self-Check: PASSED

- Files: all nine listed modified files exist and the Task 2 commit touches only `http_surface_tests.rs` under `src` and `crates`.
- Commits: `169cf78e` and `5a7b993d` are on `claude/laughing-dirac-e0h2ax`.
