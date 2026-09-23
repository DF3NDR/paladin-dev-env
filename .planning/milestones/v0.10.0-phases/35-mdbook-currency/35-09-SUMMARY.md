---
phase: 35-mdbook-currency
plan: 09
subsystem: docs
tags: [mdbook, appendix, paladin-ports, paladin-llm, council, battalion-patterns, integration-tests, security-scanning]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: "35-01's minted CURR-06..CURR-10 requirement IDs and the D-13 import rule established in 35-CONTEXT.md/35-RESEARCH.md"
provides:
  - "D-13 import rule applied to all six systemic-import appendix pages (minio, redis-queue, sanctum-migration, port-trait-template, provider-expansion, sentinel) — both `paladin::paladin_ports::` exit grep patterns closed on every page this plan touches"
  - "council.md's CouncilExecutionService constructor and CouncilResult/TerminationCondition/CouncilConfig shapes corrected to the live paladin-battalion API"
  - "battalion-patterns-guide.md's four opening imports corrected to the compiling facade path"
  - "integration-tests.md's Main test files inventory rebuilt to cover all 59 live tests/integration/*.rs files"
  - "security-scanning.md's Snyk and CodeQL sections rewritten to the measured, source-of-truth disposition from security.instructions.md, with all five .cargo/audit.toml advisories listed"
affects: ["35-10"]

# Tech tracking
tech-stack:
  added: []
  patterns: ["D-11c: appendix pages fenced rust,ignore wholesale once touched, proved via a throwaway examples/_scratch.rs + cargo check --example _scratch probe per page, never committed"]

key-files:
  created: []
  modified:
    - docs/src/appendix/minio-file-repository-setup.md
    - docs/src/appendix/redis-queue-adapter-setup.md
    - docs/src/appendix/sanctum-migration.md
    - docs/src/appendix/port-trait-template.md
    - docs/src/appendix/provider-expansion.md
    - docs/src/appendix/sentinel.md
    - docs/src/appendix/council.md
    - docs/src/appendix/battalion-patterns-guide.md
    - docs/src/appendix/integration-tests.md
    - docs/src/appendix/security-scanning.md

