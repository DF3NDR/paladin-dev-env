# Phase 41: Admission-Time Allowance Enforcement - Context

**Gathered:** 2026-10-02
**Status:** Ready for planning
**Mode:** interactive — every decision below was selected by the operator (four gray areas,
sixteen questions, logged in `41-DISCUSSION-LOG.md`). The recommended option was chosen in
fifteen of sixteen; the one deliberate departure is D-08 (schedule-fired runs are brought under
allowances in this phase rather than recorded as a gap).

<domain>
## Phase Boundary

This phase delivers the **admission slice of the Treasurer** — the first code that can say "no"
to a run on the strength of money already spent. Concretely:

1. **Allowance configuration** (ALLOW-01). A new `treasurer.allowance` section lets an operator
   set, per tenant and per API key, a rolling-period allowance with an optional lifetime cap, in
   the operator currency, distinct from every existing `max_tokens` meaning. Window boundaries are
   computed in UTC from the store clock (`TreasuryLedgerPort::store_now`), never a worker's local
   clock.
2. **Admission refusal** (ALLOW-02). Submitting a run — `POST /runs`, fork, the HTTP agent
   execute/stream routes, and a schedule-fired run — while the caller's tenant or API-key allowance
   is exhausted is refused with a typed error **before any run row is written**, as `429` on the
   wire.
3. **Warn-threshold notice** (ALLOW-04). Crossing a configurable threshold (default 80 %) emits
   exactly one trace event plus a herald line and an operator webhook notice per window, deduped
   through the store so the guarantee holds across worker replicas, without blocking the run.
4. **The gating pieces those need**: one additive balance-read method on `TreasuryLedgerPort`,
   a store-backed once-per-window notices table, the creator principal persisted on the schedule
   row, one core `AllowanceRefusal` value, one new `TraceEvent` variant, ADR-0056, and the usual
   X-03 / `MIGRATION.md` / CHANGELOG / docs bookkeeping.

**Not in this phase:** reservations (holds) at admission or at the superstep boundary, the
mid-run halt, `RunStatus::Halted` from a spend check, deriving a per-run `TokenBudget` from the
remaining allowance, and the SSE `done`/`Cancelled` fix — all Phase 42 (ALLOW-03, ALLOW-05,
PLAT-09, ADR-0052). Rate pacing (Phase 43). A warn-threshold ladder, a `treasury allowance` CLI
view, a tenant registry, per-API-key read narrowing, thread-route scoping, multi-currency FX —
see *Deferred Ideas*.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by ADR-0052/0053, Phases 38-40 and milestone-level decisions — not re-asked)

- **D-00a:** ADR-0053 and Phase 39 D-01/D-03/D-04 are binding and **cited, not re-argued**: the
  `007` ledger schema and every existing `TreasuryLedgerPort` method are final; the port knows
  **no allowance policy** — this phase computes ceilings and windows from config and hands them
  in; a scope+window balance is the plain `SUM` of signed `amount_nanos` contributions over rows
  whose `attributed_at` falls in `[window_start, window_end)`; `store_now()` is the only clock
  for window boundaries (ALLOW-01); `TreasuryLedgerError::Refused { balance, hold, ceiling }`
  already carries the figures this phase's refusal reuses verbatim.
- **D-00b:** ADR-0052 fixes where Phase 42 halts (engine superstep boundary; agent-loop
  `TokenBudget` cutoff). This phase writes **no `reserve` rows**, touches neither
  `crates/paladin-battalion/src/engine/{superstep,mod}.rs` nor
  `src/application/services/paladin/middleware/limits.rs`, and wires no `TokenBudget`. Everything
  it installs at admission must leave Phase 42's reservation trivially attachable.
- **D-00c:** Identity is Phase 40's: `PrincipalRef { api_key_id, tenant_id, role }` on
  `SubmitRun.requested_by` / `ForkRun.requested_by`, `None` = internal caller; the tenant is
  server-derived only (Phase 40 D-02); `paladin_web::agent_auth::Principal { id, role, tenant_id
  }` on every authenticated route; `LedgerScope::from_attribution` is the only attribution→scope
  mapping (Phase 40 D-15/40-04). Phase 40 D-11's Admin bypass is a **read-visibility** rule and
  carries no implication for spend (see D-09).
- **D-00d:** Config sub-structs follow the house shape — `Default` + `validate()` that never
  clamps and fails closed naming the offending entry + `EnvOverridable` where a scalar warrants
  it; collection-shaped fields are config-file only; omitting the section changes nothing
  (Phase 38 D-00h, `src/config/treasurer.rs` module docs, which already say "Later phases (41,
  43) add `allowance` and pacing keys under this same `treasurer:` section"). Decimal strings are
  parsed by the existing exact-integer grammar (`parse_price_nanos_per_million`'s rules: ASCII
  digits, optional `.` and 1-9 digits, no sign/exponent/separator), never `f64`.
- **D-00e:** X-03 governs public API: every break needs a `MIGRATION.md` §9.2 row and, when
  marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry (Phase 38 D-00g, 39-08 D-27 method);
  the CI `semver` job is a known no-op against the 0.9.0 baseline (STATE.md concern) — the
  register is the control. New `TraceEvent` variants are N/A-registered (38-09 precedent: the
  type postdates the baseline). `RunSubmissionError` and `TraceEvent` are both
  `#[non_exhaustive]`, so the new variants are additive on the Rust side.
- **D-00f:** Vocabulary: `Treasurer` is the officer and a framework-only word (ADR-0050;
  Milestone 14 overview §3 `GarrisonTreasury` guardrail); "allowance", "window", "tenant",
  "API key", "notice" are plain words for units and identifiers (Phase 30 D-01). The service
  type may now be named `Treasurer` — ADR-0050's dated note (2026-09-25) released the 0/0
  code-symbol condition for Milestone 14.
- **D-00g:** Gates: 82 % workspace line-coverage floor (ADR-0006); `make clean-code`,
  `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional surface
  change), `make security`, and the manual credential-handling review in
  `.github/instructions/security.instructions.md` on every commit. **No log line, error body,
  trace event, webhook payload or herald line ever carries an API key value**; key *names*
  (`ApiKeyConfig.name`, already the `api_key_id`) and tenant ids are fine.
- **D-00h:** One operator currency in v0.11.0 (Phase 38 D-00b); every amount is `Cost { nanos:
  i64, currency }`; display goes through `format_cost` / `ExecutionMetadata::cost_display`
  (`0.0450 USD`) exactly once at the edge. A balance in a different currency than the configured
  allowance is a refusal (`CurrencyMismatch`), never a conversion.
- **D-00i:** ADRs take the next free number from `.planning/decisions/PROMOTION.md` — currently
  **0056** — and advance that line in the same commit (Phase 38 D-00f). Migrations continue the
  `NNN_verb_noun.sql` pair convention; `009_add_run_attribution_check.sql` already exists on both
  backends, so this phase's migrations start at **`010`**.

### Window semantics & allowance shape

- **D-01:** Windows are **tumbling and period-aligned to the UTC Unix epoch**: for a configured
  `period` of `P` seconds and a store clock reading `now` (from `store_now()`, truncated to
  seconds), `window_start = floor(now / P) * P` and `window_end = window_start + P`, both as
  `DateTime<Utc>`. No worker-local clock, no calendar arithmetic, no trailing window. An
  operator can predict exactly when spend resets; "one notice per window" has the stable key
  `window_start`; a refused caller can be told the reset instant. The documented cost is that a
  tenant can spend up to twice the allowance across one boundary — accepted. — **Reversibility:**
  costly — the reset instant becomes part of the documented `429`/`Retry-After` contract, the
  notices table is keyed on `window_start`, and Phase 42's reservations will use the same window
  function; switching to a trailing window later changes all three.
- **D-02:** Config grammar — a new `treasurer.allowance` subsection (`AllowanceConfig`, under the
  existing `TreasurerConfig`):
  ```yaml
  treasurer:
    currency: "USD"
    pricing: { ... }            # Phase 38, unchanged
    allowance:
      warn_at: 80               # default; integer percent of each ceiling; 0 disables
      webhook:                  # optional operator notice target (D-17)
        url: "https://ops.example.com/paladin/allowance"
        secret: "${ALLOWANCE_WEBHOOK_SECRET}"
      tenants:
        acme:       { period: "24h", amount: "25.00", lifetime: "500.00" }
      api_keys:
        ci-runner:  { period: "1h",  amount: "2.50", warn_at: 90 }
  ```
  `period` is `<integer><unit>` with units `m`, `h`, `d` (minutes exist for tests and short
  demos; weeks are not a unit — write `7d`); it must be ≥ `1m` and is used as a whole number of
  seconds. `amount` and `lifetime` are decimal strings in **whole currency units** (`"25.00"` =
  25 USD = `25_000_000_000` nano-units), parsed with the same exact-integer grammar as prices
  (one shared parser, scaled by `1e9` instead of per-1M); zero, malformed, negative-looking or
  overflowing values are rejected at `validate()` naming the full path
  (`treasurer.allowance.tenants.acme.amount`). Both maps are keyed by the plain `TenantId`
  string and the `ApiKeyConfig.name` respectively, and are **config-file only** (no env form);
  `treasurer.allowance.warn_at` is the one scalar with an env override
  (`APP_TREASURER_ALLOWANCE_WARN_AT`). Omitting `allowance:` entirely is inert. —
  **Reversibility:** costly — operator config files and `config.example.yml` carry the grammar;
  a changed grammar is a §9.5 break.
- **D-03:** Composition — **every configured limit must fit.** For a principal `(tenant, key)`
  up to four ceilings apply: key window, key lifetime, tenant window, tenant lifetime. Each is
  evaluated against its own balance; the first exhausted one (in a deterministic order the
  planner fixes, recommended key-window → key-lifetime → tenant-window → tenant-lifetime) names
  the refusal. A scope with no allowance entry has no ceiling and is **not consulted** (no ledger
  read) — "unlimited" is the absence of an entry, never a sentinel amount. A lifetime cap is a
  window `[Unix epoch, far future)` over the same balance function, i.e. every row the scope has
  ever written; it has no reset instant.
- **D-04:** The tenant-wide figure comes from **one additive read method on
  `TreasuryLedgerPort`** — `balance(BalanceQuery { tenant_id, api_key_id: Option<String>,
  window_start, window_end }) -> Result<Cost, TreasuryLedgerError>` (names are the planner's) —
  summing signed `amount_nanos` contributions over rows attributed inside the window;
  `api_key_id: None` sums every key of the tenant, `Some(k)` the exact `(tenant, key)` pair.
  Implemented on all three adapters (`in_memory`, `sqlite`, `postgres`) behind the shared
  contract suite (new clauses: tenant-wide vs key-scoped sums, window edge inclusivity, mixed
  currency → `CurrencyMismatch`, empty scope → zero in the requested currency). It reads
  `amount_nanos`, not `charged_nanos`, so Phase 42's reservations are automatically counted once
  they exist. "Final" in Phase 39 D-01 / Phase 40 D-00a covered the schema and the existing
  methods; an additive method is not a schema change. — **Reversibility:** costly —
  `TreasuryLedgerPort` is a published `paladin-ports` trait; a new required method breaks
  external implementors (`trait_method_added`), so it is recorded in §9.2 with an allowlist entry
  (`requirement_id = "ALLOW-01"`), and Phase 42's mid-run check builds on the same method.

### Admission check & coverage

- **D-05:** Admission is a **check only — no hold, no ledger write.** For each applicable
  ceiling the Treasurer reads the balance (D-04) and refuses when `balance >= ceiling` (a balance
  exactly at the ceiling is exhausted: nothing is left to spend). Two runs admitted in the same
  instant may both start; that race is closed by Phase 42's reservation at the superstep
  boundary (ADR-0052), not here. Nothing about a refused submit reaches the ledger. —
  **Reversibility:** reversible — Phase 42 adds a `reserve` beside this check without changing it.
- **D-06:** **One shared admission function**, owned by a new application-layer service type
  `Treasurer` (home: `src/application/services/treasurer/`, sibling of `run/` and `paladin/`;
  hexagonal: it depends on `paladin-ports` and `paladin-core` only, never on storage). Its
  entry point takes the principal (`&PrincipalRef`, or the agent route's `Principal` mapped to
  one) plus the id the caller is about to use (`RunId` for submit/fork, the execution id for
  agent execute) and returns `Ok(Admission)` carrying any warn crossing to notify, or
  `Err(AllowanceRefusal)` / a ledger error. It is called from exactly three places:
  `RunSubmissionService::submit`, `RunSubmissionService::fork`, and the `/agents/{id}/execute`
  and `/agents/{id}/execute/stream` handlers in `crates/paladin-web/src/agent_controller.rs`
  (before `execute_scoped` — the handler already has the `Principal`, so no executor-port error
  type changes). In `submit` it sits **after** the SSRF guard, `resolve` and
  `authorize_invocation` (cheap, pure refusals first) and **before** `insert_with_latest` /
  `insert` / `enqueue` — the SSRF guard's "reject before any row is written" shape, as the
  research names. The service is injected as `Option<Arc<Treasurer>>` on `RunSubmissionService`
  (a `with_treasurer` builder, mirroring `with_ssrf_guard`) and on `AgentApiState`; it is `None`
  — and admission is a no-op — when `treasurer.allowance` is omitted.
- **D-07:** Coverage — **every principal-bearing path is gated**; `requested_by: None`
  (same-process embedders, tests, CLI) is never gated and never attributed, exactly as Phase 40
  D-10 left it. The HTTP agent execute path is gated because Phase 40 D-16 made it settle spend
  under the principal; leaving it open would let a refused `POST /runs` caller spend the same
  allowance through `/agents/{id}/execute`.
- **D-08:** **Schedule-fired runs are brought under allowances in this phase** (the operator's
  one departure from the recommended option; closes Phase 40 D-10's deferred idea). `RunSchedule`
  gains `created_by: Option<RunAttribution>` (`#[serde(default)]`, builder-set), persisted by a
  new migration pair `010_add_run_schedule_created_by.sql` (`ALTER TABLE run_schedules ADD
  COLUMN tenant_id TEXT NULL`, `api_key_id TEXT NULL` — `NULL` = created before this phase or by
  a principal-less caller), stamped by `create_schedule` from its `Principal`, round-tripped by
  all three schedule adapters and their contract suite. `RunScheduleService`'s fire site builds
  the `SubmitRun` so that the fired run is **attributed to and admitted against** the creator's
  tenant and key; pre-existing rows with `NULL` keep today's behaviour (`requested_by: None`,
  unattributed, ungated) and are named in a `WINDOWS.md` row plus the schedule docs so the gap
  is visible, not silent. A creator key later removed from `http.auth.api_keys` still attributes
  and gates by its persisted name and tenant (an allowance keyed on that name keeps applying; a
  tenant allowance always applies). How the attribution travels through `SubmitRun` — a
  `PrincipalRef` carrying a role snapshot, or a separate attribution-only field — is the
  planner's call under one constraint: the invocation role check that schedule-fired runs skip
  today must not silently start applying (or silently keep being skipped) without a written
  decision in the plan. `PATCH /schedules/{id}` does not change `created_by` (deferred idea). —
  **Reversibility:** one-way — persisted columns on `run_schedules` on both backends.
