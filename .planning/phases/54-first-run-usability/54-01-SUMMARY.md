---
phase: 54-first-run-usability
plan: 01
subsystem: config
tags: [auth, api-keys, config-validation, first-run, security]

requires:
  - phase: 53-tenant-isolation-closure
    provides: AuthConfig::validate reserved-literal checks and the M-B-28 MIGRATION row this plan extends
provides:
  - AuthConfig::validate refuses any http.auth.api_keys[].key containing "${", naming the entry and never the value or variable
  - config.example.yml ships api_keys [] with commented key "" entries and states which fields expand
  - MIGRATION 11.1 row M-B-29, 11.5 bullet, 11.8 step 4 and a CHANGELOG [Unreleased] Security bullet
affects: [54-02 docker docs, 54-06 k8s secret config, 54-09 evidence and manual credential review]

tech-stack:
  added: []
  patterns:
    - "Refusal messages interpolate only the entry name; every test asserts the value and variable name are absent"
    - "Shipped-example tests read config.example.yml through a wrapper struct over one section"

key-files:
  created: []
  modified:
    - src/config/agents.rs
    - src/bin/paladin-server.rs
    - config.example.yml
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "Substring check on '${' (not the LLM exact-placeholder grammar): refuses exact, embedded and unterminated shapes; a bare '$' stays valid"
  - "Check runs regardless of http.auth.enabled and before the tenant and duplicate checks, so the placeholder is always the reported cause"
  - "The uncomment-unedited test rewrites the example's own text in memory, so the file is the fixture"

patterns-established:
  - "FIRST-01: a secret field that the server does not expand refuses the expansion syntax instead of loading it as a literal"

requirements-completed: [FIRST-01]

coverage:
  - id: D1
    description: "A key containing ${ in any shape is refused at config load with a message naming the entry only, with auth enabled or disabled"
    requirement: FIRST-01
    verification:
      - kind: unit
        ref: "cargo test --lib config::agents:: (validate_refuses_an_unexpanded_placeholder_key, validate_accepts_a_bare_dollar_in_a_key, validate_refuses_a_placeholder_key_while_auth_is_disabled, validate_names_a_later_entry_whose_key_is_a_placeholder, validate_reports_a_placeholder_key_before_tenant_and_duplicate_checks)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Boot path refuses a placeholder key in build_auth_config, which runs before TcpListener::bind"
    requirement: FIRST-01
    verification:
      - kind: unit
        ref: "cargo test --features web-server --bin paladin-server build_auth_config (build_auth_config_refuses_an_unexpanded_placeholder_key)"
        status: pass
    human_judgment: false
  - id: D3
    description: "config.example.yml configures no key, validates, and its entries uncommented unedited are refused as empty"
    requirement: FIRST-01
    verification:
      - kind: unit
        ref: "cargo test --lib config::agents::tests::shipped_example"
        status: pass
    human_judgment: false
  - id: D4
    description: "MIGRATION M-B-29 / 11.5 / 11.8 and CHANGELOG record the refusal; the semver allowlist gate still passes"
    requirement: FIRST-01
    verification:
      - kind: other
        ref: "make check-migration-allowlist"
        status: pass
    human_judgment: false
---

# Phase 54 Plan 01: Refuse an unexpanded placeholder HTTP API key Summary

**`AuthConfig::validate` now refuses any `http.auth.api_keys[].key` containing `${`, naming the entry and never the value; the shipped example carries no loadable key and says exactly which fields expand.**

## Performance

- **Duration:** about 25 min
- **Started:** 2026-11-01T00:00Z (approx.)
- **Completed:** 2026-11-01T00:23Z
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files modified:** 5
- **Plan-start SHA:** `75f1606650de47e9476ed1c6aedd22d2c7d3d450`

## Accomplishments

- Closed the B1 / DF-01 trap: a key copied from the v0.11.0 example (`"${PALADIN_API_KEY_CI}"`) used to load the placeholder text as a working credential. It now stops config load, and so boot, before any listener binds.
- The refusal message is `http.auth.api_keys[<name>]: 'key' is an unexpanded '${…}' placeholder -- the server does not expand environment variables in http.auth.api_keys; set the literal key value`. It never contains the key or a variable name; every new refusal test asserts both absences.
- `config.example.yml` keeps `http.auth.enabled: true` with `api_keys: []`; a verbatim copy fails closed with the existing "no credentials configured" error. The header and the LLM comment now scope `${NAME}` indirection to `llm.<provider>.api_key`.
- MIGRATION 11.1 `M-B-29`, a 11.5 bullet and 11.8 step 4 (rotate any key that was a placeholder) plus a CHANGELOG `[Unreleased]` Security bullet record the change, including the Docker baked-config impact (PD-01-3).

