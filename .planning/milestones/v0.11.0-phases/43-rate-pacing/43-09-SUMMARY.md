---
phase: 43-rate-pacing
plan: 09
subsystem: llm-pacing
tags: [cadence, redis, config, env-overrides, composition-root, shared-wiring, run-engine, docs]

requires:
  - phase: 43-rate-pacing
    provides: CadenceConfig, build_cadence, compose_llm, CadenceWiring (43-01); AgentRuntimeDeps.cadence (43-06); RedisCadence / RedisCadenceConfig behind redis-cadence (43-07); ResilientCadence and InMemoryCadence::with_multiplier (43-08)
provides:
  - CadenceBackend::Redis { url_env } with boot-time url_env validation (empty name, unset variable), naming treasurer.cadence.backend.redis.url_env
  - impl EnvOverridable for CadenceConfig (seven scalar APP_TREASURER_CADENCE_* variables; backend has no env form), called from TreasurerConfig::apply_env_overrides
  - build_cadence redis branch: ResilientCadence(RedisCadence, InMemoryCadence x degraded_multiplier), built without connecting; a boot error naming redis-cadence and backend in_process on a binary without the feature
  - build_agent_registry_with_cadence, paladin_port_from_settings_with_cadence, build_run_api_with_cadence, FacadeProvisioner::with_cadence (the existing entry points keep their signatures and delegate)
  - the run engine's PaladinPort composed through compose_llm (paced); paladin-server builds ONE CadenceWiring and shares clones with the registry, provisioner and run API
  - operator documentation (configuration guide, HTTP service host topology), MIGRATION 9.1/9.2/9.5, CHANGELOG, refreshed API baseline
affects: [43-10, 43-11, 43-13]

tech-stack:
  added: []
  patterns:
    - "One wiring per process: the server builds a CadenceWiring once and hands clones to every composition root; each public entry point keeps its signature and delegates to a *_with_cadence variant"
    - "Config carries the NAME of the env var holding a credential-bearing URL; validate() checks the name and presence, never the value"
    - "A with_treasurer that cannot build the wiring records the reason and provision() reports it, instead of silently pacing in-process"

key-files:
  created: []
  modified:
    - src/config/treasurer.rs
    - src/infrastructure/cadence.rs
    - src/infrastructure/web/agent_host.rs
    - src/infrastructure/web/facade_provisioner.rs
    - src/infrastructure/web/run_api_wiring.rs
    - src/bin/paladin-server.rs
    - docs/src/getting-started/configuration.md
    - docs/src/deployment-topologies/http-service-host.md
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "The Redis URL never enters config, Debug, serialisation or a log line: CadenceConfig holds only the variable name, build_cadence reads the value straight into RedisCadence (redacting Debug and errors), and the boot log line names the backend kind only -- not even the variable name"
  - "A configured-but-unreachable Redis never blocks boot (RedisCadence::new is lazy); a missing feature is a boot error, never a silent in-process fallback"
  - "build_run_api builds its own wiring and so now also rejects an invalid treasurer.cadence even when run_store is disabled; the server uses build_run_api_with_cadence with the shared wiring"
  - "FacadeProvisioner::with_treasurer records a wiring build failure and provision() reports it, so a library user who never calls with_cadence cannot get silent in-process pacing from a redis config on a binary without redis-cadence"

patterns-established:
  - "Shared-state proof by control: one test shows one wiring paces two composed ports, a sibling test shows two separately built wirings do not"

requirements-completed: [PACE-02, PACE-03, PACE-05]

duration: about 1 h 30 min
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 09: Operator surface and shared server wiring Summary

**`treasurer.cadence.backend: { redis: { url_env } }` builds the resilient Redis composite without connecting, seven scalar env overrides tune the rest, and `paladin-server` now builds one wiring that paces the resident agents, the runtime provisioner and the run engine's port together.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** 998ae11e (Task 1), 56ed5be0 (Task 2)
- **Files:** 11 modified, none created
- **Redis used for one live run:** a local `redis-server` on `127.0.0.1:6380` (no persistence), shut down with `redis-cli shutdown nosave` before returning (a follow-up `ping` was refused).

