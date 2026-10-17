---
status: complete
phase: 50-provider-parity-config-driven-model-swap
source: [50-VERIFICATION.md]
started: 2026-10-16T22:51:11Z
updated: 2026-10-17T16:03:45Z
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
result: issue
reported: "Row 74: don't dismiss the provider's total token counts -- those are what the vendor charges for, and the Treasurer and Commissary depend on these counts; record as a gap and create a gap-closure plan. Row 75: option (a) -- switch the OpenAI adapter to always send max_completion_tokens. Row 76: option (a) -- align the Kimi adapter-wide fallback profile with the seeded Kimi profiles (temperature, top_p and penalties not sent)."
severity: major

## Summary

total: 5
passed: 4
issues: 1
pending: 0
skipped: 0
blocked: 0

## Gaps

<!-- YAML format for plan-phase --gaps consumption -->
- gap_id: G-50-5a
  truth: "Every billable token category a vendor reports reaches a TokenUsage sub-count (prompt, completion, cache-read, cache-write, reasoning), so Treasurer pricing (cost_of_call) and the allowance check see the full billed usage; the vendor's own total is parsed and reconciled against prompt + completion rather than silently discarded, while the D-02 invariant total_tokens == prompt + completion is kept (WINDOWS row 74)"
  status: failed
  reason: "User reported: don't dismiss the provider's total token counts -- those are what the vendor charges for, and the Treasurer and Commissary depend on these counts; record as a gap and create a gap-closure plan."
  severity: major
  test: 5
  artifacts: []
  missing: []
- gap_id: G-50-5b
  truth: "The OpenAI adapter sends the output-token limit as max_completion_tokens, so the seeded default model gpt-5.4-nano accepts requests built from OpenAIConfig::new / from_env defaults instead of rejecting max_tokens with HTTP 400 (WINDOWS row 75, owner chose option (a))"
  status: failed
  reason: "User reported: Row 75: option (a) -- switch the OpenAI adapter to always send max_completion_tokens."
  severity: major
  test: 5
  artifacts: []
  missing: []
- gap_id: G-50-5c
  truth: "The Kimi adapter-wide fallback profile (unseeded Kimi model ids) matches the seeded Kimi profiles: temperature, top_p, frequency_penalty and presence_penalty are not sent, replacing the stale 2026-08-22 Fixed(1.0)/Fixed(0.95)/(-2.0..2.0) declarations that the 2026-10-15 probe contradicts (WINDOWS row 76, owner chose option (a))"
  status: failed
  reason: "User reported: Row 76: option (a) -- align the Kimi adapter-wide fallback profile with the seeded Kimi profiles (temperature, top_p and penalties not sent)."
  severity: minor
  test: 5
  artifacts: []
  missing: []
