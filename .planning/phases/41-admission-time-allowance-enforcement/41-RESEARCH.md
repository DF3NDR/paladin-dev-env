# Phase 41: Admission-Time Allowance Enforcement - Research

**Researched:** 2026-10-02
**Domain:** Rust hexagonal service work (config grammar, ledger read method, store-deduped notices, admission gate in `RunSubmissionService` and the HTTP agent routes, `429` contract, trace/herald/webhook notice legs)
**Confidence:** HIGH for every codebase fact (all read directly this session); MEDIUM for the recommended designs where CONTEXT.md leaves a planner call or where a CONTEXT.md assumption did not survive verification.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

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

### Deferred Ideas (OUT OF SCOPE)

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
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ALLOW-01 | Operator can set an `allowance` per tenant and per API key with a rolling period and an optional lifetime cap, distinct from every `max_tokens` meaning; window boundaries computed in UTC from the store or server clock, never a worker's local clock | `AllowanceConfig` grammar reuses the exact-integer decimal parser already in `src/config/treasurer.rs` (it already scales by 1e9, so `"25.00"` is `25_000_000_000`); window math is pure integer arithmetic on `TreasuryLedgerPort::store_now()` (§Code Examples 1); the fake-ledger test strategy that proves the store clock, not `Utc::now()`, is used (§Validation Architecture); `treasurer.allowance` boot validation and the D-11 cross-check data sources (§Architecture Patterns, Pattern 6) |
| ALLOW-02 | Submitting a run while the caller's tenant or API-key allowance is exhausted is refused at admission with a typed error and no run is persisted | Single admission slot in `RunSubmissionService::submit`/`fork` (verified line order in §Architecture Patterns); new `AllowanceAdmissionPort` required because `paladin-web` cannot name the facade's `Treasurer` (§CONTEXT Corrections C1); `429` + `Retry-After` needs `ApiError` header support (§C8); four principal-bearing agent paths, not three (§C4); OpenAPI v0.9 golden exception (§C9) |
| ALLOW-04 | A configurable warn threshold emits one trace event plus a herald and webhook notice per window, without blocking the run | Store-enforced once-per-window `treasury_notices` table (NULL-in-UNIQUE trap, §Pitfall 3); claim-before-insert ordering and worker-side readback so the trace event lands on the run's own stream without a `seq` collision (§C6, Pattern 4); operator webhook delivery through the existing machinery needs a signing-secret branch and an event discriminator (§C3, Pattern 5); stateful `HeraldTraceSink` fold and a shared `allowance_warning_display()` line (§Pattern 7) |
</phase_requirements>

## Summary

Phase 41 is mostly composition of machinery that already exists and is well-patterned: the ledger port and its three adapters, the contract-suite style, the SSRF-guard-shaped admission slot in `submit`, the config-struct idiom, the durable webhook queue and the trace/herald fold. The new logic is small: window arithmetic, a four-ceiling evaluation loop, a `balance` SUM, a notices claim, and a `429` mapping. The risk is not the algorithm; it is five places where CONTEXT.md's wording does not match the code as it stands. Reading the sources turned up the following, each detailed in §CONTEXT Corrections: `paladin-web` cannot depend on the facade `Treasurer` (crate layering), so the agent handlers need a port in `paladin-ports`; the operator webhook cannot reuse the run-webhook signing path as-is (the secret is read from the run row, and `webhook_deliveries.run_id`/`event` are non-null and typed to run lifecycle); a `StandaloneEmitter`-style trace emission from the submit process would collide with the engine's `seq` counter in `run_traces (thread_id, seq)`, so the trace event must be emitted by the worker; a `UNIQUE` index over a nullable `api_key_id` does not dedup tenant-scope notices on either backend; and `POST /agents/{id}/jobs` is a fourth principal-bearing spend path D-06/D-07 do not list.

The existing infrastructure carries the load. The ledger `007` schema already indexes `(tenant_id, api_key_id, attributed_at)` and stores signed `amount_nanos`; `balance` is one `SUM` with or without the key predicate, and the existing `reserve` path already contains the exact currency-probe-then-SUM pattern to copy. The in-memory ledger has no injectable clock (`store_now` is `Utc::now()` truncated), so window-boundary behaviour must be proven against a scripted ledger test double at the `Treasurer` level, while the adapters' half-open edge is proven with the sleep-bracketing technique the existing `spend_window_is_half_open` clause already uses. There are no new external dependencies.

**Primary recommendation:** Build `Treasurer` as a facade service implementing a new `AllowanceAdmissionPort` (input port in `paladin-ports`), take `&RunAttribution` rather than `PrincipalRef` so schedule-fired runs can be gated without changing the invocation role check, claim notices *before* the run insert (with a best-effort abandon on insert failure), emit the trace event from the worker by reading notices by `run_id` on the `Queued` first-dispatch arm only, and treat the five CONTEXT corrections as planner decisions to confirm at a blocking checkpoint before migrations `010`/`011` (one-way) are written.

## Project Constraints (from CLAUDE.md)

Extracted from `/home/user/paladin-dev-env/CLAUDE.md` and its imported `.github/` instruction files. [VERIFIED: system-reminder project instructions]

- **TDD (Red-Green-Refactor).** Write the failing test first. Coverage floor: **82% workspace line coverage** (ADR-0006), gated by `cargo llvm-cov --fail-under-lines` in CI's `coverage` job. All public APIs need doc tests.
- **Dependencies flow inward only.** core -> nothing; application/ports -> core; adapters -> core + ports. Never import infrastructure from core or application.
- **Ubiquitous language.** Medieval Military terms (Paladin, Battalion, Garrison, Arsenal, Citadel, Herald, Quest, Commissary, ...) in code, docs and comments. `Treasurer` is a framework-only word (ADR-0050); "allowance", "window", "tenant", "API key", "notice" stay plain words.
- **Before committing a parent task:** `cargo test` -> `cargo fmt --check` -> `cargo clippy` -> `make api-surface` (refresh with `make api-surface-update` plus a CHANGELOG entry for an intentional surface change), conventional-commit message. Stop after each major task and wait for go-ahead.
- **Security.** Run `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings` on new code, plus the manual credential-handling review in `security.instructions.md`. CodeQL is advisory-only; **do not reintroduce Snyk or record a phase as blocked on it.** No log line, error body, trace event, webhook payload or herald line carries an API key value; response bodies are redacted before truncation; HTTP clients carrying a credential header never follow redirects; no config type carrying a secret is `Debug`-formatted or serialised outward.
- **No `unwrap()`/`expect()`/`panic!` in library code**; return `Result`. Borrow over clone; keep iterators lazy.
- **Error enums use `thiserror`**, layer-specific, converted at boundaries with `From`. Ports are `Send + Sync`. Builders for complex construction. `Node<T>` pattern for entities.
- **Quality bar:** every public item has rustdoc with examples; `cargo clippy` clean; `cargo fmt`; `make clean-code` (fmt + lint + lint-shell + check + doc-check + check-api-examples).
- **ADR rule:** next free number from `.planning/decisions/PROMOTION.md` (currently 0056); advance the line in the same commit. Required ADR headings, in order: `Status`, `Context`, `Decision`, `Considered Options` (bulleted), `Code Locations` (bulleted), `Code Conformance`, `Downstream Consumers`. [VERIFIED: PROMOTION.md lines 258-280]
- No project skills directory exists (`.claude/skills`, `.agents/skills` absent). [VERIFIED: ls]

## CONTEXT.md Corrections and Planner Decisions Required

These are facts verified in the source tree that either contradict CONTEXT.md wording or leave a gap the planner must close. None reopens a locked decision's intent; each changes *how* it is implemented. Recommend one blocking `checkpoint:decision` plan task covering C1, C3, C5, C6 and C11 before any one-way migration is written (the 39-01 precedent).

