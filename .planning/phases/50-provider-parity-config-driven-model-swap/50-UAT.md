---
status: complete
phase: 50-provider-parity-config-driven-model-swap
source: [50-VERIFICATION.md, 50-19-SUMMARY.md, 50-20-SUMMARY.md, 50-21-SUMMARY.md, 50-22-SUMMARY.md, 50-23-SUMMARY.md, 50-24-SUMMARY.md, 50-25-SUMMARY.md, 50-26-SUMMARY.md, 50-27-SUMMARY.md, 50-28-SUMMARY.md, 50-29-SUMMARY.md, 50-30-SUMMARY.md, 50-31-SUMMARY.md]
started: 2026-10-16T22:51:11Z
updated: 2026-10-19T02:09:17Z
---

## Current Test

[testing complete]

## Tests


### 1. CI coverage job on the Phase 50 head (82 % workspace floor, ADR-0006)
expected: The `coverage` job is green on the Phase 50 head; cargo-llvm-cov is absent here, so only CI can measure it.
result: pass

### 2. CI test-integration-docker job (Docker-bound integration suites)
expected: All Docker-backed suites (Redis, RustFS, Postgres, Ollama Tier 2) are green on the Phase 50 head; Docker is absent here.
result: pass

### 3. mdBook build of docs/src with the Switching providers page
expected: `mdbook build` exits 0; `docs/src/user-guides/switching-providers.md` is reachable from the SUMMARY nav and its per-provider/per-model and response-mapping tables render. Only grep-level link checks and the Rust registry drift test ran here.
result: pass

### 4. Operator credential-gated smoke run on the finished tree
expected: |
  `cargo run -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`
  prints PASS for every configured provider, SKIPPED for unconfigured ones, and exits non-zero only if nothing was configured.
  The 2026-10-15 --probe run is on record in 50-SEED-PROBE.md; the plain smoke mode has no run on the final tree.
result: pass

### 5. Owner decision on open ledger rows WINDOWS 74, 75, 76
expected: |
  Row 74: keep `total_tokens = prompt + completion` (D-02 invariant) or open a paladin-core plan to carry the vendor total.
  Row 75: OpenAI adapter sends `max_tokens`, which `gpt-5.4-nano` rejects with HTTP 400; decide whether to switch to `max_completion_tokens` per model.
  Row 76: the Kimi adapter-wide fallback profile (Fixed 1.0 temperature) contradicts the 2026-10-15 probe for unseeded Kimi ids.
  Each is an accepted, ledgered deviation that needs an owner call code cannot make.
result: pass
resolved_by: [50-19-PLAN.md, 50-20-PLAN.md, 50-21-PLAN.md, 50-22-PLAN.md, 50-23-PLAN.md, 50-24-PLAN.md, 50-25-PLAN.md, 50-26-PLAN.md, 50-27-PLAN.md, 50-28-PLAN.md, 50-29-PLAN.md, 50-30-PLAN.md, 50-31-PLAN.md]
resolved_at: 2026-10-19
reported: "Row 74: don't dismiss the provider's total token counts -- those are what the vendor charges for, and the Treasurer and Commissary depend on these counts; record as a gap and create a gap-closure plan. Row 75: option (a) -- switch the OpenAI adapter to always send max_completion_tokens. Row 76: option (a) -- align the Kimi adapter-wide fallback profile with the seeded Kimi profiles (temperature, top_p and penalties not sent)."
severity: major

### 6. OpenAI default model gpt-5.4-nano accepts the request (G-50-5b, WINDOWS 75)
expected: |
  With OPENAI_API_KEY set, `cargo run -p paladin-llm --example live_vendor_smoke --features openai` prints PASS for openai against its new default model `gpt-5.4-nano` (no HTTP 400). The request body carries `max_completion_tokens` and no `max_tokens` key on `generate`, `generate_stream` and a `ResponseFormat::JsonSchema` call; `--dry-run` shows the same body without credentials.
result: pass

### 7. Unseeded Kimi model id runs on the no-sampling fallback (G-50-5c, WINDOWS 76)
expected: |
  A call to a Kimi id that is not seeded (anything other than kimi-k2.6 / kimi-k3) is ACCEPTED (200) and its request body carries no temperature, top_p, frequency_penalty or presence_penalty, exactly like the seeded Kimi rows; max_tokens is still sent. The Switching providers fallback row and note [f] describe this NotSent profile, not the old Fixed(1.0)/Fixed(0.95) one.
result: pass

