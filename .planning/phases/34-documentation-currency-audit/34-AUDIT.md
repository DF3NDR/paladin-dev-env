# Phase 34 Documentation Currency Audit — The Single Canonical Inventory (D-01)

**The single D-01 record.** One artifact, seven sections in the order below, appended to by
plans 34-02 through 34-09 as each compiles its assigned section or sweeps its assigned files.
No other file in this phase carries a currency verdict, a rustdoc finding row, an examples row,
or a Phase 35/36 work-list item — `34-EVIDENCE.md` holds only the verbatim command captures this
file's rows cite by evidence anchor (D-02), and `deferred-items.md` holds only the non-doc,
non-example findings this file's §7 points into (D-19).

**Which plan appends which section:**
- §1 Shipped-surface checklist — plan 34-02
- §2 mdBook verdict table (rows) — plans 34-03, 34-04, 34-05
- §3 Rustdoc findings table (rows) — plans 34-06, 34-07
- §4 Examples table (rows) — plan 34-08
- §5, §6, §7 (work lists, deferred routing) — plan 34-09

## Method (read before adding or trusting a row)

A verdict is settled by **content, never by file existence or modification time** (D-00b,
Phase 16 `16-DOCS-01-VERDICTS.md` Method, restated here verbatim in spirit). "File exists" and
"file was touched recently" prove nothing about whether a page still describes the shipped tree.
A row's Verdict is `current` only if every applicable signal class was actually run against that
page and found to match; `stale` or `missing` only after the same run found a mismatch, and only
when the row's Findings cell names the phase and shipped item the page fails to describe. A
**settled verdict with an empty findings cell is invalid** — `34-check.sh` assertion (c) makes
this mechanical. A seeded row's Verdict is `pending` and its Findings cell reads exactly the
literal `pending — not yet swept (no signal class run)` — this placeholder is the
`16-DOCS-01-VERDICTS.md` concurrency rule made mechanical: an unswept page must never be
indistinguishable from a checked one.

**Evidence — nine signal classes** (D-07): the Phase 16 eight (version strings, dependency pins,
crate names, module/source paths, `make` targets, workflow/job names, error types, feature
flags), each with its own producing command, run per page by `34-signals.sh <path>`; plus a
ninth, the shipped-surface checklist hit (D-08), which degrades to an explicit `SKIPPED` line
until plan 34-02 writes `34-shipped-tokens.txt`. Every Findings cell below names the command
actually run, never a copy of this list.

**ID scheme (D-03):** every work-list item carries a stable ID — `MB-nn` (mdBook → Phase 35),
`RD-nn` (rustdoc → Phase 36), `EX-nn` (examples → Phase 36) — numbered in table order and never
renumbered, because Phases 35 and 36 close items by ID (their SUMMARY/VERIFICATION cite
"MB-07 closed by commit X"). The first mdBook row worked in §2 below fixes the numbering origin.

**Sizing rubric (D-04, verbatim):**
- **S** — a one-line or one-link fix on an existing page, a single rustdoc link repair, an
  example that needs one rename.
- **M** — a section rewrite or a new section on an existing page, a rustdoc block on an
  undocumented item family, an example needing a signature-level update.
- **L** — a new page, a new example, or a rewrite of more than half of an existing page or
  example.

## Measurement Header

**HEAD SHA measured:** `ee1fb160f8e743e638b32beb6c4e32be4ede9325`
**Date:** 2026-09-17
**Branch:** `feature/phase-33`

**D-23 invariance argument:** every Phase 34 commit touches only `.planning/` (SC5, proven per
commit by the git diff in `34-check.sh` assertion (d) — see the deviation note there for why the
base reference is the Phase 34 start SHA above, not `main`). Because the source tree every row in
this file measures is therefore identical at every Phase 34 commit, a later plan's
`git rev-parse HEAD` differing from the SHA above does not invalidate any row measured at this
SHA. If the branch moves for any other reason (a rebase, a maintainer commit on the same branch),
Phases 35/36 re-run the D-12/D-16 commands and diff against the rows recorded here rather than
trusting the counts unchanged.

