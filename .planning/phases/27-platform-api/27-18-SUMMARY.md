---
phase: 27-platform-api
plan: 18
subsystem: api
tags: [e2e, sse, webhooks, openapi, sdk-generation, ci, coverage, public-api]

# Dependency graph
requires:
  - phase: 27-platform-api (plan 17)
    provides: "build_run_api(configs, settings, coordinator, waypoint_store, auth, code_registry) -> RunApiHandles — the single production entry point wired by paladin-server.rs"
  - phase: 27-platform-api (plans 01, 10, 12, 13, 15)
    provides: "run_router/thread_router/agent_router, RunEventStreamPort SSE framing (seven wire events), webhook delivery + sign_webhook_body, assistant publish/resolve, thread fork/history"
provides:
  - "tests/integration/e2e_platform_api_test.rs (+ [[test]] entry, required-features = [\"web-server\"]): PRD 06 acceptance-1 proven end to end over the real router — build_run_api + a hand-wired ParleyPortAdapter/GraphRegistry thread surface, mockito for both the OpenAI-shaped LLM and the webhook receiver, no Docker"
  - ".github/workflows/ci.yml sdk-clients job: generates a Python + TypeScript client from crates/paladin-web/openapi.json (pinned openapitools/openapi-generator-cli:v7.25.0, npm fallback), asserts non-empty generated trees, boots paladin-server and runs scripts/sdk-smoke/{smoke.py,smoke.ts} against it"
  - "scripts/sdk-smoke/{run.sh,smoke.py,smoke.ts,package.json,tsconfig.json,smoke-config.yml}"
  - "MIGRATION.md §9.6 rows for every /v1/runs, /v1/assistants*, /v1/schedules* route this phase added, the phase-wide openapi.json diff review, the coverage-floor evidence, and the sdk-clients job description"
  - ".project/current-exports.txt regenerated (cargo-public-api 0.52.0) and confirmed matching HEAD via ./scripts/check-api-surface.sh"
affects: [29-ship]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A test's own thread-surface ParleyPortAdapter/GraphRegistry, compiled from the SAME WarGraphDoc JSON with a fresh EngineRegistries::new(), resolves the persisted Waypoint's graph_fingerprint deterministically (content-hash, not object identity) without sharing any process state with the run engine that actually produced the Waypoint — the pattern any future full-router integration test needing POST /threads/{id}/resume to work should reuse."
    - "SSE integration-test reads must stop at the first target event via Body::into_data_stream() + a bounded per-chunk tokio::time::timeout, never drain the whole body with axum::body::to_bytes: the degraded polling path (D-26) never closes the stream until the run reaches a TERMINAL status, so draining an AwaitingInput run's stream hangs until a later resume step — a self-deadlock this plan hit and diagnosed via direct-poll bisection before fixing."
    - "A WarGraphDoc's `contains` edge condition matches the WHOLE post-merge serialized state, not a specific field: a bare \"true\"/\"false\" needle (as in the pre-existing approval_gate.json fixture) is ambiguous and can fire multiple outgoing edges in the same superstep (DispatchConflict) or fail to route at all; a field-qualified needle (`\"approved\":true`) is required, mirroring tests/integration/e2e_approval_gate_test.rs's own hand-built-graph precedent."

key-files:
  created:
    - tests/integration/e2e_platform_api_test.rs
    - scripts/sdk-smoke/run.sh
    - scripts/sdk-smoke/smoke.py
    - scripts/sdk-smoke/smoke.ts
    - scripts/sdk-smoke/package.json
    - scripts/sdk-smoke/tsconfig.json
    - scripts/sdk-smoke/smoke-config.yml
  modified:
    - Cargo.toml
    - .github/workflows/ci.yml
    - MIGRATION.md
    - .project/current-exports.txt

