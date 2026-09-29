# ADR-0054: Tenant identity is server-derived; run reads are tenant-scoped with an operator (Admin) bypass

## Status

Accepted

**Date:** 2026-09-29

## Context

`WINDOWS.md` row 32 (WR-03, Phase 27) recorded that the run read routes — `GET /runs`,
`GET /runs/{run_id}`, `GET /runs/{run_id}/webhook-deliveries` — required authentication only: `RunQuery`
carried no caller identity, neither `RunRepositoryPort::list`/`get` nor
`WebhookDeliveryRepositoryPort::list_for_run` applied a requester-derived filter, so any authenticated
principal of any role could read every run in the deployment, including another caller's webhook
target URL, and `run_id` enumeration was easier than for a random identifier. Phase 40 (TENANT-01,
TENANT-02, PLAT-07) closes that window.

Before this phase a `Principal` (`crates/paladin-web/src/agent_auth.rs`) carried only an `id` and a
`UserRole`; a run row recorded no submitter; and every ledger settlement Phase 39 wrote carried the
documented `LedgerScope::unattributed()` sentinel. Phase 41 (per-tenant and per-API-key allowances,
ALLOW-01..) must be able to evaluate a spend ceiling for the principal that caused a run, which
requires that the principal's tenant exist, be trustworthy, and travel with the run and its ledger
rows. This ADR records the identity model and the read-scope rule those phases build on, resolved by
the Phase 40 planning decisions D-02, D-05, D-10, D-11, D-12 and D-13 (40-CONTEXT.md) and confirmed
at plan 40-01's one-way-door checkpoint on 2026-09-28.

## Decision

1. **The tenant is server-derived only (D-02).** A principal's tenant is looked up from operator
   configuration (`AgentAuthConfig`) inside `authenticate()` and nowhere else. No request header,
   query parameter or body field names a tenant; `SubmitRunRequest` gains no tenant field. A caller
   presenting API key A cannot obtain tenant B by any request-side means — proven end-to-end by
   `tenant_scoped_run_read_tracer` (`src/application/services/run/http_surface_tests.rs`).

2. **Every credential maps to exactly one tenant; there is no implicit default (D-05, D-03).**
   Every `http.auth.api_keys[]` entry must carry `tenant`; a bearer-token deployment must set
   `http.auth.bearer_token.tenant`; `paladin-server` refuses to boot when either is missing, naming
   the key's `name` and never its secret value, and `AuthConfig::validate` additionally rejects a
   duplicate `name`, a duplicate `key` value, an empty `key` and an invalid `name`. A silently
   defaulted `"default"` tenant would pool every caller's runs and spend into one scope — exactly
   the cross-tenant leak this phase exists to close — so this pair is the one deliberate exception
   to X-09's "defaults to today's behaviour" rule (MIGRATION.md §9.5, §9.8 step 3). Open access
   (`bearer_token.enabled = false`, no API keys) attaches the documented `open-access` sentinel
   tenant with the Admin role, so open mode keeps its deployment-wide reads through rule 3, not
   through a bypass of it.

3. **The visibility unit is the tenant, with an Admin bypass (D-11, D-10).** A principal may see a
   run iff `run.submitted_by.tenant_id == principal.tenant_id`, or `principal.role == Admin`. Admin
   is the deployment-operator role (it already gates the registry-shaped routes), so it sees every
   run, including rows with `submitted_by = None`. A run with no recorded principal — schedule-fired
   runs, same-process embedders, tests — is visible to Admin only and settles under the Phase 39
   `unattributed` ledger sentinel; making schedule-fired runs inherit their schedule's creator is
   deferred to Phase 41. A non-Admin principal never sees an unattributed run.

4. **One shared rule, applied by exactly two mechanisms (D-12, D-13, D-14).** The rule is the core
   value type `RunReadScope { All, Tenant(TenantId) }` (`paladin_core::platform::container::
   principal`), derived from a `Principal` once by `Principal::read_scope()`. It is applied (a) on
   list reads by `RunQuery.scope`, which every `RunRepositoryPort::list` adapter (in-memory,
   SQLite, PostgreSQL) evaluates inside the query — `WHERE tenant_id = ?` — so a scoped page and
   its cursor remain a correct keyset walk rather than a post-filtered page, and the `thread_id`/
   `assistant_id`/`status` filters compose with it (AND); and (b) on single-run reads by the one
   controller helper `load_visible_run`, which is the entry point of every `/runs/{run_id}*` route
   — `GET /runs/{run_id}`, `GET /runs/{run_id}/stream`, `GET /runs/{run_id}/webhook-deliveries`
   and `POST /runs/{run_id}/cancel` — checked before any SSE upgrade, delivery listing or role
   check. `RunRepositoryPort::get` keeps its principal-free signature for the worker and services.

5. **A hidden run is the missing-run `404`, never a `403`.** `load_visible_run` returns the
   byte-identical `404 unknown run '{id}'` for another tenant's run and for a run that does not
   exist — no status, body, shape or timing difference — so a foreign `run_id` cannot be confirmed
   to exist by any run route, and cancel can neither leak existence nor perform a cross-tenant
   mutation. A route-matrix test enumerates every `/runs` route and proves each answers `404` for
   another tenant's run and `200`/`202` for its own, so a future route cannot be added without
   joining the list.

