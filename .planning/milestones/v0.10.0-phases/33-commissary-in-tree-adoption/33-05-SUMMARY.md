---
phase: 33-commissary-in-tree-adoption
plan: 05
subsystem: release-hygiene
tags: [rust, semver, migration-register, changelog, api-surface, cargo-semver-checks, rag, commissary]

# Dependency graph
requires:
  - phase: 33-commissary-in-tree-adoption
    plan: "03"
    provides: "ration_respects_budget_and_rank_order proptest and the four named edge tests pinning D-05/D-08/D-10(a)/the budget boundary"
  - phase: 33-commissary-in-tree-adoption
    plan: "04"
    provides: "tests/integration/rag_commissary_test.rs (F4 evidence), both D-19 exit greps empty, Commissary module doc naming RAG as its first production caller"
provides:
  - "Six empirically-derived cargo-semver-checks 0.50.0 discovery runs (all --release-type minor), covering paladin-memory/paladin-ai default-features and content-processing plus paladin-llm default-features, confirming zero fired lints for this phase's RagRetrievalService/retrieve_context_with_timeout return-type break"
  - "MIGRATION.md §9.2 rows for paladin-memory | RagRetrievalService and paladin-memory | retrieve_context_with_timeout, both N/A migration guidance with no allowlist mirror (the tool has no lint for an inherent method's or free function's return-type or parameter-type change)"
  - "CHANGELOG.md [0.10.0] Behavioral changes / Changed / Added entries for the RAG rationing break, the paladin-memory -> paladin-llm dependency edge, and the folded-in Phase 32 facade re-export bullets ([Unreleased] header deleted)"
  - "Regenerated .project/current-exports.txt carrying RagRetrievalResult/RagRetrievalError/ShedItem/rag_omission_marker under application::services::sanctum, zero drift confirmed by make api-surface"
affects: [33-06-audit]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Empirical semver-lint discovery, never guessed: six recorded runs, exact commands and tool version pinned, every lint id traced to captured output — repeats the Phase 32 template exactly"
    - "cargo-semver-checks' --features <name> (without --default-features or --only-explicit-features) triggers an 'enable everything' heuristic that pulls in the unrelated qdrant feature and surfaces a pre-existing qdrant-client 1.18.0-vs-1.19.0 dependency-resolution drift in the tool's own from-scratch placeholder-crate build — same root cause and same --only-explicit-features workaround as 32-05"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "No MIGRATION.md allowlist entry was written for paladin-memory | RagRetrievalService or paladin-memory | retrieve_context_with_timeout -- both fired zero cargo-semver-checks 0.50.0 lints across every discovery run (all six), confirmed by inspecting the tool's full 254-lint catalog: no lint in the 0.50.0 catalog checks an inherent method's or a free function's return-type change (only the unit-type special case, function_now_returns_unit/inherent_method_now_returns_unit), and no lint checks a parameter's type as opposed to its count. This is the identical tool-coverage gap Phase 32 documented for Commissary::new/from_port's parameter-count break, now empirically confirmed to extend to return-type and parameter-type changes too. Both rows are written as N/A migration guidance, mirroring the paladin-llm | Commissary row's template exactly."
  - "No crates/paladin-memory/Cargo.toml or root Cargo.toml lints table was created or extended -- no lint fired on any run for this phase's RAG breaks, so none needed a per-crate suppression."
  - "The CHANGELOG's [Unreleased] header was deleted and its two Added bullets (Commissary facade re-exports, window-resolver facade re-exports, both Phase 32) folded into [0.10.0]'s Added block, per the orchestrator-approved RESEARCH Pitfall 4 resolution named in the plan -- Phase 29 D-18/D-21 already bumped 0.10.0 untagged on this branch, so nothing should sit above it mid-cycle."
  - "The Behavioral changes section intro was updated from 'Four' to 'Five' with a clarifying clause, since the new RAG bullet has no MIGRATION.md §9.1 worked-example row (it results from the already-registered §9.2 signature break, not a config-driven behavior toggle) -- the intro's original claim that every item is 'detailed... in MIGRATION.md §9.1' would otherwise become false."

patterns-established:
  - "Pattern: a signature break whose §9.2 row is N/A (zero fired lints) can still carry a Behavioral changes CHANGELOG bullet when the break also changes runtime behavior -- the bullet points at the §9.2 row and a docs page for the worked example instead of a §9.1 table row, and the section's own intro sentence is corrected to say so rather than silently overclaiming full §9.1 coverage."

requirements-completed: [COMM-04]

coverage:
  - id: D1
    description: "Six empirical cargo-semver-checks 0.50.0 discovery runs (four --default-features, two --features content-processing shapes across paladin-memory/paladin-ai, plus paladin-llm --default-features), all carrying --release-type minor; none report zero checks evaluated. Zero lints fired for this phase's RagRetrievalService/retrieve_context_with_timeout return-type break across all six runs -- confirmed as a genuine cargo-semver-checks 0.50.0 coverage gap (no lint checks an inherent method's or free function's return-type or parameter-type change), not a guessed or suppressed absence."
    requirement: "COMM-04"
    verification:
      - kind: other
        ref: "33-05-SUMMARY.md Task 1 -- six captured discovery run transcripts plus the cargo semver-checks --list catalog inspection"
        status: pass
    human_judgment: false
  - id: D2
    description: "MIGRATION.md gains two new section 9.2 rows (paladin-memory | RagRetrievalService, paladin-memory | retrieve_context_with_timeout), both N/A with no allowlist mirror; make check-migration-allowlist and make check-gates both exit 0, set-equal in both directions; grep -c TBD MIGRATION.md is 0."
    requirement: "COMM-04"
    verification:
      - kind: other
        ref: "33-05-SUMMARY.md Task 2 -- make check-migration-allowlist and make check-gates output, both set-equal"
        status: pass
    human_judgment: false
  - id: D3
    description: "CHANGELOG.md [0.10.0] gains one Behavioral changes bullet, one Changed bullet and one Added line for this phase's RAG rationing break and dependency edge; the [Unreleased] header is deleted with its two Added bullets folded into [0.10.0]; the Phase 31 TokenUsage and Phase 32 Commissary entries are verified present by grep, not rewritten; every new version string reads v0.10.0."
    requirement: "COMM-04"
    verification:
      - kind: other
        ref: "grep -c '^## \\[Unreleased\\]' CHANGELOG.md == 0; awk-scoped [0.10.0] greps for rag/TokenUsage/Commissary all non-zero; grep -c 'v0.11.0' CHANGELOG.md MIGRATION.md == 0 for both"
        status: pass
    human_judgment: false
  - id: D4
    description: ".project/current-exports.txt is regenerated via make api-surface-update in the same commit as the CHANGELOG edit, carries RagRetrievalResult/RagRetrievalError/ShedItem/rag_omission_marker under application::services::sanctum, and make api-surface exits 0 (zero drift)."
    requirement: "COMM-04"
    verification:
      - kind: other
        ref: "make api-surface-update (3959 items) followed by make api-surface -- 'API surface unchanged'"
        status: pass
    human_judgment: false

# Metrics
duration: ~50min
completed: 2026-09-16
status: complete
---

# Phase 33 Plan 05: Release Bookkeeping — Semver Discovery, Migration Register, CHANGELOG and API Baseline Summary

**Six empirical `cargo-semver-checks` discovery runs confirm zero fired lints for the RAG retrieval API break — mirroring Phase 32's `Commissary` tool-coverage gap exactly — feeding two `N/A` MIGRATION.md §9.2 rows, three CHANGELOG `[0.10.0]` entries (with the stale `[Unreleased]` header folded in and deleted), and a regenerated, drift-free `.project/current-exports.txt`.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-09-16T17:02Z (approximate — Task 1's first discovery run)
- **Completed:** 2026-09-16T17:52Z
- **Tasks:** 3
- **Files modified:** 3 (`MIGRATION.md`, `CHANGELOG.md`, `.project/current-exports.txt` — no `.cargo/semver-checks-allowlist.toml` or `Cargo.toml` change, since no lint fired on any run)

## Accomplishments

- Six real `cargo-semver-checks 0.50.0` discovery runs recorded verbatim below, every command carrying `--release-type minor`, none reporting zero checks evaluated.
- Diagnosed and worked around the same `qdrant-client` 1.18.0-vs-1.19.0 dependency-resolution drift in `cargo-semver-checks`' own from-scratch placeholder-crate build that Phase 32 hit (Rule 3), using the identical `--only-explicit-features` fix.
- Confirmed, via the tool's own 254-lint catalog, that `cargo-semver-checks 0.50.0` has no lint covering an inherent method's or free function's return-type change (beyond the unit-type special case) or a parameter's type (as opposed to count) change — explaining, not guessing, why `RagRetrievalService`'s return-type break and `format_for_prompt`'s parameter-type break produced zero tool output across all six runs.
- Two new `MIGRATION.md` §9.2 rows for `paladin-memory | RagRetrievalService` and `paladin-memory | retrieve_context_with_timeout`, both marked `N/A` with no allowlist mirror, mirroring the Phase 32 `paladin-llm | Commissary` template exactly — `make check-migration-allowlist` and `make check-gates` both exit 0, set-equal.
- Three `CHANGELOG.md` `[0.10.0]` entries (one Behavioral changes bullet, one Changed bullet, one Added line) plus the fold of Phase 32's two `[Unreleased]` `### Added` bullets into `[0.10.0]`'s own Added block and deletion of the now-empty `[Unreleased]` header.
- `.project/current-exports.txt` regenerated via `make api-surface-update` (3959 items) in the same commit as the CHANGELOG edit; `make api-surface` confirms zero drift.

## Task Commits

Each task was committed atomically:

1. **Task 1: run empirical semver discovery and record every lint that actually fires** - no code, no commit (this SUMMARY records the six runs; per the plan's own instruction, "this task writes no register file")
2. **Task 2: write the §9.2 rows, their allowlist mirrors and the per-crate suppressions** - `1b25e5d6` (docs)
3. **Task 3: CHANGELOG [0.10.0] and the regenerated public-surface baseline** - `7d38a424` (docs)

**Plan metadata:** (this commit, once created)

## Task 1: Empirical semver-lint discovery

**Tool version:** `cargo-semver-checks 0.50.0` (confirmed via `cargo semver-checks --version`, matches the CI pin and Phase 32's own recorded version).

All six runs below carry `--release-type minor` (mandatory — without it, a crate already bumped to `0.10.0` is treated as major-equivalent against the `0.9.0` baseline and the tool skips every lint).

### Run 1 — `paladin-memory`, default features

```
cargo semver-checks check-release --package paladin-memory --default-features --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-memory v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.032s] 196 checks: 196 pass, 58 skip
     Summary no semver update required
```

**Result: 196 checks evaluated, 0 fail.** No lint fired for `RagRetrievalService::retrieve_context`'s return-type break (`RagRetrievalResult`/`RagRetrievalError` replacing the v0.9.0 shape) or `format_for_prompt`'s parameter-type break.

### Run 2 — `paladin-memory`, `--features content-processing`

```
cargo semver-checks check-release --package paladin-memory --features content-processing --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-memory v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.047s] 196 checks: 194 pass, 2 fail, 0 warn, 58 skip

--- failure struct_missing: pub struct removed or renamed ---
Failed in:
  struct paladin_memory::garrison::token_counter::TokenCounterFactory, ...
  struct paladin_memory::garrison::TokenCounterFactory, ...
  struct paladin_memory::prelude::TokenCounterFactory, ...

--- failure trait_missing: pub trait removed or renamed ---
Failed in:
  trait paladin_memory::garrison::token_counter::TokenCounter, ...
  trait paladin_memory::garrison::TokenCounter, ...
  trait paladin_memory::prelude::TokenCounter, ...

     Summary semver requires new major version: 2 major and 0 minor checks failed
```

**Result: 196 checks evaluated, 2 fail — both PRE-EXISTING from Phase 32** (`TokenCounter`/`TokenCounterFactory` removal, already registered in `MIGRATION.md` §9.2 and `.cargo/semver-checks-allowlist.toml` by plan 32-05). **Zero new lints fired for any Phase 33 RAG change** in this run.

### Run 3 — `paladin-ai` (facade), default features

```
cargo semver-checks check-release --package paladin-ai --default-features --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-ai v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.129s] 195 checks: 195 pass, 59 skip
     Summary no semver update required
```

**Result: 195 checks evaluated, 0 fail.**

### Run 4 — `paladin-ai` (facade), `--features content-processing` — required the same command adaptation Phase 32 hit (Rule 3, blocking)

**First attempt, exactly as CONTEXT.md's `<specifics>` block specifies:**

```
cargo semver-checks check-release --package paladin-ai --features content-processing --baseline-version 0.9.0 --release-type minor
```

**Failed to build** (exit 101) — identical root cause to 32-05-SUMMARY.md Run 6: the tool's "enable everything except unstable/nightly/bench/no_std" heuristic (triggered because neither `--default-features` nor `--only-explicit-features` was passed) pulls in the `qdrant` feature, and `cargo-semver-checks`' own from-scratch placeholder-crate resolution picks `qdrant-client 1.19.0` (vs. the workspace's locked `1.18.0`), which added a required `memory` field to `VectorParams` that pre-existing code in `crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117` (untouched by any Phase 33 plan) does not set:

```
error[E0063]: missing field `memory` in initializer of `VectorParams`
   --> crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117:57
error: could not compile `paladin-memory` (lib) due to 1 previous error
error: failed to build rustdoc for crate paladin-ai v0.10.0
```

**Fix (Rule 3 — blocking, scratch-tool-invocation only, no source or manifest file touched):** re-ran with `--only-explicit-features --features content-processing --features default`, the identical workaround 32-05-SUMMARY.md recorded:

```
cargo semver-checks check-release --package paladin-ai --only-explicit-features --features content-processing --features default --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-ai v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.374s] 195 checks: 195 pass, 59 skip
     Summary no semver update required
```

**Result: 195 checks evaluated, 0 fail.** No `paladin-ai` row was needed per the plan's own instruction (a row is added "only if Task 1's discovery actually fired a lint for the facade re-export").

**Deviation logged:** [Rule 3 - Blocking] documented above; no `Cargo.toml`, `Cargo.lock`, or source file was modified — only the `cargo-semver-checks` invocation's own feature-selection flags were adapted. `git status --porcelain` confirmed clean (no tracked-file changes) before this task's commit.

### Run 5 — `paladin-llm`, default features (no change expected — confirmed empirically)

```
cargo semver-checks check-release --package paladin-llm --default-features --baseline-version 0.9.0 --release-type minor
```

```
    Checking paladin-llm v0.9.0 -> v0.10.0 (assume minor change)
     Checked [   0.074s] 196 checks: 196 pass, 58 skip
     Summary no semver update required
```

**Result: 196 checks evaluated, 0 fail.** This phase makes no change to `paladin-llm`'s public surface (D-04 — no new `Commissary` constructor); confirmed empirically rather than merely assumed.

### Run 6 — re-run of the one `paladin-memory` invocation that fired lints (Run 2), to confirm the result is stable

```
cargo semver-checks check-release --package paladin-memory --features content-processing --baseline-version 0.9.0 --release-type minor
```

```
    Checked [...] 196 checks: 194 pass, 2 fail, 0 warn, 58 skip
--- failure struct_missing ... TokenCounterFactory (3 paths) ---
--- failure trait_missing ... TokenCounter (3 paths) ---
     Summary semver requires new major version: 2 major and 0 minor checks failed
```

**Result: identical to Run 2** — the same two pre-existing lints, same three import paths each, confirming the result is stable and not a cache artifact.

### Discovery summary table

| # | Command | Fired lints | Verdict |
|---|---|---|---|
| 1 | `paladin-memory --default-features` | none | RAG return-type break invisible to this feature shape too — consistent with the tool-coverage gap below |
| 2 | `paladin-memory --features content-processing` | `struct_missing`, `trait_missing` | Both PRE-EXISTING (Phase 32's `TokenCounter`/`TokenCounterFactory` removal); zero new lints for Phase 33's own breaks |
| 3 | `paladin-ai --default-features` | none | — |
| 4 | `paladin-ai --only-explicit-features --features content-processing --features default` (adapted, see above) | none | — |
| 5 | `paladin-llm --default-features` | none | Confirms no change to `paladin-llm`'s surface this phase |
| 6 | `paladin-memory --features content-processing` (re-run of Run 2) | `struct_missing`, `trait_missing` | Stable, identical to Run 2 — not a cache artifact |

**No run in this set reports zero checks evaluated.** Every command carries `--release-type minor`. **Zero lints fired for this phase's own RAG breaks across all six runs** — investigated, not accepted at face value: `cargo semver-checks --list`'s full 254-lint catalog contains no lint for an inherent method's or free function's return-type change (only `function_now_returns_unit`/`inherent_method_now_returns_unit`, both scoped to the unit-type special case) and no lint for a parameter's *type* change (every `*_parameter_count_changed` lint checks count, never type). `RagRetrievalService::retrieve_context`'s return type changing from its v0.9.0 shape to `Result<RagRetrievalResult, RagRetrievalError>`, and `format_for_prompt`'s parameter changing to `&RagRetrievalResult`, are both exactly this shape of break — a genuine, empirically-confirmed tool coverage gap, the same one Phase 32 already documented for `Commissary::new`/`from_port`'s parameter-*count* break, now confirmed to extend to return-type and parameter-*type* changes as well.

---

## Task 2: MIGRATION.md §9.2 rows

Per the plan's own instruction ("a pair with zero fired lints keeps its row as migration guidance marked N/A with the tool-coverage note and add NO allowlist entry"), two new rows were added, both `N/A`:

| Crate | Type | Deliberate-breaking? |
|---|---|---|
| `paladin-memory` | `RagRetrievalService` | N/A — zero fired lints (return-type + parameter-type break has no matching lint in the 0.50.0 catalog) |
| `paladin-memory` | `retrieve_context_with_timeout` | N/A — same empirical basis; free function's return-type change also has no matching lint |

No `paladin-ai` row was added — Runs 3-4 both fired zero lints for the facade re-export. Both rows carry migration guidance (read `.memories`/`.shed` off the result; render `RagRetainedMemory::body`, never `memory.content`) and cite the six discovery runs above. A new note, mirroring Phase 32's own note for the `Commissary` row, was added immediately after the two rows, explaining the zero-fired-lint finding explicitly so a reader auditing the register does not mistake the absence for an oversight.

**No `.cargo/semver-checks-allowlist.toml` entry and no `Cargo.toml` lints-table change** were made — neither lint fired for either pair, on any run, so there is nothing to suppress. `git status --porcelain` after this task shows only `MIGRATION.md` changed.

### Verification run in this task

```
make check-migration-allowlist
```
→ 15 pairs in both the MIGRATION.md register and the allowlist, set-equal in both directions (the two new `N/A` rows correctly excluded, since only `Y` rows enter the register).

```
make check-gates
```
→ exit 0 (CHANGELOG-per-crate, package-name allow-list, advisory-exception register, workflow-suppression scan, trigger-policy table, CodeQL dismissal register, and the same row-level set-equality check above — all pass).

```
grep -c TBD MIGRATION.md
```
→ `0`.

Committed as `docs(33): register the RAG retrieval API break in MIGRATION.md §9.2` (`1b25e5d6`).

---

## Task 3: CHANGELOG `[0.10.0]` and the regenerated public-surface baseline

### `### Behavioral changes` — new bullet, intro count corrected

The section's intro previously read "Four user-visible behavior changes ship... Each is detailed... in `MIGRATION.md` §9.1." A fifth bullet was added for the RAG rationing change (truncation marker, shed record, oversized-memory-retained-truncated, pessimistic-ratio injected volume — naming `RagRetrievalResult::prompt_tokens`/`allotted_tokens`/`exact_tally`). Since this bullet has no companion §9.1 table row (it results from the already-registered §9.2 signature break, not a config-driven toggle), the intro sentence was corrected to "Five... The first four (`M-B-01`…`M-B-04`) are detailed... in §9.1; the fifth... results from the signature break registered in §9.2... with its own worked example in `docs/src/architecture/commissary.md`" — so the intro's claim stays accurate rather than silently overclaiming full §9.1 coverage.

### `### Changed` — new bullet

Added immediately after Phase 32's `Commissary::new`/`from_port` bullet: `RagRetrievalService::retrieve_context`/`retrieve_context_with_timeout` returning `RagRetrievalResult`, `format_for_prompt` taking that struct, the new `RagRetrievalError` enum, and the new `with_token_counter` builder — pointing at the two `MIGRATION.md` §9.2 rows Task 2 wrote (both `N/A`, same tool-coverage reason as the `Commissary` row immediately above it).

### `### Added` — new line, plus the `[Unreleased]` fold

Added a line for the `paladin-memory` → `paladin-llm` production dependency edge (the workspace's first unconditional production lateral adapter-to-adapter crate edge). Then, per the plan's orchestrator-approved resolution of RESEARCH Pitfall 4: the two `[Unreleased]` `### Added` bullets (Commissary facade re-exports, window-resolver facade re-exports — both Phase 32) were folded into `[0.10.0]`'s own `### Added` block immediately after the new dependency-edge line, and the now-empty `## [Unreleased]` header was deleted outright. Phase 29 D-18/D-21 already bumped `0.10.0` untagged on this branch, so nothing should sit above `[0.10.0]` mid-cycle — this is what makes COMM-04's "the `[0.10.0]` section carries the Phase 31/32 API entries" criterion literally true.

### Verification present-by-grep (Phase 31/32 entries NOT rewritten)

```
grep -c '^## \[Unreleased\]' CHANGELOG.md          → 0
awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md | grep -ci 'rag'         → 17
awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md | grep -c 'TokenUsage'   → 4
awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md | grep -c 'Commissary'  → 15
grep -c 'v0.11.0' CHANGELOG.md                     → 0
grep -c 'v0.11.0' MIGRATION.md                     → 0
```

All pass. The Phase 31 `TokenUsage` bullets and the Phase 32 `Commissary` entries are present, untouched (only new content was added around them).

### Public-surface baseline regeneration

```
make api-surface-update
```
→ `✅ API surface extracted to .project/current-exports.txt (3959 items)`.

```
grep -n "RagRetrievalResult\|RagRetrievalError\|ShedItem\|rag_omission_marker" .project/current-exports.txt
```
→ all four present under both `paladin::application::services::sanctum::*` and its `rag_retrieval_service` re-export sub-module, plus `ShedItem` at the crate root (`paladin::ShedItem`).

```
make api-surface
```
→ `✅ API surface unchanged` (zero drift against the just-regenerated baseline, confirming the CHANGELOG edit and the baseline landed in the same commit with nothing left stale).

Both files staged and committed together as `docs(33): record the RAG rationing behaviour change in CHANGELOG [0.10.0]` (`7d38a424`) — `git show --stat 7d38a424` lists both `CHANGELOG.md` and `.project/current-exports.txt`.

---

## Files Created/Modified

- `MIGRATION.md` - two new §9.2 rows (`paladin-memory | RagRetrievalService`, `paladin-memory | retrieve_context_with_timeout`), both `N/A`, plus an explanatory note mirroring the Phase 32 `Commissary` row's template
- `CHANGELOG.md` - one Behavioral changes bullet (with a corrected "Four" → "Five" intro), one Changed bullet, one Added line for the RAG rationing break; folds Phase 32's two `[Unreleased]` Added bullets into `[0.10.0]` and deletes the `[Unreleased]` header
- `.project/current-exports.txt` - regenerated public-surface baseline (3959 items), now carrying the RAG result/error/`ShedItem`/marker re-exports
- `.planning/phases/33-commissary-in-tree-adoption/33-05-SUMMARY.md` - this file

## Decisions Made

See the frontmatter `key-decisions` block above — the central calls are (1) writing zero rows'-worth of allowlist entries because empirical discovery fired nothing for either `paladin-memory` pair, confirmed via the tool's own lint catalog rather than assumed from the Phase 32 precedent alone, and (2) correcting the CHANGELOG's Behavioral-changes intro sentence from "Four" to "Five" with an explicit carve-out, rather than silently adding a fifth bullet that would make the intro's own claim ("each is detailed... in §9.1") false.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adapted the paladin-ai content-processing discovery command to avoid the same unrelated dependency-resolution drift Phase 32 hit**
- **Found during:** Task 1, Run 4
- **Issue:** The exact command CONTEXT.md's `<specifics>` block specifies (`cargo semver-checks check-release --package paladin-ai --features content-processing --baseline-version 0.9.0 --release-type minor`) failed to build — identical root cause to 32-05-SUMMARY.md's Run 6: the tool's "enable everything" heuristic pulls in the `qdrant` feature, and its own from-scratch placeholder-crate resolution picks `qdrant-client 1.19.0` (vs. the workspace's locked `1.18.0`), which requires a `memory` field `crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117` (pre-existing, untouched by any Phase 33 plan) does not set.
- **Fix:** Re-ran with `--only-explicit-features --features content-processing --features default`, the identical workaround Phase 32 already recorded. No source or manifest file was touched — only the diagnostic tool invocation's own flags.
- **Files modified:** none (tool invocation only)
- **Verification:** the adapted command completed cleanly (195 checks, 0 fail); `git status --porcelain` confirmed no tracked-file changes both before and after.
- **Committed in:** n/a (Task 1 writes no register file; the deviation is recorded here per the plan's own instruction)

---

**Total deviations:** 1 auto-fixed (Rule 3, blocking). No scope creep — it adapts a diagnostic tool invocation to route around the same unrelated, already-diagnosed dependency-drift bug in the tool's own resolution mechanism that Phase 32 hit; no source touched.

## Issues Encountered

The one substantive question this plan resolved by investigation rather than assumption: **why did `RagRetrievalService::retrieve_context`'s return-type break and `format_for_prompt`'s parameter-type break fire zero `cargo-semver-checks` lints, across every feature shape?** Resolved definitively via `cargo semver-checks --list` (the tool's own 254-lint catalog): the only return-type-change lints (`function_now_returns_unit`/`inherent_method_now_returns_unit`) are scoped to the unit-type special case, and every `*_parameter_count_changed` lint checks parameter *count*, never *type*. This is a genuine tool coverage gap for this specific breaking-change shape — the same one Phase 32 already documented for `Commissary::new`/`from_port`'s parameter-count break — now confirmed, not merely assumed by analogy, to extend to return-type and parameter-type changes as well.

## Known Stubs

None — this plan touches no executable Rust and wires no UI; every change is release-bookkeeping prose (MIGRATION.md rows, CHANGELOG bullets, the regenerated API baseline).

## Threat Flags

None — this plan's threat model (T-33-12 through T-33-14) covers exactly the surface this plan touches (allowlist-entry tampering, a behavioral change shipping without a release note, public-surface baseline drift); no new surface outside that register was introduced. The `qdrant-client` version-drift finding (Task 1, Run 4) is a `cargo-semver-checks` tooling artifact confined to that tool's own from-scratch dependency resolution — it does not affect the real workspace build and introduces no new threat surface in shipped code.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- COMM-04's register half is done: every measured break from this phase has a §9.2 row (both `N/A`, correctly reflecting zero fired lints), the `[0.10.0]` CHANGELOG section carries the RAG note plus the verified-present Phase 31/32 entries, and the API baseline is current with zero drift — plan 33-06's gate sweep can run on a tree whose paperwork already agrees with its code.
- **Carried forward, not blocking:** none newly introduced by this plan. The pre-existing `cargo doc -D warnings` red gate and any other pre-existing carried items from Phases 31/32 remain exactly as those phases recorded them; this plan touched no executable Rust and introduced no new red gate.
- No blockers.

---
*Phase: 33-commissary-in-tree-adoption*
*Completed: 2026-09-16*

## Self-Check: PASSED

- `MIGRATION.md` — FOUND, contains `paladin-memory | RagRetrievalService` and `paladin-memory | retrieve_context_with_timeout` §9.2 rows, both `N/A`; `grep -c TBD MIGRATION.md` = 0; `grep -c 'v0.11.0' MIGRATION.md` = 0
- `CHANGELOG.md` — FOUND, contains no `## [Unreleased]` header; `[0.10.0]`-scoped greps confirm `rag` (17), `TokenUsage` (4), `Commissary` (15) all present; `grep -c 'v0.11.0' CHANGELOG.md` = 0
- `.project/current-exports.txt` — FOUND, contains `RagRetrievalResult`, `RagRetrievalError`, `ShedItem`, `rag_omission_marker` under `paladin::application::services::sanctum` (both the top-level barrel and the `rag_retrieval_service` sub-module) and `ShedItem` at the crate root
- Commit `1b25e5d6` — FOUND in `git log`
- Commit `7d38a424` — FOUND in `git log`
- `make check-migration-allowlist` — exit 0, set-equal in both directions (15 pairs)
- `make check-gates` — exit 0
- `make api-surface` — exit 0, "API surface unchanged"
- Both task commits confirmed to introduce no unexpected file deletions (`git diff --diff-filter=D --name-only HEAD~1 HEAD` empty for both `1b25e5d6` and `7d38a424`)
