---
phase: 24-pause-resume-history-graceful-shutdown
plan: 11
subsystem: api
tags: [rust, axum, utoipa, hexagonal-architecture, hitl, http]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "24-10's ParleyPort/ParleyError/ResumeAccepted (paladin-ports), ParleyPortAdapter/GraphRegistry facade (src/application/services/parley/), WaypointStoreConfig; 24-09's ShutdownCoordinator process wiring in paladin-server.rs; 24-06's WaypointSummary.fork_of and Waypoint.fork_of; 24-01's Parley value types and WaypointStatus::AwaitingInput{parleys,responses}"
provides:
  - "ThreadApiState (crates/paladin-web/src/thread_controller.rs): Option<Arc<dyn WaypointPort>> + Option<Arc<dyn ParleyPort>> + auth, mirroring AgentApiState's injection-only shape -- paladin-web gains no paladin-battalion dependency (cargo tree -p paladin-web -e normal confirmed clean)"
  - "GET/POST/GET /v1/threads/{id}/{state,resume,history} -- thread_router(state) nested under API_V1_PREFIX, authenticated by the SAME require_authentication middleware AgentApiState's routes use, merged alongside agent_router (never inside it) by src/bin/paladin-server.rs"
  - "crate::agent_auth::HasAgentAuth (agent_auth.rs) -- genericizes require_authentication<S: HasAgentAuth> over any router state exposing an AgentAuthConfig, so AgentApiState and ThreadApiState share one middleware implementation instead of two copies"
  - "map_parley_error: the D-25 status table verbatim -- 404 ThreadNotFound; 409 with DISTINCT codes thread_not_awaiting_input/graph_not_registered; 400 with parley_id in details for UnknownParleyId/ParleyAlreadyAnswered/ResponseShapeInvalid/ParleyExpired; 501 when unwired, naming APP_WAYPOINT_STORE_BACKEND"
  - "crates/paladin-web/openapi.json regenerated with the three /v1/threads/* paths (553 purely-additive insertions, 0 deletions -- every pre-existing /v1/agents/* path byte-identical, both by raw diff and by openapi_pre_existing_agent_paths_are_unchanged)"
  - "src/bin/paladin-server.rs::build_thread_state -- WaypointStoreConfig-driven: Disabled -> None-valued ThreadApiState (every route 501); Sqlite/Postgres -> a real WaypointPort store, a WarEngine registered with the SAME ShutdownCoordinator run() cancels, a facade ParleyPortAdapter, and an empty GraphRegistry (NoRegisteredGraphsPaladinPort, ADR-0039); Postgres gated behind the storage-postgres feature, fails closed by name when absent"
