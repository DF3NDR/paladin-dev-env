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

**Phase 52 batch.** The rows added by Phase 52 (alerts 30 through 81, every one of them triaged against the source line on the current tree) share the review date 2027-06-30 so one sweep re-triages them together; the six earlier rows (alerts 28, 40, 45, 46, 47 and 49) keep their own review dates.

## The 37 live dismissals

Declared dismissals: 37

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
alert_number = 30
rule_id = "rust/cleartext-logging"
path = "examples/battalion_checkpoint_recovery.rs:87"
why_present = "In `main` of the battalion_checkpoint_recovery example, the `println!` at examples/battalion_checkpoint_recovery.rs:87 prints `extract_paladin.uuid` in its \"ExtractPaladin created (ID: ...)\" line, directly after `PaladinBuilder::build()` returns. The rule's sensitive-data heuristic marks a value read from a field named `uuid` as private information."
why_dismissed = "The only interpolated argument is the Paladin entity UUID, the identifier every Citadel and Battalion line correlates on. It is not a credential, token or personal datum and grants no access on its own. Direct inspection of lines 81-87 shows no key, response body or caller-supplied text reaching the statement. Same classification as alert 40, here in an example binary rather than a library log."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 87 in the main function of examples/battalion_checkpoint_recovery.rs; an example binary, not part of the shipped paladin-server binary or any library surface"
compensating_control = "The statement writes to the terminal of whoever runs the example by hand; it is stdout, not a log pipeline. The printed UUID is an opaque v4 value that authorises nothing."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the example is changed to print something other than the Paladin UUID -- whichever comes first"

[[dismissal]]
alert_number = 31
rule_id = "rust/cleartext-logging"
path = "examples/citadel_autosave.rs:81"
why_present = "In `main` of the citadel_autosave example, `println!(\"   ID: {}\", paladin.uuid)` at examples/citadel_autosave.rs:81 follows the builder chain ending `.enable_autosave().build().await?` (lines 75-77) and prints the freshly built Paladin's `uuid` field. The `uuid` field name is what the sensitive-data heuristic keys on."
why_dismissed = "The value is the Paladin entity UUID, printed one line below the Paladin's display name (`paladin.data.name`, line 80) so the reader of the demo can match it to the state file the example later inspects. It is an identifier, not a secret; no other expression is interpolated on line 81."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 81 in the main function of examples/citadel_autosave.rs (an example binary); the tainted expression is paladin.uuid"
compensating_control = "Stdout of a hand-run example, not a log pipeline; the UUID is an opaque v4 value that grants no access, and the example is not part of any release artifact of the paladin crates."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the Paladin identifier type gains a redacted Display that examples should print -- whichever comes first"

[[dismissal]]
alert_number = 33
rule_id = "rust/cleartext-logging"
path = "examples/citadel_autosave.rs:130"
why_present = "The closing `println!` at examples/citadel_autosave.rs:130-131 interpolates `state_dir` and `paladin_id` into \"State saved to: {}/paladin-{}.json\". `paladin_id` is the Paladin's `uuid` (the `uuid` field name is the heuristic's trigger); `state_dir` is a plain path string."
why_dismissed = "`state_dir` is the string literal \"./example-states\" bound at line 36 of the same function, and `paladin_id` is the Paladin entity UUID that `FileCitadel` embeds in the state file name. Neither is a credential: the line only tells the reader of the demo which file the example wrote."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the final println! (lines 129-132) in the main function of examples/citadel_autosave.rs; tainted expression is the Paladin UUID, the other argument is a hard-coded relative path"
compensating_control = "Hand-run example writing to stdout; the printed path is a literal and the UUID is an opaque v4 identifier that is already part of the on-disk file name, so the line discloses nothing a directory listing would not."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the example is changed to print a path that comes from user input -- whichever comes first"

[[dismissal]]
alert_number = 34
rule_id = "rust/cleartext-logging"
path = "examples/citadel_restore.rs:76"
why_present = "In `main` of the citadel_restore example, line 72 binds `let paladin_id = paladin.uuid;` and the `println!` at examples/citadel_restore.rs:76 prints `paladin_id` as \"ID\". The taint is the `uuid` field name carried through the local binding."
why_dismissed = "The printed value is the original Paladin's entity UUID, shown so the demo can compare it with the restored Paladin's UUID later in the same function (lines 141-143). Direct inspection of lines 72-77 confirms only the Paladin name, the UUID and the static word \"enabled\" are printed."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 76 in the main function of examples/citadel_restore.rs; tainted expression is the paladin_id binding of paladin.uuid"
compensating_control = "Stdout of a hand-run example, not a log pipeline; an opaque v4 UUID that authorises nothing, in a binary that is not part of the shipped server."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the example stops printing the original identifier -- whichever comes first"

