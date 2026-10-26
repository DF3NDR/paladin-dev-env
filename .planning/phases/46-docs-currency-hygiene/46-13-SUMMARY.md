---
phase: 46-docs-currency-hygiene
plan: 13
subsystem: docs-upgrade-entry-and-bookkeeping
tags: [upgrading, migration-guide, changelog, switching-providers, nyquist, state, curr-23, curr-24, d-03, d-05, d-17, d-25]

requires:
  - phase: 46-11
    provides: frozen MIGRATION.md section 10 (10.1 rows, 10.8 operator checklist and library-consumer DR table), freeze commit fc48c010
  - phase: 46-12
    provides: CHANGELOG [Unreleased] citations already pointing at section 10
  - phase: 46-14
    provides: retargeted DR-19 line on the provider page
  - phase: 46-02..46-04
    provides: the eight v0.10.0 VALIDATION.md files at status validated with nyquist_compliant true
  - phase: 46-05
    provides: the check-dependency-allowlist gate named in the CHANGELOG Changed bullet
  - phase: 46-07
    provides: the Treasurer page and configuration reference named in the CHANGELOG Documentation bullet
provides:
  - "Book Upgrading page leading with v0.10.1 to v0.11.0 (condensed M-B-05..M-B-22 table, DR-1..DR-26 index by topic, six-step operator checklist), linking MIGRATION.md section 10 anchors"
  - "Migration Guide pointer line to the v0.11.0 record"
  - "Dated, attributed DeepSeek listing note (G-4) on the Switching providers page"
  - "CHANGELOG [Unreleased] ### Documentation subsection and a Changed bullet for the dependency-allowlist gate"
  - "ROADMAP dependency note, STATE.md Nyquist outcome and open-WINDOWS-rows bullet, web3sec todo dated update"
affects: [47]

tech-stack:
  added: []
  patterns:
    - "Book page as an orientation over the register: rows named by id, reference-style links to MIGRATION.md anchors, authoritative text left in MIGRATION.md"

key-files:
  created: []
  modified:
    - docs/src/api-reference/upgrading.md
    - docs/src/api-reference/migration-guide.md
    - docs/src/user-guides/switching-providers.md
    - CHANGELOG.md
    - .planning/ROADMAP.md
    - .planning/STATE.md
    - .planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md

key-decisions:
  - "DR-7 carries the pacing/decorator topic line (RateLimitExceeded, Cadence composition, decorator forwarding); DR-21 stays under reasoning effort, so each DR id appears once in the consumer index"
  - "The v0.9.x to v0.10.0 sections moved under ### headings with text unchanged; a short orientation paragraph replaces the old intro and links the section 9 anchors"
  - "The CHANGELOG Documentation bullets cite MIGRATION.md section 10 from the start (46-12 request)"
  - "docs/src/SUMMARY.md untouched (D-05): the guide is reached through the existing Upgrading and Migration Guide entries"

requirements-completed: [CURR-23, CURR-24]

duration: 17 min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 13: Upgrading page entry, G-4 note, CHANGELOG docs and planning bookkeeping Summary

The book's Upgrading page now opens with v0.10.1 to v0.11.0 (a condensed M-B-05..M-B-22 table, the DR-1..DR-26 consumer index by topic and the six-step operator checklist, each linking `MIGRATION.md` section 10 by anchor), the provider page carries the consumer's dated DeepSeek listing finding, the release note lists the phase's docs, and ROADMAP, STATE and the web3sec todo record the outcome and the hand-offs.

## Accomplishments

- **Tracer (commit `0be565b3`):** `docs/src/api-reference/upgrading.md` leads with `## Upgrading from v0.10.1 to v0.11.0` and three subsections: `### Behavioral changes` (18 rows, M-B-05 to M-B-22, each id once), `### Library consumers` (26 DR ids, each once, grouped under Battalion and Aegis, the `Entity<T>` rename, provider configuration and construction, token usage cost and spend, reasoning effort, rate pacing and decorators), `### Operator checklist` (the six 10.8 steps, one line each). Links use reference definitions to `MIGRATION.md#101-behavioral-changes-user-visible-without-code-changes` and `#108-upgrade-checklist`. The v0.9.x to v0.10.0 content sits below as `## Upgrading from v0.9.x to v0.10.0` with its four former sections as `###` subsections, text unchanged (the `#92-...` links stay). `## Full migration record` names both parts and links `#10-upgrading-from-v0101-to-v0110` and `#9-upgrading-from-v090-to-v0100`. `migration-guide.md` gains the pointer line under its H1 intro and names v0.11.0 as the release the Upgrading page leads with. The tracer verify passed before expansion ("Tracer verified end-to-end -- expanding") and the cheap legs were re-run on the committed tree (`docs/src/SUMMARY.md` unchanged from the plan base).
- **Task 2 (commit `976f2554`):** `switching-providers.md` gains, before "Probed but not seeded", the dated and attributed note: as of 2026-10-24, from the downstream consumer's live `/models` check and not re-run by Paladin, DeepSeek lists `deepseek-flash` (seeded) and does not list `deepseek-v4-flash` (unseeded, adapter-wide fallback, one warning per model). CHANGELOG `[Unreleased]` gains `### Documentation` (after `### Fixed`; four bullets: Treasurer page and configuration reference, two-part `MIGRATION.md` with section 10, Upgrading page, DeepSeek note) and one `### Changed` bullet for `check-dependency-allowlist` (ADR-0015, D-21). Only two hunks in `CHANGELOG.md`; nothing below `[Unreleased]` changed.
- **Task 3 (commit `16293785`):** ROADMAP's Phase 46 `**Depends on**` paragraph carries the dated satisfied-by note. STATE.md: the "Nyquist validation" concerns bullet is struck through and rewritten to the outcome (05-21 clause kept open), a new "Open WINDOWS rows at Phase 46 close" concerns bullet is added, and the v0.10.0-close deferred row reads "Reconciled by Phase 46 ... 22 true, 24 true, 29 true, 30 true, 34 true, 36 true, 36.1 true, all `status: validated`; 28 promoted (D-15)". `git diff -U0 .planning/STATE.md` touches only those three places; the 05-17, 18-21 and 1-4 rows are unchanged. The web3sec todo stays in `pending/`, its `files:` entry names section 10.8, and it gains `## Update 2026-10-24 (Phase 46)` citing `46-FEEDBACK-web3sec.md`, the folded G-1..G-7 and muster gaps (plans 46-11, 46-06) and freeze commit `fc48c010`; the re-pin on the `v0.11.0` tag, the consumer's gates and the return report are left to web3sec after Phase 47 (D-25).

## Open WINDOWS rows at phase end (for /gsd-ship in Phase 47)

`windows status` at close: `open_count` 13 (waived 39, fixed 30, total 82), the same 13 rows as at planning: **56, 58, 62, 63, 64, 65, 66, 67, 68, 69, 73, 77, 79**. None is a Phase 46 row (46-05's rows 80-82 are waived or fixed). Phase 47 dispositions them or records an override; STATE.md carries one concerns bullet saying so (T-46-28).

## Task Commits

1. Task 1 (tracer): `0be565b3` docs(46-13): the book's Upgrading page leads with v0.10.1 to v0.11.0 (D-05)
2. Task 2: `976f2554` docs(46-13): DeepSeek listing note; CHANGELOG documentation entries for Phase 46 (D-03)
3. Task 3: `16293785` docs(46-13): ROADMAP dependency note, STATE Nyquist outcome, web3sec todo update (D-03, D-17, D-25)

## Deviations from Plan

### Auto-fixed Issues

None.

### Record notes

**1. [Record] The Task 3 commit lacks the two attribution trailers**
- **Found during:** Task 3 commit review.
- **Issue:** the plan prescribes the `gsd-tools query commit` helper for Task 3, and the message string passed to it did not carry the `Co-Authored-By` / `Claude-Session` trailer lines the run instructions require; the first two task commits and the SUMMARY commit carry them. Commits cannot be amended in this run.
- **Impact:** none on content; the trailers on `16293785` are missing.

**2. [Wording] The CHANGELOG gate bullet says "a step in `ci.yml`"** rather than naming a job, because the step sits among other gate steps and the plan names only the step's title.

**Total deviations:** 0 auto-fixed, 2 record notes. **Impact:** none on scope.

## Verification

- Task 1, 2 and 3 verify commands exit 0. `make check-doc-config` (161 YAML blocks, 0 failed), `make check-doc-examples`, `make check-api-examples` and `make check-gates` (includes `check-migration-allowlist` and the ADR-0015 allowlist gate: "paladin-core (14) and paladin-ports (12) [dependencies] match ADR-0015's measured sets") exit 0. `make check-changelogs` passes. `cargo test --offline --test legacy_removal_guard --test entity_rename_guard --test treasurer_vocabulary_guard` passes (8 + 3 + 3), run for Tasks 1 and 2 (the offline cache was sufficient; no online fallback was needed).
- F4 completeness: this plan adds no live `9.x` citation. The only 9.x text added is on `upgrading.md` inside the v0.9.x to v0.10.0 section (the page is a history file for the legacy and entity guards) and names the v0.10 record.
- Rows and ids: every M-B-05..M-B-22 and DR-1..DR-26 id appears exactly once in the page (asserted by script before the tracer commit).
- Not run here, left to phase verification: `make clean-code`, `make api-surface`, `make check-consumer-surface` and the two `cargo test -p` battalion legs (no `.rs` file changed in this plan), `make security` (pre-existing RUSTSEC-2026-0386 on tokio, unrelated).

### CI / post-merge checks (mdbook is not installed here)

- The `docs` workflow's mdbook build and `mdbook-linkcheck` render `upgrading.md` and `migration-guide.md`; the reference-style links (`[mig-101]`, `[mig-108]`) and the duplicate `### Behavioral changes` heading (one under each version) need that render to confirm.
- GitHub anchor slugs for section 10 (`#10-upgrading-from-v0101-to-v0110`, `#101-...`, `#108-upgrade-checklist`) are confirmed when `MIGRATION.md` renders on `main` (RESEARCH A1).
- CI `coverage` (82 percent, ADR-0006), `semver`, `docs` and `consumer-surface` jobs before `/gsd-verify-work`.

## Known Stubs

None.

## Threat Flags

None. T-46-26 mitigated (the page names rows by id and links them; the authoritative text stays in `MIGRATION.md`). T-46-27 mitigated (the G-4 note is dated and attributed to the consumer's live check, not presented as verified by Paladin). T-46-28 mitigated (STATE.md lists the 13 open WINDOWS ids).

## Next Phase Readiness

Plan 13 is the last plan of Phase 46. The orchestrator runs phase verification and completion; Phase 47 (the `v0.11.0` release) dispositions the 13 open WINDOWS rows and re-reads `MIGRATION.md` section 10 only if a code change after `e28e43b5` moves the consumer surface.

## Self-Check: PASSED

- Commits found: `0be565b3`, `976f2554`, `16293785`.
- Files exist: `upgrading.md` (contains `## Upgrading from v0.10.1 to v0.11.0`), `migration-guide.md`, `switching-providers.md` (contains `deepseek-v4-flash`), `CHANGELOG.md` (`### Documentation` under `[Unreleased]`), the web3sec todo under `pending/` (none under `completed/`).
