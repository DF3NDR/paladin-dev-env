---
created: 2026-11-18T00:00:00Z
title: "z.ai: add the `llm.zai` config block and finish full provider integration"
area: paladin-llm
severity: warning
files:
  - crates/paladin-llm/src/config/llm.rs
  - crates/paladin-llm/src/provider_factory.rs
  - crates/paladin-llm/src/registry/seeded.rs
  - tests/provider_swap_matrix.rs
  - MIGRATION.md
  - .cargo/semver-checks-allowlist.toml
  - scripts/check-migration-allowlist.sh
  - .planning/quick/261117-t5c-we-need-to-create-a-new-ai-provider-adap/261117-t5c-PLAN.md
owner: v0.12.0 milestone
dispositioned_by: quick task 261117-t5c (owner decision 2026-11-17, env-only now, config block next milestone)
---

## Problem

Quick task 261117-t5c shipped the z.ai (GLM) adapter `paladin_llm::zai` (feature `zai`, facade
`llm-zai`) configured through `ZAI_API_KEY`, `ZAI_BASE_URL`, `ZAI_MODEL` and `ZAI_TIMEOUT_SECONDS`
only. An `llm.zai` YAML section is not read, and the docs say so. The owner wants z.ai to be a
full provider with its own `LlmConfig` block like the others.

## What to do

1. Add `LlmConfig.zai: Option<LlmProviderConfig>` and drive construction from it. This is a
   deliberate semver break (`LlmConfig` is a public, constructible struct), so the release is at
   least 0.12.0. The work is the "option-a" branch of the quick task's PLAN.md Task 3: MIGRATION.md
   section 12 with its 12.2 allowlist row, a `.cargo/semver-checks-allowlist.toml` entry, and
   `scripts/check-migration-allowlist.sh` plus its test re-pointed to section 12.2.
2. Add z.ai to `tests/provider_swap_matrix.rs` and the `config_example_roundtrip` fixtures.
3. Replace the "is not read" statements (rustdoc in `config/llm.rs`, `config.example.yml`, the docs
   pages and crate README touched by `6858079`) and remove the test that pins the block as unread.
4. Run `live_vendor_smoke` with a real `ZAI_API_KEY` to produce live evidence, then seed the model
   table and effort profile in `registry/seeded.rs`. Until then every z.ai model is unprofiled and
   explicit reasoning effort is refused.
5. Once z.ai is seeded, add its rows to the switching-providers and reasoning-effort docs tables
   (drift tests currently pin those tables to the nine seeded providers).
6. Settle the stop-sequence cap: docs.z.ai prose says one stop word, its schema says `maxItems: 4`.
   The adapter currently sends at most one (fail-closed). Confirm with the live probe.
7. Three `provider_factory` tests fail on `--no-default-features --features zai` because they need
   a provider with a config block; they should pass once the block exists. Confirm.
8. Coverage of `zai/adapter.rs` was never measured locally (no `cargo llvm-cov`); check the CI
   `coverage` job figure.
