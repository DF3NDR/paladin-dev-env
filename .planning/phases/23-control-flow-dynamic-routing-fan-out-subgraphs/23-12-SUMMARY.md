---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 12
subsystem: docs
tags: [mdbook, migration-guide, changelog, semver-checks, msrv, coverage, api-surface, cargo-audit, cargo-deny]

# Dependency graph
requires:
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "Waves 1-8 (Plans 23-01..23-11): Directive/NextStep routing (CF-02), DirectiveParser
      (D-11), Muster fan-out (CF-03), NodeSpec::Battalion subgraphs (CF-04), LlmDecisionEvaluator
      and Commander StrategySelection::Semantic (CF-05), the BUG-01 fail-closed fix (M-B-01), and
      EngineConfig (23-07)."
provides:
  - "docs/src/user-guides/control-flow.md: the mdBook page for Directives, DirectiveParser,
    Muster, subgraphs and LLM-evaluated edges, wired into docs/src/SUMMARY.md."
  - "MIGRATION.md §9.2 with no CF-owned TBD and an explicit deliberate-zero note for the
    new-in-0.10 engine/Waypoint types this phase reshaped."
  - "CHANGELOG.md [Unreleased] entries for M-B-01 and CF-02..CF-05."
  - "Recorded green program-gate evidence (semver-checks, MSRV 1.88, make security, clippy, fmt,
    coverage, API-surface) on the phase's final commit, plus a regenerated
    .project/current-exports.txt for the CF-05 EngineConfig surface."
affects: [phase-24-hitl, phase-29-ship]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Program-gate closeout as its own auto plan (no vertical slice): docs + register + evidence,
      not runtime behavior."

key-files:
  created:
    - docs/src/user-guides/control-flow.md
  modified:
    - docs/src/SUMMARY.md
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "The control-flow mdBook page links to MIGRATION.md and rustdoc by plain text/path reference,
    not by markdown hyperlink or docs.rs URL — mdbook-linkcheck runs with warning-policy=error and
    no existing page in this book hyperlinks to root-level MIGRATION.md, so an unverified anchor
    guess was judged a build-break risk not worth taking for a citation."
  - "WarGraph::validate's signature change is recorded precisely (gained edge_evaluators:
    &EdgeEvaluatorRegistry for CF-01's fail-closed check) rather than described as 'unchanged in
    shape', after checking the actual function signature in graph.rs."

requirements-completed: [CF-01, CF-02, CF-03, CF-04, CF-05]

coverage:
  - id: D1
    description: "New mdBook page docs/src/user-guides/control-flow.md documents Directives
      (Goto/End/Parley), DirectiveParser, Muster, subgraphs (NodeSpec::Battalion/StateMap) and
      LLM-evaluated edges (LlmDecisionEvaluator, Commander Semantic) with minimal examples and a
      short WarGraph preamble; wired into docs/src/SUMMARY.md under User Guides."
    requirement: "CF-01"
    verification:
      - kind: other
        ref: "grep -c 'control-flow.md' docs/src/SUMMARY.md == 1; awk User-Guides-block check prints the entry"
        status: pass
      - kind: other
        ref: "cargo doc --workspace --no-deps (17 unresolved-link warnings, all pre-existing at HEAD -- no .rs file touched)"
        status: pass
      - kind: other
        ref: "cargo test --workspace --doc (0 failed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "MIGRATION.md §9.2 carries no CF-owned TBD and an explicit deliberate-zero note
      for StateNode/NodeSpec/NodeContext/EngineLimits/EngineError/WarGraph/Waypoint, all absent at
      v0.9.0."
    requirement: "CF-02"
    verification:
      - kind: other
        ref: "grep -c 'TBD — owner CF-' MIGRATION.md == 0"
        status: pass
      - kind: other
        ref: "grep -c for 'deliberate zero'/'v0.9.0'/'StateNode'/'Waypoint' in the new §9.2 note, all present"
        status: pass
    human_judgment: false
  - id: D3
    description: "CHANGELOG.md [Unreleased] records M-B-01 (Changed, linking MIGRATION.md §9.1)
      and Added entries for Directive routing, Muster, NodeSpec::Battalion subgraphs, LLM-evaluated
      edges/Commander Semantic, and EngineConfig; no [0.10.0] release heading added."
    requirement: "CF-05"
    verification:
      - kind: other
        ref: "grep -c 'M-B-01' CHANGELOG.md >= 1 and matched entry contains MIGRATION.md; substring
          checks for Directive/Muster/Battalion/LlmDecision/EngineConfig all present in [Unreleased]"
        status: pass
      - kind: other
        ref: "grep -c '^## \\[0\\.10\\.0\\]' CHANGELOG.md == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "Program-gate evidence recorded on the phase's final commit: cargo semver-checks
      (all 11 published crates vs v0.9.0), MSRV 1.88, make security, clippy -D warnings, fmt
      --check, workspace coverage >= 82%, and API-surface check (regenerated baseline for the
      CF-05 EngineConfig surface)."
    requirement: "CF-05"
    verification:
      - kind: other
        ref: "commands and verbatim output recorded below under 'Program-Gate Evidence'"
        status: pass
    human_judgment: false

