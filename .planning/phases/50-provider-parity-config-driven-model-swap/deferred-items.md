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

## Update from plan 50-16 (closeout)

- The NON-test lib half of the plan 50-15 item is fixed: `StreamCompletion::observe_refusal` now carries
  an `#[allow(dead_code)]` with its reason, so `cargo build -p paladin-llm --no-default-features
  --features gemini` and `--features deepseek` no longer warn (and `cargo clippy --lib -D warnings` is
  clean for both). The test-build warnings in the same single-feature builds (an unused `chat`
  variable, `cadence::PinnedSpread` / `pin_waiter_spread`, `fallback::test_support::ProfiledStubPort`)
  remain deferred: they exist only in `--tests` builds under one feature and the workspace gate
  (`--workspace --all-features --all-targets -- -D warnings`) is clean.
- `cargo semver-checks` for `paladin-battalion` against the published 0.10.1 with the repository's own
  lints table reports one failure, `enum_struct_variant_field_added` on `RunOutcome::Halted.cause`
  (Phase 42, ALLOW-03). It is not a Phase 50 item and is not registered here. CI's `semver` job
  compares against 0.9.0, so it does not see it; the 0.10.1 baseline is the D-27 register method.
- Plan 50-32 gate run (2026-10-19): `make security` exits 2 in `make audit`. `cargo audit` reports 3 vulnerabilities in
  `zip 3.0.0` (RUSTSEC-2026-0367, -0368, -0369: extraction symlink and SUID handling) plus the allowed unmaintained and
  unsound warnings. Plan 50-32 changes no `Cargo.toml` or `Cargo.lock`, so none of these is caused by it; the advisories
  are newly published against a transitive dependency. Not fixed here (scope boundary). Needs a dependency bump or a
  reasoned `.cargo/audit.toml` allowlist entry in a plan that owns dependencies.
