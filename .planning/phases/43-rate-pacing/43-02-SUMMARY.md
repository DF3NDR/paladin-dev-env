---
phase: 43-rate-pacing
plan: 02
subsystem: llm-pacing
tags: [rate-limit, 429, retry-after, cadence, llm-error, semver, x-10-register]

requires:
  - phase: 43-rate-pacing
    provides: CadencePort, InMemoryCadence and the CadenceLlmAdapter decorator from plan 43-01
provides:
  - LlmError::RateLimitExceeded { retry_after, hints } as a #[non_exhaustive] struct variant, with rate_limited, rate_limited_with_hints, retry_after and rate_limit_hints
  - RateLimitHints, RateLimitDimension, RateLimitDimensionKind and RetryDelaySource (paladin-ports), with the effective_retry_after precedence
  - the Cadence decorator passing the provider's delay to the port as a minimum, clamping a reset-derived delay to max_backoff, and refusing a gate beyond max_wait with a typed error (D-06)
  - every in-tree LlmError::RateLimitExceeded consumer swept to the constructor and the { .. } pattern
  - the variant change registered: MIGRATION 9.2 rows, two semver lint allows plus allowlist entries, CHANGELOG
affects: [43-03, 43-04, 43-05, 43-06]

tech-stack:
  added: []
  patterns:
    - "A typed carrier on an error variant is built only through constructors and read only through accessors, so the variant can keep growing under #[non_exhaustive]"
    - "A delay is never guessed: absent or unparseable signal gives retry_after() == None"
    - "A self-generated refusal returns before the outcome is recorded, so it can never escalate the shared streak"

key-files:
  created:
    - crates/paladin-ports/src/output/rate_limit_hints.rs
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-llm/src/cadence.rs
    - crates/paladin-llm/src/error.rs
    - crates/paladin-llm/src/http_status.rs
    - crates/paladin-llm/src/mock.rs
    - crates/paladin-llm/src/pricing.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-battalion/src/commander.rs
    - crates/paladin-battalion/src/conclave_execution_service.rs
    - crates/paladin-battalion/src/llm_decision.rs
    - crates/paladin-battalion/src/llm_failure.rs
    - crates/paladin-eval/src/scenario.rs
    - src/infrastructure/cadence.rs
    - tests/cli/error_handling_test.rs
    - tests/cli/formation_execution_test.rs
    - tests/cli/paladin_execution_test.rs
    - tests/integration/openai_content_analysis_integration_test.rs
    - tests/integration/paladin_integration_test.rs
    - tests/unit/llm/anthropic_adapter_test.rs
    - tests/unit/llm/deepseek_adapter_test.rs
    - tests/unit/mock_llm_adapter_test.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .cargo/semver-checks-allowlist.toml

key-decisions:
  - "An empty RateLimitHints is not stored on the error: rate_limited_with_hints keeps hints None when the parse recorded nothing, so rate_limit_hints().is_some() means a header snapshot really exists"
  - "minimum_delay clamps only a delay whose effective source is ResetHeader; an explicit Retry-After or retry-after-ms value is passed to the port unreduced (jitter can only add)"
  - "The D-06 cap is checked on every gate reading, so a gate a concurrent 429 stretches past max_wait mid-wait is surfaced too; the refusal returns before note_outcome and records nothing"
  - "Two new crate-wide semver allows in paladin-ports (enum_unit_variant_changed_kind, enum_variant_marked_non_exhaustive) because the D-27 diagnostic showed the existing allows do not cover this variant"

patterns-established:
  - "Register reconciliation measures first (D-27 diagnostic, manifest restored byte-for-byte), then edits Cargo.toml, allowlist and MIGRATION in one commit"

requirements-completed: [PACE-01, PACE-02]

duration: about 35 min of execution (three commits between 14:16 and 14:24 UTC, plus verification)
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 02: Typed 429 retry delay and the Cadence minimum-delay and max_wait rules Summary

**LlmError::RateLimitExceeded now carries the provider's own retry delay and an optional header snapshot, and the Cadence honours that delay as a floor, bounds a reset-derived estimate to max_backoff, and surfaces any gate longer than max_wait as an immediate typed error.**

## Performance

- **Tasks:** 3 of 3
- **Commits:** 8890623a, 62224eb0, 8822d818
- **Files:** 1 created, 34 modified (30 sources and tests, 4 registers)

## Accomplishments

