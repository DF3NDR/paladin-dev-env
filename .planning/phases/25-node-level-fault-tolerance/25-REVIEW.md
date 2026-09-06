---
phase: 25-node-level-fault-tolerance
reviewed: 2026-09-06T03:38:17Z
depth: standard
files_reviewed: 79
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .github/workflows/ci.yml
  - .project/v0.10.0/08-traceability-matrix.md
  - crates/doc-examples/src/fault_tolerance.rs
  - crates/doc-examples/src/lib.rs
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-battalion/src/conclave_execution_service.rs
  - crates/paladin-battalion/src/engine/bridges.rs
  - crates/paladin-battalion/src/engine/cache_key.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/heartbeat.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/node.rs
  - crates/paladin-battalion/src/engine/registries.rs
  - crates/paladin-battalion/src/engine/retry.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/error_handler.rs
  - crates/paladin-battalion/src/lib.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-battalion/src/retry_predicate.rs
  - crates/paladin-core/Cargo.toml
  - crates/paladin-core/src/lib.rs
  - crates/paladin-core/src/platform/container/aegis.rs
  - crates/paladin-core/src/platform/container/battalion/mod.rs
  - crates/paladin-core/src/platform/container/battlefield.rs
  - crates/paladin-core/src/platform/container/execution_result.rs
  - crates/paladin-core/src/platform/container/heartbeat.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/node_cache.rs
  - crates/paladin-core/src/platform/container/node_error.rs
  - crates/paladin-core/src/platform/container/paladin_error.rs
  - crates/paladin-core/src/platform/container/transience.rs
  - crates/paladin-core/src/platform/container/waypoint.rs
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/grok/adapter.rs
  - crates/paladin-llm/src/http_status.rs
  - crates/paladin-llm/src/kimi/adapter.rs
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/mock.rs
  - crates/paladin-llm/src/ollama/adapter.rs
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/src/openai_compatible/adapter.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/node_cache_port.rs
  - crates/paladin-ports/src/output/paladin_port.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-storage/Cargo.toml
  - crates/paladin-storage/src/lib.rs
  - crates/paladin-storage/src/node_cache/contract_tests.rs
  - crates/paladin-storage/src/node_cache/in_memory.rs
  - crates/paladin-storage/src/node_cache/mod.rs
  - crates/paladin-storage/src/node_cache/redis.rs
  - crates/paladin-storage/src/waypoint/contract_tests.rs
  - crates/paladin-storage/src/waypoint/in_memory.rs
  - crates/paladin-storage/src/waypoint/postgres.rs
  - crates/paladin-storage/src/waypoint/sqlite.rs
  - docs/src/SUMMARY.md
  - docs/src/user-guides/fault-tolerance.md
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/paladin/temperature_service.rs
  - src/application/services/parley/adapter.rs
  - src/config/engine.rs
  - src/config/mod.rs
  - src/config/node_cache.rs
  - src/core/platform/mod.rs
  - tests/cli/environment_tests.rs
  - tests/helpers/mock_paladin_port.rs
  - tests/integration/aegis_retry_stress_test.rs
  - tests/integration/e2e_compensation_chain_test.rs
  - tests/integration/e2e_muster_defer_order_test.rs
  - tests/integration/subgraph_formation_in_campaign_test.rs
  - tests/unit/handoff_service_test.rs
  - tests/unit/llm/anthropic_adapter_test.rs
  - tests/unit/llm/deepseek_adapter_test.rs
  - tests/unit/paladin_execution_service_test.rs
findings:
  critical: 2
  warning: 3
  info: 1
  total: 6
status: issues_found
---

# Phase 25: Code Review Report

**Reviewed:** 2026-09-06T03:38:17Z
**Depth:** standard
**Files Reviewed:** 79 (via `git diff --name-only` from `diff_base`)
**Status:** issues_found

## Summary

