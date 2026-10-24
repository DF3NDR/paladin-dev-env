---
phase: 50-provider-parity-config-driven-model-swap
plan: 11
subsystem: api
tags: [rust, paladin-ports, llm-response, reasoning, semver-register, migration, parity]

requires:
  - phase: 50
    provides: "plan 50-10: the paladin-llm D-27 register and the measured-diagnostic procedure; plan 50-02..50-09 adapters and config that later plans (50-12, 50-13) will teach to fill the new fields"
provides:
  - "LlmResponse.reasoning: Option<String>, directly after cost, #[serde(default, skip_serializing_if = Option::is_none)]; StreamingResponse.reasoning with the same attributes (additive: the struct is #[non_exhaustive])"
  - "LlmResponse::new(request_id, model, content, finish_reason, usage) (fresh id, cost None, reasoning None, created_at now, empty metadata, function_call None), LlmResponse::with_reasoning and StreamingResponse::with_reasoning (empty or whitespace-only text is None), all with # Examples"
  - "all 83 in-tree LlmResponse literals (adapters, mocks, services, tests, benches, examples, the consumer muster, one module-doc literal, three mdBook pages) name reasoning: None, migrated in one commit"
  - "the existing paladin-ports | LlmResponse 9.2 row and allowlist entry extended to both fields (PRICE-03, PARITY-05), 9.8 DR-6 and its insulating paragraph updated, CHANGELOG Added and Breaking Changes bullets"
affects: [50-12, 50-13, 50-16, 51]

tech-stack:
  added: []
  patterns:
    - "Constructor plus chainable with_* for a constructible struct: LlmResponse::new names only the five fields every adapter knows, so the next field is free for callers that use it (DR-6)"
    - "Register rows are extended, never duplicated: a second field on an already-registered struct edits the existing row and entry (set equality by crate|type pair)"
    - "Empty vendor text never becomes Some(\"\"): with_reasoning trims and stores None"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - tests/consumer_surface_muster.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - CHANGELOG.md
    - "42 further files holding LlmResponse literals (see Task 2 commit dc1f25fb): paladin-battalion, paladin-content, paladin-eval, paladin-llm (adapters, mock, pricing, conformance, bench), paladin-memory, three examples, six root-crate services, test helpers and 17 test files, three mdBook pages"

key-decisions:
  - "reasoning sits directly after cost on LlmResponse (A1-50-11); serde ignores position and the muster literal names fields, so position only affects reading order"
  - "The three mdBook pages are updated for accuracy in the same commit (A2-50-11); development-setup.md shows a partial literal ending in a '// ... other fields' comment, so reasoning: None was inserted before the comment by hand"
  - "No new register pair: the D-27 run lists LlmResponse.reasoning under constructible_struct_adds_field, the lint the existing paladin-ports | LlmResponse entry already covers"

patterns-established:
  - "Migrating a constructible struct's literals: a throwaway script outside the repository (scratchpad) closes each literal by brace depth, a precise re-check script counts braces from the literal's own opening brace, and the compiler is the final judge"

requirements-completed: [PARITY-05]

coverage:
  - id: D1
    description: "D-11: LlmResponse and StreamingResponse carry a typed reasoning: Option<String>; metadata is not used for reasoning text"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "paladin-ports llm_port::tests::streaming_with_reasoning_stores_the_text; llm_response_reasoning_round_trips_through_json; llm_response_new_has_no_reasoning (metadata has no reasoning_content key)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Edge PARITY-05/empty (carrier half): with_reasoning stores None for text empty after trimming, on both types"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "llm_port::tests::with_reasoning_ignores_empty_text; streaming_with_reasoning_ignores_empty_text; two doc tests"
        status: pass
    human_judgment: false
  - id: D3
    description: "Runtime State Inventory: a persisted LlmResponse (and StreamingResponse) JSON document without a reasoning key deserializes to None; None serializes with no key"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "llm_port::tests::llm_response_without_reasoning_key_deserializes_to_none; streaming_without_reasoning_key_deserializes_to_none"
        status: pass
    human_judgment: false
  - id: D4
    description: "LlmResponse::new sets the five given fields and neutral defaults, fresh id each call"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "llm_port::tests::llm_response_new_sets_the_given_fields_and_defaults_the_rest; doc test LlmResponse::new"
        status: pass
    human_judgment: false
  - id: D5
    description: "D-00i: every in-tree LlmResponse literal migrated atomically; the workspace builds all targets and all features; make check-consumer-surface exits 0"
    requirement: "PARITY-05"
    verification:
      - kind: integration
        ref: "cargo build --workspace --all-targets --all-features; cargo test --workspace --all-features --no-run; make check-consumer-surface (the_consumer_surface_still_resolves ok); precise literal check: 83 literals, 0 missing"
        status: pass
    human_judgment: false
  - id: D6
    description: "D-00b: the break is measured and registered on the existing row; no new pair; set equality holds"
    requirement: "PARITY-05"
    verification:
      - kind: integration
        ref: "cargo semver-checks check-release --package paladin-ports --baseline-version 0.10.1 --release-type minor --default-features (crate-wide allows disabled, restored, cmp-proven); make check-migration-allowlist; make check-gates; make api-surface"
        status: pass
    human_judgment: false

