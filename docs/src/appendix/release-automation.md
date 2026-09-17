# Release Automation

This document records the evaluation of workspace release tooling for the Paladin framework, the
selected tool, and the operator guide for cutting a release. It is part of **Milestone 10 — CI
Hardening and Release Automation, Epic 3**.

> **Recovery:** If a release stops partway through, or the pre-publish consistency gate blocks
> `publish-crates`, see [Release Recovery](release-recovery.md) for the failure path — how to
> establish what actually reached crates.io, how to complete forward, and the yank policy.

## Tooling Evaluation: `cargo-release` vs. `release-plz`

| Dimension | `cargo-release` | `release-plz` |
|-----------|-----------------|---------------|
| Trigger model | Manual, developer-invoked command (`cargo release`) | PR-bot: opens/maintains a "release PR" automatically from `main` |
| Changelog handling | Works with a curated `CHANGELOG.md`; can run hooks to edit it | Auto-generates changelog from Conventional Commits |
| Workspace publish order | Built-in: publishes members in dependency order, supports lockstep or independent versions | Built-in: computes order, also opinionated about per-crate versioning |
| Version bumping | Bumps `[package].version` + internal `workspace.dependencies` pins in lockstep | Bumps versions per-crate based on detected changes |
| Required secrets / infra | `CARGO_REGISTRY_TOKEN` for publish; no bot, no extra app | `CARGO_REGISTRY_TOKEN` **plus** a GitHub token/app for the release-PR bot |
| Operational model | Fits an existing tag-triggered pipeline: bump+tag locally, CI publishes on the tag | Replaces the manual flow with a continuously-updated release PR |
| Maintenance cost | Low: one config file (`release.toml`), no running bot | Higher: bot behavior, PR hygiene, commit-message discipline enforced |
| Fit with current practice | High — matches curated `CHANGELOG.md`, lockstep `0.3.0`-everywhere, and `release.yml` `v*.*.*` trigger | Lower — requires moving to Conventional-Commit-driven changelog + PR-bot workflow |