key-decisions:
  - "The E2E test does NOT publish crates/paladin-battalion/tests/fixtures/graph_docs/approval_gate.json verbatim. Its bare true/false contains edges (review -> writer on false, review -> review on true) do not reach Completed on approval — confirmed empirically against a real WarEngine (a throwaway probe appended to graph_doc_round_trip.rs, run, and reverted, never committed) before writing the test: submitting approved=true re-enters the SAME gate node and re-suspends indefinitely. The test defines its own, structurally identical Workflow body (writer -> review Gate -> terminal act/cancel Paladin leaves) with field-qualified `\"approved\":true`/`\"approved\":false` conditions instead."
  - "SSE reading uses Body::into_data_stream() with a bounded per-chunk timeout, stopping at the first parley event, rather than draining the whole response body with axum::body::to_bytes as the plan's own action text suggested (http_body_util::BodyExt::frame). Draining the whole body deadlocks against this test's own later resume step for an AwaitingInput run in degraded mode (see key-decisions and the tech-stack pattern above) — diagnosed via a direct-GET-poll bisection that isolated the hang to the SSE read specifically, not to run submission. No new dependency needed: axum::body::Body::into_data_stream() (axum-core 0.5) is the identical primitive http_body_util::BodyExt::frame would have wrapped."
  - "The thread-surface ParleyPortAdapter/GraphRegistry compiles workflow_body() a SECOND time (EngineRegistries::new(), matching AssistantValidator's own empty-registries construction) rather than reusing DocGraphRegistry (27-12) — DocGraphRegistry needs the exact AssistantResolver/AssistantRepositoryPort instances build_run_api constructs internally, which RunApiHandles does not expose. A fingerprint-keyed GraphRegistry needs only the same JSON body, proven content-deterministic by paladin-battalion's own fingerprint_is_deterministic_across_calls/wargraph_doc_fingerprint_two_process tests."
  - "scripts/sdk-smoke/smoke-config.yml is a new file outside 27-18-PLAN.md's declared files_modified frontmatter (Rule 3, blocking): run.sh needs SOME Settings file exposing a code-registered agent and a static API key for paladin-server to boot the smoke against, and config.test.yml (the repo's existing test config) has neither. A minimal, standalone, narrowly-scoped file colocated with run.sh is the smallest footprint that makes the declared script runnable."
  - "Regenerating .project/current-exports.txt surfaced one real, pre-existing signature change this plan did not make: ParleyPortAdapter::new's registry parameter is Arc<dyn GraphResolver> in the actual code, but the OLD committed baseline still recorded Arc<GraphRegistry> (the concrete pre-27-12 type). 27-12 changed the signature (documented in its own SUMMARY) but never regenerated this baseline. This plan's regeneration is the first to correctly reflect it — recorded here since the plan's own acceptance criterion literally reads 'additions only' and this diff has one substantive removal (the stale signature line), not a change 27-18 introduced."

requirements-completed: [PLAT-01, PLAT-02, PLAT-03, PLAT-04, PLAT-05, PLAT-06]

coverage:
  - id: D1
    description: "PRD 06 acceptance 1 passes as one integration test: publish a Workflow assistant with a Gate, submit a run, observe SSE superstep-then-parley, the run suspends AwaitingInput, a mockito webhook receives the awaiting_input payload (parleys non-empty, attempt==1, X-Paladin-Signature verified via sign_webhook_body), resume via POST /threads/{id}/resume, the run completes (attempt==2 webhook, both deliveries recorded delivered), GET /threads/{id}/history shows >=3 waypoints with one AwaitingInput, and POST /threads/{id}/fork from the terminal waypoint reaches a terminal status — all Docker-free"
    requirement: "PLAT-03"
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_platform_api_test.rs#e2e_platform_api_acceptance_1_full_lifecycle — cargo test --features web-server --test e2e_platform_api: test result: ok. 1 passed (run 4x consecutively, stable every time, ~12s each)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A sdk-clients CI job generates a Python and a TypeScript client from crates/paladin-web/openapi.json with one pinned openapi-generator-cli image (npm fallback), asserts non-empty generated trees (>=20 files each), boots paladin-server on the all-InMemory/SQLite profile, and smoke-tests list assistants -> submit run -> poll status from each client; runs on every PR/push"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "python3 -c \"import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))\" exits 0; grep -c '^  sdk-clients:$' -> 1; grep -c 'openapi-generator-cli' -> 4; 2 non-empty-tree guards verified via sed -n range extraction (the plan's own literal awk range self-matches on line 1, see Issues Encountered); test -x scripts/sdk-smoke/run.sh && python3 -m py_compile scripts/sdk-smoke/smoke.py exits 0; smoke.ts type-checks clean (tsc 5.6.3, strict) against a hand-written ambient-module stub of the expected Configuration/AssistantsApi/RunsApi shape (the real generated client could not be produced locally — no Java, no Docker; recorded honestly as WINDOWS.md id 30, not claimed as run)"
        status: pass
      - kind: manual_procedural
        ref: "CI's own sdk-clients job run on this branch (post-merge) is the FIRST real proof the generator + both live smokes pass — see Task 3's checklist below"
        status: unknown
    human_judgment: true
    rationale: "The job's own generation + live-server smoke steps require Docker/Java this devcontainer does not have; CI is the only place this can actually run. The YAML, scripts, and syntax are all locally proven; the live generator run and both smokes are not."
  - id: D3
    description: "openapi.json is diff-reviewed as a whole for the phase: every new path is additive and no pre-existing path or schema changed except the registered ResumeAcceptedResponse.run_id"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "Structural (not raw-text) diff of crates/paladin-web/openapi.json between a0c7e6e1 (the commit immediately before 27-01's first touch of the file) and HEAD: 0 removed paths, 0 removed methods on shared paths, 0 removed schemas, 0 removed properties on shared schemas; 14 added paths, 27 added schemas; openapi_matches_committed_baseline (drift guard) passes, confirming no regeneration was needed"
        status: pass
    human_judgment: false
  - id: D4
    description: "MIGRATION.md §9.6 lists every new endpoint with its auth tier, and the resume-response field"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "awk '/^## 9.6/,/^## 9.7/' MIGRATION.md | grep -c '/v1/' -> 40 (>= 22); three new tables added (runs submit/get/stream, all 7 /v1/assistants* routes, all 5 /v1/schedules* routes) with per-route status codes and D-46 auth tier called out in prose for each"
        status: pass
    human_judgment: false
  - id: D5
    description: "Workspace line coverage stays at or above the 82% floor under CI's exact invocation"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1 run to completion locally (no --fail-under-lines error emitted; 0 test failures across the whole workspace, ~9 min): lcov.info summed to 99446/110695 = 89.84% lines, 10671/12865 = 82.95% functions — well above the floor. Ran without live Redis/MinIO services (no Docker in this devcontainer), so this is a Tier-1-scoped measurement under the exact gated feature set (Tier-2 redis-queue/storage-postgres suites are compiled out entirely under these features, not self-skipped — 0 SKIP: lines), not identical to CI's service-backed run; CI's coverage job remains the canonical figure of record."
        status: pass
    human_judgment: false
  - id: D6
    description: ".project/current-exports.txt is regenerated so the api-surface CI job passes on the phase's purely additive public-API growth"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "./scripts/check-api-surface.sh .project/current-exports.txt -> \"API surface unchanged\" (exit 0); cargo-public-api 0.52.0, matching CI's pinned version"
        status: pass
    human_judgment: false

duration: ~62min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 18: E2E Acceptance-1, Generated SDK Clients Gate, and Phase Closeout Summary

**One Docker-free integration test proves PRD 06 acceptance 1 (Gate workflow, SSE, webhook, resume, history, fork) over the real Platform API router; a new `sdk-clients` CI job proves the OpenAPI spec is genuinely generatable into a working Python and TypeScript client; and MIGRATION.md §9.6 / the public-API baseline / the 82% coverage floor close out the phase's remaining bookkeeping.**

