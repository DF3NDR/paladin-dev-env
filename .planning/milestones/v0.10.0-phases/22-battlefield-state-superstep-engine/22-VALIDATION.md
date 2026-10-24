---
phase: 22
slug: battlefield-state-superstep-engine
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-01
validated: 2026-10-24
reconciled: "2026-10-24 by Phase 46 plan 46-02, reconcile-only (D-14/D-16): map rebuilt from the archived PLAN/SUMMARY/VERIFICATION; no test written"
---

# Phase 22 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

Reconciled 2026-10-24 by Phase 46 plan 46-02 in **reconcile-only** mode on HEAD `8b405f00`
(branch `claude/laughing-dirac-e0h2ax`): the per-task map below was rebuilt mechanically from the 44
`<task>` elements of `22-01..22-17-PLAN.md`, then each row was checked against `22-VERIFICATION.md`
(status `passed`, 2026-09-02), the plan SUMMARYs and `22-UAT.md`. The automated commands marked
"re-run" were executed in this session and passed; rows marked "seal" or "CI evidence" cite the
phase's own close-out record because the command needs Docker, `cargo-llvm-cov` or a network. No test
or implementation file was written or changed (D-14): the Nyquist auditor's gap-filling half was
deliberately not run.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in), `#[tokio::test]` / `#[tokio::test(flavor = "multi_thread")]`, `criterion` for benches (`benches/engine_benchmarks.rs`) |
| **Config file** | none dedicated — workspace `Cargo.toml` + per-crate `[dev-dependencies]`; root `[[test]]` targets `war_engine_tracer`, `e2e_crash_resume`, `golden_bridge_equivalence`, `waypoint_retention_fault_injection` |
| **Quick run command** | `cargo test -p paladin-ai-core --lib` / `cargo test -p paladin-battalion --lib` / `cargo test -p paladin-storage --features sqlite --lib` (Tier 1, no services; pick the crate the task touched). The core crate's package name on HEAD is `paladin-ai-core` (directory `crates/paladin-core`) |
| **Full suite command** | `make test-all` (unit + integration); `make test-integration-docker` for Tier 2 Postgres contract tests (also CI's `postgres-integration` job) |
| **Estimated runtime** | ~2 s per crate warm for the re-run set below (cold builds dominate); minutes for full workspace + Docker tier |

---

## Sampling Rate

- **After every task commit:** Run the touched crate's quick command (`cargo test -p <crate> --lib`), plus `cargo fmt --check` and `cargo clippy -- -D warnings`
- **After every plan wave:** Run `cargo test` (full workspace) + `cargo test --doc`
- **Before `/gsd-verify-work`:** Full suite green — `make test-all`, plus `make test-integration-docker` for the Postgres contract suite, plus the `semver` and `msrv` CI jobs green
- **Max feedback latency:** ~180 seconds

---

## Per-Task Verification Map

Test type legend: **unit** = in-crate `#[cfg(test)]`; **integ** = `tests/` binary; **contract** = shared `WaypointPort` suite (Tier 1 InMemory/SQLite, Tier 2 Postgres); **gate** = offline file/tool check; **checkpoint** = human decision or verification gate at plan time (not a test). Threat Ref is the plan's `T-22-NN` register range, closed in `22-SECURITY.md`. Status legend: **✅ green** (re-run on HEAD this session, or "seal" = green at the phase's own close-out and still resolving on HEAD, or "CI evidence"); **⚠ superseded** (a later phase changed the target; the HEAD equivalent is named); **✅ decided / checkpoint resolved** (a plan-time gate, cited to its SUMMARY).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 22-01-01 | 01 | 1 | ENG-01, ENG-02, ENG-03, ENG-04, ENG-05 | T-22-01..03 | N/A (decision: `GraphFingerprint` encoding, D-04 one-way; resolved option-b `v1:{blake3_hex}` per 22-01-SUMMARY) | checkpoint | — | — | ✅ decided |
| 22-01-02 | 01 | 1 | ENG-01, ENG-02, ENG-03, ENG-04, ENG-05 | T-22-01..03 | `Strict` durability: a `WaypointPort::save` failure fails the run (`EngineError::WaypointWrite`); `ThreadNotFound`; `GraphMismatch` | tracer: integ + unit | `cargo test --test war_engine_tracer && cargo test -p paladin-ai-core --lib battlefield && cargo test -p paladin-ai-core --lib waypoint && cargo test -p paladin-battalion --lib engine && cargo test -p paladin-storage --lib waypoint` | ✅ | ✅ green (3 + 43 + 40 + 604 + 48; `cargo fmt --check` clean) |
| 22-02-01 | 02 | 2 | ENG-01 | T-22-04..06 | Schema enforced hard: unknown field / wrong type is a typed error | unit + doc | `cargo test -p paladin-ai-core --lib battlefield && cargo test --doc -p paladin-ai-core` | ✅ | ✅ green (43 + 150 doc) |
| 22-02-02 | 02 | 2 | ENG-01 | T-22-04..06 | Deterministic multi-writer merge across all five dispatch rules | unit + doc | `cargo test -p paladin-ai-core --lib battlefield && cargo test --doc -p paladin-ai-core` | ✅ | ✅ green (43 + 150 doc) |
| 22-03-01 | 03 | 2 | ENG-03, ENG-05 | T-22-07..09 | `ThreadId` validated at the port boundary; not-found is typed | unit + doc | `cargo test -p paladin-ai-core --lib waypoint && cargo test -p paladin-ports --lib waypoint_port && cargo test --doc -p paladin-ai-core -p paladin-ports` | ✅ | ✅ green (40 + 6 + 150 + 230 doc) |
| 22-03-02 | 03 | 2 | ENG-03, ENG-05 | T-22-07..09 | One shared generic contract suite, run against `InMemoryWaypointStore` | contract | `cargo test -p paladin-storage --lib waypoint` | ✅ | ✅ green (48) |
| 22-04-01 | 04 | 2 | ENG-08 | T-22-10..12 | `MIGRATION.md` §9.1-9.8 present; M-B-01..03 pre-populated; no unowned `TBD` (replaces the seeded Manual-Only row) | gate | `test -f MIGRATION.md && [ "$(grep -c '^## 9\.[1-8] ' MIGRATION.md)" = "8" ] && grep -q 'M-B-01' MIGRATION.md && grep -q 'M-B-03' MIGRATION.md && grep -q 'WaypointRetentionConfig' MIGRATION.md && [ "$(grep -n 'TBD' MIGRATION.md \| grep -vE '(ENG\|CF\|HITL\|FT\|RT\|PLAT\|OBS\|SHIP)-' \| wc -l)" = "0" ] && grep -q 'MIGRATION.md' README.md` | ✅ | ✅ green (re-run: 8 headings, 0 unowned `TBD`) |
| 22-04-02 | 04 | 2 | ENG-08 | T-22-10..12 | MSRV declared once and enforced by a CI job | gate | `grep -n '^rust-version' Cargo.toml && cargo metadata --format-version 1 --no-deps \| grep -o '"rust_version":"1.88"' \| wc -l && grep -q '^  msrv:' .github/workflows/ci.yml` | ✅ | ⚠ superseded: Phase 22.1 (22.1-VERIFICATION truth 13) moved the floor 1.85 → 1.88 as one `workspace.package.rust-version`; HEAD equivalent re-run: 12 packages report `1.88`, `msrv` job at `ci.yml:287`; literal `= "1.85"` count is 0 |
| 22-04-03 | 04 | 2 | ENG-08 | T-22-10..12 | `semver` job pinned to the published 0.9.0 baseline; allowlist has no wildcard entries | gate | `test -f .cargo/semver-checks-allowlist.toml && grep -q '^  semver:' .github/workflows/ci.yml && grep -q 'paladin-ai-core' .github/workflows/ci.yml && grep -q '0.9.0' .github/workflows/ci.yml && [ "$(grep -nE '^\s*(ignore\|allow)\s*=\s*\["?\*' .cargo/semver-checks-allowlist.toml \| wc -l)" = "0" ]` | ✅ | ✅ green (grep legs re-run; `actionlint` and `cargo semver-checks` are CI evidence: `ci.yml` `semver` job at line 339) |
| 22-05-01 | 05 | 3 | ENG-02 | T-22-13..16 | Cycles permitted; construction validated | unit + integ | `cargo test -p paladin-battalion --lib engine && cargo test --test war_engine_tracer` | ✅ | ✅ green (604 + 3) |
| 22-05-02 | 05 | 3 | ENG-02 | T-22-13..16 | Snapshot isolation per superstep; exactly one Waypoint per superstep | unit + integ | `cargo test -p paladin-battalion --lib engine && cargo test --test war_engine_tracer` | ✅ | ✅ green (604 + 3) |
| 22-05-03 | 05 | 3 | ENG-02 | T-22-13..16 | Bounded iteration: `max_supersteps` / `max_node_visits` produce a `Failed` Waypoint, never a hang | unit + integ | `cargo test -p paladin-battalion --lib engine && cargo test --test war_engine_tracer && cargo fmt --check` | ✅ | ✅ green (604 + 3; fmt clean) |
| 22-06-01 | 06 | 3 | ENG-05 | T-22-17..21 | Statements parameterised; versioned migration with the thread/created index | contract (Tier 1 SQLite) | `cargo test -p paladin-storage --features sqlite --lib waypoint && grep -q 'idx_waypoints_thread_created' crates/paladin-storage/migrations/sqlite/001_create_waypoints_table.sql` | ✅ | ✅ green (91; index present) |
| 22-06-02 | 06 | 3 | ENG-05 | T-22-17..21 | Postgres adapter behind the `postgres` feature; Docker-gated suite self-skips without a server | contract (Tier 2 Postgres) | `cargo test -p paladin-storage --features postgres --lib waypoint::postgres` | ✅ | ✅ green (37, Docker-less skip path re-run here; live-server leg is CI evidence: `postgres-integration` job, run 33688238662 then 33706948313, 22.1-CI-EVIDENCE.md) |
| 22-06-03 | 06 | 3 | ENG-05 | T-22-17..21 | `WaypointRetentionConfig` validated; retention routine green on SQLite | unit + contract | `cargo test -p paladin-ai --lib config::waypoint_retention && cargo test -p paladin-storage --features sqlite --lib waypoint` | ✅ | ✅ green (7 + 91) |
| 22-07-01 | 07 | 4 | ENG-01, ENG-02 | T-22-22..25 | Join / defer / not-firing frontier semantics never strand a join | unit + integ | `cargo test -p paladin-battalion --lib engine && cargo test --test war_engine_tracer` | ✅ | ✅ green (604 + 3) |
| 22-07-02 | 07 | 4 | ENG-01, ENG-02 | T-22-22..25 | Custom dispatch registry is engine-owned, never in `paladin-core` (X-01) | unit + gate | `cargo test -p paladin-battalion --lib engine && [ "$(grep -rnw 'DispatchRegistry' crates/paladin-core/src/ \| wc -l)" = "0" ]` | ✅ | ✅ green (604; the literal `grep -rn` returns 3 `CustomDispatchRegistry` substring hits, so the leg is recalibrated to `-w` as 22-07-SUMMARY recorded; `-w` count 0) |
| 22-07-03 | 07 | 4 | ENG-01, ENG-02 | T-22-22..25 | Byte-identical determinism under 20 seeded randomized schedules and a 100-iteration multi-thread stress | unit + multi_thread | `cargo test -p paladin-battalion --lib engine && cargo test --test war_engine_tracer && cargo fmt --check` | ✅ | ✅ green (604 + 3; fmt clean) |
| 22-08-01 | 08 | 5 | ENG-03, ENG-04 | T-22-26..29 | Paladin nodes; `InputMapping` template resolution; token accounting | unit + doc | `cargo test -p paladin-battalion --lib engine && cargo test --doc -p paladin-battalion` | ✅ | ✅ green (604 + 61 doc) |
| 22-08-02 | 08 | 5 | ENG-03, ENG-04 | T-22-26..29 | Resume restores state, vanguard and visit counts; fingerprint mismatch refuses resume | unit + integ | `cargo test -p paladin-battalion --lib engine && cargo test --test war_engine_tracer` | ✅ | ✅ green (604 + 3) |
| 22-08-03 | 08 | 5 | ENG-03, ENG-04 | T-22-26..29 | Crash-resume: zero re-execution; final Battlefield equals the control run | integration (E2E-1) | `cargo test --test e2e_crash_resume && cargo test --test war_engine_tracer && cargo test -p paladin-battalion --lib` | ✅ | ✅ green (32 + 3 + 892) |
| 22-09-01 | 09 | 6 | ENG-07 | T-22-30..34 | `TraceSink` dispatcher is bounded, drop-oldest, counted-drop: tracing never stalls a run | unit + doc | `cargo test -p paladin-ports --lib && cargo test -p paladin-battalion --lib engine && cargo test --doc -p paladin-ports` | ✅ | ✅ green (299 + 604 + 230 doc) |
| 22-09-02 | 09 | 6 | ENG-07 | T-22-30..34 | `NodeInterceptor` chain ordered and empty by default | unit + integ | `cargo test -p paladin-battalion --lib engine && cargo test --test e2e_crash_resume` | ✅ | ✅ green (604 + 32) |
| 22-09-03 | 09 | 6 | ENG-07 | T-22-30..34 | Cancellation finishes the in-flight superstep into a `Halted` Waypoint | unit + integ | `cargo test -p paladin-battalion --lib engine && cargo test --test e2e_crash_resume && cargo test --test war_engine_tracer && cargo fmt --check` | ✅ | ✅ green (604 + 32 + 3; fmt clean) |
| 22-10-01 | 10 | 6 | ENG-02, ENG-05 | T-22-35..36 | ENG-NFR-01 SQLite save p50 measured honestly: 73.09 ms vs the 10 ms target, miss accepted as baseline (22-UAT test 4 `result: pass`) | bench + sign-off | `cargo bench --bench engine_benchmarks -- --test` | ✅ | ✅ green (re-run exit 0; the miss itself is signed off in 22-UAT test 4) |
| 22-10-02 | 10 | 6 | ENG-02, ENG-05 | T-22-35..36 | ENG-NFR-02 memory per superstep measured; `Battlefield` clone bound enforced | bench + example | `cargo bench --bench engine_benchmarks -- --test && cargo run --release --example war_engine_memory_baseline` | ✅ | ✅ green (re-run exit 0; `battlefield_clone_bound_check=PASS`, observed 1 <= ceiling 5) |
| 22-11-01 | 11 | 7 | ENG-06 | T-22-37..39 | `from_formation` / `from_phalanx` / `from_campaign` bridges; legacy services untouched by this plan | unit + gate | `cargo test -p paladin-battalion --lib` | ✅ | ✅ green (892 re-run; the `git diff --stat` leg is a plan-time invariant, green at seal per 22-11-SUMMARY) |
| 22-11-02 | 11 | 7 | ENG-06 | T-22-37..39 | Bridge output equals the legacy services on golden inputs | integration (golden) | `cargo test --test golden_bridge_equivalence && cargo test --test e2e_crash_resume && cargo test --test war_engine_tracer` | ✅ | ✅ green (36 + 32 + 3) |
| 22-11-03 | 11 | 7 | ENG-06 | T-22-37..39 | Phase coverage gate on the new modules | CI gate | `bash scripts/coverage.sh` | ✅ | ✅ green (seal: 22-11-SUMMARY recorded exit 0 under `cargo-llvm-cov` 0.8.7; CI evidence: the `coverage` job, `ci.yml:1507`, now the single ADR-0006 82% workspace line gate; not re-run, no `cargo-llvm-cov` in the sandbox) |
| 22-12-01 | 12 | 1 | ENG-05 | T-22-40..42 | `postgres-integration` job: healthy-wait, `pg_isready`, single-threaded run, SKIP and short-count guards, no `needs` edge | gate (YAML structure) | `python3 - <<'PY' (yaml.safe_load of .github/workflows/ci.yml: job present; needles postgres-test, WAYPOINT_POSTGRES_TEST_URL, pg_isready, waypoint::postgres, --test-threads=1, --nocapture, grep -qx healthy, tokio::test, test result: ok; no needs; failure-detection steps)` | ✅ | ✅ green (re-run: all 9 needles present, no `needs`, 3 guard steps; live execution is CI evidence, runs 33688238662 / 33706948313) |
| 22-12-02 | 12 | 1 | ENG-05 | T-22-40..42 | Exactly one CI job added; Makefile runner uses the same flags | gate | `grep -c -- '--test-threads=1' Makefile` | ✅ | ⚠ superseded: the diff-against-pre-22-12 `ci.yml` leg is a point-in-time comparison and cannot be re-evaluated on HEAD (later phases add jobs); equivalent is 22-12-01 (re-run); Makefile leg re-run: count 2 >= 1 |
| 22-13-01 | 13 | 1 | ENG-05 | T-22-43..46 | `delete_waypoint` / `prune_thread` added to `WaypointPort` contract-first | contract | `cargo test -p paladin-storage --lib waypoint::in_memory && cargo test -p paladin-ports --lib waypoint_port` | ✅ | ✅ green (35 + 6) |
| 22-13-02 | 13 | 1 | ENG-05 | T-22-43..46 | Transactional `prune_thread` overrides; keep-set bound as parameters | contract (SQLite, Postgres) | `cargo test -p paladin-storage --features sqlite --lib waypoint::sqlite && cargo test -p paladin-storage --features postgres --lib waypoint::postgres` | ✅ | ✅ green (39 + 37 Docker-less skip path; live Postgres leg is CI evidence) |
| 22-13-03 | 13 | 1 | ENG-05 | T-22-43..46 | No SQL statement built by interpolation in either backend (T-22-43..46) | gate | `for f in crates/paladin-storage/src/waypoint/{sqlite,postgres}.rs; do sed -n '1,/^#\[cfg(test)\]/p' "$f" \| grep -v '^\s*//' \| grep -vi 'format!("waypoint\\|format!("unsupported' \| grep -Ei 'format!\("[^"]*(select\|insert\|update\|delete)' && exit 1; done; cargo fmt --check` | ✅ | ✅ green (re-run: no interpolated statements; fmt clean; `cargo test --workspace` and clippy are CI evidence, run 33706948313) |
| 22-14-01 | 14 | 2 | ENG-05 | T-22-47..50 | `prune` rewritten on the keep-set primitive; protected set moved to the application layer | unit + gate | `cargo test -p paladin-storage --features sqlite --lib waypoint::retention && cargo test -p paladin-ai --lib application::services::waypoint_retention && grep -c monotone .project/v0.10.0/01-battlefield-state-and-execution-engine.md` | ✅ | ✅ green (12 + 7; `monotone` count 1; `cargo build --workspace --all-features` is CI evidence) |
| 22-14-02 | 14 | 2 | ENG-05 | T-22-47..50 | A crash or cancellation mid-prune never loses the keep-set | integration (fault injection) | `cargo test --test waypoint_retention_fault_injection` | ✅ | ✅ green (3) |
| 22-14-03 | 14 | 2 | ENG-05 | T-22-47..50 | Workspace green; `make security` clean; pre-release classification recorded | CI gate | `cargo test --workspace && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && make security` | ✅ | ✅ green (seal; `cargo fmt --check` re-run clean; CI evidence: run 33706948313 green end to end including Security Audit and License & Dependency Policy, 22.1-CI-EVIDENCE.md) |
| 22-15-01 | 15 | 1 | ENG-02 | T-22-51..54 | Regression tests for BUG-02 written and observed failing before the fix | tdd RED capture | `cargo test -p paladin-battalion --lib engine::graph (expected to FAIL at plan time)` | ✅ | ⚠ superseded: a RED-phase command fails by design and cannot pass on HEAD; its GREEN counterpart is 22-15-02 (re-run) |
| 22-15-02 | 15 | 1 | ENG-02 | T-22-51..54 | Eligible-set validation: a node outside reachable-from-entry plus `dynamic_target` is `EngineError::UnreachableNode` before any node runs | unit | `cargo test -p paladin-battalion --lib engine::graph` | ✅ | ✅ green (136) |
| 22-15-03 | 15 | 1 | ENG-02 | T-22-51..54 | Every fixture the check caught is corrected; workspace green | CI gate | `cargo test --workspace && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` | ✅ | ✅ green (seal; battalion lib 892 and `cargo fmt --check` re-run clean; CI evidence: run 33706948313) |
| 22-16-01 | 16 | 2 | ENG-02 | T-22-55..58 | Strandedness-adjacent fixtures audited and classified in `22-deferred-items.md` | gate | `python3 - <<'PY' (22-deferred-items.md names loop_gate, strandedness dodge, readiness dodge, unrelated, campaign)` | ✅ | ✅ green (re-run: all five markers present; `cargo test --workspace` is CI evidence) |
| 22-16-02 | 16 | 2 | ENG-02 | T-22-55..58 | Readiness defect recorded as a runnable `#[ignore]`d reproduction | tdd (readiness defect) | `cargo test -p paladin-battalion --lib engine::superstep -- --ignored` | ✅ | ⚠ superseded: Phase 22.1 fixed the defect (ENG-FR-06a starvation-release fallback, D-03); the reproduction is no longer ignored (the `--ignored` re-run selects 0 tests) and its HEAD equivalent `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn` passes inside the 604-test `engine` run |
| 22-16-03 | 16 | 2 | ENG-02 | T-22-55..58 | Pre-release classification confirmed from the repository (engine absent at v0.9.0) | gate | `git tag --list \| wc -l && ! git cat-file -e v0.9.0:crates/paladin-battalion/src/engine/mod.rs` | ✅ | ✅ green (re-run: 15 tags; `engine/mod.rs` absent at `v0.9.0`) |
| 22-17-01 | 17 | 3 | ENG-02, ENG-05 | T-22-59..60 | N/A (human-verify: CI execution evidence for G-22-1; readiness defect disposition) | checkpoint | — | — | ✅ checkpoint resolved (run 33688238662, re-captured green end to end as run 33706948313; readiness defect routed to and fixed in Phase 22.1) |

