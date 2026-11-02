# v0.11.1 — Dogfood findings log

Running log of issues found while using v0.11.0 for real: the maintainer's own runs, the web3sec
consumer's return, and anyone else exercising the published `0.11.0` crates, `paladin-cli`, or
`paladin-server`. One entry per finding. Append; never rewrite an earlier entry (add a dated note).

When a finding is promoted into the v0.11.1 roadmap it keeps its `DF-nn` id so the roadmap row, the fix
commit and this log stay cross-referenced.

## Entry format

```
### DF-nn — <one-line title>
- **Found:** <date> · **By:** <maintainer | web3sec | other> · **Surface:** <cli | server | library | docs | ci | k8s>
- **Version:** 0.11.0 (crate / tag / commit if different)
- **What happened:** <observed behavior, verbatim error text in a code block>
- **Expected:** <what the docs, the type, or common sense said would happen>
- **Repro:** <command or minimal code; config excerpt with secrets removed>
- **Severity:** S1 security/isolation · S2 wrong result or crash · S3 misleading docs/config · S4 hygiene
- **Related:** <WINDOWS row, review id, inventory row in 01-carried-gaps-inventory.md, or none>
- **Disposition:** open | in v0.11.1 | out (reason) | duplicate of DF-nn
```

## Surfaces worth deliberately exercising

These are the entry points a new user meets first; each is a place where a stale string or an unexpanded
placeholder turns into a confusing first run.

- `README.md` Quick Example and Getting Started → `cargo add` / `Cargo.toml` snippet → first `basic_paladin`
  example.
