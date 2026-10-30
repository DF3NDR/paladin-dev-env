---
phase: 52-ci-release-pipeline-repair
verified: 2026-10-29T23:59:00Z
status: human_needed
score: 5/5 roadmap success criteria verified (plus 8/8 requirement IDs accounted for)
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
deferred: []
behavior_unverified_items: []
human_verification:
  - test: "Judgment-tier prohibitions (unverified-prohibition, human review recommended): confirm the 10 judgment-tier must-NOT items listed under 'Prohibitions' below"
    expected: "Maintainer agrees each did not happen. This verifier's verdicts on them are NON-AUTHORITATIVE LLM-judge verdicts."
    why_human: "ADR-550 D3/D4: judgment-tier prohibitions never pass silently; they need human resolution at the end-of-phase checkpoint."
  - test: "Platform CodeQL dismissals: with a token that can read code-scanning alerts, run GET /repos/DF3NDR/paladin-dev-env/code-scanning/alerts?state=open and ?state=dismissed"
    expected: "0 open CodeQL rule alerts; the 37 register alert numbers all dismissed with the register's reasons (24 false positive, 12 used in tests, 1 won't fix); 20 OSV advisory alerts still open by design"
    why_human: "The unauthenticated API returns 403 for code-scanning alerts, so the platform side of H-04 rests on the maintainer's pasted read-back (52-HANDOVER.md). Roadmap SC5 itself is worded on the register and is verified."
  - test: "Merge of PR #61 and the later v0.11.1 tag"
    expected: "PR #61 (open, label 'release', head dcaf6af26) merges with all 46 required contexts green; the first live use of the release-body size guard is the v0.11.1 tag (Phase 58)"
    why_human: "Merge and tag are maintainer-gated and outside this phase; the guard has only been exercised by stubbed harnesses, never a live release call (D-12)."
---

# Phase 52: CI & Release Pipeline Repair - Verification Report

**Phase Goal:** Every later phase in this milestone, and the `v0.11.1` release itself, runs under CI and release gates that actually check something: the semver job compares against the published `0.11.0` baseline instead of reading `0 checks`, the migration-allowlist, end-to-end and publish-dry-run gates fire before the irreversible merge, and the release-body builder can no longer fail on a long changelog.
**Verified:** 2026-10-29
**Status:** human_needed (no gaps; goal achieved in the codebase; flagged judgment-tier prohibitions and one unreadable platform read-back need a human)
**Re-verification:** No - initial verification

