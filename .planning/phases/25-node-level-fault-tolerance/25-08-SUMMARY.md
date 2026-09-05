---
phase: 25-node-level-fault-tolerance
plan: 08
subsystem: llm-fallback
tags: [fallback, llm-port, transience, trace-event, paladin-result, semver, migration]

# Dependency graph
requires:
  - phase: 25-02
    provides: LlmError::ProviderError, LlmError::AllProvidersFailed, LlmError::transience()
  - phase: 25-05
    provides: map_http_status redact-then-bound (the error summaries AllProvidersFailed.attempts carries)
  - phase: 25-07
    provides: TraceEvent attempt/cache_hit shape that FallbackHop sits beside
provides:
  - "FallbackLlmAdapter (crates/paladin-llm/src/fallback.rs): a plain LlmPort over an ordered chain of LlmPorts; hops on Transient|Unknown only, Permanent short-circuits, exhaustion -> AllProvidersFailed in chain order, no cross-call state"
  - "Streaming first-chunk rule: fall through only on a call Err or a first-item Err (peeked and replayed); after any Ok chunk errors propagate with the prefix intact"
  - "TraceEvent::FallbackHop { node_id: Option<NodeId>, from_provider, to_provider } plus a log::warn! per hop, via FallbackLlmAdapter::with_trace_sink"
  - "LlmResponse.metadata[\"paladin.served_by\"] stamped by the adapter (SERVED_BY_METADATA_KEY); PaladinExecutionService copies it into PaladinResult.served_by"
  - "PaladinResult.served_by: Option<String> (#[serde(default, skip_serializing_if)]), constructible and NOT non_exhaustive; MIGRATION.md 9.2 row Y, allowlist entry 5, paladin-core constructible_struct_adds_field = allow"
  - "MockLlmAdapter::with_provider_name / with_stream_items / with_model_query_error test facilities"