- **D-09:** **No Admin bypass.** Allowances bind every principal regardless of role: an Admin
  key with an entry is refused and warned like any other; an unlimited key is one with no entry.
  Spend is money, not visibility, and an Admin credential must not be an unbounded spend path.
  Phase 40 D-11 (Admin reads every run) is unchanged and unrelated.
- **D-10:** **Fail closed when an allowance applies.** If any required balance read fails
  (`TreasuryLedgerError::Backend`, `CurrencyMismatch`, …) for a principal that has at least one
  configured ceiling, `submit`/`fork` return `RunSubmissionError::Backend { .. }` (`500`) and the
  agent handlers return the equivalent `ApiError::internal`; no run is persisted or enqueued. A
  principal with no configured ceiling never touches the ledger, so a deployment without
  allowances keeps working through a ledger outage. Phase 39 D-08's "a settle failure does not
  fail the run" stays true for *settlement*; admission is authoritative, not observational.
- **D-11:** **Boot-time coherence.** `treasurer.allowance` with any entry requires a ledger
  backend: `run_store.backend: disabled` plus a non-empty allowance section is a `validate()` /
  server-boot error naming both keys (fail closed, `build_auth_config` voice). Every
  `api_keys.<name>` entry must name a key in `http.auth.api_keys` and every `tenants.<id>` entry
  a tenant some API key or `bearer_token.tenant` maps to — a typo'd entry that silently enforces
  nothing is the Phase 40 D-05 failure mode and is rejected at boot. (The operator did not take
  the "more questions" slot on this cross-check; it is Phase 40 D-07's named deferred idea and the
  recommended disposition is recorded here as a decision, not discretion, because a silent miss is
  a security-shaped failure.)

