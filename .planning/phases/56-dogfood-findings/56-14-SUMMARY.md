---
phase: 56-dogfood-findings
plan: 14
subsystem: dogfood-findings
tags: [paladin-server, config, env-overlay, settings, trace, agent-runtime, web-server, df-26, dogf-06, stage-c, gap-closure]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-12 load_settings + boot-time Settings::validate() (DF-08); 56-13 (DF-19) closed, clean tree"
provides:
  - "Settings::get_trace_config / get_agent_runtime_config / get_web_server_config: house-shape accessors (clone + apply_env_overrides), each with a doctest"
  - "load_settings writes the three overlaid sections back before the treasurer check and Settings::validate()"
  - "Six bin regression tests + one lib contract test, all #[serial], env restored in Drop"
  - "MIGRATION M-B-38 (11.1), 11.5 bullet, 11.8 step 10; M-B-37 sentence amended; CHANGELOG Added + Fixed bullets"
  - "DF-26 closed in the findings log; check-findings-log.sh prints OK"
affects: [56-10, 56-verification]

tech-stack:
  added: []
  patterns:
    - "Env-touching tests: #[serial] + a scoped helper whose restore runs in Drop, so a failed assertion cannot leak a variable"
    - "Server boot judges the overlaid (running) values, not the file values"

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-14-SUMMARY.md
  modified:
    - src/config/settings.rs
    - src/bin/paladin-server.rs
    - .project/current-exports.txt
    - CHANGELOG.md
    - MIGRATION.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "PD-14-2: house accessors plus write-back in load_settings; the direct readers in run_api_wiring / agent_host / facade_provisioner are unchanged and receive overlaid values"
  - "PD-14-3: write-backs sit immediately after load_from_file, so the treasurer refusal stays first and byte-identical (an overlay cannot fail)"
  - "PD-14-4: with_overlay_env helper restores the prior environment in Drop"

patterns-established:
  - "Settings::get_*_config(&self) is the single way library code reads a section with its documented env overlay"

requirements-completed: [DOGF-06]

duration: ~1 session
completed: 2026-11-03
---

# Phase 56 Plan 14: DF-26 env overlays on the server Summary

**`paladin-server` now applies the documented `PALADIN_TRACE_*`, `APP_AGENT_RUNTIME_*` and `APP_WEB_SERVER_*` overlays through three new `Settings::get_*_config()` accessors, written back inside `load_settings` before DF-08's `validate()`, so `PALADIN_TRACE_CHANNEL_CAPACITY=0` is refused at boot instead of booted.**

Plan-start SHA: `5168738f8d20d98e4517e0ebec80815c4e3a3b17` (HEAD at start; free disk 4319 MB).

Requirements: DOGF-06 is marked complete by this plan. Every promoted S1/S2 finding (DF-07, DF-08, DF-19, DF-26) is now fixed with a regression test and `./scripts/check-findings-log.sh` prints OK. DOGF-01 was already complete.

Commit trailers: the orchestrator named `Claude Fable 5.1`; this executor is Sonnet 5.5, so every commit carries the harness-provided `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and the same `Claude-Session` trailer.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | `8859af226` | fix(56-14): paladin-server applies PALADIN_TRACE_* overlays before validating the config (DF-26) |
| 2 | `9dce25b42` | feat(56-14): Settings gains get_trace/agent_runtime/web_server_config; paladin-server applies all three overlays before validating (DF-26) |
| 3a | `6f92bc18e` | docs(56-14): DF-26 CHANGELOG and MIGRATION M-B-38 (DF-26) |
| 3b | `2f5f4344a` | docs(56-14): DF-26 disposition in v0.11.1 (DF-26, DOGF-06) |

## Red-then-green record

Build profile for every cargo call: `-p paladin-ai --features cli,web-server`.

**Task 1 bin RED** (`--bin paladin-server load_settings`): `test result: FAILED. 5 passed; 2 failed`. The two new tests failed: `load_settings_applies_a_trace_env_overlay` (persist stayed `false`) and `load_settings_refuses_a_zero_trace_channel_capacity_from_the_environment` (`load_error` panicked: `load_settings accepted a configuration it should refuse`). The five existing tests passed under `#[serial]` inside `with_overlay_env(&[], ..)`.
**Task 1 GREEN:** `7 passed; 0 failed`.

