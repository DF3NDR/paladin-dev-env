---
phase: 58-crate-release
plan: 13
subsystem: release
tags: [release, registry, trusted-publishing, smoke, ship-08, d-00c, d-00h, d-00l, d-00r, d-13]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-12 (SHA-B 251303bcd, annotated tag v0.11.1, release.yml run 42304251278)
provides:
  - "chore/58-close cut at SHA-B (local, not pushed) carrying the landed 58-11 and 58-12 records"
  - "two-SHA proof: annotated tag v0.11.1 peels to SHA-B = PR 64 merge_commit_sha = origin/main, parents 8b23cb4c5 and SHA-A2"
  - "58-CI-EVIDENCE.md `### Post-publish verification`: ci.yml rows on SHA-B, release.yml per-job rows, publish-crates.sh outcome table, the Release read, the twelve-row registry table with trustpub_data, the keyless smoke record"
affects: [58-14]

tech-stack:
  added: []
  patterns:
    - "The registry sparse-index table (with trustpub_data), not the workflow log, is the proof of publication"
    - "Consumer smoke in a scratch crate outside the repository: no [patch], no path dependency, offline mock adapter, no key"

key-files:
  created:
    - .planning/phases/58-crate-release/58-13-SUMMARY.md
  modified:
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md
    - .planning/phases/58-crate-release/.continue-here.md

key-decisions:
  - "Tag v0.11.1 is on the reviewed merge commit: equality with merge_commit_sha, containment in origin/main and the two-parent set all hold; no D-00n finding (tagger date 2026-11-06 = changelog headings)"
  - "The post-tick records land on chore/58-close, not the release branch, so the maintainer's tick stays that branch's last pushed commit"

requirements-completed: []

duration: "about 2026-11-06T22:36Z to 22:46Z"
completed: 2026-11-06
---

# Phase 58 Plan 13: Tag Proof, Release Run, Registry Table and Keyless Smoke Summary

**The annotated tag `v0.11.1` sits on the PR 64 merge commit `SHA-B` (`251303bcd`), all twelve `release.yml` jobs and the one `ci.yml` run on `SHA-B` are `success`, the GitHub Release is live with 10 assets and a 17049-character body, `58-registry-probe.sh --mode present --trustpub` reads 12 of 12 crates at `0.11.1` with a Trusted Publishing record naming run `42304251278` and `SHA-B`, and a keyless scratch consumer builds and runs the README Quick Example against the published crates.**

SHIP-08 is not ticked here: plan 58-14 ticks it from this evidence.

## Commits in this plan (on `chore/58-close`, nothing pushed)

| Task | Commit | Subject |
|------|--------|---------|
| 1 | `cd5aa21ed` | `docs(58-13): land the post-tick records on chore/58-close` |
| 2 | `8dab8eb4e` | `docs(58-13): record the post-merge and release runs and the v0.11.1 Release` |
| 3 | `8b825d825` | `docs(58-13): registry table, Trusted Publishing record and keyless smoke for v0.11.1` |

The summary and the STATE/ROADMAP edits follow in a fourth docs commit.

## Task 1 (tracer): the tag is the merge commit; the branch is cut; the records land

