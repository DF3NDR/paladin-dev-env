---
phase: 50-provider-parity-config-driven-model-swap
reviewed: 2026-10-19T00:00:00Z
depth: standard
iteration: 2
scope: changes since 42feb9bd (plans 50-19..50-31 plus review-fix commits)
files_reviewed: 40
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - CHANGELOG.md
  - MIGRATION.md
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/src/platform/container/cost.rs
  - crates/paladin-eval/src/scenario.rs
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/examples/live_vendor_smoke.rs
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/anthropic/vision.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/compat/types.rs
  - crates/paladin-llm/src/conformance.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/grok/adapter.rs
  - crates/paladin-llm/src/kimi/adapter.rs
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/src/openai/vision.rs
  - crates/paladin-llm/src/pricing.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-llm/src/response_guards.rs
  - crates/paladin-llm/src/usage_reconcile.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/spend_scope.rs
  - docs/src/user-guides/switching-providers.md
  - src/application/services/paladin/paladin_execution_service.rs
  - tests/consumer_surface_muster.rs
  - tests/provider_swap_matrix.rs
findings:
  critical: 0
  warning: 4
  info: 6
  total: 10
status: issues_found
---

# Phase 50: Code Review Report (iteration 2)

**Reviewed:** 2026-10-19
**Depth:** standard
**Files Reviewed:** 40
**Status:** issues_found

This is iteration 2 of the Phase 50 code review. It is scoped to the changes since the first review commit `42feb9bd`: gap-closure plans 50-19..50-31 (G-50-5a usage and billing fidelity, G-50-5b OpenAI `max_completion_tokens`, G-50-5c Kimi fallback profile) plus the review-fix commits. It supersedes the earlier report at this path. Cargo was not run, because a workspace test run holds the build lock; everything below comes from reading the code and diffs.

## Summary

The change set adds typed billing to `LlmError`: `BilledUsage`, plus `EmptyCompletion` and `Refusal` as `#[non_exhaustive]` struct variants. On top of that it adds:

- Usage reconciliation (`usage_reconcile.rs`), which compares each adapter's mapped usage with the vendor's own total and records a finding.
- Reasoning and cache folds for Grok, Gemini and Anthropic vision.
- Two task-local carriers, `ATTEMPT_SPEND` (per Battalion attempt) and `HOPPED_BILLS` (per model call).
- Bill carrying in the execution service, the fallback chain and the pricing decorator.
- Kimi's fallback profile switched to `NotSent`.
- OpenAI's output cap sent as `max_completion_tokens`.

I traced the main paths and found no data-loss or security defect.

- **Carrier design:** `ATTEMPT_SPEND` is installed outside the attempt future, and the service re-installs it inside the one `tokio::spawn` it owns for streams. `HOPPED_BILLS` is installed around the whole retry loop, and the chain records at its one hop decision point.
- **Double counting:** the exhausted-chain case (earlier hops go to the collector, the last hop rides on `AllProvidersFailed`) is correct and tested. Success versus failure is one source per outcome in the engine.
- **Credentials and redaction:** no new logging, error text or client path touches a credential. The `--usage` harness client still never follows redirects and prints numbers only.
- **Layering:** dependencies still flow inward (`spend_scope` in ports uses core types, and adapters and the service use ports).

The remaining defects are pricing and attribution gaps on the streamed and fallback paths, one resource-accounting hole when an attempt is dropped mid-call, and a billing-parity gap between adapters that the documentation says is closed. There are no BLOCKERs.

## Warnings

### WR-01: A streamed billed error is priced at the request model, not the bill's own model, so a fallback-served stream is mispriced

**File:** `crates/paladin-llm/src/pricing.rs:286`
**Issue:** In `PricingLlmAdapter::generate_stream`, the `Err` item arm calls `price_billed_error(&table, Some(&model), err)`, where `model` is the outer request's model. Every other billed-error site passes `None` and prices at `billed.model`: `generate`, and the early `generate_stream` open error a few lines above. Behind `Pricing(Fallback(..))`, the hop that served the stream rewrote the request model (`request_for_hop`), and the adapter named its own model in the bill. When hop 2 (`m2`) answers a stream that ends in a billed `EmptyCompletion` or `Refusal`, the bill names `m2` but is priced at hop 1's (`m1`) row. If the models are priced differently, the Treasurer charges the wrong rate. If `m1` has no row, the bill is silently unpriced.

