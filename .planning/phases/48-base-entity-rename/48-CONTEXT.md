# Phase 48: Base Entity Rename - Context

**Gathered:** 2026-10-10 (code survey) / 2026-10-12 (phase added)
**Status:** Draft — ground truth surveyed at `61de2de` on 2026-10-10 and **re-verified at
`25dc267` on 2026-10-12, after Phase 44 sealed** (every count below is from the post-44 tree;
Phase 44's 128-file change touched neither `crates/paladin-core/src/base/` nor any of the eight
`Node<T>`-based container files — `git diff --stat 61de2de..HEAD` on them is empty). The
decisions marked
*recommended* below are the surveyor's proposal and have not been confirmed by the owner. Run
`/gsd-discuss-phase 48` to confirm or change them, or `/gsd-plan-phase 48` to accept them as-is.
**Mode:** owner-requested phase, created from a conversation rather than `--auto`

<domain>
## Phase Boundary

One rename, its fallout, and the cleanup it exposes:

1. **The rename (ENTITY-01, ENTITY-02, ENTITY-03).** The base-layer wrapper
   `paladin_core::base::entity::node::Node<T>` becomes `paladin_core::base::entity::Entity<T>`;
   its payload field `node` becomes `data`. Every alias (`Paladin`, `Council`, `Grove`, `User`,
   `Log`) and every wrapper struct (`ContentItem { node: Node<ContentData> }`,
   `PromptItem { node: Node<PromptData> }`) follows. The version-history family
   `NodeVersion` / `NodeVersionMetadata` / `NodeVersionRepository` / `NodeVersionService` /
   `InMemoryNodeVersionRepository` (`base/service/node_version_service.rs`) becomes
   `EntityVersion*`. A deprecated `Node<T>` alias and the old module path survive one minor
   release. Persisted JSON keeps loading through `#[serde(alias = "node")]`.

2. **The base-layer disposition (ENTITY-04).** `Field`, `Collection`, `Element`,
   `FieldVersionService` and `CollectionVersioningService` have zero consumers outside `base/`.
   Each is removed, or kept with a written reason. The `version: bool` flag on the wrapper gets a
   rustdoc contract.

3. **The vocabulary rule (ENTITY-05).** "Node" is reserved for the graph sense. An ADR says so;
   CLAUDE.md / `.github/copilot-instructions.md`'s naming table and the mdBook architecture pages
   agree; `MIGRATION.md` §9.2 and the semver allowlist carry the breaks.

**Not in this phase:** any change to the graph vocabulary (`NodeId`, `NodeSpec`, `StateNode`,
`NodeContext`, `NodeCache*`, `NodeInterceptor`, `NodeExecutionRecord`, `NodeOutcomeKind`,
`NodeProgressKind`, `TraceEvent::Node*`, `docs/schemas/wargraph-doc.schema.json`, the
`node_cache` config section, Redis node-cache keys); the legacy `battalion::NodeError`
removal (already done — Phase 44, LEGACY-01, sealed 2026-10-11, pinned by
`tests/legacy_removal_guard.rs`); migrating the owned downstream system (it adopts the milestone
at once per the clean-break policy, or compiles against the alias if D-02's alternative is
chosen); designing a general revisioning
system for the platform (see Deferred); renaming `platform/container/` or splitting it.

</domain>

<ground_truth>
## What the survey found (tree at `61de2de`, 2026-10-10)

### The base layer today

`crates/paladin-core/src/base/` is laid out as originally designed — `entity/` (node,
collection, field, message), `component/` (action, element, event), `service/` (five
services) — but most of it is dormant:

| Base item | Consumers outside `base/` |
|---|---|
| `Node<T>` (`base/entity/node.rs`, 188 lines) | heavy — see below |
| `Message`, `Action`, `Event`, `MessageService`, `AnalysisService` | 7–21 files each |
| `Field`, `Collection`, `Element` | **0** |
| `FieldVersionService`, `CollectionVersioningService` | **0** |
| `NodeVersionService` / `NodeVersionRepository` | only `src/core/platform/manager/content_service.rs` (root crate) and `tests/functional/content_{lifecycle,fetching_pipeline}_test.rs` |