### 8. Live usage-containment probe on record (G-50-5a)
expected: |
  `live_vendor_smoke --usage` makes one call per seeded model and prints one containment verdict per line (reasoning inside completion, reasoning beside completion, indeterminate, no vendor total). The 2026-10-18 run in 50-USAGE-PROBE.md shows all three Grok models beside completion with the billed-cost cross-check consistent to the tick on each, and Kimi/DeepSeek/Qwen inside. The adapter semantics match those verdicts: Grok keeps ReasoningBesideCompletion (now labelled measured), no other provider flipped, and no new WINDOWS row was needed.
result: pass

### 9. Vendor total is reconciled, never discarded (G-50-5a, WINDOWS 74)
expected: |
  On every adapter except Anthropic (which reports no total), the vendor's total_tokens / totalTokenCount is parsed and compared with prompt + completion. `total_tokens` stays `prompt + completion` (D-02). When they disagree, the response (and a stream's terminal chunk) carries `paladin.usage_unreconciled` metadata reading `total:vendor=N,mapped=M`, and one warn line per (provider, model) is logged at target `paladin::usage`. Gemini `toolUsePromptTokenCount` folds into prompt; Grok reasoning folds into completion; Anthropic vision cache read/write fold into prompt on both vision surfaces.
result: pass

### 10. Billed empty and refused calls carry usage, cost and a ledger row (G-50-5a)
expected: |
  An `LlmError::EmptyCompletion { .. }` or `LlmError::Refusal { .. }` from any of the nine adapters, buffered or streamed, returns `Some` from `billed_usage()` with the served model and the vendor's token counts; behind `PricingLlmAdapter` the bill carries a cost. In the agent loop the failed call is folded into the run's usage and cost and settled once to the treasury ledger under its own ordinal. A hopped-past fallback refusal is priced at its own model and settled in the call's one row with a per-model breakdown; failed, timed-out and structured Battalion attempts count their recorded spend. Matrix case `billed_failures_and_hopped_refusals_reach_the_ledger` passes under `--features llm-all`.
result: pass

### 11. CI coverage job on the gap-closure head (82 % workspace floor, ADR-0006)
expected: The `coverage` job is green on the head that carries plans 50-19..50-31 (commit e0b01fdc or later); cargo-llvm-cov is absent here, so only CI can measure it.
result: pass

### 12. CI test-integration-docker job on the gap-closure head
expected: All Docker-backed suites (Redis, RustFS, Postgres, Ollama Tier 2) are green on the head that carries plans 50-19..50-31; Docker is absent here.
result: pass

### 13. mdBook build with the rewritten usage, billing and migration docs
expected: |
  `mdbook build` exits 0. In `docs/src/user-guides/switching-providers.md` the `## Responses` section names every fold (Grok reasoning, Gemini thoughts and tool-use prompt, Anthropic cache read/write including vision), each of the nine reasoning cells carries a `measured 2026-10-18` or `not live-measured` label, the Billing bullet describes `billed_usage()`, hop bills and failed attempts, the refusal pattern reads `Refusal { provider, model, detail, .. }`, and Known limits lists only remainder (c). MIGRATION 9.1 rows M-B-18..M-B-22 and 9.8 rows DR-17/DR-18 render.
result: pass

### 14. Owner sign-off on the breaking change and the one open ledger row
expected: |
  Two items are recorded as owner decisions, not code gaps. (1) `paladin-ports` `LlmError::EmptyCompletion` changed from a tuple to a struct variant (semver `enum_tuple_variant_changed_kind`, allowlisted with a reason; MIGRATION DR-17 / M-B-22; root and crate CHANGELOG Breaking Changes). (2) WINDOWS row 77 stays open by the 2026-10-17 decision: Grok's `cost_in_usd_ticks` / `num_sources_used` are not parsed or compared with the Treasurer price (G-50-5a remainder (c)); ADR-0061 limit (6), the root CHANGELOG known limitation and the guide's Known limits name it. Rows 74, 75 and 76 read `fixed`.
result: pass

## Summary

total: 14
passed: 14
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