- `paladin-cli onboarding` and `paladin-cli setup-check` (their live tests are `#[ignore]`d: "Requires valid
  API key", so they are the least-exercised code in CI).
- `paladin-server` from `config.example.yml` with `http.auth.enabled: true` and an `api_keys[].key` taken
  verbatim from the example (see DF-01).
- `k8s/server/` manifests as shipped (configmap header claim, worker-deployment labels).
- `paladin-cli treasury …` spend queries by tenant / key / model, and the CLI-only aggregate view the HTTP
  surface lacks (audit G1).
- The `examples/` set against a seeded provider other than OpenAI, and against Ollama / OpenAI-compatible
  (never probed, no seeded rows).
- The Switching-providers page walked for a provider the maintainer actually has a key for.
- `docs/src/contributing/testing-guide.md` local coverage walkthrough on a Docker-capable machine (todo
  `2026-08-13-verify-local-coverage-reproduction`).

## Findings

Pre-seeded on 2026-10-27 from reading the tree, not from a run. Each needs a real reproduction before it is
promoted.

### DF-01 — Example HTTP auth keys are `${VAR}` placeholders the loader never expands
- **Found:** 2026-10-27 (by reading) · **By:** Claude · **Surface:** server, k8s, docs
- **Version:** 0.11.0
- **What happened:** `config.example.yml` `http.auth.api_keys[].key: "${PALADIN_API_KEY_CI}"` and
  `k8s/server/configmap.yaml` (lines 36, 40; header says "expanded by the server at load time") would load
  the literal placeholder string as the API key. The loader performs no `${VAR}` expansion; the behavior is
  pinned by `src/config/treasurer.rs::yaml_env_placeholder_is_not_expanded`.
- **Expected:** either expansion, or a boot-time refusal naming the unexpanded key.
- **Repro:** copy the example, set `http.auth.enabled: true`, start `paladin-server`, call with the literal
  `${PALADIN_API_KEY_CI}` as the bearer — it should *not* authenticate, and it will.
- **Severity:** S2 (and S1 if someone ships it)
- **Related:** WINDOWS row 62; inventory B1, C3
- **Disposition:** open → recommend **in v0.11.1**
- **Disposition (2026-11-02):** in v0.11.1 — fixed by FIRST-01 (Phase 54): the boot refuses an unexpanded ${…} auth key and the example and k8s manifest no longer ship one; proof: validate_refuses_an_unexpanded_placeholder_key, validate_refuses_a_placeholder_key_while_auth_is_disabled, validate_names_a_later_entry_whose_key_is_a_placeholder, validate_reports_a_placeholder_key_before_tenant_and_duplicate_checks, build_auth_config_refuses_an_unexpanded_placeholder_key and yaml_env_placeholder_is_not_expanded, re-run green on 6212e3ecf (56-DOGFOOD-EVIDENCE.md)

### DF-02 — README advertises version 0.8.0 and a `0.5` install line
- **Found:** 2026-10-27 (by reading) · **By:** Claude · **Surface:** docs
- **Version:** 0.11.0
- **What happened:** `README.md:183` "Current version: **0.8.0**"; `README.md:108` `paladin-ai = "0.5"`.
  `docs/src/api-reference/crate-map.md:190` and `feature-flags.md:132,167,333` pin `"0.10.0"` (the yanked
  partial release); `docs/src/appendix/sentinel.md:66` pins `"0.5"`.
- **Expected:** `0.11.0` (or `0.11`) everywhere a user copies from.
- **Repro:** `grep -rnE 'paladin-ai = "|Current version' README.md docs/src`
- **Severity:** S3
- **Related:** inventory C1, C2; Phase 46 (docs currency) did not catch it, so a gate is the real fix
- **Disposition:** open → recommend **in v0.11.1** with a `check-version-strings` gate
- **Disposition (2026-11-02):** in v0.11.1 — fixed by FIRST-02/FIRST-03 (Phase 54): README and docs pin 0.11.0 and make check-version-strings gates it; proof: make check-version-strings (every copyable version string equals '0.11.0') and tests/scripts/check-version-strings_test.sh, re-run green on 6212e3ecf (56-DOGFOOD-EVIDENCE.md)

### DF-03 — `ContentFilter::filter_content_list` panics on first call (`todo!()` in library code)
- **Found:** 2026-10-27 (by reading) · **By:** Claude · **Surface:** library (`paladin-content`)
- **Version:** 0.11.0
- **What happened:** `crates/paladin-content/src/services/content_filtering_service.rs:21-26` constructs
  every `ContentList` field with `todo!()`; any caller with a non-empty filtered list panics.
- **Expected:** a result or a typed error; never a panic from a published crate.
- **Repro:** call `filter_content_list` on a list with one matching item.
- **Severity:** S2
- **Related:** inventory B2, G3 (same unfinished subsystem)
- **Disposition:** open → recommend **in v0.11.1** (remove or return `Err`; do not finish the feature)
- **Disposition (2026-11-02):** in v0.11.1 — fixed by FIXES-01 (Phase 55): file deleted and the stub-panic gate holds; proof: make check-no-stub-panics (0 findings) and tests/scripts/check-no-stub-panics_test.sh, re-run green on 6212e3ecf (56-DOGFOOD-EVIDENCE.md)

### DF-04 — Cross-tenant thread access through `/v1/threads/*`, run submission and fork
- **Found:** Phase 40 review (2026-09-29), never dispositioned · **By:** code review · **Surface:** server
- **Version:** 0.11.0
- **What happened:** see inventory A1–A3, A5. Listed here because a multi-tenant dogfood will hit it on the
  first `GET /v1/threads` with a second tenant's key.
- **Severity:** S1
- **Related:** WINDOWS row 58; 40-REVIEW WR-01, WR-02, IN-04
- **Disposition:** open → recommend **in v0.11.1**, first code phase
- **Disposition (2026-11-02):** in v0.11.1 — fixed by TENANT-03, TENANT-04, TENANT-05, TENANT-07 (Phase 53): every thread, run-submit, fork and cancel path scopes to the caller's tenant; proof: every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404, cross_tenant_submit_on_a_thread_is_the_unknown_thread_404, cross_tenant_fork_is_the_unknown_thread_404, cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag, cancel_of_another_tenants_run_is_not_found_and_writes_nothing, job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass and permits_thread_admits_all_and_only_the_latest_runs_tenant, re-run green on 6212e3ecf (56-DOGFOOD-EVIDENCE.md)

<!-- append new findings below this line -->

### DF-05 — `config.example.yml` claims `agent_runtime` is written out in full but omits `model_fallback.steps`
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-02) · **Surface:** docs
- **Version:** 0.11.0 (also HEAD 20808e4ae)
- **What happened:** the comment above `agent_runtime:` says every field "is written out explicitly and is
  IDENTICAL to what AgentRuntimeConfig::default() produces". The example writes
  `model_fallback: { enabled: false, providers: [] }`, but `ModelFallbackConfig` also has a `steps` field. The
  loaded `Settings` serialises a path the example never writes. Output of
  `example_sections_claimed_complete_write_every_field` (plan 56-02, lead L5) on the unmodified example:
  ```
  config.example.yml claims these sections are written out in full, but the loaded Settings serialises these paths the example does not write:
    agent_runtime.model_fallback.steps
  ```
