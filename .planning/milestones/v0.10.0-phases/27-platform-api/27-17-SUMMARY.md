---
phase: 27-platform-api
plan: 17
subsystem: api
tags: [wiring, config, sqlite, postgres, redis, run-engine, war-engine, run-worker-pool, openapi]

# Dependency graph
requires:
  - phase: 27-platform-api (plan 06)
    provides: "RunStoreConfig/RunQueueConfig/RunWorkerConfig/RunStreamConfig/AssistantsConfig/SchedulesConfig/WebhooksConfig — Default + apply_env_overrides() + validate()"
  - phase: 27-platform-api (plan 15)
    provides: "RunApiState builder surface (with_submission/with_repository/with_run_events/with_assistants/with_code_registry/with_expose_code_registry/with_schedules/with_webhook_deliveries/with_auth), run_router/run_openapi_router already merging assistant_controller/schedule_controller routes, ThreadApiState.with_runs/with_run_submission"
  - phase: 27-platform-api (plans 01,02,04,07,09,10,11,12,13,14)
    provides: "RunSubmissionService, RunWorkerPool (with_engine_factory/with_cancellation_probing/with_event_bus/with_webhook_deliveries), the four Sqlite/Postgres repository adapters, InMemory/Redis run queue, StoredAssistantResolver/ChainedResolver/AssistantService/AssistantValidator, ScheduleService (ScheduleAdminPort), WebhookDeliveryService/SsrfGuard, RunEventBus/RunEventStreamService"
provides:
  - "src/infrastructure/web/run_api_wiring.rs: build_run_api(configs, settings, coordinator, waypoint_store, auth, code_registry) -> Result<RunApiHandles, ..> — the single production entry point turning the seven X-09 config structs into stores/queue/worker-pool/resolvers/event-bus/RunApiState, off by default and fail-closed on a feature/config mismatch"
  - "RunApiConfigs, RunApiHandles, CodeAgentResolver (AssistantResolver over AgentRegistry, D-32), ErasedWaypointStore (Arc<dyn WaypointPort> -> Sized W for WarEngine<W>/RunWorkerPool<W>)"
  - "From<&RunWorkerConfig> for RunWorkerOptions, From<&WebhooksConfig> for WebhookDeliveryOptions, From<&SchedulesConfig> for ScheduleServiceOptions — kept in run_api_wiring.rs, never in src/config/"
  - "facade_provisioner::paladin_port_from_settings(&Settings) -> Result<Arc<dyn PaladinPort>, HostBuildError> — the run engine's real PaladinPort, the SAME default-provider resolution FacadeProvisioner/build_agent use"
  - "src/bin/paladin-server.rs: run() wires the whole Platform API — build_waypoint_store (shared by the thread surface and the run engine), build_run_api, parley_extras threaded into the thread surface's ParleyPortAdapter (PLAT-FR-06), run_router merged alongside agent_router/thread_router"
  - "config.example.yml env-var-only documentation for all seven subsystems; MIGRATION.md §9.5 seven-struct table"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ErasedWaypointStore: a newtype delegating WaypointPort over an already-erased Arc<dyn WaypointPort>, so WarEngine<W>/RunWorkerPool<W>'s Sized-generic bound is satisfiable without build_run_api's own public signature needing to be generic over W — the run engine and the thread surface's own separate WarEngine now share ONE waypoint store instance without either function knowing the other's concrete backend type"
    - "One waypoint store, two WarEngines: build_waypoint_store (async, Disabled/Sqlite/Postgres) runs ONCE in paladin-server.rs's run(); its Arc<dyn WaypointPort> result feeds BOTH build_run_api's run engine and thread_state_from_store's thread-surface engine, with parley_extras (RunRepositoryPort + RunQueuePort) threaded from the former into the latter's ParleyPortAdapter so a resume against a thread with an active run row re-enqueues durably instead of spawning in-process (D-19..D-23)"
    - "Feature-gated repository/queue backends fail closed at build_run_api call time, not at config validate() time: build_sqlite_quartet always compiles (sqlite feature always on); build_postgres_quartet/build_redis_run_queue each carry a #[cfg(feature = ...)] real implementation and a #[cfg(not(feature = ...))] twin returning a startup error naming the missing cargo feature — mirrors paladin-server.rs's own pre-existing build_postgres_thread_state precedent"