### Refusal contract

- **D-12:** The wire status for an exhausted allowance is **`429 Too Many Requests`**, with a
  **`Retry-After`** header carrying the whole seconds from `store_now()` until `window_end` for a
  window ceiling and **omitted** for a lifetime ceiling. The body `code` is
  **`allowance_exhausted`** — never the existing per-key rate limiter's `too_many_requests` — so
  a client can tell quota from pacing; `ApiError::new(StatusCode::TOO_MANY_REQUESTS,
  "allowance_exhausted", ..)` plus the header is one shared `paladin-web` helper used by the run
  and agent controllers alike. — **Reversibility:** costly — it becomes the documented HTTP
  contract (§9.6, `platform-api.md`) and SDK smoke tests may assert on it.
- **D-13:** The body's `details` object carries the figures, nothing more: `{ "scope":
  "api_key" | "tenant", "kind": "window" | "lifetime", "balance": "<format_cost>", "ceiling":
  "<format_cost>", "window_start": <RFC 3339> | null, "window_end": <RFC 3339> | null }`.
  Balance and ceiling use `format_cost` (`"25.0000 USD"`); nano-unit integers are not exposed on
  this error (the `CostDto` precedent of 39-06 is for *reports*, this is a refusal). The caller
  already knows its own key name and tenant, and neither is repeated in the body.
- **D-14:** **One core refusal value, wrapped per port.** `AllowanceRefusal { scope_kind,
  limit_kind, balance: Cost, ceiling: Cost, window: Option<(DateTime<Utc>, DateTime<Utc>)> }`
  lives in `paladin-core` beside `treasury_ledger.rs` (pure, serde-derived, `Debug`/`Clone`/
  `PartialEq`, `Display` rendering the D-13 sentence). `RunSubmissionError` gains
  `AllowanceExhausted(AllowanceRefusal)` (additive under `#[non_exhaustive]`; §9.2 row marked
  `N`); `map_submission_error` in `run_controller.rs` and the agent handlers both go through the
  D-12 helper; the D-19 trace event and the D-17 webhook payload carry the same value, so the
  refusal, the warning, the trace and the webhook all speak one shape.

### Warn notice delivery & dedup

- **D-15:** The threshold is **one `warn_at` integer percent**: a global
  `treasurer.allowance.warn_at` (default **80**), overridable per tenant/key entry, `0` disables
  for that entry. A crossing is evaluated at **every admission** — the only Phase 41 evaluation
  point — for every applicable ceiling, in nano-units with checked `i128` arithmetic
  (`balance * 100 >= ceiling * warn_at`), never `f64`. Values above `100` are rejected at
  `validate()`. The pre-admission balance is used (there is no hold, D-05); a crossing never
  blocks or delays the admitted run.
- **D-16:** **Once per window, enforced by the store.** A new migration pair
  `011_create_treasury_notices.sql` creates `treasury_notices (notice_id, scope_kind, tenant_id,
  api_key_id, limit_kind, window_start, ceiling_nanos, currency, balance_nanos, run_id NULL,
  recorded_at, schema_version)` with a `UNIQUE` index on `(scope_kind, tenant_id, api_key_id,
  limit_kind, window_start, ceiling_nanos)` and `INSERT … ON CONFLICT DO NOTHING` — the same
  store-enforced idempotency shape as the settle index (ADR-0053 §4, 39 D-06) and the webhook
  `claim`. Zero rows affected means another replica (or an earlier admission) already notified
  this window: emit nothing. Including `ceiling_nanos` in the key means an operator who raises an
  allowance re-arms the notice for the same window; a lifetime ceiling (`window_start` = epoch)
  therefore notifies once per configured cap value. The write goes through the same backend
  selection as the ledger (`build_treasury_ledger` / `RunStoreConfig`), on all three adapters,
  with contract clauses (first insert wins, duplicate is `AlreadyRecorded`, raised ceiling
  re-arms). Whether it is a method on `TreasuryLedgerPort` or a sibling port is the planner's
  call. — **Reversibility:** one-way — a persisted table on both backends.
