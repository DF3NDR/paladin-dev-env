# Phase 53: Tenant Isolation Closure - Research

**Researched:** 2026-10-31
**Domain:** Rust / axum authorization gates, hexagonal ports, boot-time wiring assertions, semver-gated public surface (single workspace, no new external dependencies)
**Confidence:** HIGH (every finding below was read from the tree this session; the one tool-verified external fact is the cargo-semver-checks lint list)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried forward (locked by ADR-0054, Phase 52, the milestone open and the SPEC - not re-asked)**

- **D-00a:** Gates as Phase 52 D-00a: 82 % workspace line coverage (ADR-0006), `make clean-code`, `make check-gates`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional public-surface change), `make security`, `make check-migration-allowlist`, the manual credential-handling review in `.github/instructions/security.instructions.md`. No `unwrap`/`expect`/`panic!` in library code. TDD: every behavior below is watched failing first.
- **D-00b:** Semver as Phase 52 D-01/D-03: every public change is judged at `--release-type patch` against the `0.11.0` baseline for all twelve published packages; a deliberate break needs a `MIGRATION.md` §11.2 row and a mirrored `.cargo/semver-checks-allowlist.toml` entry in the same commit and flips the milestone to `v0.12.0`. A break not forced by the security fix is reshaped or dropped (roadmap scope note).
- **D-00c:** ADR-0054 governs the rule: tenant is server-derived only; `RunReadScope { All, Tenant }` is the one value type; hidden equals missing (same status, code, body bytes, headers, never `403` for an existence question); Admin bypass; a `Tenant` scope never permits an unattributed run; a route matrix is read from the router's own OpenAPI document.
- **D-00d:** Phase 40's WR-01..WR-04 fixes are present in the tree (verified 2026-10-30): `RunSubmissionService::ensure_thread_visible` (submit/fork), the service-layer scope check in `cancel`, `internal_repo_error` + `RUN_STORE_ERROR_MESSAGE` in `run_controller.rs`. TENANT-04, -05 and -06 are prove-then-lock: the plan starts by running the SPEC's acceptance checks against the tree and adds only the missing regression tests; only what fails is rebuilt.
- **D-00e:** New ADRs take the next free number from `.planning/decisions/PROMOTION.md` (currently **0063**) and advance that line in the same commit. `MIGRATION.md` §9/§10 and `CHANGELOG.md` `[0.11.0]` are byte-frozen. Logging stays on the `log` facade.
- **D-00f:** Observable contracts fixed by `53-SPEC.md` and not revisited here: corrupt row → `404` non-admin / fixed `500` Admin; zero-run thread Admin-only; job scoped to (agent, tenant) with Admin bypass; boot refusal = Treasurer configured AND no engine factory; reserved set `open-access`, `unattributed` (tenant + key name), `anonymous` (key name), exact and case-sensitive; one `warn!` with count + every id ascending, advisory scan that never blocks boot.

**Thread ownership source and the version call (TENANT-03, SC5)**

