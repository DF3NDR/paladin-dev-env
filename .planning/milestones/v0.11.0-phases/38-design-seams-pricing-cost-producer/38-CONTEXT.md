# Phase 38: Design Seams & Pricing/Cost Producer - Context

**Gathered:** 2026-09-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 38 delivers four things and nothing else:

1. **ADR-0052 — mid-run Treasurer enforcement attachment point** across `WarEngine` (engine path)
   and `PaladinExecutionService` (agent loop), written before any code in this phase.
2. **ADR-0053 — ledger balance model** (append-only, derive-on-read) including the reserve /
   settle / release row shape and the settlement idempotency key that Phase 39's `007` migration
   will implement.
3. **An operator-configured per-model price table** (`treasurer:` section, decimal strings per
   1M tokens, one ISO currency, empty by default, malformed / negative prices rejected at config
   validation) — PRICE-01.
4. **A pure fixed-point cost function** (`TokenUsage` × price row → `Cost` in i64 nano-units, no
   `f64` accumulation, unit-tested per token type) and a **real producer for
   `ExecutionMetadata.cost_estimate`** on both run paths, with the rustdoc reserved note changed
   to "produced by the Treasurer" — PRICE-02, PRICE-03.

Not in this phase: the ledger port and adapters (Phase 39), tenant identity (Phase 40),
allowances and admission refusal (Phase 41), the halt itself and its SSE fix (Phase 42), rate
pacing (Phase 43), persisting cost on the `Run` row or showing it on `GET /runs*` / the CLI
(Phase 39 LEDGR-04), and the Treasurer mdBook page (Phase 46 CURR-23).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by earlier phases and milestone-level decisions — not re-asked)

- **D-00a:** Full governance (per-tenant + per-API-key) is in scope for the milestone; prices come
  from operator config only, nothing bundled (REQUIREMENTS.md "Out of Scope": bundled table).
- **D-00b:** One operator currency in v0.11.0. Multi-currency / FX (FUT-12) and long-context tier
  pricing (FUT-11) are v2 — do not design hooks for them.
- **D-00c:** An unpriced model yields `None`, never `0` (PRICE-03). Internal currency math is
  integer fixed-point; `f64` is never accumulated or compared (research Pitfall 1).
- **D-00d:** `Treasurer` is a framework-only word (ADR-0050, Milestone 13 overview §5.3).
  `TokenBudget`, `TokenCounterPort`, `TokenUsage` and `max_tokens` are not renamed (ADR-0049,
  ADR-0050, two-officer model). The Commissary is untouched.
- **D-00e:** Vocabulary rules hold in every new line: Medieval-military terms for domain roles
  (Phase 30 D-01); no bare token total beside a `TokenUsage` split (Phase 31 D-08).
- **D-00f:** ADRs use the flat zero-padded counter in `.planning/decisions/PROMOTION.md`; the next
  free numbers are 0052 and 0053, and the numbering index there is updated in the same commit.
- **D-00g:** X-03 governs public API: any break needs a `MIGRATION.md` §9.2 row and a
  `cargo semver-checks` allowlist entry (ADR-0051 superseded X-03 for Phases 31-33 only). This
  phase is designed to be **additive** (see D-03, D-10) so no such row is expected.
- **D-00h:** Config sub-structs mirror `AgentRuntimeConfig`: `Default` + `validate()` +
  `EnvOverridable` with `APP_*` env vars, inert when the section is omitted, `validate()` never
  clamps.
- **D-00i:** Shipped tree outranks any document (Phase 34 D-00g and successors). In particular the
  research summary's `src/config/application_settings.rs` path is stale — the settings root is
  `src/config/settings.rs`.
- **D-00j:** 82 % workspace line-coverage floor (ADR-0006); `make clean-code`, `make api-surface`
  (an intentional surface change is refreshed with `make api-surface-update` plus a CHANGELOG
  entry), `make security`, and the manual credential-handling review still gate every commit.

### Price units & rounding

- **D-01:** Unit prices are written **per 1M tokens** as decimal strings (e.g. `"2.50"` for
  $2.50/M), matching every provider's published sheet so operators copy figures verbatim.
  — **Reversibility:** costly — every operator config and the docs would need rewriting if the
  unit changed after release.