- **Expected:** every field of the section written out, as the comment says, so an operator reading the example
  sees that `steps` exists.
- **Repro:** `cargo test -p paladin-ai --test config_example_roundtrip example_sections_claimed_complete_write_every_field`
- **Severity:** S3 (misleading config, D-04)
- **Related:** DOGF-03; lead L5
- **Disposition:** in v0.11.1 — fixed in plan 56-02: the example writes `steps: []` under `agent_runtime.model_fallback`; regression: `example_sections_claimed_complete_write_every_field`

### DF-06 — An unquoted treasurer price with 19 significant digits loses digits on load
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-02) · **Surface:** library
- **Version:** 0.11.0 (also HEAD 20808e4ae)
- **What happened:** `treasurer.pricing.<id>` axes are `String` fields documented as decimal strings. An
  operator who writes one unquoted gets it read as a number by the `config` crate and re-rendered. Credible
  prices keep their decimal value (`2.50` loads as `"2.5"`, `10.00` as `"10"`, `0.075` and `15` unchanged,
  pinned by `unquoted_credible_prices_keep_their_decimal_value`); a 19-significant-digit price does not:
  ```
  prompt: 1234567890.123456789   ->   loaded as "1234567890.1234567"
  ```
- **Expected:** the same decimal value, or a load error; never a silently different price.
- **Repro:** a config whose `treasurer.pricing.probe.prompt` is the unquoted literal above, loaded through
  `Settings::load_from_file` (one-off measurement in plan 56-02, not kept as a test).
- **Severity:** S4 (hygiene, D-04: no credible per-1M-token price has 19 significant digits, and the example
  already says prices are DECIMAL STRINGS and shows them quoted)
- **Related:** DOGF-03; quoted values are pinned byte-identical by `quoted_money_strings_arrive_byte_identical`
- **Disposition:** out (re-homed: .planning/todos/pending/2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md)

### DF-07 — `paladin` binary logs the whole `Settings` with `{:?}`, printing `http.auth.api_keys[].key` in plaintext
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-05) · **Surface:** cli
- **Version:** 0.11.0 (tag v0.11.0; reproduced on HEAD 1203a0dfd)
- **What happened:** `src/main.rs:35` is `info!("Loaded configuration: {:?}", config);`, present at tag v0.11.0 as
  well. The binary initialises `env_logger` with `default_filter_or("debug")`, so with `RUST_LOG` unset the line is
  emitted. `Settings` and `ApiKeyConfig` (`src/config/agents.rs`) both `#[derive(Debug)]`, so the literal key is part
  of the line. Probe with a fake key, a copy of `config.example.yml` whose only change is one `http.auth.api_keys`
  entry; the first log line (truncated, key replaced):
  ```
  [... INFO  paladin] Loaded configuration: Settings { llm_type: "openai", ... auth: AuthConfig { enabled: true, api_keys: [ApiKeyConfig { key: "<fake-key>", name: "l3-probe", role: User, tenant: "Acme-Prod" }], bearer_token: BearerTokenAuthConfig { enabled: false, tenant: None } }, docs: DocsConfig { enabled: true } }), ...
  ```
  The literal appears exactly once in the log (count 1). The same line also carries, unredacted, `queue.redis_password`,
  `file_storage.minio_access_key` / `minio_secret_key` and a non-empty top-level `llm_api_key` (one occurrence each,
  second probe); `EmailAdapterConfig.password` (feature `notifications`) and credential-bearing URLs are the same
  shape from source. Provider keys are the exception: `LlmProviderConfig` redacts (`api_key: <redacted>`), which
  shows the sibling auth/queue/storage types were missed. The process then exited 1 on an unrelated SQLite path
  error from the example, after the line was written. Evidence: `56-DOGFOOD-EVIDENCE.md` § Lead L3.
- **Expected:** no credential in any log line at the binary's default filter. Either the `{:?}` of the whole config is
  dropped, or the credential-bearing types redact in `Debug`. A redacting `Debug` impl for `ApiKeyConfig` (and the
  queue, file-storage, `Settings.llm_api_key` and SMTP types) changes no public signature (D-00b patch shape);
  Stage C decides which.
