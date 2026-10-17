---
phase: 50-provider-parity-config-driven-model-swap
plan: 20
subsystem: llm
tags: [rust, paladin-llm, kimi, sampling-parameters, gap-closure, g-50-5c, adr-0061, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5c]

requires:
  - phase: 50
    provides: "50-03 fallback_from_compat and the per-model profile admission; 50-06 seeded kimi-k2.6 and kimi-k3 rows; 50-16 closeout that ledgered the stale fallback as WINDOWS row 76"
provides:
  - "Kimi adapter-wide fallback declares temperature, top_p, frequency_penalty and presence_penalty NotSent, equal to the seeded Kimi rows, at its source (the four request_parameters flags false; the two Fixed overrides removed)"
  - "Kimi temperature_range Some((0.0, 1.0)), ADR-0004's framework default, stated as no vendor measurement"
  - "Tests kimi_fallback_matches_the_seeded_sampling_declarations (reads seeded::KIMI), kimi_fallback_profile_sends_no_sampling_values, unseeded_kimi_body_carries_no_sampling_values, kimi_explicit_sampling_values_on_an_unseeded_model_are_omitted_and_recorded, get_capabilities_reports_the_framework_default_temperature_span, every_new_provider_declares_a_temperature_range, matrix case kimi_unseeded_model_refuses_an_explicit_temperature"
  - "Switching providers page, both CHANGELOGs, MIGRATION M-B-13/M-B-14 and ADR-0061 limit (5) describe the shipped fallback; WINDOWS row 76 fixed through the tool"
affects: [50-31]

tech-stack:
  added: []
  patterns:
    - "A fallback profile derived from a preset's request_parameters flags false (the Grok idiom) rather than overridden per parameter after fallback_from_compat"
    - "A fallback-vs-seed test that reads the seeded table instead of restating its literals"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/examples/live_vendor_smoke.rs
    - tests/provider_swap_matrix.rs
    - docs/src/user-guides/switching-providers.md
    - CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - MIGRATION.md
    - .planning/decisions/0061-per-model-parameter-support-policy.md
    - .planning/WINDOWS.md

key-decisions:
  - "Owner option (a): NotSent is the only mode-independent declaration, because the vendor's single legal temperature depends on the thinking mode (only 1 on 2026-08-22, only 0.6 with thinking disabled on 2026-10-15)"
  - "Kimi temperature_range becomes the framework default (0.0, 1.0) rather than None, because every shipped adapter must declare Some(_); no admission reads it while model_profile returns Some"
  - "OpenAI-compatible is constructed with a declared range in the new invariant, since an empty declaration reports None by design (operator-declared, D-04)"

patterns-established: []

requirements-completed: [PARITY-02, PARITY-04]

duration: 40min
completed: 2026-10-17
---

# Phase 50 Plan 20: Kimi fallback sends no sampling values (G-50-5c) Summary

**An unseeded Kimi model id now behaves exactly like `kimi-k2.6` and `kimi-k3`: temperature, top_p and both penalties are `NotSent`, an explicit temperature is refused at build by name, and a framework default is omitted and recorded; WINDOWS row 76 is fixed.**

## Performance

- **Duration:** about 40 min
- **Completed:** 2026-10-17
- **Tasks:** 3 of 3 (Task 1 was the tracer; its gate passed in autonomous mode: "Tracer verified end-to-end - expanding")
- **Files modified:** 10

## Accomplishments

- `KimiAdapter::new` declares the four sampling flags of `request_parameters` false (every field still explicit, WR-02), so `fallback_from_compat` emits `NotSent`; the two `Fixed(1.0)` / `Fixed(0.95)` overrides and their 2026-08-22 comment are gone. `max_tokens` stays carried, so the configured output cap still reaches the wire.
- The history comment cites the 2026-10-15 probe ("only 0.6", "only 0.95", penalties "only 0"), the 2026-08-22 "only 1" measurement and why the legal value is thinking-mode dependent, and states that a seeded entry or a `ModelOverride` still decides for its own model.
- `temperature_range` is `Some((0.0, 1.0))` with rustdoc-style comment stating it is no vendor claim and is read by no admission; the smoke harness's Kimi `ProviderSpec` mirrors it.
- The cross-adapter invariant now constructs Kimi and the other five compat-era adapters (`every_new_provider_declares_a_temperature_range`); the stale claim that the old invariant covered Kimi is gone.
- Docs, CHANGELOGs, MIGRATION and ADR-0061 describe the fallback that ships.

