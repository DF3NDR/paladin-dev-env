---
phase: 30-token-economy-vocabulary-commissary-anchoring
fixed_at: 2026-09-14T22:05:23Z
review_path: .planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-REVIEW.md
iteration: 1
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 30: Code Review Fix Report

**Fixed at:** 2026-09-14T22:05:23Z
**Source review:** .planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 1
- Fixed: 1
- Skipped: 0

## Fixed Issues

### WR-01: `llm.anthropic.max_tokens` is not a real config key

**Files modified:** `docs/src/getting-started/configuration.md`
**Commit:** 06b25fc8
**Applied fix:** Corrected the "Per-request completion cap" row of the Token Budget
Terminology table (line 414). Removed the false claim that `llm.anthropic.max_tokens`
is a settable YAML config key — `LlmProviderConfig` (`crates/paladin-llm/src/config/llm.rs`)
has no `max_tokens` field, and `AnthropicConfig::from_env()`
(`crates/paladin-llm/src/anthropic/adapter.rs`) reads only the `ANTHROPIC_MAX_TOKENS`
environment variable, disjoint from the YAML `llm.anthropic.*` block. The cell now reads:

> `LlmRequest` metadata `"max_tokens"` (OpenAI/DeepSeek fallback-override); `ANTHROPIC_MAX_TOKENS` env var, read by `AnthropicConfig::from_env()` (Anthropic, required, env-only — no YAML key)

Verified the edited row still has the same pipe/column count (4 pipes, 3 columns) as the
table header and neighboring rows before committing. Docs-only change; no Rust code
touched, so `cargo test` was not required. Committed with the `cargo-fmt`/`cargo-clippy`
pre-commit hooks enabled (both passed, ~10s on a warm target).

## Skipped Issues

None — the only in-scope finding was fixed.

---

_Fixed: 2026-09-14T22:05:23Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
