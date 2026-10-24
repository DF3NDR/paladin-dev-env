---
phase: 50-provider-parity-config-driven-model-swap
plan: 16
subsystem: docs
tags: [rust, paladin-llm, mdbook, drift-test, semver, migration, changelog, adr-0061, parity, phase-gate]

requires:
  - phase: 50
    provides: "50-06 dated registry, 50-09 create_from_config, 50-12/50-13 typed refusal and empty-output rule, 50-14 per-step fallback, 50-15 the 17-case suite, 50-17 the seed probe and R-2, 50-18 R-1 option-b run metadata"
provides:
  - "docs/src/user-guides/switching-providers.md: the config-only swap, library path, ADR-0061 policy, window precedence, per-provider/per-model behaviour table, response-mapping table, fallback chains"
  - "registry test switching_providers_page_lists_every_seeded_model (with twelve controls) that fails when the page and seeded.rs disagree on rows, values, dates or order"
  - "MIGRATION.md 9.8 DR-12..DR-16 plus a Phase 50 behaviour list; root and paladin-ports/paladin-llm/paladin-battalion CHANGELOG entries"
  - "consumer_surface_muster pins for create_from_config, LlmConfig, LlmProviderConfig, ModelOverride, ModelProfile, a Refusal arm, FallbackStep/from_steps, LlmResponse::new and the AnthropicConfig knobs"
  - "closeout D-27 over four crates: no unregistered Phase 50 lint"
  - "ADR-0061 Code Conformance = conforms; PROJECT.md row Implemented; the Phase 50 gate recorded green"
affects: [phase-51, phase-46, web3sec-consumer-bump]

tech-stack:
  added: []
  patterns:
    - "A published behaviour table is guarded by a test that renders each registry row to its exact cells and compares the page cell by cell, with footnote markers stripped and in-file mutation controls"
    - "Skip only when the tree has no docs/ directory (Phase 48 precedent), with a positive control that docs/-without-the-page fails"

key-files:
  created:
    - docs/src/user-guides/switching-providers.md
  modified:
    - crates/paladin-llm/src/registry/mod.rs
    - docs/src/SUMMARY.md
    - docs/src/appendix/provider-expansion.md
    - docs/src/appendix/sentinel.md
    - docs/src/contributing/contributing-providers.md
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - tests/consumer_surface_muster.rs
    - .planning/decisions/0061-per-model-parameter-support-policy.md
    - .planning/PROJECT.md
    - .planning/WINDOWS.md
    - .planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md
    - .planning/phases/50-provider-parity-config-driven-model-swap/deferred-items.md
    - crates/paladin-llm/src/{anthropic,gemini}/adapter.rs
    - crates/paladin-llm/src/{config/bridge,provider_factory,response_guards}.rs

key-decisions:
  - "The drift test compares every column of every seeded row (window, max output, temperature, top_p, penalties, stop, reasoning, JSON mode), not only the model id and date, so the page and the registry agree exactly"
  - "The test is compiled only when all nine adapter features are on; a partial build has no seeded table for the other providers to compare against. The workspace gate (llm-all) and --all-features both compile it"
  - "Adapter-wide fallback rows were measured by building each adapter through create_from_config and reading model_profile for an unseeded id (scratch test, deleted), then labelled unverified on the page; the Kimi fallback row carries a footnote saying it contradicts the 2026-10-15 probe"
  - "Usage-cap column: the five compat presets read 'unverified -- owner accepted (2026-10-16)' exactly as R-2 recorded; OpenAI, Anthropic, DeepSeek and Gemini are described as code-level mappings or absence of a mapping, not as verified, because the probe captured no quota body for any vendor"
  - "The page describes TokenUsage.total_tokens as prompt + completion (the implemented invariant); WINDOWS row 74 is left open and not reopened"

patterns-established:
  - "Closeout D-27: copy each manifest, strip its crate-wide allow lines, run cargo-semver-checks against 0.10.1, restore and prove with cmp, delete target/semver-checks"

requirements-completed: [PARITY-07, PARITY-01, PARITY-02, PARITY-03, PARITY-04, PARITY-05, PARITY-06, PARITY-08]

coverage:
  - id: D1
    description: "The Switching providers page exists, is linked from SUMMARY.md, provider-expansion.md and contributing-providers.md, and carries both tables"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib switching_page (13 tests); grep of the three links; relative-link existence check over four pages"
        status: pass
    human_judgment: false
  - id: D2
    description: "The page cannot drift from the registry: missing, extra, reordered, re-dated or re-valued rows fail, and docs/-without-the-page fails while a docs-less tree skips"
    requirement: "PARITY-07"
    verification:
      - kind: unit
        ref: "switching_providers_page_lists_every_seeded_model plus a_missing_seeded_row, an_extra_row, an_edited_verified_on_date, an_edited_value, rows_out_of_order, a_provider_with_no_fallback_row, a_fallback_row_must_read_unverified, a_missing_response_mapping_row, a_tree_with_docs_but_without_the_page, a_tree_without_a_docs_directory_is_skipped; plus two by-hand mutations of the real file (deleted grok-4.5 row, kimi-k3 date 2026-10-14) that failed with named messages and were restored with cmp"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every Phase 50 break and addition is in the consumer note and the changelogs; the muster pins the adopted surfaces"
    requirement: "PARITY-07"
    verification:
      - kind: integration
        ref: "./scripts/check-changelogs.sh; make check-migration-allowlist; make check-consumer-surface; sed of 9.8 | grep -c '^| DR-1[2-9] |' prints 5"
        status: pass
    human_judgment: false
  - id: D4
    description: "The register is complete: no Phase 50 lint lacks a 9.2 row and allowlist entry"
    requirement: "PARITY-07"
    verification:
      - kind: other
        ref: "closeout D-27, cargo-semver-checks 0.50.0 vs 0.10.1, minor, allows disabled and restored (cmp) for paladin-ports, paladin-llm (default and nine-adapter features), paladin-battalion, paladin-ai"
        status: pass
    human_judgment: false
  - id: D5
    description: "ADR-0061 conforms with measured evidence; R-1 and R-2 answered; PROJECT.md row Implemented"
    requirement: "PARITY-07"
    verification:
      - kind: other
        ref: "grep -q conforms; two R rows, zero pending; PROJECT.md row 1848 reads Implemented"
        status: pass
    human_judgment: true
    rationale: "The Code Conformance paragraph is an authored judgement that the evidence it cites supports 'conforms'; the owner reviews it at phase verification"
  - id: D6
    description: "The phase gate is green except the environmental and ledgered failures"
    requirement: "PARITY-07"
    verification:
      - kind: other
        ref: "cargo test --workspace --features integration-tests,llm-all --no-fail-fast: 7840 passed, 2 failed (WINDOWS row 73 pair); make clean-code, check-gates, api-surface, security, check-consumer-surface, check-migration-allowlist, check-api-examples all exit 0; matrix --ignored --list lists 0"
        status: pass
    human_judgment: false

duration: ~95min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 16: Switching providers page, consumer upgrade note and phase closeout Summary

**The "Switching providers" mdBook page now publishes what each of nine providers and fifteen seeded models was measured to do, kept honest by a registry drift test that compares every cell; every Phase 50 break is indexed in MIGRATION.md 9.8 and the changelogs, the muster pins the surfaces the consumer adopts, ADR-0061 reads `conforms`, and the phase gate is green apart from two ledgered sandbox failures.**

## Performance

- **Duration:** about 95 min (start time was not captured at launch; estimated from the first build and the commit timestamps)
- **Completed:** 2026-10-16T21:59Z (last task commit)
- **Tasks:** 3 of 3, plus one gate-driven fix commit
- **Files modified:** 25 (1 created)

## Accomplishments