`Node<T>` carries `uuid`, `created`, `modified`, `name: Option<String>`, the payload field
`node: T`, and `version: bool`. The flag only records whether versioning is "enabled"; nothing
connects a Paladin, Council, Grove, User or Log to a revision store. The only honoured path is
`ContentItem` → `NodeVersionService`. When Phase 27 needed versioned Assistants it built
`AssistantVersion { version: u32, … }` independently.

`platform/manager/` inside `paladin-core` still holds only `user_service.rs`.

### What is built on `Node<T>`

Only 8 of the 70 files in `crates/paladin-core/src/platform/container/` use it:

- `paladin.rs:229` — `pub type Paladin = Node<PaladinData>;`
- `citadel.rs:48` — a **duplicate** `pub type Paladin = Node<PaladinData>;` (remove)
- `battalion/council.rs:268` — `Council`; `battalion/grove.rs:291` — `Grove`
- `user.rs:165` — `User`; `log.rs:397` — `Log`
- `content.rs:12` and `prompt.rs:11` — `ContentItem` / `PromptItem` hold a `node: Node<…>` field

Everything newer stands alone with its own typed id and `schema_version`: `Run`, `Assistant`,
`Waypoint`, `VaultRecord`, `Battlefield`, `TraceEvent`, the treasury ledger, `CachedDelta`.
`platform/container` has become a general folder of domain types, not a layer extending base.

### Where the graph sense came from, and how big it is

The graph vocabulary arrived with the v0.10.0 superstep engine (`WarGraph`, Phase 22,
2026-09-02), grew in Phase 25 ("Node-Level Fault Tolerance") and again in Phase 43 (Redis node
cache). The older `Campaign` already used petgraph's `NodeIndex`. Git history is squashed, so
these dates come from `.planning/ROADMAP.md`.

| Graph sense (workspace `.rs` at `25dc267`, excluding `base/entity/node.rs`) | Occurrences | Files |
|---|---|---|
| `NodeId` | 1,910 | 91 |
| `node_id` | 1,151 | 85 |
| `NodeSpec` | 1,034 | 64 |
| `StateNode` | 410 | 42 |
| `NodeError` (one type since Phase 44) | 396 | 56 |
| `NodeCache` / `NodeCacheKey` / `NodeCachePort` | 378 / 96 / 65 | 20 / 8 / 12 |
| `NodeOutcomeKind` | 208 | 27 |
| `NodeContext` | 192 | 47 |
| `NodeExecutionRecord` | 72 | 15 |
| `NodeInterceptor` | 47 | 8 |
| `NodeDoc` (graph export) | 44 | 3 |
| `NodeProgressKind` | 22 | 6 |

| Entity sense | Occurrences | Files |
|---|---|---|
| `use …base::entity::node` | 148 | 117 |
| `Node::new` | 334 | 117 |
| `.node` payload access (a handful are `graph.node(..)`) | 739 | 107 |
| `Node<` in signatures | 25 | 17 |
| mdBook / README mentions of `Node<T>` | — | 9 (`docs/src/architecture/{domain-model,design-patterns,overview,crate-map}.md`, `docs/src/api-reference/{crate-map,stable-api}.md`, `docs/src/assets/LayerArchitecture.svg`, `README.md`, `crates/paladin-core/README.md`) |
| `examples/`, `doc-examples`, `benches/` | — | 13 |
| `api_surface_current.txt` lines naming `node_version_service` | 9 | 1 |