duration: ~50min
completed: 2026-09-04
status: complete
---

# Phase 23 Plan 12: Program-Gate Closeout Summary

**Control-flow mdBook page (Directives/DirectiveParser/Muster/subgraphs/LLM-routing) wired into
the book; MIGRATION.md §9.2 fully resolved with a deliberate-zero note; CHANGELOG.md
`[Unreleased]` records CF-01..CF-05; and semver-checks, MSRV 1.88, `make security`, clippy, fmt,
coverage (87.56%) and the API-surface check are all green on the phase's final commit.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 3
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- Wrote `docs/src/user-guides/control-flow.md` (243 lines, under the plan's 400-line cap) covering
  `NextStep` (`Edges`/`Goto`/`Muster`/`End`/`Parley`), `DirectiveParser` (`PlainOutput`/
  `StructuredDirective`), Muster worker templates and the `muster.` namespace, `NodeSpec::Battalion`
  subgraphs with `StateMap`, `LlmDecisionEvaluator` and Commander `StrategySelection::Semantic`,
  the egress-boundary warning, and an M-B-01 migration callout — wired into `docs/src/SUMMARY.md`.
- Added `MIGRATION.md` §9.2's deliberate-zero note for the new-in-0.10 engine/Waypoint types this
  phase reshaped, and confirmed no CF-owned `TBD` remains anywhere in the file (the three CF-owned
  rows were already resolved by Plans 23-01/23-03).
- Added `CHANGELOG.md` `[Unreleased]` entries: `### Changed` for M-B-01 (custom edge conditions
  now fail closed) and `### Added` for Directive routing, Muster, `NodeSpec::Battalion` subgraphs,
  LLM-evaluated edges / Commander Semantic, and `EngineConfig`.
- Ran every program gate on the phase's final commit and recorded verbatim evidence: `cargo
  semver-checks` clean against v0.9.0 for all 11 published crates, MSRV 1.88 clean, `make
  security` clean, clippy/fmt clean, workspace coverage 87.56% (floor 82%), and regenerated
  `.project/current-exports.txt` after `check-api-surface.sh` caught the CF-05 `EngineConfig`
  surface it was tracking.

## Task Commits

1. **Task 1: The control-flow mdBook page, wired into the book** - `4738220c` (docs)
2. **Task 2: Finalise the §9.2 register and the CHANGELOG** - `4c5e8e2c` (docs)
3. **Task 3: Record the program-gate evidence** - `1fd36613` (chore — `.project/current-exports.txt`
   regeneration; the gate *evidence* itself is recorded here in the SUMMARY, not as a code change)

## Files Created/Modified

- `docs/src/user-guides/control-flow.md` - new mdBook page (243 lines)
- `docs/src/SUMMARY.md` - one new entry under `# User Guides`
- `MIGRATION.md` - §9.2 deliberate-zero note (+2 lines)
- `CHANGELOG.md` - `[Unreleased]` `### Changed`/`### Added` entries (+39 lines)
- `.project/current-exports.txt` - regenerated (62 new lines, all `paladin::config::engine::EngineConfig`
  struct/impls; 0 removals)

## Program-Gate Evidence

All commands below were run against commit `4c5e8e2c86d34da5181e1e855bb9ea41773557ca` (the state
after Tasks 1-2, before Task 3's own `.project/current-exports.txt` regeneration commit
`1fd36613`). Task 3 makes no source or test change, so no gate result differs between the two
commits — the regenerated baseline is exactly what `check-api-surface.sh` demanded at
`4c5e8e2c`.

**`cargo fmt --check`** — exit 0, no output (clean).

**`cargo clippy --workspace --all-targets --all-features -- -D warnings`** — exit 0.
`Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 57s`, zero warnings.

**`make security`** (cargo-audit + cargo-deny) — exit 0. Final line: `advisories ok, bans ok,
licenses ok, sources ok`. Two informational `cargo-deny` warnings noted (a yanked `spin 0.9.8`
transitive dependency, a duplicate `thiserror`/`thiserror-impl` 1.x/2.x pair) — neither is a
gate failure and neither is new to this plan (`git diff HEAD -- .cargo/audit.toml deny.toml` is
empty; this plan touches no dependency).

**MSRV 1.88** — `cargo +1.88 check --workspace --all-features --all-targets --locked` — exit 0.
`Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 52s`, zero errors.

**`cargo semver-checks`** vs the published v0.9.0 baseline, `--default-features`, all 11 crates
from the CI `semver` job's package list — every one reported `Summary no semver update required`,
exit 0:

| Package | Checks | Result |
|---|---|---|
| paladin-ai | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-ai-core | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-ports | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-battalion | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-herald | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-llm | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-memory | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-storage | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-notifications | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-content | 196 checks: 196 pass, 58 skip | no semver update required |
| paladin-web | 196 checks: 196 pass, 58 skip | no semver update required |

`.cargo/semver-checks-allowlist.toml` is unchanged (`git diff HEAD` empty) and stays at zero
entries — matching MIGRATION.md §9.2's CF-owned rows, all of which are `Deliberate-breaking? = N`.
`paladin-battalion` — the crate carrying every CF-01..CF-05 change — is included in the table
above and shows zero semver breaks, confirming the phase's additive-only claim mechanically rather
than by inspection alone.

**`cargo test --workspace`** — exit 0. `test result: ok. 523 passed; 0 failed` (paladin-ai lib) and
every other crate's suite reports `0 failed` (41 `test result:` lines total across the workspace,
grep-verified for zero `FAILED`).

