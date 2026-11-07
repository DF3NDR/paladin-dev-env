---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 10
subsystem: paladin-llm, paladin-storage, paladin-notifications
tags: [rustdoc, intra-doc-links, adr-0033, cirel-08, d-11]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-01 and 55-04 (new rustdoc in storage and llm, measured here) and 55-08 (the fix method, copied)"
provides:
  - "paladin-llm, paladin-storage and paladin-notifications read zero rustdoc diagnostics under both ADR-0033 bars with no crate-level suppression"
  - "before and after counts per bar, class and file, for plan 55-11's ADR-0033 amendment and plan 55-13's evidence"
affects: [55-11, 55-13]

tech-stack:
  added: []
  patterns:
    - "A target that exists under one feature set only, a private or pub(crate) item, or a module reachable only through a feature, is a plain code span; an unconditional paladin-ports / paladin-core item gets its explicit path; a crate-local public item gets a crate:: path"
    - "The workspace default bar unifies features across members, so it can resolve a link (anthropic, deepseek, sqlite, redis-queue) that a per-crate `cargo doc -p` run cannot: run the per-crate default, all-features and no-default-features forms as well as the two workspace bars"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/**/*.rs (18 files in total, doc comments only)
    - crates/paladin-storage/src/lib.rs
    - crates/paladin-storage/src/**/*.rs (11 files in total, doc comments only)
    - crates/paladin-notifications/src/lib.rs

key-decisions:
  - "D-11 applied as locked: the three suppression lines removed, every diagnostic fixed where written, no doc sentence deleted, no rustdoc allow or expect left at any level in the three crates"
  - "Unconditional ports targets inside the scheduler-gated module (SchedulerPort, JobInfo, JobSpec::metadata, get_job_status, get_job_info) got explicit paladin_ports paths instead of code spans: the module's docs exist only when the module is compiled, and the targets are unconditional, so both bars accept the link. Genuinely gated or private targets are code spans"

requirements-completed: [CIREL-08]

duration: 40min
completed: 2026-11-01
---

# Phase 55 Plan 10: paladin-llm, paladin-storage, paladin-notifications rustdoc links Summary

**The three crates dropped from 117 / 27 / 3 default-bar rustdoc diagnostics (105 / 28 / 0 under all-features) to 0 under both CI bars and no longer carry a crate-level suppression: explicit paths for unconditional targets, plain code spans for feature-gated, private and `pub(crate)` ones, and rustdoc is the only thing that changed.**

Plan-start SHA: `7b379ce0382e5151086376a8d5204fe69b1406e2`

## Performance

- **Duration:** about 40 minutes
- **Tasks:** 2 of 2 (Task 1 is the tracer), three crate commits
- **Files modified:** 19 in `paladin-llm`, 11 in `paladin-storage`, 1 in `paladin-notifications`; all doc-comment lines plus the three removed attribute lines

## Accomplishments

- `#![allow(rustdoc::broken_intra_doc_links)]` is gone from `crates/paladin-llm/src/lib.rs`, `crates/paladin-storage/src/lib.rs` and `crates/paladin-notifications/src/lib.rs`. `grep -rnE 'allow\(rustdoc|expect\(rustdoc'` over the three `src` trees prints nothing.
- Diagnostics were located from the printed span when there was one and from the printed link text otherwise (a throwaway parser and apply script kept in the session scratchpad, not committed), applied by a label-to-path table, then re-checked by doc builds.
- The facade (plan 55-11) is the only suppressed crate left.

## Task Commits

1. **Task 1 (tracer): paladin-llm** - `8ea9f1357` (docs). Both workspace bars green on this head; tracer `<verify>` re-run end to end before expanding (auto mode active), passed.
2. **Task 2a: paladin-storage** - `d53d4233c` (docs). Both workspace bars green on this head.
3. **Task 2b: paladin-notifications** - `23b6426d8` (docs). Both workspace bars green on this head.

Order is llm, storage, notifications, as D-11 requires.

## Measured before (paladin-llm)

Suppression line removed in the working tree; `cargo doc --workspace --no-deps --offline` and `cargo doc --workspace --all-features --no-deps --offline` (second without `-D`, so it lists everything).

| Bar | Total | Unresolved link | Redundant explicit link |
|-----|-------|-----------------|-------------------------|
| default | 117 | 117 | 0 |
| all-features | 105 | 105 | 0 |

