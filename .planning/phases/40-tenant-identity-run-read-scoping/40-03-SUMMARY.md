---
phase: 40-tenant-identity-run-read-scoping
plan: 03
subsystem: auth
tags: [tenant-scoping, config-validation, fail-closed, api-keys, bearer-token, operator-docs]

# Dependency graph
requires:
  - phase: 40-tenant-identity-run-read-scoping
    provides: "40-01: ApiKeyConfig.tenant, BearerTokenAuthConfig.tenant, build_auth_config tenant parsing and its fail-closed messages, TenantId/TenantIdError, Principal::read_scope"
provides:
  - "AuthConfig::validate() -> Result<(), String> -- one fail-closed check over every api_keys entry (empty key, invalid/duplicate name, missing/invalid tenant, duplicate key value) and the bearer tenant rule, never clamping or rewriting a value"
  - "build_auth_config calls cfg.validate() first, so paladin-server refuses to boot on the first offending entry, named by `name`/index, never by key value"
  - "Operator documentation of the key-to-tenant mapping, bearer_token.tenant, the tenant-scoped run read rule with the Admin bypass, the 404-for-a-foreign-run rule and the open-access tenant (http-service-host.md)"
  - "D-06 onboarding clause resolved against the shipped tree: the paladin-cli onboarding template emits only LLM provider env vars, never an http.auth block, so there is no emitter to extend"
