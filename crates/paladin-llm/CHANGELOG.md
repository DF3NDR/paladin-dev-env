# Changelog

All notable changes to `paladin-llm` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

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

- Every adapter now surfaces its first 429 instead of retrying it inside its own loop (the OpenAI
  adapter, the compat engine, the Anthropic adapter and, beyond the original scope, DeepSeek and
  Gemini), carrying the provider's `Retry-After` and rate-limit headers on
  `LlmError::RateLimitExceeded`; network, timeout and 5xx retries are unchanged. An OpenAI 429
  coded `insufficient_quota` and an Anthropic 429 coded `enforced_spend_limit_reached` map to
  `UsageLimitExceeded`, not to a rate limit. A `FallbackLlmAdapter` built with `with_cadence` now
  retries a 429 on the same hop after its gate, for up to the pace budget, before hopping; a 5xx,
  timeout or network error still hops at once, and a chain built without `with_cadence` is unchanged
  (Cadence persona, ADR-0058).

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
