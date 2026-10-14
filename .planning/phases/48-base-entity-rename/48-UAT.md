---
status: testing
phase: 48-base-entity-rename
source: [48-VERIFICATION.md]
started: 2026-10-14T00:24:17Z
updated: 2026-10-14T12:53:00Z
---

## Current Test

number: 1
name: CI result for HEAD f092886 (or any commit at or after 2ad9fa6): Docker Integration Tests, Coverage, pre-commit
expected: |
  Docker Integration Tests green (the 2ad9fa6 guard fix lets tests/entity_rename_guard.rs run in a
  build context with no docs/); Coverage green with --fail-under-lines 82.
awaiting: user response

## Tests

### 1. CI result for HEAD f092886 (or any commit at or after 2ad9fa6): Docker Integration Tests, Coverage, pre-commit
expected: Docker Integration Tests green (the 2ad9fa6 guard fix lets tests/entity_rename_guard.rs run in a build context with no docs/); Coverage green with --fail-under-lines 82. Context: the CI run for f092886 was still queued at verification; on dcbcb63 (run 38487208284) Coverage succeeded, Docker Integration Tests failed on the guard regression that 2ad9fa6 fixes, and ci.yml overall was cancelled. The fix was reproduced locally by plan 48-09 and by the positive control scanner_accepts_a_build_context_without_docs, but CI has not yet confirmed it. The pre-commit workflow has been red since 2026-10-12, before any Phase 48 code; its cause is unread.
result: [pending]

### 2. Run make security (cargo-audit and cargo-deny) where they are installed, and read the Security Audit and License and Dependency Policy CI job logs
expected: Pass, or a failure that is already present on the pre-phase tree. Context: neither tool is installed in the sandbox. Both CI jobs are red on dcbcb63 and were red before any Phase 48 code; Cargo.lock is byte-identical to the phase base (3cac94f). Which advisory or licence rule fires is unknown. security.instructions.md requires make security before a phase is sealed.
evidence (2026-10-14, orchestrator): cargo-audit 0.22.2 and cargo-deny 0.20.2 installed; `make security` exits 0 on 0a20e7ba (advisories/bans/licenses/sources ok, 9 allowed warnings, 0 vulnerabilities). Root cause of the red CI jobs was five RustSec advisories on quinn-proto (fixed by 1ff46a35, upgrade to 0.11.19) plus the unmaintained unsafe-libyaml advisory (governed by 12c6a5fa as register row 12). CI confirmation on 0a20e7ba pending.
result: [pending]

### 3. Measure the workspace line-coverage percentage (cargo llvm-cov --fail-under-lines 82) or read it from the CI Coverage log
expected: >= 82 %. Context: no cargo-llvm-cov, no Docker daemon and low disk in the sandbox. The only evidence is the CI Coverage job conclusion on dcbcb63 (success), not a number; the coverage-summary artifact download was denied (403).
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
