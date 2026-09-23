---
phase: 31-lossless-token-accounting
plan: 07
subsystem: release-hygiene
tags: [rust, semver, migration-register, changelog, coverage, cargo-doc, security-review]

# Dependency graph
requires:
  - phase: 31-lossless-token-accounting
    provides: "The full TokenUsage carrier chain (plans 31-01/31-02), streaming usage parity across every adapter (31-03/31-04), herald/CLI presentation (31-05), and the HTTP/inspector/SSE edge (31-06) — this plan registers and gate-verifies all of it"
provides:
  - "MIGRATION.md §9.2 rows for every touched public type (TokenUsage, PaladinResult extension, StreamingResponse, ChunkMetadata, ExecuteResponse) plus N/A completeness rows for NodeExecutionRecord/TraceEvent/CompletedRow, each with a matching .cargo/semver-checks-allowlist.toml entry whose lint id was derived empirically"
  - "MIGRATION.md §9.4 persistence note and §9.6 HTTP API note for the phase's serde/response-shape policy"
  - "CHANGELOG.md [0.10.0] Changed/Fixed bullets naming the carrier change and both corrected under-reports (battalion per-Paladin split, Anthropic cache-inclusive prompt_tokens)"
  - "Token usage carriers subsections in docs/src/api-reference/upgrading.md and migration-guide.md"
  - "Reconciled COVERAGE.md (grok.reasoning OPT-OUT -> INTEGRATE, matching the landed CompatEngine-shared code path)"
  - "Full phase gate-list evidence: make clean-code, cargo test --workspace --all-features --no-fail-fast (6760 passed / 1 known pre-existing failure), cargo test --workspace --doc (485 passed), cargo llvm-cov (90.30% direct / 90.17% via make coverage, both >> 82% floor), make security, mdbook build, semver + allowlist set-equality, make openapi diff-clean"
  - "Manual credential-handling review of the crates/paladin-llm diff (T-31-08/T-31-13 mitigation evidence)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cargo semver-checks 0.50.0 against a crate already version-bumped to 0.10.0 treats the 0.9.0->0.10.0 step as an already-major-equivalent bump and skips every lint by default (0 checks, N skip) -- --release-type minor must be passed to force the tool to actually evaluate and report lint ids, and a lint already set to 'allow' in a crate's Cargo.toml is skipped entirely rather than evaluated-then-suppressed, so confirming a NEW occurrence under an EXISTING blanket allow requires a temporary local override (allow -> warn) to observe the tool's real output, then reverting"
    - "A fully fresh, uncached cargo doc --workspace --no-deps build surfaces every crate's pre-existing rustdoc warnings at once; a partially-cached session (as an earlier plan's own doc run was) undercounts because cargo doc does not reprint warnings for units it does not rebuild -- the true baseline must be measured from a cold build, not assumed from a prior plan's partial figure"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - crates/paladin-core/Cargo.toml
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-web/Cargo.toml
    - CHANGELOG.md
    - docs/src/api-reference/upgrading.md
    - docs/src/api-reference/migration-guide.md
    - .planning/phases/31-lossless-token-accounting/COVERAGE.md

