completed: 2026-10-24
---
created: 2026-10-11T18:00:00Z
title: Reject removed v0.10 Battalion and Conclave config keys with an error naming the key
area: battalion
severity: major
files:
  - crates/paladin-core/src/platform/container/battalion/mod.rs
  - crates/paladin-core/src/platform/container/battalion/conclave.rs
  - .planning/decisions/0059-legacy-clean-break-removal.md
  - MIGRATION.md
  - CHANGELOG.md
owner: repo maintainer
source: Phase 44 code review, 44-REVIEW.md WR-03
recheck_by: before the Phase 47 v0.11.0 publish
decision_status: operator leans to option B (reject); confirm before implementing
---

## Problem

Phase 44 removed `timeout_seconds`, `retry_policy` and `error_strategy` from `BattalionConfig`
and `timeout_seconds` from `ConclaveConfig`. A stored v0.10 document that still carries those keys
deserializes without error: serde ignores the unknown keys and `aegis` becomes `Aegis::default()`,
which has no timeout and no retry. A run that was bounded in v0.10 is silently unbounded in
v0.11. This is the behaviour Phase 44 chose (ADR-0059 D-00c: no compatibility deserializer,
untagged enum or alias) and it is documented in the MIGRATION.md 9.2 `BattalionConfig` row and the
CHANGELOG, but nothing tells the user at load time.

## Options

- **A. Keep ignoring the keys** and give the behaviour a prominent callout in the Phase 46 v0.10 to
  v0.11 upgrade guide.
- **B. Reject the keys** with a deserialization error that names each removed key and its Aegis
  replacement, e.g. "`timeout_seconds` was removed in v0.11; set `aegis.timeout.run_timeout`
  (MIGRATION.md 9.1 M-B-06)".

## Operator position (2026-10-11)

The operator leans to **B**. Paladin currently has one downstream consumer, so a loud failure costs
one project migration and makes the break impossible to miss.

## Orchestrator assessment

Agrees with B, with three implementation constraints:

1. Reject only the named removed keys. Do not add `#[serde(deny_unknown_fields)]`: that rejects
   every unknown key and would also break forward compatibility for keys added later.
2. B does not reintroduce a compatibility shim, so it is arguably consistent with D-00c's intent.
   It does change behaviour Phase 44 recorded ("the removed keys are ignored"), so amend ADR-0059
   and update the MIGRATION.md 9.1/9.2 rows and the CHANGELOG in the same change.
3. The CLI's own YAML `timeout_seconds` key is a separate, still-supported schema that maps onto
   `aegis.timeout`. It must keep working.

## Solution sketch (if B is confirmed)

- A per-key guard on `BattalionConfig` and `ConclaveConfig`, for example a private
  `#[serde(default, skip_serializing, deserialize_with = ...)]` field per removed key whose
  deserializer always errors with the replacement text, or a custom `Deserialize` wrapper.
- Tests: each removed key, alone and combined, fails with a message naming it; a v0.11 document and
  a document with an unrelated unknown key still load; the round-trip test still passes.
- Re-run `cargo test --test legacy_removal_guard` and `make check-gates`. The new error strings name
  removed items, so they may need the guard's allowed-context treatment.
- Could be a `/gsd-quick` task or folded into Phase 46 or 47.

Resolved by Phase 46 plan 46-08 (option B): the removed keys are refused at load via a private wire struct in `battalion/legacy_keys.rs`; see ADR-0059's 2026-10-24 amendment and MIGRATION.md M-B-06.
