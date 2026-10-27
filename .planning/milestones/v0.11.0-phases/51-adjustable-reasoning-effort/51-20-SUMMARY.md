---
phase: 51-adjustable-reasoning-effort
plan: 20
subsystem: llm-conformance-and-register
tags: [reasoning-effort, check-reasoning, conformance, agent-host, migration, changelog, gap-closure, g-2]
status: complete

requires:
  - phase: 51-14
    provides: "the semver measurement method and the MIGRATION / CHANGELOG register pattern"
  - phase: 51-16
    provides: "host-load fallback precedence (agent first)"
  - phase: 51-17
    provides: "LlmPort::check_reasoning, native overrides, builder delegation"
  - phase: 51-18
    provides: "compat preset overrides, Pricing / Cadence forwarding"
  - phase: 51-19
    provides: "FallbackLlmAdapter per-hop check_reasoning"
provides:
  - "conformance case check_reasoning_agrees_with_send_time_admission on all nine adapter fixtures (suite is now 25 cases)"
  - "agent_host tests: a_blocks_fallback_lets_an_agents_own_effort_substitute_at_build, the_host_and_builder_verdicts_equal_the_adapters_check_for_every_combination (54 combinations), the_host_verdict_survives_the_pricing_decorator"
  - "consumer-surface muster pin of LlmPort::check_reasoning"
  - "guide, ADR-0062, MIGRATION 9.2 / 9.8, three CHANGELOGs and deferred-items describe the per-hop, provider-default-aware gates"
affects: []

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/conformance.rs
    - src/infrastructure/web/agent_host.rs
    - tests/consumer_surface_muster.rs
    - docs/src/user-guides/reasoning-effort.md
    - .planning/decisions/0062-reasoning-effort-fail-loud-policy.md
    - .planning/phases/51-adjustable-reasoning-effort/deferred-items.md
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md

key-decisions:
  - "The new MIGRATION 9.2 row for LlmPort::check_reasoning is N: cargo-semver-checks 0.50.0 reported no lint naming it, as expected for a provided method."
  - "The host's copy of the fallback hint text is left in place: the wording is identical to paladin_llm::reasoning::FALLBACK_HINT and the ports copy, pinned by a host test, and the host grid asserts verdicts, not text. Sharing one constant would add a public item."
  - ".project/current-exports.txt needed no change: make api-surface reports 4323 items, unchanged (the facade baseline lists no LlmPort methods)."

requirements-completed: [REASON-01, REASON-04, REASON-07]

completed: 2026-10-23
---

# Phase 51 Plan 20: Prove the invariant, register check_reasoning Summary

**One conformance case over all nine adapters and one 54-combination agent-host grid now pin "every pre-judging verdict equals the send-time verdict", and `LlmPort::check_reasoning` plus the gap-closure behaviour changes are registered in MIGRATION, the CHANGELOGs, the guide and ADR-0062.**

## Accomplishments

