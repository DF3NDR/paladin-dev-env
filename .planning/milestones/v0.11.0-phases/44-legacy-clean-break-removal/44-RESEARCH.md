# Phase 44: Legacy Clean-Break Removal - Research

**Researched:** 2026-10-09
**Domain:** Rust API removal and refactor inside the Paladin Cargo workspace (hexagonal architecture): retiring the legacy Battalion error/retry/timeout surface and the untyped `PaladinError::LlmError(String)`, moving the survivors onto the Aegis policy bundle and the `Transience` taxonomy, and recording every break (ADR, `MIGRATION.md`, semver allowlist, examples, docs).
**Confidence:** HIGH for the codebase facts (every claim below was grepped or read in the tree this session). MEDIUM for the predicted `cargo-semver-checks` lint ids, which the D-27 diagnostic must measure, not trust.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

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

### Deferred Ideas (OUT OF SCOPE)

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
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LEGACY-01 | `battalion::RetryPolicy`, `battalion::ErrorStrategy`, `battalion::NodeError` removed; same-named Aegis / Flow types untouched | Full blast radius table (section "Blast radius"); `BattalionConfig.aegis` shape; attempt-runner pattern; legacy-to-Aegis mapping table; `node_errors` retype mapping via the existing `llm_failure::to_node_error_source`; ordering constraints; the `paladin_battalion::retry` module that CONTEXT does not name but that must go |
| LEGACY-02 | Formation / Phalanx / Campaign legacy timeout handling removed; Aegis timeouts are the only timeout path | Verified wrapper sites (formation 162-182, phalanx 127-171, campaign 247-272, commander 472-485, conclave service 96-130); per-attempt timeout surfacing as `BattalionError::Node(NodeError { source: Timeout(Run\|Idle) })`; the "loses its only wall clock" consequence for ChainOfCommand / Grove / Council |
| LEGACY-03 | `PaladinError::LlmError(String)` removed; Conclave retry check on `LlmFailure` / `Transience` | Every construction / match / rustdoc / test site listed; before-after behaviour tables for the Conclave predicate and the circuit breaker; `engine::retry` signatures for D-12 |
| LEGACY-04 | X-03 supersession ADR; §9.2 row + allowlist entry per removal; examples + docs updated | ADR-0059 shape and supersession mechanics; §9.1 / §9.2 column rules and the set-equality key; D-27 diagnostic method; Cargo.toml lint-table consequences; full docs / examples / CHANGELOG file lists |
</phase_requirements>

## Summary

The phase is a pure-Rust removal and migration with **no new external dependency**. Every building block it needs already exists in the tree and was read this session: `Aegis` (`RetryPolicy`, `TimeoutPolicy`, `ErrorHandlerSpec`), the structured `node_error::NodeError` / `NodeErrorSource` / `TimeoutKind`, `Transience`, `PaladinError::transience()`, `llm_failure::to_node_error_source` (already converts a `PaladinError` into a `NodeErrorSource`), and `engine::retry::{backoff_delay, wait_backoff, should_retry}`. The work is to (a) give `BattalionConfig` an `aegis` field, (b) introduce **one** shared per-attempt runner in `paladin-battalion` that Formation, Phalanx, Campaign and the Conclave all call (timeout, retry, backoff, typed failure record), (c) retype `node_errors`, (d) delete the superseded types, wrappers, variants and predicates, and (e) record everything.

CONTEXT.md is accurate on the decisions, but grepping the tree surfaced **ten facts the planner must not miss** (next section). The three that change plan scope most: the public `paladin_battalion::retry` module (a third backoff implementation over the legacy `RetryPolicy`, re-exported by the facade) is not named in CONTEXT but cannot survive the type removal; `ConclaveError::Timeout(u64)` and the Conclave service's own `tokio::timeout` wrapper must go with `BattalionError::Timeout`; and the Aegis `RetryPolicy` semantics differ from the legacy one in two ways that silently change retry behaviour (`max_attempts` counts total attempts, not retries; the default `TransientOnly` predicate does not retry the `ExecutionError` that every in-tree test mock returns).

**Primary recommendation:** Land it in four ordered stages that each leave the workspace compiling: (1) ADR-0059 opening plan; (2) the `PaladinError` / circuit-breaker / Conclave-predicate side of LEGACY-03 (independent of the Battalion config work); (3) additive `BattalionConfig.aegis` plus a `pub(crate)` attempt runner, then the atomic service cut (services + `node_errors` retype + heralds + tests), then the deletion commit (legacy types, builders, `retry.rs`, `Timeout` variants, `ConclaveConfig` timeout) together with examples / CLI / docs; (4) a closeout plan that runs the D-27 diagnostic, writes the measured §9.1 / §9.2 rows, allowlist entries and `Cargo.toml` allow lines, and runs the full gate.

## Research Findings That Extend CONTEXT.md (planner must read)

All `[VERIFIED: codebase grep]` unless tagged.

1. **`paladin_battalion::retry` is a public module the removal breaks, and CONTEXT does not name it.** `crates/paladin-battalion/src/retry.rs` (`calculate_retry_delay`, `should_retry`) is typed on the legacy `battalion::RetryPolicy`, is called only by `formation_service.rs` (line 12 import, line 339), is re-exported by the facade at `src/application/services/battalion/mod.rs:18` (`pub use paladin_battalion::retry;`), appears in the committed API baseline `.project/current-exports.txt:410`, is described in `crates/paladin-battalion/src/lib.rs` ("[`retry`] — Exponential back-off retry helper"), and is shown to users in `docs/src/deployment/production.md:420-445`. D-12 ("one backoff implementation in the crate") already implies deleting it. Consequences: delete the file and the facade re-export in the same commit as the type removal; `make api-surface-update` **will** be needed (one line); add a `paladin-battalion` §9.2 row + allowlist entry; `crates/paladin-battalion/Cargo.toml` has **no** `[package.metadata.cargo-semver-checks.lints]` table today, so the closeout must add one if the diagnostic fires.
2. **`ConclaveError::Timeout(u64)` and the Conclave service's own timeout wrapper.** `conclave_execution_service.rs:96-130` wraps `execute_internal` in `tokio::time::timeout(conclave.config.timeout_seconds)` and returns `ConclaveError::Timeout`. `conclave.rs:442-443` declares the variant (the enum is **not** `#[non_exhaustive]`), and `conclave.rs:459` maps it to `BattalionError::Timeout`. With `BattalionError::Timeout` retired and the wrapper removed the variant has no producer; remove it and give `ConclaveError` its own §9.2 row (`enum_variant_missing`). `Conclave::validate` (`conclave.rs:313-324`) also range-checks `timeout_seconds` (10..=3600) and has two tests (`conclave.rs:594-612`); replace with `Aegis::validate` plus the zero-duration check. The doc `docs/src/appendix/conclave-pattern.md:674` matches `ConclaveError::Timeout`.
3. **Campaign gains behaviour; Phalanx's "retry" never existed.** `campaign_service.rs` never read `error_strategy` or `retry_policy` (it only has the timeout wrapper, lines 247-272, and fails fast via `?` at line 350). Phalanx's `RetryThenContinue` arm (`phalanx_service.rs:227-234`) only logs "retries handled at Paladin level" — no retry exists. Formation retries only under `RetryThenContinue`. So `aegis.retry` is **new** behaviour for Phalanx and Campaign. Campaign has no continue-past-failure mode; the parity choice is "Campaign ignores `on_error`" (see Open Question 4).
4. **Retry semantics differ from the legacy policy in two behaviour-changing ways.**
   - Legacy `max_attempts` counts **retries** (`should_retry` is `attempt < max_attempts` starting at 0, so `max_attempts: 3` = 4 executions; `crates/paladin-battalion/src/retry.rs:100-102` doctest). Aegis `max_attempts` counts **total attempts** (`aegis.rs:101-104`). The Commander's Conclave bridge already uses the Aegis meaning (`max_attempts.saturating_sub(1)`, `commander.rs:638`). Migration rule: `new = old + 1`.
   - Legacy `RetryThenContinue` retried **every** error. Aegis defaults to `RetryPredicate::TransientOnly`, which never retries `Transience::Unknown`. The shared test mocks (`tests/helpers/mock_paladin_port.rs` `fail_always`, `fail_paladin`, `fail_until_attempt`; `IntegrationMockPaladinPort` in `tests/integration/battalion/formation_integration_test.rs:70` and `phalanx_integration_test.rs:79`) all return `PaladinError::ExecutionError` (`Unknown`). Tests that exercise retry must set `retry_on: RetryPredicate::TransientAndUnknown` (to keep "retry everything") or use `fail_paladin_until_attempt`, which already returns a Transient `LlmFailure`. The examples that replace `RetryThenContinue` should show `TransientAndUnknown` and say why. Permanent errors are no longer retried under either predicate (a §9.1 row).
