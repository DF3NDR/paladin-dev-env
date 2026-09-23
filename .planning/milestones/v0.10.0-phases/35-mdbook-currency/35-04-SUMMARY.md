---
phase: 35-mdbook-currency
plan: 04
subsystem: docs
tags: [mdbook, introduction, domain-model, commissary, overview, hexagonal-design, design-patterns, vocabulary]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: "plan 35-01's docs/src/user-guides/superstep-engine.md nav-linked page, the forward-link target for D-10's remaining engine dependents"
provides:
  - "MB-02, MB-03, MB-04, MB-05, MB-08, MB-09, MB-10, MB-11 and MB-12 closed"
  - "docs/src/introduction.md's Medieval Military Theme table cut to a labelled 8-term excerpt linking domain-model.md's full table (D-18)"
  - "docs/src/architecture/domain-model.md's GarrisonEntry struct corrected to the live shape and Battlefield/Waypoint/Aegis/TraceRecord added to Core Domain Entities (D-19)"
  - "docs/src/architecture/commissary.md line 7 reworded off the literal pre-rename term, pointing at ADR-0049 (D-17)"
  - "docs/src/architecture/overview.md naming eleven library crates plus the facade and the Phase 22-33 surface (WarEngine, Battlefield, Waypoint, Aegis, Commissary, TraceRecord, Platform API)"
  - "docs/src/architecture/hexagonal-design.md's LlmPort excerpt corrected to the single-LlmRequest-parameter signature"
  - "docs/src/architecture/design-patterns.md's pattern-5 constructor corrected to name ArsenalPort as the fourth parameter"
affects: ["35-10 (phase close, folds the adr-index.md deferred observation into deferred-items.md and reruns the phase-wide D-21 exit greps)"]

# Tech tracking
tech-stack:
  added: []
  patterns: ["labelled term-table excerpt pattern for a page that references but does not duplicate the domain-model.md ubiquitous-language table (D-18)"]

key-files:
  created:
    - .planning/phases/35-mdbook-currency/35-04-SUMMARY.md
  modified:
    - docs/src/introduction.md
    - docs/src/architecture/domain-model.md
    - docs/src/architecture/commissary.md
    - docs/src/architecture/overview.md
    - docs/src/architecture/hexagonal-design.md
    - docs/src/architecture/design-patterns.md

key-decisions:
  - "The introduction's Medieval Military Theme excerpt keeps exactly 8 terms (Paladin, Battalion, Formation, Phalanx, Campaign, Chain of Command, Garrison, Arsenal), all spelled identically to domain-model.md's table, per D-18's at-most-eight rule — Maneuver, Armament, Citadel and Herald were dropped from the excerpt (still present in the one complete list on domain-model.md)."
  - "hexagonal-design.md's OpenAIAdapter sample directly below the corrected LlmPort trait was also updated to the single-LlmRequest-parameter form (Rule 1): the plan's action scoped the fix to the trait excerpt and told the executor to leave GarrisonPort/SanctumPort/ArsenalPort/FileStoragePort samples alone, but the OpenAIAdapter block is an `impl LlmPort for OpenAIAdapter` of the exact trait just corrected — leaving it on the two-parameter form would have made the page self-contradictory within four lines of scroll."
  - "introduction.md gained a new '🧭 Deployment Topologies' section (previously absent) rather than folding the deployment-topologies/overview.md link into the existing '🚢 Deployment' section, mirroring docs/src/SUMMARY.md's own separate top-level chapter for the topology-selection guide."

requirements-completed: [CURR-09]