- Matches RESEARCH at `a78338ddb` (117 / 105), so 55-04's edits did not move the count.
- 16 of 117 carried a source span; 101 had none and were located by matching the printed line against the sources (every one matched exactly one line).
- Per file (default bar): `fallback.rs` 37, `lib.rs` 13, `cadence.rs` 11, `services/commissary.rs` 9, `provider_factory.rs` 8, `window.rs` 8, `redaction.rs` 5, `registry/mod.rs` 4, `sampling.rs` 4, `usage_reconcile.rs` 4, `http_status.rs` 3, `openai/mod.rs` 3, `pricing.rs` 3, `mock.rs` 2, `config/bridge.rs` 1, `config/vision.rs` 1, `error.rs` 1.
- Default bar only (14): `crate::openai::vision` (`config/vision.rs:5`); `kimi`, `qwen`, `grok`, `ollama`, `openai_compatible`, `gemini` adapter and config links at `lib.rs:16-21` (12); `openai::OpenAIEmbeddingAdapter` (`lib.rs:24`).
- All-features only (2): `super::engine` (`compat/types.rs:4`) and `adapter` (`gemini/mod.rs:6`), both inside provider-gated modules.
- Both bars (103): bare names of crate-local, `paladin-ports` and `paladin-core` items linked from a module's own `//!` docs.

## Measured after (paladin-llm)

- Head `8ea9f1357`: `cargo doc --workspace --no-deps --offline` prints no `warning:` line (117 -> 0); `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --offline` exits 0 (105 -> 0).
- `cargo doc -p paladin-llm --no-deps --offline` 0 warnings, after finding and fixing four more (the `anthropic` and `deepseek` rows at `lib.rs:14-15`, hidden in the workspace default run because other members enable those features).
- `cargo test -p paladin-llm --doc --offline`: 26 passed, 0 failed. `cargo fmt --check` clean.
- The plan's diff check (`git diff <plan-start SHA> -- crates/paladin-llm/src`, doc-comment lines excluded) prints only the removed attribute line.

## Measured before (paladin-storage)

| Bar | Total | Unresolved link | Redundant explicit link |
|-----|-------|-----------------|-------------------------|
| default | 27 | 27 | 0 |
| all-features | 28 | 28 | 0 |

- Matches RESEARCH (27 / 28). 5 of 27 carried a span; the rest were located by text (two scheduler lines matched two sites each and were assigned to the `//!` occurrence).
- Per file (default bar): `cadence/resilient.rs` 8, `lib.rs` 4, `cadence/contract_tests.rs` 3, `run_trace/contract_tests.rs` 4, `cadence/mod.rs` 2, `treasury/notice_contract_tests.rs` 2, `cron.rs` 1, `run_trace/retention.rs` 1, `treasury/contract_tests.rs` 1, `waypoint/retention.rs` 1. All-features adds `scheduler.rs` 7 and drops `lib.rs`, `cron.rs` and one `resilient.rs` row.
- Default bar only (6): `crate::cadence::redis::RedisCadence`, `crate::scheduler::TokioCronSchedulerAdapter`, and the `mysql_content_repository`, `minio`, `cadence::redis`, `scheduler` rows of the `lib.rs` feature table.
- All-features only (7), all in `scheduler.rs`: `SchedulerPort`, `JobInfo`, `JobRegistration`, `JobSpec::metadata`, `SchedulerPort::get_job_status`, `SchedulerPort::get_job_info`, `TokioCronSchedulerAdapter::record_job_run`.

## Measured after (paladin-storage)

- Head `d53d4233c`: default bar 0 (27 -> 0), all-features bar exit 0 (28 -> 0).
- Per-crate runs: default, `--all-features`, `--no-default-features` and the explicit `sqlite,mysql,postgres,s3,redis-queue,redis-cache,redis-cadence,scheduler` set all read 0, after fixing three more the workspace run had hidden (`sqlite_content_repository`, `sqlite_user_repository`, `redis` at `lib.rs:10,13` and `cadence/mod.rs:6`).
- `cargo test -p paladin-storage --doc --offline`: 4 passed, 0 failed. `cargo fmt --check` clean. Diff check prints only the removed attribute line.

## Measured before (paladin-notifications)

| Bar | Total | Unresolved link |
|-----|-------|-----------------|
| default | 3 | 3 |
| all-features | 0 | 0 |

The three rows are `email_notification_adapter`, `push_notification_adapter`, `system_notification_adapter` in the `lib.rs` feature table (`lib.rs:9-11`). Matches RESEARCH (3 / 0).

## Measured after (paladin-notifications)

- Head `23b6426d8`: default bar 0 (3 -> 0), all-features bar exit 0 (0 -> 0); `cargo doc -p paladin-notifications` reads 0 under default, `--all-features` and `--no-default-features`.
- `cargo test -p paladin-notifications --doc --offline`: 0 tests (the crate has no doctests); the command passes. `cargo fmt --check` clean. Diff check prints only the removed attribute line.
- `make api-surface` after all three commits: `API surface unchanged` (4325 items, nightly-2026-09-20).

## How each class was fixed

