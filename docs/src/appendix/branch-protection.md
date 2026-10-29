# Branch & Release-Tag Protection

This document describes the **main-only release policy** for the Paladin Framework, the three
layers that enforce it, and the applied state of the GitHub rulesets that back Layer 3. For how to
branch, what CI runs on a push, and how a change reaches `main` day to day, see
[Branching Model](../contributing/branching-model.md) — that page is written for contributors; this
one is the administrator-facing enforcement detail behind the checks it describes.

> **Policy in one sentence:** release tags (`v*.*.*`) may only be created from commits that are
> contained in the `main` branch. `main` is the single source of truth for released code.

---

## Why this policy exists

Milestone 10 Epic 3 made releases fully tag-driven: pushing a `v*.*.*` tag triggers
[`.github/workflows/release.yml`](https://github.com/DF3NDR/paladin-dev-env/tree/main/.github/workflows), which runs the test suite,
publishes crates to crates.io, builds Docker images and binaries, and generates an SBOM.

When the first release (Epic 4) was cut, the tag was pushed from a **feature branch** that
had not yet been merged into `main`. The pipeline only keyed off the tag, not the branch, so it would
have published code that never passed through the reviewed `main` branch. Epic 5 closed that gap.

---

## The three enforcement layers

| Layer | Where | What it enforces | Authoritative? |
|-------|-------|------------------|----------------|
| 1. CI guard | `verify-tag-source` job in `release.yml` | The tagged commit is an ancestor of `origin/main`; otherwise the whole pipeline fails before publishing. | **Yes** |
| 2. Local guard | `make release` target in `Makefile` | Refuses to bump/tag unless on an up-to-date `main`. Fast feedback before any push. | No (advisory) |
| 3. Platform rulesets | `.github/rulesets/*.json`, applied | PR + passing checks required to land on `main`; only authorized actors may create `v*` tags; `release/*` branches are pre-emptively protected. | Defense in depth |

### Layer 1 — CI guard (`verify-tag-source`)

The release workflow's first job resolves the release commit (`github.sha` for a tag push, or the
commit the dispatched `inputs.tag` points to) and runs:

```bash
git merge-base --is-ancestor "$RELEASE_SHA" origin/main
```

If the commit is **not** contained in `main`, the job emits a `::error::` annotation and exits
non-zero. The `test` and `create-release` jobs declare `needs: verify-tag-source`, so a failed guard
prevents publishing, Docker, binaries, and SBOM from running. This layer is authoritative because it
cannot be bypassed locally.

### Layer 2 — Local guard (`make release`)

Before bumping versions or tagging, `make release`:

1. Checks the current branch is `main`.
2. Fetches `origin/main` and fails if local `HEAD` is behind it.

Both checks run **before** any destructive action, so a wrong-branch release stops immediately with
no version bump, commit, or tag.

**Emergency override (hotfix branches only):**

```bash
RELEASE_ALLOW_ANY_BRANCH=1 make release VERSION=0.4.1
```

This bypasses **only** the branch-name check (the up-to-date check still runs). The CI guard (Layer 1)
remains authoritative — an override here does not let an unmerged commit publish from CI.

### Layer 3 — GitHub rulesets

