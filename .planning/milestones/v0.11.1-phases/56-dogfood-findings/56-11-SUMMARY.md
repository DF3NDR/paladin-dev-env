---
phase: 56-dogfood-findings
plan: 11
subsystem: dogfood-findings
tags: [security, redaction, debug, credential-logging, df-07, dogf-06, dogf-01, stage-c, gap-closure]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-05 Lead L3 evidence (probe, credential audit table), DF-07 findings-log entry"
provides:
  - "src/config/redaction.rs: crate-private RedactedSecret Debug wrapper (<redacted>, or empty-string marker when empty)"
  - "Hand-written redacting Debug on ApiKeyConfig, Settings, QueueConfig, FileStorageConfig and paladin-notifications EmailAdapterConfig (AuthConfig and WebHttpConfig delegate)"
  - "paladin binary startup line rendered by a private settings_summary(&Settings); no Settings value is Debug-formatted in src/main.rs"
  - "CHANGELOG [Unreleased] ### Security (root and paladin-notifications), MIGRATION 11.1 row M-B-36 and 11.8 step 8"
  - "DF-07 closed in the findings log (in v0.11.1, fixed by 74317f48a, fbc514410)"
affects: [56-12, 56-13, 56-verification, SHIP-10]

tech-stack:
  added: []
  patterns:
    - "Hand-written Debug with one .field per struct field plus a guard test that compares serde_json::to_value keys with the pretty-Debug field lines"
    - "Redaction tests assert the planted sentinel is present in the field before formatting (never vacuous) and that <redacted> appears"

key-files:
  created:
    - src/config/redaction.rs
    - .planning/phases/56-dogfood-findings/56-11-SUMMARY.md
  modified:
    - src/config/mod.rs
    - src/config/agents.rs
    - src/config/settings.rs
    - src/config/queue.rs
    - src/config/file_storage.rs
    - src/main.rs
    - crates/paladin-notifications/src/email_notification_adapter.rs
    - crates/paladin-notifications/CHANGELOG.md
    - CHANGELOG.md
    - MIGRATION.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "Both halves of DF-07 fixed (PD-11-1): the whole-Settings format is removed at its only call site and every credential-bearing config type redacts in Debug"
  - "RedactedSecret is pub(crate) in paladin-ai and module-private in paladin-notifications (PD-11-2): no public surface added to any crate"
  - "URL-shaped fields (llm_url, SourceConfig.url, QdrantSanctumConfig.url, MCPServerConfig.endpoint) stay unredacted, accepted as T-56-41; Serialize still emits secrets, accepted as T-56-42"

patterns-established:
  - "settings_debug_names_every_serialized_field: pretty Debug puts each top-level field on a line with four spaces of indent, so a nested field name cannot satisfy the guard by accident"

requirements-completed: [DOGF-06, DOGF-01]

duration: ~15min
completed: 2026-11-03
---

# Phase 56 Plan 11: DF-07 credential-in-log fix Summary

**The `paladin` binary no longer logs its whole `Settings`: the startup line is a five-fact credential-free summary, and `ApiKeyConfig`, `Settings`, `QueueConfig`, `FileStorageConfig` and `EmailAdapterConfig` print `<redacted>` for every set secret in `Debug`. A re-probe of the real binary found 0 of 5 planted credentials in its log.**

Plan-start SHA: `c504c0fb55164f4fca73b51c89d7dccde3393948`. Free disk at start: 5430M on `/` (low point 1499M after the final optional `cargo check`; no ENOSPC, `target/debug/incremental` never deleted).

Model note: the orchestrator's trailer names a different model; commits carry the harness-provided attribution for this run (`Claude Sonnet 5.5`) plus the `Claude-Session` line.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | `74317f48a` | fix(56-11): redact ApiKeyConfig in Debug and stop logging the whole Settings (DF-07) |
| 2 | `fbc514410` | fix(56-11): redact Settings, queue, MinIO and SMTP secrets in Debug (DF-07) |
| 3 (records) | `0ce0ac1bd` | docs(56-11): DF-07 CHANGELOG, MIGRATION M-B-36 and rotation step (DF-07) |
| 3 (log) | `06f06354f` | docs(56-11): DF-07 disposition in v0.11.1 (DF-07, DOGF-06) |

## Red-then-green record

Every new test was run against the derived `Debug` (or the unchanged startup format) first, and each sentinel assertion failed before the fix. Assertion messages name the leaking field, not the rendered text.

