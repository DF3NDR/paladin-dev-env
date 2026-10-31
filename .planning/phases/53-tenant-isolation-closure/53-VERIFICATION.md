---
phase: 53-tenant-isolation-closure
verified: 2026-10-31T16:30:00Z
status: human_needed
score: 5/5 roadmap success criteria verified (25/25 SPEC acceptance criteria satisfied; P1-P4 verified by tests)
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
deferred: []
behavior_unverified_items: []
human_verification:
  - test: "P5 / AC-N5 (judgment tier, unverified-prohibition -- human review recommended): confirm no configuration key or no-op factory lets a configured Treasurer boot without the engine-factory path"
    expected: "assert_treasurer_enforced(treasurer.is_some(), pool.has_engine_factory()) is the only guard; no setting turns it off. The guard cannot see a no-op factory closure (has_engine_factory only reports Option::is_some)."
    why_human: "A no-op factory cannot be detected mechanically; the phase itself says code review is the control. Autonomous LLM verdict: NO escape hatch found in the tree (no settings key, the refusal text is a constant, the shipped path always passes a real factory). Recorded as NON-AUTHORITATIVE."
  - test: "Review the five advisory code-review warnings (53-REVIEW.md WR-01..WR-05) and decide accept-as-residual vs follow-up (none contradicts a SPEC acceptance criterion)"
    expected: "Each is either recorded as a residual in ADR-0063 or scheduled; in particular WR-01 (an UnknownSchemaVersion run row aborts every non-admin GET /v1/threads with 500) and WR-02 (list(limit 1) decodes the second-newest row, so a corrupt older row locks the owner out)."
    why_human: "Design/priority decisions, not goal failures; the phase goal is met without them."
---

# Phase 53: Tenant Isolation Closure Verification Report

**Phase Goal:** A principal of one tenant can no longer enumerate, read, run against or mutate another tenant's threads, runs or jobs, and a configured Treasurer can no longer be silently dropped by worker-pool wiring.
**Verified:** 2026-10-31
**Status:** human_needed (all must-haves verified in code and by passing tests; one judgment-tier prohibition and the advisory review warnings need a human decision)
**Re-verification:** No -- initial verification

Verification was done against the tree at HEAD (`dc55c3941`), not from the SUMMARYs. Scoped tests were run by the verifier.

## Goal Achievement

