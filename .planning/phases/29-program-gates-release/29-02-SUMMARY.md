---
phase: 29-program-gates-release
plan: 02
subsystem: testing
tags: [openapi, utoipa, serde_json, backward-compatibility, semver, golden-diff]

# Dependency graph
requires:
  - phase: 24-hitl-agent-thread-management
    provides: "thread routes merged into the OpenAPI spec (versioned_thread_parts), the openapi.rs generator/drift-guard seam this plan's test reuses"
  - phase: 27-platform-api
    provides: "run routes merged into the OpenAPI spec (versioned_run_parts), so HEAD's document has the extra paths this plan's restriction excludes"
provides:
  - "A frozen, provenance-checked v0.9.0 OpenAPI baseline (crates/paladin-web/tests/fixtures/openapi-v0.9.0.json) with a sibling README.md recording its blob SHA and producing command"
  - "crates/paladin-web/tests/openapi_golden_v0_9.rs: a 6-test golden diff proving the six pre-existing v0.9 paths, their transitive $ref schema closure, and components.securitySchemes are unchanged in v0.10, with info.version as the one sanctioned normalisation"
  - "SHIP-02's HTTP-surface half, gated on every PR via the existing paladin-web per-crate CI test job (no new CI job needed, D-09)"
affects: ["29-05 (MIGRATION.md §9.6 rewrite citing this test by file name)", "29-09 (the v0.10.0 bump, which regenerates info.version and must keep this test green)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Path-restricted, $ref-closure golden diff at the serde_json::Value level, modeled on the existing openapi_matches_committed_baseline drift guard but scoped to a subset of paths + their closure instead of the whole document"
    - "Frozen historical fixture with sibling README.md provenance (tag, blob SHA, producing command), deliberately offering no regeneration escape hatch"

key-files:
  created:
    - crates/paladin-web/tests/fixtures/openapi-v0.9.0.json
    - crates/paladin-web/tests/fixtures/README.md
    - crates/paladin-web/tests/openapi_golden_v0_9.rs
  modified: []

key-decisions:
  - "info_version_is_the_only_normalisation compares info_sans_version(doc) (the `info` sub-object only, version key removed) rather than whole-document normalisation, because HEAD's generated document still carries 17 extra paths' worth of components.schemas beyond the 8 in the frozen baseline -- comparing whole documents after only stripping info.version would spuriously fail on that unrelated, expected difference."
  - "The test does not hard-assert that the real generated document's info.version differs from the baseline's 0.9.0, since Cargo.toml is still at 0.9.0 at this point in the phase (the version bump is a later plan, 29-09, per CONTEXT.md's stated wave ordering). Instead the test proves the normalisation is scoped to exactly the version key via synthetic before/after documents, which is robust whether or not the real versions currently coincide."

patterns-established:
  - "Golden-diff tests that must survive a later version bump build their own synthetic proof of the normalisation's scope (rather than relying on the real pre-bump/post-bump version strings actually differing at write time)."

requirements-completed: [SHIP-02]

coverage:
  - id: D1
    description: "The six pre-existing v0.9 paths' operation objects are byte-for-byte equivalent (serde_json::Value) between the generated v0.10 spec and the frozen v0.9.0 baseline, with the restriction itself proven non-empty (exactly 6 entries on both sides)"
    requirement: "SHIP-02"
    verification:
      - kind: integration
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs#v0_9_path_restriction_is_non_empty"
        status: pass
      - kind: integration
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs#openapi_v0_9_paths_match_the_frozen_baseline"
        status: pass
    human_judgment: false
  - id: D2
    description: "The transitive $ref closure of the six paths' operations into components.schemas is deep-equal between the two documents, and the closure is non-empty and fully resolved (no dangling $ref)"
    requirement: "SHIP-02"
    verification:
      - kind: integration
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs#ref_closure_schemas_match_the_frozen_baseline"
        status: pass
      - kind: integration
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs#ref_closure_is_non_empty_and_fully_resolved"
        status: pass
    human_judgment: false
  - id: D3
    description: "components.securitySchemes matches in full between the two documents, and info.version is proven to be the ONLY sanctioned normalisation (a synthetic version-only difference normalises away; a synthetic other-field difference does not)"
    requirement: "SHIP-02"
    verification:
      - kind: integration
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs#security_schemes_match_the_frozen_baseline"
        status: pass
      - kind: integration
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs#info_version_is_the_only_normalisation"
        status: pass
    human_judgment: false
  - id: D4
    description: "The frozen v0.9.0 baseline's provenance is verifiable and was NOT produced by regenerating the spec at HEAD or copying HEAD's openapi.json"
    requirement: "SHIP-02"
    verification:
      - kind: other
        ref: "git show v0.9.0:crates/paladin-web/openapi.json | git hash-object --stdin  ==  git hash-object crates/paladin-web/tests/fixtures/openapi-v0.9.0.json  (both f9d22f27f57f8da0957d8bc28c662a546ee6b0a6)"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 02: OpenAPI Golden Diff for the Six Pre-Existing v0.9 Paths Summary

**A frozen, provenance-checked v0.9.0 OpenAPI baseline plus a 6-test path-restricted, `$ref`-closure golden diff proving v0.10's HTTP surface did not change under a v0.9 client's six existing routes.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 2 (1 tracer + 1 TDD expansion)
- **Files modified:** 3 (all new)

## Accomplishments
- Committed `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` verbatim from `git show v0.9.0:crates/paladin-web/openapi.json` — blob SHA `f9d22f27f57f8da0957d8bc28c662a546ee6b0a6`, re-derivable and re-verified with `git hash-object`
- Added `crates/paladin-web/tests/fixtures/README.md` recording the tag, tag commit, producing command, and blob SHA (JSON has no comment syntax)
- Built `crates/paladin-web/tests/openapi_golden_v0_9.rs` with 6 passing tests: path restriction non-emptiness, path-operation equality, `$ref`-closure schema equality, closure non-emptiness/full-resolution, `securitySchemes` equality, and an explicit proof that `info.version` is the only sanctioned normalisation
- No new CI job needed — the test runs under the existing `cargo test -p paladin-web` per-crate CI matrix job (D-09)
- Zero production Rust code touched (X-03 boundary held)

## Task Commits

Each task was committed atomically; Task 2 (TDD) produced a RED then a GREEN commit:

1. **Task 1: One frozen baseline, one restricted diff — generator to golden comparison over the six v0.9 paths** (tracer) - `f74891f4` (feat)
2. **Task 2 RED: add failing tests for the `$ref` closure, `securitySchemes`, and `info.version` rules** - `549f35e3` (test)
3. **Task 2 GREEN: implement the transitive `$ref` closure over `components.schemas`** - `bace1da2` (feat)

**Plan metadata:** this commit (docs: complete plan)

## TDD Gate Compliance

Task 2 (`tdd="true"`) followed the mandatory RED → GREEN sequence:
- **RED** (`549f35e3`): added `ref_closure_schemas_match_the_frozen_baseline` and `ref_closure_is_non_empty_and_fully_resolved`, both calling a stubbed `ref_closure()` that panicked via `todo!()`. `cargo test -p paladin-web --test openapi_golden_v0_9` reported `4 passed; 2 failed` — the 2 failures were exactly the expected `todo!()` panics, confirming a genuine RED (not a compile failure, not an accidentally-passing test).
- **GREEN** (`bace1da2`): implemented `ref_closure()` (frontier-based `$ref` collection and recursive resolution into `components.schemas`, returning a `BTreeMap` for deterministic iteration). All 6 tests passed.
- No REFACTOR commit was needed — the GREEN implementation required no follow-up cleanup.

## Files Created/Modified
- `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` - Frozen, byte-identical `v0.9.0` OpenAPI document (6 paths, 8 schemas, 2 security schemes)
- `crates/paladin-web/tests/fixtures/README.md` - Provenance record (tag, commit, producing command, blob SHA) for the frozen fixture
- `crates/paladin-web/tests/openapi_golden_v0_9.rs` - The path-restricted, `$ref`-closure golden diff (6 tests): `v0_9_path_restriction_is_non_empty`, `openapi_v0_9_paths_match_the_frozen_baseline`, `ref_closure_schemas_match_the_frozen_baseline`, `ref_closure_is_non_empty_and_fully_resolved`, `security_schemes_match_the_frozen_baseline`, `info_version_is_the_only_normalisation`

## Decisions Made
- **`info_version_is_the_only_normalisation` compares `info` sub-objects, not whole documents.** The plan's `<action>` text described "remove `info.version` from both sides before comparing," but a literal whole-document comparison after only stripping `info.version` would spuriously fail: HEAD's generated document carries `components.schemas` for all 23 v0.10 paths (8 more schemas among them just from the assistant/thread/run APIs merged in since v0.9.0), which is an expected, already-tolerated difference the path-restriction and `$ref`-closure tests handle separately, not something this specific test should also be asserting on. Scoping the comparison to `doc["info"]` (via a new `info_sans_version()` helper) isolates the claim this test actually makes.
- **No hard assertion that the real generated document's `info.version` differs from the baseline's `0.9.0` today.** At this point in Phase 29's wave ordering (SHIP-02 tests run before the SHIP-04 version bump, per `29-CONTEXT.md`'s "Claude's Discretion" plan-ordering note), the workspace `Cargo.toml` is still at `0.9.0` — so the two real documents' `info.version` values currently coincide. The test instead proves the normalisation's *scope* (exactly the `version` key, nothing else) using two synthetic documents derived from the real baseline: one with only `version` changed (must normalise away) and one with `title` also changed (must NOT normalise away). This makes the test's guarantee correct both now and after the later bump regenerates the baseline's `info.version`, exactly as D-08 requires ("this test must stay green across it").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed a literal `UPDATE_OPENAPI` token from the module doc comment**
- **Found during:** Task 1 acceptance-criteria check (`grep -c 'UPDATE_OPENAPI' ... is 0`)
- **Issue:** The module doc comment described the file as having "no `UPDATE_OPENAPI`-style regeneration escape hatch," which itself contains the literal token the acceptance criteria requires to be absent from the file.
- **Fix:** Reworded to "no environment-variable-driven regeneration escape hatch," preserving the same meaning without the literal token.
- **Files modified:** `crates/paladin-web/tests/openapi_golden_v0_9.rs`
- **Verification:** `grep -c 'UPDATE_OPENAPI' crates/paladin-web/tests/openapi_golden_v0_9.rs` returns `0`.
- **Committed in:** `f74891f4` (Task 1 commit; caught and fixed before commit)