Public wire formats that carry the graph sense and must not move: `wargraph-doc.schema.json`
(`nodes`, `max_node_visits`), `TraceEvent::NodeStarted/NodeFinished/NodeProgress`, persisted
`Waypoint` / `NodeExecutionRecord`, `CachedDelta` + Redis keys, `config.example.yml`'s
`node_cache` section and `src/config/node_cache.rs`. Migrations mention it only in a comment
(`crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql:17`).

### The collision is already in the code

- `NodeSpec::Paladin { paladin: Box<Paladin> }` (`crates/paladin-battalion/src/engine/graph.rs:42`)
  — a graph node wrapping an entity Node; its rustdoc has to say "a `Node<PaladinData>`".
- In `engine/superstep.rs`, `paladin.node` (entity payload), `graph.node(node_id)` (graph
  lookup) and `progress.node == NodeId::new("planner")` (graph id) appear in the same file.
- At survey time two unrelated types were both called `NodeError`: the legacy
  `battalion::NodeError { node_name, error }` (`battalion/mod.rs:520`) and the structured
  `platform::container::node_error::NodeError` (`node_error.rs:152`), with a warning comment and
  two pinning tests telling them apart. **Phase 44 (LEGACY-01, sealed 2026-10-11) removed the
  legacy one** — re-verified on `846a5cf`: `node_error.rs:152` is now the only `pub struct
  NodeError` in the workspace, and `BattalionResult.node_errors` was retyped to it (44-06).

### Why rename the entity and not the graph

1. The graph sense is the one every reader and every dependency (petgraph, LangGraph-style
   docs) expects; renaming it would work against both.
2. The graph sense is ~5× larger and is in public wire formats; renaming it breaks JSON
   schemas, trace streams, cache keys and config, not just Rust.
3. The entity rename is mechanical, the module is already called `base::entity`, and the owner
   described nodes as "a type of entity".
4. The one argument the other way — the owned downstream system uses "node" in the entity sense
   — is answered by the deprecated alias (ENTITY-02): it keeps compiling and migrates with the
   same find-and-replace later.

</ground_truth>

<decisions>
## Implementation Decisions

### Carried forward (locked by earlier phases — not re-asked)

- **D-00a:** Shipped tree outranks any document. 82 % workspace line-coverage floor (ADR-0006).
  `make clean-code`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an
  intentional public-surface change), `make security`, `make check-api-examples` and the manual
  credential-handling review gate every commit.
- **D-00b:** X-03 governs public API: every break needs a `MIGRATION.md` §9.2 row and, when
  marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry naming one crate and one lint, kept
  set-equal by `make check-migration-allowlist`. This phase is a deliberate break and will have
  several rows. **X-03 as written says "deprecations allowed, removals are not, before
  v0.11.0"** (PROJECT.md *X-03 supersession*); it has been superseded twice by the overview's
  clean-break policy (§5.1) — for Phases 31-33, and for Phase 44 by ADR-0059 — on the recorded
  grounds that the project is pre-1.0 with one coordinated downstream consumer that pins its
  submodule pointer and adopts a whole milestone at once. `MIGRATION.md` §9.7 Deprecations is
  deliberately empty: no phase has ever marked an item `#[deprecated]`. A Phase 48 clean break
  therefore needs its own recorded, phase-scoped X-03 supersession (ADR-0060, the ADR-0059
  shape); a deprecated alias instead would follow X-03 as written and make §9.7 non-empty for
  the first time. See D-02.
- **D-00e:** Phase 44 sealed 2026-10-11 (ADR-0059; `tests/legacy_removal_guard.rs`). It touched
  none of this phase's files. Its guard bans a *declaration or alias* of `NodeError` in the core
  battalion module and the `battalion::NodeError` path anywhere — nothing this phase does trips
  it, but every push must keep `cargo test --test legacy_removal_guard` green.
- **D-00c:** The two-officer model and the Treasurer vocabulary (ADR-0049/0050, Phase 30 VOCAB)
  are untouched; this phase adds a vocabulary rule, it does not revisit one.
- **D-00d:** `paladin-core` has zero dependencies (ADR-0015); nothing in this phase may add one.

