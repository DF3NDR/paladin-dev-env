---
phase: 50-provider-parity-config-driven-model-swap
plan: 04
subsystem: testing
tags: [rust, paladin-llm, live-smoke, probe, credential-gate, redaction, parity]

requires:
  - phase: 50
    provides: "plan 50-02: the ModelProfile contract the probe evidence will seed (json_object and json_schema fields included)"
provides:
  - "live_vendor_smoke: one table-driven harness over all nine providers (OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini, Ollama, OpenAI-compatible), each behind its own credential gate"
  - "SKIPPED (<VAR> not set) by name for an unconfigured provider; zero configured providers exits non-zero with 'no provider was configured -- nothing was tested'"
  - "--probe: markdown transcript of the live models listing and, per candidate model, default / temperature / top_p / penalties / stop / max_tokens / JSON-object / JSON-schema outcomes, usage field names, reasoning-text presence and quota-shaped errors"
  - "--dry-run: the full nine-provider probe plan with no network access and no credential read"
  - "--models <provider>=<id>[,<id>...] (repeatable): extra candidate models for the operator's run"
  - "[[example]] live_vendor_smoke required-features widened to the nine adapter features"
affects: [50-06, 50-13, 50-15, 50-16, 50-17]

tech-stack:
  added: []
  patterns:
    - "Const provider table driving both the smoke and the probe, so a tenth provider is one row"
    - "Redact-then-bound on every printed vendor body (diagnostic_excerpt), then table-escape, never the reverse"
    - "Vacuous-pass refusal: an unconfigured provider is SKIPPED by name and a run that tested nothing exits non-zero"
    - "Credentialed probe client built with redirect::Policy::none(); credentials only in headers, URLs printed through redact_url"

key-files:
  created: []
  modified:
    - crates/paladin-llm/examples/live_vendor_smoke.rs
    - crates/paladin-llm/Cargo.toml

key-decisions:
  - "Probe calls carry a bounded output cap (256) and a 120 s timeout; Moonshot acceptance probes add thinking disabled, and one extra default-thinking call per Moonshot model observes the reasoning fields (A3-50-04). Every body extra is printed in the row it was used on."
  - "The OpenAI baseline cap field is max_completion_tokens, so a newer model that rejects max_tokens does not poison every row; max_tokens=64 and max_completion_tokens=64 are then both probed as single-parameter calls."
  - "The JSON-mode probes use a prompt that names JSON, because OpenAI-chat json_object mode refuses a prompt that does not, which would record a prompt artefact as a capability finding."
  - "Anthropic and DeepSeek have no live model fetch, so the smoke model-list half is reported NOT APPLICABLE and is not counted as a pass."

patterns-established:
  - "Harness exit code is a pure function (smoke_exit_code) of configured and failed counts, unit-tested"
  - "Offline proof of an operator-run harness: --dry-run for the plan, plus a local mock vendor for the live code paths"

requirements-completed: []

coverage:
  - id: D1
    description: "All nine providers in one table behind the credential gate; an absent credential prints SKIPPED (<VAR> not set); an all-unset run exits non-zero saying nothing was tested"
    requirement: "PARITY-07"
    verification:
      - kind: integration
        ref: "Task 1 automated verify: cargo run -q -p paladin-llm --example live_vendor_smoke with every credential variable unset -> non-zero exit, 'nothing was tested', 9 SKIPPED lines"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --example live_vendor_smoke (an_absent_credential_is_named_and_never_a_pass, zero_configured_providers_is_a_failure_not_a_vacuous_pass, openai_compatible_needs_both_its_key_and_base_url)"
        status: pass
    human_judgment: false
  - id: D2
    description: "--probe --dry-run prints nine ## provider sections with endpoints, candidate models and the ordered probe list including json_schema, exits 0, reads no credential"
    requirement: "PARITY-07"
    verification:
      - kind: integration
        ref: "Task 2 automated verify: --probe --dry-run with every credential unset -> exit 0, 9 lines matching '^## ', json_schema present"
        status: pass
      - kind: unit
        ref: "the_dry_run_lists_all_nine_providers_and_names_no_credential; every_provider_plan_lists_the_json_schema_probe_and_anthropic_marks_it_na"
        status: pass
    human_judgment: false
  - id: D3
    description: "Temperature is probed at the declared low end, high end, 0.5 and the next value outside the high end (Qwen's half-open [0, 2) probes 2.0)"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "temperature_points_measure_the_range_endpoints_and_the_next_value_outside"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every printed vendor body is redacted before it is bounded; the probe client never follows a redirect; no URL carries userinfo or a credential query value; no header value is printed"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "a_printed_body_is_redacted_before_it_is_bounded (key straddling the 512-char boundary), printed_urls_carry_no_userinfo_and_no_credential_query_value"
        status: pass
      - kind: integration
        ref: "local mock vendor (not committed): echoed Authorization header and key= in error bodies printed as [REDACTED]; a 302 to a second local port received no request (redirect not followed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Request bodies put each parameter on its own wire field (OpenAI-chat stop/max_tokens/response_format, Anthropic stop_sequences, Gemini generationConfig.*), vary exactly one thing from the baseline, and apply the Moonshot extra everywhere except the default-thinking call"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "request_bodies_put_each_parameter_on_its_own_wire_field; the_max_tokens_probe_replaces_the_baseline_cap_instead_of_doubling_it; moonshot_thinking_is_disabled_everywhere_except_the_default_thinking_probe"
        status: pass
    human_judgment: false
  - id: D6
    description: "Live behaviour against the real vendors (accepted/rejected outcomes, usage field names, quota-shaped errors) is measured on the operator's keys"
    requirement: "PARITY-04"
    verification:
      - kind: other
        ref: "plan 50-17: operator-run transcript 50-SEED-PROBE.md. No vendor key or vendor host is reachable in this environment, so none of the real wire parsing has been exercised against a real vendor."
        status: not-run
    human_judgment: true