**2. [Rule 1 - Bug] Fixed `info_version_is_the_only_normalisation`'s comparison scope during RED-phase authoring**
- **Found during:** Task 2 RED-phase test run — the test failed with a real (unintended) difference at `/components/schemas/AssistantListResponse` because the initial `without_info_version()` helper cloned and compared the WHOLE document, not just `info`.
- **Issue:** Whole-document comparison surfaced the (expected, already out-of-scope for this test) difference in `components.schemas` between the 6-path baseline and the 23-path HEAD document.
- **Fix:** Renamed/rescoped the helper to `info_sans_version(doc) -> Value` returning only the (cloned, version-stripped) `info` sub-object, and updated the test to compare `info_sans_version(&generated)` vs `info_sans_version(&baseline)` instead of whole documents.
- **Files modified:** `crates/paladin-web/tests/openapi_golden_v0_9.rs`
- **Verification:** Re-ran the suite; `info_version_is_the_only_normalisation` passed for the intended reason (info fields other than `version` agree) rather than failing for an unrelated reason.
- **Committed in:** `549f35e3` (Task 2 RED commit; fixed before the RED commit was made, so RED reflects only the intended 2 `ref_closure`-related failures)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs caught and fixed before the affected commit landed, so no deviation is visible in the final commit history's test output)
**Impact on plan:** Both fixes were necessary for the test to make the correct claim; no scope creep, no production code touched.