- **D-17:** The webhook notice goes to an **operator-level target**,
  `treasurer.allowance.webhook { url, secret: Option<String> }`, delivered through the
  **existing** durable machinery — a `webhook_deliveries` row, HMAC-SHA256 over the exact stored
  bytes (`X-Paladin-Signature`), the no-redirect client, bounded retries by
  `WebhookDeliveryService` — never a second HTTP client. The URL passes the SSRF guard at wiring
  time (`build_run_api`, fail closed with the key path in the message) and at send time (the
  service's existing check). The payload is a new shape beside `WebhookPayload`: `{ "event":
  "allowance_warning", "scope", "kind", "balance", "ceiling", "window_start", "window_end",
  "warn_at", "run_id": <admitting run> | null, "timestamp" }` — the D-13 figures plus the
  threshold and the admitting run's id (known before insert because `RunId::new_v7()` is
  generated first; `null` on the agent execute path). No run input, no key value, no secret. The
  `secret` field reuses `WebhookSpec`'s redacting `Debug` discipline. How a non-run-lifecycle
  delivery fits the `webhook_deliveries` row (`run_id` nullable vs the admitting run's id; a
  distinct `event` discriminator) is the planner's call under the constraint that
  `GET /runs/{id}/webhook-deliveries` for the admitting run does **not** list operator notices
  (they are not the caller's). Omitting `webhook:` disables only the webhook leg; trace and
  herald still fire. — **Reversibility:** costly — the payload key set is a documented contract
  (`platform-api.md` Webhooks section) and a prohibition boundary like `WebhookPayload`'s.
- **D-18:** The trace and herald legs are **one new `TraceEvent::AllowanceWarning { scope_kind,
  limit_kind, balance, ceiling, window_start, window_end, warn_at }`** (additive under
  `#[non_exhaustive]`, `#[serde(default)]` discipline, N/A-registered per D-00e) emitted exactly
  once on the **admitted run's** trace stream, so `thread_id`/`run_id`/`seq` envelope it like
  any other event; `HeraldTraceSink` folds it into `ExecutionMetadata.metadata` under one key
  (`treasurer.allowance_warning`), and the markdown, JSON and table heralds each render **one
  line** from it (`⚠ allowance: 82% of 25.0000 USD (api_key, window resets
  2026-10-03T00:00:00Z)`), the way all three render cost through `cost_display()` (Phase 38
  D-11/38-05). On the HTTP agent execute path the same event is emitted on the
  `PaladinExecutionService` trace (the `MiddlewareEvent` precedent) and reaches that call's
  `ExecutionMetadata` the same way. The event is emitted only by the replica that won the D-16
  insert, so "exactly one trace event plus a herald and webhook notice per window" is one
  emission observed three ways.

### Bookkeeping, ADR and docs

- **D-19:** One ADR, **ADR-0056 "Allowance admission model: tumbling UTC windows, check-only
  admission, every-limit composition, store-deduped notices"**, recording D-01, D-03, D-05, D-09,
  D-10 and D-16 with the rejected alternatives from the discussion log; `PROMOTION.md` advances
  to 0057 in the same commit. Phase 42 cites it beside ADR-0052/0053 rather than re-opening
  admission.
- **D-20:** Registers and docs land in the same commits as the code: `MIGRATION.md` §9.2
  (`TreasuryLedgerPort` method `Y` + allowlist; `RunSubmissionError`/`TraceEvent` variants
  `N`/N/A; `RunSchedule` field additive), §9.4 (migrations `010`, `011`), §9.5 (the new
  `treasurer.allowance` keys — additive, no break), §9.6 (`429 allowance_exhausted` +
  `Retry-After`, the operator webhook payload, schedule attribution); `config.example.yml`;
  `docs/src/getting-started/configuration.md`; `docs/src/api-reference/platform-api.md`
  (errors, webhooks, authentication and scopes); `docs/src/deployment-topologies/
  http-service-host.md`; root `CHANGELOG.md` `[Unreleased]` (Added + Changed); `WINDOWS.md`
  row for pre-existing schedules without `created_by` (D-08); `make api-surface-update` for the
  new public items. The Treasurer mdBook page itself stays Phase 46 (CURR-23).

### Claude's Discretion

- Exact type and module names (`Treasurer`, `AllowanceConfig`, `AllowanceEntry`,
  `BalanceQuery`, `AllowanceRefusal`, `NoticeOutcome`, `AllowanceWarning`) and whether the core
  types share `treasury_ledger.rs` or a new `allowance.rs`; the names above are defaults the
  planner may tighten.
- The deterministic evaluation order of the four ceilings (D-03) and whether all four balance
  reads happen or evaluation short-circuits at the first refusal (recommended: short-circuit, but
  evaluate every warn crossing when admitted so no notice is skipped).
- Whether `balance` is a required trait method (recommended; one §9.2 row, all in-tree impls are
  ours) or a defaulted one that returns `InvalidRequest` for external implementors.
- `period` grammar edges: whether `s` is accepted for tests, the maximum period, and whether the
  epoch-alignment of a period that is not a divisor of a day (e.g. `7h`) is simply accepted (it
  is well-defined) or warned about in `validate()`.
- The emission mechanics for D-18 on the submit path: whether the trace event is emitted at
  admission through the run-scoped emitter keyed on the just-generated `RunId`, or the winning
  notices row is read back by the worker at run start and emitted there — either is acceptable
  provided it lands on the run's own stream exactly once and survives a crash between admission
  and dispatch (the notices row is the durable record either way).
- How the schedule attribution travels through `SubmitRun` (D-08) and the exact `010` column
  names; whether `create_schedule` by a principal-less caller stores `NULL` or is rejected.
- Whether a refused admission emits a `warn` log line (recommended: yes, naming scope kind,
  tenant id and the figures, never the key value) and whether it is also a trace event
  (recommended: no — there is no run to attach it to).
