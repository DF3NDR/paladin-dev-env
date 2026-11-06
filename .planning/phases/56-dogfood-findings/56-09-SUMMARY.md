---
phase: 56-dogfood-findings
plan: 09
subsystem: dogfood-findings
tags: [dogfood, coverage-walk, cargo-llvm-cov, testing-guide, findings-log, devcontainer, dogf-05, df-20, df-21, df-22, df-23, df-24, df-25]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-03 coverage runbook (slots C1..C6, LF/LH snippet, traps), 56-08 findings-log state (last id DF-19)"
provides:
  - "56-RUNBOOK-COVERAGE.md slots C1..C6 filled with the maintainer's verbatim output, plus a dated Comparison line (PASS)"
  - "The folded todo 2026-08-13-verify-local-coverage-reproduction closed with a measured figure under .planning/todos/completed/"
  - "DF-20..DF-25 in the findings log; DF-21..DF-25 closed by contained fixes in this plan, DF-20 re-homed to a pending todo"
  - "testing-guide.md Test Coverage section corrected (pinned install, devcontainer notes, no-percentage fix with the LF/LH snippet)"
affects: [Stage C, 56-10, SHIP-10]

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-09-SUMMARY.md
    - .planning/todos/pending/2026-11-03-devcontainer-install-cargo-llvm-cov.md
  modified:
    - .planning/phases/56-dogfood-findings/56-RUNBOOK-COVERAGE.md
    - .project/v0.11.1/02-dogfood-findings-log.md
    - docs/src/contributing/testing-guide.md
    - Makefile
    - .github/workflows/ci.yml
    - .planning/todos/completed/2026-08-13-verify-local-coverage-reproduction.md

key-decisions:
  - "PASS under D-17 / PD-09-1: local 92.79 percent and CI 92.86 percent both truncate to 92 and both are >= 82, so the todo was moved to completed/ (P8 satisfied: the local figure is the maintainer's C5 line, not copied from CI)"
  - "The same-commit basis is the Markdown-only delta 21dacf0d6 -> d9a48cfc6, because 21dacf0d6's own CI run (41695288977) was cancelled by the next push; the agent re-ran git diff --stat 21dacf0d6 d9a48cfc6 -- . ':!*.md' and it is empty"
  - "DF-20 (devcontainer lacks cargo-llvm-cov) is out (re-homed to a pending todo): .devcontainer/Dockerfile.dev is outside this plan's files_modified, so the one-line image fix was not made"
  - "The Makefile coverage-html one-line fix (DF-25) was made from reading the tree, not from a walk step: trap 4 was not run. It is logged as found by Claude and the entry says so"

requirements-completed: [DOGF-05]

duration: ~1h of agent time in this continuation (Task 1 ran earlier on 2026-11-03)
completed: 2026-11-03
---

# Phase 56 Plan 09: Local coverage walk against CI Summary

**The maintainer walked the testing guide's coverage procedure inside the devcontainer and the local figure agrees with CI on the D-17 rule (local 92.79 percent, CI 92.86 percent, both 92 truncated, both above the 82 floor), so the folded 2026-08-13 todo is closed; the walk exposed six guide, CI-label and devcontainer gaps (DF-20..DF-25), five of them fixed here and one re-homed.**

## Performance

