# Phase 44: Legacy Clean-Break Removal - Pattern Map

**Mapped:** 2026-10-09
**Files analyzed:** 41 (grouped below into 22 pattern assignments)
**Analogs found:** 40 / 41 (the one new runtime file, the shared attempt runner, has no exact analog; it is assembled from `engine/retry.rs` and `llm_failure.rs`)

Line numbers were read this session. The 44-RESEARCH.md "Findings That Extend CONTEXT.md" (ten items) remain authoritative for blast radius; this file only says what to copy from where.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-battalion/src/aegis_attempt.rs` (NEW, `pub(crate)`) | service helper | request-response (retry+timeout loop) | `crates/paladin-battalion/src/engine/retry.rs` + `llm_failure.rs::to_node_error_source` | role-match (composition) |
| `crates/paladin-core/src/platform/container/battalion/mod.rs` | model (config/result/error) | transform | itself (`BattalionConfig`, `BattalionError::Node`) + `aegis.rs::Aegis` | exact |
| `crates/paladin-core/src/platform/container/battalion/conclave.rs` | model | transform | same file (`ConclaveConfig`, `ConclaveError`) | exact |
| `crates/paladin-core/src/platform/container/paladin_error.rs` | model (error enum) | transform | same file (`transience()` table) | exact |
| `crates/paladin-battalion/src/formation_service.rs` | service | request-response (sequential) | same file; new body from `aegis_attempt.rs` | exact |
| `crates/paladin-battalion/src/phalanx_service.rs` | service | concurrent fan-out | same file | exact |
| `crates/paladin-battalion/src/campaign_service.rs` | service | DAG, per-node | same file | exact |
| `crates/paladin-battalion/src/conclave_execution_service.rs` | service | request-response + retry | `engine/retry.rs` (`backoff_delay`, `wait_backoff`) | role-match |
| `crates/paladin-battalion/src/commander.rs` | service (router) | request-response | same file | exact |
| `crates/paladin-battalion/src/retry.rs` (DELETE) + `src/application/services/battalion/mod.rs:18` re-export | utility | n/a | deletion; replacement is `engine/retry.rs` | n/a |
| `crates/paladin-battalion/src/llm_failure.rs`, `error_aggregation.rs` | utility | transform | same files | exact |
| `src/infrastructure/resilience/circuit_breaker.rs` | infrastructure | request-response | `PaladinError::transience()` call style in conclave predicate | role-match |
| `crates/paladin-herald/src/{json,markdown,table}_herald.rs` | adapter (presenter) | transform | same files | exact |
| `src/application/cli/commands/battalion.rs`, `.../templates/battalion_template.rs` | CLI/config | request-response | same files | exact |
| `examples/{commander_full_config,phalanx_parallel,formation_sequential,campaign_workflow,conclave_expert_panel,commander_with_metadata_export,commander_auto,commander_basic}.rs`, `crates/doc-examples/src/orchestration.rs` | example | n/a | `aegis.rs` doc examples | role-match |
| tests: `tests/helpers/mock_paladin_port.rs`, `tests/integration/{battalion/*,commander_*,battalion_herald_end_to_end_test}.rs`, `tests/unit/{circuit_breaker_test,paladin_error_test,battalion/*,herald_consolidation_test}.rs` | test | n/a | `engine/retry.rs` tests (paused clock) | role-match |
| NEW guard test (symbols gone) e.g. `tests/legacy_removal_guard.rs` | test | file-I/O scan | `tests/treasurer_vocabulary_guard.rs` | exact |
| `.planning/decisions/0059-legacy-clean-break-removal.md` (NEW) | ADR | n/a | `.planning/decisions/0051-token-economy-versioning-x03-supersession.md`, `0058-rate-pacing-cadence.md` | exact |
| `.planning/decisions/0001-battalion-config.md`, `0002-battalion-result.md` | ADR (edit) | n/a | PROMOTION.md "Supersession mechanism" | exact |
| `MIGRATION.md` §9.1 / §9.2 | docs/register | n/a | Phase 43 M-B-05 and `paladin-ports | LlmError` row | exact |
| `.cargo/semver-checks-allowlist.toml` | config | n/a | Phase 43 block (end of file) | exact |
| `crates/paladin-core/Cargo.toml` (+ new table in `crates/paladin-battalion/Cargo.toml`) | config | n/a | `crates/paladin-core/Cargo.toml:73-78` | exact |
| `CHANGELOG.md` `[Unreleased]` | docs | n/a | existing `### Removed` at line 649 | exact |
| mdBook pages (orchestration, battalion-patterns, conclave-pattern, configuration, production, paladin-agents, paladin-configuration, architecture-decisions, battalion-vision-support, battalion-orchestration) | docs | n/a | n/a | no code analog |

## Pattern Assignments

### `crates/paladin-battalion/src/aegis_attempt.rs` (NEW; helper, retry+timeout loop)

**Analog:** `crates/paladin-battalion/src/engine/retry.rs` (reuse, do not copy) and `crates/paladin-battalion/src/llm_failure.rs`.

**Reuse these signatures verbatim** (engine/retry.rs:48-117):
```rust
pub fn backoff_delay(policy: &RetryPolicy, attempt: u32) -> Duration   // attempt >= 2 = the attempt about to run
pub async fn wait_backoff(delay: Duration, token: &Option<CancellationToken>) -> bool // false = cancelled, do not retry
pub fn should_retry(policy: &RetryPolicy, err: &NodeError, _attempt: u32) -> bool {
    policy.retry_on.admits(err.transience)
}
```
Imports to mirror (engine/retry.rs:20-27): `paladin_core::platform::container::aegis::RetryPolicy`, `...::node_error::NodeError`, `tokio_util::sync::CancellationToken`.

**PaladinError to NodeErrorSource** (llm_failure.rs, `to_node_error_source`, doc at ~126-170): call it, do not re-derive. `LlmFailure` becomes `NodeErrorSource::Llm { kind: "LlmFailure", status, provider, message }`, any other variant becomes `NodeErrorSource::Paladin { kind }`. A timeout is `NodeErrorSource::Timeout(TimeoutKind::Run|Idle)` with `Transience::Transient`. `node_id = NodeId::new(paladin.node.name.clone())`.

**Core loop:** use RESEARCH.md Pattern 2 sketch, but restructure so there is no `unwrap()` / `unreachable!()` (library code rule; clippy `-D warnings`). Exactly one `tokio::time::timeout` site in the crate's legacy files.

**Test idiom to copy** (engine/retry.rs header, lines 12-18): `#[tokio::test(start_paused = true)]` and measure `tokio::time::Instant::now()` deltas around an awaited call. Never assert on real sleeps.

---

### `crates/paladin-core/src/platform/container/battalion/mod.rs` (model; config reshape)

**Analog:** same file, `BattalionConfig` lines 36-75, and `Aegis` (`aegis.rs:43-55`, derives `Serialize/Deserialize/Default`).

**Before** (mod.rs:36-75):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattalionConfig {
    pub name: String,
    pub description: Option<String>,
    pub timeout_seconds: u64,
    pub retry_policy: RetryPolicy,
    pub error_strategy: ErrorStrategy,
    pub metadata_output_dir: Option<PathBuf>,
}
```
**After:** replace the three fields with `#[serde(default)] pub aegis: Aegis`; `new()` sets `aegis: Aegis::default()`; builder `with_aegis(mut self, aegis: Aegis) -> Self`. Add `validate_aegis()` here (pure, dependency-free; RESEARCH Pattern 5). Keep no `deny_unknown_fields` so pre-phase JSON still deserialises; pin with a test (the `#[serde(default)]` precedent from `node_errors` / `served_by`).

**Error enum:** delete `BattalionError::Timeout(u64)` (mod.rs:812-814) and its test literal (mod.rs:1055). A timeout surfaces through the existing `BattalionError::Node(NodeError)` variant (structured `node_error::NodeError`, path-qualified; see MIGRATION.md §9.2 `BattalionError` row). Add no alias for the removed names (D-00c). Docs on the struct (`use ...::{BattalionConfig, ErrorStrategy}; .with_timeout(300)` doctest at lines ~30-34) must be rewritten to the Aegis form, since doc tests are mandatory.

---

### `crates/paladin-core/src/platform/container/paladin_error.rs` (error enum; delete variant + predicates)

**Analog:** same file. `is_retryable` is at line 188, `is_terminal` at 201, `transience()` at 219 (the classification source, keep). Doc comment at line 56 links `PaladinError::is_retryable`; fix it. Tests at 323-335 (`test_is_retryable`, `test_is_terminal`) are deleted, not ported; port any still-valuable assertion onto `transience()`.

Pattern for exhaustive matches over this `#[non_exhaustive]` enum: variants removed in the same commit as every in-tree arm (`StopReason`, `BattalionError` precedents). Remaining `LlmError(String)` mentions to scrub (CONTEXT D-09): `paladin_port.rs` module docs, `llm_failure.rs` label table, `paladin-eval/src/runner.rs` comment, `temperature_service.rs`, `paladin_execution_service.rs` rustdoc, `docs/src/user-guides/paladin-agents.md:357`. Do NOT touch `HandoffError/PromptError/PlanningError::LlmError` (D-15).

---

### `crates/paladin-battalion/src/formation_service.rs` (service, sequential)

**Analog:** itself. Wrapper to delete (lines 162-182):
```rust
let timeout_duration = Duration::from_secs(formation.config.timeout_seconds);
match timeout(timeout_duration, self.execute_internal(formation, initial_input, battalion_id)).await {
    Ok(result) => { ... result }
    Err(_) => { ... Err(BattalionError::Timeout(formation.config.timeout_seconds)) }
}
```
Replace with a direct `self.execute_internal(...).await` after `formation.config.validate_aegis()?` (services can be built without a Commander).

**Continue-past-failure pattern to keep** (lines 232-258), re-keyed from `ErrorStrategy` to `aegis.on_error`:
```rust
ErrorStrategy::ContinueOnError | ErrorStrategy::RetryThenContinue => {
    node_errors.push(NodeError { node_name: paladin.node.name.clone(), error: error.to_string() });
    aggregated_error.add_error(error);
    current_input = String::new();   // continue with empty output
}
```
becomes `Some(ErrorHandlerSpec::Absorb { .. })` pushing `AttemptFailure.node_error` (the structured `node_error::NodeError`), and `None` returns the error (fail fast). `RetryThenContinue` is `aegis.retry` + `Absorb`. Drop the private `execute_paladin_with_strategy` retry and the `use crate::retry` import (line 12; call at 339).

---

### `crates/paladin-battalion/src/phalanx_service.rs` and `campaign_service.rs`

**Analog:** themselves; wrappers at phalanx 127-171, campaign 247-272. Same replacement: call the shared runner per Paladin / per node, no `tokio::timeout`. Phalanx: carry `Vec<AttemptFailure>` through `execute_collect_all` / `execute_first_success` / `execute_majority` instead of the `"{name}: {error}"` string with `split_once` (lines 246-262); keep "collect every failure then `AggregationError`" for `on_error == None`. Campaign: `?` fail-fast at line 350 stays; it ignores `on_error` (RESEARCH Open Question 4); it already validates in `execute`, so add `validate_aegis` there.

---

### `crates/paladin-battalion/src/conclave_execution_service.rs` (service, retry)

**Analog:** same file lines 296-396 plus `engine/retry.rs`.

**Replace** the whole `is_retryable_error` match (lines 343-380, which includes a message-sniffing `LlmError(msg)` arm, a per-variant list, and two `matches!(error.transience(), Transience::Transient)` arms) with the single-source form the existing `LlmFailure` / wildcard arms already use:
```rust
fn is_retryable_error(error: &PaladinError) -> bool {
    error.transience() == Transience::Transient
}
```
**Replace** `calculate_retry_delay` (lines 386-396, `2^attempt`, cap 16, +-20% jitter via `rand`) at the call site (line 321 `Self::calculate_retry_delay(retries - 1)` then line ~335 `sleep(delay).await`) with:
```rust
let delay = backoff_delay(&policy, retries + 1);
if !wait_backoff(delay, &None).await { return Err(e); }
```
where `policy` is `conclave.config.battalion_config.aegis.retry.clone().unwrap_or_default()` (ConclaveConfig already embeds `battalion_config`, conclave.rs:91). Remove the `timeout(...)` wrapper at lines 96-128 and the `ConclaveError::Timeout` return. Tests at 720-726 (`calculate_retry_delay`) and 731-801 (`is_retryable_error`) migrate: keep the transient/permanent/unknown cases at 774-801, delete the delay tests, add `Timeout`/`CircuitBreakerOpen`/`ArsenalError` rows per RESEARCH "Behaviour deltas". Line 630 `.with_timeout(60)` in tests goes.

---

### `crates/paladin-battalion/src/commander.rs`

**Analog:** itself. Delete execute() wrapper (472-485). Keep the bridges, re-sourced: `aegis.timeout.run_timeout` to Conclave/Maneuver timeout; Conclave retries stay `max_attempts.saturating_sub(1)` (line 638, already the Aegis meaning). Builder validation at 1774-1784 (`timeout_seconds == 0`, `max_attempts == 0`) becomes `validate_aegis()`. Builder default config (1766-1771, doc "Timeout: 300 seconds" at 1432): keep a 300 s per-attempt `run_timeout` (RESEARCH Finding 5). `CommanderBuilder::error_strategy(maneuver::ErrorStrategy)` stays; default `maneuver::ErrorStrategy::default()`.

---

### `src/infrastructure/resilience/circuit_breaker.rs` (infrastructure)

**Analog:** same file lines 215-235 (sync `call`) and ~281 (async `call_async`).
```rust
Err(e) => {
    // Only count retryable errors as failures for circuit breaker
    if e.is_retryable() {
        self.on_failure();
    }
    Err(e)
}
```
Change both sites to `if e.transience() == Transience::Transient { self.on_failure(); }` (import `paladin_core::platform::container::transience::Transience`). Update `tests/unit/circuit_breaker_test.rs` for the Unknown / Permanent cases (they stop tripping; §9.1 row).

---

### `crates/paladin-herald/src/{json,markdown,table}_herald.rs`

**Analogs (current code):**
- markdown_herald.rs:395-399: `self.format_field(&node_error.node_name, &node_error.error)`
- table_herald.rs:239-250: `writeln!(&mut output, "  {}: {}", node_error.node_name, node_error.error)` with `.map_err(|e| HeraldError::SerializationError(format!("Failed to write output: {}", e)))?`
- json_herald.rs:153: `"node_errors": result.node_errors,` serialises directly, so the shape changes automatically; test at 443-462 asserts `["node_name"]`/`["error"]` and must become `["node_id"]`, `["attempt"]`, `["transience"]`, `["source"]`.

Render per D-08 as `"{node_id} (attempt {attempt}, {transience:?}): {node_error}"` using the structured type's `Display` (node_error.rs:36-179). Test fixtures use `paladin_core::platform::container::battalion::NodeError { node_name, error }` (markdown_herald.rs:746); retype to `node_error::NodeError`. `src/application/cli/formatters/output.rs:716` only has `node_errors: Vec::new()` and needs no change (RESEARCH Finding 7).

---

### NEW guard test (e.g. `tests/legacy_removal_guard.rs`)

**Analog:** `tests/treasurer_vocabulary_guard.rs` (exact). Copy: plain `std::fs` walk, `skip_directory` (`target`, `node_modules`, dot-dirs except `.planning`/`.project`), forbidden tokens assembled at run time (`["Retry", "Policy"].concat()` style) so the file does not match itself, an allowlist const of files that document the removal, and two positive controls (planted-violation tree must report all; clean tree must report none) before the repository scan. Must not flag the Aegis `RetryPolicy`, `node_error::NodeError` or `maneuver::ErrorStrategy`; match on path-qualified `battalion::` forms or on field names (`timeout_seconds` on BattalionConfig, `with_error_strategy`).

---

### `.planning/decisions/0059-legacy-clean-break-removal.md` (NEW ADR)

**Analogs:** ADR-0051 (X-03 supersession shape; headings at 3/9/28/60/67/75/82: Status, Context, Decision, Considered Options, Code Locations, Code Conformance, Downstream Consumers) and ADR-0058 (design ADR with lettered Decision subsections `### (a) ...`, `**D-nn.**` bold lead-ins, `**Reversibility:**` lines, `**Date:**`, `**Phase:**` header).

Copy from 0051:
```markdown
# ADR-0051: Token-economy phases land as clean breaks inside the untagged v0.10.0
## Status
Accepted
**Date:** 2026-09-14
...
**X-03 is superseded for Phases 31, 32 and 33 only**, on the operator's 2026-09-14 decision. The
exception extends to no other phase, no other milestone, and no other public API anywhere ...
**What still applies.** Every break ... still gets a `MIGRATION.md` §9.2 row **and** a `cargo semver-checks` allowlist row ... **as documentation for the downstream refactor, never as a compatibility shim.**
```
ADR-0059 adaptations: title with v0.11.0; "X-03 is superseded for Phase 44 only"; state it does NOT inherit ADR-0051 and "a later removal records its own supersession"; add `## Supersedes` naming ADR-0001 and ADR-0002 (PROMOTION.md "Supersession mechanism"); Decision subsections for D-01/02, D-03, D-05..D-08, D-10..D-14. Headings required per PROMOTION.md; `## Code Conformance` must carry a `conforms` / `must change` verdict.

---

### `.planning/decisions/0001-battalion-config.md`, `0002-battalion-result.md` (edit)

**Analog:** PROMOTION.md lines 309-318. Change `## Status` body from `Accepted` to the bare word `Superseded`, followed by a prose line naming ADR-0059 and a date; leave the rest as history. Add the 0059 row plus two supersession notes in `.planning/PROJECT.md` Key Decisions.

---

### `MIGRATION.md` §9.1 / §9.2

**Analog 1, §9.1:** row `M-B-05` (line 30), format `| M-B-NN | **Bold headline (REQ, Phase 44 plan 44-NN).** prose ... | Who is affected | Required action |`. Rows needed: per-attempt timeout (D-02, includes ChainOfCommand/Grove now unbounded), `node_errors` JSON shape (D-06/D-08), Conclave predicate deltas (D-10/D-11), circuit-breaker `Transient`-only (D-14), `max_attempts` counts total attempts (`old + 1`) and `TransientOnly` default (RESEARCH Finding 4).

**Analog 2, §9.2:** header at line 173 and the `BattalionError` row at line 181:
```
| `paladin-ai-core` | `BattalionError` | new variant ... | `#[non_exhaustive]`, suppressed via `[package.metadata.cargo-semver-checks.lints] enum_marked_non_exhaustive = "allow"` in `crates/paladin-core/Cargo.toml`, mirrored in `.cargo/semver-checks-allowlist.toml` | Y — <justification> | FT-FR-02 |
```
and the measured-wording Y/N cells from Phase 43 (lines 270-289): "measured at the Phase 43 closeout (plan 43-13) with cargo-semver-checks 0.50.0 against the published 0.10.1, `--release-type minor`, <crate>'s crate-wide allows disabled and restored byte-for-byte: 196 checks, N lints fired (...)". Crate cell = crates.io name (`paladin-ai-core`; `paladin-battalion`; `paladin-ai` for root).

**Rows to cut** (one first-backticked identifier per row, per RESEARCH Finding 10, overriding CONTEXT D-19's single `battalion module` row): `BattalionConfig`, `RetryPolicy`, `ErrorStrategy`, `NodeError`, `BattalionResult`, `BattalionError`, `PaladinError` (`LlmError`, `is_retryable`, `is_terminal`; one row, several lints), `ConclaveConfig`, `ConclaveError`, plus `paladin-battalion | retry` for the deleted module. Requirement IDs `LEGACY-01..04`.

**Gate:** `scripts/check-migration-allowlist.sh` derives the `crate | type` pair from the first backticked identifier of the Type cell and the first whitespace token of the allowlist `migration_row`; run `make check-migration-allowlist` after edits.

---

### `.cargo/semver-checks-allowlist.toml`

**Analog:** the Phase 43 block (end of file). Copy section comment + entry shape:
```toml
# --- Phase 43 (Rate Pacing, PACE-01/02) -------------------------------------
# Entries below mirror MIGRATION.md's `paladin-ports | LlmError` row ... The D-27 diagnostic (cargo-semver-checks 0.50.0 vs
# 0.10.1, crate-wide allows disabled) fired these two lints for it ...

[[entry]]
crate = "paladin-ports"
lint = "enum_unit_variant_changed_kind"
migration_row = "paladin-ports | LlmError"
requirement_id = "PACE-01"
justification = "LlmError::RateLimitExceeded changed from a unit variant ... Every in-tree value and pattern site migrated in the same commit series."
```
New block "Phase 44 (Legacy Clean-Break Removal, LEGACY-01..04)": one `[[entry]]` per (published crate, lint, row). Lint ids come from the D-27 diagnostic, not from guessing (candidates: `struct_missing`, `enum_missing`, `enum_variant_missing`, `struct_pub_field_missing`, `inherent_method_missing`, `module_missing`/`pub_module_level_const_missing` for `retry`). Sibling comment text in `crates/paladin-core/Cargo.toml:60-72` shows how the empirical derivation is cited.

---

### `crates/paladin-core/Cargo.toml` (and `crates/paladin-battalion/Cargo.toml`, new table)

**Analog:** `crates/paladin-core/Cargo.toml:73-78`:
```toml
[package.metadata.cargo-semver-checks.lints]
enum_marked_non_exhaustive = "allow"
constructible_struct_adds_field = "allow"
inherent_method_missing = "allow"
struct_pub_field_missing = "allow"
struct_marked_non_exhaustive = "allow"
```
Add only the lint ids the diagnostic fires and that are not already present, with a comment block in the 60-72 style naming the phase, plan, and the diagnostic command. `paladin-battalion` has no such table today; add one if the diagnostic fires there. Temporary disabling for the diagnostic must be restored byte-for-byte.

---

### `CHANGELOG.md` `[Unreleased]`

**Analog:** existing `### Removed` (line 649, bullet list of items with short rationale) and `### Changed` (line 538) inside `[Unreleased]` (line 8). Add bullets naming each removed item (legacy `RetryPolicy`/`ErrorStrategy`/`NodeError`, `BattalionConfig` timeout/retry/strategy fields and builders, `BattalionError::Timeout`, `ConclaveError::Timeout`, `ConclaveConfig::with_timeout`, `PaladinError::LlmError`/`is_retryable`/`is_terminal`, `paladin_battalion::retry`), a `### Changed` line for `aegis` and `node_errors`, citing ADR-0059 and LEGACY-01..04. Also run `make api-surface-update` (baseline `.project/current-exports.txt`, line ~410 for `retry`).

---

### Examples and doc-examples

**Analog for the replacement form:** `Aegis` / `RetryPolicy` / `TimeoutPolicy` / `ErrorHandlerSpec::Absorb` in `crates/paladin-core/src/platform/container/aegis.rs` (struct at 43-55; `Aegis::validate` rejects `max_attempts == 0`). Show `aegis: Aegis { retry: Some(RetryPolicy { max_attempts: old + 1, retry_on: RetryPredicate::TransientAndUnknown, .. }), timeout: Some(TimeoutPolicy { run_timeout: Some(..), .. }), on_error: Some(ErrorHandlerSpec::Absorb { .. }) , ..Default::default() }` where the example used `RetryThenContinue`. Every `[[example]]` must still build (REL-05); `crates/doc-examples/src/orchestration.rs` is compiled by `make check-doc-examples`. Extra files outside CONTEXT: `examples/commander_auto.rs:83`, `examples/commander_basic.rs:98-105`, `src/application/cli/commands/battalion.rs:268,434,594,607,825`. `docs/src/getting-started/configuration.md:356-370`: replace the unread `battalion:` YAML block with a pointer to programmatic `BattalionConfig.aegis` (RESEARCH Finding 9).

## Shared Patterns

### Single classification source
**Source:** `crates/paladin-core/src/platform/container/paladin_error.rs:219` (`PaladinError::transience()`), `transience.rs` (`Transience`, not `#[non_exhaustive]`).
**Apply to:** Conclave predicate, circuit breaker, attempt runner. All call `error.transience()`; none re-derive a variant table or read `Display` text. Retry decision goes through `RetryPredicate::admits` via `engine::retry::should_retry`.

### One backoff implementation
**Source:** `crates/paladin-battalion/src/engine/retry.rs` (`backoff_delay`, `wait_backoff`).
**Apply to:** attempt runner, Conclave. The legacy `crate::retry` module and `calculate_retry_delay` are deleted.

### Paused-clock tests
**Source:** `engine/retry.rs` tests (`#[tokio::test(start_paused = true)]`). `test-util` is already a dev-dependency of the root crate and `paladin-battalion`. Apply to all new timeout/retry tests.

### Test mocks return Unknown
**Source:** `tests/helpers/mock_paladin_port.rs` (`fail_always`, `fail_paladin`, `fail_until_attempt` return `PaladinError::ExecutionError`, which is `Unknown`; `fail_paladin_until_attempt` returns a Transient `LlmFailure`). Tests that expect retry set `retry_on: RetryPredicate::TransientAndUnknown` or use the Transient mock.

### Same-commit exhaustive-match updates
`BattalionError::Timeout`, `ConclaveError::Timeout`, `PaladinError::LlmError` are all matched in the files listed in CONTEXT "Integration Points" and RESEARCH Finding 8. The variant and every in-tree arm go in the same commit (precedents: `StopReason`, `RateLimitExceeded`).

### Clean-break bookkeeping
Row in §9.2 marked `Y` plus `[[entry]]` per lint in the same commit; `make check-migration-allowlist`; lints measured by the D-27 diagnostic; no shims, aliases or re-exports (D-00c).

### Redaction before truncation
`crates/paladin-llm/src/redaction.rs` rule (security.instructions.md) applies if any new error path embeds provider text; `to_node_error_source` already carries redacted text (D-34), so reuse it instead of formatting responses yourself.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/paladin-battalion/src/aegis_attempt.rs` | helper | per-attempt timeout+retry loop | No existing in-crate function combines `tokio::time::timeout` + `should_retry` + `backoff_delay`; the engine's superstep loop (`engine/superstep.rs`, not read this session) is the closest behavioural reference, and RESEARCH Pattern 2 is the sketch |
| mdBook pages | docs | n/a | prose; follow D-20 and RESEARCH Finding 8/9 file lists |

## Metadata

**Analog search scope:** `crates/paladin-core/src/platform/container/`, `crates/paladin-battalion/src/`, `crates/paladin-herald/src/`, `src/infrastructure/resilience/`, `tests/`, `.planning/decisions/`, `MIGRATION.md`, `.cargo/`, `CHANGELOG.md`, `scripts/`
**Files scanned:** ~25 read or grepped this session (RESEARCH.md lines 456-818 only previewed; consult it for the Runtime State Inventory, behaviour-delta tables and Open Questions)
**Pattern extraction date:** 2026-10-09