duration: ~30min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 11: Typed reasoning carriers on LlmResponse and StreamingResponse Summary

**`LlmResponse.reasoning` and `StreamingResponse.reasoning` now exist as typed `Option<String>` fields with `LlmResponse::new` and chainable `with_reasoning` on both, all 83 in-tree `LlmResponse` literals were migrated in one commit, and the second `LlmResponse` break is registered on its existing Phase 38 row from a measured cargo-semver-checks run.**

## Performance

- **Duration:** about 30 min (start 15:04Z, end 15:34Z)
- **Tasks:** 3 of 3 (four task commits: Task 1 has a RED and a GREEN commit)
- **Files:** 46 touched (43 in the Task 2 commit, 3 in the Task 3 commit plus the Task 1 commits on `llm_port.rs`), 0 created besides this SUMMARY

## Accomplishments

- **Task 1 (additive).** `StreamingResponse.reasoning` and `with_reasoning` (additive because the struct is `#[non_exhaustive]`; `delta` and `terminal` set it `None`), and `LlmResponse::new` with a doc test. RED commit `3379129b` had three failing tests against compile-clean scaffolds; GREEN is `1e81fec0`.
- **Task 2 (the registered break).** `LlmResponse.reasoning` after `cost`, `LlmResponse::with_reasoning`, the legacy-JSON, round-trip and empty-text tests, then one commit, `dc1f25fb`, adding `reasoning: None` to every literal in 43 files (201 insertions, no deletions): 40 Rust files, the module-level doc literal in `llm_port.rs`, and the three mdBook pages. The consumer muster's full literal is in that commit and `make check-consumer-surface` exits 0.
- **Task 3 (register).** D-27 measured for `paladin-ports`; the existing 9.2 row names both fields and the new constructor, `PARITY-05` joins `PRICE-03` on the row and the allowlist entry, DR-6 says this is the second break it warned about and the constructor now exists, CHANGELOG gains an `### Added` and a `### Breaking Changes` bullet.

## D-27 diagnostic (measured)

Tool: `cargo-semver-checks 0.50.0` (installed, nothing installed). Command: `cargo semver-checks check-release --package paladin-ports --baseline-version 0.10.1 --release-type minor --default-features`, with the five crate-wide allow lines deleted from `crates/paladin-ports/Cargo.toml`.

