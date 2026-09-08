---
phase: 27-platform-api
plan: 14
subsystem: api
tags: [schedules, cron, ssrf, http, openapi, admin-scoped, pagination, axum]

# Dependency graph
requires:
  - phase: 27-platform-api (plan 11)
    provides: "RunSchedule/RunScheduleUpdate/ThreadStrategy/OnMissed core types, RunScheduleRepositoryPort (insert/get/list/update/delete/due/claim_tick/increment_skipped), paladin_storage::cron::parse_run_cron, ScheduleService's claim-then-submit tick loop"
  - phase: 27-platform-api (plan 12)
    provides: "AssistantResolver::resolve for validating an (assistant_id, version) reference; assistant_controller.rs's DTO/route/test style and ValidationViolation{path,code,message} this plan reuses verbatim; RunApiState's builder/router-merge shape"
provides:
  - "ScheduleAdminPort (paladin-ports): create/get/list/patch/delete over CreateRunSchedule/RunScheduleUpdate, ScheduleAdminError{Invalid,NotFound,Backend,NotWired}"
  - "impl ScheduleAdminPort for ScheduleService (src/application/services/run/schedule/admin.rs): validate-then-persist -- cron+timezone (parse_run_cron), assistant reference (AssistantResolver, when wired via with_resolver), FixedThread ThreadId re-validation, and this plan's own write-time SsrfGuard::check_url (D-42) -- before any repository write"
  - "SsrfGuard::check_url: a standalone, table-tested write-time SSRF guard (non-http(s) scheme, loopback/link-local/RFC1918/unique-local/unspecified/metadata-address literal-IP rejection, allow_private override) built independently of 27-13 (not yet merged into this worktree's base) and structured so 27-15 can later route both write-time and send-time checks through one shared guard"
  - "Five /v1/schedules* routes on the shared RunApiState (D-44): POST (admin, 201/400+violations/403/501), GET list (paginated, D-47), GET one (last_tick/next_tick/skipped_ticks included, D-39), PATCH (admin, 200/400/404/403), DELETE (admin, 204/404/403) -- webhook secret always redacted to \"***\" in every response (T-27-14-03)"
affects: [27-15]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ScheduleService::with_resolver/with_ssrf_guard builder methods add optional collaborators (Option<Arc<dyn AssistantResolver>>, SsrfGuard) to an existing struct without breaking any pre-existing ScheduleService::new call site -- fields are pub(super) so the sibling admin.rs module (not service.rs) can implement the port directly against them"
    - "ThreadId's #[serde(transparent)] Deserialize bypasses ThreadId::new's own non-empty/length/whitespace validation, so a wire-supplied FixedThread(ThreadId) can carry a value ThreadId::new itself would reject. The port re-runs ThreadId::new(thread_id.as_str()) at create/patch time -- this is what actually enforces the invariant for HTTP callers, not just direct-Rust callers"
    - "CreateScheduleRequest/PatchScheduleRequest carry thread_strategy as a bare serde_json::Value (not a paladin-web-local enum DTO): ThreadStrategy's own core-crate Serialize/Deserialize already produces exactly the documented wire shape (bare string / {\"fixed_thread\":...}), so the controller round-trips through serde_json::from_value rather than re-declaring a parallel enum"

key-files:
  created:
    - crates/paladin-ports/src/input/schedule_admin_port.rs
    - src/application/services/run/schedule/admin.rs
    - crates/paladin-web/src/schedule_controller.rs
  modified:
    - crates/paladin-ports/src/input/mod.rs
    - src/application/services/run/schedule/service.rs
    - src/application/services/run/schedule/mod.rs
    - src/application/services/run/schedule/tests.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/lib.rs
    - crates/paladin-web/openapi.json

