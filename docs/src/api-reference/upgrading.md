# Upgrading

This page orients an operator or library consumer for two upgrades: **v0.10.1 to v0.11.0**
(first, below) and **v0.9.x to v0.10.0** (kept further down). It condenses and links; it is not
a second register. The authoritative, exhaustive record of every behavioral change, Rust API
change, schema migration, configuration surface, HTTP surface change and upgrade checklist is
the root
[`MIGRATION.md`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md) file:
section 10 for v0.11.0, section 9 for v0.10.0. Read this page first, then consult `MIGRATION.md`
for the full text of any row named here, including the worked code examples.

## Upgrading from v0.10.1 to v0.11.0

v0.11.0 is the Treasurer release: spend governance (a price table, a cost ledger, per-tenant and
per-key allowances), Cadence rate pacing after a provider 429, provider parity across all nine
adapters, and reasoning effort. Four clean breaks ship with it, with no deprecation cycle:
the legacy Battalion retry, error-strategy and timeout types are replaced by `Aegis`, the base
wrapper `Node<T>` becomes `Entity<T>`, the provider-parity surface changes outright, and
reasoning effort adds its own breaks. The table and lists below name each row by id and link
its section of `MIGRATION.md`; the authoritative text stays there.

### Behavioral changes

Every v0.11.0 change an operator can observe **without** touching their own code, condensed
from [`MIGRATION.md` section 10.1][mig-101] to one line each (M-B-05 onward; M-B-01 to M-B-04
are the v0.10.0 rows further down this page). See that section for the "who is affected"
detail and the worked example.

