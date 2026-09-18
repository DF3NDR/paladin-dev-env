# Phase 31: Lossless Token Accounting - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-14
**Phase:** 31-lossless-token-accounting
**Mode:** `--auto` — every question resolved to the recommended option without prompting; each
selection is logged below so the operator can audit or overturn it before planning.
**Areas discussed:** `total_tokens` semantics and optional-field contract; Carrier shape, naming,
arithmetic and `from_total`; Streaming usage parity; Adapter population of cache/reasoning
fields; Herald and CLI observability; Edge surfaces (HTTP/SSE/OpenAPI); Compatibility mechanics
(serde, X-10.3, MIGRATION/allowlist/CHANGELOG); Test strategy and gates

`[--auto] Selected all gray areas: total_tokens semantics; Carrier shape; Streaming parity; Adapter population; Herald/CLI observability; Edge surfaces; Compatibility mechanics; Test strategy.`

---

## `total_tokens` semantics and the optional-field contract

| Option | Description | Selected |
|--------|-------------|----------|
| Inclusive "of which" (recommended) | `total = prompt + completion`; cache/reasoning are sub-counts already inside those two; adapters normalise (Anthropic `prompt = input + cache_read + cache_creation`, Gemini `completion = candidates + thoughts`) | ✓ |
| Exclusive / additive | `total = prompt + completion + cache_read + cache_write + reasoning`; matches Anthropic's raw wire, contradicts OpenAI/Gemini's | |
| Provider-defined | `total` is whatever the provider reports; semantics vary per adapter | |

`[auto] total_tokens — Q: "Does total_tokens include cache and reasoning tokens?" → Selected: "Inclusive 'of which'" (recommended default)`
`[auto] total_tokens — Q: "What does None vs Some(0) mean?" → Selected: "None = not reported, Some(0) = reported zero; never fabricate" (recommended default)`
`[auto] total_tokens — Q: "#[non_exhaustive] or constructible for TokenUsage?" → Selected: "Constructible, option (b), PaladinResult precedent — it has Default" (recommended default)`
`[auto] total_tokens — Q: "Keep u32 or widen to u64?" → Selected: "Keep u32 with saturating arithmetic; widening deferred" (recommended default)`

**Notes:** Inclusive semantics keep `TokenUsage::new`, `PartialEq`, `TokenBudget` and the eval
`total_tokens_max` assertion unchanged and let the Milestone 14 pricing function compute
discounted cache pricing by subtraction.

---

## Carrier shape: naming, constructibility, aggregation arithmetic, `from_total`

| Option | Description | Selected |
|--------|-------------|----------|
| Field named `usage` (recommended) | Mirrors `LlmResponse.usage`; `ExecutionMetadata.token_usage` keeps its name | ✓ |
| Field named `token_usage` | Mirrors `ExecutionMetadata` | |
| Delete `from_total` entirely (recommended) | Removes the information-losing constructor; fixtures use `new` | ✓ |
| Keep `from_total` off the battalion path | Satisfies the letter of ACCT-02 but leaves the footgun | |
| `BattalionResult.total_tokens` stays beside the map (recommended) | Bare-count rule: a total may coexist with a full carrier, never be the only one | ✓ |
| Replace `BattalionResult.total_tokens` with `TokenUsage` | Consistent with `RunFinished` but touches every strategy service and herald for no new information | |
| `Add`/`AddAssign`/`Sum` on `TokenUsage` (recommended) | One shared saturating accumulator with the Option-merge rule | ✓ |
| Ad-hoc summation at each site | Status quo; four divergent accumulators | |

`[auto] Carrier — Q: "Name of the carrier field?" → Selected: "usage" (recommended default)`
`[auto] Carrier — Q: "What happens to TokenUsage::from_total?" → Selected: "Delete entirely" (recommended default)`
`[auto] Carrier — Q: "BattalionResult.total_tokens?" → Selected: "Keep beside per_paladin_tokens (bare-count rule)" (recommended default)`
`[auto] Carrier — Q: "Where does accumulation live?" → Selected: "Add/AddAssign/Sum on the type, saturating" (recommended default)`
`[auto] Carrier — Q: "Convenience accessor on PaladinResult?" → Selected: "None — result.usage.total_tokens suffices (PRD R3)" (recommended default)`

---

## Streaming usage parity

| Option | Description | Selected |
|--------|-------------|----------|
| `StreamingResponse.usage: Option<TokenUsage>` on the terminal chunk, struct `#[non_exhaustive]` (recommended) | Adapters hold the finish reason until the usage frame; consumer keeps stopping at the first finished chunk | ✓ |
| Separate trailing usage chunk after finish | Requires every consumer to keep draining past `finish_reason`; breaks the `if is_final { return; }` consumer | |
| Cumulative per-chunk usage | Provider-shape-dependent; ambiguous for OpenAI-family | |
| Explicit exception for the generic OpenAI-compatible server (recommended) | `None` when the server ignores `stream_options`; documented in rustdoc + provider page | ✓ |
| Estimate via `TokenCounterPort` when streamed usage is missing | Mixes estimates into billed counts; the Treasurer would price an estimate | |
| One shared conformance case (recommended) | `streaming_usage_equals_non_streaming_usage` in `llm_conformance_suite!`; suite extended to every adapter with a real stream parser | ✓ |
| Hand-written test per adapter only | No shared bar; the gap that created the conformance suite | |

`[auto] Streaming — Q: "How does usage travel on the stream?" → Selected: "Terminal-chunk contract on StreamingResponse.usage" (recommended default)`
`[auto] Streaming — Q: "Fallback when a provider omits streamed usage?" → Selected: "No estimation; TokenUsage::default() + one warn! log; explicit documented exception" (recommended default)`
`[auto] Streaming — Q: "Where is the parity proven?" → Selected: "Shared conformance case, all adapters instantiate it (or an equivalent test)" (recommended default)`
`[auto] Streaming — Q: "How does the execution service expose streamed usage?" → Selected: "ChunkMetadata.usage on the final PaladinStreamChunk" (recommended default)`