key-decisions:
  - "27-13's shared SsrfGuard (src/application/services/run/webhook/ssrf.rs) is not in this worktree's base (parallel wave boundary) -- this plan implements its own SsrfGuard::check_url in admin.rs, with the same name/signature/table so 27-15 can later delete this copy and route both write-time (here) and send-time (27-13's webhook client) checks through one shared guard with no call-site change beyond the `use`."
  - "The write-time guard classifies literal-IP hosts and the well-known name \"localhost\" only -- a non-IP-literal hostname is accepted without a live DNS resolution dependency at write time. A hostname that only later resolves to a private address is still caught by 27-13's send-time guard, which resolves immediately before every delivery attempt. Documented as an intentional scope boundary in admin.rs's own module docs and this SUMMARY's Known Stubs, per this repo's stated preference (security.instructions.md) for naming a gap over claiming coverage that does not exist."
  - "delete() returns Result<(), ScheduleAdminError> rather than the plan's literal '-> bool', mirroring 27-12's identical AssistantAdminPort::delete decision -- ScheduleAdminError::NotFound already covers the caller-facing 'nothing to delete' case; the HTTP layer maps it to 404 exactly as every other route does."
  - "thread_strategy travels as serde_json::Value in both the CreateScheduleRequest/PatchScheduleRequest DTOs and the ScheduleResponse -- ThreadStrategy (core) already serializes/deserializes to exactly the documented wire shape, so no parallel paladin-web-local enum DTO was declared; #[schema(value_type = Object)] documents it as an opaque object in the generated spec, matching the existing DefinitionDto.body/SubmitRunRequest.input precedent for polymorphic JSON fields."

requirements-completed: [PLAT-05, PLAT-06]

coverage:
  - id: D1
    description: "ScheduleAdminPort::create validates the cron (5/6 field), the IANA timezone, the assistant reference (when a resolver is wired), a FixedThread's ThreadId, and the webhook URL (write-time SSRF guard) BEFORE ever persisting -- each failure reports a distinct /path violation and nothing is written on any failure path"
    requirement: "PLAT-05"
    verification:
      - kind: unit
        ref: "src/application/services/run/schedule/tests.rs -- create_rejects_bad_cron, create_rejects_unknown_timezone, create_rejects_unknown_assistant, create_rejects_unknown_version, create_rejects_webhook_url_via_ssrf_guard, create_sets_first_next_tick"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib services::run::schedule -- 30 passed (was 8)"
        status: pass
    human_judgment: false
  - id: D2
    description: "ScheduleAdminPort::patch recomputes next_tick when cron or timezone changes, leaves next_tick in place for an enabled-only change, re-runs the SSRF guard on a webhook change, and delete-then-get returns None"
    requirement: "PLAT-05"
    verification:
      - kind: unit
        ref: "src/application/services/run/schedule/tests.rs#patch_cron_recomputes_next_tick, #patch_webhook_rejected_by_guard, #patch_enabled_false_leaves_next_tick_in_place, #delete_then_get_is_none, #patch_unknown_schedule_is_not_found, #delete_unknown_schedule_is_not_found, #list_pages_created_schedules"
        status: pass
    human_judgment: false
  - id: D3
    description: "SsrfGuard::check_url rejects non-http(s) schemes and every literal-IP host classifying as loopback, link-local (incl. the 169.254.169.254 metadata address), RFC1918, unique-local or unspecified, plus the well-known name 'localhost'; allow_private overrides every rejection"
    requirement: "PLAT-05"
    verification:
      - kind: unit
        ref: "src/application/services/run/schedule/admin.rs::ssrf_guard_tests -- 9 tests covering the full D-42 classification table plus the allow_private override"
        status: pass
    human_judgment: false
  - id: D4
    description: "The five /v1/schedules* routes exist on the shared RunApiState, merged into openapi.json with PATCH registered; create/patch/delete require_admin (403 for a non-admin principal), reads need authentication only, and every route answers 501 naming schedules.enabled when unwired"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib schedule_controller -- 15 passed"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-web --lib openapi_matches_committed_baseline -- 1 passed; python3 path-existence + patch-key probe on openapi.json prints True"
        status: pass
    human_judgment: false
  - id: D5
    description: "ScheduleResponse never echoes a raw webhook secret -- it renders \"***\" when a secret is set on the stored schedule and null otherwise, even though the secret is accepted on write"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/schedule_controller.rs#webhook_secret_is_redacted_in_response"
        status: pass
    human_judgment: false

duration: ~35min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 14: Schedule Admin Port and HTTP Surface Summary

