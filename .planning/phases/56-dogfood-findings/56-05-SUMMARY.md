---
phase: 56-dogfood-findings
plan: 05
subsystem: dogfood-findings
tags: [findings-log, binary-probes, credential-in-logs, config-validation, dogf-01, dogf-06, df-07, df-08, df-09]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-01 scripts/check-findings-log.sh (last-Disposition-bullet grammar); 56-04 fully dispositioned DF-01..DF-06 and the 56-DOGFOOD-EVIDENCE.md file"
provides:
  - "56-DOGFOOD-EVIDENCE.md sections Lead L3, Lead L6 and Lead L4: commands, exits, counts and classified outcomes"
  - "DF-07 (S1) and DF-08 (S2) logged open -> promote in v0.11.1 for Stage C; DF-09 (S3) logged and fixed by a comment-only edit"
  - "config.example.yml web_server comment now says no shipped binary mounts the developer inspector"
affects: [56-06..56-10, Stage C (/gsd-plan-phase 56 --gaps), SHIP-10]

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-05-SUMMARY.md
  modified:
    - .planning/phases/56-dogfood-findings/56-DOGFOOD-EVIDENCE.md
    - .project/v0.11.1/02-dogfood-findings-log.md
    - config.example.yml

key-decisions:
  - "L3 is one finding with one root cause (derived Debug over credential-bearing types, rendered whole at src/main.rs:35); the other unredacted types found by the audit are listed in the same entry rather than split"
  - "L6 graded S2 although channel_capacity 0 is harmlessly clamped to 1: the same gap lets trace.otel.enabled on a build without the otel feature silently export nothing, and D-04 says an arguable grade goes to the higher severity"
  - "The L4 comment says the section is 'parsed into Settings and checked by Settings::validate()' rather than 'parsed and validated', because DF-08 shows the server never calls validate()"
  - "requirements DOGF-01 and DOGF-06 NOT marked complete here: DOGF-06 cannot close until DF-07 and DF-08 are fixed in Stage C"

requirements-completed: []

duration: ~12min
completed: 2026-11-02
---

# Phase 56 Plan 05: Reproduce the three binary-level research leads Summary

**L3 reproduced (the `paladin` binary prints `http.auth.api_keys[].key` and four other credentials in its first log line at the default `debug` filter, DF-07 S1), L6 reproduced (`paladin-server` boots with config its validator rejects, DF-08 S2), L4 reproduced and fixed as a comment (no shipped binary mounts the inspector, DF-09 S3). The checker is red on DF-07 and DF-08 by design.**

## Performance

- **Plan-start SHA:** `1203a0dfd`
- **Tasks:** 2 of 2 (1 tracer, 1 auto)
- **Commits:** `4ee454557` (Task 1, L3 and DF-07), `28e0be203` (Task 2, L6, L4, DF-08, DF-09 and the comment)
- **Log:** +53 lines, 0 deletions (`git diff --numstat`)

## Probe results

| lead | binary / build | outcome | classification | DF | severity | disposition |
|------|----------------|---------|----------------|----|----------|-------------|
| L3 | `paladin`, `--features cli`, probe `exit=1` (unrelated SQLite path error after the leak) | fake `api_keys[0].key` appears 1 time in the first log line; a second probe shows `queue.redis_password`, `file_storage.minio_access_key` / `minio_secret_key` and `llm_api_key` also leak (1 each) | reproduced | DF-07 | S1 | open -> promote **in v0.11.1** |
| L6 | `paladin-server`, `--features web-server`, `exit=124` (killed by `timeout` while serving) | `paladin-server listening on` with `trace.channel_capacity: 0`; no panic (engine clamps to 1); `otel.enabled: true` without the `otel` feature also boots silently | reproduced (listens) | DF-08 | S2 | open -> promote **in v0.11.1** |
| L4 | same server | `GET /v1/dev-ui/threads/probe` = 404 while `/health` = 200; `create_dev_ui_router` has callers only in tests and one example; nothing reads `Settings.web_server` | reproduced | DF-09 | S3 | in v0.11.1 (fixed in this plan, comment only) |

Facts worth carrying forward:

- `src/main.rs:35` (`info!("Loaded configuration: {:?}", config)`) ships at tag `v0.11.0` unchanged, and `ApiKeyConfig` derived `Debug` there too. `LlmProviderConfig` already redacts (`api_key: <redacted>` in the same line), which is the pattern to copy.
- The credential-type audit table in the evidence lists 15 types: `Settings` (`llm_api_key`), `ApiKeyConfig`, `AuthConfig`, `WebHttpConfig`, `QueueConfig`, `FileStorageConfig`, `EmailAdapterConfig` (feature `notifications`, from source), `QdrantSanctumConfig`, `SourceConfig` and `MCPServerConfig` (URLs) are unredacted; `LlmProviderConfig`, `OtelConfig`, `AllowanceWebhookConfig`, `NodeCacheConfig` redact; `BearerTokenAuthConfig` holds no secret.
- Only one non-test site formats a whole config: `src/main.rs:35`. `paladin-server` does not log its `Settings`.
- L6 attempt 1 stopped on an unrelated refusal (`run_store.backend is configured but no waypoint store is wired`); the single permitted retry added only `APP_WAYPOINT_STORE_BACKEND` and `APP_WAYPOINT_STORE_SQLITE_PATH`.

## Checker output (final)

```
.project/v0.11.1/02-dogfood-findings-log.md:173: DF-07: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:204: DF-08: OPEN (the last Disposition bullet reads 'open')
parsed 9 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

`./scripts/check-findings-log.sh` exits 1, naming DF-07 and DF-08 as OPEN. That is the intended Stage C signal (D-05, D-06); DF-09 reads as final.

## Stage C inputs

Every `open -> promote **in v0.11.1**` entry this plan appended, for the `/gsd-plan-phase 56 --gaps` replan:

| id | sev | what Stage C must do (patch shape, D-00b: no public signature changes) |
|----|-----|------------------------------------------------------------------------|
| DF-07 | S1 | Stop the `{:?}` of the whole `Settings` at `src/main.rs:35`, or give a redacting `Debug` to `ApiKeyConfig` and the other credential-bearing types in the audit (`Settings.llm_api_key`, `QueueConfig.redis_password`, `FileStorageConfig` keys, `EmailAdapterConfig.password`); add a regression test that captures the line and asserts no key literal; fix per `LlmProviderConfig` / `RedactedKey` precedent. |
| DF-08 | S2 | Call `settings.validate()` in `src/bin/paladin-server.rs` `run()` right after `Settings::load_from_file`, mapping the error like the neighbouring `invalid ... configuration` refusals; test with `trace.channel_capacity: 0` and `trace.otel.enabled: true` on a non-`otel` build; decide whether `agent_runtime` is then validated twice. |

DF-09 needs nothing further. Not probed here, by plan: lead L8 (setup-check's Anthropic model id) lives in `56-RUNBOOK-KEYED-WALK.md` K8.

## Deviations from Plan

### Auto-fixed Issues

None for code. One wording departure: the plan asked the comment to say the `web_server` section "is parsed and validated". DF-08 shows the server never calls `Settings::validate()`, so the comment says it is "parsed into `Settings` and checked by `Settings::validate()`" instead, which is true. No behaviour is affected.

### Plan-level notes

- The probes ran in `mktemp -d` scratch directories outside the repository (the plan's PD-05-2) rather than the session scratchpad path; both directories were deleted after use. Every probe credential was a fake, and the evidence file and the log contain no probe literal (`grep -c` of the fake prefix in both: 0).
- The L6 probe needed one retry with two extra env variables (see above); it is allowed by the plan's flagged-assumption path and is not a deviation.
- Disk was never a constraint: 5860M free at start, 4067M after both scoped builds and both test runs; no cleanup, no retry, no ENOSPC.

## Verification

- `cargo test -p paladin-ai --test config_example_roundtrip`: `14 passed; 0 failed` after the comment edit.
- `cargo test -p paladin-ai --lib --features web-server -- example`: `6 passed; 0 failed`.
- `config.example.yml` diff: 6 added lines and 1 removed line, every one a `#` comment (`git diff -U0` filter returned 0 non-comment changes).
- `git diff --quiet 1203a0dfd -- src/main.rs src/config/agents.rs src/bin/paladin-server.rs crates/paladin-web/src/app.rs`: unchanged (P10).
- No file deletions in either commit.

## Known Stubs

None.

## Threat Flags

None beyond the findings logged. The unredacted-credential surface is DF-07 itself (T-56-18), now logged S1 and promoted.

## Self-Check: PASSED

- FOUND: `.planning/phases/56-dogfood-findings/56-DOGFOOD-EVIDENCE.md` with `## Lead L3`, `## Lead L6`, `## Lead L4`
- FOUND: commits `4ee454557` and `28e0be203` in `git log`
- FOUND: DF-07, DF-08, DF-09 in the findings log; DF-07 and DF-08 last bullet begins `open -> promote` (checker names both OPEN)
