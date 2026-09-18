---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 11
subsystem: docs
tags: [examples-gallery, readme, gallery-index, cross-check, currency]

# Dependency graph
requires:
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-01-SUMMARY.md (house example shape, Token Economy Examples section precedent)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-06-SUMMARY.md (WarEngine Configuration & Checkpoints, Control Flow & Dynamic Routing programs)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-07-SUMMARY.md (Human-in-the-Loop Gate, Graceful Shutdown programs)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-08-SUMMARY.md (Agent Runtime Middleware, Structured Output, Sanctum RAG Retrieval programs)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-09-SUMMARY.md (Platform API Client, Webhook Receiver programs; HTTP-service-host router-parity fix)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-10-SUMMARY.md (Node-Result Cache, Observability Tracing/OTel, Eval Scenarios programs)
  - phase: 34-documentation-currency-audit
    provides: 34-AUDIT.md sec4 (the 59 EX-nn gap rows and the five stale EX-nn rows -- EX-01, EX-33, EX-55, EX-121, EX-122)
provides:
  - examples/README.md -- corrected Getting Started block (Rust 1.88), corrected
    PaladinResult snippet fields (output/usage/execution_time_ms), and a complete
    Table of Contents + section for all 62 programs on disk
  - 36-evidence/36-11-readme.txt -- before/after text for the EX-01/EX-122 fixes,
    the eleven-header and both-directions cross-check evidence
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "The README's own Demonstrates-line-count invariant (one bold Demonstrates
      line per on-disk .rs file) is only meaningful if no non-.rs section also
      carries the same bold prefix -- a single stray cli_configs/*.yaml section
      using **Demonstrates:** where its five siblings use plain prose broke the
      count silently until this plan's own both-directions cross-check exercise
      surfaced it."
    - "GitHub's heading-to-anchor slug algorithm strips non-alphanumeric/space/
      hyphen characters (parens, slashes, ampersands) without inserting a
      replacement, so 'RAG & Retrieval' slugs to #rag--retrieval and 'Commander
      Strategies (Council / Grove / Conclave)' slugs to
      #commander-strategies-council--grove--conclave -- double hyphens are the
      expected, correct output for a heading with a removed-not-replaced
      character between two spaces, not a typo."

key-files:
  created:
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-11-readme.txt
  modified:
    - examples/README.md

key-decisions:
  - "All three tasks' README edits landed in ONE commit (D-26), per this plan's
    own explicit override of the generic per-task-commit default -- the plan's
    own text states 'Do not commit yet' after Tasks 1 and 2, and Task 3 makes
    the single commit for the whole file."
  - "Fixed a pre-existing Demonstrates-line-count defect in the
    cli_configs/maneuver.yaml section (Rule 1: auto-fixed bug) rather than
    leaving it -- the stray **Demonstrates:** prefix that none of its five
    sibling cli_configs/*.yaml sections carry made the file's total
    Demonstrates-line count exceed the .rs on-disk file count by one BEFORE
    this plan started, which would have made Task 3's own count-equality
    <verify> command fail no matter how correctly the 24 new sections were
    written. Folded the bold line into the section's plain descriptive
    sentence, matching every sibling yaml section's format."
  - "Grouped all 24 new sections and their new ## headings into two contiguous
    insertion zones (Vision/Document Processing/HTTP Service Host/RAG &
    Retrieval/Commander Strategies before Performance Benchmarking Examples;
    the twelve remaining Task-3 headings after it) rather than interleaving
    single sections across the file's existing eight thousand-line span --
    the plan's own acceptance criteria require TOC-anchor/heading matching and
    grouping by capability cluster, not a specific document position, and a
    contiguous block is far less error-prone to edit and verify than scattered
    single-section insertions."

patterns-established:
  - "The examples/README.md gallery-index-completeness invariant (both-directions
    comm cross-check + Demonstrates-line-count equality) is now proven empty/
    equal end to end for the first time in this phase -- any future PLAN adding
    a new example must add its README section in the SAME commit or this
    invariant breaks again on the next full-phase audit."

requirements-completed: [CURR-14]

coverage:
  - id: D1
    description: "EX-01 closed: examples/README.md's Getting Started block states the workspace's real minimum Rust version (1.88, matching Cargo.toml's rust-version), and every feature name in the 'Run with Specific Features' block (redis-queue, s3-storage) exists in the root Cargo.toml [features] table"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "RUSTV=$(grep -m1 '^rust-version' Cargo.toml | sed 's/.*\"\\(.*\\)\".*/\\1/'); grep -q \"Rust $RUSTV\" examples/README.md (exit 0, RUSTV=1.88)"
        status: pass
    human_judgment: false
  - id: D2
    description: "EX-122 closed: the three drifted PaladinResult snippet lines (Basic Paladin Examples, Logging and Observability, Building a Custom Example) now read the real field names -- output, usage.total_tokens, execution_time_ms -- and no README line names a PaladinResult field that does not exist"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "grep -cE 'response\\.content|response\\.token_usage|response\\.execution_time([^_]|$)' examples/README.md -> 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "EX-121 closed: all eleven previously unlisted on-disk programs (commander_council.rs, commander_grove.rs, conclave_expert_panel.rs, council_discussion.rs, document_processing.rs, grove_routing.rs, http_service_host.rs, paladin_with_rag.rs, vision_analysis.rs, vision_battalion.rs, war_engine_memory_baseline.rs) now have a ### [name.rs] section and a matching TOC entry"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "comm -23 against the eleven-name list and the extracted ### [name.rs] header list -> empty"
        status: pass
    human_judgment: false
  - id: D4
    description: "All 24 previously-undocumented on-disk programs (the eleven from EX-121 plus the thirteen new Phase 36 programs) have a README section, every 59-row EX-nn gap capability has exactly one Demonstrates line naming it, and the both-directions listed-vs-on-disk cross-check is empty (62 on-disk .rs files == 62 listed sections == 62 Demonstrates lines)"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "comm -23 and comm -13 between /tmp/36-11-ondisk.txt (62) and /tmp/36-11-listed.txt (62) both empty; grep -c '^\\*\\*Demonstrates:\\*\\*' examples/README.md == 62"
        status: pass
    human_judgment: false
  - id: D5
    description: "The single README commit (D-26) touches only examples/README.md and this plan's own evidence file -- no example .rs file, no Cargo.toml, no CI/script/entry-point file is modified"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "git diff --stat HEAD~1 HEAD -> examples/README.md and 36-evidence/36-11-readme.txt only"
        status: pass
    human_judgment: false

