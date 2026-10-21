---
phase: 51-adjustable-reasoning-effort
plan: 07
subsystem: agent-config
tags: [reasoning-effort, paladin-data, paladin-builder, fallback-chain, preflight, semver-register]
status: complete

requires:
  - phase: 51-01
    provides: "ReasoningEffort/ReasoningFallback, EffortSupport::resolve and intersect, LlmRequest.reasoning, LlmError::UnsupportedParameter"
  - phase: 51-03
    provides: "reasoning metadata keys stamped by the adapters, admit/in_force"
  - phase: 51-06
    provides: "provider-level default applied per request below the request's own value"
provides:
  - "PaladinData.reasoning_effort / reasoning_fallback (serde default, skipped when None) with every full literal migrated"
  - "PaladinBuilder::reasoning_effort / reasoning_fallback setters"
  - "The agent's effort and fallback on all three LlmRequest the execution service builds, only when set"
  - "D-09 build-time refusal in PaladinBuilder::validate (chain: intersected profile; no profile: refused as unmeasured)"
  - "D-08 FallbackLlmAdapter::preflight_reasoning across every hop on generate and generate_stream"
  - "MIGRATION 9.2 Y row, allowlist entry and consumer-muster update for the PaladinData break"
affects: [51-08, 51-09, 51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "agent level of the precedence set only when the agent set it, so an agent that sets nothing builds today's request"
    - "build-time check and adapter send-time check share EffortSupport::resolve, so they cannot drift"
    - "chain pre-flight resolves every hop against its own step model before any hop is called"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/paladin.rs
    - crates/paladin-core/Cargo.toml
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-battalion/src/council_service.rs
    - crates/paladin-battalion/src/engine/graph_doc.rs
    - src/application/services/assistant/validator.rs
    - src/core/mod.rs
    - src/application/services/paladin/paladin_builder.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - crates/paladin-llm/src/fallback.rs
    - crates/paladin-llm/CHANGELOG.md
    - tests/cli_isolation_test.rs
    - tests/consumer_surface_muster.rs
    - .cargo/semver-checks-allowlist.toml
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "Build-time check for a chain uses the chain's intersected profile (A1-51-07): conservative, a level must be usable on every hop, while at send time each hop may substitute differently."
  - "The pre-flight checks the request's own effort only (A2-51-07); a hop adapter's provider default is resolved by that adapter at send time and refuses there, naming llm.<provider>.reasoning_effort."
  - "The pre-flight error reason reads 'fallback hop <index>: <refusal>' with a zero-based chain index."
  - "The agent level is inlined at each request site (not a helper) as the plan's acceptance grep expects three with_reasoning( sites."

patterns-established:
  - "A PaladinData field addition registers its MIGRATION 9.2 row, allowlist entry and muster literal in the same commit."

requirements-completed: []
requirements-progress: [REASON-01, REASON-04, REASON-05, REASON-07]

duration: about 3h (cold worktree build, tracer checkpoint round trip)
completed: 2026-10-21
---

# Phase 51 Plan 07: Agent-level reasoning effort Summary

**`PaladinBuilder::reasoning_effort` / `reasoning_fallback` set a per-agent default carried on every request the execution service builds, refused at build when the model cannot honour it, and a fallback chain pre-flights the effort against every hop's own profile before its first call.**

REASON-01, -04, -05 and -07 advance but are not marked complete: the YAML agent field, the build-time provider-key naming, the trace and cache wiring, docs and probe-seeded model data land in plans 51-08 to 51-14. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: `PaladinBuilder::reasoning_effort(High)` reaches the request the execution service sends, with the `PaladinData` break registered | `92025f7f` |
| 2 | Build-time refusal (D-09) and the other two request sites | `53f95d67` |
| 3 | Pre-flight every fallback hop before the first call (D-08), effort metadata surviving the chain's stamping | `69a15b90` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("Approved") before Tasks 2 and 3 began.

## What was built

- **`PaladinData`.** `reasoning_effort: Option<ReasoningEffort>` and `reasoning_fallback: Option<ReasoningFallback>` after `dynamic_temperature`, `#[serde(default, skip_serializing_if = "Option::is_none")]`, `None` in `Default`. A default `PaladinData` serialises with neither key; a document without them loads; `Budget(2048)` round-trips as `{"budget":2048}`. Full literals migrated in `paladin.rs` (module and alias doc tests and `Default`), `council_service.rs`, `graph_doc.rs`, `validator.rs`, `src/core/mod.rs` (doc), `tests/cli_isolation_test.rs` (2) and `tests/consumer_surface_muster.rs` `paladin_literal` (now fifteen fields, comment updated; the workspace compiles with all features, which proves no literal was missed).
- **Builder.** `#[must_use]` setters with `# Examples` (`?`, no panicking calls). `validate` resolves an expressed effort through `profile.effort.resolve(effort, fallback.unwrap_or_default())` and refuses with a `ConfigurationError`: "model 'm' on provider 'p' does not accept parameter 'reasoning_effort' = medium: <refusal>" (a `Refuse` fallback on an unlisted level adds a hint to set a `reasoning_fallback`). A port with no profile refuses an explicit effort ("it reports no model profile, so an explicit reasoning effort cannot be checked"); a builder that sets no effort is never judged. With `NearestLower` the build succeeds and `PaladinData` keeps `Medium` (substitution is per request, at the adapter). A `FallbackLlmAdapter` is checked against its intersected profile.
- **Execution service.** The loop request (about line 3140), the `execute_with_retry` request and the streaming request each call `with_reasoning` / `with_reasoning_fallback` only when the agent set them. `merge_run_metadata` already copies every `paladin.*` key except `served_by`, so no code change: a test pins that the latest call's `paladin.reasoning_requested` / `paladin.reasoning_effective` win.
- **Chain.** Private `FallbackLlmAdapter::preflight_reasoning`, called first in `generate` and `generate_stream`: when the request carries an effort, each hop in chain order is asked `model_profile(step model, else request model)`; no profile or a `resolve` refusal returns `LlmError::unsupported_parameter("reasoning_effort", effort, provider, model, "fallback hop <index>: <reason>")`. No hop is called. A request with no effort skips it. The served-response stamping already uses `insert` for its two keys only (and `with_metadata` on the terminal chunk), so the hop's effort keys survive; tests assert it on both paths. Documented in the type's rustdoc.
- **Register.** MIGRATION 9.2 `Y` row `paladin-ai-core | PaladinData`, allowlist entry (`register-only (unmeasured: cargo-semver-checks absent on 2026-10-21; measured in plan 51-14)`, `REASON-01`), a comment in `crates/paladin-core/Cargo.toml` (the expected lint, `constructible_struct_adds_field`, is already allowed crate-wide), CHANGELOG `### Changed` BREAKING entries in the root and `paladin-core`, `### Added` entries in the root and `paladin-llm`. `.project/current-exports.txt` refreshed (the two builder setters; 4315 to 4319 items).

## TDD record

- **Task 1:** the field, setters and request wiring were written before the tests (order inversion, as in plans 51-04 and 51-06), so there was no RED run. Sensitivity was checked by mutation: replacing `request.with_reasoning(effort)` with a no-op made `the_loop_request_carries_the_agents_effort_and_fallback` fail; reverted. Accepted at the tracer checkpoint.
- **Task 2:** RED run before the implementation: 5 failed (the three refusal cases, the retry site, the streaming site), the others passed. Then GREEN.
- **Task 3:** RED run before the implementation: 5 of 8 new tests failed (unmeasured hop, no-profile hop, lowest-index, ordering, step-model chain); the three that already held (nearest-lower passes, streamed terminal keys, no-effort skip) are regression guards. Then GREEN.

## Verification

- `cargo test --offline -p paladin-ai-core --lib paladin` 37 passed; `-p paladin-ai --lib paladin_builder` and `paladin_execution_service` pass (160 in the combined Task 2 run); `-p paladin-llm --all-features --lib fallback` 92 passed (existing fallback tests unchanged); doc tests for `paladin` and `paladin_builder` pass.
- `cargo test --offline --workspace --all-features --no-fail-fast`: no failures on the second run. The first plain run showed four `application::services::run::cancel_tests` failures (timing under full-workspace load); they pass in isolation (5 of 5) and in the second full run, and touch no code in this plan, so I count them flaky and unrelated.
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --all` clean.
- `make check-gates` (includes `check-migration-allowlist`), `make check-api-examples`, `make check-consumer-surface` exit 0. `make api-surface` failed with the two intended additions before `make api-surface-update`, then exits 0 ("API surface unchanged", 4319 items). The consumer muster passes (`--features llm-anthropic,llm-deepseek`) and now pins the builder setters and the two payload fields.
- Not run: the full `make clean-code` chain and `make security` (no dependency added); `cargo-semver-checks` is absent in this container, so the allowlist lint is register-only until plan 51-14.
- Manual credential-handling review (security.instructions.md): no credential-bearing type touched. The new `ConfigurationError` and `UnsupportedParameter` texts are built from provider name, model, the `reasoning_effort` parameter name, the effort value, the chain index and the refusal's fixed wording only; no key, header, redirect policy, response body or prompt is rendered.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality / prohibition] Builder-setter pin added to the consumer muster**
- **Found during:** Task 1. The plan only asked for `paladin_literal`'s fifteen fields. A new public builder surface with no muster pin is the gap the muster exists to close, so `builder_and_mcp_http_pins` now calls both setters and reads the two payload fields. The function's `PaladinBuilder::new(llm)` became `new(Arc::clone(&llm))` to allow it.
- **Commit:** `92025f7f`

### Notes against the plan's wording

- **No RED run for Task 1** (see TDD record); the no-op mutation is the sensitivity evidence. Disclosed at the checkpoint and accepted.
- **Duplicate allowlist pair accepted.** `paladin-ai-core | PaladinData` already existed from the removed Citadel placeholder (ENTITY-03, `struct_missing`). The new row and entry share the pair; the row-level set-equality gate deduplicates on `crate | type` (the `PaladinResult` precedent), and `make check-migration-allowlist` exits 0. Disclosed at the checkpoint and accepted.
- **`crates/paladin-core/Cargo.toml`** received only a comment (the lint table already allows `constructible_struct_adds_field`); no lint line was added or removed.
- **A3-51-07:** no committed schema or golden that includes `PaladinData` changed (the graph-doc round trip, schema golden and workspace tests pass), so nothing was re-blessed.
- **Streaming metadata path** needed no code change: `merge_run_metadata` is shared and the chain's `with_metadata` stamping adds keys rather than replacing the map.
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used; commits used `--no-verify` as the wave allows.

## Authentication Gates

None.

## Known Stubs

None. Deliberate limitations: the agent effort has no YAML field until plan 51-08; node-cache fingerprinting of an agent effort lands in 51-09; the chain build-time check is the conservative intersection (A1-51-07) and the pre-flight does not look at hop-level provider defaults (A2-51-07). `cargo-semver-checks` measurement of the register entry is plan 51-14.

## Threat Flags

None. T-51-23 (a chain silently losing or changing effort on failover) is mitigated by the pre-flight and the per-hop stamping tests; T-51-24 (snapshots failing to load or changing bytes) by the serde defaults and the round-trip tests; T-51-25 (an agent misconfiguration surfacing only at run time) by the build-time refusal.

## Self-Check: PASSED

- Files: this SUMMARY and every file in `key-files.modified` present.
- Commits `92025f7f`, `53f95d67` and `69a15b90` present in `git log` on `worktree-agent-a05ed1f2bc6190e25`.
