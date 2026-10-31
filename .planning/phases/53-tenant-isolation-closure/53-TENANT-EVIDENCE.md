# Phase 53 Tenant Evidence (plan 53-01)

Recorded 2026-10-31 on branch `claude/laughing-dirac-e0h2ax`. Plan base: `bf137d850b39aeeb8fdcd9240ef1cdc4cc5fd5d3`.
Every `test result:` line below was copied from a run on the plan's base, before any edit
(D-00d), unless the row says otherwise.

## Prove-then-lock (D-00d)

Root-crate commands use the prefix `cargo test --lib --features web-server` and the full module
path (`application::services::run::...`) with `-- --exact`.

| Requirement | Acceptance criterion | Pre-existing test | Command | `test result:` line | Gap |
|-------------|----------------------|-------------------|---------|---------------------|-----|
| TENANT-04 | AC-07 submit onto another tenant's thread is `UnknownThread` | `submit_on_another_tenants_thread_is_rejected_as_unknown_thread` | `cargo test --lib --features web-server application::services::run::submission::tests::submit_on_another_tenants_thread_is_rejected_as_unknown_thread -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1458 filtered out; finished in 0.00s` | No HTTP-level test per route (closed by `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404`, below) |
| TENANT-04 | AC-07 owner, Admin, internal caller and fresh thread ids pass the guard | `submit_thread_tenant_guard_admits_owner_admin_internal_and_fresh_threads` | `cargo test --lib --features web-server application::services::run::submission::tests::submit_thread_tenant_guard_admits_owner_admin_internal_and_fresh_threads -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1458 filtered out; finished in 0.00s` | None at service level |
| TENANT-04 | AC-07 fork onto another tenant's thread is `UnknownThread` | `fork_on_another_tenants_thread_is_rejected_as_unknown_thread` | `cargo test --lib --features web-server application::services::run::submission::tests::fork_on_another_tenants_thread_is_rejected_as_unknown_thread -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1458 filtered out; finished in 0.00s` | No HTTP-level test per route (fork half closed in Task 2) |
| TENANT-05 | AC-08 foreign cancel is `NotFound` and writes nothing | `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` | `cargo test --lib --features web-server application::services::run::submission::tests::cancel_of_another_tenants_run_is_not_found_and_writes_nothing -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1458 filtered out; finished in 0.00s` | None |
| TENANT-05 | AC-08 owner and Admin still cancel | `cancel_admits_the_owning_tenant_and_admin` | `cargo test --lib --features web-server application::services::run::submission::tests::cancel_admits_the_owning_tenant_and_admin -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1458 filtered out; finished in 0.00s` | None |
| TENANT-05 | AC-08 foreign cancel over HTTP is a 404 with no cancel flag | `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` | `cargo test --lib --features web-server application::services::run::http_surface_tests::cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1458 filtered out; finished in 0.12s` | None |
| TENANT-06 | Run-store errors are a generic 500 without backend text (get) | `get_run_repository_error_is_a_generic_500_without_backend_text` | `cargo test -p paladin-web --lib run_controller` | `test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 239 filtered out; finished in 0.14s` | Only an Admin principal is exercised; the non-admin `404` is new behavior owned by plan 53-02 |
| TENANT-06 | Run-store errors are a generic 500 without backend text (list) | `list_runs_repository_error_is_a_generic_500_without_backend_text` | `cargo test -p paladin-web --lib run_controller` | (same run: 56 passed, 0 failed) | Same gap, owned by plan 53-02 |

Whole-module base check: `cargo test --lib --features web-server submission::tests` printed
`test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 1427 filtered out; finished in 0.01s`.

### Tests added by this plan