The test `priced_billed_stream_error_carries_its_cost` pins this behaviour on purpose ("Phase 31 stream contract"). That contract exists because an `Ok` terminal chunk carries no served-model string. A billed error does carry one, in `billed.model`, so the limitation does not apply here. The early-open path (`AllProvidersFailed` before any chunk) already prices at the bill's own model, so the two stream paths disagree.
**Fix:**
```rust
// pricing.rs, generate_stream, Err arm
Err(err) => {
    if let Some(bills) = &hopped {
        Self::price_hopped_bills(&table, bills);
    }
    // The bill names the hop that billed it; price that model, as `generate` does.
    Err(Self::price_billed_error(&table, None, err))
}
```
Then update the stream test to expect pricing at the bill's model. If the owner wants request-model pricing kept for non-fallback streams, fall back to `Some(&model)` only when `billed.model` is empty.

### WR-02: A streamed call's own bill is settled under `model_used`, not the model that billed it

**File:** `src/application/services/paladin/paladin_execution_service.rs:475-492` (`settle_streamed_call`), called from `:3998-4007` and `:4195-4204`
**Issue:** `settle_streamed_call` settles `own` (a billed terminal error's cost) under `(model_used, cost)`, where `model_used` is the Paladin's configured model. The cost itself was priced against the bill's model (WR-01 aside, and always on the early `AllProvidersFailed` open error, where `None` was passed). With a fallback chain whose last hop used a different model, the ledger row's `model_breakdown` credits the amount to the configured model, while the per-hop bills are keyed correctly by `bill.model`. The buffered path already does this right through `priced_bill_parts(&failure.bills, ..)`, which uses `bill.model`. The two paths therefore produce different `model_breakdown` keys for the same event.
**Fix:** Pass the whole `BilledUsage` instead of only its cost, and use its model, falling back to `model_used` only when it is empty:
```rust
async fn settle_streamed_call(
    settlement: Option<&StreamSettlement>,
    model_used: &str,
    hop_bills: &[BilledUsage],
    own: Option<&BilledUsage>,   // was Option<&Cost>
) { /* own.and_then(|b| b.cost.as_ref().map(|c| (non_empty(&b.model, model_used), c))) */ }
```
This is needed on the early-open error and on the mid-stream `Err` item.

### WR-03: Hopped-past bills are lost when the attempt future is dropped mid-call, which G-50-5a (b) is meant to cover

**File:** `src/application/services/paladin/paladin_execution_service.rs:3003-3027` (`execute_with_retry_and_temperature`), `crates/paladin-ports/src/output/spend_scope.rs:279` (`record_hopped_bill`)
**Issue:** The module docs promise that an attempt's spend is counted "however the attempt ends", including a deadline or idle drop. A bill the fallback chain records goes only into the call's `HoppedBills`. It reaches `ATTEMPT_SPEND` (`record_incurred_spend`) only after the whole scoped call completes and `hopped.take()` runs. If the Battalion deadline or idle bound drops the attempt future after hop 1 returned a billed refusal but while hop 2 is in flight, the collector is dropped with the future. That bill never reaches the attempt scope, and the timed-out attempt is charged nothing for a refusal the vendor billed. This is the same shape the plan fixed for ordinary loop calls, since `recorded_spend` reads the scope handle the engine holds. For a hopped bill the handle never sees it.
**Fix:** Make the record eager. For example, have `record_hopped_bill` also push the (unpriced) usage into `ATTEMPT_SPEND` and mark the collector entry as already counted, so the service's later fold settles it without re-recording. If that is too invasive, state the limit in the `spend_scope` and MIGRATION DR-18 text ("a hop bill is counted when its model call ends") so the "however it ends" claim is not overstated. Add a test with a stalling second hop under a paused clock.

### WR-04: OpenAI and DeepSeek still drop the bill on a no-choices response, while the docs say every adapter carries it

**File:** `crates/paladin-llm/src/openai/adapter.rs:1087-1091`, `crates/paladin-llm/src/deepseek/adapter.rs:916-918`
**Issue:** The compat engine (and Gemini) now map usage first and turn a no-choices body into `EmptyCompletion` carrying the bill. OpenAI returns `LlmError::ProcessingError("No choices in response")` and DeepSeek returns `ProcessingError("DeepSeek response contained no choices")`, both discarding a `usage` object they parsed. Two consequences:
1. A billed response is unbilled, contradicting the guide ("All nine adapters fill it, buffered and streamed") and the CHANGELOG parity claims.
2. `ProcessingError` is `Transience::Unknown`, so the retry loop may re-send the same billed request while the first bill is dropped. The compat `EmptyCompletion` is `Permanent` and not retried.

Behaviour is also inconsistent across the nine adapters for the same wire event.
**Fix:** Mirror the compat/Gemini handling. Map usage before the check, and on empty choices return `bill(LlmError::empty_completion("Response contained no choices"), billed_model, Some(usage))`. Add the matching conformance case or per-adapter test, and update MIGRATION M-B-22, which lists only the five compat presets.

## Info

### IN-01: The folds in the billed-failure branch of the agent loop update locals that are dropped on return

**File:** `src/application/services/paladin/paladin_execution_service.rs:1981-1990, 2002-2006`
**Issue:** The failure branch runs `usage += bill.usage.clone(); cost_tally.record_call(..)` and then `return Err(failure.error)`. Both are locals of the loop and are discarded. The error cannot carry them, and the comment claims they are "folded into the run's tallies". Only `record_incurred_spend`, the ledger settle and the `debug!` line (which prints the now-pointless `run_total_tokens`) have an effect.
**Fix:** Drop the two dead statements and the `run_total_tokens` field, and keep a comment that the bill is reported to the attempt scope and the ledger only.

### IN-02: `UsageLog` is unbounded, unlike the other warn-once sets

**File:** `crates/paladin-llm/src/usage_reconcile.rs:150`
**Issue:** `UsageLog.warned` grows by one `(model, check)` key per distinct model string with no cap. The sibling sets bound this (`UNPRICED_MODEL_WARNING_CAPACITY`, T-38-07, and the new `HOP_BILL_WARNING_CAPACITY`). The key is `request.model`, so it is caller-chosen, but `Ollama` and `OpenAI-compatible` take operator or tenant-supplied model ids, and the cost is at most three keys per id.
**Fix:** Add a capacity guard (return without inserting past N, still returning the metadata value).

### IN-03: Conformance case count is documented wrongly

**File:** `docs/src/user-guides/switching-providers.md:275`
**Issue:** The guide says "one 19-case conformance suite", but `crates/paladin-llm/src/conformance.rs:2096` pins `CASE_COUNT == 20` and its own docs say twenty. `docs/src/contributing/contributing-providers.md:349` still says 17.
**Fix:** Say 20 in both pages (or avoid the number and point at `CASE_COUNT`).

### IN-04: Currency-mismatch log also fires on `checked_add` overflow

**File:** `src/application/services/paladin/paladin_execution_service.rs:535-543`
**Issue:** `Cost::checked_add` returns `None` for either a currency mismatch or an `i64` overflow, but the `error!` text always says "parts disagree on currency". Overflow is practically unreachable, so this is only a misleading diagnostic.
**Fix:** Word it as "cannot be summed (currency mismatch or overflow)" or check `part.currency() != cost.currency()` before adding.

### IN-05: Unused parameter in the `--usage` harness

**File:** `crates/paladin-llm/examples/live_vendor_smoke.rs:2068`
**Issue:** `usage_candidates(_spec: &ProviderSpec, ..)` ignores its first argument, and every caller and test passes a spec for nothing.
**Fix:** Remove the parameter.

### IN-06: Vendor-supplied model string is logged unsanitised in the hop-bill warning

**File:** `crates/paladin-llm/src/fallback.rs:236-247`
**Issue:** The `paladin::usage` warning interpolates `billed.model`, which for a buffered hop is the vendor-reported `response.model`. A hostile or buggy vendor could embed newlines or very long text (log injection, bounded only by the 256-entry warn-once set). It carries no credential and no content, so the redaction rule is not violated.
**Fix:** Truncate the model id and strip control characters (or use `{model:?}`) before logging.

---

_Reviewed: 2026-10-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
