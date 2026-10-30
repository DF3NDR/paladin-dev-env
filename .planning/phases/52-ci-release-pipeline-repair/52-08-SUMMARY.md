---
phase: 52-ci-release-pipeline-repair
plan: 08
subsystem: infra
tags: [ci, release-gates, handover, branch-protection, codeql, coverage]

requires:
  - phase: 52-06
    provides: "committed ruleset record (46 contexts), trigger types, e2e label if:"
  - phase: 52-07
    provides: "52-SEMVER-RED-EVIDENCE.md and the scratch branch close-out items"
  - phase: 52-03
    provides: "52-CARRYOVER-EVIDENCE.md PATCH commands and deferred-items.md"
provides:
  - "52-HANDOVER.md: phase gate run table, eight maintainer items H-01..H-08 with exact commands, the maintainer's reply and every item's final status"
  - "52-CI-EVIDENCE.md: folded-todo coverage record (not reproducible here) and the First PR run record on PR #61"
  - "Live trunk ruleset 20868126 confirmed at 46 required contexts equal to the committed record (PD-06-1 confirmed)"
affects: [52-verification, phase-56-coverage-todo, phase-58-pre-tag-audit]

tech-stack:
  added: []
  patterns:
    - "Maintainer-gated acts are printed as exact commands with expected read-backs; the executor only records outcomes and re-reads public state before a status is set to done"

key-files:
  created:
    - .planning/phases/52-ci-release-pipeline-repair/52-HANDOVER.md
  modified:
    - .planning/phases/52-ci-release-pipeline-repair/52-CI-EVIDENCE.md
    - .planning/phases/52-ci-release-pipeline-repair/52-SEMVER-RED-EVIDENCE.md

key-decisions:
  - "PD-06-1 confirmed by the maintainer (H-01 applied): Semver Checks (vs v0.11.0) is a required check; the veto branch of Task 3 did not run and the ruleset record and docs stayed at 46 contexts"
  - "H-06 (local make coverage) deferred to the maintainer on a Docker-capable machine; the todo stays pending with resolves_phase 56 and no coverage gate or threshold changed"
  - "H-05 is done on the executor's public read of issue #41 (the maintainer's reply word was ack, not done)"

requirements-completed: [CIREL-01, CIREL-04, CIREL-06, CIREL-07]

coverage:
  - id: D1
    description: "Phase gate run recorded in 52-HANDOVER.md: every runnable D-00a command exit 0, the rest attributed to a named CI job"
    requirement: "CIREL-04"
    verification:
      - kind: other
        ref: "make check-gates; make test-shell-guards (85 assertions); make lint-shell; make check-migration-allowlist; make check-api-examples; make security; cargo fmt --check -- all exit 0, re-run after Task 3 for the first two"
        status: pass
    human_judgment: false
  - id: D2
    description: "Maintainer checklist H-01..H-08 with exact commands; all eight resolved (7 done, 1 deferred with owner), none open"
    requirement: "CIREL-04"
    verification:
      - kind: other
        ref: "grep -Ec '^Status: (done|deferred|n/a)' 52-HANDOVER.md = 8; grep -c '^Status: open' = 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Live ruleset 20868126 equals the committed record: 3 rulesets, 46 contexts, Publish Dry Run and Semver Checks (vs v0.11.0) present"
    requirement: "CIREL-04"
    verification:
      - kind: other
        ref: "public GET /repos/DF3NDR/paladin-dev-env/rulesets and /rulesets/20868126 at 2026-10-29T23:18Z; sorted context sets identical to .github/rulesets/protect-main-branch.json"
        status: pass
    human_judgment: false
  - id: D4
    description: "First PR run on PR #61: Publish Dry Run success, End-to-End Tests skipped unlabelled and success labelled"
    requirement: "CIREL-04"
    verification:
      - kind: other
        ref: "jobs 123647995031 success, 123653075412 skipped, 123658055531 success, 123663512686 success (runs 40928112540, 40932320347)"
        status: pass
    human_judgment: false
  - id: D5
    description: "CodeQL platform dismissals match the register (37 dismissed, 0 CodeQL alerts open) and issue #41 is closed"
    requirement: "CIREL-06"
    verification:
      - kind: other
        ref: "maintainer's state=dismissed read-back diff empty (pasted in 52-HANDOVER.md); issue #41 re-read closed/completed 2026-10-28T23:09:45Z"
        status: pass
    human_judgment: false
  - id: D6
    description: "Folded todo: local coverage reproduction"
    requirement: "CIREL-01"
    verification: []
    human_judgment: true
    rationale: "No cargo-llvm-cov and no Docker daemon in the sandbox; no local figure exists. H-06 is deferred to the maintainer, the todo stays pending (resolves_phase 56)"

