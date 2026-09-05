---
phase: 25-node-level-fault-tolerance
plan: 05
subsystem: infra
tags: [llm-adapters, error-taxonomy, http-status, redaction, security, thiserror]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-02)
    provides: "LlmError::ProviderError { provider, status: u16, message } and LlmError::transience() classifying by status value"
  - phase: 17-provider-adapters
    provides: "crate::redaction::{redact_credentials, bounded_excerpt, RESPONSE_EXCERPT_CHAR_BUDGET} and the shared CompatEngine backing five presets"
provides:
  - "paladin_llm::http_status::map_http_status(provider, status, body, api_key) -> LlmError: the ONE status-to-variant mapping for all nine adapters, redact-then-bound on char boundaries"
  - "All nine adapters (openai, anthropic, deepseek, gemini, kimi, qwen, grok, ollama, openai_compatible) emit a typed ProviderError { status } for every non-2xx status without a dedicated variant; zero ProcessingError(format!(\"HTTP ...\")) sites remain in paladin-llm"
  - "CompatEngine::with_provider_name / provider_name() / DEFAULT_PROVIDER_NAME (additive API) so each thin preset's name lands in ProviderError.provider"
  - "Status assertions in crate tests and root tests read ProviderError's typed status field, never a rendered message"
affects: [25-08-fallback-chain, 25-10-error-handlers, 25-14-semver-gate-evidence, 26-rt-06-retry-paths]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One crate-level mapping helper (http_status.rs) extracted from nine per-adapter match blocks, the same extraction shape redaction.rs used for credential scrubbing (25-PATTERNS.md 'exact' analog)"
    - "Provider-specific pre-checks (Anthropic 403, Anthropic/Gemini body-shaped 400s, Gemini RPC-status envelope, refused-redirect 3xx) run BEFORE the shared helper and never duplicate its table; the helper's rustdoc names this contract"
    - "Adapters pass the RAW body to the helper so redact->bound happens exactly once (pre-excerpting would bound twice and truncate the first elision marker)"
    - "Per-preset routing test compares the adapter's live mockito error against map_http_status()'s own output for the same status/body/key, proving routing rather than re-asserting the table"

key-files:
  created:
    - crates/paladin-llm/src/http_status.rs
  modified:
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - tests/unit/llm/deepseek_adapter_test.rs
    - tests/unit/llm/anthropic_adapter_test.rs

key-decisions:
  - "The five OpenAI-compatible presets (kimi, qwen, grok, ollama, openai_compatible) do not own a non-2xx branch: they all delegate to CompatEngine::map_error. The engine was rewired once (in Task 2, since kimi/qwen needed it) and each preset names its engine via the new with_provider_name builder; the per-preset adapter.rs files call map_http_status in their routing tests, which is what proves the delegation rather than a private copy."
  - "Provider identity reached CompatEngine through an additive builder (with_provider_name) plus a private field, NOT a new pub field on CompatEngineConfig: that struct is constructible by literal, so a new field would trip cargo-semver-checks' constructible_struct_adds_field in the paladin-llm semver job (X-10)."
  - "Refused-redirect 3xx arms in CompatEngine and Gemini construct ProviderError { status: 3xx, message: <actionable text> } directly rather than via the helper: the variant is exactly what the helper would choose (Permanent by value, FT-FR-01), only the operator-facing message is enriched. The 'no new LlmError variant' rationale (PROV-02 / T-17-54) that pinned them to ProcessingError is superseded by 25-02's ProviderError; the retry-cost note stays."
  - "Anthropic's 403 -> AuthenticationError and Gemini's 401|403 -> AuthenticationError pre-checks are preserved ahead of the helper: both adapters' retry loops halt only on the non-retryable set (Auth/InvalidPrompt/Empty/UsageLimit), so letting a rejected credential fall to the helper's generic ProviderError { 403 } would re-transmit it up to max_retries times (WR-03 rationale)."
  - "The helper's 400 arm applies OpenAI's byte-identical 'maximum context length' predicate on the full REDACTED body (not the bounded excerpt) so a signature past the 512-char budget is not missed; only the bounded excerpt is ever emitted."
  - "The 401 message text is now the helper's uniform 'Invalid API key for provider ...' (previously per-adapter prose such as 'Check DEEPSEEK_API_KEY'): the variant is unchanged, no test asserted the prose, and one message source is the point of the plan."