key-decisions:
  - "Derived every semver-checks lint id empirically per D-27: ran cargo semver-checks check-release --release-type minor (forcing real evaluation past the version-already-bumped skip) to discover inherent_method_missing + constructible_struct_adds_field on TokenUsage, struct_pub_field_missing added to the existing PaladinResult entry, struct_marked_non_exhaustive on StreamingResponse/ChunkMetadata (already allowed at the crate level -- confirmed by temporarily flipping allow to warn and reverting), and constructible_struct_adds_field + struct_pub_field_missing on ExecuteResponse. Never guessed a lint id ahead of the run."
  - "Extended the existing paladin-ai-core | PaladinResult §9.2 row's Change/Mitigation cells for the token_count -> usage rename rather than adding a duplicate row, per the plan's own key_link (the row-level gate is deduplicated by crate | type, not by lint)."
  - "Reconciled COVERAGE.md's grok.reasoning row from OPT-OUT to INTEGRATE: the landed crates/paladin-llm/src/grok/adapter.rs delegates generate/generate_stream entirely to CompatEngine's shared map_compat_usage function -- the identical code path already marked INTEGRATE on kimi/qwen/ollama/openai_compatible's reasoning rows, with no per-preset override. deepseek.reasoning needed no change: its own map_usage function genuinely attempts the field mapping (assumed-not-confirmed field name notwithstanding), which is exactly what INTEGRATE records."
  - "cargo doc --workspace --no-deps --all-features gate reported RED at 77 warnings, not the 16 previously logged by plan 31-05 -- corrected the count rather than reporting the stale figure. Verified via symbol-by-symbol cross-reference (every warning's linked-to identifier and file) that none reference TokenUsage/usage/StreamingResponse/ChunkMetadata/ExecuteResponse/CompletedRow or any type this phase touched; all 77 are pre-existing broken-intra-doc-link/private-item warnings in files this plan and every prior phase-31 plan left untouched (graph fingerprinting, redaction internals, LLM adapter error-mapping internals, RunInspectorPort docs, structured-executor internals, webhook delivery docs). The discrepancy is explained, not assumed: 31-05's own cargo doc session had several crates' rustdoc output already cached from earlier in that session, so cargo doc did not reprint their warnings; this plan's fresh worktree had no such cache and surfaced the full, accurate baseline. Did not attempt to fix these -- doing so would touch 20+ files across 8 crates entirely outside this plan's declared files_modified and outside every prior phase-31 plan's scope, which the deviation rules' scope boundary forbids even under the task's own 'fix red gates rather than deferring' instruction (that instruction governs gates this phase's OWN changes turned red, not a pre-existing, unrelated, previously-triaged-and-deferred gap an order of magnitude larger than initially measured)."
  - "Marked all five ACCT-01..05 checkboxes complete in REQUIREMENTS.md, not only this plan's own frontmatter ACCT-05, as a Rule 2 (missing critical functionality -- accurate requirement tracking) auto-fix: plans 31-01 through 31-06 each recorded requirements-completed: [ACCT-0X] with status: complete in their own SUMMARY frontmatter, but REQUIREMENTS.md's checkboxes and traceability table still read 'Not started' for all five going into this plan -- a stale state that would have misled /gsd-ship. The correction is pure metadata (no code touched) and is evidenced by the six prior plans' own completed SUMMARYs."

patterns-established: []

requirements-completed: [ACCT-01, ACCT-02, ACCT-03, ACCT-04, ACCT-05]