coverage:
  - id: D1
    description: "MB-04/MB-05 closed: introduction.md's term table is an 8-term excerpt naming and linking domain-model.md, and the nav index links every page the audit found missing plus a deployment-topologies page"
    requirement: "CURR-09"
    verification:
      - kind: other
        ref: "for p in control-flow.md fault-tolerance.md parley-and-chronicle.md agent-runtime.md eval-harness.md superstep-engine.md platform-api.md observability.md commissary.md deployment-topologies; do grep -q \"$p\" docs/src/introduction.md; done && grep -q domain-model.md docs/src/introduction.md"
        status: pass
      - kind: other
        ref: "mdbook build docs/ -- No broken links found"
        status: pass
    human_judgment: false
  - id: D2
    description: "MB-02 closed: commissary.md line 7 no longer names the pre-rename term, points at ADR-0049 instead"
    requirement: "CURR-09"
    verification:
      - kind: other
        ref: "grep -q 'ADR-0049' docs/src/architecture/commissary.md"
        status: pass
      - kind: other
        ref: "grep -rniE '\\bQuartermaster\\b' docs/src/architecture/commissary.md -- empty"
        status: pass
    human_judgment: false
  - id: D3
    description: "MB-03/MB-11 closed: domain-model.md's GarrisonEntry fence matches the live struct field-for-field and Battlefield/Waypoint/Aegis/TraceRecord are documented with guide links"
    requirement: "CURR-09"
    verification:
      - kind: other
        ref: "grep -q ConversationRole && grep -q is_summary && grep -q 'Option<u32>' docs/src/architecture/domain-model.md; for e in Battlefield Waypoint Aegis TraceRecord; do grep -q \"$e\"; done"
        status: pass
      - kind: other
        ref: "grep -c MessageRole docs/src/architecture/domain-model.md == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "MB-08/MB-09 closed: overview.md names eleven library crates plus the facade (including paladin-eval, paladin-herald) and the Phase 22-33 surface"
    requirement: "CURR-09"
    verification:
      - kind: other
        ref: "grep -q paladin-eval && grep -q paladin-herald && grep -q superstep-engine.md && grep -q platform-api.md docs/src/architecture/overview.md"
        status: pass
      - kind: other
        ref: "grep -c 'nine focused crates' docs/src/architecture/overview.md == 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "MB-10 closed: hexagonal-design.md's LlmPort excerpt shows the single-LlmRequest-parameter generate signature"
    requirement: "CURR-09"
    verification:
      - kind: other
        ref: "grep -q LlmRequest docs/src/architecture/hexagonal-design.md"
        status: pass
    human_judgment: false
  - id: D6
    description: "MB-12 closed: design-patterns.md's pattern-5 constructor names ArsenalPort as the fourth parameter, not Herald"
    requirement: "CURR-09"
    verification:
      - kind: other
        ref: "grep -q ArsenalPort docs/src/architecture/design-patterns.md"
        status: pass
    human_judgment: false
  - id: D7
    description: "Full docs.yml gate sequence green on the plan's final commit"
    verification:
      - kind: other
        ref: "mdbook-mermaid install docs/ -- git status --porcelain -- docs empty aside from the intended edit"
        status: pass
      - kind: other
        ref: "mdbook build docs/ -- No broken links found (run after every commit)"
        status: pass
      - kind: other
        ref: "./scripts/check-doc-examples.sh -- 0 checked, 623 skipped, 0 failed"
        status: pass
      - kind: other
        ref: "./scripts/check-doc-config.sh -- 152 YAML blocks checked, 0 failed"
        status: pass
    human_judgment: false

# Metrics
duration: 35min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 04: Introduction and Architecture Currency Summary