## Performance

- **Duration:** ~62 min
- **Started:** 2026-09-08T10:31:00Z (approx, worktree base `f47a6f29`)
- **Completed:** 2026-09-08T11:33:00Z
- **Tasks:** 2 (`type="auto" tdd="true"`, `type="auto"`); Task 3 (`checkpoint:human-verify`) auto-approved as a confirmation step per auto-mode
- **Files modified:** 11 (7 created, 4 modified)

## Accomplishments

- `tests/integration/e2e_platform_api_test.rs` (+ its `[[test]] required-features = ["web-server"]` entry) drives the FULL acceptance-1 lifecycle over `agent_router().merge(thread_router()).merge(run_router())`, `build_run_api` over a temp-file SQLite run store + in-memory queue + `webhooks.allow_private = true`, a hand-wired `ParleyPortAdapter`/`GraphRegistry` thread surface, and two independent mockito servers (one standing in for the OpenAI chat-completions endpoint via `OPENAI_BASE_URL`, one for the webhook receiver). Runs stably in ~12s, 4/4 consecutive passes, no Docker.
- Confirmed empirically (via a throwaway, never-committed probe against a real `WarEngine`) that `crates/paladin-battalion/tests/fixtures/graph_docs/approval_gate.json`'s own bare `"true"`/`"false"` `contains` edges do not terminate on approval — the test defines its own field-qualified equivalent instead (see key-decisions).
- `.github/workflows/ci.yml`'s new `sdk-clients` job: pinned `openapitools/openapi-generator-cli:v7.25.0` (npm-package fallback documented and wired), non-vacuous `>= 20`-file generated-tree guards for both Python and TypeScript, builds `paladin-server`, and runs `scripts/sdk-smoke/run.sh` — list assistants → submit a run for a code-registered agent → poll to a terminal status, on both clients, on every PR/push.
- `scripts/sdk-smoke/{run.sh,smoke.py,smoke.ts,package.json,tsconfig.json,smoke-config.yml}`: `run.sh` boots `paladin-server` on the all-InMemory/SQLite profile with a static test API key and a resident code-registered agent, waits for `/health`, `pip install`s + runs `smoke.py`, `npm ci` + `tsc` + `node`s `smoke.ts`, then tears the server down on any exit path. `smoke.py` type-checks (`py_compile`) locally; `smoke.ts` type-checks clean against a hand-written stub of the expected generated-client shape (the real generator could not run locally — no Java, no Docker).
- `MIGRATION.md` §9.6 gained three new route tables (the run submit/get/stream routes, all seven `/v1/assistants*` routes, all five `/v1/schedules*` routes — 21 rows this plan itself owed, per D-53) plus the phase-wide `openapi.json` diff review, the 82%-floor coverage evidence, and the `sdk-clients` job description.
- `.project/current-exports.txt` regenerated (`cargo-public-api` 0.52.0, matching CI); `./scripts/check-api-surface.sh` confirms it now matches HEAD exactly.
- Local coverage run (no live Redis/MinIO) under CI's exact command: **89.84% line coverage**, comfortably above the 82% floor, 0 test failures across the whole workspace.

## Task Commits

Each task was committed atomically:

1. **Task 1: `e2e_platform_api` — the acceptance-1 lifecycle over the real router** - `12cd5956` (test)
2. **Task 2: `sdk-clients` CI job and smoke scripts; phase-wide OpenAPI diff review; §9.6** - `837a4966` (ci)
3. **Task 3: Confirm the Tier-2 and SDK evidence on CI** - auto-approved confirmation step under auto-mode (`workflow._auto_chain_active=true`); CI evidence gathered post-merge by the orchestrator — see the checklist below.

