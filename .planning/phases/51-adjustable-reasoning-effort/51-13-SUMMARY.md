---
phase: 51-adjustable-reasoning-effort
plan: 13
subsystem: docs-and-decisions
tags: [reasoning-effort, mdbook, drift-test, adr-0062, adr-0004-amendment, mapping-table, kimi-guidance]
status: complete

requires:
  - phase: 51-03
    provides: "admit, the refusal text, the metadata keys, ModelOverride.effort"
  - phase: 51-06
    provides: "llm.<provider>.reasoning_effort / reasoning_fallback / reasoning_wire, per-request precedence"
  - phase: 51-07
    provides: "PaladinBuilder::reasoning_effort, the build-time refusal, the chain pre-flight"
  - phase: 51-08
    provides: "agents[].reasoning_effort and the key-naming refusals"
  - phase: 51-09
    provides: "NodeFinished trace fields and the node-cache fingerprint"
  - phase: 51-11
    provides: "Kimi, Qwen and Grok probe-confirmed wires and seeded effort data"
  - phase: 51-12
    provides: "Anthropic, OpenAI, DeepSeek and Gemini probe-confirmed wires and seeded effort data"
provides:
  - "docs/src/user-guides/reasoning-effort.md: the Reasoning Effort guide with a 9-provider declared-wire table and a 62-model per-model table"
  - "registry::reasoning_page_tests: a 14-case drift test that derives every table cell from the nine built adapters"
  - "A measured kimi-k3 effort / output-budget / timeout section and one llm.kimi example (low, 16000, 300)"
  - "ADR-0062 (Accepted) amending ADR-0004, with the ADR-0004 and ADR-0061 dated lines and the PROMOTION index row, next free number 0063"
affects: [51-14]

tech-stack:
  added: []
  patterns:
    - "a docs table whose every cell is derived from the built adapters through EffortSupport::resolve, so the page cannot drift from the code"
    - "adapters built once per test process behind a OnceLock when a test mutates the page dozens of times"

key-files:
  created:
    - docs/src/user-guides/reasoning-effort.md
    - .planning/decisions/0062-reasoning-effort-fail-loud-policy.md
    - .planning/phases/51-adjustable-reasoning-effort/deferred-items.md
  modified:
    - crates/paladin-llm/src/registry/mod.rs
    - docs/src/SUMMARY.md
    - docs/src/user-guides/switching-providers.md
    - docs/src/appendix/provider-expansion.md
    - .planning/decisions/0004-temperature-validation.md
    - .planning/decisions/0061-per-model-parameter-support-policy.md
    - .planning/decisions/PROMOTION.md

key-decisions:
  - "The mapping table has two sub-tables under `## Mapping table`: a declared-wire row per provider and a row per seeded model, so a provider with no wire (Ollama, OpenAI-compatible) states why in prose while model cells stay a closed vocabulary."
  - "Unmeasured seeded models (gpt-4o-mini, chat-latest, grok-4.20-non-reasoning) get a row that reads `refused` everywhere and `unmeasured`, rather than no row, so the page cannot be read as silent about them."
  - "The wire-field text for the seven named vendors is a test-side table; whether a provider ships a wire at all is read from the adapter."
  - "The page's one Kimi example is `low`, `max_tokens: 16000`, `timeout_seconds: 300`, the combination the 2026-10-13 measurement ran (stop in 56 s)."

patterns-established:
  - "Plan 51-14 can run the page's llm.kimi block verbatim against the live API."

requirements-completed: []
requirements-progress: [REASON-02, REASON-06, REASON-07]

duration: about 3h (cold worktree build, tracer checkpoint round trip)
completed: 2026-10-22
---

# Phase 51 Plan 13: Reasoning Effort guide, drift-checked mapping table and ADR-0062 Summary

**An mdBook "Reasoning Effort" page whose mapping table is derived from the nine built adapters by a 14-case drift test, a measured `kimi-k3` effort/budget/timeout example, and ADR-0062 recording the exact-or-refuse policy as an amendment of ADR-0004.**

REASON-02, REASON-06 and REASON-07 advance but are not marked complete: plan 51-14 (the surface register, the live re-run of the documented Kimi example and CHANGELOG completeness) remains. `requirements-completed` is empty; REQUIREMENTS.md, STATE.md and ROADMAP.md are untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| (deviation, Rule 3) | Define the Switching Providers footnote markers so mdbook linkcheck passes | `c69b7cf6` |
| 1 | Tracer: the page and its drift test on DeepSeek, checked against the built adapter | `cb549e74` |
| 2 | The full nine-provider table and the guidance, with the measured Kimi example | `5918ed83` |
| 3 | ADR-0062, the ADR-0004 and ADR-0061 dated lines, the PROMOTION index | `ecd85ffe` |

The Task 1 tracer gate was resolved by the user ("approved") before Tasks 2 and 3, including the footnote deviation below.

## What was built

- **The page** (`docs/src/user-guides/reasoning-effort.md`, linked from `docs/src/SUMMARY.md` after Switching Providers). Sections in order: values and spellings; where to set it (request, agent, provider, vendor default, with `LlmRequest::with_reasoning`, `PaladinBuilder::reasoning_effort`, `agents[].reasoning_effort`, `llm.<provider>.reasoning_effort` / `*Config::with_reasoning_effort`); when the model cannot honour it (`LlmError::UnsupportedParameter`, `Permanent`, before HTTP, the error text quoted); `reasoning_fallback` and what it never substitutes; unmeasured models and the `llm.<provider>.models.<id>.effort` override shape; fallback chains (per-hop pre-flight, `fallback hop <index>`); build-time checks and the key-naming refusals; self-hosted endpoints (`reasoning_wire`, model levels, the `live_vendor_smoke --effort` proof: the invalid-value-rejected row and the low-versus-high effect row); observability (`paladin.reasoning_requested` / `paladin.reasoning_effective`, `NodeFinished`, `TokenUsage.reasoning_tokens`, `LlmResponse.reasoning`); `## Mapping table`; `## Effort, output budget and timeout`; notes and limits.
- **The mapping table.** `### Declared wire per provider`: nine rows (seven with their field and the 2026-10-22 date; Ollama and OpenAI-compatible `unverified:` with the reason). `### Effort per model`: 62 rows, one per seeded model, cells `exact`, `refused`, `refused (coerced)`, `budget <min>-<max>`, and the probe date or `unmeasured`. A vendor-by-vendor reading guide follows (OpenAI `none` for off, Anthropic's four-model adaptive pairing, DeepSeek's coerced levels, Kimi's `thinking.effort` and the never-sent top-level field, Qwen's no-named-levels, Grok's coerced `xhigh` on `grok-4.5`, Gemini's lower-case `thinkingLevel`, no native `Budget`).
- **The drift test** (`registry::reasoning_page_tests`, beside `switching_page_tests`, gated on all nine features). Builds the nine adapters once per process with a dummy key and no network, reads `model_profile(id)` (after wire masking) and asks `EffortSupport::resolve(effort, Refuse)` for each of Off, Auto, Minimal..Max; reads the budget span and `effort_verified_on`; compares the page's two tables row by row (rows missing, extra and out of order, then every cell, naming the provider, model and column). Whether a provider ships a wire is read from the adapter (`WireUnsupported`): such a provider's wire cell must read `unverified:` plus a reason. Mutation cases: an edited cell in a scratch copy read through `check_page_in`, a deleted row for **every** measured seeded model of every provider, an extra row, a missing or edited wire row, an edited date, a missing heading, an unmeasured row edited to `exact`, a wire-less provider naming a field. A tree without `docs/` is a recorded `Skipped`; `docs/` without the page fails.
- **Effort, output budget and timeout.** Reasoning tokens count against `max_tokens`; `EmptyCompletion` (finish `length`) is the backstop, not a validation; the 300 s reasoning default. The 2026-10-13 `kimi-k3` table (low 56 s / high 359 s / max and no field `length`-empty / off 2 s / low at 64 tokens `length`-empty) and the 2026-10-22 probe table (low 40, high 48, max 116 reasoning tokens at a 2,048 cap; `minimal`, `medium`, `xhigh` rejected; off and auto accepted). The one `llm.kimi` block: `reasoning_effort: low`, `max_tokens: 16000`, `timeout_seconds: 300`, each value traced in a comment to the measurement it came from and stated as configuration, never a compiled default. `kimi-k3`'s seeded row lists low, high, max, off and auto, so the value is admitted by the adapter and by the build-time check.
- **ADR-0062** (`.planning/decisions/0062-reasoning-effort-fail-loud-policy.md`): Status Accepted, Phase 51, **Amends ADR-0004**, extends ADR-0061. Decisions (a) to (k) cover D-00d..D-00i and D-01..D-11, each naming its code mechanism; an Owner checkpoints table records the 2026-10-22 answers of plans 51-10, 51-11 and 51-12; Considered Options lists every alternative CONTEXT rejected; Consequences carry the reversibility ratings (D-03, D-05, D-10 costly) and the `reason` field; Code Locations; Code Conformance `conforms` with the named tests. ADR-0004 gains `**Amended by ADR-0062**` under its Status; ADR-0061 gains one dated pointer to `51-EFFORT-PROBE.md` (its seed-rule text is byte-identical); `PROMOTION.md` appends the 0062 row and reads `**Next free ADR number: 0063**` with a dated note.

## TDD record

- **Task 1:** RED first: with no page, 8 of 11 new tests failed ("docs/ exists but .../reasoning-effort.md cannot be read"). GREEN with the DeepSeek rows. One test indexed the wrong column (the cell edit hit `Auto`, not `Low`); the message named it and the test was fixed, not the check.
- **Task 2:** the nine-adapter infrastructure was added and the page's rows were generated from the adapters (a temporary printing test, removed before the commit), so the table starts true by construction and the mutation cases prove the check is not vacuous. The first run of the delete-every-measured-row case took 172 s because each of about 60 iterations rebuilt nine adapters; the adapters are now built once behind a `OnceLock` (the module runs in about 1 s).
- **Task 3:** documentation only; its verify command (the plan's grep chain) exits 0.

## Verification

- `cargo test --offline -p paladin-llm --all-features --lib`: 1494 passed, 0 failed. `reasoning_page_tests`: 14 passed. `registry::`: 88 passed (`switching_page_tests` unaffected).
- `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings` clean; `cargo fmt -p paladin-llm --check` clean.
- `mdbook build docs` exits 0 with linkcheck "No broken links found" (after `mdbook-mermaid install docs/`, see below); every in-page anchor the page links to exists in the rendered HTML. `./scripts/check-doc-config.sh`: 160 YAML blocks, 0 failed. The page's inline Rust blocks have no `fn main`, so `check-doc-examples.sh` skips them by its own rule; the script was not run as a whole.
- The Task 2 greps (`## Effort, output budget and timeout`, `reasoning_effort`, `live_vendor_smoke`) and the Task 3 verify chain exit 0. `make check-gates` exits 0 (migration allowlist, API examples, consumer surface included).
- **`make clean-code` does not exit 0** on this tree: its `doc-check` step reports three warnings in files other plans own (see Deferred Issues). fmt and clippy pass. `make security` was not run: no dependency was added or changed.
- Credential scan of the page: no key-shaped string and no literal key; the examples use `${MOONSHOT_API_KEY}` and `${DEEPSEEK_API_KEY}` placeholders only. The only key-like literal added is the `"test-key"` dummy in the cfg(test) module, the repository's existing pattern. No vendor call was made.
- Manual credential-handling review (security.instructions.md): no client, header, redirect-policy, `Debug` or error-body path was touched; the test builds adapters with no network call.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `mdbook build docs` already failed at the base commit**
- **Found during:** Task 1 verify. `mdbook-linkcheck` 0.7.7 (the CI version) with `warning-policy = error` read the 155 footnote markers `[a]`..`[i]` on the Switching Providers page as "Potential incomplete link", so the plan's required `mdbook build` exited 101. Reproduced on a scratch copy of `docs/` with this plan's page and SUMMARY entry removed: the same 155 errors, none in the new page.
- **Fix:** nine hidden reference definitions (`[a]: #per-provider-and-per-model-behaviour` and so on) appended to `docs/src/user-guides/switching-providers.md`. No marker text changed, so `switching_page_tests` reads the same page. Side effect: the in-table markers now render as links to that section.
- **Commit:** `c69b7cf6`. Shown to the user at the tracer checkpoint and accepted.

**2. Build prerequisite, not a code change:** the book needs `mdbook-mermaid install docs/` before it builds, because `docs/mermaid.min.js` and `docs/mermaid-init.js` are gitignored and generated (CI does the same step). It writes only ignored files.

### Notes against the plan's wording

- **Two sub-tables, not one.** The plan describes one table with a `(declared wire)` provider row. A 12-column model table cannot carry a wire field plus a prose reason without a vocabulary exception, so the page has `### Declared wire per provider` (4 columns) and `### Effort per model` (12 columns) under `## Mapping table`. Both are drift-checked.
- **Cell vocabulary extended** with `refused (coerced)` (a coerced level refuses under every fallback, and the reader should see why) and `no-op` (reserved for `NoReasoning` `Off`; no seeded model uses it), in addition to the plan's `exact`, `refused` and `budget <min>-<max>`.
- **Unmeasured models are listed.** `gpt-4o-mini`, `chat-latest` and `grok-4.20-non-reasoning` have rows reading `refused` and `unmeasured`, so the table covers all 62 seeded models (the plan required rows for measured models and left the rest open).
- **Wire-field text** for the seven named vendors is a test-side table; the drift test reads from the adapters only whether a provider has a wire at all (the compat engine's wire is a private field outside `registry`). Each wire is pinned by its adapter's wire-body tests and the conformance case `seeded_effort_levels_reach_the_wire`.
- **Page order.** The mapping table follows the guidance and precedes the budget/timeout section, because 62 rows at the top would push the explanation off the first screen.
- **PROJECT.md `## Key Decisions` row** (promotion step 6) is not added: the plan does not list PROJECT.md and it is a shared planning artifact. The orchestrator or the phase transition should add the ADR-0062 row.
- **Commit hooks:** commits used `--no-verify` as the wave allows; the trailer follows the attribution reminder (`Claude Sonnet 5.5`).

## Deferred Issues

Logged in `.planning/phases/51-adjustable-reasoning-effort/deferred-items.md` (found by `make clean-code`'s rustdoc zero-warning step, none in a file this plan touches): an unresolved `[`NODE_CACHE_KEY_VERSION`]` doc link in `paladin-battalion/src/engine/cache_key.rs` (plan 51-09); two redundant explicit link targets in `paladin-llm/src/reasoning.rs` (plan 51-03); and a `dead_code` warning for `thinking_effort_wire_value` in the default-feature test build of `paladin-llm/src/conformance.rs` (plan 51-12). Each is a one-line fix; `make doc-check` fails until the first two are made.

## Authentication Gates

None. No live vendor call was made.

## Known Stubs

None. Deliberate limits, all stated on the page: only a provider-wide default exists (no per-model default); the provider-level default is not in the node-cache key; the OpenAI Responses API, the command-line tool's agent YAML and facade `AgentSpec` agents carry no effort; no native model accepts `Budget`; Ollama and OpenAI-compatible ship `unverified`.

## Threat Flags

None. The only new surface is documentation and a cfg(test) module. T-51-38 (a documented mapping drifting from the code) is mitigated by `reasoning_page_tests`; T-51-39 (a documented Kimi example that exhausts its budget or times out) by taking the example's values from the 2026-10-13 measurement (low, 16,000 cap, stop in 56 s against a 300 s timeout), with plan 51-14 to re-run it live.

## Self-Check: PASSED

- Files present: `docs/src/user-guides/reasoning-effort.md`, `.planning/decisions/0062-reasoning-effort-fail-loud-policy.md`, `.planning/phases/51-adjustable-reasoning-effort/deferred-items.md`, and every file in `key-files.modified`.
- Commits `c69b7cf6`, `cb549e74`, `5918ed83` and `ecd85ffe` present on `worktree-agent-a2eb16aa98b074555`.
- `ls .planning/decisions/0062-*.md` lists exactly one file; ADR-0061's diff is the one appended pointer line.
