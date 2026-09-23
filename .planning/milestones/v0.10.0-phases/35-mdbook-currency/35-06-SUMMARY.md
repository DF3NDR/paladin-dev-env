---
phase: 35-mdbook-currency
plan: 06
subsystem: docs
tags: [mdbook, ci, github-actions, opentelemetry, coverage, currency-audit]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: 35-01's superstep-engine.md and doc-examples groundwork this plan links against
provides:
  - "docs/src/deployment/cicd.md rebuilt on the real ci.yml/release.yml job inventory, with a required-versus-advisory column sourced from protect-main-branch.json"
  - "docs/src/contributing/testing-guide.md's CI section and coverage command corrected to the real workflows and the scripts/coverage.sh invocation"
  - "docs/src/operations/{monitoring,troubleshooting,performance-tuning}.md's three superseded dated callouts rewritten to current truth, no new dated callout added"
affects: [35-10]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Retained YAML/bash excerpts are always verbatim, captioned `# excerpt: <source file>[ — job: <name>]`, never re-typed from memory"
    - "A CI/CD page states required-vs-advisory per job by reading `.github/rulesets/protect-main-branch.json`'s `required_status_checks` context list, not by guessing from job intent"

key-files:
  created: []
  modified:
    - docs/src/deployment/cicd.md
    - docs/src/contributing/testing-guide.md
    - docs/src/operations/monitoring.md
    - docs/src/operations/performance-tuning.md
    - docs/src/operations/troubleshooting.md

key-decisions:
  - "Kept the ci.yml job table exhaustive (27 jobs) rather than only the acceptance-criteria minimum subset, since D-15's backstop truth is 'no invented job name and no invented workflow filename survives' — a partial table invites the next audit to re-flag the omitted jobs as missing."
  - "Release.yml's table has no required-or-advisory column: none of its jobs appear in protect-main-branch.json's required_status_checks (they run on tag push, not PR-to-main), so a required/advisory column would be a fabricated data point, not a sourced one."
  - "Rephrased away from the literal substrings 'build-release' and 'actions-rs/toolchain' when describing what was previously fabricated, since the plan's own verification greps for zero occurrences of those strings anywhere on the page (including in a sentence explaining they were removed)."

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "docs/src/deployment/cicd.md replaces the fabricated 3-job ci.yml YAML sample and the build-release/create-release release.yml sample with real job tables (27 ci.yml jobs, 9 release.yml jobs), adds codeql.yml to the Workflow Structure listing, and states the CodeQL advisory-only disposition verbatim from security.instructions.md"
    requirement: CURR-06
    verification:
      - kind: other
        ref: "mdbook build docs/ (No broken links found) + ./scripts/check-doc-config.sh (151 YAML blocks, 0 failed) + task verify grep set (codeql.yml, verify-tag-source, check-release-consistency, fail-under-lines present; build-release absent; actionlint/api-surface/osv-scanner/crate-isolation/publish-dry-run present)"
        status: pass
    human_judgment: false
  - id: D2
    description: "docs/src/contributing/testing-guide.md's CI Integration section replaces the fabricated test.yml/actions-rs/toolchain sample with a pointer to cicd.md's job table plus a verbatim coverage-job excerpt; the coverage command is corrected to scripts/coverage.sh's real integration-tests,llm-all + --fail-under-lines 82 invocation"
    requirement: CURR-07
    verification:
      - kind: other
        ref: "mdbook build docs/ + ./scripts/check-doc-config.sh + task verify grep set (scripts/coverage.sh, integration-tests,llm-all, fail-under-lines present; workflows/test.yml and actions-rs/toolchain absent)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The three superseded dated callouts on monitoring.md, troubleshooting.md and performance-tuning.md are rewritten to current truth (OpenTelemetry is now a real optional dependency behind otel; engine_benchmarks.rs now exists) with no new dated callout added anywhere"
    requirement: CURR-08
    verification:
      - kind: other
        ref: "mdbook build docs/ + ./scripts/check-doc-config.sh + task verify grep set (otel/observability.md present, opentelemetry_jaeger/tracing_opentelemetry absent on monitoring.md; engine_benchmarks.rs present on performance-tuning.md; 'Corrected 2026-09' absent from all three files)"
        status: pass
    human_judgment: false