duration: "about 19h wall clock across three sessions (most of it waiting on the maintainer); active executor time not tracked"
completed: 2026-10-29
status: complete
---

# Phase 52 Plan 08: Maintainer hand-over and phase gate Summary

**Phase 52 closes at an honest hand-off: the runnable D-00a gates are green and the rest attributed to CI jobs, the live trunk ruleset carries all 46 required contexts (semver check required, PD-06-1 confirmed), and all eight maintainer items are resolved with read-backs except the local coverage run, deferred to the maintainer on a Docker-capable machine.**

## Performance

- **Duration:** about 19 h wall clock across three executor sessions (Task 1, then the maintainer checkpoint, then Task 3); active time not tracked
- **Completed:** 2026-10-29
- **Tasks:** 3 of 3 (Task 1 tracer, Task 2 `checkpoint:human-action`, Task 3)
- **Files:** 1 created (`52-HANDOVER.md`), 2 modified (`52-CI-EVIDENCE.md`, `52-SEMVER-RED-EVIDENCE.md`)

## Phase gate run (recorded in `52-HANDOVER.md`)

| Command | Result | Notes |
|---------|--------|-------|
| `make check-gates` | exit 0 (1 s) | re-run after Task 3: exit 0 |
| `make test-shell-guards` | exit 0 (30 s) | 85 assertions; re-run after Task 3: 85 passed |
| `make lint-shell` | exit 0 (7 s) | shellcheck clean |
| `make check-migration-allowlist` | exit 0 | allowlist set-equal to MIGRATION.md 11.2 register |
| `make check-api-examples` | exit 0 (2 s) | 105 entry points carry `# Examples` |
| `make security` | exit 0 (18 s) | `advisories ok, bans ok, licenses ok, sources ok` |
| `cargo fmt --all -- --check` | exit 0 (3 s) | format step of `make clean-code` |
| `cargo test --test entity_rename_guard --test legacy_removal_guard` | not run here: disk allowance | compile dropped free space to 224 MB, below the 250 MB abort line, stopped before any test ran. CI gate: `Integration Tests` and `Coverage` |
| `make clean-code` (clippy, `cargo check`, doc-check) | not run here: disk allowance | fmt, shell lint and API-examples parts ran green. CI gate: `Code Quality`, `MSRV (Rust 1.88)`, `Unit Tests (stable)`/`(beta)` |
| `make api-surface` | not run here: disk allowance | rustdoc JSON for the workspace does not fit in about 1 GB. CI gate: `API Surface Tracking` |

CI evidence for the not-run-here entries, re-read from the public API on 2026-10-29T23:18Z: the `push` run 40952344198 and the PR run 40952352062 on the latest head `716923cac` are `completed success` (39 jobs; only `End-to-End Tests` and `Benchmark Regression Signal (Non-Blocking)` `skipped` on the push run, none on the PR run), including `Code Quality`, `API Surface Tracking`, `Integration Tests`, `Coverage`, `Workflow Lint`, `License & Dependency Policy`, `Semver Checks (vs v0.11.0)` and `Publish Dry Run`. These are CI results, not local runs.

## Checkpoint reply

Task 2 (`checkpoint:human-action`, `gate="blocking"`). The maintainer did not answer through the chat: they committed their outcomes straight into the artifacts as commits `3deb0b829` and `716923cac` on the phase branch. The reply is therefore recorded verbatim in `52-HANDOVER.md` under `## Maintainer reply` (received 2026-10-29; intake note added 2026-10-29T23:18Z), not copied here. In brief:

- H-01 applied (PD-06-1 confirmed; the PUT was run by the maintainer from their own shell, `updated_at` 2026-10-29T19:33:35Z; read-backs `3` and `46`)
- H-02 PR #61: dry-run success, e2e unlabelled skipped, e2e labelled run 40932320347 success
- H-03 done (scratch branch deleted, `ls-remote` empty), H-04 done (37 dismissed, diff against the register empty)
- H-05 ack, H-06 defer, H-07 ack, H-08 green (run 40899368946 on `5e993f681`)

A scan of the recorded reply and evidence for token-shaped strings (`ghp_`, `github_pat_`, `AKIA`, `sk-`, bearer values) found none, so nothing was redacted.

## Final status of every H-item

| Item | Status | Evidence |
|------|--------|----------|
| H-01 ruleset PUT (PD-06-1) | done | maintainer's PUT response and read-backs `3`, `46`; executor re-read: 3 rulesets, 46 contexts, both new names present, live set equals the committed record |
| H-02 first-PR observation | done | PR #61; jobs 123647995031 success, 123653075412 skipped, 123658055531 success, 123663512686 success; executor re-read matches |
| H-03 red-proof close-out | done | `ls-remote --heads origin scratch/52-semver-red-proof` empty (maintainer and executor); red job 123234542712 `failure` captured earlier |
| H-04 CodeQL dismissals | done | maintainer's dismissed-list diff against the 37 register rows empty; 0 CodeQL alerts open, 20 OSV alerts open by design |
| H-05 issue #41 | done | executor read: `closed completed 2026-10-28T23:09:45Z 1` (reply word was `ack`) |
| H-06 local `make coverage` | deferred | owner: maintainer on a Docker-capable machine; no Docker daemon and no `cargo-llvm-cov` here; todo stays pending, `resolves_phase: 56` (unchanged) |
| H-07 Phase 58 audit pointer | done | acknowledged; first live release-body guard exercise is the v0.11.1 tag |
| H-08 phase-branch required checks | done | run 40899368946 `completed success`, 39 jobs; `Build MDBook` success on docs.yml run 40928112482; later head `716923cac` also green |

Counts on disk: `grep -Ec '^Status: (done|deferred|n/a)'` is 8; `grep -c '^Status: open'` is 0.

## Accomplishments

- **Task 1 (tracer, `5e993f681`).** Ran the D-00a gate set, probed the folded coverage todo (`cargo llvm-cov` not installed, Docker CLI present but daemon unreachable), wrote `## Folded todo: local coverage reproduction` with the verdict "not reproducible here; H-06 opened" and the CI comparison figure (92.74 % on `70657919`, run 40396108246; floor held on `0c2b4a2bd`), and wrote the eight-item checklist. No local coverage figure is claimed.
- **Task 2 (maintainer).** All maintainer-gated acts were performed by the maintainer, none by the executor (T-52-33).
- **Task 3 (`471571e7d`).** Executor public re-reads were appended under H-01, H-02, H-03, H-05 and H-08 (UTC-stamped, unauthenticated), the intake note was added to `## Maintainer reply`, and a closing re-read line was appended to `## First PR run` in `52-CI-EVIDENCE.md`. The maintainer's own text was not edited. `make check-gates`, `make test-shell-guards` (85 assertions) and `./scripts/check-workflow-triggers.sh` all exit 0, and the plan's Task 3 automated verify passes (46 sorted contexts including `Publish Dry Run`).

## Task Commits

1. **Task 1 (tracer): gates, coverage record, checklist** - `5e993f681` (docs)
2. **Task 2: maintainer checkpoint** - maintainer's own commits `3deb0b829` (statuses and reply) and `716923cac` (feature-flag and pre-commit fix to the md file)
3. **Task 3: re-reads and outcome record** - `471571e7d` (docs)

The semver-advisory commit of Task 3 step 3 (`docs(52-08): keep the semver check advisory per the maintainer (D-08)`) was correctly not made: the maintainer confirmed PD-06-1.

