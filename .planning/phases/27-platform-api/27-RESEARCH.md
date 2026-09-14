# Phase 27: Platform API - Research

**Researched:** 2026-09-08
**Domain:** Rust hexagonal-architecture durable run server (async worker pool, SQL contract-tested
repositories, Redis lease queue, SSE bridging, cron scheduling, HMAC-signed webhooks, OpenAPI/SDK
generation) on top of an existing superstep engine.
**Confidence:** HIGH for everything backed by a direct read of vendored dependency source, this
repo's own `Cargo.lock`/`Cargo.toml`, or this repo's own `.rs` files (cited by path:line below).
MEDIUM/LOW is called out explicitly where a claim rests on the PRD/CONTEXT text alone and could not
be independently confirmed against code in this session.

<user_constraints>
## User Constraints (from CONTEXT.md)

`.planning/phases/27-platform-api/27-CONTEXT.md` locks **54 auto-selected decisions (D-01…D-54)**
across nine subsystems. They are BINDING — do not re-open them. This section gives the one-line
binding statement for each so the planner does not have to re-open the full file for orientation;
the full rationale, rejected alternatives and reversibility ratings live in 27-CONTEXT.md itself and
remain the authoritative text.

### Locked Decisions (condensed; see 27-CONTEXT.md for full text)

**Runs & persistence (PLAT-01):** D-01 `RunId`/`Run`/`RunStatus`/`AssistantRef` are new core types in
`paladin-core::platform::container::run` (`#[non_exhaustive]`, `schema_version`). D-02 the status
machine is a pure `RunStatus::try_transition` function, tested first. D-03 `RunRepositoryPort` in
`paladin-ports`, SQLite+Postgres+InMemory adapters in `paladin-storage` behind the *existing*
`sqlite`/`postgres` features, one shared contract suite mirroring `waypoint/contract_tests.rs`. D-04
transitions are compare-and-set (`UPDATE ... WHERE status = ?from`), never read-modify-write. D-05 no
separate status-history table — the Waypoint chain is the audit trail.

**Queue, leases, redelivery (PLAT-02):** D-06 `RunQueuePort` is a NEW port, `QueuePort` untouched.
D-07 the queue carries a pointer (`QueuedRun{run_id,...}`), not a payload; worker re-reads via
`RunRepositoryPort`. D-08 the Redis adapter uses a sorted-set visibility lease via Lua (`script`
feature already enabled), not Streams. D-09 redelivery always resumes (`WarEngine::resume`/
`resume_with`), never restarts. D-10 heartbeat = lease/4, derived not separately configured.

**Worker placement & shutdown:** D-11 worker pool lives in the facade
(`src/application/services/run/...`), never in `paladin-web` (ADR-0031). D-12 `paladin-web` writes
through a new `RunSubmissionPort`, reads `RunRepositoryPort` directly. D-13 workers are tokio tasks
registered with the existing `ShutdownCoordinator`.

**Cancellation (PLAT-FR-04):** D-14 new `CancellationProbe` trait (`paladin-ports`), infallible
`async fn is_cancelled(&self, thread: &ThreadId) -> bool`, consulted beside — not instead of — the
existing `CancellationToken`. D-15 debouncing is the adapter's job (`min_probe_interval_ms`), not the
engine's. D-16 cancel is persisted-flag-first, local-signal-second; engine returns
`RunOutcome::Halted`, run recorded `Cancelled`.

