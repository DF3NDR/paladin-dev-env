---
status: partial
phase: 40-tenant-identity-run-read-scoping
source: [40-VERIFICATION.md]
started: 2026-09-29T12:36:44Z
updated: 2026-09-29T19:44:28Z
---

## Current Test

number: 4
name: CI-only evidence
expected: |
  CI postgres-integration, coverage (82% line floor) and the web-server integration job (tests/integration/e2e_platform_api_test.rs) are green on the phase's final tree.
  Blocker status: lifted. The MinIO pull failure was resolved by Phase 45 (RustFS swap, ADR-0055). The Phase 40 final code commit be3a9030 is an ancestor of HEAD 730f521f; every commit since touches only .planning/ or Phase 45 (RustFS/CI/docs).
  Live evidence: ci.yml run 478 (id 36770517439, commit 730f521f, https://github.com/DF3NDR/paladin-dev-env/actions/runs/36770517439) concluded success. Green: "Run every *::postgres contract suite" (postgres-integration, no SKIP path), "Measure coverage" with cargo-llvm-cov (coverage job), "Run the e2e_platform_api test binary" (web-server integration, non-zero test selection asserted), Integration Tests (RustFS), Docker Integration Tests, Kubernetes Smoke Test, unit + doc tests, clippy/fmt/docs, audit, deny, API surface. Only skips are the by-design on-failure steps. Run 472 (id 36751442387, commit 2a97fd9c) also green on the same Phase 40 code.
awaiting: user response

## Tests

### 1. Scope decision on code-review WR-01
expected: Either a fix with a contract-style test for submit and fork, or WINDOWS.md row 58 amended to name the POST /runs {thread_id} and fork paths explicitly. See 40-REVIEW.md WR-01 and 40-VERIFICATION.md human_verification item 1.
result: pass

### 2. Backstop truth (40-03): key-to-tenant mapping immutable after boot
expected: A human confirms the no-mutation-after-boot invariant on AgentAuthConfig (api_keys writes exist only in #[cfg(test)]), or adds a held-out test that clones AgentAuthConfig into two router states and asserts the same key resolves the same tenant under concurrent requests.
result: pass

### 3. Judgment-tier prohibitions (20 items, non-authoritative "held" verdicts)
expected: A human reviews the flagged prohibitions and confirms or rejects the verdicts; in particular re-runs the credential-handling review required by .github/instructions/security.instructions.md over the Phase 40 diff (git diff 629ef660..HEAD).
result: pass

### 4. CI-only evidence
expected: CI postgres-integration, coverage (82% line floor) and the web-server integration job (tests/integration/e2e_platform_api_test.rs) are green on the phase's final tree. The sandbox has no Docker daemon and could not produce this evidence.
result: blocked
blocked_by: third-party
reason: "Blocked. CI runs 444 and 442 on the phase's final code tree fail at container init: quay.io/minio/minio pin returns unauthorized (MinIO locked anonymous pulls ~2026-09-24; pin identical on main, main green 2026-09-23). Coverage, Integration, Docker Integration and K8s Smoke cannot run; postgres-integration, Redis, Ollama, audit, deny, API-surface are green. Not a Phase 40 defect; resolved by Phase 45 (RustFS swap) or an interim image re-pin."

### 5. Disposition of code-review WR-02, WR-03 and WR-04
expected: Decide whether to apply the shared permits check inside RunSubmissionService::cancel and a generic-body repository-error helper in run_controller now, or file them (WINDOWS.md or Phase 46 hygiene). Neither blocks the roadmap criteria, which are met at the HTTP route level.
result: pass

## Summary

total: 5
passed: 4
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps
