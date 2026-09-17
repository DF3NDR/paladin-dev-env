---
phase: 35-mdbook-currency
plan: 05
subsystem: docs
tags: [mdbook, adr, crate-map, feature-flags, migration-guide, stable-api]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: plan 35-01's superstep-engine.md nav entry (this plan's SUMMARY.md sits above it in the Contributing section, not adjacent — no ordering conflict)
provides:
  - "docs/src/contributing/adr-index.md — a new ADR index page listing the nine consumer/operator-visible ADRs"
  - "docs/src/SUMMARY.md retitle: Architecture Decisions -> Adapter Development Guide for the existing page, plus the new Architecture Decisions entry for adr-index.md"
  - "Both crate-map pages (architecture/crate-map.md, api-reference/crate-map.md) corrected to eleven library crates plus the facade, 0.10.0 pins, and the mem --> llm mermaid edge"
  - "api-reference/feature-flags.md regenerated from Cargo.toml: otel, dev-ui, redis-cache, storage-postgres added; storage aggregate corrected; Dockerfile base image and every version pin updated to 0.10.0/rust:1.93-slim-bookworm"
  - "api-reference/migration-guide.md's opening line and Timeline table now name v0.10.0 as current"
  - "api-reference/stable-api.md rerooted onto paladin_core::platform::container:: and paladin_ports::output:: live crate paths, version/footer corrected to 0.10.0, Public crates list extended with paladin-eval and paladin-herald"
affects: [35-10]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ADR index page pattern: GitHub blob URLs (not relative .planning/ links) so linkcheck's follow-web-links=false setting skips fetching them"
    - "D-24 grouped commits: tightly-coupled page pairs (the two crate-map pages) land in one commit"

key-files:
  created:
    - docs/src/contributing/adr-index.md
  modified:
    - docs/src/SUMMARY.md
    - docs/src/contributing/architecture-decisions.md
    - docs/src/architecture/crate-map.md
    - docs/src/api-reference/crate-map.md
    - docs/src/api-reference/feature-flags.md
    - docs/src/api-reference/migration-guide.md
    - docs/src/api-reference/stable-api.md

key-decisions:
  - "The api-reference/crate-map.md page's own paladin-llm feature table was left unextended (kimi/qwen/grok/ollama/openai-compatible/gemini) — the plan scoped that extension to architecture/crate-map.md only; api-reference/crate-map.md instead got the mem --> llm mermaid edge, per the plan's explicit per-page split"
  - "stable-api.md's Rust-path fix was scoped to the two roots the plan named (paladin_core::platform::container:: and paladin_ports::output::) — Builder/Error/Config catalogue rows under the facade's own application::services/config paths were left untouched since neither the plan's read_first nor its acceptance criteria named them, and CommanderBuilder/CouncilBuilder/GroveBuilder have since moved to paladin-battalion/paladin-core, a deeper drift outside this plan's stated scope"
  - "Added an illustrative-fragment header note to stable-api.md per D-11(b) since the page lacked one; no bare \\`\\`\\`rust fence existed to begin with (all were already \\`\\`\\`rust,ignore)"
  - "Removed stable-api.md's stale forward-looking 'Milestone 7 target: 0.2.0 lockstep' line when correcting the baseline version line, since the milestone has long since passed and surpassed that target"

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "MB-35: adr-index.md created with nine ADR rows (blob-URL Record column), architecture-decisions.md retitled to Adapter Development Guide in nav, adr-index.md inserted directly after it"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "task 1 <verify> automated block (adr-index.md existence, SUMMARY.md grep checks, ADR-number grep loop, mdbook build) — see task commit 745e8a64"
        status: pass
    human_judgment: false
  - id: D2
    description: "MB-13/MB-14: both crate-map pages corrected to eleven library crates plus facade (paladin-eval, paladin-herald named), 0.10.0 pins, paladin-llm feature table extended on architecture/crate-map.md, mem --> llm mermaid edge added on api-reference/crate-map.md"
    requirement: "CURR-07"
    verification:
      - kind: other
        ref: "task 2 <verify> automated block (mem-->llm grep, version-pin regex, mdbook build, check-doc-config.sh) — see task commit bc2a50d1"
        status: pass
    human_judgment: false
  - id: D3
    description: "MB-15: feature-flags.md regenerated from Cargo.toml — otel, dev-ui, redis-cache, storage-postgres added; Dockerfile pin corrected to rust:1.93-slim-bookworm; relocated LLM-adapter import fixed to paladin_llm::openai::OpenAIAdapter"
    requirement: "CURR-07"
    verification:
      - kind: other
        ref: "task 2 <verify> automated block (per-flag grep loop, Dockerfile-pin grep, stale-import grep) — see task commit 10dba3e2"
        status: pass
    human_judgment: false
  - id: D4
    description: "MB-16: migration-guide.md's opening sentence and Timeline table now mark v0.10.0 as the current release; upgrading.md left untouched per D-00f"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "task 3 <verify> automated block (v0.10.0 grep, stale-claim negative grep, upgrading.md diff --quiet) — see task commit cb824ee2"
        status: pass
    human_judgment: false
  - id: D5
    description: "MB-17: stable-api.md rerooted onto paladin_core::platform::container:: and paladin_ports::output:: paths, version/footer at 0.10.0, Public crates list carries paladin-eval and paladin-herald, no bare \\`\\`\\`rust fences"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "task 3 <verify> automated block (path-root grep pair, eval/herald grep, upgrading.md diff --quiet, mdbook build) — see task commit f2b25fd3"
        status: pass
    human_judgment: false

