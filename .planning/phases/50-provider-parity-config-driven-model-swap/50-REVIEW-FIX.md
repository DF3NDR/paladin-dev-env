---
phase: 50-provider-parity-config-driven-model-swap
fixed_at: 2026-10-19T00:00:00Z
review_path: .planning/phases/50-provider-parity-config-driven-model-swap/50-REVIEW.md
iteration: 2
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 50: Code Review Fix Report (iteration 2)

**Fixed at:** 2026-10-19
**Source review:** `.planning/phases/50-provider-parity-config-driven-model-swap/50-REVIEW.md`
**Iteration:** 2

**Summary:**
- Findings in scope: 4 (WR-01..WR-04; IN-01..IN-06 are out of scope for `critical_warning`)
- Fixed: 4 (WR-03 by documenting the limit, not by a code change; see below)
- Skipped: 0

Each fix landed with a failing regression test first (WR-01, WR-02, WR-04). Work was done directly on
branch `claude/laughing-dirac-e0h2ax` in the main checkout, as the caller instructed, rather than in a
separate worktree.

## Fixed Issues

### WR-01: A streamed billed error is priced at the request model, not the bill's own model

**Files modified:** `crates/paladin-llm/src/pricing.rs`
**Commit:** 5a9f61d0
**Applied fix:** In `PricingLlmAdapter::generate_stream`, the `Err` item arm now prices a billed error
at `billed.model` through `price_billed_error(.., None, ..)`, as `generate` and the early-open error do.
The request model is used only when the bill names no model. Traced first: every streaming adapter
builds its bill with the hop's own (already rewritten) request model, so the bill's model is the right
one. `priced_billed_stream_error_carries_its_cost` was rewritten to pin the new behaviour (bill model
priced even when the request model has no row; a priced request model does not leak onto an unpriced
bill; an empty bill model falls back to the request model). Rustdoc on `generate_stream` and
`price_billed_error` and the paladin-llm CHANGELOG wording were updated.

### WR-02: A streamed call's own bill is settled under `model_used`, not the model that billed it

**Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
**Commit:** 62f1a764
**Applied fix:** `settle_streamed_call` now takes `own: Option<(&str, &Cost)>`. A billed terminal error
(early open error and mid-stream `Err` item) builds that pair with a new shared `priced_bill_part`
helper (bill model, `model_used` only when the bill names none); `priced_bill_parts` for the hop bills
now uses the same helper, so the buffered and streamed paths key identically. The terminal chunk keeps
`model_used` because a chunk carries no served-model string. Tests: `billed_stream_failure_is_settled`
now expects the ledger group to be the bill's model (`bill-model`), and
`hopped_bills_survive_stream_exhaustion` uses a Paladin model distinct from both hops so the last hop's
bill cannot be masked by coincidence (both were red before the change). This differs from the review's
suggested signature (`Option<&BilledUsage>`) because the terminal-chunk caller has a `Cost`, not a bill.

### WR-03: Hopped-past bills are lost when the attempt future is dropped mid-call

**Files modified:** `crates/paladin-ports/src/output/spend_scope.rs`, `MIGRATION.md`
**Commit:** 0cdbc302
**Status:** fixed: requires human verification (documented limit, no behaviour change)
**Applied fix:** The finding holds when traced: a hop bill reaches `ATTEMPT_SPEND` only after the whole
model call returns, so an attempt dropped while a later hop is in flight is not charged for the earlier
hop's billed refusal. The review's first option (record eagerly) was not applied: the bill can only be
priced once the call returns to `PricingLlmAdapter` outside the chain, and recording an unpriced bill
into the scope would make the attempt's whole cost unknown (`IncurredSpend` treats any `None` as unknown
for good), losing the settlement of the attempt's other, priced calls. Doing it right means giving the
collector a pricer and a counted-flag across the ports public surface, the pricing decorator and three
service settle sites, which is a redesign beyond a review fix. The review's second option was applied:
`spend_scope` module docs gain a "The limit: a hop bill is counted when its model call ends" section,
and MIGRATION DR-18 states the same, so "however the attempt ends" is no longer overstated. The paused-clock
test was not added, since it would only pin the documented limit. If the owner wants the eager behaviour,
it should be planned as its own gap-closure plan.

### WR-04: OpenAI and DeepSeek drop the bill on a no-choices response

**Files modified:** `crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/deepseek/adapter.rs`,
`crates/paladin-llm/CHANGELOG.md`, `MIGRATION.md`
**Commit:** a28b4f48
**Applied fix:** Both adapters now map usage before the choices check and return
`bill(LlmError::empty_completion("Response contained no choices"), billed_model, Some(usage))`, the
shape the compat engine and Gemini already use (`EmptyCompletion` is `Transience::Permanent`, so the
billed request is no longer re-sent). Their `usage` is a required field, so the bill is always present.
New tests `openai_no_choices_body_is_a_billed_empty_completion` and
`deepseek_no_choices_body_is_a_billed_empty_completion` (both red before). MIGRATION M-B-22 and the
paladin-llm CHANGELOG record the behaviour change. This commit also carries the one-line CHANGELOG
wording update for WR-01.

## Skipped Issues

None.

## Verification run

- `cargo fmt --check`: clean.
- `cargo clippy -p paladin-llm --all-features --all-targets -- -D warnings`: clean (also run for
  `paladin-ports` and `paladin-ai`, root crate, with `--all-targets`).
- `cargo test -p paladin-llm --all-features`: 1204 passed, 0 failed.
- `cargo test -p paladin-ai --lib`: 1333 passed, 0 failed.
- `cargo test -p paladin-ports`: 270 + 209 passed, 0 failed (94 ignored).
- `make api-surface`: unchanged (4298 items); no public-surface change, so no `api-surface-update`.

---

_Fixed: 2026-10-19_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
