---
phase: 29-program-gates-release
plan: 08
subsystem: docs
tags: [windows-ledger, defect-register, ship-03, ship-04, d-24, release-gate]

# Dependency graph
requires:
  - phase: 29-04
    provides: "29-04-SUMMARY.md's per-FR evidence table and Section 4 cross-reference to WINDOWS.md rows 24/25 (used to confirm neither row is flagged as an unmet FR before waiving)"
  - phase: 29-07
    provides: "29-07-SUMMARY.md's Section 6-9 findings (used to confirm none of the nine bucket-3 deviation rows is flagged as an unmet FR), the D-16 accepted-deviation record and its STATE.md D-37 sign-off citation, and the proposed-but-not-written 72-warning cargo-doc deviation note (left out of this plan's scope per the plan's own total_count=35 math)"
provides:
  - "`.planning/WINDOWS.md` fully triaged: 25 previously-open rows plus one new row all carry a disposition and a cited reason; open_count 0, total_count 35"
  - "The four bucket-2 CI run/job identifiers and their job names, recorded here since `gsd-tools windows fixed` carries no reason argument"
affects: ["29-09 (release readiness section 10 and the changelog Known limitations entry can now cite a closed WINDOWS.md register)"]

# Tech tracking
tech-stack:
  added: []
  patterns: ["ledger-transition-only-through-CLI (no direct markdown/frontmatter edits)", "citation-lives-in-SUMMARY-when-the-CLI-verb-cannot-carry-it (documented and confirmed against the fixed handler's actual source before choosing this path, not assumed)"]

key-files:
  created: []
  modified:
    - .planning/WINDOWS.md

key-decisions:
  - "Confirmed directly from `.claude/gsd-core/bin/lib/broken-windows.cjs`'s `cmdWindowsMarkFixed`/`markFixed` source (not assumed from the plan's own hedge) that the `fixed` verb takes only an id and never accepts or stores a reason -- `markFixed` spreads the existing entry and only sets `status`/`resolved_at`. The four bucket-2 CI run/job identifiers are therefore recorded in this SUMMARY's own evidence table below, not in the ledger row, exactly as the plan's own contingency instructed."
  - "The new bucket-4 row (id 35, D-16 Phase 28 tracing-overhead deviation) was appended then immediately waived, not left open. `gsd-tools windows append` always creates a row in `open` status; the plan's own Task 2 acceptance criterion requires `open_count` to reach 0 (or exactly the ids this SUMMARY names as held-open findings, of which there are none), and `/gsd-ship` blocks on `open_count > 0` regardless of which bucket a row belongs to. Since D-16's own disposition is 'ACCEPTED' (a maintainer sign-off already on record, STATE.md D-37), not a code fix, `waived` -- not `fixed` -- is the correct terminal status."
  - "No row is held open as an unmet-FR finding. 29-04-SUMMARY.md and 29-07-SUMMARY.md were read in full before any bucket-3 waiver; neither records any of rows 23/24/25/29/30/31/32/33/34 as an unmet FR -- 29-04's Section 4 cross-references rows 24/25 by number as an already-scoped, already-open item (not a new finding), and 29-07 explicitly declines to write a WINDOWS.md row itself (isolation instruction) while proposing the D-16 row this plan writes. Consequently `open_count` reaches exactly 0, not a partial count."
  - "The pre-existing 72-warning `cargo doc --workspace --no-deps` condition that 29-07-SUMMARY.md proposed as a WINDOWS.md row is NOT filed by this plan. 29-08-PLAN.md's own `<interfaces>` section enumerates exactly one new row for bucket 4 (the D-16 overhead deviation) and its Task 2 acceptance criteria require `total_count` to be exactly 35 (34 + 1) -- filing a second new row would make `total_count` 36 and fail that criterion. 29-CONTEXT.md's Open Question 2 resolution record names the cargo-doc row's owner as `29-07-PLAN.md` / `29-09-PLAN.md`, not this plan; it remains proposed-but-unfiled, for plan 29-09 (or a later phase) to actually write if wanted."

patterns-established:
  - "When a CLI transition verb cannot carry a citation (confirmed by reading its source, not inferred from its help text), the citation is recorded in the executing plan's own SUMMARY.md with an explicit cross-reference statement in the ledger's own commit message and SUMMARY decisions section, rather than working around the verb's limitation with a direct file edit."

requirements-completed: [SHIP-03]

coverage:
  - id: D1
    description: "All 25 previously-open WINDOWS.md rows are triaged (21 waived, 4 fixed) with cited reasons; open_count reaches 0; no row held open as an unmet-FR finding"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "node .claude/gsd-core/bin/gsd-tools.cjs windows status -> open_count 0, waived_count 26, fixed_count 9, total_count 35"
        status: pass
    human_judgment: false
  - id: D2
    description: "No row deleted; every waived row carries a non-empty, per-row reason (not a template pasted across rows); at least 12 bucket-1 reasons cite v0.9.0-MILESTONE-AUDIT.md"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "awk row-count over .planning/WINDOWS.md == 35; awk empty-reason-on-waived check == 0; grep -c v0.9.0-MILESTONE-AUDIT .planning/WINDOWS.md == 24"
        status: pass
    human_judgment: false
  - id: D3
    description: "Bucket-2 rows (22, 26, 27, 28) marked fixed against real CI evidence, never a locally-reproduced number; the CI run/job identifiers are recorded here since the fixed verb cannot carry a reason"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "awk status==fixed check over rows 22/26/27/28 == 0 bad; evidence table below cites job 102125436566 / run 34245093476 (rows 22, 26), run 34344074367 (row 28), job 102482098437 / run 34356304863 (row 27)"
        status: pass
    human_judgment: false
  - id: D4
    description: "One new row filed for the Phase 28 tracing-overhead deviation (D-16), citing PRD 07 acceptance 6, the measured 22.18%/18.46% figures, and the maintainer sign-off; frontmatter counts reconcile and total_count grows by exactly 1"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "new row id 35, kind deviation, phase 28; grep -c 22.18 .planning/WINDOWS.md == 4; total_count == 35; open_count+waived_count+fixed_count == total_count"
        status: pass
    human_judgment: false

# Metrics
duration: ~50min
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 08: WINDOWS.md Defect Register Triage Summary

**Triaged all 25 previously-open `WINDOWS.md` rows through `gsd-tools windows waive`/`fixed`/`append` alone: 21 rows (12 pre-v0.9.0 debt + 9 documented design deviations) waived with per-row citations, 4 Docker-gated/non-canonical rows marked fixed against real CI evidence, and one new row filed and waived for the Phase 28 tracing-overhead deviation — closing the `/gsd-ship` `open_count > 0` gate on evidence, with `open_count` 0 and `total_count` 35.**

## Performance

- **Duration:** ~50min
- **Tasks:** 2
- **Files modified:** 1 (`.planning/WINDOWS.md`, via CLI only — no direct edits)

## Accomplishments

