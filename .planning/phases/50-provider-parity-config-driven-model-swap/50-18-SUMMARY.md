---
phase: 50-provider-parity-config-driven-model-swap
plan: 18
subsystem: llm
tags: [rust, paladin-core, paladin-ports, paladin-llm, run-metadata, streaming, semver, adr-0061, parity]

requires:
  - phase: 50
    provides: "50-03/50-05 params_omitted, unprofiled_model and response_format_adjusted keys on LlmResponse.metadata; 50-11 additive-field and atomic literal-migration pattern; 50-12/50-13 StreamCompletion terminal bookkeeping; 50-14 served_by / served_model keys and per-step fallback; 50-15 the 17-case conformance suite"
provides:
  - "ADR-0061 R-1 answered option-b (owner, 2026-10-16) and subsection (h) rewritten for the chosen reach"
  - "PaladinResult.run_metadata: BTreeMap<String, String> and the private PaladinExecutionService::merge_run_metadata helper"
  - "StreamingResponse.metadata and StreamingResponse::with_metadata; five stream paths (compat engine, OpenAI, DeepSeek, Anthropic, Gemini) stamp the terminal chunk"
  - "FallbackLlmAdapter streams stamp paladin.served_by and paladin.served_model on the terminal chunk"
  - "ChunkMetadata.run_metadata and ChunkMetadata::with_run_metadata: the final chunk of a streamed execution-service run carries the merged keys"
affects: [50-16]

tech-stack:
  added: []
  patterns:
    - "One merge rule for buffered and streamed runs: union (first-seen, de-duplicated) for params_omitted and unprofiled_model, last value for every other paladin.* key, served_by never duplicated into PaladinResult, non-paladin. keys never copied"
    - "A field on a constructible struct lands in the same commit as every full literal that needs it (A3-50-18), as 50-11 did for LlmResponse.reasoning"
    - "An additive field on a #[non_exhaustive] struct is measured with cargo-semver-checks (crate-wide allows disabled, restored byte-for-byte) and fires no lint"

key-files:
  created: []
  modified:
    - .planning/decisions/0061-per-model-parameter-support-policy.md
    - crates/paladin-core/src/platform/container/execution_result.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/fallback.rs
    - tests/cli/environment_tests.rs
    - tests/unit/handoff_service_test.rs
    - tests/consumer_surface_muster.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - CHANGELOG.md

key-decisions:
  - "Owner reading R-1 = option-b (2026-10-16): the run-metadata entry reaches every caller, buffered and streamed"
  - "A streamed execution-service run assembles no PaladinResult (its result is the final PaladinStreamChunk), so the streamed half of the merge lands on ChunkMetadata.run_metadata, additive on a #[non_exhaustive] struct"
  - "ChunkMetadata.run_metadata also carries paladin.served_by, because a stream chunk has no dedicated field for it (PaladinResult has served_by)"
  - "The fallback stream stamps served_by and served_model on its terminal chunk, so a streamed fallback call is as visible as a buffered one"
  - "The existing paladin-ai-core | PaladinResult 9.2 row and allowlist entry are extended; no new pair"

patterns-established:
  - "Stream metadata is stamped on the terminal chunk inside the existing [DONE] / message_stop / finish-reason branch, after StreamCompletion has judged it Ok; an Err terminal carries none"

requirements-completed: [PARITY-02, PARITY-04, PARITY-06]

