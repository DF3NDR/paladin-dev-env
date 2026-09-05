# Phase 25 — Deferred Items

Out-of-scope observations surfaced by executors during Phase 25. Logged by the orchestrator
after wave merges so sibling worktree agents never race on this file.

## From 25-05 (shared `map_http_status` helper)

- **Adapter-local retry loops still classify by exclusion.** `CompatEngine::call_api_with_retry`,
  `DeepSeekAdapter::call_api_with_retry`, `AnthropicAdapter::execute_with_retry` and
  `GeminiAdapter::execute_with_retry` halt only on
  `AuthenticationError | InvalidPrompt | EmptyCompletion | UsageLimitExceeded`, so a Permanent
  `ProviderError` (403, 409, 422, 3xx) is still retried `max_retries` times inside the adapter —
  exactly as the pre-existing `ProcessingError` was (no regression). The loops do not yet consult
  `LlmError::transience()`. Transience-aware retry is Aegis's job (25-01); RT-06 (Phase 26)
  re-verifies the three retry paths. Source: `25-05-SUMMARY.md` § Deferred Issues.
