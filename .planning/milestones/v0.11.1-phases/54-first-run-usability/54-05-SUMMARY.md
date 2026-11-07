---
phase: 54-first-run-usability
plan: 05
subsystem: docs
tags: [changelog, adr, node-cache, docs, first-run]
status: complete

requires:
  - phase: 54-first-run-usability
    provides: "54-01..54-04 first-run fixes; this plan touches disjoint files"
provides:
  - "paladin-memory and paladin-content [Unreleased] ### Fixed entries correcting the 0.11.0 'No functional change' line"
  - "ADR-0056 amended in place to cite ADR-0057 (check-only boundary, accepted race, WINDOWS row 65)"
  - "fault-tolerance.md sections 'Node cache on the shipped server' and 'Cache key version and rotation'"
affects: [54-09 evidence, 58 lockstep release (finalize stamps [0.11.1])]

tech-stack:
  added: []
  patterns:
    - "Additive corrections: published [0.11.0] sections stay byte-identical; the fix lands under [Unreleased]"
    - "One authoritative doc section, linked from other pages rather than restated"

key-files:
  created: []
  modified:
    - crates/paladin-memory/CHANGELOG.md
    - crates/paladin-content/CHANGELOG.md
    - .planning/decisions/0056-allowance-admission-model.md
    - docs/src/user-guides/fault-tolerance.md
    - docs/src/user-guides/reasoning-effort.md
    - docs/src/getting-started/configuration.md

key-decisions:
  - "Crate corrections go under ### Fixed, each opening 'Correction to the [0.11.0] entry:' (PD-05-1)"
  - "ADR-0056 is amended in place; no new ADR, PROMOTION.md untouched (PD-05-2)"
  - "Two sub-headings under Node Caching hold all FIRST-09 content; other pages link (PD-05-3)"

requirements-completed: [FIRST-05, FIRST-06, FIRST-09]

duration: ~15min
completed: 2026-11-01
---

# Phase 54 Plan 05: Stale records corrected Summary

Three records that contradicted what shipped now say what shipped: the memory and content 0.11.0 changelog correction (one-way persisted key change), ADR-0056's amendment citing ADR-0057, and the node-cache guide's shipped-server and key-rotation text.

## Plan-start SHA

`6da0b66ffb405fa13374e70644d2894f9f7232b6`

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | memory and content changelog `[Unreleased]` corrections | 466390bcd |
| 2 | ADR-0056 amendment citing ADR-0057 | a04affd41 |
| 3 | node-cache guide sections and links | 6cb77fb96 |

## Prove-then-lock outputs

**Task 1.** Before editing, neither changelog had `## [Unreleased]` (grep printed nothing); both `[0.11.0]` sections held only `No functional change; version bumped in lockstep with the workspace.`

**Task 2.** `grep -n 'ADR-0057' src/application/services/treasurer/tests.rs` hit lines 17 and 19 (module doc already cites ADR-0057 and WINDOWS row 65), so the file was left untouched. `grep -n 'reserv' 0056-...md` before editing hit the stale passages at l.57, 70, 120, 122, 217-219 plus unrelated uses (l.23, 31).

**Task 3.** `grep -n 'attaches no node cache\|NODE_CACHE_KEY_VERSION\|key_prefix'` on fault-tolerance.md hit only l.364 and l.376 (`key_prefix` in the backends paragraph and table). `grep -rn 'NodeCacheConfig' src/bin src/infrastructure/web crates/paladin-web/src` printed nothing (rc=1), confirming the shipped server reads no `NodeCacheConfig`.

## Scratch finalize run

Run against copies in a scratchpad directory (real changelogs untouched):

```
./scripts/finalize-crate-changelogs.sh --version 0.11.1 --date 2026-11-01 --metadata-json <scratch>/meta.json
OK: 2 publishable package(s) checked for version '0.11.1' (date 2026-11-01):
  - paladin-content: finalized
  - paladin-memory: finalized
```

Each copy then read `## [Unreleased]`, blank, `## [0.11.1] - 2026-11-01`, `### Fixed`, the correction bullet, then `## [0.11.0] - 2026-10-26`. The real files keep `## [Unreleased]` and have no `[0.11.1]` heading; the lockstep bump is Phase 58's.

## Verification

- Tracer gate (auto mode): the tracer's verify command re-ran clean before the expansion tasks.
- `make check-changelogs` and `make check-doc-config` exit 0 (161 YAML blocks, 0 failed).
- `git diff 6da0b66ff -- crates` shows additions only; `[0.11.0]` sections byte-identical.
- ADR-0056: amendment paragraph present, no stale phrase remains, 2026-10-06 dated note kept, `ADR-0057` appears 8 times; `PROMOTION.md` and `treasurer/tests.rs` unchanged.
- `git diff --stat 6da0b66ff HEAD` lists exactly the six `files_modified`.

## Deviations from Plan

None - plan executed exactly as written. The ADR (c) and (d) edits reflowed one following line each to keep the surrounding sentence intact.

## Known Stubs

None.

## Threat Flags

None. Docs and changelog edits only; T-54-12, T-54-13 and T-54-14 are mitigated by the entries above.

## Self-Check: PASSED

All six modified files exist; commits 466390bcd, a04affd41 and 6cb77fb96 are present on the branch.