| Test | Red run (before fix) | Green run (after fix) |
|------|----------------------|-----------------------|
| `api_key_config_debug_redacts_the_key` | FAILED: `ApiKeyConfig` Debug printed the key (`key: "<fake-key>"`) | ok |
| `auth_and_web_http_config_debug_redact_the_key` | FAILED: `AuthConfig Debug leaked the key` | ok |
| `redacted_secret_debug_redacts_a_value_and_quotes_empty` | did not exist (module absent); written with the helper | ok |
| `settings_summary_names_no_credential` | FAILED: `startup line leaked llm_api_key` | ok |
| `settings_summary_names_the_shape_without_values` | FAILED: `no llm_type in summary` | ok |
| `queue_config_debug_redacts_the_redis_password` | FAILED: `QueueConfig Debug leaked the redis password` | ok |
| `file_storage_config_debug_redacts_both_minio_keys` | FAILED: `FileStorageConfig Debug leaked the access key` | ok |
| `settings_debug_redacts_every_credential_it_reaches` | FAILED: `Settings Debug leaked llm_api_key` | ok |
| `settings_debug_names_every_serialized_field` | passed against the derive (it is the guard for the hand-written impl, as the plan states) | ok |
| `email_adapter_config_debug_redacts_the_password` | FAILED: `EmailAdapterConfig Debug leaked the password` | ok |

Commands (all on the PD-11-4 profile): `cargo test -p paladin-ai --features cli,web-server --lib debug_redact` (7 passed, includes the pre-existing `otel_debug_redacts_header_values`), `--lib settings_debug_names_every_serialized_field`, `--bin paladin settings_summary` (2 passed), and `cargo test -p paladin-notifications --features email --lib debug_redact` (1 passed).

Tracer gate (auto mode): after the Task 1 commit the tracer's `<verify>` was re-run end to end and passed (`TRACER-VERIFIED`) before Task 2 started.

## Re-probe

Binary built from HEAD (`cargo build -p paladin-ai --bin paladin --features cli,web-server`, 3258M free before, 2727M after). Scratch directory outside the repo, deleted afterwards. Config: `config.example.yml` with `http.auth.api_keys` holding one entry (`name: "df07-probe"`, `role: "user"`, `tenant: "Acme-Prod"`), a non-empty `llm_api_key`, a `queue:` block with a `redis_password`, and a `file_storage:` block with both MinIO keys. Command: `env -u RUST_LOG OPENAI_API_KEY=<fake-env> timeout 20 target/debug/paladin --config ./l3b.yml > l3b.log 2>&1`. Exit code 1: the example's SQLite path is not openable from the scratch directory, logged after `Starting services...`, the same unrelated error 56-05 saw. The startup line is written before `setup_and_run`, so the probe is valid. Each fake was first confirmed present once in the config file (non-vacuous).

| field | count in log |
|-------|--------------|
| `http.auth.api_keys[0].key` (`<fake-key-1>`) | 0 |
| `llm_api_key` (`<fake-key-2>`) | 0 |
| `queue.redis_password` (`<fake-key-3>`) | 0 |
| `file_storage.minio_access_key` (`<fake-key-4>`) | 0 |
| `file_storage.minio_secret_key` (`<fake-key-5>`) | 0 |
| `OPENAI_API_KEY` env fake (extra check) | 0 |

First log line, values replaced by their field names:

```
[... INFO  paladin] Loaded configuration: llm_type=<llm_type>, server=<host>:<port>, agents=<count>, http.auth.enabled=<bool>, http.auth.api_keys=<count>
```

The log has 6 lines; exactly one contains `Loaded configuration:`.

## Security review

Per `.github/instructions/security.instructions.md` "Manual review still matters".

**(a) Non-test `{:?}` / `{:#?}` / `dbg!` format sites in `src/`**, re-listed with the 56-05 audit's grep shape (format specifier or `?config` / `?settings` applied to a config-ish value, matches below the file's first `#[cfg(test)]` only):

| site | content | verdict |
|------|---------|---------|
| `src/main.rs:54` | `eprintln!("Failed to load configuration: {:?}", e)` | prints the load error `e`, not a config |
| `src/bin/paladin-server.rs:220` | `format!("{:?}", run_configs.run_store.backend)` | a backend label |

The former `src/main.rs:35` whole-`Settings` site is gone, and `grep -v '^\s*//' src/main.rs | grep -cF '{:?}", config)'` prints 0. No remaining site formats a `Settings` or a credential-bearing config.

**(b) Audit table rows that said "redacts? NO"** (56-05 Lead L3), and their status now:

