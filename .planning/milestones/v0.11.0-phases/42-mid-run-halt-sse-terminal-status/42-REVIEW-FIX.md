---
phase: 42-mid-run-halt-sse-terminal-status
fixed_at: 2026-10-07T18:00:00Z
review_path: .planning/phases/42-mid-run-halt-sse-terminal-status/42-REVIEW.md
iteration: 1
findings_in_scope: 16
fixed: 15
skipped: 1
status: partial
---

# Phase 42: Code Review Fix Report (round 2)

**Fixed at:** 2026-10-07
**Source review:** `.planning/phases/42-mid-run-halt-sse-terminal-status/42-REVIEW.md` (iteration 2, HEAD d44ba469)
**Iteration:** 1 (of this fix pass; it overwrites the previous round's report, which is preserved in git history)
**Branch:** `claude/laughing-dirac-e0h2ax` (worked directly in the checkout, no worktree, as instructed). Nothing pushed.

**Summary:**
- Findings in scope: 16 (WR-5, WR-6, IN-1 to IN-14; `fix_scope: all`)
- Fixed: 15
- Skipped: 1 (IN-7, not actionable by the review's own verdict)

15 commits, one per finding, each `fix(42): <ID> ...`. No `.planning` file is committed; this report is left uncommitted for the orchestrator.

**Needs human verification (logic or behaviour changes, not merely syntax):** WR-5, WR-6, IN-3, IN-6, IN-12. IN-4 changes a rendered string for a malformed input only. IN-9 is a behaviour-preserving extraction. All of these have tests that pass, but the tests encode my reading of the intent.

## Fixed Issues

### WR-5: replay stale-record skip only covered a `Completed`/`Failed` row
**Status:** fixed: requires human verification
**Files modified:** `src/application/services/run/events.rs`, `src/application/services/run/stream_tests.rs`
**Commit:** 89182bad
**Applied fix:** Followed the review's proposal. In `replay_stream`, when a terminal `RunFinished` survives the existing row rules, a new `later_record_of_run_exists` helper decides whether it really ends the run: it looks in `pending`, then reads further pages from the port (keeping only records of this run or with no run id) until it finds one or the port is empty. If a later same-run record exists, the terminal record is skipped; the replay then ends on the run's own last terminal record or, failing that, on the row-synthesized terminal event. The `awaiting_input` suspension record keeps ending the replay as before, and a port read error answers "no later record" so a store failure is not hidden behind an endless skip. The existing `Completed`/`Failed` skip is kept.
**Tests added (all pass):** `replay_skips_a_drained_halted_record_when_the_run_later_halted_on_spend`, `..._when_the_run_later_cancelled`, `replay_keeps_a_last_reasonless_halted_record_beside_a_cancelled_row` (the genuine-end guard: the new rule must not swallow a real end), `replay_skips_a_stale_halted_record_found_at_a_page_boundary` (stale record is the 256th record of the first page). I confirmed the new rule is what makes the Halted, Cancelled and page-boundary cases pass: pre-fix the stale record at seq 2 (or 256) is emitted as the terminal `done`.
**Residual:** the replay now reads one extra page when a terminal record is the last record of a page. A run whose trace legitimately has an unmapped (wire-silent) record after its `RunFinished` would fall back to the row-synthesized terminal event (losing `trace_seq` and usage on that `done`); the engine writes `RunFinished` last, so I do not expect this.

### WR-6: second run on a thread loses the trace records whose `seq` the first run used
**Status:** fixed: requires human verification (a design decision was taken, see below)
**Files modified:** `crates/paladin-ports/src/output/run_trace_port.rs`, `crates/paladin-battalion/src/engine/hooks.rs`, `crates/paladin-storage/src/run_trace/{in_memory,sqlite,postgres,contract_tests}.rs`, `src/application/services/run/worker.rs`, `src/application/services/run/stream_tests.rs`, `CHANGELOG.md` and the `paladin-ports`, `paladin-battalion`, `paladin-storage` CHANGELOGs
**Commit:** 7f1c4970
**Applied fix:** Option (a) from the review, which stays inside the constraints you gave (no migration, no change to the `(thread_id, seq)` key):
- `RunTracePort::max_seq(&ThreadId) -> Result<u64, RunTraceError>` with a defaulted implementation that pages through `read` (correct for any existing implementor, linear in rows); the in-memory, SQLite and PostgreSQL stores override it with an index lookup (`MAX(seq)`).
- `TraceDispatcher::with_seq_origin(origin)` (default `0`, uses `fetch_max` so the sequence can never move backwards).
- `RunWorkerPool::trace_seq_origin` seeds both dispatchers (`run_once` and `run_agent`) from the thread's max seq, only when `trace.persist` is on and a port is wired; a failed read logs the run id and error and falls back to `0`, since a trace never gates a run.
- The CHANGELOG "Known limitations" bullet for this defect is removed and replaced by a Fixed entry.
**Tests added (all pass):** `default_max_seq_pages_through_read` (two pages plus a gap), `seq_origin_starts_the_sequence_after_the_origin`, `seq_origin_zero_changes_nothing_and_never_moves_backwards`, a shared contract clause `max_seq_lets_a_second_run_on_a_thread_keep_its_records` (wired into the in-memory and SQLite suites and into `run_all`), and a worker-level test `a_second_run_on_a_thread_keeps_its_trace_records` (40 pre-existing records of another run on the thread; the new run's RunStarted through RunFinished are all persisted, numbered 41 upward and gapless). I confirmed the worker test fails (times out waiting for `RunFinished`) when the origin is forced to 0.
**Residual / for a human:**
- The PostgreSQL override and its contract test are Docker-gated and were NOT run here; they compile and pass `cargo clippy --features postgres -- -D warnings`.
- Records already lost to past collisions are not recoverable; only new dispatches are numbered after the thread's max.
- Two runs dispatched concurrently on the same thread can still read the same max and collide. That is not the fork/requeue path the review describes (those are sequential), and closing it needs option (b) (key on `(thread_id, run_id, seq)`, migration 015), which I did not take.
- If the maintainers prefer option (b), `max_seq` and `with_seq_origin` can be removed without touching stored data.

### IN-1: a spend halt ignores a persisted cancel flag
**Status:** fixed (documentation only, the review's safe default)
**Files modified:** `docs/src/api-reference/platform-api.md`
**Commit:** 860d2283
**Applied fix:** The "Cancelling a run" section now states that a cancel from another instance is read through a debounced probe and can miss the boundary where the guard halts on spend, in which case the run is recorded `halted` with its reason (ADR-0057). The `Spend` arm of `map_outcome` is unchanged. The product question (should such a run be `cancelled`?) is left open.

### IN-2: agent-kind spend halts claim no operator notice
**Status:** fixed (documentation only, the review's preferred default)
**Files modified:** `docs/src/api-reference/platform-api.md`
**Commit:** 8ce84edb
**Applied fix:** The platform docs now say the operator `allowance_halted` notice is sent only for a halt at an engine superstep boundary; an agent-kind halt records `halted` and notifies the run's own webhook only.
**Residual:** the reviewer also suggested rewording D-18 in `42-CONTEXT.md`. That file is under `.planning`, which you told me not to commit, so it is untouched and D-18's wording still reads wider than the code.

### IN-3: truncation notice calls an allowance halt "the final answer"
**Status:** fixed: requires human verification (changes user-visible output text)
**Files modified:** `src/application/services/paladin/middleware/limits.rs`, `src/application/services/run/http_surface_tests.rs`, `src/application/services/run/worker_tests.rs`, `src/infrastructure/web/agent_host.rs`, `docs/src/api-reference/platform-api.md`, `CHANGELOG.md`
**Commit:** ba69d500
**Applied fix:** New `ALLOWANCE_HALT_NOTICE` ("[budget] Allowance reached — this response is partial; the run was halted.") is appended when the stop is the allowance halt; `TOKEN_BUDGET_NOTICE` stays for the operator budget. Updated the three assertions in `limits.rs` and the three cross-module assertions that matched the old text, plus the two docs passages and a CHANGELOG `Changed` entry.
**Tests run (pass):** the limits middleware tests, `agent_kind_run_halts_on_the_derived_budget`, `agent_kind_run_crossing_its_derived_budget_is_halted_with_the_reason`, and `agent_execute_halts_on_the_derived_budget` (needs `--features web-server`, run that way).

### IN-4: `herald_line` labels a window refusal with no window as a lifetime cap
**Status:** fixed
**Files modified:** `crates/paladin-core/src/platform/container/allowance.rs`, `crates/paladin-core/CHANGELOG.md`
**Commit:** 117a6ef2
**Applied fix:** Both renderers (`HaltReason::herald_line`, `AllowanceWarning::herald_line`) now share one `horizon_phrase` helper that matches the limit kind explicitly: `(Window, Some(end))` gives "window resets ...", `(Window, None)` gives "window", `(Lifetime, _)` gives "lifetime cap". `AllowanceLimitKind` is not `#[non_exhaustive]`, so no wildcard arm was added. Two tests added, one per renderer. Only a malformed input renders differently; dated window and lifetime lines are unchanged.

### IN-5: derived-budget halt persists `balance == ceiling`
**Status:** fixed (documentation only, the review's safe default)
**Files modified:** `docs/src/api-reference/platform-api.md`
**Commit:** 29627488
**Applied fix:** The agent-kind halt section now states that `balance` on a halt at the derived figure is a conservative upper bound (the ceiling itself), not a ledger reading, and that a zero or exhausted figure at dispatch carries the measured balance.

### IN-6: one unreadable `halt_reason` fails `GET /runs`, `get` and the SSE poller
**Status:** fixed: requires human verification (behaviour change on the read path)
**Files modified:** `crates/paladin-storage/src/run/sqlite.rs`, `crates/paladin-storage/src/run/postgres.rs`, `crates/paladin-storage/Cargo.toml`, `Cargo.lock`, `crates/paladin-storage/CHANGELOG.md`
**Commit:** ab7ee51d
**Applied fix:** A `halt_reason` that does not deserialize now reads as `None` and logs the run id at `error` (never the stored text or the serde message, which can quote it). The write side stays strict. The pinning test was renamed to `an_unreadable_stored_halt_reason_reads_as_none_not_an_error` and now also asserts that `list` still returns both rows when one reason is unreadable.
**Residual:** this needed a new `log` dependency edge on `paladin-storage` (the workspace `log`, one added line in `Cargo.lock`); say so if a dependency change should have been a separate decision. A halted run whose reason is unreadable now reads as `halted` with no reason, which is the intended degradation. The PostgreSQL copy compiles and lints clean but has no test of its own (the existing one is SQLite; Docker-gated here).

### IN-7: migration 014 drops the arbiter index
**Status:** see Skipped Issues below.

### IN-8: outcome exposed while the row is still `Running`
**Status:** fixed (comment only, as the review prescribes)
**Files modified:** `src/application/services/run/worker.rs`
**Commit:** 12e3dc6b
**Applied fix:** A comment beside the G14 reorder states the narrow, self-healing window and why it is accepted.

### IN-9: `run_agent` repeats the `RunFinished` literal five times
**Status:** fixed (partially: see residual)
**Files modified:** `src/application/services/run/worker.rs`
**Commit:** fe00c2a7
**Applied fix:** `emit_agent_run_finished(&dispatcher, status, halt_reason, duration_ms)` replaces the five literals. Behaviour is unchanged (same field values, usage and cost still read from the dispatcher). Worker tests (`agent_kind`, `agent_budget`, `run::worker`: 71 tests) pass.
**Residual (not done, tracked):** bundling the per-call arguments of `execute_bounded` / `execute_internal` into a context struct, which would remove their two `too_many_arguments` allows, is a larger refactor of a hot service with no behavioural payoff, so I left it. The `paladin-eval` `Cancelled` gap needs no action per the review.

### IN-10: WR-4 fail-closed log line has a run of literal spaces and bypasses `fail_closed_message`
**Status:** fixed
**Files modified:** `src/application/services/treasurer/guard.rs`, `src/application/services/treasurer/tests.rs`
**Commit:** 9c7057e9
**Applied fix:** The line is now built through `fail_closed_message` (so it gains `scope=` and the shared shape) using a new `UNSETTLED_SPEND_ERROR` constant whose continuation no longer embeds padding. Added `fail_closed_log_line_for_a_lost_charge_is_one_greppable_line` (names run, scope, tenant; no double spaces; one line; no key value). All 117 treasurer tests pass.

### IN-11: WR-4 widened `ledger_unavailable` but the docs still say "could not be read"
**Status:** fixed
**Files modified:** `crates/paladin-core/src/platform/container/allowance.rs`, `crates/paladin-ports/src/output/spend_guard.rs`, `docs/src/api-reference/platform-api.md`
**Commit:** 4a02d9ce
**Applied fix:** Reworded the `HaltReason::LedgerUnavailable` doc, the `SpendGuard` module doc, and the platform-api.md passages (halt object, resume, `done` payload, operator-notice) to cover a charge that could not be written. The wire object is unchanged and no variant was added. `cargo doc` for both crates builds.
**Residual:** the platform-api.md passage on an agent-kind run's dispatch-time "unreadable ledger" halt was left as is: an agent-kind run has no superstep charge, so that sentence is already accurate.

### IN-12: a nested child's halt is recorded `Skipped { reason: "shutdown" }`
**Status:** fixed: requires human verification (changes a persisted and traced reason string)
**Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`, `crates/paladin-battalion/CHANGELOG.md`
**Commit:** 16952e43
**Applied fix:** `ChildHalted` now maps to `Skipped { reason: "child_halted" }` at both sites (`node_outcome_kind` and the Waypoint bookkeeping record), via one `CHILD_HALTED_REASON` constant; `Interrupted` keeps `"shutdown"`. I searched the trace, eval and golden fixtures and docs for assertions on `"shutdown"`: every hit is an `Interrupted` (drain) case, none for a child halt, so the review's "treat as not actionable" condition did not apply. `child_battalion_halt_on_spend_halts_the_parent` now pins the persisted reason. All 836 `paladin-battalion` lib tests pass.

### IN-13: WR-1 documented but not enforced; "one superstep" left in the CHANGELOG
**Status:** fixed (CHANGELOG half only)
**Files modified:** `CHANGELOG.md`
**Commit:** 9fa92887
**Applied fix:** The Known-limitations sentence now says "at most one top-level superstep's spend per run (including any nested Battalion run that superstep contains)".
**Residual:** `WINDOWS.md` row 65 is machine-managed under `.planning` and needs a human to amend it through the `gsd-tools` window flow, as the review says. Enforcement (a derived child settlement key or an `in_flight` argument on `SpendGuard::check`) remains a design change and was not attempted.

### IN-14: WR-3 retry has no ceiling, one doc line is 130 columns
**Status:** fixed (the mandatory half)
**Files modified:** `src/application/services/treasurer/guard.rs`
**Commit:** 25ec8c4f
**Applied fix:** Rewrapped the `claimed` doc comment to 100 columns (no other line in the file exceeds 100).
**Residual:** the optional "memoise after three consecutive `Failed` answers" cap was not applied; it is a behaviour change to the notice path that the review marked optional and the log noise is bounded by the superstep count.

## Skipped Issues

### IN-7: Migration 014 drops the arbiter index that older binaries' `ON CONFLICT` list targets
**Status:** skipped
**File:** `crates/paladin-storage/migrations/{postgres,sqlite}/014_add_treasury_notice_kind.sql`
**Reason:** The review itself downgrades this to "not actionable" and says the migration files must NOT be edited: sqlx records a checksum per applied migration, so changing even a comment in `014_*.sql` breaks startup for databases that already applied it, and the Postgres test `include_str!`s the file. The rollout caveat it wanted already exists in `MIGRATION.md`. No file touched.
**Original issue:** Migration 014 drops the arbiter index a pre-014 binary's `ON CONFLICT` list targets.

## Verification performed

Environment: root checkout, branch `claude/laughing-dirac-e0h2ax`. Disk fell to about 1.4 GB free at the end (target/ grew), so I stopped short of running doc-tests for the touched crates.

Run and passing:
- `cargo fmt --check`: clean at the end of every commit.
- `cargo clippy -p paladin-ai --lib --tests -- -D warnings` (after the root-crate commits); `cargo clippy -p paladin-ai-core --all-targets -- -D warnings`; `cargo clippy -p paladin-ports -p paladin-battalion -p paladin-storage --all-targets --features paladin-storage/sqlite,paladin-storage/postgres -- -D warnings`; `cargo clippy -p paladin-battalion --all-targets -- -D warnings`; `cargo clippy -p paladin-storage --all-targets --features sqlite,postgres -- -D warnings`. All clean.
- `cargo test --lib` (root crate, default features): 1255 passed, 0 failed, run at the end over all commits.
- `cargo test -p paladin-battalion --lib`: 836 passed (after IN-12). `engine::hooks` subset 21 passed (WR-6).
- `cargo test -p paladin-ports --lib run_trace`: 5 passed (WR-6).
- `cargo test -p paladin-storage --features sqlite --lib run_trace`: 29 passed (WR-6); `... run::`: 74 passed (IN-6).
- `cargo test -p paladin-ai-core --lib allowance`: 43 passed (IN-4).
- `cargo test --lib --features web-server -- agent_execute_halts_on_the_derived_budget`: 1 passed (IN-3).
- `cargo test --test treasurer_vocabulary_guard`: 3 passed.
- `cargo doc -p paladin-ports -p paladin-ai-core --no-deps`: builds.
- `make api-surface`: "API surface unchanged" (run before WR-6's commit and again at the end). The baseline tracks the root crate only, so the new `paladin-ports` and `paladin-battalion` public items (`RunTracePort::max_seq`, `TraceDispatcher::with_seq_origin`) are not in it; they are recorded in the per-crate CHANGELOGs and the root CHANGELOG instead. No `api-surface-update` was needed.

Not run:
- The PostgreSQL run-trace and run-repository tests (Docker-gated; compile and lint only).
- Doc-tests for `paladin-ports` / `paladin-ai-core` (disk). No doc example was changed; the `RunTracePort` doc example still compiles because `max_seq` is defaulted.
- The full workspace test suite and the other feature combinations.

---

_Fixed: 2026-10-07_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
