# ADR-0059: Legacy Battalion error, retry and timeout surfaces and the stringly PaladinError LLM variant leave the tree as a clean break in v0.11.0

## Status

Accepted

**Date:** 2026-10-09

**Phase:** 44 (Legacy Clean-Break Removal), plan 44-02; Code Conformance is measured at the closeout, plan 44-12.

## Supersedes

- ADR-0001 (`BattalionConfig` field set): its three legacy fields, `timeout_seconds`, `retry_policy` and `error_strategy`, are replaced by one `aegis: Aegis` policy bundle.
- ADR-0002 (`BattalionResult` field set): the element type of `node_errors` changes from the legacy `{ node_name, error }` summary to the structured `node_error::NodeError`; every other field of the set is unchanged.

## Context

Corpus rule X-03 ("deprecations allowed, removals are not") still governs the tree. ADR-0051 superseded it for Phases 31-33 only, and the Out of Scope entry in `PROJECT.md` says that a later removal records its own supersession and does not inherit that one. Phase 44 removes public API, so it needs its own record. This ADR is that record. ADR-0051 is the shape this record copies (a scope sentence, a "what still applies" paragraph, a version identity) and never its licence.

Milestone v0.11.0 carries a Legacy API clean-break bullet and the requirements LEGACY-01..04. Since v0.10 the legacy Battalion `RetryPolicy`, `ErrorStrategy` and `NodeError` summary, the whole-run timeout wrappers, `BattalionError::Timeout`, the stringly `PaladinError` LLM variant and the two legacy predicates `is_retryable()` / `is_terminal()` have sat beside their typed replacements: Aegis (`retry`, `timeout`, `on_error`), the structured `node_error::NodeError`, and `LlmFailure` with `Transience` (Phase 25). The framework is pre-1.0 and has a single downstream consumer, which is refactoring onto the replacements in one coordinated step, so carrying both sets forward would ship cruft for no reader.

The decisions below are the Phase 44 planning decisions D-00a..D-00f and D-01..D-21 (`44-CONTEXT.md`), the operator's selections in `44-DISCUSSION-LOG.md`, and the planner's resolutions of the ten Open Questions in `44-RESEARCH.md`. Where a resolution extends or refines a locked decision, the Decision section says so in its own sentence. No CONTEXT decision was re-opened. Plan 44-01 (the tracer) has already landed the additive half of the design (`BattalionConfig.aegis`, `with_aegis`, `validate_aegis`, the crate-private `aegis_attempt` runner, Campaign on the runner), and this ADR describes that design as it landed.

## Decision

### (a) X-03 is superseded for Phase 44 only (D-00a, D-00b, D-00c, D-00d, D-00e, D-00f)

**D-00a. X-03 is superseded for Phase 44 only**, and exactly for the removals recorded in subsections (b) to (i) below. Every other public API in the tree is still governed by X-03 exactly as written. A later removal records its own supersession; it does not inherit this one. This ADR does not inherit ADR-0051: ADR-0051 appears here only as the shape this record copies, never as its licence, and no Phase 44 removal cites it.

**What still applies.**

- **D-00b.** Every deliberate break gets a `MIGRATION.md` 9.2 row and, when the row is marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry per lint, row-level set-equal in both directions (`make check-migration-allowlist`). The lints are measured by the D-27 diagnostic at the closeout (plan 44-12), not trusted from a list; the Crate cell is the crates.io package name.
- **D-00c.** A clean break means no shim: no `#[deprecated]` bridge, no type alias, no re-export that keeps a removed name resolvable, no compatibility deserializer. Rows and allowlist entries are documentation for the downstream consumer's refactor, never a compatibility layer.
- **D-00d.** Gates are unchanged: the 82 percent workspace line-coverage floor (ADR-0006), `make clean-code`, `make api-surface` (with `make api-surface-update` and a CHANGELOG entry for the intentional surface change), `make security`, MSRV 1.88 and `make check-migration-allowlist`.
- **D-00e.** Version identity is **v0.11.0** everywhere: CHANGELOG `[Unreleased]`, this ADR, the register rows.
- **D-00f.** Phase 26's rule that middleware never retries itself, and the `RetryPolicy` ownership of retries, stand. The Phase 25 `Transience` taxonomy (`PaladinError::transience()`, `LlmError::transience()`) stays the one classification source.