**Plan metadata:** committed alongside this SUMMARY (worktree mode — STATE.md/ROADMAP.md updates deferred to the orchestrator).

_TDD note: Task 1 carries `tdd="true"`. The test was written, iteratively debugged against a real running server (see Issues Encountered), and verified green (4/4 consecutive runs) before its single commit — no separate RED-then-GREEN commit pair, consistent with every prior `27-platform-api` plan's documented convention for this worktree (a `type="execute"`-shaped single-commit-per-task history, not a literal red/green split, since the plan's own frontmatter has no `type: tdd`)._

## Files Created/Modified

- `tests/integration/e2e_platform_api_test.rs` — the acceptance-1 E2E test; `workflow_body()`, `UnreachableInThisTestPaladinPort`, `send`/`read_full_sse_body`/`wait_for_capture`/`poll_until_terminal`/`wait_for_delivered_deliveries` helpers.
- `Cargo.toml` — `[[test]] name = "e2e_platform_api" required-features = ["web-server"]`.
- `.github/workflows/ci.yml` — the `sdk-clients` job.
- `scripts/sdk-smoke/run.sh` — orchestrates boot → smoke → teardown.
- `scripts/sdk-smoke/smoke.py` — Python generated-client smoke (list/submit/poll).
- `scripts/sdk-smoke/smoke.ts` — TypeScript generated-client smoke (list/submit/poll).
- `scripts/sdk-smoke/package.json` / `tsconfig.json` — the `paladin-sdk` `file:` dependency on the generated tree + `typescript`/`@types/node`.
- `scripts/sdk-smoke/smoke-config.yml` — the standalone `Settings` file `run.sh` boots against (deviation, see key-decisions).
- `MIGRATION.md` — §9.6 additions (runs submit/get/stream, assistants×7, schedules×5, the phase-wide diff review, coverage evidence, `sdk-clients` description).
- `.project/current-exports.txt` — regenerated.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **The E2E test builds its own Workflow body rather than publishing `approval_gate.json` verbatim** — that fixture's bare `"true"`/`"false"` `contains` edges do not reach `Completed` on approval (empirically reproduced, never committed). A field-qualified version (`"approved":true`/`"approved":false`), matching `e2e_approval_gate_test.rs`'s own hand-built-graph convention, does.
2. **SSE reading is incremental and stops at `parley`, not a full-body drain.** `axum::body::to_bytes` on the whole SSE response deadlocks for an `AwaitingInput` run in degraded mode (D-26's poll loop never closes the stream until a TERMINAL status, which this test's own later resume step is what produces) — diagnosed by bisecting with a direct `GET /v1/runs/{id}` poll loop that proved the run itself suspends correctly in ~1s, isolating the hang to the SSE read. `Body::into_data_stream()` (no new dependency) with a bounded per-chunk timeout replaces the plan's own suggested `http_body_util::BodyExt::frame` — same primitive, already available via `axum-core`.
3. **The thread-surface registry re-compiles the workflow body rather than using `DocGraphRegistry`** — `DocGraphRegistry` needs the exact `AssistantResolver`/`AssistantRepositoryPort` `build_run_api` builds internally, which `RunApiHandles` does not expose. A fingerprint-keyed `GraphRegistry` over a second compile of the identical JSON is simpler and provably equivalent (`WarGraph::fingerprint()` is a deterministic content hash, proven by `paladin-battalion`'s own two-process fingerprint test).
4. **`scripts/sdk-smoke/smoke-config.yml`** is a new file outside the plan's declared `files_modified` (Rule 3) — `run.sh` needs some config exposing a code-registered agent + static API key, and `config.test.yml` has neither.
5. **`.project/current-exports.txt`'s regeneration surfaced a real, pre-existing signature change** (`ParleyPortAdapter::new`'s registry param, `Arc<GraphRegistry>` → `Arc<dyn GraphResolver>`) that 27-12 made but never captured in this baseline — recorded honestly rather than glossed over, since it makes the diff not literally "additions only."

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `approval_gate.json`'s own edges do not reach `Completed` on approval**
- **Found during:** Task 1, designing the workflow body to publish
- **Issue:** The fixture's `review -> review` self-loop (bare `contains "true"`) re-suspends the SAME gate node indefinitely instead of terminating — confirmed by a throwaway probe test appended to `crates/paladin-battalion/tests/graph_doc_round_trip.rs`, run against a real `WarEngine`, then reverted before any commit (never landed in the tree).
- **Fix:** The E2E test defines its own equivalent Workflow body with field-qualified `contains` conditions (`"approved":true`/`"approved":false`) and two terminal Paladin leaves (`act`/`cancel`, no outgoing edges).
- **Files modified:** `tests/integration/e2e_platform_api_test.rs` (the fixture file itself was never touched)
- **Verification:** `cargo test --features web-server --test e2e_platform_api` — 1/1 passed, 4 consecutive runs.
- **Committed in:** `12cd5956` (Task 1 commit)

