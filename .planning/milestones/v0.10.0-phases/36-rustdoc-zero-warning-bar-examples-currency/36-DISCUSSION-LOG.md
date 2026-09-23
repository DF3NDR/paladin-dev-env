# Phase 36: Rustdoc Zero-Warning Bar & Examples Currency - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-17
**Phase:** 36-rustdoc-zero-warning-bar-examples-currency
**Mode:** `--auto` — no user prompts; the recommended option was selected for every question
and logged inline. `[auto]` marks each selection.
**Areas discussed:** Rustdoc fix technique & re-baseline, Gate wiring, Example gap-list grouping,
Stale examples & README closure, Doc-test rule boundary, Closure bookkeeping

---

## Rustdoc fix technique & re-baseline

| Option | Description | Selected |
|--------|-------------|----------|
| De-link private targets to plain code font | Keeps api-surface unchanged; the defect is corrected, not hidden | ✓ |
| Widen visibility of the private target | Would move `.project/current-exports.txt` and trip SC5 | |
| `#[allow(rustdoc::private_intra_doc_links)]` | Hides the defect the bar exists to catch | |

**[auto] choice:** De-link; never widen visibility, never allow-list (D-05).

| Option | Description | Selected |
|--------|-------------|----------|
| Re-measure at research time and diff against §6 | Phase 35 added six `doc-examples` modules to the `--workspace` doc build; D-23 rule is triggered | ✓ |
| Trust the 143 rows unchanged | Cheaper, but the bar is zero, not 143 | |

**[auto] choice:** Re-measure; mint `RD-144+` / `EX-123+` for new rows; record `closed-by-drift` for rows that no longer reproduce (D-01, D-02).
**Notes:** Verified at discuss time that no library source moved since the audit SHA (only `Cargo.lock` +1 line outside `docs/` and `doc-examples`).

---

## Gate wiring

| Option | Description | Selected |
|--------|-------------|----------|
| `make doc-check` + `clean-code` dependency + pre-push hook | One source of truth, both local entry points, wired in the last wave | ✓ |
| Pre-push hook only | Misses `make clean-code`, which CLAUDE.md names as the pre-commit ritual | |
| `clean-code` only | Misses the push-time gate that catches GSD executor commits bypassing commit hooks | |

**[auto] choice:** All three, last wave (D-11, D-13).

| Option | Description | Selected |
|--------|-------------|----------|
| Add the all-features `-D warnings` run to the CI lint job | Mirrors `ci.yml:63`; proven by a real run in `36-CI-EVIDENCE.md` | ✓ |
| Local gates only | ROADMAP SC2's literal minimum, but `--no-verify` pushes would still regrow the count | |

**[auto] choice:** Add the CI step (D-12).
**Notes:** `cargo test --workspace --doc` already runs in CI at `ci.yml:499`; not duplicated. `scripts/check-all-examples.sh` is rewritten to the CI four-invocation split but kept out of pre-push (D-14).

---

## Example gap-list grouping

| Option | Description | Selected |
|--------|-------------|----------|
| One program per capability cluster (≈10-14) | Each `EX-nn` maps to a named program + README section; runnable stories | ✓ |
| One program per gap row (59 files) | Literal reading of "sized L each"; unmaintainable gallery | |
| `doc-examples` anchors only | Compile-verified but not "runnable example listed in `examples/README.md`" (SC4) | |

**[auto] choice:** Per cluster (D-15).

| Option | Description | Selected |
|--------|-------------|----------|
| Default features + mock LLM first; gate only where the capability is the feature | Keeps most programs under the bare `--examples` selector; gated ones get `[[example]]` + a CI invocation | ✓ |
| Everything under `--all-features` | Hides the required-features gap CI documents at `ci.yml:538-546` | |

**[auto] choice:** Default-first (D-16, D-17). No deletions expected (D-18).

---

## Stale examples & README closure

| Option | Description | Selected |
|--------|-------------|----------|
| Mount `thread_router` and `run_router` in `http_service_host` (both copies) | Makes the server-parity claim true; re-renders the topology pages via `doc-examples` | ✓ |
| Narrow the claim in comments/README | Leaves the example behind the shipped server | |

**[auto] choice:** Mount the routers (D-19).

| Option | Description | Selected |
|--------|-------------|----------|
| Fix the three stale snippet lines; new sections carry no snippet | Bounded drift; nothing compile-checks README snippets | ✓ |
| Drop every snippet block | Larger diff for no currency gain | |
| Keep and mirror every file | A fourth unchecked copy of every API shape | |

**[auto] choice:** Fix existing, no snippets in new sections (D-20, D-21, D-22).

---

## Doc-test rule boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Leave the 19 MISSING headings and the script drift to Phase 36.1 | 36.1 SC2 owns the disposition; Phase 34 D-00e froze the 76-item scope | ✓ |
| Fix the 19 here | Widens a rule scope Phase 36 has no mandate to change; double-owns a 36.1 deliverable | |
| Refreeze the entry-point file at 101 here | Same objection | |

**[auto] choice:** Leave to 36.1; Phase 36 keeps doctests green (D-23).

---

## Closure bookkeeping

| Option | Description | Selected |
|--------|-------------|----------|
| House pattern: `36-EVIDENCE.md` + `36-evidence/`, closure table per SUMMARY, `36-CI-EVIDENCE.md`, WINDOWS via gsd-tools, `CURR-*` | Matches Phases 33-35 | ✓ |
| Inline evidence in SUMMARYs only | Loses the verbatim captures Phase 36.1 and 37 verify against | |

**[auto] choice:** House pattern (D-24…D-28).

---

## Claude's Discretion

- Plan count and wave shape; RAG cluster placement; how EX-107 is made runnable; cluster
  program file names; whether the per-crate sweep joins `make doc-check`.

## Deferred Ideas

- Regenerating `doc-coverage-report.md`; `check-examples` in pre-push; a `docsrs` cfg build;
  consolidating the lint job's rustdoc steps into a matrix; Phase 36.1's prose and entry-point
  items; the two pending todos (reviewed, not folded — Phase 36.1 SC5 owns them).
