---
phase: 35-mdbook-currency
plan: 02
subsystem: docs
tags: [mdbook, doc-examples, paladin-builder, arsenal, herald, commander, rag, sanctum]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: "35-01's superstep-engine page, lib.rs's existing pub mod list shape, and the fault_tolerance.rs anchor convention this plan's five modules follow"
provides:
  - "MB-27, MB-19, MB-24, MB-20 and MB-28 closed — five signature-level §2 findings promoted from hand-written, non-compiling fences to compile-verified crates/doc-examples anchors"
  - "Five new crates/doc-examples modules (paladin_agents, arsenal_tools, herald_output, battalion_patterns, sanctum_vector_memory) registered in lib.rs by addition only (D-26)"
  - "paladin-memory added as a doc-examples Cargo.toml dependency (no extra feature — InMemoryGarrison/InMemorySanctum/RagRetrievalService are unconditional modules)"
affects: ["any later Phase 35 plan touching docs/src/user-guides/*.md or crates/doc-examples/src/lib.rs"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "doc-examples module per signature-level guide finding (D-12): one file, // ANCHOR: region(s), pub mod line, {{#include}} on the page — same shape as fault_tolerance.rs"
    - "Local, module-scoped mock adapters (EmbeddingPort) for ports support.rs does not cover, instead of editing support.rs (D-26)"

key-files:
  created:
    - crates/doc-examples/src/paladin_agents.rs
    - crates/doc-examples/src/arsenal_tools.rs
    - crates/doc-examples/src/herald_output.rs
    - crates/doc-examples/src/battalion_patterns.rs
    - crates/doc-examples/src/sanctum_vector_memory.rs
  modified:
    - crates/doc-examples/src/lib.rs
    - crates/doc-examples/Cargo.toml
    - docs/src/user-guides/paladin-agents.md
    - docs/src/user-guides/arsenal-tools.md
    - docs/src/user-guides/herald-output.md
    - docs/src/user-guides/battalion-patterns.md
    - docs/src/user-guides/sanctum-vector-memory.md

key-decisions:
  - "Added paladin-memory as a doc-examples Cargo.toml dependency (files_modified didn't list it, but the plan's own threat model T-35-SC names paladin-memory as one of the expected workspace path dependencies) — Rule 3 auto-fix, not a registry-package install, no Package Legitimacy Gate concern."
  - "paladin_agents.rs's attach_garrison anchor demonstrates both InMemoryGarrison::new(config) AND with_handoffs in one function (they compose naturally on the same builder chain), so the page's Memory — Garrison and Agent Handoffs sections both {{#include}} the same anchor rather than two narrower ones."
  - "sanctum_vector_memory.rs defines its own module-local MockEmbedder (EmbeddingPort) rather than adding one to support.rs, keeping D-26's additions-only boundary on the shared support module intact."
  - "rag_format's anchor calls the real RagRetrievalService::format_for_prompt (public, already appends the omission marker via the shared rag_omission_marker function) rather than hand-rolling a second renderer that could drift from it."

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "MB-27 closed: paladin-agents.md's Memory — Garrison and Agent Handoffs sections now include compile-verified anchors showing InMemoryGarrison::new(config) and with_handoffs; dependency pin corrected to 0.10.0"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "./scripts/check-doc-examples.sh (Layer 1 cargo check -p paladin-doc-examples)"
        status: pass
      - kind: other
        ref: "grep -c 'with_specialist' docs/src/user-guides/paladin-agents.md == 0; grep -cE '\"0\\.[5-9]\\.[0-9]+\"' docs/src/user-guides/paladin-agents.md == 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "MB-19 closed: arsenal-tools.md's Custom Armaments and Handoff Tool sections now include compile-verified anchors showing the five-field ArmamentResult, ArmamentCall::arguments, and with_handoffs"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "./scripts/check-doc-examples.sh; grep -q 'execution_time_ms' crates/doc-examples/src/arsenal_tools.rs"
        status: pass
    human_judgment: false
  - id: D3
    description: "MB-24 closed: herald-output.md documents and includes all seven Herald trait methods (format_paladin_result, format_battalion_result, format_stream_chunk, finalize_stream, format_error, name, mime_type)"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "grep -c 'fn format_paladin_result\\|fn format_battalion_result\\|fn format_stream_chunk\\|fn finalize_stream\\|fn format_error\\|fn name\\|fn mime_type' crates/doc-examples/src/herald_output.rs == 7"
        status: pass
    human_judgment: false
  - id: D4
    description: "MB-20 closed: battalion-patterns.md's Commander section now includes a compile-verified anchor using CommanderBuilder + single-argument execute; dependency pin corrected to 0.10.0"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "./scripts/check-doc-examples.sh; grep -c '{{#include ../../../crates/doc-examples/src/battalion_patterns.rs:commander}}' docs/src/user-guides/battalion-patterns.md == 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "MB-28 closed: sanctum-vector-memory.md's service name corrected to camelCase RagRetrievalService and the RAG section extended to document RagRetrievalResult, ShedItem, RagRetrievalError, retrieve_context_with_timeout, with_token_counter and the omission marker via two new compile-verified anchors"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "grep -c 'RAGRetrievalService' docs/src/user-guides/sanctum-vector-memory.md == 0; grep -cE '\\bTokenCounterFactory\\b|garrison::TokenCounter\\b' docs/src/user-guides/sanctum-vector-memory.md == 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "Full docs.yml gate sequence green on the plan's final commit (mdbook-mermaid install with no drift, mdbook build + linkcheck, check-doc-examples.sh, check-doc-config.sh, make api-surface unchanged)"
    requirement: "CURR-07"
    verification:
      - kind: other
        ref: "mdbook build docs/ — 'No broken links found'"
        status: pass
      - kind: other
        ref: "./scripts/check-doc-examples.sh — 0 checked/623 skipped/0 failed"
        status: pass
      - kind: other
        ref: "./scripts/check-doc-config.sh — 154 YAML blocks checked, 0 failed"
        status: pass
      - kind: other
        ref: "make api-surface — API surface unchanged"
        status: pass
    human_judgment: false

# Metrics
duration: ~25min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 02: Five Signature-Level User-Guide Fixes Summary

**Closed MB-27, MB-19, MB-24, MB-20 and MB-28 by promoting each broken constructor/method call
into a new compile-verified `crates/doc-examples` module — five new modules, five corrected
user-guide pages, one commit per page.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-09-17T13:38:00Z (approx.)
- **Completed:** 2026-09-17T13:57:20Z
- **Tasks:** 3
- **Files modified:** 12 (5 created, 7 modified)

## Accomplishments

- **MB-27 (paladin-agents.md):** new `crates/doc-examples/src/paladin_agents.rs` with
  `build_agent` (PaladinBuilder chain against a mock LLM) and `attach_garrison`
  (`InMemoryGarrison::new(config)` — the live one-argument constructor — plus `with_handoffs`
  taking the whole specialist list) anchors. Page's Memory — Garrison and Agent Handoffs
  sections now `{{#include}}` these anchors instead of a zero-argument constructor and a
  nonexistent per-call `with_specialist` chain; dependency pin corrected `0.5.0` → `0.10.0`.
- **MB-19 (arsenal-tools.md):** new `crates/doc-examples/src/arsenal_tools.rs` with
  `custom_armament` (the live five-field `ArmamentResult` including `call_id` and
  `execution_time_ms`, reading input from `ArmamentCall::arguments`, not a nonexistent `args`
  field) and `handoffs` (`with_handoffs`) anchors. Custom Armaments and Handoff Tool sections
  now include the compile-verified anchors.
- **MB-24 (herald-output.md):** new `crates/doc-examples/src/herald_output.rs` with a
  `custom_herald` anchor implementing the full seven-method `Herald` trait
  (`format_paladin_result`, `format_battalion_result`, `format_stream_chunk`,
  `finalize_stream`, `format_error`, `name`, `mime_type`) — the page previously showed only
  three. The trait listing and Custom Herald Implementation sample now agree with
  `output-formatting.md`.
- **MB-20 (battalion-patterns.md):** new `crates/doc-examples/src/battalion_patterns.rs` with
  a `commander` anchor building a `Commander` through `CommanderBuilder` (the same builder
  `orchestration.md` uses) and running it with the live single-argument `execute` method —
  the page's direct `Commander::new(paladin_port, paladin_registry)` two-argument form and
  four-argument `execute` call do not exist. Dependency pin corrected `0.5.0` → `0.10.0`.
- **MB-28 (sanctum-vector-memory.md):** new `crates/doc-examples/src/sanctum_vector_memory.rs`
  with `rag_retrieve` (builds a `RagRetrievalService` — camelCase `Rag`, not the page's stale
  `RAGRetrievalService` — over an in-memory Sanctum, injects an exact token counter via
  `with_token_counter`, calls the timeout-bounded `retrieve_context_with_timeout`) and
  `rag_format` (renders a `RagRetrievalResult` via `format_for_prompt`, which appends the
  shared RAG omission marker when memories are shed) anchors. The RAG section now documents
  the entire Phase 33 surface the audit found absent — `RagRetrievalResult`, `ShedItem`,
  `RagRetrievalError`, `retrieve_context_with_timeout`, `with_token_counter`, the omission
  marker — without reintroducing the Phase 32 deleted token-counter types.
- Full `docs.yml` gate sequence verified green on the final commit: `mdbook-mermaid install
  docs/` left `git status --porcelain -- docs` clean, `mdbook build docs/` reported "No broken
  links found", `./scripts/check-doc-examples.sh` reported 0 checked/623 skipped/0 failed,
  `./scripts/check-doc-config.sh` reported 154 YAML blocks/0 failed, and `make api-surface`
  confirmed no public-surface drift (`doc-examples` is `publish = false`).
- `git diff 0533e722..HEAD --name-only -- src/ crates/` lists only paths under
  `crates/doc-examples/` (D-00c honored).

## Closure Table (D-23)

| MB-nn | Page | Disposition | Commit | How the §5 finding was addressed |
|-------|------|-------------|--------|-----------------------------------|
| MB-27 | `docs/src/user-guides/paladin-agents.md` | corrected | `7207e894` | New `paladin_agents.rs` module (`build_agent`, `attach_garrison`); page's Garrison/Handoffs sections now `{{#include}}` the compile-verified anchors; version pin `0.5.0`→`0.10.0` |
| MB-19 | `docs/src/user-guides/arsenal-tools.md` | corrected | `2cc4bdd3` | New `arsenal_tools.rs` module (`custom_armament`, `handoffs`); Custom Armaments/Handoff Tool sections now `{{#include}}` the compile-verified anchors |
| MB-24 | `docs/src/user-guides/herald-output.md` | corrected | `83d44878` | New `herald_output.rs` module (`custom_herald`, all 7 trait methods); Herald Trait listing and Custom Herald Implementation now agree with `output-formatting.md` |
| MB-20 | `docs/src/user-guides/battalion-patterns.md` | corrected | `dc771569` | New `battalion_patterns.rs` module (`commander`, via `CommanderBuilder`); Commander section now `{{#include}}`s the compile-verified anchor; version pin `0.5.0`→`0.10.0` |
| MB-28 | `docs/src/user-guides/sanctum-vector-memory.md` | corrected | `185cacf5` | New `sanctum_vector_memory.rs` module (`rag_retrieve`, `rag_format`); RAG section extended to document the full Phase 33 surface; service name corrected to camelCase |

## Task Commits

Each task was committed atomically (one commit per page per D-24, except Task 1 which is a
single `type="tracer"` task):

1. **Task 1: MB-27 — paladin-agents.md end to end through a new module** — `7207e894`
2. **Task 2: MB-19 and MB-24 — arsenal-tools.md and herald-output.md** —
   `2cc4bdd3` (arsenal-tools.md) + `83d44878` (herald-output.md)
3. **Task 3: MB-20 and MB-28 — battalion-patterns.md and sanctum-vector-memory.md** —
   `dc771569` (battalion-patterns.md) + `185cacf5` (sanctum-vector-memory.md)

_Note: Tasks 2 and 3 each land two page commits per D-24 ("one commit per page"); the shared
`lib.rs` `pub mod` addition for each module is split across the two commits so each page's
commit carries only its own new module's registration line._

## Files Created/Modified

- `crates/doc-examples/src/paladin_agents.rs` - New module: `build_agent`, `attach_garrison` anchors
- `crates/doc-examples/src/arsenal_tools.rs` - New module: `custom_armament`, `handoffs` anchors
- `crates/doc-examples/src/herald_output.rs` - New module: `custom_herald` anchor (7-method Herald impl)
- `crates/doc-examples/src/battalion_patterns.rs` - New module: `commander` anchor
- `crates/doc-examples/src/sanctum_vector_memory.rs` - New module: `rag_retrieve`, `rag_format` anchors
- `crates/doc-examples/src/lib.rs` - Five `pub mod` registrations, additions only (D-26)
- `crates/doc-examples/Cargo.toml` - Added `paladin-memory` dependency (Rule 3 auto-fix, see Deviations)
- `docs/src/user-guides/paladin-agents.md` - Garrison/Handoffs sections now compile-verified; version pin fixed
- `docs/src/user-guides/arsenal-tools.md` - Custom Armaments/Handoff Tool sections now compile-verified
- `docs/src/user-guides/herald-output.md` - Full 7-method Herald trait documented and compile-verified
- `docs/src/user-guides/battalion-patterns.md` - Commander section now compile-verified; version pin fixed
- `docs/src/user-guides/sanctum-vector-memory.md` - Service name fixed; RAG section extended to the Phase 33 surface

## Decisions Made

- **Added `paladin-memory` as a `crates/doc-examples/Cargo.toml` dependency.** The plan's
  `files_modified` list didn't name `Cargo.toml`, but `InMemoryGarrison` (MB-27) and
  `InMemorySanctum`/`RagRetrievalService` (MB-28) all live in `paladin-memory`, which
  `doc-examples` did not previously depend on. The plan's own `<threat_model>` (T-35-SC) already
  names `paladin-memory` as one of the "existing first-party workspace path dependencies" the new
  modules use, so this was an anticipated, not a discovered, need. No extra Cargo feature was
  required — `InMemoryGarrison`, `InMemorySanctum` and the RAG retrieval service are all
  unconditional modules (`default = []` on `paladin-memory`, no `#[cfg(feature = ...)]` on any of
  the three).
- **`attach_garrison` anchor serves both the Memory — Garrison and Agent Handoffs sections.**
  The live API composes Garrison attachment and specialist registration on the same
  `PaladinBuilder` chain, so one function demonstrates both — the page's two sections both
  `{{#include}}` the same anchor rather than splitting into two narrower functions.
- **Local `MockEmbedder` inside `sanctum_vector_memory.rs`, not added to `support.rs`.** No
  shipped `EmbeddingPort` mock exists outside test modules of other crates; adding one to the
  shared `support.rs` would violate D-26's additions-only boundary on Phase 35's own module set
  (support.rs isn't a Phase 35 addition). A module-local mock keeps the boundary intact.
- **`rag_format` calls the real `RagRetrievalService::format_for_prompt`** rather than
  reimplementing a renderer, so the doc example can never drift from the shared
  `rag_omission_marker` behavior the facade's `format_retrieved_context` also uses.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing `paladin-memory` Cargo dependency to `crates/doc-examples`**
- **Found during:** Task 1 (paladin_agents.rs authoring)
- **Issue:** `InMemoryGarrison` (needed for the `attach_garrison` anchor) lives in
  `paladin-memory`, which `crates/doc-examples/Cargo.toml` did not list as a dependency —
  `cargo check -p paladin-doc-examples` would fail to resolve the import.
- **Fix:** Added `paladin-memory = { version = "0.10.0", path = "../paladin-memory" }` (no
  extra feature — the needed modules are unconditional). This is a workspace-internal path
  dependency, not a registry-package install, so the Rule 3 package-install exclusion and
  Package Legitimacy Gate do not apply; the plan's own threat model already anticipated this
  exact dependency.
- **Files modified:** `crates/doc-examples/Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo check -p paladin-doc-examples` compiles clean; `./scripts/check-doc-examples.sh` passes.
- **Committed in:** `7207e894` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking — missing workspace dependency)
**Impact on plan:** Necessary for the plan's own anticipated module shape (named in its threat
model) to compile. No scope creep — no registry package involved, no feature flags added beyond
what the anchors require.