# Metrics
duration: ~1h10min
completed: 2026-09-17
status: complete
---

# Phase 36 Plan 11: Examples README Gallery-Index Completion Summary

**Closed the last three stale `examples/README.md` currency rows (EX-01 minimum-Rust-version, EX-121 eleven previously unlisted programs, EX-122 three drifted `PaladinResult` snippet fields) and added a gallery section for every one of the fourteen new Phase 36 example programs, bringing the on-disk-vs-listed cross-check to empty in both directions (62 == 62) for the first time in this phase -- one atomic commit, zero `make api-surface` drift (no `.rs` file touched).**

## Performance

- **Duration:** ~1h10min
- **Started:** 2026-09-17T23:07:19Z
- **Completed:** 2026-09-17T23:17:23Z
- **Tasks:** 3
- **Files modified:** 2 (1 README, 1 new evidence file)

## Accomplishments

- Fixed EX-01: `examples/README.md`'s Getting Started block claimed "Rust 1.70 or
  later" against a workspace `rust-version` of `1.88` -- corrected to match. Checked
  the "Run with Specific Features" block's two named features (`redis-queue`,
  `s3-storage`) against the root `Cargo.toml` `[features]` table; both already exist,
  no correction needed there.
- Fixed EX-122: three README snippet lines named `PaladinResult` fields that do not
  exist (`response.content`, `response.token_usage.total_tokens`,
  `response.execution_time`) -- corrected all three (Basic Paladin Examples snippet,
  Logging and Observability advanced snippet, and a third instance in the Building a
  Custom Example snippet, found while checking the rest of the page per the plan's own
  instruction) to the real fields: `output`, `usage.total_tokens`, `execution_time_ms`.
  Both pre-existing snippet blocks stay in place, corrected not deleted.
