---
phase: 41-admission-time-allowance-enforcement
plan: 09
subsystem: treasurer
tags: [allowance, adr, migration-register, api-surface, changelog, phase-gate, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 08
    provides: the finished admission, notice, trace, herald and operator-webhook legs this plan records and registers
provides:
  - ADR-0056 (allowance admission model) with the 41-01 option-b outcome in its Decision section
  - PROMOTION.md next free ADR number advanced 0056 to 0057 with a dated note; PROJECT.md Key Decisions row
  - facade re-export paladin::core::platform::container::allowance
  - MIGRATION.md 9.2 consolidated new-in-0.11 completeness rows (paladin-ai-core, paladin-ports, paladin-ai)
  - Phase 41 entries in the root and paladin-core, -ports, -storage, -web, -herald CHANGELOGs
  - regenerated .project/current-exports.txt (4188 items)
affects: [phase-42, phase-46]

tech-stack:
  added: []
  patterns:
    - "One ADR carries every admission decision plus the accepted race and burst, so Phase 42 cites it rather than re-opening admission"
    - "A consolidated completeness row per crate, one published package name in each Crate cell, keeps the set-equality awk happy"

key-files:
  created:
    - .planning/decisions/0056-allowance-admission-model.md
  modified:
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md
    - src/core/platform/mod.rs
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "ADR-0056 Code Conformance is `conforms`: every cited decision is pinned by a named test, each name verified to exist"
  - "Commit trailers use the session attribution reminder's Claude Sonnet 5.5 line, as 41-01, 41-02 and 41-06 did, not the Claude Fable 5.1 line in the dispatch note"

patterns-established:
  - "The baseline .project/current-exports.txt lists a re-exported module as one `pub use` line and does not enumerate its contents"

requirements-completed: [ALLOW-01, ALLOW-02, ALLOW-04]

duration: ~40min
completed: 2026-10-04
status: complete
---

# Phase 41 Plan 09: ADR-0056, surface closeout and the phase gate Summary

**ADR-0056 records the allowance admission model (tumbling UTC windows, check-only admission, every-limit composition, no role bypass, fail-closed, store-deduped notices, the option-b checkpoint outcome, the accepted over-admission race and the accepted 2x boundary burst), the allowance module is re-exported from the facade and registered, baselined and changelogged, and every locally runnable phase gate is green.**

## Performance

- **Duration:** ~40 min (warm workspace; one workspace test run with relink)
- **Completed:** 2026-10-04
- **Tasks:** 2 (both `type="auto"`)
- **Files:** 1 created, 11 modified

## Accomplishments

- **ADR-0056 (Task 1).** Headings in the required order (`Status|Context|Decision|Considered Options|Code Locations|Code Conformance|Downstream Consumers`). The Decision section opens with the 41-01 checkpoint outcome (option-b: design approved as proposed, defaulted `balance` honouring D-00b, the `''`/epoch sentinels, claim-before-insert with abandon, C3 Option A, D-08 schedule columns, and the D-17 amendment to a twelve-key operator payload) and then one paragraph each for D-01, D-03, D-05, D-09, D-10, D-16 and D-18's observation legs. D-18 states plainly that on the HTTP agent routes the trace event is emitted only when an agent-path trace emitter is wired and the herald line appears only on the streamed final chunk (non-streaming `execute` and `jobs` produce no `ExecutionMetadata`, 41-07), best-effort by design, while the durable notice row and the operator webhook always fire. Considered Options lists every rejected alternative from the discussion log with its reason (trailing windows and GCRA, calendar and enum grammars, most-specific-wins, reusing `spend()`, reserve-at-admission, ungated agent routes and schedules, Admin bypass, 402/403, message-only body, per-enum refusal variants, fail-open, threshold ladder and run-webhook targets, in-process dedup, log-line-only). Code Conformance names the proving tests per decision; a script confirmed every cited test function exists.
- **ADR line and PROJECT.md.** `PROMOTION.md` gains the `| 0056 |` index row and `**Next free ADR number: 0057**` with a dated note (`ls .planning/decisions/0056-*.md` was re-run first and printed no match). `PROJECT.md` Key Decisions gains the ADR-0056 row beside ADR-0054/0055.
- **Surface (Task 2).** `pub use paladin_core::platform::container::allowance;` added to `src/core/platform/mod.rs` before `autonomous_config`. MIGRATION.md 9.2 gained three consolidated completeness rows, one per crate (`paladin-ai-core`: the nine allowance types/functions plus `BalanceQuery`; `paladin-ports`: `AllowanceAdmissionPort`, `AdmissionError`, `TreasuryNoticePort`; `paladin-ai`: `Treasurer`, `AllowancePolicy`, `ScopeAllowance`, `Ceiling`, `window_for`, `OperatorNoticeTarget`, `AllowanceWarningPayload`, `AllowanceConfig`, `AllowanceEntryConfig`, `AllowanceWebhookConfig`, `build_treasury_notices`), each `N/A (new types; X-10 governs only pre-existing types)`, requirements ALLOW-01, ALLOW-02, ALLOW-04. Split per crate so each Crate cell is one published package name. Five crate CHANGELOGs and the root CHANGELOG describe the phase.
- **Baseline.** `make api-surface` first reported exactly one differing line (`pub use paladin::core::platform::container::allowance`), so no earlier plan's refresh had been missed; `make api-surface-update` regenerated the file (4188 items) and `make api-surface` then exited 0.

## Task Commits

1. **Task 1: ADR-0056, PROMOTION.md to 0057, PROJECT.md row** - `b71524b` (docs)
2. **Task 2: facade re-export, consolidated register rows, CHANGELOGs, baseline** - `14b2dac` (feat)

## Verification (local phase gate)

- `cargo test --workspace` (default features): **6344 passed, 0 failed, 229 ignored**, exit 0 (the same count as 41-08, as expected: this plan adds no code).
- `cargo fmt --check` clean; `make clean-code` exit 0 (fmt, clippy, shellcheck, `cargo check`, rustdoc zero-warning bar, 104 public API `# Examples` headings); `make check-gates` exit 0 (including `check-migration-allowlist`: set-equal); `./scripts/check-migration-allowlist.sh` exit 0; `make check-changelogs` exit 0; no `TBD` in MIGRATION.md.
- `cargo test -p paladin-web --test openapi_golden_v0_9`: 8 passed.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: "API surface unchanged" after the refresh.
- `make security`: advisories ok, bans ok, licenses ok, sources ok.
- Acceptance greps: `src/core/platform/mod.rs` contains the re-export; each of the five crate CHANGELOGs' `[Unreleased]` section mentions allowance; `PROMOTION.md` has `**Next free ADR number: 0057**`, a `| 0056 |` row and a `plan 41-09, ADR-0056` note; `PROJECT.md` contains `0056-allowance-admission-model.md`; the ADR contains D-01, D-03, D-05, D-09, D-10, D-16, ADR-0052 and ADR-0053.
- Manual credential-handling review across the phase diff (`git diff 8be4972^..HEAD`): the only log or format line touching an API-key-shaped identifier is the config error naming the key's *name*; no log line, error body, trace event, webhook payload or herald line carries a key value or the webhook secret (consistent with each earlier plan's own review and the tests that assert it).
- **CI-authoritative, not runnable here:** the `coverage` job (82% workspace line floor, ADR-0006) and the `postgres-integration` job (no `SKIP:` lines). Status is pending until CI runs on the pushed branch. Locally, PostgreSQL legs were run for real in 41-02, 41-05, 41-06 and 41-08 against the throwaway cluster, and were not brought up again here because this plan changed no storage code; env-gated PostgreSQL tests skipped in the workspace run above.
- Not run here: the UAT operator walkthrough (`treasurer.allowance.api_keys.ci-runner: { period: "1h", amount: "2.50" }`, spend to `2.5000 USD`, observe `429 allowance_exhausted` and `Retry-After`, compare with `paladin-cli treasury spend --api-key ci-runner --since <window_start>`); it is for `/gsd-verify-work`. `allowance_admission_tracer` exercises the refusal and `Retry-After` end to end in the workspace run.