### Observable Truths (ROADMAP success criteria, the contract)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | Tenant B on any `/v1/threads/*` route of A's thread gets the missing-thread 404; `GET /v1/threads` omits A's threads; an OpenAPI-derived matrix fails on a route that skips the gate | VERIFIED | `thread_controller.rs`: `load_visible_thread` (l.860) is called by `get_thread_state`, `resume_thread`, `get_thread_history`, `get_thread`, `fork_thread`, `delete_thread` (gate precedes `require_admin` on resume/delete); `list_threads` (l.1179) uses `visible_thread_page` with `RunReadScope::permits_thread` for non-admin, empty page when no run store. `every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404` enumerates ops from `versioned_thread_parts(..).1` via `published_thread_id_operations`, asserts equality with `THREAD_ROUTE_TABLE`, then status/headers/body equality vs a missing thread. `cargo test -p paladin-web --lib thread_controller`: 52 passed, 0 failed. |
| SC2 | B cannot run against/fork A's thread (404), cannot read A's job, and a library caller of `RunSubmissionService::cancel` with a foreign tenant is refused at the service | VERIFIED | `submission.rs` `ensure_thread_visible` (l.277) uses `permits_thread`, returns `UnknownThread`; called by submit and fork. `cancel` (l.513) fetches the run, returns `NotFound` for a non-permitted scope before `authorize_invocation` and before `request_cancel`. `get_job_scoped` is the routed handler (`agent_openapi_router`), reads `JobStore::get_visible(job_id, agent_id, scope)`; owner recorded by `create_owned` in `enqueue_job`; no other `jobs.create()` caller. Tests: `cross_tenant_*` 3 passed; `submission` 38 passed (incl. `cancel_of_another_tenants_run_is_not_found_and_writes_nothing`); `agent_controller` 71 passed (`job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass`); `job_store` 10 passed. |
| SC3 | Corrupt/half-attributed run row and any `Serialization` error reach a client as a fixed redacted error; foreign tenant still gets the missing-run 404 | VERIFIED | `run_controller.rs`: `redact_store_error_for_scope` (l.715) + `run_repo_error_for_scope` (l.741): `Serialization` is hideable for a Tenant scope (missing-run 404), Admin and all other errors get the constant `"run store error"` 500; detail goes to `log::error!` only. `load_visible_run` (l.957) uses it, and is the entry of get/stream/webhook-deliveries/cancel (l.1021, 1176, 1247, 1339). Thread routes use the same rule (`waypoint_error_for_scope`, `thread_store_error`). Tests `serialization_on_every_run_id_route_is_hidden_for_tenant_and_redacted_for_admin`, `unattributed_valid_run_is_hidden_from_tenant_and_visible_to_admin`, `serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin` pass (`run_controller` 62 passed). |
| SC4a | With a Treasurer configured, boot refuses unless the pool was built via `with_engine_factory` (test proves it) | VERIFIED | `run_api_wiring.rs` l.1035 `assert_treasurer_enforced(treasurer.is_some(), pool.has_engine_factory())?` runs after `with_treasurer` and before `pool.spawn`; `paladin-server.rs` calls `build_run_api_with_cadence` (l.229) before `TcpListener::bind` (l.304). `RunWorkerPool::has_engine_factory` (worker.rs l.847) reads the real field. Tests `treasurer_assertion_refuses_a_configured_treasurer_without_an_engine_factory` and the shipped-wiring pass case are in the 37 passing `run_api_wiring` tests. |
| SC4b | Reserved literals rejected at config load | VERIFIED | `src/config/agents.rs` l.205-265 reject `open-access`/`unattributed` as key name/tenant/bearer tenant and `anonymous` as key name, exact match, message names the entry and never the key. `config::agents` 29 passed (`validate_rejects_reserved_literals_in_every_position`, `validate_accepts_case_variants_of_reserved_literals`). |
| SC4c | Boot logs exactly one `warn!` listing `NULL created_by` schedules, no backfill route | VERIFIED | `scan_unattributed_schedules_paged` only calls `repo.list`; `scan_report` yields at most one Warn (count, ids sorted ascending, re-create action) and at most one fixed Error line with no error text; called once at l.855 inside the run-store-enabled path; boot never blocked. Tests in `run_api_wiring` (zero/one/several/half-attributed/read-only) pass. No new route (openapi.json diff holds only description strings plus a documented 404). |
| SC5 | Patch-vs-`v0.12.0` decision recorded before Phase 58 | VERIFIED | `53-VERSION-DECISION.md` records v0.11.1 with twelve per-package lines `223 checks: 223 pass, 31 skip`, `0 failure(s)`; `.cargo/semver-checks-allowlist.toml` has 0 `[[entry]]`; MIGRATION 11.2 has no data row; `git diff 2b16763d0^..HEAD` shows no removed `pub` item and no added `#[deprecated]`. ROADMAP milestone row and Phase 53 scope note agree (v0.11.1). Behavioural changes are in MIGRATION 11.1 (M-B-23..28) and 11.6. I did not re-run `scripts/check-semver.sh` (long, disk-limited); the recorded lines are the only semver evidence, and the diff review is consistent with them. |

**Score:** 5/5 truths verified. All behaviour-dependent truths (hidden==missing, cancel-writes-nothing, scan-writes-nothing, ownership by latest run) have a passing behavioural test that I ran, so `behavior_unverified: 0`.

### Prohibitions