- Crate-local public items named from a module's own `//!` docs (`CadenceLlmAdapter`, `FallbackLlmAdapter::*`, `Commissary::*`, `WindowFallbackPolicy::*`, `redact_credentials`, and so on): explicit `crate::<module>::Item` target, label kept. This is the same finding 55-08 recorded: a module whose docs are also attached from `lib.rs` resolves its links in the parent scope, so a bare imported name fails.
- `paladin-ports` items (`LlmError`, `LlmPort`, `CadencePort`, `TraceEmitter`, `RunTracePort`, `SchedulerPort`, ...): explicit `paladin_ports::output::<module>::Item`. `paladin-core` items (`TraceEvent`, `TokenUsage`): explicit `paladin_core::platform::container::<file>::Item`.
- Plain code spans: provider modules behind their own feature (`anthropic`, `deepseek`, `kimi`, `qwen`, `grok`, `ollama`, `openai_compatible`, `gemini`), `openai::OpenAIEmbeddingAdapter`, `crate::openai::vision`, the backend rows of the storage feature table (`sqlite_*`, `mysql_content_repository`, `minio`, `redis`, `cadence::redis`, `scheduler`), `crate::cadence::redis::RedisCadence`, `crate::scheduler::TokioCronSchedulerAdapter`, the three notification adapter modules, `super::engine` and `adapter`; and private or `pub(crate)` items (`provider_registry`, `check_max_tokens`, `usage_reconcile::reconcile`, `slack`, `JobRegistration`).
- Links already written in `[`text`](target)` form (`retry_after`, `generate_stream`, `create`, `create_from_config`, `get_default_provider`, `list_available_providers`) kept their text; only the target was made explicit.
- `provider_factory.rs:538` `[`list_available_providers`]` became `(Self::list_available_providers)`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Workspace default bar hides links a per-crate run flags**
- **Found during:** Task 1 and Task 2 per-crate re-runs
- **Issue:** the plan-start measurement (`cargo doc --workspace`) resolves `anthropic::*`, `deepseek::*` (llm) and `sqlite_*`, `redis` (storage) because other workspace members enable those features. `cargo doc -p <crate>` and `--no-default-features` do not, so 4 llm and 3 storage links would have been red for any consumer building the crate alone.
- **Fix:** all seven became code spans in the same commits, which keeps the crates clean under the workspace bars and the per-crate forms.
- **Files modified:** `crates/paladin-llm/src/lib.rs`, `crates/paladin-storage/src/lib.rs`, `crates/paladin-storage/src/cadence/mod.rs`
- **Commits:** `8ea9f1357`, `d53d4233c`

**2. [Judgment, within D-11] Explicit paths instead of code spans for the unconditional scheduler-module targets**
- The plan's truths list `SchedulerPort`, `JobInfo`, `JobSpec::metadata`, `get_job_status`, `get_job_info`, `record_job_run` among the code-span set. Those links live in `scheduler.rs`, a module compiled only under the `scheduler` feature, and their targets (`paladin_ports` items and a `pub` method of the module's own adapter) are unconditional, so an explicit path resolves whenever the doc exists and is absent from the default bar entirely. They are live links; `JobRegistration` (a private struct) and everything else gated or private is a code span. Both bars verified green.
- **Commit:** `d53d4233c`

**Total deviations:** 1 auto-fixed (Rule 3), 1 judgment call. No impact on the public API; `make api-surface` unchanged.

## Known Stubs

None.

## Issues Encountered

- `cargo doc -p paladin-llm --no-default-features --no-deps --offline` still reports 6 warnings. Not a CI bar (the bars are the default and all-features workspace runs, ADR-0033); they are links from always-compiled docs (for example `openai::OpenAIAdapter` and `mock::MockLlmAdapter` in the `lib.rs` feature table) to the two default-on modules. Left as written: converting them would turn live links into spans for a configuration no bar and no published docs.rs build uses. Noted for the 55-13 residuals list.
- No disk pressure; no `target/` cleanup was needed. All cargo runs used `CARGO_INCREMENTAL=0` and `--offline`.

## Threat Flags

None. Rustdoc comment text only; T-55-25 is mitigated: no rustdoc suppression exists at any level in the three crates, no doc text was deleted, and both workspace bars were green on every crate commit.

## Next Phase Readiness

- Plan 55-11 (facade, ADR-0033 amendment) can reuse the method. The ADR amendment rows are: `paladin-llm` 117 -> 0 default / 105 -> 0 all-features (all unresolved link); `paladin-storage` 27 -> 0 / 28 -> 0 (all unresolved link); `paladin-notifications` 3 -> 0 / 0 -> 0 (all unresolved link). Redundant explicit links: 0 in all three crates.

## Self-Check: PASSED

- Commits `8ea9f1357`, `d53d4233c` and `23b6426d8` exist on `claude/laughing-dirac-e0h2ax`, in llm, storage, notifications order.
- None of the three crates has a rustdoc `allow` or `expect`.
