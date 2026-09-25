# Requirements: Paladin — Milestone v0.11.0 "Treasurer Spend Governance"

**Defined:** 2026-09-24
**Core Value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.

**Source of truth:** `.project/Milestone_14-Treasurer/` (overview + Epic 1 PRD R1-R6) for the
Treasurer; PROJECT.md *Current Milestone* for the supporting scope; `.planning/research/SUMMARY.md`
for integration points and pitfalls. Operator decisions recorded 2026-09-24: full governance
(per-tenant + per-API-key); prices from operator config only, nothing bundled; rolling-period
allowances + optional lifetime cap; refuse at admission AND halt runs in flight on every run path;
soft warn threshold; tenant mapped from API key; ledger via a new port with in-memory/SQLite/Postgres;
pacing in-process + Redis-shared + stampede lock, degrading to local pacing when Redis is down;
clean-break removal of the legacy Battalion error/retry/timeout surfaces and
`PaladinError::LlmError(String)`.

## v1 Requirements

Requirements for this milestone. Each maps to exactly one roadmap phase.

### Pricing (PRICE)

- [ ] **PRICE-01**: Operator can configure a per-model price table (prompt, completion, cache-read,
  cache-write and reasoning unit prices, written as decimal strings); the default table is empty,
  and negative or malformed prices are rejected at config validation
- [ ] **PRICE-02**: A pure function maps a `TokenUsage` and the price table to an exact cost in
  fixed-point integer micro-units (no `f64` accumulation), unit-tested per token type including
  cache and reasoning tokens
  *(Amended 2026-09-25, Phase 38 plan 38-01: the cost unit is i64 nano-units (1e-9), a finer scale
  that satisfies "micro-units" — see 38-CONTEXT.md D-02 and ADR-0053.)*
- [ ] **PRICE-03**: A completed run reports its currency cost in `ExecutionMetadata.cost_estimate`
  end-to-end; a model with no configured price yields `None` (never `0`), and the field's reserved
  rustdoc note is updated to "produced by the Treasurer"

### Spend ledger (LEDGR)

- [ ] **LEDGR-01**: A `TreasuryLedgerPort` with in-memory, SQLite and Postgres adapters passes one
  shared contract-test suite, backed by a `007` migration in both
  `crates/paladin-storage/migrations/{sqlite,postgres}/`
- [ ] **LEDGR-02**: Draws reserve then settle atomically, so concurrent draws never overspend —
  when N draws race and only N−1 fit, exactly N−1 succeed, on every adapter
- [ ] **LEDGR-03**: Settlement is idempotent, keyed on run, superstep and attempt, so lease
  redelivery, resume, retries and model fallback never charge twice
- [ ] **LEDGR-04**: Operator can view spend per tenant, API key, run and model over a time window
  from the CLI, and spend appears in herald output and trace events

### Tenant identity (TENANT)

- [ ] **TENANT-01**: Operator config maps each API key to a tenant and the authenticated
  `Principal` carries `tenant_id`; a caller cannot assert its own tenant. The breaking change is
  recorded in `MIGRATION.md` §9.2 and the `cargo semver-checks` allowlist
- [ ] **TENANT-02**: Every run records its submitting principal (API key id and tenant), so spend
  and run reads are attributed to the caller

### Allowances (ALLOW)

- [ ] **ALLOW-01**: Operator can set an `allowance` per tenant and per API key with a rolling period
  and an optional lifetime cap, distinct from every existing `max_tokens` meaning; window boundaries
  are computed in UTC from the store or server clock, never a worker's local clock
- [ ] **ALLOW-02**: Submitting a run while the caller's tenant or API-key allowance is exhausted is
  refused at admission with a typed error, and no run is persisted
- [ ] **ALLOW-03**: A run in flight whose next draw would overspend halts cleanly — typed Treasurer
  error, status `Halted`, last checkpoint kept — and can be resumed once the allowance is replenished
  or the period resets; this holds on both engine-driven (`WarEngine`) and agent-loop
  (`PaladinExecutionService`) runs, with the enforcement attachment point recorded in an ADR
- [ ] **ALLOW-04**: A configurable warn threshold (for example 80%) emits one trace event plus a
  herald and webhook notice per window, without blocking the run
- [ ] **ALLOW-05**: The Treasurer derives the per-run `TokenBudget` from the remaining allowance and
  works alongside `TokenBudget`, `ModelCallLimit`, `ToolCallLimit` and the Commissary without
  replacing any of them; a guard keeps `Treasurer` a framework-only word

