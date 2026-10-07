---
status: testing
phase: 42-mid-run-halt-sse-terminal-status
source: [42-VERIFICATION.md]
started: 2026-10-07T10:54:02Z
updated: 2026-10-07T20:55:10Z
---

## Current Test

number: 2
name: Decide the ROADMAP SC1/SC2 and REQUIREMENTS ALLOW-03 wording for the agent loop
expected: |
  Agent-kind runs have no Waypoint by construction (CONTEXT D-08, WINDOWS.md row 64); a halted
  agent-kind run resumes by a fresh POST /v1/runs, while the engine path continues from its Halted
  Waypoint. Either amend the roadmap and requirement wording or accept the override the verifier
  drafted in 42-VERIFICATION.md ("Caveats on literal wording", C2).
awaiting: user response

## Tests

### 1. Review the judgment-tier prohibition verdicts in 42-VERIFICATION.md
expected: Each MUST NOT either confirmed as not-happening (the evidence column points at code and a named passing test for every one) or redirected. These are NON-AUTHORITATIVE LLM-judge verdicts: unverified-prohibition, human review recommended. The prohibitions are descriptor-less (no `verification: test` tier), so by the ADR-550 D3/D4 soft-gate they cannot be closed by an automated verifier.
result: passed (operator confirmed all 15 verdicts, 2026-10-07T20:55:10Z)

### 2. Decide the ROADMAP SC1/SC2 and REQUIREMENTS ALLOW-03 wording for the agent loop ("last checkpoint kept ... on both engine and agent-loop runs")
expected: Agent-kind runs have no Waypoint by construction (CONTEXT D-08, WINDOWS.md row 64, `persist_agent_halt`); a halted agent-kind run resumes by a fresh `POST /v1/runs`, while the engine path continues from its Halted Waypoint. The roadmap and requirement text were never amended to say so. Either amend the wording or accept the override the verifier drafted in 42-VERIFICATION.md ("Caveats on literal wording", C2) with your name and timestamp.
result: [pending]

## Summary

total: 2
passed: 1
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
