---
phase: 58-crate-release
plan: 15
subsystem: credential-class-fixes
tags: [security, credentials, serde, setup-check, onboarding, ship-10, ship-09, gap-closure]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-05 register, evidence file and the maintainer's D-07 amendment (fix WR-03, WR-04, WR-09 before the tag)
provides:
  - "setup-check no longer echoes a malformed REDIS_URL or a failing QDRANT_URL's credentials (WR-03)"
  - "the onboarding wizard writes .env at mode 0600 on unix and narrows an existing file before the first write (WR-04)"
  - "skip_serializing (no default) on six credential fields, so no serialised config carries a secret and loading is unchanged (WR-09)"
  - "MIGRATION 11.1 rows M-B-48, M-B-49, M-B-50, Operators item 13, and [Unreleased] entries in the root and notifications changelogs"
  - "register rows R-47, R-48, R-53 fixed; evidence finding F-58-06"
affects: [58-06, 58-07, 58-08, 58-12]

tech-stack:
  added: []
  patterns:
    - "Value-free error text for a config value that may carry a credential"
    - "Create with mode 0600 and then set_permissions on the open handle before writing, so an older wider file is narrowed first"
    - "skip_serializing without default: omit on output, keep a required field required on load"

key-files:
  created:
    - .planning/phases/58-crate-release/58-15-SUMMARY.md
  modified:
    - src/application/cli/commands/setup_check.rs
    - src/application/cli/commands/onboarding.rs
    - src/config/settings.rs
    - src/config/agents.rs
    - src/config/queue.rs
    - src/config/file_storage.rs
    - tests/config_example_roundtrip.rs
    - crates/paladin-notifications/src/email_notification_adapter.rs
    - CHANGELOG.md
    - crates/paladin-notifications/CHANGELOG.md
    - MIGRATION.md
    - .planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md

key-decisions:
  - "WR-09 takes 56-REVIEW's first option (skip_serializing, no default); the step 0 consumer gate found no serialise-then-deserialise consumer, so the documented-invariant fallback was not needed"
  - "The Qdrant test pins a query-string credential as well as userinfo, because reqwest 0.12.28 already strips userinfo from its own error text"
  - "M-B-50 states that it supersedes M-B-36's sentence that Serialize output is unchanged; no existing line was edited"

requirements-completed: []

duration: "about 25 min (2026-11-05T16:45Z to 17:10Z)"
completed: 2026-11-05
---

# Phase 58 Plan 15: Credential-class 56-REVIEW Fixes Summary

**WR-03, WR-04 and WR-09 are fixed in 0.11.1 as four RED/GREEN pairs: a value-free Redis error and a URL-free Qdrant error, a 0600 `.env`, and `skip_serializing` on six credential fields, with MIGRATION 11.1 rows M-B-48..M-B-50 and register rows R-47, R-48, R-53 now `fixed`.**

## Performance

- **Tasks:** 5 of 5 (Task 1 tracer, Tasks 2-4 auto TDD, Task 5 docs and gates).
- **BASE:** `1f53434e70da1b92f420d1a7a561f13f0eba95fb` (head before the first plan commit).
- **Files:** 13 paths changed by the `(58-15)` commits, exactly the plan's `files_modified`; no dependency, version, remote or `.gitleaks.toml` change.

## Commits

| # | Commit | Subject |
|---|--------|---------|
| 1 | `ec55f7fcc` | test(58-15): setup-check must not echo a Redis or Qdrant URL's credentials (56-REVIEW WR-03) |
| 2 | `25b89618c` | fix(58-15): hide the URL in setup-check Redis and Qdrant errors (56-REVIEW WR-03) |
| 3 | `2a38220db` | test(58-15): onboarding must write .env at mode 0600 (56-REVIEW WR-04) |
| 4 | `a61367a0d` | fix(58-15): write the onboarding .env at mode 0600 (56-REVIEW WR-04) |
| 5 | `070864b03` | test(58-15): serialised config must omit every credential field (56-REVIEW WR-09) |
| 6 | `e88fea3b9` | fix(58-15): skip_serializing on the Settings, ApiKeyConfig, queue and MinIO secret fields (56-REVIEW WR-09) |
| 7 | `b7703b9ea` | test(58-15): the SMTP password must not serialise (56-REVIEW WR-09) |
| 8 | `d06702cf4` | fix(58-15): skip_serializing on the SMTP password (56-REVIEW WR-09) |
| 9 | `f5548d4d1` | docs(58-15): changelog and MIGRATION 11.1 entries for the credential-class fixes (WR-03, WR-04, WR-09) |
| 10 | `695599f84` | docs(58-15): flip register rows R-47, R-48 and R-53 to fixed and record F-58-06 (D-07 amended) |

SHA-G (the head the code gates ran on) is `f5548d4d1a9bf0612e05e07f927b7050e85b5f1f`; the one later commit changes planning files only.

## RED and GREEN results