**Closed nine MB-nn findings across `introduction.md` and the four architecture pages —
Medieval Military term-table excerpt, complete nav index, the corrected `GarrisonEntry` struct
plus four missing domain entities, the `Commissary` rename pointer to ADR-0049, eleven crates
plus the facade, and two corrected code-pattern fences — leaving one out-of-scope `Quartermaster`
occurrence in `adr-index.md` (plan 35-05's page) as the sole remaining SC4 gap.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-09-17T14:10:00Z
- **Completed:** 2026-09-17T14:19:15Z
- **Tasks:** 3 (one tracer, two auto)
- **Files modified:** 6

## D-23 Closure Table

| MB-nn | Page | Disposition | Commit | How the §5 finding was addressed |
|-------|------|-------------|--------|-----------------------------------|
| MB-04 | `docs/src/introduction.md` | fixed | `6fcd779e` | Medieval Military Theme table cut from 12 to 8 terms, each spelled exactly as `domain-model.md`'s table; a new sentence above it names it an excerpt and links `architecture/domain-model.md#medieval-military-naming-convention` as the full list |
| MB-05 | `docs/src/introduction.md` | fixed | `6fcd779e` | Nav index gained `superstep-engine.md`, `control-flow.md`, `parley-and-chronicle.md`, `fault-tolerance.md`, `agent-runtime.md`, `eval-harness.md` (User Guides); `commissary.md` (Architecture); a new "🧭 Deployment Topologies" section linking `deployment-topologies/overview.md`; `platform-api.md` (Deployment); `observability.md` (Operations) |
| MB-02 | `docs/src/architecture/commissary.md` | fixed | `24b0e5b0` | Line 7 reworded — "the design record, the rename rationale, and the rejected-name list are in ADR-0049" — the literal pre-rename term no longer appears on this page |
| MB-03 | `docs/src/architecture/domain-model.md` | fixed | `e2a2e81f` | `GarrisonEntry` fence replaced with the live seven-field struct (`id: Uuid`, `role: ConversationRole`, `content`, `timestamp: DateTime<Utc>`, `metadata`, `token_count: Option<u32>`, `is_summary: bool`), fenced `rust,ignore` with the source path already in the header comment |
| MB-11 | `docs/src/architecture/domain-model.md` | fixed | `e2a2e81f` | Four new Core Domain Entities subsections added — Battlefield, Waypoint, Aegis, TraceRecord — each one-to-two sentences, each linking its guide (`superstep-engine.md` x2, `fault-tolerance.md`, `observability.md`) |
| MB-08 | `docs/src/architecture/overview.md` | fixed | `ecde2121` | Opening line and crate table corrected from "nine focused crates" to "eleven library crates plus a facade"; `paladin-herald` and `paladin-eval` rows added with one-line purposes |
| MB-09 | `docs/src/architecture/overview.md` | fixed | `ecde2121` | Five new subsections added before Technology Stack — WarEngine, Aegis, Commissary, Observability (TraceRecord), Platform API — each linking its guide/page |
| MB-10 | `docs/src/architecture/hexagonal-design.md` | fixed | `aef92924` | `LlmPort` trait excerpt corrected from the two-parameter `(messages, config)` form to `generate(&self, request: LlmRequest)`; the directly-adjacent `OpenAIAdapter` sample also updated for internal consistency (deviation, see below) |
| MB-12 | `docs/src/architecture/design-patterns.md` | fixed | `7add15fe` | Pattern-5 `PaladinExecutionService::new` sample's fourth parameter corrected from `herald: Option<Arc<dyn Herald>>` to `arsenal: Option<Arc<dyn ArsenalPort>>`, matching the live constructor; struct field renamed `llm` → `llm_port` and reordered to match construction order, source path comment added |

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer): MB-04 and MB-05 — introduction.md end to end** — `6fcd779e`
2. **Task 2: MB-03, MB-11 and MB-02 — domain-model.md and commissary.md** — `e2a2e81f` (domain-model.md), `24b0e5b0` (commissary.md)
3. **Task 3: MB-08, MB-09, MB-10 and MB-12 — overview, hexagonal design and design patterns** — `ecde2121` (overview.md), `aef92924` (hexagonal-design.md), `7add15fe` (design-patterns.md)

The tracer task's own `<verify>` (nav-index links plus `mdbook build docs/`) was re-run and passed
before starting Task 2, per the tracer feedback gate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `hexagonal-design.md`'s `OpenAIAdapter` sample left inconsistent by a
literal reading of the task's file scope**
- **Found during:** Task 3, immediately after correcting the `LlmPort` trait excerpt
- **Issue:** The plan's action for MB-10 named only the trait excerpt and said "Leave the other
  port samples alone; the audit did not re-verify them and did not flag them" — but the
  `OpenAIAdapter` code block six lines below the trait excerpt is an `impl LlmPort for
  OpenAIAdapter` of the exact trait just corrected, still showing the retired two-parameter
  `generate(&self, messages: &[Message], config: &LlmConfig)` signature. Leaving it would make
  the page self-contradictory within the same screen.
