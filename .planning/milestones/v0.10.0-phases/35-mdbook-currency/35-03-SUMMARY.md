---
phase: 35-mdbook-currency
plan: 03
subsystem: docs
tags: [mdbook, docs, version-pins, msrv, feature-flags, control-flow, fault-tolerance, tool-integration]

# Dependency graph
requires:
  - phase: 35-mdbook-currency (plan 01, wave 1)
    provides: "docs/src/user-guides/superstep-engine.md" (the MB-30 engine guide, linked from control-flow.md's MB-22 fix)
provides:
  - "Nine Getting Started / User Guides pages brought to v0.10.0 currency (version pins, MSRV 1.88, regenerated feature-flag inventory)"
  - "control-flow.md's first inbound link to the new superstep-engine guide, and a corrected, shipped-as-of-Phase-24 description of NextStep::Parley"
  - "fault-tolerance.md's graph fingerprint claim corrected from v5 to v6"
  - "tool-integration.md's reachability note extended with the shipped prompt-level tool-call middleware pair"
affects: [35-10 (CHANGELOG + exit greps + 35-EVIDENCE.md)]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - docs/src/getting-started/installation.md
    - docs/src/getting-started/quickstart.md
    - docs/src/user-guides/agent-orchestrator-bridge.md
    - docs/src/user-guides/content-processing.md
    - docs/src/user-guides/maneuver-flow-dsl.md
    - docs/src/user-guides/orchestration.md
    - docs/src/user-guides/control-flow.md
    - docs/src/user-guides/fault-tolerance.md
    - docs/src/user-guides/tool-integration.md

key-decisions:
  - "installation.md's Feature Flag Profiles table keeps its existing small 'recommended profile' shape (now the three default LLM providers plus the four opt-in flags the page already used) and gains a new 'Full feature inventory' table below it listing every Cargo.toml [features] flag, its crate, and what it gates, per D-22 — the page's structure was not replaced."
  - "installation.md carries no Rust code fence, so D-11(b)'s illustrative-fragment header note was not added to it (nothing to mark illustrative); the other eight pages in this plan already carry the note from prior phases."
  - "control-flow.md's NextStep::Parley code-sample comment ('suspend — not implemented this phase, fails the run') was corrected alongside the prose fix even though the plan's action text says to leave the code sample alone — the plan's own carve-out is scoped to variant order and field shapes (which the audit confirmed match the live enum), not to a trailing comment that would otherwise directly contradict the corrected prose two lines below it on the same page (Rule 1 - bug)."

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "installation.md corrected to v0.10.0 dependency pins, MSRV 1.88, and a Cargo.toml-regenerated Feature Flag Profiles inventory (MB-06)"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "grep -cE '\"0\\.[5-9]\\.[0-9]+\"' docs/src/getting-started/installation.md == 0; grep -cE '\\b1\\.(70|75|85)(\\.[0-9]+)?\\b' docs/src/getting-started/installation.md == 0; page names otel/dev-ui/redis-cache/storage-postgres/llm-kimi/llm-qwen/llm-grok/llm-ollama/llm-gemini/llm-openai-compatible/vision/content-processing/web-server/notifications/qdrant/cli"
        status: pass
      - kind: other
        ref: "mdbook build docs/ (No broken links found); ./scripts/check-doc-config.sh (152 YAML blocks, 0 failed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "quickstart.md, agent-orchestrator-bridge.md, content-processing.md, maneuver-flow-dsl.md, orchestration.md retargeted to v0.10.0 (MB-07, MB-18, MB-21, MB-25, MB-26), one commit per page"
    requirement: "CURR-07"
    verification:
      - kind: other
        ref: "grep -rcE '\"0\\.[5-9]\\.[0-9]+\"' quickstart.md maneuver-flow-dsl.md == 0; grep -rcE 'current \\*\\*v0\\.[5-9]' agent-orchestrator-bridge.md content-processing.md orchestration.md == 0; git log --oneline --grep 'MB-07|MB-18|MB-21|MB-25|MB-26' each >= 1 commit"
        status: pass
      - kind: other
        ref: "mdbook build docs/ (No broken links found)"
        status: pass
    human_judgment: false
  - id: D3
    description: "control-flow.md links the new superstep-engine guide and drops the planning-corpus reference (D-10); Parley described as shipped since Phase 24 with the APP_ENGINE_MAX_MUSTER_TASKS override named; fault-tolerance.md's fingerprint version corrected to v6; tool-integration.md's reachability note gains the shipped ToolCallProtocolMiddleware/FinishOnPlainAnswerMiddleware pair and reasoning_agent preset, linked to agent-runtime.md (MB-22, MB-23, MB-29)"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "grep -c 'superstep-engine.md' control-flow.md == 1; grep -c '23-CONTEXT.md' control-flow.md == 0; grep -c 'ParleyNotSupported' control-flow.md == 0; grep -c 'APP_ENGINE_MAX_MUSTER_TASKS' control-flow.md == 1; grep -cE 'fingerprint version `v5`|version `v5`' fault-tolerance.md == 0 and page contains v6; tool-integration.md contains ToolCallProtocolMiddleware, FinishOnPlainAnswerMiddleware, reasoning_agent, agent-runtime.md link"
        status: pass
      - kind: other
        ref: "mdbook build docs/ (No broken links found); ./scripts/check-doc-examples.sh (0 checked, 623 skipped, 0 failed — all included examples compile, README in sync)"
        status: pass
    human_judgment: false

duration: ~10min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 03: Getting Started / User Guides D-20 Sweep + Three Content Corrections Summary

**Nine Getting Started and User Guides pages brought to v0.10.0 currency — version pins, MSRV 1.88, a Cargo.toml-regenerated feature-flag inventory, control-flow.md's first link to the new engine guide, a corrected shipped-Parley narrative, the fault-tolerance fingerprint bump to v6, and tool-integration.md's shipped prompt-level tool-call middleware note.**

## Performance

- **Duration:** ~10 min
- **Completed:** 2026-09-17T14:06:00Z
- **Tasks:** 3 (9 pages, 9 commits)
- **Files modified:** 9

## Accomplishments
- `installation.md` (MB-06): MSRV corrected to 1.88 across the toolchain table, the "Why Rust >=" heading and the verification line; every dependency pin changed from `"0.5.0"` to `"0.10.0"`; the Feature Flag Profiles table regenerated from the facade `Cargo.toml` `[features]` block into a minimal recommended profile plus a full inventory of all shipped flags (crate + what it gates), including the previously-undocumented `otel`, `dev-ui`, `redis-cache`, `storage-postgres` and the six additional LLM provider flags; a sentence added noting `paladin-cli` requires `--features cli`.
- `quickstart.md`, `agent-orchestrator-bridge.md`, `content-processing.md`, `maneuver-flow-dsl.md`, `orchestration.md` (MB-07, MB-18, MB-21, MB-25, MB-26): every stale version pin and "current vX workspace" sentence corrected to `0.10.0`/`v0.10.0`, one commit per page, no other content touched (constructor calls, feature-flag claims, subcommand docs and `{{#include}}` anchors left exactly as the audit confirmed them).
- `control-flow.md` (MB-22): the deferred-engine-guide sentence replaced with a link to `[WarEngine: Battlefield State & Superstep Execution](superstep-engine.md)`, dropping the `23-CONTEXT.md` planning-corpus reference (D-10); the `NextStep::Parley` prose rewritten to describe the mechanism as fully implemented since Phase 24 (HITL-01/HITL-02), linking `parley-and-chronicle.md`; `APP_ENGINE_MAX_MUSTER_TASKS` added beside the existing `max_muster_tasks` mention.
- `fault-tolerance.md` (MB-23): the graph fingerprint version claim corrected from `v5` to `v6`, with a note that Phase 26's `output_schema` addition is what bumped it, linking the new engine guide's fingerprint section.
- `tool-integration.md` (MB-29): the Reachability note extended to keep the still-true narrower fact (no adapter populates the wire-level function-call field) while adding the shipped opt-in `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware` pair installed by the `reasoning_agent` preset, linked to `agent-runtime.md`.

## Closure Table (D-23)

| MB-nn | Page | Disposition | Commit | How the §5 finding was addressed |
|-------|------|-------------|--------|-----------------------------------|
| MB-06 | `docs/src/getting-started/installation.md` | corrected | `aa2e1883` | MSRV 1.85→1.88, all six pins 0.5.0→0.10.0, Feature Flag Profiles table regenerated from `Cargo.toml` `[features]` (5 flags → 28 flags across two tables) |
| MB-07 | `docs/src/getting-started/quickstart.md` | corrected | `700c2588` | Three dependency pins 0.7.0→0.10.0; execution-service constructor call left untouched |
| MB-18 | `docs/src/user-guides/agent-orchestrator-bridge.md` | corrected | `926fd5f1` | "current **v0.5.0**" → "current **v0.10.0**" |
| MB-21 | `docs/src/user-guides/content-processing.md` | corrected | `d75783a5` | Header sentence and "not yet implemented in v0.5.0" line both → v0.10.0; declared-but-no-adapter and commented-out claims left untouched |
| MB-25 | `docs/src/user-guides/maneuver-flow-dsl.md` | corrected | `ddd79c33` | Crate pin and closing Version line 0.8.0→0.10.0; subcommand docs left untouched |
| MB-26 | `docs/src/user-guides/orchestration.md` | corrected | `3bdc862d` | "current **v0.8.0**" → "current **v0.10.0**"; builder samples and `{{#include}}` anchors left untouched |
| MB-22 | `docs/src/user-guides/control-flow.md` | corrected | `778bc53d` | Deferred-documentation sentence replaced with a link to `superstep-engine.md` (D-10); Parley prose + code-sample comment rewritten to describe the shipped Phase 24 mechanism, linked to `parley-and-chronicle.md`; `APP_ENGINE_MAX_MUSTER_TASKS` named beside `max_muster_tasks` |
| MB-23 | `docs/src/user-guides/fault-tolerance.md` | corrected | `d2156c0e` | Fingerprint version claim v5→v6, with a note on Phase 26's `output_schema` bump and a link to the engine guide's fingerprint section |
| MB-29 | `docs/src/user-guides/tool-integration.md` | corrected | `35630a85` | Reachability note extended with `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware` and the `reasoning_agent` preset, linked to `agent-runtime.md` |

## Task Commits

Each task was committed atomically, one commit per page (D-24):

1. **Task 1: MB-06 — installation.md, the full D-20 and D-22 sweep** — `aa2e1883` (docs)
2. **Task 2: MB-07, MB-18, MB-21, MB-25, MB-26 — the version-string sweep** — `700c2588`, `926fd5f1`, `d75783a5`, `ddd79c33`, `3bdc862d` (docs, 5 commits)
3. **Task 3: MB-22, MB-23, MB-29 — the three line-pinned content corrections** — `778bc53d`, `d2156c0e`, `35630a85` (docs, 3 commits)

No separate plan-metadata commit — this is a worktree-mode execution; the orchestrator handles the shared STATE.md/ROADMAP.md commit centrally after the wave merges. This SUMMARY.md is committed separately below.

## Files Created/Modified
- `docs/src/getting-started/installation.md` — MSRV 1.88, v0.10.0 pins, regenerated Feature Flag Profiles table
- `docs/src/getting-started/quickstart.md` — v0.10.0 dependency pins
- `docs/src/user-guides/agent-orchestrator-bridge.md` — v0.10.0 workspace sentence
- `docs/src/user-guides/content-processing.md` — v0.10.0 workspace sentence, v0.10.0 capabilities-table intro
- `docs/src/user-guides/maneuver-flow-dsl.md` — v0.10.0 crate pin and Version line
- `docs/src/user-guides/orchestration.md` — v0.10.0 workspace sentence
- `docs/src/user-guides/control-flow.md` — superstep-engine.md link, corrected Parley narrative, APP_ENGINE_MAX_MUSTER_TASKS
- `docs/src/user-guides/fault-tolerance.md` — fingerprint version v5→v6
- `docs/src/user-guides/tool-integration.md` — reachability note extended with shipped middleware pair

## Decisions Made
- installation.md's Feature Flag Profiles table keeps a short recommended-profile table (now the three default LLM providers) and gains a full-inventory table below it per D-22, rather than replacing the page's existing structure.
- No illustrative-fragment header note was added to installation.md — the page carries no Rust code fence, so D-11(b)'s "mark it illustrative" clause has nothing to act on there.
- The `NextStep::Parley` code-sample trailing comment in control-flow.md was corrected alongside the prose (Rule 1 - bug: the plan's "leave the code sample alone" carve-out is scoped to variant order/fields, not a comment that would otherwise contradict the corrected prose two lines below it).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected the stale `NextStep::Parley` code-sample comment on control-flow.md**
- **Found during:** Task 3 (control-flow.md, MB-22)
- **Issue:** The `NextStep` enum code sample's inline comment on the `Parley` variant read `// suspend — not implemented this phase, fails the run`. The plan's action text says "Leave the `NextStep` code sample alone — the audit confirmed the variant order and fields match the live enum," but that carve-out is about structure, not this comment's stale claim — which, left in place, would directly contradict the corrected prose two lines below it stating Parley is fully implemented since Phase 24.
- **Fix:** Changed the comment to `// suspend the run awaiting external input (see below)`.
- **Files modified:** `docs/src/user-guides/control-flow.md`
- **Verification:** `grep -c 'ParleyNotSupported' control-flow.md` returns 0; `mdbook build docs/` and `./scripts/check-doc-examples.sh` both still pass.
- **Committed in:** `778bc53d` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary for internal page consistency; no scope creep — same file, same task, same MB-22 finding.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All nine MB-nn rows this plan owns are closed with one commit per page; `mdbook build docs/`, `./scripts/check-doc-examples.sh`, and `./scripts/check-doc-config.sh` all exit 0 on the final commit.
- `control-flow.md` now carries the MB-22 inbound link to `superstep-engine.md` that plan 35-01's dependents (MB-05, MB-09, MB-11) also need to land, per D-10.
- No deferred observations were made on any page the audit settled `current` — nothing to fold into `deferred-items.md` for plan 35-10.
- Plan 35-10 (CHANGELOG + exit greps + `35-EVIDENCE.md`) can cite this plan's nine commits directly via `git log --oneline --grep 'MB-'`.

## Self-Check: PASSED

All 9 modified pages plus this SUMMARY.md file confirmed present on disk; all 9 task commit
hashes (`aa2e1883`, `700c2588`, `926fd5f1`, `d75783a5`, `ddd79c33`, `3bdc862d`, `778bc53d`,
`d2156c0e`, `35630a85`) confirmed present in `git log --oneline`.

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*