*Status: ✅ green · ⚠ superseded (HEAD equivalent named) · ✅ decided / checkpoint resolved · ❌ gap (none)*

**Requirement roll-up**

| Requirement | Tasks | Green | Superseded (with equivalent) | Checkpoints |
|---|---|---|---|---|
| ENG-01 | 7 | 6 | 0 | 1 |
| ENG-02 | 17 | 13 | 2 | 2 |
| ENG-03 | 7 | 6 | 0 | 1 |
| ENG-04 | 5 | 4 | 0 | 1 |
| ENG-05 | 18 | 15 | 1 | 2 |
| ENG-06 | 3 | 3 | 0 | 0 |
| ENG-07 | 3 | 3 | 0 | 0 |
| ENG-08 | 3 | 2 | 1 | 0 |

Every requirement id has at least one green automated row re-run on HEAD: ENG-01 (22-02-01/02), ENG-02
(22-05-01..03, 22-07-01..03), ENG-03 (22-08-02), ENG-04 (22-08-03), ENG-05 (22-03-02, 22-06-01), ENG-06
(22-11-02), ENG-07 (22-09-01..03), ENG-08 (22-04-01, 22-04-03).

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Every Wave 0 file the seeded list named exists on HEAD (paths updated where later phases moved them):

- [x] `crates/paladin-core/src/platform/container/battlefield.rs` + `#[cfg(test)] mod tests` — ENG-01
- [x] `crates/paladin-core/src/platform/container/waypoint.rs` + `#[cfg(test)] mod tests` — ENG-03 type shapes
- [x] `crates/paladin-ports/src/output/waypoint_port.rs` — ENG-03 port contract
- [x] `crates/paladin-storage/src/waypoint/contract_tests.rs` (shared generic test fns, D-09) — ENG-05
- [x] `crates/paladin-battalion/src/engine/mod.rs` + `#[cfg(test)] mod tests` — ENG-02
- [x] `tests/integration/e2e_crash_resume_test.rs` (root `[[test]] e2e_crash_resume`) — ENG-04 / E2E-1
- [x] `tests/integration/golden_bridge_equivalence_test.rs` (root `[[test]] golden_bridge_equivalence`) — ENG-06
- [x] `crates/paladin-storage/migrations/{sqlite,postgres}/001_create_waypoints_table.sql` — schema for ENG-05 (successor of the seeded `00X_create_waypoints_table.sql`)
- [x] `.github/workflows/ci.yml` `semver` (line 339) and `msrv` (line 287) jobs — ENG-08
- [x] Framework install: none — `cargo test` / `criterion` already fully set up