- **Repro:** from a scratch directory outside the repo, with `config.example.yml` copied to `l3.yml` and the line
  `    api_keys: []` replaced by one entry (`key: "<fake-key>"`, `name: "l3-probe"`, `role: "user"`,
  `tenant: "Acme-Prod"`):
  `cargo build -p paladin-ai --bin paladin --features cli` then
  `env -u RUST_LOG OPENAI_API_KEY=<fake-key> timeout 20 target/debug/paladin --config ./l3.yml > l3.log 2>&1`, and
  `grep -c '<the fake literal>' l3.log` (1 or more reproduces).
- **Severity:** S1 (D-04: graded at the worst credible deployment. The binary's default filter is `debug`, so any
  container or systemd unit running `paladin --config` ships every static API key into the log aggregator that
  collects stdout/stderr; the keys are the only authentication in front of the agent API, and the config documents
  them as literal secrets)
- **Related:** lead L3 (56-RESEARCH.md); `.github/instructions/security.instructions.md` manual credential-handling
  review (no log statement may interpolate an API key or `Debug`-format a config type carrying one); same root cause
  for the unredacted types listed in the audit (`QueueConfig`, `FileStorageConfig`, `Settings.llm_api_key`,
  `EmailAdapterConfig`)
- **Disposition:** open → promote **in v0.11.1** (S1, D-05; fix in Stage C, D-06)

### DF-08 — `paladin-server` never calls `Settings::validate()`: a `trace` value its type rejects boots
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-05) · **Surface:** server
- **Version:** 0.11.0 (tag v0.11.0; reproduced on HEAD 4ee454557)
- **What happened:** `Settings::validate()` (`src/config/settings.rs:109`) validates `agent_runtime`, `trace`,
  `web_server` and the treasurer, and has no non-test caller; `src/bin/paladin-server.rs` validates the treasurer,
  run store, run worker and the other env configs one by one but never `trace` or `web_server`. The example config
  with `trace.channel_capacity: 0` (which `TraceConfig::validate_typed` rejects as `ZeroChannelCapacity`), auth
  disabled and fake provider keys boots and serves:
  ```
  INFO paladin_server: paladin-server listening on http://127.0.0.1:44907 — serving 2 agent(s): ["researcher", "summarizer"]
  ```
  `/health` answered 200, no `panicked at`. With `channel_capacity: 0` the engine clamps the queue to 1
  (`TraceDispatcher::with_capacity`, `capacity.max(1)`), so the first run neither panics nor hangs. A second boot
  with `trace.otel.enabled: true` on a build without the `otel` feature, which `validate()` rejects as
  `FeatureNotCompiled { feature: "otel" }`, also listens, with no OTLP line and no error: configured telemetry
  export silently never happens.
- **Expected:** a config its own validator rejects is refused at boot naming the key, as the treasurer, run-store
  and auth configs already are. Calling `settings.validate()` after `Settings::load_from_file` changes no public
  signature (D-00b patch shape); Stage C decides, including whether `agent_runtime` is then validated twice.
- **Repro:** copy `config.example.yml`, set `server.host: "127.0.0.1"`, a free `server.port`,
  `http.auth.enabled: false`, `trace.channel_capacity: 0`; then
  `cargo build -p paladin-ai --bin paladin-server --features web-server` and
  `env -u RUST_LOG OPENAI_API_KEY=<fake-key> APP_RUN_STORE_BACKEND=sqlite APP_RUN_STORE_PATH=sqlite://$S/runs.db APP_WAYPOINT_STORE_BACKEND=sqlite APP_WAYPOINT_STORE_SQLITE_PATH=sqlite://$S/wp.db PALADIN_CONFIG=./l6.yml timeout 20 target/debug/paladin-server`
  and look for `paladin-server listening on` (the run store needs the waypoint store variables or boot stops on an
  unrelated refusal). Evidence: `56-DOGFOOD-EVIDENCE.md` § Lead L6.
- **Severity:** S2 (D-04: arguable between S2 and S3, logged at the higher one. The `channel_capacity: 0` case is
  clamped and harmless, but the same gap lets `otel.enabled: true` on a build without the feature silently export
  nothing, which is a wrong result with no error, and it applies to every field `validate()` covers)
- **Related:** lead L6 (56-RESEARCH.md); DF-09 (`web_server` is also covered by the unused `validate()`)
- **Disposition:** open → promote **in v0.11.1** (S2, D-05; fix in Stage C)