| # | Finding | Evidence | Recommended resolution |
|---|---------|----------|------------------------|
| C1 | **`paladin-web` cannot name `Treasurer`.** D-06 says the service is injected as `Option<Arc<Treasurer>>` on `AgentApiState`, but `Treasurer` lives in the facade crate (`src/application/services/treasurer/`) and `paladin-web` depends only on `paladin-ports` and `paladin-core`; the facade depends on `paladin-web`. A direct type reference is a dependency cycle. | `crates/paladin-web/Cargo.toml` `[dependencies]` lines 16-17; root `Cargo.toml` line 114 (`paladin-web` optional dep). `RunApiState.submission: Arc<dyn RunSubmissionPort>` is the established pattern. [VERIFIED: codebase] | Add an **input port** `AllowanceAdmissionPort` in `crates/paladin-ports/src/input/allowance_admission_port.rs`; the facade `Treasurer` implements it; `AgentApiState.treasurer: Option<Arc<dyn AllowanceAdmissionPort>>`; `RunSubmissionService::with_treasurer` takes the same trait object. Value types (`AllowanceRefusal`, `AllowanceWarning`, `Admission`) live in `paladin-core` (D-14). |
| C2 | **Migration `009` exists on Postgres only.** D-00i says it exists "on both backends". The SQLite directory runs `001`..`008`; `009_add_run_attribution_check.sql` is Postgres-only because SQLite cannot add a cross-column `CHECK` via `ALTER TABLE`. | `ls crates/paladin-storage/migrations/{sqlite,postgres}`; header of the Postgres `009` file. [VERIFIED: codebase] | Keep numbering aligned across backends: SQLite goes `008` -> `010` (a gap at `009`, documented in the `010` header). Whether `sqlx::migrate!` tolerates a non-contiguous version sequence is `[ASSUMED]` (docs.rs was unreachable); the first SQLite adapter test after adding `010` proves it, so make that a Wave 0 sanity check. Fallback if it fails: a no-op `009` on SQLite. |
| C3 | **Operator webhook does not fit the delivery row/service as-is.** (a) `webhook_deliveries.run_id` is `NOT NULL` and `thread_id` is `NOT NULL`; (b) `WebhookDelivery.event` is the closed `RunEventKind` enum and both adapters' `event_from_str` return an error on an unknown string, so one unparseable row would fail the whole `claim_due` batch for any replica that does not know the new value; (c) `WebhookDeliveryService::process` loads the signing key from `runs.get(delivery.run_id)` -> `run.webhook.secret`, so an operator notice would be signed with the *caller's* secret (or loop in `Retrying` forever if its `run_id` is not a real run); (d) `GET /runs/{id}/webhook-deliveries` lists by `run_id`. | `crates/paladin-storage/migrations/sqlite/005_create_webhook_deliveries_table.sql`; `crates/paladin-core/src/platform/container/webhook.rs`; `crates/paladin-storage/src/webhook/sqlite.rs` lines 64-82; `src/application/services/run/webhook/service.rs` `process`. [VERIFIED: codebase] | **Recommended (Option A):** add `RunEventKind::AllowanceWarning` and mark the enum `#[non_exhaustive]` in the same change (so only the already-allowed `enum_marked_non_exhaustive` lint fires, the `StopReason` precedent). Give operator rows a dedicated correlation `RunId` (a fresh UUIDv7, the notice's own id) as `delivery.run_id` and a fixed `ThreadId` such as `treasurer-notices`; the payload carries the real admitting `run_id`. No run owns that id, so `list_for_run` never returns it and **no adapter query changes**. Add `WebhookDeliveryService::with_operator_notice_secret(Option<String>)`; `process` branches on `event == AllowanceWarning` *before* the run lookup and signs with the configured operator secret (the secret stays off the row, prohibition P1). The caller-facing `parse_event_kind` in `run_controller.rs` and `schedule_controller.rs` already rejects unknown strings via its `other =>` arm, so a caller can never subscribe to the operator event and the OpenAPI schema (events are `Vec<String>` there) is unaffected. **Alternative (Option B):** a `kind` discriminator column (new migration) plus a `list_for_run` filter; heavier (three adapters + contract) and still needs an `event` value. Rollout caveat for either option: enable `allowance.webhook` only after every replica runs the new build (see Pitfall 12). |
| C4 | **A fourth spend path is ungated.** `POST /agents/{id}/jobs` (`enqueue_job`) builds `RunScope::default().with_ledger_scope(principal.ledger_scope())` and spawns `execute_scoped` exactly like `execute`/`execute/stream`. D-06/D-07 list only the latter two; D-07's own reasoning ("leaving it open would let a refused caller spend the same allowance") applies equally. | `crates/paladin-web/src/agent_controller.rs` lines 677-720. [VERIFIED: codebase] | Gate `enqueue_job` too, **synchronously before the spawn** (a refused job answers `429`, never a job id that later fails). The shared function is then called from four handler sites plus `submit`/`fork`. `POST /threads/{id}/resume` continues an already-admitted run and is mid-run (Phase 42), not an admission; `POST /threads/{id}/fork` already goes through `RunSubmissionService::fork`. |
| C5 | **`UNIQUE` over a nullable column does not dedup.** D-16's key includes `api_key_id`, which is `NULL` for tenant-scope notices. Verified on SQLite 3.45.1 this session: two `INSERT ... ON CONFLICT DO NOTHING` with a `NULL` in the unique column both report `rowcount 1`; with `''` the second reports `0`. PostgreSQL treats NULLs as distinct in unique indexes by default (`NULLS NOT DISTINCT` is PG15+ opt-in). | python `sqlite3` run; [CITED: enterprisedb.com/postgres-tutorials/postgresql-unique-constraint-null-allowing-only-one-null] | Make `api_key_id TEXT NOT NULL` with the documented sentinel `''` for tenant-scope rows (real key ids are non-empty by `TenantId`-style validation). Add a contract clause that two tenant-scope claims for the same window yield exactly one `Recorded`. |
| C6 | **A trace event emitted at admission collides on `seq`.** `run_traces` has `PRIMARY KEY (thread_id, seq)`; `seq` is stamped per `TraceDispatcher`/`StandaloneEmitter` from its own counter starting at 1. The engine's per-run dispatcher is built in the worker, and `RunStarted` is its `seq = 1`. A second emitter in the submit process would also stamp `seq = 1` for the same thread. The herald line additionally needs the event *inside the worker's sink* (`HeraldTraceSink` runs only there). | `crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql`; `trace_sink_port.rs` `StandaloneEmitter`; `worker.rs` `run_once` lines ~1019-1070. [VERIFIED: codebase] | Use the discretion option "read the winning notices row back at run start": admission writes the notice row with `run_id` set; the worker, on the `RunStatus::Queued` arm of `run_once` only (first dispatch, never `Running` redelivery or `AwaitingInput` resume), reads `notices_for_run(run_id)` and emits one `TraceEvent::AllowanceWarning` per row through the run's own emitter immediately before `start`. A `Queued -> Running` CAS (`update_status`) already guarantees one first dispatch. |
| C7 | **No `${VAR}` expansion exists in the config loader.** `Settings::load_from_file` is a plain `config::Config::builder().add_source(File::new(..))`; a grep of `src/` and `crates/` found no env-placeholder expansion. D-02's example `secret: "${ALLOWANCE_WEBHOOK_SECRET}"` would arrive as that literal string. (The same appears to apply to the existing `http.auth.api_keys[].key: "${PALADIN_API_KEY_CI}"` examples; if so the k8s configmap relies on something this research did not find.) | `src/config/settings.rs` `load_from_file`; `config.example.yml` line 161; `k8s/server/configmap.yaml` line 36. [VERIFIED: codebase grep] — that the `config` crate does not expand is `[ASSUMED]` | Do not rely on `${...}`. Give the webhook secret an explicit scalar env override (`APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`, applied in `EnvOverridable`) and document the config-file field as optional. Add a Wave 0 unit test that loads a YAML containing `${X}` and asserts the literal, so the behaviour is pinned rather than assumed. Raise the existing `api_keys` observation as an Open Question for the operator. |
| C8 | **`ApiError` cannot carry a header.** It renders `(status, Json(body))` only and `map_submission_error`/handlers return `ApiError`. `Retry-After` needs a new optional field on `ApiError` (private field, so additive). The value must come from the *store* clock: the refusal needs the instant it was evaluated at (`evaluated_at`), which D-14's field list omits. | `crates/paladin-web/src/error.rs`. [VERIFIED: codebase] | Add `ApiError::with_retry_after(secs: u64)`; `IntoResponse` inserts `Retry-After` when set. Add `evaluated_at: DateTime<Utc>` to `AllowanceRefusal`; `retry_after = ceil((window_end - evaluated_at).num_seconds()).max(1)`, `None` for a lifetime refusal. Never call `Utc::now()` in `paladin-web`. |
| C9 | **Adding `429` to the agent routes' `utoipa::path` annotations fails `openapi_v0_9_paths_match_the_frozen_baseline`.** That test compares the six `/v1/agents...` operation objects (including `responses`) byte-for-byte against the frozen `v0.9.0` document. Phases 31 and 39 added narrowly scoped, documented strip functions for schema divergences. | `crates/paladin-web/tests/openapi_golden_v0_9.rs` (`strip_known_v0_10_execute_response_divergence`, `openapi_v0_9_paths_match_the_frozen_baseline`). [VERIFIED: codebase] | Add the `429` response to the three agent operations (`execute`, `execute/stream`, `jobs`) and add a third sanctioned exception that removes `responses["429"]` from those operation objects in **both** documents before comparison, with a module-doc paragraph in the Phase 31/39 style. Regenerate `crates/paladin-web/openapi.json` with `make openapi`. The `/runs` and `/threads/{id}/fork` operations are not in the v0.9 set. |
| C10 | **`BalanceQuery` needs a currency and should use `Option` bounds.** D-04's field list omits the currency, but its own contract text says "empty scope -> zero in the requested currency" (a `Cost` needs a currency). A lifetime ceiling as a "far future" `window_end` risks `TIMESTAMPTZ`/chrono range edges; `SpendQuery.since/until` already model "unbounded" as `Option<DateTime<Utc>>`. | `treasury_ledger.rs` `SpendQuery`; `sqlite.rs` `BALANCE_QUERY`. [VERIFIED: codebase] | `BalanceQuery { tenant_id, api_key_id: Option<String>, currency: CurrencyCode, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>> }`; lifetime = both `None`. |
| C11 | **Register disposition for `TreasuryLedgerPort::balance` (D-04/D-20 say §9.2 `Y` + allowlist).** `treasury_ledger_port.rs` was first committed 2026-09-29 (shallow clone) / the migration header is dated 2026-09-27, after the v0.10.1 release (2026-09-20). The repo carries no git tags here, so "never published" is `[ASSUMED]`. The project's own policy (the `RunQuery` and `RunResponse` rows) marks additions to types that post-date the published baseline `N/A`, and the CI set-equality check requires a `Y` row and an allowlist entry to exist together. | `MIGRATION.md` lines 208-235; `.github/workflows/ci.yml` lines 396-482; `CHANGELOG.md` `[0.10.1] - 2026-09-20`. | Default to D-04 as written (a conservative `Y` row plus a `register-only` allowlist entry for `paladin-ports | TreasuryLedgerPort`, `requirement_id = "ALLOW-01"`) because it is operator-locked and harmless; have the plan record the honest justification ("type post-dates v0.10.1; recorded `Y` for external implementors of the port") and confirm with the operator at the checkpoint whether `N/A` is preferred. Note `paladin-ports` has no crate-wide `trait_method_added` allow line; none is needed because nothing fires against a baseline lacking the trait. |
| C12 | **D-08 vs D-11 on a removed creator key.** D-08 says a schedule creator key later removed from `http.auth.api_keys` "still attributes and gates by its persisted name ... an allowance keyed on that name keeps applying", but D-11 makes an `allowance.api_keys.<name>` entry that names a key absent from `http.auth.api_keys` a boot error. Both cannot hold. | CONTEXT D-08, D-11. | D-11 wins at boot (a locked, security-shaped rule). Document: removing a key requires removing its allowance entry; thereafter that key's surviving schedules are still *attributed* to `(tenant, key)` and gated by the **tenant** allowance only. Add a unit test for exactly that. |
| C13 | **`SubmitRun`/`ForkRun`/`CreateRunSchedule` are plain `pub` structs, not `#[non_exhaustive]`.** Adding a field (D-08's attribution for schedule-fired runs; `created_by` on creation) breaks struct-literal constructors in ~6 files and is a `Y` register row with a `constructible_struct_adds_field` allowlist entry (already crate-wide allowed in `paladin-ports/Cargo.toml`), following the Phase 40 `SubmitRun` precedent. | `run_submission_port.rs`; `schedule_admin_port.rs`; `grep "SubmitRun {"` = 6 files. [VERIFIED: codebase] | `SubmitRun.attributed_to: Option<RunAttribution>` (documented: used only when `requested_by` is `None`; never grants a role). Effective attribution in `submit` = `requested_by.attribution()` else `attributed_to`. Written decision required by D-08: **schedule-fired runs keep skipping `authorize_invocation`** (an Admin creator is not automatically in a restricted assistant's `allowed_roles`, so applying the check at fire time would silently break existing schedules). `CreateRunSchedule.created_by: Option<RunAttribution>` likewise. |
| C14 | **`build_run_api` early-returns when the run store is `Disabled`.** The D-11 "disabled + non-empty allowance" check must run before that return, and `AgentApiState` is built later in `paladin-server.rs::run()`, outside `build_run_api`. | `run_api_wiring.rs` lines 476-500; `paladin-server.rs` lines 200-235. [VERIFIED: codebase] | Build the `Treasurer` inside `build_run_api` from `settings.get_treasurer_config()`, `&configs.run_store`, and the already-passed `auth: AgentAuthConfig` (it carries `api_keys -> Principal{id, tenant_id}` and `bearer_tenant`, enough for the D-11 cross-check); run the disabled-store check at the top, before the early return. Return the handle as a new `RunApiHandles.treasurer` field and attach it to `AgentApiState` in `run()`. No `build_run_api` signature change (avoids a `function_parameter_count_changed` break). |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Allowance config grammar, validation, resolution to nano-units and seconds | `src/config/treasurer.rs` (facade config layer) | — | House idiom: `Default` + `validate()` + `EnvOverridable`, resolved once (the `price_table()` pattern) so the service never re-parses strings |
| Window arithmetic, four-ceiling evaluation, warn-crossing math | Application service (`Treasurer`, `src/application/services/treasurer/`) | — | Policy lives in the application layer; ports stay policy-free (ADR-0053, D-00a) |
| Tenant/key balance `SUM` over a window | Storage adapters behind `TreasuryLedgerPort::balance` | `paladin-ports` (trait), `paladin-core` (`BalanceQuery`) | Derive-on-read ledger; store does the aggregation, not the service |
| Once-per-window notice dedup | Storage (unique index + `ON CONFLICT DO NOTHING`) behind a `TreasuryNoticePort` | — | Store-enforced idempotency across replicas (39 D-06 precedent), never an application-level "have I notified?" check |
| Admission gate (check, claim, refuse) | `RunSubmissionService` (facade) and agent handlers (`paladin-web`) via `AllowanceAdmissionPort` | `Treasurer` | Structurally the SSRF guard's slot: before any run row is written |
| Wire contract (`429`, `Retry-After`, body) | `paladin-web` (`ApiError` + one shared helper) | — | HTTP concerns stay in the web crate; the core refusal value is transport-neutral |
| Trace event emission | Worker (`RunWorkerPool::run_once`/`run_agent`) | `PaladinExecutionService` for HTTP agent paths | The per-run `TraceDispatcher` owns `seq`; only the worker holds it |
| Herald line | `HeraldTraceSink` fold + three renderers (`paladin-herald`) via one core display helper | — | Mirrors `cost_display()` |
| Operator webhook delivery | Existing `WebhookDeliveryService` (facade) over `webhook_deliveries` | `Treasurer` (enqueue) | Reuse the durable queue, signing, no-redirect client, SSRF guard |
| Schedule creator attribution | Core (`RunSchedule.created_by`), schedule adapters + migration `010`, `create_schedule` stamp, fire site | — | Replays the Phase 40 `008` pattern on `run_schedules` |

## Standard Stack

All work is in-tree. **No new external crates are required.** Versions below are the workspace's current pins, read from the manifests. [VERIFIED: Cargo.toml, crates/*/Cargo.toml]

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `chrono` | 0.4.38 | `DateTime<Utc>`, `DateTime::from_timestamp`, `timestamp()` for epoch-aligned window math | Already the project's time type; ledger columns bind `DateTime<Utc>` |
| `sqlx` | 0.8 (sqlite, postgres, migrate, chrono, uuid, json) | Adapters and embedded migrations (`sqlx::migrate!`) | Existing adapters; `QueryBuilder` is the house pattern for dynamic filters |
| `async-trait` | workspace | New port traits | Existing port convention |
| `thiserror` | workspace | `TreasurerError`, `AdmissionError`, notice errors | House error idiom |
| `serde` / `serde_json` | workspace | `AllowanceRefusal`, `AllowanceWarning`, webhook payload | House value-type derives |
| `axum` | 0.8.4 | `Retry-After` header on the `429` response | Existing web stack |
| `tokio` | 1 (full) | `#[tokio::test]` incl. multi-thread flavor for the 16-way notice race | Existing test runtime |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serial_test` | existing dev-dep | Env-override tests (`APP_TREASURER_ALLOWANCE_WARN_AT`, secret) | Same as `treasurer.rs` `env_override_currency` |
| `tower` (`ServiceExt::oneshot`) | 0.5 (dev) | HTTP end-to-end proof | `http_surface_tests.rs` precedent |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `AllowanceAdmissionPort` | Put `Treasurer` in `paladin-web` | Violates ADR-0031 layering (policy in the web crate) and still needs ledger+notices+webhook ports there; rejected |
| `TreasuryNoticePort` sibling trait | Extra methods on `TreasuryLedgerPort` | Sibling adds no breaking change to the 8 existing implementors (3 adapters + 5 test doubles); costs a second `Arc` handle, avoided by implementing both traits on the same adapter struct and building one adapter, two `Arc`s |
| Required `balance` method | Defaulted method returning `InvalidRequest` | Defaulted avoids touching 5 test doubles but turns a forgotten override into a silent "every admission 500s"; D-04 recommends required; follow it and stub the doubles |
| Tumbling windows | Sliding/trailing, GCRA | Locked by D-01 (costly reversal); do not revisit |

**Installation:** none (no new packages).

**Version verification:** not applicable; every crate is already resolved in the workspace lockfile and none is added. `cargo` toolchain is pinned to `1.97.1` by `rust-toolchain.toml` and is installed. [VERIFIED: rustup show]

## Package Legitimacy Audit

No external packages are installed or upgraded by this phase, so the `package-legitimacy` seam was not run (nothing to check).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | — | — | — | — | — | No new dependency |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 caller (API key / bearer)                         schedule tick (any replica)
        |                                                    |
   authenticate() -> Principal{id,role,tenant}      RunSchedule.created_by -> SubmitRun
        |                                              {requested_by: None, attributed_to: Some}
        v                                                    |
 +--------------------+  POST /runs, fork            +-------v---------+
 | paladin-web        |----------------------------->| RunSubmission-  |
 | run/thread ctrl    |                              | Service         |
 +--------------------+                              |  1 SSRF guard   |
 | agent ctrl         |  execute / stream / jobs     |  2 resolve      |
 | (AgentApiState.    |------+                       |  3 authorize    |
 |  treasurer: port)  |      |                       |  4 thread vis.  |
 +--------------------+      |                       |  5 run_id=v7()  |
                             v                       |  6 ADMIT -------+--+
                  +-----------------------+          |  7 insert run   |  |
                  | AllowanceAdmissionPort|<---------+  8 enqueue run  |  |
                  |  admit / confirm /    |          |  9 confirm      |  |
                  |  abandon              |          +-----------------+  |
                  +----------+------------+                               |
                             | impl                                       |
                  +----------v------------+   ceilings for (tenant,key):  |
                  | Treasurer (facade)    |   key-window, key-life,       |
                  |  policy (resolved     |   tenant-window, tenant-life  |
                  |  nano/secs)           |                               |
                  +--+-------+--------+---+                               |
          store_now()|       |balance |record_notice (claim, run_id)      |
                     v       v        v                                   |
              +-------------------------------+   refuse: Err(AllowanceRefusal)
              | TreasuryLedgerPort            |   -> RunSubmissionError::AllowanceExhausted
              | TreasuryNoticePort            |   -> ApiError 429 + Retry-After (store clock)
              | (in_memory | sqlite | postgres)|
              +-------------------------------+
   confirm: for each WON notice -> webhook_deliveries row (operator event, operator secret)
   abandon: delete WON notices of this admission (run never persisted)

 worker (any replica)                                   herald / trace sinks
   run_once -> Queued arm only ---- notices_for_run(run_id) --> TraceEvent::AllowanceWarning
        |                                                    (run's own dispatcher, seq stamped)
        +-> HeraldTraceSink remembers it; at RunFinished folds
            metadata["treasurer.allowance_warning"] -> 3 heralds render ONE line
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
├── allowance.rs              # AllowanceScopeKind, AllowanceLimitKind, AllowanceRefusal, AllowanceWarning, Admission (new)
├── treasury_ledger.rs        # + BalanceQuery
├── trace.rs                  # + TraceEvent::AllowanceWarning
├── run.rs                    # RunEventKind (+ AllowanceWarning, #[non_exhaustive])
├── run_schedule.rs           # RunSchedule.created_by
└── herald.rs                 # ExecutionMetadata::allowance_warning_display()
crates/paladin-ports/src/
├── input/allowance_admission_port.rs   # AllowanceAdmissionPort + AdmissionError (new)
├── input/run_submission_port.rs        # SubmitRun.attributed_to, RunSubmissionError::AllowanceExhausted
├── input/schedule_admin_port.rs        # CreateRunSchedule.created_by
└── output/{treasury_ledger_port.rs (+balance), treasury_notice_port.rs (new)}
crates/paladin-storage/
├── migrations/{sqlite,postgres}/010_add_run_schedule_created_by.sql
├── migrations/{sqlite,postgres}/011_create_treasury_notices.sql
├── (optional) 012_add_treasury_ledger_tenant_index.sql
└── src/{treasury/{in_memory,sqlite,postgres,contract_tests}.rs, run_schedule/*, webhook/*}
src/
├── config/treasurer.rs                       # AllowanceConfig (+ resolve, validate, cross-check)
├── application/services/treasurer/{mod,policy,window,tests}.rs   # Treasurer (new)
├── application/services/run/{submission,worker,schedule/service,webhook/service}.rs
├── infrastructure/telemetry/herald_sink.rs   # stateful fold
└── infrastructure/web/run_api_wiring.rs      # build_treasurer, RunApiHandles.treasurer
crates/paladin-web/src/{error,run_controller,agent_controller,thread_controller,schedule_controller}.rs
crates/paladin-herald/src/{markdown,json,table}_herald.rs
```

### Pattern 1: Epoch-aligned tumbling window from the store clock
**What:** `window_start = floor(now/P)*P`, `window_end = window_start + P`, `now` from `ledger.store_now()` truncated to whole seconds; lifetime = unbounded `BalanceQuery`.
**When to use:** every window ceiling; one `store_now()` per admission, reused for every ceiling and for `Retry-After`.
**Example:** see Code Examples 1.

### Pattern 2: Short-circuit evaluation, evaluate every crossing when admitted
**What:** Evaluate ceilings in the deterministic order key-window -> key-lifetime -> tenant-window -> tenant-lifetime (D-03 recommended); the first with `balance >= ceiling` returns the refusal immediately. Warn crossings are collected only on the admitted path, across all four, so no notice is skipped. A scope with no entry is never read (no ledger touch); a principal with zero ceilings returns before `store_now()`.
**Why:** keeps a deployment without allowances working through a ledger outage (D-10) and bounds admission cost to one clock read plus at most four SUMs.

### Pattern 3: Claim before insert, abandon on failure
**What:** `admit` claims won notices (rows carry the admitting `run_id`) *before* the run row is inserted, so the worker can always find them; the submission service calls `confirm` after a successful insert+enqueue (enqueues the operator webhook for each won notice, best-effort) or `abandon` if insert/enqueue fails (deletes only this admission's won rows so the next admission re-wins). A crash between claim and insert can lose one window's notice (documented; the guarantee is **at most once per window, exactly once on the non-failure path**, never duplicated).
**Why:** claim-after-insert has a worker race (the worker can start and finish before the claim lands); claim-before-insert without abandon lets a `ThreadBusy` `409` at the crossing request consume the window's only notice. The agent handlers call `admit` then `confirm` back-to-back (no row to fail).

### Pattern 4: Worker-side trace emission on first dispatch
**What:** in `run_once`, capture `let first_dispatch = matches!(run.status, RunStatus::Queued);` before the status match; after the per-run emitter exists and before `start`/`fork`, if `first_dispatch`, read `notices_for_run(&run.run_id)` and emit one `TraceEvent::AllowanceWarning` per row. Do the same in `run_agent` after its dispatcher is built. With no composed sink the emitter is `None` and nothing is emitted (the durable legs are unaffected).
**Consequence:** the event is `seq = 1` ahead of `RunStarted` for runs that carry a notice; tests that assert `RunStarted` is `seq = 1` are unaffected because they carry no notice.

### Pattern 5: Operator webhook through the existing durable queue
**What:** `Treasurer::confirm` builds `WebhookDelivery::new(delivery_id, correlation_run_id, ThreadId("treasurer-notices"), RunEventKind::AllowanceWarning, url, payload_json, now)`; `payload_json` is serialized **once** and stored (D-41: signed and sent verbatim). `WebhookDeliveryService::process` branches on the event before the run lookup and signs with the operator secret held on the service. The SSRF guard already re-checks the URL at send time; the same `SsrfGuard` checks it at wiring time in `build_treasurer` (fail closed, naming `treasurer.allowance.webhook.url`).
**Payload (key set is a prohibition boundary, mirror `WebhookPayload`'s test):** `event, scope, kind, balance, ceiling, window_start, window_end, warn_at, run_id, timestamp` — add a `serializes_with_exactly_the_documented_keys` test.

### Pattern 6: Config resolution and the D-11 cross-check
**What:** `AllowanceConfig::resolve(&self, currency) -> Result<AllowancePolicy, String>` parses every string once; `validate()` is `resolve().map(|_| ())` (the `price_table()`/`validate()` pair). The cross-check is a separate pure function `AllowanceConfig::validate_against(&self, tenants: &BTreeSet<String>, key_names: &BTreeSet<String>, run_store_disabled: bool)` fed from `AgentAuthConfig` in `build_treasurer`, because `RunStoreConfig` and the auth config are not part of `Settings` and cannot be seen by `TreasurerConfig::validate()`.
**Edge cases to specify:** with `http.auth.enabled = false` the principal is `anonymous` in tenant `open-access` (`Principal::open_access`, a private fn today), so those two literals must be accepted as targets; bearer principals have dynamic ids, so only tenant-level allowances can govern them (an `api_keys.<subject>` entry cannot be validated at boot and is rejected).

### Pattern 7: Stateful herald fold with one shared display helper
**What:** `HeraldTraceSink` is currently stateless and acts only on `RunFinished`. Add interior state (`Mutex<Vec<AllowanceWarning>>`), record on `AllowanceWarning`, and at `RunFinished` add `metadata["treasurer.allowance_warning"]` (the serialized warning(s)). Put the one-line rendering in `ExecutionMetadata::allowance_warning_display() -> Option<String>` (next to `cost_display()`) so the markdown, JSON and table heralds all call one function.
**HTTP agent routes:** there is no run, no worker emitter and, for non-streaming `execute`, no `ExecutionMetadata` render at all. The durable legs (notice row + webhook) always fire; the trace event and herald line apply only where an emitter or `ExecutionMetadata` exists (the streaming final chunk). Carry the won warning in `RunScope` (additive field, `#[non_exhaustive]` struct) so `PaladinExecutionService` can emit it through `cx.trace_emitter.or_else(current_trace_emitter)` — the `MiddlewareEvent` precedent, a no-op when no emitter is wired.

### Anti-Patterns to Avoid
- **Reading `Utc::now()` anywhere in window, boundary or `Retry-After` logic.** Only `store_now()`.
- **`f64` for any comparison.** `format_cost` converts nanos to `f64` for *display only*; threshold math is `balance as i128 * 100 >= ceiling as i128 * warn_at as i128`.
- **Emitting the trace event from the submit process** (seq collision, C6).
- **Gating on `PrincipalRef` role or exempting Admin** (D-09). The Treasurer takes `&RunAttribution`.
- **`WHERE (? IS NULL OR col >= ?)` filters.** They defeat index use under prepared/generic plans; use `QueryBuilder` with the predicate pushed only when present (the `spend` precedent).
- **A far-future sentinel `window_end`** for lifetime ceilings (C10).
- **Putting `Treasurer` or any facade type in `paladin-web`** (C1).
- **Logging, trace-emitting or webhooking an API key value.** Names and tenant ids only.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Decimal money parsing | A second parser or `f64` | `parse_price_nanos_per_million` in `src/config/treasurer.rs` (verified: result = integer part x 1e9 + 9-digit padded fraction, i.e. already nano-units; factor/rename it, do not fork it) | Exact-integer grammar, overflow-checked; allows `0`, so allowance `validate()` must reject zero explicitly |
| Once-per-window dedup | An in-process `HashSet` or "SELECT then INSERT" | Unique index + `INSERT ... ON CONFLICT DO NOTHING`; `rows_affected == 0` means lost | Correct across replicas; the settle index is the in-repo proof (zero rows -> `AlreadySettled`) |
| Webhook transport, signing, retries, SSRF | A second HTTP client or HMAC | `WebhookDeliveryService`, `sign_webhook_body`, `SsrfGuard`, `build_webhook_client` | Security-reviewed slice; D-17 forbids a second client |
| Run/ledger scope mapping | A new attribution->scope function | `LedgerScope::from_attribution`, `PrincipalRef::attribution()` | The single mapping (Phase 40 D-15) |
| Tenant/key identifier validation | New regex | `TenantId::new` for tenant ids and API key names | Same rules the auth config already enforces |
| Dynamic SQL filters | String-formatted SQL | `sqlx::QueryBuilder` seeded with a static literal | House rule: no caller value is ever interpolated into SQL text |
| Backend URL redaction in errors | Ad hoc | `crate::waypoint::redact::redact_database_url_password` via the adapter `wrap` fns | Security instruction: redact before truncation |
| Timestamp truncation for storage | Own rounding | `crate::run::storage_timestamp` | One TEXT encoding, lexical order = chronological (verified: zero-fraction and micro-fraction RFC 3339 strings order correctly) |
| `429` body shape | Free-form JSON | `ApiError::new(StatusCode::TOO_MANY_REQUESTS, "allowance_exhausted", msg).with_details(..)` + new `with_retry_after` | One envelope for the whole API; never reuse code `too_many_requests` (the per-IP limiter) |

**Key insight:** every hard sub-problem here (idempotency, signing, SSRF, scope mapping, money parsing) already has a reviewed in-repo solution. The phase's own code should be window arithmetic, ordering, and glue.

## Common Pitfalls

### Pitfall 1: Phantom notice from a run that never persists
**What goes wrong:** the notice is claimed at admission, then `insert_with_latest`/`insert` fails (`ThreadBusy`, backend error) or enqueue fails; the window's only notice is consumed by a run that does not exist, and the webhook (if sent at claim time) described a run that was never created.
**Why:** claim and insert are two writes with no shared transaction.
**How to avoid:** Pattern 3 (claim, insert, `confirm`/`abandon`); enqueue the operator webhook only in `confirm`.
**Warning signs:** a notices row whose `run_id` has no `runs` row; a `409 thread_busy` immediately followed by a missing warning.

### Pitfall 2: `seq` collision on the run's trace stream
See C6. **Warning sign:** `run_traces` primary-key violations or a swallowed duplicate-seq write in `PersistingTraceSink` for a freshly admitted run.

### Pitfall 3: `NULL` in a `UNIQUE` key never conflicts
See C5. **How to avoid:** `api_key_id NOT NULL` with `''` for tenant scope; contract clause for the tenant-scope duplicate.

### Pitfall 4: Operator notice signed with the wrong secret
See C3(c). Without the operator-secret branch the service reads `runs.get(correlation_id)` -> `None` and reschedules forever (the existing WR-27-01 arm). **Warning sign:** operator deliveries stuck `Retrying` with `run ... not found while loading signing key`.

### Pitfall 5: `${VAR}` not expanded
See C7. **Warning sign:** the receiver's HMAC never verifies and the secret equals the literal placeholder text.

### Pitfall 6: v0.9 OpenAPI golden fails on a `429` annotation
See C9. **Warning sign:** `openapi_v0_9_paths_match_the_frozen_baseline` red after annotating the agent handlers.

### Pitfall 7: Refusal maps to `500`
`map_submission_error` ends with `other => internal_repo_error(...)`. If the new `RunSubmissionError::AllowanceExhausted` arm is forgotten it silently becomes a `500`, and `thread_controller::fork_thread` reuses the same function. **How to avoid:** an explicit arm plus a test asserting `429` + `allowance_exhausted` + `Retry-After` through both `POST /v1/runs` and `POST /v1/threads/{id}/fork`.

### Pitfall 8: The `jobs` route bypass
See C4. **Warning sign:** a refused `POST /runs` caller still accumulates ledger spend through `POST /agents/{id}/jobs`.

### Pitfall 9: Role check silently starts applying to schedules
See C13. Building a `PrincipalRef` for a schedule-fired run would route it through `authorize_invocation`; an Admin creator is not in a `[user]`-only assistant's `allowed_roles`. **How to avoid:** `attributed_to` carries identity only; the written decision keeps the skip.

### Pitfall 10: Flaky window tests near a boundary
A real-clock end-to-end test that seeds the ledger and then POSTs can straddle an epoch-aligned boundary (a `1h` window flips every hour on the hour). **How to avoid:** unit-prove window math with the scripted ledger double; in HTTP end-to-end tests use `period: "1d"` and assert `window_start` computed before and after seeding are equal (retry once if not), or prove lifetime refusal (no window) end to end.

### Pitfall 11: Changing `treasurer.currency` after rows exist
`balance` treats a foreign-currency row in scope as `CurrencyMismatch`, which D-10 turns into a `500` for every allowanced principal in that scope; for a lifetime ceiling this is permanent. **How to avoid:** document in `configuration.md`/ADR-0056: changing the operator currency with allowances in force requires a ledger migration; consider a boot-time probe is out of scope.

### Pitfall 12: Mixed-version rollout
An older replica reading a `webhook_deliveries` row with `event = 'allowance_warning'` fails `event_from_str` for the whole `claim_due` batch (poison pill), and an older reader of `run_traces` hits an unknown `kind`. **How to avoid:** document "upgrade every replica before setting `allowance.webhook`" in `http-service-host.md`; the trace-stream impact is limited to runs carrying a notice.

### Pitfall 13: SQLite migration number gap
See C2. Proven by the first SQLite adapter test after `010` lands; keep a no-op `009` as the documented fallback.

### Pitfall 14: Tenant-wide SUM scans
The ledger index is `(tenant_id, api_key_id, attributed_at)`. A tenant-wide window `SUM` can use only the `tenant_id` prefix and must walk every key's rows; a tenant lifetime `SUM` reads every row the tenant ever wrote. At admission frequency this can grow. **Recommendation `[ASSUMED]`:** add `CREATE INDEX IF NOT EXISTS idx_treasury_ledger_tenant_window ON treasury_ledger (tenant_id, attributed_at)` in a new `012` migration (additive; not a change to the final `007` schema). Defer if measurement shows it is not needed.

### Pitfall 15: Exhaustive `TraceEvent` matches
`trace.rs` has an in-crate exhaustive match with no wildcard and a `assert_eq!(events.len(), 12)` ("twelve variants") test; both must be updated for the thirteenth. Every other match in the workspace already has a wildcard (`events.rs` bus mapping returns `None`, `superstep_of` returns `0`, `otel_sink` ignores, `log_sink` serializes generically, `eval.rs` falls to `Debug`). Test helpers in `engine/mod.rs`, `engine/hooks.rs` and `examples/observability_tracing.rs` name variants with a `_ => "unknown"` arm and need no change.

### Pitfall 16: SSRF guard rejects an internal operator URL by default
The most natural operator target (an internal alerting host) is private; `SsrfGuard::new(allow_private)` uses `webhooks.allow_private` (default `false`). **How to avoid:** document that an internal target requires `webhooks.allow_private: true` (which never overrides the metadata-address rejection) and make the boot error name both keys.

### Pitfall 17: Typo'd `treasurer.allowance` keys stay inert
`TreasurerConfig` is `#[serde(default)]` with no `deny_unknown_fields`, so `treasurer.allowances:` (plural) or `treasurer.allowence:` parses to an empty policy and enforces nothing — the Phase 40 D-05 failure shape. **How to avoid:** `#[serde(deny_unknown_fields)]` on `AllowanceConfig`, its entry struct and its webhook struct; recommend adding it to `TreasurerConfig` too (the section is unreleased, so stricter loading breaks no published config). Record in §9.5.

### Pitfall 18: Secret-bearing config type
`TreasurerConfig` derives `Debug` and `Serialize`; a nested `webhook.secret` must not leak through either. **How to avoid:** manual redacting `Debug` (the `WebhookSpec` pattern) and `#[serde(skip_serializing)]` on the secret; add a test that `format!("{config:?}")` and `serde_json::to_string(&config)` contain no secret text.

### Pitfall 19: Postgres code is outside the coverage gate
`scripts/coverage.sh` does not start Postgres; `*::postgres` tests skip locally and in the coverage job and run in CI's `postgres-integration` job (it fails on any `SKIP:` line and discovers `treasury::postgres` by module-path substring). Keep Postgres-specific code thin and mirror the SQLite adapter line for line so the in-memory and SQLite legs carry the coverage.

## Code Examples

Patterns for the planner to reference. Sources are in-tree files verified this session.

### 1. Window function (pure; unit-testable with a fake clock)
```rust
// Source: derived from D-01; arithmetic only, no local clock.
use chrono::{DateTime, Utc};

/// Epoch-aligned tumbling window containing `now` for a period of `period_secs` seconds.
pub fn window_for(
    now: DateTime<Utc>,
    period_secs: u64,
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let p = i64::try_from(period_secs).ok().filter(|p| *p > 0)?;
    let start = now.timestamp().div_euclid(p).checked_mul(p)?;
    let end = start.checked_add(p)?;
    Some((
        DateTime::from_timestamp(start, 0)?,
        DateTime::from_timestamp(end, 0)?,
    ))
}
```

### 2. Admission loop (short-circuit refuse, collect crossings)
```rust
// Source: shape per D-03/D-05/D-10/D-15; names are the planner's.
pub async fn admit(&self, subject: &RunAttribution, run: Option<&RunId>)
    -> Result<Admission, AdmissionError>
{
    let ceilings = self.policy.ceilings_for(subject);   // key-window, key-life, tenant-window, tenant-life
    if ceilings.is_empty() { return Ok(Admission::none()); }   // no ledger touch (D-03, D-10)
    let now = self.ledger.store_now().await.map_err(AdmissionError::backend)?;
    let mut crossings = Vec::new();
    for c in &ceilings {
        let (since, until) = match c.period_secs {
            Some(p) => window_for(now, p).map(|(s, e)| (Some(s), Some(e))).ok_or_else(..)?,
            None => (None, None),                           // lifetime
        };
        let balance = self.ledger.balance(BalanceQuery { /* tenant, key?, currency, since, until */ }).await
            .map_err(AdmissionError::backend)?;            // CurrencyMismatch etc. -> fail closed
        if balance.nanos() >= c.ceiling_nanos {
            return Err(AdmissionError::Refused(AllowanceRefusal { /* kind, balance, ceiling,
                window: since.zip(until), evaluated_at: now */ }));
        }
        // i128, never f64:
        if c.warn_at > 0 && i128::from(balance.nanos()) * 100
            >= i128::from(c.ceiling_nanos) * i128::from(c.warn_at) { crossings.push((c, balance, since, until)); }
    }
    self.claim(crossings, run).await                        // INSERT .. ON CONFLICT DO NOTHING per crossing
}
```

### 3. `balance` SQL (SQLite shown; Postgres adds `::BIGINT` on the aggregate)
```rust
// Source: mirrors BALANCE_QUERY / FOREIGN_CURRENCY_QUERY in crates/paladin-storage/src/treasury/sqlite.rs.
// Static prefix + QueryBuilder pushes (house pattern from `spend`), never `? IS NULL OR ...`.
const BALANCE_PREFIX: &str =
    "SELECT COALESCE(SUM(amount_nanos), 0) FROM treasury_ledger WHERE tenant_id = ? AND currency = ?";
// then: if let Some(k) = api_key { qb.push(" AND api_key_id = ").push_bind(k) }
//       if let Some(s) = since    { qb.push(" AND attributed_at >= ").push_bind(storage_timestamp(s)) }
//       if let Some(u) = until    { qb.push(" AND attributed_at < ").push_bind(storage_timestamp(u)) }
// Foreign-currency probe first, with the SAME scope/window predicates and `currency <> ?`:
//   SELECT currency FROM treasury_ledger WHERE tenant_id = ? [AND api_key_id = ?] [window] AND currency <> ? LIMIT 1
// Hit -> TreasuryLedgerError::CurrencyMismatch (never convert).
```
Run the probe and the SUM in one read transaction on both SQL backends so they see one snapshot; unlike `reserve` no write lock or advisory lock is needed (a read).

### 4. Notice claim (store-enforced, tenant scope uses `''`)
```sql
-- Source: shape of the settle index/INSERT in 007 + SETTLE_INSERT. Migration 011 (both backends):
CREATE TABLE IF NOT EXISTS treasury_notices (
  notice_id TEXT PRIMARY KEY NOT NULL,
  scope_kind TEXT NOT NULL CHECK (scope_kind IN ('tenant','api_key')),
  tenant_id TEXT NOT NULL,
  api_key_id TEXT NOT NULL,          -- '' for tenant scope: NULL would never conflict (C5)
  limit_kind TEXT NOT NULL CHECK (limit_kind IN ('window','lifetime')),
  window_start TEXT NOT NULL,        -- TIMESTAMPTZ on Postgres; Unix epoch for lifetime
  window_end TEXT NULL,              -- needed by the trace event/herald/webhook on readback
  ceiling_nanos BIGINT NOT NULL,
  currency TEXT NOT NULL,
  balance_nanos BIGINT NOT NULL,
  warn_at INTEGER NOT NULL,          -- ditto
  run_id TEXT NULL,
  recorded_at TEXT NOT NULL,
  schema_version TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once
  ON treasury_notices (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos);
CREATE INDEX IF NOT EXISTS idx_treasury_notices_run ON treasury_notices (run_id) WHERE run_id IS NOT NULL;
-- INSERT ... ON CONFLICT (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos) DO NOTHING
-- rows_affected == 0 -> NoticeOutcome::AlreadyRecorded
```
`window_end` and `warn_at` extend D-16's column list; they are required to rebuild the `AllowanceWarning` event on readback and are a planner call to confirm at the checkpoint.

### 5. `Retry-After` on `ApiError`
```rust
// Source: extends crates/paladin-web/src/error.rs (private field => additive).
pub fn with_retry_after(mut self, secs: u64) -> Self { self.retry_after = Some(secs); self }
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut resp = (self.status, Json(self.to_body())).into_response();
        if let Some(secs) = self.retry_after {
            resp.headers_mut().insert(axum::http::header::RETRY_AFTER, HeaderValue::from(secs));
        }
        resp
    }
}
// one shared helper used by run + agent controllers:
// allowance_exhausted(refusal) -> ApiError::new(TOO_MANY_REQUESTS, "allowance_exhausted", refusal.to_string())
//     .with_details(json!({scope, kind, balance: format_cost(..), ceiling: format_cost(..), window_start, window_end}))
//     .with_retry_after(secs)   // only when a window ceiling; omitted for lifetime
```
Whether `HeaderValue::from(u64)` is the right conversion for axum 0.8's re-exported `http` version is `[ASSUMED]` (standard `http` crate API); the first test of the header proves it.

### 6. Period parse (`<integer><unit>`, units `m`/`h`/`d`)
```rust
// Source: derived from D-02 + the checked-arithmetic style of parse_price_nanos_per_million.
fn parse_period_secs(raw: &str) -> Result<u64, PeriodError> {
    let (digits, unit) = raw.split_at(raw.len().saturating_sub(1));
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) { return Err(PeriodError::Malformed); }
    let n: u64 = digits.parse().map_err(|_| PeriodError::Overflow)?;
    let mult = match unit { "m" => 60, "h" => 3_600, "d" => 86_400, _ => return Err(PeriodError::Malformed) };
    let secs = n.checked_mul(mult).ok_or(PeriodError::Overflow)?;
    if secs < 60 { return Err(PeriodError::TooShort); }      // >= 1m (D-02)
    // Recommend a documented maximum (e.g. 366d) so window_end arithmetic is always in range.
    Ok(secs)
}
```
Do not trim: ` 1h`, `1H`, `1.5h`, `-1h`, `0m` are all rejected (never clamp, D-00d).

### 7. Admission test shape ("touches nothing")
```rust
// Source: submit_with_a_loopback_webhook_url_is_rejected_and_touches_nothing (submission.rs ~617).
// Seed `ceiling` nanos via ledger.settle(.. LedgerScope::new(tenant, key) ..), submit with
// requested_by: Some(PrincipalRef::new(key, tenant, UserRole::User)), then assert:
//   matches!(err, RunSubmissionError::AllowanceExhausted(_))
//   queue.depth().await? == 0 && repository.list(RunQuery::default()).await?.items.is_empty()
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-IP request pacing (`too_many_requests`) | Per-principal money allowance (`allowance_exhausted`) | This phase | Two different `429`s; clients tell quota from pacing by `code` |
| Spend visible only after the fact (`paladin-cli treasury spend`) | Spend consulted at admission | This phase | First code path that says "no" on money already spent |
| Run webhooks carry run lifecycle only | Operator notices through the same queue | This phase | New event discriminator; caller-facing twin deferred |

**Deprecated/outdated:** none. Fixed tumbling windows trade a documented 2x boundary burst for predictable reset (D-01, accepted).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `sqlx::migrate!` accepts a non-contiguous version sequence (SQLite `008` then `010`) | C2, Pitfall 13 | A no-op SQLite `009` is needed; trivial, proven by the first adapter test |
| A2 | The `config` crate does not expand `${VAR}` in YAML values (only a grep of this repo supports it) | C7, Pitfall 5 | If it does expand, the D-02 example works as written and the extra env override is merely redundant |
| A3 | `TreasuryLedgerPort` was never in a published release (derived from dates; no tags in this clone) | C11 | If it was published, D-04's `Y` + allowlist is exactly right and no discussion is needed |
| A4 | A tenant-wide `(tenant_id, attributed_at)` index is worth adding (performance reasoning only, not measured) | Pitfall 14 | Unneeded migration `012`; harmless but avoidable |
| A5 | `HeaderValue::from(u64)` compiles against axum 0.8's `http` types | Code Example 5 | One-line fix (`to_string().parse()`) |
| A6 | A UUIDv7 produced by `RunId::new_v7()` is acceptable as a delivery correlation id and `ThreadId::new("treasurer-notices")` validates | C3 | Use a real generated thread id instead; trivial |
| A7 | Postgres contract tests cannot run locally (no daemon, no server here) so CI's `postgres-integration` job is the only proof for the Postgres legs | Environment Availability | A local cluster would give earlier feedback |

## Open Questions (RESOLVED)

Every question below is resolved by a Phase 41 plan; each carries its own RESOLVED note citing the plan and task that settles it.

1. **How is the operator webhook secret supplied?**
   - Known: no `${VAR}` expansion code exists (C7); `treasurer.currency` is the only scalar env override today.
   - Unclear: whether the existing `http.auth.api_keys[].key: "${...}"` examples work in practice.
   - Recommendation: add `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`, pin the `${}` behaviour with a test, and ask the operator whether the api_keys examples need a follow-up.
   - **RESOLVED:** 41-03 Task 1 adds the `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` env override and pins the literal `${VAR}` behaviour with `yaml_env_placeholder_is_not_expanded`; 41-03 Task 2 files the conditional WINDOWS.md deviation row for the `http.auth.api_keys[].key: "${...}"` examples (owner: Phase 46 docs currency); 41-08 wires the secret onto `WebhookDeliveryService::with_operator_notice_secret`.

2. **Which `RunEventKind` disposition for operator rows?** (C3 Option A vs B.) Recommendation: Option A with a correlation `RunId`. Decide at the checkpoint, before migration `011`.
   - **RESOLVED (design fixed in 41-01 Task 1 item 6, operator confirmation at that checkpoint):** C3 Option A -- `RunEventKind::AllowanceWarning`, the enum marked `#[non_exhaustive]`, a fresh correlation `RunId` no run owns and the fixed thread `treasurer-notices`; an option-c redirect recorded in 41-01-SUMMARY.md overrides it. Implemented by 41-08 Tasks 1-2.

3. **Should the trace event also be emitted on the HTTP agent paths?** Known: no worker emitter exists there; the precedent is "emit if an emitter is wired". Recommendation: carry the warning in `RunScope`; emit when an emitter exists; render the herald line only on the streaming final chunk. Confirm the scope is acceptable.
   - **RESOLVED:** 41-07 Task 2 adds `RunScope.allowance_warnings`; `PaladinExecutionService` emits it once when a trace emitter is wired and the streamed final chunk's `ExecutionMetadata` carries the herald line. The best-effort nature of these two legs on the agent routes (the durable notice row and the operator webhook always fire) is recorded in ADR-0056 by 41-09 Task 1 and in platform-api.md by 41-08 Task 3.

4. **`allowance.api_keys` / `tenants` with auth disabled.** Accept exactly `anonymous` / `open-access` as valid targets when `http.auth.enabled = false`? (Needs constants exported from `agent_auth.rs`.) Recommendation: yes.
   - **RESOLVED (yes):** 41-03 Task 2 adds `paladin_web::agent_auth::OPEN_ACCESS_PRINCIPAL_ID` and makes the D-11 cross-check accept `api_keys.anonymous` / `tenants.open-access` when `http.auth.enabled` is false (`build_run_api_accepts_open_access_targets_when_auth_is_disabled`).

5. **Maximum `period`, and lifetime-only entries.** D-02 requires `period` and `amount`; a lifetime-only cap is not expressible. Recommend a documented max (366d) and leaving lifetime-only as a future extension.
   - **RESOLVED:** 41-01 Task 1 item 8 records the discretion call and 41-01 Task 3 implements it (`MAX_ALLOWANCE_PERIOD_SECS` = 366 days, `period` and `amount` required on every entry, so a lifetime-only entry is not expressible); 41-03 Task 1 documents it in config.example.yml and configuration.md.

6. **Does an allowance-refused schedule tick count as `skipped_ticks`?** Today any non-`ThreadBusy` submit error becomes `SkipReason::SubmissionError` without incrementing the counter. Recommendation: add `SkipReason::AllowanceExhausted` (enum is `#[non_exhaustive]`) and call `increment_skipped`, so operators see the missed tick in the PLAT-FR-13 counter.
   - **RESOLVED (yes):** 41-05 Task 2 adds `SkipReason::AllowanceExhausted` and calls `increment_skipped` on an allowance-refused tick (`tick_for_an_exhausted_creator_is_skipped_and_counted`).

7. **Is `ScheduleResponse` to expose `created_by`?** `GET /schedules` is not tenant-scoped, so exposing it would reveal tenant names across tenants. Recommendation: do not expose in this phase.
   - **RESOLVED (no):** 41-05 Task 2 keeps `created_by` out of `ScheduleResponse` (`schedule_response_does_not_expose_the_creator`, plus an awk acceptance check on the struct).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (pinned) | everything | yes | 1.97.1 (`rust-toolchain.toml`) | — |
| `make` | gates (`clean-code`, `api-surface`, `check-gates`) | yes | present | run underlying `cargo` commands |
| Docker daemon | Postgres/Redis/RustFS contract and coverage services | no (client only, no socket) | 29.3.1 client | CI `postgres-integration` and `coverage` jobs; or a local PostgreSQL cluster |
| PostgreSQL server | `treasury::postgres` contract clauses | no (`psql` client only; `pg_isready` no response) | — | Tests print `SKIP:` locally; CI job is authoritative (fails on any `SKIP:`) |
| Redis | coverage script / queue tests | no (binary present, not running) | — | CI |
| `cargo-llvm-cov` | 82% coverage gate | no | — | CI `coverage` job; `bash scripts/coverage.sh` needs services |
| `cargo-semver-checks`, `cargo-audit`, `cargo-deny`, `cargo-cyclonedx`, `mdbook` | `make security`, semver job, docs | no | — | CI; install per Makefile if local proof is wanted |
| Python 3 `sqlite3` module | quick SQL semantics checks | yes | SQLite 3.45.1 | — |
| Node 22 | gsd tooling (`windows append`) | yes | 22 | — |

**Missing dependencies with no fallback:** none for planning. Execution will rely on CI for the Postgres legs, coverage, `make security` and the semver job.
**Missing dependencies with fallback:** all of the above rows marked "no".

## Validation Architecture

Nyquist validation is enabled (`workflow.nyquist_validation` is absent from `.planning/config.json`, which is treated as enabled). [VERIFIED: .planning/config.json]

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (native `#[cfg(test)]`, doctests, `#[tokio::test]`; multi-thread flavor for race clauses). No external framework |
| Config file | none; shared ledger contract suite is the plain module `crates/paladin-storage/src/treasury/contract_tests.rs`; coverage via `cargo llvm-cov --fail-under-lines 82` (`scripts/coverage.sh`) |
| Quick run command | `cargo test -p paladin-storage --features sqlite --lib treasury::` (per-crate quick runs below) |
| Full suite command | `cargo test --workspace`; Postgres leg `STORAGE_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test cargo test -p paladin-storage --features postgres --lib postgres -- --test-threads=1` |

Per-area quick commands (package names verified from the 39/40 validation docs): `cargo test -p paladin-ai-core --lib allowance`, `cargo test -p paladin-ports --doc allowance`, `cargo test -p paladin-storage --lib treasury::in_memory`, `cargo test -p paladin-storage --features sqlite --lib treasury:: run_schedule::`, `cargo test -p paladin-ai --lib config::treasurer`, `cargo test -p paladin-ai --features web-server --lib application::services::treasurer`, `cargo test -p paladin-ai --features web-server --lib application::services::run`, `cargo test -p paladin-web --lib`, `cargo test -p paladin-web --test openapi_golden_v0_9`, `cargo test -p paladin-herald`.

### How the in-memory path simulates `store_now()` for window-boundary tests
`InMemoryTreasuryLedger::store_now` is `storage_timestamp(Utc::now())` and `reserve` stamps `attributed_at` the same way; there is **no injectable clock**. Two complementary techniques, no production-surface change needed:
1. **Treasurer-level scripted ledger double** (a `FakeLedger` implementing `TreasuryLedgerPort` with a settable `now: Mutex<DateTime<Utc>>` and a `Vec<(scope, instant, nanos)>` whose `balance` filters by `[since, until)`): drives `now` to `ws-1s`, `ws`, `ws+P-1s`, `ws+P`, a leap-free epoch multiple, and a `now` decades away from the real clock, proving the **store** clock (not `Utc::now()`) selects the window, the boundary is half-open, `Retry-After` equals `window_end - evaluated_at`, and a lifetime ceiling ignores time. Reuse this double for the warn-per-window rollover test (next window re-arms).
2. **Adapter-level**: the `balance_window_is_half_open` contract clause uses the existing sleep-bracketing technique (`spend_window_is_half_open`, 20 ms sleeps around sampled `store_now()` values); exact `attributed_at == window_start` inclusivity and `== window_end` exclusivity are proven by adapter-local tests that insert a row with a chosen `attributed_at` (in-memory pushes an `Entry` directly; SQLite/Postgres insert via `sqlx` raw SQL).
If a public test seam is preferred later, `InMemoryTreasuryLedger::with_clock(Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>)` is the minimal addition (a `make api-surface-update` item); this research does not require it.

### Contract-suite placement
- **Ledger clauses** go in `crates/paladin-storage/src/treasury/contract_tests.rs` (plain `pub async fn` per clause, unique `contract_scope(clause)` tenants), wired unchanged from `in_memory.rs`, `sqlite.rs` and `postgres.rs` `#[tokio::test]`s (`store_or_skip` for Postgres): `balance_sums_signed_contributions_in_window` (reserve + reserved settle + release), `tenant_balance_equals_sum_of_key_balances` (the phase's first red test), `key_balance_excludes_other_keys_and_tenants`, `balance_window_is_half_open`, `balance_unbounded_counts_every_row`, `balance_mixed_currency_is_currency_mismatch`, `balance_of_empty_scope_is_zero_in_requested_currency`.
- **Notice clauses** in a new `crates/paladin-storage/src/treasury/notice_contract_tests.rs` (or the same file), same wiring: `first_claim_wins_duplicate_is_already_recorded`, `tenant_scope_duplicate_dedups` (guards C5), `raised_ceiling_rearms_the_same_window`, `distinct_window_start_is_a_distinct_notice`, `sixteen_concurrent_claims_yield_exactly_one_recorded` (`Arc<dyn>`, multi-thread; SQLite via `new_shared_file` WAL), `notices_for_run_returns_only_won_rows`, `abandon_removes_only_this_admissions_rows`.
- **Schedule clauses** in `crates/paladin-storage/src/run_schedule/contract_tests.rs`: `created_by_round_trips`, `null_created_by_reads_back_none`, `half_attributed_row_is_rejected_on_read`, `update_never_changes_created_by`.

### HTTP end-to-end proof shape
Follow `tenant_scoped_run_read_tracer` / `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` in `src/application/services/run/http_surface_tests.rs`: real `RunSubmissionService` over `SqliteRunRepository` (temp file) + `InMemoryRunQueue`, a ledger (in-memory or temp-file SQLite) seeded with `settle` rows under `LedgerScope::new(tenant, key)`, `run_router(RunApiState::new().with_submission(..).with_repository(..).with_auth(..))`, `tower::ServiceExt::oneshot`.
- `429` on `POST /v1/runs`: status `429`; `Retry-After` is a positive integer <= the period; body `error.code == "allowance_exhausted"` and `details` carries exactly `scope, kind, balance, ceiling, window_start, window_end` (balance/ceiling as `format_cost` strings); no `tenant`/key echoed; `repository.list` is empty and `queue.depth() == 0`; another tenant's key still gets `202`.
- Lifetime refusal: `Retry-After` header **absent**.
- `POST /v1/threads/{id}/fork` returns the same `429` (through `map_submission_error`).
- Agent routes: `agent_router(AgentApiState::new(registry).with_treasurer(port).with_auth(..))` with a counting stub executor (existing `MockExecutor`-style doubles in `agent_controller.rs` tests); `execute`, `execute/stream` and `jobs` each answer `429` and the executor call count stays `0`; an unlimited key is unaffected.
- Warn: below threshold -> no notice row; at the threshold -> exactly one `treasury_notices` row, one operator `webhook_deliveries` row, still `202`; a second admission in the same window adds nothing; payload key set pinned.
- Worker readback: in `worker_tests.rs` style, submit with a pre-existing notice row for the run, drive `run_once`, assert exactly one `AllowanceWarning` record precedes `RunStarted`, a `Running` redelivery does not re-emit, and `HeraldTraceSink` folds `treasurer.allowance_warning` into the metadata.
- Schedule: a schedule whose creator is at the ceiling ticks to `SkipReason::AllowanceExhausted`, writes no run, and increments `skipped_ticks`; a `NULL created_by` schedule still fires unattributed and ungated.

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ALLOW-01 | Grammar parse/reject (period, amount, lifetime, warn_at, unknown fields, zero, overflow, path-precise errors) | unit | `cargo test -p paladin-ai --lib config::treasurer` | ❌ Wave 0 (extend existing module) |
| ALLOW-01 | Window math at boundaries from store clock; lifetime ignores time | unit (scripted ledger) | `cargo test -p paladin-ai --features web-server --lib application::services::treasurer` | ❌ Wave 0 |
| ALLOW-01 | `balance` semantics on all three adapters | contract | `cargo test -p paladin-storage --features sqlite --lib treasury::` (+ Postgres leg in CI) | ❌ Wave 0 |
| ALLOW-01 | D-11 boot coherence (disabled store, unknown key/tenant, auth-disabled targets) | unit | `cargo test -p paladin-ai --lib config::treasurer` + `... infrastructure::web::run_api_wiring` | ❌ Wave 0 |
| ALLOW-02 | Refusal before any write (submit, fork, schedule) | unit/service | `cargo test -p paladin-ai --features web-server --lib application::services::run::submission` | ❌ Wave 0 |
| ALLOW-02 | `429` + `Retry-After` + body on `/runs`, fork, execute, stream, jobs | e2e | `cargo test -p paladin-ai --features web-server --lib application::services::run::http_surface_tests` | ❌ Wave 0 |
| ALLOW-02 | Fail closed on ledger error (`500`, nothing persisted); no-ceiling principal never touches the ledger | unit | same as above | ❌ Wave 0 |
| ALLOW-04 | Exactly one notice per scope+window across 16 concurrent claims; raised ceiling re-arms | contract | `cargo test -p paladin-storage --features sqlite --lib treasury::` | ❌ Wave 0 |
| ALLOW-04 | One trace event, one herald line (3 renderers), one operator webhook; payload key set | unit/e2e | `cargo test -p paladin-ai --features web-server --lib application::services::run` + `cargo test -p paladin-herald` + `cargo test -p paladin-core --lib trace` | ❌ Wave 0 |
| ALLOW-04 | Run is never blocked by a notice/webhook failure | unit | treasurer + worker tests | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** the touched crate's targeted `cargo test ... --lib <module>` plus `cargo clippy -p <crate> --all-targets -- -D warnings`.
- **Per wave merge:** `cargo test --workspace` (at minimum the storage `treasury::` and run-service suites for early waves).
- **Phase gate (before `/gsd-verify-work`):** `cargo fmt --check`, `cargo test --workspace`, `make clean-code`, `make api-surface` (after `make api-surface-update` + CHANGELOG), `make check-gates` (includes `check-migration-allowlist`), `make security`, `make openapi` with `git diff --exit-code crates/paladin-web/openapi.json` clean, `cargo test -p paladin-web --test openapi_golden_v0_9`, CI `coverage` (>= 82%) and `postgres-integration` (no `SKIP:`).

### Wave 0 Gaps
- [ ] `crates/paladin-core/src/platform/container/allowance.rs` + unit tests (refusal `Display`, serde, `Admission`)
- [ ] `crates/paladin-ports/src/input/allowance_admission_port.rs`, `output/treasury_notice_port.rs` (compiling rustdoc mocks)
- [ ] `balance` clauses and notice clauses in the contract suite, wired for in-memory, SQLite, Postgres; stubs for the 5 existing test doubles (`paladin_execution_service.rs` `FailingTreasuryLedger`, `worker.rs` `RecordingTreasuryLedger`, `run_controller.rs` `StubTreasuryLedger`, `engine/mod.rs` `RecordingTreasuryLedger` and `FailingTreasuryLedger`) plus the port's rustdoc mock and in-file `MockLedger`
- [ ] `src/application/services/treasurer/tests.rs` scripted `FakeLedger`
- [ ] A test pinning `${VAR}` literal behaviour in `src/config/treasurer.rs`
- [ ] A SQLite adapter smoke that opens a fresh pool after `010` lands (proves the `009` gap, A1)
- [ ] Update `trace.rs` variant-count test (12 -> 13) and its exhaustive match
- [ ] Framework install: none

## Security Domain

`security_enforcement` is not set in `.planning/config.json`, so it is treated as enabled. [VERIFIED: .planning/config.json]

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (consumes Phase 40's authenticated `Principal`; adds no credential flow) | existing `authenticate()` |
| V3 Session Management | no | — |
| V4 Access Control | **yes** — allowances bind every principal, no Admin bypass (D-09); tenant is server-derived only; refusal body never echoes key or tenant; the `jobs` route must be gated (C4) | `Principal`/`RunAttribution` from the server-derived principal; one shared admission function |
| V5 Input Validation | **yes** — operator config (period/amount/lifetime/warn_at, map keys), webhook URL | exact-integer grammar, `TenantId::new` for map keys, `deny_unknown_fields`, `SsrfGuard` at wiring and send time |
| V6 Cryptography | **yes** (HMAC only) | `sign_webhook_body` (HMAC-SHA256 over the exact stored bytes); never hand-rolled |
| V7 Error Handling & Logging | **yes** — no key values in logs/errors/traces/webhooks; refusal logs `warn` with scope kind, tenant id, figures | redacting `Debug` on secret-bearing config; `redact_database_url_password` in adapter errors |
| V8 Data Protection | **yes** — operator webhook secret held in config/env, never on the delivery row, never serialised outward | `#[serde(skip_serializing)]`, manual `Debug`, env override |
| V9 Communications | **yes** — outbound HTTP to an operator-chosen URL carrying a signature header | no-redirect client (`Policy::none()`), SSRF guard (DNS rebinding remains the documented, accepted limitation) |
| V13 API | **yes** — `429` contract, `Retry-After` from the store clock | `ApiError` envelope |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Allowance bypass via an ungated spend route (`/agents/{id}/jobs`, schedules, fork) | Elevation of Privilege / Tampering | One admission function called from every principal-bearing path (C4); schedule `created_by` attribution (D-08) |
| Over-admission race (two admissions at the same instant) | Tampering | Accepted by D-05 (check-only); closed by Phase 42 reservations; documented in ADR-0056 |
| SSRF via the operator webhook URL | Spoofing/Information Disclosure | `SsrfGuard` at wiring and send time, no redirects; private targets need `webhooks.allow_private` |
| Webhook secret leakage (Debug, Serialize, logs, delivery row) | Information Disclosure | Redacting `Debug`, `skip_serializing`, secret stays on the service not the row (P1), payload key-set test |
| Mis-signed or unsigned operator notice (wrong secret path) | Spoofing | Operator-secret branch in `process` before the run lookup (C3) |
| Silent no-op from a typo'd config key or entry | Repudiation / Denial of intended control | `deny_unknown_fields`, D-11 boot cross-check naming the offending path |
| Fail-open on ledger outage for an allowanced principal | Elevation of Privilege | Fail closed `500` (D-10); principals with no ceiling are unaffected |
| Cross-tenant information via the refusal body | Information Disclosure | Body carries only the caller's own scope figures; no key name or tenant repeated (D-13) |
| Clock manipulation (worker or client skew moving a window) | Tampering | Only `store_now()` is used; `Retry-After` derived from the same instant (C8) |
| SQL injection through tenant/key identifiers | Tampering | Bound parameters only; `QueryBuilder` with static prefixes |

Manual credential-handling review (required: no Rust SAST gates a merge) must confirm for the Phase 41 diff: no log line interpolates an API key; the operator secret is absent from `Debug`/`Serialize` output; the webhook client is the existing no-redirect one; refusal and notice payloads carry no key value.

## Sources

### Primary (HIGH confidence) — codebase, read this session
- `.planning/phases/41-admission-time-allowance-enforcement/41-CONTEXT.md` — all 20 decisions, discretion, deferred
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/research/SUMMARY.md` §Phase 41, `.planning/decisions/PROMOTION.md`, `.planning/decisions/0054-tenant-scoped-run-reads.md`, `.planning/phases/39-spend-ledger/39-VALIDATION.md`
- `crates/paladin-ports/src/output/treasury_ledger_port.rs`, `.../input/run_submission_port.rs`, `.../input/schedule_admin_port.rs`, `.../output/webhook_delivery_port.rs`, `.../output/trace_sink_port.rs`
- `crates/paladin-core/src/platform/container/{treasury_ledger,principal,trace,run,run_schedule,run_scope,webhook,herald}.rs`
- `crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,postgres,contract_tests}.rs`, `crates/paladin-storage/src/run_schedule/*`, `crates/paladin-storage/src/webhook/sqlite.rs`, `crates/paladin-storage/migrations/{sqlite,postgres}/{004,005,006,007,008,009}_*.sql`
- `src/config/{treasurer,agents,settings}.rs`, `src/application/services/run/{submission,worker,events}.rs`, `.../run/schedule/{service,admin}.rs`, `.../run/webhook/{mod,service,ssrf}.rs`, `.../run/http_surface_tests.rs`, `src/infrastructure/web/run_api_wiring.rs`, `src/infrastructure/telemetry/{herald_sink,otel_sink,log_sink}.rs`, `src/bin/paladin-server.rs`
- `crates/paladin-web/src/{error,agent_auth,agent_controller,run_controller,schedule_controller,openapi}.rs`, `crates/paladin-web/tests/openapi_golden_v0_9.rs`, `crates/paladin-web/Cargo.toml`, `crates/paladin-herald/src/*`
- `MIGRATION.md` §9.2 rows and the CI set-equality step (`.github/workflows/ci.yml` lines 396-482), `.cargo/semver-checks-allowlist.toml`, crate `Cargo.toml` lint tables, `.planning/WINDOWS.md`, `Makefile`, `scripts/coverage.sh`
- Local verification: Python `sqlite3` 3.45.1 run proving NULL-in-UNIQUE does not dedup, `''` does, RFC 3339 lexical ordering with/without fractional seconds, `UPDATE ... RETURNING`

### Secondary (MEDIUM confidence)
- [CITED: https://www.enterprisedb.com/postgres-tutorials/postgresql-unique-constraint-null-allowing-only-one-null] and the PostgreSQL unique-index documentation surfaced by search — NULLs distinct by default, `NULLS NOT DISTINCT` is PG15+ opt-in

### Tertiary (LOW confidence)
- docs.rs and crates.io were unreachable (egress blocked); the `sqlx` migration-gap behaviour (A1), `config` crate `${}` behaviour (A2) and axum header conversion (A5) are `[ASSUMED]` and each has a cheap Wave 0 or first-test proof. Context7 and the `research-plan` seam were not used: no third-party API is in scope.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependency; every pin read from manifests.
- Architecture: HIGH for integration points and constraints (read directly); MEDIUM for the recommended notice/webhook/trace design where CONTEXT.md delegates to the planner.
- Pitfalls: HIGH — each traced to a specific file or reproduced locally (NULL-in-UNIQUE).

**Research date:** 2026-10-02
**Valid until:** 2026-10-16 (fast-moving: Phases 42-44 land in the same files; re-verify `submission.rs`, `worker.rs` and `run_api_wiring.rs` line anchors before planning if they have changed)
