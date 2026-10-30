---
phase: 40-tenant-identity-run-read-scoping
plan: 05
subsystem: api
tags: [tenant-scoping, axum, utoipa, openapi, sqlite, run-routes, plat-07]

# Dependency graph
requires:
  - phase: 40-tenant-identity-run-read-scoping
    provides: "40-01: TenantId/PrincipalRef/RunAttribution/RunReadScope, Run.submitted_by, Principal::read_scope, load_visible_run on GET /runs/{id}; 40-02: RunQuery.scope applied inside every repository adapter"
  - phase: 27-platform-api-run-submission
    provides: run_router, RunApiState, stream_run/cancel_run/list_webhook_deliveries, SqliteRunRepository, RunSubmissionService
provides:
  - "load_visible_run is the entry point of all four /runs/{run_id}* routes: GET, GET /stream (before the SSE upgrade), POST /cancel (before RunSubmissionPort::cancel and its allowed_roles check), GET /webhook-deliveries (before list_for_run) -- D-13"
  - "every_run_id_route_hides_foreign_runs_behind_the_missing_run_404: route matrix enumerated from the router's own OpenAPI document (research Pitfall 10 guard)"
  - "cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag: real on-disk SQLite proof that a foreign cancel writes nothing"
  - "RunAttributionDto { tenant_id, api_key_id } + RunResponse.submitted_by (additive, always serialised, null when unattributed, never key value or role) -- D-19"
  - "regenerated crates/paladin-web/openapi.json (RunAttributionDto schema, nullable submitted_by, reworded/added 404 rows, run-store 501 notes)"
  - "published read-scope contract: run_controller module docs 'Read scope (PLAT-07)', platform-api.md 'Authentication and scopes', MIGRATION.md 9.6 entries + 9.2 RunResponse row extension -- D-18/D-21"
affects: [40-06, 41-allowances, phase-41, thread-route-scoping]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Route-set regression guard: enumerate operations from the router's own utoipa OpenApi (versioned_run_parts(..).1) and compare against an explicit table, so an unlisted route fails CI with an actionable message"
    - "Gate-before-port on every single-resource route: own-port 501 -> parse id -> require run store (501) -> load_visible_run -> route's own port; existing 501 tests stay green because the order is preserved"
    - "Recording port double keyed by api_key_id to prove a gate answers before the port is reached (zero foreign calls), complementing a real-store e2e that checks the persisted flag"
    - "Public handler rustdoc refers to a private helper with plain backticks, not an intra-doc link, so cargo doc -D warnings (CI) does not fail on private_intra_doc_links"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/openapi.json
    - src/application/services/run/http_surface_tests.rs
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md

key-decisions:
  - "Matrix lives in run_controller.rs (cargo test -p paladin-web, no Docker) and enumerates from the OpenAPI document rather than a hand list; the SQLite cancel proof lives in http_surface_tests.rs under -p paladin-ai (research Pitfall 7)"
  - "stream/cancel/webhook-deliveries now also answer 501 naming run_store.backend when the run store is unwired -- the gate needs the run row; recorded in MIGRATION.md 9.6 as part of the behaviour change"
  - "utoipa response descriptions written as single-line strings: a backslash-continued literal leaks indentation runs into the generated openapi.json"
  - "openapi_golden_v0_9.rs untouched (Pitfall 6): RunResponse and every /v1/runs* path postdate the frozen v0.9.0 baseline"
  - "tests/integration/e2e_platform_api_test.rs (web-server feature) not run locally: it needs a separate feature-unified workspace build at 11 GB free disk, and its single Admin key makes the gate a pass-through by construction; left to CI"

patterns-established:
  - "Route matrix from the OpenAPI document: any future /runs/{run_id}* route must join the table AND call load_visible_run"
  - "Visibility before role: for mutating single-resource routes the tenant gate runs before the assistant allowed_roles check so a 404 never becomes a 403 oracle"

requirements-completed: [PLAT-07, TENANT-02]

coverage:
  - id: D1
    description: "GET /stream, POST /cancel and GET /webhook-deliveries answer the missing-run 404 to another tenant and 200/202 to the owner, a same-tenant peer and an Admin, through load_visible_run"
    requirement: PLAT-07
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#every_run_id_route_hides_foreign_runs_behind_the_missing_run_404"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller (46 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The route matrix enumerates exactly the four /v1/runs/{run_id}* operations from the router's own OpenAPI document and fails on any unlisted route"
    requirement: PLAT-07
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#every_run_id_route_hides_foreign_runs_behind_the_missing_run_404 (published_run_id_operations)"
        status: pass
    human_judgment: false
  - id: D3
    description: "A cross-tenant cancel over on-disk SQLite is a 404 that never reaches RunSubmissionPort::cancel and leaves cancel_requested false; the owner's cancel then answers 202 and sets it"
    requirement: PLAT-07
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag"
        status: pass
    human_judgment: false
  - id: D4
    description: "RunResponse.submitted_by is { tenant_id, api_key_id } for an attributed run, null otherwise, never the key value or the role; openapi.json regenerated and the v0.9 golden untouched"
    requirement: TENANT-02
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#run_response_carries_submitted_by_and_never_the_key_or_role"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#run_response_submitted_by_is_null_for_an_unattributed_run"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline; cargo test -p paladin-web --test openapi_golden_v0_9 (7 passed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Read-scope contract published: run_controller module docs, platform-api.md 'Authentication and scopes', MIGRATION.md 9.6 entries and the 9.2 RunResponse row extension"
    requirement: PLAT-07
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh; RUSTDOCFLAGS=-D warnings cargo doc -p paladin-web --no-deps; acceptance greps in the plan"
        status: pass
    human_judgment: true
    rationale: "Prose accuracy of the operator-facing docs and register rows is a reading judgment; the automated checks only prove the required strings are present and the register parses"

