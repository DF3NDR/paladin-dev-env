# Milestone 13: Token Economy — Commissary anchoring, lossless accounting, Treasurer

**Project:** Paladin Framework
**Milestone:** 13 — Token Economy (budgeting · accounting · governance vocabulary)
**Version Target:** v0.11.0 (Epics 1–4). Treasurer is deferred to **Milestone 14** — it does **not** run this cycle (operator-confirmed 2026-09-14).
**Status:** Planning — ready for a standalone GSD session rooted at this repo
**Created:** 2026-09-14
**Author:** Reconciliation planning pass (handoff from the downstream Web3 Security Paladin repo)

---

## 0. Read this first — handoff context

This milestone is **self-contained**: everything a standalone agent needs is in this directory
(`overview/` + `Epic_1..5/prd-*.md`). It was authored from the downstream **Web3 Security Paladin**
repo (the only current consumer of this framework) and then placed here so it travels with the
Paladin repo. Do **not** assume access to the downstream repo's `.project/` reports.

### Provenance of the analysis

Three 2026-09-13 analyses in the downstream repo drove this milestone. Their load-bearing content
is inlined below and in the epics, so they need not be fetched:

- A Paladin-side naming report (framed the task as "rename `Commissary`").
- A Web3sec-side cleanup report (Quartermaster orphan buckets; `Treasurer`↔fixture collision).
- A deep Paladin-only token-economy systems analysis (five token concerns; findings **F1–F8**;
  decisions **D-1…D-9**). This last one is the authority; where the naming report and it disagreed,
  **this milestone follows the systems analysis.** Its finding/decision IDs are referenced
  throughout and summarized in §4.

### The one settled disagreement (locked decision)

The naming report proposed renaming the live `Commissary` service to a financial term. The systems
analysis showed that is wrong: **`Commissary` and `Treasurer` are two different officers, not two
names for one role.** The operator has **confirmed the two-officer model** and **confirmed the term
`Treasurer`**. Therefore:

| Term | Locked decision |
|---|---|
| **Commissary** | **Keep, do not rename.** Input-side, per-call *window rationing* ("what fits in this sortie's pack"): `verify_fits`, `dispense`, `Consignment`, `Stockpile`, `ShedItem`. Anchor it (docs/ADR/mdBook) and give it a real in-tree caller. |
| **Treasurer** | **Reserve now (Epic 1 ADR), build in Milestone 14 (not this cycle).** New output-side role: cross-run spend governance + cost ledger — per-tenant/per-key allowances, currency pricing, `cost_estimate` production, rate pacing. **Not** a rename of `TokenBudget`. |
| **Quartermaster** | **Stays retired.** Delete the two orphan prose refs (§3). No functional symbol exists. |
| `TokenBudget`, `TokenCounterPort`, `TokenUsage`, `max_tokens`, `token_budget.*` | **Do not rename.** Units + technical ports stay plain; roles get medieval names (the vocabulary rule, Epic 1 / D-1). Shipped in v0.10.0; renaming for vocabulary alone would be a needless breaking change. |

`Paymaster` was rejected: a paymaster pays *the troops*, but tokens are spent *on the provider*, not
paid to Paladins. `Comptroller` was the collision-free alternative; the operator chose `Treasurer`
and accepts the downstream guardrail (§5).

---

## 1. Executive summary

Paladin's "token economy" is five concerns spread across seven crates + the facade. Only one of them
(`Commissary`) uses Medieval-Military vocabulary; the rest use plain industry terms. The subsystem
works but is unfinished in three ways this milestone fixes:

1. **`Commissary` is unanchored** — a tested, facade-exported primitive with **no in-tree caller**,
   no mdBook page, absent from the ubiquitous-language list, and its design ADR lives only on an
   abandoned branch (F4). (Epics 1, 4.)
2. **Accounting is lossy above the LLM port** — `TokenUsage`'s prompt/completion split is collapsed
   to a bare total one layer up, so nothing cost-shaped (currency cost, per-model pricing, the
   Treasurer) can be built (F1). (Epic 2 — the keystone.)
3. **Duplicated primitives** — two token-counting contracts and two window-resolution algorithms
   answer the same questions differently (F2, F3). (Epic 3.)

On that fixed base, this milestone kills the last silent-truncation path (F6, Epic 4) and reserves
then optionally builds the **Treasurer** cross-run governance role (Epic 5).

### Success criteria (milestone)

- The vocabulary rule (units/ports plain, roles medieval) is written into `PROJECT.md` and the
  domain model; `Commissary` appears in both; a Treasurer reservation ADR exists.