# Metrics
duration: 15min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 05: Close the API Reference and Contributing rows Summary

**Regenerated the crate-map, feature-flag, migration-guide and stable-API reference pages from the
live Cargo.toml/crate tree, and added a new GitHub-blob-linked ADR index page under Contributing.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-09-17T13:58:00Z
- **Completed:** 2026-09-17T14:13:00Z
- **Tasks:** 3
- **Files modified:** 8 (1 created, 7 modified)

## Accomplishments
- Closed MB-35 (retitle-and-add): `contributing/architecture-decisions.md` renamed to "Adapter Development Guide" in the nav, and a new `contributing/adr-index.md` page indexes nine consumer/operator-visible ADRs (0033, 0037, 0039, 0042, 0047, 0048, 0049, 0050, 0051) via GitHub blob URLs
- Closed MB-13/MB-14 (crate-map pair): both crate-map pages now name eleven library crates plus the facade (adding `paladin-eval`, `paladin-herald`), every version claim reads 0.10.0, `architecture/crate-map.md`'s `paladin-llm` feature table gained the six Phase 17 provider flags, and `api-reference/crate-map.md`'s mermaid graph gained the `mem --> llm` edge so diagram and prose agree
- Closed MB-15: `api-reference/feature-flags.md` regenerated from the facade and per-crate `Cargo.toml` `[features]` blocks — added `otel`, `dev-ui`, `redis-cache`, `storage-postgres`; corrected the `storage` aggregate (storage-mysql + storage-postgres, not storage-sqlite); fixed the Dockerfile excerpt to `rust:1.93-slim-bookworm`; fixed the relocated LLM-adapter import to `paladin_llm::openai::OpenAIAdapter`; every version pin now reads 0.10.0
- Closed MB-16: `migration-guide.md`'s opening sentence and Timeline table now name v0.10.0 as the current release, without duplicating `MIGRATION.md`/`upgrading.md` content
- Closed MB-17: `stable-api.md`'s catalogue paths rerooted from the pre-workspace `paladin::core::…`/`paladin::application::…` layout onto the live `paladin_core::platform::container::…` and `paladin_ports::output::…` crate paths; Version/footer corrected to 0.10.0; Public crates list extended with `paladin-eval` and `paladin-herald`; added the D-11(b) illustrative-fragment note

## Task Commits

Each task was committed atomically:

1. **Task 1: MB-35 — retitle the adapter guide and add the ADR index page end to end** - `745e8a64` (docs)
2. **Task 2: MB-13, MB-14 and MB-15 — the crate-map pair and the feature-flags page** - `bc2a50d1` (docs, crate-map pair) + `10dba3e2` (docs, feature-flags)
3. **Task 3: MB-16 and MB-17 — migration-guide.md and stable-api.md** - `cb824ee2` (docs, migration-guide) + `f2b25fd3` (docs, stable-api)