requirements-completed: [FT-01]

coverage:
  - id: D1
    description: "One shared map_http_status maps 401/429/402/404/400 to their dedicated variants and every other non-2xx to ProviderError carrying the status as a typed u16; redacts before bounding on char boundaries"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-llm/src/http_status.rs#tests::{unmapped_statuses_become_provider_error_with_the_status_field,dedicated_statuses_keep_their_existing_variants,unknown_4xx_becomes_provider_error_not_processing_error,excerpt_is_redacted_before_it_is_bounded,multibyte_body_is_bounded_on_char_boundaries,empty_body_produces_an_empty_message_and_still_classifies}"
        status: pass
      - kind: doc
        ref: "cargo test --doc -p paladin-llm --all-features (6 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "All nine adapters route their non-2xx branches through the helper; no stringly HTTP fallback and no status-by-substring assertion remains"
    requirement: FT-01
    verification:
      - kind: other
        ref: "test \"$(grep -rl 'map_http_status(' crates/paladin-llm/src/*/adapter.rs | wc -l)\" -eq 9 (pass, 9)"
        status: pass
      - kind: other
        ref: "! grep -rnE 'ProcessingError\\(format!\\(\"HTTP' crates/paladin-llm/src/ (pass, no matches)"
        status: pass
      - kind: other
        ref: "! grep -rnE 'contains\\(\"(4[0-9][0-9]|5[0-9][0-9])\"\\)' crates/paladin-llm/src/ tests/unit/llm/deepseek_adapter_test.rs tests/unit/mock_llm_adapter_test.rs (pass, no matches)"
        status: pass
      - kind: unit
        ref: "per-adapter *_non_2xx_routes_through_the_shared_mapper (9), kimi_http_500_carries_a_typed_status, streaming_non_2xx_routes_through_the_shared_mapper, gemini_error_envelope_still_parses_before_mapping, ollama_local_server_4xx_maps_without_a_dedicated_variant"
        status: pass
    human_judgment: false
  - id: D3
    description: "Workspace builds, tests, lints and formats clean across all targets and features"
    requirement: FT-01
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --lib (68 default-feature) and --all-features (347) (exit 0)"
        status: pass
      - kind: integration
        ref: "cargo test --test lib (692 passed, 14 ignored/self-skipped, exit 0)"
        status: pass
      - kind: other
        ref: "cargo fmt --all -- --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
    human_judgment: false

duration: ~35min (worktree spawn 21:31Z to final task commit 21:58Z, plus verification)
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 05: Shared HTTP-Status Mapping (`map_http_status`) Summary

**One `map_http_status` helper now classifies every non-2xx provider response for all nine LLM adapters, redacting the body before bounding it on character boundaries and emitting a typed `ProviderError { status: u16 }` for every status without a dedicated variant — the last string-encoded statuses `LlmError::transience()` could not see through are gone.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-09-05T21:31Z (worktree spawn)
- **Completed:** 2026-09-05T22:05Z (verification battery green)
- **Tasks:** 3 (Task 1 as RED + GREEN commits)
- **Files modified:** 14 (1 created, 13 modified; +1136 / -217 lines)

## Accomplishments

