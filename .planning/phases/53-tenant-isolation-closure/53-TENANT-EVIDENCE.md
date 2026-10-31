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
