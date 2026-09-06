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

## From 25-11 (worker-template handler restrictions, handler Parley, compensation-chain E2E)

- **A successful mustered task returning a non-`Edges` step has no guard.** D-22's
  `MusterHandlerMustBeDeltaOnly` covers only the *handler* path (a `Custom` handler returning
  `Goto`/`End`/`Parley`/`Muster` from inside a mustered task). A mustered task that *succeeds*
  and returns a non-`Edges` step is pre-existing behaviour outside D-22's handler scope and was
  left untouched. Candidate for a later Muster-hardening pass. Source: `25-11-SUMMARY.md`
  § "For the orchestrator's attention".

## From 25-12 (E2E-3 seam replacement, X-05 stress, kill-during-backoff and run-timeout E2E)

- **`NodeExecutionRecord` for mustered tasks carries no `task_key`.** Records produced by a
  shared worker template are distinguishable only by position and attempt, which is why the
  recovering-worker fixture had to use one worker template per task (`w1`..`w5`). An additive
  `task_key: Option<String>` on the record would let future E2E tests (and operators reading
  Chronicle output) address one mustered task directly without per-task templates.
  Source: `25-12-SUMMARY.md` § "Surfaced for deferred-items.md".

## From 25-13 (node-cache engine integration)

- **Cache eviction surface is unwired.** `cache_key::graph_prefix` / `cache_key::node_prefix`
  and the port's `invalidate(prefix)` exist, but no engine or operator path calls them yet.
  An operator-facing "evict this graph / node" surface is a candidate for a later phase.
- **`CacheKeySpec::Custom` stays deferred** per D-28 (only the declared field-set key
  composition ships in v0.10.0).
- **`Cargo.lock` gained one line (25-13)**: `blake3 = "1.8.2"` became a direct dependency edge of
  `paladin-battalion` (no new package in the graph). Noted so the semver/security gates in
  25-14 do not read it as an unexplained lockfile drift.
  Source: `25-13-SUMMARY.md` § "For the orchestrator / deferred-items.md".

## From 25-14 (close-out: manual credential-handling review, security.instructions.md)

- **The three pre-Phase-17 adapters follow redirects with a credential header attached.**
  `openai/adapter.rs`, `anthropic/adapter.rs` and `deepseek/adapter.rs` build their `reqwest`
  client with `Client::builder().timeout(..)` only, so reqwest's default policy (follow up to 10
  redirects) applies; the six Phase 17 adapters and `CompatEngine` set
  `redirect::Policy::none()` (PROV-02 / T-17-54). reqwest strips `Authorization` on a cross-host
  redirect but not Anthropic's `x-api-key`, so the manual-review rule "HTTP clients sending a
  credential header do not follow redirects" is not met by these three. Pre-existing since v0.8,
  untouched by Phase 25 (25-05 changed only their non-2xx mapping), outside this close-out plan's
  scope; recorded as an open finding rather than a fix. Candidate for Phase 26's RT-06 retry-path
  re-verification or a small `fix(llm)` of its own — align the three on `Policy::none()` and the
  typed refused-redirect `ProviderError { status: 3xx }` the compat engine already emits.
- **`RedisQueueConfig` still derives `Debug` (and `Serialize`) over a raw `redis_password`.**
  Phase 25's `RedisNodeCacheConfig` mirrored that pre-existing derive verbatim (D-27) and was
  hardened in 25-14 with a redacting manual `Debug` (T-25-70); the v0.8 queue config it copied
  was not touched. Neither type is `Debug`-formatted anywhere in-tree today, and both still derive
  `Serialize` (removing it is a semver-major trait removal, not a close-out change). Candidate for
  a storage-crate hardening pass: a redacting `Debug` on `RedisQueueConfig`, and a decision on
  whether either config should be `Serialize` at all.
- **`lcov.info` is a tracked file at the repository root.** It was committed by the pre-v0.8 CLI
  unit-test commits (`5d487584`, `a2380777`) and is overwritten by every `cargo llvm-cov ...
  --lcov --output-path lcov.info` run (`scripts/coverage.sh`, `make coverage`, CI's `coverage`
  job), so a local coverage measurement dirties the working tree with a 5 MB artifact. 25-14
  restored it with `git checkout -- lcov.info` after measuring rather than committing a stale
  report. Candidate for a `chore`: `git rm --cached lcov.info` plus a `.gitignore` entry.
- **`parley_resume_stress::stress_run_completes_within_the_timeout_guard` is wall-clock
  sensitive.** It failed once during 25-14's first `cargo test --workspace` run (a 10 s
  `tokio::time::timeout` guard around ten concurrent SQLite resumes, with the container's load
  average at ~7 on 8 cores from sibling work) and passed on the standalone re-run (34/34 in
  3.7 s) and on the full workspace re-run. Nothing in Phase 25 touches it. Candidate for a
  Phase 24 follow-up: widen the guard or move the scenario onto a paused clock.