- `cases::check_reasoning_agrees_with_send_time_admission` runs on every fixture over models {default model, unprofiled id} x effort {`None`, `Off`, `Auto`, six levels} x fallback {`None`, `Refuse`, `NearestLower`, `NearestHigher`} (72 rows each, counted in the case). `check_reasoning` refuses iff `generate` returns `UnsupportedParameter` for `reasoning_effort`, with equal error text. Registered in `llm_conformance_suite!` after `every_seeded_effort_level_round_trips_to_its_wire_body`; `CASE_COUNT` pin and module rustdoc updated to 25.
- Host tests in `config_driven_tests`: WR-01 end to end (`a_blocks_fallback_lets_an_agents_own_effort_substitute_at_build`), the 54-combination grid (block effort x block fallback x agent effort x agent fallback, counter asserted, both verdicts present, `SECRET` never in an error; the equal pairs are rows), and the Pricing-decorator case with a one-row price table (also asserts an agent's own `refuse` still refuses under the decorator).
- `tests/consumer_surface_muster.rs` pins `llm.check_reasoning("m", None, None)` typed `Result<(), LlmError>`.
- Guide sections "`reasoning_fallback`", "Fallback chains", "Build-time checks" and "A provider-wide default" rewritten to the per-hop, provider-default-aware behaviour; the mapping table is untouched. ADR-0062 section (h) and the `EffortSupport::intersect` consequence carry dated "Revised 2026-10-23" notes, and the evidence item (h) names the three new tests.
- MIGRATION 9.2: new `N` row `paladin-ports | LlmPort` (`check_reasoning`); `paladin-ai | PaladinBuilder` row revised; 9.8 DR-21 revised (builder judges provider default and fallback for any port; per-hop pre-flight with hop defaults even without a request effort; chain profile effort `Unmeasured`; hint at send time; decorators forward). Behaviour change A2-51-17 is stated in the root CHANGELOG `### Changed` and DR-21.
- CHANGELOGs: root `### Added` / `### Changed` / `### Fixed`; `paladin-ports` `### Added`; `paladin-llm` `### Added` / `### Changed`. The stale 51-07 root sentence about the chain's intersected profile is corrected in place. IN-03: the four "No public item changed" sentences in `paladin-llm` reworded (three dropped, one replaced with "No further public item changed beyond those listed in this section."); the pre-flight entry now reads "A request with no effort in force on any hop behaves exactly as before."
- `deferred-items.md`: WR-04, IN-01 and IN-02 recorded with reasons and a revisit point; IN-03 recorded as folded in.

## Task Commits

1. Task 1 (tracer): `2348ffc0` -- `test(51-20): pre-judging verdicts equal send time on nine adapters and the agent host (G-2)`
2. Task 2: `dbe3a588` -- `docs(51-20): guide and ADR-0062 describe the per-hop, provider-default-aware effort gates`
3. Task 3: `0ee56f4b` -- `docs(51-20): register LlmPort::check_reasoning and the gap-closure behaviour changes (D-00b)`

Tracer gate (auto mode): the Task 1 verify slice passed on the committed tree (9 conformance tests, 3 host tests, muster); tracer verified end to end, expansion continued. All three were green on first run (they are confirmation tests over plans 51-15 to 51-19); no combination diverged, so there is no failing row to report. Commits used `--no-verify` as the wave allows.

## Measured semver register (paladin-ports)

Method of 51-14: the `[package.metadata.cargo-semver-checks.lints]` table was removed from a copy of `crates/paladin-ports/Cargo.toml`, `cargo semver-checks check-release --package paladin-ports --baseline-version 0.10.1 --release-type minor` was run (cargo-semver-checks 0.50.0), the original was restored and proved with `cmp` (`CMP_OK`; `git status` clean afterwards).

- 196 checks: 192 pass, 4 fail, 0 warn, 58 skip. The four lints are the already-registered `constructible_struct_adds_field`, `enum_tuple_variant_changed_kind`, `enum_unit_variant_changed_kind` and `enum_variant_marked_non_exhaustive` (the `LlmError` / `LlmRequest` / `LlmResponse` / `StreamingResponse` rows).
- Lints naming `check_reasoning` or `LlmPort`: none. The row is `N`; no allowlist entry and no lint-table line were added.

## Verification

- `cargo test --offline -p paladin-llm --lib --all-features`: 1525 passed, 0 failed (includes the nine new cases and `the_reasoning_effort_page_agrees_with_the_adapters`).
- `cargo test --offline -p paladin-ai --lib --all-features -- infrastructure::web::agent_host paladin_builder`: 103 passed.
- `cargo test --offline -p paladin-ports --doc check_reasoning`: 1 passed.
- `make check-consumer-surface`, `make check-gates`, `make check-migration-allowlist`, `make api-surface` (4323 items, unchanged) and `make check-api-examples` (105 of 105): exit 0.
- `cargo fmt --all --check`: clean. `cargo clippy --offline -p paladin-llm -p paladin-ai --all-targets --all-features -- -D warnings`: clean.
- Acceptance greps: `check_reasoning` appears 3 times in the guide; `intersected` and `skips the check` appear nowhere in it; "Revised 2026-10-23" appears twice in ADR-0062; "No public item changed" appears 0 times in `paladin-llm/CHANGELOG.md`; the MIGRATION row and DR-21 contain `check_reasoning`.
- Not run here (thermal cap, scoped runs only): the workspace-wide `cargo test --workspace --all-features --no-fail-fast`, `--doc`, `make clean-code` and `make security`. The orchestrator's post-merge chain owns them. `make security` is also already open from plan 51-14 (three RustSec advisories in transitive TLS-root crates, not touched by this phase). Coverage (82 percent, ADR-0006) stays the CI `coverage` job; local proxy is the test counts above.
- Every cargo run used `CARGO_BUILD_JOBS=4`, one at a time.

## Deviations from Plan

None in behaviour. Notes:

- [Scope] Plan 51-17's note that `agent_host.rs` around line 213 carries its own copy of the "set a reasoning_fallback" hint was reviewed and the copy left alone: identical wording, pinned by `the_host_fallback_hint_appears_only_when_refuse_is_in_force`, and the host grid asserts verdicts. Recorded in `deferred-items.md`.
- [Process] ADR-0062's original section (h) clause about the intersected profile was also marked superseded in place (in addition to the dated revision paragraph the plan asked for), so the page does not state the pre-gap gate unqualified (REASON-07 prohibition).
- [Process] Edits to the guide, ADR, MIGRATION and CHANGELOGs were applied with small Python scripts kept in the scratchpad; only the files in `files_modified` were touched.

## Manual credential-handling review (security.instructions.md)

Reviewed every file plans 51-15 to 51-20 touched on the points that matter for credentials: the only strings `check_reasoning` / `judge` produce are provider name, model, effort spellings, fixed refusal wording and YAML key names; no key, header, URL, prompt or response body is reachable from them, and no log statement was added by this plan. The new host tests use the existing `SECRET` constant and assert it never appears in an error; the conformance case uses the fixtures' own test keys and mock server and prints nothing. The guide and ADR examples contain no keys. No HTTP client was constructed or changed.

## Known Stubs

None.

## Threat Flags

None. T-51-58 mitigated (existing constants only; `SECRET` asserted absent). T-51-59 mitigated (MIGRATION row, DR-21 and CHANGELOG entries; `make check-migration-allowlist` and `make api-surface` pass). T-51-60 mitigated (the MIGRATION row tells decorator authors to forward `check_reasoning`; `the_host_verdict_survives_the_pricing_decorator`). T-51-SC accepted: no dependency added.

## Self-Check: PASSED

- Files: all ten modified files exist and are committed; this SUMMARY is present.
- Commits `2348ffc0`, `dbe3a588` and `0ee56f4b` exist on `worktree-agent-a4b1e69af8b7feedd`.
