---
phase: 34-documentation-currency-audit
plan: 04
subsystem: docs
tags: [documentation-audit, superstep-engine, herald, rag, coverage-gate, ci-workflows]

# Dependency graph
requires:
  - phase: 34-03
    provides: the mdBook build/orphan/vocabulary/object-store baseline, MB numbering at MB-17, the D-07 per-page evidence method
provides:
  - 34-AUDIT.md §2 — 40 further settled page verdicts (20 user-guides, 20 deployment/operations/contributing), 19 new MB-nn rows (MB-18..MB-36)
  - 34-AUDIT.md §2 row 94 — the superstep-engine missing-page decision, MB-30, with its own evidence subsection
  - 34-AUDIT.md "Coverage command comparison" subsection — the testing-guide.md/Makefile/scripts/coverage.sh 3-way comparison closing the folded coverage todo's documentation slice
  - .planning/phases/34-documentation-currency-audit/deferred-items.md — opened, one entry (the Docker-machine coverage walk)
affects: [34-05-appendix-verdicts, 34-09-work-list-assembly]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Live-code cross-check as the primary D-07 evidence source for user-guides/deployment/operations/contributing pages — struct-literal field-count reads, trait method-count reads, constructor-arity reads, and script/workflow-body reads (scripts/coverage.sh, ci.yml, release.yml) were the evidence that found every genuine defect this plan recorded, not the version-pin grep alone"

key-files:
  created:
    - .planning/phases/34-documentation-currency-audit/deferred-items.md
  modified:
    - .planning/phases/34-documentation-currency-audit/34-AUDIT.md
    - .planning/phases/34-documentation-currency-audit/34-EVIDENCE.md

key-decisions:
  - "The Phase 22 superstep engine (WarEngine/Battlefield/Waypoint/Vanguard/max_supersteps) is settled `missing`, not `stale` on an existing page: control-flow.md's own line 29-30 states verbatim that 'the full engine guide is future documentation,' and 23-CONTEXT.md confirms the deferral was never picked up by a later phase or docs pass. Recorded as a new §2 row (94) rather than folding the finding into control-flow.md's own row, with a proposed nav position immediately before control-flow.md."
  - "Six pages in Task 2 were found stale not by version-pin grep but by reading the live script/workflow body the page claims to describe: cicd.md's CI/Release Pipeline YAML samples name jobs (build-release, a 3-job ci.yml) that do not exist in the real ~26-job ci.yml or 9-job release.yml; testing-guide.md's coverage command is missing the ,llm-all feature flag scripts/coverage.sh actually runs (the difference between measuring 3 of 9 LLM adapters and all 9); monitoring.md/troubleshooting.md both still assert opentelemetry is not a workspace dependency, which Phase 28's otel feature made false."
  - "Two genuine within-milestone code-vs-doc regressions were found via direct rustdoc/comment reads rather than grep: control-flow.md still describes NextStep::Parley as unimplemented though Phase 24 HITL-01 shipped it (the live EngineError::ParleyNotSupported variant's own doc comment says 'Superseded... no longer reachable'), and fault-tolerance.md's fingerprint-version claim (v5) is one Phase 26 D-29 bump behind the live v6."
  - "sanctum-vector-memory.md's entire RAG section predates Phase 33 by content, not by a missed grep: it never names RagRetrievalResult/ShedItem/RagRetrievalError/the truncation marker, and even the service's own name is stale (RAGRetrievalService vs the live RagRetrievalService, camelCase Rag)."
  - "The Docker-machine coverage-walk remainder was routed to a newly-opened deferred-items.md pointer rather than an MB-nn or an invented gsd-tools WINDOWS.md row, per the plan's explicit instruction that this devcontainer's lack of Docker is out of scope for this phase."

requirements-completed: []

coverage:
  - id: D1
    description: "All 20 docs/src/user-guides/ pages settled with command-backed verdicts (8 current, 12 stale via MB-18..MB-29), each findings cell naming at least 3 signal classes"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "bash .planning/phases/34-documentation-currency-audit/34-check.sh --seed (all 5 seed-mode assertions PASS)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The Phase 22 superstep engine is decided by content as a missing page (row 94, MB-30), with the full per-token grep evidence and the in-tree control-flow.md/23-CONTEXT.md deferral admission quoted verbatim, plus a proposed nav position"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "grep -A5 'Superstep-engine dedicated-page decision' .planning/phases/34-documentation-currency-audit/34-AUDIT.md"
        status: pass
    human_judgment: false
  - id: D3
    description: "All 20 deployment/deployment-topologies/operations/contributing pages settled with command-backed verdicts (14 current, 6 stale via MB-31..MB-36)"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "bash .planning/phases/34-documentation-currency-audit/34-check.sh --seed (all 5 seed-mode assertions PASS)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The folded coverage todo is closed to its documentation slice: testing-guide.md's stated coverage command, the Makefile coverage target body, and scripts/coverage.sh (what ci.yml's coverage job actually runs) are compared side by side, finding the page's command is missing ,llm-all; the Docker-machine reproduction walk is explicitly routed to deferred-items.md, not absorbed into an MB-nn"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "grep -A3 'Coverage command comparison' .planning/phases/34-documentation-currency-audit/34-AUDIT.md; test -f .planning/phases/34-documentation-currency-audit/deferred-items.md"
        status: pass
    human_judgment: false
  - id: D5
    description: "Phase remains read-only outside .planning/ across both task commits (SC5/D-22/CURR-05); 34-check.sh --seed stays green after each commit"
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "git status --porcelain -- . ':!.planning' (empty after each of the two task commits)"
        status: pass
    human_judgment: false
  - id: D6
    description: "No duplicate MB-/RD-/EX- ID exists after 19 new MB-nn IDs (MB-18..MB-36) were minted across the two tasks"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "grep -oE 'MB-[0-9]+' 34-AUDIT.md | sort | uniq -d (empty)"
        status: pass
    human_judgment: false
  - id: D7
    description: "garrison-memory.md, memory-management.md and sanctum-vector-memory.md each state explicitly whether the page names a type Phase 32 deleted, naming the type if so — all three confirmed clean (none name the deleted garrison::TokenCounter trait or TokenCounterFactory struct)"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "grep -c 'Phase 32 deleted-type check (explicit)' 34-AUDIT.md (3 occurrences, one per page)"
        status: pass
    human_judgment: false
---

# Phase 34 Plan 04: mdBook User-Guides and Deployment/Operations/Contributing Verdicts Summary

**Settled 40 further mdBook page verdicts (26 current, 14 stale, 19 new MB-18..MB-36 rows) plus the Phase 22 superstep-engine missing-page decision, finding the engine's dedicated guide was deferred in Phase 23 and never picked up — the audit's largest crop of genuine, live-code-verified defects yet, including a Phase 24 capability (Parley/Gate) still described as unimplemented, a stale graph-fingerprint version, a fabricated CI/CD workflow sample, and a coverage command one feature flag short of what CI actually runs.**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-09-17T04:22:44Z
- **Completed:** 2026-09-17T04:50:19Z
- **Tasks:** 2 completed
- **Files modified:** 3 (2 modified, 1 created)