- Zero `Quartermaster` references remain in Paladin source.
- A prompt/completion/cache/reasoning **breakdown** survives from the LLM port to `RunFinished` and
  a herald; the battalion path no longer zeroes the split.
- Exactly one live token-counting contract path and one window resolver.
- `Commissary` has a production in-tree caller exercised by integration tests; no silent
  token-based truncation remains in-tree.
- (Milestone 14, deferred) a run reports currency cost and a per-key allowance can refuse a draw.

---

## 2. Epics (execute in dependency order)

| Epic | Title | Version | Breaking | Depends on |
|---|---|---|---|---|
| **1** | Vocabulary & docs foundation | v0.11.0 | No (docs only) | — |
| **2** | Lossless token accounting (full `TokenUsage`) | v0.11.0 | **Yes** (MIGRATION §9.2) | — (keystone) |
| **3** | Unify token primitives (one counter, one resolver) | v0.11.0 | Deprecations + shimmed ctor | Epic 2 |
| **4** | Commissary in-tree adoption (RAG + resolver) | v0.11.0 | No (behavioural) | Epic 3 |

**Milestone 14 (deferred, not this cycle):** Treasurer — cross-run spend governance + cost ledger.
Reserved by this milestone's Epic 1 ADR; specified in `../../Milestone_14-Treasurer/`. Depends on
Epic 2. Does not run this cycle.

```
Epic 1 (docs) ─────────────────┐
                               ├─▶ Epic 4 (Commissary adoption)
Epic 2 (accounting) ─▶ Epic 3 ─┘
        │
        └──────────────────────────▶ [Milestone 14: Treasurer]
```

Epic 1 can land alone and first. Epic 2 is the keystone — Epic 3 (this cycle) and Milestone 14 (later)
both need it. This milestone ships Epics 1–4 only.

---

## 3. Verified current-state anchors (grounded 2026-09-14 in this repo)

The implementing agent should re-confirm exact line numbers before editing (files evolve), but these
were verified present at authoring time:

| Anchor | Location | Note |
|---|---|---|
| `Commissary` service | `crates/paladin-llm/src/services/commissary.rs` (~39 KB) | `verify_fits`, `dispense`; exports `Commissary`, `CommissaryError`, `CommissaryPlan`, `Consignment`, `ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile` from `src/lib.rs`. |
| `TokenCounterPort` | `crates/paladin-ports/src/output/token_counter_port.rs` | `count(&self, text, model) -> u32` + `name()`. Infallible, no exactness signal (D-13). New in 0.10. |
| Legacy `TokenCounter` + `TokenCounterFactory` | `crates/paladin-memory/src/garrison/token_counter.rs` | Fallible; still re-exported 3×. Not deprecated. (Epic 3 / F3.) |
| `TokenBudget`/`ModelCallLimit`/`ToolCallLimit` | `src/application/services/paladin/middleware/limits.rs` | Per-run caps; `StopReason::TokenBudget`. Keep. |
| `HistoryTrimmer` | `src/application/services/paladin/middleware/history.rs` | Second window resolver (config table → provider → default). (Epic 3 / F2.) |
| RAG silent truncation | `crates/paladin-memory/src/services/rag_retrieval_service.rs` (`truncate_to_token_budget`) | `content.len()/4`, silent drop. (Epic 4 / F6, D-13.) |
| `cost_estimate` (no producer) | `crates/paladin-core/src/platform/container/herald.rs` (`ExecutionMetadata.cost_estimate: Option<f64>`) | Field + builder + heralds read it; **nothing writes it.** (Epic 5 / F7.) |
| Orphan Quartermaster ref #1 | `src/lib.rs:195` | Provenance comment: "…re-port of the removed Quartermaster/Convoy/apportion capability." Delete/annotate in Epic 1. |
| Orphan Quartermaster ref #2 | `.project/project-management/paladin-project-plan-final.md` | `name: "SirQuartermaster"` example. Annotate in Epic 1. |
| No functional Quartermaster symbol | (verified) | No `struct/enum/impl/mod/use Quartermaster` or `Quartermaster::` anywhere. |

---

## 4. Findings/decisions carried from the systems analysis (inlined)

**Findings (why the work exists):**
- **F1** Lossy accounting: `PaladinResult` carries a bare total; `BattalionResult.per_paladin_tokens`
  is built with `TokenUsage::from_total`, zeroing prompt/completion. → Epic 2.
- **F2** Divergent window resolution: `HistoryTrimmer` vs `Commissary::new` use different precedence.
  → Epic 3.
