# Phase 32: Unified Token Primitives - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-15
**Phase:** 32-unified-token-primitives
**Mode:** `--auto` — every question resolved to the recommended option without user prompts
**Areas discussed:** Shared resolver contract, Exactness on the port, Legacy counter retirement, Equivalence proof and release bookkeeping

---

## Shared resolver contract (PRIM-04)

### Q: How is strict mode expressed?

| Option | Description | Selected |
|--------|-------------|----------|
| Fallback-policy enum | `Default(u32)` (lenient, always resolves) vs `Strict { caller_fallback: Option<u32> }` (errors when nothing yields a window); Commissary's caller fallback keeps working as caller policy | ✓ |
| `bool strict` + `Option<u32>` default | Two redundant/invalid states (strict + Some default, lenient + None) | |
| Two functions (`resolve` / `resolve_strict`) | Duplicates the precedence walk | |

**Auto-selected:** Fallback-policy enum (recommended default) → D-01

### Q: Module placement and result shape?

| Option | Description | Selected |
|--------|-------------|----------|
| Top-level `paladin_llm::window` | The PRD's own example path; result carries tokens + `WindowSource`; facade re-export next to Commissary | ✓ |
| `services::window` next to commissary | Groups with the one consumer in-crate | |
| Private helper inside `commissary.rs` | HistoryTrimmer would import from a service module | |

**Auto-selected:** Top-level `window` module (recommended default) → D-02

### Q: Does Commissary gain a config table?

| Option | Description | Selected |
|--------|-------------|----------|
| No | `CommissaryPlan` unchanged; empty table keeps windows identical (PRD §4) | ✓ |
| Yes, add `model_context_limits` to `CommissaryPlan` | New capability; behaviour change | |

**Auto-selected:** No (recommended default) → D-03; the table is a deferred idea

### Q: When does Commissary resolve?

| Option | Description | Selected |
|--------|-------------|----------|
| Once in `new`, stored | Replaces the inline guard; same error variant and text | ✓ |
| Per `allotted_tokens()` call | Re-resolves every time; no benefit | |

**Auto-selected:** Once in `new` (recommended default) → D-04, D-05

---

## Exactness on the port (PRIM-01, PRIM-02)

### Q: What does `is_exact` promise?

| Option | Description | Selected |
|--------|-------------|----------|
| Adapter-instance property, default `false` | "This adapter as constructed counts with the model's own tokenizer" | ✓ |
| Per-call `is_exact(model)` | Changes the port signature; over-reach | |

**Auto-selected:** Adapter-instance property (recommended default) → D-06

### Q: `TiktokenCounter::is_exact` returns?

| Option | Description | Selected |
|--------|-------------|----------|
| `true` unconditionally, caveat in rustdoc | Exact for the encoding resolved at `new(model)`; `count` ignores `model` today | ✓ |
| `true` only when `model == self.model_name` | Needs the model plumbed into `is_exact` | |

**Auto-selected:** Unconditional `true` (recommended default) → D-07

### Q: Heuristic — override or rely on default?

| Option | Description | Selected |
|--------|-------------|----------|
| Rely on trait default; port doc test proves the default | Test asserting `false` doubles as the PRIM-01 default proof | ✓ |
| Explicit `fn is_exact(&self) -> bool { false }` override | Redundant with the default | |

**Auto-selected:** Rely on default (recommended default) → D-07

### Q: How does Commissary source `Stockpile::exact_tally`?

| Option | Description | Selected |
|--------|-------------|----------|
| Read `counter.is_exact()` at use; drop the cached field | One source of truth | ✓ |
| Cache at construction in the existing private field | Duplicate state | |

**Auto-selected:** Read from the port at use (recommended default) → D-08

---

## Legacy counter retirement (PRIM-03)

### Q: Use the `#[deprecated]` escape hatch?

| Option | Description | Selected |
|--------|-------------|----------|
| No — remove outright | Scout found zero in-tree callers outside the definition file and three re-export sites | ✓ |
| Deprecate first, remove in Phase 33 | Only sanctioned if a caller cannot migrate; none exists | |

**Auto-selected:** Remove outright (recommended default) → D-09

### Q: What survives on `TiktokenCounter`?