duration: 22min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 06: CI/Operations Currency Correction Summary

**Rebuilt `cicd.md` and `testing-guide.md`'s CI sections on the real `ci.yml`/`release.yml` job
inventory (with a ruleset-sourced required-vs-advisory column), and retired three superseded
dated callouts on the Operations pages without leaving a new one behind.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-09-17T14:03:00Z (approx, first file read)
- **Completed:** 2026-09-17T14:25:00Z
- **Tasks:** 3 (5 commits — Task 3 is three commits, one per page)
- **Files modified:** 5

## Accomplishments
- `docs/src/deployment/cicd.md`: added `codeql.yml` to the Workflow Structure listing; replaced
  the fabricated 3-job `ci.yml` YAML sample with a 27-row job table (Job / Display name / What it
  gates / Required-or-advisory, the last column read from
  `.github/rulesets/protect-main-branch.json`); replaced the fabricated `build-release`/
  `create-release` `release.yml` sample with a 9-row real job table naming `verify-tag-source`
  explicitly as the GitFlow tag-source guard; added a `codeql.yml` section stating the
  advisory-only disposition verbatim from `security.instructions.md`; kept two verbatim captioned
  excerpts (the `coverage` job's `Measure coverage` step, and `scripts/coverage.sh`'s
  `--fail-under-lines` invocation).
- `docs/src/contributing/testing-guide.md`: replaced the fabricated `test.yml`/
  `actions-rs/toolchain` sample with a pointer to `cicd.md`'s job table plus the same verbatim
  `coverage` job excerpt; corrected both places the coverage command was shown (the "Local
  generation" full invocation and "The scope" explanation) to `scripts/coverage.sh`'s real
  `--features integration-tests,llm-all --fail-under-lines 82` invocation, with the one-sentence
  explanation of why `llm-all` matters (default feature set builds 3 of 9 shipped LLM adapters).
- `docs/src/operations/monitoring.md` (MB-32): rewrote the Overview premise to state which half
  still holds (no `prometheus` dependency, no `/metrics` route) and which no longer does
  (`opentelemetry` is now a real optional dependency behind `otel`); replaced the fabricated
  Jaeger/`opentelemetry_jaeger` sample with a section naming the shipped `OtelTraceSink`, the
  `otel` feature, and a link to `operations/observability.md`; left the Health Checks callout
  untouched (audit re-verified it).
- `docs/src/operations/troubleshooting.md` (MB-34): corrected the same repeated
  dependency-absence premise while keeping every conclusion the audit confirmed still holds (no
  `/metrics` route, the fabricated port claim, the real Dockerfile ports, no `logging:` key).
- `docs/src/operations/performance-tuning.md` (MB-33): rewrote the Benchmark Results callout to
  name both real benchmark files (`config_benchmarks.rs` and `engine_benchmarks.rs`), kept the
  still-accurate part about the four never-fixed drafted benchmark files, and linked the new
  [Superstep Engine guide](../user-guides/superstep-engine.md).
- Retired the "Corrected 2026-08-24" dated-callout style on all three Operations pages per D-16 —
  every rewritten passage states current truth in plain prose, no new dated marker added anywhere
  in this plan's five files.

## Task Commits

Each task was committed atomically:

1. **Task 1: MB-31 — cicd.md rebuilt on the real workflow inventory, end to end** - `ea34a45c` (docs)
2. **Task 2: MB-36 — testing-guide.md's CI section and coverage command** - `2bb7c833` (docs)
3. **Task 3a: MB-32 — monitoring.md tracing premise** - `69f3336f` (docs)
4. **Task 3b: MB-34 — troubleshooting.md tracing premise** - `8c31b557` (docs)
5. **Task 3c: MB-33 — performance-tuning.md engine benchmark** - `e5d16af8` (docs)

_Note: Task 1 is `type="tracer"` — its own `<verify>` grep set was re-run immediately after
commit and passed before Task 2 began (autonomous-run tracer feedback gate)._

## Files Created/Modified
- `docs/src/deployment/cicd.md` - Real `ci.yml`/`release.yml` job tables, `codeql.yml` entry, advisory-only CodeQL disposition
- `docs/src/contributing/testing-guide.md` - Real CI job pointer + corrected coverage invocation
- `docs/src/operations/monitoring.md` - Corrected OpenTelemetry/otel premise, shipped OTLP sink section
- `docs/src/operations/troubleshooting.md` - Corrected repeated tracing-dependency premise, conclusions kept
- `docs/src/operations/performance-tuning.md` - Named `engine_benchmarks.rs` alongside `config_benchmarks.rs`

## Decisions Made
- Built the full 27-row `ci.yml` job table rather than only the acceptance-criteria minimum,
  since D-15's backstop truth ("no invented job name and no invented workflow filename survives")
  reads as exhaustive, not sample-only — a partial table would leave real jobs unaccounted for and
  invite the next audit pass to re-flag them.
- `release.yml`'s table carries no required-or-advisory column: none of its 9 jobs appear in
  `protect-main-branch.json`'s `required_status_checks` (they trigger on tag push, not PR-to-main),
  so a required/advisory value there would be invented, not sourced.
- Avoided the literal substrings `build-release` and `actions-rs/toolchain` even in prose
  explaining they were removed, after the first draft's explanatory sentence itself tripped the
  plan's own `grep -c` verification (which checks for zero occurrences anywhere on the page, not
  just in a fenced code block).

