---
phase: 47-v0-11-0-crate-release
verified: 2026-10-26T21:50:00Z
status: passed
score: 8/9 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
human_verification:

  - test: "Read the condensed root CHANGELOG.md [0.11.0] entry against the B-1 ledger in 47-CI-EVIDENCE.md (and, where in doubt, against `git show 700a92d86f3d6b2e24bbd2c163daec7c307297e0:CHANGELOG.md`) and answer the open `- B-1 maintainer read: requested` line."
    expected: "The condensed entry adds no claim beyond the uncondensed text and drops no breaking change, security note or removal. Answer recorded as an appended dated entry in 47-CI-EVIDENCE.md."
    why_human: "Plan 47-02 Task 3 asked for this judgment and it is unanswered. The mechanical proxy (ids, BREAKING markers, figures, byte-identical subsections) passes, and the verifier re-ran its own version of it, but a changed meaning is not detectable by grep."

  - test: "Open the `Coverage` job of ci.yml run 40368801098 (job 121733645613, merge commit 70657919) in the Actions UI and copy the printed `Lines:` figure into 47-CI-EVIDENCE.md Findings and the audit's 'Outside the table' paragraph (todo 2026-10-26-record-v0-11-0-coverage-lines-figure.md)."
    expected: "A figure at or above 82 percent (ADR-0006). A figure below 82 would mean the gate or the figure was misread and must be escalated."
    why_human: "The job log and the `coverage-summary` artifact (id 12672956238) redirect to a blob host this sandbox denies; no check-run output carries the number. The floor GATE is proven green (see truth 6); only the printed figure is unread."
deferred: []
---

# Phase 47: v0.11.0 Crate Release Verification Report

**Phase Goal:** v0.11.0 ships to crates.io with every gate green.
**Verified:** 2026-10-26T21:50:00Z
**Status:** human_needed (no gaps, no blockers; two non-blocking human items)
**Re-verification:** No, initial verification

