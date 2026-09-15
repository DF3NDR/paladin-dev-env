---
phase: 32-unified-token-primitives
plan: 05
subsystem: release-hygiene
tags: [rust, semver, migration-register, changelog, coverage, cargo-doc, cargo-semver-checks]

# Dependency graph
requires:
  - phase: 32-01
    provides: "TokenCounterPort::is_exact; Commissary::new/from_port with the exactness argument removed"
  - phase: 32-02
    provides: "paladin_llm::window::resolve_context_window, WindowFallbackPolicy, WindowSource"
  - phase: 32-03
    provides: "Legacy garrison::TokenCounter trait and TokenCounterFactory removed; re-exports narrowed to TiktokenCounter"
  - phase: 32-04
    provides: "Commissary and HistoryTrimmer both wired to the shared resolver; local LimitSource enum deleted"
provides:
  - "Six empirically-derived cargo-semver-checks discovery runs (four --default-features, two --features content-processing), all carrying --release-type minor, none reporting zero checks evaluated"
  - "MIGRATION.md section 9.2 rows for the crate-and-type pairs a lint actually fired for (paladin-memory | TokenCounter, paladin-memory | TokenCounterFactory), with matching .cargo/semver-checks-allowlist.toml entries in the same commit"
  - "CHANGELOG.md [0.10.0] Changed/Removed/Added bullets covering the Commissary constructor break, the shared resolver, the legacy pair removal, and TokenCounterPort::is_exact"
  - "Token primitives subsections on docs/src/api-reference/upgrading.md and migration-guide.md"
  - "The PRIM-05 phase gate evidence: make clean-code, workspace tests + explicit doctests, zero-warning cargo doc, mdbook build, make security, coverage (measured or attributed to CI), the row-level set-equality re-run, and the final exit grep"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Empirical semver-lint discovery, never guessed: six recorded runs, exact commands and tool version pinned, every lint id traced to captured output"
    - "cargo-semver-checks' --features <name> (without --default-features or --only-explicit-features) triggers an 'enable everything except unstable/nightly/bench/no_std' heuristic that can pull in unrelated optional features (here: qdrant) and surface pre-existing, unrelated dependency-version drift between the workspace's committed Cargo.lock and cargo-semver-checks' own from-scratch placeholder-crate resolution"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - CHANGELOG.md
    - docs/src/api-reference/upgrading.md
    - docs/src/api-reference/migration-guide.md

key-decisions:
  - "No MIGRATION.md row or allowlist entry was written for paladin-llm | Commissary or paladin-ports | TokenCounterPort -- both fired zero cargo-semver-checks 0.50.0 lints across every discovery run, confirmed by inspecting the tool's full 254-lint catalog (no lint checks an inherent/associated function's parameter count). Both breaks are still recorded in CHANGELOG.md, independent of semver-checks detection. This follows the plan's own EDGE(PRIM-05/empty) rule over its 'expected' framing where the two conflicted."
  - "No crates/paladin-llm/Cargo.toml, crates/paladin-memory/Cargo.toml or crates/paladin-ports/Cargo.toml lints table was created or extended -- neither fired lint occurred on a default-features run, so none needed a suppression to keep CI's semver job green."
  - "The RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps gate is recorded RED, following Phase 31's own explicit precedent for this identical command (31-07-SUMMARY.md): the 14 unresolved-link errors are pre-existing, confined to paladin-ai-core's graph-fingerprinting and webhook-delivery doc families, and outside this plan's declared files_modified -- not fixed, per the deviation rules' Scope Boundary."

patterns-established:
  - "A crate/type pair with zero fired semver-checks lints across every recorded discovery run gets zero MIGRATION.md rows and zero allowlist entries, even when CONTEXT.md's own prose framed it as an expected break -- empirical tool output overrides an authored expectation, and the gap is explained (tool coverage limitation, confirmed via --list) rather than silently absorbed."

requirements-completed: [PRIM-05]

coverage:
  - id: D1
    description: "Six empirical cargo-semver-checks discovery runs (four --default-features, two --features content-processing), all carrying --release-type minor, tool version 0.50.0 confirmed matching the CI pin; none report zero checks evaluated. paladin-memory's content-processing run fires struct_missing (TokenCounterFactory) and trait_missing (TokenCounter); all other five runs fire nothing, including paladin-llm's default run for the Commissary constructor break -- confirmed as a genuine cargo-semver-checks 0.50.0 coverage gap (no lint checks inherent-method parameter count), not a guessed or suppressed absence."
    requirement: "PRIM-05"
    verification:
      - kind: other
        ref: "32-05-SUMMARY.md Task 1 -- six captured discovery run transcripts plus the cargo semver-checks --list catalog inspection"
        status: pass
    human_judgment: false
  - id: D2
    description: "MIGRATION.md gains two new section 9.2 rows (paladin-memory | TokenCounter, paladin-memory | TokenCounterFactory) with matching .cargo/semver-checks-allowlist.toml entries in the same commit; the CI row-level set-equality step, copied verbatim and run locally, exits 0 and reports set-equal in both directions."
    requirement: "PRIM-05"
    verification:
      - kind: other
        ref: "32-05-SUMMARY.md Task 2 -- row-level set-equality script output (all 15 pairs match in both directions)"
        status: pass
    human_judgment: false
  - id: D3
    description: "CHANGELOG.md [0.10.0] gains one Changed, one Removed and one Added bullet; both reader-facing docs pages gain a Token primitives section linking to MIGRATION.md section 9.2; every new version string reads v0.10.0."
    requirement: "PRIM-05"
    verification:
      - kind: other
        ref: "grep -q '^## Token primitives' docs/src/api-reference/upgrading.md; grep -q '^### Token primitives' docs/src/api-reference/migration-guide.md; ! grep -rqn 'v0\\.11\\.0' CHANGELOG.md docs/src/api-reference/upgrading.md docs/src/api-reference/migration-guide.md (all pass)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Full PRIM-05 phase gate evidence recorded: make clean-code GREEN, workspace tests 6771 passed / 1 known pre-existing failure, explicit doctests GREEN (137 + 147 passed), cargo doc RED (pre-existing, unrelated, matches Phase 31's own precedent for this command), mdbook build GREEN, make security GREEN, coverage measured locally at 90.25% against the 82% floor, final exit grep empty with the exclusion confirmed meaningful."
    requirement: "PRIM-05"
    verification:
      - kind: other
        ref: "32-05-SUMMARY.md Task 3 -- full gate evidence section"
        status: pass
    human_judgment: true
    rationale: "The cargo doc gate is honestly RED (pre-existing, unrelated); a human should confirm this matches the accepted Phase 31 precedent rather than being auto-classified as a full pass."

