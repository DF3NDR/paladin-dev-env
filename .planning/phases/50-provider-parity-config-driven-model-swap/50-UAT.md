---
status: testing
phase: 50-provider-parity-config-driven-model-swap
source: [50-VERIFICATION.md]
started: 2026-10-16T22:51:11Z
updated: 2026-10-17T07:24:13Z
---

## Current Test

number: 3
name: mdBook build of docs/src with the Switching providers page
expected: |
  `mdbook build` exits 0; `docs/src/user-guides/switching-providers.md` is reachable from the SUMMARY nav and its per-provider/per-model and response-mapping tables render. Only grep-level link checks and the Rust registry drift test ran here.
awaiting: user response

## Tests

### 1. CI coverage job on the Phase 50 head (82 % workspace floor, ADR-0006)
expected: The `coverage` job is green on the Phase 50 head; cargo-llvm-cov is absent here, so only CI can measure it.
result: pass

### 2. CI test-integration-docker job (Docker-bound integration suites)
expected: All Docker-backed suites (Redis, RustFS, Postgres, Ollama Tier 2) are green on the Phase 50 head; Docker is absent here.
result: pass

### 3. mdBook build of docs/src with the Switching providers page
expected: `mdbook build` exits 0; `docs/src/user-guides/switching-providers.md` is reachable from the SUMMARY nav and its per-provider/per-model and response-mapping tables render. Only grep-level link checks and the Rust registry drift test ran here.
result: [pending]

### 4. Operator credential-gated smoke run on the finished tree
expected: |
  `cargo run -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`
  prints PASS for every configured provider, SKIPPED for unconfigured ones, and exits non-zero only if nothing was configured.
  The 2026-10-15 --probe run is on record in 50-SEED-PROBE.md; the plain smoke mode has no run on the final tree.
result: [pending]

### 5. Owner decision on open ledger rows WINDOWS 74, 75, 76
expected: |
  Row 74: keep `total_tokens = prompt + completion` (D-02 invariant) or open a paladin-core plan to carry the vendor total.
  Row 75: OpenAI adapter sends `max_tokens`, which `gpt-5.4-nano` rejects with HTTP 400; decide whether to switch to `max_completion_tokens` per model.
  Row 76: the Kimi adapter-wide fallback profile (Fixed 1.0 temperature) contradicts the 2026-10-15 probe for unseeded Kimi ids.
  Each is an accepted, ledgered deviation that needs an owner call code cannot make.
result: [pending]

## Summary

total: 5
passed: 2
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