5. **The default 300 s bound disappears, and three Commander-routed patterns lose their only wall clock.** `BattalionConfig::new` defaults `timeout_seconds: 300`; `Aegis::default()` has `timeout: None`. The Commander's `execute()` wrapper (`commander.rs:472-485`) is the only bound over ChainOfCommand and Grove (neither service reads any timeout; `grep` of `chain_of_command_service.rs` / `grove_service.rs` finds none) and the Council has only a per-speaker 300 s `timeout` (`council_service.rs:213-216`, not in scope). Maneuver keeps a flow-level timeout from `ManeuverConfig.timeout`. D-02/D-03 accept "no whole-battalion wall clock"; the §9.1 row and the docs must say plainly that ChainOfCommand and Grove are now unbounded unless the caller wraps `execute()` or sets per-call timeouts in their own ports. `CommanderBuilder`'s default config (`commander.rs:1766-1771`, documented "Timeout: 300 seconds" at line 1432) should keep a 300 s per-attempt `run_timeout` so a bare `CommanderBuilder` is not silently unbounded (Open Question 1).
6. **Real before/after behaviour of the typed predicate (D-10 / D-14).** `PaladinError::transience()` marks `Timeout` `Transient`, so the example in D-10 does not occur. The variants whose Conclave answer actually changes versus the hand-written arms are listed in "Behaviour deltas" below (`CircuitBreakerOpen`, `GarrisonError(Storage|Tokenization)`, `ArsenalError(Timeout|TransportError)` become retried; `ExecutionError` was already **not** retried by the Conclave). The circuit breaker's production reach is narrower than CONTEXT implies: `paladin_execution_service.rs:2828` and `:3009` wrap only `llm_port.generate(..)` and map errors through `to_paladin_error`, so in production the breaker only ever sees `LlmFailure`; D-14 therefore means Permanent (401/400) and Unknown (`ProcessingError`) LLM failures **stop** tripping it.
7. **`src/application/cli/formatters/output.rs` renders no `node_errors`.** The only occurrence is `node_errors: Vec::new()` in a test fixture (line 716), which compiles unchanged after the retype. Per-node failure rendering lives only in `json_herald.rs:153`, `markdown_herald.rs:395-399`, `table_herald.rs:239-250`. D-08's "and output.rs" needs no code change there.
8. **Files outside CONTEXT's lists that use the removed API (must be migrated or the workspace will not compile):** `crates/doc-examples/src/orchestration.rs:11,21` (compiled mdBook anchor `formation`, gate `make check-doc-examples`); `examples/commander_auto.rs:83`, `examples/commander_basic.rs:98-105`; `src/application/cli/commands/battalion.rs:268,434,594,607,825` (five `with_timeout` sites, two on `ConclaveConfig`); `tests/unit/battalion/formation_tests.rs:87,119`, `tests/unit/battalion/campaign_service_tests.rs:136`, `tests/integration/battalion/{campaign,formation,phalanx,load}_integration_test.rs` and `load_test.rs` (struct literals at lines 122 and 212), `tests/integration/battalion_herald_end_to_end_test.rs`, `tests/unit/circuit_breaker_test.rs`, `tests/unit/paladin_error_test.rs`; in-crate tests in `commander.rs`, the three services, `battalion/mod.rs`, `conclave.rs`, `error_aggregation.rs`, `llm_failure.rs`, `herald` crates. Docs beyond CONTEXT: `docs/src/deployment/production.md:420-445`, `docs/src/user-guides/paladin-agents.md:357` (a `LlmError(String)` retryability table row), `docs/src/user-guides/paladin-configuration.md:337`, `docs/src/contributing/architecture-decisions.md:122,128`, `docs/src/appendix/battalion-vision-support.md:301`, `docs/src/deployment-topologies/battalion-orchestration.md` (includes the doc-examples `phalanx` anchor), plus CLI help / README text if `grep` after the edit finds more.
9. **`docs/src/getting-started/configuration.md:356-370` documents a `battalion:` YAML section that no Rust type reads** (`grep` for `max_concurrent_paladins` / `metadata_output_enabled` in `src/` and `crates/` is empty; `orchestration.md` itself says "there is no `battalion:` section in `config.yml`"). The `error_strategy` / `retry` keys there name concepts being removed. Replace the block with a pointer to programmatic `BattalionConfig.aegis`, do not invent an Aegis YAML schema. `scripts/check-doc-config.sh` only checks that YAML blocks parse.
10. **The §9.2 pair key is one identifier per row.** The awk in `scripts/check-migration-allowlist.sh` takes the first backticked identifier of the Type cell and strips the allowlist's `migration_row` to its first whitespace token. A row keyed `battalion module` therefore collapses to the pair `paladin-ai-core | battalion`, and one row cannot own three types unambiguously. Cut **three** rows (`RetryPolicy`, `ErrorStrategy`, `NodeError`) rather than one `battalion module` row; each removed type then pairs 1:1 with its `struct_missing` / `enum_missing` entry.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Policy vocabulary (`Aegis`, `RetryPolicy`, `TimeoutPolicy`, `ErrorHandlerSpec`) and `BattalionConfig.aegis` | Domain core (`paladin-core`) | — | Pure serde value types; core must stay dependency-free (ADR-0015) |
| Failure classification (`PaladinError::transience()`, `Transience`) | Domain core | — | Single classification source (D-00f); already table-driven |
| Structured failure record (`node_error::NodeError`, `BattalionResult.node_errors`) | Domain core | Herald (render) | Serialised value carried on the result |
| Per-attempt timeout + retry + backoff loop for legacy patterns | Application services (`paladin-battalion`) | Domain core (policy types) | Needs `tokio`, `PaladinPort`, `engine::retry`; must not live in core |
| `PaladinError` to `NodeErrorSource` conversion | Application services (`llm_failure.rs`) | — | Already exists as `to_node_error_source`; reuse it, do not duplicate |
| Conclave retry predicate and backoff | Application services (`conclave_execution_service.rs`) | Domain core (`transience()`) | D-10 / D-12 |
| Circuit-breaker accounting | Infrastructure (`src/infrastructure/resilience`) | Domain core (`transience()`) | D-14 |
| Rendering of `node_errors` | Presentation adapters (`paladin-herald`) | — | JSON / Markdown / Table heralds |
| Legacy config construction at the edge (CLI) | Application / CLI (`src/application/cli`) | — | Maps the CLI's own YAML `timeout_seconds` onto `Aegis.timeout` |
| ADR, register, allowlist, CHANGELOG | Documentation / governance (`.planning`, `MIGRATION.md`, `.cargo`) | — | LEGACY-04 |

## Standard Stack

### Core (all in-tree; no new dependency is added by this phase)

| Library / module | Version | Purpose | Why standard |
|---|---|---|---|
| `paladin_core::platform::container::aegis` (`Aegis`, `RetryPolicy`, `TimeoutPolicy`, `RetryPredicate`, `ErrorHandlerSpec`) | workspace 0.10.1 | The one policy vocabulary (user chose it at every fork) | Already `Serialize + Deserialize + Default + PartialEq`; `Aegis::validate()` rejects `max_attempts == 0` [VERIFIED: aegis.rs:42,66] |
| `paladin_core::platform::container::node_error` (`NodeError`, `NodeErrorSource`, `TimeoutKind`) | 0.10.1 | Structured per-node failure; replaces the legacy summary struct | `Display`, `Serialize`, `Deserialize`, field order pinned by tests [VERIFIED: node_error.rs:36-179] |
| `paladin_core::platform::container::transience::Transience` | 0.10.1 | Three-valued, deliberately not `#[non_exhaustive]` | Classification source [VERIFIED: transience.rs:31-42] |
| `paladin_battalion::engine::retry::{backoff_delay, wait_backoff, should_retry}` | 0.10.1 | The one backoff implementation | `backoff_delay(&aegis::RetryPolicy, attempt: u32) -> Duration` (attempt >= 2 is the attempt about to run); `wait_backoff(Duration, &Option<CancellationToken>) -> bool`; `should_retry(&RetryPolicy, &NodeError, attempt) -> bool` delegating to `RetryPredicate::admits` [VERIFIED: engine/retry.rs:48-117] |
| `paladin_battalion::llm_failure::{to_paladin_error, to_node_error_source}` | 0.10.1 | LlmError to PaladinError; PaladinError to NodeErrorSource | `to_node_error_source` already maps `LlmFailure` to `NodeErrorSource::Llm` and every other variant to `NodeErrorSource::Paladin { kind }` [VERIFIED: llm_failure.rs:126-170] |
| `tokio` (`time::timeout`, `time::pause`/`start_paused`), `tokio_util::sync::CancellationToken` | workspace | Per-attempt wall clock; cancellation-aware backoff | `test-util` is already a dev-dependency of both `paladin-battalion` and the root crate [VERIFIED: Cargo.toml:294, crates/paladin-battalion/Cargo.toml:57] |
| Toolchain | rustc/cargo 1.97.1 (pinned in `rust-toolchain.toml`), MSRV 1.88, `nightly-2026-09-20` for `make api-surface` | — | [VERIFIED: rust-toolchain.toml, `rustup toolchain list`] |

### Supporting / tooling

| Tool | Version | Purpose | When |
|---|---|---|---|
| `cargo-semver-checks` | 0.50.0 (CI pin, `ci.yml`) | D-27 diagnostic | Closeout plan; **not installed locally** (see Environment Availability) |
| `cargo-public-api` | 0.52.0 | `make api-surface` / `api-surface-update` | Installed [VERIFIED: `cargo +nightly-2026-09-20 public-api --version`] |
| `cargo-llvm-cov` | — | 82 % floor | CI `coverage` job only; not installed locally |
| `cargo-audit`, `cargo-deny` | installed | `make security` | Closeout gate |

### Alternatives Considered

