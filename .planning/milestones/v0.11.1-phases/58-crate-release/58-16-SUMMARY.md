---
phase: 58-crate-release
plan: 16
subsystem: credential-class-fixes
tags: [security, credentials, serde, debug-redaction, semver, ship-10, ship-09, gap-closure]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-06 register lock (R-95 re-homed), plan 58-15 credential-class pattern (skip_serializing with no default), the maintainer's second D-07 amendment (Fix before tag)
provides:
  - "skip_serializing (no serde default) on NodeCacheConfig.redis_password, RedisQueueConfig.redis_password, RedisNodeCacheConfig.redis_password, MinioConfig.access_key and MinioConfig.secret_key; loading is unchanged and pinned"
  - "hand-written redacting Debug for RedisQueueConfig and MinioConfig through a module-private RedactedSecret per file"
  - "MIGRATION 11.1 rows M-B-51 and M-B-52, and [Unreleased] Security entries in the root and paladin-storage changelogs"
  - "register row R-95 fixed, R-95 todo completed, evidence finding F-58-07"
  - "D-11 semver green on SHA-S2 (12 packages, 223 checks each, 0 failures) and the pre-bump head SHA-P2 pushed"
affects: [58-07, 58-08, 58-12, 58-14]

tech-stack:
  added: []
  patterns:
    - "skip_serializing without default: omit on output, keep a required field required on load"
    - "Module-private RedactedSecret (set secret prints <redacted>, empty prints the empty-string form) where a crate cannot reach a pub(crate) helper"
    - "names-every-field Debug drift guard: the pretty Debug of a default value must name every serialised key plus the redacted ones"

key-files:
  created:
    - .planning/phases/58-crate-release/58-16-SUMMARY.md
  modified:
    - src/config/node_cache.rs
    - crates/paladin-storage/src/redis.rs
    - crates/paladin-storage/src/node_cache/redis.rs
    - crates/paladin-storage/src/minio.rs
    - CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - MIGRATION.md
    - .planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md
    - .planning/phases/58-crate-release/.continue-here.md
    - .planning/todos/completed/2026-11-05-serialize-plaintext-secrets-storage-and-node-cache-configs.md

key-decisions:
  - "Second D-07 amendment (2026-11-05): the four R-95 config types are fixed in 0.11.1 before the tag. The maintainer's answer to Keep re-homed versus Fix before tag was the option label: Fix before tag"
  - "A module-private RedactedSecret in redis.rs and in minio.rs rather than a shared paladin-storage helper, because a shared helper would need a lib.rs change outside this plan's file list (the paladin-notifications DF-07 precedent)"
  - "No serde default on any of the five fields, so MinioConfig without either key still fails to load and the Option passwords load as before"
  - "No MIGRATION 11.2 row (no field, type or signature change) and no Operators item (no host artifact to fix); M-B-52's action cell carries the rotate-if-logged advice, as M-B-48 does"

requirements-completed: []

duration: "about 32 min (2026-11-05T18:05Z to 18:37Z)"
completed: 2026-11-05
---

# Phase 58 Plan 16: Residual Serialize-Secret Config Fixes Summary

**The four register-R-95 config types are fixed in 0.11.1 as three RED/GREEN pairs: `skip_serializing` on five plaintext secret fields (no serde default) and a redacting hand-written `Debug` for `RedisQueueConfig` and `MinioConfig`, with MIGRATION 11.1 rows M-B-51 and M-B-52, R-95 now `fixed`, D-11 semver green on SHA-S2 and the pre-bump head SHA-P2 pushed.**

## Performance

- **Duration:** about 32 minutes (2026-11-05T18:05Z to 18:37Z)
- **Tasks:** 5 of 5
- **Commits:** 6 code (3 RED, 3 GREEN) and 4 docs, then this plan's own metadata commit
- **BASE (head before any 58-16 commit):** `f05e51006aa0c854d8130e00334961a916dda86e`

## Commits