- `RateLimitHints` family in `paladin-ports`: private fields, parsed integers and durations only (no raw header string exists to render through `Debug`), `effective_retry_after()` implementing the plan's precedence (explicit delay; else largest reset among exhausted dimensions; else smallest reset when no dimension reports `remaining`; else `None`).
- `LlmError::RateLimitExceeded { retry_after, hints }` marked variant-level `#[non_exhaustive]`; `Display` stays exactly `Rate limit exceeded`, `transience()` stays `Transient`; `retry_after()` and `rate_limit_hints()` look through `AllProvidersFailed` to its last error.
- Workspace sweep: 29 files, value sites became `LlmError::rate_limited(None)`, pattern sites became `LlmError::RateLimitExceeded { .. }`, `LlmProviderError::RateLimitExceeded` and `LlmErrorKind::RateLimit` map through `rate_limited(None)`. No test assertion on the 429 text changed.
- Cadence decorator: private `minimum_delay` feeds `record_rate_limited`; `wait_for_gate` returns `Result<u32, LlmError>` and refuses a reading above `max_wait` with `LlmError::rate_limited(Some(wait))` before any provider call or port write.
- Registered: MIGRATION 9.2 rows (variant change `Y`, new module N/A), two crate-wide semver allows with matching allowlist entries, CHANGELOG Added and Changed entries.

## Red / green record

- **Task 1:** behavior tests written first; `cargo test -p paladin-ports --lib llm_port` failed to compile with 26 errors (no `rate_limited`, `retry_after`, ...). Green after the reshape: 28 llm_port tests, 7 rate_limit_hints tests.
- **Task 2:** six behavior tests added first; five failed against the 43-01 decorator (`explicit_retry_after_is_honoured_as_a_minimum`, `an_explicit_delay_above_max_backoff_is_never_reduced`, `gate_of_exactly_max_wait_is_waited_out`, `gate_one_ms_beyond_max_wait_surfaces_immediately`, `surfaced_refusal_is_not_recorded_as_a_new_429`). The reset-clamp and unchanged-error tests passed trivially before the change, so the clamp test was tightened to assert the wait equals `max_backoff` exactly (the estimate is used, only bounded). All 15 cadence tests pass after.

## Task Commits

1. **Task 1: reshape RateLimitExceeded, add RateLimitHints, sweep the workspace** - `8890623a`
2. **Task 2: delay as a minimum, D-06 max_wait cap** - `62224eb0`
3. **Task 3: register the variant change (MIGRATION, semver allows and allowlist, CHANGELOG)** - `8822d818`

## D-27 diagnostic (cargo-semver-checks 0.50.0 vs 0.10.1, `--release-type minor`, crate-wide allows disabled)

Lints fired for `paladin-ports` (restored byte-for-byte, `git diff --quiet` clean before the real edit):

| Lint | Subject | Covered by an existing allow? |
|------|---------|-------------------------------|
| `enum_unit_variant_changed_kind` | `LlmError::RateLimitExceeded` | No -> added |
| `enum_variant_marked_non_exhaustive` | `LlmError::RateLimitExceeded` | No -> added |
| `constructible_struct_adds_field` | `SubmitRun.attributed_to`, `LlmResponse.cost`, `RunQuery.scope`, `RunOutcomeRecord.halt_reason`, `CreateRunSchedule.created_by` | Yes (pre-existing, not this plan) |

Nothing fired for the `RateLimitHints` family (new items) or the new `LlmError` methods. After the edit: the same run against 0.10.1 reports 191 pass, 0 fail; the CI-parity run against the pinned 0.9.0 baseline reports 0 checks, 254 skip (the already-documented no-op noted in the Phase 38 decisions). `./scripts/check-migration-allowlist.sh` exits 0.

