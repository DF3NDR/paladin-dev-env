---
phase: 42-mid-run-halt-sse-terminal-status
fixed_at: 2026-10-07T13:20:00Z
review_path: .planning/phases/42-mid-run-halt-sse-terminal-status/42-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 42: Code Review Fix Report

**Fixed at:** 2026-10-07
**Source review:** .planning/phases/42-mid-run-halt-sse-terminal-status/42-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-1, WR-1, WR-2, WR-3, WR-4; IN-* not touched)
- Fixed: 5
- Skipped: 0

Four of the five fixes are logic changes and are marked "requires human verification" below: the
tests passing proves the intended behaviour, not that the product semantics are the ones you want.
Two fixes are deliberately narrower than the review's full suggestion (WR-1 is documentation only,
WR-2 fixes the replay side only); each states its residual and a proposed follow-up.

Verification run before the final report, in the isolated worktree on the shared target dir:
`cargo fmt --check`, `cargo clippy -p paladin-battalion -p paladin-ports --all-targets -- -D warnings`,
`cargo clippy --lib --tests -- -D warnings`, `cargo test -p paladin-battalion -p paladin-ports`
(834 + doc tests pass), `cargo test --lib` (1249 pass), `cargo test --test treasurer_vocabulary_guard`,
`make api-surface` (failed once for the intended WR-4 addition; baseline refreshed with
`make api-surface-update` in the WR-4 commit).

## Fixed Issues

### CR-1: A nested `Battalion` child that halts is recorded as a completed node