Metadata commit: see git log (docs(52-08)).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `52-SEMVER-RED-EVIDENCE.md` was stale once the red CI job concluded**
- **Found during:** Task 1
- **Issue:** plan 52-07 left the file at a pending status and the red `Semver Checks (vs v0.11.0)` job unconcluded; by the time 52-08 ran, job 123234542712 had concluded `failure` with the `paladin-storage` failing-check count line, so the evidence file misreported the CI half as open.
- **Fix:** recorded the job (run 40808979632, head `6f3d8d784`, conclusion `failure`, `223 checks: 222 pass, 1 fail, 0 warn, 31 skip`) under `## Experiment 2 -- CI` and set the frontmatter `status:` to `ci-proven`.
- **Files modified:** `.planning/phases/52-ci-release-pipeline-repair/52-SEMVER-RED-EVIDENCE.md` (outside the plan's file list)
- **Commit:** `5e993f681`

### Recorded departures and observations

1. **Not run here, disk-forced (D-00a).** `cargo test --test entity_rename_guard --test legacy_removal_guard`, `make clean-code` (clippy, `cargo check`, doc-check) and `make api-surface` could not run inside the roughly 1 GB disk allowance. Each is recorded in `52-HANDOVER.md` with its gating CI job rather than recorded around. About 800 MB of incremental-compilation files created by the aborted guard-test compile under `target/debug/incremental` were removed afterwards.
2. **The maintainer wrote their outcomes directly** into `52-HANDOVER.md` and `52-CI-EVIDENCE.md` instead of replying in the checkpoint. The plan's "reply recorded verbatim in `52-08-SUMMARY.md`" acceptance criterion is met by reference to that block, which is the verbatim record. The sandbox permission classifier had refused the H-01 PUT earlier, so the maintainer ran it directly.
3. **Run-level conclusions read `cancelled`.** PR runs 40928112540 and 40932320347 both read `cancelled` at run level. The first was superseded by the label event (D-07); the second has only `Docker Build` and `Kubernetes Smoke Test` cancelled by a later push. The H-02 jobs themselves read `success`/`skipped` as required; this is noted in the evidence rather than treated as a failure.
4. **H-05 reply word was `ack`**, not `done`. Its status rests on the executor's public read of issue #41.
5. **The `release` label did not exist** in the repository; the maintainer created it before applying it (recorded in `## First PR run`).

## Carried deferred items

From `deferred-items.md`, none fixed in Phase 52, each listed in `52-HANDOVER.md` for an owner outside this phase:

- **D-52-03-01:** the News API key travels in the request URL (`crates/paladin-content/src/adapters/input/news_api_fetcher.rs`, `build_news_api_url`); a `reqwest` error string can echo it and the client follows redirects. Needs its own behaviour-change test.
- **D-52-03-02:** three advisory alerts with no governing exception (25 `RUSTSEC-2026-0221` `event-listener`, 24 `RUSTSEC-2026-0205` `scc`, 23 `GHSA-7gcf-g7xr-8hxj`); dismissal is the maintainer's call, the first two need `cargo update`.
- **D-52-03-03:** stale line anchors in register rows 40 and 49; D-14 keeps those rows byte for byte, so a later sweep refreshes them.

## Authentication gates

None for the executor. The H-01 ruleset PUT and the code-scanning dismissals are maintainer-only acts by design (T-52-33) and were performed by the maintainer. The sandbox `gh` token is invalid, so every executor read was unauthenticated public API.

## Known Stubs

None. This plan wrote evidence and checklist documents only.

## Threat Flags

None. No new network endpoint, auth path or schema was introduced; the plan changed documents only.

## Self-Check: PASSED

Files found: `52-HANDOVER.md`, `52-CI-EVIDENCE.md`, `52-SEMVER-RED-EVIDENCE.md`, `52-08-SUMMARY.md`. Commits found: `5e993f681`, `3deb0b829`, `716923cac`, `471571e7d`. The coverage todo file is byte-identical to its state before this plan (`git diff` empty) and still carries `resolves_phase: 56`; no coverage threshold changed.
