---
phase: 44-legacy-clean-break-removal
verified: 2026-10-11T00:00:00Z
status: passed
score: 4/4 roadmap success criteria verified (plus 7/7 supporting plan-level truths spot-checked)
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
deferred: []
advisory_followups:
  - id: WR-01
    severity: warning
    summary: "Phalanx cancellation does not stop Paladins that have not started (queued behind with_max_concurrency)"
    violates_success_criterion: none
  - id: WR-02
    severity: warning
    summary: "Phalanx::with_max_concurrency(0) builds Semaphore::new(0); the permit wait is outside any per-attempt bound, so the run hangs"
    violates_success_criterion: none
  - id: WR-03
    severity: warning
    summary: "Removed v0.10 BattalionConfig/ConclaveConfig keys are silently ignored (fail-open to Aegis::default())"
    violates_success_criterion: none
  - id: WR-04
    severity: warning
    summary: "Commander no longer bounds ChainOfCommand/Grove; run_timeout is per-attempt for Formation/Phalanx/Campaign but a whole-flow clock for Maneuver; Maneuver ignores aegis.on_error/retry without a warn"
    violates_success_criterion: none
  - id: WR-05
    severity: warning
    summary: "Crate-wide cargo-semver-checks allow lines mask future accidental removals in paladin-core and paladin-battalion"
    violates_success_criterion: none
---

# Phase 44: Legacy Clean-Break Removal Verification Report

**Phase Goal:** The superseded Battalion error/retry/timeout surfaces and the untyped `LlmError` string variant are gone from the tree, with every removal recorded.
**Verified:** 2026-10-11
**Status:** passed (with five advisory warnings, none of which violates a success criterion or a plan must-have; see "Review warnings assessment")
**Re-verification:** No, initial verification
**HEAD:** 8d05e975 on claude/laughing-dirac-e0h2ax

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `battalion::RetryPolicy`, `battalion::ErrorStrategy`, `battalion::NodeError` no longer exist; same-named Aegis and Flow types untouched and compile | VERIFIED | Independent grep of `crates/paladin-core/src/platform/container/battalion/` and `crates/paladin-battalion/src`: every remaining `RetryPolicy` is the Aegis type (`aegis::RetryPolicy`, aliased `AegisRetry` in `battalion/mod.rs`, `engine/graph_doc.rs`, `superstep.rs`); every remaining `NodeError` is `node_error::NodeError` (the structured type, now the element of `BattalionResult.node_errors`, `battalion/mod.rs:618`, and `BattalionError::Node`, `mod.rs:872`). No `ContinueOnError`/`RetryThenContinue`/`calculate_retry_delay` anywhere in `crates/`, `src/`, `examples/`, `tests/` (outside the guard file). `crates/paladin-battalion/src/retry.rs` is deleted. `cargo check --example phalanx_parallel --example commander_full_config --example campaign_workflow --example conclave_expert_panel` finishes clean (whole workspace type-checks, so the Aegis/Flow/maneuver types compile). |
| 2 | Formation, Phalanx, Campaign carry no legacy timeout handling; Aegis timeouts are the only timeout path | VERIFIED | `grep` of `timeout(`/`timeout_seconds` in the five legacy-pattern service files shows no production `timeout(` call; the sole `tokio::time::timeout` is `aegis_attempt.rs:157` in `attempt_once`. `BattalionConfig` has no `timeout_seconds` (only a v0.10 JSON fixture in a test module, `mod.rs:1071`). `BattalionError::Timeout` and `ConclaveError::Timeout` gone. `cargo test -p paladin-battalion --lib -- aegis_attempt conclave_retry_predicate phalanx_ formation_ campaign_ commander_builder`: 83 passed, 0 failed (paused-clock per-attempt tests included). The remaining `timeout_seconds` in the CLI (`src/application/cli/config/battalion_config.rs:284`, YAML templates) is the CLI's own YAML key, mapped onto `aegis.timeout.run_timeout` per ADR-0059 Open Question 8, not legacy handling. `Maneuver` has its own whole-flow timeout, which is out of scope (the criterion names Formation, Phalanx, Campaign). |
| 3 | `PaladinError::LlmError(String)` gone; `conclave_execution_service.rs` retry check uses typed `LlmFailure`/`Transience` | VERIFIED | `paladin_error.rs` has `LlmFailure { transience, .. }` and no `LlmError` variant, no `is_retryable`/`is_terminal`. Repo-wide grep for `PaladinError::LlmError` and `.is_retryable()` is empty; the remaining `LlmError(String)` hits are `HandoffError`, `PromptError`, `PlanningError` (explicitly out of scope, ADR-0059 D-15) and the `.github/copilot-instructions.md` illustrative snippet. `conclave_execution_service.rs:370-371`: `is_retryable_error` is `error.transience() == Transience::Transient`, no message sniffing; tests `conclave_retry_predicate_is_transient_only` / `..._ignores_message_text` pass. |
| 4 | X-03 supersession in an ADR; every removal has a MIGRATION 9.2 row, a semver-checks allowlist entry, updated examples/docs | VERIFIED | ADR-0059 exists (Accepted, `## Supersedes` ADR-0001/0002, "X-03 is superseded for Phase 44 only"); ADR-0001 and ADR-0002 read `Superseded`; PROMOTION index and PROJECT.md rows present. MIGRATION.md 9.2 has Phase 44 rows (lines 296-305) for `BattalionConfig`, `RetryPolicy`, `ErrorStrategy`, `NodeError`, `BattalionResult`, `BattalionError`, `PaladinError`, `ConclaveConfig`, `ConclaveError`, `paladin-battalion | retry`; 9.1 rows M-B-06..; 9.4 stored-JSON note. `.cargo/semver-checks-allowlist.toml` has 16 Phase 44 entries, one per measured lint per row (the `BattalionResult` row is deliberately register-only, `N`). `./scripts/check-migration-allowlist.sh` run by me: "Allowlist is set-equal to the MIGRATION.md 9.2 deliberate-breaking register". `./scripts/check-changelogs.sh`: pass. `git diff --stat b362d6cb HEAD -- examples`: 9 examples migrated; the mdBook pages listed in ADR Code Locations are changed; `cargo check` of four of them passes. |