- Fixed EX-121: added five new `##` sections (**Vision**, **Document Processing**,
  **HTTP Service Host**, **RAG & Retrieval**, **Commander Strategies (Council / Grove /
  Conclave)**) with matching TOC bullets, covering all eleven previously-undiscoverable
  on-disk programs (`commander_council.rs`, `commander_grove.rs`,
  `conclave_expert_panel.rs`, `council_discussion.rs`, `document_processing.rs`,
  `grove_routing.rs`, `http_service_host.rs`, `paladin_with_rag.rs`, `vision_analysis.rs`,
  `vision_battalion.rs`), and joined `war_engine_memory_baseline.rs` to the existing
  **Performance Benchmarking Examples** heading.
- Added ten more `##` sections (**WarEngine Configuration & Checkpoints**, **Control
  Flow & Dynamic Routing**, **Human-in-the-Loop**, **Graceful Shutdown**, **Agent
  Runtime & Middleware**, **Structured Output**, **Platform API**, **Node-Result
  Cache**, **Observability & Tracing**, **Evaluation**) for the thirteen remaining new
  Phase 36 programs -- `sanctum_rag_retrieval.rs` joined the **RAG & Retrieval** section
  this plan's own Task 2 created. Every gated program's fenced run command carries the
  exact feature list its `[[example]]` block declares in `Cargo.toml`
  (`vision,llm-openai`; `content-processing`; `web-server`; `web-server,dev-ui`;
  `redis-cache`; `otel`), and the two build-only programs (`node_result_cache.rs`,
  `observability_otel_export.rs`) each state their external-service prerequisite
  (a running Redis server; a reachable OTLP/HTTP collector) and that they are
  build-verified only in CI.
- Discovered and fixed (Rule 1) a pre-existing defect that would otherwise have broken
  Task 3's own Demonstrates-line-count invariant: `cli_configs/maneuver.yaml`'s section
  carried a stray `**Demonstrates:**` bold prefix none of its five sibling
  `cli_configs/*.yaml` sections use, making the file's Demonstrates-line count exceed
  the on-disk `.rs` file count by one before this plan started. Folded into a plain
  descriptive sentence matching its siblings.
- Ran the Phase 34 both-directions cross-check after all 24 new sections landed:
  `ls examples/*.rs` (62 files) and the extracted `### [name.rs]` header list (62
  headers) are byte-identical sets -- both `comm -23` and `comm -13` differences are
  empty -- and the `**Demonstrates:**` line count (62) equals the on-disk `.rs` file
  count (62).
- One commit (`c9129dd0`) carries the entire plan's changes, per D-26; `git diff --stat`
  against it lists only `examples/README.md` and this plan's own evidence file.

## Task Commits

All three tasks' edits to `examples/README.md` are cumulative and land in a single
commit per this plan's own explicit D-26 override of the generic per-task-commit
default (the plan text says "Do not commit yet" after Tasks 1 and 2):

1. **Tasks 1, 2 and 3 (EX-01, EX-121, EX-122, and the new program sections)** - `c9129dd0` (docs)

**Plan metadata:** _pending -- this SUMMARY's own commit_

## Files Created/Modified

- `examples/README.md` - Getting Started currency fix, corrected `PaladinResult` snippet fields, 24 new program sections across 15 new `##` headings, 1 pre-existing Demonstrates-line-count fix
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-11-readme.txt` - before/after text for the EX-01/EX-122 fixes, the eleven-header cross-check, and the final both-directions cross-check with both lists

## Closure Table (D-24) -- the five stale rows

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| EX-01 | `examples/README.md` Getting Started block | same, line 25 | stale minimum Rust version | "Rust 1.70 or later" -> "Rust 1.88 or later" (matches `Cargo.toml` `rust-version`) | `c9129dd0` |
| EX-121 | `examples/README.md` (eleven programs never mentioned) | same, 5 new `##` sections + 1 joined section | gallery-completeness gap | eleven new `### [name.rs]` sections + TOC bullets | `c9129dd0` |
| EX-122 | `examples/README.md` lines ~74, ~1350-1351, ~1455 | same | drifted `PaladinResult` field names | `response.content`/`token_usage.total_tokens`/`execution_time` -> `output`/`usage.total_tokens`/`execution_time_ms` | `c9129dd0` |
| EX-33 | `examples/http_service_host.rs` | same | router-parity (closed by plan 36-09, this plan adds its README section) | `## HTTP Service Host` section added | `c9129dd0` |
| EX-55 | `crates/doc-examples/src/http_service_host.rs` | same | router-parity doc-examples snippet (closed by plan 36-09; not a `examples/` gallery file, no README section applicable) | n/a (closed entirely by 36-09) | `3363d08d` (36-09) |

