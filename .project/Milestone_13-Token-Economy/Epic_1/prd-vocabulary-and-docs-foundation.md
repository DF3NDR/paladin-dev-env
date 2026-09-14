# PRD: Vocabulary & Docs Foundation (Milestone 13, Epic 1)

**Project:** Paladin Framework
**Milestone:** 13 — Token Economy
**Epic:** 1 — Vocabulary & docs foundation
**Version Target:** v0.11.0
**Status:** Ready for Planning
**Created:** 2026-09-14
**Covers:** D-1, D-2, D-3, D-8, D-9 (docs), F4 (partial), F5 (docs)

> Read `../overview/Milestone-13_Token-Economy.md` first for locked decisions and current-state
> anchors. This epic is **docs-only, non-breaking**, and can land alone and first.

---

## 1. Overview

Paladin's token-economy vocabulary is undecided in writing, and the shipped `Commissary` service has
no documentation home. This epic freezes the naming model so every later epic (and the downstream
app) builds on one vocabulary, and it anchors `Commissary` where readers and future agents will find
it. It also reserves the `Treasurer` term with a one-page ADR, documents the four meanings of
`max_tokens`, and removes the last two orphan `Quartermaster` prose references.

No code behaviour changes. Output is documentation, one ADR, mdBook pages, and comment edits.

## 2. Goals

- Record the vocabulary rule: **units and technical ports stay plain; roles get Medieval-Military
  names** (D-1).
- Anchor `Commissary` (D-2): ubiquitous-language list, domain-model table, an on-branch ADR (port
  the design record + the rejected-name list from the abandoned branch), and an mdBook page.
- Reserve `Treasurer` (D-3): a one-page ADR stating exactly what the role will own and the downstream
  collision guardrail.
- Disambiguate `max_tokens` in docs (D-8) and annotate `cost_estimate` as reserved (D-9).
- Delete/annotate the two orphan `Quartermaster` references.

## 3. Requirements

- **R1 (D-1).** Add a ubiquitous-language rule to `PROJECT.md` and
  `docs/src/architecture/domain-model.md`: units/measures (`TokenUsage`, `max_tokens`,
  `max_context_tokens`) and technical ports (`TokenCounterPort`, `LlmPort`, `EmbeddingPort`) keep
  plain names; domain roles/places/events get Medieval-Military names.
- **R2 (D-2).** Add `Commissary` to the ubiquitous-language list and the domain-model table, framed
  as the **input-side per-call window-rationing officer** (issues rations under scarcity per sortie).
- **R3 (D-2).** Create an on-branch ADR for `Commissary` (e.g. `.planning/decisions/NNNN-commissary-*.md`
  or the repo's ADR location): its design (`verify_fits` guard + `dispense` allocator, fail-loud /
  never-silent), the Quartermaster→Commissary rename rationale, and the explicit rejected-name list.
  Reconstruct from the abandoned-branch ADR-0010 and the port commit history.
- **R4 (D-2).** Write an mdBook page for `Commissary` under `docs/src/` (concept + the
  `Consignment`/`Stockpile`/`ShedItem` model + a usage sketch), linked from the architecture nav.
- **R5 (D-3).** Write a one-page `Treasurer` **reservation** ADR: the role is reserved (0/0 verified),
  will own cross-run/per-tenant/per-API-key allowances, per-model currency pricing, `cost_estimate`
  production, and rate pacing; it will *install* a `TokenBudget` per run rather than replace it. State
  the downstream guardrail: `Treasurer` is a framework-only word and must never appear as an
  audit-target/fixture domain term.
- **R6 (D-8, F5).** Add one table to `docs/src/getting-started/configuration.md` documenting the four
  `max_tokens` meanings (Garrison store cap, RAG injection cap, per-request completion cap, run-level
  `token_budget` cap) and stating that any future Treasurer-level cap uses a distinct key
  (`allowance`), not `max_tokens`.
- **R7 (D-9, F7).** Update the rustdoc on `ExecutionMetadata.cost_estimate` to "reserved for
  Treasurer (Epic 5 / FUT-08); no in-tree producer yet." Do not remove the field.
- **R8.** Delete or annotate the two orphan `Quartermaster` references: the `src/lib.rs:195`
  provenance comment (reword to reference `Commissary` without the retired term, or drop) and the
  `.project/project-management/paladin-project-plan-final.md` `SirQuartermaster` example (annotate as
  historical). After this, `grep -rn Quartermaster crates src` returns nothing.

## 4. Out of scope

- Any code-behaviour change. Any rename of shipped types. Building the Treasurer (Epic 5).

## 5. Tests / verification

- Docs build clean; mdBook link-check passes; the new Commissary page is reachable from nav.
- `grep -rniE '\bQuartermaster\b' crates src` (excluding `.planning/` history) returns zero.
- ADRs render and are indexed; semver-checks unaffected (no API change).

## 6. Exit criteria

`Commissary` is in the ubiquitous-language list and domain model with an ADR and an mdBook page; a
Treasurer reservation ADR exists with the term (`Treasurer`) and guardrail locked; the four
`max_tokens` meanings and the `cost_estimate` reservation are documented; zero Quartermaster refs
remain in source.

## 7. Dependencies

None. Lands first and alone.
