# ADR-0063: Threads and jobs are tenant-visible through the run-read rule

## Status

Accepted

**Date:** 2026-10-31

**Phase:** 53 (Tenant Isolation Closure), plan 53-07; written after plans 53-01 to 53-06 built what it records.

**Amends ADR-0054** (tenant-scoped run reads): ADR-0054 scoped the run routes and deferred the thread routes (Phase 40 D-14, WINDOWS row 58, T-40-23). This ADR extends the same rule, `RunReadScope`, to the thread routes and to the agent job poll, and adds one role-aware redaction rule for unreadable rows. It does not restate ADR-0054; rules 1 to 5 there stand.

## Context

A thread carries no tenant of its own. Before this phase every `/v1/threads/{id}*` route required authentication only, so an authenticated principal of any tenant could list every thread and read another tenant's Waypoint state and history, although that tenant's `/v1/runs` routes answered `404` (WINDOWS row 58). The agent job poll (`GET /v1/agents/{id}/jobs/{job_id}`) had the same shape: any authenticated caller holding a job id could read the result of a job another tenant enqueued (Phase 40 review finding IN-04).

Two further gaps sat beside these. First, an unreadable stored row (a run row that fails to deserialize, a Waypoint of an unsupported schema version) answered every role with the fixed `500`, which tells a tenant that a row exists at an id it cannot see (Phase 40 findings WR-03 and WR-04). Second, three boot-time conditions let isolation fail quietly: a Treasurer configured on a pool with no engine factory, schedules with no recorded creator, and API key or tenant names that collide with the `open-access`, `unattributed` and `anonymous` sentinels. Those three are covered by plan 53-06 and the migration notes, not by this ADR's rules, and are listed under Consequences only where they leave a residual.

Out of scope here: the audit G2 findings, and recording a tenant on the thread itself (deferred, below).

## Decision

1. **A thread belongs to the tenant of its latest run.** The latest run is the one with the newest `submitted_at`, with `run_id` as the tiebreak. `RunReadScope::permits_thread(&self, latest: Option<&Run>) -> bool` (`paladin-ai-core`) is the one rule: `All` permits every thread, a `Tenant` scope permits a thread only when its latest run is permitted by `permits`, and a thread with no run is visible to an Admin scope only. Reads of a zero-run thread are therefore Admin-only. Thread birth stays open: `ensure_thread_visible` calls `permits_thread` only with `Some(&latest)`, so a first `POST /v1/runs {thread_id}` or fork onto an id with no runs still works. A foreign tenant submitting or forking onto another tenant's thread gets the `unknown thread` `404`, with no run inserted and nothing enqueued (plan 53-01).

2. **Every `/v1/threads/{id}*` route enters one gate, `load_visible_thread`, and a hidden thread is the missing thread.** The six `{id}` operations (state, get, history, resume, fork, delete) call it first, after the `501` check and the id parse, and before any body-dependent work. On the two admin-only mutations (`resume`, `delete`) the gate runs before `require_admin`, so a foreign non-admin gets `404`, never a `403` that would confirm the thread exists; a same-tenant non-admin still gets `403`. An `All` scope returns from the gate before any run lookup. `GET /v1/threads/{id}/history` of an unknown thread is `404` for every role. The route set is enumerated from the router's own OpenAPI document and compared with a table, so a new ungated route fails a test (plan 53-03).

3. **No run store wired fails closed for a non-admin.** The owner of a thread cannot be established without the run repository, so `load_visible_thread` answers a non-admin with the missing-thread `404`, and `GET /v1/threads` answers a non-admin with the empty page `{"items":[],"next_cursor":null}`. An Admin is unaffected (plans 53-03 and 53-04).

4. **`GET /v1/threads` is a bounded over-fetch walk for a non-admin.** `visible_thread_page` reads the waypoint store in batches of `THREAD_SCAN_BATCH` (100) threads, looks up each thread's latest run with the `All` scope, and keeps the thread when `permits_thread` admits it. A request examines at most `MAX_EXAMINED_THREADS` (500) threads. It stops when `limit` threads are kept or the cap is reached, and returns a cursor taken from the last examined thread, so a filtered page may be shorter than `limit` while the next page always advances and never repeats an item; clients page until `next_cursor` is `null`. A tenant's run `Serialization` error during the owner lookup is logged at `error` and the thread is skipped, so one tenant's corrupt row cannot fail another tenant's list; any other lookup error is the fixed `run store error` `500`. An Admin keeps the existing single `limit + 1` fetch with no owner lookup (plan 53-04).