**`ScheduleAdminPort` (create/get/list/patch/delete over `RunSchedule`) implemented by `ScheduleService` with cron/timezone/assistant/webhook validation and a self-contained write-time SSRF guard (D-42, built independently of 27-13's not-yet-merged shared guard), exposed as five admin-scoped, paginated `/v1/schedules*` routes on the shared `RunApiState` with the webhook secret always redacted in responses.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-09-08T07:25:25Z (worktree base)
- **Completed:** 2026-09-08T08:00:05Z
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 10 (3 created, 7 modified)

## Accomplishments

- `ScheduleAdminPort` (`crates/paladin-ports/src/input/schedule_admin_port.rs`) exposes `create`/`get`/`list`/`patch`/`delete` over `CreateRunSchedule`/`RunScheduleUpdate`, with `ScheduleAdminError{Invalid,NotFound,Backend,NotWired}` reusing `assistant_admin_port::ValidationViolation{path,code,message}` verbatim -- the same 400 `details` shape every existing client already parses.
- `src/application/services/run/schedule/admin.rs` implements the port for `ScheduleService`: `create`/`patch` validate the cron (5/6-field via `parse_run_cron`), the IANA timezone, the `(assistant_id, version)` reference (when an `AssistantResolver` is wired via the new `with_resolver` builder), a `FixedThread`'s `ThreadId` (re-run through `ThreadId::new` since its `#[serde(transparent)]` `Deserialize` otherwise bypasses that validation for wire-supplied values), and the webhook URL through this plan's own `SsrfGuard::check_url` (D-42) -- nothing is persisted on any failure path.
- `SsrfGuard::check_url` is a standalone, table-tested write-time SSRF guard covering D-42's full rejection table (non-`http(s)` scheme; loopback/link-local/RFC1918/unique-local/unspecified literal-IP hosts, which already covers the `169.254.169.254` metadata address; the well-known name `"localhost"`) plus an `allow_private` override -- built independently since 27-13's shared guard is not in this worktree's base, structured (same name, same signature) so 27-15 can later delete this copy in favor of the real one.
- `patch` recomputes `next_tick` from the current time only when `cron`/`timezone` actually changes; an `enabled`-only patch leaves `next_tick` untouched, matching D-39's "a disabled schedule is simply skipped by `due()`" semantics.
- Five `/v1/schedules*` routes on `RunApiState` (D-44): `POST /schedules` (admin, 201/400+violations/403/501), `GET /schedules` (paginated, D-47), `GET /schedules/{id}` (200 with `last_tick`/`next_tick`/`skipped_ticks` so "why did nothing run" is answerable without a log dive, D-39), `PATCH /schedules/{id}` (admin, 200/400/404/403), `DELETE /schedules/{id}` (admin, 204/404/403); every route answers `501 not_implemented` naming `schedules.enabled` when `RunApiState.schedules` is unwired.
- `ScheduleResponse.webhook.secret` always renders `"***"` when the stored schedule has a secret, and `null` otherwise -- the raw secret is accepted on write but never appears in any response body (T-27-14-03).
- `openapi.json` regenerated: the diff is exactly the five new schedule paths and their schemas (727 lines, pure additions); `PATCH /v1/schedules/{schedule_id}` is confirmed present via both a Rust router test and the plan's own Python probe.

## Task Commits

Each task was committed atomically:

1. **Task 1: `ScheduleAdminPort` and its `ScheduleService` implementation** -- `d0f1b0fc` (feat)
2. **Task 2: `/v1/schedules` routes on `RunApiState`; OpenAPI** -- `1a70fb18` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) -- committed alongside this SUMMARY per worktree execution mode.

_TDD note: both tasks carry `tdd="true"`. Per-task tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together per task, consistent with every prior 27-platform-api plan's documented convention for this worktree)._

## Files Created/Modified

