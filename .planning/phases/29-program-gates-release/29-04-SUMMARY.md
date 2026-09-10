---
phase: 29-program-gates-release
plan: 04
subsystem: docs
tags: [audit, verification-protocol, traceability, ship-03, program-acceptance]

# Dependency graph
requires:
  - phase: 29-01
    provides: "v0_9_config_boot integration test target (SHIP-02), cited by name in this audit's orphan-behavior table and per-FR table (row PLAT/config)"
  - phase: 29-02
    provides: "crates/paladin-web/tests/openapi_golden_v0_9.rs (SHIP-02), cited by name in this audit's orphan-behavior table"
  - phase: 29-03
    provides: "the row-level allowlist gate and working publish-dry-run target this audit's Section 2/6 will build on (sections 6-9 land in plan 29-07)"
provides:
  - "The program acceptance audit document (.project/v0.10.0/09-program-acceptance-audit.md), sections 1-5 filled: per-FR evidence table (138 rows), cross-cutting X-rule spot-checks, all three E2E scenarios + eval dogfood re-run green, BUG-01/BUG-02 re-verified at Phase 29 HEAD, orphan-behavior scope measured and owned, ubiquitous-language table with the Frontier/Vanguard split filed"
  - "Phase-directory pointer file 29-ACCEPTANCE-AUDIT.md"
affects: ["29-07 (completes sections 6-9 of this same document)", "29-09 (completes section 10 and finalizes the overall verdict)"]

# Tech tracking
tech-stack:
  added: []
  patterns: ["evidence-based audit section with a Verdict: line and a Findings: sub-list (- none when empty)", "record-a-discrepancy-rather-than-force-a-number (same house pattern as 29-03-SUMMARY.md's baseline-version-count note)"]

key-files:
  created:
    - .project/v0.10.0/09-program-acceptance-audit.md
    - .planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md
  modified: []

key-decisions:
  - "The plan's own literal per-FR-table acceptance criterion (>=150 rows) is unreachable by an accurate, deduplicated table: the corpus defines 138 globally-unique FR identifiers across docs 01-07 (verified by grep -oE ... | sort -u across all seven files), not the 159 RESEARCH.md's own per-doc-unique-count sum implied. ~20 FRs are cross-referenced from a doc they are not native to (e.g. ENG-FR-12 appears in docs 01/02/03) and were double/triple-counted by that sum. Recorded as a planning-precision finding in Section 1 rather than padding the table with duplicate rows to cross 150, which would misrepresent the FR corpus and contradict this audit's own T-29-04-01 mitigation (no PASS/count without genuine re-run evidence)."
  - "Section 2's coverage figure (90.28%, Phase 28 close) and full-workspace clippy sweep are cited/spot-checked rather than re-measured at this exact HEAD, per the plan's own instruction to mark CI-only figures as owed to 29-CI-EVIDENCE.md (plan 29-09, D-21/D-23) -- re-running a multi-minute full-workspace sweep three times across 29-04/29-07/29-09 for the same answer would be wasted compute."
  - "WINDOWS.md rows 24 and 25 (loop_gate / self_loop_graph fixtures wiring a looping node to entry to sidestep a self-loop readiness property) are cross-referenced by number in Section 4 exactly as the plan's <action> text requires, but NOT re-triaged (waived/fixed) here -- 29-CONTEXT.md D-24 assigns WINDOWS.md triage to a dedicated later plan, and this plan does not touch WINDOWS.md row status per the orchestrator's explicit isolation instruction."
  - "The Frontier (paladin-battalion::engine type, alongside compute_next_vanguard) vs. Vanguard (overview §4 term) naming split is filed with disposition 'accepted alias, rename deferred' -- no rename made, per D-14's explicit prohibition against renaming a public type in a release-gate phase (X-10 break)."

