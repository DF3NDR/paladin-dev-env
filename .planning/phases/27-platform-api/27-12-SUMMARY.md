---
phase: 27-platform-api
plan: 12
subsystem: api
tags: [assistants, validation, wargraphdoc, run-submission, parley-resume, hexagonal-ports, openapi]

# Dependency graph
requires:
  - phase: 27-platform-api (plan 05)
    provides: "WarGraphDoc::compile(&EngineRegistries) -> Result<WarGraph, CompileError> -- compile IS validation"
  - phase: 27-platform-api (plan 09)
    provides: "AssistantRepositoryPort (no update method), Assistant/AssistantVersion/AssistantDefinition core types, RunRepositoryPort::insert_with_latest"
  - phase: 27-platform-api (plan 01)
    provides: "AssistantResolver trait, ResolvedAssistant/Runnable, CodeWorkflowResolver, RunSubmissionService"
  - phase: 27-platform-api (plan 10)
    provides: "the current run_controller.rs shape (RunApiState, run_openapi_router) this plan extends"
provides:
  - "AssistantAdminPort (create/publish_version/get/get_version/list/list_versions/delete -- no update method), ValidationViolation, PublishAssistant, AssistantAdminError (crates/paladin-ports/src/input/assistant_admin_port.rs)"
  - "AssistantValidator -- compile-is-validation (D-31): Agent bodies validate structurally into a real Paladin via AgentDefinition (facade twin of paladin-web's AgentSpec JSON shape); Workflow bodies validate via WarGraphDoc::compile against EngineRegistries; a credential_field_forbidden scan at any depth; tools/middleware accepted-but-rejected-if-nonempty"
  - "AssistantService implements AssistantAdminPort over AssistantRepositoryPort, validating before every write"
  - "StoredAssistantResolver (re-validates and caches immutable published versions forever) + ChainedResolver (stored first, code second) behind the existing services::run::resolver::AssistantResolver seam"
  - "DocGraphRegistry -- resolves a suspended thread's graph from the run row's frozen (assistant_id, version) via a new GraphResolver trait; GraphRegistry implements the same trait by fingerprint (X-03, no breaking change to paladin-server)"
  - "RunSubmissionService::submit routes a version:None submission against a Stored assistant through insert_with_latest (D-30); a pinned version or a code-registered assistant uses a plain insert"
  - "Seven /v1/assistants* routes on the shared RunApiState (D-44): POST/GET/DELETE /assistants, POST/GET /assistants/{id}/versions[/{version}] -- no PUT/PATCH route ever registered (D-29); synthetic code-registry entries in GET /assistants (D-32); 409 code_registered_immutable on every mutating route for a code id, checked BEFORE the admin port is ever called"