---

## Manual-Only Verifications

The seeded Manual-Only row (MIGRATION.md §9 skeleton completeness) is now provable by a command and is automated row 22-04-01 (`grep -c '^## 9\.[1-8] ' MIGRATION.md` prints 8). One policy sign-off remains, recorded as a cited artifact rather than an untested behaviour:

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ENG-NFR-01: SQLite Waypoint save p50 measured at 73.09 ms against a 10 ms target (row 22-10-01) | ENG-05 | The benchmark runs and is recorded; whether a miss against the target is acceptable is a maintainer decision, not a test outcome. | Signed off in `22-UAT.md` test 4 (`result: pass`, accepted as the measured baseline for v0.10.0). To re-measure: `cargo bench --bench engine_benchmarks`. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify, a cited sign-off artifact, or Wave 0 dependencies (41 automatable tasks; 3 plan-time checkpoints are not tests)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (22-01-01 and 22-17-01 are the only checkpoints, each adjacent to automated rows)
- [x] Wave 0 covers all MISSING references (none missing on HEAD)
- [x] No watch-mode flags
- [x] Feedback latency < 180s for the per-crate quick commands (warm)
- [x] `nyquist_compliant: true` set in frontmatter — every requirement id has a green automated row and no row is a gap

**Approval:** validated 2026-10-24 (reconcile-only, Phase 46 plan 46-02)

---

## Validation Audit 2026-10-24

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

- **Re-run evidence (HEAD `8b405f00`, all exit 0):** `paladin-ai-core --lib battlefield` 43, `--lib waypoint` 40, `--doc` 150; `paladin-ports --lib` 299, `--lib waypoint_port` 6, `--doc` 230; `paladin-battalion --lib` 892, `--lib engine` 604, `--lib engine::graph` 136, `--doc` 61; `paladin-storage --lib waypoint` 48, `--features sqlite --lib waypoint` 91 (`waypoint::sqlite` 39, `waypoint::in_memory` 35, `waypoint::retention` 12), `--features postgres --lib waypoint::postgres` 37 (Docker-less skip path); `paladin-ai --lib config::waypoint_retention` 7, `--lib application::services::waypoint_retention` 7; root targets `war_engine_tracer` 3, `e2e_crash_resume` 32, `golden_bridge_equivalence` 36, `waypoint_retention_fault_injection` 3; `cargo bench --bench engine_benchmarks -- --test` and `cargo run --release --example war_engine_memory_baseline` (clone bound PASS); `cargo fmt --check`; grep/python gates for rows 22-04-01..03, 22-07-02, 22-12-01, 22-13-03, 22-16-01, 22-16-03.
- **Superseded with a HEAD equivalent (not gaps):** 22-04-02 (MSRV floor 1.85 to 1.88, Phase 22.1), 22-12-02 (point-in-time CI job diff; equivalent 22-12-01), 22-15-01 (RED capture; equivalent 22-15-02), 22-16-02 (readiness reproduction fixed by Phase 22.1; its test now passes in the `engine` run).
- **Recalibrated:** 22-07-02's literal `grep -rn 'DispatchRegistry'` matches the `CustomDispatchRegistry` alias (3 hits); `-w` returns 0, as 22-07-SUMMARY recorded.
- **CI evidence, not re-run locally:** live Postgres Tier 2 (`postgres-integration`, runs 33688238662 and 33706948313), `actionlint`, `cargo semver-checks`, `cargo-llvm-cov` coverage, workspace `cargo test` / clippy / `make security` (run 33706948313 is green end to end).
- **Not spawned:** `gsd-nyquist-auditor` — there is no uncovered task for it to write a test for, and Phase 46 writes no tests (D-14).

---

## Gaps

None.