### Proposed for this phase (*recommended*, open for `/gsd-discuss-phase 48`)

- **D-01 (recommended): new name is `Entity<T>`**, module `base::entity::entity` (or flattened
  to `base::entity::Entity` — planner's call), payload field `data`. The base layer predates the
  Medieval Military theme and is meant to sit under any system, so it keeps a generic name; the
  theme stays on domain concepts (Paladin, Battalion, …). Alternatives considered: `Record<T>`
  (collides with `VaultRecord` in prose), `Versioned<T>` (over-promises — see D-04),
  `Aggregate<T>` (DDD-loaded; `Paladin` is the aggregate root, the wrapper is not).
- **D-02 (recommended, revised 2026-10-12 after re-reading X-03): clean break, no alias.**
  `Node<T>` and the `base::entity::node` module are removed outright, matching Phases 31-33 and
  44 and the stated reason for both (one coordinated downstream consumer that adopts the whole
  milestone at once). ADR-0060 records the Phase-48-only X-03 supersession in the ADR-0059 shape;
  `MIGRATION.md` §9.7 stays empty. **Alternative (X-03 as written):**
  `#[deprecated(since = "0.11.0", note = "renamed to Entity<T>; removed in 0.12.0")] pub type
  Node<T> = Entity<T>;` plus a re-exporting `base::entity::node` module, no in-tree use surviving
  (so `-D warnings` stays clean), removal scheduled in §9.7 for v0.12.0. Choose the alternative
  only if the downstream system needs to straddle v0.11 and the rename rather than adopt the
  milestone at once — that is the owner's call (open question 2). ENTITY-02 is written to accept
  either.
- **D-03 (recommended): serde compatibility by alias, not by version bump.** The payload field
  gets `#[serde(alias = "node")]` so pre-rename JSON (Citadel snapshots, stored Paladins,
  SQLite/Postgres rows) deserializes; new writes use `data`. A fixture test loads a captured
  pre-rename `Paladin` and `ContentItem` JSON. No `schema_version` is added to `Entity<T>` —
  the wrapper never had one and the newer records version themselves.
- **D-04 (recommended):** `version: bool` keeps its name and gets a contract, not a feature.
  Rustdoc states: the flag declares that an entity *wants* revision history; it is honoured only
  by a caller that routes writes through `EntityVersionService` (today: `ContentItem` via the
  root crate's `ContentItemService`). Designing platform-wide revisioning is deferred (see
  below) — this phase must not grow it.
- **D-05 (recommended): base-layer disposition.** Remove `Field`, `Collection`, `Element`,
  `FieldVersionService` and `CollectionVersioningService` (0 consumers; ~1,800 lines of code and
  tests that the 82 % floor currently counts). If the owner wants any kept for the downstream
  system, keep it with a one-line reason in `base/mod.rs` rustdoc and a §9.2 "retained" note —
  do not keep it silently. Removing `Field`/`Collection` also ends a second, smaller collision
  with the engine's `FieldName`/`FieldSpec` in `battlefield.rs`.
- **D-06 (recommended): sequencing.** Runs after Phase 44 (sealed 2026-10-11, so there is one
  `NodeError` and the Battalion files are quiet) and before Phase 46 (so the v0.10 → v0.11
  migration guide, CURR-23, carries the rename) and Phase 47 (so v0.11.0 ships it). The duplicate
  `Paladin` alias in `citadel.rs:48` is removed here, not in 44.
- **D-07 (recommended): graph side is frozen, old names stay gone.** A repository-scan test in
  the exact shape of Phase 44's `tests/legacy_removal_guard.rs` (std only; forbidden strings
  assembled with `concat` so the file never contains one whole; two positive controls over
  temporary trees; file-scoped rules report a missing target; skip set = dot-directories,
  `CHANGELOG.md`s, root `MIGRATION.md`, the two upgrade guides, itself) with rules for: (a) no
  `base::entity::node` path and no `Node::new`/`Node<` entity use outside the alias module if
  D-02's alternative is chosen; (b) the graph identifiers under *Not in this phase* still defined
  at their current files. The ADR names the rule: *"node" = a vertex in a WarGraph/Campaign;
  "Entity" = the base wrapper.* CLAUDE.md's naming table and `.github/copilot-instructions.md`
  get an `Entity` row.

