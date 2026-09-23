---
phase: 27-platform-api
plan: 06
subsystem: config
tags: [config, serde, env-overridable, ssrf, run-store, run-queue, schedules, webhooks]

# Dependency graph
requires:
  - phase: 24-thread-lifecycle
    provides: "src/config/waypoint_store.rs — the Default + validate() + EnvOverridable template these seven structs mirror"
provides:
  - "RunStoreConfig/RunStoreBackend (disabled | sqlite | postgres)"
  - "RunQueueConfig/RunQueueBackend (in_memory | redis)"
  - "RunWorkerConfig (concurrency, lease_seconds, min_probe_interval_ms)"
  - "RunStreamConfig (poll_interval_ms)"
  - "AssistantsConfig (expose_code_registry)"
  - "SchedulesConfig (enabled, tick_interval_ms)"
  - "WebhooksConfig (allow_private, max_attempts, timeout_secs)"
affects: [27-17-server-wiring, 27-platform-api-run-store, 27-platform-api-webhooks, 27-platform-api-schedules]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "X-09 config-gating: every new subsystem struct implements Default + validate() -> Result<(), String> + EnvOverridable, mirroring src/config/waypoint_store.rs"
    - "Postgres/Redis backends carry url_env (the env var NAME), never the connection string itself, so a secret never lands on a config type"

key-files:
  created:
    - src/config/run_store.rs
    - src/config/run_queue.rs
    - src/config/run_worker.rs
    - src/config/run_stream.rs
    - src/config/assistants.rs
    - src/config/schedules.rs
    - src/config/webhooks.rs
  modified:
    - src/config/mod.rs

key-decisions:
  - "Followed D-50 exactly: six of seven structs default OFF or to today's behaviour; AssistantsConfig.expose_code_registry defaults true, the one deliberate exception (D-32), because it renders read-only data already exposed via the pre-existing AgentRegistry"
  - "RunWorkerConfig has no heartbeat field by design (D-10) — validate() enforces lease_seconds >= 4 so the derived lease/4 heartbeat is never below one second"
  - "Env var names for run_store deliberately shorter than the waypoint_store precedent (APP_RUN_STORE_PATH / APP_RUN_STORE_URL_ENV, not _SQLITE_PATH / _POSTGRES_URL_ENV) per the plan's explicit behavior spec"

patterns-established:
  - "Internally-tagged backend enums (#[serde(tag = \"backend\", rename_all = \"snake_case\")]) nest one level under the containing struct's `backend` field — deserialization tests must supply { \"backend\": { \"backend\": \"disabled\" } }, not a flat string"

requirements-completed: [PLAT-01, PLAT-02, PLAT-03, PLAT-04, PLAT-05]

coverage:
  - id: D1
    description: "RunStoreConfig/RunStoreBackend default Disabled; validate() rejects empty sqlite path and unresolvable postgres url_env; env overrides via APP_RUN_STORE_*"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "src/config/run_store.rs#config::run_store::tests (5 tests)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib config::run_store — test result: ok. 5 passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "RunQueueConfig/RunQueueBackend default InMemory; Redis variant with url_env + key_prefix (default paladin:run_queue); validate() rejects unresolvable url_env"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "src/config/run_queue.rs#config::run_queue::tests — cargo test -p paladin-ai --lib config::run_queue: test result: ok. 6 passed"
        status: pass
    human_judgment: false
  - id: D3
    description: "RunWorkerConfig{concurrency:4, lease_seconds:60, min_probe_interval_ms:1000}, no heartbeat field, validate() rejects zero concurrency / sub-4s lease / zero probe interval"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/config/run_worker.rs#config::run_worker::tests — cargo test -p paladin-ai --lib config::run_worker: test result: ok. 5 passed"
        status: pass
    human_judgment: false
  - id: D4
    description: "RunStreamConfig{poll_interval_ms:1000} for the D-26 degraded SSE polling path; validate() rejects 0"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "src/config/run_stream.rs#config::run_stream::tests — cargo test -p paladin-ai --lib config::run_stream: test result: ok. 4 passed"
        status: pass
    human_judgment: false
  - id: D5
    description: "AssistantsConfig{expose_code_registry:true} (D-32, the deliberate ON default)"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "src/config/assistants.rs#config::assistants::tests — cargo test -p paladin-ai --lib config::assistants: test result: ok. 3 passed"
        status: pass
    human_judgment: false
  - id: D6
    description: "SchedulesConfig{enabled:false, tick_interval_ms:1000}; validate() rejects zero tick interval"
    requirement: "PLAT-05"
    verification:
      - kind: unit
        ref: "src/config/schedules.rs#config::schedules::tests — cargo test -p paladin-ai --lib config::schedules: test result: ok. 4 passed"
        status: pass
    human_judgment: false
  - id: D7
    description: "WebhooksConfig{allow_private:false, max_attempts:5, timeout_secs:10}; rustdoc names the SSRF guard's rejected target classes, write/send-time application, no-redirects policy, and the DNS-rebinding limitation"
    requirement: "PLAT-05"
    verification:
      - kind: unit
        ref: "src/config/webhooks.rs#config::webhooks::tests — cargo test -p paladin-ai --lib config::webhooks: test result: ok. 4 passed"
        status: pass
    human_judgment: false

