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
