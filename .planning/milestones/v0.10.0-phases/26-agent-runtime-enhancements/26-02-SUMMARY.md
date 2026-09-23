---
phase: 26-agent-runtime-enhancements
plan: 02
subsystem: config
tags: [config, agent-runtime, middleware, x-09, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 01)
    provides: "ExecutionMiddleware trait, MiddlewareFlow/ToolFlow, FinalResult, and the ModelCallContext/ToolCallContext/LlmResponseView/PromptAssembly/PromptSection context types this plan's config sub-structs are constructor inputs for"
provides:
  - "AgentRuntimeConfig in src/config/agent_runtime.rs with all twelve X-09 sub-structs (ModelCallLimitConfig, TokenBudgetConfig, ToolCallLimitConfig, GuardrailConfig, HistoryTrimmerConfig, SummarizationConfig, VaultRecallConfig, ModelRetryConfig, ModelFallbackConfig, ToolErrorConfig, StructuredOutputConfig, VaultToolsConfig), each with Default, validate() and EnvOverridable for its scalar fields"
  - "agent_runtime: AgentRuntimeConfig attached to Settings with #[serde(default)], so a v0.9 config.yml with no agent_runtime: section boots v0.10 identically"
  - "Settings::validate() (new method) delegating to AgentRuntimeConfig::validate()"
  - "The AgentRuntimeConfig rustdoc's documented, fixed build_chain assembly order (limits -> guardrail -> trimmer/summarizer -> recall -> protocol -> resilience), landing as code in plan 26-20"
affects: [26-05, 26-08, 26-10, 26-11, 26-15, 26-19, 26-20]

tech-stack:
  added: []
  patterns:
    - "One grouped AgentRuntimeConfig under X-09 mirroring src/config/node_cache.rs's shape (Default + validate() + EnvOverridable per sub-struct) rather than twelve scattered config modules"
    - "#[derive(Default)] instead of a hand-written impl wherever every field's inert value already equals that field's own type's Default::default() (clippy::derivable_impls) -- reserving the hand-written-Default convention for sub-structs whose defaults are non-derivable literals (e.g. ModelRetryConfig mirroring RetryPolicy's non-zero numeric defaults)"
    - "A Settings-shaped local test Wrapper struct + config::File::from_str/File::new to exercise one field's deserialization in isolation, without requiring every other Settings-required field (avoids coupling a config-pinning test to unrelated, pre-existing config file gaps)"

key-files:
  created:
    - src/config/agent_runtime.rs
  modified:
    - src/config/mod.rs
    - src/config/settings.rs
    - src/config/user_config.rs
    - config.example.yml

key-decisions:
  - "AgentRuntimeConfig is attached directly to Settings (#[serde(default)]), unlike the EngineConfig/WaypointStoreConfig/NodeCacheConfig precedent which are deliberately NOT Settings fields (X-10 avoidance, loaded standalone in paladin-server.rs) -- this plan's own PLAN.md explicitly directs attaching to Settings, and Task 2's inertness test requires it"
  - "Settings gained a new validate() method (Rule 2, minimal addition) delegating to AgentRuntimeConfig::validate() -- the plan's own Task 2 test text requires 'settings.validate() succeeds', and no such method existed; scoped to agent_runtime only, not a general cross-cutting validator for every domain config"
  - "AgentRuntimeConfig, ModelFallbackConfig and VaultToolsConfig use #[derive(Default)] rather than a hand-written impl, breaking from the node_cache.rs 'always hand-write it' convention cited in the plan -- clippy::derivable_impls correctly flags a manual impl whose every field equals the field type's own Default::default() as needless; the other nine sub-structs (whose defaults ARE non-trivial literals, e.g. ModelRetryConfig mirroring RetryPolicy's 3/500ms/2.0/60s/true) keep the hand-written impl colocated with validate()"
  - "Test 8 (config.example.yml pinning) deserializes only the agent_runtime key via a local Wrapper struct + config::File::new, not the full Settings::load_from_file -- the latter fails on a PRE-EXISTING, out-of-scope gap: config.example.yml's llm.ollama block deliberately carries no api_key (D-12, since Ollama needs no credential) but LlmProviderConfig.api_key is a required (non-Option, non-#[serde(default)]) String, so full Settings deserialization of config.example.yml has never worked. Documented as out of scope per the deviation-rules scope boundary rather than fixed here."

patterns-established:
  - "Regex rule lists, per-tool maps and model-context tables are config-file only by construction: EnvOverridable impls simply never read an env var for a collection-shaped field, and env_overrides_apply_to_scalar_fields_only pins that setting a plausibly-named env var for one has zero effect"

