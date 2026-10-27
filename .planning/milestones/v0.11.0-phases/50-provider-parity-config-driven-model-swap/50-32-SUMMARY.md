---
phase: 50-provider-parity-config-driven-model-swap
plan: 32
subsystem: spend-scope, pricing, execution-service, docs, ledger
tags: [rust, paladin, paladin-ports, paladin-llm, spend-scope, hopped-bills, fallback, pricing, billing, battalion, changelog, migration, windows-ledger, gap-closure, g-50-5a, wr-03, parity]
status: complete
gap_closure: true
gap_ids: [WR-03]

requires:
  - phase: 50
    provides: "50-29 ATTEMPT_SPEND and the engine's race_attempt spend read; 50-30 HoppedBills / HOPPED_BILLS / record_hopped_bill, hop bills settled in the call's one ledger row; 50-31 docs, M-B-21 / DR-18, matrix case"
provides:
  - "paladin-ports: HoppedBills::install_pricer (first pricer installed wins) and HoppedBills::count_and_take (counts only uncounted entries); record_hopped_bill prices through the installed pricer and counts into ATTEMPT_SPEND at once; per-entry counted flag; hand-written Debug using try_lock"
  - "paladin-llm: PricingLlmAdapter installs its hop-bill pricer on the active collector before generate and generate_stream delegate"
  - "root: PaladinExecutionService takes every collector with count_and_take (buffered helper and the five streamed exits) and no longer re-records hop bills; CallFailure.hop_bills keeps hop bills and the failing call's own bills distinct"
  - "paused-clock regression tests on the buffered path, the streamed path, the real Battalion engine (run deadline and idle bound), the pricing decorator and the new paladin-ports API"
  - "docs: spend_scope subsection 'Counted at the hop, however the attempt ends' (the 0cdbc302 limit text removed), pricing.rs and fallback.rs module docs, MIGRATION DR-18 and M-B-21, root / paladin-ports / paladin-llm CHANGELOGs"
  - "WINDOWS row 78 (WR-03) appended and fixed through the tool"
affects: []

tech-stack:
  added: []
  patterns:
    - "Price first, count at the hop: the outermost decorator hands the per-call collector a pricer before it delegates; the collector prices a bill against its own model and counts it into the attempt scope when it is recorded, with a private counted flag making the later take idempotent"
    - "Paused-clock drop test: ATTEMPT_SPEND.scope(scope, fut) inside tokio::time::timeout, with a second hop that never resolves, mirrors race_attempt"

key-files:
  created:
    - .planning/phases/50-provider-parity-config-driven-model-swap/50-32-SUMMARY.md
  modified:
    - crates/paladin-ports/src/output/spend_scope.rs
    - crates/paladin-llm/src/pricing.rs
    - crates/paladin-llm/src/fallback.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - .planning/WINDOWS.md
    - .planning/phases/50-provider-parity-config-driven-model-swap/deferred-items.md

key-decisions:
  - "A hop bill is priced and counted into the attempt scope when the chain hops past it, not when the whole model call returns. The pricer travels with the collector (install_pricer), so the attempt's cost is never made unknown by an early unpriced record while a pricing decorator is in the stack (A1-50-32, A2-50-32)."
  - "Both new methods are unreleased surface (spend_scope ships in v0.11.0): CHANGELOG '### Added', no section 9.2 row, no allowlist entry (A3-50-32, D-00b). make api-surface was unchanged and both semver-checks runs passed."
  - "Hop bills and the failing call's own bills stay distinct in CallFailure (hop_bills vs bills); settle part order stays hop bills first, then the call's own (A5-50-32)."

patterns-established: []

requirements-completed: [PARITY-05, PARITY-06]

duration: "not measured precisely; Task 1 committed 13:13Z, this continuation ran roughly 15:30Z to 15:55Z (the checkpoint wait is excluded)"
completed: 2026-10-19
---

# Phase 50 Plan 32: Count a hopped-past fallback bill toward the attempt at its hop (WR-03) Summary

**A bill a fallback chain records for a hop it moves past (a billed refusal under `hop_on_refusal(true)`) is now priced against its own model and counted into the attempt's `ATTEMPT_SPEND` scope at the hop, via `HoppedBills::install_pricer` / `count_and_take`, so a Battalion attempt the engine's run deadline or idle bound cuts off while a later hop is still in flight is charged for it once; the service still settles every hop bill in the call's one ledger row.**