Nothing in SUMMARY.md or the phase evidence files was taken as proof. Every external fact below was re-read live and read-only from GitHub REST, the crates.io sparse index, and git. Job logs were not readable (GitHub's blob host is denied from this sandbox); that is a recorded sandbox limit, not a failure.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: all 12 publishable crates are live on crates.io at `0.11.0` | VERIFIED | `cargo metadata --no-deps` on the tree gives 13 workspace packages, 12 with `publish == null` (paladin-ai, -ai-core, -battalion, -content, -eval, -herald, -llm, -memory, -notifications, -ports, -storage, -web); `paladin-doc-examples` has `publish = []`. Live sparse-index GET (User-Agent set) for each of the 12 returned exactly one `0.11.0` line with `yanked=false` and a checksum; `0.10.1` is also present and unyanked for all 12. Index dependency metadata for `paladin-web` 0.11.0 reads `^0.11.0` and `paladin-ports` reads `paladin_core =0.11.0`. Every `Cargo.lock` workspace package is `0.11.0` at the tag. |
| 2 | SC2: the release tag is cut on a `main` merge commit, per the two-SHA rule | VERIFIED | `git fetch origin --tags`. `git rev-parse 'v0.11.0^{commit}'` = `70657919beadbd341a72be2d0107080017a803f0` = `origin/main` = `GET /pulls/59` `merge_commit_sha` (`merged: true`, base `main`). The tag is annotated (object `77a37c18…`, tagger Am0rfu5, 2026-10-26T20:29:02Z). The tagged commit has two parents, `df24689c…` (previous main tip) and `72e13f95…` (PR head), so it is a real merge commit, not a squash. Its tree `885e4ac5…` equals the PR head's tree. `ci.yml` run 40368801098 on that SHA concluded `success` before the tag. Ruleset 20868099 protects `refs/tags/v*` against creation, deletion and non-fast-forward. |
| 3 | SC3a: CHANGELOG and MIGRATION.md are complete for v0.11.0 | VERIFIED | Root and all 11 crate changelogs have `## [0.11.0] - 2026-10-26` as the first `## [` heading; `[Unreleased]` section bodies are empty in all 11 crate files and absent from the root. `MIGRATION.md` has section 10 "Upgrading from v0.10.1 to v0.11.0" with operator and library-consumer checklists and register 10.2; zero `TBD`. Debt-marker scan (`TBD`/`FIXME`/`XXX`) over the 27 non-planning files changed since the pre-phase base: none. No file under `src/` or `crates/*/src/` changed in the phase. `scripts/check-changelogs.sh` run locally: OK. Release body extraction is 108,926 bytes (limit 110,000). Residual judgment on condensation fidelity is truth 9. |
| 4 | SC3b: the publish-order gate is green on the release run | VERIFIED (under the plan's recorded reading A-SC3) | `release.yml` run 40396108246 (`head_branch v0.11.0`, head SHA = merge commit, attempt 1) concluded `success`; all 12 jobs `success`, including `Verify Tag From Main`, `Pre-Publish Consistency Gate`, `Publish to crates.io` (step `Publish crates in dependency order` success), `Create Release`, `Finalize Release Body`. `release.yml` has no step literally named `Check publish order`. The gate script `check-publish-order.sh` is a step of `License & Dependency Policy` in `ci.yml`, `success` on run 40368801098 for the merge commit, and the release run's consistency gate requires that CI run to be green. I also ran `scripts/check-publish-order.sh` locally on the tree: "OK: 12 crate(s) checked in publish order". The practical effect is proven by the registry: all 12 published in dependency order with no failure. See Warnings for the interpretation caveat. |
| 5 | The eight signed re-seal rows plus `make security` exist, with each CI-attributed leg decided by the maintainer and its CI twin green | VERIFIED | `47-ACCEPTANCE-AUDIT.md` has rows 1 to 8 with commands, SHA and results; `make security` exit 0 recorded (B-2 tokio pin to 1.53.3 clears RUSTSEC-2026-0386). Row 4 `paladin-web`, row 5 MSRV and row 6 `make test` leg are marked "not a local pass" with verbatim maintainer decisions. Independent check: on the final PR head `72e13f95` the twin jobs `Semver Checks (vs v0.9.0)`, `MSRV (Rust 1.88)`, `Unit Tests (stable)` and `Coverage` are `success`; on the merge commit `Publish Dry Run`, `End-to-End Tests`, `Security Audit` and `License & Dependency Policy` are `success`. The release itself then ran the publish path for real and succeeded. |
| 6 | PR CI required contexts, merge-commit CI and release run are green | VERIFIED | Active ruleset 20868126 lists 44 required contexts (CodeQL is not among them; `pre-commit run --all-files`, `Coverage` and `License & Dependency Policy` are). On the merge commit `70657919` every one of the 44 is `success` by name (75 success, 1 skipped non-blocking benchmark signal, 0 failure). On the final PR head `72e13f95`: 93 success, 6 skipped, 1 failure (advisory `CodeQL`, not required). The 6 skips include `End-to-End Tests` and `Publish Dry Run`, which are `main`-push-only and then passed on the merge commit. `pre-commit`, red on SHA-A0, is `success` on `72e13f95`. `Coverage` ran `scripts/coverage.sh`, which passes `--fail-under-lines 82` (floor 82, ADR-0006); the `Measure coverage` step is `success`, so the floor gate passed. The printed figure is unread (human item 2). |
| 7 | The sign-off box was ticked by the maintainer on the final head, before the tag | VERIFIED | `git log` on `47-ACCEPTANCE-AUDIT.md`: tick at `6ef9207a` and re-confirmation at `72e13f958`, both authored by Am0rfu5 (the maintainer). `72e13f958` is the PR head that the merge took. The box reads `- [x]` at the tag, at `72e13f95` and at HEAD. The uppercase `[X]` of the first tick was normalised in the re-confirmation commit; this has no effect. The tag (20:29Z) post-dates the merge-commit CI (18:35Z) by 1 h 54 min. |
| 8 | SHIP-07 is accounted for and ticked on observed fact | VERIFIED | All ten plans declare `requirements: [SHIP-07]`; `REQUIREMENTS.md` line 434 is `- [x] **SHIP-07**` and the traceability row 515 is `Phase 47 / Complete`; no other requirement id maps to Phase 47, so there are no orphans. The tick was applied after a live 12-of-12 registry re-read (plan 47-09) and this verification re-confirms it. |
| 9 | The condensed root `[0.11.0]` changelog adds no claim beyond the 156,528-byte original (B-1) | UNCERTAIN (WARNING, human decision requested) | Maintainer-approved remedy B1=a. Independent mechanical check by the verifier against `700a92d86` (`[Unreleased]` 156,528 bytes) vs the tagged `[0.11.0]` (108,926 bytes): all 17 `M-B-`, 26 `DR-`, 15 `ADR-`, 4 `RUSTSEC-` and 46 requirement ids are preserved with none missing; `BREAKING` markers 20 of 20; "one-way" statements 2 of 2. The original is reachable at the cited SHA, an ancestor of the tag. A changed meaning is a human judgment, still unanswered. |

**Score:** 8/9 truths verified (0 present, behavior-unverified). No truth failed.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| 13 workspace `Cargo.toml` + `Cargo.lock` | all at 0.11.0, internal deps `^0.11.0`, one `=0.11.0` pin | VERIFIED | `Cargo.lock` read at the tag; code review 47-REVIEW.md reached the same result. |
| `CHANGELOG.md` + 11 `crates/*/CHANGELOG.md` | stamped `[0.11.0] - 2026-10-26` | VERIFIED | Counted at the tag. |
| `MIGRATION.md` section 10 | complete, no marker | VERIFIED | `TBD` count 0 at the tag. |
| `crates/paladin-web/openapi.json` | `info.version` 0.11.0 | VERIFIED | Per code review; golden test is a CI twin. |
| `47-ACCEPTANCE-AUDIT.md` | eight rows, sign-off box | VERIFIED | Read in full. |
| `47-CI-EVIDENCE.md` | Provenance, Local sweep, CI-run table, Registry, Findings | VERIFIED | Present (203 KB); spot-checked claims match live reads. |
| `47-READY-TO-CLOSE.md`, `47-HANDOFF-web3sec.md` | hand-offs | VERIFIED | Present; handoff tag, SHA, run id and registry table match live facts. |
| `MILESTONES.md` v0.11.0 release-record block | tag, SHA, run, sign-off | VERIFIED | Matches live facts. |
| GitHub Release `v0.11.0` | published, marker present | VERIFIED | REST: not draft, not prerelease, body 109,810 chars, `paladin:release-artifacts` marker present, 10 assets, published 2026-10-26T20:29:52Z. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| tag `v0.11.0` | PR #59 merge commit | peel equality | WIRED | three-way equality with `origin/main` and `merge_commit_sha` |
| merge commit | ci.yml run | `head_sha` | WIRED | run 40368801098 `success` |
| tag push | release.yml run | `head_branch v0.11.0` | WIRED | run 40396108246, same SHA |
| release run | registry | Trusted Publishing | WIRED | per the evidence file, each version's `trustpub_data` names the repo, the run and the merge SHA (not independently re-read here beyond `0.11.0` presence) |

### Data-Flow Trace (Level 4)

Not applicable: the phase produces release metadata and registry state, not a data-rendering component.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Publish order is dependency-correct | `bash scripts/check-publish-order.sh` | "OK: 12 crate(s) checked in publish order" | PASS |
| Every crate has a changelog | `bash scripts/check-changelogs.sh` | OK | PASS |
| Release body within limit | `scripts/extract-changelog-section.sh --version 0.11.0 \| wc -c` | 108926 | PASS |
| Version/changelog clauses of the pre-publish gate | `check-release-consistency.sh --tag v0.11.0 --sha <tag>` | only `CI_LOOKUP_FAILED` (script's `gh` call is blocked by the sandbox, HTTP 403); no `MISMATCH` or `CHANGELOG_MISMATCH` clause fired | SKIP for the CI clause (sandbox), CI clause proven by the release run's own job |

### Probe Execution

No `probe-*.sh` is declared by the phase plans. Step 7c: SKIPPED.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| SHIP-07 | 47-01 to 47-10 | v0.11.0 released: 12 crates at 0.11.0, tag on a main merge commit, CHANGELOG and MIGRATION complete, publish-order gate green | SATISFIED | truths 1 to 4; ticked in REQUIREMENTS.md |

No orphaned requirement ids for Phase 47.

### Anti-Patterns Found

None in code (no source file changed). Debt-marker scan over the phase's changed files: clean. The only `Unreleased` token left is a pre-existing stale link definition (WR-02 below).

### Judgment on the known-context items

| Item | Verdict | Reason |
|------|---------|--------|
| (a) redate 10-25 to 10-26; SHA-A4 holds the SHA-A2 role | Documented deviation, no goal impact | The maintainer authored the re-confirmation on the head the merge took; the merge tree equals that head's tree; the box is `[x]` at the tag. |
| (b) rows 4 (`paladin-web`), 5, 6 test leg CI-attributed | Documented deviation, no goal impact | Each carries a verbatim maintainer decision and is stated as "not a local pass". I confirmed the twins are `success` on the final head, and the real publish path then succeeded. |
| (c) Coverage `Lines:` figure unread | Not a gap against the goal; recorded as human item 2 | The 82 percent floor gate is wired and its step is green; only the number is unread. A tracked todo exists (recheck by 2026-11-02). |
| (d) B-1 read unanswered | Not a gap; human item 1 | Remedy was maintainer-approved; verifier's independent proxy passes. |
| (e) advisory CodeQL red, 24 alerts | Not a gap | Not among the 44 required contexts (read from ruleset 20868126); advisory-only by documented project posture; red on pre-bump heads too. |
| (f) `[X]` later `[x]` | Cosmetic | The re-confirmation commit shows `[x]`. |
| (g) 47-REVIEW.md: 0 critical, 2 warning, 2 info | No gap | See Warnings. |

### Warnings (non-blocking)

- **WR-01 (from 47-REVIEW.md, confirmed by me):** `crates/paladin-memory/CHANGELOG.md` and `crates/paladin-content/CHANGELOG.md` say "No functional change" for 0.11.0, while the root CHANGELOG and MIGRATION 10.1 M-B-12 / 10.4 document a one-way persisted-key change (`paladin.node` to `paladin.data`, `ContentItem` entity key) that those crates read and write. The crate changelogs ship inside the published, immutable crates. Root and MIGRATION are accurate, so SC3's repo-level claim holds. Fix in the next patch release's changelog.
- **WR-02:** dangling `[Unreleased]: https://github.com/jamatulli/paladin/compare/v0.2.0...HEAD` link definition at `CHANGELOG.md:3396`, pre-existing, harmless.
- **A-SC3 reading:** the ROADMAP wording "publish-order gate green on the release run" is satisfied by the plan-recorded, flagged assumption that `release.yml` has no step of its own and relies on the green `ci.yml` step plus its own dependency-order publish job. I found no maintainer confirmation of this reading in the phase files. The evidence is strong (gate script green on three SHAs and locally, 12 of 12 published in order), but if the maintainer wants the literal wording, a future release could add the script as a `release.yml` step.
- **Minor doc inconsistencies:** the audit's sign-off line still says "Unticked, as designed" beside a ticked box; 47-REVIEW.md IN-01 quotes 108,951 bytes where the shipped extraction script gives 108,926 (the difference is a boundary convention); 47-READY-TO-CLOSE.md's figure of 108,926 is the correct one.
- **Sandbox limits recorded:** job logs unreadable; the Trusted Publishing `trustpub_data` per version and the per-crate `published-now` log table were not re-read by me; `gh release view` (GraphQL) blocked, REST used instead.

### Human Verification Required

1. **B-1 condensation read.** Read the condensed root `[0.11.0]` against the ledger and the original at `700a92d86`; answer `- B-1 maintainer read:`. Expected: no claim added, no breaking change or security note dropped. Why human: meaning, not identifiers.
2. **Coverage `Lines:` figure.** Copy it from job 121733645613 (run 40368801098) into the evidence file, per the pending todo. Expected: at or above 82 percent. Why human: log and artifact are behind a host this sandbox cannot reach.

### Gaps Summary

No gaps. All three ROADMAP success criteria are observably true against live GitHub and registry state, the tag sits on PR #59's merge commit with equal tree to the signed-off head, every required context is green on the merge commit, and SHIP-07 is accounted for. Status is `human_needed` solely because two non-blocking human items remain open, per the verification decision tree. Neither blocks proceeding to `phase.complete 47`, the milestone audit, or the close-out merge. The 13 open WINDOWS rows listed in 47-READY-TO-CLOSE.md all originate in other phases and are for the milestone audit.

---

_Verified: 2026-10-26T21:50:00Z_
_Verifier: Claude (gsd-verifier)_