| ID | Change | Required action |
|---|---|---|
| M-B-05 | Rate pacing (Cadence) is on by default: after a provider 429 each (provider, model) gate holds later calls for an exponential back-off (500 ms up to 30 s). No adapter retries a 429 inside its own loop any more; the first 429 reaches the pacing layer. | None required. Set `treasurer.cadence.enabled: false` to opt out, or tune the `treasurer.cadence` keys. |
| M-B-06 | Battalion timeouts bound each attempt, not the run: `BattalionConfig.aegis.timeout` bounds every Paladin attempt and there is no whole-Battalion clock. A stored `BattalionConfig` or `ConclaveConfig` that still carries `timeout_seconds`, `retry_policy` or `error_strategy` is refused at load. | Convert `timeout_seconds` to `aegis.timeout.run_timeout`, sized as the longest single Paladin call. Wrap `Commander::execute()` in `tokio::time::timeout` for a whole-run bound. |
| M-B-07 | Battalion retry follows the Aegis policy: `aegis.retry.max_attempts` counts total attempts (N retries is `N + 1`), the default `TransientOnly` predicate does not retry `Unknown` failures, and Phalanx and Campaign now retry. | Replace `with_retry_policy` and `with_error_strategy` with `with_aegis(..)`; use `max_attempts: N + 1` and `TransientAndUnknown` to keep retry-everything. |
| M-B-08 | `BattalionResult.node_errors` carries the structured `NodeError` (`node_id`, `attempt`, `transience`, `source`) instead of `{node_name, error}`, and the herald lines change. | Read `node_id` where you read `node_name` and render `source`; to keep the v0.10 text read `message` from the `Paladin`, `Llm` and `Function` arms; update herald-line parsers. |
| M-B-09 | The Conclave retries by `PaladinError::transience()` alone, never message text, with the engine back-off instead of 1, 2, 4, 8, 16 s. | None for the predicate. Set `max_attempts` to the old `retry_attempts + 1` to keep a retry count. |
| M-B-10 | The circuit breaker counts `Transient` failures only; a `Permanent` failure (for example a 401 from a bad key) or an `Unknown` one no longer opens it. | Alert on repeated `Permanent` provider failures instead of relying on the breaker to cut them off. |
| M-B-11 | The Commander's default Maneuver strategy is fail-fast (v0.10 mapped to `ContinueParallel`); it logs one warning per execution if `aegis.on_error` or `aegis.retry` is set and the Maneuver config is derived from `aegis`, since a Maneuver ignores both. | Call `CommanderBuilder::error_strategy(maneuver::ErrorStrategy::ContinueParallel)` explicitly if siblings must finish after one branch fails; it keeps the `aegis`-derived Maneuver timeout, whereas `maneuver_config(..)` replaces the whole config, timeout included. |
| M-B-12 | Persisted entity JSON uses new keys (`data`, `entity`, `entity_id`, `entity_data`); v0.11 reads both spellings and writes only the new ones, so v0.10 cannot read what v0.11 wrote (including a Citadel snapshot with status `Reasoning`). | Back up that state first. Upgrade every reader and queue consumer before any writer. To downgrade, restore the backup. |
| M-B-13 | Every adapter follows the per-model parameter policy: an explicit temperature or stop words the model's profile does not admit fail the build, and OpenAI no longer sends a fabricated `temperature` or `top_p`. | Drop `temperature` from Anthropic agents; keep an OpenAI agent's stop words to four or fewer; set `temperature` explicitly on a direct OpenAI `LlmRequest` if you relied on `0.7`. |
| M-B-14 | Default models, model lists and windows come from a dated registry (for example Anthropic `claude-haiku-5-5`, DeepSeek `deepseek-flash`); an unseeded model keeps working with one warning per model. | Pin the model explicitly (`ANTHROPIC_MODEL`, `DEEPSEEK_MODEL`, and so on) to keep the old default. |
| M-B-15 | `llm.<provider>` blocks now take effect; before, the server parsed and ignored them. | Review every block (`base_url`, `default_temperature`, `${VAR}` keys) before upgrading. |
| M-B-16 | Empty output and refusals are typed: `LlmError::EmptyCompletion` and `LlmError::Refusal` (permanent, never retried) replace an empty `Ok` on the OpenAI, DeepSeek and compat adapters. | Handle both where an empty or refused answer matters; treat `Refusal` as a policy outcome. |
| M-B-17 | An Anthropic refusal is `LlmError::Refusal`; Anthropic stop reasons are named (`FunctionCall`, `Length`); Gemini blocked-content finishes are `ContentFilter` and its empty `STOP` output is `EmptyCompletion`. | Match `FunctionCall`, `Length` and `ContentFilter`; handle `Refusal` and `EmptyCompletion` on Anthropic and Gemini as on the other providers. |
| M-B-18 | The OpenAI adapter sends its output cap as `max_completion_tokens` instead of `max_tokens`. | For a server that accepts only the old key, use the `openai-compatible` preset. |
| M-B-19 | Grok usage counts reasoning tokens in `completion_tokens` and `total_tokens`. | Re-size Grok token budgets, allowances and spend limits. |
| M-B-20 | Vendor usage totals are reconciled on every adapter; a disagreement adds `paladin.usage_unreconciled` to the response metadata. | Re-size budgets, allowances and spend limits; treat `paladin.usage_unreconciled` as a signal that the vendor's reporting changed. |
| M-B-21 | Billed failures, hopped-past fallback bills and failed engine attempts are now charged to the ledger and the allowance. | Re-size allowances, spend limits and budgets against the ledger's new figures; no configuration or code change is needed. |
| M-B-22 | Two billed-failure error changes: on the compat presets an empty or refused body with no `usage` is `EmptyCompletion` or `Refusal` (it was `ProcessingError`), and a no-`choices` body is `EmptyCompletion` carrying the billed usage. | Match `LlmError::EmptyCompletion { .. }` and `LlmError::Refusal { .. }` (or `transience()`); read counts from `error.billed_usage()`. |

### Library consumers

For a consumer of the facade `paladin-ai` (plus `paladin-ports` and `paladin-memory`) moving from
the `v0.10.1` tag. Each item below is one `DR-n` row of the
[`MIGRATION.md` section 10.8][mig-108] "Library consumers" table, grouped by topic; that table
holds the replacement code, and each row cites its section 10.2 register row.

**Battalion and Aegis**

- [DR-1][mig-108]: the Battalion `ErrorStrategy` enum is removed; use `aegis.on_error`.
- [DR-2][mig-108]: the legacy Battalion `NodeError` summary is removed; `node_errors` holds the structured `NodeError`.
- [DR-8][mig-108]: `BattalionConfig` loses `timeout_seconds`, `retry_policy` and `error_strategy` and gains `aegis` / `with_aegis`; a stored document carrying a removed key is refused.
- [DR-9][mig-108]: `BattalionError::Timeout` is removed; a timeout surfaces as a `Node` error with a `Timeout` source, and under Phalanx `Absorb` it is a `node_errors` entry, not an `Err`.
- [DR-10][mig-108]: `PaladinError::LlmError(String)`, `is_retryable()` and `is_terminal()` are removed; use `LlmFailure` and `transience()`.
- [DR-11][mig-108]: `StopReason` gains `CallLimit`, `TokenBudget` and `AllowanceHalted(..)` (no action behind the wildcard arm the enum already requires).