[[dismissal]]
alert_number = 36
rule_id = "rust/cleartext-logging"
path = "examples/citadel_restore.rs:141"
why_present = "The \"Verifying restored state\" block of examples/citadel_restore.rs prints `paladin_id` (the original Paladin's `uuid`, bound at line 72) at line 141 as \"Original ID\". The heuristic keys on the `uuid` field name."
why_dismissed = "The value is the original Paladin's entity UUID, printed beside the restored one (line 142) and the equality result (line 143) to show the restore preserved identity. It is an identifier, not a secret; lines 140-144 interpolate nothing else sensitive."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 141 in the main function of examples/citadel_restore.rs; tainted expression is paladin_id"
compensating_control = "Stdout of a hand-run example; an opaque v4 UUID that grants no access. Sibling to alerts 34 and 37, which are the same identifier printed at two other lines of this function."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the verification block is reworked to compare identifiers without printing them -- whichever comes first"

[[dismissal]]
alert_number = 37
rule_id = "rust/cleartext-logging"
path = "examples/citadel_restore.rs:142"
why_present = "The same \"Verifying restored state\" block prints `restored_paladin.uuid` at examples/citadel_restore.rs:142 as \"Restored ID\"; `restored_paladin` comes from `PaladinBuilder::...restore_from(paladin_id).await?.build().await?` (lines 126-131). The `uuid` field name is the heuristic's trigger."
why_dismissed = "The printed value is the UUID of the Paladin restored from Citadel state, which by design equals the original identifier printed on line 141. It carries no credential, session token or personal datum; line 143 only compares the two identifiers."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 142 in the main function of examples/citadel_restore.rs; tainted expression is restored_paladin.uuid"
compensating_control = "Stdout of a hand-run example, not a log pipeline; the identifier is an opaque v4 UUID and the example is not compiled into any release artifact of the paladin crates."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the example stops printing the restored identifier -- whichever comes first"

[[dismissal]]
alert_number = 39
rule_id = "rust/cleartext-logging"
path = "crates/paladin-memory/src/citadel/file_citadel.rs:175"
why_present = "`FileCitadel::save_paladin` (the `CitadelPort` impl, fn at line 160) ends with `info!(\"Saved Paladin state: {} to {}\", state.paladin.uuid, path.display())` at crates/paladin-memory/src/citadel/file_citadel.rs:174-178, after the JSON state has been written to `path`. The `uuid` field name is what the sensitive-data heuristic keys on; `path.display()` is the state file's own path."
why_dismissed = "Both interpolated values are identifiers: the Paladin entity UUID (the same one alert 40 concerns) and the path of the state file the Citadel just wrote, which embeds that UUID in its file name. The Paladin state itself (garrison contents, history) is serialised into the file and is never passed to this statement. Direct inspection of lines 167-178 confirms nothing else is interpolated."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single info! statement at the end of FileCitadel::save_paladin; tainted expression is state.paladin.uuid"
compensating_control = "Info-level lifecycle log carrying an opaque v4 UUID and a local file path; the credential-handling review in .github/instructions/security.instructions.md remains the control for any log that could touch a real secret, and nothing in the saved state is interpolated here."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the Citadel gains a redacting identifier type this statement should use -- whichever comes first"

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
alert_number = 41
rule_id = "rust/cleartext-logging"
path = "src/application/services/orchestration/mod.rs:797"
why_present = "`Orchestrator::start_session` (fn at src/application/services/orchestration/mod.rs:794) inserts the caller's `OrchestrationContext` into `active_sessions` and logs `log::info!(\"Started orchestration session: {}\", context.session_id)` at line 797. The heuristic reads the field name `session_id` as a session credential."
why_dismissed = "`OrchestrationContext::session_id` is a `Uuid::new_v4()` minted in `OrchestrationContext::new` (crates/paladin-core/src/platform/container/orchestration_context.rs:28) and documented as the \"unique identifier for the orchestration session\". It is the in-process key of the `active_sessions` map, a correlation identifier for a multi-Paladin run; it is not an authentication session, bearer token or cookie, and it authorises nothing."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single info! statement in Orchestrator::start_session; tainted expression is context.session_id"
compensating_control = "The value is an opaque random UUID with no authority attached, logged at info level so an operator can correlate one orchestration run across log lines."
revisit_condition = "the rust-queries heuristic stops classifying a `session_id` field as a credential, or OrchestrationContext::session_id is repurposed to carry anything with authority -- whichever comes first"