### Rate pacing (PACE)

- [ ] **PACE-01**: `LlmError::RateLimitExceeded` carries a retry delay parsed from `Retry-After`
  (delta-seconds or HTTP-date) and from provider rate-limit headers, with the header names verified
  against official OpenAI and Anthropic documentation
- [ ] **PACE-02**: An `LlmPort` pacing decorator paces each provider and model in-process and backs
  off with jitter on a 429, treating the retry delay as a minimum; it wraps every
  `FallbackLlmAdapter` hop, and a mocked-429 test proves back-off rather than thrash
- [ ] **PACE-03**: With Redis configured, pacing state is shared across worker instances through an
  atomic Lua script using the Redis server clock, so a 429 slows the whole fleet
- [ ] **PACE-04**: A distributed cache-stampede lock (set-if-absent with expiry, fencing token,
  delete-only-if-owner) stops concurrent workers from issuing the same cached request twice
- [ ] **PACE-05**: When Redis is unavailable, pacing degrades to conservative per-process pacing
  with a trace warning and never runs unpaced; this is covered by a test

### Legacy clean break (LEGACY)

- [ ] **LEGACY-01**: Legacy `battalion::RetryPolicy`, `battalion::ErrorStrategy` and
  `battalion::NodeError` are removed, while the same-named Aegis and Flow types stay untouched
- [ ] **LEGACY-02**: Legacy Formation, Phalanx and Campaign timeout handling is removed in favour of
  Aegis timeouts
- [ ] **LEGACY-03**: `PaladinError::LlmError(String)` is removed, and the message-matching retry
  check in `conclave_execution_service.rs` moves to the typed `LlmFailure` / `Transience` taxonomy
- [ ] **LEGACY-04**: The X-03 supersession is recorded in an ADR, and every removal has a
  `MIGRATION.md` §9.2 row and a `cargo semver-checks` allowlist entry, with examples and docs updated

### Object storage (STORE)

- [ ] **STORE-01**: The dev/test compose stack and the Coverage, Integration Tests, Docker
  Integration Tests and Kubernetes Smoke Test CI jobs run against RustFS pinned to an exact tag,
  with bucket bootstrap replacing `mc`; no MinIO image remains in any live configuration
- [ ] **STORE-02**: The `FileStoragePort` contract suite (including presigned URLs, multipart
  uploads and ETags) passes against RustFS; the existing S3 adapter is reused, and a new adapter
  behind a feature flag is added only if that suite fails
- [ ] **STORE-03**: Storage docs are updated, and the decision on whether the production k8s
  manifest also moves to RustFS is recorded

### Platform (PLAT, continued)

- [ ] **PLAT-07**: `GET /runs` and every `/runs/{id}*` read route return only runs the calling
  principal may see, enforced by one shared authorization function; another caller's run returns
  404 (`WINDOWS.md` row 32). The admin-role override is decided in the phase
- [ ] **PLAT-08**: Legacy `Runnable::Agent` runs emit SSE live events and webhook deliveries like
  graph runs (row 31)
- [ ] **PLAT-09**: The SSE `done` event matches the persisted status — `Cancelled` for a
  caller-cancelled run and `Halted` with the Treasurer reason for a spend halt (D-14)

### Observability (OBS, continued)

- [ ] **OBS-05**: `LogTraceSink` / `TraceDispatcher::emit` skips serialisation when logging is
  disabled and reuses its buffers; the tracing-overhead benchmark is re-measured and either meets
  PRD 07's ≤ 3 % bar or records a new accepted figure on the record (D-16, row 35)

### Docs and hygiene (CURR, continued)

- [ ] **CURR-22**: The docs fixes assigned by the v0.10.0 acceptance audit are closed: "twelve
  publishable crates" in `development-setup.md`, `release-automation.md` and `release-recovery.md`;
  `paladin-eval` Trusted Publishing and Credential History rows; the CHANGELOG `[0.10.0]` heading
  date
- [ ] **CURR-23**: A Treasurer mdBook page, a configuration reference for pricing, `allowance` and
  the warn threshold, and a v0.10 → v0.11 `MIGRATION.md` guide are published
- [ ] **CURR-24**: Nyquist validation for Phases 22, 24, 29, 30, 34, 36 and 36.1 moves out of
  `draft`, and Phase 28 is made `nyquist_compliant` or its gaps are recorded
- [ ] **CURR-25**: The three v2 debt lines (oversized service files, clone/lock contention,
  dependency-allowlist drift) are each fixed or waived with a written reason