| Test | Where | Passed on arrival? | Watched-red line |
|------|-------|--------------------|------------------|
| `permits_thread_admits_all_and_only_the_latest_runs_tenant` | `crates/paladin-core/src/platform/container/principal.rs` | No: written before the method, RED was a compile failure `error[E0599]: no method named \`permits_thread\` found for enum \`principal::RunReadScope\`` | (compile red, then green: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 742 filtered out`) |
| `RunReadScope::permits_thread` doctest | same file | Written with the method | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 188 filtered out` |
| `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404` | `src/application/services/run/http_surface_tests.rs` | Yes (the behavior already existed) | See below |

### TENANT-04 watched-red (PD-01-3), submit route

The behavior already existed, so the lock passed on arrival. To prove the test can fail, the
guard arm in `ensure_thread_visible` was changed locally to `Some(latest) if false && !scope.permits_thread(Some(&latest)) =>`
(never committed; the exact file was restored from a backup copy and the test re-run green).
With the guard disabled the test failed at `src/application/services/run/http_surface_tests.rs:1623`:

```
assertion `left == right` failed: foreign submit: {"error":{"code":"thread_busy","details":{"remedy":"resume"},"message":"thread 'acme-thread-1' already has an active run -- resume it via POST /threads/acme-thread-1/resume"}}
  left: 409
 right: 404
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1459 filtered out; finished in 0.53s
```

That failure also shows what the guard protects: without it a foreign tenant is told the thread
exists and is busy. Restored run: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1459 filtered out; finished in 0.21s`.

### TENANT-04 watched-red (PD-01-3), fork route

`cross_tenant_fork_is_the_unknown_thread_404` (`src/application/services/run/http_surface_tests.rs`)
mounts `thread_router(ThreadApiState::new().with_waypoints(..).with_runs(..).with_run_submission(..).with_auth(..))`,
the same wiring `src/bin/paladin-server.rs` uses, over the real `RunSubmissionService`, an in-memory
repository, queue and Waypoint store, and a worker that ran the owner's run to completion.
It passed on arrival (`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1460 filtered out; finished in 0.01s`).
With the same local, uncommitted mutation (guard arm `if false && ..`, restored byte-for-byte from a
backup afterwards, `git diff` empty for `submission.rs`) it failed:

```
thread 'application::services::run::http_surface_tests::cross_tenant_fork_is_the_unknown_thread_404' panicked at src/application/services/run/http_surface_tests.rs:1806:9:
assertion `left == right` failed: foreign fork: {"run_id":"01a18fb6-356f-7773-92b2-adbdfaef6ed5","thread_id":"acme-fork-1"}
  left: 202
 right: 404
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1460 filtered out; finished in 0.10s
```

Without the guard a foreign tenant's fork is accepted and dispatches a new run onto the owner's thread.
The test asserts, for the foreign user key: `404`, `error.code == "not_found"`,
`error.message == "unknown thread 'acme-fork-1'"`, run count unchanged (1) and queue depth unchanged.
The owner key then gets `202`; an Admin of `ops-tenant` then gets `409 thread_busy` (the owner's fork is active), which proves the guard admitted it.

### TENANT-05 lock re-run on this plan's head (AC-08)

AC-08 is **met by an existing test, with no router in the service test**: no new cancel test was added (D-00d).
Re-run on this plan's head (after commit `2b16763d0` and the Task 2 test added):

| Test | Command | `test result:` line |
|------|---------|---------------------|
| `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` (service-level, foreign `PrincipalRef` -> `RunSubmissionError::NotFound`, `cancel_requested == false`) | `cargo test --lib --features web-server application::services::run::submission::tests::cancel_of_another_tenants_run_is_not_found_and_writes_nothing -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1460 filtered out; finished in 0.00s` |
| `cancel_admits_the_owning_tenant_and_admin` (owner and Admin still cancel) | `cargo test --lib --features web-server application::services::run::submission::tests::cancel_admits_the_owning_tenant_and_admin -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1460 filtered out; finished in 0.00s` |
| `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` (HTTP, through `run_router`) | `cargo test --lib --features web-server application::services::run::http_surface_tests::cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag -- --exact` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1460 filtered out; finished in 0.18s` |

## Semver probe

Command: `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-ai-core`
(`cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.11.0 --release-type patch`),
run with `RunReadScope::permits_thread` added:

```
semver: paladin-ai-core: 223 checks: 223 pass, 31 skip
semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
```

The added inherent method passes at patch level. Plan 53-08 is the binding run.

## Residuals

- **Corrupt latest-run row on submit (PD-01-4, T-53-05, accepted).** A corrupt latest-run row read by
  `ensure_thread_visible` keeps today's redacted `500 "run store error"` (it reaches
  `map_submission_error`'s `Backend` arm and carries no backend text). D-09 limits the role-dependent
  `404` mapping to `load_visible_run` and `load_visible_thread`; fork gets it through the controller
  gate in plan 53-03.
- **Fork body equality (PD-01-5).** On today's tree a foreign fork answers `unknown thread '<id>'`, while a
  never-existing thread answers `unknown waypoint '<wp>' on thread '<id>'` (the service checks the waypoint
  after the guard). Byte equality of foreign-versus-missing fork bodies is proven at the controller by plan
  53-03's route matrix, not by this plan's test.
- **Thread birth (T-53-04, accepted; 40-CONTEXT D-14).** `submit` accepts a caller-chosen id for a thread
  with no runs, so an unused thread id stays distinguishable from a hidden one on submit. Thread ids are
  UUIDv7 by default; tenant-namespaced thread ids are deferred. `ensure_thread_visible` therefore calls
  `permits_thread` only with `Some(&latest)`.
- **`make api-surface` (not run).** `.project/current-exports.txt` covers the root crate only and holds
  no `RunReadScope` entry; this plan changes no root-crate public item (the `ensure_thread_visible` edit
  is a private fn body), so the baseline is unaffected.
