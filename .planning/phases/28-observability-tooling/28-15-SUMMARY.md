---
phase: 28-observability-tooling
plan: 15
subsystem: ui
tags: [rust, axum, hexagonal-architecture, dev-tool, mermaid, html, admin-gated]

# Dependency graph
requires:
  - phase: 28-observability-tooling (plan 14)
    provides: "RunInspectorPort::inspect(&ThreadId) -> Result<InspectorView, InspectorError> and the whole InspectorView/SuperstepRow/CompletedRow/VisitSummary/InspectorSource/SuperstepStatus value chain (core-typed, ADR-0016/0031) -- the page's only data source"
  - phase: 28-observability-tooling (plan 02)
    provides: "DevUiConfig::mermaid_url default (jsDelivr ESM bundle) the served page substitutes at request time"
provides:
  - "crates/paladin-web's FIRST [features] section, declaring dev-ui (default off, default = [] explicit)"
  - "GET /v1/dev-ui/threads/{id} (dev_ui_controller.rs): admin-gated, feature-gated, renders one thread's InspectorView as a static HTML page; 404 for an unknown thread, 501 when no RunInspectorPort is wired, both through the structured ApiError envelope"
  - "crates/paladin-web/src/dev_ui/inspector.html: the four-panel static template (diagram, node visits, fired edges, superstep table) with an inline vanilla-JS renderer covering every UI-SPEC state row, included at compile time via include_str!"
  - "create_dev_ui_router (app.rs): mounts the route under the SAME require_auth/require_admin middleware layers create_app_router's admin_routes block already uses"
  - "Root Cargo.toml umbrella dev-ui feature and a new .github/workflows/feature-flags.yml matrix leg"