### (b) `BattalionConfig.aegis` (D-01; Open Question 5)

**D-01. The three legacy fields and their builders go; one `aegis` field replaces them.** `BattalionConfig` loses `timeout_seconds`, `retry_policy` and `error_strategy` together with `with_timeout`, `with_retry_policy` and `with_error_strategy`. It gains `#[serde(default)] pub aegis: Aegis` (the existing policy bundle in `aegis.rs`, which already derives `Serialize`, `Deserialize` and `Default`), a `with_aegis(Aegis)` builder and a pure `validate_aegis` check beside the type. Formation, Phalanx and Campaign honour `aegis.timeout` and `aegis.retry`, and `aegis.on_error` where the pattern has a continue mode; `aegis.cache` is ignored by the legacy patterns and the field rustdoc says so. `validate_aegis` runs at each service's entry before any Paladin runs and rejects `retry.max_attempts == 0`, `ErrorHandlerSpec::Route` and `Custom` handlers (they need engine state and registries), `RetryPredicate::Custom` and zero-length durations; it ignores `cache`. **Open Question 5, resolved:** `RetryPredicate::Custom` on a legacy pattern would silently never retry (it needs the engine registry), so `validate_aegis` rejects it beside the handlers instead. The per-pattern contract is written once, in the `aegis` field rustdoc (plan 44-01), and the later plans implement against that text. **Reversibility: costly.** Every in-tree config literal, the Commander, the CLI templates and the downstream consumer's configs change shape, and a serialized `BattalionConfig` document loses three keys and gains one.

### (c) Per-attempt timeouts and one runner (D-02; Open Question 1; Open Question 8)

**D-02. The Aegis timeout bounds each Paladin attempt.** `TimeoutPolicy.run_timeout` is a per-attempt wall clock with the engine's meaning, and `idle_timeout` degrades to a per-attempt wall clock where the port reports no progress; the tighter bound wins and a tie is reported as `TimeoutKind::Run`. There is no whole-Battalion wall clock any more: a ten-step Formation may run ten times the figure, and the migration register and the guides say so. One crate-private module, `aegis_attempt` in `paladin_battalion`, holds the only `tokio::time::timeout` for Formation, Phalanx, Campaign and the Conclave (the runner `run_with_aegis` and the single-attempt primitive `attempt_once`). ChainOfCommand and Grove read no timeout and are unbounded unless the caller bounds `execute()` or sets per-call timeouts in their own ports; the Council keeps its own per-speaker 300 s bound (research Finding 5).

**Open Question 1, resolved as a planner resolution beside D-01 (plan 44-07, test `commander_builder_default_bounds_each_attempt`).** `BattalionConfig::new` follows D-01 literally and uses `Aegis::default()`, which arms no timeout, so a bare `BattalionConfig` is now unbounded. `CommanderBuilder`'s default configuration keeps its documented bound by setting a 300 s per-attempt `run_timeout`, so a bare `CommanderBuilder` is not silently unbounded. This is recorded in `MIGRATION.md` 9.1 row `M-B-06`.

**Open Question 8, resolved (plan 44-08).** The CLI keeps its own YAML `timeout_seconds` keys and maps them to `aegis.timeout.run_timeout`, so a CLI timeout now bounds each attempt and the template and doc comments say "per attempt". No new CLI schema key is introduced.

### (d) Timeout variants retired (D-03; Open Question 2; Open Question 3)

