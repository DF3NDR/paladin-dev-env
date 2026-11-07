---
gsd_state_version: 1.0
milestone: v0.11.1
milestone_name: Dogfood Cleanup (Phases 52-58) — shipped v0.11.1 2026-11-06, closed 2026-11-07
status: Awaiting next milestone
stopped_at: Milestone v0.11.1 closed and archived 2026-11-07; next /gsd-new-milestone (new phases start at Phase 59)
last_updated: "2026-11-07T14:37:15.353Z"
last_activity: 2026-11-07
last_activity_desc: Milestone v0.11.1 completed and archived
progress:
  total_phases: 7
  completed_phases: 7
  total_plans: 84
  completed_plans: 84
current_phase: 58
current_phase_name: Crate Release
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-11-07 after the v0.11.1 milestone close)

**Core value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.
**Current focus:** Planning the next milestone — `/gsd-new-milestone` (questioning → research →
requirements → roadmap). New phases start at **Phase 59**. No active phase, no `REQUIREMENTS.md`.

**Previous milestone:** v0.11.1 "Dogfood Cleanup" closed 2026-11-07 (tag 2026-11-06) — 7 phases
(52-58, integer phases only; no insertion, no extension), 84 plans, 172 tasks, 52/53 requirements
(FIXES-04 re-homed to the next minor by Phase 55 D-05), 11 days from roadmap to tag (2026-10-27 →
2026-11-06; 443 commits on the release PR #64 per GitHub after PRs #61 and #62, `70657919` →
`84686acfb` → `8b23cb4c5` → `251303bcd` → `6082a787e`). Archived to `milestones/v0.11.1-ROADMAP.md`,
`v0.11.1-REQUIREMENTS.md`, `v0.11.1-MILESTONE-AUDIT.md` (status **`passed`** — the first — 0 gaps,
6/6 seams, 4/4 flows, 6/6 pre-tag phases Nyquist-compliant, `tech_debt: []`) and `v0.11.1-phases/`.
Released on one tag: `v0.11.1` on merge commit `251303bcd` (2026-11-06; run `42304251278`, all 12
publishable crates on crates.io at `0.11.1` through Trusted Publishing; `Coverage` 92.86 % on the
release PR head). Closeout type `override_closeout` — 0 unverified phases, 45 open artifacts
acknowledged (all pending todos, 41 of them the dated re-home register) — see *Deferred Items*.
Full record: MILESTONES.md.

**Prior:** v0.11.0 "Treasurer Spend Governance" closed 2026-10-27 (15 phases, 174 plans, 63/66
requirements, 1,126 commits; one tag `v0.11.0` on `70657919`, coverage 92.74 %); v0.10.0 "Durable
Agent Execution Runtime" closed 2026-09-23 (19 phases, 231 plans, 88/89 requirements, 1,678
commits; released as `v0.10.1` on `f7dae267` after the `v0.10.0` tag published 3 of 12 crates);
v0.9.0 "Security Tooling" shipped 2026-09-01 (4 phases, 25 plans, 20/20; tag `v0.9.0` on
`0b5d4106`); v0.8.0 shipped 2026-08-24 (14 phases, 149 plans, 65/65); v0.7.1 shipped 2026-08-04
(4 phases, 38 plans, 25/25). All archived under `milestones/`.

## Current Position

Phase: Milestone v0.11.1 complete
Plan: —
Status: Awaiting next milestone
Last activity: 2026-11-07 — Milestone v0.11.1 completed and archived

Progress: [████████████████████] 7/7 phases (v0.11.1) — plans [████████████████████] 84/84 ([██████████] 100%)

## Performance Metrics

**Velocity:**

- Total plans completed: 673
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
| 55 | 13 | - | - |
| 56 | 14 | - | - |
| 57 | 16 | - | - |
| 58 | 16 | - | - |

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
| Phase 55 P11 | 40 min | 3 tasks | 39 files |
| Phase 55 P12 | 30min | 3 tasks | 8 files |
| Phase 55 P13 | 45min | 3 tasks | 3 files |
| Phase 56 P01 | ~20min | 2 tasks | 3 files |
| Phase 56 P02 | 15min | 3 tasks | 9 files |
| Phase 56 P03 | 25min | 3 tasks | 3 files |
| Phase 56 P04 | ~15min | 2 tasks | 2 files |
| Phase 56 P05 | 12min | 2 tasks | 4 files |
| Phase 56 P06 | ~20min | 2 tasks | 5 files |
| Phase 56 P07 | 25min | 3 tasks | 13 files |
| Phase 56 P08 | 45min | 3 tasks | 5 files |
| Phase 56 P09 | ~1h | 3 tasks | 8 files |
| Phase 56 P11 | 15min | 3 tasks | 11 files |
| Phase 56 P12 | 30min | 2 tasks | 4 files |
| Phase 56 P13 | 2 sessions | 4 tasks | 7 files |
| Phase 56 P14 | 1 session | 3 tasks | 6 files |
| Phase 56 P10 | 20min (resume) | 3 tasks | 12 files |
| Phase 57 P01 | approx 10min (continuation) | 3 tasks | 14 files |
| Phase 57 P05 | 12min | 3 tasks | 9 files |
| Phase 57 P08 | 35min | 3 tasks | 36 files |
| Phase 57 P07 | 30min | 3 tasks | 2 files |
| Phase 57 P02 | 70min | 3 tasks | 9 files |
| Phase 57 P03 | 75min | 3 tasks | 5 files |
| Phase 57 P06 | 28min | 3 tasks | 6 files |
| Phase 57 P09 | 35min | 2 tasks | 5 files |
| Phase 57 P04 | 35min | 2 tasks | 7 files |
| Phase 57 P10 | ~1h | 3 tasks | 5 files |
| Phase 57 P12 | 20min | 2 tasks | 4 files |
| Phase 57 P13 | 26min | 3 tasks | 5 files |
| Phase 57 P14 | 41min | 3 tasks | 7 files |
| Phase 57 P15 | 42min | 3 tasks | 6 files |
| Phase 57 P16 | ~60min | 3 tasks | 9 files |
| Phase 57 P11 | ~2h | 3 tasks | 5 files |
| Phase 58 P01 | 85min | 3 tasks | 7 files |
| Phase 58 P02 | n/a | 2 tasks | 27 files |
| Phase 58 P03 | not measured | 3 tasks | 115 files |
| Phase 58 P04 | not measured | 3 tasks | 5 files |
| Phase 58 P05 | 4h45m (incl. decision wait) | 3 tasks | 4 files |
| Phase 58 P15 | 25min | 5 tasks | 13 files |
| Phase 58 P06 | 21min | 3 tasks | 52 files |
| Phase 58 P16 | 32min | 5 tasks | 11 files |
| Phase 58 P07 | 15min | 3 tasks | 54 files |
| Phase 58 P08 | 65min | 3 tasks | 4 files |
| Phase 58 P09 | 2h20m | 3 tasks | 3 files |
| Phase 58 P10 | 1h | 3 tasks | 6 files |
| Phase 58 P11 | about 1h50m incl. checkpoint pause | 3 tasks | 17 files |
| Phase 58 P12 | about 5h20m incl. checkpoint pauses | 3 tasks | 0 files committed (plan forbids commits) |
| Phase 58 P13 | 10min | 3 tasks | 3 files |
| Phase 58 P14 | 20min | 3 tasks | 8 files |

## Accumulated Context

### Decisions

**Cleared at the v0.11.1 close (2026-11-07).** Every phase-level decision this section carried for
Phases 38-58 is recorded in its own `NN-CONTEXT.md`, `NN-CI-EVIDENCE.md` and the SUMMARY decision
lists under `milestones/v0.11.0-phases/` and `milestones/v0.11.1-phases/`; the locked ones are
ADRs in `.planning/decisions/` and the `## Key Decisions` table in PROJECT.md (Phase 52-58 owner
decisions appended there with outcomes at this close). Only what a later milestone must honour is
kept here:

- **Release rules are locked (Phase 47 D-00a…D-00s, re-carried by Phase 58).** Only the
  maintainer ticks a sign-off box, merges the release PR, pushes the tag and dismisses a platform
  alert; the release PR merges with a merge commit and the tag targets that commit (equality, not
  ancestry); `main` is frozen from the bump push to the merge; the bump is the last three commits
  on the branch; every non-bump change lands first; a red gate from the re-seal onward is a hard
  stop (`FLAKE=named-rerun` is the one exception: exactly one re-run, only for the named
  `treasury_spend_prints_one_row_per_currency` failure, never in `release.yml`); post-tag commits
  reach `main` only through a documentation-only `chore/NN-close` branch and PR; registry proof is
  a sparse-index read plus a keyless scratch-crate smoke, never the workflow log.