- **D-02:** The internal cost unit is **nano-units of the currency (1e-9) held in `i64`**. Each
  LLM call is rounded **once, half-up, to the nearest nano-unit**; run totals are exact integer
  sums of the per-call figures (saturating, like `TokenUsage`). PRICE-02's "micro-units" wording
  is satisfied at a finer scale for the reason recorded here: at $0.15/M a single token is 0.15
  micro-units and would round to zero per call. Intermediate products (`u32` tokens × per-1M
  nano price) must use `i128` before the divide-by-1M and rounding step; there is no `f64`
  anywhere in the path. — **Reversibility:** one-way — the unit is the ledger's persisted
  amount from Phase 39 on; changing it later needs a data migration.
- **D-03:** `ExecutionMetadata.cost_estimate` **stays `Option<f64>`**. The nano-unit `i64` is
  authoritative; the `f64` is produced exactly once at the display edge (`nanos as f64 / 1e9`)
  and is never fed back into any comparison, aggregation or enforcement. No public API break,
  no MIGRATION row.
- **D-04:** The price table carries one **`currency`** field: an ISO 4217 3-letter code, default
  `"USD"`, validated (exactly three ASCII uppercase letters). Heralds stop printing a hard-coded
  `$` and render the cost as four decimals followed by the code (e.g. `0.0450 USD`) so a EUR
  operator never sees a dollar sign. The JSON herald keeps emitting the numeric field and adds the
  currency beside it.

### Price table keying & gaps

- **D-05:** A price row is keyed by the **bare model name** and matched against
  `LlmResponse.model` (the string the provider echoes back) — the same spelling every
  `default_model` in `config.example.yml` already uses. No provider plumbing. A `provider/model`
  composite key may be added additively later if a real collision appears (deferred, see below).
- **D-06:** A row **requires `prompt` and `completion`**; `cache_read`, `cache_write` and
  `reasoning` are optional. Because `TokenUsage` already counts cache tokens inside
  `prompt_tokens` and reasoning tokens inside `completion_tokens`, an omitted `cache_read` /
  `cache_write` bills those tokens at the `prompt` price and an omitted `reasoning` bills at the
  `completion` price. A priced row therefore always yields `Some(cost)`. Cost per call is:
  `(prompt − cache_read − cache_write) × prompt + cache_read × cache_read_price + cache_write ×
  cache_write_price + (completion − reasoning) × completion + reasoning × reasoning_price`, with
  `None` sub-counts treated as zero. Unit tests cover each axis, the fallback-to-parent rule, and
  the `None`-sub-count rule.
- **D-07:** The table lives in a **new top-level `treasurer:` config section** implemented in
  `src/config/treasurer.rs`, shaped
  `treasurer: { currency: "USD", pricing: { <model>: { prompt, completion, cache_read?,
  cache_write?, reasoning? } } }`, wired into `Settings` (`src/config/settings.rs`) exactly like
  `agent_runtime`. Omitting the section changes nothing (empty table, USD). Phases 41 and 43 add
  `allowance` and pacing keys under this same section. Validation rejects negative, non-decimal,
  empty and NaN-like strings and unknown currency shapes; zero is a valid price (free tier).
  — **Reversibility:** costly — the section name is operator-facing config.
- **D-08:** When a call's model has no row, cost is `None` **and one `warn`-level log/trace line
  per unpriced model per process** names the model, deduplicated (a once-set keyed by model
  name). Never a warning per call, never silent.

### Cost producer path

- **D-09:** Cost is computed **per call, at the `LlmPort` boundary**, by a pricing decorator
  wrapping `Arc<dyn LlmPort>` in `paladin-llm` (sibling shape to `FallbackLlmAdapter`,
  `crates/paladin-llm/src/fallback.rs`). It prices each `LlmResponse` from that response's own
  `model` and `usage`, so it behaves identically on the engine path and the agent loop, survives
  `FallbackLlmAdapter` provider hops and mixed-model graphs, and is the natural home for the
  Phase 43 pacing decorator. Streaming responses are priced from the terminal chunk's usage only
  (Phase 31 contract).
- **D-10:** **Cost rides beside usage everywhere usage travels.** A small domain value type
  (working name `Cost { nanos: i64, currency: CurrencyCode }`, saturating `Add`/`AddAssign`/`Sum`
  like `TokenUsage`, `Debug`/`Clone`/`PartialEq`/`Serialize`/`Deserialize`) is added as an
  **additive `#[serde(default)] Option<Cost>` field next to `usage`** on `LlmResponse`
  (`paladin-ports`), `PaladinResult` (`paladin-core`), `TraceEvent::NodeFinished` and
  `TraceEvent::RunFinished` (`paladin-core`). Aggregation reuses the exact paths `TokenUsage`
  already flows through (`TraceDispatcher::total_usage` gains a `total_cost` twin). `None`
  propagates: if any priced call in a run was unpriced, the run cost is `None`. Pre-existing
  serialized documents without the field deserialize with `cost == None` (the D-25 precedent).
  — **Reversibility:** costly — four public structs gain a field; removing it later is an X-03
  break.