**Score:** 4/4 roadmap truths verified; 0 behavior-unverified.

### Supporting plan-level truths spot-checked

| Truth (plan) | Status | Evidence |
|--------------|--------|----------|
| Single tracked removal guard exists and can fail (44-13) | VERIFIED | `cargo test --test legacy_removal_guard` run by me: 3 passed (`legacy_surfaces_are_gone_from_the_tree`, and positive controls `scanner_reports_planted_legacy_uses`, `scanner_accepts_a_clean_tree`). |
| `BattalionConfig.aegis` + `validate_aegis` + runner (44-01) | VERIFIED | `aegis_attempt.rs` present, runner matrix tests pass; Campaign/Formation/Phalanx call `run_with_aegis`. |
| Phalanx: no whole-run wrapper, typed failures in declaration order, backoff cancellation (44-05) | VERIFIED | `phalanx_service.rs` has no `timeout(`; `test_cancellation_support`, `phalanx_*` tests pass. The scope of the cancellation must-have is "returns `Cancelled`; a token cancelled while a Paladin waits in backoff stops its retries"; both are tested. |
| `node_errors` retyped with operator decision recorded (44-06) | VERIFIED | `battalion/mod.rs:618` structured type; ADR-0059 (g) records the 2026-10-11 option-a outcome. |
| Commander bridges and 300 s per-attempt builder default (44-07) | VERIFIED | `commander_builder_*` tests pass; ADR (c)/(e). |
| Circuit breaker on `transience()` (44-03) | VERIFIED | Documented and tested (`circuit_breaker_counts_transient_failures_only`); CHANGELOG records the behavior delta. |
| ADR Code Conformance `conforms` (44-12) | VERIFIED | Named tests exist; allowlist/MIGRATION gate re-run by me. |

### Requirements Coverage

Every PLAN `requirements:` ID is accounted for; REQUIREMENTS.md maps exactly LEGACY-01..04 to Phase 44 (lines 231-234, all `Complete`, checkboxes `[x]`). No orphaned requirement.

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| LEGACY-01 | 44-01, 04, 05, 06, 10, 12, 13 | SATISFIED | SC1 evidence above. |
| LEGACY-02 | 44-01, 04, 05, 07, 09, 10, 12, 13 | SATISFIED | SC2 evidence above. |
| LEGACY-03 | 44-03, 07, 09, 10, 12, 13 | SATISFIED | SC3 evidence above. |
| LEGACY-04 | 44-02, 08, 09, 11, 12 | SATISFIED | SC4 evidence above. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Source-tree guard | `cargo test --test legacy_removal_guard` | 3 passed | PASS |
| Aegis runner, Conclave predicate, Phalanx/Formation/Campaign/Commander builder | `cargo test -p paladin-battalion --lib -- aegis_attempt conclave_retry_predicate phalanx_ formation_ campaign_ commander_builder` | 83 passed, 0 failed | PASS |
| Register/allowlist set equality | `./scripts/check-migration-allowlist.sh` | set-equal | PASS |
| Changelog coverage | `./scripts/check-changelogs.sh` | 11 crates OK | PASS |
| Examples type-check | `cargo check --example phalanx_parallel --example commander_full_config --example campaign_workflow --example conclave_expert_panel` | Finished, no errors | PASS |

### Probe Execution

Step 7c: SKIPPED. No probe is declared in any Phase 44 PLAN.

### Anti-Patterns Found

No unreferenced `TBD`/`FIXME`/`XXX` debt markers were introduced in the reviewed files (the guard and review found none). The review's IN-02 (dead `_started_at` binding, pre-existing) and IN-03 (weak `>= 4` assertion) are info-level only.

### Pre-existing test failures

