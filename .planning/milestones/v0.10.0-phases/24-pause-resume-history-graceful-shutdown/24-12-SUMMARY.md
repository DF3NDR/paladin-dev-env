---
phase: 24-pause-resume-history-graceful-shutdown
plan: 12
subsystem: docs
tags: [mdbook, migration-guide, changelog, traceability, semver-checks, gate-evidence]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "Plans 24-01..24-11's complete HITL-01..05 implementation: Parley suspend/resume, Gate node, resume_with validation matrix, Chronicle replay/fork, graceful shutdown, and the thread HTTP surface"
provides:
  - "docs/src/user-guides/parley-and-chronicle.md — the phase's mdBook user guide, wired into SUMMARY.md after control-flow.md (D-30)"
  - "MIGRATION.md §9.2 — the Waypoint row resolved (AwaitingInput reshape + fork_of), ParleyPort recorded as a new trait, a Phase 24 deliberate-zero note, and a genuine deliberate-breaking row for require_authentication (found by this plan's own gate-evidence run, not by plan 24-11)"
  - "MIGRATION.md §9.6 — the three thread endpoint paths and status sets filled; the golden openapi.json diff left as SHIP-02's Phase 29 scope"
  - "CHANGELOG.md [Unreleased] — HITL-01..05's user-visible changes and the v3->v4 fingerprint bump"
  - ".project/v0.10.0/08-traceability-matrix.md — concrete test anchors on rows G-05, G-06, G-09, G-15, G-26"
  - ".cargo/semver-checks-allowlist.toml + crates/paladin-web/Cargo.toml — the require_authentication deliberate-breaking suppression, set-equal to the new MIGRATION.md row"
  - "Recorded gate evidence: cargo test --workspace, cargo fmt --check, cargo clippy -D warnings, make security, cargo doc, MSRV 1.88, cargo semver-checks (all 11 published packages vs 0.9.0), a local reduced-feature coverage measurement, and a manual credential-handling review"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A per-crate cargo-semver-checks suppression ([package.metadata.cargo-semver-checks.lints]) is added ONLY in the same commit as its matching MIGRATION.md §9.2 deliberate-breaking row and .cargo/semver-checks-allowlist.toml entry — CI's semver job enforces set-equality between the two registers, so a suppression with no MIGRATION.md row (or vice versa) fails the gate"

key-files:
  created:
    - docs/src/user-guides/parley-and-chronicle.md
  modified:
    - docs/src/SUMMARY.md
    - MIGRATION.md
    - CHANGELOG.md
    - .project/v0.10.0/08-traceability-matrix.md
    - .cargo/semver-checks-allowlist.toml
    - crates/paladin-web/Cargo.toml
    - .planning/WINDOWS.md

key-decisions:
  - "A genuine, previously-unregistered public API break (paladin-web's require_authentication gaining a generic type parameter, landed in plan 24-11) was found by this plan's own gate-evidence run of cargo semver-checks against the v0.9.0 baseline — not by plan 24-11 itself, whose own SUMMARY documented the change as a Rule 3 deviation but never registered it in MIGRATION.md §9.2. Fixed in place (Rule 1 — a correctness/compliance defect) rather than left as a finding: a MIGRATION.md row (Deliberate-breaking = Y), a matching .cargo/semver-checks-allowlist.toml entry, and the actual per-crate tool suppression in crates/paladin-web/Cargo.toml, verified by re-running cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0 clean afterward."
  - "Workspace line coverage was measured locally with cargo llvm-cov --workspace --features web-server (87.11%, above the 82% ADR-0006 floor) rather than the canonical make coverage / scripts/coverage.sh invocation (--features integration-tests,llm-all), because that script hard-fails without a reachable Redis and MinIO, and no Docker daemon exists in this devcontainer — the exact Phase 17 precedent (WINDOWS.md row 13, later closed by a CI run). Recorded honestly as a reduced-feature local measurement, not CI's official figure; a WINDOWS.md row files the gap."
  - "cargo semver-checks was run per-published-package with --default-features --baseline-version 0.9.0 (matching CI's exact ci.yml invocation) rather than trusting a single --workspace --baseline-rev v0.9.0 run, because the latter's own feature-union heuristic trips a pre-existing, unrelated paladin-memory/qdrant-client compile break (documented in ci.yml's own comment and Phase 22's deferred-items) that has nothing to do with this phase — running the CI-equivalent command per package is what actually proves the gate, not an easier command that reports a false negative."
  - "A second WINDOWS.md row (unrun-verify) was filed for Phase 24's own new Postgres Tier-2 contract-suite cases (D-02/D-14/D-15, landed in plan 24-06), which self-skip locally for the identical no-Docker reason already carried for Phase 22's row 22 — this plan's own gate-evidence pass is what surfaces the gap for the phase as a whole, even though the cases themselves were authored two plans earlier."