This phase adds a large, carefully-engineered node-level fault-tolerance layer
(Aegis retry/timeout/cache/error-handler policies, a shared HTTP-status-to-
`LlmError` mapper, a structured `PaladinError::LlmFailure`/`NodeError` error
taxonomy, and a model-fallback adapter) across `paladin-core`,
`paladin-battalion`, `paladin-llm`, and `paladin-storage`. The engineering
quality of the reviewed code is unusually high: exhaustive rustdoc reasoning
attached to nearly every non-trivial decision, table-driven transience tests
covering every enum variant, paused-clock backoff tests, parameterized SQL
throughout the Postgres/SQLite waypoint stores, and a dedicated
redact-then-bound credential-handling discipline (`paladin-llm::redaction`,
`http_status::map_http_status`) that is unit-tested against the exact
truncation-boundary attack it exists to prevent.

Despite that rigor, the credential-redaction discipline the crate otherwise
enforces has one real gap (the Anthropic adapter's usage-cap "regain access"
hint is built directly from the raw, unredacted response body) and one
pre-existing gap this phase's own file list still carries (three of the nine
LLM adapters do not set `redirect::Policy::none()`, unlike every adapter this
phase's sibling work already hardened). Both are documented below. The retry
loop this phase's own `LlmFailure.transience` field could have hardened at
the facade layer (`PaladinExecutionService`) was left retrying `Permanent`
failures unconditionally — a real behavioral gap, though one the codebase's
own tests document as an intentional in-scope decision for this phase, so it
is recorded as a warning rather than a blocker.

## Critical Issues

### CR-01: Anthropic usage-cap "regain access" hint is extracted from the raw, unredacted response body — bypasses the crate's redact-before-bound discipline

**File:** `crates/paladin-llm/src/anthropic/adapter.rs:320-334` (dispatch), `654-682` (`extract_regain_hint`)

**Issue:** `AnthropicAdapter::map_error`'s `400` arm for a usage-cap body calls
`extract_regain_hint(body)` directly on the **raw** HTTP response body:

```rust
400 if body.contains(ANTHROPIC_USAGE_CAP_SIGNATURE) => LlmError::UsageLimitExceeded {
    provider: ANTHROPIC_PROVIDER.to_string(),
    regain_hint: extract_regain_hint(body),
},
```

Every other non-2xx path in this file (and every other adapter in the crate)
routes the raw body through `crate::redaction::redact_credentials` /
`crate::http_status::map_http_status` before any excerpt of it is placed on
an error value — `http_status.rs`'s own module doc states this ordering is
"load-bearing" and is unit-tested against exactly the truncation-boundary
credential leak this function is now exempt from
(`excerpt_is_redacted_before_it_is_bounded`). `extract_regain_hint` has no
redaction pass at all: it slices the substring following `"regain access"`
out of the untouched provider body, bounds it to 120 characters, and returns
it verbatim as `LlmError::UsageLimitExceeded.regain_hint` — a value the
adapter's own rustdoc says is "displayed VERBATIM to the operator."

`body` here is exactly the class of input this codebase's own security
documentation calls "attacker- or third-party-influenced" (see
`node_error.rs`'s "Security: redact before you bound" module doc, and
`http_status.rs`'s "a remote, attacker-influenceable string that may echo the
request back verbatim (gateways do)"). A compromised or misconfigured
gateway/proxy in front of `AnthropicConfig::base_url` (operator-configurable)
that echoes request headers near a crafted "regain access" phrase — or any
future usage-cap message that happens to include request context — would
have that content forwarded into an operator-facing error field with zero
redaction, unlike every other body-derived string in this codebase.

**Fix:** Redact before extracting/bounding, exactly as `map_http_status`
already does for every other status:

```rust
400 if body.contains(ANTHROPIC_USAGE_CAP_SIGNATURE) => {
    let redacted = crate::redaction::redact_credentials(body, &self.config.api_key);
    LlmError::UsageLimitExceeded {
        provider: ANTHROPIC_PROVIDER.to_string(),
        regain_hint: extract_regain_hint(&redacted),
    }
}
```

(`extract_regain_hint`'s own char-bounding then still applies to the
redacted string, so the ordering stays redact-then-bound throughout.)

### CR-02: `openai`/`anthropic`/`deepseek` HTTP clients still follow redirects, replaying credential headers to a redirect target — violates the project's own written security rule

**File:** `crates/paladin-llm/src/openai/adapter.rs:193` (`Client::builder()`), `crates/paladin-llm/src/anthropic/adapter.rs:150`, `crates/paladin-llm/src/deepseek/adapter.rs:393`

**Issue:** `.github/instructions/security.instructions.md` (imported by
`CLAUDE.md` and explicitly called out for this review) states as a manual
review requirement: *"HTTP clients sending a credential header do not follow
redirects, so the header cannot be forwarded to an attacker-influenced
host."* Every adapter this crate has hardened for this — `CompatEngine`
(kimi/qwen/grok/ollama/openai-compatible) and the bespoke Gemini adapter —
constructs its `reqwest::Client` with `redirect_policy: Some(Policy::none())`
and documents exactly this threat in its own rustdoc
(`compat/engine.rs:180-201`, `gemini/adapter.rs:56-59`).

`openai/adapter.rs`, `anthropic/adapter.rs` and `deepseek/adapter.rs` — all
three in this phase's own reviewed file list, both because plan 25-05
modified their non-2xx mapping and because they are exercised by this
phase's `map_http_status` migration — still build their `Client` with no
`.redirect(...)` call at all, i.e. reqwest's default policy (follow up to 10
hops). Concretely:

- The Anthropic adapter sends its credential as a custom `x-api-key` header
  (`build_headers`, line 165). reqwest's built-in cross-host header
  stripping only removes `Authorization`, `Cookie`, `Cookie2`,
  `Proxy-Authorization` and `WWW-Authenticate` on a cross-host redirect — it
  does **not** strip `x-api-key`. A compromised/malicious/misconfigured
  `api.anthropic.com`-fronting endpoint returning a `3xx` can therefore have
  the operator's live Anthropic key replayed verbatim to any
  attacker-influenced `Location`.
- OpenAI and DeepSeek send `Authorization: Bearer <key>`, which reqwest does
  strip on a *cross-host* redirect but still forwards on a *same-host*
  redirect (e.g. a same-host path-based bounce), and in either case the
  request itself is silently retried against a URL the operator never
  configured, rather than surfacing the actionable "refused redirect"
  `ProviderError` every other adapter in this crate now returns.

This is already recorded as an open, deferred finding in
`.planning/phases/25-node-level-fault-tolerance/deferred-items.md` and the
25-14 close-out summary — it is not new to this phase — but it remains
unresolved in three files this phase's own file list carries, and it
directly contradicts a written, imported project security rule rather than
being a stylistic gap. Recording it here so it is visible from the code
review artifact, not only a deferred-items note.

**Fix:** Add `.redirect(reqwest::redirect::Policy::none())` to each of the
three `Client::builder()` call sites, and add the same `300..=399 =>
ProviderError { .. }` arm `CompatEngine::map_error`/`GeminiAdapter::map_error`
already carry, so a refused redirect surfaces as an actionable error instead
of reqwest's default follow behavior.

## Warnings

### WR-01: DeepSeek adapter duplicates the crate's shared credential-redaction logic instead of using it

**File:** `crates/paladin-llm/src/deepseek/adapter.rs:256-362`

**Issue:** `crates/paladin-llm/src/redaction.rs`'s own module doc states it
was "Extracted from the DeepSeek adapter (`deepseek/adapter.rs:250-356`) so
both the shared OpenAI-compatible core... and the bespoke Gemini adapter...
share one implementation of this security-critical behaviour." That
extraction happened, but `deepseek/adapter.rs` was never migrated to import
from `crate::redaction` — it still carries its own byte-for-byte copy of
`RESPONSE_EXCERPT_CHAR_BUDGET`, `CREDENTIAL_PLACEHOLDER`, `bounded_excerpt`,
`redact_token_after`, and `redact_credentials` (lines 261-362), used by its
own `diagnostic_excerpt` (line 501) for parse-failure diagnostics. Two
independent copies of the same security-critical redaction routine now exist
in the crate; a future fix to the shared module (a new credential shape, a
bug fix in the truncation-boundary handling) will silently not apply to
DeepSeek unless someone remembers to patch both.

**Fix:** Replace `deepseek/adapter.rs`'s local `redact_credentials` /
`bounded_excerpt` / `redact_token_after` / `CREDENTIAL_PLACEHOLDER` /
`RESPONSE_EXCERPT_CHAR_BUDGET` with `use crate::redaction::{..., diagnostic_excerpt}` and delete the duplicated block, mirroring what `compat/engine.rs`
and `gemini/adapter.rs` already do.

