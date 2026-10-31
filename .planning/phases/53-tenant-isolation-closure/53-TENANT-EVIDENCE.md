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

## Closing-test re-run (plan 53-07, T-53-37)

Re-run on the head of plan 53-07 (after the ADR and docs commits, before `windows fixed`), 2026-10-31.
The row 58 and row 63 closing tests passed before either row was marked fixed.

| Row | Command | `test result:` line |
|-----|---------|---------------------|
| 58 | `cargo test -p paladin-web --lib thread_controller::tests::every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 328 filtered out; finished in 0.01s` |
| 58 | `cargo test -p paladin-web --lib thread_controller::tests::list_threads_` | `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 318 filtered out; finished in 0.02s` (eight Phase 53 tests plus three pre-existing `list_threads_*` tests) |
| 63 | `cargo test --lib --features web-server schedule_scan_reports_every_unattributed_schedule_ascending_in_one_line` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1473 filtered out; finished in 0.60s` |

WINDOWS counters (`.planning/WINDOWS.md` frontmatter), recorded before and after `gsd-tools windows fixed 58` and `... fixed 63`:

| Counter | Before | After |
|---------|--------|-------|
| `open_count` | 15 | 13 |
| `waived_count` | 39 | 39 |
| `fixed_count` | 30 | 32 |
| `total_count` | 84 | 84 |

Only rows 58 and 63 changed (compared row by row against a copy taken before the edit); both gained an evidence pointer in the reason cell. Row 63 is closed on the boot warning that names each schedule with no recorded creator (`M-B-28`, `MIGRATION.md` 11.8); no backfill of `created_by` is claimed and operators still re-create those schedules through `POST /v1/schedules`.

## Review dispositions

One disposition per Phase 40 finding that Phase 57's sweep will read (the archived `40-REVIEW.md` and `40-REVIEW-FIX.md` are not edited, PD-07-3). Phase 40 commit hashes are quoted as cited in `40-REVIEW-FIX.md`; those objects are not present in this clone's history, so the Phase 53 commits are the ones that can be resolved here.

| Finding | Requirement | Disposition | Closed by (commit and test) |
|---------|-------------|-------------|-----------------------------|
| WR-01 (submit and fork onto another tenant's thread) | TENANT-04 | Closed. Phase 40 added the guard (`6d7efb16`); Phase 53 routes it through the one core rule `RunReadScope::permits_thread` and locks both routes over real HTTP. | `2b16763d0`, `66002e74b`; `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404`, `cross_tenant_fork_is_the_unknown_thread_404`; foreign-versus-missing fork body equality in `every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404` (`e466c2654`, `0502efa2f`, `260f4c5ee`). Accepted residual: an unused thread id stays distinguishable on submit (ADR-0063). |
| WR-02 (cancel isolation enforced only in the controller) | TENANT-05 | Closed. Phase 40 moved the check into `RunSubmissionService::cancel` (`fa054746`); Phase 53 re-ran the service-level and HTTP tests on the head (prove-then-lock, no new test needed). | `cancel_of_another_tenants_run_is_not_found_and_writes_nothing`, `cancel_admits_the_owning_tenant_and_admin`, `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` (see "TENANT-05 lock re-run on this plan's head (AC-08)" above; re-run on the 53-07 head: `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` printed `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1473 filtered out; finished in 0.12s`). |
| WR-03 (corrupt or half-attributed row breaks hidden equals missing) | TENANT-06 | Closed. Phase 40 made the body a fixed 500 and added the Postgres CHECK (`05612748`, `9afaef2b`); Phase 53 makes an unreadable run or Waypoint row the missing-object 404 for a tenant and the fixed 500 for an Admin, on every `/v1/runs/{run_id}*` and `/v1/threads/{id}*` route. Residuals: run `UnknownSchemaVersion` stays 500 (T-53-10); a corrupt latest-run row on submit stays the redacted 500 (T-53-05). | `30b532813`, `839a8119a` (runs); `e466c2654`, `0502efa2f`, `260f4c5ee` (threads); `corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal`, `serialization_on_every_run_id_route_is_hidden_for_tenant_and_redacted_for_admin`, `serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin`. |
| WR-04 (Serialization messages reach API clients) | TENANT-06 | Closed. Phase 40 added `internal_repo_error` (`05612748`); Phase 53 removes every remaining backend-text site in `thread_controller.rs` and renders `thread store error` and `run store error` only. | `0502efa2f`; the 53-03 verify regex `ApiError::internal\((e|other)\.to_string\(\)\)` matches nothing in `thread_controller.rs` (re-run in 53-07: 0 matches); `redact_store_error_for_scope_hides_only_hideable_errors_for_a_tenant`, `run_backend_error_stays_a_fixed_500_for_a_tenant`, `list_threads_skips_a_corrupt_thread_for_a_tenant` (asserts the injected text never reaches a body). |
| IN-01 (reserved literals accepted as tenant and key names) | TENANT-09 | Closed. `open-access`, `unattributed` and `anonymous` are refused at load as a key name, and `open-access` and `unattributed` as a key or bearer tenant (exact, case-sensitive). | `4de0bcbc2`; `validate_rejects_reserved_literals_in_every_position`, `validate_reports_the_first_reserved_offender`, `validate_accepts_case_variants_of_reserved_literals`, `reserved_key_name_matches_the_open_access_principal_id`. |
| IN-04 (job poll ignores principal and tenant) | TENANT-07 | Closed. A job is visible to its enqueuing `(agent id, tenant)` or an Admin; legacy unowned jobs are Admin-only. `get_job` is kept unrouted; its `#[deprecated]` attribute is deferred to the next minor (`MIGRATION.md` 11.7) because the patch semver gate rejects it. | `cc0077527`, `010ee3158`; `job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass`, `job_poll_is_scoped_to_the_enqueuing_agent_and_tenant`, `evicted_job_poll_is_404_for_every_role`. |