affects: ["28-16 (integration test helpers/evals -- ran in a parallel worktree, untouched by this plan)", "28-17 (docs/MIGRATION.md rows referencing this feature)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Static template + typed JSON script element (no build pipeline): InspectorView is serialized once, HTML-escaped for `</` and `<!--` (T-28-15-02), and substituted into a #inspector-data <script type=\"application/json\"> element; a second #dev-ui-mermaid-config script element carries the JSON-string-escaped Mermaid URL the same way, so neither substitution risks breaking out of its surrounding tag"
    - "Admin gating reuses app.rs's pre-existing require_auth/require_admin middleware pair (AuthPort + AuthClaims) verbatim rather than the agent_auth Principal system thread/run/schedule controllers use -- the dev-ui route is a standalone axum::Router (create_dev_ui_router), not merged into create_app_router or an OpenApiRouter, so it carries no #[utoipa::path] and cannot enter openapi.json regardless of feature state"
    - "Client-side rendering only: the server hands over one immutable JSON payload; all four panels (including the payload-parse-failure banner) render synchronously from that payload inside one try/catch, with zero additional network requests except the Mermaid ESM module import"

key-files:
  created:
    - crates/paladin-web/src/dev_ui_controller.rs
    - crates/paladin-web/src/dev_ui/inspector.html
  modified:
    - crates/paladin-web/Cargo.toml
    - crates/paladin-web/src/app.rs
    - crates/paladin-web/src/lib.rs
    - Cargo.toml
    - .github/workflows/feature-flags.yml

key-decisions:
  - "Admin gating reuses app.rs's require_auth/require_admin (AuthPort/AuthClaims/UserRole::Admin) rather than agent_auth's Principal-based require_authentication the thread/run/schedule controllers use -- both are legitimate RBAC layers in this crate; app.rs's pair is the one the plan's own read_first and acceptance criteria (grep for require_admin in app.rs) point at, and RunInspectorPort's own module docs name `require_auth` + `require_admin` by those exact identifiers as the dev-ui route's authorization boundary."
  - "create_dev_ui_router is NOT merged into create_app_router by default -- it mirrors how paladin-server.rs merges thread_router/run_router alongside agent_router rather than inside create_app_router. Production wiring of a real RunInspectorPort backend into a running server binary (paladin-server.rs) is out of this plan's files_modified scope; this plan proves the route, template and admin gating against an in-test port only, per the plan's own acceptance criteria."
  - "The Mermaid classDef color block (light/dark hex triples per 28-UI-SPEC.md) is assembled and appended client-side by the page's own JS, not baked into InspectorView.mermaid server-side -- a static server-rendered Mermaid source cannot know the browser's OS theme preference at render time, so theme-awareness has to live in the client."
  - "Cache-hit CompletedRow duration_ms/token_count are None by construction (28-14) -- the page renders a dash for these rather than deriving one client-side, keeping the 'no misleading number' guarantee end-to-end from port to page."

requirements-completed: [OBS-03]

coverage:
  - id: D1
    description: "crates/paladin-web's first [features] section declares dev-ui (default off); GET /v1/dev-ui/threads/{id} is admin-gated (require_auth + require_admin) and feature-gated"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_unauthenticated_request_is_rejected"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_authenticated_non_admin_request_is_rejected"
        status: pass
      - kind: other
        ref: "grep -c '^\\[features\\]' crates/paladin-web/Cargo.toml == 1; cargo build -p paladin-web (default features) compiles the controller out"
        status: pass
    human_judgment: false
  - id: D2
    description: "The route carries no OpenAPI path attribute and is structurally absent from openapi.json with the feature enabled"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/openapi.rs#openapi_matches_committed_baseline (run with --features dev-ui)"
        status: pass
      - kind: other
        ref: "git diff --exit-code crates/paladin-web/openapi.json"
        status: pass
    human_judgment: false
  - id: D3
    description: "404 for an unknown thread, 501 when no RunInspectorPort is wired, both through the structured ApiError envelope"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_unknown_thread_is_404"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_without_a_wired_port_is_501"
        status: pass
    human_judgment: false
  - id: D4
    description: "The page is one static template with the InspectorView JSON injected into a typed JSON script element; </ and <!-- are escaped in the embedded payload; the page makes no fetches"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_page_returns_html_for_a_known_thread"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_page_escapes_the_embedded_payload"
        status: pass
      - kind: other
        ref: "grep -c 'fetch(' crates/paladin-web/src/dev_ui/inspector.html == 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "The smoke test proves the OBS-03 acceptance question at the payload level: the branching fixture's fired edge and its three-visit summary (count 3, supersteps [2,4,6]) are both present in the embedded JSON"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_page_embeds_the_fired_branch_and_the_three_visit_summary"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every locked Copywriting Contract string and all five outcome CSS classes are present in the served page, independent of request data -- a silent drop of a state is a test failure"
    requirement: "OBS-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_page_contains_every_locked_copy_string"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#dev_ui_page_contains_every_outcome_class"
        status: pass
    human_judgment: false
  - id: D7
    description: "Visual/interaction fidelity of the four rendered panels (diagram render quality, Mermaid theme-switching, actual browser layout at the documented breakpoints) is not exercised by an automated browser test -- the plan's own acceptance scope is DOM/payload-level only (PRD 07 acceptance 4: 'no browser')"
    verification: []
    human_judgment: true
    rationale: "The plan explicitly scopes verification to the embedded JSON payload, not rendered pixels (28-15-PLAN.md <verify>, D-26). A human visually spot-checking the served page against 28-UI-SPEC.md's colour/typography/spacing tables in a real browser is the only way to confirm the inline JS renderer's DOM output matches the design contract; this was not performed as part of automated execution."

# Metrics
duration: 28min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 15: Dev-UI Run Inspector Page Summary

**Feature-gated (`dev-ui`, default off) admin-only `GET /v1/dev-ui/threads/{id}` page rendering `RunInspectorPort`'s `InspectorView` as a static four-panel HTML document (diagram, node visits, fired edges, superstep table) via a typed embedded JSON script element and an inline vanilla-JS renderer -- no build pipeline, no fetches, no values-shown mode.**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-09-09T05:00:13Z
- **Completed:** 2026-09-09T05:27:51Z
- **Tasks:** 2 (1 tracer, 1 auto) -- 2 commits total
- **Files modified:** 7 (2 new, 5 modified)

## Accomplishments

- `crates/paladin-web/Cargo.toml`: the crate's FIRST `[features]` section (`default = []`, `dev-ui = []`) -- every other capability in this crate compiled unconditionally before this plan.
- `crates/paladin-web/src/dev_ui_controller.rs`: `DevUiState` (`Option<Arc<dyn RunInspectorPort>>` + `mermaid_url: String`, `#[derive(Clone)]`), the `dev_ui_inspector_page` handler (escapes `</`/`<!--` in the serialized `InspectorView` before embedding, JSON-string-escapes the Mermaid URL for its own typed script element, maps `InspectorError` -> `ApiError` per D-25's `404`/`501`/`500` split), and 16 tests.
- `crates/paladin-web/src/dev_ui/inspector.html`: a single `include_str!`-ed static template -- spacing scale, four-size/two-weight typography, light+dark colour custom properties (including the five-outcome colour table), the four panels in the specified hierarchy (full-width diagram, a two-column Node Visits/Fired Edges grid collapsing under 900px, full-width Supersteps table), and one inline module script that parses the embedded JSON synchronously (payload-parse-failure banner shared across the three data panels on failure) and separately loads Mermaid from the configured URL with a loading state, a 5s timeout, and a raw-source fallback.
- `crates/paladin-web/src/app.rs`: `create_dev_ui_router(auth_port, state)` -- mounts the route under the SAME `require_auth`/`require_admin` middleware layers `create_app_router`'s `admin_routes` block already uses, reused verbatim rather than reimplemented.
- Root `Cargo.toml`: umbrella `dev-ui = ["paladin-web/dev-ui"]`, absent from `default` and `full` (X-11.4), mirroring `otel`'s identical precedent.
- `.github/workflows/feature-flags.yml`: new `dev-ui` matrix leg (`--no-default-features --features dev-ui`).
- No OpenAPI path attribute anywhere in this module; the route is a plain `axum::Router`, never merged into `build_openapi`'s three router sources -- `openapi.json` is verified byte-identical with the feature enabled.

## UI-SPEC Coverage Map

Every one of the 29 `covered` UI-Consideration rows (28-UI-SPEC.md) and the 1 `backstop` row, mapped to where each is implemented:

| Row | Where implemented |
|-----|--------------------|
| empty E1 | `inspector.html` JS `renderEmptyOrPanels` hides `#diagram-panel` and shows `#empty-state` when `view.supersteps.length === 0`; `renderDiagram` returns before any Mermaid work when the panel is hidden -- no CDN request. |
| loading E1 | `#diagram-loading` shows `"Rendering diagram…"` until `renderDiagram`'s dynamic `import()` + `mermaid.render()` resolve; a 5s `setTimeout` swaps in the error/fallback state. |
| error E1 | `renderDiagram`'s `catch` block and the timeout branch both populate `#diagram-error-text` (`"Mermaid could not be loaded from ... Showing the raw diagram source below."`) and `#diagram-source` with the raw Mermaid text; the Fired Edges panel renders independently of the diagram's success/failure. |
| populated E1 | `diagramClassDefBlock` appends light/dark `classDef` triples for all five outcomes; `mermaid.initialize({ theme: ... })` selects the scheme from `prefers-color-scheme`; the observed-only title suffix is prepended when `view.observed_only`. |
| overflow E1 | `.diagram-container { overflow-x:auto; overflow-y:auto; max-height:70vh; }`. |
| long-text E1 (backstop) | Not independently tested -- inherited from Mermaid's own flowchart label wrapping, exactly as 28-UI-SPEC.md's own resolution states ("Mermaid's own flowchart label wrapping is inherited untested"). No golden fixture surfaced an overflow in this plan's fixtures; left as documented, not silently dropped. |
| empty E2 | `renderVisits` shows `#visits-empty` (`"No nodes have executed yet."`) when `view.visits.length === 0`; proven independent of the whole-page empty state by `dev_ui_page_embeds_populated_supersteps_with_empty_visits`. |
| loading E2 | No loading state exists in the template -- `renderVisits` runs synchronously in the same module script, after the `#inspector-data` element in document order. |
| error E2 | `renderParseFailure` populates `#visits-list` with the shared parse-failure banner + raw payload `<pre>` inside the module script's outer `try/catch`. |
| populated E2 | `renderVisits` builds `"<node_id> ran N time(s): supersteps a, b, c"` per `VisitSummary`; proven by `dev_ui_page_embeds_the_fired_branch_and_the_three_visit_summary` (count 3, supersteps [2,4,6]). |
| partial E2 | Cache-hit rows carry `duration_ms`/`token_count: None` by construction (28-14); `dev_ui_page_embeds_cache_hit_row_with_no_duration_or_tokens` proves the embedded payload carries `null` for both, which the table renderer's `dash()` helper covers for the Supersteps panel's completed-row figures. |
| overflow E2 | `#visits-list { max-height:400px; overflow-y:auto; }`. |
| zero-one-many E2 | `pluralize(n, "time")` in `renderVisits` -- `"1 time"` vs `"N times"`; zero falls to the empty row. |
| long-text E2 | `.mono { overflow-wrap: anywhere; }` applied via `nodeSpan()` on every node id. |
| empty E3 | `renderEdges` shows `#edges-empty` (`"No edges evaluated yet."`) when no superstep carries a fired or evaluated edge; proven by `dev_ui_page_embeds_supersteps_with_no_fired_edges`. |
| loading E3 | Same synchronous render as E2 -- no loading state. |
| error E3 | Shares `renderParseFailure`'s banner via `#edges-list`. |
| populated E3 | `renderEdges` groups by superstep, fired edges first then evaluated-not-fired, using the locked `"superstep N: a → b fired; ..."` shape; proven by the fired-edge assertions in `dev_ui_page_embeds_the_fired_branch_and_the_three_visit_summary`. |
| partial E3 | `renderEdges` appends `COPY.evaluatedUnavailableSuffix` when `view.source === "Waypoints"` and a group's `evaluated_edges` is empty; proven at the payload level by `dev_ui_page_marks_waypoints_source_for_the_evaluated_unavailable_case` (source, fired_edges and empty evaluated_edges all asserted). |
| overflow E3 | `#edges-list { max-height:400px; overflow-y:auto; }`. |
| zero-one-many E3 | `renderEdges` iterates every `fired_edges`/`evaluated_edges` pair individually -- no collapse/`+N more` logic exists. |
| long-text E3 | `.mono { overflow-wrap: anywhere; }` on `nodeSpan()`; `.edge-arrow { white-space: nowrap; }` keeps the arrow glyph attached to its target id. |
| empty E4 | `renderEmptyOrPanels` hides `#superstep-panel` and shows `#empty-state` under the same `supersteps.length === 0` condition as E1; proven by `dev_ui_page_embeds_empty_supersteps_for_a_thread_with_no_history`. |
| loading E4 | Same synchronous render as E2/E3 -- no loading state. |
| error E4 | `renderParseFailure` inserts a colspan-4 row into `#superstep-tbody` carrying the shared banner; page-level 404/501 never reach the template at all (`ApiError` JSON envelope short-circuits before any HTML is served). |
| populated E4 | `renderSupersteps` builds one row per `SuperstepRow`: number, vanguard (mono), completed nodes as outcome-badge + id pairs, field-change NAMES only (`row.field_changes.join(", ")`); the last row gets `.latest-superstep` (accent left border). |
| partial E4 | `renderSupersteps` renders `COPY.noFieldChanges` (`"no field changes"`) for an empty `field_changes`, and `COPY.awaitingInput` (`"— (awaiting input)"`) for an empty `completed` list when `status === "AwaitingInput"`; both proven at the payload level by `dev_ui_page_embeds_awaiting_input_superstep_with_empty_completed`. |
| overflow E4 | `.superstep-table-wrapper { max-height:600px; overflow-y:auto; overflow-x:auto; }` with `thead th { position: sticky; top: 0; }`; no pagination control anywhere in the template. |
| zero-one-many E4 | Zero -> `#empty-state`; one row -> no pagination chrome (none exists); many -> the same scrolling body as `overflow E4`. |
| long-text E4 | `.mono` on the vanguard and field-changes cells; table cells have no `white-space: nowrap`/`text-overflow: ellipsis` anywhere, so they wrap rather than clip. |

## Task Commits

Each task was committed atomically:

1. **Task 1: One thread, one page — the feature, the route, the template and the smoke test** (tracer) - `8454dbeb` (feat)
2. **Task 2: The four panels — every state the UI contract names** (auto) - `e2407427` (feat)

**Plan metadata:** (this commit, following this SUMMARY)

**Tracer feedback gate:** this is a worktree-isolated, wave-parallel executor with no human in the loop for this wave (`workflow._auto_chain_active: false`, `workflow.auto_advance` unset in `.planning/config.json`, but no interactive checkpoint is reachable from inside a spawned worktree agent). Per the autonomous branch of the tracer feedback gate, the tracer's own `<verify>` (`cargo test -p paladin-web --features dev-ui --lib dev_ui_controller` + `git diff --exit-code crates/paladin-web/openapi.json`) was re-run immediately after committing Task 1 -- both green -- before proceeding to Task 2's panel expansion.

## Files Created/Modified

- `crates/paladin-web/Cargo.toml` - New `[features]` section: `default = []`, `dev-ui = []`
- `crates/paladin-web/src/dev_ui_controller.rs` - New: `DevUiState`, `dev_ui_inspector_page`, `escape_for_script`, `map_inspector_error`; 16 tests
- `crates/paladin-web/src/dev_ui/inspector.html` - New: the four-panel static template + inline renderer (`include_str!`-ed)
- `crates/paladin-web/src/app.rs` - New `create_dev_ui_router`, admin-gated via the pre-existing `require_auth`/`require_admin` pair
- `crates/paladin-web/src/lib.rs` - `#[cfg(feature = "dev-ui")] pub mod dev_ui_controller;`
- `Cargo.toml` - Umbrella `dev-ui = ["paladin-web/dev-ui"]`, absent from `default`/`full`
- `.github/workflows/feature-flags.yml` - New `dev-ui` matrix leg

## Decisions Made

See `key-decisions` in frontmatter. The most consequential: admin gating reuses `app.rs`'s pre-existing `require_auth`/`require_admin` (`AuthPort`/`AuthClaims`) pair rather than the `agent_auth` Principal-based system the thread/run/schedule controllers use for their own admin-gated routes. Both are legitimate, pre-existing RBAC layers in this crate; `RunInspectorPort`'s own module docs (28-14) name the dev-ui route's authorization boundary as exactly `require_auth` + `require_admin` by those identifiers, and the plan's acceptance criteria grep for `require_admin` inside `app.rs` specifically, confirming this was the intended precedent to reuse rather than the agent_auth alternative.

## Deviations from Plan

None - plan executed exactly as written. Production wiring of a real `RunInspectorPort` backend into `src/bin/paladin-server.rs` was correctly out of scope: the plan's `files_modified` list names only `crates/paladin-web/*`, root `Cargo.toml`, and `.github/workflows/feature-flags.yml` -- `paladin-server.rs` wiring belongs to a future plan (not this wave), and this plan's own acceptance criteria (an in-test `RunInspectorPort`, `cargo test -p paladin-web --features dev-ui --lib dev_ui_controller`) never require a live server binary.

## Issues Encountered

- `cargo fmt --all` reformatted two long test-fixture lines/import lists inside `dev_ui_controller.rs` after each task's initial write; both were re-verified green (`cargo fmt --all --check`, then a re-run of the full test suite) before committing. No functional change.
- An early draft of the module doc comments in `dev_ui_controller.rs` and the tests both referenced the literal string `"utoipa"` while explaining why the route carries no OpenAPI annotation -- this tripped the plan's own acceptance criterion (`grep -c 'utoipa' crates/paladin-web/src/dev_ui_controller.rs` must be `0`). Reworded to describe the OpenAPI-aware router type without naming the crate; re-verified `0` before committing Task 1.

## User Setup Required

None - no external service configuration required. Air-gapped operators configure `web_server.dev_ui.mermaid_url` (28-02, already shipped) to point at a local Mermaid mirror; no action needed from this plan.

## Next Phase Readiness

- The `dev-ui` feature, route, template and admin gating are complete and self-contained within `crates/paladin-web` -- ready for a future plan to wire a real `RunInspectorPort` into `src/bin/paladin-server.rs` (following the `RunEventStreamPort` precedent in `src/infrastructure/web/run_api_wiring.rs`) whenever that becomes in-scope.
- `openapi.json` is unaffected regardless of feature state -- generated SDK clients need no change for this plan.
- 28-16 (integration test helpers/evals, parallel worktree) and 28-17 (docs/MIGRATION.md) were not touched by this plan and remain independently mergeable.
- No blockers.

## Self-Check: PASSED

- FOUND: crates/paladin-web/src/dev_ui_controller.rs
- FOUND: crates/paladin-web/src/dev_ui/inspector.html
- FOUND: commit 8454dbeb
- FOUND: commit e2407427

## Verification Commands Run (all green)

- `cargo test -p paladin-web --features dev-ui --lib dev_ui_controller` -- 16 passed
- `cargo test -p paladin-web --features dev-ui --lib openapi_matches_committed_baseline` -- 1 passed
- `git diff --exit-code crates/paladin-web/openapi.json` -- clean
- `cargo build -p paladin-web` (default features) -- controller compiled out
- `cargo fmt --all --check` -- exit 0
- `cargo check -p paladin-web --all-targets` (feature off) -- exit 0
- `cargo clippy -p paladin-web --all-targets --features dev-ui -- -D warnings` -- exit 0
- `cargo clippy -p paladin-web --all-targets -- -D warnings` (feature off) -- exit 0
- `cargo clippy -p paladin-ai --all-targets --features web-server,dev-ui -- -D warnings` -- exit 0
- `cargo tree -p paladin-web --no-default-features -e normal | grep -c -E "paladin-battalion|paladin-llm|paladin-storage|paladin-ai "` -- 0 (ADR-0031 upheld)
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo test -p paladin-web --lib` (feature off) -- 222 passed, including `openapi_matches_committed_baseline` unchanged
- `cargo test --features web-server --test e2e_platform_api` -- 1 passed (unaffected by this plan)
- Locked copy strings (all 8) present across `inspector.html`/`dev_ui_controller.rs`
- All five outcome classes (`outcome-success`, `outcome-failed`, `outcome-parleyed`, `outcome-skipped`, `outcome-cache_hit`) present in `inspector.html`
- `prefers-color-scheme` -- 2 occurrences; `overflow-wrap` -- 2; `max-height` -- 4; `fetch(` -- 0
- `grep -c 'utoipa' crates/paladin-web/src/dev_ui_controller.rs` -- 0
- `grep -c '^\[features\]' crates/paladin-web/Cargo.toml` -- 1
- `grep -c 'require_admin' crates/paladin-web/src/app.rs` -- 3

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
