---
phase: 56-dogfood-findings
plan: 12
subsystem: dogfood-findings
tags: [config, boot-validation, trace, agent-runtime, otel, df-08, dogf-06, stage-c, gap-closure]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-05 Lead L6 evidence (repro and otel probe), DF-08 findings-log entry"
provides:
  - "src/bin/paladin-server.rs: private load_settings(config_path) -> Result<Settings, Box<dyn Error>>, called first by run(): load, treasurer check (unchanged, first), Settings::validate() mapped to `invalid configuration: <key> ...`"
  - "Five load_settings regression tests and an example_with helper (three refusal tests watched failing first)"
  - "CHANGELOG [Unreleased] ### Changed bullet, MIGRATION 11.1 row M-B-37 and 11.8 step 9"
  - "DF-08 closed in the findings log (in v0.11.1, fixed by 59bb84e0d)"
affects: [56-13, 56-verification, SHIP-10]

tech-stack:
  added: []
  patterns:
    - "One load helper owns every config-level boot refusal, so a refusal reaches main()'s `paladin-server failed to start` line before any store, provider or listener exists"
    - "example_with(edits): each edit target is asserted to occur exactly once in config.example.yml, so a regression test can never pass by silently missing its edit"

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-12-SUMMARY.md
  modified:
    - src/bin/paladin-server.rs
    - CHANGELOG.md
    - MIGRATION.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "Placement and mapping (PD-12-1): validate() runs inside load_settings, right after the treasurer check; prefix `invalid configuration: `"
  - "Double validation kept (PD-12-2): the treasurer check stays first and byte-identical; Settings::validate() re-runs it as a pure no-op; agent_runtime is validated only here"
  - "Binary-only, behavioural (D-00b): MIGRATION 11.1 and 11.8 plus CHANGELOG [Unreleased], never 11.2; no library file changed"

patterns-established:
  - "otel refusal test plants a bearer-shaped sentinel in otel.headers and asserts it is absent from the refusal text"

requirements-completed: []

duration: ~30min
completed: 2026-11-03
---

# Phase 56 Plan 12: DF-08 boot-time whole-config validation Summary

**`paladin-server` now calls `Settings::validate()` at boot, through a private `load_settings` helper that runs before anything is built, so `trace.channel_capacity: 0`, `trace.otel.enabled: true` without the `otel` feature and an invalid `agent_runtime` value stop the server on `paladin-server failed to start: invalid configuration: <key> ...` instead of booting into clamped or silent behaviour.**

Plan-start SHA: `4a8d8a7f6e524bfbc9155a9175a77e03ea727f1c`. Free disk at start: 3561M on `/` (low point 2766M; no ENOSPC, `target/debug/incremental` never deleted). Built with `--features cli,web-server` only, so the `otel` feature was off and the `#[cfg(not(feature = "otel"))]` test ran.

Requirements: DOGF-06 deliberately NOT marked complete (DF-19 is still open after this plan); DOGF-01 was already complete.

