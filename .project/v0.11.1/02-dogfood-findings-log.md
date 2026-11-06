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
- **Disposition (2026-11-03):** in v0.11.1 — fixed by 74317f48a, fbc514410 (plan 56-11); regression: api_key_config_debug_redacts_the_key, settings_summary_names_no_credential, settings_debug_redacts_every_credential_it_reaches, queue_config_debug_redacts_the_redis_password, file_storage_config_debug_redacts_both_minio_keys, email_adapter_config_debug_redacts_the_password; binary re-probe 0/5

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
- **Disposition (2026-11-03):** in v0.11.1 — fixed by 59bb84e0d (plan 56-12); regression: load_settings_refuses_a_zero_trace_channel_capacity, load_settings_refuses_otel_enabled_without_the_otel_feature, load_settings_refuses_an_invalid_agent_runtime_value, load_settings_accepts_the_shipped_example_config; boot probe refuses (exit 1)

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
- **Disposition (2026-11-02):** in v0.11.1 — fixed in plan 56-07: README.md and crate-map.md list all three default features; proof: tomllib prints the root manifest default = [llm-openai, llm-anthropic, llm-deepseek], matching the comments (56-FIRST-RUN-WALK.md, Contained fixes)

### DF-12 — `examples/basic_paladin.rs` in the published package cannot be run from a consumer crate, and no doc says how a crates.io user runs it
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** docs
- **Version:** 0.11.0 (published crate; also HEAD a39a338dd)
- **What happened:** the package lists `examples/**` in `include`, so `basic_paladin.rs` is in the downloaded
  `paladin-ai-0.11.0` source. Copied into a crate whose `[dependencies]` are the README's (`paladin-ai`, `tokio`) and
  run with `cargo run --example basic_paladin`, it does not compile, because it imports `paladin_ports` and the
  README never asks for that crate:
  ```
  error[E0433]: cannot find module or crate `paladin_ports` in this scope
    --> examples/basic_paladin.rs:22:5
     |
  22 | use paladin_ports::output::llm_port::LlmPort;
     |     ^^^^^^^^^^^^^ use of unresolved module or unlinked crate `paladin_ports`
     = help: if you wanted to use a crate named `paladin_ports`, use `cargo add paladin_ports` to add it to your `Cargo.toml`
  ```
  exit 101. After `cargo add paladin-ports@0.11.0` the same example runs and exits 0 (`✨ Example completed
  successfully!`). The only documented route to the examples is `git clone` then `cargo run --example basic_paladin`
  (`docs/src/getting-started/quickstart.md`, same text at the tag and on HEAD), which is a checkout, not the
  published crate; neither the README nor the Quickstart says that an example copied into your own crate needs
  `paladin-ports`.
- **Expected:** the docs say how to run an example (from a clone) and what a copied example needs in `[dependencies]`.
- **Repro:** scratch crate on crates.io `0.11.0`; copy `examples/basic_paladin.rs` from
  `~/.cargo/registry/src/*/paladin-ai-0.11.0/examples/` into its `examples/`; `cargo run --example basic_paladin`.
- **Severity:** S4 (hygiene, D-04: the documented clone route works; only a reader who tries the published source meets it)
- **Related:** step 4 of `56-FIRST-RUN-WALK.md`; the README Quick Example avoids the import through `paladin::prelude::*`
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-07)
- **Disposition (2026-11-02):** in v0.11.1 — fixed in plan 56-07: Quickstart says cargo run --example needs a checkout and how a crates.io user copies and runs basic_paladin with paladin-ports; proof: 56-06 step 4 ran exactly those steps, exit 101 then 0 (56-FIRST-RUN-WALK.md, Contained fixes)

### DF-13 — The Quickstart and Installation pages at the `v0.11.0` tag pin `0.10.0`, which resolves to `0.10.1`
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** docs
- **Version:** 0.11.0 (tag v0.11.0, docs site built from it)
- **What happened:** `git show v0.11.0:docs/src/getting-started/quickstart.md` has `paladin-ai    = "0.10.0"`,
  `paladin-ports = "0.10.0"` and `paladin-llm   = { version = "0.10.0", features = ["openai"] }`;
  `installation.md` at the tag pins `0.10.0` for six crates. Resolved as written in a scratch crate, `cargo metadata`
  picks `paladin-ai 0.10.1` and `paladin-llm 0.10.1` (`registry+https://github.com/rust-lang/crates.io-index`), so a
  reader following the Quickstart for the `0.11.0` release builds the previous release. HEAD's Quickstart pins `0.11.0`
  (`diff` of the file against the tag shows only those three lines) and `make check-version-strings` is green on HEAD
  ("every copyable version string equals '0.11.0'"), so this is DF-02's class of defect on two more pages (D-00e).
- **Expected:** the pages pin the release being read.
- **Repro:** `git show v0.11.0:docs/src/getting-started/quickstart.md`; put its `[dependencies]` block into a scratch
  crate; `cargo metadata --format-version 1` and read the `paladin-*` versions.
- **Severity:** S3
- **Related:** DF-02, DF-10; step 5 of `56-FIRST-RUN-WALK.md`. The walk used `0.11.0` for the three `paladin-*` crates
  from here on (as the keyed runbook K2 allows) and said so.
- **Disposition:** duplicate of DF-02

### DF-14 — Installation page names a `paladin-llm` feature `llm-openai` that does not exist (lead L1)
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** docs
- **Version:** 0.11.0 (published crates; same line on HEAD a39a338dd, `installation.md:57`)
- **What happened:** the `[dependencies]` block of `docs/src/getting-started/installation.md` has
  `paladin-llm       = { version = "0.11.0", features = ["llm-openai"] }`. `llm-openai` is a feature of the
  `paladin-ai` umbrella; published `paladin-llm 0.11.0` calls it `openai`. Resolved in a scratch crate with the HEAD
  block's versions (every `paladin-*` at `0.11.0`):
  ```
  error: failed to select a version for `paladin-llm`.
  versions that meet the requirements `^0.11.0` are: 0.11.0

  package `installation-as-written` depends on `paladin-llm` with feature `llm-openai` but `paladin-llm` does not have that feature.
  help: available features: anthropic, deepseek, default, gemini, grok, kimi, mock, ollama, openai, openai-compatible, openai-embeddings, qwen, vision
  ```
  exit 101. The same block with `features = ["openai"]` resolves (exit 0, all seven `paladin-*` at `0.11.0`); that
  pair is the proof plan 56-07 will cite. The tag's block fails the same way (it also pins `0.10.0`, see DF-13).
- **Expected:** a block a reader can paste and resolve.
- **Repro:** scratch crate; put the block in `[dependencies]`; `cargo metadata --format-version 1`.
- **Severity:** S3
- **Related:** lead L1 (56-RESEARCH.md); step 5b of `56-FIRST-RUN-WALK.md`
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-07)
- **Disposition (2026-11-02):** in v0.11.1 — fixed in plan 56-07: installation.md names paladin-llm feature openai; proof: cargo metadata resolves the corrected block on crates.io 0.11.0 and rejects the old one naming llm-openai (56-FIRST-RUN-WALK.md, Contained fixes)

