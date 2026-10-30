---
phase: 41-admission-time-allowance-enforcement
plan: 04
subsystem: treasurer
tags: [allowance, admission, agent-routes, fork, openapi, golden-gate, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 01
    provides: AllowanceAdmissionPort, AdmissionError, ApiError::allowance_exhausted, RunSubmissionService admit/confirm/abandon shape
  - phase: 41-admission-time-allowance-enforcement
    plan: 03
    provides: RunApiHandles.treasurer built by build_run_api
provides:
  - AgentApiState.treasurer, AgentApiState::with_treasurer and the private admit_principal gate
  - admission on execute, execute/stream and jobs (jobs refuses before jobs.create() and the spawn)
  - fork admission through the same private admit_and_persist lifecycle submit uses
  - 429 allowance_exhausted published on five OpenAPI operations, openapi.json regenerated
  - third sanctioned v0.9 golden exception (strip_known_v0_11_allowance_429) with a scope-pinning test
  - paladin-server passes the build_run_api Treasurer to AgentApiState
affects: [41-05, 41-06, 41-07, 41-08, 41-09]

tech-stack:
  added: []
  patterns:
    - "One admit-then-persist helper (admit_and_persist) owns the confirm/abandon rule for submit and fork"
    - "Agent routes admit then confirm back to back: there is no run row whose insert could still fail"
    - "Handler doc comments feed the OpenAPI operation description, so a frozen-baseline operation keeps its rustdoc unchanged and documents new behaviour in plain // comments plus the utoipa responses entry"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/agent_controller.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/thread_controller.rs
    - crates/paladin-web/tests/openapi_golden_v0_9.rs
    - crates/paladin-web/openapi.json
    - src/application/services/run/submission.rs
    - src/application/services/run/http_surface_tests.rs
    - src/bin/paladin-server.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - docs/src/api-reference/platform-api.md
    - CHANGELOG.md

key-decisions:
  - "Fork and submit share one private admit_and_persist helper, so the confirm/abandon rule lives in one place and submit's behaviour is unchanged"
  - "The agent-route doc comments stay byte-identical (rustdoc is the OpenAPI description, which the frozen v0.9 gate compares); the new behaviour is documented in // comments and the 429 response entry"
  - "500 is added to the OpenAPI responses of submit_run and fork_thread only, not the three agent operations, so the golden exception removes only the 429 key as the plan's truth requires"
  - "The agent-route admission binding is named _admission (not admission) until 41-07 threads warnings into the RunScope"

patterns-established:
  - "Golden-gate exceptions are applied inside restrict_paths to BOTH documents, with a restrict_paths_unstripped twin used only to prove the exception's scope"

requirements-completed: [ALLOW-02]

duration: ~60min
completed: 2026-10-03
status: complete
---

# Phase 41 Plan 04: Fork and agent-route coverage, Treasurer wired into AgentApiState, 429 published Summary

**Every principal-bearing route that starts spend -- `POST /v1/runs`, `POST /v1/threads/{id}/fork`, and the HTTP agent routes `execute`, `execute/stream` and `jobs` -- now refuses an exhausted or unverifiable allowanced caller before any work begins, `paladin-server` enforces one Treasurer over one ledger on all of them, and the `429 allowance_exhausted` is published in the OpenAPI document behind a narrowly-scoped v0.9 golden exception.**

## Performance

- **Duration:** ~60 min (warm workspace, no cold build)
- **Completed:** 2026-10-03
- **Tasks:** 3 (all `type="auto"`)
- **Files modified:** 12 (no new files)

## Accomplishments

- `AgentApiState` gained `treasurer: Option<Arc<dyn AllowanceAdmissionPort>>` and `with_treasurer`. One private `admit_principal(&state, &principal)` maps `Principal` to `PrincipalRef` to `RunAttribution` (role never reaches the Treasurer), calls `admit(.., None)` then `confirm` immediately, maps a refusal to `ApiError::allowance_exhausted`, and any other error to a logged, generic `500 allowance check failed` (no backend detail in the response).
- `execute_agent`, `execute_agent_stream` and `enqueue_job` call it after the cheap pure checks (`authorize_invoke`, `resolve_timeout`) and before the `RunScope` is built. For `execute/stream` a refusal is a plain JSON `429`, returned before either the streaming or the buffered branch. For `jobs` it is returned before `jobs.create()` and `tokio::spawn`, so a refused caller never receives a `job_id` and the job store stays empty.
- `RunSubmissionService::fork` now runs the same admit -> insert -> enqueue -> confirm/abandon lifecycle as `submit`, through one private `admit_and_persist` helper both call. `submit`'s observable behaviour and the 41-01 tests are unchanged.
- `paladin-server` clones `run_handles.treasurer` before `run_state` is moved and attaches it to `AgentApiState` (no `build_run_api` signature change, per 41-03's note).
- `429` response entries on `submit_run`, `fork_thread`, `execute_agent`, `execute_agent_stream` and `enqueue_job`; `crates/paladin-web/openapi.json` regenerated (five `"429"` entries).
- `openapi_golden_v0_9.rs`: `strip_known_v0_11_allowance_429` removes only the `"429"` key from `responses` on `post` of exactly three paths, applied inside `restrict_paths` to both documents; module docs gain the `## Phase 41 exception` section; `allowance_429_exception_is_narrowly_scoped` fails if the exception ever widens (another status, another operation, a schema).
- Registers and docs: MIGRATION.md 9.2 `AgentApiState` row (Y) with its allowlist entry, 9.6 extension, `### Allowance refusals` in platform-api.md (five routes, body, six `details` keys with an example, `Retry-After` semantics, Admin bound, difference from the per-IP `too_many_requests`), CHANGELOG.

## Task Commits

1. **Task 1: agent routes gated by one admission helper, Treasurer attached in paladin-server** - `0dcfcbe` (feat)
2. **Task 2: fork admission, Admin binding, parallel refusals** - `2570db4` (feat)
3. **Task 3: 429 in OpenAPI, golden exception, openapi.json, registers and docs** - `fc935ee` (feat)

## TDD / red evidence

- **Task 1:** the five new `agent_controller` tests were written with only the `treasurer` field and `with_treasurer` in place (no gating); all five failed (`5 failed; 47 passed`), then passed once `admit_principal` was added to the three handlers.
- **Task 2:** honest note, the shared `admit_and_persist` refactor landed before the new fork tests. The red evidence is a mutation: replacing the fork call with the un-gated `persist_and_enqueue` failed `fork_by_an_exhausted_principal_is_refused_and_touches_nothing`, `fork_admitted_confirms_after_enqueue` and `fork_failing_after_admission_calls_abandon_once` (`3 failed`), then restored. The parallel-refusal and Admin tests pass against the production gate that 41-01 built; they pin behaviour rather than drive new code.
- **Task 3:** the first golden run after the rustdoc edits failed `openapi_v0_9_paths_match_the_frozen_baseline` (see deviation 1), which is the gate working as designed.

## Verification

- `cargo test -p paladin-web` all green (270 lib tests, `openapi_golden_v0_9` 8 passed); `cargo test -p paladin-ai --features web-server --lib application::services::run` 181 passed; `--bin paladin-server` 19 passed.
- Named tests: `execute_refused_by_the_treasurer_is_429_and_never_executes`, `stream_refused_by_the_treasurer_is_429_and_never_executes` (both with and without a streamer), `jobs_refused_by_the_treasurer_is_429_without_a_job`, `agent_routes_fail_closed_when_the_allowance_check_fails`, `admitted_agent_call_passes_the_callers_attribution_and_confirms`, `fork_by_an_exhausted_principal_is_refused_and_touches_nothing`, `fork_admitted_confirms_after_enqueue`, `fork_failing_after_admission_calls_abandon_once`, `fork_without_a_principal_never_calls_the_treasurer`, `admin_principal_is_bound_by_its_allowance`, `fork_route_maps_allowance_exhausted_to_429`, `parallel_submissions_by_an_exhausted_key_are_all_refused_and_write_nothing`, `allowance_429_exception_is_narrowly_scoped`; the 41-01 `allowance_admission_tracer` still passes.
- `cargo test -p paladin-web --lib openapi_matches_committed_baseline` passes without `UPDATE_OPENAPI`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p paladin-web`: clean.
- `./scripts/check-migration-allowlist.sh` exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` reports "API surface unchanged" (4156 items), as the plan predicted -- no baseline refresh needed.
- Acceptance greps: `admit_principal(&state, &principal)` appears 3 times before `#[cfg(test)]`; in `enqueue_job` the admission call precedes `jobs.create`; `grep -c '"429"' openapi.json` prints 5; `with_treasurer` is in `src/bin/paladin-server.rs`.
- Not run: `make security` (no dependency changed in this plan); no Docker or PostgreSQL leg touched.
- Manual credential-handling review: the agent-route `429` body carries only the six D-13 keys; the `500` body is the fixed string `allowance check failed`, and a test asserts a backend detail string never reaches the client; the server-side log line carries the backend error text only (a ledger error, no key value, tenant-less), and no code path formats an API key.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Handler rustdoc changes broke the frozen v0.9 golden gate**
- **Found during:** Task 3, first run of `openapi_golden_v0_9`
- **Issue:** the plan asked for the `Returns:` lists of the three agent handlers to gain `429`/`500` lines. utoipa copies a handler's doc comment into the operation `description`, so the edit changed three `description` strings that the frozen v0.9 baseline compares byte for byte; widening the exception to cover descriptions would have violated the plan's own truth that the exception removes only `responses["429"]`.
- **Fix:** reverted the rustdoc on `execute_agent`, `execute_agent_stream` and `enqueue_job` to its original text and documented the allowance behaviour in plain `//` comments above each handler (with a note why) plus the new `429` response entry. The golden gate is unchanged in power.
- **Files modified:** `crates/paladin-web/src/agent_controller.rs`
- **Commit:** `fc935ee`

**2. [Note - scope] `500` OpenAPI response added only to `submit_run` and `fork_thread`**
- The plan said to add a `500` "where an operation does not already list one". Adding it to the three frozen agent operations would have required the golden exception to strip `500` as well, contradicting the truth that it removes only `"429"`. The fail-closed `500` on the agent routes is documented in the code comments, the platform-api.md section and MIGRATION.md 9.6 instead.

**3. [Note - naming] Agent-route admission binding is `_admission`**
- The plan asked for a local named `admission`; with `-D warnings` an unused binding fails the build, so it is `_admission` with a comment that 41-07 will thread its warnings into the `RunScope` (and drop the underscore).

**4. [Note - test shape] `fork_failing_after_admission_calls_abandon_once` drives `admit_and_persist` directly**
- `fork` itself checks for an active run before it reaches admission, so an insert-time `ThreadBusy` is only reachable as a race. The test hands the private lifecycle a second run on the busy thread to prove abandon fires exactly once and no row is persisted. `submit_insert_failure_after_admission_calls_abandon` (41-01) covers the same rule through the public `submit`.

**Total deviations:** 1 auto-fixed bug, 3 notes; no scope change.

## Authentication Gates

None.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-41-18 (bypass through `jobs`/`execute`/`execute/stream`/fork) is mitigated by the one `admit_principal` helper called in all three agent handlers, the shared submit/fork lifecycle, and executor/streamer call count 0 asserted on refusal. T-41-19 (Admin as an unbounded path) by `admin_principal_is_bound_by_its_allowance`. T-41-20 (fail-open) by `agent_routes_fail_closed_when_the_allowance_check_fails`. T-41-21 (500 body disclosure) by the fixed generic message plus a test that the backend detail is absent. T-41-22 (job ids for refused work) by the admission-before-`jobs.create()` ordering and `jobs_refused_by_the_treasurer_is_429_without_a_job`.

## Notes for later plans

- 41-07 renames `_admission` to `admission` in the three agent handlers when it threads warnings into the `RunScope`.
- The webhook delivery of operator notices is still 41-08's scope; nothing in this plan's docs claims it exists, and the D-17 option-b payload amendment (tenant_id and api_key_id names) was not touched here.
- Commit trailers use the `Claude Fable 5.1` line from this dispatch's attribution block (as 41-03 did); the orchestrator may want to normalise with 41-01 and 41-02 before push.

## Self-Check: PASSED

Commits `0dcfcbe`, `2570db4` and `fc935ee` are present in `git log`; all twelve listed files exist and were modified; `crates/paladin-web/openapi.json` contains five `"429"` entries.
