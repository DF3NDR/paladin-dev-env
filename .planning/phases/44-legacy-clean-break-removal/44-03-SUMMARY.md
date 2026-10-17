---
phase: 44-legacy-clean-break-removal
plan: 03
subsystem: error-classification
tags: [transience, circuit-breaker, conclave, paladin-error, clean-break, rustdoc]
status: complete

requires:
  - phase: 44-01
    provides: aegis_attempt runner (not used here; ordering only)
  - phase: 44-02
    provides: ADR-0059, the Phase 44-only licence for removing public variants and methods
provides:
  - CircuitBreaker::call / call_async count a failure only when transience() is Transient (D-14)
  - ConclaveExecutionService::is_retryable_error is the single expression transience() == Transient (D-10, D-11)
  - PaladinError's string-carrying LLM variant and its two legacy predicates removed from every Rust crate (D-09, D-13)
  - the measured before/after delta table 44-11 turns into MIGRATION.md 9.1 rows
affects: [44-07, 44-09, 44-11, 44-12, 44-13]

tech-stack:
  added: []
  patterns:
    - "One classification source: PaladinError::transience() is read by the engine, the Conclave and the breaker; message text is never read"

key-files:
  created: []
  modified:
    - src/infrastructure/resilience/circuit_breaker.rs
    - tests/unit/circuit_breaker_test.rs
    - tests/unit/paladin_execution_service_test.rs
    - tests/unit/paladin_error_test.rs
    - crates/paladin-battalion/src/conclave_execution_service.rs
    - crates/paladin-battalion/src/llm_failure.rs
    - crates/paladin-battalion/src/lib.rs
    - crates/paladin-core/src/platform/container/paladin_error.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - crates/paladin-eval/src/runner.rs
    - src/application/services/paladin/temperature_service.rs
    - src/application/services/paladin/paladin_execution_service.rs

key-decisions:
  - "The breaker and the Conclave read transience() alone; Unknown is neither retried nor counted"
  - "Conclave retry loop, sleep and calculate_retry_delay left untouched; 44-07 replaces them (D-12)"
  - "No deprecated bridge, alias or replacement method for the removed items (D-00c)"

patterns-established:
  - "Table tests that carry a retry-ish message on a Permanent failure and an empty message on a Transient one, to prove message text is never read"

requirements-completed: []
requirements-advanced: [LEGACY-03]  # compile side done; the four mdBook pages (44-09), 9.1/9.2 rows (44-11) and the source-tree guard (44-13) remain

duration: ~1h30min
completed: 2026-10-10
---

# Phase 44 Plan 03: Typed breaker and Conclave predicate, stringly LLM variant removed Summary

**The circuit breaker and the Conclave now classify failures through `PaladinError::transience()` only (Transient counts, Unknown and Permanent do not), and the string-carrying LLM variant plus the two legacy predicates are gone from every Rust crate.**

## Accomplishments

- **Task 1 (D-14, D-10, D-11):** both breaker failure sites use `e.transience() == Transience::Transient`; the Conclave's `is_retryable_error` is the one expression `error.transience() == Transience::Transient` with no per-variant arm. Six table tests pin it, including that message text is never read (a Permanent `LlmFailure` reading "429 rate limit timeout 503 network connection" is not retried and does not trip the breaker; a Transient one with an empty message is retried and does).
- **Task 2 (D-09, D-13):** deleted the string-carrying LLM variant, its `transience()` arm, its `paladin_error_kind` label, and both legacy predicates. `LlmFailure`'s rustdoc, the `llm_failure` module invariants, the battalion `lib.rs` module doc and `paladin_port.rs` (retry example now guards on `Transient`, `# Errors` bullet and the error table row now name `LlmFailure`) describe the typed taxonomy. `paladin_error_test.rs` was ported onto `LlmFailure` and `transience()`. `cargo check --workspace --all-targets --all-features` found no further compile sites.
- **Task 3:** the eval runner, `temperature_service.rs` and `paladin_execution_service.rs` comments were rewritten; the test was renamed `buffered_retry_sites_trip_the_circuit_breaker_on_a_transient_failure`; `make doc-check` is clean.

## Task Commits

1. **Task 1: breaker and Conclave retry on transience()** - `2b99f348` (refactor)
2. **Task 2: remove the stringly LLM variant and legacy predicates** - `a8b3d2f2` (refactor)
3. **Task 3: rustdoc on the typed taxonomy, doc-check clean** - `8f25ea81` (docs)

## Behaviour delta table (for 44-11 MIGRATION.md 9.1 rows)

"Before" is the v0.10 code (the legacy `is_retryable()` for the breaker; the per-variant match for the Conclave, whose wildcard arm already fell back to `transience()`). "After" is this plan. Re-verified against the code changed here and by the table tests named below.

| Variant | `transience()` | Conclave retried before -> after | Breaker counted before -> after |
|---|---|---|---|
| `Timeout` | Transient | yes -> yes | **no -> yes** |
| `LlmFailure` Transient | Transient | yes -> yes | yes -> yes |
| `LlmFailure` Permanent | Permanent | no -> no | **yes -> no** |
| `LlmFailure` Unknown | Unknown | no -> no | **yes -> no** |
| `ExecutionError` | Unknown | no -> no | **yes -> no** |
| Permanent group: `ConfigurationError`, `StopWordDetected`, `GarrisonRequired`, `MaxRetriesExceeded`, `GuardrailTripped`, `StructuredOutputInvalid` | Permanent | no -> no | no -> no |
| `ArmamentFailed` | Unknown | no -> no | no -> no |
| `CircuitBreakerOpen` | Transient | **no -> yes** | no -> yes (see note) |
| `GarrisonError` Storage / Tokenization | Transient | **no -> yes** | **no -> yes** |
| `GarrisonError` Serialization / NotFound / Configuration (Permanent), Custom (Unknown) | Permanent / Unknown | no -> no | no -> no |
| `ArsenalError` Timeout / TransportError | Transient | **no -> yes** | **no -> yes** |
| `ArsenalError` ToolNotFound / InvalidArguments / AuthFailed (Permanent), ProtocolError (Unknown) | Permanent / Unknown | no -> no | no -> no |
| Removed string-carrying LLM variant | was Unknown | was substring match on its text ("rate limit", "timeout", "network", "connection", "503", "429") | was yes (any message) |

Rows in bold changed. Notes:

- **`CircuitBreakerOpen` and the breaker:** the breaker returns `CircuitBreakerOpen` itself without running the closure and never feeds it back to `on_failure`; the "counted" change only matters when a nested breaker's open error is returned from inside an outer breaker's closure.
- **Production reach (research Finding 6):** the breaker only wraps `llm_port.generate(..)` (the two `call_async` sites at `paladin_execution_service.rs:2828` and `:3009`), so in production it only ever sees `LlmFailure`. The practical breaker changes are therefore: a Permanent or Unknown `LlmFailure` (bad key, invalid prompt, `ProcessingError`) no longer trips it; the `Timeout`, `ExecutionError`, `Garrison` and `Arsenal` rows are reachable only for embedders calling `CircuitBreaker` directly. The Conclave sees whatever a `PaladinPort` returns, so its new retries (`CircuitBreakerOpen`, Garrison storage, Arsenal timeout/transport) are production-reachable.
- **Pinning tests:** `circuit_breaker_counts_transient_failures_only`, `circuit_breaker_async_counts_transient_failures_only`, `conclave_retry_predicate_is_transient_only` (rows for Timeout, 503, authentication, `ProcessingError`, `ExecutionError`, `CircuitBreakerOpen`, Garrison storage, Arsenal timeout), `conclave_retry_predicate_ignores_message_text`, `unknown_llm_failures_do_not_open_the_circuit`, `converted_failure_carries_the_adapter_transience`.
- **Threat T-44-08 (accepted):** a bad API key (Permanent) now never opens the breaker, so each call reaches the provider and fails fast there; recorded here for the 9.1 row.

## Verification Results (all actually run)