- **F3** Two counting contracts coexist (legacy fallible trait + new infallible port). → Epic 3.
- **F4** `Commissary` unanchored (no caller, no docs, ADR only on abandoned branch). → Epics 1, 4.
- **F5** `max_tokens` means four things (Garrison store cap, RAG injection cap, per-request
  completion cap, run-level budget). → Epic 1 docs (D-8).
- **F6** RAG silent truncation still ships. → Epic 4.
- **F7** `cost_estimate` is a dead field. → Milestone 14 (D-9).
- **F8** Streaming may under-report usage; verify per adapter. → Epic 2.

**Decisions (what to do):** D-1 vocabulary split (Epic 1) · D-2 anchor Commissary (Epic 1) · D-3
reserve Treasurer (Epic 1 ADR; build in Milestone 14) · D-4 fix lossy accounting (Epic 2) · D-5 one
counter contract (Epic 3) · D-6 one window resolver (Epic 3) · D-7 first Commissary consumer via RAG
(Epic 4) · D-8 `max_tokens` disambiguation docs (Epic 1) · D-9 `cost_estimate` (Milestone 14).

---

## 5. Cross-repo compatibility constraints (do not lose these)

The downstream **Web3 Security Paladin** app is the **only current consumer** of this framework, and
it consumes `Commissary` heavily (~150+ references in one agent alone). Therefore:

1. **Clean break is preferred over shims (pre-1.0, single coordinated consumer).** The downstream app
   is the only consumer, it will be deliberately refactored to adopt these changes, and it controls
   its own submodule pointer (staying on the prior Paladin commit until it adopts the whole milestone
   at once). So breaking changes land **outright — no deprecation shims.** Epic 3 drops
   `Commissary::new`'s `is_exact_counter` argument directly, and Epic 2 changes the token carriers
   directly; web3sec adopts the new signatures as part of its coordinated refactor. Rationale:
   carrying deprecation shims pre-1.0 ships cruft while the API is still being shaped — keep the
   surface clean now and switch to deprecation windows only once Paladin stabilizes (1.0) or gains
   independent external consumers. Every break still gets a MIGRATION §9.2 entry **as documentation
   for the refactor** (not as a compatibility shim).
2. **Coordinate the pointer bump with the refactor.** Land Epics 1–4 and cut a v0.11.0 release; the
   downstream repo bumps its submodule pointer and adopts the new APIs/term in **one coordinated
   step**, staying on the prior Paladin commit until then so it is never left broken on a new pointer.
   Tradeoff accepted: pulling any new Paladin commit means adopting all of the milestone's breaking
   changes at once, not cherry-picking — fine for a single operator doing a lockstep refactor.
3. **`Treasurer` guardrail.** `Treasurer` is 0/0 in this repo but **collides downstream** with a
   benchmark fixture (`GarrisonTreasury`, an audit-*target* domain term). The framework's
   `Treasurer` must stay strictly a framework word; the reservation ADR (Epic 1) records this
   guardrail so downstream never mixes the two.
4. **Do not rewrite `.planning/` history.** Phase docs recording the earlier
   Quartermaster→Commissary port stay as written; only forward-looking docs adopt the vocabulary.

---

## 6. How to run this milestone (GSD in this repo)

- This repo has its **own GSD install (v1.8.0)** and its **own git-tracked `.planning/`**. Run all
  GSD commands from a session **rooted at this repo** (its `.claude/settings.json` already scopes
  `cargo`/`make` permissions and sources the build env).
- These PRDs are the df3ndr-pipeline inputs. Turn each epic into GSD execution via the repo's normal
  flow (e.g. ingest/plan the epic, then `/gsd-plan-phase` → `/gsd-execute-phase`), or feed each
  `prd-*.md` to `/create-prd`→`/generate-tasks`→`/process-task-list` per the repo's convention.
- **Engineering discipline (inherited, enforce every epic):** `make clean-code` (fmt + clippy +
  check) before commits; **no `unwrap`/`expect`/`panic!` in library code**; unit ≥80% / integration
  ≥70%; every breaking change gets a MIGRATION §9.2 entry and a semver-checks allowlist update.
- Commits land in **this** (`DF3NDR/paladin-dev-env`) repo; the downstream repo only bumps the
  submodule pointer after a release.

---

## 7. Out of scope

- Any rename of `Commissary`, `TokenBudget`, `TokenCounterPort`, or `TokenUsage`.
- Any change to the downstream Web3sec app (that is a separate, parallel track in its own repo).
- Reintroducing `Quartermaster` in any form.
