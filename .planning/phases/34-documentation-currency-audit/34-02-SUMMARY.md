---
phase: 34-documentation-currency-audit
plan: 02
subsystem: docs
tags: [documentation-audit, shipped-surface-checklist, requirements-traceability, vocabulary]

# Dependency graph
requires:
  - phase: 34-01
    provides: the 34-AUDIT.md canonical inventory skeleton (header, 7 sections, 92 seeded mdBook rows, 34-signals.sh, 34-check.sh)
provides:
  - 34-AUDIT.md §1 — the D-08 shipped-surface checklist (91 SS-nn rows across 13 phase tables, Phases 22-33)
  - 34-shipped-tokens.txt — the 91-line grep token file that closes 34-signals.sh's class 9 SKIPPED degrade path
  - D-10 ubiquitous-language confirmation (three named lists line-anchored, one additional partial list recorded)
affects: [34-03-mdbook-verdicts, 34-04-mdbook-verdicts, 34-05-mdbook-verdicts, 34-06-rustdoc-default, 34-07-rustdoc-all-features, 34-08-examples, 34-09-work-list-assembly]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SS-nn checklist ID scheme (D-08): one stable ID per shipped item, phase-grouped, cited by every later §2/§3/§4 verdict rather than re-derived"
    - "Facade-only exports-diff scope note: .project/current-exports.txt tracks only pub paladin::... paths (cargo-public-api on the facade crate), so a 0-hit identifier token means facade-scope, not absence, unless independently confirmed missing"

key-files:
  created:
    - .planning/phases/34-documentation-currency-audit/34-shipped-tokens.txt
  modified:
    - .planning/phases/34-documentation-currency-audit/34-AUDIT.md
    - .planning/phases/34-documentation-currency-audit/34-EVIDENCE.md

key-decisions:
  - "Checklist rows sourced from CHANGELOG.md [0.10.0] end-to-end plus MIGRATION.md §9.2/§9.6 supplementation, per D-08 precedence, with the exports diff used only as a fifteen-plus-token cross-check, never read prose-style"
  - "The three D-10-named ubiquitous-language lists confirmed by line-anchored grep; a genuine fourth partial list found at docs/src/introduction.md (12 terms, no Commissary) — recorded for a later mdBook-sweep plan to judge, not judged here"
  - "Exports-diff cross-check's six 0-hit tokens (Aegis, FallbackLlmAdapter, StreamingResponse, ChunkMetadata, LlmError, NodeError) attributed to current-exports.txt's own facade-only scope (measured: 1095 of 7924 lines are pub paladin::... paths), not to a tree/document disagreement"

requirements-completed: []

coverage:
  - id: D1
    description: "34-AUDIT.md §1 holds the D-08 shipped-surface checklist as 91 phase-grouped SS-nn rows (13 tables, Phases 22-33), each with a Kind from the eleven-value enum, a Source citing CHANGELOG/MIGRATION/REQUIREMENTS, a requirement ID or '-', and a single-literal grep token; 34-shipped-tokens.txt holds the matching 91-line token file"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "D=.planning/phases/34-documentation-currency-audit; test \"$(grep -c '^| SS-[0-9]' $D/34-AUDIT.md)\" -ge 40; test \"$(grep -vc '^#' $D/34-shipped-tokens.txt)\" = \"$(grep -c '^| SS-[0-9]' $D/34-AUDIT.md)\"; grep -c '^#### Phase ' $D/34-AUDIT.md == 13; grep -oE 'SS-[0-9]+' $D/34-AUDIT.md | sort | uniq -d (empty)"
        status: pass
    human_judgment: false
  - id: D2
    description: "34-shipped-tokens.txt closes 34-signals.sh's class 9 SKIPPED degrade path — every mdBook page can now be grepped against the shipped surface"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "bash .planning/phases/34-documentation-currency-audit/34-signals.sh docs/src/architecture/commissary.md (class 9 prints real grep -nFf hits, no SKIPPED marker)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The three D-10 ubiquitous-language lists (.github/copilot-instructions.md, .planning/PROJECT.md, docs/src/architecture/domain-model.md) confirmed by line-anchored Commissary grep; a fourth partial list found and recorded"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "grep -n 'Commissary' .github/copilot-instructions.md (line 36); grep -n 'Commissary' .planning/PROJECT.md (line 1324); grep -n 'Commissary' docs/src/architecture/domain-model.md (line 30)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Phase remains read-only outside .planning/ across this plan's commit (SC5/D-22/CURR-05); 34-check.sh --seed stays green"
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "git status --porcelain -- . ':!.planning' (empty); bash .planning/phases/34-documentation-currency-audit/34-check.sh --seed (all 5 seed-mode assertions PASS)"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-09-17
status: complete
---