## Issues Encountered

None. Every API shape used (`InMemoryGarrison::new(config)`, `with_handoffs`, `ArmamentResult`'s
five fields, `ArmamentCall::arguments`, the seven-method `Herald` trait, `CommanderBuilder`,
`RagRetrievalService`, `retrieve_context_with_timeout`, `with_token_counter`,
`format_for_prompt`) was verified directly against the live tree before writing code, and
`cargo check -p paladin-doc-examples` compiled clean on the first attempt for all five modules;
`cargo fmt --check` needed one small wrap fix in `sanctum_vector_memory.rs` (a `use` line over
100 chars), applied before the Task 3 commits; `cargo clippy -p paladin-doc-examples --all-targets
-- -D warnings` was clean throughout.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- MB-27, MB-19, MB-24, MB-20 and MB-28 are closed; five of the seven D-12 candidate modules now
  exist (MB-10 and MB-12 were explicitly dropped from this plan's scope per D-12's "drop a
  candidate whose fix is a fragment under D-11(b)" rule — plan 35-04 corrects them in place as
  marked fragments).
- `crates/doc-examples/src/lib.rs` now registers ten `pub mod` entries alongside `support` — all
  additions, no existing module edited (D-26 honored for this plan).
- No blockers for later Phase 35 waves. No observation was made on a page the audit settled
  `current` during this plan's work — no `## Deferred observations` section is needed.

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

All 11 created/modified source and doc files confirmed present on disk. All 5 page commit hashes
(`7207e894`, `2cc4bdd3`, `83d44878`, `dc771569`, `185cacf5`) confirmed present in `git log`.
