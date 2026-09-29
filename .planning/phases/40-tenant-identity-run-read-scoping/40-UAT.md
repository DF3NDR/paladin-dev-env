---
status: testing
phase: 40-tenant-identity-run-read-scoping
source: [40-VERIFICATION.md]
started: 2026-09-29T12:36:44Z
updated: 2026-09-29T12:36:44Z
---

## Current Test

number: 1
name: Scope decision on code-review WR-01 (cross-tenant thread resume/fork via POST /v1/runs {thread_id} and POST /v1/threads/{id}/fork)
expected: |
  Either a fix with a contract-style test for submit and fork (apply RunReadScope::permits to the
  thread's latest run and return the same not-found error as a missing thread), or WINDOWS.md row 58
  amended to name both write paths explicitly, before Phase 41 builds admission enforcement on the
  thread path.
awaiting: user response

## Tests

### 1. Scope decision on code-review WR-01
expected: Either a fix with a contract-style test for submit and fork, or WINDOWS.md row 58 amended to name the POST /runs {thread_id} and fork paths explicitly. See 40-REVIEW.md WR-01 and 40-VERIFICATION.md human_verification item 1.
result: [pending]

### 2. Backstop truth (40-03): key-to-tenant mapping immutable after boot
expected: A human confirms the no-mutation-after-boot invariant on AgentAuthConfig (api_keys writes exist only in #[cfg(test)]), or adds a held-out test that clones AgentAuthConfig into two router states and asserts the same key resolves the same tenant under concurrent requests.
result: [pending]

### 3. Judgment-tier prohibitions (20 items, non-authoritative "held" verdicts)
expected: A human reviews the flagged prohibitions and confirms or rejects the verdicts; in particular re-runs the credential-handling review required by .github/instructions/security.instructions.md over the Phase 40 diff (git diff 629ef660..HEAD).
result: [pending]

### 4. CI-only evidence
expected: CI postgres-integration, coverage (82% line floor) and the web-server integration job (tests/integration/e2e_platform_api_test.rs) are green on the phase's final tree. The sandbox has no Docker daemon and could not produce this evidence.
result: [pending]

### 5. Disposition of code-review WR-02, WR-03 and WR-04
expected: Decide whether to apply the shared permits check inside RunSubmissionService::cancel and a generic-body repository-error helper in run_controller now, or file them (WINDOWS.md or Phase 46 hygiene). Neither blocks the roadmap criteria, which are met at the HTTP route level.
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
