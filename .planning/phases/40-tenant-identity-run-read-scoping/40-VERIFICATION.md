---
phase: 40-tenant-identity-run-read-scoping
verified: 2026-09-29T12:45:00Z
status: passed
score: 4/4 roadmap success criteria verified; 43/44 plan truths verified (1 abstained, insufficient_spec; 0 failed)
behavior_unverified: 0
overrides_applied: 1
overrides:

  - must_have: ".project/current-exports.txt contains RunReadScope (40-06 artifact)"
    reason: "cargo-public-api lists a re-exported foreign-crate module as one `pub use paladin::core::platform::container::principal` line and never its items; every facade-visible Phase 40 item is present in the refreshed baseline and `make api-surface` exits 0. Tool-design limitation recorded and waived as WINDOWS.md row 59."
    accepted_by: "operator (WINDOWS.md row 59 waiver)"
    accepted_at: "2026-09-29T12:16:51Z"
deferred:

  - truth: "Schedule-fired runs record their submitting principal (today `schedule/service.rs` passes `requested_by: None`, so they persist `submitted_by = None`, are Admin-only visible, and settle under the `unattributed` ledger sentinel)"
    addressed_in: "Phase 41"
    evidence: "40-CONTEXT D-10 and ADR-0054 rule 3 defer schedule-principal inheritance to Phase 41; Phase 41 SC 2 ('Submitting a run while the caller's tenant or API-key allowance is exhausted is refused ... before any run row is written') cannot be evaluated for a principal-less run path"

  - truth: "`/v1/threads/*` routes are tenant-scoped"
    addressed_in: "Phase 41 planning or a v0.11 hygiene phase (WINDOWS.md row 58, open)"
    evidence: "PLAT-07 names only `GET /runs` and `/runs/{id}*`; 40-CONTEXT D-14 and ADR-0054 'Downstream Consumers' record the deferral; WINDOWS.md row 58 carries the closing condition and owner"
human_verification:

  - test: "Scope decision on code-review WR-01 (40-REVIEW.md): `POST /v1/runs` with a caller-supplied `thread_id`, and `POST /v1/threads/{id}/fork`, accept another tenant's thread without a tenant check (`src/application/services/run/submission.rs` submit: `request.thread_id.unwrap_or_else(generate_thread_id)` with no visibility check; fork: `RunQuery { thread_id, ..Default::default() }` is `scope: All` and the principal is never compared to the thread's runs). A user-role principal of tenant B can resume/fork tenant A's Waypoint into a run attributed to B, which B may then read. Decide whether this is inside PLAT-07's intent (fix: when `thread_id` is `Some` and `requested_by` is `Some`, look up the thread's latest run with `scope: All` and return the same not-found error as a missing thread when `RunReadScope::for_principal(role, tenant).permits(&latest)` is false) or part of the D-14 thread-tenancy deferral (then extend WINDOWS.md row 58 to name the `POST /runs {thread_id}` and fork paths explicitly)."
    expected: "Either a fix with a contract-style test for submit and fork, or WINDOWS.md row 58 amended to name both paths, before Phase 41 builds admission enforcement on the thread path."
    why_human: "The four roadmap success criteria name only read routes and are met; whether cross-tenant thread resume via a write route is inside this phase's goal ('no caller can read another caller's runs') is a scope judgment the verifier cannot make."

  - test: "Backstop truth (40-03, `verification: backstop`): 'the key-to-tenant mapping is immutable after boot -- `AgentAuthConfig` is built once by `build_auth_config`, cloned into each router state, and never written again, so concurrent requests cannot observe a changing tenant for the same key.' Structural observation only: `grep -nE 'api_keys\\.(insert|remove|clear|retain|extend)|api_keys\\s*=' crates/paladin-web/src/*.rs` finds writes only inside `#[cfg(test)]` modules, and `require_authentication` reads `state.agent_auth()` immutably."
    expected: "A human confirms the no-mutation-after-boot invariant (or adds a held-out test that clones `AgentAuthConfig` into two router states and asserts the same key resolves the same tenant under concurrent requests)."
    why_human: "Tagged non-inferable at spec time; symbol presence and wiring are not explicit evidence for a concurrency invariant. Recorded as `insufficient_spec`, not as a failure."

  - test: "Judgment-tier prohibitions (20 `must_haves.prohibitions` items across the six plans, all `verification: null`). The verifier's non-authoritative LLM-judge verdict is 'held' for every item (table in 'Prohibitions' below), with test-backed evidence for 11 of them. The remaining 9 rest on code reading: no API-key value in any log, error, response body, persisted row, doc or example (D-00g; the 40-06 SUMMARY records the manual credential-handling review of `git diff 629ef660..HEAD` as clean); no new Medieval-military officer word; no Snyk step; no tenant field on any request DTO."
    expected: "A human reviews the flagged prohibitions and confirms or rejects the non-authoritative verdicts; in particular re-runs the credential-handling review required by `.github/instructions/security.instructions.md` over the Phase 40 diff."
    why_human: "ADR-550 D4: judgment-tier prohibitions are never a silent pass in an autonomous run; they are flagged `unverified-prohibition -- human review recommended`."

  - test: "CI-only evidence this sandbox cannot produce (no Docker daemon, ~11 GB disk): the eight live PostgreSQL clauses in `crates/paladin-storage/src/run/postgres.rs` (six shared contract clauses plus `insert_with_latest_persists_attribution` and `unattributed_run_is_stored_as_sql_null`) self-skip locally; the 82 % `cargo llvm-cov --fail-under-lines` coverage job; the `web-server`-feature integration target `tests/integration/e2e_platform_api_test.rs` (exercises `/stream` and `/webhook-deliveries` with an Admin key). The full `cargo test --workspace` figure (57 suites, 6067 passed, 0 failed, 229 ignored) is the 40-06 SUMMARY's record of the gate run on `be3a9030`; the only saved workspace log in the scratchpad is dated 2026-09-26 (5855 passed) and predates this phase, so that figure was not independently re-observed here. Every commit after `be3a9030` touches only `.planning/` (confirmed with `git log --stat`)."
    expected: "CI `postgres-integration`, `coverage` and the integration job are green on the phase's final tree."
    why_human: "Requires Docker services and a full-workspace build that the environment and disk budget do not allow; recorded as CI-only, neither a pass nor a failure."

  - test: "Disposition of code-review WR-02, WR-03 and WR-04 (40-REVIEW.md), confirmed against the tree: (WR-02) `RunSubmissionService::cancel` receives `Option<PrincipalRef>` carrying `tenant_id` but only feeds `role` to `authorize_invocation`, so tenant isolation on cancel is enforced solely by `run_controller::cancel_run`'s `load_visible_run` pre-check -- an in-process embedder calling the port directly can cancel a foreign run; (WR-03/04) `row_to_run` returns `RunRepositoryError::Serialization` for a half-attributed or invalid-tenant row and `load_visible_run`/`list_runs` map every repository error through `ApiError::internal(e.to_string())` (`crates/paladin-web/src/error.rs:135` echoes the message verbatim), so a corrupt foreign row answers 500 with backend text instead of the uniform 404, and one corrupt row fails a tenant's whole list page."
    expected: "Decide: apply the shared `permits` check inside `RunSubmissionService::cancel` and a generic-body repository-error helper now, or file them (WINDOWS.md or Phase 46 hygiene). Neither blocks the roadmap criteria, which are stated at the HTTP route level and are met there."
    why_human: "Defense-in-depth and hardening trade-offs; the roadmap contract is satisfied as written."