## Closure Table -- EX-nn gap row -> program -> README section (all 59 rows)

| ID | program | README section |
|---|---|---|
| EX-109 | token_economy_commissary.rs | Token Economy Examples |
| EX-111 | token_economy_commissary.rs | Token Economy Examples |
| EX-112 | token_economy_commissary.rs | Token Economy Examples |
| EX-113 | token_economy_commissary.rs | Token Economy Examples |
| EX-114 | token_economy_commissary.rs | Token Economy Examples |
| EX-115 | token_economy_commissary.rs | Token Economy Examples |
| EX-62 | war_engine_configuration.rs | WarEngine Configuration & Checkpoints |
| EX-63 | war_engine_configuration.rs | WarEngine Configuration & Checkpoints |
| EX-64 | war_engine_configuration.rs | WarEngine Configuration & Checkpoints |
| EX-65 | war_engine_configuration.rs | WarEngine Configuration & Checkpoints |
| EX-66 | war_engine_configuration.rs | WarEngine Configuration & Checkpoints |
| EX-80 | war_engine_configuration.rs | WarEngine Configuration & Checkpoints |
| EX-67 | control_flow_dynamic_routing.rs | Control Flow & Dynamic Routing |
| EX-68 | control_flow_dynamic_routing.rs | Control Flow & Dynamic Routing |
| EX-69 | control_flow_dynamic_routing.rs | Control Flow & Dynamic Routing |
| EX-70 | control_flow_dynamic_routing.rs | Control Flow & Dynamic Routing |
| EX-71 | human_in_the_loop_gate.rs | Human-in-the-Loop |
| EX-72 | human_in_the_loop_gate.rs | Human-in-the-Loop |
| EX-73 | human_in_the_loop_gate.rs | Human-in-the-Loop |
| EX-74 | graceful_shutdown.rs | Graceful Shutdown |
| EX-75 | graceful_shutdown.rs | Graceful Shutdown |
| EX-76 | graceful_shutdown.rs | Graceful Shutdown |
| EX-83 | agent_runtime_middleware.rs | Agent Runtime & Middleware |
| EX-84 | agent_runtime_middleware.rs | Agent Runtime & Middleware |
| EX-85 | agent_runtime_middleware.rs | Agent Runtime & Middleware |
| EX-86 | agent_runtime_middleware.rs | Agent Runtime & Middleware |
| EX-87 | agent_runtime_middleware.rs | Agent Runtime & Middleware |
| EX-89 | agent_runtime_middleware.rs | Agent Runtime & Middleware |
| EX-88 | structured_output_schema.rs | Structured Output |
| EX-90 | structured_output_schema.rs | Structured Output |
| EX-116 | sanctum_rag_retrieval.rs | RAG & Retrieval |
| EX-117 | sanctum_rag_retrieval.rs | RAG & Retrieval |
| EX-118 | sanctum_rag_retrieval.rs | RAG & Retrieval |
| EX-119 | sanctum_rag_retrieval.rs | RAG & Retrieval |
| EX-120 | sanctum_rag_retrieval.rs | RAG & Retrieval |
| EX-77 | platform_api_client.rs | Platform API |
| EX-78 | platform_api_client.rs | Platform API |
| EX-79 | platform_api_client.rs | Platform API |
| EX-91 | platform_api_client.rs | Platform API |
| EX-92 | platform_api_client.rs | Platform API |
| EX-93 | platform_api_client.rs | Platform API |
| EX-94 | platform_api_client.rs | Platform API |
| EX-95 | platform_api_client.rs | Platform API |
| EX-98 | platform_api_client.rs | Platform API |
| EX-99 | platform_api_client.rs | Platform API |
| EX-104 | platform_api_client.rs | Platform API |
| EX-110 | platform_api_client.rs | Platform API |
| EX-96 | webhook_receiver.rs | Platform API |
| EX-97 | webhook_receiver.rs | Platform API |
| EX-81 | node_result_cache.rs | Node-Result Cache |
| EX-82 | node_result_cache.rs | Node-Result Cache |
| EX-100 | observability_tracing.rs | Observability & Tracing |
| EX-101 | observability_tracing.rs | Observability & Tracing |
| EX-102 | observability_tracing.rs | Observability & Tracing |
| EX-108 | observability_tracing.rs | Observability & Tracing |
| EX-103 | observability_otel_export.rs | Observability & Tracing |
| EX-105 | eval_scenarios_demo.rs | Evaluation |
| EX-106 | eval_scenarios_demo.rs | Evaluation |
| EX-107 | eval_scenarios_demo.rs | Evaluation |

