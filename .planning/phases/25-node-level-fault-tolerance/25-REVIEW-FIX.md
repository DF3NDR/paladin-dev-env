---
phase: 25-node-level-fault-tolerance
fixed_at: 2026-09-06T12:06:11Z
review_path: .planning/phases/25-node-level-fault-tolerance/25-REVIEW.md
iteration: 1
fix_scope: critical_warning
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
additional_fixes: 1
---

# Phase 25: Code Review Fix Report

**Fixed at:** 2026-09-06T12:06:11Z
**Source review:** .planning/phases/25-node-level-fault-tolerance/25-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (CR-01, CR-02, WR-01, WR-02)
- Fixed: 4
- Skipped: 0
- Out of scope (not attempted): IN-01

## Fixed Issues

### CR-01: Anthropic usage-cap "regain access" hint extracted from raw, unredacted body

**Files modified:** `crates/paladin-llm/src/anthropic/adapter.rs`
**Commit:** `21e9c989`
**Applied fix:** `AnthropicAdapter::map_error`'s `400` usage-cap arm now redacts
`body` via `crate::redaction::redact_credentials(body, &self.config.api_key)`
before handing it to `extract_regain_hint`, restoring the crate-wide
redact-then-bound discipline (matching `map_http_status`'s ordering).
`extract_regain_hint`'s own char-bounding still applies to the redacted
string, so ordering stays redact-then-bound throughout.

Added a regression test
(`map_error_400_usage_cap_redacts_the_configured_api_key_before_extracting_the_regain_hint`)
that embeds the adapter's configured API key inside a fake usage-cap body
next to "regain access" prose and asserts the key never survives into
`regain_hint`, while surrounding diagnostic prose (a date) does.

**Verification:**
- `cargo fmt --all --check` — pass
- `cargo check --workspace --all-targets --all-features` — pass
- `cargo test -p paladin-llm --all-features anthropic` — 29 passed, 0 failed
- Commit-hook `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass

### CR-02: openai/anthropic/deepseek HTTP clients followed redirects

**Files modified:** `crates/paladin-llm/src/openai/adapter.rs`,
`crates/paladin-llm/src/anthropic/adapter.rs`,
`crates/paladin-llm/src/deepseek/adapter.rs`,
`tests/unit/llm/anthropic_adapter_test.rs`,
`tests/unit/llm/deepseek_adapter_test.rs`
**Commit:** `2ea6328b`
**Applied fix:** Added `.redirect(reqwest::redirect::Policy::none())` to all
three adapters' `Client::builder()` call sites, mirroring
`CompatEngine::new`/`GeminiAdapter::new`. Added a `300..=399` arm to each
adapter's `map_error` (OpenAI needed a new `map_error` wrapper method since it
previously called `map_http_status` directly at each call site; all three call
sites were updated to route through it) that returns an actionable
`LlmError::ProviderError` naming the refused redirect, mirroring
`CompatEngine::map_error`/`GeminiAdapter::map_error`'s existing wording.

Added an in-file `map_error` unit test per adapter (openai, anthropic,
deepseek) asserting the `300..=399` mapping, plus end-to-end mockito
regression tests in the anthropic and deepseek integration suites
(`test_anthropic_client_refuses_to_follow_a_redirect`,
`test_deepseek_client_refuses_to_follow_a_redirect`, and an in-crate
`openai_client_refuses_to_follow_a_redirect`) that mock a `302` response with
a `Location` header pointing at a second mock expecting **zero** calls,
proving the redirect target is never contacted (not just asserting on the
returned error shape).

**Verification:**
- `cargo fmt --all --check` — pass
- `cargo check --workspace --all-targets --all-features` — pass
- `cargo test -p paladin-llm --all-features` — 368 passed, 0 failed
- `cargo test -p paladin-ai --all-features anthropic_adapter_test` — 10 passed, 0 failed (both binaries that include this integration file)
- `cargo test -p paladin-ai --all-features deepseek_adapter_test` — 9 passed, 0 failed (both binaries)
- Commit-hook `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass

### WR-01: DeepSeek adapter duplicated the crate's shared redaction module

**Files modified:** `crates/paladin-llm/src/deepseek/adapter.rs`,
`crates/paladin-llm/src/redaction.rs`
**Commit:** `1e50ca3f`
**Applied fix:** Removed `deepseek/adapter.rs`'s local
`RESPONSE_EXCERPT_CHAR_BUDGET`, `CREDENTIAL_PLACEHOLDER`, `bounded_excerpt`,
`redact_token_after`, `redact_credentials` and replaced them with
`use crate::redaction::{RESPONSE_EXCERPT_CHAR_BUDGET, bounded_excerpt,
redact_credentials}` (plus a test-only import of `CREDENTIAL_PLACEHOLDER`),
mirroring `compat/engine.rs` and `gemini/adapter.rs`. `diagnostic_excerpt`'s
method body and signature are unchanged — it now calls the imported
functions instead of local copies.

`redaction.rs`'s `CREDENTIAL_PLACEHOLDER` was widened from private to
`pub(crate)` (the minimal visibility change needed) so DeepSeek's existing
redaction tests (`diagnostic_excerpt_never_echoes_the_configured_api_key`,
`redact_credentials_masks_bearer_and_sk_tokens_it_was_not_configured_with`,
etc.) could keep asserting against the shared constant rather than a
copy-pasted literal. No existing test coverage was deleted, only repointed at
the shared module.

**Verification:**
- `cargo fmt --all --check` — pass
- `cargo check --workspace --all-targets --all-features` — pass
- `cargo test -p paladin-llm --all-features deepseek` — 33 passed, 0 failed
- `cargo test -p paladin-llm --all-features` (full crate) — 368 passed, 0 failed
- Commit-hook `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass

### WR-02: PaladinExecutionService retried Permanent-classified LLM failures

**Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
**Commit:** `724db484`
**Applied fix:** Both `execute_with_retry_and_temperature` and
`execute_with_retry` gained a new match arm —
`Err(e) if e.transience() == Transience::Permanent`, checked immediately
after the `CircuitBreakerOpen` fail-fast arm and *before* the
`attempt >= max_attempts` exhaustion arm — that returns the error immediately
without sleeping or retrying. Decision on surfaced error type (per finding
guidance): the underlying typed error (typically `PaladinError::LlmFailure`)
is returned directly rather than wrapping it in `MaxRetriesExceeded`, so
operators see the real cause (e.g. "invalid API key") on the very first
attempt instead of after burning the full retry budget. This is documented in
the new rustdoc on `execute_with_retry_and_temperature`'s `# Errors` section
and inline comments on both new match arms.

`Transience::Transient` and `Transience::Unknown` are unaffected — they still
fall through to the pre-existing exhaustion/backoff-and-retry arms unchanged.
`PaladinError::CircuitBreakerOpen` still fails fast via its own pre-existing
arm, unaffected.

Test changes:
- The existing `buffered_retry_sites_trip_the_circuit_breaker_like_the_legacy_variant`
  test (which the review noted documents the pre-fix behavior) was **not**
  modified — it exercises a `provider_503` failure, which is
  `Transience::Transient`, so this fix does not change its outcome or
  assertions. Left as a regression guard that the fix does not touch
  Transient/CircuitBreakerOpen control flow.
- Added `permanent_failure_is_not_retried_by_buffered_retry_sites`: drives
  both retry sites with `auth_failure` (Permanent) against a generous circuit
  breaker (threshold 5) and asserts `port.calls() == 1` and the returned error
  is `PaladinError::LlmFailure { transience: Permanent, .. }` (not
  `MaxRetriesExceeded`), for both sites.
- Added `transient_and_unknown_failures_still_retry_until_max_attempts`:
  drives both sites with `provider_503` (Transient) and a `ProcessingError`
  (Unknown, since it has no typed field to distinguish it from Permanent) and
  asserts both attempts are spent (`port.calls() == 2`) before
  `MaxRetriesExceeded` surfaces — confirming (c) from the finding-specific
  guidance: Unknown and Transient still retry, only Permanent short-circuits.

**Verification:**
- `cargo fmt --all --check` — pass
- `cargo check --workspace --all-targets --all-features` — pass
- `cargo test -p paladin-ai --all-features --lib "application::services::paladin::paladin_execution_service"` — 27 passed, 0 failed (includes the 3 tests above)
- `cargo test -p paladin-ai --all-features paladin_execution_service` (also runs `tests/unit/paladin_execution_service_test.rs`) — 21 passed, 0 failed
- Commit-hook `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass

**Note on verification tier:** This finding's fix is a control-flow/logic
change (adds a new match arm gating retry on classified transience), not a
pure syntax fix. Tier 1/2 verification (compiles, all cited tests pass,
including two new tests specifically designed to fail if the logic were
wrong) all passed, and the fix mirrors an established pattern already proven
correct in three sibling adapters (`anthropic::execute_with_retry`,
`deepseek::call_api_with_retry`, `compat::CompatEngine`'s retry loop) plus
the engine-level `should_retry` gate. Recommend a human still skim the two new
match arms and their placement (before vs. after the `attempt >= max_attempts`
arm) given the logic-correctness caveat in the fixer's own verification
policy.

### Follow-on (beyond REVIEW.md scope): Anthropic deserialization-failure excerpt was bounded without redaction

**Found by:** the orchestrator while verifying CR-01/WR-01 — not a REVIEW.md finding,
recorded here so the fix pass is complete on its own.
**Files modified:** `crates/paladin-llm/src/anthropic/adapter.rs`,
`tests/unit/llm/anthropic_adapter_test.rs`
**Commit:** `6b566916`
**Issue:** `AnthropicAdapter::generate`'s deserialization-failure path embedded
`bounded_excerpt(&body, RESPONSE_EXCERPT_CHAR_BUDGET)` of a 2xx response body
straight into `LlmError::ProcessingError` with no redaction pass, through a
private copy of the bounding helper (the same duplication class WR-01 removed
from DeepSeek). The body is third-party-influenceable (a gateway in front of
`base_url` can echo request headers, including `x-api-key`), and
`security.instructions.md` requires bodies to be redacted BEFORE truncation into
errors or logs. Every other adapter's equivalent path already used
`crate::redaction::diagnostic_excerpt`.
**Applied fix:** Routes the excerpt through `crate::redaction::diagnostic_excerpt`
(redact, then bound) and removes the adapter's private
`RESPONSE_EXCERPT_CHAR_BUDGET` / `bounded_excerpt`, repointing its two existing
excerpt tests at the shared module. Adds
`test_anthropic_malformed_response_excerpt_never_echoes_the_configured_api_key`
(mockito, malformed 200 body echoing the configured key and a bearer token).

**Verification:**
- Red run before the fix: FAILED with `configured API key leaked into the error: ... "x-api-key":"test-api-key" ...`
- Green run after the fix: both binaries that include the file — pass
- `cargo test -p paladin-llm --all-features anthropic` — 30 passed, 0 failed
- `cargo fmt --all --check` — pass
- `cargo check --workspace --all-targets --all-features` — pass
- Commit hook (fmt check, `cargo clippy --workspace -D warnings`, hardcoded-secret scan) — pass

## Skipped Issues

None — all four in-scope findings (CR-01, CR-02, WR-01, WR-02) were fixed and
verified.

## Out of Scope / Not Acted On

### IN-01: `RedisNodeCache::scan_pattern` builds `SCAN MATCH` pattern by naive string interpolation

**File:** `crates/paladin-storage/src/node_cache/redis.rs:114-118`
**Reason out of scope:** `fix_scope` for this run is `critical_warning`; IN-01
is an Info-severity finding and was explicitly excluded per the workflow
configuration. Not attempted, not modified. The reviewer's own writeup notes
this is "unlikely to be reachable with today's identifier grammar" given
`NodeId`/`FieldName` validation elsewhere in the codebase, so it is a
reasonable candidate for a future `--fix-scope=all` pass or a standalone
follow-up if `NodeId`'s allowed character set is ever widened.

## Full workspace test suite (one-time, end of run)

Ran `cargo test --workspace --all-features` once after all four commits, per
the fixer's verification policy (not required per-commit).

Result: **1 failure, unrelated to any of the four fixes.**
`cli_isolation::test_cli_feature_is_not_default` failed with:
```
The `cli` feature is enabled during a library-only test run. Check that
`cli` is NOT listed in the `default` feature set in Cargo.toml.
```
This test's own module doc states it is meant to run as
`cargo test --test cli_isolation` or `--no-default-features` (library-only,
`cli` feature OFF) — it is a guardrail against `cli`-only dependencies
leaking into the library compilation path. Running the full suite with
`--all-features` (as this verification step and the mechanics instructions
specify) force-enables the `cli` feature, which trips this specific test by
design; it is not a `default`-feature-set regression. Confirmed unrelated to
this session's changes: none of the four fixes touch `Cargo.toml`, feature
gating, or `tests/cli_isolation_test.rs`, and re-running just that test
target with `--all-features` reproduces the identical failure in isolation
(`cargo test -p paladin-ai --test cli_isolation --all-features`), independent
of which commit is checked out.

Every other test in the full-suite run passed (hundreds of tests across
`paladin-llm`, `paladin-battalion`, `paladin-core`, `paladin-storage`, and
the root `paladin-ai` crate's unit and integration suites, including all
targeted commands listed per-finding above). All four commits' own
pre-commit hook (`cargo fmt --check` + `cargo clippy --workspace
--all-targets --all-features -- -D warnings`) passed cleanly.

## Additional Observations (not acted on)

- While applying WR-01, noticed `deserialize_null_as_empty_string` is *also*
  duplicated verbatim between `deepseek/adapter.rs` and `crate::redaction`
  (both define an identical function). This was NOT included in WR-01's
  fix — the finding's Fix section named only the credential-redaction items
  (`RESPONSE_EXCERPT_CHAR_BUDGET`, `CREDENTIAL_PLACEHOLDER`, `bounded_excerpt`,
  `redact_token_after`, `redact_credentials`) — but it is the same category of
  drift risk (a fix to one copy's null-tolerance would not propagate to the
  other) and could be worth a small follow-up finding in a future review pass.
- CR-02's fix required giving `OpenAIAdapter` a new private `map_error`
  method (it previously called `map_http_status` directly at three call
  sites with no wrapper). This is a slightly larger structural change than
  the one-line `.redirect(...)` addition the anthropic/deepseek adapters
  needed, since those two already had a `map_error` method to extend. Called
  out here since it's more code than the other two adapters' fixes, though
  the resulting shape now matches anthropic/deepseek/compat/gemini
  consistently.

---

_Fixed: 2026-09-06T12:06:11Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