### Mechanics the planner should expect

- Bulk edit shape: `base::entity::node::Node` → `base::entity::Entity` in 117 files; `.node` →
  `.data` in ~106 files, **excluding** `graph.node(` / `progress.node` / `task.node` and the
  `node_name`/`node_id` fields (a regex on `\.node\b` followed by `.` or `;` or `)` catches the
  payload accesses; review each engine-file hit by hand — `cache_key.rs:240` is entity,
  `superstep.rs:1923` is graph).
- `api_surface_current.txt` changes on every renamed public item; refresh once at the end with
  `make api-surface-update`, not per commit.
- `cargo semver-checks` lints expected: `struct_missing` / `struct_pub_field_missing` /
  `module_missing` / `trait_missing` on `paladin-core`; `function_signature_changed` on the root
  crate's `ContentItemService` (its signatures name `NodeVersion<…>`). Each is one §9.2 row + one
  allowlist entry. §9.2 rows have **no row IDs** — columns are `Crate | Type / Trait | Change |
  Mitigation | Deliberate-breaking? | Req`, and the Crate cell is the **published** name
  (`paladin-ai-core` for `crates/paladin-core`). Allowlist entries are
  `[[entry]] crate / lint / migration_row / requirement_id / justification` (48 entries today);
  `make check-migration-allowlist` enforces row-level set equality. Precedent to copy: the
  Phase 44 rows landed by plan 44-11 (`e767fc7`) and ADR-0059 (plan 44-02).
- Docs: 9 mdBook/README files, `LayerArchitecture.svg` (text inside the SVG), `crates/paladin-core/
  README.md`, `src/core/mod.rs` module docs (still describes `Node<T>`), rustdoc on `paladin.rs`
  lines 4-11 and 140-229 and `council.rs:246`.
- Examples: 13 files under `examples/`, `doc-examples`, `benches/`; `make check-api-examples`
  must stay green.

</decisions>

<deferred>
## Deferred Ideas

- **Platform-wide revisioning.** The original base design wanted every entity revisionable. Today
  only `ContentItem` is, and `Assistant` versions itself. Deciding whether revisioning belongs on
  `Entity<T>` (one service, one repository trait) or stays per-aggregate is a design decision for
  a later milestone; this phase only documents the current contract (D-04).
- **Splitting `platform/container/`.** 70 files, of which 8 are base-derived and the rest are
  self-standing records. Whether to split by bounded context (engine / platform API / treasury /
  legacy) is out of scope here.
- **Migrating the owned downstream system.** It compiles against the deprecated alias; its own
  find-and-replace happens on its own schedule before v0.12.0 removes the alias.
- **Renaming `platform/container/log.rs`'s `Log = Node<LogContainer>`** to something that does not
  read like a log *line* — noticed, not in scope.

</deferred>

<open_questions>
## For the owner (answered in `/gsd-discuss-phase 48` or by editing D-01..D-07 above)

1. Is `Entity<T>` / `.data` the name, or do you want a themed or different generic name?
2. Clean break (D-02 as recommended: no alias, ADR-0060 supersedes X-03 for this phase, §9.7
   stays empty — the Phase 31-33 / 44 pattern), or a `#[deprecated]` `Node<T>` alias for one
   release (X-03 as written) so the downstream system can straddle the rename?
3. Remove all five dormant base items, or keep any of them for the downstream system?
4. Should the rename land in the same PR as Phase 44's legacy removal, or as its own PR after
   44 merges? (D-06 assumes its own PR, after.)
</open_questions>
