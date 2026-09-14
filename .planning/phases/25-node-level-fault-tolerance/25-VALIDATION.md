---
phase: 25
slug: node-level-fault-tolerance
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-05
validated: 2026-09-06
---

# Phase 25 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in harness); `#[tokio::test]` for async, `#[tokio::test(flavor = "multi_thread")]` for concurrency, `#[tokio::test(start_paused = true)]` for deterministic-clock timing |
| **Config file** | none — Rust's built-in test harness needs no config file |
| **Quick run command** | `cargo test -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-llm -p paladin-storage` |
| **Full suite command** | `cargo test --workspace` plus the three named integration binaries (`cargo test --test e2e_muster_defer_order`, `--test e2e_compensation_chain`, `--test aegis_retry_stress`) |
| **Estimated runtime** | ~120 s quick (crate-scoped), ~360 s full workspace + integration |

**Integration-test caveat:** the repository's default `make test` runs `--lib --bins` only, so nothing
under `tests/integration/` is covered by it. Every plan that adds or edits an integration test names
its exact `cargo test --test <name>` command in its own verification block.

**Docker caveat:** the devcontainer has no Docker. The `paladin-storage` Redis-cache tier and the
existing Postgres Waypoint tier self-skip locally (the `waypoint/postgres.rs` SKIP precedent) and are
provable only in the Docker-gated CI job. Their evidence is recorded as CI-only and is never reported
as locally passed.

---

## Sampling Rate

