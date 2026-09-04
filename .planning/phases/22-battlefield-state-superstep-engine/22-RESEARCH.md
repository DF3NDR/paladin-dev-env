# Phase 22: Battlefield State & Superstep Engine - Research

**Researched:** 2026-09-01
**Domain:** Rust workspace engine design (typed shared state + cyclic superstep executor + durable checkpointing), hexagonal architecture, CI semver/MSRV gating
**Confidence:** HIGH

## Summary

This phase is almost entirely spec-driven: PRD 01 (`01-battlefield-state-and-execution-engine.md`)
and the program overview already lock every type shape, error variant, FR, default, and test
order. The research task here is not "what stack should we use" — it is "what does the current
tree actually look like, so the plan doesn't assume a fact that's false." Every fact below was
verified against the tree at HEAD, not assumed from the PRD's prose.

Three tree facts materially change what the plan must include beyond the PRD's own text:

1. **`uuid` is pinned to `1.8.0`** in `[workspace.dependencies]`, with only `v4` + `serde`
   features enabled. The `v7` feature exists on that exact version (confirmed via docs.rs) and
   needs zero version bump — just add `"v7"` to the workspace `uuid` features array. This is a
   one-line diff, not a dependency bump.
2. **No `[workspace.package]` table exists.** Every one of the 11 publishable crates (`paladin-ai-core`
   aka `paladin-core` dir, `paladin-ports`, `paladin-battalion`, `paladin-herald`, `paladin-llm`,
   `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-web`,
   `paladin-ai` facade) declares `version`, `edition`, `license`, etc. independently in its own
   `[package]` block. D-07's "the workspace advertises `workspace.package.rust-version = "1.85"`"
   cannot be satisfied by editing one file — either introduce a `[workspace.package]` table with
   `rust-version.workspace = true` inheritance in all 11 manifests (bigger diff, cleaner going
   forward), or add `rust-version = "1.85"` to each of the 11 `[package]` blocks individually
   (smaller diff per file, more repetition). The plan must pick one; this is a `Claude's Discretion`
   item but the PRD's wording favors the workspace-table form since it says "the workspace
   advertises."
3. **`paladin-storage`'s `sqlx` feature set has no `postgres` feature wired today** — only
   `sqlite` and `mysql` map to `sqlx/sqlite` / `sqlx/mysql`. D-01/ENG-FR-17 requires a new
   `postgres` feature (`sqlx/postgres`) plus, per D-01, a facade passthrough feature on `paladin-ai`
   mirroring the existing `sqlite`/`mysql` pattern.

**Primary recommendation:** Follow the PRD's type shapes and TDD ordering (§7) verbatim; the
plan's job is sequencing and file layout, not invention. Use `tokio_util::sync::CancellationToken`
(already a `paladin-battalion` dependency, already used elsewhere in the tree) for ENG-FR-23 rather
than hand-rolling cancellation. Keep the WarEngine's internal graph representation as a plain
`HashMap<NodeId, NodeSpec>` + `Vec<EdgeSpec>` (as the PRD sketches) rather than wrapping `petgraph`,
because `petgraph`'s `toposort`-based cycle rejection is exactly the behavior ENG-FR-02 must NOT
inherit — reusing petgraph for the executable graph risks accidentally reusing that helper.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Battlefield/StateDelta/DispatchRule types | Core (`paladin-core`) | — | Pure domain value types, no I/O, must stay dependency-pure (ADR-0015) |
| Waypoint/ThreadId/WaypointId/NodeExecutionRecord types | Core (`paladin-core`) | — | Pure value types persisted by adapters; core owns shape per ADR-0016 pattern (core owns value types, ports re-export) |
| `WaypointPort` trait | Application/Ports (`paladin-ports`) | — | Port trait, no I/O implementation, `Send + Sync` async-trait per existing port convention |
| `WarEngine`/`WarGraph`/superstep loop | Application (`paladin-battalion`, new `engine` module) | — | Orchestration logic; composes `PaladinPort` + `WaypointPort`, mirrors existing Formation/Phalanx/Campaign services' placement |
| `InMemoryWaypointStore` | Infrastructure (`paladin-storage`) | — | Adapter implementing `WaypointPort`; D-01 places all three backends in `paladin-storage` |
| `SqliteWaypointStore` / `PostgresWaypointStore` | Infrastructure (`paladin-storage`) | Database/Storage | SQL adapters behind `sqlite`/`postgres` features, following existing `sqlx` feature-gating convention |
| Legacy bridges (`from_formation`/`from_phalanx`/`from_campaign`) | Application (`paladin-battalion::engine`) | — | Additive constructors alongside untouched legacy services; must not modify `campaign_service.rs`'s toposort call site |
| `MIGRATION.md` + CI `semver`/`msrv` jobs | Program scaffolding (repo root / `.github/workflows/`) | — | Not a runtime tier; cross-cutting release governance (X-10/X-11) |

## Package Legitimacy Audit

