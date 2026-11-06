---
created: 2026-11-05T00:00:00Z
title: "Codecov upload fails silently (no CODECOV_TOKEN, fail_ci_if_error unset) and sweeps non-coverage JSON files"
area: ci
severity: minor
files:
  - .github/workflows/ci.yml
owner: repo maintainer
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-72)
---

## Problem

Codecov upload fails silently (no CODECOV_TOKEN, fail_ci_if_error unset) and sweeps non-coverage JSON files.

Carried into the v0.11.1 pre-tag audit register as R-72; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: read STATE.md and .github/workflows/ci.yml on 9eba21d9b: the decision to supply CODECOV_TOKEN and set fail_ci_if_error, or drop the step, is still open; the 82 percent floor gate (cargo llvm-cov --fail-under-lines) is unaffected

## What to do

supply CODECOV_TOKEN and set fail_ci_if_error, or drop the step.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner repo maintainer.

## Source

- Finding: STATE Blockers/Concerns (v0.11.0 close); 47-CI-EVIDENCE Findings
- Register row: R-72 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