---

## Adapter population of cache/reasoning fields

| Option | Description | Selected |
|--------|-------------|----------|
| Populate where the provider's existing usage object carries the figure (recommended) | OpenAI/DeepSeek `*_details`, Anthropic cache + thinking, Gemini cached + thoughts; `None` elsewhere | ✓ |
| Struct fields only, no producer | Herald breakdown would show `null` everywhere; ACCT-04 unobservable in practice | |
| Also add provider-side request flags (e.g. enable caching) | New capability; out of scope | |

`[auto] Adapters — Q: "Do adapters populate the new fields in this phase?" → Selected: "Yes, from existing usage objects, never fabricated" (recommended default)`
`[auto] Adapters — Q: "Correct Anthropic prompt_tokens to include cached input?" → Selected: "Yes — it is the D-02 invariant; flagged in CHANGELOG as a corrected under-report" (recommended default)`

---

## Herald and CLI observability

| Option | Description | Selected |
|--------|-------------|----------|
| JSON: full `usage` object, optionals always present as `null` (recommended) | Stable key set for machine consumers | ✓ |
| JSON: skip `None` optionals | Smaller payload; unstable key set | |
| Markdown: prompt/completion/total always, optionals only when `Some` (recommended) | Human-facing; omit unreported | ✓ |
| Markdown: render "n/a" for unreported | Noisy | |
| TableHerald: discretion, total column minimum (recommended) | Not required by ACCT-04 | ✓ |

`[auto] Heralds — Q: "Which heralds carry the breakdown?" → Selected: "JsonHerald and MarkdownHerald (both required by ACCT-04); TableHerald discretion" (recommended default)`
`[auto] Heralds — Q: "JSON null policy?" → Selected: "Always emit the three optional keys" (recommended default)`

---

## Edge surfaces: HTTP DTOs, SSE, inspector, OpenAPI baseline

| Option | Description | Selected |
|--------|-------------|----------|
| Carry the full object at the HTTP edge via a `TokenUsageResponse` DTO; regenerate `openapi.json`; §9.6 row (recommended) | Uses the clean-break window; avoids a second break after 1.0 | ✓ |
| Map `usage.total_tokens` into the existing `token_count` DTO fields | Zero OpenAPI churn, but re-collapses the split one layer above `RunFinished` | |

`[auto] Edge — Q: "Do the HTTP response DTOs change shape?" → Selected: "Yes — TokenUsageResponse in paladin-web, make openapi, §9.6" (recommended default)`

---

## Compatibility mechanics: serde, X-10.3, MIGRATION/allowlist/CHANGELOG

| Option | Description | Selected |
|--------|-------------|----------|
| `#[serde(default)]` on new fields, old keys dropped, no legacy-shape deserializer, §9.4 note (recommended) | Waypoints/traces are new-in-untagged-0.10; only dev databases exist | ✓ |
| Custom deserializer mapping legacy `token_count` → `usage.total_tokens` | Preserves dev-database history at the cost of a shim-shaped intermediate struct | |
| Empirical lint discovery via `cargo semver-checks` per package, one `[[entry]]` per firing lint (recommended) | Guarantees the D-04 row-level gate passes | ✓ |
| Guess lint ids from precedent | Risks a red `semver` job | |
| CHANGELOG entries in `[0.10.0]` (recommended) | ACCT-05 wording; untagged release | ✓ |
| CHANGELOG entries in `[Unreleased]` | Contradicts ACCT-05 | |

`[auto] Compat — Q: "Legacy JSON / persisted Waypoint policy?" → Selected: "serde(default) + drop old keys + §9.4 note; no mapping shim" (recommended default)`
`[auto] Compat — Q: "How are allowlist rows derived?" → Selected: "Run cargo semver-checks and mirror what fires" (recommended default)`
`[auto] Compat — Q: "Which CHANGELOG section?" → Selected: "[0.10.0]" (recommended default)`

---

## Test strategy and gates

| Option | Description | Selected |
|--------|-------------|----------|
| Engine-level round-trip via `RecordingPaladinPort` + Formation/Phalanx split tests + serde tests (recommended) | Mirrors existing test patterns at the exact seams | ✓ |
| Single end-to-end HTTP test only | Slower, less precise about which carrier lost data | |

`[auto] Tests — Q: "Where does the ACCT-02 round-trip test live?" → Selected: "crates/paladin-battalion engine tests next to the existing token_count record test" (recommended default)`

---

## Todo cross-reference

- Folded (score 0.6): "Verify local `make coverage` reproduces CI's 82.39 % figure" — as a
  verification note under ACCT-05 only.
- Reviewed, not folded (score 0.6): "Evaluate replacing MinIO with RustFS" — keyword false
  positive; the `>= 0.4` auto-fold rule was overridden because folding unrelated infrastructure
  work would violate the phase scope guardrail. Logged here for audit.

## Claude's Discretion

- D-02 rustdoc wording; `Mutex` vs atomics in `TraceDispatcher`; `StreamingResponse`
  constructor names; `TableHerald` columns; CLI text; DTO module placement; plan/commit
  granularity (five natural waves listed in CONTEXT.md).

## Deferred Ideas

- Estimation fallback for missing streamed usage (Milestone 14 opt-in candidate).
- Widening counters to `u64`.
- Currency cost / pricing / `cost_estimate` / allowances / pacing (Milestone 14).
- Per-chunk token hints untouched.
- Shared `TokenUsage` human formatter.