_Note: Task 2 and Task 3 each produced two commits per the plan's explicit per-page split._

## Files Created/Modified
- `docs/src/contributing/adr-index.md` - New ADR index page, nine rows, GitHub blob-URL Record column
- `docs/src/SUMMARY.md` - Retitled the Contributing entry to "Adapter Development Guide", inserted the new "Architecture Decisions" entry directly after it
- `docs/src/contributing/architecture-decisions.md` - Added a one-line pointer to the new ADR index page
- `docs/src/architecture/crate-map.md` - Crate count/inventory fix (eleven + facade), 0.10.0 pins, paladin-llm feature table extended with six Phase 17 flags
- `docs/src/api-reference/crate-map.md` - Crate table extended with paladin-eval/paladin-herald, 0.10.0 pins throughout, mem --> llm mermaid edge added
- `docs/src/api-reference/feature-flags.md` - Storage & Queue table corrected, new Observability & Admin Flags section (otel, dev-ui), full aggregate updated, all version pins to 0.10.0, Dockerfile pin fixed, relocated-adapter import fixed
- `docs/src/api-reference/migration-guide.md` - Opening sentence and Timeline table corrected to v0.10.0 current
- `docs/src/api-reference/stable-api.md` - Catalogue paths rerooted onto paladin_core/paladin_ports, version/footer corrected, Public crates list extended, illustrative-fragment note added

## Decisions Made
- Scoped the `paladin-llm` feature-table extension to `architecture/crate-map.md` only (not `api-reference/crate-map.md`), and the `mem --> llm` mermaid-edge addition to `api-reference/crate-map.md` only (it already existed on `architecture/crate-map.md`) — this matches the plan's explicit per-page instruction split rather than duplicating both changes on both pages
- Scoped `stable-api.md`'s path rewrite to the two roots the plan named (`paladin_core::platform::container::` and `paladin_ports::output::`); left Builder/Error/Config catalogue rows under the facade's own `application::services`/`config` paths untouched, since `CommanderBuilder`, `CouncilBuilder` and `GroveBuilder` have since moved to `paladin-battalion`/`paladin-core` respectively — tracing and correcting every one of those moves is a deeper drift than this plan's read_first or acceptance criteria called for
- Removed `stable-api.md`'s stale "Milestone 7 target: 0.2.0 lockstep" forward-looking line when correcting the baseline version, since that milestone has long since passed

## Deviations from Plan

None - plan executed exactly as written. All required corrections (crate counts, version pins, ADR index, feature-flag regeneration, mermaid edge, migration-guide Timeline, stable-API path reroot) were completed per the task actions; no Rule 1-4 auto-fixes were needed beyond the plan's own instructions.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Deferred observations

None observed. This plan touched no page the audit had settled `current`; all seven touched pages (`SUMMARY.md`, `architecture-decisions.md`, both crate-map pages, `feature-flags.md`, `migration-guide.md`, `stable-api.md`) were audit-flagged `stale` rows (MB-35, MB-13, MB-14, MB-15, MB-16, MB-17) that this plan closed.

## Next Phase Readiness
- All six MB rows this plan owned (MB-35, MB-13, MB-14, MB-15, MB-16, MB-17) are closed
- `docs/src/api-reference/upgrading.md` remains untouched per D-00f, as required
- No `src/` or `crates/` files were touched (D-00c honored)
- Ready for plan 35-10 to fold this SUMMARY's (empty) deferred-observations set into `deferred-items.md`, and for the phase-level `docs.yml` sequence to run on the merged tree

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- `docs/src/contributing/adr-index.md` — FOUND
- `.planning/phases/35-mdbook-currency/35-05-SUMMARY.md` — FOUND
- Commit `745e8a64` (MB-35) — FOUND
- Commit `bc2a50d1` (MB-13, MB-14) — FOUND
- Commit `10dba3e2` (MB-15) — FOUND
- Commit `cb824ee2` (MB-16) — FOUND
- Commit `f2b25fd3` (MB-17) — FOUND