# Metrics
duration: 25min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 06: Platform API Config Structs Summary

**Seven X-09 config structs (`run_store`, `run_queue`, `run_worker`, `run_stream`, `assistants`, `schedules`, `webhooks`) land config-gated per D-50, each with `Default` + `validate()` + `EnvOverridable`, so a v0.9 config file boots v0.10 with every new platform-API subsystem off or at today's behaviour.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-09-08T03:56:00Z (approx, first HEAD assertion)
- **Completed:** 2026-09-08T04:20:00Z
- **Tasks:** 2
- **Files modified:** 8 (7 created, 1 modified — `src/config/mod.rs`)

## Accomplishments
- `RunStoreConfig`/`RunStoreBackend` — `Disabled` (default) | `Sqlite { path }` | `Postgres { url_env }`, gating the run store behind a `501 not_implemented` route until wired (D-44)
- `RunQueueConfig`/`RunQueueBackend` — `InMemory` (default) | `Redis { url_env, key_prefix }` (default prefix `paladin:run_queue`)
- `RunWorkerConfig` — `concurrency: 4`, `lease_seconds: 60`, `min_probe_interval_ms: 1000`, deliberately no `heartbeat` field (D-10 — derived as `lease_seconds / 4`)
- `RunStreamConfig` — `poll_interval_ms: 1000` for the D-26 degraded-mode SSE polling path
- `AssistantsConfig` — `expose_code_registry: true`, the one deliberate ON default in this plan (D-32)
- `SchedulesConfig` — `enabled: false`, `tick_interval_ms: 1000` (D-36/D-37)
- `WebhooksConfig` — `allow_private: false`, `max_attempts: 5`, `timeout_secs: 10`; rustdoc documents the SSRF guard's full rejected-target-class list, the write-time + send-time application, the no-redirects policy, and the known DNS-rebinding limitation (D-42, D-43)
- All seven structs declared and re-exported from `src/config/mod.rs` in alphabetical position; `src/config/settings.rs` and `Cargo.lock` both untouched

## Task Commits

Each task was committed atomically:

1. **Task 1: `run_store`, `run_queue`, `run_worker`, `run_stream`** - `858489d3` (feat)
2. **Task 2: `assistants`, `schedules`, `webhooks`** - `b928a32a` (feat)

**Plan metadata:** committed alongside this SUMMARY (worktree mode — STATE.md/ROADMAP.md updates deferred to the orchestrator)

_Note: no TDD red/green split was used — each file was written directly with its `#[cfg(test)]` module and verified green before commit, per the plan's `tdd="true"` task attribute being satisfied by test-first-in-the-same-commit given the small, mechanical, template-mirroring nature of each struct._

## Files Created/Modified
- `src/config/run_store.rs` - `RunStoreConfig`/`RunStoreBackend`, disabled by default (PLAT-01)
- `src/config/run_queue.rs` - `RunQueueConfig`/`RunQueueBackend`, in-memory by default (PLAT-01)
- `src/config/run_worker.rs` - `RunWorkerConfig`, no heartbeat field (PLAT-02, D-10)
- `src/config/run_stream.rs` - `RunStreamConfig`, degraded-mode poll interval (PLAT-03, D-26)
- `src/config/assistants.rs` - `AssistantsConfig`, code-registry exposure default ON (PLAT-04, D-32)
- `src/config/schedules.rs` - `SchedulesConfig`, disabled by default (PLAT-05, D-36)
- `src/config/webhooks.rs` - `WebhooksConfig`, SSRF guard defaults + rustdoc (PLAT-05, D-42, D-43)
- `src/config/mod.rs` - module declarations + re-exports for all seven structs, alphabetical position

