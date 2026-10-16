# Phase 50 deferred items

Out-of-scope discoveries logged by executors, not fixed in the plan that found them.

## Found in plan 50-15

- **Single-feature clippy is not clean for `paladin-llm`.** `cargo clippy -p paladin-llm --lib --tests
  --no-default-features --features <one adapter>` reports warnings and, for `gemini` and `deepseek`
  alone, a hard error in the NON-test lib build: `response_guards::StreamCompletion::observe_refusal`
  is never used when only one of those two features is on (plan 50-12/50-13 code). Other warnings in
  the same builds: an unused `chat` variable, `cadence::PinnedSpread` / `pin_waiter_spread`, and
  `fallback::test_support::ProfiledStubPort`. None is in a file plan 50-15 changed; the workspace gate
  (`--workspace --all-features --all-targets -- -D warnings`) is clean. Plan 50-15's own additions
  add no warning in any of the seven single-feature builds checked (`#[allow(dead_code)]` with a
  reason on the three items that only some features construct).