- **D-11:** Surfaces that must show cost **in this phase**: (a) `TraceEvent::RunFinished`
  carries the run total so `LogTraceSink` and every trace consumer sees it; (b) heralds
  (markdown, JSON, table) print `ExecutionMetadata.cost_estimate` with the currency per D-04.
  `PaladinResult.cost` is populated (D-10) but exposing it on the HTTP agent response, persisting
  cost on the `Run` row, `GET /runs/{id}` and the CLI are **Phase 39 LEDGR-04**.
- **D-12:** `ExecutionMetadata` gets a **real production producer on both run paths**: the engine
  builds it from `RunFinished` (model, usage, cost, duration) and the agent loop builds it when a
  streamed execution finishes; each hands it to `Herald::finalize_stream`. Today nothing outside
  tests constructs the type. The rustdoc on `cost_estimate`, `total_cost()` and the struct-level
  field list (`crates/paladin-core/src/platform/container/herald.rs`) changes from "Reserved for
  the Treasurer … no in-tree producer yet" to "produced by the Treasurer", and the illustrative
  `.cost_estimate(0.045)` doc example becomes a real one.

### ADR posture

- **D-13 (ADR-0052, attachment point):** The operator's leaning, to be validated by research and
  recorded: **metering lives in the `LlmPort` pricing decorator on both paths** (D-09), and the
  **halt is raised where a run can checkpoint** — the engine's superstep loop for `WarEngine`
  runs, and the existing `TokenBudget` cutoff (`StopReason::TokenBudget`,
  `src/application/services/paladin/middleware/limits.rs`) for the agent loop.
  `AgentRuntimeConfig::build_chain` **stays unwired for the engine path**. The ADR must state the
  rejected alternative (wiring `build_chain` into the engine) and why, and must be written so
  Phase 42's plans cite it rather than re-open the question. — **Reversibility:** costly —
  Phases 41 and 42 build on it.
