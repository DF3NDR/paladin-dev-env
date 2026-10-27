# Phase 47 — Ready to Close

**Phase:** 47 v0.11.0 Crate Release
**Status:** Ready to close. No plan in this phase runs `/gsd-audit-milestone` or
`/gsd-complete-milestone` — this document is the hand-off; the maintainer runs both, when ready,
against what is enumerated below.

Written 2026-10-26 on `chore/47-close` (cut from the tagged merge commit
`70657919beadbd341a72be2d0107080017a803f0`). Every `Observed` line below points at the section of
`47-CI-EVIDENCE.md` that proves it; where a figure was re-read at write time, the time of the read
is stated beside it.

---

## Success Criteria — restated against what was observed

### SC1 — All publishable crates live on crates.io at 0.11.0

**Observed: TRUE.** All 12 publishable crates (derived from `cargo metadata`, not hard-coded;
`paladin-doc-examples` carries `publish = []` and is excluded) read `0.11.0` on the crates.io sparse
index with `yanked: false` and a 64-hex checksum, and every version's `trustpub_data` names
`DF3NDR/paladin-dev-env`, run `40396108246` and the merge commit — they were published by the release
run for the tag through Trusted Publishing, not by a user token (`published_by` is `null`).

- **Live re-read made while writing this document: 2026-10-26T21:06:04Z to 21:06:07Z, 12 of 12.** One
  `GET https://index.crates.io/<path>` per derived crate with the mandatory `User-Agent` and no
  credential: HTTP 200 for all twelve, exactly one `0.11.0` line per crate, `yanked` false for all
  twelve, checksum length 64 for all twelve. (The table's own read is 2026-10-26T20:47:01Z to
  20:47:05Z, re-read at 20:50Z in plan 47-08 and at 20:53:30Z in plan 47-09; this one is the fourth.)
- Not claimed: the per-crate `published-now` / `already-at-this-version` table of the publish job's
  log. The log answers 403 from this sandbox and was not transcribed. The registry rows, not the
  workflow log, are the proof (D-00h).
- Evidence: `47-CI-EVIDENCE.md` § "Registry verification (D-00h)" → "Post-publish verification
  (plan 47-08, 2026-10-26T20:47Z)" (the formal table and the Trusted Publishing proof) and § "CI-run
  table" → "Plan 47-08 — the post-merge runs on SHA-B and the `release.yml` run for the tag".
  Requirement `SHIP-07` was ticked on that table (plan 47-09).

### SC2 — The tag is cut on a main merge commit, per the two-SHA rule