### DF-09 — `web_server.dev_ui.mermaid_url` is documented as live config but no shipped binary mounts the inspector
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-05) · **Surface:** docs
- **Version:** 0.11.0 (tag v0.11.0; reproduced on HEAD 4ee454557)
- **What happened:** the comment above `web_server:` in `config.example.yml` described the section as "currently
  just the developer inspector UI" and the key as a setting an operator can point at a local mirror. Nothing reads
  it: `create_dev_ui_router` has callers only in tests and `examples/platform_api_client.rs` (which hard-codes the
  URL), no code outside `src/config/` reads `Settings.web_server`, and `paladin-server` booted from the example
  answers `GET /v1/dev-ui/threads/probe` with 404 while `/health` answers 200:
  ```
  $ curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:44907/v1/dev-ui/threads/probe
  404
  ```
- **Expected:** the example says what the section does in this release: parsed, mounted by no shipped binary, and
  reachable only by an application calling `paladin_web::app::create_dev_ui_router` with feature `dev-ui`.
- **Repro:** `git grep -n 'create_dev_ui_router' -- src crates examples tests` and
  `git grep -nE 'web_server|get_web_server|WebServerConfig' -- src ':!src/config' crates examples`, then boot
  `paladin-server` from the example (auth disabled, free port) and `curl` the route above. Evidence:
  `56-DOGFOOD-EVIDENCE.md` § Lead L4.
- **Severity:** S3 (misleading config, D-04)
- **Related:** lead L4 (56-RESEARCH.md); DOGF-03
- **Disposition:** in v0.11.1 — fixed in plan 56-05: the example's web_server comment says no shipped binary mounts the inspector; wiring it would be a new route (out, D-00d)

### DF-10 — The published README's `[dependencies]` snippet resolves to `paladin-ai 0.5.1`, not the release the reader is looking at
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** docs
- **Version:** 0.11.0 (published crate)
- **What happened:** a new user copies the Getting Started block of the README that crates.io shows for
  `paladin-ai 0.11.0` into a fresh crate and resolves it. The snippet's `0.5` requirement picks `0.5.1`, six minor
  releases behind the page they read it on, and the same README says `Current version: **0.8.0**`:
  ```
  paladin-ai = "0.5"                      # default features: ["llm-openai"]
  Adding paladin-ai v0.5.1 (available: v0.11.0)
  Current version: **0.8.0**.
  ```
  `cargo metadata` on the scratch crate printed `paladin-ai 0.5.1 registry+https://github.com/rust-lang/crates.io-index`.
  HEAD's README pins `0.11.0` and says `Current version: **0.11.0**` (Phase 54, FIRST-02), so this is DF-02's defect
  as a published-crate reader meets it (D-00e).
- **Expected:** the snippet resolves to the release being read (`0.11.0`).
- **Repro:** scratch crate outside any repository; replace `[dependencies]` with the block from
  `~/.cargo/registry/src/*/paladin-ai-0.11.0/README.md` (or `git show v0.11.0:README.md`); run
  `cargo metadata --format-version 1` and read the `paladin-ai` package version.
- **Severity:** S3
- **Related:** DF-02; step 2 of `56-FIRST-RUN-WALK.md`
- **Disposition:** duplicate of DF-02

### DF-11 — README's `# default features: ["llm-openai"]` comment names one of the three default features
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** docs
- **Version:** 0.11.0 (published crate; also HEAD a39a338dd)
- **What happened:** the Getting Started snippet comments `# default features: ["llm-openai"]`. The package's
  `default` feature is `["llm-openai", "llm-anthropic", "llm-deepseek"]` (published `Cargo.toml` line 77; HEAD root
  manifest the same), and `cargo add paladin-ai@0.11.0` printed all three as enabled:
  ```
             + llm-anthropic
             + llm-deepseek
             + llm-openai
  ```
  A reader who relies on the comment to trim compile time (`default-features = false`) learns the wrong set.
- **Expected:** the comment lists all three defaults.
- **Repro:** `grep -n 'default features' README.md`, then `sed -n '/^default = \[/,/\]/p' Cargo.toml`.
- **Severity:** S4 (hygiene, D-04: nothing breaks; the reader only compiles more than they thought)
- **Related:** step 2 of `56-FIRST-RUN-WALK.md`; the same comment in `docs/src/api-reference/crate-map.md`
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-07)
