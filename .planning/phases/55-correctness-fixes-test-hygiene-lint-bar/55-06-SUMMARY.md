---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 06
subsystem: paladin-content
tags: [content-ingestion, planning, fences, fixes-02, d-03]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-01..55-05 landed; no code dependency"
provides:
  - "FileContentFetcher::ingest_content refuses with a named Err after validation instead of reporting Ok(()) for an ingestion it never performed"
  - "PlanningService::create_plan builds every Subtask with an empty expected_output; the invented placeholder sentence appears in no plan and no prompt"
  - "The subtask prompt omits its EXPECTED OUTPUT line and the focus-on-expected-output closing when expected_output is empty or whitespace, and keeps today's text byte-for-byte when a caller supplies one"
  - "rustdoc stating the unfinished / empty-means-unspecified rules on the impl, the three metadata helpers, the planning module and Subtask.expected_output"
affects: [55-12, 55-13]

tech-stack:
  added: []
  patterns:
    - "A fence refuses or states nothing; it never reports a fabricated success or injects fabricated text"

key-files:
  created: []
  modified:
    - crates/paladin-content/src/adapters/input/file_content_fetcher.rs
    - src/application/services/paladin/planning_service.rs
    - crates/paladin-core/src/platform/container/planning.rs

key-decisions:
  - "PD-06-1: ingest_content refuses (the caller read found no dependence on the no-op Ok(())); validation runs first so a URL-less item keeps its specific error"
  - "PD-06-2: expected_output is the empty string, not the description, which is already in the prompt as SUBTASK:"
  - "The refusal text lives in a private const so the exact sentence is greppable on one line; the old placeholder sentence is built with concat! in the test guard so the source holds no copy outside it"

requirements-completed: [FIXES-02]

duration: 25min
completed: 2026-11-01
---

# Phase 55 Plan 06: Content-processing stub fences Summary

**`ingest_content` on the file fetcher now refuses with a named error instead of reporting a stored item that was never stored, and planned subtasks carry an empty `expected_output` with a prompt that omits the empty line.**

Plan-start SHA: `998f93d5652dff4fe5da0cad10b4fa9dba4a5f8e`

## Performance

- Tasks: 2 of 2
- Task commits: `75be7db94` (Task 1, tracer), `290ec10b4` (Task 2)

## Accomplishments

- `FileContentFetcher::ingest_content` keeps its per-type URL validation, then returns `Err("FileContentFetcher::ingest_content is unfinished: content ingestion is not implemented, nothing was stored")`. `fetch_content` is untouched.
- The three private metadata helpers still return `None`; their inline to-do comments became a `///` line each saying not implemented, always `None`.
- `create_plan` no longer fabricates an expected output; `execute_subtask` builds its prompt from two shapes and omits the `EXPECTED OUTPUT:` line and the "focused on the expected output" clause when none is set (RESEARCH Pitfall 4).
- Three new tests plus the `RecordingLlmPort` double (wraps `MockLlmPort`, records every `LlmRequest`).

## Caller read for `ingest_content` (evidence for plan 55-12)

Command: `grep -rn 'ingest_content' crates src tests examples docs`. Every hit:

| Hit | What it is | Depends on the no-op `Ok(())`? |
|-----|------------|--------------------------------|
| `crates/paladin-ports/src/input/content_input_port.rs:20` | trait doc example `round_trip`, compile-only generic over `&dyn ContentIngestionPort` | No |
| `crates/paladin-ports/src/input/content_input_port.rs:27` | the trait method declaration, `Result<(), String>` | No (signature unchanged) |
| `crates/paladin-content/src/adapters/input/file_content_fetcher.rs:152` | this impl | It is the subject |
| `crates/paladin-content/src/adapters/input/file_content_fetcher.rs:308-323` | its own test, flipped by this plan | No |
| `src/application/services/content/content_ingestion_service.rs:198, 536, 879` | a different, async `ingest_content(String, metadata)` on `ContentIngestionService` returning `IngestionResult`; not the port | No (unrelated method with the same name) |
| `docs/src/contributing/testing-guide.md:406` | a documentation snippet calling `app.ingest_content` on an `Application`, a different API | No |