**2026-08 note:** the "Required secrets / infra" row above describes the pre-2026-08 posture and
is retained as the historical record of this decision, not amended to match what shipped later.
As of Phase 19 (crates.io Trusted Publishing), publishing no longer uses a stored `CARGO_REGISTRY_TOKEN`
registry credential at all — see [Trusted Publishing](#trusted-publishing) below for the credential
path that replaced it.

### Recommendation & Decision: **`cargo-release`**

`cargo-release` is selected. The Paladin repository already has:

- a **curated `CHANGELOG.md`** with a `## [Unreleased]` section (we want to keep authoring it, not
  auto-generate it),
- **lockstep versioning** (every public crate is `0.3.0`; `docs/RELEASE_CHECKLIST.md` mandates a
  "lockstep version update across public crates"), and
- a **tag-triggered pipeline** (`.github/workflows/release.yml` already fires on `v*.*.*`).

`cargo-release` slots directly into this model: a maintainer runs a single command (wrapped by
`make release VERSION=x.y.z`) that bumps all crates in lockstep, finalizes the changelog, commits,
tags `v x.y.z`, and pushes. The push triggers CI, which publishes the crates to crates.io in
dependency order. No PR-bot, no GitHub App, and no change to the curated-changelog or
Conventional-Commit practice is required.

`release-plz` is a strong tool but optimizes for a different workflow (PR-bot + auto-changelog +
per-crate version detection) that would be a larger process change for marginal benefit here. It can
be revisited if the project later adopts strict Conventional Commits and prefers a continuous
release-PR model.

## Reproducible Installation

`cargo-release` is installed the same way locally and in CI, pinned and `--locked`:

```bash
cargo install cargo-release --locked
```

(The CI publish job installs it with `--locked` so the build is reproducible from `Cargo.lock`.)

## Release Configuration (`release.toml`)

The repo-root `release.toml` encodes:

- **Lockstep versioning** — `shared-version = true` so all publishable crates move to the same
  version in one bump, and the internal `workspace.dependencies` pins are updated to match.
- **Dependency-ordered publishing** — `cargo-release` publishes workspace members in topological
  dependency order: `paladin-core` → `paladin-ports` → the leaf tier (`paladin-battalion`,
  `paladin-llm`, `paladin-memory`, `paladin-web`, `paladin-notifications`, `paladin-content`,
  `paladin-storage`) → `paladin` (facade).
- **Tag/commit conventions** — a single workspace tag `v{{version}}` is created (the
  `.github/workflows/release.yml` pipeline keys off `v*.*.*`).

## Canonical Publish Order

The eleven-crate order actually published by `.github/workflows/release.yml`'s `publish-crates`
job (package names, dependency-first):

1. `paladin-ai-core` (source directory `crates/paladin-core`)
2. `paladin-ports`
3. `paladin-herald`
4. `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`,
   `paladin-notifications`, `paladin-content`, `paladin-storage` (parallel-safe leaf tier;
   published sequentially in the workflow, but none depends on another)
5. `paladin-ai` (facade, package name `paladin-ai`, source directory is the workspace root)

**Why `paladin-herald` sits after `paladin-ports` rather than immediately after `paladin-ai-core`:**
it carries a normal `[dependencies]` edge on `paladin-ai-core` (must publish after it) but also a
version-pinned `[dev-dependencies]` edge on `paladin-ports`, which Cargo records in the published
manifest and crates.io validates against the index at publish time — so `paladin-ports` must
already be on the registry before `paladin-herald` can be published. This full reasoning is
recorded in `.planning/phases/19-crates-io-trusted-publishing-replace-the-long-lived-registry/19-PUBLISH-EVIDENCE.md`
under "Dependency-Order Constraints," including the superseded (wrong) insertion point two earlier
planning documents proposed.

No package named for a separate CLI binary exists in this workspace; no publish-order row exists
for one.

**Packaging note:** the workspace-root `Cargo.toml` (`paladin-ai`'s package root, since the facade
crate's manifest lives at the repository root) carries a `package.include` allowlist. Without it,
`cargo package` bundles the entire repository tree — `docs/`, `.planning/`, `.claude/` and
everything else — and exceeds crates.io's 10 MiB upload cap. `cargo publish --dry-run` does not
catch this: dry-run aborts before the upload step where the server enforces the size limit. The
ten `crates/*` packages are unaffected, since their package roots are their own subdirectories.

## Operator Guide: Cutting a Release

A release is cut through a version-bump PR merged to `main`, followed by an annotated tag pushed
directly to the merge commit; CI does the publishing. This is the flow actually exercised by the
`0.8.1-rc.1` and `0.8.1-rc.2` releases (2026-08-26 / 2026-08-27), not the single-command flow
`make release` was originally written to run end-to-end.

**`make release`'s final step — `git push origin HEAD` to `main` — no longer works.** The
"Protect main branch" ruleset (PR-only merges, zero bypass actors) blocks a direct push; this last
succeeded at `v0.5.1` (2026-06-04). `make release`'s other steps (semver validation,
`release-check`, the lockstep version bump, and the changelog finalization) are still correct and
still used — only the trailing push to `main` cannot complete under the current ruleset, so that
half of the release is done by hand through a PR instead.

1. From an up-to-date `main`, create a version-bump branch (e.g. `chore/release-<version>`).
2. Bump every public crate in lockstep and update internal dependency pins:
   `cargo release version <version> --execute --no-confirm --workspace`.
3. Finalize `CHANGELOG.md`: move the `## [Unreleased]` section under a new
   `## [<version>] - <date>` heading.
4. Regenerate the OpenAPI baseline: `make openapi`. The baseline embeds the crate version;
   `make release` does not automate this step, and `make release-check` fails without it.
5. Run `make release-check` locally (format, lint, full tests, audit, release build) — it must
   pass end-to-end before opening the PR.
6. Open a PR from the branch to `main`; merge once green.
7. Create an annotated tag `v<version>` on the merge commit and push it directly to `origin`. A
   branch push to `main` is blocked by the ruleset above, but tag creation carries a
   repository-admin bypass on the separate "Protect release tags" ruleset, so this step succeeds
   where step-7-as-a-branch-push would not.
8. The tag push triggers `.github/workflows/release.yml`: `Verify Tag From Main`, `Test Suite`,
   `Create Release`, `Publish to crates.io` (Trusted Publishing — see below), `Build and Push
   Docker Images`, `Build Binaries`, and `Generate SBOM`.

Install the tool once with:

```bash
cargo install --locked cargo-release
```

**Known operational caveats:**

- **Re-dispatching a release is safe even if the GitHub release object already exists.**
  `create-release`'s "Create or reuse release" step (`scripts/create-or-reuse-release.sh`) looks
  the release up by tag first — an HTTP 200 reuses the existing object, a 404 creates it — with no
  `actions/create-release@v1` dependency and no upsert-failure mode. A `workflow_dispatch` or
  Actions-UI re-run after a failed attempt does not require deleting the stale release object
  first; see `release-recovery.md`'s "Completing forward" section for the full re-run playbook.
- **The four Build Binaries matrix jobs (`ubuntu-latest`/`macos-latest` × two targets each) have
  historically failed on every release run observed prior to this phase**, root-caused (D-05) to
  builds silently omitting `paladin-cli`/`paladin-server` when Cargo's `required-features` were
  unmet. `scripts/package-release-binaries.sh`'s expected-binary assertion now hard-fails the leg
  instead of silently shipping an incomplete archive. This does not gate crates.io publishing —
  `publish-crates` depends on three jobs: `test`, `create-release`, and
  `check-release-consistency` (the pre-publish consistency gate, PUBOPS-01, which fails closed if
  the release tag's version disagrees with any publishable crate's manifest version) — so judge
  publish health by the `publish-crates` job and the registry state, never by the workflow's
  overall run conclusion alone.

### Dry Run (no live publish)

To exercise the pipeline without publishing to crates.io, trigger the workflow manually with the
`dry_run` input set to `true`:

```bash
gh workflow run release.yml -f tag=v0.4.0-rc.1 -f dry_run=true
```

In dry-run mode the publish job runs `cargo publish --dry-run` for each crate in order instead of a
real publish. Locally, the same validation is available via:

```bash
make publish-dry-run
```

See [Dry-Run Claim Boundary](#dry-run-claim-boundary) below for exactly what a green dry run does
and does not prove.

## Release Notes and Attached Artifacts

This section describes what a `vX.Y.Z` release actually hands a consumer — where the notes come
from, exactly what is attached, how to verify it, and whether any of it is signed.

### Body source

The release body for `vX.Y.Z` is the root `CHANGELOG.md` `## [X.Y.Z]` section, extracted by
`scripts/extract-changelog-section.sh` — everything between that heading and the next `## [`
heading. A tag whose version has no such section fails the `create-release` job outright; the job
log prints the exact remedy:

```
run make release VERSION=X.Y.Z (finalizes changelogs) before tagging
```

The pipeline has no alternate body source — there is no fallback to a commit-log summary. A
heading-only section (no body text before the next heading) is accepted and extracts to an empty
string, because `scripts/finalize-crate-changelogs.sh` legitimately produces one for a quiet
prerelease; presence of the heading is the pass signal, not presence of content. The ten per-crate
changelogs that ship inside the published crates contribute nothing to the release body — they are
a separate, per-package artifact, not release notes.

### Artifact inventory

| Artifact | Detail |
|---|---|
| Binaries | `paladin`, `paladin-cli`, `paladin-server` — all three, per target |
| Feature set | `cli,web-server` on every leg, plus `vendored-openssl` on the aarch64 Linux leg (the `cross` container has no target-arch system OpenSSL) |
| Targets | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| Archive naming | `paladin-<os>-<arch>.tar.gz`, one per target (`paladin-linux-amd64`, `paladin-linux-arm64`, `paladin-macos-amd64`, `paladin-macos-arm64`) |
| Per-asset checksums | `<archive>.tar.gz.sha256`, one per archive |
| Aggregated checksums | `SHA256SUMS`, covering every archive actually visible on the release at finalize time |
| SBOM | A CycloneDX document for the root `paladin-ai` package, `paladin-<version>.cdx.json` |
| Container image | Multi-arch (`linux/amd64`, `linux/arm64`) image pushed to `ghcr.io` |

This table states fact only for what the pipeline produces. `expected_binaries_for_target` in
`scripts/package-release-binaries.sh` is the single place a target's binary set is narrowed — if a
future leg ships fewer than three binaries, that function's case entries are where the change is
made and recorded, and this table must be updated to match.

### Verification

After downloading the archives and `SHA256SUMS` from the release:

```
sha256sum -c SHA256SUMS
```

On macOS, where GNU coreutils' `sha256sum` is absent:

```
shasum -a 256 -c SHA256SUMS
```

To verify the container image is the exact one this release built (rather than trusting a mutable
tag), pull it by the immutable digest the release body names:

```
docker pull ghcr.io/your-org/paladin@sha256:<digest>
```

These commands are quoted here word-for-word from what `scripts/finalize-release-body.sh` composes
into the release body itself, so the two cannot drift apart.

### Assembly order

`create-release` publishes the curated notes alone, from the `CHANGELOG.md` section above. A
terminal `finalize-release-body` job then appends the artifact sections described here, reading
them from the real outputs of `build-docker`, `build-binaries` and `sbom` — never from a
hand-reconstructed guess. A leg that failed or was skipped contributes no section, so the release
never advertises an artifact the run did not actually produce.

### Signing and build provenance

**The attached artifacts carry checksums but no signature and no build attestation.** A checksum
proves integrity against the release as published — that the bytes you downloaded match what the
release page says — it does not prove who built them or that they came from this project's own CI
run rather than a compromised upload. Do not read a passing `sha256sum -c` as proof of origin.

This is a deliberate deferral, not an oversight. Adopting a signing or provenance mechanism —
`cosign` or GitHub's native artifact attestations — would add new action surface plus key and
identity management in the same phase that removed archived, unpinned actions, and no consumer
requirement demands it yet. `actions/attest-build-provenance` is the natural candidate when signing
is taken up: it is GitHub-native, requires no new key material to manage, and integrates directly
with the existing `gh release upload` flow. Revisit when a consumer or a registry policy actually
requires provenance, not before.

The measured image size is reported the same way — advisory prose against a 500 MB target, never a
gate. A run whose image exceeds the target still finishes green, honestly reporting the figure in
the release body rather than failing on an unvalidated threshold.

## Trusted Publishing

The `publish-crates` job holds `id-token: write` at job scope, runs under the `crates-io` GitHub
Environment, and calls `rust-lang/crates-io-auth-action@v1` to exchange its GitHub OIDC identity
for a crates.io token that expires in roughly thirty minutes and is revoked by the action's own
post step at job end. `cargo publish` consumes it from the same `CARGO_REGISTRY_TOKEN` environment
variable it always did. There is no repository secret to configure and none to be absent.

### Environment and Protection Posture

- **Environment:** `crates-io`, on `DF3NDR/paladin-dev-env`.
- **Deployment policy:** restricted to `v*.*.*` refs, typed as a **tag** rule — a branch push
  cannot reach this environment, only a ref matching `v*.*.*` typed as a tag can.
- **Reviewer gate:** none. No wait timer either, so tag-push releases stay unattended (D-08) — the
  ref restriction is the protection, not a human approval step.
- **Secrets:** none. The environment's secret store reads `total_count: 0`; it exists to constrain
  identity via the OIDC subject claim, not to hold credentials — giving it a secret store would
  reintroduce the standing-token pattern this phase removed.

Tightening this later to require reviewer approval is a repository-settings change plus a
one-line update to this posture description — a recorded, deliberate choice today, not a default
nobody examined.

### Per-Crate Trust Configuration

Each crates.io package below carries its own Trusted Publishing configuration, pointing at this
repository, the `release.yml` workflow, and the `crates-io` environment. Equality is by crates.io
package name, never by directory name — the two rows where they diverge (`paladin-ai-core` /
`paladin-ai`) carry their own source directory rather than a footnote.

| Crate name | Source directory | Workflow filename | Environment name | Link date | Status |
|---|---|---|---|---|---|
| `paladin-ai-core` | `crates/paladin-core` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-ports` | `crates/paladin-ports` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-herald` | `crates/paladin-herald` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-battalion` | `crates/paladin-battalion` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-llm` | `crates/paladin-llm` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-memory` | `crates/paladin-memory` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-web` | `crates/paladin-web` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-notifications` | `crates/paladin-notifications` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-content` | `crates/paladin-content` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-storage` | `crates/paladin-storage` | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |
| `paladin-ai` | workspace root (`Cargo.toml`) | `release.yml` | `crates-io` | 2026-08-27 | linked (reported) |

All eleven crates are covered; none is excluded. "linked (reported)" reflects that this table is
copied from the evidence ledger's Trust Link Ledger, whose "linked" confirmation came from the
human operator who created these configurations one at a time through the crates.io web UI (no
API or CLI exists for this step) — not independently re-verified by re-opening each crate's
settings page, because crates.io exposes no public API for reading a Trusted Publishing
configuration back. The proof publish (`0.8.1-rc.2`, run 33089177606) is what falsifies a silently
missing or misconfigured link: a crate whose configuration was not actually saved would fail at
the `Authenticate with crates.io` step or at that crate's own publish step rather than passing
silently — and all eleven passed. If any row is later found incorrect, it is corrected here and
the discrepancy is recorded, not silently overwritten.

### Credential History

| Event | Date | Actor | Evidence |
|---|---|---|---|
| Bootstrap publish of `0.8.1-rc.1` (all eleven crates) using the standing `CARGO_REGISTRY_TOKEN` | 2026-08-26 | Am0rfu5 (repository owner; workflow dispatches and PR merges performed by Claude Code at the owner's explicit request) | `.planning/phases/19-crates-io-trusted-publishing-replace-the-long-lived-registry/19-PUBLISH-EVIDENCE.md`, "Bootstrap Publish (old credential)"; run [33009214745](https://github.com/DF3NDR/paladin-dev-env/actions/runs/33009214745) |
| OIDC proof publish of `0.8.1-rc.2` (all eleven crates, non-null `trustpub_data`) | 2026-08-27 | Am0rfu5 (repository owner; PR merges and the tag push performed by Claude Code at the owner's explicit delegation) | `.planning/phases/19-crates-io-trusted-publishing-replace-the-long-lived-registry/19-PUBLISH-EVIDENCE.md`, "OIDC Proof Event (PUB-03)"; run [33089177606](https://github.com/DF3NDR/paladin-dev-env/actions/runs/33089177606) |
| crates.io publish-scoped token ("Paladin") revoked | 2026-08-27 | Am0rfu5, via the crates.io UI (Account Settings → API Tokens) | `.planning/phases/19-crates-io-trusted-publishing-replace-the-long-lived-registry/19-PUBLISH-EVIDENCE.md`, "Revocation Ledger" row 1 |
| GitHub repository secret `CARGO_REGISTRY_TOKEN` deleted from `DF3NDR/paladin-dev-env` | 2026-08-27 | Am0rfu5, via the GitHub web UI (agent-side `gh secret delete` was blocked by the local Claude Code permission classifier, so the deletion was routed to the human) | `.planning/phases/19-crates-io-trusted-publishing-replace-the-long-lived-registry/19-PUBLISH-EVIDENCE.md`, "Revocation Ledger" rows 2-3 |

This history is not folded into `SECURITY-EXCEPTIONS.md`. That register is scoped to RustSec
advisory suppressions and is mechanically parsed by `scripts/check-advisory-register.sh`, which
expects each row to match its own `[[exception]]` TOML schema — a credential-history event has no
advisory ID, no affected crate in `Cargo.lock`'s sense, and no revisit condition in that schema's
terms, so adding it there would either break the parser or force a distorted fit. This table keeps
the same discipline (dated, named actor, cited evidence) without borrowing the register's format.

### Dry-Run Claim Boundary

Dry-run mode skips the OIDC exchange entirely, because `cargo publish --dry-run` needs no
credential — the `Authenticate with crates.io` step only runs `if: steps.mode.outputs.dry_run !=
'true'`. This keeps dry runs working on forks and before any Trust Publisher Configuration exists.

It also fixes what a green dry run is allowed to mean: **it asserts packaging validity and asserts
nothing whatsoever about authentication.** A future reader pointing at a green dry run as proof
the OIDC credential path works is the specific mistake this boundary exists to prevent — the
`0.8.1-rc.1` packaging failure (a `413` on `paladin-ai`, below the size cap this document's
Canonical Publish Order section notes) is a concrete instance of a dry run passing while the real
upload failed for a reason dry-run mode cannot observe.

### Break-Glass Recovery

If the OIDC path breaks (an expired-mid-loop token, a misconfigured Trust Publisher Configuration,
a revoked environment policy), the recovery is:

1. Mint a new crates.io token with publish scope.
2. Temporarily restore a secret-based credential on the `publish-crates` job's publish step
   (add the token back as a repository or environment secret and point `CARGO_REGISTRY_TOKEN`
   at it instead of the OIDC action's output).
3. Publish.
4. Revoke the token and delete the secret again — recording both halves in the Credential
   History table above, the same way the original revocation was recorded.

Naming this path here is what stops it from being improvised badly under pressure.

### Known Limits

- **A single minted token must outlive the whole eleven-crate sequential loop**, including its
  index-wait sleeps. The proof run used roughly 23% of the token's ~30-minute lifetime
  (`Authenticate with crates.io` completed at `15:48:40Z`; the last crate's registry `created_at`
  read `15:55:35.67Z`, a span of ~6m56s) — comfortable at current pacing, but an accepted risk
  (T-19-13), not a guarantee. A late-loop authentication failure would be that risk materializing,
  not a workflow bug; re-running the job mints a fresh token.
- **crates.io's "Trusted Publishing Only" enforcement mode exists and is not enabled.** It was
  deliberately left off on every crate so the break-glass path above stays available. Enabling it
  per-crate would remove that fallback in exchange for closing off any standing-token path
  entirely.
- **`workflow_dispatch`-triggered runs minting a Trusted Publishing token is untested.** The proof
  publish deliberately used a tag push, not `workflow_dispatch`, specifically to avoid depending
  on this untested assumption. Whether a dispatched run is eligible to mint a token remains
  unestablished by anything in this phase.
