---
phase: 30-token-economy-vocabulary-commissary-anchoring
reviewed: 2026-09-14T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - .github/copilot-instructions.md
  - .project/project-management/paladin-project-plan-final.md
  - crates/paladin-core/src/platform/container/herald.rs
  - docs/src/SUMMARY.md
  - docs/src/architecture/commissary.md
  - docs/src/architecture/domain-model.md
  - docs/src/getting-started/configuration.md
  - src/lib.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
status: issues_found
---

# Phase 30: Code Review Report

**Reviewed:** 2026-09-14T00:00:00Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

Phase 30 is documentation-only. `git diff f22661e3b501f1e1dd64d81a1159ad950588ae8f..HEAD` confirms
the two `.rs` files in scope (`crates/paladin-core/src/platform/container/herald.rs`, `src/lib.rs`)
changed only comment/doc-comment lines — no signature, behavior, or test change. The `herald.rs`
diff replaces a previously-misleading `total_cost()`/`cost_estimate` doc claim ("calculates a
basic estimate") with an accurate one ("reserved for the Treasurer, no in-tree producer, returns
`None`"); this was verified against the actual method body (`self.cost_estimate` returned as-is,
unchanged) and against a repo-wide grep for `.cost_estimate(` callers — the only caller in the
tree is a test, confirming the new doc wording is correct where the old wording was not.

The new `docs/src/architecture/commissary.md` page was checked line-by-line against
`crates/paladin-llm/src/services/commissary.rs`: every named type (`Consignment`,
`ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile`, `CommissaryPlan`, `CommissaryError`
and its 5 variants), every `Commissary` method (`new`, `from_port`, `verify_fits`, `dispense`,
`allotted_tokens` — confirmed to be the *complete* set of public methods on `Commissary`, matching
the page's "no more than these five" claim), every struct field, and every cited source line range
(`commissary.rs:631-686`, `:911-929`, `:230-292`) checked out exactly.

One factual error was found in the newly-added "Token Budget Terminology" table in
`docs/src/getting-started/configuration.md`: it names `llm.anthropic.max_tokens` as a real,
settable YAML config key. It is not — see WR-01.

All other cross-checked claims (`garrison.max_tokens`, `rag.max_tokens`,
`agent_runtime.token_budget.max_tokens`, `ANTHROPIC_MAX_TOKENS`, the `Commissary` row and
`ADR-0049` reference added to `domain-model.md` and `.github/copilot-instructions.md`, the
`Commissary` re-export comment update in `src/lib.rs`, and the `docs/src/SUMMARY.md` nav entry)
were verified against the source tree and are accurate.

## Warnings

### WR-01: `llm.anthropic.max_tokens` is not a real config key

**File:** `docs/src/getting-started/configuration.md:414`
**Issue:** The new "Token Budget Terminology" table's "Per-request completion cap" row states:

> `LlmRequest` metadata `"max_tokens"` (OpenAI/DeepSeek fallback-override); `llm.anthropic.max_tokens` / `ANTHROPIC_MAX_TOKENS` (Anthropic, required)

`llm.anthropic.max_tokens` does not exist as a settable configuration path. The struct that backs
YAML `llm.anthropic.*` is `LlmProviderConfig` (`crates/paladin-llm/src/config/llm.rs:9-21`), whose
fields are `api_key`, `base_url`, `default_model`, `default_temperature`, `timeout_seconds`,
`max_retries` — there is no `max_tokens` field. Separately, the `AnthropicConfig` struct that
*does* have a `max_tokens: u32` field (`crates/paladin-llm/src/anthropic/adapter.rs:53`) is
constructed exclusively via `AnthropicConfig::from_env()`
(`crates/paladin-llm/src/provider_factory.rs:88-92`, `construct_anthropic`), which reads only the
`ANTHROPIC_MAX_TOKENS` environment variable (`crates/paladin-llm/src/anthropic/adapter.rs:80-83`).
There is no code path anywhere in the tree that copies a YAML `llm.anthropic.max_tokens` value
into that struct — the two config layers (`LlmProviderConfig` for the six-provider YAML block and
`AnthropicConfig::from_env()` for the actual adapter construction) are disjoint for this field.

An operator who reads this table and sets `llm.anthropic.max_tokens: 8192` in `config.yml`
expecting it to raise the Anthropic completion cap will have that value silently ignored — the
adapter will still use `ANTHROPIC_MAX_TOKENS` (or its 4096 default) regardless. This is exactly
the kind of misleading-but-plausible config key the "Honesty about exactness" ethos elsewhere in
this phase's own `commissary.md` page argues against.

**Fix:** Drop the `llm.anthropic.max_tokens` half of the cell and document only the real path:

```diff
-| Per-request completion cap | `LlmRequest` metadata `"max_tokens"` (OpenAI/DeepSeek fallback-override); `llm.anthropic.max_tokens` / `ANTHROPIC_MAX_TOKENS` (Anthropic, required) | provider adapters (`crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/anthropic/adapter.rs`) |
+| Per-request completion cap | `LlmRequest` metadata `"max_tokens"` (OpenAI/DeepSeek fallback-override); `ANTHROPIC_MAX_TOKENS` env var, read by `AnthropicConfig::from_env()` (Anthropic, required, env-only — no YAML key) | provider adapters (`crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/anthropic/adapter.rs`) |
```

---

_Reviewed: 2026-09-14T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
