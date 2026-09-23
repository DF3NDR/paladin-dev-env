---
phase: 34-documentation-currency-audit
plan: 07
subsystem: docs
tags: [documentation-audit, rustdoc, cargo-doc, all-features, ci-gate, doctest, adr-0033]

# Dependency graph
requires:
  - phase: 34-06
    provides: 34-AUDIT.md's §3 default-feature enumeration (RD-01..RD-66), the reusable 34-rustdoc-rows.sh parser, and the workspace all-features run recorded as a partial floor (not an enumeration)
provides:
  - 34-AUDIT.md §3 per-crate all-features enumeration — 77 new RD-67..RD-143 rows across all twelve crates (eight red, four green), the true all-features floor Phase 36 sizes against
  - 34-rustdoc-rows.sh extended with an optional `<crate-override>` argument, making it reusable against a single-crate `-p <crate>` capture (which never prints the per-crate summary line the default parser depends on) without breaking its existing default-feature code path
  - 34-AUDIT.md §3 doctest baseline subsection — cargo test --workspace --doc, 462 passed / 0 failed / 210 ignored, green, 0 rows
  - 34-AUDIT.md §3 entry-point `# Examples`-heading gate record — 101 derived entry points (+25 drift against the frozen 76-item Phase 16 enumeration), 19 MISSING, gate exits 1, routed to deferred-items.md
  - §3 close: counted totals (143 total RD-nn rows, both D-12 bars quoted verbatim, HEAD SHA) satisfying ROADMAP Success Criterion 2 in full