### DF-15 — No doc names a `cargo install` line for `paladin-cli`, and the line a reader tries first fails (lead L2)
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** docs
- **Version:** 0.11.0 (published crate; no `cargo install` of the CLI on HEAD a39a338dd either)
- **What happened:** the CLI appendix pages say to build from a checkout (`cargo build --release --features cli --bin
  paladin-cli`) and no page gives an install line for a crates.io user. The obvious line fails:
  ```
  $ cargo install paladin-ai --version 0.11.0 --bin paladin-cli
  error: failed to compile `paladin-ai v0.11.0`, ...
  Caused by:
    target `paladin-cli` in package `paladin-ai` requires the features: `cli`
    Consider enabling them by passing, e.g., `--features="cli"`
  ```
  exit 101. The form with the feature works: `cargo install paladin-ai --version 0.11.0 --features cli --locked
  --bin paladin-cli --root <scratch>` exits 0 (`Installed package \`paladin-ai v0.11.0\` (executable \`paladin-cli\`)`),
  and the installed binary runs (`paladin-cli --version` prints `paladin 0.11.0`). Assumption A1 is proven.
- **Expected:** the install docs carry a working line for the CLI.
- **Repro:** the two commands above, `--root` set to a scratch directory.
- **Severity:** S3
- **Related:** lead L2 (56-RESEARCH.md); step 7 of `56-FIRST-RUN-WALK.md`
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-07)
- **Disposition (2026-11-02):** in v0.11.1 — fixed in plan 56-07: installation.md and cli-onboarding.md carry cargo install paladin-ai --features cli --locked --bin paladin-cli; proof: 56-06 step 7(b) ran it with --version 0.11.0, exit 0 (56-FIRST-RUN-WALK.md, Contained fixes)

### DF-16 — `paladin-cli setup-check` prints its sections in a different order on each run and bare headings with no rows when no key is set
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** cli
- **Version:** 0.11.0 (published crate; `setup_check.rs` is unchanged between the tag and HEAD a39a338dd)
- **What happened:** with no provider key, `REDIS_URL` or `QDRANT_URL` and no `.env`, the section order of the same
  command differed from run to run (four of the runs, headings only):
  ```
  Service: LLM Provider: Environment: System:
  System: Service: Environment: LLM Provider:
  System: LLM Provider: Service: Environment:
  LLM Provider: Environment: System: Service:
  ```
  The cause is in the source: `setup_check.rs` groups results in a `HashMap<String, Vec<_>>` and prints it in
  iteration order. Without `--verbose` the `Skip` rows are not printed, so `Service:` and `LLM Provider:` appear as
  bare headings with nothing under them, and the only sign that no provider is configured is the `.env` warning:
  ```
  Service:

  LLM Provider:

  Environment:
    ⚠ .env file
  ⚠   Not found - run 'paladin onboarding' to create
  ```
  Exit code 2, as the page documents for warnings. `--verbose` lists `○ OpenAI`, `○ Anthropic`, `○ DeepSeek`,
  `○ Redis`, `○ Qdrant` (each `... not configured`) and `Skipped: 5`. The sample output on `cli-setup-check.md` is a
  different layout (`✓ Paladin CLI: v0.1.0`, `Providers:`, `Services (Optional):`); the real line is `✓ Paladin CLI v0.11.0`.
- **Expected:** a stable section order, no empty headings, and a keyless first run that says which keys are missing.
- **Repro:** from an empty directory, `env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY -u REDIS_URL
  -u QDRANT_URL paladin-cli setup-check`, three or four times.
- **Severity:** S3 (misleading output, D-04: nothing breaks, but the command meant to reassure a new user is unstable
  and silent about the missing key)
- **Related:** step 9 of `56-FIRST-RUN-WALK.md`; DF-17 (the same file's `paladin` strings)
- **Disposition:** out (re-homed: .planning/todos/pending/2026-11-02-setup-check-first-run-output.md)

### DF-17 — Docs and the CLI's own messages tell the reader to run `paladin onboarding` / `paladin setup-check`, but the installed binary is `paladin-cli`
- **Found:** 2026-11-02 · **By:** Claude (Phase 56, plan 56-06) · **Surface:** docs
- **Version:** 0.11.0 (published crate; the six pages are byte-identical at the tag and on HEAD a39a338dd)
- **What happened:** with only the install's `bin` directory on `PATH`, the command as the docs print it fails:
  ```
  $ paladin onboarding
  /bin/bash: line 1: paladin: command not found
  exit=127
  $ paladin setup-check
  /bin/bash: line 1: paladin: command not found
  exit=127
  ```
  `command -v paladin-cli` finds the binary. The wrong name is on `cli-onboarding.md`, `cli-setup-check.md`,
  `cli-usage.md`, `cli-configuration.md`, `cli-muster.md` and `cli-council.md`, and in the CLI's own output: the
  `setup-check` warning (`Not found - run 'paladin onboarding' to create`), its Next Steps box, and the onboarding
  messages and generated `.env` header (`src/application/cli/commands/{setup_check,onboarding}.rs`,
  `src/application/cli/templates/env.rs`). The `paladin` binary in the same package takes only `--config`
  (`src/main.rs`), so a reader who installs it too gets no `onboarding` subcommand. One root cause: the tool is
  `paladin-cli` while its clap name is `paladin` (`paladin-cli --version` prints `paladin 0.11.0`).
- **Expected:** every command line a reader copies names the binary that has the subcommand.
- **Repro:** install per DF-15 into a scratch root; `PATH=<root>/bin paladin onboarding`.
- **Severity:** S3
- **Related:** step 11 of `56-FIRST-RUN-WALK.md`; keyed runbook K9 anticipates it. Plan 56-07 fixes the docs lines;
  the strings inside the binary are a code change (P10) and are re-homed with DF-16's todo.
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-07)
- **Disposition (2026-11-02):** in v0.11.1 — fixed in plan 56-07: the six CLI appendix pages invoke paladin-cli onboarding and paladin-cli setup-check; the binary strings stay re-homed with DF-16 (.planning/todos/pending/2026-11-02-setup-check-first-run-output.md, P10); proof: anchored grep finds 0 bare-paladin invocations and 38 paladin-cli ones (56-FIRST-RUN-WALK.md, Contained fixes)

### DF-18 — The Quickstart's `.model("gpt-4")` fails against a real OpenAI key: the model is deprecated
- **Found:** 2026-11-03 · **By:** maintainer · **Surface:** docs
- **Version:** 0.11.0 (published crates; `docs/src/getting-started/quickstart.md:53` is byte-identical at tag `v0.11.0` and on HEAD dc190dba6)
- **What happened:** the Quickstart `main.rs` as written, built against crates.io `0.11.0` with the `openai` feature,
  stops at the first provider call (exit 1):
  ```
  Error: LlmFailure { transience: Permanent, status: None, provider: None, message: "Model not available: {\n    \"error\": {\n        \"message\": \"The model `gpt-4` has been deprecated, learn more here: https://platform.openai.com/docs/deprecations\",\n        \"type\": \"invalid_request_error\",\n        \"param\": null,\n        \"code\": \"model_not_found\"\n    }\n}\n" }
  ```
  After `sed -i 's/"gpt-4"/"gpt-4o-mini"/' src/main.rs` the same program prints `Output : Hello! How can I assist you
  today?`, `Tokens : 32`, `Time   : 1390ms` (exit 0). The exact literal `"gpt-4"` is not a one-page problem: on HEAD it
  appears at 109 sites in 54 files (40 sites in 18 files under `docs/src`, 69 sites in 36 files under `examples/`, none
  in `README.md`). The seeded OpenAI registry rows prefer `gpt-4o-mini` and `gpt-5.4-nano`.
