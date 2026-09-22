# Release Recovery

> **When to use this document:** a release that stopped partway through the tag→publish
> pipeline, or a gate failure blocking `publish-crates` from running at all. It is the runbook
> for [Release Automation](release-automation.md) (the tool and the happy path) and
> [Release Checklist](release-checklist.md) (the end-to-end process). Read this after the job
> log, not instead of it — the vocabulary here matches the outcome table and gate messages an
> operator will actually be holding.

**Status: tested (2026-08-30).** This procedure has been exercised against a real induced
partial-publish failure — twice — and completed via the recovery steps documented below with no
deviation from them. The full rehearsal record, including three per-crate registry snapshots,
both runs' per-crate outcome tables, and independent OIDC-provenance verification, is in
`.planning/phases/20-release-pipeline-recovery-idempotent-re-runs-and-a-pre-publi/20-RECOVERY-EVIDENCE.md`.
Run URLs:
[33210072054](https://github.com/DF3NDR/paladin-dev-env/actions/runs/33210072054) (`v0.8.1-rc.3`,
pre-Phase-20 pipeline) and
[33322587044](https://github.com/DF3NDR/paladin-dev-env/actions/runs/33322587044) (`v0.8.1-rc.4`,
Phase 20's own gate and recovery scripts, live).

The rehearsal proved three procedure points beyond what the sections below already documented:

1. **Release commits travel via PR, not a direct push to `main`.** The repository's branch
   ruleset requires a pull request plus all required checks; `make release`'s direct
   `git push origin HEAD` is rejected. Cut the release commit on a branch, open a PR, and only
   push the tag once that PR has landed as a **merge commit** on `main` — a squash or rebase
   merge would orphan the tag from the history it claims to describe.
2. **An unpublished tag may be moved; a published version never may.** If a gate failure blocks
   every attempt before the first `cargo publish` runs, nothing has reached the registry yet, and
   re-tagging after a fix is safe — the rehearsal did this twice, recorded in
   `20-RECOVERY-EVIDENCE.md`'s Findings 5 and 6. The moment any crate reaches `200` on the
   registry, that version is permanently fixed (§4 below) and the tag must not move.
3. **A tag pushed before its commit's CI run completes is refused, not silently accepted.** The
   gate reports `CI_MISMATCH` for a real-but-not-yet-successful CI run distinctly from
   `CI_LOOKUP_FAILED` (no CI run found at all — see §6). Recovery needs no re-tag: wait for CI to
   record success on the tagged commit, then re-run the same workflow run.

## 1. Establishing what actually reached crates.io

Before doing anything else, find out which of the eleven crates are already at the tag version.
This is the same registry-state check `scripts/publish-crates.sh` performs before attempting
each crate's publish — the runbook and the pipeline read the same source of truth, so they
cannot disagree.

```bash
VERSION="0.8.1-rc.2"   # strip any leading "v" from the tag first
for name in paladin-ai-core paladin-ports paladin-herald paladin-battalion paladin-llm \
            paladin-memory paladin-web paladin-notifications paladin-content \
            paladin-storage paladin-ai; do
  status=$(curl -s -o /dev/null -w '%{http_code}' \
    -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' \
    "https://crates.io/api/v1/crates/${name}/${VERSION}")
  case "${status}" in
    200) echo "${name}: already-at-this-version" ;;
    404) echo "${name}: not-yet-published" ;;
    *)   echo "${name}: unexpected HTTP ${status} -- investigate before assuming either state" ;;
  esac
done
```

The `User-Agent` header is not optional — crates.io answers `403` to a request without one, and
without it every crate reads as a false `unexpected HTTP 403`, the worst possible answer while
mid-incident. `200` means the version exists and can never be re-uploaded, even if it was later
yanked (see §4) — a yanked version's versioned endpoint still returns `200`.

If a crate reads `not-yet-published` here but a *dependent* crate's publish still fails on
dependency resolution, the second place to look is the sparse index
(`https://index.crates.io/<index-path>`, e.g. `pa/la/paladin-ports`) — the index can lag the
registry's own database record by a short, bounded window, which is exactly what
`_pc_wait_for_index_visibility` in `scripts/publish-crates.sh` polls for after every real
publish.

## 2. Reading the run

`publish-crates`'s own step writes a per-crate outcome table to `$GITHUB_STEP_SUMMARY` and to
the job log. Every crate ends in exactly one of four states:

- **`published-now`** — not on the registry before this run; this run published it and the
  index confirmed it visible before the poll timeout.
- **`already-at-this-version`** — the registry already had this exact version before this run
  attempted anything; no publish was attempted for this crate.
- **`skipped`** — this run never attempted the crate at all: either the whole run was a
  dry-run, or an earlier crate in dependency order `failed` and every crate after it was
  abandoned rather than attempted against a dependency that never landed.
- **`failed`** — the pre-check returned an unrecognized HTTP status, `cargo publish` itself
  exited non-zero, or the post-publish index-visibility poll timed out.

**A run reporting zero crates as `published-now` fails deliberately.** That is not a broken
pipeline; it means every one of the eleven crates was already `already-at-this-version` before
the run started — the tag is already fully published and there was nothing left to recover. The
job's own error message names the version, states the tag appears fully published, and points
back at this document.

The workflow's overall run conclusion can be red for reasons that have nothing to do with
publishing — the Build Binaries matrix has failed on every observed run, undiagnosed, and does
not gate `publish-crates` (which depends only on `test`, `create-release` and
`check-release-consistency`). The `publish-crates` job's own outcome table is the authoritative
record of what actually moved on crates.io; do not infer publish health from the workflow's
overall green/red.

## 3. Completing forward

The default recovery for a partially-published tag is to **re-run the same tag's existing
workflow run** from the Actions UI (or `gh run rerun <run-id>`):

1. **"Re-run failed jobs" first.** It skips every job that already succeeded, including
   `create-release` (which is created once and reused, never re-created) and any crate the
   `publish-crates` loop already moved to `already-at-this-version`.
2. **"Re-run all jobs" as the fallback**, if the first option does not resolve it. This is also
   safe: `create-release` looks up and reuses the existing release object rather than failing
   on a duplicate, `check-release-consistency` re-verifies the same gate, and
   `scripts/publish-crates.sh`'s registry-state pre-check means every already-published crate is
   skipped on the re-attempt rather than re-uploaded (which crates.io would reject anyway).

Both re-run shapes reuse the same tag ref, and that is deliberate, not incidental. The
`crates-io` GitHub Environment's deployment policy restricts entry to a ref matching `v*.*.*`
typed as a **tag**, and the OIDC subject claim the Trusted Publishing token exchange validates
is bound to that same tag-push event. `workflow_dispatch` is **not** the documented recovery
path: whether a dispatched run is even eligible to mint a Trusted Publishing token is an
untested assumption (Phase 19's evidence log records it as such), and a re-run of the original
tag-push event carries no such open question. If a rehearsal ever incidentally proves
`workflow_dispatch` eligible, that is recorded as a fact discovered, not adopted as an
alternative path here.

**Never run two release workflow executions against the same tag concurrently.** Both runs would
perform the same registry-state pre-check at roughly the same moment, before either has
published anything; the one that loses the race attempts a real `cargo publish` for a crate the
other run is simultaneously publishing, and crates.io rejects the loser with what looks like an
ordinary publish failure but is actually a self-inflicted race. If you discover a run already in
flight, either let it finish before starting another, or cancel it first (`gh run cancel
<run-id>`) — do not start a second run "just in case" while the first is still executing.

### Re-running the body-finalizing job

The terminal `finalize-release-body` job (`scripts/finalize-release-body.sh`) is safe to re-run
under either re-run shape above. It reads the release body back with `gh release view`, truncates
it at a fixed marker, and **rebuilds the artifact sections from the run's current job outputs —
it never appends.** Re-running it after nothing else changed converges on the same body, byte for
byte. Re-running it after a previously-failed artifact leg (`build-docker`, `build-binaries`, or
`sbom`) has since succeeded adds that leg's section, because the rebuild reads whatever outputs
are available at the moment it runs — it does not need every leg to have succeeded together in
the same run. Asset uploads (`gh release upload ... --clobber`) are idempotent by name, so a
re-run's checksum aggregation and binary uploads replace rather than duplicate.

## 4. When completing forward is not enough

A version that has landed on crates.io is never deleted and never re-uploaded. crates.io does
not permit either operation, and — separately — a retry of an already-published version cannot
succeed regardless: §1's pre-check would report it `already-at-this-version` and
`scripts/publish-crates.sh` would skip it by design.

If a published version turns out to be bad (broken build, a mistake in what shipped, a security
issue), the correction is a **new patch version**, published normally through the pipeline, plus
yanking the bad one. Yanking hides a version from *new* dependency resolution — a project that
has not yet locked to it will no longer select it — but it does not remove the version, and any
`Cargo.lock` that already resolved to it keeps resolving to it and keeps building against it.

## 5. Who may yank, and how it is recorded

**Only the crate-owner account on crates.io — the repository owner — may yank.** CI does not
yank, and must not: no workflow, script, or Makefile target in this repository performs a yank,
and the OIDC-minted Trusted Publishing token (scoped to publish, expiring in roughly thirty
minutes) is deliberately never used for one. Yanking is a human act taken from the crates.io web
UI or with `cargo yank` run locally, authenticated as the owning account, never from a CI
credential.

Command shape, run once per affected crate:

```bash
cargo yank --version <X.Y.Z> <crate-name>
```

To reverse a yank (`cargo yank --version <X.Y.Z> --undo <crate-name>`) is the same authority and
the same recording obligation below.

### Yank register

Every yank gets a row here — append one per yank, do not summarize multiple yanks into one row.
These entries live in this table, not in `SECURITY-EXCEPTIONS.md`: that file's schema is
mechanically checked by `scripts/check-advisory-register.sh` for RustSec advisory suppressions
specifically, and a yank record carries no advisory ID, no `Cargo.lock` crate entry in that
schema's sense, and no revisit condition — adding it there would either break the parser or
force a distorted fit into a contract it does not belong to.

| Version | Crates | Reason | Owner | Date |
|---|---|---|---|---|
| 0.10.0 | `paladin-ai-core` | Not defective — orphaned partial publish of `v0.10.0`, superseded by `0.10.1`. | Am0rfu5 | 2026-09-21 |
| 0.10.0 | `paladin-ports` | Not defective — orphaned partial publish of `v0.10.0`, superseded by `0.10.1`. | Am0rfu5 | 2026-09-21 |
| 0.10.0 | `paladin-herald` | Not defective — orphaned partial publish of `v0.10.0`, superseded by `0.10.1`. | Am0rfu5 | 2026-09-21 |

## 6. When the gate blocks the release

`scripts/check-release-consistency.sh` (invoked both as a `release.yml` job and locally via
`make check-release-consistency`) reports every mismatch it finds in one run, not just the
first. Each status token below is what you will see verbatim in the job log or your terminal.

### `MISMATCH`

A publishable crate's `Cargo.toml` `[package] version` does not exactly equal the tag version
(string equality — `0.8.1` and `0.8.1-rc.2` do not match). Fix: bump the crate to match the tag
(the workspace uses lockstep versioning, so this should mean re-running the version bump across
all eleven crates) or fix the tag if it was cut against the wrong commit.

### `ZERO_PACKAGES`

`cargo metadata` enumerated no publishable packages at all. Fix: check for a broken workspace
manifest or an accidental `publish = false` added to every crate — this should never happen in
normal operation and points at a manifest-level regression, not a release-timing issue.

### `MISSING_TAG`

The gate was invoked without `--tag`. Fix: pass `--tag vX.Y.Z` (or the bare version).

### `CHANGELOG_MISMATCH` — changelog file not found

A publishable package has no `CHANGELOG.md` next to its own manifest at all. Fix: add one
following the Keep-a-Changelog shape its sibling crates already use.

### `CHANGELOG_MISMATCH` — no section for this version

The changelog file exists but has no `## [X.Y.Z]` heading for the tag version. This section is
normally written by the release tooling (`make release`'s changelog finalization step, extended
to cover all ten crate changelogs alongside the root one) as part of cutting the release, not by
hand. Fix: run the release flow rather than hand-editing eleven files; if you are seeing this
locally before tagging, it means the finalization step has not run yet.

This gate's clause checks the same root-changelog condition as the `create-release` job's own
`extract-changelog-section.sh` step, which fails first and separately if the root `CHANGELOG.md`
has no `## [X.Y.Z]` section — see [Body source](release-automation.md#body-source). Whichever
side of the pipeline you hit this from, the fix is the same: finalize the changelog and re-tag.

### `CI_MISMATCH`

The tagged commit's most recent completed `ci.yml` run did not conclude `success`, or no
completed run was found for that SHA at all. This includes the case where **the tagged commit
has no recorded successful CI run whatsoever** — there is nothing to fall back to and no tag
trigger exists on `ci.yml` to manufacture one on demand (deliberately: adding one would duplicate
the eighteen-job suite and drift from the main-branch run it exists to check). Fix: one of two
remedies, and no others —

1. **Re-run CI on `main` at that exact commit** (`gh run rerun` against `ci.yml`'s run for that
   SHA, or a fresh `gh workflow run ci.yml` if no run exists at all for it), then retry the
   release.
2. **Fix and re-tag** — if the commit itself is bad, correct it on `main`, then cut a fresh tag
   against the corrected commit.

### `CI_LOOKUP_FAILED`

The GitHub API lookup for CI runs itself failed — a transport error, an authorization problem,
or rate limiting. This is deliberately never conflated with "no successful run exists"; it means
the check could not be performed, not that it failed. Fix: resolve API access (token scope,
network, rate-limit backoff) and re-run the gate.

### `MISSING_SHA`

The gate was run with `GITHUB_ACTIONS=true` (i.e., inside a workflow) but without `--sha`,
so the CI-conclusion check could not run at all — the gate fails closed rather than silently
skipping a clause on the CI path. Outside GitHub Actions, an absent `--sha` instead runs the
manifest and changelog clauses only and says explicitly that the CI clause was not checked;
that combination is a valid local pass, not this failure.

### Combined failures (`MISMATCH_AND_CHANGELOG`, `MISMATCH_AND_CI`, `CHANGELOG_AND_CI`, `MISMATCH_AND_CHANGELOG_AND_CI`)

More than one of the clauses above failed in the same run. The report lists every individual
failure from every failing clause — read the detail lines, not just the combined status token,
and apply each remedy above independently.
