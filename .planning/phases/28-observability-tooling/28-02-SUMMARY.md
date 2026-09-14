---
phase: 28-observability-tooling
plan: 02
subsystem: infra
tags: [config, serde, thiserror, otel, env-vars, redaction]

# Dependency graph
requires:
  - phase: 28-observability-tooling (plan 01, parallel wave)
    provides: "TraceEvent/TraceSink types this config's fields (channel_capacity, persist, state_values, heartbeat_interval_secs) size and gate — consumed by 28-01, not depended on by this plan"
provides:
  - "TraceConfig (Default/validate/validate_typed/EnvOverridable) in src/config/trace.rs — the runtime trace pipeline's dispatcher capacity, log-sink switch, state-value redaction/cap, heartbeat rate limit, and OTLP export config"
  - "OtelConfig with a manual, header-redacting Debug impl — never derives Debug"
  - "TraceConfigError (ZeroChannelCapacity/ZeroValueCapBytes/EndpointNotHttp/FeatureNotCompiled), a typed, non_exhaustive error enum"
  - "Settings.trace: TraceConfig and Settings.web_server: WebServerConfig, both #[serde(default)], both validated from Settings::validate()"
  - "WebServerConfig/DevUiConfig (new types) carrying web_server.dev_ui.mermaid_url, defaulting to the jsDelivr mermaid@11 ESM bundle URL"
  - "PALADIN_TRACE_* env-var family (9 vars) and APP_WEB_SERVER_DEV_UI_MERMAID_URL"
  - "Documented trace: and web_server: sections in config.example.yml and config.test.yml"