affects: [34-08-examples-sweep, 34-09-work-list-assembly, 36-rustdoc-remediation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-crate rustdoc capture attribution: a single-crate `cargo doc -p <crate> --all-features --no-deps` invocation never prints the per-crate 'generated N warnings/errors' summary line a workspace-wide capture always does — under -D warnings the job aborts on its first content diagnostic before any summary line would print, and a clean crate's own closing 'Generated .../index.html' line is a different pattern entirely. Attribution for this shape is therefore a crate-override, not a summary-line consumption: the invoking script already knows the crate (it is the -p argument), so trust that instead of re-deriving it from stream content."

key-files:
  created:
    - .planning/phases/34-documentation-currency-audit/34-evidence/34-07-percrate/ (12 per-crate capture files)
    - .planning/phases/34-documentation-currency-audit/34-evidence/34-07-doctests.txt
    - .planning/phases/34-documentation-currency-audit/34-evidence/34-07-public-api-examples.txt
  modified:
    - .planning/phases/34-documentation-currency-audit/34-AUDIT.md
    - .planning/phases/34-documentation-currency-audit/34-EVIDENCE.md
    - .planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh
    - .planning/phases/34-documentation-currency-audit/deferred-items.md

key-decisions:
  - "34-rustdoc-rows.sh's crate-attribution algorithm cannot be reused unmodified for a per-crate capture, contrary to 34-06-SUMMARY.md's forward note — the precondition check's own dry run against a fresh paladin-memory capture failed with 'diagnostic block(s) never attributed to a crate' before any of the eleven-crate sweep ran, proving live (not assumed) that a -p <crate> invocation never emits the summary line the attribution pass depends on."
  - "Fixed via a Rule 3 (blocking-issue) script extension, not hand-transcription: added an optional third <crate-override> argument that bypasses summary-line attribution entirely and assigns every parsed block directly to the named crate — correct by construction for a single-crate invocation. The existing default-feature (--workspace) code path is provably unchanged (re-running the parser against the already-committed 34-06 capture without the new argument reproduces byte-identical output)."
  - "The Run-cell text and the evidence-anchor path were corrected in the same edit: a crate-override run's Run cell now quotes the actual -p <crate> invocation (D-12/D-14) instead of always citing the workspace command's D-12/D-13 text, and the evidence anchor recovers the full 34-evidence/... suffix from the capture's own path instead of assuming every capture sits one level below 34-evidence/ directly (this plan's captures sit a level deeper, at 34-evidence/34-07-percrate/<crate>.txt)."
  - "The true all-features floor is 77 content errors across 8 of 12 crates, not 14 (paladin-ai-core alone, the figure the roadmap, 34-CONTEXT.md D-14 and STATE.md's Phase 32 close note all still carry) and not 17 (plan 34-06's own workspace-run partial view). This run's totals match RESEARCH.md's independently-measured Pattern 2 table exactly, crate-for-crate and error-for-error, at a HEAD several commits later — confirming the D-22 .planning/-only invariance argument holds in practice."
  - "The entry-point # Examples-heading gate's scope drift (76 frozen -> 101 derived) and its 19 current MISSING violations are routed to deferred-items.md per D-00e/D-19, not minted as RD-nn/EX-nn/MB-nn rows and not fixed — the finding is about the rule's own apparatus (ungated, undermaintained), not about any single page, diagnostic or example."

requirements-completed: [CURR-02, CURR-05]

coverage:
  - id: D1
    description: "All twelve crates (eleven library crates plus the facade) are swept individually under RUSTDOCFLAGS=\"-D warnings\" cargo doc -p <crate> --all-features --no-deps, with every content error enumerated as its own RD-nn row (crate, real file:line, kind, verbatim message, location source, evidence anchor, size) — the true all-features floor D-14 requires, since the single --workspace invocation only ever surfaces a scheduling-dependent partial subset before its own concurrency-driven abort."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "12 parser runs each printing 'RECONCILED: N content diagnostics == N rows emitted' (36+0+14+0+0+9+1+0+1+1+8+7=77, exit 0 each); grep -c '^| RD-[0-9]' 34-AUDIT.md == 143; grep -cE '^\\| RD-[0-9]+ \\|[^|]*\\|[^|]*\\| [A-Za-z0-9_./-]+\\.rs:[0-9]+ ' 34-AUDIT.md == 143 (every row ends with a real file:line)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The known-answer HeuristicTokenCounter link (WINDOWS.md row 37, CONTEXT.md's own method self-test) appears in the per-crate enumeration at crates/paladin-memory/src/token_counter/mod.rs:3, verified before the remaining ten-crate sweep ran."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "grep -q 'crates/paladin-memory/src/token_counter/mod.rs:3' 34-AUDIT.md (RD-126, and re-verified as the first crate swept, before the other ten)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The doctest baseline (cargo test --workspace --doc, default features per RESEARCH.md Pitfall P-08) is measured and recorded — 462 passed, 0 failed, 210 ignored, green, 0 rows minted — because the coverage gate and the --tests selector both skip doctests entirely."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "34-evidence/34-07-doctests.txt (exit 0, 13 per-crate 'test result:' lines summing to 462/0/210); grep -q 'cargo test --workspace --doc' 34-AUDIT.md; grep -c 'cargo test --workspace --all-features --doc' 34-AUDIT.md == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "The public-API # Examples-heading gate (scripts/check-public-api-examples.sh) is run in both gate and list mode, its 101-item derived set and 19-item MISSING violation table are recorded in full, its drift against the frozen 76-item Phase 16 enumeration is stated in numbers, and its disposition is routed to deferred-items.md rather than fixed or absorbed into an RD-nn/EX-nn/MB-nn row (D-00c, D-00e)."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "34-evidence/34-07-public-api-examples.txt (gate exit 1, list mode 'TOTAL: 101 entry points -- 82 OK, 19 MISSING, 0 SINGULAR'); deferred-items.md '## Plan 34-07, Task 2' entry; git status --porcelain -- crates src scripts empty (no violation fixed, no script edited)"
        status: pass
    human_judgment: false
  - id: D5
    description: "§3 closes with a summary subsection whose every figure is counted from the rows and captures on disk (default-run total, per-crate all-features total, doctest result, entry-point gate result, total RD-nn count, HEAD SHA) rather than recalled from RESEARCH.md or CONTEXT.md, satisfying ROADMAP Success Criterion 2 in full."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "grep -A20 '§3 close — counted totals' 34-AUDIT.md; total row count 143 reconciles against 34-check.sh assertion (b) (no duplicate ID)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Phase remains read-only outside .planning/ across both task commits (SC5/D-22/CURR-05); 34-check.sh --seed stays green after each commit; config.json (the orchestrator's ephemeral _auto_chain_active flag) is never staged."
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "git status --porcelain -- . ':!.planning' empty after each of the two task commits (97c4aa3b, 1594cd6d); bash 34-check.sh --seed PASS on all 5 assertions after each commit; git show --stat on both commits confirms .planning/config.json never appears"
        status: pass
    human_judgment: false

duration: ~16min
completed: 2026-09-17
status: complete
---

# Phase 34 Plan 07: Per-Crate Rustdoc Sweep, Doctest Baseline & Entry-Point Gate Summary

**Swept all twelve crates individually under `-D warnings --all-features`, finding the true floor to be 77 content errors across 8 red crates (not the carried "14 unresolved links in paladin-ai-core") — a 5.5x undercount that would have undersized Phase 36's work list by 63 items — plus a green 462/0/210 doctest baseline and a routed-to-deferred 101-item `# Examples`-heading gate record (19 MISSING, +25 drift against the frozen 76-item Phase 16 enumeration).**

## Performance

- **Duration:** ~16 min
- **Started:** 2026-09-17T05:43:30Z (approx., end of plan 34-06)
- **Completed:** 2026-09-17T05:59:33Z
- **Tasks:** 2 completed
- **Files modified:** 16 (4 modified, 12+2 created: 12 per-crate captures + 2 baseline captures)

## Accomplishments
- Precondition check found `34-rustdoc-rows.sh` (written by plan 34-06) cannot parse a single-crate `-p <crate>` capture unmodified — a `-p <crate>` invocation never prints the per-crate "generated N warnings/errors" summary line the attribution pass depends on (confirmed live against both a red capture, `paladin-memory`, and a green one, `paladin-herald`) — fixed via a Rule 3 extension: an optional `<crate-override>` argument that bypasses summary-line attribution and assigns every block directly to the named crate; the existing default-feature code path is provably unchanged (byte-identical re-run against 34-06's own committed capture)
- Ran `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps` live for all twelve crates (eleven library crates plus the facade `paladin-ai`, derived from the tree, matching RESEARCH.md Pattern 2's list): 8 red (`paladin-battalion` 36, `paladin-ai-core` 14, `paladin-llm` 9, `paladin-memory` 1, `paladin-ports` 1, `paladin-storage` 1, `paladin-web` 8, `paladin-ai` 7 = 77 total), 4 green (`paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-notifications`), 68s combined wall time, matching RESEARCH.md's independently-measured table exactly
- Verified the known-answer `HeuristicTokenCounter` case (`crates/paladin-memory/src/token_counter/mod.rs:3`, WINDOWS.md row 37) resolves correctly *before* trusting the remaining ten-crate sweep, per the plan's own gating instruction
- Pasted all 77 rows into `34-AUDIT.md` §3 as `RD-67..RD-143` under twelve per-crate subheadings, each with an inline row-count reconciliation, plus a twelve-row summary table (crate, exit, error count, wall time) and prose stating the 5.5x undercount (77 vs the carried "14") and the 63-item work-list undersizing this would otherwise have caused
- Ran `cargo test --workspace --doc` under default features (462 passed, 0 failed, 210 ignored, green — matching RESEARCH.md exactly) and both modes of `scripts/check-public-api-examples.sh` (101 derived entry points, 19 MISSING, gate exit 1, +25 drift against the frozen 76-item Phase 16 enumeration; confirmed no CI job or `make` target runs it)
- Routed the entry-point gate's scope drift and 19 violations to `deferred-items.md` under a new `## Plan 34-07, Task 2` heading (append-only, D-00e/D-19) rather than fixing any violation or extending the rule's enumerated set
- Closed §3 with a counted-totals subsection: 143 total `RD-nn` rows, both D-12 bars quoted verbatim, doctest and entry-point results, HEAD SHA — satisfying ROADMAP Success Criterion 2 in full
- Appended 23 numbered evidence rows (128-150) to `34-EVIDENCE.md` across the two tasks' sections

## Task Commits

1. **Task 1: Per-crate all-features sweep across all twelve crates, every error enumerated** — `97c4aa3b` (docs)
2. **Task 2: Doctest baseline and the public-API example-heading gate record** — `1594cd6d` (docs)

_Both commits touch only `.planning/`; no TDD cycle applies to this documentation-only plan._

## Files Created/Modified
- `.planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh` — extended with an optional `<crate-override>` argument, a per-crate Run-cell string, and a depth-agnostic evidence-anchor path (modified)
- `.planning/phases/34-documentation-currency-audit/34-evidence/34-07-percrate/*.txt` — twelve verbatim per-crate `cargo doc` captures (created)
- `.planning/phases/34-documentation-currency-audit/34-evidence/34-07-doctests.txt` — verbatim `cargo test --workspace --doc` capture (created)
- `.planning/phases/34-documentation-currency-audit/34-evidence/34-07-public-api-examples.txt` — verbatim gate-mode and list-mode `check-public-api-examples.sh` capture (created)
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` — §3 per-crate all-features enumeration (77 new RD rows), doctest subsection, entry-point gate subsection, §3 close (modified)
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` — evidence rows 128-150 appended (modified)
- `.planning/phases/34-documentation-currency-audit/deferred-items.md` — `## Plan 34-07, Task 2` entry appended (modified)

## Decisions Made
- The crate-attribution mechanism plan 34-06 wrote cannot be reused unmodified for a per-crate capture — discovered live by the precondition check, not assumed from 34-06-SUMMARY.md's forward note, and fixed as a Rule 3 blocking-issue extension rather than by hand-transcribing 77 rows.
- The Run-cell text and evidence-anchor path were corrected in the same script edit (crate-specific invocation quoted per row; anchor recovers the full `34-evidence/...` suffix rather than assuming a fixed capture depth) — both were required for the per-crate rows to be individually reproducible and correctly evidenced.
- 77, not 14 and not 17, is recorded as the true all-features floor, with the 5.5x undercount and 63-item undersizing stated explicitly in numbers rather than left for Phase 36 to discover.
- The entry-point gate's scope drift and violations are deferred-register findings (D-00e/D-19), not `RD-nn`/`EX-nn`/`MB-nn` rows — the finding is about the rule's own ungated apparatus, not any single page, diagnostic, or example.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Extended `34-rustdoc-rows.sh` with a crate-override argument**
- **Found during:** Task 1's own precondition check
- **Issue:** The parser's crate-attribution pass consumes each crate's "generated N warnings/errors" summary line to know how many pending diagnostics belong to it — a line a `--workspace` capture always emits but a single-crate `-p <crate>` invocation never does (the job aborts on its first content diagnostic under `-D warnings` before any summary line would print; a clean crate's own closing line is a different pattern). Running the parser unmodified against a fresh `paladin-memory` capture failed with `FATAL: 1 diagnostic block(s) never attributed to a crate`.
- **Fix:** Added an optional third `<crate-override>` argument that bypasses summary-line attribution entirely and assigns every parsed block directly to the named crate — correct by construction for a `-p <crate>` invocation. Also corrected the Run-cell text (per-crate invocation instead of the workspace command) and the evidence-anchor path (recovers the full `34-evidence/...` suffix instead of assuming a fixed depth) in the same edit, since both were wrong for this task's deeper capture directory.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh`
- **Verification:** Known-answer case (`paladin-memory`) resolves correctly with the new argument; re-running the parser against 34-06's own committed default-feature capture without the new argument reproduces byte-identical output (regression-checked before the ten-crate sweep proceeded).
- **Committed in:** `97c4aa3b` (Task 1)

**2. [Rule 1 - Bug] Reworded three §3-close bullet points that repeated already-minted `RD-nn` literal IDs in prose**
- **Found during:** Task 2's first `34-check.sh --seed` run after adding the "§3 close — counted totals" subsection
- **Issue:** `34-check.sh` assertion (b) flagged `RD-01`, `RD-02`, `RD-66`, `RD-67`, `RD-143` as duplicates because the summary prose cited row-range boundaries by literal ID string ("`RD-02`..`RD-66`", "`RD-67`..`RD-143`", "`RD-01`..`RD-143`") — the same recurring false-positive class every prior plan in this phase (34-01 through 34-06) has already hit and fixed.
- **Fix:** Reworded every boundary reference positionally ("spanning the default-feature enumeration's own row range", "spanning this task's own twelve per-crate subheadings' row range immediately above", "spanning the tracer row through this task's own last per-crate row") instead of repeating literal ID strings.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `bash 34-check.sh --seed` assertion (b) passes after the fix; `grep -oE 'RD-[0-9]+' 34-AUDIT.md | sort | uniq -d` prints nothing.
- **Committed in:** `1594cd6d` (Task 2)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking-issue script extension, 1 Rule 1 bug — the same recurring literal-ID-collision class every prior plan in this phase has caught)
**Impact on plan:** Neither fix touches anything outside `.planning/`, and neither changes any already-committed row's crate, file:line, kind, or size. Both were required to make the plan's own `<verify>` blocks and `34-check.sh --seed` pass as specified — the script extension in particular was the exact "extend the tool rather than hand-transcribe" outcome the precondition asked for.

## Issues Encountered
None beyond the two auto-fixed deviations above — both were anticipated failure modes the plan's own precondition and phase-specific rules were written to catch.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- `34-AUDIT.md` §3 is now fully closed: default-feature enumeration (plan 34-06, `RD-01..RD-66`), per-crate all-features enumeration (this plan, `RD-67..RD-143`), doctest baseline (green), entry-point gate record (routed to deferred), and a counted-totals close subsection — 143 total `RD-nn` rows, every one ending in a real `.rs:<line>`, no duplicate ID.
- `34-rustdoc-rows.sh` is now reusable against either a `--workspace` capture or a single-crate `-p <crate>` capture via its optional third argument — plan 34-08 (or any future rustdoc re-sweep) can reuse it unmodified against either shape.
- Phase 36's `RD-nn` work list can now size against the true floor (77, not 14) — the 63-item undersizing this plan corrected is the single highest-value finding of this phase, per the plan's own objective statement.
- The entry-point `# Examples`-heading gate's scope drift and 19 violations are recorded in `deferred-items.md` under `## Plan 34-07, Task 2`, ready for plan 34-09's §7 pointer.
- `34-check.sh --seed` remains green after both task commits; `--final` mode's three additional assertions are still not exercised (expected — plan 34-09 is the first plan that runs `--final`, once §4/§5/§6/§7 are populated by plan 34-08 and 34-09).
- No blockers. Plan 34-08 (the examples sweep) is next.

---
*Phase: 34-documentation-currency-audit*
*Completed: 2026-09-17*

## Self-Check: PASSED

All created/modified files found on disk (`34-rustdoc-rows.sh`, all twelve `34-evidence/34-07-percrate/*.txt` captures, `34-evidence/34-07-doctests.txt`, `34-evidence/34-07-public-api-examples.txt`); both task commits (`97c4aa3b`, `1594cd6d`) found in `git log`.