- **The page** (`docs/src/user-guides/switching-providers.md`, eight `##` sections): what switching means, the configuration swap (agent `provider`/`model`, every `llm.<provider>` key, `${VAR}` placeholders, the D-13 precedence table, the 300 s reasoning default, `AgentDefinition.timeout_seconds` is not the HTTP timeout), the library path (`create_from_config`, the nine `*Config` builders, facade re-exports and the `llm-*` features with ADR-0046 defaults), the ADR-0061 policy (explicit refused by name, auto omitted with `warn!` and `paladin.params_omitted`, `paladin.unprofiled_model`, `paladin.response_format_adjusted`, the `max_output_tokens` refusal, window precedence), where the keys are visible under R-1 option-b (`LlmResponse.metadata`, `PaladinResult.run_metadata`, the stream's terminal chunk, `ChunkMetadata.run_metadata`; not the HTTP body), known limits, the 24-row behaviour table (15 seeded models plus one fallback row per provider, fixed order), the nine-row response-mapping table, fallback chains (`FallbackStep`, `from_steps`, `agent_runtime.model_fallback.steps`, `paladin.served_by` / `paladin.served_model`, `hop_on_refusal`) and how the table is maintained.
- **The drift test** `switching_providers_page_lists_every_seeded_model` renders each seeded profile to its eleven cells and compares the page cell by cell (footnote markers stripped), asserts the exact `(provider, model)` sequence (nine providers, models ascending, one `(adapter-wide fallback)` row last per provider, each reading `unverified`) and the nine-provider order of the response-mapping table. Twelve sibling tests control it.
- **Docs wiring:** `SUMMARY.md` entry, a cross-link paragraph in `provider-expansion.md` and a new "Conformance Suite and Model Profile" subsection in `contributing-providers.md` (17-case suite, `model_profile`); the three `*Config` literal examples in `sentinel.md` and `provider-expansion.md` rewritten in the option-a shape with `claude-haiku-5-5` and no `claude-3-*` id.
- **Consumer note:** MIGRATION.md 9.8 gains DR-12 (nine `*Config` types), DR-13 (`LlmProviderConfig`), DR-14 (`ModelFallbackConfig`, `AgentRuntimeDeps`), DR-15 (`PaladinResult.run_metadata`, `ChunkMetadata`, `StreamingResponse`), DR-16 (`FallbackChainError`), each pointing at its 9.2 row, plus a Phase 50 behaviour list and the muster paragraph; the deferred `new(..)` harmonisation todo is referenced, not acted on; "Insulating the next bump" already named `LlmResponse::new` and `with_reasoning` (50-11) and now also covers DR-15.
- **Changelogs:** root `[Unreleased]` reconciled (a PARITY-08 `### Fixed` bullet, an `### Added` bullet for the page, suite, matrix and smoke, a Phase 50 `### Known limitations` bullet, a stale R-1 sentence corrected) and new Phase 50 entries in `paladin-ports`, `paladin-llm` and `paladin-battalion`.
- **Muster:** a never-called `phase_50_surface` pins `create_from_config`, `is_configured`, `LlmConfig`, `LlmProviderConfig` builders, `ModelOverride`, `ModelProfile` and `LlmPort::model_profile`, a `LlmError::Refusal { .. }` arm, `FallbackStep` / `from_steps` / `hop_on_refusal`, `LlmResponse::new` plus `with_reasoning`, and the `AnthropicConfig` knob builders, under the existing two features.

## Task Commits

1. **Task 1: page, links, drift test** - `f8bf122a` (docs)
2. **Task 2: consumer note, changelogs, muster** - `92c1e34b` (docs)
3. **Task 3: ADR-0061 conforms, PROJECT.md, ledger** - `5568f177` (docs)
4. **Gate fix (found by `make clean-code`)** - `0c5c4fc2` (fix)

**Plan metadata:** the final `docs(50-16)` commit carrying this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Phase gate (Task 3), command by command

| Command | Result |
|---|---|
| `cargo test --workspace --features integration-tests,llm-all --no-fail-fast` | **Ran to completion.** 58 test binaries plus 6 doctest runs: **7,840 passed, 2 failed, 245 ignored.** The two failures are exactly `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` (`the run never reached a terminal status`, 10 s worker deadline), the ledgered sandbox pair of `WINDOWS.md` row 73, identical on the untouched tree. No other test failed. The `integration::redis_queue_integration_test` cases did not appear in this run, so no Docker-bound failure occurred. The 245 ignored are the standing `#[ignore]` Docker and live-credential cases; none is in `provider_swap_matrix`. |
| `cargo test --test provider_swap_matrix --features llm-all -- --ignored --list` | exit 0, **0 tests listed**; the un-ignored run is 50 passed, 0 ignored |
| `make clean-code` | **exit 0 end to end for the first time this phase** (fmt, clippy `-D warnings`, shellcheck clean, check, the rustdoc zero-warning bar and doctests, `check-api-examples`). Its first run failed on three findings in Phase 50 code, fixed in `0c5c4fc2` (see Deviations) |
| `cargo clippy --workspace --all-features --all-targets -- -D warnings`, `cargo fmt --check` | clean |
| `make check-gates` | exit 0 |
| `make api-surface` | exit 0, "API surface unchanged (4298 items)" |
| `make security` | exit 0 (`advisories ok, bans ok, licenses ok, sources ok`; cargo-audit lists its standing unmaintained-crate warnings, no new dependency was added by this plan) |
| `make check-consumer-surface` | exit 0 |
| `make check-migration-allowlist` | exit 0 |
| `make check-api-examples` | exit 0 (105 of 105 entry points) |

**Could not run here:** Docker-bound suites (no `/var/run/docker.sock`), so CI's `test-integration-docker` job is where those are green; `cargo llvm-cov` (the 82 % workspace line-coverage floor, ADR-0006, is read from CI's `coverage` job and no percentage is quoted); `mdbook` (the page's links were checked by a grep script over four pages, and CI's docs job builds the book); the credential-gated nine-provider `live_vendor_smoke` run (a manual UAT item, flagged assumption A3).

## Closeout D-27 (cargo-semver-checks 0.50.0 vs published 0.10.1, `--release-type minor`, each crate's allows disabled and restored with `cmp`, `target/semver-checks` deleted after)

| Crate | Checks | Lints fired | On Phase 50 items |
|---|---|---|---|
| `paladin-ports` (default features) | 196: 193 pass, 3 fail | `constructible_struct_adds_field`, `enum_unit_variant_changed_kind`, `enum_variant_marked_non_exhaustive` | `LlmResponse.reasoning` only, already on the `paladin-ports \| LlmResponse` row and entry. `LlmError::Refusal`, `StreamingResponse`, `ChunkMetadata`, `ModelProfile` fired nothing. Earlier-phase, not registered here: `LlmResponse.cost`, `SubmitRun.attributed_to`, `RunQuery.scope`, `RunOutcomeRecord.halt_reason`, `CreateRunSchedule.created_by` and the `LlmError::RateLimitExceeded` variant lints |
| `paladin-llm` (default features) | 196: 194 pass, 2 fail | `enum_discriminants_undefined_non_unit_variant`, `struct_marked_non_exhaustive` | `FallbackChainError`, `OpenAIConfig`, `LlmProviderConfig`, all registered |
| `paladin-llm` (all nine adapter features) | 196: 194 pass, 2 fail | the same two | `FallbackChainError` plus all nine `*Config` types and `LlmProviderConfig`, all registered |
| `paladin-battalion` | 196: 193 pass, 3 fail | `enum_struct_variant_field_added`, `function_missing`, `module_missing` | none (Phase 42 `RunOutcome::Halted.cause`, Phase 44 legacy retry removal) |
| `paladin-ai` | 196: 195 pass, 1 fail | `constructible_struct_adds_field` | `AgentRuntimeDeps.llm_config`, `ModelFallbackConfig.steps`, both registered; earlier-phase: `WebhookPayload.halt_reason`, `AgentRuntimeDeps.cadence`, `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, `Settings.treasurer` |

**No unregistered Phase 50 lint remains**, so no row, entry or manifest line was added in this plan. All five manifests restored byte for byte (`cmp` and `sha256sum` before and after in the scratchpad `d27/` outputs). `paladin-ai-core` was not re-run: its only Phase 50 item (`PaladinResult.run_metadata`) was measured at 50-18 and no core file changed since.

One side finding, not Phase 50's: with the repository's own lints table (allows enabled) `paladin-battalion` against 0.10.1 still reports `enum_struct_variant_field_added` on `RunOutcome::Halted.cause` (Phase 42). CI's `semver` job compares against 0.9.0 and so does not see it; it is recorded in `deferred-items.md`, not registered.

## Phase-diff review of added `unwrap(`, `expect(`, `panic!(`

`git diff 865a3968..HEAD` (the parent of the first Phase 50 code commit) over 60 changed non-test-path Rust files, each file split at its first `#[cfg(test)]` module (with a check that no non-test item follows it): **zero** `.unwrap()`, `.expect(`, `panic!(`, `unreachable!(`, `.unwrap_err()` or `.expect_err(` added outside `#[cfg(test)]` code. The matches that exist are all inside test modules (the Anthropic adapter's and the new drift test's).

## Manual credential-handling review (security.instructions.md) over the phase's credential-touching code

- **Redact before truncate:** `Refusal.detail` is built only through `redaction::diagnostic_excerpt(text, api_key)` in `response_guards::classify_completion` (credentials removed, then bounded by scalar values); the Anthropic refusal text and the `EmptyCompletion` block census go through the same function; the suite case `refusal_is_typed_and_permanent` plants each adapter's own key in the vendor's refusal text on all nine adapters and asserts the detail never contains it.
- **No key in logs, `Debug`, metadata or errors:** no `warn!`/`info!`/`debug!`/`error!` in the phase diff interpolates a key (grep: none); the eight key-holding `*Config` types and `LlmProviderConfig` have manual `Debug` impls printing `<redacted>`, each with a test asserting the secret never appears; `FallbackStep`'s `Debug` prints provider name and model only; the `paladin.*` metadata values are provider names, model ids and parameter names; layering errors name provider, model and the variable tried, never a value.
- **Redirects refused on every adapter, including factory-built ones:** the conformance `redirect_is_not_followed_with_a_credential_header` (307, with `GET` and `POST` mocks at `expect(0)`, mutation-checked at 50-15 to fail a following client) runs on all nine fixtures; `factory_built_adapter_refuses_redirects` (50-09, mutation-checked) covers adapters built through `create_from_config`.
- **`${VAR}` placeholders never sent literally:** only an exact `${NAME}` is a placeholder; an unset or blank variable is a construction error naming provider, model and variable (`an_unset_or_blank_placeholder_variable_is_missing_not_literal`, `lenient_resolution_never_errors_and_never_sends_a_placeholder`, and the wire-level `a_placeholder_key_is_resolved_and_never_sent_literally`).
- **This plan's own surface:** documentation, a test that reads one checked-in file, and constant-string pins; the page shows only `${...}` placeholders and no key shape; the muster passes only empty strings.

## Files Created/Modified

See `key-files` above. The page is the only created file; the three adapter and helper edits in `0c5c4fc2` are doc-comment and `allow(dead_code)` changes with no behaviour change.

## Decisions Made

See `key-decisions`. The two worth a reviewer's attention: the drift test compares all eleven columns (so a changed window or temperature range on either side fails it), and the usage-cap column does not call OpenAI, Anthropic, DeepSeek or Gemini "verified" because the probe captured no quota body for any vendor.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `make clean-code` failed in `doc-check` on Phase 50 code**
- **Found during:** Task 3 (first end-to-end `make clean-code`, enabled by shellcheck now being installed)
- **Issue:** public rustdoc linked to private items in `config/bridge.rs` (`resolve_layers`, also a redundant explicit link), the Anthropic `generate_stream` doc and the Gemini `generate_stream` doc (`classify_completion`), failing the zero-warning bar and, under `--all-features -D warnings`, the build; separately a default-feature `cargo check --all-targets` warned that the test helper `provider_factory::tests::from_config::env_of` is unused without the `kimi` feature.
- **Fix:** plain code spans instead of intra-doc links; a reasoned `#[allow(dead_code)]` on `env_of`.
- **Files modified:** `crates/paladin-llm/src/config/bridge.rs`, `anthropic/adapter.rs`, `gemini/adapter.rs`, `provider_factory.rs`
- **Verification:** `make clean-code` exits 0 end to end; `cargo clippy --workspace --all-features --all-targets -- -D warnings` clean
- **Committed in:** `0c5c4fc2`

**2. [Rule 1 - Bug] A non-test warning in single-feature builds (the plan 50-15 deferred item)**
- **Found during:** Task 3 (checking the gap `deferred-items.md` recorded)
- **Issue:** `cargo build -p paladin-llm --no-default-features --features gemini` (and `deepseek`) warned that `StreamCompletion::observe_refusal` is never used, a plan 50-12/50-13 item a consumer building one adapter would see.
- **Fix:** a reasoned `#[allow(dead_code)]`; both builds and `cargo clippy --lib -D warnings` are now clean. The test-build warnings in the same single-feature builds stay deferred (recorded).
- **Files modified:** `crates/paladin-llm/src/response_guards.rs`, `deferred-items.md`
- **Committed in:** `0c5c4fc2`

**3. [Rule 2 - Missing critical] Root CHANGELOG gaps and a stale statement**
- **Issue:** the root `[Unreleased]` had no entry for the PARITY-08 user-turn fix, the page, suite, matrix and smoke, or the Phase 50 known limits, and its 50-03 bullet still said streamed calls warn only "until R-1 decides".
- **Fix:** a `### Fixed` bullet (worded from the commit evidence: the OpenAI adapter read `context` only), an `### Added` bullet, a `### Known limitations` bullet, and the R-1 sentence corrected.

**4. [Rule 2 - Missing critical] Stale facts beside the rewritten literals in `provider-expansion.md`**
- **Issue:** the page says its capability table is "read directly from each adapter's `get_capabilities()`", but the Max Context row (and prose such as "64K", "200K") predated plan 50-06's default-model windows; the YAML used a `model:` key that is not a block key, `deepseek-chat` and `claude-3-5-sonnet-20241022` defaults, and a three-argument `AnthropicConfig::new` that no longer exists.
- **Fix:** the row now matches the measured `get_capabilities()` of each adapter, the prose points at the per-model table, the YAML uses `default_model` with the current defaults, and the literals are in the option-a shape. Broader content staleness stays Phase 46's.

**5. [Plan wording interpreted] The drift test is gated on all nine adapter features**
- The plan says the test skips only when `docs/` is absent. It also needs every seeded table, so the module is `#[cfg(all(test, <nine features>))]`; both the workspace gate and `--all-features` compile it. The no-`docs/` skip and the docs-without-the-page failure behave as planned and have controls.

**6. [Plan wording interpreted] "`verified (<date>)` or `unverified -- owner accepted (<date>)` per compat vendor"**
- No compat vendor has captured evidence, so all five read `unverified -- owner accepted (2026-10-16)`; the four non-compat vendors are described by their code-level mapping (or its absence) and are explicitly not part of R-2.

**7. [Process] Two ledger rows added, three closed, in the closeout register check**
- Closed on measured evidence: row 70 (the full workspace run completed), row 71 (50 passed, 0 ignored, `--ignored --list` empty) and row 72 (`make clean-code` end to end). Left open: row 73 (the sandbox pair) and row 74 (`total_tokens`, owner decision pending, not reopened). Added: row 75 (the OpenAI adapter sends `max_tokens`, which `gpt-5.4-nano` rejects) and row 76 (the Kimi adapter-wide fallback profile contradicts the probe), both carried unfixed from 50-06/08/09/15 with no ledger entry until now.

### Notes, not deviations

- The by-hand drift verification the plan asks to record: with the real file, deleting the `grok-4.5` row failed with `the page has no behaviour row for provider 'grok' model `grok-4.5``, and changing the `kimi-k3` date failed with `kimi `kimi-k3`: column 2 reads `2026-10-14` on the page but the registry says `2026-10-15``; the file was restored and `cmp` confirmed it.
- `cargo fmt -- tests/consumer_surface_muster.rs` was used once; it reformatted only that file.
- The seeded tables are not in model-id order in `seeded.rs` (OpenAI, Grok, Gemini); the test sorts, as the page does.
- The deferred `new(..)` harmonisation todo is referenced from DR-12 and the page and is not acted on.

## Known Stubs

None. The page's "not set" cells are recorded absences (no value in the probe), not placeholders, and the plan's documented manual items (the live smoke run) are listed above as not run here.

## Threat Flags

None. The plan adds documentation, a test that reads one checked-in file under `CARGO_MANIFEST_DIR`, muster pins and a ledger entry; it adds no endpoint, auth path or schema. T-50-16-01 (stating unverified behaviour as fact) is mitigated by the drift test and the unverified labels; T-50-16-02 by the closeout D-27; T-50-16-03 by the credential review above; T-50-16-04 by the phase-diff review.

## Issues Encountered

- `make clean-code` could only run end to end now that shellcheck is installed; it found the doc-check failures above, which no earlier plan could have seen.
- Disk stayed above 11 GB free throughout; `target/semver-checks` was deleted after each D-27 run and no `cargo clean` was used.

## User Setup Required

None. Manual UAT (not a gate): the operator runs `live_vendor_smoke` with keys over the nine providers, and CI's `coverage`, `semver`, docs and `test-integration-docker` jobs are the proof of record for what could not run here.

## Next Phase Readiness

- Phase 50 is complete: 18 of 18 plans. Open owner items carried forward: WINDOWS rows 73 (CI is the proof), 74 (`total_tokens`), 75 (OpenAI `max_tokens` against `gpt-5.4-nano`) and 76 (Kimi fallback profile), and the deferred `new(..)` harmonisation todo.
- Phase 51 (adjustable reasoning effort) adds to the same `#[non_exhaustive]` `ModelProfile` and configs without a further literal break.

---
*Phase: 50-provider-parity-config-driven-model-swap*
*Completed: 2026-10-16*

## Self-Check: PASSED

Created file `docs/src/user-guides/switching-providers.md` and this SUMMARY exist; commits `f8bf122a`, `92c1e34b`, `0c5c4fc2` and `5568f177` are present in `git log`; no file was deleted by any of them.
