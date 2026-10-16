# Changelog

All notable changes to `paladin-llm` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

- Phase 50 (provider parity and configuration-driven model swap; PARITY-01..08; ADR-0061):
  - **Sampling policy and registry.** `sampling` (the one wire filter every request builder calls: an
    explicit value the model cannot honour is refused at build, any other is omitted with a
    `warn!` and `paladin.params_omitted`; `paladin.response_format_adjusted`; the `max_tokens`
    refusal) and `registry` (`ModelOverride`, `UNPROFILED_MODEL_METADATA_KEY`, the dated per-vendor
    seeded tables for Anthropic, OpenAI, DeepSeek, Kimi, Qwen, Grok, Gemini and Ollama, every value
    traced to the 2026-10-15 operator probe; `ANTHROPIC_DEFAULT_MODEL` and
    `DEEPSEEK_DEFAULT_MODEL`). All nine adapters declare `model_profile`; the three decorators
    forward it and a fallback chain reports the intersection of its hops.
  - **Configuration.** `LlmProviderFactory::create_from_config` and `is_configured`; the
    `llm.<provider>` block gains `max_tokens`, `context_window` and `models.<model-id>`, with
    optional `api_key`, exact `${NAME}` placeholders and the 300 s reasoning-model timeout default.
    Each of the nine `*Config` types gains `max_tokens`, `max_retries`, `context_window` and
    `model_overrides` with `with_max_tokens`, `with_max_retries`, `with_context_window` and
    `with_model_overrides` (and redacting `Debug`).
  - **Windows.** `window::effective_capabilities` and `Commissary::for_model` ration against the
    model's own window (per-model override, then adapter-wide `context_window`, then seeded, then
    fallback).
  - **Responses.** `response_guards` (one empty-output rule: `EmptyCompletion`; typed
    `LlmError::Refusal`), reasoning text on `LlmResponse.reasoning` and `StreamingResponse.reasoning`,
    compat `cache_write_tokens`, Anthropic refusal, `stop_details` and thinking text, Gemini on the
    shared guard, and the `paladin.*` keys stamped on a stream's terminal chunk.
  - **Fallback.** `FallbackStep`, `FallbackLlmAdapter::from_steps`, `hop_on_refusal`,
    `SERVED_MODEL_METADATA_KEY` (`paladin.served_model`) and `FallbackChainError::EmptyStepModel`.
  - **Tests and tools.** The conformance suite grows to seventeen cases and measures all nine
    adapters (Anthropic included); `live_vendor_smoke` covers all nine providers with `--probe`,
    `--models` and `--dry-run`.

- Phase 43 (rate pacing, the Cadence; PACE-01, PACE-02; ADR-0058): `CadenceLlmAdapter`,
  `CadenceSettings`, `CadenceWiring` and `with_cadence` (the `cadence` module), a stateless `LlmPort`
  decorator that holds a call while its per-(provider, model) gate is closed and reports a provider
  429 to the `CadencePort`; it never retries, refuses a wait beyond `max_wait` without calling the
  provider, paces `generate_stream` like `generate`, and spreads the callers one gate releases. It
  composes as `Pricing(Cadence(provider))`. The `rate_limit_headers` module
  (`hints_from_headers`, `parse_retry_after`, `parse_go_duration`, `RateLimitHeaderFamily` and the
  header-name constants) turns a 429's headers into `RateLimitHints`: `Retry-After` in
  delta-seconds or HTTP-date, `retry-after-ms`, OpenAI's `x-ratelimit-*` and Anthropic's
  `anthropic-ratelimit-*`, every name verified against the provider's own documentation. The
  `map_http_status_with_hints` function carries those hints through the shared status mapping
  (`map_http_status` is unchanged). `FallbackLlmAdapter::with_cadence` wraps every hop in the
  decorator. Conformance case 10, `rate_limit_is_surfaced_once_with_its_retry_delay`, joins the
  shared adapter suite. `httpdate` is now a direct dependency and `rand` a required one.
- `Commissary` — a prompt-budgeting service that pre-flight-guards an assembled prompt
  against a provider's declared context window (`Commissary::verify_fits`) and
  bounded-allocates caller-prioritised material into a `Stockpile` (`Commissary::dispense`),
  never dropping shed items silently; see `src/services/commissary.rs`.
- `pricing` module: `PricingLlmAdapter`/`with_pricing`, a `FallbackLlmAdapter`-shaped `LlmPort`
  decorator that prices every call — streaming and non-streaming — at the served model's own
  usage, with a process-wide, capacity-bounded warn-once log line per unpriced model name
  (PRICE-02, PRICE-03).

### Changed