**2. [Rule 1 - Bug] Draining the whole SSE response body deadlocks the test against its own later resume step**
- **Found during:** Task 1, first `cargo test` run — the whole test hung past its own 60s timeout at the SSE-open step
- **Issue:** `axum::body::to_bytes(response.into_body(), usize::MAX)` waits for the stream to naturally end; the degraded polling path (D-26) only ends on a TERMINAL run status, which `AwaitingInput` is not — the stream would only close after the test's OWN later resume call, a self-deadlock.
- **Fix:** Bisected by temporarily replacing the SSE-open call with a direct `GET /v1/runs/{id}` poll loop (confirmed the run reaches `awaiting_input` in ~1s on its own, ruling out a submission/dispatch bug), then rewrote SSE reading to use `Body::into_data_stream()` with a bounded per-chunk `tokio::time::timeout`, stopping as soon as `event: parley` is observed in the accumulated buffer.
- **Files modified:** `tests/integration/e2e_platform_api_test.rs`
- **Verification:** `cargo test --features web-server --test e2e_platform_api` — 1/1 passed in ~12s (was: timeout at 60s).
- **Committed in:** `12cd5956` (Task 1 commit)

**3. [Rule 3 - Blocking] `scripts/sdk-smoke/run.sh` needs a config file `config.test.yml` cannot provide**
- **Found during:** Task 2, designing `run.sh`'s boot sequence
- **Issue:** `paladin-server` needs a `Settings` YAML with a resident code-registered agent and a static test API key; `config.test.yml` (the repo's existing test config, outside this plan's declared `files_modified`) has neither an `agents:` section nor `http.auth.api_keys`.
- **Fix:** Added `scripts/sdk-smoke/smoke-config.yml` — a minimal, standalone, complete `Settings` file (not a partial overlay — `Settings::load_from_file` reads exactly one file, no merge) colocated with the script that needs it.
- **Files modified:** `scripts/sdk-smoke/smoke-config.yml` (new)
- **Verification:** File shape reviewed against `Settings`'s required top-level fields (`llm_type`/`llm_url`/`llm_api_key`/`server`/`sources`/`max_file_size`) and `config.example.yml`'s own `agents:`/`http.auth` precedent; not boot-tested locally (no Docker/Java for the generator half of the pipeline this config feeds — see Issues Encountered).
- **Committed in:** `837a4966` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 1 bug fixes discovered and resolved during Task 1's own verification loop before its commit, 1 Rule 3 blocking file addition necessary for Task 2's declared `run.sh` to be runnable).
**Impact on plan:** All three were necessary to reach a genuinely passing, non-deadlocking E2E test and a runnable smoke pipeline; none changed the plan's architecture or scope.

## Issues Encountered