- Whether the D-16 notices write and the D-17 webhook enqueue share a transaction, or the
  webhook enqueue follows the winning insert best-effort (recommended: the insert is the
  truth; a failed enqueue after a won insert is logged at `error` and not retried, mirroring
  39 D-08's posture for the non-authoritative leg).
- Test topology: contract-suite placement for the new ledger clauses, how the in-memory adapter
  simulates `store_now()` for window-boundary tests, and the HTTP end-to-end proof shape
  (`http_surface_tests.rs` precedent) for `429` on `POST /runs` and `/agents/{id}/execute`.
- Migration file naming beyond the `NNN_verb_noun.sql` convention and whether `010`/`011` are
  ordered schedule-first or notices-first.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked design (cite, do not re-open)
- `.planning/decisions/0053-ledger-balance-model.md` — append-only, derive-on-read balance;
  signed contributions; window attribution by the reservation's instant; nano-units; the SUM
  this phase's `balance` method computes.
- `.planning/decisions/0052-mid-run-treasurer-enforcement.md` — where Phase 42 reserves and
  halts; this phase must not pre-empt either attachment point.
- `.planning/decisions/0050-treasurer-reservation.md` — the two-officer model, the
  framework-only word, the dated note releasing the name for Milestone 14 code.
- `.planning/decisions/0054-tenant-scoped-run-reads.md` — server-derived tenant, Admin read
  bypass (a read rule only; D-09 here is independent).
- `.planning/decisions/PROMOTION.md` — next free ADR number **0056** (D-19).
- `.planning/phases/39-spend-ledger/39-CONTEXT.md` — D-01 (schema/port final), D-03 (policy-free
  `reserve`, `Refused` figures), D-04 (`store_now`), D-06 (store-enforced idempotency — the
  D-16 precedent), D-08 (settle-only writer; what admission must not disturb), D-09 (CLI reads
  the store via `RunStoreConfig`).
- `.planning/phases/40-tenant-identity-run-read-scoping/40-CONTEXT.md` — D-04 (`PrincipalRef`),
  D-05/D-06 (key→tenant mapping and `AuthConfig::validate` — the D-11 cross-check target),
  D-07 (tenant registry deferred), D-10 (schedule-fired runs — closed by D-08 here), D-11/D-12
  (read scope), D-16 (agent execute path attribution — why D-07 gates it).
- `.planning/phases/38-design-seams-pricing-cost-producer/38-CONTEXT.md` — D-00 house rules,
  D-01 (decimal-string grammar), D-02 (nano-units), D-04 (currency rendering), D-07
  (`treasurer:` section), D-11 (herald surfaces).

### Milestone scope and requirements
- `.planning/ROADMAP.md` §"Phase 41" — goal and success criteria 1-3; §"Phase 42" — what is
  explicitly not here (ALLOW-03, ALLOW-05, PLAT-09).
- `.planning/REQUIREMENTS.md` — ALLOW-01, ALLOW-02, ALLOW-04 (this phase); ALLOW-03, ALLOW-05
  (boundary); TENANT-01/02, PLAT-07 (identity this phase consumes); FUT-05, FUT-12.
- `.planning/research/SUMMARY.md` §"Phase 41" and Pitfalls 1, 2, 4 (f64 money, check-then-act,
  clock skew) — the integration point ("structurally identical to the existing SSRF guard") and
  the `AllowanceExhausted`-style variant.
- `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` R3 (the
  `allowance` key, distinct from the four `max_tokens` meanings), §5 (allowance refusal test).
- `.project/Milestone_14-Treasurer/overview/Milestone-14_Treasurer.md` §3 — vocabulary guardrail.

### Breaking-change register and conventions
- `MIGRATION.md` §9.2, §9.4, §9.5, §9.6 — row columns and the set-equality contract.
- `.cargo/semver-checks-allowlist.toml` — entry schema (D-04 needs one entry).
- `crates/paladin-ports/Cargo.toml`, `crates/paladin-core/Cargo.toml`,
  `crates/paladin-web/Cargo.toml` `[package.metadata.cargo-semver-checks.lints]` — which lints are
  already crate-wide allowed (39-08 D-27 diagnostic).
- `.planning/WINDOWS.md` — row format for the D-08 pre-existing-schedules gap.

### Code the phase extends (read, do not re-derive)
- `src/config/treasurer.rs` — `TreasurerConfig`, `parse_price_nanos_per_million` (the grammar
  D-02 reuses), the module docs promising `allowance` under this section.
- `src/config/agents.rs` — `ApiKeyConfig { key, name, role, tenant }`, `AuthConfig::validate`
  (the D-11 cross-check's data source); `src/config/run_store.rs` — `RunStoreBackend`
  (`Disabled` ⇒ D-11 boot error); `src/config/settings.rs` — where `treasurer` hangs.
- `crates/paladin-ports/src/output/treasury_ledger_port.rs` — `TreasuryLedgerPort`,
  `TreasuryLedgerError::{Refused, CurrencyMismatch, Backend}`, the rustdoc mock to extend.
- `crates/paladin-core/src/platform/container/treasury_ledger.rs` — `LedgerScope`,
  `ReserveRequest` (the window-bounds shape `BalanceQuery` mirrors), `SpendQuery`, `format_cost`.
- `crates/paladin-core/src/platform/container/cost.rs` — `Cost`, `CurrencyCode`.
- `crates/paladin-core/src/platform/container/principal.rs` — `TenantId`, `PrincipalRef`,
  `RunAttribution` (D-08 persists one on the schedule row).
- `crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,postgres,contract_tests}.rs` and
  `crates/paladin-storage/migrations/{sqlite,postgres}/007_create_treasury_ledger_table.sql` —
  the balance SUM (`amount_nanos`, `attributed_at`), the per-scope index, the settle `ON
  CONFLICT` idempotency precedent, header style for `010`/`011`.
- `crates/paladin-storage/migrations/{sqlite,postgres}/004_create_run_schedules_table.sql`,
  `009_add_run_attribution_check.sql` — the schedule table D-08 alters; the latest number.
- `crates/paladin-core/src/platform/container/run_schedule.rs` — `RunSchedule` (gains
  `created_by`); `crates/paladin-storage/src/schedule/` adapters and contract suite.
- `src/application/services/run/submission.rs` — `submit` (SSRF guard → resolve →
  `authorize_invocation` → thread check → `Run::new` → insert → enqueue: D-06's slot),
  `fork`, `with_ssrf_guard` (the builder shape `with_treasurer` mirrors).
- `src/application/services/run/schedule/service.rs` — the fire site building `SubmitRun {
  requested_by: None, .. }` (D-08).
- `crates/paladin-web/src/schedule_controller.rs` — `create_schedule` with `Principal` (D-08
  stamp); `crates/paladin-web/src/run_controller.rs` — `map_submission_error`, `submit_run`,
  `fork`; `crates/paladin-web/src/agent_controller.rs` — `execute_agent`,
  `execute_agent_stream`, `AgentApiState` (D-06/D-07 injection site); `crates/paladin-web/src/
  error.rs` — `ApiError::new`, `too_many_requests` (the `code` D-12 must not reuse),
  `with_details`; `crates/paladin-web/src/http_layers.rs` — the existing 429 rate limiter.
- `crates/paladin-web/src/agent_auth.rs` — `Principal`, `ledger_scope()`, `read_scope()` (the
  one-function-per-rule home for a `principal_ref()` if needed).
- `crates/paladin-ports/src/input/run_submission_port.rs` — `SubmitRun`, `ForkRun`,
  `RunSubmissionError` (`#[non_exhaustive]`; D-14 variant).
- `src/application/services/run/webhook/{mod,service,ssrf,signature,client}.rs` —
  `WebhookPayload` (the key-set discipline D-17 copies), `WebhookDeliveryService`, `SsrfGuard`,
  `sign_webhook_body`; `crates/paladin-storage/migrations/{sqlite,postgres}/
  005_create_webhook_deliveries_table.sql` — the row D-17 reuses.
- `src/infrastructure/web/run_api_wiring.rs` — `build_treasury_ledger`, `build_run_api` (where
  the `Treasurer` is built from `TreasurerConfig` + the ledger and handed to the submission
  service, the agent state and the webhook URL check); `src/bin/paladin-server.rs` —
  `build_auth_config` (fail-closed voice).
- `crates/paladin-core/src/platform/container/trace.rs` — `TraceEvent` (`#[non_exhaustive]`,
  `MiddlewareEvent` precedent), `TraceRecord`; `src/infrastructure/telemetry/herald_sink.rs` —
  `HeraldTraceSink` (D-18 fold); `crates/paladin-core/src/platform/container/herald.rs` —
  `ExecutionMetadata.metadata`, `cost_display`; `crates/paladin-herald/src/{markdown,json,table}
  _herald.rs` — the three renderers.
- `src/application/services/paladin/paladin_execution_service.rs` — the agent-loop trace
  emission site (D-18 agent path).
- `src/application/services/run/http_surface_tests.rs`, `crates/paladin-web/tests/
  openapi_golden_v0_9.rs` — end-to-end and golden-diff precedents.
- `config.example.yml`, `docs/src/getting-started/configuration.md`,
  `docs/src/api-reference/platform-api.md`, `docs/src/deployment-topologies/http-service-host.md`,
  `CHANGELOG.md` `[Unreleased]` — D-20 targets.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TreasuryLedgerPort::store_now()` — the store clock already exists on all three adapters;
  D-01's window function is pure arithmetic on its result.
- `parse_price_nanos_per_million` in `src/config/treasurer.rs` — the exact-integer decimal
  grammar; D-02's amount parser is the same function with a `1e9` scale (factor it, don't fork).
- The ledger `007` schema already indexes `(tenant_id, api_key_id, attributed_at)` and stores
  signed `amount_nanos`; D-04's `balance` is one `SUM` with or without the key predicate.
- `INSERT … ON CONFLICT DO NOTHING` + partial/unique index precedent (`007` settle index,
  `webhook_deliveries` claim, `run_traces (thread_id, seq)`) — D-16's dedup is the same shape.
- `SsrfGuard`, `sign_webhook_body`, `build_webhook_client`, `WebhookDeliveryService` — D-17
  adds a payload and a config key, not a transport.
- `RunSubmissionService::with_ssrf_guard` / the write-time SSRF check at the top of `submit` —
  the exact builder and placement shape for `with_treasurer` and the admission call.
- `AuthConfig::validate()` (Phase 40 D-06) — already iterates every key name and tenant; D-11's
  cross-check reads the same lists.
- `RunId::new_v7()` is generated before insert in `submit` — the admitting run's id is available
  to the notice without reordering the function.
- `HeraldTraceSink` + `ExecutionMetadata.metadata` + `cost_display()` across all three heralds
  (38-05) — D-18's one-line render has a worked precedent per renderer.
- `ApiError::new(status, code, message).with_details(json)` — D-12/D-13 need no new error
  plumbing, only a helper and a header.
- Schedule adapters' contract suite and `RunSchedule`'s builder — D-08's column is the Phase 40
  `Run.submitted_by` pattern (`008`) replayed on `run_schedules`.

### Established Patterns
- **Policy-free ports, policy in the application layer:** the ledger takes ceilings and
  windows; the `Treasurer` service computes them. `paladin-battalion` and `paladin-storage` never
  learn about allowances.
- **Fail-closed config with the offending path in the message** (`http.auth.api_keys[<name>]:
  'tenant' is required …`); D-02/D-11 keep that voice.
- **Store-enforced idempotency over application-level "have I done this?" checks** (39 D-06).
- **404/429 bodies never echo credentials**; key *names* are already treated as log-safe ids.
- **Additive `#[serde(default)]` fields and `#[non_exhaustive]` enums** keep Rust-side changes
  non-breaking; the register still records them.
- **Trace sinks are lossy; money is not** (39 D-08) — the notices row is the durable truth, the
  trace event and webhook are observations of it.
- **One shared function per rule** (`load_visible_run`, `RunReadScope`, `LedgerScope::from_
  attribution`): D-06 is the allowance rule's single home.

### Integration Points
- `build_run_api` → `Treasurer::new(allowance_config, ledger, notices, webhook)` →
  `RunSubmissionService::with_treasurer`, `AgentApiState.treasurer`, schedule service (unchanged
  — it submits through the same `RunSubmissionService`).
- `submit`/`fork` → `treasurer.admit(..)` → `Err(AllowanceRefusal)` →
  `RunSubmissionError::AllowanceExhausted` → `map_submission_error` → `429` + `Retry-After`.
- `execute_agent`/`execute_agent_stream` → `treasurer.admit(..)` → `429` via the same helper.
- `admit` (warn crossing) → `treasury_notices` insert (D-16) → on win: `TraceEvent::
  AllowanceWarning` on the run's stream + `webhook_deliveries` enqueue (D-17).
- `create_schedule` → `RunSchedule.created_by` → `010` columns → fire site → `SubmitRun`
  attribution → `Run.submitted_by` + admission.
- `TreasurerConfig::validate` → `AllowanceConfig::validate` (+ cross-check against
  `AuthConfig` and `RunStoreConfig` at the composition root, D-11).
- `HeraldTraceSink` → `ExecutionMetadata.metadata["treasurer.allowance_warning"]` → three
  heralds.

</code_context>

<specifics>
## Specific Ideas

- An operator should be able to write `treasurer.allowance.api_keys.ci-runner: { period: "1h",
  amount: "2.50" }`, submit runs until the ledger shows `2.5000 USD` for that key in the current
  hour, and see the next `POST /runs` answer `429 allowance_exhausted` with `Retry-After` equal to
  the seconds left in that hour and a body naming `balance`, `ceiling`, `window_start` and
  `window_end` — with `paladin-cli treasury spend --api-key ci-runner --since <window_start>`
  agreeing with the figure.
- The phase's first red tests: (a) a contract clause proving `balance(tenant, None)` equals the
  sum of `balance(tenant, Some(k))` over its keys on all three adapters; (b) a submission test
  that a principal at `ceiling` is refused and the run store and queue are untouched (the
  `submit_with_a_loopback_webhook_url_is_rejected_and_touches_nothing` shape); (c) a notices
  clause where sixteen concurrent crossings of the same window yield exactly one `Recorded`.
- The herald line reads like the cost line it sits beside: `⚠ allowance: 82% of 25.0000 USD
  (api_key, window resets 2026-10-03T00:00:00Z)`.
- `Retry-After` is omitted, not `0`, for a lifetime refusal — a client must not spin.
- `WINDOWS.md` gets one row this phase (pre-existing schedules without `created_by`), filed
  through the tool, not by hand.

</specifics>

<deferred>
## Deferred Ideas

- **Warn-threshold ladder** (`warn_at: [50, 80, 95]`) — a later enhancement; D-15/D-16's key
  would gain a rung component.
- **Notice on the admitted run's own caller webhook** (`RunEventKind::AllowanceWarning`) — the
  tenant-facing twin of D-17; separate payload shape to document.
- **`paladin-cli treasury allowance` view** of remaining allowance per tenant/key for the
  current window — operator surface; reads the same `balance` method.
- **Mid-run hold and halt, `TokenBudget` derivation from remaining allowance, SSE `done`
  status** — Phase 42 (ALLOW-03, ALLOW-05, PLAT-09).
- **Rate pacing** — Phase 43 (PACE-01..05).
- **`PATCH /schedules/{id}` re-assigning `created_by`**, and backfilling `created_by` on
  pre-existing schedules — out of scope; the `WINDOWS.md` row names the gap.
- **Tenant registry** beyond the key mapping, and per-API-key read narrowing — FUT-05 (v2).
- **Thread-route read scoping** (`/threads/*`) — still the Phase 40 deferred item; untouched.
- **Multi-currency / FX** — FUT-12.
- **Treasurer mdBook page** — Phase 46 (CURR-23).

### Reviewed Todos (not folded)
- *Verify local `make coverage` reproduces CI's figure*
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`;
  `todo.match-phase` score 0.2 on the keyword "local" only, below the 0.4 auto-fold threshold).
  User-owned, Docker-capable-machine task unrelated to allowances; same disposition as Phases
  38-40 and 45.

</deferred>

---

*Phase: 41-admission-time-allowance-enforcement*
*Context gathered: 2026-10-02*