| Task | RED (before the fix) | GREEN |
|------|----------------------|-------|
| 1 WR-03 (tracer) | `check_redis_format_error_does_not_echo_the_value` and `check_qdrant_failure_does_not_echo_the_url_credentials` failed; 6 passed | `test result: ok. 8 passed; 0 failed; 3 ignored` (module); tracer `<verify>` re-run, all items passed, so expansion continued |
| 2 WR-04 | `write_env_file_at_creates_a_new_file_at_mode_0600` and `..._narrows_an_existing_0644_file_to_0600` failed (umask 0022); the stays-0600 test passed | `test result: ok. 9 passed; 0 failed; 3 ignored` (module) |
| 3 WR-09 root | the four omit tests (`settings_serialisation_omits_every_credential_it_reaches`, `api_key_config_serialisation_omits_the_key`, `queue_config_serialisation_omits_the_redis_password`, `file_storage_config_serialisation_omits_both_minio_keys`) and two round-trip tests (`verbatim_example_round_trips_through_load_from_file`, `serialised_settings_carry_no_llm_api_key_and_no_webhook_secret`) failed; the load pin passed | `config::` lib filter `250 passed; 0 failed`; round-trip binary `14 passed; 0 failed` |
| 4 WR-09 SMTP | `email_adapter_config_serialisation_omits_the_password` and `settings_serialisation_omits_the_smtp_password` failed | `paladin-notifications --features email` `9 passed`; `config::settings` with `notifications` `5 passed` |

## Task 3 step 0 consumer gate

Command: `grep -rnE 'serde_(json|yaml)::to_|toml::to_' src crates tests examples benches`, filtered to the eight type names. Hits:

- `src/config/settings.rs:606` (`settings_debug_names_every_serialized_field`: lists `Settings::default()` keys, never deserialises)
- `tests/config_example_roundtrip.rs:129, 366, 420` (re-serialise a loaded `Settings` for a key-path diff, never deserialise)

A wider search for indirect paths (`Config::try_from(&`, `Json(settings)`, `to_value(&settings`) found none. No consumer deserialises serialised output of these types, so the fallback was not taken.

## Gate table on SHA-G (CARGO_INCREMENTAL=0)

| Gate | Result |
|------|--------|
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo test -p paladin-ai --features cli,notifications --lib` | `1633 passed; 0 failed; 6 ignored` (F-58-03 flake did not fire) |
| `cargo test -p paladin-ai --features cli,notifications --test config_example_roundtrip --test cli` | `14 passed; 0 failed` in the round-trip binary, `130 passed` in the other, exit 0 |
| `cargo test -p paladin-notifications --features email` | `9 passed; 0 failed` |
| `make api-surface` | `API surface unchanged` (4331 items) |
| `make check-gates` | exit 0 (`scanned 627 file(s) (19 test-only), 0 finding(s)`) |
| `pre-commit run --all-files` | all hooks passed; tree clean (re-run on the final head, also passed) |
| `58-check-register.sh --draft` | `fixed 51, waived 37, re-homed 46; 0 failure(s)` |

No coverage figure is claimed from this sandbox (D-00e). `check-semver.sh --release-type patch` was not run here; an attribute-only change adds no public item (`make api-surface` unchanged), and plan 58-06 re-reads it on its own head.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in the plan's premise] The Qdrant userinfo test cannot fail RED**
- **Found during:** Task 1 RED run.
- **Issue:** the plan expected both new tests to fail on RED. reqwest 0.12.28 strips URL userinfo before it builds the error, so `http://user:<pw>@127.0.0.1:1` already produced `...for url (http://127.0.0.1:1/collections)`; only the Redis test failed. A credential in the query string (`?api-key=<pw>`) is still echoed, and that is the leak `without_url()` fixes.
- **Fix:** the Qdrant test loops over both forms (userinfo and query string); the query form fails RED and passes GREEN, so the test would catch a regression of either.
- **Files modified:** `src/application/cli/commands/setup_check.rs`
- **Commit:** `ec55f7fcc` (RED), `25b89618c` (GREEN)

**2. [Rule 1 - test fixture] `UserRole` serialises lowercase**
- **Found during:** Task 3 RED run: the load pin failed on `"role": "User"` ("expected `admin` or `user`").
- **Fix:** the fixture uses `"user"`, and the without-key case now asserts the error names `missing field \`key\``, so it fails for the intended reason.
- **Commit:** `070864b03`

**3. [Plan naming] Storage config type names in F-58-06**
- The plan named `RedisConfig` and "the Redis node-cache config"; the actual types are `RedisQueueConfig` (`redis.rs`) and `RedisNodeCacheConfig` (`node_cache/redis.rs`). F-58-06 uses the real names.
- **Commit:** `695599f84`

Task 4's GREEN commit staged only `email_notification_adapter.rs`: `settings.rs` needed no change after its RED commit.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access pattern or schema; the change narrows existing surface (T-58-150 to T-58-154 mitigated; T-58-156 accepted as planned).

## Residuals and follow-ups

- **F-58-06 residuals (names and counts only), left unchanged and unregistered for plan 58-06's sweep:** four same-class types, `NodeCacheConfig` (`src/config/node_cache.rs`, not reachable from `Settings`), `RedisQueueConfig`, `MinioConfig` and `RedisNodeCacheConfig` (all `paladin-storage`), each deriving `Serialize` with a plaintext secret field. Pre-fix `.env` files stay 0644 until rewritten or `chmod 600` (`M-B-49`).
- **Docs candidate (not edited, P-15-1):** `docs/src/appendix/cli-onboarding.md` could state that the wizard writes `.env` at mode 0600 on unix.
- **For plans 58-07 and 58-08:** the 0.11.1 credential-class ids are `M-B-48`, `M-B-49`, `M-B-50`; `M-B-36` says `Serialize` output is unchanged and `M-B-50` states that it supersedes that sentence.
- No requirement ticked: SHIP-08/09/10 are earned by plan 58-14 after the tag.

## Self-Check: PASSED

All 13 `files_modified` paths exist, the ten commit ids resolve with `git cat-file -e`, and no file was deleted since BASE.
