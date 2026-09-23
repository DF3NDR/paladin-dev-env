---
phase: 35-mdbook-currency
plan: 10
subsystem: docs
tags: [mdbook, changelog, evidence, closure-table, exit-greps, phase-close]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: "All eight wave-1/wave-2 plans' (35-01 through 35-09) closed MB-nn rows and their SUMMARY.md closure tables/deferred observations, merged to the phase branch before this plan started"
provides:
  - "35-EVIDENCE.md — the consolidated sixty-row MB-nn closure table, the full docs.yml gate run (twice — an initial capture and a final-run re-verification), make api-surface, and all seven D-21 exit greps with a fully-reasoned allowlist"
  - "CHANGELOG.md [0.10.0] ### Documentation subsection, placed after ### Fixed and before ### Known limitations, with zero MB-nn identifiers"
  - "deferred-items.md completed: every plan's ## Deferred observations section folded in, with 35-04's adr-index.md occurrence marked resolved and three still-open items given a proposed classification and owner"
  - "Five residual D-21 defects (MSRV samples on four appendix pages, a fabricated CI/CD sample on cli-testing.md) fixed as Rule-1 deviations, follow-ups to MB-53/MB-50/MB-46/MB-44/MB-45"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "NN-EVIDENCE.md house pattern: Measurement Header, gate captures verbatim, exit-grep allowlist table with file:line/matched-text/reason columns, closure table, then a final-run section re-verifying the same gate on the plan's own last commit"

key-files:
  created:
    - .planning/phases/35-mdbook-currency/35-EVIDENCE.md
    - .planning/phases/35-mdbook-currency/35-10-SUMMARY.md
  modified:
    - CHANGELOG.md
    - .planning/phases/35-mdbook-currency/deferred-items.md
    - docs/src/appendix/cli-setup-check.md
    - docs/src/appendix/cli-testing.md
    - docs/src/appendix/cli-usage.md
    - docs/src/appendix/minio-file-repository-setup.md
    - docs/src/appendix/redis-queue-adapter-setup.md

key-decisions:
  - "Fixed the five residual D-21 defects the orchestrator's pre-flight found (four MSRV-sample one-liners plus cli-testing.md's fabricated .github/workflows/test.yml CI/CD Integration block) as Task 1 Rule-1 deviations, before running the evidence-capturing gate — these are follow-ups to MB-53/MB-50/MB-46/MB-44 (already-closed rows whose own pages carried one more stale line each) and MB-45 (cli-testing.md's own row, closed by plan 35-07, carried this separate stale block that plan's own scope did not reach)."
  - "cli-testing.md's CI/CD Integration section was rewritten per D-15's pattern — a pointer to cicd.md's job table plus a verbatim captioned excerpt of the real cli-tests job from ci.yml — rather than simply deleting the section, since the page's own surrounding prose (cargo insta review/accept/reject) still needed a correct CI cross-reference."
  - "The allowlist table's docker-compose.test.yml/config.test.yml rows were verified against the actual filesystem (both files confirmed present on disk) before being allowlisted, not assumed from the SUMMARY text alone."
  - "35-04's adr-index.md Quartermaster deferred observation was folded into the register as CLOSED, not open — confirmed live (grep empty, line 14 reworded) and attributed to the orchestrator's own 64a44c51 cross-plan integration commit, which landed after wave 2 merged and before this plan started; no WINDOWS.md row existed to resolve (35-04 never committed one)."

requirements-completed: [CURR-06, CURR-07, CURR-08, CURR-09, CURR-10]

