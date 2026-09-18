# ADR-0050: `Treasurer` reserved for cross-run spend governance

## Status

Accepted

**Date:** 2026-09-14

## Context

Milestone 13 (Token Economy) splits token-economy work across two officers. `Commissary`
(ADR-0049) is the **input-side, per-call window-rationing officer** — it measures an
already-assembled prompt against a provider's declared context window and allocates a
caller-prioritised `Consignment` into a `Stockpile`, fail-loud / never-silent. The
**output-side, cross-run spend-governance role** is a separate responsibility — currency
pricing, allowances, pacing — that Milestone 13 explicitly does not build (it was originally
scoped as the fifth epic of Milestone 13 and was split out into its own milestone by operator decision on
2026-09-14, per `.project/Milestone_14-Treasurer/overview/Milestone-14_Treasurer.md` §0).
Deferring the build is cheap; deferring the **name** is not — Phases 31-33 and the downstream
Web3 Security Paladin app would otherwise be free to invent competing vocabulary for this role
before it exists, and a later rename would ripple through both. This ADR reserves the name now,
before any code is written against it.

**Authoring-time verification.** `grep -rn Treasurer crates src`, run at authoring time
(2026-09-14, commit `2c581d95065820479689cc8c6261b9f6a88ff499`), returns **no matches** (exit 1)
— the term is 0/0 in-tree as both a code symbol and a bare word.

**The successor state, stated up front so this record does not age into a lie.** The reservation
is **0/0 as a code symbol, permanently** — that is the durable invariant this ADR makes, not the
weaker "0/0 everywhere" claim. Later in this same phase, plan 30-03 (VOCAB-05) writes the
reserved role's name into rustdoc prose on `ExecutionMetadata.cost_estimate` in
`crates/paladin-core/src/platform/container/herald.rs`, naming it as that field's future
producer. After Phase 30 closes, a bare `grep -rn Treasurer crates src` therefore returns
rustdoc lines only (`///` or `//!`), never a `struct`, `enum`, `trait`, `mod`, `fn`, `impl`,
`use` or `type` declaration of the name, and never a path-qualified use (`Treasurer::…`). A
reader who re-runs the bare grep after Phase 30 and finds a rustdoc-only hit should read this
paragraph, not conclude the reservation was broken.

## Decision

**What the role will own** (built in Milestone 14, not this cycle): cross-run / per-tenant /
per-API-key **allowances** that can refuse a draw; per-model currency **pricing**; production of
`ExecutionMetadata.cost_estimate` (`crates/paladin-core/src/platform/container/herald.rs`),
which has no in-tree producer today; and rate **pacing**.

**What it does NOT do.** The Treasurer **installs** a per-run `TokenBudget` — the existing
middleware at `src/application/services/paladin/middleware/limits.rs`, configured via
`agent_runtime.token_budget.*` — rather than replacing it. `TokenBudget` already caps a single
run's accumulated `total_tokens` and finishes that run gracefully with
`StopReason::TokenBudget` when crossed; the Treasurer's cross-run allowances compose with that
existing per-run mechanism, they do not supersede or remove it.

**When.** Built in **Milestone 14** (`.project/Milestone_14-Treasurer/`), which hard-depends on
Phase 31 (Milestone 13 Epic 2's lossless `TokenUsage` prompt/completion/cache/reasoning split —
the Treasurer needs that full split and is impossible on the pre-split shape). Not this cycle,
not this milestone. Rustdoc and prose elsewhere in the repo point at **Milestone 14**, never at
"the fifth epic" — that PRD wording predates the Milestone 14 split confirmed 2026-09-14.

**The downstream guardrail.** `Treasurer` is a **framework-only word**. In the downstream Web3
Security Paladin app it collides with a benchmark fixture, `GarrisonTreasury`, which is an
audit-*target* domain term (a thing the security scanner examines), not a framework role. The
framework term must never be used as an audit-target or fixture domain term downstream, and the
downstream fixture vocabulary must never be repurposed for this framework role. This is a
cross-repo convention documented here, not a lint or CI rule enforced in this repo.

**Version identity.** This record and every artifact it cites uses **v0.10.0** throughout — the
current, untagged development version, and no other.

**The two-officer model** — `Commissary` (input-side, per-call rationing) and `Treasurer`
(output-side, cross-run spend governance) — is **operator-confirmed 2026-09-14** and locked; it
is not revisited by a future phase.

## Considered Options

- **`Paymaster`** (rejected — a paymaster pays the troops, whereas tokens in this framework are spent with the LLM provider, not disbursed to a party under the framework's own command; the metaphor points the wrong direction for an officer who tracks and limits outbound spend).
- **`Comptroller`** (a collision-free alternative — no known conflict with either this repo's or the downstream Web3 Security Paladin repo's vocabulary; viable, but not chosen).
- **`Treasurer`** (chosen, with the downstream `GarrisonTreasury` guardrail accepted as the cost of the more evocative Medieval-Military fit — see the downstream guardrail above).
- **Reserve nothing; name the role only when it is built** (rejected — the downstream Web3 Security Paladin app is already writing spend-adjacent vocabulary of its own, and Phases 31-33 build the prerequisites this role depends on; reserving the name now prevents a competing term from taking root before Milestone 14 exists).

## Code Locations

- `src/application/services/paladin/middleware/limits.rs` — the existing per-run `TokenBudget` middleware, the install point the Treasurer will attach to rather than replace; the Treasurer itself appears in this file **nowhere** today.
- `src/config/agent_runtime.rs` — `TokenBudgetConfig` and the `token_budget.*` config surface `TokenBudget` is constructed from; the Treasurer itself appears in this file **nowhere** today.
- `crates/paladin-core/src/platform/container/herald.rs` — `ExecutionMetadata.cost_estimate`, the field this role will one day produce; the Treasurer itself appears in this file **nowhere** today — plan 30-03 (VOCAB-05, later in this same phase) adds the first rustdoc-prose mention.
- `.project/Milestone_14-Treasurer/` — the owning milestone (`overview/Milestone-14_Treasurer.md`, `Epic_1/prd-treasurer-spend-governance.md`), reserved / deferred, hard-depends on Phase 31; the Treasurer itself appears in this location **nowhere** as a code symbol — it is planning prose only.

## Code Conformance

conforms

This ADR reserves a word in writing and institutes no code change. No module, type, feature
flag, config key or file bearing the reserved name exists anywhere under `crates/` or `src/`
after this ADR lands — the 0/0 symbol-scoped grep is the exit condition this record establishes,
not a target for a future scaffold.

## Downstream Consumers

- **Milestone 14** (`.project/Milestone_14-Treasurer/`) — the owning milestone that builds against this reservation once its hard prerequisite (Phase 31) has landed.
- **Phase 31** — Milestone 13 Epic 2's lossless token-accounting split, the hard prerequisite this ADR names as the trigger that must land before Milestone 14 can be scheduled.
- **The downstream Web3 Security Paladin app** — the repo whose own `GarrisonTreasury` fixture vocabulary this ADR's guardrail keeps separate from the framework's reserved role name.
