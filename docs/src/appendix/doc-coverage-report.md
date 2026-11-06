# Documentation Coverage Report

> **Regenerated 2026-11 (Phase 55, CIREL-08).** This page records a measurement, not a coverage
> estimate: the two rustdoc bars ADR-0033 ratifies, run at commit `14ee125e7`, with every
> crate-level rustdoc suppression removed. The bar and the decision that removed the suppressions
> are in ADR-0033 (`.planning/decisions/0033-cargo-doc-warning-bar.md`, amendment of 2026-11-01).

Date: 2026-11-01
Commit: `14ee125e7696c2258adbd9645fa3eb0e1779e95c`

## Bars

ADR-0033 sets one bar for the workspace's rustdoc, enforced in two forms by `make doc-check`.
Both ran at the commit above and both read zero warnings. All thirteen workspace members were
documented (a `Documenting` line for each: `paladin-ai`, `paladin-ai-core`, `paladin-battalion`,
`paladin-content`, `paladin-doc-examples`, `paladin-eval`, `paladin-herald`, `paladin-llm`,
`paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`).

1. Default features, zero `warning:` lines:

   ```bash
   cargo doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt
   ```

   Result: exit 0, no `warning:` line in the output.

2. All features, every rustdoc warning promoted to an error:

   ```bash
   RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
   ```

   Result: exit 0.

`make doc-check` runs both and then `cargo test --workspace --doc`; it exited 0 at this commit.
Per-crate forms of the facade (`cargo doc -p paladin-ai`, with default features, with
`--all-features` and with `--no-default-features`) also read zero warnings.

## Crate roots enforcing missing_docs

Twelve crate roots set `#![warn(missing_docs)]`:

- `src/lib.rs` (the `paladin-ai` facade)
- `crates/paladin-battalion/src/lib.rs`
- `crates/paladin-content/src/lib.rs`
- `crates/paladin-core/src/lib.rs`
- `crates/paladin-eval/src/lib.rs`
- `crates/paladin-herald/src/lib.rs`
- `crates/paladin-llm/src/lib.rs`
- `crates/paladin-memory/src/lib.rs`
- `crates/paladin-notifications/src/lib.rs`
- `crates/paladin-ports/src/lib.rs`
- `crates/paladin-storage/src/lib.rs`
- `crates/paladin-web/src/lib.rs`

`paladin-ports` additionally sets `#![warn(rustdoc::broken_intra_doc_links)]`, the only rustdoc
lint attribute left in the workspace. No `allow(rustdoc` or `expect(rustdoc` remains in
`crates/*/src` or `src`.

## Suppressions removed

Eight crate-level `#![allow(rustdoc::...)]` attributes sat beneath the bar until Phase 55. Each was
removed and the diagnostics it hid were fixed in the doc comment that wrote the link. Counts are
diagnostics per crate under the default and all-features runs with that crate's attributes lifted,
measured at the start of the plan that fixed it.

| # | Crate | File:line (as removed) | Lint | Diagnostics hidden (default / all-features) |
|---|-------|------------------------|------|---------------------------------------------|
| 1 | `paladin-ports` | `crates/paladin-ports/src/lib.rs:51` | `rustdoc::broken_intra_doc_links` | 187 / 187 (170 unresolved link, with row 2's 17 redundant explicit link target) |
| 2 | `paladin-ports` | `crates/paladin-ports/src/lib.rs:52` | `rustdoc::redundant_explicit_links` | (counted in row 1) |
| 3 | `paladin-llm` | `crates/paladin-llm/src/lib.rs:49` | `rustdoc::broken_intra_doc_links` | 117 / 105 |
| 4 | `paladin-storage` | `crates/paladin-storage/src/lib.rs:20` | `rustdoc::broken_intra_doc_links` | 27 / 28 |
| 5 | `paladin-notifications` | `crates/paladin-notifications/src/lib.rs:14` | `rustdoc::broken_intra_doc_links` | 3 / 0 |
| 6 | `paladin-ai` | `src/lib.rs:117` | `rustdoc::broken_intra_doc_links` | 140 / 135 (127 + 13 default, 122 + 13 all-features) |
| 7 | `paladin-ai` | `src/lib.rs:118` | `rustdoc::redundant_explicit_links` | (13, counted in row 6) |
| 8 | `paladin-ai` | `src/lib.rs:119` | `rustdoc::invalid_html_tags` | 0 |

The workspace total before was 474 diagnostics under the default bar and 455 under the all-features
bar; after, 0 and 0. The other crates (`paladin-ai-core`, `paladin-battalion`, `paladin-content`,
`paladin-eval`, `paladin-herald`, `paladin-memory`, `paladin-web` and `paladin-doc-examples`)
reported none and needed no edit.

## History

An earlier snapshot of this page, dated 2026-05-28 (Milestone 7, Epic 4, Task 3.0), listed per-crate
coverage percentages that no tool measured. This regeneration retired that snapshot and its figures;
git history is the record of it. No coverage tool is used here: the page records what the two bars
and the `missing_docs` roots enforce, not a per-item coverage percentage.
