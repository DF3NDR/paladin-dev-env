---
phase: 22-battlefield-state-superstep-engine
verified: 2026-09-02T22:45:00Z
status: passed
score: 8/8 requirements satisfied; 5/5 roadmap success criteria hold; 1 new human-decision item (CR-01)
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: "5/5 roadmap SCs hold, 4 human-verification items"
  gaps_closed:
    - "G-22-1: Postgres Tier 2 contract suite now executes against a live server in CI (postgres-integration job) — closed on execution evidence (CI run 33688238662 / job 100440861780), independently re-fetched and confirmed in this pass, not just re-read from SUMMARY.md"
    - "G-22-2: WaypointPort::prune_thread (delete-only-unprotected, monotone/idempotent/crash-safe by construction) replaces the delete-then-resave sequence; retention.rs rewritten on it; protected-set moved to application layer; fault-injection acceptance test passes"
    - "G-22-3 / BUG-02: WarGraph::validate now rejects any node outside the eligible set (reachable-from-entry ∪ dynamic_target) with EngineError::UnreachableNode, before any node executes; every fixture that previously dodged the defect is audited and corrected"
    - "ENG-NFR-01 (SQLite save-latency miss): decision recorded via UAT test 4 — accepted as measured baseline (result: pass)"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Decide whether WarGraph::fingerprint() must be extended to hash defer_flags (and dynamic_targets) before Phase 23+ builds further on resume, given the code's own doc comment claims resuming makes a fresh-vs-resumed divergence 'structurally impossible' but the fingerprint does not detect a defer_flags change between the graph that crashed and the graph passed to resume."
    expected: "Either fix WarGraph::fingerprint() to include defer_flags/dynamic_targets in its hashed bytes (22-REVIEW.md CR-01's suggested patch), add a regression test asserting two graphs differing only in add_deferred_node vs add_node produce different fingerprints, or explicitly accept the gap and narrow the 'structurally impossible' doc-comment claim to match what the check actually covers."
    disposition: "ROUTED TO PHASE 22.1 (recorded 2026-09-02): per the developer's standing instruction at the 22-17 checkpoint — all gaps not closed in Phase 22 route to inserted Phase 22.1 — CR-01 is registered as item (4) of Phase 22.1's goal in ROADMAP.md, alongside the readiness defect and the MSRV decision. Not fixed in Phase 22."
    why_human: "This is a design-level correctness gap in the resume-safety guarantee identified by a fresh code review (22-REVIEW.md CR-01, dated after the 22-17 gap-closure checkpoint closed), confirmed here by direct read of graph.rs:398-437 (fingerprint omits self.defer_flags and self.dynamic_targets) and mod.rs:461 (the 'structurally impossible' claim). It is not caught by any of this phase's own tests (fingerprint_is_deterministic_across_calls and fingerprint_is_unchanged_by_insertion_order cover determinism and ordering only, never a defer-flag difference), and ENG-FR-14's literal text ('hash over node ids, edge specs, schema') is silent on defer_flags either way, so it does not fail a literal must_have truth this phase declared. It has not been triaged through UAT or routed to Phase 22.1 the way the readiness defect and MSRV items were at the 22-17 checkpoint — it is new since that checkpoint closed."
---

# Phase 22: Battlefield State & Superstep Engine Verification Report

**Phase Goal:** A Rust developer can declare typed shared state exchanged through per-field dispatch rules, and run cyclic multi-agent graphs in supersteps that checkpoint automatically and resume with zero re-execution after a crash.
**Verified:** 2026-09-02T22:45:00Z
**Status:** passed (CR-01 routed to Phase 22.1 by recorded developer disposition; see frontmatter)
**Re-verification:** Yes — after gap closure (plans 22-12..22-17, closing UAT gaps G-22-1/G-22-2/G-22-3)

## Method