**Task 2 bin RED:** `8 passed; 3 failed`. Failed: `load_settings_applies_an_agent_runtime_env_overlay`, `load_settings_refuses_an_invalid_agent_runtime_value_from_the_environment`, `load_settings_applies_a_web_server_env_overlay`. The no-op guard `load_settings_leaves_file_values_when_no_overlay_is_set` passed (guard, passes before and after).
**Task 2 lib RED** (`--lib section_wrappers`): compile error, `error[E0599]: no method named get_agent_runtime_config found for struct settings::Settings` (and the same for `get_web_server_config`); `error: could not compile paladin-ai (lib test) due to 2 previous errors`.
**Task 2 GREEN:** bin `11 passed; 0 failed` (the build has no `otel`, so all 11 run); lib `section_wrappers_return_an_overlaid_copy_and_leave_settings_untouched ... ok`; `--lib config::` `278 passed; 0 failed` (includes `trace_env_overrides_apply` and `env_overrides_apply_to_scalar_fields_only` under `#[serial]`); doctests:

```
test src/config/settings.rs - config::settings::Settings::get_trace_config (line 303) ... ok
test src/config/settings.rs - config::settings::Settings::get_web_server_config (line 347) ... ok
test src/config/settings.rs - config::settings::Settings::get_agent_runtime_config (line 325) ... ok
test result: ok. 3 passed; 0 failed
```

## Boot probe

Real binary built from Task 1's tree (`cargo build -q -p paladin-ai --bin paladin-server --features cli,web-server`). `control.yml` was `config.example.yml` in a `mktemp -d` directory outside the repo, with `server.host: "127.0.0.1"`, a free port from python3 `socket`, and `http.auth.enabled: false` (each edit asserted to apply once). Both runs from that directory; the directory was deleted afterwards. No retry with run-store/waypoint variables was needed.

`ok` (variable unset):

```
env -u RUST_LOG -u PALADIN_TRACE_CHANNEL_CAPACITY OPENAI_API_KEY=test-key-not-a-secret-df26 PALADIN_CONFIG=./control.yml timeout 20 <repo>/target/debug/paladin-server > ok.log 2>&1; echo "exit=$?" >> ok.log
[INFO  paladin_server] Loading configuration from './control.yml'
[INFO  paladin_server] paladin-server listening on http://127.0.0.1:55065 — serving 2 agent(s): ["researcher", "summarizer"]
[INFO  paladin_server] received SIGTERM; shutting down
exit=124
```

`listening on` count: 1, exit 124 (timeout killed a healthy server).

`env0` (same command plus `PALADIN_TRACE_CHANNEL_CAPACITY=0`):

```
[INFO  paladin_server] Loading configuration from './control.yml'
[ERROR paladin_server] paladin-server failed to start: invalid configuration: trace.channel_capacity must be greater than 0
exit=1
```

Refusal line once, `listening on` count 0, exit 1.

## Public API baseline

`make api-surface` first reported the change (three `+pub fn ...Settings::get_{trace,web_server}_config` lines, plus agent_runtime, then `make: *** [Makefile:454: api-surface] Error 1`). After `make api-surface-update` (4331 items), `git diff -U0 -- .project/current-exports.txt`:

- removed: only the `# Public API Surface - Generated 2026-10-31 13:09:03 UTC` header and `Total public items: 4325`;
- added: exactly six `pub fn` lines (`get_trace_config`, `get_agent_runtime_config`, `get_web_server_config`, each listed twice, as the file lists every `Settings` method twice) plus the new header and `Total public items: 4331` (old total plus 6);
- `grep -cE '^pub fn paladin::config::(settings::)?Settings::get_(trace|agent_runtime|web_server)_config\(&self\)'` prints `6`;
- re-run `make api-surface`: `API surface unchanged`.

## Gates

