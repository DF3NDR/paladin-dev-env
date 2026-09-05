---
phase: 25-node-level-fault-tolerance
plan: 06
subsystem: infra
tags: [error-taxonomy, transience, llm-error, paladin-error, circuit-breaker]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-02)
    provides: "PaladinError::LlmFailure { transience, status, provider, message }, LlmError::ProviderError / AllProvidersFailed, LlmError::transience()"
provides:
  - "paladin_battalion::llm_failure::to_paladin_error(&LlmError) -> PaladinError: the ONE LlmError -> PaladinError::LlmFailure conversion, transience/status/provider read from typed fields, message = the source's own Display"
  - "All five first-party LlmError erasure sites (four in paladin_execution_service.rs, one in temperature_service.rs) now build the structured LlmFailure; src/ contains zero PaladinError::LlmError( constructions"
  - "Rendered text at every migrated site is byte-identical to the legacy erasure (LLM error: {e}); circuit-breaker accounting unchanged and circuit_breaker.rs untouched"
affects: [25-07-waypoint-attempt-history, 25-08-fallback-chain, 25-10-error-handlers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-crate error conversion lives in the lowest crate that can see BOTH types (paladin-battalion sees paladin-ports' LlmError and paladin-core's PaladinError); the facade reaches it through its existing dependency edge instead of duplicating it"
    - "RED commit compiles: the helper is stubbed as the exact legacy behaviour being retired, so preservation guards (rendered text, is_retryable) pass at RED while the structured-field tests fail for the right reason"
    - "Test doubles carry a fn() -> LlmError factory instead of a stored error, because LlmError is Clone but a factory keeps each call's error fresh and the double Send + Sync without a Mutex"

key-files:
  created:
    - crates/paladin-battalion/src/llm_failure.rs
  modified:
    - crates/paladin-battalion/src/lib.rs
    - crates/paladin-battalion/src/conclave_execution_service.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/paladin/temperature_service.rs

key-decisions:
  - "conclave_execution_service.rs holds no LlmError erasure: its three grep hits for PaladinError::LlmError( are the is_retryable_error match ARM for the retained legacy variant (line 347) and two legacy-variant constructions inside test_is_retryable_error (735/738). Nothing typed exists there to convert, so no production line in that file changed; instead a test proves the Conclave retry predicate classifies a to_paladin_error-converted LlmFailure by typed transience (503 and 500 -> retried, AuthenticationError/ProcessingError -> not)."
  - "The legacy-variant constructions in TEST code (paladin_error.rs's own tests from 25-02, conclave's test_is_retryable_error, tests/unit/paladin_error_test.rs) are kept on purpose: X-03 requires the retained public variant's behaviour to stay provably unchanged, and those tests are that proof. The plan's literal acceptance grep (`! grep -rn 'PaladinError::LlmError(' crates/ src/`) is unsatisfiable by construction because paladin_error.rs must keep match arms for the variant in is_retryable()/transience(); the intent -- zero first-party PRODUCTION constructions -- is met and reported with a refined grep below."
  - "The two buffered retry-loop sites never return the converted error to a caller (unchanged: they log it and end in MaxRetriesExceeded or CircuitBreakerOpen). Their contract is pinned two ways: rendered text by llm_failure's every-variant test, and circuit-breaker accounting by a test that trips a threshold-1 breaker on the first converted failure and observes CircuitBreakerOpen on the second attempt with exactly one provider call (T-25-25)."
  - "typed_origin's wildcard arm is compiler-required (LlmError is #[non_exhaustive] from paladin-battalion's point of view, D-04) and documented as the place a future status-carrying variant must add its own arm to have its fields cross."

requirements-completed: [FT-01]

coverage:
  - id: D1
    description: "One conversion function turns every LlmError variant into LlmFailure with transience from LlmError::transience(), status/provider from typed fields (None where absent, never a sentinel), and a message byte-identical to the legacy erasure"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_failure.rs#tests::conversion_preserves_the_rendered_message_exactly (every variant)"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_failure.rs#tests::conversion_carries_transience_from_the_source"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_failure.rs#tests::{provider_error_conversion_carries_status_and_provider,variants_without_a_status_convert_with_none,usage_limit_exceeded_carries_its_provider,all_providers_failed_converts_with_its_last_error_transience,converted_failure_is_retryable_like_the_legacy_variant}"
        status: pass
      - kind: other
        ref: "cargo test -p paladin-battalion --doc llm_failure (1 passed); ! grep -qE 'contains\\(|split\\(|parse::<u16>' crates/paladin-battalion/src/llm_failure.rs (0 hits)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every first-party site holding a real LlmError produces the structured LlmFailure; the stream-open and mid-stream sites surface it to callers with Transient/Some(503)/Some(\"openai\") for a provider 503 and Permanent/None/None for an authentication failure; the temperature site likewise"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests::paladin_execution_service_surfaces_structured_llm_failure"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests::permanent_provider_failure_surfaces_as_permanent"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/temperature_service.rs#tests::temperature_service_surfaces_structured_llm_failure"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/conclave_execution_service.rs#tests::conclave_execution_service_surfaces_structured_llm_failure"
        status: pass
    human_judgment: false
  - id: D3
    description: "No rendered error text changed and the circuit breaker's behaviour did not drift: converted failures render `LLM error: {e}` at every observable site, a threshold-1 breaker trips on the first converted failure exactly as it did on the legacy variant, and circuit_breaker.rs has a zero-line diff across the plan"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests::rendered_error_text_at_every_migrated_site_is_unchanged"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests::buffered_retry_sites_trip_the_circuit_breaker_like_the_legacy_variant"
        status: pass
      - kind: other
        ref: "git diff --name-only 7b8815e8 HEAD -- src/infrastructure/resilience/circuit_breaker.rs | wc -l == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "Workspace builds, lints and tests clean across all targets and features after the migration"
    requirement: FT-01
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0); cargo fmt --all -- --check (exit 0)"
        status: pass
      - kind: other
        ref: "cargo test --workspace --lib (exit 0, every crate ok); cargo test -p paladin-battalion --lib (616 passed); cargo test -p paladin-ai --lib (550 passed); cargo test --doc -p paladin-battalion (47 passed); cargo test --doc -p paladin-ai (113 passed)"
        status: pass
    human_judgment: false

