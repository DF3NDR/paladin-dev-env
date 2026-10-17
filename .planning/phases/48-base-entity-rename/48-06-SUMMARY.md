---
phase: 48-base-entity-rename
plan: 06
subsystem: docs
tags: [docs, entity, vocabulary-rule, d-05, d-07, adr-0060, mdbook, naming-table]

requires:
  - phase: 48-04
    provides: "Entity<T> at paladin_core::base::entity::Entity in the private wrapper module"
  - phase: 48-05
    provides: "Final field set and order (uuid, created, modified, data, name, version) and the v0.10 read-alias"
provides:
  - "mdBook domain-model, design-patterns, crate-map, overview and api-reference pages describe Entity<T> with its real fields and the node / Entity vocabulary rule"
  - "An Entity row in the .github/copilot-instructions.md naming table (after Quest) and Entity<T> plus the vocabulary sentence in CLAUDE.md"
  - "The retained base items documented under their real names (CollectionType<T>, Item<T>, Field<T>) and marked retained pending review"
affects: [48-08, 48-09]

tech-stack:
  added: []
  patterns:
    - "Documents are written from the shipped tree (wrapper.rs, collection.rs, field.rs), not from older documents (D-00a)"

key-files:
  created: []
  modified:
    - docs/src/architecture/domain-model.md
    - docs/src/architecture/design-patterns.md
    - docs/src/architecture/crate-map.md
    - docs/src/architecture/overview.md
    - docs/src/api-reference/crate-map.md
    - docs/src/api-reference/stable-api.md
    - docs/src/appendix/user-rest-api.md
    - docs/src/assets/LayerArchitecture.svg
    - CLAUDE.md
    - .github/copilot-instructions.md
    - README.md
    - crates/paladin-core/README.md

key-decisions:
  - "The CLAUDE.md edit is three added lines and one changed line (crate-table cell plus a two-line Ubiquitous-language extension), inside the plan's four-line cap"
  - "stable-api.md listed no version-family rows, so none were added; the version service is documented in domain-model.md's Base Primitives table"
  - "The paladin-core README usage snippet uses PaladinStatus::Idle because the TaskStatus type the old snippet named exists nowhere in the workspace"

patterns-established: []

requirements-completed: [ENTITY-05, ENTITY-04]

coverage:
  - id: D1
    description: "ENTITY-05 / D-07: domain-model, design-patterns and crate-map describe Entity<T> with the six real fields in declaration order and state the node / Entity vocabulary rule"
    requirement: "ENTITY-05"
    verification:
      - kind: other
        ref: "acceptance greps: '## Entity<T> Pattern' 1, 'means a vertex in a WarGraph or Campaign graph' 1, all six field names inside the section, 'paladin.data.name' 1; make check-doc-examples exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-07 naming tables: Entity row after Quest in copilot-instructions.md; Entity<T> and the vocabulary sentence in CLAUDE.md; Treasurer rows untouched"
    requirement: "ENTITY-05"
    verification:
      - kind: other
        ref: "grep '| **Entity** |' exit 0; Quest/Entity adjacency 1; Treasurer/Commissary/Cadence diff lines 0; CLAUDE.md git diff --numstat 3 added 1 deleted"
        status: pass
    human_judgment: false
  - id: D3
    description: "ENTITY-04 / D-05: retained base items stay documented and marked retained pending review; collection.rs / field.rs remain in the trees; SVG keeps Collection, Field and the three service labels"
    requirement: "ENTITY-04"
    verification:
      - kind: other
        ref: "'retained pending review' in stable-api.md 2; collection.rs|field.rs count 2 (crate-map) and 2 (overview); SVG '>Entity<' 1 and four retained labels 4"
        status: pass
    human_judgment: false
  - id: D4
    description: "No edited file names the old wrapper path, generic form or version-family names; graph-sense text and docs/schemas untouched; SVG is well-formed XML"
    requirement: "ENTITY-05"
    verification:
      - kind: other
        ref: "forbidden-pattern grep over the mdBook pages and the four Task 2 files prints nothing; git diff -- docs/schemas empty; minidom parse of LayerArchitecture.svg ok; make doc-check exit 0"
        status: pass
    human_judgment: false

duration: about 25m
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 06: Entity<T> documentation and the node vocabulary rule Summary

**The mdBook pages, layer SVG, both READMEs, CLAUDE.md and the Copilot naming table now describe `Entity<T>` as the code ships it (`uuid, created, modified, data, name, version`), state that "node" means only a WarGraph/Campaign vertex (ADR-0060), and keep the five retained base items documented as retained pending review.**

