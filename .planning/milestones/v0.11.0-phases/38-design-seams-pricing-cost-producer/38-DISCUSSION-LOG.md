# Phase 38: Design Seams & Pricing/Cost Producer - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-25
**Phase:** 38-design-seams-pricing-cost-producer
**Areas discussed:** Price units & rounding, Price table keying & gaps, Cost producer path, ADR posture

Before the areas were chosen, the operator also approved the v0.11.0 roadmap as written
(Phases 38-47; already committed as `fce163be` / `08715648`).

---

## Price units & rounding

### Q1 — In the price table, what unit does an operator write a unit price in?

| Option | Description | Selected |
|--------|-------------|----------|
| Per 1M tokens | Matches every provider's published sheet; per-token prices would truncate to 0 in micro-units | ✓ |
| Per 1K tokens | Older OpenAI convention; operators must divide per-1M sheets by 1000 | |
| Per token | Most literal; long decimal strings and nano-or-finer scaling needed | |

**User's choice:** Per 1M tokens

### Q2 — What internal fixed-point unit and rounding rule should the cost function use?

| Option | Description | Selected |
|--------|-------------|----------|
| Nano-units, round half-up per call | 1e-9 of the currency in i64; a 1-token call at $0.15/M is 150 nano-units; run totals are exact sums | ✓ |
| Micro-units, round half-up per call | Literal PRICE-02 wording; sub-micro prices round to 0 per call | |
| Micro-units, exact per run | Unrounded rational per call, rounded once at run total; wider intermediate type | |

**User's choice:** Nano-units, round half-up per call
**Notes:** PRICE-02's "micro-units" is satisfied at a finer scale; rationale recorded in CONTEXT D-02.

### Q3 — How should the exact cost reach the public `ExecutionMetadata.cost_estimate` field (`Option<f64>` today)?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep Option<f64>, convert at boundary | No API break; i64 authoritative, f64 produced once at display edge | ✓ |
| Change to a fixed-point money type | Honest type but X-03 break: MIGRATION row, semver allowlist, herald updates | |
| Add a sibling exact field, keep f64 | Additive but two fields must agree | |

**User's choice:** Keep Option<f64>, convert at boundary

### Q4 — How should the operator's currency be declared and shown?

| Option | Description | Selected |
|--------|-------------|----------|
| ISO code in config, heralds print it | `currency: "USD"` default, validated; heralds print `0.0450 USD` | ✓ |
| ISO code in config, keep `$` display | Store the code, leave herald formatting for later | |
| No currency field, USD assumed | Simplest; locks operators to USD semantics | |

**User's choice:** ISO code in config, heralds print it

---

## Price table keying & gaps

### Q1 — How is a price row keyed and matched to a call?

| Option | Description | Selected |
|--------|-------------|----------|
| Bare model name, matched to LlmResponse.model | Same spelling as every `default_model`; no provider plumbing | ✓ |
| provider/model composite key | Unambiguous across providers; operators must learn provider names | |
| Bare name with optional provider override | Two lookup shapes to test and document | |

**User's choice:** Bare model name, matched to LlmResponse.model

### Q2 — What does a row that omits some of the five axes mean?

| Option | Description | Selected |
|--------|-------------|----------|
| Prompt & completion required; cache/reasoning default to parent | Omitted cache axes bill at prompt price, omitted reasoning at completion price | ✓ |
| All five axes required | Missing any price is a validation error | |
| Any missing axis makes the model unpriced | Partial row yields None for every call | |

**User's choice:** Prompt & completion required; cache/reasoning default to parent

### Q3 — Where does the price table live in config.yml, and what shape?

| Option | Description | Selected |
|--------|-------------|----------|
| New top-level `treasurer:` section | `src/config/treasurer.rs`, mirrors AgentRuntimeConfig; allowance/pacing keys join later | ✓ |
| Under the existing `llm:` section | Prices near model names but later keys would split | |
| Under `agent_runtime:` | Wrong officer; engine path never reads it | |

**User's choice:** New top-level `treasurer:` section

### Q4 — When a call's model has no price row, cost is None (locked). Should anything else happen?

| Option | Description | Selected |
|--------|-------------|----------|
| None plus one trace warning per model per process | First call on an unpriced model logs once, deduplicated | ✓ |
| Silent None | No signal at all | |
| None plus a warning on every call | Floods the log on a busy campaign | |

**User's choice:** None plus one trace warning per model per process

---

## Cost producer path

### Q1 — Where is cost computed?

| Option | Description | Selected |
|--------|-------------|----------|
| Per call, at the LlmPort boundary | Pricing decorator wrapping Arc<dyn LlmPort>; identical on both paths, fallback-safe | ✓ |
| Per run, from aggregated TokenUsage | Single model name; wrong for multi-model runs; trace events carry no model | |
| Per node, in the engine and agent loop separately | Two code paths; blind to a fallback hop's real model | |

**User's choice:** Per call, at the LlmPort boundary