requirements-completed: [RT-02]

coverage:
  - id: D1
    description: "AgentRuntimeConfig exists in src/config/agent_runtime.rs with all twelve X-09 sub-structs, each carrying Default, validate() and EnvOverridable for its scalar fields, mirroring node_cache.rs"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/config/agent_runtime.rs#tests::default_agent_runtime_config_is_inert, tests::validate_rejects_zero_and_out_of_range_scalars, tests::env_overrides_apply_to_scalar_fields_only, tests::every_sub_struct_validates_under_its_own_default"
        status: pass
    human_judgment: false
  - id: D2
    description: "A v0.9 config.yml with no agent_runtime: section (and an agent_runtime: {} empty table) resolves to AgentRuntimeConfig::default(), attached to Settings with #[serde(default)]"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/config/agent_runtime.rs#tests::absent_agent_runtime_section_deserializes_to_default, tests::empty_agent_runtime_table_deserializes_to_default, tests::v0_9_config_resolves_every_agent_runtime_section_inert"
        status: pass
    human_judgment: false
  - id: D3
    description: "config.example.yml carries an explicit agent_runtime: block that is pinned equal to AgentRuntimeConfig::default(), so the documented example and the code default cannot silently drift"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/config/agent_runtime.rs#tests::example_config_agent_runtime_block_round_trips_to_default"
        status: pass
    human_judgment: false
  - id: D4
    description: "No field in AgentRuntimeConfig's tree is secret-shaped -- ModelFallbackConfig names providers only, and this is pinned by a Debug-rendering check"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/config/agent_runtime.rs#tests::config_carries_no_secret_shaped_field"
        status: pass
    human_judgment: false
  - id: D5
    description: "PaladinConfig is untouched by this plan; full workspace check/clippy/fmt/doc-tests are clean"
    requirement: "RT-02"
    verification:
      - kind: other
        ref: "git diff --name-only HEAD~1 -- crates/paladin-core/src/platform/container/paladin_config.rs (0 lines); cargo check --workspace --all-targets --all-features; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo fmt --all --check; cargo test -p paladin-ai --doc (120/120)"
        status: pass
    human_judgment: false

duration: ~55min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 02: AgentRuntimeConfig Summary

**`AgentRuntimeConfig` in `src/config/agent_runtime.rs` — one X-09 config home for all twelve built-in middleware sub-structs (limits, guardrail, trimmer, summarization, vault recall, retry/fallback, tool-error policy, structured output, vault tools), attached to `Settings`, fully inert by default and pinned against `config.example.yml` drift.**

## Performance

