---
status: testing
phase: 56-dogfood-findings
source: [56-VERIFICATION.md]
started: 2026-11-04T01:40:07Z
updated: 2026-11-04T01:40:07Z
---

## Current Test

number: 1
name: DF-18 severity call — is S3 / out (re-homed) acceptable for the Quickstart's deprecated `gpt-4` literal?
expected: |
  Maintainer either accepts S3 (docs literal, clear provider error, 109-site sweep re-homed to
  .planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md) or regrades to S2, in
  which case the one-line Quickstart fix (docs/src/getting-started/quickstart.md:53) lands before the tag.
awaiting: user response

## Tests

### 1. DF-18 severity call — Quickstart `.model("gpt-4")` fails against a real OpenAI key
expected: Accept S3 / re-homed (v0.11.1 ships the Quickstart line unchanged; the 109-site sweep is a todo), or regrade to S2 and land the one-line Quickstart fix before the tag. The first program a new user copies exits 1 on a working key; the triager's reason (fixing one of 109 leaves the root cause) is recorded.
result: [pending]

### 2. DOGF-05 scope — does a devcontainer walk satisfy "on a Docker-capable machine"?
expected: Accept the D-17 pass (local 92.79 %, CI 92.86 %, both truncate to 92, both ≥ 82) knowing guide step 1 (`make services-up`) and the bare-host password-protected Redis path (trap 2) were not exercised; or ask for a bare-host re-run.
result: [pending]

### 3. Judgment-tier prohibitions P1, P4, P11 and test-tier P3
expected: Maintainer confirms P1 (no simulated web3sec return), P4 (no web3sec disposition without evidence) and P11 (the agent did not pick the D-16 option) held; P3 (findings log append-only) is accepted on the measured evidence (zero deleted lines since the seed commit) or gets a standing enforcement test.
result: [pending]

### 4. Relayed keyed evidence — K1..K9 and the DF-19 Anthropic re-check
expected: Maintainer confirms the K2/K6/K7/K8 output in 56-RUNBOOK-KEYED-WALK.md is theirs, and that `setup-check --verbose` read `✓ Anthropic` on commit 889b5f1bb on 2026-11-03. K1 was recorded as "not captured".
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