affects: [24-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A shared axum auth middleware genericized over a `HasAgentAuth` trait bound rather than duplicated per router state -- AgentApiState and ThreadApiState both implement it and pass a turbofish (require_authentication::<S>) at their own route_layer call site, since a bare generic fn item does not let type inference flow back from from_fn_with_state's own state argument"
    - "#[serde(transparent)] newtype ids (ParleyId, WaypointId) parsed from an HTTP path/query string via serde_json::from_value(Value::String(..)) rather than a public from_uuid constructor the owning core crate does not expose -- reuses the type's own Deserialize impl instead of widening its API"
    - "utoipa::openapi::OpenApi::merge composes a second router's OpenAPI document into build_openapi's output from a throwaway, unwired state -- the same 'discarded router, kept spec' pattern versioned_agent_parts already used, extended to a second controller module"
    - "Two 409 cases sharing one HTTP status get ONE #[utoipa::path] response entry whose description names both distinct envelope `code`s (OpenAPI's per-status response map cannot hold two entries under the identical key) -- the wire behavior stays distinct (map_parley_error sets a different `code` per case), only the spec's shape is merged"

key-files:
  created:
    - crates/paladin-web/src/thread_controller.rs
  modified:
    - crates/paladin-web/src/agent_auth.rs
    - crates/paladin-web/src/agent_controller.rs
    - crates/paladin-web/src/lib.rs
    - crates/paladin-web/src/openapi.rs
    - crates/paladin-web/openapi.json
    - src/bin/paladin-server.rs
    - Cargo.toml

key-decisions:
  - "require_authentication genericized (HasAgentAuth trait, agent_auth.rs) rather than thread_controller.rs carrying its own copy of the ~15-line middleware -- the plan's own action text says thread_router reuses 'the same route_layer(...require_authentication)' as agent_router; achieving a literal shared function required this Rule 3 blocking edit to a file outside the plan's declared <files> list (agent_auth.rs), with a matching turbofish fix at agent_controller.rs's own pre-existing call site so it still compiles."
  - "ParleyId/WaypointId parsed from HTTP strings via the serde_json::from_value(Value::String(..)) round-trip on their existing #[serde(transparent)] Deserialize impls, not a new from_uuid constructor -- both types are paladin-core value types this crate cannot extend (ADR-0016), and the round-trip needs no new API surface at all."
  - "Two 409 cases (thread_not_awaiting_input, graph_not_registered) share one OpenAPI response entry naming both codes in its description, since utoipa's Responses map is keyed by HTTP status string and cannot hold two distinct 409 entries -- the runtime envelope still carries the correct, distinct `code` per case (map_parley_error), so client-observable behavior is unaffected; only the spec's own representation is necessarily merged."
  - "kind/prompt on a submitted ParleyResponseInput are NOT accepted from the HTTP client at all (only parley_id, value, responded_by) -- WarEngine::resume_with stamps both from the matching request's own fields regardless of what a caller supplies (24-01's D-07/24-04 precedent, confirmed by reading resume_with's own re-stamping code at mod.rs:1579), so the wire DTO never asks a client to know or repeat them, and the handler supplies harmless placeholders."
  - "build_thread_state's Postgres branch is gated behind the pre-existing storage-postgres feature (X-11.4: no Postgres driver in the default paladin-ai build) and fails the whole server start with a named-feature error when a Postgres backend is configured without it -- fail-closed, matching build_auth_config's own established precedent in the same file, rather than a silent fallback to Disabled."
  - "tower = { features = [\"util\"] } added to Cargo.toml's [dev-dependencies] so paladin-server.rs's own #[cfg(test)] module can drive the composed router via tower::util::oneshot, mirroring paladin-web's identical dev-dependency and test convention -- a Rule 3 blocking necessity for Task 3's own acceptance criteria (thread_router_is_merged_alongside_agent_router, thread_routes_share_the_agent_auth_middleware), not scope creep."

patterns-established: []

requirements-completed: [HITL-05]

coverage:
  - id: D1
    description: "ThreadApiState + thread_router + the three handlers, mirroring AgentApiState's shape with no paladin-battalion dependency; every route answers 501 naming the config key when unwired"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#get_thread_state_returns_status_and_parleys_when_suspended"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#get_thread_state_omits_parleys_when_not_suspended"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#get_thread_state_unknown_thread_is_404"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#thread_routes_return_501_when_no_backend_is_wired"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#thread_routes_require_authentication"
        status: pass
      - kind: other
        ref: "cargo tree -p paladin-web -e normal (no paladin-battalion edge)"
        status: pass
    human_judgment: false
  - id: D2
    description: "POST /threads/{id}/resume status mapping is exact per D-25: 202 accepted, 404 unknown thread, both 409 codes distinct, all four 400 cases carry parley_id in details"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#post_resume_returns_202_with_thread_and_state_url"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#post_resume_unknown_thread_is_404"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#post_resume_on_running_thread_is_409_thread_not_awaiting_input"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#post_resume_with_unregistered_graph_is_409_graph_not_registered"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#post_resume_bad_response_is_400_with_parley_id_in_details"
        status: pass
    human_judgment: false
  - id: D3
    description: "GET /threads/{id}/history paginates with limit (<=100) and an opaque waypoint_id cursor, no overlap across pages, null next_cursor on the last page"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#get_thread_history_paginates_with_limit_and_cursor"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/thread_controller.rs#get_thread_history_rejects_limit_above_100"
        status: pass
    human_judgment: false
  - id: D4
    description: "openapi.json is regenerated with the three thread paths (full status sets, both 409 codes documented) and every pre-existing agent path is byte-identical"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/openapi.rs#openapi_lists_the_three_thread_paths"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/openapi.rs#openapi_thread_paths_document_every_status"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/openapi.rs#openapi_matches_committed_baseline"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/openapi.rs#openapi_pre_existing_agent_paths_are_unchanged"
        status: pass
      - kind: other
        ref: "git diff --stat crates/paladin-web/openapi.json (553 insertions, 0 deletions)"
        status: pass
    human_judgment: false
  - id: D5
    description: "paladin-server composes the thread surface behind the same auth as /v1/agents/*, wires a real backend from WaypointStoreConfig, and leaves AgentApiState untouched"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "src/bin/paladin-server.rs#server_wires_no_waypoint_backend_by_default"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#server_wires_sqlite_backend_when_configured"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#thread_router_is_merged_alongside_agent_router"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#thread_routes_share_the_agent_auth_middleware"
        status: pass
      - kind: other
        ref: "git diff cc7a3caa..HEAD -- crates/paladin-web/src/agent_controller.rs (only the require_authentication::<AgentApiState> turbofish; AgentApiState's own struct body untouched)"
        status: pass
    human_judgment: false

duration: ~180min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 11: Threads over HTTP -- `ThreadApiState`, thread_router, resume/history status mapping Summary

**`GET/POST/GET /v1/threads/{id}/{state,resume,history}` land in `paladin-web` on a `ThreadApiState` struct of their own (never touching `AgentApiState`), backed by `paladin-ports`' `WaypointPort`/`ParleyPort` trait objects with zero `paladin-battalion` dependency, and `src/bin/paladin-server.rs` wires a real backend from `WaypointStoreConfig` behind the same authentication middleware `/v1/agents/*` already uses.**

## Performance

- **Duration:** ~180 min
- **Tasks:** 3 (each a TDD RED/GREEN pair)
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- `crates/paladin-web/src/thread_controller.rs` (new) lands `ThreadApiState { waypoints: Option<Arc<dyn WaypointPort>>, parley: Option<Arc<dyn ParleyPort>>, auth }`, the DTOs (`ThreadStateResponse`, `ParleyRequestDto`/`ParleyResponseDto`, `ResumeRequest`/`ParleyResponseInput`, `ResumeAcceptedResponse`, `HistoryResponse`/`WaypointSummaryDto`), and the three handlers (`get_thread_state`, `resume_thread`, `get_thread_history`), following `agent_controller.rs`'s conventions structurally (state shape, `#[utoipa::path]` annotations, `route_layer`/`nest` composition, `tower::util::oneshot` test style) throughout.
- `map_parley_error` implements D-25's status table exactly: `404` `ThreadNotFound`; `409` with **distinct** envelope codes `thread_not_awaiting_input`/`graph_not_registered`; `400` with the offending `parley_id` in `details` for `UnknownParleyId`/`ParleyAlreadyAnswered`/`ResponseShapeInvalid`/`ParleyExpired`; `501` naming `APP_WAYPOINT_STORE_BACKEND` when `ThreadApiState.parley`/`.waypoints` is `None`.
- `POST /threads/{id}/resume` converts each `ParleyResponseInput` (client supplies only `parley_id`, `value`, `responded_by`) into a core `ParleyResponse` with `kind`/`prompt` placeholders `WarEngine::resume_with` stamps over regardless of caller input (confirmed by reading its own re-stamping code, `mod.rs:1579`), calls `ParleyPort::resume_with`, and returns `202 { thread_id, state_url }` for a client to poll.
- `GET /threads/{id}/history` reads `WaypointPort::history` with `limit` (`>100` is `400`) and an opaque cursor whose content is the last page's final `waypoint_id`; `next_cursor` is populated only when the page is full (`len == limit`), `None` on the last page.
- `crate::agent_auth::HasAgentAuth` (added to `agent_auth.rs`) genericizes `require_authentication<S: HasAgentAuth>` so `ThreadApiState` reuses the SAME middleware `AgentApiState`'s routes already use, rather than a duplicated copy; the one existing call site (`agent_openapi_router`) gained a `::<AgentApiState>` turbofish since a bare generic fn item does not let axum's `from_fn_with_state` infer `S` from its own state argument -- `AgentApiState`'s own struct body is untouched (`git diff` confirms).
- `openapi.rs::build_openapi` merges `thread_controller`'s OpenAPI parts (built from a throwaway, unwired `ThreadApiState::new()` -- D-24 requires the paths listed regardless of runtime wiring) into the agent document via `utoipa::openapi::OpenApi::merge`; `crates/paladin-web/openapi.json` regenerated with `UPDATE_OPENAPI=1` -- the diff is purely additive (553 insertions, 0 deletions), and `openapi_pre_existing_agent_paths_are_unchanged` proves every pre-existing `/v1/agents/*` path is byte-identical.
- `src/bin/paladin-server.rs::build_thread_state` composes the thread surface from `WaypointStoreConfig`: `Disabled` (default) yields `None`-valued state fields; `Sqlite`/`Postgres` construct a real store, a `WarEngine` registered with the SAME `ShutdownCoordinator` `run()` cancels on SIGTERM/SIGINT, a facade `ParleyPortAdapter` over it, and an empty `GraphRegistry` -- this process registers no `WarGraph`s itself (`NoRegisteredGraphsPaladinPort`, ADR-0039), so an HTTP resume against this binary alone reports `GraphNotRegistered` until an embedder registers a graph; end-to-end HTTP resume against a real graph is proven in `paladin-web`'s own oneshot tests with an in-test registry instead. The Postgres branch is feature-gated behind the pre-existing `storage-postgres` feature (X-11.4) and fails the server start closed, naming the missing feature, when configured without it. `thread_router(thread_state)` is merged ALONGSIDE `agent_router(state)` in `run()`, never inside it.

## Task Commits

1. **Task 1: `ThreadApiState`, DTOs, `thread_router` and the three handlers** -- RED/GREEN pair:
   - `8b7ca2c5` -- `test(24-11): reproduce ThreadApiState/thread_router/handlers contract on stub handlers (red)` -- the full state/DTO/router/test surface lands, but the three handlers are stubbed to always return `500` (RED-STATE MARKER); 12 of 15 tests fail (the 3 passing ones do not depend on handler logic).
   - `251d0eba` -- `feat(24-11): land get_thread_state, resume_thread and get_thread_history (HITL-05, D-24/D-25)` -- full implementation; 15/15 tests pass.
2. **Task 2: Register the thread paths in the OpenAPI spec and regenerate `openapi.json`** -- RED/GREEN pair:
   - `c557d8e9` -- `test(24-11): reproduce thread-path OpenAPI registration on unmerged build_openapi (red)` -- 3 new tests added while `build_openapi` does not yet merge the thread paths in; 2 of 10 openapi tests fail.
   - `a0c7e6e1` -- `feat(24-11): merge thread paths into build_openapi and regenerate openapi.json (HITL-05, D-24/D-27)` -- merge landed, `openapi.json` regenerated; 10/10 tests pass.
3. **Task 3: Compose the thread surface in `paladin-server`** -- RED/GREEN pair:
   - `9fd9e7be` -- `test(24-11): reproduce thread surface composition in paladin-server on a stub build_thread_state (red)` -- `build_thread_state` stubbed to always error; 2 of 4 new tests fail (the other 2 exercise already-shipped `paladin-web` composition directly and pass vacuously, noted in the commit message per the 24-09 stub-RED precedent).
   - `79d5f11b` -- `feat(24-11): wire the thread surface into paladin-server (HITL-05, D-24/D-25/D-26)` -- full wiring landed; 11/11 tests pass.

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-web/src/thread_controller.rs` (new) -- `ThreadApiState`, DTOs, `get_thread_state`/`resume_thread`/`get_thread_history`, `thread_router`; 15 unit tests.
- `crates/paladin-web/src/agent_auth.rs` -- `HasAgentAuth` trait; `require_authentication` genericized over `S: HasAgentAuth`.
- `crates/paladin-web/src/agent_controller.rs` -- one-line turbofish fix at the existing `require_authentication` call site (`AgentApiState`'s own definition untouched).
- `crates/paladin-web/src/lib.rs` -- `pub mod thread_controller;` + `ThreadApiState`/`thread_router` re-exports.
- `crates/paladin-web/src/openapi.rs` -- `build_openapi` merges thread paths in; 3 new tests.
- `crates/paladin-web/openapi.json` -- regenerated (553 additive insertions, 0 deletions).
- `src/bin/paladin-server.rs` -- `NoRegisteredGraphsPaladinPort`, `build_thread_state`/`thread_state_over_store`/`build_postgres_thread_state`, `run()` wiring; 4 new tests.
- `Cargo.toml` -- `tower = { features = ["util"] }` added to `[dev-dependencies]` (Rule 3, test-only).

## Decisions Made

- **`require_authentication` genericized via `HasAgentAuth`, not duplicated.** The plan's own action text calls for `thread_router` to reuse "the same `route_layer(...require_authentication)`" as `agent_router`; achieving that literally required genericizing the shared function over any state exposing an `AgentAuthConfig`, landed in `agent_auth.rs` (outside Task 1's declared `<files>` list, Rule 3 blocking) with a matching turbofish fix at the one existing call site.
- **`ParleyId`/`WaypointId` parsed from HTTP strings via their own `#[serde(transparent)]` `Deserialize` impl**, round-tripped through `serde_json::from_value(Value::String(..))`, rather than a new `from_uuid` constructor -- both are `paladin-core` types this crate cannot extend (ADR-0016).
- **Both `409` cases share one OpenAPI response entry** naming both codes in its description -- `utoipa`'s `Responses` map is keyed by HTTP status string and cannot hold two entries under the identical `409` key; the runtime envelope still carries the correct, distinct `code` per case.
- **`kind`/`prompt` are not accepted from an HTTP client at all** on `ParleyResponseInput` -- `WarEngine::resume_with` stamps both from the matching request's own fields regardless of caller input (confirmed by reading its source, `mod.rs:1579`), so the wire DTO is deliberately narrower than the core `ParleyResponse` type it projects into.
- **`build_thread_state`'s Postgres branch is gated behind `storage-postgres`** (X-11.4) and fails the server start closed, by name, rather than silently falling back to `Disabled`.
- **`tower = { features = ["util"] }` added to root `Cargo.toml`'s `[dev-dependencies]`** so `paladin-server.rs`'s own test module can drive the composed router via `tower::util::oneshot` -- Task 3's own acceptance tests (`thread_router_is_merged_alongside_agent_router`, `thread_routes_share_the_agent_auth_middleware`) require it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Genericized `require_authentication` (`agent_auth.rs`, not in Task 1's `<files>` list)**
- **Found during:** Task 1, wiring `thread_router`'s `route_layer`.
- **Issue:** `require_authentication` was hard-coded to `State<AgentApiState>`; the plan's own text requires `thread_router` to layer the identical function, which cannot type-check against a second, different state type without a generic bound.
- **Fix:** Added `HasAgentAuth` trait (`agent_auth.rs`) with an `AgentApiState` impl; `require_authentication` became generic over `S: HasAgentAuth + Clone + Send + Sync + 'static`. `ThreadApiState` implements the same trait in `thread_controller.rs`. The one pre-existing call site in `agent_controller.rs` needed a `::<AgentApiState>` turbofish since bare generic fn items do not let `from_fn_with_state` infer `S` from its own argument.
- **Files modified:** `crates/paladin-web/src/agent_auth.rs`, `crates/paladin-web/src/agent_controller.rs` (one line).
- **Verification:** `thread_routes_require_authentication`/`thread_routes_share_the_agent_auth_middleware` pass; `git diff cc7a3caa..HEAD -- crates/paladin-web/src/agent_controller.rs` shows only the turbofish line -- `AgentApiState`'s own struct body is byte-identical.
- **Committed in:** `8b7ca2c5` (RED, both files land together since the trait must exist for `ThreadApiState`'s `HasAgentAuth` impl to compile).

**2. [Rule 3 - Blocking] Added `tower` (`util` feature) to root `Cargo.toml`'s `[dev-dependencies]`**
- **Found during:** Task 3, writing `thread_router_is_merged_alongside_agent_router`/`thread_routes_share_the_agent_auth_middleware`.
- **Issue:** `src/bin/paladin-server.rs`'s own test module needed `tower::util::oneshot` to drive the composed router; the root `paladin-ai` package had no direct `tower` dependency (only `paladin-web`, a different crate, already carried it).
- **Fix:** Added `tower = { version = "0.5", features = ["util"] }` to `[dev-dependencies]`, unified by the workspace resolver with `paladin-web`'s own pinned `0.5`.
- **Files modified:** `Cargo.toml`, `Cargo.lock`.
- **Verification:** `cargo test --bin paladin-server --features web-server` -- 11/11 pass.
- **Committed in:** `9fd9e7be` (RED).

---

**Total deviations:** 2 auto-fixed (both Rule 3, both blocking necessities directly required by the plan's own stated composition, not scope creep). **Impact on plan:** No scope creep -- both changes are minimal, mechanical consequences of literally reusing the shared middleware and of this codebase's own established `tower::util::oneshot` test convention.

## Issues Encountered

- **`PathItem` (utoipa) does not implement `Debug` outside the `debug` feature.** `openapi_pre_existing_agent_paths_are_unchanged`'s first draft used `assert_eq!` directly on `Option<&PathItem>`, which fails to compile (`E0277`). Fixed by comparing `serde_json::to_value(..)` of each side instead -- the same representation the committed `openapi.json` drift guard already trusts.
- **Passing a generic function item to `axum::middleware::from_fn_with_state` does not let type inference flow backward from the state argument into the function's own (unrelated) generic parameter.** Required an explicit `::<AgentApiState>`/`::<ThreadApiState>` turbofish at each of the two call sites -- documented in the Deviations section above.
- **Pre-commit hook skipped (worktree mode), per the orchestrator's `workflow.worktree_skip_hooks=true` allowance for this run** (a cold `cargo clippy --workspace --all-targets --all-features` pre-commit hook exceeds this environment's command timeout). Every commit used `--no-verify`; `cargo fmt --check`, `cargo clippy -p paladin-web --all-targets -- -D warnings`, `cargo clippy -p paladin-ai --features web-server --all-targets -- -D warnings`, and `cargo build --workspace --features web-server` were all run and verified clean before this SUMMARY was written, in addition to `cargo test --workspace --features web-server --no-fail-fast` (0 failures across every crate).

## User Setup Required

None -- no external service configuration required. `WaypointStoreConfig` defaults to `Disabled` (landed in 24-10); an operator who wants the thread surface live sets `APP_WAYPOINT_STORE_BACKEND=sqlite|postgres` plus the matching path/url-env-name variable, exactly as 24-10's own `WaypointStoreConfig` rustdoc documents.

## Next Phase Readiness

- HITL-05 is fully delivered: all three thread endpoints are reachable, authenticated identically to `/v1/agents/*`, correctly status-mapped (including both `409` codes and the unwired `501`), paginated with an opaque cursor, and listed in a regenerated `openapi.json` with zero drift on pre-existing paths. `gsd_run query requirements.mark-complete HITL-05` was run and confirmed applied to both the checkbox and traceability-table surfaces of `.planning/REQUIREMENTS.md`.
- Plan 24-12 (mdBook page, `MIGRATION.md`/`CHANGELOG.md`/traceability sweep) can document the thread surface's HTTP contract, the interim "authenticated caller, any role" posture on `POST /threads/{id}/resume` (T-24-46, accepted per D-24), and the graph-author warning against templating secrets into a Gate payload (T-24-45) referenced in this plan's own threat register.
- No blockers. The known, accepted limitation from 24-10 (the shadow-validation adapter's narrow, untested race window between a predicted-partial and an actually-complete submission) is unaffected by this plan -- this plan's own handlers never touch that internal adapter logic, only its public `ParleyPort`/`ParleyError` surface.

## Self-Check: PASSED

All 7 files (1 created, 6 modified) verified present on disk; all 6 commit hashes (`8b7ca2c5`, `251d0eba`, `c557d8e9`, `a0c7e6e1`, `9fd9e7be`, `79d5f11b`) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