**`cargo doc --workspace --no-deps`** — exit 0. 17 `warning: unresolved link` occurrences, all
pre-existing at HEAD (confirmed via `git status --short` showing zero `.rs` files touched by this
plan — the count cannot have moved from a docs-only change).

**`cargo test --workspace --doc`** — exit 0, `0 failed` across every crate (104/63/33/116/5/10
passed per-crate, remainder 0/0/0).

**Workspace line coverage** (`bash scripts/coverage.sh`, the same `cargo llvm-cov --workspace
--features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82` form
CI's `coverage` job runs, against live `redis`/`minio` compose-network services reachable from
this worktree) — exit 0. Summed from `lcov.info` (the same method the CI `coverage` job's summary
step uses, since `cargo llvm-cov report --summary-only --workspace` is not a valid invocation):

```
Scope: --workspace --features integration-tests,llm-all (the gated measurement)
Lines:     61598/70353 = 87.56%
Functions: 6637/8253 = 80.42%
```

87.56% is 5.56 points above the 82.00% ADR-0006 floor.

**`./scripts/check-api-surface.sh`** — first run: exit 1, diff showed 62 additions (all
`paladin::config::engine::EngineConfig` struct fields and trait impls — `Clone`, `Debug`,
`Default`, `Serialize`/`Deserialize`, `EnvOverridable`, the `From<EngineConfig> for EngineLimits`
conversion, and the auto marker traits) and 0 removals. This is CF-05's `EngineConfig`
(`src/config/engine.rs`, landed in Plan 23-07) — a real, additive surface move the tracked
baseline had not yet captured. Regenerated via `./scripts/extract-public-api.sh
.project/current-exports.txt` (2030 items); re-ran `check-api-surface.sh` — exit 0, `API surface
unchanged`.

## Decisions Made

- Linked the control-flow page to `MIGRATION.md` and to `DirectiveParser`'s rustdoc by plain
  text/path mention rather than a markdown hyperlink, after checking `docs/book.toml`
  (`[output.linkcheck] warning-policy = "error"`) and finding no existing page in the book
  hyperlinks to the root-level `MIGRATION.md` — an unverified anchor slug guess was judged a
  build-break risk not worth taking for a citation that plain text serves equally well.
- Recorded `WarGraph::validate`'s signature change precisely in the §9.2 deliberate-zero note
  (gained an `edge_evaluators: &EdgeEvaluatorRegistry` parameter) rather than the plan's looser
  "signature changed" framing, after reading the actual function signature in `graph.rs` line 488.

## Deviations from Plan

None — plan executed exactly as written. Task 3's `.project/current-exports.txt` regeneration is
prescribed by the task's own action step 2 ("Regenerate ... if and only if this phase moved the
public surface"), not a deviation.

## Issues Encountered

None. Redis and MinIO were reachable via their compose-network hostnames (`redis:6379`,
`minio:9000`) inside this worktree, so the full `integration-tests,llm-all` coverage measurement
ran rather than falling back to a reduced-evidence path.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 23 (Control Flow — Dynamic Routing, Fan-Out & Subgraphs) is fully closed: all five
  requirements (CF-01..CF-05) delivered across Plans 23-01..23-12, the mdBook page is reachable,
  `MIGRATION.md` carries no open CF-owned item, `CHANGELOG.md [Unreleased]` records every
  user-visible change, and every program gate is green with recorded evidence.
- `MIGRATION.md`'s remaining `TBD` items (M-B-02/M-B-03 worked examples, §9.6 HTTP API, §9.7
  Deprecations, §9.8 Upgrade checklist, various non-CF §9.2 rows) belong to later phases (HITL-04
  Phase 24, RT-07 Phase 26, SHIP-01/SHIP-02 Phase 29) per their existing owner annotations — none
  is CF-owned and none was touched by this plan, per D-29's `.project/` registration scope.
- No blockers for Phase 24 (HITL).

## Self-Check: PASSED

- FOUND: `docs/src/user-guides/control-flow.md`
- FOUND: `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-12-SUMMARY.md`
- FOUND commit `4738220c` (Task 1)
- FOUND commit `4c5e8e2c` (Task 2)
- FOUND commit `1fd36613` (Task 3)

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-04*