affects: [26-agent-runtime (RT-FR-09 middleware delegates to FallbackLlmAdapter), 28-observability (may enrich FallbackHop.node_id), 25-14 (guide + gate evidence)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Composing adapter over a Vec<Arc<dyn Port>> with no trait change (D-24)"
    - "Peek-then-replay on a boxed stream: Box::into_pin, next().await once, stream::iter(peeked).chain(rest)"
    - "Additive public field on a constructible struct registered deliberate-breaking in the same commit (D-26)"

key-files:
  created:
    - crates/paladin-llm/src/fallback.rs
  modified:
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/mock.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-core/src/platform/container/execution_result.rs
    - crates/paladin-core/Cargo.toml
    - crates/paladin-ports/src/output/paladin_port.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - tests/unit/paladin_execution_service_test.rs
    - tests/unit/handoff_service_test.rs
    - tests/cli/environment_tests.rs
    - .cargo/semver-checks-allowlist.toml
    - MIGRATION.md

key-decisions:
  - "Checkpoint (Task 1) resolved as auto-selected option-b-constructible per the orchestrator's auto-mode contract, matching locked decision D-26: served_by is an additive field on a constructible, NOT non_exhaustive PaladinResult, registered deliberate-breaking"
  - "Per-hop log uses log::warn! (the workspace's logging facade; no first-party crate depends on tracing directly) instead of the plan's literal tracing::warn! -- same level, same content, consistent with circuit_breaker.rs"
  - "The trace sink is awaited inline by the adapter and its Result discarded, rather than routed through a fire-and-forget queue: the adapter sits below the engine's TraceDispatcher and a synchronous emit keeps hop events deterministic for consumers"
  - "Empty chain rejected at construction with the typed FallbackChainError::EmptyChain; identity methods have no panicking path (chain_exhausted() fallback error instead of indexing)"

patterns-established:
  - "Full-literal migration: rustdoc examples take ..Default::default(); code sites take an explicit served_by: None"

requirements-completed: [FT-05]

coverage:
  - id: D1
    description: "FallbackLlmAdapter hops on Transient/Unknown only, Permanent short-circuits, exhaustion reports AllProvidersFailed in chain order, every call starts at element 0, concurrent calls share no state"
    requirement: FT-05
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib fallback (16 tests: three_provider_chain_falls_through_two_transient_failures, permanent_error_short_circuits_after_one_call, unknown_error_hops, exhaustion_returns_all_providers_failed_in_chain_order, every_call_starts_at_the_first_provider, concurrent_calls_share_no_hop_state, ...)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Streaming first-chunk rule: fall through on a call error or first-item Err; after an Ok chunk the error propagates with the prefix and no provider switch"
    requirement: FT-05
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib streaming_ (streaming_error_before_the_first_chunk_falls_through, streaming_call_error_falls_through, streaming_error_after_the_first_chunk_propagates_with_the_prefix, streaming_exhaustion_returns_all_providers_failed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Per-hop observability: TraceEvent::FallbackHop with node_id None and both provider names, plus a warn! log line"
    requirement: FT-05
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib each_hop_emits_a_trace_event_and_a_warning; cargo test -p paladin-ports --lib fallback_hop_variant_constructs_with_no_node_id"
        status: pass
    human_judgment: false
  - id: D4
    description: "PaladinResult.served_by is additive, legacy JSON byte-identical, Default/new still work, struct not non_exhaustive, fallback-served results name the serving provider"
    requirement: FT-05
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib execution_result (4 tests); cargo test --test unit served_by (fallback_served_result_records_the_serving_provider, a_non_fallback_result_leaves_served_by_none, paladin_result_is_not_marked_non_exhaustive)"
        status: pass
      - kind: automated_ui
        ref: "cargo check --workspace --all-targets --all-features; cargo test --doc -p paladin-ai-core; cargo test --doc -p paladin-ports"
        status: pass
    human_judgment: false
  - id: D5
    description: "Deliberate-breaking register: MIGRATION.md 9.2 PaladinResult row Y, fifth allowlist entry, paladin-core constructible_struct_adds_field suppression, all in the field's commit"
    requirement: FT-05
    verification:
      - kind: other
        ref: "test \"$(grep -c '^\\[\\[entry\\]\\]' .cargo/semver-checks-allowlist.toml)\" -eq 5; ! grep -q 'TBD — owner FT-05' MIGRATION.md"
        status: pass
    human_judgment: false

# Metrics
duration: 21min
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 08: Model Fallback (FallbackLlmAdapter + PaladinResult.served_by) Summary

**A chain-composing `FallbackLlmAdapter` that hops on Transient/Unknown errors only, never after a streamed chunk, reports exhaustion in chain order with a `FallbackHop` event per hop, and records the serving provider on a still-constructible `PaladinResult.served_by` registered deliberate-breaking under D-26.**

## Performance

- **Duration:** 21 min
- **Started:** 2026-09-05T22:54:34Z
- **Completed:** 2026-09-05T23:15:55Z
- **Tasks:** 3 (1 checkpoint auto-resolved + 2 TDD tasks)
- **Files modified:** 14 (1 created)

## Accomplishments

- `crates/paladin-llm/src/fallback.rs`: `FallbackLlmAdapter::new(Vec<Arc<dyn LlmPort>>)` implements `LlmPort` with no trait change. `generate` iterates from element 0 on every call, hops on `Transient | Unknown`, short-circuits on `Permanent`, stamps the serving provider into `LlmResponse.metadata["paladin.served_by"]`, and returns `AllProvidersFailed { attempts, last }` in chain order on exhaustion. `generate_stream` peeks exactly one item and replays it (`stream::iter(peeked).chain(rest)`), so fall-through happens only before the first chunk. Identity: `"fallback"`, first element's capabilities, first `Ok` for `validate_model`/`get_available_models` (last error otherwise). Empty chains are rejected at construction (`FallbackChainError::EmptyChain`). Ungated (ADR-0046): `cargo build -p paladin-llm --no-default-features` and `--features mock` both exit 0.
- `TraceEvent::FallbackHop { node_id: Option<NodeId>, from_provider, to_provider }` in `paladin-ports`; the adapter emits it (`node_id: None`) through `with_trace_sink` and logs a `warn!` naming both providers and the error's transience.
- `PaladinResult.served_by: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, `Default`/`new()` set `None`, a `was_served_by_fallback()` helper, and the struct's rustdoc now explains why it is deliberately constructible and not `#[non_exhaustive]`. `PaladinExecutionService::execute_internal` copies the metadata key into the result only when present.
- Every full struct literal in the tree was migrated by a script driven from `grep -rn 'PaladinResult *{'` (15 sites in 6 files; the other ~70 already used `..Default::default()`): rustdoc examples in `paladin_port.rs` and `execution_result.rs` now use functional update, code sites gained `served_by: None`. `cargo check --workspace --all-targets --all-features` proves it.
- Register closed in the same commit as the field: MIGRATION.md §9.2 `PaladinResult` row `TBD -> Y` with the D-26 justification (including that FT-FR-17 wrongly assumed functional update survives `#[non_exhaustive]`), fifth `[[entry]]` in `.cargo/semver-checks-allowlist.toml` (`constructible_struct_adds_field`, `FT-FR-17`), and `constructible_struct_adds_field = "allow"` in `crates/paladin-core/Cargo.toml`.
- `MockLlmAdapter` gained `with_provider_name`, `with_stream_items` (scriptable per-item stream, counted as one call) and `with_model_query_error`, exactly what the chain tests needed.

## Task Commits

1. **Task 1: Checkpoint — PaladinResult field strategy** — no commit; auto-selected `option-b-constructible` (D-26) by the orchestrator's auto-mode contract, recorded here as the user's answer.
2. **Task 2: FallbackLlmAdapter** — `005c2948` (test: RED, 14 failing behaviour tests + stub impl) → `90609199` (feat: GREEN)
3. **Task 3: served_by, full-literal migration, register row** — `a654589b` (test: RED, does not compile on the missing field by design) → `a574fc54` (feat: GREEN — field, migrations, service wiring, MIGRATION.md, allowlist, Cargo suppression in ONE commit)

**Plan metadata:** see the final `docs(25-08)` commit.

## Files Created/Modified

- `crates/paladin-llm/src/fallback.rs` — `FallbackLlmAdapter`, `FallbackChainError`, `SERVED_BY_METADATA_KEY`, `FALLBACK_PROVIDER_NAME`, 16 tests
- `crates/paladin-llm/src/lib.rs` — `pub mod fallback;` (ungated) + provider table row
- `crates/paladin-llm/src/mock.rs` — per-instance provider name, scripted stream items, model-query error
- `crates/paladin-ports/src/output/trace_sink_port.rs` — `TraceEvent::FallbackHop` + construction test
- `crates/paladin-core/src/platform/container/execution_result.rs` — `served_by` field, Default/new, rustdoc, `was_served_by_fallback`, 4 serde tests
- `crates/paladin-core/Cargo.toml` — `constructible_struct_adds_field = "allow"`
- `crates/paladin-ports/src/output/paladin_port.rs` — 2 rustdoc literals → functional update; 5 test literals → `served_by: None`
- `crates/paladin-battalion/src/engine/test_support.rs` — `RecordingPaladinPort` literal migrated
- `src/application/services/paladin/paladin_execution_service.rs` — reads `paladin.served_by` metadata into `served_by` on both result literals
- `tests/unit/paladin_execution_service_test.rs` — `served_by` module (Tests 4–6)
- `tests/unit/handoff_service_test.rs`, `tests/cli/environment_tests.rs` — literals migrated
- `.cargo/semver-checks-allowlist.toml` — fifth entry
- `MIGRATION.md` — §9.2 `PaladinResult` row resolved

## Decisions Made

- **Task 1 checkpoint → option (b) (D-26).** Constructible struct, additive field, deliberate-breaking register entry; full literals migrated here. Option (a) would have broken every cross-crate `..Default::default()` site.
- **`log::warn!` rather than `tracing::warn!`.** No first-party crate depends on `tracing` directly (it is only transitive in `Cargo.lock`); `log` is the facade every crate uses, including `circuit_breaker.rs`, the analog RESEARCH.md cites for the warn level. The subscriber in the root binary already bridges `log` records. Level and payload are as specified.
- **Sink awaited inline.** The adapter awaits `TraceSink::on_event` for each hop and discards its `Result` (debug-logged). This keeps hop events ordered and observable by the time `generate` returns; a slow sink would delay a hop, which is documented on `with_trace_sink`.
- **Circuit-breaker boundary is documentation (D-24).** The module rustdoc states that the facade `CircuitBreaker` yields `PaladinError::CircuitBreakerOpen` above the port, so a breaker wrapping an individual `LlmPort` must surface an `LlmError` the chain classifies `Transient`. No new variant.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Convention] `log::warn!` in place of `tracing::warn!`**
- **Found during:** Task 2
- **Issue:** The plan names `tracing::warn!`, but `paladin-llm` (and every first-party crate) uses the `log` facade and has no `tracing` dependency; adding one would be a new direct dependency the RESEARCH.md package audit did not propose.
- **Fix:** `log::warn!` with the same level and both provider names, matching `circuit_breaker.rs`.
- **Files modified:** `crates/paladin-llm/src/fallback.rs`
- **Verification:** `each_hop_emits_a_trace_event_and_a_warning` exercises the `record_hop` path (trace event asserted; the log line is emitted on the same path but not captured by a log harness).
- **Committed in:** `90609199`

**2. [Rule 3 - Blocking] Extra mock facility `with_model_query_error`**
- **Found during:** Task 2, Test 9
- **Issue:** `MockLlmAdapter::validate_model`/`get_available_models` could only answer `Ok`, so "first element that answers Ok" was untestable.
- **Fix:** Added `with_model_query_error(LlmError)`; also `with_provider_name` and `with_stream_items`, which the plan anticipated.
- **Files modified:** `crates/paladin-llm/src/mock.rs`
- **Committed in:** `005c2948`

**3. [Rule 1 - Bug] Migration script double-stamped one literal**
- **Found during:** Task 3
- **Issue:** The literal-migration script also matched the test literal in `execution_result.rs` that already carried `served_by: None`, producing a duplicate field.
- **Fix:** Removed the duplicate before the GREEN commit; `cargo check --workspace --all-targets --all-features` confirmed no other site was touched incorrectly.
- **Committed in:** `a574fc54`

### Observations (no action)

- The research estimated ~21 full literals; the grep-driven migration found 15 across 6 files (rustdoc ×3 incl. `execution_result.rs`, `paladin_port.rs` tests ×5, `test_support.rs` ×1, `paladin_execution_service.rs` ×2, `handoff_service_test.rs` ×2, `environment_tests.rs` ×2). All remaining `PaladinResult {` sites use `..Default::default()`.
- `paladin-llm` sets `[lib] doctest = false`; `cargo test -p paladin-llm --doc` therefore exits 0 without exercising the `FallbackLlmAdapter` rustdoc example. The example is kept and is feature-gated on `mock`.
- The RED commit for Task 3 (`a654589b`) intentionally does not compile on its own — the failing tests reference the not-yet-existing field, which is the only honest RED for a struct-field addition.

## Verification (orchestrator-required set, all exit 0)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets --all-features` | exit 0 (3m59s) |
| `cargo test -p paladin-llm --lib` | 84 passed |
| `cargo test -p paladin-ports --lib` | 120 passed |
| `cargo test -p paladin-ai-core --lib` | 484 passed |
| `cargo test -p paladin-battalion --lib` | 636 passed |
| `cargo test -p paladin-ai --lib` | 550 passed |
| `cargo test --doc -p paladin-llm` / `paladin-ports` / `paladin-ai-core` / `paladin-battalion` / `paladin-ai` | 7 / 118 / 71 / 48 / 113 passed |
| `cargo test --test unit` | 431 passed, 11 ignored |
| `cargo test --features cli --test cli` | 106 passed |
| `cargo build -p paladin-llm --no-default-features --features mock` and `--no-default-features` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `grep -c '^\[\[entry\]\]' .cargo/semver-checks-allowlist.toml` | 5 |
| `grep 'TBD — owner FT-05' MIGRATION.md` | no match |
| `grep -B3 'pub struct PaladinResult' execution_result.rs \| grep non_exhaustive` | no match |

## Threat Register Outcomes

| Threat | Disposition | Evidence |
|---|---|---|
| T-25-33 silent provider switch mid-response | mitigated | `streaming_error_after_the_first_chunk_propagates_with_the_prefix` asserts provider 2 call count 0 and no hop event |
| T-25-34 caller not knowing which provider answered | mitigated | `FallbackHop` per hop, `warn!` per hop, `served_by` on the result (`fallback_served_result_records_the_serving_provider`) |
| T-25-35 permanent failure burning every provider | mitigated | `permanent_error_short_circuits_after_one_call` asserts zero calls to providers 2 and 3 |
| T-25-36 shared hop state across concurrent calls | mitigated | adapter holds only `chain` + `trace_sink`; `every_call_starts_at_the_first_provider`, `concurrent_calls_share_no_hop_state` (16 tasks, multi-thread, 10 s guard, exact counts) |
| T-25-37 provider error summaries in `attempts` | mitigated | entries are `LlmError::to_string()` of the already-redacted source error; no body is added |
| T-25-38 `served_by` read as provenance | accepted | rustdoc on the field states it is observability, not attestation |
| T-25-SC package installs | accepted | no package added; `log`/`futures`/`thiserror` were already dependencies |

## Known Stubs

None.

## Threat Flags

None — no new network endpoint, auth path, file access or schema at a trust boundary; the adapter composes in-process ports.

## Self-Check: PASSED

- `crates/paladin-llm/src/fallback.rs` — FOUND
- commits `005c2948`, `90609199`, `a654589b`, `a574fc54` — FOUND in `git log`