Commit trailers: the orchestrator named `Claude Fable 5.1`; this executor is Sonnet 5.5, so the commits carry the harness-provided `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and the same `Claude-Session` line.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | `59bb84e0d` | fix(56-12): paladin-server validates the whole config at boot (DF-08) |
| 2 (records) | `30a8126b7` | docs(56-12): DF-08 CHANGELOG and MIGRATION M-B-37 (DF-08) |
| 2 (log) | `c54d784dd` | docs(56-12): DF-08 disposition in v0.11.1 (DF-08, DOGF-06) |

## Red-then-green record

Step 2 refactor (move `Settings::load_from_file` and the treasurer check into `load_settings`, no behaviour change), then the five tests, then:

`cargo test -p paladin-ai --features cli,web-server --bin paladin-server load_settings` (RED, `load_settings` returned `Ok`):

```
test tests::load_settings_accepts_the_shipped_example_config ... ok
test tests::load_settings_keeps_the_treasurer_refusal_first ... ok
test tests::load_settings_refuses_a_zero_trace_channel_capacity ... FAILED
test tests::load_settings_refuses_an_invalid_agent_runtime_value ... FAILED
test tests::load_settings_refuses_otel_enabled_without_the_otel_feature ... FAILED
test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 21 filtered out
```

After adding `settings.validate().map_err(|e| format!("invalid configuration: {e}"))?;` (GREEN):

```
test tests::load_settings_accepts_the_shipped_example_config ... ok
test tests::load_settings_keeps_the_treasurer_refusal_first ... ok
test tests::load_settings_refuses_a_zero_trace_channel_capacity ... ok
test tests::load_settings_refuses_an_invalid_agent_runtime_value ... ok
test tests::load_settings_refuses_otel_enabled_without_the_otel_feature ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out
```

The three refusal tests failed first; the example and treasurer tests are guards that pass before and after.

## Boot probe

Real binary built from HEAD (`cargo build -p paladin-ai --bin paladin-server --features cli,web-server`), run as DF-08's Repro runs it: `env -u RUST_LOG OPENAI_API_KEY=test-key-not-a-secret-df08 PALADIN_CONFIG=./<file> timeout 20 target/debug/paladin-server` in a `mktemp -d` directory outside the repo (deleted afterwards). `bad.yml` differs from `config.example.yml` only in `channel_capacity: 0`; `control.yml` only in `server.host: "127.0.0.1"`, `server.port: 45209` and `http.auth.enabled: false`.

`bad`: exit 1, refusal line once, `listening on` count 0.

```
[2026-11-03T23:02:41Z INFO  paladin_server] Loading configuration from './bad.yml'
[2026-11-03T23:02:41Z ERROR paladin_server] paladin-server failed to start: invalid configuration: trace.channel_capacity must be greater than 0
exit=1
```

`control`: `listening on` count 1, exit 124 (from `timeout`), no retry with the 56-05 run-store variables needed.

```
[2026-11-03T23:02:41Z INFO  paladin_server] Loading configuration from './control.yml'
[2026-11-03T23:02:41Z INFO  paladin_server] Rate pacing enabled (treasurer.cadence.backend: in_process)
...
[2026-11-03T23:02:41Z INFO  paladin_server] paladin-server listening on http://127.0.0.1:45209 — serving 2 agent(s): ["researcher", "summarizer"]
...
[2026-11-03T23:03:01Z INFO  paladin_server] received SIGTERM; shutting down
exit=124
```

## Gates

Each gate preceded by `df -BM --output=avail /` (3561M, 3039M, 3028M, 2766M, never below 2766M).

- `cargo fmt --all -- --check`: clean.
- `cargo clippy -p paladin-ai --features cli,web-server --bin paladin-server -- -D warnings`: exit 0, `Finished dev profile`.
- Same with `--profile test`: exit 0, `Finished test profile`. Because `--profile test` does not enable `cfg(test)`, the same command with `--tests` was also run to lint the new test code: exit 0.
- `make api-surface`: `API surface unchanged` (4325 items).
- `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-ai`: `223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`. It ran to completion (not a NOT RUN).
- `make check-gates`: exit 0; ends `OK: scanned 625 file(s) (19 test-only), 0 finding(s)`; the MIGRATION 11.2 register and allowlist remain set-equal.
- `git diff --quiet 4a8d8a7f6 -- Cargo.toml Cargo.lock crates/*/Cargo.toml src/lib.rs src/config`: true (no dependency or library change, P-12-1).
- Findings log: `git diff --numstat` for the disposition commit `1 0` (0 deletions); `./scripts/check-findings-log.sh` ends `.project/v0.11.1/02-dogfood-findings-log.md:478: DF-19: OPEN (the last Disposition bullet reads 'open')`, naming exactly DF-19.
- Records: CHANGELOG diff `2 0`, MIGRATION diff `2 0`; hunks sit only in `[Unreleased]`, section 11.1 and section 11.8. Section 11.2, parts 9 and 10 and `[0.11.0]` untouched.
- `pre-commit run trailing-whitespace` and `end-of-file-fixer` (one invocation each) passed on CHANGELOG.md, MIGRATION.md and the log.
- No `unwrap()`, `expect(` or `panic!` added outside `mod tests`.

## Deviations from Plan

None. The plan executed as written. One ordering note: I first wrote the `settings.validate()` call together with the refactor, then removed it before the RED run so the red run is genuine; the committed history contains only the final state.

## Authentication gates

None.

## Known Stubs

None.

## Threat Flags

None. The change adds no endpoint, auth path or file access. The refusal line echoes only validator text (key names, scheme, currency), and the otel test pins that an `otel.headers` value never appears (T-56-46).

## Handed back

For the orchestrator to verify and log as DF-26 (next free id); this plan adds no DF entry (D-01, D-04, D-00e, P-12-2):

PD-12-4 (file value only). No production code calls `TraceConfig::apply_env_overrides`, `AgentRuntimeConfig::apply_env_overrides` or `WebServerConfig::apply_env_overrides` on the server's `Settings`. So `validate()` judges exactly the values the server runs on. The overlays MIGRATION.md section 9.5 documents (`PALADIN_TRACE_*`, `APP_AGENT_RUNTIME_*`) are therefore never applied. That is a candidate new finding, handed back to the orchestrator for the log (D-01, D-04, D-00e) and not fixed here (P-12-2).

(Executor spot check: a grep for non-test callers of those three `apply_env_overrides` on the server's `Settings` found none; the only hits are the sub-config fan-out inside `agent_runtime.rs` and a call inside `trace.rs`'s own tests. The `load_settings` rustdoc records the same fact.)

## Stage C inputs

None new. DF-08 is closed; DF-19 remains the only open finding; the `paladin` binary is deliberately not validated (PD-12-3).

## Self-Check: PASSED

- `src/bin/paladin-server.rs` contains `fn load_settings`, `settings.validate()`, `invalid configuration: {e}` and `load_settings(&config_path)`; found.
- Commits `59bb84e0d`, `30a8126b7`, `c54d784dd` found in `git log`.
- MIGRATION row `| M-B-37 |`, CHANGELOG `[Unreleased]` DF-08 bullet and DF-08 `(plan 56-12)` disposition found.
