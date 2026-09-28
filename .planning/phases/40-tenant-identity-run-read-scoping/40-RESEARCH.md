# Phase 40: Tenant Identity & Run-Read Scoping - Research

**Researched:** 2026-09-28
**Domain:** Rust hexagonal-architecture multi-tenant identity + row-level authorization (HTTP API, SQL storage)
**Confidence:** HIGH (every claim below is grounded in a specific file:line read this session; no web search was needed — this is a pure in-tree extension of existing, well-documented Phase 27/38/39 patterns)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried forward (locked by earlier phases and milestone-level decisions — not re-asked)**

- **D-00a:** Phase 39 D-01 is binding: the `007` ledger schema, `LedgerScope { tenant_id: String,
  api_key_id: String }` and every `TreasuryLedgerPort` method are **final**. This phase replaces
  only the *source* of the scope (the run's recorded principal) — never the schema, the port or
  the queries. `LedgerScope::unattributed()` stays defined and documented as the value stamped
  when no principal exists.
- **D-00b:** The identity carrier is the existing `paladin_web::agent_auth::Principal` attached by
  `require_authentication` (ADR-0040 opaque bearer tokens, ADR-0041 in-process token store, D-44
  shared middleware, D-46 two-tier authorization). No new middleware, no new credential type.
- **D-00c:** X-03 governs public API: every break needs a `MIGRATION.md` §9.2 row and, when marked
  `Y`, a `.cargo/semver-checks-allowlist.toml` entry naming one crate and one lint (Phase 38
  D-00g, 39-08 D-27 method). The CI `semver` job is a known no-op against the 0.9.0 baseline
  (STATE.md concern) — the rows are still written; the register is the control, not the tool.
- **D-00d:** Config sub-structs follow the house shape: `Default` + `validate()` never clamps,
  fail closed with a message naming the offending entry (Phase 38 D-00h; `build_auth_config` in
  `src/bin/paladin-server.rs` is the existing fail-closed precedent).
- **D-00e:** Vocabulary: Medieval-military words for roles, plain words for units and identifiers
  (Phase 30 D-01). "Tenant", "API key", "principal" are plain identifiers, not new officers.
- **D-00f:** ADRs take the next free number from `.planning/decisions/PROMOTION.md` (currently
  **0054**) and advance that line in the same commit (Phase 38 D-00f).
- **D-00g:** Shipped tree outranks any document; 82 % coverage floor; `make clean-code`,
  `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional surface
  change), `make security`, and the manual credential-handling review gate every commit
  (Phase 38 D-00i/D-00j). No log line, error or response body ever carries the API key value.

**Identity carrier shape**

- **D-01:** `Principal` gains a **required** `tenant_id: TenantId` field: `Principal { id, role,
  tenant_id }`. There is no `Option` — every authenticated principal belongs to exactly one
  tenant. `TenantId` is a new newtype in `paladin-core`
  (`crates/paladin-core/src/platform/container/principal.rs`, alongside `RunId`/`ThreadId`
  precedent): non-empty, trimmed, no whitespace, printable ASCII, bounded length (planner picks
  the bound; `ThreadId`'s validation is the analog), `as_str()`, `Display`, `Serialize`/
  `Deserialize`, `Hash`/`Eq`. — **Reversibility:** one-way — `Principal` is a published
  `paladin-web` type with public fields; adding a required field breaks every external
  constructor, which is exactly the break TENANT-01 pre-authorises and §9.2 records.
- **D-02:** The tenant is **server-derived only**. No request header, query parameter or body
  field names a tenant; `SubmitRunRequest` gains no tenant field; the mapping is looked up from
  `AgentAuthConfig` in `authenticate()` and nowhere else. A test proves a caller presenting key
  A cannot obtain tenant B by any request-side means (success criterion 1).
- **D-03:** **Bearer-token principals** (ADR-0040) carry the tenant configured at
  `http.auth.bearer_token.tenant`, a new field that is **required when `bearer_token.enabled`
  is true** and rejected at `validate()` when missing; their `Principal.id` remains the token's
  `user_id` string. `AuthClaims` is untouched — the tenant is a deployment-level mapping, not a
  claim. **Open-access** (`enabled = false`) attaches `Principal::open_access()` with a documented
  sentinel `TenantId::OPEN_ACCESS` (literal `"open-access"`) and `role = Admin`, so open mode
  keeps its deployment-wide read behaviour by the Admin rule in D-11, not by a bypass.
- **D-04:** The `(String, UserRole)` tuple that `SubmitRun.requested_by`, `ForkRun.requested_by`
  and `RunSubmissionPort::cancel(.., requested_by)` carry is **replaced by one core value type**
  `PrincipalRef { api_key_id: String, tenant_id: TenantId, role: UserRole }` (same module as
  `TenantId`; `From<&paladin_web::Principal>` lives in `paladin-web`). `authorize_invocation` in
  `RunSubmissionService` reads `.role` from it; `Option<PrincipalRef>` keeps the existing
  "`None` = internal caller, skip the role check" semantics. — **Reversibility:** one-way — a
  `paladin-ports` signature change on a published trait and two published structs; recorded in
  §9.2 with allowlist entries.

**Config mapping surface**

- **D-05:** `ApiKeyConfig` gains a **required** `tenant: String` field (parsed into `TenantId` at
  validation): every `http.auth.api_keys` entry MUST name its tenant. A config that omits it
  fails `validate()` / server boot with a message naming the key's `name`. There is **no implicit
  default tenant** — an implicit `"default"` would silently pool every key's spend into one
  scope and make Phase 41's per-tenant allowances enforce the wrong thing. Recorded as a §9.5
  configuration break (the one deliberate X-09 exception this phase makes) with the
  `config.example.yml` and `docs/src/deployment-topologies/http-service-host.md` examples
  updated. — **Reversibility:** costly — every operator config with API keys must be edited on
  upgrade; the migration guide row is the mitigation.
- **D-06:** The existing `ApiKeyConfig.name` **is** the API key id (it already is the
  `Principal.id` and "appears in logs"); no new `id` field. Because `name` becomes an
  attribution and scope key, `AuthConfig::validate()` (new; there is none today) rejects
  duplicate `name` values, duplicate `key` values, and any `name`/`tenant` that fails `TenantId`-
  style identifier rules. The `paladin-cli` onboarding template (`src/application/cli/
  templates/env.rs`, `commands/onboarding.rs`) emits the `tenant` field.
- **D-07:** No tenant registry section in this phase. Tenants exist only as the set of values
  named by the key mapping. Phase 41 may add a cross-check that every allowance's tenant id is
  one some key maps to (deferred idea).

**Run-row attribution and migration**

- **D-08:** `Run` gains `submitted_by: Option<RunAttribution>` where `RunAttribution { tenant_id:
  TenantId, api_key_id: String }` (same core module; `PrincipalRef::attribution()` derives it,
  role is deliberately **not** persisted — roles are config, not data). `#[serde(default)]`,
  set by `Run::new`'s builder chain (`with_submitted_by`), populated by `RunSubmissionService::
  submit`/`fork` from `request.requested_by`. `Run` is already `#[non_exhaustive]` with a builder
  (X-10.3), so the Rust-side addition is additive; the `RUN_SCHEMA_VERSION` question is settled by
  the X-04 rule in `run.rs`'s own module docs (Claude's discretion below).
- **D-09:** Persistence is a new pair of migrations `crates/paladin-storage/migrations/{sqlite,
  postgres}/008_add_run_attribution_columns.sql`: `ALTER TABLE runs ADD COLUMN tenant_id TEXT
  NULL`, `ALTER TABLE runs ADD COLUMN api_key_id TEXT NULL`, plus a covering index on
  `(tenant_id, submitted_at DESC, run_id DESC)` for the scoped keyset list. **`NULL` means "no
  principal recorded"** (pre-v0.11 rows, schedule-fired and internal submits) — never the string
  `"unattributed"`; the sentinel is a ledger-side value (D-00a), not a run-row value. `INSERT`/
  `SELECT` constants in `run/sqlite.rs` and `run/postgres.rs` and the in-memory adapter carry the
  new fields; the run contract suite gains attribution round-trip and scoped-list cases. —
  **Reversibility:** one-way — persisted columns; reshaping later needs a data migration.
- **D-10:** Runs submitted with `requested_by: None` — schedule-fired runs (`schedule/service.rs`
  passes `None` today), same-process embedders, tests — persist `submitted_by = None` this phase.
  They are visible to Admin principals only (D-11) and settle under `LedgerScope::unattributed()`.
  Making schedule-fired runs inherit the schedule creator's principal is **deferred** and flagged
  for Phase 41, which needs it for "refuse at admission on every run path".

**Read-scope rule and the shared authorization function**

- **D-11:** Visibility unit is the **tenant**, with an **Admin bypass**: a principal may see a run
  iff `run.submitted_by.tenant_id == principal.tenant_id`, OR `principal.role == Admin` (Admin is
  the deployment-operator role — it already gates registry-shaped routes — so it sees every run,
  including `submitted_by = None` rows). A non-Admin principal never sees an unattributed run.
  Per-API-key narrowing within a tenant is deferred (FUT-05 territory). — **Reversibility:**
  costly — the rule becomes a documented HTTP contract (§9.6, platform-api.md) and Phase 41
  builds on tenant scope.