- **Expected:** the first program a new user copies reaches a completion with a key that works.
- **Repro:** scratch crate with the Quickstart dependency block (`paladin-ai`, `paladin-ports`, `paladin-llm` with
  `features = ["openai"]`, all `0.11.0`, plus `tokio`) and the Quickstart `main.rs`; `OPENAI_API_KEY` set; `cargo run`.
  Count: `grep -rn '"gpt-4"' docs/src examples README.md | wc -l` (109).
- **Severity:** S3 (D-04: a docs literal. The result is a clear provider error on the very first keyed run, but nothing
  in the library is wrong, and the fix is a text change)
- **Related:** K2 and K9 of `56-RUNBOOK-KEYED-WALK.md`; step 5 of `56-FIRST-RUN-WALK.md` (the keyless walk stopped at the
  missing key, one step before this); DF-13 (the same page's version pin)
- **Disposition:** out (re-homed: .planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md)

### DF-19 — `setup-check` and `onboarding` reject a valid Anthropic key: the probe hard-codes `claude-3-haiku-20240307` (lead L8)
- **Found:** 2026-11-03 · **By:** maintainer · **Surface:** cli
- **Version:** 0.11.0 (published crate; `setup_check.rs` and `onboarding.rs` carry the same literal at tag `v0.11.0` and on HEAD dc190dba6)
- **What happened:** with a working Anthropic key exported alongside the OpenAI key, `setup-check --verbose` fails the
  Anthropic row and the whole run (exit 1):
  ```
  LLM Provider:
    ✓ OpenAI
  ℹ   All checks passed
    ✗ Anthropic
  ✗   API validation failed: Paladin execution error: Anthropic API validation failed (404 Not Found)
  ...
    Passed: 4
    Failed: 1
    Skipped: 3

  ✗ Setup check failed - please address the errors above
  ```
  The key itself is valid (the maintainer uses it elsewhere). The cause is in the source: the validator posts a one-token
  request to `https://api.anthropic.com/v1/messages` with `"model": "claude-3-haiku-20240307"`
  (`src/application/cli/commands/setup_check.rs:101`), and `src/application/cli/commands/onboarding.rs:99` carries the
  same literal, so `onboarding` validates the same way. A 404 on that request is the status a retired model id gets; the
  CLI discards the response body, so the id is the inferred cause, not one read from the body. The error text names
  neither the model nor the provider's reply, so the reader cannot tell a bad key from a bad probe. The seeded registry
  (`crates/paladin-llm/src/registry/mod.rs`) knows `claude-haiku-5-5` and `claude-opus-4-8`. The OpenAI row passed in the
  same run, and the OpenAI half of `onboarding` validated (K6).
- **Expected:** a valid key passes validation; a failure says which model was probed and what the provider answered.
- **Repro:** install per DF-15; export a valid `ANTHROPIC_API_KEY` (read with `read -rs`); `paladin-cli setup-check
  --verbose`. Source check: `grep -n '"model"' src/application/cli/commands/setup_check.rs
  src/application/cli/commands/onboarding.rs`.
- **Severity:** S2 (D-04: a wrong result. The command meant to confirm a working setup reports a valid key as failed and
  exits 1, so an Anthropic-only user who runs `onboarding` is told their key is bad)
- **Related:** lead L8 (56-RESEARCH.md), assumption A6; K8 of `56-RUNBOOK-KEYED-WALK.md`; DF-16 (the same file)
- **Disposition:** open → promote **in v0.11.1** (S2, D-05; fix in Stage C, D-06)
- **Disposition (2026-11-03):** in v0.11.1 — fixed by 343300aff, 889b5f1bb (plan 56-13); regression: anthropic_probe_model_is_the_adapters_seeded_default, anthropic_probe_posts_the_probe_model_and_passes_on_200, anthropic_probe_failure_names_the_model_and_the_provider_reply, anthropic_probe_does_not_follow_a_redirect; maintainer re-check 2026-11-03 ✓ Anthropic

### DF-20 — The devcontainer installs the `llvm-tools-preview` component but not `cargo-llvm-cov`, so the first `make coverage` after a rebuild fails
- **Found:** 2026-11-03 · **By:** maintainer · **Surface:** docs
- **Version:** 0.11.0 (devcontainer files at commit 21dacf0d6)
- **What happened:** on a freshly rebuilt devcontainer, step 0 of the coverage walk could not report a tool version
  because the subcommand does not exist (C1, first attempt):
  ```
  error: no such command: `llvm-cov`

  help: view all installed commands with `cargo --list`
  help: find a package to install `llvm-cov` with `cargo search cargo-llvm-cov`
  ```
  `.devcontainer/Dockerfile.dev:112` adds the `llvm-tools-preview` rustup component and the lines that follow install
  `cargo-watch`, `cargo-edit`, `cargo-audit`, `cargo-deny`, `cargo-semver-checks` and others, but nothing installs
  `cargo-llvm-cov`; `post-create.sh` does not either. Yet `.devcontainer/README.md:10` lists "llvm-cov" among the
  cargo extensions the container provides. Phase 36.1 had found a copy at `/usr/local/cargo/bin/cargo-llvm-cov`, but
  `/usr/local/cargo/bin` is not a persisted volume in `.devcontainer/docker-compose.yml` (only
  `/usr/local/cargo/registry` is), so a hand install does not survive a rebuild. The maintainer installed it by hand at
  CI's pin (`cargo install cargo-llvm-cov --version 0.8.7 --locked`), after which `cargo llvm-cov --version` printed
  `cargo-llvm-cov 0.8.7`.
- **Expected:** the project's own devcontainer can run `make coverage` once its services are up, as its README implies.
- **Repro:** rebuild the devcontainer; `cargo llvm-cov --version`. Source check: `grep -n 'llvm' .devcontainer/Dockerfile.dev
  .devcontainer/README.md`.
- **Severity:** S3 (D-04: the README claims a tool the image does not ship; the failure is a clear error with a
  one-line fix, and CI is unaffected)
- **Related:** C1, C6 bullet 1, trap 3 of `56-RUNBOOK-COVERAGE.md`; DF-21
- **Disposition:** open → **in v0.11.1** (contained: add the pinned install to `Dockerfile.dev`)
- **Disposition (2026-11-03):** out (re-homed: .planning/todos/pending/2026-11-03-devcontainer-install-cargo-llvm-cov.md; the fix is one `RUN cargo install --locked --version 0.8.7 cargo-llvm-cov` line in `.devcontainer/Dockerfile.dev`, a file outside plan 56-09's `files_modified`; the guide now tells a devcontainer reader to check `cargo llvm-cov --version` first, see DF-21)

### DF-21 — The testing guide installs `cargo-llvm-cov` unpinned while CI pins 0.8.7, and says its commands are the same ones CI runs
- **Found:** 2026-11-03 · **By:** maintainer · **Surface:** docs
- **Version:** 0.11.0 (`docs/src/contributing/testing-guide.md` at commit 21dacf0d6)
- **What happened:** the guide's "Prerequisites" gave `cargo install cargo-llvm-cov --locked` (and
  `cargo binstall cargo-llvm-cov`) with no `--version`, while the guide's opening says "every command here is the same
  command CI runs, not an approximation of it". CI installs `cargo-llvm-cov@0.8.7` (`taiki-e/install-action`,
  `.github/workflows/ci.yml:1489`). An unpinned install takes the newest release. The maintainer pinned it by hand
  (C6):
  ```
  Trap 3 (unpinned install in the guide): bit in spirit — the guide's `cargo install
  cargo-llvm-cov --locked` has no `--version`; the maintainer pinned `0.8.7` by hand to match CI.
  ```
  In this walk the pin did not change the outcome (local 92.79 percent against CI 92.86 percent, both truncate to 92),
  so the unpinned form was not shown to move the figure; the risk is that a newer tool release can, and the guide
  gave no way to tell tool drift from a code difference.