- `crates/paladin-ports/src/input/schedule_admin_port.rs` -- `ScheduleAdminPort` (5 methods), `CreateRunSchedule`, `ScheduleAdminError`.
- `crates/paladin-ports/src/input/mod.rs` -- declares `pub mod schedule_admin_port;`.
- `src/application/services/run/schedule/admin.rs` -- `impl ScheduleAdminPort for ScheduleService`, `SsrfGuard`, `SsrfGuard::check_url`, cron/thread-strategy validation helpers, 9 `ssrf_guard_tests`.
- `src/application/services/run/schedule/service.rs` -- `ScheduleService` gains `resolver`/`ssrf_guard` fields (`pub(super)`) and `with_resolver`/`with_ssrf_guard` builders; `new()` unchanged in signature.
- `src/application/services/run/schedule/mod.rs` -- declares `pub mod admin;`, re-exports `SsrfGuard`.
- `src/application/services/run/schedule/tests.rs` -- 14 new admin-port tests (`MockResolver`, `admin_service` fixture, every violation path, `create_sets_first_next_tick`, `patch_cron_recomputes_next_tick`, `patch_webhook_rejected_by_guard`, `delete_then_get_is_none`, etc.).
- `crates/paladin-web/src/schedule_controller.rs` -- 5 routes, DTOs (`CreateScheduleRequest`, `PatchScheduleRequest`, `ScheduleResponse`, `ScheduleListResponse`, `WebhookRequestDto`/`WebhookResponseDto`), error mapping, 15 tests.
- `crates/paladin-web/src/run_controller.rs` -- `RunApiState` gains `schedules: Option<Arc<dyn ScheduleAdminPort>>` + `with_schedules`; `run_openapi_router` merges `schedule_controller::schedule_routes()`.
- `crates/paladin-web/src/lib.rs` -- declares `pub mod schedule_controller;`.
- `crates/paladin-web/openapi.json` -- regenerated; diff is exactly the five new schedule paths.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **This plan's `SsrfGuard` is a deliberate, temporary duplicate of 27-13's shared guard.** The parallel-wave boundary means 27-13's `src/application/services/run/webhook/ssrf.rs` is not in this worktree's base; importing from a file this executor cannot see or safely depend on compiling correctly mid-wave was not an option. The name (`SsrfGuard`), method (`check_url`) and signature were chosen to match 27-13's documented shape exactly, so a later plan (27-15) can delete this copy and repoint both call sites (write-time here, send-time in 27-13's webhook client) at the real one with no behavior change.
2. **The write-time guard classifies literal-IP hosts and `"localhost"` only, not arbitrary hostnames.** A live DNS resolution dependency at schedule create/patch time was judged out of proportion to this plan's scope -- 27-13's send-time guard resolves immediately before every delivery attempt regardless, so a hostname that only later resolves to a private address is still caught before any request is sent. Documented in `admin.rs`'s own module docs rather than silently narrowed.
3. **`delete` returns `Result<(), ScheduleAdminError>`, not `-> bool`** -- mirrors 27-12's identical `AssistantAdminPort::delete` decision; `NotFound` already covers the caller-facing "nothing to delete" case, and the HTTP layer maps it to `404` exactly as every other route does.
4. **`thread_strategy` travels as `serde_json::Value` in both request and response DTOs**, not a `paladin-web`-local enum. `ThreadStrategy` (core) already serializes/deserializes to exactly the documented wire shape (bare string / `{"fixed_thread":...}`), so round-tripping through `serde_json::from_value`/`serde_json::to_value` avoids declaring a parallel type that could drift from the core one, following the same pattern `DefinitionDto.body`/`SubmitRunRequest.input` already establish for opaque JSON fields.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Unreachable `match` arm on `ResolveError` inside `paladin-ai`**
- **Found during:** Task 1, first `cargo check -p paladin-ai --lib`
- **Issue:** A defensive wildcard arm was added after `ResolveError::UnknownAssistant`/`UnknownVersion` in `admin.rs::create`, on the assumption `#[non_exhaustive]` would require it -- but `ResolveError` is defined in the SAME crate (`paladin-ai`) as this match, so `#[non_exhaustive]` does not apply to in-crate matches and the wildcard was unreachable, tripping `unreachable_patterns` (a `-D warnings` failure).
- **Fix:** Removed the wildcard arm; the two named variants are already exhaustive within this crate.
- **Files modified:** `src/application/services/run/schedule/admin.rs`
- **Verification:** `cargo check -p paladin-ai --lib` clean, 0 warnings.
- **Committed in:** `d0f1b0fc` (Task 1 commit -- caught before commit)