Method note: SUMMARY.md claims were not used as evidence. Everything below was read from the tree at `dcaf6af26`, re-run locally (fast shell harnesses only), or re-read from the public GitHub API (unauthenticated). No build, test or doc run of Rust was performed (disk is ~1 GB); one live `check-semver.sh --package paladin-eval` probe was.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Semver job named for the 0.11.0 baseline, covers all twelve crates incl. `paladin-eval`, logs a non-zero check count; a throwaway API break turns it red | VERIFIED | `ci.yml:394-424` job `Semver Checks (vs v0.11.0)` runs only `./scripts/check-semver.sh`; the script runs `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type patch` over `CRATES` (sourced from `scripts/publish-crates.sh`: 12 names, `paladin-eval` present) and fails a package with no count or `0 checks` (`evaluated zero checks`). Tool pinned `cargo-semver-checks@0.50.0`. Live probe here: `paladin-eval: 223 checks: 223 pass, 31 skip`, exit 0. Green CI on head `716923cac` (PR run 40952352062, job 123724237746): annotation lists all twelve crates at `223 checks`. RED proof, re-read from the API: job 123234542712 on `scratch/52-semver-red-proof` (head `6f3d8d78`, run 40808979632) is `completed`/`failure`, step "Check semver against the published v0.11.0 baseline" failed, annotation `paladin-storage: lint failure (exit 100)`; scratch SHA is not an ancestor of HEAD (`merge-base --is-ancestor` rc=1); remote scratch branch is gone (`git ls-remote` empty). |
| 2 | Crate-wide semver allows and mirrored allowlist entries are gone; semver job stays green | VERIFIED | `find . -name Cargo.toml ... \| xargs grep -l semver` finds nothing (all six tables, 28 lines, removed); `.cargo/semver-checks-allowlist.toml` has 0 `[[entry]]` blocks; semver job green at `716923cac` (tree after removal), twelve packages `223 checks`; `52-CI-EVIDENCE.md` D-02 table records each package measured with every table removed. |
| 3 | On a PR, before merge, `check-migration-allowlist`, `End-to-End Tests` and `Publish Dry Run` all run and report | VERIFIED | `ci.yml:29-31` `pull_request.types: [opened, synchronize, reopened, labeled]`; `check-migration-allowlist.sh` is a step (line 183-184) of the required `License & Dependency Policy` job with no `if:`; `publish-dry-run` (`needs: [lint, test]`) has no job `if:`; `e2e-tests` `if:` = push to main OR pull_request with `contains(github.event.pull_request.labels.*.name, 'release')` (element match). Live observation on PR #61 (still open, label `release` present): unlabelled run 40928112540 `Publish Dry Run` success / `End-to-End Tests` skipped; labelled run 40932320347 e2e success (job 123663512686); PR run 40952352062 on head `716923cac`: all 39 jobs success, none skipped. Ruleset 20868126 live read: 46 contexts, includes `Publish Dry Run`, `Semver Checks (vs v0.11.0)`, `License & Dependency Policy`, and equals the committed `.github/rulesets/protect-main-branch.json` set. |
| 4 | `create-or-reuse-release.sh` builds a >131,072-byte body without argv, proven by a regression test over that size | VERIFIED | `scripts/create-or-reuse-release.sh:127` uses `jq -n --rawfile body`. `tests/scripts/create-or-reuse-release_test.sh` case 10 builds a body > 131072 bytes at test time and asserts one create call, payload `.body` length > 131072 and byte-identity; suite passes (49 assertions). Mutation check (scratch copy outside the repo): replacing `--rawfile` with `--arg body "$(cat ...)"` makes the suite fail 7/49 including case 10 (zero create calls, body length 0), so the test is a real regression guard. Shared helper `scripts/release-body-fit.sh` (85 assertions) and finalize guard in `finalize-release-body.sh` (129 assertions incl. idempotency) pass. |
| 5 | CodeQL alerts carry dismissal reasons in `CODEQL-DISMISSALS.md` with register check green and advisory-only posture unchanged; issue #41 closed with `v0.11.0` Release reference; publish-order reading recorded as accepted in release docs | VERIFIED (platform half: see Human Verification 2) | Register: 37 rows (6 earlier byte-identical per `git diff` showing only the heading/count lines removed, + 31 new), parsed with `tomllib`: 37 distinct alert numbers, 37 distinct `why_present`, 37 distinct `why_dismissed` (min 192 chars, not templated), `Declared dismissals: 37` = heading = row count; `make check-codeql-dismissals` is part of `make check-gates` rc=0; its harness passes. All 24 PR #59-head annotation locations appear among the 37 CodeQL alerts (the "24" was the PR-diff view; the maintainer's list held 57 alerts: 37 CodeQL + 20 OSV, recorded in `52-CARRYOVER-EVIDENCE.md`). `codeql.yml` and `release.yml` unchanged since phase start (`git diff 3ece15f4c HEAD` empty). Issue #41 live read: `closed completed 2026-10-28T23:09:45Z`, 1 comment (id 6482461863) citing the v0.11.0 Release URL; live `releases/tags/v0.11.0` confirms published 2026-10-26T20:29:52Z, 10 assets, not draft (matches the comment). Publish-order reading: `docs/src/appendix/release-automation.md:101` paragraph "accepted 2026-10-28, Phase 52, CIREL-09" names `Check publish order` + dependency-ordered `publish-crates` + `Pre-Publish Consistency Gate`, and states `release.yml` has no added step. |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified). Plan-level truths were also spot-checked: finalize idempotency/under-cap identity/over-cap failure paths (harness), tripwire boundary 0 vs 1 checks (check-semver harness, 70 assertions), allowlist window `## 11.2`..`## 11.3` with frozen-register cases (20 assertions), MIGRATION.md bytes from `# 10.` to EOF identical to pre-plan state (md5 `b9fdedc4...` before and after), CHANGELOG.md untouched, trigger register row records `labeled` (`branching-model.md:49`), 46-context wording in `branch-protection.md` and `cicd.md`.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `scripts/check-semver.sh` | single semver gate, tripwire | VERIFIED | substantive (full implementation), wired from `ci.yml:424` and `Makefile:260-261` (`check-semver`); ran live |
| `tests/scripts/check-semver_test.sh` | stubbed-cargo harness | VERIFIED | 70 assertions pass |
| `scripts/release-body-fit.sh`, `scripts/create-or-reuse-release.sh`, `scripts/finalize-release-body.sh` + tests | size-proof body builder, finalize guard | VERIFIED | wired by `RELEASE_BODY_FIT_LIB_ONLY=1` source; harnesses pass (85/49/129) |
| `scripts/check-migration-allowlist.sh`, `MIGRATION.md` §11, `.cargo/semver-checks-allowlist.toml` | §11.2-only register gate, empty allowlist | VERIFIED | `make check-migration-allowlist` green inside `check-gates`; 20-assertion harness passes |
| `.github/workflows/ci.yml` | renamed semver job, PR types, e2e label `if`, dry-run on every PR, allowlist in required job | VERIFIED | read directly; no inline awk set-equality left |
| `.github/rulesets/protect-main-branch.json` | 46-context record | VERIFIED | 46 contexts; live ruleset identical |
| `CODEQL-DISMISSALS.md` | 37 governed rows | VERIFIED | see SC5 |
| `docs/src/appendix/release-automation.md`, `release-checklist.md`, `branch-protection.md`, `cicd.md`, `branching-model.md` | label act, publish-order reading, size limit, 46 contexts, trigger row | VERIFIED | grep-confirmed; `check-workflow-triggers.sh` green |
| `52-SEMVER-RED-EVIDENCE.md`, `52-CI-EVIDENCE.md`, `52-HANDOVER.md`, `52-CARRYOVER-EVIDENCE.md` | evidence records | VERIFIED | factual claims re-read from the public API match (job conclusions, ruleset, issue, branch) |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| `ci.yml` semver job | `scripts/check-semver.sh` | `run: ./scripts/check-semver.sh` (line 424) | WIRED |
| `check-semver.sh` | `publish-crates.sh` | `PUBLISH_CRATES_LIB_ONLY=1 source` (12 names) | WIRED |
| `ci.yml` cargo-deny job | `check-migration-allowlist.sh` | step line 183-184; job is a required context in the ruleset | WIRED |
| `check-migration-allowlist.sh` | `MIGRATION.md` | awk window `## 11.2 `..`## 11.3 ` | WIRED |
| `create-or-reuse-release.sh` / `finalize-release-body.sh` | `release-body-fit.sh` | `RELEASE_BODY_FIT_LIB_ONLY=1` source, `release_body_fit_for_create` | WIRED |
| ruleset record | `ci.yml` job names | `check-workflow-triggers.sh` clause 3 (green) | WIRED |