**The `Entity<T>` rename**

- [DR-3][mig-108]: `Node<T>` becomes `Entity<T>` with payload `.data`; no alias under the old path.
- [DR-4][mig-108]: the `PromptItem` and `ContentItem` wrapper field `node` becomes `entity`.

**Provider configuration and construction**

- [DR-12][mig-108]: the nine adapter configs gain the PARITY-03 knob fields and become `#[non_exhaustive]`; build with `new(..)` and the `with_*` builders.
- [DR-13][mig-108]: `LlmProviderConfig` gains `max_tokens`, `context_window` and `models`, makes `api_key` optional and becomes `#[non_exhaustive]`.
- [DR-14][mig-108]: `ModelFallbackConfig` and `AgentRuntimeDeps` gain fields; end a literal with `..Default::default()`.
- [DR-16][mig-108]: `FallbackChainError` gains `EmptyStepModel { index }`; match with a wildcard arm.
- [DR-19][mig-108]: the seven `*Config::new(..)` constructors drop the `base_url` argument (and Anthropic's `max_tokens`); set them with the builders.

**Token usage, cost and spend**

- [DR-5][mig-108]: `PaladinResult` gains `cost`; use `..Default::default()` or `PaladinResult::new(..)`.
- [DR-6][mig-108]: `LlmResponse` gains `cost` and `reasoning`; build with `LlmResponse::new(..)`.
- [DR-15][mig-108]: `PaladinResult` gains `run_metadata`; `ChunkMetadata` and `StreamingResponse` gain metadata.
- [DR-17][mig-108]: `LlmError::EmptyCompletion` and `Refusal` become `#[non_exhaustive]` struct variants carrying `billed`; read the bill with `billed_usage()`.
- [DR-18][mig-108]: reading a failed execution's spend through the `ATTEMPT_SPEND` scope (additive, no compile break).

**Reasoning effort**

- [DR-20][mig-108]: an explicit effort on a model nobody measured is refused before HTTP.
- [DR-21][mig-108]: a provider-level default effort is judged against each agent's model at build; a decorator over an `LlmPort` must forward `check_reasoning`.
- [DR-22][mig-108]: `CompatRequestParameters` gains `reasoning` and `all()` becomes `all(reasoning)`.
- [DR-23][mig-108]: `PaladinData` and `AgentDefinition` gain `reasoning_effort` and `reasoning_fallback`.
- [DR-24][mig-108]: `TraceEvent::NodeFinished` gains `reasoning_requested` and `reasoning_effective`.
- [DR-25][mig-108]: Paladin node-cache keys append the effort only when it is set.
- [DR-26][mig-108]: a consumer that sets no effort meets nothing new.

**Rate pacing and decorators**

- [DR-7][mig-108]: `LlmError::RateLimitExceeded` becomes a `#[non_exhaustive]` struct variant `{ retry_after, hints }`; build with `LlmError::rate_limited(..)`. A directly built adapter has no 429 pacing unless Cadence is composed, and an `LlmPort` decorator must forward `model_profile` and `check_reasoning` (both in the "Library consumers" subsection of section 10.8).

### Operator checklist

One ordered checklist for upgrading a v0.10.1 deployment, mirrored from
[`MIGRATION.md` section 10.8][mig-108] ("Operators: v0.10.1 to v0.11.0"), one line per step.

1. **Back up state.** Snapshot the run store, the waypoint store, the Garrison SQLite database if used and any Citadel state files; the new migrations are not reversed, and v0.10 cannot read entity JSON or Citadel snapshots that v0.11.0 wrote.
2. **Apply migrations by starting the new binary.** Migrations `007` to `014` run at adapter construction via `sqlx::migrate!`; there is no separate migrate command.
3. **Update config.** Add a `tenant:` line to every `http.auth.api_keys` entry and set `http.auth.bearer_token.tenant` (the server refuses to boot otherwise); convert removed Battalion keys (M-B-06); review each `llm.<provider>` block (M-B-15); decide on Cadence (M-B-05); `treasurer` is optional.
4. **Roll out the allowance webhook only after every replica runs v0.11.0.** An older replica cannot decode the `allowance_warning` and `allowance_halted` deliveries and stalls run-webhook delivery.
5. **Deploy.** Roll out the new binary and manifests with the step 3 config already in place.
6. **Verify.** Run `paladin-cli setup-check --verbose`, send one request to an agent route with a key that carries a `tenant:` line, and confirm another tenant's run answers `404`.

[mig-101]: https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#101-behavioral-changes-user-visible-without-code-changes
[mig-108]: https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#108-upgrade-checklist

## Upgrading from v0.9.x to v0.10.0

For an operator or library consumer moving a deployment or dependency from **v0.9.x to
v0.10.0**. The authoritative record is section 9 of `MIGRATION.md`
([`#91-...`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#91-behavioral-changes-user-visible-without-code-changes)
to
[`#98-...`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#98-upgrade-checklist)):
the complete §9.1–§9.8 detail, including worked code examples for each behavioral change.

### Behavioral changes

Every v0.10.0 change an operator can observe **without** touching their own code, condensed
from `MIGRATION.md` §9.1 to one line each. See that section for the full "who is affected" /
"required user action" detail and worked examples.

| ID | Change | Required action |
|---|---|---|
| M-B-01 | `EdgeCondition::Custom(name)` no longer silently evaluates to `true` when no evaluator is registered (BUG-01 fix) — an unregistered custom edge now fails graph validation before any node executes. | Register an evaluator for each custom condition name, or replace the condition with `Contains`/`Regex`/`Always`. |
| M-B-02 | Graceful shutdown: on SIGTERM/SIGINT the process now waits up to `shutdown_grace` (default 30s) for in-flight engine runs to halt before exiting. | Set `terminationGracePeriodSeconds` to at least `60` (twice the default grace) in every Deployment manifest. |
| M-B-03 | No behavioral change to the default policy — `tool_error_mode` names the v0.9 behavior (`FeedToModel`) rather than introducing a new one, and the fed-back error text is now redacted-then-bounded before the model sees it. | None required to keep today's behavior. Set `tool_error_mode = FailRun` to opt into failing the run on a tool error instead. |
| M-B-04 | Any graph executed through the new `WarEngine` writes one `Waypoint` (a full `Battlefield` snapshot) after every superstep by default. Legacy `Formation`/`Phalanx`/`Campaign`/`Commander` execution paths are completely unaffected — they write no Waypoints. | Only applies if you adopt the new `WarEngine`/`WarGraph` APIs: choose a `WaypointPort` backend and review `WaypointDurability` and `WaypointRetentionConfig`. |

### Upgrade checklist

One ordered, copy-pasteable checklist for upgrading a v0.9.0 deployment to v0.10.0, mirrored
from `MIGRATION.md` §9.8.

1. **Back up state.** Snapshot every state directory and database this deployment uses: the
   waypoint store (`SqliteWaypointStore`/`PostgresWaypointStore`'s backing file or database),
   the run store (`RunStoreConfig`'s SQLite file or PostgreSQL database), the Garrison SQLite
   database if used, and any Citadel state files. There is no destructive migration to reverse
   a bad upgrade against; a restored backup plus the v0.9.0 binary is the rollback path.
2. **Apply migrations — by starting the new binary, not a separate command.** Every migration
   this program added runs automatically at adapter construction via `sqlx::migrate!`; there is
   no `sqlx migrate run` step to invoke by hand. Start the new `paladin-server` binary once
   against the restored backup and confirm it comes up cleanly. The same automatic-migration
   mechanism applies to the PostgreSQL-backed adapters — no separate manual migration step is
   needed there either.
3. **Update config — nothing is required.** Every new v0.10 config surface defaults to today's
   behavior, proven by the `v0_9_config_boot` integration test: a v0.9 configuration file boots
   this binary with every new subsystem inert. No `config.yml` edit is required to preserve v0.9
   behavior; add a section only when actually adopting a new capability.
4. **Raise `terminationGracePeriodSeconds`.** Set it to at least `60` in every Deployment
   manifest before rolling out this upgrade (M-B-02). The shipped manifests
   (`k8s/deployment.yaml`, `k8s/server/deployment.yaml`, `k8s/server/worker-deployment.yaml`)
   already carry `terminationGracePeriodSeconds: 60`; a forked or hand-written manifest needs
   the same change.
5. **Register a custom evaluator for every `EdgeCondition::Custom` name in use.** M-B-01's fix
   makes an unregistered custom edge condition a validation failure, not a silent always-true.
   Register one via `CampaignExecutionService::with_evaluator("name", Arc::new(evaluator))` on
   the legacy execution path, or `WarEngine::with_edge_evaluator("name", Arc::new(evaluator))`
   on the `WarEngine` path, before calling `execute`/`start`.
6. **Deploy.** Roll out the new binary and manifests with the grace period and evaluator
   registrations from steps 4-5 already in place.
7. **Verify.** Run `paladin-cli setup-check --verbose` for an environment/toolchain/provider/
   service connectivity check; if the deployment uses the Maneuver flow DSL, run `paladin-cli
   maneuver validate` against its flow configuration; and run `paladin-cli eval run <glob>`
   against a representative scenario glob for a behavioral post-deploy check. `GraphCommands`
   exposes exactly one subcommand, `export` (`paladin-cli graph export --format mermaid|dot`),
   for inspecting a graph's structure — there is no separate command for a runtime probe.

### Token usage carriers

Every place a Paladin run's token usage is reported now carries the full prompt/completion split
plus optional cache-read/cache-write/reasoning sub-counts, instead of a single bare count
(ACCT-01/ACCT-02/ACCT-03). `PaladinResult.usage`, `NodeExecutionRecord.usage`,
`TraceEvent::NodeFinished.usage`, `TraceEvent::RunFinished.usage`, `StreamingResponse.usage`,
`ChunkMetadata.usage`, and the HTTP `ExecuteResponse.usage` all replace their former
`token_count`/`total_tokens` field with a `TokenUsage`. `TokenUsage::from_total` — the
total-only constructor — is deleted outright with no deprecated replacement; construct a
`TokenUsage` via `TokenUsage::new(prompt, completion)` plus the `with_cache_read`/
`with_cache_write`/`with_reasoning` builders instead. Two under-reports are also corrected: the
battalion per-Paladin split was previously zeroed, and Anthropic's `prompt_tokens` previously
excluded cached input. See [`MIGRATION.md` §9.2](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#92-rust-api-changes-compile-affecting-the-x-10-register)
for the full per-type register and the `CHANGELOG.md` `[0.10.0]` entry for the corrected figures.

### Token primitives

Two duplications in the token-counting/window-resolution primitives are collapsed to one each in
v0.10.0 (PRIM-01…PRIM-04). `TokenCounterPort` gained `fn is_exact(&self) -> bool { false }` — the
counting port now declares its own exactness, so `Commissary::new` and `Commissary::from_port`
no longer take a caller-supplied `is_exact_counter: bool` argument; `Commissary` reads
`Stockpile.exact_tally` live from the injected counter's `is_exact()` instead. The legacy
fallible `garrison::TokenCounter` trait and its `TokenCounterFactory` are removed outright, with
no deprecated replacement — `TiktokenCounter` survives as the sole implementor of
`TokenCounterPort`, which is now the only counting contract in the workspace. Separately, both
context-window precedence walks (`Commissary::new`'s inline fallback guard and
`HistoryTrimmer::resolve_limit`) now call the same shared `paladin_llm::window::resolve_context_window`
function instead of each maintaining its own. See [`MIGRATION.md` §9.2](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#92-rust-api-changes-compile-affecting-the-x-10-register)
for the full per-type register.

Separately, in v0.10.0 (Phase 33, COMM-01…03), `RagRetrievalService::retrieve_context`
and `retrieve_context_with_timeout` change their return type to a result struct
carrying the Commissary's shed record, and `format_for_prompt` changes its parameter
to that struct; a new `with_token_counter` builder mirrors
`PaladinExecutionService::with_token_counter`. Read `.memories` and `.shed` off the
returned result rather than the old `Vec`. See the `paladin-memory | RagRetrievalService`
and `paladin-memory | retrieve_context_with_timeout` rows in
[`MIGRATION.md` §9.2](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#92-rust-api-changes-compile-affecting-the-x-10-register).

## Full migration record

For every behavioral change's worked examples, the complete Rust API change register, schema
migrations, configuration and environment variable reference, and the HTTP API compatibility
notes, see the root
[`MIGRATION.md`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md) file. It has
two parts: section 10, "Upgrading from v0.10.1 to v0.11.0"
([`#10-upgrading-from-v0101-to-v0110`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#10-upgrading-from-v0101-to-v0110)),
and section 9, "Upgrading from v0.9.0 to v0.10.0"
([`#9-upgrading-from-v090-to-v0100`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#9-upgrading-from-v090-to-v0100)).
