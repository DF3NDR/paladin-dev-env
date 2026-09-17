# Phase 36 Deferred Items Register

Per the orchestrator's record-only evidence request attached to plan 36-12 (mirroring the Phase
34 D-00g / Phase 35 D-26/D-27 pattern): a finding surfaced during closing-gate work that is
neither fixed as part of this plan's own scope nor silently absorbed is recorded here as a
pointer, not fixed.

## Plan 36-12 — pre-existing crate-level rustdoc lint suppressions

**Observation.** Five workspace crates carry eight crate-level `#![allow(rustdoc::...)]`
attributes, predating Phase 36, that sit *underneath* the zero-`warning:`-line bar this phase
ratifies and closes (ADR-0033 / D-00a). The bar's "zero" figure is therefore a product of both
(a) the 143 `RD-nn` link/HTML defects plans 36-01 through 36-08 fixed, and (b) these eight
suppressions, which hide a further, unmeasured set of diagnostics that were never counted or
triaged as part of the phase's audit or closure tables.

**Measurement (2026-09-17, commit `fed7b72e7d31b2e8cbca36bd66a9c53db6a920ea`, plan 36-12 Task
2).** With a clean tree, the eight attribute lines were temporarily replaced with `//`-prefixed
Rust line comments (not deleted), `cargo doc --workspace --no-deps` was re-run, and the tree was
restored via `git checkout --` immediately after capture — `git status --porcelain -- src crates`
confirmed empty afterward, and a follow-up `cargo doc --workspace --no-deps` run confirmed the
real (allows-in-place) bar is still green (0 `warning:` lines). None of the eight attributes were
removed or altered in any committed state.

With the eight allows disabled:

| Crate | `generated N warnings` (per-crate summary line) |
|---|---|
| `paladin-ports` | 119 |
| `paladin-ai` (facade) | 108 |
| `paladin-llm` | 69 |
| `paladin-storage` | 10 |
| `paladin-notifications` | 3 |
| **Total content diagnostics** | **309** |
| Total `warning:`-prefixed lines (309 content + 5 per-crate summary lines) | 314 |

The eight suppressed attributes, by file:line (pre-measurement state, i.e. what is actually
committed and in force today):

| File:line | Lint suppressed |
|---|---|
| `src/lib.rs:117` | `rustdoc::broken_intra_doc_links` |
| `src/lib.rs:118` | `rustdoc::redundant_explicit_links` |
| `src/lib.rs:119` | `rustdoc::invalid_html_tags` |
| `crates/paladin-llm/src/lib.rs:49` | `rustdoc::broken_intra_doc_links` |
| `crates/paladin-ports/src/lib.rs:51` | `rustdoc::broken_intra_doc_links` (downgrades the crate's own `#![warn(...)]` one line above it) |
| `crates/paladin-ports/src/lib.rs:52` | `rustdoc::redundant_explicit_links` |
| `crates/paladin-storage/src/lib.rs:19` | `rustdoc::broken_intra_doc_links` |
| `crates/paladin-notifications/src/lib.rs:14` | `rustdoc::broken_intra_doc_links` |

Full verbatim capture (all 314 lines, unedited): `36-evidence/36-12-closing-measurement.txt`
section "Pre-existing crate-level rustdoc allows" and the raw command transcript beneath it.

**Proposed classification.** Not an `RD-nn` row (the 143-row audit list, `34-AUDIT.md` §6, was
measured against the tree *with* these allows in force, so none of the 309 diagnostics were ever
in scope for this phase's own closure tables) and not itself a phase failure — the allows are a
pre-existing, deliberate downgrade, not something Phase 36 introduced. It is a genuine,
now-quantified gap between "the CI-enforced bar reads zero" and "the tree has zero rustdoc
diagnostics of these three classes." A future phase (a candidate for Phase 36.1, alongside its
already-scoped `# Examples`-heading and entry-point work) should decide, attribute by attribute,
whether each of the eight is (a) removed and its now-visible diagnostics fixed the same way plans
36-01 through 36-08 fixed the 143 `RD-nn` rows, or (b) kept and given a written justification
recorded in ADR-0033 explaining why that specific lint class is structurally unfixable for that
specific crate (e.g. `paladin-ports`' own comment already states its reason: cross-crate doc links
that resolve in the facade crate but not in the isolated port crate).

**Owner.** Unassigned — out of this plan's own scope (D-28: `make api-surface` and the bar
commands are the gate; removing or fixing these suppressions is a `.rs` source-behavior change
this plan's `<files_modified>` list does not include, and no library source under `src/` or
`crates/` was modified by this plan). Recommended owner: Phase 36.1, or a standalone
rustdoc-currency quick task, whichever lands first.

---

*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Register opened: 2026-09-17, plan 36-12*