# Metrics
duration: ~80min
completed: 2026-09-15
status: complete
---

# Phase 32 Plan 05: Release Bookkeeping — Semver Discovery, Migration Register, Gate Evidence Summary

**Six empirical `cargo-semver-checks` discovery runs (with a Rule-3-adapted command for one unrelated dependency-drift blocker) feed two real MIGRATION.md rows and allowlist entries, three CHANGELOG bullets, two new docs sections, and the full PRIM-05 gate evidence — including an honestly-recorded pre-existing `cargo doc` red gate and a real, measured 90.25% local coverage figure.**

## Performance

- **Duration:** ~80 min
- **Started:** 2026-09-15T17:25:00Z (approximate — Task 1's first discovery run)
- **Completed:** 2026-09-15T18:45:00Z (approximate)
- **Tasks:** 3
- **Files modified:** 5 (`MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `CHANGELOG.md`, `docs/src/api-reference/upgrading.md`, `docs/src/api-reference/migration-guide.md` — no `Cargo.toml` file, since no lint fired on any default-features run)

## Accomplishments

- Six real `cargo-semver-checks 0.50.0` discovery runs recorded verbatim, every command carrying `--release-type minor`, none reporting zero checks evaluated
- Diagnosed and worked around a genuine, pre-existing, unrelated `qdrant-client` 1.18.0-vs-1.19.0 dependency-resolution drift in `cargo-semver-checks`' own from-scratch placeholder-crate build (Rule 3), confirmed via direct inspection of the tool's scratch `Cargo.lock` and `Cargo.toml`
- Confirmed, via the tool's own 254-lint catalog, that `cargo-semver-checks 0.50.0` has no lint covering an inherent/associated function's parameter-count change — explaining, not guessing, why the `Commissary::new`/`from_port` break produced zero tool output
- Two new `MIGRATION.md` §9.2 rows and two matching `.cargo/semver-checks-allowlist.toml` entries, landed in the same commit, for the only crate/type pairs a lint actually fired for
- Three `CHANGELOG.md` `[0.10.0]` bullets and two new `## Token primitives` / `### Token primitives` docs sections
- The CI row-level set-equality step re-run locally, verbatim, exit 0
- The full PRIM-05 gate evidence recorded honestly, including one pre-existing RED gate (`cargo doc`, matching Phase 31's own precedent) and a real, measured local coverage figure (90.25%, not a claimed CI-only reproduction)

## Task Commits

Each task was committed atomically:

1. **Task 1: run empirical semver discovery and record every lint that actually fires** - `d3944c6d` (docs)
2. **Task 2: write the section 9.2 rows, the matching allowlist entries, the lint tables, the CHANGELOG bullets and the two migration subsections** - `54598f1a` (docs)
3. **Task 3: run and record the PRIM-05 phase gate evidence** - committed with this SUMMARY's closing commit (docs)

## Task 1: Empirical semver-lint discovery

**Tool version:** `cargo-semver-checks 0.50.0` (confirmed via `cargo semver-checks --version`, matches the CI pin `taiki-e/install-action@v2` in `.github/workflows/ci.yml`).

All six runs below carry `--release-type minor` (D-14's mandatory flag — without it, a crate already bumped to `0.10.0` is treated as major-equivalent against the `0.9.0` baseline and the tool skips every lint, printing `0 checks: 0 pass, N skip` and exiting 0; that failure signature is what this run avoids).

### Run 1 — `paladin-ports`, default features

```
cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-ports v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.232s] 194 checks: 194 pass, 60 skip
     Summary no semver update required
```

**Result: 194 checks evaluated, 0 fail.** No lint fired for `TokenCounterPort::is_exact` — the crate's own `[package.metadata.cargo-semver-checks.lints]` table (`enum_marked_non_exhaustive`/`struct_marked_non_exhaustive` = allow, from Phase 26/31) has no bearing on trait-method-addition lints, so this is a genuine "nothing fired" result, not a suppressed one. Matches D-07's own hedge ("likely nothing fires" for an additive defaulted trait method) — confirmed empirically, not assumed.

### Run 2 — `paladin-llm`, default features

```
cargo semver-checks check-release --package paladin-llm --default-features --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-llm v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.032s] 196 checks: 196 pass, 58 skip
     Summary no semver update required
```

**Result: 196 checks evaluated, 0 fail.** No lint fired for `Commissary::new`/`Commissary::from_port` losing their `is_exact_counter: bool` parameter (D-08), despite that being a genuine, deliberate breaking change to a published signature. This was investigated rather than accepted at face value: `cargo semver-checks --list` (254 lints total, saved verbatim to the scratch directory) contains `function_parameter_count_changed` (major) for **free/module-level** `pub fn` only, and separately fifteen `inherent_method_*` lints (`inherent_method_added`, `..._missing`, `..._changed_abi`, `..._const_removed`, `..._generic_type_reordered`, `..._const_generic_reordered`, `..._must_use_added/removed`, `..._no_longer_unsafe`, `..._no_longer_unwind`/`..._now_unwind`, `..._now_const`, `..._now_doc_hidden`, `..._now_returns_unit`, `..._unsafe_added`) — **none of which checks an inherent/associated function's parameter count**. `Commissary::new` and `Commissary::from_port` are associated functions inside `impl Commissary { ... }` (inherent, not a free function), so `function_parameter_count_changed` does not apply to them, `inherent_method_missing` does not fire because the methods still exist under their prior names, and no other lint in the 0.50.0 catalog covers "an inherent method's parameter count changed". **This is a genuine, empirically-confirmed tool coverage gap for this specific breaking-change shape, not a guessed absence and not a suppressed occurrence** (the llm crate has no lints table at all — nothing to suppress with). Per this plan's own EDGE(PRIM-05/empty) rule, a pair with zero fired lints gets zero allowlist entries and zero MIGRATION.md §9.2 row; the constructor break is still recorded in CHANGELOG.md's `[0.10.0]` Changed bullet (D-16), independent of semver-checks output.

### Run 3 — `paladin-memory`, default features

```
cargo semver-checks check-release --package paladin-memory --default-features --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-memory v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.070s] 196 checks: 196 pass, 58 skip
     Summary no semver update required
```

**Result: 196 checks evaluated, 0 fail.** Expected and confirmed: `content-processing` (which gates the legacy `TokenCounter`/`TokenCounterFactory` removal) is not part of `paladin-memory`'s `default = []` feature set, so this default-features run cannot observe PRIM-03's removal at all — exactly Pitfall 3's documented blind spot, closed by Run 5 below.

### Run 4 — `paladin-ai` (facade), default features

```
cargo semver-checks check-release --package paladin-ai --default-features --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-ai v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.164s] 195 checks: 195 pass, 59 skip
     Summary no semver update required
```

**Result: 195 checks evaluated, 0 fail.** Same reasoning as Run 3 — `content-processing` is not in the facade's `default = ["llm-openai", "llm-anthropic", "llm-deepseek"]`, so nothing from PRIM-03's removal is visible here by design.

### Run 5 — `paladin-memory`, `--features content-processing`

```
cargo semver-checks check-release --package paladin-memory --features content-processing --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-memory v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.057s] 196 checks: 194 pass, 2 fail, 0 warn, 58 skip

--- failure struct_missing: pub struct removed or renamed ---

Failed in:
  struct paladin_memory::garrison::token_counter::TokenCounterFactory, previously in file .../paladin-memory-0.9.0/src/garrison/token_counter.rs:136
  struct paladin_memory::garrison::TokenCounterFactory, previously in file .../paladin-memory-0.9.0/src/garrison/token_counter.rs:136
  struct paladin_memory::prelude::TokenCounterFactory, previously in file .../paladin-memory-0.9.0/src/garrison/token_counter.rs:136

--- failure trait_missing: pub trait removed or renamed ---

Failed in:
  trait paladin_memory::garrison::token_counter::TokenCounter, previously in file .../paladin-memory-0.9.0/src/garrison/token_counter.rs:12
  trait paladin_memory::garrison::TokenCounter, previously in file .../paladin-memory-0.9.0/src/garrison/token_counter.rs:12
  trait paladin_memory::prelude::TokenCounter, previously in file .../paladin-memory-0.9.0/src/garrison/token_counter.rs:12

     Summary semver requires new major version: 2 major and 0 minor checks failed
```

**Result: 196 checks evaluated, 2 lints fired.** `struct_missing` on `TokenCounterFactory` (3 import paths: `garrison::token_counter::TokenCounterFactory`, `garrison::TokenCounterFactory`, `prelude::TokenCounterFactory`) and `trait_missing` on `TokenCounter` (same 3-path pattern for `garrison::token_counter::TokenCounter`, `garrison::TokenCounter`, `prelude::TokenCounter`) — exactly D-14's expected pair for the legacy counting trait and its factory, confirmed empirically rather than assumed. This is the evidence Pitfall 3 requires: the default-features run (Run 3) is silent on this removal; only the feature-enabled run reveals it.

### Run 6 — `paladin-ai` (facade), `--features content-processing` — required a command adaptation (Rule 3, blocking)

**First attempt, exactly as specified in `32-CONTEXT.md`'s `<specifics>` block:**

```
cargo semver-checks check-release --package paladin-ai --features content-processing --baseline-version 0.9.0 --release-type minor
```

This **failed to build** (exit 101), unrelated to any Phase 32 change:

```
    Checking paladin-memory v0.10.0 (/workspace/.claude/worktrees/agent-a53842d5f0389b195/crates/paladin-memory)
error[E0063]: missing field `memory` in initializer of `VectorParams`
   --> crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117:57
    |
117 | ...                   config: Some(Config::Params(VectorParams {
    |                                                   ^^^^^^^^^^^^ missing `memory`

error: could not compile `paladin-memory` (lib) due to 1 previous error
error: failed to build rustdoc for crate paladin-ai v0.10.0
```

**Root cause, diagnosed rather than assumed:** `cargo semver-checks check-release --package <p> --features <name>` — with neither `--default-features` nor `--only-explicit-features` — applies the tool's documented heuristic ("enable all features except unstable/nightly/bench/no_std and `_`-prefixed ones"), confirmed via the tool's own printed repro command, which listed literally every feature `paladin-ai` defines, including `qdrant`. Cargo-semver-checks builds the "current" side in an isolated scratch crate under `target/semver-checks/local-paladin_ai-.../` with `[workspace] members = []` and **no copy of the repo's committed `Cargo.lock`** (confirmed by inspecting that scratch directory's own freshly-generated `Cargo.lock`), so it re-resolves dependencies from scratch. `qdrant-client = { version = "1.14" }` (root `Cargo.toml:45`) is satisfied by both the workspace-pinned `1.18.0` (in the real `Cargo.lock`, confirmed via `grep -A2 'name = "qdrant-client"' Cargo.lock`) and the newer `1.19.0` (which the scratch resolution picked, confirmed via the scratch `Cargo.lock`). `qdrant-client` `1.19.0` added a required `memory` field to `VectorParams` that `1.18.0` does not have, and `crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117` (pre-existing code, untouched by any Phase 32 plan) only sets the fields `1.18.0` requires — so the scratch build's independent dependency resolution surfaces a genuine, pre-existing incompatibility between the workspace's pinned `qdrant-client` version and the newest one on crates.io, entirely orthogonal to token counting. Confirmed pre-existing and unrelated: `cargo check -p paladin-memory --all-features` against the real, locked workspace (using `qdrant-client 1.18.0`) succeeds cleanly.

**Fix (Rule 3 — blocking, scratch-tool-invocation only, no source or manifest file touched):** re-ran with `--only-explicit-features` (which disables the "enable everything" heuristic) plus explicit `--features content-processing --features default`, since `content-processing` (`Cargo.toml:515`) does not itself require `qdrant`, and the facade's `default = ["llm-openai", "llm-anthropic", "llm-deepseek"]` needed to be requested explicitly once the heuristic was turned off:

```
cargo semver-checks check-release --package paladin-ai --only-explicit-features --features content-processing --features default --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-ai v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.287s] 195 checks: 195 pass, 59 skip
     Summary no semver update required
```

**Result: 195 checks evaluated, 0 fail.** No lint fired for the facade's own re-export chain (`src/infrastructure/adapters/garrison/mod.rs`'s `token_counter` compat sub-module and its top-level re-export, both narrowed to `TiktokenCounter` in plan 32-03). Expected reasoning recorded per the task's own instruction ("where a lint you expected does NOT fire, record that too with the likely reason"): by the time this plan runs, plan 32-03 has already removed `TokenCounter`/`TokenCounterFactory` from every re-export site including the facade's, so there is no residual removal for this run to observe at the `paladin-ai` crate boundary specifically — the break was already fully captured at its origin crate (`paladin-memory`, Run 5). Whether cargo-semver-checks would even have followed the three-hop re-export chain (facade → `infrastructure::adapters::garrison` → `paladin-memory`) to attribute a break to `paladin-ai`'s own public surface is not established either way by this result, since the origin trait/struct were already gone from the facade's dependency tree by Run 6's baseline-vs-current diff time.

**Deviation logged:** [Rule 3 - Blocking] documented above; no `Cargo.toml`, `Cargo.lock`, or source file was modified — only the `cargo-semver-checks` invocation's own feature-selection flags were adapted, confined to this diagnostic command. `git status --porcelain` confirmed clean (no tracked-file changes) before this task's commit.

### Discovery summary table

| # | Package | Feature set | Checks evaluated | Fired lints | Fired-lint item paths |
|---|---|---|---|---|---|
| 1 | paladin-ports | `--default-features` | 194 | none | — |
| 2 | paladin-llm | `--default-features` | 196 | none | — (see tool-coverage-gap analysis above) |
| 3 | paladin-memory | `--default-features` | 196 | none | — (content-processing not default) |
| 4 | paladin-ai | `--default-features` | 195 | none | — (content-processing not default) |
| 5 | paladin-memory | `--features content-processing` | 196 | `struct_missing`, `trait_missing` | `TokenCounterFactory` (3 paths), `TokenCounter` (3 paths) |
| 6 | paladin-ai | `--only-explicit-features --features content-processing --features default` (adapted, see above) | 195 | none | — |

**No run in this set reports zero checks evaluated** (`0 checks: 0 pass, N skip`) — every command above carries `--release-type minor`, satisfying D-14's mandatory-flag requirement, and every run that could logically fire zero lints (Runs 1-4, 6) did so with a real, non-zero evaluated-check count, not a skipped evaluation.

**Cargo.toml/allowlist/MIGRATION.md state after Task 1:** unmodified — `git status --porcelain -- MIGRATION.md .cargo/semver-checks-allowlist.toml '*/Cargo.toml'` reports no changes; those edits are Task 2's work. No crate-level lint severity was flipped during this task (Pitfall 2's workaround was not needed — `paladin-llm` has no lints table to interact with, and `paladin-ports`'s existing `enum_marked_non_exhaustive`/`struct_marked_non_exhaustive` allows are unrelated to every lint category checked above).

---

## Task 2: Migration register, allowlist, CHANGELOG, and reader-facing docs

Writing scope was strictly bounded to what Task 1 actually observed. No lint id was written into
a row, an entry, or a Cargo.toml lints table unless a discovery run reported it.

### MIGRATION.md §9.2 — two new rows

Both new rows are `paladin-memory`, matching the two lints Run 5 (content-processing) fired:

| Crate | Type | Deliberate-breaking? |
|---|---|---|
| `paladin-memory` | `TokenCounter` | Y — `trait_missing` |
| `paladin-memory` | `TokenCounterFactory` | Y — `struct_missing` |

**No row exists for `paladin-llm | Commissary` or `paladin-ports | TokenCounterPort`** — both
crate/type pairs fired zero lints across every discovery run (Task 1, Runs 1-2, 6). Per this
plan's own EDGE(PRIM-05/empty) truth, a pair with zero fired lints gets zero rows and zero
allowlist entries; both breaks are still described in CHANGELOG.md's `[0.10.0]` section below,
independent of semver-checks detection. A note was added to MIGRATION.md immediately after the
new rows explaining this explicitly, so a reader auditing the register does not mistake the
absence for an oversight.

### `.cargo/semver-checks-allowlist.toml` — two new entries, same commit as their rows

```toml
[[entry]]
crate = "paladin-memory"
lint = "trait_missing"
migration_row = "paladin-memory | TokenCounter"
requirement_id = "PRIM-03, PRIM-05"

[[entry]]
crate = "paladin-memory"
lint = "struct_missing"
migration_row = "paladin-memory | TokenCounterFactory"
requirement_id = "PRIM-03, PRIM-05"
```

Both justifications name the `content-processing` feature gate and state plainly that CI's
`--default-features`-only `semver` job cannot observe either removal — the entry and its
justification are the only record of that gap (D-14, Pitfall 3).

### Per-crate lints tables — no changes

Neither lint fired on a **default-features** run (only on the `content-processing` run), so per
the plan's own instruction no `[package.metadata.cargo-semver-checks.lints]` line was added to
`crates/paladin-memory/Cargo.toml` — CI's default-features `semver` job never evaluates this lint
pair, so an unnecessary allow there would hide a future real (default-features) occurrence.
`crates/paladin-llm/Cargo.toml` and `crates/paladin-ports/Cargo.toml` are **unmodified** — no
lint fired for either crate on any run, default-features or content-processing. Confirmed via
`git status --porcelain -- crates/paladin-llm/Cargo.toml crates/paladin-memory/Cargo.toml
crates/paladin-ports/Cargo.toml` showing no changes to any of the three files.

### CHANGELOG.md `[0.10.0]` — three bullets

- **Changed:** `Commissary::new`/`Commissary::from_port` losing the `is_exact_counter` argument,
  plus the shared `paladin_llm::window::resolve_context_window` resolver replacing two independent
  inline precedence walks.
- **Removed:** the legacy `garrison::TokenCounter` trait and `TokenCounterFactory`, deleted
  outright with no deprecated replacement.
- **Added:** `TokenCounterPort::is_exact(&self) -> bool`.

All three link to `MIGRATION.md` §9.2; every version string in the new text reads `v0.10.0`
(confirmed: `grep -rn 'v0\.11\.0' CHANGELOG.md docs/src/api-reference/upgrading.md
docs/src/api-reference/migration-guide.md` returns nothing).

### Reader-facing docs — two new sections

`docs/src/api-reference/upgrading.md` gained `## Token primitives` directly after its existing
`## Token usage carriers` section; `docs/src/api-reference/migration-guide.md` gained
`### Token primitives` directly after its `### Token usage carriers` subsection. Both describe the
same three changes (exactness moved to the port, the legacy pair's removal, the shared resolver)
in each page's existing register and length, linking to `MIGRATION.md` §9.2 the same way their
neighboring sections do.

### Verification run in this task

- `mdbook-mermaid install docs/` (Rule 3 — the same gitignored-asset regeneration plan 32-01's
  Task 2 already documented for a fresh worktree) followed by `mdbook build docs/`: exit 0,
  `[INFO] mdbook_linkcheck] No broken links found`.
- `cargo check --workspace --all-features --all-targets`: exit 0 (all twelve crates + the facade +
  doc-examples), using the real workspace `Cargo.lock` (no qdrant-client drift — that issue was
  specific to `cargo-semver-checks`' own from-scratch placeholder-crate resolution, see Task 1).
- `grep -q '^## Token primitives' docs/src/api-reference/upgrading.md` and
  `grep -q '^### Token primitives' docs/src/api-reference/migration-guide.md`: both found.
- `! grep -rqn 'v0\.11\.0' CHANGELOG.md docs/src/api-reference/upgrading.md
  docs/src/api-reference/migration-guide.md`: confirmed, nothing found.
- `! git grep -qnE '\bTokenCounterFactory\b|garrison::TokenCounter\b' -- crates`: confirmed,
  nothing found (plan 32-03's exit grep stays clean; the new Cargo.toml comments — of which there
  are none — did not reintroduce either name).
- **The CI row-level set-equality step**, copied verbatim from `.github/workflows/ci.yml` lines
  377-445 into a scratch script and run locally from the worktree root:

```
MIGRATION.md section 9.2 deliberate-breaking crate|type pairs:
paladin-ai | Settings
paladin-ai-core | BattalionError
paladin-ai-core | GarrisonEntry
paladin-ai-core | PaladinError
paladin-ai-core | PaladinResult
paladin-ai-core | StopReason
paladin-ai-core | TokenUsage
paladin-memory | TokenCounter
paladin-memory | TokenCounterFactory
paladin-ports | ChunkMetadata
paladin-ports | LlmError
paladin-ports | LlmRequest
paladin-ports | StreamingResponse
paladin-web | ExecuteResponse
paladin-web | require_authentication
Allowlist crate|type pairs:
paladin-ai | Settings
paladin-ai-core | BattalionError
paladin-ai-core | GarrisonEntry
paladin-ai-core | PaladinError
paladin-ai-core | PaladinResult
paladin-ai-core | StopReason
paladin-ai-core | TokenUsage
paladin-memory | TokenCounter
paladin-memory | TokenCounterFactory
paladin-ports | ChunkMetadata
paladin-ports | LlmError
paladin-ports | LlmRequest
paladin-ports | StreamingResponse
paladin-web | ExecuteResponse
paladin-web | require_authentication
Allowlist is set-equal to the MIGRATION.md section 9.2 deliberate-breaking register (crate|type pairs).
```

**Exits 0, set-equal in both directions.** The two new `paladin-memory | TokenCounter` /
`paladin-memory | TokenCounterFactory` pairs appear in both lists; every pre-existing pair from
Phases 25-31 is unchanged.

### Deviations recorded in this task

**[Rule 3 - Blocking] Regenerated missing mdbook-mermaid gitignored assets before `mdbook build`**
— identical root cause and fix to plan 32-01's Task 2 deviation (a fresh worktree has no
`docs/mermaid.min.js`/`docs/mermaid-init.js`, both gitignored, regenerated via
`mdbook-mermaid install docs/`). No tracked file changed.

---

## Task 3: PRIM-05 phase gate evidence

Every command below was actually run in this worktree; nothing here is a claimed-but-unrun result.

### `make clean-code` — GREEN

`fmt` + `clippy --workspace --all-targets --all-features -- -D warnings` + `lint-shell` + `cargo check --workspace --all-targets`, all exit 0. `✅ shellcheck clean`. No warnings emitted by any of the four sub-targets.

### `cargo test --workspace --all-features --no-fail-fast` — GREEN modulo one known pre-existing failure

**6771 passed, 1 failed** (summed from every `test result:` line in the run, including the doctests this invocation folds in). The one failure is `-p paladin-ai --test cli_isolation`'s
`test_cli_feature_is_not_default` — confirmed, by reading its panic message and test name, as
the exact pre-existing `--all-features`-forces-`cli`-on conflict `deferred-items.md` already
logs under "Plan 32-04, verification" and PROJECT.md's Phase 31 close-out already carries. Not
introduced by this plan; this plan's own two commits (Task 1, Task 2) touch no file in
`src/application/cli/` or any `cli`-feature-gated path.

### Doctests, explicit — GREEN

- `cargo test -p paladin-ports --doc`: **137 passed, 0 failed, 94 ignored** — includes
  `output::token_counter_port::TokenCounterPort (line 53)`, the counting port's `is_exact`
  doc-tested default this phase added (32-01).
- `cargo test -p paladin-ai --doc`: **147 passed, 0 failed, 18 ignored**.
- `crates/paladin-llm/Cargo.toml:16` and `crates/paladin-memory/Cargo.toml:16` both carry
  `[lib] doctest = false` (confirmed by direct read) — a `--doc` run for either crate would
  select nothing by design, not report a vacuous pass; neither command was run standalone for
  that reason (their doc-tested behavior, where it exists, is proven through the `#[test]` suite
  instead, per D-14/RESEARCH.md's own framing for this workspace's `doctest = false` crates).

### `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` — RED, pre-existing, unrelated, not fixed

This gate is honestly recorded RED, matching Phase 31's own precedent for this exact command
(`31-07-SUMMARY.md`, "One gate is honestly red"). The build fails at the very first crate in
build order, `paladin-ai-core`, with 14 unresolved-intra-doc-link errors, so `cargo doc` never
proceeds to the other eleven crates in this run:

```
error: unresolved link to `StateNode::run`
error: unresolved link to `extract_json`
error: unresolved link to `TraceEvent`               (x2, different call sites)
error: unresolved link to `TraceRecord`               (x2)
error: unresolved link to `TraceEvent::DeltaMerged`
error: unresolved link to `FieldChange`
error: unresolved link to `FieldChange::value`
error: unresolved link to `TraceEvent::NodeProgress`  (x2)
error: unresolved link to `TraceEvent::ParleyRaised`  (x2)
error: unresolved link to `WebhookDelivery`
error: unresolved link to `WEBHOOK_DELIVERY_SCHEMA_VERSION`
error: could not document `paladin-ai-core`
```

Every one of these 14 errors falls into one of the two families `31-07-SUMMARY.md` already named
among its own 77-warning pre-existing baseline: **graph fingerprinting** (`StateNode::run`,
`extract_json`, `TraceEvent`/`TraceRecord`/`FieldChange`/`DeltaMerged`/`NodeProgress`/
`ParleyRaised`) and **webhook delivery docs** (`WebhookDelivery`,
`WEBHOOK_DELIVERY_SCHEMA_VERSION`). None references any symbol this plan or any other Phase 32
plan touched — confirmed: `grep -iE "TokenCounter|Commissary|is_exact|WindowSource|WindowFallbackPolicy|resolve_context_window|ResolvedWindow|UnknownContextWindow|HistoryTrimmer"` against the captured
output returns nothing. Because this run fails at the first crate, its 14-error count is a
strict subset of Phase 31's own fuller 77-warning measurement (which reached every crate before
counting), not a contradiction of it — the same pre-existing gap, observed from a narrower
vantage point.

**Not fixed, following Phase 31's own explicit reasoning for this identical gate:** fixing these
links touches `paladin-core`'s `graph_doc`/`trace`/webhook-delivery-adjacent modules, entirely
outside this plan's declared `files_modified` (`MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`,
three `Cargo.toml` files, `CHANGELOG.md`, two docs pages) and outside every Phase 32 plan's scope.
The task's own "fix red gates rather than deferring" instruction governs gates THIS phase's own
changes turned red — Phase 31 already established this reading for this exact command, and it
applies unchanged here since nothing about the failure changed.

### `mdbook build docs/` — GREEN

Exit 0, `[INFO] mdbook_linkcheck] No broken links found`. Required `mdbook-mermaid install docs/`
first (Rule 3 — the fresh-worktree gitignored-asset regeneration already documented in Task 2 and
plan 32-01's Task 2; no tracked file changed).

### `make security` — GREEN

`advisories ok, bans ok, licenses ok, sources ok`. Exit 0. Two informational `warning[yanked]`
notices (the `spin 0.9.8` crate, transitively pulled by `flume`/`lazy_static` via `sqlx-sqlite`
and several other paths) are pre-existing, pre-allowlisted, and unrelated to this plan's text-only
changes.

**Manual credential-handling review verdict (CLAUDE.md's primary control, since no Rust SAST
gates a merge):** this plan touches no API key, no external response body, and no outbound HTTP
client — every file this plan modifies (`MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`,
`CHANGELOG.md`, two docs pages) is prose/TOML documentation with zero executable Rust. The
review's three checks (redaction-before-truncation, no credential interpolated into a log/Debug
output, no credential-header-carrying client following redirects) are not engaged — stated
explicitly rather than left unmentioned.

### Coverage — GREEN, measured locally, reproducing the CI-equivalent figure

`redis:6379` and `minio:9000` (the devcontainer's own compose-network hostnames) were reachable
directly — `docker` the CLI binary is absent from this worktree's `PATH`, but the services
themselves were already up and reachable, so `make services-up` was not needed. `bash
scripts/coverage.sh` (== `make coverage`, the CI-equivalent path — `--features
integration-tests,llm-all`, real services, `--fail-under-lines 82`) ran to completion and exited
0. `cargo llvm-cov report --summary-only` against the resulting `lcov.info`:

```
TOTAL   46482   4530   90.25%   3859   622   83.88%   32397   3167   90.22%   0   0   -
```

**90.25% line coverage, comfortably clearing the 82% ADR-0006 floor** (+8.25pp), in the same
range as Phase 31's own figures (90.30% direct / 90.17% via `make coverage`) — this plan touched
no executable Rust, so no material coverage delta was expected or observed. **This is the answer
to the folded coverage todo for this plan: the local run reproduced a real, measured figure — not
an unqualified claim of reproduction — via the real, CI-equivalent script against real services.**
The todo (`2026-08-13-verify-local-coverage-reproduction.md`) keeps its pending status per its own
text; this is a verification note, not a resolution.

### The six discovery runs and the row-level set-equality result

Referenced, not repeated — see Task 1 (six discovery runs, tool version `0.50.0`) and Task 2 (the
row-level set-equality script copied verbatim from `ci.yml` lines 377-445, exit 0, set-equal in
both directions).

### Final exit grep — re-run with the two migration pages excluded, empty

```
git grep -nE '\bTokenCounterFactory\b|garrison::TokenCounter\b|is_exact_counter' -- crates src docs/src examples benches tests ':!docs/src/api-reference/upgrading.md' ':!docs/src/api-reference/migration-guide.md'
```

Exit 1 (no matches — clean). **Exclusion confirmed meaningful, not vacuous:** the same grep
against ONLY the two excluded pages finds all three names (`is_exact_counter` and
`garrison::TokenCounter` in both pages, `TokenCounterFactory` in both) — the pages correctly
document the removed names, and the exclusion is what lets the exit grep stay clean elsewhere
while those pages do their job.

### Requirement coverage

| Requirement | Delivered by | Proven by |
|---|---|---|
| PRIM-01 (`is_exact` defaults to `false`, both adapters answer correctly) | Plan 32-01 | `cargo test -p paladin-ports --doc token_counter_port` (this run: 137 passed, includes the `is_exact` doc test); `cargo test -p paladin-memory --features content-processing --lib is_exact` (32-01-SUMMARY.md D2) |
| PRIM-02 (`Commissary::new`/`from_port` drop the exactness argument) | Plan 32-01 | `cargo test -p paladin-llm --lib commissary` (32-01-SUMMARY.md D3, 32-04-SUMMARY.md D1: 20/20) |
| PRIM-03 (legacy `TokenCounter`/`TokenCounterFactory` removed) | Plan 32-03 | This plan's Task 1 Run 5 (`struct_missing`/`trait_missing`, empirically fired); this plan's Task 3 final exit grep (empty) |
| PRIM-04 (one shared context-window resolver) | Plans 32-02, 32-04 | `cargo test -p paladin-llm --lib window` (32-02-SUMMARY.md D2/D3: 9/9; 32-04-SUMMARY.md D4: 14/14 after rewiring); `cargo test -p paladin-ai --lib history` (32-04-SUMMARY.md D3: 25/25) |
| PRIM-05 (release bookkeeping: register, allowlist, CHANGELOG, docs, gate evidence) | This plan (32-05) | This SUMMARY in full — six discovery runs (Task 1), the row-level set-equality re-run (Task 2), and every gate above (Task 3) |

## Files Created/Modified

- `MIGRATION.md` - two new §9.2 rows (`paladin-memory | TokenCounter`, `paladin-memory | TokenCounterFactory`) plus a note explaining why `paladin-llm | Commissary` and `paladin-ports | TokenCounterPort` have no row
- `.cargo/semver-checks-allowlist.toml` - two new `[[entry]]` blocks matching the two new rows, each justification naming the `content-processing` feature gate
- `CHANGELOG.md` - one Changed, one Removed (new subsection) and one Added bullet in `[0.10.0]`
- `docs/src/api-reference/upgrading.md` - new `## Token primitives` section
- `docs/src/api-reference/migration-guide.md` - new `### Token primitives` subsection
- `.planning/phases/32-unified-token-primitives/32-05-SUMMARY.md` - this file, built incrementally across all three tasks

## Decisions Made

See the frontmatter `key-decisions` block above — the two central calls are (1) writing zero rows/entries for `paladin-llm | Commissary` and `paladin-ports | TokenCounterPort` because empirical discovery fired nothing for either, overriding the plan's own "expected" framing, and (2) recording the pre-existing `cargo doc -D warnings` gate honestly RED rather than fixing files outside this plan's scope, following Phase 31's own explicit precedent for the identical command.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adapted the paladin-ai content-processing discovery command to avoid an unrelated dependency-resolution drift**
- **Found during:** Task 1, Run 6
- **Issue:** The exact command CONTEXT.md's `<specifics>` block specifies (`cargo semver-checks check-release --package paladin-ai --features content-processing --baseline-version 0.9.0 --release-type minor`) failed to build — `cargo-semver-checks`' own from-scratch placeholder-crate resolution picked `qdrant-client 1.19.0` (vs. the workspace's locked `1.18.0`), and `1.19.0` added a required `memory` field to `VectorParams` that pre-existing code in `crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117` (untouched by any Phase 32 plan) does not set.
- **Fix:** Re-ran with `--only-explicit-features --features content-processing --features default`, which disables the tool's "enable everything" heuristic that pulled in the unrelated `qdrant` feature. No source or manifest file was touched — only the diagnostic tool invocation's own flags.
- **Files modified:** none (tool invocation only)
- **Verification:** the adapted command completed cleanly (195 checks, 0 fail); `git status --porcelain` confirmed no tracked-file changes both before and after.
- **Committed in:** `d3944c6d` (Task 1 commit)

**2. [Rule 3 - Blocking] Regenerated missing mdbook-mermaid gitignored assets (Tasks 2 and 3)**
- **Found during:** Task 2's `mdbook build docs/` verify step; recurred identically in Task 3
- **Issue:** Same root cause plan 32-01's Task 2 already documented — a fresh worktree has no `docs/mermaid.min.js`/`docs/mermaid-init.js` (both gitignored, regenerated at build time).
- **Fix:** `mdbook-mermaid install docs/` (documented, idempotent, already-installed-tool local asset regeneration — not a package install).
- **Files modified:** none tracked (gitignored files only).
- **Verification:** `mdbook build docs/` then exits 0, `No broken links found`.
- **Committed in:** n/a (gitignored files, not committed)

---

**Total deviations:** 2 auto-fixed (both Rule 3, blocking). No scope creep — one adapts a diagnostic tool invocation to route around an unrelated dependency-drift bug in the tool's own resolution mechanism (no source touched); the other regenerates a documented, gitignored build artifact required by every fresh checkout/worktree.

## Issues Encountered

The one substantive open question this plan resolved by investigation rather than by assumption: **why did the `Commissary::new`/`from_port` constructor-arity break (D-08) fire zero `cargo-semver-checks` lints?** Resolved definitively via `cargo semver-checks --list` (the tool's own 254-lint catalog): `function_parameter_count_changed` exists only for free/module-level `pub fn`, and none of the fifteen `inherent_method_*` lints in the 0.50.0 catalog covers an inherent/associated function's parameter count. This is a genuine tool coverage gap for this specific breaking-change shape, documented in `MIGRATION.md`'s note and this SUMMARY rather than papered over with an invented lint id.

## Known Stubs

None — this plan touches no executable Rust and wires no UI; every change is release-bookkeeping prose/TOML (MIGRATION.md rows, allowlist entries, CHANGELOG bullets, two docs sections).

## Threat Flags

None — this plan's threat model (T-32-17 through T-32-21, T-32-SC) covers exactly the surface this plan touches (semver discovery repudiation, feature-gated-break tampering, register/allowlist set-equality, information disclosure in the new text, and the coverage-gate-reported-without-being-run risk); no new surface outside that register was introduced. The `qdrant-client` version-drift finding (Task 1, Run 6) is a `cargo-semver-checks` tooling artifact confined to that tool's own from-scratch dependency resolution — it does not affect the real workspace build (`cargo check --workspace --all-features --all-targets` against the real, locked `Cargo.lock` passed cleanly) and introduces no new threat surface in shipped code.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- PRIM-05 is complete: every break this phase shipped that a `cargo semver-checks` lint actually fired for has a `MIGRATION.md` §9.2 row and a matching allowlist entry; the row-level gate is green in both directions; the CHANGELOG and both reader-facing migration pages record all three changes; the full phase gate evidence is recorded honestly, including one pre-existing RED gate this plan did not introduce and correctly declined to fix.
- **Phase 32 (Unified Token Primitives) is complete** — PRIM-01 through PRIM-05 are all done, verified above with the plan and command that proves each.
- **Carried forward, not blocking:** the pre-existing `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` RED gate (14 unresolved links in `paladin-ai-core`'s graph-fingerprinting and webhook-delivery doc families) remains open, as Phase 31 already logged it; the pre-existing `cli_isolation` `--all-features` test conflict remains open, as Phases 31 and 32's own plan 32-04 already logged it; the pre-existing broken `[`HeuristicTokenCounter`]` intra-doc link plan 32-03 logged in `deferred-items.md` also remains open (a narrower, different gate than the one measured here). None of these three items is new, and none blocks PRIM-05's own success criteria.
- No blockers.

## Self-Check: PASSED

All files listed under `key-files.modified` in the frontmatter are confirmed present and modified
on disk (`git status --porcelain` across all three commits shows exactly `MIGRATION.md`,
`.cargo/semver-checks-allowlist.toml`, `CHANGELOG.md`, `docs/src/api-reference/upgrading.md`,
`docs/src/api-reference/migration-guide.md`, and this SUMMARY — no `Cargo.toml` file, matching the
empirical finding that no lint fired on any default-features run). All three task commit hashes
(`d3944c6d`, `54598f1a`, and this SUMMARY's own closing commit) are confirmed present via
`git log --oneline -5` on branch `worktree-agent-a53842d5f0389b195`.
