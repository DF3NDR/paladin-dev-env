---
phase: 50-provider-parity-config-driven-model-swap
plan: 19
subsystem: llm
tags: [rust, paladin-llm, openai, wire-format, gap-closure, g-50-5b, adr-0061, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5b]

requires:
  - phase: 50
    provides: "50-08 OpenAIConfig.max_tokens knob and the ADR-0061 (i) max-output check; 50-16 closeout that ledgered the defect as WINDOWS row 75"
provides:
  - "OpenAIRequest and OpenAIVisionApiRequest serialise the output cap as max_completion_tokens (serde rename; Rust field names and every Paladin-side name unchanged)"
  - "Test openai_gpt_5_4_nano_bodies_carry_max_completion_tokens covering the buffered, streamed and json_schema bodies; vision_request_carries_max_completion_tokens"
  - "live_vendor_smoke drives gpt-5.4-nano for OpenAI by default"
  - "MIGRATION 9.1 row M-B-18; Switching providers, root and crate CHANGELOG and ADR-0061 stated limit (5) state the measured scope; WINDOWS row 75 fixed"
affects: [50-20, 50-31]

tech-stack:
  added: []
  patterns:
    - "A wire-key rename done with a serde attribute on the single request constructor's struct, so every caller of build_request changes together"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/openai/vision.rs
    - crates/paladin-llm/examples/live_vendor_smoke.rs
    - docs/src/user-guides/switching-providers.md
    - CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - MIGRATION.md
    - .planning/decisions/0061-per-model-parameter-support-policy.md
    - .planning/WINDOWS.md

key-decisions:
  - "Owner option (a): the rename is unconditional, with no per-model profile field for the parameter name"
  - "Measured acceptance of max_completion_tokens covers gpt-5.4-nano and gpt-4o-mini only (2026-10-15); older OpenAI ids and non-OpenAI hosts behind OPENAI_BASE_URL are unmeasured and routed to the openai-compatible preset"

patterns-established: []

requirements-completed: [PARITY-04, PARITY-03]

duration: 25min
completed: 2026-10-17
---

# Phase 50 Plan 19: OpenAI sends max_completion_tokens (G-50-5b) Summary

**The OpenAI adapter now sends its output limit as `max_completion_tokens` on every chat and vision body, so the seeded `gpt-5.4-nano` no longer answers HTTP 400; Paladin-side names are unchanged and WINDOWS row 75 is fixed.**

## Performance

- **Duration:** about 25 min (this continuation; Task 1 ran in the previous agent)
- **Completed:** 2026-10-17
- **Tasks:** 3 of 3
- **Files modified:** 9

## Accomplishments

- Task 1 (tracer, `cba89a08`): `#[serde(rename = "max_completion_tokens")]` on `OpenAIRequest`'s cap field; the existing wire assertions flipped with old-key absence checks; rustdoc rewritten. The user approved the tracer gate.
- Task 2 (`5d63cff2`): the `gpt-5.4-nano` body test now also covers `generate_stream` and a `ResponseFormat::JsonSchema` `generate`; `OpenAIVisionApiRequest` serialises its limit as `max_completion_tokens`; the smoke default model is `gpt-5.4-nano`.
- Task 3 (`c9842254`): Switching providers known-limit bullet, root CHANGELOG (known limitation (3) removed and the rest renumbered, `### Fixed` bullet added), crate CHANGELOG, MIGRATION 9.1 `M-B-18`, ADR-0061 stated limit (5) and WINDOWS row 75 (via `gsd-tools windows fixed 75`; `open_count` 14 to 13).

## Task Commits

1. Task 1: `cba89a08` fix(50-19): send the OpenAI output cap as max_completion_tokens (G-50-5b)
2. Task 2: `5d63cff2` fix(50-19): rename the OpenAI vision cap and smoke the seeded OpenAI model (G-50-5b)
3. Task 3: `c9842254` docs(50-19): record max_completion_tokens and close WINDOWS row 75 (G-50-5b)

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 1142 passed, 0 failed.
- `cargo test -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`: 25 passed.
- DeepSeek (87) and compat (78) wire tests pass unedited.
- `cargo test --test provider_swap_matrix --features llm-all`: 50 passed.
- `switching_page_tests` (drift test included): 13 passed.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0.
- `make check-gates` and `make api-surface` ("API surface unchanged", 4298 items): exit 0.
- `make clean-code` was not run to completion: it stops at `lint-shell` because shellcheck is not installed in this sandbox (already ledgered, WINDOWS row 72). Its fmt and clippy parts were run directly as above.
- No `unwrap(`, `expect(` or `panic!(` was added outside `#[cfg(test)]` code (checked on the diff of `crates/`; every hit is in test modules).
- Manual credential-handling review (security.instructions.md): no new log line, `Debug` output or error text carries a key; no HTTP client, redirect policy or redaction path was touched; the only wire change is the name of the output-cap key.

## Baseline failures (not caused by this plan)

- The two `run_api_wiring` tests ledgered in WINDOWS row 73, and the `integration::redis_queue_integration_test` tests that need `/var/run/docker.sock`. Not run or chased.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Test-module name shadowing and missing imports**
- **Found during:** Task 2
- **Issue:** the extended test bound a local `server` that shadowed `test_support::server`, and the nested module lacked `Arc`, `Mutex`, `Value`, `json!` and `StreamExt` imports.
- **Fix:** locals renamed to `mock_server`, imports added in the `max_completion_tokens_wiring` module.
- **Files modified:** crates/paladin-llm/src/openai/adapter.rs
- **Commit:** 5d63cff2

**2. [Rule 1 - Bug] New smoke test looked up the wrong provider name**
- **Found during:** Task 2
- **Issue:** my `the_openai_default_candidate_is_the_first_seeded_model` test searched `PROVIDERS` for `"openai"`; the table names it `"OpenAI"`.
- **Fix:** corrected the lookup.
- **Commit:** 5d63cff2

### Process notes

- The streamed and json_schema parts of the nano test passed on first run by design (they share `build_request` with the buffered path, which Task 1 already fixed); they pin that the paths stay together. The vision test was written in the same step as its rename rather than watched failing first, because stashing to demonstrate red is prohibited here; its assertion on the old key's absence is what would have failed.
- The smoke default is pinned by a unit test on `OPENAI_PROBE_DEFAULT_MODEL` and the `PROVIDERS` entry rather than by calling `build_openai`, which needs `OPENAI_API_KEY` in the environment.

## Known Stubs

None.

## Threat Flags

None. T-50-19-01 (non-OpenAI server behind `OPENAI_BASE_URL`) is mitigated by the M-B-18 row and the docs; T-50-19-02 by the three-path nano test plus the old-key absence checks; T-50-19-03 accepted as planned.

## Self-Check: PASSED

All nine files modified are present; commits `cba89a08`, `5d63cff2` and `c9842254` exist on the branch.