## What was built

- **paladin-ports (`spend_scope.rs`).** `HoppedBills` now holds `entries: Arc<Mutex<Vec<HopEntry>>>` (private `HopEntry { bill, counted }`) and `pricer: Arc<OnceLock<HopPricer>>`.
  - `install_pricer(..) -> bool`: the first pricer installed wins (the outermost `PricingLlmAdapter`'s table prices every hop bill).
  - `count_and_take() -> Vec<BilledUsage>`: moves the entries out under the lock, releases it, counts only the uncounted ones, returns every bill in recording order.
  - `record_hopped_bill` prices an unpriced bill through the installed pricer and counts it at once; with no pricer it counts with `cost: None`, as the end-of-call fold would have.
  - `push`, `price_unpriced` and `take` keep their signatures and effects (`push` neither prices nor counts). The pricer and `record_incurred_spend` run with no collector lock held; poisoned locks are recovered; `Debug` uses `try_lock` and prints a pricer-installed flag, never the closure.
- **paladin-llm (`pricing.rs`).** A private `install_hop_pricer` runs before `inner.generate` and before `inner.generate_stream`, pricing against the entry's own model through the same `price_or_warn` (PARITY-06, D-15). The after-call passes stay as the idempotent safety net.
- **Root service (`paladin_execution_service.rs`).** `count_and_take` at the buffered helper and all five streamed exits; `CallFailure.hop_bills`; the buffered failure and success branches, `execute_structured_call` and `settle_streamed_call` no longer record hop bills into the scope; they still fold and settle them.
- **Docs.** Module docs in `spend_scope.rs`, `pricing.rs` and `fallback.rs`; MIGRATION DR-18 (what changed, plus the two consumer rules) and M-B-21; the three CHANGELOGs.

## Task commits

| Task | Name | Commit |
| ---- | ---- | ------ |
| 1 | Tracer: hopped-past bill priced and counted at its hop (WR-03) | `7edec618` |
| 2 | Module docs, MIGRATION and CHANGELOGs describe counting at the hop; 0cdbc302 limit text removed | `dea5ee5a` |
| 3 | Gates run; WR-03 ledger row 78 closed (plus a deferred-items note) | `13a87145` |

The tracer feedback gate was returned as a `checkpoint:human-verify` after `7edec618` and approved as is; Task 1's code was not reworked.

## Red output (captured by the first executor before any production edit)

- paladin-llm `hop_bills_are_priced_and_counted_at_the_hop` failed at `crates/paladin-llm/src/pricing.rs:1038`: `assertion left == right failed: hop 1's bill is already counted  left: 0  right: 1`.
- root `hop_bill_counts_when_the_attempt_is_dropped_mid_call`: `assertion left == right failed: hop 1's bill is already counted  left: 0  right: 1`.
- root `streamed_hop_bill_counts_when_the_attempt_is_dropped_mid_open`: same message, `left: 0  right: 1`.
- root `engine_charges_a_hop_bill_when_the_attempt_is_cut_mid_call` (run-deadline variant, the first to fail): `assertion left == right failed: run-deadline: the boundary settles hop 1's priced bill once  left: None  right: Some((Cost { nanos: 3600000, currency: CurrencyCode("USD") }, 1))`. The idle variant was not reached red (the loop stops at the first failure); it passes green after the fix.
- paladin-ports `a_bill_with_no_pricer_is_counted_unpriced`: `assertion left == right failed: counted at the hop  left: 0  right: 1`; `push_neither_prices_nor_counts` passed in the same run (guard).
- paladin-ports new-API tests were red by compile error (7 errors): E0599 no method `install_pricer` on `HoppedBills` (`spend_scope.rs` lines 582, 602, 625, 627), E0599 no method `count_and_take` (lines 612, 619), E0609 no field `entries` (line 533).
- Guard `hop_bills_are_counted_once_across_the_calls_retries` was green before and after.

## Green after the fix (Task 1)

paladin-ports 275 unit tests plus doc tests; paladin-llm `--all-features` `pricing::` 18 and `fallback::` 52; paladin-ai `application::services::paladin::paladin_execution_service::` 106; `provider_swap_matrix` (llm-all) 52; both engine variants of `engine_charges_a_hop_bill_when_the_attempt_is_cut_mid_call` (`Timeout(Run)` and `Timeout(Idle)`, each settling `(c1, 1)` once); the six plan 50-30 exactly-once tests unchanged.

## Gates (Task 3), exit statuses

| Gate | Result |
| ---- | ------ |
| `make api-surface` | exit 0, unchanged (no `make api-surface-update`, no `.project/current-exports.txt` change) |
| `make check-api-examples`, `make check-migration-allowlist`, `make check-gates` | exit 0 each |
| `cargo semver-checks check-release --package paladin-ports` / `paladin-llm` `--default-features --baseline-version 0.9.0` | exit 0 each ("no semver update required"); no allowlist entry added |
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p paladin-ports -p paladin-llm -p paladin-ai --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo test -p paladin-ports` | exit 0 (275 unit, 211 doc) |
| `cargo test -p paladin-llm --all-features --lib` | exit 0 (1205) |
| `cargo test -p paladin-ai --lib --all-features -- --skip <the two row-73 tests>` | exit 0 (1641 passed, 6 ignored, 2 filtered) |
| `cargo test -p paladin-ai --test provider_swap_matrix --features llm-all` | exit 0 (52) |
| `cargo test -p paladin-ports --doc`, `cargo doc -p paladin-ports -p paladin-llm --no-deps --all-features` (Task 2) | exit 0, no warnings |
| `make security` | exit 2, see Deferred Issues (pre-existing, not caused by this plan) |

Manual credential-handling review: `BilledUsage` holds a model id, token counts and a `Cost` only. The collector, the installed pricer and the scope hold nothing else, `spend_scope.rs` contains no log statement, and the pricer reuses the existing `price_or_warn` warn-once line (model id only). No API key, response body or content reaches the collector, the pricer, the scope or any log line. `git diff` of the plan's production changes shows no `unwrap(`, `expect(` or `panic!(` outside tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `make_paladin(2)` runs a second reasoning loop in the retries guard**
- **Found during:** Task 1, `hop_bills_are_counted_once_across_the_calls_retries`
- **Issue:** `make_paladin(2)` ran a second reasoning loop (usage 3*U1 + 2*U2 instead of 2*U1 + U2), so it did not exercise a retry of one call.
- **Fix:** The plan's stated fallback: `make_paladin(1)` plus a `ModelRetryMiddleware` `RetryPolicy` (`max_attempts: 2`, `initial_interval` 100 ms, `jitter: false`), with the same assertions on the one call. Test-only.
- **Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
- **Commit:** `7edec618`

**2. [Rule 3 - Blocking] `priced_refusal_stream_chain` generalised**
- **Found during:** Task 1, the streamed drop test
- **Issue:** The helper took `MockLlmAdapter` parameters, so the streamed test could not reuse `StallingLlmPort`.
- **Fix:** Generalised to `impl LlmPort + 'static`; existing callers unchanged. Test-only.
- **Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
- **Commit:** `7edec618`

### Ledger bookkeeping (not a deviation from intent)

The WR-03 row was appended as id 78 in Task 1 (open 12 to 13, total 77 to 78) and fixed through the tool in Task 3 (open back to 12, fixed 28 to 29, total 78).

## Auth gates

None.

## Deferred Issues

- `make security` fails in `make audit`: `cargo audit` reports 3 vulnerabilities in `zip 3.0.0` (RUSTSEC-2026-0367, -0368, -0369: extraction symlink and SUID handling) plus allowed unmaintained and unsound warnings. Plan 50-32 changes no `Cargo.toml` or `Cargo.lock`; the advisories are newly published against a transitive dependency. Out of scope, logged in `deferred-items.md`; needs a dependency bump or a reasoned `.cargo/audit.toml` allowlist entry in a plan that owns dependencies. `make deny` was not separately run.
- Owner question Q1 (A1-50-32) stays open: an out-of-tree decorator that only calls `price_unpriced` after its inner call reaches the ledger row but not the attempt scope; DR-18 now tells such a decorator to call `install_pricer` first.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file access or schema change; the collector, pricer and scope carry a model id, token counts and a cost only.

## Self-Check: PASSED

- Commits `7edec618`, `dea5ee5a`, `13a87145` are on `claude/laughing-dirac-e0h2ax`.
- `pub fn install_pricer` and `pub fn count_and_take` each appear once in `spend_scope.rs`; `grep -c 'when its model call ends'` is 0 in `spend_scope.rs` and `MIGRATION.md`.
- WINDOWS row 78 is `fixed`; frontmatter reads open 12, fixed 29, total 78.