- Read `.planning/WINDOWS.md` in full, `.planning/milestones/v0.9.0-MILESTONE-AUDIT.md` (confirmed `status: tech_debt`, "no critical blockers"), `29-04-SUMMARY.md` and `29-07-SUMMARY.md` (confirmed neither records any bucket-3 row as an unmet FR), and the SUMMARY files that originally recorded each bucket-3 deviation (`22-11-SUMMARY.md`, `22-16-SUMMARY.md`, `27-12-SUMMARY.md`, `27-18-SUMMARY.md`/`27-21-SUMMARY.md`, `27-23-SUMMARY.md` ×2, `28-11-SUMMARY.md`, `28-13-SUMMARY.md`) before writing a single waiver.
- Waived 12 bucket-1 rows (2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 14, 19) — pre-v0.9.0 debt — each with a distinct, per-row reason citing `v0.9.0-MILESTONE-AUDIT.md`'s `tech_debt` status and zero critical blockers, plus a sentence naming what specifically remains unproven for that row (no reason reused verbatim across rows).
- Waived 9 bucket-3 rows (23, 24, 25, 29, 30, 31, 32, 33, 34) — documented design deviations — each citing the specific SUMMARY that recorded the decision. Rows 24/25 (the self-loop fixture workaround) cross-reference `29-04-SUMMARY.md`'s audit Section 4 by name, matching that audit's own record that these are already-scoped, already-open rows rather than a new finding.
- Confirmed directly from `broken-windows.cjs` source that `gsd-tools windows fixed <id>` takes no reason argument (`markFixed` only sets `status`/`resolved_at`, spreading the rest of the entry unchanged) before choosing how to carry bucket-2's CI citations — following the plan's own contingency instruction rather than guessing.
- Marked rows 22, 26 and 28 `fixed` against real CI evidence (job/run identifiers below, recorded in this SUMMARY per the `fixed` verb's limitation) and row 27 `fixed` against the canonical `coverage` job's most recent post-merge figure (90.28%, above the 82% ADR-0006 floor) rather than a locally-reproduced number.
- Filed one new row (id 35, kind `deviation`, phase 28) for the Phase 28 tracing-overhead FAIL (D-16: PRD 07 acceptance 6's ≤3% bar, measured +22.18%/+18.46%), then waived it citing the maintainer sign-off already recorded at Phase 28 close-out UAT (`STATE.md` D-37) — an ACCEPTED deviation, not a code fix, so `waived` is the correct terminal status and `open_count` reaches 0.
- Reconciled the ledger after every transition: `open_count` 25 → 4 (after Task 1) → 1 (after the three CI-evidence fixes and the coverage fix) → 0 (after waiving the newly-filed row); `total_count` 34 → 35; `waived_count` 4 → 26; `fixed_count` 5 → 9. No row deleted at any point (35 data rows present throughout, plus the pre-existing 34).

## Task Commits

1. **Task 1: Triage buckets 1 and 3 — waive every pre-v0.9.0 row and every documented design deviation, each with a cited reason** — `35293e6e` (docs)
2. **Task 2: Triage bucket 2 against real CI runs, file the overhead deviation row, and reconcile the counts** — `ead41f67` (docs)

**Plan metadata:** committed as part of this SUMMARY (see final commit).

## Files Created/Modified

- `.planning/WINDOWS.md` — all 35 rows now carry a terminal status (`waived` or `fixed`) with a non-empty reason cell where the CLI can carry one; `open_count: 0`, `waived_count: 26`, `fixed_count: 9`, `total_count: 35`.

## Bucket-2 CI Evidence (recorded here — `gsd-tools windows fixed` carries no reason argument)

| Row | Phase | What it closes | Job name | Job / Run identifier | Evidence file |
|-----|-------|-----------------|----------|----------------------|----------------|
| 22 | 22 | Postgres Tier-2 waypoint contract suite, never run locally | `Postgres Storage Contract Suites (live server)` (`postgres-integration`) | job `102125436566`, run `34245093476` — `test result: ok. 87 passed; 0 failed`, waypoint/run/assistant/run_schedule/webhook `*::postgres` tests all exercised the live server | `.planning/phases/27-platform-api/27-CI-EVIDENCE.md` (Tier-2/CI evidence table) |
| 26 | 24 | Phase 24's new Postgres Tier-2 cases (`awaiting_input_payload_round_trips`, `fork_of_round_trips`, `latest_prefers_most_recently_created_across_branches`) | Same job as row 22 — `postgres-integration` job runs the whole `*::postgres` suite, and this run postdates Phase 24's close (2026-09-05) by three days | job `102125436566`, run `34245093476` | Same as row 22 |
| 27 | 24 | Coverage measured with a non-canonical `cargo llvm-cov` invocation | `Coverage` (canonical `coverage` job, `cargo llvm-cov --fail-under-lines 82`) | job `102482098437`, run `34356304863` (post-merge push 2, Phase 28 close `ff78a6b5`) — `Lines: 108172/119822 = 90.28%`, above the 82% ADR-0006 floor | `.planning/phases/28-observability-tooling/28-CI-EVIDENCE.md` (Post-merge CI evidence, Push 2) |
| 28 | 25 | RedisNodeCache Tier-2 live-server contract suite, never run locally | `Redis Node Cache Contract Suite (live server)` (`redis-cache-integration`) | run `34344074367` (`ci.yml`, pre-close-out Phase 28 run) — conclusion: success | `.planning/phases/28-observability-tooling/28-CI-EVIDENCE.md` (`ci.yml` per-job detail table) |

Row 27's figure is the most recent canonical `coverage` job run available at the time of this plan (post Phase 28 close, closer to HEAD than Phase 24's own 89.98%/27-CI-EVIDENCE.md figure or the earlier 90.27% push-1 figure) — cited rather than either earlier figure, per the plan's instruction to use "the canonical `coverage` job's figure ... from a named run," not necessarily the row's own originating phase's run.

## Decisions Made

- **`gsd-tools windows fixed`'s inability to carry a reason was confirmed by reading source, not assumed.** `.claude/gsd-core/bin/lib/broken-windows.cjs`'s `cmdWindowsMarkFixed` calls `parseArgs(args, { flags: [], required: [], positionals: 1 })` (one positional, the id) and `markFixed` spreads the existing entry, setting only `status` and `resolved_at` — `reason` is never touched. This is why rows 12/13/20/21 (the ledger's own `fixed`-with-a-long-reason precedents) carry populated reason text despite the current CLI's `fixed` verb being reason-less: those reasons predate this constraint or were set through a different path (the ledger's own 2026-08-23 normalization note on row 21 documents one prior off-schema fix-up). Rather than replicate that pattern with a direct edit — forbidden by this plan's own instructions — the four bucket-2 citations are recorded in this SUMMARY's evidence table above, exactly as the plan's own contingency text directs.
- **The new bucket-4 row (id 35) was appended, then immediately waived, not left open.** `gsd-tools windows append` always creates a row in `open` status (confirmed from `appendWindow`'s source: `status: 'open'` is hardcoded). The plan's Task 2 acceptance criteria require `open_count` to be 0 (or exactly the ids this SUMMARY names as held-open findings — there are none), and `/gsd-ship`'s gate is `open_count > 0` regardless of bucket. D-16's own disposition is "ACCEPTED for v0.10.0" — a maintainer sign-off already on record (`STATE.md` D-37), not a pending code fix — so `waived` (not `fixed`) is the correct terminal status, citing that sign-off directly.
- **No row is held open as an unmet-FR finding.** `29-04-SUMMARY.md` and `29-07-SUMMARY.md` were read in full before any bucket-3 waiver. Neither records any of rows 23, 24, 25, 29, 30, 31, 32, 33, 34 as an unmet FR: `29-04`'s Section 4 cross-references rows 24/25 by number as an already-open, already-scoped item (not a newly-discovered gap), and `29-07` explicitly declines to write to `WINDOWS.md` itself (isolation instruction) while proposing exactly the one new row (D-16) this plan writes. Consequently `open_count` reaches exactly 0, with zero rows named as deliberately held open.
- **The 72-warning `cargo doc` deviation row that `29-07-SUMMARY.md` proposed is NOT filed by this plan.** `29-08-PLAN.md`'s own `<interfaces>` section enumerates exactly one new row for bucket 4 (12 + 4 + 9 + 1 = 34 + 1 = 35), and Task 2's acceptance criteria require `total_count` to be exactly 35 — filing a second new row here would make it 36 and fail that criterion outright. `29-CONTEXT.md`'s Open Question 2 resolution record names the row's owner as `29-07-PLAN.md` / `29-09-PLAN.md`, not this plan (29-08); it remains a named, proposed-but-unfiled condition (recorded in the audit document's Section 8 and `29-CI-EVIDENCE.md`, per 29-07's own scope) for plan 29-09 or a later phase to file if still wanted.
- **Every transition went through the CLI; the frontmatter was never hand-edited.** Confirmed at each step via `node .claude/gsd-core/bin/gsd-tools.cjs windows status` and post-hoc `git diff` review of both commits — the only frontmatter fields that changed across either commit are `open_count`, `waived_count`, `fixed_count`, `total_count`, and `last_updated`.

## Deviations from Plan

### Auto-fixed Issues

None. The plan's own contingency text for the `fixed` verb's reason limitation (Task 2's `<action>`) was followed exactly as written — this is not a deviation from the plan, it is the plan's own anticipated branch, confirmed and taken.

**Total deviations:** 0. No code touched, no scope creep — only `.planning/WINDOWS.md`, and only through the named CLI verbs.

## Threat Flags

None. This plan touches only the defect register through its own CLI and adds no new attack surface, no new dependency, and no code.

## Known Stubs

None introduced. Row 34 (the pre-existing `run_run_export` stub, Phase 28) is triaged (waived) by this plan, not newly created here — its underlying stub in `src/application/cli/commands/run.rs:268` is unchanged; only the ledger's disposition of the already-recorded stub changed.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 29-09 can proceed to fill audit section 10 (release readiness) and the root `CHANGELOG.md`'s `[0.10.0]` Known limitations entry, citing a fully-triaged `WINDOWS.md` (`open_count: 0`) rather than an untriaged register.
- The pre-existing 72-warning `cargo doc` condition remains a named, unfiled `WINDOWS.md` candidate row — plan 29-09 (or a later phase) can file it if still wanted; `29-CI-EVIDENCE.md` and the audit document's Section 8 already carry the measured count (72) as a carried, out-of-scope condition regardless.
- No blockers. `.planning/STATE.md`, `.planning/ROADMAP.md`, and `.planning/REQUIREMENTS.md` checkboxes were left untouched per the orchestrator's explicit instruction — those transitions belong to the orchestrator after this wave completes.

## Self-Check: PASSED

- `.planning/WINDOWS.md` — FOUND, modified
- Commit `35293e6e` (Task 1) — FOUND in `git log --oneline`
- Commit `ead41f67` (Task 2) — FOUND in `git log --oneline`
- `node .claude/gsd-core/bin/gsd-tools.cjs windows status` → `open_count: 0`, `waived_count: 26`, `fixed_count: 9`, `total_count: 35` — CONFIRMED
- Row count in `.planning/WINDOWS.md` (`awk` over `| N |` rows) → 35 — CONFIRMED
- Rows 22/26/27/28 all read `status: fixed` — CONFIRMED
- Empty-reason-on-waived count → 0 — CONFIRMED
- `grep -c v0.9.0-MILESTONE-AUDIT .planning/WINDOWS.md` → 24 (≥ 12 required) — CONFIRMED
- `grep -c 22.18 .planning/WINDOWS.md` → 4 — CONFIRMED
- `open_count + waived_count + fixed_count == total_count` (0+26+9=35) — CONFIRMED
- `git diff` on both commits touches only `.planning/WINDOWS.md` — CONFIRMED

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