### Q2 — How does per-call cost travel up to the run total and into ExecutionMetadata?

| Option | Description | Selected |
|--------|-------------|----------|
| Cost rides beside usage everywhere usage travels | Additive `Option<Cost>` next to `usage` on LlmResponse, PaladinResult, NodeFinished, RunFinished | ✓ |
| Separate trace event per priced call | New TraceEvent + listener; decorator has no run id today | |
| Stuff cost into LlmResponse.metadata | Stringly typed, no place to sum | |

**User's choice:** Cost rides beside usage everywhere usage travels

### Q3 — Which surfaces must show a run's cost in THIS phase? (multi-select)

| Option | Description | Selected |
|--------|-------------|----------|
| RunFinished trace event carries total cost | `cost` beside `usage` on the terminal trace event | ✓ |
| Heralds print cost from ExecutionMetadata | Markdown/JSON/table render cost with currency; real producer added | ✓ |
| Agent-loop PaladinResult carries cost | Populate for the HTTP agent route this phase | |
| Run API / CLI show cost | Persist on Run row, GET /runs/{id}, CLI — overlaps LEDGR-04 | |

**User's choice:** RunFinished trace event + heralds. HTTP/Run API/CLI surfaces left to Phase 39 LEDGR-04.

### Q4 — Nothing in production builds ExecutionMetadata today. Where should the producer live?

| Option | Description | Selected |
|--------|-------------|----------|
| Both run paths build it at completion | Engine from RunFinished, agent loop on streamed completion; each hands it to Herald::finalize_stream | ✓ |
| Agent loop only this phase | Smaller diff; engine runs still produce none | |
| Engine only this phase | Primary path covered; legacy HTTP route reports no cost | |

**User's choice:** Both run paths build it at completion

---

## ADR posture

### Q1 — For the mid-run enforcement attachment point ADR, what is your leaning?

| Option | Description | Selected |
|--------|-------------|----------|
| LlmPort decorator on both paths, engine hook for the halt | Metering in the pricing decorator; halt raised at checkpoint points (engine superstep loop / TokenBudget cutoff); build_chain stays unwired | ✓ |
| Wire build_chain into the engine path | One mechanism on both paths; first-time change to the primary run path | |
| No leaning, researcher decides | Hand over constraints, accept the ADR's recommendation | |

**User's choice:** LlmPort decorator on both paths, engine hook for the halt

### Q2 — For the ledger balance model ADR, what is your leaning?

| Option | Description | Selected |
|--------|-------------|----------|
| Append-only ledger, derive balance on read | Immutable rows; SUM inside the reserving transaction; one table; audit trail | ✓ |
| Running balance per window | Second table, rollover job, drift risk | |
| No leaning, researcher decides | Constraints only | |

**User's choice:** Append-only ledger, derive balance on read

### Q3 — How far should the ledger ADR go?

| Option | Description | Selected |
|--------|-------------|----------|
| Balance model plus row shape and idempotency key | Row kinds, i64 nano-unit amounts + currency, key (run_id, superstep, attempt); DDL stays Phase 39's | ✓ |
| Balance model only | Phase 39 re-opens the row shape | |
| Full schema in the ADR | Complete 007 DDL locked before any contract test | |

**User's choice:** Balance model plus row shape and idempotency key

### Q4 — When in the phase are the two ADRs written, and how are they numbered?

| Option | Description | Selected |
|--------|-------------|----------|
| ADRs first, as the phase's opening plan | Plan 38-01 writes ADR-0052/0053 and updates PROMOTION.md, then pricing plans | ✓ |
| ADRs last, after the pricing code | Hindsight from code; blocks Phase 39/41/42 planning | |
| ADRs in parallel with pricing plans | Fastest; decorator design and ADR agreement becomes accidental | |

**User's choice:** ADRs first, as the phase's opening plan

---

## Claude's Discretion

- Type homes for `Cost` / `CurrencyCode` / `PriceTable` / cost function (paladin-core) and where
  `rust_decimal` (or a hand-rolled exact parser) is used (facade config only).
- Where the pricing decorator is installed so both run paths get it; whether an empty table skips
  installation.
- What `ExecutionMetadata.model_used` reports for a multi-model engine run.
- Env-override shape for the map-shaped price table.
- Dedup mechanism for the one-warning-per-unpriced-model rule.
- Test fixture shape for per-token-type unit tests and the end-to-end herald test.

## Deferred Ideas

- `provider/model` composite price keys (add only on a real collision).
- Cost on `PaladinResult` over HTTP, on the `Run` row, `GET /runs/{id}` and the CLI → Phase 39
  LEDGR-04.
- Fixed-point money type replacing `cost_estimate: Option<f64>` → not before a 1.0 API reshape.
- Multi-currency / FX (FUT-12), long-context tier pricing (FUT-11) → v2.
- Reviewed, not folded: RustFS-for-MinIO todo → Phase 45.