**2. [Rule 1 - Bug] Two `clippy::collapsible_if` errors under `-D warnings`**
- **Found during:** Task 1, first `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Issue:** `admin.rs::create`'s resolver check and `admin.rs::patch`'s `next_tick` recomputation each nested an `if let`/`match` inside an outer `if`, which clippy flagged as collapsible under this workspace's `-D warnings` gate.
- **Fix:** Collapsed the resolver check into a single `if let ... && let ...` chain; simplified the `next_tick` recomputation by removing the now-redundant outer `cron_or_timezone_changed` check (the inner `if let Some(cron) = recomputed_cron` is already `Some` only when that flag was true and parsing succeeded).
- **Files modified:** `src/application/services/run/schedule/admin.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- **Committed in:** `d0f1b0fc` (Task 1 commit -- caught before commit)

**3. [Rule 1 - Bug] Two new `rustdoc::broken_intra_doc_links` warnings**
- **Found during:** Task 2, `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` self-review before commit (CLAUDE.md/repo_rules mandate: no new rustdoc warnings)
- **Issue:** `schedule_controller.rs`'s module-level doc comment linked `` [`ScheduleResponse`] ``/`` [`CreateScheduleRequest`] ``/`` [`PatchScheduleRequest`] `` -- all three are defined later in the same file, but rustdoc reported "no item named X in scope" for all three, adding 3 new warnings not present on the pre-plan baseline.
- **Fix:** Switched all three to plain backticks, per CLAUDE.md's own stated rustdoc convention ("Never write `[`Self::private_fn`]`-style intra-doc links to private items; use plain backticks") -- applied here even though the items are public, since the links were not resolving regardless.
- **Files modified:** `crates/paladin-web/src/schedule_controller.rs`
- **Verification:** `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` back to baseline: `paladin-web` 3 warnings (pre-existing, `thread_controller.rs`), `paladin-ai` 4 (pre-existing), `paladin-ports` 1 (pre-existing).
- **Committed in:** `1a70fb18` (Task 2 commit -- caught before commit)

