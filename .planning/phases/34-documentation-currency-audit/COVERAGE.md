# Phase 34 — API Coverage Declaration

**Detectors:** `bin/lib/api-coverage.cjs --json` and the assumption-delta scanner, both run over
the Phase 34 ROADMAP section at plan time.

Detector runs (recorded as a list, not a table — the `verify:pre` seal gate parses ANY pipe table
in this file as coverage-matrix rows, and a declaration alongside rows is rejected as
contradictory):

- api-coverage scan, scope ROADMAP Phase 34 section: `detected: false`, `signals: []`
- assumption-delta scan, scope ROADMAP Phase 34 section: `detected: false`, `signals: []`

No external API integration: this phase is a read-only documentation measurement over an
already-shipped tree. The only APIs it names — the platform API, the provider APIs — are
documentation subjects the audit reads about, never integration targets it calls. The phase adds
no dependency, no network call and no new input-handling code.

Both plan-time detector runs returned `detected: false` on this phase's ROADMAP section — recorded
here so the seal-time re-scan over the PLAN bodies has the reasoned declaration it needs even if a
PLAN body's own prose (naming `cargo doc`, `mdbook build`, HTTP routes it merely reads about from
`MIGRATION.md` §9.6, or provider adapter names it greps for in rustdoc) trips the term vocabulary
without the phase doing any actual integration work. Re-reading the phase scope confirms the
classification:

- No `Cargo.toml` manifest in this phase is edited at all — Success Criterion 5 (D-22) forbids any
  change outside `.planning/`, and no dependency, feature flag or `[[example]]` target is added,
  removed or reconfigured by any of this phase's nine plans.
- No provider adapter, HTTP client, webhook, OAuth flow or MCP server is added, configured or
  called. Every command this phase runs (`cargo doc`, `cargo build --examples`, `cargo test
  --workspace --doc`, `mdbook build docs/`, the `scripts/check-doc-*.sh` gates) is a local,
  offline build/lint tool already vendored and pinned in the devcontainer (34-RESEARCH.md
  "Environment Availability" — no missing dependency, no fallback needed).
- The platform API and provider APIs the audit's shipped-surface checklist (D-08) reads about
  (`POST /v1/runs`, the OpenAI/DeepSeek/Anthropic adapters, `MIGRATION.md` §9.6's HTTP surface)
  are read as text from already-committed release documents (`CHANGELOG.md`, `MIGRATION.md`,
  `REQUIREMENTS.md`) — the audit never issues a request against any of them.

No capability matrix is produced, because fabricating rows for a phase that integrates nothing
would be noise rather than coverage.
