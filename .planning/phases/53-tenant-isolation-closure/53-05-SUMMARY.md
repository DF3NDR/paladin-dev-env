---
phase: 53-tenant-isolation-closure
plan: 05
subsystem: tenant-isolation
tags: [rust, tenant-isolation, job-poll, job-store, axum, openapi, semver, tdd]

requires:
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-01 RunReadScope (All / Tenant) and Principal::read_scope()"
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-03 hidden-equals-missing comparison pattern (status, sorted headers, swapped body bytes)"
provides:
  - "JobStore::create_owned and JobStore::get_visible: per-job (agent id, TenantId) owner in private Inner.owners, evicted with the job"
  - "agent_controller::get_job_scoped routed with operation_id get_job; openapi.json byte-identical"
  - "get_job kept (signature, body, utoipa annotation), unrouted, no #[deprecated]"
  - "MIGRATION.md 11.1 row M-B-26, 11.6 job paragraph, 11.7 get_job superseded; paladin-web CHANGELOG entries"
affects: [53-07, 53-08]

tech-stack:
  added: []
  patterns:
    - "Owner kept in private store state beside the record, so a published record type and its wire shape never change"
    - "One private insert-and-evict body behind create and create_owned, so the job map and owner map cannot drift"
    - "Re-route a published handler by adding a scoped sibling with the same operation_id and the same rustdoc, keeping the old handler and its #[utoipa::path] (which generates a public __path_ item)"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/job_store.rs
    - crates/paladin-web/src/agent_controller.rs
    - crates/paladin-web/CHANGELOG.md
    - MIGRATION.md

key-decisions:
  - "get_job keeps its #[utoipa::path] annotation: removing it would delete the generated public __path_get_job item, itself a patch-gate break. The old handler's rustdoc gains a leading superseded paragraph and keeps the original two paragraphs"
  - "No #[deprecated] on get_job (PD-05-1): the attribute is recorded in rustdoc and MIGRATION 11.7 and deferred to the next minor"
  - "Admin (All) reads any job under any agent id; Tenant needs the exact (agent id, tenant) pair; legacy create() jobs are Admin-only (PD-05-2)"
  - "Existing poll_job and get_job_unknown_returns_404 moved onto get_job_scoped with admin(); unscoped_get_job_is_kept_for_embedders pins get_job (PD-05-3)"

patterns-established:
  - "Router tracer over versioned_agent_parts with an auth-enabled AgentApiState and X-API-Key principals; Observed {status, sorted headers minus content-length, body} compared hidden versus unknown"

requirements-completed: [TENANT-07]

