# Phase 56 Dogfood Evidence

Plan 56-04 re-checks the four pre-seeded findings (DF-01 to DF-04) on HEAD `6212e3ecf` (full SHA
`6212e3ecf95f2db612f86a525b85d60f6fa0033c`) on 2026-11-02 (UTC) before any closing disposition is written
(D-00e, D-02, P4). Each row records the command, its exit code and its result line. Every named test was
confirmed as `<name> ... ok` in the captured output, so no row rests on a filter that matched nothing.

## Prove-then-lock (D-00e): DF-01…DF-04

| DF | owner (requirement, phase) | command | exit | result line | HEAD |
|----|----------------------------|---------|------|-------------|------|
| DF-01 | FIRST-01 (Phase 54) | `cargo test -p paladin-ai --lib --features web-server -- validate_refuses_an_unexpanded_placeholder_key validate_refuses_a_placeholder_key_while_auth_is_disabled validate_names_a_later_entry_whose_key_is_a_placeholder validate_reports_a_placeholder_key_before_tenant_and_duplicate_checks yaml_env_placeholder_is_not_expanded` (run together with the four DF-04 root-lib tests below; 9 tests in one run) | 0 | `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1486 filtered out` (the five DF-01 names each `... ok`) | 6212e3ecf |
| DF-01 | FIRST-01 (Phase 54) | `cargo test -p paladin-ai --bin paladin-server --features web-server -- build_auth_config_refuses_an_unexpanded_placeholder_key` | 0 | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out` | 6212e3ecf |
| DF-01 | FIRST-01 (Phase 54) | `grep -nE '(^\s*\|[{,]\s*\|-\s+)key:\s*"?\$\{' config.example.yml` | 1 | no output: the example ships no `${...}` key | 6212e3ecf |
| DF-01 | FIRST-01 (Phase 54) | `test ! -e k8s/server/configmap.yaml` | 0 | file absent (the ConfigMap that claimed load-time expansion was retired) | 6212e3ecf |
| DF-02 | FIRST-02, FIRST-03 (Phase 54) | `make check-version-strings` | 0 | `OK: 128 file(s) scanned, every copyable version string equals '0.11.0'` | 6212e3ecf |
| DF-02 | FIRST-02, FIRST-03 (Phase 54) | `./tests/scripts/check-version-strings_test.sh` | 0 | `37 assertion(s) passed.` | 6212e3ecf |
| DF-02 | FIRST-02 (Phase 54) | `grep -n 'Current version' README.md` | 0 | `185:Current version: **0.11.0**.` | 6212e3ecf |
| DF-03 | FIXES-01 (Phase 55) | `make check-no-stub-panics` | 0 | `OK: scanned 624 file(s) (19 test-only), 0 finding(s)` | 6212e3ecf |
| DF-03 | FIXES-01 (Phase 55) | `bash tests/scripts/check-no-stub-panics_test.sh` | 0 | `88 assertion(s) passed.` | 6212e3ecf |
| DF-03 | FIXES-01 (Phase 55) | `test ! -e crates/paladin-content/src/services/content_filtering_service.rs` | 0 | file absent | 6212e3ecf |
| DF-04 | TENANT-04, TENANT-05 (Phase 53) | same root-lib run as DF-01 (`cargo test -p paladin-ai --lib --features web-server -- …`), names `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404 cross_tenant_fork_is_the_unknown_thread_404 cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag cancel_of_another_tenants_run_is_not_found_and_writes_nothing` | 0 | `test result: ok. 9 passed; 0 failed` (the four DF-04 names each `... ok`) | 6212e3ecf |
| DF-04 | TENANT-03, TENANT-07 (Phase 53) | `cargo test -p paladin-web --lib -- every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404 job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass` | 0 | `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 327 filtered out` | 6212e3ecf |
| DF-04 | TENANT-03 (Phase 53) | `cargo test -p paladin-ai-core --lib -- permits_thread_admits_all_and_only_the_latest_runs_tenant` | 0 | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 750 filtered out` | 6212e3ecf |

Notes per entry:

- DF-01: the process-level boot refusal and the watched-red record are in
  `.planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md` (section "Boot refusal (SC1)" and the
  "Prove-then-lock (D-00f)" table). `yaml_env_placeholder_is_not_expanded` still pins that the loader does no
  `${VAR}` expansion, which is why the remedy is a boot-time refusal, the second of the two remedies the entry
  offered.
- DF-02: the gate is the real fix the entry asked for ("a gate is the real fix"); it is green on HEAD and its own
  test script passes. Historical `**Since:** v0.10.0` statements are exempt by design (54-04).
- DF-03: the file that held the `todo!()` constructors was deleted in Phase 55, which meets the entry's remedy
  ("remove or return `Err`"); the stub-panic gate holds the line with 0 findings across 624 scanned files.
- DF-04: the isolation evidence and watched-red record are in
  `.planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md` and `53-VERIFICATION.md`; the tests above
  are the same tests re-run on today's tree.

Disk: scoped `-p <pkg>` builds only. Free space on `/` was 2993M before the first cargo command and 2402M after
the last; no cleanup or retry was needed, and no run failed.

## Lead L3

Plan 56-05 (DOGF-01, DOGF-06, D-00e). Lead L3: the `paladin` binary logs the whole `Settings` with `{:?}`, and
`ApiKeyConfig` derives `Debug`, so `http.auth.api_keys[].key` would print in plaintext. Plan-start SHA
`1203a0dfd`. Every credential below is a fake; each is shown as `<fake-key>` and the real probe literals appear
nowhere in this file (T-56-19).

### Provenance

| check | command | result |
|-------|---------|--------|
| the line ships in 0.11.0 | `git show v0.11.0:src/main.rs \| grep -n 'Loaded configuration'` | `35:    info!("Loaded configuration: {:?}", config);` |
| same call on HEAD | `grep -n 'Loaded configuration' src/main.rs` | `35:    info!("Loaded configuration: {:?}", config);` (identical) |
| `ApiKeyConfig` at 0.11.0 | `git show v0.11.0:src/config/agents.rs \| grep -n 'pub struct ApiKeyConfig' -B2` | `#[derive(Debug, Clone, Serialize, Deserialize)]` directly above the struct: no redaction at the tag either |
| log filter | `src/main.rs:24` | `env_logger::Builder::from_env(Env::default().default_filter_or("debug"))`: with `RUST_LOG` unset the binary logs at `debug`, so an `info!` line is emitted |

### Probe

Build: `cargo build -p paladin-ai --bin paladin --features cli` (free space on `/` 5860M before, 4899M after; no
cleanup needed). Scratch directory from `mktemp -d` outside the repository, deleted at the end.

`l3.yml` is `config.example.yml` with the single line `    api_keys: []` replaced by one entry whose `key` is a
fake literal, `name: "l3-probe"`, `role: "user"`, `tenant: "Acme-Prod"`.

```
$ env -u RUST_LOG OPENAI_API_KEY=<fake-key> timeout 20 <repo>/target/debug/paladin --config ./l3.yml > l3.log 2>&1
exit=1
$ grep -c '<fake-key literal for api_keys[0].key>' l3.log
1
```

`exit=1` is the example's default SQLite path not being openable from the scratch directory
(`Error: ConnectionError("error returned from database: (code: 14) unable to open database file")`, logged after
`Starting services...`). It is unrelated to the finding: the leaking line is the FIRST line of the log, written
before `setup_and_run` ran. Count 1 is a reproduction.

The `Loaded configuration` line (first of 6 log lines, 6657 characters; truncated to the relevant stretch, key
replaced):

```
[2026-11-02T13:58:39Z INFO  paladin] Loaded configuration: Settings { llm_type: "openai", llm_url: "https://api.openai.com/v1", llm_api_key: "", server: ServerConfig { host: "0.0.0.0", port: 8080 }, ...
 ... rate_limit: RateLimitConfig { enabled: false, per_second: 10, burst: 20 }, auth: AuthConfig { enabled: true, api_keys: [ApiKeyConfig { key: "<fake-key>", name: "l3-probe", role: User, tenant: "Acme-Prod" }], bearer_token: BearerTokenAuthConfig { enabled: false, tenant: None } }, docs: DocsConfig { enabled: true } }), agent_runtime: AgentRuntimeConfig { model_call_limit: ...
```