- **Headline:** 196 checks: 193 pass, 3 fail, 0 warn, 58 skip. Exit 100 (expected: allows disabled).
- **Lints fired:** `constructible_struct_adds_field`, `enum_unit_variant_changed_kind`, `enum_variant_marked_non_exhaustive`. All three are already in the crate-wide table and in the allowlist.
- **This change:** `constructible_struct_adds_field` lists `LlmResponse.reasoning` (llm_port.rs:1018) beside the Phase 38 `LlmResponse.cost` (llm_port.rs:1007). `StreamingResponse` fires nothing, as predicted. No new lint kind, so no stop-and-report.
- **Other items on the same run (earlier phases', listed, not registered here):** `SubmitRun.attributed_to`, `RunQuery.scope`, `RunOutcomeRecord.halt_reason`, `CreateRunSchedule.created_by`, and `LlmError::RateLimitExceeded` (both variant lints).
- **Restore proof:** the manifest was copied to a scratch directory before editing; after the run `cmp` against the copy printed nothing (identical), the `sha256sum` was `04b53d16...bce95b` before and after, and `git status` shows the file clean.
- `target/semver-checks` was deleted after the run. The raw output is in the session scratchpad (`d27-ports.txt`).
- No separate before-run was made: the diff against the prior state is the one new line, `LlmResponse.reasoning`.

## Task Commits

1. **Task 1 RED** - `3379129b` (test; `reasoning` field and failing-by-design scaffolds, 3 of 5 new tests fail)
2. **Task 1 GREEN** - `1e81fec0` (feat; `with_reasoning`, `LlmResponse::new`)
3. **Task 2** - `dc1f25fb` (feat; the field, constructor method, tests and all 43 files of literals)
4. **Task 3** - `fd858dd7` (docs; `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `CHANGELOG.md`)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Verification

- `cargo build --workspace --all-targets --all-features`: exit 0. `cargo test --workspace --all-features --no-run`: exit 0.
- `cargo test --workspace --all-features --no-fail-fast`: 8500 passed, 18 failed. The 18 are the two ledgered `run_api_wiring` baseline failures (WINDOWS.md row 73) and 16 `integration::redis_queue_integration_test` tests that need a Docker socket (`Failed to start Redis: SocketNotFoundError("/var/run/docker.sock")`) -- environmental, in the `--test lib` target the orchestrator's `--lib --bins` baseline does not run, and unrelated to `LlmResponse`. Everything else is green, including all doc tests.
- `cargo fmt --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`: clean. `make check-gates`, `make check-consumer-surface`, `make api-surface` (unchanged, 4278 items), `make check-api-examples` (105 entry points), `make check-migration-allowlist`, `scripts/check-changelogs.sh`: exit 0. `make clean-code` not run end to end (shellcheck absent).
- Acceptance greps: `pub reasoning: Option<String>` 2 in `llm_port.rs`; `reasoning: None` in the muster 1; `migration_row = "paladin-ports | LlmResponse"` 1 (extended, not duplicated); 9.2 `LlmResponse` row mentions `reasoning`; DR-6 mentions `reasoning`; `PARITY-05` in the allowlist.
- **Manual credential-handling review (D-00a):** no credential path touched; the change adds a response field and mechanical `None` literals, no logging, formatting, HTTP client or config type.
- **Threat model:** T-50-11-01 accepted (same trust class as `content`). T-50-11-02 mitigated (`#[serde(default)]` plus `llm_response_without_reasoning_key_deserializes_to_none` and the streaming twin). T-50-11-03 mitigated (D-27 with `cmp` restore proof above). The PARITY-05 prohibition holds: no reasoning text goes through `metadata` (`llm_response_new_has_no_reasoning` asserts the `reasoning_content` key is absent).

## Decisions Made

See `key-decisions`. Flagged assumptions: A1-50-11 held; A2-50-11 held (the pages are prose examples, updated for accuracy).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Migration script mishandled two literal shapes**
- **Found during:** Task 2 review of the diff, before building
- **Issue:** the module-level `//!` doc literal in `llm_port.rs` got `, reasoning: None })` spliced onto its closing line (its closer line carries the comment prefix), and `development-setup.md`'s partial literal ended in a `// ... other fields` comment, so the script refused to touch it.
- **Fix:** both fixed by hand before any build or commit; the diff was then reviewed to confirm every added line in the other 41 files is exactly `reasoning: None,`.
- **Files modified:** `crates/paladin-ports/src/output/llm_port.rs`, `docs/src/contributing/development-setup.md`
- **Commit:** `dc1f25fb`

**2. [Plan acceptance check has a false positive] the awk literal-coverage check prints `tests/helpers/mock_llm_adapter.rs:198`**
- The check's brace counting includes a leading `}` on the opening line (`} => Ok(LlmResponse {`), so it closes that literal at depth 0 on its own line and reports it missing. The literal does name `reasoning: None` (line 213) and compiles. A corrected check counting braces from the literal's own opening brace reports 83 literals, 0 missing; that is the evidence used. The plan's literal file list matched the tree exactly (no literal outside it), so there is nothing extra to list.

**3. [Rule 3 - Blocking] Disk**
- The first workspace build filled the disk (the shell tool's output file hit ENOSPC). Deleted `target/debug/incremental`, `target/debug/examples`, `target/semver-checks` and the extensionless test binaries in `target/debug/deps` as the baseline notes allow, rebuilt, and deleted `target/debug/incremental` again before the full test run. No tracked file affected; no `cargo clean`.

**4. [Scope note] `.project/current-exports.txt` not committed**
- It is in the plan's file list, but `make api-surface-update` changed only the generated-at timestamp line (the baseline tracks root-crate re-exports and this change adds none; `make api-surface` reports unchanged). The timestamp-only diff was reverted rather than committed.

**5. [TDD gate] Tasks 2 and 3 have no RED commit**
- Task 1 has `3379129b` (RED) before `1e81fec0` (GREEN). Task 2's new tests (legacy-JSON, round trip, empty text) ride in the same commit as the field because the field and every literal must land together; Task 3 is a measured register verified by the tool and the set-equality gate.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path, file access or trust boundary was added; the new field holds model output of the same trust class as `content`.

## Issues Encountered

- `pkill -f 'cargo build'` once matched and terminated the shell running it; no repository state was affected.
- Two background command outputs were lost to the full disk before cleanup; both commands were simply re-run.

## User Setup Required

None.

## Next Phase Readiness

- Plans 50-12 and 50-13 can fill `LlmResponse.reasoning` and `StreamingResponse.reasoning` from vendor bodies (compat reasoning text, Anthropic thinking text); every response constructor in the tree already names the field, and new code should prefer `LlmResponse::new(..).with_reasoning(..)`.
- Plan 50-16 owns the Phase 50 consumer rows in 9.8; DR-6 here already names the second break and the constructor.

## Self-Check: PASSED

- Files: this SUMMARY, `crates/paladin-ports/src/output/llm_port.rs`, `tests/consumer_surface_muster.rs`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `CHANGELOG.md` present.
- Commits `3379129b`, `1e81fec0`, `dc1f25fb`, `fd858dd7` present in `git log`; no deletions in any of them.