duration: 26min
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 06: One LlmError -> LlmFailure Conversion, Five Erasure Sites Migrated Summary

**Landed `paladin_battalion::llm_failure::to_paladin_error` as the single `LlmError` -> `PaladinError::LlmFailure` conversion (transience, HTTP status and provider read from typed fields, message equal to the source's own `Display`) and routed every first-party erasure site in the facade through it, with rendered text and circuit-breaker accounting provably unchanged and `circuit_breaker.rs` untouched.**

## Performance

- **Duration:** ~26 min
- **Started:** 2026-09-05T21:31:00Z (worktree spawn)
- **Completed:** 2026-09-05T21:57:00Z
- **Tasks:** 2 (Task 1 as RED + GREEN commits)
- **Files modified:** 5 (1 created, 4 modified; 720 insertions, 5 deletions)

## Accomplishments

- **Task 1 — the helper** (`579e0e57` RED, `a1eb2e28` GREEN): `crates/paladin-battalion/src/llm_failure.rs` with `pub fn to_paladin_error(err: &LlmError) -> PaladinError`. It builds `PaladinError::LlmFailure { transience: err.transience(), status, provider, message: err.to_string() }`, where a private `typed_origin` reads `status`/`provider` from `ProviderError`'s own fields, `provider` from `UsageLimitExceeded`, recurses into `AllProvidersFailed.last`, and yields `(None, None)` for every other variant — never a sentinel, never anything parsed from a message (the negative grep for `contains(`/`split(`/`parse::<u16>` is 0). Module rustdoc records why it lives in `paladin-battalion` (X-01: `paladin-core` cannot see `LlmError`; the facade already depends on `paladin-battalion`) and the three invariants (X-03 byte-identical rendering, FT-FR-01 typed-only fields, T-25-25 unchanged retryability). Seven unit tests cover all twelve `LlmError` variants plus a doc test. Registered as `pub mod llm_failure` in `lib.rs`.
- **Task 2 — the migration** (`cde20651`): the four `paladin_execution_service.rs` sites (`execute_with_retry_and_temperature` and `execute_with_retry` retry loops, `execute_stream`'s open-stream `map_err`, and the mid-stream `tx.send(Err(..))`) and the one `temperature_service.rs` site now call `to_paladin_error(&e)`. Only the constructed variant changed — no control flow, propagation shape or public signature moved. `src/` reaches the helper via `use paladin_battalion::llm_failure::to_paladin_error;` (no copy under `src/`). Six new tests: structured-failure surfacing for the stream-open, mid-stream and temperature sites (503 -> `Transient`/`Some(503)`/`Some("openai")`; auth -> `Permanent`/`None`/`None`), the rendered-text guard, a circuit-breaker accounting guard for the two buffered sites, and a Conclave retry-predicate test over converted failures.

## Task Commits

1. **Task 1 (RED): failing tests for the conversion helper** — `579e0e57` (test)
2. **Task 1 (GREEN): `llm_failure::to_paladin_error`** — `a1eb2e28` (feat)
3. **Task 2: migrate the erasure sites, retire first-party `LlmError(String)` construction** — `cde20651` (feat)

**Plan metadata:** this SUMMARY's own `docs(25-06)` commit.

## Files Created/Modified

- `crates/paladin-battalion/src/llm_failure.rs` — new: `to_paladin_error`, private `typed_origin`, module rustdoc, 7 unit tests, 1 doc test
- `crates/paladin-battalion/src/lib.rs` — `pub mod llm_failure;` plus a crate-doc bullet
- `crates/paladin-battalion/src/conclave_execution_service.rs` — test only: `conclave_execution_service_surfaces_structured_llm_failure`
- `src/application/services/paladin/paladin_execution_service.rs` — 4 sites migrated; import; `FailingLlmPort` test double; 4 tests
- `src/application/services/paladin/temperature_service.rs` — 1 site migrated; import; `FailingLlmPort` test double; 1 test

## Site Audit (the "eight sites")

The plan counted eight erasure sites from a grep of `PaladinError::LlmError(`. Located by search rather than line number, they resolve as:

| # | File | What it actually is | Action |
|---|------|---------------------|--------|
| 1 | `paladin_execution_service.rs` ~1607 (`execute_with_retry_and_temperature`) | real `LlmError` erased inside the circuit-breaker closure | migrated |
| 2 | `paladin_execution_service.rs` ~1731 (`execute_with_retry`) | same | migrated |
| 3 | `paladin_execution_service.rs` ~1901 (`execute_stream` open) | `.map_err(\|e\| LlmError(e.to_string()))?` | migrated |
| 4 | `paladin_execution_service.rs` ~1926 (mid-stream) | `tx.send(Err(LlmError(e.to_string())))` | migrated |
| 5 | `temperature_service.rs` ~178 (`detect_task_type_with_llm`) | `.map_err(...)?` | migrated |
| 6 | `conclave_execution_service.rs:347` | match **arm** `PaladinError::LlmError(msg) =>` in `is_retryable_error` for the retained legacy variant | not a construction; left (plan: do not remove 25-02's arms) |
| 7 | `conclave_execution_service.rs:735` | legacy construction inside `test_is_retryable_error` | left; pins the retained arm (X-03) |
| 8 | `conclave_execution_service.rs:738` | same | left |

No additional site was found by the search. The Conclave service holds only `PaladinError`s from `PaladinPort`, so there is nothing typed to convert there — the information is genuinely unavailable at that layer, not thrown away by it.

**Refined acceptance grep.** Production constructions of the legacy variant across `crates/` and `src/`: **0**. Every remaining hit of `PaladinError::LlmError(` is a match-arm pattern (`paladin_error.rs:113,159`, `conclave_execution_service.rs:347`), a rustdoc mention (`paladin_port.rs:210`, `llm_failure.rs`, two test doc-comments), or a test that pins the retained variant's own behaviour (`paladin_error.rs` tests from 25-02, `conclave` `test_is_retryable_error`). `LlmError(String)` remains declared in `paladin_error.rs`.

## Decisions Made

- **RED commit compiles.** The stub for `to_paladin_error` at RED was the exact legacy erasure, so the two X-03 guards (rendered text, `is_retryable`) passed at RED and the five structured-field tests failed with `expected PaladinError::LlmFailure, got LlmError("...")` — a RED that documents the very behaviour being retired rather than a compile error that breaks bisect.
- **Buffered sites are pinned through the circuit breaker, not by return value.** `execute_with_retry*` end in `MaxRetriesExceeded`/`CircuitBreakerOpen` by design and the plan forbids changing that. A threshold-1 breaker + `MaxLoops::Fixed(2)` yields `CircuitBreakerOpen` after exactly one provider call for both functions, which is only possible if `is_retryable()` counted the converted failure — the same answer the legacy variant gave (T-25-25).
- **`err_expect` clippy lint.** Three `.err().expect(..)` chains in new tests were rewritten to `.expect_err(..)` after the workspace clippy pass flagged them; no production code was affected.
- **Legacy test constructions kept** (see key-decisions above).

## Deviations from Plan

### Plan-text corrections (no code deviation)

**1. "Three erasure sites in `conclave_execution_service.rs`" — there are none.**
- **Found during:** Task 1 read-first
- **Issue:** the three grep hits are a match arm and two test constructions of the retained legacy variant, not conversions of an `LlmError`.
- **Resolution:** no production line in that file changed; `conclave_execution_service_surfaces_structured_llm_failure` was added to satisfy the plan's Test 3 intent by proving the Conclave predicate honours converted failures. Recorded in the Site Audit above per the plan's own instruction to "record it in the SUMMARY naming the file and line, and state why the information is genuinely unavailable there".

**2. Literal acceptance grep `! grep -rn 'PaladinError::LlmError(' crates/ src/` cannot pass.**
- **Issue:** `paladin_error.rs` must keep `PaladinError::LlmError(_)` arms in `is_retryable()` and `transience()` (exhaustive matches over the enum's own variants), so the grep always hits.
- **Resolution:** the criterion's intent (zero first-party production constructions) is met and reported with the refined grep above.

### Auto-fixed Issues

None — no Rule 1-3 fix was needed outside the plan's own scope. (The `err_expect` lint was in code this plan added, fixed before commit.)

## Issues Encountered

- The sandbox refused two large heredoc insertions as "too complex"; the same edits were applied with the Edit tool. No content differed.

## Threat Flags

None. No new network endpoint, auth path, file access or schema change. The message carried into `LlmFailure` is the source `LlmError`'s own rendering; every provider-sourced path into that rendering is already redacted-then-bounded by `map_http_status` (plan 25-05, T-25-26). This plan adds no new unredacted path.

## Known Stubs

None.

## Broken-windows ledger

Nothing to record: no stubs, skipped tests or unrun `<verify>`. The two plan-text corrections above are documentation, not defects. `.planning/WINDOWS.md` was deliberately not touched from a parallel worktree to avoid a merge conflict with sibling agents.

## Verification (exit codes)

| Command | Exit |
|---|---|
| `cargo check --workspace --all-targets --all-features` | 0 |
| `cargo test -p paladin-battalion --lib` (616 passed) | 0 |
| `cargo test -p paladin-ai --lib` (550 passed) | 0 |
| `cargo test --doc -p paladin-battalion` (47 passed, 52 ignored) | 0 |
| `cargo test --doc -p paladin-ai` (113 passed, 17 ignored) | 0 |
| `cargo test --workspace --lib` | 0 |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| `git diff --name-only 7b8815e8 HEAD -- src/infrastructure/resilience/circuit_breaker.rs \| wc -l` | `0` |

No integration binary under `tests/` or `crates/*/tests/` was touched, so none was run individually.

## User Setup Required

None.

## Next Phase Readiness

- Plan 25-07 can build `NodeErrorSource::Llm` from `PaladinError::LlmFailure`'s typed fields via `PaladinError::transience()`; every first-party LLM failure now arrives structured.
- Plan 25-08's `FallbackLlmAdapter` produces `AllProvidersFailed`, which this helper already converts (transience, status and provider from `last`).
- The retained `PaladinError::LlmError(String)` is now constructed by no first-party production code; a future plan may deprecate it under X-10 without touching any call site.

## Self-Check: PASSED

- FOUND: `crates/paladin-battalion/src/llm_failure.rs`
- FOUND: `.planning/phases/25-node-level-fault-tolerance/25-06-SUMMARY.md`
- FOUND commit: `579e0e57` (Task 1 RED)
- FOUND commit: `a1eb2e28` (Task 1 GREEN)
- FOUND commit: `cde20651` (Task 2)

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-05*
