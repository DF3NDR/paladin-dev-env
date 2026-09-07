---
phase: 26-agent-runtime-enhancements
plan: 14
subsystem: llm-provider-ports
tags: [conformance-testing, mockito, openai-compatible, gemini, ollama, transience, redaction, rust]

requires:
  - phase: 26-06
    provides: "response_format on the wire for four adapter paths -- read for context, not modified by this plan"
  - phase: 25 (node-level-fault-tolerance)
    provides: "map_http_status (D-03), the shared non-2xx mapper this suite measures rather than re-implements"
provides:
  - "crates/paladin-llm/src/conformance.rs -- a ConformanceFixture trait and an llm_conformance_suite! macro producing a fixed 8-case list (usage extraction, streaming order + terminal stop, mid-stream error before/after first chunk, dedicated 401/404/400/402 mappings, transience-by-value for 408/429/5xx vs other 4xx, credential redaction, refused redirect), instantiated once each for openai_compatible, gemini and ollama"
  - "A measurement finding: all 24 cells (3 adapters x 8 cases) pass with zero production code changes -- every case the scout expected as a gap was a missing TEST, not missing behavior"
  - "docs/src/getting-started/configuration.md's 'Running against a local Ollama server' section: ollama serve/pull, the OLLAMA_BASE_URL override, a reasoning_agent preview snippet, and the exact cargo test --test ollama_docker --features integration-tests,llm-ollama command"
  - "The G-22 measurement table and the no-second-Ollama-file decision recorded in .project/v0.10.0/08-traceability-matrix.md"
affects: [26-21]

tech-stack:
  added: []
  patterns:
    - "A macro-generated test-case count derived from the SAME token list the #[tokio::test] functions are generated from (recursive macro_rules counting), so a case silently dropped from the list shrinks the count too rather than passing quietly"
    - "A generic conformance case tests via a synthetic sk-prefixed credential rather than each fixture's own configured key, sidestepping a real naming collision (Ollama's placeholder credential is the literal string 'ollama', which also legitimately appears in every ProviderError's Display/Debug as the provider name)"

key-files:
  created:
    - crates/paladin-llm/src/conformance.rs
  modified:
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - docs/src/getting-started/configuration.md
    - .project/v0.10.0/08-traceability-matrix.md

key-decisions:
  - "Each adapter's fixture + macro invocation lives in a nested `mod conformance_suite` inside that adapter's existing `mod tests`, not inline -- so every generated test's full path contains the substring 'conformance', which is what makes `cargo test --lib conformance` (the plan's own acceptance criterion) select all 24 real-adapter cases plus the 9 meta/trivial-fixture cases, not just the ones physically inside conformance.rs."
  - "The credential-redaction case (Test 8) uses a synthetic sk-prefixed token, never a fixture's own configured credential. Ollama's real credential is the fixed, non-secret placeholder \"ollama\" (D-12), which is also the literal provider name every ProviderError's Display/Debug legitimately carries -- asserting the whole rendered string never contains \"ollama\" would fail on the provider name, not a leak. A synthetic token exercises redact_credentials's shape-based sk-/Bearer passes (defense in depth) uniformly across all three fixtures with no such collision."
  - "Test 1's meta fixture (TrivialAdapter) is a fully self-contained, non-feature-gated LlmPort built directly on reqwest and crate::http_status::map_http_status -- not on CompatEngine or any real provider adapter -- so the macro-correctness check compiles and runs under any feature combination paladin-llm is tested with, not only --all-features."
  - "The conformance module is gated on `any(feature = openai, anthropic, deepseek, kimi, qwen, grok, ollama, openai-compatible, gemini)` rather than a bare `feature = \"reqwest\"` cfg, because Cargo's namespaced-features rule suppresses the implicit `reqwest` feature once any `[features]` entry references it via `dep:reqwest` (which `openai` etc. already do)."
  - "Measurement found zero gaps: all three shipped adapters already pass every one of the 8 cases with no production code change. Per D-31's 'measure before fix' protocol, there is therefore only ONE commit for Task 1 (the measurement itself), not a measurement-then-fix pair -- every scout-predicted gap (no 429/5xx-by-value on Ollama, no mid-stream-error on openai_compatible, Gemini 5xx only via map_error units) turned out to be a missing TEST, not missing behavior, because Phase 25's D-03 mapper was already correctly wired into all three non-2xx branches."
  - "The Ollama recipe's reasoning_agent snippet is marked rust,ignore and explicitly notes that paladin::presets/ReasoningAgent do not exist in the tree yet (plans 26-20/26-21 land in later waves) -- written against D-35's exact documented signature, not invented, per the plan's own instruction."