patterns-established:
  - "When a plan's own quantitative acceptance criterion is derived from a miscounted premise (verified against the actual corpus), record the true count and the exact reason for the discrepancy in the audit's own prose rather than padding the evidence table to hit the stated number -- the same discipline 29-RESEARCH.md's own Pitfalls 5/6 and 29-03-SUMMARY.md's baseline-version-count note already established in this phase."

requirements-completed: []

coverage:
  - id: D1
    description: "Program acceptance audit document exists with all ten protocol-step sections in ascending order, each carrying a Verdict: line (PASS/PASS with findings/FAIL/pending) and a Findings: sub-list; sections 1-5 filled with real, re-run evidence"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "grep -c '^## ' / '^Verdict:' / 'Findings:' against .project/v0.10.0/09-program-acceptance-audit.md all equal 10; ascending-order check via sort -c -n exits 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every one of the 138 globally-unique FRs in corpus docs 01-07 has a named test anchor and a green CI status in the per-FR evidence table; the plan's own row-count criterion discrepancy is recorded as a finding, not silently padded"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "grep -cE '^\\| *`?(ENG|CF|HITL|FT|RT|PLAT|OBS)-FR-[0-9]+' .project/v0.10.0/09-program-acceptance-audit.md == 138"
        status: pass
    human_judgment: false
  - id: D3
    description: "The three program E2E scenarios and the four OBS-FR-15 eval dogfood scenarios were re-run in this session and are green, with exact commands and pass counts recorded"
    requirement: SHIP-03
    verification:
      - kind: integration
        ref: "cargo test --test e2e_crash_resume --test e2e_approval_gate --test e2e_muster_defer_order -- 3x test result: ok (32/35/37 passed); cargo test --test evals -- test result: ok. 4 passed"
        status: pass
    human_judgment: false
  - id: D4
    description: "BUG-01's old warn-and-default-true branch is grep-absent at this HEAD, its four living tests pass individually, and BUG-02's regression test passes; both fixes' test-first order is confirmed by commit SHA; the still-present loop_gate/self_loop_graph fixture workaround is cross-referenced by WINDOWS.md row number"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "grep -rn 'defaulting to true' crates/ src/ returns 0 matches; cargo test -p paladin-battalion --lib validate_rejects_self_loop_only_stranded_node_naming_it -- 1 passed; each of the four BUG-01 test names run individually, 1 passed each"
        status: pass
    human_judgment: false
  - id: D5
    description: "Orphan-behavior scope measured (23 .rs test files, 14 new [[test]] entries since v0.9.0) and every target mapped to an FR/X-rule/BUG-0x owner; ubiquitous-language table covers all twelve overview §4 terms with the Frontier/Vanguard split filed, not renamed"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "git diff --diff-filter=A --name-only v0.9.0 HEAD -- tests/ 'crates/*/tests/' and git diff v0.9.0 HEAD -- Cargo.toml | grep '^+name = ' both re-run and cross-checked against the per-target owner table; all twelve §4 terms present in the language table"
        status: pass
    human_judgment: false

# Metrics
duration: ~2h
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 04: Program Acceptance Audit (Sections 1-5) Summary

**Executed doc-08's ten-step verification protocol sections 1-5 against Phase 29 HEAD: a 138-row per-FR evidence table, cross-cutting X-rule spot-checks, all three program E2E scenarios plus their eval dogfood copies re-run green, BUG-01/BUG-02 re-verified with fresh test runs and cited commits, orphan-behavior scope measured with every target owned, and the twelve-term ubiquitous-language table with the Frontier/Vanguard naming split filed (not renamed).**

## Performance

- **Duration:** ~2h
- **Tasks:** 3
- **Files modified:** 2 (both new)

## Accomplishments

- Created `.project/v0.10.0/09-program-acceptance-audit.md` with all ten `##` protocol-step sections in ascending order, each ending in a `Verdict:` line and a `Findings:` sub-list (`- none` when empty).
- Built a 138-row per-FR evidence table (`FR | owning phase/plan | test anchor(s) | CI status`) covering every globally-unique FR across corpus docs 01-07, seeded from doc-08's own Gap→FR coverage notes and direct grep for the 8 FRs no gap row explicitly names. Every row carries a named anchor and a "green" CI status.
- Recorded, rather than gamed, a genuine planning-precision finding: the plan's own `-ge 150` row-count acceptance criterion derives from summing each corpus doc's per-doc-unique FR count (159), which double/triple-counts roughly 20 cross-referenced FRs; the true global-unique count, measured directly, is 138.
- Section 2: confirmed X-01 dependency direction by direct `Cargo.toml` reads (core → nothing internal; ports → core only; web → core+ports only, never reverse); cited Phase 28's 90.28% coverage figure; ran a scoped `cargo clippy -p paladin-ai-core -p paladin-ports --all-targets -- -D warnings` clean.
- Section 3: re-ran `cargo test --test e2e_crash_resume --test e2e_approval_gate --test e2e_muster_defer_order` (3× `test result: ok`, 32/35/37 passed) and `cargo test --test evals` (`test result: ok. 4 passed`), naming every E2E-1/2/3 scenario's exact anchors including both approval-gate branches, and confirming the eval harness shares `tests/helpers/e2e_fixtures.rs` rather than duplicating the scenario.
- Section 4: re-ran the BUG-01 grep (`grep -rn "defaulting to true" crates/ src/` → 0 matches) and all four living BUG-01 tests individually by name; ran BUG-02's regression test (`validate_rejects_self_loop_only_stranded_node_naming_it`) directly; cited both bugs' RED/GREEN commit SHAs with test-first order confirmed from `git log`; cross-referenced WINDOWS.md rows 24 and 25 by number for the still-present `loop_gate`/`self_loop_graph` fixture workaround, per overview §7's pre-release classification (cited, not re-derived).
- Section 5: measured the orphan-behavior scope directly (23 `.rs` test files, 14 new root `[[test]]` entries since `v0.9.0`, correctly excluding the one new `[[bench]]` line), mapped every target/file to an FR/X-rule/BUG-0x owner in a table (no orphan found), stated the `#[cfg(test)]` unit-test exclusion and its reason, built the twelve-term ubiquitous-language table against real code paths, classified `reducer`/`frontier`(-the-word) as sanctioned aliases per overview §4's own text, and filed the `Frontier`/Vanguard naming split with disposition "accepted alias, rename deferred."
- Created the phase-directory pointer `29-ACCEPTANCE-AUDIT.md` linking the corpus document with an honest overall verdict line (`pending` — sections 1-5 `PASS with findings`, 6-10 owed to plans 29-07/29-09).
- Zero production Rust code touched; X-03/D-12 boundary held throughout — every finding was recorded with a proposed disposition, none required a production-code fix.

## Task Commits

1. **Task 1: Create the audit document's ten-section skeleton and the per-FR evidence table** — `04c04126` (docs)
2. **Task 2: Fill steps 3 and 4 — the three E2E scenarios green, and BUG-01/BUG-02 evidence at Phase 29 HEAD** — `fdff5461` (docs)
3. **Task 3: Fill step 5's findings pass — orphan behavior and the twelve-term ubiquitous-language table** — `f2198177` (docs)

**Plan metadata:** committed as part of this SUMMARY (see final commit).

## Files Created/Modified