---

# Phase 40: Tenant Identity & Run-Read Scoping Verification Report

**Phase Goal:** Every run is attributable to the tenant and API key that submitted it, and no caller can read another caller's runs.
**Verified:** 2026-09-29T12:45:00Z (tree `8c47f370`; code last changed at `be3a9030`, every later commit is `.planning/`-only)
**Status:** human_needed
**Re-verification:** No -- initial verification

Method: goal-backward. The four ROADMAP success criteria are the contract; the six plans' `must_haves` (44 truths, 22 artifacts, 16 key links, 20 prohibitions) add detail and never reduce scope. Every claim below was checked against the tree with grep/`sed`, tool queries (`verify.artifacts`, `verify.key-links`), `./scripts/check-migration-allowlist.sh`, and 62 named tests run in this session (`cargo test -p <crate> --lib -- <names>`; the full workspace suite was deliberately not re-run -- ~10 GB of artifacts against ~11 GB free). SUMMARY claims were treated as hypotheses, not evidence.

## Goal Achievement

### Observable Truths -- ROADMAP Success Criteria

| # | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | Every API key in operator config maps to a tenant, the authenticated `Principal` carries `tenant_id`, and a caller cannot assert a different tenant than its own key's mapping | VERIFIED | `crates/paladin-web/src/agent_auth.rs:46-54` `#[non_exhaustive] pub struct Principal { id, role, tenant_id: TenantId }`, `Principal::new` (:59), `read_scope()` (:76), `ledger_scope()` (:88); `authenticate()` (:193-221) resolves the tenant only from `config.bearer_tenant` (bearer branch, fails closed when `None`) or from the configured `Principal` in `config.api_keys` -- no header/query/body read; open access attaches `Principal::open_access()` = `("anonymous", Admin, TenantId::open_access())` (:70-71, :243). `src/config/agents.rs:101` `pub tenant: String` on `ApiKeyConfig`, `:113` `tenant: Option<String>` on `BearerTokenAuthConfig`, `:180` `AuthConfig::validate()` with the `'tenant' is required` / `not a valid tenant id` / duplicate `name` / duplicate `key` messages; `src/bin/paladin-server.rs:361` `cfg.validate()?` is the first statement of `build_auth_config`, `:393` `Principal::new(k.name.clone(), k.role, tenant)`. `SubmitRunRequest` keeps exactly five fields (`assistant_id, version, thread_id, input, webhook`). Tests run and passed: `agent_auth::tests::every_principal_source_yields_a_tenant`, `verified_bearer_token_without_configured_tenant_fails_closed`, `run_controller::tests::list_runs_ignores_a_tenant_query_parameter`, and the end-to-end `http_surface_tests::tenant_scoped_run_read_tracer` (spoofs `x-tenant-id: globex`, `?tenant_id=globex`, body `tenant_id`/`tenant`: run still attributed to `acme`/`svc-a`), plus `config::agents::tests::validate_rejects_*` (6) |
| 2 | Every submitted run records its submitting principal (API key id and tenant) | VERIFIED | `crates/paladin-core/src/platform/container/run.rs:417` `pub submitted_by: Option<RunAttribution>`, `:462` `with_submitted_by`, `:50` `RUN_SCHEMA_VERSION = "v1"` unchanged; `src/application/services/run/submission.rs:287-288` (submit) and `:456-457` (fork) stamp `principal_ref.attribution()`; `run_controller.rs:763` and `:1063` pass `PrincipalRef::from(&principal)`. Migration `008_add_run_attribution_columns.sql` in both `migrations/sqlite/` and `migrations/postgres/` (`ALTER TABLE runs ADD COLUMN tenant_id TEXT NULL`, `api_key_id TEXT NULL`, `CREATE INDEX ... idx_runs_tenant_submitted`). All three adapters read/write the columns: `sqlite.rs` (INSERT/SELECT constants :48-87, `row_to_run` :295-308 rejects a half-attributed row, binds :399/:761), `postgres.rs` (:56-92, :277-290, :425/:758), `in_memory.rs` (stores the `Run`). Ledger attribution: `treasury_ledger.rs:113` `LedgerScope::from_attribution` is the single mapping; `worker.rs:981` (engine `SettlementContext.scope`) and `:1272` (agent-kind `RunScope::with_ledger_scope`) both use it; no `unattributed()` remains in worker non-test code; `agent_controller.rs:318/614/699` build `RunScope::default().with_ledger_scope(principal.ledger_scope())` for execute, stream (both branches) and jobs. Tests run and passed: `sqlite::tests::{migration_008_adds_nullable_attribution_columns_and_the_scoped_index, half_attributed_row_is_a_serialization_error, unattributed_run_is_stored_as_sql_null, attribution_survives_every_status_and_attempt_update}`, `in_memory::contract_suite::attribution_survives_every_status_and_attempt_update`, `postgres::tests::unattributed_run_is_stored_as_sql_null` (DB-free constant check), `worker::tests::attributed_engine_run_settles_under_its_submitting_principal_scope`, `unattributed_engine_run_settles_under_the_unattributed_sentinel`, `agent_loop_settles_under_the_run_scope_ledger_scope`, `stream_scoped_settles_under_the_scope_ledger_scope`, `executor_port_execute_scoped_settles_under_the_scope_ledger_scope`, and the tracer. Schedule-fired runs still pass `requested_by: None` (`schedule/service.rs:347`) -- the locked D-10 deferral to Phase 41, listed under Deferred Items |
| 3 | `GET /runs` and every `/runs/{id}*` read route return only runs the calling principal may see -- another caller's run returns 404 -- enforced by one shared authorization function used on every such route, not duplicated per endpoint | VERIFIED | The shared rule is `RunReadScope::{for_principal, permits}` (`principal.rs:219-235`; `Tenant(t)` never permits `submitted_by == None`). Single-run half: `run_controller.rs:849-865` `load_visible_run` fetches then checks `principal.read_scope().permits(&run)` and returns the byte-identical `ApiError::not_found("unknown run '{id}'")` on both the missing and the hidden path; it is called by `get_run` (:905), `cancel_run` (:1060, before `RunSubmissionPort::cancel`), `list_webhook_deliveries` (:1131, before `list_for_run`) and `stream_run` (:1223, before the SSE upgrade). `permits(` appears exactly once in paladin-web non-test code (:860). List half: `list_runs` (:987) sets `scope: principal.read_scope()`; `RunQuery.scope: RunReadScope` (`run_repository_port.rs:79`, `Default = All`); `sqlite.rs:525-540` and `postgres.rs:348-363` push ` AND tenant_id = ` + bind before the cursor clause and `ORDER BY submitted_at DESC, run_id DESC LIMIT`; `in_memory.rs:166` `.filter(|r| query.scope.permits(r))` before sort/page. The router publishes exactly six run operations (:1257-1262) and no other crate registers a `/runs` path. Tests run and passed: `run_controller_auth::every_run_id_route_hides_foreign_runs_behind_the_missing_run_404` (enumerates `/v1/runs/{run_id}*` from the router's own OpenAPI document, fails on table drift; owner/peer/Admin succeed, foreign key gets the swapped-id byte-identical 404), `get_run_hides_a_foreign_tenant_run_behind_the_missing_run_404`, `http_surface_tests::{tenant_scoped_run_read_tracer, tenant_scoped_run_list_e2e, cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag}` (real `run_router` over on-disk SQLite), `sqlite/in_memory::list_scoped_pagination_has_no_gap_or_overlap`, `postgres::tests::list_query_applies_the_tenant_scope_before_order_by`. Caveat (not a criterion failure): review WR-02 -- the service-level `cancel` does not itself apply the rule; isolation is route-level, exactly as the criterion states |
| 4 | The breaking `Principal` change is recorded in `MIGRATION.md` §9.2 and the `cargo semver-checks` allowlist | VERIFIED | `MIGRATION.md` §9.2 carries the `paladin-web | Principal` row (new required `tenant_id`, `Principal::new`, `read_scope`, marked `Y`, D-27 measurement recorded) plus `AgentAuthConfig`, `ApiKeyConfig`, `BearerTokenAuthConfig`, `SubmitRun`, `ForkRun`, `RunSubmissionPort` (`Y`) and `Run`, `RunQuery`, `RunScope`, `PaladinExecutorPort`, `StreamingExecutorPort`, `AuthConfig`, the principal types (`N`); §9.4 records migration 008, §9.5 the required `tenant` keys and the four boot rejections, §9.6 the read-scope change and `RunResponse.submitted_by`, §9.8 the one required config edit. `.cargo/semver-checks-allowlist.toml` has mirrored entries (`migration_row = "paladin-web | Principal"`, `requirement_id = "TENANT-01"`, ...). `./scripts/check-migration-allowlist.sh` run in this session: set-equal both directions, exit 0 |