| ID | Prohibition | Tier | Status | Evidence |
|----|-------------|------|--------|----------|
| P1 / AC-N1 | Hidden object distinguishable from missing | test | VERIFIED | `assert_hidden_equals_missing` compares status, headers and body in the thread matrix; run-route and job tests assert byte equality; `hidden()` closures build the identical `ApiError::not_found`. |
| P2 / AC-N2 | No key value or raw row text in boot errors/logs/validation | test | VERIFIED | Validation messages interpolate only `name`/`tenant` literals; scan `Err(_)` discards error text and logs a position only; redaction constants fixed; tests assert injected message text absent from bodies. |
| P3 / AC-N3 | No tenant from header/query/body | test | VERIFIED | Tenant comes only from `Principal::read_scope()`; `tenant_hints_in_headers_or_query_change_nothing_on_thread_routes` and `job_poll_ignores_tenant_hints` pass. |
| P4 / AC-N4 | No write during the `NULL created_by` scan | test | VERIFIED | Scan calls `list` only; byte-identical-store test passes. |
| P5 / AC-N5 | No switch or no-op factory lets a configured Treasurer boot unenforced | judgment | UNCERTAIN (flagged: unverified-prohibition, human review recommended) | No config key found; constant refusal; but the guard is an `Option::is_some` tripwire and cannot see a no-op factory (IN-02, doc says so). Non-authoritative LLM verdict: no escape hatch present. |

### Required Artifacts / Key Links

| Artifact | Status | Details |
|----------|--------|---------|
| `RunReadScope::permits_thread` (`principal.rs` l.325) | VERIFIED | Used by `thread_controller.rs` (gate and list walk) and `submission.rs` (`ensure_thread_visible`); doctest and unit test present |
| `load_visible_thread` + `visible_thread_page` | VERIFIED, WIRED | All 7 handlers; Level-4: reads real `WaypointPort`/`RunRepositoryPort` |
| `redact_store_error_for_scope` / `run_repo_error_for_scope` | VERIFIED, WIRED | Shared by run and thread controllers |
| `JobStore::create_owned` / `get_visible`, `get_job_scoped` | VERIFIED, WIRED | Routed under `operation_id = "get_job"`; legacy `get_job` kept unrouted, undeprecated by design |
| `assert_treasurer_enforced`, `has_engine_factory` | VERIFIED, WIRED | Called with live values before pool spawn and before listener bind |
| `scan_report` / `report_unattributed_schedules` | VERIFIED, WIRED | Invoked once in `build_run_api_with_cadence` |
| `RESERVED_PRINCIPAL_LITERALS`, `RESERVED_KEY_NAME_LITERALS` | VERIFIED, WIRED | Consumed by `AuthConfig::validate` |
| ADR-0063, MIGRATION 11.1/11.6/11.8, WINDOWS rows 58/63 | VERIFIED | MIGRATION entries M-B-23..28 present; row 63 `fixed`, row 58 present as closed by this phase |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Thread gate and matrix | `cargo test -p paladin-web --lib thread_controller` | 52 passed, 0 failed | PASS |
| Run redaction | `cargo test -p paladin-web --lib run_controller` | 62 passed | PASS |
| Job scoping | `cargo test -p paladin-web --lib agent_controller` / `job_store` | 71 / 10 passed | PASS |
| Core thread-owner rule | `cargo test -p paladin-ai-core --lib principal` | 10 passed | PASS |
| Reserved literals | `cargo test --lib --features web-server config::agents` | 29 passed | PASS |
| Submit/fork/cancel HTTP locks | `... cross_tenant_` | 3 passed | PASS |
| Service submit/cancel | `... submission` | 38 passed | PASS |
| Boot guards | `... run_api_wiring` | 37 passed, 2 failed | PASS for Phase 53 tests; the 2 failures are `build_run_api_persists_*` (network-bound, pre-existing, TESTS-01 / Phase 55), not attributable to this phase |

### Requirements Coverage