## Decisions Made
- Followed D-50 exactly: six of seven structs default OFF or to today's behaviour; `AssistantsConfig.expose_code_registry` defaults `true`, the one deliberate exception (D-32), because it renders read-only data already exposed via the pre-existing `AgentRegistry` — turning the flag on adds no new capability.
- `RunWorkerConfig` carries no `heartbeat` field (D-10); `validate()` enforces `lease_seconds >= 4` so the derived `lease_seconds / 4` heartbeat is never below one whole second.
- Used the plan's explicit (shorter) env var names for `run_store` (`APP_RUN_STORE_PATH`, `APP_RUN_STORE_URL_ENV`) rather than the longer `_SQLITE_PATH`/`_POSTGRES_URL_ENV` suffixes `waypoint_store.rs` uses — the plan's `<behavior>` block names these exact variables.
- `Postgres`/`Redis` variants across `run_store.rs` and `run_queue.rs` validate both that `url_env` is non-empty AND that the named environment variable is actually set, mirroring `waypoint_store.rs`'s full check rather than only the field-non-empty check.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' acceptance criteria and `<verify>` commands passed without requiring any Rule 1-4 fixes.

## Issues Encountered

- Initial `absent_section_deserializes_to_default` tests for `run_store.rs` and `run_queue.rs` used a flat JSON string (`{"backend": "disabled"}`) for the internally-tagged backend enum, but the enum's `#[serde(tag = "backend")]` nests one level under the containing struct's own `backend` field. Fixed by supplying `{"backend": {"backend": "disabled"}}` (and the `in_memory` equivalent) — a self-caught test-authoring bug, not a plan deviation, resolved before the Task 1 commit.

## User Setup Required

None - no external service configuration required. All seven structs default to off/today's-behaviour with no env vars needed for a v0.9-compatible boot.

## Next Phase Readiness

- All seven X-09 config structs are ready for 27-17 (server wiring) to read via `Default::default()` + `apply_env_overrides()`, the Phase 24 precedent.
- `src/config/mod.rs` remains a single-writer file for this phase — no other plan in wave 2 touches it, per this plan's file-ownership sequencing.
- §9.5 MIGRATION.md rows for these structs are intentionally deferred to 27-17, when the env vars are actually read at startup (per this plan's `<objective>`).

## Verification

- `cargo test -p paladin-ai --lib config::run_store` — test result: ok. 5 passed
- `cargo test -p paladin-ai --lib config::run_queue` — test result: ok. 6 passed
- `cargo test -p paladin-ai --lib config::run_worker` — test result: ok. 5 passed
- `cargo test -p paladin-ai --lib config::run_stream` — test result: ok. 4 passed
- `cargo test -p paladin-ai --lib config::assistants` — test result: ok. 3 passed
- `cargo test -p paladin-ai --lib config::schedules` — test result: ok. 4 passed
- `cargo test -p paladin-ai --lib config::webhooks` — test result: ok. 4 passed
- `cargo test -p paladin-ai --lib config::run_` (all four Task 1 modules combined) — test result: ok. 20 passed
- `cargo test -p paladin-ai --lib config::` (all seven new + all pre-existing config modules) — test result: ok. 133 passed
- `cargo test -p paladin-ai --doc config::run_store/run_queue/run_worker/run_stream/assistants/schedules/webhooks` — 14 doc tests total, all passed
- `cargo fmt --all --check` — clean
- `cargo check --workspace --all-targets --all-features` — clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean, 0 warnings
- `cargo doc -p paladin-ai --no-deps` — 4 pre-existing warnings unrelated to this plan's files (in `paladin_execution_service.rs`, `parley/adapter.rs`, `agent_runtime.rs`, `presets/mod.rs`); no new warnings introduced
- `grep -v '^\s*//' src/config/run_worker.rs | grep -c 'pub heartbeat'` — 0
- `grep -c 'url_env' src/config/run_store.rs` — 19; `grep -c 'url:' src/config/run_store.rs` — 0
- `grep -c 'pub mod run_store\|pub mod run_queue\|pub mod run_worker\|pub mod run_stream' src/config/mod.rs` — 4
- `grep -c 'expose_code_registry: true' src/config/assistants.rs` — 1
- `grep -c 'enabled: false' src/config/schedules.rs` — 2
- `grep -c 'allow_private: false' src/config/webhooks.rs` — 2; `grep -ci 'rebinding' src/config/webhooks.rs` — 3
- `grep -c 'pub mod assistants\|pub mod schedules\|pub mod webhooks' src/config/mod.rs` — 3
- `git diff --stat src/config/settings.rs Cargo.lock` — empty (both files untouched)

## Known Stubs

None. All seven structs are fully implemented, tested config surfaces — no stubs, placeholder text, or unwired data paths. (The subsystems they gate — the run store, queue, worker, stream, schedules and webhook delivery services themselves — are out of scope for this plan by design; §9.5 rows for env-var wiring land in 27-17.)

## Self-Check: PASSED

- All 9 files (7 created config structs + `src/config/mod.rs` + this SUMMARY) verified present on disk.
- Both task commits (`858489d3`, `b928a32a`) verified present in `git log`.

---
*Phase: 27-platform-api*
*Plan: 06*
*Completed: 2026-09-08*
