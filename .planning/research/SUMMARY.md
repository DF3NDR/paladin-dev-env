# Project Research Summary

**Project:** Paladin — v0.11.0 "Treasurer Spend Governance"
**Domain:** Output-side LLM spend governance (pricing, allowances, ledger, rate pacing) added to an existing durable Rust multi-agent orchestration runtime, plus a clean-break legacy removal and one infra swap (RustFS for MinIO)
**Researched:** 2026-09-24
**Confidence:** HIGH (architecture, pitfalls — grounded in direct reads of shipped code); MEDIUM (stack version specifics, feature-landscape comparables — web-sourced, cross-checked)

## Executive Summary

v0.11.0 adds a new "Treasurer" — an output-side officer that computes per-call/per-run cost from operator-configured prices, enforces per-tenant and per-API-key spend allowances (rolling window + optional lifetime cap) at both admission and mid-run, persists a durable spend ledger, and paces outbound calls against provider 429s (in-process and Redis-shared). It composes with, but does not replace, the existing input-side Commissary and the `TokenBudget`/`ModelCallLimit`/`ToolCallLimit` middleware (ADR-0049/0050, already locked). The milestone also does a clean-break removal of legacy Battalion `RetryPolicy`/`ErrorStrategy`/`NodeError`/timeouts and `PaladinError::LlmError(String)`, swaps MinIO for RustFS, fixes an SSE `done`/`Cancelled` bug, closes a `GET /runs*` per-caller-scoping gap, wires SSE/webhook emission for `Runnable::Agent`, and fixes trace-serialization overhead — all bundled as "adjacent hygiene" phases around the Treasurer core.

Comparable systems (LiteLLM, Portkey, Helicone, Langfuse, LangSmith, OpenRouter) validate the PRD's shape almost exactly: price tables keyed by model with prompt/completion/cache-read/cache-write/reasoning axes; budgets as cap+reset-cadence per identity, checked at admission; post-hoc settlement once real token counts return (no system does pre-flight exact cost prediction); Retry-After-driven back-off with jitter. Paladin's genuinely novel piece — a clean, resumable mid-run halt on overspend, built on existing Waypoint checkpointing and Aegis fault handling — has **no prior art to copy** among the surveyed competitors, since none of them have durable multi-step runs; this should be treated as the highest-design-risk, most bespoke slice of the milestone.

The dominant risks are all concurrency/precision correctness issues, not unknowns: currency math must be integer/fixed-point internally (never `f64`-accumulated) even though the existing `ExecutionMetadata.cost_estimate: Option<f64>` field stays `f64` at the display boundary; allowance draws must be reserve-then-settle with one atomic operation, not check-then-write, to avoid double-spend across the already-concurrent worker pool; ledger settlement needs an idempotency key tied to run/attempt to survive Phase 27's proven lease-redelivery path; rolling windows must use UTC/server-clock semantics (mirroring the existing Redis run-queue's `TIME`-based lease-expiry pattern), not local-calendar arithmetic. The Redis-shared rate-pacing/stampede-lock piece (R4/FUT-09) is the one sub-area with no adjacent in-tree pattern to imitate and an explicit fail-open-vs-fail-closed design decision the PRD leaves open — flagged for deeper phase-specific research. The architecture research also surfaces one genuinely open build-order question: the primary v0.10.0 run path (`WarEngine`/superstep engine) never wires `AgentRuntimeConfig::build_chain`/`TokenBudget` at all today, so "the Treasurer installs the per-run TokenBudget" (PRD's literal language) needs a first-class design decision about where mid-run enforcement actually attaches, not an assumption that existing wiring can simply be extended.

## Key Findings

### Recommended Stack

New dependencies are deliberately minimal. `rust_decimal` (1.43.0, MIT) parses operator-entered decimal prices at the config boundary only; the ledger's **persisted** unit must be a plain fixed-point `i64` (micro/nano-USD) column — `sqlx` deliberately does not support `rust_decimal`/`bigdecimal` for SQLite (a maintainer decision), and SQLite is this workspace's always-on backend, so `i64` is the only representation that works identically across in-memory/SQLite/Postgres. `governor` (0.10.4, MIT, GCRA-based, `dashmap`-keyed) handles in-process rate pacing per provider+model; it is in-process only by design, so cross-worker sharing reuses the codebase's *own* proven Redis Lua-atomic idiom (`RunQueuePort`'s `redis::Script` pattern) rather than a new crate — zero new Cargo dependencies for that half. `httpdate` (already transitively resolved) should be promoted to a direct dependency to parse RFC 7231 `Retry-After` HTTP-dates. RustFS 1.0.0 (GA, Apache-2.0, S3-wire-compatible) should be tried first as a drop-in target for the *existing* `rust-s3`-based MinIO adapter before writing any new adapter code — pin the exact tag, never `:latest` (this todo exists because a floating MinIO tag disappeared). Tracing overhead (D-16) should be fixed with two no-new-dependency changes (a `log_enabled!` guard and `serde_json::to_writer` buffer reuse) before reaching for a SIMD JSON replacement.