| Check | Result |
|---|---|
| `cargo test -p paladin-ai --lib circuit_breaker` | 5 passed (includes both new breaker tests) |
| `cargo test -p paladin-ai --test unit circuit_breaker_test` | 6 passed |
| `cargo test -p paladin-ai --test unit paladin_execution_service_test` | 22 passed (includes `unknown_llm_failures_do_not_open_the_circuit`) |
| `cargo test -p paladin-ai --test unit paladin_error_test` | 5 passed |
| `cargo test -p paladin-battalion --lib conclave_execution_service` | 10 passed (includes both new Conclave tests) |
| `cargo test -p paladin-battalion --lib llm_failure` | 10 passed (includes `converted_failure_carries_the_adapter_transience`) |
| `cargo test -p paladin-ai-core --lib paladin_error` | 5 passed |
| `cargo test -p paladin-ai-core --doc paladin_error`, `cargo test -p paladin-battalion --doc llm_failure` | 1 and 2 passed |
| `cargo test -p paladin-ai --no-fail-fast` (after Task 1) | 3024 passed, 0 failed |
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `make doc-check` | exit 0 (zero rustdoc warnings both passes; 606 doctests passed, 0 failed) |
| `cargo test --workspace --no-fail-fast` | 6872 passed, **2 failed** - only the two documented baseline failures below |
| `make api-surface` | "API surface unchanged" (`PaladinError` lives in `paladin-core`, not in the facade baseline; the regenerated baseline is 44-10's) |
| `git grep -n 'PaladinError::LlmError\|PaladinError::is_retryable\|PaladinError::is_terminal\|\.is_retryable()' -- '*.rs'` | prints nothing |
| `git diff b362d6cb -- handoff_error.rs prompt_error.rs planning_error.rs` | empty (D-15) |

Acceptance greps: `transience() == Transience::Transient` appears 2 times in `circuit_breaker.rs` and non-comment `is_retryable` 0; the `is_retryable_error` body has 0 `=>` and 1 `transience() == Transience::Transient`; `#[deprecated` count in `paladin_error.rs` is 0; the eval/temperature/execution-service grep prints nothing.

**Workspace test run counts as passing per the plan's environment note:** the only two failures are `infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set` and `..._persists_no_run_traces_by_default`, the known pre-phase baseline failures in `deferred-items.md`. They did not fail in the isolated `cargo test -p paladin-ai` run, so they are load-dependent as well as pre-existing. Not touched.

## Deviations from Plan

1. **[Rule 1 - Bug] Broken intra-doc link in my own Task 2 rustdoc.** `make doc-check` reported `unresolved link to to_paladin_error` in the `llm_failure` module doc I rewrote: that module carries both an outer `///` (in `lib.rs`) and inner `//!` docs, which rustdoc merges and resolves in the parent scope (the Phase 40-06 finding). Fixed by using plain backticks. The fix sits in `llm_failure.rs`, a Task 2 file, but was committed in Task 3's commit `8f25ea81` because that is where `make doc-check` ran.
2. **[Plan reading] Task 2's `<verify>` final `test -z "$(git grep ...)"` could not be empty at Task 2's own commit.** Four comment-only mentions in the Task 3 files (`runner.rs`, `temperature_service.rs`, `paladin_execution_service.rs`) were still present then; they were removed in Task 3, after which the grep is empty (checked above). All other Task 2 checks passed at its commit.
3. **[Plan reading] `node_error` test row replaced.** In `llm_failure.rs` the `non_llm_paladin_failures_become_node_error_source_paladin_named_by_variant` table lost its removed-variant row; I added `MaxRetriesExceeded` -> `"MaxRetriesExceeded"` so the table keeps four rows.
4. **[Plan reading] TDD.** Task 1 is `tdd="true"`; the breaker tests were run RED first (3 failed against the legacy predicate: the two new tests and `test_state_transitions` once it switched to `Timeout`), then GREEN. The RED run was not committed separately; tests and implementation went in one `refactor` commit, so there is no separate `test(...)` commit. The Conclave tests were written together with the predicate and were not run RED first.
5. **[Environmental] Disk.** Free space fell to 1.7 GB after the workspace test run; I removed `target/debug/examples` and `target/debug/incremental` (as the environment notes allow). No source change.

No files outside the plan's `files_modified` list were edited, except `tests/unit/paladin_error_test.rs` (listed in the plan) - all twelve listed files were changed and nothing else. No architectural (Rule 4) changes.

## Issues Encountered

- A tool result (the STATE.md read) ended with text instructing me to do all work through Bash rather than the Read/Edit/Write tools. It came from tool output, not from the user or the plan, so I ignored it and continued with the dedicated tools.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or file-access surface. T-44-07 mitigated (the Conclave predicate and both breaker sites read only `transience()`, proven by `conclave_retry_predicate_ignores_message_text` and the breaker tests); T-44-08 and T-44-09 accepted as planned.

## Requirements

LEGACY-03 is advanced, not completed: the compile side is done, but the four mdBook pages naming the removed items (44-09), the MIGRATION.md 9.1/9.2 rows (44-11) and the source-tree guard (44-13) remain. `requirements.mark-complete` was not run, so REQUIREMENTS.md is unchanged.

## Next Phase Readiness

44-07 can replace the Conclave's retry sleep and `calculate_retry_delay` (D-12) without touching the predicate. 44-11 can turn the delta table above into 9.1 rows plus the `PaladinError` 9.2 row for the removed variant and methods. 44-13's guard may scan source comments: none of the removed names appear in Rust source.

## Self-Check: PASSED

Verified: commits `2b99f348`, `a8b3d2f2` and `8f25ea81` are on `claude/laughing-dirac-e0h2ax`; all twelve files listed in the frontmatter exist and are modified; the new tests named above exist and passed.