- **Expected:** the guide installs the version CI measures with, and says why.
- **Repro:** `grep -n 'cargo install cargo-llvm-cov' docs/src/contributing/testing-guide.md` against `grep -n
  'cargo-llvm-cov@' .github/workflows/ci.yml`.
- **Severity:** S3 (D-04: a guide step that does not match the claim made for it; the fix is a text change)
- **Related:** C1, C6 (trap 3), trap 3 of `56-RUNBOOK-COVERAGE.md`; DF-20
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-09)
- **Disposition (2026-11-03):** in v0.11.1 — fixed in plan 56-09: the guide's Prerequisites now install `cargo-llvm-cov --version 0.8.7 --locked` (and the `cargo binstall` form), name `ci.yml`'s pin as the source and say why (tool drift reads as a CI disagreement), and tell a devcontainer reader to run `cargo llvm-cov --version` first because the image may not carry the tool (DF-20); proof: `grep -n 'version 0.8.7' docs/src/contributing/testing-guide.md` finds both install forms

### DF-22 — `make coverage` prints no coverage percentage, although the guide says it prints an LCOV summary
- **Found:** 2026-11-03 · **By:** maintainer · **Surface:** docs
- **Version:** 0.11.0 (`docs/src/contributing/testing-guide.md` "Reading the output" at commit 21dacf0d6)
- **What happened:** the guide said "`make coverage` prints an LCOV summary". The run (exit 0, every suite passed) ended
  with the test results and one line, and no figure (C3):
  ```
      Finished report saved to lcov.info
  exit=0
  ```
  The maintainer's note (C3): "No percentage was printed (trap 1 confirmed)." `scripts/coverage.sh` runs
  `cargo llvm-cov ... --lcov --output-path lcov.info`, which writes a file and prints nothing; CI says the same in its
  own comments above its "Coverage summary" step and computes the figure itself. The maintainer needed the LF/LH
  snippet from the runbook to get a number (C5):
  ```
  Lines: 170650/183901 = 92.79%  (truncated: 92)
  ```
- **Expected:** after following the guide the reader can read the line figure the gate compares.
- **Repro:** `make coverage` with the services up; look for a percentage in the output.
- **Severity:** S3 (D-04: the guide states output the command does not produce, and the one number the section is
  about cannot be read from it)
- **Related:** C3, C5, C6 bullet 3, trap 1 of `56-RUNBOOK-COVERAGE.md`
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-09)
- **Disposition (2026-11-03):** in v0.11.1 — fixed in plan 56-09: "Reading the output" now says `make coverage` writes `lcov.info` and prints no percentage, gives the same LF/LH summation CI runs as a copyable snippet, and says to compare its truncated value with the `coverage-summary` artifact of the CI run on the same commit; proof: the snippet prints `Lines: 165/200 = 82.50%  (truncated: 82)` on a two-record `lcov.info` (`LF:100 LH:90`, `LF:100 LH:75`), the runbook's sanity case, and the maintainer's own run of it produced C5

### DF-23 — Guide step 1 (`make services-up`) cannot run inside the project's own devcontainer, and the guide does not say the compose siblings satisfy it there
- **Found:** 2026-11-03 · **By:** maintainer · **Surface:** docs
- **Version:** 0.11.0 (`docs/src/contributing/testing-guide.md` "Local generation" at commit 21dacf0d6)
- **What happened:** the guide's "Local generation" starts with `make services-up`, which runs `docker compose` and so
  needs a Docker daemon. The devcontainer has no Docker socket, so the step cannot run there. The maintainer skipped it
  and recorded (C2):
  ```
  $ getent hosts redis rustfs
  redis-cli -h redis -p 6379 ping 2>&1 || echo "redis-cli not installed (script falls back to a TCP probe)"
  172.19.0.3      redis
  172.19.0.4      rustfs
  PONG
  ```
  Redis and RustFS were already up as siblings on `paladin-dev-network`
  (`.devcontainer/docker-compose.yml:36-37` names `redis:6379` and `rustfs:9000`), and `scripts/coverage.sh` probes
  `redis:6379` / `rustfs:9000` first, so `make coverage` found them. The guide said none of this, so a contributor in the
  devcontainer meets a step that fails and has no way to know the next one will still work. Redis answered `PONG`
  unauthenticated on 6379 there (research assumption A3, settled for the devcontainer; the bare-host case with the
  password-protected `docker/docker-compose.yml` Redis was not walked). Guide step 1 as written was therefore not
  exercised on a bare host.
