---
phase: 50-provider-parity-config-driven-model-swap
plan: 02
subsystem: api
tags: [rust, paladin-ports, paladin-llm, adr, model-profile, parameter-support, decorators, parity]

requires:
  - phase: 50
    provides: "plan 50-01: the nine-provider matrix whose red cases the policy plans (50-03, 50-05) turn green"
provides:
  - "ADR-0061, amending ADR-0004: refuse an explicit sampling parameter the target cannot honour, omit an auto-selected one visibly; D-01..D-04a, profile precedence, owner readings R-1 and R-2 recorded pending"
  - "paladin_ports::output::model_profile: ModelProfile (non_exhaustive, fail closed from new(), with_* setters), ParamSupport::{Range, Fixed, NotSent}, StopSupport, SamplingParam"
  - "LlmPort::model_profile(&self, model) -> Option<ModelProfile>, defaulted to None (additive)"
  - "model_profile forwarded by PricingLlmAdapter and CadenceLlmAdapter; FallbackLlmAdapter returns the intersection of its hops' profiles"
affects: [50-03, 50-05, 50-06, 50-07, 50-14, 50-15, 50-16, 50-17, 50-18, 51]

tech-stack:
  added: []
  patterns:
    - "One admission rule (ParamSupport::admits) shared by the build-time check and the wire filter; no clamping, snapping or rounding"
    - "Fail-closed profile construction: ModelProfile::new() states nothing as sendable, so a forgotten field refuses loudly"
    - "Order-independent intersection (commutative and associative over a NaN/edge table) for a fallback chain's combined profile"
    - "Decorators forward new defaulted port methods, each proven by a test, with the Pricing(Cadence(provider)) composition covered"

key-files:
  created:
    - .planning/decisions/0061-per-model-parameter-support-policy.md
    - crates/paladin-ports/src/output/model_profile.rs
  modified:
    - .planning/decisions/0004-temperature-validation.md
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-llm/src/pricing.rs
    - crates/paladin-llm/src/cadence.rs
    - crates/paladin-llm/src/fallback.rs
    - CHANGELOG.md

key-decisions:
  - "ADR-0061 records owner readings R-1 (run-metadata reach, one-way, decided at 50-18 Task 1) and R-2 (usage-cap evidence per compat vendor, reversible, decided at 50-17 Task 1) as pending rather than choosing for the owner"
  - "ParamSupport::intersect normalises a NaN bound or an inverted range to NotSent first, so commutativity and associativity hold over the NaN rows of the test table"
  - "ModelProfile also implements Default (identical to new(), fail closed) so containers can hold one; it is documented as never a struct-update base"
  - "FallbackLlmAdapter skips a hop that returns None when intersecting (A3-50-02); plan 50-14 re-points each hop at its own model"
  - "The decorator tests share a crate-private, cfg(test) ProfiledStubPort defined in fallback.rs, so no new file and no public surface"

patterns-established:
  - "Licence-plan ADR first, then the interface-first contract plans implement against"
  - "Defaulted additive port method plus forwarding on every decorator, with a composition test"

requirements-completed: [PARITY-02]

coverage:
  - id: D1
    description: "ADR-0061 exists, amends ADR-0004, holds the D-01/D-02 policy, D-03 shape, D-04a semantics, mechanism limit, precedence, registry request-shaping fields and the Owner readings section (R-1, R-2 pending)"
    requirement: "PARITY-02"
    verification:
      - kind: other
        ref: "test -f .planning/decisions/0061-per-model-parameter-support-policy.md && grep -q 'Amends ADR-0004' ... (Task 1 automated verify)"
        status: pass
    human_judgment: false
  - id: D2
    description: "ParamSupport::admits is the single admission rule: inclusive range, exact Fixed, NotSent admits nothing, NaN admitted by none; Range(x,x) equals Fixed(x)"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ports --lib model_profile::tests::param_support_admits_table"
        status: pass
    human_judgment: false
  - id: D3
    description: "ParamSupport, StopSupport and ModelProfile intersect are commutative and associative over the table"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ports --lib model_profile::tests::intersect_is_order_independent"
        status: pass
    human_judgment: false
  - id: D4
    description: "ModelProfile::new() is fail closed; LlmPort::model_profile defaults to None; ProviderCapabilities guard still passes"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ports --lib (model_profile_new_is_fail_closed, model_profile_defaults_to_none, provider_capabilities_gained_no_field)"
        status: pass
      - kind: doc
        ref: "cargo test -p paladin-ports --doc model_profile (12 passed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Pricing and Cadence forward model_profile; Fallback returns the hops' intersection; the Pricing(Cadence(provider)) composition returns the provider's profile"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib (pricing_forwards_model_profile, cadence_forwards_model_profile, pricing_over_cadence_forwards_model_profile, fallback_model_profile_is_the_intersection_of_hops)"
        status: pass
    human_judgment: false

