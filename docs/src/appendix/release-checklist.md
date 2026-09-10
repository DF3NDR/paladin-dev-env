# Release Checklist

This checklist defines the required release path from code freeze through publish and announcement.

> **Automation:** Most of this checklist is automated by `make release VERSION=x.y.z` and the
> tag-triggered `.github/workflows/release.yml` pipeline. See
> [RELEASE_AUTOMATION.md](release-automation.md) for the tooling decision (`cargo-release`) and the
> operator guide. This checklist remains the authoritative description of the end-to-end process and
> the manual verification steps.

> **Recovery:** If a release stops partway through, or a gate blocks `publish-crates` from
> running, see [Release Recovery](release-recovery.md) for how to establish what actually
> reached crates.io, how to complete forward, and the yank policy for a bad publish.

## 1. Code Freeze

- Confirm release branch and freeze window.
- Stop non-release feature merges.
- Confirm open blockers are triaged.

## 2. Changelog Finalization

- Ensure root changelog and per-crate changelogs are updated. The per-crate changelogs are now
  stamped by the release tooling (`make release`'s changelog finalization step, extended
  alongside the root changelog) rather than edited by hand across all ten crates.
- **The root `## [VERSION]` section is a hard prerequisite for the release run, not just good
  practice.** Tagging without it fails the `create-release` job outright — there is no fallback
  body source. If this happens, the fix is to finalize the changelog (`make release
  VERSION=x.y.z`) and re-tag; see [Release Automation](release-automation.md#body-source).
- Ensure notable breaking changes are explicitly called out.
- Verify release notes map to merged changes.

## 3. Version Bump

- Apply lockstep version update across public crates.
- Verify crate dependency versions remain aligned.
- Re-check Cargo.toml metadata completeness.

## 4. CI and Local Validation

Run and require success for:

- cargo test --workspace
- cargo fmt --all -- --check
- cargo clippy --workspace -- -D warnings
- cargo doc --workspace --no-deps
- cargo audit

## 5. Dry-Run Publish Validation

Run one workspace-wide dry-run, which packages and verifies all twelve publishable crates in
dependency order in a single command:

1. paladin-ai-core
2. paladin-ports
3. paladin-herald
4. paladin-battalion, paladin-llm, paladin-memory, paladin-web, paladin-notifications,
   paladin-content, paladin-storage (leaf tier)
5. paladin-eval
6. paladin-ai

Use:

- `cargo publish --workspace --dry-run` (or `make publish-dry-run`)

`paladin-doc-examples` is skipped automatically because it is marked `publish = false`.
`paladin-eval` is included here even though the `semver` CI job excludes it from the
published-baseline diff (it has no published `0.9.0` baseline to diff against — ADR-0048); its
dry-run packaging and dependency resolution are still verified like every other crate.

The workspace form resolves intra-workspace dependencies from local paths instead of against the
crates.io registry, so every crate — including one that depends on a sibling not yet
published — verifies cleanly in dependency order before anything is actually published. There is
no "expect dependent dry-runs to fail until prerequisites are available" caveat with this
command, unlike a per-crate `cargo publish --dry-run -p <crate>` loop, which resolves each
dependent's pinned version against the registry and fails until its dependencies are actually
live there.

## 6. Publish

Publish in dependency-first order:

1. paladin-ai-core
2. paladin-ports
3. paladin-herald
4. paladin-battalion, paladin-llm, paladin-memory, paladin-web, paladin-notifications,
   paladin-content, paladin-storage (leaf tier)
5. paladin-eval
6. paladin-ai

After each publish, verify crate availability on crates.io before continuing.

Publishing authenticates through crates.io Trusted Publishing, from the `publish-crates` job under
the `crates-io` GitHub Environment — there is no token to configure. See the per-crate trust table
and credential history in [Release Automation](release-automation.md#trusted-publishing).

## 7. Tag and Announcement

- Create and push release tag.
- Publish release notes.
- Announce release in project communication channels.
- Confirm docs.rs build status for published crates.

## 8. Post-Release Verification

- Re-run quick smoke tests on published versions.
- Verify dependency resolution for a downstream sample app.
- Download the archives plus `SHA256SUMS` from the release and run the one-command verification:
  `sha256sum -c SHA256SUMS` (or `shasum -a 256 -c SHA256SUMS` on macOS).
- Pull the container image by the immutable digest the release body names
  (`docker pull <image>@sha256:<digest>`), rather than trusting a mutable tag.
- Confirm the release body matches the root `CHANGELOG.md` `## [X.Y.Z]` section for that version.
- Each target carries **three** binaries (`paladin`, `paladin-cli`, `paladin-server`) in its
  archive — an operator who sees only one asset per target knows something went wrong. See
  [Release Automation](release-automation.md#artifact-inventory) for the full inventory table.
- Log follow-up items for next release cycle.