- **After every task commit:** `cargo test -p <crate the task touched>` (scoped, fast)
- **After every plan wave:** `cargo test --workspace` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo fmt --check`
- **Before `/gsd-verify-work`:** full suite green, plus `make security`, `cargo semver-checks` vs 0.9.0, the MSRV check at 1.88, and coverage ≥ 82% (ADR-0006) — plan 25-14 Task 3
- **Max feedback latency:** 120 s (crate-scoped quick run)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 25-01-01 | 01 | 1 | FT-01, FT-02 | T-25-01 | Checkpoint — human confirms the public `NodeError`/`Aegis` naming before it ships | checkpoint | *(blocking `checkpoint:decision`, no automated command)* | n/a | ✅ resolved — auto-mode `proceed-as-locked` (D-06/D-07/D-09), recorded in 25-01-SUMMARY.md |
| 25-01-02 | 01 | 1 | FT-01, FT-02 | T-25-01/04/05 | Provider text redacted before bounding; failed-attempt delta discarded; interceptor decisions not retried | unit + integration | `cargo test -p paladin-ai-core transience && cargo test -p paladin-ai-core node_error && cargo test -p paladin-ai-core aegis && cargo test -p paladin-battalion transient_function_node_failure_is_retried_and_run_completes` | ✅ | ✅ green |
| 25-01-03 | 01 | 1 | FT-02 | T-25-02/03 | Backoff races the cancellation token; `max_attempts == 0` rejected | unit (paused clock) | `cargo test -p paladin-battalion backoff_sequence_is_exact_with_jitter_off && cargo test -p paladin-battalion backoff_wait_returns_early_when_the_run_is_cancelled` | ✅ | ✅ green |
| 25-02-01 | 02 | 2 | FT-01 | T-25-06/07 | Transience classified from typed fields only, never from a rendered message | unit | `cargo test -p paladin-ai-core paladin_error_transience_table && cargo test -p paladin-ports llm_error_transience_table` | ✅ | ✅ green |
| 25-02-02 | 02 | 2 | FT-01 | T-25-08/09 | Non-exhaustive marking registered with its allowlist entry in the same commit; breaker behaviour unchanged | build + unit | `cargo build --workspace --all-features --all-targets && cargo test -p paladin-ai-core battalion_error_node_carries_the_structured_node_error` | ✅ | ✅ green |
| 25-03-01 | 03 | 2 | FT-02, FT-04 | T-25-11/13 | Checkpoint — human confirms the node-kind matrix and the stored fingerprint contract | checkpoint | *(blocking `checkpoint:decision`, no automated command)* | n/a | ✅ resolved — auto-mode `proceed-as-locked` (D-11/D-12), recorded in 25-03-SUMMARY.md |
| 25-03-02 | 03 | 2 | FT-02, FT-04 | T-25-10/12/13 | Unregistered `Custom` names and unsupported node kinds fail closed before any node runs | unit | `cargo test -p paladin-battalion unregistered_custom_retry_predicate_fails_validation && cargo test -p paladin-battalion battalion_node_rejects_retry_and_cache_but_accepts_timeout_and_on_error` | ✅ | ✅ green |
| 25-03-03 | 03 | 2 | FT-02 | T-25-11 | Routing/merge policy hashed, tuning excluded; version tag bumped with the goldens | unit | `cargo test -p paladin-battalion fingerprint_version_is_v5 && cargo test -p paladin-battalion tuning_retry_does_not_change_the_fingerprint` | ✅ | ✅ green |
| 25-04-01 | 04 | 2 | FT-06 | T-25-15/16/19 | Cache is best-effort; keys do not collide across graphs | unit (contract suite) | `cargo test -p paladin-storage node_cache` | ✅ | ✅ green |
| 25-04-02 | 04 | 2 | FT-06 | T-25-15/17 | Redis feature in no default set; invalidation uses a cursor scan | unit + build | `cargo build -p paladin-storage --features redis-cache && cargo test -p paladin-storage --features redis-cache node_cache` | ✅ | ✅ green (local tier, 20 passed); Redis live tier CI-only — `redis-cache-integration` green on run 34051074633 |
| 25-04-03 | 04 | 2 | FT-06 | T-25-18 | Password never `Debug`-printed; config off by default | unit | `cargo test default_node_cache_config_is_disabled && cargo test env_overrides_apply_for_every_field` | ✅ | ✅ green |
| 25-05-01 | 05 | 3 | FT-01 | T-25-20/21 | Redact-then-bound on character boundaries, in one place for nine adapters | unit | `cargo test -p paladin-llm http_status::tests && cargo test -p paladin-llm excerpt_is_redacted_before_it_is_bounded` | ✅ | ✅ green |
| 25-05-02 | 05 | 3 | FT-01 | T-25-22/23 | No adapter keeps a private status mapping; status never read from text | unit | `cargo test -p paladin-llm --all-features openai && cargo test -p paladin-llm --all-features deepseek && cargo test -p paladin-llm --all-features kimi` | ✅ | ✅ green |
| 25-05-03 | 05 | 3 | FT-01 | T-25-22/23 | All nine adapters on the one helper | unit | `cargo test -p paladin-llm --all-features` | ✅ | ✅ green |
| 25-06-01 | 06 | 3 | FT-01 | T-25-24/26 | Conversion reads typed fields only; rendered text unchanged | unit | `cargo test -p paladin-battalion conversion_preserves_the_rendered_message_exactly && cargo test -p paladin-battalion conversion_carries_transience_from_the_source` | ✅ | ✅ green |
| 25-06-02 | 06 | 3 | FT-01 | T-25-25/27 | Circuit-breaker behaviour and log lines unchanged | unit | `cargo test paladin_execution_service_surfaces_structured_llm_failure && cargo test rendered_error_text_at_every_migrated_site_is_unchanged` | ✅ | ✅ green |
| 25-07-01 | 07 | 3 | FT-01, FT-02 | T-25-28/32 | Additive persisted fields; old payloads still deserialise | unit + contract | `cargo test -p paladin-ai-core failed_attempts_are_recorded_in_order && cargo test -p paladin-storage waypoint_with_attempt_history_round_trips` | ✅ | ✅ green (in-memory + sqlite tiers); Postgres tier CI-only — `postgres-integration` green on run 34051074633 |
| 25-07-02 | 07 | 3 | FT-01 | T-25-28/32 | Structured failure travels; display line unchanged | unit + contract | `cargo test -p paladin-battalion exhausted_retry_writes_a_failed_waypoint_carrying_the_structured_error && cargo test -p paladin-storage failed_waypoint_with_node_error_round_trips` | ✅ | ✅ green |
| 25-07-03 | 07 | 3 | FT-02 | T-25-29/30/31 | Per-task retry isolation; no Waypoint between attempts | unit | `cargo test -p paladin-battalion one_mustered_task_retries_without_re_running_siblings && cargo test -p paladin-battalion no_waypoint_is_written_between_attempts` | ✅ | ✅ green |
| 25-08-01 | 08 | 4 | FT-05 | T-25-38 | Checkpoint — human confirms the public `PaladinResult` field strategy | checkpoint | *(blocking `checkpoint:decision`, no automated command)* | n/a | ✅ resolved — auto-mode `option-b-constructible` (D-26), recorded in 25-08-SUMMARY.md |
| 25-08-02 | 08 | 4 | FT-05 | T-25-33/34/35/36 | No mid-stream provider switch; permanent errors short-circuit; no cross-call state | unit + multi-thread | `cargo test -p paladin-llm permanent_error_short_circuits_after_one_call && cargo test -p paladin-llm streaming_error_after_the_first_chunk_propagates_with_the_prefix && cargo test -p paladin-llm concurrent_calls_share_no_hop_state` | ✅ | ✅ green |
| 25-08-03 | 08 | 4 | FT-05 | T-25-34/37 | Legacy JSON byte-identical when the field is absent; register row landed with the field | unit + build | `cargo test -p paladin-ai-core served_by_is_absent_from_legacy_json && cargo build --workspace --all-features --all-targets` | ✅ | ✅ green |
| 25-09-01 | 09 | 5 | FT-03 | T-25-43 | Defaulted trait method only; no reverse crate dependency | unit | `cargo test -p paladin-ports execute_observed_defaults_to_execute && cargo test -p paladin-battalion node_context_keeps_its_derives_with_a_heartbeat_handle` | ✅ | ✅ green |
| 25-09-02 | 09 | 5 | FT-03 | T-25-39/40/41/42 | Stalled work killed, slow work not; timed-out attempt discarded | unit (paused clock) | `cargo test -p paladin-battalion a_port_that_stalls_300ms_fails_with_timeout_idle && cargo test -p paladin-battalion a_slow_but_progressing_node_fails_on_run_timeout_not_idle` | ✅ | ✅ green |
| 25-09-03 | 09 | 5 | FT-03 | T-25-39/44 | Run-level bound enforced through the existing limit path; R-23-01 stays accepted | unit | `cargo test -p paladin-battalion engine_run_timeout_ends_the_run_with_a_typed_error && cargo test -p paladin-battalion the_tightest_bound_fires` | ✅ | ✅ green |
| 25-10-01 | 10 | 6 | FT-04 | T-25-46/49 | `Route` cannot write a `Sum` field or target a template | unit | `cargo test -p paladin-battalion route_error_field_must_not_use_sum_dispatch && cargo test -p paladin-battalion route_target_must_not_be_a_worker_template` | ✅ | ✅ green |
| 25-10-02 | 10 | 6 | FT-04 | T-25-45/48/50 | Handler runs only after exhaustion; structured error into declared state | unit | `cargo test -p paladin-battalion route_writes_the_structured_error_and_places_the_target && cargo test -p paladin-battalion a_handler_does_not_run_while_retries_remain` | ✅ | ✅ green |
| 25-10-03 | 10 | 6 | FT-04 | T-25-47 | Compensation cycles bounded by the existing visit limit | unit (timeout-guarded) | `cargo test -p paladin-battalion a_compensation_cycle_terminates_with_the_visit_limit` | ✅ | ✅ green |
| 25-11-01 | 11 | 7 | FT-04 | T-25-51 | No routing out of a single mustered task | unit | `cargo test -p paladin-battalion route_on_a_worker_template_is_rejected_at_validation && cargo test -p paladin-battalion a_worker_handler_returning_goto_fails_the_run_with_a_typed_error` | ✅ | ✅ green |
| 25-11-02 | 11 | 7 | FT-04 | T-25-53/54/55 | Handler Parley uses the durable suspension path only | unit | `cargo test -p paladin-battalion a_handler_raised_parley_suspends_the_run && cargo test -p paladin-battalion the_post_resume_rerun_is_a_fresh_attempt_one` | ✅ | ✅ green |
| 25-11-03 | 11 | 7 | FT-04 | T-25-52 | Compensation chain and loop bound proven end to end | integration | `cargo test --test e2e_compensation_chain` | ✅ | ✅ green |
| 25-12-01 | 12 | 8 | FT-02 | T-25-59 | Additive mock-port counter; global semantics unchanged | unit | `cargo test --workspace --tests fail_paladin_until_attempt_is_scoped_to_one_paladin` | ✅ | ✅ green |
| 25-12-02 | 12 | 8 | FT-02, FT-04 | T-25-59 | E2E-3 proven by real retry, exact counts, default predicate | integration | `cargo test --test e2e_muster_defer_order` | ✅ | ✅ green (37/37) |
| 25-12-03 | 12 | 8 | FT-02, FT-03 | T-25-56/57/58/60 | Concurrency isolation, prompt shutdown, resume at attempt 1, nested bounds | integration (multi-thread, timeout-guarded) | `cargo test --test aegis_retry_stress` | ✅ | ✅ green |
| 25-13-01 | 13 | 8 | FT-06 | T-25-65/67 | `CachePolicy` without a backend or on a denied field fails closed | unit | `cargo test -p paladin-battalion a_cache_policy_without_an_engine_cache_fails_validation && cargo test -p paladin-battalion a_cache_policy_on_a_deny_output_field_fails_validation` | ✅ | ✅ green |
| 25-13-02 | 13 | 8 | FT-06 | T-25-61/62/64 | Key covers graph identity; failures never cached | unit | `cargo test -p paladin-battalion key_includes_the_graph_fingerprint && cargo test -p paladin-battalion a_failed_attempt_is_never_stored` | ✅ | ✅ green |
| 25-13-03 | 13 | 8 | FT-06 | T-25-63 | Backend failure degrades throughput, never correctness | unit (paused clock) | `cargo test -p paladin-battalion ttl_expiry_re_executes_the_node && cargo test -p paladin-battalion a_put_failure_leaves_the_run_completed` | ✅ | ✅ green |
| 25-14-01 | 14 | 9 | FT-01…FT-06 | T-25-71 | Guide states the four real limitations, including R-23-01 still accepted | doc test + build | `cargo test --workspace --doc` (plus the mdBook build) | ✅ | ✅ green |
| 25-14-02 | 14 | 9 | FT-01…FT-06 | T-25-68/73 | Register and allowlist agree; anchors point at real tests | source assertion | `test "$(grep -c '^\[\[entry\]\]' .cargo/semver-checks-allowlist.toml)" -eq 5` | ✅ | ✅ green |
| 25-14-03 | 14 | 9 | FT-01…FT-06 | T-25-69/70/71/72 | Every gate run and recorded; credential review performed; R-23-01 re-listed as accepted | CI gate | `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && make security && cargo test --workspace` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every `❌ W0` row above is a test that does not exist yet and is created by its own task in the same
plan (RED before GREEN). There is **no** separate Wave 0 plan, because this repository needs no test
framework installation and no shared fixture scaffold — `cargo test`, `cargo-semver-checks 0.50.0` and
`cargo-llvm-cov 0.8.7` are all already installed and pinned. Three genuine Wave-0-shaped gaps are
sequenced inside the plan set rather than deferred:

- [x] **The paused-clock idiom does not exist anywhere in this repository.** A repo-wide search found
      zero uses of `tokio::time::pause` in `paladin-battalion` src or tests. Plan **25-01 Task 3**
      establishes it once in `crates/paladin-battalion/src/engine/retry.rs` with a module rustdoc
      instructing every later timing test (the idle-timeout tests in 25-09, the kill-during-backoff
      test in 25-12, the TTL tests in 25-13) to copy that shape rather than re-derive one.
- [x] **`FaultyPaladinPort` needs `fail_paladin_until_attempt` before the E2E-3 replacement.** Plan
      **25-12 Task 1** lands it, sequenced immediately before Task 2 which consumes it — never
      concurrent with it.
- [x] **`RecordingNodeCache` for engine-side cache tests.** Plan **25-13 Task 2** adds it in-crate to
      `engine/test_support.rs` rather than adding a `paladin-storage` dev-dependency to
      `paladin-battalion`, which has none today.

- [x] Framework install: **none required.**

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions | Outcome (2026-09-06) |
|----------|-------------|------------|-------------------|----------------------|
| Credential-handling review of every path touching an API key or an external response body | FT-01, FT-05, FT-06 | No merge-gating Rust SAST exists: CodeQL is advisory-only at the tested version and Snyk has no Rust coverage, so `security.instructions.md`'s manual review is the primary control | For `map_http_status`, `llm_failure::to_paladin_error`, `FallbackLlmAdapter` and `NodeCacheConfig`: read each and confirm (1) response bodies are redacted **before** truncation, (2) no log statement interpolates an API key and no config type carrying one is `Debug`-formatted outward, (3) no HTTP client sending a credential header follows redirects. Record the outcome in plan 25-14's SUMMARY. | Performed in 25-14 Task 3 and recorded in 25-14-SUMMARY.md. One open finding (three pre-Phase-17 adapters followed redirects with a credential header) was fixed by `2ea6328b` (CR-02, 25-REVIEW-FIX.md). 25-SECURITY.md: `status: verified`, `threats_open: 0`. |
| Redis-backed node-cache contract tier | FT-06 | Docker is absent from this devcontainer; the tier self-skips locally | Runs in the existing Docker-gated CI job (`docker-integration`). Record the result as CI-only evidence; never report it as locally passed. | CI-only — `redis-cache-integration` green on runs 34042790005 and 34051074633 (UAT test 1): 9 contract cases executed against a live Redis, SKIP-detection step passed. Self-skipped locally as designed. |
| Postgres Waypoint contract case for the new `Failed`/`attempts` payloads | FT-01, FT-02 | Same — no Docker locally | Runs in the existing `postgres-integration` CI job. CI-only evidence. | CI-only — `postgres-integration` green on run 34051074633 (every job green per UAT test 1 resolution). Self-skipped locally as designed. |
| mdBook build of the new fault-tolerance guide | FT-01…FT-06 | Requires the `mdbook` binary, which is a documentation toolchain rather than part of `cargo test` | `cd docs && mdbook build` — plan 25-14 Task 1 records the result. | `cd docs && mdbook build` exit 0, "No broken links found" (25-14-SUMMARY.md Task 1). |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or a documented checkpoint/manual exception (re-checked 2026-09-06: every automated command names a test that exists and runs green) (three
      `checkpoint:decision` tasks — 25-01-01, 25-03-01, 25-08-01 — are blocking human gates by design
      and carry no automated command)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (the longest run without
      one is 1 — every checkpoint is immediately followed by an automated task in the same plan)
- [x] Wave 0 covers all MISSING references (the three sequenced gaps above; no separate Wave 0 plan
      is needed)
- [x] No watch-mode flags
- [x] Feedback latency < 120 s for the crate-scoped quick run
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-09-06 (validate-phase audit — see below)

---

## Validation Audit 2026-09-06

| Metric | Count |
|--------|-------|
| Gaps found | 8 |
| Resolved | 8 |
| Escalated | 0 |

**Method.** State A audit (VALIDATION.md pre-existed from plan-phase). All 42 task rows were
cross-referenced against the executed tree: every test named in an automated command was located by
`fn <name>` grep across `crates/`, `src/` and `tests/`; all 121 `ref:` anchors in the fourteen
SUMMARY files resolve to a real test at the cited path; the three `checkpoint:decision` rows carry an
auto-mode resolution recorded in their SUMMARY. Then **every automated command in the map was re-run
verbatim** and its exit code and per-binary `running N tests` / `passed` counts were read, rather than
trusting SUMMARY "verified" lines. 29 rows passed as written. 8 rows were **command defects** — the
tests exist and pass, but the recorded command either could not run or silently selected zero tests:

| Rows | Defect | Correction | Re-run result |
|------|--------|------------|---------------|
| 25-01-02, 25-02-01, 25-02-02, 25-07-01, 25-08-03 (and the Quick-run command above) | `-p paladin-core` — no such package; the core crate's package is `paladin-ai-core` (exit 101) | `-p paladin-ai-core` | 17 / 2 / 1 / 2 / 1 passed, 0 failed |
| 25-05-01 | `map_http_status` filter matches no test path (tests live under `http_status::tests`); ran 0 tests, exit 0 | filter `http_status::tests` | 6 + 1 passed |
| 25-05-02 | `deepseek` / `kimi` filters select 0 tests under default features (`default = ["openai", "mock"]`); only `openai` ran | add `--all-features` to each | 52 / 33 / 32 passed |
| 25-12-01 | `--all-targets` also runs `benches/config_benchmarks.rs`, which panics at startup on a missing `llm.anthropic.api_key` setting (exit 101 although the test itself passed 9×) | `--tests` | 9 passed, 0 failed |

Every corrected command is now the one recorded in the map. All corrections are to the map only; no
implementation or test file was touched, no test file was generated, and no gsd-nyquist-auditor spawn
was required (nothing was missing or failing — the auditor would have had nothing to write).

**Whole-suite evidence** (same session, warm checkout):

| Command | Result |
|---------|--------|
| `cargo test --workspace` | exit 0, 0 failed |
| `cargo test --test e2e_muster_defer_order` | 37 passed |
| `cargo test --test e2e_compensation_chain` | 5 passed |
| `cargo test --test aegis_retry_stress` | 37 passed |
| `cargo test -p paladin-llm --all-features` | 368 passed |
| `cargo build -p paladin-storage --features redis-cache` | exit 0 |
| `cargo test -p paladin-storage --features redis-cache node_cache` | 20 passed (live-server tier self-skipped) |
| `cargo test --workspace --doc` | exit 0, 0 failed |
| 25-14-03 gate row (`cargo fmt --check && cargo clippy … -D warnings && make security && cargo test --workspace`) | exit 0, 4426 passed, 0 failed |
| `grep -c '^\[\[entry\]\]' .cargo/semver-checks-allowlist.toml` | 5 |

**Requirement coverage** (rows in the Per-Task Map, all automated rows green after correction):

| Requirement | Automated rows | Checkpoint rows |
|-------------|----------------|-----------------|
| FT-01 | 14 | 0 |
| FT-02 | 14 | 0 |
| FT-03 | 7 | 0 |
| FT-04 | 12 | 0 |
| FT-05 | 6 | 0 |
| FT-06 | 9 | 0 |

**Notes for readers.** The 25-04-03 and 25-06-02 tests live in the root `paladin-ai` crate
(`src/config/node_cache.rs`, `src/application/services/paladin/`), so their bare `cargo test <name>`
commands must run from the workspace root, as written. The Redis/Postgres live-server tiers remain
CI-only evidence, now backed by a green run (34051074633) rather than a pending expectation.

**Result:** `nyquist_compliant: true`, `status: validated`, `wave_0_complete: true`.
