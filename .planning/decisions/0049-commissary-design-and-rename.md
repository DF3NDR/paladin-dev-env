# ADR-0049: `Commissary` design, rename rationale, and rejected names

## Status

Accepted

**Date:** 2026-09-14

## Context

`crates/paladin-llm/src/services/commissary.rs` ships today as an unconditional facade export
(`src/lib.rs`) with no in-tree caller and no docs page. Its own module doc (`commissary.rs:1-39`)
states two deliberately separate responsibilities, applying the same fail-loud stance
`PaladinBuilder` already takes on `temperature_range` (ADR-0004: error rather than silently
clamp) to the INPUT side of a call — the assembled prompt — instead of the output-side
temperature:

- `Commissary::verify_fits` — a pre-flight GUARD. It measures an already-assembled prompt against
  the provider's declared context window (`ProviderCapabilities::max_context_tokens`) minus the
  caller's reserved completion budget, and returns
  `CommissaryError::ContextOverflow { measured_tokens, allotted_tokens, provider }` when it would
  overflow. It never trims.
- `Commissary::dispense` — a bounded ALLOCATOR over FIXED (non-sheddable) material plus a
  caller-prioritised `Consignment`. It returns a `Stockpile` whose retained items are clamped to a
  per-item byte share (with a visible truncation marker when a cut was needed) and whose shed
  items are each recorded in `Stockpile.shed` with label, priority and original size. Nothing is
  dropped silently — the explicit anti-pattern this module rejects is
  `RagRetrievalService::truncate_to_token_budget`, which drops lowest-scoring items with no marker
  and no record.

**Honesty clause.** `TokenCounterPort::count` is infallible and carries no built-in exactness
signal (unlike the removed, fallible `TokenCounter::is_exact`), so the caller supplies
`is_exact_counter` explicitly at `Commissary::new` construction time — it already knows which
concrete counter it injected. That flag threads straight into `Stockpile.exact_tally` so a caller
reading a `Commissary`-produced stockpile can tell an exact tally from an estimate.

**Refusal to invent a window.** A provider that declares no context window
(`max_context_tokens: None`) gets no invented window: construction fails with
`CommissaryError::UndeclaredContextWindow` unless the caller supplies
`CommissaryPlan::fallback_context_tokens` explicitly — that is caller policy, not something the
framework guesses.

This design is not new. It is a re-port of a capability first designed and accepted on an
abandoned branch as `Quartermaster` — see
`git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`
(ADR-0010 on that branch, accepted 2026-08-04, 168 lines). That record is cited here by its full
branch-qualified path as history; it is never copied wholesale into this in-tree series and is not
renumbered — the in-tree `.planning/decisions/0010-milestone-3-epic-numbering.md` is a different,
unrelated decision and stays byte-unchanged.

## Decision

**The capability is re-ported to v0.10.0 under new vocabulary, never as `Quartermaster`.** Per
`.planning/PROJECT.md`'s Ubiquitous language rule (D-01/D-02): units and measures and technical
port traits keep plain industry names, while domain roles, places and events get
Medieval-Military names. `Commissary` — the officer who issues rations under scarcity — is the
domain role the input-side rationing responsibility maps onto. The rename landed in two commits
that ported the abandoned branch's design and test suite upstream: `348f5910`
(`feat(paladin-llm): port v0.10.0-native prompt-budgeting Commissary service`) and `35fd8390`
(`test(paladin-llm): port Commissary unit tests + wire module + facade export`). Both commits' own
bodies record that the chosen names were verified-free — 0/0 grep across both this repo and the
downstream Web3 Security Paladin repo — before landing.

`Commissary` keeps its name; it is not renamed again. `TokenBudget`, `TokenCounterPort`,
`TokenUsage`, `max_tokens` and `token_budget.*` keep their plain industry names, unchanged.
`Quartermaster` stays retired and is never reintroduced. This decision is locked by the Milestone
13 overview §0 and is not revisited by a future phase.

## Considered Options

- **`Quartermaster`** (rejected — the retired name itself; carried on the abandoned branch, not reintroduced in this repo; avoided per the port commits' own 0/0-verified naming list).
- **`Convoy`** (rejected — the abandoned branch's caller-prioritised-material carrier type; avoided per the port commits' naming list, replaced by `Consignment`).
- **`apportion`** (rejected — the abandoned branch's allocator verb; avoided per the port commits' naming list, replaced by `dispense`).
- **`Provisioner`** (rejected — considered during the v0.10.0 re-port and avoided per the port commits' naming list).
- **`ProvisioningPlan`** (rejected — considered during the v0.10.0 re-port and avoided per the port commits' naming list, replaced by `CommissaryPlan`).
- **`Muster`/`muster`** (rejected — from the abandoned branch's own ADR-0010 "Vocabulary" section: collides with this framework's own troop-assembly vocabulary, the `paladin muster` CLI command and `Commands::Muster`; re-confirmed avoided by the port commits).
- **`provision()`** (rejected — from the abandoned branch's own ADR-0010 "Vocabulary" section: collides with `SandboxPort::provision`, a downstream container-lifecycle method).
- **`ContextRation`** (rejected — from the abandoned branch's own ADR-0010 "Vocabulary" section: collides with live downstream domain vocabulary).
- **`Allocation`** (rejected — from the abandoned branch's own ADR-0010 "Vocabulary" section: collides with the downstream Prover's on-chain entitlement invariant, `paid(x) <= allocation(x)`).

## Code Locations

- `crates/paladin-llm/src/services/commissary.rs` — the shipped service: `Commissary::new`, `Commissary::from_port`, `Commissary::verify_fits`, `Commissary::dispense`, `Commissary::allotted_tokens`; types `CommissaryPlan`, `Consignment`, `ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile`, `CommissaryError`.
- `src/lib.rs` — the unconditional facade re-export block making `Commissary` and its supporting types available at the `paladin` facade path.
- `crates/paladin-ports/src/output/token_counter_port.rs` — the infallible `TokenCounterPort` contract `Commissary` is built on.
- `git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md` — the historical record this ADR reconstructs from, cited by its full branch-qualified path; never checked out, never treated as a live in-tree ADR.

## Code Conformance

conforms

The tree already ships exactly the design this ADR records: `verify_fits` and `dispense` behave
as described, the fail-loud / never-silent stance holds, and `is_exact_counter` already threads
into `Stockpile.exact_tally`. This ADR instructs no code change — it records the design, the
rename rationale and the rejected-name list so a future reader does not have to reconstruct them
from an abandoned branch.

## Downstream Consumers

- **Phase 33** (in-tree adoption, COMM-01..03) — the first in-tree caller of `Commissary::dispense`, replacing `RagRetrievalService::truncate_to_token_budget`'s silent-drop behaviour.
- **`docs/src/architecture/commissary.md`** (this phase, plan 30-01 Task 2) — the mdBook page that gives the shipped service a documentation home reachable from the architecture nav.
- **The downstream Web3 Security Paladin app** — the repo whose own vocabulary this ADR's rejected-name list and the port commits' 0/0 grep were checked against.