## Task Commits

1. **Task 1 (tracer): fallback source, pinning tests rewritten, fallback-vs-seed test** - `583779a6` (fix)
2. **Task 2: temperature span, invariant extended to Kimi, smoke mirror, matrix case** - `17dc2fa3` (test)
3. **Task 3: docs, changelogs, MIGRATION, ADR-0061, WINDOWS row 76** - `faf81149` (docs)

The fallback-vs-seed test was run red first in Task 1 (failed on `Fixed(1.0)` vs `NotSent`) before the source change. In Task 2 the three new tests and the source change were written in one pass, so their red state was not run separately.

## Deviations from Plan

None - plan executed as written. Two small judgement calls inside the plan's scope:

- The new invariant constructs OpenAI-compatible with a declared `temperature_range: Some((0.0, 2.0))`; with the empty declaration the plan's `Some(_)` assertion could not hold, because that adapter reports `None` by design (operator-declared).
- Known-limitation items (4)-(7) in the root CHANGELOG were renumbered to (3)-(6) after deleting the Kimi item.

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 1144 passed, 0 failed (includes `kimi::` 76 tests, `capability_invariants` 4, `switching_page_tests` 13).
- `cargo test -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`: 25 passed.
- `cargo test --test provider_swap_matrix --features llm-all`: 51 passed (includes `kimi_unseeded_model_refuses_an_explicit_temperature`).
- `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `make check-gates`: exit 0. `make api-surface`: "API surface unchanged" (4298 items).
- `make clean-code` was not run as a whole: it stops at `lint-shell` because shellcheck is missing (WINDOWS row 72, baseline); its fmt and clippy parts were run directly above.
- Not exercised, baseline and not caused by this plan: the two `run_api_wiring` tests (WINDOWS row 73) and the Docker-bound `integration::redis_queue_integration_test` tests. The credential-gated `live_vendor_smoke` run needs vendor keys that do not exist here.
- Diff check for `unwrap(` / `expect(` / `panic!(`: every added occurrence is inside `#[cfg(test)]` code or the `tests/` integration test; none was added to non-test code.
- Manual credential-handling review: no credential path, header, redirect policy or response-body redaction was touched; the change only alters which sampling parameters reach the request body.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file access or trust-boundary schema. Threats T-50-20-01 to T-50-20-03 are mitigated: an explicit value is refused by name (matrix case), a default is omitted and recorded, the four parameters are `NotSent` on the fallback with a seed-comparison test, and every `request_parameters` field remains explicit.

## Flagged assumptions

- A1-50-20 held: `PaladinBuilder` reads `temperature_range` only when `model_profile` returns `None`, which `KimiAdapter` never does.
- A2-50-20 remains an unmeasured blind spot: no thinking-enabled temperature probe has run; `NotSent` makes it moot for every Kimi id. The smoke harness comment notes the next probe should measure the span's ends and middle.
- A3-50-20 unchanged: the fallback's `reasoning: false` and `json_schema: false` still differ from the seeded rows; outside the owner's decision.

## Self-Check: PASSED

- Files modified exist: kimi/adapter.rs, lib.rs, live_vendor_smoke.rs, provider_swap_matrix.rs, switching-providers.md, both CHANGELOGs, MIGRATION.md, ADR-0061, WINDOWS.md.
- Commits `583779a6`, `17dc2fa3`, `faf81149` exist on `claude/laughing-dirac-e0h2ax`.
- WINDOWS row 76 reads `fixed` (tool-written).