affects: [27-15]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Direct Node::new(PaladinData, name) construction for an Agent-kind assistant, mirroring WarGraphDoc::compile's own PaladinNodeDoc -> PaladinData conversion, rather than routing through PaladinBuilder -- avoids an Arc<dyn LlmPort> dependency in AssistantValidator entirely, since the builder's only real use of that dependency (a provider's declared temperature_range) is deliberately NOT what this validator checks against (a stored definition outlives any one provider wiring; the validator enforces a provider-agnostic [0.0, 1.0] range instead)."
    - "GraphResolver trait takes BOTH thread and fingerprint (not one or the other): GraphRegistry's impl ignores thread and looks up by fingerprint (X-03, unchanged behavior); DocGraphRegistry's impl ignores fingerprint and looks up the thread's active run's frozen (assistant_id, version) instead. This reconciles the plan's two descriptions (\"resolve_for_thread\" vs \"GraphRegistry implements the same trait by fingerprint\") without requiring GraphRegistry to hold a second WaypointPort dependency."
    - "Code-registry-conflict check lives in the HTTP controller, not the admin port: assistant_controller.rs checks state.code_registry before calling create/publish_version/delete, so AssistantService/AssistantAdminPort stay code-registry-agnostic and AgentRegistry is never touched by a mutating assistant route (X-03)."

key-files:
  created:
    - crates/paladin-ports/src/input/assistant_admin_port.rs
    - src/application/services/assistant/mod.rs
    - src/application/services/assistant/service.rs
    - src/application/services/assistant/validator.rs
    - src/application/services/assistant/resolver.rs
    - src/application/services/assistant/doc_registry.rs
    - src/application/services/assistant/tests.rs
    - crates/paladin-web/src/assistant_controller.rs
  modified:
    - crates/paladin-ports/src/input/mod.rs
    - src/application/services/mod.rs
    - src/application/services/run/resolver.rs
    - src/application/services/run/submission.rs
    - src/application/services/parley/adapter.rs
    - src/application/services/parley/registry.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/lib.rs
    - crates/paladin-web/openapi.json

key-decisions:
  - "AssistantValidator holds only Arc<EngineRegistries>, no Arc<dyn LlmPort>/paladin_port -- the plan's action text offered either as an option; constructing the Agent's Paladin directly via Node::new (the same pattern WarGraphDoc::compile already uses for a paladin node) makes an LLM-port dependency unnecessary, since this validator's own temperature check is provider-agnostic by design (a stored definition outlives any one provider)."
  - "GraphResolver::resolve(&self, thread: &ThreadId, fingerprint: &GraphFingerprint) -> Option<Arc<WarGraph>> takes both parameters, with each implementation ignoring the one it does not need -- reconciles the plan's two phrasings for this trait without adding a second dependency to GraphRegistry."
  - "delete(id) returns Result<(), AssistantAdminError> rather than the plan's literal '-> bool' -- mirrors AssistantRepositoryPort::soft_delete's own Result<(), AssistantRepositoryError> shape exactly (NotFound already covers the caller-facing 'nothing to delete' case); more idiomatic than an ok/not-found boolean and the HTTP layer maps NotFound to a 404 exactly as it does everywhere else."
  - "The code-registry-conflict (409 code_registered_immutable) check lives in assistant_controller.rs, checked BEFORE calling the admin port -- not inside AssistantService/AssistantAdminPort, which stay code-registry-agnostic. Keeps the facade service's dependency surface to AssistantRepositoryPort + AssistantValidator only."
  - "GET /assistants merges synthetic code-registry entries only on the FIRST page (cursor is None) -- a full heterogeneous keyset merge across the stored repository and the in-process AgentRegistry, spanning multiple pages, is out of scope for this plan. Documented in-code and recorded in .planning/WINDOWS.md (id 29, kind deviation, open) rather than silently narrowed."

requirements-completed: [PLAT-04]

coverage:
  - id: D1
    description: "AssistantValidator makes compile the validation gate: Agent bodies validate structurally into a real Paladin, Workflow bodies validate via WarGraphDoc::compile against EngineRegistries with every CompileError variant mapped to a distinct ValidationViolation code, and a credential_field_forbidden scan rejects api_key/token/secret/authorization at any depth"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "src/application/services/assistant/tests.rs -- 21 validator/service/freeze-at-submit tests, e.g. empty_agent_body_is_missing_field, agent_body_with_credential_field_at_any_depth_is_forbidden, workflow_with_unregistered_edge_evaluator_is_rejected, valid_agent_definition_yields_a_runnable_paladin, valid_workflow_definition_compiles"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib services::assistant -- 28 passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "No update method exists anywhere on AssistantAdminPort or AssistantRepositoryPort; a stored version is immutable -- publishing v2 never rewrites v1, proven behaviorally"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "src/application/services/assistant/tests.rs#assistant_version_immutable"
        status: pass
      - kind: other
        ref: "grep -c 'async fn update' crates/paladin-ports/src/input/assistant_admin_port.rs == 0; grep -cE 'routes!\\((put|patch)_' crates/paladin-web/src/assistant_controller.rs == 0; grep -c 'put(' crates/paladin-web/src/assistant_controller.rs == 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "POST /runs without version resolves a stored assistant's latest through insert_with_latest (D-30); publishing a new version after a run was submitted never changes the already-persisted run's frozen version"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "src/application/services/assistant/tests.rs#submit_without_version_freezes_latest"
        status: pass
      - kind: unit
        ref: "src/application/services/run/submission.rs -- submit_without_version_against_a_stored_assistant_freezes_latest, submit_with_explicit_version_uses_a_pinned_insert_not_latest"
        status: pass
    human_judgment: false
  - id: D4
    description: "The seven /v1/assistants* routes exist on the shared RunApiState, merged into openapi.json; no PUT/PATCH route registered; mutating routes require admin and reject a code-registered id with 409 before the admin port is ever called"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib assistant_controller -- 14 passed"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline -- 1 passed; python3 path-existence + no-put probe on openapi.json prints True"
        status: pass
    human_judgment: false
  - id: D5
    description: "POST /assistants with an empty definition body {} returns 400 with a non-empty machine-readable violation list in details, and nothing is persisted (a follow-up GET returns 404)"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/assistant_controller.rs#post_assistants_empty_definition_body_is_400_and_persists_nothing"
        status: pass
    human_judgment: false

duration: ~45min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 12: Assistant Publish, Resolve & Route Summary

**Assistants become runnable end-to-end: `AssistantValidator` makes compile the validation gate for both Agent and Workflow definitions, `AssistantService`/`AssistantAdminPort` publish append-only immutable versions, `StoredAssistantResolver`/`ChainedResolver` let `POST /runs` freeze a stored assistant's `latest` at submit time, `DocGraphRegistry` lets a resume find a suspended run's graph through that frozen version, and seven `/v1/assistants*` routes expose it all with synthetic code-registry entries and a 409 on any attempt to mutate one.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-09-08T06:27:20Z (worktree base)
- **Completed:** 2026-09-08T07:12:53Z
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 17 (8 created, 9 modified)

## Accomplishments

- `AssistantAdminPort` (`crates/paladin-ports/src/input/assistant_admin_port.rs`) exposes exactly `create`/`publish_version`/`get`/`get_version`/`list`/`list_versions`/`delete` — grep-verified zero `async fn update` — with `ValidationViolation { path, code, message }` as the published `400` `details` shape every client parses.
- `AssistantValidator` (`src/application/services/assistant/validator.rs`) is the compile-is-validation gate (D-31): an Agent body validates structurally through `AgentDefinition` (the facade's own web-independent twin of `paladin-web`'s `AgentSpec` JSON shape) and builds a real `Paladin` via direct `Node::new` construction (mirroring `WarGraphDoc::compile`'s own precedent, no `LlmPort` dependency needed); a Workflow body validates by deserialising to a `WarGraphDoc` and calling `compile()` against the caller's `EngineRegistries`, with every `CompileError` variant mapped to a distinct `ValidationViolation` code; a recursive credential scan rejects `api_key`/`token`/`secret`/`authorization` at any depth in either body shape (prohibition P1, threat T-27-12-04).
- `AssistantService` implements `AssistantAdminPort` over `AssistantRepositoryPort`, validating before every write — an invalid definition never reaches the repository.
- `StoredAssistantResolver` (re-validates a stored version at resolve time and caches the immutable result forever per `(assistant_id, version)`) and `ChainedResolver` (tries stored first, falls back to code) extend `services::run::resolver::AssistantResolver` — the SAME trait 27-01's `CodeWorkflowResolver` already implements; `ResolvedAssistant` gained a `source: AssistantSource` field so downstream code can tell stored from code-registered resolutions.
- `RunSubmissionService::submit` routes a `version: None` submission against a Stored assistant through `RunRepositoryPort::insert_with_latest` (D-30, freezing the version inside the database's own atomic insert); a pinned `version: Some(v)` or a code-registered assistant uses a plain `insert`.
- A new `GraphResolver` trait (`parley/adapter.rs`) lets `ParleyPortAdapter` resolve a thread's graph by EITHER fingerprint (`GraphRegistry`, unchanged behavior, X-03) OR the thread's active run's frozen `(assistant_id, version)` (`DocGraphRegistry`, `assistant/doc_registry.rs`) — both implement the same trait, so `paladin-server`'s existing empty-registry construction keeps compiling via `Arc` unsized coercion with no source change required there.
- `crates/paladin-web/src/assistant_controller.rs` adds seven `/v1/assistants*` routes on the shared `RunApiState` (D-44), merged into `run_openapi_router` so one router carries both `/v1/runs*` and `/v1/assistants*`: `POST /assistants` (admin, 201/400/409), `GET /assistants` (paginated, merges synthetic `{ assistant_id, latest: 1, source: "code" }` entries on the first page when `expose_code_registry` is on), `GET /assistants/{id}` (falls back to a synthetic entry for a code-registered id), `DELETE /assistants/{id}` (admin, 204/409), `POST /assistants/{id}/versions` (admin, 201/404/409), `GET /assistants/{id}/versions` (ascending, paginated), `GET /assistants/{id}/versions/{version}` (404 for `0`/above-`latest`). No `PUT`/`PATCH` route is ever registered anywhere (D-29) — the router cannot express an update.
- `openapi.json` regenerated: the diff is exactly the four new assistant paths; no pre-existing path changed.

## Task Commits

Each task was committed atomically:

1. **Task 1: `AssistantAdminPort`, `AssistantValidator`, `AssistantService`, `StoredAssistantResolver`, `DocGraphRegistry`** — `c1ebb439` (feat)
2. **Task 2: Assistant routes on `RunApiState` with synthetic code-registry entries; OpenAPI** — `4f96d537` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) — committed alongside this SUMMARY per worktree execution mode.

_TDD note: both tasks carry `tdd="true"`. Per-task tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together per task, consistent with every prior 27-platform-api plan's documented convention for this worktree)._

## Files Created/Modified

- `crates/paladin-ports/src/input/assistant_admin_port.rs` — `AssistantAdminPort` (7 methods, no update), `ValidationViolation`, `PublishAssistant`, `AssistantAdminError`.
- `crates/paladin-ports/src/input/mod.rs` — declares `pub mod assistant_admin_port;`.
- `src/application/services/assistant/mod.rs` — module wiring + re-exports.
- `src/application/services/assistant/validator.rs` — `AssistantValidator`, `AgentDefinition`, `Validated`, credential scan, `CompileError` → `ValidationViolation` mapping.
- `src/application/services/assistant/service.rs` — `AssistantService` implements `AssistantAdminPort`.
- `src/application/services/assistant/resolver.rs` — `StoredAssistantResolver`, `ChainedResolver`.
- `src/application/services/assistant/doc_registry.rs` — `DocGraphRegistry` implements `GraphResolver` by run-row lookup.
- `src/application/services/assistant/tests.rs` — 21 tests: validator cases, service create/version/list/delete, `assistant_version_immutable`, `submit_without_version_freezes_latest`.
- `src/application/services/mod.rs` — declares `pub mod assistant;`.
- `src/application/services/run/resolver.rs` — `ResolvedAssistant` gains `source: AssistantSource`; `CodeWorkflowResolver` sets `AssistantSource::Code`.
- `src/application/services/run/submission.rs` — `submit` routes `version: None` + Stored through `insert_with_latest`; `map_repository_error` gains `UnknownAssistant`; 2 new tests.
- `src/application/services/parley/adapter.rs` — new `GraphResolver` trait; `ParleyPortAdapter.registry` is now `Arc<dyn GraphResolver>`; `resume_with` calls `resolve(thread, fingerprint).await`.
- `src/application/services/parley/registry.rs` — `GraphRegistry` implements `GraphResolver` by fingerprint, `thread` ignored.
- `crates/paladin-web/src/assistant_controller.rs` — 7 routes, DTOs, error mapping, synthetic code-registry entries; 14 tests.
- `crates/paladin-web/src/run_controller.rs` — `RunApiState` gains `assistants`/`code_registry`/`expose_code_registry` + builders; `run_openapi_router` merges `assistant_controller::assistant_routes()`.
- `crates/paladin-web/src/lib.rs` — declares `pub mod assistant_controller;`.
- `crates/paladin-web/openapi.json` — regenerated; diff is exactly the four new assistant paths.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **`AssistantValidator` needs no `LlmPort`.** The plan's action text offered `Arc<dyn LlmPort> or paladin_port` as an option for building the Agent's `Paladin`. Constructing it directly via `Node::new(PaladinData, name)` — the exact pattern `WarGraphDoc::compile`'s own `PaladinNodeDoc` → `PaladinData` conversion already uses — avoids that dependency entirely, since the only real use `PaladinBuilder::build()` would have made of an `LlmPort` here (reading a provider's declared `temperature_range`) is deliberately NOT what this validator checks: a stored assistant definition outlives any one provider wiring, so the validator enforces a provider-agnostic `[0.0, 1.0]` range instead.
2. **`GraphResolver::resolve` takes both `thread` and `fingerprint`.** The plan's own text described this seam two ways — `DocGraphRegistry` "resolves ... by reading the thread's active run's frozen `(assistant_id, version)`" (thread-keyed) versus "`GraphRegistry` implements the same trait by fingerprint" (fingerprint-keyed). Taking both parameters and letting each implementation ignore the one it does not need reconciles both descriptions without giving `GraphRegistry` a second `WaypointPort`/`RunRepositoryPort` dependency it does not otherwise need.
3. **`delete` returns `Result<(), AssistantAdminError>`, not `-> bool`.** Mirrors `AssistantRepositoryPort::soft_delete`'s own shape exactly; `NotFound` already covers "nothing to delete," and the HTTP layer maps it to `404` the same way every other route does. More idiomatic than an `Ok(true)`/`Ok(false)` boolean that would need its own mapping convention.
4. **The `code_registered_immutable` 409 check lives in the controller, not the port.** `AssistantService`/`AssistantAdminPort` never learn about the code registry; `assistant_controller.rs` checks `state.code_registry` before calling `create`/`publish_version`/`delete`, so `AgentRegistry` is never touched by a mutating assistant route (X-03) and the facade service's dependency surface stays to `AssistantRepositoryPort` + `AssistantValidator` only.
5. **`GET /assistants` merges synthetic code entries on the first page only.** A genuinely correct keyset merge across two heterogeneous sources (the stored repository's own cursor and the in-process `AgentRegistry`, which has no cursor concept at all) spanning multiple pages is out of scope for this plan. Recorded in `.planning/WINDOWS.md` (id 29, `kind: deviation`, `status: open`) rather than silently narrowed or left undocumented.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `Validated` needed a hand-written `Debug` impl**
- **Found during:** Task 1, first `cargo check -p paladin-ai --lib --tests`
- **Issue:** `Result<Validated, Vec<ValidationViolation>>::unwrap_err()` in tests requires `Validated: Debug`, but `Validated::Workflow(Arc<WarGraph>)` cannot derive `Debug` (`WarGraph`, `paladin-battalion`, does not implement it).
- **Fix:** Added a hand-written, deliberately shallow `Debug` impl (prints `Validated::Agent(..)`/`Validated::Workflow(..)`, never internals) — the exact same precedent `services::run::resolver::Runnable` already established for the identical problem.
- **Files modified:** `src/application/services/assistant/validator.rs`
- **Verification:** `cargo check -p paladin-ai --lib --tests` clean; all 28 `services::assistant` tests pass.
- **Committed in:** `c1ebb439` (Task 1 commit)

**2. [Rule 3 - Blocking] `paladin-web` does not depend on `paladin-storage`**
- **Found during:** Task 2, first `cargo check -p paladin-web --lib --tests`
- **Issue:** The test module's first draft used `paladin_storage::assistant::in_memory::InMemoryAssistantRepository` as a convenient test double, but `crates/paladin-web/Cargo.toml` has no `paladin-storage` dependency (dev or prod) — adding one would be a new-dependency edit this plan's parallel-execution boundary requires stopping and reporting as a blocker, not silently making.
- **Fix:** Replaced the test double with a fully self-contained `TestAdminPort` implementing `AssistantAdminPort` directly over a plain `Mutex<HashMap<..>>`, needing no new crate dependency.
- **Files modified:** `crates/paladin-web/src/assistant_controller.rs`
- **Verification:** `cargo check -p paladin-web --lib --tests` clean; all 14 `assistant_controller` tests pass; `git diff crates/paladin-web/Cargo.toml` is empty (no dependency edit made).
- **Committed in:** `4f96d537` (Task 2 commit)

**3. [Rule 1 - Bug] A self-referential `include_str!` test matched its own assertion string**
- **Found during:** Task 2, first `cargo test -p paladin-web --lib assistant_controller`
- **Issue:** A test asserting "no `routes!(put_...)`/`routes!(patch_...)` in this file" via `include_str!(...).matches(...)` matched its OWN string-literal assertion text, failing `1 == 0`.
- **Fix:** Removed the self-referential Rust test (the equivalent check is the plan's own grep-based acceptance criterion, run externally, plus the runtime proof `no_put_or_patch_route_exists` already covers via a real mounted-router request) and reworded the explanatory comment so it does not itself contain the matched substring.
- **Files modified:** `crates/paladin-web/src/assistant_controller.rs`
- **Verification:** `grep -cE 'routes!\((put|patch)_' crates/paladin-web/src/assistant_controller.rs` → `0`; all 14 tests pass.
- **Committed in:** `4f96d537` (Task 2 commit)

**4. [Rule 1 - Bug] Two new rustdoc `broken_intra_doc_links`/`private_intra_doc_links` warnings**
- **Found during:** Task 2, `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` self-review before commit (CLAUDE.md/repo_rules mandate: no new rustdoc warnings)
- **Issue:** Module-doc links `` [`assistant_routes`] ``, `` [`require_admin`] `` and two `` [`MAX_ASSISTANT_LIMIT`] `` references pointed at `pub(crate)`/private items, tripping `rustdoc::broken_intra_doc_links`/`rustdoc::private_intra_doc_links` — 4 new warnings not present on the pre-plan baseline.
- **Fix:** Switched all four to plain backticks, per CLAUDE.md's own stated rustdoc convention ("Never write `[`Self::private_fn`]`-style intra-doc links to private items; use plain backticks").
- **Files modified:** `crates/paladin-web/src/assistant_controller.rs`
- **Verification:** `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` → `paladin-web` back to 3 warnings (all pre-existing, `thread_controller.rs`), `paladin-ai` at 4 (pre-existing), `paladin-ports` at 1 (pre-existing, `structured_executor_port.rs`, unrelated).
- **Committed in:** `4f96d537` (Task 2 commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 1 bug fixes discovered via type-checking, 1 Rule 1 bug fix from a self-referential test, 1 Rule 1 rustdoc-lint fix, 1 Rule 3 blocking fix avoiding an undeclared new dependency)
**Impact on plan:** All four were necessary to reach a compiling, fully-passing state matching the plan's own acceptance criteria and this repo's zero-new-rustdoc-warnings/no-undeclared-dependency rules; none changed the plan's architecture or scope.

## Issues Encountered

None beyond the four auto-fixed deviations above — each was caught and resolved during this task's own verification loop, before the relevant commit.

## Known Stubs

- **`GET /assistants` code-registry merge is first-page-only** (`crates/paladin-web/src/assistant_controller.rs::list_assistants`) — intentional scope boundary, not a bug: a correct keyset merge across the stored repository's own cursor and the code-registered `AgentRegistry` (which has no cursor concept) across multiple pages was judged out of scope for this plan. Recorded in `.planning/WINDOWS.md` (id 29, `kind: deviation`, `status: open`). No future plan is currently assigned to close it; a later Platform API pagination plan (PLAT-06 scope) is the natural owner if a caller ever needs it.
- **`tools`/`middleware` fields on `AgentDefinition` are accepted-but-rejected-if-non-empty** (`crates/paladin-ports/src/input/assistant_admin_port.rs` is unaffected; the field lives in `src/application/services/assistant/validator.rs`'s `AgentDefinition`) — deliberate forward-compatibility placeholder per the plan's own action text ("accepted but validated as empty until a later phase wires them"), not a functional gap this plan was asked to close.

## User Setup Required

None — no external service configuration required. Every test in this plan runs against `InMemoryAssistantRepository`/`InMemoryRunRepository`/a hand-rolled in-memory `AssistantAdminPort` test double, with no Docker dependency.

## Next Phase Readiness

- `AssistantAdminPort`/`AssistantService`/`AssistantValidator` are ready for `src/bin/paladin-server.rs` to wire into production (not done by this plan — no `src/bin/paladin-server.rs` file in this plan's `files_modified`); the seam is proven end-to-end against `InMemory*` adapters and is adapter-agnostic (any `AssistantRepositoryPort` implementor — SQLite, Postgres, from 27-09 — plugs in unchanged).
- `StoredAssistantResolver`/`ChainedResolver` are ready for a later plan's production `RunSubmissionService`/`ParleyPortAdapter` wiring to swap in for the tracer's bare `CodeWorkflowResolver`, with zero breaking change to the `AssistantResolver` trait itself.
- `DocGraphRegistry` is ready to replace `GraphRegistry` at the `ParleyPortAdapter` construction site once a later plan wires a real `AssistantResolver`/`RunRepositoryPort` pair into production — `GraphResolver`'s dual `(thread, fingerprint)` signature means either can be swapped in without touching `ParleyPortAdapter` itself.
- No blockers. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` introduces no new warnings beyond the pre-existing baseline.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-ports/src/input/assistant_admin_port.rs`
- FOUND: `src/application/services/assistant/mod.rs`
- FOUND: `src/application/services/assistant/service.rs`
- FOUND: `src/application/services/assistant/validator.rs`
- FOUND: `src/application/services/assistant/resolver.rs`
- FOUND: `src/application/services/assistant/doc_registry.rs`
- FOUND: `src/application/services/assistant/tests.rs`
- FOUND: `crates/paladin-web/src/assistant_controller.rs`

**Commits verified to exist (git log --oneline):**
- FOUND: `c1ebb439` feat(27-12): add AssistantAdminPort, AssistantValidator, AssistantService, StoredAssistantResolver, DocGraphRegistry
- FOUND: `4f96d537` feat(27-12): add assistant routes on RunApiState with synthetic code-registry entries

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai --lib services::assistant` → `test result: ok. 28 passed`
- `cargo test -p paladin-ai --lib assistant_version_immutable` → `test result: ok. 1 passed`
- `cargo test -p paladin-ai --lib services::parley` → `test result: ok. 18 passed`
- `cargo test -p paladin-web --lib assistant_controller` → `test result: ok. 14 passed`
- `cargo test -p paladin-web --lib openapi_matches_committed_baseline` → `test result: ok. 1 passed`
- `cargo test -p paladin-web --lib` (full crate) → `test result: ok. 166 passed`
- `grep -c 'async fn update' crates/paladin-ports/src/input/assistant_admin_port.rs` → `0`
- `grep -c 'credential_field_forbidden' src/application/services/assistant/validator.rs` → `2`
- `grep -c 'insert_with_latest' src/application/services/run/submission.rs` → `3`
- `grep -c 'dyn GraphResolver' src/application/services/parley/adapter.rs` → `2`
- `grep -v '^\s*//' src/application/services/assistant/*.rs | grep -c 'paladin_web'` → `0`
- `grep -cE 'routes!\((put|patch)_' crates/paladin-web/src/assistant_controller.rs` → `0`
- `grep -c 'put(' crates/paladin-web/src/assistant_controller.rs` → `0`
- `grep -c 'code_registered_immutable' crates/paladin-web/src/assistant_controller.rs` → `14`
- `grep -c 'require_admin' crates/paladin-web/src/assistant_controller.rs` → `5`
- `python3` path-existence + no-`put`-key probe on `crates/paladin-web/openapi.json` → `True`
- `git diff --stat crates/paladin-web/openapi.json` → pure additions (831 insertions, 0 deletions), only the four new assistant paths
- `cargo fmt --all -- --check` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- `cargo check --workspace --all-targets --all-features` → exit 0 (full workspace)
- `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` → no new warnings (3/4/1 pre-existing warnings respectively, unrelated to this plan)
- No unexpected file deletions in either commit (`git diff --diff-filter=D --name-only` empty for both)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
