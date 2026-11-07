# Phase 58 — Ready to Close

**Phase:** 58 Crate Release (milestone v0.11.1 Dogfood Cleanup)
**Status:** Ready to close. No plan in this phase runs `/gsd-audit-milestone` or
`/gsd-complete-milestone`; the agent ran neither (D-00l). This document is the hand-off: the maintainer runs them,
when ready, against what is enumerated below.

Written 2026-11-06 on `chore/58-close` (cut from the tagged merge commit
`251303bcd143c5529c737afe8a2c61b05688f018`, `SHA-B`). Every `Observed` line points at the section of
`58-CI-EVIDENCE.md` (or another file) that proves it; where a figure was re-read at write time, the time of the read
is stated beside it.

---

## The five ROADMAP success criteria, read against what was observed

### Success criterion 1 — version strings, manifests, changelogs and the gates on the release commit

**Observed: TRUE.** The workspace, the twelve publishable manifests and the twelve changelogs read `0.11.1`
(changelog headings `## [0.11.1] - 2026-11-06`, tagger date 2026-11-06, no D-00n finding). The eight-row re-seal on `SHA-R`
(`48146bc99`) recorded no failing row, and the CI-evidence entries `make clean-code`, `make security`,
`make api-surface` and `make check-version-strings` (`OK: 128 file(s) scanned, every copyable version string equals
'0.11.1'`); each has a CI twin that concluded `success`. On the merge commit `SHA-B` the single `ci.yml` run
`42272241102` is `success` (38 success, 1 skipped non-blocking benchmark, 0 red), and `SHA-B`'s tree is identical to
the signed-off head `SHA-A2` (`git diff 251303bcd dffa996fa` is empty).

- Evidence: `58-ACCEPTANCE-AUDIT.md` (the eight-row table and "CI-evidence entries"); `58-CI-EVIDENCE.md`
  `### Plan 58-07 — the bump set (D-14)` and `### Post-merge ci.yml runs on SHA-B`.
- Not claimed: the MSRV floor and the full local `make test` leg were CI-attributed by decision (D-00e), not local
  passes; the acceptance audit says so row by row.

### Success criterion 2 — `MIGRATION.md` states the consumer upgrade story

**Observed: TRUE.** `MIGRATION.md` section 11.8 carries `### Library consumers: v0.11.0 → v0.11.1`, in the tagged
tree: it states that a `0.11.0` consumer takes `0.11.1` without changing source, with the one serialisation caveat
(the credential fields of nine config types are no longer written, `M-B-50`, `M-B-51`), section 11.2 has no row and
the allowlist has no entry. The pointer-list wording is the maintainer's `WORDING=pointer-list` decision.

- Live re-read while writing (2026-11-06T22:46Z): `git show v0.11.1:MIGRATION.md` holds the heading line.
- Evidence: `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)` (WORDING rule) and `### D-11 semver on SHA-R`
  (`0 failure(s)` for all twelve packages at `--release-type patch`, 223 checks each).

### Success criterion 3 — the milestone audit reads `passed`; `windows status` reads `open_count: 0`; no undispositioned review warning

**Observed: TRUE.** `.planning/v0.11.1-MILESTONE-AUDIT.md` reads `status: passed` (Phases 52-57, 49 of 50
requirements satisfied and FIXES-04 re-homed with an owner and a date); it is in the tagged tree
(`git show v0.11.1:.planning/v0.11.1-MILESTONE-AUDIT.md`, 2026-11-06T22:46Z). `.planning/WINDOWS.md` reads
`open_count: 0`. The pre-tag disposition register `58-AUDIT-DISPOSITIONS.md` re-ran at write time:
`register: checked 135 row(s) (R: 95, V: 40); fixed 52, waived 37, re-homed 46; 0 failure(s)`. Every re-homed row
has an owner, the date `2026-11-09` and a pending todo, and all 44 pending todos carry `recheck_by: 2026-11-09`.

- Evidence: `58-AUDIT-DISPOSITIONS.md`; `58-CI-EVIDENCE.md` `## Audit sweep (D-06)` and
  `## Maintainer decisions (58-10)` (`AUDIT=validate-first`).