key-files:
  created:
    - src/infrastructure/web/run_api_wiring.rs
  modified:
    - src/infrastructure/web/mod.rs
    - src/infrastructure/web/facade_provisioner.rs
    - src/bin/paladin-server.rs
    - config.example.yml
    - MIGRATION.md

key-decisions:
  - "build_run_api is NOT generic over W: WaypointPort — it takes Option<Arc<dyn WaypointPort>> (already erased) and wraps it once, internally, in ErasedWaypointStore. The plan's own artifact line names the signature as `(configs, settings, coordinator, waypoint store, auth, registry)` without pinning the exact waypoint-store type; WarEngine<W>/RunWorkerPool<W> both require W: Sized, which `dyn WaypointPort` is not, so a literal generic-W signature would have forced paladin-server.rs's run() to pass a concrete SqliteWaypointStore/PostgresWaypointStore/turbofish-typed-None per branch. The erasure wrapper keeps build_run_api's signature simple and lets paladin-server.rs build the waypoint store exactly once, shared by both engines."
  - "paladin_port_from_settings lives in facade_provisioner.rs (not run_api_wiring.rs), per the plan's own <interfaces> text ('factor a fn paladin_port_from_settings(&Settings) -> Arc<dyn PaladinPort> in facade_provisioner.rs if none exists') — a file outside this plan's declared files_modified frontmatter, but explicitly named as the target by the plan's own action text. Documented here as a deviation (see below)."
  - "CodeAgentResolver takes only Arc<AgentRegistry>, not a second FacadeProvisioner/provisioner parameter the plan's prose also mentions: AgentRegistry::get(id) already returns an AgentEntry carrying a fully-built Arc<Paladin>, so re-provisioning through FacadeProvisioner would rebuild work the registry already did at agent-registration time. Runnable::Agent(entry.paladin) is the direct, correct mapping."
  - "build_thread_state (the pre-existing 4-arg async wrapper two Phase-24 tests call directly) is kept but marked #[cfg(test)] only: run() itself now calls build_waypoint_store + thread_state_from_store directly (it needs the parley_extras parameter build_thread_state's signature has no room for), so the wrapper is genuinely test-only dead code in the production build — marking it #[cfg(test)] was necessary to keep clippy -D warnings clean on the bin target, not a functional change."

requirements-completed: [PLAT-01, PLAT-02, PLAT-03, PLAT-04, PLAT-05, PLAT-06]

coverage:
  - id: D1
    description: "build_run_api turns the seven config structs into a fully wired run API when run_store is enabled, and an unwired, no-task-spawning RunApiState when it is Disabled (the default) — every new route then answers 501 not_implemented"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "src/infrastructure/web/run_api_wiring.rs#tests::defaults_wire_nothing_and_answer_501, tests::sqlite_and_in_memory_wires_three_tasks_and_every_state_field"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --features web-server --lib run_api_wiring -- test result: ok. 5 passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "A configured postgres run store or redis queue on a binary built without storage-postgres/redis-queue is a startup error naming the missing cargo feature — never a silent fallback"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "src/infrastructure/web/run_api_wiring.rs#postgres_feature_gate_tests::postgres_run_store_without_the_feature_errors_naming_it, redis_feature_gate_tests::redis_run_queue_without_the_feature_errors_naming_it (compiled under #[cfg(not(feature = ...))])"
        status: pass
    human_judgment: false
  - id: D3
    description: "run_store enabled with no waypoint store wired is a startup error naming waypoint_store.backend, never a silent InMemory/degraded fallback"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "src/infrastructure/web/run_api_wiring.rs#tests::missing_waypoint_store_errors_naming_the_config_key"
        status: pass
    human_judgment: false
  - id: D4
    description: "Code-registered agents are exposed to the run pipeline through CodeAgentResolver so POST /runs for a code-registered agent id resolves to Runnable::Agent, executed via the worker's wired PaladinPort — AgentRegistry itself is never mutated"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/infrastructure/web/run_api_wiring.rs — CodeAgentResolver implements AssistantResolver; grep -c 'CodeAgentResolver' shows the type registered in build_run_api's ChainedResolver construction"
        status: pass
    human_judgment: false
  - id: D5
    description: "paladin-server.rs's run() merges run_router alongside agent_router/thread_router before with_http_layers, threads handles.parley_extras into the SAME ParleyPortAdapter the thread surface builds (PLAT-FR-06), and threads run_repository/thread_run_submission onto ThreadApiState"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "src/bin/paladin-server.rs#tests::run_router_is_merged_alongside_agent_and_thread_routers, default_config_spawns_no_run_services, openapi_json_lists_run_paths"
        status: pass
      - kind: unit
        ref: "cargo test --bin paladin-server --features web-server -- test result: ok. 14 passed (11 pre-existing + 3 new)"
        status: pass
      - kind: manual_procedural
        ref: "manual boot of the binary against defaults (auth disabled): startup log 'run server disabled', POST /v1/runs -> 501, GET /openapi.json lists /v1/runs; a second boot with a real sqlite run store + waypoint store + in-memory queue: POST /v1/runs against an unknown assistant -> real 404 JSON, GET /v1/runs -> 200, Ctrl-C drains cleanly"
        status: pass
    human_judgment: false
  - id: D6
    description: "MIGRATION.md §9.5 lists every new config struct, key, env var and default; config.example.yml documents all seven subsystems as env-var-only"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "awk '/^## 9.5/,/^## 9.6/' MIGRATION.md | grep -c 'RunStoreConfig\\|RunQueueConfig\\|RunWorkerConfig\\|RunStreamConfig\\|AssistantsConfig\\|SchedulesConfig\\|WebhooksConfig' -> 9; grep -c 'run_store:\\|run_queue:\\|run_worker:\\|run_stream:\\|assistants:\\|schedules:\\|webhooks:' config.example.yml -> 8"
        status: pass
    human_judgment: false

duration: ~55min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 17: Platform API Server Wiring Summary

**`build_run_api` turns the seven Phase-27 config structs into a fully wired, off-by-default run engine — stores, queue, worker pool, resolvers (stored + code-registered), event bus, scheduler and webhook delivery — and `paladin-server.rs` merges it alongside the existing agent/thread routers, sharing one waypoint store between both `WarEngine`s and threading durable-resume plumbing into the thread surface's `ParleyPortAdapter`.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-09-08T09:40:00Z (approx, worktree base `516e5512`)
- **Completed:** 2026-09-08T10:22:00Z
- **Tasks:** 2 (both `type="auto"`)
- **Files modified:** 6 (1 created, 5 modified)

## Accomplishments

