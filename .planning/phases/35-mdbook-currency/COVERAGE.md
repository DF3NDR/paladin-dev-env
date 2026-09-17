# Phase 35 — API Coverage Declaration

**Detectors:** `bin/lib/api-coverage.cjs --json`, run at seal time (2026-09-17) over the
Phase 35 ROADMAP section and each of the ten PLAN bodies individually.

Detector runs (recorded as a list, not a table — the `verify:pre` seal gate parses ANY pipe table
in this file as coverage-matrix rows, and a declaration alongside rows is rejected as
contradictory):

- api-coverage scan, scope ROADMAP Phase 35 section: `detected: false`, `signals: []`
- api-coverage scan, 35-01/02/03/05/06/08/09/10-PLAN.md: `detected: false`, `signals: []`
- api-coverage scan, 35-04-PLAN.md: `detected: true`, one `(surface) api` signal — the MB-09
  acceptance line listing the shipped surface `architecture/overview.md` must *name*
  (`WarEngine`, `Battlefield`, `Waypoint`, `Aegis`, `Commissary`, the platform API, …)
- api-coverage scan, 35-07-PLAN.md: `detected: true`, one `(surface) api` signal — the
  `cli-configuration.md` scheduler-troubleshooting note stating the scheduler is wired into the
  platform API's `/v1/schedules*` route family, which the page is corrected to *point at*

No external API integration: this phase is a documentation-currency sweep over the already-shipped
v0.10.0 mdBook (`docs/src/`) plus its compile-verified `crates/doc-examples` companions. The only
APIs it names — the platform API's route family, the OpenAI/DeepSeek/Anthropic provider adapters,
the `LlmPort`/`ArsenalPort`/`SanctumPort` port traits — are documentation subjects whose *shape* the
pages are corrected to match, never integration targets the phase calls, wraps or configures.

Both PLAN-body signals are the `<Service> API` surface rule matching the phrase "Platform API" in
prose that describes what a page must name or link. Re-reading the phase scope confirms the
classification:

- No provider adapter, HTTP client, webhook, OAuth flow or MCP server is added, configured or
  called. The only manifest change in the phase is `crates/doc-examples` (new `superstep_engine`,
  `paladin_agents`, `arsenal_tools`, `herald_output`, `battalion_patterns`, `sanctum_vector_memory`
  modules registered in its `lib.rs`), a compile-only crate whose sole purpose is to keep the
  book's `{{#include}}` snippets type-checked against the workspace; it adds no dependency edge to
  any external service and `make api-surface` reports the public surface unchanged
  (35-EVIDENCE.md).
- Every command the phase runs (`mdbook-mermaid install`, `mdbook build` with the `linkcheck`
  backend, `scripts/check-doc-examples.sh`, `scripts/check-doc-config.sh`, `make api-surface`, the
  live `paladin-cli <cmd> --help` captures that rebuilt the CLI appendix pages, the
  `cargo check --example _scratch` import-path probes) is a local, offline build/lint tool already
  vendored and pinned in the devcontainer.
- The platform API routes, provider adapter constructors and port-trait signatures that pages such
  as `architecture/overview.md`, `appendix/cli-configuration.md`, `appendix/provider-expansion.md`
  and `appendix/sentinel.md` are corrected to describe are read from the committed source tree
  (`crates/paladin-web`, `crates/paladin-llm`, `crates/paladin-ports`) and reproduced as text; the
  phase never issues a request against any of them.

No capability matrix is produced, because fabricating rows for a phase that integrates nothing
would be noise rather than coverage.
