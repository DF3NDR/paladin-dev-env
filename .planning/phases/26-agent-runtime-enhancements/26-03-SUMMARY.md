---
phase: 26-agent-runtime-enhancements
plan: 03
subsystem: llm-provider-ports
tags: [llm-request, non-exhaustive, semver, builder-pattern, response-format, structured-output, rust]

requires:
  - phase: 26-01
    provides: "MockLlmAdapter request-recording extension used by the new inertness test"
provides:
  - "LlmRequest::new(model, prompt) constructor plus chainable with_attachments/with_stream/with_metadata/with_response_format builders, all doc-tested"
  - "ResponseFormat #[non_exhaustive] enum (JsonObject, JsonSchema { name, schema, strict }) on LlmRequest.response_format: Option<ResponseFormat>"
  - "LlmRequest marked #[non_exhaustive] with the struct_marked_non_exhaustive semver-checks suppression, allowlist entry, and MIGRATION.md §9.2 row resolved Y"
  - "All 36 downstream LlmRequest construction sites migrated to the constructor; zero struct literals remain outside paladin-ports"
  - "all_adapters_ignore_response_format_until_wired inertness test proving response_format has no observable effect until plan 26-06 wires native JSON modes"
affects: [26-06, 26-21]

tech-stack:
  added: []
  patterns:
    - "X-10.3 option (a) for a Default-less struct: #[non_exhaustive] + constructor/builder is the only viable treatment (contrasted with Phase 25 D-26's option (b), which required a Default to preserve functional-update syntax)"
    - "Compile-time exhaustive-literal test as the guard against a prohibited field addition (provider_capabilities_gained_no_field)"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-ports/Cargo.toml
    - .cargo/semver-checks-allowlist.toml
    - MIGRATION.md
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/fallback.rs
    - crates/paladin-llm/src/mock.rs
    - crates/paladin-llm/src/llm_analysis_service.rs
    - crates/paladin-llm/src/anthropic/vision.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/benches/llm_serialization_benchmarks.rs
    - crates/paladin-llm/examples/live_vendor_smoke.rs
    - crates/paladin-battalion/src/commander.rs
    - crates/paladin-battalion/src/grove_service.rs
    - crates/paladin-battalion/src/llm_decision.rs
    - crates/paladin-memory/src/services/memory_extraction_service.rs
    - crates/paladin-ports/src/output/vision_llm_port.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/paladin/planning_service.rs
    - src/application/services/paladin/prompt_generation_service.rs
    - src/application/services/paladin/temperature_service.rs
    - tests/helpers/mock_llm_adapter.rs
    - tests/integration/anthropic_provider_test.rs
    - tests/integration/cli_real_providers_test.rs
    - tests/integration/deepseek_provider_test.rs
    - tests/integration/llm_live_api_tests.rs
    - tests/integration/ollama_docker_test.rs
    - tests/integration/openai_content_analysis_integration_test.rs
    - tests/integration/openai_provider_test.rs
    - tests/integration/provider_switching_test.rs
    - tests/integration/vision_integration_test.rs
    - tests/unit/llm/anthropic_adapter_test.rs
    - tests/unit/llm/deepseek_adapter_test.rs
    - tests/unit/mock_llm_adapter_test.rs

key-decisions:
  - "Checkpoint auto-resolved to option-a-as-locked (see Checkpoint resolutions below)"
  - "The three integration-test sites that pinned id: prompt.uuid() (anthropic/openai/deepseek 'simple completion' tests) switch to the constructor's fresh id: their own assertions only check self-consistency (captured request_id equals response.request_id from the same built request), never the id's specific value, so no public API surface was added to preserve a pinned id no test actually needed"
  - "crates/paladin-ports/Cargo.toml's suppression table uses the hyphenated [package.metadata.cargo-semver-checks.lints] key (matching the pre-existing, CI-proven Phase 25 suppression in the same file), not the underscored cargo_semver_checks the plan's action text and acceptance criteria literally name -- confirmed functional by a live cargo semver-checks run (see Deviations)"

patterns-established:
  - "New request-hint enum fields land as #[non_exhaustive] from day one beside the struct they extend, documented as a hint an unsupporting adapter ignores harmlessly rather than a contract"

requirements-completed: [RT-05]

