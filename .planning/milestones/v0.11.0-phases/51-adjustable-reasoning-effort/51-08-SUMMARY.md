---
phase: 51-adjustable-reasoning-effort
plan: 08
subsystem: agent-config
tags: [reasoning-effort, agent-yaml, agent-host, build-time-refusal, semver-register]
status: complete

requires:
  - phase: 51-01
    provides: "ReasoningEffort/ReasoningFallback, EffortSupport::resolve, EffortRefusal"
  - phase: 51-06
    provides: "LlmProviderConfig.reasoning_effort / reasoning_fallback, applied per request below the request"
  - phase: 51-07
    provides: "PaladinBuilder::reasoning_effort / reasoning_fallback and the D-09 build-time refusal"
provides:
  - "AgentDefinition.reasoning_effort / reasoning_fallback (YAML agents[].reasoning_effort, agents[].reasoning_fallback)"
  - "Agent host passes both to the builder; the built Paladin carries them and 51-07's threading sends them"
  - "Private name_reasoning_key: an agent-effort refusal ends with (set by `agents[<id>].reasoning_effort`)"
  - "Build-time check of llm.<provider>.reasoning_effort against the agent's own model, refusing by key (Pitfall 6)"
  - "MIGRATION 9.2 Y row and allowlist entry for the AgentDefinition break"
affects: [51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "provider block default is checked at build time but never copied onto the agent, so request > agent > provider holds"
    - "build-time check and adapter send-time check share EffortSupport::resolve"

key-files:
  created: []
  modified:
    - src/config/agents.rs
    - src/infrastructure/web/agent_host.rs
    - src/infrastructure/web/facade_provisioner.rs
    - Cargo.toml
    - .cargo/semver-checks-allowlist.toml
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "The block's fallback wins over the agent's when checking a provider default (mirrors the adapter's per-request resolution); the agent's fallback applies when the block sets none."
  - "A provider default on a model that reports no profile is refused at build as unmeasured (D-03), naming the block key."
  - "Unknown YAML spellings need no custom deserializer: the config loader's error already names the field and echoes the value (tested through config::Config)."

patterns-established:
  - "Agent-level and provider-level refusals both end with (set by `<yaml key>`)."

requirements-completed: []
requirements-progress: [REASON-01, REASON-04, REASON-07]

duration: about 1h 30m (cold worktree build, tracer checkpoint round trip)
completed: 2026-10-21
---

# Phase 51 Plan 08: Per-agent reasoning effort in YAML Summary

**`agents[].reasoning_effort` / `reasoning_fallback` in YAML reach the built Paladin and its model calls, and the agent host refuses at build, naming the exact YAML key, any agent- or provider-level effort the agent's model cannot honour.**

REASON-01, -04 and -07 advance but are not marked complete: trace and cache wiring, docs and probe-seeded model data land in plans 51-09 to 51-14. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: `agents[].reasoning_effort: high` in YAML builds an agent whose model calls carry `high`, with the `AgentDefinition` break registered | `68acc24d` |
| 2 | Build-time refusals name the YAML key: the agent's own, or the provider block's checked against the agent's model | `38e06dc4` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("Approved") before Task 2 began.

## What was built

- **`AgentDefinition`.** `reasoning_effort: Option<ReasoningEffort>` and `reasoning_fallback: Option<ReasoningFallback>` after `timeout_seconds`, `#[serde(default)]`, with rustdoc (spellings, precedence, build refusal) and `reasoning_effort: low` in the struct's YAML example. Literals migrated: three in `agent_host.rs` tests and `spec_to_definition` in `facade_provisioner.rs`. The `AgentDefinition` in `src/application/services/assistant/validator.rs` is a different, unrelated type; that file needed no change.
- **Builder wiring.** `build_agent_with_llm_and_block_temperature` calls `builder.reasoning_effort(e)` / `.reasoning_fallback(f)` when set; the 51-07 builder judges them against the model's profile.
- **Key naming (D-09).** Private `name_reasoning_key(error, key)` mirrors `name_temperature_key`: a `ConfigurationError` naming `reasoning_effort` gets ``(set by `agents[<id>].reasoning_effort`)`` appended when the definition set the effort; anything else, or no key, passes through.
- **Provider default (Pitfall 6).** `BlockReasoning { effort, fallback, key }` is read in `build_agent` from `config.get_provider_config(&provider)` with key `llm.<yaml_provider_key>.reasoning_effort`. When the definition sets no effort and the block does, `check_block_reasoning` resolves the block's value against `llm.model_profile(&def.model)` (block fallback, else the agent's, else `Refuse`; no profile refuses) and returns `HostBuildError::Build` ending in ``(set by `llm.<provider>.reasoning_effort`)``. The value is never passed to the builder, so `PaladinData.reasoning_effort` stays `None` and the adapter still applies it below the agent. An agent that sets its own effort is judged on that value only.
- **Register.** MIGRATION 9.2 `Y` row `paladin-ai | AgentDefinition`, allowlist entry (`register-only (unmeasured: cargo-semver-checks absent on 2026-10-21; measured in plan 51-14)`, `REASON-01`), comment in the root `Cargo.toml` (the crate-wide `constructible_struct_adds_field` allow already covers the expected lint; no lint line added), CHANGELOG `### Changed` (BREAKING) and two `### Added` entries, `.project/current-exports.txt` refreshed (4319 to 4323 items, the four added lines being the two fields on `config::agents::AgentDefinition` and on its `config::AgentDefinition` re-export). Task 2 changed no public item: `make api-surface` was unchanged.