coverage:
  - id: D1
    description: "R-1 is an owner decision on record in ADR-0061 with its date and rating; (h) states the chosen reach"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "Task 1 automated verify (exactly one R-1 row, names option-b, carries 2026-10-16, no pending) passed before any other edit"
        status: pass
    human_judgment: true
  - id: D2
    description: "PaladinResult.run_metadata merges every call's paladin.* keys by the ADR-0061 (h) rule and serialises byte-identically when empty"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "paladin_execution_service::tests::run_metadata_collects_the_paladin_keys_of_every_call, run_metadata_is_empty_when_no_call_stamped_anything, merge_run_metadata_follows_the_adr_0061_rule; execution_result::tests::paladin_result_without_run_metadata_key_deserializes_to_empty, run_metadata_round_trips_when_present, default_and_new_leave_run_metadata_empty"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every stream path stamps the terminal chunk only, and an Err terminal carries none"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "compat_stream_terminal_chunk_carries_metadata, openai_/deepseek_/anthropic_/gemini_stream_terminal_chunk_carries_metadata, compat_stream_terminal_chunk_has_no_metadata_when_nothing_was_adjusted, fallback::tests::the_fallback_stream_stamps_served_by_and_served_model_on_its_terminal_chunk; conformance_suite 153 tests (17 cases x 9 fixtures) still green"
        status: pass
    human_judgment: false
  - id: D4
    description: "A streamed execution-service run delivers the merged keys on its final chunk"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "paladin_execution_service::tests::streamed_run_merges_the_terminal_chunk_keys_into_run_metadata, streamed_run_without_stamped_keys_has_empty_run_metadata; paladin_port::tests::test_chunk_metadata_with_run_metadata_carries_the_visibility_keys"
        status: pass
    human_judgment: false
  - id: D5
    description: "The PaladinResult break is measured, registered by extending the existing 9.2 row and entry, and the gates pass"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "D-27 run: cargo semver-checks 0.50.0, paladin-ai-core, baseline 0.10.1, minor, allows disabled and restored (cmp): constructible_struct_adds_field on PaladinResult.run_metadata beside PaladinResult.cost; make check-migration-allowlist, make check-consumer-surface, make check-gates, make api-surface all exit 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "The additive StreamingResponse.metadata and ChunkMetadata.run_metadata fields are not breaking"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo semver-checks, paladin-ports, baseline 0.10.1, minor, crate-wide allows disabled and restored (cmp): three pre-existing unrelated failures, none names StreamingResponse, ChunkMetadata or metadata"
        status: pass
    human_judgment: false

duration: ~65min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 18: Owner reading R-1 -- run metadata reaches every caller Summary

**Owner decision (Task 1, recorded verbatim, dated 2026-10-16):** "option-b: the run-metadata entry reaches every caller, buffered and streamed; PaladinResult.run_metadata and the streaming terminal chunk carry the paladin.* keys. One registered break taken before the consumer's single move to v0.11.0."

**The `paladin.*` visibility keys now reach the run result (`PaladinResult.run_metadata`), every stream's terminal chunk (`StreamingResponse.metadata`) and the final chunk of a streamed run (`ChunkMetadata.run_metadata`), merged by one rule, with one registered break on `PaladinResult`.**

## Performance

