# Phase 22: Battlefield State & Superstep Engine - Context

**Gathered:** 2026-09-01
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected on recommended defaults; audit trail in 22-DISCUSSION-LOG.md)

<domain>
## Phase Boundary

Phase 22 delivers the keystone of the v0.10.0 Durable Agent Execution Runtime, and nothing from
later epics beyond their seams:

1. **Battlefield typed state** in `paladin-core` (new `platform::container::battlefield`):
   `BattlefieldSchema`, `FieldSpec`, `DispatchRule` (`LastWrite`/`Append`/`MergeObject`/`Sum`/
   `Custom`), `StateDelta`, typed accessors, schema enforcement, structured `BattlefieldError` —
   with **no new `paladin-core` dependencies** (ENG-01).
2. **Waypoint domain types** in `paladin-core` (`platform::container::waypoint`): `ThreadId`,
   `WaypointId`, `Waypoint`, `WaypointStatus` (incl. the `AwaitingInput`/`ParleyRequest` stub for
   Doc 03), `NodeExecutionRecord`, `NodeId` newtype.
3. **`WaypointPort`** in `paladin-ports` (`output::waypoint_port`) — separate from `CitadelPort`,
   rationale documented in rustdoc.
4. **`WarEngine` + `WarGraph`** in `paladin-battalion` (new `engine` module): superstep loop,
   cycles/self-loops with bounded iteration, deterministic frontier + merge order, same-superstep
   snapshot isolation, join/defer semantics, automatic one-Waypoint-per-superstep under default
   `Strict` durability, `resume` with zero re-execution, graph fingerprint check (ENG-02…04).
5. **Three Waypoint backends** (InMemory, SQLite, Postgres) sharing one contract suite, plus
   `WaypointRetentionConfig` cleanup (ENG-05).
6. **Legacy bridges** `from_formation`/`from_phalanx`/`from_campaign` with golden
   output-equivalence tests; legacy execution services byte-identical (ENG-06; sole sanctioned
   exception BUG-01 is owned by Phase 23/CF-01, not this phase).
7. **Seams only** for later docs: `TraceSink` hook (bounded channel, drop-oldest, counted drops),
   ordered `NodeInterceptor` chain (default empty), `CancellationToken` → `Halted` Waypoint
   (ENG-07). Implement the hooks, not their consumers.
8. **Program scaffolding** (ENG-08): root `MIGRATION.md` with the §9 skeleton and pre-populated
   entries; new `cargo semver-checks` CI job (vs published v0.9.0 crates, per-item allowlist);
   new MSRV CI job (Rust 1.85, full workspace, `--all-features`) — both on every PR.

**Out of this phase** (later phases own them): dynamic `Goto`/Directive routing, Muster fan-out,
subgraph nodes (Phase 23); parley/resume-with-payload, forking, graceful-shutdown wiring
(Phase 24); retry/timeout/error handlers (Phase 25); middleware consumers (Phase 26); HTTP
exposure (Phase 27); trace consumers (Phase 28).

</domain>

<decisions>
## Implementation Decisions

The PRD (doc 01) is the FR-level source of truth and already locks type shapes, error variants,
FR semantics, defaults (`max_supersteps` 50, `max_node_visits` 25, durability `Strict`,
parallelism = vanguard size) and the test plan. The decisions below settle only what the PRD left
open. Do not re-litigate anything the PRD states.

### Waypoint backend placement & schema
- **D-01:** All three `WaypointPort` backends live in **`paladin-storage`** (PRD offers
  "`paladin-memory` or `paladin-storage`"): `InMemoryWaypointStore` un-gated (tests/dev),
  `SqliteWaypointStore` on the existing sqlx/SQLite stack, `PostgresWaypointStore` behind a new
  `postgres` feature on `paladin-storage` with a facade passthrough feature — following the
  existing feature-gating convention (X-07). Rationale: `paladin-storage` is the
  repositories/persistence/Citadel home; `paladin-memory` owns Garrison/Sanctum conversation
  memory. — **Reversibility:** costly — moving adapters between published crates later touches
  feature flags, facade re-exports, and downstream `Cargo.toml`s.
