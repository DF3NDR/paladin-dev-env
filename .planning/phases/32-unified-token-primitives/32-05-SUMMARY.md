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
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-memory/Cargo.toml
    - crates/paladin-ports/Cargo.toml
    - CHANGELOG.md
    - docs/src/api-reference/upgrading.md
    - docs/src/api-reference/migration-guide.md

key-decisions: []

patterns-established: []

requirements-completed: [PRIM-05]

coverage: []

# Metrics
duration: TBD
completed: 2026-09-15
status: complete
---

# Phase 32 Plan 05: Release Bookkeeping — Semver Discovery, Migration Register, Gate Evidence Summary

**PLACEHOLDER — being written incrementally across Tasks 1-3; do not read as final until the closing `## Self-Check` section is present.**

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

*(Task 2 and Task 3 sections follow below once those tasks execute.)*