## Deviations from Plan

**1. [Note - acceptance] `.project/current-exports.txt` does not contain `AllowanceRefusal`.**
- The baseline lists a re-exported module as a single `pub use ...::allowance` line (as it does for `principal` and `treasury_ledger`) and does not enumerate the module's items; `AllowanceRefusal` is referenced only by `paladin-core`/`paladin-ports` signatures, which the root-crate baseline does not cover. `AllowanceAdmissionPort`, `TreasuryNoticePort` and `Treasurer` are present. `make api-surface` exits 0 and the `allowance` re-export line is in the baseline, which is the substance of the criterion. No code was changed to force the string in.

**2. [Rule 3 - Blocking] Installed `shellcheck` for `make clean-code`.**
- **Issue:** the `lint-shell` target failed with `shellcheck not found`.
- **Fix:** `apt-get install -y shellcheck` (a distribution package, CI and the devcontainer's own tool); no repository file changed. `make clean-code` then exited 0.

**3. [Note - attribution] Commit trailers.** Both commits carry `Co-Authored-By: Claude Sonnet 5.5` plus the session line, from the session's attribution reminder, not the `Claude Fable 5.1` line in the dispatch note (41-01, 41-02, 41-06 did the same; 41-03, 41-04, 41-05, 41-07 and 41-08 used the Fable line). The orchestrator may normalise before push.

**Total deviations:** 1 blocking tooling install, 2 notes; no scope change.

## Requirements

`gsd-tools requirements mark-complete ALLOW-01 ALLOW-02 ALLOW-04` marked ALLOW-01 complete (the only one still open; ALLOW-02 and ALLOW-04 were already complete from 41-04 and 41-08). Basis: ALLOW-01's whole surface (config grammar, tumbling windows on the store clock, four-ceiling composition, balance on all three adapters) is implemented and proven by 41-01 through 41-08 and the green local gate above; the CI-only gates remain to confirm. ALLOW-03 and ALLOW-05 are Phase 42's.

## Authentication Gates

None.

## Known Stubs

None. No placeholder or empty-value flow was added in this plan (documentation, a re-export, registers and a regenerated baseline).

## Threat Flags

None. T-41-44 (unregistered public surface reaching a release) is mitigated: the consolidated 9.2 rows, `check-migration-allowlist`, the regenerated baseline and `make api-surface` all pass. T-41-45 (supply chain) is accepted as planned: no dependency was added and `make security` passes.

## Notes for later plans

- Phase 42 should cite ADR-0056 for window, composition and notice semantics, reuse `window_for`, `TreasuryLedgerPort::balance` and the fixed ceiling order, and close the D-05 over-admission race with a reservation beside the admission check.
- Phase 46's Treasurer mdBook page cites ADR-0056 and the MIGRATION.md 9.2/9.4/9.5/9.6 entries.
- Open `WINDOWS.md` rows from this phase: row 62 (config loader does not expand `${VAR}` placeholders, owner Phase 46) and row 63 (pre-Phase-41 schedules fire unattributed and ungated until re-created).
- `HeraldTraceSink` is only correct while the worker builds one sink per run dispatch (41-07); unchanged here.

## Self-Check: PASSED

`.planning/decisions/0056-allowance-admission-model.md` exists; commits `b71524b` and `14b2dac` are present in `git log`; `src/core/platform/mod.rs` contains the re-export.
