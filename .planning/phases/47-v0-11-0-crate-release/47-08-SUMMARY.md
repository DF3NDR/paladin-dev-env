---
phase: 47-v0-11-0-crate-release
plan: 08
subsystem: release
tags: [release, tag, merge-commit, equality-proof, crates-io, trusted-publishing, sparse-index, d-00h, d-00l]

requires:
  - phase: 47
    provides: "47-07: the maintainer's merge of PR #59 and the annotated tag v0.11.0 (reply `tagged ...` at 2026-10-26T20:32Z), and the post-tag resume condition"
provides:
  - "The proof that the annotated tag v0.11.0 (77a37c18) peels to 70657919beadbd341a72be2d0107080017a803f0 = PR #59 merge_commit_sha = origin/main, with the two expected parents; `- SHA-B:` in Provenance"
  - "chore/47-close (not pushed), cut from the tagged merge commit, holding the deferred plan 47-06 and 47-07 records, the Coverage todo and this plan's evidence"
  - "ci.yml run 40368801098 on the merge commit (39 jobs, 38 success, 1 skipped by design; End-to-End Tests and Publish Dry Run passing for the first time) and release.yml run 40396108246 per job"
  - "The formal registry table: 12 of 12 crates at 0.11.0, yanked false, 64-hex checksums, with trustpub_data naming DF3NDR/paladin-dev-env, run 40396108246 and the merge commit"
affects: [47-09, 47-10]

tech-stack:
  added: []
  patterns:
    - "Equality, not ancestry, binds a tag to the reviewed merge: peeled tag == GET /pulls/N merge_commit_sha == origin/main, plus a two-parent check"
    - "Publication is proven from the registry (sparse index and trustpub_data), never from the workflow log or the run's overall conclusion (D-00h)"

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-08-SUMMARY.md
    - .planning/phases/47-v0-11-0-crate-release/47-07-SUMMARY.md
    - .planning/phases/47-v0-11-0-crate-release/47-06-SUMMARY.md
    - .planning/todos/pending/2026-10-26-record-v0-11-0-coverage-lines-figure.md
  modified:
    - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md
    - .planning/phases/47-v0-11-0-crate-release/.continue-here.md
    - .planning/STATE.md
    - .planning/ROADMAP.md

key-decisions:
  - "Plan 47-08's `SHA-A2` (the merge's PR-side parent) is read as SHA-A4 72e13f95, the final head, as in plan 47-07"
  - "The 41-character first token of the maintainer's Task 3 reply is recorded verbatim and not used; every SHA is resolved from the tag itself"
  - "The release run is green, so the D-18 read-only-diagnosis branch did not apply; the plan ran to completion"

requirements-completed: []

duration: "about 12m of agent work (2026-10-26T20:43Z to about 20:55Z)"
completed: 2026-10-26
status: complete
---

# Phase 47 Plan 08: tag equality, the close-out branch, the post-merge and release runs, and the registry table Summary

**The annotated tag `v0.11.0` (object `77a37c1846e5ab6716a51b78fb39541bccd1a7ae`) peels to the merge commit `70657919beadbd341a72be2d0107080017a803f0`, which is PR #59's `merge_commit_sha` and `origin/main`; `release.yml` run 40396108246 and its `Publish to crates.io` job concluded `success`, and all twelve publishable crates read `0.11.0`, unyanked, from the crates.io sparse index with Trusted Publishing metadata naming that run and that commit.**

## Result

- **Equality (Task 1).** `git cat-file -t v0.11.0` is `tag`; tagger `Am0rfu5`, 1793046542 -0400 (2026-10-26T20:29:02Z), message `v0.11.0`. `git rev-parse 'v0.11.0^{commit}'` = `GET /pulls/59 .merge_commit_sha` = `origin/main` = `70657919beadbd341a72be2d0107080017a803f0` (SHA-B); PR #59 `merged: true`, `merged_at` 2026-10-26T17:27:08Z. Exactly two parents: `df24689c3d1fd52b0e730105c15b22c2c4ad1404` (the frozen `main`) and SHA-A4 `72e13f9583ffe2b89580049045ac094ed3d46dbf`. The merge tree equals SHA-A4's tree. The tagger date is 1 h 54 min after `ci.yml` concluded on the merge commit (18:35:02Z) and equals DATE `2026-10-26`; the twelve `## [0.11.0] - 2026-10-26` headings in the tagged tree read the same. No date mismatch.
- **Close-out branch (Task 1).** `chore/47-close` cut from `70657919` with `git checkout -b`; the six uncommitted entries carried across (diffstat identical before and after), committed in three groups (`a0b3fcf3` plan 47-06 close-out, `37708dbb` plan 47-07 record with `47-07-SUMMARY.md` and the Task 3 reply, `fd70e888` the Coverage todo). Stray-commit range `72e13f95..claude/laughing-dirac-e0h2ax` = 0. The remote release-branch ref no longer exists (404; deleted after the merge); local branch still `72e13f95`. Not pushed.
- **Post-merge and release runs (Task 2).** Six runs on `70657919`, all `success`: `ci.yml` 40368801098 (39 jobs, 38 `success`, 1 `skipped` by design `Benchmark Regression Signal (Non-Blocking)`; `End-to-End Tests` 121746301100 and `Publish Dry Run` 121737571718 ran for the first time on this tree and passed), `codeql.yml`, `feature-flags.yml` (20 jobs), `Docs`, `pre-commit`, and `release.yml` 40396108246. `Check publish order` step `success`; the A-SC3 sentence is recorded. `release.yml` per job, twelve rows: `Verify Tag From Main`, `Test Suite`, `Create Release`, `Pre-Publish Consistency Gate`, `Publish to crates.io` (121833090264), `Finalize Release Body`, four `Build Binaries` legs, `Build and Push Docker Images`, `Generate SBOM`: all `success`. **The publish job's conclusion is `success`, recorded on its own**, not inferred from the run. The publish job log is unreadable here (403 on the blob host), so the per-crate outcome table is not transcribed; the registry `created_at` order matches the `CRATES` array. GitHub Release object (read only): `Release v0.11.0`, not draft, not prerelease, body 109810 characters (cap 125,000), artifact marker present, 10 assets.
- **Registry (Task 3).** N derived from `cargo metadata` = **12** (`paladin-doc-examples` excluded, `publish = []`). Formal table: 12 rows, each `0.11.0`, a 64-hex checksum, `yanked` `false`, `LC_ALL=C` sorted; read 2026-10-26T20:47:01Z to 20:47:05Z, 4 min 41 s after the publish job's `completed_at`, re-read live at 20:50Z (12 of 12). `trustpub_data` on every crate: `provider` `github`, `repository` `DF3NDR/paladin-dev-env`, `run_id` `40396108246`, `sha` `70657919...`. No credential was used for any read.

## Task commits (all on `chore/47-close`, none pushed)

| Commit | What |
|---|---|
| `a0b3fcf3` | `docs(47-06)`: close out plan 47-06 (deferred) |
| `37708dbb` | `docs(47-07)`: the checks on SHA-A4, the one-way decision and the merge-and-tag reply, with `47-07-SUMMARY.md` (deferred) |
| `fd70e888` | `docs(47-07)`: the Coverage `Lines:` figure todo (deferred) |
| `f026d8e6` | `docs(47-08)`: Task 1, tag equality, Provenance `- SHA-B:`, resume file |
| `7ccb9af4` | `docs(47-08)`: Task 2, post-merge and release runs |
| `eddd9ac1` | `docs(47-08)`: Task 3, registry verification |

## Deviations from Plan

**1. [Plan text drift] `SHA-A2` read as SHA-A4.** The plan's two-parent check and remote-branch check name `SHA-A2`; the final head, and the merge's second parent, is SHA-A4 `72e13f95`. The automated verify of Task 1 was run against the `- SHA-A4:` line. Same substitution as plan 47-07.

**2. [Plan check made vacuous] The remote release branch was deleted.** The plan's "if it still exists, it still points at SHA-A2" check has nothing to check: `refs/heads/claude/laughing-dirac-e0h2ax` answers 404 on GitHub. Recorded as a finding; the commit is the merge's second parent and PR #59's recorded `head.sha`.

**3. [Maintainer paste defect] The Task 3 reply's first token has 41 characters** (stray leading `7`). Recorded verbatim in the resume file and the evidence Findings; nothing was resolved from it.

**4. [Unreadable source] The publish job's per-crate outcome table.** `GET actions/jobs/121833090264/logs` is forbidden from this sandbox (blob host). Not transcribed; per-crate state comes from the registry, as the plan prescribes.

## Outstanding, not blocking

- The `Coverage` job's printed `Lines:` figure against the 82 percent floor: still unread, tracked by `.planning/todos/pending/2026-10-26-record-v0-11-0-coverage-lines-figure.md` (recheck by 2026-11-02).
- The B-1 condensation read by the maintainer (`B-1 maintainer read: requested`).
- SHIP-07 is not ticked here; it closes in plan 47-09.

## Known Stubs

None. This plan edits release records only.

## Threat Flags

None. No file outside `.planning/` changed; no new endpoint, auth path or schema.

## Self-Check: PASSED

Commits `a0b3fcf3`, `37708dbb`, `fd70e888`, `f026d8e6`, `7ccb9af4`, `eddd9ac1` exist on `chore/47-close`; `git diff --name-only 70657919 HEAD` lists only `.planning/` paths; `47-06-SUMMARY.md`, `47-07-SUMMARY.md` and this file exist; `- SHA-B: 70657919beadbd341a72be2d0107080017a803f0` is in Provenance; 12 formal-table rows match the row pattern; the branch is not on the remote.
