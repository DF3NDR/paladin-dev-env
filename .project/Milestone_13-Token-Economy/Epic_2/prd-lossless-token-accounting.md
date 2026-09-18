# PRD: Lossless Token Accounting (Milestone 13, Epic 2)

**Project:** Paladin Framework
**Milestone:** 13 — Token Economy
**Epic:** 2 — Lossless token accounting (full `TokenUsage` end-to-end)
**Version Target:** v0.11.0
**Status:** Ready for Planning
**Breaking:** **Yes** — MIGRATION §9.2 entry + semver-checks allowlist update required
**Created:** 2026-09-14
**Covers:** D-4, F1, F8

> Read `../overview/Milestone-13_Token-Economy.md` first. **This is the keystone epic**: Epics 3 and
> 5 depend on it. Nothing cost-shaped (currency cost, per-model pricing, the Treasurer) is buildable
> until the prompt/completion split survives above the LLM port.

---

## 1. Overview

Today the provider's `TokenUsage { prompt, completion, total }` is collapsed to a single total one
layer above the LLM port: `PaladinResult` carries a bare count and `BattalionResult.per_paladin_tokens`
is built with `TokenUsage::from_total`, so `prompt_tokens`/`completion_tokens` are always zero there.
Every aggregate upstream (engine, trace, web, CLI, eval) inherits the loss (F1). Providers price
prompt, completion, cache, and reasoning tokens differently, so cost accounting is impossible on this
shape. This epic carries the full `TokenUsage` end-to-end and extends it with optional cache/reasoning
fields. It also verifies streaming paths actually accumulate usage (F8).

## 2. Goals

- Replace bare token totals with full `TokenUsage` on every carrier from the LLM port up to
  `RunFinished`.
- Extend `TokenUsage` with optional `cache_read_tokens`, `cache_write_tokens`, `reasoning_tokens`
  (all `#[serde(default)]`, back-compatible).
- Keep a `token_count` accessor for one release as a deprecation shim.
- Verify per-adapter streaming usage accumulation so budgets are trustworthy on `execute_stream`.

## 3. Requirements

- **R1 (D-4).** Carry `TokenUsage` (not a bare `u32`/`u64`) on: `PaladinResult`,
  `BattalionResult.per_paladin_tokens`, `NodeExecutionRecord` (Waypoint), `TraceEvent::NodeFinished`,
  and `RunFinished`. Remove `TokenUsage::from_total` from the battalion aggregation path (it zeroes
  the split). *Confirm exact carrier definitions by grep before editing — the systems analysis names
  these; verify field names/locations in `paladin-core` and the engine.*
- **R2 (D-4).** Extend `TokenUsage` (`paladin-core/.../token_usage.rs`, single definition, re-exported
  ~4×) with `cache_read_tokens: Option<u32>`, `cache_write_tokens: Option<u32>`,
  `reasoning_tokens: Option<u32>`, each `#[serde(default)]`. `total` semantics documented (does/does
  not include cache/reasoning — pick one, state it).
- **R3 (D-4).** Clean break on the carrier shape — no compatibility shim for the downstream (it adopts
  the new `TokenUsage` shape in its coordinated refactor; see overview §5). A `total()` convenience
  accessor may be kept **only if it earns its place as permanent good API** (a genuine convenience on
  a struct that carries a split), not as a temporary bridge for web3sec. Do not add a `#[deprecated]
  remove-in-0.12` accessor whose only purpose is downstream compatibility.
- **R4 (F8).** Audit each LLM adapter's streaming path (`execute_stream`): confirm accumulated usage
  is emitted (the execution service currently accumulates `response.usage.total_tokens` on the
  non-streaming path only). Where a streamed run under-reports, fix it so the same `TokenUsage`
  arrives as on the non-streaming path. Document any adapter that genuinely cannot report streamed
  usage.
- **R5.** Surface the breakdown in at least one herald (json + markdown) so the split is observable
  end-to-end (prerequisite for Epic 5's cost line).
- **R6.** MIGRATION §9.2 entry describing the shape change and the accessor shim; update the
  semver-checks allowlist for the `#[non_exhaustive]` struct changes.

## 4. Out of scope

- Currency cost, pricing tables, `cost_estimate` production (Epic 5).
- Renaming any token type.

## 5. Tests / verification

- Round-trip unit tests: a known `TokenUsage` (non-zero prompt AND completion, plus cache/reasoning)
  survives through each carrier to `RunFinished` without collapsing to a total.
- Battalion test: `per_paladin_tokens` preserves the split (regression against the `from_total` bug).
- Serde back-compat: legacy JSON without the new fields deserializes via defaults; new JSON
  round-trips.
- Streaming: per-adapter test asserting accumulated usage on `execute_stream` matches the
  non-streaming path (or an explicit documented exception).
- `make clean-code` green; coverage bars met.

## 6. Exit criteria

A prompt/completion/cache/reasoning breakdown is visible at `RunFinished` and in a herald; the
battalion path no longer zeroes the split; streamed runs report usage; the change is registered in
MIGRATION and passes semver-checks with the allowlist update.

## 7. Dependencies / downstream impact

- **Depends on:** nothing (keystone).
- **Downstream (Web3sec):** adopts the new `TokenUsage` carrier shape as part of its coordinated
  Milestone 13 refactor (clean break, no shim). The MIGRATION §9.2 entry (R6) is the migration guide.
