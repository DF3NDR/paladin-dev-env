---
phase: 50-provider-parity-config-driven-model-swap
plan: 30
subsystem: execution-service
tags: [rust, paladin, paladin-llm, fallback, pricing, treasury-ledger, spend-scope, task-local, settlement, streaming, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-27 billed EmptyCompletion / Refusal priced by PricingLlmAdapter; 50-28 private CallFailure folded and settled; 50-29 IncurredSpend and the ATTEMPT_SPEND attempt spend scope"
provides:
  - "paladin_ports::output::spend_scope::{HoppedBills (new, push, price_unpriced, take), HOPPED_BILLS, record_hopped_bill}: a per-model-call collector of the bills a fallback chain moved past"
  - "FallbackLlmAdapter::after_failure records the bill of every billed error it hops past (an exhausted chain keeps the last hop's bill on AllProvidersFailed); with no collector installed it warns once per (provider, model) under paladin::usage, numbers only, bounded at 256 pairs"
  - "PricingLlmAdapter prices every unpriced collector entry against the entry's own model: after generate (Ok or Err), after generate_stream returns and again at the stream's terminal chunk or error item; an early generate_stream error's own bill is priced against its bill's model"
  - "execute_with_retry_and_temperature installs a fresh HOPPED_BILLS collector around the whole call (all retries) and returns a private CallOutcome { response, bills }; a CallFailure carries the hopped bills before its own"
  - "settle_agent_loop_call settles one call's priced parts in ONE row under (run_id, ordinal, 1): amount the Cost::checked_add sum, model_breakdown nanos per model; a currency mismatch logs error! and settles nothing"
  - "Buffered loop, structured call and every streamed exit (terminal chunk, terminal error, early generate_stream error, stream ended without a final marker, dropped receiver) fold hop bills into usage, cost and the attempt scope and settle them with the call; the streamed terminal chunk's usage and cost fold them in"
  - "Tests hopped_refusal_bill_is_settled_with_the_served_call (tracer), a_call_with_no_hop_bill_settles_one_part, hopped_bills_survive_chain_exhaustion, structured_call_records_its_hopped_bill, hopped_refusal_bill_is_settled_on_the_stream, hopped_bills_survive_stream_exhaustion, hopped_bills_are_priced_against_their_own_model, five fallback unit tests, five HoppedBills unit tests and two doc tests"
affects: [50-31]

tech-stack:
  added: []
  patterns:
    - "A per-call task-local collector carries side data (hopped-past bills) through a shared Arc<dyn LlmPort> decorator stack without changing LlmResponse"
    - "One settlement row per call ordinal with a per-model breakdown for multi-part calls (ADR-0053 section 4)"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/spend_scope.rs
    - crates/paladin-llm/src/fallback.rs
    - crates/paladin-llm/src/pricing.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md

key-decisions:
  - "The hop is recorded only where the chain really hops (after_failure's Some(next) arm); the exhausted chain's last bill stays on AllProvidersFailed, so CallFailure.bills = hopped bills then the error's own bill, never a double count"
  - "One settlement row per call (A1-50-30): the existing (run_id, ordinal, 1) key, amount the checked_add sum, a per-model breakdown summed per model"
  - "Streamed terminal chunk metadata folds hop bills into its cost by the CostTally rule and into its usage only when the provider reported a usage for the served call (D-17: a usage-less chunk stays usage-less; its hop bills still settle)"
  - "An early generate_stream error's own bill is priced against the bill's own model and settled with the hopped bills (it was neither priced nor settled before)"
  - "HOP_BILL_LOG_TARGET stays private (the warning target is the existing paladin::usage); no new public paladin-llm item"

patterns-established:
  - "Every exit of a streamed call goes through settle_streamed_call, which records hop bills to the attempt scope and settles the call's parts once"

requirements-completed: [PARITY-05, PARITY-06]

duration: 30min
completed: 2026-10-18
---

# Phase 50 Plan 30: Bills of hopped-past fallback hops are settled with the call (G-50-5a remainder (a)) Summary

**When a fallback chain moves past a hop whose error the vendor billed (a `Refusal` under `hop_on_refusal(true)`),
that hop's bill is now priced against its own model, counted in the run's usage and cost and in the attempt spend
scope, and settled once in the call's own ledger row with a per-model breakdown. This holds whether a later hop
answers or the chain ends in `AllProvidersFailed`, on the buffered and the streamed path. No public type changed
shape.**

- Tracer: `PricingLlmAdapter(FallbackLlmAdapter[hop1 -> m1, hop2 -> m2].hop_on_refusal(true))`. Hop 1 refuses after
  the vendor billed U1 = 1_000 / 40 on `m1`; hop 2 answers on `m2` with U2 = 1_000 / 2_000. Through a service with
  `InMemoryTreasuryLedger` in `EveryCall` mode, the run succeeds and:
  - the ledger holds one row for ordinal 1, amount c1 + c2, breakdown `{m1: c1, m2: c2}`;
  - `PaladinResult.usage` is U1 + U2 and `PaladinResult.cost` is c1 + c2;
  - the `ATTEMPT_SPEND` snapshot holds 2 calls with the same totals.

## Performance

- **Duration:** about 30 min (2026-10-18 23:32Z to 23:51Z for the two tasks and the changelog, then the gate run on
  the committed tree to 23:57Z).
- **Tasks:** 2 of 2, plus a changelog commit.
  - Task 1 was the tracer. Its gate passed in autonomous mode: after the commit, the tracer test and the
    `fallback::` suite were re-run on the clean committed tree and passed (the full Task 1 `<verify>` and clippy had
    passed on that identical tree just before the commit). "Tracer verified end-to-end - expanding."
- **Files modified:** 7 (4 source, 3 changelogs).

## Accomplishments

- **Red first, both tasks.**
  - Task 1: `hopped_refusal_bill_is_settled_with_the_served_call` failed with "the refused hop's billed usage is
    counted": left U2 (1_000 / 2_000), right U1 + U2 (2_000 / 2_040). The control
    `a_call_with_no_hop_bill_settles_one_part` passed, as it should.
  - Task 2: `hopped_refusal_bill_is_settled_on_the_stream` failed on the terminal usage (U2 only);
    `hopped_bills_survive_stream_exhaustion` failed with an empty ledger (`{}`); `structured_call_records_its_hopped_bill`
    failed with 1 recorded call instead of 2; `hopped_bills_are_priced_against_their_own_model` failed on its stream
    half (`cost: None`); the fallback warn-once test failed to compile (no `HopBillWarnings` / `carry_hopped_bill`).
  - `hopped_bills_survive_chain_exhaustion` (buffered) already passed after Task 1: Task 1's design records hop 1 at
    its hop and keeps hop 2's bill on `AllProvidersFailed`, so `CallFailure.bills` already held both and settled one
    row. It stays as a regression test; see Deviations.
- **`spend_scope` (paladin-ports):** `HoppedBills` over `Arc<Mutex<Vec<BilledUsage>>>` (`new`, `push`,
  `price_unpriced` (in place, in order, only `cost: None` entries, so idempotent), `take`; poison recovered with
  `PoisonError::into_inner`), the `HOPPED_BILLS` task-local and `record_hopped_bill(billed) -> bool`, each with
  rustdoc and a doc example; the module rustdoc gains a "Hopped-past bills" section.
- **`FallbackLlmAdapter`:** `after_failure`'s hop arm (the one place a hop is decided) records `err.billed_usage()`
  through `carry_hopped_bill`; with no collector, a bounded `HopBillWarnings` set (256 pairs, the
  `UNPRICED_MODEL_WARNING_CAPACITY` precedent) warns once per (provider, model) at target `paladin::usage` with the
  provider, model and token counts only. Module and `hop_on_refusal` rustdoc updated.
- **`PricingLlmAdapter`:** `price_hopped_bills` / `price_active_hopped_bills` price each unpriced entry against its
  own model through `price_or_warn`. `generate` prices after the inner call, `Ok` or `Err`; `generate_stream`
  captures the collector before the inner call, prices after it returns and again on the terminal chunk and on an
  `Err` item; an early `generate_stream` error is now priced through `price_billed_error(.., None, ..)`.
- **Execution service:**
  - `execute_with_retry_and_temperature` wraps the old body (now `execute_with_retry_inner`) in a fresh
    `HOPPED_BILLS` scope and returns `CallOutcome { response, bills }`; on failure the hopped bills go before
    `CallFailure`'s own.
  - `settle_agent_loop_call(ledger, scope, run_id, ordinal, parts)` settles all of a call's priced parts in one row;
    `priced_bill_parts` maps bills to parts (an empty bill model falls back to the Paladin's model).
  - Buffered loop: each hop bill is folded into `usage`, `cost_tally` and the attempt scope before the served call,
    and the call settles once (`hop bills + served response`); the failure arm settles all `CallFailure.bills` in one
    row instead of one settle per bill (which would have hit `AlreadySettled` for the second).
  - `execute_structured_call` records `CallOutcome.bills` into the attempt scope.
  - Stream: `stream_settlement` is resolved before the stream opens; one collector wraps `generate_stream` and is
    re-installed in the forwarding task; `settle_streamed_call` runs at every exit (terminal chunk, terminal error,
    early `generate_stream` error, a stream ending without a final marker, a dropped receiver). The terminal chunk's
    `ChunkMetadata` and `ExecutionMetadata` usage and cost fold the hop bills in.
- **Changelogs:** root, `paladin-ports` and `paladin-llm` entries.

## Task Commits

1. **Task 1 (tracer): bills of hopped-past fallback hops are settled with the call**: `5a5096f1` (feat)
2. **Task 2: hopped bills survive exhaustion and streams**: `9e12d3b8` (feat)
3. **Changelog entries for hopped-past bills**: `980fcbf7` (docs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The failure arm settled each bill separately under one key**
- **Found during:** Task 1, while generalising the settle.
- **Issue:** plan 50-28's buffered failure arm settled every bill in `CallFailure.bills` with its own
  `settle_model_call` under the same `(run_id, loop_num, 1)` key. With hop bills in that list, the second settle would
  be dropped as `AlreadySettled` and its charge lost.
- **Fix:** the arm folds every bill, then settles all priced bills once through the multi-part settle.
- **Commit:** `5a5096f1`

**2. [Rule 2 - Missing critical functionality] An early `generate_stream` error's bill was neither priced nor settled**
- **Found during:** Task 2.
- **Issue:** a fallback stream whose hops all refuse fails in `generate_stream` itself (`AllProvidersFailed`, carrying
  the last hop's bill). `PricingLlmAdapter::generate_stream` passed that error through unpriced (`?`), and the service
  converted it with no settle and no attempt-scope record, so the last hop's bill was lost even though the plan
  requires the early-error exit to settle.
- **Fix:** the pricing decorator prices an early error's bill against the bill's own model (the hop that answered
  last, not the request's model); the service records that bill to the attempt scope and settles it with the hopped
  bills. A single, non-fallback adapter that returns a billed error from the call itself now settles too.
- **Commit:** `9e12d3b8`

**3. [Rule 2 - Missing critical functionality] A dropped stream receiver left hop bills unsettled**
- **Found during:** Task 2.
- **Issue:** the forwarding task returns early when the caller drops the stream; any hop bills taken from the
  collector only at the terminal chunk would never settle.
- **Fix:** that exit also goes through `settle_streamed_call` (a no-op once the terminal chunk took them).
- **Commit:** `9e12d3b8`

### Other deviations

- **`hopped_bills_survive_chain_exhaustion` was green before Task 2.** Task 1's design already carried both bills on
  the buffered path (see Accomplishments), so this one Task 2 test was not observed red. Its stream counterpart,
  `hopped_bills_survive_stream_exhaustion`, was red.
- **Streamed terminal metadata folds hop bills.** The plan names the stream's settle; for parity with the buffered
  `PaladinResult` the terminal `ChunkMetadata` / `ExecutionMetadata` usage and cost now include the hop bills too
  (cost by the `CostTally` rule; usage only when the provider reported the served call's usage, D-17).
- **Extra tests:** `a_call_with_no_hop_bill_settles_one_part` (the Task 1 control), `structured_call_records_its_hopped_bill`,
  `hopped_bills_survive_stream_exhaustion`, and the fallback tests `a_billed_hop_is_recorded_for_settlement`,
  `an_unbilled_hop_records_nothing`, `exhaustion_records_every_hop_but_the_last`, `a_billed_hop_on_the_stream_is_recorded`,
  `a_billed_hop_with_no_collector_warns_once_per_provider_and_model`.
- **Two streamed exits are not test-covered:** the dropped-receiver exit and the stream that ends without a final
  marker go through the same `settle_streamed_call` as the tested exits, but no test drives a hop bill through them.
- **Warn-once is tested on its set, not on captured log output:** the crate has no log-capture harness, so the test
  drives `carry_hopped_bill` and `HopBillWarnings::should_warn` directly and checks the chain is unchanged.
- **Semver run shape:** the orchestrator's D-27 shape (vs the published 0.10.1, `--release-type minor`) was run, not
  the plan's CI shape (vs 0.9.0). See Verification.
- **Commit trailer:** the closeout and 50-30 commits carry `Co-Authored-By: Claude Opus 5.5`, the model that made them;
  the orchestrator's message named `Claude Fable 5.1`, which earlier commits on this branch carry.

## Verification

`CARGO_INCREMENTAL=0`. Exit 0 unless stated:

- **Task 1 `<verify>`** (before the commit, on the tree committed as `5a5096f1`): `cargo test -p paladin-ports`
  (270 unit + 209 doc, 94 ignored), `cargo test -p paladin-llm --all-features --lib fallback::` (47),
  `pricing::` (16), `cargo test --lib hopped_refusal_bill_is_settled_with_the_served_call` (1),
  `cargo test --lib paladin_execution_service` (98); `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` clean, so `5a5096f1` passes clippy on its own.
- **Task 2 `<verify>`** (before the commit, on the tree committed as `9e12d3b8`): `cargo test -p paladin-llm
  --all-features --lib` (1202), `cargo test -p paladin-ports` (270 + 209 doc), `cargo test --lib
  paladin_execution_service` (102, including all six root hop tests), `cargo test --test provider_swap_matrix
  --features llm-all` (51); clippy clean, so `9e12d3b8` passes clippy on its own. `cargo fmt --all --check` clean;
  `make check-gates` OK (changelog guard included).
- **Gates on the committed tree (`980fcbf7`, clean working tree):**
  - `make clean-code`: exit 0 (fmt unchanged, clippy, shellcheck, `cargo check`, both rustdoc passes, 644 workspace
    doctests, the `# Examples` gate with 105 entry points).
  - `make check-consumer-surface`: passed (1 test).
  - `make api-surface`: unchanged (4298 items). The new items live in `paladin-ports` and are not re-exported by the
    facade, so no `api-surface-update` was needed; changelog entries were added anyway (`980fcbf7`).
  - `cargo semver-checks` 0.50.0 vs the published 0.10.1, `--release-type minor` (crate-wide allows active):
    `paladin-ports` 190 checks, all pass; `paladin-llm` 194 checks, all pass ("no semver update required"). CI's own
    shape for `paladin-ports` (`--default-features --baseline-version 0.9.0`) also passes. No lint, so nothing to
    register under D-27. `paladin-ai-core` and `paladin-battalion` are untouched by this plan (their pre-existing
    0.10.1-relative lints are recorded in the 50-29 SUMMARY).
- **`cargo test --workspace --lib --no-fail-fast`** (default features): 13 binaries, 5137 passed, 2 failed, so exit
  101. The two failures are the ledgered baseline (WINDOWS row 73): `build_run_api_persists_no_run_traces_by_default`
  and `build_run_api_persists_run_traces_when_trace_persist_is_set`.
- **`make security`: exit 2, the known baseline.** `cargo audit` reports exactly RUSTSEC-2026-0367/0368/0369
  (`zip 3.0.0`) plus 9 allowed warnings, so make stops before `cargo deny`. This plan changes no dependency and no
  `Cargo.lock`.
- **`unwrap(` / `expect(` / `panic!(` added outside `#[cfg(test)]`:** none. Every addition sits in `spend_scope.rs`'s
  test module, the `fallback` / `pricing` test modules, or the root crate's `agent_loop_cost_tests` /
  `streamed_cost_tests`.
- **Manual credential-handling review (settle, collector and warn paths):**
  - `HoppedBills` holds `BilledUsage` values only: a model id, token counts and a `Cost`; never refusal text, response
    content, a key id or a credential (T-50-30-04).
  - The one new `warn!` (fallback, no collector) names the provider, the model and two token counts. The
    `settle_agent_loop_call` `debug!` gained a part count; its `error!` lines name the run id, ordinal, nanos and
    currencies only (the 39-05 rule).
  - Each hop bill is priced against its own model (T-50-30-03), tested at two different rates.
  - One row per call ordinal with a per-model breakdown; a fresh collector per call (T-50-30-02).
  - No HTTP client, header, redirect policy or response-body handling changed.
- **Not run:** Docker-bound integration tests and live vendor calls. The change is internal to the decorators and the
  execution service and is exercised with in-memory ledgers and mock hops.

## Known Stubs

None.

## Threat Flags

None. Against the register:
- T-50-30-01 (mitigated): recorded at the single hop decision point, priced outside the chain, settled on both paths
  and on exhaustion (tracer, `hopped_bills_survive_chain_exhaustion`, both stream tests).
- T-50-30-02 (mitigated): one row per ordinal (`by_run.settlements == 1` in every hop test); a fresh collector per call.
- T-50-30-03 (mitigated): `hopped_bills_are_priced_against_their_own_model`.
- T-50-30-04 (mitigated): numbers, a model id and a `Cost` only.

## Flagged assumptions

- A1-50-30 held: one settlement row per call under `(run_id, ordinal, 1)`; the breakdown names each model.
- A2-50-30 held: every part comes from the one `PriceTable`; a mismatch would log `error!` and settle nothing (not
  reachable in the tests).
- A3-50-30 held: the collector spans all of one call's retries (it wraps `execute_with_retry_inner`).
- A4-50-30 held: `hop_on_refusal` is unchanged and still off by default; only what is counted changed.

## Self-Check: PASSED

- `crates/paladin-ports/src/output/spend_scope.rs` contains `pub static HOPPED_BILLS` (1).
- `crates/paladin-llm/src/fallback.rs` contains `record_hopped_bill`.
- `src/application/services/paladin/paladin_execution_service.rs` contains `hopped_refusal_bill_is_settled_with_the_served_call`.
- Commits `5a5096f1`, `9e12d3b8` and `980fcbf7` are on `claude/laughing-dirac-e0h2ax`.
