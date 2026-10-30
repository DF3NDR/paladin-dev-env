---
phase: 50-provider-parity-config-driven-model-swap
reviewed: 2026-10-19T00:00:00Z
depth: standard
iteration: 3
scope: changes since 3886b94c (gap-closure plan 50-32, review finding WR-03; commits 7edec618, dea5ee5a, 13a87145)
files_reviewed: 8
files_reviewed_list:
  - crates/paladin-ports/src/output/spend_scope.rs
  - crates/paladin-llm/src/pricing.rs
  - crates/paladin-llm/src/fallback.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - MIGRATION.md
  - CHANGELOG.md
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-llm/CHANGELOG.md
findings:
  critical: 0
  warning: 0
  info: 4
  total: 4
status: issues_found
---

# Phase 50: Code Review Report (iteration 3)

**Reviewed:** 2026-10-19
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found (4 info items; no blocker, no warning)

## Summary

Scope was the plan 50-32 diff (`git diff 3886b94c..HEAD`): hop bills are now priced and counted
into `ATTEMPT_SPEND` at the hop instead of when the whole model call returns. I found no
correctness, security or data-loss defect in the changed code. The six points the review was asked
to verify all hold:

1. **Counted-once.** `HoppedBills::record` prices the bill, calls `record_incurred_spend`, and
   stores the returned `bool` as the entry's private `counted` flag. `push` stores `counted:
   false`. `count_and_take` drains the list under the lock, releases the guard (the guard is a
   statement temporary), then counts only entries with `counted == false`. A second
   `count_and_take` sees an empty list, so it is idempotent. `record_hopped_bill` counts exactly
   once, inside the scope when one is visible and otherwise at take time. The unit tests
   `count_and_take_counts_only_what_was_not_counted` and `push_neither_prices_nor_counts` pin this.
2. **Pricer and `Debug`.** `install_pricer` uses `OnceLock::set(..).is_ok()`, so the first pricer
   wins and a later one is dropped. The hand-written `Debug` uses `try_lock`. It recovers a
   poisoned guard via `into_inner`, prints `<locked>` on `WouldBlock`, never blocks and never
   panics, and prints only `pricer_installed: bool`, never the closure.
3. **Poisoning.** Every access to the entries goes through `lock()`
   (`unwrap_or_else(PoisonError::into_inner)`); `Debug` handles `TryLockError::Poisoned`. No
   `unwrap`, `expect` or `panic!` is added to library code (the `expect` and `panic!` calls are
   inside `#[cfg(test)]`).
4. **`PricingLlmAdapter`.** `generate` and `generate_stream` both install the hop pricer before
   they delegate. In `generate_stream` it is installed on the captured handle before
   `inner.generate_stream`. The pricer prices against the entry's own model through
   `price_or_warn`. The after-call `price_hopped_bills` passes only touch entries still at
   `cost: None`, so they stay idempotent.