In the same line the provider keys ARE redacted (`LlmProviderConfig { api_key: <redacted>, ... }`): the
redaction discipline exists for providers and was not applied to the sibling auth type.

### Probe 2: other unredacted types in the same line

`l3b.yml` is `l3.yml` plus a `queue:` block with `redis_password`, a `file_storage:` block with `minio_access_key`
and `minio_secret_key`, and a non-empty top-level `llm_api_key`, each a distinct fake. Same command, `exit=1`
(same unrelated SQLite error). Counts of each distinct fake literal in `l3b.log`:

| field | occurrences in log |
|-------|--------------------|
| `http.auth.api_keys[0].key` | 1 |
| `queue.redis_password` | 1 |
| `file_storage.minio_secret_key` | 1 |
| `file_storage.minio_access_key` | 1 |
| top-level `llm_api_key` | 1 |

Five distinct credentials land in one log line.

### Audit: credential-shaped fields reachable from `Settings`

`Settings` itself is `#[derive(Debug, Serialize, Deserialize, Clone)]` (`src/config/settings.rs:29`) and holds a
plain `llm_api_key: String` (`:33`), so a derived `Debug` of `Settings` prints it. Found with
`grep -rnEi "^\s*pub\s+\w*(password|secret|token|key|credential|auth|dsn|endpoint|url|uri)\w*\s*:" src/config
crates/paladin-llm/src/config` plus a read of every type the `Settings` fields reach.

| type | file | credential-shaped field | `Debug` | redacts? |
|------|------|------------------------|---------|----------|
| `Settings` | `src/config/settings.rs:33` | `llm_api_key: String` | derived | NO (empty in the shipped example, but any operator who sets it prints it) |
| `ApiKeyConfig` | `src/config/agents.rs:94` | `key: String` (the literal secret) | derived | NO (reproduced, probe 1) |
| `AuthConfig` | `src/config/agents.rs:132` | `api_keys: Vec<ApiKeyConfig>` | derived | NO (delegates to `ApiKeyConfig`) |
| `BearerTokenAuthConfig` | `src/config/agents.rs:111` | none (`enabled`, `tenant` only; the token is server-issued) | derived | n/a, no secret held |
| `WebHttpConfig` | `src/config/agents.rs:313` | holds `auth: AuthConfig` | derived | NO (delegates) |
| `QueueConfig` | `src/config/queue.rs:11` | `redis_password: Option<String>` | derived | NO (reproduced, probe 2) |
| `FileStorageConfig` | `src/config/file_storage.rs:10` | `minio_access_key`, `minio_secret_key: String` | derived | NO (reproduced, probe 2) |
| `EmailAdapterConfig` | `crates/paladin-notifications/src/email_notification_adapter.rs:49` | `password: String` (SMTP) | derived | NO (reachable through `NotificationConfig`, feature `notifications`; not in the `cli` build probed, from source) |
| `QdrantSanctumConfig` | `crates/paladin-memory/src/config/sanctum.rs:24` | `url: String` (may embed credentials) | derived | NO (a URL; credential only if the operator embeds one; source only) |
| `SourceConfig` | `src/config/web_server.rs:12` | `url: String` (content source URL) | derived | NO (URL; lower risk, source only) |
| `MCPServerConfig` | `src/config/arsenal.rs:19` | `endpoint: Option<String>`; `auth_token_env` is an env-var NAME and `skip_serializing` | derived | endpoint NO (URL); the token itself is never in config |
| `LlmProviderConfig` | `crates/paladin-llm/src/config/llm.rs:41` | `api_key: String` | manual (`RedactedKey`) | YES (observed `api_key: <redacted>`) |
| `OtelConfig` | `src/config/trace.rs:167` | `headers` values | manual | YES (keys only) |
| `AllowanceWebhookConfig` | `src/config/treasurer.rs:460` | `secret: Option<String>` | manual | YES (`[redacted]`) |
| `NodeCacheConfig` | `src/config/node_cache.rs:96` | `redis_password: Option<String>` | manual | YES |

