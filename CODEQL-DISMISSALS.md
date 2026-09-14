# CodeQL Alert Dismissal Register

This file is the authoritative **governance** register for every CodeQL code-scanning alert this
Paladin workspace dismisses. It exists because a code-scanning dismissal in GitHub's UI is a
free-text reason string attached to the alert — it carries no named owner, no review date, no
scope and no compensating control, and none of it is queryable outside the platform itself. The
four fields a governed dismissal needs can only live somewhere a script can read them structurally.
This register gives those fields a structured home, modelled directly on `SECURITY-EXCEPTIONS.md`
(D-17), which already proved this register-plus-guard shape for advisory suppressions
(ADR-0036).

**The register is the governance surface; GitHub's alert store is the enforcement surface.** They
are reconciled by a named manual command, not by the guard below — every guard script in this
repository is offline by design, and this one is no exception. Run this to compare the register
against the live dismissed-alert set on GitHub:

```bash
gh api "/repos/DF3NDR/paladin-dev-env/code-scanning/alerts?state=dismissed&per_page=100" --paginate
```

Running that command is a human step. The guard below does not perform it, does not make any
network call, and does not claim to know what GitHub's live alert store currently holds — it
validates only that this file is internally consistent and not stale.

**Adoption context.** Per `.planning/phases/18-rust-sast-evaluate-and-adopt-codeql/18-CODEQL-EVIDENCE.md`'s
`## Verdict`, CodeQL (`2.26.3` / `rust-queries` `0.1.40`) is disqualified from promotion to a
required check — `.github/workflows/codeql.yml` runs advisory-only, on every push/PR/schedule,
never gating a merge. That status does not make a dismissed alert's governance optional: the one
class CodeQL's Rust query suite fires reliably on (`rust/hard-coded-cryptographic-value`) still
produces alerts a human has to triage, and an ungoverned dismissal of one of those alerts is the
same assurance-theatre failure mode this register exists to prevent regardless of whether the scan
that raised it can block a merge.

**Schema contract, stated plainly:** dismissing an alert on the platform without adding a row here
is a governance failure the guard cannot see, so it is stated as a rule and checked at
reconciliation. Adding a row here with any empty field, a past review date, or a duplicate alert
number fails CI.

## The 6 live dismissals

Declared dismissals: 6