- **Tasks:** 3 of 3 (1 tracer, 1 checkpoint:human-action, 1 auto)
- **Commits:** `1e50b92b4` (Task 1), `9dcaaf720` (Task 2: slots placed), `370f36383` (Task 3). The orchestrator's `21dacf0d6` (STATE.md) and `d9a48cfc6` (whitespace fix in three 56-08 files) sit between them.
- **No cargo build was run.** `make coverage` was not run here (it is the maintainer's measurement).

## Checkpoint reply

- **Reply text:** `coverage walk pasted`
- **Date:** 2026-11-03 (the reply time of day was not relayed; the UTC time of this continuation is about 21:40Z)
- **Channel:** relayed through chat. The orchestrator placed the verbatim text, plus the CI figure it read from the job log, in a scratch file; the agent copied each `## Cn` section into the matching `### Cn` slot of `56-RUNBOOK-COVERAGE.md`. Slot validation before placement: C1 holds the 40-hex SHA `21dacf0d63b55e7a5f14d76eba207c107e4889aa`, C3 holds `exit=0`, C5 matches `Lines: <int>/<int> = <float>%  (truncated: <int>)`.
- **Where it ran:** inside the devcontainer (compose siblings `redis` and `rustfs` on `paladin-dev-network`, no Docker socket), not on a bare Docker host.
- **CI figure:** sourced by the orchestrator from the job log of run `41703614636`, job `Coverage` (id `126153522434`, conclusion `success`), because `gh run download` is refused by the sandbox proxy. It is therefore a relayed figure, not one the agent fetched (PD-03-2's fallback).
- **Comparison commit:** `d9a48cfc6`, a Markdown-only delta from the measured `21dacf0d6` (see the pass-rule table).
- No credential appears in the reply, in this SUMMARY, or in any committed file.

## Pass rule (D-17, PD-09-1)

| | Local (C5, maintainer) | CI (job log, relayed) |
|---|---|---|
| Commit | `21dacf0d63b55e7a5f14d76eba207c107e4889aa` | `d9a48cfc6` (run `41703614636`, job `126153522434`, `success`) |
| `Lines:` | `170650/183901 = 92.79%` | `170767/183901 = 92.86%` |
| `int()` of the percentage | 92 | 92 |
| >= 82 | yes | yes |
| `cargo-llvm-cov` | 0.8.7 (hand-installed at CI's pin) | 0.8.7 (`ci.yml` pin) |
| Functions | not computed | `17617/20367 = 86.50%` |

**Verdict: PASS.** Equal whole percents, both at or above 82. The agent re-derived both truncations from the integer `LH/LF` (92.79 and 92.86). The denominators are identical (183901) and the numerators differ by 117 hit lines (0.07 points); the walk did not attribute that. Candidate causes, none verified: hit-line variation between environments under `--test-threads=1`, or a difference between CI's service stack and the devcontainer siblings. Same-commit basis: `git diff --stat 21dacf0d6 d9a48cfc6 -- . ':!*.md'` is empty, so `d9a48cfc6` measured the same code as `21dacf0d6`.

**Trap 2 (the port mismatch 6379/9000 against 6380/9010, and the password-protected compose Redis against the script's unauthenticated probe) is still an unverified hypothesis.** It did not bite in the devcontainer and was not exercised; it is not logged as a finding. Only the devcontainer case is settled (the compose Redis answered `PONG` unauthenticated on `redis:6379`, assumption A3). The bare-host path through `make services-up` is unwalked.

## Findings

Every C6 bullet and bitten trap maps to one entry:

| Source | DF | Severity | Disposition |
|---|---|---|---|
| C1/C6 bullet 1: `cargo-llvm-cov` missing after a devcontainer rebuild | DF-20 | S3 | out (re-homed: `.planning/todos/pending/2026-11-03-devcontainer-install-cargo-llvm-cov.md`) |
| C6 trap 3: guide installs unpinned, CI pins 0.8.7 | DF-21 | S3 | in v0.11.1, fixed in plan 56-09 (guide) |
| C3/C6 bullet 3, trap 1: `make coverage` prints no percentage | DF-22 | S3 | in v0.11.1, fixed in plan 56-09 (guide, LF/LH snippet) |
| C2/C6 bullet 2: `make services-up` not runnable in the devcontainer | DF-23 | S3 | in v0.11.1, fixed in plan 56-09 (guide paragraph) |
| C6 bullet 5, trap 6: stale `Scope:` label | DF-24 | S4 | in v0.11.1, fixed in plan 56-09 (`ci.yml`, one line) |
| Trap 4 (read from the tree; not run in the walk) | DF-25 | S3 | in v0.11.1, fixed in plan 56-09 (`Makefile`, one line) |
| C6 bullet 4: the measured commit's CI run was cancelled by a later push | none | | A process note on the Phase 56 runbook, not a shipped doc or a code defect. Recorded in the Comparison line instead |
| C6 bullet 6: traps 2, 4 and 5 did not bite or were not exercised | none | | Nothing bit; trap 2 stays an open hypothesis (above) |
| C6 bullet 8: `make services-down` not applicable, `coverage-run.log` removed | none | | Not a stumble |

Log checks: `git diff --numstat` for the log is `142 0` (no deletion). `./scripts/check-findings-log.sh` parses 25 entries and names exactly DF-07, DF-08 and DF-19 as OPEN, which are the three Stage C promotions.

## Todo

`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md` was moved with `git mv` to `.planning/todos/completed/` and gained `## Resolution (2026-11-03, Phase 56 plan 56-09)` carrying both figures, both commits, the CI run and job ids, both tool versions, the devcontainer caveat (guide step 1 `make services-up` not exercised on a bare host) and the DF ids. The resolution also states that the todo's Solution step 6 (`make ci-full`, `make test-cli`, `make bench-check`) was not part of the D-17 walk and was not run.

## Deviations from Plan

### Auto-fixed Issues

None under Rules 1-3. The scope choices below are plan-boundary decisions.

### Judgment calls

**1. DF-25 and the `Makefile` edit rest on reading, not on a walk step.** PD-09-2 allows the `coverage-html` line "only if its trap is confirmed and logged". Trap 4 was not run. The scope mismatch is certain from the recipe text and the guide's own explanation of `llm-all`, so the entry is logged with `By: Claude (by reading…)`, claims no measured difference, and the fix is checked with `make -n coverage-html`. The target itself was not run.

**2. DF-20's fix was not made.** The orchestrator named `.devcontainer/Dockerfile.dev` as in scope only if the plan allowed it; it is not in `files_modified`, so the `RUN cargo install --locked --version 0.8.7 cargo-llvm-cov` line was filed as a pending todo. `.devcontainer/README.md:10` already lists "llvm-cov" as provided, which becomes true after that line lands. The guide's devcontainer paragraph is written to hold either way ("may not carry", check `cargo llvm-cov --version` first).

**3. DF-24 left a second stale label.** The step-summary heading `### Coverage (workspace, --features integration-tests)` in `ci.yml` (two lines below the `Scope:` print) carries the same stale scope. PD-09-2 and the acceptance criteria cap the `ci.yml` edit at one changed line, so only the `Scope:` line was changed and the entry's closing bullet says the heading was left. It is a candidate for the next plan that touches that step.

**4. Attribution trailers.** The orchestrator's text named `Claude Fable 5.1` for the trailer; this agent is Sonnet 5.5, so the commits carry the harness-provided `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` with the same `Claude-Session` line.

**5. Hook invocation slip.** The first whitespace check passed two hook ids to one `pre-commit run` and errored, and the Task 2 commit was made before it was re-run. Each hook was then run separately on every touched file and passed (and again before the Task 3 commit); the commit hooks also ran at commit time.

### Verification notes

- `make check-version-strings`: exit 0, 128 files scanned.
- `make -n coverage-html`: prints `cargo llvm-cov --workspace --features integration-tests,llm-all --html --output-dir target/coverage`.
- `ci.yml` parses with `yaml.safe_load`. `actionlint` is not installed in this environment, so the CI "Workflow Lint" job on the next push is the proof of record (the Phase 55-01 precedent). `shellcheck` is installed but no script changed.
- `make check-doc-config` was not run: no YAML fence in `docs/src` changed (the guide edits add bash fences only).
- The guide snippet was run in a scratch directory on the runbook's two-record case and printed `Lines: 165/200 = 82.50%  (truncated: 82)`.
- `CHANGELOG.md` `[0.11.0]` and `MIGRATION.md` were not touched. No `[Unreleased]` bullet was added: the plan does not ask for one and the changes are docs, a Makefile target scope and a CI label.

## Auth gates

None.

## Known Stubs

None. No stub, placeholder or empty-value pattern in any file this plan touched.

## Threat Flags

None. The `ci.yml` change is a printed string; `Makefile` gains a feature name on an existing command; no network, auth or file-access surface is new. T-56-32 was mitigated by the integer `LH/LF` truncation, the same-commit check and the move only on pass; T-56-33 by the slots holding command output only (no `.env` content or credential appears in any slot); T-56-34 by the one-line cap and the YAML parse.

## Stage C inputs

None new from this plan. The open promotions are unchanged: DF-07 (S1), DF-08 (S2) and DF-19 (S2). DF-20 has a pending todo (`.planning/todos/pending/2026-11-03-devcontainer-install-cargo-llvm-cov.md`) and can be picked up at any time; it is not a Stage C blocker.

## Self-Check: PASSED

- FOUND: `.planning/phases/56-dogfood-findings/56-RUNBOOK-COVERAGE.md` (six slots filled, `Comparison 2026-11-03` line in `## Triage checklist`)
- FOUND: `.planning/todos/completed/2026-08-13-verify-local-coverage-reproduction.md` (`## Resolution`), and the pending original is gone
- FOUND: `.planning/todos/pending/2026-11-03-devcontainer-install-cargo-llvm-cov.md`
- FOUND: commits `1e50b92b4`, `9dcaaf720`, `370f36383`
- `./scripts/check-findings-log.sh` lists exactly DF-07, DF-08, DF-19 as OPEN; the log diff deletes 0 lines
- `git diff --numstat` for `Makefile` and `ci.yml` is `1 1` each