## TDD record

- **Task 1:** RED run before the implementation: the new tests failed to compile (unknown fields, missing imports). Then GREEN.
- **Task 2:** tests written first; the RED state was the compile failure on the missing `name_reasoning_key`, so the behavioural cases were not seen failing individually. Sensitivity was therefore checked by mutation: disabling the `check_block_reasoning` call (`if false && ...`) made `a_blocks_default_effort_the_model_cannot_honour_refuses_the_agent_naming_the_block_key` and `the_agents_fallback_applies_to_a_block_default_that_sets_none` fail; reverted.
- Tests added: `config::agents` 5 (all spellings, unset, YAML round trip through the real `config` loader, unknown spellings naming the field); `agent_host::tests` 4 (YAML effort reaches the Paladin and the request, no effort builds today's agent, agent refusal names its key, `name_reasoning_key` rewords only effort refusals); `agent_host::config_driven_tests` 5 (block default refused by key on an unmeasured model, listed model builds and `data.reasoning_effort` stays `None`, block fallback substitutes, agent fallback applies when the block sets none, agent's own effort judged alone, silent block and agent build as before).

## Verification

- `cargo test --offline -p paladin-ai --lib config::agents`: 25 passed. `cargo test --offline -p paladin-ai --all-features --lib infrastructure::web::agent_host`: 40 passed (the module is compiled only with `--all-features`); the existing agent_host tests are unchanged and pass.
- `cargo test --offline --workspace --all-features --no-run` compiles. `cargo clippy --offline -p paladin-ai --all-targets --all-features -- -D warnings` clean; `cargo fmt --all` clean.
- `make check-gates` (includes `check-migration-allowlist`), `make api-surface`, `make check-consumer-surface`, `make check-api-examples` exit 0.
- Not run: the full `make clean-code` chain, `make security` (no dependency added), and the full workspace test run (only the affected crate's tests; the orchestrator runs the hook chain over the merged wave). `cargo-semver-checks` is absent in this container, so the allowlist lint is register-only until plan 51-14.
- Manual credential-handling review (security.instructions.md): no credential-bearing type touched. The new refusal text is built from model, provider name, the `reasoning_effort` parameter name, the effort value, the refusal's fixed wording and a YAML key only; a test asserts the block's key never appears in the error.

## Deviations from Plan

None - plan executed as written, with these notes:

- **`validator.rs` untouched.** The plan lists it in `files_modified`, but it defines a separate assistant `AgentDefinition` that is not the config type and contains no literal of the changed one.
- **Row text.** The MIGRATION cell says `paladin-ai` (not `paladin-ai (facade)`) so the pair `paladin-ai | AgentDefinition` matches the allowlist entry under the set-equality gate, as the existing `paladin-ai` rows do.
- **Task 2 RED was a compile failure**, backed by the mutation check above.
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used; commits used `--no-verify` as the wave allows.
- **Out of scope (A1-51-08):** the CLI's own agent YAML (`src/application/cli/config/paladin_config.rs`) gains no effort key; plan 51-13's docs say so.

## Authentication Gates

None.

## Known Stubs

None. Deliberate limitations: `FacadeProvisioner`-provisioned agents (`AgentSpec`) carry no effort, so `spec_to_definition` sets both fields to `None`; the node-cache fingerprint of an agent effort is plan 51-09; `cargo-semver-checks` measurement of the register entry is plan 51-14.

## Threat Flags

None. T-51-26 (a mistyped spelling silently treated as unset) is mitigated by the closed serde enums and the config-loader test naming the field; T-51-27 (a provider default that refuses every call, found only at run time) by the build-time block check naming `llm.<provider>.reasoning_effort`.

## Self-Check: PASSED

- Files: this SUMMARY and every file in `key-files.modified` present.
- Commits `68acc24d` and `38e06dc4` present in `git log` on `worktree-agent-a95d8d1fcded977e9`.