**Status:** fixed: requires human verification
**Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`, `crates/paladin-battalion/CHANGELOG.md`
**Commit:** 22d1aacd
**Applied fix:** The child's `Ok(RunOutcome::Halted { cause, .. })` arm now returns a new private
`NodeFailure::ChildHalted(cause)` instead of a successful empty delta. The dispatch task converts it to a
new `NodeRunOutcome::ChildHalted(cause)`, which the bookkeeping loop records exactly like the existing
`Interrupted` outcome (`Skipped { reason: "shutdown" }`, never merged, node pushed onto
`aborted_node_ids`). The existing "aborted node" Halted-Waypoint branch therefore re-lists the Battalion
node on the vanguard, so a resume re-enters the child through its own `Halted` Waypoint (resume-mid-child
path) and returns `RunOutcome::Halted` in the same superstep, with no dependence on a successor node.
The child's own cause survives: probe cancel wins, then a child `CancelRequested`, then a cancelled
token, then the child's cause (so `Spend(..)` reaches the parent), then `Token`. The halted child is
never retried and never handed to an `on_error` handler. Tests added first and seen red:
`terminal_child_battalion_spend_halt_halts_the_parent_and_is_resumable` and
`child_battalion_spend_halt_with_successor_is_resumable_through_the_child` (assert Halted with the
child's `Spend` cause, a Halted Waypoint whose vanguard contains `sub`, and that a resume completes
with the child's node running exactly once). The pre-existing
`cancellation_is_observed_at_the_child_superstep_boundary` and
`child_battalion_halt_on_spend_halts_the_parent` pass unchanged (only the latter's doc comment was
updated).

### WR-1: A nested child's boundary guard is blind to its own run's spend

**Status:** fixed (documentation only; enforcement not implemented)
**Files modified:** `.planning/decisions/0057-mid-run-halt-contract.md`, `crates/paladin-ports/src/output/spend_guard.rs`, `docs/src/api-reference/platform-api.md`, `CHANGELOG.md`
**Commit:** 3e4d08a6
**Applied fix:** Took the review's "at minimum" path: the overshoot bound is now stated as one
TOP-LEVEL superstep, including any nested Battalion run it contains, in ADR-0057's "Overshoot, stated
exactly" paragraph, a new "Overshoot bound and nested runs" section on the `SpendGuard` rustdoc, the
platform-api allowance-notice section and the CHANGELOG line that claimed "one superstep".
**Residual (not enforced):** a long child graph can still overspend by its whole length. Enforcing it
needs either a derived child settlement key (`SettlementKey` is `(run_id, superstep, attempt)`, so
reusing the parent's `run_id` with child superstep numbers would collide and be dropped as
`AlreadySettled`, hence a design decision) or a `SpendGuard::check` signature change carrying the
in-flight total. I did not add a `WINDOWS.md` row: that ledger is machine-managed (frontmatter counts
plus a JSON block, and this `gsd-tools` build exposes no `windows` command), and row 65 already records
the per-run one-superstep bound. Suggest amending row 65's description with "one top-level superstep,
including nested Battalion runs".

### WR-2: Replay is scoped to the thread, not the run

**Status:** fixed (replay side): requires human verification; trace-store key not changed
**Files modified:** `src/application/services/run/events.rs`, `src/application/services/run/stream_tests.rs`, `CHANGELOG.md`
**Commit:** 2724a7bb
**Applied fix:** `replay_stream` now skips any record whose `run_id` is `Some(other)` (records with no
run id are kept, so legacy rows replay as before), so a prior run's `RunFinished` can no longer end a
later run's replay after a fork. `replay_terminal_override` also skips a persisted
`RunFinished { Halted }` when the run row is `Completed` or `Failed` (a drained worker's reasonless
halt record for a run that was requeued and ran on); the existing convergence cases (row `Cancelled`
or `Halted` over a `halted` record) are unchanged. Tests added first and seen red:
`replay_ignores_another_runs_records_on_a_shared_thread` and
`replay_skips_a_drained_halted_record_when_the_run_later_completed`.
**Residual (not fixed, needs a schema or port change):** `run_traces` is keyed `(thread_id, seq)` with
`ON CONFLICT DO NOTHING` and each dispatch starts `seq` at 1, so a second run on the same thread still
loses any record whose `seq` the first run used. The replay now shows only the second run's surviving
records and still ends on its own terminal event (or the row-synthesized one), but its early events can
be missing. Recorded under "Known limitations" in `CHANGELOG.md`. Proposed patch: add
`TraceDispatcher::with_seq_origin(u64)` and seed it from the thread's current maximum `seq` in
`worker.rs` (needs a max-seq query on `RunTracePort`, a public port addition), or key `run_traces` on
`(thread_id, run_id, seq)` with a new migration 015 for sqlite and postgres.

### WR-3: The guard's in-run notice memo is set before the store write

**Status:** fixed: requires human verification
**Files modified:** `src/application/services/treasurer/mod.rs`, `src/application/services/treasurer/guard.rs`, `src/application/services/treasurer/tests.rs`, `CHANGELOG.md`
**Commit:** 6bd6744a
**Applied fix:** `claim_record`, `claim_notice` and `claim_halt_notice` now return a crate-private
tri-state `NoticeClaim { Won(Box<AllowanceNotice>), AlreadyRecorded, Failed }` instead of collapsing a
store error into `None`. The guard no longer inserts the claim key before the write: it checks
`already_claimed`, performs the claim, then `settle_claim` memoises only `Won` and `AlreadyRecorded`,
so a `Failed` write is retried at the next boundary. Admission uses the `Won` arm only (unchanged
behaviour). Test added first and seen red:
`a_failed_mid_run_warning_claim_is_retried_at_the_next_boundary` (fails once, then succeeds; asserts a
retry, exactly one warning, one queued delivery, and that the memo applies again afterwards).
**Residual:** a halt notice lost to a store error is still not retried by the same run, because a
spend halt is sticky and the run ends at once (the memo no longer blocks a retry, but there is no later
boundary). Noted in the CHANGELOG.

### WR-4: A failed or skipped settlement silently blinds the guard

**Status:** fixed: requires human verification
**Files modified:** `crates/paladin-ports/src/output/spend_guard.rs`, `crates/paladin-battalion/src/engine/settlement.rs`, `crates/paladin-battalion/src/engine/superstep.rs`, `crates/paladin-battalion/src/engine/mod.rs`, `src/application/services/treasurer/guard.rs`, `src/application/services/treasurer/tests.rs`, `.project/current-exports.txt`, `CHANGELOG.md`, `crates/paladin-ports/CHANGELOG.md`, `crates/paladin-battalion/CHANGELOG.md`
**Commit:** 44209d8e
**Applied fix:** `SpendHook::settle_boundary` now returns a crate-private `SettleHealth`
(`ChargeLost` for a ledger `Err` or a currency mismatch; `Healthy` for written, duplicate, nothing to
charge, or a child hook). After settling, the engine calls a new defaulted, synchronous
`SpendGuard::note_unsettled_spend(&ThreadId)` when the charge was lost. `TreasurerSpendGuard` records
it in a shared `AtomicBool` and, at its next check, answers sticky
`Halt(HaltReason::LedgerUnavailable)` when a ceiling applies and shows headroom (an exhausted ceiling
already halts; a principal with no ceiling still gets `Continue`). Settlement still never fails,
retries or halts a run itself. Tests: `settlement.rs` unit tests for the three health outcomes, the
engine tests `a_lost_settlement_is_reported_to_the_guard_and_halts_the_next_boundary` (seen red before
wiring) and `a_lost_settlement_does_not_halt_a_run_whose_guard_ignores_the_signal`, and the treasurer
tests `guard_fails_closed_once_a_superstep_charge_could_not_be_written` and
`an_unsettled_charge_does_not_halt_a_principal_without_a_ceiling`.
**Please confirm (public surface and semantics):** this adds one public, non-breaking method to the
published `SpendGuard` port (`TreasurerSpendGuard` gains the override, which is the one line the
api-surface baseline picked up). It deliberately changes the earlier "a ledger failure never halts a
run" posture for runs that have a guard on a metered principal, matching the review's D-03 argument.
The older [Unreleased] CHANGELOG lines that say a ledger failure "never halts" were left as history.

## Skipped Issues

None. No finding was skipped.

## Process notes

- Work ran in an isolated worktree on branch `gsd-reviewfix/42-2711`, fast-forwarded onto
  `claude/laughing-dirac-e0h2ax` at the end; nothing was pushed and no existing commit was rewritten.
- The disk filled once mid-run (`No space left on device`). I deleted the regenerable
  `target/debug/incremental` cache under the shared `target/` dir to continue and built with
  `CARGO_INCREMENTAL=0` afterwards. No source or tracked file was affected.

---

_Fixed: 2026-10-07_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
