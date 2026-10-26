---
phase: 40-tenant-identity-run-read-scoping
plan: 06
subsystem: release-records
tags: [cargo-semver-checks, migration-register, changelog, adr, cargo-public-api, windows-ledger, tenant-scoping]

# Dependency graph
requires:
  - phase: 40-tenant-identity-run-read-scoping
    provides: "40-01 principal types + Principal.tenant_id + Run.submitted_by; 40-02 RunQuery.scope; 40-03 ApiKeyConfig.tenant / BearerTokenAuthConfig.tenant / AuthConfig::validate; 40-04 RunScope.ledger_scope + defaulted executor-port methods; 40-05 RunResponse.submitted_by + load_visible_run on every /runs/{run_id}* route"
  - phase: 39-cost-attribution
    provides: "39-08 D-27 measurement method (disable crate-wide semver allow lines, measure against the published baseline with --release-type minor, restore byte-for-byte)"
provides:
  - "MIGRATION.md 9.2 carries an empirical cargo-semver-checks result on every Phase 40 row (15 rows, 7-pipe), 9.5 records the four 40-03 boot rejections, 9.8 names the one required config edit (tenant on every API key, bearer_token.tenant when bearer is enabled)"
  - ".cargo/semver-checks-allowlist.toml reconciled to the tool: paladin-web | Principal fires struct_marked_non_exhaustive only; ./scripts/check-migration-allowlist.sh exits 0"
  - "ADR-0054 tenant-scoped run reads (Accepted; D-02 server-derived tenant, D-05 required mapping, D-11 tenant visibility with Admin bypass, 404-not-403), indexed in PROMOTION.md (next free 0055) and PROJECT.md Key Decisions -- one commit (D-00f)"
  - "Root CHANGELOG [Unreleased] Added/Changed/Breaking Changes for the phase; paladin-core, paladin-ports, paladin-storage, paladin-web CHANGELOGs name their own changes"
  - "Facade re-export pub use paladin_core::platform::container::principal in src/core/platform/mod.rs; .project/current-exports.txt refreshed on nightly-2026-09-20 (4035 -> 4044 items)"
  - "WINDOWS.md: row 57 waived, row 58 (thread-route read-scope gap, D-14, owner Phase 41) open, row 59 (current-exports acceptance grep is unsatisfiable by tool design) open for operator waiver"
  - "Full gate run on the final tree: cargo test --workspace, cargo fmt --check, make clean-code, make security, make check-gates, make openapi (no diff), make api-surface -- all green"
affects: [phase-41, allowances, v0.11-release, thread-route-scoping]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Measure, do not predict: every 9.2 row's lint column is what cargo-semver-checks 0.50.0 actually reported against the published baseline; a register entry the tool contradicts is corrected, not defended"
    - "A pub mod carries either an outer /// doc or inner //! docs, never both: rustdoc merges the two, loses spans and resolves the combined text in the parent scope, so every intra-doc link in the inner docs becomes a broken_intra_doc_links warning with no source location"
    - "Facade module re-exports appear in .project/current-exports.txt as a single pub use line (cost, treasury_ledger, principal); acceptance greps must target the re-export line, not the module's items"

key-files:
  created:
    - .planning/decisions/0054-tenant-scoped-run-reads.md
  modified:
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - src/core/platform/mod.rs
    - .project/current-exports.txt
    - crates/paladin-core/src/platform/container/mod.rs
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md
    - .planning/WINDOWS.md

key-decisions:
  - "paladin-web | Principal allowlist entry reduced to struct_marked_non_exhaustive: the tool does not fire constructible_struct_adds_field for a field added to a struct that becomes #[non_exhaustive] in the same change, so 40-01's two-lint prediction was corrected to the one measured lint (Rule 1)"
  - "No Cargo.toml semver allow line was added: every lint the D-27 diagnostic fired is already covered by an existing crate-wide allow, so the register gained rows and empirical notes but no new suppression"
  - "WINDOWS.md row 32 stays waived (Phase 29 accepted deviation): gsd-tools windows fixed 32 refuses with WINDOWS_ALREADY_RESOLVED, hand edits are forbidden, and the closing condition is recorded as met in the CHANGELOG and ADR-0054 instead"
  - "Row 59 filed rather than forcing the RunReadScope grep: cargo-public-api lists a re-exported module as one line; the facade-visible Phase 40 items are all present in the refreshed baseline and make api-surface exits 0"
  - "The rustdoc fix (drop the outer /// on pub mod principal) is a doc-only change with no public-API item added or removed; make api-surface confirmed 'API surface unchanged' after it"