### Data-Flow Trace (Level 4)

Not applicable: no component renders dynamic data; the artifacts are scripts, workflow and records. Equivalent trace done for the semver gate: the count shown in CI annotations is parsed from the live tool output (`_cs_check_line`), and the red run proved a real lint failure propagates to a failing job.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Live semver probe against 0.11.0 baseline | `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-eval` | `223 checks: 223 pass, 31 skip`, `0 failure(s)`, rc 0; disk flat at 987 MB | PASS |
| All offline release gates | `make check-gates` | rc 0 (changelogs, crate names, advisory register, workflow suppressions, triggers, CodeQL register, allowlist, publish order, dependency allowlist) | PASS |
| Shell guard suite | `make test-shell-guards` | rc 0, 85 assertions in final harness; individually: release 49, fit 85, finalize 129, semver 70, allowlist 20, codeql 11, triggers 11 | PASS |
| shellcheck on touched scripts/tests | `shellcheck --severity=warning ...` | clean | PASS |
| >131,072-byte regression is meaningful | mutate `--rawfile` to `--arg` in a scratch copy, run harness | 7/49 fail incl. case 10 | PASS |
| CI semver red proof | public API job 123234542712 | `failure`, `paladin-storage: lint failure (exit 100)` | PASS |

### Probe Execution

SKIPPED: no `probe-*.sh` scripts are declared by the phase or present under `scripts/*/tests/`.

### Requirements Coverage