**Score:** 4/4 roadmap success criteria verified (0 present-but-behavior-unverified).

### Observable Truths -- PLAN must_haves (44 truths)

| Plan | Truths | Status | Evidence / notes |
| ---- | ------ | ------ | ---------------- |
| 40-01 | 10 | 10 VERIFIED | Tracer, D-02 spoofing, bearer ordering and fail-closed, every-source-yields-a-tenant, `TenantId::new` rules (`principal.rs:83-98`: empty, >128 bytes, any whitespace, non-printable-ASCII rejected; exactly 128 accepted; serde deserialize validates -- `tenant_id_*` tests passed), `RunReadScope` semantics (4 tests passed), `Option<PrincipalRef>` on `SubmitRun`/`ForkRun`/`cancel` (`run_submission_port.rs:34,55,270`) with `None` skipping `allowed_roles` (`submission.rs:226`), role never persisted (`PrincipalRef::attribution` drops it; test passed), boot refusal messages (`paladin-server.rs:380,406`; tests `build_auth_config_fails_closed_when_an_api_key_has_no_tenant`, `build_auth_config_requires_a_bearer_tenant_when_bearer_is_enabled` exist -- `--bin paladin-server --features web-server` target not rebuilt here for disk reasons, covered by the 40-06 gate run), migration 008 + SQLite semantics (tests passed), §9.2/9.4/9.5 + allowlist set-equality (script exit 0) |
| 40-02 | 9 | 9 VERIFIED | Scoped list on all three adapters, empty page shape, predicate-before-ORDER-BY (SQL-text test + live keyset walk passed on SQLite and in-memory; Postgres SQL-text test passed, live clause CI-only), D-14 AND composition (`tenant_scoped_run_list_e2e` passed), `RunQuery.scope` default `All` with `list_runs` the only narrowing caller (grep: `scope:` set only at `run_controller.rs:987`), unattributed rows NULL/never listed under `Tenant`, attribution immutable under every mutating method (`attribution_survives_every_status_and_attempt_update` passed on SQLite and in-memory), single-INSERT on both Postgres paths (constants :56/:92 carry `tenant_id, api_key_id`; `insert_constants_carry_the_attribution_columns`), fork attribution (`submission.rs:456-457`; `fork_run_completes_from_waypoint`) |
| 40-03 | 7 | 6 VERIFIED, 1 ABSTAINED (`insufficient_spec`) | `validate()` exists, never clamps, called first at boot; empty/whitespace/non-printable tenant rejected with the two exact messages (tests passed); duplicate `name`/`key` rejected naming names only (`validate_rejects_a_duplicate_key_value_without_printing_it` passed); empty `key` and invalid `name` rejected; bearer tenant required iff enabled; `AuthConfig` keeps exactly `enabled, api_keys, bearer_token` (`agents.rs:121-130`); onboarding template emits no `http.auth` block (grep over `src/application/cli` finds only the Rust parameter `api_keys: &HashMap<K, String>` -- WINDOWS row 57, waived); `http-service-host.md:82,103-115` documents the mapping, `bearer_token.tenant`, the read rule and the 404. **Backstop truth** (mapping immutable after boot): abstained -- see Human Verification item 2 |
| 40-04 | 6 | 6 VERIFIED | `from_attribution` is the only attribution-to-scope mapping (grep: worker `:981`, `:1272`, `agent_auth.rs:89`); engine run settles under `(acme, svc-a)` / sentinel for `None` (worker tests passed); `RunScope.ledger_scope` + `with_ledger_scope` (`run_scope.rs:91,151`), agent-loop settle reads it (`paladin_execution_service.rs:1295-1298`, `:3273`, `:3609` fallback only when `None`); defaulted `execute_scoped`/`execute_stream_scoped` on both ports (`paladin_executor_port.rs:112`, `streaming_executor_port.rs:126`), service overrides (`:3225`, `:3267`), all three handlers call them (`agent_controller.rs:321,619,633,703`; five `*_attributes_spend_to_*` tests exist, incl. `open_access_execute_attributes_spend_to_the_open_access_tenant`); 007 migrations, `TreasuryLedgerPort`, ledger adapters, `treasury.rs` CLI and migrations 002-006 byte-unchanged since `629ef660` (`git diff --stat` empty) |
| 40-05 | 6 | 6 VERIFIED | `load_visible_run` on all four `/runs/{run_id}*` handlers and `permits` nowhere else in paladin-web; route matrix reads operations from the OpenAPI doc and drifts closed (test passed); same-tenant peer 200/202, foreign 404 byte-identical (test passed); cross-tenant cancel never reaches the port and writes no flag (`cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` passed over on-disk SQLite); `RunAttributionDto { tenant_id, api_key_id }` (`run_controller.rs:378`), `RunResponse.submitted_by` (:434), `openapi.json` carries `RunAttributionDto` (2 hits) and `submitted_by`; `tests/openapi_golden_v0_9.rs` unchanged since `629ef660`; module docs rewritten, `platform-api.md:397-417` states the tenant rule, the 404, `?tenant_id` ignored, and that `/v1/threads/*` is not yet scoped; §9.6 rows present |
| 40-06 | 6 | 6 VERIFIED (2 with plan-text notes) | 15 Phase 40 §9.2 rows carry D-27 results, allowlist set-equal (script exit 0), `paladin-web | Principal` reduced to `struct_marked_non_exhaustive` as measured; §9.5 boot rejections and §9.8 step present; ADR-0054 exists with the seven headings, `Accepted`, `## Code Conformance` = conforms, indexed in `PROMOTION.md:74` with `Next free ADR number: 0055` (:76) and `PROJECT.md:1712`; **WINDOWS row 32**: the truth says `fixed` -- the row has been `waived` since Phase 29 and the ledger tool has no `waived -> fixed` transition, so the literal is unsatisfiable by design; its closing condition (per-caller/tenant filter across controller and adapters) is met in code and recorded in ADR-0054 and the CHANGELOG -- verified on intent, plan-text issue; row 58 (thread routes) filed open with closing condition and owner; root `CHANGELOG.md` `[Unreleased]` has Added (:143,157,172), Changed (:196-219) and Breaking Changes (:223-227) Phase 40 entries and all four crate CHANGELOGs name their changes; facade `src/core/platform/mod.rs:31` re-exports `principal`; `.project/current-exports.txt` refreshed (carries `execute_scoped`, `execute_stream_scoped`, `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, `cancel`'s `Option<principal::PrincipalRef>`, the `principal` re-export); gate truth accepted per operator instruction on the 40-06 gate run at `be3a9030` with only `.planning/` commits since (see Human Verification item 4) |

**Plan-truth score:** 43/44 verified, 1 abstained (`insufficient_spec`, backstop tier), 0 failed.

### Deferred Items

| # | Item | Addressed In | Evidence |
| --- | ---- | ------------ | -------- |
| 1 | Schedule-fired runs persist `submitted_by = None` and settle under the `unattributed` sentinel (`schedule/service.rs:347` `requested_by: None`) | Phase 41 | 40-CONTEXT D-10; ADR-0054 rule 3; Phase 41 SC 2 needs a principal on every run path |
| 2 | `/v1/threads/*` routes are not tenant-scoped (threads carry no tenant) | Phase 41 planning or v0.11 hygiene | WINDOWS.md row 58 (open, closing condition + owner); ADR-0054 Downstream Consumers; `platform-api.md` states it |

### Required Artifacts

`gsd-tools query verify.artifacts` over the six plans: 21/22 pass. Wiring (Level 3) confirmed by grep for each; Level 4 data-flow is not applicable (no UI-rendered data) except `RunResponse.submitted_by`, which is populated from `Run.submitted_by` in the handler DTO mapping and proven by `run_response_submitted_by_is_null_for_an_unattributed_run` and the tracer's `submitted_by` assertions.

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/paladin-core/src/platform/container/principal.rs` | `TenantId`, `TenantIdError`, `TENANT_ID_MAX_LEN`, `PrincipalRef`, `RunAttribution`, `RunReadScope` | VERIFIED | 342 lines, 9 unit tests (all passed), registered `pub mod principal;` (`container/mod.rs:36`), imported by `agent_auth.rs`, `run.rs`, `run_repository_port.rs`, all three adapters |
| `crates/paladin-core/src/platform/container/run.rs` | `submitted_by`, `with_submitted_by` | VERIFIED | :417, :462; consumed by `submission.rs`, adapters, `permits` |
| `crates/paladin-storage/migrations/{sqlite,postgres}/008_add_run_attribution_columns.sql` | columns + `idx_runs_tenant_submitted` | VERIFIED | both present, identical DDL shape; applied by the embedded migrator (`migration_008_...` test passed) |
| `crates/paladin-web/src/agent_auth.rs` | `Principal.tenant_id`, `Principal::new`, `read_scope`, `bearer_tenant`, `From<&Principal> for PrincipalRef` | VERIFIED | :54, :59, :76, :116, :93 |
| `crates/paladin-web/src/run_controller.rs` | `load_visible_run`, `RunAttributionDto`, `RunResponse.submitted_by`, route matrix test | VERIFIED | :849, :378, :434, :3111 |
| `src/application/services/run/http_surface_tests.rs` | `tenant_scoped_run_read_tracer` | VERIFIED | :325; also `tenant_scoped_run_list_e2e` (:536), `cross_tenant_cancel_...` (:738); all passed |
| `crates/paladin-ports/src/output/run_repository_port.rs` | `RunQuery.scope` | VERIFIED | :79, `Default = All` |
| `crates/paladin-storage/src/run/contract_tests.rs` | six attribution/scoped-list clauses | VERIFIED | :827-1127; wired 6/6 into `sqlite.rs`, `in_memory.rs`, `postgres.rs` |
| `crates/paladin-storage/src/run/postgres.rs` | attribution columns + scoped `list_query` | VERIFIED | :56-92, :277-290, :332-363; DB-free tests passed, live clauses CI-only |
| `src/config/agents.rs` | `AuthConfig::validate` | VERIFIED | :180; 11 tests + doctest |
| `docs/src/deployment-topologies/http-service-host.md` | tenant mapping docs | VERIFIED | :82, :103-115 |
| `crates/paladin-core/src/platform/container/treasury_ledger.rs` | `from_attribution` | VERIFIED | :113 |
| `crates/paladin-core/src/platform/container/run_scope.rs` | `ledger_scope`, `with_ledger_scope` | VERIFIED | :91, :151 |
| `crates/paladin-ports/src/output/paladin_executor_port.rs` / `streaming_executor_port.rs` | defaulted scoped methods | VERIFIED | :112 / :126; overridden by `PaladinExecutionService` |
| `crates/paladin-web/openapi.json` | `RunAttributionDto`, `submitted_by` | VERIFIED | present; golden v0.9 test file unchanged |
| `docs/src/api-reference/platform-api.md` | tenant read-scope contract | VERIFIED | :397-417 |
| `.planning/decisions/0054-tenant-scoped-run-reads.md` | ADR-0054 | VERIFIED | seven headings, `## Code Conformance` conforms |
| `src/core/platform/mod.rs` | facade re-export | VERIFIED | :31 |
| `.project/current-exports.txt` | contains `RunReadScope` | PASSED (override) | Override: cargo-public-api lists the re-exported module as one `pub use ... principal` line (:7046) -- accepted via WINDOWS.md row 59 waiver, 2026-09-29 |

### Key Link Verification

`gsd-tools query verify.key-links` reported 10/16 because six plan patterns are double-escaped (`\\(`) and the tool rejects or misses them; each of those six was confirmed by direct grep.

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `agent_auth.rs` | `principal.rs` | `RunReadScope::for_principal`; `From<&Principal> for PrincipalRef` | WIRED | :77, :93 |
| `run_controller.rs` | `principal.rs` | `load_visible_run` calls `principal.read_scope().permits(&run)` | WIRED | :860 (tool pattern miss; grep hit) |
| `submission.rs` | `run.rs` | `with_submitted_by(principal_ref.attribution())` on submit/fork | WIRED | :288, :457 |
| `sqlite.rs` | migration 008 | INSERT/SELECT constants name `tenant_id, api_key_id` | WIRED | :48-87 |
| `paladin-server.rs` | `agent_auth.rs` | `build_auth_config` parses tenants, `Principal::new`, `bearer_tenant` | WIRED | :377-411 |
| `run_controller.rs` | `run_repository_port.rs` | `list_runs` sets `scope: principal.read_scope()` | WIRED | :987 (tool pattern miss; grep hit) |
| `sqlite.rs` | `principal.rs` | `RunReadScope::Tenant` pushes ` AND tenant_id = ` before ORDER BY | WIRED | :526-540 |
| `in_memory.rs` | `principal.rs` | `query.scope.permits(r)` | WIRED | :166 (tool pattern miss; grep hit) |
| `paladin-server.rs` | `agents.rs` | `cfg.validate()?` first in `build_auth_config` | WIRED | :361 (tool pattern miss; grep hit) |
| `worker.rs` | `treasury_ledger.rs` | `LedgerScope::from_attribution(run.submitted_by.as_ref())` on both paths | WIRED | :981, :1272 |
| `paladin_execution_service.rs` | `run_scope.rs` | settle writer reads `scope.ledger_scope` | WIRED | :1295-1298, :3273 |
| `agent_controller.rs` | `paladin_executor_port.rs` | handlers call `execute_scoped`/`execute_stream_scoped` with `principal.ledger_scope()` | WIRED | :318-321, :614-633, :699-703 (tool regex error; grep hit) |
| `run_controller.rs` | `principal.rs` | stream/cancel/webhook-deliveries call `load_visible_run` | WIRED | :1060, :1131, :1223 (tool regex error; grep hit) |
| `run_controller.rs` | `openapi.json` | `submitted_by` drift guard | WIRED | `openapi_matches_committed_baseline`; `submitted_by` present in both |
| `MIGRATION.md` | `.cargo/semver-checks-allowlist.toml` | set-equality of §9.2 `Y` pairs and `migration_row` | WIRED | `check-migration-allowlist.sh` exit 0 |
| `PROMOTION.md` | ADR-0054 | index row + `Next free ADR number: 0055` | WIRED | :74, :76 |

### Behavioral Spot-Checks

Named tests only; the full workspace suite was not run (see Human Verification item 4).

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| `TenantId`/`RunReadScope` rules | `cargo test -p paladin-ai-core --lib principal::` | 9 passed, 0 failed | PASS |
| Scoped keyset walk, attribution immutability, half-attributed rejection, migration 008, Postgres SQL-text scope position | `cargo test -p paladin-storage --features sqlite,postgres --lib -- list_scoped_pagination_has_no_gap_or_overlap attribution_survives_every_status_and_attempt_update half_attributed_row_is_a_serialization_error list_query_applies_the_tenant_scope_before_order_by unattributed_run_is_stored_as_sql_null migration_008` | 11 passed, 0 failed (Postgres live variants self-skip without a DB) | PASS |
| Route matrix, foreign-run 404, every principal source has a tenant, bearer fail-closed, `?tenant_id` ignored, `submitted_by` null for unattributed | `cargo test -p paladin-web --lib -- every_run_id_route_hides_foreign_runs_behind_the_missing_run_404 every_principal_source_yields_a_tenant verified_bearer_token_without_configured_tenant_fails_closed get_run_hides_a_foreign_tenant_run_behind_the_missing_run_404 list_runs_ignores_a_tenant_query_parameter run_response_submitted_by` | 6 passed, 0 failed | PASS |
| End-to-end tracer, scoped list, cross-tenant cancel, worker/agent-loop/stream ledger attribution, config validation | `cargo test -p paladin-ai --lib -- tenant_scoped cross_tenant_cancel ledger_scope validate_rejects from_attribution settles_under` | 34 passed, 0 failed (incl. `tenant_scoped_run_read_tracer`, `tenant_scoped_run_list_e2e`, `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag`, `attributed_engine_run_settles_under_its_submitting_principal_scope`, `unattributed_engine_run_settles_under_the_unattributed_sentinel`) | PASS |
| Register set-equality | `./scripts/check-migration-allowlist.sh` | set-equal both directions | PASS |
| `paladin-server` boot fail-closed tests (`build_auth_config_*`, 7 tests at `paladin-server.rs:734-883`) | not run (`--bin paladin-server --features web-server` rebuild too large for the disk budget) | exist; covered by the 40-06 gate run | SKIP |
| Postgres live clauses, coverage ≥ 82 %, `e2e_platform_api` integration target | n/a (no Docker) | CI-only | SKIP |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes are declared by any Phase 40 PLAN or SUMMARY and none exist in the tree for this phase. The one declared runnable check, `./scripts/check-migration-allowlist.sh`, was executed in this process: exit 0.

### Requirements Coverage

REQUIREMENTS.md maps exactly TENANT-01, TENANT-02 and PLAT-07 to Phase 40 (rows 219, 220, 238, all `Complete`); every PLAN `requirements` ID is one of these three; no orphaned requirement.

| Requirement | Source Plans | Description | Status | Evidence |
| ----------- | ------------ | ----------- | ------ | -------- |
| TENANT-01 | 40-01, 40-03, 40-06 | Operator config maps each API key to a tenant; `Principal` carries `tenant_id`; a caller cannot assert its own tenant; breaking change recorded in §9.2 and the allowlist | SATISFIED | SC 1 and SC 4 evidence above |
| TENANT-02 | 40-01, 40-02, 40-04, 40-05, 40-06 | Every run records its submitting principal (API key id and tenant) so spend and run reads are attributed | SATISFIED | SC 2 evidence; ledger scope from `from_attribution` on worker and HTTP agent paths; `RunResponse.submitted_by` |
| PLAT-07 | 40-01, 40-02, 40-05, 40-06 | `GET /runs` and every `/runs/{id}*` read route return only visible runs via one shared authorization function; foreign run is 404; admin override decided in-phase | SATISFIED | SC 3 evidence; Admin bypass decided (D-11, ADR-0054 rule 3); WINDOWS row 32's closing condition met |

### Prohibitions (judgment-tier, non-authoritative -- flagged for human review)

All 20 `must_haves.prohibitions` items carry `verification: null`; they are disposed as judgment-tier per ADR-550 D4. Verdicts below are the verifier's LLM-judge reading and are NOT authoritative.

| Prohibition (condensed) | Plans | Evidence | Verdict |
| ----------------------- | ----- | -------- | ------- |
| No client-asserted tenant (header/query/body) | 01, 03, 05 | `authenticate()` reads config only; `SubmitRunRequest` five fields; tracer + `list_runs_ignores_a_tenant_query_parameter` passed | held (test-backed) |
| No 403 or distinct body for a hidden run | 01, 05 | route matrix + tracer byte-identical 404 passed | held (test-backed) |
| Never log/echo/persist an API key value | 01, 03, 04 | no `format!`/log interpolates `.key`; `validate_rejects_a_duplicate_key_value_without_printing_it` passed; `api_key_id` is the configured name; 40-06 manual review recorded clean. Pre-existing `#[derive(Debug)]` on `ApiKeyConfig` (present at `629ef660`) noted, out of scope | held (partly test-backed; human review recommended) |
| No `RUN_SCHEMA_VERSION` bump / X-04 guard change | 01, 02, 06 | `run.rs:50` `"v1"`; `git diff` shows no guard change | held |
| Never persist the `unattributed` sentinel in `runs.tenant_id`/`api_key_id` | 01, 02 | `unattributed_run_is_stored_as_sql_null` passed (SQLite; Postgres DB-free constant check) | held (test-backed) |
| No post-filter of a fetched page | 02, 05 | predicate inside `QueryBuilder` (`sqlite.rs:526`, `postgres.rs:349`); `list_query_applies_the_tenant_scope_before_order_by` passed | held (test-backed) |
| 007 ledger schema / `TreasuryLedgerPort` / adapters / queries unchanged | 04, 06 | `git diff --stat 629ef660..HEAD` empty over those paths | held (observed) |
| No `TokenBudget`/`Commissary` rename, no new officer word | 01, 04, 06 | `TenantId`, `PrincipalRef`, `RunAttribution`, `RunReadScope` are plain identifiers | held (judgment) |
| No Snyk step / not recorded as blocked on one | 06 | no SUMMARY records a Snyk step | held (judgment) |
| No implicit/default tenant | 03 | `validate()` rejects an empty tenant; no `"default"` literal in `agents.rs`/`paladin-server.rs` | held (test-backed) |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| (25 phase-modified files) | -- | `TBD` / `FIXME` / `XXX` | none found | debt-marker gate passes |
| (25 phase-modified files) | -- | `TODO` / `HACK` / `todo!` | none found in phase code | -- |
| `src/application/services/paladin/paladin_execution_service.rs` | 3789-5648 | `unimplemented!()` | Info | all 11 are inside `#[cfg(test)]` modules and pre-date the phase (11 at `629ef660`, 11 now) |
| `crates/paladin-ports/src/output/run_repository_port.rs` | 30-36 | stale Phase 27 module doc ("still needs a real (non-`todo!`) implementation") | Info | pre-existing (present at `629ef660`); every method is implemented and the contract suite runs on all three adapters |
| `src/config/agents.rs` | 84 | `#[derive(Debug)]` on `ApiKeyConfig` (carries the secret `key`) | Info | pre-existing at `629ef660`; not Debug-formatted anywhere in the Phase 40 diff; candidate for a hygiene pass |
| `crates/paladin-web/src/run_controller.rs` | 856, 990 | `ApiError::internal(e.to_string())` echoes repository error text (review WR-03/WR-04) | Warning | pre-existing pattern; Phase 40 adds attribution/tenant validation texts to it; corrupt foreign row answers 500 not 404 -- Human Verification item 5 |
| `src/application/services/run/submission.rs` | 262-290, 437-466 | caller-supplied `thread_id` accepted without a tenant check on submit/fork (review WR-01) | Warning | cross-tenant thread resume/fork through a write route; scope decision -- Human Verification item 1 |
| `src/application/services/run/submission.rs` | 333-367 | `cancel` ignores `PrincipalRef.tenant_id` (review WR-02) | Warning | isolation is route-level only; Human Verification item 5 |

### Human Verification Required

See the `human_verification` frontmatter list (five items): (1) WR-01 scope decision on cross-tenant thread resume/fork via `POST /v1/runs {thread_id}`; (2) the abstained backstop truth on post-boot immutability of the key-to-tenant mapping; (3) the 20 judgment-tier prohibitions, especially the credential-handling review; (4) CI-only evidence (Postgres live clauses, coverage, integration target, the full-suite figure); (5) disposition of WR-02/WR-03/WR-04.

### Gaps Summary

No gap blocks the phase goal. All four roadmap success criteria are verified against the tree with passing behavioral tests, and requirements TENANT-01, TENANT-02 and PLAT-07 are satisfied. The status is `human_needed`, not `passed`, because (a) one 40-03 truth is tagged `verification: backstop` and abstains without a held-out test, (b) every prohibition is judgment-tier and must be flagged in an autonomous run, (c) several evidence items are CI-only in this sandbox, and (d) the post-phase code review's WR-01 exposes a cross-tenant mutation path (`POST /v1/runs` with another tenant's `thread_id`, and fork) that sits between PLAT-07's read-route wording and the D-14 thread-tenancy deferral -- WINDOWS.md row 58 does not currently name it, so a human must decide whether it is fixed now or recorded as deferred.

Known accepted limitations, not counted as gaps: `/v1/threads/*` unscoped (row 58), schedule-fired runs unattributed (D-10, Phase 41), WINDOWS rows 57 and 59 waived plan-text/tool-design issues, WINDOWS row 32 `waived` rather than `fixed` (ledger state machine has no such transition; closing condition met).

---

_Verified: 2026-09-29T12:45:00Z_
_Verifier: gsd-verifier_