[[dismissal]]
alert_number = 42
rule_id = "rust/cleartext-logging"
path = "src/application/services/orchestration/mod.rs:807"
why_present = "`Orchestrator::end_session` (fn at line 802) removes the session from `active_sessions` and logs `\"Ended orchestration session: {} (duration: {}s)\"` with `session_id` and a computed `duration` at src/application/services/orchestration/mod.rs:806-810. The `session_id` name is the heuristic's trigger."
why_dismissed = "Same orchestration correlation UUID as alert 41, plus `duration`, an integer number of seconds computed as `Utc::now().timestamp() - context.started_at.timestamp()` (line 805). Inspection of the statement shows no other expression: nothing the caller supplied (initiator, environment, metadata) is logged."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single info! statement in the `if let Some(context)` branch of Orchestrator::end_session; tainted expression is the session_id parameter"
compensating_control = "Opaque random UUID plus an integer duration at info level; no authority is attached to the identifier (see alert 41)."
revisit_condition = "as for alert 41 -- the heuristic stops treating `session_id` as a credential, or the session identifier gains authority -- whichever comes first"

[[dismissal]]
alert_number = 43
rule_id = "rust/cleartext-logging"
path = "src/application/services/paladin/paladin_builder.rs:1518"
why_present = "`PaladinBuilder::user_name(...)` is a fluent setter whose name matches the sensitive-data heuristic, so the whole builder value is tainted, as in alerts 45-47. `PaladinBuilder::build` (fn at paladin_builder.rs:1500) binds `agent_name` from the builder (lines 1507-1511) and logs `log::info!(\"Auto-generated system prompt for agent: {}\", agent_name)` at line 1518."
why_dismissed = "`agent_name` is either the string literal \"Agent\" (when `self.data.name` is empty) or `&self.data.name`, the Paladin's own display name; it is never `self.data.user_name`. Direct inspection of lines 1507-1518 confirms the two fields are not conflated. The taint is whole-struct over-approximation from the same-named setter on a different field."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the info! statement in the Ok(generated_prompt) arm of the auto-prompt branch of PaladinBuilder::build; only agent_name is interpolated"
compensating_control = "The Paladin display name is a caller-chosen, non-secret label that Herald output and the prompt service's own log (alert 45) already surface; no credential or user identity flows through `agent_name`."
revisit_condition = "rust-queries gains field-sensitive taint tracking for struct setters (so a `user_name` setter no longer taints sibling fields), or `agent_name` is changed to derive from `user_name` -- whichever comes first"

[[dismissal]]
alert_number = 44
rule_id = "rust/cleartext-logging"
path = "src/application/services/paladin/paladin_builder.rs:1591"
why_present = "Same whole-builder taint as alert 43: `user_name(...)` taints `self`, and in `PaladinBuilder::build` the handoff block (lines 1585-1593) logs `\"Auto-registered handoff tool with {} specialists\"` with `self.specialist_agents.len()` at paladin_builder.rs:1590-1593, a value read off the tainted builder."
why_dismissed = "The interpolated value is `usize` length of the specialist-agent list, a count; the specialist names and prompts are not logged. The taint reaches it only because `self` is tainted as a whole."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the info! statement inside `if self.handoffs_configured` / `if let Some(arsenal)` in PaladinBuilder::build; the only argument is self.specialist_agents.len()"
compensating_control = "An integer count at info level discloses no name, key or user identity; the handoff tool itself is registered in the Arsenal and never printed."
revisit_condition = "rust-queries gains field-sensitive taint tracking for struct setters, or the statement is changed to list specialist names -- whichever comes first"

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