## Deviations from Plan

None - plan executed exactly as written. Two self-inflicted near-misses were caught and fixed
before commit (see Decisions Made above) rather than being deviations from the plan itself — the
plan's verify greps did exactly what they were designed to do.

## Deferred observations

- `docs/src/contributing/testing-guide.md` line 96's `tests/` directory-structure ASCII tree
  lists `fixtures/config.test.yml`, which trips the phase-level D-21 backstop grep
  (`grep -rn 'test\.yml\|build-release' docs/src/deployment/cicd.md
  docs/src/contributing/testing-guide.md`) as a false positive — it is a real, pre-existing
  fixture-file reference (`config.test.yml` exists at repo root, not under `tests/fixtures/`,
  which is itself a separate, pre-existing minor inaccuracy in that ASCII tree), unrelated to any
  fabricated CI workflow name, and it predates this plan's changes (confirmed via `git show
  ea34a45c~1`). Out of scope for MB-36's task instructions (which named only lines 628-686 and the
  coverage section). Flagging for 35-10 to fold into `deferred-items.md` — the tree's own claim
  that the fixture lives at `tests/fixtures/config.test.yml` rather than the repo root is worth a
  follow-up correction, separate from this plan's CI/coverage scope.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- MB-31, MB-36, MB-32, MB-33 and MB-34 are all closed; CURR-06/07/08 requirements satisfied.
- `docs/src/deployment/cicd.md` and `docs/src/contributing/testing-guide.md` now agree with each
  other and with the live workflow files — a later plan touching either page has a verified
  baseline to diff against instead of re-deriving the job inventory from scratch.
- The one deferred observation above (a stale `tests/fixtures/` path claim, unrelated to this
  plan's scope) is ready for 35-10's `deferred-items.md` sweep.

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: docs/src/deployment/cicd.md
- FOUND: docs/src/contributing/testing-guide.md
- FOUND: docs/src/operations/monitoring.md
- FOUND: docs/src/operations/troubleshooting.md
- FOUND: docs/src/operations/performance-tuning.md
- FOUND commit: ea34a45c
- FOUND commit: 2bb7c833
- FOUND commit: 69f3336f
- FOUND commit: 8c31b557
- FOUND commit: e5d16af8