The tool was not installed in this sandbox. It was fetched as the prebuilt `v0.50.0` release binary (the version and source CI's `semver` job pins via `taiki-e/install-action`) into the session scratchpad, not through a package manager, and was not added to the repository.

## Verification

- `cargo check --workspace --all-targets --all-features`: exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean (after Task 1); `cargo clippy -p paladin-llm --all-targets --all-features -- -D warnings` clean after Task 2; `cargo fmt --check` clean.
- `cargo test --workspace --no-fail-fast -- --skip build_run_api_persists` (default features): exit 0, no failing test.
- `cargo test -p paladin-ports --lib` (223 passed), `--doc` (175 passed), `cargo test -p paladin-llm --all-features` (546 passed), `paladin-battalion` (836 passed), `paladin-eval`, `paladin-ai --lib cadence` (11 passed, including the 43-01 tracer's neighbours).
- `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-llm -p paladin-ports --no-deps --all-features`: clean.
- Acceptance grep for a unit-shaped use of the variant outside comments and intra-doc links: 0.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: exit 0, "API surface unchanged" (see Deviation 2).
- `make check-changelogs`: all 11 crates covered.
- Manual credential-handling review: no new log line interpolates a key, URL or header value; the D-06 debug line names provider, model and two durations only; `RateLimitHints` has no string field.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `src/infrastructure/cadence.rs` updated although not in `files_modified`**
- **Found during:** Task 1 (`cargo check --workspace --all-targets --all-features`)
- **Issue:** 43-01's tracer test asserts `matches!(first, Err(LlmError::RateLimitExceeded))`, a unit-shaped pattern that no longer compiles.
- **Fix:** pattern became `LlmError::RateLimitExceeded { .. }`. Same one-pattern update as the 29 listed files; the plan's grep list said "confirmed by `cargo check`", which is how this site was found.
- **Files modified:** `src/infrastructure/cadence.rs`
- **Commit:** 8890623a

**2. [Plan acceptance wording] `.project/current-exports.txt` unchanged**
- **Found during:** Task 3
- **Issue:** the acceptance line says the baseline contains `rate_limited_with_hints`. The baseline is extracted from the `paladin` facade crate only and lists `LlmError` as the single line `pub use paladin::prelude::LlmError` with no members, so no `paladin-ports` method can appear. This is the same situation recorded as Deviation 3 of plan 43-01.
- **Fix:** none. `make api-surface` exits 0 with "API surface unchanged", and the new symbols are covered by the 9.2 register (which `check-migration-allowlist.sh` and the plan's greps verify). The file was not touched, and no re-export was added just to satisfy the grep.
- **Commit:** 8822d818 (no baseline change)

**3. [Rule 2 - Missing critical functionality] Two semver lint allows added (the plan's conditional branch)**
- **Found during:** Task 3
- **Issue:** the D-27 diagnostic fired `enum_unit_variant_changed_kind` and `enum_variant_marked_non_exhaustive`, neither covered by `paladin-ports`' existing crate-wide allows.
- **Fix:** added both under `[package.metadata.cargo-semver-checks.lints]` with a PACE-01 comment, two matching `[[entry]]` blocks (row `paladin-ports | LlmError`), and marked the 9.2 row `Y`. Allowlist and register stay set-equal.
- **Files modified:** `crates/paladin-ports/Cargo.toml`, `.cargo/semver-checks-allowlist.toml`, `MIGRATION.md`
- **Commit:** 8822d818

**4. [Rule 3 - Blocking] Panic message text in `error.rs`**
- **Found during:** Task 1 acceptance
- **Issue:** a test's `panic!("expected LlmError::RateLimitExceeded, got ...")` string literal matched the acceptance grep for a unit-shaped use.
- **Fix:** reworded to "expected a rate-limit error, got ...". Behaviour unchanged.
- **Commit:** 8890623a

**Total deviations:** 4 (2 Rule 3, 1 Rule 2, 1 acceptance-criterion wording). **Impact:** none on behaviour.

## Deferred Issues

- `docs/src/appendix/provider-expansion.md` (lines 475, 513) and `docs/src/contributing/contributing-providers.md` (line 472) show an illustrative `RateLimitExceeded { retry_after }` / `{ retry_after: 60 }` shape that never matched the real type. They are prose, not compiled, and predate this plan; they now partly resemble the real variant but are still wrong (`retry_after` is `Option<Duration>` and there is a `hints` field). Left for the documentation pass in 43-13.
- A full `cargo test --workspace --all-features` (including `--features web-server`) was not run; only default-feature workspace tests and `--all-features` check, clippy and the `paladin-llm` suite were. The two known `build_run_api_persists_*` web-server timeouts were skipped by name as instructed and did not appear.

## Issues Encountered

- **Environment:** free disk fell to about 5.8 GB during the semver diagnostic. `target/debug/incremental` (regenerable, about 5.9 GB) was deleted before the workspace test and the run used `CARGO_INCREMENTAL=0`; no ENOSPC occurred.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-06 (D-06 refusal at exactly `max_wait` + 1 ms, tests at the boundary, delays already clamped to `CADENCE_DELAY_CEILING` by the port), T-43-07 (`RateLimitHints` stores parsed integers and durations only; `debug_renders_numbers_only` asserts no string payload), T-43-08 (`surfaced_refusal_is_not_recorded_as_a_new_429`).

## Next Phase Readiness

43-03 can fill `RateLimitHints` from real response headers and add `map_http_status_with_hints`; every adapter's 429 arm currently builds `LlmError::rate_limited(None)` and is the single place it will change. 43-04 adds the conformance case against this carrier.

## Self-Check: PASSED

All created and modified files exist, and commits 8890623a, 62224eb0 and 8822d818 are present in `git log` on `claude/laughing-dirac-e0h2ax`.