### Release (SHIP, continued)

- [ ] **SHIP-07**: v0.11.0 is released: all 12 publishable crates on crates.io at `0.11.0`, the tag
  cut on a `main` merge commit, CHANGELOG and MIGRATION complete, and the publish-order gate green

## v2 Requirements

Deferred. Tracked but not in this roadmap. FUT-08 (currency cost), FUT-09 (rate pacing and
stampede locks) and FUT-10 (RustFS) are promoted into this milestone as PRICE-*, PACE-* and STORE-*.

### Platform & Tooling

- **FUT-01**: Hand-polished Python/TypeScript SDKs
- **FUT-02**: Full graphical IDE / live-editing studio
- **FUT-03**: Multi-region/HA storage replication
- **FUT-04**: Billing / usage metering (payment integration)
- **FUT-05**: Multi-tenant orgs / RBAC beyond existing scopes and the API-key → tenant mapping
- **FUT-13**: DNS-rebinding address pinning for the webhook SSRF guard

### Runtime

- **FUT-06**: Automatic memory extraction/writing policies for the Vault
- **FUT-07**: LLM-as-judge eval scoring
- **FUT-11**: Long-context tier pricing (distinct unit prices above a context threshold, e.g. ~200K
  tokens)
- **FUT-12**: Multi-currency pricing and FX conversion (v0.11.0 prices in one operator currency)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Exact per-call cost prediction before the response | Completion, cache and reasoning counts are unknowable pre-flight; every comparable system settles after the fact |
| Bundled / default price table | Operator decision: prices come from config only; a bundled table goes stale in a published library |
| Billing and payment integration | FUT-04; the Treasurer governs spend, it does not charge anyone |
| Hosted spend dashboard | Spend is surfaced through heralds, CLI and traces; a UI is FUT-02 territory |
| Retroactive allowance enforcement on historical runs | Allowances apply from configuration forward only |
| Reworking `TokenBudget` / `ModelCallLimit` / `ToolCallLimit` or any Commissary (input-side) work | PRD §4; the Treasurer composes these, it does not replace them (ADR-0049, ADR-0050) |
| Renaming `TokenBudget`, `TokenCounterPort`, `TokenUsage` or `max_tokens` | Locked by the two-officer model decisions |
| Using `Treasurer` outside the framework vocabulary | Milestone 13 §5.3 guardrail: it must never mix with the downstream `GarrisonTreasury` term |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PRICE-01 | Phase 38 | Pending |
| PRICE-02 | Phase 38 | Pending |
| PRICE-03 | Phase 38 | Pending |
| LEDGR-01 | Phase 39 | Pending |
| LEDGR-02 | Phase 39 | Pending |
| LEDGR-03 | Phase 39 | Pending |
| LEDGR-04 | Phase 39 | Pending |
| TENANT-01 | Phase 40 | Pending |
| TENANT-02 | Phase 40 | Pending |
| ALLOW-01 | Phase 41 | Pending |
| ALLOW-02 | Phase 41 | Pending |
| ALLOW-03 | Phase 42 | Pending |
| ALLOW-04 | Phase 41 | Pending |
| ALLOW-05 | Phase 42 | Pending |
| PACE-01 | Phase 43 | Pending |
| PACE-02 | Phase 43 | Pending |
| PACE-03 | Phase 43 | Pending |
| PACE-04 | Phase 43 | Pending |
| PACE-05 | Phase 43 | Pending |
| LEGACY-01 | Phase 44 | Pending |
| LEGACY-02 | Phase 44 | Pending |
| LEGACY-03 | Phase 44 | Pending |
| LEGACY-04 | Phase 44 | Pending |
| STORE-01 | Phase 45 | Pending |
| STORE-02 | Phase 45 | Pending |
| STORE-03 | Phase 45 | Pending |
| PLAT-07 | Phase 40 | Pending |
| PLAT-08 | Phase 45 | Pending |
| PLAT-09 | Phase 42 | Pending |
| OBS-05 | Phase 45 | Pending |
| CURR-22 | Phase 46 | Pending |
| CURR-23 | Phase 46 | Pending |
| CURR-24 | Phase 46 | Pending |
| CURR-25 | Phase 46 | Pending |
| SHIP-07 | Phase 47 | Pending |

**Coverage:**
- v1 requirements: 35 total
- Mapped to phases: 35
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-24*
*Last updated: 2026-09-24 — roadmap created (`/gsd-new-project` roadmapper), Phases 38-47, 35/35
requirements mapped, 100% coverage*