| Option | Description | Selected |
|--------|-------------|----------|
| `new`, `impl TokenCounterPort`, inherent `model_name`, `is_exact` | Minimal surface; `count_tokens` folded into `count` | ✓ |
| Also keep a fallible inherent `count_tokens` | A second counting path — the thing this phase removes | |

**Auto-selected:** Minimal surface (recommended default) → D-10

### Q: Facade `token_counter` compat sub-module?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep, narrowed to `TiktokenCounter` | Compat path still resolves for the surviving type | ✓ |
| Delete the sub-module | An extra break with no PRD mandate | |

**Auto-selected:** Keep, narrowed (recommended default) → D-11

### Q: Doc sweep?

| Option | Description | Selected |
|--------|-------------|----------|
| Named file list + exit grep | `lib.rs` feature table, crate-map, memory-management, commissary docs + mdBook page, upgrading/migration pages | ✓ |
| Grep-and-fix ad hoc | No exit criterion | |

**Auto-selected:** Named list + exit grep (recommended default) → D-12

---

## Equivalence proof and release bookkeeping (PRIM-04, PRIM-05)

### Q: What is the "equivalence snapshot"?

| Option | Description | Selected |
|--------|-------------|----------|
| TDD-ordered fixture tests committed before the refactor, unchanged after | Commit order is the proof; distinct non-round numbers | ✓ |
| `insta` snapshots | Only `paladin-eval` pins insta; new dev-dep for no gain | |

**Auto-selected:** TDD-ordered fixtures (recommended default) → D-13

### Q: Lint discovery for feature-gated removals?

| Option | Description | Selected |
|--------|-------------|----------|
| Empirical; run `--default-features` (CI's command) AND `--features content-processing` for `paladin-memory`/`paladin-ai` | CI cannot observe the gated removal; record what fires and why | ✓ |
| Guess the lint ids | Contradicts Phase 31 D-27 | |

**Auto-selected:** Empirical, both feature sets (recommended default) → D-14

### Q: §9.2 row granularity?

| Option | Description | Selected |
|--------|-------------|----------|
| One row per `crate \| Type` the row-level gate keys on | `paladin-llm \| Commissary`, `paladin-memory \| TokenCounter`, `paladin-memory \| TokenCounterFactory`; `paladin-ai`/`paladin-ports` only if a lint fires | ✓ |
| One row per requirement ("legacy-counter removal") | The gate reduces the type cell to its first identifier — a combined row would key on one type only | |

**Auto-selected:** Per `crate | Type` (recommended default) → D-15

### Q: CHANGELOG and commit mechanics?

| Option | Description | Selected |
|--------|-------------|----------|
| `[0.10.0]` Changed/Removed/Added bullets; plain `git commit`; gate evidence in the last SUMMARY | Phase 31 D-28 + Phase 30 specifics | ✓ |
| `[Unreleased]` section | Contradicts PRIM-05 and Phase 31 D-28 | |

**Auto-selected:** `[0.10.0]`, plain `git commit` (recommended default) → D-16

---

## Todo cross-reference

| Todo | Score | Action |
|------|-------|--------|
| Verify local `make coverage` reproduces CI's 82.39 % | 0.6 | Folded as a verification note (same as Phase 31) |
| Evaluate replacing MinIO with RustFS | 0.9 | Reviewed, NOT folded — keyword-only match, no scope overlap; scope guardrail overrides the mechanical ≥ 0.4 rule |

## Claude's Discretion

- Exact type/variant names for the resolver and whether it takes `&ProviderCapabilities` or `Option<u32>`.
- Whether `history.rs`'s `LimitSource` is deleted or mapped to `WindowSource`.
- `TiktokenCounter` cache implementation detail while the trait is removed.
- Test-double naming and the configurable-exactness knob.
- Plan/wave granularity (four natural waves listed in CONTEXT.md).

## Deferred Ideas

- Per-model override table on `CommissaryPlan`.
- Commissary production caller, RAG truncation, release-gate re-seal — Phase 33.
- Resolver in `paladin-ports` — rejected for this phase (ROADMAP/PRD lock `paladin-llm`).
- `#[non_exhaustive]` on `CommissaryPlan` / `Commissary`.
- Treasurer and everything cost-shaped — Milestone 14.