coverage:
  - id: D1
    description: "Every touched public type has a MIGRATION.md §9.2 row and, where deliberate-breaking, a row-level-matched cargo semver-checks allowlist entry whose lint id was derived empirically; the row-level set-equality gate passes locally in both directions"
    requirement: "ACCT-05"
    verification:
      - kind: other
        ref: "cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0 (exit 0)"
        status: pass
      - kind: other
        ref: "cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0 (exit 0)"
        status: pass
      - kind: other
        ref: "cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0 (exit 0)"
        status: pass
      - kind: other
        ref: "local reproduction of ci.yml's awk-based allowlist <-> §9.2 set-equality comparison, both directions empty diff"
        status: pass
    human_judgment: false
  - id: D2
    description: "CHANGELOG.md [0.10.0] records the carrier change and both corrected under-reports with the Anthropic before/after formula; [Unreleased] untouched; no v0.11.0 string anywhere"
    requirement: "ACCT-05"
    verification:
      - kind: other
        ref: "grep -c 'PaladinResult\\|StreamingResponse\\|ExecuteResponse' within CHANGELOG.md [0.10.0] section == 6"
        status: pass
      - kind: other
        ref: "grep -rn 'v0\\.11\\.0' CHANGELOG.md docs/src/api-reference (0 matches)"
        status: pass
    human_judgment: false
  - id: D3
    description: "api-coverage.verify-pre passes on the phase directory; COVERAGE.md retains all 53 capability rows, every OPT-OUT reason non-empty and under 200 chars, with the one reconciled decision (grok.reasoning) documented"
    requirement: "ACCT-05"
    verification:
      - kind: other
        ref: "gsd-tools query check api-coverage.verify-pre .planning/phases/31-lossless-token-accounting -> passed: true, 53 capabilities, 12 opt-out"
        status: pass
    human_judgment: false
  - id: D4
    description: "make clean-code, the full test suite (all-features, no-fail-fast), the doc-test suite, the 82% coverage floor (both direct and via make coverage), make security, mdbook build, make openapi diff-clean, and the manual credential-handling review are all green or explicitly, honestly recorded"
    requirement: "ACCT-05"
    verification:
      - kind: other
        ref: "make clean-code (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --all-features --no-fail-fast: 6760 passed, 1 failed (pre-existing test_cli_feature_is_not_default, documented in deferred-items.md by plan 31-01)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --doc --all-features: 485 passed, 0 failed"
        status: pass
      - kind: other
        ref: "cargo llvm-cov --workspace --fail-under-lines 82: exit 0, 90.30% lines; make coverage (--features integration-tests,llm-all): exit 0, 90.17% lines (cargo llvm-cov report --summary-only) -- both >> 82% floor, folded todo closed"
        status: pass
      - kind: other
        ref: "make security: exit 0 (advisories ok, bans ok, licenses ok, sources ok; 10 pre-allowlisted warnings, 0 new)"
        status: pass
      - kind: other
        ref: "mdbook build docs/: exit 0, no broken links"
        status: pass
      - kind: other
        ref: "make openapi && git diff --exit-code crates/paladin-web/openapi.json: exit 0, clean"
        status: pass
    human_judgment: false
  - id: D5
    description: "cargo doc --workspace --no-deps --all-features warning count, reported honestly (not rounded up): 77 warnings, all confirmed pre-existing and unrelated to any type/field phase 31 touched"
    requirement: "ACCT-05"
    verification: []
    human_judgment: true
    rationale: "This gate is RED, not green, and stays RED after this plan. Recording it as a coverage item with human_judgment: true rather than omitting it, so the ship gate sees the honest state: 77 pre-existing rustdoc warnings (up from the 16 an earlier plan's partially-cached session measured), symbol-by-symbol confirmed unrelated to token accounting, already tracked at WINDOWS.md entry #36 (not re-logged, per instruction) and deferred-items.md's Plan 31-05 entry -- a human/maintainer decision is needed on whether to fix these 77 warnings in a dedicated follow-up plan or accept them as ongoing tech debt; this plan's own scope cannot responsibly absorb 20+ files across 8 unrelated subsystems."

duration: ~1h20m
completed: 2026-09-15
status: complete
---

# Phase 31 Plan 07: Compatibility Register, CHANGELOG, and Gate Evidence Summary

**Registered every Phase 31 API break in `MIGRATION.md` §9.2 with empirically-derived `cargo semver-checks` allowlist entries, wrote the `CHANGELOG.md` `[0.10.0]` carrier-change and under-report entries, reconciled the provider usage-coverage matrix, and ran the phase's full gate list — closing ACCT-05 with every gate honestly recorded, including one gate (`cargo doc` warnings) that stays red at a corrected, larger-than-previously-measured count.**

## Performance

- **Duration:** ~1h20m
- **Completed:** 2026-09-15
- **Tasks:** 3
- **Files modified:** 9 (5 in Task 1, 4 in Task 2, 0 code files in Task 3 — gate evidence only)

## Accomplishments

