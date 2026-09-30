---
status: testing
phase: 45-rustfs-swap-platform-observability-deviations
source: [45-VERIFICATION.md]
started: 2026-09-30T17:06:56Z
updated: 2026-09-30T17:06:56Z
---

## Current Test

number: 1
name: CI run on the phase tree — RustFS service containers and the contract suite
expected: |
  Push the phase tree and read the first ci.yml run: the Integration Tests, Coverage, Docker Integration Tests and Kubernetes Smoke Test jobs are green; each RustFS service container / compose service / k8s pod becomes healthy on /health/ready; the contract-suite step prints "test result: ok. N passed" with N >= 11 (compiled-in count check also >= 11); the smoke test applies k8s/rustfs.yaml and "kubectl wait -l app=rustfs" succeeds. Record the run id here.
awaiting: user response

## Tests

### 1. CI run on the phase tree — RustFS service containers and the contract suite
expected: Push the phase tree and read the first ci.yml run: the Integration Tests, Coverage, Docker Integration Tests and Kubernetes Smoke Test jobs are green; each RustFS service container / compose service / k8s pod becomes healthy on /health/ready; the contract-suite step prints "test result: ok. N passed" with N >= 11 (compiled-in count check also >= 11); the smoke test applies k8s/rustfs.yaml and "kubectl wait -l app=rustfs" succeeds. Record the run id here. (Backstop truths from 45-01 and 45-04; no Docker daemon in the authoring sandbox.)
result: [pending]

### 2. actionlint in the same CI run
expected: actionlint passes with the services.command suppression removed from .github/actionlint.yaml (no "command:" key remains in any workflow). (Backstop truth from 45-04; actionlint not installed in the sandbox.)
result: [pending]

### 3. Docs workflow for the retitled storage page
expected: mdbook build and mdbook-linkcheck pass for docs/src/appendix/minio-file-repository-setup.md; the SUMMARY.md entry "S3-Compatible File Storage Setup" resolves. (Backstop truth from 45-06; mdbook not installed in the sandbox.)
result: [pending]

### 4. Maintainer acceptance of the OBS-05 re-measured tracing overhead
expected: Decide whether to accept +19.36 % (log_sink) / +16.19 % (composite) at point C versus the <= 3 % PRD 07 bar as the new recorded figure (45-BENCH-EVIDENCE.md, WINDOWS.md row 61 open). If accepted, run "node .claude/gsd-core/bin/gsd-tools.cjs windows waive 61 \"<acceptance text>\"". If rejected, row 61 stays open and further optimisation or the I/O-bound re-scope becomes a follow-up. Weigh the noise caveat: point C run 1 measured +6.98 % / +7.29 %, and the target-off rows still cost +16-19 %, so the remaining cost is in the dispatcher/sink path, not serialisation. (Backstop truth from 45-07; CONTEXT D-19, Phase 28 D-37 precedent.)
result: [pending]

### 5. Follow-ups (not Phase 45 criteria)
expected: On a Docker-capable machine, "make services-up" then "make coverage" reproduces CI's coverage figure (todo 2026-08-13); once CI is green, "/gsd-verify-work 40" flips Phase 40 UAT test 4 from blocked to pass (CONTEXT D-02).
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