## Accomplishments
- Settled all 20 `docs/src/user-guides/` page verdicts: **current** — `agent-runtime.md`, `eval-harness.md`, `garrison-memory.md`, `graph-visualization.md`, `memory-management.md`, `output-formatting.md`, `paladin-configuration.md`, `parley-and-chronicle.md`; **stale** — `agent-orchestrator-bridge.md` (MB-18, version pin), `arsenal-tools.md` (MB-19, `ArmamentResult` struct literal missing two fields + wrong field name + `with_specialist`/`with_handoffs` shape mismatch), `battalion-patterns.md` (MB-20, version pin + wrong `Commander::new`/`execute` signature), `content-processing.md` (MB-21, version pin), `control-flow.md` (MB-22, stale Parley description — Phase 24 shipped what the page still calls unimplemented), `fault-tolerance.md` (MB-23, fingerprint version `v5` vs live `v6`), `herald-output.md` (MB-24, Herald trait shown with 3 of 7 live methods), `maneuver-flow-dsl.md` (MB-25, version pins), `orchestration.md` (MB-26, version pin), `paladin-agents.md` (MB-27, version pin + `InMemoryGarrison::new()` arity + `with_specialist`/`with_handoffs`), `sanctum-vector-memory.md` (MB-28, Phase 33's entire RAG surface absent), `tool-integration.md` (MB-29, reachability note omits Phase 26's opt-in tool-call protocol)
- Settled the Phase 22 superstep-engine question by content: no page documents `WarEngine`/`Battlefield`/`Waypoint`/`Vanguard`/supersteps as its primary subject — every one of 6 hit pages mentions it only in passing — recorded as a new `missing`-verdict row (94, MB-30) with full per-token grep evidence, the verbatim in-tree deferral admission from `control-flow.md`/`23-CONTEXT.md`, and a proposed nav position immediately before `control-flow.md`
- Settled all 20 `docs/src/{deployment,deployment-topologies,operations,contributing}/` page verdicts: **current** — all 6 `deployment-topologies/` pages, `docker.md`, `kubernetes.md`, `production.md`, `logging.md`, `observability.md`, `branching-model.md`, `contributing-providers.md`, `development-setup.md`; **stale** — `cicd.md` (MB-31, fabricated CI/Release Pipeline job samples + missing `codeql.yml` from the workflow listing), `monitoring.md` (MB-32, Distributed Tracing section fabricated/pre-dates Phase 28's real OTel sink, stale "opentelemetry not a dependency" claim), `performance-tuning.md` (MB-33, `benches/` file-count correction one file stale since `engine_benchmarks.rs`), `troubleshooting.md` (MB-34, same stale opentelemetry-dependency claim), `architecture-decisions.md` (MB-35, nav titles it "Architecture Decisions" but the page is an Adapter Development Guide with zero ADR mentions), `testing-guide.md` (MB-36, coverage command missing `,llm-all` + fabricated non-existent `test.yml` CI sample)
- Recorded the required coverage-command 3-way comparison (`testing-guide.md`'s stated command / `Makefile`'s `coverage` target / `scripts/coverage.sh`, what `ci.yml`'s `coverage` job actually runs) as its own `34-AUDIT.md` subsection, finding the floor (82%) agrees everywhere but the page's shown feature list (`integration-tests` alone) is one flag short of the real, shared script (`integration-tests,llm-all`) — the difference between measuring 3 of 9 shipped LLM adapters and all 9 (84.32% vs 85.01%, per the fixing commit's own comment)
- Opened `deferred-items.md` and routed the Docker-machine coverage-reproduction walk to it as a pointer, per the plan's explicit instruction that this remainder stays the maintainer's own item and is not absorbed into an `MB-nn`
- garrison-memory.md, memory-management.md and sanctum-vector-memory.md each carry an explicit "Phase 32 deleted-type check" statement (none name the deleted `garrison::TokenCounter` trait or `TokenCounterFactory` struct)
- Appended 31 numbered evidence rows (48-78) to `34-EVIDENCE.md` across the two tasks' sections

## Task Commits

1. **Task 1: Settle verdicts for the 20 user-guides pages** — `3319dbdf` (docs)
2. **Task 2: Settle verdicts for the 20 deployment, deployment-topologies, operations and contributing pages** — `0bed7131` (docs)

_Both commits touch only `.planning/`; no TDD cycle applies to this documentation-only plan._

## Files Created/Modified
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` — 40 settled page verdicts, one new missing-page row (94), the superstep-engine decision subsection, the coverage-comparison subsection (modified)
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` — evidence rows 48-78 appended (modified)
- `.planning/phases/34-documentation-currency-audit/deferred-items.md` — opened with the Docker-machine coverage-walk pointer (created)

## Decisions Made
- The superstep-engine missing-page finding was recorded as a brand-new §2 row (94) rather than folded into `control-flow.md`'s own row, because D-06 requires a `missing` verdict to name the nav position a real page should take — a fact that belongs to the engine's own (non-existent) page identity, not to any one page that merely references the engine in passing.
- Both `arsenal-tools.md` and `paladin-agents.md` carry the identical `with_specialist`/`with_handoffs` API-shape mismatch (traced to a pre-v0.10.0 commit, 2026-05-30) — recorded as two separate `MB-nn` findings (one per page) rather than a single cross-page item, since D-01/D-03 scope work-list items per page/location, but each row cross-references the other's evidence rather than re-deriving it.
- `docker.md`'s and `kubernetes.md`'s repeated illustrative `v0.8.0` image tags (6 and 2 occurrences respectively) were judged NOT stale, distinguishing them from the explicit "the current published version is X" claims 34-03 found stale elsewhere (`installation.md`, `stable-api.md`) — an example command's tag is not itself a factual claim, and D-06 requires a page to misdescribe a shipped item, not merely use a dated placeholder in a command.
- The Phase 32 deleted-type disposition for the three memory pages was written as an explicit, separately-labelled sentence in each row ("Phase 32 deleted-type check (explicit)...") rather than folded into the general findings prose, to make the acceptance criterion's requirement mechanically greppable rather than merely satisfied in spirit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed 8 duplicate-ID false positives from cross-referencing sibling rows' MB-nn IDs in prose**
- **Found during:** Task 1 (6 occurrences) and Task 2 (2 occurrences), first `34-check.sh --seed` re-runs after minting each task's MB-nn rows
- **Issue:** `34-check.sh` assertion (b) (`grep -oE '(MB|RD|EX)-[0-9]+' | sort | uniq -d` must be empty) flagged MB-03, MB-07, MB-19 (×2), MB-20, MB-23, MB-24 (Task 1) and MB-36 (×1, Task 2) as "duplicate" because a row's own findings-cell prose repeated a literal ID string belonging to a *different* row, when cross-referencing that other row's finding — the same false-positive class every prior plan in this phase (34-01, 34-02, 34-03) already hit and fixed the same way.
- **Fix:** Reworded each cross-reference to describe the sibling row positionally ("row 76's §2 row above", "`herald-output.md`'s §2 row above (row 84)") instead of repeating the literal ID string.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `bash 34-check.sh --seed` assertion (b) passes; `grep -oE 'MB-[0-9]+' 34-AUDIT.md | sort | uniq -d` prints nothing.
- **Committed in:** `3319dbdf` (Task 1), `0bed7131` (Task 2)

**2. [Rule 1 - Bug] Corrected three per-token grep counts in the superstep-engine decision subsection during self-review**
- **Found during:** Task 1, immediately after drafting the subsection, re-running each cited `grep -c` command to verify the numbers before committing
- **Issue:** The first draft mis-copied the `Waypoint`-bare page list (wrongly included `eval-harness.md`, omitted `agent-runtime.md`), understated `max_supersteps`'s second hit (only counted `control-flow.md`, missed `fault-tolerance.md:399`), and claimed zero `Vanguard` hits in `user-guides/` when `fault-tolerance.md:228` actually has one.
- **Fix:** Re-ran each `grep -c`/`grep -n` command fresh and rewrote the per-token table to match the reproduced output exactly, including the one genuine `Vanguard` hit and its context.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** Every number in the "Superstep-engine dedicated-page decision" subsection's table was re-derived from a freshly-run command immediately before this summary was written.
- **Committed in:** `3319dbdf` (Task 1)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — the same recurring literal-ID-collision bug this phase's own `34-check.sh` gate has caught in every prior plan, plus one self-caught evidence-accuracy correction)
**Impact on plan:** Neither fix touches anything outside `.planning/`, and neither changes any verdict, MB ID, or size already assigned — both were required to make the plan's own `<verify>` blocks pass as specified and to keep the audit's own evidence honest.

## Issues Encountered
- `cicd.md`'s "CI Pipeline"/"Release Pipeline" YAML samples and `testing-guide.md`'s "CI Integration" YAML sample are both fabricated in the same way 34-03's `stable-api.md`/`design-patterns.md` findings were — plausible-looking, internally consistent code that simply does not correspond to any file or job in the live tree. Both pages had ALREADY been through a partial 2026-08-24 correction pass (visible as "Corrected 2026-08-24 (Phase 16 / DOCS-01)" callouts elsewhere on the same pages) that fixed some fabricated sections but missed these ones — a useful signal that a page carrying one correction callout is not proof the whole page is clean.
- Two findings (`monitoring.md`/`troubleshooting.md`'s "opentelemetry is not a dependency" claim) were true when written and became false only because Phase 28 — within this very milestone — added `opentelemetry` as a real, optional dependency. This is a different failure mode from "always been wrong": a correction pass can itself go stale if a later phase changes the fact it corrected against, which is exactly what D-00b's "content, never mtime" rule exists to catch mechanically rather than trusting a page's own "Corrected" timestamp as proof of current accuracy.
- `benches/engine_benchmarks.rs` (added 2026-09-02, within-milestone) makes `performance-tuning.md`'s own 2026-08-24 correction one file stale — the same "a correction can itself go stale" pattern as the opentelemetry finding, at a smaller scale.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- `34-AUDIT.md` §2 now carries 59 settled rows (1 from 34-01 + 18 from 34-03 + 40 from this plan) out of 93 pages, plus the one new missing-page row (94) — 34 `appendix/` pages remain for plan 34-05.
- MB numbering is at MB-36; plan 34-05 continues from MB-37 without renumbering any row above.
- `34-check.sh --seed` remains green; `--final` mode's three additional assertions are still not exercised (expected — the §2 sweep completes with plan 34-05, `examples/*.rs` mapping is plan 34-08's scope, and work-list assembly is plan 34-09's scope).
- No blockers. The volume and severity of genuine defects this plan found (a still-described-as-unimplemented Phase 24 capability, a one-version-behind fingerprint claim, an entirely absent Phase 33 RAG surface, a fabricated CI/CD pipeline sample, a coverage command that undercounts adapter coverage) is a strong signal that Phase 35's mdBook remediation work is substantial, not a light touch-up pass — plan 34-09's work-list assembly should expect a heavier-than-`MB-17`-baseline Phase 35 scope.

---
*Phase: 34-documentation-currency-audit*
*Completed: 2026-09-17*

## Self-Check: PASSED

All three created/modified artifacts found on disk (`34-AUDIT.md`, `34-EVIDENCE.md`,
`deferred-items.md`); both task commits (`3319dbdf`, `0bed7131`) found in `git log`.
