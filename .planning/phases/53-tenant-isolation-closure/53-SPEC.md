# Phase 53: Tenant Isolation Closure — Specification

**Created:** 2026-10-30
**Ambiguity score:** 0.12 (gate: ≤ 0.20)
**Requirements:** 9 locked

## Goal

A principal of one tenant can no longer enumerate, read, run against or mutate another tenant's
threads, runs or jobs through any `/v1/threads/*`, `/v1/runs`, `/v1/runs/{id}/cancel` or
`/agents/{id}/jobs/{job_id}` path (a hidden object answers exactly like a missing one); a
configured Treasurer can no longer be silently dropped by worker-pool wiring; reserved principal
literals are refused at config load; unattributed legacy schedules are surfaced once at boot; and
the `v0.11.1`-versus-`v0.12.0` decision is recorded before Phase 58 opens.

## Background

Phase 40 (ADR-0054) made run reads tenant-scoped: `RunReadScope { All, Tenant }` is applied by
`RunQuery.scope` on lists and by `run_controller::load_visible_run` on every `/runs/{run_id}*`
route, with a route-matrix test (`published_run_id_operations`) built from the router's own
OpenAPI document. Threads were explicitly deferred (40-CONTEXT D-14, WINDOWS row 58): there is
no threads table — a thread exists only as `waypoints.thread_id` plus `runs.thread_id` — and
`Waypoint`, `ThreadSummary` and `WaypointPort` carry no tenant. Every `/v1/threads/*` handler in
`crates/paladin-web/src/thread_controller.rs` extracts the `Principal` and ignores it, so any
authenticated principal can list every thread and read any thread's Waypoint state and history.

Phase 40's review-fix (`40-REVIEW-FIX.md`, WR-01..WR-04) already landed: a private
`RunSubmissionService::ensure_thread_visible` derives a thread's tenant from its latest run and
refuses foreign `submit {thread_id}` and `fork` with `UnknownThread`; `cancel` applies the read
scope in the service before writing the cancel flag; `run_controller::internal_repo_error` renders
every repository failure as a fixed `500 "run store error"`. The carried-gaps inventory (built from
`40-REVIEW.md` alone) listed those as open, so TENANT-04/05/06 are prove-then-lock items. Still
open: `get_job` (`agent_controller.rs`) ignores both the principal and the `{id}` path segment
(IN-04); `AuthConfig::validate` accepts the sentinel literals `open-access`, `unattributed` and
`anonymous` as real names (IN-01); `build_run_api` (`run_api_wiring.rs`) always pairs
`with_engine_factory` and `with_treasurer` but nothing asserts the pairing (audit G2); schedules
from before Phase 41 have `NULL created_by` and fire ungated (WINDOWS row 63).

All public types involved (`Waypoint` in `paladin-ai-core`; `ThreadSummary`, `WaypointPort` in
`paladin-ports`; `JobRecord` in `paladin-web`) are all-public-field types without
`#[non_exhaustive]`, and Phase 52 armed the semver gate at `--release-type patch` against the
`0.11.0` baseline with `MIGRATION.md` §11.2 as the only register it reads. Recording a tenant on
any of them is therefore a registered break and a `v0.12.0`; deriving it from runs is additive.

## Requirements