- **The `sdk-clients` job's generator half and both live smokes could not be run locally.** This devcontainer has no Docker daemon (consistent with every prior `27-*`/`22`/`24`/`17` plan's own documented gap) and no Java runtime (the npm `@openapitools/openapi-generator-cli` fallback wraps a JAR). `smoke.py` was proven to compile (`py_compile`); `smoke.ts` was proven to type-check clean (`tsc` 5.6.3, `strict`) against a hand-written ambient-module stub matching the expected `Configuration`/`AssistantsApi`/`RunsApi` shape — but neither script has been run against a REAL generated client. Recorded plainly (never claimed as passed) and filed to `.planning/WINDOWS.md` (id 30, `kind: deviation`, `status: open`) so it stays visible until CI's own run proves or disproves the field/method-name assumptions. If the real generator's output differs (e.g. a different security-scheme wiring convention, or `snake_case` vs `camelCase` field names on the TypeScript side), the fix is localized to the two smoke scripts' field-access helpers (`_get`/`field`), which were deliberately written tolerant of both a typed-model and a plain-dict/object shape for exactly this reason.
- **The plan's own literal acceptance-criterion `awk` range for the non-empty-tree guards self-matches on line 1** (`awk '/^  sdk-clients:/,/^  [a-z-]*:$/'` — the job's own opening line `  sdk-clients:` also satisfies the END pattern, so the range never reaches the two `find … | wc -l` lines inside the job body). Verified the intent holds via a corrected extraction (`sed -n '/^  sdk-clients:$/,/^  coverage:$/p' | grep -c 'find .* -type f | wc -l'` → `2`) — mirrors 27-15-SUMMARY.md's own documented precedent for an analogous acceptance-criterion mismatch (a planner-authored `awk`/grep expression that does not account for a self-matching job name), not a defect in this plan's YAML.
- **The `.project/current-exports.txt` regeneration is not literally "additions only"** against the OLD committed file — one real signature change from 27-12 (never captured in that plan's own baseline update) surfaces as a removal now. See key-decisions #5 for the full account; `./scripts/check-api-surface.sh` (the actual CI gate) passes cleanly since the new baseline correctly matches HEAD.

## User Setup Required

None — no external service configuration required. The E2E test runs entirely against temp-file SQLite stores, the in-memory run queue, and two local mockito servers (Tier 1, no Docker). The `sdk-clients` CI job needs Docker (present on `ubuntu-latest`, absent here) or, as fallback, Java (absent here too) — this is CI-only infrastructure, not a manual setup step for a human.

## Next Phase Readiness

- PRD 06 acceptance criteria 1 and 7 (the CI-only `sdk-clients` job existing and structurally sound) are proven; 2, 4, 5, 6 have their CI/UAT evidence from prior plans (27-03's `redis-queue`/`postgres-integration`, 27-13/27-15's Tier-2 rows); 8 (coverage) is measured locally at 89.84% (>= 82% floor) with CI's own job as the canonical figure; 9 (versioning gate: §9.2–§9.6 filled, `semver`/`msrv` green) has §9.6 now complete — `semver`/`msrv`/`api-surface` are separate pre-existing CI jobs this plan did not need to touch, now backed by a correctly-synced `current-exports.txt`.
- Phase 27's own scope is complete pending the human-in-the-loop CI confirmation this plan's Task 3 named (see the checklist immediately below) — the orchestrator pushes the merged phase branch and records the real job outcomes.
- No blockers to closing the phase. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit.

### Task 3 checklist (for the orchestrator, post-merge CI confirmation)

Auto-approved under `workflow._auto_chain_active=true` per the checkpoint-pre-resolution instructions — this plan did not push anything or open a PR (a `worktree-agent-*` branch's commits are not yet merged). The orchestrator's own push of the merged `feature/phase-26` branch is what produces real CI evidence for:

- [ ] `redis-queue` — green, log contains "All run_queue::redis tests exercised the live server"
- [ ] `postgres-integration` — green, declared-vs-selected counts equal across `waypoint`, `run`, `assistant`, `run_schedule`, `webhook`
- [ ] `sdk-clients` — green, both "generated N files" lines show N ≥ 20, both Python and TypeScript smokes print a `run_id` and a terminal status
- [ ] `coverage` — green, ≥ 82% (this plan's own local measurement: 89.84%, Tier-1-scoped, no live Redis/MinIO)
- [ ] `msrv` — green, 1.88
- [ ] `semver` — green
- [ ] `api-surface` — green (this plan regenerated `.project/current-exports.txt`; `check-api-surface.sh` confirmed it matches HEAD locally)
- [ ] `test` — green, includes `e2e_platform_api` (this plan's own new test, 4/4 local passes)

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `tests/integration/e2e_platform_api_test.rs`
- FOUND: `scripts/sdk-smoke/run.sh`
- FOUND: `scripts/sdk-smoke/smoke.py`
- FOUND: `scripts/sdk-smoke/smoke.ts`
- FOUND: `scripts/sdk-smoke/package.json`
- FOUND: `scripts/sdk-smoke/tsconfig.json`
- FOUND: `scripts/sdk-smoke/smoke-config.yml`

**Commits verified to exist (git log --oneline):**
- FOUND: `12cd5956` test(27-18): add e2e_platform_api — PRD 06 acceptance-1 full lifecycle
- FOUND: `837a4966` ci(27-18): add sdk-clients CI job, smoke scripts, §9.6, API-surface baseline

**Verification commands re-run and confirmed passing:**
- `cargo test --features web-server --test e2e_platform_api` → `test result: ok. 1 passed` (re-run 4× consecutively, stable every time, ~12s each)
- `grep -c 'name = "e2e_platform_api"' Cargo.toml` → `1`
- `grep -c 'awaiting_input' tests/integration/e2e_platform_api_test.rs` → `8` (≥ 2)
- `grep -c 'fork' tests/integration/e2e_platform_api_test.rs` → `12` (≥ 1)
- `grep -c 'sign_webhook_body' tests/integration/e2e_platform_api_test.rs` → `2` (≥ 1)
- `grep -c 'docker' tests/integration/e2e_platform_api_test.rs` → `0`
- `grep -c '^  sdk-clients:$' .github/workflows/ci.yml` → `1`
- `grep -c 'openapi-generator-cli' .github/workflows/ci.yml` → `4` (≥ 2)
- `sed -n '/^  sdk-clients:$/,/^  coverage:$/p' .github/workflows/ci.yml | grep -c 'find .* -type f | wc -l'` → `2` (the plan's own literal `awk` range self-matches on line 1, see Issues Encountered)
- `test -x scripts/sdk-smoke/run.sh && python3 -m py_compile scripts/sdk-smoke/smoke.py` → exit `0`
- `grep -c 'run_id' scripts/sdk-smoke/smoke.py` → `13` (≥ 1); `grep -c 'run_id' scripts/sdk-smoke/smoke.ts` → `4` (≥ 1)
- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` → exit `0`
- `awk '/^## 9.6/,/^## 9.7/' MIGRATION.md | grep -c '/v1/'` → `40` (≥ 22)
- `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1` → exit `0`, 0 test failures; lcov.info sums to 89.84% lines (≥ 82.00%)
- `./scripts/check-api-surface.sh .project/current-exports.txt` → exit `0`, "API surface unchanged"; `git diff --stat .project/current-exports.txt` → 1645 insertions, 4 deletions (1 timestamp line, 1 summary-count line, 2 identical lines reflecting 27-12's own already-shipped `ParleyPortAdapter::new` signature change — see key-decisions #5, Issues Encountered)
- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- No unexpected file deletions in either commit (`git diff --diff-filter=D --name-only HEAD~1 HEAD` empty for both)

---
*Phase: 27-platform-api*
*Plan: 18*
*Completed: 2026-09-08*