| type | status |
|------|--------|
| `Settings` (`llm_api_key`) | fixed here: hand-written Debug, `settings_debug_redacts_every_credential_it_reaches` |
| `ApiKeyConfig` (`key`) | fixed here: `api_key_config_debug_redacts_the_key` |
| `AuthConfig` | fixed by delegation: `auth_and_web_http_config_debug_redact_the_key` |
| `WebHttpConfig` | fixed by delegation: same test |
| `QueueConfig` (`redis_password`) | fixed here: `queue_config_debug_redacts_the_redis_password` |
| `FileStorageConfig` (both MinIO keys) | fixed here: `file_storage_config_debug_redacts_both_minio_keys` |
| `EmailAdapterConfig` (`password`) | fixed here: `email_adapter_config_debug_redacts_the_password` |
| `QdrantSanctumConfig.url`, `SourceConfig.url`, `MCPServerConfig.endpoint` (and `llm_url`) | accepted as T-56-41 (URL-shaped, a credential only if an operator embeds userinfo; `settings_summary` renders no URL) |
| `BearerTokenAuthConfig` | n/a, holds no secret |

Accepted separately as T-56-42: `Serialize` of these types still emits secrets; no shipped code path serialises `Settings` outward and `skip_serializing` would change the 56-02 round-trip (a separate decision).

**(c) No log statement added here interpolates a key.** The only new log call is `info!("{}", settings_summary(&config))`, whose output is `llm_type`, `server` host and port, the agent count, `http.auth.enabled` and the count of `api_keys`. Test assertion messages name the leaking field and never print the rendered `Settings`. Probe and test literals are obviously fake; this SUMMARY shows them as `<fake-...>`. `Debug` on every changed type prints `<redacted>` (or the empty-string marker when empty), `redacted_secret_debug_redacts_a_value_and_quotes_empty` pins that rule.

## Gates

Disk printed before every build step (see start/low figures above).

| Gate | Result line |
|------|-------------|
| `cargo fmt --all -- --check` | clean (`FMT-CLEAN`) |
| `cargo clippy -p paladin-ai --features cli,web-server --lib --bins -- -D warnings` | `Finished dev profile` with no warnings |
| same with `--profile test` | `Finished test profile` with no warnings |
| `cargo clippy -p paladin-notifications --features email --lib -- -D warnings` | `Finished dev profile` with no warnings |
| same with `--profile test` | `Finished test profile` with no warnings |
| `make api-surface` | `API surface extracted ... (4325 items)` / `API surface unchanged` |
| `check-semver.sh --package paladin-ai` | `semver: paladin-ai: 223 checks: 223 pass, 31 skip` / `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` |
| `check-semver.sh --package paladin-notifications` | `semver: paladin-notifications: 223 checks: 223 pass, 31 skip` / `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` |
| `make check-gates` | exit 0; includes `Allowlist is set-equal to the MIGRATION.md 11.2 deliberate-breaking register`, `every copyable version string equals '0.11.0'`, `0 finding(s)` from the stub-panic scan |
| `git diff --quiet <plan-start-sha> -- Cargo.toml Cargo.lock crates/*/Cargo.toml` | no dependency change, so `make security` is unaffected (runs at phase verification) |
| `.cargo/semver-checks-allowlist.toml` and MIGRATION 11.2 | unchanged (byte-identical, diff checked) |

Extra check not in the plan: `cargo check -p paladin-ai --features cli,web-server,notifications --lib` compiled, so the `#[cfg(feature = "notifications")]` field in `Settings`'s hand-written Debug builds. It is not covered by a test in the default profile.

Frozen records (D-00c): MIGRATION parts 10 and below, section 11.2, and every `[0.11.0]` changelog section diffed byte-identical to the plan start; the records diff has 0 deleted lines. Findings log diff: `1 0` (one line added, 0 deleted).

## Deviations from Plan

None - plan executed exactly as written. Two minor notes, neither a rule deviation:

- `cargo fmt --all` reflowed one `Settings::load_from_file(concat!(...))` call in a new test in `src/main.rs`; the reformatted text is what was committed.
- The `Settings` rustdoc says URL-shaped fields print as written (T-56-41) rather than claiming a `Settings` never prints a credential, so the documentation matches the accepted residual risk.

## Authentication gates

None.

## Known Stubs

None. No placeholder text, hardcoded empty values or unwired data sources were added.

## Threat Flags

None. No new endpoint, auth path, file access or schema change. This plan only reduces what is logged and formatted.

## Stage C inputs

None new. No finding was discovered while executing this plan, so nothing was added to the log (next free id stays DF-26). `./scripts/check-findings-log.sh` after this plan names exactly `DF-08` and `DF-19` as OPEN.

## Self-Check: PASSED

Files found: `src/config/redaction.rs`, `src/config/agents.rs`, `src/main.rs`, `src/config/settings.rs`, `src/config/queue.rs`, `src/config/file_storage.rs`, `crates/paladin-notifications/src/email_notification_adapter.rs`. Commits found: `74317f48a`, `fbc514410`, `0ce0ac1bd`, `06f06354f`.