| # | Commit | Message |
|---|--------|---------|
| 1 | `86bc21d28` | test(58-16): NodeCacheConfig must not serialise the Redis password (R-95) |
| 2 | `eabdfb282` | fix(58-16): skip_serializing on the NodeCacheConfig Redis password (R-95) |
| 3 | `84a3b32a7` | test(58-16): the paladin-storage Redis configs must not serialise their password and RedisQueueConfig Debug must redact it (R-95) |
| 4 | `1decd184d` | fix(58-16): skip_serializing on the Redis queue and node-cache passwords and a redacting RedisQueueConfig Debug (R-95) |
| 5 | `496df651e` | test(58-16): MinioConfig must not serialise or Debug-print its keys (R-95) |
| 6 | `f1605c971` | fix(58-16): skip_serializing on the MinioConfig keys and a redacting MinioConfig Debug (R-95) |
| 7 | `7b06e5e6b` | docs(58-16): changelog and MIGRATION 11.1 entries (this is SHA-G2) |
| 8 | `0143b50d5` | docs(58-16): flip register row R-95 to fixed and record F-58-07 (this is SHA-S2) |
| 9 | `042df99b2` | docs(58-16): record D-11 semver and the pre-bump sweep on SHA-S2 (this is SHA-P2) |
| 10 | `171d7efad` | docs(58-16): resume file for CI on SHA-P2 (local; rides the next push) |

## RED and GREEN results

| Task | RED (failing) | RED (passing) | GREEN |
|------|---------------|---------------|-------|
| 1 `NodeCacheConfig` (tracer) | `node_cache_config_serialisation_omits_the_redis_password` (6 passed, 1 failed) | `node_cache_config_still_loads_a_redis_password` and the five existing tests | `test result: ok. 7 passed; 0 failed`; clippy `-D warnings` clean |
| 2 Redis configs | `redis_queue_config_serialisation_omits_the_redis_password`, `redis_queue_config_debug_redacts_the_redis_password`, `redis_node_cache_config_serialisation_omits_the_redis_password` (5 passed, 3 failed) | `redis_queue_config_debug_names_every_field`, both load pins, both `..._default_matches_documented_values` | 8 targeted pass; full `--lib` with `redis-queue,redis-cache`: 363 passed, 0 failed; clippy clean |
| 3 `MinioConfig` | `minio_config_serialisation_omits_both_keys`, `minio_config_debug_redacts_both_keys` (12 passed, 2 failed) | `minio_config_debug_names_every_field`, `minio_config_requires_both_keys_on_load`, rewritten `test_minio_config_serialization`, the other `minio_config` tests | 21 `minio` tests passed, 0 failed; clippy clean |

The tracer gate (Task 1) was verified end to end (RED before GREEN, one attribute, no serde default, `pub struct NodeCacheConfig` declaration unreflowed, fmt) before the expansion tasks.

## Step 0 consumer gate

Read-only greps over `src`, `crates`, `tests`, `examples`, `benches` (markdown excluded) for serde calls and for the four type names. Result equals the plan-time one:

- One serialise-then-deserialise consumer of the four types, test-only: `test_minio_config_serialization` in `crates/paladin-storage/src/minio.rs`. No production consumer.
- The `serde_json` hits in `redis.rs` are core's `QueueConfig` and `QueueItem`; in `node_cache/redis.rs` they are `CachedDelta`. Every other use is a struct literal or `Default::default()`: `to_minio_config` in `src/config/settings.rs`, `src/config/setup/service_runner.rs`, `examples/node_result_cache.rs`, `crates/doc-examples/src/queue_worker.rs`, and the integration tests (`file_storage_integration_tests.rs`, `redis_queue_integration_test.rs`, `queue_port_contract.rs`).
- No production path `{:?}`-formats one of the four outward. The adapters (`MinioAdapter`, the Redis queue adapter, `RedisNodeCache`) hold their config privately.
- The fallback (stop before GREEN, return a blocking checkpoint) was not needed.

## Gate table on SHA-G2 `7b06e5e6b5fcb3f2f6ceabb2ef2a25e297b2d456`

All exit 0 (`CARGO_INCREMENTAL=0`):

| Gate | Result |
|------|--------|
| `cargo fmt --check` | no output |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `Finished dev profile`, no warning |
| `cargo test -p paladin-ai --features cli --lib config::` | `test result: ok. 286 passed; 0 failed` (N recorded, not a pass criterion) |
| `cargo test -p paladin-storage --features s3,redis-queue,redis-cache --lib` | `test result: ok. 384 passed; 0 failed` |
| `make api-surface` | `API surface unchanged` (4331 items) |
| `make check-gates` | version strings `0.11.0` (128 files), 0 stub-panic findings (627 files), 0 deprecated model literals (184 files) |
| `make doc-check` | `doc-check passed (ADR-0033)` |
| `pre-commit run --all-files` | every hook Passed; tree clean |

No coverage figure is claimed (D-00e).

## D-11 semver and sweep on SHA-S2 `0143b50d596eee90a3b185c2e9ce94ad0292a2a8`

`SHA-S2` is the head after the register commit; `git diff --name-only SHA-G2 SHA-S2 -- . ':!.planning'` is empty.

`SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh` (cargo-semver-checks 0.50.0, 2026-11-05T18:28Z to 18:33Z), exit 0: 12 packages, each `223 checks: 223 pass, 31 skip`; `semver: 12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`. N > 0 for every package, so the zero-check tripwire did not fire. Recorded as `### D-11 semver on SHA-S2`.

The 58-06 sweep list re-run on SHA-S2, all exit 0: `make check-gates` (same three OK lines), `make test-shell-guards` (`85 assertion(s) passed.`), `cargo fmt --check`, `make security` (`9 allowed warnings found`; `advisories ok, bans ok, licenses ok, sources ok`), `make api-surface` (unchanged, 4331 items), `pre-commit run --all-files` (all Passed), `./scripts/check-review-dispositions.sh` (`checked 51 finding(s) across 13 review file(s); 0 failure(s)`), and the strict register verifier (`register: checked 135 row(s) (R: 95, V: 40); fixed 52, waived 37, re-homed 46; 0 failure(s)`). Local sweep rows 18-34 hold these.

## Push: SHA-P2 `042df99b2c48bc249c206fdba6372e1e67b34e7d`

- `main` re-read through REST at 2026-11-05T18:34:52Z: `8b23cb4c55a10fcf0b08fab1af595e1f184e5f2e` (unmoved, D-02).
- Plain `git push origin claude/laughing-dirac-e0h2ax` (`f05e51006..042df99b2`), exit 0, no force, no `--no-verify`. The captured log shows only the ref update; the pre-push hooks printed nothing on success.
- `git ls-remote` equals SHA-P2; no open pull request for the branch; no `v0.11.1*` tag on the remote; `git diff --name-only SHA-S2 SHA-P2 -- . ':!.planning'` empty.
- Runs queued on SHA-P2, read once at 18:35:11Z and not waited for: `pre-commit` `42085634093` (in_progress), `ci.yml` `42085634184` (pending), `codeql.yml` `42085634162` (in_progress), `feature-flags.yml` `42085634381` (queued); all event `push`.
- One read of SHA-P's four runs: `codeql.yml` `42075372834` and `ci.yml` `42075372810` completed `cancelled` (superseded through the concurrency group); `pre-commit` `42075372712` and `feature-flags.yml` `42075372714` completed `success`. No SHA-P run concluded `failure`.
- `.continue-here.md` is rewritten (`plan: 16`, `status: paused`, `- SHA-P2:` beside `- SHA-P:`, resume condition on `head_sha=042df99b2...`, no watcher, no polling, `8b23cb4c5` constraint kept). The plan is stopped here.

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan-recorded rewrite, D-00d] `test_minio_config_serialization` rewritten in the RED commit**
- **Found during:** Task 3 (and step 0 of Task 1)
- **Issue:** the one in-tree serialise-then-deserialise consumer deserialised its own serialised `MinioConfig`, which the fix breaks (a missing-field error once the keys are not written).
- **Fix:** the RED commit `496df651e` rewrites the body to serialise the default, insert both keys into the object, deserialise, and read back endpoint, bucket and both keys. The name and the two `contains` assertions (`localhost:9000`, `paladin-files`) are kept, and it asserts nothing about key absence, so it passes before and after the fix.
- **Files modified:** `crates/paladin-storage/src/minio.rs`
- **Commit:** `496df651e`

**2. [Rule 3 - Blocking] Formatting fix folded into the Task 2 RED commit**
- **Found during:** Task 2 step 1
- **Issue:** `cargo fmt --check` flagged one `assert!` line in the new `redis.rs` test after the commit had already been made (the format check and the commit were separated by `;`).
- **Fix:** ran `cargo fmt -p paladin-storage` (it touched only `redis.rs`) and amended my own just-made, unpushed commit before any later commit. The RED commit id is therefore `84a3b32a7` (it was `ff198eee1` before the amend); the failing-test results were read before the amend and are unchanged by whitespace.
- **Files modified:** `crates/paladin-storage/src/redis.rs`
- **Commit:** `84a3b32a7`

