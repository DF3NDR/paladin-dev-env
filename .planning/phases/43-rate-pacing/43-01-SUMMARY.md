---
phase: 43-rate-pacing
plan: 01
subsystem: llm-pacing
tags: [cadence, rate-limit, 429, backoff, llm-decorator, treasurer-config, tracer]

requires:
  - phase: 41-admission-time-allowance-enforcement
    provides: Treasurer facade and the `treasurer` config subtree the cadence subtree joins
provides:
  - CadencePort output port, CadencePolicy (exponential back-off with full jitter) and CadenceKey/GateReading/CadenceError (paladin-ports)
  - InMemoryCadence in-process gate adapter (paladin-storage)
  - CadenceLlmAdapter decorator, CadenceSettings, CadenceWiring and with_cadence (paladin-llm), composed as Pricing(Cadence(provider))
  - treasurer.cadence config subtree (on by default), build_cadence and compose_llm (facade)
  - OpenAI adapter surfaces its first 429 instead of retrying inside its own loop
  - the end-to-end tracer test cadence_tracer_paces_a_real_openai_429_end_to_end
  - the tracer's surface on the X-10 register, 9.1/9.5 tables, CHANGELOG and the public-API baseline
affects: [43-02, 43-03, 43-04, 43-05, 43-06, 43-07, 43-08, 43-09, 43-10]

tech-stack:
  added: []
  patterns:
    - "Decorator over LlmPort that gates a call on a per-(provider, model) port and never retries (D-01)"
    - "Gate state is reactive: created only on a 429 and removed on success once elapsed (D-04)"
    - "paladin-llm depends on the CadencePort trait only, never on an adapter or on redis (D-09)"

key-files:
  created:
    - crates/paladin-ports/src/output/cadence_port.rs
    - crates/paladin-storage/src/cadence/mod.rs
    - crates/paladin-storage/src/cadence/in_memory.rs
    - crates/paladin-llm/src/cadence.rs
    - src/infrastructure/cadence.rs
  modified:
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-storage/src/lib.rs
    - crates/paladin-storage/Cargo.toml
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-llm/src/openai/adapter.rs
    - src/config/mod.rs
    - src/config/treasurer.rs
    - src/infrastructure/mod.rs
    - src/infrastructure/web/agent_host.rs
    - src/infrastructure/web/facade_provisioner.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt
    - Cargo.lock

key-decisions:
  - "Pacing is on by default (D-08): `treasurer.cadence.enabled: false` is the opt-out, recorded as M-B-05 in MIGRATION 9.1"
  - "The OpenAI adapter surfaces its first 429 (D-02) so the decorator sees every 429 and each costs exactly one provider hit"
  - "Config-loaded and runtime-provisioned agents are paced by default; engine-port and server-wide shared wiring is owned by plan 43-09"
  - "The facade public-API baseline covers only the `paladin` crate, so sub-crate symbols appear on the 9.2 register, not in current-exports.txt"

patterns-established:
  - "Tracer-first execution: a thin production-quality slice through config, composition, decorator, port, adapter and a real OpenAI adapter against an HTTP mock, proven red then green before expansion"

requirements-completed: [PACE-02]

duration: multi-session (tracer gate checkpoint between Tasks 2 and 3)
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 01: Cadence tracer Summary

**Provider 429 pacing end to end: a per-(provider, model) CadencePort gate, an in-process adapter and a CadenceLlmAdapter decorator composed as Pricing(Cadence(provider)), on by default via treasurer.cadence, with the OpenAI adapter surfacing its first 429.**

## Performance

- **Tasks:** 3 of 3 (Task 2 is the tracer; the human-verify feedback gate after it was approved)
- **Commits:** a3ef4e97, a9fef8f2, c46be0a9

## Accomplishments

- `CadencePort` and `CadencePolicy` in `paladin-ports` with saturating, clamped delay math (`CADENCE_DELAY_CEILING`) covering NaN, negative and huge inputs, plus the in-process `InMemoryCadence` in `paladin-storage`.
- `CadenceLlmAdapter` in `paladin-llm`: holds a call while its gate is closed, reports a provider 429 to the port, never retries, and logs only provider, model and durations under `paladin::cadence`.
- `treasurer.cadence` (`CadenceConfig`, `deny_unknown_fields`, `validate()` naming the offending key), `build_cadence` and `compose_llm` in the facade; config-loaded agents (`agent_host.rs`) and runtime-provisioned agents (`FacadeProvisioner`) are paced by default.
- OpenAI adapter changed to surface a 429 on the first attempt (D-02).
- The tracer `cadence_tracer_paces_a_real_openai_429_end_to_end` drives config, `build_cadence`, `compose_llm`, the decorator, the port, the in-process adapter and a real OpenAI adapter against a mockito server.
- Task 3 registered the surface: MIGRATION 9.1 row M-B-05, four 9.2 rows (all `N`), the 9.5 `treasurer.cadence` subtree (all eight keys with defaults), the CHANGELOG `[Unreleased]` PACE-02 bullet, and the refreshed `.project/current-exports.txt` (117 added lines, all cadence symbols, no removals beyond the two generated header lines).

## Red / green record (tracer)