- **D-12:** The **one shared function** is a core value type `RunReadScope` (same module as
  `TenantId`): `enum RunReadScope { All, Tenant(TenantId) }` with `fn for_principal(role,
  tenant_id) -> Self` and `fn permits(&self, run: &Run) -> bool`. `paladin-web` derives it from
  `Principal` once (`Principal::read_scope()` next to `authorize_invoke`/`require_admin` in
  `agent_auth.rs`). It is applied in exactly two mechanisms and nowhere else:
  1. **List:** `RunQuery` gains `scope: RunReadScope` (`All` is the `Default` so internal callers
     are unchanged); every `RunRepositoryPort::list` adapter applies `Tenant(t)` as
     `WHERE tenant_id = ?` in SQL (in-memory: a filter), so the scoped page and cursor stay a
     correct keyset walk rather than a post-filtered page.
  2. **Single run:** one controller helper `load_visible_run(repository, &principal, run_id) ->
     Result<Run, ApiError>` that fetches then checks `permits`, returning the **same
     `404 "unknown run '{id}'"`** as a missing row (no `403`, no timing/shape difference).
     `RunRepositoryPort::get` keeps its signature (the worker and services call it without a
     principal).
- **D-13:** `load_visible_run` is the entry point of **every** `/runs/{id}*` route: `GET
  /runs/{id}`, `GET /runs/{id}/stream` (before the SSE upgrade; the stream port is untouched),
  `GET /runs/{id}/webhook-deliveries` (`list_for_run` is untouched — the run is checked first),
  and also `POST /runs/{id}/cancel` — a foreign run must `404` on cancel too, otherwise cancel
  both leaks existence and permits a cross-tenant mutation. Cancel keeps its `allowed_roles`
  check inside `RunSubmissionService` after the visibility check. A route-level test enumerates
  every `/runs` route and proves each returns `404` for another tenant's run and `200`/`202` for
  its own, so a future route cannot be added without joining the list.
- **D-14:** `GET /runs` filters (`thread_id`, `assistant_id`, `status`) compose with the scope
  (AND). A thread owned by another tenant simply yields an empty page. Thread routes themselves
  (`/threads/{id}/state|history|resume|fork`, `DELETE /threads/{id}`) are **out of scope** and
  recorded as the next `WINDOWS.md` row (deferred idea) — they are named by PLAT-07 nowhere and
  threads carry no tenant.

**Ledger scope source (Phase 39 D-01 hand-off)**

- **D-15:** The worker (`src/application/services/run/worker.rs`, the `SettlementContext { scope:
  LedgerScope::unattributed(), .. }` site) builds the scope from `run.submitted_by`:
  `Some(a) => LedgerScope::new(a.tenant_id.as_str(), &a.api_key_id)`, `None =>
  LedgerScope::unattributed()`. One helper (`LedgerScope::from_attribution(Option<&
  RunAttribution>)` or equivalent) is the only place that mapping is written.
- **D-16:** Agent-kind runs (`run_agent` → `execute_scoped` with `RunScope`, 39-05/39-07) carry
  the same attribution: `RunScope` gains `ledger_scope: Option<LedgerScope>` (additive, `None`
  default), set by the worker from the run row, read by `settle_agent_loop_call` in
  `paladin_execution_service.rs` in place of its hard-coded `unattributed()`. The plain HTTP
  `/agents/{id}/execute[/stream]` path — which has a `Principal` but no run row — sets it from the
  principal in `agent_controller.rs`, so agent-execute spend is attributed too. The sentinel is
  stamped only when there is genuinely no principal (embedded library callers).
- **D-17:** The CLI `treasury spend` query defaults in `src/application/cli/commands/treasury.rs`
  are untouched; they are query-side filters, not writers.

**Breaking-change bookkeeping, surface and docs**

- **D-18:** `MIGRATION.md` rows written in the same commits as the code: §9.2 for `paladin-web
  Principal` (field added; `constructible_struct_adds_field`, marked `Y`, allowlist entry with
  `requirement_id = "TENANT-01"`), for `paladin-ports SubmitRun`/`ForkRun`/`RunSubmissionPort::
  cancel` (D-04, marked `Y`, allowlist entries), and for `paladin-ai-core Run` (additive under
  `#[non_exhaustive]`, marked `N`); §9.4 for migration `008`; §9.5 for the required `tenant` key
  and `bearer_token.tenant`; §9.6 for the read-scope behaviour change and the `RunResponse`
  addition. Where a lint is already crate-wide `allow`ed in `Cargo.toml`, the 39-08 D-27
  temporarily-disable-and-revert diagnostic confirms which entries the set-equality check needs.
- **D-19:** `RunResponse` gains an **additive** `#[serde(default)] submitted_by: Option<
  RunAttributionDto { tenant_id, api_key_id }>` (`null` for unattributed rows), mirroring the
  `cost` precedent in 39-06 including the `openapi_golden_v0_9.rs` sanctioned-exception row.
  Never the key value, never the role.
- **D-20:** One ADR, **ADR-0054** "Tenant identity is server-derived; run reads are tenant-scoped
  with an operator (Admin) bypass", recording D-02, D-05, D-11 and the `404`-not-`403` rule.
  `PROMOTION.md`'s next-free line advances to 0055 in the same commit.
