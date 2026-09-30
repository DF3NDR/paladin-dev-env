---
status: testing
phase: 45-rustfs-swap-platform-observability-deviations
source: [45-VERIFICATION.md]
started: 2026-09-30T17:06:56Z
updated: 2026-09-30T19:10:00Z
---

## Current Test

number: 3
name: Docs workflow for the retitled storage page
expected: |
  mdbook build and mdbook-linkcheck pass for docs/src/appendix/minio-file-repository-setup.md; the SUMMARY.md entry "S3-Compatible File Storage Setup" resolves.
awaiting: user response (docs.yml triggers only on pull_request and push to main, so no run exists for the branch push yet)

## Tests

### 1. CI run on the phase tree — RustFS service containers and the contract suite
expected: Push the phase tree and read the first ci.yml run: the Integration Tests, Coverage, Docker Integration Tests and Kubernetes Smoke Test jobs are green; each RustFS service container / compose service / k8s pod becomes healthy on /health/ready; the contract-suite step prints "test result: ok. N passed" with N >= 11 (compiled-in count check also >= 11); the smoke test applies k8s/rustfs.yaml and "kubectl wait -l app=rustfs" succeeds. Record the run id here. (Backstop truths from 45-01 and 45-04; no Docker daemon in the authoring sandbox.)
result: pass — ci.yml run 36751442387 (run number 472, commit 2a97fd9c, https://github.com/DF3NDR/paladin-dev-env/actions/runs/36751442387) concluded success: 34 jobs green, 3 skipped by design (Benchmark Regression Signal, Publish Dry Run, End-to-End Tests). Integration Tests, Coverage, Docker Integration Tests, Docker Build and Kubernetes Smoke Test all green. Docker Integration Tests log: "test result: ok. 11 passed; 0 failed" against rustfs-test:9000 and "file storage contract suite passed: 11". Kubernetes Smoke Test log: pod paladin-rustfs-69bd87f78-89xzn (Labels app=rustfs, image rustfs/rustfs:1.0.0, Image ID sha256:8cc9801755448b71a786705ce76692c77e14936cccd87cf2fc31842e58f4d1ff, readiness http-get /health/ready, RUSTFS_ACCESS_KEY/RUSTFS_SECRET_KEY from paladin-secrets) reached Ready=True with restart count 0; paladin pod's wait-for-rustfs init container completed and "Startup time: 6 seconds" passed the 30 s budget. The earlier run 36749137406 (471, commit de716c39) was cancelled by concurrency when 472 superseded it.

### 2. actionlint in the same CI run
expected: actionlint passes with the services.command suppression removed from .github/actionlint.yaml (no "command:" key remains in any workflow). (Backstop truth from 45-04; actionlint not installed in the sandbox.)
result: pass — Workflow Lint job green in ci.yml run 36751442387 (and in the earlier run 36749137406 before it was superseded).

### 3. Docs workflow for the retitled storage page
expected: mdbook build and mdbook-linkcheck pass for docs/src/appendix/minio-file-repository-setup.md; the SUMMARY.md entry "S3-Compatible File Storage Setup" resolves. (Backstop truth from 45-06; mdbook not installed in the sandbox.)
result: [pending] — docs.yml triggers only on pull_request and push to main, so no Build MDBook run exists for the branch push. Evidence lands with the first pull request opened from this branch (Build MDBook is a required check there). SUMMARY.md line 115 carries the "S3-Compatible File Storage Setup" entry pointing at appendix/minio-file-repository-setup.md.

### 4. Maintainer acceptance of the OBS-05 re-measured tracing overhead
expected: Decide whether to accept +19.36 % (log_sink) / +16.19 % (composite) at point C versus the <= 3 % PRD 07 bar as the new recorded figure (45-BENCH-EVIDENCE.md, WINDOWS.md row 61 open). If accepted, run "node .claude/gsd-core/bin/gsd-tools.cjs windows waive 61 \"<acceptance text>\"". If rejected, row 61 stays open and further optimisation or the I/O-bound re-scope becomes a follow-up. Weigh the noise caveat: point C run 1 measured +6.98 % / +7.29 %, and the target-off rows still cost +16-19 %, so the remaining cost is in the dispatcher/sink path, not serialisation. (Backstop truth from 45-07; CONTEXT D-19, Phase 28 D-37 precedent.)
result: [pending]

### 5. Follow-ups (not Phase 45 criteria)
expected: On a Docker-capable machine, "make services-up" then "make coverage" reproduces CI's coverage figure (todo 2026-08-13); once CI is green, "/gsd-verify-work 40" flips Phase 40 UAT test 4 from blocked to pass (CONTEXT D-02).
result: [pending]

## Summary

total: 5
passed: 2
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
