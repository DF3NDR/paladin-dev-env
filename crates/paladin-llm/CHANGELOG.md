# Changelog

All notable changes to `paladin-llm` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added
- `Commissary` — a prompt-budgeting service that pre-flight-guards an assembled prompt
  against a provider's declared context window (`Commissary::verify_fits`) and
  bounded-allocates caller-prioritised material into a `Stockpile` (`Commissary::dispense`),
  never dropping shed items silently; see `src/services/commissary.rs`.

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
