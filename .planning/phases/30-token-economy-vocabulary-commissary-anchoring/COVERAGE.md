# Phase 30 — API Coverage Declaration

**Detector:** `bin/lib/api-coverage.cjs --json` over the Phase 30 ROADMAP section + the three
PLAN bodies.

| Run | Scope | Result |
|-----|-------|--------|
| 1 (pre-plan) | ROADMAP Phase 30 section only | `detected: false`, `signals: []` |
| 2 (post-plan) | ROADMAP section + `30-01/02/03-PLAN.md` | `detected: true`, one signal: verb `(surface)`, noun `api` |

**No external API integration: this phase writes documentation only — three ADRs, one mdBook page,
two doc tables, five rustdoc lines and two comment edits — and adds no dependency, no client, no
endpoint and no call to any external service.**

The single post-plan signal is a false positive on the literal string `INVENTED API:` inside plan
30-01 Task 2's anti-invention verify gate — a shell `echo` that fails the task if the new mdBook
page names a method or type absent from `crates/paladin-llm/src/services/commissary.rs`. The
matched noun is `api` in that error message; the "verb" is the detector's `(surface)` fallback, not
an integration verb. Re-reading the phase scope confirms the classification:

- `Cargo.toml` / `Cargo.lock`: unchanged in all three plans (asserted in each plan's acceptance
  criteria). 30-RESEARCH.md records "Package Legitimacy Audit: not applicable — this phase installs
  no new dependencies".
- No HTTP client, SDK, webhook, OAuth flow, MCP server or provider call is added or configured.
- The only *existing* external-provider surfaces the phase mentions are the already-shipped LLM
  adapters, and it mentions them as **documentation subjects** (naming the real per-request
  `max_tokens` surfaces in `configuration.md`), never as integration targets.

No capability matrix is produced, because fabricating one for a phase that integrates nothing would
be noise rather than coverage.
