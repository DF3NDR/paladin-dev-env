---
phase: 34-documentation-currency-audit
plan: 06
subsystem: docs
tags: [documentation-audit, rustdoc, cargo-doc, ci-gate, adr-0033]

# Dependency graph
requires:
  - phase: 34-05
    provides: 34-AUDIT.md's closed 93-page mdBook partition (§2), the D-07 per-page evidence method, RD-01/EX-01 tracer rows already worked in §3/§4
provides:
  - 34-AUDIT.md §3 default-feature enumeration — 65 new RD-02..RD-66 rows (66 total with the RD-01 tracer), every row carrying a real crate/file:line/kind/message/location-source/evidence-anchor/size
  - 34-rustdoc-rows.sh — a reusable, generically-written parser (crate/src-dir map built live from Cargo.toml, never hardcoded) that plan 34-07's per-crate sweep can reuse unmodified against an error:-prefixed capture
  - The crate-attribution algorithm (summary-line/abort-boundary chunking against the pending diagnostic queue, validated against every `-->`-bearing diagnostic's own path) needed to correctly attribute a location-less diagnostic in a workspace-wide, concurrently-scheduled cargo doc run — not documented anywhere in RESEARCH.md, discovered and proven live by this plan
  - 34-AUDIT.md §3 workspace all-features subsection — this run's 17-error, 4-crate partial view (paladin-memory 1, paladin-web 8, paladin-storage 1, paladin-ai facade 7), explicitly not minting rows, with D-14's single-crate abort prose corrected by a third independent measurement
affects: [34-07-per-crate-rustdoc-sweep, 34-09-work-list-assembly, 36-rustdoc-remediation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Content-based crate/location attribution for a concurrently-scheduled cargo doc run: never trust stream position (a diagnostic printing before crate X's own abort/summary boundary can genuinely belong to crate Y) — always derive the crate either from the diagnostic's own rustdoc `-->` span, or by grepping the exact quoted source-line snippet (not the bare bracket identifier alone, which collapses same-identifier warnings at different lines) restricted to doc-comment lines inside the correct crate's own src/ tree."
    - "Per-crate summary-line chunking: cargo batches a `<crate> (lib doc) generated N warnings` line per crate but flushes those lines together at the end of a shared concurrent-scheduling window, not immediately after each crate's own diagnostics — the pending diagnostic queue is consumed front-to-back against each summary's own count, in the order the summaries appear, which correctly reconstructs true crate ownership even when the raw stream visually looks like it belongs to a different crate."

key-files:
  created:
    - .planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh
    - .planning/phases/34-documentation-currency-audit/34-evidence/34-06-cargo-doc-default.txt
    - .planning/phases/34-documentation-currency-audit/34-evidence/34-06-cargo-doc-allfeatures-workspace.txt
  modified:
    - .planning/phases/34-documentation-currency-audit/34-AUDIT.md
    - .planning/phases/34-documentation-currency-audit/34-EVIDENCE.md

key-decisions:
  - "Crate attribution for location-less diagnostics is derived by content (grep-recovered file path), never by stream position — validated against a real, reproduced discrepancy: three `unclosed HTML tag`/`unresolved link` diagnostics that print immediately before paladin-memory's own abort/summary boundary in the raw capture actually belong to paladin-web, confirmed by finding their exact quoted snippets uniquely in crates/paladin-web/src/dev_ui_controller.rs."
  - "Location recovery for a P-01 no-location diagnostic uses the full quoted source-line snippet (the text under rustdoc's own 'the link appears in this line:' note), not just the bracket-quoted identifier the RD-01 tracer row used — the bare identifier alone collapsed two independent TraceRecord warnings (trace.rs:6 and trace.rs:17) to the same first-match location; the full-sentence snippet disambiguates every one of the 36 grep-recovered rows to exactly one match, with zero remaining ambiguity."
  - "The workspace all-features run (Task 2) mints no RD-nn rows, per the plan's own explicit instruction — its 17 content errors are a strict subset of what plan 34-07's per-crate sweep will enumerate with full rows, and counting them here too would double-count the same diagnostics under two different run labels."
  - "D-14's CONTEXT.md prose ('aborts at the first failing crate in build order, paladin-ai-core') is recorded corrected, not merely re-measured: this plan's own live re-run found a fourth abort composition (paladin-memory, paladin-web, paladin-storage, paladin-ai — no paladin-ai-core at all) distinct from both Phase 32's single-crate figure and RESEARCH.md's own 3-crate measurement, confirming the abort set is genuinely scheduling-dependent rather than reproducible per any single-crate framing."
  - "All measurements were re-run live at this plan's own HEAD rather than copied from RESEARCH.md or 34-01's prior capture, per the phase's operating rule — the default-feature run reproduced the identical 73-warning/65-content-diagnostic count 34-01 already captured (confirmed by diff, differing only in cargo's own incremental-build progress-line order), while the all-features run reproduced yet another distinct abort composition, underscoring why the per-crate sweep (not the workspace command) is the only accurate all-features enumeration."

requirements-completed: [CURR-02, CURR-05]

coverage:
  - id: D1
    description: "The ci.yml lint-job 'Check documentation' command is quoted verbatim in 34-AUDIT.md (ci.yml:62-63 citation plus ADR-0033 ratification), and every one of the 65 content warnings from the default-feature cargo doc run is enumerated as its own RD-nn row with crate, real file:line, kind, verbatim message, location source, evidence anchor and size — never summarised as a count."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "bash .planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh .planning/phases/34-documentation-currency-audit/34-evidence/34-06-cargo-doc-default.txt default (RECONCILED: 65 content diagnostics == 65 rows emitted, exit 0); grep -c '^| RD-[0-9]' .planning/phases/34-documentation-currency-audit/34-AUDIT.md == 66"
        status: pass
    human_judgment: false
  - id: D2
    description: "Rows whose rustdoc diagnostic carries no location (36 of 65) are given a real file:line recovered by grepping the diagnostic's own quoted source-line snippet against the attributed crate's src/ tree — validated with zero ambiguous multi-match fallbacks, including two independent same-identifier TraceRecord warnings correctly disambiguated to two different lines."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "grep -c 'grep recovery' .planning/phases/34-documentation-currency-audit/34-AUDIT.md (36 rows); grep -c 'first taken' .planning/phases/34-documentation-currency-audit/34-AUDIT.md (0 — no ambiguous fallback was needed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The default-feature warning count (73 total lines, 65 content diagnostics) is reported alongside the rows with this run's measured HEAD SHA, with an explicit drift comparison against the 72 (Phase 29) and 73 (Phase 33) figures earlier phases recorded, attributing no movement to any specific commit."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "grep -A6 'Count reconciliation' .planning/phases/34-documentation-currency-audit/34-AUDIT.md; grep -A4 'Drift against the two prior HEAD SHA counts' .planning/phases/34-documentation-currency-audit/34-AUDIT.md"
        status: pass
    human_judgment: false
  - id: D4
    description: "WINDOWS.md rows 36 and 37 are cross-checked against the enumeration (row 37 matches the default-run's own HeuristicTokenCounter row exactly) and confirmed still open, with git status --porcelain -- .planning/WINDOWS.md proving neither row was edited by this phase."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "grep -A6 'WINDOWS.md cross-check' .planning/phases/34-documentation-currency-audit/34-AUDIT.md; git log -p --follow -- .planning/WINDOWS.md (no commit from this plan touches the file)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The workspace all-features run is recorded as a partial view (17 content errors across 4 crates: paladin-memory, paladin-web, paladin-storage, paladin-ai), explicitly minting no RD-nn rows, with its concurrency-driven abort behaviour stated and D-14's single-crate abort prose corrected by a third independent measurement."
    requirement: "CURR-02"
    verification:
      - kind: other
        ref: "grep -A6 'Workspace all-features run' .planning/phases/34-documentation-currency-audit/34-AUDIT.md; grep -c '^| RD-[0-9]' .planning/phases/34-documentation-currency-audit/34-AUDIT.md == 66 (unchanged by Task 2)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Phase remains read-only outside .planning/ across both task commits (SC5/D-22/CURR-05); 34-check.sh --seed stays green after each commit; WINDOWS.md untouched."
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "git status --porcelain -- . ':!.planning' (empty after each of the two task commits); bash .planning/phases/34-documentation-currency-audit/34-check.sh --seed (PASS on all 5 assertions after each commit)"
        status: pass
    human_judgment: false

duration: ~18min
completed: 2026-09-17
status: complete
---

# Phase 34 Plan 06: Default-Feature Rustdoc Enumeration Summary

**Enumerated all 65 content warnings from the live `cargo doc --workspace --no-deps` run as `RD-02..RD-66` pipe-table rows (36 of them location-less, recovered by full quoted-snippet grep, zero ambiguous fallbacks), wrote a reusable `34-rustdoc-rows.sh` parser plan 34-07 inherits unmodified, and recorded the workspace all-features run's 17-error/4-crate abort as an explicit floor — correcting D-14's single-crate abort prose with a third independent measurement.**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-09-17T05:23:00Z (approx.)
- **Completed:** 2026-09-17T05:41:29Z
- **Tasks:** 2 completed
- **Files modified:** 5 (2 modified, 3 created)

## Accomplishments
- Ran the exact `ci.yml:62-63` "Check documentation" command live (`cargo doc --workspace --no-deps`, teed to `34-evidence/34-06-cargo-doc-default.txt`): exit `0` for `cargo doc` itself, but the trailing `! grep -q "warning:"` negation exits `1` — the composite CI gate is **RED**, 73 total `warning:` lines, 65 content diagnostics after excluding the 8 per-crate summary lines
- Wrote `34-rustdoc-rows.sh`: a generic Python-backed bash parser that (1) attributes every diagnostic to its true crate by consuming cargo's batched per-crate summary lines front-to-back against the pending diagnostic queue — proven live against 4 independent crate-count splits (`paladin-ai`/`paladin-web` 5+3, `paladin-battalion`/`paladin-storage`/`paladin-llm` 36+1+4, `paladin-ports`/`paladin-ai-core` 1+14, `paladin-memory` 1) — and (2) recovers `file:line` for the 36 of 65 diagnostics carrying no rustdoc `-->` span by grepping the exact quoted source-line snippet (not the bare bracket identifier, which would have collapsed two independent `TraceRecord` warnings to one line) restricted to doc-comment lines inside the attributed crate's own `src/` tree
- Pasted all 65 rows into `34-AUDIT.md` §3 as `RD-02..RD-66` (RD-01 already existed as the plan 34-01 tracer), with the CI bar quoted byte-identical and cited to `ci.yml:62-63` and ADR-0033, a full count reconciliation (73 total / 8 summaries / 65 content = 65 rows), a drift note against 72 (Phase 29) and 73 (Phase 33, unchanged), a Kind distribution tally (24 private intra-doc link, 36 unresolved link, 3 redundant explicit link, 2 unclosed HTML tag), and a WINDOWS.md rows 36/37 cross-check confirming row 37 matches the default run's own last row exactly
- Ran `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` live (exit `101`, 34s, 17 content errors): attributed each error to its true crate (`paladin-memory` 1, `paladin-web` 8, `paladin-storage` 1, `paladin-ai` facade 7) by content-based grep rather than stream position — 3 of paladin-web's location-less errors print before `paladin-memory`'s own abort boundary in the raw capture, which stream-position attribution would have gotten wrong
- Recorded this run as an explicit **floor, not an enumeration**, minting no `RD-nn` rows (per the plan's instruction, since plan 34-07's per-crate sweep is the real enumeration); corrected D-14's CONTEXT.md prose ("aborts at the first failing crate in build order, `paladin-ai-core`") with a third independent measurement — this run's abort set does not include `paladin-ai-core` at all, a fourth distinct composition after Phase 32's 1-crate and RESEARCH.md's 3-crate findings
- Appended 19 numbered evidence rows (109-127) to `34-EVIDENCE.md` across the two tasks' sections

## Task Commits

1. **Task 1: Run the ci.yml documentation command and enumerate every default-feature warning as a row** — `b4399b3c` (docs)
2. **Task 2: Record the workspace all-features run as what CI's bar sees, with its abort behaviour stated** — `5b2bfef6` (docs)

_Both commits touch only `.planning/`; no TDD cycle applies to this documentation-only plan._

## Files Created/Modified
- `.planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh` — the crate-attributing, snippet-grep-recovering rustdoc diagnostic parser (created, executable, shellcheck-clean)
- `.planning/phases/34-documentation-currency-audit/34-evidence/34-06-cargo-doc-default.txt` — verbatim `cargo doc --workspace --no-deps` capture, 73 warning: lines, 65 content diagnostics (created)
- `.planning/phases/34-documentation-currency-audit/34-evidence/34-06-cargo-doc-allfeatures-workspace.txt` — verbatim `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` capture, 17 content errors across 4 crates (created)
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` — §3 default-feature enumeration (65 new RD rows) and the workspace all-features subsection (modified)
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` — evidence rows 109-127 appended (modified)

## Decisions Made
- Crate attribution for concurrently-scheduled diagnostics is derived by content (the diagnostic's own `-->` path, or a grep-recovered path), never by raw stream position — proven necessary by a real discrepancy in both this plan's own captures (see key-decisions above).
- Location recovery uses the full quoted source-line snippet, not the bare bracket identifier the RD-01 tracer row used — the identifier alone is insufficiently precise once the same identifier is linked from more than one doc-comment line in the same crate.
- Task 2 mints no `RD-nn` rows, honoring the plan's explicit instruction that the workspace all-features run is a partial view whose findings would double-count against plan 34-07's per-crate sweep.
- `34-rustdoc-rows.sh` builds its package-name-to-src-dir map live from every `crates/*/Cargo.toml` `[package] name` field (plus the root `Cargo.toml`), rather than hardcoding the crate list RESEARCH.md's table names (which does not match `ls crates/`, per the crate-name discrepancy 34-01-SUMMARY.md already flagged) — this makes the script correct today and resilient to any future crate rename or addition.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reworded 6 prose mentions of `RD-01`/`RD-66` that collided with `34-check.sh`'s duplicate-ID assertion**
- **Found during:** Task 1 and Task 2, each task's first `34-check.sh --seed` run after adding new prose that cited the tracer row (`RD-01`) or the newly-minted `HeuristicTokenCounter` row (`RD-66`) by literal ID string
- **Issue:** `34-check.sh` assertion (b) (`grep -oE '(MB|RD|EX)-[0-9]+' | sort | uniq -d` must be empty) flagged both IDs as "duplicate" because explanatory prose (the citation-back note, the WINDOWS.md cross-check paragraph, the row-count summary sentence, the Task 2 crate table's own evidence cell) repeated the literal ID string belonging to a row already present elsewhere in the file — the exact same false-positive class every prior plan in this phase (34-01 through 34-05) already hit and fixed for `MB-nn` cross-references.
- **Fix:** Reworded every cross-reference to describe the row positionally ("the known-answer row worked at the top of this section", "the matching row in the default-feature enumeration's own table below", "the same known-answer case worked at the top of this section") instead of repeating the literal ID string.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `bash 34-check.sh --seed` assertion (b) passes after each fix; `grep -oE '(MB|RD|EX)-[0-9]+' 34-AUDIT.md | sort | uniq -d` prints nothing.
- **Committed in:** `b4399b3c` (Task 1), `5b2bfef6` (Task 2)

**2. [Rule 1 - Bug] Corrected the all-features capture's recorded line count after the pre-commit trailing-whitespace hook normalized it**
- **Found during:** Task 2, after the first commit attempt failed on the `trim-trailing-whitespace` hook and re-staging the hook's own fix changed the file's line count from the figure the prose had already stated
- **Issue:** `34-AUDIT.md`'s Task 2 subsection initially stated the raw capture was "183 lines" (an estimate made before the hook's normalization pass); the hook's fix (stripping trailing whitespace from `cargo doc`'s own multi-line diagnostic continuation formatting) left the file at 164 lines with identical diagnostic content (confirmed: `grep -c '^error:'` and `grep -c 'could not document'` both unchanged at 21/4 before and after).
- **Fix:** Updated the line-count citation to the accurate, post-hook, post-commit figure (164 lines), with a one-clause note explaining the normalization.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `wc -l 34-evidence/34-06-cargo-doc-allfeatures-workspace.txt` matches the cited figure exactly on the committed file.
- **Committed in:** `5b2bfef6` (Task 2)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — the same recurring literal-string-collision-with-the-plan's-own-gate class every prior plan in this phase has caught, plus one factual-accuracy correction after a pre-commit hook normalized a file this plan's own prose cited a pre-normalization line count for)
**Impact on plan:** Neither fix touches anything outside `.planning/`, and neither changes any row's crate, file:line, kind, or size already assigned — both were required to make the plan's own `<verify>` blocks and this plan's own accuracy standard pass as specified.

## Issues Encountered
- The workspace all-features run's abort composition is genuinely non-deterministic run-to-run, more so than RESEARCH.md's own P-02 pitfall description suggested: a third scratch re-run performed purely to sanity-check the committed capture's line count (not itself committed as evidence) produced a *fourth* distinct abort composition (574 raw lines, `paladin-battalion` errors present, a different crate order entirely) beyond the three already recorded in this plan's own AUDIT.md prose (Phase 32's 1-crate figure, RESEARCH.md's 3-crate figure, and this plan's own committed 4-crate figure). This scratch run was not incorporated into the committed record — the plan calls for one canonical measurement per task, not a chase for a stable answer that this command structurally cannot produce under `--workspace` concurrency. The finding itself (severe run-to-run non-determinism) is already fully captured in the committed subsection's own concurrency-abort explanation and does not need a fourth data point to be true.
- `34-rustdoc-rows.sh`'s crate-attribution algorithm (summary-line/abort-boundary chunking) is not documented anywhere in `34-RESEARCH.md` — RESEARCH.md's Pattern 3/Pitfall P-01 covers only the single-crate, known-answer grep-recovery case (`HeuristicTokenCounter`), never how to attribute a location-less diagnostic to a crate in a workspace-wide, concurrently-scheduled run where stream position is unreliable. This plan measured and validated the chunking approach live before trusting it for all 36 grep-recovered §3 rows and the Task 2 crate table (34-EVIDENCE.md rows 109-127 document the validation).

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- `34-AUDIT.md` §3 now carries 66 `RD-nn` rows (the RD-01 tracer plus RD-02..RD-66 from this plan's default-feature enumeration), every one with a real `file:line`, a named Kind, and an evidence anchor — plan 34-07's per-crate `-D warnings --all-features` sweep can begin immediately, reusing `34-rustdoc-rows.sh` unmodified against each crate's own `error:`-prefixed capture (pass the crate's own capture path and a label containing "allfeatures" to switch the diagnostic prefix the parser walks).
- The workspace all-features run is recorded honestly as a floor with its own crate/error breakdown, explicitly pointing forward to plan 34-07 as the section that enumerates — no `RD-nn` numbering collision risk between this plan and 34-07, since this plan minted none from that run.
- `34-check.sh --seed` remains green after both task commits; `--final` mode's three additional assertions are still not exercised (expected — plan 34-09 is the first plan that runs `--final`, once §4/§5/§6/§7 are populated by later plans).
- No blockers. The severity of the all-features run's non-determinism (a fourth distinct abort composition observed in an uncommitted sanity check, beyond the three already on record) is a heads-up for plan 34-07: the per-crate sweep (`-p <crate>`, one invocation per crate, never `--workspace`) is not merely more complete than the workspace command — it is the only way to get a reproducible, crate-scoped answer at all, since the workspace command's own abort point is not stable across runs even at a fixed HEAD.

---
*Phase: 34-documentation-currency-audit*
*Completed: 2026-09-17*

## Self-Check: PASSED

All 3 created files found on disk (`34-rustdoc-rows.sh`, `34-evidence/34-06-cargo-doc-default.txt`, `34-evidence/34-06-cargo-doc-allfeatures-workspace.txt`); both task commits (`b4399b3c`, `5b2bfef6`) found in `git log`.