Related, found by name rather than by `ingest_content`: `tests/content_ingestion_pipeline.rs:35` imports `ContentIngestionPort` and only calls `fetch_content` (l.62); `docs/src/user-guides/content-processing.md:70` documents `fetch_content` only; `docs/src/api-reference/stable-api.md:519` lists the trait. No shipped caller of `ContentIngestionPort::ingest_content` exists outside the trait doc example, so D-03's refuse branch applies and no `warn!` branch was needed. The `content_ingestion_pipeline` target (`--features content-processing`) still passes (1 passed, 1 ignored for network).

## `expected_output` choice

Empty string. The description is already sent as `SUBTASK:`; repeating it as an expected output would be a second fabrication, and asking the LLM for one is out of scope (D-00d). The `Subtask::new` doctest (`Analysis report`) is untouched and passes.

## Watched failing (prove-then-lock)

- Task 1: `cargo test -p paladin-content --offline --lib file_content_fetcher` after the flip: `test_ingest_content_refuses_valid_content_as_unfinished` FAILED (left `Ok(())`, right the refusal); 4 passed, 1 failed.
- Task 2: `cargo test -p paladin-ai --lib --offline planning_service` before the fix: `create_plan_does_not_fabricate_an_expected_output` and `subtask_prompt_omits_expected_output_when_none_is_set` FAILED; `subtask_prompt_keeps_a_supplied_expected_output` passed (it guards today's behaviour, as the plan intends). After the fix: 14 passed, 0 failed.

## Verification

- `cargo test -p paladin-content --offline --lib`: 75 passed.
- `cargo test -p paladin-ai --lib --offline application::services::paladin`: 328 passed.
- `cargo test -p paladin-ai-core --offline --doc planning`: 2 passed.
- `cargo fmt --check`: clean. `cargo clippy -p paladin-content --tests` and `-p paladin-ai --lib --tests --offline -- -D warnings`: clean.
- `cargo doc -p paladin-ai-core --no-deps --offline`: 0 warnings.
- `make api-surface`: unchanged (4325 items). `git diff 998f93d56 -- crates/paladin-ports` is empty; the `planning.rs` diff touches only `///` lines; no `pub fn` signature changed.
- Acceptance greps: `fn test_ingest_content_validation` count 0; `Ok(())` inside `fn ingest_content` count 0; `Expected output from subtask execution` count 0 in `planning_service.rs`.

## Task Commits

1. Task 1 (tracer): `75be7db94` fix(55-06): FileContentFetcher::ingest_content refuses instead of reporting a no-op success
2. Task 2: `290ec10b4` fix(55-06): planned subtasks carry no fabricated expected output and the prompt omits the empty line

The tracer gate ran in auto-chain mode: the Task 1 `<verify>` was re-run end to end and passed before Task 2 began.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Verify grep needs the refusal sentence on one source line**
- **Found during:** Task 1
- **Issue:** rustfmt and the 100-column convention split the refusal literal over a `\` continuation, so the plan's `grep -q 'ingest_content is unfinished: content ingestion is not implemented, nothing was stored'` could not match.
- **Fix:** the sentence is a private `const INGEST_CONTENT_UNFINISHED: &str` on one line (rustfmt does not break string literals); the test asserts the same sentence as a literal.
- **Commit:** `75be7db94`

**2. [Rule 3 - Blocking] The acceptance grep for the old placeholder sentence would have matched the guard test**
- **Found during:** Task 2
- **Issue:** the plan requires `grep -c 'Expected output from subtask execution'` to be 0 in `planning_service.rs`, while the test must assert the sentence is absent from plans and prompts.
- **Fix:** the tests use a `FORMER_PLACEHOLDER` const built with `concat!` from two pieces, so the guard still pins the old sentence while the file holds no copy of it.
- **Commit:** `290ec10b4`

**3. [self-inflicted, fixed before commit] a slicing slip duplicated part of `planning_service.rs` mid-edit**
- A scripted edit located the end of the prompt block at an earlier `UserPrompt` occurrence, producing duplicate definitions. The file was rebuilt from its own content in the same session, `git diff` showed only the intended hunks, and the tests were re-run green. Nothing wrong was committed.

No other deviations. No stubs were added, so no WINDOWS.md rows. No new threat surface; T-55-15 and T-55-16 are mitigated as planned.

## Issues Encountered

None blocking. Disk stayed near 6.4 GB free; `CARGO_INCREMENTAL=0` was set and no cleanup of `target/` was needed.

## Self-Check: PASSED

Both modified files and the three commit hashes were verified present.