## Prove-then-lock (D-00f)

- Task 1 step 1: `grep -n 'unexpanded' src/config/agents.rs` printed nothing (exit 1) at the plan-start SHA, so the refusal did not exist.
- Task 2 step 1: `grep -nE '(^\s*|[{,]\s*|-\s+)key:\s*"?\$\{' config.example.yml` hit exactly two lines, 182 (`ci-runner`, `${PALADIN_API_KEY_CI}`) and 186 (`frontend`, `${PALADIN_API_KEY_APP}`). After the edit the same grep finds nothing.

## Watched failing first (RED)

- `cargo test --lib config::agents::` before the validator branch: 4 failed, 30 passed. Failing: `validate_refuses_an_unexpanded_placeholder_key`, `validate_refuses_a_placeholder_key_while_auth_is_disabled`, `validate_names_a_later_entry_whose_key_is_a_placeholder`, `validate_reports_a_placeholder_key_before_tenant_and_duplicate_checks`. `validate_accepts_a_bare_dollar_in_a_key` passed already, as the plan anticipated.
- `cargo test --features web-server --bin paladin-server build_auth_config_refuses`: `build_auth_config_refuses_an_unexpanded_placeholder_key` failed.
- `cargo test --lib config::agents::tests::shipped_example` before the example edit: both `shipped_example_http_auth_validates_and_configures_no_key` and `shipped_example_api_key_entries_uncommented_are_refused_as_empty` failed.
- All of the above are green after implementation.

## Task Commits

1. **Task 1: tracer, placeholder refusal and boot proof** - `8585492f1` (feat)
2. **Task 2: config.example.yml and shipped-example tests** - `a050f72a5` (docs)
3. **Task 3: MIGRATION M-B-29 and CHANGELOG** - `127cddc60` (docs)

The tracer feedback gate ran in auto mode: the tracer `<verify>` (`config::agents::` lib tests and `build_auth_config` bin tests) was green end to end before expansion.

## Files Created/Modified

- `src/config/agents.rs` - the `entry.key.contains("${")` refusal, rustdoc for `ApiKeyConfig` and `validate`, 7 new tests
- `src/bin/paladin-server.rs` - `build_auth_config_refuses_an_unexpanded_placeholder_key`
- `config.example.yml` - `api_keys: []`, commented `key: ""` entries, honest header and LLM comment, treasurer note
- `MIGRATION.md` - row `M-B-29`, 11.5 bullet, 11.8 step 4 (additions only)
- `CHANGELOG.md` - one `[Unreleased]` Security bullet

## Decisions Made

- Used a plain substring check on `${` rather than reusing the LLM `resolve_secret_placeholder` grammar, so embedded and unterminated shapes are refused as well. No expansion, no `key_env` field, no change to the LLM mechanism (D-00e).
- Ordered the check after the reserved-name check and before the tenant and duplicate checks, so the placeholder is always the reported cause.

## Deviations from Plan

**1. [Rule 1 - Formatting] `cargo fmt` collapsed one multi-line call in a new test**
- **Found during:** Task 2 step 5
- **Fix:** ran `cargo fmt` before committing; no behavior change.
- **Commit:** `a050f72a5`

Otherwise the plan was executed as written. The Task 2 plan text says the uncommented example "contains no `${`"; the example file legitimately keeps `"${OPENAI_API_KEY}"`-style LLM lines, so the test asserts the parsed `http.auth.api_keys[].key` values contain none instead. This is a wording narrowing of the same intent, not a behavior change.

## Issues Encountered

None. Disk was tight (about 4 GB free at the end) but no build failed, so no caches were deleted.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or trust-boundary surface; the change only tightens an existing validator. T-54-01 to T-54-04 are mitigated by the tests above.

## Next Phase Readiness

- `k8s/server/configmap.yaml` still carries the two `${PALADIN_API_KEY_*}` keys and is now refused at boot; plan 54-06 owns that file.
- `tests/fixtures/config/v0.9.0-config.example.yml` also holds placeholder keys but is a frozen record that no test loads (the integration test uses `v0.9.0-config.test.yml`).
- Plan 54-02 changes the Docker docs for the literal-key requirement stated in `M-B-29`.

## Self-Check: PASSED

- Files: `src/config/agents.rs`, `src/bin/paladin-server.rs`, `config.example.yml`, `MIGRATION.md`, `CHANGELOG.md` all modified and committed.
- Commits `8585492f1`, `a050f72a5`, `127cddc60` present in `git log`.
- `cargo fmt --check`, clippy (lib and bin, `-D warnings`), the seven example-loading tests, `config::` lib tests (236 passed) and `make check-migration-allowlist` all passed.