affects: [40-04, 40-05, 40-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Config validate() reports the first offending entry in declaration order, addressing it as api_keys[<name>] once the name is known to be a usable identifier and api_keys[#<index>] before that"
    - "Secret-adjacent duplicate detection: the seen-values map records key value -> first name so the error can name both entries without ever printing the value"

key-files:
  created: []
  modified:
    - src/config/agents.rs
    - src/bin/paladin-server.rs
    - docs/src/deployment-topologies/http-service-host.md

key-decisions:
  - "validate() runs even when http.auth.enabled is false: a disabled section's keys take effect the moment auth is re-enabled, so a malformed mapping is never accepted (an 11th test, validate_checks_api_keys_even_when_auth_is_disabled, pins it)"
  - "Index addressing in messages is 0-based (api_keys[#0]) to match YAML/JSON list positions; used only for the two checks (empty key, invalid name) that run before the name is known to be printable"
  - "The plan's Task 2 negated grep (`! grep -rqE '^\\s*api_keys\\s*:' src/application/cli`) is a false positive on a Rust parameter name (env.rs:22 `api_keys: &HashMap<K, String>`); no CLI code was renamed to satisfy it -- the intent was verified with a refined grep and recorded here"

patterns-established:
  - "Pattern: boot-time config comparisons use plain `==`, not ct_eq -- the constant-time discipline belongs to the request path (lookup_api_key), and the rustdoc says so at the comparison site"

requirements-completed: [TENANT-01]

coverage:
  - id: D1
    description: "AuthConfig::validate() rejects an empty, whitespace-bearing, or non-printable-ASCII tenant with the exact 'tenant' is required / 'tenant' is not a valid tenant id messages, naming the key and never its value, trimming nothing"
    requirement: "TENANT-01"
    verification:
      - kind: unit
        ref: "src/config/agents.rs#validate_rejects_an_api_key_without_a_tenant"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs#validate_rejects_a_whitespace_or_non_printable_tenant"
        status: pass
    human_judgment: false
  - id: D2
    description: "Duplicate key names and duplicate key values are rejected at boot; the duplicate-key message names both keys by name and never contains the shared secret (T-40-12, T-40-13); an empty key value is rejected (T-40-14); an invalid name is rejected under the tenant-id identifier rules"
    requirement: "TENANT-01"
    verification:
      - kind: unit
        ref: "src/config/agents.rs#validate_rejects_a_duplicate_key_name"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs#validate_rejects_a_duplicate_key_value_without_printing_it"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs#validate_rejects_an_empty_key_value"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs#validate_rejects_an_invalid_key_name"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#build_auth_config_rejects_duplicate_api_key_names_at_boot"
        status: pass
    human_judgment: false
  - id: D3
    description: "Two keys with different names mapped to one tenant validate and boot into two principals with equal tenant_id and read_scope() and different id (D-06 adjacency)"
    requirement: "TENANT-01"
    verification:
      - kind: unit
        ref: "src/config/agents.rs#validate_accepts_two_keys_mapped_to_the_same_tenant"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#build_auth_config_maps_two_keys_of_one_tenant_to_one_read_scope"
        status: pass
    human_judgment: false
  - id: D4
    description: "bearer_token.tenant is required (None or empty rejected, invalid rejected) when bearer_token.enabled is true and ignored when it is false (D-03); AuthConfig::default() validates; the API keys are validated even when auth is disabled"
    requirement: "TENANT-01"
    verification:
      - kind: unit
        ref: "src/config/agents.rs#validate_requires_a_bearer_tenant_when_bearer_is_enabled"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs#validate_ignores_the_bearer_tenant_when_bearer_is_disabled"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs#default_auth_config_validates"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs#validate_checks_api_keys_even_when_auth_is_disabled"
        status: pass
      - kind: unit
        ref: "src/config/agents.rs - config::agents::AuthConfig::validate (doctest)"
        status: pass
    human_judgment: false
  - id: D5
    description: "http-service-host.md shows tenant on the example key and documents the required mapping, uniqueness rules, bearer_token.tenant, server-derived tenant, user/admin read scope, the 404 for a foreign run and the open-access tenant"
    requirement: "TENANT-01"
    verification:
      - kind: other
        ref: "grep -q 'tenant: \"platform-ops\"' && grep -q open-access && grep -q bearer_token.tenant && grep -q 404 docs/src/deployment-topologies/http-service-host.md (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: ~35m
completed: 2026-09-28
status: complete
---

# Phase 40 Plan 03: Fail-Closed Key-to-Tenant Mapping Summary

**`AuthConfig::validate()` now rejects every malformed, ambiguous or tenantless API-key entry at boot -- empty key values, invalid or duplicate names, missing or invalid tenants, duplicate key values (naming both keys by `name`, never the secret) and a missing bearer tenant -- and `paladin-server` calls it before anything else, while the HTTP service host page tells operators what every key needs and what each role can read.**

## Performance

- **Duration:** ~35m
- **Completed:** 2026-09-28
- **Tasks:** 2 (Task 1 `tdd="true"`: one RED and one GREEN commit; Task 2: docs)
- **Files modified:** 3

## Accomplishments

- `AuthConfig::validate(&self) -> Result<(), String>` in `src/config/agents.rs` (house shape per D-00d): never clamps, trims or rewrites; fails on the first offending entry in declaration order; addresses an entry as `http.auth.api_keys[<name>]` once the name is a usable identifier and `http.auth.api_keys[#<index>]` (0-based) before that; validates the keys even when `enabled` is `false`
- Per-entry checks, in order: empty `key`; `name` failing `TenantId::new` (the API key id on every run and ledger row, D-06); empty `tenant` (`'tenant' is required — every API key must map to a tenant (Phase 40, TENANT-01)`); `tenant` failing `TenantId::new`; a repeated `name`; a repeated `key` value (`duplicate 'key' — the same secret is already configured for api key '<first>' (the value is never printed)`). Then, only when `bearer_token.enabled`, a missing/empty or invalid `bearer_token.tenant`
- `AuthConfig` keeps exactly its three fields (`enabled`, `api_keys`, `bearer_token`) -- no tenant registry (D-07)
- `build_auth_config` in `src/bin/paladin-server.rs` calls `cfg.validate()?` as its first statement, before the `enabled` branch; 40-01's per-key/bearer parsing stays fallible (`map_err`/`?`, no panicking calls) even though it can no longer fail on a validated config
- `ApiKeyConfig` rustdoc rewritten: `key` non-empty and unique, `name` is the API key id recorded on every run and ledger row with the same identifier rules as a tenant id
- Eleven `agents.rs` tests (the ten the plan names plus `validate_checks_api_keys_even_when_auth_is_disabled`), one doctest, and two server tests (`build_auth_config_rejects_duplicate_api_key_names_at_boot`, `build_auth_config_maps_two_keys_of_one_tenant_to_one_read_scope`)
- `docs/src/deployment-topologies/http-service-host.md`: `tenant: "platform-ops"` on the example key; a "Tenants and what a key can read" paragraph covering the required mapping, no implicit default tenant, uniqueness of names and values, `bearer_token.tenant`, the server-derived tenant, attribution, the user/admin read scope over `GET /runs` and every `/runs/{run_id}*` route, the `404`-not-`403` rule and the `open-access` tenant when auth is disabled; the multi-replica bearer paragraph kept verbatim
- D-06's onboarding clause closed against the shipped tree (D-00g): `src/application/cli/templates/env.rs` writes only `OPENAI_API_KEY`/`ANTHROPIC_API_KEY`/`DEEPSEEK_API_KEY` lines plus commented optional-service URLs into `.env`, and `onboarding.rs::write_env_file` feeds it provider keys only -- no CLI code emits an `http.auth` or `api_keys:` YAML block, so there is no emitter to extend

## Task Commits

1. **Task 1 (RED)** — `feb550fe` test(40-03): add failing AuthConfig::validate and duplicate-key boot tests
2. **Task 1 (GREEN)** — `1e573c84` feat(40-03): validate the API key-to-tenant mapping fail-closed at boot
3. **Task 2** — `df085b9d` docs(40-03): document the API key-to-tenant mapping and tenant-scoped run reads

## Files Created/Modified

- `src/config/agents.rs` — `use std::collections::{HashMap, HashSet}`, `TenantId` import, `impl AuthConfig { pub fn validate }` with rustdoc and doctest, `ApiKeyConfig` rustdoc, eleven new tests and three test helpers
- `src/bin/paladin-server.rs` — `cfg.validate()?` first in `build_auth_config`, rustdoc updated, `RunReadScope` test import, two new tests
- `docs/src/deployment-topologies/http-service-host.md` — example key gains `tenant`, new tenant paragraph in "Authentication & authorization"

## Decisions Made

- `validate()` checks the API keys regardless of `enabled` (plan action text); pinned by an extra test beyond the plan's ten.
- Message index addressing is 0-based (`api_keys[#0]`), matching YAML/JSON list positions, and is used only where the name is not yet known to be printable.
- Boot-time duplicate detection compares key values with plain `==` via `HashMap<&str, &str>` (value -> first name); the constant-time discipline stays on the request path (`lookup_api_key`/`ct_eq`), and the code comment says so.
- The plan's Task 2 negated grep was not made to pass by renaming CLI code (see Deviations).

## Deviations from Plan

### Recorded, not fixed

**1. [Plan acceptance check] Task 2's final negated grep is a false positive**
- **Found during:** Task 2 verification
- **Issue:** `! grep -rqE '^\s*api_keys\s*:' src/application/cli` exits non-zero because `src/application/cli/templates/env.rs:22` reads `api_keys: &HashMap<K, String>,` -- a Rust function parameter, not a YAML list. The intent ("no CLI template emits a YAML API-key list") holds: `grep -rnE '"[^"]*api_keys\s*:|http\.auth|X-API-Key' src/application/cli` returns nothing, and reading `EnvTemplate::generate` and `write_env_file` confirms only provider env vars are written.
- **Disposition:** not fixed. Renaming a private parameter in a file outside the plan's `files_modified` purely to satisfy a regex would be change-for-the-check. The finding the grep was meant to record (no emitter to extend for D-06's onboarding clause) is recorded above instead.
- **Files modified:** none

### Notes on the TDD run

- In the RED run, `build_auth_config_maps_two_keys_of_one_tenant_to_one_read_scope` already passed: it is an adjacency guard over behaviour 40-01 landed (tenant parsing into `Principal`), not a new-behaviour test. The genuinely red tests were the ten `agents.rs` tests (compile error: no `validate`) and `build_auth_config_rejects_duplicate_api_key_names_at_boot` (panicked on `Ok`). GREEN turned all of them green without touching the adjacency test.
- One test beyond the plan's list was added (`validate_checks_api_keys_even_when_auth_is_disabled`) to pin a behaviour the plan's action text requires.

### Doc-vs-tree note

- The new doc paragraph states the read rule for **every** `/runs/{run_id}*` route, as the plan's action text directs. In the tree at this commit, `load_visible_run` gates `GET /runs/{id}` and `POST /runs/{id}/cancel` (40-01) and `GET /runs` is scoped (40-02); `/stream` and `/webhook-deliveries` join the gate in 40-05 of this same phase. 40-05 should not need to touch this paragraph.

## Issues Encountered

- None. Disk went from 13 GB free to 11 GB free (the `--features web-server --all-targets` clippy build); no `cargo clean`, clippy run once. No Docker-gated tests are involved in this plan.

## Known Stubs

None -- no placeholders, skipped tests or TODOs were introduced.

## Threat Flags

None beyond the plan's own `<threat_model>`: T-40-12 (validation text never carries a key value -- `validate_rejects_a_duplicate_key_value_without_printing_it` and every other `validate_*` test assert the value is absent), T-40-13 (duplicate key values rejected at boot) and T-40-14 (empty key values rejected at boot) are all mitigated as specified.

## User Setup Required

None. Operators upgrading with existing `http.auth.api_keys` entries must add `tenant` to each and ensure names and key values are unique -- the MIGRATION.md 9.5 row written in 40-01 (to be extended with the duplicate-rejection clause in 40-06) is the mitigation.

## Next Phase Readiness

- 40-04 (ledger scope source) is unaffected by this plan.
- 40-05 (`load_visible_run` on stream/webhook-deliveries, `RunResponse.submitted_by`) may cite the new doc paragraph as already describing its routes.
- 40-06 must extend the MIGRATION.md 9.5 row with the duplicate-`name`/duplicate-`key` and empty-`key`/invalid-`name` rejections this plan introduced (the plan's own artifact table assigns that extension to 40-06), and should note that the onboarding template needed no change.
- `make api-surface` will report drift until 40-06 refreshes the baseline (expected; 39-08 precedent). `AuthConfig::validate` is a new public method on the facade crate and belongs in that refresh and its CHANGELOG entry.

---
*Phase: 40-tenant-identity-run-read-scoping*
*Completed: 2026-09-28*

## Self-Check: PASSED

All three modified files and this SUMMARY exist on disk; all three task commit hashes (`feb550fe`, `1e573c84`, `df085b9d`) are present in `git log --oneline --all`.