- `git cat-file -t v0.11.1` = `tag`; tag object `65b13ed08e4e0be06bc0db390e5f682748db6941`; tagger Am0rfu5, 2026-11-06 22:22:39 +0000, message `v0.11.1`.
- `git rev-parse 'v0.11.1^{commit}'` = `251303bcd143c5529c737afe8a2c61b05688f018` = `origin/main` = PR 64 `merge_commit_sha` (`merged: true`, `merged_at` 2026-11-06T18:36:15Z). Equality, not just ancestry; `git merge-base --is-ancestor` of it in `origin/main` holds.
- `git rev-list --parents -n 1`: exactly two parents, `8b23cb4c55a10fcf0b08fab1af595e1f184e5f2e` (old `main`) and `dffa996fa84c7b35c69f95987231426296f864b2` (SHA-A2). The merge commit's tree is identical to SHA-A2's (`git diff 251303bcd dffa996fa` empty).
- D-00n: all twelve changelog headings at `SHA-B` read `## [0.11.1] - 2026-11-06`; tagger date 2026-11-06. Match, no finding.
- Stray reconciliation (D-00l): `git rev-list --count dffa996fa..claude/laughing-dirac-e0h2ax` = 2 (`75f36c72d`, `83af6d02d`). `git diff dffa996fa 83af6d02d --stat -- .planning/` listed exactly the five files of those two commits (192 insertions, 20 deletions). Branch cut recipe, run in order: `git stash push -u -m "58-12 records"`; `git fetch origin --tags`; `git switch -c chore/58-close 251303bcd143c5529c737afe8a2c61b05688f018`; `git checkout 83af6d02d49ee7fa50a4b2fdd8e879e4ba7d71ca -- .planning/`; `git stash pop` (clean, no conflicts). The release branch stays at local `83af6d02d`, remote SHA-A2; it was never pushed, reset, rebased or rewritten.
- Landed in one commit: `58-11-SUMMARY.md`, `58-12-SUMMARY.md`, the plan 58-11 and 58-12 evidence rows, the STATE and ROADMAP edits, the resume file; Provenance gained `- SHA-B:` and the tick/merge/tag lines with the equality note and the tagger-date comparison. `<verify><automated>` passed.

## Task 2: post-merge and release runs, the Release

