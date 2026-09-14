# PRD: Treasurer — Cross-Run Spend Governance + Cost Ledger (Milestone 14, Epic 1)

**Project:** Paladin Framework
**Milestone:** 14 — Treasurer (deferred; does not run in the Milestone 13 cycle)
**Epic:** 1 — Treasurer role: allowances, pricing, cost ledger, pacing
**Version Target:** v0.12.0+ (later milestone)
**Status:** Reserved — the term is reserved by the Milestone 13 Epic 1 ADR; **operator-confirmed
2026-09-14 that this milestone does NOT run in the Milestone 13 cycle.** Schedule later.
**Breaking:** Additive on top of Milestone 13 Epic 2's accounting shape
**Created:** 2026-09-14
**Covers:** D-3 (build half), D-9, F7; FUT-08, FUT-09

> Read `../../Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` first. **Hard
> dependency: Milestone 13 Epic 2 (lossless accounting)** — this work is impossible on the pre-split
> accounting shape, so Milestone 13 must land first. This is a later, additive body of work.

---

## 1. Overview

The **Treasurer** is the output-side officer the two-officer model reserves: it keeps the campaign
purse, sets and enforces allowances across runs/tenants/keys, prices token usage in currency, keeps
the cost ledger, and paces against provider rate limits. It is **not** a rename of `TokenBudget` — it
*installs* a per-run `TokenBudget` the way `AgentRuntimeConfig::build_chain` does today, and reads the
full `TokenUsage` breakdown Epic 2 makes available. This is where `ExecutionMetadata.cost_estimate`
(a currently dead field, F7) finally gets a producer.

## 2. Goals

- Per-model **pricing** (prompt/completion/cache/reasoning) → currency cost.
- Produce `ExecutionMetadata.cost_estimate`; surface cost through heralds/CLI/traces.
- **Cross-run / per-tenant / per-API-key allowances** that can refuse a draw.
- **Rate pacing** (FUT-09) against provider rate limits.

## 3. Requirements

- **R1 (FUT-08).** A pricing table keyed by model with prompt/completion/cache/reasoning unit prices;
  a function mapping a `TokenUsage` (Epic 2 shape) to a currency cost.
- **R2 (D-9, F7).** Populate `ExecutionMetadata.cost_estimate` end-to-end from R1; update the rustdoc
  reserved-note (Milestone 13 Epic 1 R7) to "produced by Treasurer." If the build decision is to descope currency,
  remove the field instead — but default is populate.
- **R3.** A `Treasurer` role/service (Medieval-Military-named per the vocabulary rule) that owns
  cross-run policy: given a per-tenant/per-key **allowance** (new config key `allowance`, distinct
  from the four `max_tokens` meanings per D-8), it authorizes or refuses a run's draw and installs the
  per-run `TokenBudget` mechanism.
- **R4 (FUT-09).** Rate pacing: back off / pace requests against `LlmError::RateLimitExceeded` (429)
  signals so a campaign does not thrash a rate-limited provider.
- **R5.** A cost/spend **ledger** surfaced through heralds, CLI, and traces (uses Epic 2's breakdown +
  R1 pricing).
- **R6.** Docs: mdBook page for the Treasurer; MIGRATION entry if any public surface is added;
  configuration docs for `allowance` and pricing tables.

## 4. Out of scope

- Reworking the per-run limit middleware (`TokenBudget`/`ModelCallLimit`/`ToolCallLimit`) — the
  Treasurer composes them, does not replace them.
- Any Commissary/input-side work (Milestone 13, Epics 1–4).

## 5. Tests / verification

- Pricing→cost math per model, including cache/reasoning tokens.
- `cost_estimate` has a producer: assert non-null currency cost end-to-end for a run.
- Allowance enforcement across a simulated multi-run campaign: a per-key allowance refuses a draw
  once exhausted.
- Pacing under a mocked 429 (asserts back-off, not thrash).
- `make clean-code` green; coverage bars met.

## 6. Exit criteria

A run reports currency cost; a per-key allowance can refuse a draw; `cost_estimate` is populated (or
explicitly removed if descoped); rate pacing responds to 429s.

## 7. Dependencies / open questions

- **Depends on:** Milestone 13 Epic 2 (the split accounting shape). Hard prerequisite — schedule this
  milestone only after Milestone 13 has landed.
- **Open (operator, when this milestone is scheduled):** is per-tenant/per-API-key spend governance in
  scope, or is single-operator per-run cost reporting enough for now? (This decides whether R3/R4 are
  full or minimal.)
- **Guardrail:** keep `Treasurer` a framework-only word; never let it bleed into downstream
  audit-target/fixture vocabulary (Milestone 13 overview §5.3).