## Accomplishments

- **Config (D-10).** `CadenceBackend::Redis { url_env }` (externally tagged, snake_case, `deny_unknown_fields`, so `redis: { url: ... }` or a missing `url_env` fails to load). `CadenceConfig::validate` rejects an empty or blank `url_env` and one naming an unset variable, both naming `treasurer.cadence.backend.redis.url_env` (the variable name only, never its value). `impl EnvOverridable for CadenceConfig` reads the seven `APP_TREASURER_CADENCE_*` scalars with `read_env::<bool|u64|f64>`; an unparseable value leaves the field; `backend` has no env form.
- **`build_cadence`.** For `redis`: read the URL from the named variable, build `RedisCadence::new(RedisCadenceConfig::new(url), policy)` (synchronous, no connection) and wrap it in `ResilientCadence::new(redis, InMemoryCadence::new(policy).with_multiplier(degraded_multiplier))`. Without `redis-cadence` it returns an error naming the feature and `treasurer.cadence.backend: in_process`. Module docs gained the backend table, the degraded behaviour and the fixed `paladin:cadence` namespace (43-07's flagged assumption A3).
- **Composition roots (D-08).** `build_agent_registry_with_cadence`, `paladin_port_from_settings_with_cadence`, `build_run_api_with_cadence` and `FacadeProvisioner::with_cadence` take the caller's `Option<CadenceWiring>`. The pre-existing entry points keep their signatures, build a wiring from `settings.treasurer.cadence` and delegate. The run engine's port now goes through `compose_llm` (no `with_pricing(` call remains in `facade_provisioner.rs`). `paladin-server` builds the wiring once, logs one line (`Rate pacing enabled (treasurer.cadence.backend: in_process|redis)` or `disabled`), and passes clones to all three.
- **Docs.** `configuration.md` gained "Treasurer rate pacing (Cadence)" (what it does, an example per backend, a key table with type, default and env var for all eight keys, one line each for the D-06, pace-budget, degraded-multiplier and lock-TTL semantics, the namespace caveat). `http-service-host.md` gained "Fleet-wide pacing with Redis" (build feature, env var, outage behaviour, namespace). `./scripts/check-doc-config.sh`: 153 blocks, 0 failed.
- **Register.** MIGRATION 9.1 (M-B-05 extended), two 9.2 rows (both `N`, allowlist set-equal), 9.5 (`redis` backend, the seven env vars, boot validation); CHANGELOG Phase 43 bullet extended; API baseline refreshed (7 added items, 2 header lines changed).

## Tests added

- `config::treasurer`: `redis_backend_deserializes_from_the_documented_yaml`, `redis_backend_carries_the_variable_name_never_a_url`, `validate_rejects_an_empty_url_env_naming_the_key`, `validate_rejects_an_unset_url_env_naming_the_variable` (`#[serial]`), `unknown_key_under_the_redis_backend_fails_to_load`, `each_scalar_env_override_applies`, `an_unparseable_override_leaves_the_field`, `backend_has_no_env_override`.
- `infrastructure::cadence`: `build_cadence_redis_backend_builds_without_connecting` (feature, a dead `127.0.0.1:1` URL carrying a password, then a record against the degraded fallback), `build_cadence_redis_backend_without_the_feature_names_the_feature`, `build_cadence_redis_backend_with_an_unset_variable_names_the_variable`, `one_wiring_paces_the_agent_host_and_the_run_engine_together`, `separate_wirings_do_not_share_gate_state` (the control), `build_cadence_redis_backend_shares_state_between_two_workers` (live Redis, self-skips without one).
- `facade_provisioner`: `provisioner_with_cadence_uses_the_supplied_wiring`, `paladin_port_from_settings_with_ledger_rejects_an_invalid_cadence_config_before_resolving_a_provider`, `disabled_cadence_composes_pricing_only_at_every_site`, `provisioner_without_the_feature_rejects_a_redis_backend_instead_of_pacing_in_process`.
- `run_api_wiring`: `build_run_api_rejects_an_invalid_cadence_section_naming_its_key`, `build_run_api_with_cadence_accepts_a_shared_wiring_or_none`.
- `paladin-server`: `cadence_boot_summary_names_the_backend_kind_and_never_the_url_or_variable`.

## Red / green record

- **Task 1:** tests first. The first run failed to compile (`no variant named Redis found for enum CadenceBackend`, 4 errors). After the config change `build_cadence`'s match was non-exhaustive, then fixed; all 49 `config::treasurer` tests, 9 `infrastructure::cadence` (feature on: 11 with the live Redis tests) pass.
- **Task 2:** the `*_with_cadence` implementation was written before its tests (process departure from strict test-first order), so the new tests were validated by mutation instead of a red run. Mutation: giving the run-engine port a separately built wiring (backup and copy back, no `git checkout`) made `one_wiring_paces_the_agent_host_and_the_run_engine_together` fail with "the run engine's call reached its provider only 0ns after the agent host's 429 ... at least 500ms"; the file was restored and the diff against the backup was clean.

## Verification

- `cargo test -p paladin-ai --lib config::treasurer`: 49 passed. `--lib infrastructure::cadence`: 9 passed; with `--features redis-cadence` (live Redis on 6380): 11 passed, no `SKIP:` line.
- `cargo test -p paladin-ai --features web-server --lib -- --skip build_run_api_persists`: 1346 passed, 0 failed (2 filtered out, the known pre-existing failures). `--bin paladin-server` (with `web-server,redis-cadence`): 20 passed.
- `cargo build --bin paladin-server --features web-server` and with `,redis-cadence`: both succeed.
- `cargo test -p paladin-ai --features web-server,redis-cadence --doc`: 185 passed (the new `no_run` examples compile).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo clippy -p paladin-ai --all-targets --features redis-cadence,web-server -- -D warnings`: clean. `cargo fmt --check`: clean.
- `./scripts/check-doc-config.sh` exit 0; `./scripts/check-migration-allowlist.sh` set-equal; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` failed while stale (every changed line a cadence symbol), then after `make api-surface-update`: "API surface unchanged".
- Acceptance greps: `grep -vE '^\s*//' facade_provisioner.rs | grep -c 'with_pricing('` prints 0; `paladin-server.rs` has exactly one `build_cadence(` call plus `build_run_api_with_cadence(` and `.with_cadence(`; `ResilientCadence::new(` and `redis-cadence` are in `cadence.rs`; the docs contain the required strings; the seven env names are each read in `treasurer.rs`.
- Manual credential-handling review: the URL is read from the named variable inside `build_redis_wiring` and handed to `RedisCadence` (hand-written redacting `Debug` and errors from 43-07). No error built here contains a value: validation messages carry the variable name; tests assert `hunter2` is absent from the missing-feature error and from the provisioner failure message. The boot line names the backend kind only (tested to omit the variable name and `://`). `CadenceConfig` still derives `Debug`, but holds only a name. No new log line carries a key, header or URL.

## Task Commits

1. **Task 1: redis backend, url_env validation, scalar env overrides, resilient redis wiring** - `998ae11e`
2. **Task 2: shared wiring across the three composition roots, docs, registration** - `56ed5be0`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Correctness] `FacadeProvisioner` no longer swallows a wiring build failure**
- **Found during:** Task 2
- **Issue:** the 43-01 `with_treasurer` ignored a `build_cadence` error and kept the default in-process wiring. With `backend: redis` on a binary without `redis-cadence`, a library user calling `FacadeProvisioner::from_settings` (not the server, which uses `with_cadence`) would silently get in-process pacing -- exactly the downgrade T-43-35 forbids.
- **Fix:** `with_treasurer` records the reason in a private `cadence_error`; `provision()` returns `ProvisionError::Failed("invalid treasurer configuration: ...")`; `with_cadence` clears it. Test `provisioner_without_the_feature_rejects_a_redis_backend_instead_of_pacing_in_process`.
- **Files modified:** `src/infrastructure/web/facade_provisioner.rs`
- **Commit:** 56ed5be0

**2. [Plan acceptance wording] `.project/current-exports.txt` does not contain the new `*_with_cadence` functions**
- **Issue:** the baseline is extracted with default features, and `infrastructure::web` is behind `web-server`, so `build_agent_registry_with_cadence`, `paladin_port_from_settings_with_cadence`, `build_run_api_with_cadence` and `FacadeProvisioner::with_cadence` cannot appear (the same applies to the existing `build_run_api`). Only the default-feature items changed: `CadenceBackend::Redis`, its `url_env` field and `CadenceConfig`'s `EnvOverridable` impl (7 added lines, 2 header lines).
- **Fix:** none; the functions are covered by the 9.2 register and `make api-surface` exits 0.

**3. [Behaviour note] `build_run_api` now rejects an invalid `treasurer.cadence` even when `run_store` is disabled**
- It builds its own wiring before delegating, so the section is validated up front (and `backend: redis` on a binary without the feature is a boot error). This is documented in its `# Errors` rustdoc. The server is unaffected: it validates `treasurer` at the top of `run()` already.

**4. [Test placement] `one_wiring_paces_the_agent_host_and_the_run_engine_together` lives in `src/infrastructure/cadence.rs`**
- The acceptance command runs `--lib` without `web-server`, so the test sits in the always-compiled module. It composes two ports with `compose_llm` over one wiring, which is the property; the `*_with_cadence` entry points themselves are covered hermetically by the validation and delegation tests above (they resolve a real provider, so a paced end-to-end through them is not hermetic).