- **D-02:** SQLite `waypoints.payload` column is **TEXT holding serialized JSON** (PRD allows
  BLOB/JSON), for debuggability and alignment with the sqlx `json` feature already enabled.
  Postgres uses JSONB per the PRD. Migrations follow the existing convention
  (`crates/paladin-memory/migrations/001_…` pattern) in the owning crate, registered in
  `MIGRATION.md` §9.4. — **Reversibility:** costly — changing the column type after v0.10.0
  ships requires a `waypoints` table migration on user databases.

### Identity, fingerprint & legacy mapping
- **D-03:** `WaypointId` uses **UUIDv7** (PRD-preferred, time-ordered). Enable the `v7` feature
  on the existing workspace `uuid = 1.8` dependency — a feature addition on an existing dep, not
  a new `paladin-core` dependency, so ENG-01's constraint holds. Record in `MIGRATION.md` §9.3.
- **D-04:** The graph fingerprint (ENG-FR-14) is **blake3 over a canonical, deterministically
  ordered serialization** of node ids, edge specs, and schema (sorted iteration — never raw
  `HashMap` order), hex-encoded, excluding prompts/models per the PRD. blake3 is already a
  `paladin-core` dependency. — **Reversibility:** one-way — changing the algorithm or
  canonicalization after release makes every stored Waypoint's fingerprint mismatch, breaking
  `resume` for all existing threads with `GraphMismatch` on unchanged graphs.
- **D-05:** `from_campaign`'s legacy `Uuid` → `NodeId` mapping is **deterministic and
  human-readable**: the Paladin name slug when unique within the graph, else
  `{name-slug}-{short-uuid}`. The mapping must be stable across repeated construction from the
  same Campaign so fingerprints and golden tests are reproducible. — **Reversibility:** costly —
  golden equivalence tests and stored fingerprints of bridged graphs depend on it.