- **D-14 (ADR-0053, ledger model):** **Append-only ledger, balance derived on read.** Every
  reserve / settle / release is an immutable row; the window balance is a `SUM` over rows in the
  period computed inside the same transaction as the reserve (`SELECT … FOR UPDATE` on Postgres,
  SQLite's serialized writer, a mutex for in-memory). One table, no drift between a counter and
  its history, audit trail for free; the growth cost is mitigated by an index on
  (tenant, key, settled_at). The rejected alternative (a running `allowance_windows` balance
  table) is recorded with its drift and rollover costs. — **Reversibility:** one-way — Phase 39's
  `007` migration implements it.
- **D-15 (ADR-0053 depth):** The ledger ADR fixes what Phase 39 needs before writing its
  migration: the balance model (D-14), the **row kinds** (reserve / settle / release), **amounts in
  `i64` nano-units with the currency code** (D-02), and the **settlement idempotency key
  `(run_id, superstep, attempt)`**. Column names, indexes and DDL stay Phase 39's.
- **D-16 (timing):** **ADRs first, as the phase's opening plan** (38-01): write ADR-0052 and
  ADR-0053, update `PROMOTION.md`'s numbering index, then start the pricing plans. This matches
  the roadmap's "recorded before any dependent phase begins" and lets research findings land in
  the ADR text. Pricing plans may run in parallel with each other but not before 38-01 is
  committed.

### Claude's Discretion

- **Type homes.** `Cost`, `CurrencyCode`, `PriceRow`/`PriceTable` (nano-units per 1M tokens as
  `i64`) and the pure cost function belong in `paladin-core` (pure, no new deps) so `paladin-llm`
  and `paladin-battalion` can use them without importing the facade. `rust_decimal` (1.43, MIT,
  allowed by `deny.toml`, not yet in `Cargo.lock`) is used **only** in the facade config parser
  (`src/config/treasurer.rs`) to turn decimal strings into `i64` nano-units per 1M; the planner
  may substitute a hand-rolled exact decimal-string parser if it keeps the dependency count flat.
- **Decorator wiring.** Where the pricing decorator is installed (ProviderFactory,
  `setup/`, the run worker's LLM resolution) so that both run paths get it whenever the
  `treasurer.pricing` table is non-empty — and whether an empty table skips installing it.
- **`model_used` on multi-model runs.** What `ExecutionMetadata.model_used` says for an engine
  run that touched several models (e.g. the first Paladin node's model, or `"mixed"`); the
  per-call `Cost` already handles the money side correctly.
- **Env overrides for a map-shaped table.** `EnvOverridable` for `treasurer.currency` is
  straightforward (`APP_TREASURER_CURRENCY`); whether a per-model price can be overridden by env
  at all, or only via the file, is the planner's call — document the choice in rustdoc either way.
- **Dedup mechanism** for the one-warning-per-unpriced-model rule (D-08).
- **Test fixture shape** for the per-token-type unit tests and the end-to-end herald test with a
  mock `LlmPort` returning a priced model.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone scope and requirements
- `.project/Milestone_14-Treasurer/overview/Milestone-14_Treasurer.md` — why the Treasurer is
  its own milestone, the two-officer model, the `GarrisonTreasury` vocabulary guardrail (§3).
- `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` — R1 (price table
  + cost function), R2 (`cost_estimate` producer + rustdoc note), §4 out of scope, §5 tests.
- `.planning/REQUIREMENTS.md` — PRICE-01..03 (this phase), LEDGR-01..04 and ALLOW-03/05 (what
  the two ADRs must unblock), "Out of Scope" table, v2 FUT-11/FUT-12.
- `.planning/ROADMAP.md` — Phase 38 goal, success criteria 1-4, research flag; Phase 39/41/42
  "Depends on" lines that name this phase's ADRs.
- `.planning/PROJECT.md` — *Current Milestone: v0.11.0 Treasurer Spend Governance* section (key
  context: MSRV 1.88, coverage floor, tags on `main` merge commits).

### Research
- `.planning/research/SUMMARY.md` — stack (`rust_decimal`, `i64` fixed point, sqlx has no
  decimal support on SQLite), architecture (`TreasuryLedgerPort` mirrors `RunRepositoryPort`),
  Pitfalls 1-5, the attachment-point gap, the derive-on-read vs running-balance gap.

### Standing decisions
- `.planning/decisions/0049-commissary-design-and-rename.md` — Commissary is input-side and
  untouched; naming rules.
- `.planning/decisions/0050-treasurer-reservation.md` — `Treasurer` reserved as the output-side
  officer; 0/0 as a code symbol until this milestone; framework-only word.
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — X-03 still governs
  every public API outside Phases 31-33; MIGRATION §9.2 row + semver-checks allowlist on any
  break.
- `.planning/decisions/PROMOTION.md` — ADR numbering scheme and index (next free: 0052, 0053),
  required headings, supersession mechanism.

### Code the phase extends (read, do not re-derive)
- `crates/paladin-core/src/platform/container/token_usage.rs` — `TokenUsage` five-axis shape,
  containment invariants, saturating accumulation (the model for `Cost`).
- `crates/paladin-core/src/platform/container/herald.rs` — `ExecutionMetadata`,
  `ExecutionMetadataBuilder`, `total_cost()`, the reserved-note rustdoc to rewrite, the
  `Herald::finalize_stream` contract.
- `crates/paladin-core/src/platform/container/trace.rs` — `TraceEvent::NodeFinished` /
  `RunFinished` (gain `cost`), `FallbackHop`.
- `crates/paladin-core/src/platform/container/execution_result.rs` — `PaladinResult.usage`
  (gains a `cost` sibling) and the D-25 `#[serde(default)]` precedent.
- `crates/paladin-ports/src/output/llm_port.rs` — `LlmResponse { model, usage, … }`,
  `LlmPort::get_provider_name`.
- `crates/paladin-llm/src/fallback.rs` — `FallbackLlmAdapter`, the decorator shape to mirror.
- `crates/paladin-battalion/src/engine/hooks.rs` and `engine/mod.rs` — `TraceDispatcher::
  total_usage` and the `RunFinished` emission site (~line 2089) where the engine-side
  `ExecutionMetadata` producer attaches.
- `src/config/agent_runtime.rs` (`TokenBudgetConfig`, `EnvOverridable`, `build_chain`) and
  `src/config/settings.rs` — the config shape and the `Settings` root to extend.
- `src/application/services/paladin/middleware/limits.rs` — `TokenBudget` cutoff the agent-loop
  side of ADR-0052 reuses.
- `crates/paladin-herald/src/{markdown,json,table}_herald.rs` — `finalize_stream` cost rendering
  to change per D-04.
- `config.example.yml` — where the `treasurer:` example section goes, in the `agent_runtime:`
  comment style.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TokenUsage` (five axes, `None` = unreported, saturating `Add`/`Sum`) — `Cost` copies its
  shape and accumulation rules one-for-one.
- `FallbackLlmAdapter` — proven `Arc<dyn LlmPort>` decorator pattern with its own error type and
  `get_provider_name`; the pricing decorator is its sibling.
- `AgentRuntimeConfig` / `TokenBudgetConfig` — the `Default` + `validate()` + `EnvOverridable`
  config idiom, including the "omit the section, nothing changes" guarantee and the
  `agent_runtime.token_budget.max_tokens must be …` error-message style.
- `TraceDispatcher::total_usage()` — per-run aggregation of `NodeFinished.usage`; add
  `total_cost()` beside it.
- `ExecutionMetadataBuilder` — already validates and carries `cost_estimate`; only needs a
  production caller and rustdoc changes.
- Heralds already branch on `cost_estimate.is_some()`; only the format string changes.

### Established Patterns
- Hexagonal boundaries: `paladin-core` has no deps beyond serde/uuid/chrono; ports import core;
  `paladin-llm` imports ports + core; the facade (`src/`) is the only place config parsing and
  `rust_decimal` may live.
- Additive fields on serialized domain types use `#[serde(default)]` so persisted JSON from earlier
  versions still deserializes (`PaladinResult.usage`, D-25).
- Per-run scratch state never lives on a shared middleware/decorator struct (limits.rs D-03); the
  pricing decorator must be stateless apart from the immutable price table and the warn-once set.
- Trace events are the run's audit stream; `RunFinished.usage` is tallied synchronously in the
  dispatcher, not by the async consumer (engine/mod.rs D-02/D-04/D-11) — `cost` follows the same
  rule.

### Integration Points
- `LlmResponse` (ports) → pricing decorator (paladin-llm) → `PaladinResult.cost` (agent loop) and
  `NodeFinished.cost` (engine bridges) → `RunFinished.cost` (dispatcher) → `ExecutionMetadata`
  producer → `Herald::finalize_stream`.
- `Settings.treasurer` (new) → `TreasurerConfig::validate()` at boot → `PriceTable` handed to
  whichever factory builds the `LlmPort` for both the run worker and the agent HTTP route.
- `.planning/decisions/0052-*.md`, `0053-*.md` + `PROMOTION.md` index — Phase 39/41/42 plans
  cite them by number.
- Model strings are bare provider IDs everywhere (`gpt-4`, `claude-3-5-sonnet-20241022`,
  `kimi-k3`, `qwen-plus`, `grok-4.6`); no provider-qualified naming exists to reuse.

</code_context>

<specifics>
## Specific Ideas

- Operators should be able to paste a provider's price sheet line for line: `"2.50"` prompt,
  `"10.00"` completion, `"0.25"` cache_read — no unit conversion, no scientific notation.
- A blank cost in herald output must be explainable from the log: exactly one
  "no price configured for model X" line per process, not zero and not thousands.
- The herald line reads like `Cost: 0.0450 USD`, never `$0.0450` once a currency is configured.
- The two ADRs are written to be *cited*, not re-argued: each names the rejected alternative and
  the phase (39, 41, 42) that must reference it.

</specifics>

<deferred>
## Deferred Ideas

- **`provider/model` composite price keys** — add additively only if a real cross-provider
  collision on a bare model name shows up; not in v0.11.0.
- **Cost on `PaladinResult` over HTTP, on the `Run` row, `GET /runs/{id}` and the CLI** — Phase 39
  LEDGR-04 ("spend per tenant, API key, run and model … from the CLI, herald output and trace
  events").
- **Changing `cost_estimate` to a fixed-point money type** — considered and rejected for v0.11.0
  (X-03 break for no operator-visible gain); revisit only at a 1.0 API reshape.
- **Multi-currency / FX (FUT-12) and long-context tier pricing (FUT-11)** — v2, per
  REQUIREMENTS.md.

### Reviewed Todos (not folded)
- *Evaluate replacing MinIO with RustFS in the dev/test stack*
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, match score
  0.2 on the keyword "phase" only) — belongs to Phase 45 (STORE-01..03); not this phase.

</deferred>

---

*Phase: 38-design-seams-pricing-cost-producer*
*Context gathered: 2026-09-25*