- Not claimed: the audit is the pre-tag read, scoped to Phases 52-57 by design; SHIP-08/09/10 are proven by the
  post-tag reads below, not by that audit.

### Success criterion 4 — `v0.11.1` tagged on `main`, shipped through `release.yml` with no manual step; semver non-zero

**Observed: TRUE.** The annotated tag `v0.11.1` (object `65b13ed08e4e0be06bc0db390e5f682748db6941`) peels to
`251303bcd143c5529c737afe8a2c61b05688f018`, which is byte-identical to PR #64's `merge_commit_sha` and to
`origin/main`; the commit has exactly two parents (`8b23cb4c5` old `main`, `dffa996fa` `SHA-A2`). Equality, not
ancestry, was compared. Release run `42304251278` (event `push`, `run_attempt` 1, never re-run, no
`workflow_dispatch`) concluded `success` with all 12 jobs `success`; `publish-crates` printed twelve `published-now`
rows with an OIDC-minted token; no changelog condensation, no hand-run publish. The semver job on the release pull
request read `223 checks: 223 pass, 31 skip` per package (non-zero).

- Evidence: `58-CI-EVIDENCE.md` `## Provenance` ("Tick, merge and tag"), `### Post-publish verification`
  (`#### The tag-triggered release.yml run, per job`), and `### Plan 58-10 — every run on SHA-A0 (labelled)`
  ("Semver (success criterion 4)").

### Success criterion 5 — twelve crates on crates.io at `0.11.1`, unyanked, Trusted Publishing; GitHub Release with assets

**Observed: TRUE.** All 12 publishable crates (derived from `cargo metadata`, not typed) read `0.11.1` on the
crates.io sparse index with `yanked: false` and a 64-hex checksum, and every version's `trustpub_data` names provider
`github`, repository `DF3NDR/paladin-dev-env`, run `42304251278` and `SHA-B`. The GitHub Release `v0.11.1` exists,
is neither draft nor prerelease, has a body of 17049 characters (limit 125,000, truncation sentinel absent) and ten
assets. A keyless scratch crate resolved the facade and its seven siblings at `0.11.1` from crates.io, built in
1m 54s and printed `Hello from Paladin!`.

- **Live re-read made while writing this document: 2026-11-06T22:46:59Z, 12 of 12** (`58-registry-probe.sh
  --version 0.11.1 --mode present`, last line `probe: 12 crate(s), mode present, version 0.11.1: OK`), immediately
  before `SHIP-08`, `SHIP-09` and `SHIP-10` were ticked through `requirements.mark-complete`. The table's own read is
  2026-11-06T22:40Z.
- Evidence: `58-CI-EVIDENCE.md` `### Post-publish verification` (`#### Registry read`, `#### The GitHub Release
  v0.11.1`, `#### Keyless consumer smoke on the published 0.11.1`).
- Not claimed: the smoke covers eight of the twelve crates through `paladin-ai`'s default closure; the other four
  (`paladin-content`, `paladin-eval`, `paladin-notifications`, `paladin-web`) are covered by the registry table.

---

## Requirements

`SHIP-08`, `SHIP-09` and `SHIP-10` are ticked (`- [x]` in `REQUIREMENTS.md`, traceability rows `Complete`) through
`requirements.mark-complete`, after the live 12-of-12 re-read above, the recorded smoke, the §11.8 subsection in the
tagged tree and the `passed` audit in the tagged tree (plan 58-14, P-08-2). No sign-off box was touched (D-00a). The
ROADMAP Phase 58 checkbox and Progress row, and `phase.complete 58`, are not applied by this plan beyond the plan
58-14 line.

---

## Prerequisites for /gsd-audit-milestone v0.11.1

An optional post-tag re-run; the pre-tag audit already reads `passed`.

