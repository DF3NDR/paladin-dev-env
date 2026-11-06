---
created: 2026-11-06T23:40:35Z
title: "cancel_tests: two timing-dependent tests race the run's completion (local_cancel_signals_token, cross_instance_cancel_probe)"
area: run-worker
severity: warning
files:
  - src/application/services/run/cancel_tests.rs:308
  - src/application/services/run/cancel_tests.rs:366
  - .planning/phases/58-crate-release/58-CI-EVIDENCE.md
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 close-out (PR #65 CI, 2026-11-06)
---

## Problem

Two tests introduced in Phase 55 (`27d77c3ac`, TESTS-02 / D-09) depend on the `slow-chain` run still
being in flight when the test acts:

- `local_cancel_signals_token` waits until the run's local cancel token is *registered*, then calls
  `cancel()` and asserts `was_local`. Under CI load the run reached its terminal state and unregistered
  first, so `was_local` read false. Seen on `feature-flags.yml` run 42310773494 (`Build & Test (full)`,
  head `d2c74c0e3`, code tree equal to `main` at `251303bcd` where the same job passed).
- `cross_instance_cancel_probe` wraps the whole scenario in a 30 s `tokio::time::timeout`; under a full
  parallel `cargo test --workspace` on a disk-pressured machine it exceeded 30 s (Phase 58 regression gate,
  2026-11-06) and passed in isolation in 19 s.

Neither failure is a product defect: the cancel path itself is correct, the tests race it.

## What to do

Make the tests deterministic instead of widening timeouts: give the test engine factory a gate the test
holds until `cancel()` has returned (so the run cannot complete before the cancel is observed), and drop the
wall-clock bound on the cross-instance probe in favour of the same gate. Keep the D-09 rule (no fixed sleeps).
Land as a test-only code PR on `main` after `v0.11.1`; the documentation-only `chore/58-close` branch does
not carry code (D-00l).

## Evidence

- `.planning/phases/58-crate-release/58-CI-EVIDENCE.md`, `### Plan 58-14 — chore/58-close`, "Post-close CI".
- PR #65 standing-down comment (2026-11-06).
