# Phase 44: Legacy Clean-Break Removal - Context

**Gathered:** 2026-10-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Four superseded surfaces leave the tree, with every removal recorded: the legacy
`battalion::RetryPolicy`, `battalion::ErrorStrategy` and `battalion::NodeError` (LEGACY-01); the
Formation / Phalanx / Campaign tokio timeout wrappers, so that Aegis timeouts are the only timeout
path for those patterns (LEGACY-02); `PaladinError::LlmError(String)`, with
`conclave_execution_service.rs`'s message-matching retry check moved onto the typed
`LlmFailure` / `Transience` taxonomy (LEGACY-03); and the bookkeeping for all of it — an ADR
recording the X-03 supersession, a `MIGRATION.md` §9.2 row and a `.cargo/semver-checks-allowlist.toml`
entry per deliberate break, and updated examples and docs (LEGACY-04).

The same-named Aegis `RetryPolicy` (`crates/paladin-core/src/platform/container/aegis.rs`), the
structured engine `node_error::NodeError` and the Flow `maneuver::ErrorStrategy` are **untouched**.
The sibling `HandoffError::LlmError(String)`, `PromptError::LlmError(String)` and
`PlanningError::LlmError(String)` variants are untouched (see Deferred Ideas).

**In scope by discussion (D-03, D-15):** the Commander's own `execute()` timeout wrapper,
`ConclaveConfig::with_timeout` and `BattalionError::Timeout(u64)` get the same treatment as the
three named patterns, and the legacy predicates `PaladinError::is_retryable()` /
`PaladinError::is_terminal()` are removed with the circuit breaker moved onto `transience()`.

**Not in this phase:** the v0.10 → v0.11 `MIGRATION.md` upgrade *guide* and the Treasurer mdBook
page (Phase 46 owns the guide; this phase owns the §9.1/§9.2 rows), the v0.11.0 crates.io release
(Phase 47), any new Battalion capability, and the pending Phase 41 work (unrelated files).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by ADR-0051, PROMOTION.md and Phases 38-43 — cited, not re-asked)

- **D-00a:** X-03 ("deprecations allowed, removals are not") still governs the tree. ADR-0051's
  supersession covered Phases 31-33 only and says a later removal "records its own supersession,
  it does not inherit this one" (PROJECT.md *Out of Scope*). Phase 44 therefore writes its own ADR
  (D-17) and never cites ADR-0051 as its licence.
- **D-00b:** Every break gets a `MIGRATION.md` §9.2 row and, when marked `Y`, a
  `.cargo/semver-checks-allowlist.toml` `[[entry]]` naming one published crate name and one lint,
  row-level set-equal in both directions (`make check-migration-allowlist`, Phase 29 D-04). Lints
  are found empirically by the D-27 diagnostic (temporarily disable the crate-wide `allow` lines in
  the crate's `Cargo.toml`, run `cargo semver-checks check-release --package <crate>
  --baseline-version 0.10.1 --release-type minor`, restore) — Phase 38, 39-08 and 43-13 precedent.
  The **Crate** cell is the crates.io package name (`paladin-ai-core`, not `paladin-core`).
- **D-00c:** Clean break means no shims: no `#[deprecated]` bridges, no type aliases, no
  re-exports keeping a removed name resolvable. Rows and allowlist entries are documentation for
  the single downstream consumer's coordinated refactor, never a compatibility layer (ADR-0051
  "What still applies").
