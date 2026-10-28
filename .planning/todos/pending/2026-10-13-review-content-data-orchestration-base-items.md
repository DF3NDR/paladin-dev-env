---
created: 2026-10-13T16:30:00Z
title: Review the unfinished content / general data orchestration system before removing any base-layer item
area: core
severity: minor
files:
  - crates/paladin-core/src/base/entity/field.rs
  - crates/paladin-core/src/base/entity/collection.rs
  - crates/paladin-core/src/base/component/element.rs
  - crates/paladin-core/src/base/service/field_version_service.rs
  - crates/paladin-core/src/base/service/collection_versioning_service.rs
  - crates/paladin-core/src/base/mod.rs
  - crates/paladin-core/src/platform/container/content.rs
  - crates/paladin-core/src/platform/container/prompt.rs
  - src/core/platform/manager/content_service.rs
owner: repo maintainer
source: Phase 48 discuss session, Base-layer disposition area (owner answer 2026-10-13)
recheck_by: before the milestone after v0.11.0 is roadmapped
decision_status: owner decided these items stay in Phase 48; the review itself is unscheduled
resolves_phase: 55
---

## Problem

Five base-layer items have zero consumers outside `crates/paladin-core/src/base/`:
`Field` (`base/entity/field.rs`, 29 lines), `Collection`/`CollectionType`/`Item`
(`base/entity/collection.rs`, 77 lines), the comment-only `element` module
(`base/component/element.rs`, 25 lines), `FieldVersionService`
(`base/service/field_version_service.rs`, 600 lines) and `CollectionVersioningService`
(`base/service/collection_versioning_service.rs`, 1,115 lines). Phase 48's survey
(`48-CONTEXT.md`, D-05) recommended removing them as dormant.

The owner's answer (2026-10-13): **they are not dormant by design — they are part of the
`content` / `general data orchestration` system that was started with the original base layer
and never finished.** Removing them on a consumer count alone would delete scaffolding that
system still expects. Phase 48 therefore keeps all five (rustdoc on `base/mod.rs` names the
system and points here) and renames nothing in them beyond what the `Node<T>` → `Entity<T>`
sweep touches.

## What the review has to establish

- What the content / general data orchestration system was meant to do, from
  `.project/Milestone_1-MVP` onward (the `ContentItem` → `NodeVersionService` →
  `ContentItemService` path in the root crate is the only wired slice today).
- Which of the five items that design still needs, which are superseded by newer platform types
  (`Battlefield`'s `FieldName`/`FieldSpec` overlaps `Field`; `Assistant` versions itself
  independently of `*VersionService`), and which can go.
- Whether platform-wide revisioning (the `Entity<T>::version` flag's unfulfilled promise — see
  `48-CONTEXT.md` D-04 and *Deferred Ideas*) belongs to the same design.

## Options

- **A. Finish the system** in a later milestone and keep all five as its foundation.
- **B. Retire it**: a Phase-44-style clean break for whichever items the review finds unneeded,
  with §9.2 rows, allowlist entries and a repository-scan guard rule.
- **C. Partial**: keep `Collection`/`Item` and the two services, drop `Field` and `element` once
  `Battlefield`'s field types are confirmed as their replacement.

## Operator position (2026-10-13)

Keep everything until the review has been done; do not let a consumer count decide it.

## Residual entity-sense identifiers (Phase 48 planning note)

*Dated 2026-10-13 (plan 48-02, ADR-0060).* Phase 48 renames the base wrapper to `Entity<T>` and reserves
"node" for the graph sense (ADR-0060), but D-05 keeps the five items unchanged beyond the rename sweep, so
these entity-sense identifiers survive inside them and are this review's to decide:

- `Field.nid` (`crates/paladin-core/src/base/entity/field.rs`)
- `FieldVersion.node_id`, `FieldVersionHistory.node_id` and `FieldVersionComparison.node_id`
  (`crates/paladin-core/src/base/service/field_version_service.rs`)
- `FieldVersionRepository::get_node_field_versions` and `FieldVersionService::get_node_field_versions`
  (`crates/paladin-core/src/base/service/field_version_service.rs`)

ADR-0060 lists the same set as the one bounded exception to its vocabulary rule, and
`tests/entity_rename_guard.rs` deliberately does not ban these names. Renaming any of them is a public break on
`paladin-ai-core` with its own `MIGRATION.md` 9.2 rows, so it belongs to the review's outcome (options A to C
above), not to Phase 48.

Also note that the two retained services now import `ChangeType`, `HashAlgorithm` and `VersioningError` from
`entity_version_service` (plan 48-03), so the review should read that dependency when it weighs the options.
