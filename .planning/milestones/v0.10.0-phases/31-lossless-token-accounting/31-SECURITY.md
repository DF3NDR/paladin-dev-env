---
phase: 31
slug: lossless-token-accounting
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-15
---

# Phase 31 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — all seven `31-0N-PLAN.md` files carry a `<threat_model>` block (26 distinct threat ids `T-31-01`..`T-31-26` plus the per-plan `T-31-SC` package-install item, recorded once below with the union of its evidence; plan 31-07 re-lists `T-31-08`/`T-31-13` as the closing credential-handling review rather than as new threats). `31-03-SUMMARY.md` and `31-04-SUMMARY.md` carry `## Threat Flags` sections, both reporting nothing beyond their plan's pre-declared register and restating the per-threat evidence used below. Verification depth: ASVS L1 grep-depth, per the short-circuit rule (`threats_open: 0`, `register_authored_at_plan_time: true`, `asvs_level: 1`); `make security` (T-31-26) was additionally re-run live on 2026-09-15 rather than only read from the plan 31-07 gate evidence, and the phase-wide `Cargo.toml` diff guard (T-31-SC) was executed against the pre-phase base `e6c76bb3^`.

One evidence gap is recorded honestly rather than papered over: the accepted risk T-31-07 cites MIGRATION.md §9.4's statement that `TraceEvent::NodeFinished`/`RunFinished` carry `#[serde(default)]` on `usage`, but `crates/paladin-core/src/platform/container/trace.rs` lines 237 and 292 carry no such attribute, and the D-25 legacy-JSON tests cover `PaladinResult` and `NodeExecutionRecord` only. A pre-phase persisted `TraceRecord` row of those two kinds therefore fails to deserialise with a typed `RunTraceError::Serialization` (`crates/paladin-storage/src/run_trace/sqlite.rs:137`) instead of reading as zero usage. This changes the shape of the accepted risk (unreadable history rather than zero-valued history), not its severity or its safety property (no panic, resume unaffected) — see the Accepted Risks Log.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Provider HTTP response body → adapter serde structs | Untrusted numeric usage input crosses here; plans 31-03/31-04 add new parsing of streamed usage frames on this boundary for five adapters | Prompt/completion/cache/reasoning token counts |
| Provider SSE event stream → adapter usage accumulation | Anthropic `message_start`/`message_delta` events and Gemini cumulative `usageMetadata` frames accumulate into a fixed-size `TokenUsage` | Per-event numeric deltas; event count is attacker-influenced |
| Adapter error / log strings → operator logs | A raw response body must not cross here unredacted; every deserialisation-failure branch must route through `diagnostic_excerpt` (redact before truncate) | Response excerpts, never credentials |
| Persisted Waypoint / `run_traces` JSON → `TokenUsage` / `NodeExecutionRecord` / `TraceRecord` deserialisation | Untrusted-at-rest JSON; the retired `token_count` key is dropped and the new `usage` field defaults (or, for `TraceRecord`, fails typed) | Stored JSON payload columns |
| Execution-service stream → SSE / `paladin-web` streaming consumer | `ChunkMetadata.usage` and the `node_finished` / `run_finished` payloads change shape on the wire | Numeric usage object |
| HTTP client → `paladin-web` response serialization | `ExecuteResponse` gains a `usage: TokenUsageResponse` object; the published response contract changes shape | Numeric usage object for the caller's own request |
| Herald / CLI rendered output → operator terminal, log file or downstream JSON consumer | Numeric usage figures cross; no credential or response body may accompany them | Integer token counts, per-Paladin tables |
| Generated `openapi.json` → CI Python-client generation job | A stale or malformed schema breaks a downstream build | OpenAPI document |
| Documented compatibility register (`MIGRATION.md` §9.2) → downstream consumer upgrading across the break | A missing or wrong row silently misleads an upgrade | Semver lint ids, migration rows |
| Dependency advisory feed → `make security` | New advisories against already-vendored crates surface here | RustSec advisories |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-31-01 | Tampering | `TokenUsage` arithmetic (`Add`/`AddAssign`/`Sum`, `new`) | medium | mitigate | `crates/paladin-core/src/platform/container/token_usage.rs`: 7 `saturating_add` sites, rustdoc line 20 states the saturate-at-`u32::MAX` contract, and the boundary tests at lines 213–218 and 259 assert `u32::MAX` clamping on both the required and optional fields | closed |
| T-31-02 | Denial of Service | `TokenUsage` deserialisation of persisted/provider JSON | low | mitigate | The three new fields (`cache_read_tokens`, `cache_write_tokens`, `reasoning_tokens`) are `Option<u32>` behind `#[serde(default)]` (`token_usage.rs` lines 32–44); non-numeric / negative / out-of-range input is rejected by serde as `Err` | closed |
| T-31-03 | Information Disclosure | New rustdoc / doc-test examples in `token_usage.rs` | low | mitigate | Credential-shape grep (`(sk-\|api[_-]?key\|Bearer )[A-Za-z0-9_-]{8,}`) over `token_usage.rs`: 0 hits | closed |
| T-31-04 | Tampering | `TraceDispatcher` run-total accumulator (`crates/paladin-battalion/src/engine/hooks.rs`) | medium | mitigate | `hooks.rs:310–315` accumulates via `*total += usage` (saturating `AddAssign`) under `Mutex<TokenUsage>`; both lock sites (lines 314, 395) recover with `PoisonError::into_inner` so a poisoned sink thread cannot turn the next `emit` into a panic | closed |
| T-31-05 | Denial of Service | `NodeExecutionRecord` / `TraceEvent` deserialisation of a stored blob | low | mitigate | `waypoint.rs:577–578` carries `#[serde(default)]` on `usage`; `TraceRecord` rows are read through `serde_json::from_str(..).map_err(RunTraceError::Serialization)` (`run_trace/sqlite.rs:137`), so a malformed blob yields a typed `Err`, never a panic; no non-test `unwrap`/`expect`/`panic!` in the new adapter or core code (checked in `anthropic/adapter.rs` and `gemini/adapter.rs` above their `#[cfg(test)]` modules: 0 hits) | closed |
| T-31-06 | Information Disclosure | SSE `node_finished` / `run_finished` payload gaining the usage object | low | accept | Payload gains only integer token counts already visible to the same authenticated consumer through the herald and `GET` inspector routes; no new principal, no request/response content — see Accepted Risks Log AR-31-01 | closed |
| T-31-07 | Repudiation | Pre-phase Waypoint / trace rows after the `token_count` → `usage` break | low | accept | MIGRATION.md §9.4 documents the break; `waypoint.rs#legacy_json_deserialises_with_default_usage` and `execution_result.rs#legacy_json_deserialises_with_served_by_none` prove the zero-usage contract for `NodeExecutionRecord` / `PaladinResult`. **Evidence gap:** `TraceEvent` variants carry no `#[serde(default)]` (`trace.rs:237`, `:292`), so pre-phase `TraceRecord` rows fail typed rather than reading as zero — see AR-31-02 | closed |
| T-31-08 | Information Disclosure | New usage-frame parsing in `compat/engine.rs`, `openai/adapter.rs`, `deepseek/adapter.rs` | high | mitigate | `diagnostic_excerpt` call count in `compat/engine.rs` is 6 (unchanged, per 31-03 SUMMARY Threat Flags); new usage structs extract typed numeric fields only (`Option<u32>` + `#[serde(default)]`: openai 7/3, deepseek 6/3); plan 31-07 Task 3's manual credential-handling review over the whole-phase `crates/paladin-llm` diff (18 files) confirmed no key in a log line, redact-before-truncate intact, no redirect-following added; 19 `Policy::none()` redirect sites remain across the adapters | closed |
| T-31-09 | Denial of Service | Malformed or adversarial usage frame on the OpenAI-compatible streams | medium | mitigate | Every new field is `Option<u32>` behind `#[serde(default)]`; frames parse via `serde_json::from_str` returning `Result`; an absent usage frame degrades to `usage: None` and the D-17 no-usage path records `TokenUsage::default()` (`paladin_execution_service.rs:3246–3252`) | closed |
| T-31-10 | Information Disclosure | D-17 `warn!` emitted when a streamed call reports no usage | low | accept | `paladin_execution_service.rs:3248–3252` interpolates only `{provider_name}` (the provider's static identifier); no request/response content and no credential — AR-31-03 | closed |
| T-31-11 | Spoofing | An estimate presented as a provider-billed figure | medium | mitigate | `paladin_execution_service.rs:6211` `streamed_final_chunk_with_no_reported_usage_never_consults_the_token_counter` asserts `TokenCounterPort` is never called on the no-usage streaming path | closed |
| T-31-12 | Information Disclosure | Credential-shaped literal in a newly captured OpenAI/DeepSeek fixture | low | mitigate | Key-shape grep over `openai/adapter.rs` and `deepseek/adapter.rs`: matches only in pre-existing redaction-test fixtures (`deepseek/adapter.rs:1142`, `:1176`, `:1187`, `:1195`), none introduced by this phase (confirmed by 31-03 SUMMARY Threat Flags) | closed |
| T-31-13 | Information Disclosure | New usage parsing in `anthropic/adapter.rs` and `gemini/adapter.rs` | high | mitigate | `diagnostic_excerpt` call count in `anthropic/adapter.rs` is 2 (unchanged, per 31-04 SUMMARY Threat Flags); `gemini/adapter.rs` 3; new structs typed `Option<u32>` + `#[serde(default)]` (anthropic 13/3, gemini 9/8); covered by the same plan 31-07 Task 3 credential-handling review and the passing `test_anthropic_client_refuses_to_follow_a_redirect`, `test_anthropic_malformed_response_excerpt_never_echoes_the_configured_api_key`, `gemini_does_not_replay_the_api_key_header_to_a_redirect_target` tests | closed |
| T-31-14 | Denial of Service | Adversarial Anthropic event stream (unbounded `message_delta` events) | medium | mitigate | Accumulator is a fixed-size `TokenUsage` updated in place with saturating adds (`anthropic/adapter.rs`: 3 `saturating` sites; `gemini/adapter.rs`: 2); no per-event allocation grows with event count; 0 non-test `unwrap`/`expect`/`panic!` in either adapter | closed |
| T-31-15 | Tampering | Provider over-reporting a cached-token figure larger than its own input count | low | accept | Inequality contract (`cache_read_tokens + cache_write_tokens <= prompt_tokens`, `reasoning_tokens <= completion_tokens`) documented on `TokenUsage` (`token_usage.rs` lines 16–17, 31, 36, 41) and asserted on the phase's own fixtures; deliberately not enforced at runtime — AR-31-04 | closed |
| T-31-16 | Information Disclosure | Credential-shaped literal in the new Anthropic fixture | low | mitigate | `sk-ant-` grep over `anthropic/adapter.rs`: only the pre-existing `sk-ant-test123` test-config literal (lines 940–1575), none in the new `CACHED_PROMPT_SONNET_5_JSON` fixture (per 31-04 SUMMARY Threat Flags) | closed |
| T-31-17 | Information Disclosure | Herald and CLI output gaining cache and reasoning figures | low | accept | Integer token counts already available to the same principal through the existing total; no prompt text, response body or credential rendered — AR-31-05 | closed |
| T-31-18 | Denial of Service | Markdown per-Paladin usage table over a large battalion | low | accept | One row per entry of a map the herald already iterated; only the column count grows — AR-31-06 | closed |
| T-31-19 | Tampering | A bare total drifting from the usage object it sits beside in JSON output | low | mitigate | `crates/paladin-herald/src/json_herald.rs:535` and `:740` assert `parsed["total_tokens"] == parsed["usage"]["total_tokens"]` for both `paladin_result_to_json` and `finalize_stream` (D-08 coexistence), plus `agent_controller.rs:1846` for `ExecuteResponse.total_tokens` | closed |
| T-31-20 | Information Disclosure | `ExecuteResponse`, `CompletedRow` and SSE payloads gaining cache and reasoning figures | low | accept | Integer counts for a call the requesting principal already made and already receives a total for; every affected route keeps its existing authentication middleware — AR-31-07 | closed |
| T-31-21 | Tampering | Committed `openapi.json` drifting from the served specification | medium | mitigate | `crates/paladin-web/src/openapi.rs:264` `openapi_matches_committed_baseline` test; plan 31-07 gate `make openapi && git diff --exit-code crates/paladin-web/openapi.json` clean; `git status` on `openapi.json` clean at audit time | closed |
| T-31-22 | Elevation of Privilege | A new DTO bypassing an existing route guard | low | accept | No route, handler signature or middleware layer added or reordered; `require_authentication` layering in `agent_controller.rs` untouched (1 site, unchanged) — AR-31-08 | closed |
| T-31-23 | Tampering | An inward-flowing dependency introduced to share the schema-annotated DTO | medium | mitigate | `TokenUsageResponse` lives in `paladin-web` only; grep for `utoipa`/`paladin-web` in `crates/paladin-core/Cargo.toml` and `crates/paladin-ports/Cargo.toml`: 0 hits | closed |
| T-31-24 | Repudiation | `MIGRATION.md` §9.2 register vs `.cargo/semver-checks-allowlist.toml` | high | mitigate | Plan 31-07 Task 1 ran the same awk-based row-level set-equality comparison `ci.yml` performs against the finished register: empty diff in both directions; seven new `[[entry]]` blocks match the seven new §9.2 rows | closed |
| T-31-25 | Tampering | Guessed rather than observed semver lint ids | medium | mitigate | Every lint id derived empirically via `cargo semver-checks check-release --release-type minor` (forcing evaluation past the version-already-bumped skip) and, for the crate-level-allowed `struct_marked_non_exhaustive`, a temporary `allow` → `warn` flip reverted after observing the fire; all three literal CI-matching `check-release` commands exit 0 (31-07 SUMMARY, D-27) | closed |
| T-31-26 | Tampering | A new advisory against an already-vendored dependency | medium | mitigate | `make security` re-run live 2026-09-15 during this audit: exit 0, `advisories ok, bans ok, licenses ok, sources ok`, 0 new un-allowlisted advisories (matches plan 31-07 gate evidence: 10 pre-allowlisted warnings, 0 new) | closed |
| T-31-SC | Tampering | Package-manager installs | low | accept | `git diff e6c76bb3^..HEAD -- Cargo.toml crates/*/Cargo.toml` shows 38 added lines across `paladin-core`, `paladin-ports`, `paladin-web`, every one a `[package.metadata.cargo-semver-checks.lints]` allow entry or comment — 0 dependency-line changes; the one tool install (`cargo-semver-checks` 0.50.0, `--locked`, matching `ci.yml`'s pin) never enters the dependency graph — AR-31-09 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-31-01 | T-31-06 | SSE payloads gain only integer token counts already exposed to the same authenticated consumer via herald and inspector routes; no new principal and no content | Plan 31-02 threat model (executor) | 2026-09-15 |
| AR-31-02 | T-31-07 | Token history is not resume-critical: `resume` rebuilds from the Battlefield snapshot and graph fingerprint, never from accumulated counts. `NodeExecutionRecord`/`PaladinResult` pre-phase documents read as `TokenUsage::default()` (tested). **Corrected shape of the risk:** pre-phase `TraceRecord` rows of kind `node_finished`/`run_finished` do *not* read as zero — `TraceEvent` carries no `#[serde(default)]` on `usage`, so `RunTracePort::read` returns `RunTraceError::Serialization` for those rows. Safety property (typed error, no panic, resume unaffected) holds; MIGRATION.md §9.4's `#[serde(default)]` claim for `TraceEvent` is inaccurate and should be fixed either by adding the attribute to `trace.rs:237`/`:292` (preferred, restores the documented behaviour) or by correcting §9.4 | Plan 31-02 threat model (executor); shape corrected by this audit | 2026-09-15 |
| AR-31-03 | T-31-10 | The D-17 `warn!` names only the provider's static identifier, matching the existing `FallbackHop` trace-event pattern | Plan 31-03 threat model (executor) | 2026-09-15 |
| AR-31-04 | T-31-15 | Enforcing the cache/reasoning inequality at runtime would reject a provider's own billed figure; the Treasurer prices what the provider billed. Contract documented on `TokenUsage` and asserted on fixtures | Plan 31-04 threat model (executor) | 2026-09-15 |
| AR-31-05 | T-31-17 | Herald/CLI cache and reasoning figures are integer counts already available to the same principal | Plan 31-05 threat model (executor) | 2026-09-15 |
| AR-31-06 | T-31-18 | Markdown per-Paladin table row count equals the already-iterated map; only columns grow | Plan 31-05 threat model (executor) | 2026-09-15 |
| AR-31-07 | T-31-20 | `ExecuteResponse`/`CompletedRow`/SSE gain integer counts for the requester's own call; authentication middleware unchanged | Plan 31-06 threat model (executor) | 2026-09-15 |
| AR-31-08 | T-31-22 | Only a response-body field's type changes; no route, handler or middleware added or reordered | Plan 31-06 threat model (executor) | 2026-09-15 |
| AR-31-09 | T-31-SC | No `Cargo.toml` dependency lines changed across the phase (diff-verified); `cargo-semver-checks` is a pinned, `--locked` developer tool outside the dependency graph; `make security` green | Plans 31-01..31-07 threat models (executor) | 2026-09-15 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-15 | 27 | 27 | 0 | /gsd-secure-phase 31 (orchestrator, L1 grep-depth short-circuit; `make security` re-run live) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-15