1. **One thread visibility gate (TENANT-03)**: Every `/v1/threads/*` handler — list, get, state,
   history, resume, fork, delete — resolves the thread through one shared visibility gate whose
   rule is: visible if the caller's `RunReadScope` permits the thread's latest run (newest
   `created_at`, run id as tiebreak), or the caller is Admin; a thread with no run row is visible
   to Admin only.
   - Current: all seven handlers ignore the extracted `Principal`; `list_threads` returns every
     thread; `get_thread`/`get_thread_state`/`get_thread_history` read any thread's Waypoints.
   - Target: a non-admin principal of another tenant receives, on every `/v1/threads/{id}*`
     operation, the same `404` status, error code and body bytes a missing thread yields; `GET
     /v1/threads` omits threads the caller may not see (including zero-run threads for
     non-admin) and returns `{ "items": [], "next_cursor": null }` when nothing is visible; a
     filtered page may hold fewer than `limit` items while `next_cursor` still advances, and
     never repeats an item; a second `DELETE` of the same thread is `404` for every role.
   - Acceptance: a route-matrix test enumerates every `(METHOD, /v1/threads/{id}*)` operation
     from `versioned_thread_parts(..).1` (never a hand list), and for each proves `404` with the
     missing-thread body for a foreign tenant's thread and the documented success status for
     the owner and for Admin; a list test proves omission and the empty-page shape; adding a
     `/threads/{id}*` route that bypasses the gate fails the matrix test.

