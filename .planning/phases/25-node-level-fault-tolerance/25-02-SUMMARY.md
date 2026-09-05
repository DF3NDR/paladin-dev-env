---
phase: 25-node-level-fault-tolerance
plan: 02
subsystem: infra
tags: [error-taxonomy, thiserror, semver, transience, non-exhaustive]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-01)
    provides: "paladin_core::platform::container::transience::Transience (Transient/Permanent/Unknown, not non_exhaustive)"
provides:
  - "PaladinError::transience() and LlmError::transience(): table-driven, one arm per variant, classify from typed fields only"
  - "PaladinError::LlmFailure { transience, status, provider, message } — Display byte-identical to legacy LlmError(String), is_retryable() -> true"
  - "LlmError::ProviderError { provider, status, message } and LlmError::AllProvidersFailed { attempts, last: Box<LlmError> }"
  - "BattalionError::Node(NodeError), path-qualified to the structured node_error::NodeError, never the legacy battalion::NodeError summary"
  - "PaladinError, LlmError and BattalionError marked #[non_exhaustive], with wildcard arms at every downstream exhaustive match and a completed X-10 register (MIGRATION.md §9.2 + .cargo/semver-checks-allowlist.toml + per-crate lint suppressions)"
affects: [25-03-registries-and-validation, 25-05-provider-status-mapping, 25-08-fallback-chain, 25-14-semver-gate-evidence]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Table-driven per-variant classification test as the enforcement mechanism for D-05 (a new variant added later without its own transience() arm fails the match arm exhaustiveness check at compile time, and its test-table row is what proves the classification, not an assumption)"
    - "Non-exhaustive downstream wildcard arms delegate to the enum's own transience()/classification method rather than guessing a boolean, so a future variant degrades gracefully instead of silently misclassifying"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/paladin_error.rs
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-core/src/platform/container/battalion/mod.rs
    - crates/paladin-core/Cargo.toml
    - crates/paladin-ports/Cargo.toml
    - .cargo/semver-checks-allowlist.toml
    - MIGRATION.md
    - crates/paladin-battalion/src/conclave_execution_service.rs
    - crates/paladin-battalion/src/llm_decision.rs
    - crates/paladin-llm/src/compat/engine.rs

key-decisions:
  - "This plan's execution crashed mid-Task-2 in a prior session. The orchestrator's crash-recovery commit (cf8e2d3f) had already added match arms for LlmFailure/ProviderError/AllProvidersFailed to the three exhaustive-match consumers named in the plan's read_first. This executor re-derived the rest of Task 2 from the plan text against the current tree rather than applying the crashed session's saved patch verbatim, then added a SECOND wildcard arm to each of those same three consumers once BattalionError/PaladinError/LlmError were actually marked #[non_exhaustive] -- the crash-recovery commit's arms covered the new VARIANTS, not the non-exhaustive-enum compiler requirement, which is a separate obligation that only appears once #[non_exhaustive] itself lands."
  - "conclave_execution_service::is_retryable_error's wildcard arm delegates to error.transience() == Transient rather than a hardcoded false, mirroring the LlmFailure arm immediately above it -- a future PaladinError variant gets classified by its own typed transience field instead of a silently-wrong guess."
  - "compat::engine::classify_fetch_failure's rustdoc previously stated 'deliberately no wildcard arm' as a design choice forcing a compile error on new variants. That design intent is now overridden by the compiler: #[non_exhaustive] on LlmError, a type from a downstream crate, makes a wildcard arm mandatory regardless of enum-local exhaustiveness. Rustdoc was updated to record this precisely (compiler-required, not a design relaxation) rather than leaving stale text that contradicts the code beneath it."
  - "llm_decision::llm_error_class's wildcard returns the generic label \"unknown error\" -- consistent with every other classification label in that function being a short fixed string safe to interpolate, never provider-supplied text."

requirements-completed: [FT-01]

coverage:
  - id: D1
    description: "PaladinError::transience() and LlmError::transience() classify every variant from typed fields only, with a table-driven test asserting one row per variant (landed by Task 1, 2a70f579)"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/paladin_error.rs#tests::paladin_error_transience_table"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/llm_port.rs#tests::llm_error_transience_table"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/llm_port.rs#tests::provider_error_status_boundaries_classify_by_value"
        status: pass
    human_judgment: false
  - id: D2
    description: "BattalionError::Node(NodeError) carries the structured node_error::NodeError (never the legacy battalion::NodeError summary), is Clone, and its Display names the node id and source summary"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battalion/mod.rs#tests::battalion_error_node_carries_the_structured_node_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battalion/mod.rs#tests::battalion_error_node_uses_the_core_node_error_not_the_legacy_summary"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battalion/mod.rs#tests::legacy_battalion_node_error_summary_is_unchanged"
        status: pass
    human_judgment: false
  - id: D3
    description: "PaladinError, LlmError and BattalionError are all #[non_exhaustive]; every in-tree exhaustive match gained a wildcard arm; the workspace builds and lints clean across all targets and features"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "cargo build --workspace --all-features --all-targets (exit 0)"
        status: pass
      - kind: other
        ref: "cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: other
        ref: "cargo fmt --check (exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The X-10 register (MIGRATION.md §9.2 rows + .cargo/semver-checks-allowlist.toml entries + per-crate lint suppressions) is complete and set-equal: 4 allowlist entries, 3 enum_marked_non_exhaustive lints, 4 §9.2 rows marked Y"
    requirement: FT-01
    verification:
      - kind: other
        ref: "test \"$(grep -c '^[[entry]]' .cargo/semver-checks-allowlist.toml)\" -eq 4 (pass)"
        status: pass
      - kind: other
        ref: "test \"$(grep -c 'enum_marked_non_exhaustive' .cargo/semver-checks-allowlist.toml)\" -eq 3 (pass)"
        status: pass
    human_judgment: false

duration: 34min (this session, resuming after crash; ~165min total across both sessions per commit timestamps 16:54-20:25)
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 02: FT-01 Error Taxonomy — transience(), LlmFailure/ProviderError/AllProvidersFailed, BattalionError::Node, X-10 Register Summary

**Landed table-driven `transience()` on `PaladinError`/`LlmError`, the structured `LlmFailure`/`ProviderError`/`AllProvidersFailed` variants, `BattalionError::Node(NodeError)`, and marked all three pre-existing public enums `#[non_exhaustive]` with a complete `MIGRATION.md` §9.2 / semver-allowlist register — resuming a crashed prior session from its Task 1 commit and the orchestrator's interim match-arm fix.**

## Performance

- **Duration:** ~34 min (this resumed session: verification of prior work + all of Task 2's remaining scope)
- **Started:** 2026-09-05T19:50:00Z (approx, this session's spawn)
- **Completed:** 2026-09-05T20:25:00Z
- **Tasks:** 2 (Task 1 landed by a prior crashed session; Task 2 completed this session)
- **Files modified:** 10 across the whole plan (this session's commit touched all 10; Task 1's commit touched 2, the crash-recovery commit touched 3 of the same 10)

## Accomplishments

- **Task 1 (verified, not redone)** — `2a70f579`: `PaladinError::LlmFailure { transience, status, provider, message }` with `Display` byte-identical to the legacy `LlmError(String)` arm; `PaladinError::transience()` and `LlmError::transience()` as exhaustive, one-arm-per-variant classifiers reading typed fields only; `LlmError::ProviderError { provider, status, message }` and `LlmError::AllProvidersFailed { attempts, last: Box<LlmError> }`; table-driven tests pinning every variant's classification, the 408/429/5xx-vs-other-4xx boundary, empty-message non-effect, `AllProvidersFailed`'s last-error delegation, and every legacy `is_retryable()`/`is_terminal()` answer unchanged (470 `paladin-ai-core` + 115 `paladin-ports` lib tests passing at that commit).
- **Crash-recovery interim fix (verified, not redone)** — `cf8e2d3f`: exhaustive-match arms for the three new `LlmFailure`/`ProviderError`/`AllProvidersFailed` variants in `conclave_execution_service::is_retryable_error`, `llm_decision::llm_error_class`, and `compat::engine::classify_fetch_failure`, restoring workspace compilation after the crash.
- **Task 2 (this session)** — `d15d5204`:
  - `BattalionError::Node(NodeError)`, path-qualified to the new `paladin_core::platform::container::node_error::NodeError` (never the legacy `battalion::NodeError { node_name, error }` summary, whose rustdoc now cross-links the split per D-06).
  - `PaladinError`, `LlmError` and `BattalionError` marked `#[non_exhaustive]` (X-10.2, D-04); `StopReason` deliberately untouched (owned by RT-02, Phase 26 — confirmed by a zero-line diff on `execution_result.rs`).
  - A second round of wildcard arms added to the same three consumer sites the crash-recovery commit touched — the crash-recovery arms covered the new *variants*; this round covers the separate, compiler-mandated non-exhaustive-enum wildcard requirement, which only appears once `#[non_exhaustive]` itself is applied:
    - `is_retryable_error`: wildcard delegates to `error.transience() == Transient`, matching the `LlmFailure` arm's own approach.
    - `llm_error_class`: wildcard returns the generic `"unknown error"` label.
    - `classify_fetch_failure`: wildcard returns `Supported` (matching the existing generic non-success catch-all arms); its rustdoc was corrected from "deliberately no wildcard arm" to record that the wildcard is now compiler-required, not a design relaxation.
  - Three `.cargo/semver-checks-allowlist.toml` `[[entry]]` blocks (`enum_marked_non_exhaustive`, requirement `FT-01`) and matching `[package.metadata.cargo-semver-checks.lints]` suppressions added to `crates/paladin-core/Cargo.toml` and `crates/paladin-ports/Cargo.toml`, mirroring the pre-existing `paladin-web` precedent.
  - Three `MIGRATION.md` §9.2 `TBD` rows (`BattalionError`, `PaladinError`, `LlmError`) resolved to `Y` with their D-02/D-03/D-06 justifications; the `PaladinResult` row is left `TBD`, owned by plan 25-08, as scoped.
  - Three new tests: `battalion_error_node_carries_the_structured_node_error`, `battalion_error_node_uses_the_core_node_error_not_the_legacy_summary`, `legacy_battalion_node_error_summary_is_unchanged`.
- Confirmed counts: allowlist holds exactly 4 `[[entry]]` blocks (1 pre-existing + 3 new), exactly 3 `enum_marked_non_exhaustive` lints, and §9.2 has exactly 4 rows marked `Y`.

## Task Commits

1. **Task 1: transience() on PaladinError and LlmError, table-driven per variant** — `2a70f579` (feat) — landed by a prior crashed session; verified present and correct, not redone.
   - Interim: `cf8e2d3f` (fix) — orchestrator's crash-recovery commit restoring compilation.
2. **Task 2: BattalionError::Node, the three-enum non-exhaustive change, and the X-10 register rows** — `d15d5204` (feat) — this session.

**Plan metadata:** (this commit, `docs(25-02): ...`)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/paladin_error.rs` — `LlmFailure` variant, `transience()`, `#[non_exhaustive]` (Task 1 + Task 2)
- `crates/paladin-ports/src/output/llm_port.rs` — `ProviderError`/`AllProvidersFailed` variants, `transience()`, `#[non_exhaustive]` (Task 1 + Task 2)
- `crates/paladin-core/src/platform/container/battalion/mod.rs` — `BattalionError::Node(NodeError)`, `#[non_exhaustive]`, legacy `NodeError` rustdoc cross-link, 3 new tests (Task 2)
- `crates/paladin-core/Cargo.toml` — `[package.metadata.cargo-semver-checks.lints] enum_marked_non_exhaustive = "allow"` (Task 2)
- `crates/paladin-ports/Cargo.toml` — same suppression (Task 2)
- `.cargo/semver-checks-allowlist.toml` — 3 new `[[entry]]` blocks (Task 2)
- `MIGRATION.md` — 3 §9.2 rows resolved TBD -> Y (Task 2)
- `crates/paladin-battalion/src/conclave_execution_service.rs` — new-variant arms (crash recovery) + non-exhaustive wildcard arm (Task 2)
- `crates/paladin-battalion/src/llm_decision.rs` — same pattern (crash recovery + Task 2)
- `crates/paladin-llm/src/compat/engine.rs` — same pattern (crash recovery + Task 2), rustdoc corrected

## Decisions Made

- **Re-derived Task 2 from the plan rather than applying the crashed session's saved patch verbatim.** The prior session's uncommitted WIP (saved to a scratchpad patch by the orchestrator) was consistent with the plan and used as a structural hint, but this executor independently verified every piece against the current tree (which already included the crash-recovery commit's arms) before writing it, per the resume instructions.
- **Two separate rounds of wildcard-arm additions to the same three files** are not redundant: the crash-recovery commit's arms handle the three concrete new *variants* (`LlmFailure`/`ProviderError`/`AllProvidersFailed`), which the compiler required the moment those variants existed, independent of `#[non_exhaustive]`. This session's arms handle the *foreign non-exhaustive enum* requirement, which only exists once `#[non_exhaustive]` itself lands — Rust requires a wildcard in a downstream crate's match over a non-exhaustive enum even when every current variant already has its own arm.
- **`is_retryable_error`'s wildcard delegates to `transience()`** rather than a bare `false`, so a future `PaladinError` variant is classified by its own typed field instead of silently defaulting to non-retryable (which could be equally wrong).
- **`classify_fetch_failure`'s rustdoc was corrected, not just its code.** The function's original design intent ("deliberately no wildcard arm... rather than silently landing in a catch-all") is now factually false once `LlmError` is `#[non_exhaustive]` — leaving the stale claim would mislead a future reader into thinking the wildcard is a lapse rather than a compiler requirement.

## Deviations from Plan

None beyond the crash-resume mechanics documented above (Task 1 verified rather than re-executed, Task 2 executed against a tree that already had the crash-recovery commit's variant-arms). No Rule 1-4 auto-fixes were needed: the plan's own instructions anticipated the exact match sites that broke, and the two rounds of wildcard-arm additions were both scoped exactly as the plan's `<action>` text specified ("Run `cargo build --workspace --all-features --all-targets` and add a wildcard arm to every exhaustive match the compiler names").

## Issues Encountered

None beyond the crash-resume context itself, which is documented in full above and in `<resume_state>`.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The FT-01 error taxonomy (`transience()`, `LlmFailure`, `ProviderError`, `AllProvidersFailed`, `BattalionError::Node`) is in its final shape; downstream plans (25-05 provider-status mapping, 25-08 fallback chain) build on stable types.
- `cargo semver-checks check-release --workspace --baseline-version 0.9.0` was attempted but did not complete within this session's time budget (full-workspace baseline rebuild); per the plan's own `<verification>` section, "full gate evidence is plan 25-14's; this plan confirms no unregistered break" — the register itself (§9.2 + allowlist, set-equal, counted) is the artifact this plan is responsible for, and it is complete and verified by direct count.
- `PaladinResult`'s §9.2 row remains `TBD`, explicitly owned by plan 25-08, as scoped by this plan's own action text.
- No blockers for the next wave.

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-05*
