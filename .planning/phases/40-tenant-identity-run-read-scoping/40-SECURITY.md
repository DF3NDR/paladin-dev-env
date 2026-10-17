---
phase: 40
slug: tenant-identity-run-read-scoping
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-30
---

# Phase 40 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — every one of the six PLAN.md files (40-01 through 40-06) carries a `<threat_model>` block, 26 threats in total (T-40-01 .. T-40-26). Verification depth: ASVS L1 (grep-level presence checks against the implementation, the phase artifacts and CI run 478), per the short-circuit rule for a plan-time register with `threats_open: 0`. Every SUMMARY.md `## Threat Flags` section reports "None" beyond the plan register. The post-phase code review (40-REVIEW.md, 40-REVIEW-FIX.md) found no Critical finding; its four warnings (WR-01..WR-04) were fixed and human-verified in 40-UAT.md tests 1 and 5.

Evidence tree: commit `be3a9030` (Phase 40 final code commit) is an ancestor of `730f521f`, on which ci.yml run 478 (id 36770517439) is fully green — Postgres contract suites, coverage (82 % floor), the `e2e_platform_api` integration binary, cargo-audit, cargo-deny and the semver / MIGRATION.md 9.2 set-equality check.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| HTTP client -> `authenticate()` | untrusted `X-API-Key` / `Authorization` headers, any spoofed tenant header | credentials; tenant derived from config only |
| HTTP client -> `POST /v1/runs` body/query | untrusted JSON body and query string reaching `submit_run` | run request; no tenant field |
| HTTP client -> `GET /v1/runs`, `/v1/runs/{id}`, `/stream`, `/cancel`, `/webhook-deliveries` | enumerable UUIDv7 ids, filters and cursors | run rows, SSE events, webhook target URLs |
| operator config -> `AuthConfig::validate` / `build_auth_config` | key values, names, tenants, bearer tenant read once at boot | secret key values (never echoed) |
| run row -> `row_to_run` | persisted attribution read back into `TenantId` | `tenant_id`, `api_key_id` |
| `RunQuery.scope` -> SQL adapters | tenant id reaching dynamic list SQL | bound parameter only |
| run row / `Principal` -> ledger settlement | recorded `submitted_by` or the authenticated caller becomes a `LedgerScope` | `(tenant, api_key_id)` |
| future route author -> `run_openapi_router` | a new `/runs/{run_id}*` route added without the visibility gate | route inventory |
| register -> CI `semver` job | MIGRATION.md 9.2 / allowlist pair that reviewers and CI trust | break register |
| docs, examples, CHANGELOG -> readers | text that could leak or teach unsafe credential handling | config examples (`${...}` placeholders) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-40-01 | Elevation of Privilege | `authenticate()` / `submit_run` tenant source | high | mitigate | tenant only from `AgentAuthConfig` (`bearer_tenant`, api_keys values); `tenant_is_derived_from_config_never_from_request_headers` (`crates/paladin-web/src/agent_auth.rs`); `tenant_scoped_run_read_tracer` spoofs header, body and query (`src/application/services/run/http_surface_tests.rs`) | closed |
| T-40-02 | Information Disclosure | `GET /v1/runs/{id}` IDOR over UUIDv7 ids | high | mitigate | `load_visible_run` + `RunReadScope::permits` (`run_controller.rs:921`; `permits(` count = 1 in the controller); `get_run_hides_a_foreign_tenant_run_behind_the_missing_run_404` | closed |
| T-40-03 | Information Disclosure | hidden-vs-missing existence oracle | medium | mitigate | single `unknown run '{run_id}'` literal on every path (4 occurrences, all in `run_controller.rs`); byte-equality asserted by the tracer and the route matrix | closed |
| T-40-04 | Information Disclosure | `build_auth_config` error text | medium | mitigate | `build_auth_config_fails_closed_when_an_api_key_has_no_tenant` asserts the key value is absent (`src/bin/paladin-server.rs`) | closed |
| T-40-05 | Spoofing | bearer principal with no tenant | high | mitigate | `bearer_tenant` required at boot when bearer is enabled; `verified_bearer_token_without_configured_tenant_fails_closed` (`agent_auth.rs`) | closed |
| T-40-06 | Tampering | persisted attribution read back | medium | mitigate | `TenantId::new` validation on read maps to `RunRepositoryError::Serialization` (`postgres.rs:280`); half-attributed rows rejected; SQL through `&'static str` constants and binds | closed |
| T-40-07 | Information Disclosure | stream / cancel / webhook-deliveries between 40-01 and 40-05 | medium | accept | transient inside the phase; closed before seal by 40-05 — `load_visible_run` now gates `get_run` (:921), `stream_run` (:1076), `cancel_run` (:1147) and `list_webhook_deliveries` (:1239) | closed |
| T-40-08 | Information Disclosure | `GET /v1/runs` listing other tenants' runs | high | mitigate | `scope: principal.read_scope()` in `list_runs`; `tenant_scoped_run_list_e2e`; `list_scoped_to_tenant_returns_only_that_tenants_runs` in the shared contract suite (SQLite, Postgres, in-memory) | closed |
| T-40-09 | Information Disclosure | keyset cursor / page under-fill revealing hidden rows | medium | mitigate | predicate inside the `QueryBuilder` before `ORDER BY ... LIMIT`; `list_scoped_pagination_has_no_gap_or_overlap` on all adapters | closed |
| T-40-10 | Tampering | tenant id in dynamic list SQL | medium | mitigate | `push_bind` only; `list_query_applies_the_tenant_scope_before_order_by` inspects the placeholder form (`postgres.rs`) | closed |
| T-40-11 | Tampering | attribution overwritten after insert | medium | mitigate | no production `UPDATE` touches `tenant_id`/`api_key_id` (the only such statements sit inside `#[cfg(test)]` modules, sqlite.rs:1147 and postgres.rs:1308, as the immutability probes); `attribution_survives_every_status_and_attempt_update` on all adapters | closed |
| T-40-12 | Information Disclosure | duplicate-key / validation error text | medium | mitigate | messages name `name`/index only; `validate_rejects_a_duplicate_key_value_without_printing_it` (`src/config/agents.rs`) | closed |
| T-40-13 | Spoofing | duplicate key value -> first-match resolves an arbitrary principal | high | mitigate | duplicate key values rejected at boot (same test) | closed |
| T-40-14 | Elevation of Privilege | empty key value accepted as a credential | medium | mitigate | `validate_rejects_an_empty_key_value` (`src/config/agents.rs`) | closed |
| T-40-15 | Repudiation | spend misattributed (sentinel on an attributed run, or wrong tenant) | high | mitigate | one mapping `LedgerScope::from_attribution` on every writer path (`run_scope.rs`, `treasury_ledger.rs`, `worker.rs`, `agent_auth.rs`); `engine_run_settles_under_its_run_id_and_first_attempt` | closed |
| T-40-16 | Tampering | caller-influenced ledger scope on `/agents/{id}/execute` | high | mitigate | scope built only from the authenticated `Principal` via `ledger_scope()` (`agent_controller.rs`, `agent_host.rs`, `paladin_execution_service.rs`); controller tests pin it | closed |
| T-40-17 | Information Disclosure | settle log lines | low | accept | log lines carry run id, ordinal, nanos and currency only (`paladin_execution_service.rs:409` documents the rule); `api_key_id` is a key name, never the secret | closed |
| T-40-18 | Information Disclosure | `stream_run` SSE of a foreign run | high | mitigate | `load_visible_run` before the SSE upgrade (`run_controller.rs:1076`); matrix row | closed |
| T-40-19 | Tampering / Elevation of Privilege | cross-tenant `cancel_run` | high | mitigate | `load_visible_run` before cancel (`run_controller.rs:1147`); WR-02 fix additionally applies `permits` inside `RunSubmissionService::cancel` (commit fa054746) | closed |
| T-40-20 | Information Disclosure | foreign webhook target URLs via `list_webhook_deliveries` | high | mitigate | `load_visible_run` before `list_for_run` (`run_controller.rs:1239`); matrix row | closed |
| T-40-21 | Information Disclosure | per-route existence oracle (403 vs 404, message drift) | medium | mitigate | every route returns the single 404 literal; `every_run_id_route_hides_foreign_runs_behind_the_missing_run_404` (`run_controller.rs:3154`) asserts byte equality | closed |
| T-40-22 | Elevation of Privilege | a future `/runs/{run_id}*` route that skips the gate | medium | mitigate | the same matrix enumerates operations from the router's OpenAPI document and fails on any unlisted route; `run_openapi_router_contains_run_paths` | closed |
| T-40-23 | Information Disclosure | `/v1/threads/*` reads remain deployment-wide | high | transfer | out of scope by locked decision D-14; recorded as `.planning/WINDOWS.md` row 58 (open, owner Phase 41 or a v0.11 hygiene phase); WR-01 fix (commit 6d7efb16, `ensure_thread_visible`) closes the run-side mutation path via `POST /v1/runs {thread_id}` and fork; stated in module docs, platform-api.md and MIGRATION.md 9.6 | closed |
| T-40-24 | Repudiation | a Phase 40 break shipped without a register row | medium | mitigate | CI `semver` job step "Verify allowlist is set-equal to the MIGRATION.md §9.2 register" green in run 478 | closed |
| T-40-25 | Information Disclosure | a real API key value in a doc, example, CHANGELOG or error text | high | mitigate | manual credential-handling review over `git diff 629ef660..HEAD` performed and passed (40-UAT.md test 3); added config examples use `${...}` placeholders only | closed |
| T-40-26 | Tampering (supply chain) | new dependencies | low | accept | `git diff 629ef660 be3a9030` touches no `Cargo.toml` or `Cargo.lock`; cargo-audit and cargo-deny green in run 478 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-40-01 | T-40-07 | Exposure window existed only between plans 40-01 and 40-05 inside the phase; no release was cut mid-phase and 40-05 closed every route before seal. | 40-01 PLAN (plan-time), confirmed by 40-05 SUMMARY | 2026-09-30 |
| R-40-02 | T-40-17 | Settle log lines carry run id, ordinal, nanos and currency only; `api_key_id` is the configured key name and never the secret value. | 40-04 PLAN (plan-time), confirmed by 40-04 SUMMARY | 2026-09-30 |
| R-40-03 | T-40-26 | Phase 40 adds no crate; dependency scanners (cargo-audit, cargo-deny) stay green. | 40-06 PLAN (plan-time), confirmed by CI run 478 | 2026-09-30 |

*Accepted risks do not resurface in future audit runs.*

Transferred: T-40-23 is tracked outside this phase as `.planning/WINDOWS.md` row 58 and does not count toward `threats_open`; it re-enters a threat register when that row's owner phase is planned.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-30 | 26 | 26 | 0 | /gsd-secure-phase 40 (orchestrator, L1 grep-level; short-circuit, no auditor subagent) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-30