2. **Submit and fork refuse a foreign thread (TENANT-04, prove-then-lock)**: `POST /v1/runs
   {thread_id}` and `POST /v1/threads/{id}/fork` refuse a thread whose latest run belongs to
   another tenant with the missing-thread `404` (`RunSubmissionError::UnknownThread`).
   - Current: implemented by `ensure_thread_visible` (WR-01, commit `6d7efb16`) with
     contract-style tests in `http_surface_tests.rs`; not yet re-verified in this milestone.
   - Target: behavior unchanged and locked: refused for a user-role principal of another tenant;
     accepted for the owning tenant, for Admin, for a principal-less internal caller, and for a
     fresh thread id with no runs (thread birth — the known limitation that an unused id is
     distinguishable from a hidden one is recorded in the gate's rustdoc).
   - Acceptance: an HTTP-level regression test per route proves the four outcomes above; a
     verifier confirms the test exists and passes on the phase head.

3. **Service-layer cancel isolation (TENANT-05, prove-then-lock)**:
   `RunSubmissionService::cancel` refuses a run the principal's scope does not permit with
   `NotFound { run_id }` — identical to a missing run — before role authorization and before
   `request_cancel` writes the durable cancel flag.
   - Current: implemented in WR-02 (commit `fa054746`) with service tests.
   - Target: unchanged and locked; a library caller with no HTTP controller in the path cannot
     cancel a foreign tenant's run, and `cancel_requested` stays `false`.
   - Acceptance: a service-level test (no router) proves `NotFound` for a foreign tenant and
     `cancel_requested == false` afterwards; owner and Admin still cancel.

4. **Redacted run-store errors with hidden-equals-missing (TENANT-06)**: A corrupt or
   half-attributed run row, and any `RunRepositoryError::Serialization` on a run route, is
   rendered as the missing-run `404` to a non-admin principal and as the fixed redacted `500`
   (body exactly the constant `"run store error"`) to an Admin; no backend text reaches any
   response body.
   - Current: `internal_repo_error` returns the fixed `500` to every caller (WR-03/WR-04), so a
     non-admin can tell a corrupt row exists; Postgres migration 009 prevents half-attributed
     rows at write time, SQLite rejects them on read.
   - Target: non-admin → `404` byte-identical to missing; Admin → fixed `500`; detail only in the
     server log; an unattributed-but-valid run stays `404` for non-admin and `200` for Admin
     (ADR-0054 rule 3).
   - Acceptance: tests inject a `Serialization` error from the repository on `GET /runs/{id}`,
     `GET /runs/{id}/stream`, `GET /runs/{id}/webhook-deliveries`, `POST /runs/{id}/cancel` and
     the thread routes that load runs, and assert the status per role and that no body contains
     the injected message text.

5. **Tenant-scoped job polling (TENANT-07)**: `GET /agents/{id}/jobs/{job_id}` returns a job
   only to a principal of the tenant that enqueued it, under the agent id that enqueued it, or
   to an Admin; any other combination answers the same `404` an unknown job id yields.
   - Current: `get_job` ignores the principal and the `{id}` segment; any authenticated
     principal holding a job id reads its result.
   - Target: the job's owning `(agent id, tenant)` is recorded at `enqueue_job` time; a foreign
     tenant, or the right tenant under the wrong agent id, gets `404 unknown job`; Admin reads
     any job; an evicted job stays `404`.
   - Acceptance: tests for owner-tenant/same-agent (200), second key of the same tenant (200),
     foreign tenant (404), owner tenant under another agent id (404), Admin (200), with the
     foreign-tenant body byte-identical to the unknown-job body.

6. **Treasurer wiring is asserted at boot (TENANT-08)**: When a Treasurer is configured
   (`treasurer.allowance` has entries), `paladin-server` refuses to start, before binding any
   listener, unless the `RunWorkerPool` was built with an engine factory — the only path that
   attaches settlement, the spend guard and the cancel probe.
   - Current: `build_run_api` always calls `with_engine_factory` and `with_treasurer`; nothing
     checks the pairing, so a future wiring change drops enforcement silently (audit G2).
   - Target: boot exits non-zero with an error that names the misconfiguration (Treasurer
     configured, worker pool has no engine factory); a Treasurer-less deployment is unaffected.
   - Acceptance: one test builds the pool with a Treasurer and no engine factory and asserts the
     refusal error; one test proves the shipped wiring (factory present) passes; a verifier
     confirms the check runs before the HTTP listener is bound.

7. **Reserved literals rejected at config load (TENANT-09)**: `AuthConfig::validate` rejects
   the exact, case-sensitive literals `open-access` and `unattributed` as an API key `tenant` or
   `name`, and `anonymous` as an API key `name` (and `open-access`/`unattributed` as
   `bearer_token.tenant`), with a message naming the entry and never the key value.
   - Current: any valid identifier is accepted, so a real principal can collide with the
     open-access sentinel or satisfy `LedgerScope::is_unattributed()`.
   - Target: the first offending entry, in declaration order, is reported as
     `http.auth.api_keys[<name>]`/`[#<index>]` or `http.auth.bearer_token` and the config is
     rejected; existing name/tenant/duplicate validation is unchanged; `Open-Access` and other
     case variants are not rejected.
   - Acceptance: table tests cover each literal in each position (rejected) and a case variant
     (accepted); the error string contains the entry name and not the secret.

8. **One boot warning for unattributed schedules (TENANT-10)**: Server boot emits exactly one
   `warn!` line when one or more `run_schedules` rows have `NULL created_by`, listing the count,
   every schedule id in ascending `schedule_id` order, and the operator action (re-create via
   `POST /v1/schedules`); it emits no such line when there are none or when the schedule store
   is disabled. No backfill or `PATCH` re-assignment route is added.
   - Current: such schedules fire unattributed and ungated with no operator signal (WINDOWS row
     63).
   - Target: the scan pages the schedule store at boot and is advisory: a row the adapter cannot
     read (half-attributed → `Serialization`) is logged at `error!` with the schedule id when
     known and never the row text, and boot continues.
   - Acceptance: tests with zero, one and several unattributed schedules assert zero or exactly
     one `warn!` with the exact count and id list; a half-attributed row yields one `error!` and a
     successful boot; the store is byte-identical before and after the scan.

9. **Version decision recorded (success criterion 5)**: Before Phase 58 opens, the phase records
   the `v0.11.1`-versus-`v0.12.0` decision in `53-VERSION-DECISION.md` (phase directory) and
   STATE.md: either every change is additive — semver job green at `--release-type patch`
   against `0.11.0`, `MIGRATION.md` §11.2 empty, allowlist at zero entries — or the one
   deliberate break is named in a §11.2 row and a mirrored allowlist entry in the same commit
   and the milestone version becomes `v0.12.0`.
   - Current: §11.2 is empty; no decision recorded.
   - Target: a dated decision with the semver job's `N checks: X pass, 0 fail` line (N > 0) for
     all twelve packages, or the registered break. HTTP behavior changes forced by this phase
     (`404` where `200` was for foreign tenants) are recorded as behavioral changes in §11.1 and
     §11.6, not as §11.2 breaks.
   - Acceptance: the decision file exists with one of the two outcomes and its evidence;
     `make check-migration-allowlist` is green; the ROADMAP/STATE version strings agree with it.

## Boundaries

**In scope:**
- One shared thread visibility gate and its OpenAPI-derived route matrix for all seven
  `/v1/threads/*` operations (R1).
- Regression tests locking the already-landed submit/fork and cancel isolation (R2, R3).
- Role-dependent redaction of corrupt-row and `Serialization` errors on run and thread routes (R4).
- `(agent, tenant)` ownership on jobs and a tenant-scoped `get_job` (R5).
- Boot-time Treasurer-wiring assertion with refusal and tests (R6).
- Reserved-literal rejection in `AuthConfig::validate` (R7).
- One boot-time `warn!` for `NULL created_by` schedules, advisory scan (R8).
- The recorded version decision, with §11.1/§11.6 behavioral entries (R9).
- `WINDOWS.md` rows 58 and 63 closed on this phase's evidence; Phase 40 WR-01..04 and IN-01/IN-04
  dispositioned.

**Out of scope:**
- Any new HTTP route, port or adapter — milestone scope guard; the fixes route existing handlers
  through the gate.
- Backfill or `PATCH` re-assignment of `created_by` (TENANT-11) — deferred by milestone decision
  A7; only if a dogfood deployment has such rows.
- Tenant-namespaced thread ids (closing the "unused id vs hidden id" distinction on submit) —
  deferred thread-tenancy work (40-CONTEXT D-14); recorded as a known limitation.
- Making schedule-fired runs inherit a creator, allowance enforcement changes, aggregate spend
  routes — Phase 41/42 territory and inventory §H accepted contracts.
- Resolve-then-connect SSRF pinning, SQL-injection review of the tenant predicate — canon
  security items owned by `/gsd-secure-phase` and the manual credential review.
- The lockstep version bump and the tag — Phase 58.
- How the gate derives or stores the tenant, where the gate helper lives, how list filtering is
  implemented — `/gsd-discuss-phase 53` decisions (the checkpoint already holds the pending
  question).

## Constraints

- Semver: Phase 52 D-01/D-03 — every public-API change is judged at `--release-type patch`
  against the `0.11.0` crates.io baseline; a deliberate break needs a `MIGRATION.md` §11.2 row and
  a mirrored `.cargo/semver-checks-allowlist.toml` entry in the same commit and flips the
  milestone to `v0.12.0` (R9). A break not forced by a security fix is reshaped or dropped.
- Dependency direction: core → nothing; ports → core; adapters → core + ports. The gate rule
  lives where `RunReadScope` lives; `paladin-web` never imports infrastructure.
- Repo gates on every commit: 82 % workspace line coverage, `make clean-code`, `make check-gates`,
  `make api-surface` (+ `api-surface-update` and a CHANGELOG entry for an intentional surface
  change), `make security`, `make check-migration-allowlist`, the manual credential-handling
  review in `.github/instructions/security.instructions.md`. No `unwrap`/`expect`/`panic!` in
  library code.
- Hidden equals missing (ADR-0054 rule 5): same status, code, body bytes and headers; never a
  `403` for an existence question.
- The tenant is server-derived only (ADR-0054 rule 1): no request header, query or body field
  may name a tenant.
- Shipped `MIGRATION.md` §9/§10 and `CHANGELOG.md` `[0.11.0]` are byte-frozen; new ADRs take
  number 0063 onward from `.planning/decisions/PROMOTION.md`.
- Logging uses the existing `log` facade; no `tracing` dependency is added.

## Acceptance Criteria

- [ ] AC-01 (R1): For every `(METHOD, path)` under `/v1/threads/{id}` read from the thread router's
      own OpenAPI document, a foreign tenant's bearer gets `404` with a body byte-identical to the
      missing-thread body; owner and Admin get the documented success status.
- [ ] AC-02 (R1): `GET /v1/threads` as tenant B omits tenant A's threads and zero-run threads and
      returns `{ "items": [], "next_cursor": null }` when nothing is visible; a filtered page may be
      shorter than `limit` with a non-null `next_cursor`; no item is repeated across pages.
- [ ] AC-03 (R1): A thread with no run row is `404`/omitted for a non-admin and visible to Admin.
- [ ] AC-04 (R1): A legacy thread with runs from two tenants is visible to the latest run's tenant
      and to Admin only.
- [ ] AC-05 (R1): A second `DELETE /v1/threads/{id}` on an already-deleted thread is `404` for Admin.
- [ ] AC-06 (R1): Adding a `/threads/{id}*` route to `thread_openapi_router` without the gate makes
      the matrix test fail.
- [ ] AC-07 (R2): `POST /v1/runs {thread_id}` and `POST /v1/threads/{id}/fork` against another
      tenant's thread return `404`; owner, Admin, principal-less caller and fresh thread id succeed.
- [ ] AC-08 (R3): `RunSubmissionService::cancel` with a foreign tenant's `PrincipalRef` returns
      `NotFound` and leaves `cancel_requested == false`, with no router in the test.
- [ ] AC-09 (R4): A `Serialization` error from `get` renders `404` (missing-run body) for a non-admin
      and `500` with body exactly `"run store error"` for Admin on every run route and on thread
      routes that load runs.
- [ ] AC-10 (R4): No response body on any run or thread route contains the injected repository
      error text.
- [ ] AC-11 (R4): An unattributed valid run is `404` for a non-admin and `200` for Admin.
- [ ] AC-12 (R5): `GET /agents/{id}/jobs/{job_id}`: owner tenant same agent `200`; second key of
      the owner tenant `200`; foreign tenant `404`; owner tenant under another agent id `404`;
      Admin `200`; the two `404` bodies equal the unknown-job body.
- [ ] AC-13 (R6): With `treasurer.allowance` configured and a pool built without an engine
      factory, boot returns an error naming the misconfiguration before any listener binds.
- [ ] AC-14 (R6): The shipped `build_run_api` wiring with a Treasurer boots (factory present).
- [ ] AC-15 (R7): `open-access`/`unattributed` as a key `tenant` or `name`, `anonymous` as a key
      `name`, and `open-access`/`unattributed` as `bearer_token.tenant` are each rejected with a
      message naming the entry and not the key value.
- [ ] AC-16 (R7): `Open-Access`, `Anonymous` and other case variants are accepted; all pre-existing
      `validate` checks still pass their existing tests.
- [ ] AC-17 (R8): Zero unattributed schedules, or schedule store disabled → no `warn!`; one → exactly
      one `warn!` with count 1 and that id; several → exactly one `warn!` with the exact count and
      every id ascending, plus the re-create instruction.
- [ ] AC-18 (R8): A half-attributed schedule row produces one `error!` (id when known, never row
      text) and boot succeeds; no schedule row changes during the scan.
- [ ] AC-19 (R9): `53-VERSION-DECISION.md` exists before Phase 58 with either the green patch-level
      semver line for all twelve packages and an empty §11.2, or a §11.2 row + allowlist entry and
      `v0.12.0`; `make check-migration-allowlist` is green.
- [ ] AC-20 (R9): Foreign-tenant `200 → 404` changes are recorded in `MIGRATION.md` §11.1 and §11.6.
- [ ] AC-N1 (P1, must NOT): no route in scope returns `403`, a distinct error code, a different body
      or an extra header for a hidden object versus a missing one.
- [ ] AC-N2 (P2, must NOT): no boot error, `warn!`, `error!` or validation message contains an API
      key value or raw database row text.
- [ ] AC-N3 (P3, must NOT): no handler or gate reads a tenant from a request header, query or body.
- [ ] AC-N4 (P4, must NOT): the `NULL created_by` scan performs no insert, update or delete.
- [ ] AC-N5 (P5, must NOT, judgment): no configuration key or no-op factory lets a configured
      Treasurer boot without the engine-factory path.

## Edge Coverage

**Coverage:** 24/24 applicable edges resolved · 0 unresolved

| Category | Requirement | Status | Resolution / Reason |
|----------|-------------|--------|---------------------|
| adjacency | R1 | ✅ covered | AC-04: mixed-tenant legacy thread belongs to the latest run's tenant |
| empty | R1 | ✅ covered | AC-02, AC-03: zero-run thread hidden; empty page shape fixed |
| ordering | R1 | ✅ covered | AC-02: newest-activity order kept; short pages with advancing cursor; no duplicates |
| idempotency | R1 | ✅ covered | AC-05: second DELETE is 404 |
| concurrency | R1 | ⛔ dismissed | Read-only gate; a run submitted between check and read is at most one request stale (ADR-0054 keyset backstop); mutating paths re-check inside the service |
| unclassified | R2 | ✅ covered | AC-07: fresh thread id accepted (thread birth); known limitation recorded |
| unclassified | R3 | ✅ covered | AC-08: hidden cancel leaves `cancel_requested` false |
| empty | R4 | ✅ covered | AC-11: unattributed valid run is 404 non-admin / 200 Admin |
| encoding | R4 | ✅ covered | AC-09: redacted body is the byte-exact ASCII constant |
| idempotency | R4 | ⛔ dismissed | Stateless read path: the answer depends only on the row and the principal |
| concurrency | R4 | ⛔ dismissed | Stateless read path; no shared mutable state in the error mapping |
| unclassified | R5 | ✅ covered | AC-12: agent-id mismatch is 404; evicted job stays 404 |
| unclassified | R6 | ✅ covered | AC-13, AC-14: refusal case and shipped-wiring pass case |
| empty | R7 | ✅ covered | AC-16: pre-existing empty/invalid-name checks unchanged |
| encoding | R7 | ✅ covered | AC-15, AC-16: exact, case-sensitive match; no trimming or normalization |
| concurrency | R7 | ⛔ dismissed | Single-threaded boot-time config validation |
| boundary | R8 | ✅ covered | AC-17: zero → no line; one → one line with count 1 |
| adjacency | R8 | ✅ covered | AC-18: half-attributed row → one `error!`, boot continues |
| empty | R8 | ✅ covered | AC-17: none, or store disabled → no line |
| ordering | R8 | ✅ covered | AC-17: ids ascending by `schedule_id` |
| precision | R8 | ⛔ dismissed | A count of listed ids; no arithmetic, no cap, no rounding |
| adjacency | R9 | ✅ covered | AC-20: behavioral HTTP changes go to §11.1/§11.6, not §11.2 |
| empty | R9 | ✅ covered | AC-19: zero breaks → empty §11.2, zero allowlist entries, `v0.11.1` |
| ordering | R9 | ✅ covered | AC-19: recorded before Phase 58 opens, in `53-VERSION-DECISION.md` and STATE.md |

## Prohibitions (must-NOT)

**Coverage:** 5/5 applicable prohibitions resolved · 0 unresolved

| Prohibition (must-NOT statement) | Requirement | Status | Verification / Reason |
|----------------------------------|-------------|--------|------------------------|
| MUST NOT let a hidden thread, run or job be distinguishable from a missing one by status, error code, body bytes or headers (no `403`, no distinct code) | R1, R4, R5 | resolved | test — AC-N1; descriptor to be captured by plan-phase (negative assertions inside the route-matrix tests) |
| MUST NOT print an API key value or raw database row text in any boot error, `warn!`, `error!` or validation message | R6, R7, R8 | resolved | test — AC-N2 |
| MUST NOT read a tenant from any request header, query parameter or body (tenant is server-derived, ADR-0054 rule 1) | R1, R5 | resolved | test — AC-N3 |
| MUST NOT write during the `NULL created_by` scan: no backfill, re-assignment, disable or delete of any schedule row | R8 | resolved | test — AC-N4 |
| MUST NOT provide a configuration switch or no-op factory escape hatch that lets a configured Treasurer boot unenforced | R6 | resolved | judgment — AC-N5, reviewed in code review and `/gsd-secure-phase` |

Canon referrals (not minted here): SQL injection on the tenant predicate and SSRF/DNS rebinding
are canon security items owned by `/gsd-secure-phase` and the manual credential-handling review.

## Ambiguity Report

| Dimension          | Score | Min  | Status | Notes                                                        |
|--------------------|-------|------|--------|--------------------------------------------------------------|
| Goal Clarity       | 0.90  | 0.75 | ✓      | Nine falsifiable requirements mapped to TENANT-03..10 + SC5  |
| Boundary Clarity   | 0.85  | 0.70 | ✓      | Explicit out-of-scope list; HOW deferred to discuss-phase    |
| Constraint Clarity | 0.85  | 0.65 | ✓      | Patch-level semver gate, hidden==missing, server-derived tenant |
| Acceptance Criteria| 0.90  | 0.70 | ✓      | 20 positive + 5 negative pass/fail criteria                  |
| **Ambiguity**      | 0.12  | ≤0.20| ✓      |                                                              |

Status: ✓ = met minimum, ⚠ = below minimum (planner treats as assumption)

## Interview Log

| Round | Perspective               | Question summary                                          | Decision locked                                                                 |
|-------|---------------------------|-----------------------------------------------------------|---------------------------------------------------------------------------------|
| 0     | Researcher (scout)        | What exists today?                                        | WR-01..04 present in tree; no threads table; public types not `non_exhaustive` |
| 1     | Researcher                | Corrupt row / Serialization: what does a client observe?  | 404 for non-admin, fixed redacted 500 for Admin                                |
| 1     | Researcher                | Thread with waypoints but no run row, non-admin view?     | Hidden: 404 / omitted; Admin-only (mirrors ADR-0054 rule 3)                    |
| 1     | Researcher                | Job visibility unit?                                      | Tenant with Admin bypass; foreign tenant gets unknown-job 404                  |
| 2     | Boundary Keeper / Failure | What must boot refuse on (TENANT-08)?                     | Treasurer configured AND no engine factory → named error, before listener bind |
| 2     | Boundary Keeper           | Which reserved literals, where?                           | `open-access`, `unattributed` (tenant+key), `anonymous` (key); exact, case-sensitive; `AuthConfig::validate` |
| 2     | Failure Analyst           | Shape of the `NULL created_by` warn?                      | One line: count + every id + operator action; none when zero or store disabled |
| 5.5   | Edge probe                | Mixed-tenant legacy thread owner?                         | Latest run's tenant                                                             |
| 5.5   | Edge probe                | Job under mismatched agent id?                            | 404; job scoped to (agent, tenant)                                              |
| 5.5   | Edge probe                | Half-attributed schedule row during the scan?             | `error!`, keep booting; scan is advisory                                        |
| 5.5   | Edge probe                | Remaining 21 edges                                        | 16 specified as AC lines, 5 dismissed with reasons                              |
| 5.6   | Prohibition probe         | Five bespoke must-NOTs                                    | All accepted: P1–P4 test tier, P5 judgment tier                                 |

---

*Phase: 53-tenant-isolation-closure*
*Spec created: 2026-10-30*
*Next step: /gsd-discuss-phase 53 — implementation decisions (ownership derivation vs recording, gate location, list filtering, job-owner storage, assertion placement)*