- **D-01:** **Derive the thread's tenant from its runs; do not record it.** A thread's owner is the attribution (`submitted_by`) of its latest run (newest `created_at`, run id as tiebreak), looked up with the unrestricted `All` scope as an internal authorization read; a thread with no run row has no owner and is Admin-only. This is the rule `ensure_thread_visible` already applies on submit and fork, so one rule serves reads and writes. No column is added to `waypoints`, no threads table is created, and `Waypoint`, `ThreadSummary` and `WaypointPort` are untouched - every change is additive and the milestone stays `v0.11.1`. (Reversible.)
- **D-02:** **Version decision: additive, `v0.11.1`, proven by the semver job.** The phase ends with `53-VERSION-DECISION.md` recording the green `Semver Checks (vs v0.11.0)` job line (`N checks: X pass, 0 fail`, N > 0) for all twelve packages on the phase PR head, an empty §11.2 and a zero-entry allowlist. If the job flags any change, the change is reshaped to additive (new inherent methods, new functions, new private state) rather than registered; a break is registered only if no additive shape exists for a security fix, and then the file names it and the milestone becomes `v0.12.0`. Foreign-tenant `200 → 404` changes are entered in `MIGRATION.md` §11.1 and §11.6 as behavioral changes, never as §11.2 rows.
- **D-03:** **The ownership rule lives in core, next to `RunReadScope`.** One additive inherent method on `RunReadScope` (name is the planner's - e.g. `permits_thread_owner(latest_run: Option<&Run>) -> bool`) expresses "latest run permitted, or `All`; no run → `All` only". `RunSubmissionService::ensure_thread_visible` is refactored to call it and the thread gate (D-05) calls it, so the rule is written exactly once. Adding an inherent method is not a semver break. (Reversible.)
- **D-04:** **`GET /v1/threads` filters by bounded over-fetch, not a new port method.** The handler walks `WaypointPort::list_threads(Some(page), before)` pages, resolves each thread's latest run with `RunQuery { thread_id, limit: 1, scope: All }` (the same lookup D-01 uses), keeps the threads the caller's scope permits, and stops when `limit` visible items are collected or the store is exhausted; `next_cursor` is the `last_updated_at` of the last *examined* thread so the keyset walk never repeats an item, and a page may be shorter than `limit` with a non-null cursor. Admin (`All`) skips every lookup. The N+1 cost is accepted for this cleanup phase; a runs-side listing (`RunRepositoryPort` change) is a break and is not taken. (Reversible.)

**Gate shape and run-less threads (TENANT-03)**

- **D-05:** **One `load_visible_thread` helper in `thread_controller.rs`, mirroring `load_visible_run`.** Signature shape: `(state: &ThreadApiState, principal: &Principal, thread: &ThreadId) -> Result<Waypoint, ApiError>`; it loads the latest Waypoint, resolves the latest run through `state.runs`, applies D-03, and returns the byte-identical `404 unknown thread '{id}'` for missing and hidden alike. Every `/threads/{id}*` handler enters through it; `list_threads` uses the D-04 walk with the same rule. The service half (`ensure_thread_visible`) stays where it is for submit/fork.
- **D-06:** **Gate before `require_admin` on `resume` and `delete`.** A foreign non-admin gets `404` (hidden equals missing, SC1 names resume and delete explicitly); a same-tenant non-admin then gets the existing `403`; Admin passes both. Missing thread → `404` for every role.
- **D-07:** **No run repository wired ⇒ fail closed for non-admin.** When `ThreadApiState.runs` is `None` (run store disabled) ownership cannot be established, so a non-admin principal sees no thread (`404` / empty list) and Admin is unaffected. Documented in the module docs that replace the stale "PLAT-06 pending" paragraph.
- **D-08:** **Route matrix = `published_run_id_operations` pattern.** The thread test module gains a `published_thread_id_operations(&OpenApi)` reader over `versioned_thread_parts(..).1` for every `(METHOD, /v1/threads/{id}*)` operation and a table of expected success statuses per operation; it sends each operation as the owner (success), as a foreign user-role key (`404`, body equal to a never-existing id's body) and as Admin (success); a separate test proves list omission and the empty-page shape. Adding a route without joining the table fails the test.

**Corrupt rows and Serialization errors (TENANT-06)**

- **D-09:** **Role-dependent mapping inside `load_visible_run` and `load_visible_thread` only.** `RunRepositoryError::Serialization` (and the SQLite read-time half-attribution rejection, which surfaces as `Serialization`) maps to `ApiError::not_found` for a `Tenant` scope and to `internal_repo_error` (fixed `500`) for `All`; every other repository error stays the fixed `500` for every role (an outage is not an existence signal). List routes keep the fixed `500` for all roles - there is no single object to hide. Thread routes that load runs (D-05) use the same mapping.
- **D-10:** Tests inject `Serialization` through the existing failing-repository test double in `run_controller.rs` (the `Some(message)` seam) and assert status per role and that no body contains the injected text, on `GET /runs/{id}`, `/stream`, `/webhook-deliveries`, `POST /cancel` and the thread routes.

**Jobs, boot assertion, small guards (TENANT-07, -08, -09, -10)**

- **D-11:** **Job owner lives in `JobStore`'s private state, not on `JobRecord`.** `Inner` gains a per-job owner `(agent_id, TenantId)` (or `Option<RunAttribution>` + agent id); additive methods `JobStore::create_owned(agent_id, &Principal)` and `JobStore::get_visible(job_id, agent_id, &RunReadScope) -> Option<JobRecord>` are added, the existing `create`/`get` stay for compatibility. `enqueue_job` records the owner; `get_job` reads through `get_visible` and answers `404 unknown job '{id}'` for a hidden, agent-mismatched, evicted or unknown job alike. `JobRecord` is unchanged, so `paladin-web`'s public surface does not break. (Reversible.)
- **D-12:** **Treasurer assertion = one pure check called from `build_run_api`.** An additive `RunWorkerPool::has_engine_factory(&self) -> bool` (or equivalent inspector) plus a pure function in `run_api_wiring.rs` (`assert_treasurer_enforced(treasurer_configured: bool, has_engine_factory: bool) -> Result<(), String>`) that returns the named error "treasurer configured but the run worker pool has no engine factory; settlement, spend guard and cancel probe would not attach". `build_run_api` calls it right after the pool is built and before `spawn`, so the server's `main` fails before any listener binds. Tests: the pure function refuses (configured + no factory) and passes (configured + factory, or unconfigured); an integration test proves the shipped wiring boots with a Treasurer.
- **D-13:** **Reserved literals as one exported constant.** `principal.rs` (paladin-ai-core) gains `pub const RESERVED_PRINCIPAL_LITERALS: &[&str] = &["open-access", "unattributed"]` beside `TenantId::OPEN_ACCESS`, and the key-name-only reservation of `"anonymous"` is a second constant; `AuthConfig::validate` checks `tenant`, `name` and `bearer_token.tenant` against them with the existing message shape (`http.auth.api_keys[{name}]: 'tenant' is the reserved literal 'open-access' …`), never printing the key. `TenantId::new` is unchanged.
- **D-14:** **`NULL created_by` scan in `run_api_wiring`, after the schedule store opens.** Only when the schedule store exists: page `RunScheduleRepositoryPort::list(100, cursor)` to exhaustion, collect ids whose `created_by` is `None`, emit exactly one `log::warn!` (count, ids ascending, "re-create via POST /v1/schedules"); on a page error emit one `log::error!` with the cursor position and stop the scan - boot continues. No port change, no write.
- **D-15:** **ADR-0063 amends ADR-0054.** A short ADR records the thread rule (D-01/D-03), the fail-closed no-run-store rule (D-07), the job rule (D-11) and the role-dependent corrupt-row mapping (D-09), and advances the PROMOTION line to 0064.

### Claude's Discretion

- Exact names of the new inherent methods and the wiring assertion function (D-03, D-11, D-12).
- Page size for the D-04 walk and whether the D-04 lookup is shared as a private helper with `active_run_id_for`.
- The wording of the module-doc rewrite in `thread_controller.rs` (the PLAT-06 paragraph is stale).
- Evidence file naming beyond `53-VERSION-DECISION.md` (one `53-TENANT-EVIDENCE.md` for the prove-then-lock results of D-00d is acceptable).
- Whether WINDOWS rows 58 and 63 are closed in this phase's final plan or handed to Phase 57 with the evidence pointer (either is acceptable; the row text must cite the matrix test and the warn test).

### Deferred Ideas (OUT OF SCOPE)

- **Record the tenant on the thread** (a `tenant_id` column or threads table plus public fields on `Waypoint`/`ThreadSummary`) - a registered break; candidate for the next minor if the N+1 list cost (D-04) matters in a dogfood deployment.
- **Repository-side thread listing by scope** (`RunRepositoryPort`/`WaypointPort` method) - same reason, same horizon.
- **Tenant-namespaced thread ids** to close the "unused id vs hidden id" distinction on submit - 40-CONTEXT D-14 deferral, unchanged.
- **Backfill / `PATCH` re-assignment of `created_by`** (TENANT-11) - only if a dogfood deployment has such rows.
- Reviewed todos not folded: `2026-08-13-verify-local-coverage-reproduction.md` (Phase 56), `2026-10-13-review-content-data-orchestration-base-items.md` (Phase 55/G3), `2026-10-14-web3sec-single-bump-to-v0-11-0.md` (downstream consumer), `2026-10-20-openai-responses-api-for-pro-tier.md` (ruled out at milestone open).

Locked requirements, boundaries and the 25 acceptance criteria (AC-01..AC-20, AC-N1..AC-N5) live in `53-SPEC.md` and are not restated here; this file researches HOW.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TENANT-03 | Every `/v1/threads/*` handler resolves the thread through one visibility gate | Patterns 1-3; Pitfalls 2, 3, 4, 5, 6, 8 |
| TENANT-04 | `POST /v1/runs {thread_id}` and `POST /v1/threads/{id}/fork` refuse a foreign thread | Prove-then-lock table (service tests exist; HTTP-level tests missing); Pattern 1 and Pitfall 2 (no-run semantics must not change) |
| TENANT-05 | `RunSubmissionService::cancel` enforces tenant at the service | Prove-then-lock table (service AND HTTP tests exist - AC-08 already satisfied; lock with evidence only) |
| TENANT-06 | Corrupt row / `Serialization` redacted, hidden equals missing | Pattern 4, Pitfall 7 (thread routes leak backend text at seven sites) |
| TENANT-07 | `get_job` checks principal and tenant | **Pitfall 1 (signature change is a semver break)**, Pattern 5 |
| TENANT-08 | Boot refuses Treasurer-without-engine-factory | Pattern 6, Pitfall 9 |
| TENANT-09 | Reserved literals rejected at config load | Pattern 7 |
| TENANT-10 | One boot `warn!` for `NULL created_by` schedules | Pattern 8, Pitfalls 10, 11 |
</phase_requirements>

## Summary

The whole phase is in-repo Rust work against code that was read line by line this session. No new
crate, port or route is needed, and no external package is installed (Package Legitimacy Audit: none
required). The design in CONTEXT.md D-01..D-15 is sound and implementable. This research found **one
decision that cannot be implemented as written without breaking the semver gate (D-11's `get_job`)**,
**one rule in D-03 that, applied literally, would regress submit/fork (thread birth)**, and **six
gaps the decisions do not mention** that the planner must schedule: the thread routes leak backend
error text at seven sites, `GET /threads/{id}/history` currently answers `200 []` for an unknown
thread, several existing tests assert the pre-gate behavior and must be re-seeded, the run ordering
key is `submitted_at` (the SPEC/CONTEXT say `created_at`), the per-thread lookup in the list walk can
turn one tenant's corrupt row into everyone's `500`, and the list walk as specified is not actually
bounded.

TENANT-04, -05 and -06 are prove-then-lock as D-00d says. Measured against the tree: TENANT-05 is
fully covered (service test `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` plus the
HTTP test `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag`) so only evidence is needed;
TENANT-04 has service-level tests for every outcome the SPEC lists but **no HTTP-level test per
route**, which AC-07 requires; TENANT-06's existing tests only exercise an Admin principal (which
stays `500`), so the non-admin `404` half is new behavior, not a lock.

The environment cannot currently compile the root `paladin-ai` crate (disk is 100% full: `No space
left on device` during `cargo test --lib --features web-server`); `paladin-web` lib tests do run
(34 `thread_controller` tests pass offline). The planner must account for this in verification (see
Environment Availability).

**Primary recommendation:** Build in this order - (1) core rule + service refactor keeping the
no-run-open submit/fork behavior, (2) a shared `pub(crate)` role-aware error mapper in
`run_controller.rs` reused by `load_visible_run` and a new `load_visible_thread`, (3) thread gate,
list walk (with an examination cap) and the matrix, (4) jobs via an **additive `get_job_scoped`
handler plus deprecation of `get_job`** (or record the break and go to `v0.12.0`; the planner must
resolve Open Question 1 first), (5) wiring assertion, config reserved literals, schedule scan,
(6) docs/ADR/MIGRATION/WINDOWS, and last (7) `53-VERSION-DECISION.md` from the CI semver line.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Thread ownership rule ("latest run permitted, or All; no run -> All only") | Core domain (`RunReadScope`, `paladin-core`) | - | Pure value rule, no I/O; core is where the existing run rule lives (ADR-0054 rule 4) |
| Thread read/mutate visibility gate (7 routes) | API / Backend (`paladin-web` `thread_controller`) | Application service (`ensure_thread_visible` for submit/fork writes) | Mirrors `load_visible_run`; paladin-web never imports infrastructure |
| Role-dependent corrupt-row redaction | API / Backend (`paladin-web`) | - | Only the controller knows the caller's scope when rendering the error (D-09) |
| Job ownership `(agent, tenant)` | API / Backend (`JobStore` private state) | - | Jobs are ephemeral and in-memory in `paladin-web`; no port involved |
| Treasurer-wiring assertion | Infrastructure wiring (`run_api_wiring.rs`) | Application (`RunWorkerPool` inspector) | `build_run_api` is the one place pool + Treasurer + submission meet |
| Reserved principal literals | Config validation (`src/config/agents.rs`) | Core (constants beside `TenantId::OPEN_ACCESS`) | Boot-time, fail-closed, before any listener |
| `NULL created_by` schedule scan | Infrastructure wiring (`run_api_wiring.rs`) | Port (`RunScheduleRepositoryPort::list`, read-only) | Advisory boot scan, no port change, no write |
| Version decision record | Process / docs (`53-VERSION-DECISION.md`, `MIGRATION.md` §11) | CI (`Semver Checks (vs v0.11.0)`) | Evidence comes from the CI job on the PR head |

## Standard Stack

No new libraries. Everything uses what the workspace already depends on. [VERIFIED: codebase grep of `crates/paladin-web/Cargo.toml` and root `Cargo.toml`]

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8.4 | Handlers, extractors, `Extension<Principal>` | Already the HTTP layer of `paladin-web` |
| utoipa / utoipa-axum | 5 / 0.2 | `#[utoipa::path]` + `OpenApiRouter`; the matrix reads its document | The run matrix test already reads `versioned_run_parts(..).1` this way |
| log | 0.4.21 | `warn!`/`error!` for the schedule scan and redacted-error logging | CLAUDE.md/CONTEXT D-00e: stay on the `log` facade, no `tracing` |
| thiserror, async-trait | workspace | Existing error enums and port traits | Existing conventions |
| tower (`util`) + http-body-util | 0.5 / 0.1 (dev-deps) | `Router::oneshot` HTTP-level tests | Already used by the thread and run tests |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| cargo-semver-checks | 0.50.0 (installed) | Local reproduction of the CI semver job (`make check-semver`) | After any public item is touched; needs crates.io and a build (not runnable while the disk is full) |
| cargo-audit / cargo-deny | 0.22.2 / 0.20.2 (installed) | `make security` | Per commit gate |
| cargo-llvm-cov | not installed | 82 % coverage gate | CI only locally; add new code with tests so the floor holds |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Derive owner from runs (D-01) | Record a tenant column on waypoints | Registered break -> `v0.12.0`; explicitly deferred |
| Per-item lookup walk (D-04) | New `RunRepositoryPort`/`WaypointPort` method | Break; explicitly deferred |
| New additive `get_job_scoped` handler (recommended for F1) | Route-layer middleware that gates `get_job` unchanged | Middleware keeps one handler name but splits the check from the handler and needs path-param extraction in the layer; more moving parts |
| New additive `get_job_scoped` handler | Change `get_job` signature and take `v0.12.0` | Honest and simple, but flips the milestone version for a job-polling guard; D-02 says reshape first |

**Installation:** none (no new dependencies).

**Version verification:** no package added, so no registry check applies. The only tool fact verified was the lint list: `cargo semver-checks --list` (0.50.0) shows `function_parameter_count_changed  major`. [VERIFIED: cargo-semver-checks 0.50.0 `--list`]

## Package Legitimacy Audit

No external packages are installed or added by this phase. **Packages removed due to [SLOP] verdict:** none. **Packages flagged as suspicious [SUS]:** none. The `package-legitimacy` seam was not run because there is nothing to check.

## Architecture Patterns

### System Architecture Diagram

```
                       HTTP request (X-API-Key / Bearer)
                                   |
                     require_authentication (route_layer)
                      inserts Principal{id, role, tenant}
                      (auth disabled -> open-access Admin)
                                   |
        +--------------------------+-----------------------------+------------------+
        |                          |                             |                  |
 /v1/threads/* (7 ops)     /v1/runs/{id}* (4 ops)       POST /v1/runs, fork     /agents/{id}/jobs/{job}
        |                          |                             |                  |
 load_visible_thread       load_visible_run              RunSubmissionService   JobStore::get_visible
  1 waypoints.latest()      repo.get(run_id)              ensure_thread_visible   (agent, tenant) owner
  2 scope==All? skip        scope.permits(run)            (latest run, All scope)  Admin bypass
  3 runs.list(thread,1,All) |                             no-run => OPEN (birth)   |
  4 scope.permits_thread()  |                             |                        |
        |                   +--- shared role-aware error mapper (pub(crate)) ---+    |
        |                       Serialization: Tenant->404 hidden, All->500 fixed     |
        |                       other repo error: fixed 500 for all roles             |
        v                          v                             v                  v
   200 / 404 (hidden==missing)  200 / 404                  202 / 404 UnknownThread  200 / 404 unknown job
        |
   resume/delete: gate FIRST, then require_admin  (foreign 404, same-tenant user 403, Admin ok)
   list: walk list_threads pages -> per-thread lookup -> keep permitted -> cursor = last examined

 Boot (run_api_wiring::build_run_api_with_cadence)
   AuthConfig::validate (reserved literals) --> run store opened --> [schedule scan: ONE warn!/error!]
   --> Treasurer built --> pool built --> assert_treasurer_enforced(...)  --> pool.spawn --> ... --> main binds listener
```

### Recommended Project Structure
```
crates/paladin-core/src/platform/container/principal.rs   # RunReadScope::permits_thread (+ doctest), RESERVED_* consts
crates/paladin-web/src/run_controller.rs                   # pub(crate) role-aware error mapper; load_visible_run uses it
crates/paladin-web/src/thread_controller.rs                # load_visible_thread, list walk, redaction helper, matrix tests
crates/paladin-web/src/job_store.rs                        # owner map in Inner, create_owned, get_visible
crates/paladin-web/src/agent_controller.rs                 # enqueue_job -> create_owned; get_job_scoped route
src/application/services/run/submission.rs                 # ensure_thread_visible calls the core rule (no-run stays open)
src/application/services/run/worker.rs                     # RunWorkerPool::has_engine_factory (additive)
src/infrastructure/web/run_api_wiring.rs                   # assert_treasurer_enforced, schedule scan
src/config/agents.rs                                       # AuthConfig::validate reserved-literal rows + table tests
.planning/decisions/0063-*.md  .planning/decisions/PROMOTION.md   # ADR + line advance to 0064
MIGRATION.md §11.1/§11.6/(§11.7)  crates/*/CHANGELOG.md  .project/current-exports.txt
.planning/phases/53-.../53-VERSION-DECISION.md  53-TENANT-EVIDENCE.md
```

### Pattern 1: The core thread rule - written once, no-run policy stays with the caller
**What:** Add one inherent method on `RunReadScope`. It answers only "may this scope see a thread whose latest run is `latest`". The *no-run* branch is a caller policy, because the two callers legitimately differ: the read gate hides a zero-run thread from non-admin (D-00f), while submit/fork must keep admitting a fresh thread id (thread birth, AC-07).
**When to use:** `load_visible_thread`, the list walk, and `ensure_thread_visible`.
**Example:**
```rust
// crates/paladin-core/src/platform/container/principal.rs  (additive inherent method)
impl RunReadScope {
    /// Whether this scope may see a thread whose newest run is `latest`.
    /// `All` sees every thread; a `Tenant` scope sees a thread only when its
    /// latest run is attributed to that tenant. A thread with NO run has no
    /// owner: only `All` sees it. (Callers that must admit a brand-new thread
    /// id - submit/fork - decide that themselves before asking.)
    pub fn permits_thread(&self, latest: Option<&Run>) -> bool {
        match (self, latest) {
            (Self::All, _) => true,
            (Self::Tenant(_), None) => false,
            (Self::Tenant(_), Some(run)) => self.permits(run),
        }
    }
}

// src/application/services/run/submission.rs  - ensure_thread_visible keeps thread birth open
match page.items.into_iter().next() {
    None => Ok(()),                                              // thread birth: unchanged
    Some(latest) if !scope.permits_thread(Some(&latest)) => Err(UnknownThread { .. }),
    Some(_) => Ok(()),
}
```
Source: derived from `RunReadScope::permits` and `ensure_thread_visible` in the tree. [VERIFIED: codebase read]

### Pattern 2: `load_visible_thread` (mirrors `load_visible_run`)
**What:** One private helper every `/threads/{id}*` handler enters through, after `parse_thread_id` and the 501 backend check.
**Sequence:** `waypoints.latest(&thread)` (missing -> 404) -> if `scope == All` return the Waypoint -> if `state.runs` is `None` -> 404 (D-07 fail closed) -> `runs.list(RunQuery{thread_id: Some(thread), limit: 1, ..Default::default()})` -> `scope.permits_thread(items.first())` else 404.
**Rules:**
- The 404 text must be the same string the missing case uses, `format!("unknown thread '{id}'")`.
- For `All`, skip the run lookup entirely (Admin cannot hit a run-store `Serialization` here, which keeps the Admin thread routes independent of run-row health).
- `RunQuery`'s `scope` defaults to `All`, which is the required internal-authorization read; do not set it to the caller's scope.
- Run ordering is `submitted_at DESC, run_id DESC` in every adapter [VERIFIED: `crates/paladin-storage/src/run/sqlite.rs:583`, `postgres.rs:388`]; `limit: 1` therefore returns the "latest run". The SPEC/CONTEXT call this `created_at`; `Run` has no `created_at` (its field is `submitted_at`), so plans and docs must say `submitted_at`.

### Pattern 3: Thread list walk with a hard cap
**What:** D-04 as written stops only on "limit visible or store exhausted", which is unbounded for a tenant that owns few threads in a store with many foreign ones (every request walks the whole store with one query per thread). Add a constant cap on *examined* threads per request (recommended `MAX_EXAMINED = 500`, batch size 100, both module constants - the planner's discretion per CONTEXT). When the cap is hit the page is returned short with `next_cursor = last examined last_updated_at`, which AC-02 explicitly allows.
**Algorithm:**
```
visible = []; examined = 0; cursor = before
if scope == All: keep today's single-fetch limit+1 path unchanged (no lookups)
loop:
  batch = list_threads(Some(BATCH), cursor)              # exclusive `before` cursor
  if batch.is_empty(): next_cursor = None; break
  for s in batch:
      examined += 1; last_examined = s.last_updated_at
      if visible_to(scope, s): visible.push(s)
      if visible.len() == limit or examined >= MAX_EXAMINED:
          next_cursor = Some(last_examined); break outer   # may be short, may be non-null
  if batch.len() < BATCH: next_cursor = None; break         # store exhausted
  cursor = Some(last_examined)
```
**Notes:** the exclusive-timestamp cursor means two threads with an identical `last_updated_at` straddling a page boundary can be skipped. That is a pre-existing property of `WaypointPort::list_threads` [VERIFIED: port docs, `before` is an exclusive `DateTime`], not introduced here; the rustdoc on the route already calls the walk a keyset, not a snapshot.

### Pattern 4: One shared role-aware error mapper (TENANT-06)
**What:** `internal_repo_error` is private to `run_controller.rs`. Make a `pub(crate)` mapper there and reuse it from `thread_controller.rs` so the redaction rule exists once.
```rust
// run_controller.rs
pub(crate) fn repo_error_for_scope(
    scope: &RunReadScope,
    context: &str,
    err: RunRepositoryError,
    hidden: impl FnOnce() -> ApiError,        // the missing-object 404 for THIS route
) -> ApiError {
    match (&err, scope) {
        (RunRepositoryError::Serialization { .. }, RunReadScope::Tenant(_)) => {
            log::error!("run API {context} failed (hidden as not found): {err}");
            hidden()
        }
        _ => internal_repo_error(context, err),   // fixed 500, logs detail
    }
}
```
`load_visible_run` calls it with `hidden = || ApiError::not_found(format!("unknown run '{run_id}'"))`; `load_visible_thread` with the thread 404. The `All` arm and every non-`Serialization` error stay the fixed `500 "run store error"`. Always `log::error!` the detail in both branches so an operator still sees the corrupt row.
Extension recommended here (consistent with D-09's intent, not in the locked text): apply the same hide-for-`Tenant` rule to `WaypointError::Serialization` and `SchemaVersionUnsupported` from `waypoints.latest()` in `load_visible_thread`, because before `latest()` returns the owner is unknown and a distinct `500` would reveal that a corrupt thread exists under that id.

### Pattern 5: Jobs - owner in `JobStore` private state (D-11) and the handler problem
`Inner` gains `owners: HashMap<String, (String /*agent id*/, TenantId)>`, evicted together with the job in the existing eviction loop. Add `create_owned(&self, agent_id: &str, principal: &Principal) -> String` and `get_visible(&self, id: &str, agent_id: &str, scope: &RunReadScope) -> Option<JobRecord>`: `All` -> record regardless of agent or tenant; `Tenant(t)` -> record only when the stored owner is exactly `(agent_id, t)`; a job created through the legacy `create()` has no owner and is visible to `All` only (fail closed, same shape as an unattributed run). `enqueue_job` switches to `create_owned(&id, &principal)`; `agent_id` is the path `id` after `registry.get(&id)` succeeded, so it is a registered agent id.
**The handler:** see Pitfall 1. `get_job` must read the `Principal`, which a new extractor parameter would add, and that is a semver break.

### Pattern 6: Treasurer assertion (TENANT-08)
```rust
// run_api_wiring.rs
fn assert_treasurer_enforced(treasurer_configured: bool, has_engine_factory: bool)
    -> Result<(), String>
{
    if treasurer_configured && !has_engine_factory {
        return Err("treasurer configured but the run worker pool has no engine factory; \
                    settlement, spend guard and cancel probe would not attach".to_string());
    }
    Ok(())
}
// in build_run_api_with_cadence, after `let mut pool = ... ; pool = pool.with_treasurer(..)` and
// BEFORE `Arc::clone(&pool).spawn(..)`:
assert_treasurer_enforced(treasurer.is_some(), pool.has_engine_factory())?;
```
Hardening within the locked contract (optional, additive): also expose `RunWorkerPool::has_treasurer()` and assert the pool actually holds the Treasurer, since removing `pool.with_treasurer(..)` while keeping the factory also silently drops the spend guard (the guard attaches only when `pool.treasurer` is `Some`). To make AC-13 meaningful the refusal test should build a real `RunWorkerPool` without `with_engine_factory`, take `pool.has_engine_factory()`, and pass it to the pure function (or a tiny wrapper taking `&RunWorkerPool`). AC-N5 is a judgment check: do not add a config key or a default no-op factory; note that `has_engine_factory()` cannot tell a real factory from a no-op closure, so code review (and `/gsd-secure-phase`) is the control, as the SPEC says.

### Pattern 7: Reserved literals (TENANT-09)
Add in `principal.rs`: `pub const RESERVED_PRINCIPAL_LITERALS: &[&str] = &[TenantId::OPEN_ACCESS, LedgerScope::UNATTRIBUTED];` (reference the two existing consts so the literals have one source; `treasury_ledger.rs` is in the same crate) and a second const for the key-name-only `"anonymous"`. `paladin_web::agent_auth::OPEN_ACCESS_PRINCIPAL_ID` is the same literal but lives in `paladin-web`, which core cannot import, so add a root-crate test asserting equality to stop drift. In `AuthConfig::validate`, insert the checks right after the existing `TenantId::new` check of the same field so every pre-existing message and ordering is unchanged:
- `name` in `RESERVED_PRINCIPAL_LITERALS` or equal to `"anonymous"` -> `http.auth.api_keys[{name}]: 'name' is the reserved literal '{name}' ...` (the name is a safe label here).
- `tenant` in `RESERVED_PRINCIPAL_LITERALS` -> `http.auth.api_keys[{name}]: 'tenant' is the reserved literal '{tenant}' ...`.
- `bearer_token.tenant` in `RESERVED_PRINCIPAL_LITERALS` -> `http.auth.bearer_token: 'tenant' is the reserved literal ...`, inside the existing `bearer_token.enabled` branch (the existing test `validate_ignores_the_bearer_tenant_when_bearer_is_disabled` keeps passing; rejecting a disabled section is a different behavior and not asked for).
Match exactly and case-sensitively, with no trimming. No existing config, doc or test uses these literals as tenant or key name [VERIFIED: grep of yml/yaml/md/rs/json outside `.planning`/`.project`/`target`].

### Pattern 8: Schedule scan (TENANT-10)
Split into a testable pure-ish core and a thin emitter so "exactly one `warn!`" is proven without depending on a process-global logger:
```rust
enum ScanOutcome { Clean, Unattributed(Vec<RunScheduleId>), PageError { after: Option<RunScheduleId> } }
async fn scan_unattributed_schedules(repo: &dyn RunScheduleRepositoryPort) -> ScanOutcome { /* list(100, cursor) loop */ }
fn report_scan(outcome: &ScanOutcome) { /* Clean => nothing; Unattributed => ONE warn!; PageError => ONE error! */ }
```
Collect ids from `item.created_by.is_none()`; `list` already returns ascending `schedule_id`, so ids stay ascending across pages; still `sort()` defensively before formatting. Call it in `build_run_api_with_cadence` right after the repository quartet is built (the run store is enabled there; the `Disabled` arm returns earlier, which is the "store disabled -> no line" case). The scan performs only `list` calls (AC-N4); assert in a test with a wrapper repository that records every method called.
For end-to-end emission use the capturing-logger pattern already in the repo (`src/bin/paladin-server.rs` tests, `paladin-storage/src/cadence/test_logger.rs`); `log::set_logger` is once per process and tests run on parallel threads, so match records by a unique marker (the test's own schedule ids) rather than counting all warns.

### Anti-Patterns to Avoid
- **Copying the no-run branch into `ensure_thread_visible` as "All only":** it would 404 every first-time `POST /v1/runs {thread_id: "my-new-thread"}` from a non-admin, breaking thread birth (AC-07, and the existing service test `submit_thread_tenant_guard_admits_owner_admin_internal_and_fresh_threads`).
- **Gating after `require_admin`:** a foreign non-admin would see `403` and learn the thread exists; D-06 says gate first.
- **Tenant from anywhere but `Principal`:** the gate takes `principal.read_scope()`; never accept a tenant from header, query or body (AC-N3).
- **`ApiError::internal(e.to_string())` on any in-scope route:** echoes backend text (Pitfall 7).
- **Logging `{err}` of a schedule `Serialization`/`Backend` error at boot:** use a fixed message and the cursor position only (AC-N2).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Route enumeration for the matrix | A hand-maintained route list | `published_run_id_operations` pattern over `versioned_thread_parts(..).1` | The point of the test is that a new route fails it; copy the reader and `assert_eq!(published, tabled)` |
| Hidden == missing body check | Comparing against a fixed string | Replace the hidden id with the missing id in the hidden body, then `assert_eq!` to the missing body (as `every_run_id_route_hides_foreign_runs_behind_the_missing_run_404` does) | The 404 message embeds the path id |
| Per-tenant list filtering | A SQL predicate or new port method | Per-thread `RunQuery{limit:1}` lookup (D-04) | A port change is a registered break |
| Log capture | A new logger crate | The existing `CapturingLogger` pattern | `log` facade only, one logger per process |
| Cursor encoding | New cursor format | `encode_cursor`/`decode_cursor::<DateTime<Utc>>` already in `list_threads` | Keeps the wire format unchanged |
| Admin/tenant scope | Comparing `role`/`tenant` by hand in handlers | `Principal::read_scope()` -> `RunReadScope` | The one value type (ADR-0054 rule 4) |

**Key insight:** the rule is small; the risk is entirely in (a) leaving a route or error path outside it and (b) changing public signatures. Both are what the matrix test and the semver job exist to catch.

## Runtime State Inventory

Not a rename/refactor/migration phase (no identifier is renamed, no data migration). One narrow runtime-state note: the `NULL created_by` schedule rows (WINDOWS row 63) are existing stored data that this phase only **reports**; no migration, backfill or write is performed. Other categories: stored data - no schema change (no tenant column, per D-01); live service config - none; OS-registered state - none; secrets/env vars - none; build artifacts - none.

## Common Pitfalls

### Pitfall 1: `get_job` signature change is a semver break
**What goes wrong:** `get_job` is `pub async fn` in a `pub mod agent_controller` of published `paladin-web`; adding `Extension<Principal>` changes its parameter count. `function_parameter_count_changed` is a **major** lint [VERIFIED: `cargo semver-checks --list`, 0.50.0], so the CI job fails and D-02's "empty §11.2, zero allowlist" outcome is impossible as D-11 is written. D-11 reasons only about `JobRecord`.
**How to avoid:** see Open Question 1. Recommended additive shape: add `pub async fn get_job_scoped(State, Extension<Principal>, Path<(String,String)>)` (own `#[utoipa::path]`, same path/operation id semantics), route it in `agent_openapi_router`, mark the old `get_job` `#[deprecated(note = "unscoped; use get_job_scoped")]` and log a §11.7 entry. A parameter *type* swap that keeps the count at 2 would dodge the lint but is still a real caller break - do not use it to turn the job green.
**Warning signs:** semver job red on `paladin-web` naming `get_job`; existing tests `poll_job`/`get_job_unknown_returns_404` call `get_job(State, Path)` directly and must move to the new handler.

### Pitfall 2: Thread birth regression from D-03
**What goes wrong:** a literal "no run -> All only" inside `ensure_thread_visible` breaks first submits to a fresh `thread_id`. **How to avoid:** Pattern 1 - keep `None -> Ok(())` in the service; only the read gate treats no-run as hidden.

### Pitfall 3: `history` of an unknown thread changes from `200 []` to `404`
**What goes wrong:** `get_thread_history` today never 404s (it returns `{items: [], next_cursor: null}` for any id). With the gate in front, a missing thread is `404` for every role including Admin and the owner (D-05: "byte-identical 404 for missing and hidden alike"). The path's utoipa responses also have no `404` entry. **How to avoid:** add the `404` response to the annotation, record the change in `MIGRATION.md` §11.6 as a behavioral change, and re-seed existing tests. `get_thread_history_paginates_with_limit_and_cursor` seeds only history (`seed_history`), not `latest`, so it would start failing; seed a latest Waypoint (and, for non-admin cases, a run).

### Pitfall 4: Existing tests that assert pre-gate behavior
- `post_resume_with_non_admin_role_is_403` (user key, unseeded thread): becomes `404` under D-06. Re-seed the thread and an owned run so same-tenant non-admin still gets `403`.
- `delete_thread_non_admin_is_403`, `get_thread_state_with_non_admin_role_is_200_not_403`: check each; the latter already expects `404` for an unknown thread.
- `MockRunRepository.list` in the thread tests ignores its query and returns an empty page, and `MockWaypointStore.list_threads` returns `vec![]`: the matrix and walk tests need new doubles (a run repo that honors `thread_id`, can hold attributed runs, and can inject `Serialization`; a waypoint store whose `list_threads` honors `limit`/`before`). `run_controller.rs`'s `MockRepository` already has the `fail_reads_with` seam but is private to that test module.

### Pitfall 5: Hidden-body comparison and id substrings
**What goes wrong:** the thread 404 message embeds the raw path id, so hidden-vs-missing equality only holds after swapping ids, and `str::replace` can mangle the body if one id is a substring of the other. **How to avoid:** use ids with no containment relationship (e.g. `acme-thread-1` vs `no-such-9f3a`), and also assert status, `error.code` and headers equal, not just body (AC-N1 names status, code, body bytes and headers).

### Pitfall 6: Auth-disabled deployments and test principals
With `http.auth.enabled = false` every request is the `open-access` Admin, so everything resolves `All`; the matrix test must use auth-enabled API keys (as the run matrix does). Also note the D-07 consequence for operators: a deployment with a waypoint store but the run store disabled makes all thread routes `404` for non-admin principals - record it in §11.1.

### Pitfall 7: Thread routes leak backend text today (F4)
`thread_controller.rs` has seven sites that render `ApiError::internal(e.to_string())` from a `WaypointError` or `RunRepositoryError` (`map_parley_error` fallback line 620; state 667; history 805; `list_threads` 890; `get_thread` 956; delete 1088 and 1101). `WaypointError::Serialization(String)` and `Backend{source}` Display text reaches the client. AC-10 requires "no response body on any run or thread route contains the injected repository error text" and success criterion 3 says "any `Serialization` error". Replace all of them with a thread-side fixed-message helper (log the detail, return a constant such as `"thread store error"`), including the `ParleyError` fallthrough. This is not in D-09/D-10 as worded; schedule it explicitly.

### Pitfall 8: One tenant's corrupt row must not break everyone's list
In the list walk, a per-thread `runs.list` returning `Serialization` would, under D-09's "list routes keep the fixed 500", fail the whole page for any non-admin whose walk touches a foreign tenant's corrupt thread. Recommended (consistent with D-09's purpose): inside the walk, treat `Serialization` for a `Tenant` scope as "not visible" (log, skip), keep the fixed `500` for other repository errors. Also note SQLite's `list` maps `limit + 1` rows before popping the extra one [VERIFIED: `sqlite.rs:575-600`], so a corrupt *second-newest* row also makes the `limit: 1` latest-run lookup return `Serialization`.

### Pitfall 9: The assertion must run before spawn and be provable
`pool.spawn(..)` starts worker tasks; place the call before it. Because shipped wiring always supplies a factory, a test that only boots `build_run_api` never exercises the refusal; AC-13 needs the unit-level refusal (pool without a factory) and AC-14 the shipped-wiring pass (an existing test `build_run_api_wires_a_treasurer_only_when_allowances_are_configured` already builds with a Treasurer; extend or reuse it).

### Pitfall 10: The scan sees page-level failures, not row-level
`RunScheduleRepositoryPort::list` maps every row of a page before returning, so one half-attributed row fails the whole page of up to 100 schedules, and the error carries no schedule id. The scan therefore cannot name the bad row or report the unattributed ids on that page, and (per D-14) stops. Log one `error!` with the cursor position (the last id of the previous page, or "start") and a fixed message, never `{err}`. Document that unattributed schedules after the failing page are not reported.

### Pitfall 11: Scan scope vs `schedules.enabled`
The repository exists whenever the run store is enabled, even when `schedules.enabled: false` (tick loop not spawned, so those rows do not fire). D-14 says "only when the schedule store exists", which this research reads as "run store enabled". If the planner prefers not to warn when nothing fires, gate on `configs.schedules.enabled` instead; see Open Question 3.

### Pitfall 12: Public-surface bookkeeping
`.project/current-exports.txt` covers the root `paladin` lib (it lists `RunWorkerPool`), so `RunWorkerPool::has_engine_factory` needs `make api-surface-update` plus a root `CHANGELOG.md` entry; new `paladin-web`/`paladin-ai-core`/`paladin-ai` items need entries in `crates/paladin-web/CHANGELOG.md` etc. `[0.11.0]` sections are byte-frozen: add under a new unreleased heading, not inside `[0.11.0]`. `MIGRATION.md` §9/§10 are frozen too.

## Code Examples

### Route matrix reader for threads (copy of the run pattern)
```rust
// Source: crates/paladin-web/src/run_controller.rs (published_run_id_operations) - same shape
fn published_thread_id_operations(api: &utoipa::openapi::OpenApi) -> Vec<(String, String)> {
    let mut ops = Vec::new();
    for (path, item) in &api.paths.paths {
        if !path.starts_with("/v1/threads/{id}") { continue; }
        for (m, present) in [("GET", item.get.is_some()), ("PUT", item.put.is_some()),
            ("POST", item.post.is_some()), ("DELETE", item.delete.is_some()),
            ("OPTIONS", item.options.is_some()), ("HEAD", item.head.is_some()),
            ("PATCH", item.patch.is_some()), ("TRACE", item.trace.is_some())] {
            if present { ops.push((m.to_string(), path.clone())); }
        }
    }
    ops.sort(); ops
}
// Table (owner success statuses): GET /v1/threads/{id} 200, GET .../state 200, GET .../history 200,
// POST .../resume 202 (needs Admin: owner-as-user gets 403 - see below), POST .../fork 202,
// DELETE /v1/threads/{id} 204.
```
Matrix caveat: `resume` and `delete` are Admin-only after the gate (D-06), so the "owner" column for those two rows is an Admin of the owning tenant, and a separate assertion covers same-tenant user -> `403` and foreign user -> `404`. The routes in scope today are exactly seven operations under six paths (`/threads` list is not under `{id}` and is covered by the list test). `DELETE` mutates, so give each (method, role) iteration its own seeded thread or re-seed between iterations; a second DELETE is `404` for every role (AC-05).

### `get_visible` ownership check
```rust
// crates/paladin-web/src/job_store.rs (additive)
pub fn get_visible(&self, id: &str, agent_id: &str, scope: &RunReadScope) -> Option<JobRecord> {
    let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
    let record = inner.jobs.get(id)?;
    let allowed = match scope {
        RunReadScope::All => true,
        RunReadScope::Tenant(t) => inner
            .owners.get(id).is_some_and(|(a, owner)| a == agent_id && owner == t),
    };
    allowed.then(|| record.clone())
}
```
`get_job_scoped` renders `ApiError::not_found(format!("unknown job '{job_id}'"))` for `None`; the foreign body then equals the unknown-job body once ids are swapped (AC-12). Poisoned-lock handling already uses `unwrap_or_else(|e| e.into_inner())`; keep it (no `unwrap`).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Thread routes authenticated-any-role, resume admin-only "pending PLAT-06" | Derive-from-runs visibility gate, hidden == missing | This phase | The module docs in `thread_controller.rs` and the "not tenant-scoped" paragraph at the end of `run_controller.rs`'s module docs are stale and must both be rewritten |
| Repo errors fixed `500` for every role (Phase 40 WR-03/04) | `Serialization` hidden as `404` for non-admin, fixed `500` for Admin | This phase | Existing Admin-principal tests stay green; non-admin cases are new |

**Deprecated/outdated:** the "PLAT-06 pending" authorization paragraph (`thread_controller.rs:31-47`) and the `GET /threads` doc line "Authenticated, any role"; `agent_controller::get_job` after F1's recommendation.

## Prove-then-Lock results (D-00d), measured 2026-10-31

| Req | Criterion | Tree evidence | Missing -> add |
|-----|-----------|---------------|----------------|
| TENANT-04 | submit/fork foreign thread refused | `ensure_thread_visible` present (`submission.rs:269`); service tests `submit_on_another_tenants_thread_is_rejected_as_unknown_thread`, `submit_thread_tenant_guard_admits_owner_admin_internal_and_fresh_threads`, `fork_on_another_tenants_thread_is_rejected_as_unknown_thread` | **HTTP-level test per route** (`POST /v1/runs {thread_id}` and `POST /v1/threads/{id}/fork`) covering foreign 404, owner, Admin, fresh id. Note a fork on a fresh id cannot succeed at HTTP level (no waypoint to fork from -> 404); prove the fresh-id acceptance at service level and the HTTP fork test for foreign/owner/Admin |
| TENANT-05 | cancel isolated at service | `submission.rs:507-535`; service tests `cancel_of_another_tenants_run_is_not_found_and_writes_nothing`, `cancel_admits_the_owning_tenant_and_admin`; HTTP test `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` | none; record as evidence (AC-08 already met, no router in the service test) |
| TENANT-06 | redacted errors, hidden == missing | `internal_repo_error`/`RUN_STORE_ERROR_MESSAGE` present; tests use an Admin principal (`tester_principal`) | non-admin `404` for `Serialization` on get/stream/webhook-deliveries/cancel (new behavior), thread routes, AC-11 unattributed-valid-run test (non-admin 404 / Admin 200) |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cargo-semver-checks` will flag `get_job`'s new parameter (lint list verified; the specific run against this tree was not possible because the disk is full) | Pitfall 1 | If it did not, D-11 as written would pass the gate; the break would still be real for callers, so the recommendation stands |
| A2 | Choosing `MAX_EXAMINED = 500`, `BATCH = 100` is acceptable | Pattern 3 | Pure tuning; planner discretion per CONTEXT |
| A3 | Skipping a thread (rather than `500`) when a foreign thread's latest-run lookup hits `Serialization` for a `Tenant` scope is acceptable | Pitfall 8 | If the owner wants strict D-09 text, one tenant's corrupt row can fail other tenants' `GET /threads` |
| A4 | Extending the hide-for-`Tenant` mapping to `WaypointError::Serialization` is acceptable | Pattern 4 | Without it, a corrupt waypoint row distinguishes a hidden thread from a missing one for non-admin |
| A5 | "Schedule store exists" in D-14 means run store enabled, regardless of `schedules.enabled` | Pitfall 11 | Operators with `schedules.enabled: false` may see a warning for rows that do not fire |
| A6 | Local root-crate tests cannot run until disk space is freed; CI will run them | Environment Availability | Plans that gate on local root-crate runs would stall |

## Open Questions (RESOLVED)
1. **`get_job` signature vs the additive-only rule (blocks D-11 and D-02).**
   - What we know: any added extractor parameter is a major semver break; D-11 assumed `JobRecord` was the only public surface.
   - What's unclear: whether the owner prefers (a) additive `get_job_scoped` + deprecated unscoped `get_job` (milestone stays `v0.11.1`, one `#[deprecated]` entry in §11.7), (b) a route-layer gate with `get_job` untouched, or (c) the deliberate break (`MIGRATION.md` §11.2 row, mirrored allowlist entry, milestone `v0.12.0`).
   - Recommendation: (a). Resolve before writing the job plan; the plan must not list `get_job`'s new signature as additive.
2. **List walk: strict D-09 `500` or skip on per-item `Serialization`** (Pitfall 8). Recommendation: skip for `Tenant` scope, log, keep `500` for other errors.
3. **Scan gate: run store enabled vs `schedules.enabled`** (Pitfall 11). Recommendation: follow D-14 literally.
4. **Thread-route error text constant.** Reuse `"run store error"` or a new `"thread store error"`. Recommendation: a new constant naming the thread store; the SPEC fixes the run constant only.
5. **`53-VERSION-DECISION.md` timing.** It needs the CI `Semver Checks (vs v0.11.0)` line from the phase PR head, which cannot be produced locally while the disk is full. The final plan needs a checkpoint that waits for the PR job (or a local run once disk is freed).

**Resolutions (plan-phase, 2026-10-31, orchestrator under --auto):**
1. RESOLVED → 53-05: `get_job` keeps its signature and body and is unrouted; additive `get_job_scoped` serves the route; deprecation in rustdoc and MIGRATION §11.7 only (a `#[deprecated]` attribute fails the patch gate, `function_marked_deprecated`); milestone stays `v0.11.1`.
2. RESOLVED → 53-04: a foreign thread whose latest-run lookup hits `Serialization` is skipped and logged for a `Tenant` scope; other errors stay the fixed `500`.
3. RESOLVED → 53-06: the scan runs whenever the schedule store exists (D-14 literal), regardless of `schedules.enabled`.
4. RESOLVED → 53-03: waypoint-store failures render the new `"thread store error"` constant; run-repository failures keep `"run store error"` (AC-09).
5. RESOLVED → 53-08: local semver reproduction for the twelve packages first (disk freed 2026-10-31); the PR job line is the fallback behind a checkpoint; `53-VERSION-DECISION.md` is the last artifact.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | all tests | yes | 1.97.1 | - |
| `paladin-web` lib tests | thread/run/job tests | yes (34 `thread_controller` tests pass offline) | - | - |
| Root `paladin-ai` lib tests (`--features web-server`) | `run_api_wiring`, `config::agents`, `submission.rs` tests | **no - disk 100% full, rustc `No space left on device`** | - | Free space (`cargo clean` of stale target dirs by the owner) or rely on CI; do not delete `target/` from a research task |
| cargo-semver-checks | local semver reproduction | installed (0.50.0) but needs a build + crates.io | 0.50.0 | CI job on the PR head |
| cargo-audit / cargo-deny | `make security` | yes | 0.22.2 / 0.20.2 | - |
| cargo-llvm-cov | coverage floor | no | - | CI `coverage` job |
| Docker | integration tests | present (not needed for this phase) | 29.8.2 | - |

**Missing dependencies with no fallback:** none blocking planning; disk space blocks local root-crate verification.
**Missing dependencies with fallback:** root-crate test runs and semver/coverage -> CI.

## Validation Architecture

`workflow.nyquist_validation` is `true` in `.planning/config.json`.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]`/`#[tokio::test]`; axum `Router::oneshot` (tower `util`); `insta` present in root dev-deps |
| Config file | none (workspace `Cargo.toml`; coverage via `scripts/coverage.sh`) |
| Quick run command | `cargo test -p paladin-web --lib thread_controller` (verified: 34 pass, offline) |
| Full suite command | `cargo test` then `make clean-code`, `make check-gates`, `make security`, `make api-surface`; CI adds semver and coverage |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TENANT-03 | Matrix: every `/v1/threads/{id}*` op, foreign 404 == missing, owner/Admin success; list omission/empty page/short page/no repeats; zero-run hidden; mixed-tenant latest-run owner; second DELETE 404 | unit/HTTP | `cargo test -p paladin-web --lib thread_controller` | Wave 0 (new tests + doubles) |
| TENANT-03 | `RunReadScope::permits_thread` table + doctest | unit | `cargo test -p paladin-ai-core --lib principal` | Wave 0 |
| TENANT-04 | HTTP per route: foreign 404; owner/Admin accepted; fresh id (service) | integration | `cargo test --lib --features web-server http_surface_tests` | Wave 0 (HTTP-level missing) |
| TENANT-05 | service cancel foreign -> `NotFound`, flag false | unit | `cargo test --lib --features web-server cancel_of_another` | yes (evidence only) |
| TENANT-06 | `Serialization` per role on get/stream/webhooks/cancel and thread routes; no body contains injected text; unattributed run 404/200 | unit/HTTP | `cargo test -p paladin-web --lib run_controller` / `thread_controller` | partial (Admin only exists) |
| TENANT-07 | job matrix: owner 200, peer key 200, foreign 404, other agent id 404, Admin 200, body == unknown-job body; evicted 404 | unit | `cargo test -p paladin-web --lib job_store` and `agent_controller` | Wave 0 |
| TENANT-08 | pure refusal/pass + pool-without-factory refusal + shipped wiring boots with Treasurer | unit/integration | `cargo test --lib --features web-server run_api_wiring` | Wave 0 |
| TENANT-09 | each reserved literal in each position rejected, case variants accepted, message has name not key | unit | `cargo test --lib --features web-server config::agents` | Wave 0 rows |
| TENANT-10 | 0/1/many unattributed -> 0/1/1 warn with exact count+ascending ids; half-attributed -> one error + boot ok; store byte-identical (no writes) | unit | `cargo test --lib --features web-server run_api_wiring` | Wave 0 |
| SC5 | semver green all 12 packages, §11.2 empty, allowlist zero | CI/gate | `make check-migration-allowlist`; CI `Semver Checks (vs v0.11.0)` | CI only |

### Sampling Rate
- **Per task commit:** the quick command for the touched crate, plus `cargo fmt --check` and `cargo clippy -p <crate> -- -D warnings`.
- **Per wave merge:** `cargo test` (workspace), `make clean-code`, `make check-gates`.
- **Phase gate:** full suite, `make api-surface`, `make security`, CI semver line recorded, coverage >= 82 %.

### Wave 0 Gaps
- [ ] Thread-test doubles: a run repository honoring `thread_id` with attributed runs and an injectable `Serialization`; a waypoint store whose `list_threads` honors `limit`/`before`.
- [ ] `published_thread_id_operations` + route-matrix test and list/walk tests in `thread_controller.rs`.
- [ ] HTTP-level foreign submit/fork tests in `src/application/services/run/http_surface_tests.rs`.
- [ ] Capturing-logger test for the schedule scan (or the `ScanOutcome` split of Pattern 8).
- [ ] Re-seed existing thread tests per Pitfalls 3-4.
- [ ] Free disk space for local root-crate runs (environment, not code).

## Security Domain

`security_enforcement` is not set to `false` in config, so this section applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (unchanged; `require_authentication` already layers every route) | existing middleware |
| V3 Session Management | no | stateless API keys / bearer |
| V4 Access Control | **yes** | `RunReadScope` rule; one gate per object type; hidden == missing; Admin bypass; route matrix guards new routes |
| V5 Input Validation | yes | `parse_thread_id`, `TenantId::new`; reserved-literal rejection at config load |
| V6 Cryptography | no | no crypto added |
| V7 Error Handling & Logging | **yes** | fixed redacted error bodies, detail only in `log::error!`; no key values or row text in boot messages (AC-N2) |
| V14 Configuration | yes | fail-closed boot: reserved literals refused, Treasurer-without-factory refused |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Cross-tenant enumeration/read of threads (IDOR) | Information disclosure | Gate every `/threads/{id}*`; list omits foreign threads |
| Existence oracle via `403`, distinct code, differing body or `500` for a corrupt foreign row | Information disclosure | Same status/code/body/headers for hidden and missing; `Serialization` hidden as `404` for non-admin |
| Cross-tenant mutation (resume, fork, delete, cancel, run against thread) | Tampering | Gate before `require_admin`; service-layer scope in `cancel`/`ensure_thread_visible` |
| Backend text in error bodies (connection strings, row shapes) | Information disclosure | Fixed messages, detail to server log (all seven thread sites, Pitfall 7) |
| Job result readable by any holder of a job id | Information disclosure | `(agent, tenant)` owner check; foreign/mismatch -> unknown-job 404 |
| Silent loss of spend enforcement | Elevation/Repudiation | Boot refusal when Treasurer is configured without the engine-factory path |
| Principal collides with sentinel tenant/name (`open-access`, `unattributed`, `anonymous`) | Spoofing/Elevation | Reserved literals rejected at config load |
| Secret in logs or errors | Information disclosure | Manual credential review per `.github/instructions/security.instructions.md`; no `Debug` of config carrying keys; scan logs ids and counts only |
| SSRF / DNS rebinding, SQL injection on the tenant predicate | - | Out of scope: owned by `/gsd-secure-phase` and the manual review (SPEC canon referrals) |

CodeQL is advisory-only for Rust and does not gate; Snyk must not be reintroduced (project security instructions). The manual credential-handling review remains the control for the new error/log paths.

## Sources

### Primary (HIGH confidence)
- Codebase, read this session: `crates/paladin-web/src/{thread_controller,run_controller,agent_controller,job_store,agent_auth,lib}.rs`; `crates/paladin-core/src/platform/container/principal.rs`; `src/application/services/run/{submission,worker,http_surface_tests}.rs`; `src/infrastructure/web/run_api_wiring.rs`; `src/config/agents.rs`; `src/bin/paladin-server.rs`; `crates/paladin-storage/src/run/sqlite.rs`, `run_schedule/sqlite.rs`; `crates/paladin-ports/src/output/{run_repository_port,run_schedule_repository_port,waypoint_port}.rs`
- `.planning/phases/53-tenant-isolation-closure/53-CONTEXT.md`, `53-SPEC.md`; `.planning/REQUIREMENTS.md`, `.planning/WINDOWS.md` rows 58/63, `MIGRATION.md` §11, `.cargo/semver-checks-allowlist.toml`, `scripts/check-semver.sh`
- `cargo semver-checks --list` (0.50.0) for lint severities; `cargo test -p paladin-web --lib thread_controller --offline` (34 passed)

### Secondary (MEDIUM confidence)
- None used.

### Tertiary (LOW confidence)
- None used. No web search was needed; the phase is internal to the repository.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies; versions read from manifests.
- Architecture: HIGH - every touched function was read; patterns copy existing in-repo ones.
- Pitfalls: HIGH for F1-F5 and the test-impact list (read from the tree); MEDIUM for A3/A4/A5 recommendations (design choices needing owner confirmation).
- Semver outcome: MEDIUM - lint severity verified, but the full job could not be run locally (disk).

**Research date:** 2026-10-31
**Valid until:** 2026-11-30 (code-specific; invalidated by any change to the files listed in Sources)