# Phase 34 Plan 02: Shipped-Surface Checklist Summary

**Compiled the D-08 shipped-surface checklist into `34-AUDIT.md` §1 — 91 phase-grouped `SS-nn` rows across all 13 Phase 22-33 tables, each carrying a Kind/Source/Req-ID/grep-token, plus `34-shipped-tokens.txt` closing `34-signals.sh`'s class-9 `SKIPPED` degrade path.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-09-17 (immediately following 34-01)
- **Completed:** 2026-09-17T03:56:50Z (commit `ee81ae7c`)
- **Tasks:** 1 completed
- **Files modified:** 3 (2 modified, 1 created)

## Accomplishments
- Read `CHANGELOG.md`'s `[0.10.0]` section end-to-end (427 lines, 6 subsections) and `MIGRATION.md` §9.1-§9.8 in full, deriving 91 `SS-nn` checklist rows grouped into 13 phase tables (Phase 22, 22.1, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33), each attributed to a Kind from the eleven-value enum (`type`, `route`, `config key`, `env var`, `CLI subcommand`, `feature flag`, `vocabulary term`, `behavioral change`, `migration`, `dependency`), a Source (CHANGELOG/MIGRATION §9.n/REQUIREMENTS), a requirement ID (or `—` where none applies), and a single-literal grep token
- Confirmed every required proof token is present as its own row: `WarEngine`, `Commissary`, `resolve_context_window`, `WindowSource`, `RagRetrievalResult`, `TokenUsage`
- Wrote `34-shipped-tokens.txt` — 91 grep tokens in `SS-nn` order, one per line, a leading `#`-prefixed comment naming the ID range and the measured HEAD SHA; verified `34-signals.sh`'s class 9 against `docs/src/architecture/commissary.md` now returns real hits with no `SKIPPED` marker
- Ran the D-08 exports-diff cross-check over 22 identifier tokens (exceeding the 15-token minimum): 16 confirmed present in `git diff v0.9.0..HEAD -- .project/current-exports.txt` (4,376 added lines, matching CONTEXT.md's own figure exactly); the 6 not-found tokens (`Aegis`, `FallbackLlmAdapter`, `StreamingResponse`, `ChunkMetadata`, `LlmError`, `NodeError`) were traced to `.project/current-exports.txt`'s own documented facade-only scope (`cargo-public-api` on the `paladin` crate, `pub paladin::...` paths only — 1095 of 7924 lines), not to a tree/document disagreement
- Confirmed the three D-10-named ubiquitous-language lists (`.github/copilot-instructions.md` line 36, `.planning/PROJECT.md` line 1324, `docs/src/architecture/domain-model.md` line 30) by direct `Commissary` grep, and additionally found and recorded a genuine fourth, partial list at `docs/src/introduction.md` lines 78-91 (12 terms, missing `Commissary`/`Sanctum`/`Sentinel`/`Quest`/`Conclave`/`Council`/`Grove`/`Commander`) — left for a later mdBook-sweep plan to judge for currency, not judged here
- Appended evidence rows 14-25 to `34-EVIDENCE.md` covering every command this plan ran

## Task Commits

1. **Task 1: Compile the shipped-surface checklist into 34-AUDIT.md §1** — `ee81ae7c` (docs)

_Single-task plan; no TDD cycle applies to this documentation-only work._

## Files Created/Modified
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` — §1 filled with 91 `SS-nn` rows, D-10 confirmation subsection, exports-diff cross-check note (modified)
- `.planning/phases/34-documentation-currency-audit/34-shipped-tokens.txt` — 91-line grep token file for `34-signals.sh` class 9 (created)
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` — evidence rows 14-25 appended (modified)

## Decisions Made
- Checklist rows were derived by reading CHANGELOG end-to-end first (the primary readable source per D-08), then MIGRATION §9.2 (public types) and §9.6 (routes) for items CHANGELOG did not itself carry, then cross-checked against the exports diff — matching the plan's own instructed order exactly.
- Kind cell values were held strictly to the eleven-value enum; no row was forced into `deprecation` since `MIGRATION.md` §9.7 confirms zero items were deprecated across the whole v0.10.0 program — that absence is recorded in the Method statement already, not re-asserted as a checklist row.
- The fourth ubiquitous-language list found at `docs/src/introduction.md` was recorded as a fact (line-anchored, term-enumerated) but not adjudicated for staleness — CONTEXT.md's own scope note for this task is compiling the checklist, not settling §2 verdicts, and the phase's Method statement reserves verdicts for the mdBook-sweep plans.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed a broken markdown-table cell that used escaped pipes**
- **Found during:** Task 1, first `awk -F'|' '{print $4}'` Kind-column self-check after writing all 91 rows
- **Issue:** Row `SS-52`'s Shipped-item cell was originally written as `` `APP_RUN_STORE_BACKEND` (disabled\|sqlite\|postgres) `` — the backslash-escaped pipes are valid inline-code-adjacent prose but `awk -F'|'` (and any naive pipe-delimited table parser, including a human skimming column alignment) still splits on the literal `|` character regardless of the preceding backslash, shifting every subsequent column of that one row and making its apparent Kind value read as `sqlite\` instead of `env var`.
- **Fix:** Reworded the cell to `(disabled, sqlite or postgres)` — no pipe characters, same information, zero column-count risk.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `grep '^| SS-' 34-AUDIT.md | awk -F'|' '{print NF, $0}' | awk '$1!=8'` prints nothing (every row has exactly 8 pipe-delimited fields); `awk -F'|' '{print $4}' | sort -u` now yields only the eleven-value enum's members.
- **Committed in:** `ee81ae7c` (Task 1 commit)

**2. [Rule 1 - Bug] Replaced the plan's own literal token-file acceptance check with the check it actually intends**
- **Found during:** Task 1, verifying the acceptance criterion `for t in WarEngine Commissary resolve_context_window WindowSource RagRetrievalResult TokenUsage; do grep -q "| $t |" 34-shipped-tokens.txt || echo MISSING $t; done`
- **Issue:** This literal command greps `34-shipped-tokens.txt` for a substring `| WarEngine |` (pipe-token-pipe) — but the same task's own action text requires the token file to hold "one grep token per line ... with no table markup". A no-markup file can never contain a `|`-delimited substring, so the literal acceptance command as written would report every required token `MISSING` even when the file is correctly built, directly contradicting the task's own format instruction one paragraph earlier.
- **Fix:** Verified the intended check instead — `grep -qxF "$t" 34-shipped-tokens.txt` (exact-line match) for each of the six required tokens — which correctly confirms all six are present as their own lines in the no-markup file.
- **Files modified:** None (verification-only; no content fix needed once the correct check was used — all six tokens were already present as required by the file's own build instructions).
- **Verification:** `for t in WarEngine Commissary resolve_context_window WindowSource RagRetrievalResult TokenUsage; do grep -qxF "$t" 34-shipped-tokens.txt || echo MISSING $t; done` prints nothing.
- **Committed in:** `ee81ae7c` (Task 1 commit; evidence row 23 in `34-EVIDENCE.md`)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs in the plan's own literal cell text / acceptance-criteria command, discovered while proving the task's own `<verify>` block)
**Impact on plan:** Both fixes were required to make the plan's own tracer verification pass as specified; neither touches anything outside `.planning/`, and neither changes the `SS-nn` row schema or numbering Phases 35/36 will cite into.

## Issues Encountered
- `.project/current-exports.txt` is scoped to the `paladin` facade crate only (`cargo-public-api` on `pub paladin::...` paths, confirmed by its own header comment and by `grep -c '^pub paladin::'` = 1095 of 7924 lines). Six of the twenty-two sampled identifier tokens (`Aegis`, `FallbackLlmAdapter`, `StreamingResponse`, `ChunkMetadata`, `LlmError`, `NodeError`) returned 0 diff hits purely because their defining type is not re-exported through the facade prelude, not because the item is unshipped or the token is wrong. Recorded as a documented evidence-source limitation in `34-AUDIT.md`'s exports-diff cross-check note — a heads-up for any later plan that wants to use this same exports-diff file as an authoritative "is this exported" source for a `paladin-ports`/`paladin-llm`/`paladin-core` type not surfaced at the facade root.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- `34-AUDIT.md` §1 is complete: 91 `SS-nn` rows, phase-grouped, requirement-attributed, grep-token-carrying. Plans 34-03/34-04/34-05 (mdBook sweep) can now cite a checklist row directly instead of re-deriving shipped-surface facts, and `34-signals.sh`'s class 9 returns real hits for every page.
- The `docs/src/introduction.md` fourth-list finding (a stale/incomplete 12-term naming table missing `Commissary` and seven other terms) is recorded but unjudged — whichever of 34-03/34-04/34-05 sweeps `introduction.md` should treat this as a live candidate finding, not re-discover it from scratch.
- No blockers. `34-check.sh --seed` remains green; `--final` mode's three additional assertions are still not yet exercised (expected — no plan has closed the placeholder `pending` rows yet).

---
*Phase: 34-documentation-currency-audit*
*Completed: 2026-09-17*

## Self-Check: PASSED

All 3 files found on disk (`34-AUDIT.md`, `34-shipped-tokens.txt`, `34-EVIDENCE.md` modified/created); commit `ee81ae7c` found in `git log`.