key-decisions:
  - "D-13's import rule was applied to every occurrence of the paladin::paladin_ports:: double-nesting defect found on a page this plan touches, not only the lines the audit's Cites cell named — sentinel.md carried a sixth, previously-uncited input::document_port instance of the same defect, fixed in a follow-up commit on the same page."
  - "Fenced every bare rust code block rust,ignore on all ten pages (not only the blocks whose imports were corrected), matching the tracer task's (MB-50) own acceptance criterion and D-11(c)'s literal wording that appendix pages are corrected in place and fenced rust,ignore as a page-level disposition."
  - "council.md required correcting far more than the constructor/result-field shapes MB-48's finding cited: TerminationCondition's real shape is unit-variant (MaxRounds, Consensus, ModeratorDecision) with the round count living on CouncilConfig::max_rounds/CouncilBuilder::max_rounds(), not a MaxRounds(u32) tuple; CouncilConfig's real fields are max_rounds/turn_strategy/termination_condition/include_history, not the five-field shape (with a fabricated turn_timeout and a store_history typo) the page showed. All five occurrences across the page were corrected since the primary Quick Start sample had to scratch-compile, and leaving the other four illustrative fragments internally contradictory on the same page was not an acceptable outcome."
  - "provider-expansion.md's provider-comparison table was rewritten from measured ProviderCapabilities values read directly from each adapter's get_capabilities() implementation (crates/paladin-llm/src/*/adapter.rs), not transcribed from the page's own prior claims — the page previously showed Tool Calling/Function Calling as Yes for all three profiled providers, but no shipped adapter declares either true today (confirmed by the ProviderCapabilities struct's own rustdoc)."
  - "OpenAI's programmatic-configuration examples on provider-expansion.md and sentinel.md used a fabricated OpenAILlmAdapter::new(api_key, Option<base_url>, Option<Duration>) three-positional-arg constructor and a fabricated OpenAiConfig{ ..Default::default() } struct literal (OpenAIConfig has no Default impl and no model field); both were rewritten to the real OpenAIConfig{api_key, base_url, organization, timeout_seconds, max_retries} shape and OpenAIAdapter::new(config)."

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "MB-50 (minio-file-repository-setup.md): all six paladin::paladin_ports:: occurrences corrected to the bare paladin_ports::output::file_storage_port crate path; adapter imports and container-image pins left unchanged; page fenced rust,ignore wholesale; scratch-compile proved"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "grep -c 'paladin::paladin_ports::' docs/src/appendix/minio-file-repository-setup.md == 0; cargo check --example _scratch --features cli,s3-storage"
        status: pass
    human_judgment: false
  - id: D2
    description: "MB-53 (redis-queue-adapter-setup.md): queue-port import corrected; a second, previously-uncited QueueError import (paladin::core::platform::manager::queue_service, a module that no longer exists) discovered and corrected to paladin_ports::output::queue_port::QueueError during the scratch-compile probe"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "cargo check --example _scratch --features cli,redis-queue"
        status: pass
    human_judgment: false
  - id: D3
    description: "MB-56 (sanctum-migration.md): all three paladin::paladin_ports:: occurrences corrected (one, line 47, previously uncited); the output::{SanctumPort, EmbeddingPort} single-line glob split into two module-qualified imports since output:: does not re-export either symbol directly"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "cargo check --example _scratch --features cli,qdrant"
        status: pass
    human_judgment: false
  - id: D4
    description: "MB-51 (port-trait-template.md): all four template placeholder imports corrected (two previously uncited, at lines 152 and 235); probe substitutes a real port (file_storage_port) for the placeholder to prove the shape"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "cargo check --example _scratch --features cli"
        status: pass
    human_judgment: false
  - id: D5
    description: "MB-52 (provider-expansion.md): three adapter imports and one port import corrected; provider-comparison table expanded to all nine shipped providers with measured ProviderCapabilities values; version footer corrected to 0.10.0; four OpenAILlmAdapter three-arg-constructor call sites across the page rewritten to the real OpenAIConfig/OpenAIAdapter shape"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "cargo check --example _scratch --features cli,llm-openai,llm-deepseek,llm-anthropic; for p in kimi qwen grok ollama gemini; do grep -qi \"$p\" ...; done"
        status: pass
    human_judgment: false
  - id: D6
    description: "MB-58 (sentinel.md): three adapter imports corrected (module path + OpenAIAdapter casing); two fabricated OpenAiConfig{..Default::default()} struct literals rewritten to the real 5-field OpenAIConfig shape; a sixth, previously-uncited paladin_ports::input::document_port import corrected in a follow-up commit"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "cargo check --example _scratch --features cli,vision,llm-openai,llm-anthropic"
        status: pass
    human_judgment: false
  - id: D7
    description: "MB-48 (council.md): CouncilExecutionService::new corrected to its live 3-argument form (both call sites on the page); CouncilResult/TerminationCondition/CouncilConfig field and variant shapes corrected throughout the page to match crates/paladin-core/src/platform/container/battalion/council.rs and crates/paladin-battalion/src/council_service.rs"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "cargo check --example _scratch --features cli; grep -cE 'conversation_history|final_output' docs/src/appendix/council.md == 0"
        status: pass
    human_judgment: false
  - id: D8
    description: "MB-38 (battalion-patterns-guide.md): all four opening use paladin::battalion::*; imports replaced with the compiling paladin::core::platform::container::battalion::*; path"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "grep -c 'use paladin::battalion::' docs/src/appendix/battalion-patterns-guide.md == 0; cargo check --example _scratch --features cli"
        status: pass
    human_judgment: false
  - id: D9
    description: "MB-49 (integration-tests.md): Main test files inventory rebuilt from ls tests/integration/*.rs — all 59 live test files (26 previously missing) now present with Crate Scope, Services Required and Feature Gate columns read from tests/integration/mod.rs and Cargo.toml [[test]] entries"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "comm -23 <(ls tests/integration/*.rs | xargs -n1 basename | grep -v mod.rs | sort) <(grep -oE '`[a-z0-9_]+_test(s)?\\.rs`' docs/src/appendix/integration-tests.md | tr -d '`' | sort -u) — empty"
        status: pass
    human_judgment: false
  - id: D10
    description: "MB-57 (security-scanning.md): Snyk section rewritten to the evaluated-and-removed / zero-Rust-coverage disposition; new Known Gap: No Rust SAST section states CodeQL's advisory-only disposition; tracked-exceptions list expanded from 2 to all 5 .cargo/audit.toml advisories"
    requirement: "CURR-08"
    verification:
      - kind: other
        ref: "grep -qi 'evaluated and removed' docs/src/appendix/security-scanning.md; grep -qi codeql docs/src/appendix/security-scanning.md; ./scripts/check-doc-config.sh"
        status: pass
    human_judgment: false

