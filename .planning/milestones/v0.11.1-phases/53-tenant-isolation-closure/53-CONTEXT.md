# Phase 53: Tenant Isolation Closure - Context

**Gathered:** 2026-10-30
**Status:** Ready for planning
**Mode:** `--auto` (recommended option selected for every question; resumed from the paused
checkpoint of the same day; the user answered the WHAT questions in `53-SPEC.md` interactively)

<domain>
## Phase Boundary

Close the milestone's only S1 cluster: a principal of one tenant can no longer enumerate, read,
run against or mutate another tenant's threads, runs or jobs through `/v1/threads/*`,
`POST /v1/runs {thread_id}`, `POST /v1/threads/{id}/fork`, `RunSubmissionService::cancel` or
`GET /agents/{id}/jobs/{job_id}`; a hidden object answers exactly like a missing one; a corrupt
run row is redacted by role; a configured Treasurer cannot boot unenforced; reserved principal
literals are refused at config load; unattributed legacy schedules are surfaced once at boot; and
the `v0.11.1`-versus-`v0.12.0` decision is recorded before Phase 58. No new routes, ports or
adapters. Requirements TENANT-03..10 (inventory rows A1-A8).

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**9 requirements are locked.** See `53-SPEC.md` for full requirements, boundaries, and acceptance
criteria (20 positive and 5 negative pass/fail criteria, 24/24 edges and 5/5 prohibitions resolved).

Downstream agents MUST read `53-SPEC.md` before planning or implementing. Requirements are not
duplicated here.

**In scope (from SPEC.md):**
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

**Out of scope (from SPEC.md):**
- Any new HTTP route, port or adapter — milestone scope guard.
- Backfill or `PATCH` re-assignment of `created_by` (TENANT-11) — deferred by milestone decision A7.
- Tenant-namespaced thread ids — deferred thread-tenancy work (40-CONTEXT D-14); known limitation.
- Schedule-fired runs inheriting a creator, allowance enforcement changes, aggregate spend routes.
- Resolve-then-connect SSRF pinning, SQL-injection review of the tenant predicate — canon items
  owned by `/gsd-secure-phase` and the manual credential review.
- The lockstep version bump and the tag — Phase 58.

</spec_lock>

<decisions>
## Implementation Decisions

### Carried forward (locked by ADR-0054, Phase 52, the milestone open and the SPEC — not re-asked)

- **D-00a:** Gates as Phase 52 D-00a: 82 % workspace line coverage (ADR-0006), `make clean-code`,
  `make check-gates`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for
  an intentional public-surface change), `make security`, `make check-migration-allowlist`, the
  manual credential-handling review in `.github/instructions/security.instructions.md`. No
  `unwrap`/`expect`/`panic!` in library code. TDD: every behavior below is watched failing first.
- **D-00b:** Semver as Phase 52 D-01/D-03: every public change is judged at `--release-type patch`
  against the `0.11.0` baseline for all twelve published packages; a deliberate break needs a
  `MIGRATION.md` §11.2 row and a mirrored `.cargo/semver-checks-allowlist.toml` entry in the same
  commit and flips the milestone to `v0.12.0`. A break not forced by the security fix is
  reshaped or dropped (roadmap scope note).
- **D-00c:** ADR-0054 governs the rule: tenant is server-derived only; `RunReadScope { All,
  Tenant }` is the one value type; hidden equals missing (same status, code, body bytes, headers,
  never `403` for an existence question); Admin bypass; a `Tenant` scope never permits an
  unattributed run; a route matrix is read from the router's own OpenAPI document.
- **D-00d:** Phase 40's WR-01..WR-04 fixes are present in the tree (verified 2026-10-30):
  `RunSubmissionService::ensure_thread_visible` (submit/fork), the service-layer scope check in
  `cancel`, `internal_repo_error` + `RUN_STORE_ERROR_MESSAGE` in `run_controller.rs`. TENANT-04,
  -05 and -06 are prove-then-lock: the plan starts by running the SPEC's acceptance checks against
  the tree and adds only the missing regression tests; only what fails is rebuilt.