| Instead of | Could use | Tradeoff |
|---|---|---|
| One shared `pub(crate)` attempt runner | Inline retry loops in each of Formation / Phalanx / Campaign / Conclave | Four copies of timeout + retry + typed-failure logic; the legacy design already diverged (Formation retried, Phalanx did not). The runner is the cheapest way to give "Aegis timeouts are the only timeout path" a single greppable site |
| Reusing `engine::retry::should_retry(&NodeError)` | Calling `retry_on.admits(err.transience())` directly | Equivalent (`should_retry` is a one-line delegate). Building the `NodeError` first is free because the runner needs it for `node_errors` anyway |
| Three §9.2 rows (one per removed type) | One `battalion module` row (D-19's example wording) | The set-equality key is the first backticked identifier; one row cannot pair three `struct_missing` / `enum_missing` entries unambiguously (Finding 10) |

**Installation:** none (`cargo` workspace only). `package-legitimacy` was not run because no crate is added or changed in `Cargo.toml` `[dependencies]`.

## Package Legitimacy Audit

No external packages are installed or upgraded by this phase. `tokio`, `tokio-util`, `rand`, `serde`, `thiserror` are existing workspace dependencies. The only external artefact is the CI-pinned `cargo-semver-checks@0.50.0` **binary**, used as a developer tool (it is not a crate dependency and is not added to any manifest); Phase 43 plan 43-13 fetched the `v0.50.0` GitHub release asset for it. A `HEAD` of `https://github.com/obi1kenobi/cargo-semver-checks/releases/download/v0.50.0/cargo-semver-checks-x86_64-unknown-linux-gnu.tar.gz` returned `302` to the release CDN this session through the agent proxy `[VERIFIED: curl]`.

**Packages removed due to [SLOP] verdict:** none. **Packages flagged [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
 caller / CLI / Commander
        │  BattalionConfig { name, description, metadata_output_dir, aegis: Aegis }   (D-01)
        ▼
 ┌───────────────────────────── Commander::execute (NO timeout wrapper, D-03) ──────────────────────────┐
 │ resolve strategy → validate aegis (reject Route/Custom handler, Custom retry predicate, zero durations,│
 │ max_attempts == 0)                                                                                     │
 └──┬───────────────┬───────────────┬───────────────────┬─────────────────┬──────────────────┬──────────┘
    ▼               ▼               ▼                   ▼                 ▼                  ▼
 Formation       Phalanx         Campaign          Conclave        ChainOfCommand/       Maneuver
 (sequential)    (concurrent)    (DAG, per node)   (experts ∥ +     Council/Grove         (flow DSL;
    │               │               │               aggregator)     (UNBOUNDED now,       timeout from
    └───────┬───────┴───────┬───────┘                   │            see Finding 5)        aegis.timeout
            ▼                                           ▼                                  .run_timeout, D-04)
   ┌──────────────────────────────────────────────────────────────────┐
   │ attempt runner  (pub(crate), ONE tokio::time::timeout site)      │
   │   for attempt in 1..=max_attempts(aegis.retry, default 1):       │
   │     bound = min(run_timeout, idle_timeout)  [idle degrades to    │
   │             wall clock: PaladinPort::execute reports no progress]│
   │     res   = timeout(bound, port.execute(paladin, input))         │
   │     Err(e)→ NodeError{ node_id = paladin name, attempt,           │
   │              transience = e.transience(), source =                │
   │              to_node_error_source(&e) | Timeout(Run\|Idle) }      │
   │     retry? = attempt < max && retry_on.admits(transience)         │
   │     wait   = wait_backoff(backoff_delay(retry, attempt+1), cancel)│
   └───────────────┬───────────────────────────────┬─────────────────┘
                   │ Ok(PaladinResult)             │ Err(NodeError)
                   ▼                               ▼
          paladin_results / metrics     on_error == None  → return Err(BattalionError::Node(NodeError))   [timeout]
                                                              or BattalionError::PaladinError(String)      [other]
                                        on_error == Absorb → push NodeError to node_errors, continue with
                                                             EMPTY output (today's Formation contract)
                   │                               │
                   └───────────────┬───────────────┘
                                   ▼
      BattalionResult { status: Completed, paladin_failure_count = node_errors.len(),
                        node_errors: Vec<node_error::NodeError> }          (D-06, D-07)
                                   │
                                   ▼
      JSON herald (serialises the objects) / Markdown + Table heralds (id, attempt, transience, Display)   (D-08)

 Conclave path:  ConclaveConfig { retry_attempts (retries, 0..=5), battalion_config.aegis }
   each expert: loop { runner attempt (timeout) ; retry iff error.transience() == Transient ;
                       delay = backoff_delay(aegis.retry.unwrap_or_default() with max_attempts = retry_attempts+1) }
   aggregator: one attempt under the same per-attempt bound (no retry today; keep)

 Circuit breaker:  CircuitBreaker::call / call_async → count a failure iff error.transience() == Transient   (D-14)
```

### Recommended Project Structure (changes only)

```
crates/paladin-core/src/platform/container/
├── battalion/mod.rs            # BattalionConfig{aegis}; DELETE RetryPolicy, ErrorStrategy, NodeError; retype node_errors;
│                               #   DELETE BattalionError::Timeout; add validate_aegis()
├── battalion/conclave.rs       # DELETE timeout_seconds, with_timeout, range checks, ConclaveError::Timeout + From arm
└── paladin_error.rs            # DELETE LlmError variant, is_retryable, is_terminal; trim tests
crates/paladin-battalion/src/
├── aegis_attempt.rs  (NEW, pub(crate))   # the single per-attempt runner (name is the planner's; keep it non-pub)
├── retry.rs          (DELETE)            # legacy backoff over the legacy RetryPolicy
├── formation_service.rs / phalanx_service.rs / campaign_service.rs   # use the runner; no tokio::timeout
├── conclave_execution_service.rs         # typed predicate, engine backoff, no wrapper
├── commander.rs                          # no wrapper; aegis bridging; validation; builder default
├── error_aggregation.rs                  # test-only Timeout references
└── llm_failure.rs                        # drop LlmError label arm + test rows
crates/paladin-herald/src/{json,markdown,table}_herald.rs   # render typed node_errors
src/application/services/battalion/mod.rs                   # drop `pub use paladin_battalion::retry;`
src/infrastructure/resilience/circuit_breaker.rs            # transience()-based counting
src/application/cli/commands/battalion.rs                   # with_timeout → with_aegis(...)
```

### Pattern 1: `BattalionConfig` after the cut (D-01)

```rust
// Source: crates/paladin-core/src/platform/container/battalion/mod.rs (shape to implement)
use crate::platform::container::aegis::Aegis;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattalionConfig {
    pub name: String,
    pub description: Option<String>,
    /// Per-attempt timeout (`run_timeout`; `idle_timeout` degrades to a wall clock),
    /// retry (total attempts, `retry_on`), and continue-past-failure (`Absorb`). `cache`
    /// is ignored by the legacy patterns; `Route`/`Custom` handlers and `Custom` retry
    /// predicates are rejected by `validate_aegis`.
    #[serde(default)]
    pub aegis: Aegis,
    pub metadata_output_dir: Option<PathBuf>,
}
impl BattalionConfig {
    pub fn with_aegis(mut self, aegis: Aegis) -> Self { self.aegis = aegis; self }
    /// Pure, dependency-free: lives in core beside the type.
    pub fn validate_aegis(&self) -> Result<(), BattalionError> { /* see Pattern 5 */ }
}
```

A pre-phase serialised document (`timeout_seconds`, `retry_policy`, `error_strategy` keys) still deserialises: `BattalionConfig` has no `deny_unknown_fields`, so the three legacy keys are ignored and `aegis` defaults [VERIFIED: struct derive at mod.rs:36; no serde container attribute]. Pin that with a test (it is the `#[serde(default)]` precedent D-00 cites).

### Pattern 2: the shared attempt runner (sketch built from the verified signatures)

```rust
// crates/paladin-battalion/src/aegis_attempt.rs — pub(crate): no new public surface
pub(crate) struct AttemptOutcome { pub result: PaladinResult, pub attempts: u32 }
pub(crate) struct AttemptFailure { pub error: PaladinError, pub node_error: NodeError }

pub(crate) async fn run_with_aegis(
    port: &Arc<dyn PaladinPort>, paladin: &Paladin, input: &str,
    aegis: &Aegis, cancel: &Option<CancellationToken>,
) -> Result<AttemptOutcome, AttemptFailure> {
    let max = aegis.retry.as_ref().map_or(1, |r| r.max_attempts);
    let bound = aegis.timeout.as_ref().and_then(effective_bound); // min of the Some(..)s
    let node_id = NodeId::new(paladin.node.name.clone());          // D-06 mapping: name is the id
    for attempt in 1..=max {
        let call = port.execute(paladin, input);
        let raw = match bound {
            Some((d, _kind)) => tokio::time::timeout(d, call).await,
            None => Ok(call.await),
        };
        let (error, source, transience) = match raw {
            Ok(Ok(result)) => return Ok(AttemptOutcome { result, attempts: attempt }),
            Ok(Err(e)) => { let t = e.transience(); let s = to_node_error_source(&e); (e, s, t) }
            Err(_elapsed) => {                      // per-attempt wall clock fired
                let kind = bound.unwrap().1;       // Run, or Idle when idle < run (or run unset)
                (PaladinError::Timeout(bound.unwrap().0.as_secs()),
                 NodeErrorSource::Timeout(kind), Transience::Transient)
            }
        };
        let node_error = NodeError { node_id: node_id.clone(), attempt, transience, source };
        let retry = aegis.retry.as_ref();
        if let Some(policy) = retry
            && attempt < max
            && should_retry(policy, &node_error, attempt)
            && wait_backoff(backoff_delay(policy, attempt + 1), cancel).await
        { continue; }
        return Err(AttemptFailure { error, node_error });
    }
    unreachable!("max >= 1 is guaranteed by Aegis::validate");
}
```

Notes: `Aegis::validate()` rejects `max_attempts == 0` but **not** a zero `Duration`; add the zero check to `validate_aegis` (the engine rejects `Some(Duration::ZERO)` in `WarGraph::validate`, per the `TimeoutPolicy` rustdoc). Remove the `unreachable!` if clippy `-D warnings` or the "no panic in library code" rule objects; restructure as a `loop` that returns. Library code may not `unwrap()`: the sketch's `bound.unwrap()` must become a pattern-bound value.

### Pattern 3: continue-past-failure and the `node_errors` mapping (D-05..D-07)

- `aegis.on_error == None`: fail fast. Formation returns on the first failure; Phalanx collects every failure then returns `BattalionError::AggregationError` (today's behaviour, keep); Campaign returns on the first failure.
- `Some(ErrorHandlerSpec::Absorb { .. })`: record the `AttemptFailure.node_error`, count it in `paladin_failure_count`, continue. Formation continues with an **empty** string as the next input (today: `current_input = String::new()`, `formation_service.rs:258`); the Absorb `fallback_delta` is a documented no-op for string pipelines. (The mdBook text `orchestration.md:86-87` claims "its input is passed through"; that is wrong today and must be corrected.)
- Status stays `BattalionStatus::Completed` with `node_errors` non-empty (D-07).
- `node_id` = Paladin name (`NodeId::new(name)`, a transparent string newtype [VERIFIED: waypoint.rs:293-307]); `source` for a Paladin failure = `llm_failure::to_node_error_source(&err)` (`Llm` for an `LlmFailure`, `Paladin { kind }` otherwise); `transience` = `err.transience()`; a per-attempt timeout is `NodeErrorSource::Timeout(TimeoutKind::Run)` (or `Idle` when the idle bound is the tighter one) with `Transience::Transient`, exactly what the engine records [VERIFIED: aegis.rs:203-208, node_error.rs:36-55].
- Phalanx currently carries failures as `"{name}: {error}"` strings and re-parses them with `split_once` (`phalanx_service.rs:246-262`); the retype is the moment to carry `Vec<AttemptFailure>` through `execute_collect_all` / `execute_first_success` / `execute_majority` instead, which also removes the fragile `failed_names` string parse.

### Pattern 4: the Conclave on the typed taxonomy (D-10..D-12)

```rust
// conclave_execution_service.rs
fn is_retryable_error(error: &PaladinError) -> bool {
    error.transience() == Transience::Transient            // D-10: no per-variant arms
}
// retry loop: attempts = conclave.config.retry_attempts + 1 (retry_attempts counts RETRIES, 0..=5);
// policy = battalion_config.aegis.retry.clone().unwrap_or_default() with max_attempts overridden;
// delay  = backoff_delay(&policy, retries + 1);   wait = wait_backoff(delay, &None).await
```

`ConclaveConfig` already embeds `battalion_config: BattalionConfig` (`conclave.rs:91`), so the service reads `conclave.config.battalion_config.aegis` directly; no new `ConclaveConfig` field and no plumbing change. D-04's "derive `retry_attempts` from `aegis.retry.max_attempts`" is the Commander bridge: keep `max_attempts.saturating_sub(1)` (`commander.rs:638`).

### Pattern 5: validation (Claude's discretion, D-05)

Put one pure `BattalionConfig::validate_aegis()` in `paladin-core` (rejects: `Aegis::validate()` failure, `on_error` of `Route`/`Custom` (and a `_` arm, `ErrorHandlerSpec` is `#[non_exhaustive]`), `RetryPredicate::Custom(_)` which `admits` as always-false and would silently never retry, `Some(Duration::ZERO)` for either timeout). Call it from `CommanderBuilder::build` (replacing `timeout_seconds == 0` and `max_attempts == 0` checks at `commander.rs:1774-1784`), from `Conclave::validate`, and at the top of each of Formation / Phalanx / Campaign `execute` (services can be built without a Commander; `Campaign` already validates in `execute`).

### Anti-Patterns to Avoid

- **A second `tokio::time::timeout` anywhere in the five legacy files.** The success criterion is "Aegis timeouts are the only timeout path"; the runner is the single site. Council's per-speaker 300 s `timeout` is a different, out-of-scope pattern.
- **Keeping `BattalionError::Timeout(u64)` "just unconstructed".** D-03 retires it; an unconstructed public variant is the shim D-00c forbids.
- **A compat deserializer for the old `node_errors` shape.** D-06 is a one-way door; do not add an untagged enum that accepts `{node_name, error}`. Document that a v0.10 `BattalionResult` JSON with a **non-empty** `node_errors` no longer deserialises (an empty array or an absent key still does, via `#[serde(default)]`).
- **Mapping `max_attempts` one-to-one in docs and examples.** Always show `old + 1`.
- **Re-deriving the transience table in the Conclave or the breaker.** Both call `error.transience()` and nothing else.
- **Asserting on wall-clock sleeps.** Use `#[tokio::test(start_paused = true)]` (the idiom `engine/retry.rs` established).

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---|---|---|---|
| Backoff delay with cap and jitter | A fourth delay function | `engine::retry::backoff_delay` | Already tested; exponent/cap/jitter edge cases handled (`base.is_zero()` guard avoids an empty `gen_range`) |
| Cancellation-aware sleep | `tokio::time::sleep` in the loop | `engine::retry::wait_backoff` | Races the sleep against the token; returns `false` when cancelled |
| Transience to retry decision | `match` on `PaladinError` variants | `RetryPredicate::admits` via `should_retry` | The single home of the transience-to-boolean table (Phase 26 D-11) |
| `PaladinError` to `NodeErrorSource` | A new conversion | `llm_failure::to_node_error_source` | Already carries `kind`, `status`, `provider`; redaction happened upstream (D-34) |
| Timeout kind naming | A string like `"timeout"` | `NodeErrorSource::Timeout(TimeoutKind::Run \| Idle)` | Typed, `Display` stable, never inferred from message text |
| Policy validation | Ad hoc range checks | `Aegis::validate()` plus one `validate_aegis()` | Keeps one definition of a valid policy |
| Source-tree guard for "this symbol is gone" | A `grep` in prose | A Rust guard test with positive controls, modelled on `tests/treasurer_vocabulary_guard.rs` | A guard that cannot fail proves nothing; that file's two positive controls are the house pattern |

**Key insight:** every piece of the replacement already exists and is tested; this phase is wiring plus deletion. The risk is not building something new, it is the long tail of consumers (tests, examples, docs, re-exports) and the behavioural deltas hidden behind a rename.

## Runtime State Inventory

> Rename / removal / migration phase. All five categories answered.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `BattalionResult` is a serialised value. Writers: the JSON herald (`json_herald.rs:153`, emits `node_errors`), the metadata exporter if `metadata_output_dir` is set. `citadel.rs` persists `BattalionCheckpointConfig`, a different type (ADR-0001), not `BattalionConfig`. No database table or HTTP DTO references `BattalionConfig` or `node_errors` (`paladin-web`, `paladin-storage` clean per `git grep`) | **Data shape only, no migration job.** Old JSON with a non-empty `node_errors` stops deserialising into the new type; empty or absent still reads. Record in `MIGRATION.md` §9.1 (herald JSON shape) and §9.4 (stored `BattalionResult` JSON) and pin with a test. No code edit for stored data; no `#[serde]` compatibility shim (D-00c) |
| Live service config | None. No external service holds a battalion timeout / retry / error-strategy value; the framework's YAML `Settings` has no `battalion:` section (Finding 9). `examples/cli_configs/*.yaml` carry the **CLI's own** `timeout_seconds`, a separate schema | None for services. Decide the CLI YAML key mapping (Open Question 8): recommended keep the key, map it to `Aegis.timeout.run_timeout`, state "per attempt" |
| OS-registered state | None found (no systemd / launchd / Task Scheduler / pm2 artefact references these types) — verified by `git grep` over `docker/`, `k8s/`, `scripts/`, `.github/` for `timeout_seconds` battalion keys returning nothing relevant | None |
| Secrets and env vars | None. No env var or secret key is named after the removed items. `APP_*` overrides do not include battalion keys | None |
| Build artifacts / installed packages | `target/` (19 GB, local) and tracked generated files `lcov.info`, `final-api.txt`, `api_surface_current.txt` still list the old names; no script or CI gate reads `final-api.txt` or `api_surface_current.txt` (`git grep` empty). `.project/current-exports.txt` **is** a gate input (line 410 lists the `retry` re-export) | Regenerate `.project/current-exports.txt` with `make api-surface-update` (nightly-2026-09-20). Leave the other three generated files alone; note in the closeout that they are stale artefacts, not gates. No crates.io state changes until Phase 47 |

**The canonical question — what still holds the old string after every file is edited?** Only previously serialised `BattalionResult` JSON documents (herald output files, exported metadata) and the downstream consumer's own checked-out config code. Both are documented, neither is migrated by code.

## Common Pitfalls

### Pitfall 1: The retry default silently stops retrying everything the tests throw at it
**What goes wrong:** Tests and examples ported with `retry: Some(RetryPolicy { max_attempts: 3, .. })` never retry, because the mocks return `ExecutionError` (`Unknown`) and the default predicate is `TransientOnly`.
**Why:** Legacy `RetryThenContinue` retried all errors; Aegis gates on transience (Finding 4).
**Avoid:** `retry_on: RetryPredicate::TransientAndUnknown` where "retry everything" is intended, or `FaultyPaladinPort::fail_paladin_until_attempt` (Transient `LlmFailure`). Add one test per predicate arm on a legacy pattern.
**Warning signs:** `call_count` stays at the number of Paladins; "retried" assertions fail by exactly the retry count.

### Pitfall 2: `max_attempts` off by one
**What goes wrong:** `max_attempts: 3` now yields 3 executions, not 4; or a migrated example claims "3 retries".
**Avoid:** Use `old + 1` in migrations; state "total attempts, including the first" in the field docs and examples. The Conclave bridge is already correct.

### Pitfall 3: The Conclave's attempt count versus the Aegis policy
**What goes wrong:** Setting `max_attempts` on the policy and `retry_attempts` on `ConclaveConfig` disagree. `with_retry_attempts` clamps to 5 and `validate` rejects > 5.
**Avoid:** `ConclaveConfig.retry_attempts` is authoritative for the count (D-04); the Aegis policy contributes only backoff shape and `retry_on`. Test the two together. When `aegis.retry` is `None` and the Commander bridges, keep the `ConclaveConfig` default of 2 retries (Open Question 6) so a bare Commander Conclave does not lose today's resilience.

### Pitfall 4: Tests that still sleep for real
**What goes wrong:** The current Conclave tests sleep 1 s + 2 s of real time (`test_partial_success_with_one_expert_failure` fails an expert 10 times with `Timeout(10)`); porting them unchanged keeps the suite slow and, with the engine's jitter, flaky on ranges.
**Avoid:** `#[tokio::test(start_paused = true)]` and measure with `tokio::time::Instant` deltas (the `engine/retry.rs` idiom). Replace `test_retry_delay_calculation` (it calls the private function being deleted) with a test of the chosen policy's `backoff_delay` sequence at `jitter: false`.

### Pitfall 5: `PaladinError::Timeout(u64)` truncates sub-second bounds
**What goes wrong:** A 250 ms per-attempt bound surfaces in the Conclave as `PaladinError::Timeout(0)`.
**Avoid:** Use `.as_secs().max(1)` for the Conclave's `PaladinError::Timeout` payload, or prefer the `NodeError` path in the three pattern services where the kind is typed. Do not widen `PaladinError::Timeout`.

### Pitfall 6: rustdoc intra-doc links to removed items
**What goes wrong:** `make doc-check` (rustdoc zero-warning bar, inside `make clean-code`) fails on a link to a deleted item. Phase 43 hit exactly this (`61de2dec`).
**Where:** `paladin_error.rs` (the `LlmFailure` doc links `PaladinError::LlmError` and `is_retryable`), `aegis.rs:10-16` (module doc names the legacy pair; stale text, not a link, but wrong), `battalion/mod.rs` (the `BattalionError::Node` doc and the legacy `NodeError` doc cross-link each other), `conclave_execution_service.rs:77`, `commander.rs:379,414,439`, `error_aggregation.rs` module docs, `paladin_port.rs:210,825`.
**Avoid:** After each deletion run `cargo doc --workspace --no-deps` with the repo's flags (`make doc-check`).

### Pitfall 7: Unused imports and `-D warnings`
**What goes wrong:** Removing the wrappers leaves `use tokio::time::{Duration, timeout}`, `rand::Rng`, `sleep` imports unused in `formation_service.rs`, `phalanx_service.rs`, `campaign_service.rs`, `commander.rs` (`Duration` is still used by the Maneuver bridge), and `conclave_execution_service.rs`.
**Avoid:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` after every task.

### Pitfall 8: Planner hard gate on literals
**What goes wrong:** `gsd-tools verify` rejects a plan whose `<action>` body contains a literal that an acceptance criterion negative-greps for (`verify.cjs:166-231`).
**Avoid:** Acceptance criteria that assert absence (`! git grep -n 'PaladinError::LlmError'`) need `<!-- planner-discipline-allow: PaladinError::LlmError -->` (the Phase 25-06 plan did this) or must be phrased by concept in the action body.

### Pitfall 9: A struct-literal sweep hides behind `Default`
**What goes wrong:** Adding `aegis` breaks full `BattalionConfig { .. }` literals at `commander.rs:1928`, `tests/integration/battalion/load_test.rs:122,212` (and any in `tests/helpers`); `..Default::default()` sites such as `crates/doc-examples/src/orchestration.rs:20` keep compiling but must still drop `error_strategy`.
**Avoid:** Prefer builders in tests. `constructible_struct_adds_field` is already allowed crate-wide in `paladin-core`, so no new lint line for the addition.

### Pitfall 10: Examples and the doc-examples crate fail the build silently late
**What goes wrong:** `cargo test` builds examples, and `make check-doc-examples` compiles `paladin-doc-examples` separately (`cargo check --manifest-path crates/doc-examples/Cargo.toml`). A stale example shows up in CI, not in `-p paladin-battalion` runs.
**Avoid:** Run `cargo check --workspace --all-targets --all-features`, `make check-doc-examples`, `make check-examples` before declaring a wave done.

### Pitfall 11: Order of the D-27 diagnostic
**What goes wrong:** Running the diagnostic before the removals are final yields lint ids that change; writing rows from prediction (Phase 43's `AgentRuntimeDeps` row was corrected `N` to `Y` at closeout).
**Avoid:** Draft rows during implementation as `Y`/`TBD`; measure once at closeout; correct rows and entries in the same commit as the `Cargo.toml` allow lines.

## Blast radius

### LEGACY-01 / LEGACY-02 (BattalionConfig, RetryPolicy, ErrorStrategy, NodeError, timeouts)

| Surface | Sites (all `[VERIFIED: git grep]`) |
|---|---|
| Definition | `crates/paladin-core/src/platform/container/battalion/mod.rs`: `BattalionConfig` 36-55, builders 86-106, `RetryPolicy` 188-224, `ErrorStrategy` 239-250, `NodeError` 519-525, `node_errors` 581-582, `Timeout` 812-814, `Node` 838-839; ~20 in-file tests/doctests |
| Services | `formation_service.rs` (imports 11-16, wrapper 162-182, strategy match 239-280, helper 316-361, tests), `phalanx_service.rs` (wrappers 127-171, strategy 207-236, string-parsed errors 246-262, tests 703-755, 838-874), `campaign_service.rs` (wrapper 247-272, line 350 execute) |
| Commander | `commander.rs`: imports 8,22; wrapper 472-485; Conclave bridge 634-638; Maneuver bridge 880-892 (`timeout: Some(Duration::from_secs(self.config.timeout_seconds))`); builder default 1766-1771; validation 1774-1784; ~40 doc and test sites (e.g. 144, 176-182, 1432-1470, 1927-1933, 2021-2033, 2564-2670, 3202-3535) |
| Dead module | `crates/paladin-battalion/src/retry.rs` (whole file), `lib.rs` docs + `pub mod retry`, `src/application/services/battalion/mod.rs:18` |
| Error aggregation | `error_aggregation.rs:299,311,331` (test-only `Timeout(300)`; swap for another variant) |
| Conclave | `conclave.rs` (config 93-94,137,146-150; validate 313-324; error 442-443; From 459; tests 572-612), `conclave_execution_service.rs` (96-130, 343-396, tests 630, 718-748) |
| Heralds | `json_herald.rs:153, 443-462`, `markdown_herald.rs:395-399, 746`, `table_herald.rs:239-250, 615`; `tests/integration/battalion_herald_end_to_end_test.rs:323-351`; `tests/integration/commander_error_paths_test.rs:118-124, 249-250` |
| Facade / CLI | `src/application/cli/commands/battalion.rs:268, 434, 594, 607, 825` (607 is `ConclaveConfig::with_timeout`), `src/lib.rs:219` and `src/prelude.rs:22` (re-export `BattalionConfig`, unaffected), `src/application/cli/formatters/output.rs` (none needed) |
| Examples (8) | `commander_full_config.rs`, `phalanx_parallel.rs`, `formation_sequential.rs`, `campaign_workflow.rs`, `conclave_expert_panel.rs`, `commander_with_metadata_export.rs`, **`commander_auto.rs`, `commander_basic.rs`** (the last two are not in CONTEXT); `examples/README.md` (two `error_strategy` mentions are the Maneuver one: check before editing) |
| doc-examples crate | `crates/doc-examples/src/orchestration.rs:11,21` (anchor `formation`, used by `orchestration.md`; the `phalanx` anchor is also included by `deployment-topologies/battalion-orchestration.md:31`) |
| Docs | `user-guides/orchestration.md` (77-87, 280-300), `user-guides/battalion-patterns.md` (283-305), `appendix/conclave-pattern.md` (113, 159, 182, 209, 319, 631, 674, 890-894, 971, 998; YAML at 395-481 is the CLI schema), `getting-started/configuration.md:356-370` (fictional section), `appendix/battalion-vision-support.md:301`, `deployment/production.md:420-445`, `appendix/cli-usage.md:818,1010` (CLI YAML, review only) |
| Tests (root crate) | `tests/integration/commander_integration_tests.rs` (~28 sites), `commander_error_paths_test.rs` (~11), `battalion/{formation,phalanx,campaign}_integration_test.rs`, `battalion/load_test.rs`, `tests/unit/battalion/{formation_tests,campaign_service_tests}.rs`, `tests/helpers/mock_paladin_port.rs` (doc comment 74-76) |
| Same-named types that must NOT change | `aegis::RetryPolicy`, `node_error::NodeError`, `maneuver::ErrorStrategy` (`FailFast`/`ContinueParallel`/`IgnoreErrors`); `ManeuverConfig`; `BattalionCheckpointConfig` in `citadel.rs`; `engine/*`; `crates/doc-examples/src/fault_tolerance.rs` |

### LEGACY-03 (`PaladinError::LlmError`, predicates, Conclave, breaker)

| Surface | Sites |
|---|---|
| Variant, arms, tests | `paladin_error.rs` (49-50, 191, 256, tests 324, 335, 360, 474, 491, 508), `llm_failure.rs` (196 label arm, 251 test row, 437 test), `conclave_execution_service.rs` (347, 736, 739), `tests/unit/paladin_error_test.rs` (34-72) |
| Rustdoc / comments | `paladin_port.rs:210` (module-doc match arm), `:825` (`# Errors` list), `paladin-eval/src/runner.rs:759,764`, `temperature_service.rs:428`, `paladin_execution_service.rs:4918`, `paladin-battalion/src/lib.rs:54`, `llm_failure.rs` module docs (7, 30, 54, 94, 323) |
| Predicates | `is_retryable`: `circuit_breaker.rs:224, 281`, `llm_failure.rs:437`, `tests/unit/paladin_error_test.rs`; `is_terminal`: tests only (every other `is_terminal` hit is `RunStatus` / `PaladinStatus` / `IsTerminal`) |
| Breaker tests | `circuit_breaker.rs:477-478` and `tests/unit/circuit_breaker_test.rs:39,50,72,107,154` all trip the breaker with `ExecutionError`; they must use a Transient error (e.g. `PaladinError::Timeout(1)` or a Transient `LlmFailure`) |
| Docs | `docs/src/user-guides/paladin-agents.md:357`, `paladin-configuration.md:337`, `contributing/architecture-decisions.md:122,128`, `user-guides/tool-integration.md:842` (comment) |

### Behaviour deltas (feed §9.1; `[VERIFIED]` against `paladin_error.rs:188-303` and `conclave_execution_service.rs:343-381`)

| `PaladinError` | `transience()` | Conclave retried today | Conclave after | Breaker counted today | Breaker after |
|---|---|---|---|---|---|
| `Timeout` | Transient | yes | yes | no | **yes** |
| `LlmFailure` Transient / Permanent / Unknown | as carried | yes / no / no | yes / no / no | yes / yes / yes | yes / **no** / **no** |
| `ExecutionError` | Unknown | no | no | **yes** | no |
| `ConfigurationError`, `StopWordDetected`, `GarrisonRequired`, `MaxRetriesExceeded`, `GuardrailTripped`, `StructuredOutputInvalid` | Permanent | no | no | no | no |
| `ArmamentFailed` | Unknown | no | no | no | no |
| `CircuitBreakerOpen` | Transient | no | **yes** | no | **yes** |
| `GarrisonError` Storage / Tokenization | Transient | no | **yes** | no | **yes** |
| `GarrisonError` other | Permanent / Unknown | no | no | no | no |
| `ArsenalError` Timeout / TransportError | Transient | no | **yes** | no | **yes** |
| `ArsenalError` other | Permanent / Unknown | no | no | no | no |
| `LlmError(msg)` (removed) | — | by substring | — | yes | — |

Production reach: the breaker only ever wraps `generate()` and sees `LlmFailure` (Finding 6); the Conclave sees whatever a `PaladinPort` returns.

## Ordering constraints (tree must compile at every commit)

1. **ADR-0059 first** (opening plan). It is the licence for the removals; ADR-0051/0058 precedent writes the ADR at the start (Phase 30) or the close (Phase 43). Recommend: write it up front as `Accepted`, fill Code Conformance and the measured lint facts in the closeout plan. Also: PROMOTION.md row + next-free `0060`, ADR-0001 and ADR-0002 `## Status` become the bare word `Superseded` plus a prose line, ADR-0059 carries a `## Supersedes` line (PROMOTION.md supersession mechanism), PROJECT.md Key Decisions row and its *Out of Scope* X-03 bullet (lines ~1123-1125) gain the Phase 44 note.
2. **LEGACY-03 predicate side, no dependency on the Battalion config work, can run in parallel with stage 3a:**
   a. circuit breaker onto `transience()` + its tests (Pitfall: `ExecutionError` tests);
   b. Conclave `is_retryable_error` collapse (keep the old `sleep`/backoff for now);
   c. one atomic commit removing `LlmError`, `is_retryable`, `is_terminal` with every arm / test / rustdoc / doc site (compile-driven: delete, then `cargo check --workspace --all-targets --all-features` lists the rest).
3. **Battalion config work** (strictly ordered):
   a. Additive: `BattalionConfig.aegis` + `with_aegis` + `validate_aegis` (fix the 3-4 struct literals in the same commit) and the `pub(crate)` runner with unit tests (TDD, paused clock). Old fields still exist.
   b. **Atomic service cut:** Formation / Phalanx / Campaign / Conclave / Commander switch to the runner and `aegis`; timeout wrappers removed; `node_errors` retyped (core + services + three heralds + their tests + e2e/golden tests); Conclave backoff moves to `engine::retry` (D-12) and `calculate_retry_delay` goes; all tests migrated to `aegis`. At the end of 3b the legacy fields still compile but nothing reads them. (Split heralds out only if the core retype and heralds land in the same commit; they cannot be separated.)
   c. **Deletion commit:** remove the legacy fields, builders, `RetryPolicy`, `ErrorStrategy`, `NodeError`, `retry.rs` + facade re-export, `BattalionError::Timeout`, `ConclaveError::Timeout`, `ConclaveConfig.timeout_seconds` / `with_timeout`, their validation and tests; migrate CLI (`commands/battalion.rs`), the 8 examples, `doc-examples`, and the doc pages in the same commit (examples and `doc-examples` compile under `cargo test`/CI, so they cannot lag).
4. **Closeout plan:** D-27 diagnostic, §9.1 rows (start at `M-B-06`), §9.2 rows, allowlist entries, `Cargo.toml` allow lines (same commit), CHANGELOG (root + core + battalion + herald), `make api-surface-update`, ADR conformance, full gate.

## Code Examples

### Legacy to Aegis mapping (use verbatim in examples, §9.1/§9.2 wording and CHANGELOG)

| Legacy | Aegis equivalent on `BattalionConfig.aegis` |
|---|---|
| `ErrorStrategy::FailFast` | `on_error: None`, `retry: None` |
| `ErrorStrategy::ContinueOnError` | `on_error: Some(ErrorHandlerSpec::Absorb { fallback_delta: StateDelta::new() })` |
| `ErrorStrategy::RetryThenContinue` + `RetryPolicy { max_attempts: N, base_delay, max_delay, exponential_backoff, jitter }` | `retry: Some(aegis::RetryPolicy { max_attempts: N + 1, initial_interval: base_delay, backoff_factor: if exponential_backoff { 2.0 } else { 1.0 }, max_interval: max_delay, jitter, retry_on: RetryPredicate::TransientAndUnknown })` plus the `Absorb` handler above |
| `.with_timeout(secs)` | `timeout: Some(TimeoutPolicy { run_timeout: Some(Duration::from_secs(secs)), idle_timeout: None })`, now **per attempt** |
| `battalion::NodeError { node_name, error }` | `node_error::NodeError { node_id, attempt, transience, source }` |
| `BattalionError::Timeout(secs)` | `BattalionError::Node(NodeError { source: NodeErrorSource::Timeout(TimeoutKind::Run), .. })` |
| `PaladinError::LlmError(s)` | `PaladinError::LlmFailure { transience, status, provider, message }` (build with `paladin_battalion::llm_failure::to_paladin_error(&llm_error)`) |
| `err.is_retryable()` | `err.transience() == Transience::Transient` |
| `err.is_terminal()` | `err.transience() == Transience::Permanent` is the closest typed read; there is no equivalent for the old set (`Timeout`, `CircuitBreakerOpen` are now `Transient`). State this plainly in the row |

### Per-attempt timeout test under a paused clock

```rust
// Source: idiom established in crates/paladin-battalion/src/engine/retry.rs tests
#[tokio::test(start_paused = true)]
async fn formation_attempt_times_out_per_attempt_not_per_run() {
    let aegis = Aegis {
        timeout: Some(TimeoutPolicy { run_timeout: Some(Duration::from_secs(1)), idle_timeout: None }),
        ..Aegis::default()
    };
    // 10 steps of 600 ms each = 6 s total, every attempt under the 1 s bound -> succeeds (D-02).
    // One step of 1.5 s -> Err(BattalionError::Node(NodeError { source: Timeout(TimeoutKind::Run), attempt: 1, .. })).
}
```

### Typed Conclave predicate test (replaces `test_is_retryable_error`)

```rust
let cases = [
    (PaladinError::Timeout(10), true),
    (to_paladin_error(&LlmError::ProviderError { provider: "p".into(), status: 503, message: "x".into() }), true),
    (to_paladin_error(&LlmError::AuthenticationError("k".into())), false),
    (to_paladin_error(&LlmError::ProcessingError("opaque".into())), false), // Unknown: not retried (D-11)
    (PaladinError::ExecutionError("x".into()), false),
    (PaladinError::CircuitBreakerOpen, true),                               // newly retried (delta)
];
for (err, expected) in cases { assert_eq!(ConclaveExecutionService::is_retryable_error(&err), expected, "{err:?}"); }
```

### Source-tree guard (model: `tests/treasurer_vocabulary_guard.rs`)

A `tests/legacy_removal_guard.rs` that walks the tree (skip `.git`, `target`, `.planning`, `.project`, `CHANGELOG*.md`, `MIGRATION.md`, `docs/src/api-reference`, itself) and fails on: the legacy battalion `RetryPolicy` / `ErrorStrategy` / `NodeError` import paths, `BattalionError::Timeout`, `ConclaveError::Timeout`, `PaladinError::LlmError`, `\.is_retryable\(\)` on `PaladinError`, `tokio::time::timeout` or `timeout(` in the five legacy files (allowing the runner), `paladin_battalion::retry::` (the legacy module; `engine::retry` stays). Assemble the forbidden strings at run time and add the two positive controls (planted-violation tree reports all; clean tree reports none).

## State of the Art

| Old approach | Current approach | When changed | Impact |
|---|---|---|---|
| `BattalionConfig { timeout_seconds, retry_policy, error_strategy }` | `BattalionConfig { aegis }` | this phase (v0.11.0) | One policy vocabulary for engine, agent loop and legacy patterns |
| Whole-battalion wall clock | Per-attempt wall clock | this phase | A 10-step Formation may run 10x the bound; ChainOfCommand / Grove unbounded |
| `PaladinError::LlmError(String)` + substring retry sniffing | `LlmFailure { transience, status, provider, message }` + `transience()` | Phase 25 introduced; this phase removes the legacy variant | No message parsing anywhere |
| `is_retryable()` / `is_terminal()` | `transience()` | Phase 25 D-05 deprecated-in-prose; removed here | One classification source |
| Conclave own backoff (1,2,4,8,16 s, +-20 %) | `engine::retry::backoff_delay` (default 500 ms x2 to 60 s, additive jitter in [0,1)x) | this phase | Different delay schedule: note in §9.1 |

**Deprecated / outdated (to fix in docs):** the `orchestration.md` claim that `ContinueOnError` "passes the input through"; the `configuration.md` `battalion:` YAML section; `conclave-pattern.md` backoff table ("2^n seconds +-20 % jitter"); the `production.md` retry helper.

## Security Domain

`security_enforcement` is not set to `false` in `.planning/config.json` (the file has only `workflow` keys), so this section applies. CLAUDE.md / `security.instructions.md`: `make security`, `cargo clippy -- -D warnings`, and the manual credential-handling review; **no Snyk step; CodeQL is advisory-only**.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth surface touched) |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | `BattalionConfig::validate_aegis` (reject unsupported handler / predicate, zero durations, `max_attempts == 0`); legacy-JSON deserialisation tests; serde `default` |
| V6 Cryptography | no | — |
| V7 Error Handling and Logging | yes | `node_errors` text comes from `PaladinError::to_string()`, already redacted upstream (`LlmFailure.message` is redacted before bounding, D-34); heralds render the same text as before plus id / attempt / transience, which are not secrets; no new log line may interpolate a key |
| V11 Business Logic / Resilience | yes | Removing whole-run timeouts is a denial-of-service posture change for ChainOfCommand / Grove (unbounded); documented in §9.1 and the docs; the circuit breaker now ignores Permanent failures so a bad key no longer trips it (documented) |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Hung Paladin call with no bound (ChainOfCommand / Grove / Council aggregation) | Denial of service | Document; per-call bounds belong to the port / adapter; optionally keep `CommanderBuilder`'s 300 s per-attempt default |
| Retry storm from `TransientAndUnknown` on a non-idempotent step | Denial of service / Tampering | Default stays `TransientOnly`; examples explain `TransientAndUnknown` is an opt-in |
| Credential leakage through the richer herald line | Information disclosure | Render only `node_id`, `attempt`, `transience` and the already-redacted `Display`; reuse `to_node_error_source` (message is `err.to_string()` of a redacted error); add a herald test with a secret-shaped string in an `LlmFailure.message` that was redacted upstream |
| Old JSON replayed into the new `node_errors` type | Tampering (shape confusion) | Strict typed deserialisation; no untagged compat enum |

Manual review checklist for the closeout (per `security.instructions.md`): response bodies still redacted before truncation (no change in `redaction.rs` paths); no log statement interpolates an API key; no HTTP client is added or changed (this phase touches none, so `Policy::none()` redirect posture is unchanged). Record "no credential-handling code changed" with the `git diff` evidence, as 43-13 did.

## Validation Architecture

> `workflow.nyquist_validation` is not set in `.planning/config.json` (absent = enabled).

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (libtest) + doc tests; `tokio::test` with `start_paused`; coverage via `cargo llvm-cov` in CI only |
| Config | `Cargo.toml` `[[test]]` entries; root `tests/lib.rs` (target `lib`, contains `integration` and `unit` modules) plus explicit `unit` target; per-crate `src` unit tests |
| Quick run (per task) | `cargo test -p paladin-ai-core --lib <module>` (verified: `paladin_error` ran 8 tests) and `cargo test -p paladin-battalion --lib <module>` (verified: `conclave_execution_service` ran 9 tests in ~3 s after a ~1.5 min first build) |
| Per-wave | `cargo check --workspace --all-targets --all-features` then `cargo test --workspace --no-fail-fast` (Phase 43 gate form: `-- --skip build_run_api_persists` for two environment-bound web tests) |
| Phase gate | `make clean-code` (fmt, clippy `-D warnings`, lint-shell, check, doc-check, check-api-examples), `cargo test --workspace`, `cargo test --workspace --doc`, `make security`, `make check-gates` (includes `check-migration-allowlist`), `make check-doc-examples`, `make check-doc-config`, `make check-examples`, `make api-surface`, `./scripts/check-changelogs.sh` |

### Phase Requirements to Test Map

| Req ID | Behaviour | Test type | Automated command | File exists? |
|--------|-----------|-----------|-------------------|--------------|
| LEGACY-01 | Legacy `battalion::{RetryPolicy, ErrorStrategy, NodeError}` gone; Aegis / Flow / structured `NodeError` untouched | guard test + compile | `cargo test --test legacy_removal_guard`; `cargo check --workspace --all-targets --all-features`; `cargo test -p paladin-ai-core aegis node_error`; `cargo test -p paladin-battalion maneuver`; `cargo check --manifest-path crates/doc-examples/Cargo.toml` | guard: Wave 0 (new); rest exist |
| LEGACY-01 | `BattalionConfig` serde: new shape round-trips; v0.10 JSON (three legacy keys) still deserialises with `aegis` default | unit | `cargo test -p paladin-ai-core battalion::tests` | new tests in `battalion/mod.rs` |
| LEGACY-01 | `Absorb` collects typed `node_errors` (Formation, Phalanx), `Completed` + non-zero `paladin_failure_count`; `None` fails fast; Route / Custom / `Custom` predicate / zero durations rejected | unit | `cargo test -p paladin-battalion formation_service phalanx_service campaign_service` and `cargo test -p paladin-ai-core validate_aegis` | new |
| LEGACY-01 | Heralds render typed errors (id, attempt, transience, Display); JSON shape golden updated | unit + integration | `cargo test -p paladin-herald`; `cargo test -p paladin-ai --test lib integration::battalion_herald_end_to_end_test` | exist, to update |
| LEGACY-02 | Per-attempt timeout surfaces `BattalionError::Node(.. Timeout(Run))` on Formation / Phalanx / Campaign; no whole-run bound; `idle_timeout` degrades to wall clock; retry after timeout under a retry policy | unit (paused clock) | `cargo test -p paladin-battalion aegis_attempt formation_service phalanx_service campaign_service` | new (replaces `test_timeout_enforcement` in phalanx / commander and the two integration tests) |
| LEGACY-02 | No `timeout(` left outside the runner in the five legacy files | guard test | `cargo test --test legacy_removal_guard` | Wave 0 |
| LEGACY-03 | `transience()` table without `LlmError`; Conclave predicate table incl. deltas; Conclave retry uses engine backoff (paused-clock sequence at `jitter:false`) and `retry_attempts`; breaker counts Transient only | unit | `cargo test -p paladin-ai-core paladin_error`; `cargo test -p paladin-battalion conclave_execution_service llm_failure`; `cargo test -p paladin-ai --lib circuit_breaker`; `cargo test -p paladin-ai --test unit circuit_breaker_test paladin_error_test` | exist, to rewrite |
| LEGACY-03 | `grep` of the tree for the removed variant is empty outside history | guard test | `cargo test --test legacy_removal_guard` | Wave 0 |
| LEGACY-04 | §9.2 rows == allowlist entries (set-equal) and every `Y` row has an entry | script | `make check-migration-allowlist`; `bash tests/scripts/check-migration-allowlist_test.sh` | exist |
| LEGACY-04 | Measured lints match rows (D-27 diagnostic captured in the closeout SUMMARY) | manual-only (needs the binary) | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.10.1 --release-type minor` per affected package, crate-wide allows disabled then restored byte-for-byte | manual; justified: tool not installed locally |
| LEGACY-04 | ADR-0059 exists, PROMOTION next-free is 0060, ADR-0001/0002 Status is `Superseded` | script / shell check | `ls .planning/decisions/0059-*.md`; `grep -n 'Next free ADR number: 0060' .planning/decisions/PROMOTION.md`; `sed -n '/^## Status/,/^## /p' .planning/decisions/0001-*.md` | new |
| LEGACY-04 | Examples build; docs examples compile; YAML parses; API baseline updated; CHANGELOGs present | gate commands | `make check-examples`; `make check-doc-examples`; `make check-doc-config`; `make api-surface` (after `make api-surface-update`); `./scripts/check-changelogs.sh` | exist |

### Sampling Rate
- **Per task commit:** the narrowest `cargo test -p <crate> --lib <module>` for the files touched, plus `cargo check --workspace --all-targets` when a public item was deleted.
- **Per wave merge:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --no-fail-fast`, `make check-doc-examples`.
- **Phase gate:** full `make clean-code` + workspace tests + doc tests + `make security` + `make check-gates` + `make api-surface`; coverage is CI-only (`cargo-llvm-cov` not installed locally), so keep the floor by writing the new tests TDD-first and deleting legacy tests together with the code they covered.

### Wave 0 Gaps
- [ ] `tests/legacy_removal_guard.rs` — covers LEGACY-01/02/03 absence claims (with positive controls); no `Cargo.toml` entry is needed: root-level `tests/*.rs` files are auto-discovered (`tests/treasurer_vocabulary_guard.rs` has no `[[test]]` entry and runs as `cargo test --test treasurer_vocabulary_guard`) [VERIFIED: `grep` of `Cargo.toml`, no `autotests` key].
- [ ] `crates/paladin-battalion/src/aegis_attempt.rs` unit tests (timeout kinds, retry gating per predicate, attempts count, cancellation during backoff, paused clock).
- [ ] New `BattalionConfig` serde / `validate_aegis` tests in `battalion/mod.rs`.
- [ ] Framework: none to install (tokio `test-util` already a dev-dependency in both crates).
- [ ] Verify the exact `--test lib` filter paths for the root integration tests with `cargo test -p paladin-ai --test lib -- --list` once (not run this session; the `integration::<module>` filter form is `[ASSUMED]`).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc / cargo | everything | yes | 1.97.1 (pinned by `rust-toolchain.toml`) | — |
| rustfmt, clippy | `make clean-code` | yes | with toolchain | — |
| `nightly-2026-09-20` + `cargo-public-api` 0.52.0 | `make api-surface[-update]` | yes | as pinned | — |
| `cargo-semver-checks` 0.50.0 | D-27 diagnostic, LEGACY-04 | **no** | — | Download the `v0.50.0` `x86_64-unknown-linux-gnu` release asset with `curl` through the session proxy into the session scratchpad (Phase 43-13 method; `302` confirmed reachable this session), never commit it; restore each manifest from a byte copy and confirm `git diff --quiet <manifest>` |
| `cargo-llvm-cov` | 82 % floor | **no** | — | CI `coverage` job gates it; state "not measured locally" in the closeout |
| `cargo-audit`, `cargo-deny` | `make security` | yes | installed | — |
| `shellcheck`, `python3` + PyYAML, `jq` | `lint-shell`, `check-doc-config`, scripts | yes | present | — |
| `mdbook` | building the book | **no** | — | Not required: `check-doc-examples.sh` compiles the examples crate and syntax-scans fences without mdBook |
| `redis-server`, `docker` | Redis-backed integration suites | yes | present | Not needed by this phase |

**Missing with no fallback:** none. **Missing with fallback:** `cargo-semver-checks` (download prebuilt), `cargo-llvm-cov` (CI-only), `mdbook` (not needed).

## Open Questions (RESOLVED)

All ten questions were resolved during planning (2026-10-09). Each carries an inline `RESOLVED` line naming the plan that implements the answer and the ADR-0059 Decision subsection that records it (ADR-0059 is written by plan 44-02).

1. **Default timeout after the cut.** `BattalionConfig::new` loses its implicit 300 s bound; `CommanderBuilder`'s documented default config ("Timeout: 300 seconds") would become unbounded.
   - Known: D-01 says `#[serde(default)] aegis: Aegis` (default has no timeout).
   - Unclear: whether the builder default should keep a bound.
   - **Recommendation:** `BattalionConfig::new` uses `Aegis::default()` (follow D-01 literally; §9.1 states the default is now unbounded); `CommanderBuilder`'s default-config branch sets `run_timeout: Some(300 s)` so its documented default survives.
   - **RESOLVED** (recommendation adopted): `BattalionConfig::new` keeps `Aegis::default()` with no timeout (plan 44-01); `CommanderBuilder`'s default config arms a 300 s per-attempt `run_timeout` (plan 44-07 Task 1, test `commander_builder_default_bounds_each_attempt`). Recorded in ADR-0059 Decision (c) as a planner resolution beside D-01, and in MIGRATION.md 9.1 row `M-B-06` (plan 44-11).
2. **`ConclaveError::Timeout(u64)`.** CONTEXT retires `BattalionError::Timeout` only.
   - **Recommendation:** remove `ConclaveError::Timeout` (no producer remains) and give `ConclaveError` its own §9.2 row; the Conclave surfaces an expert attempt timeout as `PaladinError::Timeout` (Transient) so it is retried, and an aggregator timeout as `ConclaveError::AggregatorFailed`.
   - **RESOLVED** (recommendation adopted; extends D-03): `ConclaveError::Timeout` and its `From` arm are removed (plan 44-10 Task 3); an expert attempt timeout is a retried `PaladinError::Timeout` and an aggregator timeout is `ConclaveError::AggregatorFailed` (plan 44-07 Task 2, tests `conclave_expert_attempt_times_out_and_retries`, `conclave_aggregator_timeout_is_aggregator_failed`); `ConclaveError` gets its own 9.2 row (plan 44-11 Task 2). Recorded in ADR-0059 Decision (d).
3. **Non-timeout fail-fast errors on Formation / Phalanx / Campaign.**
   - **Recommendation:** keep `BattalionError::PaladinError(String)` (unchanged contract, minimal test churn); only the per-attempt timeout surfaces as `BattalionError::Node(NodeError)` (D-03). `AttemptFailure.node_error` still feeds `node_errors` under `Absorb`.
   - **RESOLVED** (recommendation adopted): non-timeout fail-fast failures stay `BattalionError::PaladinError(String)`; a timed-out final attempt is `BattalionError::Node(NodeError)` with a `Timeout` source on Formation (plan 44-04) and Campaign (plan 44-01); Phalanx keeps its collect-then-fail `BattalionError::AggregationError` listing each failure's `NodeError` display (plan 44-05). Recorded in ADR-0059 Decision (d).
4. **Campaign and `on_error`.** Campaign had no continue mode.
   - **Recommendation:** Campaign honours `timeout` and `retry` only and documents that `on_error` is ignored (parity with the old `error_strategy`, which it also ignored); do not invent skip-subgraph semantics.
   - **RESOLVED** (recommendation adopted): Campaign honours `aegis.timeout` and `aegis.retry` and ignores `on_error` with a logged warning (plan 44-01, test `campaign_ignores_on_error_and_fails_fast`; documented by plan 44-09). Recorded in ADR-0059 Decision (f) and MIGRATION.md 9.1 row `M-B-07` (plan 44-11).
5. **`RetryPredicate::Custom` on legacy patterns** always admits `false` (needs the engine registry).
   - **Recommendation:** reject it in `validate_aegis` alongside `Route` / `Custom` handlers rather than silently never retrying.
   - **RESOLVED** (recommendation adopted): `validate_aegis` rejects `RetryPredicate::Custom` together with `Route` / `Custom` handlers (plan 44-01 Task 2, the `validate_aegis_*` tests; reused by the Commander in plan 44-07). Recorded in ADR-0059 Decision (b).
6. **Commander to Conclave attempts when `aegis.retry` is `None`.** Today every Commander Conclave gets `max_attempts(3) - 1 = 2` retries.
   - **Recommendation:** `None` leaves `ConclaveConfig`'s default of 2; `Some(p)` sets `p.max_attempts.saturating_sub(1)` (clamped to 5 by `with_retry_attempts`).
   - **RESOLVED** (recommendation adopted; refines D-04's literal mapping): `aegis.retry = None` keeps the default of 2; `Some(p)` sets `retry_attempts = p.max_attempts.saturating_sub(1)`, clamped to 5, because `max_attempts` counts total attempts (Finding 4) while `retry_attempts` counts retries after the first (plan 44-07 Task 1, test `commander_conclave_bridge_derives_retry_attempts`). The `- 1` refinement of D-04 is stated explicitly in ADR-0059 Decision (e) ("This refines D-04") and in MIGRATION.md 9.1 row `M-B-09` (plan 44-11).
7. **D-10 literal versus `retry_on`.** D-10 fixes `is_retryable_error` to `transience() == Transient`; Aegis `retry_on` could admit `Unknown`.
   - **Recommendation:** follow D-10 literally for the Conclave (predicate is `Transient`-only; the policy supplies backoff shape), and note in the field docs that `retry_on` is honoured by Formation / Phalanx / Campaign but the Conclave is `Transient`-only. If the operator prefers one rule everywhere, use `retry_on.admits(err.transience())` in the Conclave too; identical under the default.
   - **RESOLVED** (D-10 followed literally): the Conclave predicate is `transience() == Transient` regardless of `aegis.retry.retry_on` (plan 44-03 Task 1, test `conclave_retry_predicate_is_transient_only`; the retry loop in plan 44-07 Task 2); `retry_on` is honoured by Formation, Phalanx and Campaign (plans 44-01, 44-04, 44-05) and the split is documented in the `aegis` field rustdoc (plan 44-01) and the guides (plan 44-09). Recorded in ADR-0059 Decision (h).
8. **CLI YAML key.** `src/application/cli/config/battalion_config.rs:284` (`ConclaveConfig.timeout_seconds`) and the CLI templates are the CLI's own schema.
   - **Recommendation:** keep the YAML keys, map to `Aegis.timeout.run_timeout`, and reword the template / docs comments to "per attempt".
   - **RESOLVED** (recommendation adopted): the CLI keeps its YAML `timeout_seconds` keys and maps them to `aegis.timeout.run_timeout`, reworded as per attempt (plan 44-08 Task 1; CLI and Conclave pages in plan 44-09 Task 2). Recorded in ADR-0059 Decision (c) and MIGRATION.md 9.1 row `M-B-06` (plan 44-11).
9. **`configuration.md` `battalion:` section** is fictional (Finding 9). **Recommendation:** replace with a pointer to `BattalionConfig.aegis`; do not add a YAML schema.
   - **RESOLVED** (recommendation adopted): both fictional blocks go -- the `## Battalion (Multi-agent Orchestration)` section's `battalion:` YAML block (near line 358) and its env-var line are replaced by a pointer to `BattalionConfig.aegis`, and the `battalion:` stanza in the `## Complete Example (config.yml)` block (near line 706) is deleted; no YAML schema is added (plan 44-09 Task 1). Recorded in ADR-0059 Decision (j).
10. **`paladin_battalion::retry` deletion** (Finding 1) is implied by D-12 but not stated; confirm it is in scope (it is unavoidable: the module cannot compile without the legacy type).
   - **RESOLVED** (in scope): the legacy module, its `pub mod` line and the facade re-export are deleted and the API baseline is regenerated (plan 44-10 Task 1); its 9.2 row is `paladin-battalion | retry` (plan 44-11 Task 2). Recorded in ADR-0059 Decision (i).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Lints the D-27 diagnostic will fire: `paladin-ai-core` `struct_missing` (`RetryPolicy`, `NodeError`), `enum_missing` (`ErrorStrategy`), `enum_variant_missing` (`BattalionError::Timeout`, `ConclaveError::Timeout`, `PaladinError::LlmError`), `struct_pub_field_missing` and `inherent_method_missing` (already crate-allowed), `constructible_struct_adds_field` (already allowed); `paladin-battalion` `module_missing` (and possibly `function_missing`) for `retry`; **none** for the `node_errors` field retype (no lint models a changed field type, the Phase 40 `register-only` precedent) | Architecture / Ordering | Wrong lint ids mean wrong allowlist entries and missing `Cargo.toml` allow lines; mitigated because the closeout measures instead of trusting this list |
| A2 | The facade crate `paladin-ai` does not double-report removed foreign (re-exported) items, so entries are per defining crate | Ordering | If it does, extra `paladin-ai` rows / entries and a root `Cargo.toml` lint table change; Phase 31/32/43 precedent says per defining crate |
| A3 | `make api-surface` changes only by the `retry` re-export line (baseline lists facade-level items; `BattalionConfig` / `BattalionError` re-exports stay) | Validation | A larger diff is still handled by `make api-surface-update` + CHANGELOG; low risk |
| A4 | `cargo test -p paladin-ai --test lib integration::<module>` is the right filter form for the root integration modules | Validation | Wrong command text only; verify with `--list` |
| A5 | No other consumer of `paladin_battalion::retry` exists outside the files `git grep` found | Findings | A missed consumer fails `cargo check --workspace --all-targets`, which is the authoritative discovery step anyway |
| A6 | Open Question recommendations 1-8 are acceptable defaults | Open Questions | Each is a planner / discuss decision; none contradicts a locked D-xx |
| A7 | Stored v0.10 `BattalionResult` JSON with non-empty `node_errors` exists nowhere in-repo except tests | Runtime State | If a fixture does, a golden file needs updating (the `git grep` for `node_name` found only the tests listed) |

## Project Constraints (from CLAUDE.md)

- **TDD (Red-Green-Refactor), coverage floor 82 % workspace line coverage** (ADR-0006), enforced by CI `cargo llvm-cov --fail-under-lines`; all public APIs need doc tests (doctests on `BattalionConfig`, `with_aegis`, and any public replacement must compile).
- **Dependencies flow inward only:** core imports nothing; `paladin-core` stays dependency-free (so `validate_aegis` lives in core, the runner in `paladin-battalion`).
- **Ubiquitous language:** Medieval Military terms (Paladin, Battalion, Garrison, Arsenal, Citadel, Herald, Quest, Aegis, Battlefield, Waypoint); any new module name must fit (`aegis_attempt.rs` is acceptable; avoid generic names).
- **Before committing a parent task:** `cargo test`, `cargo fmt --check`, `cargo clippy`, `make api-surface` (intentional change: `make api-surface-update` + CHANGELOG entry), conventional-commit message; stop after each major task.
- **Security:** `make security`, `cargo clippy -- -D warnings`, manual credential review; **never reintroduce Snyk**; CodeQL is advisory.
- **No `unwrap()` / `expect()` / `panic!` in library code**; prefer borrowing; lazy iterators. (The attempt-runner sketch's `unwrap`s and `unreachable!` must be restructured.)
- **Commit attribution** (system reminder): end commit messages with the `Co-Authored-By` and `Claude-Session` lines given for this session.
- Research does not change source; this file is the only artefact.

## Sources

### Primary (HIGH confidence — read in this tree this session)
- `.planning/phases/44-legacy-clean-break-removal/44-CONTEXT.md` (all of it); `.planning/REQUIREMENTS.md` LEGACY-01..04; `.planning/decisions/{PROMOTION,0001,0002,0051,0058}.md`; `.planning/phases/43-rate-pacing/43-13-SUMMARY.md` (D-27 method and gate list)
- `crates/paladin-core/src/platform/container/{aegis,node_error,transience,paladin_error,waypoint}.rs`; `battalion/{mod,conclave}.rs`
- `crates/paladin-battalion/src/{formation,phalanx,campaign,commander,conclave_execution,error_aggregation,llm_failure,retry,lib}.rs`, `engine/retry.rs`
- `crates/paladin-herald/src/{json,markdown,table}_herald.rs`; `src/infrastructure/resilience/circuit_breaker.rs`; `src/application/services/paladin/paladin_execution_service.rs` (breaker call sites); `src/application/cli/commands/battalion.rs`; `src/application/services/battalion/mod.rs`
- `tests/helpers/mock_paladin_port.rs`, `tests/integration/{commander_*,battalion/*}`, `tests/unit/{circuit_breaker_test,paladin_error_test}.rs`, `tests/treasurer_vocabulary_guard.rs`
- `MIGRATION.md` §9.1 / §9.2; `.cargo/semver-checks-allowlist.toml`; `scripts/check-migration-allowlist.sh`; `.github/workflows/ci.yml` (semver job); `Makefile`; `scripts/check-doc-examples.sh`, `check-doc-config.sh`; crate `Cargo.toml` lint tables
- Executed: `cargo test -p paladin-ai-core --lib paladin_error` (8 passed), `cargo test -p paladin-battalion --lib conclave_execution_service` (9 passed, 3.0 s); `rustup toolchain list`; `curl -I` of the `cargo-semver-checks` v0.50.0 release asset (302)

### Secondary (MEDIUM)
- `.github/copilot-instructions.md`, `.github/instructions/*.md` (project rules via CLAUDE.md)
- `/workspace/.claude/gsd-core/bin/lib/verify.cjs` (planner literal gate)

### Tertiary (LOW)
- None. No web search was needed: the phase is wholly internal to the repository.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components exist in-tree; signatures read directly.
- Architecture (runner, ordering, config shape): HIGH for facts, MEDIUM for the exact runner API (a sketch; the planner owns the final shape).
- Pitfalls: HIGH — each is tied to a specific line or an executed observation.
- Predicted semver lint ids: MEDIUM-LOW until the D-27 diagnostic runs (Assumption A1).

**Research date:** 2026-10-09
**Valid until:** 2026-10-23 for the file/line references (the tree is moving: Phase 41 work is pending in unrelated files); re-grep line numbers before editing, they are anchors, not contracts.