## Issues Encountered
None beyond the two auto-fixed items above.

## Known Stubs
None. Both tasks' production-facing artifacts (the frozen fixture and the test file) are fully wired — no placeholder data, no unimplemented `todo!()` remaining after Task 2's GREEN commit.

## Threat Flags
None. This plan adds no new attack surface — it is two read-only integration tests and a frozen JSON fixture over an already-existing, already-reviewed OpenAPI generator.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- SHIP-02's HTTP-surface half is complete and gated on every PR via the existing `paladin-web` CI test job.
- Plan 29-05 can now rewrite `MIGRATION.md` §9.6's "a golden diff" pointer to cite `crates/paladin-web/tests/openapi_golden_v0_9.rs` by file name.
- Plan 29-09 (the v0.10.0 version bump) will change `info.version` in both the real generated spec and the `openapi_matches_committed_baseline` drift-guard baseline — this plan's `info_version_is_the_only_normalisation` test is specifically designed to stay green across that change, since it proves the normalisation's scope synthetically rather than depending on the current (pre-bump) version strings actually differing.
- No blockers.

## Self-Check: PASSED

- `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` — FOUND
- `crates/paladin-web/tests/fixtures/README.md` — FOUND
- `crates/paladin-web/tests/openapi_golden_v0_9.rs` — FOUND
- Commit `f74891f4` (Task 1) — FOUND in git log
- Commit `549f35e3` (Task 2 RED) — FOUND in git log
- Commit `bace1da2` (Task 2 GREEN) — FOUND in git log

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