### WR-02: `PaladinExecutionService`'s buffered retry loops retry `Permanent`-classified LLM failures unconditionally, despite this phase adding the typed `transience` field they could gate on

**File:** `src/application/services/paladin/paladin_execution_service.rs:1700-1744` (`execute_with_retry_and_temperature`), `:1822-1868` (`execute_with_retry`)

**Issue:** Both retry loops convert the port's `LlmError` via
`to_paladin_error(&e)` — this phase's own new conversion that attaches a
typed `Transience` to the resulting `PaladinError::LlmFailure` — but then
match on the result with only two special cases:

```rust
Err(PaladinError::CircuitBreakerOpen) => { /* fail fast */ }
Err(_e) if attempt >= max_attempts => { /* MaxRetriesExceeded */ }
Err(e) => { /* sleep(backoff_ms) and retry, unconditionally */ }
```

There is no check of `e.transience()` (or the now-available
`PaladinError::transience()`) before the unconditional-retry arm. A
`Permanent` failure — e.g. `LlmError::AuthenticationError` (rejected
credential) or `LlmError::InvalidPrompt` — is retried up to
`min(max_loops, 10)` times with exponential backoff (100ms, 200ms, 400ms,
...), re-sending the identical rejected request each time. This is
inconsistent with:

- every LLM adapter's own retry loop in this same PR
  (`anthropic::execute_with_retry`, `deepseek::call_api_with_retry`,
  `compat::CompatEngine::call_api_with_retry`), each of which explicitly
  excludes `AuthenticationError | InvalidPrompt | EmptyCompletion |
  UsageLimitExceeded` from its retryable set for exactly this reason
  ("needs operator intervention, not a retry" / "will not clear on
  backoff");
- the new Aegis/`engine::retry::should_retry`, which gates every
  superstep-engine retry on `Transience::Transient` (or
  `TransientAndUnknown`) by design (D-15/FT-FR-05).

This file's own test suite (`buffered_retry_sites_trip_the_circuit_breaker_like_the_legacy_variant`) documents this as a deliberately
unchanged, pre-existing behavior under this phase's X-03 "no behavior
change" scope constraint — so it is not a regression this phase introduced —
but the phase *did* wire the exact typed data (`LlmFailure.transience`) that
would make the fix a small, local change, and left it unused at this call
site. Left as-is, a misconfigured API key burns the full retry budget (and
multi-second backoff) on every single execution rather than failing fast.

**Fix:** Gate the retry arm on `e.transience() != Transience::Permanent`
(or reuse `PaladinError::is_retryable()`/a transience-aware check), returning
the error immediately for a `Permanent` classification instead of falling
into the backoff-and-retry arm.

## Info

### IN-01: `RedisNodeCache::scan_pattern` builds a `SCAN MATCH` pattern by naive string interpolation, with no escaping of Redis glob metacharacters in `prefix`

**File:** `crates/paladin-storage/src/node_cache/redis.rs:114-118`

**Issue:** `scan_pattern` is `format!("{}:{}*", config.key_prefix, prefix)`.
If a caller-supplied `prefix` (ultimately derived from a `NodeId` or graph
fingerprint composed by `engine::cache_key`) ever contains a Redis glob
metacharacter (`*`, `?`, `[`, `]`), `invalidate(prefix)` would match a
broader (or narrower) key set than the literal prefix intended — e.g. a node
id containing `*` could cause `invalidate` to sweep sibling nodes' cache
entries. `NodeId`/`FieldName` validation elsewhere in this codebase is fairly
restrictive, so this is unlikely to be reachable with today's identifier
grammar, but there is no defensive escaping here the way, for example, the
Postgres/SQLite stores use bound parameters instead of string interpolation
for every query. Worth a short comment or an escape pass if `NodeId`'s
allowed character set is ever widened.

**Fix:** Escape `*`, `?`, `[`, `]` in `prefix` before composing the `SCAN
MATCH` pattern (or document why `NodeId`'s grammar guarantees these
characters can never appear).

---

_Reviewed: 2026-09-06T03:38:17Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