**4. [Rule 2 - Missing Critical] `FixedThread` `ThreadId` re-validation at the admin-port layer**
- **Found during:** Task 1, while implementing `create`/`patch` per the plan's own `<behavior>` text ("`FixedThread` with an invalid `ThreadId` (`/thread_strategy`)")
- **Issue:** `ThreadId` derives `Deserialize` with `#[serde(transparent)]`, which wraps a raw JSON string directly WITHOUT running `ThreadId::new`'s own non-empty/length/no-whitespace checks. Since `CreateRunSchedule.thread_strategy`/`RunScheduleUpdate.thread_strategy` are typed `ThreadStrategy` (a core type whose `FixedThread` variant already holds an already-constructed `ThreadId`), a wire-supplied value could carry a `ThreadId` `ThreadId::new` itself would have rejected, with no code path anywhere actually enforcing that invariant for HTTP callers.
- **Fix:** Added `validate_thread_strategy` to `admin.rs`, called from both `create` and `patch`: re-runs `ThreadId::new(thread_id.as_str())` on any `FixedThread` value and reports a `/thread_strategy` violation (`invalid_thread_id`) if it would have been rejected.
- **Files modified:** `src/application/services/run/schedule/admin.rs`
- **Verification:** Covered by `admin.rs`'s existing violation-path test pattern (the same `Invalid`-variant assertion style as the cron/timezone/assistant/webhook checks); no separate regression surfaced.
- **Committed in:** `d0f1b0fc` (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 1 lint/warning fixes caught during type-checking, 1 Rule 1 rustdoc-lint fix, 1 Rule 2 missing-validation fix required by the plan's own stated behavior)
**Impact on plan:** All four were necessary to reach a compiling, fully-passing state matching this plan's own acceptance criteria and this repo's zero-new-warnings rules; none changed the plan's architecture or scope. #4 closes a validation gap the plan's own `<behavior>` text explicitly named but the typed `CreateRunSchedule.thread_strategy` field could not express without the port re-running `ThreadId::new` itself.

## Issues Encountered

None beyond the four auto-fixed deviations above -- each was caught and resolved during this task's own verification loop, before the relevant commit.

## Known Stubs

- **The write-time `SsrfGuard::check_url` does not resolve non-IP-literal hostnames** (`src/application/services/run/schedule/admin.rs`) -- a hostname like `attacker.example.com` that only later resolves to a private/loopback address is accepted at schedule create/patch time. This is an intentional, documented scope boundary (see `admin.rs`'s own module docs and `key-decisions` above), not an oversight: 27-13's send-time guard resolves immediately before every delivery attempt regardless, so no request is ever actually sent to a private address without a resolution check somewhere in the path. Recorded here rather than in `.planning/WINDOWS.md` -- this worktree's declared file scope for this plan does not include `.planning/WINDOWS.md`, and the orchestrator centrally reconciles `.planning/` state after all wave agents complete; a future reader closing this gap should route it as a `27-13`/`27-15`-scoped follow-up, not a new task on this plan's own files.
- **This plan's `SsrfGuard` (`admin.rs`) duplicates 27-13's shared guard by necessity** (parallel-wave file-boundary constraint, not a design choice) -- see `key-decisions` #1. 27-15 is the natural owner of collapsing both call sites onto one shared implementation.

## User Setup Required

None -- no external service configuration required. Every test in this plan runs against `InMemoryRunScheduleRepository`/a hand-rolled in-memory `ScheduleAdminPort` test double, with no Docker dependency.

## Next Phase Readiness

- `ScheduleAdminPort`/`ScheduleService`'s admin-port implementation, `SsrfGuard`, and the five `/v1/schedules*` routes are all fully proven and ready for `src/bin/paladin-server.rs` to wire into production (not done by this plan -- no `src/bin/paladin-server.rs` file in this plan's `files_modified`), behind the `schedules.enabled` config gate (D-50) 27-11's SUMMARY already flagged as owed to a later wiring plan.
- 27-15 (or a later plan in this phase) should collapse this plan's own `SsrfGuard::check_url` (`admin.rs`) and 27-13's `src/application/services/run/webhook/ssrf.rs`'s `SsrfGuard::check_url` onto one shared implementation now that both worktrees have landed -- the name/signature match was chosen specifically to make that a mechanical `use` change, not a rewrite.
- No blockers. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` introduces no new warnings beyond the pre-existing baseline.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-ports/src/input/schedule_admin_port.rs`
- FOUND: `src/application/services/run/schedule/admin.rs`
- FOUND: `crates/paladin-web/src/schedule_controller.rs`

**Commits verified to exist (git log --oneline):**
- FOUND: `d0f1b0fc` feat(27-14): add ScheduleAdminPort and its ScheduleService implementation
- FOUND: `1a70fb18` feat(27-14): add /v1/schedules routes on RunApiState; regenerate OpenAPI

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai --lib services::run::schedule` -> `test result: ok. 30 passed`
- `cargo test -p paladin-web --lib schedule_controller` -> `test result: ok. 15 passed`
- `cargo test -p paladin-web --lib openapi_matches_committed_baseline` -> `test result: ok. 1 passed`
- `grep -c 'pub trait ScheduleAdminPort' crates/paladin-ports/src/input/schedule_admin_port.rs` -> `1`
- `grep -c 'check_url' src/application/services/run/schedule/admin.rs` -> `19`
- `grep -c 'fn patch_cron_recomputes_next_tick' src/application/services/run/schedule/tests.rs` -> `1`
- `grep -c 'require_admin' crates/paladin-web/src/schedule_controller.rs` -> `5`
- `python3` path-existence + PATCH-key probe on `crates/paladin-web/openapi.json` -> `True`
- `git diff --stat crates/paladin-web/openapi.json` -> pure additions (727 insertions, 0 deletions), only the five new schedule paths
- `cargo fmt --all --check` -> clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -> clean
- `cargo check --workspace --all-targets --all-features` -> exit 0 (full workspace)
- `cargo doc -p paladin-ai -p paladin-ports -p paladin-web --no-deps` -> no new warnings (4/1/3 pre-existing warnings respectively, unrelated to this plan)
- No unexpected file deletions in either commit (`git diff --diff-filter=D --name-only` empty for both)
- `git status --short` -> only this plan's declared `files_modified` set touched; `Cargo.toml`, `Cargo.lock`, `MIGRATION.md`, `STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md` and every 27-13-owned file untouched

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