No new external packages are introduced this phase beyond a **feature addition** on an existing
workspace dependency (`uuid`'s `v7` feature) and a **feature addition** on an existing dependency
(`sqlx`'s `postgres` feature, already part of the `sqlx = "0.8"` crate already in the tree). Neither
requires a new `Cargo.toml` dependency line naming a new crate name, so the slopsquat/hallucination
surface this gate exists to catch does not apply. For completeness, both are verified below anyway
since a feature name can also be fabricated by training data.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `uuid` (existing dep, `v7` feature added) | crates.io | 1.8.0 pinned since project start; max stable today 1.26.0 | very high (foundational crate) | github.com/uuid-rs/uuid | OK [VERIFIED: docs.rs feature list for uuid 1.8.0] | Approved — feature flag only, no new dependency line |
| `sqlx` (existing dep, `postgres` feature added) | crates.io | 0.8 pinned since project start; max stable today 0.9.0 | very high | github.com/launchbadge/sqlx | OK [VERIFIED: docs.rs feature list for sqlx 0.8] | Approved — feature flag only, no new dependency line |
| `cargo-semver-checks` (new CI tool, not a Cargo.toml dependency) | crates.io | mature (obi1kenobi, long-running project) | high | github.com/obi1kenobi/cargo-semver-checks | OK [CITED: crates.io + GitHub Action README] | Approved — CI tool installed via `cargo install` or `obi1kenobi/cargo-semver-checks-action`, not a workspace dependency |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Standard Stack

### Core (already in the tree — no additions beyond feature flags)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde` / `serde_json` | 1 / 1.0 | `Battlefield`/`StateDelta`/`Waypoint` serialization | Already the workspace's only serialization stack; ENG-01 forbids new core deps |
| `uuid` (+ `v7` feature) | 1.8.0 | `WaypointId` (time-ordered UUIDv7 per D-03) | Already a core dep at `v4`; `v7` feature exists at this exact pinned version [VERIFIED: docs.rs] |
| `blake3` | 1.8.2 | Graph fingerprint (D-04) | Already a `paladin-core` dependency (used for content/field/node version hashing elsewhere in the tree) — zero new deps |
| `thiserror` | 2 | `BattlefieldError`, `WaypointError`, `EngineError` | Existing per-layer error enum convention throughout the codebase |
| `tokio` | 1 (full) | Async runtime, `RwLock` for `InMemoryWaypointStore` | Already a workspace dep everywhere |
| `tokio-util` | 0.7 (already a `paladin-battalion` dep) | `CancellationToken` for ENG-FR-23 | Already present in `paladin-battalion/Cargo.toml`; already used elsewhere in the tree (`tests/integration/mcp_streamable_http_test.rs`) — don't hand-roll cancellation |
| `petgraph` | 0.6 | Available if the WarEngine's internal representation wants graph algorithms | Already a dep of both `paladin-core` and `paladin-battalion`; **do not** reuse its `toposort`-based cycle check for the WarGraph — that's the exact behavior ENG-FR-02 must not inherit from Campaign |

### Supporting (feature additions, no new crate names)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `sqlx` (+ `postgres` feature) | 0.8 (already pinned) | `PostgresWaypointStore` | Add `postgres` feature to `paladin-storage`'s existing `sqlx` optional dependency, mirroring the existing `sqlite`/`mysql` feature-gate pattern |
| `sqlx` (existing `sqlite`, `json`, `migrate` features) | 0.8 | `SqliteWaypointStore`, `waypoints.payload` as `TEXT` JSON (D-02) | Already enabled workspace-wide; the `json` feature is already active so JSON-in-TEXT round-trips without extra plumbing |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom `WarGraph` adjacency (`HashMap<NodeId, NodeSpec>` + `Vec<EdgeSpec>`) | `petgraph::Graph` wrapper | petgraph gives free algorithms (SCC, cycle detection) but its ergonomic cycle-rejecting helpers (`toposort`) are the wrong tool here; a hand-rolled frontier/vanguard computation over plain maps is simpler to make deterministic (ENG-FR-04) and keeps the "not-firing" join semantics (ENG-FR-06) explicit rather than fighting petgraph's traversal iterators |
| `tokio_util::sync::CancellationToken` | Hand-rolled `Arc<AtomicBool>` + poll loop | tokio_util's token composes (child tokens), is already imported elsewhere in the tree, and is the idiomatic tokio cancellation primitive — no reason to hand-roll |
| blake3 canonical fingerprint | sha2 (also already a core dep) | PRD/D-04 explicitly choose blake3; sha2 is available but not the specified choice — don't substitute |

**Installation (feature flags only, no new `Cargo.toml` dependency entries):**
```toml
# Cargo.toml [workspace.dependencies]
uuid = { version = "1.8.0", features = ["v4", "v7", "serde"] }   # add "v7"

# crates/paladin-storage/Cargo.toml [features]
postgres = ["dep:sqlx", "sqlx/postgres"]                          # new feature, mirrors sqlite/mysql
```

**Version verification:** confirmed live against crates.io/docs.rs during this research session
(2026-09-01):
- `uuid` current max stable is `1.26.0`; the workspace's pinned `1.8.0` already exposes the `v7`
  feature (feature list fetched from `docs.rs/crate/uuid/1.8.0/features` — 21 features including
  `v7`, which pulls in `atomic` + `rng`/`getrandom`). No version bump required.
- `sqlx` current max stable is `0.9.0`; the workspace's pinned `0.8` line already exposes a
  `postgres` feature per the sqlx 0.8 docs.rs feature-flags page. No version bump required.
- `cargo-semver-checks` current max stable on crates.io is `0.50.0` [CITED: crates.io API,
  2026-09-01]. No version is pinned in the tree yet since the tool doesn't exist there; the CI job
  should pin the action to a tagged release (e.g. `obi1kenobi/cargo-semver-checks-action@v2`) per
  the action's own README rather than `@main`.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────┐
                    │   Caller: WarEngine::start(graph, thread,    │
                    │   initial_delta)  or  ::resume(graph,thread) │
                    └───────────────────┬───────────────────────────┘
                                        │
                         ┌──────────────▼───────────────┐
                         │  resume? → load latest         │
                         │  Waypoint via WaypointPort,     │
                         │  verify graph fingerprint       │
                         │  (ENG-FR-14), restore           │
                         │  Battlefield+Vanguard+visits     │
                         └──────────────┬───────────────┘
                                        │
                     ┌──────────────────▼───────────────────┐
                     │        SUPERSTEP LOOP (ENG-FR-01)      │
                     │  1. take current Vanguard              │
      ┌──────────────┤  2. snapshot Battlefield (read-only,   │
      │  cancel?      │     Arc-shared; ENG-FR-05 isolation)  │
      │  (token)      │  3. execute Vanguard nodes CONCURRENTLY│
      │               │     (parallelism = vanguard size)      │
      │        ┌──────┤     via NodeInterceptor chain (empty   │
      │        │      │     default, ENG-FR-22) → StateNode /  │
      │        │      │     PaladinPort.execute() w/ InputMapping│
      │        │      │  4. collect StateDelta per node         │
      │        │      │  5. MERGE deltas → Battlefield via       │
      │        │      │     DispatchRule (deterministic order,  │
      │        │      │     ENG-FR-08) — LastWrite/Append/       │
      │        │      │     MergeObject/Sum/Custom               │
      │        │      │  6. compute next Vanguard: join/defer    │
      │        │      │     semantics (ENG-FR-06), stable order  │
      │        │      │  7. persist exactly ONE Waypoint         │
      │        │      │     via WaypointPort (ENG-FR-11) —       │
      │        │      │     write failure fails run (Strict)     │
      │        │      │  8. TraceSink events fired (ENG-FR-21,   │
      │        │      │     fire-and-forget, drop-oldest)        │
      │        │      └──────┬─────────────────────────────────┘
      │        │             │  Vanguard empty? → RunOutcome::Completed
      │        │             │  limit hit? → RecursionLimitExceeded / NodeVisitLimitExceeded
      │        └─────────────┤  cancelled? → Halted Waypoint, RunOutcome::Halted
      │                      │  parley? → AwaitingInput (Doc 03 wiring; stub only here)
      └──────────────────────┘
                              │
                    ┌─────────▼──────────┐
                    │  WaypointPort impl  │
                    │  InMemory / SQLite /│
                    │  Postgres (shared   │
                    │  contract suite)    │
                    └─────────────────────┘
```

A reader can trace E2E-1 (crash-resume) by following: `start()` → superstep loop → Waypoint
persisted after superstep 3 → engine dropped → `resume()` re-enters at the top, loads that exact
Waypoint, and re-enters the loop at superstep 4 without re-executing steps 1-3's nodes.

### Recommended Project Structure
```
crates/paladin-core/src/platform/container/
├── battlefield.rs           # or battlefield/{mod,schema,dispatch,delta}.rs — Claude's discretion
├── battlefield_error.rs     # BattlefieldError, following the existing *_error.rs sibling pattern
├── waypoint.rs               # or waypoint/{mod,status,record}.rs
└── battalion/                # existing — untouched (formation.rs, phalanx.rs, campaign.rs)

crates/paladin-ports/src/output/
└── waypoint_port.rs           # WaypointPort trait + WaypointError, mirrors file_storage_port.rs shape

crates/paladin-battalion/src/
├── engine/
│   ├── mod.rs                 # WarEngine, WarGraph, EngineLimits, RunOutcome, EngineError
│   ├── superstep.rs            # superstep loop internals (Claude's discretion on split)
│   ├── dispatch_registry.rs    # Custom dispatch rule registration (ENG-FR-09)
│   ├── input_mapping.rs        # InputMapping template resolution (X-03 bridge)
│   ├── bridges.rs               # from_formation/from_phalanx/from_campaign
│   └── node.rs                  # NodeSpec, StateNode trait, NodeContext
├── formation_service.rs        # existing — untouched
├── phalanx_service.rs           # existing — untouched
├── campaign_service.rs           # existing — untouched (toposort call site at :236 stays)
└── commander.rs                  # existing — untouched

crates/paladin-storage/src/
├── waypoint/
│   ├── in_memory.rs             # InMemoryWaypointStore
│   ├── sqlite.rs                 # SqliteWaypointStore
│   └── postgres.rs               # PostgresWaypointStore (behind `postgres` feature)
└── ...

crates/paladin-storage/migrations/
└── 00X_create_waypoints_table.sql   # follows crates/paladin-memory/migrations/001_... convention

MIGRATION.md                          # repository root, new this phase (ENG-08)
.github/workflows/ci.yml              # + semver job, + msrv job
```

### Pattern 1: Per-Field Dispatch as a Reducer Map
**What:** `Battlefield::merge(deltas: Vec<(NodeId, StateDelta)>) -> Result<(), BattlefieldError>`
resolves each touched field's `DispatchRule` and applies it against all deltas targeting that
field in one call, in the ENG-FR-08 deterministic order (`(NodeId lexicographic, emission index)`).
**When to use:** Every superstep merge step (step 5 in the diagram above).
**Example:**
```rust
// Source: PRD 01 §3.1/§3.2 (type shapes are locked; this is the resolution shape they imply)
pub enum DispatchRule {
    LastWrite,
    Append,
    MergeObject,
    Sum,
    Custom(String),
}

// Concurrent LastWrite writers to the same field => hard error, not last-writer-wins silently.
// Concurrent Append writers => merge ordered by (NodeId, emission index) for byte-identical output.
```

### Pattern 2: Snapshot Isolation via Arc-Shared Read View
**What:** Before executing a superstep's Vanguard, clone an `Arc<Battlefield>` once; every
concurrently-executing node reads that same Arc. Deltas are collected into a `Vec` and merged only
after all nodes in the superstep finish — never mutated in place mid-superstep.
**When to use:** Every superstep, to satisfy ENG-FR-05 (no node sees a peer's delta this superstep)
and ENG-NFR-02 (one Battlefield clone per superstep maximum).
**Example:**
```rust
// Source: PRD 01 §5 ENG-NFR-02 + D-12 ("Snapshot isolation via a single Arc-shared
// pre-superstep read snapshot; deltas merged only after all superstep nodes complete")
let snapshot: Arc<Battlefield> = Arc::new(current_battlefield.clone());
let deltas: Vec<(NodeId, StateDelta)> = futures::future::join_all(
    vanguard.iter().map(|node_id| execute_node(node_id, Arc::clone(&snapshot)))
).await.into_iter().collect::<Result<_, _>>()?;
// merge deltas into current_battlefield only here, after the join
```

### Pattern 3: Legacy Bridge via Default Schema + InputMapping
**What:** `WarGraph::from_campaign(campaign: &Campaign) -> WarGraph` builds a 3-field default
schema (`input: LastWrite`, `output: LastWrite`, `history: Append`) and generates `InputMapping`
templates that reproduce today's string-only data flow, including the exact `"\n\n---\n\n"`
Campaign fan-in separator found at `crates/paladin-battalion/src/campaign_service.rs:373`.
**When to use:** ENG-FR-19's three bridge constructors.
**Example:**
```rust
// Source: crates/paladin-battalion/src/campaign_service.rs:373 (verified in tree)
// existing legacy fan-in — the bridge's InputMapping must reproduce this exactly:
Ok(inputs.join("\n\n---\n\n"))
```

### Anti-Patterns to Avoid
- **Reusing `petgraph::algo::toposort` for `WarGraph` validation:** this is the exact mechanism
  Campaign uses to reject cycles (`campaign_service.rs:236`, `battalion/campaign.rs:255`) — reusing
  it (even indirectly, by wrapping the WarGraph in a `petgraph::Graph` and calling a "validate"
  helper that happens to toposort) silently reintroduces the acyclic constraint ENG-FR-02
  explicitly forbids.
- **Mutating the Battlefield mid-superstep:** breaks ENG-FR-05 isolation and makes the ENG-FR-08
  determinism test flaky under randomized scheduling — always collect-then-merge.
- **Storing the Waypoint payload as a delta chain:** PRD §3.3 is explicit — "delta-encoding is a
  backend optimization, not a contract." Every Waypoint carries the FULL Battlefield snapshot at
  the domain-type level; a backend MAY internally delta-encode for storage efficiency but the
  `Waypoint.battlefield` field the port contract exposes is always a complete snapshot.
- **Extending `PaladinPort` or `CitadelPort` with new required methods** for engine needs — X-10.4
  forbids this; introduce new traits (`WaypointPort`, `StateNode`) instead, exactly as the PRD
  already does.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cooperative cancellation for ENG-FR-23 | Custom `Arc<AtomicBool>` + polling | `tokio_util::sync::CancellationToken` | Already a `paladin-battalion` dependency (`tokio-util = "0.7"`), already used in the tree (`tests/integration/mcp_streamable_http_test.rs`), supports child-token composition for future per-node cancellation |
| Content-addressable fingerprint for ENG-FR-14 | Custom rolling hash / `std::hash::Hash` + `DefaultHasher` (not stable across process/platform) | `blake3::Hasher` over a canonically-ordered byte stream | Already a `paladin-core` dependency used identically elsewhere (`collection_versioning_service.rs`, `field_version_service.rs`, `node_version_service.rs` all call `blake3::hash(...)`) — same pattern, new call site |
| Async trait test doubling for the `WaypointPort` contract suite | Hand-written duplicate test functions per backend | Generic async test functions over `&dyn WaypointPort` (D-09) invoked from per-backend `#[tokio::test]`s | D-09 already settles this; PRD explicitly permits either a macro or generic-fn form, generic fn chosen for diagnostics |
| UUIDv7 generation | Custom time-ordered ID scheme | `uuid::Uuid::now_v7()` (the `v7` feature) | Feature already available at the pinned `uuid = 1.8.0` version; time-ordering + RFC compliance for free |

**Key insight:** almost nothing new needs writing from scratch in this domain — the workspace
already carries blake3, tokio-util's CancellationToken, sqlx's json/postgres features, and the
Formation/Phalanx/Campaign services whose data flow the bridges must reproduce byte-for-byte. The
risk in this phase is re-deriving something that already exists in a subtly different (and
therefore non-bridging) shape, not missing a library.

## Runtime State Inventory

Not applicable — this phase is additive greenfield work (new modules, new crate features, new CI
jobs). It is not a rename/refactor/migration phase. Skipping per the trigger condition.

## Common Pitfalls

### Pitfall 1: Assuming `cargo semver-checks`'s default crates.io lookup is sufficient
**What goes wrong:** `cargo-semver-checks` defaults to comparing against "the latest normal
version published on crates.io" for each crate name. Since every workspace crate's package name
(e.g. `paladin-ai-core`, not the directory name `paladin-core`) is what's published, an
unqualified `cargo semver-checks check-release` run from the workspace root should resolve
correctly today (0.9.0 is latest) — but if a crate is ever bumped ahead of a full release (e.g. a
`0.9.1` published as a hotfix), the "vs published v0.9.0" requirement in D-06 would silently
diverge from the tool's default baseline.
**Why it happens:** D-06 requires comparing against "the published v0.9.0 crates.io versions"
specifically, but the tool's default is "latest published," which is not necessarily 0.9.0 forever.
**How to avoid:** Pass `--baseline-version 0.9.0` explicitly (or per-crate via the action's
`baseline-version` input) rather than relying on the default, matching D-06's exact wording.
**Warning signs:** A CI `semver` job that passes locally against a stale local baseline but behaves
differently once a crate gets an out-of-band patch release.

### Pitfall 2: Package name vs. directory name vs. lib name confusion in the semver job
**What goes wrong:** `crates/paladin-core/` produces a crate published as `paladin-ai-core` with
lib name `paladin_core`. A CI job or `--package` flag written against the directory name will
silently no-op or error.
**Why it happens:** The workspace was extracted from a monolith (per ADR-0015/0016 history) and
kept the historical `paladin-ai-core` crates.io name while renaming the directory/lib for
ergonomics.
**How to avoid:** Enumerate the `[package] name = "..."` value from each crate's `Cargo.toml`
(verified this session: `paladin-ai-core`, `paladin-ports`, `paladin-battalion`, `paladin-herald`,
`paladin-llm`, `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`,
`paladin-web`, `paladin-ai`) when writing the `package:`/`exclude:` inputs to the semver-checks
action, not the `crates/*` directory names.
**Warning signs:** A "passing" semver job that produces zero diagnostic output because it silently
matched no packages.

### Pitfall 3: `#[non_exhaustive]` is not yet a tree convention
**What goes wrong:** A plan task might assume `PaladinError`/`BattalionError`/`LlmError` are
already `#[non_exhaustive]` (since X-10.2 recommends it) and skip adding it when a later phase
(FT-01) extends them. This phase doesn't touch those enums, but the NEW enums it introduces
(`BattlefieldError`, `WaypointError`, `EngineError`, `WaypointStatus`) should be evaluated for
`#[non_exhaustive]` proactively since they will likely gain variants in Docs 02-04.
**Why it happens:** Verified via grep: zero `#[non_exhaustive]` annotations exist anywhere in
`paladin-core`/`paladin-ports` today. There is no existing precedent to copy from in this codebase.
**How to avoid:** Since these are brand-new types (not pre-existing per X-10's own scoping —
X-10 only mandates the treatment for *pre-existing* public types gaining a variant/field), marking
them `#[non_exhaustive]` from day one is optional but recommended given docs 02-04 are known to
extend `WaypointStatus` (`AwaitingInput`/`Halted` already exist; Doc 03 fleshes out
`AwaitingInput`'s payload) and will extend `EngineError`.
**Warning signs:** A later phase (23-25) needing a deliberate-breaking MIGRATION.md entry for a
type this phase could have future-proofed for free.

### Pitfall 4: `paladin-storage`'s `sqlx` postgres feature needs a facade passthrough too
**What goes wrong:** Adding `postgres = ["dep:sqlx", "sqlx/postgres"]` to `paladin-storage` alone
is invisible to consumers of the `paladin-ai` facade crate unless the facade also gains a
passthrough feature (mirroring how `paladin-storage = { workspace = true, features = ["sqlite"] }`
is already wired in the facade's `[dependencies]`).
**Why it happens:** The existing `sqlite`/`mysql` features already follow this two-layer pattern
(crate feature + facade passthrough); D-01 explicitly calls this out ("a facade passthrough
feature — following the existing feature-gating convention, X-07").
**How to avoid:** Add both the `paladin-storage` feature AND a corresponding facade-level feature
(likely named `postgres` on `paladin-ai` too) that enables `paladin-storage/postgres`.
**Warning signs:** `cargo build -p paladin-ai --features postgres` failing to actually compile the
Postgres adapter, or requiring `--features paladin-storage/postgres` as a workaround.

### Pitfall 5: Determinism test flakiness from HashMap iteration order
**What goes wrong:** `Battlefield.values: HashMap<FieldName, serde_json::Value>` and any
`HashMap<NodeId, NodeSpec>` graph representation have non-deterministic iteration order across
runs (Rust's default hasher is randomized per-process). Serializing a `Battlefield` for the
ENG-FR-08 byte-identical assertion, or computing the blake3 fingerprint (D-04), over raw `HashMap`
iteration will produce different byte sequences run-to-run even with identical logical content.
**Why it happens:** `std::collections::HashMap`'s iteration order is not stable; `serde_json`'s
default `Map` (a `BTreeMap` wrapper unless the `preserve_order` feature is enabled) sorts object
keys alphabetically when serializing **if** the value is `serde_json::Value::Object`, so this risk
is actually already mitigated for `serde_json::Value` fields — but any Rust-level `HashMap<NodeId,
_>` the WarGraph or fingerprint code iterates directly (not through serde_json's own serializer)
is NOT auto-sorted and must be explicitly sorted before hashing/serializing.
**How to avoid:** D-04 already specifies "sorted iteration — never raw `HashMap` order" for the
fingerprint. Apply the same rule to any Rust-level map the engine iterates for
serialization/hashing purposes, not just the fingerprint: collect keys, sort, then iterate.
Confirm which `serde_json` feature flags are active in `paladin-core` (verified: `serde_json`
workspace dep declares no explicit `preserve_order` feature, so its default `Map` type is the
`BTreeMap`-backed one — alphabetically ordered on serialize, which is good for `Battlefield`'s
`values: HashMap<FieldName, Value>` once that outer HashMap is itself converted to a sorted
representation before embedding).
**Warning signs:** The ENG-FR-08 20-iteration repeat test passing locally but failing intermittently
in CI (different process randomization seed) or on a different machine.

## Code Examples

### Existing legacy fan-in the bridge must reproduce exactly
```rust
// Source: crates/paladin-battalion/src/campaign_service.rs:373 (verified in tree at HEAD)
Ok(inputs.join("\n\n---\n\n"))
```

### Existing Campaign cycle-rejection call sites the engine must NOT touch or inherit
```rust
// Source: crates/paladin-battalion/src/campaign_service.rs:236 (verified in tree)
let sorted_nodes = toposort(campaign.graph(), None).map_err(|cycle| {
    // ... cycle.node_id() ...
});

// Source: crates/paladin-core/src/platform/container/battalion/campaign.rs:255 (verified in tree)
if petgraph::algo::toposort(&self.graph, None).is_err() {
    // "Campaign graph contains a cycle, must be a DAG"
}
```

### Existing blake3 usage pattern to follow for the graph fingerprint (D-04)
```rust
// Source: crates/paladin-core/src/base/service/field_version_service.rs:339 (verified in tree)
let hash = blake3::hash(&serialized);
```

### Existing `CancellationToken` usage pattern to follow for ENG-FR-23
```rust
// Source: tests/integration/mcp_streamable_http_test.rs:38,179 (verified in tree)
use tokio_util::sync::CancellationToken;
let ct = CancellationToken::new();
```

### Existing `PaladinPort` shape the engine's `NodeSpec::Paladin` execution must call
```rust
// Source: crates/paladin-ports/src/output/paladin_port.rs:683 (verified in tree)
async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError>;
```

### Existing `CitadelConfig` shape to mirror for new engine config structs (X-09)
```rust
// Source: src/config/citadel.rs:8-31 (verified in tree)
pub struct CitadelConfig {
    pub enabled: bool,
    pub state_dir: String,
    pub autosave_enabled: bool,
    pub cleanup_enabled: bool,
    pub max_state_age_days: Option<u32>,
}
impl Default for CitadelConfig { /* ... */ }
impl CitadelConfig {
    pub fn validate(&self) -> Result<(), String> { /* ... */ }
}
// EnvOverridable trait: src/config/env_utils.rs:33-36
pub trait EnvOverridable {
    fn apply_env_overrides(&mut self);
}
```

### Existing Garrison migration convention to follow for the Waypoints table (D-02)
```sql
-- Source: crates/paladin-memory/migrations/001_create_garrison_tables.sql (verified in tree)
CREATE TABLE IF NOT EXISTS garrison_entries (
    id TEXT PRIMARY KEY NOT NULL,
    paladin_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool')),
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    token_count INTEGER,
    metadata TEXT, -- JSON blob for flexible metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_paladin_timestamp ON garrison_entries(paladin_id, timestamp DESC);
```
The `waypoints` table (`crates/paladin-storage/migrations/00X_create_waypoints_table.sql`) should
follow this same style: `TEXT PRIMARY KEY`/`TEXT` id columns, an explicit `CREATE INDEX` for the
`(thread_id, created_at DESC)` access pattern ENG-FR-16 names, and a `TEXT` payload column per D-02.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| String-in/string-out `PaladinPort::execute` as the only data-flow mechanism | Typed `Battlefield`/`StateDelta` with per-field dispatch rules | This phase (v0.10.0, Doc 01) | Legacy path stays fully supported via bridges (ENG-FR-19/20); nothing is removed |
| Campaign's toposort-enforced DAG-only execution | `WarEngine`'s bounded-superstep cyclic execution | This phase | Campaign itself is untouched; the new engine is additive, not a replacement |
| No automatic checkpointing (Citadel only on explicit invocation) | Automatic Waypoint per superstep | This phase | Citadel remains for whole-entity snapshots; WaypointPort is a new, separate, higher-frequency persistence path |

**Deprecated/outdated:** nothing in this phase is deprecated — X-03 forbids removals, and no `M-B`
entry in the program overview marks anything from Doc 01 as behavior-breaking other than the two
unrelated M-B-02/M-B-03 items owned by later phases.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `[workspace.package]` table does not currently exist and each of the 11 crates declares `rust-version`-eligible fields independently — verified by grep returning zero hits for `[workspace.package]` or `rust-version` anywhere in the tree. This is a **verified fact**, not an assumption, but the *recommended remediation* (introduce a `[workspace.package]` table vs. edit 11 files individually) is Claude's Discretion per the CONTEXT.md, not a locked decision. | Standard Stack / Summary point 2 | If the planner picks per-crate edits instead of a workspace table, the diff is larger but functionally equivalent; low risk either way |
| A2 | `cargo-semver-checks`'s default crates.io-lookup behavior and its `--baseline-version`/action `baseline-version` input syntax, sourced via WebSearch summarizing the tool's GitHub README/Marketplace page, not fetched from the primer README directly in full. | Common Pitfalls #1, Standard Stack | If the action's exact flag name has changed since the search summary was generated, the CI job's YAML syntax needs a one-line correction during implementation — low risk, easily caught by the job failing to parse |
| A3 | `#[non_exhaustive]` being a good idea for the phase's brand-new enums is a recommendation, not a PRD requirement — X-10 only scopes this rule to *pre-existing* public types. | Common Pitfalls #3 | If skipped, Docs 02-04 extending `WaypointStatus`/`EngineError` will need to record those as pre-existing-type changes in a later `MIGRATION.md` §9.2 entry instead of avoiding the entry altogether — a documentation-only cost, not a functional risk |

## Open Questions

1. **Should `[workspace.package]` be introduced this phase, or should `rust-version` be added to each of the 11 `Cargo.toml`s individually?**
   - What we know: Neither exists today; D-07 requires the workspace to "advertise"
     `rust-version = "1.85"`; both approaches satisfy `cargo msrv verify`/the MSRV CI job.
   - What's unclear: Whether introducing a `[workspace.package]` table risks touching unrelated
     fields (license, authors, repository) in a way that could be read as an out-of-scope refactor
     under X-03's stop-and-flag rule, versus just adding the one field per crate.
   - Recommendation: Add `rust-version = "1.85"` directly to each of the 11 `[package]` blocks
     (smallest possible diff, no risk of accidentally changing other package metadata via
     workspace inheritance). Introducing a full `[workspace.package]` table is a larger structural
     change better suited to a dedicated cleanup, not bundled into this phase's MSRV job addition.

2. **Exact `postgres` feature name on the `paladin-ai` facade crate.**
   - What we know: The existing pattern is `paladin-storage = { workspace = true, features =
     ["sqlite"] }` in the facade's `[dependencies]`, with `sqlite`/`mysql` as facade-level feature
     names too (needs confirming against the facade `Cargo.toml`'s own `[features]` table, which
     was not fully read this session).
   - What's unclear: Whether the facade's `[features]` table names its passthrough features
     identically to `paladin-storage`'s (`sqlite`, `mysql`) or with a prefix.
   - Recommendation: The planner/implementer should read `Cargo.toml`'s (repo root) `[features]`
     table in full before naming the new `postgres` feature, and mirror whatever naming convention
     `sqlite`/`mysql` already use there.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| SQLite (via sqlx) | `SqliteWaypointStore` contract tests (Tier 1) | Yes (in-process, no external service) | sqlx 0.8 `sqlite` feature, already enabled | — |
| Postgres | `PostgresWaypointStore` contract tests (Tier 2) | Not running by default; no `postgres` service exists yet in `docker/docker-compose.test.yml` (verified: only `redis-test`, `minio-test`, `ollama-test` services present) | — | D-10 requires adding a `postgres` service to the docker-compose test file; `make test-integration-docker` is the sanctioned Tier 2 target |
| Docker / docker-compose | Tier 2 Postgres contract tests | Assumed available in CI (existing `test-integration-docker` Makefile target already used by Ollama Tier 2 tests) | — | — |
| Rust 1.85 toolchain (MSRV) | New `msrv` CI job (D-07) | Not currently pinned anywhere; `rust-toolchain.toml` pins dev toolchain to `1.97.1` | — | MSRV job must explicitly install/pin 1.85 via `dtolnay/rust-toolchain@1.85` or equivalent, independent of `rust-toolchain.toml`'s 1.97.1 (which would otherwise override it, per the existing `test` job's own `RUSTUP_TOOLCHAIN` override pattern) |
| `cargo-semver-checks` | New `semver` CI job (D-06) | Not installed in the tree; must be installed via `cargo install cargo-semver-checks --locked` or the `obi1kenobi/cargo-semver-checks-action` | 0.50.0 current max stable [CITED: crates.io] | — |

**Missing dependencies with no fallback:**
- None — every gap above (Postgres service, MSRV toolchain pin, semver-checks tool) has a
  documented, in-scope path to add it as part of this phase's own deliverables (ENG-05, ENG-08).

**Missing dependencies with fallback:**
- None beyond the above; all are phase deliverables, not external blockers.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), `#[tokio::test]`/`#[tokio::test(flavor = "multi_thread")]`, `criterion` 0.5 for benches |
| Config file | none dedicated — workspace `Cargo.toml` + per-crate `[dev-dependencies]` |
| Quick run command | `cargo test -p paladin-core --lib` / `cargo test -p paladin-battalion --lib` (Tier 1, no services) |
| Full suite command | `make test-all` (unit + integration); `make test-integration-docker` for Tier 2 Postgres contract tests |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ENG-01 | Schema enforcement, dispatch rules, typed accessors | unit | `cargo test -p paladin-core battlefield::` | ❌ Wave 0 |
| ENG-02 | Superstep loop, cycles, determinism, join/defer | unit + `#[tokio::test(flavor="multi_thread")]` | `cargo test -p paladin-battalion engine::` | ❌ Wave 0 |
| ENG-03 | Automatic Waypoint per superstep, Strict durability failure | unit + contract | `cargo test -p paladin-storage waypoint_contract::` | ❌ Wave 0 |
| ENG-04 | Resume with zero re-execution (E2E-1) | integration | `cargo test --test e2e_crash_resume` | ❌ Wave 0 |
| ENG-05 | Three backends pass shared contract suite | contract (Tier 1 for InMemory/SQLite) + Tier 2 (Postgres) | `cargo test -p paladin-storage --features sqlite waypoint_contract::` / `make test-integration-docker` | ❌ Wave 0 |
| ENG-06 | Legacy bridges golden-equivalence | integration | `cargo test --test golden_bridge_equivalence` | ❌ Wave 0 |
| ENG-07 | TraceSink/NodeInterceptor/CancellationToken seams | unit | `cargo test -p paladin-battalion engine::hooks::` | ❌ Wave 0 |
| ENG-08 | MIGRATION.md complete, CI jobs green | manual/CI | `cargo semver-checks check-release --baseline-version 0.9.0`; MSRV job in CI | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p paladin-core --lib` / `-p paladin-battalion --lib` / `-p paladin-storage --lib` (whichever crate the task touched), plus `cargo fmt --check` and `cargo clippy -- -D warnings`.
- **Per wave merge:** `cargo test` (full workspace) + `cargo test --doc`.
- **Phase gate:** `make test-all` green, plus `make test-integration-docker` for the Postgres contract suite, plus the new `semver` and `msrv` CI jobs green, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/paladin-core/src/platform/container/battlefield.rs` + `#[cfg(test)] mod tests` — covers ENG-01
- [ ] `crates/paladin-core/src/platform/container/waypoint.rs` + `#[cfg(test)] mod tests` — covers ENG-03 type shapes
- [ ] `crates/paladin-ports/src/output/waypoint_port.rs` — covers ENG-03 port contract
- [ ] `crates/paladin-storage/src/waypoint/contract_tests.rs` (shared generic test fns, D-09) — covers ENG-05
- [ ] `crates/paladin-battalion/src/engine/mod.rs` + `#[cfg(test)] mod tests` — covers ENG-02
- [ ] `tests/e2e_crash_resume.rs` (or under `tests/integration/`) — covers ENG-04/E2E-1
- [ ] `tests/golden_bridge_equivalence.rs` — covers ENG-06
- [ ] `crates/paladin-storage/migrations/00X_create_waypoints_table.sql` — schema for ENG-05 SQLite
- [ ] `.github/workflows/ci.yml` `semver` and `msrv` jobs — covers ENG-08
- [ ] Framework install: none — `cargo test`/`criterion` already fully set up; only new test *files*, not new test *tooling*, are needed

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | This phase has no HTTP/auth surface (deferred to Doc 06/Phase 27) |
| V3 Session Management | No | `ThreadId`/`WaypointId` are workflow identifiers, not auth sessions |
| V4 Access Control | No | No new access-control surface this phase |
| V5 Input Validation | Yes | `BattlefieldSchema` enforcement (`UnknownField`, `MissingRequiredField`, `TypeMismatch`) is itself the input-validation control for typed state; `ThreadId` validation (non-empty, ≤256 chars, no whitespace per PRD §3.3) is a straightforward newtype constructor validation |
| V6 Cryptography | Marginal | `blake3` fingerprint is a content hash for equality/change-detection, not a security control (not used for auth, integrity-against-tampering, or secrets) — no cryptographic requirement beyond "use an existing, already-vetted hash function," which D-04 already satisfies |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SQL injection in `SqliteWaypointStore`/`PostgresWaypointStore` | Tampering | `sqlx`'s compile-time-checked or bound-parameter query macros (already the convention elsewhere in `paladin-storage`/`paladin-memory`) — never string-interpolate `thread_id`/`waypoint_id` into SQL |
| Deserialization of untrusted/stale schema versions causing a panic or silent misparse | Tampering / Denial of Service | X-04's mandatory `schema_version` field + typed `SchemaVersionUnsupported` error — this phase must implement the check, not assume serde will fail gracefully on its own (serde will happily deserialize a structurally-compatible-but-semantically-wrong-version payload) |
| Unbounded Waypoint growth exhausting storage (a long-running cyclic workflow persisting every superstep) | Denial of Service | `WaypointRetentionConfig` (ENG-FR-18) — must never delete the latest Waypoint or an `AwaitingInput` Waypoint, but otherwise bounds storage growth; `max_supersteps`/`max_node_visits` (ENG-FR-03) additionally bound the *number* of Waypoints a single run can ever produce |
| A malicious or buggy `Custom` dispatch rule / `NodeInterceptor` panicking and taking down the whole engine | Denial of Service | Not explicitly required by the PRD for this phase (no FR mandates `catch_unwind` around custom dispatch or interceptors) — flag as a gap: the TraceSink (ENG-FR-21) is explicitly required to survive a panicking/slow sink, but the PRD does not make the same requirement for `Custom` dispatch closures or `NodeInterceptor`. Recommend the plan at minimum document this asymmetry; wrapping arbitrary closures in `catch_unwind` is Claude's Discretion since the PRD is silent here. |

## Sources

### Primary (HIGH confidence)
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` — full FR/type-shape/test-plan source of truth, read in full this session
- `.project/v0.10.0/00-program-overview.md` — cross-cutting X-01…X-11 rules, ubiquitous language, E2E-1, MIGRATION.md structure, read in full this session
- `.planning/phases/22-battlefield-state-superstep-engine/22-CONTEXT.md` — locked D-01…D-12 decisions, read in full this session
- `.planning/REQUIREMENTS.md` — ENG-01…08 requirement text and traceability, read in full this session
- Direct tree reads this session: `Cargo.toml` (workspace deps), `crates/paladin-core/Cargo.toml`, `crates/paladin-ports/Cargo.toml`, `crates/paladin-battalion/Cargo.toml`, `crates/paladin-storage/Cargo.toml`, `crates/paladin-memory/migrations/001_create_garrison_tables.sql`, `crates/paladin-battalion/src/campaign_service.rs` (toposort + fan-in), `crates/paladin-core/src/platform/container/battalion/campaign.rs`, `crates/paladin-ports/src/output/paladin_port.rs`, `tests/helpers/mock_paladin_port.rs`, `src/application/services/orchestration/listener.rs` (X-05 stress pattern), `src/config/citadel.rs`, `src/config/env_utils.rs`, `.github/workflows/ci.yml` (full job list, existing `api-surface`/`test`/`publish-dry-run` jobs), `docker/docker-compose.yml`, `docker/docker-compose.test.yml`, `Makefile` (`test-integration-docker` target), `rust-toolchain.toml`, `.planning/decisions/0015-core-ports-dependency-allowlist.md`, `.planning/decisions/0016-port-value-type-ownership.md`, `crates/paladin-core/src/platform/container/paladin_error.rs`, `crates/paladin-core/src/platform/container/battalion/mod.rs` (BattalionError variants), grep for `#[non_exhaustive]` (zero hits), grep for `blake3::` and `CancellationToken` usage across the tree

### Secondary (MEDIUM confidence)
- docs.rs feature-flags page for `uuid` 1.8.0 (fetched via WebFetch this session) — confirms `v7` feature exists at the pinned version
- docs.rs feature-flags page for `sqlx` 0.8.0 (fetched via WebFetch this session) — confirms `postgres` feature name
- crates.io API responses for `uuid`, `sqlx`, `cargo-semver-checks` max-stable versions (fetched via `curl` this session)
- WebSearch summaries of `obi1kenobi/cargo-semver-checks-action` README and `cargo-semver-checks` CLI baseline flags — used for the CI job design; the underlying README was not fetched verbatim, only summarized by the search tool

### Tertiary (LOW confidence)
- None — every claim above is either read directly from the tree this session or cited to an external doc/registry fetched this session. See the Assumptions Log for the two items (A1, A2) with the lowest-confidence provenance.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every library named is already in the tree; only feature-flag additions verified live against docs.rs/crates.io
- Architecture: HIGH — type shapes and FR semantics are locked in the PRD; tree facts (toposort call sites, fan-in separator, PaladinPort signature) verified by direct read
- Pitfalls: HIGH for tree-fact pitfalls (package naming, feature wiring, HashMap ordering); MEDIUM for the `cargo-semver-checks` exact CLI flag syntax (sourced via WebSearch summary, not the primary README verbatim)

**Research date:** 2026-09-01
**Valid until:** 2026-10-01 (30 days — stable domain; the only fast-moving element is `cargo-semver-checks`'s own CLI surface, worth re-checking if the phase's implementation is delayed past that window)
