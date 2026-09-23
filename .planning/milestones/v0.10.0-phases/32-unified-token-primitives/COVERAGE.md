# Phase 32 — API Coverage Declaration

**Detector:** `bin/lib/api-coverage.cjs --json` over the Phase 32 ROADMAP section plus the five PLAN
bodies.

Detector runs (recorded as a list, not a table — the `verify:pre` seal gate parses ANY pipe table in
this file as coverage-matrix rows, and a declaration alongside rows is rejected as contradictory):

- Run 1 (pre-plan), scope ROADMAP Phase 32 section only: `detected: false`, `signals: []`
- Run 2 (post-plan), scope ROADMAP section + `32-01` through `32-05-PLAN.md`: `detected: true`, one
  signal — verb `consume`, noun `api`

**No external API integration: this phase is a pure in-tree Rust refactor. It adds one defaulted
method to an existing port trait, removes a dead legacy trait and its factory, drops one argument from
two constructors, extracts one shared pure function from two duplicated precedence walks, and writes
the matching release-register rows. It adds no dependency, no HTTP client, no SDK, no endpoint and no
call to any external service.**

The single post-plan signal is a false positive on the phrase "new public API in `paladin-llm` that
two services and the facade will **consume**" inside plan 32-02's `<reversibility>` element. "API"
there means this workspace's own Rust public API surface — the thing `cargo semver-checks` diffs —
and "consume" means two in-tree callers calling an in-tree function. Neither is an external
integration verb in the sense this gate exists to catch. Re-reading the phase scope confirms the
classification:

- `Cargo.toml` `[dependencies]` tables are unchanged in all five plans. The only manifest edits in
  the phase are `[package.metadata.cargo-semver-checks.lints]` lines in plan 32-05, which configure a
  lint tool and pull in nothing. 32-RESEARCH.md's Package Legitimacy Audit records: "No external
  packages are installed by this phase"; the existing optional `tiktoken-rs` dependency is untouched.
- No provider adapter, HTTP client, webhook, OAuth flow or MCP server is added, configured or called.
  The phase does not touch `crates/paladin-llm`'s provider adapters at all — only its `services` and
  the new `window` module, both of which are pure, synchronous and I/O-free.
- The one *existing* external surface the phase mentions is `ProviderCapabilities::max_context_tokens`,
  a value each already-shipped adapter declares locally. The phase reads that field through a pure
  function; it makes no provider call, so there is no capability surface to enumerate and no opt-out
  to reason about.

No capability matrix is produced, because fabricating rows for a phase that integrates nothing would
be noise rather than coverage.