59 rows total, matching the plan's own stated count.

## Decisions Made

- All three tasks' edits landed in one commit (D-26), per the plan's own explicit
  "Do not commit yet" instruction after Tasks 1 and 2.
- Grouped the 24 new sections into two contiguous insertion zones (before and after
  the existing Performance Benchmarking Examples heading) rather than interleaving
  single sections across the document -- easier to verify and less error-prone, and
  the plan's own acceptance criteria require heading/TOC/grouping correctness, not a
  specific document position.
- Fixed the pre-existing `cli_configs/maneuver.yaml` Demonstrates-line-count
  inconsistency (Rule 1) rather than leaving it -- otherwise Task 3's own
  count-equality `<verify>` command could never pass regardless of how correctly the
  24 new sections were written.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed a pre-existing Demonstrates-line-count inconsistency in the `cli_configs/maneuver.yaml` section**
- **Found during:** Task 3, while computing the both-directions cross-check baseline before adding the thirteen remaining new sections
- **Issue:** `### [cli_configs/maneuver.yaml](cli_configs/maneuver.yaml)` carried a `**Demonstrates:**` bold-line prefix that none of its five sibling `cli_configs/*.yaml` sections use (they all use plain prose). This made the file's total `**Demonstrates:**` line count (39, pre-plan) exceed the on-disk `.rs` file count (38, pre-plan) by one -- a discrepancy that predates this plan and would have made the plan's own Task 3 `<verify>` command (`Demonstrates` count == `.rs` file count) fail no matter how correctly the 24 new .rs sections were written.
- **Fix:** Removed the stray `**Demonstrates:**` prefix and folded its text into the section's plain descriptive sentence, matching every sibling `cli_configs/*.yaml` section's format. No content removed, no snippet block touched.
- **Files modified:** examples/README.md
- **Verification:** `grep -c '^\*\*Demonstrates:\*\*' examples/README.md` now equals `ls examples/*.rs | wc -l` (both 62) after all 24 new sections were added.
- **Committed in:** `c9129dd0` (the plan's single commit)

---

**Total deviations:** 1 auto-fixed (1 bug, discovered while computing the plan's own required cross-check, not assumed from reading the file).
**Impact on plan:** The fix was necessary for the plan's own Task 3 `<verify>` command to be satisfiable at all; no scope change beyond making the file's existing invariant self-consistent.

## Issues Encountered

None beyond the one auto-fixed issue documented above.

## User Setup Required

None - this plan only edits `examples/README.md`; no external service configuration
required.

## Next Phase Readiness

- All five stale `EX-nn` rows (EX-01, EX-33, EX-55, EX-121, EX-122) and all 59 gap
  `EX-nn` rows are now closed across the whole phase (plans 36-01, 36-06 through
  36-10, and this plan). The `examples/README.md` gallery index is complete and
  cross-check-verified in both directions for the first time in this phase.
- Nothing belonging to Phase 36.1 SC2 was absorbed: no entry-point heading was added
  to any public item, `16-DOCS-03-ENTRY-POINTS.md` was not refrozen, and
  `scripts/check-public-api-examples.sh` was not wired into CI or `make`.
- Plan 36-12 (owns CI and `scripts/check-all-examples.sh`) still needs to add the
  `node_result_cache` and `observability_otel_export` build invocations to the
  "Example Muster" CI job and the local examples script, per D-17 -- no CI/script
  edit was made here, per this plan's own scope note.
- No blockers for subsequent Phase 36 plans.

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: examples/README.md
- FOUND: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-11-readme.txt
- FOUND: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-11-SUMMARY.md
- FOUND commit: c9129dd0
