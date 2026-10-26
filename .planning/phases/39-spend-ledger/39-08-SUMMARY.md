---
phase: 39-spend-ledger
plan: 08
subsystem: release-gates
tags: [rust, semver, migration, changelog, api-surface, rustdoc, release-record]

# Dependency graph
requires:
  - phase: 39-spend-ledger (plans 01-07)
    provides: "Every public-API change Phase 39 made: TreasuryLedgerPort and its three
      adapters (LEDGR-01), reserve/release race-proof admission and settlement idempotency
      (LEDGR-02, LEDGR-03), production settle writers on both run paths, RunScope.run_id,
      the ledger-derived HTTP cost surface, and the CLI treasury spend command (LEDGR-04)
      — all measured and registered by this plan"
provides:
  - "Empirical cargo-semver-checks 0.50.0 measurement of every Phase 39 public-API change
    against both the CI-pinned v0.9.0 baseline (all 11 CI packages) and the published
    v0.10.1 baseline (--release-type minor, the 6 changed packages)"
  - "MIGRATION.md §9.2: paladin-web | ExecuteResponse extended with a Phase 39 note (cost
    field, already-suppressed lint, no new allowlist entry); a new paladin-web | RunResponse
    row (N/A for CI set-equality, new-in-0.10 type) for the v0.10 -> v0.11 migration guide"
  - "CHANGELOG.md [Unreleased] Added/Changed entries covering the whole phase; five
    per-crate CHANGELOG.md [Unreleased] bullets"
  - "Facade re-export: core::platform::container::treasury_ledger"
  - "Refreshed .project/current-exports.txt (4035 items) via the CI-pinned
    nightly-2026-09-20 toolchain; make api-surface exits 0"
  - "Zero rustdoc warnings workspace-wide (3 Phase-39-introduced broken intra-doc links
    fixed): treasury_ledger.rs, worker.rs, facade_provisioner.rs, run_api_wiring.rs"
  - "A clean, documented run of every project release gate on the phase's final tree:
    cargo test --workspace, cargo fmt --check, make clean-code, make security,
    make check-gates, make openapi (no drift); LEDGR-04's herald/trace clause confirmed
    unchanged from Phase 38"
affects: ["40-tenant-identity", "41-allowance-ceilings", "42-mid-run-halt", "46-mdbook-treasurer-docs (CURR-23)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Empirical semver-checks derivation (D-27, Phase 38 plan 38-09 precedent): every
      MIGRATION.md registration in this plan is copy-pasted from an actual cargo
      semver-checks check-release run's output, never predicted -- including a diagnostic
      run with paladin-web's crate-wide constructible_struct_adds_field suppression
      temporarily disabled and immediately reverted, to distinguish 'already covered by an
      existing blanket allow' from 'genuinely unsuppressed'"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - .project/current-exports.txt
    - src/core/platform/mod.rs
    - crates/paladin-core/src/platform/container/treasury_ledger.rs
    - src/application/services/run/worker.rs
    - src/infrastructure/web/facade_provisioner.rs
    - src/infrastructure/web/run_api_wiring.rs

key-decisions:
  - "No .cargo/semver-checks-allowlist.toml entry and no Cargo.toml lint-table line was
    added anywhere in this phase. Every lint the 0.10.1 diagnostic run reported
    (paladin-web's RunResponse.cost, ExecuteResponse.cost) was empirically confirmed
    already covered by that crate's existing crate-wide constructible_struct_adds_field
    suppression -- and the set-equality gate dedups by crate|type, not by lint, so
    ExecuteResponse's existing Phase 31 allowlist entry already covers the new field too."
  - "RunResponse gained its first MIGRATION.md §9.2 row, marked N/A for CI set-equality --
    it is a new-in-0.10 type (shipped Phase 27, PLAT), absent at the v0.9.0 baseline X-10
    governs, so its cost field addition is recorded for the v0.10 -> v0.11 migration guide
    (Phase 46, CURR-23) rather than as a CI-gated deliberate-breaking row, mirroring the
    ThreadApiState/ResumeAcceptedResponse precedent already in the document."
  - "The CI-parity check (--baseline-version 0.9.0, no --release-type override) reports
    '0 checks: 0 pass, 254 skip' for ALL 11 packages, unconditionally -- re-confirming, not
    newly discovering, the standing Phase-38/STATE.md finding that this check is a no-op
    for every package in every plan of this milestone until the v0.11.0 version bump. Not
    fixed here (ci.yml is not in this plan's files_modified)."
  - "TraceEvent::NodeFinished/RunFinished.cost still fires enum_struct_variant_field_added
    against the published v0.10.1 baseline -- unchanged from Phase 38's own registration of
    this exact finding (this phase's diff touches neither trace.rs nor herald.rs); no new
    action taken, the existing N/A row already covers it."
  - "Installed no new tools -- cargo-semver-checks 0.50.0, cargo-audit 0.22.2 and
    cargo-deny 0.20.2 were already present from Phase 38's install; reused the same
    file:// SWAGGER_UI_DOWNLOAD_URL workaround Phase 38 documented for paladin-web's
    rustdoc build (this sandbox's egress proxy still blocks the swagger-ui GitHub zip)."

requirements-completed: [LEDGR-01, LEDGR-02, LEDGR-03, LEDGR-04]

coverage:
  - id: D1
    description: "Every one of the 11 CI packages passes cargo semver-checks against the
      CI-pinned v0.9.0 baseline (--default-features); the loop and the migration-allowlist/
      check-gates set-equality gate both exit 0 on the phase's final tree"
    requirement: LEDGR-01
    verification:
      - kind: other
        ref: "cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0 (all 11 CI packages, each exit 0)"
        status: pass
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh"
        status: pass
      - kind: other
        ref: "make check-gates"
        status: pass
    human_judgment: false
  - id: D2
    description: "The published v0.10.1 baseline (--release-type minor) measurement for the
      6 packages Phase 39 changed captures every genuine, unsuppressed and already-suppressed
      lint, all registered or explicitly confirmed as needing no new registration in
      MIGRATION.md §9.2"
    requirement: LEDGR-01
    verification:
      - kind: other
        ref: "cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.10.1 --release-type minor (paladin-ai, paladin-ai-core, paladin-ports, paladin-storage, paladin-battalion, paladin-web)"
        status: pass
    human_judgment: false
  - id: D3
    description: "MIGRATION.md §9.2's ExecuteResponse row carries a dated Phase 39 extension;
      a new RunResponse row is added marked N/A; the allowlist stays set-equal (16
      crate|type pairs, unchanged) since no new deliberate-breaking entry was needed"
    requirement: LEDGR-01
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh (16 pairs, set-equal)"
        status: pass
    human_judgment: false
  - id: D4
    description: "CHANGELOG.md gains one [Unreleased] section (Added + new Changed) naming
      every Phase 39 public-facing change; each of the five changed crates' own
      CHANGELOG.md [Unreleased] names its own treasury/cost addition"
    requirement: LEDGR-04
    verification:
      - kind: other
        ref: "grep -c '^## \\[Unreleased\\]' CHANGELOG.md == 1; per-crate greps for treasury|cost under each Unreleased section, all non-zero"
        status: pass
    human_judgment: false
  - id: D5
    description: "The facade re-exports core::platform::container::treasury_ledger; the
      public-surface baseline is refreshed with the CI-pinned nightly-2026-09-20 toolchain;
      TreasuryLedgerPort is present; make api-surface exits 0 against the refreshed baseline"
    requirement: LEDGR-04
    verification:
      - kind: other
        ref: "PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update && make api-surface"
        status: pass
    human_judgment: false
  - id: D6
    description: "The phase's final tree passes every commit gate CLAUDE.md names: cargo
      test --workspace (0 failures across every test result line), cargo fmt --check, make
      clean-code (clippy -D warnings, rustdoc zero-warning bar, public-API examples gate),
      make security (9 pre-existing allowed advisory warnings, 0 new), make check-gates;
      make openapi produces no diff on crates/paladin-web/openapi.json"
    requirement: LEDGR-01
    verification:
      - kind: other
        ref: "cargo test --workspace"
        status: pass
      - kind: other
        ref: "cargo fmt --check"
        status: pass
      - kind: other
        ref: "make clean-code"
        status: pass
      - kind: other
        ref: "make security"
        status: pass
      - kind: other
        ref: "make check-gates"
        status: pass
      - kind: other
        ref: "make openapi && git diff --exit-code crates/paladin-web/openapi.json"
        status: pass
    human_judgment: false
  - id: D7
    description: "LEDGR-04's herald/trace clause is confirmed unchanged from Phase 38: no
      new TraceEvent variant or herald field exists in this phase's diff, and the cost-sum
      and herald cost tests still pass"
    requirement: LEDGR-04
    verification:
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib run_finished_cost_sums_priced_paladin_nodes"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-herald --lib (53 passed, 0 failed)"
        status: pass
      - kind: other
        ref: "git diff <phase-start>..HEAD -- crates/paladin-core/src/platform/container/{trace,herald}.rs (empty)"
        status: pass
    human_judgment: false
  - id: D8
    description: "Manual credential-handling review over the full Phase 39 diff: settle log
      lines never interpolate api_key_id/tenant_id (an opaque LedgerScope label, the
      unattributed sentinel this phase), both SQL adapters redact the database URL's
      password before any other handling of a connection error, and no new HTTP client was
      added anywhere in the phase"
    requirement: LEDGR-01
    verification: []
    human_judgment: true
    rationale: "This is a manual source-inspection review per security.instructions.md, not
      something a unit test asserts -- grep-scanned the whole phase diff (git diff
      15be9f79..HEAD -- crates src) for credential-shaped identifiers and reqwest client
      construction, then read the engine and agent-loop settle log call sites directly to
      confirm only run_id/superstep/attempt/ordinal/nanos/currency are interpolated, never
      api_key_id/tenant_id. Findings are stated in prose below; a human reviewer re-reading
      the same diff would reach the same conclusions (clean; both SQL adapters redact
      first, log lines carry no scope identity, no new HTTP client anywhere in the phase)."

duration: ~55min
completed: 2026-09-28
status: complete
---

# Phase 39 Plan 08: Spend Ledger — Phase Closeout on the Project's Release Gates Summary

**Every Phase 39 public-API change measured empirically with cargo-semver-checks against both the CI-pinned v0.9.0 baseline and the published v0.10.1 baseline, registered in MIGRATION.md §9.2, described in the CHANGELOG, the facade re-exporting the new `treasury_ledger` module, and the phase's final tree run through every commit and security gate the project requires before a phase is sealed.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-09-28T01:14:00Z (approx, per STATE.md's prior session timestamp)
- **Completed:** 2026-09-28T02:08:55Z
- **Tasks:** 2 (Task 1: semver measurement + MIGRATION.md registration; Task 2: facade re-export, CHANGELOG/API-baseline, full gate suite)
- **Files modified:** 13 (1 in Task 1's commit, 12 across Task 2's — 4 of those were rustdoc-link fixes discovered while running `make clean-code`, not originally in the plan's declared file list)

## Accomplishments

- **Task 1 (`ef4431ab`):** Ran the exact CI-parity loop (`cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0`) across all 11 published CI packages: every one reports `0 checks: 0 pass, 254 skip — Summary no semver update required`, re-confirming (not newly discovering) the standing Phase-38/STATE.md finding that this check is an unconditional no-op for every package until the v0.11.0 version bump — `paladin-web` needed the same `file://` `SWAGGER_UI_DOWNLOAD_URL` local-zip workaround Phase 38 documented (this sandbox's egress proxy still blocks the swagger-ui GitHub download). Then ran the published `v0.10.1` baseline with `--release-type minor` for the six packages Phase 39 changed: `paladin-ai-core` reports one failure — the same `TraceEvent::NodeFinished`/`RunFinished.cost` `enum_struct_variant_field_added` finding Phase 38 already registered (this phase's diff touches neither `trace.rs` nor `herald.rs`) — every other package (`paladin-ai`, `paladin-ports`, `paladin-storage`, `paladin-battalion`) is clean. `paladin-web` reported zero fails on the plain run because its crate-wide `constructible_struct_adds_field = "allow"` line (added Phase 31) already suppresses everything; a diagnostic run with that one line temporarily disabled confirmed both `RunResponse.cost` (39-06) and `ExecuteResponse.cost` (39-06) genuinely fire `constructible_struct_adds_field`, then the line was restored unchanged. Extended `ExecuteResponse`'s existing §9.2 row (Phase 31) with a dated Phase 39 note (no new allowlist entry — same lint id, same pair, already covered); added a brand-new `RunResponse` row marked N/A for CI set-equality (a new-in-0.10 type, absent at the v0.9.0 baseline X-10 governs), recorded for the v0.10 → v0.11 migration guide (Phase 46, CURR-23). `./scripts/check-migration-allowlist.sh` and `make check-gates` both confirm the allowlist stays set-equal (16 `crate | type` pairs, unchanged).
- **Task 2 (`980caf94`):** Added `pub use paladin_core::platform::container::treasury_ledger;` to the facade's flat re-export list (alphabetically, after `token_usage`, before `trigger`). Added a `## [Unreleased]` `### Added` block to `CHANGELOG.md` covering the whole phase (`TreasuryLedgerPort`, the three adapters and contract suite, the `007` migration, race-proof reserve/idempotent settle, `paladin-cli treasury spend`, the engine and agent-loop production settle writers, `RunScope::with_run_id`, and the ledger composition points), plus a new `### Changed` section (the ledger-derived HTTP cost, the inverted Phase 38 deferral, the `unattributed` scope sentinel). Added matching `[Unreleased]` bullets to the five changed crates' own `CHANGELOG.md` files. Regenerated `.project/current-exports.txt` with `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20` (4035 items, purely additive); `make api-surface` exits 0. Ran the full gate suite on the final tree: `cargo fmt --check`, `cargo test --workspace` (every `test result:` line across 57 suites reports `0 failed`), `make clean-code`, `make security`, `make check-gates`, and `make openapi` (`git diff --exit-code crates/paladin-web/openapi.json` — no drift, the baseline was already regenerated and committed at 39-06). `make clean-code`'s `doc-check` step surfaced three rustdoc intra-doc-link warnings across four files this phase's own plans introduced without re-running `make doc-check`; fixed all of them (see Deviations) so the ADR-0033 zero-warning bar holds. Confirmed LEDGR-04's herald/trace clause unchanged from Phase 38 (`run_finished_cost_sums_priced_paladin_nodes` and the full `paladin-herald` suite pass; `trace.rs`/`herald.rs` carry no diff across the whole phase). Performed the manual credential-handling review over the full phase diff.

## Task Commits

1. **Task 1: Measure every Phase 39 API change with cargo-semver-checks and register what it reports** - `ef4431ab` (docs)
2. **Task 2: Facade re-export, CHANGELOG entries, refreshed API baseline, and the full gate suite** - `980caf94` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `MIGRATION.md` — §9.2: `ExecuteResponse` row extended with a Phase 39 note; new `RunResponse` row (N/A)
- `CHANGELOG.md` — `[Unreleased]` `### Added` block for the whole phase; new `### Changed` bullets
- `crates/{paladin-core,paladin-ports,paladin-storage,paladin-battalion,paladin-web}/CHANGELOG.md` — `[Unreleased]` bullets for each crate's own Phase 39 surface
- `.project/current-exports.txt` — refreshed public API baseline (4035 items, CI-pinned `nightly-2026-09-20`)
- `src/core/platform/mod.rs` — `pub use paladin_core::platform::container::treasury_ledger;`
- `crates/paladin-core/src/platform/container/treasury_ledger.rs` — module-doc `ReservationId`/`SettlementKey` links switched to fully-qualified paths (rustdoc fix)
- `src/application/services/run/worker.rs` — `with_treasury_ledger`'s link to the private `Self::run_agent` switched to a plain code span (rustdoc fix)
- `src/infrastructure/web/facade_provisioner.rs` — link to the private `EngineExecutionPort::execute_scoped` switched to a plain code span (rustdoc fix)
- `src/infrastructure/web/run_api_wiring.rs` — link to the private `build_postgres_quartet` switched to a plain code span (rustdoc fix)

## Decisions Made

See `key-decisions` in the frontmatter for the full empirical reasoning. In short: no new `.cargo/semver-checks-allowlist.toml` or `Cargo.toml` lint-table entry was needed anywhere (every genuinely-fired lint was already covered by an existing crate-wide suppression, confirmed by a temporary-disable-and-revert diagnostic on `paladin-web`); `RunResponse` got its first §9.2 row, marked N/A because it postdates the `v0.9.0` baseline entirely, matching the `ThreadApiState`/`ResumeAcceptedResponse` precedent already in the document.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Three rustdoc intra-doc-link warnings blocking `make doc-check`/`make clean-code`'s ADR-0033 zero-warning bar**
- **Found during:** Task 2, running `make clean-code` on the final tree
- **Issue:** `cargo doc --workspace --no-deps` reported 5 warnings across 4 files, all introduced by earlier plans in this phase (39-01 through 39-07) that never re-ran `make doc-check` after landing: `treasury_ledger.rs`'s module-level `//!` doc comment linking bare `[`ReservationId`]`/`[`SettlementKey`]` with no `use` in scope (unresolved — a module doc's bare-name link resolution does not reach sibling items the way an item's own `///` doc comment's does, the same root cause 38-09 already fixed once in `cost.rs`); `worker.rs`'s `with_treasury_ledger` doc linking the PRIVATE `Self::run_agent`; `facade_provisioner.rs`'s doc linking the PRIVATE `EngineExecutionPort::execute_scoped`; `run_api_wiring.rs`'s `build_treasury_ledger` doc linking the PRIVATE `build_postgres_quartet` — links inside a PUBLIC item's doc comment cannot resolve to a private item under default `cargo doc` (no `--document-private-items`), the exact pattern 38-09's `worker.rs` fix already established a precedent for.
- **Fix:** `treasury_ledger.rs`: switched the bare-name links to the already-proven-working fully-qualified-path form (`` [`crate::platform::container::treasury_ledger::ReservationId`] ``), matching 38-09's `cost.rs`/`herald.rs` precedent. `worker.rs`, `facade_provisioner.rs`, `run_api_wiring.rs`: switched the three private-item links to plain code spans (`` `Self::run_agent` ``, `` the internal `EngineExecutionPort::execute_scoped` ``, `` `build_postgres_quartet` ``), matching 38-09's own established precedent for exactly this situation.
- **Files modified:** `crates/paladin-core/src/platform/container/treasury_ledger.rs`, `src/application/services/run/worker.rs`, `src/infrastructure/web/facade_provisioner.rs`, `src/infrastructure/web/run_api_wiring.rs`
- **Verification:** `cargo doc --workspace --no-deps 2>&1 | grep -i warning` produces no output; the full `make clean-code` run (fmt, lint, lint-shell, check, doc-check, check-api-examples) exits 0.
- **Committed in:** `980caf94` (Task 2 commit)

---

**Total deviations:** 1 (a rustdoc bug fix spanning 4 files). **Impact on plan:** Comment-only, zero behavior change, required for this plan's own explicitly-stated gate (`make clean-code`/ADR-0033) to pass — no scope creep.

## Measured discrepancy between the plan's `must_haves` and the tool's actual output

Recorded here in full per the project's "derived from the tool's actual output, never guessed" standard, rather than silently forcing a match — mirroring 38-09's identical `cost::Cost` finding:

**`.project/current-exports.txt` does not contain the literal substrings `SqliteTreasuryLedger` or `treasury_ledger::LedgerScope`, by the extraction tool's own design, not by omission.** `cargo-public-api` only enumerates the *facade* crate's (`paladin`/`paladin-ai`) own item signatures; it never traverses into a `pub use`-re-exported module living in a *different* crate (`paladin-core`, `paladin-storage`) to list that module's descendant items individually — confirmed as the identical, pre-existing, systemic pattern 38-09 already documented for `cost::Cost` (also absent) and `container::garrison`/`TokenUsage` (neither has a standalone struct line either). `TreasuryLedgerPort` *does* appear (3 occurrences), because it reaches the baseline indirectly through two facade-level functions that take it as a parameter (`PaladinExecutionService::with_treasury_ledger`, `RunWorkerPool::with_treasury_ledger`). `SqliteTreasuryLedger` has no such facade-crate-local function returning or taking it directly — `build_treasury_ledger` returns `Arc<dyn TreasuryLedgerPort>`, never the concrete adapter type — and `LedgerScope` likewise has no facade-level function taking or returning it bare. Adding a facade-level function purely to satisfy these two substring greps would be an unrequested, unjustified public-API surface addition (Rule 4 territory) with no `CONTEXT.md` or plan basis; not done, per the same reasoning 38-09 already applied to `cost::Cost`. The one substring the plan's acceptance criterion also names, `TreasuryLedgerPort`, does pass.

## Issues Encountered

- The sandbox's root filesystem hovered near the ~4 GB free floor throughout this plan (mirroring every prior 39-* plan's SUMMARY). Cleared `target/debug/incremental` and `target/semver-checks` repeatedly between semver-checks package runs and before `make api-surface-update`/`cargo test --workspace`/`make clean-code` (never `target/debug/deps`, never `cargo clean`); no build ever failed on `ENOSPC` this plan.
- `cargo-audit`'s yanked-crate lookup reported the same pre-existing `spin`/`lazy_static` transitive-dependency yanked/unsound warnings 38-09 already logged (9 allowed warnings total, all covered by `.cargo/audit.toml`/`deny.toml`'s existing ignore entries; `git diff --stat Cargo.lock` produces no output — this phase added zero dependencies).

## User Setup Required

None - no external service configuration required.

## Credential-handling review

Grep-scanned the full phase diff (`git diff 15be9f79..HEAD -- crates src`) for credential-shaped identifiers (`api_key`, `password`, `secret`, `url_env`) and `reqwest` client construction, then read every settle-writer log call site directly:

- `crates/paladin-battalion/src/engine/settlement.rs`'s `SpendHook::settle_boundary` and `src/application/services/paladin/paladin_execution_service.rs`'s `settle_agent_loop_call` both log only `run_id`, `superstep`/`ordinal`, `attempt`, `amount.nanos()` and `amount.currency()` on `Settled`/`AlreadySettled`/`Err` — never `api_key_id` or `tenant_id`. Every ledger row's `api_key_id`/`tenant_id` this phase's production writers stamp is the documented `LedgerScope::unattributed()` sentinel (D-01) — an opaque scope label, never a real secret — and no code path logs it.
- Both new SQL adapters (`crates/paladin-storage/src/treasury/{sqlite,postgres}.rs`) route every connection error through `redact_database_url_password(&err.to_string(), database_url)` before any other handling — redaction happens first, with no intervening truncation, per `security.instructions.md`'s "redact before truncation" rule.
- No new HTTP client was added anywhere in this phase (`grep -n "reqwest::Client\|Client::builder"` over the full phase diff produces no output) — the ledger is entirely embedded-database traffic (SQLite/Postgres/in-memory) and CLI reads, no new network egress.

## Coverage

The 82% workspace line-coverage floor (ADR-0006) is read from CI's `coverage` job, not measured locally — this devcontainer has no Docker, the same standing concern `STATE.md` has carried since the v0.10.0 close.

## Postgres suite location

The full 21-clause `TreasuryLedgerPort` contract suite ran live against Postgres at plan 39-03 (23 `treasury::postgres::tests`, 0 `SKIP:` lines), on a local Postgres 16 cluster (`pg_ctlcluster`) with a scratch `paladin`/`paladin_treasury_test` role and database — this sandbox has no Docker daemon. CI's `postgres-integration` job (`--lib postgres` filter) remains the authority for the Docker-gated path this repository ships to contributors without a locally reachable Postgres; this plan did not re-run that suite (no code in `crates/paladin-storage/src/treasury/` changed since 39-03).

## Next Phase Readiness

- **LEDGR-01, LEDGR-02, LEDGR-03 and LEDGR-04 are delivered on a tree that passes every project gate**, per this plan's own success criterion: `cargo test --workspace`, `cargo fmt --check`, `make clean-code`, `make security`, `make check-gates`, `./scripts/check-migration-allowlist.sh`, and `make openapi` (no drift) all exit 0 on the final commit.
- **The herald/trace half of LEDGR-04 is confirmed unchanged from Phase 38** — no new `TraceEvent` variant or herald field exists in this phase's diff.
- **One measured, documented finding carried forward, not fixed here**: the CI `semver` job's `v0.9.0` baseline remains a no-op for every package until the version is bumped for `v0.11.0` (unchanged from Phase 38's own finding, re-confirmed here). Not a Phase 39 defect — `ci.yml` is not in this plan's `files_modified`.
- **One new measured discrepancy, documented above, not fixed here**: `SqliteTreasuryLedger`/`treasury_ledger::LedgerScope` cannot appear as literal substrings in `.project/current-exports.txt` given how `cargo-public-api` scopes its extraction (mirrors 38-09's identical `cost::Cost` finding). Does not block this phase's own success criterion.
- Phase 39 (spend-ledger) is now fully closed: the ledger's domain types and port (39-01), race-proof reserve/idempotent settle across all three adapters (39-02, 39-03), the engine-path (39-04) and agent-loop (39-05) production settle writers, the ledger-derived HTTP cost surface (39-06), production wiring end-to-end from `POST /v1/runs` to `GET /v1/runs/{id}`'s `cost.display` (39-07), and this plan's release-record closeout (39-08).
- No blockers to starting Phase 40 (tenant identity), which replaces the `unattributed` scope's source (the run's recorded principal), never the schema, the port or the queries (D-01).

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-28*

## Self-Check: PASSED

All modified files and commit hashes verified present on disk / in `git log --oneline --all`:
- `MIGRATION.md` — FOUND (contains `Extended, Phase 39 (LEDGR-04)` for `ExecuteResponse`, and a new `RunResponse` row)
- `CHANGELOG.md` — FOUND (exactly one `## [Unreleased]` line, containing `TreasuryLedgerPort`, `treasury spend`, `007`, `unattributed`, `cost`)
- `.project/current-exports.txt` — FOUND (4035 items; contains `TreasuryLedgerPort`; does not contain the literal substrings `SqliteTreasuryLedger`/`treasury_ledger::LedgerScope`, documented above)
- `src/core/platform/mod.rs` — FOUND (contains `pub use paladin_core::platform::container::treasury_ledger;`)
- `ef4431ab` (Task 1) — FOUND
- `980caf94` (Task 2) — FOUND

Re-ran plan-level `<verification>` on the final tree: the 11-package `cargo semver-checks` loop against `v0.9.0` exits 0 for every package; `./scripts/check-migration-allowlist.sh`, `make check-gates`, `make api-surface`, `make clean-code`, `make security`, `cargo test --workspace`, `cargo fmt --check` all exit 0; `make openapi` leaves `crates/paladin-web/openapi.json` unchanged. All task-level `<acceptance_criteria>` re-verified against the final source, with the one documented exception above.
