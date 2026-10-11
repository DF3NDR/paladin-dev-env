---
phase: 44-legacy-clean-break-removal
reviewed: 2026-10-11T00:00:00Z
depth: standard
iteration: 1
status: all_fixed
findings_in_scope: 2
fixed: 2
skipped: 0
new_findings:
  critical: 0
  warning: 1
  info: 2
  total: 3
new_findings_fixed: [WR-N1]
new_findings_queued: [IN-N1, IN-N2]
files_reviewed: 3
files_reviewed_list:
  - crates/paladin-battalion/src/aegis_attempt.rs
  - crates/paladin-battalion/src/phalanx_service.rs
  - crates/paladin-core/src/platform/container/battalion/phalanx.rs
---

# Phase 44: Code Review Fix Verification (iteration 1)

**Scope:** WR-01 and WR-02 from `44-REVIEW.md`.
**Commits:** `229a28a5` (RED tests), `cf0be3db` (fix, GREEN).

WR-03, WR-04, WR-05 and IN-01 through IN-05 were deliberately not in scope. They are queued as todos in
`.planning/todos/pending/2026-10-11-reject-removed-v0-10-battalion-config-keys.md` and
`.planning/todos/pending/2026-10-11-phase-44-review-follow-ups.md`. Both files exist and are untracked.

## Verdicts

| Finding | Verdict | Fixing commit |
|---------|---------|---------------|
| WR-02 zero `max_concurrency` hangs | **fixed** | `cf0be3db` |
| WR-01 cancelled Phalanx still starts queued Paladins | **fixed** | `cf0be3db` |

No Critical issues. One new Warning and two Info items, none a regression from the fix (details below).

## WR-02: fixed

- `validate_concurrency_limit` is at `phalanx_service.rs:505-513`. It returns `BattalionError::ValidationError` when `phalanx.max_concurrency() == Some(0)`.
- It is called as the second statement of both public entry points: `execute` at `phalanx_service.rs:160` and `execute_with_cancellation` at `phalanx_service.rs:186`. Both calls come after `validate_aegis()` and before `execute_internal`. In `execute_with_cancellation` it also comes before the `select!`, so a zero limit is rejected even when the token has already fired. No Paladin is spawned.
- No other path reaches `Semaphore::new(0)`:
  - `Semaphore::new(max)` for Phalanx exists only at `phalanx_service.rs:347`, inside `execute_collect_all`.
  - The private helpers have these callers only: `execute_collect_all` is called from `execute_internal` (219) and `execute_majority` (453). `execute_first_success` and `execute_majority` are called only from `execute_internal` (222, 224). `execute_internal` is called only from `execute` (168) and `execute_with_cancellation` (198).
  - Outside the service, `PhalanxExecutionService` is used only at `commander.rs:605-606`, `battalion_processor.rs:139-141`, `doc-examples/src/orchestration.rs:58`, and the benches. All of them call `execute`. Nothing else calls the private helpers. The two in-module test calls of `execute_collect_all` (`phalanx_service.rs:1614`) are tests.
  - `with_max_concurrency` has no production caller. The only callers are tests and `benches/battalion_benchmarks.rs:115`.
  - Unrelated semaphores are not Phalanx: `engine/superstep.rs:2709-2710` already clamps with `.max(1)`; `resource_controls.rs:181` is the Arsenal; `BattalionCheckpointConfig.max_concurrency` (`citadel.rs:287`) is only a persisted field.
  - `FirstSuccess` builds no semaphore, so it never reads `max_concurrency`. That is pre-existing; see IN-N1.
- `with_max_concurrency` keeps its infallible signature, so the public API is unchanged. Its rustdoc states the at-least-1 rule (`phalanx.rs:98-100`).
- Test `phalanx_rejects_zero_max_concurrency_before_any_paladin_runs` (`phalanx_service.rs:925-960`) covers both entry points. It wraps each call in a 5 s `tokio::time::timeout`, asserts `ValidationError` containing `max_concurrency`, and asserts that 0 port calls were made. Per the RED commit it hung before the fix. I did not re-run the RED state.

## WR-01: fixed

Three layers, all in `cf0be3db`:

1. **`run_paladin` permit wait** (`phalanx_service.rs:530-558`).
   - When a token exists, the wait is a `biased` `select!` with `token.cancelled()` listed first. A queued Paladin returns `AttemptFailure::cancelled(&name, 1)` without ever holding a permit or calling the port.
   - Because the token branch is polled first, a token that has already fired wins over a permit that is also ready. Tokio's `Acquire` is cancel-safe: the dropped acquire future is not granted a permit.
2. **`run_with_aegis` pre-attempt check** (`aegis_attempt.rs:208-214`).
   - `is_cancelled()` is checked at the top of every loop iteration, before `attempt_once`.
   - This covers the case where the permit is granted and the token fires between the select and the first await. There is no await point between the `select!` returning `Ok(permit)` and this check, so the only residual window is the instant between the check and `port.execute` starting. That is indistinguishable from an attempt already in flight.
3. **FirstSuccess** (`phalanx_service.rs:392-443`).
   - It has no semaphore, and its futures live inside `execute_internal`'s future. When `execute_with_cancellation` returns `Cancelled`, the `select!` drops the whole future tree, so nothing is left behind. The `run_with_aegis` check additionally stops attempts started after a fire within the same poll cycle.

Test `cancelled_phalanx_does_not_start_queued_paladins` (`phalanx_service.rs:880-917`) uses concurrency 1, 3 Paladins and a 1 s delay. It cancels at 100 ms, advances 5 s of virtual time, and asserts exactly 1 port call (3 before the fix). Test `cancelled_before_the_first_attempt_makes_no_call` (`aegis_attempt.rs:559-576`) pins the runner-level check.