- `ci.yml` on `SHA-B`: exactly one run, push `42272241102`, `success`, `run_attempt` 1; 39 jobs (38 success, 1 skipped non-blocking benchmark); SDK smoke success. Four other push workflows on `SHA-B` (codeql `42272241068`, feature-flags `42272241351`, Docs `42272241496`, pre-commit `42272241069`) all `success`.
- `release.yml` run `42304251278` (push, `head_branch` `v0.11.1`, `run_attempt` 1, created 22:22:45Z, completed `success` 22:35:11Z): 12 of 12 jobs `success`: Verify Tag From Main `128189466796`; Pre-Publish Consistency Gate `128189540098`; Create Release `128189540127`; Test Suite `128189540148`; Generate SBOM `128189620139`; Build Binaries x4 (`128189620230` linux-arm64, `128189620269` macos-arm64, `128189620280` macos-amd64, `128189620400` linux-amd64); Build and Push Docker Images `128189620342`; Publish to crates.io `128191963842`; Finalize Release Body `128194658510`.
- The `publish-crates` job log was readable (`gh api .../actions/jobs/128191963842/logs`). Its `publish-crates.sh` outcome table: 12 rows, all `published-now` (`paladin-ai-core`, `paladin-ports`, `paladin-herald`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`, `paladin-notifications`, `paladin-content`, `paladin-storage`, `paladin-eval`, `paladin-ai`). The OIDC `rust-lang/crates-io-auth-action@v1` step retrieved a Trusted Publishing token and the post-job step revoked it.
- Manual steps between tag and publish: none (push-triggered, `run_attempt` 1, no `workflow_dispatch`, six seconds from tag to run start).
- GitHub Release `v0.11.1`: name `Release v0.11.1`, `draft` false, `prerelease` false, published 2026-11-06T22:23:05Z; body 17049 characters / 17059 bytes against the 125,000-character limit; truncation sentinel `<!-- paladin:release-truncated -->` absent (the guard's first live run was the no-op path); `<!-- paladin:release-artifacts -->` marker exactly once; last body line (the footer `finalize-release-body.sh` writes) `Measured image size: 33 MB -- within target (500 MB target, advisory only).`; 10 assets, all `uploaded`: four tarballs (18.5 to 23.4 MB), four `.sha256` (93 bytes each), `paladin-v0.11.1.cdx.json` (499438 bytes), `SHA256SUMS` (372 bytes).
- No red job, no partial publish, no missing Release, so D-00s did not trip.

## Task 3: registry table and keyless smoke

- Probe `bash .planning/phases/58-crate-release/58-registry-probe.sh --version 0.11.1 --mode present --trustpub` at 2026-11-06T22:40:13Z-22:40:17Z, exit 0, last line `probe: 12 crate(s), mode present, version 0.11.1: OK`. **Registry: 12/12 crates at 0.11.1**, N = 12 derived from `cargo metadata`, one `0.11.1` line per crate, `yanked` false, 64-hex checksums, `trustpub_data` provider `github`, repository `DF3NDR/paladin-dev-env`, `run_id` `42304251278`, `sha` `251303bcd143c5529c737afe8a2c61b05688f018` in every row. Only the mandatory `User-Agent` header was sent; no credential.
- Smoke: scratch crate `paladin-smoke-0-11-1` under `/tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/smoke-0.11.1/` (not a git repository, outside the workspace); `cargo add paladin-ai@0.11.1`, `cargo add tokio --features macros,rt-multi-thread`; `src/main.rs` is the README Quick Example from `README.md` at `SHA-B`; no `[patch]`, no `path` dependency, no workspace inheritance. `cargo build` (debug, scratch `CARGO_TARGET_DIR`) succeeded in 1m 54s with 0 warnings; `cargo run` exit 0 printed `Hello from Paladin!` (the mock's reply). `Cargo.lock` and `cargo tree`: `paladin-ai`, `-ai-core`, `-battalion`, `-herald`, `-llm`, `-memory`, `-ports`, `-storage`, all `0.11.1` from `registry+https://github.com/rust-lang/crates.io-index`. No token, no credentials file and no provider key existed in the environment.
- Disk: free space was 6.6 GB before (above the 4 GB precondition); the scratch `target/` (2.2 GB) was deleted after recording, and `df -h /` read 6.5 GB available afterwards (4.4 GB with the target present). `target/debug/incremental` of the repository was not touched.

## Deviations from Plan

None - plan executed exactly as written. The orchestrator's branch-cut recipe (stash, switch, checkout `.planning/` from `83af6d02d`, pop) is the plan's "stash, switch, pop if needed" with the stray-commit re-application; it is recorded in the Task 1 commit body, the resume file and above. Two bookkeeping choices, both inside the plan's scope: the registry subsection sits first inside `### Post-publish verification` and the sub-parts use `####` headings, so the Task 3 `sed` range reads the table rows; and `requirements-completed` is left empty because plan 58-14 owns the SHIP-08 tick.

## Auth gates

None. No credential was requested or used (P-08-3, D-00g).

## Known Stubs

None.

## Threat Flags

None. This plan added no code, endpoint, auth path or trust-boundary schema change; it only read the registry, the GitHub API and the release.

## Next

Plan 58-14 Task 1 on `chore/58-close`: the SHIP-08 tick from the 12/12 registry read and the smoke, the web3sec and ready-to-close hand-offs, and the documentation-only pull request. Plan 58-14 pushes `chore/58-close`; the release branch is not pushed.

## Self-Check: PASSED

- Files exist: `58-13-SUMMARY.md`, `58-CI-EVIDENCE.md`, `.continue-here.md`, `58-11-SUMMARY.md`, `58-12-SUMMARY.md` in `.planning/phases/58-crate-release/`.
- Commits exist on `chore/58-close`: `cd5aa21ed`, `8dab8eb4e`, `8b825d825` (checked with `git log`).
- All three `<verify><automated>` blocks printed their pass marker; `chore/58-close` merge-base with `SHA-B` is `SHA-B`.
- Nothing pushed; the release branch `claude/laughing-dirac-e0h2ax` is at local `83af6d02d`, remote SHA-A2, untouched.