# Metrics
duration: 15min
completed: 2026-09-29
status: complete
---

# Phase 40 Plan 05: Run-route read-scope closure Summary

**Every `/v1/runs/{run_id}*` route (stream, cancel, webhook-deliveries) now enters through `load_visible_run`, an OpenAPI-enumerated route matrix keeps it that way, a real-SQLite proof shows a foreign cancel writes nothing, and the read contract ships as `RunResponse.submitted_by`, module docs, platform-api.md and MIGRATION.md 9.6.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-09-29T01:42:07Z
- **Completed:** 2026-09-29T01:57:21Z
- **Tasks:** 2 (both TDD, 4 commits)
- **Files modified:** 5

## Accomplishments

- `stream_run`, `cancel_run` and `list_webhook_deliveries` gate on `load_visible_run` after their own-port 501 and id parsing and before the SSE upgrade / `RunSubmissionPort::cancel` / `list_for_run` respectively (D-13). `permits(` still appears exactly once in non-test code, and each of the four handlers calls `load_visible_run` exactly once.
- `every_run_id_route_hides_foreign_runs_behind_the_missing_run_404` reads the `/v1/runs/{run_id}*` operations from `versioned_run_parts(state).1` (all eight HTTP methods per path item), fails on any mismatch with the four-row table, and for every row proves owner / same-tenant peer / Admin succeed, a foreign tenant gets a 404 byte-identical to the missing-run 404 once ids are swapped, and the recording cancel double saw calls only from `svc-a`, `svc-a2` and `ops` (research Pitfall 10, T-40-18..T-40-22).
- `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` drives the real `run_router` + `RunSubmissionService` over an on-disk `SqliteRunRepository`: key b's cancel is 404 and `cancel_requested` stays false with the run still `Queued`; key a's cancel is 202 and the flag is set (T-40-19).
- `RunAttributionDto { tenant_id, api_key_id }` with `From<&RunAttribution>`; `RunResponse.submitted_by` is additive, always serialised, `null` when unattributed, and tests assert exactly two keys, no configured key value and no `role` anywhere in the body (D-19).
- `crates/paladin-web/openapi.json` regenerated once via `openapi_matches_committed_baseline` (53 insertions, 14 deletions before the whitespace fix): `RunAttributionDto` schema, nullable `submitted_by`, `get_run`/`list_runs` tenant-scope descriptions, reworded 404s on `stream`/`cancel`, a new 404 row on `webhook-deliveries`, and run-store notes on the three 501s. `tests/openapi_golden_v0_9.rs` untouched and passing (7/7).
- Module docs: "Read scope (PLAT-07) -- per tenant, with an operator bypass" replaces the WR-03 deployment-wide section; `platform-api.md` "Authentication and scopes" describes the tenant model, the 404 rule, `submitted_by`, and states `/v1/threads/*` is not yet scoped; MIGRATION.md 9.6 gains the read-scope and `submitted_by` entries and the 9.2 `RunResponse` row is extended (allowlist set-equality check passes).

## Task Commits

Each task was committed atomically (TDD: test then feat):

1. **Task 1: Stream, cancel and webhook-deliveries enter through load_visible_run; the route matrix and a real-SQLite cross-tenant cancel proof**
   - `86516aad` (test) failing route matrix and cross-tenant cancel tests -- RED confirmed: `/stream` answered 200 and `/cancel` 202 to a foreign tenant
   - `a175d652` (feat) gate the three handlers; seed existing tests with a visible run; add run-store-unwired 501 tests
2. **Task 2: RunResponse.submitted_by, regenerated openapi.json, and the published read-scope contract**
   - `8b8a9913` (test) failing `submitted_by` DTO tests
   - `03cdfd92` (feat) DTO + field, handler docs, regenerated baseline, module docs, platform-api.md, MIGRATION.md

**Plan metadata:** see the final `docs(40-05)` commit.

## Files Created/Modified

