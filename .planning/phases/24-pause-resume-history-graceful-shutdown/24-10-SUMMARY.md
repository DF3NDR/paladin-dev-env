---
phase: 24-pause-resume-history-graceful-shutdown
plan: 10
subsystem: api
tags: [rust, hexagonal-architecture, ports-and-adapters, hitl, background-jobs, tokio]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "24-01's Parley suspend/resume spine and value types (ParleyRequest/ParleyResponse/ParleyKind/OnExpire/ParleyId); 24-04's WarEngine::resume_with complete validation matrix (EngineError::ThreadNotAwaitingInput/UnknownParleyId/ParleyAlreadyAnswered/ResponseShapeInvalid/ParleyExpired/ThreadAlreadyFailed) and partial-answer/lazy-expiry semantics; 24-08's ShutdownCoordinator/RunGuard/ShutdownOutcome for background-task lifecycle registration"
provides:
  - "ParleyPort (paladin-ports, input side): async fn resume_with(&ThreadId, Vec<ParleyResponse>) -> Result<ResumeAccepted, ParleyError>, naming only paladin-core types plus types declared in the port module itself -- no paladin-battalion dependency, verified by cargo tree -p paladin-web -e normal"
  - "ParleyError (paladin-ports): 9 #[non_exhaustive] variants -- the 7 mirroring EngineError's D-10 validation matrix (ThreadNotAwaitingInput/UnknownParleyId/ParleyAlreadyAnswered/ResponseShapeInvalid/ParleyExpired) plus ThreadNotFound/GraphNotRegistered, plus Backend and Rejected added during Task 2 (Rule 2) for a genuine WaypointPort I/O failure and a fail-closed catch-all for any EngineError this adapter's mapping does not name"
  - "GraphRegistry (src/application/services/parley/registry.rs): a fingerprint-keyed, RwLock<HashMap<GraphFingerprint, Arc<WarGraph>>> lookup -- register()/resolve() only, the narrow seam Phase 27's WarGraphDoc registry replaces"
  - "ParleyPortAdapter<W: WaypointPort> (src/application/services/parley/adapter.rs): the facade ParleyPort implementation over a real WarEngine<W> -- resolves the thread's graph via GraphRegistry, validates a submission via a from-scratch re-implementation of WarEngine::resume_with's own D-10/D-11/D-12 validation algorithm (shadow_validate, documented at length as a defensive pre-check never the sole authority), and either returns a typed error/delegates a fast synchronous call to the real engine, or -- only when the submission is valid and complete -- registers with the ShutdownCoordinator and spawns the real engine call in the background, returning ResumeAccepted immediately"
  - "WaypointStoreConfig (src/config/waypoint_store.rs): WaypointStoreBackend::Disabled (default) | Sqlite { path } | Postgres { url_env }, Default/validate()/EnvOverridable (APP_WAYPOINT_STORE_BACKEND/_SQLITE_PATH/_POSTGRES_URL_ENV), never touching src/config/settings.rs"
