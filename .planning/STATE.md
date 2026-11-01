---
gsd_state_version: 1.0
milestone: v0.11.1
milestone_name: Dogfood Cleanup (Phases 52-58) — in progress
current_phase: 55
current_phase_name: Correctness Fixes, Test Hygiene & Lint Bar
status: executing
stopped_at: Completed 55-10-PLAN.md
last_updated: "2026-11-01T17:34:24.028Z"
last_activity: 2026-11-01
last_activity_desc: Phase 55 execution started
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 38
  completed_plans: 35
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-11-01 after Phase 54)

**Core value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.
**Current focus:** Phase 55 — Correctness Fixes, Test Hygiene & Lint Bar
52-UAT 3/3 passed; PR #61 merged to `main` as `84686acfb` on 2026-10-30)
(carried-gaps inventory, dogfood findings log, scoping notes): 7 phases (52-58), 53/53 requirements
mapped. Phase 52 (CI & Release Pipeline Repair) completed 2026-10-30; Phase 53 (Tenant Isolation Closure) completed 2026-10-31; Phase 54 (First-Run Usability) completed 2026-11-01; next: Phase 55. Decisions at open: OpenAI Responses API out,
`${VAR}` auth keys refused at boot, research skipped.

**Previous milestone:** v0.11.0 "Treasurer Spend Governance" closed 2026-10-27 — 15 phases
(38-51, with 50.1 inserted and 48-51 added mid-milestone from a code survey and three web3sec
handoffs), 174 plans, 281 tasks, 63/66 requirements (ADOPT-01…03 superseded at source by the
owner's 2026-10-14 scope cut), 34 days (2026-09-24 → 2026-10-27; 1,126 commits on the release
PR #59 per GitHub, `df24689c` → `70657919` → `a49200da`). Archived to `milestones/v0.11.0-ROADMAP.md`,
`v0.11.0-REQUIREMENTS.md`, `v0.11.0-MILESTONE-AUDIT.md` (status `tech_debt`, 0 gaps, 8/8 seams,
8/8 flows, 15/15 Nyquist-compliant) and `v0.11.0-phases/`. Released on one tag: `v0.11.0` on
merge commit `70657919` (2026-10-26; run `40396108246`, all 12 publishable crates on crates.io at
`0.11.0` through Trusted Publishing; `Coverage` 92.74 %). Closeout type `override_closeout` — see
*Deferred Items*. Full record: MILESTONES.md.

**Prior:** v0.10.0 "Durable Agent Execution Runtime" closed 2026-09-23 (19 phases, 231 plans,
88/89 requirements, 1,678 commits; released as `v0.10.1` on `f7dae267` after the `v0.10.0` tag
published 3 of 12 crates); v0.9.0 "Security Tooling" shipped 2026-09-01 (4 phases, 25 plans,
20/20; tag `v0.9.0` on `0b5d4106`); v0.8.0 shipped 2026-08-24 (14 phases, 149 plans, 65/65);
v0.7.1 shipped 2026-08-04 (4 phases, 38 plans, 25/25). All archived under `milestones/`.

## Current Position

Phase: 55 (Correctness Fixes, Test Hygiene & Lint Bar) — EXECUTING
Plan: 11 of 13
Status: Ready to execute
Last activity: 2026-11-01 — Phase 55 execution started

Progress: [█████████░░░░░░░░░░░] 3/7 phases ([█████████░] 92%) — Phase 54 plans [████████████████████] 9/9 (100%)

## Performance Metrics

**Velocity:**

- Total plans completed: 614
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 11 | - | - |
| 02 | 11 | - | - |
| 3 | 8 | - | - |
| 05 | 13 | - | - |
| 06 | 10 | - | - |
| 07 | 13 | - | - |
| 08 | 9 | - | - |
| 10 | 11 | - | - |
| 11 | 5 | - | - |
| 12 | 4 | - | - |
| 13 | 13 | - | - |
| 14 | 8 | - | - |
| 15 | 10 | - | - |
| 17 | 22 | - | - |
| 16 | 14 | - | - |
| 18 | 7 | - | - |
| 19 | 5 | - | - |
| 20 | 7 | - | - |
| 21 | 6 | - | - |
| 22 | 17 | - | - |
| 22.1 | 7 | - | - |
| 23 | 12 | - | - |
| 24 | 14 | - | - |
| 25 | 14 | - | - |
| 26 | 21 | - | - |
| 27 | 26 | - | - |
| 28 | 17 | - | - |
| 29 | 9 | - | - |
| 30 | 3 | - | - |
| 31 | 7 | - | - |
| 32 | 5 | - | - |
| 33 | 6 | - | - |
| 34 | 9 | - | - |
| 35 | 10 | - | - |
| 36 | 13 | - | - |
| 36.1 | 14 | - | - |
| 37 | 8 | - | - |
| 37.1 | 16 | - | - |
| 38 | 9 | - | - |
| 39 | 8 | - | - |
| 45 | 7 | - | - |
| 40 | 6 | - | - |
| 41 | 9 | - | - |
| 42 | 12 | - | - |
| 43 | 13 | - | - |
| 44 | 13 | - | - |
| 48 | 9 | - | - |
| 49 | 3 | - | - |
| 50 | 32 | - | - |
| 50.1 | 9 | - | - |
| 51 | 20 | - | - |
| 46 | 14 | - | - |
| 47 | 10 | - | - |
| 52 | 8 | - | - |
| 53 | 8 | - | - |
| 54 | 9 | - | - |

*Updated after each plan completion*

**Recent Trend:**

- Last 5 plans: —
- Trend: —

**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P09 | ~27min + human review cycle | 3 tasks | 2 files |
| Phase 01 P10 | 20min | 2 tasks | 1 files |
| Phase 01 P12 | 40min | 4 tasks | 4 files |
| Phase 03 P01 | 16min | 2 tasks | 1 files |
| Phase 3 P2 | 25min | 2 tasks | 5 files |
| Phase 03 P03 | ~15min | 3 tasks | 2 files |
| Phase 03 P05 | ~25min | 2 tasks | 1 files |
| Phase 03 P06 | ~20min | 2 tasks | 3 files |
| Phase 03 P04 | 35min | 3 tasks | 3 files |
| Phase 03 P07 | 14min | 2 tasks | 2 files |
| Phase 03 P08 | ~30min | 3 tasks | 4 files |
| Phase 06 P07 | ~50min | 3 tasks | 4 files |
| Phase 12 P02 | ~20min | 2 tasks | 3 files |
| Phase 12 P03 | ~20min | 2 tasks | 3 files |
| Phase 12 P04 | ~25min | 2 tasks | 3 files |
| Phase 20 P07 | 35min | 3 tasks | 3 files |
| Phase 34 P01 | 17min | 2 tasks | 9 files |
| Phase 34 P02 | 15min | 1 tasks | 3 files |
| Phase 34 P03 | 18min | 2 tasks | 3 files |
| Phase 34 P04 | 28min | 2 tasks | 3 files |
| Phase 34 P05 | ~60min | 2 tasks | 2 files |
| Phase 34 P06 | ~18min | 2 tasks | 5 files |
| Phase 34 P07 | 16min | 2 tasks | 16 files |
| Phase 34 P08 | ~90min | 2 tasks | 3 files |
| Phase 34 P09 | ~75min | 2 tasks | 4 files |
| Phase 36 P01 | ~45min | 2 tasks | 9 files |
| Phase 36 P02 | ~55min | 3 tasks | 10 files |
| Phase 36 P03 | ~20min | 2 tasks | 5 files |
| Phase 36 P04 | ~25min | 2 tasks | 8 files |
| Phase 36 P05 | 35min | 2 tasks | 8 files |
| Phase 36 P06 | 30min | 2 tasks | 3 files |
| Phase 36 P07 | ~55min | 2 tasks | 3 files |
| Phase 36 P08 | ~2h10min | 3 tasks | 4 files |
| Phase 36 P09 | ~1h30min | 3 tasks | 6 files |
| Phase 36 P10 | ~1h50min | 3 tasks | 6 files |
| Phase 36 P11 | ~1h10min | 3 tasks | 2 files |
| Phase 36 P12 | ~1h | 2 tasks | 8 files |
| Phase 36 P13 | 25min | 3 tasks | 4 files |
| Phase 36.1 P01 | 20min | 2 tasks | 3 files |
| Phase 36.1 P02 | 15min | 2 tasks | 2 files |
| Phase 36.1 P03 | 20min | 3 tasks | 4 files |
| Phase 36.1 P04 | ~35min | 3 tasks | 9 files |
| Phase 36.1 P05 | ~15min | 3 tasks | 7 files |
| Phase 36.1 P06 | 6min | 3 tasks | 4 files |
| Phase 36.1 P07 | ~15min | 3 tasks | 4 files |
| Phase 36.1 P08 | ~20min | 2 tasks | 3 files |
| Phase 36.1 P09 | ~35min | 2 tasks | 3 files |
| Phase 36.1 P10 | ~15min | 2 tasks | 3 files |
| Phase 36.1 P11 | ~11min | 2 tasks | 7 files |
| Phase 36.1 P12 | ~20min | 3 tasks | 6 files |
| Phase 36.1 P13 | ~28min | 3 tasks | 3 files |
| Phase 36.1 P14 | ~20min | 2 tasks | 1 files |
| Phase 37 P01 | 35min | 3 tasks | 2 files |
| Phase 37 P02 | 50min | 1 tasks | 2 files |
| Phase 37 P03 | 53min | 2 tasks | 1 files |
| Phase 37 P04 | 7min | 2 tasks | 5 files |
| Phase 37 P05 | 14min | 2 tasks | 2 files |
| Phase 37 P07 | 25min | 2 tasks | 1 files |
| Phase 37.1 P01 | ~20min | 2 tasks | 2 files |
| Phase 37.1 P02 | ~40min | 3 tasks | 5 files |
| Phase 37.1 P03 | ~25min | 3 tasks | 3 files |
| Phase 37.1 P04 | ~16min | 3 tasks | 2 files |
| Phase 37.1 P05 | ~9min | 3 tasks | 3 files |
| Phase 37.1 P06 | ~28min | 3 tasks | 29 files |
| Phase 37.1 P07 | ~2h30min | 3 tasks | 3 files |
| Phase 37.1 P08 | ~35min | 2 tasks | 3 files |
| Phase 37.1 P09 | ~1h10min | 2 tasks | 2 files |
| Phase 37.1 P10 | ~20min | 2 tasks | 2 files (uncommitted) |
| Phase 38 P01 | ~5min | 2 tasks | 7 files |
| Phase 38 P02 | 57min | 2 tasks | 10 files |
| Phase 38 P03 | 23min | 2 tasks | 10 files |
| Phase 38 P04 | 95min | 2 tasks | 43 files |
| Phase 38 P05 | 9min | 2 tasks | 2 files |
| Phase 38 P06 | ~28min | 2 tasks | 18 files |
| Phase 38 P07 | ~25min | 3 tasks | 8 files |
| Phase 38 P08 | ~26min | 2 tasks | 5 files |
| Phase 38 P09 | ~110min | 2 tasks | 10 files |
| Phase 39 P01 | ~30min | 2 tasks | 11 files |
| Phase 39 P02 | ~30min | 2 tasks | 6 files |
| Phase 39 P03 | ~42min | 2 tasks | 4 files |
| Phase 39 P04 | ~55min | 2 tasks | 4 files |
| Phase 39 P05 | ~50min | 2 tasks | 4 files |
| Phase 39 P06 | ~20min | 2 tasks | 4 files |
| Phase 39 P07 | ~50min | 2 tasks | 4 files |
| Phase 39 P08 | ~55min | 2 tasks | 13 files |
| Phase 40 P40-01 | ~2h (continuation) | 3 tasks | 29 files |
| Phase 40 P02 | ~1h | 2 tasks | 7 files |
| Phase 40 P03 | ~35m | 2 tasks | 3 files |
| Phase 40 P04 | 17 min | 2 tasks | 9 files |
| Phase 40 P05 | 15min | 2 tasks | 5 files |
| Phase 40 P06 | 60min | 3 tasks | 14 files |
| Phase 45 P01 | 55min | 3 tasks | 9 files |
| Phase 45 P02 | 45min | 3 tasks | 7 files |
| Phase 45 P03 | 40min | 3 tasks | 6 files |
| Phase 45 P04 | 95min | 3 tasks | 15 files |
| Phase 45 P05 | 40min | 3 tasks | 16 files |
| Phase 45 P06 | 50min | 3 tasks | 10 files |
| Phase 45 P07 | n/a (human measurement) | 2 tasks | 10 files |
| Phase 41 P01 | 45min | 4 tasks | 28 files |
| Phase 41 P02 | 40min | 2 tasks | 6 files |
| Phase 41 P03 | 75min | 2 tasks | 11 files |
| Phase 41 P04 | 60min | 3 tasks | 12 files |
| Phase 41 P05 | 75min | 3 tasks | 22 files |
| Phase 41 P06 | 75min | 3 tasks | 24 files |
| Phase 41 P07 | 65min | 2 tasks | 17 files |
| Phase 41 P08 | ~2h | 3 tasks | 21 files |
| Phase 41 P09 | 40min | 2 tasks | 12 files |
| Phase 42 P01 | 10min | 2 tasks | 3 files |
| Phase 42 P02 | 58 | 3 tasks | 19 files |
| Phase 42 P03 | 40min | 3 tasks | 20 files |
| Phase 42 P04 | 25min | 2 tasks | 9 files |
| Phase 42 P05 | 40min | 3 tasks | 23 files |
| Phase 42 P06 | 45min | 2 tasks | 14 files |
| Phase 42 P07 | 1h | 3 tasks | 12 files |
| Phase 42 P08 | 45min | 3 tasks | 11 files |
| Phase 42 P09 | 66min | 2 tasks | 16 files |
| Phase 42 P10 | 25min | 3 tasks | 25 files |
| Phase 42 P11 | 35min | 3 tasks | 10 files |
| Phase 42 P12 | 35min | 3 tasks | 18 files |
| Phase 43 P01 | multi-session | 3 tasks | 18 files |
| Phase 43 P02 | ~35min | 3 tasks | 35 files |
| Phase 43 P03 | ~25min | 3 tasks | 9 files |
| Phase 43 P05 | 2h | 3 tasks | 7 files |
| Phase 43 P04 | 1h | 2 tasks | 6 files |
| Phase 43 P06 | 100min | 2 tasks | 7 files |
| Phase 43 P07 | 75min | 2 tasks | 11 files |
| Phase 43 P08 | 1h | 2 tasks | 8 files |
| Phase 43 P09 | 90min | 2 tasks | 11 files |
| Phase 43 P10 | 1h45m | 2 tasks | 14 files |
| Phase 43 P11 | 1 h | 2 tasks | 7 files |
| Phase 43 P12 | 2 sessions | 3 tasks | 3 files |
| Phase 43 P13 | 40min | 2 tasks | 13 files |
| Phase 44 P01 | 2h | 2 tasks | 5 files |
| Phase 44 P02 | ~30min | 2 tasks | 5 files |
| Phase 44 P03 | ~1h30min | 3 tasks | 12 files |
| Phase 44 P04 | 50min | 2 tasks | 6 files |
| Phase 44 P05 | ~1h30min | 2 tasks | 2 files |
| Phase 44 P06 | 1h | 3 tasks | 9 files |
| Phase 44 P07 | 55min | 2 tasks | 3 files |
| Phase 44 P08 | 50min | 3 tasks | 13 files |
| Phase 44 P09 | 35min | 3 tasks | 13 files |
| Phase 44 P10 | 55min | 3 tasks | 17 files |
| Phase 44 P11 | ~1h30min | 2 tasks | 4 files |
| Phase 44 P13 | 35min | 2 tasks | 2 files |
| Phase 44 P12 | ~1h | 2 tasks | 6 files |
| Phase 48 P02 | 20min | 2 tasks | 4 files |
| Phase 48 P01 | split (container restart) | 2 tasks | 7 files |
| Phase 48 P03 | 25min | 2 tasks | 12 files |
| Phase 48 P04 | 1h | 3 tasks | 140 files |
| Phase 48 P05 | 75m | 2 tasks | 110 files |
| Phase 48 P06 | 25m | 2 tasks | 12 files |
| Phase 48 P07 | 30min | 2 tasks | 5 files |
| Phase 48 P08 | 25 min | 2 tasks | 1 files |
| Phase 48 P09 | 1h | 2 tasks | 6 files |
| Phase 49 P01 | 125min | 3 tasks | 4 files |
| Phase 49 P02 | 25min | 2 tasks | 2 files |
| Phase 49 P03 | 25min | 2 tasks | 1 files |
| Phase 50 P01 | 32min | 3 tasks | 10 files |
| Phase 50 P02 | 40min | 3 tasks | 11 files |
| Phase 50 P04 | 55min | 2 tasks | 2 files |
| Phase 50 P03 | ~45min | 3 tasks | 16 files |
| Phase 50 P05 | 20min | 3 tasks | 10 files |
| Phase 50 P17 | 25min | 2 tasks | 2 files |
| Phase 50 P06 | ~1h30m | 3 tasks | 13 files |
| Phase 50 P07 | 40min | 2 tasks | 6 files |
| Phase 50 P08 | 65min | 3 tasks | 23 files |
| Phase 50 P09 | 40min | 3 tasks | 13 files |
| Phase 50 P10 | 45min | 3 tasks | 11 files |
| Phase 50 P11 | 30min | 3 tasks | 46 files |
| Phase 50 P12 | 35min | 3 tasks | 14 files |
| Phase 50 P13 | 30min | 3 tasks | 11 files |
| Phase 50 P14 | 90min | 3 tasks | 11 files |
| Phase 50 P15 | 60min | 2 tasks | 10 files |
| Phase 50 P18 | 65min | 3 tasks | 18 files |
| Phase 50 P16 | 95min | 3 tasks | 25 files |
| Phase 50 P19 | 25min | 3 tasks | 9 files |
| Phase 50 P20 | 40min | 3 tasks | 10 files |
| Phase 50 P21 | 29min | 3 tasks | 10 files |
| Phase 50 P23 | 37min | 2 tasks | 7 files |
| Phase 50 P22 | 10min | 3 tasks | 2 files |
| Phase 50 P24 | 21min | 3 tasks | 11 files |
| Phase 50 P25 | 63min | 2 tasks | 8 files |
| Phase 50 P26 | 227min | 2 tasks | 14 files |
| Phase 50 P27 | 72min | 2 tasks | 8 files |
| Phase 50 P28 | 40min | 2 tasks | 1 files |
| Phase 50 P29 | 58min | 3 tasks | 10 files |
| Phase 50 P30 | 30min | 2 tasks | 7 files |
| Phase 50 P31 | 16min | 3 tasks | 8 files |
| Phase 50 P32 | approx 1h20m | 3 tasks | 10 files |
| Phase 50.1 P01 | 35min | 3 tasks | 26 files |
| Phase 50.1 P02 | 55min | 2 tasks | 19 files |
| Phase 50.1 P03 | 30min | 2 tasks | 5 files |
| Phase 50.1 P04 | 50min | 3 tasks | 13 files |
| Phase 50.1 P05 | 60min | 2 tasks | 24 files |
| Phase 50.1 P06 | 75min | 3 tasks | 21 files |
| Phase 50.1 P07 | 45min | 3 tasks | 2 files |
| Phase 46 P01 | 25min | 3 tasks | 15 files |
| Phase 46 P02 | 20 min | 2 tasks | 2 files |
| Phase 46 P03 | 35 min | 3 tasks | 3 files |
| Phase 46 P04 | ~90 min | 3 tasks | 3 files |
| Phase 46 P05 | ~12 min | 2 tasks | 9 files |
| Phase 46 P06 | 3 min | 2 tasks | 1 files |
| Phase 46 P07 | 26 min | 2 tasks | 5 files |
| Phase 46 P08 | ~35 min | 2 tasks | 9 files |
| Phase 46 P09 | 25 min | 2 tasks | 10 files |
| Phase 46 P10 | 13 min | 2 tasks | 4 files |
| Phase 46 P11 | 20 min | 3 tasks | 1 files |
| Phase 46 P12 | 9 min | 3 tasks | 14 files |
| Phase 46 P14 | 15 min | 3 tasks | 4 files |
| Phase 46 P13 | 17 min | 3 tasks | 7 files |
| Phase 47 P01 | 1h | 3 tasks | 2 files |
| Phase 47 P02 | 2h20m | 3 tasks | 4 files |
| Phase 47 P03 | 50m | 3 tasks | 28 files |
| Phase 47 P04 | 45m (final continuation) | 3 tasks | 3 files |
| Phase 47 P05 | 8m | 2 tasks | 3 files |
| Phase 47 P06 | not separable (maintainer waits and CI) | 3 tasks | 5 files |
| Phase 47 P07 | not separable (maintainer decisions, merge, CI) | 3 tasks | 3 files |
| Phase 47 P08 | 12m | 3 tasks | 5 files |
| Phase 47 P09 | 5m | 2 tasks | 7 files |
| Phase 47 P10 | 30m | 2 tasks | 5 files |
| Phase 52 P01 | 40min | 2 tasks | 5 files |
| Phase 52 P02 | 30min | 3 tasks | 5 files |
| Phase 52 P03 | 51min | 3 tasks | 5 files |
| Phase 52 P04 | 25min | 2 tasks | 2 files |
| Phase 52 P05 | 14min | 2 tasks | 12 files |
| Phase 52 P06 | 35min | 3 tasks | 7 files |
| Phase 52 P07 | 70min | 3 tasks | 1 files |
| Phase 52 P08 | about 19h wall clock (3 sessions) | 3 tasks | 3 files |
| Phase 53 P01 | 10min | 2 tasks | 5 files |
| Phase 53 P02 | 25min | 2 tasks | 3 files |
| Phase 53 P03 | 70min | 3 tasks | 5 files |
| Phase 53 P04 | 45min | 2 tasks | 4 files |
| Phase 53 P05 | 40min | 2 tasks | 4 files |
| Phase 53 P06 | 14min | 3 tasks | 8 files |
| Phase 53 P07 | 30min | 3 tasks | 9 files |
| Phase 53 P08 | 12min | 3 tasks | 4 files |
| Phase 54 P01 | 25min | 3 tasks | 5 files |
| Phase 54 P02 | 10min | 2 tasks | 3 files |
| Phase 54 P03 | 5min | 2 tasks | 6 files |
| Phase 54 P04 | 8min | 3 tasks | 10 files |
| Phase 54 P05 | 15min | 3 tasks | 6 files |
| Phase 54 P06 | 20min | 3 tasks | 9 files |
| Phase 54 P07 | 25min | 3 tasks | 4 files |
| Phase 54 P08 | 10min | 2 tasks | 5 files |
| Phase 54 P09 | 20min | 3 tasks | 6 files |
| Phase 55 P01 | 12min | 3 tasks | 9 files |
| Phase 55 P02 | 45min | 2 tasks | 8 files |
| Phase 55 P03 | 35min | 2 tasks | 3 files |
| Phase 55 P04 | 50 min | 3 tasks | 8 files |
| Phase 55 P05 | 55 min | 3 tasks | 6 files |
| Phase 55 P06 | 25min | 2 tasks | 3 files |
| Phase 55 P07 | 15min | 2 tasks | 8 files |
| Phase 55 P08 | 45min | 2 tasks | 43 files |
| Phase 55 P09 | 11 min | 3 tasks | 3 files |
| Phase 55 P10 | 40min | 2 tasks | 31 files |

## Accumulated Context

### Decisions

**Cleared at the v0.10.0 close (2026-09-23).** Every phase-level decision this section carried for
Phases 22-37.1 is recorded in its own `NN-CONTEXT.md` (and the SUMMARY decision lists) under
`milestones/v0.10.0-phases/`; the locked ones are ADRs in `.planning/decisions/` and the
`## Key Decisions` table in PROJECT.md. Only what a later milestone must honour is kept here:

- **Two-officer token model** — `Commissary` (input-side rationing) is kept and anchored;
  `Treasurer` (output-side spend governance) is reserved for Milestone 14, not built (ADR-0049,
  ADR-0050). `TokenBudget`, `TokenCounterPort`, `TokenUsage`, `max_tokens` are not renamed.

- **X-03 supersession is scoped** — clean breaks were allowed for Phases 31-33 only (ADR-0051); any
  further removal before v0.11.0 needs its own recorded supersession. Phase 44 (ADR-0059) and
  Phase 48 (ADR-0060) each recorded their own.

- **Base-layer wrapper is `Entity<T>`, "node" means a graph vertex** (Phase 48, ADR-0060) —
  `paladin_core::base::entity::Entity<T>` with payload field `data` replaced `Node<T>` as a clean
  break; serde read-aliases keep pre-rename JSON loading (data compatibility, not an API shim);
  `NodeId`/`NodeSpec`/`StateNode`/`TraceEvent::Node*` and the wire schemas are untouched and the
  freeze is pinned by `tests/entity_rename_guard.rs`. Phase 46's v0.10 → v0.11 `MIGRATION.md`
  guide (CURR-23) must carry the Phase 48 §9.2 rows.

- **`paladin-memory` → `paladin-llm`** (`default-features = false`) is the workspace's one lateral
  adapter-to-adapter edge (Phase 33 D-01/D-03); `reqwest` never enters the normal graph through it.

- **MSRV 1.88, measured** from the `time` ≥ 0.3.47 / `rmcp` `process-wrap` chain, single-sourced in
  `workspace.package.rust-version` and enforced by the `msrv` CI job (Phase 22.1; `MIGRATION.md`
  §9.3).

- **Accepted deviations, to revisit** — tracing overhead re-measured in Phase 45 at +19.36 % (log
  sink) / +16.19 % (composite) against PRD 07's ≤ 3 % bar after the D-17 fixes (was +22.18 % /
  +18.46 %, Phase 28 D-16/D-37); the maintainer accepted the new figure at Phase 45 UAT test 4 and
  `WINDOWS.md` row 61 (amending row 35) is waived with that acceptance text; the remaining cost is
  in the dispatcher/sink path, not serialisation (`45-BENCH-EVIDENCE.md`). Closed in this milestone:
  run-inspection routes are tenant-scoped (Phase 40, ADR-0054, row 32) and legacy `Runnable::Agent`
  runs now emit SSE/webhook events (Phase 45 PLAT-08, row 31 → fixed row 60). Still open: SSE `done`
  reports `halted` for a caller-cancelled run whose persisted status is `Cancelled` (Phase 27 D-14;
  Phase 42).

- **Reasoning effort is honoured exactly or refused before HTTP** (Phase 51, ADR-0062, amends
  ADR-0004 and extends ADR-0061) — one `#[non_exhaustive]` `ReasoningEffort` per request, agent and
  provider; substitution only under an explicit `NearestLower` / `NearestHigher` with both values on
  the response and trace; per-model levels and budget bounds seeded only from the dated
  `51-EFFORT-PROBE.md` transcript (an unprobed model stays `Unmeasured` and refuses until overridden);
  `LlmPort::check_reasoning` pre-judges without sending. Owner decisions at UAT 2026-10-23: Anthropic
  `Budget(n)` stays operator-override-only (the 4.6 pair is the only `budget_tokens`-legal target and
  costs at or above every successor); Gemini 2.5 ids stay `Unmeasured` (404 at probe time). Phase 46's
  guide must carry the Phase 51 MIGRATION §9.2 rows and DR-20..26.

- **Release mechanics** — tags are cut on `main` merge commits per the Phase 29 two-SHA rule; the
  `.project/v0.10.0/09-program-acceptance-audit.md` sign-off boxes are ticked by a human, never an
  agent (Phase 29 D-17, Phase 37 D-00a); `make publish-dry-run` resolves from the local workspace
  overlay and is blind to registry publish order — `scripts/check-publish-order.sh` in
  `make check-gates` and CI is the gate for that class (Phase 37.1).

- **Partial-publish recovery** — maintainer's option A: a patch release through the same pipeline,
  the failed tag left in place and its GitHub Release bannered/pre-release, orphaned crate
  versions yanked by the maintainer only after the patch is registry-verified, one register row
  per crate in `docs/src/appendix/release-recovery.md` §5 (Phase 37.1 D-02).

- **v0.11.0 released (Phase 47, 2026-10-26)** — the annotated tag `v0.11.0` peels to PR #59's merge
  commit `70657919` (two-SHA rule held; the tag was cut 1 h 54 min after `ci.yml` went green on it);
  all 12 publishable crates are live at `0.11.0` through Trusted Publishing by `release.yml` run
  40396108246; the `Coverage` job on the merge commit printed `Lines: 167104/180183 = 92.74%` against
  the 82 % floor. Maintainer decisions on the way: the root `[Unreleased]` was condensed in 12,000-byte
  chunks under a ledger to fit the GitHub release-body limit (B-1, accepted by the maintainer's read
  at UAT), the tokio advisory was cleared by a lockfile pin (B-2), the changelog stamps were redated
  to 2026-10-26, and two re-seal rows the sandbox could not run (`paladin-web` 0.9.0 semver
  baseline build; `make test`) were attributed to their CI twins. Close-out docs ride PR #60.

The pre-close text of this section (Phase 33/32/29/28/23 decision digests and the tool-appended
`[Phase ?]` bullets back to Phase 3) is in this file's git history at commit `6a08c293`.

- [Phase ?]: ADR-0052: mid-run Treasurer enforcement — metering at LlmPort pricing decorator both paths, halt at WarEngine superstep boundary + agent-loop TokenBudget cutoff
- [Phase ?]: ADR-0053: append-only derive-on-read ledger; superstep-aggregate settlement selected at plan 38-01 checkpoint, D-15 key (run_id, superstep, attempt) kept unamended
- [Phase ?]: 38-02: CostTally (Empty/Priced/Unknown poisoning accumulator) added for run-level cost totals shared by TraceDispatcher::total_cost (38-06) and the agent loop (38-07); Task 1's cost_of_call arithmetic needed no correction under the 17-test contract.
- [Phase 38]: Hand-rolled exact-integer decimal parser instead of rust_decimal for treasurer.pricing (no new dependency). — Narrow grammar (digits + optional 1-9-digit fraction) is simple to prove exact with checked_mul/checked_add; keeps dependency count flat (no Cargo.lock/deny/audit/MSRV change, no package-legitimacy checkpoint).
- [Phase ?]: PricingLlmAdapter::generate prices response.model/response.usage via a shared price_or_warn helper, surviving FallbackLlmAdapter hops to differently-named models — 38-04: D-09/D-05 -- one shared helper keeps the pricing/warn-once rule identical on both the streaming and non-streaming run paths
- [Phase 38]: All three heralds (markdown, JSON, table) now render cost exclusively through ExecutionMetadata::cost_display()/cost_currency(); table herald acceptance-grep for removed placeholder literals forbids them anywhere in the file, so regression tests must build forbidden strings from non-contiguous fragments rather than writing them verbatim. — 38-05 closes D-04/D-11 for the JSON and table heralds (markdown done in 38-02) and research Pitfall 4 (table herald ignored its argument).
- [Phase 38]: 38-06: TraceDispatcher::total_cost added as the synchronous twin of total_usage, folded via CostTally::record_node inside emit() itself; all five WarEngine RunFinished emission sites populated with cost: trace.total_cost(). The engine's real Paladin-attempt NodeFinished site stays cost: None pending 38-07's per-attempt wiring.
- [Phase 38]: execute_structured_call forwards response.cost.clone() beside usage, even though Task 2's action text names only the reasoning loop -- required so Task 3's structured-output arm (structured.raw.cost) has a real value to read, matching D-10's "cost rides beside usage everywhere usage travels" rule.
- [Phase 38]: 38-08: HeraldTraceSink wired the engine path's ExecutionMetadata producer into RunWorkerPool; end-to-end engine tests attach HeraldTraceSink directly to a bare WarEngine (mirroring tracer_e2e.rs), not through the full RunWorkerPool/queue/repository harness -- proving the price-table-to-rendered-herald-text chain without unrelated harness weight.
- [Phase 38]: 38-09: The CI-pinned cargo-semver-checks --baseline-version 0.9.0 job is currently a no-op for every package (0 checks, 254 skip) because the in-tree version (0.10.1) already reads as a pre-1.0 major-equivalent bump over that two-milestones-old baseline -- not a Phase 38 defect, but a standing gap until the version is bumped for v0.11.0. — Empirically confirmed across all 11 CI packages; ci.yml is not in this plan's files_modified so it is documented, not fixed. Flag for whichever phase next touches the semver job or performs the v0.11.0 version bump.
- [Phase 38]: 38-09: TraceEvent::NodeFinished/RunFinished.cost fires enum_struct_variant_field_added against the published v0.10.1 baseline (a genuinely unsuppressed lint) but is invisible to the CI-pinned v0.9.0 comparison since TraceEvent postdates that baseline -- registered N/A in MIGRATION.md for the v0.10 -> v0.11 migration guide (Phase 46, CURR-23), no allowlist entry. — Confirmed via cargo semver-checks check-release --baseline-version 0.10.1 --release-type minor; the X-10-governed CI gate only tracks breaks against the published v0.9.0 baseline, so no Cargo.toml suppression or allowlist entry applies to a type that did not exist at that baseline.
- [Phase ?]: 39-01: Task 1 checkpoint (007 treasury_ledger schema/index design) approved as proposed (option-a) by the operator before execution -- one-way scope columns and partial unique settlement index, D-12 index keyed on attributed_at per planner deviation
- [Phase ?]: 39-01: TreasuryLedgerPort's Postgres CLI arm returns the same not(storage-postgres)-feature configuration error regardless of feature flags, since PostgresTreasuryLedger does not exist until 39-03
- [Phase ?]: [Phase 39] 39-02: contract_tests.rs authored as one file spanning both tasks' clauses (shared helpers, non-test-gated module); Task 1's commit includes the full suite text, Task 2 wires it into InMemoryTreasuryLedger and the SQLite test module
- [Phase ?]: [Phase 39] 39-02: real balance+hold overflow (checked_add returning None) proven by priming a fresh scope with a small balance then reserving i64::MAX against ceiling i64::MAX -- the plan's literal 'hold i64::MAX ceiling 0' example never overflows from a balance of 0
- [Phase ?]: 39-03: PostgresTreasuryLedger serializes reserve/reserved-settle/release per scope with a transaction-scoped pg_advisory_xact_lock(hashtext($1)::bigint) taken before any SUM or state read; an unreserved settle needs no lock, since idempotency is enforced entirely by the partial unique settlement index.
- [Phase ?]: 39-03: No Docker daemon available in this sandbox; brought up a local Postgres 16 cluster (pg_ctlcluster) with a scratch paladin/paladin_treasury_test role+database to prove all 23 treasury::postgres tests live (0 SKIP:), per the plan's own fallback instruction. CI's postgres-integration job remains the authority for the Docker-gated path.
- [Phase 39]: 39-04: SpendHook::child carried #[allow(dead_code)] in Task 1's commit (removed in Task 2) since its only production call site (ChildEngineResources.spend) is wired in Task 2 -- a documented Rule 3 auto-fix to keep Task 1's own clippy -D warnings gate green standalone.
- [Phase 39]: 39-04: Every superstep::run/run_with_namespace call site (18 total, including 14 test-only helpers across superstep.rs/mod.rs/graph.rs) needed the new spend trailing parameter, not just the 4 named production sites -- found by grepping every call site rather than trusting the plan's named line numbers.
- [Phase ?]: 39-05: settle_agent_loop_call extracted as a free function so execute_stream_inner's spawned task (owns no &self) shares the exact settle logic PaladinExecutionService::settle_model_call delegates to for the buffered loop.
- [Phase ?]: 39-05: the streamed agent-loop path settles only under AgentLoopSettlement::EveryCall (never PlatformRunsOnly) since no execute_stream_scoped exists -- a stream never carries a RunScope to read a Platform run id from, so it always settles under (execution_id, 1, 1).
- [Phase ?]: 39-05: build_agent_with_llm/build_agent gained a trailing Option<Arc<dyn TreasuryLedgerPort>> parameter mirroring the existing trailing price_table parameter, keeping the Treasurer's pricing and ledger concerns parallel at one call site rather than a second composition mechanism.
- [Phase ?]: [Phase 39] 39-06: CostDto shared by RunResponse and ExecuteResponse; run_costs makes exactly one TreasuryLedgerPort::spend(SpendGroupBy::Run) call per GET /runs request/page, degrading to null on any ledger error or mixed-currency run
- [Phase ?]: [Phase 39] 39-06: openapi_golden_v0_9.rs (SHIP-02 frozen v0.9.0 diff, not in the plan's files_modified) needed a second sanctioned ExecuteResponse exception (cost/CostDto) mirroring the Phase 31 usage/token_count one -- discovered by the plan's own cargo test -p paladin-web verify command
- [Phase ?]: 39-07: RunWorkerPool::with_treasury_ledger attaches WarEngine::with_treasury_ledger to every per-run engine_factory-built engine keyed by SettlementContext { scope: LedgerScope::unattributed(), run_id, attempt } where attempt is run.attempt on first dispatch or bump_attempt's return on a Running redelivery (D-07); run_agent now dispatches through execute_scoped with RunScope::default().with_run_id(run.run_id) so an agent-kind run settles under the Platform run id via 39-05's PlatformRunsOnly writer.
- [Phase ?]: 39-07: build_treasury_ledger(&RunStoreConfig) shares the run store's own backend selection (Disabled -> None, Sqlite -> SqliteTreasuryLedger, Postgres -> PostgresTreasuryLedger on storage-postgres); build_run_api wires it into the worker pool, the run engine's PaladinPort (PlatformRunsOnly) and RunApiState, and paladin-server.rs builds it ahead of the agent registry for build_agent_registry_with_ledger and FacadeProvisioner::with_treasury_ledger.
- [Phase ?]: 39-07: build_run_api_wires_the_treasury_ledger added as its own dedicated test (rather than only extending the two existing RunApiState-field tests) so the plan's own acceptance criterion -- a test literally named build_run_api_wires_the_treasury_ledger passing -- is satisfied by name.
- [Phase ?]: [Phase 39] 39-08: no new .cargo/semver-checks-allowlist.toml entry or Cargo.toml lint-table line was needed anywhere -- every lint the 0.10.1 diagnostic run reported (paladin-web's RunResponse.cost, ExecuteResponse.cost) was already covered by an existing crate-wide suppression, confirmed by a temporarily-disabled-and-reverted diagnostic (D-27 method).
- [Phase ?]: [Phase 39] 39-08: RunResponse gained its first MIGRATION.md §9.2 row, marked N/A for CI set-equality -- it is a new-in-0.10 type absent at the v0.9.0 baseline, so its cost field addition is recorded for the v0.10 -> v0.11 migration guide (Phase 46, CURR-23) rather than as a CI-gated row, mirroring the ThreadApiState/ResumeAcceptedResponse precedent.
- [Phase ?]: Task 1 checkpoint: option-a (Principal #[non_exhaustive] + Principal::new) auto-selected under --auto mode
- [Phase ?]: Postgres run-attribution columns deliberately deferred to 40-02; SQLite adapter fully wired this plan
- [Phase ?]: 40-02: RunQuery.scope defaults to RunReadScope::All so internal callers stay unscoped; only list_runs narrows it from Principal::read_scope() (D-12 / Pitfall 8)
- [Phase ?]: 40-02: tenant scope is a bound predicate inside each adapter's own keyset query (SQLite/Postgres WHERE tenant_id = ?, in-memory permits before sort/page), never a Rust post-filter
- [Phase ?]: 40-03: AuthConfig::validate() checks api_keys even when http.auth.enabled is false -- a disabled section's keys take effect the moment auth is re-enabled
- [Phase ?]: 40-03: boot-time duplicate key-value detection compares with plain == (value -> first name map) and names both keys by name only; ct_eq stays on the request path
- [Phase 40]: 40-04: LedgerScope::from_attribution is the only attribution-to-scope mapping; worker (engine + agent-kind) and HTTP agent handlers settle under the submitting principal, the unattributed sentinel only where no principal exists
- [Phase 40]: 40-04: PaladinExecutorPort::execute_scoped and StreamingExecutorPort::execute_stream_scoped are defaulted scope-ignoring delegates (X-10.4); an unattributed agent-kind run's RunScope carries Some(sentinel), the None fallback is reached only by scope-less embedded callers
- [Phase 40]: 40-05: stream/cancel/webhook-deliveries gate on load_visible_run and now answer 501 naming run_store.backend when the run store is unwired; the route matrix enumerates /v1/runs/{run_id}* from the router's own OpenAPI document
- [Phase 40]: 40-05: utoipa descriptions are single-line strings and public handler docs name the private load_visible_run in plain backticks (cargo doc -D warnings)
- [Phase ?]: 40-06: paladin-web | Principal semver allowlist entry reduced to struct_marked_non_exhaustive -- cargo-semver-checks 0.50.0 does not fire constructible_struct_adds_field for a field added to a struct made #[non_exhaustive] in the same change
- [Phase ?]: 40-06: no Cargo.toml semver allow line added for Phase 40 -- every lint the D-27 diagnostic fired is already under an existing crate-wide allow
- [Phase ?]: 40-06: WINDOWS.md row 32 stays waived (tool refuses WINDOWS_ALREADY_RESOLVED); closing condition recorded as met in CHANGELOG and ADR-0054; row 59 filed for the unsatisfiable RunReadScope current-exports grep
- [Phase ?]: 40-06: a pub mod carries an outer /// doc or inner //! docs, never both -- rustdoc merges them and resolves intra-doc links in the parent scope (7 span-less warnings fixed in be3a9030)
- [Phase ?]: Phase 45-01: RustFS passes the FileStoragePort contract suite with the existing rust-s3 adapter (11 cases, native 1.0.0); D-08 second-adapter path stays closed
- [Phase ?]: Phase 45-01: RUSTSEC-2025-0111 register row re-pointed at tokio-tar -> testcontainers; still suppressed because testcontainers 0.24.0 pulls tokio-tar
- [Phase ?]: 45-02: agent runs route failure through RunFinished{Failed} then persist_failure (one terminal event); record_engine_failure keeps its direct publish for graph-path callers
- [Phase ?]: 45-02: WINDOWS row 31 stays waived; closure recorded as fixed row 60 via the ledger tool (40-06 precedent)
- [Phase ?]: 45-03: LogTraceSink stays a Copy unit struct with a thread-local bounded buffer; enablement guard kept outside write_trace_line; TraceDispatcher::emit gets no guard (seq/tallies/replay, D-00f) - rustdoc only
- [Phase ?]: 45-04: s3-storage not added to Coverage job or scripts/coverage.sh (Pitfall 7); contract suite gated by its own compiled-in and passed-count CI steps
- [Phase ?]: 45-04: k8s RustFS runs with console disabled, non-root uid 10001; k8s/minio.yaml renamed to k8s/rustfs.yaml
- [Phase ?]: 45-05: dev compose inherits the pinned RustFS image in the .dev.yml override; console stays on in dev/devcontainer (off in CI/k8s); coverage.sh log label kept
- [Phase ?]: Phase 45-06: ADR-0055 records RustFS as the dev/test and reference object store; interim MinIO re-pin todo superseded not executed; s3-storage stays out of scripts/coverage.sh
- [Phase 45]: 45-07: OBS-05 re-measured, Verdict AMEND: log_sink +19.36 %/composite +16.19 % at point C (target enabled); WINDOWS row 61 open pending maintainer acceptance at Phase 45 UAT, then windows waive 61
- [Phase ?]: [Phase 41] 41-01: TreasuryLedgerPort::balance is a defaulted method (InvalidRequest) so the 9.2 register rows are N with no allowlist entry; admission fails closed on any ledger error and a request with no principal never reaches the Treasurer
- [Phase ?]: [Phase 41] 41-01: design checkpoint resolved option-b -- D-17 operator webhook payload amended to carry tenant_id and api_key_id (twelve keys)
- [Phase ?]: 41-02: No Treasurer production change needed; admission rule proven over a scripted store clock. Over-admission race remains an accepted ADR-0056/Phase 42 backstop
- [Phase ?]: 41-03: allowance webhook secret is env-supplied (APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET) because the config loader does not expand ${VAR}; WINDOWS row 62 tracks the example configs
- [Phase ?]: 41-03: build_run_api runs the D-11 allowance coherence check before the disabled-store early return and builds one Treasurer only when entries exist
- [Phase ?]: 41-04: fork and submit share one private admit_and_persist lifecycle; agent routes admit then confirm back to back via admit_principal (jobs refuses before jobs.create)
- [Phase ?]: 41-04: agent-handler rustdoc left unchanged because utoipa copies it into the frozen v0.9 operation description; golden exception removes only responses[429] on three agent operations
- [Phase 41]: Schedule-fired runs keep skipping authorize_invocation: SubmitRun.attributed_to is identity only, never a role (41-05, D-08/C13)
- [Phase 41]: ScheduleResponse does not expose a schedule's created_by while GET /v1/schedules is not tenant-scoped (41-05, Open Question 7)
- [Phase 41]: Pre-Phase-41 schedules (NULL created_by) fire unattributed and ungated; tracked as open WINDOWS.md row 63 (41-05, D-08)
- [Phase ?]: 41-06: notices dedup is store-enforced (unique idx_treasury_notices_once + ON CONFLICT DO NOTHING); api_key_id '' for tenant scope and epoch window_start for lifetime; claim before insert, abandon discards; notice failure never blocks a run
- [Phase ?]: 41-06: validate_notice rejects a window notice without bounds and a lifetime notice with a window, so adapters agree on lifetime read-back
- [Phase 41]: 41-07: allowance trace event emitted by the worker on the Queued first dispatch before RunStarted; one shared herald line via ExecutionMetadata::allowance_warning_display; notice store opened once in build_run_api and shared by pool and Treasurer
- [Phase 41]: Operator allowance notice rides the durable webhook_deliveries queue via RunEventKind::AllowanceWarning and a correlation run id no run owns (C3 Option A, twelve-key payload, option-b)
- [Phase 41]: Operator webhook secret is held only on WebhookDeliveryService and signs allowance_warning deliveries before any run lookup; never on the row
- [Phase 41]: build_run_api SSRF-checks treasurer.allowance.webhook.url at boot before spawning anything; enable the webhook only after every replica runs this build (Pitfall 12)
- [Phase ?]: ADR-0056 (allowance admission model) accepted: tumbling UTC windows, check-only admission, every-limit composition, no role bypass, fail-closed, store-deduped notices; over-admission race closed by Phase 42
- [Phase ?]: Phase 42-01: option-b design gate; ADR-0057 records check-only boundary, typed HaltCause, fork-as-resume, derived agent budget; true streamed done carries informational halt_reason on crossing
- [Phase 42]: Plan 42-02: the superstep-boundary spend check is a check-only balance read through the one shared Treasurer::evaluate that admission also calls; no reserve row, settlement and the ledger schema untouched — ADR-0057 D-01/D-17: one function per rule keeps admission and the boundary from drifting; the same-instant over-admission race stays accepted and bounded to one superstep per run
- [Phase 42]: Plan 42-02: RunOutcome::Halted carries a typed HaltCause; a spend halt maps to Halted with error None whatever the cancel and shutdown flags say, CancelRequested maps to Cancelled, and the in-process Token cause keeps today's precedence — ADR-0057 D-05: the cause is typed at the source so the worker, the persisted reason (42-03) and the SSE done (42-05) all read one value instead of re-querying flags
- [Phase ?]: 42-03: halt_reason is the last runs INSERT column on both backends; record_outcome precedes update_status for Halted and Cancelled (G14); the halted webhook key is gated on status Halted
- [Phase ?]: 42-04: No new public symbol; the fail-closed log line gained the scope kinds via a pure builder (fail_closed_message) tested without a process-wide logger
- [Phase ?]: 42-04: A halted run resumes only by a forked run that re-runs admission (429 + Retry-After while exhausted; 500 while the ledger is unreadable); the halted run stays terminal
- [Phase ?]: 42-06: PerRunCancelProbe attached on every factory-built engine: a same-instance caller cancel reaches the engine as CancelRequested; RunFinishStatus::Cancelled (non_exhaustive) drives done/cancelled
- [Phase ?]: 42-06: A worker drain emits no done: RunEventBusSink drops a reasonless Halted while the shutdown token is cancelled; a spend halt during shutdown is still emitted
- [Phase ?]: [Phase 42-07]: Admission::is_empty still answers only whether notices were won; a derived budget does not make an admission non-empty
- [Phase ?]: [Phase 42-07]: derive_budget refuses an exhausted ceiling before any pricing lookup; a foreign-currency price table is a Backend error naming both codes, never converted
- [Phase ?]: [Phase 42-07]: TokenBudget tightest-wins: a tie goes to the Treasurer (AllowanceHalted), an operator win keeps TokenBudget; the derived figure lives on ModelCallContext, never the middleware
- [Phase ?]: [Phase 42-07]: halt figures report the binding ceiling at balance equal to ceiling (A5); a zero-figure refusal carries the real balance
- [Phase ?]: 42-08: option-b true-stream done carries an informational halt_reason only on a strict total_tokens crossing of the derived figure; supersedes the byte-identical-in-all-cases reading; no-crossing and no-usage streams stay byte-identical
- [Phase ?]: 42-08: operator agent_runtime.token_budget now takes effect on HTTP agent routes via the one TokenBudget installed per agent service; tightest wins, a tie goes to the allowance
- [Phase ?]: 42-08: v0.9 golden Phase 42 exception is one document-level function (strip_known_v0_11_halt_reason) applied at load; unstripped loaders only for the scope test
- [Phase ?]: 42-09: the budget for a worker-dispatched agent-kind run is re-derived at dispatch from the ledger, never persisted at submit time; a refusal halts with real figures, an unreadable ledger halts ledger_unavailable, an unpriced model records Failed, all without calling the LLM
- [Phase ?]: 42-09: the shared run-engine service installs TokenBudget in Treasurer-only mode (operator figure forced off) so an engine node is never capped; agent-kind runs write no Waypoint and resume by resubmission (WINDOWS.md row 64, D-08)
- [Phase ?]: [42-10] notice_kind is the last column of the rebuilt idx_treasury_notices_once and of every ON CONFLICT arbiter list; the arbiter tests read migration 014
- [Phase ?]: [42-10] operator_event_for(NoticeKind) is crate-private and shared by the payload and the delivery row, so the paladin facade surface is unchanged
- [Phase ?]: [42-10] A pre-014 replica against a migrated database cannot infer the rebuilt index: its notice claim errors (logged, skipped); migrate and upgrade replicas together (MIGRATION 9.4)
- [Phase ?]: [Phase 42-11] The guard's per-run claim memo is keyed like the store's own notice identity (scope, tenant, key name, limit, window start, ceiling, notice kind) and covers the halt claim too; held behind an Arc so the public TreasurerSpendGuard keeps its Freeze auto trait
- [Phase ?]: [Phase 42-11] The worker builds the per-run spend guard after the run's own trace emitter exists; a run with no trace sink still claims and queues its notices and only skips the stream event; a halt notice emits no trace event
- [Phase ?]: Phase 42-12: the halt line states the currency once after the ceiling (25.0000 of 25.0000 USD); ExecutionMetadata stores the rendered line, never the typed HaltReason
- [Phase ?]: Phase 42-12: the G2 windows row states the option-b scope (a true stream's done carries an informational halt_reason only when its terminal usage crossed the derived figure; not byte-identical in every case)
- [Phase ?]: Phase 42-12: the vocabulary guard assembles both forbidden strings at run time and proves it can fail through a planted temporary tree before scanning the repository
- [Phase 43]: 43-01: pacing on by default (treasurer.cadence.enabled false opts out); OpenAI adapter surfaces its first 429; engine-port and shared gate wiring deferred to 43-09
- [Phase ?]: 43-02: an empty RateLimitHints is not stored on the error; the Cadence passes an explicit delay to the port unreduced but clamps a ResetHeader-derived delay to max_backoff; a gate beyond max_wait is refused before any provider call or port write (D-06)
- [Phase ?]: 43-02: paladin-ports gained crate-wide semver allows enum_unit_variant_changed_kind and enum_variant_marked_non_exhaustive (D-27 diagnostic) with two allowlist entries under row paladin-ports | LlmError; .project/current-exports.txt unchanged because the facade baseline lists LlmError as one re-export line
- [Phase ?]: 43-03: an unusable Retry-After falls through to retry-after-ms then the exhausted reset; hints_from_headers returns None when nothing usable parsed; Go durations use exact saturating u128 nanosecond arithmetic
- [Phase ?]: 43-03: quota-class 429s (OpenAI insufficient_quota, Anthropic enforced_spend_limit_reached) map to permanent UsageLimitExceeded before the generic arm; Anthropic surfaces its first 429 (D-02); OpenAI header names and the quota code string stay pending the 43-12 operator checkpoint
- [Phase ?]: 43-05: contract paused clause uses tokio::time::sleep, not advance (test-util is dev-only); waiter spread is internal, additive, capped at min(wait/10, 1 s); InMemoryCadence bounded at 4096 keys
- [Phase ?]: 43-04: D-02 (surface the first 429) extended to the compat engine presets, DeepSeek and Gemini (Open Question 1); reversible, to be recorded in ADR-0058
- [Phase ?]: 43-06: fallback pace budget is wall-clock time from a hop's first 429; a gate that is unreadable, clear or beyond max_wait is not paced (hop rule applies)
- [Phase ?]: 43-06: config-built fallback chains are paced by default via AgentRuntimeDeps.cadence (in-process, None opts out); with_cadence is a no-op on an already-paced chain
- [Phase ?]: 43-07: Redis cadence gates computed in Lua on the server TIME with relative waits; connection lazy with explicit timeouts; provider half of the key escapes % and : so keys are injective
- [Phase ?]: 43-08: the Redis-outage warning is a log::warn! under paladin::cadence (no TraceEvent variant); ResilientCadence latches on the first primary error, probes at most every 5 s, merges the fallback gate on recovery
- [Phase ?]: 43-09: Redis URL never in config/Debug/logs -- CadenceConfig holds only url_env name; boot log names backend kind only
- [Phase ?]: 43-09: one CadenceWiring built in paladin-server shared by registry, provisioner and run API; FacadeProvisioner rejects (not silently downgrades) a redis config on a binary without redis-cadence
- [Phase ?]: 43-10: NodeCachePort gains one defaulted put_fenced (delegates to put) so D-14 fencing reaches the Redis cache without changing D-29 or any implementor; flagged reading of D-00d/D-11 for the operator and ADR-0058
- [Phase ?]: 43-10: Redis lock/counter keys use the %lock and %fence segments so they can never equal a pacing key (a provider named lock would collide with the plan's literal layout)
- [Phase ?]: 43-10: FencingToken carries its source (Distributed/Local) with no Ord; Local tokens are a plain put at the cache and never compared with Distributed ones
- [Phase ?]: 43-11: the stampede-lock winner re-reads the node cache once after acquiring the lock (double-checked locking) and serves an entry that appeared between its first miss and its acquisition; engines without a lock keep exactly one get per miss
- [Phase ?]: 43-11: a lock wait cancelled by the run returns Interrupted (attempt 1) so the run ends Halted and the node is re-listed for resume; a task aborted at the grace deadline cannot unlock and its lock expires by TTL
- [Phase 43]: 43-12: OpenAI provider header names recorded VERIFIED (operator, 2026-10-09) because sandbox egress blocked OpenAI hosts; Anthropic rows carry fetched verbatim quotes
- [Phase ?]: Phase 43-13: ADR-0058 records the Cadence design; NodeCachePort::put_fenced (defaulted) is the one deliberate reading of D-00d/D-11, lock_ttl_secs defaults to 120, no lock renewal
- [Phase ?]: Phase 43-13: AgentRuntimeDeps.cadence corrected from N to Y in the X-10 register (constructible_struct_adds_field, covered by the root crate-wide allow); allowlist entry added
- [Phase ?]: [44-01] Campaign ignores aegis.on_error (no continue mode) and warns once per execution
- [Phase ?]: [44-01] validate_aegis rejects Route/Custom handlers, Custom retry predicates and zero durations; cache ignored
- [Phase ?]: [44-01] idle_timeout degrades to a per-attempt wall clock; tighter bound wins, tie is TimeoutKind::Run
- [Phase ?]: [44-02] ADR-0059 supersedes X-03 for Phase 44 only; ADR-0051 is a copied shape, not inherited
- [Phase ?]: [44-02] ADR-0059 marks the planner refinements: retry_attempts = max_attempts - 1 (refines D-04), ConclaveError::Timeout removal (extends D-03), three separate 9.2 rows (Finding 10)
- [Phase ?]: [44-03] Breaker and Conclave read transience() alone, Transient-only; Unknown is neither retried nor counted; the stringly LLM variant and both legacy predicates are removed with no bridge
- [Phase ?]: 44-04: Formation Absorb keeps the v0.10 node_errors shape until 44-06; unsupported on_error variants fail closed with ValidationError
- [Phase ?]: [44-05] Phalanx keeps collect-then-fail under on_error None; execute_with_cancellation polls the token first (biased) so Cancelled is deterministic
- [Phase ?]: 44-06: node_errors element persisted as {node_id (Paladin name), attempt, transience, source}; operator chose option-a on 2026-10-11
- [Phase ?]: 44-06 context: operator is considering a later phase 44.1 renaming the Node<T> entity envelope to Entity<T>; out of scope for phase 44
- [Phase ?]: 44-07: Conclave retry_attempts stays the retry count; aegis.retry supplies only the backoff shape and the predicate stays Transient-only; aggregator is a single attempt
- [Phase 44]: 44-08: Aegis types imported from paladin_core directly; CLI maps timeout_seconds onto aegis.timeout.run_timeout per attempt (schema unchanged)
- [Phase ?]: 44-09: docs point to programmatic BattalionConfig.aegis; no Aegis YAML schema invented, config.yml has no battalion: section
- [Phase ?]: 44-10: v0.10 BattalionConfig documents stay readable by ignoring the removed keys (no deny_unknown_fields, alias or untagged shim); ChainOfCommand test drops its dead timeout assignment
- [Phase ?]: 44-11: Phase 44 breaks registered from a measured cargo-semver-checks 0.50.0 run vs 0.10.1 (10 9.2 rows, 15 allowlist entries); BattalionResult node_errors retype is an N row (no lint modelled); paladin-ai facade retry re-export fired no lint
- [Phase ?]: 44-13: legacy-removal guard skip set is exactly the declared history set; scoped-variant and missing-target controls live inside scanner_reports_planted_legacy_uses
- [Phase 44]: Phase 44-12: ADR-0059 Code Conformance is conforms; LEGACY-04 ticked after the full phase gate (workspace tests green apart from the 2 baseline run_api_wiring failures, doctests, clippy, fmt, clean-code, security, check-gates, api-surface, examples, MSRV 1.88, both guards)
- [Phase 48]: 48-02: ADR-0060 supersedes X-03 for Phase 48 only; serde read-aliases are data compatibility, not an API shim; the five unused base items are retained pending the content / general data orchestration review with a bounded list of residual entity-sense identifiers
- [Phase ?]: Phase 48-01: harness reads today's names; later plans rewrite keys; tests avoid naming the citadel placeholder types
- [Phase 48]: 48-03: serde read-aliases node_id/node_data sit on EntityVersion only, no serde(default); both spellings rejected as duplicate field (D-09, D-11)
- [Phase 48]: 48-03: regenerated API baseline diff is 14 ContentItemService content lines plus the generated-at header line; header kept truthful
- [Phase ?]: 48-04: Stage A landed as one commit because the scripted pass alone fails cargo fmt; the citadel placeholder importers measured at four files plus citadel.rs (CONTEXT said five)
- [Phase ?]: 48-04: base module rustdoc links Entity<T> via crate::base::entity::Entity (relative entity::Entity does not resolve in a module's inner doc)
- [Phase ?]: 48-05: Stage B landed as one commit (cafc93b); Entity.data / ContentItem.entity / PromptItem.entity are read-aliased to node, alias edge cases pinned by six tests (D-03, D-08)
- [Phase ?]: [Phase 48-06] SVG closed with a missing </svg>; README usage snippet uses PaladinStatus::Idle because TaskStatus does not exist
- [Phase 48]: Phase 48-07: register only the lints the D-27 diagnostic fired (11 on paladin-ai-core, 1 on paladin-ai); ContentItem/PromptItem register both the removed node field and the added entity field; ContentItemService is a register-only N row
- [Phase ?]: 48-08: entity rename guard pins five rules and the graph freeze; skip set copied from Phase 44 unchanged, real tree needed no source fix
- [Phase 48]: Phase 48-09: ADR-0060 Code Conformance conforms; make security and the numeric coverage figure recorded as gaps (tools/artifact unavailable in sandbox; CI security jobs red before Phase 48)
- [Phase 48]: Phase 48-09: entity rename guard skips the graph schema only when the tree has no docs/ directory (Docker test image context)
- [Phase 49]: check-consumer-surface stays outside check-gates (it builds the facade crate; check-gates members are offline script guards)
- [Phase 49]: muster [[test]] uses required-features so --no-default-features fails loudly instead of skipping
- [Phase ?]: Phase 49-02: library-consumer note is a ### subsection after MIGRATION.md 9.8 step 7 (no new ## 9.9); rows labelled DR-n so the allowlist guard never reads them; section 9.2 wording wins over the research drift register
- [Phase 49]: 49-03: red-proof scripts stay outside the repo; full text recorded in 49-MUSTER-RED-EVIDENCE.md
- [Phase 49]: scope cut 2026-10-14 — the consumer never adopts v0.10.1; it moves once to v0.11.0 after Phases 50-51 (MIGRATION.md §9.8 consumer note is the spec); plans 49-04..06 cancelled
- [Phase ?]: 50-01: matrix asserts the query is contained in the last user turn (the execution service folds system prompt + query into UserPrompt::query); adapter-level test keeps exact equality
- [Phase ?]: 50-01: PARITY-01 left open (matrix has 19 red cases ignored until plans 50-03/05/12/13/14); only PARITY-08 marked complete
- [Phase ?]: [Phase 50-02] ADR-0061 amends ADR-0004: refuse explicit, omit auto visibly; ModelProfile is additive and fail closed; owner readings R-1 (50-18) and R-2 (50-17) recorded pending
- [Phase ?]: 50-04: probe calls are bounded (max_tokens 256, 120 s) with Moonshot thinking disabled; OpenAI baseline cap field is max_completion_tokens; JSON-mode probes use a JSON-naming prompt
- [Phase ?]: [Phase 50] 50-03: ModelOverride is pinned through the config crate (the production loader), not serde_yaml, because serde_yaml 0.9 reads enums from YAML tags and would not accept the {fixed: 0.95} form operators write
- [Phase ?]: [Phase 50] 50-03: a compat preset that declares no temperature_range gets ADR-0004's (0.0, 1.0) in its fallback profile, so the wire filter and the builder check agree; a direct call above 1.0 on such an endpoint is now omitted and recorded (listed for the plan 50-05 MIGRATION 9.1 row)
- [Phase ?]: [Phase 50] 50-03: an effective max_tokens above a profile's max_output_tokens is refused by name before any request; the framework default PromptParameters.max_tokens is 1000, so seeded max_output_tokens (plan 50-06) must stay at or above it or the default must be made explicit (plan 50-08)
- [Phase 50-05]: RequestNotes moved into sampling.rs so the compat engine and the four bespoke adapters stamp the three metadata keys through one type
- [Phase 50-05]: Anthropic adapter-wide profile states temperature and top_p NotSent, so an explicit temperature is refused at build by name; OpenAI keeps max_tokens 4096 as a compiled default that is checked against the profile
- [Phase ?]: 50-17: owner reading R-2 answered accept unverified for kimi, qwen, grok, ollama, openai-compatible (2026-10-16); no usage-cap error_override to be encoded in 50-13; Ollama and OpenAI-compatible deliberately skipped in the seed probe
- [Phase ?]: 50-06: seed only models the probe measured end to end (a seeded entry replaces the fallback); Grok temperature Range(0.0, 1.99) from the vendor's observed 'less than 2' text; Kimi temperature/top_p NotSent; OpenAI/Anthropic/Gemini penalties NotSent (wire has no field)
- [Phase ?]: 50-06: defaults moved to operator-identified current models (claude-haiku-5-5, deepseek-flash, kimi-k2.6, qwen3.8-flash, grok-4.3, gemini-3.5-flash-lite); adapter windows follow the default model's seeded window
- [Phase ?]: [Phase 50-07]: Per-model window reaches both readers through window::effective_capabilities (overlay on ProviderCapabilities), not a new WindowSource variant; Commissary::for_model added, Commissary::new unchanged
- [Phase ?]: 50-08: owner chose option-a - all nine paladin-llm *Config types are #[non_exhaustive] with with_max_tokens/with_max_retries/with_context_window/with_model_overrides builders; every new()/from_env() signature unchanged; signature drift tracked as a separate todo
- [Phase ?]: 50-08: Anthropic and Gemini keep their total-attempts retry counter (max_retries passed through unchanged, default 3); compat presets, OpenAI and DeepSeek count retries after the first attempt; unification deferred
- [Phase ?]: 50-09: LlmProviderConfig is non_exhaustive with new(..) and with_* builders (option-a, applied from 50-08); breaking change registered with the semver measurement in 50-10
- [Phase ?]: 50-09: An unset ${NAME} placeholder is a construction error even when the provider env var is set; is_configured agrees (false)
- [Phase ?]: 50-09: D-13 layering lives in config/layers.rs as a pure function over an injected environment; the From conversions run it leniently with no environment and never carry a placeholder as the key
- [Phase ?]: 50-09: Env vars are read only for fields the block leaves unset; the 300 s reasoning default counts the block's models overrides
- [Phase ?]: 50-10: unconfigured registered provider is HostBuildError::Provider (names agent, provider, model, variable); UnknownProvider kept for unregistered names; HostBuildError enum unchanged
- [Phase ?]: 50-10: LlmConfig::validate accepts a recognised default provider with no block (env-only default provider valid)
- [Phase ?]: 50-10: D-27 measured: only struct_marked_non_exhaustive fired (196 checks) on the nine *Config types and LlmProviderConfig; registered with 10 rows and 10 entries; re-run exits 0
- [Phase ?]: 50-11: LlmResponse.reasoning sits after cost; registered on the existing paladin-ports | LlmResponse 9.2 row and allowlist entry (PRICE-03, PARITY-05), no new pair; D-27 for paladin-ports fired constructible_struct_adds_field on LlmResponse.cost and .reasoning only for this item
- [Phase ?]: 50-12: TokenUsage.total_tokens stays prompt+completion (D-02); vendor total not copied despite plan truth; owner decision needed if vendor total must be preserved (paladin-core contract change)
- [Phase ?]: 50-12: response_guards::classify_completion plus StreamCompletion decide refusal > content_filter > function_call > empty-on-any-stop for compat, OpenAI, DeepSeek; widen its cfg feature list in 50-13 for anthropic and gemini
- [Phase ?]: 50-13: no usage-cap error_override encoded for kimi/qwen/grok/ollama/openai-compatible (owner reading R-2: accepted unverified 2026-10-16)
- [Phase ?]: 50-13: Gemini RECITATION/BLOCKLIST/PROHIBITED_CONTENT/SPII map to ContentFilter like SAFETY; RECITATION was Error(value)
- [Phase 50]: 50-14: paladin.served_by/served_model are recorded on buffered responses only (streams carry no metadata); hop_on_refusal is code-level only, no config key (D-10)
- [Phase 50]: 50-14: FallbackChainError::EmptyStepModel fires enum_discriminants_undefined_non_unit_variant (measured, not forecast); registered with one paladin-llm manifest allow line
- [Phase ?]: 50-15: Anthropic's warn-and-skip stream behaviour and DeepSeek's lack of a typed refusal are asserted by name in the conformance suite (MalformedFrame, RefusalShape) rather than skipped; OpenAI fixture default model is the unseeded gpt-4o (adapter-wide window 128000)
- [Phase ?]: 50-18: Owner reading R-1 = option-b (2026-10-16): the paladin.* run-metadata keys reach every caller -- PaladinResult.run_metadata, StreamingResponse.metadata on the terminal chunk, and ChunkMetadata.run_metadata on a streamed run's final chunk; one registered break on PaladinResult, measured by D-27 (constructible_struct_adds_field only)
- [Phase ?]: 50-18: a streamed execution-service run has no PaladinResult, so the streamed merge lands on additive ChunkMetadata.run_metadata (carrying paladin.served_by too); FallbackLlmAdapter streams stamp served_by and served_model on the terminal chunk
- [Phase ?]: 50-16: the Switching providers drift test compares every column of every seeded row and is compiled only with all nine adapter features; the usage-cap column reads unverified (owner accepted 2026-10-16) for the five compat presets and does not call any other vendor verified
- [Phase ?]: 50-16: closeout register check closed WINDOWS rows 70, 71, 72 on measured evidence, left 73 and 74 open, and recorded rows 75 (OpenAI max_tokens vs gpt-5.4-nano) and 76 (Kimi fallback profile) as open owner items
- [Phase ?]: 50-19: OpenAI adapter sends max_completion_tokens for every model (owner option (a)); measured on gpt-5.4-nano and gpt-4o-mini only, other hosts use the openai-compatible preset
- [Phase ?]: 50-20: Kimi adapter-wide fallback declares temperature, top_p and both penalties NotSent (owner option (a), G-50-5c); temperature_range is ADR-0004's (0.0, 1.0), no vendor claim
- [Phase ?]: 50-21: Grok folds reasoning_tokens into completion_tokens (ReasoningBesideCompletion, secondary-sourced, live-measured by 50-22); compat presets parse the vendor total and record disagreements under paladin.usage_unreconciled; D-02 kept, no TokenUsage field added
- [Phase ?]: 50-23: Gemini folds toolUsePromptTokenCount into prompt_tokens; OpenAI, DeepSeek and Gemini parse their vendor total (now Option, absence = None) and record disagreements under paladin.usage_unreconciled on both paths; Anthropic runs containment checks only; Anthropic vision folds cache counts; D-02 kept
- [Phase ?]: 50-22: live usage probe (operator, 2026-10-18): Grok grok-4.3/4.5/4.6 reasoning beside completion, billed at the completion rate (cost_in_usd_ticks consistent to the tick); DeepSeek, Kimi, qwen3.8-flash inside; Gemini thoughts beside candidates; OpenAI, qwen-plus, gemini-3.5-flash-lite indeterminate; Ollama and OpenAI-compatible not configured
- [Phase ?]: 50-24: 50-USAGE-PROBE.md verdicts applied by the fixed rules with no semantics flip; Grok ReasoningBesideCompletion now live-measured (cost cross-check consistent); Kimi, Qwen, DeepSeek, Gemini labelled measured 2026-10-18; OpenAI, Anthropic, Ollama, OpenAI-compatible not live-measured; no WINDOWS row (no mixed or other result); MIGRATION M-B-20
- [Phase ?]: 50-25: LlmError::empty_completion / LlmError::refusal are permanent public constructors (A2-50-25); seven consumer files moved to shape-proof forms ahead of 50-26
- [Phase ?]: 50-26: LlmError::EmptyCompletion { detail, billed } and Refusal { .., billed } are #[non_exhaustive] struct variants carrying Option<Box<BilledUsage>>; billed_usage() looks through AllProvidersFailed; compat buffered path bills; D-27 measured enum_tuple_variant_changed_kind (newly allowed), registered in the same commit
- [Phase ?]: 50-27: every adapter bills EmptyCompletion/Refusal on both paths (stream bills on the request model); PricingLlmAdapter prices billed errors via one helper (generate: served model; stream: request model); DeepSeek annotate_with_usage removed; conformance CASE_COUNT 20
- [Phase ?]: 50-28: private CallFailure { error, bills } carries a failed call's BilledUsage out of the retry helper; buffered loop folds and settles priced bills (ordinal loop_num), stream settles a billed terminal error once (ordinal 1, model_used); no public type change
- [Phase ?]: 50-29: IncurredSpend + paladin_ports spend_scope (SpendScope, ATTEMPT_SPEND task-local, record_incurred_spend, current_attempt_spend); race_attempt scopes every attempt so failed, timed-out and structured attempts report their recorded spend; successes keep PaladinResult.cost; no public type changes shape
- [Phase ?]: 50-30: paladin_ports spend_scope HoppedBills / HOPPED_BILLS / record_hopped_bill; FallbackLlmAdapter records every billed hop it moves past (warn-once with no collector), PricingLlmAdapter prices each against its own model; the agent loop folds hop bills into usage, cost and the attempt scope and settles each call's parts in ONE row (run_id, ordinal, 1) with a per-model breakdown, buffered, structured and at every streamed exit; no public type changes shape
- [Phase ?]: 50-31: G-50-5a closed end to end: provider_swap_matrix case billed_failures_and_hopped_refusals_reach_the_ledger (real adapters, Pricing(Fallback), execution service, InMemoryTreasuryLedger); docs, CHANGELOGs, MIGRATION M-B-21/M-B-22/DR-18 and ADR-0061 limit (6) describe the shipped billing contract; WINDOWS row 74 fixed, remainder (c) (Grok cost_in_usd_ticks / num_sources_used) open as row 77 by the owner's 2026-10-17 deferral
- [Phase ?]: 50-32: a hopped-past fallback bill is priced (collector-installed pricer, own model) and counted into ATTEMPT_SPEND at the hop; count_and_take counts only uncounted entries; additive unreleased surface (D-00b)
- [Phase ?]: 50.1-01: Aegis family reaches consumers through five facade module re-exports plus nine prelude names; pacing-only path is public with_cadence; section 9.8 spend-cap wording names only evidenced vendors
- [Phase ?]: [Phase 50.1 plan 02] D-05 proceed-as-locked (owner, 2026-10-20): DeepSeekConfig::new(api_key, model); with_base_url/with_timeout_seconds on all nine configs; measured lint method_parameter_count_changed registered
- [Phase ?]: 50.1-03: Experiment 2 mutates FinishReason (one library site, finish_label); consumer-surface CI job is Advisory attribution, not a new merge gate
- [Phase ?]: 50.1-04: D-05 applied as locked to Gemini, Qwen, Grok and Kimi; each allowlist entry names method_parameter_count_changed as measured per task; SURFACE-02 stays open until 50.1-05 and -06
- [Phase ?]: 50.1-05: D-06 applied as proceed-as-locked; OpenAIConfig.model is a default-model label with no context-window change (128,000 stays); D-05 new(api_key, model) with no alias
- [Phase ?]: Phase 50.1-06: D-07 resolved by owner as compiled-default (2026-10-20): AnthropicConfig::new sets max_tokens 4096 for every model; with_max_tokens is the override; no seeded_max_output helper
- [Phase ?]: 50.1-07: Claude ran the SURFACE-06 seed probe at the operator's direction on the operator's keys (plan's no-key premise was stale); provenance says so
- [Phase ?]: 50.1-07: OpenAI ids with a Deprecated badge that were measured end to end (gpt-5.1, gpt-5, gpt-5-mini, gpt-5-nano, o3) SHOULD be seeded by plan 50.1-09; D-15 excludes only shutdowns on or before 2027-01-31
- [Phase ?]: 50.1-07: grok-4.7 (temperature=2.00 UNREACHABLE in both runs) and the 7 default-rejected ids (gpt-5.5-pro, gpt-5.4-pro, gpt-5.2-pro, gpt-5-pro, gemini-2.5-pro/flash/flash-lite) are not seedable; 50 of 58 candidates measured end to end
- [Phase 50.1 seal]: UAT 1/1 — owner chose fix-before-tag for WR-01..03 (2026-10-20). WR-01/02 were doc-only; WR-03 (`5ebd315b`) tightened `validate()` in seven adapters to a full `http://`/`https://` prefix, a behaviour change recorded in both CHANGELOGs and MIGRATION §9.8 (`aa48314d`) at the owner's request before sealing
- [Phase ?]: 46-01: paladin-eval publishes after the leaf tier and before paladin-ai; its trust link is 'proven by publish' (run 35659477719), not operator-reported
- [Phase 46]: 46-02: Phases 22 and 24 reconciled reconcile-only (D-14): both validated with nyquist_compliant true, 0 gaps; no test written — Every requirement id has a green automated row re-run on HEAD; Steps 4-5 of validate-phase skipped
- [Phase 46]: 46-03: Phase 28 promoted to nyquist_compliant true on 28-UAT 2/2 and WINDOWS rows 35/61 (D-15); Phase 29 and 30 validated and compliant true with superseded rows carrying re-run equivalents; reconcile-only, no test written
- [Phase 46]: 46-04: Phases 34, 36 and 36.1 reconciled out of draft, nyquist_compliant true, 0 gaps (reconcile-only, D-14) — Every requirement id has green automated rows; superseded verify literals name the scoped equivalent re-run on HEAD; Phase 34 gates run from path-adjusted scratchpad copies, never committed.
- [Phase 46]: 46-05: ADR-0015 amended 2026-10-24 (paladin-core 14, paladin-ports 12 incl. log) and pinned by scripts/check-dependency-allowlist.sh in make check-gates and CI cargo-deny; WINDOWS rows 80/81 waived, 82 fixed — Guard compares [dependencies] keys exactly and rejects build/target tables; the sets are mirrored in the ADR so the guard cannot silently bless drift (D-21)
- [Phase 46]: 46-06: consumer-surface muster pins four web3sec items (Stockpile::render, BattalionResult counters, prelude glob in private mod prelude_glob, llm_port::ModelProfile re-export), test-only, each shown red then green
- [Phase 46]: Cadence is one section of the Treasurer page (D-08); the Treasurer page states bounded overshoot and tumbling-window double-spend rather than promising an allowance can never be overspent; ADR-0055 left out of the ADR index
- [Phase 46]: D-23: removed v0.10 BattalionConfig/ConclaveConfig keys are refused at load via a private try_from wire struct (legacy_keys.rs); no shim, alias or mapping, no deny_unknown_fields, ADR-0059 amended — A bounded v0.10 run must not load as unbounded in v0.11; message names keys and aegis.* replacements and cites M-B-06, never a value
- [Phase 46]: 46-09: Commander warns once per Maneuver execution when aegis.on_error or aegis.retry is set, via the pure private fn maneuver_ignored_aegis_policies (D-24 / WR-04); WR-05, IN-05, IN-N1 closed, IN-01..IN-04 and IN-N2 deferred — Maneuver reads only aegis.timeout.run_timeout; a silently dropped policy needs a signal, as Campaign already gives
- [Phase 46]: 46-10: MIGRATION.md is two-part (section 10 above kept v0.10 record 9); register gate window is /^## (9|10)\.2 / in script, ci.yml and harness; four in-place rows split into v0.10.1 text in 9.2 plus same-key 10.2 delta rows — D-01/D-02: row identity preserved, gate widened in the same commit as the first row moved
- [Phase ?]: 46-11: section 10 frozen 2026-10-24 on web3sec's dogfood report against e28e43b5; DR-11 cites both registers (10.2 AllowanceHalted, 9.2 CallLimit/TokenBudget); D-06 warn! deferred
- [Phase 46]: 46-12: every [Unreleased] 9.x citation (root and crate changelogs) names a v0.11 row and is retargeted to section 10; the recipe misses 44 lines (backtick/line-break forms), classified and retargeted too — D-02 / RESEARCH F4: the release note is live, not history; plan acceptance covers every 9.x citation left
- [Phase ?]: 46-14: #[non_exhaustive] and PaladinError variant comments keep 9.2 (v0.10.1 register rows); muster forward rules and DR/Phase 44-50 citations retargeted to section 10
- [Phase ?]: [Phase 47] 47-01: maintainer pre-build decision B1=a B2=a R7=a EMPTY=a SPACE=a DATE=2026-10-25 (reply 2026-10-25T18:19:21Z)
- [Phase ?]: 47-02: B2=a applied (tokio 1.53.3, Cargo.lock only) and B1=a applied (root [Unreleased] 156528 -> 108926 bytes, SHA-PRE-B1 700a92d8); Added tightened in the Task 3 commit
- [Phase 47]: 47-04: row 4 paladin-web and row 6 make test leg are CI-attributed by maintainer decision (D-00j halts); sign-off box stays unticked
- [Phase 47]: 47-06: Option 1 for the pre-commit red on SHA-A0 (SHA-A1 pushed over it); accept-X for the uppercase tick; redate to 2026-10-26; the maintainer's re-confirmation SHA-A4 72e13f95 is the final head and plan 47-07 reads its checks on SHA-A4 where it says SHA-A2
- [Phase ?]: [Phase 47] 47-07: Task 2 proceed (2026-10-26T16:49Z); Task 3 reply `tagged ...` (20:32Z) carried a 41-character first token (stray leading 7), recorded verbatim and not used; plan-text SHA-A2 read as SHA-A4 72e13f95
- [Phase ?]: [Phase 47] 47-08: tag v0.11.0 (77a37c18) peels to merge commit 70657919 = PR #59 merge_commit_sha = origin/main, parents df24689c and SHA-A4; release.yml run 40396108246 green, publish job success; 12 of 12 crates at 0.11.0 unyanked with trustpub_data naming DF3NDR/paladin-dev-env; chore/47-close cut from the merge commit and not pushed
- [Phase 47]: v0.11.0 released: tag `v0.11.0` on merge commit `70657919beadbd341a72be2d0107080017a803f0`, release run `40396108246`, 12/12 crates registry-verified at `0.11.0` (unyanked, sparse-index re-read 2026-10-26T20:53Z), Trusted Publishing — D-16: SHIP-07 was ticked on the live sparse-index table (12 of 12 derived crates at 0.11.0, yanked false, trustpub_data naming run 40396108246), not on the merge, the tag or the workflow log.
- [Phase 47]: Phase 47 ends at ready-to-close: 47-READY-TO-CLOSE.md written, chore/47-close pushed and docs-only PR #60 opened on the maintainer's push ok; audit, milestone close and merge stay the maintainer's — D-17 and D-00l: the last plan ends at a hand-off; SC1-SC3 observed TRUE from evidence (SC3 under B1=a), the Coverage Lines figure and the B-1 read stated as open
- [Phase ?]: 52-01: semver loop lives in tested scripts/check-semver.sh (zero-check tripwire); allowlist gate runs from required cargo-deny job; --release-type patch fixed by flag
- [Phase ?]: Phase 52-02: release body travels file -> jq --rawfile -> stdin; one shared scripts/release-body-fit.sh (limit 125000, reserve 5000, code points) truncates at a line boundary with sentinel footer; empty limit var means default
- [Phase 52]: 52-03: the 20 OSV-Scanner advisory alerts are out of the CodeQL register's scope (17 governed by SECURITY-EXCEPTIONS.md, 3 ungoverned left open for the maintainer)
- [Phase 52]: 52-03: 31 new CODEQL-DISMISSALS rows merged in ascending alert_number order between existing blocks; no alert fixed; platform dismissal commands handed over as H-04
- [Phase ?]: 52-04: finalize size guard re-fits only the curated section to limit minus the artifact part via the shared release-body-fit.sh; artifact sections never altered
- [Phase ?]: Phase 52-05: all six crate-wide semver allow tables (28 lines) removed after measuring 12/12 packages at 223 checks pass vs 0.11.0 (--release-type patch); MIGRATION.md section 11 opened with an empty X-10 register and the allowlist gate reads only 11.2 to 11.3
- [Phase ?]: 52-06: Publish Dry Run runs on every PR; End-to-End Tests runs on main pushes and release-labelled PRs (labeled activity type); committed ruleset record is 46 contexts, live PUT is maintainer handover H-01 (PD-06-1 awaiting user confirmation)
- [Phase 52]: 52-07: semver red proof via leaf-crate cron_field_count pub->pub(crate); local RED-PROVEN (222 pass/1 fail, function_missing); CI red job capture and remote scratch-branch delete handed over as H-03 (gh pr create refused, GraphQL 403)
- [Phase ?]: 52-08: PD-06-1 confirmed by the maintainer; Semver Checks (vs v0.11.0) is a required check, live ruleset 20868126 carries 46 contexts equal to the committed record
- [Phase ?]: 52-08: H-06 local make coverage deferred to the maintainer (Docker-capable machine); todo stays pending, resolves_phase 56, no coverage gate changed
- [Phase ?]: 53-01: RunReadScope::permits_thread is the one thread-owner rule (latest run by submitted_at, run_id tiebreak); ensure_thread_visible calls it only with Some(&latest) so thread birth stays open
- [Phase ?]: Phase 53-02: only RunRepositoryError::Serialization is hideable via redact_store_error_for_scope; UnknownSchemaVersion and all other errors stay the fixed 'run store error' 500 for every role (D-09)
- [Phase ?]: 53-03: Admin scope skips the owner lookup in load_visible_thread; only DELETE (busy check) loads runs for an Admin and answers the fixed 'run store error' 500 (PD-03-5)
- [Phase ?]: 53-03: waypoint Serialization/SchemaVersionUnsupported are hideable (tenant gets the missing-thread 404); Backend/NotFound stay the fixed 'thread store error' 500 (PD-03-3)
- [Phase ?]: [Phase 53-04] GET /v1/threads for a Tenant scope walks list_threads in batches of 100 with a 500-thread examination cap, per-thread latest-run lookup (scope All) plus permits_thread; cursor is the last examined thread's last_updated_at; a Serialization owner lookup is skipped and logged, other run errors are the fixed 500; Admin keeps the single limit+1 fetch
- [Phase ?]: 53-05: get_job keeps its #[utoipa::path] annotation (generated public __path_get_job item) and gets no #[deprecated]; get_job_scoped is routed with operation_id get_job so openapi.json stays byte-identical; JobStore owner (agent id, tenant) lives in private Inner.owners
- [Phase ?]: Phase 53-06: Treasurer refusal fires exactly on Treasurer configured AND no engine factory (D-00f); no config escape hatch; factory-present-but-with_treasurer-dropped is an accepted residual
- [Phase ?]: Phase 53-06: schedule creator scan runs whenever the schedule repository exists, is read-only and advisory, emits at most one warn and one error
- [Phase ?]: Phase 53-06: reserved literals open-access/unattributed (key name, key tenant, bearer tenant) and anonymous (key name) are refused at config load, exact and case-sensitive
- [Phase ?]: ADR-0063 (Phase 53-07): threads belong to the tenant of their latest run, jobs to their enqueuing agent id and tenant; amends ADR-0054; get_job #[deprecated] deferred to next minor
- [Phase ?]: Phase 53-08 (2026-10-31): v0.11.1 patch decided, every change additive. Evidence: local reproduction of Semver Checks (vs v0.11.0) with scripts/check-semver.sh at --release-type patch on head 4ef9ddd2a, 223 checks: 223 pass, 31 skip for all twelve packages, 0 failure(s); MIGRATION.md 11.2 has no row, allowlist has no entry. Recorded in .planning/phases/53-tenant-isolation-closure/53-VERSION-DECISION.md
- [Phase 53]: UAT (2026-10-31): AC-N5 passed on an owner read of the shipped wiring (the no-op-factory blind spot stays a code-review control); review warnings WR-01, WR-02, WR-03 carried as open WINDOWS rows 85-87 (proposed owner Phase 55), WR-04 and WR-05 recorded as ADR-0063 residuals
- [Phase 54]: UAT (2026-11-01): 3/3 passed — WR-02 (`openai-api-key: "REPLACE_ME"` in `k8s/server/secret.yaml.example`) accepted as residual AR-54-03 (SC1/FIRST-01 cover `http.auth.api_keys[].key` only); the kind cluster apply (Secret-mounted config, `defaultMode 0440`, `fsGroup 65532`) and the compose step-2-before-step-1 path (WR-03) verified by the maintainer with real Docker/kind. CR-01 (`make release` runs no version-string gate before the tag) carried to Phase 58 as a pre-tag step.
- [Phase ?]: Phase 54-01: HTTP API keys containing '${' are refused at config load (substring check, any auth state, before tenant/duplicate checks); message names the entry, never the value or variable (FIRST-01)
- [Phase ?]: 54-03: prose banners naming the current release made version-free (PD-03-1) so only gated pattern classes carry a literal workspace version
- [Phase ?]: 54-04: present-tense version claims and deployment image tags made version-free (X.Y.Z placeholders matching release.yml), not bumped; architecture-decisions.md swept not exempted
- [Phase ?]: 54-05: crate corrections under [Unreleased] ### Fixed; ADR-0056 amended in place citing ADR-0057; node-cache key/rotation text lives in fault-tolerance.md and is linked elsewhere
- [Phase ?]: 54-06: k8s server config delivered as the config.yml key of paladin-server-secrets (items projection, defaultMode 0440); configmap.yaml retired; WORKSPACE_VERSION tags docker-build-server
- [Phase ?]: [Phase 54-07]: check-version-strings status order NO_VERSION, STALE_EXEMPTION, ZERO_FILES, CLASS_UNMATCHED, MISMATCH, OK with every named failure and finding listed; image candidates are v?digits.digits tags compared exactly
- [Phase ?]: 54-08: footer [Unreleased] compare link deleted, not retargeted; release comments made count-free; release.toml records cargo-release 1.1.7 (D-15, D-16, D-17)
- [Phase ?]: Phase 54-09: FIRST-08 reconciled by dated note, not by padding the facade snapshot; UnsupportedParameter and check_reasoning live in paladin-ports, guarded by make check-semver (WINDOWS row 59 precedent)
- [Phase ?]: 55-01: check-no-stub-panics gate has no exemption list; out-of-line #[cfg(test)] mod files are exempt by transitive resolution only; refusal text for ContentListRepository is one helper per adapter file
- [Phase ?]: 55-02: a model unpriced at dispatch records Halted with HaltReason::ModelUnpriced { model } (resumable), not Failed; ADR-0057 amended in place (FIXES-03)
- [Phase ?]: 55-03: LocalRunTokens::is_registered is #[cfg(test)] pub(crate) and polled every 5 ms inside the 30 s timeout (not a watch/Notify signal); the four commander happy-path tests are kept, each pointing at its relocated error-path test
- [Phase ?]: Phase 55-04: SELF_HOSTED_PROVIDERS is the single pub(crate) pair in reasoning.rs; claude-opus-4-8 keeps reasoning: false with the comment and pin citing the 50.1 and 51 probe rows (D-06 as reconciled); paladin-llm single-feature clippy fixed by cfg gates only (D-10)
- [Phase 55]: 55-05: unversioned field history is EntityNotFound (repository), purge stays Ok(0); collection count changes carry Uuid::nil() sentinel; no VersioningError variant or pub item changed — D-03 as reconciled: least-destructive fences, removal is a next-minor semver decision
- [Phase ?]: 55-06: ingest_content refuses with a named Err after validation (no shipped caller of the no-op); Subtask expected_output is empty and the prompt omits the empty line (D-03)
- [Phase 55]: Phase 55-07: FIXES-04 re-homed to the next minor (D-05); RunStatusValue and Assertion are exhaustive public enums of published paladin-eval, so the variant is major at the patch gate; recorded in rustdoc, schema descriptions, guide, REQUIREMENTS v2 Eval harness, ROADMAP SC2, inventory B5 and phase deferred-items.md
- [Phase 55]: 55-08: paladin-ports rustdoc suppressions removed; 187 diagnostics (170 unresolved, 17 redundant) fixed at source, 0 under both bars (CIREL-08, D-11)
- [Phase 55]: 55-09: the run engine's PaladinPort is injected only through the pub(crate) EnginePortSource seam (override Some only in run_api_wiring tests); the boot warning for unpriced reachable models is advisory, covers the code registry plus stored agent-kind assistants, and logs under paladin::pricing without refusing boot — D-07 and D-08 as reconciled: no public item added, B4 never refuses, three unchecked classes named in the warning and the guides
- [Phase ?]: Phase 55-10: paladin-llm, paladin-storage and paladin-notifications crate-level rustdoc suppressions removed; feature-gated, private and pub(crate) link targets are plain code spans, unconditional ports/core targets get explicit paths; the workspace default bar unifies features so per-crate default, all-features and no-default-features runs are also checked (CIREL-08, D-11)

### Pending Todos

Four pending todos were acknowledged as deferred at the v0.11.0 close (see *Deferred Items*):

- `todos/pending/2026-08-13-verify-local-coverage-reproduction.md` — user-owned; walk the
  documented `make services-up` → `make coverage` procedure on a Docker-capable machine (now
  against the RustFS dev stack) and confirm it reproduces the CI figure (now 92.74 % on the
  v0.11.0 merge commit, not the 82.39 % the todo quotes). `recheck_by: 2026-10-16` has passed;
  carried since the v0.8.0 close.

- `todos/pending/2026-10-13-review-content-data-orchestration-base-items.md` — review the
  unfinished content / general data orchestration system before removing any of the five base
  items Phase 48 retained (ENTITY-04, D-05, ADR-0060).

- `todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` — web3sec moves once, straight to
  `v0.11.0`, with `MIGRATION.md` section 10 / §9.8 and `47-HANDOFF-web3sec.md` as the spec; stays
  pending until the consumer reports back (Phase 49 D-15). Owner: repo maintainer (relay).

- `todos/pending/2026-10-20-openai-responses-api-for-pro-tier.md` — add OpenAI Responses API
  support so `gpt-5.5-pro` and siblings are usable (Phase 50.1 seed probe finding).

### Blockers/Concerns

**Phase 54 close (2026-11-01, 54-VERIFICATION `passed` 5/5, 54-UAT 3/3, 54-SECURITY 0 open):**

- **Pre-existing, environment-specific test failure** — `run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` fail in the sandbox (`the run never reached a terminal status`, 10 s deadline) on base `8b23cb4c5` too; CI was green at the PR #62 merge. No Phase 54 change touches that file. Phase 55 TESTS-01 (hermetic scripted port) is the owner.
- **CR-01: `make release` commits and tags without running `check-version-strings.sh`** (CONTEXT D-12 chose check-only). Phase 58's bump PR must carry the string edits and run the gate before the tag; recorded in 54-VERIFICATION `deferred` and 54-FIRST-EVIDENCE "Carry-forward to Phase 58".
- **Review follow-ups (advisory, not WINDOWS rows):** WR-01 copy-paste Cargo snippets name non-existent features (`storage-sqlite`, `llm-openai`); WR-04 the Treasurer webhook `secret` placeholder is not refused; WR-05 Python < 3.11 traceback from the gate; IN-02 worker pods mount admin API keys they never use. Cheap fixes for the next doc pass or Phase 57.
- **Phase 54 work is on `claude/laughing-dirac-e0h2ax`, not yet merged to `main`** — a PR is the maintainer's step before Phase 55 builds on it.

**Phase 53 close (2026-10-31, 53-VERIFICATION `passed` 5/5, 53-UAT 2/2, 53-SECURITY 0 open):**

- **WINDOWS rows 85-87 are open** (53-REVIEW WR-01…03, owner-accepted follow-ups): a newer-schema run row
  fails every non-admin `GET /v1/threads` with a 500 (row 85); a corrupt older run row locks the owner out
  of their thread because the latest-run lookup decodes `limit + 1` rows (row 86); thread ownership
  transfers to whoever submitted the latest run, including an Admin fork or an unattributed schedule run
  (row 87). Proposed owner Phase 55 (correctness fixes); `open_count` is now 16, and the `v0.11.1` exit
  criterion needs 0, so Phase 55 or Phase 57 must fix or waive them.

- ~~**Phase 53 work is on `claude/laughing-dirac-e0h2ax`, not yet merged to `main`.**~~ — **merged by the
  maintainer** (PR #62, merge commit `8b23cb4c5` on `main`, 2026-10-31); the branch was restarted from that
  merge for Phase 54.

**Phase 52 close (2026-10-30, 52-VERIFICATION `passed` 5/5, 52-UAT 3/3, 52-SECURITY 0 open):**

- ~~**PR #61 is open, not merged**~~ — **merged by the maintainer** (merge commit `84686acfb` on `main`,
  2026-10-30; head `dcaf6af26`, 46 required contexts incl. the new `Semver Checks (vs v0.11.0)`). `main`
  now carries the armed gates; later phases keep working on `claude/laughing-dirac-e0h2ax` and merge
  `main` back in before the next PR.

- **Release-body size guard has never run live** (D-12) — `scripts/release-body-fit.sh` and the create/finalize
  guards are proven only by the stubbed harnesses and the >131,072-byte regression; the first live use is
  the `v0.11.1` tag in Phase 58. Read that release's body size and footer at the tag.

- **Three OSV-Scanner advisory alerts are ungoverned** (alerts 23, 24, 25 — `D-52-03-02` in
  `52-ci-release-pipeline-repair/deferred-items.md`): no `SECURITY-EXCEPTIONS.md` row, `.cargo/audit.toml`
  or `deny.toml` entry; left open for the maintainer by design. The other 17 OSV alerts are governed by rows.

- **News API key travels in the URL and `reqwest` error text can echo it** (`D-52-03-01`,
  `news_api_fetcher.rs`; CodeQL alert 54 itself is a false positive) — a Phase 55 correctness candidate:
  header-carried key, URL stripped from the mapped error, `Policy::none()`.

- **Label events re-run the whole `ci.yml`** including the ~63-minute Docker build (PD-06-3) — documented,
  not engineered around; apply the `release` label once.

- **`CODEQL-DISMISSALS.md` rows 40 and 49 carry stale line anchors** (left per D-14; logged in
  deferred-items) — fix when those files are next touched.

- **H-06 (local `make coverage` reproduction) deferred again** to the maintainer on a Docker-capable
  machine; the pending todo stays with `resolves_phase: 56`.

**v0.11.1 roadmap-time findings (2026-10-27, measured against the tree while drafting the roadmap):**

- ~~**Phase 53, verify first:**~~ — **done in Phase 53** (plans prove-then-lock, `53-TENANT-EVIDENCE.md`). Phase 40's `40-REVIEW-FIX.md` already records WR-01…WR-04 (TENANT-04, -05, -06) as fixed
  (`ensure_thread_visible` in `submission.rs`, tenant check inside `RunSubmissionService::cancel`, a fixed run-store
  error message in `run_controller.rs`); the carried-gaps inventory lists them open. Plans prove-then-lock those three.

- ~~**Phase 53, decision point:**~~ — **decided in Phase 53: `v0.11.1` patch, every change additive**
  (`53-VERSION-DECISION.md`). If the thread-tenant fix (or any security change) forces a persisted or public-surface
  break, the milestone becomes `v0.12.0`; record the decision before Phase 58 opens.

- **Phase 55, FIXES-01 premise:** `content_filtering_service.rs` is commented out of `services/mod.rs` and is not valid
  Rust, so its `todo!()` is unreachable; the reachable stubs are twenty `todo!()` bodies in the SQLite and MySQL
  `ContentListRepository` impls in `paladin-storage`.

- **Phase 55, FIXES-04:** `paladin-eval`'s `RunStatusValue` is not `#[non_exhaustive]`, so adding `Cancelled` trips the
  patch-level semver gate Phase 52 arms; settle shape-or-break at discuss-phase.

- **Phase 56 depends on external inputs** (the maintainer's runs, the web3sec return, a Docker-capable machine for
  DOGF-05) that the roadmap cannot schedule; an unreturned DOGF-02 is held or re-homed with an owner and date.

- **Phase 57:** `open_count: 0` also needs rows 64-66 (accepted ADR-0057 contracts) and 77 (Grok deferral) waived with a
  reason or re-homed; no requirement names them beyond REG-02's closing clause.

**No blockers at the v0.11.0 close (2026-10-27).** All 15 phases `passed`; audit `tech_debt`
with 0 gaps, 8/8 seams, 8/8 flows; `WINDOWS.md` `open_count: 13` (the rows are dispositioned as
accepted debt with owners in `milestones/v0.11.0-MILESTONE-AUDIT.md`, not closed). The per-phase
close notes this section carried for Phases 38-51 are archived with their phases
(`milestones/v0.11.0-phases/`); the v0.10.0 close note is preserved in this file's git history at
`a49200da`. Open concerns carried into the next milestone:

- **Coverage is CI-attributed, not locally measurable** — this devcontainer has no Docker; the
  82 % floor (ADR-0006) is read from the CI `coverage` job (90.44 % at PR #56; 92.74 % on the
  v0.11.0 merge commit, read at the Phase 47 verify step through the GitHub job-logs API, which
  reaches the log host the sandbox's own egress cannot). The local reproduction walkthrough is the
  pending user-owned todo above.

- **Codecov upload is failing silently** — on the v0.11.0 merge-commit run (40368801098, job
  121733645613) the `Upload coverage to Codecov` step concluded `success` while the Codecov CLI
  logged `Token required - not valid tokenless upload` (`Token length: 0`, `fail_ci_if_error`
  unset), so nothing reaches Codecov and the step can never go red; the CLI also sweeps in
  `.claude/gsd-core/references/edge-probe-fixtures/*/expected-coverage.json` and two
  `.planning/intel/classifications/*.json` as "coverage files". The 82 % floor gate
  (`cargo llvm-cov --fail-under-lines`) is unaffected. Observed at the Phase 47 verify step
  (`47-CI-EVIDENCE.md` Findings, 2026-10-26). Maintainer decision: supply `CODECOV_TOKEN` and set
  `fail_ci_if_error`, or drop the step.

- **Tracing overhead** now accepted at +19.36 % / +16.19 % (Phase 45 re-measure, row 61 waived);
  serialisation is no longer the target — the target-off rows still cost +16-19 %, so any further
  optimisation is in the dispatcher/sink path, or the I/O-bound re-scope Phase 45 D-19 deferred.

- **Webhook SSRF guard does not pin the resolved address** between check and connect — DNS
  rebinding is a documented limitation (`src/application/services/run/webhook/ssrf.rs` module docs,
  `security.instructions.md`).

- **`/v1/threads/*` routes are not tenant-scoped** (Phase 40 D-14 deferral, T-40-23 transferred,
  WINDOWS.md row 58 open) — threads carry no tenant, so an authenticated principal can list any
  thread and read its Waypoint state/history although that tenant's `/runs` routes answer 404; the
  run-side mutation path (`POST /runs {thread_id}`, fork) is closed by WR-01's
  `ensure_thread_visible`. Closing condition: record a tenant on the thread and route every
  `/threads/{id}*` handler through one shared gate like `load_visible_run`. Owner: Phase 41 planning
  if allowance enforcement needs thread ownership, else a v0.11 hygiene phase.

- ~~**Terminal MinIO pin**~~ — **resolved by Phase 45 (2026-09-30):** every live configuration
  runs `rustfs/rustfs:1.0.0` (manifest-list digest recorded), `mc` is gone, the 11-case
  `FileStoragePort` contract suite is green in CI run 36751442387 and the k8s smoke pod is Ready
  (ADR-0055). The image is still a single third-party pin; bump it deliberately.

- **`cargo-semver-checks` 0.50.0 coverage gap** for inherent-method return-type and tool-coverage
  classes — covered by `MIGRATION.md` §9.2 rows instead (Phases 32/33).

- ~~**Nyquist validation**~~ — **resolved for the eight v0.10.0 phases by Phase 46 (2026-10-24, plans
  46-02..46-04, D-14..D-17):** 22, 24, 29, 30, 34, 36 and 36.1 now read `status: validated` with
  `nyquist_compliant: true`, and 28 is promoted to `nyquist_compliant: true` (D-15); any residual
  gap is listed in each file's `## Gaps`. **Still open:** archived phases 05-21 likewise
  unreconciled. `/gsd-validate-phase <N>` each; coverage TODO, not a compliance failure.

- **Open WINDOWS rows at the v0.11.0 close:** 13 (ids 56, 58, 62-69, 73, 77, 79, per `windows
  status`) — Phase 47 recorded them as not its own (`47-CI-EVIDENCE.md`, 2026-10-25) and released
  through PR #59 without `/gsd-ship`; the milestone audit inventoried every one under `tech_debt`
  with an owner. `/gsd-ship` blocks while `open_count > 0`, so the next milestone's first shipping
  phase either dispositions them or records an override. The audit's two integration warnings ride
  with them: G1 (no aggregate HTTP spend route; LEDGR-04 is per-run over HTTP) and G2 (nothing
  asserts the engine-factory wiring that attaches settlement, spend guard and cancel probe).

- **Phase 47 carried findings** (`47-CI-EVIDENCE.md` D-00d, `47-REVIEW.md`) — **the release-process
  half is resolved by Phase 52 (2026-10-30):** semver re-pinned to `0.11.0` with `paladin-eval` (12/12 crates,
  223 checks pass, zero-check tripwire), `check-migration-allowlist` in the required cargo-deny job, Publish
  Dry Run on every PR and End-to-End Tests under the `release` label, release body via `jq --rawfile` with
  the size guard, 37 CodeQL alerts registered and dismissed, issue #41 closed. **Still open:** the
  `paladin-memory` / `paladin-content` changelog wording (Phase 54/55), stale `k8s/` labels and "ten crates"
  comments (Phase 54's `check-version-strings`). Original text: the
  `paladin-memory` / `paladin-content` crate changelogs say "No functional change" for 0.11.0
  although the persisted-key change (`paladin.node` → `paladin.data`) is one-way — fix in the next
  patch release's changelog, the published crates are immutable; the release-body builder still
  fails above 131,072 bytes (B-1 root cause); re-pin the cargo-semver-checks baseline 0.9.0 →
  0.11.0 and add `paladin-eval`; wire `check-migration-allowlist` into `ci.yml`; run Publish Dry
  Run and End-to-End Tests on PRs; stale `v0.10.0` labels in `k8s/server/worker-deployment.yaml`;
  stale "ten crates" comments; 24 advisory CodeQL alerts on the PR heads, untriaged.

- **Three RustSec advisories carried as exceptions** (RUSTSEC-2026-0367/0368/0369, `zip 3.0.0`
  via the `utoipa-swagger-ui 9.0.2` build-dependency) in `SECURITY-EXCEPTIONS.md`, review date
  2026-12-31 — an exception, not a fix.

- **Bookkeeping drift in the corpus acceptance audit** — the seven §11 judgment-tier sign-off boxes
  and the `v0.10.0` tag box in `.project/v0.10.0/09-program-acceptance-audit.md` are still `- [ ]`
  on disk although 29-UAT recorded the pass and the tag was cut; tick by hand or annotate as
  superseded by §13 (v0.10.1, ticked).

- ~~**`release/*` ruleset bypass** granted temporarily on 2026-09-21 for the v0.10.1 push must be
  removed now that PR #56 is merged.~~ **Removed 2026-09-23** — ruleset `20868128` is back to the
  checked-in shape (`bypass_actors: []`, `current_user_can_bypass: never`); `release/*` branches
  accept one push again, so plan repeat pushes as PRs into the branch.

- **Phase 37 reads incomplete to `init.manager`** (8 of 11 plans with SUMMARYs; 37-09..37-11
  superseded) — expected, documented in MILESTONES.md *Known Gaps*; do not "fix" by fabricating
  SUMMARYs.

- ~~cargo-semver-checks 0.50.0's CI-pinned --baseline-version 0.9.0 job is currently a no-op~~ — **resolved by
  Phase 52 (2026-10-30, plans 52-01/52-05/52-07):** baseline `0.11.0` at `--release-type patch` by flag, 12/12
  packages at 223 checks pass, a scratch-branch leaf break turned the job red, and `Semver Checks (vs v0.11.0)`
  is a required context in ruleset 20868126. Original text: cargo-semver-checks 0.50.0's CI-pinned --baseline-version 0.9.0 job is currently a no-op for every one of the 11 checked packages -- confirmed 2026-09-26 during Phase 38 plan 38-09's closeout measurement. The in-tree workspace version (0.10.1, the last real release) already differs from that two-milestones-old baseline by a pre-1.0 "major-equivalent" bump, so the tool skips all lint evaluation (0 checks, 254 skip) unconditionally. This will remain true for every plan in every phase of the v0.11.0 milestone until the version is bumped at the eventual release commit. Not fixed in 38-09 (ci.yml is not in that plan's files_modified) -- flag for whichever phase next touches ci.yml's semver job or performs the v0.11.0 version bump. **Still a no-op at v0.11.0** (Phase 47 re-seal row 57, 2026-10-25: `0 checks, 254 skip` for 10 of 11 packages against 0.9.0; `paladin-web` attributed to its CI twin); now that `0.11.0` is published the baseline can move to it -- a v0.12 hygiene item.

- Phase 58: 7 crate changelogs lack an [Unreleased] anchor or [0.11.1] section before finalize; the check-version-strings gate must go green in the bump commit -- see 54-FIRST-EVIDENCE.md Carry-forward to Phase 58
- Pre-existing, environment-specific: infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default and build_run_api_persists_run_traces_when_trace_persist_is_set fail in this container (run never reached a terminal status, run_api_wiring.rs:3036; make test reports 1479 passed, 2 failed), identically on base 8b23cb4c5; cause undiagnosed, a later phase should diagnose -- see 54-FIRST-EVIDENCE.md Known residuals

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260912-whj | Regenerate API surface snapshot so ci.yml API Surface Tracking check passes on feature/v0.10.0-web3sec-dogfooding | 2026-09-12 | 786a3ba5 | [260912-whj-regenerate-api-surface-snapshot-so-ci-ym](./quick/260912-whj-regenerate-api-surface-snapshot-so-ci-ym/) |
| 260913-15w | Pin MinIO service image to quay.io last known-good release after Docker Hub minio/minio removal | 2026-09-13 | 06765765 | [260913-15w-pin-minio-service-image-to-quay-io-last-](./quick/260913-15w-pin-minio-service-image-to-quay-io-last-/) |
| 260913-h7l | Replace dl.min.io mc download in ci.yml with checksum-verified pinned GitHub release asset after MinIO retired community downloads | 2026-09-13 | 9d0aa7a0 | [260913-h7l-replace-dl-min-io-mc-download-in-ci-yml-](./quick/260913-h7l-replace-dl-min-io-mc-download-in-ci-yml-/) |
| 260916-h40 | Regenerate API surface baseline for Phase 32 window exports and add a local pre-push API surface gate | 2026-09-16 | cd185c9b | [260916-h40-regenerate-api-surface-baseline-for-phas](./quick/260916-h40-regenerate-api-surface-baseline-for-phas/) |

### Roadmap Evolution

- Phase 15.1 inserted after Phase 15: Git & CI Governance — branch protection, trigger surface, gitflow model, docs/BRANCH_PROTECTION.md (URGENT)
- Phase 17 added: Additional LLM Provider Adapters — provider-selection study (PROV-01) then feature-gated adapters for survivors (PROV-02..04); first forward phase beyond the ingest
- Phase 18 added: Rust SAST: evaluate and adopt CodeQL — new Security Tooling milestone; SAST-01..04 minted at roadmap time
- Phase 19 added: crates.io Trusted Publishing — replace the long-lived CARGO_REGISTRY_TOKEN with OIDC-issued ephemeral tokens; PUB-01..PUB-05 minted at roadmap time
- Phase 20 added: Release Pipeline Recovery — idempotent re-runs on the same tag, a pre-publish gate over tag/manifest/changelog/CI agreement, and a stuck-halfway runbook with a yank policy; PUBOPS-01..PUBOPS-05 minted at roadmap time
- Phase 21 added: Release Artifacts — Curated Release Notes and Attached Distributables (v0.9.0; ARTIFACT-01..06 minted, twenty-second prefix)
- Phase 22.1 inserted after Phase 22: Engine readiness defect and MSRV follow-up (readiness defect from 22-16 audit; MSRV 1.85 vs rmcp-pinned process-wrap decision; green postgres-integration run confirmation) (URGENT)
- Phase ? changed: Phase 22.1 scope grew on 2026-09-03: BUG-04 (resume rebuilds the Frontier from scratch, losing pre-crash edge resolutions) promoted from CONTEXT.md deferred ideas into the phase at the 22.1-05 checkpoint by developer decision; ENG-04 re-opened for the phase; CI evidence to be re-captured on the final head.
- Phase 30 added: Token-Economy Vocabulary & Commissary Anchoring — vocabulary rule, Commissary ADR + mdBook page, Treasurer reservation ADR, `max_tokens` table, Quartermaster purge, clean-break versioning ADR (VOCAB-01..07 minted, twenty-seventh prefix; docs only; source `.project/Milestone_13-Token-Economy/Epic_1`)
- Phase 31 added: Lossless Token Accounting — full `TokenUsage` carried port→`RunFinished`→herald, optional cache/reasoning fields, `from_total` removed from the battalion path, streaming parity (ACCT-01..05, twenty-eighth prefix; keystone; clean break under the X-03 supersession)
- Phase 32 added: Unified Token Primitives — `TokenCounterPort::is_exact`, `Commissary::new` without `is_exact_counter`, legacy `TokenCounter`/`TokenCounterFactory` retired, one shared window resolver with strict mode (PRIM-01..05, twenty-ninth prefix; clean break)
- Phase 33 added: Commissary In-Tree Adoption — RAG through `Commissary::dispense` with shed records + marker, integration-tested production caller, Phase 29 release gates re-sealed (COMM-01..04, thirtieth prefix). Milestone 14 Treasurer (`.project/Milestone_14-Treasurer/`) reserved, not roadmapped.
- Phase 34 added: Documentation Currency Audit — read-only inventory of mdBook / rustdoc / examples gaps against Phases 22-33 (and any v0.9.0 leftovers); scopes Phases 35-36 (2026-09-17, pre-tag release readiness)
- Phase 35 added: mdBook Currency — close every Phase 34 mdBook gap; `mdbook build` + linkcheck green (2026-09-17)
- Phase 36 added: Rustdoc Zero-Warning Bar & Examples Currency — `cargo doc` 73→0 warnings so CI "Check documentation" is green, 14 `--all-features` intra-doc links resolved, `examples/` + `doc-examples` current (2026-09-17)
- Phase 37 added: v0.10.0 Crate Release — re-seal the Phase 29 gates on the final commit, merge to `main`, `release.yml` tags `v0.10.0`, all publishable crates on crates.io at `0.10.0` (2026-09-17)
- Phase 36.1 inserted after Phase 36: Deferred Items Closure — walk the Phase 31/32/34/35 deferred-items registers, WINDOWS.md #36-37 and the two pending todos; fix, waive with reason, or re-home each; bring WINDOWS.md back into agreement with the registers before Phase 37 tags v0.10.0 (URGENT)
- Phase 37.1 inserted after Phase 37: v0.10.1 Patch Release — tag v0.10.0 published 3/12 crates (battalion versioned dev-dep vs CRATES order); maintainer chose recovery option A (URGENT)
- Phase 48 added: Base Entity Rename — `base::entity::node::Node<T>` → `Entity<T>` (payload `node` → `data`; clean break under a Phase-48-only X-03 supersession or a one-release deprecated alias, decided at discuss; serde alias for persisted JSON), unused base-layer items dispositioned, "node" reserved for the graph sense; ENTITY-01..05 minted; **sequenced before Phases 46-47**, added after Phase 44 sealed (owner decision 2026-10-12 after the code survey in `48-CONTEXT.md`, re-verified on the post-44 tree)
- Phase 49 added: Downstream Adoption of v0.10.1 (web3sec Consumer Bump) — Paladin's only consumer moves from pin 35fd839 onto the v0.10.1 tag in one coordinated step (Milestone 13 §5, no shims); B-1/B-2/B-3 adopted consumer-side with the D-2 token-usage mapping decided; Paladin gains a compile-only consumer-surface muster and a MIGRATION.md consumer note (ADOPT-01..05 minted, from the web3sec handoff filed as 49-HANDOFF-web3sec.md); **sequenced before Phases 46-47**, the first of a series of consumer-driven phases (owner decision 2026-10-14)
- Phase 50 added: Provider Parity & Configuration-Driven Model Swap — any agent movable between the nine LLM providers by config only, or an early build error naming provider/model/parameter: OpenAI empty-user-message defect fixed first, parameter support per provider and per model (ADR amending 0004), config-driven provider construction as a library API with all nine adapter/config pairs on the facade, dated per-model profile registry feeding Commissary, response/error parity (empty output, typed refusal, usage cap, cache-write, reasoning text, stop sequences), per-step fallback models, nine-adapter conformance + live smoke + mdBook "Switching providers" page (PARITY-01..08 minted, from the second web3sec handoff filed as 50-HANDOFF-web3sec-parity.md; D-1..D-4 settled at discuss); **sequenced after Phase 49 and before Phases 46-47** (owner decision 2026-10-14)
- Phase 51 added: Adjustable Reasoning Effort — a provider-neutral `ReasoningEffort` on `LlmRequest`, `PaladinData`, the YAML agent definition and `llm.<provider>` config (request → agent → provider → vendor default), translated exactly to each vendor's native wire form or refused before the HTTP call with `LlmError::UnsupportedParameter`; compile-enforced `ReasoningWire` per compat preset, model-aware mapping in the native adapters, levels from Phase 50's model-profile registry, Kimi silent-no-op regression test, requested/effective effort on the response and trace stream, `Budget(n)` bounds, live per-vendor invalid-effort probes, ADR amending 0004, mdBook "Reasoning effort" page (REASON-01..07 minted, fortieth prefix; source `51-HANDOFF-web3sec-reasoning-effort.md`; runs after 50, before 46-47; ships in v0.11.0)
- Phase 50.1 inserted after Phase 50: Consumer-Surface Readiness for v0.11.0 — the Aegis family (`Aegis`, `ErrorHandlerSpec`, `TimeoutPolicy`, `StateDelta`, `NodeError`, `NodeErrorSource`, `TimeoutKind`, `NodeId`, `Transience`) on the facade so §9.8 DR-1/2/8/9 compile without `paladin-ai-core`; one `*Config::new(..)` shape plus `with_timeout_seconds` / `with_base_url` on the shared knob builders as a registered clean break (resolves the 2026-10-16 harmonisation todo by owner decision 2026-10-18); the consumer-surface muster extended to handoff Appendix B with a red proof; four MIGRATION §9.8 gaps (pre-`v0.10.0` P31/P32/P33 pointers, lost in-adapter 429 retry + Cadence recipe, decorator `model_profile` forwarding, adapter-wide context window following the seeded default); a facade pacing-only path; `claude-opus-4-8` / `deepseek-v4-pro` seeds under the seed rule (D-E: listing-only evidence conflicts with `seeded.rs`'s measured-end-to-end rule — probe re-run, listing tier, or decline). SURFACE-01..06 minted, forty-first prefix; source `50.1-HANDOFF-web3sec-consumer-surface.md`; runs after 50, before 51, because 51 extends the same nine `*Config` types (owner decision 2026-10-18, filed 2026-10-19)
- Milestone v0.11.0 closed 2026-10-27: Phases 38-51 archived to `milestones/v0.11.0-*`; ROADMAP.md collapsed; next phases start at 52
- Milestone v0.11.1 "Dogfood Cleanup" roadmap created 2026-10-27: Phases 52-58 — CI & Release Pipeline Repair (52), Tenant Isolation Closure (53), First-Run Usability (54), Correctness Fixes / Test Hygiene / Lint Bar (55), Dogfood Findings (56, scope filled from the findings log), Register Hygiene Close-Out (57), Crate Release (58); 53 requirements mapped (CIREL-08 placed in 55 beside TESTS-03/04); integer phases only, no insertions yet

## Deferred Items

### Acknowledged at v0.11.0 milestone close (2026-10-27)

**Verification overrides: 0** — all 15 phases (38-51) report `phase_complete: true` and
`verification_status: passed` (`init.manager` `all_phases_verified: true`); 174/174 plans carry a
SUMMARY. **Requirement gaps acknowledged: 3** — ADOPT-01…03 superseded by the owner's 2026-10-14
scope cut, deliberately unticked (MILESTONES.md *Known Gaps*). **Open artifacts acknowledged: 9**
at the pre-close `audit-open` run: three `debug/` sessions, two UAT rows and four pending todos.
Closeout type `override_closeout`, on the strength of those artifacts rather than any unverified
phase.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| debug | `kimi-fallback-profile-stale` | `diagnosed` (root cause found; session scoped `find_root_cause_only`). **Fixed by Phase 50 plan 50-20** — unseeded Kimi ids now send no temperature/top_p/penalties; WINDOWS row 76 `fixed`. The session file is left at `diagnosed` as the record of the investigation; move to `debug/resolved/` when convenient | Phase 50 UAT (2026-10-17); acknowledged v0.11.0 close |
| debug | `openai-max-completion-tokens` | `diagnosed`. **Fixed by plan 50-19** — the OpenAI adapter sends `max_completion_tokens` on every chat and vision body; WINDOWS row 75 `fixed` | Phase 50 UAT (2026-10-17); acknowledged v0.11.0 close |
| debug | `vendor-token-usage-fidelity` | `diagnosed`. **Fixed by plans 50-21..50-23** — every compat preset, OpenAI, DeepSeek and Gemini parse the vendor's own total on both paths and record a disagreement under `paladin.usage_unreconciled`; the Grok reasoning undercount is priced as xAI bills it; WINDOWS row 74 `fixed`. Remainder (c) — Grok's `cost_in_usd_ticks` / `num_sources_used` unparsed — is row 77, open | Phase 50 UAT (2026-10-17); acknowledged v0.11.0 close |
| uat | Phase 41 `41-UAT-RUNBOOK.md` reads `[unknown]` to the audit tool | Not a gap: the runbook is the operator walkthrough script, not a UAT record; `41-UAT.md` is complete 4/4 (operator sign-off 2026-10-12) | Acknowledged v0.11.0 close |
| uat | Phase 42 `42-UAT.md` `[passed]`, 0 pending | Not a gap: listed by the tool because its status token is `passed` rather than `complete`; 42-VERIFICATION `passed` under the operator's accepted ALLOW-03 override of 2026-10-07 | Acknowledged v0.11.0 close |
| todo | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Walk `make services-up` → `make coverage` on a Docker-capable machine (RustFS stack) and confirm the CI figure reproduces (92.74 % on `70657919`). `recheck_by: 2026-10-16` passed unactioned | v0.8.0 close; re-acknowledged v0.9.0, v0.10.0 and v0.11.0 |
| todo | `2026-10-13-review-content-data-orchestration-base-items` | Open — owner: repo maintainer. Review the unfinished content / data orchestration system before removing any of the five base items Phase 48 retained (ENTITY-04, D-05, ADR-0060) | Phase 48 (2026-10-13); acknowledged v0.11.0 close |
| todo | `2026-10-14-web3sec-single-bump-to-v0-11-0` | Open — owner: repo maintainer (relay to web3sec). The consumer's single move to `v0.11.0` against `MIGRATION.md` section 10 / §9.8 and `47-HANDOFF-web3sec.md`; supersedes ADOPT-01…03 (Phase 49 D-15). Stays pending until web3sec reports back | Phase 49 scope cut (2026-10-14); acknowledged v0.11.0 close |
| todo | `2026-10-20-openai-responses-api-for-pro-tier` | Open — owner: repo maintainer. OpenAI Responses API support so the pro tier (`gpt-5.5-pro` and siblings) is usable; a candidate requirement for the next milestone | Phase 50.1 seed probe (2026-10-20); acknowledged v0.11.0 close |
| requirement | ADOPT-01…03 | Superseded by the owner's 2026-10-14 scope cut; amend-at-source note in `milestones/v0.11.0-REQUIREMENTS.md`, traceability rows `Superseded (scope cut 2026-10-14)` | Phase 49 (2026-10-14) |
| debt | Audit `tech_debt` register (13 open WINDOWS rows; G1/G2; the Phase 47 carried findings; the Codecov silent failure; REASON-06 override-only / `Unmeasured` policies; the three `zip` RustSec exceptions; five SUMMARY-frontmatter hygiene omissions; the SSRF resolve-then-connect limitation) | Inventoried with owners in `milestones/v0.11.0-MILESTONE-AUDIT.md`, not duplicated here | v0.11.0 close |
| testing | Nyquist validation unreconciled for archived Phases 05-21 | Still open (coverage TODO, not a compliance failure); Phases 22-37.1 were reconciled by Phase 46 and Phases 38-51 by the 2026-10-27 `/gsd-validate-phase` sweep | v0.10.0 close; re-acknowledged v0.11.0 |

### Acknowledged at v0.10.0 milestone close (2026-09-23)

**Verification overrides: 0** — all 19 phases (22-37.1) report `verification_status: passed` with
`behavior_unverified: 0`. **Phase-completion override: 1** — Phase 37 reads `phase_complete: false`
because plans 37-09, 37-10 and 37-11 were superseded by Phase 37.1 (dated notes in the plan files,
no SUMMARY) after the `v0.10.0` tag published 3 of 12 crates; its `37-VERIFICATION.md` is `passed`
5/5 with SC4 stated as superseded. **Requirement gap acknowledged: 1** — SHIP-05 superseded by
SHIP-06, deliberately unticked (MILESTONES.md *Known Gaps*). **Open artifacts acknowledged: 2** —
both pending todos, each already dispositioned as deferred past v0.10.0 by Phase 36.1 (CURR-20)
with `recheck_by: 2026-10-16`. Closeout type `override_closeout`.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| todo | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Walk `make services-up` → `make coverage` on a Docker-capable machine and confirm the CI figure reproduces (CI now 90.44 %). Carries no `resolves_phase` tag by design | v0.8.0 close; re-acknowledged v0.9.0 and v0.10.0 |
| todo | `2026-09-13-evaluate-rustfs-replacement-for-minio` | Open — owner: repo maintainer; FUT-10 in the archived requirements. The quay.io MinIO pin (quick task 260913-15w) is terminal | Phase 36.1 (2026-09-18); acknowledged v0.10.0 close |
| phase | Phase 37 plans 37-09..37-11 | Superseded by Phase 37.1 — never executed; scope (post-publish verification, milestone-close recording for `v0.10.0`) delivered under `v0.10.1` | Phase 37.1 (2026-09-19) |
| requirement | SHIP-05 | Superseded by SHIP-06; amend-at-source note dated 2026-09-22 in `milestones/v0.10.0-REQUIREMENTS.md` | Phase 37.1 |
| testing | Nyquist validation unreconciled for Phases 22, 24, 29, 30, 34, 36, 36.1 (`draft`) and 28 (`nyquist_compliant: false`) | Reconciled by Phase 46 (plans 46-02..46-04, D-14..D-17): 22 `nyquist_compliant: true`, 24 true, 29 true, 30 true, 34 true, 36 true, 36.1 true, all now `status: validated`; 28 promoted to `nyquist_compliant: true` (D-15). Gaps listed in each file's `## Gaps`. Archived Phases 05-21 remain open (rows below) | v0.10.0 close; resolved Phase 46 (2026-10-24) |
| debt | Audit `tech_debt` register (tracing overhead D-16; `WINDOWS.md` waived rows 23-25, 29-35, 45, 51-55; §11 sign-off boxes; IN-01/IN-02; SSE `done` collapse; three roadmap-level v2 lines; Milestone 14 reserved) | Inventoried with owners in `milestones/v0.10.0-MILESTONE-AUDIT.md`, not duplicated here | v0.10.0 close |

### Acknowledged at v0.9.0 milestone close (2026-09-01)

**Verification overrides: 0** — all 4 phases (18-21) report `phase_complete: true` and
`verification_status: passed`. **Open artifacts acknowledged: 1** (the same user-owned pending
todo acknowledged at the v0.8.0 close, unchanged). Closeout type `override_closeout`, on the
strength of that todo rather than any unverified phase.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Unchanged since the v0.8.0 close: verifies the documented local procedure (`make services-up`, then `make coverage`) reproduces CI's 82.39% on a Docker-capable machine. Deliberately carries no `resolves_phase` tag so a close cannot silently absorb it | v0.8.0 close, re-acknowledged v0.9.0 |
| Testing | Nyquist validation unreconciled for Phases 18-21 | All 4 `VALIDATION.md` files read `status: draft` (NOT-VALIDATED per #2117) — coverage TODO, not a compliance failure. Run `/gsd-validate-phase <N>`. Joins the same open item for archived Phases 05-17 | v0.9.0 close |

Every human-verification backstop the phase verifications declared was closed by recorded UAT
before the close: the Phase 19 crates.io token revocation (operator, 2026-08-28, `19-UAT.md`) and
both Phase 21 checks — out-of-band pull-by-digest and `paladin-cli` execution (user, 2026-09-01,
`21-UAT.md`). The remaining v0.9.0 debt items (CodeQL re-probe trigger, `workflow_dispatch`
publish path, `make publish-dry-run`, dead `upload_url` script output) are inventoried in
`milestones/v0.9.0-MILESTONE-AUDIT.md`, not duplicated here.

### Acknowledged at v0.8.0 milestone close (2026-08-24)

**Verification overrides: 0** — all 14 phases (05-17) report `phase_complete: true` and
`verification_status: passed`. **Open artifacts acknowledged: 1.** Closeout type
`override_closeout`, on the strength of the pending todo below rather than any unverified phase.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Verifies that the documented local procedure (`make services-up`, then `make coverage`) reproduces CI's measured 82.39% on a Docker-capable machine. The CI half is confirmed (run `31727496744` at commit `e9e3267`); the local half has never been walked end-to-end, because no authoring environment in Phase 15 had Docker or `cargo-llvm-cov`, and none is available at close. **Acknowledged rather than closed by design:** the todo deliberately carries no `resolves_phase` tag precisely so that a phase close cannot silently absorb it | v0.8.0 close |
| Testing | Nyquist validation never reconciled for Phases 05-17 | All 13 `VALIDATION.md` files read `status: draft` — seeded by plan-phase, never promoted by validate-phase, so `nyquist_compliant` is not authoritative. Phase 06 has no `VALIDATION.md` at all. Coverage TODO, not a compliance failure (#2117). Run `/gsd-validate-phase <N>` | v0.8.0 close |
| Security | No static taint analysis for first-party Rust | Snyk was measured and removed 2026-08-18 (0 findings on a four-vulnerability Rust probe vs 3 for identical JavaScript). `cargo-audit`/`cargo-deny` scan dependencies; clippy is a lint. **Not deferred indefinitely — owned by Phase 18 (SAST-01…04) in the v0.9.0 Security Tooling milestone** | v0.8.0 close |

The full debt inventory — 25 recorded items across 10 phases, plus 12 open and 4 waived
`WINDOWS.md` rows — is in `.planning/milestones/v0.8.0-MILESTONE-AUDIT.md`, not duplicated here.

### Acknowledged at v0.7.1 milestone close (2026-08-04)

**Verification overrides: 1.** Closeout type `override_closeout`.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Verification | Phase 1 verification timestamp stale | `01-VERIFICATION.md` records `passed` 5/5 at 2026-07-31T16:46:51Z; commit `be2ff05` (2026-08-03) later added `01-04-SUMMARY.md`, so `init.manager` reports `verification_status: stale` / `phase_complete: false`. The commit is documentation-only ("No ADR, measurement, or source changes"); accepted as an override rather than re-verified | v0.7.1 close |
| Integration | Herald not reachable from Campaign, Chain of Command, or the Commander router (audit WARN-01) | Formation and Phalanx wire Herald; the other three carry zero references. `format_battalion_result` is pattern-agnostic so no requirement's text is falsified, but the composite Chain-of-Command developer flow does not compose without the caller invoking a Herald directly. Unassigned — candidate for Phase 6. **Closed 2026-08-05, plan 06-07: adopted under CLOSE-02 and closed by plan 06-02** — see `.planning/ROADMAP.md`'s Phase 6 WARN-01 outcome note and `.planning/ledgers/milestone-02-03.md`'s CLOSE-02 scope section (Epic 22 cross-reference) for the full record | v0.7.1 close |
| Testing | Nyquist validation never reconciled for Phases 1-4 | All four `VALIDATION.md` files read `status: draft` — seeded by plan-phase, never promoted by validate-phase, so `nyquist_compliant` is not authoritative. Coverage TODO, not a compliance failure (#2117). Run `/gsd-validate-phase 1`…`4` | v0.7.1 close |

### Carried from earlier ingest runs

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | Live-provider-API integration tests (Epic 6 task 7.0, 18 subtasks) | **Un-deferred by run 2** — suite ships behind `live-api-tests`; only the skip-vs-fail semantics remain open (VERIFY-06) | Ingest run 1, revised run 2 |
| Testing | CLI end-to-end tests (Epic 9 tasks 13.4-13.6) | **Un-deferred by run 2** — the blocking mock provider shipped (REQ-mock-llm-adapter) along with the Tier-1 CLI suites | Ingest run 1, revised run 2 |
| Testing | Garrison large-conversation perf test (Epic 2 task 9.14) | Deferred — marked future enhancement | Ingest run 1 |
| Testing | Vision and RAG latency targets never measured (single image < 5 s; retrieval < 500 ms p95; extraction < 3 s p95) | Deferred to v2 — no baseline document exists | Ingest run 2 |
| Tech debt | Oversized service file decomposition (2,757 / 2,294 / 1,840 lines) | Waived at Phase 46 -- WINDOWS row 80 (D-18): measured 2026-10-24, `engine/superstep.rs` 16,018 raw / ~5,363 production, `engine/mod.rs` 14,324 / ~3,251, `paladin_execution_service.rs` 10,858 / ~4,419; candidate phase for the next milestone | Ingest run 1 |
| Tech debt | Clone/lock-contention optimization | Waived at Phase 46 -- WINDOWS row 81 (D-19): no bench attributes a regression to cloning or lock wait; re-open on such a regression | Ingest run 1 |
| Tech debt | Single-threaded orchestration scheduler (`orchestration/scheduler.rs`) | Deferred to v2 — `tokio-cron-scheduler` is already a dependency and already adapted in `paladin-storage` | Ingest run 2 |
| Scope | MCP WebSocket transport | Deferred — recorded as a known limitation by the Epic 23 completion summary | Ingest run 2 |
| Scope | Garrison semantic search / vector context retrieval in the CLI path (recency-based selection only) | Deferred — Epic 23 known limitation; superseded in spirit by Sanctum | Ingest run 2 |
| Scope | Grove learning from past routing decisions | Out of scope — Epic 16 NG-3; the release-notes `PerformanceBased` claim is verified absent from the tree | Ingest run 2 |
| Scope | Automatic Garrison-to-Sanctum migration | Out of scope — Epic 11 explicit non-goal | Ingest run 2 |
| Scope | Batch vision API | Out of scope — Epic 20 NG-6; concurrency is a Battalion concern | Ingest run 2 |
| Scope | Registry multi-tenancy, persistence, distribution | Out of scope — Epic 22 explicit non-goals | Ingest run 2 |
| Scope | ~~Milestones 9-12 feature work~~ | **Closed by run 5** — all four milestones ingested and verified shipped (M9 100%, M10 100%, M11 92.0%, M12 99.0%). Recorded in the 120-row *Milestone 9-12 as-shipped ledger*, not deferred | Ingest run 1, narrowed runs 2-4, closed run 5 |
| Tech debt | **D1 — `src/core/` re-export shims** (6 files, 49 facade importers) | **KEEP, by decision** — removal means rewriting 49 files and preserving `platform/mod.rs`'s maneuver/parser injection, which carries real logic. Becomes debt only if a no-alias policy is adopted (ARCH-04) → FACADE-02 | Ingest run 4 |
| Tech debt | **D2 — mis-layered `src/core/platform/manager/` services** (`content_service`, `event_manager`, `user_service`) | Deferred, medium/medium — partly overtaken: reconciliation commit `6704807` found "no user-service split was needed" because `UserServiceTrait` and the DTOs already live in `paladin-core`. Overlaps the run-3 v2 `user_service` relocation item; do not plan twice → FACADE-02 | Ingest run 4 |
| Tech debt | **D3 — entangled Paladin services** (`planning`/`prompt_generation`/`temperature`/`handoff`, ~2,750 LOC) | **KEEP for now**, high/high — needs the `paladin_builder.rs` / `paladin_execution_service.rs` coupling untangled first, and the targets (`paladin-battalion`, `paladin-llm`) are leaf-to-leaf edges gated on HARD-05 → FACADE-02 | Ingest run 4 |
| Tech debt | **D4 — `content_ingestion_service.rs` placement** (~1,211 LOC) | Deferred, medium/medium — M7 Epic 1's PRD listed it as moving to `paladin-content`; the facade kept its own copy. Needs a dependency-coupling review → FACADE-02 | Ingest run 4 |
| Tech debt | **D5 — residual `println!`/`eprintln!`/`dbg!`** | **Verified exact: 17 occurrences across 6 files**, down from ~435 across 36. The register's own quick win; low/low → FACADE-01 | Ingest run 4 |
| Scope | The `paladin user …` CLI command surface (1,065 LOC, 8 subcommands) | Deferred on purpose — it was declared but **never dispatched**, so it compiled and did nothing. Backend intact; reintroduction is "mostly re-wiring", recoverable verbatim from the M8 removal commit on `chore/facade-cleanup-m8-finish` → FACADE-03(a) | Ingest run 4 |
| Scope | The TensorFlow ML adapter and the `ml` feature flag (636 LOC) | Deferred on purpose — a `#[doc(hidden)]` stub nothing consumed. **Reintroduction condition is the load-bearing part**: a dedicated `paladin-ml` leaf crate, never the facade, with the flag on that crate; `MlPort` stays in the workspace → FACADE-03(b) | Ingest run 4 |
| Scope | A future **content-delivery crate** | Reserved by M7 Epic 1 §4.5.2 as the "correct long-term home" for `file_content_repository.rs`; the file was then deleted and no later document mentions the crate. Carried so the idea is not lost silently | Ingest run 4 |
| Scope | `paladin-arsenal` and `paladin-sanctum` crates | Out of scope — named only by a superseded disposition record that contradicts its own governing PRD. Neither exists; Milestone 9 is 100% complete. Triaging the list is FACADE-04 | Ingest run 4 |
| Tech debt | `paladin-core` / `paladin-ports` dependency allowlists brought back in line with reality (declared 6 and 7; ship 14 and 10) | Fixed -- ADR-0015 (14 / 12 after the 2026-10-24 amendment) and `scripts/check-dependency-allowlist.sh`; WINDOWS row 82 (D-20/D-21) | Ingest run 3 |
| Tech debt | `retry`, `rate_limiter` and `bulkhead` primitives in `src/infrastructure/resilience/`, plus consolidating the retry logic in `mcp_sse_adapter.rs` and `api_content_deliverer.rs` | Deferred — explicitly scoped out by Milestone 6 Epic 4, which shipped the module scaffold only | Ingest run 3 |
| Tech debt | Full `user_service` relocation out of `src/core/platform/manager/` (with `UserServiceFactory`, `user_config.rs`, user CLI commands, user API controller, `SqliteUserRepository`) | Deferred — Milestone 6 Epic 2 scoped it out and flagged it for "a future Epic" | Ingest run 3 |
| Scope | A `paladin-cli` workspace crate | Out of scope — the Milestone 5 overview's target structure named it, the Epic 6 PRD's non-goal rejected it, and the code agrees with the PRD (a `cli` feature plus `[[bin]] paladin-cli`) | Ingest run 3 |
| Scope | MCP feature flags (`mcp-arsenal` / `mcp-transports` / `mcp-stdio` / `mcp-sse`) | Out of scope — eliminated by a dated 2026-04-15 PRD note; Arsenal and its transports compile unconditionally | Ingest run 3 |
| Scope | A `paladin-infra` crate, and a `CircuitBreakerPort` trait abstraction | Out of scope — both explicitly rejected by Milestone 6 Epic 4, which accepted the resulting layering inversion as a pragmatic trade-off inside the facade crate | Ingest run 3 |
| Tech debt | **Deferred-QA Epic 25 — CI/CD pipeline enhancement** (`cli-tests`, `bench-check` and `coverage` jobs, `.codecov.yml`, four Makefile targets, eight deprecated actions, CONTRIBUTING coverage docs) | **Un-deferred by run 5 — verified unbuilt item by item and promoted to Phase 15** (PIPE-01 … PIPE-05). The register's own recommended first epic: "establishes quality gates that validate all subsequent work" | Ingest run 5 |
| Tech debt | **Deferred-QA Epic 26 — documentation and rustdoc** (architecture doc modernization, zero rustdoc warnings in CI, 100% public-API rustdoc, four asciinema demos) | **Un-deferred by run 5 — promoted to Phase 16** (DOCS-02 … DOCS-04). The architecture document is verified frozen at 311 lines with zero of seven newer subsystems | Ingest run 5 |
| Scope | **Deferred-QA Epic 27 — LLM tool calling** (`tools` on `LlmRequest`, `ToolDefinition`, `ToolCall`, `tool_calls` on `LlmResponse`, all three adapters) | **Decision required, not deferred again** — verified entirely absent; it is a **breaking change to the `LlmPort` trait** by the PRD's own admission, both its open questions are unanswered, and Arsenal/MCP already provides tool execution through a different seam → WEB-04. The separable defect (`ProviderCapabilities` over-reporting) is correctable today → WEB-03 | Ingest run 5 |
| Testing | **Deferred-QA Epics 28-29 — platform-services and event-system coverage** (`user_service.rs` ~4.23% → ≥ 80%; the listener orchestrator ~57.83% → ≥ 80%, with concurrency, deadlock, 1000-event-burst and distributed-tracing scope) | **Partially un-deferred by run 5 — promoted to Phase 15** (DEFER-01 … DEFER-03). **Scope real, numbers not**: both module paths are stale and both baselines predate Milestone 9's tests. Blocked on the shared mock infrastructure that does not exist, and `user_service.rs` must be sequenced against M8 deferred item D2 | Ingest run 5 |
| Tech debt | The shared `Send + Sync` mock and async-test infrastructure (`MockUserRepository`, `MockLogPort`, `MockNotificationService`, `MockEventSource`, `MockTriggerExecutor`, Tokio time control) | **Un-deferred by run 5 → DEFER-01.** Named as an unchecked prerequisite by `DEFERRED_COVERAGE.md` and by both coverage Epics; ~6-10 of the 35-45 estimated hours. Placement (`tests/common/` versus the existing `tests/helpers/`) and `mockall`-versus-hand-written are both unanswered | Ingest run 5 |
| Scope | A shared-store `AuthPort` implementation for multi-process serving | **Never deferred — never requirement-ed at all.** M9 Epic 5 §6.1 anticipated it in prose ("a multi-process deployment would later need a shared store") and M12 Epic 7 then shipped `k8s/deployment.yaml`. **No requirement in the 263-document corpus covers it** → WEB-02 | Ingest run 5 |
| Scope | Garrison (memory) and Arsenal (tools/MCP) wiring for HTTP-served agents | Deferred by M12 Epic 2 and restated by Epic 3 — "agents are LLM + prompt only here". **Whether this is planned scope or a permanent property of the topology is undecided**, and the deployment-topologies decision matrix that routes readers between topologies must say which → ORCH-04(b) | Ingest run 5 |
| Scope | Hot-reloading `config.yml`; TLS termination in `paladin-server`; fine-grained scopes beyond `allowed_roles` + admin gate; encrypting config at rest | Out of scope — all four are explicit Milestone 12 non-goals. TLS is a proxy/ingress concern; secrets management is "the operator's responsibility, as with LLM keys" | Ingest run 5 |
| Scope | Benchmark regression **detection** (`critcmp`, `github-action-benchmark`) | Out of scope — Deferred-QA Epic 25 non-goal. Note the inversion: `benchmark-regression-signal` already ships from M7 Epic 3 while the `bench-check` compile prerequisite does not → PIPE-01 | Ingest run 5 |
| Scope | Rewriting the 35 mdbook appendix files | Out of scope — M11 Epic 3 non-goal ("reference/archive material"). **One exception is under decision**: `design-and-architecture.md`, whose relocation into that exempt chapter is precisely why its gap survived → DOCS-02 | Ingest run 5 |

## Session Continuity

Last session: 2026-11-01T17:34:24.006Z
Stopped at: Completed 55-10-PLAN.md
Resume file: None

## Operator Next Steps

- **Phase 54 is complete (2026-11-01):** verification `passed` 5/5, UAT 3/3, security 0 open (32 threats, 3 accepted
  risks), no new ADR. The work is on `claude/laughing-dirac-e0h2ax`; open a PR to `main` before Phase 55 builds on
  it. Next: `/clear` then `/gsd-discuss-phase 55` (Correctness Fixes, Test Hygiene & Lint Bar — settle the
  `RunStatusValue::Cancelled` semver question and the WINDOWS rows 85-87 owner there).

- **Phase 53 is complete (2026-10-31):** verification `passed`, UAT 2/2, security 0 open, ADR-0063 minted,
  `v0.11.1` stays a patch. Review warnings WR-01…03 are open WINDOWS rows 85-87 for a later phase. The work
  merged to `main` (PR #62, `8b23cb4c5`, 2026-10-31). Phase 54 context gathered 2026-10-31
  (`54-CONTEXT.md`); next: `/gsd-plan-phase 54`.

- **Phase 52 is complete (2026-10-30):** verification `passed`, UAT 3/3, security 0 open. PR #61 is merged
  (`84686acfb` on `main`, 2026-10-30); the one maintainer act still ahead is the `v0.11.1` tag in Phase 58. Next: `/clear` then `/gsd-discuss-phase 53` (Tenant Isolation Closure — verify Phase 40's
  `40-REVIEW-FIX.md` fixes first, and record the `v0.11.1`-vs-`v0.12.0` decision there).

- **Milestone v0.11.1 roadmap (2026-10-27):** `.planning/ROADMAP.md` (Phases 52-58) and the
  `REQUIREMENTS.md` traceability table. Phase 56 stays a frame until the findings log has content;
  append maintainer-run and web3sec findings to `.project/v0.11.1/02-dogfood-findings-log.md` as they arrive.

- **Milestone v0.11.0 is closed (2026-10-27).** Archive: `milestones/v0.11.0-ROADMAP.md`,
  `v0.11.0-REQUIREMENTS.md`, `v0.11.0-MILESTONE-AUDIT.md`, `v0.11.0-phases/`; record:
  `MILESTONES.md`; retrospective: `RETROSPECTIVE.md`. `REQUIREMENTS.md` is removed — the next
  milestone mints a fresh one. The close landed on `claude/laughing-dirac-e0h2ax`; merge it to
  `main` through a documentation-only PR like `chore/47-close` (PR #60).

- **Start the next milestone:** `/clear` then `/gsd-new-milestone` (questioning → research →
  requirements → roadmap). New phases start at **Phase 52**. Candidate inputs already on file: the
  four pending todos above (the OpenAI Responses API for the pro tier; the base-item review behind
  ENTITY-04); the audit's G1 (an aggregate HTTP spend route, LEDGR-04's HTTP half) and G2 (assert
  the engine-factory Treasurer wiring); the untenanted `/v1/threads/*` routes (row 58); the
  tracing-overhead bar (rows 35/61, still not met); the Phase 47 carried release-process findings
  (semver baseline re-pin to 0.11.0 plus `paladin-eval`, `check-migration-allowlist` in `ci.yml`,
  the release-body builder's 131,072-byte cap, stale `k8s/` labels); the Codecov token decision;
  the `zip` RustSec exceptions due for review 2026-12-31; and the `v0.11.0-MILESTONE-AUDIT.md`
  `tech_debt` register, which the audit itself recommends reviewing before the next roadmap.

- **Maintainer-only items observed at the close, unchanged:** the Codecov upload step reports
  `success` while its CLI fails for want of a token (supply `CODECOV_TOKEN` and set
  `fail_ci_if_error`, or drop the step); the seven §11 sign-off boxes in
  `.project/v0.10.0/09-program-acceptance-audit.md` are still `- [ ]` on disk; web3sec's single
  bump to `v0.11.0` is the consumer's own next step.