This supersedes the pre-gap-closure `22-VERIFICATION.md`. Rather than trust SUMMARY.md/UAT.md
resolution claims, for each of the three closed gaps I read the actual current source and, where the
claim rested on an external CI run rather than something reproducible in-sandbox (G-22-1), I
independently re-fetched that run and its job logs via `gh api` rather than re-reading the SUMMARY's
account of it. I also read the freshly generated `22-REVIEW.md` (dated after the 22-17 checkpoint
closed) end to end and verified its one Critical finding against source directly, since it was not
part of the UAT closure cycle and is not yet triaged.

1. Read all three UAT gaps (G-22-1/2/3) and their `resolved` dispositions in `22-UAT.md`.
2. Read the gap-closure plans (22-12 through 22-17) — `must_haves`, `key-decisions`, `coverage`.
3. Read `crates/paladin-battalion/src/engine/graph.rs` (`validate_eligible_set`, `fingerprint`),
   `crates/paladin-ports/src/output/waypoint_port.rs` (`delete_waypoint`, `prune_thread`), and
   `crates/paladin-storage/src/waypoint/retention.rs` directly to confirm each fix's shape against
   the gap's `missing:` list, not just its `resolution:` prose.
4. Built the workspace (`cargo build --workspace --all-features`) — clean.
5. Ran the directly relevant test suites: `paladin-battalion --lib engine::` (108 passed, 1
   intentionally `#[ignore]`d), `--test e2e_crash_resume` (27/27, including the E2E-1 zero-re-execution
   assertion), `paladin-storage --lib --features sqlite waypoint::` (67 passed, including all new
   `delete_waypoint`/`prune_thread` contract functions and every pre-existing retention test),
   `--test waypoint_retention_fault_injection` (3/3, including the SQLite cancellation-mid-prune
   acceptance test).
6. Ran `cargo fmt --check` (clean) and `cargo clippy -p paladin-battalion -p paladin-storage
   -p paladin-ports --all-targets -- -D warnings` (clean).
7. For G-22-1, fetched CI run `33688238662` via `gh api` (not trusted from SUMMARY text), located the
   `postgres-integration` job (`100440861780`, conclusion `success`), and pulled its raw log to
   independently confirm all four evidence facts 22-17-SUMMARY.md claims: container reported
   `healthy`, `pg_isready` returned `accepting connections`, the suite reported
   `test result: ok. 26 passed; 0 failed`, and the SKIP-detection step ran.
8. Read `22-REVIEW.md` (fresh, post-checkpoint) in full and verified its one Critical finding (CR-01)
   by reading `graph.rs:398-437` and `mod.rs:448-461` directly — confirmed real, not yet triaged
   through UAT or routed to Phase 22.1.
9. Confirmed no unreferenced `TBD`/`FIXME`/`XXX` markers in any file this gap-closure cycle touched.
10. Re-confirmed REQUIREMENTS.md's ENG-01..08 traceability table still shows all eight as Complete
    with Phase 22 as sole owner, and that the union of all fourteen plans' `requirements:` fields
    (01-11 plus 12-17) is exactly ENG-01..08 with no orphans.

## Gap Closure Verification (G-22-1, G-22-2, G-22-3)

