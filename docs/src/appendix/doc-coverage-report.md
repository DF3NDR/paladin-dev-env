# Documentation Coverage Report

> **Archived — historical document.** This page records the workspace's `cargo doc` coverage and
> warning status as of 2026-05-28 (Milestone 7, Epic 4, Task 3.0) and is not maintained. The
> zero-warning `cargo doc --workspace --no-deps` bar it describes is ratified in ADR-0033
> (`.planning/decisions/0033-cargo-doc-warning-bar.md`); the current measurement against that bar
> is tracked by Phase 36 (Rustdoc Zero-Warning Bar & Examples Currency), which regenerates this
> report once the bar is green rather than this page being regenerated now.

Date: 2026-05-28
Milestone: 7
Epic: 4, Task 3.0

## Methodology

Coverage status is based on two checks:

1. Crate-root documentation enforcement using `#![warn(missing_docs)]` in public crate `lib.rs` roots.
2. Workspace documentation build using:

```bash
cargo doc --workspace --no-deps
```

Result recorded at the time: docs build succeeded with no warnings. That result is historical —
the Phase 34 audit measured 73 `warning:` lines against this same command, and the current figure
is tracked by Phase 36 (see the banner above), not by this page.

## Crate Coverage Summary

This is the snapshot's own nine-crate inventory, predating `paladin-eval`, `paladin-herald` and
the `paladin-ai` facade.

- paladin: >= 90% (stable surface documented, rustdoc warnings clean)
- paladin-core: >= 90% (crate-root docs enforced, warnings clean)
- paladin-ports: >= 90% (crate-root docs enforced, warnings clean)
- paladin-battalion: >= 90% (crate-root docs enforced, warnings clean)
- paladin-llm: >= 90% (crate-root docs enforced, warnings clean)
- paladin-memory: >= 90% (crate-root docs enforced, warnings clean)
- paladin-web: >= 90% (crate-root docs enforced, warnings clean)
- paladin-notifications: >= 90% (crate-root docs enforced, warnings clean)
- paladin-content: >= 90% (crate-root docs enforced, warnings clean)
- paladin-storage: >= 90% (crate-root docs enforced, warnings clean)

## Notes

- Stable API expectations are tracked in `STABLE_API.md` with per-crate stability tiers.
- This report is intended for release readiness tracking in Milestone 7 Epic 4.
