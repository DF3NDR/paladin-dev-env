---
phase: 22-battlefield-state-superstep-engine
verified: 2026-09-02T04:30:00Z
status: human_needed
score: 8/8 roadmap success criteria verified (must-have truths pass); 4 items require human decision
behavior_unverified: 1 # Postgres Tier 2 real-server contract-suite pass (Docker unavailable in this sandbox)
overrides_applied: 0
human_verification:
  - test: "Run `make test-integration-docker` (or otherwise bring up `postgres-test`) and confirm `PostgresWaypointStore` passes the full shared contract suite against a real Postgres server."
    expected: "All contract functions pass identically to SQLite/InMemory, matching WINDOWS.md ledger item 22."
    why_human: "No Docker daemon is available in this verification sandbox; only compile, lint and the clean-skip path were provable here."
  - test: "Decide whether `WaypointRetentionConfig::prune`'s delete-then-resave sequence (crates/paladin-storage/src/waypoint/retention.rs:130-151) is acceptable to ship as-is, given it is not atomic against a crash/backend-failure between `delete_thread` and the resave loop, and can destroy a thread's protected (latest / AwaitingInput) Waypoints in that window."
    expected: "Either accept the risk explicitly (retention is disabled by default and not wired to any scheduler yet, per WR-02), or require a fix/override before this ships as a callable production routine."
    why_human: "This is a design-level data-loss risk identified by code review (22-REVIEW.md CR-01/WR-03), not something the phase's own tests exercise (no test injects a crash mid-prune). It contradicts the module's own documented invariant under failure, but does not violate any single tested must-have truth."
  - test: "Decide whether the silent-stranded-node gap (a non-entry `WarGraph` node whose only incoming edges eventually trace back to itself has no path to ever become ready, `WarGraph::validate` does not catch it, and the run reports `RunOutcome::Completed` as if the whole graph executed) needs a validation fix before Phase 23+ build on top of the engine."
    expected: "Either add a reachability-from-entry check to `WarGraph::validate` (22-REVIEW.md CR-02's suggested fix), or explicitly accept this as a known, documented limitation (every self-loop test and the E2E-1 fixture itself already work around it by making the looping node a graph entry — see the comment at tests/integration/e2e_crash_resume_test.rs:112-127)."
    why_human: "Design-level judgment call on whether 'self-loop and cycles work' (which IS true and tested for entry nodes) is sufficient, or whether the unguarded general case is a release blocker for a framework whose headline claim is durable, correct cyclic execution."
  - test: "Decide whether ENG-NFR-01's measured SQLite Waypoint-save p50 (73.09 ms, 7.3x over the 10ms target) is acceptable for v0.10.0, or whether it needs a follow-up (e.g. explicit `journal_mode=WAL`/`synchronous=NORMAL` SQLite pragmas) before release."
    expected: "A decision recorded (accept as measured baseline vs. require a fix) since this is a headline non-functional claim of a 'checkpoint every superstep' design."
    why_human: "The plan's own instruction was to measure and record honestly rather than tune to pass — which the executor did (22-10-SUMMARY.md) — but the miss itself is a business/architecture decision, not something this verifier can resolve."
---

# Phase 22: Battlefield State & Superstep Engine Verification Report

**Phase Goal:** A Rust developer can declare typed shared state exchanged through per-field dispatch rules, and run cyclic multi-agent graphs in supersteps that checkpoint automatically and resume with zero re-execution after a crash.
**Verified:** 2026-09-02T04:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Method

This is an 11-plan, 7-wave phase (ENG-01 through ENG-08). Rather than trust SUMMARY.md claims, I:

1. Read all 11 PLAN.md files (must_haves, acceptance criteria, threat models) and all 11 SUMMARY.md files.
2. Read 22-REVIEW.md (code review, 2 Critical / 4 Warning / 1 Info findings) and 22-VALIDATION.md.
3. Built the full workspace (`cargo build --workspace --all-features`) — succeeded.
4. Ran the three cross-crate integration tests directly: `war_engine_tracer` (3/3 pass), `e2e_crash_resume` (27/27 pass, including `e2e_1_crash_resume_matches_control_run_with_no_reexecution`), `golden_bridge_equivalence` (31/31 pass).
5. Ran lib test suites directly: `paladin-ai-core --lib` (427 pass), `paladin-ports --lib` (105 pass), `paladin-battalion --lib` (316 pass), `paladin-storage --lib --features sqlite` (64 pass, including the SQLite `WaypointPort` contract suite and retention routine tests).
6. Ran `cargo clippy --workspace --all-targets -- -D warnings` (clean) and `cargo fmt --check` (clean).
7. Grepped for the literal acceptance-criteria patterns from each plan (SQL injection, `toposort`/`petgraph` boundary violations, `BestEffort` misuse, raw JSON values in errors, `DispatchRegistry` core-boundary leakage) and manually inspected any surprising hit.
8. Read the source of the two Critical findings in 22-REVIEW.md (`retention.rs`, `superstep.rs`'s `Frontier`) directly to confirm or refute them against the actual code, rather than taking the review's word for it.
9. Confirmed MIGRATION.md, the `semver`/`msrv` CI jobs, the semver allowlist, `rust-version` declarations, the `postgres` feature/facade passthrough, and `cargo tree -e features` default-set exclusion, all directly.
10. Cross-referenced REQUIREMENTS.md against every plan's `requirements:` frontmatter field.

I could not run the Postgres Tier 2 contract suite (no Docker daemon in this sandbox) or re-run the ~35-minute full-workspace coverage measurement — both are called out explicitly below rather than assumed.

## Goal Achievement — Roadmap Success Criteria

| # | Success Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Developer declares `BattlefieldSchema`, nodes exchange typed `StateDelta`s via 5 dispatch rules; unknown-field/missing-required are hard structured errors; no new core deps (ENG-01) | ✓ VERIFIED | `crates/paladin-core/src/platform/container/battlefield.rs` has `DispatchRule::{LastWrite,Append,MergeObject,Sum,Custom}`, `get::<T>`/`set::<T>`, `initialize`; `battlefield_error.rs` has all 6 variants, zero `serde_json::Value` embedded (`grep` = 0); `paladin-ai-core --lib` 427 tests pass; `git diff --stat crates/paladin-core/Cargo.toml` shows no new dependency per 22-01-SUMMARY.md |
| 2 | `WarEngine` executes cyclic graphs (self-loops included) in bounded supersteps, deterministic frontier/merge (byte-identical over 20+ randomized-scheduling iterations), join/defer never deadlocks on a not-firing branch (ENG-02) | ✓ VERIFIED, with 1 flagged edge case | `engine::graph::validate` accepts cycles/self-loops, rejects zero limits (`toposort`/`petgraph` boundary greps = 0 inside `WarGraph`'s own validation); `paladin-battalion --lib` 316 tests pass including the diamond-join, not-firing-branch, defer, 20-iteration shuffled-determinism and 100-iteration multi-thread stress tests (per 22-05/22-07 SUMMARYs). **Flagged:** 22-REVIEW.md CR-02 (confirmed by direct code read of `superstep.rs` `Frontier`) — a non-entry node whose only incoming edges eventually trace back to itself is never marked dead nor ever ready, so it silently never executes while the run still reports `Completed`; `WarGraph::validate` has no reachability check to catch this. Every self-loop test in the tree (including E2E-1's own fixture, see its code comment at lines 112-127) works around this by making the looping node a graph entry — the tested/claimed shape (self-loop as entry) does work; the untested shape (self-loop fed only by itself, not an entry) does not. |
| 3 | Exactly one Waypoint persisted automatically per superstep, `(thread_id, waypoint_id)` addressed with parent lineage and stable fingerprint; write failure fails the run under default `Strict` (ENG-03) | ✓ VERIFIED | `superstep.rs` persists one Waypoint per superstep after merge, before the continue decision (tested: 3-superstep run -> exactly 3 Waypoints with correct parent chain); `WaypointDurability::Strict` is the un-chosen default and `BestEffort` is used nowhere outside its own two tests (`grep` confirms 0 unexpected hits); `GraphFingerprint` is `v1:{blake3_hex}` over a sorted canonical stream, confirmed stable across repeated construction |
| 4 | Program scenario E2E-1: engine killed after superstep 3, fresh engine/backend/thread_id resumes with zero re-execution, final Battlefield equals an uninterrupted control run, exactly one Waypoint per completed superstep (ENG-04) | ✓ VERIFIED | `tests/integration/e2e_crash_resume_test.rs::e2e_1_crash_resume_matches_control_run_with_no_reexecution` — ran directly, passes. Uses a real `SqliteWaypointStore` over a temp file (not in-memory), drops and reconstructs the engine, asserts all 3 E2E-1 clauses separately plus loop-count equality |
| 5 | Three `WaypointPort` backends pass one shared contract suite; legacy bridges reproduce data flow with golden tests; `MIGRATION.md` §9 skeleton exists; `semver`/`msrv` CI jobs green on every PR (ENG-05, ENG-06, ENG-07, ENG-08) | ✓ VERIFIED for InMemory/SQLite (Tier 1) and all CI/doc scaffolding; ⚠️ Postgres Tier 2 real-server pass unverified here | InMemory (5 tests) and SQLite (16 tests incl. all 13+ contract functions) both green, ran directly. Postgres: `cargo build -p paladin-storage --features postgres` succeeds, code mirrors SQLite structure exactly, `cargo tree -e features -p paladin-ai` confirms no postgres driver in the default set — but the actual contract-suite-against-a-live-server pass could not be run here (no Docker), consistent with the pre-existing, honestly-recorded WINDOWS.md ledger item 22 ("unrun-verify", open). `golden_bridge_equivalence` (31/31) proves Formation/Phalanx/Campaign bridges byte-for-byte, including the false-branch and empty-list cases, over untouched legacy services (`git diff --stat` on the 4 legacy service files is empty). `MIGRATION.md` has all 8 `## 9.x` headings, `M-B-01`-`M-B-03`, an 11+-row §9.2 register, zero un-owned `TBD`s. `.github/workflows/ci.yml` has both `semver:` and `msrv:` jobs with no `needs:` edge, pinned to the published `0.9.0` baseline and package names (`paladin-ai-core`, not the directory name); `.cargo/semver-checks-allowlist.toml` exists and is empty (correctly, since Phase 22 touched no pre-existing public type per its own note) |

**Score:** 5/5 roadmap Success Criteria hold as literally worded and tested. 1 present-but-behavior-unverified item (Postgres Tier 2 live-server pass) and 2 review-confirmed design gaps (CR-01, CR-02) are surfaced below rather than silently absorbed into a clean pass.

## Requirements Coverage (REQUIREMENTS.md cross-reference)

| Requirement | Owning Plan(s) | Status | Evidence |
|---|---|---|---|
| ENG-01 | 22-01, 22-02, 22-07 | ✓ SATISFIED | Typed accessors, schema enforcement, 5-rule deterministic merge with conflict detection; unit-tested exhaustively (see above) |
| ENG-02 | 22-01, 22-05, 22-07, 22-10 | ✓ SATISFIED (1 flagged edge case, CR-02) | Cyclic superstep loop, bounded iteration, join/defer, 20-iteration determinism + 100-iteration stress test |
| ENG-03 | 22-01, 22-03, 22-08 | ✓ SATISFIED | One Waypoint/superstep, parent lineage, `Strict` durability default |
| ENG-04 | 22-01, 22-08 | ✓ SATISFIED | `resume` restores Battlefield/vanguard/visit-counts; E2E-1 integration test passes |
| ENG-05 | 22-01, 22-03, 22-06, 22-10 | ✓ SATISFIED (Postgres live-server run unverified here) | Contract suite shared across 3 backends; retention routine correct under tested (non-crash) conditions — see CR-01 flag |
| ENG-06 | 22-11 | ✓ SATISFIED | Golden byte-for-byte equivalence tests, legacy services untouched, coverage gate closed (86.35%/96.17%, measured and recorded with methodology) |
| ENG-07 | 22-09 | ✓ SATISFIED | `TraceSink` (fire-and-forget, drop-oldest, counted), `NodeInterceptor` chain (empty-by-default equivalence proven), `CancellationToken` -> `Halted` Waypoint, all with dedicated blocking/erroring/timeout tests |
| ENG-08 | 22-04 | ✓ SATISFIED | `MIGRATION.md`, `semver`/`msrv` CI jobs, `rust-version` on all 11 publishable crates |

No orphaned requirements: REQUIREMENTS.md's ENG-01..08 all appear in at least one plan's `requirements:` field, and the union of all plans' `requirements:` fields is exactly ENG-01..08.

## Direct Code-Review Finding Verification

22-REVIEW.md (advisory) reported 2 Critical and 4 Warning findings. I read the actual source for both Critical findings rather than taking the review at face value:

### CR-01 (confirmed real): Retention prune is not atomic
`crates/paladin-storage/src/waypoint/retention.rs:130-151` — `prune()` fetches survivor Waypoints via `get()`, calls `port.delete_thread()` (wiping the ENTIRE thread, survivors included), then `save()`s each survivor back one at a time. A crash/backend failure between `delete_thread` and the completion of the resave loop destroys the thread's protected Waypoints — including the "latest" and any `AwaitingInput` Waypoint the routine's own doc comment calls "unrecoverable" to lose. All of the plan's own tests pass because none of them inject a mid-prune failure. **Mitigating factor confirmed by direct grep:** `retention::prune` and `WaypointRetentionConfig` are not invoked from anywhere outside their own unit tests (WR-02) — nothing in the tree schedules or calls this routine today, so the risk is currently latent, not live. Flagged as a human-verification item above rather than a BLOCKER, since it does not violate any single must_haves truth that this phase's tests exercise, but it is a real, review-confirmed gap in a "durable" system's own stated invariant.

### CR-02 (confirmed real): Stranded self-referencing node
`crates/paladin-battalion/src/engine/superstep.rs`'s `Frontier::propagate_dead`/`is_ready` (lines 582-727) — traced by hand. A non-entry node `N` whose only incoming edge is a self-loop (`N -> N`) is never marked `dead` (its one incoming edge is `Pending`, not resolved) and never becomes `is_ready` (the same `Pending` self-edge blocks it) — it can never execute. `WarGraph::validate` (graph.rs:211-249) has no reachability-from-entry check, so this graph shape passes validation and the run completes with `RunOutcome::Completed`, silently omitting the node's work. Confirmed as a known, worked-around limitation: `tests/integration/e2e_crash_resume_test.rs`'s own fixture comment (lines 112-127) states this exact mechanism and explains why its self-loop node is deliberately made a graph *entry* to avoid it — every self-loop test in the tree does the same. The **tested and claimed** shape (self-loop as entry point) genuinely works; the **untested, unguarded** shape (self-loop-only node that is not an entry) silently drops work with no error.

Both findings are real and are surfaced as human-verification items in the frontmatter above rather than either being silently waved through or used to fail the whole phase outright — neither breaches a must_haves truth that this phase's own test suite exercises, but both are legitimate design-level risks a human should explicitly accept or require fixed before later phases (23+) build on this engine.

## Anti-Pattern / Prohibition Scan

| Check | Result |
|---|---|
| `TBD`/`FIXME`/`XXX` in phase-touched source files | 0 hits |
| `serde_json::Value` embedded in `BattlefieldError` | 0 hits (prohibition resolved) |
| `format!(...SELECT/INSERT/UPDATE/DELETE...)` in SQL backends | 0 hits (bound parameters only, prohibition resolved) |
| `BestEffort` selected outside its own file/tests | 0 hits (prohibition resolved) |
| `toposort` reused in `engine/` | 0 hits |
| `petgraph` reused in `engine/` | 3 hits, all in `bridges.rs` reading the **legacy** `Campaign`'s existing petgraph structure to build a `WarGraph` — not wrapping `WarGraph` itself in petgraph. Confirmed by direct inspection: not a violation of the Plan 22-05 invariant, which was scoped to the engine's own graph type |
| `DispatchRegistry` referenced in `paladin-core` | 3 substring hits, all `CustomDispatchRegistry`/`CustomDispatchResolver` — plain `HashMap` type aliases distinct from the application-layer `DispatchRegistry` struct in `paladin-battalion`. Confirmed by direct inspection: no `use` of `paladin_battalion` anywhere in `paladin-core`; the X-01 boundary holds despite the literal grep from Plan 22-07's acceptance criteria matching by substring |
| MIGRATION.md un-owned `TBD` | 0 hits |

## Non-Functional Claims (measured, not asserted)

| Claim | Measured | Target | Result |
|---|---|---|---|
| ENG-NFR-01: SQLite Waypoint save p50 (Battlefield just under 1 MiB) | 73.09 ms | < 10 ms | **Miss (7.3x over)** — honestly recorded per 22-10-SUMMARY.md, not tuned away. Likely cause (documented, not independently re-verified): no explicit `journal_mode`/`synchronous` pragma, so every write may pay a full `fsync` |
| ENG-NFR-02: Battlefield clones per superstep | 1 (pointer-identity proven) | ≤ 1 + concurrent node views | Pass |
| Coverage (ADR-0006 gate) | 86.35% workspace / 96.17% new modules | ≥82% / ≥85% | Pass, per 22-11-SUMMARY.md's documented methodology (`lcov.info` LF/LH summed directly, not the rosier summary command) — not independently re-run here due to its ~35-minute cost, but the recorded command, methodology and file scope match this project's own established (ADR-0006) practice |

The ENG-NFR-01 miss is not a roadmap Success Criterion or a REQUIREMENTS.md item on its own — the phase's obligation was to *measure* it, which it did, honestly. It is surfaced as a human-verification item because it is a headline non-functional claim of a "checkpoint every superstep" design.

## Deferred / Disclosed Items (not gaps)

- **Retention not wired to any scheduler (WR-02).** `WaypointRetentionConfig`/`retention::prune` are complete and unit-tested but nothing constructs or invokes them from `Settings` or a job scheduler yet. This was explicitly disclosed as deferred in 22-06-SUMMARY.md ("ready for a later plan to wire into the job-scheduling system") and the config defaults `enabled: false`, so no operator gets silent behavior today. Not a phase-22 gap; worth tracking for whichever phase claims ENG-FR-18's "callable from the existing job-scheduling system" wiring in full.
- **`from_campaign` schema deviation (WINDOWS.md ledger #23).** `from_campaign` adds one dedicated `LastWrite` field per Paladin beyond the literal three-field default schema, to avoid spurious `DispatchConflict`s on concurrent DAG fan-out. This is a disclosed, reasoned deviation from ENG-FR-19's literal wording, verified not to break golden-test byte-equivalence (`golden_bridge_equivalence` passes including the diamond fan-in case).
- **`run_timeout` carried but not enforced (IN-01).** Explicitly deferred to a later phase in both the plan text and rustdoc; not a phase-22 obligation.

## Human Verification Required

See frontmatter `human_verification` for the 4 items (Postgres Tier 2 live-server pass, CR-01 retention atomicity, CR-02 stranded-node validation gap, ENG-NFR-01 target miss). None of these are silent — all four are either pre-existing, explicitly-tracked WINDOWS.md ledger items, or code-review findings confirmed by direct source inspection in this verification pass.

## Gaps Summary

No must_haves truth failed as literally tested. No artifact is missing or a stub. No key link is unwired. The full available test suite (workspace build, 3 integration tests, 4 crates' lib tests, clippy, fmt) is green. The phase's status is `human_needed` rather than `passed` because: (1) one contract-suite leg (Postgres against a real server) cannot be run in this sandbox and is already an open, tracked WINDOWS.md item; and (2) two code-review-confirmed design gaps (non-atomic retention prune, unguarded stranded-node graph shape) are real risks that a human should explicitly accept or require closed before Phase 23+ builds further on this engine, even though neither one breaches a must_haves truth this phase's own tests exercise.

---

_Verified: 2026-09-02T04:30:00Z_
_Verifier: Claude (gsd-verifier)_