patterns-established:
  - "Register reconciliation runs the measurement first and edits the prose second; the allowlist script's set-equality check is the acceptance gate, not a grep on predicted lint names"
  - "Every Phase 40 facade-visible item is asserted via the refreshed baseline plus make api-surface, not via item-name greps that the re-export line cannot satisfy"

requirements-completed: [TENANT-01, TENANT-02, PLAT-07]

coverage:
  - id: D1
    description: "Every Phase 40 public-API change measured with cargo-semver-checks 0.50.0 against v0.10.1 (--release-type minor, D-27 method, lints restored byte-for-byte) and against the CI-pinned v0.9.0 baseline; 9.2 rows carry the empirical result; allowlist set-equal to the register"
    requirement: TENANT-01
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh (exit 0); 11-package CI-parity cargo semver-checks --baseline-version 0.9.0 (all exit 0); D-27 diagnostic over paladin-ai, paladin-ai-core, paladin-ports, paladin-storage, paladin-web (RESTORED_CLEAN)"
        status: pass
    human_judgment: false
  - id: D2
    description: "ADR-0054 with the seven PROMOTION.md headings, Accepted, conforms; PROMOTION.md index row and next-free 0055; PROJECT.md Key Decisions row -- all in commit 39ab90ae"
    requirement: TENANT-02
    verification:
      - kind: other
        ref: "grep -c '^## ' .planning/decisions/0054-tenant-scoped-run-reads.md; grep 'Next free ADR number: 0055' .planning/decisions/PROMOTION.md; git show --stat 39ab90ae"
        status: pass
    human_judgment: false
  - id: D3
    description: "CHANGELOG entries in root and the four touched crates; facade re-export of principal; .project/current-exports.txt refreshed and make api-surface exits 0"
    requirement: PLAT-07
    verification:
      - kind: other
        ref: "make check-changelogs (11 crates); PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface ('API surface unchanged', 4044 items)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Final tree passes cargo test --workspace, cargo fmt --check, make clean-code, make security, make check-gates, make openapi (no diff), make api-surface"
    requirement: PLAT-07
    verification:
      - kind: integration
        ref: "cargo test --workspace: 57 suites, 6067 passed, 0 failed, 229 ignored"
        status: pass
      - kind: other
        ref: "cargo fmt --check; make clean-code; make security; make check-gates; make openapi && git diff --exit-code crates/paladin-web/openapi.json; make api-surface -- all exit 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "Manual credential-handling review of the Phase 40 diff (629ef660..HEAD): no key value in any log, error, response, row, doc or example"
    requirement: TENANT-01
    verification: []
    human_judgment: true
    rationale: "security.instructions.md names this a manual review because no merge-gating Rust SAST exists; the review result is recorded below under Credential-handling review"
  - id: D6
    description: "Workspace line coverage >= 82% and the Postgres-gated repository tests"
    requirement: PLAT-07
    verification:
      - kind: other
        ref: "CI coverage job (cargo llvm-cov --fail-under-lines 82) and Postgres integration tests -- both need Docker services this sandbox lacks"
        status: unknown
    human_judgment: false

# Metrics
duration: ~60min (including a disk-exhaustion pause and resume)
completed: 2026-09-29
status: complete
---

# Phase 40 Plan 06: Release Records, Register Reconciliation and the Final Gate Run Summary

**Every Phase 40 API change is now measured rather than predicted in the X-10 register, the ADR-0054 trio is published, all five CHANGELOGs and the public-surface baseline are refreshed, and the final tree passes every gate CLAUDE.md names.**

## Performance

