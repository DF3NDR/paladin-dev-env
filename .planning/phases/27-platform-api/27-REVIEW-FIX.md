---
phase: 27-platform-api
fixed_at: 2026-09-08T18:20:00Z
review_path: /workspace/.planning/phases/27-platform-api/27-REVIEW.md
iteration: 1
findings_in_scope: 1
fixed: 1
skipped: 1
status: partial
---

# Phase 27: Code Review Fix Report

**Fixed at:** 2026-09-08T18:20:00Z
**Source review:** /workspace/.planning/phases/27-platform-api/27-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope (fix_scope=critical_warning): 1 (WR-27-01)
- Fixed: 1
- Skipped: 1 (IN-27-01, out of scope per fix_scope=critical_warning)

## Fixed Issues

### WR-27-01: `Ok(None)` signing-key branch still sends with a fallback empty key, unlike the just-fixed `Err` branch

**Files modified:** `src/application/services/run/webhook/service.rs`, `src/application/services/run/webhook/tests.rs`
**Commit:** `4b6592de`
**Applied fix:** Changed the `Ok(None)` arm of the signing-key match in `WebhookDeliveryService::process` (previously `Ok(None) => String::new()`, which fell through to sign-and-send with an empty key) to mirror the sibling `Err` arm fixed for WR-01: it now logs a `log::warn!` naming the missing run and delivery, computes a backoff delay via `backoff_for(new_attempt)`, and calls `self.finish` with `WebhookAttemptOutcome::Retrying { next_attempt_at }` and a bounded `last_error` describing the missing run, then returns without ever calling the HTTP client. Added `webhook_signing_key_missing_run_reschedules_without_sending` to `tests.rs`, mirroring `webhook_signing_key_load_failure_reschedules_without_sending` exactly but using a new `MissingRunRepository` double (a `RunRepositoryPort` impl whose `get` always returns `Ok(None)`) in place of `FailingRunRepository`. The test asserts the mock HTTP target receives zero hits, the delivery ends in `Retrying` status with `next_attempt_at` strictly after the clock, and `last_error` is populated while `last_response_status` stays `None` — the same shape asserted for the `Err` case.

Verification: `cargo fmt --check` passed (no diff). `cargo clippy -p paladin-ai --tests -- -D warnings` passed with exit code 0, no warnings. `cargo test -p paladin-ai --lib webhook` passed: 40/40 tests green, including the new `webhook_signing_key_missing_run_reschedules_without_sending` and the pre-existing `webhook_signing_key_load_failure_reschedules_without_sending`. The commit's pre-commit hook additionally re-ran `cargo fmt (check)` and `cargo clippy (workspace, -D warnings)`, both reported `Passed`.

## Skipped Issues

### IN-27-01: Stale `eslint-disable-next-line` comment on an ES `import`, not a `require()`

**File:** `scripts/sdk-smoke/smoke.ts:30-31`
**Reason:** Out of scope for this run. `fix_scope` is `critical_warning`; IN-27-01 is an Info-severity finding, and the orchestrator prompt explicitly excluded it ("IN-27-01 is Info and OUT of scope — do not touch `scripts/sdk-smoke/smoke.ts`"). No changes were made to this file.
**Original issue:** The import statement is preceded by `// eslint-disable-next-line @typescript-eslint/no-var-requires`, but the following line is an ES module `import`, not a `require()` call — `no-var-requires` only fires on `require(...)`. The directive is a harmless no-op today but is misleading and would itself trigger an "unused eslint-disable directive" lint if ESLint were ever wired up for this file.

---

_Fixed: 2026-09-08T18:20:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
