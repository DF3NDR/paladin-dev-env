# Phase 34: Documentation Currency Audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-17
**Phase:** 34-documentation-currency-audit
**Mode:** `--auto` — every selection below is the recommended default chosen by Claude without a
user prompt. The maintainer may overrule any row by editing `34-CONTEXT.md` before planning.
**Areas discussed:** Inventory shape & item IDs; mdBook verdict method & shipped-surface baseline;
Rustdoc measurement protocol; Examples build & currency protocol; Partition, sizing & deferred
routing; Read-only enforcement & tool hygiene

---

## Inventory shape & item IDs

| Option | Description | Selected |
|--------|-------------|----------|
| Single `34-AUDIT.md` + `34-EVIDENCE.md` | One canonical inventory with all three verdict tables and both work lists; verbatim tool captures in a companion evidence file (the `NN-CI-EVIDENCE.md` house pattern) | ✓ |
| One file per partition + index | Phase 16 style (`16-DOCS-01-VERDICTS.md` etc.) with a summary index | |
| Everything in one file including raw output | Tables and logs interleaved | |

**Auto-selected:** Single `34-AUDIT.md` + `34-EVIDENCE.md`
**Notes:** Success Criteria 1 and 4 say "a single audit document" / "the inventory"; one file keeps
that literally true. Raw logs would bloat the tables, hence the evidence companion.

| Option | Description | Selected |
|--------|-------------|----------|
| `MB-nn` / `RD-nn` / `EX-nn` IDs + S/M/L by edit kind | Stable, partition-prefixed IDs that Phases 35/36 close by name; size defined by edit kind not hours | ✓ |
| Sequential `F-nn` + hour estimates | One sequence, time-based sizing | |
| No IDs, group by page | Rows only, closure tracked by page path | |

**Auto-selected:** `MB-nn` / `RD-nn` / `EX-nn` + S/M/L by edit kind
**Notes:** Hour estimates for doc edits are noise; edit kind is what the planner batches on.

---

## mdBook verdict method & shipped-surface baseline

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 16 signal classes + shipped-surface checklist | Eight grep-producing signals plus a ninth class checking the page against a per-phase list of shipped items compiled first | ✓ |
| Prose read only | Read each page end to end, judge by understanding | |
| mtime / git-log heuristic | Treat recently touched pages as current | |

**Auto-selected:** Phase 16 signals + shipped-surface checklist
**Notes:** Phase 16 D-00e already rejected mtime as evidence; prose-only cannot cite "the shipped
item the page fails to describe" as SC1 demands.

| Option | Description | Selected |
|--------|-------------|----------|
| All 93 `.md` incl. appendix + orphans | Every file on disk under `docs/src`, orphans flagged | ✓ |
| SUMMARY.md-linked pages only | Skip files not in the nav | |
| User guides + API reference only | Skip appendix, deployment, operations | |

**Auto-selected:** All 93 including appendix and orphans
**Notes:** SC1 says "every mdBook page under `docs/src/`"; the appendix holds the oldest pages.

| Option | Description | Selected |
|--------|-------------|----------|
| CHANGELOG + MIGRATION §9 + exports diff + REQUIREMENTS | Mechanical sources first; per-phase CONTEXT only to clarify | ✓ |
| Read every phase SUMMARY | 158 plan summaries across 13 phases | |
| Read every phase CONTEXT in full | 13 CONTEXT files, ~200 decisions | |

**Auto-selected:** CHANGELOG + MIGRATION + exports diff + REQUIREMENTS
**Notes:** Universal anti-pattern rules 6-7 forbid reading other phases' plans in full and cap
SUMMARY reads at frontmatter; the mechanical sources are complete by construction (X-10 gates).

---

## Rustdoc measurement protocol

| Option | Description | Selected |
|--------|-------------|----------|
| CI commands verbatim + per-crate `--all-features` runs, locally, toolchain recorded | Run `ci.yml:63` and the `-D warnings --all-features` command as written, then per-crate runs so the build-order abort at `paladin-ai-core` cannot truncate the enumeration | ✓ |
| CI commands only, trust the CI log | Quote the last CI run's output | |
| Per-crate only | Skip the workspace-level commands | |

**Auto-selected:** CI commands + per-crate runs locally
**Notes:** SC2 requires the `ci.yml` command quoted verbatim and every line enumerated; the
workspace `-D warnings` run stops at the first failing crate (32-05-SUMMARY), so per-crate runs
are necessary for completeness. Doctests added as a baseline because Phase 36 SC3 needs them and
the coverage/`--tests` gates skip them.

---

## Examples build & currency protocol

| Option | Description | Selected |
|--------|-------------|----------|
| Four `ci.yml` invocations + `check-doc-examples.sh` | The exact split CI uses (bulk selector silently skips 4 gated targets) plus the docs-job script for `doc-examples` | ✓ |
| Single `cargo build --examples` | One command | |
| `cargo build --all-targets --all-features` | Broad build | |

**Auto-selected:** Four CI invocations + `check-doc-examples.sh`
**Notes:** The single-command form under-covers (43/47) with exit 0; `--all-features` builds a
different feature graph than any CI job and conflicts with `cli_isolation`.

| Option | Description | Selected |
|--------|-------------|----------|
| Capability map + removed-API/vocabulary check + gap list | Three recorded checks per program and a separate undemonstrated-capability table | ✓ |
| Build status only | Green/red per target | |
| Build status + README cross-check only | Green/red plus README consistency | |

**Auto-selected:** Capability map + vocabulary + gap list
**Notes:** SC3 asks "which Phase 22-33 API it should demonstrate but does not" — build status
alone cannot answer that.

---

## Partition, sizing & deferred routing

| Option | Description | Selected |
|--------|-------------|----------|
| Phase-local `deferred-items.md` | The register Phases 25/28/31/32 executors already use | ✓ |
| New milestone-level register | A new `.planning/DEFERRED.md` | |
| Append to WINDOWS.md directly | Rows via direct edit | |

**Auto-selected:** Phase-local `deferred-items.md`
**Notes:** Direct WINDOWS.md edits violate anti-pattern rule 15; a new register duplicates an
existing pattern.

| Option | Description | Selected |
|--------|-------------|----------|
| `CURR-*` | Documentation *currency*; fits 34, 35 and 36 | ✓ |
| `AUDIT-*` | Fits 34 only; 35/36 fix rather than audit | |
| `BOOK-*` / `RDOC-*` split | Two prefixes for one work stream | |

**Auto-selected:** `CURR-*` (recommendation to the planner, which mints requirements)
**Notes:** `DOCS-*` is spent (Phase 16); the ROADMAP says 35/36 share Phase 34's prefix.

---

## Read-only enforcement & tool hygiene

| Option | Description | Selected |
|--------|-------------|----------|
| Recorded `git diff --stat … ':!.planning'` per commit | Mechanical proof in the evidence file; `mdbook-mermaid install` side effects checked and reverted | ✓ |
| Trust the executor | Assert in SUMMARY | |
| Branch protection rule | A CI/ruleset change (itself outside `.planning/`) | |

**Auto-selected:** Recorded diff check per commit
**Notes:** SC5 is a "what must be TRUE" criterion; the evidence must be a command's output.

---

## Claude's Discretion

- Table column layout beyond the mandatory columns.
- Shipped-surface checklist layout (per-phase tables vs one table with a phase column).
- Whether `34-EVIDENCE.md` inlines raw captures or links a `34-evidence/` subdirectory.
- Plan count and wave shape (suggested: checklist → three parallel partitions → assembly).

## Deferred Ideas

- PROJECT.md's "no crate ships its own `examples/`" is contradicted by
  `crates/paladin-llm/examples/live_vendor_smoke.rs` — planning-corpus correction.
- `docs/src/appendix/doc-coverage-report.md` disposition (archive vs regenerate) — Phase 35.
- A rustdoc gate in `make clean-code` / pre-push — Phase 36 SC2.
- Devcontainer vs CI lint-toolchain drift — recorded if seen; pinning is a later decision.
- The two folded todos' non-documentation remainders (Docker-machine coverage walk; RustFS
  evaluation) stay open as pending todos.
