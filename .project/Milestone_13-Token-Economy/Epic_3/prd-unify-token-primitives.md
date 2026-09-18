# PRD: Unify Token Primitives (Milestone 13, Epic 3)

**Project:** Paladin Framework
**Milestone:** 13 — Token Economy
**Epic:** 3 — One counting contract, one window resolver
**Version Target:** v0.11.0
**Status:** Ready for Planning
**Breaking:** Yes — clean break, no shims (pre-1.0, single coordinated consumer). MIGRATION §9.2 entries.
**Created:** 2026-09-14
**Covers:** D-5, D-6, F2, F3

> Read `../overview/Milestone-13_Token-Economy.md` first (esp. §5: clean-break policy). Depends on
> Epic 2. **The `Commissary::new` change is a clean break — no deprecation shim.** The downstream app
> (the only consumer, ~150+ call sites) adopts the new signature in its coordinated Milestone 13
> refactor; it stays on the prior Paladin commit until then, so it is never broken on a new pointer.

---

## 1. Overview

Two duplications make the token primitives confusing. **(F3)** Two counting contracts coexist: the
new infallible `TokenCounterPort` (0.10, D-13) and the legacy fallible `garrison::TokenCounter` +
`TokenCounterFactory`, still re-exported from three places. **(F2)** Two window-resolution algorithms
answer "what is this model's context window?" with different precedence: `HistoryTrimmer.resolve_limit`
(config table → provider capabilities → default) and `Commissary::new` (provider capabilities →
caller fallback, refusing to invent a window). This epic collapses each duplication to one.

## 2. Goals

- One token-counting contract path, with an exactness signal restored on the port.
- One shared window resolver consumed by both `HistoryTrimmer` and `Commissary`, preserving
  Commissary's strict "no invented window" mode.

## 3. Requirements

- **R1 (D-5).** Add `fn is_exact(&self) -> bool { false }` (default) to `TokenCounterPort`. The
  tiktoken adapter returns `true`; heuristic returns `false`.
- **R2 (D-5).** Drop `Commissary::new`'s `is_exact_counter: bool` argument (it exists only because the
  port lacked `is_exact()`); read exactness from the port instead. **Clean break — remove the old
  argument outright, no forwarding shim.** Update every in-tree call site; the downstream app adopts
  the new signature in its coordinated refactor.
- **R3 (D-5).** Retire legacy `garrison::TokenCounter` and `TokenCounterFactory` in favour of the
  port. **Prefer removing them outright this epic** (clean break) once all in-tree callers are
  migrated to `TokenCounterPort` and the three re-exports are dropped. If an internal caller cannot be
  migrated this epic, mark it `#[deprecated]` and remove next epic — but the default is direct
  removal, not a deprecation window.
- **R4 (D-6).** Extract `HistoryTrimmer.resolve_limit`'s precedence (config table → provider
  capabilities → default) into a shared resolver in `paladin-llm` (e.g.
  `window::resolve_context_window(...)`). Both `HistoryTrimmer` and `Commissary` consume it.
- **R5 (D-6).** Preserve Commissary's ADR-0010 refusal semantics ("no invented window") as an explicit
  `strict` mode of the shared resolver, so Commissary still errors rather than defaulting when the
  window is unknown.
- **R6.** MIGRATION §9.2 entries documenting the breaks (the `Commissary::new` signature change and
  the legacy-counter removal) as migration guidance for the downstream refactor.

## 4. Out of scope

- Changing Commissary's behaviour/output (windows resolved must be identical; see R2 test in §5).

## 5. Tests / verification

- Resolver precedence tests: config-table hit vs provider-capability hit vs default fallback; strict
  mode refuses (errors) when the window is unknown.
- Equivalence snapshot: `Commissary` resolves the same window via the shared resolver as before this
  epic (no behavioural drift).
- All in-tree call sites compile against the new `Commissary::new` signature and the port-only counter
  (the break is fully absorbed inside Paladin; no dangling references to the removed API).
- `make clean-code` green.

## 6. Exit criteria

Exactly one live counting-contract path (port + `is_exact()`); the legacy trait/factory removed (or,
if a caller blocked it, deprecated for next-epic removal); both `HistoryTrimmer` and `Commissary`
resolve the window through one function; `Commissary::new` has the new signature with no forwarding
shim; MIGRATION documents the breaks.

## 7. Dependencies / downstream impact

- **Depends on:** Epic 2 (the primitives layer settles together).
- **Downstream (Web3sec):** the ~150+ Commissary call sites are updated to the new signature as part
  of web3sec's coordinated Milestone 13 refactor (after this release lands), not bridged by a shim.
  Web3sec stays on the prior Paladin commit until it adopts the whole milestone. The MIGRATION entry
  (R6) is the migration guide for that refactor.