- `crates/paladin-web/src/run_controller.rs` - three gated handlers, `RunAttributionDto`, `RunResponse.submitted_by`, rewritten module docs and handler rustdoc/utoipa, the matrix test, two DTO tests, three run-store-unwired 501 tests, `repository_with` fixture helper
- `crates/paladin-web/openapi.json` - regenerated HEAD baseline
- `src/application/services/run/http_surface_tests.rs` - `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag`
- `docs/src/api-reference/platform-api.md` - "Authentication and scopes": Reads row and the tenant read-scope paragraphs replacing "What a `GET` can see today"
- `MIGRATION.md` - 9.6 "Run reads are tenant-scoped" and "RunResponse gains submitted_by" entries; 9.2 `paladin-web | RunResponse` row extended with the Phase 40 sentence

## Decisions Made

- The three secondary routes now also answer `501` naming `run_store.backend` when the run store is unwired (they previously needed only their own port). The gate reads the run row, so this is inherent to D-13; recorded in MIGRATION.md 9.6 and covered by three new tests. Production wiring (`src/infrastructure/web/run_api_wiring.rs`) always sets the repository together with the events and webhook ports, so no deployed configuration changes behaviour.
- utoipa `description` strings are written on one line: a backslash-continued literal keeps the continuation indentation in the generated JSON (and `cargo fmt` then joins the lines with the spaces baked in).
- Public handler rustdoc names `load_visible_run` in plain backticks rather than an intra-doc link, because CI builds docs with `RUSTDOCFLAGS=-D warnings` and a public item linking to a private one is a `private_intra_doc_links` warning. Verified with `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --no-deps` (clean).
- `tests/integration/e2e_platform_api_test.rs` (target `e2e_platform_api`, requires the `web-server` feature) exercises `/stream` and `/webhook-deliveries` with a single Admin key; it was not run locally because the feature-unified build would add a second workspace artifact set at 11 GB free disk. The gate is a pass-through for an Admin key by construction (D-11), so no behaviour change is expected there; CI runs it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Run-store-unwired 501 coverage on all three routes**
- **Found during:** Task 1
- **Issue:** The plan's behaviour list requires "with the run store unwired they answer 501 naming the run store" but named no test for it.
- **Fix:** Added `run_stream_returns_501_naming_the_run_store_when_only_events_are_wired`, `cancel_run_returns_501_naming_the_run_store_when_only_submission_is_wired` and `list_webhook_deliveries_returns_501_naming_the_run_store_when_only_deliveries_wired`, each asserting the body names `run_store.backend`.
- **Files modified:** crates/paladin-web/src/run_controller.rs
- **Committed in:** a175d652

**2. [Rule 1 - Bug] Whitespace runs in generated OpenAPI descriptions**
- **Found during:** Task 2 (openapi regeneration)
- **Issue:** Two new multi-line `description = "... \` utoipa strings emitted runs of 13 spaces into `openapi.json`.
- **Fix:** Collapsed the two literals to single lines and regenerated; verified no space runs in the affected descriptions.
- **Files modified:** crates/paladin-web/src/run_controller.rs, crates/paladin-web/openapi.json
- **Committed in:** 03cdfd92

**3. [Rule 1 - Bug] Private intra-doc links in public handler docs**
- **Found during:** Task 2
- **Issue:** Four `[`load_visible_run`]` links from `pub async fn` docs to the private helper would fail CI's `cargo doc -D warnings`.
- **Fix:** Plain backticks; `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --no-deps` passes.
- **Files modified:** crates/paladin-web/src/run_controller.rs
- **Committed in:** 03cdfd92

---

**Total deviations:** 3 auto-fixed (1 missing test coverage, 2 doc-generation bugs)
**Impact on plan:** No scope change; all three are correctness of the artefacts the plan already required.

## Issues Encountered

- None blocking. Disk fell from 13 GB to 11 GB free during the run; no `cargo clean`, and the `web-server`-feature integration target was deliberately left to CI (see Decisions Made).
- Docker-gated (Postgres) tests were not run; none of this plan's changes touch a Postgres adapter.

## Known Stubs

None.

## Threat Flags

None -- every new surface (the three gated routes, the matrix, the DTO) is already in the plan's threat register (T-40-18..T-40-23). No new endpoint, auth path, file access or schema change was introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- PLAT-07 success criterion 3 holds on the whole route set; 40-06 can close `WINDOWS.md` row 32 and add the thread-route row (T-40-23), write ADR-0054, the CHANGELOG entries, and refresh the API surface (`make api-surface-update`: `RunAttributionDto` and `RunResponse.submitted_by` are new public surface in `paladin-web`).
- `MIGRATION.md` 9.6 and the 9.2 `RunResponse` row are done by this plan; 40-06 owns the remaining rows and the D-27 confirmation.

---
*Phase: 40-tenant-identity-run-read-scoping*
*Completed: 2026-09-29*

## Self-Check: PASSED

All 6 files listed under key-files/Files Created/Modified exist on disk and all 4 task commits (86516aad, a175d652, 8b8a9913, 03cdfd92) are present in git history.