affects: ["28-03 (heartbeat rate limit)", "28-06 (log-sink default-on, dispatcher capacity)", "28-09 (OTLP endpoint/headers, the otel Cargo feature itself)", "28-11 (persistence switch)", "28-15 (inspector's mermaid_url)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Container-level #[serde(default)] on a config struct (backed by a manual impl Default) instead of per-field #[serde(default = \"fn\")] attributes — lets a partially-specified YAML section (e.g. only channel_capacity set) resolve every other field to its default"
    - "validate() -> Result<String> as a thin wrapper over validate_typed() -> Result<TraceConfigError>, composing into Settings::validate()'s unchanged Result<(), String> signature while giving library callers a typed error to match on (X-06)"
    - "Manual Debug impl that redacts a BTreeMap<String,String>'s values, keeping the keys — same shape as HeartbeatHandle's manual Debug in paladin-core"

key-files:
  created:
    - src/config/trace.rs
  modified:
    - src/config/mod.rs
    - src/config/settings.rs
    - src/config/web_server.rs
    - src/config/user_config.rs
    - config.example.yml
    - config.test.yml

key-decisions:
  - "WebServerConfig did not pre-exist in the codebase despite the plan (and D-36/D-39) treating it as pre-existing — created it fresh in src/config/web_server.rs (Rule 3: blocking issue, in-scope file) with just the one field this plan needs (dev_ui), mirroring the house Default/validate/EnvOverridable shape"
  - "Task 1's tracer-gate verify was re-run and confirmed green before proceeding to Task 2/3 (autonomous branch of the tracer feedback gate) rather than pausing for interactive human-verify, because this plan is autonomous:true, running as a worktree-parallel wave agent with no human present to answer a checkpoint"
  - "example_config_still_loads_and_validates does not call Settings::load_from_file(\"config.example.yml\") end-to-end; it uses a minimal Wrapper struct deserializing only trace: and web_server:, following the identical precedent already established in src/config/agent_runtime.rs's own test module — Settings::load_from_file on that file hits a pre-existing, documented, out-of-scope gap (llm.ollama deliberately omits api_key, but LlmProviderConfig::api_key is a required String) unrelated to this plan"
  - "cfg!(feature = \"otel\") is used before the otel Cargo feature itself exists (28-09's scope); a scoped #[allow(unexpected_cfgs)] documents why, rather than declaring the feature early in Cargo.toml (out of this plan's file scope) or suppressing the lint workspace-wide"

patterns-established:
  - "Config structs with a nested credential-shaped map (OtelConfig.headers) never derive Debug; a hand-written impl prints keys only, replacing every value with a fixed marker"

requirements-completed: [OBS-01, OBS-02]

coverage:
  - id: D1
    description: "TraceConfig deserializes from an absent trace: section with documented defaults (log_sink=true, channel_capacity=1024, persist=false, state_values=false, value_cap_bytes=256, heartbeat_interval_secs=5, otel.enabled=false), and a partial trace: section overriding one field resolves every other field to its default"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/config/trace.rs#config::trace::tests::trace_section_partial_override_applies_and_defaults_rest"
        status: pass
      - kind: unit
        ref: "src/config/trace.rs#config::trace::tests::absent_trace_section_deserializes_to_default"
        status: pass
    human_judgment: false
  - id: D2
    description: "TraceConfig::validate_typed rejects channel_capacity==0, value_cap_bytes==0, a non-http(s) otel.endpoint when otel.enabled, and otel.enabled on a build without the otel feature, each as a distinct typed TraceConfigError variant; Settings::validate() composes trace.validate() unchanged in signature"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/config/trace.rs#config::trace::tests::validate_typed_rejects_each_invalid_case_distinctly"
        status: pass
      - kind: unit
        ref: "src/config/trace.rs#config::trace::tests::validate_rejects_zero_channel_capacity"
        status: pass
    human_judgment: false
  - id: D3
    description: "OtelConfig's Debug output never contains a header value — only header keys and a fixed redaction marker"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/config/trace.rs#config::trace::tests::otel_debug_redacts_header_values"
        status: pass
    human_judgment: false
  - id: D4
    description: "All nine PALADIN_TRACE_* env vars apply through EnvOverridable, changing exactly those TraceConfig/OtelConfig fields"
    requirement: "OBS-01"
    verification:
      - kind: unit
        ref: "src/config/trace.rs#config::trace::tests::trace_env_overrides_apply"
        status: pass
    human_judgment: false
  - id: D5
    description: "web_server.dev_ui.mermaid_url defaults to the jsDelivr mermaid@11 ESM bundle URL, is settable via YAML and APP_WEB_SERVER_DEV_UI_MERMAID_URL, and both config.example.yml and config.test.yml document the full trace: and web_server.dev_ui: sections without breaking Settings deserialization"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/config/web_server.rs#config::web_server::tests::default_mermaid_url_is_the_jsdelivr_bundle"
        status: pass
      - kind: unit
        ref: "src/config/web_server.rs#config::web_server::tests::partial_dev_ui_section_overrides_and_absent_defaults"
        status: pass
      - kind: unit
        ref: "src/config/web_server.rs#config::web_server::tests::example_config_still_loads_and_validates"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-09-08
status: complete
---

# Phase 28 Plan 02: Trace/Web-Server Config Home Summary

**`TraceConfig`/`OtelConfig` (house Default+validate+EnvOverridable shape, redacting `Debug`, typed `TraceConfigError`) and a newly-created `WebServerConfig.dev_ui.mermaid_url`, both wired into `Settings` and documented in both shipped YAML files.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-09-08T22:14:33Z
- **Tasks:** 3
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments
- `src/config/trace.rs`: `TraceConfig` (7 fields, all `#[serde(default)]` at the container level) and nested `OtelConfig`, mirroring `RunStreamConfig`'s Default/validate/EnvOverridable shape
- `OtelConfig` never derives `Debug` — a manual impl prints header keys only, replacing every value with `<redacted>` (security instructions, D-36)
- `TraceConfigError` (`ZeroChannelCapacity`/`ZeroValueCapBytes`/`EndpointNotHttp`/`FeatureNotCompiled`), `#[non_exhaustive]`, backing `validate_typed()`; `validate()` stringifies it so `Settings::validate()`'s `Result<(), String>` signature is unchanged (X-03)
- `EnvOverridable for TraceConfig` reads all nine documented `PALADIN_TRACE_*` variables
- `WebServerConfig`/`DevUiConfig` created fresh in `src/config/web_server.rs` (did not pre-exist despite plan assumptions), carrying `web_server.dev_ui.mermaid_url` defaulting to the jsDelivr `mermaid@11` ESM bundle, overridable via `APP_WEB_SERVER_DEV_UI_MERMAID_URL`
- `Settings` gains `#[serde(default)] trace: TraceConfig` and `#[serde(default)] web_server: WebServerConfig`; `Settings::validate()` composes both
- `config.example.yml` and `config.test.yml` both document the full `trace:` and `web_server.dev_ui:` sections at their defaults (example file fully commented, including the state_values redaction/cap/never-reaches-SSE note)

## Task Commits

Task 1 (tracer) and Tasks 2–3 (tdd="true") were each committed atomically, with TDD tasks split into RED/GREEN commits:

1. **Task 1: end-to-end `trace:` config → validated `Settings.trace`** - `48f97fcf` (feat)
2. **Task 2: `OtelConfig` redaction, typed errors, `PALADIN_TRACE_*` overrides**
   - RED - `2ab40417` (test)
   - GREEN - `8586e252` (feat)
3. **Task 3: `web_server.dev_ui.mermaid_url` and documented YAML sections**
   - RED - `94476848` (test)
   - GREEN - `ed61149c` (feat)

**Plan metadata:** (this commit, following)

_Task 1 is `type="tracer"`: committed like `type="auto"`, then its own `<verify>` was re-run and confirmed green before Task 2 began (see Deviations)._

## Files Created/Modified
- `src/config/trace.rs` - `TraceConfig`, `OtelConfig`, `TraceConfigError`, all tests (new file)
- `src/config/web_server.rs` - `DevUiConfig`, `WebServerConfig` added alongside pre-existing `SourceConfig`/`ServerConfig`/`MessageServiceSettings`
- `src/config/settings.rs` - `Settings.trace`, `Settings.web_server` fields; `validate()` composes both
- `src/config/mod.rs` - `pub mod trace;` + re-exports (`TraceConfig`, `OtelConfig`, `TraceConfigError`, `DevUiConfig`, `WebServerConfig`)
- `src/config/user_config.rs` - `create_test_settings()` test helper updated for two new required `Settings` fields (blocking-compile fix, Rule 3)
- `config.example.yml` - full documented `trace:` and `web_server.dev_ui:` sections
- `config.test.yml` - full `trace:` section (channel_capacity overridden) and `web_server.dev_ui:` section

## Decisions Made
- `WebServerConfig` did not pre-exist in the codebase (despite the plan's `<interfaces>`/D-36/D-39 treating it as pre-existing at v0.9.0) — created it fresh, in-scope, with the house shape, carrying only `dev_ui` for now
- Followed `agent_runtime.rs`'s established `Wrapper`-deserialization precedent for the `config.example.yml` round-trip test rather than fixing the unrelated `llm.ollama`/`LlmProviderConfig::api_key` gap that blocks `Settings::load_from_file` on that file today
- Used a scoped `#[allow(unexpected_cfgs)]` for `cfg!(feature = "otel")` since the `otel` Cargo feature itself is 28-09's scope to declare

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `WebServerConfig` did not exist — created it in-scope**
- **Found during:** Task 3
- **Issue:** The plan's `<action>` for Task 3 says "extend that struct's `EnvOverridable`" and D-36/D-39/project_execution_rules all describe `WebServerConfig` as pre-existing at v0.9.0. It does not exist anywhere in the codebase (confirmed via full-tree grep); only `ServerConfig`/`SourceConfig`/`MessageServiceSettings` exist in `src/config/web_server.rs`. A prior Milestone 6 planning doc ("Decompose App Settings") proposed it but it was apparently never built.
- **Fix:** Created `WebServerConfig` fresh in `src/config/web_server.rs`, house-shaped (`Default`/`validate`/`EnvOverridable`), carrying just `dev_ui: DevUiConfig` — the field this plan needs. Wired `#[serde(default)] pub web_server: WebServerConfig` into `Settings`, composed into `Settings::validate()`.
- **Files modified:** src/config/web_server.rs, src/config/settings.rs, src/config/mod.rs
- **Verification:** `cargo test -p paladin-ai --lib config::web_server` (3/3 pass); `cargo clippy -p paladin-ai --all-targets --all-features -- -D warnings` clean
- **Committed in:** 94476848 (RED), ed61149c (GREEN)

**2. [Rule 3 - Blocking] `user_config.rs`'s test helper missing new required `Settings` fields**
- **Found during:** Task 1 (recurred at Task 3)
- **Issue:** `src/config/user_config.rs`'s `create_test_settings()` builds `Settings { ... }` via a full struct literal (no `..Default::default()` spread). Adding `Settings.trace` (Task 1) and later `Settings.web_server` (Task 3) as new non-`Option` fields broke this literal's compilation (`E0063: missing field`).
- **Fix:** Added `trace: crate::config::TraceConfig::default()` and, in Task 3, `web_server: crate::config::WebServerConfig::default()` to the literal. Checked the other two `Settings { ... }` literal sites in the tree (`facade_provisioner.rs`, `agent_host.rs`) — both already use `..Settings::default()`/`..Default::default()` spreads and needed no change.
- **Files modified:** src/config/user_config.rs
- **Verification:** `cargo check --workspace --all-targets --all-features` clean
- **Committed in:** 48f97fcf (Task 1's field), 94476848 (Task 3's field)

**3. [Rule 3 - Blocking] `cfg!(feature = "otel")` triggers `unexpected_cfgs` under `-D warnings`**
- **Found during:** Task 1
- **Issue:** The `otel` Cargo feature does not exist yet (28-09's scope to declare); rustc's `--check-cfg` lint flags any reference to an undeclared feature name as unexpected, which `-D warnings` promotes to a hard error, blocking the required clippy/build gates.
- **Fix:** Added a scoped `#[allow(unexpected_cfgs)]` on the single `cfg!(...)` binding inside `validate_typed`, with a comment explaining this is expected until 28-09 registers the feature. Did not touch `Cargo.toml` (out of this plan's file scope; would conflict with 28-09's ownership of that feature declaration).
- **Files modified:** src/config/trace.rs
- **Verification:** `cargo clippy -p paladin-ai --all-targets --all-features -- -D warnings` clean
- **Committed in:** 48f97fcf

---

**Total deviations:** 3 auto-fixed (all Rule 3 — blocking compile/lint issues directly caused by this plan's own additions)
**Impact on plan:** All three were necessary to keep the workspace compiling and lint-clean; none touched code outside this plan's declared scope except the two other `Settings` struct-literal test-helper call sites checked (and found already safe). No scope creep.

## Issues Encountered
- The tracer feedback gate (Task 1, `type="tracer"`) calls for either an interactive `checkpoint:human-verify` or, in autonomous mode, a re-run-and-continue. `.planning/config.json` shows `_auto_chain_active: false` and no `auto_advance` key, but this plan runs `autonomous: true` as a spawned worktree-parallel wave agent with no interactive human available to answer a checkpoint mid-wave. Re-ran Task 1's own `<verify>` (green) and proceeded directly to Task 2, consistent with the autonomous branch of the gate and this executor's `return_contract` (which expects a single `PLAN COMPLETE`, not a mid-plan pause).
- `Settings::load_from_file("config.example.yml")` fails today on a pre-existing, out-of-scope gap (`llm.ollama`'s deliberately-absent `api_key` vs. `LlmProviderConfig::api_key: String`), already documented and worked around in `agent_runtime.rs`'s own test module. Not fixed here (scope boundary) — Task 3's `example_config_still_loads_and_validates` test uses the same `Wrapper`-deserialization technique instead. No `WINDOWS.md` entry needed (pre-existing and already tracked by the `agent_runtime.rs` precedent's own comment).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `Settings.trace` and `Settings.web_server` are live, validated, and documented — every later plan reading `TraceConfig`/`OtelConfig`/`DevUiConfig` (28-01, 28-03, 28-06, 28-09, 28-11, 28-15) has a real, tested home to read from.
- `TraceConfigError::FeatureNotCompiled` will start actually firing once 28-09 declares the `otel` Cargo feature — no further change needed here.
- `WebServerConfig` is new: any later plan adding another `web_server`-scoped setting extends this struct rather than creating a second one.

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-08*

## Self-Check: PASSED

- FOUND: src/config/trace.rs
- FOUND: src/config/web_server.rs
- FOUND: commit 48f97fcf (Task 1)
- FOUND: commit 2ab40417 (Task 2 RED)
- FOUND: commit 8586e252 (Task 2 GREEN)
- FOUND: commit 94476848 (Task 3 RED)
- FOUND: commit ed61149c (Task 3 GREEN)