**5. [Extra beyond the plan]** the `separate_wirings_do_not_share_gate_state` control test, the live `build_cadence_redis_backend_shares_state_between_two_workers` test (it matches no CI job's mandatory filter, so CI does not require it to run; it self-skips without Redis), `paladin_port_from_settings_with_cadence` re-validating `treasurer.cadence` even though the wiring is supplied, and a `cadence_boot_summary` helper in the server so the log line is unit-testable.

**Total deviations:** 5 (1 Rule 2, 1 acceptance wording, 1 behaviour note, 1 placement, 1 additive). **Impact:** none on planned behaviour.

## Authentication Gates

None.

## Issues Encountered

- None blocking. Free disk was about 5 GB; no ENOSPC and no `target/` cleanup.
- Known pre-existing failures `build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` (time out waiting for a run to reach a terminal status) were skipped with `--skip build_run_api_persists`, not touched and not run.
- `make security` was not re-run (no dependency, feature or lockfile change). A full `cargo test --workspace` was not run; the gates above are per-crate and per-feature plus the workspace clippy.

## Flagged Assumptions

- The configuration guide describes `lock_ttl_secs` as consumed by `WarEngine::with_cadence`, which arrives in plan 43-11; until then the key is validated and unused (it says so for the shipped server, which attaches no node cache).
- The fixed `paladin:cadence` namespace (A3) is now documented in both operator pages; there is still no `key_prefix` config key, by design.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-33 (name-only config, redacting `Debug` and errors, name-only boot line, absence asserted in tests), T-43-34 (lazy construction; the test builds against `127.0.0.1:1` in under a second and the first record degrades), T-43-35 (boot error naming `redis-cadence`; the provisioner now rejects rather than falling back), T-43-36 (one wiring built in `paladin-server` and shared; the shared and separate-wiring tests).

## Next Phase Readiness

43-10 adds lock methods to `CadencePort`; `ResilientCadence` and the `RedisCadence` scripts get a fail-open lock arm there, and `build_cadence` needs no change. 43-11 wires `WarEngine::with_cadence`, which will consume `lock_ttl_secs`. 43-13 records the ADR and the PROMOTION entry; the documented behaviours here (namespace, no `TraceEvent` variant, reactive-only) are ready to cite.

## Self-Check: PASSED

All modified files exist; commits 998ae11e and 56ed5be0 are present in `git log` on `claude/laughing-dirac-e0h2ax`; the Redis server on port 6380 is shut down.