- **Patch-or-minor is decided when the first break is known, with cargo-semver-checks at
  `--release-type patch` against the published baseline as the arbiter** (Phase 53
  `53-VERSION-DECISION.md`, Phase 58 D-11). The semver baseline is `0.11.0` until the re-pin todo
  lands (`2026-11-05-repin-semver-baseline-to-0-11-1.md`); every gate added since Phase 52 fails on
  an empty scan and is proven red on a planted defect before it is trusted.
- **FIXES-04 (`RunStatusValue::Cancelled`) is a minor-version change by decision** (Phase 55 D-05,
  register R-13): adding the variant or marking either `paladin-eval` enum `#[non_exhaustive]` is
  major at patch; the next minor's first phase owns it (`UPDATE_EVAL_SCHEMA=1`).
- **A thread belongs to the tenant of its earliest attributed run** (`owner_run_for_thread`, Phase
  57 D-02 amending ADR-0063's latest-run rule); hidden equals missing on every tenant route; a job
  belongs to its enqueuing `(agent id, tenant)`; a newer-schema owner row is skipped in lists and a
  `404` on single reads.
- **Carried findings get one of three verdicts and nothing else** — fixed (commit and test named),
  waived (owner, revisit, basis) or re-homed (owner, `recheck_by`, a pending todo) — under a
  verifier that derives its expected ids from the source documents (`58-check-register.sh`,
  `scripts/check-review-dispositions.sh`, `scripts/check-findings-log.sh`). The 41 Phase 58
  re-homes all carry `recheck_by: 2026-11-09`; 33 are addressed to `v0.12.0 Phase 1`.
- **Maintainer statements that are not re-provable from the repository are recorded as such:** the
  `sk-proj-…` key rotated 2026-11-05 (R-84 fixed), no history purge (R-85 waived, `PURGE=none`),
  the eight F-58-08 CodeQL alerts dismissed on the platform (`CODEQL=dismiss-now`), Phases 54 and
  56 Nyquist-validated by a planning-only audit (`AUDIT=validate-first`).
- **Secrets never serialise and never print:** nine config types carry `skip_serializing` with no
  serde default on their credential fields (M-B-48…M-B-52); `Settings`, `ApiKeyConfig`,
  `QueueConfig`, `FileStorageConfig`, `EmailAdapterConfig`, `RedisQueueConfig` and `MinioConfig`
  print `<redacted>` in `Debug`; the startup line is `settings_summary` (counts only). A
  serialise-then-deserialise round trip of `MinioConfig` / `EmailAdapterConfig` no longer works by
  design (58-REVIEW WR-02, stated in M-B-50/51).
- **Boot refusals are fail-closed and name the key, never the value:** any `${` in an
  `http.auth.api_keys[].key`, an invalid `Settings` after the `APP_*` overlay, a configured
  Treasurer without the engine-factory worker path, reserved tenant/key literals. An empty
  operator-webhook secret is treated as absent with one warning (PD-05-3; the env-wipes-YAML case
  is a maintainer-owned todo).
- **Still accepted, not met:** OBS-05 tracing overhead (rows 35/61, two closes); DNS-rebinding
  pinning (FUT-13); the fence-marker TTL cap's 28-minute residual (EA-57-20); the bare-host
  coverage path (DOGF-05 accepted on the devcontainer figure).

### Pending Todos

45 pending todos in `todos/pending/` were acknowledged as deferred at the v0.11.1 close on
2026-11-07 — every one listed with owner, area and recheck date in *Deferred Items* below. 41 are
the re-home register Phase 58 wrote by design (plan 58-06, D-09: one todo per re-homed disposition
row, owner named, `recheck_by: 2026-11-09`); the other four are the carried OpenAI Responses API item
(2026-10-20) and three filed during Phases 56-58 (`setup-check` first-run output, the unquoted
Treasurer price losing digits, the DeepSeek reasoning-budget hint). By owner: 33 `v0.12.0 Phase 1`,
12 repo maintainer. None is a verification gap; the three previously carried todos
(`2026-08-13-verify-local-coverage-reproduction`, `2026-10-13-review-content-data-orchestration-base-items`,
`2026-10-14-web3sec-single-bump-to-v0-11-0`) were closed by Phases 56, 55 and 56 and sit in
`todos/completed/`.

### Blockers/Concerns

**v0.11.1 close (2026-11-07):** nothing blocks the next milestone. What still stands, all owned:

- **Maintainer-only items, re-homed again with `recheck_by: 2026-11-09`:** the Codecov upload
  reporting `success` while its CLI fails for want of `CODECOV_TOKEN` (R-72 — supply the token and
  set `fail_ci_if_error`, or drop the step; the 82 % floor gate is unaffected); the seven §11
  judgment-tier sign-off boxes and the `v0.10.0` tag box in
  `.project/v0.10.0/09-program-acceptance-audit.md` still `- [ ]` on disk (R-73 — a human tick or a
  "superseded by §13" note); three OSV-Scanner alerts (23-25) with no governing exception; the
  bare-host coverage walk; the empty `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` wiping a YAML secret
  (57 WR-01); the worker `nack`-and-redeliver outcome loss (57 CR-01, a correctness defect).
- **Correctness items addressed to `v0.12.0 Phase 1`:** `AdmissionSettlement::drop` can refund an
  admission for a run that then commits (57 IN-03); two `cancel_tests` race the run's completion
  (filed 2026-11-06 after a post-close CI flake on `chore/58-close`); the `treasury_spend` test-level
  flake (F-58-03, named-rerun rule in force); `s3_error_code` embeds server text unsanitised (57
  IN-04); the News API key travelling in the URL (D-52-03-01); a `${...}` placeholder in the
  Treasurer webhook secret not refused (54 WR-04).
- **Post-release advisory from `58-REVIEW.md` (0 critical, 5 warnings):** §11.8's "without changing
  source" holds at compile time while a `MinioConfig` / `EmailAdapterConfig` round trip no longer
  works (WR-02, stated in M-B-50/51); `OtelConfig.headers` still serialises (WR-01).
- **Environment:** the sandbox cannot run `cargo test --workspace` or the MSRV leg (CI-attributed
  by D-00e, three milestones running) and has no Docker socket; `58-VALIDATION.md` reads `draft`
  (release phase, no requirement-bearing tasks beyond SHIP-08…10).
- **Branch state at close:** `main` is `6082a787e` (PR #65 merge); the archive lands on
  `claude/laughing-dirac-e0h2ax`, restarted from `main`, and reaches `main` through a
  documentation-only PR like PR #60 and PR #65.

**Phase 57 close (2026-11-05, 57-VERIFICATION `passed` 5/5, 57-UAT 17/17 + 2 automated, 57-SECURITY 0 open,
57-VALIDATION `validated`):**

- **Code review non-blocking findings carried to Phase 58** (`57-REVIEW.md`, 0 blocking): CR-01 the
  worker's nack-and-redeliver on a failed second write loses the run's outcome when only that write
  fails; WR-01 an empty `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` env value silently wipes a
  YAML-configured secret; WR-02 `check-review-dispositions.sh` accepts non-evidence as a closed
  pointer. Five info items (IN-01…05) recorded there.

- **Two orphaned Phase 55 deferrals have no owner or revisit** (`57-REGISTER-EVIDENCE.md` "Orphaned
  deferrals"): the News API key travelling in the URL with `reqwest` error text able to echo it
  (D-52-03-01, `news_api_fetcher.rs`) and the `${...}` placeholder in the Treasurer webhook `secret`
  not refused (Phase 54 WR-04). Neither is a WINDOWS row; Phase 58's pre-tag audit must re-home them
  with an owner and a date or they read as `tech_debt`.

- **`Generated SDK Clients` smoke is red in CI** (job 126917997570 on run 41927940508) and was red on
  two earlier runs, green on 41246424413; pre-dates Phase 57, no WINDOWS row, cause undiagnosed.
  `Docker Build` and `Kubernetes Smoke Test` were cancelled on that run (superseded push), not failed.

- **The review-disposition checker is manual** — exercised by `make test-shell-guards` through its 22
  fixtures but not wired into `make check-gates` or CI, by design; the Phase 58 audit runs it by hand.

- **Phases 54-57 are all on `claude/laughing-dirac-e0h2ax`, not yet merged to `main`** (`main` is still
  the PR #62 merge `8b23cb4c5`; the branch is 358 commits ahead). A PR to `main` is the maintainer's
  step before the Phase 58 bump commit, and the Phase 56 `sk-proj-…` literal finding below is still
  open for that pre-tag pass.

**Phase 56 close (2026-11-04, 56-VERIFICATION `passed` 11/11, 56-UAT 4/4, 56-SECURITY 0 open):**

- **A complete `sk-proj-…` OpenAI key literal is tracked in
  `.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md:53`** (in the tree
  since at least `928c6d5f9`, 2026-07-30, so also in history). Found by the 56-SECURITY tree scan;
  outside the phase's register (predates it). Maintainer action: rotate the key if it was ever
  live, scrub the literal, decide on a history purge. A tracked-tree key-shape scan in
  `check-gates` (the 56-08 staged-file scanner, made standing) is the candidate guard for Phase 57.

- **Bare-host coverage path still unwalked** — DOGF-05 accepted at UAT on the devcontainer figure
  (no Docker socket); guide step 1 (`make services-up`) and the password-protected Redis path
  (trap 2) remain unexercised.

- **Re-homed `out` findings carry six pending todos** in `.planning/todos/pending/` (DF-06, DF-16,
  DF-18 incl. the 109-site deprecated `gpt-4` literal sweep, DF-20, DF-34, DF-36). v0.11.1 ships
  the Quickstart `.model("gpt-4")` line unchanged by owner decision.

- **Phase 56 work is on `claude/laughing-dirac-e0h2ax`, not yet merged to `main`** — a PR is the
  maintainer's step before Phase 57 builds on it.

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

- ~~**H-06 (local `make coverage` reproduction) deferred again**~~ — **closed in Phase 56** (todo in `completed/`, 92.79 % local vs 92.86 % CI, run `41703614636`). Was: deferred to the maintainer on a Docker-capable
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

- ~~**Phase 56 depends on external inputs**~~ — **all three arrived and were triaged in Phase 56** (DF-10..DF-26 from the walks, DF-27..DF-36 from the web3sec return). Was: (the maintainer's runs, the web3sec return, a Docker-capable machine for
  DOGF-05) that the roadmap cannot schedule; an unreturned DOGF-02 is held or re-homed with an owner and date.

- ~~**Phase 57:** `open_count: 0` also needs rows 64-66 (accepted ADR-0057 contracts) and 77 (Grok deferral) waived with a
  reason or re-homed; no requirement names them beyond REG-02's closing clause.~~ — **resolved in Phase 57
  (2026-11-05)**: every open row waived or re-homed with owner and date, `WINDOWS.md` sealed at `open_count: 0` by 57-11
  (57-VERIFICATION criterion 1).

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
- Phase 55 carry-forward: 6 crate changelogs still lack an [Unreleased] anchor before the 0.11.1 finalize; WINDOWS row 56 can close on the recorded rustdoc measurement (Phase 57); rows 85-87 await a 56.x phase -- see 55-CORRECTNESS-EVIDENCE.md Carry-forward. **Update 2026-11-05:** row 56 and rows 85-87 closed in Phase 57 (57-02/57-03 fixes, 57-11 seal); the changelog anchors remain Phase 58's

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
- Milestone v0.11.1 closed 2026-11-07 (tag `v0.11.1` 2026-11-06): Phases 52-58 archived to `milestones/v0.11.1-*` and `v0.11.1-phases/`; ROADMAP.md collapsed to a `<details>` block; the roadmap held its shape (no insertion, no extension); next phases start at 59

## Deferred Items

### Acknowledged at v0.11.1 milestone close (2026-11-07)

**Verification overrides: 0** — all 7 phases (52-58) report `phase_complete: true` and
`verification_status: passed` (`init.manager` `all_phases_verified: true`); 84/84 plans carry a
SUMMARY. **Requirement gaps acknowledged: 1** — FIXES-04 re-homed to the next minor by Phase 55
D-05 (register row R-13), deliberately unticked (MILESTONES.md *Known Gaps*). **Open artifacts
acknowledged: 45** at the pre-close `audit-open` run — all pending todos (0 debug sessions, 0
quick tasks, 0 threads, 0 UAT or verification gaps). 41 are the re-home register Phase 58 wrote by
design (one dated todo per re-homed disposition row, `recheck_by: 2026-11-09`); the other four are
the carried OpenAI Responses API item and three filed during Phases 56-58. Closeout type
`override_closeout`, on the strength of those artifacts rather than any unverified phase. Titles
below are the todo files' own frontmatter, pipes and newlines stripped.

| Category | Item | Title (area) | Owner | Recheck by | Source |
|----------|------|--------------|-------|------------|--------|
| todo | `2026-10-20-openai-responses-api-for-pro-tier` | Add OpenAI Responses API support so the pro tier (gpt-5.5-pro and siblings) is usable (llm-adapters) | repo maintainer | 2026-11-09 | carried from the v0.11.0 close |
| todo | `2026-11-02-setup-check-first-run-output` | paladin-cli setup-check section order is unstable, keyless runs show bare headings, and the CLI's own messages name `paladin` (cli) | repo maintainer | 2026-11-09 | Phase 56 plan 56-06 (DF-16, DF-17) |
| todo | `2026-11-02-unquoted-treasurer-price-loses-digits-on-load` | An unquoted treasurer price with 19 significant digits loses digits on load (config) | repo maintainer | 2026-11-09 | Phase 56 plan 56-02 (DF-06) |
| todo | `2026-11-04-reasoning-budget-hint-for-deepseek-empty-completion` | Budget-aware default or reasoning-share hint for reasoning models that spend max_tokens thinking (llm-providers) | repo maintainer | 2026-11-09 | Phase 56 plan 56-10 (DF-34) |
| todo | `2026-11-05-admission-settlement-drop-refunds-committed-run` | 57-REVIEW IN-03: AdmissionSettlement::drop can refund an admission for a run that then commits (run-submission) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-06) |
| todo | `2026-11-05-cargo-snippets-name-nonexistent-features` | 54 WR-01: copy-paste Cargo snippets name features that do not exist (docs) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-26) |
| todo | `2026-11-05-codecov-upload-fails-silently` | Codecov upload fails silently (no CODECOV_TOKEN, fail_ci_if_error unset) and sweeps non-coverage JSON files (ci) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-72) |
| todo | `2026-11-05-config-roundtrip-test-claims-no-app-env-dependency` | 56 WR-07: the config round-trip test claims no APP_* dependency but calls Settings::new() (test-hygiene) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-51) |
| todo | `2026-11-05-crate-wide-allow-mirror-blind-spots` | 52 WR-03: the crate-wide-allow mirror cannot see warn, workspace metadata or non-double-quoted forms (release-tooling) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-36) |
| todo | `2026-11-05-dockerfile-dev-cargo-release-pin-drift` | .devcontainer/Dockerfile.dev pins cargo-release 1.1.2 while releases since plan 47-01 use 1.1.7 (devcontainer) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-82) |
| todo | `2026-11-05-docs-name-unread-paladin-request-timeout` | 56 WR-08: docs name PALADIN_REQUEST_TIMEOUT, which no code reads, and the paladin-cli rename is partial (docs) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-52) |
| todo | `2026-11-05-embedding-port-rustdoc-literal-backslash-n` | EmbeddingPort rustdoc is one physical line with literal backslash-n text (docs) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-14) |
| todo | `2026-11-05-empty-treasurer-webhook-secret-env-wipes-yaml-secret` | 57-REVIEW WR-01: an empty APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET silently wipes a YAML-configured secret (config) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-02) |
| todo | `2026-11-05-eval-run-status-value-lacks-cancelled` | FIXES-04: paladin-eval RunStatusValue has no Cancelled value (paladin-eval) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-13) |
| todo | `2026-11-05-feature-stub-todos-trigger-service-runner` | Feature-stub TODOs in trigger.rs (JSONPath payload matching, cooldown), service_runner.rs (notification adapter wrapp... (run-services) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-22) |
| todo | `2026-11-05-fence-marker-ttl-cap-reopens-stale-writer-overwrite` | 57-REVIEW IN-05: the fence marker TTL cap reopens stale-writer overwrite for long-lived node-cache entries (storage) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-08) |
| todo | `2026-11-05-findings-log-check-accepts-vacuous-dispositions` | 56 WR-02: check-findings-log.sh accepts vacuous final dispositions (out () (x), duplicate cycles, 'in v0.11.1 - TBD') (release-tooling) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-46) |
| todo | `2026-11-05-findings-log-check-drops-malformed-df-heading` | 56 WR-01: check-findings-log.sh silently drops a malformed ### DF- heading (confirmed false pass) (release-tooling) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-45) |
| todo | `2026-11-05-ignore-inventory-in-testing-guide` | An #[ignore] inventory in the testing guide (E8, DOCS-09) (docs) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-21) |
| todo | `2026-11-05-migration-allowlist-check-passes-on-missing-heading` | 52 WR-02: check-migration-allowlist.sh silently passes when the 11.2 heading is missing or renamed (release-tooling) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-35) |
| todo | `2026-11-05-news-api-key-in-url-and-error-text` | News API key travels in the URL and reqwest error text can echo it (D-52-03-01) (content-security) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-09) |
| todo | `2026-11-05-onboarding-claims-keys-validated-after-failures` | 56 WR-05: onboarding reports 'All API keys validated successfully!' after failed validations (cli) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-49) |
| todo | `2026-11-05-openai-429-header-name-fetched-quote` | OpenAI 429 header-name evidence rests on operator attestation (2026-10-09), not a fetched quote (llm-cadence) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-88) |
| todo | `2026-11-05-osv-scanner-alerts-23-25-no-governing-exception` | Three OSV-Scanner alerts 23-25 with no governing exception (RUSTSEC-2026-0221, RUSTSEC-2026-0205, GHSA-7gcf-g7xr-8hxj) (security) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-11) |
| todo | `2026-11-05-pricing-coverage-check-at-graph-registration` | The pricing-coverage check at graph registration and enumerating workflow assistants' node models for the boot warning (treasurer) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-19) |
| todo | `2026-11-05-refresh-codebase-concerns-and-testing-docs` | Refreshing .planning/codebase/CONCERNS.md and TESTING.md (dated 2026-07-30, stale on the commander tests) (planning-docs) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-24) |
| todo | `2026-11-05-refuse-placeholder-in-treasurer-webhook-secret` | A ${...} placeholder in the Treasurer webhook secret is not refused (54 WR-04) (config) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-10) |
| todo | `2026-11-05-registry-membership-check-for-other-model-ids` | A registry-membership model check for the other ids (gpt-4o, gpt-3.5-turbo, gpt-4-turbo and the rest): serving status... (docs-models) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-80) |
| todo | `2026-11-05-release-body-fit-set-e-suppressed-write-failures` | 52 WR-01: set -e is suppressed inside the release-body size-fit functions, so write failures are swallowed (release-tooling) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-34) |
| todo | `2026-11-05-repin-semver-baseline-to-0-11-1` | Re-pin the semver baseline to the released 0.11.1: SEMVER_BASELINE_VERSION in scripts/check-semver.sh, the ci.yml job... (release-tooling) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-78) |
| todo | `2026-11-05-retire-base-layer-items` | Retiring or partially retiring the base-layer items (todo options B/C, MIGRATION 12.2 rows, allowlist entries, a guar... (core-base) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-18) |
| todo | `2026-11-05-rust-side-gpt-4-model-id-sweep` | The deprecated gpt-4 model id in crates/*/src, src/, crates/doc-examples and tests/ (the GarrisonConfig default token... (docs-models) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-79) |
| todo | `2026-11-05-s3-error-code-embeds-unsanitised-server-text` | 57-REVIEW IN-04: s3_error_code embeds up to 64 characters of server-controlled text unsanitised (storage) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-07) |
| todo | `2026-11-05-section-11-judgment-tier-signoff-boxes` | The seven section-11 judgment-tier sign-off boxes and the v0.10.0 tag box in .project/v0.10.0/09-program-acceptance-a... (release-docs) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-73) |
| todo | `2026-11-05-serial-does-not-protect-env-readers` | 56 WR-06: #[serial] does not protect against concurrent env readers (unsafe set_var in edition 2024) (test-hygiene) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-50) |
| todo | `2026-11-05-share-warn-once-set-boot-and-settle` | Sharing the warn-once set between boot and settle time through an additive pub fn in paladin-llm (llm) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-20) |
| todo | `2026-11-05-stale-codeql-dismissal-line-anchors` | Stale CodeQL line anchors in CODEQL-DISMISSALS.md rows 40 and 49 (D-52-03-03) (security-docs) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-12) |
| todo | `2026-11-05-stale-latest-run-wording-after-d02-rename` | 57-REVIEW IN-02: stale 'latest run' wording left behind by the D-02 rename (docs) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-05) |
| todo | `2026-11-05-thirteen-unflagged-illustrative-gpt-4-tokens` | Thirteen unflagged illustrative gpt-4 tokens remain in docs and examples, including the pricing-table key at docs/src... (docs-models) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-81) |
| todo | `2026-11-05-treasury-spend-one-row-per-currency-flake` | F-58-03: treasury_spend_prints_one_row_per_currency is a test-level flake (test-hygiene) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-77) |
| todo | `2026-11-05-walk-bare-host-coverage-path` | The bare-host coverage path (make services-up, password-protected Redis, trap 2) is still unwalked (coverage) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-55) |
| todo | `2026-11-05-worker-doc-comment-split-across-wrong-item` | 57-REVIEW IN-01: a doc comment is split across the wrong item in worker.rs (docs) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-04) |
| todo | `2026-11-05-worker-nack-redeliver-loses-run-outcome` | 57-REVIEW CR-01: the worker's nack-and-redeliver loses a run's outcome when only the second write fails (run-worker) | repo maintainer | 2026-11-09 | Phase 58 plan 58-06 (R-01) |
| todo | `2026-11-05-worker-pods-mount-full-config-with-admin-keys` | 54 IN-02: worker pods mount the full config including literal admin API keys they never use (k8s) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 plan 58-06 (R-30) |
| todo | `2026-11-06-cancel-tests-timing-races` | cancel_tests: two timing-dependent tests race the run's completion (local_cancel_signals_token, cross_instance_cancel_probe) (run-worker) | v0.12.0 Phase 1 | 2026-11-09 | Phase 58 close-out (PR #65 CI, 2026-11-06) |
| requirement | FIXES-04 | `paladin-eval` `RunStatusValue::Cancelled` — re-homed (next minor), checkbox `[ ]`, traceability `Re-homed (next minor)` (paladin-eval) | v0.12.0 Phase 1 | 2026-11-09 | Phase 55 D-05, register R-13 |
| testing | Nyquist validation unreconciled for archived Phases 05-21; `58-VALIDATION.md` `draft` (release phase) | Coverage TODO, not a compliance failure; Phases 22-57 reconciled | — | — | v0.10.0 close; re-acknowledged v0.11.0 and v0.11.1 |
| debt | Four `zip 3.0.0` RustSec exceptions (0367/0368/0369/0429) | Re-dated to 2027-03-31 in Phase 57 (REG-06); lift waits on the utoipa 6 migration | repo maintainer | 2027-03-31 | v0.11.0 close; re-checked v0.11.1 |

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

Last session: 2026-11-07 (milestone close)
Stopped at: Milestone v0.11.1 closed and archived; next `/gsd-new-milestone` (new phases start at Phase 59)
Resume file: None

## Operator Next Steps

- **Milestone v0.11.1 is closed (2026-11-07).** Archive: `milestones/v0.11.1-ROADMAP.md`,
  `v0.11.1-REQUIREMENTS.md`, `v0.11.1-MILESTONE-AUDIT.md` (`passed`), `v0.11.1-phases/`; record:
  `MILESTONES.md`; retrospective: `RETROSPECTIVE.md`. `REQUIREMENTS.md` is removed — the next
  milestone mints a fresh one. The `v0.11.1` tag already existed (cut by the maintainer 2026-11-06,
  object `65b13ed08` on `251303bcd`) and was not re-created. The close landed on
  `claude/laughing-dirac-e0h2ax` (restarted from `main` at `6082a787e`); merge it to `main` through a
  documentation-only PR like `chore/58-close` (PR #65).

- **Start the next milestone:** `/clear` then `/gsd-new-milestone` (questioning → research →
  requirements → roadmap). New phases start at **Phase 59**. Plan the first phase from the
  Phase 58 re-home register before anything new: 33 of the 45 pending todos are addressed to
  `v0.12.0 Phase 1` with `recheck_by: 2026-11-09` (the worker outcome-loss CR-01, the
  `AdmissionSettlement::drop` refund, the two `cancel_tests` races, FIXES-04, the semver baseline
  re-pin to `0.11.1`, the base-layer retirement, the `#[ignore]` inventory among them). Other
  candidate inputs on file: PROV-05 (OpenAI Responses API, the stated first feature item), the v2
  list in `milestones/v0.11.1-REQUIREMENTS.md`, the FUT register, the two credential probes the
  retrospective asks for in `check-gates`, and the tracing-overhead bar (rows 35/61).

- **Maintainer-only items at the close, all with `recheck_by: 2026-11-09`:** the Codecov upload
  (R-72: `CODECOV_TOKEN` + `fail_ci_if_error`, or drop the step); the §11 sign-off boxes in
  `.project/v0.10.0/09-program-acceptance-audit.md` (R-73, a human tick); the three ungoverned
  OSV-Scanner alerts 23-25; the bare-host coverage walk; the empty-webhook-secret override (57
  WR-01); the worker `nack`-and-redeliver outcome loss (57 CR-01). The outbound web3sec hand-off is
  `milestones/v0.11.1-phases/58-crate-release/58-HANDOFF-web3sec.md` (no return contract).
