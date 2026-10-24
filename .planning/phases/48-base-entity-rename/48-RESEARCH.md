# Phase 48: Base Entity Rename - Research

**Researched:** 2026-10-13
**Domain:** Rust public-API rename and cleanup inside a hexagonal workspace (serde wire compatibility, cargo-semver-checks / cargo-public-api gates, repository-scan guard tests, MIGRATION.md / ADR registers)
**Confidence:** HIGH (the whole rename was rehearsed end to end on a scratch copy of HEAD `1a83bb6`; every gate named below was run there except the 82 % workspace coverage job and the CI `semver` job against v0.9.0)

<user_constraints>
## User Constraints (from CONTEXT.md)

> **Superseded banner (annotated 2026-10-13, plan-checker revision).** The decisions below are the surveyor's draft this research was written against. The owner answered every open question at `/gsd-discuss-phase 48` on 2026-10-13, and **`48-CONTEXT.md` is now the binding record — see its D-15** (the nine plans are regenerated from it) and D-01..D-14. Where this file and CONTEXT.md differ, CONTEXT.md wins; in particular this file's D-05 removal guidance is overridden by CONTEXT D-05 (all five base items retained pending review) and Pitfall 4's "do not unify" recommendation by CONTEXT D-10 (placeholder unified). The original banner called CONTEXT.md a surveyor's draft whose D-01..D-07 were unconfirmed; that is no longer true. The research text below is kept as written and annotated where overridden.

### Locked Decisions

**Carried forward (locked by earlier phases — not re-asked)**

- **D-00a:** Shipped tree outranks any document. 82 % workspace line-coverage floor (ADR-0006). `make clean-code`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional public-surface change), `make security`, `make check-api-examples` and the manual credential-handling review gate every commit.
- **D-00b:** X-03 governs public API: every break needs a `MIGRATION.md` §9.2 row and, when marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry naming one crate and one lint, kept set-equal by `make check-migration-allowlist`. This phase is a deliberate break and will have several rows. **X-03 as written says "deprecations allowed, removals are not, before v0.11.0"** (PROJECT.md *X-03 supersession*); it has been superseded twice by the overview's clean-break policy (§5.1) — for Phases 31-33, and for Phase 44 by ADR-0059 — on the recorded grounds that the project is pre-1.0 with one coordinated downstream consumer that pins its submodule pointer and adopts a whole milestone at once. `MIGRATION.md` §9.7 Deprecations is deliberately empty: no phase has ever marked an item `#[deprecated]`. A Phase 48 clean break therefore needs its own recorded, phase-scoped X-03 supersession (ADR-0060, the ADR-0059 shape); a deprecated alias instead would follow X-03 as written and make §9.7 non-empty for the first time. See D-02.
- **D-00e:** Phase 44 sealed 2026-10-11 (ADR-0059; `tests/legacy_removal_guard.rs`). It touched none of this phase's files. Its guard bans a *declaration or alias* of `NodeError` in the core battalion module and the `battalion::NodeError` path anywhere — nothing this phase does trips it, but every push must keep `cargo test --test legacy_removal_guard` green.
- **D-00c:** The two-officer model and the Treasurer vocabulary (ADR-0049/0050, Phase 30 VOCAB) are untouched; this phase adds a vocabulary rule, it does not revisit one.
- **D-00d:** `paladin-core` has zero dependencies (ADR-0015); nothing in this phase may add one.

**Proposed for this phase (*recommended*, open for `/gsd-discuss-phase 48`)**

- **D-01 (recommended): new name is `Entity<T>`**, module `base::entity::entity` (or flattened to `base::entity::Entity` — planner's call), payload field `data`. The base layer predates the Medieval Military theme and is meant to sit under any system, so it keeps a generic name; the theme stays on domain concepts (Paladin, Battalion, …). Alternatives considered: `Record<T>` (collides with `VaultRecord` in prose), `Versioned<T>` (over-promises — see D-04), `Aggregate<T>` (DDD-loaded; `Paladin` is the aggregate root, the wrapper is not).
- **D-02 (recommended, revised 2026-10-12 after re-reading X-03): clean break, no alias.** `Node<T>` and the `base::entity::node` module are removed outright, matching Phases 31-33 and 44 and the stated reason for both (one coordinated downstream consumer that adopts the whole milestone at once). ADR-0060 records the Phase-48-only X-03 supersession in the ADR-0059 shape; `MIGRATION.md` §9.7 stays empty. **Alternative (X-03 as written):** `#[deprecated(since = "0.11.0", note = "renamed to Entity<T>; removed in 0.12.0")] pub type Node<T> = Entity<T>;` plus a re-exporting `base::entity::node` module, no in-tree use surviving (so `-D warnings` stays clean), removal scheduled in §9.7 for v0.12.0. Choose the alternative only if the downstream system needs to straddle v0.11 and the rename rather than adopt the milestone at once — that is the owner's call (open question 2). ENTITY-02 is written to accept either.
- **D-03 (recommended): serde compatibility by alias, not by version bump.** The payload field gets `#[serde(alias = "node")]` so pre-rename JSON (Citadel snapshots, stored Paladins, SQLite/Postgres rows) deserializes; new writes use `data`. A fixture test loads a captured pre-rename `Paladin` and `ContentItem` JSON. No `schema_version` is added to `Entity<T>` — the wrapper never had one and the newer records version themselves.
- **D-04 (recommended): `version: bool` keeps its name and gets a contract, not a feature.** Rustdoc states: the flag declares that an entity *wants* revision history; it is honoured only by a caller that routes writes through `EntityVersionService` (today: `ContentItem` via the root crate's `ContentItemService`). Designing platform-wide revisioning is deferred (see below) — this phase must not grow it.
- **D-05 (recommended): base-layer disposition.** Remove `Field`, `Collection`, `Element`, `FieldVersionService` and `CollectionVersioningService` (0 consumers; ~1,800 lines of code and tests that the 82 % floor currently counts). If the owner wants any kept for the downstream system, keep it with a one-line reason in `base/mod.rs` rustdoc and a §9.2 "retained" note — do not keep it silently. Removing `Field`/`Collection` also ends a second, smaller collision with the engine's `FieldName`/`FieldSpec` in `battlefield.rs`.
  - **OVERRIDDEN by 48-CONTEXT.md D-05 (owner, 2026-10-13):** all five items are **kept, retained pending review** — they are scaffolding of the unfinished content / general-data-orchestration system; the reason is written once in `crates/paladin-core/src/base/mod.rs` rustdoc naming `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`; no §9.2 row (nothing breaks); nothing in the five files changes beyond what the `Node<T>` → `Entity<T>` sweep touches. The removal recommendation and its ~1,800-line / coverage argument above are void for this phase (recorded only as an ADR-0060 Considered Option).
- **D-06 (recommended): sequencing.** Runs after Phase 44 (sealed 2026-10-11, so there is one `NodeError` and the Battalion files are quiet) and before Phase 46 (so the v0.10 → v0.11 migration guide, CURR-23, carries the rename) and Phase 47 (so v0.11.0 ships it). The duplicate `Paladin` alias in `citadel.rs:48` is removed here, not in 44.
- **D-07 (recommended): graph side is frozen, old names stay gone.** A repository-scan test in the exact shape of Phase 44's `tests/legacy_removal_guard.rs` (std only; forbidden strings assembled with `concat` so the file never contains one whole; two positive controls over temporary trees; file-scoped rules report a missing target; skip set = dot-directories, `CHANGELOG.md`s, root `MIGRATION.md`, the two upgrade guides, itself) with rules for: (a) no `base::entity::node` path and no `Node::new`/`Node<` entity use outside the alias module if D-02's alternative is chosen; (b) the graph identifiers under *Not in this phase* still defined at their current files. The ADR names the rule: *"node" = a vertex in a WarGraph/Campaign; "Entity" = the base wrapper.* CLAUDE.md's naming table and `.github/copilot-instructions.md` get an `Entity` row.

**Mechanics the planner should expect (from CONTEXT, verbatim)**

- Bulk edit shape: `base::entity::node::Node` → `base::entity::Entity` in 117 files; `.node` → `.data` in ~106 files, **excluding** `graph.node(` / `progress.node` / `task.node` and the `node_name`/`node_id` fields (a regex on `\.node\b` followed by `.` or `;` or `)` catches the payload accesses; review each engine-file hit by hand — `cache_key.rs:240` is entity, `superstep.rs:1923` is graph).
- `api_surface_current.txt` changes on every renamed public item; refresh once at the end with `make api-surface-update`, not per commit. *(Research correction: see Pitfall 1 — the gated baseline is `.project/current-exports.txt`, and it changes by exactly 14 removed + 14 added lines, all in `ContentItemService`/`ContentItemServiceError`.)*
- `cargo semver-checks` lints expected: `struct_missing` / `struct_pub_field_missing` / `module_missing` / `trait_missing` on `paladin-core`; `function_signature_changed` on the root crate's `ContentItemService`. Each is one §9.2 row + one allowlist entry. §9.2 rows have **no row IDs** — columns are `Crate | Type / Trait | Change | Mitigation | Deliberate-breaking? | Req`, and the Crate cell is the **published** name (`paladin-ai-core` for `crates/paladin-core`). Allowlist entries are `[[entry]] crate / lint / migration_row / requirement_id / justification` (48 entries today); `make check-migration-allowlist` enforces row-level set equality. Precedent to copy: the Phase 44 rows landed by plan 44-11 (`e767fc7`) and ADR-0059 (plan 44-02).
- Docs: 9 mdBook/README files, `LayerArchitecture.svg` (text inside the SVG), `crates/paladin-core/README.md`, `src/core/mod.rs` module docs (still describes `Node<T>`), rustdoc on `paladin.rs` lines 4-11 and 140-229 and `council.rs:246`.
- Examples: 13 files under `examples/`, `doc-examples`, `benches/`; `make check-api-examples` must stay green.

### Claude's Discretion

CONTEXT.md has no explicit "Claude's Discretion" section. The surveyor left these to the planner: the module layout for `Entity` (D-01: "`base::entity::entity` or flattened — planner's call"), and the choice between the two D-02 branches pending the owner's answer to open question 2. Research recommendations for both are below (Standard Stack / Architecture Patterns / Open Questions).

### Deferred Ideas (OUT OF SCOPE)

- **Platform-wide revisioning.** The original base design wanted every entity revisionable. Today only `ContentItem` is, and `Assistant` versions itself. Deciding whether revisioning belongs on `Entity<T>` (one service, one repository trait) or stays per-aggregate is a design decision for a later milestone; this phase only documents the current contract (D-04).
- **Splitting `platform/container/`.** 70 files, of which 8 are base-derived and the rest are self-standing records. Whether to split by bounded context (engine / platform API / treasury / legacy) is out of scope here.
- **Migrating the owned downstream system.** It compiles against the deprecated alias; its own find-and-replace happens on its own schedule before v0.12.0 removes the alias.
- **Renaming `platform/container/log.rs`'s `Log = Node<LogContainer>`** to something that does not read like a log *line* — noticed, not in scope.

### Open questions CONTEXT.md leaves to the owner (RESOLVED — all answered by the owner 2026-10-13; see 48-CONTEXT.md)

1. Is `Entity<T>` / `.data` the name, or do you want a themed or different generic name? — **RESOLVED: D-01** (`Entity<T>`, payload field `data`).
2. Clean break (D-02 as recommended: no alias, ADR-0060 supersedes X-03 for this phase, §9.7 stays empty — the Phase 31-33 / 44 pattern), or a `#[deprecated]` `Node<T>` alias for one release (X-03 as written) so the downstream system can straddle the rename? — **RESOLVED: D-02** (clean break, no alias; §9.7 stays empty).
3. Remove all five dormant base items, or keep any of them for the downstream system? — **RESOLVED: D-05** (keep all five, retained pending the content-system review; overrides the removal recommendation).
4. Should the rename land in the same PR as Phase 44's legacy removal, or as its own PR after 44 merges? (D-06 assumes its own PR, after.) — **RESOLVED: D-06** (milestone branch, same PR to `main` as Phases 44-47; no separate PR).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ENTITY-01 | `paladin_core::base::entity::Entity<T>` replaces `Node<T>`; payload `node` → `data`; Paladin/Council/Grove/User/Log/ContentItem/PromptItem built on it; `#[serde(alias = "node")]` keeps pre-rename JSON loading, proven by a fixture test | Exact current struct shape (Standard Stack); rehearsed rename script and residual hand-fix list (Architecture Patterns); real pre-rename JSON captured from the unmodified tree and an alias test that passes (Code Examples); alias edge cases (duplicate/missing key) verified (Pitfall 6) |
| ENTITY-02 | Old name's fate decided and recorded: clean break + ADR-0060 (§9.7 stays empty) or `#[deprecated]` alias (§9.7 schedules removal) | Both branches' mechanics; the alias branch does NOT keep downstream compiling because a struct field cannot be aliased (Pitfall 9) |
| ENTITY-03 | Exactly one `NodeError`; `legacy_removal_guard` green; one `Paladin` alias; `NodeVersion*` → `EntityVersion*` | `citadel.rs` carries its own placeholder `PaladinData` so the alias deletion is not a pure delete (Pitfall 4); `EntityVersion*` rename rehearsed; guard stays green on the rehearsed tree |
| ENTITY-04 | `Field`/`Collection`/`Element`/`FieldVersionService`/`CollectionVersioningService` removed or kept with a reason; `version` flag contract in rustdoc | Zero outside consumers verified; real type is `CollectionVersionService` (Pitfall 5); measured coverage effect of removal (+0.4 pp core, ~+0.1 pp workspace); contract text and `create_version` enforcement point cited |
| ENTITY-05 | Graph vocabulary untouched; ADR reserves "node"; docs agree; repository-scan guard with positive controls; §9.2 rows + set-equal allowlist; API baseline refreshed + CHANGELOG; doc/examples/clean-code/coverage green | Guard design copied from `legacy_removal_guard.rs`; empirically measured semver lints (only `paladin-ai-core` fires; two NEW allow lines needed); api-surface diff is 14+14 lines; docs inventory with line numbers; gate matrix and ordering |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Extracted from `/home/user/paladin-dev-env/CLAUDE.md` and the three imported `.github` instruction files. Treat as locked.

- **TDD (Red-Green-Refactor).** Coverage floor **82 % workspace line coverage** (ADR-0006), `cargo llvm-cov --fail-under-lines`. All public APIs need doc tests.
- **Dependencies flow inward only** (core → nothing). `paladin-core` takes no new dependency (ADR-0015, D-00d).
- **Ubiquitous language:** Medieval Military terms in code/docs/comments. `Entity` is deliberately a *generic* base-layer word (D-01); the ADR and the naming table must say so rather than pretend it is themed.
- **Before committing a parent task:** `cargo test` → `cargo fmt --check` → `cargo clippy` (`-D warnings` in CI) → `make api-surface` (refresh with `make api-surface-update` + CHANGELOG entry for an intentional change). Conventional-commit message. Stop after each major task and wait for go-ahead.
- **Security:** `make security` + `cargo clippy -- -D warnings` on new/modified code; manual credential-handling review. CodeQL is advisory-only; **do not reintroduce Snyk**.
- **No `unwrap()`/`expect()`/`panic!` in library code**; prefer borrowing. (Tests may use them; new test helpers in `tests/` follow the style of `legacy_removal_guard.rs`, which returns `io::Result`.)
- **Rust conventions:** rustdoc on public items, `///` before items, ≤100-column lines where possible, `thiserror` errors, private struct fields preferred for new types (but `Entity` keeps its public fields: it is a published struct and every caller reads them).
- **Every public-surface break** needs `MIGRATION.md` §9.2 row + (when `Y`) a `.cargo/semver-checks-allowlist.toml` entry; Cargo.toml `cargo-semver-checks.lints` suppressions are registered **in the same commit** as the matching rows/entries (stated in `crates/paladin-core/Cargo.toml` header comment).
- Commit attribution lines (from the session system reminder) end every commit message and PR body.

## Summary

The rename is mechanical but wide: **157 `.rs` files change** (117 import sites, ~650 payload-access sites in 107 files, 22 struct-literal/shorthand sites, 82 `ContentItem`/`PromptItem` double-hop chains), plus ~25 prose lines in rustdoc, 13 markdown/SVG/README files and the two instruction files. It was rehearsed end to end on a scratch copy of HEAD with three perl passes and about 25 compiler-guided hand fixes. After the rehearsal `cargo check --workspace --all-targets --all-features`, `cargo test --workspace --doc`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`, `clippy -p paladin-ai-core --all-targets -D warnings`, `check-public-api-examples.sh`, `legacy_removal_guard`, and `cargo test --workspace` (6,869 passed) were all green. The only two failures (`run_api_wiring::tests::build_run_api_persists_*`) fail identically on the **unmodified** tree in this sandbox, so they are environmental, not regressions.

Six facts in the survey need correcting before planning, and they change the plan: (1) the API-surface baseline the gates read is **`.project/current-exports.txt`**, not `api_surface_current.txt` (a stale v0.2.0 file no script reads) — and it changes by exactly 14 removed + 14 added lines, all in `ContentItemService`; (2) a **`#[deprecated]` type alias cannot keep downstream compiling**, because the `.node` payload field is renamed and Rust cannot alias a struct field — the "straddle" argument for D-02's alternative is weaker than CONTEXT says; (3) `citadel.rs` defines its **own placeholder `PaladinData`/`PaladinStatus`**, so its `Paladin` alias is not a duplicate of `paladin.rs`'s type and deleting it is a small design decision, not a pure delete; (4) a file `base/entity/entity.rs` declared as `mod entity;` inside `entity/mod.rs` **fails `clippy::module_inception`** (verified), so "flatten or `entity::entity`" has a clear answer; (5) `ContentItem.node` / `PromptItem.node` are themselves entity-sense `node` fields (82 double-hop sites) that CONTEXT does not mention renaming — leaving them breaks the phase goal; (6) the type named in ENTITY-04 as `CollectionVersioningService` is actually `CollectionVersionService`.

**Primary recommendation:** Execute as 8 plans in 4 waves — ADR + fixture capture, dormant-item removal, `EntityVersion*` rename, a two-stage atomic type-then-field rename (each stage green), docs sweep, register (rows, allowlist, two new Cargo.toml allow lines, CHANGELOGs, baseline refresh), then the guard test and closeout. Take the clean break (D-02 recommended); put `Entity` in a private `wrapper` module re-exported from `base::entity`; write the `#[serde(alias = "node")]` on `Entity.data`, `ContentItem.entity` and `PromptItem.entity`.

> **Annotation (2026-10-13, plan-checker revision):** the "dormant-item removal" step of this recommendation is **overridden by 48-CONTEXT.md D-05** (all five base items retained pending review — no removal plan), and the citadel handling implied by Pitfall 4 is **overridden by CONTEXT D-10** (the placeholder `PaladinData` / `PaladinStatus` and the duplicate alias are unified with `paladin.rs`'s real types). The plan set is regenerated from CONTEXT.md per its D-15; the rest of the recommendation stands.

## Architectural Responsibility Map

This is a single-tier change inside the **domain core** (the hexagon's innermost ring) plus its mechanical fallout in outer rings. No browser/SSR/CDN/DB-schema tier is involved.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `Entity<T>` wrapper, payload field, serde alias | Core domain (`paladin-core::base::entity`) | — | Pure type; zero deps (ADR-0015); everything else depends on it |
| `EntityVersion*` revisioning types | Core domain (`paladin-core::base::service`) | Root-crate application service (`ContentItemService`) | Trait + in-memory impl live in core; the only consumer is the facade's content service |
| Persisted-JSON compatibility (Citadel snapshots) | Core domain (serde derive) | Infrastructure adapter (`FileCitadel` in `paladin-memory` reads/writes the files) | The alias is a property of the domain type; the adapter just calls `serde_json` |
| Content/user SQL repos reconstructing entities | Infrastructure (`paladin-storage`) | — | They build `Entity { .. }` from columns; only struct-literal edits, no schema change |
| Graph vocabulary freeze (NodeId, NodeSpec, …) | Engine (`paladin-battalion`, `paladin-core` waypoint/trace) | Wire formats (schema, traces, Redis keys) | Must stay byte-identical; protected by a repository-scan guard |
| API/semver registers (MIGRATION §9.2, allowlist, `.project/current-exports.txt`) | Repo governance / CI | — | Not runtime; verified by scripts |

## Standard Stack

No new runtime or dev dependency may be added (D-00d). Everything needed is already in the workspace.

### Core (already present, versions verified from `Cargo.toml` / `Cargo.lock`)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde (derive) | workspace `1` | `#[serde(alias = "node")]` on the renamed payload field | Built-in attribute; verified behaviour below [VERIFIED: scratch crate run 2026-10-13] |
| serde_json | workspace `1.0` | Fixture loading in the compat test | Already a `paladin-core` dependency |
| chrono / uuid | workspace | `created`/`modified`/`uuid` fields (unchanged) | Field order and types must not change |

### Supporting tools (CI-pinned; install locally only if the executor needs to reproduce a CI job)

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| cargo-semver-checks | 0.50.0 (CI pin) | Measure which lints the rename fires (D-27 diagnostic) | Closeout of the register plan; install: `cargo install cargo-semver-checks --locked --version 0.50.0` |
| cargo-public-api | 0.52.0 + `nightly-2026-09-20` | `make api-surface` / `make api-surface-update` | Whenever the facade surface changes (only the `EntityVersion*` plan) |
| cargo-llvm-cov | latest | Per-file line coverage (optional) | Only to re-measure the coverage effect of removals |

**Installation / reproduction of gates (all work in this environment; network goes through the session proxy):**
```bash
rustup toolchain install nightly-2026-09-20 --profile minimal      # matches Makefile PUBLIC_API_TOOLCHAIN
cargo install cargo-public-api --locked --version 0.52.0
cargo install cargo-semver-checks --locked --version 0.50.0
make api-surface            # exits 0 on the unmodified tree (verified: baseline reproduces, 4274 items)
```

**Version verification:** `cargo-semver-checks 0.50.0`, `cargo-public-api 0.52.0`, `cargo-llvm-cov` all returned `OK` from `gsd-tools query package-legitimacy check --ecosystem crates` (published 2022 / 2022 / 2021; 12.6k / 9.0k / 139k downloads per week; source repos obi1kenobi/cargo-semver-checks, cargo-public-api/cargo-public-api, taiki-e/cargo-llvm-cov). CI already pins the first two, so they are repo-sanctioned, not new.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Private `wrapper` module + `pub use` | `base/entity/entity.rs` (`mod entity;`) | **Fails `clippy::module_inception`** (warn-by-default, fatal under `-D warnings`) — verified on the rehearsal tree. Do not use. |
| Private `wrapper` module + `pub use` | Define `Entity<T>` directly in `base/entity/mod.rs` | Also valid; slightly noisier `mod.rs`. Either gives the required path `paladin_core::base::entity::Entity`. |
| `#[serde(alias = "node")]` | `#[serde(rename = "node")]` (keep the old wire key forever) | Rename would keep v0.10 readers working on v0.11 output, but contradicts D-03 and leaves "node" on the wire. Mention in ADR "Considered Options"; do not adopt. |
| Deprecated `Node<T>` alias (D-02 alternative) | Clean break | See Pitfall 9: the alias saves only the type name/path; every `.node` access still breaks. |

## Package Legitimacy Audit

This phase installs **no** project dependency (`Cargo.toml`/`Cargo.lock` stay byte-identical for dependencies; D-00d). The three developer tools below are used only to reproduce CI gates and are already pinned in `.github/workflows/ci.yml`.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| cargo-semver-checks | crates.io | ~4 yrs (2022-07) | ~12.6k/wk | github.com/obi1kenobi/cargo-semver-checks | [OK] | Approved (CI pin 0.50.0) |
| cargo-public-api | crates.io | ~4 yrs (2022-04) | ~9.0k/wk | github.com/cargo-public-api/cargo-public-api | [OK] | Approved (CI pin 0.52.0) |
| cargo-llvm-cov | crates.io | ~5 yrs (2021-01) | ~139k/wk | github.com/taiki-e/cargo-llvm-cov | [OK] | Approved (CI `coverage` job) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 persisted JSON (v0.10)            code written against v0.10
 {"uuid","created","modified",     paladin.node.name   Node::new(..)   use ..entity::node::Node
  "node":{..payload..},"name",           │                  │                    │
  "version"}                             ▼                  ▼                    ▼
        │                       ┌──────── RENAME (scripted, 2 stages) ────────────────┐
        │                       │ stage A  Node→Entity, module node→(wrapper, private) │
        │                       │ stage B  .node→.data, ContentItem.node→.entity       │
        │                       └───────────────────────┬──────────────────────────────┘
        ▼                                               ▼
 serde Deserialize (derive)                  paladin_core::base::entity::Entity<T>
   #[serde(alias="node")] data  ───────────▶   { uuid, created, modified, data, name, version }
        │  "data" also accepted                        │
        │  both keys → "duplicate field"               ├── Paladin / Council / Grove / User / Log   (type aliases)
        ▼                                              ├── ContentItem { entity }  PromptItem { entity }
 new writes always emit "data"/"entity"                └── PaladinState.paladin : Entity<citadel::PaladinData>
                                                               │
              EntityVersionService / EntityVersionRepository ◀─┘ honours Entity.version (ContentItem only)
                                    ▲
                   ContentItemService (root crate, the sole consumer)

 FROZEN (byte-identical): NodeId, NodeSpec, StateNode, NodeContext, NodeCache*, NodeInterceptor,
   NodeExecutionRecord, NodeOutcomeKind, NodeProgressKind, TraceEvent::Node*, wargraph-doc.schema.json,
   NodeCacheConfig (src/config/node_cache.rs), the single structured NodeError
        ▲ protected by tests/entity_rename_guard.rs (new) + tests/legacy_removal_guard.rs (existing)
```

### Current shape of `Node<T>` (the thing being renamed) [VERIFIED: read `crates/paladin-core/src/base/entity/node.rs`, 188 lines]

```rust
// node.rs:5-13
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Node<T> {
    pub uuid: Uuid, pub created: DateTime<Utc>, pub modified: DateTime<Utc>,
    pub node: T, pub name: Option<String>, pub version: bool,
}
// Hash is hand-written (uuid, name, node); API: new(node,name), new_with_uuid(uuid,node,name),
// update(new_data), set_name, enable_versioning, disable_versioning, is_versioning_enabled.
// Tests (node.rs:79-188): creation, update, versioning control, hash, serialization round-trip.
```
There are **no `#[serde(...)]` attributes today**, and field **declaration order is part of the wire format and of the derived `Ord`** — keep `uuid, created, modified, data, name, version` in that order.

### Recommended file layout after the phase

> **Annotation:** the "collection/field gone" and "element gone" comments in the tree below are **overridden by 48-CONTEXT.md D-05** — `entity/mod.rs` keeps `pub mod collection;` and `pub mod field;`, `component/mod.rs` keeps `element`, and `service/mod.rs` keeps `field_version_service` and `collection_versioning_service` (retained pending review). Every other line of the layout stands.

```
crates/paladin-core/src/base/
├── mod.rs                       # unchanged (component, entity, service); add module rustdoc naming the base layer's remaining contents
├── entity/
│   ├── mod.rs                   # mod wrapper; pub use wrapper::Entity; pub mod message;   (collection/field gone)
│   ├── wrapper.rs               # was node.rs (git mv); holds Entity<T> + its unit tests
│   └── message.rs               # untouched
├── component/
│   ├── mod.rs                   # pub mod action; pub mod event;      (element gone: it was a 25-line comment, no code)
│   ├── action.rs, event.rs      # untouched
└── service/
    ├── mod.rs                   # analysis_service, entity_version_service, message_service
    ├── entity_version_service.rs  # was node_version_service.rs
    ├── analysis_service.rs, message_service.rs   # untouched
crates/paladin-core/tests/
├── entity_serde_compat.rs       # NEW (ENTITY-01 proof)
└── fixtures/pre_rename/{paladin,citadel,content,prompt}.json   # NEW, captured from the unmodified tree
tests/entity_rename_guard.rs     # NEW root-level guard (ENTITY-05); auto-discovered, no [[test]] entry needed
.planning/decisions/0060-base-entity-rename.md   # NEW
```

### Pattern 1: Two-stage scripted rename (rehearsed, reproducible)

**What:** Stage A renames type and module paths (pure, no hazards). Stage B renames the payload field (all the hazards). Each stage is its own green commit.
**When to use:** Always for this phase; do not hand-edit 150 files.

```bash
# STAGE A  (run from repo root; FILES excludes planning docs and the codeql fixture)
FILES=$(git ls-files '*.rs' | grep -v '^\.planning/\|^fixtures/')
git mv crates/paladin-core/src/base/entity/node.rs crates/paladin-core/src/base/entity/wrapper.rs
#   entity/mod.rs -> "mod wrapper;\npub use wrapper::Entity;\npub mod message;\n" (add collection/field only if kept)
perl -pi -e 's/\b(paladin_core|paladin::core|crate::core|crate)::base::entity::node::Node\b/$1::base::entity::Entity/g;
             s/\bbase::entity::node::Node\b/base::entity::Entity/g' $FILES
for f in $(grep -l 'base::entity::Entity' $FILES); do
  perl -pi -e 's/(?<![:\w"])Node(?=::new\b|<| \{|::new_with_uuid)/Entity/g' "$f"   # Node::new, Node<, Node { literal
done
perl -pi -e 's/\bNode\b/Entity/g' crates/paladin-core/src/base/entity/wrapper.rs     # the definition itself
# Result on HEAD: 119 files, +366/-365. Remaining bare `Node` in entity sense = prose list in Pitfall 8.

# STAGE B  (field rename; NOTE the two guards in the regex)
perl -pi -e 'next if /\b(progress|fields)\.node\b|\bp\.node\s*==/;       # graph-sense hits, see list below
             s/(?<!\.)\.node\b(?![\w(])/.data/g' $FILES                    # (?<!\.) protects `..node`; (?![\w(]) skips graph.node( / node_id
# In wrapper.rs: declare `#[serde(alias = "node")] pub data: T,`, rename ctor params, `self.data = new_data`.
# ContentItem/PromptItem: field node -> entity (+ alias), then:
perl -0pi -e 's/\.data(\s*)\.data\b/.entity$1.data/g' $FILES                 # 82 double-hop chains incl. multi-line
perl -0pi -e 's/\b(PromptItem|ContentItem) \{(\s*)node:/$1 {$2entity:/g' $FILES # wrapper struct literals
# then compile-guided: `cargo check --workspace --all-targets --all-features --message-format short`
```

**Known graph-sense `.node` hits that must NOT change (all 7, in 3 files)** [VERIFIED: grep]:
`crates/paladin-battalion/src/engine/mod.rs:2598` (`progress.node.clone()`), `engine/superstep.rs:2300` (`progress.node.clone()`), `superstep.rs:7262` (`p.node == NodeId::new("planner")` — a closure param named `p`, **not** excluded by a receiver-name list; it surfaces as a compile error), `superstep.rs:7527` (`progress.node`), `crates/paladin-eval/src/scenario.rs:864,870,887` (`fields.node`). `graph.node(..)` calls (superstep.rs 1923/2714/3580/4568/5060, mod.rs 2583/2594, shape.rs:194, worker.rs:122, eval runner.rs:822, tests/integration/e2e_approval_gate_test.rs:237) are skipped by the `(?![\w(])` guard. `cache_key.rs:240` is entity-sense and converts correctly.

**Residual compile-guided hand fixes after the scripts (rehearsal: ~25 sites, ~5 files)** [VERIFIED: rehearsal]:
- `Entity { .., node: x, .. }` struct literals → `data: x` and `..node` struct-update bases: `node_version_service.rs:261,556,558,592,594`, `src/core/platform/manager/content_service.rs:126-131`, `src/application/services/paladin/paladin_execution_service.rs:8295`, `crates/paladin-storage/src/{sqlite,mysql}_content_repository.rs:295/795, 299/765`, `sqlite_user_repository.rs:186`, `tests/repository/mysql_content_repository_test.rs:35-44,163-172,242-246` (**feature-gated, invisible without `--all-features`**).
- `ContentItem`/`PromptItem` single-hop reads of *entity* fields (`content.entity.uuid/created/modified/name/version`, `&content_item.entity`): `sqlite_content_repository.rs:362-431`, `mysql_content_repository.rs` (same shape), `content_service.rs:59,77,146,162,178`, `tests/functional/content_lifecycle_test.rs:233-239`, `tests/repository/mysql_content_repository_test.rs:214-220`. Receiver-specific (`content.data.name` is the *entity* name, `paladin.data.name` the *payload* name) so a blind regex cannot decide; the compiler does.
- `PromptItem { node: Entity::new(..) }` literals at `paladin_execution_service.rs:2810,2999,3589`, `temperature_service.rs:144` (caught by the multi-line `PromptItem {…node:` regex above).

### Pattern 2: Fixture-driven serde compatibility test

Capture JSON from the **unmodified** tree first (one throwaway test; it printed these shapes), commit the four files, and let the compat test consume only JSON files so it compiles after the rename. Real captured shapes [VERIFIED: ran on unmodified HEAD]:

- `paladin.json` — `{"uuid":..,"created":..,"modified":..,"node":{"system_prompt":"",..,"max_loops":{"Fixed":3},..},"name":"fixture","version":true}`
- `citadel.json` — `{"paladin":{..same entity shape, "node":{citadel::PaladinData}..},"garrison":[],"execution_history":[],"created_at":..,"updated_at":..,"schema_version":"1.0.0"}`
- `content.json` — `{"node":{"uuid":..,"node":{"content":{"Text":{"path":null,"content":"hello","filesize":5}},..},"name":null,"version":true}}` (**two nested `node` keys**)
- `prompt.json` — same double-`node` nesting with `prompt_type`, `parameters`, …

### Pattern 3: Repository-scan guard (copy `tests/legacy_removal_guard.rs`)

`tests/legacy_removal_guard.rs` (≈560 lines) structure to copy verbatim: `SCANNED_EXTENSIONS = ["rs","md","toml","yaml","yml"]`; `skip_directory` (`target`, `node_modules`, every dot-dir); `skip_file` (`CHANGELOG.md`, `HISTORY_FILES` = root `MIGRATION.md` + `docs/src/api-reference/{migration-guide,upgrading}.md`, and `GUARD_FILE`); forbidden strings assembled with `[..].concat()` so the file never contains one whole; `occurs(haystack, needle, bound_start, bound_end)` identifier-boundary matcher; `scan_scoped` for file-scoped rules that **report a missing target**; `production_lines` (non-comment lines before `mod tests`); `ScratchTree` (temp dir removed on drop); three tests — `scanner_reports_planted_*` (one planted violation per rule, exact count, rule named), `scanner_accepts_a_clean_tree` (look-alikes + history files), and the real-tree test.

Recommended rules for `tests/entity_rename_guard.rs` (std only, `io::Result` tests):

| # | Rule | Scope | Planted/clean controls |
|---|------|-------|------------------------|
| 1 | Old paths absent: `base::entity::node`, `base::service::node_version_service`, `base::service::field_version_service`, `base::service::collection_versioning_service`, `base::entity::field`, `base::entity::collection`, `base::component::element` (ident-boundary end) | all scanned files outside the skip set | plant each; clean tree holds them inside `CHANGELOG.md`/`MIGRATION.md`/a dot-dir |
| 2 | Old identifiers absent: `NodeVersion` (prefix-bounded: catches `NodeVersionService`, `…Repository`, `…Metadata`, `InMemoryNodeVersionRepository`), `FieldVersionService`, `CollectionVersionService`, `CollectionVersioningService`; and `Node::new(` / `Node<` with a non-identifier char before (`StateNode::new` and `NodeId::new` must pass) | all scanned `.rs`/`.md` | clean tree includes `StateNode::new`, `NodeId::new`, `BattalionError::Node(`, `EngineError::Node` |
| 3 | No `.node` / `node:` payload access in the production lines of the 8 container files (`paladin.rs`, `citadel.rs`, `battalion/{council,grove}.rs`, `user.rs`, `log.rs`, `content.rs`, `prompt.rs`) | file-scoped, missing target reported | plant `self.node.x` and `pub node: …` |
| 4 | Exactly one `pub type Paladin =` declaration workspace-wide, in `paladin.rs`; exactly one `pub struct NodeError`, in `node_error.rs` | scan all `.rs`, count | plant a second of each; zero must also fail |
| 5 | Freeze: each graph identifier still declared at its current file — `NodeId` `waypoint.rs:295`, `NodeSpec` `engine/graph.rs:42`, `NodeContext` `engine/node.rs:52`, `StateNode` (trait in `engine/node.rs`), `NodeCacheKey`/`NodeCachePort` `paladin-ports/src/output/node_cache_port.rs:61/204`, `NodeInterceptor` `engine/hooks.rs:628`, `NodeExecutionRecord`/`NodeOutcomeKind` `waypoint.rs:563/531`, `NodeProgressKind`+`TraceEvent::NodeStarted` `trace.rs:118/220`, `NodeDoc` `engine/graph_doc.rs:270`, `NodeCacheConfig` `src/config/node_cache.rs`; `docs/schemas/wargraph-doc.schema.json` exists and contains `"nodes"` and `max_node_visits` | file-scoped (file exists AND contains the declaration needle) | clean tree has them; planted tree lacks one → reported |

"Byte-for-byte untouched" for the schema, config and trace files is best proven at verification time with `git diff --exit-code <base>..HEAD -- docs/schemas/wargraph-doc.schema.json src/config/node_cache.rs crates/paladin-core/src/platform/container/{trace,waypoint}.rs crates/paladin-battalion/src/engine/graph_doc.rs`; the guard's job is the *declaration* freeze (std-only, no hashing).

### Recommended plan decomposition (Phase 44 had 13 plans; this phase needs ~8)

> **Annotation:** this decomposition predates the owner's answers and is **superseded by 48-CONTEXT.md D-15** — no removal plan (48-02's row below is overridden by CONTEXT D-05), the citadel unification joins Stage A (overridden by CONTEXT D-10; see Pitfall 4), and the register lands once, measured, on the final tree (CONTEXT D-14). The table is kept as the research record.

| Plan | Wave | Content | Depends on | Reqs |
|------|------|---------|-----------|------|
| 48-01 | 1 | Record D-02 in `48-CONTEXT.md`; ADR-0060; `PROMOTION.md` row 0060 + "Next free ADR number: 0061"; PROJECT.md Key Decisions row; capture pre-rename fixtures (JSON only) | — | ENTITY-02 |
| 48-02 | 1 | Remove `Field`, `CollectionType`/`Item`, `Element`, `FieldVersionService`, `CollectionVersionService` (+ their mod lines, stable-api rows, tree listings); keep `ChangeType`/`HashAlgorithm`/`VersioningError` (they live in `node_version_service.rs`) | — | ENTITY-04 |
| 48-03 | 2 | `node_version_service.rs` → `entity_version_service.rs`; `NodeVersion*`→`EntityVersion*`; `VersioningError::NodeNotFound`→`EntityNotFound`; fields `node_id`→`entity_id`, `node_data`→`entity_data`; update `content_service.rs` + 2 functional tests; **refresh `.project/current-exports.txt` in this commit** (it is the only plan that moves facade lines) | 48-02 (the removed services import from this module) | ENTITY-03 |
| 48-04 | 3 | Stage A: `Node<T>`→`Entity<T>`, module move, `citadel.rs` alias removal; green with field still `node` | 48-03 (same file touched) | ENTITY-01/03 |
| 48-05 | 3 | Stage B: field rename, `ContentItem`/`PromptItem` `entity`, alias, `entity_serde_compat.rs`, `Entity` rustdoc incl. `version` contract | 48-04 | ENTITY-01/04 |
| 48-06 | 4 | Prose/docs sweep: rustdoc (list in Pitfall 8), 9 mdBook/README files, `LayerArchitecture.svg`, CLAUDE.md + `.github/copilot-instructions.md`, `crates/doc-examples/src/support.rs` | 48-05 | ENTITY-05 |
| 48-07 | 4 | Register: §9.1 M-B-12, §9.2 rows, §9.4 note, §9.7 sentence, allowlist entries, `trait_missing`/`module_missing` allow lines, CHANGELOGs, D-27 semver diagnostic | 48-05 | ENTITY-02/05 |
| 48-08 | 5 | `tests/entity_rename_guard.rs`; ADR "Code Locations / Code Conformance / Downstream Consumers" sections; full gate run; STATE/ROADMAP updates | all | ENTITY-05 |

**Commit-green constraint:** the type rename and the field rename cannot each be done gradually (a field cannot be aliased), so stages A and B are single wide commits. Splitting A from B keeps A hazard-free and B reviewable. Per the repo rule in `crates/paladin-core/Cargo.toml`'s header comment, a `[package.metadata.cargo-semver-checks.lints]` suppression, its §9.2 row and its allowlist entry land **in the same commit**; therefore give 48-02/03/04/05 their own row + entry + (if new) allow line rather than deferring all of it to 48-07, and let 48-07 be the D-27 re-measurement and prose pass. Waves 1-2 plans that touch disjoint files may run in parallel (48-01 ∥ 48-02).

### Anti-Patterns to Avoid

- **`base/entity/entity.rs` + `mod entity;`:** trips `clippy::module_inception` (verified); use a private `wrapper` module or define the type in `mod.rs`.
- **Blanket `s/\.node\b/.data/` without the `(?<!\.)` and `p.node`/`progress|fields` guards:** corrupts `..node` struct-update syntax (verified: it produced `..data`) and silently retargets graph hits.
- **Renaming `node_id`/`node_data` globally:** `node_id` has 1,151 graph-sense occurrences. Restrict the `NodeVersion` field rename to the four files that carry it (`entity_version_service.rs`, `content_service.rs`, `tests/functional/content_{lifecycle,fetching_pipeline}_test.rs`).
- **Trusting `cargo check` with default features:** `tests/repository/mysql_content_repository_test.rs` only compiles with `--all-features`. Gate on `cargo check --workspace --all-targets --all-features` (what CI's `msrv` job runs).
- **Deleting `citadel.rs:48` blindly:** see Pitfall 4.
- **Adding a `schema_version` to `Entity<T>`:** D-03 forbids it; the alias is enough.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Old-key JSON compatibility | A custom `Deserialize` impl or a pre-parse JSON rewriter | `#[serde(alias = "node")]` on the field | Verified semantics: accepts old or new key, rejects both together (`duplicate field`), rejects neither (`missing field`); no code, no risk |
| Bulk rename of 150 files | Manual edits / IDE rename (cannot see feature-gated and doc code) | The perl passes above + compiler-guided fixes | Reproducible, reviewable as a script, and the compiler enumerates what the regex cannot decide |
| Public-API diff | Hand-written "what changed" lists | `make api-surface-update`, `cargo semver-checks check-release` (D-27 diagnostic) | Repo precedent (Phase 44) measures lints, never guesses them |
| Allowlist ↔ register equality | A bespoke checker | `make check-migration-allowlist` (`scripts/check-migration-allowlist.sh`) | Already shared with CI's `semver` job |
| Repository-scan guard | A new scanning framework or regex crate | Copy `tests/legacy_removal_guard.rs` (std only) | It already has the skip set, boundary matcher, scratch tree and positive-control pattern; ADR-0015/D-00d forbid new dependencies |
| Platform-wide revisioning | Extending `Entity<T>` with a revision store | Rustdoc contract only (D-04) | Explicitly deferred |

**Key insight:** every hard part of this phase has an in-repo precedent from Phase 44 (ADR-0059, plans 44-02/44-11/44-12, the guard). The risk is not invention; it is the three places where this phase differs from 44 — a *rename* (so graph-sense look-alikes), a *serde wire alias* (a compatibility deserializer, which ADR-0059's D-00c forbids by name), and *facade re-exports of core types* (so lints fire only on `paladin-ai-core`).

## Runtime State Inventory

This is a rename/refactor phase; all five categories answered explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | **Citadel snapshots:** `FileCitadel` (`crates/paladin-memory/src/citadel/file_citadel.rs`) writes `PaladinState` JSON files whose `paladin` is an entity (`"node"` key). **In-tree SQL:** none store entity JSON — verified: SQLite/MySQL content and user repos persist columns and reconstruct `Entity { .. }` (`sqlite_content_repository.rs:295`, `mysql_content_repository.rs:299`, `sqlite_user_repository.rs:186`); waypoint/run/assistant tables hold other types (`PaladinConfigDoc`, `Battlefield`), not `Entity`. The ROADMAP/ENTITY-01 phrase "SQLite/Postgres rows" has no in-tree writer; it protects *downstream* stores. **Possible in-flight JSON:** `ContentItem` rides inside serializable orchestration types (`src/application/services/orchestration/types.rs:103-162`, `paladin-ports/src/output/content_delivery_port.rs`) [ASSUMED: whether any deployment ships those over Redis]. | **Data migration: none** (alias reads old). **Code edit:** alias on `Entity.data`, `ContentItem.entity`, `PromptItem.entity`. **Document** in §9.4/§9.8: reads are backward-compatible, *writes are not* — a v0.10 binary cannot read a v0.11-written snapshot (`missing field node`); drain Redis-borne `ContentItem` payloads or upgrade consumers before producers; downgrade needs a restore of pre-upgrade state. |
| Live service config | None. No external service (n8n/Datadog/Tailscale/…) is configured by the entity name; Redis node-cache keys and the `node_cache` settings are graph-sense and frozen. Verified by grep of `k8s/ docker/ scripts/ .github/ evals/ config*.yml` for `entity::node|Node<|NodeVersion` (only `.github/copilot-instructions.md`, a doc). | None |
| OS-registered state | None. No systemd/launchd/Task Scheduler/pm2 registration embeds the old names (same grep over `k8s/ docker/ scripts/`). | None |
| Secrets/env vars | None. No env var or secret key is named after `Node`/`node` in the entity sense (`APP_*` keys are config-section names; `node_cache` is graph). | None |
| Build artifacts / installed packages | `target/` (local, ignored); **`.project/current-exports.txt`** (tracked, the gated API baseline — 14+14 lines change); `api_surface_current.txt` (tracked, v0.2.0 snapshot read by **no** script/CI — leave alone or note as stale); `final-api.txt` (tracked, stale, unread); `lcov.info` (tracked, stale 68 % file from an old dev-machine layout — do not use as a coverage reference); published `paladin-ai-core 0.10.1` on crates.io (baseline for semver, immutable); the downstream system pins a submodule pointer (adopts the milestone at once). `Cargo.lock` unaffected (no package renames). | Regenerate `.project/current-exports.txt` via `make api-surface-update` (48-03). Do not touch the other three files. |

## Common Pitfalls

### Pitfall 1: The gated API baseline is not the file CONTEXT/ENTITY-05 name
**What goes wrong:** Plans edit or "refresh" `api_surface_current.txt`, which no script reads; `make api-surface` still fails.
**Why it happens:** CONTEXT.md and REQUIREMENTS ENTITY-05 say `api_surface_current.txt`. The Makefile (`api-surface`, `api-surface-update`, lines 397-405) and CI's `api-surface` job read **`.project/current-exports.txt`** (generated by `scripts/extract-public-api.sh` under `PUBLIC_API_TOOLCHAIN ?= nightly-2026-09-20`). `api_surface_current.txt` begins `# v0.2.0 baseline — 2026-05-30`.
**How to avoid:** Target `.project/current-exports.txt`. Measured on the rehearsal tree: the diff is exactly 14 removed + 14 added lines, **all in `ContentItemService`/`ContentItemServiceError`** (`node_version_service`→`entity_version_service`, `NodeVersion`→`EntityVersion`, `NodeVersionRepository`→`EntityVersionRepository`). The `Node`→`Entity` rename itself produces **zero** facade-surface lines (the facade re-exports `paladin_core::base` as `pub use paladin::core::base`, which `--simplified` does not expand). `make api-surface` on the unmodified tree exits 0 locally (baseline reproduces), so a green baseline can be trusted. State this in the CHANGELOG entry and fix the ENTITY-05 wording in the closeout.
**Warning signs:** `make api-surface` fails after only a type rename (it will not); or a plan lists `api_surface_current.txt` in `files_modified`.

### Pitfall 2: Blind `.node` regex hazards
**What goes wrong:** (a) `..node` struct-update becomes `..data` (compile error, easy); (b) a graph-sense `.node` is retargeted to a field named `data` that exists, compiling silently (the rehearsal's one such candidate, `p.node == NodeId::new(..)` at `superstep.rs:7262`, happened not to have a `data` field, so it failed loudly — treat that as luck); (c) `ContentItem.node.node` becomes `.data.data`.
**How to avoid:** Use the guarded regexes in Pattern 1; after Stage B, `git diff -U0 | grep '^-.*\.node'` over `crates/paladin-battalion/src/engine`, `crates/paladin-eval`, `waypoint.rs`, `trace.rs`, `run.rs`, `crates/paladin-storage/src/waypoint` must show only `paladin.node.*` lines (rehearsal: 13 lines, all entity-sense). Reviewer checks the 7-line graph list in Pattern 1 is byte-identical.
**Warning signs:** a diff line touching `TraceEvent`, `Waypoint`, `NodeProgress`, `NodeExecutionRecord` or `GraphDoc` code.

### Pitfall 3: Feature-gated and doc-only code is invisible to default `cargo check`
**What goes wrong:** `tests/repository/mysql_content_repository_test.rs` (mysql backend) compiles only with `--all-features`; doctests and `ignore`/`no_run` snippets are not compiled by `cargo check`.
**How to avoid:** Every code plan's gate is `cargo check --workspace --all-targets --all-features`, `cargo test --workspace --doc`, `make doc-check`, `make check-doc-examples`, `make check-examples`. Rehearsal result: doctests all pass (12/140/16/178/185/…), `cargo doc -D warnings --all-features` clean (the stage-A regex rewrites the doc-test `use` lines and the `[`Node<T>`](…)` intra-doc link in `src/core/mod.rs:20`).

### Pitfall 4: `citadel.rs` has its own placeholder `PaladinData`; its `Paladin` alias is not the same type

> **OVERRIDDEN by 48-CONTEXT.md D-10 (owner, 2026-10-13).** The facts below (the placeholder types, their importers, the wire superset) stand and are what D-10 relies on; the **recommendation** ("delete only the alias … do **not** unify") does not. The owner chose unification: `citadel::PaladinData`, `citadel::PaladinStatus` and the duplicate `pub type Paladin =` are removed; `PaladinState.paladin` becomes `super::paladin::Paladin`; the importers switch to `platform::container::paladin::{PaladinData, PaladinStatus}` with no re-export left in `citadel`; the v0.10 snapshot is proven to load by the fixture test's `PaladinState` case, and the v0.11-only `Reasoning` status is recorded in MIGRATION.md §9.4 beside D-03's downgrade asymmetry. Plan 48-04 Task 1 implements it.
**What goes wrong:** Treating `citadel.rs:48` as a duplicate of `paladin.rs:229` and deleting the alias, or "unifying" the two.
**Why it happens:** `citadel.rs:50-66` declares `PaladinData` ("Placeholder for PaladinData - should match Epic 1 implementation") and `PaladinStatus` (4 variants; the real one has 5, adding `Reasoning`). `pub type Paladin = Node<PaladinData>` (line 48) therefore wraps the *placeholder*. Used by `PaladinState.paladin` (line 130), `PaladinState::new` (line 156) and the tests (372, 396); imported externally as `citadel::{PaladinData, PaladinStatus, PaladinState}` by `tests/integration/citadel_integration_test.rs:9-12`, `crates/paladin-ports/src/output/citadel_port.rs:316,331`, `src/application/services/paladin/paladin_builder.rs:2141,2149`, `file_citadel.rs:349`. **No code imports `citadel::Paladin`** (grep).
**How to avoid (recommended, D-10):** delete only the alias and write `Entity<PaladinData>` at the three use sites inside `citadel.rs`. Rehearsal: compiles, tests pass, `clippy -D warnings` clean, **no wire change** (same struct shape), and the guard rule "exactly one `pub type Paladin =`" is satisfied. Do **not** unify the placeholder with `paladin::PaladinData` in this phase (extra public-API break on `PaladinState.paladin`'s type and `citadel::PaladinData`/`PaladinStatus` removal; it would be wire-compatible — the real type is a superset with `#[serde(default)]` on four fields — but it is a separate design decision). Record it under ADR "Considered Options" and the open questions. *(Overridden by CONTEXT D-10 — see the note under this pitfall's heading; "delete the alias only" now appears in ADR-0060 only as a rejected Considered Option.)*

### Pitfall 5: ENTITY-04 names a type that does not exist
**What goes wrong:** A plan greps for `CollectionVersioningService`, finds nothing, and the guard/rows/allowlist cite a phantom.
**Why it happens:** The file is `collection_versioning_service.rs` but its types are `CollectionVersionService` (the `*Service` that `check-public-api-examples.sh` enumerates), `CollectionVersioningConfig`, `CollectionVersion`, `CollectionVersionMetadata`, `CollectionVersionHistory`, `CollectionVersionComparison`, `InMemoryCollectionVersionRepository`, `CollectionVersionRepository`, `CollectionChangeType`. Likewise `field_version_service.rs` holds `FieldVersionService`, `FieldVersion*`, `FieldVersioningConfig`, `InMemoryFieldVersionRepository`, `FieldVersionRepository`, `ValidationStatus`. The §9.2 row's first backticked identifier must be a real type (`CollectionVersionService`).
**How to avoid:** Cite real symbols in rows, ADR and guard. The removal is **self-contained**: the only items these two files import from elsewhere in `base/` are `ChangeType`, `HashAlgorithm`, `VersioningError` (from `node_version_service`), so delete them **before** renaming that module. Public-API-examples gate falls 105 → 103 entry points (`FieldVersionService`, `CollectionVersionService`); the script re-derives live, no pinned count. *(The removal half of this advice is overridden by 48-CONTEXT.md D-05 — nothing is removed, so the gate keeps both entry points; the "cite real symbols" half stands, and ENTITY-04's `CollectionVersioningService` → `CollectionVersionService` wording fix is done at closeout per CONTEXT's Claude's Discretion.)*

### Pitfall 6: Alias semantics and the downgrade asymmetry
**What goes wrong:** Assuming the alias makes the change reversible, or that both keys are tolerated.
**Verified behaviour** (scratch crate, serde 1, 2026-10-13): old key only → OK; new key only → OK; **both keys → `duplicate field data`**; neither → `missing field data`. The rehearsal compat test confirmed all four fixtures load and that new writes contain `"data"`/`"entity"` and never `"node"`.
**How to avoid:** Say in §9.4 (and M-B-12 in §9.1) that v0.11 *reads* both, *writes* only the new key; a v0.10 binary cannot read v0.11 output. Do not add `rename`/`skip_serializing` tricks.

### Pitfall 7: ADR-0059's "no shim" rule forbids a compatibility deserializer
**What goes wrong:** ADR-0060, copied from ADR-0059, repeats D-00c ("no deprecated bridge, no type alias, no re-export that keeps a removed name resolvable, **no compatibility deserializer**"), then ENTITY-01 mandates exactly one.
**How to avoid:** In ADR-0060 restate D-00c as "no *API* shim"; carve out the serde alias explicitly as *data* compatibility for persisted JSON (it keeps no removed Rust name resolvable). List the alias's lifetime (recommend: kept for the whole 0.x line; removal needs its own ADR). `tests/legacy_removal_guard.rs` is unaffected (its scan strings do not include `alias`).

### Pitfall 8: Prose and non-compiled mentions the scripts do not reach
After stage A, remaining entity-sense `Node` prose to fix by hand (from a post-script grep): `paladin_builder.rs:1394`; `comment.rs:3`; `council.rs:221,452`; `grove.rs:287`; `user.rs:5,9,117,164`; `log.rs:4,396`; `workflow.rs:4`; `citadel.rs:47`; `crates/paladin-core/src/lib.rs:11`; `node_version_service.rs` header comment lines 2-24,76,92; `field.rs` comment (deleted anyway); `sqlite_content_repository.rs:294`, `mysql_content_repository.rs:298`; `tests/functional/content_lifecycle_test.rs:224,230,250`; `tests/repository/mysql_content_repository_test.rs:86,241,279`; `tests/unit/paladin_entity_test.rs:107`; `tests/cli_isolation_test.rs:122`. Leave graph prose alone (`Node ids`, `Node-cache`, `Node execution surface`, `BattalionError::Node`, `EngineError::Node`, `StateNodeError`).
Non-Rust files (line numbers at HEAD): `src/core/mod.rs:20` (+ example at 53-77 uses `Node::new`; stage A fixes the code, fix the bullet text); `docs/src/architecture/domain-model.md:41-87,196` (**the shown struct is already wrong** — it lists `id/created_at/updated_at/node` but the real fields are `uuid/created/modified/data/name/version` and the path `base/node.rs`; rewrite, don't just rename); `docs/src/architecture/design-patterns.md:6-24`; `docs/src/architecture/crate-map.md:80-83` and `overview.md:58,86-89` (trees list `collection.rs`/`field.rs` — remove with D-05; *overridden by 48-CONTEXT.md D-05: the files are retained, so these tree listings now stay correct*); `docs/src/api-reference/crate-map.md:23`; `docs/src/api-reference/stable-api.md:531,614-616` (rows for `Node<T>`, `Collection<T>`, `Field`); `docs/src/appendix/user-rest-api.md:127` ("follows the Node pattern"); `docs/src/assets/LayerArchitecture.svg:112-115` (text labels `Node`, `Collection`, `Field`); `README.md:79`; `crates/paladin-core/README.md:7,11,21` (**line 21 already references the nonexistent `base::node::Node::<String>::new`**); `.github/copilot-instructions.md:52,318-337` and `CLAUDE.md:23` (`Node<T>` in the crate table — the Medieval naming table itself is in copilot-instructions; add an `Entity` row/note there and the same sentence to CLAUDE.md); `crates/doc-examples/src/support.rs:14,81` (compiled by `check-doc-examples.sh`; stage A edits it).

### Pitfall 9: The deprecated-alias alternative (D-02) does not do what CONTEXT claims
**What goes wrong:** The owner picks the alias believing the downstream system "keeps compiling". It will not: `Node<T>` can alias `Entity<T>`, but the renamed *field* cannot be aliased, so every `paladin.node.name` (the ~650 payload-access sites; likewise every `Node { node: .. }` literal) still fails to compile, and `ContentItem.node` is gone too. Only `use …::node::Node`, `Node::new(..)` and `Node<T>` in signatures keep resolving — and each emits a deprecation warning that fails the downstream's own `-D warnings` builds.
**How to avoid / what differs if chosen anyway:** (1) say this in the owner question; (2) `base/entity/node.rs` becomes a 3-line re-export module with `#[deprecated(since = "0.11.0", note = "renamed to Entity<T>; removed in 0.12.0")] pub type Node<T> = Entity<T>;`, in-tree use must be zero (`-D warnings`); (3) `semver-checks` fires **no** `struct_missing`/`module_missing` for `Node` (item still exists) but still fires `struct_pub_field_missing` for `ContentItem.node`/`PromptItem.node`; (4) MIGRATION §9.7 becomes non-empty for the first time and must schedule removal for v0.12.0; `scripts/check-deprecations.sh` (CI `api-surface` job, line 255) requires every `#[deprecated]` to carry `since` and `note`; (5) ADR-0060 shrinks to the vocabulary rule plus the field/serde decisions (no X-03 supersession needed); (6) guard rule 1/2 gain an allowlisted single file (`base/entity/node.rs`) and the "no `Node::new`/`Node<` outside the alias module" form D-07(a) describes. Recommendation stays: clean break.

### Pitfall 10: Sandbox-only flake mistaken for a regression
`cargo test --workspace` in this environment has exactly two failures — `infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set` and `…_no_run_traces_by_default` ("the run never reached a terminal status", `run_api_wiring.rs:2369`) — and they fail identically on the unmodified HEAD (verified by running them there). Executors should compare against that baseline rather than chase them.

## Code Examples

### `Entity<T>` after the phase (target shape)

```rust
// crates/paladin-core/src/base/entity/wrapper.rs   (re-exported as paladin_core::base::entity::Entity)
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// Universal base-layer wrapper: identity, timestamps, an optional display name and a payload.
///
/// # Revisioning contract (`version`)
/// `version` declares that this entity *wants* revision history; it does not create any. The flag is
/// honoured only by a caller that routes writes through [`EntityVersionService`], whose
/// `create_version` returns [`VersioningError::VersioningDisabled`] when it is `false`. Today the only
/// such caller is `ContentItemService` (root crate), so only `ContentItem` is revisioned; `Paladin`,
/// `Council`, `Grove`, `User` and `Log` carry the flag but nothing reads it, and `Assistant` versions
/// itself with its own `AssistantVersion`. `Entity::new` sets `version: true`.
///
/// # Serialization
/// `data` was named `node` before v0.11; `#[serde(alias = "node")]` keeps every JSON document written
/// by v0.10 loadable. New writes emit `data`. A document carrying both keys is rejected.
///
/// # Examples
/// ```
/// use paladin_core::base::entity::Entity;
/// let e = Entity::new(42_u32, Some("answer".to_string()));
/// assert_eq!(e.data, 42);
/// assert!(e.is_versioning_enabled());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entity<T> {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(alias = "node")]
    pub data: T,
    pub name: Option<String>,
    pub version: bool,
}
```

### Compat test (passes on the rehearsal tree) [VERIFIED: ran]

```rust
// crates/paladin-core/tests/entity_serde_compat.rs
use paladin_core::platform::container::{
    citadel::PaladinState, content::ContentItem, paladin::Paladin, prompt::PromptItem,
};
fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/pre_rename/{name}.json", env!("CARGO_MANIFEST_DIR")))
        .expect("fixture")
}
#[test]
fn pre_rename_json_still_loads() {
    let p: Paladin = serde_json::from_str(&fixture("paladin")).expect("paladin");
    assert_eq!(p.data.model, "gpt-4");
    let st: PaladinState = serde_json::from_str(&fixture("citadel")).expect("citadel");
    assert_eq!(st.paladin.name.as_deref(), Some("fixture"));
    let c: ContentItem = serde_json::from_str(&fixture("content")).expect("content");   // double-nested "node"
    assert!(c.entity.version);
    let pr: PromptItem = serde_json::from_str(&fixture("prompt")).expect("prompt");
    assert_eq!(pr.entity.data.parameters.max_tokens, Some(1000));
    let out = serde_json::to_string(&c).unwrap();
    assert!(out.contains("\"entity\":") && !out.contains("\"node\":"), "{out}");
}
```
Add (a) a duplicate-key rejection test and (b) a generic `Entity<TestData>` round-trip using the old key, so the contract in Pitfall 6 is pinned. The fixtures were produced by this throwaway test run on the **unmodified** tree (commit them before the rename; a post-rename capture would already emit `data`):
```rust
// crates/paladin-core/tests/capture.rs (pre-rename ONLY; do not commit)
let p: Paladin = Node::new(PaladinData::default(), Some("fixture".into()));          println!("PALADIN={}", serde_json::to_string(&p)?);
let st = PaladinState::new(Node::new(CData::default(), Some("fixture".into())), vec![], vec![]);  // citadel::PaladinData
let c = ContentItem::new(ContentType::Text(TextContent::new(None, Some("hello".into()))?))?;
let pr = PromptItem::new(PromptType::Text(TextPrompt { content: "hi".into(), role: PromptRole::User }))?;
```

### Register rows (copy Phase 44's shape; Crate cell = published name)

```markdown
| `paladin-ai-core` | `Node` (the base wrapper `paladin_core::base::entity::node::Node<T>`) | struct and module removed; replaced by `paladin_core::base::entity::Entity<T>` with the payload field `node` renamed `data` (`#[serde(alias = "node")]`). Replacement: `Entity::new(data, name)`, `.data` -- landed Phase 48, D-01/D-02 | none -- clean break under ADR-0060; no API shim (the serde alias is data compatibility, not a shim). Suppressed via `struct_missing = "allow"` (existing) and `module_missing = "allow"` (new) in `crates/paladin-core/Cargo.toml`, mirrored in `.cargo/semver-checks-allowlist.toml` | Y -- one vocabulary: "node" is a WarGraph vertex only; measured with cargo-semver-checks 0.50.0 vs published 0.10.1 (`struct_missing`, `module_missing`) | ENTITY-01, ENTITY-02 |
```
```toml
[[entry]]
crate = "paladin-ai-core"
lint = "module_missing"
migration_row = "paladin-ai-core | Node"
requirement_id = "ENTITY-02"
justification = "paladin_core::base::entity::node is removed outright (ADR-0060); the replacement is paladin_core::base::entity::Entity. Every in-tree use migrated in the same commit series; suppressed by a new crate-wide allow line in crates/paladin-core/Cargo.toml."
```
The set-equality script keys on `crate | first-backticked-identifier` of the Type cell versus `migration_row` (type reduced to its first whitespace-delimited token). Several entries may share one row (one per lint).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Base wrapper `Node<T>`, field `node` | `Entity<T>`, field `data` | This phase (v0.11.0) | The only "node" left is the WarGraph vertex |
| Phase 25 "deprecate, never remove" (X-03) | Phase-scoped clean-break supersessions (ADR-0051, ADR-0059, now ADR-0060) | 2026-09 / 2026-10 | §9.7 stays empty; rows + allowlist are refactor docs, not shims |
| Semver diagnostics guessed | D-27: derive lints empirically with allows disabled, restore byte-for-byte | Phase 25/44 | Reproduced below |

**Measured semver lints for this phase** (cargo-semver-checks 0.50.0, `--baseline-version 0.10.1 --release-type minor --default-features`, `paladin-ai-core`'s crate-wide allows removed, rehearsal tree after all four code stages) [VERIFIED: ran]: Phase-48-specific failures are `struct_missing` (`Node`; `NodeVersion`, `NodeVersionMetadata`, `NodeVersionService`, `InMemoryNodeVersionRepository`, **and the unrenamed neighbours that moved with the module:** `VersionComparison`, `VersionHistory`, `VersioningConfig`; `CollectionType`, `Item`, `Field`, `FieldVersion*` (5), `CollectionVersion*` (7)), `enum_missing` (`ChangeType`, `HashAlgorithm`, `VersioningError`, `CollectionChangeType`, `ValidationStatus`), **`trait_missing`** (`NodeVersionRepository`, `FieldVersionRepository`, `CollectionVersionRepository`) — *not currently allowed*, **`module_missing`** (7 modules: `entity::{node,field,collection}`, `component::element`, `service::{node_version,field_version,collection_versioning}_service`) — *not currently allowed for this crate*, `struct_pub_field_missing` (`ContentItem.node`, `PromptItem.node`) and `constructible_struct_adds_field` (`ContentItem.entity`, `PromptItem.entity`) — both already allowed. Other lints in the same run (`enum_variant_missing`, `inherent_method_missing`, `TraceEvent`/`RunFinishStatus` items) are earlier phases' and already registered. **Action:** add `trait_missing = "allow"` and `module_missing = "allow"` to `crates/paladin-core/Cargo.toml` `[package.metadata.cargo-semver-checks.lints]` (line 90) with a Phase 48 paragraph in the header comment. Facade `paladin-ai`, `paladin-storage`, `paladin-content`, `paladin-web`, `paladin-llm` all pass clean; `paladin-ports`, `-battalion`, `-herald`, `-memory`, `-notifications` were not run (no renamed item lives in them) — run all 11 in the closeout as CI does. CI's `semver` job compares to **v0.9.0**, the Phase 44 diagnostic to **0.10.1**; run both if time allows [ASSUMED: identical result].

**Deprecated/outdated:**
- `api_surface_current.txt`, `final-api.txt`, root `lcov.info`: stale artefacts no gate reads (see Runtime State Inventory); leave untouched.
- `Node<T>` pattern text in `docs/src/architecture/*.md`: already inaccurate (wrong field names/paths); the sweep corrects, not just renames.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ContentItem` JSON may be carried over Redis (orchestration `ProcessingResult`/job types derive Serialize) in some deployment, creating a rolling-upgrade ordering hazard | Runtime State Inventory | If never queued, the §9.4/§9.8 rollout note is unnecessary but harmless |
| A2 | The CI `semver` job against v0.9.0 fires the same lint set as the 0.10.1 diagnostic | State of the Art | A lint present only against 0.9.0 would fail CI until its allow line is added; mitigated by running both baselines in the closeout |
| A3 | Rename `ContentItem.node`/`PromptItem.node` → `entity` (D-08) is what the owner wants | Open Questions | If the owner prefers keeping the field name `node` on those two structs, ~82 chain sites and the compat test shape change, and "node means one thing" is only 99 % true; must be confirmed — **confirmed by the owner: CONTEXT D-08** |
| A4 | Rename `NodeVersion.node_id`/`node_data` → `entity_id`/`entity_data` and `VersioningError::NodeNotFound` → `EntityNotFound` (D-09) | Open Questions | Leaving them keeps an entity-sense `node_id` next to the 1,151 graph-sense ones — **confirmed by the owner: CONTEXT D-09** |
| A5 | Module name `entity_version_service` and private `wrapper` module for `Entity` (D-11/D-13) | Standard Stack | Cosmetic; any name works if `base::entity::Entity` is the public path |
| A6 | Local coverage effect of removals extrapolates to the workspace figure (+~0.1 pp) because the CI figure (85.0 % over 54,326 lines, `scripts/coverage.sh` header) was not re-measured here | Validation Architecture | Floor is 82 %; even removing 1,135 lines at 100 % coverage would lower 85.0 % by <0.4 pp |
| A7 | ADR-0060 should be one record covering the X-03 supersession *and* the vocabulary rule (ADR-0059 rejected splitting supersession from design) | Architecture Patterns | If the owner wants two ADRs, numbering shifts (0060/0061) and PROMOTION.md's next-free line moves by two — **confirmed by the owner: CONTEXT D-12 (one ADR)** |

## Open Questions (RESOLVED)

All seven were answered by the owner at `/gsd-discuss-phase 48` on 2026-10-13; the binding answers are the decision IDs in `48-CONTEXT.md` cited below (CONTEXT D-15 regenerates the plans from them). The research text of each question is kept as written.

1. **D-02 branch — clean break or deprecated alias?** (owner; blocks nothing in research but must be in `48-CONTEXT.md` before planning per the ROADMAP.) — **RESOLVED: D-02** — clean break, no alias; ADR-0060 records the Phase-48-only X-03 supersession; §9.7 stays empty (rejected on Pitfall 9's measured ground).
   - What we know: the alias cannot preserve `.node` field syntax (Pitfall 9), so it saves the downstream only the type name and path, at the price of a non-empty §9.7 and deprecation warnings under their `-D warnings`.
   - Recommendation: clean break (primary path of this document); present Pitfall 9 to the owner when asking.
2. **D-08: rename `ContentItem.node` and `PromptItem.node` fields?** CONTEXT says the wrapper structs "follow" without naming the field. — **RESOLVED: D-08** — both fields become `entity`, each with `#[serde(alias = "node")]`; chains read `item.entity.data.x`.
   - Recommendation: rename to `entity` with `#[serde(alias = "node")]` (82 chain sites become `item.entity.data.x`); required for the phase goal and for guard rule 3.
3. **D-09: `NodeVersion` field/variant names.** Recommend `entity_id`, `entity_data`, `EntityNotFound`, plus `VersionComparison`/`VersionHistory`/`VersioningConfig` staying (module moves, names do not). — **RESOLVED: D-09** — as recommended, plus `VersioningConfig.max_versions_per_entity`, `entity_id` on `VersionHistory` / `VersionComparison`, and `#[serde(alias = "node_id")]` / `#[serde(alias = "node_data")]` on `EntityVersion`.
4. **D-10: `citadel.rs`** — delete only the alias (recommended) or unify the placeholder `PaladinData`/`PaladinStatus` with `paladin.rs`'s (wire-compatible but a second API break). Owner/planner call. — **RESOLVED: D-10** — **unify** (overrides this file's "delete only the alias" recommendation and Pitfall 4): the placeholder types and the duplicate alias are removed, `PaladinState.paladin` becomes the real `Paladin`, no re-export is left in `citadel`.
5. **D-12: serde alias lifetime.** Recommend "kept for the 0.x line; removal needs its own ADR", written into ADR-0060 and §9.4. — **RESOLVED: D-11** (CONTEXT numbers this decision D-11, not D-12) — the read-aliases from D-03, D-08 and D-09 stay for the whole 0.x line; removing them needs its own ADR; ADR-0060 carves them out of the "no API shim" rule as data compatibility.
6. **Docs-only wording fixes owed to correct inherited errors:** ENTITY-05 says `api_surface_current.txt` (should be `.project/current-exports.txt`); ENTITY-04 says `CollectionVersioningService` (type is `CollectionVersionService`); ENTITY-01/ROADMAP say "SQLite/Postgres rows" (no in-tree SQL stores entity JSON). Fix in the closeout, not silently. — **RESOLVED: D-15** with CONTEXT's *Claude's Discretion* list — all three are fixed at closeout (plan 48-09) with requirement IDs unchanged, and ENTITY-04's "removed" expectation is also corrected to "retained pending review" (D-05).
7. **Open question 4 from CONTEXT (same PR as Phase 44?)** — Phase 44 is sealed and on `main`'s lineage; own PR after, as D-06 assumes. Nothing in research argues otherwise. — **RESOLVED: D-06** — the opposite of this recommendation: the phase lands on the milestone branch (`claude/laughing-dirac-e0h2ax`) in the **same** PR to `main` as Phases 44-47; no separate PR and no `/gsd-pr-branch` for this phase alone.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (pinned by `rust-toolchain.toml`) | all gates | ✓ (auto-installed on first cargo call) | 1.97.1 | — |
| nightly `nightly-2026-09-20` | `make api-surface[-update]` | ✓ after `rustup toolchain install` (done in the rehearsal) | 2026-09-20 | none (the pin is the point) |
| cargo-public-api | api-surface | ✓ after install | 0.52.0 | — |
| cargo-semver-checks | D-27 diagnostic | ✓ after install (≈10 min compile) | 0.50.0 | CI `semver` job |
| cargo-llvm-cov | optional coverage measurement | ✓ after install | latest | CI `coverage` job (needs Redis + object store via `make services-up`) |
| Docker / redis-cli | `make services-up`, full coverage job | ✓ (Docker 29.8.2, redis-cli 7.0.15) | — | CI |
| python3, jq, git, gh | scripts | ✓ | 3.13 / 1.7 / 2.43 / 2.89 | — |
| mdbook (+ mermaid, linkcheck) | `docs.yml` build of the mdBook | ✗ | — | `make check-doc-examples` compiles the includes; the mdBook build runs in CI only |
| shellcheck | `make lint-shell` | ✗ | — | CI `lint` job |
| crates.io network | fetch deps / baselines | ✓ via the session proxy | — | — |

**Missing dependencies with no fallback:** none.
**Missing with fallback:** mdbook, shellcheck (CI covers). Disk: a full-workspace `target/` is ≈19 GB; the rehearsal exhausted a 30 GB scratch volume once — point `--target-dir` at a volume with ≥25 GB free or `cargo clean` between gates.

## Validation Architecture

(`workflow.nyquist_validation` is absent from `.planning/config.json`, so this section is required.)

### Test Framework

| Property | Value |
|----------|-------|
| Framework | libtest via `cargo test` (no nextest); doctests via `cargo test --doc`; shell guards in `scripts/` |
| Config file | none per-crate; root `Cargo.toml` auto-discovers `tests/*.rs` (no `[[test]]` entry needed for a new top-level guard); `crates/paladin-core` has no `tests/` yet — a new `tests/` dir is auto-discovered |
| Quick run command | `cargo test -p paladin-ai-core && cargo test -p paladin-ai --test legacy_removal_guard --test entity_rename_guard` (core lib was 708 tests before, 690 after removals — *no removals under 48-CONTEXT.md D-05, so the core lib count does not drop*; ≈20 s warm) |
| Full suite command | `cargo check --workspace --all-targets --all-features && cargo test --workspace && cargo test --workspace --doc` (6,869 passed on the rehearsal tree; expect the 2 sandbox failures in Pitfall 10) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ENTITY-01 | `Entity<T>` exists at `base::entity::Entity` with `data`; aliases + wrappers built on it | compile + unit | `cargo check --workspace --all-targets --all-features`; renamed `wrapper.rs` unit tests | ✅ (renamed tests) |
| ENTITY-01 | Pre-rename JSON (Paladin, PaladinState, ContentItem, PromptItem, generic `Entity<T>`) loads; new writes emit `data`/`entity`; both-keys rejected | integration | `cargo test -p paladin-ai-core --test entity_serde_compat` | ❌ Wave 0 (fixtures + test) |
| ENTITY-02 | Old module/type absent (clean break) | guard | `cargo test --test entity_rename_guard` rule 1/2 | ❌ new |
| ENTITY-02 | ADR-0060 exists; §9.7 still begins "Empty" | guard (file-scoped) or checklist | `.planning` is skipped by the guard, so verify in the plan: `ls .planning/decisions/0060-*.md` and `sed -n '/^## 9.7/,/^## 9.8/p' MIGRATION.md` still opens with `Empty` | ❌ (verification step) |
| ENTITY-03 | One `NodeError`, one `Paladin` alias, `legacy_removal_guard` green | guard | `cargo test --test legacy_removal_guard --test entity_rename_guard` | ✅ / ❌ new rule 4 |
| ENTITY-03 | `EntityVersion*` behave as `NodeVersion*` did | unit + functional | `cargo test -p paladin-ai-core entity_version_service`; `cargo test -p paladin-ai --test functional content_lifecycle` | ✅ (renamed) |
| ENTITY-04 | Dormant items gone *(overridden by 48-CONTEXT.md D-05: retained pending review — see 48-VALIDATION.md)*; `version` contract documented | guard + doctest | rule 1; `cargo test -p paladin-ai-core --doc entity` (the `# Examples` block); rustdoc grep in plan verification | ❌ new |
| ENTITY-05 | Graph identifiers still declared; schema/config/trace files byte-identical | guard + git | rule 5; `git diff --exit-code <phase-base>..HEAD -- <frozen files>` | ❌ new |
| ENTITY-05 | Register ↔ allowlist set-equal | script | `make check-migration-allowlist` | ✅ |
| ENTITY-05 | Public-API baseline current | script | `make api-surface` | ✅ |
| ENTITY-05 | rustdoc zero-warning, doc examples, examples gate | make | `make doc-check`, `make check-doc-examples`, `make check-api-examples`, `make check-examples` | ✅ |
| ENTITY-05 | Semver lints suppressed exactly | CI/diagnostic | `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.10.1 --release-type minor` (expect pass with the two new allow lines) | ✅ (tool) |
| ENTITY-05 | 82 % coverage floor | CI | `make services-up && make coverage` | ✅ |

### Sampling Rate
- **Per task commit:** `cargo check --workspace --all-targets --all-features` + `cargo test -p paladin-ai-core` + both guards.
- **Per wave merge:** add `cargo test --workspace`, `cargo test --workspace --doc`, `make clean-code`, `make check-gates`, `make api-surface`.
- **Phase gate:** full suite green (modulo the two baseline sandbox failures), semver diagnostic run, `make coverage` ≥ 82 %, then `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/paladin-core/tests/fixtures/pre_rename/{paladin,citadel,content,prompt}.json` — captured from the **unmodified** tree (must precede 48-04)
- [ ] `crates/paladin-core/tests/entity_serde_compat.rs` — written in 48-05 (consumes only the JSON)
- [ ] `tests/entity_rename_guard.rs` — written in 48-08 (cannot pass before the rename, so it lands last)
- [ ] Framework install: none.

**Coverage effect of the removals (question 5).** *(Moot under 48-CONTEXT.md D-05 — nothing is removed; the measurement is kept as the record behind ADR-0060's Considered Option.)* Measured with cargo-llvm-cov on `paladin-ai-core --lib` (HEAD): the five removed files hold 1,135 instrumented lines at **79.8 %** covered (906/1,135: `collection_versioning_service.rs` 739 @ 95.8 %, `field_version_service.rs` 351 @ 52.7 %, `collection.rs` 38 @ 15.8 %, `field.rs` 7 @ 100 %, `element.rs` 0), i.e. *below* the crate's 85.63 %, so removing them **raises** core coverage to 86.05 % (+0.42 pp). At workspace scale (85.01 % over 54,326 lines per `scripts/coverage.sh`) the same arithmetic gives ≈ +0.1 pp; the worst case (all removed lines 100 % covered) is −0.3 pp against a 3 pp margin. New code adds only a ~6-line `Entity` impl delta, the renamed (already-covered) version service, and tests, so the floor is not at risk. The root `lcov.info` is a stale 68 % artefact from another machine and must not be used as a reference.

## Security Domain

`security_enforcement` is not set in `.planning/config.json` (absent = enabled). The phase changes no authentication, session, access-control, cryptography or network surface. The relevant surface is deserialization of persisted JSON.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — (tenant scoping untouched) |
| V5 Input Validation | yes | serde's typed deserialization; `alias` does not widen accepted *types*, only the accepted key; both-keys is rejected; unknown extra keys behave as before (no `deny_unknown_fields` added or removed) |
| V6 Cryptography | no | — (no hashing/crypto code changes; `HashAlgorithm` moves module only) |
| V14 Configuration / supply chain | yes | no dependency added; `make security` (cargo-audit + cargo-deny) must stay green; the manual credential-handling review is unaffected (no credential-bearing code touched) |

### Known Threat Patterns for this change

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Persisted-state tampering via the alternate key (attacker supplies `node` and `data` to smuggle a second payload) | Tampering | `duplicate field` error on both keys (verified); no custom deserializer |
| Silent semantic drift from a regex retargeting graph `.node` | Tampering/Integrity | graph-line diff audit (Pitfall 2) + the freeze guard |
| Downgrade/rolling-upgrade reading unreadable state | Denial of Service (availability) | §9.4/§9.8 operator note: backup before upgrade, upgrade consumers before producers, no downgrade without restore |
| Credential leakage in new fixture JSON | Information disclosure | fixtures contain only `PaladinData::default()`/literal test strings; no keys, no URLs (reviewed in the captured output) |

## Sources

### Primary (HIGH confidence)
- Local source at HEAD `1a83bb6` (read, grepped, and rehearsed): `crates/paladin-core/src/base/**`, `platform/container/{paladin,citadel,content,prompt,user,log}.rs`, `battalion/{council,grove}.rs`, `src/core/**`, `tests/legacy_removal_guard.rs`, `MIGRATION.md` §9.1/9.2/9.4/9.7, `.cargo/semver-checks-allowlist.toml`, `crates/paladin-core/Cargo.toml`, `Makefile`, `scripts/{check-migration-allowlist,check-api-surface,extract-public-api,check-public-api-examples,coverage,check-deprecations}.sh`, `.github/workflows/ci.yml`, `.planning/decisions/{0059,PROMOTION}.md`, `.planning/{REQUIREMENTS,PROJECT,ROADMAP}.md`.
- Measured this session on scratch copies of HEAD: full rename rehearsal; `cargo check --workspace --all-targets --all-features`; `cargo test --workspace` (6,869 pass, 2 baseline-identical failures); `cargo test --workspace --doc`; `RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps`; `cargo clippy -p paladin-ai-core --all-targets -- -D warnings` (and the `module_inception` negative); `cargo semver-checks 0.50.0` on 6 crates; `cargo public-api 0.52.0` / `scripts/check-api-surface.sh` on the unmodified and renamed trees; `cargo llvm-cov` on `paladin-ai-core --lib`; serde alias behaviour in a scratch crate.
- cargo-semver-checks 0.50.0 source tarball (`https://static.crates.io/crates/cargo-semver-checks/cargo-semver-checks-0.50.0.crate`, `src/lints/*.ron`): confirms no `type_alias_missing` lint exists, so removing a type alias fires nothing.
- `gsd-tools query package-legitimacy check --ecosystem crates …` → all `OK`.

### Secondary (MEDIUM confidence)
- Phase 44 ADR-0059 text and PROMOTION.md conventions (read directly; used as the shape to copy).

### Tertiary (LOW confidence)
- None used as authority. Claims that depend on deployment behaviour (Redis-borne `ContentItem`) are tagged `[ASSUMED]` above.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; tool versions are CI pins; legitimacy seam returned OK.
- Architecture (rename mechanics, plan order): HIGH — executed end to end on a scratch tree; every gate except the 82 % coverage job and CI-only `semver` (v0.9.0 baseline) was run.
- Pitfalls: HIGH for 1-8, 10 (measured); MEDIUM for 9's "downstream cannot compile" (follows from Rust semantics and the field rename, not tested against the actual downstream system).
- Governance text (ADR/rows wording, D-08..D-13): MEDIUM — recommendations awaiting owner confirmation (Assumptions A3-A7).

**Research date:** 2026-10-13
**Valid until:** 2026-11-12 for code facts (30 days, but any merge touching `base/`, the 8 container files, `content_service.rs`, or `.project/current-exports.txt` invalidates line numbers); re-run the Pattern 1 scripts against the then-current HEAD rather than trusting counts.