**3. [Rule 3 - Blocking] Disk preconditions**
- **Found during:** Task 4 and Task 5 preconditions
- **Issue:** free space read 2.0 GB before the SHA-G2 gates and 1.7 GB before the semver run (the hooks' clippy run had recreated `target/debug/incremental` at 1.4 GB).
- **Fix:** deleted `target/debug/incremental` as the precondition allows (never `cargo clean`): 2031 MB to 3165 MB, then 1670 MB to 3047 MB. No build failed for space.

No Rule 1, 2 or 4 deviations. No authentication gate.

## Adjacent observations (names only, unregistered, unfixed)

A read-only re-run of the field-name grep over `src` and `crates` matched 35 lines. Outside this class, `OpenAIEmbeddingConfig.api_key` (`crates/paladin-llm/src/openai/embedding.rs`) and `MySqlConfig.database_url` (`crates/paladin-storage/src/mysql_content_repository.rs`) each derive `Debug` and `Clone` and no `Serialize`, so they print their field in a `Debug` rendering and cannot be serialised. `SqliteConfig.database_url` (`crates/paladin-storage/src/sqlite_content_repository.rs`) has the same derive shape. The other matched fields (the `paladin-llm` provider adapter configs, `paladin-web` controller DTOs, `user_service` DTOs, run and schedule `secret` fields) were not read. This is in F-58-07 for the maintainer to row, fix or waive (T-58-168, accepted). A docs candidate, not edited: `docs/src/user-guides/fault-tolerance.md` could say `NodeCacheConfig.redis_password` is never serialised.

## Documentation outcome

- Root `CHANGELOG.md` `[Unreleased]` `### Security`: two bullets (`M-B-51`, `M-B-52`) after the `M-B-50` bullet; `crates/paladin-storage/CHANGELOG.md`: one bullet naming both and the features `redis-queue`, `redis-cache`, `s3`.
- `MIGRATION.md` 11.1: `M-B-51` and `M-B-52` directly after `M-B-50` (four cells, no pipe in a cell). No 11.2 row and no Operators item. No changelog or MIGRATION line was removed or edited (`M-B-36`'s sentence stays; `M-B-50` already supersedes it).
- The step 0 hits equal the plan-time result, so no Operators item 14 was added.

## Known Stubs

None. No stub, TODO or placeholder was added; every test value is a fake `test-key-not-a-secret-r95-...` literal.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or trust-boundary schema change; the change removes secret output from `Serialize` and `Debug`.

## Hand-off

- **Plan 58-07 Task 1** must read `- SHA-P2:` (`042df99b2c48bc249c206fdba6372e1e67b34e7d`) and SHA-P2's runs (`head_sha=042df99b2...`) instead of SHA-P's; SHA-P's `ci.yml` and `codeql.yml` runs were cancelled by the concurrency group.
- **Plan 58-07 commit 3's MIGRATION section 11.8 paragraph, and plan 58-08's replacement of it**, name SHA-S2 (`0143b50d596eee90a3b185c2e9ce94ad0292a2a8`) and its date (2026-11-05) where they read SHA-S.
- **`M-B-51` and `M-B-52`** join `M-B-48` to `M-B-50` in the 0.11.1 credential-class pointer list of `### Library consumers: v0.11.0 → v0.11.1`.
- This plan edited none of plans 58-07 to 58-14. SHIP-08, SHIP-09 and SHIP-10 are not ticked here (plan 58-14 earns them after the tag).

## Self-Check: PASSED

- All eleven modified paths and this SUMMARY exist; the R-95 todo is under `.planning/todos/completed/` with `completed: 2026-11-05` as its first frontmatter line and is absent from `pending/`.
- All ten commit ids above resolve (`git cat-file -e`), RED precedes GREEN for each group (`git merge-base --is-ancestor`), and `git log --grep='^[a-z]*(58-16):' --name-only` names only paths in `files_modified`.
- The Task 1 to Task 5 automated verify blocks all printed their OK lines; `git ls-remote` equals SHA-P2; the strict register verifier reads `fixed 52, waived 37, re-homed 46; 0 failure(s)`.