- **Duration:** ~60 min wall clock (approx. 11:20Z start; paused at 11:58Z when the disk hit 0 bytes free during `cargo test --workspace`; resumed 12:0xZ after the orchestrator freed 13 GB)
- **Started:** 2026-09-29T11:20:00Z (approx.)
- **Completed:** 2026-09-29T12:15:00Z (approx.)
- **Tasks:** 3/3
- **Files modified:** 14 (1 created, 13 modified)

## Accomplishments

- Ran the 39-08 D-27 measurement over all five Phase 40 packages against the published v0.10.1 baseline (`--release-type minor`, crate-wide allows disabled and restored byte-for-byte, `RESTORED_CLEAN`) and the 11-package CI-parity loop against v0.9.0; every one of the 15 Phase 40 rows in MIGRATION.md 9.2 now carries its empirical result, and one register entry the tool contradicted (`paladin-web | Principal`) was corrected.
- Promoted D-02/D-05/D-11 and the 404-not-403 rule into ADR-0054 (Accepted, conforms), indexed in PROMOTION.md (next free 0055) and PROJECT.md in a single commit per D-00f.
- Recorded the phase in the root CHANGELOG `[Unreleased]` (Added, Changed, Breaking Changes) and in the paladin-core, paladin-ports, paladin-storage and paladin-web CHANGELOGs; re-exported `principal` from the facade; refreshed `.project/current-exports.txt` on the CI-pinned nightly (4035 -> 4044 items).
- Filed the D-14 thread-route deferral as WINDOWS.md row 58 with its closing condition and owner, waived row 57, and filed row 59 for the plan's unsatisfiable `RunReadScope` acceptance grep.
- Fixed the one gate failure the final run exposed: 7 span-less `broken_intra_doc_links` warnings in `paladin-ai-core` caused by an outer `///` on `pub mod principal;` merging with the module's inner docs; `make doc-check` (ADR-0033) now passes.

## Task Commits

Each task was committed atomically:

1. **Task 1: Measure every Phase 40 API change with cargo-semver-checks and reconcile MIGRATION.md 9.2/9.5/9.8 and the allowlist** - `8311a032` (docs)
2. **Task 2: CHANGELOG entries, the facade re-export of the principal module, and the refreshed public-surface baseline** - `f40dba0e` (feat)
3. **Task 3: ADR-0054 with PROMOTION.md and PROJECT.md, WINDOWS.md rows, and the full gate run** - `39ab90ae` (docs) + `be3a9030` (fix: rustdoc intra-doc links, found by the gate run)

**Plan metadata:** see the final `docs(40-06)` commit recorded in the orchestrator's completion report.

## Files Created/Modified

- `MIGRATION.md` - 9.2: 15 Phase 40 rows with empirical semver-checks results (RunQuery, RunScope, both executor ports, AuthConfig::validate, ApiKeyConfig/BearerTokenAuthConfig.tenant, Principal, AgentAuthConfig.bearer_tenant, RunResponse.submitted_by, the principal types); 9.5: the four 40-03 boot rejections with their message prefixes; 9.8: the one required config edit
- `.cargo/semver-checks-allowlist.toml` - `paladin-web | Principal` reduced to `struct_marked_non_exhaustive`; set-equal with 9.2's `Y` rows
- `CHANGELOG.md` - `[Unreleased]` Added / Changed / Breaking Changes for Phase 40
- `crates/paladin-core/CHANGELOG.md`, `crates/paladin-ports/CHANGELOG.md`, `crates/paladin-storage/CHANGELOG.md`, `crates/paladin-web/CHANGELOG.md` - each crate's own Phase 40 changes (paladin-web names `RunAttributionDto`)
- `src/core/platform/mod.rs` - `pub use paladin_core::platform::container::principal;`
- `.project/current-exports.txt` - refreshed baseline, nightly-2026-09-20, 4044 items
- `crates/paladin-core/src/platform/container/mod.rs` - outer `///` on `pub mod principal;` removed (rustdoc fix)
- `.planning/decisions/0054-tenant-scoped-run-reads.md` - ADR-0054, seven PROMOTION headings, Accepted, `## Code Conformance` conforms
- `.planning/decisions/PROMOTION.md` - 0054 index row, `**Next free ADR number: 0055**`
- `.planning/PROJECT.md` - Key Decisions row for ADR-0054
- `.planning/WINDOWS.md` - row 57 waived; rows 58 and 59 opened (tool-written)