- **D-00d:** Gates unchanged: 82 % workspace line-coverage floor (ADR-0006), `make clean-code`,
  `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for the intentional surface
  change), `make security`, MSRV 1.88, `make check-migration-allowlist`.
- **D-00e:** Version identity is **v0.11.0** everywhere (CHANGELOG `[Unreleased]`, ADR, rows).
- **D-00f:** Phase 26's "middleware never retries itself" rule and the `RetryPolicy` ownership of
  retries (engine nodes via `crates/paladin-battalion/src/engine/retry.rs`; the agent loop via
  `src/application/services/paladin/middleware/resilience.rs`) stand (Phase 43 D-00c). The Phase 25
  `Transience` taxonomy (`crates/paladin-core/src/platform/container/transience.rs`, table-driven
  `PaladinError::transience()` / `LlmError::transience()`) is the classification source.

### BattalionConfig after the cut (LEGACY-01, LEGACY-02)

- **D-01:** `BattalionConfig` loses `timeout_seconds`, `retry_policy` and `error_strategy` (and the
  `with_timeout` / `with_retry_policy` / `with_error_strategy` builders) and gains one field,
  `#[serde(default)] pub aegis: Aegis` (the existing policy bundle from
  `crates/paladin-core/src/platform/container/aegis.rs`, which already derives
  `Serialize`/`Deserialize`/`Default`), plus a `with_aegis(Aegis)` builder. Formation, Phalanx and
  Campaign honour `aegis.timeout` and `aegis.retry`; `aegis.cache` is ignored by the legacy
  patterns (documented on the field). The ADR-0001 field set is superseded (D-18).
  — **Reversibility:** costly — every in-tree config literal, the Commander, the CLI templates
  and the downstream consumer's configs change shape; a serialized `BattalionConfig` document
  loses three keys and gains one.
- **D-02:** The Aegis timeout bounds **each Paladin attempt**, with the engine's meaning:
  `TimeoutPolicy.run_timeout` is a per-attempt wall clock and `idle_timeout` degrades to a
  per-attempt wall clock where the port reports no progress (the `TimeoutPolicy` rustdoc rule).
  There is no whole-battalion wall clock any more; a ten-step Formation may run ten times the
  figure, and that is stated in the docs and the §9.1 row.
- **D-03:** The Commander's own `tokio::timeout` around `execute()` and
  `ConclaveConfig::with_timeout` / `timeout_seconds` are removed with the three named wrappers, and
  **`BattalionError::Timeout(u64)` is retired.** A per-attempt Aegis timeout surfaces through the
  structured path (`BattalionError::Node(NodeError)` with a timeout `NodeErrorSource`, as the engine
  already reports it); the planner confirms the exact variant mapping from `node_error.rs`.
  `error_aggregation.rs` and the two integration tests that match `Timeout` migrate in the same
  commit. — **Reversibility:** costly — one more removed public variant with its own row.
- **D-04:** The Commander keeps bridging into `ConclaveConfig` and the default `ManeuverConfig`,
  sourced from `aegis`: `aegis.timeout.run_timeout` → Conclave / Maneuver timeout,
  `aegis.retry.max_attempts` → `ConclaveConfig.retry_attempts` (the attempt count stays on
  `ConclaveConfig`, derived, not duplicated as a policy). `ManeuverConfig.error_strategy` falls
  back to `maneuver::ErrorStrategy::default()` (`FailFast`) because there is no
  `battalion::ErrorStrategy` left to map from; `CommanderBuilder::error_strategy(maneuver::ErrorStrategy)`
  stays as the explicit override.

### Continue-past-failure and `node_errors` (LEGACY-01)

- **D-05:** "Continue past a failed Paladin" is expressed through `aegis.on_error`:
  `Some(ErrorHandlerSpec::Absorb { .. })` means continue-and-collect; `None` means fail-fast.
  `Route` and `Custom` are rejected by validation on the legacy patterns (they need engine state
  and registries); the planner decides where that validation lives (`BattalionConfig::validate`
  or each service's entry). The old `RetryThenContinue` is the composition `aegis.retry` +
  `Absorb`, not a distinct mode. What `Absorb`'s delta means for a string pipeline is the
  planner's call (a documented no-op / empty contribution is acceptable).
- **D-06:** `BattalionResult.node_errors` retypes from `Vec<battalion::NodeError>` to
  `Vec<paladin_core::platform::container::node_error::NodeError>` (`node_id`, `attempt`,
  `transience`, `source`). The Paladin name maps into `NodeId`; the planner fixes the mapping
  (name as the id string) and the `NodeErrorSource` used for a Paladin failure. One `NodeError`
  remains in the tree. — **Reversibility:** one-way — `BattalionResult` is a serialized value
  (JSON herald, CLI formatter, Citadel-persisted results); the `node_errors` element shape changes
  from `{node_name, error}` to the structured object, recorded as a §9.1 row and a §9.2 row.
  ADR-0002's field set is superseded (D-18).
- **D-07:** A run that continued past failures reports `BattalionStatus::Completed` with a
  non-empty `node_errors` and a non-zero `paladin_failure_count` — today's contract. No
  `PartialSuccess` variant is added.
- **D-08:** The JSON / Markdown / Table heralds and `src/application/cli/formatters/output.rs` keep
  their per-node error block and render a richer line: node id, attempt and transience plus the
  `Display` text. The JSON herald's object shape changes with the retype (noted in the §9.1 row);
  golden tests are updated, not bypassed.

### Conclave retry on the typed taxonomy (LEGACY-03)

- **D-09:** `PaladinError::LlmError(String)` is removed. The `From<LlmError>` erasure path is
  already `paladin_battalion::llm_failure::to_paladin_error` → `LlmFailure { transience, status,
  provider, message }`; every remaining in-tree construction, match arm, rustdoc mention
  (`paladin_port.rs` module docs, `llm_failure.rs` label table, `paladin-eval/src/runner.rs`
  comment, `temperature_service.rs` / `paladin_execution_service.rs` rustdoc) and test literal
  is migrated or deleted in the same commit series.
- **D-10:** `ConclaveExecutionService::is_retryable_error` collapses to
  `error.transience() == Transience::Transient` — no per-variant arms. The `PaladinError::transience()`
  table is the single classification source. Any variant whose answer changes versus today's
  hand-written arms (e.g. `PaladinError::Timeout`, if the table does not mark it `Transient`) is a
  deliberate §9.1 behavioural row, not silently absorbed.
- **D-11:** `Transience::Unknown` is **not** retried by the Conclave — parity with the engine's
  default `TransientOnly` predicate and Aegis retry. `ExecutionError(_)` and other `Unknown`s stop
  being retry candidates.
- **D-12:** The Conclave drops its private `calculate_retry_delay` and uses the engine's
  `paladin_battalion::engine::retry::backoff_delay` + `wait_backoff` with the Aegis `RetryPolicy`
  from `BattalionConfig.aegis` (D-01). `ConclaveConfig.retry_attempts` remains the attempt count
  (D-04). One backoff implementation in the crate.
- **D-13:** `PaladinError::is_retryable()` and `PaladinError::is_terminal()` (documented "legacy,
  superseded by `transience()`", Phase 25 D-05) are **removed**, each with a §9.2 row on the
  `paladin-ai-core | PaladinError` pair. — **Reversibility:** costly — public inherent methods on
  a published type; the downstream consumer may call them.
- **D-14:** `src/infrastructure/resilience/circuit_breaker.rs` moves to `transience()` and counts
  **`Transient` only** toward tripping — the same rule as the Conclave and the engine.
  `ExecutionError(_)` (`Unknown`) stops counting as a retryable failure; recorded as a §9.1 row.
- **D-15:** The sibling `HandoffError::LlmError(String)`, `PromptError::LlmError(String)` and
  `PlanningError::LlmError(String)` are out of scope and untouched (requirement names
  `PaladinError` only). Noted under Deferred Ideas.

### ADR-0059 and the register (LEGACY-04)

- **D-16:** Version identity v0.11.0; the ADR number is **0059** (next free under PROMOTION.md's
  flat counter; 0058 is Phase 43's Cadence ADR).
- **D-17:** **One ADR, ADR-0059**, records both the X-03 supersession for **Phase 44 only** (ADR-0051's
  shape: scoped to this phase and this milestone, every other public API still governed by X-03,
  "a later removal records its own supersession") **and** the replacement design: `BattalionConfig.aegis`
  (D-01/D-02), the `node_errors` retype (D-06), the Conclave / circuit-breaker move to `transience()`
  (D-10, D-14), the `BattalionError::Timeout` and predicate removals (D-03, D-13). Headings per
  PROMOTION.md: Status, Context, Decision, Considered Options, Code Locations, Code Conformance,
  Downstream Consumers.
- **D-18:** ADR-0001 (`BattalionConfig` field set) and ADR-0002 (`BattalionResult` field set) are
  marked **Superseded by ADR-0059** with a dated note, per PROMOTION.md's supersession mechanism;
  their text stays as history. PROJECT.md's Key Decisions table gains the ADR-0059 row and the two
  supersession notes.
- **D-19:** `MIGRATION.md` §9.2 rows are cut **one per `crate | type` pair**, matching the
  allowlist's set-equality key: e.g. `paladin-ai-core | BattalionConfig` (three fields + three
  builders removed, `aegis` added), `paladin-ai-core | battalion module` (`RetryPolicy`,
  `ErrorStrategy`, `NodeError` removed), `paladin-ai-core | BattalionResult` (`node_errors` retype),
  `paladin-ai-core | BattalionError` (`Timeout` removed), `paladin-ai-core | PaladinError`
  (`LlmError` + `is_retryable` + `is_terminal` removed), `paladin-ai-core | ConclaveConfig`
  (timeout removed). Every lint the D-27 diagnostic fires gets its own `[[entry]]` under that
  row's `migration_row` key. Behavioural changes (D-02, D-06 shape, D-10, D-14) go in §9.1.
- **D-20:** Examples and mdBook pages **show the Aegis equivalents**: `examples/commander_full_config.rs`,
  `phalanx_parallel.rs`, `formation_sequential.rs`, `campaign_workflow.rs`,
  `conclave_expert_panel.rs`, `commander_with_metadata_export.rs` and
  `docs/src/user-guides/orchestration.md`, `battalion-patterns.md`,
  `docs/src/appendix/conclave-pattern.md`, `docs/src/getting-started/configuration.md` demonstrate
  `aegis: Aegis { retry, timeout, on_error }` where they showed `RetryPolicy` / `ErrorStrategy` /
  `with_timeout`. The examples double as the downstream consumer's migration guide. Every
  `[[example]]` target still builds (REL-05).
- **D-21:** CHANGELOG `[Unreleased]` gets a `### Removed` block naming each removed item and a
  `### Changed` line for the `aegis` field and the `node_errors` shape, citing ADR-0059 and
  LEGACY-01..04.

### Claude's Discretion

- Where `Route` / `Custom` rejection lives (D-05) and what `Absorb`'s delta means for a string
  pipeline.
- The Paladin-name → `NodeId` mapping and the `NodeErrorSource` chosen for a Paladin failure (D-06).
- The exact structured variant an Aegis per-attempt timeout surfaces as on the legacy patterns (D-03).
- Row wording, lint ids (empirical), and whether a `ConclaveConfig` row is needed once the
  diagnostic runs (D-19).
- Plan/wave split; test-helper fallout in `tests/helpers/mock_paladin_port.rs` and the
  `tests/integration/commander_*` suites.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Policy and record
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — the X-03 supersession
  shape ADR-0059 copies (scope, "what still applies", version identity); Phase 44 does not inherit it
- `.planning/decisions/PROMOTION.md` — ADR numbering (next free: 0059), required headings,
  supersession mechanism used for ADR-0001/0002 (D-18)
- `.planning/decisions/0001-battalion-config.md` — the `BattalionConfig` field set this phase
  supersedes (D-01, D-18)
- `.planning/decisions/0002-battalion-result.md` — the `BattalionResult` field set and the
  `node_errors: Vec<NodeError>` choice this phase supersedes (D-06, D-18)
- `.planning/PROJECT.md` §*Out of Scope* ("X-03 otherwise still governs — a future milestone that
  wants further removals records its own supersession") and §*Current Milestone* (Legacy API clean
  break bullet)
- `.planning/REQUIREMENTS.md` — LEGACY-01..04 wording
- `.planning/ROADMAP.md` §Phase 44 — success criteria 1-4

### Register and gates
- `MIGRATION.md` §9.1 (behavioural rows) and §9.2 (the X-10 register: column rules, crates.io
  package-name rule for the Crate cell, per-pair extension precedent)
- `.cargo/semver-checks-allowlist.toml` — `[[entry]]` schema and the Phase 43 block as the latest
  precedent for row ↔ entry pairing
- `scripts/check-migration-allowlist.sh` (via `make check-migration-allowlist`) — the set-equality gate
- `.planning/phases/43-rate-pacing/43-CONTEXT.md` — carried-forward gate block (D-00) and the
  43-13 closeout's D-27 diagnostic method
- `CHANGELOG.md` `[Unreleased]` — where the Removed / Changed entries go (D-21)

### Code being removed or reshaped
- `crates/paladin-core/src/platform/container/battalion/mod.rs` — `BattalionConfig` (fields,
  builders, serde), legacy `RetryPolicy` / `ErrorStrategy` / `NodeError`, `BattalionResult.node_errors`,
  `BattalionError::Timeout` / `Node`
- `crates/paladin-core/src/platform/container/aegis.rs` — `Aegis`, `RetryPolicy`, `TimeoutPolicy`
  (per-attempt semantics, `idle_timeout` degradation rule), `ErrorHandlerSpec::{Route, Absorb, Custom}`
- `crates/paladin-core/src/platform/container/node_error.rs` — structured `NodeError` /
  `NodeErrorSource` the retype targets (D-06) and the timeout surfacing path (D-03)
- `crates/paladin-core/src/platform/container/paladin_error.rs` — `LlmError(String)`,
  `is_retryable()`, `is_terminal()`, the `transience()` table (D-09, D-10, D-13)
- `crates/paladin-core/src/platform/container/transience.rs` — `Transience` taxonomy
- `crates/paladin-core/src/platform/container/battalion/conclave.rs` — `ConclaveConfig`
  `timeout_seconds` / `retry_attempts` / `with_timeout`
- `crates/paladin-battalion/src/formation_service.rs`, `phalanx_service.rs`, `campaign_service.rs`
  — the tokio timeout wrappers and `ErrorStrategy` matches being removed
- `crates/paladin-battalion/src/commander.rs` — the `execute()` timeout wrapper, the
  Conclave / Maneuver bridging (D-03, D-04), config validation of `timeout_seconds` / `max_attempts`
- `crates/paladin-battalion/src/conclave_execution_service.rs` — `is_retryable_error`,
  `execute_expert_with_retry`, `calculate_retry_delay` (D-10..D-12)
- `crates/paladin-battalion/src/engine/retry.rs` — `backoff_delay`, `wait_backoff`, `should_retry`
  (reused by D-12)
- `crates/paladin-battalion/src/llm_failure.rs` — the `to_paladin_error` erasure replacement and
  the variant-label table that names `LlmError`
- `crates/paladin-battalion/src/error_aggregation.rs` — `ContinueOnError` / `Timeout` references
- `src/infrastructure/resilience/circuit_breaker.rs` — `is_retryable()` call sites (D-14)
- `crates/paladin-herald/src/{json,markdown,table}_herald.rs`,
  `src/application/cli/formatters/output.rs` — `node_errors` rendering (D-08)
- `crates/paladin-eval/src/runner.rs` (comment block near `to_paladin_error`) — rustdoc that names
  the legacy erasure

### Examples and docs to rewrite (D-20)
- `examples/commander_full_config.rs`, `examples/phalanx_parallel.rs`,
  `examples/formation_sequential.rs`, `examples/campaign_workflow.rs`,
  `examples/conclave_expert_panel.rs`, `examples/commander_with_metadata_export.rs`
- `docs/src/user-guides/orchestration.md`, `docs/src/user-guides/battalion-patterns.md`,
  `docs/src/appendix/conclave-pattern.md`, `docs/src/getting-started/configuration.md`
- `src/application/cli/templates/battalion_template.rs` and `src/application/cli/commands/battalion.rs`
  — CLI-side `with_timeout` / `timeout_seconds` users of `BattalionConfig`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Aegis` (`aegis.rs`): `Serialize`/`Deserialize`/`Default`, `validate()`, holds `retry`,
  `timeout`, `on_error`, `cache` — drops straight onto `BattalionConfig` as a defaulted serde field.
- `engine::retry::{backoff_delay, wait_backoff, should_retry}` — the one backoff implementation;
  `wait_backoff` already honours a `CancellationToken`.
- `node_error::NodeError` / `NodeErrorSource` — serde value type with `Display`, already the shape
  `BattalionError::Node` and `AttemptRecord` carry.
- `paladin_battalion::llm_failure::to_paladin_error` — the typed replacement for every
  `PaladinError::LlmError(e.to_string())` erasure (the eval runner already uses it).
- `PaladinError::transience()` / `LlmError::transience()` — table-driven classification with
  per-variant rationale comments; `#[non_exhaustive]` fallbacks already classify by it.

### Established Patterns
- Clean-break bookkeeping: row in §9.2 marked `Y` + `[[entry]]` per lint in the same commit;
  lints discovered by the D-27 diagnostic; `make check-migration-allowlist` gates set-equality.
- `#[serde(default)]` on added fields so pre-phase JSON still deserialises (the `node_errors`,
  `served_by`, `cost` precedents).
- Every in-tree exhaustive match over a changed enum gains or loses its arm in the same commit
  as the variant change (`StopReason`, `BattalionError`, `RateLimitExceeded` precedents).
- ADR supersession is a Status edit plus dated note on the old ADR and a Key Decisions row in
  PROJECT.md (PROMOTION.md).

### Integration Points
- `BattalionConfig` is constructed in: the three legacy services' tests, `commander.rs`
  (`Commander::new`, validation, Conclave/Maneuver bridging), `src/application/cli/templates/battalion_template.rs`,
  `src/application/cli/commands/battalion.rs`, examples, `tests/integration/commander_*`,
  `tests/integration/battalion/*`, `tests/helpers/mock_paladin_port.rs`.
- `BattalionResult.node_errors` is read by three heralds, the CLI formatter,
  `chain_of_command_service.rs`, `commander.rs`, and
  `tests/integration/battalion_herald_end_to_end_test.rs` / `tests/unit/herald_consolidation_test.rs`.
- `BattalionError::Timeout` is matched in `error_aggregation.rs`, `conclave.rs`, the three services,
  `commander.rs`, and two integration tests.
- `PaladinError::is_retryable()` is called only from `circuit_breaker.rs` (two sites); `is_terminal()`
  has no non-test call sites (the other `is_terminal` hits are `RunStatus` / `IsTerminal`).
- No `BattalionConfig` appears in `paladin-web`, `paladin-storage` or `src/infrastructure/web`
  (no HTTP DTO or persisted-schema migration for the config itself); `citadel.rs`'s
  `BattalionCheckpointConfig` (`timeout_seconds`, `continue_on_error`) is a different type and is
  untouched (ADR-0001).

</code_context>

<specifics>
## Specific Ideas

- "One policy vocabulary": the user chose the Aegis bundle over any new knob at every fork —
  timeout, retry, continue-past-failure and backoff all come from `BattalionConfig.aegis`.
- The user chose the wider blast radius for the legacy predicates (remove `is_retryable()` /
  `is_terminal()` now rather than defer), so the end state has one classification source,
  `transience()`, in the Conclave, the circuit breaker and the engine alike.
- Examples are the migration guide for the single downstream consumer: show the Aegis form, do
  not just delete lines.

</specifics>

<deferred>
## Deferred Ideas

- The sibling `HandoffError::LlmError(String)`, `PromptError::LlmError(String)` and
  `PlanningError::LlmError(String)` variants keep the stringly shape; a later phase may move them
  onto a typed source the same way (D-15).
- A `BattalionStatus::PartialSuccess` variant was considered and rejected for this phase (D-07);
  if callers need to match on "completed with failures" without inspecting `node_errors`, that is
  a new capability for a later phase.

### Reviewed Todos (not folded)
- `2026-08-13-verify-local-coverage-reproduction.md` ("Verify local make coverage reproduces CI's
  82.39% figure", score 0.2) — user-owned carried item (`recheck_by: 2026-10-16`), unrelated to
  the legacy removal; left in the todo list.

</deferred>

---

*Phase: 44-legacy-clean-break-removal*
*Context gathered: 2026-10-09*
