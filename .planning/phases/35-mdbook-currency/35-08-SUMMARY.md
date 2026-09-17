---
phase: 35-mdbook-currency
plan: 08
subsystem: docs
tags: [mdbook, appendix, archive-banner, adr-0047, msrv, qdrant, release-automation]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: 35-01's engine guide and doc-examples scaffolding (this plan's tasks are independent
      of it but share the phase's docs.yml gate)
provides:
  - Five archive-tier appendix pages closed behind ADR-0047 banners (MB-01, MB-39, MB-47, MB-59, MB-60)
  - Three correct-tier appendix pages corrected in place (MB-37, MB-54, MB-55)
  - A repaired, renderable docs/src/appendix/user-rest-api.md (previously ended mid-Rust-source
    with unbalanced markdown structure)
affects: [35-mdbook-currency (35-10 exit greps and EVIDENCE.md), Phase 36 (rustdoc bar owns
  doc-coverage-report.md's eventual regeneration)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ADR-0047 archive banner: blockquote immediately after H1, bold 'Archived — historical
      document.' lead, names what/when/live-source/disposition-record"
    - "Leaked raw-source tail on an archive-tier page is fenced as inert text (```text) rather
      than rewritten, keeping the diff proportional (D-04) while making the page render cleanly"

key-files:
  created: []
  modified:
    - docs/src/appendix/doc-coverage-report.md
    - docs/src/appendix/user-rest-api.md
    - docs/src/appendix/user-system.md
    - docs/src/appendix/contributing-legacy.md
    - docs/src/appendix/build-baselines.md
    - docs/src/appendix/battalion-benchmarks.md
    - docs/src/appendix/sanctum-benchmarks.md
    - docs/src/appendix/release-automation.md

key-decisions:
  - "user-rest-api.md's malformed tail (a truncated Rust source/comment excerpt) was fenced as
    ```text with an explanatory note rather than rewritten — the page renders as a complete page
    without losing or editorializing the original content (D-02 'repaired only enough to render')"
  - "build-baselines.md's crate-count table header was reworded to name itself as the snapshot's
    own count, not today's tree; the historical toolchain rows were left untouched (D-20 exemption)"
  - "sanctum-benchmarks.md's Qdrant framing changed from 'future/not implemented' to 'shipped,
    benchmark numbers not yet captured' — a materially different claim, not a wording tweak"

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "MB-01 — doc-coverage-report.md archived behind an ADR-0047 banner naming
      ADR-0033 and Phase 36 as the live measure; the false zero-warning claim and the 9-crate list
      are reframed as the snapshot's own historical figures"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "mdbook build docs/ (No broken links found) + acceptance-criteria greps in
          35-08-PLAN.md Task 1, all run and passing this session"
        status: pass
    human_judgment: false
  - id: D2
    description: "MB-59, MB-60 — user-rest-api.md and user-system.md archived behind a shared
      ADR-0047 banner stating the paladin user CLI does not exist in the shipped binary while the
      user service/repository layers do; user-rest-api.md repaired to render (fences balanced)"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "awk fence-balance check + mdbook build docs/ (No broken links found), Task 2 verify
          block, run and passing this session"
        status: pass
    human_judgment: false
  - id: D3
    description: "MB-47 — contributing-legacy.md archived behind a banner pointing at
      contributing/development-setup.md; MSRV corrected to 1.88, crates/ workspace line added,
      placeholder clone URL replaced with the real repository URL"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "grep checks for 1.88 / DF3NDR/paladin-dev-env / absence of 1.70 and your-org/paladin,
          Task 2 verify block, run and passing this session"
        status: pass
    human_judgment: false
  - id: D4
    description: "MB-39 — build-baselines.md archived behind a banner naming it a dated Milestone 7
      snapshot and pointing at appendix/performance-baseline.md; crate-count table reframed as the
      snapshot's own figure"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "grep for performance-baseline.md + mdbook build docs/, Task 2 verify block, run and
          passing this session"
        status: pass
    human_judgment: false
  - id: D5
    description: "MB-37 — battalion-benchmarks.md's toolchain line corrected from 1.85+ to 1.88+,
      matching Cargo.toml rust-version; no archive banner (correct-tier, D-03)"
    requirement: "CURR-07"
    verification:
      - kind: other
        ref: "grep -q 1.88 && ! grep -qE '\\b1\\.85\\b', Task 3 verify block, run and passing"
        status: pass
    human_judgment: false
  - id: D6
    description: "MB-55 — sanctum-benchmarks.md's Qdrant adapter framing corrected from
      future/unimplemented to shipped-with-benchmarks-pending, in the summary Performance Targets
      line and the dedicated Qdrant section header and body"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "grep -ci 'when the qdrant adapter is implemented|Qdrant Adapter (Future' returns 0,
          Task 3 verify block, run and passing"
        status: pass
    human_judgment: false
  - id: D7
    description: "MB-54 — release-automation.md's operational caveat corrected to name all three
      publish-crates dependencies (test, create-release, check-release-consistency) and what the
      third gate enforces"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "grep -q check-release-consistency, Task 3 verify block, run and passing; cross-checked
          against .github/workflows/release.yml:605 needs: [test, create-release,
          check-release-consistency]"
        status: pass
    human_judgment: false

# Metrics
duration: 10min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 08: Appendix Archive-and-Correct Closure Summary

**Closed all eight D-01/D-02/D-03 appendix rows: five ADR-0047 archive banners (including a
repair of a malformed, unrenderable page) and three in-place corrections (MSRV, Qdrant shipped
status, and the publish job's third dependency).**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-09-17T14:12:36Z
- **Completed:** 2026-09-17T14:17:07Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments
- Five archive-tier appendix pages now carry an ADR-0047 banner within the first eight lines,
  each naming the live source of truth and the disposition record, with every path and nav entry
  preserved (D-01)
- `docs/src/appendix/user-rest-api.md` — previously ending mid-Rust-source with a leaked raw
  comment/test-source tail rendering as broken markdown — now renders as a complete page: the tail
  is fenced as inert text with an explanatory note, content unchanged
- Three correct-tier pages (`battalion-benchmarks.md`, `sanctum-benchmarks.md`,
  `release-automation.md`) corrected in place with no banner, per the locked D-03 tier assignment
- `mdbook build docs/` exits 0 with `No broken links found` after every commit;
  `./scripts/check-doc-config.sh` and `./scripts/check-doc-examples.sh` both pass on the final tree

## D-23 Closure Table

| MB-nn | page | disposition | commit | how the §5 finding was addressed |
|-------|------|-------------|--------|-----------------------------------|
| MB-01 | `appendix/doc-coverage-report.md` | archived | `59b3dcb1` | ADR-0047 banner naming ADR-0033 and Phase 36; the false "no warnings" claim and the 9-crate list reframed as the snapshot's own historical figures rather than corrected to today's count |
| MB-59 | `appendix/user-rest-api.md` | archived | `47ca479c` | ADR-0047 banner stating the `paladin user` CLI does not exist in the shipped binary while the user service/repository layers do; malformed tail fenced so the page renders (fences balance, `awk` check passes) |
| MB-60 | `appendix/user-system.md` | archived | `47ca479c` | Same shared banner as MB-59; clap pin left untouched (audit confirmed it matches the manifest) |
| MB-47 | `appendix/contributing-legacy.md` | archived | `3af85fc0` | ADR-0047 banner pointing at `contributing/development-setup.md`; MSRV corrected to 1.88, `crates/` workspace line added to the Project Structure block, placeholder clone URL replaced with `https://github.com/DF3NDR/paladin-dev-env` |
| MB-39 | `appendix/build-baselines.md` | archived | `2ac3cac0` | ADR-0047 banner naming it a dated Milestone 7 build-time snapshot, pointing at `appendix/performance-baseline.md`; crate-count table header reworded as the snapshot's own figure; historical toolchain rows left untouched (D-20 exemption) |
| MB-37 | `appendix/battalion-benchmarks.md` | corrected | `9dbecc08` | Toolchain line corrected from `1.85+` to `1.88+`, matching `Cargo.toml` `rust-version`; nothing else changed |
| MB-55 | `appendix/sanctum-benchmarks.md` | corrected | `59720e17` | Qdrant adapter framing corrected from future/unimplemented to shipped (module + `qdrant` feature both exist), with benchmark numbers recorded as not yet captured — a different claim from unimplemented; benchmark command and unfilled footer left as-is |
| MB-54 | `appendix/release-automation.md` | corrected | `189208f0` | Operational caveat corrected to name all three `publish-crates` dependencies (`test`, `create-release`, `check-release-consistency`) and state what the pre-publish consistency gate (PUBOPS-01) enforces |

## Task Commits

Each task was committed atomically (one commit per page, except the tightly-coupled user-page
pair per D-24):

1. **Task 1: MB-01 — archive doc-coverage-report.md end to end** — `59b3dcb1` (docs)
2. **Task 2: MB-59, MB-60, MB-47, MB-39 — the four remaining archive-tier pages** — `47ca479c`
   (user pages, docs), `3af85fc0` (contributing-legacy, docs), `2ac3cac0` (build-baselines, docs)
3. **Task 3: MB-37, MB-55, MB-54 — the three correct-tier pages** — `9dbecc08` (battalion-benchmarks,
   docs), `59720e17` (sanctum-benchmarks, docs), `189208f0` (release-automation, docs)

**Plan metadata:** this SUMMARY's own commit (docs: complete plan), made by the orchestrator after
worktree merge.

## Files Created/Modified
- `docs/src/appendix/doc-coverage-report.md` — ADR-0047 banner; "no warnings" claim and 9-crate
  list reframed as historical
- `docs/src/appendix/user-rest-api.md` — ADR-0047 banner; malformed tail fenced as inert text so
  the page renders
- `docs/src/appendix/user-system.md` — ADR-0047 banner (shares MB-59/MB-60 wording); clap pin
  left untouched
- `docs/src/appendix/contributing-legacy.md` — ADR-0047 banner; MSRV, workspace layout, and clone
  URL corrected
- `docs/src/appendix/build-baselines.md` — ADR-0047 banner; crate-count table reframed as snapshot
- `docs/src/appendix/battalion-benchmarks.md` — toolchain line corrected to 1.88
- `docs/src/appendix/sanctum-benchmarks.md` — Qdrant adapter reframed as shipped
- `docs/src/appendix/release-automation.md` — publish job's three dependencies named

## Decisions Made
- `user-rest-api.md`'s malformed tail was fenced verbatim (with a short explanatory note) rather
  than cleaned up or truncated, honoring D-02's "repair only enough to render" and D-04's
  proportionality bound — the content is unchanged, only its rendering is fixed.
- `build-baselines.md`'s toolchain rows (`rustc 1.95.0`) were left as the snapshot's own measured
  values rather than corrected to 1.88, per D-20's historical-table exemption — this is a dated
  measurement of what that snapshot's environment actually ran, not a current MSRV claim.
- `sanctum-benchmarks.md`'s unfilled footer fields (`Last Updated: TBD`) were left as-is per the
  plan's explicit instruction not to invent a value.

## Deviations from Plan

None — plan executed exactly as written. All eight pages closed with the exact commit subjects,
banner shape, and cited corrections the plan specified; no architectural changes, no scope
expansion, no blocking issues requiring a fix outside the plan's own file list.

## Issues Encountered

`mdbook build docs/` initially failed with a missing `mermaid.min.js` error because the worktree's
`docs/` directory lacked the mermaid assets. Ran `mdbook-mermaid install docs/` per the
environment's documented docs-gate order, confirmed `git status --porcelain -- docs` was clean
afterward (no stray diff to restore), then builds succeeded for the remainder of the plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All eight MB-nn rows this plan owned are closed and recorded in the D-23 table above, ready for
  plan 35-10's `35-EVIDENCE.md` roll-up and exit-grep pass.
- No deferred observations: no defect was noticed on a page the audit settled `current` while
  editing a neighbour in this plan.
- `mdbook build docs/`, `./scripts/check-doc-config.sh`, and `./scripts/check-doc-examples.sh` all
  pass on the tree as left by this plan's commits.

## Self-Check: PASSED

- All eight modified files exist at their expected paths (confirmed via `git diff --stat`, `ls`,
  and content greps during execution).
- All eight commit hashes (`59b3dcb1`, `47ca479c`, `3af85fc0`, `2ac3cac0`, `9dbecc08`, `59720e17`,
  `189208f0`, plus the tracer commit `59b3dcb1` already counted) are present in `git log --oneline`
  on this worktree branch (`worktree-agent-a4fc8581a518c5ad8`).
- `git log --oneline --grep 'MB-37'`, `--grep 'MB-54'`, `--grep 'MB-55'`, `--grep 'MB-39'`,
  `--grep 'MB-47'`, `--grep 'MB-59'` each returned at least one matching commit.
- `git diff <pre-task-1>..HEAD --name-only` lists exactly the eight `files_modified` paths and no
  others — `.planning/PROJECT.md`, `src/`, and `crates/` are untouched by this plan's commits.

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*