- **D-00e:** New ADRs take the next free number from `.planning/decisions/PROMOTION.md`
  (currently **0063**) and advance that line in the same commit. `MIGRATION.md` §9/§10 and
  `CHANGELOG.md` `[0.11.0]` are byte-frozen. Logging stays on the `log` facade.
- **D-00f:** Observable contracts fixed by `53-SPEC.md` and not revisited here: corrupt row →
  `404` non-admin / fixed `500` Admin; zero-run thread Admin-only; job scoped to (agent, tenant)
  with Admin bypass; boot refusal = Treasurer configured AND no engine factory; reserved set
  `open-access`, `unattributed` (tenant + key name), `anonymous` (key name), exact and
  case-sensitive; one `warn!` with count + every id ascending, advisory scan that never blocks boot.

### Thread ownership source and the version call (TENANT-03, SC5)

- **D-01:** **Derive the thread's tenant from its runs; do not record it.** A thread's owner is
  the attribution (`submitted_by`) of its latest run (newest `created_at`, run id as tiebreak),
  looked up with the unrestricted `All` scope as an internal authorization read; a thread with no
  run row has no owner and is Admin-only. This is the rule `ensure_thread_visible` already applies
  on submit and fork, so one rule serves reads and writes. No column is added to `waypoints`, no
  threads table is created, and `Waypoint`, `ThreadSummary` and `WaypointPort` are untouched —
  every change is additive and the milestone stays `v0.11.1`.
  `[auto] Thread ownership — Q: "Where does a thread's tenant come from?" → "Derive from the thread's runs" (recommended)`
  — **Reversibility:** reversible — recording a tenant later is a separate, additive-to-this
  decision migration in a future minor; nothing here forecloses it.
- **D-02:** **Version decision: additive, `v0.11.1`, proven by the semver job.** The phase ends
  with `53-VERSION-DECISION.md` recording the green `Semver Checks (vs v0.11.0)` job line (`N
  checks: X pass, 0 fail`, N > 0) for all twelve packages on the phase PR head, an empty §11.2 and
  a zero-entry allowlist. If the job flags any change, the change is reshaped to additive (new
  inherent methods, new functions, new private state) rather than registered; a break is
  registered only if no additive shape exists for a security fix, and then the file names it and
  the milestone becomes `v0.12.0`. Foreign-tenant `200 → 404` changes are entered in `MIGRATION.md`
  §11.1 and §11.6 as behavioral changes, never as §11.2 rows.
  `[auto] Version call — Q: "If semver flags a change?" → "Reshape to additive first" (recommended)`