- **`crates/paladin-llm/src/http_status.rs`** (`pub fn map_http_status(provider, status, body, api_key) -> LlmError`, not feature-gated, `pub mod http_status` in `lib.rs`): composes `redact_credentials` **then** `bounded_excerpt` in one place; 401/429/402/404/400 keep their dedicated variants (400 disambiguated to `TokenLimitExceeded` with OpenAI's byte-identical predicate, evaluated on the full redacted body); everything else — 5xx, 408, 3xx, and every unknown 4xx — becomes `ProviderError { provider, status, message }`. Rustdoc names why bounding first leaks a credential tail (T-25-20) and documents the pre-check contract for provider-specific arms. Six unit tests plus a doc test, committed RED first.
- **OpenAI**: all three non-2xx sites (generate, streaming, model list) route through the helper; the model-list path no longer builds `ProcessingError(format!("HTTP {}"))`, and the streaming path yields the same typed variant as the generate path (`streaming_non_2xx_routes_through_the_shared_mapper`).
- **DeepSeek**: `map_error` is a one-line delegation; both call sites pass the raw body so redact-then-bound runs exactly once (previously the generate path pre-excerpted and the stream path did not — now uniform).
- **Anthropic**: 403 pre-check and the two body-shaped 400 disambiguations preserved, then delegates; 5xx and unknown 4xx are typed.
- **Gemini**: the RPC-status envelope parse and every Google-specific arm preserved; the catch-all hands the extracted `error.message` plus the RPC status string (not the raw JSON) to the helper (`gemini_error_envelope_still_parses_before_mapping`); the refused-redirect arm is a typed `ProviderError { 3xx }` with its actionable message.
- **`CompatEngine`** (kimi/qwen/grok/ollama/openai_compatible): `map_error` takes the raw body, consults `error_override` on the redacted excerpt as before, emits a typed `ProviderError` for a refused redirect, and delegates everything else; new additive `with_provider_name` / `provider_name()` / `DEFAULT_PROVIDER_NAME` so each preset's name lands in `ProviderError.provider`. Each preset's routing test compares its live mockito 503 error field-by-field with `map_http_status`'s own output; ollama additionally pins a local-server 409 as `ProviderError`, never `ProcessingError`.
- **Tests migrated to typed-field assertions**: kimi `http_500_maps_to_processing_error_carrying_status` -> `kimi_http_500_carries_a_typed_status`; gemini `map_error_unrecognised_status_maps_to_processing_error_carrying_http_code_and_status` -> `gemini_error_envelope_still_parses_before_mapping`; compat `map_error_status_codes` (500 arm) and `map_error_maps_a_redirect_status_to_an_actionable_processing_error` -> `..._provider_error`; root `tests/unit/llm/{deepseek,anthropic}_adapter_test.rs` 500 tests assert `ProviderError { provider, status: 500 }`.
- **`LlmProviderError`** (`crates/paladin-llm/src/error.rs`) untouched — no adapter mapping needed it.

## Task Commits

1. **Task 1: The shared `map_http_status` helper, redact-then-bound** — `2bf2e145` (test, RED) + `b59c1980` (feat, GREEN)
2. **Task 2: Route the five OpenAI-family adapters through the shared helper** — `9ffff4db` (feat) — openai, deepseek, anthropic, `CompatEngine` (which carries kimi and qwen), plus the two root-crate test migrations
3. **Task 3: Route the remaining four adapters and migrate the crate-level status tests** — `977caef0` (feat) — gemini, grok, ollama, openai_compatible

**Plan metadata:** this commit (`docs(25-05): ...`)

## Files Created/Modified

- `crates/paladin-llm/src/http_status.rs` — **new**; the helper, its tests and doc test
- `crates/paladin-llm/src/lib.rs` — `pub mod http_status;` (rustfmt alphabetised the module list)
- `crates/paladin-llm/src/openai/adapter.rs` — three sites rewired; `OPENAI_PROVIDER` const; mockito status tests (new — the file had none)
- `crates/paladin-llm/src/deepseek/adapter.rs` — `map_error` delegates; raw body at both call sites; `DEEPSEEK_PROVIDER`; two tests
- `crates/paladin-llm/src/anthropic/adapter.rs` — pre-checks then delegate; `ANTHROPIC_PROVIDER`; two tests
- `crates/paladin-llm/src/gemini/adapter.rs` — catch-all and 3xx arms; `GEMINI_PROVIDER`; three stale comments corrected; three tests
- `crates/paladin-llm/src/compat/engine.rs` — `provider_name` field, `with_provider_name`, `provider_name()`, `DEFAULT_PROVIDER_NAME`; `map_error` rewrite; three call sites; four tests updated/added; `classify_fetch_failure` comments corrected
- `crates/paladin-llm/src/{kimi,qwen,grok,ollama,openai_compatible}/adapter.rs` — `*_PROVIDER` const used by `get_provider_name` and `with_provider_name`; routing tests
- `tests/unit/llm/deepseek_adapter_test.rs`, `tests/unit/llm/anthropic_adapter_test.rs` — 500 assertions read the typed field

## Decisions Made

See `key-decisions` in the frontmatter. The load-bearing ones: (1) the compat presets prove delegation via an equivalence test rather than a private mapping; (2) provider identity is an additive builder, not a semver-visible struct field; (3) refused-redirect 3xx arms are typed `ProviderError`s with enriched messages; (4) Anthropic/Gemini credential-failure pre-checks stay ahead of the helper because their retry loops would otherwise re-send a rejected key.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The "nine adapters" are four bespoke mappers plus one shared engine**
- **Found during:** Task 2 (read_first)
- **Issue:** The plan's file:line anchors predate plan 17's `CompatEngine`; kimi, qwen, grok, ollama and openai_compatible have no non-2xx branch of their own — all five delegate to `CompatEngine::map_error`. Splitting the engine change across Tasks 2 and 3 was impossible.
- **Fix:** Rewired `CompatEngine::map_error` once in Task 2 (kimi/qwen needed it); Task 3 named the remaining three presets' engines and added their tests. Every preset `adapter.rs` calls `map_http_status(` in a routing test that compares the live adapter error with the helper's output, so the plan's nine-file count assertion measures a real delegation, not a grep-satisfying token.
- **Files modified:** `crates/paladin-llm/src/compat/engine.rs` and the five preset files
- **Commit:** `9ffff4db`, `977caef0`

**2. [Rule 2 - Missing critical functionality] `CompatEngine` had no provider name to stamp on `ProviderError`**
- **Found during:** Task 2
- **Issue:** `CompatEngineConfig` carries no provider identity; without one every compat preset's `ProviderError.provider` would have been a lie or empty.
- **Fix:** Private `provider_name` field on `CompatEngine`, additive `#[must_use] pub fn with_provider_name(self, &'static str)`, `pub fn provider_name(&self)`, and `pub const DEFAULT_PROVIDER_NAME = "openai-compatible"`. Chosen over a new pub field on `CompatEngineConfig` (a by-literal-constructible struct) to avoid `constructible_struct_adds_field` in the `paladin-llm` cargo-semver-checks job. Additive API only; plan 25-14 (semver-gate evidence) should see a minor-only diff for `paladin-llm`.
- **Files modified:** `crates/paladin-llm/src/compat/engine.rs`, five preset files
- **Commit:** `9ffff4db`, `977caef0`

**3. [Rule 1 - Bug] `tests/unit/llm/anthropic_adapter_test.rs` asserted `ProcessingError` for a 500**
- **Found during:** Task 2
- **Issue:** Not in the plan's file list, but `test_anthropic_server_error_500` matched `ProcessingError(_)` and would have failed at runtime once Anthropic's 5xx became `ProviderError`.
- **Fix:** Migrated to a `ProviderError { provider: "anthropic", status: 500 }` field assertion, in the same commit as the adapter change so the commit is green.
- **Files modified:** `tests/unit/llm/anthropic_adapter_test.rs`
- **Commit:** `9ffff4db`

**4. [Rule 1 - Bug] OpenAI's model-list non-2xx site and streaming site were also stringly**
- **Found during:** Task 2 (plan: "locate every non-2xx site")
- **Issue:** `get_available_models` built `ProcessingError(format!("HTTP {}", status))` without reading the body; the streaming site had its own shorter match.
- **Fix:** Both routed through the helper (model list now reads the body with `unwrap_or_default` before mapping).
- **Files modified:** `crates/paladin-llm/src/openai/adapter.rs`
- **Commit:** `9ffff4db`

### Scope notes (no fix needed)

- `tests/unit/mock_llm_adapter_test.rs` (listed in Task 3) contained no status-by-substring assertion and no `ProcessingError` match; left untouched, verified by grep.
- `crates/paladin-llm/src/{openai,anthropic}/vision.rs` and `openai/embedding.rs` map into `VisionError` / the embeddings error type, not `LlmError`; outside this plan's `LlmError` scope and unchanged.
- `LlmProviderError` untouched, as the plan requires.

## Deferred Issues

- **Adapter-local retry loops still classify by exclusion.** `CompatEngine::call_api_with_retry`, `DeepSeekAdapter::call_api_with_retry`, `AnthropicAdapter::execute_with_retry` and `GeminiAdapter::execute_with_retry` halt only on `AuthenticationError | InvalidPrompt | EmptyCompletion | UsageLimitExceeded`, so a Permanent `ProviderError` (403, 409, 422, 3xx) is still retried `max_retries` times inside the adapter — exactly as the pre-existing `ProcessingError` was, so no regression, but the loops do not yet consult `LlmError::transience()`. Out of this plan's scope (only the status-to-variant decision moved); transience-aware retry is Aegis's job (25-01) and RT-06 (Phase 26) re-verifies the three retry paths. Not logged to the phase `deferred-items.md` to avoid an add/add conflict with sibling wave agents; surfaced here for the orchestrator.

## Known Stubs

None — no placeholder values, TODO/FIXME markers, or unwired data paths were introduced.

## Threat Flags

None. No new network endpoint, auth path, file access or schema surface; the plan's T-25-20/21/22/23 mitigations are implemented as specified (redact-then-bound in one place with the wrong-order control test; bounded excerpts; transience by typed field; exactly one mapping helper pinned by the nine-file count).

## Verification (exit codes reported per orchestrator notes)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo test -p paladin-llm --lib` (default features) | 68 passed, exit 0 |
| `cargo test -p paladin-llm --all-features --lib` | 347 passed, exit 0 |
| `cargo test --doc -p paladin-llm --all-features` | 6 passed, exit 0 |
| `cargo test --test lib` (root binary containing `tests/unit/llm/*`) | 692 passed, 14 ignored (Tier-2 self-skip), exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

Acceptance greps: `map_http_status(` present in 9/9 `adapter.rs`; zero `ProcessingError(format!("HTTP` under `crates/paladin-llm/src/`; zero `contains("4xx"/"5xx")` assertions in the crate or the two named root test files. No `unwrap()`/`expect()`/`panic!` introduced in library code (the helper has no panicking path; new `expect`s are test-only).

## Self-Check: PASSED

- FOUND: `crates/paladin-llm/src/http_status.rs`
- FOUND: `crates/paladin-llm/src/lib.rs`, `crates/paladin-llm/src/compat/engine.rs`, `tests/unit/llm/deepseek_adapter_test.rs`, `tests/unit/llm/anthropic_adapter_test.rs`
- FOUND commits: `2bf2e145`, `b59c1980`, `9ffff4db`, `977caef0` (all on `worktree-agent-a2b590e9ed24818d6`, base `7b8815e8`)
- No tracked-file deletions between base and HEAD; working tree clean before this SUMMARY.