**D-03. The Commander wrapper, the Conclave timeout and `BattalionError::Timeout(u64)` are retired.** The Commander's own `tokio::time::timeout` around `execute()`, `ConclaveConfig.timeout_seconds` and `ConclaveConfig::with_timeout`, and `BattalionError::Timeout(u64)` are removed with the three named wrappers. `error_aggregation.rs` and the integration tests that matched `Timeout` migrate in the same commit series. **Reversibility: costly** (one more removed public variant with its own row).

**Open Question 2, resolved (extends D-03).** CONTEXT retires `BattalionError::Timeout` only. With that variant gone and the Conclave's whole-run wrapper removed, `ConclaveError::Timeout(u64)` has no producer, so it is removed too, together with its `From` arm, and `ConclaveError` gets its own 9.2 row. A Conclave expert attempt that times out is `PaladinError::Timeout` (Transient, so retried within `retry_attempts`), and an aggregator attempt that times out is `ConclaveError::AggregatorFailed` (plan 44-07).

**Open Question 3, resolved (plans 44-01, 44-04, 44-05).** On Formation and Campaign a timed-out final attempt surfaces as `BattalionError::Node(NodeError { source: NodeErrorSource::Timeout(TimeoutKind::Run | Idle), transience: Transient, .. })`, and any other fail-fast failure stays `BattalionError::PaladinError(String)`, the v0.10 contract with the least test churn. Phalanx keeps its collect-then-fail contract: with `on_error` of `None` it returns `BattalionError::AggregationError` whose message lists every failure's `NodeError` display (planner discretion under D-03). A timed-out attempt reports `PaladinError::Timeout` with the bound in whole seconds, never below one; the typed `NodeErrorSource::Timeout` is authoritative.

### (e) Commander bridging (D-04; Open Question 6)

**D-04. The Commander keeps bridging into `ConclaveConfig` and the default `ManeuverConfig`, sourced from `aegis`.** `aegis.timeout.run_timeout` becomes the Maneuver flow timeout (a whole-flow bound, which is Maneuver's own meaning) and, through the embedded `battalion_config`, the Conclave's per-attempt bound. `aegis.retry` sets the Conclave's attempt count; the count stays on `ConclaveConfig.retry_attempts`, derived and not duplicated as a policy.

**Open Question 6, resolved.** This refines D-04: D-04's literal text maps `aegis.retry.max_attempts` onto `ConclaveConfig.retry_attempts`, but `max_attempts` counts total attempts including the first (research Finding 4) while `retry_attempts` counts retries after the first, so a one-to-one copy would give every expert one attempt more than the policy names. The bridge therefore subtracts one: `Some(p)` sets `retry_attempts = p.max_attempts - 1` (saturating, clamped to 5 by `with_retry_attempts`), `None` keeps the `ConclaveConfig` default of 2, and `max_attempts: 3` means three expert attempts on every pattern. The adjustment is mirrored in `MIGRATION.md` 9.1 row `M-B-09` (plan 44-11) and tested by `commander_conclave_bridge_derives_retry_attempts` (plan 44-07). `ManeuverConfig.error_strategy` defaults to `maneuver::ErrorStrategy::FailFast`, because no `battalion::ErrorStrategy` is left to map from, and `CommanderBuilder::error_strategy(maneuver::ErrorStrategy)` stays as the explicit override.

### (f) Continue past failure (D-05, D-07; Open Question 4)

**D-05. "Continue past a failed Paladin" is `aegis.on_error`.** `None` fails fast. `Some(ErrorHandlerSpec::Absorb { .. })` continues and records the failure. The old `RetryThenContinue` is the composition of `aegis.retry` and `Absorb`, not a distinct mode. `fallback_delta` is a documented no-op for a string pipeline, so a Formation that continues passes an empty input to the next Paladin. `Route` and `Custom` are rejected by `validate_aegis` (subsection (b)).

**Open Question 4, resolved (plan 44-01).** Campaign never had a continue mode, so it honours `aegis.timeout` and `aegis.retry`, ignores `on_error`, and logs one warning per execution when `on_error` is set; no skip-subgraph semantics are invented. The Conclave always continues past failed experts.

**D-07. A run that continued reports `BattalionStatus::Completed`** with a non-empty `node_errors` and a non-zero `paladin_failure_count`, which is today's contract. No `PartialSuccess` variant is added; it is deferred (see Downstream Consumers).

### (g) `node_errors` retype (D-06, D-08)

**D-06. `BattalionResult.node_errors` is retyped from `Vec<battalion::NodeError>` to `Vec<paladin_core::platform::container::node_error::NodeError>`.** The element carries `node_id`, `attempt`, `transience` and `source`. The `node_id` is the Paladin's name; `source` is `llm_failure::to_node_error_source` (an `Llm` source for an `LlmFailure`, `Paladin { kind }` otherwise) or `Timeout(kind)`; `attempt` is the attempt that produced the final failure. One `NodeError` remains in the tree. Deserialization is strict: a v0.10 `BattalionResult` JSON whose `node_errors` is non-empty no longer deserializes, an empty or absent array still does (`#[serde(default)]` is kept), and no compatibility deserializer, untagged enum or alias exists (D-00c). The change is recorded as a 9.1 row, a 9.2 row and a 9.4 note for stored `BattalionResult` JSON. The exact wire shape is confirmed at the 44-06 operator checkpoint; plan 44-06 appends the dated outcome to this subsection. **Reversibility: one-way.** `BattalionResult` is a serialized value (the JSON herald, exported metadata, Citadel-persisted results), and once a consumer stores the structured element the old shape cannot be restored without another break.

**D-08. The heralds keep their per-node error block and render a richer line.** The JSON, Markdown and Table heralds render the node id, the attempt and the transience beside the source's display text; the JSON object shape changes with the retype and golden tests are updated, not bypassed. The CLI formatter in `src/application/cli/formatters/output.rs` needs no change: it only builds an empty `node_errors` fixture (research Finding 7).

### (h) Typed classification (D-09 to D-15; Open Question 7)

**D-09. The stringly LLM variant of `PaladinError` is removed.** Every erasure goes through `llm_failure::to_paladin_error`, which yields `LlmFailure { transience, status, provider, message }`. Every remaining construction, match arm, rustdoc mention and test literal is migrated or deleted in the same commit series (plan 44-03).

**D-10, D-11. The Conclave's predicate is `error.transience() == Transience::Transient`**, with no per-variant arms, and `Transience::Unknown` is not retried, matching the engine's default `TransientOnly` predicate. **Open Question 7, resolved:** D-10 is followed literally, so the predicate is Transient-only regardless of `aegis.retry.retry_on`; the policy supplies the backoff shape and the count, `retry_on` is honoured by Formation, Phalanx and Campaign, and the field rustdoc and guides state the split.

**D-12. One backoff implementation.** The Conclave drops its private `calculate_retry_delay` and uses `engine::retry::backoff_delay` and `wait_backoff` with the `aegis.retry` shape and `retry_attempts` as the count.

**D-13. `PaladinError::is_retryable()` and `PaladinError::is_terminal()` are removed**, each with a 9.2 row on the `paladin-ai-core | PaladinError` pair. **Reversibility: costly** (public inherent methods on a published type that the downstream consumer may call). The closest typed read for the second is `transience() == Transience::Permanent`, and the row says plainly that there is no equivalent for the old set.

**D-14. The circuit breaker counts `Transient` failures only**, the same rule as the Conclave and the engine, and moves to `transience()`.

**D-15. The sibling `HandoffError`, `PromptError` and `PlanningError` stringly LLM variants are untouched**; the requirement names `PaladinError` only.

**Behaviour deltas** (research "Behaviour deltas", recorded as 9.1 rows in plan 44-11). The Conclave now also retries `CircuitBreakerOpen`, `GarrisonError` of kind Storage or Tokenization, and `ArsenalError` of kind Timeout or TransportError, which the hand-written arms did not; `ExecutionError` was already not retried by the Conclave and stays so. The circuit breaker stops counting Permanent LLM failures (401, 400) and Unknown ones (`ProcessingError`) and stops counting `ExecutionError`, and starts counting `Timeout`, `CircuitBreakerOpen` and the Storage, Tokenization and transport kinds above. In production the breaker wraps only `generate()` and so sees only `LlmFailure`.

### (i) The legacy `retry` module (Open Question 10)

**Open Question 10, resolved (implied by D-12).** The public `paladin_battalion::retry` module (a third backoff implementation over the legacy `RetryPolicy`) and its facade re-export `pub use paladin_battalion::retry;` are removed in the same commit as the type. Its only caller was Formation, and the module cannot compile without the legacy type. The engine's `engine::retry` is unaffected. The committed API baseline loses one line (`make api-surface-update` plus a CHANGELOG entry), and the register row is `paladin-battalion | retry`.

### (j) The register and the migration-guide examples (D-16 to D-21; Open Question 9; Finding 10)

**D-16, D-17. One ADR, number 0059, version identity v0.11.0.** This ADR records both the Phase 44-only supersession and the replacement design, with the PROMOTION.md heading set. **D-18.** ADR-0001 and ADR-0002 are marked `Superseded` with a dated prose line naming this ADR, and `PROJECT.md` gains the Key Decisions row and the supersession notes.

**D-19. One 9.2 row per `crate | type` pair.** The set-equality gate keys a row on the first backticked identifier of its Type cell (research Finding 10), so a single `battalion module` row would collapse to the pair `paladin-ai-core | battalion` and could not own three types. The legacy `RetryPolicy`, `ErrorStrategy` and `NodeError` therefore get three separate rows, consistent with D-19's one-row-per-pair rule, each pairing one-to-one with its `struct_missing` or `enum_missing` entry. The other rows are `BattalionConfig`, `BattalionResult` (register-only if the diagnostic fires no lint for a field retype), `BattalionError`, `ConclaveError`, `ConclaveConfig`, `PaladinError` and `paladin-battalion | retry`. Behavioural changes (D-02, D-06, D-10, D-14, the `max_attempts` meaning, the Maneuver default) go in 9.1 starting at `M-B-06`, with a 9.4 note for stored `BattalionResult` JSON, and CHANGELOG `[Unreleased]` gets Removed and Changed blocks citing this ADR and LEGACY-01..04 (D-21).

**D-20. Examples and mdBook pages show the Aegis equivalent** of every removed setting (plans 44-08 and 44-09) and double as the downstream consumer's migration guide; every `[[example]]` target still builds. **Open Question 9, resolved (plan 44-09):** the fictional `battalion:` YAML in `getting-started/configuration.md`, which no Rust type reads, is replaced by a pointer to the programmatic `BattalionConfig.aegis`, and no Aegis YAML schema is invented.

## Considered Options

- Drop the three fields with no replacement (rejected: the legacy patterns would run untimed and fail-fast, and there would be no way to configure retry or timeout).
- A timeout-only `TimeoutPolicy` field instead of the full `Aegis` bundle (rejected: leaves retry and continue-past-failure without a home and forks the policy vocabulary).
- A whole-Battalion wall clock sourced from `aegis.timeout.run_timeout` (rejected: contradicts the engine's per-attempt meaning of the same field; one meaning everywhere was the point).
- A `continue_on_error: bool` on `BattalionConfig` (rejected: a second knob beside `aegis.on_error` for the same thing).
- Dropping `node_errors` from `BattalionResult` (rejected: the heralds and the CLI report per-node failures).
- Renaming the legacy summary type and keeping its `{ node_name, error }` shape (rejected: leaves two `NodeError`-shaped types in the tree).
- A `BattalionStatus::PartialSuccess` variant (deferred: a new serialized variant is a new capability, not part of this removal).
- Keeping explicit predicate arms minus the stringly one (rejected: a second classification table beside `transience()`).
- Retrying `Transience::Unknown` in the Conclave (rejected: parity with the engine's `TransientOnly` default).
- Keeping the Conclave's private backoff (rejected: two backoff implementations in one crate).
- Keeping `is_retryable()` and `is_terminal()` as deprecated or untouched predicates (rejected: the operator chose the wider blast radius for one classification source).
- Counting `Unknown` toward the circuit breaker (rejected: parity with the Conclave and the engine).
- Two separate ADRs, one for the supersession and one for the design (rejected: the design is the reason for the supersession and the two would drift).
- Leaving ADR-0001 and ADR-0002 `Accepted` and citing them from this ADR (rejected: two live ADRs would then answer one question).
- One 9.2 row per removed item (rejected: finer than the allowlist's set-equality key).
- Deleting legacy example lines without showing the Aegis equivalent (rejected: the examples are the downstream consumer's migration guide).
- One `battalion module` 9.2 row for the three legacy types (rejected: the gate takes the first backticked identifier of the Type cell, research Finding 10).
- A compatibility deserializer for the old `node_errors` shape (rejected: forbidden by D-00c).
- Inline retry loops per pattern instead of one runner (rejected: four copies of the timeout and backoff logic to keep in step).
- A facade re-export of `aegis` (not needed: the examples import `paladin_core` directly).

## Code Locations

- `.planning/decisions/0059-legacy-clean-break-removal.md`, `.planning/decisions/0001-battalion-config.md`, `.planning/decisions/0002-battalion-result.md`, `.planning/decisions/PROMOTION.md`, `.planning/PROJECT.md` - the decision record and its index entries.
- `crates/paladin-core/src/platform/container/battalion/mod.rs`, `crates/paladin-core/src/platform/container/battalion/conclave.rs`, `crates/paladin-core/src/platform/container/aegis.rs`, `crates/paladin-core/src/platform/container/paladin_error.rs` - `BattalionConfig`, the legacy types, `BattalionResult.node_errors`, `BattalionError`, `ConclaveConfig`, `ConclaveError`, the Aegis contract and `PaladinError`.
- `crates/paladin-ports/src/output/paladin_port.rs`, `crates/paladin-eval/src/runner.rs` - rustdoc that names the removed LLM variant.
- `crates/paladin-battalion/src/aegis_attempt.rs`, `crates/paladin-battalion/src/formation_service.rs`, `crates/paladin-battalion/src/phalanx_service.rs`, `crates/paladin-battalion/src/campaign_service.rs` - the runner and the three legacy patterns.
- `crates/paladin-battalion/src/commander.rs`, `crates/paladin-battalion/src/conclave_execution_service.rs`, `crates/paladin-battalion/src/error_aggregation.rs`, `crates/paladin-battalion/src/llm_failure.rs`, `crates/paladin-battalion/src/lib.rs`, `crates/paladin-battalion/src/retry.rs` - the Commander bridges, the Conclave on the typed taxonomy, and the deleted legacy module.
- `crates/paladin-herald/src/json_herald.rs`, `crates/paladin-herald/src/markdown_herald.rs`, `crates/paladin-herald/src/table_herald.rs` - the `node_errors` rendering.
- `src/infrastructure/resilience/circuit_breaker.rs`, `src/application/services/battalion/mod.rs`, `src/application/services/paladin/paladin_execution_service.rs`, `src/application/services/paladin/temperature_service.rs` - the breaker, the facade re-export and rustdoc mentions.
- `src/application/cli/commands/battalion.rs`, `src/application/cli/config/battalion_config.rs`, `src/application/cli/templates/battalion_template.rs` - the CLI's mapping of its own YAML timeout keys.
- `examples/campaign_workflow.rs`, `examples/commander_auto.rs`, `examples/commander_basic.rs`, `examples/commander_full_config.rs`, `examples/commander_with_metadata_export.rs`, `examples/conclave_expert_panel.rs`, `examples/formation_sequential.rs`, `examples/maneuver_nested_flow.rs`, `examples/phalanx_parallel.rs`, `crates/doc-examples/src/orchestration.rs` - the migration-guide examples.
- `docs/src/appendix/battalion-vision-support.md`, `docs/src/appendix/cli-usage.md`, `docs/src/appendix/conclave-pattern.md`, `docs/src/contributing/architecture-decisions.md`, `docs/src/deployment/production.md`, `docs/src/getting-started/configuration.md`, `docs/src/user-guides/battalion-patterns.md`, `docs/src/user-guides/fault-tolerance.md`, `docs/src/user-guides/orchestration.md`, `docs/src/user-guides/paladin-agents.md`, `docs/src/user-guides/paladin-configuration.md`, `docs/src/user-guides/tool-integration.md`, `docs/DEMOS.md` - the mdBook pages and demo notes.
- `tests/helpers/mock_paladin_port.rs`, `tests/integration/battalion/campaign_integration_test.rs`, `tests/integration/battalion/formation_integration_test.rs`, `tests/integration/battalion/load_test.rs`, `tests/integration/battalion/phalanx_integration_test.rs`, `tests/integration/battalion_campaign_integration_test.rs`, `tests/integration/battalion_chain_of_command_integration_test.rs`, `tests/integration/battalion_herald_end_to_end_test.rs`, `tests/integration/commander_error_paths_test.rs`, `tests/integration/commander_integration_tests.rs` - the migrated integration tests and helpers.
- `tests/unit/battalion/campaign_service_tests.rs`, `tests/unit/battalion/formation_tests.rs`, `tests/unit/circuit_breaker_test.rs`, `tests/unit/paladin_error_test.rs`, `tests/unit/paladin_execution_service_test.rs`, `tests/legacy_removal_guard.rs` - the migrated unit tests and the source-tree guard.
- `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `.project/current-exports.txt`, `CHANGELOG.md`, `crates/paladin-core/CHANGELOG.md`, `crates/paladin-battalion/CHANGELOG.md`, `crates/paladin-herald/CHANGELOG.md`, `Cargo.toml`, `crates/paladin-core/Cargo.toml`, `crates/paladin-battalion/Cargo.toml` - the register, the allowlist, the API baseline, the changelogs and the measured semver lint lines.

## Code Conformance

must change

Until plan 44-12 measures it. The removals, the typed taxonomy move and the register land across plans 44-03 to 44-11, and plan 44-13 pins them with `tests/legacy_removal_guard.rs`. Plan 44-12 reconciles the Code Locations list above against the shipped diff, replaces this verdict with `conforms`, and names the proving tests (among them `commander_campaign_honours_aegis_per_attempt_timeout_end_to_end`, `battalion_result_rejects_v0_10_node_errors_shape`, `conclave_retry_predicate_is_transient_only`, `circuit_breaker_counts_transient_failures_only` and the guard).

## Downstream Consumers

- **Phase 46** - the v0.10 to v0.11 `MIGRATION.md` upgrade guide and the mdBook, which build on the 9.1 and 9.2 rows written here.
- **Phase 47** - the v0.11.0 release, which publishes the smaller public surface and the changelog blocks.
- **The downstream consumer's coordinated refactor** - the single consumer that moves its `BattalionConfig` literals, `node_errors` readers and `is_retryable()` callers in one step; the examples are its migration guide (D-20).
- **A later phase for the deferred items** - moving the sibling `HandoffError`, `PromptError` and `PlanningError` stringly variants onto a typed source (D-15), and a `BattalionStatus::PartialSuccess` variant if callers need to match on "completed with failures" without inspecting `node_errors` (D-07).