duration: 40min
completed: 2026-10-15
status: complete
---

# Phase 50 Plan 02: Per-Model Parameter Support Policy and ModelProfile Contract Summary

**ADR-0061 (amending ADR-0004) records the refuse-explicit / omit-auto-visibly policy, and `paladin-ports` gains the additive `ModelProfile` / `LlmPort::model_profile` contract with one tested admission rule, forwarded by the Pricing, Cadence and Fallback decorators.**

## Performance

- **Duration:** about 40 min
- **Completed:** 2026-10-15
- **Tasks:** 3 (one docs, two TDD)
- **Files modified:** 11 (2 created)

## Accomplishments

- ADR-0061 is written in the ADR-0060 section shape with lettered Decision subsections (a)-(i), an `## Owner readings` table with R-1 and R-2 both `pending` (rating and deciding checkpoint stated), Considered Options, Consequences, Code Locations, Code Conformance (`pending -- measured by plan 50-16`) and Downstream Consumers. ADR-0004 gained only an `**Amended by ADR-0061**` note (zero lines removed); `PROMOTION.md` has row 0061 and `**Next free ADR number: 0062**` with a dated note; `PROJECT.md` Key Decisions has an ADR-0061 row marked pending Phase 50.
- `crates/paladin-ports/src/output/model_profile.rs`: `ParamSupport` (`admits`, `intersect`), `StopSupport` (`admits`, `intersect`), `SamplingParam` (`as_str`) and the `#[non_exhaustive]` `ModelProfile` with `new()`, eleven `with_*` setters, `support_for` and `intersect`. `new()` is fail closed. 12 doc tests and 9 unit tests, including the NaN, both-endpoint and next-representable-`f32` cases.
- `LlmPort::model_profile` added with a default body returning `None`; the whole workspace (`cargo check --workspace --all-features`) compiles with no implementor touched. `ProviderCapabilities` and `provider_capabilities_gained_no_field` are untouched and green.
- `PricingLlmAdapter` and `CadenceLlmAdapter` delegate to `inner`; `FallbackLlmAdapter` folds its hops' `Some` profiles with `ModelProfile::intersect` (`None` when no hop declares one). Four tests prove it, including the production `Pricing(Cadence(provider))` composition and hop-order independence.

## Task Commits

1. **Task 1: ADR-0061, ADR-0004 note, PROMOTION.md at 0062, PROJECT.md row** - `95f10c08` (docs)
2. **Task 2: ModelProfile, ParamSupport, StopSupport, SamplingParam, LlmPort::model_profile** - `d673c3f8` (feat)
3. **Task 3: decorator forwarding, CHANGELOG entry** - `4aabbbb3` (feat)

**Plan metadata:** recorded in the final `docs(50-02)` commit.

## Files Created/Modified

- `.planning/decisions/0061-per-model-parameter-support-policy.md` - the ADR
- `.planning/decisions/0004-temperature-validation.md` - dated amendment note only
- `.planning/decisions/PROMOTION.md` - row 0061, next free 0062, dated note
- `.planning/PROJECT.md` - Key Decisions row
- `crates/paladin-ports/src/output/model_profile.rs` - the new types and their tests
- `crates/paladin-ports/src/output/mod.rs` - `pub mod model_profile;`
- `crates/paladin-ports/src/output/llm_port.rs` - re-export, `LlmPort::model_profile`, `model_profile_defaults_to_none`
- `crates/paladin-llm/src/{pricing,cadence,fallback}.rs` - forwarding, the intersection, tests, and the `cfg(test)` `ProfiledStubPort`
- `CHANGELOG.md` - root `[Unreleased] ### Added` entry

## Decisions Made