**Toolchain versions (verbatim), each annotated with the pin it is compared against:**

```
$ cargo --version
cargo 1.97.1 (c980f4866 2026-06-30)
$ rustc --version
rustc 1.97.1 (8bab26f4f 2026-07-14)
```
Compared against: `rust-toolchain.toml` `[toolchain] channel = "1.97.1"` — **matches exactly**.

```
$ mdbook --version
mdbook v0.4.40
$ mdbook-linkcheck --version
mdbook-linkcheck 0.7.7
$ mdbook-mermaid --version
mdbook-mermaid 0.13.0
```
Compared against: `.github/workflows/docs.yml` — `cargo install mdbook --version 0.4.40 --locked`
(line 46), `cargo install mdbook-mermaid --version 0.13.0 --locked` (line 50),
`cargo install mdbook-linkcheck --version 0.7.7 --locked` (line 54) — **all three match exactly**.

**Toolchain-drift question (D-12) — closed, not open:** `rust-toolchain.toml`'s own header states
it "overrides whatever toolchain a workflow action installed," and the `lint` job's
`dtolnay/rust-toolchain@stable` step is exactly such an override target — both this devcontainer
and CI's lint job run `cargo`/`rustc` 1.97.1. Recorded here as the RESEARCH.md finding (Finding
F-12), not left open.

**Ubiquitous-language list identification (D-10):** the "three ubiquitous-language lists" Phase 35
SC4 names are taken to be the naming table in `.github/copilot-instructions.md`, the term table in
`.planning/PROJECT.md`, and `docs/src/architecture/domain-model.md` — confirmed by direct content
match (each contains a `Commissary` row/entry) per 34-RESEARCH.md Assumption A1.

## §1 Shipped-surface checklist

Empty. Compiled by plan 34-02 from, in D-08 precedence order: (1)
`git diff v0.9.0..HEAD -- .project/current-exports.txt`; (2) `CHANGELOG.md [0.10.0]`; (3)
`MIGRATION.md` §9.1-§9.8; (4) `.planning/REQUIREMENTS.md`'s v0.10.0 capability list. Every §2/§3/§4
verdict below cites into this checklist once it exists.

## §2 mdBook verdict table

Scope: every `.md` under `docs/src/` — 93 files, live-counted via `find docs/src -name '*.md'`,
including `appendix/` and any file not linked from `SUMMARY.md` (D-05). 92 rows below are seeded
`pending` (unswept — plans 34-03/34-04/34-05 sweep them); one row
(`docs/src/appendix/doc-coverage-report.md`) is fully worked here to prove the row schema
end-to-end (D-06/D-07), per this task's method self-test.

