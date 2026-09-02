---
phase: 22-battlefield-state-superstep-engine
plan: 17
subsystem: infra
tags: [checkpoint, ci, postgres, msrv, waypoint, gap-closure]

# Dependency graph
requires:
  - phase: 22-battlefield-state-superstep-engine
    provides: "the postgres-integration CI job (22-12), the prune_thread primitive and retention rewrite (22-13/22-14), and the reachability validation + fixture audit (22-15/22-16)"
provides:
  - "the checkpoint record closing gaps G-22-1, G-22-2 and G-22-3: named CI run evidence, the readiness defect's recorded disposition, and the retention design confirmation"
affects: [22.1-engine-readiness-defect-and-msrv-follow-up]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "checkpoint closes on execution evidence (a named CI run with specific log facts), not on artifact existence"

key-files:
  created: []
  modified:
    - crates/paladin-storage/src/waypoint/postgres.rs
    - .github/workflows/ci.yml
    - .project/current-exports.txt
    - CHANGELOG.md
    - Cargo.lock

key-decisions:
  - "Readiness defect disposition (developer decision, 2026-09-02): registered and scheduled to inserted Phase 22.1 — all gaps not closable in-repo route there"
  - "Retention design confirmed as intended: protected set computed once in application-layer WaypointRetentionService, passed into the storage routine as an argument"
  - "MSRV residual routed to Phase 22.1: rmcp =2.1.0 (pinned per Phase 12.1) needs process-wrap ^9.0, whose every version requires rustc >= 1.86 against the declared 1.85 floor — raise the MSRV or move the pin; not decidable at this checkpoint"

patterns-established: []

# Metrics
duration-minutes: 180
completed: 2026-09-02
---

# Phase 22 Plan 17: Gap-closure checkpoint — CI evidence and dispositions

**Three gaps closed on the evidence each demanded: a named green CI run for the suite that had never executed, in-repo tests for the two testable fixes, and recorded developer decisions for the questions that were not an executor's to answer.**

## The CI evidence (G-22-1)

The checkpoint's first run surfaced real defects; the loop closed on the third:

- **Run 33672088907** (commit `3b111d4e`, PR #52) — the first-ever execution of the
  Postgres Tier 2 contract suite anywhere. Reachability passed, no early-return marker,
  25/26 tests passed against the live server; the one failure
  (`list_threads_empty_then_three_threads_newest_activity_first`, contract_tests.rs:193)
  was a genuine harness defect — the shared CI database carries residue between tests,
  which SQLite/in-memory never exposed because they construct a fresh store per test.
  The failure is itself the strongest proof the suite genuinely ran.
- **Run 33685990248** (commit `cffae0fc`) — after the `store_or_skip` truncation fix,
  all 26 tests passed against the live server; the job stayed red only because the
  declared-count assertion's unanchored grep counted a `#[tokio::test]` literal inside
  a comment (27 "declared" vs 26 real).
- **Run 33688238662** (commit `2b2bc1d5`, job 100440861780) — **green**. All four
  required log facts confirmed: the container reported healthy (anchored `-qx` match);
  `pg_isready` printed "accepting connections"; `test result: ok. 26 passed; 0 failed`
  with the passed-count equal to the module's 26 declared `#[tokio::test]` functions;
  and the skip-detection step printed "All waypoint::postgres tests exercised the live
  server."

G-22-1 is closed by an execution, not by the presence of a job definition.

## The readiness defect's disposition (from the 22-16 audit)

A node that is both self-looping and fed by an upstream edge can never take its first
turn (`Frontier::is_ready` requires every incoming edge resolved; a self-edge is
unresolved until the node has run) — and the run still reports `Completed`. Same
truthful-outcome class as BUG-02, different mechanism; reachability validation cannot
catch it because the node is statically reachable. Reproduction stays on demand:
`cargo test -p paladin-battalion --lib engine::superstep -- --ignored --nocapture`.

**Disposition (decided by the developer at this checkpoint): registered and scheduled
to Phase 22.1** (`.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/`,
inserted after Phase 22 in ROADMAP.md). Entry into the program defect register
(`.project/v0.10.0/00-program-overview.md` §7) happens when 22.1 is planned — nothing
under `.project/` was edited by this checkpoint beyond what plan 22-14 already owned.

## The retention design confirmation (G-22-2)

**Approved as implemented**: the routine stays in the storage module and takes the
protected set as an argument; the application-layer `WaypointRetentionService` holds
the single definition of protected (thread's latest + every AwaitingInput Waypoint),
with Parley-referenced and fork-lineage-pinned classes named as seams for their owning
phases.

## Checkpoint-driven fixes applied (commits on `feature/phase-22`)

| Commit | What |
|--------|------|
| `69bed986` | `store_or_skip` truncates the shared `paladin_waypoint_test` database per test — Postgres now has the same logically-fresh-store semantics the other two backends get by construction |
| `0b8e5b5a` | semver set-equality step tolerates an empty register/allowlist (a no-match grep under `set -euo pipefail` killed the step on its first-ever run) — job now **green** (run 33685990248) |
| `a9d5c278` | API-surface baseline regenerated for 22-14's additive retention API (+29 items, 2000 total) + CHANGELOG Unreleased entry — job now **green** |
| `44e13fbd` | 19 MSRV-incompatible transitive deps pinned back (darling 0.21.3, home 0.5.11, icu 2.1.x, serde_with 3.17, time 0.3.45, tonic 0.14.5, …); workspace builds, all 1919 tests pass |
| `2b2bc1d5` | declared-test count grep anchored to attribute position |

## Residual, routed to Phase 22.1 (not closed here)

- **MSRV**: the `msrv` job still fails, now on exactly one package — `process-wrap`
  (rustc ≥ 1.86/1.87 in every version satisfying rmcp's `^9.0`), pulled by the
  deliberately pinned `rmcp = "=2.1.0"` via its shipped `transport-child-process`
  feature (the Arsenal STDIO adapter). Raising the declared 1.85 floor (MIGRATION.md
  §9.3 / X-11.1 D-07) or moving the Phase 12.1 pin is 22.1's decision.
- **Readiness defect**: fix and register entry, per the disposition above.

## Self-Check: PASSED

- Run 33688238662's postgres-integration job is `completed/success` (verified via API).
- All four log facts extracted from job 100440861780's log, quoted above.
- `.planning/phases/22-battlefield-state-superstep-engine/22-UAT.md` gaps G-22-1/2/3
  updated; Phase 22.1 exists in ROADMAP.md with the residuals as its goal.