- **D-21:** Docs and ledgers updated in-phase: `run_controller.rs` module docs ("Read scope
  (WR-03)" section rewritten to the new model), `docs/src/api-reference/platform-api.md`
  "Authentication and scopes" (the "What a `GET` can see today" paragraph replaced),
  `docs/src/deployment-topologies/http-service-host.md`, `config.example.yml`, the root
  `CHANGELOG.md` `[Unreleased]` (Added + Changed + Breaking), and `WINDOWS.md` row 32 closed via
  the tool (`fixed`), plus a new row for the deferred thread-route gap.

### Claude's Discretion

- Exact identifier bound and character rules for `TenantId` (mirror `ThreadId`).
- Whether `RUN_SCHEMA_VERSION` bumps `"v1"` → `"v2"`: follow the X-04 rule in `run.rs`'s module
  docs; the columns are nullable and serde-defaulted either way, so v1 rows must read back.
- Whether `RunReadScope`, `TenantId`, `PrincipalRef`, `RunAttribution` share one `principal.rs`
  module or split; the names above are defaults the planner may tighten, not contracts.
- How `AgentAuthConfig` (paladin-web) receives the bearer tenant: a field on `AgentAuthConfig`
  or a field beside `token_verifier` — planner's call; `has_credentials()` semantics unchanged.
- Test topology: whether the per-route `404` matrix (D-13) is one table test in
  `run_controller.rs` or an `http_surface_tests.rs` case; both must run under `cargo test -p
  paladin-web` without Docker.
- Migration `008` file naming beyond the `NNN_verb_noun.sql` convention; SQLite `ALTER TABLE ADD
  COLUMN` is idempotent under `sqlx::migrate!`'s version tracking, so no `IF NOT EXISTS` dance.

### Deferred Ideas (OUT OF SCOPE)

- **Thread-route read scoping** (`/threads/{id}/state|history|resume|fork`, `DELETE
  /threads/{id}`) — PLAT-07 names only `/runs*`; threads have no tenant column. Record as a new
  `WINDOWS.md` row in this phase; candidate for a hygiene phase or Phase 41 if allowance
  enforcement needs thread ownership.
- **Schedule-created runs inherit the schedule creator's principal** — needs `created_by` on the
  schedule row (`004` migration successor). Flagged for **Phase 41** planning: "refuse at
  admission on every run path" cannot evaluate an allowance for a principal-less run.
- **Per-API-key visibility narrower than tenant** and any org/RBAC beyond `Admin`/`User` —
  FUT-05 (v2).
- **Tenant registry / cross-check of allowance tenant ids against the key mapping** — Phase 41
  may add it when the `allowance` config lands (D-07).
- **CLI run inspection scoping** (`paladin-cli run …` reads the store directly, no principal) —
  operator surface, not a caller surface; out of scope.
- **Exposing `submitted_by` in herald/trace output** — not requested; `RunResponse` (D-19) and
  the ledger CLI are the operator views.
</user_constraints>

## Summary

Phase 40 is a **pure extension of code that already exists and already documents exactly where it
is incomplete**. `crates/paladin-web/src/run_controller.rs`'s own module docs (lines 40-62) already
name the gap this phase closes ("Read scope (WR-03) — deployment-wide, not per-caller") and point at
`WINDOWS.md` row 32, whose `description` field is a nearly word-for-word restatement of this phase's
goal. There is no framework research needed here — no new crate, no new external service, no new
protocol. The work is: (1) add a `tenant_id` to the existing `Principal`/`ApiKeyConfig`/
`AuthConfig` triple, (2) add two nullable columns to the existing `runs` table via a
`sqlx::migrate!`-embedded `008` migration, (3) add a `scope: RunReadScope` field to the existing
`RunQuery` struct and apply it in the three existing `RunRepositoryPort::list` adapters plus one new
`load_visible_run` helper, and (4) do the X-10 bookkeeping (`MIGRATION.md` §9.2/9.4/9.5/9.6,
`.cargo/semver-checks-allowlist.toml`) the project's own house style already prescribes for exactly
this shape of change (39-08 is the immediately-preceding precedent).

**Primary recommendation:** Follow the CONTEXT.md decisions literally — they are already correct
against the live tree, verified line-by-line in this research. The one genuine architectural gap
CONTEXT does not fully resolve is **D-16's agent-execute attribution**: the plain HTTP
`/agents/{id}/execute[/stream]` path in `agent_controller.rs` calls `Arc<dyn PaladinExecutorPort>`
(`crates/paladin-ports/src/output/paladin_executor_port.rs`), a deliberately simpler trait with only
`execute(paladin, input)` — **no `RunScope` parameter at all** — not `Arc<dyn PaladinPort>` (which
does have `execute_scoped`). Closing this gap needs one additive, defaulted trait method on
`PaladinExecutorPort` (see Pitfall 1 / Pattern 3 below); the planner must add this as an explicit
task, since CONTEXT's text ("sets it from the principal in `agent_controller.rs`") undersells the
plumbing required.

A second correction to the phase description's own assumptions: **`openapi_golden_v0_9.rs`'s
sanctioned-exception mechanism does not apply to `RunResponse` at all.** That golden file restricts
its whole comparison to six `/v1/agents*` paths (`V0_9_PATHS`, `openapi_golden_v0_9.rs:64-71`);
`/v1/runs*` was introduced in Phase 27 (v0.10), after the frozen v0.9.0 baseline, so it is not part
of that document and needs no exception there. The real regeneration step for `RunResponse.submitted_by`
is `crates/paladin-web/src/openapi.rs`'s `openapi_matches_committed_baseline` test — run
`UPDATE_OPENAPI=1 cargo test -p paladin-web openapi_matches_committed_baseline` (or `make openapi`)
to refresh the HEAD-tracking baseline `tests/fixtures/openapi.json`. See Pitfall 6.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tenant identity resolution (API key/bearer → tenant) | API / Backend (`paladin-web::agent_auth`) | Config (`src/config/agents.rs`) | `authenticate()` is the one place a credential is turned into a `Principal`; config supplies the mapping table |
| Tenant assertion prevention | API / Backend (`agent_auth::authenticate`) | — | Server-derived only; `SubmitRunRequest` gains no tenant field (D-02) |
| Run attribution (who submitted it) | API / Backend → Application service (`RunSubmissionService::submit`/`fork`) | Database / Storage (`runs` row) | Principal → `PrincipalRef` conversion happens at the controller boundary; the service writes it onto the `Run` aggregate; storage persists it |
| Run-read authorization | API / Backend (`load_visible_run`, `RunQuery.scope`) | Database / Storage (`WHERE tenant_id = ?` in `list`) | The "one shared function" (D-12) lives in `paladin-web`; the SQL adapters apply the resulting filter so pagination stays correct at the SQL layer, not post-filtered in Rust |
| Ledger scope sourcing | Application service (`worker.rs` `SettlementContext`, `paladin_execution_service.rs::settle_agent_loop_call`) | Database / Storage (`treasury_ledger` rows) | Phase 39 already owns the ledger schema/port; Phase 40 only changes what value is threaded into `LedgerScope::new(...)` at these two call sites |
| Breaking-change bookkeeping | Docs / Governance (`MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, ADR-0054) | — | No runtime tier; a project-process concern |

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TENANT-01 | Operator config maps each API key to a tenant; `Principal` carries `tenant_id`; caller cannot assert its own tenant; break recorded in MIGRATION.md §9.2 + semver-checks allowlist | Pattern 1 (identity carrier), Pattern 2 (config mapping), §9.2/9.5 row formats confirmed against live `MIGRATION.md`; `.cargo/semver-checks-allowlist.toml` schema confirmed; all three crates' `Cargo.toml` lint tables read (Pitfall 5) |
| TENANT-02 | Every run records its submitting principal (API key id + tenant) | Pattern 4 (run-row attribution), exact `Run`/migration/adapter call sites in Code Examples |
| PLAT-07 | `GET /runs` + every `/runs/{id}*` route scoped to caller; another caller's run is 404; one shared authorization function | Pattern 5 (`load_visible_run`), exact route list from `run_openapi_router` (run_controller.rs:1114-1129), existing 404 literal confirmed (`unknown run '{id}'`, run_controller.rs:631) |
</phase_requirements>

## Standard Stack

No new libraries. This phase is additive Rust types + one SQL migration inside crates already in
the workspace (`paladin-core`, `paladin-ports`, `paladin-web`, `paladin-storage`, the `paladin-ai`
facade). `sqlx` (already pinned, `migrate` + `macros` features already enabled per
`MIGRATION.md` §9.3) is the only "dependency" touched, and it is untouched at the `Cargo.toml`
level — only a new migration file.

**Installation:** none.

**Version verification:** N/A — no new package.

## Package Legitimacy Audit

**Not applicable.** This phase installs zero external packages. `TenantId`, `PrincipalRef`,
`RunAttribution`, `RunReadScope` are all pure in-tree newtypes added to
`crates/paladin-core/src/platform/container/`. No `npm view`/`pip index`/`cargo search` check is
needed; there is nothing to verify against a registry.

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
Caller (API key or bearer token)
        │  X-API-Key / Authorization: Bearer …
        ▼
┌───────────────────────────────────────────────────────────────────┐
│ paladin-web :: agent_auth::authenticate()                         │
│  API-key branch: lookup_api_key(config.api_keys, key)             │
│    → Principal { id, role, tenant_id: <config's key→tenant map> } │  ← D-02: tenant NEVER
│  Bearer branch:  verifier.verify_token(token)                     │     comes from the request
│    → Principal { id: claims.user_id, role, tenant_id: <config's  │
│         http.auth.bearer_token.tenant> }                          │
│  Open-access (auth disabled): tenant_id = TenantId::OPEN_ACCESS   │
└───────────────────────────┬───────────────────────────────────────┘
                             │ Extension<Principal> injected on every request
             ┌───────────────┴────────────────┐
             ▼                                 ▼
   POST /runs, /cancel                 GET /runs, GET /runs/{id},
   (submit_run, cancel_run)            /stream, /webhook-deliveries
             │                                 │
             ▼                                 ▼
  PrincipalRef::from(&principal)     Principal::read_scope() -> RunReadScope
  → SubmitRun.requested_by            { All | Tenant(tenant_id) }
             │                                 │
             ▼                                 ▼
  RunSubmissionService::submit/fork   list_runs: RunQuery { scope, .. }
    .authorize_invocation(role)        → RunRepositoryPort::list()
    Run::with_submitted_by(attr)         (SQL: AND tenant_id = ?)
             │                          get_run/stream_run/cancel_run:
             ▼                            load_visible_run(repo, principal, id)
  RunRepositoryPort::insert()              .get(id) then permits(&run)
    (tenant_id, api_key_id columns)        → same 404 "unknown run '{id}'"
             │                                 on miss OR on hidden-run
             ▼
  Worker (worker.rs run_once/run_agent)
    SettlementContext.scope =
      LedgerScope::from_attribution(run.submitted_by)
    RunScope.ledger_scope = Some(LedgerScope::from_attribution(...))
             │
             ▼
  TreasuryLedgerPort::settle() — real tenant/api_key, not "unattributed"
```

### Recommended Project Structure

No new modules beyond what CONTEXT already names. New/changed files:

```
crates/paladin-core/src/platform/container/
├── principal.rs           # NEW: TenantId, PrincipalRef, RunAttribution, RunReadScope
│                           #   (CONTEXT gives discretion to split; one file is simplest —
│                           #    all four types are small and mutually referential)
├── run.rs                 # Run gains submitted_by: Option<RunAttribution>, with_submitted_by()
├── run_scope.rs            # RunScope gains ledger_scope: Option<LedgerScope>, with_ledger_scope()
└── treasury_ledger.rs      # LedgerScope::from_attribution(Option<&RunAttribution>) helper

crates/paladin-web/src/
├── agent_auth.rs           # Principal { id, role, tenant_id }; AgentAuthConfig gains
│                           #   bearer_tenant field (or similar); Principal::read_scope()
└── run_controller.rs       # load_visible_run() helper; get_run/stream_run/cancel_run/
                            #   list_webhook_deliveries route through it; RunResponse.submitted_by

crates/paladin-ports/src/
├── input/run_submission_port.rs   # SubmitRun/ForkRun.requested_by: Option<PrincipalRef>;
│                                  #   cancel(.., requested_by: Option<PrincipalRef>)
└── output/run_repository_port.rs  # RunQuery gains scope: RunReadScope (Default = All)

crates/paladin-storage/
├── migrations/{sqlite,postgres}/008_add_run_attribution_columns.sql   # NEW
└── src/run/{sqlite,postgres,in_memory}.rs   # INSERT/SELECT/list column + WHERE additions

src/config/agents.rs        # ApiKeyConfig.tenant: String; BearerTokenAuthConfig.tenant: Option<String>;
                             #   AuthConfig::validate() (NEW — none exists today)
src/bin/paladin-server.rs   # build_auth_config: parse+attach tenant onto Principal + bearer tenant
src/application/services/run/
├── submission.rs           # authorize_invocation reads PrincipalRef.role; submit/fork set
│                           #   run.with_submitted_by(requested_by.as_ref().map(|p| p.attribution()))
└── worker.rs                # SettlementContext.scope from run.submitted_by; RunScope.ledger_scope

crates/paladin-ports/src/output/paladin_executor_port.rs   # NEW defaulted execute_scoped method
                                                             #   (see Pitfall 1)
```

### Pattern 1: Identity carrier — `Principal` gains a required `tenant_id`

**What:** `Principal` (`crates/paladin-web/src/agent_auth.rs:37-42`) today is:
```rust
#[derive(Debug, Clone)]
pub struct Principal {
    pub id: String,
    pub role: UserRole,
}
```
D-01 adds a required (non-`Option`) `tenant_id: TenantId` field. `Principal` is **not**
`#[non_exhaustive]` today, so this is a genuine breaking struct-literal change — but see Pitfall 5:
`paladin-web`'s `Cargo.toml` already crate-wide `allow`s `constructible_struct_adds_field`
(confirmed at `crates/paladin-web/Cargo.toml:92-95`).

**Every production construction site to update** (confirmed via `grep -rn "Principal {"` across
`{src,crates,examples}`):
- `crates/paladin-web/src/agent_auth.rs:147` — bearer-token branch of `authenticate()`
- `crates/paladin-web/src/agent_auth.rs:261` — API-key path via `lookup_api_key`'s stored map value (constructed in `src/bin/paladin-server.rs:372`, see Pattern 2)
- `crates/paladin-web/src/agent_auth.rs:44-51` — `Principal::open_access()` (the `fn open_access()` body itself)
- `src/bin/paladin-server.rs:372` — `build_auth_config`'s `api_keys` map construction

**Every test-only construction site** (must also gain `tenant_id`, ~14 sites found):
`agent_auth.rs:261,349,365,369`; `agent_controller.rs:822,830`; `run_controller.rs:1852,2286`;
`assistant_controller.rs:932,939`; `thread_controller.rs:1181,1749,1756,2299`;
`schedule_controller.rs:760,767`; plus `examples/http_service_host.rs:62` and
`examples/platform_api_client.rs:144` (these are doc/example code, not `#[cfg(test)]`, and must
compile — `cargo build --examples --features web-server,dev-ui`).

**Example (current `tester_principal()` helper, run_controller.rs:1851-1856):**
```rust
fn tester_principal() -> Extension<Principal> {
    Extension(Principal {
        id: "tester".to_string(),
        role: paladin_core::platform::container::user::UserRole::Admin,
    })
}
```
Needs `tenant_id: TenantId::new("tester-tenant").unwrap()` (or similar) added — and per D-13's
per-route 404 matrix, tests will also need a *second* principal in a *different* tenant to prove
cross-tenant 404s.

**When to use:** every `Principal` construction, everywhere, no exceptions — there is no `Option`
per D-01.

### Pattern 2: Config mapping — `ApiKeyConfig`/`AuthConfig`/`build_auth_config`

**What:** `ApiKeyConfig` (`src/config/agents.rs:80-88`) today:
```rust
pub struct ApiKeyConfig {
    pub key: String,
    pub name: String,
    pub role: UserRole,
}
```
D-05/D-06 add a required `tenant: String` field, and a NEW `AuthConfig::validate()` (there is
**no `validate()` on `AuthConfig` today** — confirmed by reading the whole file, `src/config/agents.rs:102-127`).
`BearerTokenAuthConfig` (`agents.rs:91-96`) gains a `tenant: Option<String>` field, required when
`enabled: true` (validated at the same `validate()` call).

**`build_auth_config`** (`src/bin/paladin-server.rs:354-415`) is the exact fail-closed precedent
D-00d cites. Today's `api_keys` construction (`paladin-server.rs:366-378`):
```rust
let api_keys: HashMap<String, Principal> = cfg
    .api_keys
    .iter()
    .map(|k| (k.key.clone(), Principal { id: k.name.clone(), role: k.role }))
    .collect();
```
becomes `Principal { id: k.name.clone(), role: k.role, tenant_id: <parsed from k.tenant> }`. The
bearer-token branch (`paladin-server.rs:384-389`) currently builds `Some(Arc::new(InMemoryTokenAuthAdapter::new()))`
with no tenant info threaded anywhere — `AgentAuthConfig` needs a place to carry the configured
bearer tenant (CONTEXT leaves the exact field placement to the planner) so `authenticate()`'s
bearer branch (`agent_auth.rs:144-151`) can read it.

**Existing tests to extend:** `src/bin/paladin-server.rs:695-744` (`build_auth_config_warns_when_in_process_token_store_is_wired`,
`build_auth_config_fails_closed_when_enabled_with_no_credentials`); `src/config/agents.rs:246-339`
(6 tests including `auth_config_parses_api_keys_with_roles`, `auth_config_rejects_unknown_role` —
the tenant-required-with-message test is new, matching `auth_config_rejects_unknown_role`'s shape).

**Error voice (existing `validate()` precedent, `src/config/run_store.rs:103-126`):**
```rust
pub fn validate(&self) -> Result<(), String> {
    ...
    Err(format!("run store postgres backend names env var '{url_env}', which is not set"))
}
```
CONTEXT's `<specifics>` section already supplies the exact target message:
`"http.auth.api_keys[<name>]: 'tenant' is required — every API key must map to a tenant (Phase 40, TENANT-01)"`.

### Pattern 3: The `PaladinExecutorPort` gap (D-16's real plumbing cost)

**What CONTEXT under-specifies:** `agent_controller.rs::execute_agent` (line 300-327) and
`execute_agent_stream` (line 586+) call `entry.executor.execute(paladin, input)` where
`entry.executor: Arc<dyn PaladinExecutorPort>` (`AgentEntry.executor`,
`crates/paladin-web/src/agent_registry.rs:33-40`). `PaladinExecutorPort`
(`crates/paladin-ports/src/output/paladin_executor_port.rs:60-73`) is a **deliberately minimal**
trait — `async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError>`
— with **no scope parameter of any kind**. This is architecturally distinct from `PaladinPort`
(`crates/paladin-ports/src/output/paladin_port.rs`), which already has `execute_scoped` with a
defaulted body delegating to `execute` (`paladin_port.rs:985`, referenced by
`worker.rs`'s own comment at line 1251: *"for any port that does not override `execute_scoped`,
the trait's default body delegates straight to `execute`"*). `PaladinExecutionService`'s inherent
`execute()` method (`paladin_execution_service.rs:1163-1170`) already internally calls
`self.execute_scoped(paladin, input, None, &RunScope::default())` — but `PaladinExecutorPort`'s
impl for it (`paladin_execution_service.rs:3174-3179`) just calls the inherent `execute`, which
always passes `RunScope::default()` — **there is no way for `agent_controller.rs` to inject a
non-default `RunScope` through the `PaladinExecutorPort` trait object as it exists today.**

**Recommended fix (mirrors the `PaladinPort::execute_scoped` precedent exactly, and the
`RunRepositoryPort::insert_with_latest` X-10.4 default-method precedent,
`run_repository_port.rs:338-368`):** add one new, **additive, defaulted** trait method to
`PaladinExecutorPort`:
```rust
async fn execute_scoped(
    &self,
    paladin: &Paladin,
    input: &str,
    scope: &RunScope,
) -> Result<PaladinResult, PaladinError> {
    let _ = scope; // default ignores scope, behavior-identical for any implementor that doesn't override
    self.execute(paladin, input).await
}
```
`PaladinExecutionService`'s impl overrides it to call its own inherent `execute_scoped`. This is
**non-breaking** for any existing `PaladinExecutorPort` implementor (X-10.4: adding a defaulted
trait method never breaks a pre-existing implementor) — `crates/paladin-web/src/agent_registry.rs`'s
own `StubExecutor` test double (line 277-297) and `HandoffService`'s implementors are unaffected.
`agent_controller.rs::execute_agent`/`execute_agent_stream` then build
`RunScope::default().with_ledger_scope(principal.ledger_scope())` and call `execute_scoped`
instead of `execute`. This requires a `RunScope` builder addition too (`with_ledger_scope`, mirroring
the existing `with_run_id`/`with_vault_namespace` pattern, `run_scope.rs:95-119`).

**Why this matters:** without this, D-16's success criterion ("agent-execute spend is attributed
too") is unreachable through the code path the phase description names — the planner must budget a
task for this trait addition, not assume it's a one-line change inside `agent_controller.rs`.

### Pattern 4: Run-row attribution and the `008` migration

**What:** `Run` (`crates/paladin-core/src/platform/container/run.rs:355-410`) is
`#[non_exhaustive]` with a builder (`Run::new` + `with_*`, lines 412-460) — confirmed. D-08 adds:
```rust
#[serde(default)]
pub submitted_by: Option<RunAttribution>,
```
plus `Run::with_submitted_by(mut self, attribution: RunAttribution) -> Self`.

**X-04 schema-version rule — CONFIRMED, does NOT require a bump.** The exact governing precedent
(`crates/paladin-core/src/platform/container/waypoint.rs:1805-1811`, a passing test today):
```rust
fn battlefield_schema_version_is_unchanged() {
    // Every Phase 25 Waypoint addition is `#[serde(default)]`, so the
    // schema version pinned on `main` before this phase must still hold
    // (X-04 does not require a bump for a purely additive change).
    assert_eq!(BATTLEFIELD_SCHEMA_VERSION, "1.0.0");
```
`RUN_SCHEMA_VERSION` stays `"v1"`. This is reinforced by the SQLite adapter's own strict-equality
read guard (`crates/paladin-storage/src/run/sqlite.rs:285-289`):
```rust
if schema_version != RUN_SCHEMA_VERSION {
    return Err(RunRepositoryError::UnknownSchemaVersion { found: schema_version });
}
```
Bumping the constant would make every row written before the bump fail to read back — the opposite
of what an additive, nullable column needs. **Do not bump `RUN_SCHEMA_VERSION`.**

**Migration `008` — exact precedent shape.** `crates/paladin-storage/migrations/sqlite/002_create_runs_table.sql`
and `crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql` (both read in
full this session) establish the header-comment style: `-- Migration: <title>`, `-- Purpose: ...`,
`-- Version: NNN`, `-- Date: YYYY-MM-DD`, then rationale paragraphs, then DDL, then indexes with
one-line "serves ..." comments. `sqlx::migrate!` embeds every file under `migrations/{sqlite,postgres}/`
at compile time and tracks applied versions in its own bookkeeping table
(`SqliteRunRepository::new`, `sqlite.rs:110-129`: `MIGRATOR.run(&pool).await`) — **a migration file
runs exactly once per database, regardless of its own SQL idempotency.** This resolves CONTEXT's
own flagged uncertainty: SQLite's `ALTER TABLE ... ADD COLUMN` needs **no** `IF NOT EXISTS` dance
inside a versioned `sqlx::migrate!` file (unlike the ad-hoc, non-versioned runtime pattern at
`crates/paladin-storage/src/sqlite_user_repository.rs:87-91`, which manually swallows the
duplicate-column error because it runs its `ALTER TABLE` on every construction, not through
`sqlx::migrate!`). A cross-crate versioned-migration precedent for exactly this DDL shape exists at
`crates/paladin-memory/migrations/002_add_garrison_is_summary.sql:8`:
`ALTER TABLE garrison_entries ADD COLUMN is_summary INTEGER NOT NULL DEFAULT 0;` — plain, no guard,
inside a `sqlx::migrate!`-tracked file.

**SQLite `008` file (concrete):**
```sql
-- Migration: Add Run Attribution Columns
-- Purpose: Persist the submitting principal's tenant/API-key on each run (TENANT-01, TENANT-02)
-- Version: 008
-- Date: <plan date>
--
-- NULL means "no principal recorded" (pre-v0.11 rows, schedule-fired runs, same-process
-- embedders, tests) -- never the ledger's "unattributed" string sentinel (D-09; that sentinel is
-- a treasury_ledger-row concept, LedgerScope::UNATTRIBUTED, not a runs-row value).

ALTER TABLE runs ADD COLUMN tenant_id TEXT NULL;
ALTER TABLE runs ADD COLUMN api_key_id TEXT NULL;

-- Serves the scoped keyset list (D-12): WHERE tenant_id = ? combined with the existing
-- (submitted_at DESC, run_id DESC) ordering.
CREATE INDEX IF NOT EXISTS idx_runs_tenant_submitted
ON runs(tenant_id, submitted_at DESC, run_id DESC);
```
Postgres `008` is textually identical (Postgres `ALTER TABLE ADD COLUMN` has no dialect difference
here; `CREATE INDEX IF NOT EXISTS` behaves the same on both).

**Adapter changes (all three, confirmed against live code this session):**
- `crates/paladin-storage/src/run/sqlite.rs`: `INSERT_RUN`/`INSERT_RUN_WITH_LATEST`
  (lines 44-48, 81-86), `SELECT_RUN_BY_ID`/`SELECT_ACTIVE_RUN_FOR_THREAD`/`LIST_SELECT_PREFIX`
  (lines 50-70) each gain `tenant_id, api_key_id` columns; `row_to_run` (lines 221-314) reads them
  as `Option<String>` and constructs `Option<RunAttribution>`; `insert` (lines 319-378) binds
  `run.submitted_by.as_ref().map(|a| a.tenant_id.as_str())` etc.
- `list()` (`sqlite.rs:470-526`): the `QueryBuilder` push chain (lines 479-500) gains, after the
  existing `thread_id`/`assistant_id`/`status` filters and before the cursor clause:
  ```rust
  if let RunReadScope::Tenant(tenant) = &query.scope {
      builder.push(" AND tenant_id = ");
      builder.push_bind(tenant.as_str().to_string());
  }
  ```
  This keeps the keyset walk correct — the filter is inside the same `QueryBuilder`, before
  `ORDER BY ... LIMIT`, exactly like `thread_id`/`assistant_id`/`status` already are (Pitfall 2).
- `crates/paladin-storage/src/run/postgres.rs` mirrors this 1:1 (`LIST_SELECT_PREFIX`/`list()` at
  postgres.rs:76-79, 455-467 confirmed to share the identical `QueryBuilder` shape).
- `crates/paladin-storage/src/run/in_memory.rs`: `list()` (lines 151-165) is a plain iterator
  `.filter()` chain — add `.filter(|r| query.scope.permits(r))` alongside the existing
  `thread_id`/`assistant_id`/`status` filters (lines 155-162).
- `crates/paladin-storage/src/run/contract_tests.rs`: extend the existing
  `list_filters_by_thread_assistant_and_status` clause (referenced from `in_memory.rs:523-526`)
  with a scoped-list case, plus a new attribution round-trip case (insert with `submitted_by`,
  read back, assert equal) — run against all three adapters per the house pattern.

### Pattern 5: The one shared authorization function — `RunReadScope` + `load_visible_run`

**What:** New core type (module discretion per CONTEXT — `principal.rs` alongside `TenantId`):
```rust
pub enum RunReadScope {
    All,
    Tenant(TenantId),
}
impl RunReadScope {
    pub fn for_principal(role: UserRole, tenant_id: &TenantId) -> Self {
        if role == UserRole::Admin { Self::All } else { Self::Tenant(tenant_id.clone()) }
    }
    pub fn permits(&self, run: &Run) -> bool {
        match self {
            Self::All => true,
            Self::Tenant(t) => run.submitted_by.as_ref().is_some_and(|a| &a.tenant_id == t),
        }
    }
}
```
`Principal::read_scope()` lives beside `authorize_invoke`/`require_admin` in `agent_auth.rs`
(`agent_auth.rs:195-220` is exactly that neighborhood today).

**`RunQuery` addition** (`crates/paladin-ports/src/output/run_repository_port.rs:47-60`):
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunQuery {
    pub thread_id: Option<ThreadId>,
    pub assistant_id: Option<String>,
    pub status: Option<RunStatus>,
    pub limit: u32,
    pub cursor: Option<RunCursor>,
    pub scope: RunReadScope,   // NEW; Default = All (internal callers unchanged)
}
```
`RunReadScope` needs a `Default` impl returning `All` — every non-web internal caller of `.list()`
(there are several in `submission.rs::fork`, `schedule/service.rs`, tests) is unaffected because
`RunQuery { .. }`/`RunQuery::default()` already appear throughout the codebase with struct-update
syntax (confirmed via the `fork` call site, `submission.rs:414-418`, and 39/40+ test call sites).

**`load_visible_run`** — the single controller helper D-12/D-13 mandate, in `run_controller.rs`:
```rust
async fn load_visible_run(
    repository: &Arc<dyn RunRepositoryPort>,
    principal: &Principal,
    run_id: &RunId,
) -> Result<Run, ApiError> {
    let run = repository.get(run_id).await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?;
    if !principal.read_scope().permits(&run) {
        return Err(ApiError::not_found(format!("unknown run '{run_id}'")));
    }
    Ok(run)
}
```
The 404 literal is copied **verbatim** from the existing miss-path
(`run_controller.rs:817`: `.ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?`)
— same string, same `ApiError::not_found` constructor, so a scoped miss is byte-identical to a
genuine miss (D-12's "no 403, no timing/shape difference").

**Every `/runs/{id}*` route to route through it** (confirmed exhaustively from
`run_openapi_router`, `run_controller.rs:1114-1129`):
```rust
pub fn run_openapi_router(state: RunApiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(submit_run))          // POST /runs — NOT visibility-gated (new run)
        .routes(routes!(list_runs))           // GET /runs — RunQuery.scope, not load_visible_run
        .routes(routes!(get_run))             // GET /runs/{id}            → load_visible_run
        .routes(routes!(stream_run))          // GET /runs/{id}/stream     → load_visible_run (before SSE upgrade)
        .routes(routes!(cancel_run))          // POST /runs/{id}/cancel    → load_visible_run, THEN role check
        .routes(routes!(list_webhook_deliveries))  // GET /runs/{id}/webhook-deliveries → load_visible_run first
        .merge(crate::assistant_controller::assistant_routes())
        .merge(crate::schedule_controller::schedule_routes())
        ...
}
```
Four handlers change: `get_run` (currently `repository.get(&id)...ok_or_else(...)`,
`run_controller.rs:813-817`), `stream_run` (currently no visibility check at all before
`run_events.stream(&id)`, `run_controller.rs:1073-1104`), `cancel_run` (currently
`submission.cancel(&id, Some((principal.id.clone(), principal.role)))` with no prior visibility
check, `run_controller.rs:938-962`), `list_webhook_deliveries` (currently `deliveries.list_for_run(&id, ...)`
with no run-visibility check at all, `run_controller.rs:988-1017` — `list_for_run` itself is
"untouched", per D-13, meaning `load_visible_run` gates it from *outside*, not by changing the
webhook port).

### Anti-Patterns to Avoid

- **Post-filtering a fetched page in Rust instead of `WHERE tenant_id = ?` in SQL.** Breaks the
  documented keyset-pagination contract (`(submitted_at DESC, run_id DESC)`, D-47's own "not a
  point-in-time snapshot" caveat, `run_controller.rs:829-833`): a post-filtered page can under-fill
  (`limit: 20` requested, fewer than 20 tenant-visible rows returned even though more exist) or
  desync the `next_cursor` from what was actually returned. The filter MUST sit inside the same
  `QueryBuilder` chain, before `ORDER BY ... LIMIT` (Pattern 4).
- **Returning `403 Forbidden` for a hidden cross-tenant run.** D-12 is explicit: same `404`, same
  message, as a genuinely missing run. A `403` (or any observable timing/shape difference) leaks
  the run's existence to a caller who should not even know it exists.
- **Bumping `RUN_SCHEMA_VERSION`.** Confirmed unnecessary and actively harmful (Pattern 4) — the
  SQLite adapter's `row_to_run` strict-equality check would then reject every pre-bump row.
  X-04's own passing test (`waypoint.rs:1805-1811`) is the house precedent that additive changes
  never bump a schema-version constant.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tenant/API-key identifier validation | A bespoke regex or manual char-loop | Mirror `ThreadId::new` exactly (`waypoint.rs:63-77`: `Empty` / `TooLong(THREAD_ID_MAX_LEN=256)` / `ContainsWhitespace` — three checks, one error enum) | Established house pattern for every identifier newtype in this codebase (`RunId`, `ReservationId`, `ThreadId` all share the shape); reusing it avoids inventing a fourth validation rule set |
| Cross-tenant visibility check | A `HashSet<TenantId>` membership check scattered per-route | `RunReadScope::permits(&Run) -> bool`, one function, applied at exactly two call sites (D-12) | The whole point of PLAT-07's "one shared authorization function" requirement — a per-route ad-hoc check is precisely the defect `WINDOWS.md` row 32 and research Pitfall 10 (secondary-route scoping bypass) warn about |
| Keyset pagination with a tenant filter | A second, tenant-scoped index maintained by hand plus manual cursor math | Extend the existing `(submitted_at DESC, run_id DESC)` covering index to lead with `tenant_id` (`idx_runs_tenant_submitted`), reusing the identical cursor comparison logic already in `list()` | The existing cursor logic (`sqlite.rs:492-500`) already handles the `(submitted_at, run_id)` tiebreak correctly; adding a leading `tenant_id` equality predicate before it is a one-line `QueryBuilder.push` — no new pagination algorithm needed |

**Key insight:** every piece of infrastructure this phase needs — identifier newtypes, dynamic SQL
filter composition, additive `#[non_exhaustive]` struct evolution, fail-closed config validation —
already has exactly one canonical, working example in this codebase. The task is replication, not
invention.

## Common Pitfalls

### Pitfall 1: `PaladinExecutorPort` has no scope parameter (D-16's silent gap)
**What goes wrong:** A planner reads D-16 ("sets it from the principal in `agent_controller.rs`")
and assumes it's a one-line change inside the handler. It is not — the trait object
(`Arc<dyn PaladinExecutorPort>`) the handler calls has no method that accepts a `RunScope` at all.
**Why it happens:** `PaladinExecutorPort` (paladin-ports) and `PaladinPort` (paladin-ports) are two
separate traits for two separate purposes (`HandoffService` delegation vs. the engine's node
dispatch) that happen to have converged on similar-looking `execute`/`execute_scoped` shapes on
`PaladinPort` only.
**How to avoid:** Add a defaulted `execute_scoped` method to `PaladinExecutorPort` (Pattern 3) as
its own explicit task, verified by a test that a `RunScope` passed to `execute_agent` actually
reaches the ledger settle call.
**Warning signs:** A plan that touches only `agent_controller.rs` for D-16 and never touches
`crates/paladin-ports/src/output/paladin_executor_port.rs`.

### Pitfall 2: Post-filtering `RunQuery.scope` after `list()` returns
**What goes wrong:** Filtering the returned `RunPage.items` by tenant in `paladin-web` (rather than
in each adapter's SQL) silently breaks `limit`/`next_cursor` correctness under a mixed-tenant table
— exactly the class of bug D-12 explicitly forbids by requiring the filter live inside
`RunRepositoryPort::list`'s own SQL.
**Why it happens:** It is much less code to add one `.filter()` in `run_controller.rs::list_runs`
than to touch three adapter files.
**How to avoid:** `scope` must be a `RunQuery` field, consumed inside each adapter's own query
builder (Pattern 4), never a post-processing step in the controller.
**Warning signs:** `RunQuery.scope` exists but `crates/paladin-storage/src/run/{sqlite,postgres,in_memory}.rs`
are unmodified.

### Pitfall 3: SQLite `ALTER TABLE ADD COLUMN` re-run anxiety
**What goes wrong:** Adding `IF NOT EXISTS`-style guards (which SQLite's `ADD COLUMN` doesn't
support) or hand-rolled duplicate-column error swallowing (copying `sqlite_user_repository.rs:87-91`'s
pattern) into the NEW versioned `008` migration file, when that pattern exists there specifically
*because* that file's `ALTER TABLE` runs on every construction, outside `sqlx::migrate!`.
**Why it happens:** Surface-pattern-matching on the one other `ALTER TABLE` example in the tree
without checking whether it's inside a tracked migration or not.
**How to avoid:** `008` is a normal `sqlx::migrate!`-embedded file (`static MIGRATOR: ... = sqlx::migrate!("migrations/sqlite")`,
`sqlite.rs:91`) — it runs exactly once per database by `sqlx`'s own applied-version bookkeeping. No
guard needed; `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql:8` is the correct,
unguarded precedent to copy.
**Warning signs:** `008_*.sql` contains a `PRAGMA` check, a `CREATE TABLE ... AS SELECT` rebuild
dance, or any conditional logic at all.

### Pitfall 4: Confusing `LedgerScope::UNATTRIBUTED` with a run's `submitted_by = None`
**What goes wrong:** Persisting the string `"unattributed"` into the new `runs.tenant_id`/`api_key_id`
columns for schedule-fired/internal runs, instead of leaving them `NULL`.
**Why it happens:** The two sentinels look similar and are documented in the same phase.
**How to avoid:** D-09 is explicit: `NULL` on the `runs` row means "no principal recorded";
`LedgerScope::UNATTRIBUTED` (`"unattributed"`, `treasury_ledger.rs:63`) is a **ledger-row-only**
concept, produced by `LedgerScope::from_attribution(None)`, never written to the `runs` table
directly.
**Warning signs:** Any code path that writes the literal string `"unattributed"` into `runs.tenant_id`.

### Pitfall 5: Assuming a new `.cargo/semver-checks-allowlist.toml` entry is always needed
**What goes wrong:** Adding a `[[entry]]` for every §9.2 row without first checking whether the
specific lint that would fire is already crate-wide `allow`ed.
**Why it happens:** The register-discipline language ("the §9.2 row, the allowlist entry and the
Cargo.toml lint line land in one commit") reads as "always add all three."
**How to avoid:** All three touched crates ALREADY crate-wide `allow` `constructible_struct_adds_field`
— confirmed this session: `paladin-web/Cargo.toml:92-95` (`constructible_struct_adds_field = "allow"`
among four lints), `paladin-ports/Cargo.toml:66-69` (same lint, three total), `paladin-core/Cargo.toml:73-78`
(same lint, five total, plus `struct_pub_field_missing`). Per the 39-08 D-27 precedent (STATE.md:
*"no new `.cargo/semver-checks-allowlist.toml` entry or `Cargo.toml` lint-table line was needed
anywhere — every lint the 0.10.1 diagnostic run reported ... was already covered by an existing
crate-wide suppression, confirmed by a temporarily-disabled-and-reverted diagnostic"*), run the SAME
diagnostic before assuming a new entry is required: temporarily comment out the relevant `allow`
line, run `cargo semver-checks check-release` against the pinned baseline, observe which lint (if
any) fires for `Principal`/`Run`/`RunQuery`/`SubmitRun`/`ForkRun`, then revert. **Still write every
§9.2 row** regardless of the diagnostic outcome (X-10 register discipline is about the record, not
just the tool suppression) — but only add a NEW allowlist `[[entry]]` if the diagnostic shows the
existing blanket `allow` does NOT already cover it (e.g. a trait-signature change like
`RunSubmissionPort::cancel`'s parameter type swap, or `SubmitRun`/`ForkRun`'s field TYPE change
rather than a field ADDITION, may trigger a different, not-yet-allowed lint class).

### Pitfall 6: Reaching for `openapi_golden_v0_9.rs` for `RunResponse`
**What goes wrong:** Adding a `strip_known_...` exception function to
`crates/paladin-web/tests/openapi_golden_v0_9.rs` for `RunResponse.submitted_by`, mirroring 39-06's
`CostDto`/`ExecuteResponse` precedent literally.
**Why it happens:** The phase description explicitly suggests this ("the same way", "including the
sanctioned-exception row"), and 39-06 really did add exactly that kind of row — for `ExecuteResponse`.
**How to avoid:** `V0_9_PATHS` (`openapi_golden_v0_9.rs:64-71`) is fixed to six `/v1/agents*` paths;
`/v1/runs*` is entirely absent from both that constant and the frozen v0.9.0 baseline fixture
(`RunResponse` postdates v0.9.0 by a full milestone, Phase 27). The correct, and only necessary,
regeneration step is `crates/paladin-web/src/openapi.rs`'s HEAD-tracking drift guard
(`openapi_matches_committed_baseline`, `openapi.rs:263-281`): run `UPDATE_OPENAPI=1 cargo test -p
paladin-web openapi_matches_committed_baseline` (or `make openapi`) after adding the field.
**Warning signs:** A diff touching `openapi_golden_v0_9.rs`'s `strip_known_v0_10_execute_response_divergence`
function for a `RunResponse`-only field.

### Pitfall 7: Test-topology claim in CONTEXT's discretion section doesn't match crate boundaries
**What goes wrong:** CONTEXT's discretion text says the per-route 404 matrix, if placed in
`http_surface_tests.rs`, "must run under `cargo test -p paladin-web`". That file
(`src/application/services/run/http_surface_tests.rs`) is a module of the root **facade** crate
(`Cargo.toml:56-58`: `[package] name = "paladin-ai"`, lib name `paladin` at `Cargo.toml:100-101`),
**not** `paladin-web` — it is compiled and tested via `cargo test -p paladin-ai` (or a bare
`cargo test` at the workspace root), not `-p paladin-web`.
**Why it happens:** Both files sit under a directory literally named `run/` and both exercise
`run_router`, inviting the assumption they're in the same crate.
**How to avoid:** If the 404 matrix needs `cargo test -p paladin-web` specifically (e.g. to run fast
in a paladin-web-only CI job), it must live as an in-crate test in `run_controller.rs`. If it needs
the REAL `RunSubmissionService`/`SqliteRunRepository` facade wiring (as `http_surface_tests.rs`'s
existing `ten_concurrent_submits_one_accepted` does, over a real on-disk SQLite temp file, Tier 1,
no Docker), it belongs in `http_surface_tests.rs` and runs under `cargo test -p paladin-ai` (still
no Docker — Tier 1, matching the file's own documented convention).
**Warning signs:** A plan verification step that runs `cargo test -p paladin-web` and expects it to
cover an `http_surface_tests.rs` case.

### Pitfall 8: `RunReadScope` needs a `Default` that doesn't silently narrow every internal caller
**What goes wrong:** Defaulting `RunReadScope` to `Tenant(TenantId::OPEN_ACCESS)` or similar instead
of `All` would silently break every non-web internal `.list()` caller (schedule service, fork's
"thread's most recent run" lookup at `submission.rs:412-420`, and every test using
`RunQuery { .. }`/`RunQuery::default()`).
**How to avoid:** `RunReadScope::default() -> Self::All` (D-12 states this explicitly: *"`All` is
the `Default` so internal callers are unchanged"*) — verified against `submission.rs:412-420`'s own
`RunQuery { thread_id: Some(...), limit: 1, ..Default::default() }` call, which must keep seeing
every run on the thread regardless of tenant (fork's own-thread lookup is not a caller-facing read).

## Code Examples

### Existing 404-for-hidden-resource pattern to copy verbatim
```rust
// Source: crates/paladin-web/src/run_controller.rs:813-817 (get_run, current code)
let run = repository
    .get(&id)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?;
```

### Existing dynamic-filter `QueryBuilder` pattern to extend
```rust
// Source: crates/paladin-storage/src/run/sqlite.rs:479-500 (list, current code)
let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(LIST_SELECT_PREFIX);
if let Some(thread_id) = &query.thread_id {
    builder.push(" AND thread_id = ");
    builder.push_bind(thread_id.as_str().to_string());
}
// ... assistant_id, status identical shape ...
if let Some(cursor) = &query.cursor {
    builder.push(" AND (submitted_at < "); /* ... */
}
builder.push(" ORDER BY submitted_at DESC, run_id DESC LIMIT ");
builder.push_bind(fetch_limit);
```
The `scope` filter is one more `if let` block in this exact chain, placed with the other equality
filters (before the cursor clause, since order doesn't matter for `AND`-composed predicates, but
grouping with `thread_id`/`assistant_id`/`status` keeps the diff minimal).

### Existing identifier-newtype validation shape to mirror for `TenantId`
```rust
// Source: crates/paladin-core/src/platform/container/waypoint.rs:44-77 (ThreadId, current code)
pub struct ThreadId(String);

pub enum ThreadIdError {
    Empty,
    TooLong { len: usize },
    ContainsWhitespace,
}

impl ThreadId {
    pub fn new(id: impl Into<String>) -> Result<Self, ThreadIdError> {
        let id = id.into();
        if id.is_empty() { return Err(ThreadIdError::Empty); }
        if id.len() > THREAD_ID_MAX_LEN { return Err(ThreadIdError::TooLong { len: id.len() }); }
        if id.chars().any(char::is_whitespace) { return Err(ThreadIdError::ContainsWhitespace); }
        Ok(Self(id))
    }
}
```
`THREAD_ID_MAX_LEN = 256` (`waypoint.rs:35`). CONTEXT gives the planner discretion on `TenantId`'s
exact bound; 256 (matching `ThreadId`) or a tighter bound (e.g. 64, matching typical tenant-slug
conventions) are both defensible — the important structural match is the three-variant error enum
and the `new(impl Into<String>) -> Result<Self, TenantIdError>` shape, not the exact number.

### Existing `LedgerScope` construction to redirect at the worker settle site
```rust
// Source: src/application/services/run/worker.rs:965-973 (current code, hard-coded)
if let Some(ledger) = &self.treasury_ledger {
    engine = engine.with_treasury_ledger(
        Arc::clone(ledger),
        SettlementContext {
            scope: LedgerScope::unattributed(),   // <-- D-15 replaces this
            run_id: run.run_id.clone(),
            attempt,
        },
    );
}
```
D-15's replacement: `scope: LedgerScope::from_attribution(run.submitted_by.as_ref())`, where the new
helper (`treasury_ledger.rs`, alongside `LedgerScope::unattributed()`) is:
```rust
impl LedgerScope {
    pub fn from_attribution(attribution: Option<&RunAttribution>) -> Self {
        match attribution {
            Some(a) => Self::new(a.tenant_id.as_str(), &a.api_key_id),
            None => Self::unattributed(),
        }
    }
}
```

### Existing agent-loop settle hard-code to redirect (D-16)
```rust
// Source: src/application/services/paladin/paladin_execution_service.rs:393-406 (current code)
async fn settle_agent_loop_call(
    ledger: &Arc<dyn TreasuryLedgerPort>,
    run_id: &RunId,
    ordinal: u64,
    model: &str,
    cost: &Cost,
) {
    let key = SettlementKey::new(run_id.clone(), ordinal, 1);
    let request = SettleRequest::unreserved(
        LedgerScope::unattributed(),   // <-- D-16 replaces this
        key,
        cost.clone(),
        BTreeMap::from([(model.to_string(), cost.nanos())]),
    );
    ...
}
```
D-16's replacement threads a `scope: &LedgerScope` (or `Option<&LedgerScope>`) parameter through
`settle_agent_loop_call`, `settle_model_call` (`paladin_execution_service.rs:991-996`), and the two
call sites — `execute_scoped`'s internal path and `execute_stream_inner`'s spawned-task path
(`paladin_execution_service.rs:3574-3578`) — reading it from `RunScope.ledger_scope` (the new field
Pattern 3/D-16 adds), falling back to `LedgerScope::unattributed()` when the scope carries `None`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Any authenticated principal reads any run (WR-03, Phase 27) | Tenant-scoped reads with an Admin bypass (this phase) | Phase 40 | `WINDOWS.md` row 32 closes; `docs/src/api-reference/platform-api.md`'s "What a `GET` can see today" paragraph is rewritten |
| `LedgerScope::unattributed()` on every settled row (Phase 39) | Real `(tenant_id, api_key_id)` when a principal submitted the run | Phase 40 | `paladin-cli treasury spend --tenant <t>` (39 D-09) starts returning real per-tenant rows instead of everything bucketed under `"unattributed"` |

No external ecosystem "deprecated/outdated" concerns apply — this is entirely in-tree evolution of
code shipped within the last two phases (38, 39) of the same milestone.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `TenantId`'s exact max-length bound (CONTEXT leaves this to the planner; this research suggests mirroring `THREAD_ID_MAX_LEN = 256` or a tighter tenant-slug-appropriate bound like 64) | Code Examples, "identifier-newtype validation shape" | Low — either bound is internally consistent and enforced at the newtype boundary; changing it later is a validation-only, non-schema-breaking change since the column is `TEXT` either way |
| A2 | The exact field name/location for the bearer-token tenant on `AgentAuthConfig` (CONTEXT explicitly defers this to the planner: "a field on `AgentAuthConfig` or a field beside `token_verifier`") | Pattern 2 | Low — purely an internal API shape choice inside `paladin-web`, no external contract implication |
| A3 | Whether `AgentAuthConfig`'s new bearer-tenant carrying mechanism should itself be `#[non_exhaustive]`-tolerant against future auth mechanisms — not addressed by CONTEXT | Pattern 2 | Low — `AgentAuthConfig` (`agent_auth.rs:56-64`) is already a plain `#[derive(Clone)]` struct with public fields, not `#[non_exhaustive]`, so this phase's addition follows the existing (non-exhaustive-struct) precedent regardless |

**If this table is empty:** N/A — three low-risk implementation-shape assumptions are logged above;
none of them touches a requirement, a security control, or a persisted-data contract, so none needs
user confirmation before planning proceeds.

## Open Questions

1. **Does `PaladinExecutorPort`'s new `execute_scoped` default method need its own `.cargo/semver-checks-allowlist.toml` entry?**
   - What we know: adding a method WITH a default body to a trait is the textbook non-breaking
     addition (X-10.4, `run_repository_port.rs:338-368`'s own `insert_with_latest` precedent, which
     needed no allowlist entry per that method's own doc comment).
   - What's unclear: whether `cargo-semver-checks` classifies a defaulted trait-method addition on
     an object-safe trait (`dyn PaladinExecutorPort` is used as a trait object throughout
     `agent_registry.rs`) any differently than a non-object-safe one.
   - Recommendation: run the Pitfall 5 diagnostic (temporarily disable the relevant crate-wide
     `allow`, run `cargo semver-checks check-release`, observe, revert) as part of closeout, exactly
     like 39-08 did.

2. **Should the `TenantId::OPEN_ACCESS` sentinel ever appear in a persisted `runs.tenant_id` column?**
   - What we know: D-03 says open-access mode attaches `Principal::open_access()` with
     `tenant_id: TenantId::OPEN_ACCESS` and `role: Admin`; D-11's Admin bypass means an
     open-access-submitted run's `submitted_by.tenant_id` would literally be the string
     `"open-access"`, visible to every caller (since Admin sees everything anyway, and a non-Admin
     principal can't exist in an open-access deployment — `require_authentication` attaches
     `open_access()` to EVERY request when auth is disabled).
   - What's unclear: whether this literal string leaking into `RunResponse.submitted_by.tenant_id`
     in an open-access deployment is acceptable (it's arguably fine — it's not a secret, and
     open-access mode already grants full visibility) or whether the worker should special-case it
     to `None` before persisting.
   - Recommendation: persist it as-is (simplest, most honest: the run genuinely was submitted by the
     open-access principal) unless the plan-checker or a discuss-phase follow-up raises a concern;
     document the behavior explicitly in `RunResponse.submitted_by`'s rustdoc either way.

## Environment Availability

Skipped — this phase has no new external dependencies. SQLite and Postgres (the two backends the
`008` migration targets) are the SAME backends Phase 39 already exercises; no new tool, service, or
CLI is introduced. The existing `postgres-integration` CI job (gated on `STORAGE_POSTGRES_TEST_URL`,
confirmed via 39-CONTEXT's own canonical refs) already covers the Postgres adapter path for this
phase's new contract-test cases.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust built-in `#[test]`/`#[tokio::test]`), workspace-wide |
| Config file | none — `[[test]]` targets declared in `Cargo.toml`; unit tests live in `#[cfg(test)] mod tests` blocks in-file |
| Quick run command | `cargo test -p paladin-core --lib`, `cargo test -p paladin-web --lib`, `cargo test -p paladin-ports --lib`, `cargo test -p paladin-storage --lib` (Tier 1, no Docker) |
| Full suite command | `cargo test --workspace` plus the Docker-gated `postgres-integration` CI job for the new `008`-backed Postgres contract cases |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TENANT-01 | Config with a key missing `tenant` fails `validate()` with the named-key message | unit | `cargo test -p paladin-ai --lib auth_config` | ✅ `src/config/agents.rs` tests module (extend) |
| TENANT-01 | A caller presenting key A cannot obtain tenant B by any request-side means | integration | `cargo test -p paladin-web --lib` (new test asserting `SubmitRunRequest` has no tenant field / a request-body tenant is ignored) | ✅ `crates/paladin-web/src/run_controller.rs` tests module (extend) |
| TENANT-02 | A submitted run's row carries the submitting principal's tenant/API-key | unit | `cargo test -p paladin-storage --lib` (contract_tests attribution round-trip, all 3 adapters) | ✅ `crates/paladin-storage/src/run/contract_tests.rs` (extend) |
| PLAT-07 | Every `/runs/{id}*` route returns 404 for another tenant's run, 200/202 for its own | integration | `cargo test -p paladin-web --lib run_routes` (or `cargo test -p paladin-ai --lib http_surface` per Pitfall 7's crate-boundary correction) | ⚠️ Wave 0 — new enumerated-route matrix test, per D-13 |
| PLAT-07 | `GET /runs` scoped list — SQL-level filter, not post-filtered | unit | `cargo test -p paladin-storage --lib` (contract_tests scoped-list case, all 3 adapters) | ⚠️ Wave 0 — new contract clause |

### Sampling Rate
- **Per task commit:** the relevant crate's `--lib` quick command above.
- **Per wave merge:** `cargo test --workspace` (Tier 1 only; Postgres cases self-skip without
  `STORAGE_POSTGRES_TEST_URL`, matching every prior phase's documented convention).
- **Phase gate:** `make clean-code` (fmt + clippy + check), `make api-surface` (+ `-update` and a
  CHANGELOG entry for the intentional `Principal`/`Run`/`RunQuery` surface changes), `make security`,
  full workspace test suite green, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] Enumerated per-route 404 matrix test (D-13) — covers PLAT-07; place per Pitfall 7's crate
      guidance (in-crate `run_controller.rs` test for a `paladin-web`-only run, or
      `http_surface_tests.rs` for a full-facade real-adapter run).
- [ ] `crates/paladin-storage/src/run/contract_tests.rs` scoped-list + attribution-round-trip clauses
      — covers TENANT-02 and part of PLAT-07, run against all three adapters.
- [ ] `AuthConfig::validate()` itself does not exist yet — its own test module is new, not an
      extension (`src/config/agents.rs`'s existing 6 tests are the sibling pattern to extend from).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (existing, unchanged) | `agent_auth::authenticate` — opaque bearer token / constant-time API-key compare (`ct_eq`, `agent_auth.rs:104-113`); this phase adds no new authentication mechanism |
| V3 Session Management | no | Stateless bearer/API-key model; no session concept in this codebase |
| V4 Access Control | **yes — this phase's core** | `RunReadScope::permits` + `load_visible_run` as the single, shared, row-level authorization function (D-12); server-derived tenant only, never caller-asserted (D-02) |
| V5 Input Validation | yes | `TenantId::new` validates every config-sourced and derived tenant identifier before it becomes a `Principal`/`RunAttribution` field; `ApiKeyConfig`/`AuthConfig::validate()` rejects duplicate names/keys and malformed tenant identifiers at boot (fail-closed, D-00d) |
| V6 Cryptography | no (existing, unchanged) | Constant-time comparison (`ct_eq`) for API keys is untouched by this phase; no new secret material is introduced (`api_key_id` is a label, never the key value, per `LedgerScope`'s existing doc rule) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| IDOR (Insecure Direct Object Reference) — enumerable, time-ordered `run_id` (UUIDv7) letting a caller walk or guess another tenant's runs | Information Disclosure | `load_visible_run`'s tenant-scoped `permits` check on every `/runs/{id}*` route (this phase's entire purpose); `run_controller.rs`'s own module docs already name the UUIDv7-enumerability risk (lines 52-55) |
| Existence oracle via differing response codes/timing between "missing" and "hidden" | Information Disclosure | Identical `404 "unknown run '{id}'"` for both cases (D-12); no `403`, no extra DB round-trip difference — `load_visible_run` always does exactly one `.get()` regardless of outcome |
| Privilege escalation via a caller-supplied tenant claim in the request body | Elevation of Privilege | `SubmitRunRequest` gains no tenant field (D-02); the tenant is looked up server-side from `AgentAuthConfig` inside `authenticate()` and nowhere else — enforced by a test proving a request-side tenant claim (if any were accepted) has no effect |
| Credential/secret leakage in logs or error bodies when threading a new identifier through many call sites | Information Disclosure | `api_key_id` is a label (the existing `ApiKeyConfig.name`, D-06), never the secret `key` value — this rule already holds for `LedgerScope.api_key_id` (`treasury_ledger.rs:37`) and carries forward unchanged; `Principal`'s `#[derive(Debug)]` (agent_auth.rs:36) has never held the secret key value, so no redaction gap opens by adding `tenant_id` |

## Sources

### Primary (HIGH confidence — direct in-tree file reads this session)
- `crates/paladin-web/src/agent_auth.rs` (full file) — `Principal`, `AgentAuthConfig`, `authenticate`,
  `authorize_invoke`, `require_admin`, test suite shape
- `crates/paladin-web/src/run_controller.rs` (lines 1-1416, plus targeted greps to 2434) — module
  docs, all DTOs, all handlers, router assembly, existing 404 pattern, `tester_principal()`
- `crates/paladin-web/src/agent_controller.rs` (targeted reads/greps) — `execute_agent`,
  `execute_agent_stream`, `PaladinExecutorPort` usage
- `crates/paladin-web/src/agent_registry.rs` (targeted reads) — `AgentEntry.executor` type
- `crates/paladin-ports/src/output/paladin_executor_port.rs` (full file) — the scope-less trait
- `crates/paladin-ports/src/output/paladin_port.rs` (targeted) — `execute_scoped` default-method precedent
- `crates/paladin-ports/src/input/run_submission_port.rs` (full file) — `SubmitRun`, `ForkRun`, `cancel`
- `crates/paladin-ports/src/output/run_repository_port.rs` (full file) — `RunQuery`, `RunRepositoryPort`
- `crates/paladin-core/src/platform/container/run.rs` (full file) — `Run`, `RUN_SCHEMA_VERSION`, X-04 docs
- `crates/paladin-core/src/platform/container/run_scope.rs` (full file) — `RunScope`
- `crates/paladin-core/src/platform/container/treasury_ledger.rs` (full file) — `LedgerScope`, `SettlementContext`
- `crates/paladin-core/src/platform/container/user.rs` (full file) — `UserRole`
- `crates/paladin-core/src/platform/container/waypoint.rs` (targeted, ~250 lines) — `ThreadId` validation, X-04 test precedent
- `crates/paladin-storage/src/run/sqlite.rs` (lines 1-529) — full INSERT/SELECT/list/row_to_run
- `crates/paladin-storage/src/run/in_memory.rs`, `postgres.rs` (targeted greps) — parallel structure confirmed
- `crates/paladin-storage/migrations/{sqlite/002,postgres/007}_*.sql` (full files) — header/DDL style
- `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql` — versioned `ALTER TABLE ADD COLUMN` precedent
- `crates/paladin-storage/src/sqlite_user_repository.rs:87-91` — the non-versioned ALTER TABLE pattern (what NOT to copy)
- `src/config/agents.rs` (full file) — `ApiKeyConfig`, `AuthConfig`, no existing `validate()`
- `src/bin/paladin-server.rs` (full file) — `build_auth_config`, test suite
- `src/application/services/run/submission.rs` (full file) — `authorize_invocation`, `submit`, `fork`
- `src/application/services/run/worker.rs` (targeted greps ~250 lines) — `SettlementContext` site, `run_agent`
- `src/application/services/paladin/paladin_execution_service.rs` (targeted, ~150 lines across several regions) — `settle_agent_loop_call`, `execute`/`execute_scoped`, `PaladinExecutorPort` impl
- `crates/paladin-web/tests/openapi_golden_v0_9.rs` (targeted) — `V0_9_PATHS`, sanctioned-exception scope
- `crates/paladin-web/src/openapi.rs` (targeted) — `openapi_matches_committed_baseline`
- `MIGRATION.md` (§9.2, §9.4, §9.5, §9.6 sections) — row formats
- `.cargo/semver-checks-allowlist.toml` (header + sample entries) — schema
- `crates/{paladin-web,paladin-ports,paladin-core}/Cargo.toml` (`[package.metadata.cargo-semver-checks.lints]` sections) — confirmed all three already `allow` `constructible_struct_adds_field`
- `.planning/decisions/PROMOTION.md` — next free ADR = 0054
- `.planning/WINDOWS.md` (row 32, full JSON entry) — exact closing text
- `config.example.yml` (auth section) — `api_keys`/`bearer_token` YAML shape
- `docs/src/api-reference/platform-api.md` (Authentication and scopes section) — text to rewrite
- `.planning/config.json` — `nyquist_validation` absent → treated as enabled
- `.planning/phases/40-tenant-identity-run-read-scoping/40-CONTEXT.md`, `.planning/REQUIREMENTS.md`,
  `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/phases/39-spend-ledger/39-CONTEXT.md`

### Secondary (MEDIUM confidence)
- None — every claim in this document traces to a Primary source read this session; no WebSearch
  or Context7 lookup was needed (this is a pure in-tree Rust research task with no external library
  question).

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; entirely in-tree extension
- Architecture: HIGH — every call site cited against live code read this session; two corrections
  to the phase description's own assumptions found and documented (Pitfalls 1, 6)
- Pitfalls: HIGH — all eight pitfalls are grounded in a specific, cited code discrepancy, not
  general Rust/web-API folklore

**Research date:** 2026-09-28
**Valid until:** 30 days (stable in-tree codebase; the only external-facing volatility is
`cargo-semver-checks`'s own lint set, checked via the Pitfall 5 diagnostic at implementation time
regardless of this document's age)
