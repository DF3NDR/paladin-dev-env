---
status: testing
phase: 51-adjustable-reasoning-effort
source: [51-VERIFICATION.md]
started: 2026-10-23T05:05:00Z
updated: 2026-10-23T13:54:47Z
---

## Current Test

number: 1
name: CI coverage job: cargo llvm-cov --fail-under-lines 82 on the merged branch
expected: |
  Workspace line coverage at or above the 82 percent floor (ADR-0006) with the phase's added
  Rust lines (about 16.6k from plans 51-01 to 51-14 plus about 3.3k from plans 51-15 to 51-20)
  included.
awaiting: user response

## Tests

### 1. CI coverage job: cargo llvm-cov --fail-under-lines 82 on the merged branch
expected: Workspace line coverage at or above the 82 percent floor (ADR-0006) with the phase's added Rust lines included. cargo-llvm-cov is absent from the devcontainer, so only the CI `coverage` job can confirm it.
result: [pending]

### 2. Decide whether the Anthropic Budget path shipping as operator-override-only satisfies REASON-06
expected: Owner accepts that Budget(n) on Anthropic refuses on every seeded model until `llm.anthropic.models.<id>.effort.budget` declares a span (validation logic, lower bound 1024 and below max_tokens, is implemented and tested against an override), or asks for a seeded span for the claude-opus-4-6 / claude-sonnet-4-6 pair where 1024 was accepted and 1023 rejected. A policy call about ADR-0061's seed rule against REASON-06's wording; unchanged by the gap-closure plans.
result: [pending]

### 3. Decide whether Gemini 2.5 thinkingBudget ranges need an override-documented waiver
expected: Owner accepts that no Gemini 2.5 id was probed (retired for new users, 404; excluded in the candidate set), so those ids stay Unmeasured and refuse until overridden. The requirement text names a live re-verification that could not be performed against retired models; unchanged by the gap-closure plans.
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps

- gap_id: G-51-1a
  truth: "Phase 51 code passes the repo's own pre-push gate (make doc-check, ADR-0033 zero rustdoc warnings)"
  status: resolved
  reason: "Found while attempting the push needed for Test 1: `make doc-check` failed with 6 rustdoc warnings — `check_reasoning` docs (plan 51-20) linked private `Self::build_request` / `Self::wire_profile` via intra-doc links in the anthropic, deepseek, gemini and openai adapters."
  severity: minor
  test: 1
  root_cause: "Intra-doc links to private methods warn under `cargo doc --no-deps` without --document-private-items; the house style for private fns is a plain code span."
  artifacts:
    - path: "crates/paladin-llm/src/{anthropic,deepseek,gemini,openai}/adapter.rs"
      issue: "12 `[`Self::build_request`]` / `[`Self::wire_profile`]` links in check_reasoning / model_profile docs"
  missing: []
  resolved_by: "inline fix during UAT (plain code spans), commit on this branch"
  resolved_at: 2026-10-23

- gap_id: G-51-1b
  truth: "The pre-push API-surface hook renders the baseline with the pinned nightly, as CI and `make api-surface` do"
  status: resolved
  reason: "Found on the second push attempt for Test 1: the `check-api-surface` pre-push hook called scripts/check-api-surface.sh bare, so PUBLIC_API_TOOLCHAIN fell through to floating `nightly`; 1,140 rendering-only diff lines (`alloc::sync::Arc` vs `alloc::rcs::arc::Arc`, `-> Self`), zero public items changed. `make api-surface` (pinned nightly-2026-09-20) reports 4323 items unchanged."
  severity: minor
  test: 1
  root_cause: "Tooling, not Phase 51 code: the hook entry bypassed the Makefile's toolchain pin, the exact drift ci.yml:224 documents."
  artifacts:
    - path: ".pre-commit-config.yaml"
      issue: "check-api-surface entry ran the script directly without PUBLIC_API_TOOLCHAIN"
  missing: []
  resolved_by: "inline fix during UAT: hook entry now `make api-surface`"
  resolved_at: 2026-10-23