PLAN frontmatter `requirements:` union: 52-01 [CIREL-01, 03], 52-02 [05], 52-03 [06, 07], 52-04 [05], 52-05 [01, 02, 03], 52-06 [04, 09], 52-07 [01], 52-08 [01, 04, 06, 07] = CIREL-01..07 and 09, exactly the eight IDs in the ROADMAP phase header. REQUIREMENTS.md maps the same eight to Phase 52 (traceability rows 308-316). CIREL-08 is mapped to Phase 55 (Pending), not an orphan of this phase. No orphaned requirement.

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| CIREL-01 semver checks all 12 crates incl. `paladin-eval`, armed | 52-01, 05, 07, 08 | SATISFIED | SC1; live probe; red CI job |
| CIREL-02 crate-wide allows removed | 52-05 | SATISFIED | SC2 |
| CIREL-03 allowlist gate on every PR | 52-01, 05 | SATISFIED | step in required `License & Dependency Policy` job; §11.2 gate |
| CIREL-04 E2E and dry-run before merge | 52-06, 08 | SATISFIED | SC3; PR #61 runs; ruleset read-back |
| CIREL-05 release body without argv | 52-02, 04 | SATISFIED | SC4 |
| CIREL-06 CodeQL alerts triaged | 52-03, 08 | SATISFIED (platform read-back human item) | register 37 rows; check green |
| CIREL-07 issue #41 closed with Release reference | 52-03, 08 | SATISFIED | live issue read |
| CIREL-09 publish-order reading accepted in docs | 52-06 | SATISFIED | `release-automation.md:101`; `release.yml` unchanged |

### Prohibitions (must_haves.prohibitions)

Test-tier (enforcement wired and exercised; verified):

| Prohibition | Evidence |
|-------------|----------|
| 52-01 MUST NOT green a zero-check or no-count package | `check-semver_test.sh` boundary cases (0 fails, 1 passes) pass |
| 52-02 MUST NOT shorten silently / put body in argv or env / fall back to empty body | release-body-fit and create-or-reuse harnesses (stderr never contains body, sentinel footer, named failures, `--rawfile` mutation red) |
| 52-03 MUST NOT describe CodeQL as required or add Snyk | `grep Snyk CODEQL-DISMISSALS.md` empty; `codeql.yml` unchanged; ruleset record has no CodeQL context |
| 52-04 MUST NOT drop artifact sections / edit with over-limit body | finalize harness (129 assertions) |
| 52-05 MUST NOT re-add crate-wide allow / edit frozen MIGRATION or CHANGELOG | no semver table in any manifest; md5 identical from `# 10.`; CHANGELOG diff empty |
| 52-06 MUST NOT edit `release.yml` | `git diff 3ece15f4c HEAD -- .github/workflows/release.yml` empty |
| 52-07 MUST NOT merge the scratch break or commit the mutation script | scratch SHA not an ancestor of HEAD; no `.rs` file changed between planning base and HEAD (`git diff 3ece15f4c HEAD -- '*.rs'` empty) |

Judgment-tier: **unverified-prohibition, human review recommended** (verdicts below are NON-AUTHORITATIVE, none is a silent pass):