5. **A job is visible to its enqueuing agent id and tenant, or to an Admin.** `JobStore::create_owned` records the `(agent id, TenantId)` owner in private state, evicted together with the job, and `JobStore::get_visible` applies the scope. The routed handler is `agent_controller::get_job_scoped`, which carries `operation_id = "get_job"` so `openapi.json` is byte-identical. A job enqueued before this change has no recorded owner and is Admin-only. A poll under a different agent id, by a different tenant, or of an evicted job is the byte-identical `unknown job '<id>'` `404`. `get_job` is kept, unrouted, with its signature, body and `#[utoipa::path]` unchanged and its rustdoc saying it is superseded. It carries no `#[deprecated]` attribute: that attribute was measured to trip the patch-level `cargo semver-checks` lint `function_marked_deprecated`, so it is deferred to the next minor release and recorded in `MIGRATION.md` section 11.7 (plan 53-05).

6. **An unreadable row is hidden from a tenant and redacted for an Admin, through one rule.** `redact_store_error_for_scope` (`paladin-web`, `pub(crate)`) takes a `hideable` flag from a per-store classifier: for a `Tenant` scope a hideable error becomes the missing-object `404` built by the same constructor the missing and hidden branches use, and every other case is the fixed `500` with the detail only in the server log. The classifiers are `run_repo_error_for_scope` (hideable: run `Serialization` only) and `waypoint_error_for_scope` (hideable: waypoint `Serialization` and `SchemaVersionUnsupported`). The fixed messages are `run store error` and `thread store error` (`THREAD_STORE_ERROR_MESSAGE`); no thread route echoes backend text. `GET /v1/runs` keeps the fixed `500` for every role, and a `Backend` error stays the fixed `500` for a tenant, so the rule cannot become an existence oracle (plans 53-02 and 53-03).

## Consequences

- The thread and job routes now answer a tenant exactly as the run routes do: hidden equals missing, in status, headers and body bytes, proven by comparing a failing or foreign router's response with an empty router's response for the same id. `GET /v1/threads` and the job poll are behavioral changes recorded in `MIGRATION.md` section 11.1 (`M-B-23` to `M-B-26`); no `pub` signature changed and section 11.2 stays empty.
- A tenant hint in a header or query parameter changes nothing on any of these routes; the tenant is server-derived (ADR-0054 rule 1).
- A walk of `GET /v1/threads` is a keyset walk, not a snapshot, and its cursor is an exclusive timestamp: a thread updated between pages can move across the cursor. The cursor reveals only that some thread was updated at that instant (T-53-23, accepted).
- A tenant sees fewer than `limit` items on some pages, and an empty page with a non-null cursor when 500 foreign threads precede its first visible one. This is the price of filtering without a stored thread owner.

Accepted residuals, each with the reason it is not closed here:

- **Unused versus hidden thread id on submit (T-53-04).** Thread birth is open, so `POST /v1/runs {thread_id}` for an id with no runs succeeds while the same call on a foreign thread is `404`; a caller can tell an unused id from a hidden one on that route only. Thread ids are UUIDv7 by default. Tenant-namespaced thread ids, or recording a tenant on the thread at birth, is the deferred promote.
- **Corrupt latest-run row on submit (T-53-05).** `ensure_thread_visible` reads the latest run through the submission service, where a corrupt row reaches the redacted `500 "run store error"`. The role-dependent `404` is confined to the load helpers (`load_visible_run`, `load_visible_thread`), which fork also reaches through the controller gate.
- **Run `UnknownSchemaVersion` stays `500` (T-53-10).** After a downgrade, a row of a newer schema version answers a tenant `500` rather than `404`, because a version mismatch is an operator condition, not row corruption.
- **Examination cap.** The 500-thread cap with skip-and-log means a tenant whose threads sit behind more than 500 foreign threads needs several requests to reach them; a skipped corrupt thread is invisible to the tenant until an operator repairs the row.
- **Resume checks the parley port before `require_admin` (PD-03-2).** A non-admin on a deployment with no parley port gets `501`, not `403`.
- **Factory present but `with_treasurer` dropped (T-53-32, accepted under D-00f).** The boot assertion `assert_treasurer_enforced` refuses a configured Treasurer on a pool with no engine factory (`RunWorkerPool::has_engine_factory`), but a factory that is present while the wiring drops `with_treasurer` is not refused; code review covers it.
- **No-op engine factory (AC-N5).** `has_engine_factory` cannot tell what a factory builds, so a factory that builds nothing useful is undetectable; this stays a judgment check for review and `/gsd-secure-phase`.