duration: 55min
completed: 2026-10-15
status: complete
---

# Phase 50 Plan 04: Nine-provider live smoke and seed probe harness Summary

**`live_vendor_smoke` is now one table-driven, credential-gated harness over all nine providers with a `--probe` mode that measures what each vendor's models accept and report, and a `--dry-run` that proves the plan offline, so plan 50-17 has its instrument.**

## Performance

- **Duration:** about 55 min
- **Completed:** 2026-10-15
- **Tasks:** 2
- **Files modified:** 2 (0 created)

## Accomplishments

- Replaced the four copy-pasted vendor blocks with a `const PROVIDERS: [ProviderSpec; 9]` table (credential variables, shipped base URL, default model, curated models, wire, auth header shape, declared temperature range, output-cap field, acceptance extras, builder). Smoke mode keeps the two original probes (live list versus the curated fallback, default-parameter `generate()` refusing empty content and zero tokens) for every configured provider. An absent credential prints `SKIPPED (<VAR> not set)` and is never a pass; a run that configured nothing exits 1 with `no provider was configured -- nothing was tested`. An unknown flag is an error naming it (exit 2).
- Added `--probe`: raw HTTP through one `reqwest` client with `redirect::Policy::none()` and a 120 s timeout. Per configured provider it prints a markdown section with the models listing (every field the vendor returned, each through `diagnostic_excerpt`) and, per candidate model (default, curated list, `--models` extras), a table of 13 to 15 ordered single-parameter calls: default, temperature at the declared low end, high end, 0.5 and the next value outside, top_p, frequency and presence penalty (OpenAI-chat wires), 1 and 5 stop sequences, `max_tokens`, JSON-object and JSON-schema modes (Anthropic: `NOT APPLICABLE`). Each row records `ACCEPTED (<status>)` or `REJECTED (<status>): <excerpt>`, finish reason, content presence, usage field names (so cached, cache-write and reasoning token fields are visible), and reasoning-text presence. Quota-shaped errors are collected verbatim into their own section.
- Added `--dry-run` (touches neither the network nor the environment) and `--models`.
- 24 unit tests in the example (`cargo test -p paladin-llm --example live_vendor_smoke`), including the redact-before-bound case with a key straddling the 512-character boundary and a test that the two static model lists match the Anthropic and DeepSeek adapters.

## Task Commits

1. **Task 1: nine-provider credential-gated smoke table, widened example gate** - `97558ac8` (feat)
2. **Task 2: --probe, --models and --dry-run** - `eec2c932` (feat)

**Plan metadata:** recorded in the final `docs(50-04)` commit.

## Files Created/Modified

- `crates/paladin-llm/examples/live_vendor_smoke.rs` - the harness (table, smoke, probe, dry run, argument parser, tests)
- `crates/paladin-llm/Cargo.toml` - `[[example]] live_vendor_smoke` `required-features` now lists the nine adapter features

## Decisions Made

- Acceptance probes are bounded (`max_tokens` 256, 120 s) and Moonshot adds `{"thinking":{"type":"disabled"}}`, with one separate default-thinking call per Moonshot model (A3-50-04). The extra is printed in a "body extras" column on each row it was used on.
- OpenAI's baseline cap field is `max_completion_tokens` and `max_tokens=64` is probed separately, so one unsupported cap field cannot turn every row into a rejection.
- JSON-mode probes use a prompt that mentions JSON (OpenAI-chat refuses `json_object` otherwise).
- A smoke provider whose adapter has no live fetch (Anthropic, DeepSeek) reports the list half as `NOT APPLICABLE`, outside the pass count.
- Unreachable providers in probe mode are named `UNREACHABLE` and make the run exit non-zero; a probe never reports a provider as passing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] OpenAI `max_completion_tokens` probe and baseline cap**
- **Found during:** Task 2
- **Issue:** The plan lists a `max_tokens` probe only. Newer OpenAI models reject `max_tokens`, so with `max_tokens` as the baseline cap every OpenAI probe row would read REJECTED and plan 50-06 could not seed OpenAI from the transcript.
- **Fix:** OpenAI's baseline cap uses `max_completion_tokens`; the `max_tokens=64` probe replaces the baseline cap rather than adding to it; an extra `max_completion_tokens=64` probe is planned for OpenAI only.
- **Files modified:** `crates/paladin-llm/examples/live_vendor_smoke.rs`
- **Committed in:** `eec2c932`