**Core technologies:**
- `rust_decimal` 1.43.0 — parse/compute operator-configured decimal prices — avoids float rounding error at the config→cost boundary; quantize to `i64` before persisting
- Fixed-point `i64` micro-USD — the ledger's persisted/summed/compared unit — native to `sqlx` on all three backends, mirrors existing "units plain" convention (ACCT VOCAB rules)
- `governor` 0.10.4 — in-process 429 pacing, keyed per provider+model — lock-free GCRA, but no distributed backend; Redis-shared pacing reuses the existing `redis::Script` idiom instead of a new crate

### Expected Features

**Must have (table stakes, matches PRD R1-R6 almost exactly):**
- Per-model price table (prompt/completion/cache-read/cache-write/reasoning axes) — universal across every surveyed competitor
- Cost computed and attached per call and per run (`ExecutionMetadata.cost_estimate` producer)
- Unknown/unpriced model → cost stays `None`, never a guessed fallback number
- Budget/allowance cap + rolling reset cadence per identity (tenant, API key), refused at admission
- Post-hoc settle (actual cost computed after response returns — no system does pre-flight exact prediction)
- Durable ledger with query-by-window
- Retry-After/429 back-off with jitter

**Should have (worth flagging to roadmap, present in every competitor but not literally named in PRD):**
- Soft/warn threshold before hard block (LiteLLM `soft_budget`, Helicone's graduated 50/80/95%) — PRD's R3 wording reads hard-only; worth a clarifying question
- Explicit tested behavior (not just rustdoc) for "unpriced model → `None`, never `0`"

**Differentiators (genuinely novel, no prior art to copy):**
- Clean, resumable in-flight halt on overspend, built on Waypoint/Aegis — no comparable system has durable multi-step runs to solve this for
- Spend surfaced through existing trace stream/heralds, not a bolted-on dashboard product
- Treasurer composes with (never replaces) the existing Commissary/TokenBudget middleware — architecturally distinct from every surveyed product, which conflates input/output rationing

**Explicitly out of scope (anti-features):**
- Pre-flight exact per-call cost prediction (impossible before response returns)
- Bundled/default price table (PRD already says nothing bundled)
- Billing/payment integration, hosted spend-visualization dashboard, retroactive allowance enforcement against historical runs

### Architecture Approach

The existing hexagonal shape is directly extensible: a new `TreasuryLedgerPort` in `paladin-ports` mirrors `RunRepositoryPort` exactly (same crate/directory/async-trait shape), with `InMemory`/`Sqlite`/`Postgres` adapters in `paladin-storage/src/treasury/` mirroring `paladin-storage/src/run/`, and migrations at `crates/paladin-storage/migrations/{postgres,sqlite}/007_create_treasury_ledger_table.sql` (the real, non-stale path — confirmed by direct filesystem read). The Treasurer service itself is a new facade-layer application service (`src/application/services/treasurer/`), composing ports rather than being one, sibling to `RunSubmissionService`/`RunWorkerPool`. Admission-time refusal is a new pre-insert step inside `RunSubmissionService::submit`, structurally identical to the existing SSRF guard call — this preserves the file's documented zero-engine-import invariant. Mid-run halt reuses the already-existing `RunStatus::Halted` terminal status (no new status needed) and Waypoint's automatic per-superstep checkpointing (no special resume handling beyond what cancellation already exercises).

**Major components:**
1. `TreasuryLedgerPort` (+ 3 adapters + migrations) — durable append-only spend ledger, reserve/settle shape, mirrors `RunRepositoryPort`
2. `Treasurer` facade service — pricing math, admission-time allowance check, mid-run halt trigger, installs/derives the enforcement budget
3. Pricing config (`src/config/treasurer.rs`) — operator-configured price table, empty default, mirrors `AgentRuntimeConfig`'s `Default`/`validate()`/`EnvOverridable` shape
4. Rate pacer — new `LlmPort`-wrapping decorator (sibling shape to `FallbackLlmAdapter`), in-process `governor` + Redis-shared Lua-atomic pacing/stampede-lock reusing `RunQueuePort`'s proven idiom

**Genuinely open architectural question (not resolved by the codebase or PRD):** the PRD's "Treasurer installs the per-run TokenBudget" language matches the `PaladinExecutionService` agent-loop shape, but `AgentRuntimeConfig::build_chain` has **zero production callers today**, and the primary v0.10.0 run path (`WarEngine`/`NodeSpec::Paladin`) bypasses the middleware chain entirely, executing Paladin nodes through the raw `PaladinPort`. Milestone planning must explicitly decide: wire `build_chain` into the engine path for the first time, or enforce mid-run halts at a different layer (e.g. off the engine's own aggregated `TokenUsage`/trace events) for engine-driven runs. Flagged as build-order-1, ADR-worthy.

