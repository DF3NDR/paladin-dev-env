---
phase: 29-program-gates-release
plan: 05
subsystem: infra
tags: [migration-docs, ci, github-actions, semver-checks, release-engineering]

# Dependency graph
requires:
  - phase: 29-program-gates-release
    provides: "v0_9_config_boot (29-01) and openapi_golden_v0_9.rs (29-02) — the two SHIP-02 proofs this plan cites by name and path; the row-level allowlist gate style (29-03) this plan's sibling TBD-gate step copies"
provides:
  - "MIGRATION.md with all four placeholder markers closed (header ×2, §9.5, §9.8) and both stale forward-references in §9.6/§9.7 replaced with closed-state citations"
  - "A durable CI gate in the semver job that fails the build if MIGRATION.md ever reacquires an unfilled placeholder marker"
  - "The SHIP-02 v0_9_config_boot proof actually running in CI, in the existing e2e-platform-api job, with a zero/degraded-selection guard"
affects: [29-06 (Upgrading page, mirrors §9.1/§9.8), 29-07/29-09 (acceptance audit and release evidence cite this plan's closed MIGRATION.md and green CI gates)]

# Tech tracking
tech-stack:
  added: []
  patterns: [register-plus-gate-both-directions, sibling-step-same-job-no-new-job, fail-first-proven-gate]

key-files:
  created: []
  modified:
    - MIGRATION.md
    - .github/workflows/ci.yml

key-decisions:
  - "Plan's literal acceptance-criteria step counts (semver=6, e2e-platform-api=7) do not match the measured pre-task baseline at this HEAD (semver already had 6 steps, not 5, before this task's edit — the row-level allowlist step 29-03 landed did not change the step count, only its body). Implemented the actual required behavior (one new step in each job) rather than the stale literal count: final counts are semver=7 (was 6) and e2e-platform-api=7 (was 5, +2), which correctly satisfies the parenthetical intent ('one more semver step and two more e2e-platform-api steps than before this task') even though the literal '6' in the acceptance text does not. Same class of plan-vs-measured discrepancy 29-03-SUMMARY.md already recorded for the baseline-version occurrence count."
  - "Worded the new e2e-platform-api job comment to avoid the literal substring 'lib --bins' adjacent, since the acceptance criterion 'git diff ... | grep -c \"lib --bins\" is 0' checks the DIFF, not just the final file — an unrelated pre-existing occurrence of that phrase already exists elsewhere in the job's own comment block and is untouched, but a new diff-visible occurrence would have falsely tripped the 'test job untouched' check."
  - "The new v0_9_config_boot CI guard step's threshold is 9 (not 'zero'), naming the exact count 29-01-SUMMARY.md records ('9 passed'), so the step's own name and message read 'fewer than 9' rather than reusing the literal phrase 'selected zero tests' from the sibling e2e_platform_api guard — the two guards are structurally identical but the threshold differs because 9 tests, not 1, is what a healthy run selects."

patterns-established:
  - "MIGRATION.md prose that describes a self-referential CI gate (a grep over this same file) must paraphrase the literal token the gate searches for ('marker'/'placeholder', never the four-letter word itself) in every location that touches the topic, including new CI-side YAML comments that mention the file."

requirements-completed: [SHIP-01, SHIP-02]

coverage:
  - id: D1
    description: "MIGRATION.md's four placeholder-marker occurrences (header ×2, §9.5, §9.8) are all closed with substantive content — citations of the actual tests/fixtures that prove each claim, or a completed operator checklist — and the file grew rather than shrank"
    requirement: SHIP-01
    verification:
      - kind: other
        ref: "grep -c TBD MIGRATION.md == 0; git diff --numstat HEAD~2..HEAD -- MIGRATION.md shows 71 insertions vs 16 deletions"
        status: pass
    human_judgment: false
  - id: D2
    description: "§9.6's two stale SHIP-02 forward-references and §9.7's pending-action clause are replaced with closed-state citations naming the real test files and fixture paths"
    requirement: SHIP-01
    verification:
      - kind: other
        ref: "grep -c 'Still owed' MIGRATION.md == 0; grep -cE 'owner SHIP-0[12], Phase 29' MIGRATION.md == 0; grep -q openapi_golden_v0_9 / openapi-v0.9.0.json MIGRATION.md"
        status: pass
    human_judgment: false
  - id: D3
    description: "§9.8 is one ordered, 7-step, copy-pasteable operator checklist naming only real paladin-cli subcommands (setup-check, maneuver validate, graph export, eval run) — no invented paladin-cli health or graph validate command appears anywhere in the file"
    requirement: SHIP-01
    verification:
      - kind: other
        ref: "awk '/^## 9\\.8 /,0' MIGRATION.md | grep -cE '^[0-9]+\\.' == 7; grep -c 'paladin-cli health' / 'graph validate' / 'graph-validate' MIGRATION.md all == 0; grep -c setup-check / 'eval run' MIGRATION.md >= 1"
        status: pass
    human_judgment: false
  - id: D4
    description: "A new semver-job CI step fails the build when MIGRATION.md carries any unfilled placeholder marker, proven fail-first against a scratch copy with the token reinserted, and passing against the tracked file"
    requirement: SHIP-01
    verification:
      - kind: integration
        ref: "Reproduced the step's body directly: exits 0 against tracked MIGRATION.md (COUNT=0); exits non-zero against a scratch copy with the token appended (COUNT=1)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The SHIP-02 v0_9_config_boot proof runs in CI on every PR via two new steps in the existing e2e-platform-api job, guarded against a zero/degraded test-selection count, with no new workflow job created"
    requirement: SHIP-02
    verification:
      - kind: integration
        ref: "cargo test --features web-server --test v0_9_config_boot -> test result: ok. 9 passed; 0 failed; python3 yaml load confirms job count unchanged (27) and step counts semver 6->7, e2e-platform-api 5->7"
        status: pass
    human_judgment: false

# Metrics
duration: ~35min
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 05: Close MIGRATION.md and gate it in CI Summary

**MIGRATION.md's last four placeholder markers are closed with real citations (`v0_9_config_boot`, `openapi_golden_v0_9.rs`, a 7-step operator checklist), a durable CI grep-gate now fails the build if any marker reappears, and the SHIP-02 boot proof actually runs on every PR via two new steps in the existing `e2e-platform-api` job.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Rewrote MIGRATION.md's header note to past tense, closed-state prose that never spells out the literal placeholder token (so the new CI gate below can't re-trip on the file's own explanatory prose).
- Closed §9.5's SHIP-02 placeholder with a citation of `v0_9_config_boot` (`tests/integration/v0_9_config_boot_test.rs`) naming both proof levels (config-resolution + behavioral/501-not-404) and the two-fixture deviation (`v0.9.0-config.test.yml` loaded, `v0.9.0-config.example.yml` documentation-only).
- Closed both of §9.6's stale SHIP-02 forward-references with a citation of `crates/paladin-web/tests/openapi_golden_v0_9.rs` and its frozen baseline `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json`, naming the `$ref`-closure scope and `info.version` as the one sanctioned normalisation.
- Closed §9.7 as a confirmed-empty fact (no `#[deprecated]` item shipped anywhere in the program) and replaced §9.8's placeholder with one ordered, 7-step operator checklist (backup → migrate → config → grace period → evaluator registration → deploy → verify), naming only real `paladin-cli` subcommands (`setup-check`, `maneuver validate`, `graph export`, `eval run`) and carefully avoiding the literal strings `paladin-cli health`/`graph validate`/`graph-validate` anywhere in the file.
- Added a new "no unfilled placeholder marker" step to the `semver` CI job, immediately after the existing row-level allowlist step, proven fail-first (passes against the tracked file, fails against a scratch copy with the token reinserted).
- Added two new steps to the existing `e2e-platform-api` job (no new workflow job): run `v0_9_config_boot` with the `web-server` feature, then a guard that fails if fewer than 9 tests were selected — the exact count `29-01-SUMMARY.md` records.
- Verified `cargo test --features web-server --test v0_9_config_boot` green locally: `test result: ok. 9 passed; 0 failed`.

## Task Commits

1. **Task 1: Close §9.5, §9.6, §9.7 and §9.8, and rewrite the header note as a closed state** — `0172a81a` (docs)
2. **Task 2: Add the placeholder-marker gate to the `semver` job and run the boot test in the existing `e2e-platform-api` job** — `499c2681` (ci)

**Plan metadata:** committed alongside this SUMMARY (see final commit).

## Files Created/Modified

- `MIGRATION.md` — header rewritten to closed-state prose; §9.5's SHIP-02 sentence replaced with a `v0_9_config_boot` citation; §9.6's two stale forward-references replaced with `openapi_golden_v0_9.rs` citations; §9.7 confirmed empty as a closed fact; §9.8 replaced with a 7-step operator checklist.
- `.github/workflows/ci.yml` — `semver` job gained a new "no unfilled placeholder marker" step; `e2e-platform-api` job gained two new steps (run `v0_9_config_boot`, guard against under-selection) and an updated name/leading comment recording the added scope.

## Decisions Made

- **Plan's literal step-count acceptance criteria (semver=6) do not match the measured pre-task baseline.** The semver job already had 6 steps at this HEAD before this task touched it (plan 29-03's row-level allowlist rewrite changed only that step's body, not the step count) — so adding one new step correctly makes it 7, not the plan's stated 6. Implemented the real required behavior (D-01's gate step must exist) rather than force-fitting a stale literal number; final measured counts are semver 6→7 and e2e-platform-api 5→7, both exactly "one/two more than measured before this task," which is the load-bearing invariant the acceptance criteria's own parenthetical explanation states. This is the same class of plan-vs-measured drift 29-03-SUMMARY.md recorded for the `--baseline-version 0.9.0` occurrence count (5 assumed, 3 measured) — a fast-moving base, not a defect in this task's work. Recorded here for the acceptance audit (D-12/29-07).
- **Reworded the new e2e-platform-api job comment to avoid the adjacent substring "lib --bins."** The acceptance criterion checks the DIFF (`git diff ... | grep -c 'lib --bins'` must be 0) to prove the `test` job itself is untouched. A pre-existing, unrelated occurrence of that phrase already lives elsewhere in the same job's original comment block (untouched by this task, so invisible in the diff), but my first draft of the new comment also used it descriptively, which WOULD have shown up in the diff and falsely tripped the check. Reworded to reference "the `test` job's scope (see its own comment above)" instead of repeating the literal flag sequence.
- **The new `v0_9_config_boot` CI guard's message says "fewer than 9," not "selected zero tests."** The threshold this guard checks is a minimum of 9 (the real, healthy test count), not zero — reusing the sibling `e2e_platform_api` guard's exact wording would have been technically wrong (a run selecting 1-8 tests is also a degraded/stale-filter condition this guard must catch, matching the plan's own instruction to guard against "fewer than the number of tests 29-01-SUMMARY.md records," not merely zero).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan's acceptance-criteria literal] Corrected the expected `semver`-job step count**
- **Found during:** Task 2, running the `<verify>` block's YAML step-count assertion
- **Issue:** The plan's acceptance criteria and `<verify>` block assert `len(j["semver"]["steps"])==6` after this task's edit. The measured step count in this HEAD's `semver` job BEFORE this task's edit was already 6 (not 5, as the plan's "one more semver step... than before this task" phrasing implies) — plan 29-03 rewrote the allowlist step's body without changing the job's step count. Adding the one new gate step this plan's own must-haves require therefore correctly produces 7, not 6.
- **Fix:** Implemented the actual required behavior (the D-01 gate step exists, immediately after the allowlist step) and verified the *relative* invariant instead of the stale absolute literal: measured before/after counts for both jobs (semver 6→7, e2e-platform-api 5→7), confirmed the total job count is unchanged (27→27), and confirmed every other named acceptance criterion (baseline-version count unchanged at 3, test job's `--lib --bins` diff-line count 0, `v0_9_config_boot` occurrence count ≥2, `selected zero tests` count ≥2) passes as literally stated.
- **Files modified:** `.github/workflows/ci.yml`
- **Verification:** `python3 -c "import yaml; ..."` — semver 7 steps, e2e-platform-api 7 steps, 27 total jobs (same as `git show HEAD~1` before this task).
- **Committed in:** `499c2681` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — a stale numeric literal in the plan's own acceptance criteria, not a defect in this task's implementation; the plan's stated intent, "one more step than measured before this task," is satisfied exactly)
**Impact on plan:** No scope creep. The gate step and the boot-test CI wiring exist exactly as D-01/D-09 require; only a hardcoded expected-count literal in the plan text needed correcting against measured reality, mirroring 29-03's own precedent for the same class of drift.

## Issues Encountered

None beyond the acceptance-criteria numeric discrepancy documented above.

## Known Stubs

None. Every citation added to MIGRATION.md points at a real, already-shipped test file or fixture (`v0_9_config_boot`, `openapi_golden_v0_9.rs`, both proven passing); every command named in the §9.8 checklist is a real `paladin-cli` subcommand verified against the clap `Commands`/`GraphCommands`/`ManeuverCommands`/`EvalCommands` enums at HEAD.

## Threat Flags

None. This plan is documentation and CI-configuration only — no new network endpoint, auth path, file-access pattern, or schema change at a trust boundary. Both STRIDE items this plan's own `<threat_model>` names (T-29-05-01 repudiation, T-29-05-02 DoS via an invented CLI command) are mitigated by the closed citations and the verified-absent forbidden strings documented above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `MIGRATION.md` is fully closed (zero placeholder markers) and gated in CI — plan 29-06 (the mdBook Upgrading page) can now mirror §9.1/§9.8 without citing anything still pending.
- The `semver` job's new gate and the `e2e-platform-api` job's new `v0_9_config_boot` steps are both live in `.github/workflows/ci.yml` for the phase's next CI run to prove on the real PR.
- The plan-vs-measured step-count discrepancy is recorded above for the SHIP-03 acceptance audit (29-07) to see; it does not block any downstream plan — the actual CI behavior (both gates present and proven) is correct.
- No blockers for 29-06 through 29-09.

## Self-Check: PASSED

- `MIGRATION.md` — FOUND, modified
- `.github/workflows/ci.yml` — FOUND, modified
- Commit `0172a81a` (Task 1) — FOUND in `git log --oneline --all`
- Commit `499c2681` (Task 2) — FOUND in `git log --oneline --all`
- `grep -c TBD MIGRATION.md` → `0` — CONFIRMED
- `grep -c 'Still owed' MIGRATION.md` → `0` — CONFIRMED
- `grep -cE 'owner SHIP-0[12], Phase 29' MIGRATION.md` → `0` — CONFIRMED
- `grep -c 'paladin-cli health' / 'graph validate' / 'graph-validate' MIGRATION.md` → all `0` — CONFIRMED
- `python3 -c "import yaml; ..."` → semver 7 steps, e2e-platform-api 7 steps, 27 total jobs (unchanged) — CONFIRMED
- `cargo test --features web-server --test v0_9_config_boot` → `test result: ok. 9 passed; 0 failed` — CONFIRMED
- Fail-first probe on a scratch copy of `MIGRATION.md` with the token reinserted → gate correctly exits non-zero — CONFIRMED

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