- `src/infrastructure/web/run_api_wiring.rs` — `build_run_api(configs, settings, coordinator, waypoint_store, auth, code_registry) -> Result<RunApiHandles, ..>`: `run_store.backend == Disabled` returns an unwired `RunApiState` with zero spawned tasks; otherwise builds the four repositories (run/assistant/schedule/webhook) sharing the run store's own backend choice, the run queue, the `ChainedResolver(StoredAssistantResolver, CodeAgentResolver)`, the run engine's real `PaladinPort` (via `paladin_port_from_settings`), a `RunWorkerPool` wired with `with_engine_factory`/`with_cancellation_probing`/`with_event_bus`/`with_webhook_deliveries`, `RunSubmissionService`, `WebhookDeliveryService`, and — when `schedules.enabled` — `ScheduleService`, then assembles the fully-populated `RunApiState`.
- `ErasedWaypointStore` — a minimal `WaypointPort`-delegating newtype over an already-erased `Arc<dyn WaypointPort>`, letting `WarEngine<W>`/`RunWorkerPool<W>`'s `Sized`-generic bound be satisfied without `build_run_api`'s own signature needing to be generic — the run engine and the thread surface's separate `WarEngine` now share exactly one waypoint store instance.
- `CodeAgentResolver` — a new `AssistantResolver` mapping an `AgentRegistry` entry directly to `Runnable::Agent(entry.paladin)` (version pinned to `1`, mirroring `CodeWorkflowResolver`'s own convention); `AgentRegistry` itself is never touched (X-03).
- `From<&RunWorkerConfig> for RunWorkerOptions`, `From<&WebhooksConfig> for WebhookDeliveryOptions`, `From<&SchedulesConfig> for ScheduleServiceOptions` — kept in `run_api_wiring.rs`, never in `src/config/`.
- `facade_provisioner::paladin_port_from_settings` — builds the run engine's real `PaladinPort` via the SAME default-provider resolution `FacadeProvisioner`/`build_agent` already use (no per-node provider hint exists on a `WarGraphDoc`-defined `Paladin` node).
- `src/bin/paladin-server.rs::run()` — loads all seven configs (`Default` + `apply_env_overrides()` + `validate()`), builds the waypoint store once (`build_waypoint_store`, shared by both engines), calls `build_run_api`, threads `handles.parley_extras` into the thread surface's `ParleyPortAdapter` via a new `thread_state_from_store` (PLAT-FR-06, D-19..D-23), threads `run_repository`/`thread_run_submission` onto `ThreadApiState`, and merges `run_router(handles.run_state)` alongside `agent_router`/`thread_router` before `with_http_layers`. `handles.tasks` (the worker/webhook/schedule background tasks, already registered with `shutdown_coordinator`) stays alive until `run()` returns.
- 5 new unit tests in `run_api_wiring.rs` (defaults spawn nothing + 501; missing waypoint store errors naming `waypoint_store.backend`; sqlite+in-memory wires 3 tasks with every `RunApiState` field `Some`; postgres/redis without their cargo feature error naming it, compiled under `#[cfg(not(feature = ...))]`) and 3 new binary tests (`run_router_is_merged_alongside_agent_and_thread_routers`, `default_config_spawns_no_run_services`, `openapi_json_lists_run_paths`) plus 1 new `facade_provisioner` test (`paladin_port_from_settings_unknown_provider_errors`) — 9 new tests total, all 11 pre-existing `paladin-server` binary tests unaffected.
- `config.example.yml` and `MIGRATION.md` §9.5 document all seven subsystems: they carry no `Settings` field (X-09) so there is no YAML key to uncomment — every one is env-var-only, mirroring `WaypointStoreConfig`'s own precedent.
- Manually verified end to end: booted the binary twice — once against defaults (auth disabled), confirming the startup log reads "run server disabled" and `POST /v1/runs` answers `501`; once with a real sqlite run store + waypoint store + in-memory queue, confirming `POST /v1/runs` against an unknown assistant returns a genuine `404` JSON error (not `501`/a crash), `GET /v1/runs` returns `200`, and `Ctrl-C` drains the worker/webhook/schedule tasks cleanly both times.

## Task Commits

Each task was committed atomically:

1. **Task 1: `build_run_api` — config → stores/queue/services/resolvers/state, fail-closed on feature mismatch** - `25889cfd` (feat)
2. **Task 2: `paladin-server` integration, `config.example.yml`, `MIGRATION.md` §9.5** - `099d3dd4` (feat)

**Plan metadata:** committed alongside this SUMMARY (worktree mode — STATE.md/ROADMAP.md updates deferred to the orchestrator).

_Note: no TDD red/green split was used — this is a `type="execute"` plan (frontmatter has no `type: tdd`); each task was written directly with its own `#[cfg(test)]` tests and verified green before commit, consistent with every prior `27-platform-api` plan's documented convention for this worktree._

## Files Created/Modified

- `src/infrastructure/web/run_api_wiring.rs` — `RunApiConfigs`, `RunApiHandles`, `ErasedWaypointStore`, `CodeAgentResolver`, the three `From<&…Config>` conversions, `build_run_api`, and the `build_sqlite_quartet`/`build_postgres_quartet`/`build_redis_run_queue` backend helpers; 5 tests.
- `src/infrastructure/web/mod.rs` — declares `pub mod run_api_wiring;`.
- `src/infrastructure/web/facade_provisioner.rs` — `EngineExecutionPort` (the production `PaladinPort` adapter over `PaladinExecutionService`, promoting `tracer_e2e.rs`'s own test-local pattern), `paladin_port_from_settings`; 1 new test; a rustdoc private-intra-doc-link fix (plain backtick instead of a `[...]` link to the private `build_agent`) so `cargo doc` stays at the pre-existing 4-warning baseline.
- `src/bin/paladin-server.rs` — `build_waypoint_store`/`build_postgres_waypoint_store` (replacing the old `build_thread_state`'s inline waypoint-store construction), `thread_state_from_store` (replacing `thread_state_over_store`, now takes `parley_extras`), `build_thread_state` kept as a `#[cfg(test)]`-only thin wrapper; `run()` rewired to build the run API and thread the run repository/submission/`parley_extras` through; new startup log line; 3 new tests.
- `config.example.yml` — a new commented section documenting all seven Platform API subsystems as env-var-only configuration (no YAML key exists for any of them).
- `MIGRATION.md` — a new §9.5 entry: the seven-struct table (file, default, key env vars), the `AssistantsConfig` on-by-default exception (D-32), and the two fail-closed cases `build_run_api` enforces.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **`build_run_api` is non-generic**, taking `Option<Arc<dyn WaypointPort>>` and wrapping it once in `ErasedWaypointStore` internally — `WarEngine<W>`/`RunWorkerPool<W>` both require `W: Sized`, which `dyn WaypointPort` is not, and the plan's own signature line does not pin an exact waypoint-store type. This keeps the public API simple and lets `paladin-server.rs` build the waypoint store exactly once, shared by the thread surface's own `WarEngine` and the run engine.
2. **`paladin_port_from_settings` lives in `facade_provisioner.rs`**, a file outside this plan's declared `files_modified` frontmatter — but the plan's own `<interfaces>` text explicitly names this file and function ("factor a `fn paladin_port_from_settings(&Settings) -> Arc<dyn PaladinPort>` in `facade_provisioner.rs` if none exists"). Treated as plan-directed, not a spontaneous scope expansion; documented as a deviation below per the harness's own convention for files outside the literal frontmatter list.
3. **`CodeAgentResolver` takes only `Arc<AgentRegistry>`**, not a second provisioner parameter — `AgentRegistry::get(id)` already returns a fully-built `Arc<Paladin>` via its `AgentEntry`, so a provisioner would only re-do work the registry already did at agent-registration time.
4. **`build_thread_state` is now `#[cfg(test)]`-only.** `run()` itself calls `build_waypoint_store` + `thread_state_from_store` directly (it needs `parley_extras`, which `build_thread_state`'s 4-argument signature has no room for), leaving the wrapper genuinely unused in the production build — required to keep `cargo clippy -D warnings` clean on the `bin` target, not a functional change; the two pre-existing tests that call it (`server_wires_no_waypoint_backend_by_default`, `server_wires_sqlite_backend_when_configured`) pass unchanged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `facade_provisioner.rs` needed `paladin_port_from_settings`, a file outside this plan's declared `files_modified` frontmatter**
- **Found during:** Task 1, implementing the run engine's `PaladinPort`
- **Issue:** The plan's own `<interfaces>` text directs: "factor a `fn paladin_port_from_settings(&Settings) -> Arc<dyn PaladinPort>` in `facade_provisioner.rs` if none exists" — but this plan's frontmatter `files_modified` lists only `run_api_wiring.rs`, `mod.rs`, `paladin-server.rs`, `config.example.yml`, `MIGRATION.md`.
- **Fix:** Added `paladin_port_from_settings` and its `EngineExecutionPort` adapter to `facade_provisioner.rs`, promoting `src/application/services/run/tracer_e2e.rs`'s own test-local `PaladinPortAdapter` pattern (its own rustdoc already says "No production adapter of this shape exists elsewhere in the tree yet") to production status.
- **Files modified:** `src/infrastructure/web/facade_provisioner.rs`
- **Verification:** `cargo test -p paladin-ai --features web-server --lib infrastructure::web::facade_provisioner` — 3 passed (including the new `paladin_port_from_settings_unknown_provider_errors` test).
- **Committed in:** `25889cfd` (Task 1 commit)

**2. [Rule 1 - Bug] A new rustdoc `private_intra_doc_links` warning on `paladin_port_from_settings`**
- **Found during:** Task 2's own `cargo doc -p paladin-ai --no-deps --features web-server` self-review (CLAUDE.md/repo_rules mandate: no new rustdoc warnings)
- **Issue:** A `[`build_agent`]`-style link in `paladin_port_from_settings`'s own doc comment pointed at the `pub(crate)` `build_agent` function, tripping `rustdoc::private_intra_doc_links` — a 5th warning not present on the pre-plan baseline of 4.
- **Fix:** Switched to a plain backtick (`` `build_agent` ``), per CLAUDE.md's own stated rustdoc convention.
- **Files modified:** `src/infrastructure/web/facade_provisioner.rs`
- **Verification:** `cargo doc -p paladin-ai --no-deps --features web-server` → back to exactly 4 pre-existing warnings.
- **Committed in:** `099d3dd4` (Task 2 commit)

**3. [Rule 1 - Bug] `build_thread_state` became dead code in the production build after `run()` was rewired**
- **Found during:** Task 2, `cargo clippy --workspace --all-targets --all-features -- -D warnings` after rewiring `run()` to call `build_waypoint_store`/`thread_state_from_store` directly
- **Issue:** `build_thread_state` (the pre-existing Phase-24 helper) was left uncalled by production code once `run()` needed the `parley_extras` parameter it has no room for; `-D warnings` flagged it as dead code on the `bin` target.
- **Fix:** Marked `build_thread_state` `#[cfg(test)]` — it is still exercised by the two pre-existing tests that call it directly, and clippy's separate `bin` (non-test) target check no longer sees it as unused.
- **Files modified:** `src/bin/paladin-server.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; both affected tests (`server_wires_no_waypoint_backend_by_default`, `server_wires_sqlite_backend_when_configured`) still pass unchanged.
- **Committed in:** `099d3dd4` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 Rule 3 blocking cross-file fix explicitly directed by the plan's own interfaces text, 2 Rule 1 bug fixes caught during this plan's own self-review loops before their respective commits).
**Impact on plan:** All three were necessary to reach a compiling, fully green, zero-new-warning state; none changed this plan's architecture or scope.

## Issues Encountered

- **`APP_WAYPOINT_STORE_PATH` is not the real env var name** during manual verification — the actual name (established by Phase 24) is `APP_WAYPOINT_STORE_SQLITE_PATH`. Not a code defect (this plan's own new env vars, e.g. `APP_RUN_STORE_PATH`, are correctly named per 27-06's own `<behavior>` spec — the shorter form was deliberate there, per that plan's own SUMMARY); this was solely a manual-verification-command mistake, corrected mid-session before the successful live-wiring smoke test.

## Known Stubs

None. This plan is production wiring only — every artifact it builds (`build_run_api`, `paladin_port_from_settings`, `ErasedWaypointStore`, `CodeAgentResolver`) is fully implemented, tested, and manually verified end to end against a real sqlite-backed run engine. No placeholder, hardcoded empty value, or unwired data path was introduced.

## User Setup Required

None — no external service configuration required. Every automated test in this plan runs against `InMemory`/temp-file-`Sqlite` adapters (Tier 1, no Docker); the manual verification runs used a dummy `OPENAI_API_KEY` value (construction-time only, no network call made) and local temp-file sqlite stores.

## Next Phase Readiness

- The Platform API is now genuinely live end to end in `paladin-server`: every route landed across plans 27-01 through 27-16 is reachable, config-gated, and fail-closed exactly as PLAT-01..06 specify. Phase 27's own scope is complete.
- `docs/src/api-reference/platform-api.md` (27-16) already documents the config surface this plan wires at runtime; the k8s worker-replica manifest (27-16) already targets the exact env vars this plan reads.
- No blockers. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo build --bin paladin-server --features web-server` and `cargo build --bin paladin-server --features web-server,storage-postgres,redis-queue` both succeed; `cargo doc -p paladin-ai --no-deps --features web-server` introduces no new warnings beyond the pre-existing 4.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `src/infrastructure/web/run_api_wiring.rs`
- FOUND: `src/infrastructure/web/mod.rs`
- FOUND: `src/infrastructure/web/facade_provisioner.rs`
- FOUND: `src/bin/paladin-server.rs`
- FOUND: `config.example.yml`
- FOUND: `MIGRATION.md`

**Commits verified to exist (git log --oneline):**
- FOUND: `25889cfd` feat(27-17): add build_run_api — config-driven Platform API wiring
- FOUND: `099d3dd4` feat(27-17): wire the Platform API into paladin-server, config docs, MIGRATION §9.5

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai --features web-server --lib run_api_wiring` → `test result: ok. 5 passed`
- `cargo test -p paladin-ai --features web-server --lib infrastructure::web::facade_provisioner` → `test result: ok. 3 passed`
- `cargo test --bin paladin-server --features web-server` → `test result: ok. 14 passed` (11 pre-existing + 3 new)
- `grep -c 'pub async fn build_run_api' src/infrastructure/web/run_api_wiring.rs` → `1`
- `grep -c 'cfg(feature = "storage-postgres")'`/`'cfg(feature = "redis-queue")' src/infrastructure/web/run_api_wiring.rs` → `1` each
- `grep -c 'NoRegisteredGraphsPaladinPort' src/infrastructure/web/run_api_wiring.rs` → `0`
- `grep -v '^\s*//' src/config/*.rs | grep -c 'RunWorkerOptions\|WebhookDeliveryOptions'` → `0`
- `grep -c 'build_run_api' src/bin/paladin-server.rs` → `12` (≥1); `grep -c 'run_router(' src/bin/paladin-server.rs` → `2` (≥1)
- `grep -c 'run_store:\|run_queue:\|run_worker:\|run_stream:\|assistants:\|schedules:\|webhooks:' config.example.yml` → `8` (≥7)
- `awk '/^## 9.5/,/^## 9.6/' MIGRATION.md | grep -c 'RunStoreConfig\|RunQueueConfig\|RunWorkerConfig\|RunStreamConfig\|AssistantsConfig\|SchedulesConfig\|WebhooksConfig'` → `9` (≥7)
- `cargo build --bin paladin-server --features web-server,storage-postgres,redis-queue` and `cargo build --bin paladin-server --features web-server` → both exit `0`
- `cargo fmt --all --check` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- `cargo check --workspace --all-targets --all-features` → exit 0
- `cargo doc -p paladin-ai --no-deps --features web-server` → 4 pre-existing warnings, none new
- No unexpected file deletions in either commit (`git diff --diff-filter=D --name-only HEAD~1 HEAD` empty for both)
- Manual boot verification (defaults, auth disabled): startup log contains "run server disabled"; `POST /v1/runs` → `501`; `GET /openapi.json` lists `/v1/runs`; `Ctrl-C` → clean shutdown
- Manual boot verification (sqlite run store + waypoint store + in-memory queue, dummy `OPENAI_API_KEY`): startup log contains "run server is live"; `POST /v1/runs` against an unknown assistant → `404` JSON `{"error":{"code":"not_found",...}}`; `GET /v1/runs` → `200`; `Ctrl-C` → clean shutdown, worker/webhook/schedule tasks drained

---
*Phase: 27-platform-api*
*Plan: 17*
*Completed: 2026-09-08*