coverage:
  - id: D1
    description: "LlmRequest::new(model, prompt) produces a fresh id per call, empty attachments, stream: false, empty metadata, response_format: None -- documented and doc-tested"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/llm_port.rs#tests::llm_request_new_sets_documented_defaults, ::llm_request_new_generates_a_fresh_id_each_call"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ports --doc (LlmRequest::new doc test)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Chainable with_attachments/with_stream/with_metadata/with_response_format builders; chaining the same setter twice replaces rather than accumulates (last-write-wins)"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/llm_port.rs#tests::builder_methods_chain_and_last_write_wins"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ports --doc (with_attachments/with_stream/with_metadata/with_response_format doc tests)"
        status: pass
    human_judgment: false
  - id: D3
    description: "ResponseFormat is #[non_exhaustive] with exactly JsonObject and JsonSchema { name, schema, strict }; both variants round-trip through serde with equality, and an absent response_format key deserializes to None"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/llm_port.rs#tests::response_format_round_trips_through_serde_and_defaults_to_none, ::response_format_carries_both_variants"
        status: pass
    human_judgment: false
  - id: D4
    description: "ProviderCapabilities gains no field (compile-time guard: exhaustive struct literal stops compiling the moment a field is added)"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/llm_port.rs#tests::provider_capabilities_gained_no_field"
        status: pass
    human_judgment: false
  - id: D5
    description: "LlmRequest marked #[non_exhaustive]; the MIGRATION.md §9.2 row, .cargo/semver-checks-allowlist.toml entry, and crates/paladin-ports/Cargo.toml lint suppression agree, in one commit"
    requirement: "RT-05"
    verification:
      - kind: other
        ref: "grep -B3 'pub struct LlmRequest' crates/paladin-ports/src/output/llm_port.rs | grep non_exhaustive; grep LlmRequest MIGRATION.md | grep -c TBD (0); set-equality script from ci.yml's 'Verify allowlist is set-equal' step run locally (SET_EQUAL)"
        status: pass
      - kind: other
        ref: "cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0 -- 'no semver update required' (suppression silences the lint at the tool level, per X-10.6)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every LlmRequest construction site outside paladin-ports migrated to the constructor; zero struct literals remain; workspace compiles under --all-targets --all-features with zero warnings"
    requirement: "RT-05"
    verification:
      - kind: other
        ref: "grep -rn 'LlmRequest {' --include=*.rs . | grep -v '\\->' -- only crates/paladin-ports/src/output/llm_port.rs's declaration and impl-block lines remain (see Deviations for the raw grep's false-positive on fn signatures)"
        status: pass
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0, zero warnings); cargo fmt --all --check; cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib (368 passed), cargo test -p paladin-battalion --lib (721 passed), cargo test -p paladin-memory --all-features --lib (101 passed)"
        status: pass
    human_judgment: false
  - id: D7
    description: "response_format is proven inert on the wire until plan 26-06 wires it -- an adapter with the field set behaves identically to one without it"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "tests/unit/mock_llm_adapter_test.rs#all_adapters_ignore_response_format_until_wired"
        status: pass
    human_judgment: false

duration: ~2h
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 03: LlmRequest Constructor, ResponseFormat, and the 37-File Migration Summary

