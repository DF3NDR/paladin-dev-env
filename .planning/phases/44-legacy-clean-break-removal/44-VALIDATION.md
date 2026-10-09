---
phase: 44
slug: legacy-clean-break-removal
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-09
---

# Phase 44 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in harness through `cargo test`: unit `#[test]` / `#[tokio::test]` (paused-clock tests use `start_paused = true`; tokio `test-util` is already a dev-dependency of `paladin-battalion`), rustdoc doc tests, the root integration targets `lib` (`tests/lib.rs`, filter form `integration::<module>`, e.g. `integration::battalion_campaign_integration_test` -- module paths confirmed in `tests/integration/mod.rs`) and `unit` (`tests/unit/mod.rs`), auto-discovered root guards (`tests/*.rs`, e.g. `cargo test --test legacy_removal_guard`), and shell gates (`scripts/check-*.sh`, `tests/scripts/*_test.sh`). Coverage is `cargo llvm-cov` 0.8.7 in CI's `coverage` job only (82 % workspace line floor, ADR-0006); it is not installed locally, so no local command in this phase claims a coverage figure. MSRV 1.88 is CI's `msrv` job. |
| **Config file** | `Cargo.toml` (workspace members; `[[test]]` targets `lib` and `unit`), `Makefile` (gate targets), `.github/workflows/ci.yml` (`coverage`, `msrv`, `semver` jobs). No Wave 0 install. |
| **Quick run command** | `cargo test -p paladin-battalion --lib` (every runner, Formation, Phalanx, Campaign, Commander and Conclave test lives in this crate; narrow further with a module filter, e.g. `cargo test -p paladin-battalion --lib aegis_attempt`; for the core module use `cargo test -p paladin-ai-core --lib battalion`) |
| **Full suite command** | `cargo test --workspace --no-fail-fast && cargo test --workspace --doc && make clean-code && make check-migration-allowlist && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` |
| **Estimated runtime** | Quick: **7 s measured** (2026-10-09, warm cache, no source change: 844 tests, 4.4 s test time); about **1-3 min** after an edit to `paladin-battalion` or `paladin-core` (incremental rebuild, estimate). Per-task `<automated>` gates: typically **2-6 min** -- most chain a `cargo check --workspace --all-targets --all-features`, **measured at 1 min 50 s** warm. Full suite: **25-45 min** warm (estimate; `make clean-code` alone runs fmt, clippy over all targets and features, shell lint, check and the rustdoc gate). |

---

## Sampling Rate