- Phase 50: every adapter builds the user turn through one shared rule (the query, then the
  context, each only when non-empty after trimming, joined by one blank line); the OpenAI adapter
  previously sent an empty user message when only the query was set (PARITY-08). OpenAI no longer
  sends a fabricated `temperature` or `top_p`. The Kimi preset sends an explicit `1.0` and `0.95`
  instead of dropping them (and the seeded Kimi models state `not sent`). Compat presets send
  stop sequences, except Grok. Default models and windows come from the registry (Anthropic
  `claude-haiku-5-5`, DeepSeek `deepseek-flash`, Kimi `kimi-k2.6`, Qwen `qwen3.8-flash`, Grok
  `grok-4.3`, Gemini `gemini-3.5-flash-lite`), and a direct `*Config` constructor defaults a seeded
  reasoning model to 300 s. An adapter-wide `context_window` now beats seeded windows. Gemini's
  `RECITATION`, `BLOCKLIST`, `PROHIBITED_CONTENT` and `SPII` finishes are `ContentFilter` (they
  were `Error`). The `llm.<provider>` blocks now take effect in the server, and a block's
  `default_temperature` is judged against the model's profile.

- Every adapter now surfaces its first 429 instead of retrying it inside its own loop (the OpenAI
  adapter, the compat engine, the Anthropic adapter and, beyond the original scope, DeepSeek and
  Gemini), carrying the provider's `Retry-After` and rate-limit headers on
  `LlmError::RateLimitExceeded`; network, timeout and 5xx retries are unchanged. An OpenAI 429
  coded `insufficient_quota` and an Anthropic 429 coded `enforced_spend_limit_reached` map to
  `UsageLimitExceeded`, not to a rate limit. A `FallbackLlmAdapter` built with `with_cadence` now
  retries a 429 on the same hop after its gate, for up to the pace budget, before hopping; a 5xx,
  timeout or network error still hops at once, and a chain built without `with_cadence` is unchanged
  (Cadence persona, ADR-0058).

### Breaking Changes

- Phase 50 (PARITY-03, PARITY-06; owner decision option-a): the nine `*Config` types and
  `LlmProviderConfig` are `#[non_exhaustive]`, so a struct literal outside the crate no longer
  compiles; use `new(..)` or `from_env()` plus the `with_*` builders or field assignment (the `new`
  signatures are unchanged). `FallbackChainError` gains the struct variant `EmptyStepModel { index }`
  (a numeric cast of `EmptyChain` no longer compiles). Registered in `MIGRATION.md` section 9.2 and
  the crate's `cargo-semver-checks` lints table.

## [0.10.1] - 2026-09-20

Patch release carried by the workspace-wide version bump (0.10.0 -> 0.10.1). No source
change in this crate — see the root `CHANGELOG.md`'s `[0.10.1]` section for the two
release-pipeline defects this patch fixes.

## [0.10.0] - 2026-09-10

### Added
- `FallbackLlmAdapter` — a new `LlmPort` implementation that hops across a configured provider
  chain on `Transient`/`Unknown` failures only, with a first-chunk streaming rule and a
  per-hop `FallbackHop` trace event; records the serving provider on `PaladinResult::served_by`
  (paladin-core, FT-05).
- A shared LLM conformance suite (`26-14`) measuring the OpenAI, Anthropic and DeepSeek adapters
  against the same contract (0 gaps at landing).
- Gemini native JSON mode and `response_format` wiring across the OpenAI, compat-engine, and
  DeepSeek wire adapters (RT-05, D-28).

### Changed
- Every provider adapter's non-2xx handling now routes through a shared `map_http_status` helper,
  redacting the response body before bounding it and returning a typed `LlmError::ProviderError
  { status, .. }` in place of the legacy untyped `ProcessingError(String)` (FT-FR-01, FT-FR-16;
  applies to openai, deepseek, anthropic, kimi, qwen, gemini, grok, ollama, openai_compatible).
- All 36 remaining `LlmRequest` construction sites in this crate migrated to the
  `LlmRequest::new` + `with_*` builder pattern (paladin-ports RT-FR-17).

### Fixed
- Redirects are refused on the OpenAI/Anthropic/DeepSeek HTTP clients so a credential header
  cannot be forwarded to an attacker-influenced host (CR-02).
- Anthropic usage-cap and parse-failure response bodies are redacted before being bounded/excerpted
  into an error, and the DeepSeek adapter's credential-redaction routine was deduplicated (CR-01,
  WR-01 follow-ons).
- Credential-shaped redaction markers (`key=`/`token=`) now require a preceding word boundary,
  avoiding a false-positive redaction inside an unrelated token (WR-01).

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Feature-flag release notes tracking for provider families (`openai`, `anthropic`, `deepseek`, `mock`, `vision`, `openai-embeddings`).

### Changed
- Provider API stability documentation aligned with crate-tier stability expectations.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