coverage:
  - id: D1
    description: "35-EVIDENCE.md exists with the consolidated 60-row closure table (all MB-01..MB-60 present, each reconciled against 34-AUDIT.md §5 with no disposition disagreement), the full docs.yml gate sequence run twice (initial + final-run), make api-surface, and all seven D-21 checks with a fully-reasoned allowlist"
    requirement: "CURR-07"
    verification:
      - kind: other
        ref: "for n in $(seq -w 1 60); do grep -q \"MB-$n\" 35-EVIDENCE.md; done -- all 60 present"
        status: pass
      - kind: other
        ref: "grep -q 'No broken links found' 35-EVIDENCE.md; grep -q 'api-surface' 35-EVIDENCE.md"
        status: pass
      - kind: other
        ref: "mdbook build docs/; ./scripts/check-doc-examples.sh; ./scripts/check-doc-config.sh; make api-surface -- all exit 0, run live this session"
        status: pass
    human_judgment: false
  - id: D2
    description: "CHANGELOG.md [0.10.0] carries a ### Documentation subsection after ### Fixed and before ### Known limitations, with a bullet for the engine guide, one per nav section, and one naming the archived appendix pages; zero MB-nn identifiers anywhere in the file"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "awk nav-order check: ### Fixed (383) < ### Documentation (421) < ### Known limitations (459)"
        status: pass
      - kind: other
        ref: "grep -cE '\\bMB-[0-9]{2}\\b' CHANGELOG.md == 0"
        status: pass
      - kind: other
        ref: "git diff HEAD~1 --name-only == CHANGELOG.md only; git log --oneline -1 -- CHANGELOG.md starts with docs(35):"
        status: pass
    human_judgment: false
  - id: D3
    description: "deferred-items.md holds one row per ## Deferred observations entry across the nine plan SUMMARYs (or states none was recorded); no Phase 34 register entry absorbed or renumbered; 35-EVIDENCE.md's final-run section is the last recorded gate run, taken after the last content commit"
    requirement: "CURR-09"
    verification:
      - kind: other
        ref: "grep -q Owner deferred-items.md; grep -qi final 35-EVIDENCE.md"
        status: pass
      - kind: other
        ref: "mdbook-mermaid install docs/; git status --porcelain -- docs empty (D-00e); mdbook build docs/ No broken links found; both scripts + make api-surface exit 0"
        status: pass
      - kind: other
        ref: "grep -rniE Quartermaster docs/src empty; grep -rn paladin::paladin_ports:: docs/src empty; grep -rn paladin::infrastructure::adapters::llm:: docs/src -- allowlisted only"
        status: pass
    human_judgment: false
  - id: D4
    description: "All sixty MB-nn identifiers reproduce mechanically in git log --oneline --grep 'MB-' (D-00a); .planning/PROJECT.md untouched throughout the phase (D-04)"
    requirement: "CURR-10"
    verification:
      - kind: other
        ref: "for n in $(seq -w 1 60); do git log --oneline --grep \"MB-$n\" | wc -l; done -- all >= 1 (ALL 60 PRESENT)"
        status: pass
      - kind: other
        ref: "git diff --name-only 81ddddd9..HEAD -- .planning/PROJECT.md -- empty"
        status: pass
    human_judgment: false

# Metrics
duration: ~50min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 10: Close the Phase — Evidence, Changelog, Deferred Register Summary

**Wrote `35-EVIDENCE.md`'s sixty-row closure table and twice-run gate proof, added `CHANGELOG.md`'s `[0.10.0] ### Documentation` entry, folded every wave plan's deferred observations into the phase-local register, and fixed five residual stale-MSRV/fabricated-CI-sample defects the wave-2 pre-flight had surfaced.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-09-17T14:30:00Z (approx)
- **Completed:** 2026-09-17T14:44:00Z
- **Tasks:** 3
- **Files modified:** 8 (2 created, 6 modified)

## Accomplishments

- **Task 1:** Fixed five residual D-21 defects (four one-line MSRV-sample corrections on
  `redis-queue-adapter-setup.md`, `minio-file-repository-setup.md`, `cli-usage.md`,
  `cli-setup-check.md`; the fabricated `.github/workflows/test.yml` block on `cli-testing.md`
  rewritten to a real-`cli-tests`-job pointer/excerpt per D-15), then ran the full `docs.yml` gate
  sequence, `make api-surface`, all seven D-21 exit greps, and reconciled `git log --oneline
  --grep 'MB-'` against `34-AUDIT.md` §5 — all sixty IDs present, no disposition disagreement.
  Wrote `35-EVIDENCE.md` with the Measurement Header, every gate capture verbatim, the consolidated
  sixty-row closure table, and the fully-reasoned exit-grep allowlist.