- **Duration:** about 65 min (continuation agent, after the owner's Task 1 answer)
- **Tasks:** 3 of 3 (Task 1 a decision with no executor commit; Tasks 2 and 3 one commit each)
- **Files:** 18 modified, 0 created

## Accomplishments

- **Task 1 (decision).** The first edit made was ADR-0061's R-1 row: the `pending` Answer cell became `2026-10-16: option-b -- "<owner wording>"`, keeping the `one-way` rating and the `plan 50-18 Task 1` reference. The Task 1 automated verify passed (one R-1 row, names option-b, carries a date, no `pending`). No separate commit; the row landed in the Task 2 commit, as the plan says ("Task 2 rewrites subsection (h) and commits the ADR").
- **Task 2.** `PaladinResult.run_metadata: BTreeMap<String, String>` after `served_by` (`#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`, rustdoc, `Default` and `new` empty). `PaladinExecutionService::merge_run_metadata` (private) folds each call's `paladin.*` keys beside the `served_by` copy. 17 full literals migrated in the same commit. The existing `paladin-ai-core | PaladinResult` 9.2 row and allowlist entry were extended from a measured D-27 run. ADR-0061 (h) rewritten.
- **Task 3.** `StreamingResponse.metadata` with `with_metadata` (doc-tested). The compat engine (Kimi, Qwen, Grok, Ollama, generic OpenAI-compatible), OpenAI, DeepSeek, Anthropic and Gemini stamp the terminal chunk from `RequestNotes::into_metadata()`. The execution service's streamed run merges the terminal chunk's keys into `ChunkMetadata.run_metadata`.

## Task Commits

1. **Task 1** - no commit (a decision; the R-1 row is in the Task 2 commit)
2. **Task 2** - `424caa9c` (feat: owner reading R-1 -- run metadata reaches the run result)
3. **Task 3** - `436fbb7f` (feat: streamed calls carry the run metadata on their terminal chunk)

**Plan metadata:** the docs commit carrying this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## The migration list (A3-50-18)

The plan's literal check, run on the tree immediately before editing, printed 21 literals in 7 files:

| File | Literals | Migrated |
|------|----------|----------|
| `src/application/services/paladin/paladin_execution_service.rs` | 4 (1765, 1851, 2138, 2166) | 4 (`run_metadata: collected_metadata`) |
| `crates/paladin-ports/src/output/paladin_port.rs` | 9 (306, 648, 894, 971 are doc comments using `..Default::default()`; 1256, 1334, 1382, 1419, 1463 are test literals) | 5 test literals; the 4 doc examples need no change |
| `crates/paladin-core/src/platform/container/execution_result.rs` | 2 (28 doc example, 307 test) | 2 |
| `crates/paladin-battalion/src/engine/test_support.rs` | 1 (583) | 1 |
| `tests/consumer_surface_muster.rs` | 1 (153) | 1 |
| `tests/unit/handoff_service_test.rs` | 2 (405, 469) | 2 |
| `tests/cli/environment_tests.rs` | 2 (1157, 1201; the module is commented out in `tests/cli/mod.rs`) | 2 |

No further hit appeared. The Task 2 commit lists `execution_result.rs` and every file on the pre-edit list that needed a change; `cargo build --workspace --all-targets --all-features` passed at that commit.

## Measurements

- **D-27, `paladin-ai-core`** (`cargo semver-checks check-release --package paladin-ai-core --baseline-version 0.10.1 --release-type minor --default-features`, 0.50.0, the ten crate-wide allow lines removed, manifest restored and proved with `cmp`): `constructible_struct_adds_field` fires on `PaladinResult.run_metadata` beside the earlier `PaladinResult.cost`, and no other lint names the field. The eleven lints that fire are the already-registered Phase 25-48 items plus two pre-existing non-exhaustive-enum items (`RunFinishStatus::AwaitingInput` discriminant, `TraceEvent` variant fields) that this plan did not touch. The existing crate-wide allow and the existing `paladin-ai-core | PaladinResult` pair cover it, so the row was extended with `**Extended, Phase 50 (PARITY-02, owner reading R-1):**` (the plan's grep prints 1) and the entry's justification extended; no pair added.
- **`paladin-ports`** (same method, allows disabled and restored byte-for-byte): three pre-existing failures, none mentions `StreamingResponse`, `ChunkMetadata` or a `metadata` field. The additive fields on the two `#[non_exhaustive]` structs are not breaking.
- **Gates:** `make check-migration-allowlist`, `make check-consumer-surface`, `make check-gates`, `make api-surface` (unchanged, 4298 items, so `.project/current-exports.txt` needed no edit) all exit 0.
- **Workspace:** `cargo test --workspace --lib --bins --no-fail-fast` gives 5069 passed, 2 failed, the two known WINDOWS row 73 baseline failures (5051 before + 18 new tests under default features; the `paladin-llm` adapter stream tests need `--all-features` and add 7 more there). `cargo clippy --workspace --all-features --all-targets -- -D warnings` and `cargo fmt --check` are clean. `conformance_suite` is 153 tests (`CASE_COUNT` 17), all green.

## Deviations from Plan

### Auto-fixed / auto-added

**1. [Rule 2 - Missing critical functionality] A streamed execution-service run has no `PaladinResult` to merge into**
- **Found during:** Task 3, reading `execute_stream_inner`.
- **Issue:** the plan says the streaming path merges the terminal chunk's metadata "into its `PaladinResult`", but a streamed run produces `PaladinStreamChunk`s whose final chunk carries `ChunkMetadata`; there is no `PaladinResult`. Without a carrier the owner's "buffered and streamed" reach would stop at the provider stream.
- **Fix:** `ChunkMetadata.run_metadata: BTreeMap<String, String>` and `with_run_metadata` (additive on the `#[non_exhaustive]` struct, skipped when empty), populated on the final chunk through `merge_run_metadata`. `paladin.served_by` is carried in it too, because the chunk has no `served_by` field (no `paladin.*` key is dropped, per the PARITY-02 prohibition). The `paladin-web` SSE `done` event maps selected fields only and is untouched (A2-50-18 holds).
- **Files modified:** `crates/paladin-ports/src/output/paladin_port.rs` (not in the plan's file list), `src/application/services/paladin/paladin_execution_service.rs`.
- **Commit:** `436fbb7f`

**2. [Rule 2 - Missing critical functionality] The fallback stream did not name the serving hop**
- **Found during:** Task 3 (the 50-14 SUMMARY noted `paladin.served_model` could not reach streams).
- **Issue:** `FallbackLlmAdapter::generate` stamps `paladin.served_by` and `paladin.served_model`; its `generate_stream` stamped nothing, so a streamed call in the composed server stack lacked the fallback keys the owner's reading covers.
- **Fix:** the stream returned by `generate_stream` stamps both keys on the terminal chunk of the hop that served it, content chunks untouched.
- **Files modified:** `crates/paladin-llm/src/fallback.rs` (not in the plan's file list).
- **Commit:** `436fbb7f`

**3. [Rule 3 - Plan check cannot see doc-comment literals] Four literals the plan's check still lists**
- **Found during:** Task 2, re-running the plan's literal check before the commit.
- **Issue:** `paladin_port.rs` lines 306, 648, 894, 971 are rustdoc examples that end in `..Default::default()`. The check's functional-update clause cannot see through the `//!` / `///` prefix, so it reports them although they compile unchanged (`cargo test -p paladin-ports --doc` passes). The plan's acceptance wording ("prints nothing") is therefore not literally satisfied.
- **Fix:** none needed in code; the four are functional-update literals and gain the field by `Default`. `execution_result.rs` line 28 (a doc example) was migrated anyway so the field is shown.
- **Commit:** `424caa9c`

### Form deviations (no behaviour change)

- The migrated literals read `run_metadata: std::collections::BTreeMap::new()` (fully qualified) instead of adding a `use` to six files; the execution service's four literals read `run_metadata: collected_metadata` (the local is named so the literal keeps the `run_metadata:` shape the plan's check looks for, and clippy's redundant-field-name lint stays quiet).
- `.project/current-exports.txt` is listed in the plan's files but was not edited: `make api-surface` reports the surface unchanged (it tracks root-crate re-exports only).

### Process notes

- **Tasks 2 and 3 are TDD, with one commit each.** The plan forbids splitting Task 2 (A3-50-18: the field, merge and every literal land together so every commit compiles), so there is no separate RED commit. RED was observed before each implementation: compile failure for `PaladinResult.run_metadata`, `StreamingResponse.metadata`, `ChunkMetadata.run_metadata`; assertion failures for the compat, OpenAI, Anthropic, Gemini and fallback stream cases and for the streamed execution-service case. The DeepSeek stream test was written together with its implementation, so no separate failing run was observed for it.
- **Disk.** The workspace build filled the disk once; `target/debug/incremental`, `target/debug/examples`, `target/semver-checks` and the extensionless test binaries were deleted as the brief allows, and builds ran with `CARGO_INCREMENTAL=0`. `cargo clean` was not used.
- **Known baseline.** The two `run_api_wiring` failures (WINDOWS row 73) are unchanged and untouched.

## Threat Flags

None. `run_metadata` and the stream `metadata` carry only `paladin.`-prefixed keys whose writers put provider names, model ids and parameter names in them (A1-50-18, T-50-18-01); a merge test asserts a foreign `x-request-id` key is never copied, and `ExecuteResponse` and the SSE `done` event are not extended. Manual credential-handling review (D-00a): no credential-bearing value reaches `run_metadata` or a chunk's `metadata`.

## Known Stubs

None.

## Self-Check: PASSED

- `424caa9c` and `436fbb7f` exist in `git log`.
- Created/modified files exist; the `grep -c 'pub run_metadata: BTreeMap<String, String>'` on `execution_result.rs` prints 1; the 9.2 `Extended, Phase 50 (PARITY-02, owner reading R-1)` grep prints 1.
- Task 1 automated verify, `make check-migration-allowlist`, `make check-consumer-surface`, `make check-gates`, `make api-surface`, `cargo fmt --check`, workspace clippy `-D warnings` all pass; workspace lib/bin tests show only the two baseline failures.