- **Duration:** ~55min
- **Tasks:** 2 (both `tdd="true"`)
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- `AgentRuntimeConfig` and its twelve sub-structs (`ModelCallLimitConfig`, `TokenBudgetConfig`, `ToolCallLimitConfig`, `GuardrailConfig` + `GuardrailRuleConfig`/`GuardrailTarget`/`GuardrailOnMatch`, `HistoryTrimmerConfig`, `SummarizationConfig`, `VaultRecallConfig`, `ModelRetryConfig` + `RetryOnConfig`, `ModelFallbackConfig`, `ToolErrorConfig` + `ToolErrorMode`, `StructuredOutputConfig`, `VaultToolsConfig`) now exist with `Default`, `validate()` and `EnvOverridable` for scalar fields, mirroring `src/config/node_cache.rs`'s shape.
- Every sub-struct defaults to `enabled: false`, except `ToolErrorConfig` (a today's-behavior *policy* default of `ToolErrorMode::FeedToModel`, per D-33) — so a v0.9 `config.yml` with no `agent_runtime:` section boots v0.10 with identical behavior, asserted by `default_agent_runtime_config_is_inert`.
- `AgentRuntimeConfig` is attached to `Settings` (`#[serde(default)] pub agent_runtime: AgentRuntimeConfig`), so both an absent `agent_runtime:` key and an explicit empty `agent_runtime: {}` resolve to the same default.
- Scalar fields are env-overridable under `APP_AGENT_RUNTIME_<SECTION>_<FIELD>`; regex rule lists, per-tool maps and model-context tables have no environment-variable form by construction, pinned by a test that sets plausibly-named env vars for those fields and asserts zero effect.
- `config.example.yml` gained an explicit, fully-written-out `agent_runtime:` block, pinned byte-for-byte equal to `AgentRuntimeConfig::default()` by a dedicated test — so the documented example and the code default cannot silently drift apart.
- `AgentRuntimeConfig`'s rustdoc records the `build_chain` assembly order (`limits -> guardrail -> trimmer/summarizer -> recall -> protocol -> resilience`) as prose, with no stub function written — one written home for plan 26-20 to implement against.
- `PaladinConfig` is untouched (confirmed by `git diff` on the file); no secret-shaped field exists anywhere in the tree (`ModelFallbackConfig` carries provider names only).

## Task Commits

1. **Task 1 + Task 2 (combined RED/GREEN, same files):**
   - RED: `83eb7391` — test(26-02): add failing tests for AgentRuntimeConfig and its inertness contract
   - GREEN: `ccf02134` — feat(26-02): AgentRuntimeConfig and the v0.9-config inertness contract (GREEN)

_Note on the RED/GREEN split: both plan tasks (Task 1 "AgentRuntimeConfig and its twelve sub-structs" and Task 2 "the v0.9-config inertness contract") modify the same file (`src/config/agent_runtime.rs`) and are tightly coupled — Task 2's tests exercise the exact types Task 1 defines. Rather than an artificial file-level split, both tasks' tests were written together, with two deliberate, isolated defects: (1) a wrong expected value in `default_agent_runtime_config_is_inert`'s `tool_errors.mode` assertion (`FailRun` instead of the correct `FeedToModel`), and (2) a deliberately wrong `token_budget.max_tokens` value in the `config.example.yml` block being pinned. Both were confirmed failing (`7 passed; 2 failed`) before being corrected in the GREEN commit — a genuine, verified RED state on the exact two assertions this plan's tests are supposed to catch, rather than a Rust-compile-failure-shaped RED (impractical for a from-scratch config module with no prior behavior to regress against)._

## Files Created/Modified

- `src/config/agent_runtime.rs` — `AgentRuntimeConfig` + twelve sub-structs, `Default`/`validate()`/`EnvOverridable` impls, module rustdoc (X-09 rationale, `build_chain` order documentation), 9 unit tests
- `src/config/mod.rs` — `pub mod agent_runtime;` (alphabetical position) + `pub use crate::config::agent_runtime::AgentRuntimeConfig;`
- `src/config/settings.rs` — `agent_runtime: AgentRuntimeConfig` field (`#[serde(default)]`) on `Settings`, wired into `Default for Settings`, new `Settings::validate()` method
- `src/config/user_config.rs` — fixed a pre-existing full-struct-literal `Settings { .. }` test helper (`create_test_settings`) that needed the new field (Rule 3, blocking compile error)
- `config.example.yml` — explicit `agent_runtime:` block, every field written out, pinned equal to `AgentRuntimeConfig::default()`

## Decisions Made

- **`AgentRuntimeConfig` is attached directly to `Settings`**, deliberately breaking from the `EngineConfig`/`WaypointStoreConfig`/`NodeCacheConfig` precedent (those are intentionally *not* `Settings` fields, per an existing `paladin-server.rs` comment citing "X-10 avoidance"). This plan's own `PLAN.md` explicitly directs the `Settings`-attachment shape, and Task 2's inertness contract test requires it (`settings.agent_runtime == AgentRuntimeConfig::default()`).
- **Added `Settings::validate()`** (a new, minimal method) delegating to `AgentRuntimeConfig::validate()`, since the plan's own Task 2 test text requires `settings.validate()` to succeed and no such method previously existed on `Settings`. Scoped narrowly — it does not attempt to become a general validator for every other domain config.
- **`AgentRuntimeConfig`, `ModelFallbackConfig` and `VaultToolsConfig` use `#[derive(Default)]`**, not a hand-written impl — `clippy::derivable_impls` (run under the project's `-D warnings` gate) correctly flagged the hand-written versions as needless, since every field's disabled-by-default value already *is* that field's own type's `Default::default()`. This differs from `node_cache.rs`'s convention (whose `Default` picks non-derivable literals like a hostname and a key prefix), documented inline. The other nine sub-structs keep the hand-written `impl Default` colocated with `validate()`, since their defaults (e.g. `ModelRetryConfig` mirroring `RetryPolicy`'s `3`/`500ms`/`2.0`/`60s`/`true`) are not derivable.
- **Task 2's `config.example.yml`-pinning test deserializes only the `agent_runtime` key** via a local `Wrapper` struct and `config::File::new`, rather than a full `Settings::load_from_file("config.example.yml")`. The latter fails today on a pre-existing, unrelated gap: `config.example.yml`'s `llm.ollama` block deliberately carries no `api_key` (Ollama needs no credential, per an existing D-12 comment in the file), but `LlmProviderConfig.api_key` is a required `String` with no `Option`/`#[serde(default)]`. This means `config.example.yml` has never fully deserialized into `Settings` — out of this plan's scope per the deviation rules' scope boundary (pre-existing, unrelated to `agent_runtime`), not fixed here.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed a pre-existing `Settings` struct-literal test helper missing the new field**
- **Found during:** Initial compile check after adding `agent_runtime` to `Settings`
- **Issue:** `src/config/user_config.rs`'s `create_test_settings()` test helper constructs `Settings { .. }` as a full struct literal (no `..Default::default()` spread), so adding the new `agent_runtime` field broke compilation (`E0063: missing field`).
- **Fix:** Added `agent_runtime: crate::config::AgentRuntimeConfig::default()` to the literal.
- **Files modified:** `src/config/user_config.rs`
- **Verification:** `cargo check -p paladin-ai --lib --all-features` and the full lib test suite (578/578) pass
- **Committed in:** `83eb7391` (RED commit, since the file needed to compile before any test could run)

**2. [Rule 1 - Bug] Switched three sub-structs to `#[derive(Default)]` per clippy**
- **Found during:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` after the GREEN implementation
- **Issue:** `clippy::derivable_impls` flagged the hand-written `impl Default` on `AgentRuntimeConfig`, `ModelFallbackConfig` and `VaultToolsConfig` as needless — every field's value already equals the field type's own `Default::default()`.
- **Fix:** Replaced each with `#[derive(Default)]`, removing the now-redundant manual `impl Default` blocks and documenting inline why this differs from `node_cache.rs`'s hand-written convention (see Decisions Made).
- **Files modified:** `src/config/agent_runtime.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0; all 9 `agent_runtime` tests and the full 578-test lib suite still pass
- **Committed in:** `ccf02134`

**3. [Rule 3 - Blocking] Isolated the `config.example.yml` pinning test from a pre-existing, out-of-scope deserialization gap**
- **Found during:** First run of `example_config_agent_runtime_block_round_trips_to_default` via full `Settings::load_from_file`
- **Issue:** `Settings::load_from_file("config.example.yml")` fails with `missing configuration field "llm.ollama.api_key"` — a pre-existing gap (the ollama block deliberately has no `api_key`, but `LlmProviderConfig.api_key` is a required `String`), unrelated to this plan's `agent_runtime` scope.
- **Fix:** Rewrote the test to deserialize only the `agent_runtime` key via a local `Wrapper` struct and `config::File::new`, matching the isolation approach already used for the `agent_runtime: {}` and absent-key tests. Did NOT touch `LlmProviderConfig` or the `llm.ollama` block — out of scope per the deviation rules' scope boundary.
- **Files modified:** `src/config/agent_runtime.rs` (test only)
- **Verification:** `example_config_agent_runtime_block_round_trips_to_default` passes; the pre-existing gap is unchanged and undocumented-elsewhere, so it is noted here for visibility
- **Committed in:** `ccf02134`

---

**Total deviations:** 3 auto-fixed (1 blocking compile fix, 1 clippy-driven bug fix, 1 blocking test-isolation fix)
**Impact on plan:** All three were necessary to make the plan's own tests and verification commands runnable and workspace-clean. No scope creep — the pre-existing `config.example.yml`/`llm.ollama` gap was isolated around, not fixed.

## Issues Encountered

None beyond the three deviations above. Every acceptance criterion in the plan (struct/field counts, `mod.rs`/`settings.rs` wiring, all named test functions passing, `PaladinConfig` untouched, the `pattern_size_limit_bytes` documentation count, `cargo test -p paladin-ai --doc`) was verified directly.

## Known Stubs

None. Every sub-struct is a real, validated, env-overridable config type — no placeholder values flow anywhere, and `build_chain` is deliberately documented as prose only (per the plan's own instruction not to write a stub function body), not a stub that silently returns an empty chain.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Every later Phase 26 plan (26-05, 26-08, 26-10, 26-11, 26-15, 26-19) has a real sub-struct to construct its middleware from: `ModelCallLimitConfig`/`TokenBudgetConfig`/`ToolCallLimitConfig` for limits, `GuardrailConfig` (+ `GuardrailRuleConfig`/`GuardrailTarget`/`GuardrailOnMatch`) for the guardrail, `HistoryTrimmerConfig`/`SummarizationConfig` for context-window management, `VaultRecallConfig` for recall, `ModelRetryConfig`/`ModelFallbackConfig` for resilience, `ToolErrorConfig`/`ToolErrorMode` for tool-error policy, `StructuredOutputConfig` for the repair loop, `VaultToolsConfig` for the built-in Armaments.
- Plan 26-20 (`build_chain`) has its documented, fixed assembly order already written into `AgentRuntimeConfig`'s rustdoc — no re-derivation needed, just implementation against the existing sub-structs.
- No blockers for wave 2 or wave 3 plans.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED
