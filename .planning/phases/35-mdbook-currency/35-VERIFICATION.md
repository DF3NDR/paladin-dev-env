---
phase: 35-mdbook-currency
verified: 2026-09-17T16:20:00Z
status: passed
score: 9/9 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 8/9
  gaps_closed:
    - "The superstep-engine guide's compile-verified `run_engine` example correctly demonstrates `EngineConfig`/`EngineLimits` bounding a cyclic graph's iteration"
  gaps_remaining: []
  regressions: []
human_verification: []
---

# Phase 35: mdBook Currency Verification Report

**Phase Goal:** The mdBook describes the v0.10.0 tree — every gap the Phase 34 inventory records
for `docs/src/` is closed: a page exists for each Phase 22-33 capability that shipped without one,
every stale page is corrected to the shipped API and vocabulary (the superstep engine, Parley,
Aegis, the platform API, the `TokenUsage` split, `Commissary`), the Upgrading page and migration
pointers agree with `MIGRATION.md`, and `mdbook build` with the linkcheck backend is green.
**Verified:** 2026-09-17T16:20:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure

## Re-verification Note

The prior run (2026-09-17T15:05:00Z, `gaps_found`, 8/9) failed exactly one must-have: `run_engine()`
in `crates/doc-examples/src/superstep_engine.rs` computed `EngineLimits` via `configure_limits()`
and discarded them into an underscore-prefixed `_limits` binding, while `build_graph()` hardcoded
`WarGraph::new(schema, EngineLimits::default())` — the guide's headline "Bounded Iteration" example
never actually enforced the bounds it claimed to demonstrate.

Three commits landed on `feature/phase-33` since that run (tree now at `9d1b07a3`):

- **`2265b7e6`** (CR-01, the blocking gap) — changed `build_graph()`'s signature to
  `build_graph(limits: EngineLimits)`, threading the parameter into `WarGraph::new(schema, limits)`;
  `run_engine()` now calls `configure_limits()` and passes its `EngineLimits` half straight into
  `build_graph(limits)`; the `build_graph` doc comment and the page's "Building a Graph" section
  prose were updated to explain where the `EngineLimits` argument comes from.
- **`74efa469`** (WR-01, non-blocking warning) — factored a `csv_escape` helper into
  `crates/doc-examples/src/herald_output.rs` and applied it consistently across all four
  field-producing `CsvHerald` methods (previously only one method escaped commas).
- **`6ad9a55b`** (WR-02, non-blocking warning) — `arsenal_tools.rs`'s `validate_call` now matches
  on `call.arguments.get("expression")` and requires a JSON string, rejecting a present-but-wrong-
  typed value before `invoke` would ever see it (previously only checked key presence).

All three are independently confirmed below by reading the current source directly, not by
trusting `35-REVIEW-FIX.md`'s claims. No regressions were found in the eight previously-verified
must-haves.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Every Phase 34 mdBook work-list item (60 `MB-nn` rows) is closed by a page edit or new page, nav-linked where new (ROADMAP SC1 / CURR-06) | ✓ VERIFIED | Re-ran `for n in 01..60; git log --grep "MB-$n"` myself against the current tree — all 60 IDs found, 0 missing |
| 2 | `mdbook build docs/` with the linkcheck backend passes with zero broken links, exact `docs.yml` sequence including `mdbook-mermaid install` (ROADMAP SC2 / CURR-07) | ✓ VERIFIED | Ran `mdbook-mermaid install docs/` (exit 0, `git status --porcelain -- docs` empty) then `mdbook build docs/` myself — `[INFO mdbook_linkcheck] No broken links found` |
| 3 | No touched page names a type/function/config key/route/CLI flag the v0.10.0 tree does not export; runnable snippets are compile-verified in `crates/doc-examples`, illustrative snippets marked as such (ROADMAP SC3 / CURR-08) | ✓ VERIFIED | `./scripts/check-doc-examples.sh` → "All included examples compile.", 0 failed (re-ran myself). **Gap closed:** read `crates/doc-examples/src/superstep_engine.rs:79-163` directly — `build_graph(limits: EngineLimits)` now takes the limits parameter and constructs `WarGraph::new(schema, limits)` (line 92, no more hardcoded `EngineLimits::default()`); `run_engine()` (lines 152-163) calls `configure_limits()?` then passes the resulting `limits` straight into `build_graph(limits)?` — nothing is discarded. All four `// ANCHOR:`/`// ANCHOR_END:` pairs (`build_graph`, `configure_limits`, `run_engine`, `inspect_waypoints`) are intact and all four are still `{{#include}}`d by `docs/src/user-guides/superstep-engine.md` (verified both files directly). The page's "Building a Graph" prose (lines 44-48) was updated to explain the parameter comes from `configure_limits` (shown later on the page) or `EngineLimits::default()` directly — no longer contradicts the code |
| 4 | Book vocabulary matches the three ubiquitous-language lists — no `Quartermaster`, no bare token total where the Phase 31 prompt/completion split shipped (ROADMAP SC4 / CURR-09) | ✓ VERIFIED | Re-ran `grep -rniE '\bQuartermaster\b' docs/src` myself — empty |
| 5 | `CHANGELOG.md` `[0.10.0]` carries a Documentation entry summarising pages added/corrected (ROADMAP SC5 / CURR-10) | ✓ VERIFIED | Re-confirmed `### Documentation` section present (line 421), `grep -n "MB-[0-9]" CHANGELOG.md` empty |
| 6 | New `docs/src/user-guides/superstep-engine.md` page exists, correctly positioned in nav, with the four required doc-examples anchors | ✓ VERIFIED | File exists; `docs/src/SUMMARY.md` nav entry confirmed at line 25 (between Maneuver Flow DSL and Control Flow); all 4 anchors present in `superstep_engine.rs` and registered via `lib.rs` |
| 7 | D-10 forward links from introduction.md, architecture/overview.md, architecture/domain-model.md and control-flow.md to the new engine guide | ✓ VERIFIED | Unchanged since prior run — not touched by the fix commits; all four pages still link `superstep-engine.md` |
| 8 | Upgrading page and migration pointers agree with `MIGRATION.md` (goal clause; D-00f) | ✓ VERIFIED | Unchanged since prior run — not touched by the fix commits |
| 9 | Requirement traceability: CURR-06..10 declared by plans, minted in REQUIREMENTS.md, mapped 1:1 to Phase 35 with no orphans | ✓ VERIFIED | Re-confirmed: every plan's `requirements:` frontmatter cites a subset of {CURR-06..10}; `.planning/REQUIREMENTS.md` lines 495-512 declare all five, lines 628-632 map all five to Phase 35 ("Pending" — flipped at phase-completion tracking, not a Phase 35 plan deliverable, per Phase 34 precedent) |

**Score:** 9/9 truths verified (0 partial, 0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/src/user-guides/superstep-engine.md` | New WarEngine guide, nav-linked | ✓ VERIFIED | Exists, correct H1/Since header, correct nav position, prose now matches the fixed `build_graph(limits)` call shape |
| `crates/doc-examples/src/superstep_engine.rs` | 4 compile-verified anchors | ✓ VERIFIED | Compiles (`cargo check -p paladin-doc-examples` clean), registered in `lib.rs`; `run_engine` anchor now correctly threads `configure_limits()`'s `EngineLimits` into `build_graph()` — gap closed |
| `.planning/phases/35-mdbook-currency/deferred-items.md` | Deferred register with proposed classifications | ✓ VERIFIED | Unchanged since prior run |
| `docs/src/contributing/adr-index.md` | New ADR index page | ✓ VERIFIED | Unchanged since prior run |
| `35-EVIDENCE.md` | 60-row closure table + verbatim gate run | ✓ VERIFIED | Unchanged since prior run |
| `CHANGELOG.md` | `[0.10.0]` Documentation subsection | ✓ VERIFIED | Present, correctly positioned, zero `MB-nn` identifiers |
| Five new `doc-examples` modules (35-02) | paladin_agents, arsenal_tools, herald_output, battalion_patterns, sanctum_vector_memory | ✓ VERIFIED | Compiles; the two minor reference-implementation defects (WR-01 CSV escaping, WR-02 type validation) are now fixed — see below |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `docs/src/SUMMARY.md` | `user-guides/superstep-engine.md` | nav entry | ✓ WIRED | Correct position, verified directly |
| Engine page `{{#include}}`s | `crates/doc-examples/src/superstep_engine.rs` anchors | mdBook include | ✓ WIRED, semantically correct | All four anchors resolve; `run_engine`'s wiring to `configure_limits`'s output is now correct (gap closed) |
| `.github/workflows/ci.yml`/`release.yml` | `docs/src/deployment/cicd.md` job table | D-15 source of truth | ✓ WIRED | Unchanged since prior run |
| `crates/paladin-ports/src/output/*` | Appendix port imports (35-09) | D-13 import rule | ✓ WIRED | Unchanged since prior run |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| mdBook builds with linkcheck | `mdbook-mermaid install docs/ && mdbook build docs/` | exit 0, "No broken links found" | ✓ PASS (re-run) |
| Doc examples compile | `./scripts/check-doc-examples.sh` | "All included examples compile.", 0 failed | ✓ PASS (re-run) |
| `cargo check -p paladin-doc-examples` | direct compile check | `Finished` profile, no errors | ✓ PASS (re-run) |
| All 60 MB-nn IDs closed | `for n in 01..60; git log --grep "MB-$n"` | 0 missing | ✓ PASS (re-run) |
| No `Quartermaster` leakage | `grep -rniE '\bQuartermaster\b' docs/src` | empty | ✓ PASS (re-run) |
| `make api-surface` | unchanged public API | "API surface unchanged" (3959 items) | ✓ PASS (re-run) |
| **`run_engine` example enforces configured `EngineLimits`** | Read `crates/doc-examples/src/superstep_engine.rs:79-163` | `build_graph(limits)` uses the passed-in limits; `run_engine()` threads `configure_limits()`'s output into it | ✓ PASS — gap closed, confirmed by direct source read |
| CsvHerald escaping consistency (WR-01) | Read `crates/doc-examples/src/herald_output.rs:1-60` | `csv_escape` helper applied across all four field-producing methods (`format_paladin_result`, `format_battalion_result`, `finalize_stream`, `format_error`) | ✓ PASS |
| `validate_call` type checking (WR-02) | Read `crates/doc-examples/src/arsenal_tools.rs:41-66` | `validate_call` now matches on `Some(v) if v.is_string()` / `Some(_)` / `None`, rejecting wrong-typed values before `invoke` | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|-------------|--------|----------|
| CURR-06 | 35-01..09 | Every Phase 34 mdBook work-list item closed, nav-linked | ✓ SATISFIED | 60/60 MB-nn IDs closed and reconciled |
| CURR-07 | 35-01..10 | `mdbook build` + linkcheck green | ✓ SATISFIED | Independently re-run, exit 0 |
| CURR-08 | 35-01..09 | No unexported symbols named; runnable snippets compile-verified | ✓ SATISFIED | Compiles cleanly, and the flagship new example's runtime behavior now matches its own documentation — gap closed by `2265b7e6` |
| CURR-09 | 35-04 | Vocabulary matches ubiquitous-language lists | ✓ SATISFIED | Quartermaster grep empty |
| CURR-10 | 35-10 | CHANGELOG Documentation entry | ✓ SATISFIED | Present, correctly positioned, zero MB-nn leakage |

No orphaned requirements: REQUIREMENTS.md's Phase 35 rows (CURR-06..10) all appear in at least one
plan's `requirements:` frontmatter, and no plan cites a Phase-35-prefixed requirement absent from
REQUIREMENTS.md.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/doc-examples/src/superstep_engine.rs` | — | ~~Configured value computed then discarded~~ | — | **Fixed by `2265b7e6`** — `build_graph()` now takes and uses the `EngineLimits` parameter; no longer a finding |
| `crates/doc-examples/src/herald_output.rs` | — | ~~Inconsistent comma escaping~~ | — | **Fixed by `74efa469`** — `csv_escape` applied consistently across all four field-producing methods; no longer a finding |
| `crates/doc-examples/src/arsenal_tools.rs` | — | ~~`validate_call` checks key presence, not type~~ | — | **Fixed by `6ad9a55b`** — now rejects present-but-wrong-typed values; no longer a finding |
| `crates/doc-examples/src/arsenal_tools.rs` | 48 | `CalculatorTool::invoke` never evaluates `expr`, always returns hardcoded `42` | ℹ️ Info | Explicitly flagged by its own inline comment (`// ... evaluate \`expr\` ...`); cosmetic, out of the fix's declared scope (IN-01, info-level), no fix required |
| `docs/src/api-reference/stable-api.md`, `docs/src/appendix/contributing-legacy.md`, `docs/src/appendix/sanctum-benchmarks.md` | 856, 341, 285 | Pre-existing `TBD` markers | ℹ️ Info — not a Phase 35 regression | Untouched by this phase's diffs, per prior run's `git blame` confirmation; re-confirmed no new `TBD`/`FIXME`/`XXX` markers were introduced in any of the three fix-commit-touched files |

### Human Verification Required

None. All must-haves are either mechanically verifiable (grep/build/compile) or were independently
spot-checked against source.

### Gaps Summary

**No gaps.** The single blocking gap from the initial verification (2026-09-17T15:05:00Z) — the
superstep-engine guide's `run_engine` example discarding `configure_limits()`'s `EngineLimits`
instead of using them to bound the graph it runs — is closed. Read directly from
`crates/doc-examples/src/superstep_engine.rs` on the current tree (`9d1b07a3`): `build_graph()`
now accepts an `EngineLimits` parameter and constructs the graph with it; `run_engine()` computes
`configure_limits()`'s limits and passes them straight through, with nothing discarded. All four
`{{#include}}` anchors the guide page depends on are unchanged and still resolve; the page's prose
was updated to match the new call shape. `mdbook build docs/` (linkcheck) and
`scripts/check-doc-examples.sh` were both re-run directly on the current tree and are green. All
eight previously-verified must-haves were re-checked (MB-nn closure, Quartermaster grep, CHANGELOG
Documentation section, nav entry, API surface, requirements traceability) and show no regression.
The two non-blocking warnings from the initial review (CSV escaping inconsistency, type-blind
`validate_call`) are also fixed, confirmed by direct source read.

Phase 35 goal achieved: the mdBook describes the v0.10.0 tree, all 60 Phase 34 work-list items are
closed, and the flagship new superstep-engine guide's example now correctly demonstrates the
bounded-iteration behavior it teaches.

---

_Verified: 2026-09-17T16:20:00Z_
_Verifier: Claude (gsd-verifier)_
