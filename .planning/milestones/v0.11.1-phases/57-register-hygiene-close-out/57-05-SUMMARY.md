---
phase: 57-register-hygiene-close-out
plan: 05
subsystem: security-credential-handling
tags: [redaction, run-worker, operator-webhook, hmac-signature, windows-ledger, d-14]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "the Phase 57 sweep's plan-time liveness check that found 45-REVIEW WR-05 and 41-REVIEW WR-05 still live"
provides:
  - "bounded_run_error: a failed run's stored error and the worker's failed-persist warn! carry redacted-then-bounded text"
  - "an empty operator-webhook secret is treated as absent, and boot warns once when the operator webhook has no secret"
  - "WINDOWS rows 88 (45-REVIEW WR-05) and 89 (41-REVIEW WR-05), both open, for 57-10 and 57-11 to close on this plan's tests"
affects: [57-10, 57-11]

tech-stack:
  added: []
  patterns:
    - "redact-then-bound helper mirroring webhook/service.rs bounded_error (D-43), applied at the persist boundary"
    - "pure boot-warning line function (operator_webhook_secret_warning_line), tested without a logger, modelled on unpriced_model_warning_line"

key-files:
  created: []
  modified:
    - .planning/WINDOWS.md
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - src/application/services/run/webhook/service.rs
    - src/config/treasurer.rs
    - src/infrastructure/web/run_api_wiring.rs
    - MIGRATION.md
    - CHANGELOG.md
    - docs/src/api-reference/platform-api.md

key-decisions:
  - "PD-05-2: RUN_ERROR_CHAR_BUDGET = 2048, pub(super) in worker.rs so the sibling worker_tests module can see it; no public item changes"
  - "PD-05-3: 41 WR-05 is additive. Empty means absent and boot warns; nothing is refused, signature bytes for a configured secret are unchanged, and a ${...} placeholder is neither refused nor warned about (deferred Phase 54 WR-04)"
  - "operator_webhook_secret_warning_line takes Option<&AllowanceWebhookConfig> (the webhook config type run_api_wiring already reads) rather than the whole TreasurerConfig"

patterns-established:
  - "A failure text that can embed a provider body is redacted before it is bounded, then persisted and logged from the same rebound variable"

requirements-completed: [REG-05, REG-02]

duration: 12min
completed: 2026-11-04
---

# Phase 57 Plan 05: Run-error redaction and operator-webhook secret hygiene Summary

**A failed run's stored error and worker log line are now redacted-then-bounded (`bounded_run_error`), and an empty operator-webhook secret counts as absent with one boot warning that names the key and never the URL.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-11-04T14:12Z (first WINDOWS row minted 14:12:20)
- **Completed:** 2026-11-04T14:24Z
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files modified:** 9

## Accomplishments

- 45-REVIEW WR-05: `persist_failure` rebinds `error_text = bounded_run_error(&error_text)` at its top, so both the `record_outcome` call and the failed-persist `warn!` carry only `redact_secret_patterns` then `bounded_excerpt` (2,048 characters) output.
- 41-REVIEW WR-05: `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET=""` now clears the secret, `with_operator_notice_secret` filters empty to `None`, and the wiring emits one `warn!` from `operator_webhook_secret_warning_line` when the operator webhook has no non-empty secret.
- The test that pinned the raw secret marker as readable through `GET /runs/{id}` was replaced by assertions that the prefix survives and the marker does not.
- MIGRATION, CHANGELOG and `platform-api.md` updated; the public API surface is unchanged.

## Task Commits

1. **Task 1 (tracer): 45 WR-05 end to end** - `43d740c34` (fix)
2. **Task 2: 41 WR-05** - `d26c871d8` (fix)
3. **Task 3: records and root-crate gates** - `a01178a99` (docs)

## WINDOWS rows minted