Deferred, not decided here: recording a tenant on the thread (or tenant-namespaced thread ids), the `#[deprecated]` attribute on `get_job` in the next minor, a backfill or PATCH re-assignment of `created_by` for schedules created before Phase 41 (row 63 closes on the boot warning that names each unattributed schedule, not on a backfill), and Phase 57's sweep of the Phase 40 review findings, whose dispositions are in `53-TENANT-EVIDENCE.md`.

## Evidence

Plan 53-01 (`53-01-SUMMARY.md`): `permits_thread_admits_all_and_only_the_latest_runs_tenant`, `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404`, `cross_tenant_fork_is_the_unknown_thread_404`, with the existing `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` re-run on the head.

Plan 53-02: `redact_store_error_for_scope_hides_only_hideable_errors_for_a_tenant`, `corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal`, `serialization_on_every_run_id_route_is_hidden_for_tenant_and_redacted_for_admin`, `run_list_serialization_stays_a_fixed_500_for_every_role`, `run_backend_error_stays_a_fixed_500_for_a_tenant`, `unattributed_valid_run_is_hidden_from_tenant_and_visible_to_admin`.

Plan 53-03: `get_thread_is_visible_to_its_tenant_and_admin_and_hidden_from_others`, `every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404`, `zero_run_thread_is_visible_to_admin_only`, `mixed_tenant_thread_belongs_to_the_latest_runs_tenant`, `second_delete_of_a_thread_is_404_for_every_role`, `thread_routes_fail_closed_for_non_admin_without_a_run_store`, `serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin`, `tenant_hints_in_headers_or_query_change_nothing_on_thread_routes`, `post_resume_by_a_foreign_user_is_404_not_403`.

Plan 53-04: `list_threads_omits_foreign_and_zero_run_threads`, `list_threads_returns_the_empty_page_shape_when_nothing_is_visible`, `list_threads_short_pages_advance_the_cursor_without_repeats`, `list_threads_examination_cap_returns_a_short_page_with_a_cursor`, `list_threads_skips_a_corrupt_thread_for_a_tenant`, `list_threads_fails_closed_for_non_admin_without_a_run_store`, `list_threads_admin_path_performs_no_owner_lookup`, `list_threads_ignores_tenant_hints`.

Plan 53-05: `create_owned_records_the_agent_and_tenant`, `get_visible_scopes_a_job_to_its_agent_and_tenant`, `legacy_unowned_job_is_admin_only`, `evicted_owned_job_is_gone_for_every_scope`, `job_poll_is_scoped_to_the_enqueuing_agent_and_tenant`, `job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass`, `evicted_job_poll_is_404_for_every_role`, `job_poll_ignores_tenant_hints`, `unscoped_get_job_is_kept_for_embedders`; patch semver probe for `paladin-web`: 223 checks, 223 pass, 31 skip.

Plan 53-06 (the boot guards named in the residuals): `treasurer_assertion_refuses_a_configured_treasurer_without_an_engine_factory`, `pool_built_without_an_engine_factory_is_refused_with_a_treasurer`, `schedule_scan_reports_every_unattributed_schedule_ascending_in_one_line`, `validate_rejects_reserved_literals_in_every_position`.

The prove-then-lock record, the watched-red lines, the semver probe and the residual list are in `.planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md`. The binding semver run and the version decision belong to plan 53-08.
