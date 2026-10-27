---
phase: 51-adjustable-reasoning-effort
plan: 09
subsystem: trace-and-cache
tags: [reasoning-effort, trace-event, node-finished, node-cache-key, semver-register]
status: complete

requires:
  - phase: 51-01
    provides: "REASONING_REQUESTED_METADATA_KEY / REASONING_EFFECTIVE_METADATA_KEY, ReasoningEffort / ReasoningFallback serde spellings"
  - phase: 51-03
    provides: "adapters stamp the two metadata keys on the response (and the terminal streaming chunk) when an effort was in force"
  - phase: 51-07
    provides: "PaladinData.reasoning_effort / reasoning_fallback; run_metadata keeps the latest call's two reasoning keys"
provides:
  - "TraceEvent::NodeFinished.reasoning_requested / reasoning_effective (Option<String>, serde default, skipped when None)"
  - "Engine emission from each attempt's own run_metadata (Paladin arm) and structured.raw.run_metadata (structured arm)"
  - "Effort-aware node-cache fingerprint (effort and fallback appended only when set, own tag bytes), NODE_CACHE_KEY_VERSION still k1"
  - "MIGRATION 9.2 Y row and register-only allowlist entry for paladin-ai-core | TraceEvent"
affects: [51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "per-attempt reasoning rides as a fifth element of NodeDispatchResult beside cost, defaulted on every non-success path"
    - "an optional fingerprint input appends nothing when unset, so adding it moves no existing key"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/trace.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - crates/paladin-core/Cargo.toml
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/export/overlay.rs
    - crates/paladin-battalion/src/engine/cache_key.rs
    - crates/paladin-battalion/tests/export_golden.rs
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-eval/src/assertion.rs
    - crates/paladin-eval/tests/assertion_snapshots.rs
    - crates/paladin-llm/src/fallback.rs
    - src/application/services/run/events.rs
    - src/application/services/run/inspector.rs
    - src/application/services/run/stream_tests.rs
    - src/application/services/run/worker.rs
    - src/infrastructure/telemetry/herald_sink.rs
    - src/infrastructure/telemetry/otel_sink.rs
    - tests/cli/run_export_test.rs
    - tests/integration/otel_transport_test.rs
    - .cargo/semver-checks-allowlist.toml
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "The TraceEvent break is registered as a Y row with a register-only allowlist entry (the plan's acceptance criteria), although the existing NodeFinished.cost and RunFinished.halt_reason rows are N/A because TraceEvent postdates the CI-pinned v0.9.0 baseline. Plan 51-14's measurement against 0.10.1 decides whether the entry fires or is retired to N/A. Approved by the user at the tracer checkpoint."
  - "A DirectiveParse failure reports None for both fields (a failed attempt), although it still carries usage and cost."
  - "Effort and fallback sit behind tag bytes 0xE1 and 0xE2 as serde JSON, at the end of the Paladin config stream, so Budget(2048) never collides with a level and effort-only never equals fallback-only."

patterns-established:
  - "Pin the unset-input key as a literal captured before the change, then add the input."

requirements-completed: []
requirements-progress: [REASON-01, REASON-05, REASON-07]

duration: about 2h (cold worktree build, tracer checkpoint round trip)
completed: 2026-10-21
---

# Phase 51 Plan 09: Effort on the trace and in the node-cache key Summary

**Each Paladin or structured-output attempt now reports the reasoning effort it asked for and the effort it ran with on `TraceEvent::NodeFinished`, read only from its own response metadata, and a Paladin node's cache key moves with the agent's effort without disturbing any existing key.**

REASON-01, -05 and -07 advance but are not marked complete: documentation, probe-seeded model data and the measured semver lint land in plans 51-10 to 51-14. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: a Paladin attempt that ran at `high` emits `NodeFinished { reasoning_requested: high, reasoning_effective: high }`, with the `TraceEvent` break registered | `b162ac78` |
| 2 | An effort-aware node-cache key that leaves every existing key unchanged, and the surface record | `54460267` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("Approved") before Task 2 began. This SUMMARY is committed separately.

## What was built

- **`NodeFinished` fields.** `reasoning_requested` and `reasoning_effective`, `Option<String>`, after `cost`, with `#[serde(default, skip_serializing_if = "Option::is_none")]`. A row without the keys reads back with both `None`; a `None` pair emits neither key, so the export goldens are unchanged (not re-blessed).
- **Engine threading (`superstep.rs`).** `NodeDispatchResult` gains a fifth element, a private `AttemptReasoning { requested, effective }` (`Default`, `from_run_metadata`) that reads only the two core metadata-key constants. The Paladin arm fills it from `result.run_metadata`, the structured arm from `structured.raw.run_metadata`. Every other path uses `AttemptReasoning::default()`: Function, Battalion and Gate arms, the `Err` and `DirectiveParse` arms, the timeout arms of `race_attempt`, `with_incurred_spend`, skip, fail, `ChildHalted` and the semaphore arm. The cache-hit `NodeFinished` is a literal `None`. The attempt `NodeFinished` is built from the outer tuple. The agent's configured effort is never read, which is what makes the trace honest (REASON-05 prohibition).
- **Fan-out.** Every constructing literal names both fields as `None` (35 sites across 17 files, as the plan measured). Exhaustive-looking patterns that already use `..` (`overlay.rs`, `otel_sink.rs`) needed no change.
- **Register.** MIGRATION 9.2 `Y` row `paladin-ai-core | TraceEvent`, allowlist entry (`register-only (unmeasured: cargo-semver-checks absent on 2026-10-21; measured in plan 51-14)`, REASON-05), a comment in `crates/paladin-core/Cargo.toml` (no lint line touched: the expected lint `enum_struct_variant_field_added` is not in that table). `make check-migration-allowlist` exits 0.
- **Cache key (`cache_key.rs`).** `push_paladin_config` appends, only when set, tag `0xE1` plus the serde JSON of the effort, and tag `0xE2` plus the serde JSON of the fallback, after the stop words. Module and function rustdoc state that a provider-level default is not in the key. `NODE_CACHE_KEY_VERSION` is still `"k1"`.
- **CHANGELOG.** Root, `paladin-core` and `paladin-battalion` each have an `### Added` entry (the fields; the engine emission) and a `### Changed` entry (the BREAKING `NodeFinished` fields; cache keys of Paladin nodes with an effort set).

## TDD record

- **Task 1:** the behavior tests were written before the engine threading, but the first RED run failed at compile level only (the fields did not exist yet), so there was no runtime RED. Sensitivity was checked by mutation: replacing the Paladin arm's `AttemptReasoning::from_run_metadata(&result.run_metadata)` with `AttemptReasoning::default()` made `node_finished_carries_the_attempts_own_reasoning_metadata` fail; reverted. Accepted at the checkpoint.
- **Task 2:** the pinned-key test was added first with a placeholder, run on the **unmodified** composition to capture the literal, then filled in and re-run: it passed on the unmodified file. The two effort tests then failed (RED) and passed after the change (GREEN).

**Pinned key literal** (fixed fingerprint `from_canonical_bytes(b"pinned-graph")`, node `pinned`, input `input`, snapshot `x`/`y`, `PaladinData::default()` plus model `gpt-4` and system prompt `be brief`):

`k1:v6:1dd224dbea3091bd4619fb7d30eeefdcbbbca61d212248d3e8553464d1ab8e49:6:pinned:7b4e505e588bbc1e39d1fbe266d5d3607f8b886cccd20cac30fd060745c7dc30`

Test `unset_effort_key_is_the_pinned_literal` passed before and after the change.

## Tests added

- `paladin-ai-core` `trace`: `node_finished_reasoning_fields_are_skipped_when_none_and_default_on_read` (skip when `None`, default on read, `Some` round trip).
- `paladin-battalion` engine: `node_finished_carries_the_attempts_own_reasoning_metadata` (high/high, a `xhigh` to `high` substitution, an unstamped Paladin and a Function node both `None`), `node_finished_never_reads_the_agents_configured_effort`, `a_failed_attempt_emits_no_reasoning_fields`, `a_structured_attempt_reads_reasoning_from_the_raw_result` (stamped, unstamped, failed); the cache-hit trace test now also asserts `(None, None)`.
- `cache_key`: `unset_effort_key_is_the_pinned_literal`, `key_changes_when_the_agents_effort_changes`, `effort_only_and_fallback_only_keys_never_collide`.

## Verification

- `cargo test --offline -p paladin-ai-core --lib trace` 17 passed; `-p paladin-battalion --lib engine::superstep` 168 passed; `--lib engine::tests` 165 passed; `--lib cache_key` 12 passed and its 2 doc tests; `--test export_golden` 4 passed with the goldens unchanged; `paladin-eval` (56 plus snapshot tests), the root crate's `application::services::run` and `infrastructure::telemetry` tests and core `herald` tests pass.
- `cargo test --offline --workspace --all-features --no-run` compiles. Second full `cargo test --offline --workspace --all-features --no-fail-fast`: 71 test binaries ok, no failures.
- `cargo fmt --all --check` and `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings` clean. `make check-gates` (includes `check-migration-allowlist`) exits 0. `make api-surface` exits 0, "API surface unchanged" (4323 items), so `make api-surface-update` was not needed and `.project/current-exports.txt` is untouched: the baseline tracks the `paladin` facade's items, which `NodeFinished` struct-variant fields and the private cache-key tags do not alter.
- A first full-workspace run showed eleven failures in `application::services::run` (`cancel_tests`, `http_surface_tests`, `stream_tests`) under whole-workspace load; they pass in isolation (257 of 257), in a full `-p paladin-ai --lib` run (1670 passed), and in the second full workspace run. Plan 51-07 recorded the same load-sensitive family. They touch no logic this plan changed beyond adding `None` fields to test literals; counted flaky and unrelated.
- Not run: the full `make clean-code` chain and `make security` (no dependency added). `cargo-semver-checks` is absent in this container, so the allowlist lint is register-only until plan 51-14.
- Manual credential-handling review (security.instructions.md): no credential-bearing field reaches the trace. Only the two `paladin.reasoning_*` metadata keys are read, and their values are adapter-written `ReasoningEffort` `Display` spellings; no header, key, response body or prompt is read or logged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Test doubles extended in `crates/paladin-battalion/src/engine/test_support.rs`, a file outside `files_modified`**
- **Found during:** Task 1. A test needs a Paladin result carrying `run_metadata`, and the shared doubles always returned an empty map.
- **Fix:** `RecordingPaladinPort::set_run_metadata` and `SpendRecordingStructuredExecutor::recording_with_run_metadata` (a `raw_run_metadata` field on the executor). Test-only; no production change. Accepted at the checkpoint.
- **Commit:** `b162ac78`

### Notes against the plan's wording

- **Emission tests live in `engine/mod.rs`**, next to the existing `NodeFinished.cost` tests (the template the plan named), not in `superstep.rs`. The cache-hit assertion is in the `superstep.rs` tests.
- **`Y` row for `TraceEvent` versus the `N/A` precedent.** The plan's acceptance criteria require a `Y` row and an allowlist entry. The existing `TraceEvent::NodeFinished`/`RunFinished` rows (Phases 31, 38, 42) are `N/A` with no entry, because `TraceEvent` postdates the CI-pinned `v0.9.0` baseline, so the CI semver job cannot observe the break. The row, the allowlist comment and the `Cargo.toml` comment all say so. Plan 51-14's measurement against the published `0.10.1` decides whether the entry fires a lint or is retired to `N/A`. The user kept the `Y` registration at the tracer checkpoint.
- **`DirectiveParse` failures report `None`** for both fields, though they carry `cost`: the plan says a failed attempt leaves both `None`.
- **Task 1 `Cargo.toml`** received only a comment (no lint line added or removed), as in plan 51-07.
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used; commits used `--no-verify` as the wave allows.
- **Re-blessing:** none. The battalion export goldens, the eval assertion snapshots and the run export tests passed unchanged.

## Authentication Gates

None.

## Known Stubs

None. Deliberate limitations: the node-cache key does not include a provider-level default effort (`llm.<provider>.reasoning_effort`), because it is not on the agent; plan 51-13's guide is to say a changed provider default needs a new cache namespace or a version bump. The measured `cargo-semver-checks` lint for the `TraceEvent` entry is plan 51-14.

## Threat Flags

None. T-51-28 (a cached answer served across efforts) is mitigated by the effort and fallback in the fingerprint with distinct tag bytes and the pinned unset-key test; T-51-29 (a trace showing an effort that did not run) by reading only the attempt's own `run_metadata`, with a test that a configured-but-unstamped agent emits `None`; T-51-30 by reading only the two `paladin.reasoning_*` keys.

## Self-Check: PASSED

- Files: this SUMMARY and every file in `key-files.modified` present.
- Commits `b162ac78` and `54460267` present in `git log` on `worktree-agent-a098040fcc8bfe78b`.