# Metrics
duration: 32min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 09: Close the Ten Remaining Appendix Rows Summary

**Closed the systemic `paladin::paladin_ports::` / relocated-LLM-adapter import defect on six appendix pages and the four API-shape/inventory defects on council.md, battalion-patterns-guide.md, integration-tests.md and security-scanning.md — every corrected sample scratch-compile-proved, one commit per page.**

## Performance

- **Duration:** ~32 min
- **Started:** 2026-09-17T13:39:00Z
- **Completed:** 2026-09-17T14:11:00Z
- **Tasks:** 3
- **Files modified:** 10

## Accomplishments

- **Task 1 (tracer, MB-50):** Fixed all six `paladin::paladin_ports::` occurrences on
  `minio-file-repository-setup.md` (two more than the audit's Cites cell named), added the
  illustrative-fragment header note, fenced all 23 rust blocks `rust,ignore`, and proved the
  corrected imports compile with a throwaway `examples/_scratch.rs` — establishing the pattern
  for the remaining nine pages.
- **Task 2 (MB-53, MB-56, MB-51, MB-52, MB-58):** Closed the rest of the D-13 import family —
  `redis-queue-adapter-setup.md`, `sanctum-migration.md`, `port-trait-template.md`,
  `provider-expansion.md`, `sentinel.md` — each scratch-compile-proved, each with additional
  previously-uncited instances of the same defect class discovered and fixed in passing (a
  stale `QueueError` module path on redis-queue-adapter-setup.md, a third `SanctumFilter` import
  on sanctum-migration.md, two more placeholder imports on port-trait-template.md). Expanded
  `provider-expansion.md`'s provider-comparison table to all nine shipped providers using
  measured `ProviderCapabilities` values, and corrected its version footer to 0.10.0.
- **Task 3 (MB-48, MB-38, MB-49, MB-57):** Corrected `council.md`'s `CouncilExecutionService`
  constructor, `CouncilResult` fields, and (beyond the cited finding) the `TerminationCondition`
  and `CouncilConfig` shapes the page had fabricated throughout; fixed all four opening imports
  on `battalion-patterns-guide.md`; rebuilt `integration-tests.md`'s test inventory to cover all
  59 live `tests/integration/*.rs` files (26 previously missing); rewrote
  `security-scanning.md`'s Snyk and CodeQL sections to the measured disposition from
  `.github/instructions/security.instructions.md` and listed all five tracked advisories.

## Task Commits

Each task was committed atomically (one commit per page, per D-24; two pages required a
same-page follow-up commit for an additional in-scope defect discovered during the
scratch-compile probe):

1. **Task 1: MB-50** — `f65f2db2` (docs)
2. **Task 2: MB-53, MB-56, MB-51, MB-52, MB-58** — `626a6019`, `c6021fed`, `5b08ce79`,
   `b9596926`, `d5855c4f` (docs), plus `9bb7362a` (fix: MB-58 follow-up — a sixth,
   previously-uncited `paladin_ports::input::document_port` import on sentinel.md)
3. **Task 3: MB-48, MB-38, MB-49, MB-57** — `41d58304`, `1b887f18`, `fe8386c6`, `5afcc542` (docs)

## Files Created/Modified

- `docs/src/appendix/minio-file-repository-setup.md` — D-13 import fix, header note, page-wide `rust,ignore` fencing
- `docs/src/appendix/redis-queue-adapter-setup.md` — D-13 import fix (queue_port + QueueError)
- `docs/src/appendix/sanctum-migration.md` — D-13 import fix (three occurrences)
- `docs/src/appendix/port-trait-template.md` — D-13 import fix (four occurrences)
- `docs/src/appendix/provider-expansion.md` — D-13 import fix, nine-provider comparison table, version footer, OpenAI constructor rewrite (4 sites)
- `docs/src/appendix/sentinel.md` — D-13 import fix + casing, OpenAIConfig/AnthropicConfig struct-literal rewrite, DocumentPort import fix
- `docs/src/appendix/council.md` — CouncilExecutionService constructor, CouncilResult/TerminationCondition/CouncilConfig shapes
- `docs/src/appendix/battalion-patterns-guide.md` — four opening imports corrected to compiling facade path
- `docs/src/appendix/integration-tests.md` — Main test files inventory rebuilt (26 files added)
- `docs/src/appendix/security-scanning.md` — Snyk/CodeQL sections rewritten, tracked-exceptions list expanded to 5

## Decisions Made

See `key-decisions` in frontmatter for the five decisions with fullest rationale. In short:
D-13's import rule and D-11(c)'s `rust,ignore` fencing were applied page-wide (every bare `rust`
block on a touched appendix page, and every occurrence of the double-nesting defect found on a
touched page), not narrowly to only the lines the audit's Cites cell named — this surfaced
several additional, previously-uncited instances of the same defect classes, all fixed under
deviation Rule 1 (bug) since each blocked the page's own scratch-compile proof obligation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `redis-queue-adapter-setup.md`'s `QueueError` import pointed at a module
that no longer exists**
- **Found during:** Task 2, scratch-compiling the redis-queue page's corrected sample
- **Issue:** `use paladin::core::platform::manager::queue_service::QueueError;` — the audit
  recorded this as "confirmed compiling", but `queue_service` does not exist anywhere under
  `src/core/platform/manager/` in the live tree; the real `QueueError` (matching the page's
  `QueueNotFound`/`QueueFull{queue_name,capacity}`/`OperationFailed` variant usage) lives at
  `paladin_ports::output::queue_port::QueueError`.
- **Fix:** Corrected the import; both `MessagePriority` and `QueueError` now import cleanly.
- **Files modified:** `docs/src/appendix/redis-queue-adapter-setup.md`
- **Verification:** `cargo check --example _scratch --features cli,redis-queue` compiles clean.
- **Committed in:** `626a6019`

**2. [Rule 1 - Bug] `sanctum-migration.md` carried a third, previously-uncited
`paladin::paladin_ports::` occurrence (line 47) plus a broken `output::{SanctumPort,
EmbeddingPort}` glob**
- **Found during:** Task 2, applying the D-13 fix
- **Issue:** The audit's Cites cell named only lines 241 and 386; a third occurrence at line 47
  carries the same defect. Separately, line 386's `paladin::paladin_ports::output::{SanctumPort,
  EmbeddingPort}` cannot compile even after stripping the `paladin::` prefix — `output::` does
  not re-export either symbol directly; they live in the `sanctum_port` and `embedding_port`
  submodules respectively.
- **Fix:** Corrected all three occurrences; split the two-symbol glob into two module-qualified
  imports.
- **Files modified:** `docs/src/appendix/sanctum-migration.md`
- **Verification:** `cargo check --example _scratch --features cli,qdrant` compiles clean.
- **Committed in:** `c6021fed`

**3. [Rule 1 - Bug] `port-trait-template.md` carried two more placeholder-import occurrences
than the audit's Cites cell named**
- **Found during:** Task 2, applying the D-13 fix
- **Issue:** Cites named lines 48 and 60; lines 152 and 235 carry the identical
  `paladin::paladin_ports::output::port_name::…` placeholder pattern.
- **Fix:** Corrected all four occurrences with the same bare-crate-path rule.
- **Files modified:** `docs/src/appendix/port-trait-template.md`
- **Verification:** `cargo check --example _scratch --features cli` (probe substitutes
  `file_storage_port::FileStoragePort` for the placeholder) compiles clean.
- **Committed in:** `5b08ce79`

**4. [Rule 1 - Bug] `provider-expansion.md`'s OpenAI example used a fabricated three-positional-arg
constructor at four separate call sites**
- **Found during:** Task 2, correcting the OpenAI adapter import
- **Issue:** `OpenAILlmAdapter::new(api_key, Option<base_url>, Option<Duration>)` does not exist;
  the real `OpenAIAdapter::new(config: OpenAIConfig) -> Result<Self, String>` takes a config
  struct, matching the DeepSeek/Anthropic pattern already correct on the same page. The pattern
  recurred at three more call sites in the Migration Guide and Best Practices sections beyond the
  Programmatic Configuration section the finding cited.
- **Fix:** Rewrote all four call sites to `OpenAIAdapter::new(OpenAIConfig::from_env()?)?` /
  the equivalent custom-config form.
- **Files modified:** `docs/src/appendix/provider-expansion.md`
- **Verification:** `cargo check --example _scratch --features cli,llm-openai,llm-deepseek,llm-anthropic`
  compiles clean.
- **Committed in:** `b9596926`

**5. [Rule 1 - Bug] `sentinel.md`'s vision examples constructed `OpenAiConfig`/`AnthropicConfig`
via `..Default::default()`, but neither type implements `Default`, and `OpenAIConfig` has no
`model` field**
- **Found during:** Task 2, correcting the adapter imports and casing
- **Issue:** Both vision config blocks used `..Default::default()` (no such impl exists) and put
  `model: "gpt-4o"` inside `OpenAiConfig` (the real `OpenAIConfig` has no `model` field — model
  selection happens on `PaladinBuilder::model()`, as the page's own Quick Example shows two
  sections earlier). `AnthropicConfig`'s literal was also missing the required `max_tokens` field.
- **Fix:** Rewrote both config literals to the real 5-field `OpenAIConfig` shape (with a comment
  noting where `model` actually goes) and added the missing `max_tokens`/`timeout_seconds` to the
  `AnthropicConfig` literal.
- **Files modified:** `docs/src/appendix/sentinel.md`
- **Verification:** `cargo check --example _scratch --features cli,vision,llm-openai,llm-anthropic`
  compiles clean.
- **Committed in:** `d5855c4f`

**6. [Rule 1 - Bug] `sentinel.md` carried a sixth, previously-uncited `paladin::paladin_ports::`
occurrence on an unrelated port (`input::document_port`)**
- **Found during:** Task 2, running the book-wide D-13 exit grep after committing MB-58
- **Issue:** Line 365's `use paladin::paladin_ports::input::document_port::{…};` was not part of
  MB-58's cited finding (which only concerned the LLM adapter imports) but is the identical
  double-nesting defect, on the same page this plan is actively correcting.
- **Fix:** Corrected to `paladin_ports::input::document_port::{DocumentPort, DocumentSource,
  ChunkConfig}` — confirmed against `crates/paladin-ports/src/input/document_port.rs`.
- **Files modified:** `docs/src/appendix/sentinel.md`
- **Verification:** Book-wide `grep -rn 'paladin::paladin_ports::' docs/src` returns nothing;
  `mdbook build docs/` — "No broken links found".
- **Committed in:** `9bb7362a`

**7. [Rule 1 - Bug] `council.md`'s `TerminationCondition`, `CouncilConfig` and a second
`CouncilExecutionService::new` call site were fabricated beyond MB-48's cited finding**
- **Found during:** Task 3, scratch-compiling the corrected "Basic Council Example"
- **Issue:** The page's `TerminationCondition::MaxRounds(u32)` tuple variant does not exist — the
  live enum is `MaxRounds` (unit), with the round count living on `CouncilConfig::max_rounds` /
  `CouncilBuilder::max_rounds()`. This pattern recurred at five call sites across the page. The
  page's `CouncilConfig` struct definition (in the API Reference section) showed five fields
  including a fabricated `turn_timeout: Duration` and a `store_history` typo for the real
  `include_history`; the live struct has exactly four fields (`max_rounds`, `turn_strategy`,
  `termination_condition`, `include_history`). A second `CouncilExecutionService::new` call site
  in the Garrison Integration section also omitted the `registry` argument.
- **Fix:** Corrected all five `TerminationCondition::MaxRounds(N)` call sites to
  `.max_rounds(N)` + `.termination_condition(TerminationCondition::MaxRounds)`; rewrote the
  `CouncilConfig`/`TerminationCondition` struct/enum definitions in the API Reference section to
  match the live shapes; added the missing `registry` argument to the second constructor call;
  renamed `store_history` to `include_history`.
- **Files modified:** `docs/src/appendix/council.md`
- **Verification:** `cargo check --example _scratch --features cli` compiles clean; `grep -cE
  'conversation_history|final_output' docs/src/appendix/council.md` returns 0.
- **Committed in:** `41d58304`

---

**Total deviations:** 7 auto-fixed (7 bugs, all Rule 1 — every one blocked the page's own
scratch-compile proof obligation or the plan's own book-wide exit grep, discovered on a page
this plan was already actively correcting, never on a neighbouring `current`-tier page)
**Impact on plan:** All auto-fixes were necessary for the corrected samples to actually compile
and for the plan's own D-13/D-11(c) success criteria to hold. No scope creep onto pages outside
this plan's `files_modified` list.

## Issues Encountered

**`battalion-patterns-guide.md`'s body content (beyond the opening imports) still carries a
casing bug (`OpenAiAdapter`) and other pre-existing API-shape drift.** MB-38's cited finding and
this task's `<action>`/`<verify>` text scoped the fix explicitly to the four opening import
lines ("Replace each opening import with the facade forms that do compile"); the body of each of
the four examples (`OpenAiAdapter::new().build()?`, `Formation::new().add_paladin(researcher)…`,
etc.) was left as-is per that explicit scope. `OpenAiAdapter` also appears on five other pages
entirely outside this plan's `files_modified` list (`contributing/testing-guide.md`,
`appendix/battalion-vision-support.md`, `appendix/conclave-pattern.md`,
`contributing/architecture-decisions.md`, `user-guides/memory-management.md`,
`user-guides/tool-integration.md`) — not fixed, not this plan's scope.

**The book-wide D-13 `paladin::infrastructure::adapters::llm::` exit grep is not fully empty
after this plan alone.** `docs/src/api-reference/feature-flags.md:302` and
`docs/src/contributing/contributing-providers.md:272,367` still carry the pattern.
`feature-flags.md` is MB-15's own row (a different plan's responsibility per the audit's §5
work list); `contributing-providers.md` is explicitly NOT in the audit's §5 work list and was
already recorded as a deferred observation in `deferred-items.md` by plan 35-01. The
`paladin::paladin_ports::` exit grep, by contrast, IS fully empty book-wide as of this plan's
final commit (confirmed live: `grep -rn 'paladin::paladin_ports::' docs/src` returns nothing).
The final phase-closing plan (35-10) is expected to confirm full book-wide closure across all
wave plans in `35-EVIDENCE.md`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All ten `MB-nn` rows this plan owned are closed: MB-50, MB-53, MB-56, MB-51, MB-52, MB-58,
  MB-48, MB-38, MB-49, MB-57.
- Both D-13 exit greps: `paladin::paladin_ports::` is fully empty book-wide; the LLM-adapter
  grep has two known remaining hits outside this plan's scope (see Issues Encountered).
- `mdbook build docs/` — "No broken links found"; `./scripts/check-doc-examples.sh` — 0
  checked/620 skipped/0 failed; `./scripts/check-doc-config.sh` — 154 YAML blocks/0 failed;
  `make api-surface` — unchanged (3959 items).
- `git status --porcelain -- examples` empty and `examples/_scratch.rs` absent after every
  commit in this plan.
- No blockers for plan 35-10 (CHANGELOG, exit greps, `35-EVIDENCE.md`).

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

All 10 modified files confirmed present on disk with their corrections. All 11 commit hashes
confirmed present in `git log` (`f65f2db2`, `626a6019`, `c6021fed`, `5b08ce79`, `b9596926`,
`d5855c4f`, `9bb7362a`, `41d58304`, `1b887f18`, `fe8386c6`, `5afcc542`).