- **D-03:** **The ownership rule lives in core, next to `RunReadScope`.** One additive inherent
  method on `RunReadScope` (name is the planner's — e.g. `permits_thread_owner(latest_run:
  Option<&Run>) -> bool`) expresses "latest run permitted, or `All`; no run → `All` only".
  `RunSubmissionService::ensure_thread_visible` is refactored to call it and the thread gate (D-05)
  calls it, so the rule is written exactly once. Adding an inherent method is not a semver break.
  `[auto] Rule location — Q: "Where does the thread rule live?" → "Inherent method on RunReadScope" (recommended)`
  — **Reversibility:** reversible.
- **D-04:** **`GET /v1/threads` filters by bounded over-fetch, not a new port method.** The handler
  walks `WaypointPort::list_threads(Some(page), before)` pages, resolves each thread's latest run
  with `RunQuery { thread_id, limit: 1, scope: All }` (the same lookup D-01 uses), keeps the
  threads the caller's scope permits, and stops when `limit` visible items are collected or the
  store is exhausted; `next_cursor` is the `last_updated_at` of the last *examined* thread so the
  keyset walk never repeats an item, and a page may be shorter than `limit` with a non-null
  cursor. Admin (`All`) skips every lookup. The N+1 cost is accepted for this cleanup phase; a
  runs-side listing (`RunRepositoryPort` change) is a break and is not taken.
  `[auto] List filtering — Q: "Per-item derive or runs-side listing?" → "Bounded over-fetch, per-item derive" (recommended)`
  — **Reversibility:** reversible — a later minor can add a repository-side thread listing.

### Gate shape and run-less threads (TENANT-03)

- **D-05:** **One `load_visible_thread` helper in `thread_controller.rs`, mirroring
  `load_visible_run`.** Signature shape: `(state: &ThreadApiState, principal: &Principal, thread:
  &ThreadId) -> Result<Waypoint, ApiError>`; it loads the latest Waypoint, resolves the latest run
  through `state.runs`, applies D-03, and returns the byte-identical `404 unknown thread '{id}'`
  for missing and hidden alike. Every `/threads/{id}*` handler enters through it; `list_threads`
  uses the D-04 walk with the same rule. The service half (`ensure_thread_visible`) stays where it
  is for submit/fork.
  `[auto] Gate location — Q: "Controller helper or application-layer port?" → "Controller helper beside load_visible_run" (recommended)`
- **D-06:** **Gate before `require_admin` on `resume` and `delete`.** A foreign non-admin gets
  `404` (hidden equals missing, SC1 names resume and delete explicitly); a same-tenant non-admin
  then gets the existing `403`; Admin passes both. Missing thread → `404` for every role.
  `[auto] Admin routes — Q: "Gate or admin check first?" → "Gate first" (recommended)`
- **D-07:** **No run repository wired ⇒ fail closed for non-admin.** When `ThreadApiState.runs`
  is `None` (run store disabled) ownership cannot be established, so a non-admin principal sees no
  thread (`404` / empty list) and Admin is unaffected. Documented in the module docs that replace
  the stale "PLAT-06 pending" paragraph.
  `[auto] No run store — Q: "Visible or hidden when ownership cannot be derived?" → "Hidden (fail closed)" (recommended)`
- **D-08:** **Route matrix = `published_run_id_operations` pattern.** The thread test module gains
  a `published_thread_id_operations(&OpenApi)` reader over `versioned_thread_parts(..).1` for every
  `(METHOD, /v1/threads/{id}*)` operation and a table of expected success statuses per operation;
  it sends each operation as the owner (success), as a foreign user-role key (`404`, body equal
  to a never-existing id's body) and as Admin (success); a separate test proves list omission and
  the empty-page shape. Adding a route without joining the table fails the test.

### Corrupt rows and Serialization errors (TENANT-06)

- **D-09:** **Role-dependent mapping inside `load_visible_run` and `load_visible_thread` only.**
  `RunRepositoryError::Serialization` (and the SQLite read-time half-attribution rejection, which
  surfaces as `Serialization`) maps to `ApiError::not_found` for a `Tenant` scope and to
  `internal_repo_error` (fixed `500`) for `All`; every other repository error stays the fixed
  `500` for every role (an outage is not an existence signal). List routes keep the fixed `500`
  for all roles — there is no single object to hide. Thread routes that load runs (D-05) use the
  same mapping.
  `[auto] Corrupt row — Q: "Which errors are role-dependent?" → "Serialization only; others fixed 500" (recommended)`
- **D-10:** Tests inject `Serialization` through the existing failing-repository test double in
  `run_controller.rs` (the `Some(message)` seam) and assert status per role and that no body
  contains the injected text, on `GET /runs/{id}`, `/stream`, `/webhook-deliveries`, `POST
  /cancel` and the thread routes.

### Jobs, boot assertion, small guards (TENANT-07, -08, -09, -10)

- **D-11:** **Job owner lives in `JobStore`'s private state, not on `JobRecord`.** `Inner` gains a
  per-job owner `(agent_id, TenantId)` (or `Option<RunAttribution>` + agent id); additive methods
  `JobStore::create_owned(agent_id, &Principal)` and `JobStore::get_visible(job_id, agent_id,
  &RunReadScope) -> Option<JobRecord>` are added, the existing `create`/`get` stay for
  compatibility. `enqueue_job` records the owner; `get_job` reads through `get_visible` and
  answers `404 unknown job '{id}'` for a hidden, agent-mismatched, evicted or unknown job alike.
  `JobRecord` is unchanged, so `paladin-web`'s public surface does not break.
  `[auto] Job owner storage — Q: "On JobRecord or inside the store?" → "Inside the store, additive methods" (recommended)`
  — **Reversibility:** reversible.
- **D-12:** **Treasurer assertion = one pure check called from `build_run_api`.** An additive
  `RunWorkerPool::has_engine_factory(&self) -> bool` (or equivalent inspector) plus a pure
  function in `run_api_wiring.rs` (`assert_treasurer_enforced(treasurer_configured: bool,
  has_engine_factory: bool) -> Result<(), String>`) that returns the named error "treasurer
  configured but the run worker pool has no engine factory; settlement, spend guard and cancel
  probe would not attach". `build_run_api` calls it right after the pool is built and before
  `spawn`, so the server's `main` fails before any listener binds. Tests: the pure function
  refuses (configured + no factory) and passes (configured + factory, or unconfigured); an
  integration test proves the shipped wiring boots with a Treasurer.
  `[auto] Assertion placement — Q: "Pool method or wiring check?" → "Pure check in wiring, additive pool inspector" (recommended)`
- **D-13:** **Reserved literals as one exported constant.** `principal.rs` (paladin-ai-core)
  gains `pub const RESERVED_PRINCIPAL_LITERALS: &[&str] = &["open-access", "unattributed"]` beside
  `TenantId::OPEN_ACCESS`, and the key-name-only reservation of `"anonymous"` is a second
  constant; `AuthConfig::validate` checks `tenant`, `name` and `bearer_token.tenant` against them
  with the existing message shape (`http.auth.api_keys[{name}]: 'tenant' is the reserved literal
  'open-access' …`), never printing the key. `TenantId::new` is unchanged.
- **D-14:** **`NULL created_by` scan in `run_api_wiring`, after the schedule store opens.** Only
  when the schedule store exists: page `RunScheduleRepositoryPort::list(100, cursor)` to
  exhaustion, collect ids whose `created_by` is `None`, emit exactly one `log::warn!` (count, ids
  ascending, "re-create via POST /v1/schedules"); on a page error emit one `log::error!` with the
  cursor position and stop the scan — boot continues. No port change, no write.
- **D-15:** **ADR-0063 amends ADR-0054.** A short ADR records the thread rule (D-01/D-03), the
  fail-closed no-run-store rule (D-07), the job rule (D-11) and the role-dependent corrupt-row
  mapping (D-09), and advances the PROMOTION line to 0064.
  `[auto] ADR — Q: "Amend ADR-0054 or fold into evidence?" → "Short amending ADR-0063" (recommended)`

### Claude's Discretion

- Exact names of the new inherent methods and the wiring assertion function (D-03, D-11, D-12).
- Page size for the D-04 walk and whether the D-04 lookup is shared as a private helper with
  `active_run_id_for`.
- The wording of the module-doc rewrite in `thread_controller.rs` (the PLAT-06 paragraph is stale).
- Evidence file naming beyond `53-VERSION-DECISION.md` (one `53-TENANT-EVIDENCE.md` for the
  prove-then-lock results of D-00d is acceptable).
- Whether WINDOWS rows 58 and 63 are closed in this phase's final plan or handed to Phase 57 with
  the evidence pointer (either is acceptable; the row text must cite the matrix test and the warn
  test).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked requirements
- `.planning/phases/53-tenant-isolation-closure/53-SPEC.md` — **Locked requirements — MUST read
  before planning**: R1-R9, boundaries, 25 acceptance criteria, edge and prohibition tables.
- `.planning/ROADMAP.md` "Phase 53" — goal, five success criteria, scope notes (re-verify first;
  decision point; no new routes).
- `.planning/REQUIREMENTS.md` "Tenant Isolation (TENANT)" — TENANT-03..10 text and the
  roadmap-time finding on WR-01..04.

### Phase input of record
- `.project/v0.11.1/01-carried-gaps-inventory.md` §A rows A1-A8 — sources, severities, dispositions.
- `.project/v0.11.1/03-suggestions-and-scoping.md` §7 — the `v0.11.1`-vs-`v0.12.0` naming rule.
- `.planning/PROJECT.md` "Current Milestone" — decisions at milestone open (A6, A7, versioning,
  scope guards).

### Governing decisions
- `.planning/decisions/0054-tenant-scoped-run-reads.md` — the rule this phase extends to threads
  and jobs (server-derived tenant, hidden equals missing, Admin bypass, route matrix).
- `.planning/decisions/PROMOTION.md` — next free ADR number 0063 (D-15).
- `.planning/phases/52-ci-release-pipeline-repair/52-CONTEXT.md` D-01, D-03 — patch-level semver
  gate and the §11.2 register the gate reads.
- `MIGRATION.md` §11 — empty v0.11.1 register; §11.1/§11.6 receive the behavioral entries (D-02).
- `.cargo/semver-checks-allowlist.toml` — must stay at zero entries under D-02.

### Phase 40 evidence
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-REVIEW.md` — IN-01
  (reserved literals), IN-04 (`get_job`), WR-01..WR-04 findings.
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-REVIEW-FIX.md` —
  what WR-01..04 changed, commits, and the known limitation of derive-from-runs.
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-CONTEXT.md` D-14 —
  the thread deferral.
