# Phase 27 Platform API — CI Evidence Record (gap-closure plan 27-25)

**Phase:** 27-platform-api
**Branch:** `feature/phase-26`
**HEAD SHA at time of writing:** `a1dbe74eccfd605ee931613d1967e9228d89b967`
**Written:** 2026-09-08

This record closes the two `human_verification` items `27-VERIFICATION.md` left open. It has two
halves: a **Local sweep** (everything this devcontainer can prove without Docker/Java, all run and
recorded below, all green) and a **Tier-2 / CI evidence** table whose result column is deliberately
left empty by this task — filling it is the blocking human-verify checkpoint's job (Task 2 of
`27-25-PLAN.md`), not this one's, so the bar cannot be lowered after seeing the results.

---

## Local sweep

Every command below was run in this worktree against the merged output of all six gap-closure
plans (27-19 … 27-24). Each result line is quoted verbatim from the actual run.

| # | Command | Result (verbatim) | Verdict |
|---|---------|--------------------|---------|
| 1 | `cargo test --workspace --lib --bins` | 13 binaries, all `test result: ok`, 0 failed. Aggregate: **3398 passed; 0 failed** across `paladin` (918), `paladin-server` bin (14), `paladin_core` (581), `paladin_battalion` (758), `paladin_content` (96), `paladin_doc_examples` (1), `paladin_herald` (43), `paladin_llm` (166), `paladin_memory` (133), `paladin_notifications` (0 — crate has no `#[cfg(test)]` module), `paladin_ports` (179), `paladin_storage` (287), `paladin_web` (222). | ✅ PASS |
| 2 | `cargo test --workspace --doc` | 12 doc-test crates. Aggregate: **442 passed; 0 failed** — `paladin` 145, `paladin_core` 91, `paladin_battalion` 55, `paladin_content` 0, `paladin_doc_examples` 0, `paladin_herald` 0 (6 ignored), `paladin_llm` 8, `paladin_memory` 12, `paladin_notifications` 0, `paladin_ports` 131, `paladin_storage` 0, `paladin_web` 0. (Run separately from #1 per the doc-tests-don't-count-toward-llvm-cov trap — 27-VALIDATION.md.) | ✅ PASS |
| 3 | `cargo test --features web-server --test e2e_platform_api` | `test e2e_platform_api_acceptance_1_full_lifecycle ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.83s` | ✅ PASS |
| 4 | `cargo fmt --check` | Exit 0, no output. | ✅ PASS |
| 5 | `cargo clippy --workspace --all-targets -- -D warnings` | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 1m 57s` — exit 0, zero warnings. | ✅ PASS |
| 6 | `make security` | `cargo-audit`: "warning: 10 allowed warnings found" (unmaintained/unsound/yanked notices on `dotenv`, `fxhash`, `number_prefix`, `paste`, `rustls-pemfile`, `smartstring`, `event-listener`, `scc`, `chacha20`, `spin` — none are vulnerability advisories, all pre-existing per `security.instructions.md`). `cargo-deny check`: `advisories ok, bans ok, licenses ok, sources ok`. Overall exit 0. | ✅ PASS |
| 7 | `./scripts/check-api-surface.sh .project/current-exports.txt` | `✅ API surface extracted to /tmp/tmp.sDoE3vJxxX (3763 items)` / `✅ API surface unchanged` — exit 0. | ✅ PASS |
| 8 | `cargo test -p paladin-storage --features redis-queue --lib claim_marker` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 167 filtered out` (`claim_marker_gates_the_attempt_increment_in_the_claim_script`, `claim_marker_is_cleared_by_the_nack_script`, `claim_marker_is_ignored_by_queued_run_deserialization`) — 27-19's Tier-1 guard tests. | ✅ PASS |
| 9 | `cargo test -p paladin-storage --features sqlite --lib run::` | `test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 211 filtered out` — includes `storage_timestamp_truncates_sub_microsecond_digits_toward_zero`, `storage_timestamp_is_identity_at_microsecond_resolution`, `insert_then_get_round_trips_every_field`, `update_status_queued_to_running_sets_started_at_then_stale_cas_fails`, `update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing` (SQLite side of 27-20's timestamp-precision contract). | ✅ PASS |
| 10 | `python3 scripts/sdk-smoke/mock-llm.py --self-test` | `mock-llm: POST /v1/chat/completions -> handled` / `GET /v1/models -> handled` / `GET /v1/nonexistent -> handled` / `self-test: ok` | ✅ PASS |
| 11 | `python3 scripts/sdk-smoke/smoke.py --self-test` | 5 expected `SMOKE FAIL` lines (one per non-`completed` terminal status under test) then `self-test: ok (6/6 cases)` | ✅ PASS |
| 12 | `cargo test -p paladin-ai --lib bounded_body_` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 897 filtered out` (`bounded_body_at_exactly_the_cap_is_not_truncated`, `bounded_body_stops_at_the_cap`, `bounded_body_returns_a_small_body_whole` — 27-22 CR-01) | ✅ PASS |
| 13 | `cargo test -p paladin-ai --lib lease_heartbeat_with_a_zero_lease` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 899 filtered out` (`lease_heartbeat_with_a_zero_lease_never_extends` — 27-23 WR-04) | ✅ PASS |
| 14 | `python3 scripts/normalize-api-bounds.py --self-test` | `ok` | ✅ PASS |

**Local sweep verdict: 14/14 green.** Every command above selected at least one test/check and none took a self-skip path (D-51's zero-selected-tests trap does not apply — every filter above matched a non-zero, named set).

**Not run locally, by design (D-51 — Docker/Java unavailable in this devcontainer):** live Redis (`redis-queue` CI job), live Postgres (`postgres-integration` CI job), the `coverage` job's `cargo llvm-cov --workspace --features integration-tests,llm-all --fail-under-lines 82` invocation (self-skips/hard-fails without Redis+MinIO reachable), and the real `openapi-generator-cli` run inside `sdk-clients` (needs Java/Docker). These four map directly to four of the nine rows in the Tier-2/CI evidence table below.

---

## Tier-2 / CI evidence

This table is deliberately pre-filled with the job name, the gap it closes, and the exact log
string or numeric threshold that counts as proof — **before** the CI run happens, per this plan's
`must_haves.prohibitions` ("the bar being lowered after seeing results" — T-27-25-03). The
**Result** column is left empty; Task 2's checkpoint fills it after a live CI run at a SHA
containing all six gap-closure commits.

| Job (`.github/workflows/ci.yml` display name) | Gap it closes | Required proof (exact log string / threshold) | Result |
|---|---|---|---|
| `Redis Run Queue Contract Suite (live server)` | Gap 1 (PLAT-02) — Redis attempt off-by-one | 0 failed, 16 clauses run; log line reporting the suite exercised the **live server** (a self-skip printing `SKIP: ... not reachable` is NOT a pass — D-51) | |
| `Postgres Storage Contract Suites (live server)` | Gap 2 (PLAT-01) — Postgres timestamp precision | 0 failed; declared-vs-selected counts equal across `waypoint`, `run`, `assistant`, `run_schedule`, `webhook`; in particular `insert_then_get_round_trips_every_field`, `update_status_queued_to_running_sets_started_at_then_stale_cas_fails`, `update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing`, and the new `postgres_run_timestamps_round_trip_at_microsecond_precision` all pass | |
| `Coverage` | Gap 1's `impact_note` — coverage previously aborted at exit 101 before any percentage was computed | Completes with **no exit 101**; reports **>= 82%** workspace line coverage (ADR-0006 floor). Record the exact percentage. | |
| `Generated SDK Clients (Python + TypeScript) smoke` | Gap 3 (PLAT-06) — sdk-clients job failure | Both "generated N files" lines show N >= 20; both smokes print a `run_id`; both report the terminal status **exactly `completed`**; TypeScript half actually runs (no `npm ci` lockfile error) | |
| `API Surface Tracking` | Gap 4 (PLAT-06/X-10.1) — api-surface toolchain drift | Log contains `API surface unchanged` | |
| `e2e-platform-api` | Gap 5 (PLAT-06/X-02) — `e2e_platform_api` never run by any CI job | Green; passing test-result line showing a **non-zero** count (e.g. `test result: ok. 1 passed`) | |
| `Unit Tests` (`test` job) | Regression guard — nothing in the gap-closure set broke Tier-1 | Green | |
| `MSRV (Rust 1.88)` | Regression guard — gap-closure code compiles on the pinned MSRV | Green | |
| `Semver Checks (vs v0.9.0)` | Regression guard — no undeclared breaking change from the gap-closure set | Green | |

---

## Baseline

**CI run `34222317640`** (push, `feature/phase-26` @ SHA `1e81939a`, conclusion **failure**) —
the run this gap-closure set (plans 27-19 … 27-24) is measured against.

**Failing jobs in run `34222317640`:**

| Job | Result at baseline |
|---|---|
| `Redis Run Queue Contract Suite (live server)` | 13 passed / **3 failed** — `redis_run_queue_full_contract_suite_via_run_all`, `redis_run_queue_lease_expiry_redelivers_with_attempt_incremented`, `redis_run_queue_nack_requeues_after_delay_with_attempt_incremented`, all on `contract_tests.rs:105`'s `leased.queued.attempt == 1` assertion (observed `2`) |
| `Postgres Storage Contract Suites (live server)` | 83 passed / **3 failed** — `run::postgres::tests::insert_then_get_round_trips_every_field`, `update_status_queued_to_running_sets_started_at_then_stale_cas_fails`, `update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing`, all on a `DateTime<Utc>` exact-equality assertion losing sub-microsecond precision on Postgres `TIMESTAMPTZ` round-trip |
| `Coverage` | **exit 101**, no percentage computed — same 3 Redis assertions fail inside `cargo llvm-cov --workspace --features integration-tests,llm-all` (`paladin-storage`: 279 passed; 3 failed) before any line-coverage number is produced |
| `Generated SDK Clients (Python + TypeScript) smoke` | **exit 1** — TypeScript half never runs (`npm ci` needs a missing `package-lock.json`); Python half silently "passes" despite the submitted run reaching `failed` (real, unauthenticated OpenAI provider; `poll_until_terminal` accepted any terminal status as success) |
| `API Surface Tracking` | **FAILED** — `.project/current-exports.txt` diff confined to `RunWorkerPool<W>` auto-trait (`Send`/`Sync`/`Unpin`/`Freeze`/`UnsafeUnpin`) bound ordering, a toolchain-cosmetic difference between the local extraction toolchain and CI's pinned one, not a real API change |
| `Integration Tests` | 279 passed / **3 failed** — the *same* Redis attempt off-by-one as the `redis-queue` job (`contract_tests.rs:105`, `left: 2 right: 1`), confirming one root cause fails two independent jobs |

**Not run by any job at baseline:** `e2e_platform_api` (PRD 06 acceptance-1 lifecycle test) — no
`test`/`integration`/`feature-flags` job at baseline supplied `--features web-server --test
e2e_platform_api`; the test passed locally (`test result: ok. 1 passed`) but was never
CI-wired prior to plan 27-24.

**Expected delta after this gap-closure set's SHA runs CI:** all six rows above (five failing
jobs plus the Integration Tests duplicate) flip green, and `e2e-platform-api` (new job, added by
27-24) runs and passes for the first time in CI. That is exactly what the Tier-2/CI evidence
table above is structured to confirm, row by row, once a live run exists.
