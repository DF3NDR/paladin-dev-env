# API Coverage — Phase 37 (v0.10.0 Crate Release)

No external API integration: this phase ships no product capability against any external API — it
cuts a release, and its only external calls are release tooling (read-only `gh` queries plus one
`gh pr create`, read-only `index.crates.io` and `crates.io` version lookups, and `git`), none of
which is wired into `paladin` at all; no file under `src/` or `crates/*/src/` is modified by any
plan in this phase.

The `api-coverage` detector reads `detected: true` over the finished plan bodies, on the phrase
"GitHub API" inside a plan's trust-boundary table. That is release tooling described in a threat
model, not a capability surface being integrated, so per the checkpoint's own third branch this
reasoned declaration stands in place of a coverage matrix. Fabricating matrix rows for capabilities
the product does not have would be the failure this gate exists to prevent, inverted.

Scope note for the seal-time gate: the publish path (crates.io Trusted Publishing via
`rust-lang/crates-io-auth-action`) is pre-existing, already-audited release infrastructure adopted
in Phase 19, not an integration this phase builds. Its verification lives in
`37-CI-EVIDENCE.md`'s registry table and the `trustpub_data` reading, not in a capability matrix.