**2. [Process] Commit trailer model name**
- The orchestrator note gave `Co-Authored-By: Claude Fable 5.1`; the harness attribution reminder for this session specifies `Claude Sonnet 5.5`. I followed the harness reminder, which is the authority for commit attribution, so both commits on this plan carry `Claude Sonnet 5.5` (the first was amended before anything was pushed). The `Claude-Session` line is unchanged.

**Total deviations:** 2 (1 missing-functionality, 1 process). **Impact on plan:** none on scope.

## Issues Encountered

- `make clean-code` stops at `lint-shell` because `shellcheck` is not installed in this sandbox (already recorded in `.planning/WINDOWS.md` by plan 50-02). `cargo fmt --check -p paladin-llm` and `cargo clippy -p paladin-llm --examples --features <nine> -- -D warnings` pass.
- No vendor key exists and vendor hosts are blocked, so the live code paths were exercised against a throwaway local mock vendor (a Python script in the session scratchpad, not committed) through the OpenAI-compatible and Ollama rows. It confirmed the markdown transcript, usage-field extraction, quota-shaped-error collection, `[REDACTED]` of an echoed `Authorization` header and `key=`, that a 302 is reported and not followed, and the non-zero exit for an unreachable Ollama. The Anthropic and Gemini request and response shapes are covered by unit tests only.

## Verification

- Task 1 automated verify: builds; with every credential variable unset the smoke run exits non-zero, prints `nothing was tested` and 9 `SKIPPED` lines
- Task 2 automated verify: `--probe --dry-run` with every credential unset exits 0, prints exactly 9 `## ` lines, lists `json_schema`
- `--probe` with every credential unset exits non-zero with `nothing was tested`
- `cargo test -p paladin-llm --example live_vendor_smoke --features <nine>`: 24 passed
- `cargo clippy -p paladin-llm --examples --features <nine> -- -D warnings`: clean; `cargo fmt --check -p paladin-llm`: clean
- `grep -c diagnostic_excerpt` prints 5 and `grep -c 'Policy::none()'` prints 1; no `unwrap` or `expect` outside the test module
- Manual credential-handling review (D-00a, security.instructions.md): every printed vendor body, model id, listing field and transport error goes through `diagnostic_excerpt` (redact, then bound) before table escaping; the probe client is built with `redirect::Policy::none()` (a 3xx is printed as `redirect not followed`); keys travel only in headers (`bearer_auth`, `x-api-key`, `x-goog-api-key`), never a URL; every printed URL goes through `redact_url` (userinfo and credential-looking query values replaced); only credential variable NAMES are printed; `Built`, which holds the key, does not derive `Debug`; reqwest errors are printed with `without_url()`. Snyk was not run, per the project's security instructions.

## Known Stubs

None.

## Threat Flags

None. The only new surface is operator-run credentialed HTTP to the nine vendor hosts, which is the surface the plan's threat register (T-50-04-01 to T-50-04-05) covers: T-50-04-01 by redact-then-bound on every body, T-50-04-02 by `Policy::none()`, T-50-04-03 by `redact_url` and header-only credentials, T-50-04-04 by the SKIPPED naming and zero-configured exit, T-50-04-05 by the 256-token cap, 120 s timeout and Moonshot thinking disabled.

## User Setup Required

None for this plan. The operator's live run (credentials in the environment, then `cargo run -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible -- --probe`) is plan 50-17.

## Next Phase Readiness

- Plan 50-17 can run `--probe` on the operator's keys. Plan 50-06 must seed `json_object`/`json_schema` and the `Range` endpoints only from that transcript.
- Anthropic's candidate list is the adapter's current static list (mostly retired `claude-3` ids); the operator should pass current ids with `--models anthropic=<id>`. Likewise OpenAI's single default candidate `gpt-4o-mini` is a harness choice, not a seeded default.
- The harness does not accept an operator-pasted quota error body (A2-50-04); that stays a manual step in plan 50-17.
- `requirements-completed` is deliberately empty: PARITY-07 also needs the shared conformance suite and the "Switching providers" page, and PARITY-04/05 need the registry and the empty-output/refusal work, all in later plans. This plan delivers the live-smoke clause and the probe half only.

## Self-Check: PASSED

- FOUND: crates/paladin-llm/examples/live_vendor_smoke.rs
- FOUND: crates/paladin-llm/Cargo.toml (all nine features in the `live_vendor_smoke` required-features)
- FOUND commit 97558ac8 and eec2c932