coverage:
  - id: D1
    description: "JobStore records (agent, tenant) at create_owned; get_visible admits All always and Tenant only for the exact owner pair; legacy jobs Admin-only; eviction removes the owner entry"
    requirement: "TENANT-07"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib job_store (10 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Served poll route: owner and Admin 200, foreign tenant 404 byte-equal to unknown job (tracer)"
    requirement: "TENANT-07"
    verification:
      - kind: integration
        ref: "agent_controller::tests::job_poll_is_scoped_to_the_enqueuing_agent_and_tenant"
        status: pass
    human_judgment: false
  - id: D3
    description: "AC-12 matrix: owner and a second owner-tenant key 200; foreign tenant 404; owner tenant under another agent 404; Admin 200 on both agents; every 404 equal to unknown"
    requirement: "TENANT-07"
    verification:
      - kind: integration
        ref: "agent_controller::tests::job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass"
        status: pass
    human_judgment: false
  - id: D4
    description: "An evicted job is the unknown-job 404 for owner and Admin"
    requirement: "TENANT-07"
    verification:
      - kind: integration
        ref: "agent_controller::tests::evicted_job_poll_is_404_for_every_role"
        status: pass
    human_judgment: false
  - id: D5
    description: "Tenant hints in header or query change nothing (foreign key claiming acme stays hidden; owner key claiming globex stays visible)"
    requirement: "TENANT-07"
    verification:
      - kind: integration
        ref: "agent_controller::tests::job_poll_ignores_tenant_hints"
        status: pass
    human_judgment: false
  - id: D6
    description: "OpenAPI document unchanged: openapi.json not modified, committed-baseline and frozen v0.9 golden tests pass"
    requirement: "TENANT-07"
    verification:
      - kind: other
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline; cargo test -p paladin-web --test openapi_golden_v0_9; git diff --quiet -- crates/paladin-web/openapi.json"
        status: pass
    human_judgment: false
  - id: D7
    description: "Patch-level semver gate stays green for paladin-web"
    requirement: "TENANT-07"
    verification:
      - kind: other
        ref: "SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-web"
        status: pass
    human_judgment: false

duration: 40min
completed: 2026-10-31
status: complete
---

# Phase 53 Plan 05: Scoped job polls Summary

**`GET /v1/agents/{id}/jobs/{job_id}` now returns a job only to the tenant that enqueued it under the enqueuing agent id (or to an Admin): `JobStore` records the `(agent id, TenantId)` owner in private state, an additive `get_job_scoped` is routed with `operation_id = "get_job"`, and every other poll is the byte-identical `unknown job '<id>'` 404, with `openapi.json` unchanged and the paladin-web patch semver gate at 223/223.**

## Performance

- **Duration:** about 40 min wall clock (builds were warm)
- **Completed:** 2026-10-31
- **Tasks:** 2 (Task 1 tracer, Task 2 auto)
- **Files modified:** 4
- **Plan start SHA:** `5fac5ed029002b2b5a0c1e1cd114f05b1ed3c9dc`

## Accomplishments

- `JobStore`: private `Inner.owners: HashMap<String, (String, TenantId)>`; `create_owned(&self, agent_id, &Principal) -> String` and `get_visible(&self, job_id, agent_id, &RunReadScope) -> Option<JobRecord>` (with a doctest). `create` and `create_owned` share one private `insert` body that evicts the job and its owner entry together. `JobRecord`, `create` and `get` are untouched.
- `enqueue_job` records the owner with `create_owned(&id, &principal)` (the path agent id after `registry.get` succeeded).
- `get_job_scoped(State, Extension<Principal>, Path<(String, String)>)` answers `get_visible(.., &principal.read_scope())` or `ApiError::not_found("unknown job '<id>'")`; it carries `operation_id = "get_job"` and the original two rustdoc paragraphs verbatim, with the scoping explanation in a plain `//` block (the `enqueue_job` house pattern). `agent_openapi_router` now routes it in place of `get_job`.
- `get_job` keeps its signature, body and `#[utoipa::path]`; it is unrouted and its rustdoc says so. No `#[deprecated]` (PD-05-1).
- Docs: `MIGRATION.md` 11.1 row `M-B-26`, a 11.6 paragraph, a 11.7 bullet (`get_job` superseded by `get_job_scoped`; attribute deferred to the next minor because of `function_marked_deprecated`); paladin-web `CHANGELOG.md` `### Added` and `### Security`. 11.2 stays empty; sections 9 and 10 untouched.

## Task Commits

1. **Task 1 (tracer): owner recorded at enqueue and enforced on the served poll route** - `cc0077527` (feat)
2. **Task 2: full poll matrix, existing tests moved to the scoped handler, MIGRATION and CHANGELOG** - `010ee3158` (test)

Tracer feedback gate (auto mode active): the tracer's `<verify>` (job_store tests, the router tracer, both OpenAPI tests, `git diff --quiet` on `openapi.json`, the router and deprecation greps) was green after the commit and before Task 2; "Tracer verified end-to-end -- expanding".

## Prove-then-lock

- **Job store tests** were written first and failed to compile (no `create_owned`, `get_visible` or `owners`).
- **Router tracer** was written first and failed on the existing behavior: the foreign key received `200` (`left: 200, right: 404`) until `get_job_scoped` was routed.
- Task 2's matrix, eviction and hint tests pass on the implementation from Task 1 (the plan puts the handler in Task 1 and the full proofs in Task 2); the matrix covers the same code path the tracer failed on and the wrong-agent and eviction rows exercise the `get_visible` branches already unit-tested in `job_store`.

## Verification

- `cargo test -p paladin-web`: lib 329 passed (`agent_controller` 71, `job_store` 10), integration tests 5 / 9 / 3 passed. `agent_controller` rose from 66 to 71 (tracer, matrix, eviction, hints, unscoped-get_job); `get_job_unknown_returns_404` still exists by name.
- `cargo fmt --check`, `cargo clippy -p paladin-web --all-targets -- -D warnings`, `make check-migration-allowlist`, `make check-changelogs`, `make api-surface` ("API surface unchanged", 4323 items): exit 0.
- `crates/paladin-web/openapi.json` unmodified; `openapi_matches_committed_baseline` and `openapi_golden_v0_9` pass without regeneration.
- **Semver line (Task 2 step 5):** `semver: paladin-web: 223 checks: 223 pass, 31 skip` and `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` (`SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-web`). Plan 53-08 is still the binding run.
- Manual credential-handling check (security.instructions.md): no log line, key, header or body content is added; the only new response text is the existing fixed `unknown job '<id>'` message, which echoes the caller's own path segment.
- Not run (pre-existing, unrelated): `run_api_wiring::tests::build_run_api_persists_*` (network, TESTS-01, Phase 55).

## Decisions Made

See `key-decisions` above; they follow PD-05-1 to PD-05-3.

## Deviations from Plan

**1. [Rule 1 - Public-surface safety] `get_job` keeps its `#[utoipa::path]` annotation**
- **Found during:** Task 1 (first draft moved the annotation to `get_job_scoped` and left `get_job` bare)
- **Issue:** `#[utoipa::path]` generates a public `__path_get_job` item; removing it is a removed public item, which the patch semver gate rejects. The plan said only that `get_job`'s signature and body stay unchanged.
- **Fix:** restored the annotation on `get_job` (without `operation_id`) with its original two rustdoc paragraphs under a new leading "superseded" paragraph. Only one of the two handlers is routed, so the OpenAPI document holds a single `get_job` operation.
- **Verified by:** `openapi_matches_committed_baseline`, `openapi_golden_v0_9` and the semver run above.
- **Commit:** `cc0077527`

No other deviations. The agent-router rustdoc list does not name the job routes, so it needed no edit.

## Known Stubs

None.

## Threat Flags

None. The change narrows an existing route; no new endpoint, auth path, file access or schema change. T-53-27 (an embedder still routing the unscoped `get_job`) is accepted as planned and documented in rustdoc and MIGRATION 11.7.

## Notes for plans 53-07 and 53-08

- ADR-0063 should record the job owner model: `(agent id, tenant)` in `JobStore` private state, Admin full bypass, legacy unowned jobs Admin-only, hidden equals missing, and the `get_job` / `get_job_scoped` split with the `#[deprecated]` attribute deferred to the next minor.
- `GET /v1/agents/{id}/jobs/{job_id}` is a behavioral change (row `M-B-26` in 11.1), not an API break: 11.2 is untouched and the paladin-web patch gate is green.
- Follow-up for the next minor: add `#[deprecated(note = "use get_job_scoped")]` to `get_job` and add its 11.2/11.7 handling then.

## Self-Check: PASSED

- FOUND: crates/paladin-web/src/job_store.rs (`pub fn create_owned`, `pub fn get_visible`)
- FOUND: crates/paladin-web/src/agent_controller.rs (`pub async fn get_job_scoped`, `routes!(get_job_scoped)`)
- FOUND commits `cc0077527`, `010ee3158`