[[dismissal]]
alert_number = 50
rule_id = "rust/cleartext-logging"
path = "src/infrastructure/adapters/scheduling/tokio_cron_adapter.rs:166"
why_present = "Inside `TokioCronSchedulerAdapter::schedule_job` (fn at src/infrastructure/adapters/scheduling/tokio_cron_adapter.rs:152) the cron closure logs `debug!(\"Scheduled job '{}' fired (uuid={})\", label, uuid)` at line 166. `label` is a clone of `spec.label` (line 157) and `uuid` is the job UUID the scheduler passes into the closure (line 163); the `uuid` name is the heuristic's trigger."
why_dismissed = "`JobSpec::label` is documented as the \"human-readable label for the job\" (crates/paladin-ports/src/output/scheduler_port.rs:127) and the uuid is the scheduler's own job identifier; neither is a credential. The statement is debug-level and interpolates only these two values."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the debug! statement inside the async cron closure created in TokioCronSchedulerAdapter::schedule_job; tainted expressions are the job UUID and the caller-chosen label"
compensating_control = "Debug level, so not emitted at the production default; the label is a caller-chosen name for a schedule, and the manual credential-handling review in .github/instructions/security.instructions.md still applies to any caller who puts a secret in a label."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` value as private information, or JobSpec::label gains a documented secret-bearing use -- whichever comes first"

[[dismissal]]
alert_number = 51
rule_id = "rust/cleartext-logging"
path = "src/infrastructure/adapters/scheduling/tokio_cron_adapter.rs:202"
why_present = "The tail of `TokioCronSchedulerAdapter::schedule_job` logs `info!(\"Scheduled job '{}' with id {}\", label_for_log, job_id)` at src/infrastructure/adapters/scheduling/tokio_cron_adapter.rs:202, after the job info is stored in `self.jobs` (lines 197-200). `label_for_log` is `spec.label.clone()` (line 159) and `job_id` is a `JobId`, a newtype over a `Uuid` (scheduler_port.rs:48)."
why_dismissed = "Both values are identifiers: the human-readable job label and the random job id returned to the caller on the next line (`Ok(job_id)`). Inspection of the statement shows nothing else is interpolated; the cron expression and the job metadata are not logged."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single info! statement at the end of TokioCronSchedulerAdapter::schedule_job; tainted expressions are label_for_log and job_id"
compensating_control = "Info-level lifecycle log of two identifiers the caller already holds; the job id authorises nothing and the job metadata is not printed."
revisit_condition = "the rust-queries heuristic stops classifying a UUID-backed `JobId` as private information, or the statement is extended to log job metadata -- whichever comes first"

[[dismissal]]
alert_number = 52
rule_id = "rust/hard-coded-cryptographic-value"
path = "src/core/platform/manager/user_service.rs:719"
why_present = "The rule's heuristic password-parameter sink fires on the literal \"password123\" passed to `service.verify_password(\"password123\", &hash)` at src/core/platform/manager/user_service.rs:719, inside `#[tokio::test] async fn register_user_persists_an_argon2_hash_that_only_verifies_the_original_password` (fn at line 705), itself inside the `#[cfg(test)] mod tests` block opening at line 491. Same sink family as alert 28."
why_dismissed = "Direct inspection confirms this is a test fixture: \"password123\" is the password the `registration(...)` helper at line 611-618 hard-codes for every test user, and line 719 asserts that the Argon2 hash stored at registration verifies against it. It protects no real account, is never logged, and is not derived from any real credential store."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/core/platform/manager/user_service.rs; never compiled into a release binary"
compensating_control = "The literal never leaves the #[cfg(test)] compilation unit: it is not read from configuration and not reachable from any code path a release build includes."
revisit_condition = "the rust-queries heuristic stops flagging password-named parameters fed a test literal, or the test fixtures are moved to a generated placeholder -- whichever comes first"

[[dismissal]]
alert_number = 53
rule_id = "rust/hard-coded-cryptographic-value"
path = "src/core/platform/manager/user_service.rs:724"
why_present = "The same password-parameter heuristic fires on the literal \"a-completely-different-password\" passed to `service.verify_password(...)` at src/core/platform/manager/user_service.rs:723-725, the negative assertion of the test named for alert 52 (`register_user_persists_an_argon2_hash_that_only_verifies_the_original_password`), inside the `#[cfg(test)] mod tests` block opening at line 491."
why_dismissed = "Direct inspection confirms the literal is the deliberately wrong password of the negative case: line 722-727 asserts `!service.verify_password(...)` so the hash must NOT verify against it. It is not a secret and matches no real credential."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/core/platform/manager/user_service.rs; never compiled into a release binary"
compensating_control = "The literal exists only to be rejected by the assertion in this test and never leaves the #[cfg(test)] compilation unit."
revisit_condition = "the rust-queries heuristic stops flagging password-named parameters fed a test literal, or the negative case is rewritten to use a generated value -- whichever comes first"

[[dismissal]]
alert_number = 54
rule_id = "rust/cleartext-transmission"
path = "crates/paladin-content/src/adapters/input/news_api_fetcher.rs:143"
why_present = "`NewsApiFetcher::fetch_content_list` (fn at crates/paladin-content/src/adapters/input/news_api_fetcher.rs:138) builds `url` with `build_news_api_url`, which embeds `self.api_key` as an `apiKey=` query parameter, and sends it with `self.http_client.get(&url)` at line 143. The rule reports a sensitive value (the API key) reaching a network sink it cannot prove is encrypted."
why_dismissed = "The URL's scheme is a string literal: `build_news_api_url` (lines 64-72) formats `\"https://newsapi.org/v2/everything?...\"`, so the request is TLS-protected and nothing is sent in cleartext. The alert as named (cleartext transmission) is therefore a false positive. This row does NOT certify the surrounding credential handling: the key travels in the query string and `reqwest`'s error Display includes the request URL, which is recorded as a separate follow-up in deferred-items.md of Phase 52, not as a claim of safety here."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single blocking GET in NewsApiFetcher::fetch_content_list; the tainted value is the apiKey query parameter of an https://newsapi.org URL (the type is #[doc(hidden)] in paladin-content)"
compensating_control = "The transport is https:// by construction (literal scheme, no caller-supplied base URL); the manual credential-handling review in .github/instructions/security.instructions.md stays the control for the redirect-following and URL-in-error questions this row does not close."
revisit_condition = "the key moves from the query string into a request header and the client's error text is redacted (tracked in Phase 52 deferred-items.md), or the rust-queries cleartext-transmission model recognises literal https:// URLs -- whichever comes first"

[[dismissal]]
alert_number = 60
rule_id = "rust/hard-coded-cryptographic-value"
path = "src/infrastructure/security/encryption.rs:172"
why_present = "`EncryptionService::new` (fn at src/infrastructure/security/encryption.rs:171) declares `let mut key_bytes = [0u8; 32];` at line 172, a zero-initialised array, which the rule reads as a hard-coded cryptographic key. This is production code, outside the `#[cfg(test)]` block that opens at line 321."
why_dismissed = "The zeros are only the buffer's initial state: the very next statement, `OsRng.fill_bytes(&mut key_bytes)` at line 173, overwrites all 32 bytes from the operating-system CSPRNG before `ChaCha20Poly1305::new(&key_bytes.into())` reads them (line 175), and `key_bytes.zeroize()` clears the buffer afterwards. No constant key ever reaches the cipher."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single array initialiser at line 172 of EncryptionService::new in production code of the root paladin-ai crate; the buffer is filled by OsRng on the next line"
compensating_control = "The key is generated by `OsRng` on every call and zeroised after the cipher is built; `EncryptionService::from_key` is the only constructor that accepts caller-supplied key bytes."
revisit_condition = "the rust-queries hard-coded-cryptographic-value model learns that a zero buffer filled by `fill_bytes` before use is not a constant key, or the constructor stops filling the buffer from OsRng -- whichever comes first"

[[dismissal]]
alert_number = 61
rule_id = "rust/hard-coded-cryptographic-value"
path = "src/infrastructure/security/encryption.rs:281"
why_present = "`EncryptionService::encrypt_data` (fn at src/infrastructure/security/encryption.rs:279) declares `let mut nonce_bytes = [0u8; 12];` at line 281, a zero-initialised array the rule reads as a hard-coded nonce/IV. This is production code, outside the `#[cfg(test)]` block that opens at line 321."
why_dismissed = "The zeros are the buffer's initial state only: line 282 calls `OsRng.fill_bytes(&mut nonce_bytes)`, overwriting all 12 bytes with fresh randomness before `Nonce::from(nonce_bytes)` at line 283, so every encryption uses a new random nonce, which is then prepended to the ciphertext (lines 292-294). No constant nonce is ever used."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single array initialiser at line 281 of EncryptionService::encrypt_data in production code of the root paladin-ai crate; the buffer is filled by OsRng on the next line"
compensating_control = "A fresh OsRng nonce per call is the property that matters for ChaCha20-Poly1305, and it holds by construction at lines 281-283; the nonce is not secret and travels with the ciphertext."
revisit_condition = "the rust-queries model learns that a zero buffer filled by `fill_bytes` before use is not a constant, or `encrypt_data` stops filling the nonce from OsRng -- whichever comes first"

[[dismissal]]
alert_number = 62
rule_id = "rust/hard-coded-cryptographic-value"
path = "src/infrastructure/security/encryption.rs:333"
why_present = "`let key = [0u8; 32];` at src/infrastructure/security/encryption.rs:333 in `#[test] fn test_encryption_service_from_key` (fn at line 332), inside the `#[cfg(test)] mod tests` block opening at line 321, is passed to `EncryptionService::from_key(&key)` and read by the rule as a hard-coded cryptographic key."
why_dismissed = "Direct inspection confirms a test fixture: the test only asserts that `from_key` accepts a 32-byte slice (`assert!(service.is_ok())`, line 335). The all-zero array encrypts nothing of value and is never used outside this test."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/infrastructure/security/encryption.rs; never compiled into a release binary"
compensating_control = "The array lives inside one test function in a #[cfg(test)] compilation unit; production keys come from `EncryptionService::new` (OsRng) or from the caller of `from_key`."
revisit_condition = "the rust-queries heuristic stops flagging all-zero test arrays, or the test is changed to build its key from a non-literal source -- whichever comes first"

[[dismissal]]
alert_number = 63
rule_id = "rust/hard-coded-cryptographic-value"
path = "src/infrastructure/security/encryption.rs:340"
why_present = "`let key = [0u8; 16]; // Wrong size` at src/infrastructure/security/encryption.rs:340 in `#[test] fn test_encryption_service_from_invalid_key` (fn at line 339), inside the `#[cfg(test)] mod tests` block opening at line 321, is passed to `EncryptionService::from_key(&key)`."
why_dismissed = "Direct inspection confirms a test fixture for the error path: the 16-byte array is deliberately the wrong length, and the test asserts `from_key` rejects it (the `matches!` at line 342). It is not a usable key and never reaches a cipher."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/infrastructure/security/encryption.rs; never compiled into a release binary"
compensating_control = "The array is rejected by the code under test and lives only inside a #[cfg(test)] compilation unit."
revisit_condition = "the rust-queries heuristic stops flagging all-zero test arrays, or the test is changed to build its wrong-size key from a non-literal source -- whichever comes first"

[[dismissal]]
alert_number = 65
rule_id = "rust/cleartext-logging"
path = "src/config/treasurer.rs:1555"
why_present = "`assert!(err.contains(\"treasurer.allowance\"), \"{err}\")` at src/config/treasurer.rs:1555, in `#[test] fn validate_against_rejects_allowances_without_a_run_store` (fn at line 1549), inside the `#[cfg(test)] mod tests` block opening at line 878, prints `err`, the `String` returned by `AllowanceConfig::validate_against`. The CodeQL annotation for this alert names a `.keys()` expression, and the only `.keys()` calls feeding this error are the loops in `validate_against` (lines 634 and 643)."
why_dismissed = "Direct inspection of `validate_against` (lines 617-652) shows its error strings name configuration paths and entry names only (`treasurer.allowance has entries but run_store.backend is disabled ...`), never an API key value; the map is keyed by key NAME, and this test's inputs are the literals \"k\", \"1d\" and \"5\" (line 1550). The message is printed only if the assertion fails, in a test run."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/config/treasurer.rs; never compiled into a release binary"
compensating_control = "The message is an assertion failure string over literal fixtures inside a #[cfg(test)] compilation unit; the production error text is built from config path names, not key values."
revisit_condition = "the rust-queries heuristic stops treating `.keys()` on an `api_keys` map as credential-bearing, or these assertions are reworded to not print the error -- whichever comes first"

[[dismissal]]
alert_number = 66
rule_id = "rust/cleartext-logging"
path = "src/config/treasurer.rs:1556"
why_present = "`assert!(err.contains(\"run_store.backend\"), \"{err}\")` at src/config/treasurer.rs:1556, the second assertion of the test behind alert 65 (`validate_against_rejects_allowances_without_a_run_store`), prints the same `err` String in its failure message; the report is a distinct sink line of the same flow."
why_dismissed = "Same reasoning as alert 65, inspected on its own line: `err` is the run-store-disabled message (lines 626-632 of `validate_against`), which names `treasurer.allowance` and `run_store.backend` and contains no key value. It is printed only on assertion failure inside the test."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/config/treasurer.rs; never compiled into a release binary"
compensating_control = "Failure-message text over a fixed, value-free error string in a #[cfg(test)] compilation unit."
revisit_condition = "as for alert 65 -- the heuristic stops treating `.keys()` on an `api_keys` map as credential-bearing, or the assertions stop printing the error -- whichever comes first"

[[dismissal]]
alert_number = 67
rule_id = "rust/cleartext-logging"
path = "src/config/treasurer.rs:1566"
why_present = "`assert!(err.contains(\"treasurer.allowance.api_keys.ghost\"), \"{err}\")` at src/config/treasurer.rs:1566, in `#[test] fn validate_against_rejects_unknown_api_keys_and_tenants` (fn at line 1560), inside the `#[cfg(test)] mod tests` block opening at line 878, prints `err` from `validate_against` for an allowance entry keyed \"ghost\" that is absent from the known-key set {\"k\"}."
why_dismissed = "The error is built at lines 636-640 of `validate_against` from the entry NAME (`treasurer.allowance.api_keys.{name} names no key in http.auth.api_keys ...`); the name in this test is the literal \"ghost\" from line 1561, a fixture label, not a secret. The line only prints on assertion failure."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/config/treasurer.rs; never compiled into a release binary"
compensating_control = "The only interpolated name is a literal fixture label inside a #[cfg(test)] compilation unit. In production, `validate_against`'s message names the configuration entry (lines 636-640) and its text contains no key value."
revisit_condition = "the rust-queries heuristic stops treating `.keys()` on an `api_keys` map as credential-bearing, or the test stops printing the error -- whichever comes first"

[[dismissal]]
alert_number = 68
rule_id = "rust/cleartext-logging"
path = "src/config/treasurer.rs:1567"
why_present = "`assert!(err.contains(\"http.auth.api_keys\"), \"{err}\")` at src/config/treasurer.rs:1567, the next assertion in `validate_against_rejects_unknown_api_keys_and_tenants` after alert 67, prints the same `err` String in its failure message."
why_dismissed = "Same message and same inputs as alert 67: `err` names `treasurer.allowance.api_keys.ghost` and the config path `http.auth.api_keys`, and no key value. Reported as its own sink line; the printed text is a configuration path plus the literal fixture label \"ghost\"."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/config/treasurer.rs; never compiled into a release binary"
compensating_control = "Failure-message text over fixture literals inside a #[cfg(test)] compilation unit."
revisit_condition = "as for alert 67 -- the heuristic stops treating `.keys()` on an `api_keys` map as credential-bearing, or the assertions stop printing the error -- whichever comes first"

[[dismissal]]
alert_number = 69
rule_id = "rust/cleartext-logging"
path = "src/config/treasurer.rs:1583"
why_present = "`assert!(err.contains(\"treasurer.allowance.tenants.globex\"), \"{err}\")` at src/config/treasurer.rs:1583, the tail of `validate_against_rejects_unknown_api_keys_and_tenants`, prints `err` from `validate_against` for a tenant entry \"globex\" absent from the known-tenant set {\"acme\"} (lines 1569-1582); the `.keys()` taint source is the tenants loop at line 643."
why_dismissed = "The error is built at lines 645-648 from the tenant id (`treasurer.allowance.tenants.{id} names a tenant ...`); \"globex\" is a literal fixture inserted by the test itself at line 1570. A tenant id is an operator-chosen label, not a credential, and the line prints only if the assertion fails."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in src/config/treasurer.rs; never compiled into a release binary"
compensating_control = "Failure-message text over a literal fixture tenant label inside a #[cfg(test)] compilation unit."
revisit_condition = "the rust-queries heuristic stops treating `.keys()` on a tenants map fed from api-key configuration as credential-bearing, or the assertion stops printing the error -- whichever comes first"

[[dismissal]]
alert_number = 76
rule_id = "rust/cleartext-logging"
path = "examples/citadel_autosave.rs:116"
why_present = "`println!(\"   File: paladin-{}.json\", paladin_id)` at examples/citadel_autosave.rs:116 in `main` runs after `citadel.load_paladin(*paladin_id)` (lines 111-114) confirms the autosaved state exists, and prints the Paladin's `uuid` (the `paladin_id` binding) as part of the state file name. The `uuid` field name is what the heuristic keys on."
why_dismissed = "The interpolated value is the Paladin entity UUID, which `FileCitadel` already uses verbatim in the state file name (`paladin-<uuid>.json`); the line just names the file the demo reads back. Lines 115-118 print nothing else sensitive (status and update time)."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 116 in the main function of examples/citadel_autosave.rs (an example binary); tainted expression is paladin_id"
compensating_control = "Stdout of a hand-run example; an opaque v4 UUID that is already visible in the directory listing of the state directory and authorises nothing."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the example is changed to print a path derived from user input -- whichever comes first"

[[dismissal]]
alert_number = 77
rule_id = "rust/cleartext-logging"
path = "examples/citadel_restore.rs:135"
why_present = "After the Paladin is rebuilt from Citadel state (`PaladinBuilder::new(...).with_citadel(...).restore_from(paladin_id).await?.build().await?`, lines 126-131), `main` of examples/citadel_restore.rs prints `restored_paladin.uuid` at line 135 as the restored Paladin's \"ID\". The `uuid` field name is the heuristic's trigger."
why_dismissed = "The value is the restored Paladin's entity UUID, the same identifier the example restored by (`restore_from(paladin_id)`), so printing it discloses nothing the caller did not already hold. Lines 133-137 print the display name, the UUID and the status enum, no secret."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 135 in the main function of examples/citadel_restore.rs; tainted expression is restored_paladin.uuid"
compensating_control = "Stdout of a hand-run example; an opaque v4 UUID that grants no access, in a binary that is not part of the shipped server."
revisit_condition = "the rust-queries heuristic stops classifying a bare `uuid` field as private information, or the example stops printing the restored identifier -- whichever comes first"

[[dismissal]]
alert_number = 78
rule_id = "rust/cleartext-logging"
path = "examples/paladin_with_config.rs:196"
why_present = "`println!(\"  User: {}\", paladin.data.user_name)` at examples/paladin_with_config.rs:196, in `custom_config_example` (fn at line 166), prints `PaladinData::user_name`, a field whose name matches the rule's private-information heuristic. Unlike the builder alerts 43-47, this sink prints the field the heuristic is actually looking at."
why_dismissed = "The alert is accurate about the data flow, so it is registered as won't fix rather than false positive: `user_name` can hold a person's name in general. In this example it holds the literal \"SeniorDeveloper\" set at line 179 (`.user_name(\"SeniorDeveloper\")`), a demo persona label, and the line prints a configuration summary to the developer's own terminal. The example is deliberately left showing the field so the demo is complete."
dismissed_reason = "won't fix"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the single println! at line 196 of custom_config_example in examples/paladin_with_config.rs (an example binary, not the shipped server); the printed value is a hard-coded literal"
compensating_control = "Stdout of a hand-run example whose only `user_name` value is a compile-time literal; the example is not part of the shipped server binary."
revisit_condition = "the example is changed to read `user_name` from the environment or user input, or the sweep on this row's review date re-triages it -- whichever comes first"

[[dismissal]]
alert_number = 79
rule_id = "rust/cleartext-logging"
path = "src/application/services/paladin/paladin_builder.rs:1558"
why_present = "Same whole-builder taint as alert 43. In `PaladinBuilder::build` the temperature-admission warning `log::warn!(target: \"paladin::sampling\", \"auto-selected temperature {} is not accepted by model '{}' on provider '{}'; ...\")` at paladin_builder.rs:1556-1563 interpolates `optimal_temp`, `self.data.model` and `self.llm_port.get_provider_name()`; the report anchors on the format-string line 1558."
why_dismissed = "The three arguments are an `f32` temperature chosen from the task type, the model identifier string and the provider name. None is a credential or user datum; `self.data.model` is read off the builder that `user_name` tainted as a whole, which is the over-approximation."
dismissed_reason = "false positive"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "the warn! statement guarded by `!self.temperature_admitted(...)` in the auto-temperature branch of PaladinBuilder::build; arguments are optimal_temp, self.data.model and the provider name"
compensating_control = "A sampling-parameter diagnostic (ADR-0061) naming a model and provider at warn level; API keys live in the LLM adapter configuration and are not reachable from these three expressions."
revisit_condition = "rust-queries gains field-sensitive taint tracking for struct setters, or the warning is extended to include anything read from adapter configuration -- whichever comes first"

[[dismissal]]
alert_number = 80
rule_id = "rust/cleartext-logging"
path = "crates/paladin-llm/src/config/layers.rs:700"
why_present = "`other => panic!(\"expected FromEnv, got {other:?}\")` at crates/paladin-llm/src/config/layers.rs:700, the fall-through arm of `#[test] fn placeholder_resolves_from_its_variable` (fn at line 693) inside the `#[cfg(test)] mod tests` block opening at line 589, Debug-prints a `SecretSource` returned by `resolve_secret_placeholder(\"${MOONSHOT_API_KEY}\", &env)`; the rule sees an API-key-derived value reach a panic message."
why_dismissed = "Two facts from direct inspection. First, the arm runs only when the test already failed. Second, `SecretSource`'s hand-written `Debug` (lines 95-108) renders `Literal(<redacted>)` and `FromEnv { var, value: \"<redacted>\" }`, so the `{other:?}` text cannot contain the key. The fixture value is the literal \"sk-from-env\" at line 694 in any case."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in crates/paladin-llm/src/config/layers.rs; never compiled into a release binary"
compensating_control = "`SecretSource` redacts in `Debug` by construction (and a sibling test asserts that, see alert 81); the flagged arm is a failure message over a fixture literal inside a #[cfg(test)] compilation unit."
revisit_condition = "the rust-queries heuristic stops tracing `resolve_secret_placeholder` output into Debug formatting, or `SecretSource`'s redacting Debug impl is removed -- whichever comes first (the second is a security regression this row must not outlive)"

[[dismissal]]
alert_number = 81
rule_id = "rust/cleartext-logging"
path = "crates/paladin-llm/src/config/layers.rs:750"
why_present = "`assert!(!rendered.contains(\"secret-value\"), \"{rendered}\")` at crates/paladin-llm/src/config/layers.rs:750, in `#[test] fn secret_source_and_layered_debug_never_print_a_key` (fn at line 743) inside the `#[cfg(test)] mod tests` block opening at line 589, prints `rendered = format!(\"{source:?}\")` for a `SecretSource` built from `resolve_secret_placeholder(\"${K}\", &env)` and from a literal."
why_dismissed = "This test is the regression guard for the redaction: it asserts the Debug text contains no \"secret-value\" substring, so the message is printed only when the redaction is already broken, and the values involved are the fixtures \"literal-secret-value\" and \"env-secret-value\" (lines 744-746). Nothing outside a test run can reach it."
dismissed_reason = "used in tests"
owner = "DF3NDR"
review_date = "2027-06-30"
scope = "test-only code path -- the #[cfg(test)] mod tests block in crates/paladin-llm/src/config/layers.rs; never compiled into a release binary"
compensating_control = "The assertion exists to prove that `SecretSource` and `Layered` Debug output is redacted; the flagged message prints fixture values only on failure inside a #[cfg(test)] compilation unit."
revisit_condition = "the rust-queries heuristic stops tracing placeholder-resolved values into assertion messages, or the redaction tests move to a form that does not print the rendering -- whichever comes first"
```
<!-- END MACHINE-READABLE REGISTER -->