**Observed: TRUE.** The annotated tag `v0.11.0` (tag object `77a37c1846e5ab6716a51b78fb39541bccd1a7ae`)
peels to `70657919beadbd341a72be2d0107080017a803f0` (`SHA-B`), which is byte-identical to PR #59's
`merge_commit_sha` and to `origin/main`; that commit has exactly two parents, `df24689c3d1fd52b0e730105c15b22c2c4ad1404`
(the previous `main` tip, the D-02 base) and `72e13f9583ffe2b89580049045ac094ed3d46dbf` (`SHA-A4`,
PR #59's final head), so it is a merge commit and not a squash or a rebase. Containment alone was
rejected; the three values were compared for equality.

- **Which SHA is "the signed-off PR head".** The plan's wording is `SHA-A2` (`6ef9207a672cceb8aaddc2dccdd9986c33d5366b`,
  the maintainer's first tick, 2026-10-26T02:18:19Z). The redate (`SHA-A3` `02f6ee2e7aa1dea9a349444a7aa53ba41ab01ca5`)
  and the maintainer's re-confirmation (`SHA-A4`) came after it, so the signed-off head the merge took
  is `SHA-A4`. The merge tree `885e4ac570b1810eb5f96ded84853adfa297ec7b` equals SHA-A4's tree: `main`
  contributed no content change. This drift is recorded in Provenance ("SHA-A4 role").
- Order: `ci.yml` concluded `success` on the merge commit at 2026-10-26T18:35:02Z; the tag was
  created at 20:29:02Z, 1 h 54 min later, on DATE (2026-10-26).
- Live re-read while writing (2026-10-26T21:06Z): `git ls-remote origin 'refs/tags/v0.11.0*'` prints
  the tag object `77a37c18` and the peeled `70657919`; `refs/heads/main` is `70657919`; `GET /pulls/59`
  reads `merged: true`, `merge_commit_sha` `70657919`; `git rev-list --parents -n 1` prints the commit
  and exactly the two parents above.
- Evidence: `47-CI-EVIDENCE.md` § "Provenance" (`SHA-A2`, `SHA-A4`, `SHA-B` and their roles) and
  § "CI-run table" → "Plan 47-08 — the tag sits on the merge commit" (the equality proof).

### SC3 — CHANGELOG and MIGRATION complete; publish-order gate green on the release run

**Observed: TRUE.** The root `CHANGELOG.md` `[0.11.0]` entry is condensed under B1=a (the full uncondensed text is at `SHA-PRE-B1` `700a92d86f3d6b2e24bbd2c163daec7c307297e0`, the body is 108,926 bytes and the maintainer's read of the condensation is still unanswered, see the open items), the eleven crate changelogs are stamped, `MIGRATION.md` has no `TBD`, and the publish-order gate is green on the release path.

- **Re-seal row 1** (`47-ACCEPTANCE-AUDIT.md`): `MIGRATION.md` `TBD` count 0; sections 9.2 and 10.2 read together give 56 `crate|type` pairs, set-equal in both directions with `.cargo/semver-checks-allowlist.toml`. At the tag, `git show 70657919:MIGRATION.md | grep -c TBD` prints 0 (re-read at write time). Row 1 has no pull-request twin (`check-migration-allowlist` is not in `ci.yml`, D-14); the local row is its only proof.
- **Re-seal row 7**: `make check-gates` (nine offline guards) exit 0; in each of the 12 changelogs `## [Unreleased]` count 0 and `## [0.11.0] - 2026-10-26` count 1 as the first `## [` heading (re-read at write time on the tagged tree: all 12 headings read `## [0.11.0] - 2026-10-26`, root `[Unreleased]` count 0); release body 108,926 bytes against the 110,000 limit (`scripts/extract-changelog-section.sh --version 0.11.0` re-run at write time: 108926).
- **Release body record (plan 47-08):** the GitHub Release for `v0.11.0` has a body of 109,810 characters (cap 125,000) with the `<!-- paladin:release-artifacts -->` marker present; `Create Release` and `Finalize Release Body` both `success`. This closes B-1 at run level. The object was read only, never edited.
- **A-SC3 reading — the publish-order gate:** job `License & Dependency Policy` (121733645712, `success`), step 11 `Check publish order` `success`, on the `ci.yml` run 40368801098 for `SHA-B`; the same step was `success` on `SHA-A0` and on `SHA-A4` (both `ci.yml` runs). `release.yml` run 40396108246 has no separate publish-order step: its `Pre-Publish Consistency Gate` (121829552774) requires that `ci.yml` run to have concluded `success` on the tagged commit and concluded `success`, and `Publish to crates.io` (121833090264) concluded `success` on its own. The run's overall `success` is not offered as publish health; SC1's registry table is.
- Evidence: `47-ACCEPTANCE-AUDIT.md` rows 1, 7 and 8; `47-CI-EVIDENCE.md` § "B-1 condensation ledger", § "CI-run table" → "Plan 47-08 — the post-merge runs on SHA-B and the `release.yml` run for the tag".
- What is and is not proven: the mechanical proxy of the condensation (all 43 distinct `M-B-` / `DR-` / `ADR-` ids kept, figures and code spans subsets of the original, the six preserved subsections byte-identical) passed. That the condensed text adds no claim beyond the proxy is a human judgment, requested in `47-CI-EVIDENCE.md` (`- B-1 maintainer read: requested`) and not yet answered; it is carried as an open item below and is neither waived nor presumed.

---

## Requirement

`SHIP-07` is ticked (`- [x] **SHIP-07**` in `REQUIREMENTS.md`, traceability row `Complete`) through
`requirements.mark-complete`, after a live 12-of-12 sparse-index re-read (plan 47-09, D-16). The
ROADMAP Phase 47 checkbox and Progress row are applied by `phase.complete 47` at phase close and are
still unticked.

---

## Open items — stated as open, neither blockers invented nor waived

1. **The `Coverage` job's printed `Lines:` figure against the 82 percent floor (ADR-0006).** Unread; no number is recorded anywhere. The job (121733645613 on the merge commit) concluded `success` under `cargo llvm-cov --fail-under-lines 82`, which is the floor gate's conclusion only. This sandbox cannot read job logs or the `coverage-summary` artifact (blob host blocked). Tracked by `.planning/todos/pending/2026-10-26-record-v0-11-0-coverage-lines-figure.md` (owner the maintainer, recheck by 2026-11-02); otherwise it is picked up at Phase 47 verification.
2. **The B-1 condensation read** (`B-1 maintainer read: requested`, plan 47-02 Task 3): unanswered. Bears on SC3's wording above.
3. **The advisory `CodeQL` results check** was `failure` on the pull-request heads (24 high alerts, check run 121611604849, not a required context, advisory-only by project posture); not triaged.
4. **web3sec's re-pin, gates and return report** (D-15): `47-HANDOFF-web3sec.md` is written; the folded todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md` stays pending until web3sec returns.
5. **Sandbox-attributed re-seal legs** (decided by the maintainer in plans 47-04 and 47-06): row 4 `paladin-web` and row 5 MSRV and row 6's `make test` leg rest on their CI twins (`Semver Checks (vs v0.9.0)`, `MSRV (Rust 1.88)`, `Unit Tests (stable)`), all `success` — not local passes.

---

## Carried findings for the next milestone's first phase

Recorded in `47-CI-EVIDENCE.md` § "Findings carried forward (D-00d)"; none is fixed here (D-00d). Each is one line:

- **D-09:** re-pin the CI `cargo-semver-checks` baseline from `0.9.0` to `0.11.0` and add `paladin-eval` (the job is a known no-op, `0 checks: 0 pass, 254 skip`; its display name `Semver Checks (vs v0.9.0)` also carries a version).
- **D-14 (1):** wire `check-migration-allowlist` into `ci.yml` so re-seal row 1 has a pull-request twin.
- **D-14 (2):** run `Publish Dry Run` and `End-to-End Tests` on pull requests (both are `main`-push only; they executed for the first time on this tree on the merge commit, and passed).
- **B-1 root cause:** the release-body payload builder still fails on an argument larger than the 131,072-byte cap (`jq: Argument list too long`); v0.11.0 passed by condensing the changelog, the cap itself was not changed.
- Stale `version: v0.10.0` labels in `k8s/server/worker-deployment.yaml` (lines 20 and 35).
- Stale "ten crates" count comments in `release.toml` (lines 41 and 47), `scripts/finalize-crate-changelogs.sh:8` and `.github/workflows/ci.yml:277`.
- `make finalize-crate-changelogs` takes no date argument (the script's `--date` flag does), and `release.toml` records `cargo-release 1.1.2` where 1.1.7 was used.
- **CONTEXT deferred ideas:** a dry-runnable `release.yml` against a bare SHA before a tag exists; a consumer smoke test (`cargo add paladin-ai@0.11.0` in a scratch project) as a standing post-release check; the path-only rule for workspace dev-dependencies with a lint enforcing it.
- **Carried from Phases 36.1 / 37 / 37.1, unchanged:** `cargo test --workspace --all-features` in CI; removing the eight crate-level rustdoc allows; regenerating `doc-coverage-report.md`.

---

## Prerequisites for /gsd-audit-milestone v0.11.0

- **The audit's current reading:** `.planning/v0.11.0-MILESTONE-AUDIT.md` (2026-10-24) is `gaps_found`, 62/66 requirements, with `SHIP-07` its only unsatisfied requirement. `SHIP-07` is now ticked; the audit file is not edited by this phase and the maintainer's run re-reads it.
- **Phase 47 has no `VERIFICATION.md` yet.** The audit flags a phase without one as unverified (it read 14/15 phases for that reason). Phase verification and `phase.complete 47` are due before or at the audit; they commit on `chore/47-close` and need one more push.
- **Seam 8 (API-surface baseline currency, PARTIAL in the audit)** is answered by `API Surface Tracking` `success` on the release heads: on `SHA-A0` (job 121290924875, pull-request run 40233745953), on `SHA-A4` (the final head and the plan's `SHA-A2` role; `ci.yml` runs 40332190995 and 40332201657, both listed with the job `success` in the 43-context tally) and on `SHA-B` (job 121733645568, run 40368801098). The committed baseline did not move after the version bump.
- **Open WINDOWS rows** (`windows status`, re-read at write time: 13 open, 39 waived, 30 fixed of 82, `last_updated` 2026-10-24T13:47:20Z). Open ids: **56, 58, 62, 63, 64, 65, 66, 67, 68, 69, 73, 77, 79** — kinds `lint-warning` (56), `deviation` (58, 62-67, 77, 79) and `unrun-verify` (68, 69, 73), from phases 38, 40, 41, 42, 48, 50 and 50.1. **None is Phase 47's**; Phase 47 appended no row. Each is to be dispositioned (fixed or waived with a reason) or recorded as an override at the audit. `/gsd-ship` blocks on open rows and was not used by this phase because PR #59 already existed. Row 73 (`run_api_wiring`, two worker-deadline tests red in this sandbox) is the precedent for re-seal row 6.
- The open items above (Coverage `Lines:` figure, B-1 read) are for the audit to record honestly; they are not blockers this document invents or waives.

---

## Prerequisites for /gsd-complete-milestone v0.11.0

- Run it **on `chore/47-close`** so the archive moves ride this pull request.
- It inserts its own `## v0.11.0 ... (Shipped: ...)` entry in `MILESTONES.md`; **fold the plan 47-09 release-record block into it** (that block says as much: "fold this release record into it then").
- **The `v0.11.0` tag already exists** (object `77a37c18`, on `70657919`) and must not be created again; the workflow's tag step has a skip path for an existing tag.
- The folded web3sec todo stays in `todos/pending/` until web3sec returns (D-15).
- The phase-verification and `phase.complete 47` commits land on `chore/47-close` and need **one more push** before the pull request is merged.
- Nothing outside `.planning/` may change on this branch (asserted against the tag before the push).

---

## The maintainer's remaining acts, in order

1. Run phase verification for Phase 47 and `phase.complete 47` on `chore/47-close`, then push `chore/47-close`.
2. Run `/gsd-audit-milestone v0.11.0` (disposition or override the 13 open WINDOWS rows; record the two open items).
3. Run `/gsd-complete-milestone v0.11.0` on `chore/47-close` (fold the 47-09 release record into its entry; do not re-create the tag), then push.
4. Merge the docs-only pull request (`chore/47-close` → `main`) **with a merge commit**.

This phase performed none of these four steps and merged nothing.

---

## The close-out pull request

- **Pull request:** [#60](https://github.com/DF3NDR/paladin-dev-env/pull/60), `docs(47): v0.11.0 release records and close-out`, `chore/47-close` → `main`, open, not a draft, opened 2026-10-26 through REST (`gh api -X POST repos/DF3NDR/paladin-dev-env/pulls`, body from a file) after the maintainer's `push ok`.
- **Documentation-only:** `git diff --name-only v0.11.0 HEAD -- . ':!.planning'` printed nothing before the push, and `GET /pulls/60/files` lists no path outside `.planning/` (14 changed files at the read).
- **Not done by this plan:** merged, auto-merge, labelled, reviewers requested, tagged, dispatched, published, yanked, or touched the GitHub Release. `mergeable_state` reads `blocked` (required contexts and the ruleset, not a defect of the branch).
- **Detail and the read-back:** `47-CI-EVIDENCE.md` § "Findings carried forward" → "Plan 47-10: close-out branch pushed, documentation-only pull request opened". Commits added after the open (this record, the SUMMARY and the tracking files) ride the same branch and are pushed before this plan returns.

---

*Phase: 47-v0-11-0-crate-release*
*Written: 2026-10-26*