Hand-written redacting `Debug` impls under `src/config/` and `crates/paladin-llm/src/` are exactly: `OtelConfig`,
`AllowanceWebhookConfig`, `NodeCacheConfig`, `LlmProviderConfig` (plus the per-adapter configs `OpenAIConfig`,
`AnthropicConfig`, ... which are not reachable from `Settings`). No credential-bearing type on the auth, queue
or file-storage paths has one.

### Whole-config `{:?}` call sites in `src/` outside tests

`grep -rnE` for `{:?}` / `{:#?}` / `{config:?}` / `?config` / `dbg!(` applied to a config or settings value, with
the matches that sit in `#[cfg(test)]` code excluded (those are the redaction tests themselves):

| site | content | verdict |
|------|---------|---------|
| `src/main.rs:35` | `info!("Loaded configuration: {:?}", config);` | the defect: whole `Settings`, at the default `debug` filter |
| `src/main.rs:30` | `eprintln!("Failed to load configuration: {:?}", e);` | prints the load ERROR `e`, not a config; not a whole-config format |
| `src/bin/paladin-server.rs:220` | `format!("{:?}", run_configs.run_store.backend)` | a backend label, not a credential-bearing type |

No other non-test site formats a `Settings` or a whole `*Config`. `paladin-server` does NOT log its `Settings`
(its sibling-binary exposure is nil); the exposure is the `paladin` binary only.

### Classification

Reproduced. Logged as one finding (one root cause: the `Settings` `Debug` chain is derived over credential-bearing
types, and `src/main.rs:35` renders it). See the findings log entry cited in the plan 56-05 SUMMARY.

## Lead L6

Plan 56-05 (DOGF-01, DOGF-06, D-00e). Lead L6: `paladin-server` never calls `Settings::validate()`, so a `trace`
value its own type rejects boots. HEAD `4ee454557`, probed 2026-11-02. Fake provider key only.

### Source-level evidence

`git grep -nE 'Settings::validate|settings\.validate\(\)' -- src crates` (non-test hits only):

```
src/config/agent_runtime.rs:1785:    /// `AgentRuntimeConfig::default()` and `Settings::validate()`
src/config/trace.rs:102:    /// uses (X-03 -- `Settings::validate()`'s own signature must not
src/config/trace.rs:107:    /// `Settings::validate()`) don't have to match on
src/config/trace.rs:343:    /// Behavior 3: `Settings::validate()` rejects `trace.channel_capacity ==
```

Every other hit is a `#[test]` (`agent_runtime.rs:1800`, `treasurer.rs:935`). The definition is
`src/config/settings.rs:109`; it validates `agent_runtime`, `trace`, `web_server` and the treasurer.
`src/bin/paladin-server.rs` `run()` validates the treasurer, `RunStoreConfig`, `RunWorkerConfig` and the other
Platform API env configs one by one, and `agent_runtime` inside the registry build, but not `trace` or
`web_server`. The same holds at tag `v0.11.0` (`git grep -nE 'settings\.validate\(\)|Settings::validate' v0.11.0
-- src crates` shows only doc comments and tests). `settings.trace` is consumed at boot:
`src/infrastructure/web/run_api_wiring.rs:1277` (`.with_trace_config(settings.trace.clone())`), then
`src/application/services/run/worker.rs:1336` (`with_trace_capacity(self.trace_config.channel_capacity)`).

### Build and probe

Build: `cargo build -p paladin-ai --bin paladin-server --features web-server` (free space 4899M before, 4068M
after; no cleanup needed).