I ran `CARGO_INCREMENTAL=0 cargo test -p paladin-battalion --lib -- cancelled phalanx_rejects_zero`. All 3 new tests pass: 7 passed, including 4 pre-existing cancel-related tests that matched the filter. The full `paladin-battalion --lib` run gives 874 passed, 0 failed.

## Regression check

- **Classification.**
  - `AttemptFailure::cancelled` (`aegis_attempt.rs:127-142`) sets `Transience::Permanent` and `NodeErrorSource::Cancelled`. That variant already existed (`node_error.rs:107`, Display "cancelled"), so there is no new public surface.
  - `RetryPredicate::admits` (`aegis.rs:188-196`) never admits `Permanent`, and `Custom` is always false, so no predicate retries it. The retry loop also never sees it: the check returns before `attempt_once`.
- **Absorb, `node_errors` and aggregation.**
  - Cancelled failures can only be produced when a token exists, and a token exists only in `execute_with_cancellation`.
  - That function's `biased` `select!` polls the token first. By the time any cancelled failure exists the token has already fired, so the caller receives `BattalionError::Cancelled` and the partial `failures` vector is never aggregated, absorbed or turned into `node_errors`.
  - On plain `execute` the token is `None`, so no cancelled failure is created and the Absorb and AggregationError paths are unchanged.
  - The detached tasks of a cancelled run produce failures into dropped join handles. This is harmless.
- **Formation and Campaign.** Both pass `&None` to `run_with_aegis` (`formation_service.rs:220`, `campaign_service.rs:350`). The new check is a no-op for them (`cancel.as_ref()` is `None`), so their behaviour is unchanged.
- **Retry interaction.** If the token fires during a backoff, `wait_backoff` already returned false and the original failure was returned. The new loop-top check is only reachable on attempt 1, or if the token fires between `wait_backoff` returning true and the next iteration. In that case the cancelled failure replaces an unobserved failure in a run whose result is discarded. Acceptable.
- **Panics.** The diff adds no `unwrap`, `expect` or `panic!` outside test code. `run_paladin` returns typed failures. Test-only `unwrap()` and `expect()` calls on mock mutexes are fine.
- **Tests.** The new tests are not tautological. Each asserts a call count or a bounded wait that the pre-fix code cannot satisfy, consistent with the recorded RED results: 1 call made in the runner test, 3 calls instead of 1, and the 5 s hang guard tripping.

## New findings (none block closing WR-01 and WR-02)

### WR-N1: `with_max_concurrency(usize::MAX)` still panics inside the service

**Resolution (orchestrator, 2026-10-11): fixed.** `validate_concurrency_limit` now also rejects a
limit above `Semaphore::MAX_PERMITS` with a `ValidationError` before any Paladin runs. RED test
`phalanx_rejects_a_concurrency_limit_above_the_permit_maximum` failed on tokio's
"a semaphore may not have more than MAX_PERMITS permits" panic (commit `4da2bcbb`),
then passed with the fix. `cargo test -p paladin-battalion --lib`: 875 passed; clippy -D warnings,
fmt and rustdoc -D warnings clean. Commits: RED `4da2bcbb`, GREEN `dc55f18e`.

**File:** `crates/paladin-battalion/src/phalanx_service.rs:345-347` (and `validate_concurrency_limit`, 505-513)

**Issue:** This is an adjacent gap in the WR-02 area, not introduced by the fix. `tokio::sync::Semaphore::new` panics when `permits > Semaphore::MAX_PERMITS` (`usize::MAX >> 3`). The new validation rejects only 0, so any limit above that, such as `with_max_concurrency(usize::MAX)` as a deliberate "unlimited" value, panics inside `execute_collect_all`. That breaks the project's no-panic-in-library-code rule and is reached from `execute` and `execute_with_cancellation`. The panic occurs before any Paladin is spawned.

**Fix:** Either clamp or reject in `validate_concurrency_limit`:

```rust
match phalanx.max_concurrency() {
    Some(0) => Err(ValidationError(...)),
    Some(n) if n > Semaphore::MAX_PERMITS => Err(ValidationError(...)),
    _ => Ok(()),
}
```

Alternatively use `.min(Semaphore::MAX_PERMITS)` when building the semaphore. Add a one-line test.

### IN-N1: Residual behaviours are documented only partially

**File:** `crates/paladin-battalion/src/phalanx_service.rs:171-177`, `528-535`

**Issue:**
- (a) An attempt already in flight when the token fires still runs to its end, and the detached task keeps its permit until then. The `run_with_aegis` rustdoc says so, but `execute_with_cancellation`'s rustdoc mentions only the backoff case. Adding one sentence about in-flight attempts and queued Paladins would close the loop.
- (b) `FirstSuccess` ignores `max_concurrency` entirely (no semaphore). This is pre-existing and not a defect of the fix. It may surprise a caller who sets a limit with that strategy.

**Fix:** Documentation only.

### IN-N2: Coverage gaps in the new tests

**File:** `crates/paladin-battalion/src/phalanx_service.rs:880-917`

**Issue:**
- No test covers the same-instant "permit granted as the token fires" race. The `select!` ordering and the runner check cover it by construction, but only by reasoning.
- No test covers cancellation of a `FirstSuccess` or `Majority` run with a queued limit.
- The queued test would still pass on layer 2 alone (the runner check), so it does not isolate the `select!` layer. The difference is that without the `select!` a queued Paladin would still take a permit before giving up.

**Fix:** Optional. Add a paused-clock test that cancels before `execute_with_cancellation` is polled with concurrency 1, and assert 0 calls.

---

_Reviewed: 2026-10-11_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