## D-27 diagnostic: what cargo-semver-checks 0.50.0 fired per package (vs v0.10.1, `--release-type minor`, crate-wide allows disabled)

| Package | Phase 40 lint lines fired | Phase 40 items that stayed silent | Pre-existing (already registered) |
|---|---|---|---|
| `paladin-ai` | `constructible_struct_adds_field`: `ApiKeyConfig.tenant` (src/config/agents.rs:101), `BearerTokenAuthConfig.tenant` (agents.rs:113) | - | `Settings.treasurer` |
| `paladin-ai-core` | none | `Run.submitted_by`, `RunScope.ledger_scope`, `LedgerScope::from_attribution` (both structs `#[non_exhaustive]`) | `PaladinResult.cost`, `TraceEvent::NodeFinished/RunFinished.cost` (unchanged since 39-08) |
| `paladin-ports` | `constructible_struct_adds_field`: `RunQuery.scope` (run_repository_port.rs:79) | `SubmitRun`, `ForkRun`, `RunSubmissionPort` (register-only confirmed); the two defaulted executor-port methods | `LlmResponse.cost` |
| `paladin-storage` (no lints table) | none -- `196 checks: 196 pass` | - | - |
| `paladin-web` | `constructible_struct_adds_field`: `AgentAuthConfig.bearer_tenant` (agent_auth.rs:116), `RunResponse.submitted_by` (run_controller.rs:434); `struct_marked_non_exhaustive`: `Principal` (agent_auth.rs:47) | `Principal.tenant_id` did **not** fire `constructible_struct_adds_field` (a struct made `#[non_exhaustive]` in the same change is no longer exhaustively constructible) | `ExecuteResponse.cost`, `RunResponse.cost` |

**Outcome:** no Cargo.toml lint line was added -- every fired lint is under an existing crate-wide allow. All five lints tables were restored byte-for-byte (`git diff --quiet` on the five Cargo.toml files after the diagnostic). The 11-package CI-parity loop against v0.9.0 exited 0 for every package with `0 checks: 0 pass, 254 skip` -- the standing 38/39 no-op finding.

## Gate results (final tree, after `be3a9030`)

| Gate | Result |
|---|---|
| `./scripts/check-migration-allowlist.sh` | pass (set-equal after the Task 1 edits) |
| 11-package CI-parity `cargo semver-checks --baseline-version 0.9.0` | pass, all 11 exit 0 (`0 checks: 0 pass, 254 skip` each); run once, before the doc-only Task 1 edits (MIGRATION.md and the allowlist are not tool inputs) |
| D-27 diagnostic (5 packages vs v0.10.1, `--release-type minor`) | ran; `RESTORED_CLEAN`; findings in the table above |
| `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update` / `make api-surface` | pass (4035 -> 4044 items; "API surface unchanged" on the final re-check after the rustdoc fix) |
| `make check-changelogs` | pass (11 crates) |
| `cargo test --workspace` | pass: 57 suites, 6067 passed, 0 failed, 229 ignored (Docker/Postgres-gated) |
| `cargo fmt --check` | pass |
| `make clean-code` | **first run failed at `doc-check`** (7 `broken_intra_doc_links` warnings in `paladin-ai-core`); fixed in `be3a9030`; second run pass (doc-check ADR-0033, shellcheck, doctests); no fmt mutations (`git status` clean afterwards) |
| `make security` (cargo-audit + cargo-deny) | pass (exit 0; the RUSTSEC entries printed are the register-tracked advisories, cross-checked by `check-advisory-register`) |
| `make check-gates` | pass (changelogs, crate names, advisory register, workflow suppressions, workflow triggers, CodeQL dismissals, migration allowlist, publish order) |
| `make openapi && git diff --exit-code crates/paladin-web/openapi.json` | pass (regenerated, no drift) |
| Coverage >= 82 % (`cargo llvm-cov --fail-under-lines`) | **CI-only** -- the `coverage` job needs Docker services this sandbox lacks |
| Postgres-gated repository tests | **CI-only** -- no Docker daemon in the sandbox (the 229 ignored tests above) |