- **Expected:** the guide says what to do in the devcontainer, where most contributors work.
- **Repro:** in the devcontainer, `make services-up`.
- **Severity:** S3 (D-04: a guide step that does not read as written in the project's own recommended environment)
- **Related:** C2, C6 bullet 2, traps 2 and 5 of `56-RUNBOOK-COVERAGE.md` (neither bit or was exercised)
- **Disposition:** open → **in v0.11.1** (contained docs fix: plan 56-09)
- **Disposition (2026-11-03):** in v0.11.1 — fixed in plan 56-09: the guide's Prerequisites now carry a paragraph saying that inside the devcontainer there is no Docker socket, that the compose siblings `redis:6379` and `rustfs:9000` are what `scripts/coverage.sh` probes first, that step 1 is skipped there, and how to check the siblings (`getent hosts redis rustfs`, `redis-cli -h redis ping`); proof: `grep -n 'no Docker socket' docs/src/contributing/testing-guide.md`. The bare-host `make services-up` path stays unwalked and is not claimed

### DF-24 — CI's coverage summary prints `Scope: --workspace --features integration-tests`, but the measured scope is `integration-tests,llm-all`
- **Found:** 2026-11-03 · **By:** maintainer · **Surface:** ci
- **Version:** 0.11.0 (`.github/workflows/ci.yml` at commit 21dacf0d6; the job log of run 41703614636 shows the label)
- **What happened:** the `Coverage summary` step of the `coverage` job printed (C1, from the job log of run
  `41703614636`, job `126153522434`):
  ```
  Scope: --workspace --features integration-tests (the gated measurement)
  Lines:     170767/183901 = 92.86%
  Functions: 17617/20367 = 86.50%
  ```
  The measurement it summarises runs `scripts/coverage.sh`, whose command is
  `cargo llvm-cov --workspace --features integration-tests,llm-all ...`. The label omits `llm-all`, the feature that
  brings six of nine LLM adapters into the figure, so a reader comparing a local run with the artifact is told the wrong
  scope. The artifact `coverage-summary.txt` carries the same label.
- **Expected:** the label names the scope that was measured.
- **Repro:** read the `Coverage summary` step of any `ci.yml` `coverage` job; compare with `scripts/coverage.sh:101`.
- **Severity:** S4 (D-04: a stale label; the number itself is right)
- **Related:** C6 bullet 5, trap 6 of `56-RUNBOOK-COVERAGE.md`
- **Disposition:** open → **in v0.11.1** (contained: one label line in `ci.yml`, plan 56-09)
- **Disposition (2026-11-03):** in v0.11.1 — fixed in plan 56-09: the printed `Scope:` line now reads `--workspace --features integration-tests,llm-all (the gated measurement)`; the step-summary heading two lines below (`### Coverage (workspace, --features integration-tests)`) carries the same stale scope and was left, because plan 56-09 caps its `ci.yml` edit at one line (PD-09-2); proof: `git diff --numstat -- .github/workflows/ci.yml` is `1 1`, `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` parses, `actionlint` is not installed here and the CI Workflow Lint job on push is the proof of record

### DF-25 — `make coverage-html` measures `--features integration-tests` without `llm-all`, a different scope from the gated figure the guide tells the reader to compare
- **Found:** 2026-11-03 · **By:** Claude (by reading, during the coverage walk; the maintainer did not run `make coverage-html`) · **Surface:** docs
- **Version:** 0.11.0 (`Makefile` at commit 21dacf0d6)
- **What happened:** the guide's "Local generation" lists `make coverage-html` as step 3 of the same procedure and opens
  by saying every command is the one CI runs. The Makefile recipe was:
  ```
  cargo llvm-cov --workspace --features integration-tests --html --output-dir target/coverage
  ```
  without `llm-all`, while the guide's own text on `scripts/coverage.sh` says that omitting it "silently excludes the
  other six adapters' lines from both the numerator and the denominator". The HTML report's percentage is therefore not
  comparable to the gated one. This was read from the tree; the target was not run in the walk (trap 4 not run), so no
  measured difference is claimed.
- **Expected:** the HTML report covers the same scope as the gate.
- **Repro:** `make -n coverage-html | grep llvm-cov`.
- **Severity:** S3 (D-04: the optional guide step reports a figure on a scope the guide says is wrong)
- **Related:** trap 4 of `56-RUNBOOK-COVERAGE.md`; `scripts/coverage.sh:7-16`
- **Disposition:** open → **in v0.11.1** (contained: one line in `Makefile`, plan 56-09)
- **Disposition (2026-11-03):** in v0.11.1 — fixed in plan 56-09: `coverage-html` now passes `--features integration-tests,llm-all`, and the guide says the report has the same scope as the gate and needs the same services; proof: `make -n coverage-html` prints `cargo llvm-cov --workspace --features integration-tests,llm-all --html --output-dir target/coverage`, `git diff --numstat -- Makefile` is `1 1`; the target itself was not run (it needs the services and a full instrumented build)

### DF-26 — `paladin-server` never applies the `PALADIN_TRACE_*` / `APP_AGENT_RUNTIME_*` / `APP_WEB_SERVER_*` env overlays MIGRATION §9.5 documents
- **Found:** 2026-11-03 (by reading, handed back by plan 56-12 PD-12-4; verified on HEAD 8e1a6aec2 by the orchestrator) · **By:** Claude (Phase 56) · **Surface:** server, docs
- **Version:** 0.11.0 (identical at tag v0.11.0)
- **What happened:** `TraceConfig`, `AgentRuntimeConfig` and `WebServerConfig` each implement `EnvOverridable::apply_env_overrides` (`src/config/trace.rs:257`, `agent_runtime.rs:213`, `web_server.rs:92`), and `MIGRATION.md` §9.5 documents the variables per field (`PALADIN_TRACE_CHANNEL_CAPACITY`, `PALADIN_TRACE_PERSIST`, `APP_AGENT_RUNTIME_MODEL_CALL_LIMIT_ENABLED`, …). But `Settings` has no `get_trace_config()` / `get_agent_runtime_config()` / `get_web_server_config()` wrapper (every other section — queue, file_storage, notifications, citadel, herald, treasurer — has one that calls `apply_env_overrides()`, `src/config/settings.rs:177-292`), and the server reads the three fields straight off the deserialised file: `run_api_wiring.rs:1277` (`settings.trace.clone()`), `agent_host.rs:688` and `facade_provisioner.rs:92` (`settings.agent_runtime.*`). No non-test code calls the three overlay methods. An operator who sets `PALADIN_TRACE_PERSIST=true` or `APP_AGENT_RUNTIME_TOKEN_BUDGET_ENABLED=true` as §9.5 says gets the file value, silently.
- **Expected:** either the overlays are applied on the server's boot path (before `Settings::validate()`, so the validated values are the running values), or §9.5 stops listing env variables for these three sections.
- **Repro:** `PALADIN_TRACE_CHANNEL_CAPACITY=0 paladin-server --config control.yml` boots and listens (the overlay would have made DF-08's validate refuse it); `grep -rn "get_trace_config\|get_agent_runtime_config\|get_web_server_config" src/` is empty.
- **Severity:** S2 (wrong result: documented operator controls are ignored)
- **Related:** DF-08 (the fix there judges file values only, by design — plan 56-12 P-12-2); MIGRATION §9.5; inventory §H does not cover it
- **Disposition:** open → promote **in v0.11.1** (S2, D-05; fix in Stage C, D-06 — plan 56-14)
- **Disposition (2026-11-03):** in v0.11.1 — fixed by 8859af226, 9dce25b42 (plan 56-14); regression: load_settings_applies_a_trace_env_overlay, load_settings_refuses_a_zero_trace_channel_capacity_from_the_environment, load_settings_applies_an_agent_runtime_env_overlay, load_settings_refuses_an_invalid_agent_runtime_value_from_the_environment, load_settings_applies_a_web_server_env_overlay, load_settings_leaves_file_values_when_no_overlay_is_set, section_wrappers_return_an_overlaid_copy_and_leave_settings_untouched; boot probe refuses PALADIN_TRACE_CHANNEL_CAPACITY=0 (exit 1)

### DF-27 — An adapter-wide `context_window` cannot be set below a model's maximum output, so a DeepSeek ration window of 64,000 cannot be pinned through the adapter
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** docs
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 5 item 1 and section 7 item 1 (one root), quoted below. Checked on HEAD by reading source, not by running anything (no cargo build in plan 56-10): `with_context_window` in `crates/paladin-llm/src/registry/mod.rs` validates the adapter-wide window against the adapter-wide fallback and against every seeded model's maximum output, not only the model in use; the seeded maximum output of `deepseek-flash` and `deepseek-v4-pro` is 393,216 (Switching providers table), so any window below that is refused at construction. The `context_window` field's rustdoc (`crates/paladin-llm/src/deepseek/adapter.rs`) says "Construction fails when it is below a model's maximum output", and `context_window_override_beats_a_seeded_window_on_deepseek` pins a 100,000 window being refused with `393216` in the message. So web3sec's account is right, and the refusal is deliberate, loud and never a clamp. The gap is the guide: MIGRATION.md §10.8 (frozen) offers `with_context_window(n)` as the remedy for the 16x DeepSeek ration growth (64,000 to 1,048,576) and the Switching providers page's window paragraph said nothing of a floor, so the remedy cannot produce the 64,000 it was computed from.
  ```
  1. **Window pin knob (D-06).** The intent (keep today's effective 200,000 Anthropic / 64,000 DeepSeek ration
     windows) is kept, but the knob differs: the cap is applied at the Commissary site (`build_deductive_commissary`
     replaces `Commissary::from_port`, using `pinned_ration_window`), not via adapter `context_window` or
     `model_overrides`, because the adapter-wide pin is unconstructible for DeepSeek (finding 7.1).
  ```
  and, from section 7 item 1:
  ```
  1. **DeepSeek adapter-wide window pin is unconstructible below `max_output_tokens`.** Setting
     `DeepSeekConfig.context_window = Some(64_000)` fails adapter construction (the adapter-wide knob cannot be set
     below the model's `max_output_tokens`), so a consumer cannot pin a smaller DeepSeek ration window through the
     adapter. We cap at the Commissary site instead. Covered by `deductive::pin_tests`. Consider a documented
     per-role or Commissary-level window cap.
  ```
- **Expected:** the guide says the adapter-wide window has a floor at the largest maximum output, and where to cap a ration window instead.
- **Repro:** `DeepSeekConfig::new(key, "deepseek-flash".to_string()).with_context_window(64_000)`, then `DeepSeekAdapter::new(config)`: an `Err` naming `deepseek`, the model and both numbers. Not run here; the source and the pinning test above are the evidence.
- **Severity:** S3 (D-04: misleading guide; nothing is wrong at run time, construction refuses with a named error, so it is not S2)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; `MIGRATION.md` §10.8, Library consumers, Direct-adapter bullet "The adapter-wide context window follows the seeded default model" (frozen); return report §5 item 1 and §7 item 1; `docs/src/user-guides/switching-providers.md` "Window precedence"
- **Disposition (2026-11-04):** in v0.11.1 — fixed in plan 56-10 by 0c0036714 (docs only): Switching providers gains "A window cannot be set below a maximum output", which states the check against every seeded model, the 393,216 DeepSeek floor, that `with_context_window` cannot ration a DeepSeek prompt to 64,000, that a per-model override is held to the same rule on the merged profile, and that a smaller ration window is capped with `Commissary::new` and `ProviderCapabilities { max_context_tokens }` instead of `Commissary::for_model`. MIGRATION §10.8 stays byte-frozen (D-00c), so the note lives in the page, not the register. web3sec's "per-role or Commissary-level window cap" is answered with the existing `Commissary::new` route; a per-role knob would be new API and is not added (D-00d). Proof: `grep -n 'cannot be set below a maximum output' docs/src/user-guides/switching-providers.md`; the refusal itself is unchanged and was not re-run

### DF-28 — Cadence is pacing, not a retry, but MIGRATION §10.8's pre-v0.10.0 paragraph reads as if composing it restores the 429 retries
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** docs
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 5 item 2 and section 7 item 3 (one root). Checked on HEAD: `crates/paladin-llm/src/cadence.rs` module docs, "Gate, never retry (D-01)" and "A gate beyond `max_wait` is surfaced, not slept (D-06)": the decorator waits on the gate before it delegates, never re-issues a request, and the call that received the 429 returns the inner error unchanged, so the inner call count stays 1; `CadenceSettings::default().max_wait()` and `CadenceConfig::default().max_wait_secs` are 300 s. web3sec's account of the behaviour is right. Two refinements: the 300 s applies to a gate (the wait after a 429), so a gate of 300 s or less is waited out and only a longer one surfaces `RateLimitExceeded` at once, carrying the remaining gate in `retry_after`; and the call that got the 429 always surfaces it. The wording at fault is one sentence of §10.8's pre-v0.10.0 paragraph, "both now make 0 retries ... unless Cadence is composed", which a pre-v0.10 reader takes as retries restored. Every other statement is already right: M-B-05 ("wait or surface a rate-limit error after `max_wait_secs`"), `docs/src/getting-started/configuration.md` and `docs/src/architecture/treasurer.md` ("never retried inside the pacing layer"), ADR-0058. The two places that say "unless Cadence is composed" are §10.8 (frozen) and the `DR-7` bullet of `docs/src/api-reference/upgrading.md`.
  ```
  2. **Cadence composed, no in-adapter 429 retry (D-08).** We compose Cadence as a pacing gate and add explicit
     `RateLimitExceeded` transient arms. We did NOT add a bounded 429 retry loop; the single retry stays in each
     stage's transient arm. This is pacing parity, not retry parity (finding 7.3).
  ```
  and, from section 7 item 3:
  ```
  3. **Cadence is pacing, not retry, for a pre-v0.10 consumer.** A consumer upgrading from before v0.10 had
     in-adapter bounded 429 retries (DeepSeek up to 4, Anthropic up to 3 calls, ~100 ms·2ⁿ). Composing Cadence
     restores back-pressure (gate and wait) but the failing call surfaces `RateLimitExceeded` unchanged and the inner
     call count stays 1. The migration guide should frame Cadence as the replacement for retry-on-429 only with a
     stage-level retry, and state that the default `max_wait` (300 s) surfaces `RateLimitExceeded`.
  ```
- **Expected:** a pre-v0.10 consumer is told that Cadence replaces the back-pressure of the old in-adapter retry and not the retry, and what the default `max_wait` does.
- **Repro:** read `MIGRATION.md` §10.8 "Starting before `v0.10.0`" as a pin before `v0.10.0`; compose Cadence; a 429 from the provider still comes back as `RateLimitExceeded` from that call (`crates/paladin-llm/src/cadence.rs`, "Gate, never retry"). Not run here.
- **Severity:** S3 (D-04: misleading guide wording; the behaviour is the documented one)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; `MIGRATION.md` §10.8, Library consumers, "Starting before `v0.10.0`" and the Direct-adapter Cadence bullet (frozen); M-B-05; ADR-0058; return report §5 item 2 and §7 item 3
- **Disposition (2026-11-04):** in v0.11.1 — fixed in plan 56-10 by 0c0036714 (docs only): `docs/src/api-reference/upgrading.md`, "Rate pacing and decorators", gains "Cadence is pacing, not a retry", which says to read §10.8's "unless Cadence is composed" as pacing only, that the call which got the 429 returns `RateLimitExceeded` unchanged with the provider hit once, that `RetryPolicy` and `FallbackLlmAdapter` are the only retry owners, and that a gate longer than `max_wait` (default 300 s, `treasurer.cadence.max_wait_secs`) surfaces `RateLimitExceeded` at once while a gate of `max_wait` or less is waited out. Placed in the non-frozen book page that repeats the claim, because MIGRATION §10.8 is byte-frozen (D-00c) and §11 holds v0.11.1 changes, not corrections of a v0.10.1-to-v0.11.0 reading. Proof: `grep -n 'Cadence is pacing, not a retry' docs/src/api-reference/upgrading.md`

### DF-29 — web3sec routes a typed `Refusal` to a never-retried `Refused` disposition in six stages, beyond the shape-proof arm MIGRATION §10.8 needs
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** library
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 5 item 3. Checked against HEAD: `LlmError::Refusal { provider, model, detail, .. }` is `Permanent`, and §10.8 says to match it (`LlmError::Refusal { .. }`, `#[non_exhaustive]`). Routing it to an explicit never-retried error in more stages than the guide strictly needs is a consumer design choice that follows the documented contract; nothing in the guide failed to read as written, and the report says it adds no `DegradationReason`. This entry is not a defect and is recorded only because the runbook makes every §5 divergence its own entry.
  ```
  3. **Refusal disposition spread beyond deductive.** §10.8 only needs the shape-proof arm. We route typed `Refusal`
     to an explicit never-retried `Refused` error in deductive, triage, tribunal, prover, design-doc and the armory,
     without adding a new `DegradationReason`.
  ```
- **Expected:** n/a (the consumer extended a documented arm).
- **Repro:** n/a
- **Severity:** S4 (not a defect)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; `MIGRATION.md` §10.8 "A refusal is `LlmError::Refusal { .. }`" (frozen); return report §5 item 3
- **Disposition (2026-11-04):** out (consumer adaptation choice; no Paladin change)

### DF-30 — web3sec forwards `model_profile` and `check_reasoning` in all four of its `LlmPort` decorators
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** library
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 5 item 4. This is the step MIGRATION §10.8 asks for (Direct-adapter bullet: "An `LlmPort` decorator must forward `model_profile` and `check_reasoning`"), taken in full and early, as a prerequisite for the consumer's own Phase 51 adoption. It is a divergence from the muster's scope only in that web3sec did it all at once. No Paladin change follows. Recorded because the runbook makes every §5 divergence its own entry.
  ```
  4. **Decorator forwarding.** `model_profile` and `check_reasoning` are forwarded in all four decorators now (8
     one-liners), as a prerequisite for adopting P51.
  ```
- **Expected:** n/a (the guide's step, followed).
- **Repro:** n/a
- **Severity:** S4 (not a defect)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; `MIGRATION.md` §10.8 Direct-adapter bullet on decorator forwarding (frozen); DR-21; return report §5 item 4
- **Disposition (2026-11-04):** out (consumer adaptation choice; no Paladin change; it is the step the guide asks for)

### DF-31 — web3sec's bump is one pointer-plus-adaptation-plus-lock commit followed by separate behaviour and lockfile commits, not a literal single commit
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** library
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 5 item 5, and section 1: C1 `3ad5d03f` carries the gitlink move, the 19-file compile adaptation and `Cargo.lock` in one commit, with the behaviour work as later TDD commits and a separate security lockfile commit `a2e531d0`. The hand-off's single-bump rule (`47-HANDOFF-web3sec.md` §1) asks for the pointer, the adaptation and `Cargo.lock` together in one consumer commit, which C1 is; later consumer work is outside it. So this is not a divergence from the rule, only from a literal reading of the report's own plan. Consumer process, not a Paladin matter.
  ```
  5. **Commit shape.** Not a literal single commit: C1 (pointer + compile adaptation + lock), then TDD behaviour
     commits per plan, then a separate lockfile commit `a2e531d0`.
  ```
- **Expected:** n/a
- **Repro:** n/a
- **Severity:** S4 (not a defect)
- **Related:** `47-HANDOFF-web3sec.md` §1 "The single-bump rule" and §6; return report §1 and §5 item 5
- **Disposition (2026-11-04):** out (consumer process choice; no Paladin change)

### DF-32 — web3sec swaps its DeepSeek default `deepseek-v4-flash` for `deepseek-flash` and asks which model strings are aliases and for how long they resolve
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** docs
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 5 item 6 and section 7 item 2 (one root). Checked on HEAD: `deepseek-flash` is the only seeded DeepSeek default (`DEEPSEEK_DEFAULT_MODEL`, `registry/seeded.rs`); `deepseek-v4-flash` is not seeded and runs on the adapter-wide fallback with one warning per model. The Switching providers page already says so in its "DeepSeek model ids" paragraph, which records web3sec's own 2026-10-24 finding that the live listing holds `deepseek-flash` and not `deepseek-v4-flash`, and its footnote [g] and the "Probed but not seeded" paragraph say a seeded row matches its id exactly and any other alias runs on the fallback and is true only on its verified-on date. What the report adds is a consumer-side datum (the old string answered and echoed the same backend on 2026-10-28). It carries no captured body, Paladin has not re-probed the listing, and how long a vendor keeps an alias answering is the vendor's to say, not Paladin's, so it cannot be put in the guide as a date. Choosing the seeded id is the consumer's own choice and the one the page recommends.
  ```
  6. **DeepSeek default.** Swapped `deepseek-v4-flash` → `deepseek-flash` (live-listed, seeded) rather than relying on
     the alias.
  ```
  and, from section 7 item 2:
  ```
  2. **`deepseek-v4-flash` → `deepseek-flash` alias behaviour.** The old string was an alias to the same backend (live
     echo, 2026-10-28), no longer present in the provider listing. Benchmark history rows therefore carry two strings
     for one backend. The guide should say which strings are aliases and for how long they resolve.
  ```
- **Expected:** the guide names which strings are seeded and says an alias runs on the fallback; it cannot name a vendor's alias lifetime.
- **Repro:** `grep -n 'DeepSeek model ids' -A6 docs/src/user-guides/switching-providers.md`
- **Severity:** S4 (the guide already says it; the open ask is a vendor property)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; return report §3 "Model-string note", §5 item 6 and §7 item 2; `docs/src/user-guides/switching-providers.md` "DeepSeek model ids", footnote [g]
- **Disposition (2026-11-04):** out (already documented on HEAD: Switching providers, "DeepSeek model ids" and footnote [g], D-00e; an alias's lifetime is vendor-side and a re-probe enters only with a captured body, D-00d)

### DF-33 — web3sec reports MIGRATION §10.8 tooling and cross-references still citing `9.8`-style labels
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** docs
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 7 item 4. Re-checked on HEAD, by content: every `9.8` that is a section label is correct for its context. `MIGRATION.md` `## 9.8 Upgrade checklist` is the real v0.10.0 checklist; `docs/src/api-reference/upgrading.md` cites `§9.1–§9.8` and `MIGRATION.md §9.8` only inside "Upgrading from v0.9.x to v0.10.0"; `tests/consumer_surface_muster.rs` cites section 10.8; no script, test, Makefile or workflow cites `9.8` (the only other hit is `spin 0.9.8` in `CHANGELOG.md`). Paladin's own pending todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md` says "9.8" in its early text but carries a dated note (2026-10-24) saying those mentions are history. The tooling the report means is in web3sec's workspace, not in this tree; the report names no file. If web3sec names one, it re-enters as a new finding.
  ```
  4. **MIGRATION §10.8 tooling still cites `9.8`-style section labels.** Tooling and cross-references in the guide
     and muster still use `9.8`-style labels where §10.8 is current.
  ```
- **Expected:** every cross-reference to the v0.10.1-to-v0.11.0 checklist reads `10.8`.
- **Repro:** `grep -rnE '\b9\.8\b' --include=*.md --include=*.rs --include=*.sh --include=*.toml --include=*.yml --include=Makefile .` outside `.planning`, `.project` and `target`: four hits (the real `## 9.8` heading, two citations inside the book's v0.9.x-to-v0.10.0 section, and `spin 0.9.8` in `CHANGELOG.md`), all correct or unrelated.
- **Severity:** S4 (does not reproduce on HEAD)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; return report §7 item 4; D-00e
- **Disposition (2026-11-04):** out (does not reproduce on HEAD: the only `9.8` section labels are the real §9.8 and the v0.9.x-to-v0.10.0 section of the book's Upgrading page, D-00e; the tooling named is in web3sec's own tree, not Paladin's)

### DF-34 — Intermittent DeepSeek empty completion under a tight `max_tokens`: web3sec asks for a reasoning-budget note and a budget-aware default or reasoning-share hint
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** docs
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 7 item 5, and section 3 (tare control: `Empty completion: finish_reason=length ... reasoning likely consumed the entire max_tokens budget` on one intermittent DeepSeek call, failed in r1 and r2, succeeded in r3; no body captured). Checked on HEAD: the requested note already exists. `docs/src/user-guides/reasoning-effort.md`, "Effort, output budget and timeout", says reasoning tokens are output tokens, that the budget must cover the reasoning and the answer, and that a call which spends the whole cap thinking finishes `length` with no text and returns `LlmError::EmptyCompletion`, a runtime backstop and not a validation. The error text itself advises "retry with a larger max_tokens" (`crates/paladin-llm/src/response_guards.rs`). v0.11.0 made the condition typed on purpose (M-B-16). What is not on HEAD is the other half of the ask, a budget-aware default or hint, which is new behaviour with a design question (warn, refuse or raise an explicit budget) in a no-new-features patch, and web3sec gave an error string, not a captured body.
  ```
  5. **Intermittent DeepSeek reasoning-budget empty-completion under a tight `max_tokens` budget.** web3sec's
     pre-disclosed-match and triage calls to `deepseek-flash` intermittently return `Empty completion:
     finish_reason=length … reasoning likely consumed the entire max_tokens budget` (failed in 2 of 3 post-bump
     repeats, succeeded in 1). v0.11.0's typed `EmptyCompletion` makes the failure visible and mappable to
     `TruncatedCompletion`, but it moves a previously silent condition into benchmark variance. Worth a Paladin-side
     note on reasoning-model budgets, and a possible budget-aware default or reasoning-share hint for DeepSeek.
  ```
- **Expected:** the guide covers reasoning-model budgets (it does); a budget-aware default is a later design.
- **Repro:** n/a (intermittent, vendor-side; not reproduced by Paladin).
- **Severity:** S4 (the docs half is already on HEAD; the rest is an enhancement request)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; M-B-16; return report §3 and §7 item 5; CONTEXT D-00d
- **Disposition (2026-11-04):** out (re-homed: .planning/todos/pending/2026-11-04-reasoning-budget-hint-for-deepseek-empty-completion.md; the note web3sec asks for is already in reasoning-effort.md "Effort, output budget and timeout" and the error text, D-00e)

### DF-35 — web3sec's own `triage_llm_it` test asserts a `Severity:` preamble that its FU-4 commit stripped
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** library
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 7 item 6, labelled by the report itself "not a Paladin finding (FYI)". The test and the commit are in web3sec's tree and cannot be checked here; nothing in the report points at Paladin code, and it says Phase 50 touched neither `into_finding_draft` nor the test.
  ```
  6. **Stale consumer test, not a Paladin finding (FYI).** `crates/audit-agents/tests/triage_llm_it.rs:215`
     asserts `finding.hypothesis.contains("Severity:")`, which contradicts FU-4 (commit `8d242970`, which stripped the
     Severity/Difficulty preamble). The test is `#[ignore]`d, predates Phase 50, and Phase 50 touched neither
     `into_finding_draft` nor the test. It is not a model drift or a v0.11.0 regression. Cleanup todo:
     `.planning/todos/pending/2026-10-29-stale-triage-llm-it-severity-preamble-assertions.md`.
  ```
- **Expected:** n/a
- **Repro:** n/a
- **Severity:** S4 (not a defect in Paladin)
- **Related:** return report §7 item 6
- **Disposition (2026-11-04):** out (consumer-side; not a Paladin finding, per the report itself)

### DF-36 — The consumer-surface muster does not pin the three Cadence import paths web3sec's Phase 50 code takes
- **Found:** 2026-10-30 (report), relayed 2026-11-04 · **By:** web3sec · **Surface:** library
- **Version:** 0.11.0 (the report pins `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`)
- **What happened:** the return, section 4. Of the five "new paths" it lists, `LlmError::Refusal { .. }` and `LlmError::RateLimitExceeded { .. }` are already pinned by `tests/consumer_surface_muster.rs`; `paladin::infrastructure::cadence::build_cadence`, `paladin::infrastructure::cadence::with_cadence` and `paladin::config::treasurer::CadenceConfig` are not (`grep -c` on each name in the muster is 0). All three resolve on HEAD (`src/infrastructure/cadence.rs` re-exports `with_cadence` and defines `build_cadence`; `src/config/treasurer.rs` defines `CadenceConfig`) and §10.8's Cadence recipe names them. They are not unguarded: the `build_cadence` doctest compiles `CadenceConfig` and `build_cadence`, and `.project/current-exports.txt`, the `make api-surface` baseline, lists all three. So the surface cannot move unnoticed; the muster, the file that exists to mirror the consumer's imports, simply does not say so.
  ```
  Paladin P46 already pinned the four web3sec muster gaps in PALADIN-FEEDBACK-P46.md; the remaining "pinned but
  unused" muster items are forward-looking adoption pins. The web3sec `use paladin::` / `paladin_ports::` /
  `paladin_memory::` inventory (73 paths at the dogfood) was re-run after the behaviour work. **New paths** in
  consumer code since the dogfood:

  | Path | Introduced by | Use |
  |---|---|---|
  | `paladin::infrastructure::cadence::build_cadence` | 50-06 | one process-wide Cadence per run |
  | `paladin::infrastructure::cadence::with_cadence` | 50-06 | wraps all ten `build_*_llm` adapters |
  | `paladin::config::treasurer::CadenceConfig` | 50-06 | `CadenceConfig::default()` |
  ```
- **Expected:** the muster pins every path the consumer imports (runbook: every real `use` line the muster missed is its own entry).
- **Repro:** `grep -c 'build_cadence\|with_cadence\|CadenceConfig' tests/consumer_surface_muster.rs` prints 0.
- **Severity:** S3 (D-04: a muster gap with no behavioural effect)
- **Related:** `47-HANDOFF-web3sec.md` §6 item 4; `tests/consumer_surface_muster.rs`; `.project/current-exports.txt`; return report §4
- **Disposition (2026-11-04):** out (re-homed: .planning/todos/pending/2026-11-04-muster-pin-cadence-consumer-imports.md; adding the pins is a one-file test change whose proof is a facade compile, which this docs-only plan does not run, and the paths are already guarded by the `build_cadence` doctest and the api-surface baseline)