- `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` G2 — the Treasurer wiring assertion gap.
- `.planning/WINDOWS.md` rows 58 and 63 — closing conditions this phase satisfies.

### Code this phase edits
- `crates/paladin-web/src/thread_controller.rs` — seven handlers, `thread_openapi_router`,
  `versioned_thread_parts`, stale PLAT-06 module docs (D-05..D-08).
- `crates/paladin-web/src/run_controller.rs` — `load_visible_run`, `internal_repo_error`,
  `RUN_STORE_ERROR_MESSAGE`, `map_submission_error`, `published_run_id_operations` (D-08 pattern,
  D-09, D-10).
- `crates/paladin-web/src/agent_controller.rs` — `enqueue_job`, `get_job` (D-11).
- `crates/paladin-web/src/job_store.rs` — `JobStore`, `Inner`, `JobRecord` (D-11).
- `src/application/services/run/submission.rs` — `ensure_thread_visible`, `cancel` (D-00d, D-03).
- `src/application/services/run/http_surface_tests.rs` — contract-style tests for submit/fork/cancel.
- `src/application/services/run/worker.rs` — `RunWorkerPool::with_engine_factory`,
  `with_treasurer` (D-12 inspector).
- `src/infrastructure/web/run_api_wiring.rs` — `build_run_api` pool construction and schedule
  store opening (D-12, D-14).
