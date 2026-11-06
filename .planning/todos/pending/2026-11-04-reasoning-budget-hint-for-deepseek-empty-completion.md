---
created: 2026-11-04T00:00:00Z
title: Budget-aware default or reasoning-share hint for reasoning models that spend max_tokens thinking
area: llm-providers
severity: minor
files:
  - docs/src/user-guides/reasoning-effort.md
  - crates/paladin-llm/src/response_guards.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
owner: repo maintainer
recheck_by: 2026-11-09
dispositioned_by: Phase 56 plan 56-10 (DF-34)
---

## Problem

web3sec reports (return report section 7, item 5; DF-34) that its pre-disclosed-match and triage calls to
`deepseek-flash` intermittently come back `Empty completion: finish_reason=length ... reasoning likely consumed the
entire max_tokens budget` (2 of 3 repeats failed, 1 succeeded, under the same tight `max_tokens`). It asks for a
"Paladin-side note on reasoning-model budgets, and a possible budget-aware default or reasoning-share hint for
DeepSeek".

## What HEAD already does

- The note exists: `docs/src/user-guides/reasoning-effort.md`, "Effort, output budget and timeout", says reasoning
  tokens are output tokens, that the budget must cover the reasoning and the answer, and that a call which spends
  the whole cap thinking finishes `length` with no text and returns `LlmError::EmptyCompletion`, a runtime backstop
  and not a validation.
- The error text itself advises "retry with a larger max_tokens" (`response_guards.rs`, `empty_payload`).
- v0.11.0 made the condition typed on purpose (M-B-16); it was a silent empty `Ok` on DeepSeek before.

## What is not done, and why it is not in v0.11.1

A budget-aware default (a floor on `max_tokens` for a seeded reasoning model) or a reasoning-share hint is a new
behaviour with a design question (does the adapter raise a caller's explicit budget? warn? refuse?). The milestone is
a no-new-features patch, and Phase 56 holds no captured response body for the failure (the report gives the error
string only). CONTEXT D-00d lets provider-measurement work in only when a run hands over a captured body.

## Entry condition

A later milestone that takes reasoning-model budgets, or a captured `finish_reason=length` body with
`reasoning_tokens` from web3sec's DeepSeek runs. Start by deciding which of "warn", "refuse" or "raise" the adapter
may do to an explicit `max_tokens`.

## Update 2026-11-05 (Phase 58 pre-tag audit, R-65)

Acknowledged and kept open by the Phase 58 pre-tag audit (`58-AUDIT-DISPOSITIONS.md` R-65). Owner: repo maintainer. Re-check by 2026-11-09, the maintainer's `DATE` recorded in `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`.
