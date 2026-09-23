# API Coverage — Phase 37.1 (v0.10.1 Patch Release)

No external API integration: this phase ships no product capability against any external API — it
cuts a patch release that corrects the partial `v0.10.0` publish, and its only external calls are
release tooling (read-only `gh` queries plus the release-PR and tag operations, read-only
`index.crates.io` sparse-index and `crates.io` version probes for the per-crate resolution gate and
the post-publish registry sweep, and `git`), none of which is wired into `paladin` at all; the only
source-tree edits are manifest version bumps, the `paladin-battalion` dev-dependency correction,
the `CRATES` ordering gate script, the `create-or-reuse-release.sh` pipefail fix, and changelog /
`MIGRATION.md` rows. No new code path under `src/` or `crates/*/src/` calls any external service.

The `api-coverage` detector reads `detected: true` over the finished plan bodies, on the sentence in
`37.1-01-PLAN.md` that itself states "this phase integrates no external API or SDK". That is a
scope declaration, not a capability surface being integrated, so per the checkpoint's own third
branch this reasoned declaration stands in place of a coverage matrix. Fabricating matrix rows for
capabilities the product does not have would be the failure this gate exists to prevent, inverted.

Scope note for the seal-time gate: the publish path (crates.io Trusted Publishing via
`rust-lang/crates-io-auth-action`) is pre-existing, already-audited release infrastructure adopted
in Phase 19 and first exercised in Phase 37, not an integration this phase builds. Its verification
lives in `37.1-CI-EVIDENCE.md`'s registry table and `trustpub_data` reading, not in a capability
matrix. Precedent: Phase 37's `COVERAGE.md` resolved the identical detector reading the same way.