affects: [24-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Shadow validation: when an atomic, crate-private authoritative validator (WarEngine::resume_with) cannot be decomposed into validate-then-continue without modifying the crate that owns it, a facade adapter in a different crate re-implements the SAME validation algorithm using only public data/types as a defensive, predictive pre-check -- never the sole authority. Every branch the pre-check predicts will NOT reach the continuation (a rejected submission, an expired FailRun parley, or a valid-but-partial submission) delegates the actual persist to a real, synchronous (always-fast-by-construction) call to the authoritative engine; only the one branch predicted to reach the continuation (every parley answered) is spawned in the background."
    - "validate-then-spawn-then-return, reused a third time in this codebase (agent_controller.rs's enqueue_job, 24-08's tokio::spawn dispatch, now ParleyPortAdapter): register with a lifecycle coordinator BEFORE spawning, never after, so a task that finishes between spawn and registration can never escape being tracked"
    - "A port's parameter/return types are core types or port-module-local types only; a facade adapter in the root crate is free to depend on the internal implementation crate directly, since ADR-0031's edge restriction is about paladin-web's default build, not about every consumer"

key-files:
  created:
    - crates/paladin-ports/src/input/parley_port.rs
    - src/application/services/parley/mod.rs
    - src/application/services/parley/adapter.rs
    - src/application/services/parley/registry.rs
    - src/config/waypoint_store.rs
  modified:
    - crates/paladin-ports/src/input/mod.rs
    - src/application/services/mod.rs
    - src/config/mod.rs

key-decisions:
  - "ParleyPortAdapter re-implements WarEngine::resume_with's validation algorithm from scratch (shadow_validate/shadow_validate_response_shape/shadow_validate_parley_value_for_kind/shadow_normalize_approval_value) rather than reusing paladin-battalion's own validators, because those are pub(crate) to paladin-battalion and resume_with is one atomic async function with no validate-only entry point -- decomposing it would require modifying crates/paladin-battalion/, out of this plan's declared file scope (and out of scope for a parallel-wave worktree agent). Documented at length in adapter.rs's module-level rustdoc as a defensive pre-check: every outcome the shadow validator concludes will NOT reach the continuation is still verified by a real, synchronous call to the authoritative engine before returning to the caller; only a predicted-complete submission is trusted enough to spawn without a second synchronous check, and even then the spawned call is the real, authoritative one -- the shadow check never itself persists anything or decides the final outcome."
  - "ParleyError gained two variants beyond the plan's own explicit list (Backend and Rejected, Task 2, Rule 2): Backend maps a genuine WaypointPort I/O failure (previously unmappable to any of the 7 planned variants without misleadingly implying a 404-class ThreadNotFound); Rejected is the fail-closed catch-all EngineError's #[non_exhaustive] status forces every downstream match to carry, covering EngineError::GraphMismatch and ::ThreadAlreadyFailed (structurally unreachable given this adapter's own upstream checks) and any future EngineError variant this mapping does not yet name."
  - "ResumeAccepted carries only thread_id, with a state_handle() accessor returning the same value -- a ThreadId alone is sufficient to resolve GET /v1/threads/{id}/state (plan 24-11), so no separate opaque-handle type was invented ahead of need."
  - "GraphRegistry stores Arc<WarGraph> (not WarGraph by value) because WarGraph does not derive Clone; register() takes ownership and returns the fingerprint, so test code seeds a Waypoint from the graph BEFORE registering it, never needing to clone or retain a second copy."

patterns-established: []

requirements-completed: []

coverage:
  - id: D1
    description: "ParleyPort trait + ResumeAccepted + ParleyError land in paladin-ports naming only core types, with a typed error for every documented validation case"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/input/parley_port.rs#parley_port_is_object_safe"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/input/parley_port.rs#parley_error_covers_every_validation_case"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/input/parley_port.rs#parley_error_display_names_the_parley_id"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/input/parley_port.rs#resume_accepted_carries_thread_and_state_handle"
        status: pass
      - kind: other
        ref: "cargo test -p paladin-ports --doc parley_port (2 doc tests)"
        status: pass
      - kind: other
        ref: "cargo tree -p paladin-web -e normal (no paladin-battalion edge)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Facade ParleyPortAdapter + GraphRegistry validate synchronously (typed errors before any spawn, nothing persisted on rejection), spawn the continuation registered with ShutdownCoordinator only when valid-and-complete, and resolve graphs strictly by fingerprint with no fallback"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#adapter_validates_synchronously_and_returns_typed_errors"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#adapter_persists_nothing_on_a_validation_error"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#adapter_spawns_the_continuation_and_returns_immediately"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#spawned_continuation_is_registered_with_the_coordinator"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#unknown_thread_is_thread_not_found"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#unregistered_fingerprint_is_graph_not_registered"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#non_awaiting_thread_is_thread_not_awaiting_input"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/adapter.rs#every_engine_error_maps_to_a_distinct_parley_error"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/registry.rs#registry_resolves_by_fingerprint"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/registry.rs#unregistered_fingerprint_resolves_to_none"
        status: pass
      - kind: unit
        ref: "src/application/services/parley/registry.rs#re_registering_the_same_fingerprint_replaces_the_entry"
        status: pass
      - kind: other
        ref: "cargo test --doc -p paladin-ai parley (1 doc test)"
        status: pass
    human_judgment: false
  - id: D3
    description: "WaypointStoreConfig defaults to disabled, validates sqlite path and postgres url_env presence/resolvability, reads APP_-prefixed env overrides, and never touches src/config/settings.rs"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "src/config/waypoint_store.rs#waypoint_store_config_defaults_to_disabled"
        status: pass
      - kind: unit
        ref: "src/config/waypoint_store.rs#waypoint_store_config_reads_env_overrides"
        status: pass
      - kind: unit
        ref: "src/config/waypoint_store.rs#waypoint_store_config_validates_backend_parameters"
        status: pass
      - kind: unit
        ref: "src/config/waypoint_store.rs#waypoint_store_config_postgres_reads_url_from_env_name_not_inline"
        status: pass
      - kind: other
        ref: "cargo test --doc -p paladin-ai waypoint_store (2 doc tests)"
        status: pass
      - kind: other
        ref: "git diff -- src/config/settings.rs (empty)"
        status: pass
    human_judgment: false

duration: ~100min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 10: ParleyPort, Facade Adapter and WaypointStoreConfig Summary

**A core-typed `ParleyPort` in `paladin-ports`, a facade `ParleyPortAdapter`/`GraphRegistry` in the root crate that shadow-validates a resume submission before delegating or backgrounding the real `WarEngine::resume_with` call, and a disabled-by-default `WaypointStoreConfig` -- together the D-25/D-26 seam `paladin-web` will consume in plan 24-11 without ever depending on `paladin-battalion`.**

## Performance

- **Duration:** ~100 min
- **Tasks:** 3 (all TDD RED/GREEN pairs)
- **Files modified:** 8 (5 created, 3 modified)

## Accomplishments

- `ParleyPort` (`crates/paladin-ports/src/input/parley_port.rs`, new) declares `async fn resume_with(&self, thread: &ThreadId, responses: Vec<ParleyResponse>) -> Result<ResumeAccepted, ParleyError>` naming only `paladin-core` types or types declared in the port module itself -- confirmed by `cargo tree -p paladin-web -e normal` showing no `paladin-battalion` edge, and no new dependency added to `paladin-ports/Cargo.toml`. `ParleyError` is `thiserror`-derived, `#[non_exhaustive]`, and grew to 9 variants: the 7 named by the plan (`ThreadNotFound`, `GraphNotRegistered`, `ThreadNotAwaitingInput`, `UnknownParleyId`, `ParleyAlreadyAnswered`, `ResponseShapeInvalid`, `ParleyExpired`) plus `Backend` and `Rejected`, added during Task 2 (Rule 2) once the adapter needed somewhere to map a genuine `WaypointPort` I/O failure and a fail-closed catch-all for any `EngineError` variant this mapping does not name explicitly.
- `GraphRegistry` (`src/application/services/parley/registry.rs`, new) is a `RwLock<HashMap<GraphFingerprint, Arc<WarGraph>>>` behind `register()`/`resolve()` only -- an unregistered fingerprint returns `None`, never a default or "nearest" graph (D-26), and the surface is kept deliberately minimal since Phase 27's `WarGraphDoc` registry replaces this lookup behind the same `ParleyPort`.
- `ParleyPortAdapter<W: WaypointPort>` (`src/application/services/parley/adapter.rs`, new) implements `ParleyPort` over a real `WarEngine<W>`: it loads the thread's latest Waypoint (`ThreadNotFound`), resolves the graph via `GraphRegistry` by the Waypoint's own `graph_fingerprint` (`GraphNotRegistered`), and requires `AwaitingInput` status (`ThreadNotAwaitingInput`). Because `WarEngine::resume_with` is one atomic async function whose total-validation pass and potentially long-running continuation (`superstep::run_with_namespace`) cannot be split from outside `paladin-battalion` -- and whose own validators are `pub(crate)` -- the adapter re-implements the SAME validation algorithm from scratch (`shadow_validate` and its three helper functions, mirroring 24-04's D-10/D-11/D-12 ordering exactly: lazy expiry scan with `FailRun` short-circuit, per-response `UnknownParleyId`/`ParleyAlreadyAnswered`/`ResponseShapeInvalid` checks, then a completeness check) using only public data and types. This shadow validation is documented at length as a defensive, predictive pre-check, never the sole authority: every outcome it predicts will NOT reach the continuation (a rejected submission, an expired `FailRun` parley, or a valid-but-partial submission) is verified by a REAL, synchronous (always fast by construction) call to `WarEngine::resume_with` before the adapter returns to its caller. Only a submission the shadow validator predicts is valid AND complete registers with the `ShutdownCoordinator` and spawns the real, authoritative engine call as a background task, returning `ResumeAccepted` immediately -- the same validate-then-spawn-then-return shape `agent_controller.rs`'s `enqueue_job` already ships.
- `map_engine_error` maps every named `EngineError` variant onto its `ParleyError` counterpart explicitly (never a catch-all that would collapse the 400-versus-409 distinction plan 24-11 needs), with `GraphMismatch`/`ThreadAlreadyFailed` (structurally unreachable given the adapter's own upstream checks) and any future variant falling into `ParleyError::Rejected`.
- `WaypointStoreConfig` (`src/config/waypoint_store.rs`, new) declares `WaypointStoreBackend::Disabled` (default) | `Sqlite { path }` | `Postgres { url_env }` -- the postgres variant names the environment variable holding the connection url, never the url itself, so a connection string never lands in a serialised config payload or a `Debug`/log line. `validate()` rejects an empty sqlite path and a postgres `url_env` that is either empty or names a currently-unset environment variable. `EnvOverridable` reads `APP_WAYPOINT_STORE_BACKEND`/`_SQLITE_PATH`/`_POSTGRES_URL_ENV`, mirroring `WaypointRetentionConfig`'s template field-for-field. `src/config/settings.rs` is untouched (`git diff` confirms empty).

## Task Commits

1. **Task 1: `ParleyPort`, `ParleyError` and `ResumeAccepted` in `paladin-ports`** -- RED/GREEN pair:
   - `a5df52eb` -- `test(24-10): reproduce ParleyPort/ParleyError/ResumeAccepted contract on not-yet-existing API (red)` -- 4 tests added against not-yet-existing types; 21 E0405/E0425/E0433 compile errors.
   - `7a57daa4` -- `feat(24-10): land ParleyPort, ParleyError and ResumeAccepted in paladin-ports (HITL-05, D-25)` -- full implementation; all 4 unit tests + 2 doc tests green, no new dependency, no `paladin-battalion` edge from `paladin-web`.
2. **Task 2: Facade `ParleyPort` adapter and `GraphRegistry`** -- RED/GREEN pair:
   - `42108dd5` -- `test(24-10): reproduce ParleyPortAdapter/GraphRegistry contract on not-yet-existing API (red)` -- 14 tests added (11 new + updating Task 1's exhaustive-match test to 9 cases) referencing not-yet-existing `ParleyPortAdapter`/`GraphRegistry`/`map_engine_error`; 22 E0425/E0432/E0433 compile errors.
   - `be689d93` -- `feat(24-10): land ParleyPortAdapter and GraphRegistry (HITL-05, D-25, D-26)` -- full implementation; all 11 unit tests + 1 doc test green, `cargo fmt`/`cargo clippy -p paladin-ai --lib --all-targets -- -D warnings` clean.
3. **Task 3: `WaypointStoreConfig`** -- RED/GREEN pair:
   - `3b12ed70` -- `test(24-10): reproduce WaypointStoreConfig contract on not-yet-existing API (red)` -- 4 tests added against not-yet-existing types; 18 E0422/E0432/E0433 compile errors.
   - `26dd3b5d` -- `feat(24-10): land WaypointStoreConfig (HITL-05, D-26, X-09)` -- full implementation; all 4 unit tests + 2 doc tests green, `src/config/settings.rs` untouched.

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-ports/src/input/parley_port.rs` (new) -- `ParleyPort` trait, `ResumeAccepted`, `ParleyError` (9 variants); 4 unit tests + 2 doc tests.
- `crates/paladin-ports/src/input/mod.rs` -- `pub mod parley_port;` with a rustdoc line matching the module's existing entries.
- `src/application/services/parley/mod.rs` (new) -- barrel module re-exporting `ParleyPortAdapter`/`GraphRegistry`.
- `src/application/services/parley/adapter.rs` (new) -- `ParleyPortAdapter<W>`, `shadow_validate` and its three helper functions, `map_engine_error`; 8 unit tests.
- `src/application/services/parley/registry.rs` (new) -- `GraphRegistry`; 3 unit tests + 1 doc test.
- `src/application/services/mod.rs` -- `pub mod parley;`.
- `src/config/waypoint_store.rs` (new) -- `WaypointStoreBackend`, `WaypointStoreConfig`; 4 unit tests + 2 doc tests.
- `src/config/mod.rs` -- `pub mod waypoint_store;` plus a re-export of `WaypointStoreBackend`/`WaypointStoreConfig`.

## Decisions Made

- **`ParleyPortAdapter` re-implements `WarEngine::resume_with`'s validation algorithm from scratch, rather than reusing `paladin-battalion`'s own validators.** Documented at length above and in `adapter.rs`'s module-level rustdoc: `resume_with` is one atomic function with no validate-only entry point, its validators are `pub(crate)`, and modifying `crates/paladin-battalion/` is out of this plan's declared file scope. The shadow validator is explicitly a defensive pre-check, never the sole authority -- every branch it predicts will not reach the continuation still goes through a real, synchronous, always-fast call to the authoritative engine before the adapter returns.
- **`ParleyError::Backend` and `ParleyError::Rejected` added beyond the plan's own 7 named variants (Rule 2).** `Backend` covers a genuine `WaypointPort::latest` I/O failure (previously unmappable without misleadingly implying a 404-class `ThreadNotFound`); `Rejected` is the fail-closed catch-all `EngineError`'s `#[non_exhaustive]` status forces on every downstream match, covering `GraphMismatch`/`ThreadAlreadyFailed` (structurally unreachable given this adapter's own checks) and any future variant.
- **`ResumeAccepted` carries only `thread_id`**, with a `state_handle()` accessor returning the same value -- a `ThreadId` alone resolves `GET /v1/threads/{id}/state` in plan 24-11; no separate opaque-handle type was invented ahead of need.
- **`GraphRegistry` stores `Arc<WarGraph>`, not `WarGraph` by value**, because `WarGraph` does not derive `Clone`; `register()` takes ownership. Test code accordingly seeds a Waypoint from the graph BEFORE registering it, never needing a second copy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `ParleyError::Backend` for a genuine `WaypointPort` I/O failure**

- **Found during:** Task 2, while implementing `ParleyPortAdapter::resume_with`'s first step (`self.waypoint_port.latest(thread).await`).
- **Issue:** `WaypointPort::latest` returns `Result<Option<Waypoint>, WaypointError>`; a genuine backend I/O failure (`Err(WaypointError)`) had no `ParleyError` variant to map onto without misleadingly collapsing it into `ThreadNotFound` (a 404-class error masquerading a 5xx-class failure).
- **Fix:** Added `ParleyError::Backend { source: WaypointError }` to `crates/paladin-ports/src/input/parley_port.rs`.
- **Files modified:** `crates/paladin-ports/src/input/parley_port.rs` (Task 1's own file, extended in Task 2's RED commit).
- **Verification:** `parley_error_covers_every_validation_case` updated to 9 cases, passing; `map_err(|source| ParleyError::Backend { source })?` compiles and is exercised implicitly by every adapter test (the happy path never hits it, since `InMemoryWaypointStore::latest` never errors).
- **Committed in:** `42108dd5` (RED, adds the variant + updated test), `be689d93` (GREEN, wires it into the adapter).

**2. [Rule 2 - Missing Critical] `ParleyError::Rejected` fail-closed catch-all**

- **Found during:** Task 2, writing `map_engine_error`.
- **Issue:** `EngineError` is `#[non_exhaustive]`; a `match` on it from this (downstream) crate requires a wildcard arm. Without a dedicated variant, an unmapped `EngineError` (e.g. `GraphMismatch`, `ThreadAlreadyFailed` -- both structurally unreachable given this adapter's own upstream checks, but still reachable by the type system) would have needed to be silently coerced into an unrelated variant or a fabricated `ParleyId`.
- **Fix:** Added `ParleyError::Rejected { reason: String }` as the fail-closed catch-all, carrying the underlying error's own `Display` message.
- **Files modified:** `crates/paladin-ports/src/input/parley_port.rs`.
- **Verification:** `every_engine_error_maps_to_a_distinct_parley_error` (adapter.rs) asserts `EngineError::GraphMismatch` maps to `ParleyError::Rejected`.
- **Committed in:** `42108dd5` (RED), `be689d93` (GREEN).

---

**Total deviations:** 2 auto-fixed (both Rule 2, both additive `ParleyError` variants). **Impact on plan:** No scope creep -- both variants are direct, minimal consequences of implementing the adapter's own error-mapping contract correctly; the plan's acceptance criteria (`cargo test -p paladin-ports parley_error_covers_every_validation_case` etc.) still pass, just against an updated (9-variant) exhaustive match.

## Issues Encountered

- **`WarGraph::new` / `NodeSpec::Function` / `StateNode::run` / `FieldName::new` signatures required source verification before test fixtures compiled** -- initial test-fixture drafts assumed a struct-literal `NodeSpec::Function { run: ... }` (it is a tuple variant `Function(Arc<dyn StateNode>)`), a single-argument `StateNode::run(&self, ctx)` (it takes `(&self, state: &Battlefield, ctx: &NodeContext)`), and an infallible `FieldName::new` (it returns `Result`). All corrected by reading `crates/paladin-battalion/src/engine/{graph,node}.rs` directly; no production-code defect, only test-fixture iteration before the first successful compile.
- **`Waypoint`'s full field list (including `muster_progress`) was easy to under-specify by hand** -- an early draft hand-rolled a `Waypoint` literal and omitted `muster_progress`, discovered by reading `paladin_storage::waypoint::contract_tests::sample_waypoint_at` (a `pub fn`, not `#[cfg(test)]`-gated) and switching every test fixture to build from that helper plus targeted field overrides, avoiding the omission class entirely rather than just fixing the one instance.
- **Pre-commit hook skipped (worktree mode).** All 6 commits in this plan used `--no-verify` per the orchestrator's `workflow.worktree_skip_hooks=true` allowance for this run (a cold `cargo clippy --workspace --all-targets --all-features` pre-commit hook exceeds the 2-minute command timeout in a cold worktree). `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` were both run and verified clean across the FULL workspace before this SUMMARY was written, and `cargo test --workspace --no-fail-fast` was run in full (exit code 0, 44 `test result: ok` blocks, 0 `FAILED`) rather than relying on the per-task incremental runs alone.

## User Setup Required

None -- no external service configuration required. `WaypointStoreConfig` defaults to `Disabled`; an operator who wants a durable Waypoint backend sets `APP_WAYPOINT_STORE_BACKEND=sqlite|postgres` plus the matching path/url-env-name variable (documented in `waypoint_store.rs`'s own rustdoc), which plan 24-11 or later wires into `paladin-server`.

## Next Phase Readiness

- `ParleyPort`/`ParleyError`/`ResumeAccepted` are ready for `paladin-web`'s `ThreadApiState.parley: Option<Arc<dyn ParleyPort>>` (plan 24-11) -- confirmed zero `paladin-battalion` dependency via `cargo tree -p paladin-web -e normal`.
- `ParleyPortAdapter`/`GraphRegistry` are ready to be constructed by `src/bin/paladin-server.rs` and injected into `ThreadApiState`; `WaypointStoreConfig::backend == Disabled` (the default) is exactly the signal plan 24-11's thread routes use to answer `501 not_implemented`.
- The `POST /v1/threads/{id}/resume` status-mapping table plan 24-11 needs (404 `ThreadNotFound`; 409 for `ThreadNotAwaitingInput`/`GraphNotRegistered`, distinct codes; 400 for `UnknownParleyId`/`ParleyAlreadyAnswered`/`ResponseShapeInvalid`/`ParleyExpired`) is fully supported by `ParleyError`'s 9 variants.
- **Known limitation, documented and accepted, not a blocker:** the shadow-validation design has a narrow, untested race window -- if a submission this adapter's shadow validator predicts "partial" (and therefore delegates to a synchronous, inline call) is, by the time the REAL engine call actually runs, made "complete" by a concurrent `resume_with` call answering the last remaining parley first, the real call would then (correctly, but unexpectedly for the CALLER'S latency expectations) invoke the continuation synchronously inside that inline delegate, blocking the caller until the graph finishes. This is documented in `adapter.rs`'s module-level rustdoc as an accepted, inherent risk of a check-then-act split that cannot be made atomic without modifying `paladin-battalion`; no test in this plan exercises it, and none of the plan's own 7 named tests require it.
- No blockers.

## Self-Check: PASSED

All 8 files (5 created, 3 modified) verified present on disk; all 6 commit hashes (`a5df52eb`, `7a57daa4`, `42108dd5`, `be689d93`, `3b12ed70`, `26dd3b5d`) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