## Manual credential-handling review

Performed by reading each added line, per `.github/instructions/security.instructions.md` ("Manual review still matters"). CodeQL runs advisory-only in this repository and does not gate a merge, and no static taint analysis of first-party Rust gates a merge, so this manual review is the control for credential-handling code; the clean result below is the reviewer's reading, not a tool's.

The four checks are: **K** no API key interpolated; **C** no config type carrying a key is `Debug`-formatted or serialised outward; **R** no raw row text reaches a response body or is embedded by this phase's own code; **T** redaction precedes any truncation where text is bounded (not applicable when nothing is truncated).

| Plan | Message | Location | K | C | R | T |
|------|---------|----------|---|---|---|---|
| 53-02 | `log::error!("{context} failed: {detail} (hidden as not found)")` and `log::error!("{context} failed: {detail}")` in `redact_store_error_for_scope` | `crates/paladin-web/src/run_controller.rs:724`, `:727` | pass: `context` is a fixed route string, `detail` a store error | pass | see note 1 | n/a, nothing truncated |
| 53-02 | `RUN_STORE_ERROR_MESSAGE = "run store error"` (fixed `500` body; value unchanged) | `run_controller.rs:685`, used at `:753` | pass | pass | pass: constant | n/a |
| 53-02 | `unknown run '{run_id}'` / `unknown thread '{thread_id}'` (`404`, existing text) | `run_controller.rs:783`, `:794` | pass | pass | pass: echoes the caller's own path id | n/a |
| 53-03 | `THREAD_STORE_ERROR_MESSAGE = "thread store error"` (fixed `500` body) | `crates/paladin-web/src/thread_controller.rs:244`, used at `:700`, `:728`, `:822` | pass | pass | pass: constant | n/a |
| 53-03 | `thread_store_error` and `waypoint_error_for_scope` (route their detail into the 53-02 log line above) | `thread_controller.rs:694`, `:713` | pass | pass | see note 1 | n/a |
| 53-03 | `unknown thread '{thread}'` (`404`, hidden equals missing) | `thread_controller.rs:651`, `:870`, `:1432`, `:1449` | pass | pass | pass: echoes the caller's own path id | n/a |
| 53-04 | `thread API list owner lookup failed: run serialization error: {message} (thread skipped as not visible)` | `thread_controller.rs:810` | pass | pass | see note 1; log only, never a response body | n/a |
| 53-05 | `unknown job '{job_id}'` (`404`) | `crates/paladin-web/src/agent_controller.rs:963` (`get_job_scoped`) | pass | pass | pass: echoes the caller's own path id | n/a |
| 53-06 | `TREASURER_UNENFORCED_MESSAGE` (boot refusal, static text) | `src/infrastructure/web/run_api_wiring.rs:636`, used at `:649` | pass | pass | pass: constant | n/a |
| 53-06 | scan `warn!` (`N run schedule(s) have no recorded creator ...: <ids> -- re-create each via POST /v1/schedules ...`) | `run_api_wiring.rs:732` | pass | pass | pass: carries schedule ids and a count only | n/a |
| 53-06 | scan `error!` (`run schedule scan for missing creators stopped at an unreadable page (<position>) ...`) | `run_api_wiring.rs:747` | pass | pass | pass: carries a schedule id or `at start`, never the page error text (the page-error test asserts the negative) | n/a |
| 53-06 | reserved-literal validation errors for key `name`, key `tenant` and bearer `tenant` | `src/config/agents.rs:205`, `:224`, `:259` | pass: interpolates the entry name or index and the matched literal, never `entry.key` | pass | pass | n/a |

Cross-checks on the same reading:

- No `log::` call added by this phase formats a `Principal`, an `AuthConfig`, a key entry or a request body. `Principal` carries an id, a role and a tenant, never the key value.
- This phase adds no outbound HTTP client, no redirect behavior and no credential-bearing header, so the redirect and webhook checks in the instructions do not apply.
- Every response-body message in the table is a constant or echoes the caller's own path id; tests assert the injected backend text never appears in a response body on the run, thread-list and thread-route paths.

Note 1 (a limit stated plainly, not a clean pass). The three log lines that interpolate a store error write that error's `Display` text to the **server log**. For a `Serialization` error that text is a serde decode message, and serde can quote a short fragment of the offending value (for example `invalid type: string "..."`). This phase's code does not add row text of its own, and the fragment never reaches a response body. The same convention already applied before this phase (`internal_repo_error`, `run_controller.rs:693`, logs the error the same way), so this is not a new class of exposure, but a fragment of a stored row can reach the operator's log. If a Waypoint Battlefield or run input can hold a secret, an operator who ships these logs to a shared sink should treat a decode-error line as potentially sensitive. Truncation is not applied anywhere in these messages, so redaction-before-truncation does not arise. Phase 57's sweep should decide whether decode-error text needs the `crates/paladin-llm/src/redaction.rs` treatment.