patterns-established:
  - "A shared cross-adapter mockito conformance suite: `ConformanceFixture` (adapter/success_body/stream_body/error_body + a documentation-only Wire enum) + `llm_conformance_suite!` generating one #[tokio::test] per fixed case -- the template for any future adapter (Kimi/Qwen/Grok/DeepSeek/Anthropic/OpenAI) that wants the same conformance bar without re-deriving it."

requirements-completed: [RT-06]

coverage:
  - id: D1
    description: "crates/paladin-llm/src/conformance.rs holds ConformanceFixture and llm_conformance_suite!, producing 8 fixed cases per adapter; instantiated for openai_compatible, gemini and ollama with 24/24 cells measured pass"
    requirement: "RT-06"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib conformance (33 tests: 9 meta + 3 adapters x 8 cases)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib transience_by_value (4 tests)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib credential_never_appears_in_a_rendered_error (4 tests)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib stream_error_after_the_first_chunk (4 tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Transience is asserted by value (LlmError::transience()), never by parsing a rendered message -- confirmed by grep (transience() >= 1, to_string().contains( == 0 in conformance.rs)"
    requirement: "RT-06"
    verification:
      - kind: other
        ref: "grep -c 'transience()' crates/paladin-llm/src/conformance.rs == 3; grep -c 'to_string().contains(' crates/paladin-llm/src/conformance.rs == 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "The measurement commit precedes any adapter production-code edit; no adapter was restructured -- diff on the measurement commit is purely additive test-module code across the three adapter files"
    requirement: "RT-06"
    verification:
      - kind: other
        ref: "git show --stat b0fc657e: openai_compatible/adapter.rs +61/-0, gemini/adapter.rs +61/-0, ollama/adapter.rs +55/-0 (no deletions, no production-code lines touched)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The Ollama recipe documents ollama serve/pull, OLLAMA_BASE_URL, and the exact cargo test --test ollama_docker --features integration-tests,llm-ollama command, states the existing suite is RT-FR-22's artifact, and does not duplicate the config block or add a second Ollama test file"
    requirement: "RT-06"
    verification:
      - kind: other
        ref: "grep -F 'ollama serve' / 'ollama pull' / 'OLLAMA_BASE_URL' / 'cargo test --test ollama_docker --features integration-tests,llm-ollama' docs/src/getting-started/configuration.md; grep -c 'ollama:' == 1; ls tests/integration/ | grep -c ollama == 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "cargo doc --workspace --no-deps exits 0 with no new broken link introduced by this plan's changes"
    requirement: "RT-06"
    verification:
      - kind: other
        ref: "cargo doc --workspace --no-deps (exit 0; pre-existing warnings in unrelated files, none touching conformance.rs or the modified adapter/docs files)"
        status: pass
    human_judgment: false
  - id: D6
    description: "cargo fmt --all --check and cargo clippy --workspace --all-targets --all-features -- -D warnings are clean at the final commit"
    requirement: "RT-06"
    verification:
      - kind: other
        ref: "cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0, Finished dev profile)"
        status: pass
    human_judgment: false

duration: ~1h 45min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 14: LLM Provider Conformance Measurement Summary

**One shared `ConformanceFixture` + `llm_conformance_suite!` macro measures openai_compatible, Gemini and Ollama against an identical 8-case bar (usage extraction, streaming, mid-stream errors, dedicated status mappings, transience-by-value, credential redaction, refused redirects) end-to-end via mockito -- all 24 cells pass with zero adapter code changes, and the Ollama recipe now documents the existing env-gated integration suite instead of adding a second one.**

## Performance

- **Duration:** ~1h 45min
- **Tasks:** 3 (Task 1 auto/tdd, Task 2 checkpoint auto-approved, Task 3 auto)
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- **`crates/paladin-llm/src/conformance.rs`** defines `pub trait ConformanceFixture` (`adapter`,
  `success_body`, `stream_body`, `error_body`, plus a documentation-only `Wire` enum) and
  `#[macro_export] macro_rules! llm_conformance_suite!`, which expands to one `#[tokio::test]` per
  fixed case plus a `CASE_COUNT` const mechanically derived from the same identifier list the
  tests were generated from -- so a case silently dropped from the macro's list shrinks the count
  too, caught by `suite_generates_the_full_case_list_for_a_fixture`'s pinned `assert_eq!(.., 8)`.
- The suite was **instantiated for all three shipped v0.8.0 paths** -- `openai_compatible`,
  `gemini`, `ollama` -- each in a nested `mod conformance_suite` inside that adapter's existing
  test module, using real success/stream/error bodies copied from each adapter's own
  pre-existing hand-written tests (not invented for this suite).
- **Measurement result: 24/24 cells `pass`, zero gaps.** Every one of the scout's three predicted
  gaps (no explicit 429/5xx-by-value case on Ollama, no mid-stream-error case on
  `openai_compatible`, Gemini's 5xx transience asserted only via `map_error` units) turned out to
  be a missing **test**, not missing **behavior** -- Phase 25's `map_http_status` (D-03) was
  already correctly wired into all three adapters' non-2xx branches. No adapter production code
  was touched.
- **Transience is asserted by value** (`LlmError::transience()`) for 408/429/500/502/503
  (Transient) and 403/422 (Permanent) across all three adapters, through the real `generate()`
  path against a mocked non-2xx response -- never by parsing a rendered message
  (`to_string().contains(` count is 0 in `conformance.rs`).
- **Credential redaction and refused-redirect cases pass uniformly**, using a synthetic
  `sk`-prefixed token (not each fixture's own configured credential) so the check is not
  confounded by Ollama's non-secret placeholder credential (`"ollama"`) coinciding with its own
  provider-name literal in `Display`/`Debug` output.
- **The Ollama recipe** ("Running against a local Ollama server") in
  `docs/src/getting-started/configuration.md` documents `ollama serve`/`ollama pull`, points at
  the existing config block (not duplicating it), documents `OLLAMA_BASE_URL`, includes a
  `reasoning_agent` preview snippet marked pending plan 26-21, and gives the exact
  `cargo test --test ollama_docker --features integration-tests,llm-ollama` command with an
  explanation of the local-SKIP / CI-exercises-for-real split. States plainly that
  `tests/integration/ollama_docker_test.rs` is RT-FR-22's artifact and that no second file exists
  or should be added.
- **`.project/v0.10.0/08-traceability-matrix.md`'s G-22 row** now carries the full per-case
  measurement table, the scout-vs-actual comparison, and the no-second-Ollama-file decision.

## Task Commits

1. **Task 1: The shared conformance suite, run as a measurement against the three shipped paths** -- `b0fc657e` (test)
2. **Task 2: Review the per-adapter conformance measurement table and the closed gap set** -- checkpoint, auto-approved (see Checkpoint resolutions below); no commit of its own
3. **Task 3: The Ollama recipe in the guide, pointing at the suite that already exists** -- `de224a36` (docs)

_Note on the plan's "measure, then fix" two-commit protocol: Task 1 produced only ONE commit
because the measurement found zero adapter gaps -- there was nothing to fix in a second commit.
The single commit `b0fc657e` is therefore both the measurement record and the final state; `git
show --stat` on it shows purely additive changes to `conformance.rs` (new file) and the three
adapter files' test modules (61/61/55 added lines, 0 deletions each), confirming no adapter was
restructured._

## Checkpoint resolutions

**Task 2 (`checkpoint:human-verify`, `gate="blocking"`) — auto-approved (auto-mode).** Per the
orchestrator's pre-resolution instruction, this run is in auto-mode and Task 2 carries
`gate="blocking"` (not `blocking-human`), so it was resolved automatically rather than stopped on.
Evidence gathered per the checkpoint's own `<how-to-verify>` steps:

1. **Every cell is `pass`, `gap` or `n.a.` with a reason -- no blanks.** Confirmed: all 24 cells in
   `08-traceability-matrix.md`'s G-22 measurement table read `pass`; the same table is reproduced
   above in this SUMMARY.
2. **Scout's expected gaps compared against what was actually found, stated explicitly.** All
   three expected gaps are recorded in `08-traceability-matrix.md` as "contradicted as a behavior
   gap, confirmed as a pre-existing test-coverage gap" -- i.e. the expectation of a *gap* held in
   the narrow sense that no test previously existed, but was contradicted in the sense that the
   underlying *behavior* was already correct.
3. **`git log --oneline -5` confirms commit order.** `b0fc657e` (Task 1, measurement) precedes
   `de224a36` (Task 3, docs); there is no separate gap-fix commit because none was needed (see
   note above).
4. **Reported test count matches the table.** `cargo test -p paladin-llm --all-features --lib
   conformance` reports `33 passed` (9 meta/trivial-fixture cases + 3 adapters x 8 real cases =
   24), matching the 3x8 table exactly once the meta cases are excluded.
5. **`git show --stat` on the fix commit shows small, additive changes only.** There is no
   separate fix commit; `git show --stat b0fc657e` (the sole Task 1 commit) shows purely additive
   changes, confirmed above.
6. **Residual `n.a.` cells.** None exist -- all 24 cells are `pass`.

## Files Created/Modified

- `crates/paladin-llm/src/conformance.rs` -- new: `ConformanceFixture` trait, `Wire` enum,
  `llm_conformance_suite!` macro, the 8 case functions (`cases` module), and a self-contained
  `TrivialAdapter`-based meta-test proving the macro itself is correct
- `crates/paladin-llm/src/lib.rs` -- registers `mod conformance` behind a feature-aware `cfg`
- `crates/paladin-llm/src/openai_compatible/adapter.rs` -- adds `mod conformance_suite` with
  `OpenAiCompatibleFixture` and the macro invocation
- `crates/paladin-llm/src/gemini/adapter.rs` -- adds `mod conformance_suite` with `GeminiFixture`
  and the macro invocation
- `crates/paladin-llm/src/ollama/adapter.rs` -- adds `mod conformance_suite` with `OllamaFixture`
  and the macro invocation
- `docs/src/getting-started/configuration.md` -- new "Running against a local Ollama server"
  section
- `.project/v0.10.0/08-traceability-matrix.md` -- G-22 row updated; new measurement table,
  scout-vs-actual comparison, and no-second-file decision recorded

## Decisions Made

See `key-decisions` in the frontmatter above for the full list. In summary: fixtures nest in a
`conformance_suite` submodule so `--lib conformance` selects them by path substring; the
credential test uses a synthetic token rather than each fixture's real credential to avoid a
false failure on Ollama's provider-name-colliding placeholder key; the meta fixture is fully
self-contained (no `CompatEngine` dependency) so it compiles under any feature set; the
`conformance` module is feature-gated on the specific set of features that pull in `reqwest`
(Cargo's namespaced-features rule suppresses the bare `reqwest` implicit feature here); and the
measurement found zero gaps, so only one commit exists for Task 1.

## Deviations from Plan

**1. [Design decision, not a Rule 1-4 deviation] Fixtures nested in `mod conformance_suite`
rather than directly in each adapter's `mod tests`.** The plan's acceptance criteria require
`cargo test -p paladin-llm --all-features --lib conformance` to report at least 24 tests. A test's
full path must contain the literal substring "conformance" for `--lib conformance` to select it;
placing the fixture + macro invocation directly inside `mod tests` (as first drafted) produced
tests named e.g. `openai_compatible::adapter::tests::generate_and_usage_extraction`, which
`--lib conformance` does NOT match. Wrapping each in a nested `mod conformance_suite { .. }` fixed
this without changing any test's behavior. Discovered and fixed during Task 1's own verification
step, before commit.

**2. [Rule 1 - Bug, caught during authoring] Fixed a copy-paste bug in the `transience_by_value`
case draft.** An early draft of this case (never committed) contained a leftover
`unreachable!()` from iterating on the design. Caught by reading the file back before running
tests; corrected to the intended `match result { Err(e) => e, Ok(_) => panic!(..) }` plus the
`assert_eq!(err.transience(), expected, ..)` check. No behavior shipped incorrectly; the bug never
reached a commit.

No other deviations -- Task 1 and Task 3 executed per the plan's `<action>` and
`<acceptance_criteria>` sections.

## Issues Encountered

None beyond the two items above (both caught and corrected before any commit).

## Known Stubs

None. Every case in the suite drives a real adapter's real `generate()`/`generate_stream()` path
against a real (mockito) HTTP server; nothing is mocked at the `LlmPort` level except the
meta-test's own `TrivialAdapter`, which exists solely to prove the macro's own correctness and is
explicitly documented as such.

## User Setup Required

None -- no external service configuration required. The live Ollama tier remains CI-only
(`ollama-integration` job); Docker is unavailable in this devcontainer and no live Ollama test was
run locally, as directed by D-32/D-39.

## Next Phase Readiness

- The conformance suite pattern (`ConformanceFixture` + `llm_conformance_suite!`) is available for
  any future adapter (Kimi, Qwen, Grok, DeepSeek, Anthropic, OpenAI) that wants the same bar
  without re-deriving it -- not exercised in this plan (Claude's discretion, per D-31, was to
  measure only the three v0.8.0 paths named in the plan).
- Plan 26-21 (the `agent-runtime` user guide) can replace this plan's `rust,ignore` `reasoning_agent`
  preview snippet with the real, doc-tested example once `paladin::presets::reasoning_agent` and
  `ReasoningAgent` land.
- No blockers.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

- FOUND: crates/paladin-llm/src/conformance.rs
- FOUND: crates/paladin-llm/src/lib.rs (modified, `mod conformance` registered)
- FOUND: crates/paladin-llm/src/openai_compatible/adapter.rs (modified, `mod conformance_suite` present)
- FOUND: crates/paladin-llm/src/gemini/adapter.rs (modified, `mod conformance_suite` present)
- FOUND: crates/paladin-llm/src/ollama/adapter.rs (modified, `mod conformance_suite` present)
- FOUND: docs/src/getting-started/configuration.md (modified, "Running against a local Ollama server" section present)
- FOUND: .project/v0.10.0/08-traceability-matrix.md (modified, G-22 measurement table present)
- FOUND commit `b0fc657e` in `git log --oneline`
- FOUND commit `de224a36` in `git log --oneline`
- `cargo test -p paladin-llm --all-features --lib conformance`: 33 passed, 0 failed
- `cargo fmt --all --check`: clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean
- `cargo doc --workspace --no-deps`: exit 0