### Critical Pitfalls

1. **Currency math in `f64`** — `ExecutionMetadata.cost_estimate` is already a public `f64` field; do all internal accumulation/comparison in integer/fixed-point, convert to `f64` only at the display boundary; never let allowance enforcement read `f64` as authoritative.
2. **Check-then-act double-spend across concurrent workers** — model allowance draws as one atomic reserve-then-settle operation (mirroring `RunQueuePort`'s Lua-atomic claim pattern), not a separate SELECT-then-UPDATE; write the "N-1 of N succeed" concurrency contract test first.
3. **Double-charging on lease redelivery** — give every ledger settle write an idempotency key derived from `(run_id, superstep_seq, node_attempt)`, settle within the same transaction as the terminal Waypoint/`RunFinished` event, not as an independent fire-and-forget side effect.
4. **Rolling-window math on local clocks/calendar arithmetic** — store everything UTC, use the database's/Redis server's own clock for window-boundary comparisons (mirroring the existing lease-expiry `D-08` pattern), never `chrono::Local` or worker-local wall-clock math; decide explicitly whether "rolling" means sliding or fixed-boundary.
5. **Streaming responses settled/checked against usage that doesn't exist yet** — admission checks must use an estimate (never completion-token counts); settle only from the terminal chunk's `usage` (Phase 31's contract); reuse `TokenBudget`'s existing mid-run cutoff mechanism for streaming halts rather than inventing a second one.
6. **Redis-shared pacing/stampede lock has no in-tree precedent** — reuse the run-queue's server-clock, fencing-token, compare-and-delete-on-owner-match idioms; add per-worker jitter on top of any shared delay; explicitly decide and test fail-open vs. fail-closed on Redis unavailability (recommend: degrade to conservative per-process pacing, never to unpaced).
7. **Clean-break legacy removal has a wider blast radius than the milestone description implies** — `RetryPolicy`/`ErrorStrategy` names collide with distinct, in-scope Aegis/Maneuver types of the same name; `conclave_execution_service.rs`'s string-substring `is_retryable_error` depends on `PaladinError::LlmError`'s stringly-typed shape without referencing it by name — a "rename, not fix" risk that would silently undermine R4's need for reliable structured 429 detection.

## Implications for Roadmap

Based on research (stack + architecture "Suggested Build Order" + pitfalls' per-item phase tags), suggested phase structure, starting at **Phase 38** per the operator's numbering:

### Phase 38: Design seams + pricing/cost producer (R1, R2)
**Rationale:** Two open design decisions (Treasurer's mid-run attachment point; derive-on-read vs. maintain-a-running-balance for allowance state) gate every later phase's schema/wiring and should be recorded (ADR/D-nn) before code. Pricing/cost is fully self-contained, has zero dependency on ledger/allowance work, and de-risks "populate a long-dead field" early.
**Delivers:** Recorded design decisions; `src/config/treasurer.rs` price table; pure `TokenUsage x PriceTable -> cost` function (integer/fixed-point internally); `ExecutionMetadata.cost_estimate` producer wired end-to-end, visible in heralds/CLI immediately.
**Addresses:** R1, R2 (table stakes)
**Avoids:** Pitfall 1 (f64 currency math), Pitfall 6 (unpriced-model None-vs-zero), Pitfall 8 (cache-token pricing direction)

### Phase 39: Spend ledger (R5)
**Rationale:** Ledger must exist before allowance enforcement can evaluate "already spent this window"; depends on Phase 38's cost function to have something to persist, and on Phase 38's schema decision (reserve/settle shape).
**Delivers:** `TreasuryLedgerPort` (paladin-ports) + in-memory/SQLite/Postgres adapters (paladin-storage) + migrations 007 (and 008 if a separate allowance-state table is chosen), reserve-then-settle write API, idempotency-keyed settlement.
**Uses:** `i64` fixed-point ledger column; `RunRepositoryPort`'s adapter/contract-test pattern
**Implements:** `TreasuryLedgerPort` + adapters component
**Avoids:** Pitfall 2 (double-spend races), Pitfall 3 (redelivery double-charge), Pitfall 7 (retry/fallback double-count spend)

### Phase 40: Tenant/API-key identity flow
**Rationale:** Admission-time allowance enforcement needs a real identity carrier; `Principal` currently has no tenant field and `list_runs` already discards its principal — this is a prerequisite for Phase 41 and the natural place to close the `GET /runs*` scoping gap (row 32) since it needs the same plumbing.
**Delivers:** Decision on minimal (API-key-only) vs. full (+ tenant_id) identity scope; if full, a scoped breaking change to `Principal`/auth config with MIGRATION.md row and semver-checks allowlist entry; `GET /runs*` per-caller scoping fix via one shared authorization function checked per route (not just the list endpoint).
**Avoids:** Pitfall 10 (secondary-route scoping bypass)

### Phase 41: Treasurer admission-time enforcement (R3, first half)
**Rationale:** Wires into `RunSubmissionService::submit` per the architecture research's exact integration point (new step before `repository.insert`, structurally identical to the existing SSRF guard); depends on Phases 39-40.
**Delivers:** `Treasurer::authorize_draw` refusal at admission, new `RunSubmissionError::AllowanceExhausted`-style variant, rolling-window + optional lifetime-cap evaluation against the ledger.
**Implements:** `Treasurer` facade service (admission slice)
**Avoids:** Pitfall 4 (clock-skew rolling windows)

### Phase 42: Treasurer mid-run halt (R3, second half)
**Rationale:** Depends on Phase 38's resolved attachment-point decision (agent-loop `TokenBudget` vs. engine-level hook — the single largest open architectural question this research surfaces). Bundle the SSE `done`/`Cancelled`-vs-`Halted` fix (D-14) into this same phase since it shares the exact code region.
**Delivers:** New `StopReason`/halt-reason variant, reuse of existing `RunStatus::Halted`, resumable checkpoint-preserving halt on overspend; SSE terminal-event fix.
**Avoids:** Pitfall 5 (streaming settle-on-nonexistent-usage) — reuse `TokenBudget`'s existing cutoff mechanism rather than a parallel one

### Phase 43: Rate pacing (R4/FUT-09)
**Rationale:** Independent of Phases 39-42 — can run in parallel once Phase 38's design seams are settled. Flagged by pitfalls research as the one sub-area with no in-tree precedent to imitate.
**Delivers:** `governor`-based in-process pacer as an `LlmPort`-wrapping decorator (composed alongside `FallbackLlmAdapter`, never retries itself); Redis-shared pacing state + stampede lock reusing `RunQueuePort`'s Lua-atomic/server-clock idiom; explicit, tested fail-open-vs-fail-closed decision on Redis unavailability; per-worker jitter.
**Research flag:** yes — see below.

### Phase 44: Legacy clean-break removal (X-03 supersession)
**Rationale:** Independent of the Treasurer feature work; sequence after Phases 38-43 to avoid rebasing removal diffs against concurrently Treasurer-touched files (`formation_service.rs`/`battalion/mod.rs` neighbors).
**Delivers:** Removal of legacy `battalion::RetryPolicy`/`ErrorStrategy`/`NodeError`/timeouts and `PaladinError::LlmError(String)`, disambiguated by full path against the distinct in-scope Aegis/Maneuver types of the same name; migration of `conclave_execution_service.rs`'s string-matching retryability logic onto the structured `LlmFailure`/`Transience` taxonomy (not just a rename); MIGRATION.md rows + semver-checks allowlist entries.
**Avoids:** Pitfall 12 (scope-creep / silent-rename-not-fix risk)

### Phase 45: RustFS swap + remaining platform deviations + docs/hygiene
**Rationale:** Fully independent of Treasurer feature surface; RustFS's four open questions (S3 API-surface parity, image/cadence maturity, licence, whether k8s production manifest follows) should be closed as this phase's first work item, before adapter code is written. Bundle `Runnable::Agent` SSE/webhook emission wiring, tracing-overhead fix (D-16), and mdBook/docs currency here as adjacent, independently schedulable hygiene.
**Delivers:** RustFS adapter (reuse-first: point existing `rust-s3` MinIO adapter at RustFS before writing a new one) behind its own feature flag, proven via the existing `FileStoragePort` contract-test suite; `Runnable::Agent` SSE/webhook wiring; `LogTraceSink` allocation/guard fix; docs updates.
**Avoids:** Pitfall 11 (RustFS-not-drop-in assumptions)

### Phase 46: v0.11.0 crates.io release
**Rationale:** Closing phase per operator decision; depends on all prior phases being merged and semver-checked.
**Delivers:** Release.

### Phase Ordering Rationale

- Pricing/cost (38) before ledger (39) before enforcement (41-42): each layer needs the one below it to have something meaningful to compute/persist/enforce against — this mirrors the architecture research's own numbered "Suggested Build Order."
- Identity flow (40) is sequenced before admission enforcement (41) because per-tenant/per-key allowance checks need a real identity carrier that doesn't exist in `Principal` today; bundling the `GET /runs*` fix here avoids re-deriving the same plumbing twice.
- Rate pacing (43) is deliberately parallel-schedulable, not blocking the Treasurer core, since it's an independent proactive layer that wraps the LLM port rather than touching admission/ledger code.
- Legacy removal (44) and RustFS/hygiene (45) are sequenced last among feature work specifically to avoid diff conflicts with the Treasurer phases still actively touching neighboring files.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 43 (rate pacing):** No adjacent in-tree pattern to imitate for the stampede lock (unlike the ledger/admission phases, which can mirror `RunRepositoryPort`/`RunQueuePort` directly); the fail-open-vs-fail-closed decision on Redis unavailability is a genuine open design question the PRD does not resolve. Pitfalls research explicitly recommends deeper phase-specific research here.
- **Phase 42 (mid-run halt) / Phase 38 (design seams):** The engine-path vs. agent-loop-path attachment-point question for `TokenBudget`/mid-run enforcement is the single largest open architectural question surfaced by this research; needs an ADR-level design pass before implementation, not just a plan.
- **Phase 45 (RustFS):** MEDIUM-confidence claims throughout (S3 API-surface parity for presigned URLs/multipart/ETag format, `mc`-equivalent tooling, image maturity) — the phase's first work item should close these via the existing contract-test suite before any adapter code is written.

Phases with standard patterns (skip research-phase):
- **Phase 39 (ledger):** Mirrors `RunRepositoryPort`'s already-proven three-adapter/contract-test pattern exactly.
- **Phase 41 (admission enforcement):** Mirrors the existing SSRF-guard integration shape in `RunSubmissionService::submit` exactly.
- **Phase 38 (pricing/cost):** Well-understood pure-function + config-struct pattern, matches `AgentRuntimeConfig`'s existing shape.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH for crate identity/version/license (verified against crates.io/GitHub, cross-checked against workspace `Cargo.toml`/`Cargo.lock`); MEDIUM for RustFS S3 API-parity (docs.rustfs.com/lib.rs proxy-blocked, verified only via README/release notes) and `governor`'s exact MSRV (not declared in its manifest) |
| Features | MEDIUM — web sources only, cross-checked across 3+ independent products per claim; no HIGH-tier official-docs source consulted for OpenAI/Anthropic exact rate-limit header names (directionally correct, worth a direct docs check before implementation) |
| Architecture | HIGH — every claim grounded in direct `Read`/`Grep` of the shipped tree at 2026-09-24 HEAD, not the PRD's aspirational text; file paths and line-anchored quotes given throughout |
| Pitfalls | HIGH — grounded directly in this repo's shipped code, PRD, ADRs, PROJECT.md; MEDIUM/LOW only for the RustFS-specific pitfall (externally sourced general S3-compatibility failure patterns, not confirmed against RustFS's actual current coverage) |

**Overall confidence:** HIGH for what to build and how it integrates with the existing codebase; MEDIUM for exact third-party version/API-surface details (RustFS parity, provider rate-limit header names) that the build-order itself already treats as "verify via contract test," not "trust as given."

### Gaps to Address

- **Where does mid-run Treasurer enforcement actually attach** (agent-loop `build_chain`/`TokenBudget` vs. a new engine-level hook)? No existing code answers this — architecture research flags it as the largest open design question; resolve via ADR/D-nn before Phase 42 planning, ideally during Phase 38.
- **Derive-on-read vs. maintain-a-running-balance** for allowance state — materially changes the ledger schema (whether an `allowance_windows` table is needed alongside the ledger); resolve during Phase 38's design-seam step, before Phase 39's migrations are written.
- **RustFS's actual S3 API-surface parity** (presigned URLs, multipart uploads, ETag format, `mc`-equivalent tooling) is unverified beyond README-level claims — Phase 45 must close this via the existing `FileStoragePort` contract-test suite as its first work item, not assume parity from a marketing claim.
- **Fail-open vs. fail-closed on Redis unavailability for rate pacing** — the PRD does not resolve this and no other subsystem in the codebase answers it identically (run-queue degrades to in-memory fallback; webhook SSRF guard fails closed) — needs its own explicit decision and test in Phase 43.
- **Exact OpenAI/Anthropic rate-limit header names** (`x-ratelimit-remaining-requests`, `anthropic-ratelimit-*`) were sourced via web search, not official docs directly — verify against docs.anthropic.com/platform.openai.com before Phase 43 implementation.
- **Blast radius of the legacy clean-break removal** is real but bounded per the architecture research's measured call-site counts — still, every migrated call site (especially `conclave_execution_service.rs`'s string-matching retryability logic) needs an explicit "migrated to structured taxonomy, not just renamed" verification pass during Phase 44, not just a compiler-driven mechanical deletion.

## Sources

### Primary (HIGH confidence)
- Direct `Read`/`Grep` of the shipped Paladin tree at 2026-09-24 HEAD (post-v0.10.1) — `src/config/agent_runtime.rs`, `src/application/services/paladin/middleware/limits.rs`, `src/application/services/run/{submission,worker,resolver}.rs`, `crates/paladin-web/src/{agent_auth,run_controller}.rs`, `crates/paladin-core/src/platform/container/{herald,run,battalion/mod,node_error,paladin_error}.rs`, `crates/paladin-ports/src/output/{paladin_port,llm_port,run_repository_port}.rs`, `crates/paladin-battalion/src/{retry,llm_failure,engine/*}.rs`, `crates/paladin-llm/src/{http_status,services/commissary}.rs`, `crates/paladin-storage/migrations/{postgres,sqlite}/`
- `.planning/PROJECT.md`, `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` (R1-R6, scope boundary)
- `deny.toml`, workspace `Cargo.toml`/`Cargo.lock` (license allow-list, MSRV, dependency promotion precedent)
- `paupino/rust-decimal` and `boinkor-net/governor` GitHub repos + crates.io metadata (fetched directly)
- `rustfs/rustfs` GitHub repository, README, `1.0.0` release tag, `docker-compose.yml` (fetched directly)

### Secondary (MEDIUM confidence)
- LiteLLM, Portkey, Helicone, Langfuse, LangSmith, OpenRouter docs/blogs (feature-landscape survey, cross-checked 3+ sources per claim)
- OpenAI/Anthropic rate-limit guidance via third-party engineering articles (Respan, SitePoint) — header names not independently verified against official docs
- SQLx maintainers' GitHub issue discussion on rejecting `rust_decimal`/`bigdecimal` for SQLite
- Redis Blog (rate-limiting/cache-stampede patterns)

### Tertiary (LOW confidence)
- `docs.rustfs.com` S3-compatibility-matrix claims — page itself proxy-blocked in this research environment; treat RustFS API-surface parity as unverified until the Phase 45 contract-test suite proves it

---
*Research completed: 2026-09-24*
*Ready for roadmap: yes*
