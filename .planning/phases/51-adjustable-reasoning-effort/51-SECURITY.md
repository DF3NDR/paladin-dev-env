---
phase: 51
slug: adjustable-reasoning-effort
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-10-23
---

# Phase 51 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of plans 51-01 to 51-20 (60 threats with distinct
ids, T-51-01 to T-51-60, plus the shared supply-chain row T-51-SC, all authored at plan time).
No SUMMARY raised a new threat flag (every `## Threat Flags` section reads "None" and names the
plan's own rows as mitigated or accepted). Verification depth: ASVS L1. Each mitigation was located
by grep in the implementation or its named test: the fifteen tests the register names
(`the_builder_verdict_equals_check_reasoning_for_every_combination`,
`pricing_over_cadence_forwards_check_reasoning`,
`a_chain_with_a_profileless_hop_refuses_an_explicit_effort_at_build`,
`every_seeded_effort_level_round_trips_to_its_wire_body`,
`agent_run_node_finished_reads_only_the_two_reasoning_keys`,
`agent_run_node_finished_never_reads_the_agents_configured_effort`,
`failed_agent_run_node_finished_carries_no_reasoning`,
`the_host_verdict_survives_the_pricing_decorator`,
`an_agents_refuse_beats_a_provider_nearest_lower_at_build`,
`a_hops_provider_default_effort_is_pre_flighted_even_when_the_request_sets_none`,
`ollama_check_reasoning_refuses_on_the_undeclared_wire`,
`openai_compatible_check_reasoning_refuses_on_the_undeclared_wire`,
`block_nearest_lower_agent_refuse_is_refused`,
`block_nearest_lower_agent_nearest_higher_takes_the_agents_direction`, `reasoning_page_tests`)
each exist exactly once under `src/`, `crates/` or `tests/`; `RedactedKey` is used by the eleven
`paladin-llm` config/adapter files; `ModelOverride` carries `deny_unknown_fields`; every
`paladin-llm` HTTP client keeps `redirect::Policy::none()` (19 sites); `LlmError::UnsupportedParameter`
maps to `Transience::Permanent`; the committed transcripts `51-EFFORT-PROBE.md` and
`51-EFFORT-SMOKE.md` contain no vendor key shape; CI pins `cargo-semver-checks@0.50.0`. The auditor
was skipped under the short-circuit rule (`threats_open: 0`, register authored at plan time, ASVS
level 1), as for Phase 50.1. The manual credential-handling review required by
`.github/instructions/security.instructions.md` is recorded per plan in the SUMMARYs of 51-17,
51-18 and 51-19 (refusal text built only from provider, model, effort and key names; no key, header,
base URL or response body in scope of `reasoning::check`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Operator config → adapter construction | `llm.<provider>` YAML (`reasoning_effort`, `reasoning_fallback`, `reasoning_wire`, `models.<id>.effort.*`) and agent YAML (`reasoning_effort`, `reasoning_fallback`) resolved into `ModelProfile` / `ReasoningWire` at build time | effort spellings, budget bounds, wire names; API keys (redacted `Debug`) |
| Adapter → vendor HTTP | nine native adapters and the compat engine serialise one declared wire form per model or refuse before the call | request bodies carrying `effort` / `thinking` / `budget` fields; bearer or `x-api-key` header; no redirects followed |
| Response → trace / metadata | `paladin.reasoning_requested` / `paladin.reasoning_effective` on `LlmResponse.metadata` and `TraceEvent::NodeFinished` | adapter-written effort spellings only, read from the attempt's own metadata |
| Live probe harness → committed evidence | `live_vendor_smoke --effort` / `--effort-smoke` output into `51-EFFORT-PROBE.md` / `51-EFFORT-SMOKE.md` | numbers, verdicts and bounded diagnostic excerpts; owner checkpoint before spend; key-shape scan before commit |
| Developer tooling → CI | `cargo-semver-checks` pinned at 0.50.0 | lint ids measured against the published 0.10.1 |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-51-01 | Tampering / Repudiation | `EffortSupport::resolve` silently substituting a different effort | high | mitigate | exact-or-refuse by default; substitution only under an explicit `NearestLower` / `NearestHigher`; `ResolvedEffort` keeps both values; Task 2 table tests (prohibition REASON-02 transparency) (51-01; 51-01 Threat Flags) | closed |
| T-51-02 | Denial of Service | an over-large `Budget(n)` or a coerced `Max` amplifying cost | medium | mitigate | `BudgetBounds` inclusive check, never converted to a level; coerced levels refuse whatever the fallback | closed |
| T-51-03 | Information Disclosure | `UnsupportedParameter` rendering request content | medium | mitigate | the constructor takes only parameter, value, provider, model and reason; a test asserts a planted prompt string never appears in `to_string()` (prohibition REASON-02 privacy) | closed |
| T-51-04 | Denial of Service | a deterministic refusal retried by loops or fallback chains | medium | mitigate | `transience()` arm `Permanent`; battalion `typed_origin` arm; Task 1 tests | closed |
| T-51-05 | Tampering | golden files blessed from a modified adapter, hiding a body change | medium | mitigate | Task 3 precondition (no adapter file modified) and acceptance check on the commit's diff (51-01; 51-01 Threat Flags) | closed |
| T-51-06 | Information Disclosure | a key or bearer token in a printed row | high | mitigate | only numbers, verdicts and `diagnostic_excerpt` output are printed; Task 2 key-shape test; no request header is ever printed (prohibition REASON-03 privacy) (51-02; 51-02 Threat Flags) | closed |
| T-51-07 | Denial of Service | unbounded spend or runaway `max` reasoning | medium | mitigate | 2,048-token cap and 120 s timeout per call; only `--models` ids are probed; dry-run and a spend estimate precede any run (prohibition REASON-03 safety) (51-02; 51-02 Threat Flags) | closed |
| T-51-08 | Spoofing | an operator base URL for a self-hosted target | low | accept | accepted — operator-trusted by design (T-17-18); the harness's clients inherit the adapters' no-redirect posture | closed |
| T-51-09 | Tampering / Repudiation | an effort silently dropped or swapped on the wire | high | mitigate | `admit` before HTTP, never the omit-and-log path; substitution only on explicit fallback with `warn!` and both metadata keys; byte-diff tests (prohibitions REASON-02, REASON-05) (51-03; 51-03 Threat Flags) | closed |
| T-51-10 | Denial of Service | a refusal retried, or a budget that leaves no output room | medium | mitigate | `Permanent` refusal raised in `build_request` before the retry loop; zero-hit tests; budget below `max_tokens` rule (51-03; 51-03 Threat Flags) | closed |
| T-51-11 | Tampering | a malformed operator override (unsorted, overlapping, `min = 0`) | medium | mitigate | `ModelOverride::validate` with `deny_unknown_fields`, failing adapter construction with a message naming the field (51-03; 51-03 Threat Flags) | closed |
| T-51-12 | Information Disclosure | request content or a key in the refusal text or the `warn!` | medium | mitigate | both name only provider, model and effort values; a test plants a prompt string and asserts it is absent from the error and the log line (51-03; 51-03 Threat Flags) | closed |
| T-51-13 | Tampering | a preset sending a field its vendor ignores (silent no-op) | high | mitigate | one declared wire per preset, compile-enforced; exact per-wire mapping; cross-field absence test; Grok `Unsupported` (prohibition REASON-03) (51-04; 51-04 Threat Flags) | closed |
| T-51-14 | Tampering | an operator-chosen wire forwarding a free-form field name | medium | mitigate | `ReasoningWire` is a closed enum; no free-form field name reaches the body (51-04; 51-04 Threat Flags) | closed |
| T-51-15 | Denial of Service | a Qwen budget beyond the vendor's range | low | mitigate | the profile's `BudgetBounds` (from the probe or the override) checked in `resolve`; budget below `max_tokens` in `admit` (51-04; 51-04 Threat Flags) | closed |
| T-51-16 | Spoofing | a self-hosted base URL redirecting the credential | low | accept | accepted — unchanged: every compat client keeps `redirect::Policy::none()` (T-17-18) | closed |
| T-51-17 | Tampering | the deprecated Anthropic budget form sent to a model that rejects it, or a level silently ignored | high | mitigate | form chosen from the masked per-model profile; budget form only with profile bounds; zero-hit refusal tests (prohibition REASON-02 safety) (51-05; 51-05 Threat Flags) | closed |
| T-51-18 | Denial of Service | a thinking budget that consumes the whole output budget | medium | mitigate | budget inside profile bounds and below the effective output cap, refused before HTTP; `EmptyCompletion` stays the runtime backstop (prohibition REASON-06) (51-05; 51-05 Threat Flags) | closed |
| T-51-19 | Information Disclosure | reasoning text requested or surfaced beyond what the caller asked for | low | mitigate | no `display` / `includeThoughts`; text only from blocks the vendor returns, into the Phase 50 typed fields (prohibition REASON-05) (51-05; 51-05 Threat Flags) | closed |
| T-51-20 | Tampering | a named vendor's wire overridden from config into an unverified field | high | mitigate | `resolve_layers` refuses `reasoning_wire` outside `ollama` / `openai-compatible` (prohibition REASON-02 safety) (51-06; 51-06 Threat Flags) | closed |
| T-51-21 | Denial of Service | a provider-wide default refusing every request for a model that cannot honour it | medium | accept | accepted — consistent with D-00f; the refusal names `llm.<provider>.reasoning_effort`; docs (plan 51-13) say to set it only where every model supports it | closed |
| T-51-22 | Information Disclosure | a config `Debug` printing the key after new fields are added | medium | mitigate | every hand-written `Debug` keeps `RedactedKey`; the nine-type test asserts no key in `Debug` output (51-06; 51-06 Threat Flags) | closed |
| T-51-23 | Repudiation / Tampering | a chain silently losing or changing effort on failover | high | mitigate | D-08 pre-flight on every hop before any call; per-hop resolution recorded on the served response (prohibition REASON-04) (51-07; 51-07 Threat Flags) | closed |
| T-51-24 | Tampering | old Citadel snapshots failing to load or changing bytes | medium | mitigate | `#[serde(default, skip_serializing_if = "Option::is_none")]`; Task 1 round-trip tests (51-07; 51-07 Threat Flags) | closed |
| T-51-25 | Denial of Service | an agent misconfiguration surfacing only at run time | low | mitigate | D-09 build-time refusal naming the field and reason (51-07; 51-07 Threat Flags) | closed |
| T-51-26 | Tampering | a mistyped YAML spelling silently treated as unset | medium | mitigate | closed serde enums; an unknown spelling fails load naming the field (Task 1 test) (51-08; 51-08 Threat Flags) | closed |
| T-51-27 | Denial of Service | a provider default that refuses every call of an agent, found only at run time | low | mitigate | build-time check of the block value against the agent's model, naming `llm.<provider>.reasoning_effort` (51-08; 51-08 Threat Flags) | closed |
| T-51-28 | Tampering (integrity of results) | a cached answer served across efforts | medium | mitigate | effort and fallback in the fingerprint when set; distinct tag bytes; pinned unset key (prohibition REASON-01 values) (51-09; 51-09 Threat Flags) | closed |
| T-51-29 | Repudiation | a trace showing an effort that did not run | medium | mitigate | fields read only from the attempt's own `run_metadata`, never from configuration (prohibition REASON-05) (51-09; 51-09 Threat Flags) | closed |
| T-51-30 | Information Disclosure | metadata strings carrying unexpected content into OTel exports | low | mitigate | only the two `paladin.reasoning_*` keys are read; their values are adapter-written `Display` spellings (51-09; 51-09 Threat Flags) | closed |
| T-51-31 | Information Disclosure | a key echoed in a body, or pasted by hand, reaching the committed transcript | high | mitigate | harness prints numbers and `diagnostic_excerpt` output only; scratch scan with key shapes and literal credential values; negative key-shape grep; a hit stops the commit (prohibition REASON-03 privacy) (51-10; 51-10 Threat Flags) | closed |
| T-51-32 | Repudiation | an unobserved fact presented as measured | medium | mitigate | the verdict table is computed by stated rules from quoted rows; docs values sit under their own heading with URL and date (prohibition REASON-04) (51-10; 51-10 Threat Flags) | closed |
| T-51-33 | Denial of Service / spend | an unapproved or runaway run | medium | mitigate | blocking owner checkpoint before any call; 2,048-token cap and 120 s timeout; per-vendor runs; only confirmed ids (prohibition REASON-03 safety) (51-10; 51-10 Threat Flags) | closed |
| T-51-34 | Tampering | an unconfirmed wire or desk-research level seeded (silent no-op in production) | high | mitigate | owner checkpoint on the verdict table; seed rules in the truths; per-row source comments; seed-matches-transcript tests (prohibition REASON-03) (51-11; 51-11 Threat Flags) | closed |
| T-51-35 | Repudiation | the Kimi regression pinned in a direction the probe contradicts | medium | mitigate | the test is written after the decision, from quoted transcript rows (51-11; 51-11 Threat Flags) | closed |
| T-51-36 | Tampering | an unconfirmed native form or desk-research level shipped (silent no-op) | high | mitigate | caps per the recorded decision; seed rules; per-row source comments; seed-matches-transcript tests; the round-trip invariant (prohibition REASON-03) (51-12; 51-12 Threat Flags) | closed |
| T-51-37 | Repudiation | a seeded level the adapter does not actually send | medium | mitigate | `every_seeded_effort_level_round_trips_to_its_wire_body` over every seeded row and level (51-12; 51-12 Threat Flags) | closed |
| T-51-38 | Tampering / Repudiation | a documented mapping drifting from the code (a refused row shown as working) | medium | mitigate | `reasoning_page_tests` derives the expected table from the built adapters (prohibition REASON-07) (51-13; 51-13 Threat Flags) | closed |
| T-51-39 | Denial of Service | a documented Kimi example that exhausts its budget or times out | medium | mitigate | example values only from measurements; plan 51-14's live smoke re-checks it (prohibition REASON-06) (51-13; 51-13 Threat Flags) | closed |
| T-51-40 | Information Disclosure | a key reaching `51-EFFORT-SMOKE.md` | high | mitigate | harness prints numbers and metadata values only; scratch scan with key shapes and literal credential values; negative grep; a hit stops the commit (prohibition REASON-03 privacy) (51-14; 51-14 Threat Flags) | closed |
| T-51-41 | Denial of Service / spend | an unapproved live run | medium | mitigate | blocking-human checkpoint before any call; about seven calls, shown by dry-run first (prohibition REASON-03 safety) (51-14; 51-14 Threat Flags) | closed |
| T-51-42 | Repudiation | a register entry claiming a lint that was never measured | medium | mitigate | lint ids copied only from recorded output; `cmp` proof of each restored lint table (prohibition REASON-07) (51-14; 51-14 Threat Flags) | closed |
| T-51-43 | Repudiation | `run_agent` `NodeFinished` (`worker.rs` Ok arm) | medium | mitigate | fields read only from the call's own `result.run_metadata` under the two core constants; `agent_run_node_finished_never_reads_the_agents_configured_effort` pins that configuration never reaches the trace (51-15; 51-15 Threat Flags) | closed |
| T-51-44 | Information Disclosure | other `run_metadata` keys or failure text reaching the trace | low | mitigate | only the two named keys are read (`agent_run_node_finished_reads_only_the_two_reasoning_keys`); the failure arm is unchanged and `failed_agent_run_node_finished_carries_no_reasoning` asserts `AGENT_FAILURE_TEXT` reaches no trace row (51-15; 51-15 Threat Flags) | closed |
| T-51-45 | Denial of Service | `check_block_reasoning` accepting a configuration every call then refuses | medium | mitigate | agent-first precedence matching `in_force`; `block_nearest_lower_agent_refuse_is_refused` pins the refusal at load (51-16; tests `block_nearest_lower_agent_refuse_is_refused`, `block_nearest_lower_agent_nearest_higher_takes_the_agents_direction`, host `SECRET`-absence assertions (`agent_host.rs`)) | closed |
| T-51-46 | Tampering (integrity of operator intent) | a provider-wide fallback replacing an agent's explicit fallback at the gate | medium | mitigate | `def.reasoning_fallback.or(block.fallback)`; `block_nearest_lower_agent_nearest_higher_takes_the_agents_direction` (51-16; tests `block_nearest_lower_agent_refuse_is_refused`, `block_nearest_lower_agent_nearest_higher_takes_the_agents_direction`, host `SECRET`-absence assertions (`agent_host.rs`)) | closed |
| T-51-47 | Information Disclosure | refusal text | low | mitigate | text built from agent id, model, provider name, effort, fixed refusal wording and the YAML key name only; the Task 1 test asserts the block's `SECRET` is absent (51-16; tests `block_nearest_lower_agent_refuse_is_refused`, `block_nearest_lower_agent_nearest_higher_takes_the_agents_direction`, host `SECRET`-absence assertions (`agent_host.rs`)) | closed |
| T-51-48 | Denial of Service | `PaladinBuilder::build` accepting an agent every call refuses (or refusing one every call would serve) | medium | mitigate | builder delegates to `check_reasoning`, whose adapter overrides run `in_force` + `judge` over `wire_profile`; `the_builder_verdict_equals_check_reasoning_for_every_combination` and the DeepSeek full-grid parity test (51-17; 51-17 Threat Flags) | closed |
| T-51-49 | Tampering (operator intent) | a provider default overriding an explicit agent value at the gate | medium | mitigate | `in_force` is field-by-field request-first; `an_agents_refuse_beats_a_provider_nearest_lower_at_build` (51-17; 51-17 Threat Flags) | closed |
| T-51-50 | Information Disclosure | refusal text | low | mitigate | reasons are built only from provider name, model, effort spellings, fixed refusal wording and YAML key names (`refusal_reason`, `REASONING_FALLBACK_HINT`); no key, header or response body is in scope of `check`; the SUMMARY records the manual credential-handling review (51-17; 51-17 Threat Flags) | closed |
| T-51-51 | Repudiation | a pre-judge emitting substitution warnings for requests never sent | low | mitigate | `check` / `judge` take no `SubstitutionLog`; only `admit` records (51-17; 51-17 Threat Flags) | closed |
| T-51-52 | Tampering (gate integrity) | a decorator inheriting the trait default and dropping provider defaults | medium | mitigate | explicit forwarding in `pricing.rs` / `cadence.rs`; `pricing_over_cadence_forwards_check_reasoning` pins the production composition (51-18; 51-18 Threat Flags) | closed |
| T-51-53 | Information Disclosure | refusal text from the compat engine | low | mitigate | `reasoning::check` renders provider name, model, effort and key names only; the self-hosted base URL and API key are never in scope; manual credential-handling review in the SUMMARY (51-18; 51-18 Threat Flags) | closed |
| T-51-54 | Denial of Service | a self-hosted preset pre-judge admitting an effort its `Unsupported` wire refuses | medium | mitigate | the engine's `model_profile` is masked by the declared wire, so `check` refuses as send time does; covered by the Ollama / OpenAI-compatible tests (51-18; 51-18 Threat Flags) | closed |
| T-51-55 | Denial of Service | a chain that hides an incompatible hop until failover (refuses during an outage of hop 0) | medium | mitigate | per-hop `check_reasoning` with hop defaults before the first call; `a_hops_provider_default_effort_is_pre_flighted_even_when_the_request_sets_none` (51-19; 51-19 Threat Flags) | closed |
| T-51-56 | Tampering (gate integrity) | a profile-less hop silently dropped from the chain's effort answer | medium | mitigate | per-hop verdict refuses it; `model_profile` effort `Unmeasured`; `a_chain_with_a_profileless_hop_refuses_an_explicit_effort_at_build` (51-19; 51-19 Threat Flags) | closed |
| T-51-57 | Information Disclosure | chain refusal text | low | mitigate | reason is the hop's own `check_reasoning` text (provider name, model, effort, key names) plus `fallback hop <index>: `; no hop credential or base URL is in scope (51-19; 51-19 Threat Flags) | closed |
| T-51-58 | Information Disclosure | docs and test fixtures | low | mitigate | examples use parameterised keys and the existing `test-key` / `SECRET` constants only (gitleaks rule); the host tests assert `SECRET` never appears in an error (51-20; 51-20 Threat Flags) | closed |
| T-51-59 | Repudiation | register drift (a behaviour change not recorded for the tag) | medium | mitigate | MIGRATION 9.2 row, DR-21 and CHANGELOG entries in this plan; `make check-migration-allowlist`, `make api-surface` gate it (51-20; 51-20 Threat Flags) | closed |
| T-51-60 | Denial of Service | a consumer pre-judging with a port whose decorator drops the verdict | low | mitigate | MIGRATION row tells decorator authors to forward `check_reasoning`; `the_host_verdict_survives_the_pricing_decorator` (51-20; 51-20 Threat Flags) | closed |
| T-51-SC | Tampering | `cargo install cargo-semver-checks` (supply chain) (plans 51-01/03/07/10: package installs, low, accept — no dependency added) | high | mitigate | blocking-human legitimacy check on crates.io before install; exact version 0.50.0 with `--locked`, the version earlier phases measured with; a developer tool, never a workspace dependency (no `Cargo.lock` change); `.github/workflows/ci.yml` installs `cargo-semver-checks@0.50.0`; no `Cargo.lock` change in the phase | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-51-01 | T-51-08 | operator-trusted by design (T-17-18); the harness's clients inherit the adapters' no-redirect posture (plan 51-02) | owner, at plan 51-02 checkpoint | 2026-10-23 |
| AR-51-02 | T-51-16 | unchanged: every compat client keeps `redirect::Policy::none()` (T-17-18) (plan 51-04) | owner, at plan 51-04 checkpoint | 2026-10-23 |
| AR-51-03 | T-51-21 | consistent with D-00f; the refusal names `llm.<provider>.reasoning_effort`; docs (plan 51-13) say to set it only where every model supports it (plan 51-06) | owner, at plan 51-06 checkpoint | 2026-10-23 |
| AR-51-04 | T-51-SC (plans 51-01/03/07/10) | package installs: no dependency is added by those plans; the one install (51-14) is mitigated above | owner, at plan time | 2026-10-23 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-23 | 61 | 61 | 0 | /gsd-secure-phase 51 (orchestrator, L1 short-circuit; dispatched from /gsd-verify-work 51 after UAT 3/3) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-23
