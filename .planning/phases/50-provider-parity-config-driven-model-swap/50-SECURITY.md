---
phase: 50
slug: provider-parity-config-driven-model-swap
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-10-17
---

# Phase 50 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of plans 50-01 to 50-18 (66 threats, all authored at
plan time). No SUMMARY raised a new threat flag. Verification depth: ASVS L1 (each mitigation located
in the implementation or its named test). The auditor was skipped under the short-circuit rule
(`threats_open: 0`, register authored at plan time, ASVS level 1).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| caller prompt -> vendor wire | query, context, sampling values and stop sequences go into a third-party request body | user content (confidential) |
| operator YAML / environment -> constructed adapter | keys, base URLs, limits and `models.<id>` overrides merge into one credentialed client | API keys (secret) |
| constructed adapter -> vendor host at `base_url` | a credential header goes to whatever host the block names | API keys (secret) |
| runtime API caller -> provisioned agent | an authenticated caller submits an `AgentSpec` (model, prompt, temperature) | model id (untrusted input) |
| vendor response bodies / stream events -> `LlmError`, logs, persisted responses | refusal text, error bodies and reasoning text are embedded in errors, logs, waypoints and run records | untrusted text, may echo request data |
| operator transcript -> repository / compiled registry | credentialed probe output becomes a committed file and seeded facts every deployment trusts | vendor facts (integrity-sensitive) |
| hop N failure -> hop N+1 (fallback chain) | an error on one provider decides whether another is called and billed | cost / refusal signal |
| adapter metadata -> run result / stream chunk | `paladin.*` keys are copied onto structures callers persist and log | provider names, model ids, parameter names |
| documentation / register -> operators and consumer | the Switching providers page and the 9.2 register drive downstream changes | configuration guidance |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-50-01-01 | Tampering | `user_turn_text` | low | mitigate | `crates/paladin-llm/src/turn.rs` sends text byte-for-byte; multibyte unit tests; matrix row in `tests/provider_swap_matrix.rs` | closed |
| T-50-01-02 | Information Disclosure | matrix test credentials | low | accept | AR-50-01 | closed |
| T-50-01-03 | Repudiation | red case weakened to pass | medium | mitigate | `50-MATRIX-RED-EVIDENCE.md` records each original assertion and fixing plan | closed |
| T-50-02-01 | Repudiation | decorator not forwarding `model_profile` | high | mitigate | `model_profile` forwarding and tests in `pricing.rs`, `cadence.rs`, `fallback.rs` (composition included) | closed |
| T-50-02-02 | Tampering | float edge cases past `admits` | medium | mitigate | single `admits` rule in `crates/paladin-ports/src/output/model_profile.rs`; NaN and endpoint table test | closed |
| T-50-02-03 | Repudiation | profile with forgotten field allows sending | medium | mitigate | `ModelProfile::new()` fail-closed to `NotSent` (`model_profile.rs`), tested | closed |
| T-50-02-04 | Information Disclosure | ADR text quoting vendor facts or keys | low | accept | AR-50-02 | closed |
| T-50-03-01 | Tampering | override widening a range or window | high | mitigate | `ModelOverride::validate` + `validate_limits`/`validate_support`/`validate_stop`/`validate_profile`, `#[serde(default, deny_unknown_fields)]` in `crates/paladin-llm/src/registry/mod.rs`; `an_invalid_override_fails_construction_naming_provider_and_model` | closed |
| T-50-03-02 | Repudiation | parameter dropped with only a debug log | high | mitigate | `OmissionLog` (`crates/paladin-llm/src/sampling.rs`) warns and stamps metadata; tests assert both | closed |
| T-50-03-03 | Denial of Service | warn-per-call log flooding | low | mitigate | `OmissionLog` and `note_unprofiled` (`registry/mod.rs`) warn once per instance per key | closed |
| T-50-03-04 | Information Disclosure | warn message carrying secrets | low | mitigate | warn text limited to provider, model, parameter and a number (50-03 SUMMARY) | closed |
| T-50-03-05 | Denial of Service | wrong `max_output_tokens` / `json_object` | medium | mitigate | seeds from the live transcript; refusals name provider, model, value and limit; JSON downgrade logged and stamped | closed |
| T-50-04-01 | Information Disclosure | key echoed in a vendor body reaches the transcript | high | mitigate | every body through `diagnostic_excerpt` (`redaction.rs`); `50-SEED-PROBE.md` key-shape scan returns 0 matches | closed |
| T-50-04-02 | Information Disclosure | credential header forwarded by redirect (probe) | high | mitigate | probe client uses `redirect::Policy::none()` in `examples/live_vendor_smoke.rs` | closed |
| T-50-04-03 | Information Disclosure | key in a URL query string | medium | mitigate | `redact_url` in `live_vendor_smoke.rs`; header values never printed | closed |
| T-50-04-04 | Repudiation | skipped provider read as verified | high | mitigate | `SKIPPED (<VAR> not set)` naming and zero-configured non-zero exit; transcript header lists PROBED 7 / SKIPPED 2 | closed |
| T-50-04-05 | Denial of Service (cost) | unbounded reasoning on probes | medium | mitigate | `PROBE_MAX_TOKENS = 256`, `PROBE_TIMEOUT_SECS = 120`, Moonshot `thinking: disabled` extra | closed |
| T-50-05-01 | Repudiation | value dropped silently on an original adapter | high | mitigate | `anthropic_temperature_is_omitted_and_recorded` (`anthropic/adapter.rs`) | closed |
| T-50-05-02 | Tampering | stop list silently truncated | medium | mitigate | `openai_stop_sequences_past_the_maximum_are_omitted_not_truncated` (`openai/adapter.rs`) | closed |
| T-50-05-03 | Information Disclosure | credential regression in request builders | medium | mitigate | conformance `redirect_is_not_followed_with_a_credential_header` and `credential_never_appears_in_a_rendered_error` (`conformance.rs`) | closed |
| T-50-06-01 | Tampering | window seeded too large | high | mitigate | values traced to the transcript; table test rejects zero windows and max output above window; vendor overflow maps to `TokenLimitExceeded` | closed |
| T-50-06-02 | Repudiation | seeded fact with no source | medium | mitigate | `verified_on` on every seeded entry (`registry/seeded.rs`, `model_profile.rs`) | closed |
| T-50-06-03 | Denial of Service | default model no longer resolves | medium | mitigate | defaults moved to models in the live listing; M-B-14 register row with pin-to-keep instruction | closed |
| T-50-07-01 | Tampering | window widened past the vendor limit | medium | mitigate | windows only from evidenced seeds or validated overrides (T-50-03-01, T-50-06-01) | closed |
| T-50-07-02 | Denial of Service | zero window starving the Commissary | low | mitigate | zero windows rejected at seed and override validation (`validate_limits`) | closed |
| T-50-08-01 | Information Disclosure | derived `Debug` on key-carrying configs | high | mitigate | manual `Debug` impls printing `<redacted>` on the nine config types; one test per type | closed |
| T-50-08-02 | Tampering | override widening a window or range | medium | mitigate | `ModelOverride::validate` at construction, error names provider and model | closed |
| T-50-08-03 | Denial of Service (cost) | high `max_retries` multiplies paid calls | low | accept | AR-50-03 | closed |
| T-50-08-04 | Repudiation | knob that silently does nothing | medium | mitigate | per-knob behaviour tests incl. `context_window_override_beats_a_seeded_window` on each adapter | closed |
| T-50-08-05 | Tampering | adapter-wide `context_window` beating seeded windows | medium | mitigate | construction refuses a window below a seeded max output; precedence in ADR-0061 (f), rustdoc and docs page | closed |
| T-50-09-01 | Information Disclosure | redirect from a configured `base_url` | high | mitigate | `Policy::none()` on every adapter; `factory_built_adapter_refuses_redirects` (`provider_factory.rs`) | closed |
| T-50-09-02 | Information Disclosure | literal `${VAR}` sent as key | high | mitigate | exact-placeholder resolution in `provider_factory.rs`; unset variable is a construction error naming the variable | closed |
| T-50-09-03 | Information Disclosure | key in `Debug`, logs or error strings | high | mitigate | redacting `Debug` on `LlmProviderConfig`; errors name variables only; review fix WR-04 (`fff2952f`) adds `#[serde(skip_serializing)]` on `api_key` (`config/llm.rs`) | closed |
| T-50-09-04 | Tampering | crafted model id altering a request path | medium | mitigate | `layers::validate_model_id` in `create_from_config` (257-char id refused in test); Gemini `validate_model_identifier` kept | closed |
| T-50-09-05 | Tampering | env var overriding an explicit block value | medium | mitigate | block > env per field, tested per field | closed |
| T-50-10-01 | Elevation of Privilege | runtime caller steering host or key | high | mitigate | `AgentSpec` (`crates/paladin-web/src/agent_registry.rs`) carries id, name, model, prompt, temperature, stop words only; model shape-checked | closed |
| T-50-10-02 | Information Disclosure | credential in `HostBuildError` text | high | mitigate | errors carry provider, model and variable names only; tests assert the key is absent | closed |
| T-50-10-03 | Denial of Service | shipped example YAML refusing itself | medium | mitigate | `default_temperature` removed where not admitted; `existing_yaml_loads_unchanged` (`config/llm.rs`) | closed |
| T-50-10-04 | Repudiation | registered lints guessed | medium | mitigate | D-27 run with saved output, same-commit rule (50-10 SUMMARY) | closed |
| T-50-10-SC | Tampering | `cargo install cargo-semver-checks` | high | mitigate | pinned `0.50.0` matching the CI pin; `Cargo.lock` untouched | closed |
| T-50-11-01 | Information Disclosure | reasoning text persisted | medium | accept | AR-50-04 | closed |
| T-50-11-02 | Tampering | old persisted JSON failing to load | medium | mitigate | `#[serde(default, skip_serializing_if = "Option::is_none")]` on `reasoning` (`llm_port.rs`); legacy-JSON test | closed |
| T-50-11-03 | Repudiation | register row added without measurement | low | mitigate | D-27 with `cmp` restore proof in the 50-11 SUMMARY | closed |
| T-50-12-01 | Information Disclosure | key echoed in refusal text or sliced at truncation | high | mitigate | `Refusal.detail` built only through `diagnostic_excerpt` (redact, then bound); straddle and multibyte tests | closed |
| T-50-12-02 | Repudiation | refusal hidden as empty answer | high | mitigate | one precedence in `classify_completion` (`response_guards.rs`), table-tested | closed |
| T-50-12-03 | Denial of Service | refusal retried across the chain | medium | mitigate | `Refusal` is permanent; review fix CR-01 (`5f2746fa`) adds it to the non-retryable set of the Anthropic, shared-engine, DeepSeek and Gemini retry loops; conformance `refusal_is_typed_and_permanent` uses `.expect(1)` | closed |
| T-50-12-04 | Tampering | callers relying on `Ok("")` | low | accept | AR-50-05 | closed |
| T-50-13-01 | Information Disclosure | refusal or quota body echoing a key | high | mitigate | refusal detail through `diagnostic_excerpt`; `error_override` receives the redacted excerpt; malformed-event `warn!` names the event type only | closed |
| T-50-13-02 | Repudiation | refusal reported as empty completion | high | mitigate | refusal mapped before the empty check, tested buffered and streamed | closed |
| T-50-13-03 | Denial of Service | wrong quota mapping | medium | mitigate | `error_override` only for evidenced shapes keyed on error type/code, unit-tested (Qwen, Grok, Ollama adapters) | closed |
| T-50-14-01 | Repudiation | refusal hidden by an automatic hop | high | mitigate | `Refusal` short-circuits by default; opt-in only via `hop_on_refusal(true)` (`fallback.rs`), tested both ways | closed |
| T-50-14-02 | Information Disclosure | fallback step config carrying a secret | medium | mitigate | `FallbackStepConfig` holds names only; `config_carries_no_secret_shaped_field` (`src/config/agent_runtime.rs`) covers `steps` | closed |
| T-50-14-03 | Denial of Service | pacing keyed on the wrong model | medium | mitigate | `pace_hop` receives the hop model; `cadence_is_keyed_on_each_steps_model` (`fallback.rs`) | closed |
| T-50-14-04 | Tampering | wrong model sent to a vendor (B-3) | high | mitigate | per-hop model rewrite on both paths; matrix row and wire-level unit tests | closed |
| T-50-15-01 | Repudiation | conformance case silently skipped for a vendor | medium | mitigate | `RefusalShape` forces an alternative assertion; `CASE_COUNT` self-test (`conformance.rs`) | closed |
| T-50-15-02 | Information Disclosure | refusal detail leaking the fixture key | medium | mitigate | refusal case asserts the fixture key never appears in `detail` | closed |
| T-50-16-01 | Repudiation | docs stating unverified vendor behaviour as fact | medium | mitigate | drift test against the registry (`registry/mod.rs`); unverified items labelled on the page | closed |
| T-50-16-02 | Tampering | Phase 50 break shipping unregistered | medium | mitigate | closeout D-27 across four crates, same-commit registration (50-16 SUMMARY) | closed |
| T-50-16-03 | Information Disclosure | credential-handling regression across the phase | high | mitigate | manual credential-handling review recorded in 50-16 SUMMARY; `make security` exit 0; conformance redaction and redirect cases on all nine adapters | closed |
| T-50-16-04 | Elevation of Privilege | added `unwrap`/`panic!` in library code | medium | mitigate | phase-diff review of library code before the gate commit (50-16 SUMMARY) | closed |
| T-50-17-01 | Information Disclosure | key reaching the committed transcript | high | mitigate | bodies pass `diagnostic_excerpt`; operator skim; Task 2 secret scan and literal-value comparison; re-run here: 0 key-shape matches in `50-SEED-PROBE.md` | closed |
| T-50-17-02 | Repudiation | unobserved fact presented as measured | medium | mitigate | transcript header names PROBED/SKIPPED providers; operator-supplied values carry URL and date under their own heading | closed |
| T-50-17-03 | Repudiation | SC-5 usage-cap clause read as delivered for an unseen vendor | medium | mitigate | owner reading R-2 recorded per vendor in the transcript and ADR-0061; docs page lists each vendor verified/unverified | closed |
| T-50-18-01 | Information Disclosure | secret carried onto `PaladinResult` / `StreamingResponse` | low | mitigate | only `paladin.`-prefixed keys copied; merge test asserts `x-request-id` is ignored; `ExecuteResponse` not extended | closed |
| T-50-18-02 | Repudiation | omission/fallback lost before the run result | medium | mitigate | merge and terminal-chunk tests prove every key arrives (`run_metadata`) | closed |
| T-50-18-03 | Tampering | consumer `PaladinResult` literal broken by new field | low | accept | AR-50-06 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-50-01 | T-50-01-02 | The only key in the matrix tests is the literal `matrix-test-key`, sent to a local mockito server; no env var is read and no real key exists in that environment. | plan 50-01 threat model | 2026-10-17 |
| AR-50-02 | T-50-02-04 | ADR-0061 records policy only; no credential or unverified vendor fact is written into it. | plan 50-02 threat model | 2026-10-17 |
| AR-50-03 | T-50-08-03 | A high `max_retries` is the operator's explicit choice. 429s are still never retried below the Cadence (Phase 43 D-02), permanent errors are never retried, and review fix WR-02 (`946f5f86`) bounds `max_retries` and `timeout_seconds`. | plan 50-08 threat model | 2026-10-17 |
| AR-50-04 | T-50-11-01 | Reasoning text is model output of the same trust class as `content`, which is already persisted; adapters fill it from response bodies only, so no credential flows into it. | plan 50-11 threat model | 2026-10-17 |
| AR-50-05 | T-50-12-04 | Callers relying on `Ok("")` for a refusal were relying on the defect the handoff reported (G-2); the change is recorded as an M-B register row. | plan 50-12 threat model | 2026-10-17 |
| AR-50-06 | T-50-18-03 | Registered break: the 9.2 row was extended from a measured D-27 run, muster updated in the same commit, and the change is carried in the 9.8 consumer note before the single v0.11.0 move. | plan 50-18 threat model | 2026-10-17 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-17 | 66 | 66 | 0 | /gsd-secure-phase orchestrator (L1 grep-depth; auditor skipped by short-circuit rule) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-17