`SWAGGER_UI_DOWNLOAD_URL` pointed at the pre-fetched `swagger-ui-5.17.14.zip` in the session scratchpad for every `paladin-web` build (sandbox has no direct GitHub download).

## Credential-handling review (human-judgment item, security.instructions.md)

Reviewed `git diff 629ef660..HEAD` (the whole Phase 40 diff) by hand. **Clean.**

- No added `format!`, `Err(...)` or log line interpolates an API key value. The only key-adjacent added log line is `warn!("{IN_PROCESS_TOKEN_STORE_WARNING}")`, a pre-existing constant.
- `api_key_id` is built from `principal.id` (the configured key **name**), never the secret; `RunAttributionDto` carries `{ tenant_id, api_key_id }` only -- never the key value or the role (40-05 tests assert this).
- No new `{:?}` formatting of any auth or config type.
- `config.example.yml` and `k8s/server/configmap.yaml` use `${PALADIN_API_KEY_*}` placeholders; `scripts/sdk-smoke/smoke-config.yml`'s literal test key pre-dates the phase (Phase 40 added only `tenant:`).
- Pre-existing observation, out of scope for this phase: `ApiKeyConfig` derives `Debug` (already present at `629ef660`). Not a Phase 40 regression; noted for a future hygiene pass.

## Decisions Made

- `paladin-web | Principal` allowlist entry reduced to the one lint the tool actually fires (`struct_marked_non_exhaustive`); the `constructible_struct_adds_field` justification was folded into the remaining entry and the 9.2 row's Mitigation cell says so.
- No new Cargo.toml semver allow line: every fired lint is already covered crate-wide.
- WINDOWS.md row 32 left `waived` (see deviation 2); closing condition recorded as met in the root CHANGELOG and ADR-0054.
- WINDOWS.md row 59 filed instead of forcing the `RunReadScope` grep (see deviation 3); left open for the operator's waiver decision.
- Rustdoc fix applied by removing the outer `///` rather than fully qualifying the seven links: it is the smaller change, no other `pub mod` in that file carries an outer doc, and the module's inner docs already say the same thing.
- Schedule-fired runs remain unattributed (D-10); flagged for Phase 41 in ADR-0054 and the CHANGELOG.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Register contradicted by the tool] `paladin-web | Principal` allowlist entry corrected**
- **Found during:** Task 1
- **Issue:** 40-01 predicted two lints for `Principal` (`constructible_struct_adds_field` and `struct_marked_non_exhaustive`). cargo-semver-checks 0.50.0 fires only `struct_marked_non_exhaustive`: a field added to a struct that becomes `#[non_exhaustive]` in the same change does not trigger the constructible lint.
- **Fix:** Removed the `constructible_struct_adds_field` entry; folded its justification into the remaining entry; the 9.2 row's Mitigation cell records the measurement.
- **Files modified:** `.cargo/semver-checks-allowlist.toml`, `MIGRATION.md`
- **Verification:** `./scripts/check-migration-allowlist.sh` exit 0 (set-equality by crate|type)
- **Committed in:** `8311a032`

**2. [Rule 3 - Blocking gate] Seven span-less `broken_intra_doc_links` warnings in `paladin-ai-core` failed `make clean-code` (doc-check)**
- **Found during:** Task 3 gate run
- **Issue:** `crates/paladin-core/src/platform/container/mod.rs` carried an outer `///` doc on `pub mod principal;` (added in 40-01, `79201d90`) while `principal.rs` carries inner `//!` docs with intra-doc links. rustdoc merges the two attribute sets, loses the spans, and resolves the combined text in the **parent** module's scope, where `TenantId`, `PrincipalRef`, `RunAttribution`, `RunReadScope`, `RunReadScope::permits` and `TenantId::OPEN_ACCESS` are not in scope -- 7 warnings with no `-->` location. Pre-existing since 40-01 (no earlier 40-0x SUMMARY records a `make clean-code` run), but it blocks this plan's required gate.
- **Fix:** Removed the two-line outer `///` doc (no other `pub mod` in that file has one). Doc-only; no public-API item added or removed.
- **Files modified:** `crates/paladin-core/src/platform/container/mod.rs`
- **Verification:** `cargo doc -p paladin-ai-core --no-deps` warning-free; `make clean-code` exit 0 (doc-check passed, ADR-0033); `make api-surface` "API surface unchanged"
- **Committed in:** `be3a9030`