<!-- YAML format for plan-phase --gaps consumption -->
- gap_id: G-50-5a
  truth: "Every billable token category a vendor reports reaches a TokenUsage sub-count (prompt, completion, cache-read, cache-write, reasoning), so Treasurer pricing (cost_of_call) and the allowance check see the full billed usage; the vendor's own total is parsed and reconciled against prompt + completion rather than silently discarded, while the D-02 invariant total_tokens == prompt + completion is kept (WINDOWS row 74)"
  status: resolved
  resolved_by: [50-21-PLAN.md, 50-22-PLAN.md, 50-23-PLAN.md, 50-24-PLAN.md, 50-25-PLAN.md, 50-26-PLAN.md, 50-27-PLAN.md, 50-28-PLAN.md, 50-29-PLAN.md, 50-30-PLAN.md, 50-31-PLAN.md]
  resolved_at: 2026-10-19
  reason: "User reported: don't dismiss the provider's total token counts -- those are what the vendor charges for, and the Treasurer and Commissary depend on these counts; record as a gap and create a gap-closure plan."
  severity: major
  test: 5
  root_cause: "TokenUsage's inclusive contract (prompt_tokens = every billed input token, completion_tokens = every billed output token, total = prompt + completion, D-02) is assumed but never verified at the adapter boundary, and the vendor's own total -- the one figure that could verify it -- is deserialised and deliberately discarded (#[allow(dead_code)]). (1) CompatEngine::map_compat_usage applies OpenAI inclusive semantics to every compat preset; xAI Grok reports completion_tokens EXCLUSIVE of completion_tokens_details.reasoning_tokens (vendor total = prompt + completion + reasoning; secondary-sourced, not yet live-measured), so Grok reasoning calls lose reasoning tokens from total_tokens and break reasoning <= completion, and cost_of_call's reasoning.min(completion) clamp then prices the call at ~half (repro 1.163M vs 2.403M nanos). Grok cost_in_usd_ticks / num_sources_used are not parsed. (2) GeminiUsageMetadata never deserialises toolUsePromptTokenCount (latent: GeminiRequest sends no tools today; repro 430 -> 130). (3) Every adapter runs classify_completion(...)? before mapping usage, so a vendor-billed call classified EmptyCompletion/Refusal yields no TokenUsage, no cost and no ledger settle. OpenAI, Anthropic, DeepSeek and Gemini (thoughts) fold all billed token categories correctly on success. The Commissary reads no response usage."
  artifacts:
    - path: "crates/paladin-llm/src/compat/engine.rs"
      issue: "map_compat_usage (~641) applies one OpenAI-semantics mapping to all compat presets; Grok reasoning outside completion is dropped"
    - path: "crates/paladin-llm/src/compat/types.rs"
      issue: "CompatUsage.total_tokens is #[allow(dead_code)] (~126); vendor total never reconciled"
    - path: "crates/paladin-llm/src/gemini/adapter.rs"
      issue: "GeminiUsageMetadata (~1331) lacks toolUsePromptTokenCount; totalTokenCount dead_code"
    - path: "crates/paladin-llm/src/openai/adapter.rs"
      issue: "vendor total_tokens dead_code (~352)"
    - path: "crates/paladin-core/src/platform/container/cost.rs"
      issue: "cost_of_call (~386) clamps reasoning to completion, silently hiding an invariant violation"
    - path: "crates/paladin-llm (all adapters)"
      issue: "classify_completion(...)? short-circuits before usage mapping; billed-but-failed calls carry no usage/cost"
  missing:
    - "Per-vendor usage mapping that folds every billed category into the inclusive parent (Grok reasoning into completion; Gemini toolUsePromptTokenCount into prompt) so prompt + completion == vendor total"
    - "Parse the vendor total on every adapter and surface any residual mismatch (warn/metric, optionally an Option field) instead of discarding it; keep D-02"
    - "Carry usage on billed-but-failed calls (EmptyCompletion/Refusal) so the Treasurer can settle them"
    - "Per-vendor fixtures where vendor total != prompt + completion, plus a live/probe measurement of Grok reasoning containment"
  debug_session: ".planning/debug/vendor-token-usage-fidelity.md"