`paladin-ai` `infrastructure::web::run_api_wiring::tests::build_run_api_persists_*` (2 tests) fail. They fail identically with 44-01's source restored to baseline 061ee3dc (deferred-items.md; orchestrator-confirmed). They touch neither Campaign, `aegis_attempt` nor `BattalionConfig`. Not a Phase 44 regression and not attributable to this phase. They remain a standing red item for the Run API owner.

## Review warnings assessment (precise, as requested)

None of the five warnings contradicts a ROADMAP success criterion or a PLAN must-have as written. Each is an advisory follow-up. The reasoning:

- **WR-02 (`with_max_concurrency(0)` hangs).** Confirmed in code: `Phalanx::with_max_concurrency` stores `Some(0)` unvalidated (`phalanx.rs:103-106`), `phalanx_service.rs:344-345` builds `Semaphore::new(0)`, and the pre-phase file (061ee3dc, lines 136/161/329) shows the permit acquire used to sit inside the whole-run `timeout`. It is a genuine robustness regression: a degenerate, already-accepted-without-validation configuration changed from `BattalionError::Timeout` to an indefinite hang. It does not violate SC2: SC2 says Aegis timeouts are the only timeout path and the legacy handling is gone, and that is true; ADR-0059 D-02 deliberately removed the whole-run clock and states there is no whole-Battalion bound. It violates no 44-05 must-have either (the "closed limiter becomes a recorded failure" must-have concerns a closed semaphore, not a zero-permit one). The input was never valid (a zero-concurrency Phalanx cannot run), so the right fix is input validation (clamp or reject zero in `with_max_concurrency`/`Phalanx::validate`), not a restored clock. Recommended as a small follow-up fix before v0.11.0 is published (Phase 47), since it is a one-line change with a test.
- **WR-01 (queued Paladins start after cancel).** Latent before the phase (the pre-phase tasks were also detached `tokio::spawn`s). The 44-05 must-have is that `execute_with_cancellation` returns `Cancelled` and that a token cancelled during backoff stops retries; both hold and are tested. Not a gap. Follow-up: check the token after permit acquisition and before the first attempt, or use a `JoinSet`.
- **WR-03 (removed config keys silently ignored).** This is the documented, tested clean-break behavior: ADR-0059 D-00c forbids a compatibility deserializer, D-01 states a serialized document "loses three keys and gains one", and `battalion_config_v0_10_document_deserializes_with_default_aegis` pins it. The MIGRATION 9.2 `BattalionConfig` row and the paladin-core CHANGELOG say it in plain words. It satisfies LEGACY-04's "every removal recorded". The residual concern is operational (fail-open for a deployment that stored `timeout_seconds`), so a one-line callout in the 9.1 M-B-06 row or 9.8 upgrade checklist (Phase 46 owns the upgrade guide) would be a cheap mitigation. Rejecting the keys would be an explicit decision change to D-00c, to be taken by the operator, not a Phase 44 defect.
- **WR-04.** The Commander wrapper removal and the unbounded ChainOfCommand/Grove are decided (D-02/D-03, ADR (c)) and recorded in M-B-06 which names both patterns. The Maneuver `run_timeout` semantics is stated in D-04 ("a whole-flow bound, which is Maneuver's own meaning"). The remaining gap is only documentation completeness (the `aegis` rustdoc omits Maneuver; no one-time `warn!` for Maneuver as Campaign has). Advisory.
- **WR-05.** The crate-wide allow lines are the mechanism the plan prescribes (44-11) and match the repo's prior-phase convention (the root `Cargo.toml` already uses one for Phase 43). The allowlist-vs-register set-equality gate runs and passes. A dated `TODO: remove after 0.11.0` is a hygiene improvement for Phase 47, not a Phase 44 gap.

## Human Verification Required

None required for the success criteria. Two gates named in ADR-0059 (D-00d) could not run locally and remain CI-only: the 82 percent coverage floor (`cargo-llvm-cov` not installed in the sandbox) and mdBook rendering (`mdbook` not installed); `make check-doc-examples`/`check-doc-config` passed per 44-12. These are standing CI evidence for the PR, not truths of the phase goal. I did not re-run `make security` or the full `make check-examples` (62/62 per 44-12); I re-ran four example type-checks and the allowlist/changelog gates myself.

## Gaps Summary

No gaps. All four ROADMAP success criteria and LEGACY-01..04 are met in the code, with independent evidence (greps, a passing source-tree guard with positive controls, 83 targeted unit tests, the register/allowlist gate, example type-checks). Recommended follow-ups, in priority order: (1) WR-02 zero-concurrency validation, (2) WR-01 cancellation check after permit acquisition, (3) the WR-03 callout in the upgrade guide (Phase 46), (4) WR-04 Maneuver documentation and warn, (5) WR-05 dated TODO next to the new semver allow lines (Phase 47). The two `run_api_wiring` test failures are pre-existing and belong to a separate triage.

---

_Verified: 2026-10-11_
_Verifier: Claude (gsd-verifier)_