Three rulesets are applied on the live repository, imported from the definitions in
[`.github/rulesets/`](https://github.com/DF3NDR/paladin-dev-env/tree/main/.github/rulesets):

| Ruleset | Applied ruleset ID | Target | Status |
|---------|---------------------|--------|--------|
| `protect-main-branch.json` | `20868126` | `refs/heads/main` | Active |
| `protect-release-branches.json` | `20868128` | `refs/heads/release/*` | Active |
| `protect-release-tags.json` | `20868099` | `refs/tags/v*` | Active |

**Applied 2026-08-14**, verified by reading the live rulesets back from the GitHub API
(`gh api /repos/DF3NDR/paladin-dev-env/rulesets`) rather than trusting the committed JSON files
alone — the committed payloads had previously sat unapplied for months, so a page describing intent
rather than server-confirmed state would provide no real assurance.

#### The required-check set

`protect-main-branch.json` requires all 46 of the following status-check contexts to pass before a
pull request into `main` can merge:

`API Surface Tracking`, `Benchmark Compile Check`, `Build & Test (all-features)`, `Build & Test
(cli)`, `Build & Test (content-processing)`, `Build & Test (default)`, `Build & Test (full)`, `Build
& Test (llm-all)`, `Build & Test (llm-anthropic)`, `Build & Test (llm-deepseek)`, `Build & Test
(llm-openai)`, `Build & Test (no-default-features)`, `Build & Test (notifications)`, `Build & Test
(redis-queue)`, `Build & Test (s3-storage)`, `Build & Test (vision)`, `Build & Test (web-server)`,
`Build MDBook`, `CLI Isolation (library without cli feature)`, `CLI Snapshot Tests`, `Code Quality`,
`Coverage`, `Crate Isolation (paladin-ai)`, `Crate Isolation (paladin-ai-core)`, `Crate Isolation
(paladin-battalion)`, `Crate Isolation (paladin-content)`, `Crate Isolation (paladin-llm)`, `Crate
Isolation (paladin-memory)`, `Crate Isolation (paladin-notifications)`, `Crate Isolation
(paladin-ports)`, `Crate Isolation (paladin-storage)`, `Crate Isolation (paladin-web)`, `Docker
Integration Tests`, `End-to-End Tests`, `Example Muster (Feature Matrix)`, `Feature Matrix Summary`,
`Integration Tests`, `License & Dependency Policy`, `OSV Scanner`, `Publish Dry Run`, `Security
Audit`, `Semver Checks (vs v0.11.0)`, `Unit Tests (beta)`, `Unit Tests (stable)`, `Workflow Lint`,
`pre-commit run --all-files`.

**Updated 2026-10-28 (Phase 52, D-08).** `Publish Dry Run` and `Semver Checks (vs v0.11.0)` were
added to the committed payload, so the committed set is now 46 contexts. `End-to-End Tests` was
already required and needs no change: it runs on a pull request only when the maintainer has applied
the `release` label, and a job skipped by its job-level `if:` reports success to required-check
evaluation, so a label-less pull request is not blocked, while a labelled pull request runs it and
its failure blocks the merge (precedent: PR #59 merged with it skipped). The change takes effect
only after the maintainer applies it with the `PUT` under "Updating an already-applied ruleset"
below and reads the ruleset back; until then the live ruleset still carries the previous 44.

**Two jobs are deliberately excluded from the required set: `Docker Build` and `Kubernetes Smoke
Test`.** Both still run on every push and pull request — they simply do not block the merge button.
`Docker Build` measured **3762 seconds (62.7 minutes)** — the entire pipeline's critical path,
building `linux/amd64,linux/arm64` with `arm64` under QEMU emulation — against a required-set
critical path of roughly seven minutes (`Integration Tests`, the slowest required job, at 398
seconds). Requiring it would serialize every merge behind an hour-plus emulation run. See ADR-0044
(`.planning/decisions/0044-branch-protection-posture.md`) for the full reasoning and the
alternative that was considered and declined (a native-arm64 runner rework).

#### The bypass asymmetry

The trunk ruleset and the tag ruleset deliberately carry different bypass postures — read this
plainly, not as an inconsistency to "fix":

- **`protect-main-branch.json` carries no administrative bypass.** It gates the only path into
  `main`: a pull request with all 46 required checks green. A merge gate any account — including an
  administrator — can bypass at will is not a gate, only a suggestion.
- **`protect-release-tags.json` retains a bypass actor** (`actor_id: 5`, `RepositoryRole` = Admin,
  `bypass_mode: always`), because it restricts **creation** of a `refs/tags/v*` ref, not a merge.
  Without a bypass actor, tag creation itself would be restricted to nobody, and no release could
  ever be cut. The retained bypass is what makes the tag ruleset usable at all, not an oversight.

`protect-release-branches.json` follows the trunk ruleset's posture — no bypass — since it also
gates a merge (into a future backport branch), not a ref creation.

`required_approving_review_count` is `0` on both branch rulesets: the repository has exactly one
active collaborator and GitHub does not allow self-approval, so a nonzero review count would be
satisfiable only through a bypass — which is the exact self-defeating configuration the committed
payload shipped with before this policy was applied. The pull request itself, and every required
check passing against it, stay mandatory regardless. If the project gains a second active
committer, the review count is the thing to revisit.

---

## Applying or auditing the rulesets (administrators)

Rulesets require repository-admin scope. Two different calls apply, depending on whether the
ruleset already exists on the repository — using the wrong one for the situation is how a
re-application ends up creating a second, disagreeing ruleset instead of updating the one already
in force (see "Updating an already-applied ruleset" below).

### First-time application

The three rulesets were originally applied via the `gh` CLI, using a **create** call. This call is
correct only the first time a given ruleset is applied:

```bash
gh api --method POST \
  -H "Accept: application/vnd.github+json" \
  /repos/DF3NDR/paladin-dev-env/rulesets \
  --input .github/rulesets/protect-main-branch.json

gh api --method POST \
  -H "Accept: application/vnd.github+json" \
  /repos/DF3NDR/paladin-dev-env/rulesets \
  --input .github/rulesets/protect-release-branches.json

gh api --method POST \
  -H "Accept: application/vnd.github+json" \
  /repos/DF3NDR/paladin-dev-env/rulesets \
  --input .github/rulesets/protect-release-tags.json
```

The GitHub UI equivalent is **Settings → Rules → Rulesets → New ruleset → Import a ruleset**, one
upload per JSON file.

### Updating an already-applied ruleset

**Do not re-run the create call above against a ruleset that is already applied.** The create
endpoint has no notion of "this already exists, update it in place" — running it again produces a
**second** ruleset targeting the same refs, alongside the one already enforcing them, leaving two
rulesets disagreeing about the same branch. Every change to an already-applied ruleset — for
example, adding a newly promoted required-status-check context — instead goes through the
id-addressed **update** endpoint:

```
PUT /repos/{owner}/{repo}/rulesets/{ruleset_id}
```

with the whole committed JSON file as the request body, matching this document's existing
whole-file input convention. For the trunk ruleset (`protect-main-branch.json`, repository
`DF3NDR/paladin-dev-env`, applied ruleset id `20868126`):

```bash
gh api --method PUT \
  -H "Accept: application/vnd.github+json" \
  /repos/DF3NDR/paladin-dev-env/rulesets/20868126 \
  --input .github/rulesets/protect-main-branch.json
```

The other two rulesets follow the same shape, substituting their own applied ruleset id
(`protect-release-branches.json` → `20868128`; `protect-release-tags.json` → `20868099`) and JSON
file.

**Read the ruleset list back after every update.** This document already records that the
rulesets were verified by reading them back from the API rather than by trusting the committed
files, because the committed payloads had previously sat unapplied for months (see "Applied
2026-08-14" above) — the same discipline applies here, so a duplicate cannot go unnoticed:

```bash
# The repository still has exactly three rulesets — a correct update never adds a fourth.
gh api /repos/DF3NDR/paladin-dev-env/rulesets -q 'length'

# The same id, read back, now carries the updated content.
gh api /repos/DF3NDR/paladin-dev-env/rulesets/20868126
```

If the count above is anything other than three, or a new ruleset id shows up targeting the same
ref, the create call was used where the update call belonged — delete the duplicate (see "Rolling
back" below), then re-apply the change with the `PUT` form.

### Auditing the active rulesets

```bash
gh api /repos/DF3NDR/paladin-dev-env/rulesets
```

### Rolling back

Roll one back (reversible while the token retains `Administration: write`):

```bash
gh api -X DELETE /repos/DF3NDR/paladin-dev-env/rulesets/<id>
```

> The `bypass_actors` entry on `protect-release-tags.json` uses `actor_id: 5` (`RepositoryRole` =
> Admin). Adjust the role id or add team/app actors to match your organization before importing.

---

## The correct release flow under this policy

```bash
# 1. Open a PR for your changes and get it merged into main (all 46 required checks must pass).
# 2. Update your local main.
git checkout main
git pull --ff-only origin main

# 3. Cut the release from main.
make release VERSION=0.8.0
```

Pushing the resulting tag triggers `release.yml`; `verify-tag-source` confirms the tagged commit is
in `main`, and the pipeline proceeds to publish.

---

## The trunk fast-forward

`main` now carries the code every release publishes. It was fast-forwarded from a default branch
hundreds of commits stale to the tip of the branch that had been doing integration duty — a clean,
zero-conflict fast-forward with nothing on the trunk the integration branch lacked. The retired
branches are deleted, both proven ancestors of the new trunk, so no history was lost and no archival
tag was needed. Full command-level evidence lives in ADR-0043
(`.planning/decisions/0043-github-flow-trunk-and-trigger-surface.md`).

---

## Related documents

- [Release Automation](release-automation.md) — release tooling decision and operator guide.
- [Release Checklist](release-checklist.md) — manual release checklist.
- [Contributing to Paladin](../contributing/development-setup.md) — `## Releasing` section.
- [Branching Model](../contributing/branching-model.md) — the contributor-facing branching and
  trigger-surface page this document backs with enforcement detail.
