---
phase: 50-provider-parity-config-driven-model-swap
plan: 31
subsystem: provider-swap-matrix, docs, ledger
tags: [rust, paladin, paladin-llm, provider-swap-matrix, treasury-ledger, fallback, pricing, billing, changelog, migration, adr-0061, windows-ledger, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-26 BilledUsage on EmptyCompletion / Refusal; 50-27 every adapter bills, PricingLlmAdapter prices billed errors; 50-28 billed failures settled; 50-29 IncurredSpend and ATTEMPT_SPEND; 50-30 HOPPED_BILLS, hop bills settled in the call's one row"
provides:
  - "provider_swap_matrix case billed_failures_and_hopped_refusals_reach_the_ledger: real OpenAI, Anthropic and Kimi adapters over mock HTTP, with_pricing, FallbackLlmAdapter::hop_on_refusal(true), PaladinExecutionService with InMemoryTreasuryLedger (EveryCall)"
  - "Switching providers guide: a Billing bullet under ## Responses, and a Known limits bullet for remainder (c)"
  - "Root CHANGELOG: an Added entry (BilledUsage, the LlmError constructors and accessors), a Fixed entry (G-50-5a, plans 50-26 to 50-30), the Breaking Changes entry brought up to date, and known limitation (4) rewritten"
  - "paladin-llm CHANGELOG (Fixed: every adapter bills, pricing of billed errors and hop bills; Changed: DeepSeek detail) and paladin-ports CHANGELOG (Added: the additive items)"
  - "MIGRATION 9.1 M-B-21 (charging change) and M-B-22 (compat no-usage reclassification, DeepSeek detail); 9.8 DR-18 (reading a failed execution's spend) and two behaviour bullets"
  - "WINDOWS row 74 fixed; row 77 (open) for remainder (c); ADR-0061 stated limit (6) rewritten"
affects: []

tech-stack:
  added: []
  patterns:
    - "Matrix ledger case: serve each mock body under the model it is priced as (body model field plus usage), so the per-model breakdown is asserted on real adapter output"

key-files:
  created:
    - .planning/phases/50-provider-parity-config-driven-model-swap/50-31-SUMMARY.md
  modified:
    - tests/provider_swap_matrix.rs
    - docs/src/user-guides/switching-providers.md
    - CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - MIGRATION.md
    - .planning/WINDOWS.md
    - .planning/decisions/0061-per-model-parameter-support-policy.md

key-decisions:
  - "Remainder (c) row id is 77 (WINDOWS.md); row 74 closed by the tool on the owner's 2026-10-17 decision (A1-50-31)"
  - "The 50-26 behaviour change (a compat empty or refused body with no usage is EmptyCompletion / Refusal, not ProcessingError) and the 50-27 DeepSeek detail change get their own 9.1 row, M-B-22, so the charging row M-B-21 stays the single row naming 'Phase 50 plans 50-26 to 50-30'"
  - "paladin-core and paladin-battalion CHANGELOGs already carried accurate plan 50-29 entries (IncurredSpend; failed and timed-out attempts count their spend); left unchanged rather than duplicated"

patterns-established: []

requirements-completed: [PARITY-05, PARITY-07]

duration: 16min
completed: 2026-10-19
---

# Phase 50 Plan 31: G-50-5a closeout: end-to-end ledger proof, billing docs, row 74 fixed Summary

**A matrix case now runs real adapters, `PricingLlmAdapter`, `FallbackLlmAdapter` and `PaladinExecutionService`
with a treasury ledger together. It proves that a billed refusal the chain hops past, and a billed empty completion,
are each charged exactly once at their own model's price. The guide, both CHANGELOGs and MIGRATION describe the
shipped billing contract. `WINDOWS.md` row 74 is fixed, and Grok's `cost_in_usd_ticks` / `num_sources_used`
(remainder (c)) is the one open row, 77.**

The case is `billed_failures_and_hopped_refusals_reach_the_ledger`:

- **Sub-case 1, a hopped-past billed refusal.** The chain is `[openai -> anthropic]` with `hop_on_refusal(true)`.
  OpenAI refuses through its typed `refusal` field after billing 1_000 / 40 on `matrix-openai-model`. Anthropic then
  answers with 1_000 / 2_000 on `matrix-anthropic-model`. The run succeeds, and:
  - each vendor is called once;
  - the usage is 2_000 / 2_040 / 4_040, and the cost is c1 + c2;
  - the ledger holds ONE run row with one settlement and amount c1 + c2, broken down as
    `{matrix-openai-model: (c1, 1), matrix-anthropic-model: (c2, 1)}`.
- **Sub-case 2, a billed empty completion.** Kimi (the compat engine) answers `finish_reason: length` with empty
  content, billed 1_000 / 62_000 on `kimi-k2.6`. The run fails with a permanent `LlmFailure` ("Empty completion"), and
  the ledger holds one row, `{kimi-k2.6: (155_600_000, 1)}`, under the execution's own run.

## Performance

- **Duration:** about 16 min, 2026-10-19 00:15Z to 00:31Z, plus the closeout. Builds were warm from plan 50-30.
- **Tasks:** 3 of 3.
  - Task 1 was the tracer. Its gate passed in autonomous mode: after the commit, the tracer test was re-run on the
    committed tree and passed (1 passed). "Tracer verified end-to-end - expanding."
- **Files modified:** 8, plus this SUMMARY.

## Red evidence (Task 1)

The plan says the case passes on the finished tree, and it did on the first run. To show that it detects the
regression it guards, a mutation check was run instead of a pre-50-26 build. A worktree build at about 40 min and 5 GB
of free disk was not affordable.

- **Mutation.** `response_guards::bill` was turned into a no-op that returns the error unbilled, which is the behaviour
  before plan 50-26.
- **Sub-case 1 failed** with "the run's usage includes the refused hop's billed tokens": left `(1000, 2000, 3000)`,
  right `(2000, 2040, 4040)`.
- **Sub-case 2 failed** when run alone (sub-case 1 was gated off by a temporary env-var guard): "the failed call is
  settled once, at its own model's price", left `{}`, right `{"kimi-k2.6": (155600000, 1)}`. The ledger had no rows.
- **Restored.** Both files were restored from copies, and `cmp` against `HEAD` confirmed `response_guards.rs` byte for
  byte. Neither mutation was committed.
- **Before plan 50-26** (reasoned from the 50-26 to 50-30 SUMMARYs): no `billed` slot existed, so neither failure
  carried usage. The ledger would have held no row for the empty completion, and only the served hop's part for the
  refusal.

## Task Commits

1. **Task 1 (tracer): billed failures and hopped refusals reach the ledger end to end**: `6ce812e4` (test)
2. **Task 2: document the usage and billing contract**: `3ad87389` (docs)
3. **Task 3: close WINDOWS row 74 and ledger the deferred Grok cost cross-check**: `99c3c571` (docs)

## What the docs now say (written from the code, not the plan text)

- **Guide, `## Responses`, Billing bullet:**
  - `billed_usage()` returns the model, the usage and, behind pricing, the cost. It looks through `AllProvidersFailed`
    and is `None` when the vendor reported no usage. All nine adapters fill it, on both paths.
  - A billed failure is folded and settled under its call's ordinal, and it counts as its engine attempt's spend. That
    includes an error from the call that opens a stream (50-30 deviation 2).
  - Hop bills are priced against their own model and settled in the call's one row with a per-model breakdown, whether
    a later hop serves or the chain is exhausted. They settle on every exit of a stream, a dropped stream included
    (50-30 deviation 3).
  - The terminal chunk folds hop bills into its cost, and into its usage only when the provider reported one (50-30,
    D-17).
  - A failed or timed-out attempt counts its recorded spend, and a structured attempt counts every repair call. A
    success is never double counted.
  - An unpriced bill settles nothing.
  - `total_tokens` stays `prompt + completion` (D-02, kept).
- **Guide, refusal bullet:** the pattern is corrected to `Refusal { provider, model, detail, .. }`, with the variant
  noted `#[non_exhaustive]`.
- **Guide, Known limits:** one remainder (c) bullet. The guide text never said the vendor total was discarded or that
  a billed failure went unbilled, so there was nothing to remove.
- **Root CHANGELOG, known limitation (4):** now states the shipped contract (folds, the vendor-total check, billed
  failures, hop bills, failed attempts) and names remainder (c) as the one open item.
- **Root CHANGELOG, Breaking Changes:** the entry no longer says the pricing and settlement "follow in plans 50-27 to
  50-31".
- **MIGRATION M-B-22:** the compat body reclassification is scoped to the five compat presets' buffered path, which is
  the only path where `usage` became optional (`compat/types.rs`). The new outcomes are `Permanent`, where
  `ProcessingError` was `Unknown`; this was checked against `LlmError::transience`.
- **MIGRATION DR-18:** the `SpendScope::new` / `ATTEMPT_SPEND.scope` / `snapshot` recipe, the streamed-call variant,
  and `HOPPED_BILLS` for a direct fallback caller. Every method name was checked against `spend_scope.rs`.

## Deviations from Plan

### Auto-fixed Issues

None. No production code changed.

### Other deviations

- **Registration form.** The case is registered as a plain `#[tokio::test]` in the matrix list, beside the two
  fallback cases, not through `matrix_case!`. That macro takes a `case(provider)` form, and this case spans three
  providers. `matrix_covers_every_provider` is unchanged and green.
- **Red by mutation, not by a pre-50-26 build** (see Red evidence).
- **Extra MIGRATION row M-B-22.** The plan names one `M-B` row for the charging change. The orchestrator assigned this
  plan the 50-26 behaviour row as well, so it went into a second row. That keeps the plan's `grep -c` check at 1.
- **paladin-core and paladin-battalion CHANGELOGs not edited.** Both already held accurate plan 50-29 entries, and the
  plan's greps (`IncurredSpend`, `G-50-5a`) pass on them.
- **Row id in the docs.** The guide and the root CHANGELOG say remainder (c) "stays open on the `WINDOWS.md` ledger"
  without a number, because Task 2 was committed before the tool assigned one. ADR-0061 limit (6) names row 77.

## Verification

Run on the committed tree (`99c3c571`, clean working tree) unless stated. `CARGO_INCREMENTAL=0`. Results:

- **Task 1 `<verify>`** (before the commit, on the tree committed as `6ce812e4`):
  - the case: 1 passed;
  - the whole matrix: 52 passed (51 before plus the new case);
  - clippy `--workspace --all-targets --all-features -D warnings`: clean, so `6ce812e4` passes clippy on its own.
- **Task 2 `<verify>`** (before the commit):
  - all five greps succeed;
  - `cargo test -p paladin-llm --all-features --lib switching_page_tests`: 13 passed;
  - `make check-migration-allowlist`: exit 0, set-equal on both directions;
  - `make api-surface`: exit 0, unchanged (4298 items), so no `api-surface-update` was needed;
  - `make check-api-examples`: exit 0, 105 entry points;
  - `make check-gates`: exit 0.
- **Task 3 `<verify>`:** exit 0.
  - Row 74 is `fixed`. Row 77 is `| 77 | 50 | deviation | crates/paladin-llm/src/grok/adapter.rs | ... | open |`.
  - `open_count` is 12 before and after: minus row 74, plus row 77. `fixed_count` went from 27 to 28 and `total_count`
    from 76 to 77.
  - The tool's diff touched only the frontmatter counts, row 74's status and resolved date, and the new row (table and
    JSON).
- **Gates on the committed tree:**
  - `cargo fmt --all --check`: exit 0.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0.
  - `cargo test --test provider_swap_matrix --features llm-all`: 52 passed.
  - `make check-consumer-surface`: exit 0 (1 passed).
  - `make clean-code`: exit 0. That covers fmt, clippy, shellcheck, check, the doc check (ADR-0033), the workspace
    doctests and the `# Examples` gate.
- **`cargo test --workspace --lib --no-fail-fast`** (default features): 13 binaries, 5137 passed, 2 failed, so exit 101.
  The two failures are the ledgered baseline (WINDOWS row 73): `build_run_api_persists_no_run_traces_by_default` and
  `build_run_api_persists_run_traces_when_trace_persist_is_set`, both panicking at `run_api_wiring.rs:2369`. The
  tally is identical to plan 50-30's.
- **`make security`: exit 2, the known baseline.** `cargo audit` reports exactly 3 vulnerabilities, all in
  `zip 3.0.0`: RUSTSEC-2026-0367, RUSTSEC-2026-0368 and RUSTSEC-2026-0369. It also reports 9 allowed warnings, so make
  stops before `cargo deny`. This plan changes no dependency and no `Cargo.lock`.
- **Not run:**
  - The plan's `cargo test --workspace --features integration-tests,llm-all --no-fail-fast`. It is a full extra
    feature-set build on about 5 GB of free disk, and it is not in the orchestrator's gate list. The only code change
    is one test case, run above under `llm-all`.
  - `cargo semver-checks`, because no public item changed (`api-surface` unchanged).
  - Docker-bound suites and live vendor calls.
  - Per the orchestrator's instruction, these were not appended to WINDOWS (no new rows beyond row 77).
- **`unwrap(` / `expect(` / `panic!(` outside `#[cfg(test)]`:** none added. The only code is in the `tests/`
  integration target.
- **Manual credential-handling review:**
  - The new case sends only the literal `matrix-test-key`, to local mockito servers, and reads no environment
    variable.
  - The mock refusal text is a fixed string, and the assertions read numbers, model ids and the error's typed
    transience.
  - No HTTP client, header, redirect policy, log line or response-body handling changed.

## Known Stubs

None. Remainder (c) is a deliberate, owner-accepted deferral, ledgered as row 77, not a stub.

## Threat Flags

None. Against the register:
- T-50-31-01 (mitigated): row 74 was closed only on the owner's 2026-10-17 decision. Remainder (c) is kept open as row
  77 with its closing condition, and it is named in ADR-0061 limit (6), the root CHANGELOG known limitation and the
  guide's Known limits.
- T-50-31-02 (mitigated): MIGRATION 9.1 M-B-21 says who is affected and to re-size. M-B-22 covers the error
  reclassification.
- T-50-31-03 (mitigated): the matrix case drives real adapters, pricing, fallback, the execution service and the ledger
  together, and both halves are shown sensitive by the mutation check.

## Flagged assumptions

- A1-50-31 held: the owner's 2026-10-17 decision is the authority. Remainders (a) and (b) are covered by plans 50-30
  and 50-29, and (c) is deferred and open as row 77. No other acceptance is recorded.
- A2-50-31 held: the case uses the matrix's own `MockVendor`, `vendor_and_port`, `agent_builder` and
  `execution_service` helpers. It adds no Cargo target and runs under `llm-all`.

## Gap-closure set

With 50-31, all 13 Phase 50 gap-closure plans (50-19 to 50-31) are complete:
- G-50-5b: plan 50-19, row 75 fixed;
- G-50-5c: plan 50-20, row 76 fixed;
- G-50-5a: plans 50-21 to 50-31, row 74 fixed, row 77 open by the owner's choice.

Phase 50 is ready for re-verification of its UAT gaps.

## Self-Check: PASSED

- `tests/provider_swap_matrix.rs` contains `billed_failures_and_hopped_refusals_reach_the_ledger` and
  `InMemoryTreasuryLedger`.
- `.planning/WINDOWS.md` contains `cost_in_usd_ticks` (row 77, open), and row 74 is `fixed`.
- ADR-0061 contains `row 74` and `cost_in_usd_ticks`.
- Commits `6ce812e4`, `3ad87389` and `99c3c571` are on `claude/laughing-dirac-e0h2ax`.