## Accomplishments

- `domain-model.md`: the wrapper section is rewritten from `wrapper.rs` (the old section showed `id` / `created_at` / `updated_at` / `node`, which never matched the code), with the vocabulary rule, the revisioning contract (only `ContentItem`, through `EntityVersionService`) and the pre-v0.11 `node` key that still loads. The Base Primitives table names `CollectionType<T>` / `Item<T>` and `Field<T>` and adds an `EntityVersionService` row.
- `design-patterns.md`, `architecture/crate-map.md`, `architecture/overview.md`, `api-reference/crate-map.md`, `appendix/user-rest-api.md`: `Entity<T>`, `paladin.data.<field>` access, and `entity/wrapper.rs` in the trees; `collection.rs` and `field.rs` lines kept.
- `stable-api.md`: the wrapper row points at `paladin_core::base::entity::Entity` and `struct.Entity.html`; the collection row now names `CollectionType` (the nonexistent `Collection` path is gone).
- `LayerArchitecture.svg`: only the wrapper label changed to `Entity`.
- `.github/copilot-instructions.md`: `Entity` row directly after `Quest` with location `crates/paladin-core/src/base/entity/wrapper.rs`, the core tree, and the domain-entity pattern section. `CLAUDE.md`: `Entity<T>` in the `paladin-core` row and one Ubiquitous-language sentence saying `Entity` is deliberately generic, not Medieval Military (D-01).
- `crates/paladin-core/README.md`: the usage snippet names only real paths.

## Task Commits

1. **Task 1: mdBook pages and the layer SVG** - `4cb5a3f` (docs)
2. **Task 2: naming tables and READMEs** - `d080d36` (docs)

**Plan metadata:** the commit carrying this SUMMARY, STATE.md and ROADMAP.md (docs: complete plan)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] LayerArchitecture.svg was already truncated at HEAD**
- **Found during:** Task 1 verification
- **Issue:** the plan's verify requires the SVG to parse as XML, but `docs/src/assets/LayerArchitecture.svg` at HEAD ended without the root `</svg>` (minidom: "no element found: line 127"), a defect from its original commit, independent of this plan's edit.
- **Fix:** appended the missing `</svg>` line; no attribute or coordinate changed, and the only other diff is the `Node` to `Entity` label.
- **Files modified:** docs/src/assets/LayerArchitecture.svg
- **Commit:** 4cb5a3f

**2. [Rule 1 - Bug] README usage snippet named a type that does not exist**
- **Found during:** Task 2
- **Issue:** `platform::task::TaskStatus::Pending` is defined nowhere in the workspace (`grep -rn TaskStatus --include=*.rs` found nothing), and the plan's read_first asked for the real path.
- **Fix:** the snippet uses `paladin_core::platform::container::paladin::PaladinStatus::Idle`, which exists and is public.
- **Files modified:** crates/paladin-core/README.md
- **Commit:** d080d36

**3. [Rule 2 - Consistency] Remaining entity-sense `Node` words in user-rest-api.md**
- **Found during:** Task 1
- **Issue:** the plan named only the line near 127, but four other prose lines in the same appendix say "built on Node" / "Node infrastructure for versioning" in the entity sense.
- **Fix:** the four lines say `Entity`. This was prose only and none of them matches the guard's forbidden forms; the point was to leave the page self-consistent.
- **Files modified:** docs/src/appendix/user-rest-api.md
- **Commit:** 4cb5a3f

## Notes

- `stable-api.md` listed no version-family rows, so none were added.
- The `overview.md` layer-box line keeps its existing right-border column (67 characters, matching the box's top and bottom borders); the lines around it were already one or two columns wider before this plan.
- The paladin-core README snippet is not compiled by `make check-doc-examples` (the crate README is not included in rustdoc); its paths and the `Entity::new(data, name)` signature were checked against the source by hand.
- No Rust source, MIGRATION.md, allowlist or CHANGELOG file was touched (48-07's files).

## Known Stubs

None.

## Threat Flags

None. T-48-16 and T-48-17 are mitigated: the CLAUDE.md diff is within the cap, the Treasurer vocabulary rows are untouched, and every rewritten page was written from the source files.

## Self-Check: PASSED

- All twelve files listed in `key-files.modified` exist and are the complete set in `git diff --stat HEAD~2`.
- Commits `4cb5a3f` and `d080d36` exist.
- `make check-doc-examples` and `make doc-check` exit 0 (with `CARGO_INCREMENTAL=0`).