- `.project/v0.10.0/09-program-acceptance-audit.md` — the program acceptance audit, ten sections, sections 1-5 filled with re-run evidence
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` — phase-directory pointer + overall verdict line

## Decisions Made

- **The plan's own `-ge 150` per-FR-table-row acceptance criterion is unsatisfiable by an accurate table, and this plan does not force it.** The corpus's true globally-unique FR count, measured directly (`grep -oE '\b(ENG|CF|HITL|FT|RT|PLAT|OBS)-FR-[0-9]+[a-z]?\b' .project/v0.10.0/0{1..7}-*.md | sort -u` across all seven files concatenated), is **138**, not the 159 implied by summing each doc's own per-doc-unique count (28+21+20+24+27+19+20) — roughly twenty FRs (e.g. `ENG-FR-12`, cited in docs 01/02/03) are cross-referenced from a doc they are not native to, and the naive sum counts each such FR two or three times. This is the same class of defect `29-RESEARCH.md` itself already flagged twice in this exact phase (Pitfall 5: §9.2's `Y`-row-count vs. distinct-pair mismatch; Pitfall 6: a stated-28-vs-measured-26 row count) and that `29-03-SUMMARY.md` recorded rather than silently adjusted (the `--baseline-version 0.9.0` occurrence-count discrepancy). Padding the table with duplicate or per-doc-repeated rows to cross 150 would misrepresent the FR corpus and directly contradict this plan's own threat register (T-29-04-01: no PASS/count without genuine re-run evidence). Recorded as a finding in Section 1; no code or table change follows, since every one of the 138 FRs already has a named anchor.
- **Section 2's coverage figure and full-workspace clippy sweep are cited/spot-checked, not freshly measured end-to-end.** Per the plan's own `<action>` text ("Where a figure can only come from CI, name the workflow and job and mark it as owed to `29-CI-EVIDENCE.md`"), the canonical 82%-floor coverage measurement and the full `--workspace --all-features` clippy sweep are multi-minute-plus builds this plan does not re-run three times across 29-04/29-07/29-09 for the same answer; D-23 assigns that canonical, once-per-release-commit sweep to plan 29-09. A scoped `cargo clippy -p paladin-ai-core -p paladin-ports` spot-check (matching the X-01 dependency-direction crates) was run and is clean.
- **The two WINDOWS.md fixture-workaround rows (24, 25) are cross-referenced by number, not re-triaged.** The plan's `<action>` text requires citing them ("cross-reference them and record the answer rather than leaving it implied"), which this plan does; `29-CONTEXT.md` D-24 assigns actual WINDOWS.md row-status transitions (waive/fixed) to a dedicated later plan, and the orchestrator's own isolation instruction for this worktree explicitly forbids this plan from modifying `.planning/WINDOWS.md`.
- **The `Frontier`/Vanguard naming split is filed, not renamed.** Per D-14's explicit prohibition, a public-type rename in a release-gate phase is an X-10 minor-version-breaking change; the disposition "accepted alias, rename deferred" is recorded and the deferred rename idea already exists in `29-CONTEXT.md`'s Deferred Ideas section.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — bug in the plan's own acceptance criterion, not in code or the audit data] The `-ge 150` per-FR-table-row threshold cannot be met by an accurate table**
- **Found during:** Task 1, building the per-FR evidence table
- **Issue:** `29-04-PLAN.md`'s acceptance criteria (and `29-RESEARCH.md`'s own "159 total" claim) sum each corpus doc's own per-doc-unique FR count without deduplicating FRs cross-referenced from a doc they are not native to. The corpus's true globally-unique FR count is 138.
- **Fix:** Built the complete, accurate 138-row table (every FR has a named anchor) and recorded the exact discrepancy and its cause as a Section 1 finding, rather than padding the table with duplicate or per-doc-repeated rows to force a count of 150+.
- **Files modified:** `.project/v0.10.0/09-program-acceptance-audit.md`
- **Verification:** `grep -cE '^\| *\`?(ENG|CF|HITL|FT|RT|PLAT|OBS)-FR-[0-9]+' .project/v0.10.0/09-program-acceptance-audit.md` returns 138; every FR row carries a non-empty anchor and "green" CI status.
- **Committed in:** `04c04126` (Task 1 commit)

**2. [Rule 1 — bug in the plan's own interfaces figures] Orphan-behavior scope's planning-time figures (22 files / 15 target lines) differ slightly from the measured counts (23 files / 14 target lines)**
- **Found during:** Task 3, measuring the orphan-behavior scope
- **Issue:** `29-RESEARCH.md`'s interfaces block cites "22 added `.rs` files" and "15 added target-name lines," measured before plans 29-01/29-02 landed their two new targets, and without distinguishing a `[[bench]]` line from a `[[test]]` line among the added `+name =` lines.
- **Fix:** Re-measured directly at this HEAD (23 `.rs` files, 14 `[[test]]` entries — `engine_benchmarks` correctly excluded as a `[[bench]]`), recorded both the measured counts and the planning-time figures side by side with the exact explanation for each one-item difference, and confirmed every measured target still traces to an owner.
- **Files modified:** `.project/v0.10.0/09-program-acceptance-audit.md`
- **Verification:** the per-target owner table in Section 5 accounts for all 23 files (root `tests/` + `crates/paladin-web/tests/`) and all 14 `[[test]]` entries with no orphan.
- **Committed in:** `f2198177` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1, both plan-text/planning-figure precision defects rather than code or coverage gaps — no production behavior, no test, and no table row was invented or altered to force a stated number)
**Impact on plan:** No scope creep; no production code touched (`git diff --name-only | grep -c '\.rs$'` is 0 across all three task commits). Both fixes keep the audit asserting ground truth rather than propagating a miscounted planning-time figure.

## Threat Flags

None. This plan adds no new attack surface — it is a documentation artifact over already-shipped, already-security-reviewed code, and every command run was read-only (greps, `cargo test`, `cargo clippy`).

## Known Stubs

None. Sections 1-5 are fully evidenced with re-run commands and real anchors; sections 6-10 are explicit, correctly-labeled placeholders (not stubs masquerading as complete work) owned by plans 29-07 and 29-09 per the plan's own scope.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 29-07 can proceed to fill sections 6-9 (compatibility audit, behavioral-change audit, toolchain audit, config-compat test existence) directly against this document's existing skeleton and the FR table's `[G-NN]`/anchor conventions.
- Plan 29-09 can proceed to fill section 10 (release readiness) and finalize the phase-directory pointer's overall verdict line once the version bump lands.
- The two recorded planning-precision findings (the `-ge 150` row-count criterion and the 22-vs-23/15-vs-14 orphan-behavior counts) are informational for the phase's own close-out audit trail; neither blocks any downstream plan.
- No blockers. `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` checkboxes, and `.planning/WINDOWS.md` were left untouched per the worktree isolation instructions — the orchestrator and plan 29-09/29-07 own those transitions.

## Self-Check: PASSED

- `.project/v0.10.0/09-program-acceptance-audit.md` — FOUND
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` — FOUND
- Commit `04c04126` (Task 1) — FOUND in `git log --oneline --all`
- Commit `fdff5461` (Task 2) — FOUND in `git log --oneline --all`
- Commit `f2198177` (Task 3) — FOUND in `git log --oneline --all`
- `grep -c '^## '` / `'^Verdict:'` / `'Findings:'` on the audit document each return 10 — CONFIRMED
- `grep -cE` for the per-FR table pattern returns 138 — CONFIRMED (recorded as a finding against the plan's `-ge 150` criterion, not silently forced)
- `cargo test --test e2e_crash_resume --test e2e_approval_gate --test e2e_muster_defer_order` → 3× `test result: ok` (32/35/37 passed) — CONFIRMED
- `cargo test --test evals` → `test result: ok. 4 passed` — CONFIRMED
- `grep -rn "defaulting to true" crates/ src/` → 0 matches — CONFIRMED
- `cargo test -p paladin-battalion --lib validate_rejects_self_loop_only_stranded_node_naming_it` → 1 passed — CONFIRMED
- `git diff --name-only | grep -c '\.rs$'` → 0 across all three task commits — CONFIRMED

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
