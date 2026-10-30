# Phase 40: Tenant Identity & Run-Read Scoping - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-28
**Phase:** 40-tenant-identity-run-read-scoping
**Mode:** `--auto` — every selection below is the recommended default chosen by Claude without
operator input. No `AskUserQuestion` was issued.
**Areas discussed:** Identity carrier shape, Config mapping surface, Run-row attribution &
migration, Read-scope rule & shared authorization function, Ledger scope source, Breaking-change
bookkeeping & docs

---

## Identity carrier shape

| Option | Description | Selected |
|--------|-------------|----------|
| Required `TenantId` on every `Principal` | Newtype field, no `Option`; bearer tokens take a configured tenant; open-access uses a sentinel | ✓ |
| `Option<TenantId>` with `None` = unscoped | Softer break, but every downstream check grows a `None` arm and Phase 41 cannot enforce on a `None` | |
| Tenant only on API-key principals | Bearer-token principals would be permanently unattributable | |

| Option | Description | Selected |
|--------|-------------|----------|
| Core `PrincipalRef` value type replaces the tuple | One type on `SubmitRun`/`ForkRun`/`cancel`; role check reads `.role` | ✓ |
| Extend `(String, UserRole)` to a triple | Same break, worse ergonomics, no home for `attribution()` | |
| Keep the tuple, pass tenant separately | Two parameters that must always agree | |

**Auto-selected:** Required `TenantId`; `PrincipalRef` replaces the tuple.
**Notes:** TENANT-01 already pre-authorises the `Principal` break, so the cleanest shape wins.

---

## Config mapping surface

| Option | Description | Selected |
|--------|-------------|----------|
| Required `tenant` per `api_keys` entry, fail closed | Boot fails with a message naming the key; §9.5 row | ✓ |
| Optional with implicit `"default"` tenant | X-09-friendly but silently pools every key's spend into one allowance scope | |
| Separate `tenants:` registry section | Extra surface with nothing to validate against until Phase 41 | |

| Option | Description | Selected |
|--------|-------------|----------|
| Reuse `name` as the API key id; validate uniqueness | Already the principal id and log identifier | ✓ |
| Add a new `id` field | Two identifiers for one key | |

**Auto-selected:** Required `tenant`; `name` is the key id with a new `AuthConfig::validate()`.

---

## Run-row attribution & migration

| Option | Description | Selected |
|--------|-------------|----------|
| `008` migration: two nullable columns + covering index | `NULL` = no principal; index on `(tenant_id, submitted_at DESC, run_id DESC)` | ✓ |
| `NOT NULL DEFAULT 'unattributed'` | Copies the ledger sentinel into the run row; conflates "no principal" with a literal tenant | |
| Single JSON `principal` column | Not filterable in the keyset query | |

| Option | Description | Selected |
|--------|-------------|----------|
| Schedule-fired / internal runs persist `None`; Admin-only visibility | Smallest change; deferral flagged for Phase 41 | ✓ |
| Schedules record the creator's principal now | Touches the schedule schema — scope creep | |
| Reject principal-less submits | Breaks schedules and embedders | |

**Auto-selected:** Nullable columns; `None` for principal-less submits.

---

## Read-scope rule & shared authorization function

| Option | Description | Selected |
|--------|-------------|----------|
| Tenant scope + Admin bypass | Same-tenant runs visible; operator (Admin) sees everything incl. unattributed | ✓ |
| API-key scope only | Narrower than the milestone's tenant model; Phase 41 allowances are per tenant first | |
| Tenant scope, no Admin bypass | Operator loses the deployment-wide view and pre-v0.11 rows become unreadable | |

| Option | Description | Selected |
|--------|-------------|----------|
| Core `RunReadScope` + `RunQuery.scope` in SQL + one `load_visible_run` helper on every `/runs/{id}*` route (incl. cancel) | Matches WINDOWS row 32's closing condition (controller AND adapters) | ✓ |
| Controller-only post-fetch check | Post-filtered pages break keyset pagination | |
| Scope parameter on every port method | Breaks `get` for the worker and services that have no principal | |

**Auto-selected:** Tenant scope with Admin bypass; `RunReadScope` applied via list query and one helper.
**Notes:** `cancel` included so a foreign run 404s everywhere; `/threads/*` deferred.

---

## Ledger scope source

| Option | Description | Selected |
|--------|-------------|----------|
| `run.submitted_by` on the worker; `RunScope.ledger_scope` for agent-kind runs; `/agents/{id}/execute` attributes from its `Principal` | Sentinel only when no principal exists | ✓ |
| Run row only; agent-execute stays unattributed | Leaves a spending path invisible to Phase 41 | |

**Auto-selected:** All three sources wired.

---

## Breaking-change bookkeeping & docs

| Option | Description | Selected |
|--------|-------------|----------|
| Additive `submitted_by` on `RunResponse` | Mirrors the `cost` precedent; never the key value or role | ✓ |
| Do not expose | Operators cannot verify attribution over HTTP | |

| Option | Description | Selected |
|--------|-------------|----------|
| One ADR-0054 | Records server-derived tenant, tenant scope, Admin bypass, 404-not-403 | ✓ |
| No ADR | Security posture would live only in CONTEXT.md | |

**Auto-selected:** Expose `submitted_by`; write ADR-0054.

---

## Claude's Discretion

- `TenantId` identifier bound and character rules (mirror `ThreadId`).
- Whether `RUN_SCHEMA_VERSION` bumps, per the X-04 rule in `run.rs`.
- Module layout and final names of `TenantId` / `PrincipalRef` / `RunAttribution` / `RunReadScope`.
- Where `AgentAuthConfig` holds the bearer tenant.
- Test topology for the per-route 404 matrix.

## Deferred Ideas

- Thread-route read scoping (new WINDOWS.md row).
- Schedule-created runs inherit the schedule creator's principal (flag for Phase 41).
- Per-API-key visibility narrower than tenant; org/RBAC (FUT-05).
- Tenant registry / allowance tenant-id cross-check (Phase 41).
- CLI run-inspection scoping; `submitted_by` in herald/trace output.
- Reviewed, not folded: RustFS/MinIO todo (score 0.2; Phase 45 scope).
