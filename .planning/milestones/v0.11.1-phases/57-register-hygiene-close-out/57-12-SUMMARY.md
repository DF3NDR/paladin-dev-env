---
phase: 57-register-hygiene-close-out
plan: 12
subsystem: security-credential-handling
tags: [error-rendering, llm-error, reasoning-effort, windows-ledger, d-14]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-10 triage (owner option-a) that sent 51-REVIEW WR-01 to fix-in-phase"
provides:
  - "PaladinBuilder::effort_refusal renders a fixed per-variant label (llm_error_label) for any check_reasoning error other than UnsupportedParameter; no LlmError payload reaches the ConfigurationError"
  - "WINDOWS row 94 (51-REVIEW WR-01), open, for 57-11 to mark fixed on this plan's tests and commit"
  - "MIGRATION 11.1 row M-B-41 and a CHANGELOG [Unreleased] Security bullet"
affects: [57-11]

tech-stack:
  added: []
  patterns:
    - "Private fixed-label twin of paladin-battalion's llm_error_class: one arm per LlmError variant plus a wildcard for the non-exhaustive enum, payload never rendered"

key-files:
  created: []
  modified:
    - .planning/WINDOWS.md
    - src/application/services/paladin/paladin_builder.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "PD-12-1: private llm_error_label in paladin_builder.rs, same strings as llm_error_class; no new public LlmError method, so no second crate in scope"
  - "PD-12-2: no redaction helper is called, because the label carries no payload text"
  - "The vendor text is deliberately not logged instead (P-12-3, EA-57-10); a redaction-first diagnostic would be a separate change"

patterns-established:
  - "A rendering boundary that sits above a pass-through adapter (FallbackLlmAdapter forwards a non-UnsupportedParameter error unchanged) names the error by kind only"

requirements-completed: [REG-05, REG-02]

duration: 20min
completed: 2026-11-04
---

# Phase 57 Plan 12: Reasoning-check refusal text Summary

**A failed `check_reasoning` now reaches the caller of `PaladinBuilder::build` as a fixed kind label (`processing error`, `provider error`, ...), never the error's text, directly and through a `FallbackLlmAdapter`; the `UnsupportedParameter` refusal is unchanged.**

## Performance

- **Duration:** ~20 min (two executors, Task 1 then Task 2)
- **Started:** 2026-11-04T18:35Z (plan-start SHA `9e9b5cbc83ff3837bec2a8692c80f13dbf0c6aaa`)
- **Completed:** 2026-11-04T18:55Z
- **Tasks:** 2 (1 tracer, 1 auto)
- **Files modified:** 4

## Accomplishments

- 51-REVIEW WR-01: `effort_refusal`'s non-`UnsupportedParameter` arm renders `llm_error_label(&other)`. New private `fn llm_error_label(error: &LlmError) -> &'static str` has one arm per variant plus the wildcard `unknown error`, with the same strings as `paladin-battalion`'s `llm_error_class`. The `UnsupportedParameter` arm is unchanged. The doc comment above `effort_refusal` now matches the behaviour.
- `crates/paladin-llm/src/fallback.rs` and `crates/paladin-ports/src/output/llm_port.rs` are untouched (P-12-2). The diff against the plan-start SHA is empty for both.
- No `log::`/`tracing::` line was added (the grep count against the plan-start SHA is 0), and no `unwrap`/`expect`/`panic!` was added (P-12-1, P-12-3).
- MIGRATION 11.1 gains `M-B-41`; `[Unreleased]` `### Security` gains one bullet. `# 10.` onward in MIGRATION.md and `## [0.11.0]` onward in CHANGELOG.md are byte-identical to the previous commit (D-00c); `## 11.2` stays header-only.

## Task Commits

1. **Task 1 (tracer): 51 WR-01 end to end** - `f55ff310e` (fix)
2. **Task 2: credential review, records and root-crate gates** - `715c89d20` (docs)

## WINDOWS rows minted

| Id | Description starts | File:line | Kind / phase | Status |
|----|--------------------|-----------|--------------|--------|
| 94 | `51-REVIEW WR-01 (found live by the Phase 57 sweep, D-14): effort_refusal renders the whole LlmError ...` | `src/application/services/paladin/paladin_builder.rs:1202` | `deviation` / `51` | `open` |

It is minted before the fix commit and stays `open`; 57-11 marks it `fixed` on this plan's tests and commit.

## Red-green

All tests are in `application::services::paladin::paladin_builder::tests::`. They use the test-only stub `CheckFailsPort(LlmError)` (an `LlmPort` whose `check_reasoning` returns the held error) and the planted secret `sk-planted-51wr01-secret`.