- **Current reading:** `.planning/v0.11.1-MILESTONE-AUDIT.md` (2026-11-06) is `passed`. Its out-of-scope rows,
  `SHIP-08/09/10`, now read satisfied in `REQUIREMENTS.md` on the registry table, the smoke and the tagged tree, as
  the v0.11.0 precedent did after its tag. A re-run should read 52 of 53 requirements satisfied, with FIXES-04 still re-homed
  (register row R-13).
- **Phase 58 has no `VERIFICATION.md` yet.** The audit flags a phase without one as unverified; phase verification
  and `phase.complete 58` are due before or at the re-run and commit on `chore/58-close`.
- **`WINDOWS.md`** reads `open_count: 0` (re-read at write time); keep it so.
- **F-58-08** (the advisory CodeQL check on `SHA-A0`, eight `rust/cleartext-logging` alerts assessed as false
  positives) is a dispositioned tool signal, not a gap; the GitHub-side dismissal was taken on the maintainer's word
  (`58-CI-EVIDENCE.md` `## Plan 58-11`), because this sandbox cannot read it.

## Prerequisites for /gsd-complete-milestone v0.11.1

- Run it **on `chore/58-close`** so the archive moves ride the close pull request.
- It inserts its own `## v0.11.1 ... (Shipped: ...)` entry in `MILESTONES.md`; **fold the plan 58-14 release-record
  block into it** (the block says as much).
- **The `v0.11.1` tag already exists** (object `65b13ed08`, on `251303bcd`) and must not be created again.
- The close PR is open and documentation-only (`git diff --name-only origin/main...chore/58-close` is entirely under
  `.planning/`); any commit added to the branch needs one more push before the merge.
- The **44 pending todos** all carry `recheck_by: 2026-11-09`; the milestone close records them as the acknowledged
  carry-over. The `Pending Todos` list in `STATE.md` predates this milestone and is stale (three of its four files are
  already in `todos/completed/`); the close refreshes it.
- Nothing outside `.planning/` may change on this branch.

---

## The maintainer's remaining acts, in order

This phase performed none of these and merged nothing.

1. **Optional:** run `/gsd-audit-milestone v0.11.1` (see its prerequisites above), after phase verification and
   `phase.complete 58` on `chore/58-close`.
2. Run `/gsd-complete-milestone v0.11.1` on `chore/58-close` (fold the plan 58-14 release record into its entry; do
   not re-create the tag), then push.
3. **Merge the close pull request** (`chore/58-close` into `main`) **with a merge commit**.
4. **The key-rotation and purge rows, with their dates.** Nothing is left to do, and nothing was done by an agent:
   R-84 is `fixed` on the maintainer's own statement that the key was rotated on **2026-11-05** (not independently
   re-provable from the repository), and R-85 is `waived` under `PURGE=none` (the literal in history since
   `928c6d5f9` is inert once rotated; a rewrite would force-push `main`). Reopen them only if that statement was
   wrong.
5. **Codecov (R-72)** by `2026-11-09`: supply `CODECOV_TOKEN` and set `fail_ci_if_error`, or drop the upload step
   (todo `2026-11-05-codecov-upload-fails-silently.md`). The 82 percent floor gate is unaffected.
6. **The v0.10.0 sign-off boxes (R-73)** by `2026-11-09`: the eight unticked boxes in
   `.project/v0.10.0/09-program-acceptance-audit.md`, ticked by hand or annotated as superseded by section 13 (a
   human tick, never an agent's; todo `2026-11-05-section-11-judgment-tier-signoff-boxes.md`).
7. **Any `gh release edit`:** none is needed. The Release body is 17049 characters with no truncation and ten assets;
   edit it only if you want the wording changed.
8. **Option A (patch release through the same pipeline) if anything is partial:** nothing is partial. All twelve
   crates are published and unyanked, so there is no orphaned version to yank.
9. The remaining 44 pending todos by `2026-11-09` (re-check dates, owners `repo maintainer` or `v0.12.0 Phase 1`),
   starting with R-78, the semver-baseline re-pin to `0.11.1` (the baseline site, the job name and the ruleset
   context are renamed together).

---

*Phase: 58-crate-release*
*Written: 2026-11-06*
