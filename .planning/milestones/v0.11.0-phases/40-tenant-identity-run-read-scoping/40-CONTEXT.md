# Phase 40: Tenant Identity & Run-Read Scoping - Context

**Gathered:** 2026-09-28
**Status:** Ready for planning
**Mode:** `--auto` — every choice below is the recommended default, selected without operator
input and logged in `40-DISCUSSION-LOG.md`. Review before planning if any default surprises you.

<domain>
## Phase Boundary

This phase makes every Platform API run attributable to the tenant and API key that submitted it,
and stops any caller from reading another caller's runs. Concretely it delivers:

1. **A tenant on the identity carrier.** Operator config maps every API key to a tenant; the
   authenticated `Principal` carries `tenant_id`; the tenant is derived server-side only and can
   never be asserted by the caller (TENANT-01).
2. **Attribution on the run row.** Every submitted run records its submitting principal (API key
   id and tenant) in the store, on every backend, and that recorded principal becomes the source
   of the ledger's `LedgerScope` in place of the Phase 39 `unattributed` sentinel (TENANT-02,
   Phase 39 D-01).
3. **Per-caller read scope.** `GET /runs` and every `/runs/{id}*` route return only runs the
   calling principal may see; another caller's run is a `404`, enforced by ONE shared
   authorization function, not per-endpoint logic (PLAT-07; closes `WINDOWS.md` row 32).
4. **Breaking-change bookkeeping.** The `Principal` change and its companions are recorded in
   `MIGRATION.md` §9.2 and `.cargo/semver-checks-allowlist.toml` (TENANT-01, X-03/X-10).

Not in this phase: allowance config and admission refusal (Phase 41), mid-run halt (Phase 42),
read scoping of `/threads/*` routes (deferred, see below), any tenant registry or RBAC beyond the
existing two roles (FUT-05), and any change to the ledger schema, port or queries (Phase 39 D-01:
those are final).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by earlier phases and milestone-level decisions — not re-asked)

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

### Identity carrier shape

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

### Config mapping surface

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

### Run-row attribution and migration

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

### Read-scope rule and the shared authorization function

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

### Ledger scope source (Phase 39 D-01 hand-off)

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

### Breaking-change bookkeeping, surface and docs

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked design (cite, do not re-open)
- `.planning/phases/39-spend-ledger/39-CONTEXT.md` — D-01 (ledger schema/port final; Phase 40
  replaces only the scope *source*), D-07 (attempt counter), D-08 (settle-only writer sites),
  D-10 (`RunResponse.cost` precedent for additive DTO fields + openapi golden exception).
- `.planning/phases/38-design-seams-pricing-cost-producer/38-CONTEXT.md` — D-00 house rules
  (config shape, X-03, ADR numbering, coverage gates).
- `.planning/decisions/0053-ledger-balance-model.md` — `LedgerScope` columns are one-way.
- `.planning/decisions/0040-opaque-bearer-token-mechanism.md` and
  `.planning/decisions/0041-in-process-token-store-single-replica-scope.md` — the bearer-token
  principal this phase gives a tenant to.
- `.planning/decisions/PROMOTION.md` — next free ADR number (0054) for D-20.