Both rows are kind `deviation`, status `open` (57-11 marks them `fixed` on this plan's tests).

| Id | Description starts | File:line |
|----|--------------------|-----------|
| 88 | `45-REVIEW WR-05 (found live by the Phase 57 sweep, D-14): persist_failure stores the agent/engine failure text verbatim ...` | `src/application/services/run/worker.rs:1995` |
| 89 | `41-REVIEW WR-05 (found live by the Phase 57 sweep, D-14): an operator webhook configured without a secret ...` | `src/infrastructure/web/run_api_wiring.rs:1352` |

## Red-green

| Test | RED observed | GREEN |
|------|--------------|-------|
| `agent_kind_run_emits_exactly_one_terminal_event` (amended) | Runtime failure at `worker_tests.rs:1737`: `the stored run error must not carry the secret-shaped marker, got "Execution error: llm exploded: sk-agent-secret-marker"` | passes |
| `bounded_run_error_redacts_before_bounding` | Compile failure E0425: `bounded_run_error` and `RUN_ERROR_CHAR_BUDGET` not found (the helper did not exist) | passes |
| `empty_env_operator_webhook_secret_is_treated_as_absent` | Runtime failure at `treasurer.rs:1560`: `an empty env value is treated as absent`, `left: Some("")`, `right: None` | passes |
| `operator_webhook_secret_warning_line_names_the_key_and_never_the_url` | Compile failure E0422/E0425: `AllowanceWebhookConfig` not in scope and `operator_webhook_secret_warning_line` not found | passes |
| `operator_webhook_secret_warning_line_is_silent_with_a_secret` | Same compile failure as above | passes |

## Gates

- `cargo test --lib --features web-server`: `test result: ok. 1507 passed; 0 failed` (full lib suite, run after Task 2).
- Targeted: `application::services::run::worker` 72 passed; `application::services::run::webhook::` 37 passed; `infrastructure::web::run_api_wiring::` 51 passed; `config::treasurer::` 50 passed.
- `cargo clippy --lib --tests --features web-server -- -D warnings`: finished with no warnings.
- `cargo fmt --all -- --check`: clean.
- `make api-surface`: `API surface unchanged` (4331 items).
- `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-ai`: `223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`.
- `make check-gates`: exit 0 (last lines: `OK: scanned 626 file(s) (19 test-only), 0 finding(s)`).
- Doc-record greps from Task 3's `<verify>` (MIGRATION 11.1 and 11.8, CHANGELOG `[Unreleased]`, `platform-api.md`): all pass.

## Evidence

Test names for 57-10 and 57-11 to cite:

- 45 WR-05: `application::services::run::worker_tests::bounded_run_error_redacts_before_bounding` and `application::services::run::worker_tests::agent_kind_run_emits_exactly_one_terminal_event`.
- 41 WR-05: `config::treasurer::tests::empty_env_operator_webhook_secret_is_treated_as_absent`, `infrastructure::web::run_api_wiring::tests::operator_webhook_secret_warning_line_names_the_key_and_never_the_url` and `infrastructure::web::run_api_wiring::tests::operator_webhook_secret_warning_line_is_silent_with_a_secret`.
- Existing signature tests under `application::services::run::webhook::` still pass, which is the evidence that the signature bytes for a configured secret are unchanged (P-05-2).

## Deviations from Plan

### Notes

**1. Order of the 41 WR-05 row mint.** The plan's D-14 order is "mint the row before its fix". I minted WINDOWS row 89 after the Task 2 fix code was written but before it was committed, so the row exists before the commit that carries the fix. Row 88 (45 WR-05) was minted before any change, as planned. No content impact.

**2. Warning-line signature.** `operator_webhook_secret_warning_line` takes `Option<&AllowanceWebhookConfig>` rather than `&TreasurerConfig`; the plan allowed "the webhook config type it already reads". `AllowanceWebhookConfig` was added to `run_api_wiring.rs` imports.

**3. Doc comment touch-up.** A stale comment in `worker.rs` ("stays readable through `GET /runs/{id}`") was reworded to mention redaction, so no source text still claims the raw failure is readable. This is the comment-level twin of the removed test assertion.

**4. Extra verification.** I ran the full `cargo test --lib --features web-server` suite beyond the plan's targeted runs, per CLAUDE.md's pre-commit order.

### Tracer gate

tracer gate: verify re-run passed; orchestrator-approved (unattended session), not owner-approved. The Task 1 `<verify>` (both tests, the three greps, and the WINDOWS row check) was re-run after the tracer commit and passed before the expansion tasks started.

### Auto-fixed Issues

None.

## Threat Flags

None. T-57-17 and T-57-19 are mitigated by the tests above; T-57-18 is mitigated by empty-is-absent, the boot warning and the docs; T-57-20 (an absent secret still signs with the empty key) is accepted per the plan and documented in `platform-api.md` and MIGRATION 11.8 step 11.

## Known Stubs

None.

## Issues Encountered

None. Disk had 3.6 GB free throughout; no ENOSPC retry was needed.

## Next Phase Readiness

57-10 can record both reviews as `closed` pointing at this plan, and 57-11 can mark WINDOWS rows 88 and 89 `fixed` on the tests listed under Evidence.

## Self-Check: PASSED

- FOUND: `src/application/services/run/worker.rs` contains `fn bounded_run_error`
- FOUND: `src/infrastructure/web/run_api_wiring.rs` contains `fn operator_webhook_secret_warning_line`
- FOUND commits: `43d740c34`, `d26c871d8`, `a01178a99`
- FOUND: WINDOWS rows 88 and 89, status `open`