### Program CI scaffolding (ENG-08)
- **D-06:** The `semver` job runs `cargo semver-checks` against the **published v0.9.0 crates.io
  versions** of every publishable crate (matching ENG-08's wording), with the deliberate-breaking
  allowlist checked into the repo as per-item entries that must mirror `MIGRATION.md` §9.2
  exactly (X-10.5). New job in `.github/workflows/ci.yml`, on every PR.
- **D-07:** The MSRV job is a **dedicated CI job pinned to the Rust 1.85 toolchain** building the
  full workspace with `--all-features` (X-11.1). The workspace advertises
  `workspace.package.rust-version = "1.85"` and the README badge in the same change. If any
  existing or new dependency cannot satisfy 1.85, that is the X-11.2 **stop-and-flag** path
  (propose the bump with alternatives, record in §9.3) — not a silent bump. Note: the pinned dev
  toolchain stays 1.97.1 (`rust-toolchain.toml`); the MSRV job installs 1.85 explicitly.
- **D-08:** `MIGRATION.md` is created **this phase** at the repository root with the full §9
  skeleton, M-B-01…03 pre-populated, and every §9.2 register row from overview §9.2 carried in.
  It is a living document: "TBD" is acceptable only in sections owned by later epics; every item
  this phase touches (uuid `v7`, `postgres` feature, waypoint migrations, new config/env) is
  filled in now. Phase 29/SHIP-01 clears all remaining TBD.

### Contract suite & test infrastructure
- **D-09:** The shared backend contract suite (ENG-FR-17) is implemented as **generic async test
  functions taking `&dyn WaypointPort`** (or a generic parameter), invoked from per-backend
  `#[tokio::test]`s — chosen over a declarative macro for clearer failure diagnostics. PRD
  permits either form.
- **D-10:** Postgres integration tests run via the **existing docker-compose integration target**
  (`make test-integration-docker`), adding a postgres service to the compose file — Tier 2
  Docker-gated per the shipped three-tier test strategy. PRD acceptance 4 names this target
  explicitly. SQLite contract tests stay Tier 1 (no external services).
- **D-11:** The ENG-FR-08 determinism test uses **seeded randomized scheduling** (shuffle node
  spawn order / inject yields per iteration) over ≥20 iterations, asserting byte-identical
  serialized Battlefields. The X-05 stress test follows the house pattern in
  `src/application/services/orchestration/listener.rs` (exact-count assertions, explicit timeout
  guard, `#[tokio::test(flavor = "multi_thread")]`).

### Engine semantics defaults
- **D-12:** All PRD defaults are kept as stated: parallelism limit defaults to vanguard size,
  `WaypointDurability::Strict` default with documented `BestEffort`, `EngineLimits` defaults
  50/25, `run_timeout: Option<Duration>` present but plumbing-only (Doc 04 owns timeout
  semantics). Snapshot isolation via a single Arc-shared pre-superstep read snapshot; deltas
  merged only after all superstep nodes complete, in the ENG-FR-08 deterministic order.

### Claude's Discretion
- Exact file layout inside `crates/paladin-battalion/src/engine/` and the core module splits
  (`battlefield.rs` vs a `battlefield/` directory) — follow existing crate conventions.
- Error message wording, internal data structures, bench harness details (criterion is already
  set up in `benches/`).
- `WaypointSummary`/`ThreadSummary` field selection beyond what the PRD implies.
- How `NodeContext` is shaped this phase (only what ENG needs; later docs extend it).
- Plan/wave decomposition — but respect the PRD §7 TDD ordering (core types → contract suite on
  InMemory → engine with Function nodes → MockPaladinPort integration → resume → SQL backends →
  golden tests → stress → benches).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase source of truth (behavior)
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` — **The FR-level source of
  truth for this phase.** ENG-FR-01…23, type shapes (§3), NFRs (§5), acceptance criteria (§6),
  TDD test-plan ordering (§7), seams handed to later docs (§8). Every plan task traces to an FR
  here.
- `.project/v0.10.0/00-program-overview.md` — Cross-cutting rules X-01…X-11 (apply to every
  requirement), ubiquitous-language additions (§4: Battlefield, Dispatch, Superstep, Waypoint,
  Thread, Vanguard), E2E-1 acceptance scenario (§6), and the **complete required `MIGRATION.md`
  structure with pre-populated content (§9)** — ENG-08's skeleton comes verbatim from here.
- `.planning/REQUIREMENTS.md` — ENG-01…08 capability clusters with FR-range traceability; the
  scope-time conflict record and precedence note (shipped tree outranks PRD).
- `.planning/ROADMAP.md` — Phase 22 goal, dependencies, and the five success criteria.
- `.project/v0.10.0/08-traceability-matrix.md` — verification protocol the Phase 29 audit will
  run against this phase's FRs; write tests so that audit can find them.

### Standing project decisions that constrain this phase
- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — the workspace line-coverage floor the
  CI `coverage` job enforces; PRD acceptance 7 additionally wants ≥85% on new modules.
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` (ADR-0015) — the enforceable
  `paladin-core`/`paladin-ports` dependency invariant (no provider SDK, transport client, storage
  driver, web framework). Battlefield/Waypoint types must respect it; ENG-01 tightens it to "no
  new core dependencies" for this phase.
- `.planning/decisions/0016-port-value-type-ownership.md` (ADR-0016) — core owns value types,
  ports re-export; follow the same ownership pattern for Waypoint types (core) vs `WaypointPort`
  (ports).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `blake3` (1.8.2) and `sha2` already in `paladin-core` — fingerprint needs no new dependency.
- `uuid` workspace dep at 1.8 (`v4`, `serde`) — add the `v7` feature for `WaypointId` (D-03).
- `petgraph` 0.6 already in `paladin-core` — existing Campaign graph types; the WarEngine may use
  it internally or its own adjacency structures (Claude's discretion), but must not inherit
  Campaign's toposort cycle rejection.
- sqlx stack (0.8, `runtime-tokio-rustls`, `sqlite`, `migrate`) live in
  `paladin-storage`/`paladin-memory`; migrations convention at
  `crates/paladin-memory/migrations/001_create_garrison_tables.sql`.
- Mock LLM adapter (`mock` feature, default-on) — engine tests per PRD §7 use Function nodes
  first, then `MockPaladinPort`.
- `criterion` bench setup in `benches/` for ENG-NFR-01/02.
- `testcontainers` 0.24 exists but D-10 selects the docker-compose target for Postgres.
- X-05 house stress-test pattern: `src/application/services/orchestration/listener.rs`.

### Established Patterns
- Port traits in `crates/paladin-ports/src/output/`, `Send + Sync`, async-trait — `waypoint_port`
  follows `file_storage_port`/`llm_port` shape.
- thiserror per-layer error enums, `From` conversions at boundaries; no stringly-typed variants
  (X-06); every new persisted type carries `schema_version` + serde (X-04).
- X-09 config convention: new tunables (`WaypointRetentionConfig`, engine durability/limits
  config) get structs in `src/config/` with `Default`, `validate()`, `EnvOverridable`, mirroring
  `CitadelConfig`.
- Feature-gating convention: adapters gated on the owning crate + facade passthrough
  (`Cargo.toml` features table); heavy deps must not enter `paladin-ai` defaults (X-11.4).
- Three-tier testing: Tier 1 always-in-CI, Tier 2 Docker-gated, Tier 3 API-key-gated.

### Integration Points
- `crates/paladin-core/src/platform/container/` — new `battlefield` and `waypoint` modules beside
  `citadel.rs`, `battalion/`.
- `crates/paladin-ports/src/output/` — new `waypoint_port.rs`.
- `crates/paladin-battalion/src/` — new `engine/` module beside the untouched legacy services
  (`formation_service.rs`, `phalanx_service.rs`, `campaign_service.rs`, `commander.rs`).
- `crates/paladin-storage/` — three backend adapters + migrations + `postgres` feature (D-01).
- `.github/workflows/ci.yml` — two new jobs: `semver` (D-06) and MSRV (D-07). Neither exists
  today (verified by grep).
- Repository root — `MIGRATION.md` does not exist today; ENG-08 creates it (D-08).
- **Constraint confirmed in tree:** Campaign rejects cycles via toposort at
  `crates/paladin-battalion/src/campaign_service.rs:236` and
  `crates/paladin-core/src/platform/container/battalion/campaign.rs:255` — the engine is
  additive; these call sites are not modified (ENG-FR-20).

</code_context>

<specifics>
## Specific Ideas

- The Waypoint stores a **full Battlefield snapshot** — "delta-encoding is a backend
  optimization, not a contract" (PRD §3.3). Do not design a delta-chain format.
- `WaypointPort` is deliberately **separate from `CitadelPort`**; the rustdoc must state the
  rationale (high-frequency, append-mostly, thread-addressed vs coarse entity snapshots).
- `InputMapping` template strings (`{field}` placeholders, JSON-stringified unless the field is a
  JSON string) are the X-03 bridge letting string-in/string-out Paladins join typed workflows
  unchanged — including Campaign's `"\n\n---\n\n"` fan-in concatenation in `from_campaign`.
- Ubiquitous language is mandatory: Battlefield, Dispatch, Superstep, Waypoint, Thread, Vanguard,
  WarGraph, WarEngine — in code, docs, and comments (overview §4).

</specifics>

<deferred>
## Deferred Ideas

None — auto-mode discussion stayed within phase scope. Later-epic capabilities (Directive
routing, Muster, Parley, Aegis, HTTP threads, trace consumers) are already roadmapped to
Phases 23-28; this phase ships only their seams per ENG-07/PRD §8.

</deferred>

---

*Phase: 22-battlefield-state-superstep-engine*
*Context gathered: 2026-09-01*