- **After every task commit:** run that task's own `<automated>` line (map below). Each is scoped to the crate and module the task touched; when a task edits `paladin-battalion` its floor is the quick command, and when it edits the core battalion module its floor is `cargo test -p paladin-ai-core --lib battalion`. A task that deletes a public item also runs `cargo check --workspace --all-targets --all-features` (already inside those tasks' lines).
- **After every plan wave:** `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --no-fail-fast && make check-doc-examples` (research "Sampling Rate").
- **Before `/gsd-verify-work`:** the full suite command must be green, plus the rest of the 44-12 Task 2 gate: `make security`, `make check-gates`, `make check-doc-config`, `make check-examples`, `./scripts/check-changelogs.sh`, `cargo test --test legacy_removal_guard`, `cargo test --test treasurer_vocabulary_guard`.
- **Max feedback latency:** **20 min per task commit** (the budget most tasks meet in 2-6 min; the known slow outliers are 44-08-03, whose `make check-examples` builds every example under CI's feature-split matrix, and the tasks running `make doc-check` or `make api-surface` -- 44-03-03, 44-10-01, 44-10-03 -- at roughly 5-15 min, all estimates). 44-12-02 *is* the phase gate and is budgeted with the full suite (25-45 min), not as per-task feedback.

---

## Per-Task Verification Map

The Automated Command column names each task's primary check; the exact `<automated>` line of every
task is reproduced verbatim under "Automated commands (verbatim)" below.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 44-01-01 | 01 | 1 | LEGACY-01, LEGACY-02 | T-44-01 | each Campaign attempt is bounded by `aegis.timeout`; a hung Paladin surfaces a structured timeout `NodeError` after the bound | tracer (end-to-end, paused clock) | `cargo test -p paladin-battalion --lib commander::tests::commander_campaign_honours_aegis_per_attempt_timeout_end_to_end` | ❌ new test + new `aegis_attempt.rs`, created in task | ⬜ pending |
| 44-01-02 | 01 | 1 | LEGACY-01, LEGACY-02 | T-44-02, T-44-03, T-44-04 | `validate_aegis` rejects zero bounds, `max_attempts == 0`, Route / Custom handlers and the Custom predicate; retry logs carry name, attempt, transience and delay only | unit + doc | `cargo test -p paladin-battalion --lib aegis_attempt` | ❌ new tests, created TDD-first in task | ⬜ pending |
| 44-02-01 | 02 | 1 | LEGACY-04 | T-44-05, T-44-06 | ADR-0059 licenses Phase 44 only and never cites ADR-0051 as licence; D-04 refinement and every Open Question resolution recorded | shell check | `grep` heading set + `This refines D-04` + Open Questions 1-10 in ADR-0059 | ❌ ADR created in task | ⬜ pending |
| 44-02-02 | 02 | 1 | LEGACY-04 | T-44-05 | supersession of ADR-0001 / 0002 mechanically visible | shell check | `grep -q 'Next free ADR number: 0060' .planning/decisions/PROMOTION.md` + Status checks | ✅ | ⬜ pending |
| 44-03-01 | 03 | 2 | LEGACY-03 | T-44-07, T-44-08 | breaker and Conclave decisions come from `transience()`, never provider message text; breaker counts Transient only | unit | `cargo test -p paladin-ai --lib circuit_breaker` + `cargo test -p paladin-battalion --lib conclave_execution_service` | ✅ (tests rewritten) | ⬜ pending |
| 44-03-02 | 03 | 2 | LEGACY-03 | T-44-07 | the stringly variant and both legacy predicates are gone from every `*.rs` | compile + unit + grep | `cargo check --workspace --all-targets --all-features` + `cargo test -p paladin-ai-core --lib paladin_error` | ✅ | ⬜ pending |
| 44-03-03 | 03 | 2 | LEGACY-03 | T-44-09 | rewritten rustdoc leaks nothing and holds the zero-warning bar | doc + clippy | `make doc-check` | ✅ | ⬜ pending |
| 44-04-01 | 04 | 2 | LEGACY-01, LEGACY-02 | T-44-10, T-44-11 | each Formation attempt bounded; an unsupported handler is rejected, never treated as Absorb | unit (paused clock) | `cargo test -p paladin-battalion --lib formation_service` | ✅ (new tests in existing file) | ⬜ pending |
| 44-04-02 | 04 | 2 | LEGACY-01, LEGACY-02 | T-44-10 | Commander-routed Formation tests on the Aegis contract | integration | `cargo test -p paladin-ai --test lib -- integration::commander_error_paths_test ...` | ✅ | ⬜ pending |
| 44-05-01 | 05 | 2 | LEGACY-01, LEGACY-02 | T-44-12, T-44-13, T-44-14 | retries bounded per Paladin; `AggregationError` embeds only `NodeError` displays; no panicking `unwrap` | unit (paused clock) | `cargo test -p paladin-battalion --lib phalanx_service` | ✅ (new tests in existing file) | ⬜ pending |
| 44-05-02 | 05 | 2 | LEGACY-01, LEGACY-02 | T-44-12 | root Phalanx tests on per-attempt and Absorb contracts | integration | `cargo test -p paladin-ai --test lib -- integration::battalion::phalanx_integration_test` | ✅ | ⬜ pending |
| 44-06-01 | 06 | 3 | LEGACY-01 | T-44-17 | operator confirms the one-way persisted `node_errors` shape before any code change | checkpoint:decision (manual, blocking) | none by design -- gated by 44-06-03's outcome-marker grep | n/a | ⬜ pending |
| 44-06-02 | 06 | 3 | LEGACY-01 | T-44-16 | strict typed deserialization; no untagged / alias compatibility path | compile + unit + integration | `cargo check --workspace --all-targets --all-features` + `cargo test -p paladin-herald` | ✅ | ⬜ pending |
| 44-06-03 | 06 | 3 | LEGACY-01 | T-44-15, T-44-16, T-44-17 | herald lines add no provider or status; v0.10 element shape rejected; checkpoint outcome recorded | unit + shell check | `cargo test -p paladin-ai-core --lib battalion_result` + outcome-marker `grep -Eq` on ADR-0059 | ❌ new tests, created TDD-first in task | ⬜ pending |
| 44-07-01 | 07 | 3 | LEGACY-02, LEGACY-03 | T-44-18, T-44-20 | Commander validates policy via `validate_aegis`; 300 s per-attempt builder default; `retry_attempts = max_attempts - 1` bridge | unit (paused clock) | `cargo test -p paladin-battalion --lib commander` | ✅ (tests replaced in existing file) | ⬜ pending |
| 44-07-02 | 07 | 3 | LEGACY-02, LEGACY-03 | T-44-19, T-44-20 | Conclave retry Transient-only, clamped to 5 retries, engine backoff capped by `max_interval` | unit (paused clock) + doc | `cargo test -p paladin-battalion --lib conclave_execution_service` | ✅ (tests rewritten) | ⬜ pending |
| 44-08-01 | 08 | 4 | LEGACY-04 | T-44-21, T-44-22 | CLI `timeout_seconds` maps per attempt; a zero value is rejected by validation | compile + doc-examples + unit | `cargo test -p paladin-ai --lib cli` + `make check-doc-examples` | ✅ | ⬜ pending |
| 44-08-02 | 08 | 4 | LEGACY-04 | -- | N/A (examples) | build | `cargo check -p paladin-ai --examples --all-features` | ✅ | ⬜ pending |
| 44-08-03 | 08 | 4 | LEGACY-04 | -- | N/A (examples) | build | `make check-examples` | ✅ | ⬜ pending |
| 44-09-01 | 09 | 4 | LEGACY-02, LEGACY-04 | T-44-23, T-44-24 | docs state the per-attempt bound; both fictional `battalion:` YAML blocks in `configuration.md` gone; no removed API taught | doc gates + grep | `make check-doc-config` + `make check-doc-examples` + negative `git grep` / `grep` | ✅ | ⬜ pending |
| 44-09-02 | 09 | 4 | LEGACY-03, LEGACY-04 | T-44-24 | Conclave / deployment / CLI pages teach no removed API | doc gates + grep | `make check-doc-config` + negative `git grep` | ✅ | ⬜ pending |
| 44-09-03 | 09 | 4 | LEGACY-03, LEGACY-04 | T-44-24 | PaladinError pages describe `LlmFailure` / `transience()` | doc gate + grep | `make check-doc-examples` + negative `git grep` | ✅ | ⬜ pending |
| 44-10-01 | 10 | 5 | LEGACY-01 | T-44-27 | legacy retry module and facade re-export removed; API baseline regenerated, nothing unregistered | compile + unit + API gate | `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 44-10-02 | 10 | 5 | LEGACY-01, LEGACY-02 | T-44-25 | no test (including the two root Campaign / chain-of-command tests) writes a removed field; uncommitted trial deletion proves it | integration + unit | `cargo test -p paladin-ai --test lib -- ... integration::battalion_campaign_integration_test integration::battalion_chain_of_command_integration_test` | ✅ | ⬜ pending |
| 44-10-03 | 10 | 5 | LEGACY-01, LEGACY-02, LEGACY-03 | T-44-25, T-44-26 | no shim or alias; a v0.10 config document still loads with a default `aegis` | compile + unit + doc | `cargo test -p paladin-ai-core --lib battalion` + `make doc-check` | ❌ new test, created TDD-first in task | ⬜ pending |
| 44-11-01 | 11 | 6 | LEGACY-04 | T-44-28 | every behaviour change is a 9.1 row (incl. the D-04 `max_attempts - 1` bridge in `M-B-09`) | shell check | `./scripts/check-migration-allowlist.sh` | ✅ | ⬜ pending |
| 44-11-02 | 11 | 6 | LEGACY-04 | T-44-28, T-44-SC, T-44-29 | only measured lints registered; downloaded tool isolated in the scratchpad; manifests restored byte-for-byte | script + manual diagnostic | `./scripts/check-migration-allowlist.sh` + `bash tests/scripts/check-migration-allowlist_test.sh` | ✅ | ⬜ pending |
| 44-13-01 | 13 | 6 | LEGACY-01, LEGACY-02, LEGACY-03 | T-44-33 | positive controls prove every guard rule can fire and every allowed look-alike passes | guard test (TDD) | `cargo test --test legacy_removal_guard -- scanner_` | ❌ `tests/legacy_removal_guard.rs` created in task (RED first) | ⬜ pending |
| 44-13-02 | 13 | 6 | LEGACY-01, LEGACY-02, LEGACY-03 | T-44-33, T-44-34 | repository scan clean; skip list fixed to the declared history set | guard test | `cargo test --test legacy_removal_guard` | ✅ after 44-13-01 | ⬜ pending |
| 44-12-01 | 12 | 7 | LEGACY-01..04 | T-44-32 | every change on record (CHANGELOGs, ADR-0059 `conforms`) | script + grep | `./scripts/check-changelogs.sh` | ✅ | ⬜ pending |
| 44-12-02 | 12 | 7 | LEGACY-01..04 | T-44-30, T-44-31 | full gate green; manual credential-handling review recorded | full suite | the phase gate (see verbatim line) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Sampling continuity: every wave has an automated check on every implementation task; the single
task without one is the 44-06-01 decision checkpoint, followed in the same plan by two automated
tasks, so no window of three consecutive tasks lacks automated verification.

### Automated commands (verbatim)

**44-01-01** (wave 1) -- End-to-end per-attempt Aegis timeout on Campaign through the Commander -- one path only

```bash
cargo test -p paladin-battalion --lib commander::tests::commander_campaign_honours_aegis_per_attempt_timeout_end_to_end && cargo test -p paladin-battalion --lib campaign_service && cargo test -p paladin-ai-core --lib battalion && cargo check --workspace --all-targets --all-features
```

**44-01-02** (wave 1) -- validate_aegis, the runner's retry/timeout edge matrix, and the BattalionConfig.aegis contract rustdoc

```bash
cargo test -p paladin-ai-core --lib battalion && cargo test -p paladin-ai-core --doc battalion && cargo test -p paladin-battalion --lib aegis_attempt && cargo test -p paladin-battalion --lib campaign_service && cargo clippy -p paladin-ai-core -p paladin-battalion --all-targets --all-features -- -D warnings
```

**44-02-01** (wave 1) -- Write ADR-0059 -- the Phase 44-only X-03 supersession and the replacement design

```bash
f=.planning/decisions/0059-legacy-clean-break-removal.md; test -f $f && for h in '## Status' '## Supersedes' '## Context' '## Decision' '## Considered Options' '## Code Locations' '## Code Conformance' '## Downstream Consumers'; do grep -q "^$h" $f || { echo "missing $h"; exit 1; }; done && grep -q 'Phase 44 only' $f && grep -q 'one-way' $f && grep -q 'This refines D-04' $f && for n in 1 2 3 4 5 6 7 8 9 10; do grep -Eq "Open Question $n\b" $f || { echo "missing Open Question $n"; exit 1; }; done
```

**44-02-02** (wave 1) -- Supersede ADR-0001 and ADR-0002, advance PROMOTION.md, and record ADR-0059 in PROJECT.md

```bash
grep -q 'Next free ADR number: 0060' .planning/decisions/PROMOTION.md && grep -q '^| 0059 |' .planning/decisions/PROMOTION.md && for f in .planning/decisions/0001-battalion-config.md .planning/decisions/0002-battalion-result.md; do sed -n '/^## Status/,/^## /p' $f | grep -qx 'Superseded' && sed -n '/^## Status/,/^## /p' $f | grep -q 'ADR-0059' || { echo "status not superseded in $f"; exit 1; }; done && grep -q '(ADR-0059)' .planning/PROJECT.md
```

**44-03-01** (wave 2) -- Circuit breaker and Conclave retry predicate on transience() (D-14, D-10, D-11)

```bash
cargo test -p paladin-ai --lib circuit_breaker && cargo test -p paladin-ai --test unit circuit_breaker_test && cargo test -p paladin-ai --test unit paladin_execution_service_test && cargo test -p paladin-battalion --lib conclave_execution_service
```

**44-03-02** (wave 2) -- Remove the stringly LLM variant and the two legacy predicates from PaladinError (D-09, D-13)

```bash
cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --lib paladin_error && cargo test -p paladin-battalion --lib llm_failure && cargo test -p paladin-ai --test unit paladin_error_test && test -z "$(git grep -n 'PaladinError::LlmError\|PaladinError::is_retryable\|PaladinError::is_terminal\|\.is_retryable()' -- '*.rs')"
```

**44-03-03** (wave 2) -- Re-describe the facade and eval rustdoc on the typed taxonomy and hold the rustdoc zero-warning bar

```bash
make doc-check && cargo test -p paladin-ai --lib buffered_retry_sites_trip_the_circuit_breaker_on_a_transient_failure && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**44-04-01** (wave 2) -- Formation executes every Paladin attempt through the runner and continues past failure only under aegis.on_error = Absorb

```bash
cargo test -p paladin-battalion --lib formation_service && cargo check --workspace --all-targets --all-features && cargo clippy -p paladin-battalion --all-targets --all-features -- -D warnings
```

**44-04-02** (wave 2) -- Migrate the Commander-routed and root Formation tests to the Aegis equivalents

```bash
cargo test -p paladin-battalion --lib commander && cargo test -p paladin-ai --test lib -- integration::commander_error_paths_test integration::commander_integration_tests integration::battalion::formation_integration_test integration::battalion_herald_end_to_end_test && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**44-05-01** (wave 2) -- Phalanx runs every Paladin through the runner and carries typed failures in declaration order

```bash
cargo test -p paladin-battalion --lib phalanx_service && cargo check --workspace --all-targets --all-features && cargo clippy -p paladin-battalion --all-targets --all-features -- -D warnings
```

**44-05-02** (wave 2) -- Migrate the root Phalanx integration tests to the per-attempt and Absorb contracts

```bash
cargo test -p paladin-ai --test lib -- integration::battalion::phalanx_integration_test && cargo clippy -p paladin-ai --all-targets --all-features -- -D warnings
```

**44-06-01** (wave 3) -- Confirm the persisted wire shape of a node_errors element before the one-way retype (D-06)

_Blocking `checkpoint:decision` -- no automated verify by design; the outcome is gated by 44-06-03's automated grep for the ADR-0059 outcome marker._

**44-06-02** (wave 3) -- Atomically retype node_errors across core, both producers, the three heralds and every consumer test

```bash
cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --lib battalion && cargo test -p paladin-battalion --lib -- formation_service phalanx_service && cargo test -p paladin-herald && cargo test -p paladin-ai --test lib -- integration::battalion_herald_end_to_end_test integration::commander_error_paths_test
```

**44-06-03** (wave 3) -- Pin the one-way shape and the herald lines with tests, and record the checkpoint outcome in ADR-0059

```bash
cargo test -p paladin-ai-core --lib battalion_result && cargo test -p paladin-herald && grep -Eq 'Checkpoint outcome \(44-06, [0-9]{4}-[0-9]{2}-[0-9]{2}\): (option-a|option-b|redirect)' .planning/decisions/0059-legacy-clean-break-removal.md && cargo clippy -p paladin-ai-core -p paladin-herald --all-targets --all-features -- -D warnings
```

**44-07-01** (wave 3) -- Commander without its execute() wrapper, with aegis-sourced bridges, a 300 s per-attempt builder default and validate_aegis

```bash
cargo test -p paladin-battalion --lib commander && cargo clippy -p paladin-battalion --all-targets --all-features -- -D warnings
```

**44-07-02** (wave 3) -- Conclave experts and aggregator on the runner's single attempt with the engine backoff (D-12), and Conclave::validate on validate_aegis

```bash
cargo test -p paladin-battalion --lib conclave_execution_service && cargo test -p paladin-ai-core --lib conclave && cargo test -p paladin-ai-core --doc conclave && cargo check --workspace --all-targets --all-features && cargo clippy -p paladin-battalion -p paladin-ai-core --all-targets --all-features -- -D warnings
```

**44-08-01** (wave 4) -- CLI Battalion configs and the doc-examples formation anchor on Aegis (CLI YAML keys unchanged)

```bash
cargo check -p paladin-ai --all-targets --all-features && make check-doc-examples && cargo test -p paladin-ai --lib cli
```

**44-08-02** (wave 4) -- The four Commander examples show the Aegis equivalents of every v0.10 strategy

```bash
cargo check -p paladin-ai --examples --all-features && cargo clippy -p paladin-ai --examples --all-features -- -D warnings
```

**44-08-03** (wave 4) -- The Formation, Phalanx, Campaign and Conclave examples on Aegis

```bash
cargo check -p paladin-ai --examples --all-features && cargo clippy -p paladin-ai --examples --all-features -- -D warnings && make check-examples
```

**44-09-01** (wave 4) -- Battalion guides and configuration reference on BattalionConfig.aegis

```bash
make check-doc-config && make check-doc-examples && test -z "$(git grep -n 'ContinueOnError\|RetryThenContinue\|with_error_strategy\|with_retry_policy\|with_timeout(\|BattalionError::Timeout\|default_timeout_seconds: 300' -- docs/src/user-guides/orchestration.md docs/src/user-guides/battalion-patterns.md docs/src/user-guides/fault-tolerance.md docs/src/getting-started/configuration.md docs/DEMOS.md)" && test -z "$(grep -n '^battalion:\|retry_then_continue\|APP_BATTALION' docs/src/getting-started/configuration.md)"
```

**44-09-02** (wave 4) -- Conclave, deployment and CLI pages on Aegis and the engine backoff

```bash
make check-doc-config && make check-doc-examples && test -z "$(git grep -n 'ConclaveError::Timeout\|paladin_battalion::retry\|calculate_retry_delay\|with_timeout(\|battalion::RetryPolicy' -- docs/src/appendix/conclave-pattern.md docs/src/deployment/production.md docs/src/appendix/battalion-vision-support.md docs/src/appendix/cli-usage.md)"
```

**44-09-03** (wave 4) -- The PaladinError pages on LlmFailure and transience() (D-09, D-13)

```bash
make check-doc-examples && test -z "$(git grep -n 'PaladinError::LlmError\|LlmError(String)\|is_retryable()' -- docs/src/user-guides/paladin-agents.md docs/src/user-guides/paladin-configuration.md docs/src/contributing/architecture-decisions.md docs/src/user-guides/tool-integration.md)"
```

**44-10-01** (wave 5) -- Delete the legacy retry module and its facade re-export, refresh the API baseline, and retire the aggregation references

```bash
test ! -e crates/paladin-battalion/src/retry.rs && cargo check --workspace --all-targets --all-features && cargo test -p paladin-battalion --lib error_aggregation && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface
```

**44-10-02** (wave 5) -- Sweep the last test writers of the legacy builders and fields

```bash
cargo test -p paladin-ai --test lib -- integration::battalion::load_test integration::battalion::campaign_integration_test integration::battalion_campaign_integration_test integration::battalion_chain_of_command_integration_test && cargo test -p paladin-ai --test unit -- battalion && git diff --quiet -- crates/paladin-core/src/platform/container/battalion/mod.rs crates/paladin-core/src/platform/container/battalion/conclave.rs
```

**44-10-03** (wave 5) -- Delete the legacy definitions from paladin-core

```bash
cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --lib battalion && cargo test -p paladin-ai-core --lib aegis && cargo test -p paladin-ai-core --lib node_error && cargo test -p paladin-battalion --lib maneuver && make check-doc-examples && make doc-check
```

**44-11-01** (wave 6) -- MIGRATION.md 9.1 behavioural rows, the 9.4 stored-JSON note and the 9.7 correction

```bash
for id in M-B-06 M-B-07 M-B-08 M-B-09 M-B-10 M-B-11; do grep -q "^| $id |" MIGRATION.md || { echo "missing $id"; exit 1; }; done && grep -q 'Stored `BattalionResult` JSON (Phase 44' MIGRATION.md && ./scripts/check-migration-allowlist.sh
```

**44-11-02** (wave 6) -- Measure the breaks (D-27), write the Phase 44 9.2 rows, the allowlist block and the crate allow lines

```bash
./scripts/check-migration-allowlist.sh && bash tests/scripts/check-migration-allowlist_test.sh && grep -q 'Phase 44 (Legacy Clean-Break Removal' .cargo/semver-checks-allowlist.toml && cargo check --workspace
```

**44-13-01** (wave 6) -- The legacy-removal scanner and its two positive controls

```bash
cargo test --test legacy_removal_guard -- scanner_ && cargo clippy -p paladin-ai --test legacy_removal_guard --all-features -- -D warnings
```

**44-13-02** (wave 6) -- Scan the repository, reword the last residual mention, and pin the guard

```bash
cargo test --test legacy_removal_guard && cargo test --test treasurer_vocabulary_guard && cargo clippy -p paladin-ai --test legacy_removal_guard --test lib --all-features -- -D warnings && cargo fmt --check
```

**44-12-01** (wave 7) -- CHANGELOG Removed / Changed blocks (D-21), ADR-0059 conformance and the PROJECT.md outcome

```bash
./scripts/check-changelogs.sh && grep -q 'ADR-0059' CHANGELOG.md && grep -q 'LEGACY-0' crates/paladin-core/CHANGELOG.md && grep -q 'LEGACY-0' crates/paladin-battalion/CHANGELOG.md && grep -q 'LEGACY-0' crates/paladin-herald/CHANGELOG.md && sed -n '/^## Code Conformance/,/^## /p' .planning/decisions/0059-legacy-clean-break-removal.md | grep -q 'conforms'
```

**44-12-02** (wave 7) -- Run the full phase gate and the manual credential-handling review

```bash
cargo test --workspace --doc && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --check && make security && make check-gates && make check-doc-examples && make check-doc-config && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface && ./scripts/check-changelogs.sh && cargo test --test legacy_removal_guard && cargo test --test treasurer_vocabulary_guard
```

---

## Wave 0 Requirements

None -- existing infrastructure covers all phase requirements: no framework to install (tokio
`test-util` is already a `paladin-battalion` dev-dependency), and no `<automated>` line in any plan
reads `MISSING`. New tests are written TDD-first inside the task that needs them rather than in a
separate Wave 0:

- `crates/paladin-battalion/src/aegis_attempt.rs` unit tests -- created in 44-01 (wave 1)
- new `BattalionConfig` serde / `validate_aegis` tests in `crates/paladin-core/src/platform/container/battalion/mod.rs` -- 44-01 (wave 1) and 44-10 Task 3 (wave 5)
- `tests/legacy_removal_guard.rs` -- created in 44-13 (wave 6). Research listed it as a Wave 0 gap; it
  is placed after 44-10 instead because its repository scan can only pass once every removal has
  landed. Its two positive controls are written RED first in 44-13 Task 1, so the guard is proven able
  to fail before it is trusted to pass.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Operator approval of the persisted `node_errors` element shape before the one-way retype | LEGACY-01 (D-06) | one-way persisted contract; a blocking `checkpoint:decision` (44-06 Task 1) | answer option-a, option-b or a redirect; 44-06 Task 3's automated grep then requires the dated `Checkpoint outcome (44-06, ...)` marker in ADR-0059 (g) |
| D-27 cargo-semver-checks diagnostic measures the lints behind every 9.2 row | LEGACY-04 (D-00b) | `cargo-semver-checks` 0.50.0 is not installed locally; each crate's allow lines are disabled and restored around the run | 44-11 Task 2 method; the SUMMARY's per-package table with `RESTORED_CLEAN`; `make check-migration-allowlist` is the automated half |
| Manual credential-handling review | LEGACY-01..04 (D-00d) | no Rust SAST gates a merge (security.instructions.md; CodeQL is advisory-only) | 44-12 Task 2: redaction path unchanged, no new log line interpolates a key, input or output, herald line adds only id / attempt / transience, no HTTP client added or changed |
| 82 % workspace line-coverage floor and MSRV 1.88 | D-00d | `cargo-llvm-cov` is not installed locally; CI's `coverage` and `msrv` jobs gate them | read the CI results on the PR; the closeout records them as CI-gated, not measured locally |
| ADR-0059 scope wording (Phase 44 only; ADR-0051 copied as a shape, never cited as licence) | LEGACY-04 (D-00a) | prose judgement | 44-02 Task 1 acceptance: read every `grep -n 'ADR-0051'` line of the ADR |

**Environment note (measured 2026-10-09 during this revision):** `target/` is about 20 GB and the
filesystem had 4.3 GB free after one warm workspace `cargo check`. Executors should check `df -h .`
before the per-wave suites and before 44-11's diagnostic, whose `target/semver-checks` cache can
exceed 5 GB (43-13 precedent; delete that regenerable cache if space runs low).

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or are a checkpoint (44-06-01 is the only one, gated by 44-06-03)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (there are none)
- [x] No watch-mode flags
- [ ] Feedback latency within the 20 min per-task budget (estimates except the two measured figures; confirm during execution)
- [ ] `nyquist_compliant: true` set in frontmatter (set by validate-phase)

**Approval:** pending