| Gap | UAT Disposition | Verified In Codebase | Evidence |
|---|---|---|---|
| G-22-1 (Postgres Tier 2 suite never executes anywhere) | resolved | ✓ CONFIRMED | `.github/workflows/ci.yml:811-886` has `postgres-integration` job: starts `postgres-test`, waits for anchored `healthy`, asserts `pg_isready`, runs `waypoint::postgres` single-threaded with `WAYPOINT_POSTGRES_TEST_URL` set, greps for `SKIP:`. Independently re-fetched CI run `33688238662` (job `100440861780`, `conclusion: success`) via `gh api` and read its raw log directly: `accepting connections` (pg_isready), `test result: ok. 26 passed; 0 failed; 0 ignored; ... finished in 8.55s`, and the SKIP-detection step present and unfired. This is execution evidence, not a job-definition-exists claim. |
| G-22-2 (retention prune's delete-then-resave crash window) | resolved | ✓ CONFIRMED | `WaypointPort::delete_waypoint`/`prune_thread` (`crates/paladin-ports/src/output/waypoint_port.rs:243,290`) exist; SQLite/Postgres override transactionally. `retention.rs`'s `prune()` (read in full) no longer calls `delete_thread` anywhere — it calls `port.prune_thread(&thread_id, &keep_vec)` exactly once per thread with something to remove, after computing `keep_ids` from a caller-supplied `protected` fn (application-layer, per `src/application/services/waypoint_retention.rs`). `tests/integration/waypoint_retention_fault_injection_test.rs` (3/3 pass, run directly): fault-injection sweep, real-engine resume-after-interrupted-prune, and SQLite mid-prune cancellation, all assert the keep-set survives. |
| G-22-3 / BUG-02 (silent stranded node, `Completed` lies) | resolved | ✓ CONFIRMED | `WarGraph::validate_eligible_set()` (`graph.rs:301-395`, read in full) computes the eligible set as a fixed-point worklist seeded from `entry` ∪ `dynamic_targets`, expanded over declared edges; every node outside it fails with `EngineError::UnreachableNode { nodes, reason }` naming every offender. A no-entry-declared graph gets a distinct, clearer error. The one committed regression suite (`engine::lib`, 108/108 pass) includes the reachability tests; the residual, distinct "readiness" defect (self-loop node fed by an upstream edge, not a strandedness case) is recorded as a real `#[ignore]`d reproduction (`self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`, confirmed present at `superstep.rs:1385`, confirmed `1 ignored` in the direct test run) — correctly NOT claimed as fixed, and explicitly routed to inserted Phase 22.1 per `ROADMAP.md`'s own text. |

**All three UAT gaps are genuinely closed** — not merely marked `resolved` in the frontmatter. G-22-1 in particular is the one gap whose evidence I could not take on faith from any SUMMARY (the whole point of the gap was "green without ever executing"); I re-derived the evidence from the CI provider directly rather than reading 22-17-SUMMARY.md's account of it.

## Goal Achievement — Roadmap Success Criteria (re-confirmed)

| # | Success Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Typed `BattlefieldSchema`, 5 dispatch rules, structured errors, no new core deps (ENG-01) | ✓ VERIFIED | Unchanged since prior verification pass; not touched by gap-closure plans. `paladin-ai-core --lib` battlefield/waypoint tests pass as part of the workspace build. |
| 2 | Cyclic graphs execute in bounded supersteps, deterministic merge, join/defer never deadlocks (ENG-02) | ✓ VERIFIED, stranded-node gap now closed | `engine::lib` 108/108 pass. The BUG-02 stranded-node gap flagged in the prior verification round is now closed by `EngineError::UnreachableNode` (see G-22-3 above). The distinct readiness defect is disclosed, reproduced, and routed to Phase 22.1 rather than silently left. |
| 3 | Exactly one Waypoint per superstep, fingerprint, `Strict` durability default (ENG-03) | ✓ VERIFIED, with 1 new human-decision item | Unit-tested behavior unchanged and still passes. **New finding (CR-01, this pass):** `WarGraph::fingerprint()` does not hash `defer_flags`/`dynamic_targets`, so a defer-flag change between an interrupted run and its resume passes the fingerprint check silently — see Human Verification below. Confirmed by direct read of `graph.rs:398-437`, not merely cited from the review. |
| 4 | E2E-1 crash-resume, zero re-execution, final Battlefield equals control run (ENG-04) | ✓ VERIFIED | `e2e_1_crash_resume_matches_control_run_with_no_reexecution` run directly, passes (27/27 in the file). |
| 5 | Three `WaypointPort` backends pass one contract suite; legacy bridges; `MIGRATION.md`; `semver`/`msrv` CI (ENG-05..08) | ✓ VERIFIED, Postgres leg now closed | InMemory/SQLite: 67/67 direct. Postgres: closed via independently-confirmed live CI run (G-22-1 above), superseding the prior pass's "could not run, no Docker" finding. `retention.rs`'s prune_thread contract functions pass on all backends touched here. |

**Score:** 5/5 roadmap Success Criteria hold. 3/3 gaps from the prior UAT cycle are genuinely closed. 1 new human-decision item (CR-01) surfaced by a fresh code review dated after the gap-closure checkpoint, not yet triaged.

## Requirements Coverage (REQUIREMENTS.md cross-reference)

| Requirement | Status | Evidence |
|---|---|---|
| ENG-01 | ✓ SATISFIED | Unchanged; not touched by gap-closure plans. |
| ENG-02 | ✓ SATISFIED (BUG-02 closed; distinct readiness defect disclosed, routed to Phase 22.1) | Plans 22-15/22-16; `engine::lib` 108/108. |
| ENG-03 | ✓ SATISFIED (1 new human-decision item, CR-01) | Fingerprint mechanism functions as literally specified by ENG-FR-14; a gap versus the code's own broader "structurally impossible divergence" claim is disclosed, not silently absorbed. |
| ENG-04 | ✓ SATISFIED | `resume` restores state; E2E-1 passes. |
| ENG-05 | ✓ SATISFIED (all three backends, Postgres leg now confirmed by live CI evidence) | Plans 22-12/22-13/22-14; 67 SQLite/InMemory tests + independently-confirmed Postgres CI run. |
| ENG-06 | ✓ SATISFIED | Unchanged; not touched by gap-closure plans. |
| ENG-07 | ✓ SATISFIED | Unchanged; not touched by gap-closure plans. |
| ENG-08 | ✓ SATISFIED | Unchanged; not touched by gap-closure plans. |

REQUIREMENTS.md's traceability matrix (lines 351-358) lists all eight as "Complete" under Phase 22, matching this verification. No orphaned requirements: the union of `requirements:` fields across all 14 plans (01-11, 12-17) is exactly ENG-01..08.

## New Finding From Fresh Code Review (22-REVIEW.md, post-checkpoint)

`22-REVIEW.md` was generated after the 22-17 gap-closure checkpoint closed (reviewed
`2026-09-02T22:22:39Z`, ~40 minutes after 22-17's completion) and reports 1 Critical, 3 Warning, 2
Info findings. It is **not** part of the UAT-driven gap-closure cycle this re-verification is
scoped to, and none of its findings are mentioned in `22-UAT.md` or routed in `ROADMAP.md`'s Phase
22.1 scope (which covers only the readiness defect and MSRV, per that section's own text).

I independently verified the one Critical finding (CR-01) by reading source directly rather than
citing the review:

- **CR-01 (confirmed real):** `WarGraph::fingerprint()` (`graph.rs:398-437`) hashes node ids, edge
  specs, and schema field names — never `self.defer_flags` or `self.dynamic_targets`. But
  `WarEngine::resume`/`resume_with_options` (`mod.rs:448-461`) documents that this fingerprint check
  is what makes "a divergence between fresh and resumed execution... structurally impossible."
  `superstep::compute_next_vanguard` reads `graph.is_deferred(...)` live from whatever `WarGraph`
  instance is passed to `resume` — not from anything persisted in the Waypoint — so a graph with the
  same nodes/edges/schema but a changed `defer_flags` set passes the fingerprint check unmodified and
  silently produces different scheduling behavior on resume than the interrupted run would have
  produced. No existing test (`fingerprint_is_deterministic_across_calls`,
  `fingerprint_is_unchanged_by_insertion_order`) exercises a defer-flag or dynamic-target difference.
  Surfaced as the sole item in Human Verification below.

The three Warnings (`WR-01`: `InMemoryWaypointStore::list_threads` picks "latest" by `Vec` position
rather than time, inconsistent with `latest()`/`history()` and both SQL backends' window-function
queries; `WR-02`: the local `make test-integration-docker` Postgres leg lacks the readiness-wait/SKIP
guards the CI job it mirrors requires; `WR-03`: `RecursionLimitExceeded`'s `failed_node` attribution
is arbitrary among co-participants) and two Info items (`IN-01`: retention config's
`apply_env_overrides` doesn't re-run `validate()`; `IN-02`: `with_parallelism(0)` silently becomes
`1`) are lower-severity, do not bear on this phase's core crash-resume/checkpoint claims as directly
as CR-01 does, and are not elevated to a blocking human-verification item here — they are noted for
awareness and, per the review's own framing, are advisory.

## Anti-Pattern / Prohibition Scan (gap-closure-touched files)

| Check | Result |
|---|---|
| `TBD`/`FIXME`/`XXX` in files this gap-closure cycle modified (`retention.rs`, `postgres.rs`, `waypoint_port.rs`, `graph.rs`, `mod.rs`, `superstep.rs`, `waypoint_retention.rs`) | 0 hits |
| Structural anti-interpolation (SQL built by `format!` with caller-supplied ids) | 0 hits — `sqlx::QueryBuilder` (SQLite) and array-bind `<> ALL($n::text[])` (Postgres) used throughout `prune_thread` |
| `delete_thread` called from `retention.rs`'s `prune` | 0 hits — confirmed by direct read; the whole-thread delete-then-resave sequence is gone |
| `#[ignore]`d test count in `engine::lib` matches the one disclosed readiness-defect reproduction | 1 (matches; not a silently-skipped unrelated test) |

## Non-Functional Claims — Decision Recorded

| Claim | Status |
|---|---|
| ENG-NFR-01: SQLite Waypoint save p50 miss (73.09 ms vs 10 ms target) | Decision recorded via `22-UAT.md` test 4: `result: pass` — accepted as measured baseline for v0.10.0. No further action required by this verification. |

## Deferred / Disclosed Items (not gaps)

- **Frontier readiness defect (self-loop + upstream edge blocks first execution).** Distinct from
  BUG-02/G-22-3 (which was strandedness, now fixed). Reproduced by an `#[ignore]`d test, confirmed
  present and correctly asserting the *should-be* behavior (not inverted to pin the bug). Explicitly
  registered and routed to inserted Phase 22.1 per the 22-17 checkpoint decision. Not a Phase 22 gap.
- **MSRV 1.85 vs rmcp-pinned process-wrap conflict.** The `msrv` CI job is red on exactly the one
  package this conflict touches; routed to Phase 22.1 per the same checkpoint decision. Not a Phase
  22 gap — this phase's own `semver`/`msrv` job scaffolding (ENG-08) exists and is correctly wired;
  the failure is a pre-existing dependency-pin conflict the checkpoint explicitly declined to solve
  in-repo.
- **CR-01 fingerprint gap (this pass's new finding).** See Human Verification — disclosed, not
  silently absorbed, but also not unilaterally classified as a blocking gap by this verifier, since
  it does not violate the literal text of ENG-FR-14 or any must_have truth this phase declared, and a
  human developer (not this verifier) should decide whether it is a release blocker for v0.10.0 or a
  Phase 22.1/23 follow-up.

## Human Verification Required

See frontmatter `human_verification` for the one new item (CR-01, the fingerprint/defer_flags gap).
Everything the prior verification round flagged for human decision (Postgres live-server run,
retention atomicity, stranded-node validation, ENG-NFR-01) is now closed with recorded, verifiable
evidence or an explicit accept decision — none of those four remain open.

## Gaps Summary

No must_haves truth failed as literally tested, in either the original 11 plans or the 6 gap-closure
plans. All three UAT gaps (G-22-1 major, G-22-2 blocker, G-22-3 major) are genuinely resolved with
evidence independently re-confirmed in this pass — G-22-1's CI-run evidence was re-fetched from the
CI provider directly rather than trusted from SUMMARY text, which is the exact category of claim this
gap was originally about. The phase's status is `human_needed` rather than `passed` because a fresh,
post-checkpoint code review surfaced one new Critical finding (CR-01) bearing on the crash-resume
fingerprint guarantee that has not yet been triaged through UAT or routed to a follow-up phase, and
this verifier — per its role — surfaces it for a human decision rather than either silently passing
the phase or unilaterally declaring it a blocking gap.

---

_Verified: 2026-09-02T22:45:00Z_
_Verifier: Claude (gsd-verifier)_