<!-- BEGIN MACHINE-READABLE REGISTER -->
```toml
[[dismissal]]
alert_number = 28
rule_id = "rust/hard-coded-cryptographic-value"
path = "src/core/platform/manager/user_service.rs:1582"
why_present = "The rule's heuristic password-parameter sink fires on the literal \"any-password\" passed as the first argument to service.verify_password(\"any-password\", \"not-a-valid-phc-hash\") at user_service.rs:1582, inside #[tokio::test] async fn verify_password_against_a_malformed_hash_returns_a_hash_error(), itself inside the #[cfg(test)] mod tests block opening at line 491."
why_dismissed = "Direct source inspection confirms this is test-fixture data exercising the malformed-hash error path, not a leaked production credential. The literal carries no cryptographic sensitivity, is never logged, and is not derived from or usable against any real credential store -- it is an arbitrary string chosen only to be a syntactically-valid password argument."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2026-12-31"
scope = "test-only code path -- the #[cfg(test)] mod tests block in crates/paladin-core's user_service.rs; never compiled into a release binary"
compensating_control = "The flagged literal never leaves the #[cfg(test)] compilation unit: it is not read from configuration, not passed to any crypto API outside this test's own assertion, and not reachable from any code path a release build includes."
revisit_condition = "the rust-queries heuristic changes its password-parameter sink matching such that this test literal stops flagging on its own, or the test is refactored to use a non-literal placeholder value -- whichever comes first"

[[dismissal]]
alert_number = 40
rule_id = "rust/cleartext-logging"
path = "crates/paladin-memory/src/services/rag_retrieval_service.rs:106"
why_present = "The rule's sensitive-data heuristic marks any value flowing from a field named `uuid` as private information. `PaladinExecutionService::retrieve_context_with_timeout` computes `let paladin_id = paladin.uuid.to_string()` and passes it to `RagRetrievalService::retrieve_context`, whose `log::debug!(\"Retrieved {} memories for paladin {} with query: {}\", ...)` at rag_retrieval_service.rs:106 interpolates that `paladin_id`. The flagged source is the `.to_string()` on the UUID, not the query text."
why_dismissed = "A Paladin's UUID is the entity identifier every log line in this codebase correlates on (the same identifier the Citadel and Battalion examples print by design). It is not a credential, session token, or personal datum, and it is written at `debug` level only. Source inspection confirms no API key, token, or response body flows into this statement."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-03-31"
scope = "the single debug! statement in RagRetrievalService::retrieve_context; the taint is the Paladin entity UUID passed as paladin_id, unchanged on main and this branch"
compensating_control = "The log is debug-level, so it is not emitted at the production default level; the interpolated value is an opaque v4 UUID that grants no access on its own; the manual credential-handling review in .github/instructions/security.instructions.md remains the control for any log that touches a real secret."
revisit_condition = "the rust-queries sensitive-data heuristic stops classifying a bare `uuid` field as private information, or the Paladin identifier type gains a redacted Display that this log should use instead -- whichever comes first"

[[dismissal]]
alert_number = 45
rule_id = "rust/cleartext-logging"
path = "src/application/services/paladin/prompt_generation_service.rs:112"
why_present = "`PaladinBuilder::user_name(...)` is a fluent setter whose name matches the rule's private-information heuristic, so every call to it taints the whole builder value. `PaladinBuilder::build` later reads the unrelated `self.data.name` field off that tainted builder, passes it as `agent_name` into `PromptGenerationService::generate_prompt`, and the `info!(\"Generating system prompt for agent: {}\", agent_name)` at prompt_generation_service.rs:112 is reported as logging `user_name`."
why_dismissed = "The value written is `PaladinData::name`, the Paladin's own display name (e.g. \"CodeReviewer\"), never `PaladinData::user_name`. The taint is whole-struct over-approximation from a same-named setter on a different field; direct inspection of `build()` (paladin_builder.rs, the `agent_name` binding just before the `generate_prompt` call) confirms the two fields are never conflated."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-03-31"
scope = "the info! statement at the top of PromptGenerationService::generate_prompt; the only argument interpolated is agent_name, which the builder derives from PaladinData::name"
compensating_control = "The Paladin name is a caller-chosen, non-secret label already surfaced by the builder's own `Auto-generated system prompt for agent` log and by Herald output; no credential or user identity flows through this parameter."
revisit_condition = "rust-queries gains field-sensitive taint tracking for struct setters (so a `user_name` setter no longer taints sibling fields), or PromptGenerationService::generate_prompt changes its signature to receive something other than the Paladin display name -- whichever comes first"

[[dismissal]]
alert_number = 46
rule_id = "rust/cleartext-logging"
path = "src/application/services/paladin/prompt_generation_service.rs:119"
why_present = "Same flow as alert 45: `PaladinBuilder::user_name(...)` taints the whole builder, `build()` passes `self.data.name` as `agent_name`, and the cache-hit branch's `info!(\"Using cached prompt for agent: {}\", agent_name)` at prompt_generation_service.rs:119 is reported as logging `user_name`."
why_dismissed = "Identical reasoning to alert 45: the interpolated value is `PaladinData::name`, the Paladin's non-secret display name, not `PaladinData::user_name`. The three alerts 45/46/47 are one over-approximated flow reported at three sink lines of the same function."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-03-31"
scope = "the cache-hit info! statement inside PromptGenerationService::generate_prompt; same agent_name argument as alert 45"
compensating_control = "As for alert 45: the Paladin display name is a caller-chosen, non-secret label; no credential or user identity flows through this parameter."
revisit_condition = "as for alert 45 -- field-sensitive setter taint in rust-queries, or a signature change to generate_prompt, whichever comes first"

[[dismissal]]
alert_number = 47
rule_id = "rust/cleartext-logging"
path = "src/application/services/paladin/prompt_generation_service.rs:169"
why_present = "Same flow as alert 45: `PaladinBuilder::user_name(...)` taints the whole builder, `build()` passes `self.data.name` as `agent_name`, and the post-generation `info!(\"Generated system prompt for agent: {}\", agent_name)` at prompt_generation_service.rs:169 is reported as logging `user_name`."
why_dismissed = "Identical reasoning to alert 45: the interpolated value is `PaladinData::name`, the Paladin's non-secret display name, not `PaladinData::user_name`. Note the adjacent `debug!` on the following line logs the generated prompt body and is deliberately debug-level; it is not what this alert flags."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-03-31"
scope = "the post-generation info! statement inside PromptGenerationService::generate_prompt; same agent_name argument as alert 45"
compensating_control = "As for alert 45: the Paladin display name is a caller-chosen, non-secret label; no credential or user identity flows through this parameter."
revisit_condition = "as for alert 45 -- field-sensitive setter taint in rust-queries, or a signature change to generate_prompt, whichever comes first"

[[dismissal]]
alert_number = 49
rule_id = "rust/cleartext-logging"
path = "src/bin/paladin-server.rs:387"
why_present = "`build_auth_config` collects `cfg.api_keys` into the `AgentAuthConfig::api_keys` map; both names match the rule's credential heuristic, so the map is a tainted source. The startup banner `info!(\"agent API authentication ENABLED ({} API key(s){})\", auth.api_keys.len(), ...)` at paladin-server.rs:387 interpolates `auth.api_keys.len()`, and taint propagates through `.len()`."
why_dismissed = "The value written is the integer count of configured API keys, never a key, a key prefix, or a principal name. Direct inspection of the `info!` arguments confirms only `.len()` and a static `\" + bearer token\"` suffix are interpolated. The count is deliberately logged so an operator can confirm at boot that fail-closed auth found the credentials it expected (the surrounding `has_credentials()` guard refuses to start otherwise)."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-03-31"
scope = "the single startup info! in build_auth_config in the paladin-server binary; the only tainted expression is auth.api_keys.len()"
compensating_control = "The `AuthConfig`/`AgentAuthConfig` types carrying the keys are never Debug-formatted or serialised outward in this function, per the manual credential-handling review in .github/instructions/security.instructions.md; only the map's length reaches the log sink."
revisit_condition = "rust-queries stops propagating credential taint through `.len()` on a keyed collection, or the startup banner is reworked to report credential presence via a dedicated non-tainted summary type -- whichever comes first"
```
<!-- END MACHINE-READABLE REGISTER -->
