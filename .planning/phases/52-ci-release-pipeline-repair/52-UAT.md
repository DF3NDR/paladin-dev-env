---
status: testing
phase: 52-ci-release-pipeline-repair
source: [52-VERIFICATION.md]
started: 2026-10-30T00:08:40Z
updated: 2026-10-30T00:27:45Z
---

## Current Test

number: 2
name: Platform CodeQL dismissals read-back
expected: |
  With a token that can read code-scanning alerts, GET /repos/DF3NDR/paladin-dev-env/code-scanning/alerts?state=open
  shows 0 open CodeQL rule alerts and ?state=dismissed shows the 37 register alert numbers dismissed with the
  register's reasons (24 false positive, 12 used in tests, 1 won't fix); the 20 OSV advisory alerts stay open by
  design. (Unauthenticated API returns 403; H-04 currently rests on the maintainer's pasted read-back in
  52-HANDOVER.md. Roadmap SC5 as worded is about the register, which is verified.)
awaiting: user response

## Tests

### 1. Judgment-tier prohibitions (10 must-NOT items under 'Prohibitions' in 52-VERIFICATION.md)
expected: Maintainer agrees each did not happen; verifier verdicts are non-authoritative (ADR-550 D3/D4). The one needing an explicit reading is plan 52-08's prohibition on executor-performed maintainer acts (H-02..H-04 ran in a Claude Code session with the maintainer's token; H-01 by the maintainer directly).
result: pass

### 2. Platform CodeQL dismissals read-back
expected: With a token that can read code-scanning alerts, GET /repos/DF3NDR/paladin-dev-env/code-scanning/alerts?state=open shows 0 open CodeQL rule alerts and ?state=dismissed shows the 37 register alert numbers dismissed with the register's reasons (24 false positive, 12 used in tests, 1 won't fix); the 20 OSV advisory alerts stay open by design. (Unauthenticated API returns 403; H-04 currently rests on the maintainer's pasted read-back in 52-HANDOVER.md. Roadmap SC5 as worded is about the register, which is verified.)
result: [pending]

### 3. Merge of PR #61 and the later v0.11.1 tag
expected: PR #61 (open, label 'release', head on claude/laughing-dirac-e0h2ax) merges with all 46 required contexts green; the first live use of the release-body size guard is the v0.11.1 tag in Phase 58 (the guard has only been exercised by stubbed harnesses, never a live release call, D-12).
result: [pending]

## Summary

total: 3
passed: 1
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
