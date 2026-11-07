---
phase: 53-tenant-isolation-closure
plan: 01
subsystem: tenant-isolation
tags: [rust, tenant-isolation, run-read-scope, thread-ownership, axum, tdd, prove-then-lock]

requires:
  - phase: 40-tenant-scoped-runs
    provides: RunReadScope, PrincipalRef, ensure_thread_visible (WR-01), cancel scope check
provides:
  - "RunReadScope::permits_thread, the one thread-owner rule, in paladin-ai-core"
  - "ensure_thread_visible routed through the core rule with thread birth kept open"
  - "HTTP-level TENANT-04 locks for POST /v1/runs {thread_id} and POST /v1/threads/{id}/fork"
  - "53-TENANT-EVIDENCE.md: prove-then-lock record for TENANT-04, TENANT-05, TENANT-06"
affects: [53-02, 53-03, 53-04, 53-07, 53-08]

tech-stack:
  added: []
  patterns:
    - "A thread belongs to the tenant of its latest run (newest by submitted_at, run_id tiebreak); the no-run policy stays with the caller"
    - "Prove-then-lock: run the pre-existing tests on the base, add only the missing HTTP lock, show it red with a local uncommitted mutation"

key-files:
  created:
    - .planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md
  modified:
    - crates/paladin-core/src/platform/container/principal.rs
    - src/application/services/run/submission.rs
    - src/application/services/run/http_surface_tests.rs
    - crates/paladin-core/CHANGELOG.md

key-decisions:
  - "permits_thread answers only 'may this scope see a thread whose latest run is X'; ensure_thread_visible calls it only with Some(&latest) so a first submit to a fresh thread id still works (PD-01-1)"
  - "Ordering key is submitted_at DESC, run_id DESC; no doc written here says created_at for runs (PD-01-2)"
  - "Corrupt latest-run row on submit stays the redacted 500 'run store error'; D-09 confines the role-dependent 404 to the load helpers (PD-01-4)"
  - "Foreign-versus-missing fork body equality is deferred to 53-03's controller route matrix (PD-01-5)"

patterns-established:
  - "Watched-red by local mutation: replace the guard arm with 'if false && ..', copy the failing assertion into the evidence file, restore from a backup copy"

requirements-completed: [TENANT-03, TENANT-04, TENANT-05]

coverage:
  - id: D1
    description: "RunReadScope::permits_thread exists in paladin-ai-core with a doctest and a six-row unit test"
    requirement: "TENANT-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib principal::tests::permits_thread_admits_all_and_only_the_latest_runs_tenant"
        status: pass
      - kind: other
        ref: "cargo test -p paladin-ai-core --doc permits_thread"
        status: pass
    human_judgment: false
  - id: D2
    description: "ensure_thread_visible uses the core rule and keeps thread birth open"
    requirement: "TENANT-04"
    verification:
      - kind: unit
        ref: "submission::tests::submit_thread_tenant_guard_admits_owner_admin_internal_and_fresh_threads"
        status: pass
    human_judgment: false
  - id: D3
    description: "POST /v1/runs {thread_id} on another tenant's thread is the unknown-thread 404 over real HTTP, nothing inserted or enqueued"
    requirement: "TENANT-04"
    verification:
      - kind: integration
        ref: "http_surface_tests::cross_tenant_submit_on_a_thread_is_the_unknown_thread_404"
        status: pass
    human_judgment: false
  - id: D4
    description: "POST /v1/threads/{id}/fork on another tenant's thread is the unknown-thread 404 over thread_router, nothing dispatched"
    requirement: "TENANT-04"
    verification:
      - kind: integration
        ref: "http_surface_tests::cross_tenant_fork_is_the_unknown_thread_404"
        status: pass
    human_judgment: false
  - id: D5
    description: "Foreign cancel isolation (AC-08) evidenced by its existing no-router service test, re-run on this head"
    requirement: "TENANT-05"
    verification:
      - kind: unit
        ref: "submission::tests::cancel_of_another_tenants_run_is_not_found_and_writes_nothing"
        status: pass
    human_judgment: false

duration: 10min
completed: 2026-10-31
status: complete
---

# Phase 53 Plan 01: Thread-owner rule and submit/fork/cancel isolation locks Summary

**One thread-owner rule, `RunReadScope::permits_thread`, written once in core and called by `ensure_thread_visible`, with real-HTTP locks proving a foreign tenant gets the `unknown thread` 404 on `POST /v1/runs {thread_id}` and `POST /v1/threads/{id}/fork`, and nothing is inserted or dispatched.**

## Performance

- **Duration:** about 10 min wall clock (dominated by cold root-crate compiles)
- **Started:** 2026-10-31T01:59:06Z
- **Completed:** 2026-10-31T02:09Z
- **Tasks:** 2 (Task 1 tracer, Task 2 auto)
- **Files modified:** 5 (4 modified, 1 created)
- **Plan start SHA:** `bf137d850b39aeeb8fdcd9240ef1cdc4cc5fd5d3`

## Accomplishments

- `RunReadScope::permits_thread(&self, latest: Option<&Run>) -> bool` added beside `permits`: `All` permits every thread, `(Tenant, None)` is `false`, `(Tenant, Some(run))` defers to `permits`. It has a `?`-based doctest and the unit test `permits_thread_admits_all_and_only_the_latest_runs_tenant` (six rows). Module docs now name three applications of the rule.
- `ensure_thread_visible` calls `permits_thread(Some(&latest))` and no other form. `None` (no run row, thread birth) still returns `Ok(())`. Rustdoc says "newest by `submitted_at`" and records the corrupt-row residual.
- `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404`: a `globex` key on `acme-thread-1` gets `404`, `not_found`, `unknown thread 'acme-thread-1'`, run count and queue depth unchanged. The acme peer key and an Admin of `ops-tenant` get `409 thread_busy` (guard passed). A fresh `globex-fresh-7` thread gets `202`.
- `cross_tenant_fork_is_the_unknown_thread_404`: the same outcome over `thread_router` wired like `paladin-server.rs` (`with_waypoints`, `with_runs`, `with_run_submission`, `with_auth`). Owner fork is `202`, Admin fork is `409 thread_busy`.
- `53-TENANT-EVIDENCE.md` records base-run results (command and `test result:` line) for TENANT-04, TENANT-05 and TENANT-06, both watched-red lines, the TENANT-05 re-run on head (AC-08), the semver probe, and the residuals.
- `crates/paladin-core/CHANGELOG.md` gained `## [Unreleased]` above `[0.11.0]` (9 added lines, `[0.11.0]` untouched).

## Task Commits

1. **Task 1 (tracer): core rule wired through `ensure_thread_visible` to `POST /v1/runs {thread_id}`** - `2b16763d0` (feat)
2. **Task 2: fork HTTP lock, cancel lock record, CHANGELOG, semver probe** - `66002e74b` (test)

Tracer feedback gate (auto mode active): the tracer's full `<verify>` was re-run after its commit, all five commands green, before Task 2 began.

## Watched-red lines (PD-01-3)

Both locks passed on arrival because the behavior already existed. Each was shown to fail with a local, uncommitted mutation of the guard arm (`Some(latest) if false && ..`), then the file was restored from a backup copy.

Submit (`http_surface_tests.rs:1623`):
```
assertion `left == right` failed: foreign submit: {"error":{"code":"thread_busy",...,"message":"thread 'acme-thread-1' already has an active run -- resume it via POST /threads/acme-thread-1/resume"}}
  left: 409
 right: 404
```

Fork (`http_surface_tests.rs:1806`):
```
assertion `left == right` failed: foreign fork: {"run_id":"01a18fb6-356f-7773-92b2-adbdfaef6ed5","thread_id":"acme-fork-1"}
  left: 202
 right: 404
```

The core unit test was red as a compile failure (`error[E0599]: no method named permits_thread found for enum RunReadScope`) before the method existed.

## Semver probe

`SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-ai-core`:

```
semver: paladin-ai-core: 223 checks: 223 pass, 31 skip
semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
```

## Verification

- `cargo test -p paladin-ai-core --lib principal`: 10 passed. `cargo test -p paladin-ai-core --doc`: 151 passed, 38 ignored (the ignored ones were already ignored).
- `cargo test --lib --features web-server cross_tenant_`: 3 passed (submit, fork, existing cancel).
- `cargo test --lib --features web-server application::services::run::submission`: 35 passed (the WR-01/WR-02 service tests unchanged).
- `cargo fmt --check`, `cargo clippy -p paladin-ai-core --all-targets -- -D warnings`, `cargo clippy --lib --tests --features web-server -- -D warnings`: all exit 0.
- `git diff bf137d850 -- MIGRATION.md` is empty. No `pub` signature changed, no `#[deprecated]`, no `MIGRATION.md` §11.2 row (CONTEXT D-02).

## Decisions Made

See `key-decisions` above. All four were pre-made in the plan (PD-01-1, PD-01-2, PD-01-4, PD-01-5) and followed as written.

## Deviations from Plan

None - plan executed exactly as written.

Notes that are not deviations:
- `make api-surface` was not run. `.project/current-exports.txt` covers only the root crate and has no `RunReadScope` entry; this plan changes no root-crate public item (the guard edit is a private fn body). Recorded in the evidence file's `## Residuals`.
- `requirements-completed` lists TENANT-03 because the plan frontmatter does, but this plan delivers only the shared rule. `requirements.mark-complete` checked TENANT-03 and the executor reverted that one item to Pending in REQUIREMENTS.md: its text covers every `/v1/threads/*` handler, and the gate lands in plans 53-03 and 53-04. TENANT-04 and TENANT-05 are marked complete.
- Both new tests passed on arrival (expected for D-00d locks of already-landed behavior); red was shown by mutation as the plan prescribes.

## Issues Encountered

None. A first background `cargo test` run wrote its log late and an empty file at first read; it was superseded by exact foreground runs, whose lines are the ones recorded.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access or schema change; the plan adds tests and one pure method. Threats T-53-01, T-53-02, T-53-03 and T-53-06 are mitigated and evidenced; T-53-04 and T-53-05 are accepted and recorded under `## Residuals` in the evidence file.

## Next Phase Readiness

Plans 53-03 and 53-04 can call `RunReadScope::permits_thread` for the thread gate. The fork test is already wired like the shipped server, so it stays valid once 53-03 puts `load_visible_thread` in front of `fork_thread`. Foreign-versus-missing fork body equality is owed by 53-03's route matrix (PD-01-5).

## Self-Check: PASSED

- FOUND: crates/paladin-core/src/platform/container/principal.rs (`pub fn permits_thread`)
- FOUND: src/application/services/run/submission.rs (`permits_thread(Some(&latest))`)
- FOUND: src/application/services/run/http_surface_tests.rs (both new tests)
- FOUND: .planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md (`## Prove-then-lock (D-00d)`, `## Semver probe`, `## Residuals`)
- FOUND: commits `2b16763d0` and `66002e74b`