- **Task 2:** Added `CHANGELOG.md`'s `[0.10.0]` `### Documentation` subsection (positioned after
  `### Fixed`, before `### Known limitations`, matching the `[0.5.0]` precedent's tone): one bullet
  for the new superstep-engine guide, one per nav section (Getting Started/API Reference, User
  Guides/Architecture, Deployment/Operations, Contributing, Appendix), and one naming the five
  archived appendix pages and their live replacements. Zero `MB-nn` identifiers anywhere in the
  file.
- **Task 3:** Folded every plan's `## Deferred observations` SUMMARY section into
  `deferred-items.md` — four plans (35-02, 35-03, 35-05, 35-08) recorded none; 35-04's
  `adr-index.md` occurrence closed (resolved by the orchestrator's `64a44c51` before this plan
  began); three still-open items from 35-06 (`testing-guide.md`'s stale fixture-path claim),
  35-07 (`cli-configuration.md`'s Garrison/Arsenal troubleshooting entries) and 35-09
  (`battalion-patterns-guide.md`'s body-content `OpenAiAdapter` casing drift, also present on five
  other pages) each recorded with a proposed classification and an unassigned owner. Re-ran the
  full gate on the plan's own final commit and appended a final-run section to `35-EVIDENCE.md`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Run the full gate and write 35-EVIDENCE.md with the sixty-row closure table** —
   `75626bc3` (docs) — includes the five residual-defect page fixes plus the new evidence file
2. **Task 2: Write the CHANGELOG [0.10.0] Documentation subsection** — `f1418596` (docs)
3. **Task 3: Fold the deferred observations and re-run the gate on the final commit** —
   `7ce9a1b2` (docs)

**Plan metadata:** committed by the orchestrator after wave merge (worktree mode — this executor
does not write STATE.md/ROADMAP.md/REQUIREMENTS.md).

## Files Created/Modified

- `.planning/phases/35-mdbook-currency/35-EVIDENCE.md` — new: Measurement Header, gate captures
  (initial + final-run), sixty-row closure table, seven-check exit-grep allowlist
- `CHANGELOG.md` — new `[0.10.0]` `### Documentation` subsection
- `.planning/phases/35-mdbook-currency/deferred-items.md` — nine plans' deferred observations
  folded in
- `docs/src/appendix/redis-queue-adapter-setup.md` — "Rust 1.75" → "Rust 1.88"
- `docs/src/appendix/minio-file-repository-setup.md` — "Rust 1.75" → "Rust 1.88"
- `docs/src/appendix/cli-usage.md` — sample toolchain output `1.75.0` → `1.88.0`
- `docs/src/appendix/cli-setup-check.md` — four sample toolchain-output lines corrected to 1.88.0
- `docs/src/appendix/cli-testing.md` — fabricated `test.yml` CI/CD sample replaced with a real
  `cli-tests`-job pointer/excerpt

## Decisions Made

See `key-decisions` in frontmatter. In short: the five residual D-21 defects were fixed as
Task 1 Rule-1 deviations (follow-ups to already-closed MB rows whose pages carried one more stale
line each, or a stale block plan 35-07's own MB-45 scope did not reach) before the evidence-capturing
gate ran, so the evidence file records a genuinely clean state rather than a state with known,
unfixed residuals papered over by an allowlist entry that lacked a real historical reason.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Four appendix pages still showed the retired MSRV (1.75) in prose or sample output**
- **Found during:** Task 1, honoring the orchestrator's `<orchestrator_state_of_the_tree>` pre-flight
  grep results (explicitly authorized as Rule-1 follow-ups to MB-53/MB-50/MB-46/MB-44)
- **Issue:** `redis-queue-adapter-setup.md:11` and `minio-file-repository-setup.md:22` stated
  "Rust 1.75 or later"; `cli-usage.md:227` and four lines on `cli-setup-check.md` (58, 72, 218,
  258) showed sample `setup-check`/toolchain output reading `1.75.0` — all stale against the
  workspace's actual MSRV (1.88, `Cargo.toml` `rust-version`).
- **Fix:** Corrected all five occurrences to 1.88 (prose) or `1.88.0`/`rustc 1.88.0 (6b00bc388
  2025-06-23)` (sample output, keeping the illustrative build-metadata format).
- **Files modified:** `docs/src/appendix/redis-queue-adapter-setup.md`,
  `docs/src/appendix/minio-file-repository-setup.md`, `docs/src/appendix/cli-usage.md`,
  `docs/src/appendix/cli-setup-check.md`
- **Verification:** `grep -rnE '\b1\.(70|75|85)(\.[0-9]+)?\b' docs/src` returns empty (was 7 hits
  before this fix, per the orchestrator's pre-flight).
- **Committed in:** `75626bc3` (Task 1 commit)

**2. [Rule 1 - Bug] `cli-testing.md` carried a fabricated `.github/workflows/test.yml` CI/CD sample**
- **Found during:** Task 1, honoring the orchestrator's pre-flight grep result (authorized as a
  Rule-1 follow-up to MB-45, `cli-testing.md`'s own already-closed row)
- **Issue:** The "CI/CD Integration" section showed a `# .github/workflows/test.yml` YAML block
  with `NO_COLOR=1 cargo test --test cli` and `cargo insta test --test cli --check` steps — that
  workflow file does not exist; the real CLI snapshot-test job is `cli-tests` in `ci.yml`.
- **Fix:** Replaced the block with a prose pointer to `deployment/cicd.md`'s job table plus a
  verbatim, captioned excerpt (`# excerpt: .github/workflows/ci.yml — job: cli-tests`) of the real
  job's snapshot-test step, and corrected the trailing note to describe the job's actual
  zero-tests-executed guard rather than an `insta`-pending-snapshot check.
- **Files modified:** `docs/src/appendix/cli-testing.md`
- **Verification:** `grep -rn 'test\.yml\|build-release' docs/src` no longer matches this page;
  `mdbook build docs/` — No broken links found.
- **Committed in:** `75626bc3` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1, both orchestrator-authorized follow-ups to
already-closed MB rows on pages this plan's own scope reaches — `35-EVIDENCE.md` and its exit
greps)
**Impact on plan:** Necessary so the evidence file's D-21 exit-grep section records a genuinely
clean state instead of five pre-existing residuals. No scope creep — all five fixes are one line
or one self-contained block, on pages already named in the orchestrator's pre-flight authorization.

## Issues Encountered

None beyond the five residual defects handled above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All sixty `MB-nn` rows are closed and reconciled; `35-EVIDENCE.md` records the full, twice-run
  gate proof; `CHANGELOG.md` carries the `[0.10.0]` Documentation entry; `deferred-items.md` is
  complete.
- Phase 35 is ready for `/gsd-verify-work 35`.
- Three items remain genuinely open in `deferred-items.md` for a future pass (none blocking): the
  `testing-guide.md` fixture-path ASCII-tree inaccuracy, `cli-configuration.md`'s
  Garrison/Arsenal troubleshooting entries' unverified TODO claims, and the systemic
  `OpenAiAdapter` casing bug across six pages.

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: `.planning/phases/35-mdbook-currency/35-EVIDENCE.md`
- FOUND: `CHANGELOG.md` (`[0.10.0]` `### Documentation` heading present, `grep -cE '\bMB-[0-9]{2}\b'` == 0)
- FOUND: `.planning/phases/35-mdbook-currency/deferred-items.md` (Owner column present, all nine plans accounted for)
- FOUND: all five residual-defect page fixes on disk
- Commit `75626bc3` — FOUND in `git log --oneline`
- Commit `f1418596` — FOUND in `git log --oneline`
- Commit `7ce9a1b2` — FOUND in `git log --oneline`