- `src/config/agents.rs` — `AuthConfig::validate` (D-13).
- `crates/paladin-core/src/platform/container/principal.rs` — `RunReadScope`, `TenantId::OPEN_ACCESS`
  (D-03, D-13).
- `crates/paladin-ports/src/output/waypoint_port.rs`, `run_repository_port.rs`,
  `run_schedule_repository_port.rs` — read-only: the port surfaces D-04 and D-14 walk; unchanged.
- `crates/paladin-storage/src/run/sqlite.rs`, `postgres.rs` (`row_to_run`), migrations
  `001_create_waypoints_table.sql`, `010_add_run_schedule_created_by.sql` — read-only context.
- `.github/instructions/security.instructions.md` — the manual credential review that governs
  every log and error message added here.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `run_controller::load_visible_run` + `internal_repo_error`: the exact shape D-05 and D-09 copy;
  `RUN_STORE_ERROR_MESSAGE` is the redacted body constant.
- `run_controller` tests: `published_run_id_operations(&OpenApi)` and the failing-repository
  double with a `Some(message)` Serialization seam — D-08 and D-10 reuse them.
- `RunSubmissionService::ensure_thread_visible`: already derives ownership from the latest run via
  `RunQuery { thread_id, limit: 1 }` with `All` scope — D-01/D-03 extract its rule into core.