- **Fix:** Updated the `OpenAIAdapter::generate` signature to `generate(&self, request:
  LlmRequest)`, matching the corrected trait.
- **Files modified:** `docs/src/architecture/hexagonal-design.md`
- **Verification:** `mdbook build docs/` — No broken links found; re-read confirms both blocks now
  agree.
- **Committed in:** `aef92924`

---

**Total deviations:** 1 auto-fixed (1 bug/consistency)
**Impact on plan:** Cosmetic-adjacent only — the `GarrisonPort`/`SanctumPort`/`ArsenalPort`/
`FileStoragePort` samples named in the plan's own scope carve-out were left untouched, per
instruction. No scope creep beyond the one directly-contradicted sibling block.

## Deferred observations

**`docs/src/contributing/adr-index.md` line 14 still names the pre-rename term.** This plan's
Task 2 `<verify>` runs the SC4 exit grep phase-wide
(`grep -rqiE '\bQuartermaster\b' docs/src`), and it is NOT empty: `adr-index.md`'s ADR-0049
summary row reads "Re-ported under new vocabulary, never as `Quartermaster`" — a historical
citation, but a literal match against the D-17/SC4 rule, which states "no allowlisted exception."
`adr-index.md` is not in this plan's `files_modified` (it was created by plan 35-05, commit
`745e8a64`, already merged into this worktree's base before this plan started — confirmed via
`git merge-base --is-ancestor 745e8a64 <base>`). Per the worktree parallel-execution constraint
("Do not touch any file outside your plan's `files_modified` list plus your own SUMMARY.md"),
this plan does not edit `adr-index.md`. This plan's own claim — "close the phase's vocabulary
criterion at its single remaining source" — is therefore not fully true: `commissary.md` was the
single remaining source known at plan-authoring time, but plan 35-05 introduced a second,
independent occurrence after that assumption was written. Recorded in the cross-phase
`.planning/WINDOWS.md` broken-windows ledger (`kind: unmet-truth`, phase 35, file
`docs/src/contributing/adr-index.md:14`) for plan 35-10 (phase close) to fold into
`deferred-items.md` and fix before the phase-wide D-21 exit greps are run and recorded in
`35-EVIDENCE.md`. The fix itself is a one-line rewording of the same shape as this plan's
`commissary.md` fix (e.g. "Re-ported under new vocabulary, never re-adopting the retired name" or
a pointer to ADR-0049 without the literal token) and requires no further design decision.

## Issues Encountered

None beyond the deferred `adr-index.md` observation above. Every struct/trait/constructor shape
used (`GarrisonEntry`, `Battlefield`, `Waypoint`, `Aegis`, `TraceRecord`, `LlmRequest`,
`LlmPort::generate`, `PaladinExecutionService::new`, the eleven workspace crate names) was
verified directly against the live tree before writing prose, matching the plan's `<read_first>`
list exactly.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- MB-02, MB-03, MB-04, MB-05, MB-08, MB-09, MB-10, MB-11 and MB-12 are closed.
- Three of D-10's four engine dependents (`introduction.md`, `architecture/overview.md`,
  `architecture/domain-model.md`) now link `user-guides/superstep-engine.md`; the fourth
  (`user-guides/control-flow.md`, MB-22) was already closed by plan 35-03 in the same wave.
- The phase-wide SC4 vocabulary grep has exactly one remaining hit, `adr-index.md:14`, owned by
  plan 35-05 and recorded in `.planning/WINDOWS.md` for plan 35-10 to close before the phase's
  final exit-grep evidence is recorded.
- No blockers for the remaining wave-2 plans or wave 3 (35-10, phase close).

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

All 6 modified files confirmed present on disk (`docs/src/introduction.md`,
`docs/src/architecture/domain-model.md`, `docs/src/architecture/commissary.md`,
`docs/src/architecture/overview.md`, `docs/src/architecture/hexagonal-design.md`,
`docs/src/architecture/design-patterns.md`). All 6 commit hashes confirmed present in `git log`
(`6fcd779e`, `e2a2e81f`, `24b0e5b0`, `ecde2121`, `aef92924`, `7add15fe`).