| Prohibition | Non-authoritative verdict |
|-------------|---------------------------|
| 52-01 no second hand-maintained package list in ci.yml or script | Looks respected: package names only appear in comments (ci.yml 372, 418; script 50); list comes from `publish-crates.sh`. (ci.yml:562 is the unrelated pre-existing crate-isolation matrix.) |
| 52-01 allowlist gate not left only in a non-required job | Looks respected: step is in `License & Dependency Policy`, which is in the 46-context ruleset |
| 52-03 no dismissal row without inspection / no bulk justification | Spot-checked: 37 distinct, long, row-specific justifications (e.g. row 54 explicitly refuses to certify the key-in-URL handling). Source inspection of each of 37 locations not repeated |
| 52-03 issue #41 comment facts read from the public API | Comment facts (Release URL, `published_at`, 10 assets) match the live release read |
| 52-06 no automation applies the `release` label | `grep addLabels\|labeler .github` empty |
| 52-06 executor must not change the live ruleset | The PUT was performed by the maintainer (recorded in handover; live ruleset read-back matches) |
| 52-07 no push or PR before the maintainer's answer | Evidence file records `Push-ok`; cannot be independently re-observed |
| 52-07 break must not be `#[doc(hidden)]` or non-default-feature | Break was `pub fn cron_field_count` in always-compiled `pub mod cron`; the tool did report `function_missing`/`1 fail` in CI, so it was visible |
| 52-08 executor must not perform maintainer-gated acts | Handover states H-02..H-04 were run from a Claude Code session with the maintainer's own token at the maintainer's direction (H-01 PUT run by the maintainer directly). That is a maintainer act by authority, but it was AI-assisted: the maintainer should confirm that reading |
| 52-08 no `done` without reply or read-back; local coverage not claimed | H-06 correctly `deferred`, no local figure claimed; H-01/02/03/05/08 re-read here and match; H-04 is not readable without a token (see Human Verification 2) |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `.github/workflows/ci.yml` | ~198-210 | `grep -c 'TBD' MIGRATION.md` | Info | This is the pre-existing placeholder-marker gate moved into the required job, not a debt marker. No TBD/FIXME/XXX added by the phase in any other changed file |
| `crates/paladin-content/.../news_api_fetcher.rs` | 64-72, 138-146 | API key in URL; reqwest error text can echo URL; client follows redirects | Warning (out of phase scope, pre-existing) | D-52-03-01; documented in register row 54 and `deferred-items.md`; not mapped to any later ROADMAP phase (grep of ROADMAP/REQUIREMENTS finds nothing) - needs an owner |
| `Cargo.lock` | - | Three OSV alerts with no governing exception: 25 (`RUSTSEC-2026-0221` event-listener), 24 (`RUSTSEC-2026-0205` scc), 23 (`GHSA-7gcf-g7xr-8hxj`) | Warning (out of phase scope) | D-52-03-02; not dismissed or registered; unowned |
| `CODEQL-DISMISSALS.md` | rows 40, 49 | stale line anchors (`:106` vs `:360`, `:387` vs `:472`) | Info | D-52-03-03; D-14 froze these rows; guard only checks file existence |

### Informational

- ROADMAP.md line 257 still shows Phase 52 as `[ ]` although all eight plan lines are `[x]` and REQUIREMENTS.md marks the eight IDs Complete; the orchestrator should tick it on closing the phase.
- Folded todo (local `make coverage` reproduction, H-06) is correctly deferred to a Docker-capable machine (`resolves_phase: 56`); no gate or threshold changed; not a must-have of this phase.
- Not verified here by design: Rust build, clippy, doc, `make api-surface`, the two guard tests (disk allowance). They are covered by CI jobs `Code Quality`, `API Surface Tracking`, `Integration Tests`, all `success` on `716923cac` (push run 40952344198 and PR run 40952352062, both `completed success`). The only changes after that head are under `.planning/` (`git diff --name-only 716923cac HEAD | grep -v '^.planning/'` empty).
- Verifier-observed oddity, not a gap: the PR #61 head `dcaf6af26` itself has no recorded CI run in the evidence files; it differs from the green head only in `.planning/` files.

### Human Verification Required

1. **Judgment-tier prohibitions.** Test: read the ten judgment-tier rows in the Prohibitions table. Expected: maintainer accepts each non-authoritative verdict (notably the 52-08 reading that H-02..H-04 were maintainer acts executed in an AI-assisted session). Why human: ADR-550 soft-gate; judgment items never auto-pass.
2. **Platform CodeQL dismissals.** Test: read open and dismissed code-scanning alerts with an authorised token. Expected: 0 open CodeQL alerts; the 37 register alerts dismissed with matching reasons; 20 OSV alerts open by design. Why human: unauthenticated API returns 403; the recorded read-back is the maintainer's own.
3. **Merge and tag.** Test: merge PR #61 with the `release` label and all 46 contexts green; later the `v0.11.1` tag. Expected: first live exercise of the release-body size guard succeeds. Why human: maintainer-gated; outside this phase.

### Gaps Summary

No gaps. Every ROADMAP success criterion is met in the codebase and re-checked independently: the semver gate is armed at the 0.11.0 baseline over twelve crates and was seen red in CI on a real break (and green at 223 checks per crate with all crate-wide allows removed); the allowlist, dry-run and label-gated end-to-end gates run on the pull request and the live ruleset requires them; the release-body builder is argv-free with a regression that is demonstrably red against the old construction. Status is `human_needed` solely because of the flagged judgment-tier prohibitions and one platform read-back this sandbox cannot perform. Three carried findings (News API key in URL, three ungoverned OSV alerts, two stale register anchors) are out-of-scope discoveries without a later-phase owner and are surfaced as warnings, not gaps.

---

_Verified: 2026-10-29_
_Verifier: Claude (gsd-verifier)_