- `Principal::read_scope()` / `RunReadScope::for_principal`: the only scope constructors; the
  thread and job gates take a `RunReadScope`, never a raw role or tenant.
- `AuthConfig::validate` message shape (`http.auth.api_keys[{name}]: …`, never the secret) and
  its table tests in `src/config/agents.rs` and `paladin-server.rs` — D-13 adds rows.
- `JobStore` bounded `RwLock<Inner>` — D-11 extends `Inner` privately.
- `tests/scripts` and `make check-gates` — unchanged; no shell or CI edits expected.

### Established Patterns
- Hidden equals missing: one helper per object type is the entry point of every `/{id}*` route;
  tests compare the hidden body to a never-existing id's body.
- Tenant predicate is bound inside the adapter query for lists (Phase 40 D-02); D-04 keeps the
  predicate out of SQL by not touching the adapters — the filter is an authorization read per
  thread, accepted as N+1 for this phase.
- Operator-facing messages name the entry and never a secret; backend text goes to `log::error!`
  only.
- Measured, not assumed: prove-then-lock (D-00d) and the semver line recorded as evidence (D-02).
- ADRs are short amendments with a PROMOTION line advance (D-15).

### Integration Points
- `src/bin/paladin-server.rs` merges `thread_router` beside `agent_router`; `ThreadApiState` is
  built with `with_runs(handles.run_repository)` and `with_run_submission` from
  `RunApiHandles` — D-07's `None` branch is the run-store-disabled deployment.
- `build_run_api` is the single place the pool, the Treasurer, the submission service and the
  schedule store meet (D-12, D-14).
- Phase 57's review sweep reads this phase's tests to disposition WR-01..04, IN-01, IN-04 and
  WINDOWS rows 58/63; Phase 58's pre-tag audit reads `53-VERSION-DECISION.md`.

</code_context>

<specifics>
## Specific Ideas

- The semver job's green line for all twelve packages is the version decision's evidence; the
  decision file is written after the job runs on the phase PR head, not before.
- `thread_controller.rs` module docs still describe "PLAT-06 pending" authorization; the rewrite
  states the derive-from-latest-run rule and the fail-closed no-run-store rule plainly.
- The `warn!` for unattributed schedules is a single line an operator can grep once; it names the
  action (`POST /v1/schedules`), never a backfill.

</specifics>

<deferred>
## Deferred Ideas

- **Record the tenant on the thread** (a `tenant_id` column or threads table plus public fields on
  `Waypoint`/`ThreadSummary`) — a registered break; candidate for the next minor if the N+1 list
  cost (D-04) matters in a dogfood deployment.
- **Repository-side thread listing by scope** (`RunRepositoryPort`/`WaypointPort` method) — same
  reason, same horizon.
- **Tenant-namespaced thread ids** to close the "unused id vs hidden id" distinction on submit —
  40-CONTEXT D-14 deferral, unchanged.
- **Backfill / `PATCH` re-assignment of `created_by`** (TENANT-11) — only if a dogfood deployment
  has such rows.

### Reviewed Todos (not folded)
`[auto] todo.match-phase returned four matches at score 0.6; the auto-fold rule (≥ 0.4) was
overridden because every match is keyword-only and folding them would widen a security phase
against the milestone scope guard — recorded here instead.`
- `2026-08-13-verify-local-coverage-reproduction.md` — assigned to Phase 56 (`resolves_phase: 56`).
- `2026-10-13-review-content-data-orchestration-base-items.md` — paladin-core base items; Phase 55/G3.
- `2026-10-14-web3sec-single-bump-to-v0-11-0.md` — the downstream consumer's own act.
- `2026-10-20-openai-responses-api-for-pro-tier.md` — ruled out at milestone open.

</deferred>

---

*Phase: 53-tenant-isolation-closure*
*Context gathered: 2026-10-30*