- **Red:** the tracer was written first and run with the decorator active but before the OpenAI change; it failed with the mockito message "Expected 1 request(s) to POST /chat/completions ... but received 4" after about 7.9 s (the adapter's own loop retried the 429).
- **Bypass check:** with the OpenAI change applied and `compose_llm(.., None)` (no pacing), the tracer failed with "call 2 returned only 1.6 ms after call 1; the gate must hold it for >= 200 ms". That edit was reverted, which proves the assertion detects an absent gate.
- **Green:** with everything wired, the tracer passed in 0.25 s.

## Task Commits

1. **Task 1: CadencePort, CadencePolicy, InMemoryCadence** - `a3ef4e97`
2. **Task 2: CadenceLlmAdapter, OpenAI first-429, treasurer.cadence, build_cadence/compose_llm, tracer test** - `a9fef8f2`
3. **Task 3: register the surface (MIGRATION, CHANGELOG, API baseline)** - `c46be0a9`

## Verification

- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exited non-zero while stale, every changed line belonged to a cadence symbol, then after `make api-surface-update` it exits 0 ("API surface unchanged").
- `./scripts/check-migration-allowlist.sh` exits 0 (the four new rows are `N`, so the set is unchanged).
- Acceptance greps: 9.2 rows matching the cadence symbols: 4; 9.1 `treasurer.cadence` mentions: 1; `[Unreleased]` `PACE-02` mentions: 1.
- D-09: `cargo tree -p paladin-llm -e normal --depth 1 | grep -cE 'paladin-storage|redis'` prints 0.
- Per-crate gates (Tasks 1 and 2): `paladin-ai` lib 1266 passed, `paladin-llm` all-features 546 passed, `paladin-ports` and `paladin-storage` pass, the tracer passes, `cargo clippy -D warnings` and `cargo fmt --check` clean.
- Manual credential-handling review: no new log line or error interpolates a key, URL or header value.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[allow(clippy::too_many_arguments)]` on `build_agent`**
- **Found during:** Task 2
- **Issue:** threading the cadence wiring added an eighth parameter to `build_agent`, which `clippy -D warnings` rejects.
- **Fix:** targeted allow on that function rather than restructuring its signature mid-tracer.
- **Files modified:** `src/infrastructure/web/agent_host.rs`
- **Commit:** a9fef8f2

**2. [Rule 3 - Blocking] `tokio` `test-util` feature added to paladin-llm dev-dependencies**
- **Found during:** Task 2
- **Issue:** the decorator's paused-clock tests need `tokio::time::pause`, which requires `test-util`.
- **Fix:** enabled the feature in dev-dependencies only; no runtime dependency change. `Cargo.lock` changes were committed with their tasks (a3ef4e97, a9fef8f2).
- **Files modified:** `crates/paladin-llm/Cargo.toml`, `Cargo.lock`
- **Commit:** a9fef8f2

**3. [Plan acceptance wording] Baseline does not contain `CadencePort` or `with_cadence`**
- **Found during:** Task 3
- **Issue:** the acceptance line says `.project/current-exports.txt` contains `CadencePort` and `with_cadence`. The baseline is extracted from the `paladin` facade crate only, and the facade does not re-export either symbol, so they cannot appear there (112 other `Cadence` lines do: `CadenceConfig`, `CadenceBackend`, `build_cadence`, `compose_llm`, `TreasurerConfig.cadence`).
- **Fix:** none needed; the sub-crate symbols are covered by the 9.2 register (which the allowlist check and the plan's own greps verify). Not forced into the baseline by adding re-exports, which would widen the public surface for no reason.
- **Commit:** c46be0a9

**Total deviations:** 3 (2 Rule 3, 1 acceptance-criterion wording). **Impact:** none on behaviour.

## Intentional scope note

`paladin_port_from_settings_with_ledger` (the run-engine port in `facade_provisioner.rs`) still uses `with_pricing` only. Plan 43-09 owns engine-port and server-wide shared wiring. Until then `FacadeProvisioner::new()` builds its own in-process wiring and does not share gate state with the config-load registry, so a 429 seen by a config-loaded agent does not gate a runtime-provisioned agent on the same provider and model.

## Issues Encountered

- **Environment incident:** a workspace-wide `cargo test` hit ENOSPC (a linker bus error). `target/debug/incremental` was deleted (regenerable) and the crates were then tested individually as listed above. A full `cargo test --workspace` was therefore NOT completed in this plan; it remains to be run by the phase gate on a host with more free disk.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations T-43-01 through T-43-05 are implemented: saturating and clamped delay math, state created only on a 429, no retry in the decorator plus the OpenAI first-429 change, `deny_unknown_fields` with a named `validate()`, and log lines limited to provider, model and durations.

## Next Phase Readiness

Plans 43-02 onward extend the same decorator, port and config subtree (Retry-After, hard key cap, shared backends, fallback pacing, engine-port wiring in 43-09). The M-B-05 row, the CHANGELOG bullet and the 9.5 subtree are written to be extended by those plans.

## Self-Check: PASSED

All created files exist and commits a3ef4e97, a9fef8f2 and c46be0a9 are present in `git log`.