### Plan acceptance items that could not be satisfied as written

**3. WINDOWS.md row 32 cannot be marked `fixed`**
- The plan (D-21) asks for `gsd-tools windows fixed 32`. Row 32 was already `waived` on 2026-09-10 (Phase 29, accepted v0.10 deviation); the tool refuses with `WINDOWS_ALREADY_RESOLVED` (only `open -> fixed|waived` transitions exist). The plan forbids hand-editing the ledger, so row 32 stays `waived`. Its closing condition (tenant-scoped run reads) is now met and the root CHANGELOG and ADR-0054 say so. The plan's `^\| 32 \|.*\| fixed \|` acceptance grep is unsatisfiable through the sanctioned path.

**4. `grep RunReadScope .project/current-exports.txt` cannot pass by tool design**
- cargo-public-api lists a re-exported foreign-crate module as one `pub use paladin::core::platform::container::principal` line (exactly as the existing `cost` and `treasury_ledger` lines), never its items. The refreshed baseline does carry every facade-visible Phase 40 item (`execute_scoped`, `execute_stream_scoped`, `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, `AuthConfig::validate`, the `principal` re-export, `cancel`'s `Option<principal::PrincipalRef>`) and `make api-surface` exits 0. Filed as **WINDOWS.md row 59** (open, kind `deviation`, phase 40) for the operator to waive. Consequence: row 59, not the thread-route row 58, is now the ledger's last row (the plan's wording assumed row 58 would be last).

**5. WINDOWS.md row 57 waived**
- Waived (as directed by the orchestrator) with the reason that 40-03's negated acceptance grep `^\s*api_keys\s*:` over `src/application/cli` was a plan-text false positive on a Rust parameter (env.rs:22), not a code defect.

**6. 11-package parity loop not re-run after the Task 1 edits**
- The loop ran once, before the MIGRATION.md/allowlist edits. Those files are not inputs to cargo-semver-checks, so a re-run could not change the result; skipped to conserve disk (the run was already the largest consumer of the session's headroom).

### Execution note (not a plan deviation)

The first attempt at Task 3's gate run hit ENOSPC part-way through `cargo test --workspace` (root filesystem 100% full: `target/debug` grew by ~7 GB of fresh test binaries after the facade re-export invalidated every dependent target, and the semver runs created a 4.9 GB `target/semver-checks` rustdoc cache). Execution halted with the Task 3 doc edits intact and uncommitted; the orchestrator freed 13 GB (semver-checks cache, incremental, doc, tmp, stale test binaries, the partial fingerprint) and the plan resumed from the recipe in the failure report. No work was lost; no test failure was ever observed.

## Known Stubs

None. No placeholder values, TODO/FIXME markers or unwired data sources were introduced by this plan.

## Threat Flags

None. This plan added no network endpoint, auth path, file access pattern or schema change; the only source edit (`crates/paladin-core/src/platform/container/mod.rs`) is documentation-only and the facade re-export exposes types that already existed on `paladin_core`.

## Prohibitions held

- No Snyk step added and the phase is not recorded as blocked on one (CLAUDE.md, security.instructions.md).
- The 007 ledger schema, `TreasuryLedgerPort` and `RUN_SCHEMA_VERSION` were not touched.
- No new Medieval-military officer word was introduced (`TenantId` remains a plain identifier per D-00e).

## Open items handed to the orchestrator / Phase 41

- WINDOWS.md row 59: waive or keep open (operator decision).
- WINDOWS.md row 58: `/v1/threads/*` read-scope gap, owner Phase 41 planning (or a v0.11 hygiene phase).
- Schedule-fired runs unattributed (D-10) -- Phase 41.
- Coverage >= 82 % and Postgres-gated tests: confirmed only by CI.

## Self-Check: PASSED

All 14 key files exist on disk; commits 8311a032, f40dba0e, 39ab90ae, be3a9030 are in the log; the facade re-export line, the ADR Code Conformance heading and the PROMOTION next-free 0055 line are present.
