---
phase: 41-admission-time-allowance-enforcement
plan: 03
subsystem: treasurer
tags: [allowance, config, boot-coherence, treasurer-wiring, webhook-secret, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 01
    provides: AllowanceConfig api_keys grammar, AllowancePolicy, Treasurer, RunSubmissionService::with_treasurer, the 41-01 checkpoint decision (option-b)
provides:
  - full treasurer.allowance grammar (tenants, api_keys, lifetime, per-entry and global warn_at, operator webhook target)
  - AllowanceConfig::validate_against, the pure D-11 coherence check
  - AllowanceWebhookConfig with redacting Debug and skip_serializing secret
  - strict (deny_unknown_fields) loading for every struct under treasurer:
  - APP_TREASURER_ALLOWANCE_WARN_AT and APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET env overrides
  - build_run_api D-11 check before the disabled-store early return, Treasurer construction, RunApiHandles.treasurer
  - paladin_web::agent_auth::OPEN_ACCESS_PRINCIPAL_ID
affects: [41-04, 41-05, 41-06, 41-08, 41-09]

tech-stack:
  added: []
  patterns:
    - "One exact-integer decimal parser serves price axes, allowance amounts and lifetimes; every error names the full config path and the raw value, never a secret"
    - "Boot coherence is a pure function fed from AgentAuthConfig, run before any early return so a disabled store cannot hide a configured allowance"
    - "Operator webhook secret follows the WebhookSpec pattern: manual redacting Debug plus skip_serializing, supplied by env because the loader does not expand placeholders"

key-files:
  created: []
  modified:
    - src/config/treasurer.rs
    - src/config/mod.rs
    - config.example.yml
    - docs/src/getting-started/configuration.md
    - src/infrastructure/web/run_api_wiring.rs
    - crates/paladin-web/src/agent_auth.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - CHANGELOG.md
    - .planning/WINDOWS.md
    - .project/current-exports.txt

key-decisions:
  - "Honoured the 41-01 checkpoint decision (option-b): the operator webhook config carries url and secret only here; the twelve-key payload (tenant_id, api_key_id names) is delivered by 41-08, and the docs say delivery lands later in this phase"
  - "AllowanceConfig::is_empty counts only the two entry maps, so a webhook or warn_at alone builds no Treasurer"
  - "build_run_api returns an error, never skips enforcement, if allowance entries exist but no ledger was built"
  - "Filed WINDOWS.md row 62: the config loader performs no ${VAR} expansion, so the api_keys examples in config.example.yml and k8s/server/configmap.yaml load literal placeholders"

patterns-established:
  - "known_allowance_targets derives allowance-able key names (principal ids, never key values) and tenants from AgentAuthConfig, plus the open-access pair when auth is disabled"

requirements-completed: []

duration: ~75min
completed: 2026-10-03
status: complete
---

# Phase 41 Plan 03: Operator grammar, boot coherence and Treasurer wiring Summary

**`treasurer.allowance` now carries every limit kind (tenants, API keys, lifetime caps, warn thresholds, operator webhook target), every typo or incoherent entry stops `paladin-server` at boot naming its path, and `build_run_api` builds the one `Treasurer` over the run store's ledger and enforces it on `POST /v1/runs` -- proven through the real router.**

## Performance

- **Duration:** ~75 min (including a one-off `cargo install` of cargo-audit and cargo-deny)
- **Completed:** 2026-10-03
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 11 (no new files)

## Accomplishments

- `AllowanceEntryConfig` gains `lifetime` and `warn_at`; `AllowanceConfig` gains global `warn_at` (default 80, `DEFAULT_ALLOWANCE_WARN_AT`), `webhook` and `tenants`. `resolve` iterates tenants then API keys in key order and builds the `AllowancePolicy` in nano-units and whole seconds. `warn_at` above 100 is rejected at both levels, never clamped; `amount` and `lifetime` share one positive-only exact-integer grammar whose errors name `treasurer.allowance.<map>.<id>.<field>` and the raw value.
- `deny_unknown_fields` on `TreasurerConfig`, `AllowanceConfig`, `AllowanceEntryConfig` and `AllowanceWebhookConfig` (six occurrences in the file with `PriceRowConfig`): `allowence:`, `api_key:`, `perid:`, `lifetim:` and `secrett:` each fail the load.
- `AllowanceWebhookConfig { url, secret }`: manual `Debug` prints `[redacted]`, `#[serde(skip_serializing)]` on the secret, an empty url rejected without echoing the secret. `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` applies only when a webhook entry exists; `yaml_env_placeholder_is_not_expanded` pins that a `${VAR}` value arrives literally.
- `AllowanceConfig::validate_against` (pure): disabled run store, unknown API key and unknown tenant each produce an error naming the offending path and the config keys involved.
- `build_run_api` runs that check before the `Disabled` early return, with targets from `known_allowance_targets(&auth)` (principal ids, tenants, `bearer_tenant`, and `anonymous` / `open-access` when auth is disabled). With entries present it builds one `Treasurer` over the run store's own ledger, attaches it with `with_treasurer`, and returns it as `RunApiHandles.treasurer`; without entries nothing is built.
- `OPEN_ACCESS_PRINCIPAL_ID` (`"anonymous"`) added to `paladin_web::agent_auth` and used by the private `open_access()`; no behaviour change.
- `config.example.yml` (commented `allowance:` example) and `docs/src/getting-started/configuration.md` (`## Treasurer allowances`: grammar table, window and limit model, warn semantics, boot errors, currency caveat, `webhooks.allow_private` note; the Token Budget closing sentence now points at it).
- Registers: MIGRATION.md 9.2 `RunApiHandles` row (Y) with its allowlist entry, 9.5 `treasurer.allowance` bullet, CHANGELOG additions (grammar, boot coherence, a `### Changed` bullet for stricter loading), refreshed `.project/current-exports.txt` (4109 to 4156 items).

## Task Commits

1. **Task 1: full grammar, strict keys, redacted secret, validate_against, env overrides, docs** - `172e9db` (feat)
2. **Task 2: wiring, coherence before the early return, RunApiHandles.treasurer, registers, API baseline** - `efc10d9` (feat)

## TDD / red evidence

- Task 1: tests and implementation landed together in the working tree; the red demonstration is by mutation. Removing `deny_unknown_fields` from `TreasurerConfig` and changing the warn bound `> 100` to `> 101` failed `allowance_unknown_keys_fail_to_load` and `allowance_warn_at_above_100_is_rejected_globally_and_per_entry`; restored. One of my own tests (`allowance_warn_at_defaults_to_80_and_accepts_0_and_100`) failed first because a per-entry override set in iteration one leaked into iteration two; fixed in the test.
- Task 2: the same approach. Replacing `.with_treasurer(...)` with a no-op made `wired_treasurer_refuses_an_exhausted_key_through_the_run_router` fail with `left: 202, right: 429`; restored.
- Honest note: neither task was written strictly test-first; the mutation runs are the evidence that the tests can fail.

## Verification

- `cargo test -p paladin-ai --lib config::treasurer` 34 passed; `cargo test -p paladin-ai --lib config::` 191 passed; `--doc treasurer` 10 passed.
- `cargo test -p paladin-ai --features web-server --lib infrastructure::web::run_api_wiring` 16 passed (the six new named tests plus `open_access_targets_are_known_only_when_auth_is_disabled`); full `cargo test -p paladin-ai --features web-server --lib` 1128 passed.
- `cargo test -p paladin-web --lib agent_auth` 17 passed; `cargo test -p paladin-ai --features web-server --bin paladin-server` 19 passed; `--test v0_9_config_boot` 9 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean.
- `./scripts/check-migration-allowlist.sh` exit 0.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` failed before the refresh with only this plan's symbols added and one derived `AllowanceConfig::default` line replaced by a manual impl (still `Default`); after `make api-surface-update` it reports "API surface unchanged" (4156 items).
- `make security` (cargo-audit, cargo-deny 0.20.2): advisories ok, bans ok, licenses ok, sources ok.
- Acceptance greps: `deny_unknown_fields` 6; `f64` outside comments 0; `validate_against` appears once before the early return; 9.5 mentions `treasurer.allowance` 5 times.
- Manual credential-handling review: the webhook secret appears in `Debug` only as `[redacted]`, is skipped by `Serialize`, is not interpolated into any validation error (an empty-url test asserts this), and no log statement touches it. The only read of it is the env override.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Installed cargo-audit and cargo-deny for `make security`**
- **Found during:** Task 2 final gate
- **Issue:** neither tool was installed (`cargo audit` was an unknown command).
- **Fix:** `cargo install --locked cargo-audit cargo-deny` as the plan itself directs (both are the tools CI's security job installs), with `CARGO_TARGET_DIR` pointed at the scratchpad and the build directory removed afterwards. No repository file changed.

**2. [Rule 1 - Bug in own test] Warn-at edge test leaked state between iterations**
- **Found during:** Task 1 first run
- **Fix:** split the global and per-entry edge loops. No production change.

**3. [Note] Replaced rather than kept the 41-01 `allowance_unknown_keys_fail_to_load`**
- The plan's test name `allowance_unknown_keys_fail_to_load` already existed (single case). It now covers one case per struct plus the `allowence` section typo; the old body was a subset.

**Total deviations:** 2 small fixes, 1 note; no scope change.

## Authentication Gates

None.

## Known Stubs

None. The webhook `url` and `secret` are parsed, validated and redacted but nothing delivers to them yet; that is 41-08's stated scope (the docs and CHANGELOG say delivery lands later), not a stub standing in for this plan's goal.

## Threat Flags

None beyond the register. T-41-14 (typos or orphaned entries enforce nothing) is mitigated by `deny_unknown_fields`, `validate_against` and `build_run_api_rejects_an_allowance_for_an_unknown_api_key` / `..._tenant`. T-41-15 (secret disclosure) by the redacting `Debug`, `skip_serializing` and `allowance_webhook_secret_is_redacted_from_debug_and_serialize`. T-41-16 by the single checked-arithmetic parser. T-41-17 by `yaml_env_placeholder_is_not_expanded`. T-41-SSRF is transferred to 41-08 as planned.

## Notes for later plans

- `requirements-completed` stays empty: ALLOW-01 is now fully configurable and enforced on `POST /v1/runs`, but fork, the agent routes and schedule-fired runs (41-04, 41-05) and the notice legs are still open, as 41-01 and 41-02 recorded.
- 41-04 must hand `handles.treasurer` to `AgentApiState` without changing `build_run_api`'s signature (C14).
- WINDOWS.md row 62 (open, owner Phase 46): the `${PALADIN_API_KEY_*}` placeholders in `config.example.yml` and `k8s/server/configmap.yaml` are not expanded by any code in the tree; the configmap header even claims the server expands them.
- Commit trailers use the `Claude Fable 5.1` line from this dispatch's `commit_attribution` block, whereas 41-01 and 41-02 used the session reminder's model line. The orchestrator may want to normalise before push.

## Self-Check: PASSED

Both commits (`172e9db`, `efc10d9`) are present in `git log`; all eleven listed files exist and were modified.
