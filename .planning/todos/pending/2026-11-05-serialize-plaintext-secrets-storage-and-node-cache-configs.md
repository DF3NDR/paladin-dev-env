---
created: 2026-11-05T00:00:00Z
title: "Four config types still serialise a plaintext secret: NodeCacheConfig, RedisQueueConfig, MinioConfig, RedisNodeCacheConfig"
area: config-security
severity: major
files:
  - src/config/node_cache.rs:64
  - crates/paladin-storage/src/redis.rs:26
  - crates/paladin-storage/src/minio.rs:32
  - crates/paladin-storage/src/node_cache/redis.rs:110
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-95)
---

## Problem

Plan 58-15 fixed 56 WR-03, WR-04 and WR-09 (rows R-47, R-48, R-53): the secret fields of the types reachable from
`Settings` now carry `#[serde(skip_serializing)]`. Four same-class types were found by that plan's consumer gate and left
unchanged on purpose (a 58-15 prohibition), for this plan's sweep to disposition. Each derives `Serialize` with a plaintext
secret field and no `skip_serializing`:

- `NodeCacheConfig.redis_password` (`src/config/node_cache.rs:49` derive, `:64` field; not reachable from `Settings`);
- `RedisQueueConfig.redis_password` (`crates/paladin-storage/src/redis.rs:22` derive, `:26` field);
- `MinioConfig.access_key` and `secret_key` (`crates/paladin-storage/src/minio.rs:29` derive, `:32` and `:33` fields);
- `RedisNodeCacheConfig.redis_password` (`crates/paladin-storage/src/node_cache/redis.rs:103` derive, `:110` field).

A caller that serialises one of these values (for a log line, a config dump or a diff) writes the secret out.

## What is already true

Re-read on head 82d82cc5e (plan 58-06): the four derives and fields are as listed above. `NodeCacheConfig` and
`RedisNodeCacheConfig` have a manual `Debug` that renders the password as a placeholder. `RedisQueueConfig` and `MinioConfig`
derive `Debug` and so print the secret fields in a `Debug` rendering. The facade-side `QueueConfig` and `FileStorageConfig`
(the `Settings`-reachable types) were fixed by plan 58-15 (commits `e88fea3b9`, `d06702cf4`).

## What to do

Apply the 58-15 pattern to the four types: `#[serde(skip_serializing)]` with no `default`, a RED test that serialises a value
carrying a secret and asserts the key is absent, a manual redacting `Debug` where the derive prints the field, and a consumer
gate for any serialise-then-deserialise use. It changes the serialised output of published `paladin-storage` types, so it is
a behaviour change for a minor release (D-07), not a patch.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner
`v0.12.0 Phase 1` (D-09, a code item).

## Source

- Finding: `58-CI-EVIDENCE.md` F-58-06 "Residual same-class fields" and `58-15-SUMMARY.md`
- Register row: R-95 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
