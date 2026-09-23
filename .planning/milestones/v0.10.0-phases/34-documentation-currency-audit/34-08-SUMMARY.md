---
phase: 34-documentation-currency-audit
plan: 08
subsystem: docs
tags: [documentation-audit, examples, cargo-build, doc-examples, currency-audit, gap-analysis]

# Dependency graph
requires:
  - phase: 34-07
    provides: 34-AUDIT.md's §3 rustdoc close (143 RD-nn rows), the §1 shipped-surface checklist (91 SS-nn rows) this plan's obsolete-API target set and capability gap list are derived from
provides:
  - 34-AUDIT.md §4 fully populated — 122 total EX-nn rows (the pre-existing EX-01 MSRV row, 60 build rows for every examples/*.rs file, every crates/doc-examples/src/*.rs module and live_vendor_smoke.rs, a 59-row D-17(c) capability gap list, and 2 new examples/README.md findings)
  - Four ci.yml:548-558 build invocations plus scripts/check-doc-examples.sh (three layers) plus the paladin-llm live_vendor_smoke build — all six green, captured verbatim in 34-evidence/34-08-examples-builds.txt
  - D-17(a) obsolete-API sweep across all 60 programs/modules — zero genuine hits, 21 coincidental grep matches each individually resolved to a distinct current API
  - D-17(b) capability-mapping verdicts per row — two stale findings (examples/http_service_host.rs and its doc-examples sibling both overclaim "the same router the paladin-server binary uses")
  - D-18 doc-examples-module-to-docs/src-page include map for all 11 modules (support.rs has zero including pages — recorded, not omitted)
  - §4 close with counted totals satisfying ROADMAP Success Criterion 3 in full
affects: [34-09-work-list-assembly, 36-rustdoc-remediation, 35-docs-remediation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Obsolete-API false-positive resolution: a grep hit for a target-set token is not itself proof of obsolescence — `token_count` matched `StreamChunk::token_count()` (a live builder method) fifteen times before being confirmed as a distinct, current API by reading the actual call site against the actual current type definition, not the grep count alone. The same discipline resolved `TokenUsage::new`'s 2-arg constructor and `SanctumPort::search`'s untouched result shape."
    - "Server-parity capability-mapping check: comparing an example's own doc-comment claim ('assembles the app exactly as the X binary does') against the real binary's actual router composition (`src/bin/paladin-server.rs`'s three-router merge) surfaces staleness a page-level route-table sweep cannot — the earlier §2 sweep of the including docs/src page checked the documented route table and found it accurate, but never checked this broader relational claim."

key-files:
  created:
    - .planning/phases/34-documentation-currency-audit/34-evidence/34-08-examples-builds.txt (six verbatim build/gate captures)
  modified:
    - .planning/phases/34-documentation-currency-audit/34-AUDIT.md (§4 fully populated: build table, obsolete-API sweep, currency verdicts, gap list, include map, close)
    - .planning/phases/34-documentation-currency-audit/34-EVIDENCE.md (evidence rows 151-171 appended)
    - .planning/phases/34-documentation-currency-audit/deferred-items.md (Plan 34-08 Task 1 entry — stale ci.yml example-count comment)

key-decisions:
  - "48 live examples/*.rs files measured, not the 47 ci.yml's own step comment states — the one-file drift is routed to deferred-items.md per D-19 (a CI comment is neither documentation nor an example) rather than minted as an EX-nn row."
  - "Obsolete-API check (a) concluded zero genuine hits across all 60 programs and modules: every one builds clean under the exact CI feature sets, which is itself a second, independent proof against the four 'deleted outright, no shim' removals (TokenUsage::from_total, garrison::TokenCounter, TokenCounterFactory, Quartermaster, LimitSource) — Rust's compiler cannot resolve a call to a symbol that no longer exists."
  - "examples/http_service_host.rs and its crates/doc-examples sibling are both marked stale, not current, despite building clean and correctly demonstrating the agent API: their shared claim of assembling 'exactly'/'the same router' as the paladin-server binary is now false — the binary merges three routers (agent_router + thread_router + run_router) since Phases 24 and 27, the examples demonstrate only one."
  - "The D-17(c) gap list excludes SS-rows for removed items (absences by design, not undemonstrated capabilities), pure CI/tooling gates, and governance/vocabulary records — only requirement-ID-bearing, example-representable capabilities with zero grep hits become gap-list rows, yielding 59 (not 91)."
  - "21 literal EX-nn ID citations in cross-reference prose (e.g. 'see EX-01 above') were reworded to positional phrasing after 34-check.sh's duplicate-ID assertion caught them — the same recurring literal-string-collision class every prior plan in this phase (34-01 through 34-07) has already hit and fixed."

patterns-established:
  - "D-17(c) gap-list exclusion criteria: exclude an SS-row from the gap-list walk when it names a deliberate removal (nothing to demonstrate), a CI/release-tooling gate (not example-representable), or a governance/vocabulary record (not a code capability) — only requirement-attributed, code-level capabilities with zero grep hits become gap-list rows."

requirements-completed: [CURR-03, CURR-05]

coverage:
  - id: D1
    description: "The four ci.yml:548-558 build invocations plus scripts/check-doc-examples.sh (three layers) plus the paladin-llm live_vendor_smoke build are run byte-identical/verbatim and captured; every examples/*.rs file, every crates/doc-examples/src/*.rs module (excl. lib.rs) and live_vendor_smoke.rs get one EX-nn build row each, attributed to the specific invocation that covered them."
    requirement: "CURR-03"
    verification:
      - kind: other
        ref: "34-evidence/34-08-examples-builds.txt (six captures, all exit 0); grep -c '^| EX-[0-9]' 34-AUDIT.md == 122 >= 60 minimum; grep -q 'cargo build --examples --offline' and grep -q 'live_vendor_smoke' both match 34-AUDIT.md"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every §4 row carries a settled D-17 three-check currency verdict (obsolete-API, capability-mapping, gap-list), the doc-examples-to-docs/src include map covers all 11 modules, and §4 closes with counted totals — zero rows retain the seeded pending marker."
    requirement: "CURR-03"
    verification:
      - kind: other
        ref: "grep -ci 'not yet assessed' 34-AUDIT.md == 0; grep -oE 'EX-[0-9]+' 34-AUDIT.md | sort | uniq -d prints nothing; bash 34-check.sh --seed PASS on all 5 assertions"
        status: pass
    human_judgment: false
  - id: D3
    description: "Phase remains read-only outside .planning/ across both task commits (SC5/D-22/CURR-05); 34-check.sh --seed stays green after each commit; config.json is never staged."
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "git status --porcelain -- . ':!.planning' empty after each of the two task commits (8678b5ce, 035c8338); git status --porcelain -- examples crates Cargo.toml empty after each; bash 34-check.sh --seed PASS on all 5 assertions after each commit"
        status: pass
    human_judgment: false

duration: ~23min
completed: 2026-09-17
status: complete
---

# Phase 34 Plan 08: Examples Build & Currency Sweep Summary

**All 60 examples/doc-examples programs and modules build green under the exact CI feature sets, zero genuine obsolete-API references, two `http_service_host` rows marked stale on a server-router-parity overclaim, `examples/README.md` flagged for 11 undocumented programs and three stale `PaladinResult` code-snippet fields, and a 59-row capability gap list for Phase 22-33 items no example demonstrates.**

## Performance

- **Duration:** ~23 min
- **Started:** 2026-09-17T05:59:33Z (approx., end of plan 34-07)
- **Completed:** 2026-09-17T06:22:01Z
- **Tasks:** 2 completed
- **Files modified:** 4 (3 modified, 1 created)

## Accomplishments
- Re-counted the live examples surface rather than trusting `ci.yml`'s own comment: 48 `examples/*.rs` files (not the comment's stated 47 — routed to `deferred-items.md`, not minted as an `EX-nn` row), 11 `crates/doc-examples/src/*.rs` modules, 1 `crates/paladin-llm/examples/live_vendor_smoke.rs` — 60 total
- Ran all four `ci.yml:548-558` invocations byte-identical, plus `scripts/check-doc-examples.sh` (all three layers) and the `paladin-llm` `live_vendor_smoke` build (built only, never run — it reaches a live vendor and needs a credential) — all six green, captured verbatim in `34-evidence/34-08-examples-builds.txt`
- Cross-checked `[[example]]` declarations in both directions (5 declared targets, all with matching files; 43 undeclared files all build clean under the bulk selector with no `required-features` need) — zero gaps in either direction
- Wrote 60 new `EX-nn` build rows in `34-AUDIT.md` §4 (`EX-02..EX-61`), each attributed to its covering invocation, seeded with the plan's exact pending marker in the Currency/Obsolete-API/Capability cells (Task 1)
- Ran the D-17(a) obsolete-API sweep across all 60 programs/modules against a target set derived from §1's removed/renamed rows (`TokenUsage::from_total`, bare `token_count`, `Quartermaster`, `garrison::TokenCounter`, `TokenCounterFactory`, `LimitSource`, the pre-Phase-33 `memory.content` rendering pattern) — 21 coincidental grep matches, zero genuine hits, each match individually resolved by reading the real call site against the real current type
- Settled a D-17(b) capability-mapping verdict for every row; found and recorded `examples/http_service_host.rs` and its `crates/doc-examples` sibling both stale — both claim to assemble the app "exactly as"/"the same router" the `paladin-server` binary uses, but the real binary now merges three routers (`agent_router` + `thread_router` + `run_router`, Phases 24 and 27) while the examples demonstrate only one
- Audited `examples/README.md` as a page in its own right: cited the existing `EX-01` MSRV row without re-minting it, and found two new `stale` findings — 11 on-disk programs (`commander_council.rs`, `commander_grove.rs`, `conclave_expert_panel.rs`, `council_discussion.rs`, `document_processing.rs`, `grove_routing.rs`, `http_service_host.rs`, `paladin_with_rag.rs`, `vision_analysis.rs`, `vision_battalion.rs`, `war_engine_memory_baseline.rs`) entirely absent from the gallery, and three illustrative "Code snippet" blocks using `PaladinResult` field names (`response.content`, `response.token_usage.total_tokens`, `response.execution_time`) that don't exist on the shipped type (real fields: `output`, `usage: TokenUsage`, `execution_time_ms`)
- Walked all 91 §1 `SS-nn` capability rows against the examples/doc-examples tree, excluded removed items, CI/tooling gates and governance/vocabulary records, and produced a 59-row D-17(c) capability gap list (`EX-62..EX-120`) spanning 8 of the 12 phases the §1 checklist covers, each sized `L`
- Built the D-18 `doc-examples`-module-to-`docs/src`-page include map for all 11 modules from a direct `{{#include}}` grep (never a module's own doc-comment claim) — found `support.rs` has zero including pages (a shared mock-adapter dependency of the other ten, not itself an include target) and recorded that as a finding rather than omitting it
- Closed §4 with a counted-totals subsection: 122 total `EX-nn` rows (58 current / 5 stale in the main table, 59 gap-list rows), satisfying ROADMAP Success Criterion 3 in full
- Reworded 21 literal `EX-nn` ID citations in cross-reference prose to positional phrasing after `34-check.sh`'s duplicate-ID assertion caught the collision — the same recurring false-positive class every prior plan in this phase has already hit and fixed
- Appended 21 numbered evidence rows (151-171) to `34-EVIDENCE.md` across the two tasks' sections

## Task Commits

1. **Task 1: Build every example under the four CI feature sets and record per-program build status** — `8678b5ce` (docs)
2. **Task 2: Currency verdicts, the doc-examples include map, and the capability gap list** — `035c8338` (docs)

_Both commits touch only `.planning/`; no TDD cycle applies to this documentation-only plan._

## Files Created/Modified
- `.planning/phases/34-documentation-currency-audit/34-evidence/34-08-examples-builds.txt` — six verbatim build/gate captures: the four `ci.yml` invocations, `scripts/check-doc-examples.sh`'s three layers, and the `live_vendor_smoke` build (created)
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` — §4 fully populated: 122 `EX-nn` rows across the main build table, the D-17(c) gap-list table, and two `examples/README.md` findings, plus the D-18 include map and the §4 close subsection (modified)
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` — evidence rows 151-171 appended (modified)
- `.planning/phases/34-documentation-currency-audit/deferred-items.md` — `## Plan 34-08, Task 1` entry appended (stale `ci.yml` example-count comment) (modified)

## Decisions Made
- The 47→48 `ci.yml` comment drift is routed to `deferred-items.md`, not minted as an `EX-nn` row (D-19: a CI comment is neither documentation nor an example).
- Zero genuine obsolete-API hits recorded — every coincidental grep match was individually verified against the real, current call site rather than trusted at face value; a green build across all 60 programs/modules is itself independent proof against the four "deleted outright" removals.
- `examples/http_service_host.rs` and its `crates/doc-examples` sibling are marked `stale` (not `current`) despite building clean, because their own doc-comment claim of server parity is now false by two routers' worth of shipped surface (Phases 24, 27) — what they do demonstrate is correct; what they claim to demonstrate is not.
- The D-17(c) gap list applies an explicit exclusion filter (removed items, CI/tooling gates, governance/vocabulary records) rather than walking all 91 §1 rows blindly, yielding 59 genuinely undemonstrated, example-representable capabilities.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reworded 21 literal `EX-nn` ID citations that collided with `34-check.sh`'s duplicate-ID assertion**
- **Found during:** Task 2, first `34-check.sh --seed` run after writing the gap list, README audit findings, include map and §4 close subsection
- **Issue:** Several cross-reference sentences repeated an already-minted `EX-nn` literal ID in prose (e.g. "see `EX-01` above", "`EX-33`, `EX-55`", `(EX-nn)` parenthetical annotations in the include-map table, and range citations like `EX-02..EX-61` in the close subsection's bullet points) — `34-check.sh` assertion (b) flagged these as duplicate IDs, the same recurring false-positive class every prior plan in this phase (34-01 through 34-07) has already hit and fixed.
- **Fix:** Reworded every such citation to positional/descriptive phrasing ("the first worked row of the main table above", "the two `http_service_host` rows below", "the separate gap-list table above") instead of repeating literal ID strings. Also found and fixed 9 instances where the placeholder text "(own doc comment — not in examples/README.md, EX-62)" I had written for 9 undocumented-program capability cells accidentally collided with the real `EX-62` gap-list row (a numbering coincidence, not an intentional cross-reference) — reworded to "see the README gallery-gap finding below."
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `bash 34-check.sh --seed` assertion (b) passes after the fix; `grep -oE 'EX-[0-9]+' 34-AUDIT.md | sort | uniq -d` prints nothing.
- **Committed in:** `035c8338` (Task 2)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug — the same recurring literal-ID-collision class every prior plan in this phase has caught)
**Impact on plan:** The fix touches only prose wording in already-committed rows; no row's ID, verdict, or evidence content changed. Required to make `34-check.sh --seed` pass as the plan's own verify block specifies.

## Issues Encountered
None beyond the one auto-fixed deviation above — an anticipated failure mode this phase's own tooling (`34-check.sh`) exists to catch.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- `34-AUDIT.md` §4 is now fully closed: build status for every program/module (plan 34-08 Task 1), three-check currency verdicts, the D-18 include map, the D-17(c) gap list, and a counted-totals close subsection — 122 total `EX-nn` rows, every one ending in a real (non-placeholder) currency verdict or gap-list evidence cell, no duplicate ID.
- Plan 34-09 (work-list assembly) can now draw §6's Phase 36 work list from every `RD-nn` row (plan 34-06/34-07, 143 total) and every `EX-nn` row this plan wrote (122 total, of which 5 `stale`/`current`-flagged build rows plus 59 gap-list rows plus 2 `README` findings need Phase 36 attention), and route the `deferred-items.md` pointers (now including this plan's `## Plan 34-08, Task 1` entry) into §7.
- `34-check.sh --seed` remains green after both task commits; `--final` mode's three additional assertions (e, f, g) are still not exercised — expected, per 34-07-SUMMARY's own note that plan 34-09 is the first to run `--final`, once §5/§6/§7 are populated.
- No blockers. Plan 34-09 (work-list assembly and phase close) is next.

---
*Phase: 34-documentation-currency-audit*
*Completed: 2026-09-17*

## Self-Check: PASSED

All created/modified files found on disk (`34-evidence/34-08-examples-builds.txt`, `34-AUDIT.md`,
`34-EVIDENCE.md`, `deferred-items.md`, `34-08-SUMMARY.md` itself); both task commits (`8678b5ce`,
`035c8338`) found in `git log`.