| Requirement | Source Plan(s) | Status | Evidence |
|-------------|----------------|--------|----------|
| TENANT-03 | 53-01, 53-03, 53-04, 53-07, 53-08 | SATISFIED | Gate on 7 routes, filtered list, OpenAPI matrix |
| TENANT-04 | 53-01, 53-07, 53-08 | SATISFIED | `ensure_thread_visible` + HTTP-level `cross_tenant_submit/fork` tests |
| TENANT-05 | 53-01, 53-07, 53-08 | SATISFIED | Service-level cancel isolation + test |
| TENANT-06 | 53-02, 53-03, 53-07, 53-08 | SATISFIED | Role-aware redaction on run and thread routes |
| TENANT-07 | 53-05, 53-07, 53-08 | SATISFIED | `(agent, tenant)` job ownership |
| TENANT-08 | 53-06, 53-07, 53-08 | SATISFIED | Boot assertion before listener bind |
| TENANT-09 | 53-06, 53-07, 53-08 | SATISFIED | Reserved literals rejected |
| TENANT-10 | 53-06, 53-07, 53-08 | SATISFIED | One advisory warn, read-only scan |

All eight IDs appear in plan frontmatter and are marked Complete in REQUIREMENTS.md (traceability rows 317-324). No orphaned requirements: REQUIREMENTS.md maps no other ID to Phase 53.

### Anti-Patterns Found

None blocking. No new `TBD`/`FIXME`/`XXX` in the phase diff; no `unwrap`/`expect` in non-test library code of the changed files (occurrences found are in `#[cfg(test)]` modules); no `#[deprecated]` added.

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `thread_controller.rs` (list walk / `run_repo_error_for_scope`) | `UnknownSchemaVersion` is not hideable and aborts the whole non-admin list with 500 (WR-01) | Warning | Availability after a downgrade; not an isolation leak, but a 500 vs 404 existence signal on single-thread routes |
| `thread_controller.rs` `latest_run_for_thread` / `submission.rs` | `list(limit 1)` makes the adapter decode a second row, so a corrupt older row can lock the owner out (WR-02) | Warning | Owner lockout, fails closed |
| ownership rule | Latest run decides owner, so an Admin fork/submit or unattributed run transfers a thread (WR-03) | Warning | Matches SPEC AC-04 as written; documentation of the transfer is thin |
| `submission.rs` `ensure_thread_visible` | Submit on a guessable unused id is distinguishable from a hidden one (WR-04) | Warning | A SPEC-accepted, documented limitation (R2 target, 40-CONTEXT D-14) |
| `thread_controller.rs` gate | Hidden thread costs one more I/O than missing (WR-05) | Warning | Timing residual; status/headers/body identical |
| `http_surface_tests.rs:1669` | Stale comment says fork bodies are not byte-equal (IN-01) | Info | Cosmetic |
| `run_api_wiring.rs` | Scan has no non-advancing-cursor guard and an unbounded warn line (IN-03) | Info | Only on a misbehaving adapter or very many legacy rows |

### Human Verification Required

1. **P5 / AC-N5 judgment prohibition** -- confirm no configuration key or no-op factory lets a configured Treasurer boot unenforced. Expected: only the constant refusal path exists and the shipped wiring passes a real factory. Why human: a no-op closure is invisible to `has_engine_factory`; the SPEC assigns this to code review and `/gsd-secure-phase`. Autonomous verdict (non-authoritative): none found.
2. **Triage of 53-REVIEW.md WR-01..WR-05** -- accept as recorded residuals in ADR-0063 or schedule a follow-up (suggest WR-01 and WR-02 first). None breaks a SPEC acceptance criterion or the phase goal.

Also note for the maintainer: `cargo-llvm-cov` is not installed here, so the 82 % coverage floor is CI-only evidence and was not verified locally. The semver evidence in `53-VERSION-DECISION.md` was not re-run by this verifier.

### Gaps Summary

No goal-blocking gaps. Every ROADMAP success criterion and requirement TENANT-03..10 is implemented, wired into the shipped routers and boot path, and exercised by tests that I ran and that pass. The only red tests in the touched modules are the two known network-bound `build_run_api_persists_*` failures owned by TESTS-01 (Phase 55). The status is `human_needed` solely because the judgment-tier prohibition P5 cannot be mechanically proven and the advisory review warnings need a human accept-or-follow-up decision.

---

_Verified: 2026-10-31_
_Verifier: Claude (gsd-verifier)_