**Thread serialization (PLAT-FR-05):** D-17 `409 ThreadBusy` is a DB partial unique index
(`CREATE UNIQUE INDEX ... ON runs(thread_id) WHERE status IN (...)`), not an application check. D-18
`AwaitingInput` counts as busy (deliberate tightening of PRD's literal `Queued|Running`). D-19 resume
is exempt by construction — it `UPDATE`s the same `run_id`, never `INSERT`s.

**Parley integration (PLAT-03):** D-20 Phase 24's published `202` resume contract is kept verbatim;
only the mechanism behind `ParleyPort` changes to enqueue instead of spawning. D-21 the resume
response gains `run_id` (`ResumeAccepted`/`ResumeAcceptedResponse`, both `#[non_exhaustive]`,
registered in MIGRATION.md §9.2/§9.6). D-22 `AwaitingInput` releases the worker via ACK, never NACK.
D-23 one shared `attempt` counter for redelivery and resume.

**Streaming (PLAT-FR-07):** D-24 live path is a `TraceSink` adapter feeding a per-run
`tokio::sync::broadcast` bus (`RunEventBus`), fire-and-forget, drop-oldest. D-25 seven wire event
names frozen (`superstep`, `node_started`, `node_finished`, `state_delta`, `parley`, `done`, `error`)
mapped from today's `TraceEvent` in one function; unmapped variants dropped. **See Common Pitfalls —
this mapping is NOT sufficient as literally stated; two of the seven names have no `TraceEvent`
source today.** D-26 degraded mode polls `WaypointPort::history` at `run_stream.poll_interval`
(default 1s), documented as no-ordering-guarantee; 15s heartbeat comment lines on both paths. D-27
`paladin-web` consumes a `RunEventStreamPort` (input port), never owns the bus.

**Assistants (PLAT-04):** D-28 `AssistantDefinition{kind: Agent|Workflow, body: serde_json::Value}` —
a tagged envelope over opaque JSON in core; only the facade interprets it. D-29 immutability is
enforced by absent methods and absent routes (`append_version`/`get_version`/`list_versions`/
`soft_delete_assistant`; no `update_version`, no `PUT` route). D-30 `latest` resolved and frozen
inside the SAME transaction that inserts the run. D-31 validation is a facade service returning
`Vec<ValidationViolation>`; nothing persisted on failure; **for `Workflow`, compile IS the
validation.** D-32 code-registered agents exposed read-only as synthetic assistants
(`expose_code_registry: bool = true` default ON), `AgentRegistry` untouched.

**`WarGraphDoc` (PLAT-FR-12):** D-33 lives in `paladin-battalion` beside `WarGraph`,
`WarGraphDoc::compile(&EngineRegistries) -> Result<WarGraph, CompileError>`. **See Common Pitfalls —
the "existing registries" do not yet cover everything a general graph doc would need to compile; see
the NodeSpec/EngineRegistries gap below.** D-34 JSON Schema hand-authored + checked in, drift-guarded
by an example corpus validated via `jsonschema` as a dev-dependency (NOT currently resolved in
`Cargo.lock` — see Standard Stack). D-35 fingerprint stability proven across a real two-process
boundary, not a same-process round trip.

**Schedules (PLAT-05):** D-36 run schedules do NOT use `tokio-cron-scheduler`; new
`ScheduleRepositoryPort` (SQLite+Postgres) + one `ScheduleService`. D-37 a tick is claimed by a
conditional `UPDATE ... WHERE next_tick = ?next` (no leader election). D-38 cron parsed with
`croner`, both 5-field and 6-field forms accepted (confirmed API below), reusing the extracted
5↔6-field logic pattern from `scheduler.rs:59-95`. D-39 `thread_strategy`/`on_missed` are core enums
with PRD defaults; `FixedThread` onto a busy thread increments `skipped_ticks`.

**Webhooks (PLAT-FR-14/15):** D-40 delivery is a persisted queue drained by a service
(`next_attempt_at` column), not a spawned task. D-41 HMAC over the exact bytes sent, signed once from
one buffer, never re-serialized. D-42 SSRF guard is a standalone table-tested function at write AND
send time; webhook client follows no redirects (`reqwest::redirect::Policy::none()` — **already the
house pattern for every credential-bearing outbound client in this repo, see Code Examples**). DNS
rebinding pinning documented as a known limitation, not implemented. D-43 `4xx` dead-letters
immediately; `5xx`/timeout/connect-error retries 5x with 1s..60s backoff under `tokio::time::pause`.

**HTTP surface (PLAT-06):** D-44 one new `RunApiState`+`run_router` mirroring `ThreadApiState`;
`AgentApiState` untouched; unwired → `501 not_implemented`. D-45 `ThreadApiState` gains fields for
fork/delete, `#[non_exhaustive]` in the same change. D-46 two-tier auth (`authorize_invoke` for
run/resume/fork/cancel, `require_admin` for assistant/schedule mutation) — no new `UserRole` variant.
D-47 Phase 24's `limit`+opaque-cursor pagination applied verbatim everywhere. D-48 `openapi.json`
regenerated every PR (`make openapi`, confirmed mechanism below); golden-diff gate is SHIP-02's. D-49
one pinned `openapi-generator-cli` for both Python and TypeScript in a `sdk-clients` CI job running
on every PR (**Docker IS available on CI's `ubuntu-latest` runners — confirmed below**).

**Config/tests/bookkeeping:** D-50 six new config structs, all default OFF, mirroring
`src/config/waypoint_store.rs`'s exact tagged-enum shape (confirmed below). D-51 Tier 1 (local,
InMemory+SQLite+mockito+paused clock) vs Tier 2 (CI-only Redis/Postgres) test split, carried forward
verbatim from Phase 24 D-28. D-52 three named concurrency stress tests with exact counts. D-53
`MIGRATION.md` filled incrementally (file already exists and is actively maintained — confirmed
below). D-54 coverage floor 82% (confirmed current and correct below); doctest/`--tests` landmine
called out explicitly.

### Claude's Discretion

- Exact module and file names within the crates fixed above, and how the SQL migrations are split.
- Whether the run row's `webhook` spec is an inline JSON column or a joined table.
- Plan decomposition and ordering beyond PRD 06 §5's TDD sequence, and how many plans the phase
  takes.
- The precise `RunStreamEvent` payload fields, within the seven frozen event names (D-25).
- Whether `ipnet` is needed at all, or `std::net::IpAddr` classification suffices (D-42). **Research
  finding: `std::net::IpAddr` alone suffices at MSRV 1.88 — see Standard Stack.**
- Whether `run_status_history` is promoted to a real table if research surfaces an FR that reads it
  (D-05 leaves this additive).

### Deferred Ideas (OUT OF SCOPE)

The authoritative `TraceEvent` enum, `seq` ordering, OTel/structured-log sinks (OBS-01/02, Phase 28);
`WarGraphDoc → Mermaid/DOT` export (OBS-03, Phase 28); finer-grained scopes/RBAC; a multi-replica-safe
auth token store (ADR-0041 gap, no FR this phase); resolve-then-connect DNS-rebinding pinning;
`run_status_history` table; generating the JSON Schema with `schemars` instead of hand-authoring;
horizontal autoscaling/replica orchestration beyond a k8s doc example; hand-polished SDKs; resuming a
`Failed` thread; Parley propagation through nested Battalions; `MIGRATION.md` §9 finalisation / v0.9
boot test / openapi golden diff / program acceptance audit / version bump (SHIP-01…04, Phase 29).

</user_constraints>

## Project Constraints (from CLAUDE.md)

- Hexagonal dependency rule: core → nothing; ports → core; infrastructure adapters → core+ports;
  facade assembles. No new port trait imports an SDK/DB driver/HTTP client (X-01).
- TDD red-green-refactor; **coverage floor 82% workspace line coverage** (confirmed current binding
  value, not stale — see Common Pitfalls); doc tests required on all public APIs but do not count
  toward the coverage number.
- Medieval Military ubiquitous language used consistently (Battlefield, Waypoint, Parley, Aegis,
  Vault, Chronicle, Muster, Vanguard already exist from prior phases; this phase adds no new
  vocabulary term per the program overview's §4 table).
- Before committing a parent task: `cargo test` → `cargo fmt --check` → `cargo clippy -- -D
  warnings`, conventional-commit message, stop after each major task.
- Avoid `unwrap()`/`expect()`/`panic!` in library code; return `Result`. Prefer borrowing over
  cloning; keep iterators lazy.
- `make security` (cargo-audit + cargo-deny) required on new/modified code; CodeQL is
  advisory-only (not merge-gating) per `.github/instructions/security.instructions.md` — manual
  credential-handling review is the primary control. Directly binding here: the webhook client
  carries an HMAC signature and must not follow redirects (D-42); no secret may reach a webhook
  payload, trace event, run row, or error body; response bodies redacted before truncation.
- Do not reintroduce Snyk (no Rust coverage, evaluated and removed).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PLAT-01 | `POST /runs` 202 in 250ms p99, `RunRepositoryPort` (SQLite+Postgres), monotonic status machine | sqlx 0.8.6 confirmed resolved with `postgres` feature already gated (`crates/paladin-storage/Cargo.toml:25`); CAS pattern via `rows_affected()`; portable unique-violation detection via `DatabaseError::is_unique_violation()`/`kind()` (NOT `.constraint()`, which is Postgres-only) |
| PLAT-02 | Worker pool, `RunQueuePort` (InMemory+Redis), lease heartbeats, resumable redelivery, cross-instance cancellation, `409 ThreadBusy` | Redis `script` feature enabled but **currently unused anywhere in the codebase** (no Lua precedent exists — D-08 is new ground, not an established pattern); `ShutdownCoordinator` API confirmed (`engine/shutdown.rs:54,121,162`); cancellation boundary check located precisely (`engine/superstep.rs:1991-2022`) |
| PLAT-03 | `AwaitingInput` releases worker, resume re-enqueues same `run_id`, SSE bridge with degraded mode | `TraceEvent` enum has exactly 8 variants today, none is `parley` or `error` — the RunEventBus must synthesize those two wire events from `RunOutcome`/repository state, not from `TraceSink` alone (see Common Pitfalls); axum 0.8.4's `Sse::keep_alive(KeepAlive::new().interval(...))` API confirmed and **not currently used anywhere** in this codebase |
| PLAT-04 | Versioned immutable assistants, publish-time validation, `WarGraphDoc` compile+fingerprint | `AgentSpec`/`AgentProvisioner` (existing, `agent_registry.rs`) is a near-complete analog for Agent-kind validation; `PaladinConfigDoc` and `WarGraphDoc` do not exist as named types anywhere yet; `NodeSpec::Function(Arc<dyn StateNode>)` has **no name→factory registry** in `EngineRegistries` today — a general WarGraphDoc cannot resolve arbitrary Function nodes from JSON without new registry work (see Common Pitfalls) |
| PLAT-05 | Cron schedules restart-safe, HMAC webhooks with bounded retry, SSRF guard | `croner` 2.2.0 vendored source confirms exact API (`Cron::new(p).with_seconds_optional().parse()`, `find_next_occurrence(&DateTime<Tz>, bool)`); `reqwest::redirect::Policy::none()` is already the house pattern on every credential-bearing outbound client in `paladin-llm` (9 adapters); no existing SSRF guard code anywhere; `std::net::IpAddr`/`Ipv6Addr` classification methods (`is_private`, `is_loopback`, `is_unicast_link_local`, `is_unique_local`) are all stable well before MSRV 1.88 — `ipnet` is not needed |
| PLAT-06 | Auth/rate-limit/pagination on every endpoint, `openapi.json` regen, SDK generation CI job | `make openapi` mechanism confirmed exact (`UPDATE_OPENAPI=1 cargo test -p paladin-web --lib openapi_matches_committed_baseline`); CI's `ubuntu-latest` runners already run `docker compose` in 5 existing jobs — Docker IS available for an `openapi-generator-cli` container in the new `sdk-clients` job |

</phase_requirements>

## Summary

This phase is unusually well-specified before research even starts: 27-CONTEXT.md's 54 decisions
already resolve almost every architectural choice PRD 06 leaves open. The research value-add here is
narrow and concrete: (1) verify every dependency claim against this repo's actual `Cargo.lock` and
vendored source rather than training-data assumptions, (2) pin down exact engine seam locations with
line numbers so the planner can write precise task actions, and (3) surface the two places where a
locked decision's premise does not fully hold against the code as it exists today.

**Two premises need correction before planning, both with hard evidence:**

1. **The workspace MSRV is 1.88, not 1.85.** `Cargo.toml:18` (`rust-version = "1.88"`) and the CI
   `msrv` job (`RUSTUP_TOOLCHAIN: "1.88"`, `.github/workflows/ci.yml:252-255`) both confirm this. It
   was raised from 1.85 by a prior phase's D-10 (comment at `Cargo.toml:13-17`: `time >= 0.3.47`
   needs 1.88, one above what `rmcp`'s pinned `process-wrap` forces at 1.87). 27-CONTEXT.md D-53 and
   PRD 06's own X-11 citation both still say "1.85" — this is stale relative to the current tree, not
   a new finding this phase must debate; the planner should simply target 1.88 everywhere D-53 says
   1.85. (The coverage floor, by contrast, IS correctly 82% in both CLAUDE.md and the current
   `scripts/coverage.sh:35` — no correction needed there; see Common Pitfalls for why this looked
   contradictory in `ADR-0006`'s history.)
2. **`WarGraphDoc::compile()`'s "existing registries" do not cover named `Function` nodes.**
   `NodeSpec` (`engine/graph.rs:42-160`) has four variants — `Paladin`, `Function(Arc<dyn
   StateNode>)`, `Battalion` (nested graph), `Gate`. `EngineRegistries`
   (`engine/registries.rs:32-50`) only resolves `EdgeCondition::Custom`, `RetryPredicate::Custom`,
   `ErrorHandlerSpec::Custom`, and `SchemaRef::Registered` — there is no name→`Arc<dyn StateNode>`
   registry anywhere in the tree. A JSON document cannot describe an arbitrary `Function` node's
   Rust closure without one. The clean resolution (smallest change preserving D-33's intent): scope
   `WarGraphDoc`'s node-kind enum to `{Paladin, Gate, Workflow(nested doc)}` for v0.10 and document
   that a graph needing custom `Function`-node logic stays a code-registered assistant — this is a
   one-sentence JSON-Schema/rustdoc note, not new engine work, and it does not contradict anything
   PRD 06 §2.3 states (its example nodes are "kind/config/aegis", never showing a Function node).

**Primary recommendation:** Treat 27-CONTEXT.md's 54 decisions as settled; spend the plan's design
effort on the two corrections above, on the `TraceEvent`→wire-event gap (Common Pitfalls), and on
reusing the four house patterns confirmed below (waypoint contract-suite shape, tagged-enum config
structs, `Policy::none()` redirect discipline, `tower::oneshot` controller tests) rather than
re-deriving them.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Run submission/status/cancel HTTP surface | API/Backend (`paladin-web`) | — | New `RunApiState`+`run_router`; writes through `RunSubmissionPort`, reads `RunRepositoryPort` directly (D-12) |
| Run persistence & status machine | Database/Storage (`paladin-storage`) + Core (`paladin-core`) | — | `RunRepositoryPort` adapters own persistence; the pure status machine is core logic with no I/O (D-01, D-02) |
| Worker pool / engine driving | Backend facade (`src/application/services/run`) | — | ADR-0031 forbids `paladin-web → paladin-battalion`; only the facade sees the engine, queue, and repository together (D-11) |
| Queue (lease/redelivery) | Database/Storage (Redis) + Core port | — | `RunQueuePort` in `paladin-ports`, InMemory+Redis adapters in `paladin-storage` (D-06, D-08) |
| Cross-instance cancellation | Backend facade (probe adapter) + Engine seam | Database (persisted flag) | Engine consults an infallible probe at superstep boundaries; the facade adapter owns debouncing and DB reads (D-14, D-15) |
| SSE run streaming | API/Backend (`paladin-web` SSE framing) | Backend facade (`RunEventBus`) | `paladin-web` only frames SSE and heartbeats; the facade owns the live/degraded decision and the bus (D-24, D-27) |
| Assistants (versioned config) | Database/Storage + Backend facade (validation/compile) | API/Backend (routes) | Storage is append-only rows; only the facade can compile a `Workflow` body (D-28, D-31, D-33) |
| Schedules | Database/Storage + Backend facade (tick claim/dispatch) | API/Backend (routes) | Conditional-`UPDATE` tick claim is a DB-level primitive; the service layer submits the resulting run (D-36, D-37) |
| Webhooks | Backend facade (delivery service) + Database (queue table) | API/Backend (read-only deliveries endpoint) | Delivery is a persisted, drained queue for restart durability, not a spawned task (D-40) |
| OpenAPI/SDK generation | CI pipeline | API/Backend (spec source) | The spec is generated from `paladin-web`'s `utoipa` annotations; the SDK job is a CI concern only (D-48, D-49) |

## Standard Stack

### Core

| Library | Version (resolved) | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `hmac` | 0.12.1 `[VERIFIED: Cargo.lock + package-legitimacy OK, 11.0M weekly downloads, RustCrypto/MACs]` | HMAC-SHA256 webhook signatures (D-41) | Already resolved transitively via `sqlx-mysql`/`sqlx-postgres`/`rust-s3`; promoting to a direct dependency of the facade adds no new crate name to the tree and cannot change the resolved version (semver-compatible with every existing consumer) |
| `croner` | 2.2.0 `[VERIFIED: vendored source read + Cargo.lock + package-legitimacy OK, 233K weekly downloads, hexagon/croner-rust]` | Cron parsing + next-occurrence computation (D-38) | Already resolved transitively via `tokio-cron-scheduler`; exact API confirmed by direct source read (below) |
| `jsonschema` | **NOT in `Cargo.lock` today** `[VERIFIED: grep of Cargo.lock — absent; cargo info confirms 0.55.0 latest on crates.io, package-legitimacy OK, 1.57M weekly downloads, Stranger6667/jsonschema, rust-version 1.85.0]` | Validate `WarGraphDoc` example fixtures against the checked-in JSON Schema (D-34, dev-dependency only) | **Correction to 27-CONTEXT.md's "already in Cargo.lock" claim** (code_context section, "Dependency head start"): this one is NOT currently resolved anywhere in the tree, transitively or otherwise. It must be added as a genuinely new dev-dependency and run through `cargo msrv verify` at 1.88 for the first time — budget for that in the plan rather than assuming it is free. Its own declared `rust-version` (1.85.0) is below our 1.88 floor, so no MSRV conflict is expected, but it has not been proven against this workspace's dependency graph before. |
| `ipnet` | 2.12.0 (resolved, via `hyper-util`) — **likely not needed** `[VERIFIED: Cargo.lock + std::net probe on this toolchain]` | SSRF guard IP classification (D-42, discretion item) | **Discretion resolved:** `std::net::Ipv4Addr::{is_private, is_loopback, is_link_local}` and `Ipv6Addr::{is_loopback, is_unspecified, is_unique_local, is_unicast_link_local}` are all stable `std` methods; `is_unique_local`/`is_unicast_link_local` stabilized via rust-lang/rust#129238 well before this workspace's 1.88 MSRV `[CITED: github.com/rust-lang/rust/pull/129238]`. No crate dependency is needed for the SSRF guard's classification logic — only `169.254.169.254` (metadata) and the loopback/link-local/private/unique-local checks above, all coverable from `std` alone. |
| `sqlx` | 0.8.6, `postgres` feature already gated | `RunRepositoryPort`/`ScheduleRepositoryPort` SQLite+Postgres adapters | `[VERIFIED: crates/paladin-storage/Cargo.toml:25]` — `postgres = ["dep:sqlx", "sqlx/postgres"]` already exists exactly as D-03 describes; no new crate name, no Cargo.toml edit needed beyond enabling the feature on new code |
| `redis` | 0.32.2, `script` feature already enabled | Sorted-set lease queue via Lua `EVAL` (D-08) | `[VERIFIED: crates/paladin-storage/Cargo.toml:59]` feature present — **but see Common Pitfalls: no Lua/`EVAL` usage exists anywhere in this codebase today.** The feature is available, unused; this adapter is new ground, not a documented pattern extension. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `schemars` | 1.2.1 **direct dependency of the facade already** `[VERIFIED: Cargo.toml:143, "schemars = \"1.2\""]` | JSON Schema derivation, already used for structured-output (RT-05, Phase 26) | **Correction to D-34's premise:** the rejected-alternative note ("`schemars` is only in `Cargo.lock` transitively today") is no longer accurate — Phase 26 promoted it to a direct workspace dependency for `StructuredExecutorPort`. This weakens (does not invalidate) D-34's dependency-growth argument for hand-authoring the `WarGraphDoc` schema instead of deriving it; the decision itself (hand-author + drift-guard corpus) is still sound on its own merits (a document published once, versus a derive macro on a public type), just note in the plan that "adding schemars would grow the dependency graph" is no longer the load-bearing part of the rationale. |
| `mockito` | resolved (used by `paladin-llm`) | Webhook delivery HTTP mocking (D-43 acceptance test) | `[VERIFIED: crates/paladin-llm/src/conformance.rs:25]` — already the house pattern for outbound-HTTP test doubles; reuse directly, do not introduce `wiremock` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `croner` | `cron` crate | `croner` already resolved transitively (zero new tree cost) and its `find_next_occurrence` signature matches D-38's needs exactly; `cron` would be a genuinely new dependency for no gain |
| `std::net::IpAddr` classification | `ipnet` | `ipnet` adds a crate for functionality `std` already provides at this MSRV; only worth it if the guard needs CIDR *range* arithmetic beyond simple classification (it does not, per PRD 06 PLAT-FR-15's fixed list) |
| Hand-authored `WarGraphDoc` schema (D-34) | `schemars::schema_for!` derive | Now cheaper than D-34 assumed (schemars is already a direct dependency) but still couples a public wire-format schema to a Rust type's derive output rather than an explicit, reviewable JSON document — D-34's decision stands on documentation-quality grounds independent of the dependency-cost argument |

**Installation:**
```bash
# hmac, croner, ipnet (if used) already resolve from the existing lockfile once promoted
# to direct dependencies in the relevant crate's Cargo.toml — no `cargo add` version
# pinning surprises expected since the versions above are what the workspace already
# resolves to.
cargo add hmac --package <facade-or-storage-crate>
cargo add croner --package paladin-storage
# jsonschema is NEW — not currently resolved anywhere. Run cargo msrv verify after adding.
cargo add jsonschema --package paladin-battalion --dev
```

**Version verification:** confirmed live against this workspace's actual `Cargo.lock` and vendored
registry cache (`/usr/local/cargo/registry/src/...`), not training-data recall. `cargo info <pkg>`
and direct source reads were used in place of `npm view`/`pip index` (this is a Rust workspace).

## Package Legitimacy Audit

Ecosystem: `crates` (Rust/cargo). Checked via the package-legitimacy seam
(`gsd-tools query package-legitimacy check --ecosystem crates hmac croner ipnet jsonschema
schemars`).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `hmac` | crates.io | published 2016-10-06 (~10 yrs) | 11,086,653/wk | github.com/RustCrypto/MACs | OK | Approved |
| `croner` | crates.io | published 2023-11-08 (~3 yrs) | 233,617/wk | github.com/hexagon/croner-rust | OK | Approved |
| `ipnet` | crates.io | published 2017-08-14 (~9 yrs) | 10,685,439/wk | github.com/krisprice/ipnet | OK | Approved (but likely unneeded — see Standard Stack) |
| `jsonschema` | crates.io | published 2020-03-29 (~6 yrs) | 1,571,639/wk | github.com/Stranger6667/jsonschema | OK | Approved |
| `schemars` | crates.io | published 2019-08-08 (~7 yrs) | 12,350,808/wk | github.com/GREsau/schemars | OK | Approved (already a direct dependency) |
| `chrono-tz` | crates.io | published 2016-10-08 (~10 yrs) | 2,584,730/wk | github.com/chronotope/chrono-tz | OK | Approved — added at plan time (2026-09-08) because D-38's optional IANA timezone needs it and it is **not** in `Cargo.lock` (0 occurrences); genuinely new dependency, `cargo msrv verify` at 1.88 required (plan 27-11) |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none. All five packages are long-established,
high-download, source-repo-linked crates. No `checkpoint:human-verify` gate is required for any of
them on legitimacy grounds — `jsonschema`'s **novelty to this workspace's dependency graph** (not its
legitimacy) is the only reason to budget explicit `cargo msrv verify` time for it, called out in
Standard Stack above.

## Architecture Patterns

### System Architecture Diagram

```
                          ┌─────────────────────────────────────────────┐
                          │            paladin-web (API tier)            │
                          │  RunApiState/run_router  ThreadApiState+     │
                          │  (POST /runs, GET /runs/{id}, /cancel,       │
                          │   /stream, /resume, /fork, /assistants/*,    │
                          │   /schedules/*, /webhook-deliveries)         │
                          └───────────┬───────────────────┬─────────────┘
                       RunSubmissionPort              RunRepositoryPort
                         (writes: submit/           (reads: GET /runs/{id},
                          cancel/resume-adjacent)     list endpoints)
                                  │                           │
                                  ▼                           ▼
                  ┌───────────────────────────────────────────────────────┐
                  │        Facade (src/application/services/run/…)         │
                  │  RunSubmissionService ──▶ RunRepositoryPort (insert,   │
                  │       │                    freeze assistant version)   │
                  │       ▼                                                │
                  │  RunQueuePort.enqueue(QueuedRun{run_id,thread_id})     │
                  └───────────────────────────┬───────────────────────────┘
                                               │ dequeue(lease)
                                               ▼
                  ┌───────────────────────────────────────────────────────┐
                  │              RunWorkerPool (tokio tasks,               │
                  │              registered with ShutdownCoordinator)       │
                  │  1. re-read Run via RunRepositoryPort                   │
                  │  2. branch on latest Waypoint:                         │
                  │       absent      → WarEngine::start                   │
                  │       pending resp→ WarEngine::resume_with              │
                  │       otherwise   → WarEngine::resume                   │
                  │  3. extend_lease() every lease/4                       │
                  │  4. CancellationProbe consulted at superstep boundary   │
                  │     (engine/superstep.rs:1991, beside the existing     │
                  │      CancellationToken check)                          │
                  │  5. on RunOutcome: map → RunStatus, ack/nack queue,     │
                  │     enqueue webhook delivery row if terminal/Awaiting   │
                  └──────┬──────────────────┬───────────────┬─────────────┘
                         │                  │                │
                TraceSink (live)   RunOutcome (terminal    WebhookDeliveryService
                         │          /parley signal)         (persisted queue,
                         ▼                  │                drained on interval,
                ┌─────────────────┐         │                HMAC-signed, SSRF-
                │   RunEventBus    │◀────────┘                guarded, no redirects)
                │ (broadcast, per  │
                │  run, drop-oldest)│
                └────────┬─────────┘
                         │ RunEventStreamPort
                         ▼
              GET /runs/{id}/stream (SSE, 15s heartbeat,
              degraded-mode fallback polls WaypointPort::history)

        ┌──────────────────────────────┐      ┌───────────────────────────┐
        │   ScheduleService (tick loop) │      │  Assistants (facade)       │
        │  claim: UPDATE ... WHERE      │      │  Agent → AgentProvisioner   │
        │  next_tick = ?next            │      │    .provision (existing)    │
        │  → submits a Run through the  │      │  Workflow → WarGraphDoc     │
        │    same RunSubmissionService  │      │    .compile(&EngineRegistries)│
        └──────────────────────────────┘      └───────────────────────────┘
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
└── run.rs                      # RunId, Run, RunStatus, AssistantRef (D-01)

crates/paladin-ports/src/output/
├── run_repository_port.rs      # RunRepositoryPort (D-03)
├── run_queue_port.rs           # RunQueuePort, QueuedRun, LeaseToken, QueueError (D-06, D-07)
├── cancellation_probe.rs       # CancellationProbe trait (D-14)
├── run_event_stream_port.rs    # RunEventStreamPort (input side per D-27's own note; verify
│                                #   crate placement against ADR-0015's ports dependency allowlist)
└── schedule_repository_port.rs # ScheduleRepositoryPort (D-36)

crates/paladin-storage/src/
├── run/{mod,in_memory,sqlite,postgres,contract_tests}.rs   # mirrors waypoint/ (D-03)
├── run_queue/{in_memory,redis}.rs                          # mirrors node_cache/ shape (D-06/08)
└── schedule/{mod,sqlite,postgres,contract_tests}.rs        # mirrors waypoint/ (D-36)

crates/paladin-battalion/src/engine/
└── graph_doc.rs                # WarGraphDoc + compile() (D-33)

src/application/services/run/   # RunWorkerPool, RunSubmissionService, RunEventBus,
                                 # WebhookDeliveryService, ScheduleService (D-11)

src/config/
├── run_store.rs / run_queue.rs / run_worker.rs
├── assistants.rs / schedules.rs / webhooks.rs   # all mirror waypoint_store.rs's tagged-enum shape

crates/paladin-web/src/
├── run_controller.rs            # RunApiState + run_router (D-44)
└── assistant_controller.rs      # (or folded into run_controller — Claude's Discretion)
```

### Pattern 1: Compare-and-set status transition (D-04)

**What:** Every status write is `UPDATE runs SET status = ?to WHERE run_id = ?id AND status = ?from`;
zero `rows_affected()` means the transition was illegal (someone else already moved it, or it never
was `?from`).
**When to use:** Every `RunStatus` transition, every schedule tick claim (D-37).
**Example (portable across SQLite/Postgres — both `execute()` return the same `rows_affected()`
shape in sqlx 0.8):**
```rust
// Source: pattern generalized from the existing rows_affected() usage at
// crates/paladin-storage/src/waypoint/sqlite.rs:424-431 and postgres.rs:398-406
// (delete_thread/delete_waypoint), applied to a CAS UPDATE instead of a DELETE.
let result = sqlx::query(
        "UPDATE runs SET status = ?, started_at = ? WHERE run_id = ? AND status = ?"
    )
    .bind(to.as_str())
    .bind(at)
    .bind(run_id.to_string())
    .bind(from.as_str())
    .execute(&self.pool)
    .await
    .map_err(|e| self.wrap_error(e))?;

if result.rows_affected() == 0 {
    return Err(RunRepositoryError::IllegalTransition { from, to });
}
```

### Pattern 2: Portable unique-violation detection (D-17) — NOT what the existing house pattern does

**What:** `sqlx::Error::as_database_error()` → `DatabaseError::is_unique_violation()` (backed by
`ErrorKind::UniqueViolation`), never `.constraint()`.
**When to use:** Mapping the `runs(thread_id) WHERE status IN (...)` partial-unique-index violation
to `RunRepositoryError::ThreadBusy`.
**Why this needs new code, not a copy of the existing pattern:** the waypoint adapters' `wrap_error`
helper (`sqlite.rs:146-155`, `postgres.rs:130-139`) collapses **every** `sqlx::Error` into one
`WaypointError::Backend { source }` — because Waypoints never needed to distinguish a constraint
violation from any other DB failure. The run repository is the first adapter in this tree that needs
to. Critically, `DatabaseError::constraint()`'s own doc comment states:
```
/// Returns the name of the constraint that triggered the error, if applicable.
/// ### Note
/// Currently only populated by the Postgres driver.
fn constraint(&self) -> Option<&str> { None }
```
`[VERIFIED: /usr/local/cargo/.../sqlx-core-0.8.6/src/error.rs:233-238]` — SQLite's `DatabaseError`
impl never populates it (`sqlx-sqlite-0.8.6/src/error.rs` has no `constraint()` override at all). The
portable check is `.kind()`:
```rust
// Source: sqlx-sqlite-0.8.6/src/error.rs:122-129 (SQLITE_CONSTRAINT_UNIQUE |
// SQLITE_CONSTRAINT_PRIMARYKEY => ErrorKind::UniqueViolation) and
// sqlx-postgres-0.8.6/src/error.rs:211-217 (SQLSTATE 23505 => ErrorKind::UniqueViolation).
match err {
    sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
        Err(RunRepositoryError::ThreadBusy { thread_id })
    }
    other => Err(self.wrap_error(other)),
}
```
This works unambiguously as long as the `runs` table has exactly ONE unique index besides its
primary key (true under D-17's design). If a future phase adds a second unique index to `runs`,
`is_unique_violation()` alone can no longer disambiguate on SQLite (no constraint name available) —
document this coupling in the adapter's rustdoc.

### Pattern 3: Croner cron parsing (D-38) — exact confirmed API

**What:** `croner::Cron` accepts 5 OR 6 whitespace-separated fields once `.with_seconds_optional()`
is set; without it, 6 fields are rejected outright.
**Example:**
```rust
// Source: /usr/local/cargo/registry/src/.../croner-2.2.0/src/lib.rs (doc example, lines 20-31)
// and src/pattern.rs:36-100 (field-count acceptance logic), read directly from the vendored crate.
use chrono::{DateTime, Utc};
use croner::Cron;

let cron: Cron = Cron::new(&schedule.cron_expression)
    .with_seconds_optional()   // accepts BOTH 5-field and 6-field forms (D-38's requirement)
    .parse()
    .map_err(|e: croner::errors::CronError| ScheduleError::InvalidCron(e.to_string()))?;

// inclusive=false: never returns `now` itself, matching "next tick strictly after now" semantics
let next: DateTime<Utc> = cron
    .find_next_occurrence(&Utc::now(), false)
    .map_err(|e| ScheduleError::InvalidCron(e.to_string()))?;
```
`CronError` (from `errors.rs`) is a plain `#[derive(Debug)]` enum implementing `std::error::Error` —
**not** `thiserror`-derived and **not** `Clone`/`PartialEq`. Wrap it into the workspace's own
`thiserror` `ScheduleError` at the boundary (X-06 compliance is about *this workspace's* error types,
not croner's).

### Pattern 4: Config struct tagged-enum shape (D-50) — exact house template

**What:** Every new config struct should mirror `src/config/waypoint_store.rs` field-for-field.
**Example:**
```rust
// Source: src/config/waypoint_store.rs:19-49, read directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum RunStoreBackend {
    Disabled,                              // the default — X-09
    Sqlite { path: String },
    Postgres { url_env: String },          // NAME of the env var, never the URL itself —
                                            // never persisted/Debug-printed with the secret inline
}
```
This is the exact shape D-50 specifies for `run_store.rs`; the same tagged-enum + `url_env`
indirection applies to any future Postgres-backed config (`schedule_store` if split out, etc.).

### Pattern 5: No-redirect outbound HTTP client (D-42) — already the house pattern, not new

**What:** `reqwest::ClientBuilder::redirect(reqwest::redirect::Policy::none())` on any client that
sends a credential header.
**Example:**
```rust
// Source: crates/paladin-llm/src/openai/adapter.rs:253, deepseek/adapter.rs:347,
// anthropic/adapter.rs:163, gemini/adapter.rs:307 — identical call in all nine LLM adapters,
// each carrying an API-key header. This is the established house convention D-42 extends,
// not a new pattern this phase invents.
let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()?;
```
Apply the identical builder call to the webhook delivery client — it carries the HMAC signature
header, which is exactly the class of credential this pattern exists to protect.

### Anti-Patterns to Avoid

- **Copying `wrap_error`'s "collapse every sqlx::Error into one generic variant" pattern onto the run
  repository:** it silently loses the `ThreadBusy` signal on SQLite (see Pattern 2). Check
  `is_unique_violation()` BEFORE falling through to the generic wrap.
- **Assuming `WarGraphDoc` can describe an arbitrary `Function` node:** there is no registry for one;
  see the Summary's second correction and the Common Pitfalls entry below.
- **Building the `parley`/`error`/`done` SSE events purely from a `TraceEvent` mapping function:** two
  of the seven frozen wire names have no source event today (see Common Pitfalls).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cron next-occurrence computation | A custom cron parser/iterator | `croner::Cron::find_next_occurrence` | Already resolved in the lockfile, handles DST/timezone-aware chrono types, and its 5-vs-6-field acceptance logic is exactly D-38's requirement — confirmed by direct source read, not assumed |
| HMAC signing | Manual SHA-256 + constant-time compare | `hmac` crate's `Hmac<Sha256>` (RustCrypto) | Constant-time verification and correct key-handling are exactly the class of crypto primitive this workspace's own conventions forbid hand-rolling |
| IP classification for SSRF guard | A custom CIDR-matching table | `std::net::{Ipv4Addr, Ipv6Addr}` methods | Confirmed stable at this MSRV; a hand-rolled RFC1918/link-local table is exactly the kind of "reinvented, subtly wrong" logic std already gets right |
| Unique-constraint-backed invariants | Application-level check-then-insert | DB partial unique index + `is_unique_violation()` mapping | D-17's own rationale: a check-then-insert cannot hold under concurrent submits or across instances; this is not new guidance, just reinforcing the locked decision with the exact portable detection mechanism |
| JSON Schema validation of fixtures | A hand-rolled JSON structural walker | `jsonschema` crate (dev-dependency) | Purpose-built, widely used (1.57M dl/wk), and the whole point of D-34's drift guard is testing against the REAL JSON Schema semantics, not an approximation |

**Key insight:** every "don't hand-roll" item above already has either (a) a crate already resolved
in this workspace's dependency graph, or (b) a `std` capability confirmed stable at this MSRV. There
is no case in this phase where reaching for a library costs a genuinely new dependency-tree entry
except `jsonschema` (a dev-only, well-audited exception).

## Common Pitfalls

### Pitfall 1: D-25's seven-event mapping is under-specified against the real `TraceEvent` enum

**What goes wrong:** A plan that treats "map `TraceEvent` to the seven wire names in one function" as
a self-contained, mechanical task will discover mid-implementation that two of the seven names —
`parley` and `error` — have no corresponding `TraceEvent` variant to map from.
**Why it happens:** `TraceEvent` (`crates/paladin-ports/src/output/trace_sink_port.rs:67-152`) has
exactly eight variants today: `RunStarted`, `SuperstepStarted`, `NodeStarted`, `NodeFinished`,
`DeltaMerged`, `WaypointSaved`, `RunFinished`, `FallbackHop`. None of them carries a Parley/
AwaitingInput signal, and none carries an error/failure payload — `RunFinished` is emitted
unconditionally on both success and failure paths (`engine/mod.rs:1830-1831` for `start`; identical
shape in `resume`/`resume_with`) with no field distinguishing the two. `[VERIFIED: direct source
read of both files]`
**How to avoid:** Design the `RunEventBus` (D-24) so the worker itself — which owns the `RunOutcome`
returned by `WarEngine::start`/`resume`/`resume_with` — publishes the terminal wire events directly:
`RunOutcome::Completed → done`, `RunOutcome::Failed → error`, `RunOutcome::AwaitingInput → parley`
(carrying the `parleys: Vec<ParleyRequest>` the outcome already provides), `RunOutcome::Halted → done`
(a cancelled run still terminates the stream cleanly). Only the four live-progress wire events
(`superstep`, `node_started`, `node_finished`, `state_delta`) should come from the actual `TraceSink`
mapping function D-25 describes. This is a smaller, more precise version of D-24/D-25's intent, not a
contradiction of it — but a plan that assumes ALL seven come from one `TraceEvent`-mapping function
will hit a wall partway through.
**Warning signs:** A task description that says "map every TraceEvent variant to a wire event name"
without separately addressing where `parley`/`error`/`done` come from.

### Pitfall 2: `WarGraphDoc::compile()` cannot resolve a named `Function` node

**What goes wrong:** A plan that writes `WarGraphDoc`'s node-kind enum with a generic `Function{name:
String}` variant and expects `compile()` to "resolve it through the existing registries" (D-33's
literal wording) will find no registry to resolve it against.
**Why it happens:** `EngineRegistries` (`engine/registries.rs:32-50`) resolves exactly four
vocabularies: `edge_evaluators`, `retry_predicates`, `error_handlers`, `output_schemas`. None of them
maps a name to `Arc<dyn StateNode>`. `DispatchRegistry` (`engine/dispatch_registry.rs`) is a SEPARATE
field on `WarEngine` (not part of `EngineRegistries`) resolving `DispatchRule::Custom` only.
`[VERIFIED: direct source read of both files, plus engine/mod.rs:1793 confirming `dispatch_registry`
is threaded independently of `self.registries`]`
**How to avoid:** Scope `WarGraphDoc`'s node-kind vocabulary to `{Paladin, Gate, Workflow}` (a nested
`WarGraphDoc` compiles to a `NodeSpec::Battalion`) for v0.10. State this explicitly in the JSON Schema
and rustdoc as the documented boundary between data-driven and code-registered assistants — a graph
needing custom Rust logic in a node stays a code-registered `AgentRegistry` entry (which already
exists, D-32) rather than an `assistants` API-created one. This is the smallest change that preserves
D-33's actual intent (a document-format assistant compiles through the real engine's registries) — it
just narrows the document format's node vocabulary to what a registry-resolving `compile()` can
actually honor today, rather than growing new registry infrastructure mid-phase.
**Warning signs:** A `WarGraphDoc` schema draft with a `"kind": "function"` node type before a
corresponding registry design exists.

### Pitfall 3: MSRV citations in the locked decisions say 1.85; the tree is at 1.88

**What goes wrong:** A plan or CI check written to "verify at MSRV 1.85" (D-53's literal text, PRD 06
program overview X-11's literal text) will diverge from the ACTUAL `msrv` CI job, which is pinned to
1.88.
**Why it happens:** A prior phase (D-10, cited in `Cargo.toml:13-17`) already raised the workspace
MSRV to 1.88 because `time >= 0.3.47` (a RUSTSEC fix) needs 1.88, one above what `rmcp`'s pinned
`process-wrap ^9.0` forces at 1.87. `27-CONTEXT.md` and `06-platform-api.md` were both written against
the OLDER 1.85 figure and were not updated when the MSRV moved.
**How to avoid:** Every `cargo msrv verify` invocation, every "confirm at 1.85" instruction in D-53,
and every MIGRATION.md §9.3 entry this phase writes should read 1.88, matching
`.github/workflows/ci.yml:251-269`'s actual `msrv` job. This is a one-word correction, not a design
question.
**Warning signs:** Any new code or doc comment in this phase's plans that says "1.85".

### Pitfall 4: The Redis lease adapter has no Lua precedent to extend — it is greenfield

**What goes wrong:** Believing D-08's "the Lua/`scan_match` house style established by the Phase 25
node-cache adapter" means there is existing `EVAL`/`redis::Script` code to copy and adapt.
**Why it happens:** `node_cache/redis.rs` uses `scan_match` (a plain `SCAN`-based iteration) but
contains **no** Lua scripting at all. `[VERIFIED: grep for "redis::Script", "Script::new", ".eval(",
"EVALSHA" across crates/paladin-storage/src/ and src/ — zero matches anywhere in the tree]` The
`script` feature flag on the `redis` crate dependency is enabled but has never been exercised.
**How to avoid:** Budget real design/test time for the ZSET+Lua claim-and-expire script — it is new
territory for this codebase, not an extension of a working example. The `redis 0.32.2` `Script`
API (`redis::Script::new(lua_src).key(k).arg(a).invoke_async(&mut conn)`) is the right shape to
research against `redis` crate docs directly when that plan is written, since no in-repo example
exists to lean on.
**Warning signs:** A task description that says "extend the existing Lua pattern" for the queue
adapter.

### Pitfall 5: doctest / zero-test-selection coverage gaps

**What goes wrong:** Treating a passing `cargo test --doc` or a narrowly-filtered `cargo test
<pattern>` as coverage evidence for a plan's verification step.
**Why it happens:** `cargo llvm-cov` (as invoked by `scripts/coverage.sh:101`, no `--doctests` flag)
does not instrument doc tests, so a doc-tested public API can show 0% line coverage on the very lines
its own doc example exercises — the doc test still must exist (X-02) but cannot be cited as the
coverage evidence. Separately, a `cargo test <filter>` that matches zero tests exits 0, which can make
a broken or typo'd test filter look like a passing verification step.
**How to avoid:** Cite non-doctest `#[test]`/`#[tokio::test]` runs as coverage evidence; sanity-check
a new test filter actually selects tests (`--list` or a non-zero test count in the output) before
trusting a green run.
**Warning signs:** A plan's verification section citing only a doctest or an unconfirmed test-count
filter as evidence coverage moved.

## Code Examples

### SSE with 15-second keep-alive heartbeats (D-26) — confirmed axum 0.8 API, not yet used anywhere

```rust
// Source: /usr/local/cargo/registry/src/.../axum-0.8.9/src/response/sse.rs:74,517-547
// (Sse::keep_alive, KeepAlive::new().interval()), confirmed by direct source read.
// paladin-web resolves axum 0.8.4 -> 0.8.9 (crates/paladin-web/Cargo.toml:18).
// No existing call site in this codebase uses .keep_alive() today (agent_controller.rs's
// SSE stream at line 558/579 calls Sse::new(...).into_response() with no keep-alive at all) —
// this is new wiring, not a copy of an existing pattern.
use axum::response::sse::{Sse, KeepAlive};
use std::time::Duration;

Sse::new(boxed_stream)
    .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
    .into_response()
```

### Config struct with `Default` + `validate()` + `EnvOverridable` (D-50)

```rust
// Source: pattern generalized from src/config/waypoint_store.rs, read directly.
use crate::config::env_utils::{EnvOverridable, read_env};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunWorkerConfig {
    pub concurrency: usize,
    pub lease_seconds: u64,          // heartbeat derived as lease_seconds / 4 (D-10)
    pub min_probe_interval_ms: u64,
}

impl Default for RunWorkerConfig {
    fn default() -> Self {
        Self { concurrency: 4, lease_seconds: 60, min_probe_interval_ms: 1000 }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `POST /agents/{id}/execute` holds the HTTP connection for the whole run | `POST /runs` enqueues, worker pool executes | This phase (PLAT-01/02) | The exact problem statement PRD 06 §1 opens with |
| Workspace MSRV 1.85 | Workspace MSRV 1.88 | Prior phase, D-10 (`time >= 0.3.47` RUSTSEC fix chain) | Every "verify at 1.85" instruction in this phase's own locked decisions and the program overview is stale by one micro-version — correct it in the plan, do not debate it |
| `schemars` transitive-only | `schemars` 1.2 direct dependency of the facade | Phase 26 (RT-05 structured output) | D-34's dependency-growth argument against deriving the `WarGraphDoc` schema is weaker than when it was written, though the decision itself still stands |

**Deprecated/outdated:** None introduced by this phase; no `#[deprecated]` items expected (all new
capability is additive per X-03).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `RunEventStreamPort` belongs in `paladin-ports` under `output/` per D-27's "input port" framing — the exact sub-module naming (`input/` vs `output/`) was not independently re-derived against ADR-0015's allowlist text in this session | Recommended Project Structure | Low — a naming/placement detail, not a design error; easy to correct during plan review |
| A2 | The webhook delivery table's `next_attempt_at`-driven drain (D-40) should poll on a fixed interval rather than use a DB `LISTEN/NOTIFY`-style push — not explicitly confirmed against a specific existing polling-loop precedent in this codebase this session | Architecture Patterns / Don't Hand-Roll | Low — D-40 already specifies the mechanism; this is an implementation-detail assumption about drain cadence, not the mechanism choice itself |

**If this table is empty:** N/A — two low-risk implementation-detail assumptions remain; every
package-existence, API-shape, and file:line claim above was independently verified against this
repository's own source or `Cargo.lock` in this session.

## Open Questions

1. **Should `AgentSpec` (existing, `agent_registry.rs`) be extended/reused as `PaladinConfigDoc`, or
   should a new, richer type be authored?**
   - **RESOLVED (planning, 2026-09-08 — 27-12):** a new facade type `AgentDefinition`, deliberately
     *diverging* from the extend-`AgentSpec` recommendation. Rationale: `AgentSpec` is a `paladin-web`
     DTO (`agent_registry.rs`, `utoipa::ToSchema`), while D-31 puts the validator in the facade and D-28
     keeps the stored body as opaque JSON in core — extending `AgentSpec` would make the facade depend
     on a web-layer type and would grow a struct Milestone-12 provisioning already depends on (X-10.3).
     `AgentDefinition` mirrors `AgentSpec`'s JSON field names (minus `id`, plus `#[serde(default)]
     tools`) so the wire shape stays familiar; validation mirrors `ProvisionError::InvalidSpec`'s rules.
   - What we know: `AgentSpec` already carries `id, name, model, system_prompt, temperature,
     stop_words, timeout_seconds, allowed_roles` and is already `Serialize`/`Deserialize`/
     `utoipa::ToSchema`. `AgentProvisioner::provision()` already performs exactly the kind of
     structural validation PLAT-FR-09 wants for Agent-kind assistants (`ProvisionError::InvalidSpec`).
   - What's unclear: PRD 06 §2.3 describes a fuller "Paladin config as data (system prompt, model,
     limits, tools by name, middleware config)" than `AgentSpec` currently carries (no tools, no
     middleware fields) — extending `AgentSpec` risks growing a type that other phases (agent
     provisioning, Milestone 12 heritage) also depend on; a parallel `PaladinConfigDoc` risks
     duplicating validation logic `AgentProvisioner` already has.
   - Recommendation: default to extending `AgentSpec` with `#[serde(default)]` optional fields for
     tools/middleware (consistent with X-10.3's construction-path discipline `AgentSpec` already has
     no builder for — check whether it needs one first) rather than inventing a parallel type; flag
     this explicitly for the plan author to confirm before committing to file layout.

2. **Where exactly does `RunEventStreamPort` live, and does `paladin-ports`' existing `input`/
   `output` module split accommodate a port whose direction is "the facade produces, the web layer
   consumes"?**
   - **RESOLVED (planning, 2026-09-08 — 27-10):** `crates/paladin-ports/src/input/`, beside
     `RunSubmissionPort` — exactly the recommendation (facade-implemented, web-consumed, same shape as
     `ParleyPort`).
   - What we know: D-27 states `paladin-web` "consumes" the port and it "returns a `Stream` of core
     `RunStreamEvent` values" — this is shaped like other input ports in this codebase (`paladin-web`
     depends on ports that the facade implements).
   - What's unclear: whether the existing `input/`+`output/` naming convention in `paladin-ports`
     maps cleanly onto a port whose caller is `paladin-web` and whose implementor is the facade (this
     is the same shape as `RunSubmissionPort`, D-12) — worth a quick confirm against `paladin-ports/
     src/input/` vs `output/` module contents before the plan fixes the file path.
   - Recommendation: mirror wherever `RunSubmissionPort` lands (same input/output classification),
     since both are facade-implemented, web-consumed ports of the same shape.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Docker | `sdk-clients` CI job (D-49), Redis/Postgres Tier-2 contract suites (D-51) | ✗ locally (devcontainer) / ✓ in CI | CI: `ubuntu-latest` ships Docker, confirmed by 5 existing jobs (`docker-integration`, `postgres-integration`, `redis-cache-integration`, `docker`, `kubernetes-smoke`) already running `docker compose` with no special setup beyond `docker/setup-buildx-action@v3` for the build-only job | Tier-2 suites route to CI/UAT per D-51 (carried from Phase 24 D-28); never mark them passed locally |
| Redis | `RunQueuePort` Redis adapter contract suite | ✗ locally / ✓ in CI (`redis:7-alpine` service container, `ci.yml:567-573`) | 7-alpine in CI | InMemory adapter covers the same contract suite logic locally (Tier 1, D-51) |
| Postgres | `RunRepositoryPort`/`ScheduleRepositoryPort` Postgres adapter contract suite | ✗ locally / ✓ in CI (`postgres-integration` job, `ci.yml:817-892`) | via `docker/docker-compose.test.yml` | SQLite adapter covers the same contract suite logic locally (Tier 1, D-51) |
| `openapi-generator-cli` | `sdk-clients` job (D-49) | Not yet wired (new job) | N/A | Docker image (`openapitools/openapi-generator-cli`) is the natural choice given confirmed Docker availability on `ubuntu-latest`; npm/pip distributions are a documented fallback if the Docker pull proves flaky in CI |

**Missing dependencies with no fallback:** none identified — every Tier-2 dependency has a Tier-1
in-memory/SQLite fallback per the carried-forward D-51 convention.

**Missing dependencies with fallback:** Docker (local dev), Redis (local dev), Postgres (local dev) —
all three are CI-only per the standing devcontainer limitation; this is not new to this phase.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` / `cargo tokio::test` (async), `cargo llvm-cov` for coverage |
| Config file | none dedicated — `scripts/coverage.sh` is the single source of truth for the coverage invocation, shared by `make coverage` and CI's `coverage` job |
| Quick run command | `cargo test -p paladin-core -p paladin-ports -p paladin-battalion -p paladin-storage -p paladin-web` (per-crate, Tier 1 only, no Docker services) |
| Full suite command | `bash scripts/coverage.sh` (Tier 1 + whatever Tier 2 services are reachable; CI wires Redis+MinIO service containers) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PLAT-01 | `RunStatus::try_transition` monotonic status machine | unit | `cargo test -p paladin-core run_status_transition -- --test-threads=1` | ❌ Wave 0 |
| PLAT-01 | `RunRepositoryPort` CAS transition + contract suite (SQLite Tier 1, Postgres Tier 2) | integration | `cargo test -p paladin-storage --features sqlite run_repository_contract` | ❌ Wave 0 |
| PLAT-02 | `RunQueuePort` lease-expiry redelivery (InMemory Tier 1, Redis Tier 2) | integration | `cargo test -p paladin-storage run_queue_contract` | ❌ Wave 0 |
| PLAT-02 | Lease-expiry two-worker exactly-once-completion stress test | `#[tokio::test(flavor = "multi_thread")]` | `cargo test -p <facade-crate> worker_pool_lease_expiry_exactly_once -- --nocapture` | ❌ Wave 0 |
| PLAT-02 | `409 ThreadBusy` under 10 concurrent submits | `#[tokio::test(flavor = "multi_thread")]` | `cargo test -p <facade-crate> ten_concurrent_submits_one_accepted -- --nocapture` | ❌ Wave 0 |
| PLAT-03 | `AwaitingInput` releases worker (queue depth returns to 0) | integration | `cargo test -p <facade-crate> awaiting_input_acks_queue` | ❌ Wave 0 |
| PLAT-03 | SSE bridge live + degraded mode, 15s heartbeat | controller (`tower::oneshot`) | `cargo test -p paladin-web run_stream_sse` | ❌ Wave 0 |
| PLAT-04 | Version immutability (no `update_version`, no `PUT` route) + freeze-at-submit concurrency test | unit + `#[tokio::test(flavor = "multi_thread")]` | `cargo test -p <facade-crate> assistant_version_freeze_at_submit` | ❌ Wave 0 |
| PLAT-04 | `WarGraphDoc` round-trip (doc → compile → fingerprint stable across two-process boundary) | integration | `cargo test -p paladin-battalion wargraph_doc_fingerprint_two_process` | ❌ Wave 0 |
| PLAT-05 | Cron restart test (exactly-once per `on_missed` policy) | `#[tokio::test(flavor = "multi_thread")]` under `tokio::time::pause` | `cargo test -p <facade-crate> schedule_restart_exactly_once` | ❌ Wave 0 |
| PLAT-05 | Webhook signature + retry schedule (paused clock) + 4xx dead-letter + SSRF table | unit + integration (`mockito`) | `cargo test -p <facade-crate> webhook_delivery` | ❌ Wave 0 |
| PLAT-06 | Controller tests per new endpoint (`tower::oneshot` pattern) | controller | `cargo test -p paladin-web run_controller` | ❌ Wave 0 |
| PLAT-06 | `openapi.json` drift guard | unit | `cargo test -p paladin-web --lib openapi_matches_committed_baseline` | ✅ (existing test, extend its coverage as routes are added) |
| PLAT-06 | Generated Python + TS clients smoke-test | CI job | new `sdk-clients` GitHub Actions job | ❌ Wave 0 (CI infra, not a Rust test file) |

### Sampling Rate

- **Per task commit:** the relevant crate's `cargo test -p <crate>` (Tier 1 only, no Docker needed).
- **Per wave merge:** `bash scripts/coverage.sh` locally where Redis/Postgres are unreachable, this
  naturally falls back through the InMemory/SQLite Tier-1 paths per D-51 (do not treat a local
  Tier-2 skip as a failure).
- **Phase gate:** full CI green (`coverage`, `msrv` at 1.88, `semver`, `postgres-integration`,
  `redis-cache-integration` or equivalent new Redis-queue job) before `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `crates/paladin-core/src/platform/container/run.rs` + its `#[cfg(test)]` status-machine tests —
  the phase's literal first test per PRD 06 §5 item 1.
- [ ] `crates/paladin-storage/src/run/contract_tests.rs` — new shared contract suite (mirrors
  `waypoint/contract_tests.rs`).
- [ ] `crates/paladin-storage/src/run_queue/{in_memory,redis}.rs` + a new contract suite — no
  existing Lua/EVAL test fixture to extend (Pitfall 4).
- [ ] A facade-crate test module for `RunWorkerPool`/`ScheduleService`/`WebhookDeliveryService` stress
  tests (X-05 exact-count + timeout-guard convention, mirroring
  `src/application/services/orchestration/listener.rs`'s house pattern).
- [ ] `crates/paladin-battalion/tests/fixtures/graph_docs/` — the example corpus D-34's drift guard
  needs, plus the `jsonschema` dev-dependency addition and its first `cargo msrv verify` run at 1.88.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Existing opaque bearer-token `AuthPort` (ADR-0040/0041); this phase adds no new auth mechanism, reuses `require_authentication` middleware on every new route |
| V3 Session Management | no | Stateless token auth, no session concept introduced |
| V4 Access Control | yes | Two-tier `authorize_invoke`/`require_admin` convention (D-46), reusing `agent_auth.rs` exactly |
| V5 Input Validation | yes | `serde`/`utoipa` typed request bodies; `WarGraphDoc::compile()` and `AgentProvisioner::provision()` as the two validation seams (D-31); cron string validation via `croner`; webhook URL validation via the SSRF guard (D-42) |
| V6 Cryptography | yes | `hmac`/`sha2` (RustCrypto, already in the dependency graph) for webhook signatures — never hand-rolled, per this workspace's own conventions |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SSRF via webhook URL (attacker-controlled destination reaching internal/metadata services) | Spoofing/Elevation | Standalone SSRF guard at write AND send time (D-42), no crate dependency needed (std `IpAddr` classification suffices) |
| Webhook signature bypass via re-serialization drift | Tampering | Sign the exact byte buffer that is sent, never re-serialize (D-41) — verified in the RustCrypto `Hmac<Sha256>` construction |
| Credential (HMAC signature header) forwarded to an attacker-chosen host via redirect | Information Disclosure | `reqwest::redirect::Policy::none()` on the webhook client — already the house pattern for every credential-bearing outbound client in this repo (Pattern 5) |
| Thread-busy race under concurrent submits bypassing the one-active-run invariant | Tampering/DoS | DB-level partial unique index + portable `is_unique_violation()` mapping (D-17, Pattern 2) — not an application-level check, which cannot hold under concurrency |
| SQL injection via cron/webhook/assistant input fields | Tampering | `sqlx`'s parameterized `query()`/`query_as()` bind parameters exclusively — no format-string SQL anywhere in the existing waypoint adapters, and none should be introduced here |

## Sources

### Primary (HIGH confidence — direct source/registry read this session)

- `/workspace/Cargo.toml`, `/workspace/Cargo.lock`, `/workspace/crates/paladin-storage/Cargo.toml` —
  MSRV, edition, resolved dependency versions and feature gates.
- `/usr/local/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/croner-2.2.0/src/{lib,pattern,errors}.rs`
  — exact `Cron`/`CronPattern`/`CronError` API.
- `/usr/local/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.6/src/error.rs`,
  `sqlx-sqlite-0.8.6/src/error.rs`, `sqlx-postgres-0.8.6/src/error.rs` — `DatabaseError` trait,
  `ErrorKind`, per-backend `kind()`/`constraint()` behavior.
- `/usr/local/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-0.8.9/src/response/sse.rs` —
  `Sse::keep_alive`/`KeepAlive` API.
- `crates/paladin-battalion/src/engine/{mod,superstep,graph,registries,dispatch_registry,shutdown}.rs`
  — engine seams, `RunOutcome`, `NodeSpec`, `EngineRegistries`, `ShutdownCoordinator`.
- `crates/paladin-ports/src/output/trace_sink_port.rs` — `TraceEvent`'s actual eight variants.
- `crates/paladin-storage/src/waypoint/{sqlite,postgres}.rs`, `node_cache/redis.rs`,
  `scheduler.rs` — existing house patterns (contract suite shape, `wrap_error`, `scan_match`, 5↔6
  field cron normalization).
- `crates/paladin-web/src/{agent_controller,agent_registry,thread_controller,openapi}.rs` — SSE
  implementation, `AgentSpec`/`AgentProvisioner`, `OpenApiRouter` assembly, openapi drift guard.
- `crates/paladin-llm/src/{openai,deepseek,anthropic,gemini,kimi,grok,ollama,qwen,openai_compatible}/adapter.rs`
  — `Policy::none()` redirect discipline.
- `src/config/waypoint_store.rs` — tagged-enum config struct template.
- `.github/workflows/ci.yml` — `msrv` job pin (1.88), Docker-backed jobs on `ubuntu-latest`,
  `coverage` job / `scripts/coverage.sh` floor.
- `Makefile:367-371` — `make openapi` regeneration command.
- `MIGRATION.md` — confirmed already exists, actively maintained with TBD/owner annotations.
- `gsd-tools query package-legitimacy check --ecosystem crates hmac croner ipnet jsonschema schemars`
  — all five `OK`.

### Secondary (MEDIUM confidence)

- `rust-lang/rust#129238` (WebSearch, cross-checked against a local rustc 1.97.1 compile of
  `is_unique_local`/`is_unicast_link_local`/`is_private`/`is_loopback`/`is_link_local`/
  `is_unspecified` — all compiled successfully) — stabilization of the two IPv6 classification
  methods D-42's SSRF guard needs.

### Tertiary (LOW confidence)

- None — every claim in this document was either verified against this repository's own source/lock
  file or cross-checked via a local compile/registry query.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every version and feature-gate claim verified against `Cargo.lock`/
  `Cargo.toml` directly, not training-data recall.
- Architecture: HIGH for engine seams and existing patterns (direct source reads with line numbers);
  MEDIUM for the two new-port placement questions in Open Questions (naming/module-split details left
  for plan-time confirmation).
- Pitfalls: HIGH — all five are demonstrated by direct source read (absence of a registry, absence of
  a TraceEvent variant, an outdated MSRV citation, absence of Lua usage, and a documented coverage
  tooling gap), not speculation.

**Research date:** 2026-09-08
**Valid until:** 2026-10-08 (30 days — this workspace's dependency graph, MSRV, and coverage floor
have all changed within the last month per the evidence above; re-verify Cargo.lock-derived claims
if this phase's planning stretches past a few weeks)
