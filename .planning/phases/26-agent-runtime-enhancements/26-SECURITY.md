---
phase: 26
slug: agent-runtime-enhancements
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: 2026-09-07
---

# Phase 26 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 26 (Agent Runtime Enhancements: execution middleware chain, run budgets and guardrails,
context-window management and summarization, cross-session Vault memory with namespace
confinement, structured output, provider conformance close-out, and the `reasoning_agent`
preset) shipped 21 plans, every one of which carried a `<threat_model>` block at plan time. This
file consolidates those 21 registers into one de-duplicated register of 71 entries (70 numbered
plus the consolidated supply-chain row), records the verification evidence for each mitigation,
and logs the risks the phase accepted by decision. Four SUMMARY files (26-12, 26-17, 26-19,
26-20) carry a `## Threat Flags` section; all four read "None" beyond the plan register. The
other seventeen recorded no new threat surface either.

**Verification depth.** ASVS level 1 with `register_authored_at_plan_time: true`. The
preliminary grep-depth (L1) classification closed every entry at or above the `high` block
threshold, so per the secure-phase short-circuit rule the deeper auditor pass was not spawned.
Evidence below is a file and line, a test name, or a document line that pins the mitigation.
Test-level pins were additionally exercised by a live `cargo test --workspace --all-features`
run in this session (see "Verification Notes").

**Post-execution hardening folded in.** `/gsd-code-review 26 --fix` (26-REVIEW-FIX.md) landed
four behavioural commits and one documentation commit after the plans closed. They strengthen
entries already in this register rather than add new ones: `bc2cd4f3` (CR-01/WR-03,
`ToolResultFormatter::format_result`'s business-failure and success text now redact-then-bound
through one `sanitize_tool_text` helper), `f26647a8` (WR-01, `key=`/`token=` redaction markers
require a word boundary), `12c7562e` (CR-02, an `after_model` `Finish` is no longer dropped
behind an earlier `before_model` `Finish`), and `45c05fbf` (WR-02, documented — see
Verification Notes — that the structured-output surface bypasses the middleware chain). They
are cited where relevant in the Evidence column.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| middleware author → `PromptAssembly` → rendered prompt | Third-party hook code inserts text into the prompt the model receives | Structured `PromptSection`s only, never raw system-prompt concatenation |
| model response → `LlmResponseView` / accumulated output | Provider-controlled text crosses into a view every `after_model` hook can rewrite, and into the guardrail response screen | Untrusted text; screened per part, redaction lands in place |
| middleware hook / summarizer call → run wall-clock | Hook execution time is inside the run's own time budget | Bounded only by Aegis `run_timeout` and the per-run timeout (accepted) |
| operator config file / `APP_*` env → `AgentRuntimeConfig` | Operator values become limits, guardrail regexes, provider names and budgets | Validated with typed errors; no credential-shaped field |
| operator regex → the guardrail matching engine | Config text becomes a compiled program run against every prompt and response | `regex` (linear-time) under an explicit `size_limit` |
| caller schema → `LlmRequest.response_format` → provider request body / prompt instruction block | Caller-authored schema crosses to a third party and into the prompt | Caller data, never logged with a body |
| provider constrained-mode output / model output → `extract_json` → `shape_check` → serde → typed value | Fully model-controlled text becomes a typed Rust value that drives program flow | Untrusted; bounded repair loop; serde is the typed validation |
| model output → repair re-prompt | The offending output is fed back into the next prompt | Quoted as DATA, not instructions |
| model output → `output_schema` node → Battlefield state → downstream nodes | Model data becomes shared workflow state | No raw-string fallback; exhaustion is a `NodeError` |
| caller / model-supplied namespace and key → `Namespace::new` → `ConfinedVault` → store | Path-like untrusted input addresses a shared durable store; from plan 26-16 the caller is the model | Segment validation, then segment-wise grant prefix check, before the backend |
| host grant / `RunScope` → the run's `ConfinedVault` | A per-run value decides which subtree a run may address | Resolution ends in "no grant", never a root grant |
| caller / model value → `VaultRecord.value` → SQLite / semantic store | Arbitrary JSON becomes durable and is later recalled into a prompt | 64 KiB bound; plaintext at rest (accepted); recalled as stored notes |
| vector backend results → returned records | A third-party index proposes candidates | Re-filtered by namespace in our code; record loaded from the authoritative store |
| stored Vault content / summarizer output → Garrison → future prompts | Text an agent or a second model wrote re-enters later prompts | Delimited section framed as stored notes; summary flagged `is_summary` with provenance |
| conversation history → token counting → prompt budget | User- and model-authored text decides how much history reaches the provider | Entries kept whole or dropped whole; never sliced |
| tool error text → the model's context, logs and errors | Text from a tool, library or remote service is inserted into the next prompt | `redact_secret_patterns` **then** `bounded_excerpt`, in the tool-result position |
| model response content → synthesized `function_call` → the Arsenal | Model text becomes a tool invocation | Only for a tool present in `list_armaments()`; `validate_call` still gates arguments |
| several arsenals → one composite → one tool namespace | Name collisions decide which implementation a model reaches | First-registration-wins, every duplicate logged |
| middleware → `ModelCallContext.llm_override` | A hook decides which port a model call reaches | Per-run context only, read at exactly one point |
| provider HTTP response → `LlmError` → logs / model context; API key → request headers → redirects | Third-party bodies cross into rendered errors; a credential crosses the network | Redact-then-bound; `redirect::Policy::none()` on every adapter |
| on-disk v0.9 Garrison DB → the embedded migrator; entry content → SQLite `INSERT` | Operator data crosses a schema change; conversation text enters SQL | Additive migration; `sqlx` parameter binding |
| operator config → `build_chain` → installed middleware; preset defaults → uncustomised callers | Configuration decides which controls are in force | Typed error on a dropped control; preset grants no Vault access |
| `MIGRATION.md` / semver allowlist / public-API export → release record and CI gates | The phase's own claims are what downstream users and the gates rely on | Set-equality CI check; export regenerated in-commit |

---

## Threat Register

Status legend: **closed** = mitigation located in the implementation (Evidence column), or
accepted risk recorded in the Accepted Risks Log below. Paths are relative to the repository
root. Threat IDs that appeared in more than one plan register are listed once with every plan
that carried them.

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-26-01 (26-04, 26-13, 26-16) | Elevation of Privilege | `Namespace::is_prefix_of` / `ConfinedVault` prefix check / a model addressing a namespace outside its grant | critical | mitigate | Segment-wise comparison at `crates/paladin-core/src/platform/container/vault.rs:203`; zero non-comment `starts_with` in that module; `is_prefix_of_is_segment_wise_not_string_wise` (`vault.rs:655`) asserts the `alice`/`alice2` sibling case is false. `ConfinedVault` returns `VaultError::NamespaceDenied` before touching `inner` (`crates/paladin-ports/src/output/vault_confined.rs:85-95`). End-to-end: `hostile_tool_call_to_a_sibling_namespace_is_denied` (`tests/integration/vault_confinement_test.rs`) asserts the backend call count is exactly 0. | closed |
| T-26-02 (26-15) | Tampering | prompt injection via recalled Vault content | high | mitigate | Recall renders in its own delimited `## Long-term memory` `PromptSection` with a fixed stored-notes preamble; `section_frames_entries_as_stored_notes` (`src/application/services/paladin/middleware/vault_recall.rs`). Structured sections cannot be concatenated into the system prompt (T-26-13). | closed |
| T-26-03 (26-19) | Information Disclosure | a secret inside a tool error reaching the model, a log or an error | high | mitigate | `redact_secret_patterns` then `bounded_excerpt` at `src/infrastructure/adapters/arsenal/tool_result_formatter.rs:274-275`; ordering pinned by `redaction_precedes_bounding` (`crates/paladin-llm/src/redaction.rs:422`). Hardened by `bc2cd4f3` (business-failure and success text now go through the same `sanitize_tool_text` helper) and `f26647a8` (marker word-boundary). | closed |
| T-26-04 (26-08) | Denial of Service | a config-supplied `Guardrail` regex | medium | mitigate | `regex` finite-automata engine (no backtracking); every pattern compiled through `RegexBuilder::size_limit(pattern_size_limit_bytes)` at construction with a typed failure (`src/application/services/paladin/middleware/guardrail.rs:15-16,47,216`); `oversized_pattern_is_rejected_at_the_documented_bound`; no `fancy-regex` in any manifest. | closed |
| T-26-05 (26-01, 26-15, 26-21) | Denial of Service | a hanging `ExecutionMiddleware` hook / summarizer model call | medium | accept | AR-26-01. Bounded only by the node's Aegis `run_timeout` and the service's per-run timeout; a per-hook timeout is a named later decision (D-41). | closed |
| T-26-06 (26-04) | Denial of Service | unbounded `VaultRecord.value` | medium | mitigate | `DEFAULT_MAX_VALUE_BYTES` (64 KiB) enforced before the store with typed `ValueTooLarge { bytes, max }` and adapter-level `with_max_value_bytes` (`crates/paladin-memory/src/vault/in_memory.rs` and `sqlite.rs`; override trait on `crates/paladin-core/src/platform/container/vault.rs`). | closed |
| T-26-07 (26-07, 26-09) | Tampering | SQL injection through the Garrison adapter / `SqliteVault` | high | mitigate | Every query uses `sqlx` parameter binding; the only `format!` calls in `crates/paladin-memory/src/garrison/sqlite_garrison.rs` build error messages (lines 271, 303, 455), none build SQL; same pattern in `crates/paladin-memory/src/vault/`. | closed |
| T-26-08 (26-02, 26-10) | Information Disclosure | `AgentRuntimeConfig` / `ModelFallbackConfig` `Debug` or log rendering | low | mitigate | No credential-shaped field in `src/config/agent_runtime.rs` (case-insensitive grep for `api_key`/`secret`/`password`/`token` outside budget names: 0); `config_carries_no_secret_shaped_field` and `config_holds_no_credential`; providers are named, credentials stay on `LlmProviderFactory`. | closed |
| T-26-09 (26-03, 26-06, 26-17) | Information Disclosure | a `response_format` schema logged with a request body | low | mitigate | No adapter in `crates/paladin-llm/src` logs a request body or schema (grep of `debug!/info!/trace!/warn!` for `body`/`schema`/`response_format`: 0); `ResponseFormat` rustdoc (`crates/paladin-ports/src/output/llm_port.rs:558`) describes the schema as caller-appended prompt material. | closed |
| T-26-10 (26-19) | Spoofing | a model synthesizing a call to a tool it was not given | medium | mitigate | Envelope honoured only when the name is in `list_armaments()`; `an_unknown_tool_name_in_the_envelope_is_not_synthesized` (`src/application/services/paladin/middleware/tool_protocol.rs`); `validate_call` still gates arguments. | closed |
| T-26-11 (26-14) | Information Disclosure | a credential surviving into a rendered provider error | high | mitigate | `credential_never_appears_in_a_rendered_error` (`crates/paladin-llm/src/conformance.rs:322`) with the key placed near the truncation boundary, instantiated per adapter via `llm_conformance_suite!`. | closed |
| T-26-12 (26-12, 26-17) | Denial of Service | an unbounded repair loop | medium | mitigate | `max_repair_attempts` defaults to 1 in the shared `run_structured` driver; `zero_repair_attempts_means_one_call`, `exhaustion_returns_the_typed_error_with_raw_preserved`, `the_repair_loop_is_the_shared_driver` (`src/application/services/paladin/structured.rs`, `paladin_execution_service.rs`). | closed |
| T-26-13 (26-01) | Tampering | middleware-inserted `PromptSection` text | low | mitigate | `PromptSection { heading, body, placement }` renders under its own delimited heading (`src/application/services/paladin/middleware/context.rs:59-65`); never concatenated into the system prompt. | closed |
| T-26-14 (26-01) | Elevation of Privilege | `MiddlewareFlow::Fail` swallowing or rewriting an error | low | mitigate | `fail_from_before_model_propagates_unchanged_and_is_not_retried` (`src/application/services/paladin/middleware/chain.rs`); no `catch_unwind` in the chain. Adjacent precedence fix `12c7562e` (CR-02). | closed |
| T-26-15 (26-02) | Denial of Service | an operator-supplied `GuardrailConfig.pattern` | medium | mitigate | `pattern_size_limit_bytes` config field with documented default (`src/config/agent_runtime.rs`), consumed by the compile path in T-26-04. | closed |
| T-26-16 (26-02) | Tampering | a silently-clamped invalid config value | low | mitigate | `validate()` returns a typed error naming the field; `validate_rejects_zero_and_out_of_range_scalars` (`src/config/agent_runtime.rs`). | closed |
| T-26-17 (26-03) | Tampering | a missed `LlmRequest` construction site keeping a stale default | medium | mitigate | `#[non_exhaustive]` on `LlmRequest` (`crates/paladin-ports/src/output/llm_port.rs:649`) makes a missed literal a compile error outside `paladin-ports`. | closed |
| T-26-18 (26-03, 26-05) | Repudiation | an allowlist entry with no matching register row | medium | mitigate | CI `semver` job verifies set-equality between `.cargo/semver-checks-allowlist.toml` and MIGRATION.md §9.2 `Y` rows in both directions (`.github/workflows/ci.yml:351-360`); the `LlmRequest`, `StopReason` and `GarrisonEntry` entries are present with justification. | closed |
| T-26-19 (26-04) | Tampering | `Namespace` segment validation | high | mitigate | `.`, `..`, `/` and control characters rejected at construction with typed `InvalidNamespace` (`crates/paladin-core/src/platform/container/vault.rs`); 8 individual rejection assertions in the core/ports test modules. | closed |
| T-26-20 (26-04, 26-09) | Information Disclosure | backend text in `VaultError::Storage` / `Serialization` | medium | mitigate | Redact-before-bound on every mapped backend error; `storage_errors_are_typed_and_redacted` (`crates/paladin-memory/src/vault/sqlite.rs`). | closed |
| T-26-21 (26-04, 26-09) | Information Disclosure | Vault values stored as plaintext JSON | low | accept | AR-26-02. 26-RESEARCH.md Open Question 2 records encryption at rest as out of scope, consistent with the operator-responsibility stance for secrets. | closed |
| T-26-22 (26-05, 26-20) | Denial of Service | an unbounded reasoning loop, token spend or tool loop | medium | mitigate | `ModelCallLimit`, `TokenBudget`, `ToolCallLimit` (`src/application/services/paladin/middleware/limits.rs`), inert by default; `token_budget_overshoot_is_at_most_one_response`; preset installs `ToolCallLimit` from `max_tool_calls` (default 20) and `max_loops` 5; `max_tool_calls_is_enforced` (`tests/integration/reasoning_agent_test.rs`). | closed |
| T-26-23 (26-05) | Tampering | the denial message read as an instruction by the model | low | mitigate | Fixed framework function `tool_budget_exhausted_message(tool_name)` (`limits.rs:179`) naming only the tool, delivered in the tool-result position; `denied_tool_call_reason_reaches_the_model`. | closed |
| T-26-24 (26-05) | Repudiation | a downstream matcher silently mis-classifying a new `StopReason` | medium | mitigate | `CallLimit` and `TokenBudget` matched explicitly in `crates/paladin-web/src/agent_controller.rs:155-156`, `src/application/cli/formatters/output.rs` and `paladin_execution_service.rs`; enum `#[non_exhaustive]` with the X-10.2 exception declined and documented (`execution_result.rs:109`). | closed |
| T-26-25 (26-06, 26-17) | Tampering | trusting a provider's constrained mode as validation | medium | mitigate | Instruction block appended on every structured call regardless of `response_format`; `structured_run_also_appends_the_instruction_block` (`paladin_execution_service.rs`); parse and shape check always run. | closed |
| T-26-26 (26-06, 26-19) | Spoofing | an adapter silently gaining a tool-calling surface | medium | mitigate | ADR-0042 untouched: no `LlmRequest.tools`; `test_capabilities_tool_calling_matches_request_surface` (`crates/paladin-llm`). | closed |
| T-26-27 (26-07) | Tampering | a schema change losing or corrupting v0.9 Garrison data | high | mitigate | Single additive `002_add_garrison_is_summary.sql` (`ADD COLUMN … NOT NULL DEFAULT 0`); `001` byte-untouched; `existing_v0_9_database_migrates_forward` (`crates/paladin-memory/src/garrison/sqlite_garrison.rs`). | closed |
| T-26-28 (26-07, 26-09) | Tampering | schema divergence between two migration sources | medium | mitigate | Exactly one `migrations/` directory and one `sqlx::migrate!` in `paladin-memory` (`crates/paladin-memory/src/migrations.rs:36`, shared `static MIGRATOR` for Garrison and Vault); root `migrations/` mirror deleted; `Dockerfile` `COPY` lines removed (`Dockerfile:28-29`). See Verification Notes for the `Dockerfile.server`/`Dockerfile.chef` residual. | closed |
| T-26-29 (26-07) | Denial of Service | a deployment losing its schema at upgrade | medium | mitigate | Migrations embedded in the binary at compile time (`migrations.rs:36`); `docs/src/deployment/docker.md` updated in the same change. | closed |
| T-26-30 (26-08) | Information Disclosure | a redaction that misses because the prompt was flattened first | medium | mitigate | Screens run over `PromptAssembly` parts before rendering; `prompt_screen_redacts_in_the_matching_section` (`guardrail.rs`) asserts the untouched section is byte-identical. | closed |
| T-26-31 (26-08) | Information Disclosure | an operator rule name echoed into an error | low | accept | AR-26-03. `PaladinError::GuardrailTripped { rule, .. }` (`crates/paladin-core/src/platform/container/paladin_error.rs:114-117`) carries the operator's own rule name and no matched content. | closed |
| T-26-32 (26-09) | Elevation of Privilege | a vector backend returning a foreign-namespace hit | high | mitigate | `SemanticVault::search` re-filters every hit with `Namespace::is_prefix_of` and loads the authoritative record from the store; `search_re_filters_by_namespace_even_when_the_backend_does_not` (`crates/paladin-memory/src/vault/semantic.rs`). | closed |
| T-26-33 (26-09) | Elevation of Privilege | `list` leaking descendant namespaces | high | mitigate | `list` matches `ns = ?` exactly; `list_scopes_to_exactly_the_namespace` (`crates/paladin-memory/src/vault/sqlite.rs`). | closed |
| T-26-34 (26-10) | Elevation of Privilege | an `llm_override` leaking across concurrent runs | high | mitigate | Override lives on the per-run `ModelCallContext`, read at one point (`model_call_port_is_resolved_at_exactly_one_point`); `concurrent_runs_do_not_share_an_override` (`src/application/services/paladin/middleware/resilience.rs`). | closed |
| T-26-35 (26-10) | Information Disclosure | a fallback hop log naming a provider and an error | low | mitigate | Hop logging reuses `FallbackLlmAdapter::record_hop` (redaction path); no second logging site in `resilience.rs`. | closed |
| T-26-36 (26-10) | Denial of Service | a misconfigured retry policy amplifying load on a failing provider | medium | mitigate | Delays from `backoff_delay`'s capped exponential with jitter; `max_attempts == 0` is a typed error (Phase 25); `model_retry_config_maps_to_retry_policy_defaults` (`src/config/agent_runtime.rs:1739`). | closed |
| T-26-37 (26-11) | Denial of Service | an unbounded conversation exceeding the context window | medium | mitigate | `HistoryTrimmer` bounds history against a resolved limit with a reserved response budget, degrades to empty, inert by default (`src/application/services/paladin/middleware/history.rs`). | closed |
| T-26-38 (26-11) | Tampering | a mis-declared provider `max_context_tokens` silently over/under-trimming | low | mitigate | Operator `model_context_limits` table resolved first; resolved limit and its source logged at debug (`history.rs:155-160`, asserted at `history.rs:306-308`). | closed |
| T-26-39 (26-11) | Information Disclosure | a truncated message leaking half a secret into the prompt | low | mitigate | `an_entry_is_kept_whole_or_dropped_whole` (`history.rs:399`). | closed |
| T-26-40 (26-12) | Tampering | the repair re-prompt echoing model output as an instruction | medium | mitigate | Offending output re-inserted as "DATA, NOT instructions" (`src/application/services/paladin/structured.rs:179`), written once in the driver. | closed |
| T-26-41 (26-12) | Tampering | over-trusting a partial shape check | medium | mitigate | `shape_check` rustdoc enumerates the enforced keywords and states others are ignored (`crates/paladin-core/src/platform/container/structured.rs:165-189`); `shape_check_enforces_exactly_the_documented_subset` (`structured.rs:404`) proves `minLength` is not enforced. | closed |
| T-26-42 (26-12) | Denial of Service | a deeply nested schema or value in `shape_check` | low | mitigate | Recursion is over `properties` and `items` only, bounded by the caller-authored schema's own depth (`structured.rs:165,174`). The planned "caller-authored, not attacker-authored" sentence is not in the rustdoc verbatim — a documentation nit, not a control gap. | closed |
| T-26-43 (26-13) | Elevation of Privilege | a run with no grant defaulting to the root namespace | critical | mitigate | Resolution ends in no grant: no `ConfinedVault`, no vault tools listed, every vault call denied; `no_grant_means_denied_not_root` (`paladin_execution_service.rs`). | closed |
| T-26-44 (26-13, 26-16) | Elevation of Privilege | cross-run leakage of a grant under concurrency | high | mitigate | Grant resolved per run from `RunScope`; `concurrent_confined_writes_produce_zero_cross_namespace_records` (`crates/paladin-battalion/src/engine/mod.rs`) and `concurrent_confined_tool_writes_produce_zero_cross_namespace_records` (`tests/integration/vault_confinement_test.rs`) on a multi-thread executor with timeout guard. | closed |
| T-26-45 (26-13) | Tampering | a `PaladinPort` implementor silently losing the scope | medium | mitigate | `execute_scoped`'s default body delegates to `execute_observed` (`crates/paladin-ports/src/output/paladin_port.rs:842`), not `unimplemented!()`; `the_engine_always_calls_execute_observed`. | closed |
| T-26-46 (26-14) | Information Disclosure | a credential header forwarded on a cross-host redirect | high | mitigate | `redirect_is_not_followed_with_a_credential_header` (`conformance.rs:367`) asserts the redirect target is never contacted; `reqwest::redirect::Policy::none()` in every adapter (`anthropic/adapter.rs:163`, `deepseek/adapter.rs:347`, `openai_compatible/adapter.rs:572`, `ollama`, `kimi`, `qwen`). | closed |
| T-26-47 (26-14) | Repudiation | a transience misclassification hiding a retryable failure | medium | mitigate | `transience_by_value` (`conformance.rs:274-304`) asserts `LlmError::transience()` for 408/429/5xx vs other 4xx per adapter. | closed |
| T-26-48 (26-14) | Repudiation | a conformance claim that measured nothing | medium | mitigate | `llm_conformance_suite!` emits `CASE_COUNT` (`conformance.rs:430`) and `suite_generates_the_full_case_list_for_a_fixture` (`conformance.rs:646`); a dropped case is a build failure. | closed |
| T-26-49 (26-15) | Tampering | a model-authored summary becoming durable, trusted context | medium | mitigate | Summary stored as a `ConversationRole::System` entry flagged `is_summary` in the history region with `metadata["summarized_through"]` provenance (`src/application/services/paladin/middleware/summarization.rs`, `GarrisonEntry::summary`). | closed |
| T-26-50 (26-15) | Denial of Service | a failing summarizer stalling or failing runs | medium | mitigate | Any summarizer error degrades to the embedded trimmer, sets `scratch["summarization.degraded"]`, warns and continues; `degradation_does_not_depend_on_chain_order` (`summarization.rs`). | closed |
| T-26-51 (26-15) | Denial of Service | a per-run warning storm from an unsupported search | low | mitigate | `unsupported_warned: AtomicBool` warns once per middleware instance (`vault_recall.rs:118-131,175`). | closed |
| T-26-52 (26-16) | Elevation of Privilege | a run without a grant reaching the vault tools | critical | mitigate | `vault_tools_are_not_listed_without_a_grant` (`src/application/services/arsenal/vault_tools.rs`, `paladin_execution_service.rs`); with T-26-43's resolution rule. | closed |
| T-26-53 (26-16) | Spoofing | a tool-name collision routing a model to the wrong implementation | medium | mitigate | `CompositeArsenalPort` first-registration-wins with every duplicate logged at `warn` naming tool and member position (`src/application/services/arsenal/composite_arsenal.rs:13-20,69`). | closed |
| T-26-54 (26-16) | Tampering | model-authored Vault content later re-entering a prompt | medium | mitigate | Handled on the read path by T-26-02's delimited stored-notes section; guide inherits M-B-04's raw-content warning. | closed |
| T-26-55 (26-16) | Information Disclosure | vault tools reachable over the HTTP agent API | medium | accept | AR-26-04. Unreachable by construction: HTTP-served agents carry no Arsenal (ADR-0039), stated in rustdoc at `paladin_execution_service.rs:546-550`. | closed |
| T-26-56 (26-17) | Tampering | trusting a partial shape check as type validation | medium | mitigate | serde deserialization is the typed validation; `serde_deserialization_is_the_typed_validation` (`structured.rs`) routes a shape-passing, serde-failing value to the repair loop. | closed |
| T-26-57 (26-18) | Tampering | a fingerprint collision between structurally different graphs | high | mitigate | `output_schema` hashed sorted and length-prefixed under tag `v6`; `fingerprint_changes_when_output_schema_changes`, `fingerprint_version_is_v6_and_the_golden_is_repinned`, `engine_limits_are_still_excluded_from_the_hash` (`crates/paladin-battalion/src/engine/`). | closed |
| T-26-58 (26-18) | Tampering | a misconfigured structured node failing at runtime instead of validation | high | mitigate | `EngineError::UnregisteredOutputSchema`, `OutputSchemaWithStructuredDirective`, `OutputSchemaFieldNotJson` at `validate()` (`crates/paladin-battalion/src/engine/graph.rs:1021,1050,1085`) plus `validate_structured_executor_backend`; each lists every offender. | closed |
| T-26-59 (26-18) | Tampering | a node silently writing a raw string where downstream expects an object | medium | mitigate | No fallback path; `exhaustion_becomes_a_node_error_with_unknown_transience` (`tests/integration/structured_engine_node_test.rs`) asserts `output_field` untouched. | closed |
| T-26-60 (26-18) | Repudiation | a transience classification silently disabling node-level recovery | medium | mitigate | Exhaustion classified `Unknown`; `a_transient_and_unknown_aegis_may_still_retry_the_node` (`structured_engine_node_test.rs`). | closed |
| T-26-61 (26-19) | Tampering | overwriting a real adapter's `function_call` | medium | mitigate | Middleware acts only when `function_call` is `None`; `a_response_that_already_has_a_function_call_is_untouched` (`tool_protocol.rs`). | closed |
| T-26-62 (26-19) | Tampering | fed-back tool errors read as instructions | medium | mitigate | Error lands in the tool-result position with "You may retry with corrected arguments or proceed without it." (`paladin_execution_service.rs:5456` assertion); `both_arms_use_the_same_formatter`. | closed |
| T-26-63 (26-20) | Elevation of Privilege | a preset silently granting Vault access | high | mitigate | `the_preset_does_not_enable_vault_tools` (`tests/integration/reasoning_agent_test.rs`). | closed |
| T-26-64 (26-20) | Repudiation | `build_chain` silently dropping a control the operator configured | high | mitigate | `build_chain_requires_the_dependency_a_section_needs`, `build_chain_reports_every_configuration_failure_at_once` (`src/config/agent_runtime.rs`). | closed |
| T-26-65 (26-20) | Denial of Service | a failing provider hammered by the preset's default breaker | low | mitigate | `defaults_match_the_documented_options` (`src/presets/mod.rs`, `reasoning_agent_test.rs`) pins `CircuitBreaker::new(3, 2, 30s)`. | closed |
| T-26-66 (26-21) | Repudiation | an incomplete or stale `MIGRATION.md` | high | mitigate | §§9.2-9.5 completed in `21163fb9`; the only remaining `TBD`s (lines 256, 354) are explicitly owned by Phase 29 `SHIP-01`/`SHIP-02`; review-fix amendment to M-B-03 in `cfc664d5`. | closed |
| T-26-67 (26-21) | Repudiation | a semver allowlist wider than its justification | high | mitigate | Same set-equality gate as T-26-18 (`ci.yml:351-360`); exactly the three Phase 26 entries (`LlmRequest`, `StopReason`, `GarrisonEntry`) added, each with a requirement id and justification. | closed |
| T-26-68 (26-21) | Repudiation | a green `api-surface` job proving nothing | high | mitigate | `.project/current-exports.txt` (the file `ci.yml:216` checks) regenerated in `21163fb9` as part of plan 26-21. | closed |
| T-26-69 (26-21) | Repudiation | a traceability anchor naming a test that does not exist | medium | mitigate | 26-VALIDATION.md anchors name real test functions (e.g. row 26-09-02 → `search_re_filters_by_namespace_even_when_the_backend_does_not`); every test named in this register was located in the tree. | closed |
| T-26-70 (26-21) | Repudiation | claiming a tier passed that could not run locally | medium | mitigate | 26-VALIDATION.md lines 43-60 record Docker as unavailable and the Qdrant and live-Ollama tiers as CI-only / UAT-routed, never as local passes. | closed |
| T-26-SC (all plans; mitigate in 26-12, 26-21) | Tampering | supply chain: cargo installs across the phase | high | mitigate | One direct dependency added across the phase, `schemars = "1.2"` (`Cargo.toml:129`), verdict `OK` in 26-RESEARCH.md's Package Legitimacy Audit, already resolved via `rmcp 2.1.0`; `Cargo.lock` holds exactly two `schemars` packages; `exactly_two_schemars_versions_remain` (`structured.rs:411`); `make security` exit 0 in 26-21 (`advisories ok, bans ok, licenses ok, sources ok`). The nineteen per-plan "installs no package" accept rows fold into this entry. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Totals:** 71 register entries (70 numbered plus the consolidated supply-chain row) — 3 critical,
19 high, 35 medium, 14 low. 67 mitigated and verified, 4 accepted. **threats_open: 0.**

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-26-01 | T-26-05 | A hanging `ExecutionMiddleware` hook (including the summarizer's own model call) is bounded only by the node's Aegis `run_timeout` and the service's per-run timeout. Hooks are in-process code with the same trust as a `StateNode` (cf. R-23-01 / AR-25-05). A per-hook timeout is a named later decision. The trait-level rustdoc records hook scope ("once per Aegis attempt", `middleware/mod.rs:20`) but does not spell out the shared budget verbatim; this log is the authoritative record of the acceptance. | Plans 26-01, 26-15, 26-21 (D-41) | 2026-09-07 |
| AR-26-02 | T-26-21 | Vault values are stored as plaintext JSON in SQLite / the semantic store. No RT-FR names encryption at rest; 26-RESEARCH.md Open Question 2 records the decision as consistent with the project's operator-responsibility stance for secrets (the same stance taken for LLM API keys). Adding it would be unplanned scope under X-03. Values inherit M-B-04's raw-content warning. | Plans 26-04, 26-09 | 2026-09-07 |
| AR-26-03 | T-26-31 | `PaladinError::GuardrailTripped.rule` carries the operator's own configured rule name — not secret material and the only useful identifier for diagnosing a trip. No matched content is included in the error. | Plan 26-08 | 2026-09-07 |
| AR-26-04 | T-26-55 | Vault tools are unreachable over the HTTP agent API by construction: ADR-0039 gives HTTP-served agents no Arsenal at all, so there is nothing to gate. Stated in rustdoc (`paladin_execution_service.rs:546-550`) rather than worked around with an HTTP-specific check that would imply the path exists. | Plan 26-16 (D-22) | 2026-09-07 |

*Accepted risks do not resurface in future audit runs.*

---

## Verification Notes

- **Method.** Grep-depth (L1) verification per the secure-phase short-circuit rule: ASVS level 1,
  register authored at plan time, zero open entries at or above `high` after preliminary
  classification. The `gsd-security-auditor` subagent was therefore not spawned. Each row's
  Evidence column names the file and line, test, or document line that pins the mitigation.
  Negative greps recorded in the register (`starts_with` in the vault namespace module,
  `format!`-built SQL in the SQLite adapters, credential-shaped fields in `agent_runtime.rs`,
  body/schema logging in `paladin-llm`, `fancy-regex` in any manifest) were run in this session
  and returned 0. Every test function named in the register was located in the tree.
- **Live test run.** `cargo test --workspace --all-features --no-fail-fast` was run in this session on
  2026-09-07 (54 suites, **5711 passed, 3 failed, 322 ignored**). Every security-pinning test named
  in the register ran and passed, including the four adapter instantiations each of
  `credential_never_appears_in_a_rendered_error`, `redirect_is_not_followed_with_a_credential_header`
  and `transience_by_value`. The three failures are not security regressions and none touches a
  register row's control: (1) `test_cli_feature_is_not_default` is a guard that panics whenever
  the `cli` feature is compiled in, which `--all-features` does by construction — `default` in
  `Cargo.toml:359` still excludes `cli`; (2) the `graph_prefix` doctest
  (`crates/paladin-battalion/src/engine/cache_key.rs:68`) still asserts the pre-Phase-26 `k1:v5:`
  prefix after plan 26-18 bumped the fingerprint tag to `v6` (T-26-57's real pins,
  `fingerprint_version_is_v6_and_the_golden_is_repinned` and siblings, pass); (3) the
  `WarEngine::with_vault` doctest (`crates/paladin-battalion/src/engine/mod.rs:1744`, plan 26-13)
  calls a hidden `# fn build_vault() { unimplemented!() }` helper without `no_run`, so the
  example panics when executed. Items (2) and (3) also fail under default features
  (`cargo test -p paladin-battalion --doc`) and CI runs `cargo test --workspace --doc`
  (`ci.yml:431`); they are documentation-example drift owed to Phase 26 and are flagged as a
  follow-up fix, not a threat. The Docker-gated Qdrant and live-Ollama tiers did not run locally,
  consistent with T-26-70.
- **Noteworthy observations.**
  - **Structured output bypasses the middleware chain (WR-02, documented, not fixed).**
    `execute_json_schema` / `execute_structured_call` do not run the `ExecutionMiddleware`
    chain, so `Guardrail` (T-26-04, T-26-30), `VaultRecallMiddleware` (T-26-02),
    `ToolCallLimit` / `TokenBudget` / `ModelCallLimit` (T-26-22) and custom hooks are inert on
    that surface. The register rows above are scoped to the middleware-driven `execute` path,
    which is what the plans claimed; the structured path keeps its own bounded repair loop
    (T-26-12) and prompt-instruction discipline (T-26-25, T-26-40). `45c05fbf` states this on
    `StructuredExecutorPort`, `StructuredExecutorExt`, the service impl, and in
    `docs/src/user-guides/agent-runtime.md`. Anyone relying on guardrail screening for
    structured calls needs a later phase to lift the chain onto that surface.
  - **`Dockerfile.server` and `Dockerfile.chef` still `COPY migrations ./migrations`**
    (`Dockerfile.server:32`, `Dockerfile.chef:63,96`) although plan 26-07 deleted the root
    `migrations/` mirror (only the primary `Dockerfile` was in scope). Runtime is unaffected —
    migrations are embedded in the binary (T-26-29) — but `make docker-build-server` will fail
    at that `COPY` step until the lines are removed. Neither file is built in CI. Housekeeping
    follow-up, not a threat; recorded so it is not rediscovered.
  - **T-26-03 shipped stronger than planned.** The review-fix pass found that
    `ToolResultFormatter::format_result`'s business-failure and success text bypassed
    redact-then-bound entirely (CR-01/WR-03, `bc2cd4f3`) and that the `key=`/`token=` markers
    over-matched ordinary words (WR-01, `f26647a8`). Both now go through the shared
    `redact_secret_patterns` → `bounded_excerpt` sequence. IN-01 (JWT-triple redaction can
    over-redact long hyphenated hostnames) was out of the fix scope and errs in the safe
    direction; it is recorded in 26-REVIEW.md, not here.
  - **`make security`** (26-21 gate evidence) exits 0 under the configured policy. cargo-audit
    still carries the six pre-existing `unmaintained` notices, two non-vulnerability advisories
    and one yanked-crate warning noted in Phase 25; none is a Phase 26 dependency change and
    `SECURITY-EXCEPTIONS.md` housekeeping remains open from Phase 25.
  - Known gap carried from `security.instructions.md`: there is still no merge-gating Rust
    SAST. The manual credential-handling review plus the review-fix pass are the primary
    control for T-26-03/08/09/11/20/35/46; CodeQL stays advisory-only.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-07 | 71 | 71 | 0 | /gsd-secure-phase 26 (Claude, L1 grep-depth + live test run) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-07