- gap_id: G-50-5b
  truth: "The OpenAI adapter sends the output-token limit as max_completion_tokens, so the seeded default model gpt-5.4-nano accepts requests built from OpenAIConfig::new / from_env defaults instead of rejecting max_tokens with HTTP 400 (WINDOWS row 75, owner chose option (a))"
  status: resolved
  resolved_by: 50-19-PLAN.md
  resolved_at: 2026-10-19
  reason: "User reported: Row 75: option (a) -- switch the OpenAI adapter to always send max_completion_tokens."
  severity: major
  test: 5
  root_cause: "The private OpenAIRequest struct (crates/paladin-llm/src/openai/adapter.rs:222-248) serializes its max_tokens field (line 229, no serde rename) as the wire key max_tokens; build_request (529-578) is the only constructor and feeds generate, generate_stream and every response_format variant. OpenAIConfig::new/from_env default max_tokens Some(4096) and PromptParameters defaults Some(1000), so the key is always sent and gpt-5.4-nano always 400s. OpenAIVisionApiRequest.max_tokens (openai/vision.rs:112) is a second wire site (unreachable from gpt-5.4-nano today). Not caught because live_vendor_smoke drives gpt-4o-mini, which still accepts max_tokens. No other adapter reuses the struct."
  artifacts:
    - path: "crates/paladin-llm/src/openai/adapter.rs"
      issue: "OpenAIRequest.max_tokens (line 229) emits wire key max_tokens; doc comments 38-43, 109-114, 521-524, 538-539 describe it; wire tests at ~1890, 2382, 2401, 2494, 3021-3042 pin it"
    - path: "crates/paladin-llm/src/openai/vision.rs"
      issue: "OpenAIVisionApiRequest.max_tokens (line 112) also emits max_tokens"
    - path: "docs/src/user-guides/switching-providers.md"
      issue: "Known-limits bullet (~189-191) documents the HTTP 400"
    - path: "CHANGELOG.md"
      issue: "Known limitations item (3) (~1255-1257)"
    - path: ".planning/WINDOWS.md"
      issue: "row 75 open"
  missing:
    - "Rename the OpenAI wire key to max_completion_tokens (serde rename) on OpenAIRequest and OpenAIVisionApiRequest; keep OpenAIConfig.max_tokens, PromptParameters.max_tokens and check_max_tokens wording"
    - "Flip the wire tests (red first) and add a gpt-5.4-nano body test across generate, stream and json_schema asserting max_completion_tokens present and max_tokens absent"
    - "Update docs, CHANGELOG (remove limitation, add Fixed entry noting only gpt-5.4-nano and gpt-4o-mini were measured), close WINDOWS row 75"
    - "Consider pointing live_vendor_smoke at gpt-5.4-nano"
  debug_session: ".planning/debug/openai-max-completion-tokens.md"
- gap_id: G-50-5c
  truth: "The Kimi adapter-wide fallback profile (unseeded Kimi model ids) matches the seeded Kimi profiles: temperature, top_p, frequency_penalty and presence_penalty are not sent, replacing the stale 2026-08-22 Fixed(1.0)/Fixed(0.95)/(-2.0..2.0) declarations that the 2026-10-15 probe contradicts (WINDOWS row 76, owner chose option (a))"
  status: resolved
  resolved_by: 50-20-PLAN.md
  resolved_at: 2026-10-19
  reason: "User reported: Row 76: option (a) -- align the Kimi adapter-wide fallback profile with the seeded Kimi profiles (temperature, top_p and penalties not sent)."
  severity: minor
  test: 5
  root_cause: "KimiAdapter::new (crates/paladin-llm/src/kimi/adapter.rs) builds the adapter-wide fallback from two stale 2026-08-22 sources plan 50-06 left in place when it seeded kimi-k2.6/kimi-k3 as NotSent: (1) the explicit .with_temperature(ParamSupport::Fixed(1.0)).with_top_p(ParamSupport::Fixed(0.95)) overrides (lines 337-339); (2) request_parameters declaring temperature/top_p/frequency_penalty/presence_penalty carried = true (lines 295-301), which registry::fallback_from_compat (registry/mod.rs:662-694) turns into Range(-2.0, 2.0) penalties. No gate compares the fallback with the seeded rows or the probe, and four Kimi unit tests pin the stale values."
  artifacts:
    - path: "crates/paladin-llm/src/kimi/adapter.rs"
      issue: "Fixed(1.0)/Fixed(0.95) overrides (337-339); request_parameters flags true (295-301); temperature_range Some((1.0, 1.0)) (~274); four unit tests pin the stale values (e.g. kimi_model_profile_declares_the_two_measured_single_values, generate_omits_top_p_from_the_moonshot_request_body, get_capabilities_reports_the_measured_degenerate_temperature_range)"
    - path: "crates/paladin-llm/src/registry/mod.rs"
      issue: "fallback_from_compat maps carried penalties to Range(-2.0, 2.0)"
    - path: "docs/src/user-guides/switching-providers.md"
      issue: "fallback row (~216) and note [f] (~251) describe the stale profile"
    - path: ".planning/WINDOWS.md"
      issue: "row 76 open"
  missing:
    - "Set the four request_parameters flags false and delete the Fixed overrides (the Grok idiom) so the fallback declares temperature, top_p, frequency_penalty and presence_penalty NotSent, matching seeded.rs kimi-k2.6/kimi-k3"
    - "Revisit temperature_range in capabilities consistent with the lib.rs test_every_adapter_declares_a_temperature_range invariant"
    - "Rewrite the four pinning tests (red first) and add a test that the fallback matches the seeded Kimi sampling declarations"
    - "Update the docs fallback row and note [f], CHANGELOG, close WINDOWS row 76"
  debug_session: ".planning/debug/kimi-fallback-profile-stale.md"