`l6.yml` is `config.example.yml` with four edits (`diff` against the example): `server.host` `127.0.0.1`,
`server.port` a free port from python3 `socket`, `http.auth.enabled: false`, `trace.channel_capacity: 0`.
`TraceConfig::validate_typed` rejects the last (`TraceConfigError::ZeroChannelCapacity`, "trace.channel_capacity
must be greater than 0").

Attempt 1 (as the plan's command, run store only):

```
$ env -u RUST_LOG OPENAI_API_KEY=<fake-key> APP_RUN_STORE_BACKEND=sqlite APP_RUN_STORE_PATH="sqlite://$S/runs.db" PALADIN_CONFIG=./l6.yml timeout 20 target/debug/paladin-server
exit=1
ERROR paladin_server: paladin-server failed to start: run_store.backend is configured but no waypoint store is wired: set waypoint_store.backend
```

That refusal is unrelated to `trace` (it names `waypoint_store.backend`). Per the plan, ONE retry changing only keys
unrelated to `trace`: add `APP_WAYPOINT_STORE_BACKEND=sqlite` and
`APP_WAYPOINT_STORE_SQLITE_PATH=sqlite://$S/wp.db`.

Attempt 2:

```
$ env -u RUST_LOG OPENAI_API_KEY=<fake-key> APP_RUN_STORE_BACKEND=sqlite APP_RUN_STORE_PATH="sqlite://$S/runs.db" APP_WAYPOINT_STORE_BACKEND=sqlite APP_WAYPOINT_STORE_SQLITE_PATH="sqlite://$S/wp.db" PALADIN_CONFIG=./l6.yml timeout 20 target/debug/paladin-server
exit=124   (killed by `timeout` after 20s: the server was up and serving; it then logged a clean SIGTERM shutdown)
INFO paladin_server: paladin-server listening on http://127.0.0.1:44907 — serving 2 agent(s): ["researcher", "summarizer"]
$ curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:44907/health
200
```

`grep -c 'panicked at'` on the log: 0.

### Consequence of the unvalidated value

- `channel_capacity: 0`: `TraceDispatcher::with_capacity` clamps (`capacity.max(1)`,
  `crates/paladin-battalion/src/engine/hooks.rs:224`), so the first run does not panic or hang; it silently runs
  with a queue of 1 and drops trace events at the first overflow. Harmless to correctness, but the config says 0
  and the system does something else without telling the operator.
- A stronger instance of the same root cause, probed once more (`l6c.yml`: example with `trace.otel.enabled:
  true`, `channel_capacity` back at 1024, same env): `Settings::validate()` returns
  `TraceConfigError::FeatureNotCompiled { feature: "otel" }` for this build ("never silently no-ops"), yet the
  server booted (`listening on` count 1, `exit=124` from `timeout`) with no OTLP line in the log and no
  refusal. An operator who enables OTLP export on a build without the `otel` feature gets no export and no error.
- `trace.value_cap_bytes: 0` and the `web_server` section are covered by the same unused `validate()`.

### Classification

Reproduced (listens): an invalid value is accepted at boot. Not a crash. Logged as one finding, S2.

## Lead L4

Plan 56-05 (DOGF-01, DOGF-06, D-00e). Lead L4: `web_server.dev_ui.mermaid_url` is documented as live config but no
shipped binary mounts the inspector.

### Every `create_dev_ui_router` hit

`git grep -n 'create_dev_ui_router' -- src crates examples tests`:

| hit | class |
|-----|-------|
| `crates/paladin-web/src/app.rs:87` | definition |
| `crates/paladin-web/src/dev_ui_controller.rs:21,129` | doc comments |
| `crates/paladin-web/src/dev_ui_controller.rs:197,359,504,523,541` | tests (`#[cfg(test)]`) |
| `examples/platform_api_client.rs:58,320` | example (builds `DevUiState::new("https://cdn.jsdelivr.net/...")` with a hard-coded URL; does not read `Settings`) |

No hit in `src/bin/`, `src/infrastructure/` or `src/application/`: neither `paladin-server` nor `paladin` mounts it.

### Every `dev_ui` / `mermaid_url` reader outside `src/config/`

`git grep -nE 'dev_ui|mermaid_url' -- src/bin src/infrastructure src/application examples crates/paladin-web/src`:
no hit in `src/bin`, `src/infrastructure` or `src/application`. Hits in `crates/paladin-web/src` are the
definition (`DevUiState`, `dev_ui_inspector_page`), the template doc comment (`dev_ui/inspector.html:13`), doc
comments and tests. `git grep -nE 'web_server|get_web_server|WebServerConfig' -- src ':!src/config' crates
examples` finds no reader of `Settings.web_server` anywhere: the parsed value is never passed to `DevUiState`.

### Boot check

Against the L6 binary booted from the example with auth disabled (attempt 2 above, port 44907):

```
$ curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:44907/v1/dev-ui/threads/probe
404
```

`/health` returned 200 on the same server, so the 404 is a missing route, not a down server (a mounted but
unwired inspector answers 501 per `dev_ui_without_a_wired_port_is_501`).

### Classification

Reproduced: no shipped binary mounts the inspector and nothing reads `web_server.dev_ui`. The example's comment
described the section as "currently just the developer inspector UI" with no note that it is inert. Logged S3;
the contained fix is a comment correction in `config.example.yml` (comment lines only, D-05). Wiring a route is
`out` (D-00d). Tests after the edit: `cargo test -p paladin-ai --test config_example_roundtrip` 14 passed, 0
failed; `cargo test -p paladin-ai --lib --features web-server -- example` 6 passed, 0 failed.

## Close gate

Drafted by plan 56-10 task 1 on 2026-11-03 (plan-start SHA `ed5135a4eac8b74781e3cefe24f0227625e0ff91`). The
table and the checker output below are read from the log and `./scripts/check-findings-log.sh`, not by eye.

### web3sec status

no web3sec return as of 2026-11-03: slots W1 and W2 of `56-RUNBOOK-WEB3SEC.md` are empty, so nothing was
triaged and no entry with `**By:** web3sec` was written (D-15, P1). No return was simulated, inferred or
reconstructed. The D-16 decision for DOGF-02 is not yet taken.

### Every finding

| id | By | severity | last Disposition (120 chars) |
|----|----|----------|------------------------------|
| DF-01 | Claude | S2 | in v0.11.1 — fixed by FIRST-01 (Phase 54): the boot refuses an unexpanded ${…} auth key and the example and k8s manif... |
| DF-02 | Claude | S3 | in v0.11.1 — fixed by FIRST-02/FIRST-03 (Phase 54): README and docs pin 0.11.0 and make check-version-strings gates i... |
| DF-03 | Claude | S2 | in v0.11.1 — fixed by FIXES-01 (Phase 55): file deleted and the stub-panic gate holds; proof: make check-no-stub-pani... |
| DF-04 | code review | S1 | in v0.11.1 — fixed by TENANT-03, TENANT-04, TENANT-05, TENANT-07 (Phase 53): every thread, run-submit, fork and cance... |
| DF-05 | Claude | S3 | in v0.11.1 — fixed in plan 56-02: the example writes steps: [] under agent_runtime.model_fallback; regression: exampl... |
| DF-06 | Claude | S4 | out (re-homed: .planning/todos/pending/2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md) |
| DF-07 | Claude | S1 | open → promote **in v0.11.1** (S1, D-05; fix in Stage C, D-06) |
| DF-08 | Claude | S2 | open → promote **in v0.11.1** (S2, D-05; fix in Stage C) |
| DF-09 | Claude | S3 | in v0.11.1 — fixed in plan 56-05: the example's web_server comment says no shipped binary mounts the inspector; wirin... |
| DF-10 | Claude | S3 | duplicate of DF-02 |
| DF-11 | Claude | S4 | in v0.11.1 — fixed in plan 56-07: README.md and crate-map.md list all three default features; proof: tomllib prints t... |
| DF-12 | Claude | S4 | in v0.11.1 — fixed in plan 56-07: Quickstart says cargo run --example needs a checkout and how a crates.io user copie... |
| DF-13 | Claude | S3 | duplicate of DF-02 |
| DF-14 | Claude | S3 | in v0.11.1 — fixed in plan 56-07: installation.md names paladin-llm feature openai; proof: cargo metadata resolves th... |
| DF-15 | Claude | S3 | in v0.11.1 — fixed in plan 56-07: installation.md and cli-onboarding.md carry cargo install paladin-ai --features cli... |
| DF-16 | Claude | S3 | out (re-homed: .planning/todos/pending/2026-11-02-setup-check-first-run-output.md) |
| DF-17 | Claude | S3 | in v0.11.1 — fixed in plan 56-07: the six CLI appendix pages invoke paladin-cli onboarding and paladin-cli setup-chec... |
| DF-18 | maintainer | S3 | out (re-homed: .planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md) |
| DF-19 | maintainer | S2 | open → promote **in v0.11.1** (S2, D-05; fix in Stage C, D-06) |
| DF-20 | maintainer | S3 | out (re-homed: .planning/todos/pending/2026-11-03-devcontainer-install-cargo-llvm-cov.md; the fix is one RUN cargo in... |
| DF-21 | maintainer | S3 | in v0.11.1 — fixed in plan 56-09: the guide's Prerequisites now install cargo-llvm-cov --version 0.8.7 --locked (and ... |
| DF-22 | maintainer | S3 | in v0.11.1 — fixed in plan 56-09: "Reading the output" now says make coverage writes lcov.info and prints no percenta... |
| DF-23 | maintainer | S3 | in v0.11.1 — fixed in plan 56-09: the guide's Prerequisites now carry a paragraph saying that inside the devcontainer... |
| DF-24 | maintainer | S4 | in v0.11.1 — fixed in plan 56-09: the printed Scope: line now reads --workspace --features integration-tests,llm-all ... |
| DF-25 | Claude | S3 | in v0.11.1 — fixed in plan 56-09: coverage-html now passes --features integration-tests,llm-all, and the guide says t... |

### Checker output (`./scripts/check-findings-log.sh`)

```
Checking the findings log for entries without a final disposition ...
Findings-log check failed (BAD)

If this failure is unexpected:
  1. Append a dated '- **Disposition (YYYY-MM-DD):**' bullet under the entry
     (in v0.11.1 — <pointer>, out (<reason>) or duplicate of DF-nn). Never
     rewrite an earlier line; the log is append-only (D-01).
  2. This gate never edits the log. It is run at phase verification and by
     SHIP-10, not by check-gates or CI.

.project/v0.11.1/02-dogfood-findings-log.md:173: DF-07: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:204: DF-08: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:476: DF-19: OPEN (the last Disposition bullet reads 'open')
parsed 25 entries from .project/v0.11.1/02-dogfood-findings-log.md
exit=1
```

Ids still named, split by `**By:**`:

- web3sec: none
- other: DF-07 (By Claude, S1), DF-08 (By Claude, S2), DF-19 (By maintainer, S2). All three are promoted
  `in v0.11.1` and wait for a Stage C fix plan (`/gsd-plan-phase 56 --gaps`).

### S1/S2 entries whose last Disposition bullet reads `in v0.11.1`

| id | severity | fix pointer | regression test |
|----|----------|-------------|-----------------|
| DF-01 | S2 | FIRST-01 (Phase 54) | validate_refuses_an_unexpanded_placeholder_key, validate_refuses_a_placeholder_key_while_auth_is_disabled, validate_names_a_later_entry_whose_key_is_a_placeholder, validate_reports_a_placeholder_key_before_tenant_and_duplicate_checks, build_auth_config_refuses_an_unexpanded_placeholder_key, yaml_env_placeholder_is_not_expanded |
| DF-03 | S2 | FIXES-01 (Phase 55) | tests/scripts/check-no-stub-panics_test.sh and `make check-no-stub-panics` (0 findings) |
| DF-04 | S1 | TENANT-03, TENANT-04, TENANT-05, TENANT-07 (Phase 53) | every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404, cross_tenant_submit_on_a_thread_is_the_unknown_thread_404, cross_tenant_fork_is_the_unknown_thread_404, cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag, cancel_of_another_tenants_run_is_not_found_and_writes_nothing, job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass, permits_thread_admits_all_and_only_the_latest_runs_tenant |

S1/S2 entries still `open → promote **in v0.11.1**` with no fix pointer yet (Stage C, D-06): DF-07 (S1), DF-08 (S2),
DF-19 (S2). The close gate verdict is written by plan 56-10 task 3, after the D-16 decision.