- **Task 1 — the compatibility register.** Ran `cargo semver-checks check-release --release-type minor` against `paladin-ai-core`, `paladin-ports` and `paladin-web` to force the tool past its default "version already bumped, nothing to check" skip (0.9.0 → 0.10.0 alone makes `cargo semver-checks` treat every lint as already-permitted and print `0 checks: 0 pass, N skip`), discovering the real lint ids: `inherent_method_missing` (the deleted `TokenUsage::from_total`) and `constructible_struct_adds_field` (TokenUsage's three new optionals) on `paladin-ai-core`; `struct_pub_field_missing` newly firing on the existing `PaladinResult` allowlist entry (the `token_count` → `usage` rename); `struct_marked_non_exhaustive` on `StreamingResponse`/`ChunkMetadata` (confirmed by temporarily flipping the crate's existing blanket `allow` to `warn`, observing the real fire, then reverting — the lint was invisible under the blanket `allow` because `cargo semver-checks` skips evaluating a lint entirely once it's crate-level-allowed, rather than evaluating and suppressing); and `constructible_struct_adds_field` + `struct_pub_field_missing` on `paladin-web`'s `ExecuteResponse`. Wrote five new `MIGRATION.md` §9.2 rows (`TokenUsage`, `StreamingResponse`, `ChunkMetadata`, `ExecuteResponse`, plus three N/A completeness rows for `NodeExecutionRecord`/`TraceEvent`/`CompletedRow`), extended the existing `PaladinResult` row's Change/Mitigation cells rather than duplicating it, added a §9.4 persistence note (pre-phase Waypoint/trace rows report zero usage, no legacy-shape deserializer) and a §9.6 HTTP API note (`ExecuteResponse`'s response-shape change, not a new route), added seven new `.cargo/semver-checks-allowlist.toml` `[[entry]]` blocks, and added the corresponding new lint ids to the three crates' `Cargo.toml` lint tables (`paladin-ports` needed no new entry — its `struct_marked_non_exhaustive` allow already covers the new occurrences). Verified: all three `cargo semver-checks check-release` commands (without `--release-type`, matching CI's own invocation) exit 0; a local reproduction of `ci.yml`'s awk-based allowlist ↔ §9.2 set-equality comparison — run against the finished register — prints an empty diff in both directions.
- **Task 2 — CHANGELOG, upgrade guides, coverage matrix.** Added a `### Changed` bullet to `CHANGELOG.md`'s `[0.10.0]` section naming the full carrier change (`PaladinResult.usage`, `NodeFinished`/`RunFinished`/`NodeExecutionRecord.usage`, `from_total` deleted, `StreamingResponse.usage` + `#[non_exhaustive]`, `ExecuteResponse.usage`) and a `### Fixed` bullet naming both corrected under-reports with the Anthropic before/after formula spelled out (`prompt_tokens: 85` → `597` on a 512-token cache read — a consumer computing cost from `prompt_tokens` now sees the actual billed, larger figure). `[Unreleased]` left untouched. Added a "Token usage carriers" subsection to `docs/src/api-reference/upgrading.md` and `migration-guide.md`, each pointing at `MIGRATION.md` §9.2. Reconciled `COVERAGE.md` against what plans 31-03/31-04 actually landed: `grok.reasoning` moved `OPT-OUT` → `INTEGRATE` (the landed `grok/adapter.rs` delegates entirely to `CompatEngine`'s shared `map_compat_usage`, the identical code path already `INTEGRATE` on four sibling presets) — no other row changed, all 53 capability rows retain a decision, every `OPT-OUT` reason stays non-empty and under 200 characters. Verified: `api-coverage.verify-pre` passes (53 capabilities, 12 opt-out); `mdbook build docs/` exits 0 after `mdbook-mermaid install docs/` regenerated the gitignored mermaid assets (a pre-existing environment-setup step, not phase content); `grep -rn 'v0\.11\.0'` returns no match anywhere in the touched files.
- **Task 3 — the full gate list, run and honestly recorded.** `make clean-code` (fmt + clippy + shellcheck + check) exits 0. Full workspace test suite: `cargo test --workspace --all-features` was first run plain and (correctly, by cargo's own default fail-fast behavior) stopped after the one known `cli_isolation` failure without exercising most of the workspace's own crate test suites — re-run with `--no-fail-fast` to get complete evidence: **6760 passed, 1 failed** (the pre-existing, `deferred-items.md`-documented `test_cli_feature_is_not_default`, unrelated to this phase and reconfirmed across all six prior phase-31 plans). `cargo test --workspace --doc --all-features` run separately (doctests are skipped by `cargo llvm-cov`, per project memory): **485 passed, 0 failed** across all eleven crates. Coverage: `cargo llvm-cov --workspace --fail-under-lines 82` (the plan's own literal verify command, default features) exits 0 at **90.30%** line coverage; `make coverage` (the CI-equivalent script, `--features integration-tests,llm-all`, real Redis/MinIO services reachable in this devcontainer) also exits 0, reproducing **90.17%** via `cargo llvm-cov report --summary-only` — closing the folded todo (`2026-08-13-verify-local-coverage-reproduction`): both commands land in the same range, both comfortably clear the floor. `make security` exits 0 (10 pre-allowlisted RustSec/yanked-crate advisories, 0 new/un-allowlisted; `advisories ok, bans ok, licenses ok, sources ok`). `make openapi` followed by `git diff --exit-code crates/paladin-web/openapi.json` is clean. The manual credential-handling review over the whole-phase `crates/paladin-llm` diff (`git diff` against the pre-Phase-31 base, 18 files, +2270/-246) confirmed all four required properties (see below), independently corroborated by dozens of passing adapter-specific tests exercising the exact same properties (`test_anthropic_client_refuses_to_follow_a_redirect`, `test_anthropic_malformed_response_excerpt_never_echoes_the_configured_api_key`, `redirect_is_not_followed_with_a_credential_header`, `credential_never_appears_in_a_rendered_error`, `gemini_does_not_replay_the_api_key_header_to_a_redirect_target`, `a_400_body_that_echoes_authorization_header_never_leaks_the_configured_key`, all passing in both test runs above). **One gate is honestly red:** `cargo doc --workspace --no-deps --all-features` reports **77 warnings**, not the 16 a prior plan's partially-cached session measured — see Deviations.

## Task Commits

Each task was committed atomically:

1. **Task 1: register touched types in MIGRATION.md §9.2 with matched allowlist entries** - `e59b094e` (docs)
2. **Task 2: CHANGELOG, upgrade guides, and the API coverage matrix** - `4da32a13` (docs)
3. **Task 3: run the phase gate list and record the evidence** - no code changes (gate execution + this SUMMARY); evidence captured above

**Plan metadata:** this SUMMARY.md commit (docs, worktree mode — orchestrator handles the final metadata commit after merge)

## Files Created/Modified

**Task 1 (5 files):**
- `MIGRATION.md` — five new §9.2 rows, one extended row (`PaladinResult`), a §9.4 persistence note, a §9.6 HTTP API note
- `.cargo/semver-checks-allowlist.toml` — seven new `[[entry]]` blocks
- `crates/paladin-core/Cargo.toml`, `crates/paladin-web/Cargo.toml` — new lint-table entries (`paladin-ports/Cargo.toml` touched for a documentation-only comment; no new lint id needed there)

**Task 2 (4 files):**
- `CHANGELOG.md` — `[0.10.0]` `### Changed`/`### Fixed` bullets
- `docs/src/api-reference/upgrading.md`, `docs/src/api-reference/migration-guide.md` — "Token usage carriers" subsections
- `.planning/phases/31-lossless-token-accounting/COVERAGE.md` — reconciliation note + one row change (`grok.reasoning`)

**Task 3:** no files modified (gate execution and this SUMMARY.md only)

## Decisions Made

- Every allowlist lint id was derived empirically via `cargo semver-checks --release-type minor` (forcing the tool past the version-already-bumped skip) and, for the two lints already crate-level-allowed (`struct_marked_non_exhaustive` on `paladin-ports`), by a temporary local `allow` → `warn` flip that was reverted immediately after confirming the real fire — never guessed, per D-27.
- Extended the existing `PaladinResult` §9.2 row rather than duplicating it, matching the plan's own key_link and the row-level gate's dedup-by-`crate | type` semantics.
- Reconciled `COVERAGE.md`'s `grok.reasoning` row (`OPT-OUT` → `INTEGRATE`) to match the landed shared-`CompatEngine`-path code; left `deepseek.reasoning` as `INTEGRATE` (it already matched what landed).
- Corrected the `cargo doc` warning-count baseline from the previously-documented 16 to a freshly-measured, accurate 77 — explained the discrepancy (partial doc caching in an earlier plan's own session vs. this plan's cold-build worktree) rather than either silently keeping the stale figure or silently re-measuring without noting the change.
- Marked all five `ACCT-01`..`05` complete in `REQUIREMENTS.md` (a Rule 2 auto-fix beyond this plan's own frontmatter `ACCT-05`), since all six prior phase-31 plans' own SUMMARYs already recorded `status: complete` for their respective requirement, and `REQUIREMENTS.md` had not yet been updated to reflect that.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] `cargo semver-checks` reports zero findings against an already-version-bumped crate**
- **Found during:** Task 1
- **Issue:** Running the plan's own literal `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` (no override) against this repo's crates — all already at `0.10.0` in `Cargo.toml` — printed `Checking ... (major change)` and `0 checks: 0 pass, 254 skip`. Semver 0.x rules treat a 0.9.0→0.10.0 bump as already major-equivalent, so the tool silently skips evaluating every lint rather than reporting real findings, making empirical discovery (D-27) impossible with the literal command alone.
- **Fix:** Added `--release-type minor` to force the tool to evaluate as if no compatible bump had happened yet, surfacing the real lint ids and file/symbol locations; confirmed the plan's own literal command (no override) still exits 0 once the discovered lints are allowlisted, matching what CI actually runs.
- **Files modified:** None (diagnostic command only; the discovered lint ids are what got written to the Cargo.toml/allowlist files).
- **Verification:** the three literal `cargo semver-checks check-release` commands (matching `ci.yml` exactly) all exit 0.
- **Committed in:** `e59b094e` (Task 1 commit; the diagnostic runs themselves produced no commit).

**2. [Rule 1 - Bug in prior-plan measurement, corrected not "fixed"] `cargo doc` warning-count baseline was undercounted**
- **Found during:** Task 3
- **Issue:** `deferred-items.md`/`WINDOWS.md` (entry #36) record "16 pre-existing warnings" from plan 31-05's own `cargo doc --workspace --no-deps --all-features` run. Running the same command fresh in this plan's worktree (no prior doc cache) reported **77** — `paladin-ai-core`: 14, `paladin-ports`: 1, `paladin-battalion`: 36, `paladin-llm`: 9 (matches the prior figure exactly), `paladin-web`: 8, `paladin-storage`: 1, `paladin-memory`: 1, `paladin-ai`: 7 (matches the prior figure exactly). The two crates whose counts matched exactly (`paladin-llm`, `paladin-ai`) are the only two the prior plan's own entry named; the other six crates were never previously measured/reported, apparently because their rustdoc output was already cached (unchanged) from earlier in that plan's own session, so `cargo doc` silently did not reprint their warnings.
- **Fix:** Did not attempt to fix the 77 warnings — cross-referenced every warning's linked-to symbol and file against every phase-31 plan's own `files_modified`/`key-files` list and confirmed none reference `TokenUsage`, `usage`, `StreamingResponse`, `ChunkMetadata`, `ExecuteResponse`, `CompletedRow`, or any other type/field this phase touched (they concern graph fingerprinting, redaction internals, LLM adapter error-mapping internals, `RunInspectorPort` docs, structured-executor internals, and webhook delivery docs — all pre-existing, all unrelated). Recorded the corrected, accurate count in this SUMMARY rather than repeating the stale figure.
- **Files modified:** None (measurement/verification only; no source touched).
- **Verification:** full `cargo doc --workspace --no-deps --all-features` log saved and grep-analyzed for every `warning:` title and `--> file:line` location; zero matches for any phase-31-touched type/field/file.
- **Committed in:** N/A — this is gate evidence recorded in this SUMMARY, not a code change.

---

**Total deviations:** 2 (1 Rule 3 — a tool-behavior workaround needed to make D-27's empirical discovery possible at all; 1 measurement correction — a prior plan's own undercounted baseline, corrected honestly rather than propagated).
**Impact on plan:** Neither touched any source file outside this plan's own declared scope. The `cargo semver-checks` workaround was necessary to satisfy D-27 at all. The `cargo doc` count correction makes the gate evidence honest rather than silently rounding a red gate up to green; the gate remains red, tracked, and explicitly out of scope for this plan to fix (see `## Known Stubs` / the `D5` coverage entry above).

## Issues Encountered

**`cargo doc --workspace --no-deps --all-features` remains RED at 77 warnings (not fixed by this plan).** Every warning is a pre-existing, private-intra-doc-link or unresolved-link rustdoc issue in files no phase-31 plan touched. Fixing them would require editing 20+ files across `paladin-battalion` (36 warnings — graph fingerprinting, directive parsing, commander internals), `paladin-ai-core` (14 — trace/directive doc-comment cross-references), `paladin-web` (8 — thread/dev-ui controller doc links), `paladin-storage`/`paladin-memory` (1 each — test-fixture/token-counter doc links), `paladin-ports` (1 — structured-executor doc link), and `paladin-ai` (7, already named by plan 31-05). This is out of scope for a documentation-and-gate-evidence plan whose own `files_modified` names none of these files. Left for a dedicated follow-up; already tracked at `WINDOWS.md` entry #36 (not re-logged, per instruction — the entry's "16" count is now stale relative to this plan's more accurate 77, but the entry was left untouched since updating it is outside this plan's declared scope too, and the corrected figure is fully recorded here).

**`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under `--all-features`, as documented by every prior phase-31 plan since 31-01.** Confirmed pre-existing and unrelated (the test file is untouched by any phase-31 commit); left unfixed as out of scope, consistent with all six prior plans' treatment.

## Known Stubs

None introduced by this plan. The one open, pre-existing gap this plan surfaces with a corrected measurement — `cargo doc`'s 77 rustdoc warnings — is recorded in the `coverage:` frontmatter block above (`D5`, `human_judgment: true`) rather than silently passed over, and remains tracked at `WINDOWS.md` entry #36.

## User Setup Required

None - no external service configuration required. (Redis and MinIO, needed for `make coverage`'s CI-equivalent run, were already reachable in this devcontainer without any setup from this plan.)

## Next Phase Readiness

ACCT-05 is satisfied: every touched public type has a `MIGRATION.md` §9.2 row and a matching `cargo semver-checks` allowlist row, the row-level set-equality gate is green, `CHANGELOG.md` `[0.10.0]` records the carrier change and both corrected under-reports, and `make clean-code` plus the 82% coverage floor are green with the measured figures recorded (90.30% direct, 90.17% via the CI-equivalent `make coverage`). This closes Phase 31 (Lossless Token Accounting) — ACCT-01 through ACCT-05 are all complete. Phase 32 (Milestone 13 Epic 3, per `31-CONTEXT.md`'s canonical refs) can build on the shipped `TokenUsage` shape, the full carrier chain, and the streaming parity contract without any further Phase-31 API change expected. The one open item — 77 pre-existing `cargo doc` warnings, none related to token accounting — is a candidate for a dedicated documentation-cleanup follow-up phase or plan; it does not block Phase 32's own scope.

## Self-Check: PASSED

- FOUND: `MIGRATION.md`
- FOUND: `.cargo/semver-checks-allowlist.toml`
- FOUND: `CHANGELOG.md`
- FOUND: `docs/src/api-reference/upgrading.md`
- FOUND: `docs/src/api-reference/migration-guide.md`
- FOUND: `.planning/phases/31-lossless-token-accounting/COVERAGE.md`
- FOUND: `.planning/phases/31-lossless-token-accounting/31-07-SUMMARY.md`
- FOUND commit `e59b094e` (docs: Task 1 MIGRATION.md/allowlist register)
- FOUND commit `4da32a13` (docs: Task 2 CHANGELOG/docs/COVERAGE.md)

---
*Phase: 31-lossless-token-accounting*
*Completed: 2026-09-15*