patterns-established: []

requirements-completed: [HITL-01, HITL-02, HITL-03, HITL-04, HITL-05]

coverage:
  - id: D1
    description: "docs/src/user-guides/parley-and-chronicle.md teaches the approval gate, envelope-raised parleys, resume_with, partial answers, expiry, Chronicle history/replay/fork, graceful shutdown, the HTTP surface, and the paladin-notifications composition example; wired into SUMMARY.md after control-flow.md"
    requirement: "HITL-01"
    verification:
      - kind: other
        ref: "grep -c 'parley-and-chronicle' docs/src/SUMMARY.md (1 match)"
        status: pass
      - kind: other
        ref: "cargo test --workspace --doc (0 failed)"
        status: pass
      - kind: other
        ref: "mdbook build docs (0 broken links, after mdbook-mermaid install docs)"
        status: pass
    human_judgment: false
  - id: D2
    description: "MIGRATION.md §9.2/§9.6 filled per D-29: Waypoint row resolved, ParleyPort recorded new, Phase 24 deliberate-zero note added, three thread endpoints listed in §9.6; sections 9.1/9.5/9.8 left untouched (plan 24-09's scope)"
    requirement: "HITL-01"
    verification:
      - kind: other
        ref: "grep -c 'ParleyPort' MIGRATION.md (2 matches) && grep -c '/v1/threads/' MIGRATION.md (3 matches)"
        status: pass
      - kind: other
        ref: "git diff -U0 MIGRATION.md hunks confined to §9.2 (~lines 136-146) and §9.6 (~line 191+) plus the require_authentication row — no touch to §9.1/§9.5/§9.8"
        status: pass
    human_judgment: false
  - id: D3
    description: "CHANGELOG.md [Unreleased] records HITL-01..05's user-visible changes and the v3->v4 fingerprint bump"
    requirement: "HITL-01"
    verification:
      - kind: other
        ref: "grep -c 'Unreleased' CHANGELOG.md (2 matches)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Traceability-matrix rows G-05, G-06, G-09, G-15, G-26 each name a concrete, tree-verified test function and file"
    requirement: "HITL-01"
    verification:
      - kind: other
        ref: "grep -rl 'fn <name>' for all 18 named test functions across .project/v0.10.0/08-traceability-matrix.md's five new anchor lists — every one found in the tree"
        status: pass
    human_judgment: false
  - id: D5
    description: "Phase gate evidence recorded honestly: cargo test --workspace, cargo fmt --check, cargo clippy -D warnings, make security, cargo doc, MSRV 1.88, cargo semver-checks (all 11 published packages vs 0.9.0), and a manual credential-handling review, with the Postgres Tier-2 cases and the reduced-feature coverage measurement both named as local gaps in WINDOWS.md"
    requirement: "HITL-01"
    verification:
      - kind: other
        ref: "cargo test --workspace --no-fail-fast --features web-server (0 failed across every test binary)"
        status: pass
      - kind: other
        ref: "cargo fmt --check"
        status: pass
      - kind: other
        ref: "cargo clippy --workspace --features web-server -- -D warnings"
        status: pass
      - kind: other
        ref: "make security (advisories ok, bans ok, licenses ok, sources ok)"
        status: pass
      - kind: other
        ref: "cargo doc --no-deps --workspace --features web-server (no new warnings; this plan touched no .rs files)"
        status: pass
      - kind: other
        ref: "cargo +1.88 check --workspace --all-features --all-targets --locked (0 errors)"
        status: pass
      - kind: other
        ref: "cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0 for all 11 published packages (paladin-ai, paladin-ai-core, paladin-ports, paladin-battalion, paladin-herald, paladin-llm, paladin-memory, paladin-storage, paladin-notifications, paladin-content, paladin-web) — all pass clean"
        status: pass
    human_judgment: false

duration: ~120min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 12: Docs, MIGRATION/CHANGELOG/Traceability Registration and Gate Evidence Summary

**A new mdBook user guide teaches the whole HITL feature, `MIGRATION.md`/`CHANGELOG.md`/the traceability matrix are filled for Phase 24's real footprint, and the phase's own gate-evidence run caught and fixed a genuine unregistered public-API break (`require_authentication`) that plan 24-11 shipped without a `MIGRATION.md` row.**

## Performance

- **Duration:** ~120 min
- **Tasks:** 3 (all `type="auto"`, no checkpoints)
- **Files modified:** 6 modified, 1 created (plus `.planning/WINDOWS.md`, orchestrator-excluded from this count)

## Accomplishments

- `docs/src/user-guides/parley-and-chronicle.md` (new) teaches, in the order D-30 specifies: building an approval gate as one `NodeSpec::Gate` plus two conditional edges (with the `Contains` needle-anchoring caveat flagged in 24-02's own Issues Encountered); raising a parley from a Paladin node through the structured directive envelope and reading the answer back through the `parley.` namespace; `resume_with`'s complete typed-error table; partial answers as a persisted, cold-store-queryable Waypoint chain; both `on_expire` policies; `ChronicleService`'s `history`/`inspect`/`latest_on_branch`, `replay`/`fork`-with-edit, the byte-identical mainline invariant, and the branch-scoped subgraph child rule; graceful shutdown from the embedder's side (`with_shutdown_grace`, `ShutdownCoordinator`, the `Skipped`/re-listed-vanguard contract); and the HTTP surface's `202`-then-poll shape. It carries the required `paladin-notifications` composition example (PRD 03 §6 — a `NotificationDeliveryPort::deliver_notification` call built from `Notification::new`/`NotificationContent::new`/`NotificationRecipient::Email`, the real, verified API) and both accepted-risk warnings the threat model requires: never template a secret into a Gate payload (T-24-45), and the interim any-authenticated-role posture naming `PLAT-06` as successor (T-24-46). Wired into `docs/src/SUMMARY.md` immediately after the control-flow page entry.
- `MIGRATION.md` §9.2: the `Waypoint` row now names the shipped `AwaitingInput { parleys, responses }` reshape and the additive `fork_of` field (both attributed to their landing plans); `ParleyPort` is recorded as a new trait; a Phase 24 deliberate-zero note lists `RunOutcome`, `EngineError`, `NodeSpec`, `NodeContext`, `NodeOutcomeKind`, `WaypointSummary` and `ThreadApiState` as absent-at-v0.9.0 (mirroring the Phase 23 note's own form), with `TraceEvent` recorded as untouched. §9.6 lists the three thread paths (`GET .../state`, `POST .../resume`, `GET .../history`) with their status sets, taken from the regenerated `openapi.json`; the golden diff proof stays SHIP-02's Phase 29 scope. Sections 9.1, 9.5 and 9.8 (plan 24-09's own scope) are untouched, confirmed by inspecting `git diff -U0`'s hunk boundaries.
- `CHANGELOG.md`'s `[Unreleased]` section gained five entries covering HITL-01 through HITL-05's user-visible surface (the `Gate` node and typed `resume_with`, Chronicle `replay`/`fork`, graceful shutdown with its two env vars, and the three thread endpoints) plus the `v3`→`v4` graph fingerprint bump note, following the existing entry conventions (bold lead sentence, cross-links to `MIGRATION.md`/the new mdBook page).
- `.project/v0.10.0/08-traceability-matrix.md` rows G-05, G-06, G-09, G-15 and G-26 each now name concrete test functions and files — 18 total, every one confirmed present in the tree by `grep -rl "fn <name>"` before being written into the matrix, not guessed from memory.
- **Gate evidence, run and recorded honestly:** `cargo test --workspace --no-fail-fast --features web-server` (every test binary green, 0 failures — including the three integration binaries plans 24-05/24-07/24-08 added); `cargo fmt --check` clean; `cargo clippy --workspace --features web-server -- -D warnings` clean; `make security` (`advisories ok, bans ok, licenses ok, sources ok` — one pre-existing yanked-crate warning (`spin`) and one pre-existing duplicate-`thiserror`-version warning, neither new to this plan); `cargo doc --no-deps --workspace --features web-server` succeeds with only pre-existing private-intra-doc-link warnings (this plan touched no `.rs` file, so the warning set is provably unchanged); `cargo +1.88 check --workspace --all-features --all-targets --locked` — 0 errors, confirming the declared MSRV floor; a manual credential-handling review (below) found no issue in Phase 24's new surfaces.
- **`cargo semver-checks` found a real, previously-unregistered break — and it is fixed, not just reported.** `paladin-web`'s `require_authentication` gained a generic type parameter (`0 -> 1`) in plan 24-11 so `ThreadApiState`'s routes could reuse the same middleware `AgentApiState`'s routes already layer; 24-11's own SUMMARY documented this as a Rule 3 deviation but never added the required `MIGRATION.md` §9.2 row. This plan's own gate-evidence run caught it, then closed it in the same commit: a `Deliberate-breaking = Y` row in `MIGRATION.md` §9.2, a matching `[[entry]]` in `.cargo/semver-checks-allowlist.toml` (kept set-equal to the register, verified by replicating CI's own `awk`-based equality check locally), and the actual tool-level suppression (`[package.metadata.cargo-semver-checks.lints] function_requires_different_generic_type_params = "allow"`) in `crates/paladin-web/Cargo.toml`. All 11 published packages (`paladin-ai`, `paladin-ai-core`, `paladin-ports`, `paladin-battalion`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-web`) were then re-checked individually with `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` — CI's exact invocation, chosen deliberately over a single `--workspace --baseline-rev v0.9.0` run, which trips a pre-existing, unrelated `paladin-memory`/`qdrant-client` rustdoc compile failure documented in `ci.yml`'s own comment. Every package passes clean.
- **Workspace line coverage measured at 87.11%** (`cargo llvm-cov --workspace --features web-server`, well above the 82% ADR-0006 floor) — a reduced-feature LOCAL measurement, not CI's canonical `make coverage` figure, because that command's own `scripts/coverage.sh` hard-fails without a reachable Redis and MinIO and no Docker daemon exists in this devcontainer (the exact Phase 17 precedent, WINDOWS.md row 13). Recorded honestly, with a new WINDOWS.md row filing the gap for CI to close authoritatively.
- **Postgres Tier 2 contract cases recorded as CI-only, never as locally passed (D-28).** No new Postgres testing was performed by this plan (24-06 already authored and self-skip-verified the three new contract-suite cases); this plan's own contribution is naming the gap explicitly in a new WINDOWS.md row alongside Phase 22's pre-existing one, since the phase-level gate-evidence pass is exactly the point at which this carried concern needs to be visible for the whole phase, not just the plan that authored the tests.
- **Manual credential-handling review performed, no findings.** Confirmed `WaypointStoreConfig`'s `Postgres` variant carries `url_env` (an environment variable NAME) rather than a connection string, so a credential never lands in a `Debug`/serialized payload; confirmed none of Phase 24's new files (`thread_controller.rs`, `src/application/services/parley/`, `src/config/waypoint_store.rs`, `engine/shutdown.rs`) contain a single `tracing::`/`log::`/`println!` statement that could interpolate a credential; confirmed `ParleyError`'s `#[error(...)]` messages carry only thread ids, fingerprints and parley ids, never a value; confirmed no new outbound HTTP client was added by this phase's new surfaces (the "credential header + no-redirect-follow" rule from `security.instructions.md` has no new call site to apply to). No Snyk step was added or considered required.

## Task Commits

1. **Task 1: The Parley and Chronicle user guide**
   - `c81805c8` — `docs(24-12): add Parley and Chronicle user guide (HITL-01..05)`
2. **Task 2: `MIGRATION.md` sections 9.2 and 9.6, and the changelog**
   - `86a4c85b` — `docs(24-12): fill MIGRATION.md sections 9.2/9.6 and CHANGELOG.md (D-29)`
3. **Task 3: Traceability anchors and the phase gate evidence run**
   - `b011ad71` — `docs(24-12): fill traceability-matrix test anchors for G-05/06/09/15/26`
   - `812bd805` — `fix(24-12): register require_authentication's semver break (X-10, D-06)` (the Rule 1 auto-fix this task's own gate-evidence run surfaced)
   - `3f1ca10f` — `docs(24-12): record two unrun-verify items in the broken-windows ledger`

**Plan metadata:** (this commit)

No RED/GREEN split: every task in this plan is documentation/evidence-gathering against an already-complete implementation (plans 24-01..24-11) — each task's own automated `<verify>` command passing IS the completion signal.

## Files Created/Modified

- `docs/src/user-guides/parley-and-chronicle.md` — new: the phase's mdBook user guide.
- `docs/src/SUMMARY.md` — one new entry, after the control-flow page.
- `MIGRATION.md` — §9.2 (`Waypoint` row resolved, `ParleyPort` new, Phase 24 deliberate-zero note, `require_authentication` deliberate-breaking row), §9.6 (three thread paths).
- `CHANGELOG.md` — five `[Unreleased]` entries plus the fingerprint-bump note.
- `.project/v0.10.0/08-traceability-matrix.md` — test anchors on G-05, G-06, G-09, G-15, G-26.
- `.cargo/semver-checks-allowlist.toml` — one new `[[entry]]` for `paladin-web`/`function_requires_different_generic_type_params`.
- `crates/paladin-web/Cargo.toml` — the matching `[package.metadata.cargo-semver-checks.lints]` suppression.
- `.planning/WINDOWS.md` — two new `unrun-verify` rows (Phase 24 Postgres Tier-2 cases; the reduced-feature coverage measurement), via `gsd_run query windows.append`.

## Decisions Made

- **The `require_authentication` semver break was fixed in place, not just reported (Rule 1).** An unregistered public-API break discovered by this plan's own required gate-evidence step is a correctness/compliance defect against X-10's own register, not a finding to hand off — the fix (MIGRATION.md row + allowlist entry + Cargo.toml suppression) is a direct, minimal, three-file change traceable to the exact CI mechanism (`ci.yml`'s `semver` job's set-equality check) that would otherwise fail on this exact gap.
- **`cargo semver-checks` was run per-package with `--default-features --baseline-version 0.9.0`**, matching `ci.yml`'s own exact invocation, rather than trusting a single `--workspace --baseline-rev v0.9.0` run — the latter's own "enable every feature at once" heuristic trips a pre-existing, unrelated `paladin-memory`/`qdrant-client` compile break that `ci.yml`'s own comment already documents and attributes to a future phase. Running the CI-equivalent command is what actually proves (or disproves) the gate; an easier command that reports a false failure is not a substitute.
- **Workspace line coverage recorded as a reduced-feature local measurement (87.11%, `--features web-server`), not CI's canonical figure** — `scripts/coverage.sh` requires a reachable Redis and MinIO, both unavailable without Docker in this devcontainer. This mirrors Phase 17's own precedent (WINDOWS.md row 13, later closed by a CI run) exactly: record the honest local number, file the gap, do not fabricate the canonical command's success.
- **Two WINDOWS.md rows filed, not zero** — one for Phase 24's own new Postgres Tier-2 contract-suite cases (self-skipping since 24-06, but never previously named in the cross-phase ledger), one for this plan's reduced-feature coverage measurement. Both are `unrun-verify`, `open`, pointing at the exact CI job (`postgres-integration`, `coverage`) that closes each.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `require_authentication`'s unregistered semver break, discovered during Task 3's gate-evidence run**

- **Found during:** Task 3 (`cargo semver-checks --workspace --baseline-rev v0.9.0`, then confirmed per-package with `--default-features --baseline-version 0.9.0` matching CI exactly)
- **Issue:** `paladin-web`'s `require_authentication` (a pre-existing public function at `v0.9.0`, confirmed via `git show v0.9.0:crates/paladin-web/src/agent_auth.rs`) gained a generic type parameter in plan 24-11 (landed to let `ThreadApiState`'s routes reuse the same middleware `AgentApiState`'s routes already layer) — a real, `cargo semver-checks`-flagged major-version-requiring break (`function_requires_different_generic_type_params`, `0 -> 1` generic types). 24-11's own SUMMARY documented the change as a Rule 3 deviation but never added the `MIGRATION.md` §9.2 row X-10 requires for a pre-existing public-type change, and no `.cargo/semver-checks-allowlist.toml` entry existed — so `ci.yml`'s `semver` job would have failed on this exact gap the moment it ran.
- **Fix:** Added a `Deliberate-breaking = Y` row to `MIGRATION.md` §9.2 naming the exact change and its justification; added the matching `[[entry]]` to `.cargo/semver-checks-allowlist.toml` (verified set-equal to the MIGRATION.md register by replicating CI's own `awk`-based diff locally); added the actual `[package.metadata.cargo-semver-checks.lints] function_requires_different_generic_type_params = "allow"` suppression to `crates/paladin-web/Cargo.toml`.
- **Files modified:** `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `crates/paladin-web/Cargo.toml`
- **Verification:** `cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0` — clean (0 fail) after the fix, versus 1 fail before it; `cargo fmt --check` and `cargo clippy -p paladin-web --all-targets -- -D warnings` both clean after the `Cargo.toml` edit (metadata-only, no code change); `cargo check -p paladin-web` and `cargo test -p paladin-web` both green.
- **Committed in:** `812bd805`

---

**Total deviations:** 1 auto-fixed (Rule 1, a genuine unregistered-API-break correctness gap surfaced by this plan's own required gate-evidence step). **Impact on plan:** Necessary correctness fix for the X-10 register this plan's own Task 3 exists to verify; no scope creep — all three touched files are exactly what CI's `semver` job's set-equality check requires to stay consistent.

## Issues Encountered

- **`cargo semver-checks --workspace --baseline-rev v0.9.0` (a git-tag-based, all-features run) is NOT equivalent to CI's actual gate** and produced a misleading `paladin-ai` rustdoc build failure from a pre-existing, unrelated `paladin-memory`/`qdrant-client` `VectorParams` field mismatch (documented in `ci.yml`'s own comment and Phase 22's deferred-items, out of this phase's scope). Resolved by running the CI-equivalent per-package `--default-features --baseline-version 0.9.0` invocation instead, which is what actually proves the gate and is what surfaced the real `require_authentication` break cleanly, without the unrelated noise.
- **No Docker daemon in this devcontainer** blocked both the canonical `make coverage` invocation (needs Redis/MinIO) and any local Postgres Tier-2 run — both are pre-existing, carried environmental limitations (Phase 17's coverage precedent, Phase 22/23/24's Postgres precedent), not new to this plan. Both are recorded honestly rather than worked around with a fabricated pass.
- No pre-commit hook timeout issues arose — this plan's commits are documentation/config only (`.md`/`.toml` files plus one Cargo.toml metadata table), none triggering the cold `cargo clippy --workspace --all-targets --all-features` hook's multi-minute cost; every commit still used `--no-verify` per the orchestrator's `workflow.worktree_skip_hooks=true` allowance for consistency with the rest of this wave, with the equivalent `cargo fmt --check`/`cargo clippy` commands run and verified clean manually before each commit.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 24 (HITL-01 through HITL-05) is now fully documented, registered and gate-evidenced: the mdBook page teaches the whole feature with the required notification example and both accepted-risk warnings; `MIGRATION.md`/`CHANGELOG.md`/the traceability matrix record exactly what shipped; every HITL requirement's traceability row names findable, real test anchors; and the phase's X-10 API register is now complete and self-consistent (the `require_authentication` gap this plan found and closed was the only unregistered break across all 11 published packages).
- Two `WINDOWS.md` rows remain open for CI to close authoritatively: Phase 24's new Postgres Tier-2 contract-suite cases (`postgres-integration` job) and the reduced-feature local coverage measurement (`coverage` job) — both are pre-existing environmental limitations of this devcontainer, not code defects, and both have a direct historical precedent (Phase 17's coverage row, Phase 22's Postgres row) for how CI resolves them.
- No blockers. This is the last plan in Phase 24 per `24-CONTEXT.md`'s plan/wave decomposition; the phase is ready for the orchestrator's post-wave STATE.md/ROADMAP.md update and phase closeout.

## Self-Check: PASSED

`docs/src/user-guides/parley-and-chronicle.md` verified present on disk; all five commit hashes (`c81805c8`, `86a4c85b`, `b011ad71`, `812bd805`, `3f1ca10f`) verified present in `git log --oneline`; `MIGRATION.md`/`CHANGELOG.md`/`.project/v0.10.0/08-traceability-matrix.md`/`.cargo/semver-checks-allowlist.toml`/`crates/paladin-web/Cargo.toml` diffs verified confined to their documented sections.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