### Milestone scope and requirements
- `.planning/ROADMAP.md` §"Phase 40" — goal, success criteria 1-4; §"Phase 41" — the consumer of
  the identity carrier (why D-10's deferral is flagged there).
- `.planning/REQUIREMENTS.md` — TENANT-01, TENANT-02, PLAT-07 (this phase); ALLOW-01/02
  (what Phase 41 will read off the principal); FUT-05 (RBAC / tenant orgs are v2).
- `.planning/research/SUMMARY.md` §"Phase 40" and Pitfall 10 (secondary-route scoping bypass —
  the reason for D-13's enumerated-route test).
- `.planning/WINDOWS.md` row 32 (WR-03) — the exact closing condition: "a per-caller/tenant
  filter on RunQuery/get/list_for_run across the controller and the repository adapters".

### Breaking-change register and conventions
- `MIGRATION.md` §9.2 (row columns; the crates.io package-name rule for the Crate cell), §9.4,
  §9.5, §9.6.
- `.cargo/semver-checks-allowlist.toml` — entry schema and the set-equality contract with §9.2.
- `crates/paladin-web/Cargo.toml`, `crates/paladin-core/Cargo.toml`,
  `crates/paladin-ports/Cargo.toml` `[package.metadata.cargo-semver-checks.lints]` — which lints
  are already crate-wide allowed (39-08 D-27 diagnostic decides what the register needs).

### Code the phase extends (read, do not re-derive)
- `crates/paladin-web/src/agent_auth.rs` — `Principal { id, role }`, `AgentAuthConfig`,
  `authenticate()`, `require_authentication`, `authorize_invoke`, `require_admin`.
- `src/config/agents.rs` — `ApiKeyConfig { key, name, role }`, `BearerTokenAuthConfig`,
  `AuthConfig` (no `validate()` yet).
- `src/bin/paladin-server.rs::build_auth_config` — the config → `Principal` map and the
  fail-closed precedent.
- `crates/paladin-web/src/run_controller.rs` — module docs "Read scope (WR-03)", `submit_run`
  (`requested_by` tuple), `get_run`, `list_runs`, `stream_run`, `cancel_run`,
  `list_webhook_deliveries`, `run_openapi_router` (the route list D-13 must cover),
  `RunResponse`.
- `crates/paladin-web/src/agent_controller.rs` — the `/agents/{id}/execute[/stream]` handlers
  with a `Principal` and no run row (D-16).
- `crates/paladin-ports/src/input/run_submission_port.rs` — `SubmitRun`, `ForkRun`,
  `RunSubmissionPort::cancel` (the tuple D-04 replaces).
- `crates/paladin-ports/src/output/run_repository_port.rs` — `RunQuery`, `RunRepositoryPort`.
- `crates/paladin-core/src/platform/container/run.rs` — `Run` (`#[non_exhaustive]`, builder,
  X-04 schema-version rule), `RunCursor`.
- `crates/paladin-core/src/platform/container/run_scope.rs` — `RunScope` (D-16).
- `crates/paladin-core/src/platform/container/treasury_ledger.rs` — `LedgerScope`,
  `UNATTRIBUTED`.
- `crates/paladin-core/src/platform/container/user.rs` — `UserRole { Admin, User }`.
- `crates/paladin-storage/src/run/{mod,in_memory,sqlite,postgres,contract_tests}.rs` and
  `crates/paladin-storage/migrations/{sqlite,postgres}/002_create_runs_table.sql`,
  `007_create_treasury_ledger_table.sql` (header/comment style for `008`).
- `src/application/services/run/submission.rs` — `authorize_invocation`, `submit`, `fork`.
- `src/application/services/run/worker.rs` — the `SettlementContext` site (D-15) and
  `run_agent` (D-16).
- `src/application/services/paladin/paladin_execution_service.rs::settle_agent_loop_call` (D-16).
- `src/application/services/run/schedule/service.rs` — `requested_by: None` (D-10).
- `crates/paladin-web/tests/openapi_golden_v0_9.rs` — sanctioned-exception pattern (D-19).
- `docs/src/api-reference/platform-api.md` §"Authentication and scopes";
  `docs/src/deployment-topologies/http-service-host.md`; `config.example.yml` (D-21).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `require_authentication<S: HasAgentAuth>` already attaches `Principal` to every run, thread,
  agent, assistant and schedule route — the tenant rides on it for free once `authenticate()`
  fills it (D-01/D-03).
- `authorize_invoke` / `require_admin` in `agent_auth.rs` are the home for `read_scope()` — the
  "one function per rule, expressed once" pattern the module already documents.
- `RunSubmissionService::authorize_invocation` is the single choke point for `requested_by` — the
  D-04 type swap is one signature plus its three call sites.
- `Run::new` + `with_*` builders under `#[non_exhaustive]` make D-08 additive on the Rust side.
- The run store contract suite (`contract_tests.rs`, `list_filters_by_thread_assistant_and_status`)
  is the template for the scoped-list and attribution round-trip cases, run against all three
  adapters.
- `sqlx::migrate!` embeds every file under `migrations/{sqlite,postgres}/` and applies on
  construction — a new `008` file needs no runner change; `sqlite_user_repository.rs` already
  uses `ALTER TABLE … ADD COLUMN` as an in-tree precedent.
- `RunResponse.cost` (39-06) is the exact precedent for an additive, serde-defaulted DTO field with
  an openapi-golden exception (D-19).
- `RunScope` (39-05/39-07) is the existing carrier from the worker into the agent loop — D-16
  adds one optional field rather than a new plumbing path.

### Established Patterns
- **Fail-closed config:** `build_auth_config` errors with an actionable message when auth is
  enabled with no credentials; D-05/D-06 extend the same voice ("set `tenant` on api key
  `<name>`").
- **Constant-time key lookup, never echo the credential:** `lookup_api_key`/`ct_eq`; D-19 and
  every log line keep the key value out.
- **404 for hidden resources:** the run routes already answer `404 "unknown run"`; D-12 reuses the
  literal so a scoped miss is indistinguishable from a missing row.
- **Keyset pagination is in SQL, not post-filtered:** `(submitted_at DESC, run_id DESC)` with a
  cursor — D-12's `WHERE tenant_id = ?` must sit in the same query and the new index covers it.
- **X-10 register discipline:** the §9.2 row, the allowlist entry and the `Cargo.toml` lint line
  land in one commit; the 39-08 diagnostic decides whether an entry is needed for an already-
  allowed lint.

### Integration Points
- `authenticate()` (API key branch and bearer branch) → `Principal.tenant_id`.
- `submit_run` / `fork` / `cancel_run` controllers → `PrincipalRef::from(&principal)`.
- `RunSubmissionService::submit`/`fork` → `Run::with_submitted_by`.
- `RunQuery.scope` → all three `RunRepositoryPort::list` adapters.
- `load_visible_run` → `get_run`, `stream_run`, `list_webhook_deliveries`, `cancel_run`.
- Worker `SettlementContext.scope` and `RunScope.ledger_scope` → ledger rows carry real tenants;
  the `treasury spend --tenant` CLI (39 D-09) starts returning attributed rows.
- `build_auth_config` (server binary) → passes `tenant` through to `Principal` and the bearer
  tenant into `AgentAuthConfig`.

</code_context>

<specifics>
## Specific Ideas

- The enumerated-route `404` matrix (D-13) is the regression guard research Pitfall 10 asks for:
  when a future phase adds `/runs/{id}/something`, the test fails until the route joins
  `load_visible_run`.
- Error voice for config: `http.auth.api_keys[<name>]: 'tenant' is required — every API key must
  map to a tenant (Phase 40, TENANT-01)`.
- `TenantId::OPEN_ACCESS` and `LedgerScope::UNATTRIBUTED` are deliberately different literals
  (`"open-access"` vs `"unattributed"`): one names a principal that exists when auth is off, the
  other names the absence of any principal on a ledger row.

</specifics>

<deferred>
## Deferred Ideas

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

### Reviewed Todos (not folded)
- *Evaluate replacing MinIO with RustFS in the dev/test stack*
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`;
  `todo.match-phase` score 0.2 on the keyword "phase" only, below the 0.4 auto-fold threshold).
  Phase 45 STORE-01..03 scope; same disposition as Phases 38 and 39.

</deferred>

---

*Phase: 40-tenant-identity-run-read-scoping*
*Context gathered: 2026-09-28*