| Test | RED on the pre-fix head | GREEN |
|------|-------------------------|-------|
| `effort_refusal_renders_only_the_error_kind_for_a_failed_check` | FAILED. Message: `reasoning_effort for model 'probe-model' on provider 'stubbed' could not be checked: Processing error: vendor body: sk-planted-51wr01-secret upstream detail`. The first failing assertion was the lowercase `processing error` label check (the head rendered the `Display` text `Processing error: ...`), and the same message carried the planted secret. | passed |
| `effort_refusal_through_a_fallback_chain_renders_only_the_error_kind` | FAILED. Message: `reasoning_effort for model 'probe-model' on provider 'fallback' could not be checked: Processing error: vendor body: sk-planted-51wr01-secret upstream detail`. Same first failing assertion and the same planted secret. | passed |
| `effort_refusal_for_an_unsupported_parameter_names_model_provider_value_and_reason` | passed (regression guard, exact string match) | passed |

Head: 2 failed, 1 passed. After the fix: `cargo test --lib --features web-server effort_refusal_` gives 3 passed. The whole module, `cargo test --lib --features web-server application::services::paladin::paladin_builder::`, gives 59 passed, including `the_builder_verdict_equals_check_reasoning_for_every_combination` and the chain tests.

## Credential-handling review

Manual review of the changed arm against `.github/instructions/security.instructions.md` (`## Manual review still matters`):

| Check | Evidence | Verdict |
|-------|----------|---------|
| C1 Redaction before truncation | No payload text is rendered at all, so there is nothing to redact or bound. `redact_secret_patterns` is not called (PD-12-2). The two planted-secret tests, direct and through `FallbackLlmAdapter`, assert the message contains neither `sk-planted-51wr01-secret` nor the vendor body text, and the head failed them. | Pass |
| C2 No log line interpolates a key | The fix adds no log line: `git diff 9e9b5cbc83 -- src/application/services/paladin/paladin_builder.rs \| grep -cE '^\+.*(log::\|tracing::)'` prints 0. The dropped payload is not moved into a log (P-12-3). | Pass |
| C3 No key-carrying config is Debug-formatted or serialised | The arm formats `self.data.model`, `self.llm_port.get_provider_name()` and a `&'static str` label only. | Pass |
| C4 No credential-header HTTP client follows redirects | No HTTP client is touched. `git diff 9e9b5cbc83 --stat` lists only `.planning/STATE.md`, `.planning/WINDOWS.md`, `CHANGELOG.md`, `MIGRATION.md` and `paladin_builder.rs`. | Pass (not applicable) |

Residual (accepted, T-57-42 and T-57-43): an operator loses the vendor reason for a failed check, because model, provider and kind are kept but the vendor text is deliberately not logged (EA-57-10; a redaction-first diagnostic would be a separate change). The `UnsupportedParameter.reason` is the port's own refusal wording by contract (ADR-0061); in-tree adapters build it from profile data only.

Manual review stands in for static taint analysis, as `security.instructions.md` states; CodeQL is advisory-only and Snyk was removed, and neither was used here.

## Gates

`df -BM --output=avail /` was printed before each run (9640M down to 9401M, then 9579M). Every cargo run used `CARGO_INCREMENTAL=0`.

| Gate | Result |
|------|--------|
| `cargo test --lib --features web-server effort_refusal_` | 3 passed |
| `cargo test --lib --features web-server application::services::paladin::paladin_builder::` | 59 passed |
| `cargo clippy --lib --tests --features web-server -- -D warnings` | exit 0 (`Finished dev profile`) |
| `make clean-code` | exit 0 (doc-check passed, ADR-0033; all 105 D-05 public API entry points carry a plural `# Examples` heading) |
| `make api-surface` | `API surface unchanged` (4331 items) |
| `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-ai` | `paladin-ai: 223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` |
| `make security` | exit 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| `make check-gates` | exit 0 (12 crates in publish order; version strings equal `0.11.0`; stub-panic scan `0 finding(s)`) |

## Evidence for 57-11

51 WR-01: WINDOWS row **94** (open, kind `deviation`, phase `51`); fix commit `f55ff310e` (Task 1); records commit `715c89d20` (Task 2). Tests: `application::services::paladin::paladin_builder::tests::effort_refusal_renders_only_the_error_kind_for_a_failed_check`, `application::services::paladin::paladin_builder::tests::effort_refusal_through_a_fallback_chain_renders_only_the_error_kind`, `application::services::paladin::paladin_builder::tests::effort_refusal_for_an_unsupported_parameter_names_model_provider_value_and_reason`. Command: `cargo test --lib --features web-server effort_refusal_`.

## Deviations from Plan

None - plan executed exactly as written.

## Auth gates

None.

## Known Stubs

None. The `CheckFailsPort` stub is test-only by design.

## Threat Flags

None. The change removes information disclosure (T-57-41, mitigated) and adds no endpoint, auth path, file access or schema change.

## Issues Encountered

None. No gate failed and no retry was needed.

## Self-Check: PASSED

- FOUND: `/home/user/paladin-dev-env/src/application/services/paladin/paladin_builder.rs` (contains `fn llm_error_label` and `llm_error_label(&other)`)
- FOUND: MIGRATION.md `M-B-41` and CHANGELOG.md bullet, both citing `51-REVIEW WR-01`
- FOUND: WINDOWS row 94, `open`
- FOUND: commits `f55ff310e` and `715c89d20`
