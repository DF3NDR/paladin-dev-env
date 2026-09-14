# Milestone 14: Treasurer — cross-run spend governance + cost ledger

**Project:** Paladin Framework
**Milestone:** 14 — Treasurer (deferred)
**Version Target:** v0.12.0+
**Status:** Reserved / deferred — **does not run in the Milestone 13 cycle** (operator-confirmed 2026-09-14)
**Created:** 2026-09-14
**Author:** Reconciliation planning pass (handoff)

---

## 0. Why this is its own milestone

This was originally scoped as Epic 5 of Milestone 13 (Token Economy). The operator confirmed on
2026-09-14 that the Treasurer **will not run in the Milestone 13 cycle**, so it is promoted to its own
milestone and deferred. Milestone 13 ships Epics 1–4 (Commissary anchoring, lossless accounting,
primitive unification, Commissary adoption). The `Treasurer` **term is already reserved** by Milestone
13's Epic 1 ADR, so nothing is lost by deferring the build.

## 1. Scope (one epic)

- **Epic 1 — Treasurer role:** per-model currency pricing, `cost_estimate` production, cross-run /
  per-tenant / per-API-key allowances that can refuse a draw, and rate pacing. Full spec:
  `Epic_1/prd-treasurer-spend-governance.md`.

The Treasurer is the output-side officer in the two-officer model (Commissary rations the window
per call; Treasurer governs spend across runs). It **installs** the per-run `TokenBudget` mechanism
rather than replacing it.

## 2. Hard prerequisite

**Milestone 13 Epic 2 (lossless token accounting) must land first.** The Treasurer needs the full
`TokenUsage` prompt/completion/cache/reasoning split; it is impossible on the pre-split shape. Do not
schedule this milestone until Milestone 13 has shipped.

## 3. Guardrail (carried from Milestone 13)

`Treasurer` stays a framework-only word. In the downstream Web3 Security Paladin app it collides with
a benchmark fixture (`GarrisonTreasury`, an audit-target domain term); the two must never mix. See
Milestone 13 overview §5.

## 4. Open question (operator, when scheduled)

Is per-tenant / per-API-key spend governance in scope, or is single-operator per-run cost reporting
enough for a first cut? This decides whether the allowance/pacing requirements are full or minimal.