| # | Page | Verdict | Findings (signal class → cmd → result) | Cites (Phase N — item (REQ)) | MB ID(s) | Size |
|---|------|---------|------------------------------------------|-------------------------------|----------|------|
| 1 | docs/src/SUMMARY.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 2 | docs/src/api-reference/crate-map.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 3 | docs/src/api-reference/feature-flags.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 4 | docs/src/api-reference/migration-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 5 | docs/src/api-reference/platform-api.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 6 | docs/src/api-reference/stable-api.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 7 | docs/src/api-reference/upgrading.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 8 | docs/src/api-reference/wargraph-doc-schema.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 9 | docs/src/appendix/battalion-benchmarks.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 10 | docs/src/appendix/battalion-patterns-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 11 | docs/src/appendix/battalion-vision-support.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 12 | docs/src/appendix/branch-protection.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 13 | docs/src/appendix/build-baselines.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 14 | docs/src/appendix/cli-configuration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 15 | docs/src/appendix/cli-council.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 16 | docs/src/appendix/cli-muster.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 17 | docs/src/appendix/cli-onboarding.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 18 | docs/src/appendix/cli-setup-check.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 19 | docs/src/appendix/cli-testing.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 20 | docs/src/appendix/cli-usage.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 21 | docs/src/appendix/conclave-pattern.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 22 | docs/src/appendix/contributing-legacy.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 23 | docs/src/appendix/council.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 24 | docs/src/appendix/design-and-architecture.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 25 | docs/src/appendix/doc-coverage-report.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → none; class 3 (crate names): `grep -noE 'paladin-[a-z-]+' \| sort -u` → 9 hits (`paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`, `paladin-notifications`, `paladin-content`, `paladin-storage` — an incomplete, 2026-05-28-era list missing `paladin-eval`/`paladin-herald`/the `paladin-ai` facade); direct measurement: `cargo doc --workspace --no-deps 2>&1 \| tee 34-evidence/34-01-cargo-doc-default.txt` → **73** `warning:` lines (34-EVIDENCE.md #5), directly contradicting this page's line 18 ("Current result: docs build succeeds with no warnings") | Phase 29 — cargo doc zero-`warning:` bar ratified (ADR-0033, D-00a); Phase 33 — 73-warning baseline carried (`33-CI-EVIDENCE.md` row 26) (CURR-02) | MB-01 | M |
| 26 | docs/src/appendix/flow-dsl-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 27 | docs/src/appendix/grove.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 28 | docs/src/appendix/integration-tests.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 29 | docs/src/appendix/minio-file-repository-setup.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 30 | docs/src/appendix/performance-baseline.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 31 | docs/src/appendix/port-trait-template.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 32 | docs/src/appendix/provider-expansion.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 33 | docs/src/appendix/redis-queue-adapter-setup.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 34 | docs/src/appendix/release-automation.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 35 | docs/src/appendix/release-checklist.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 36 | docs/src/appendix/release-recovery.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 37 | docs/src/appendix/sanctum-benchmarks.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 38 | docs/src/appendix/sanctum-deployment.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 39 | docs/src/appendix/sanctum-migration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 40 | docs/src/appendix/security-scanning.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 41 | docs/src/appendix/sentinel.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 42 | docs/src/appendix/user-rest-api.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 43 | docs/src/appendix/user-system.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 44 | docs/src/architecture/commissary.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 45 | docs/src/architecture/crate-map.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 46 | docs/src/architecture/design-patterns.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 47 | docs/src/architecture/domain-model.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 48 | docs/src/architecture/hexagonal-design.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 49 | docs/src/architecture/overview.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 50 | docs/src/contributing/architecture-decisions.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 51 | docs/src/contributing/branching-model.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 52 | docs/src/contributing/contributing-providers.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 53 | docs/src/contributing/development-setup.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 54 | docs/src/contributing/testing-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 55 | docs/src/deployment-topologies/battalion-orchestration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 56 | docs/src/deployment-topologies/embedded-library.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 57 | docs/src/deployment-topologies/http-service-host.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 58 | docs/src/deployment-topologies/overview.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 59 | docs/src/deployment-topologies/queue-worker.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 60 | docs/src/deployment-topologies/sidecar.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 61 | docs/src/deployment/cicd.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 62 | docs/src/deployment/docker.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 63 | docs/src/deployment/kubernetes.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 64 | docs/src/deployment/production.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 65 | docs/src/getting-started/configuration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 66 | docs/src/getting-started/installation.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 67 | docs/src/getting-started/quickstart.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 68 | docs/src/introduction.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 69 | docs/src/operations/logging.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 70 | docs/src/operations/monitoring.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 71 | docs/src/operations/observability.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 72 | docs/src/operations/performance-tuning.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 73 | docs/src/operations/troubleshooting.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 74 | docs/src/user-guides/agent-orchestrator-bridge.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 75 | docs/src/user-guides/agent-runtime.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 76 | docs/src/user-guides/arsenal-tools.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 77 | docs/src/user-guides/battalion-patterns.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 78 | docs/src/user-guides/content-processing.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 79 | docs/src/user-guides/control-flow.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 80 | docs/src/user-guides/eval-harness.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 81 | docs/src/user-guides/fault-tolerance.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 82 | docs/src/user-guides/garrison-memory.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 83 | docs/src/user-guides/graph-visualization.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 84 | docs/src/user-guides/herald-output.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 85 | docs/src/user-guides/maneuver-flow-dsl.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 86 | docs/src/user-guides/memory-management.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 87 | docs/src/user-guides/orchestration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 88 | docs/src/user-guides/output-formatting.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 89 | docs/src/user-guides/paladin-agents.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 90 | docs/src/user-guides/paladin-configuration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 91 | docs/src/user-guides/parley-and-chronicle.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 92 | docs/src/user-guides/sanctum-vector-memory.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 93 | docs/src/user-guides/tool-integration.md | pending | pending — not yet swept (no signal class run) | — | — | — |

## §3 Rustdoc findings table

Rows land in plans 34-06 (default-feature `cargo doc --workspace --no-deps` enumeration against
the `ci.yml:63` bar) and 34-07 (the per-crate `-D warnings --all-features` sweep, D-14). One row
is fully worked here — the known-answer `HeuristicTokenCounter` case CONTEXT.md names as a method
self-test — to prove the P-01 grep-recovery method plans 34-06/34-07 depend on before either
enumerates a single additional row.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-01 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --all-features --no-deps` (D-12/D-14) | paladin-memory | crates/paladin-memory/src/token_counter/mod.rs:3 | unresolved link (broken_intra_doc_links) | `error: unresolved link to \`HeuristicTokenCounter\`` | grep recovery (no `-->` span — the link lives in a `//!` module-level doc comment, per Pitfall P-01; recovered via `grep -n "HeuristicTokenCounter" crates/paladin-memory/src/token_counter/mod.rs` → `3://! [\`HeuristicTokenCounter\`] is the phase-wide default...`, matching `.planning/WINDOWS.md` row 37 exactly) | 34-EVIDENCE.md #6, #7 | S |

This same warning also appears in the default-feature `cargo doc --workspace --no-deps` run
(34-evidence/34-01-cargo-doc-default.txt line 9, "warning:" not "error:" — the severity differs by
run, the location and message do not); plan 34-06 enumerates it there under its own row without
re-deriving the location, citing back to the row worked above.

## §4 Examples table

Rows land in plan 34-08 (the four `ci.yml:548-558` build invocations, the `doc-examples` gate, and
`live_vendor_smoke`). One row is fully worked here — `examples/README.md` audited as a page in its
own right (D-17) — the second CONTEXT.md-named method self-test.

| EX ID | Program / module | Build invocation | Build status | Currency verdict | Obsolete-API hits | Claimed capability → tree check | Evidence anchor | Size |
|-------|-------------------|-------------------|---------------|-------------------|--------------------|-----------------------------------|------------------|------|
| EX-01 | examples/README.md | n/a — documentation page, not a compiled program | n/a | stale | none (not an API-obsolescence finding) | Line 24 states "Rust 1.70 or later" as the minimum Rust version; `Cargo.toml` `[workspace.package] rust-version = "1.88"` (line 18) is the measured, live MSRV floor — the two disagree | 34-EVIDENCE.md #8 | S |

## §5 Phase 35 work list

Empty. Assembled by plan 34-09 from every `MB-nn` row in §2 once plans 34-03/34-04/34-05 have
swept all 93 pages, ordered per D-21 (blocking `L` items first, then by page order).

## §6 Phase 36 work list

Empty. Assembled by plan 34-09 from every `RD-nn` row in §3 and every `EX-nn` row in §4 once
plans 34-06/34-07/34-08 have completed their sweeps, ordered per D-21.

## §7 Deferred routing

Empty. Assembled by plan 34-09 from `deferred-items.md` — pointers only, per D-19 ("nothing is
absorbed into 35 or 36 by convenience").

---

*Phase: 34-documentation-currency-audit*
*Plan 34-01 wrote the header, all seven section headings, the 92 seeded §2 rows, and the one
worked row in each of §2/§3/§4. Every later plan in this phase appends rows; none renumbers or
removes a row already present here (D-03).*