- `cargo fmt --all -- --check`: clean (FMT_OK; rustfmt reformatted one assert in a new test before the Task 1 commit).
- `cargo clippy -p paladin-ai --features cli,web-server --lib --bin paladin-server -- -D warnings`: `Finished dev profile`, no warnings.
- same with `--profile test`: `Finished test profile`, no warnings.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --features cli,web-server --no-deps --lib`: `Finished dev profile`, `Generated .../target/doc/paladin/index.html`, no warnings.
- `make api-surface`: `API surface unchanged` (after the update above).
- `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-ai`: `223 checks: 223 pass, 31 skip`; `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`.
- `make check-gates`: exit 0; ends `OK: scanned 626 file(s) (19 test-only), 0 finding(s)`.
- `git diff --quiet 5168738f8 -- Cargo.toml Cargo.lock crates .cargo/semver-checks-allowlist.toml`: true (no dependency or allowlist change).
- Scope guard `git diff --quiet 8859af226~1 -- src/infrastructure src/config/trace.rs src/config/agent_runtime.rs src/config/web_server.rs src/config/env_utils.rs src/main.rs src/application`: true. `git diff --numstat -- src/config/settings.rs` over the plan: `109 0` (0 deleted lines).
- Docs diffs: `CHANGELOG.md` `4 0`; `MIGRATION.md` `5 1`, and the single removed line starts `-| M-B-37 |` (only its one sentence changed). Exactly one `### Fixed` heading in `[Unreleased]`. Section 11.2, parts 9/10 and `[0.11.0]` untouched.
- Findings log: `git diff --numstat` `1 0` (0 deletions). `./scripts/check-findings-log.sh`:

```
🔍 Checking the findings log for entries without a final disposition ...
✅ OK: every entry's last Disposition is a final form (parsed 26 entries from .project/v0.11.1/02-dogfood-findings-log.md)
```

Disk: free space fell from 4319 MB to 2304 MB across the build gates; `target/debug/incremental` was cleared once (4292 MB free) before the semver run. No ENOSPC occurred.

## Security review

Manual credential-handling review (`.github/instructions/security.instructions.md`, "Manual review still matters"), done by hand because no merge-gating Rust SAST exists:

- No new log statement is added by this plan; `read_env` logs nothing.
- The refusal echoes key names only (`trace.channel_capacity must be greater than 0`); no overlay value is interpolated.
- None of the three sections has a secret-shaped environment variable (`trace.otel.headers` is config-file only), so no credential can enter through the new overlay, and `Debug` of `Settings` already redacts what it carries.
- No HTTP client is created or modified, so the redirect and header-forwarding rules are not engaged.
- The test helper's `unsafe` `set_var`/`remove_var` calls carry `// SAFETY:` comments naming `#[serial]`; no `.unwrap()`, `.expect(` or `panic!` was added outside a `#[cfg(test)]` module.
- Test sentinels (`test-key-not-a-secret-df26`) are non-secret literals.

## Deviations from Plan

None. The plan executed as written.

- The `trailing-whitespace` and `end-of-file-fixer` hooks reported `no files to check` for the two `.rs` files in Task 1 (the hook configuration does not select `.rs`); they ran and passed on the `.md`, `.txt` and `.rs` files in the later commits, and `cargo fmt --check` is the formatter gate for Rust.
- Commit trailers use this executor's own harness-provided attribution rather than the orchestrator's named model (noted above).

## Authentication gates

None.

## Known Stubs

None.

## Threat Flags

None. The only new surface is the documented env overlay (T-56-52 to T-56-56 in the plan's register, all mitigated as planned); no new endpoint, auth path, file access pattern or trust-boundary schema change.

## Stage C inputs

None. DF-26 was the last open promoted finding; the next free finding id is DF-27 and no entry was added. 56-10 remains (wave 9).

## Self-Check: PASSED

Files verified present: `src/config/settings.rs`, `src/bin/paladin-server.rs`, `.project/current-exports.txt`, `CHANGELOG.md`, `MIGRATION.md`, `.project/v0.11.1/02-dogfood-findings-log.md`. Commits `8859af226`, `9dce25b42`, `6f92bc18e`, `2f5f4344a` verified in `git log`.