The recorded principal is persisted as `Run.submitted_by: Option<RunAttribution { tenant_id,
api_key_id }>` (nullable `runs.tenant_id`/`runs.api_key_id` columns, migration `008`,
`RUN_SCHEMA_VERSION` unchanged), surfaced read-only as `RunResponse.submitted_by`, and mapped to the
ledger by `LedgerScope::from_attribution` — one mapping for the worker path and the HTTP
agent-execute path. The key value itself is never persisted, logged, returned or formatted into an
error.

## Considered Options

- **An implicit `default` tenant for keys that name none** (rejected, D-05) — a config that
  omitted `tenant` would boot and silently pool every caller's runs and spend into one scope,
  making Phase 41's allowances enforce the wrong thing and re-creating the WR-03 leak under a new
  name. The upgrade cost — one `tenant:` line per configured key — is paid once and is visible at
  boot.
- **A client-asserted tenant (header, query parameter or body field)** (rejected, D-02) — any
  caller could name any tenant; the server would be trusting the request to scope the request.
  The tenant is a deployment-level mapping owned by the operator, not a claim.
- **Per-API-key visibility narrower than the tenant** (deferred, FUT-05) — a real need for
  multi-user tenants, but it presumes an org/RBAC model beyond `Admin`/`User` that this milestone
  does not have; tenant scope is the unit Phase 41 needs and per-key narrowing can be layered on it
  later without changing the rule's shape.
- **`403 Forbidden` for another tenant's run** (rejected, rule 5) — a `403` confirms the run
  exists and distinguishes "foreign" from "missing", turning every run route into an existence
  oracle; the `404` is what makes `run_id` enumeration no easier than for a random identifier.
- **An optional `Principal.tenant_id`** (rejected, D-01) — an unscoped principal would exist that
  neither the read rule nor a later phase's per-tenant allowances can evaluate; the required field
  is the compile-affecting break TENANT-01 pre-authorises (MIGRATION.md §9.2, `Principal` marked
  `#[non_exhaustive]` with `Principal::new` so the next field is free).
- **Post-filtering list pages in the controller** (rejected, D-12) — filtering after the
  repository returns a page breaks keyset pagination (a page can come back short or empty while
  more visible rows exist) and leaks the total through cursor behaviour; the scope belongs inside
  the adapter's query.

## Code Locations

- `crates/paladin-core/src/platform/container/principal.rs` — `TenantId`, `TenantIdError`,
  `PrincipalRef`, `RunAttribution`, `RunReadScope` (the shared rule), Phase 40 plan 40-01
- `crates/paladin-core/src/platform/container/run.rs` — `Run.submitted_by`, `Run::with_submitted_by`
- `crates/paladin-core/src/platform/container/treasury_ledger.rs` — `LedgerScope::from_attribution`,
  plan 40-04
- `crates/paladin-web/src/agent_auth.rs` — `Principal { id, role, tenant_id }`, `Principal::new`,
  `Principal::read_scope`, `Principal::ledger_scope`, `AgentAuthConfig.bearer_tenant`, the
  `authenticate()` mapping that is the only source of a tenant
- `crates/paladin-web/src/run_controller.rs` — `load_visible_run` and the route matrix test,
  `RunAttributionDto`, `RunResponse.submitted_by`, plans 40-01 and 40-05
- `crates/paladin-ports/src/output/run_repository_port.rs` — `RunQuery.scope`, the scoped-list
  contract clauses, plan 40-02
- `crates/paladin-ports/src/input/run_submission_port.rs` — `SubmitRun`/`ForkRun`/`cancel` carry
  `Option<PrincipalRef>`
- `crates/paladin-storage/src/run/{in_memory,sqlite,postgres}.rs` and
  `crates/paladin-storage/src/run/contract_tests.rs` — the three adapters applying the scope inside
  the query, plans 40-01 and 40-02
- `crates/paladin-storage/migrations/{sqlite,postgres}/008_add_run_attribution_columns.sql` — the
  nullable attribution columns and `idx_runs_tenant_submitted`
- `src/config/agents.rs` — `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`,
  `AuthConfig::validate`, plans 40-01 and 40-03
- `src/bin/paladin-server.rs` — `build_auth_config`'s fail-closed boot checks
- `src/application/services/run/submission.rs` — attribution stamping on `submit`/`fork`
- `.planning/phases/40-tenant-identity-run-read-scoping/40-CONTEXT.md` — D-01..D-21, the
  decisions this ADR records

## Code Conformance

conforms

Landed in Phase 40 (plans 40-01 through 40-05, 2026-09-28/29). The route-matrix test, the
`tenant_scoped_run_read_tracer` and the run contract suite's attribution and scoped-list clauses
pin the behaviour; a future route that bypasses `load_visible_run` fails the matrix.

## Downstream Consumers

- **Phase 41 (allowances)** — per-tenant and per-API-key ceilings read `Principal.tenant_id` and
  `PrincipalRef` as their scope key; admission-time refusal on every run path depends on the
  deferred schedule-principal item (schedule-fired runs are still unattributed under rule 3) being
  resolved there.
- **Thread-route read scoping** — `/v1/threads/*` is not tenant-scoped (threads carry no tenant);
  tracked as an open `WINDOWS.md` row filed by plan 40-06 with its closing condition (record or
  derive a tenant on the thread; route every `/threads/{id}*` handler through one shared visibility
  gate like `load_visible_run`).
- **Phase 46 (docs currency)** — the `v0.10` → `v0.11` migration guide carries §9.5's required
  `tenant` edit and §9.6's read-scope behaviour change.