- Owner readings R-1 and R-2 are written as `pending` in ADR-0061, per the plan, rather than resolved by the executor.
- `ParamSupport::intersect` normalises a `NaN` bound or an inverted range to `NotSent` before combining, so the order-independence tests hold including the degenerate rows.
- `ModelProfile` carries a `Default` impl equal to `new()` (fail closed), documented as not a struct-update base. The plan said "no `Default` derive"; an explicit impl is not a derive and keeps the fail-closed value, but see the deviation note.
- A `None` hop in a fallback chain is skipped (A3-50-02), not treated as "nothing sendable".

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `.project/current-exports.txt` cannot gain `ModelProfile`**
- **Found during:** Task 3
- **Issue:** The plan's acceptance criterion expects `grep -c 'ModelProfile' .project/current-exports.txt` to print at least 1. The baseline is generated by `cargo public-api` for the root `paladin-ai` crate only; items defined in `paladin-ports` and `paladin-llm` appear only when the root crate re-exports them, and it does not re-export these.
- **Fix:** Ran `make api-surface-update`; the only change was the generation timestamp, so I reverted that file rather than commit noise. `make api-surface` reports "API surface unchanged (4274 items)", which is the gate that matters. The new surface is recorded in the CHANGELOG entry. `.project/current-exports.txt` is therefore not in the Task 3 commit.
- **Files modified:** none (reverted)
- **Verification:** `make api-surface`, `make check-consumer-surface`, `make check-migration-allowlist`, `make check-api-examples`, `make check-gates` all pass.

**2. [Rule 3 - Blocking] `ModelProfile` has an explicit `Default` impl**
- **Found during:** Task 2
- **Issue:** The plan says "no `Default` derive and no struct-update spread". Clippy's `new_without_default` (denied under `-D warnings`) fires on a public `new()` without `Default`.
- **Fix:** An explicit `impl Default for ModelProfile { fn default() -> Self { Self::new() } }`, documented as fail closed and not a spread base. No derive, so the intent (a forgotten field never becomes sendable) is preserved.
- **Files modified:** `crates/paladin-ports/src/output/model_profile.rs`
- **Committed in:** `d673c3f8`

**3. [Process] Shared test stub placed in `fallback.rs`**
- **Issue:** Three decorator test modules need the same profiled stub port; the plan lists only the three source files.
- **Fix:** One `#[cfg(test)] pub(crate) mod test_support` with `ProfiledStubPort` in `fallback.rs`, used by the pricing and cadence tests. No new file, no public surface.
- **Committed in:** `4aabbbb3`

**Total deviations:** 3 (2 blocking, 1 process). **Impact on plan:** none on scope; the baseline criterion was unsatisfiable as worded and its intent (the surface gate and a changelog record) is met.

## Issues Encountered

- `make clean-code` reached `lint-shell` and stopped because `shellcheck` is not installed in this sandbox. `cargo fmt --check` and `cargo clippy -p paladin-ports -p paladin-llm --all-targets -- -D warnings` (the parts that cover this change) pass, and the whole workspace type-checks with `--all-features`. The shell lint step is unrun; it is recorded in the WINDOWS ledger.
- `make security` (cargo-audit, cargo-deny) was not run: no dependency was added or changed, and the plan's verify list does not include it.

## Verification

- `cargo test -p paladin-ports --lib model_profile`: 10 passed (including `model_profile_defaults_to_none`); `provider_capabilities_gained_no_field` passes
- `cargo test -p paladin-ports --doc model_profile`: 12 passed
- `cargo test -p paladin-llm --all-features --lib`: 655 passed (4 new)
- `cargo check --workspace --all-features`: clean
- `cargo fmt --check`, `cargo clippy` (both crates, all targets, `-D warnings`): clean
- `make api-surface` unchanged; `make check-consumer-surface`, `make check-api-examples`, `make check-gates`, `make check-migration-allowlist`: pass
- Task 1 automated verify passes; `git diff` of ADR-0004 removes no line
- Manual credential-handling review: no credential-bearing type, HTTP client or log statement is touched; the new types carry numeric limits and a date string only.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or file access. The threat register is mitigated as planned: T-50-02-01 by the three forwarding tests including the composition, T-50-02-02 by the edge-value `admits` table, T-50-02-03 by the fail-closed `new()` test.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plans 50-03 (shared wire filter and registry), 50-05 (original adapters' profiles and stop sequences), 50-06 and 50-14 implement against `ModelProfile`. Owner readings R-1 (plan 50-18 Task 1) and R-2 (plan 50-17 Task 1) are still pending in ADR-0061 and need their owner answers there.

## Self-Check: PASSED

- Files found: ADR-0061, `model_profile.rs`, this SUMMARY.
- Commits found: `95f10c08`, `d673c3f8`, `4aabbbb3`.
