# PRD: Commissary In-Tree Adoption (Milestone 13, Epic 4)

**Project:** Paladin Framework
**Milestone:** 13 — Token Economy
**Epic:** 4 — Commissary's first in-tree consumer; kill silent truncation
**Version Target:** v0.11.0
**Status:** Ready for Planning
**Breaking:** No (behavioural change in RAG output — CHANGELOG note)
**Created:** 2026-09-14
**Covers:** D-7, F6, F4 (completes)

> Read `../overview/Milestone-13_Token-Economy.md` first. Depends on Epic 3 (uses the shared window
> resolver).

---

## 1. Overview

`Commissary` is a tested, facade-exported primitive with **no in-tree caller** (F4), and RAG still
performs **silent** token truncation — `RagRetrievalService::truncate_to_token_budget` drops the
lowest-scoring memories with no marker and no record (F6, the exact anti-pattern ADR-0010 names, and
the deferral D-13 parked). This epic makes RAG the first production consumer of `Commissary::dispense`,
so shed memories are recorded and truncated content is marked — proving the primitive and closing the
anti-pattern in one move. It also wires `HistoryTrimmer` onto the shared resolver from Epic 3.

## 2. Goals

- Route RAG truncation through `Commissary::dispense` with score-derived priorities.
- Record every shed memory and mark truncated output (no silent drops).
- Give `Commissary` a real, integration-tested in-tree caller.

## 3. Requirements

- **R1 (D-7).** Replace `RagRetrievalService::truncate_to_token_budget`'s inline `len/4` + silent drop
  with a `Commissary::dispense` call: build a `Consignment` from the retrieved memories, priority =
  derived from relevance score, budget = `rag.max_tokens`. Retain the highest-scoring memories that
  fit; return the `Stockpile`.
- **R2 (D-7).** Surface the `ShedItem` list (which memories were dropped and why) through the RAG
  result path so callers/observability can see it, and emit a truncation **marker** when content was
  shed (the ADR-0010-compliant behaviour).
- **R3.** Wire `HistoryTrimmer` to consume the shared window resolver introduced in Epic 3 (no
  behavioural change to trimming; single source of window truth).
- **R4.** CHANGELOG note: RAG output now includes a truncation marker / shed record where it
  previously dropped silently.

## 4. Out of scope

- Changing RAG retrieval/scoring itself.
- Any Treasurer/cost work (Epic 5).

## 5. Tests / verification

- RAG truncation now emits a marker AND a shed record — assert both are present when the budget is
  exceeded, and absent when everything fits.
- Property test: retained set total ≤ `rag.max_tokens`, and the highest-scoring memories are the ones
  retained.
- Integration test exercising `Commissary::dispense` via the real RAG path (this is the F4
  production-caller evidence).
- `HistoryTrimmer` uses the shared resolver (regression: same trims as before).
- `make clean-code` green; coverage bars met.

## 6. Exit criteria

No silent token-based truncation remains in-tree; `Commissary` has a production caller exercised by
integration tests; `HistoryTrimmer` and `Commissary` share one window resolver.

## 7. Dependencies

- **Depends on:** Epic 3 (shared resolver, settled counter contract).