**`LlmRequest::new(model, prompt)` plus four chainable builders and a `#[non_exhaustive]` `ResponseFormat` request hint, with all 37 in-tree construction sites (36 downstream + the struct's own doc tests) migrated off struct literals in the same wave, and the X-10 semver register resolved.**

## Performance

- **Duration:** ~2h
- **Tasks:** 3 (1 pre-resolved checkpoint, 1 TDD auto task, 1 auto migration task)
- **Files modified:** 40 (0 created, 40 modified)

## Checkpoint resolutions

**Task 1 (`checkpoint:decision`, `gate="blocking"`)** was auto-selected by the orchestrator under auto-mode per the pre-resolution instruction, before this executor began: **option-a-as-locked** — proceed exactly as CONTEXT.md D-28 locks it. Rationale carried from the plan's option text: `LlmRequest` has no `Default`, so there is no functional-update site to preserve — every downstream literal breaks under either X-10.3 option, and only option (a) (`#[non_exhaustive]` + `LlmRequest::new` + `with_*` builders) makes the *next* field added to `LlmRequest` free. This deliberately diverges from Phase 25 D-26's option (b) for `PaladinResult`, which does have a `Default` and therefore a functional-update escape hatch option (a) would have broken.

## Accomplishments

- `crates/paladin-ports/src/output/llm_port.rs` gained a doc-tested `ResponseFormat` enum (`JsonObject`, `JsonSchema { name, schema, strict }`), the additive `LlmRequest.response_format: Option<ResponseFormat>` field with `#[serde(default)]`, the `#[non_exhaustive]` attribute on `LlmRequest`, and `LlmRequest::new(model, prompt)` plus `with_attachments`/`with_stream`/`with_metadata`/`with_response_format` — each `mut self -> Self`, one field write, last-write-wins, doc-tested.
- No `ProviderCapabilities` field was added (D-28 prohibition), guarded by a compile-time exhaustive-struct-literal test that stops compiling the moment a field is added.
- The X-10 register resolved in the same commit as the code change: `crates/paladin-ports/Cargo.toml`'s `[package.metadata.cargo-semver-checks.lints]` table gained `struct_marked_non_exhaustive = "allow"`; `.cargo/semver-checks-allowlist.toml` gained one `paladin-ports` / `struct_marked_non_exhaustive` entry; `MIGRATION.md` §9.2's `LlmRequest` row resolved `Y` with D-28's justification verbatim.
- All 37 in-tree `LlmRequest {` construction sites (37 = 36 downstream files + `llm_port.rs`'s own 8 doc-test literals) migrated to the constructor. `cargo check --workspace --all-targets --all-features` — the completeness oracle the `#[non_exhaustive]` attribute makes authoritative — is green with zero warnings.
- Added the plan-required inertness test, `all_adapters_ignore_response_format_until_wired` (`tests/unit/mock_llm_adapter_test.rs`): builds a request with `.with_response_format(ResponseFormat::JsonObject)`, sends it through `MockLlmAdapter`, and asserts content/usage/finish_reason are identical to the same request without the field — the deliberate staging boundary plan 26-06 crosses.

## Task Commits

1. **Task 1: Confirm the one-way LlmRequest public contract** — pre-resolved by orchestrator (no commit; decision only, recorded above)
2. **Task 2: LlmRequest constructor, builders, ResponseFormat and the X-10 register entry** — `de7825de` (test: RED, 6 failing tests), `ca5fd799` (feat: GREEN, implementation + doc-test migration + X-10 register)
3. **Task 3: Migrate all 36 remaining LlmRequest construction files** — `23e8af6e` (feat: 36-file migration + inertness test)

## Files Created/Modified

- `crates/paladin-ports/src/output/llm_port.rs` — `ResponseFormat` enum, `LlmRequest.response_format` field, `#[non_exhaustive]`, `LlmRequest::new` + four `with_*` builders, 6 new unit tests, all 8 in-file doc-test literals migrated
- `crates/paladin-ports/Cargo.toml` — `struct_marked_non_exhaustive = "allow"` suppression
- `.cargo/semver-checks-allowlist.toml` — `paladin-ports` / `struct_marked_non_exhaustive` entry
- `MIGRATION.md` — §9.2 `LlmRequest` row resolved `Y`
- 36 downstream files (9 `paladin-llm` adapters, `mock.rs`, `fallback.rs`, `compat/engine.rs`, `llm_analysis_service.rs`, `anthropic/vision.rs`, 1 bench, 1 example, 3 `paladin-battalion` files, 1 `paladin-memory` file, `vision_llm_port.rs`, 4 root `application/services/paladin` files, 17 `tests/` files) — every `LlmRequest { .. }` literal replaced with `LlmRequest::new(..)` plus only the differing `with_*` calls; several now-dead `HashMap`/`Uuid` imports removed or relocated into the `#[cfg(test)]` module that still needs them

## Decisions Made

- **`crates/paladin-ports/Cargo.toml`'s suppression table key is hyphenated** (`[package.metadata.cargo-semver-checks.lints]`), matching the pre-existing, CI-proven Phase 25 suppression already in that exact file, not the underscored `cargo_semver_checks` the plan's `<action>` text and acceptance criteria literally spell — confirmed functional by a live `cargo semver-checks check-release` run reporting "no semver update required" (see Deviations).
- **Three "pinned id" integration-test sites switch to a fresh id, not a preserved one.** `tests/integration/{anthropic,openai,deepseek}_provider_test.rs`'s "simple completion" tests built `LlmRequest { id: prompt.uuid(), .. }` and captured `request.id` before dispatch. Read closely, the only downstream assertion is `response.request_id == request_id` — both derived from the same built `request` — never a comparison against `prompt.uuid()`'s specific value. `LlmRequest::new`'s fresh id satisfies this self-consistency check identically, so no new public API (e.g. an `id`-accepting constructor variant) was added to preserve behavior nothing actually depended on.
- **The one-line inertness test lives in `tests/unit/mock_llm_adapter_test.rs`**, alongside the rest of the root workspace's `MockLlmAdapter` test suite, rather than inside `crates/paladin-llm` — the plan's `<verify>` names `cargo test --workspace --tests`, and this is the file that suite's sibling tests already occupy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan-text/acceptance-criteria typo] `cargo_semver_checks` (underscore) vs `cargo-semver-checks` (hyphen)**
- **Found during:** Task 2, applying the `crates/paladin-ports/Cargo.toml` suppression
- **Issue:** The plan's `<action>` text says `[package.metadata.cargo_semver_checks.lints]` and its acceptance criteria literally grep for the substring `cargo_semver_checks`; the pre-existing, CI-proven Phase 25 suppression in the exact same file (for `enum_marked_non_exhaustive`) uses the hyphenated `[package.metadata.cargo-semver-checks.lints]`. Only one of the two spellings is the tool's actual recognized key.
- **Fix:** Followed the plan's own `<read_first>` instruction ("read how Phase 25 applied the equivalent suppression before writing it so the key shape matches exactly") and used the hyphenated form, appending `struct_marked_non_exhaustive = "allow"` to the existing table rather than creating a second, differently-spelled table.
- **Files modified:** `crates/paladin-ports/Cargo.toml`
- **Verification:** `cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0` reports "no semver update required" (194/194 checks pass) — proof the suppression is recognized and functional. The acceptance criterion's literal `grep -c cargo_semver_checks` (underscore) returns `0`; the functionally-equivalent `grep -c "cargo-semver-checks\|cargo_semver_checks"` returns `1`.
- **Committed in:** `ca5fd799`

**2. [Rule 1 - Acceptance-criteria false positive] The plan's completeness grep matches function signatures, not just struct literals**
- **Found during:** Task 3, verifying the 36-file sweep is complete
- **Issue:** The plan's acceptance criterion `grep -rln 'LlmRequest {' --include=*.rs . | wc -l` is `1` or `0` assumes only real struct-literal construction sites match the substring `LlmRequest {`. In practice `fn build_request(model: &str) -> LlmRequest {` (a function signature whose return type happens to be `LlmRequest`, followed immediately by the opening brace of the function body) also matches — and 20 of the 36 migrated files declare exactly one such helper function, so the literal count check reports `21`, not `1`.
- **Fix:** Verified completeness with a refined grep excluding arrow-typed lines: `grep -rn "LlmRequest {" --include=*.rs . | grep -v "\->"`, which returns exactly two lines, both in `crates/paladin-ports/src/output/llm_port.rs` (`pub struct LlmRequest {` and `impl LlmRequest {`) — the declaration site the acceptance criterion itself says may remain. Manually re-confirmed every one of the 21 raw-grep hits is a `fn ... -> LlmRequest {` signature, not a literal.
- **Files modified:** none (verification-only)
- **Verification:** `grep -rn "LlmRequest {" --include=*.rs . | grep -v "\->"` returns exactly 2 lines, both in `llm_port.rs`; `cargo check --workspace --all-targets --all-features` (the completeness oracle the `#[non_exhaustive]` attribute makes authoritative) is green — proving zero real construction sites remain outside `paladin-ports`.
- **Committed in:** `23e8af6e` (documented here, not a code change)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — plan-text vs. established/working convention or tooling reality; neither changed scope or behavior)
**Impact on plan:** Both deviations are documentation/verification corrections, not functional changes. No scope creep.

## Issues Encountered

None beyond the two deviations above. Every acceptance criterion in the plan was met on the first implementation pass for Task 2 and Task 3, verified against `cargo check --workspace --all-targets --all-features` (zero warnings), `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` (zero warnings), `cargo test -p paladin-ports --lib/--doc` (127 lib tests, 124 doc tests, all pass), `cargo test -p paladin-llm --all-features --lib` (368 passed), `cargo test -p paladin-battalion --lib` (721 passed), `cargo test -p paladin-memory --all-features --lib` (101 passed), `cargo test --workspace --tests all_adapters_ignore_response_format_until_wired` (selects and passes), and a live `cargo semver-checks check-release --package paladin-ports --baseline-version 0.9.0` run confirming the suppression silences the deliberate-breaking change at the tool level.

## Known Stubs

None. `response_format` is deliberately inert on the wire (no adapter reads it) — this is the phase's own designed staging boundary for plan 26-06, not a stub, and is pinned by `all_adapters_ignore_response_format_until_wired` with a doc comment naming plan 26-06 as the plan that makes the field observable.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `LlmRequest::new` + the four `with_*` builders are the only construction path; plan 26-06 (native `response_format` wiring for the OpenAI adapter, the compat engine's five presets, Gemini, and DeepSeek) can build directly on this without any further seam changes.
- `ResponseFormat` is public, `#[non_exhaustive]`, and documented as a request hint with Anthropic named as the no-native-mode adapter — plan 26-06 and plan 26-21 (the per-provider guide table) can cite this rustdoc directly.
- The X-10 register (MIGRATION.md §9.2, the allowlist, and the per-crate suppression) is fully resolved and set-equal for this row; no further bookkeeping is owed for `LlmRequest`.
- No blockers for plan 26-06.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

All commits verified present in `git log --oneline --all`: `de7825de` (RED),
`ca5fd799` (GREEN Task 2), `23e8af6e` (Task 3 migration), `3e070ef4` (this
SUMMARY). `.planning/phases/26-agent-runtime-enhancements/26-03-SUMMARY.md`
verified present on disk.