5. **`PaladinExecutionService`.** The buffered helper (`execute_with_retry_and_temperature`) and
   all five streamed exits use `count_and_take`: the early `generate_stream` error, the terminal
   chunk, the dropped receiver, the terminal error item, and a stream that ends without a final
   marker. Hop bills are folded into usage and cost and settled in the call's one ledger row via
   `priced_bill_parts`. They are never passed to `record_incurred_spend` by the service; only
   `failure.bills`, `response`, and `e.billed_usage()` (the call's own bills) are. The structured
   path likewise records only `failure.bills` and the response. No double-count path exists.
6. **Docs.** The CHANGELOG, `paladin-ports/CHANGELOG`, `paladin-llm/CHANGELOG` and MIGRATION DR-18
   text matches the code. The ports entry is additive (`### Added`, "not a break"), DR-18 states
   "additive, no compile break", and the 50-32 llm entry sits under `### Fixed` as a behaviour fix.
   I did not find an unwarranted break entry.

Credential handling (manual review per `security.instructions.md`): the collector, pricer, spend
scope and `Debug` impls hold only a model id, token counts and a `Cost`. No API key, response
body, refusal text or content reaches them. The `paladin::usage` warning in `carry_hopped_bill`
still logs provider, model and token counts only. The pricer closure captures an `Arc<PriceTable>`
and nothing else.

Limits of this review: the Bash tool failed with ENOSPC (full scratch filesystem) partway through,
so the diffs for the execution service and the four docs were assessed from full-file reads and
greps rather than `git diff`. `cargo` was not run, as instructed. I could not confirm that an
api-surface snapshot was refreshed for the two new public methods; no snapshot file referencing
`spend_scope` exists in the tree (nothing in the repo outside `.rs` and `.md` files and the lock
file mentions it), so `make api-surface` should be run before the commit.

## Info

### IN-01: Garbled and stale rustdoc on `CallFailure` and `execute_with_retry_and_temperature`

**File:** `src/application/services/paladin/paladin_execution_service.rs:410-416` and `3006-3009`
**Issue:** The `CallFailure` doc sentence at 410-412 is ungrammatical. It reads "`bills` lists the
bills of every hop a fallback chain moved past during the call (plan 50-30, [`HOPPED_BILLS`]) are
kept apart in `hop_bills`", which says the opposite of the next sentence ("`bills` is the failing
call's own bill"). The `# Errors` section of `execute_with_retry_and_temperature` (3006-3009) still
says `bills` carries "the `BilledUsage` of every billed hop moved past (plan 50-30)", but plan
50-32 moved the hop bills into `hop_bills`. A reader of the rustdoc would conclude the loop
records `bills` for hops, which is the double-count this plan removed.
**Fix:**
```rust
/// `bills` is the failing call's own bill (empty for an unbilled error); the loop records it into
/// the attempt scope itself. `hop_bills` holds the bills of every hop a fallback chain moved past
/// during the call (plan 50-30, [`HOPPED_BILLS`]); the collector already counted them into the
/// attempt scope at their hop (plan 50-32), so the loop only folds and settles them.
```
and for the `# Errors` paragraph: "whose `bills` carry the failing attempt's own `BilledUsage`
(G-50-5a, plan 50-28) and whose `hop_bills` carry every billed hop moved past (plans 50-30, 50-32)".

### IN-02: Over-long rustdoc lines in `fallback.rs`

**File:** `crates/paladin-llm/src/fallback.rs:230` and `234`
**Issue:** The reworded `carry_hopped_bill` doc left line 230 at about 101 characters and line 234
at about 128 characters (`once per (provider, model) under `HOP_BILL_LOG_TARGET` that it will not
be settled. The line names the provider, the model`). The project's `rust.instructions.md` asks for
lines under 100. `cargo fmt --check` will not flag it (comment wrapping is off by default), so it
will not fail CI, but it is ragged next to the surrounding text.
**Fix:** Re-wrap the paragraph at 100 columns.

### IN-03: A counted-unpriced entry can later be priced for the ledger but not the attempt scope (documented, out-of-tree only)

**File:** `crates/paladin-ports/src/output/spend_scope.rs:355-365` (`price_unpriced`) with `390-407`
(`count_and_take`) and `422-436` (`record`)
**Issue:** When no pricer is installed, `record` counts the bill into the attempt scope with
`cost: None`, which makes the attempt's whole cost unknown. If a decorator then calls
`price_unpriced`, the entry gets a cost, and the call's ledger row and the run's tallies see a
priced bill while `ATTEMPT_SPEND` keeps the unpriced count. The doc comments and DR-18 state this
and tell out-of-tree decorators to call `install_pricer` first. In-tree it cannot occur, because
`PricingLlmAdapter` installs its pricer before delegating and its after-call pass uses the same
table, so a model with no price row stays `None` on both sides. It is a footgun only for a
third-party pricing decorator.
**Fix:** Optional. Either leave it as documented, or have `price_unpriced` skip entries whose
`counted` flag is already `true`, so the entry's cost and the scope's count cannot diverge. If you
choose the latter, update the `price_unpriced` doc to match.

### IN-04: Lower-case sentence fragment in the `install_pricer` rustdoc

**File:** `crates/paladin-ports/src/output/spend_scope.rs:344-348`
**Issue:** The paragraph begins "first pricer installed wins (the outermost PricingLlmAdapter's table
prices every hop bill)." It starts in lower case and leaves the type name unlinked, so it renders
as a stray fragment inside the method docs.
**Fix:** "The first pricer installed wins: the outermost [`PricingLlmAdapter`]'s table prices every
hop bill." The type lives in `paladin-llm`, which `paladin-ports` does not depend on, so keep it as
plain code font (`PricingLlmAdapter`) rather than an intra-doc link.

---

_Reviewed: 2026-10-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
