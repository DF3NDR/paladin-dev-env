---
phase: 25-node-level-fault-tolerance
verified: 2026-09-06T23:12:17Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 5/5
  gaps_closed:
    - "REQUIREMENTS.md documentation-tracking gap: FT-02..FT-06 checkboxes/status table now [x]/Complete (all six rows), matching FT-01"
  gaps_remaining: []
  regressions: []
---

# Phase 25: Node-Level Fault Tolerance Verification Report

**Phase Goal:** Individual nodes retry with provable backoff, distinguish stalled from slow work via
nested timeouts, compensate typed errors instead of failing the run, fail over across LLM providers,
and cache expensive deterministic results.

**Verified:** 2026-09-06T23:12:17Z
**Status:** passed
**Re-verification:** Yes — after five post-plan code-review fix commits (21e9c989..6b566916) and the public-API baseline regeneration (0e5c106c)

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 (FT-01) | `transience()` on `PaladinError`/`LlmError` is table-driven per variant; provider adapters carry status-carrying error variants with no string parsing; `BattalionError::Node(NodeError)` carries a structured `NodeError`; every touched enum is `#[non_exhaustive]` and registered in `MIGRATION.md` §9.2 | ✓ VERIFIED | Unchanged code path re-confirmed: `paladin_error.rs` (`transience()`), `LlmError::ProviderError{provider,status,message}`, `BattalionError::Node(NodeError)`. Re-ran `cargo test -p paladin-ai-core --lib -- transience_table` → 1 passed. The five review-fix commits added `300..=399` redirect-refusal arms to openai/anthropic/deepseek `map_error` (all still typed `ProviderError`, no string parsing) — verified in code at `openai/adapter.rs:219`, `deepseek/adapter.rs:455`, `anthropic/adapter.rs:345`. |
| 2 (FT-02) | Per-node Aegis retry follows exact backoff under a paused clock with asserted jitter bounds, gates on transience (Permanent → 1 attempt), discards failed-attempt deltas while keeping `AttemptRecord` history, retries per-task inside a Muster | ✓ VERIFIED | Engine-level retry (`crates/paladin-battalion/src/engine/retry.rs`) untouched by the fix commits; re-ran `backoff_sequence_is_exact_with_jitter_off`, `permanent_error_under_transient_only_takes_one_attempt`, `backoff_wait_returns_early_when_the_run_is_cancelled` → 3/3 pass. Re-ran the three named integration binaries per environment constraint: `cargo test --test e2e_muster_defer_order` → 37/37 pass (incl. `one_worker_recovers_by_real_per_task_retry`); `cargo test --test aegis_retry_stress` → 37/37 pass (incl. `resuming_after_a_kill_during_backoff_restarts_at_attempt_one`, `concurrent_tasks_do_not_share_retry_state`). WR-02's new service-level gate (below) does not touch this engine-level Aegis loop — it is a separate, higher (application-service) retry site. |
| 3 (FT-03) | A wall-clock `run_timeout` and progress-aware `idle_timeout` are distinguished and nested with engine/Battalion bounds so the tightest fires and the error names which | ✓ VERIFIED | `crates/paladin-core/src/platform/container/heartbeat.rs`, `TimeoutKind::{Run,Idle,EngineRun}` unmodified by the review-fix commits. Re-ran `cargo test --test aegis_retry_stress` (includes `an_engine_run_timeout_tighter_than_both_bounds_names_enginerun`, `a_node_run_timeout_tighter_than_the_engine_budget_names_run`) → both pass within the 37/37 result above. |
| 4 (FT-04) | Program scenario E2E-3 passes together with CF-03: `Route`/`Absorb`/registered-`Custom` compensates a transiently-failing Muster worker without failing the run; unregistered `Custom` fails closed; handler loops bounded by `max_node_visits` | ✓ VERIFIED | `superstep.rs::dispatch_error_handler` and `WarGraph::validate_aegis_handler_wiring` unmodified by the review-fix commits. Re-ran `cargo test --test e2e_compensation_chain` → 5/5 pass, including `compensation_chain_routes_a_permanent_failure_to_a_recovery_node`, `compensation_loop_terminates_at_the_visit_limit`, `on_payment_failure_parley_a_human`. |
| 5 (FT-05, FT-06) | `FallbackLlmAdapter` fails over on Transient/Unknown only (short-circuits Permanent) without silently switching providers mid-stream; a `CachePolicy`-keyed node hits its `NodeCachePort` cache with `cache_hit: true` and no re-execution; failures are never cached | ✓ VERIFIED | `crates/paladin-llm/src/fallback.rs` and `NodeCachePort`/cache_key composition unmodified by the review-fix commits (the fixes touched openai/anthropic/deepseek `adapter.rs` and `redaction.rs`, and `paladin_execution_service.rs` — none of the fallback or cache modules). Re-verified via `cargo test -p paladin-llm --all-features` scoped runs (below) that provider adapters still classify correctly through `transience()`, which `FallbackLlmAdapter`'s hop logic depends on. |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified)

### Post-Verification Code-Review Fix Commits — Re-Checked Against Current Tree

All five fixes from `25-REVIEW-FIX.md` were independently re-verified by reading the current code (not the SUMMARY narrative) and re-running the cited tests in this session:

| Commit | Claim | Code Verified | Test Verified |
|--------|-------|---------------|---------------|
| `21e9c989` (CR-01) | Anthropic 400 usage-cap body redacted via `crate::redaction::redact_credentials` before `extract_regain_hint` | `anthropic/adapter.rs:365-369` — `redact_credentials(body, ...)` then `extract_regain_hint(&redacted)` | `cargo test -p paladin-llm --all-features anthropic` → 30 passed |
| `2ea6328b` (CR-02) | openai/anthropic/deepseek clients refuse redirects (`Policy::none()`); `300..=399` maps to a typed `ProviderError` | `.redirect(reqwest::redirect::Policy::none())` present in all three adapters; `300..=399 =>` arm present in all three `map_error` fns | `cargo test -p paladin-llm --all-features openai` → 52 passed; anthropic → 30 passed; deepseek → 33 passed; `cargo test -p paladin-ai --all-features anthropic_adapter_test` → both binaries green incl. `test_anthropic_client_refuses_to_follow_a_redirect`; `deepseek_adapter_test` → both binaries green incl. `test_deepseek_client_refuses_to_follow_a_redirect` |
| `1e50ca3f` (WR-01) | DeepSeek adapter's local redaction copies removed; now imports `crate::redaction::{RESPONSE_EXCERPT_CHAR_BUDGET, bounded_excerpt, redact_credentials}` | `deepseek/adapter.rs:34-35` — imports confirmed, no local duplicate definitions remain | `cargo test -p paladin-llm --all-features deepseek` → 33 passed |
| `724db484` (WR-02) | `execute_with_retry_and_temperature`/`execute_with_retry` gain a `Transience::Permanent` short-circuit arm, checked before the `attempt >= max_attempts` exhaustion arm, after `CircuitBreakerOpen` | Both arms present at `paladin_execution_service.rs:1746` and `:1883`, in the documented order (CircuitBreakerOpen → Permanent → exhaustion → retry) | `cargo test -p paladin-ai --all-features --lib "application::services::paladin::paladin_execution_service"` → 27 passed (incl. `permanent_failure_is_not_retried_by_buffered_retry_sites`, `transient_and_unknown_failures_still_retry_until_max_attempts`); `cargo test -p paladin-ai --all-features paladin_execution_service` → 21 passed |
| `6b566916` (follow-on) | Anthropic's malformed-response excerpt now redacted via `crate::redaction::diagnostic_excerpt` before bounding, private duplicate helper removed | `anthropic/adapter.rs:354,473` call `diagnostic_excerpt`; no private `bounded_excerpt`/`RESPONSE_EXCERPT_CHAR_BUDGET` remain in the file (only imported test helper at line 796) | Covered by the same `anthropic` 30-passed run above |

**No regression found.** WR-02's new `Transience::Permanent` short-circuit is a service-layer (`PaladinExecutionService`) retry gate, distinct from the engine-layer Aegis `should_retry` gate FT-02 asserts (`crates/paladin-battalion/src/engine/retry.rs`), and does not modify or bypass it — confirmed by re-reading both call sites and by the unmodified `retry.rs` file (`git diff` shows the five fix commits touch only `crates/paladin-llm/src/{anthropic,deepseek,openai}/adapter.rs`, `crates/paladin-llm/src/redaction.rs`, `src/application/services/paladin/paladin_execution_service.rs`, and two `tests/unit/llm/*_adapter_test.rs` files — no `paladin-battalion` engine files). `transient_and_unknown_failures_still_retry_until_max_attempts` positively re-proves Transient/Unknown are unaffected.

### Required Artifacts

All artifacts listed in the previous verification (`crates/paladin-core/.../transience.rs`, `node_error.rs`, `aegis.rs`; `crates/paladin-battalion/src/engine/retry.rs`; retry/error-handler registries; `heartbeat.rs`; `crates/paladin-llm/src/fallback.rs`; node-cache trio; `cache_key.rs`; fault-tolerance guide; `MIGRATION.md` §9.1-§9.7; `.cargo/semver-checks-allowlist.toml`) are unmodified by the five review-fix commits and the doc-only commits (`0e5c106c`, `332614d4`, `354a2f39`, `ea8633f1`, `010c5743`, `1860bb00`, `83133f00`, `aee5134e`). Re-confirmed unmodified via `git show --stat` on all commits since the previous verification (`e08c4a9e..aee5134e`) — no artifact file above appears in any diff except the five listed in the fix table.

### Key Link Verification

Unchanged from the previous verification — none of the five key links (`WarGraph::aegis_for` → dispatch closure; `LlmError::transience()` → `PaladinError::LlmFailure`; `Aegis.on_error` → `dispatch_error_handler`; `FallbackLlmAdapter` → `PaladinResult.served_by`; `CachePolicy` → `NodeCachePort::get/put`) touch any file modified since `e08c4a9e`.

### Data-Flow Trace (Level 4)

Not applicable — this phase is a backend orchestration engine with no UI data-rendering surface; no new dynamic-data-rendering artifacts were introduced by the review-fix commits.

### Behavioral Spot-Checks (re-run this session, HEAD `aee5134e`)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Backoff sequence exact / Permanent 1-attempt / cancel-aware wait | `cargo test -p paladin-battalion --lib -- backoff_sequence_is_exact_with_jitter_off permanent_error_under_transient_only_takes_one_attempt backoff_wait_returns_early_when_the_run_is_cancelled` | 3 passed | ✓ PASS |
| `transience()` table-driven classification | `cargo test -p paladin-ai-core --lib -- transience_table` | 1 passed | ✓ PASS |
| E2E-3 real per-task retry (Muster) | `cargo test --test e2e_muster_defer_order` | 37 passed | ✓ PASS |
| Compensation chain + loop bound + handler-Parley | `cargo test --test e2e_compensation_chain` | 5 passed | ✓ PASS |
| Multi-thread retry stress, kill-during-backoff, nested timeout naming | `cargo test --test aegis_retry_stress` | 37 passed | ✓ PASS |
| CR-01 redaction (Anthropic usage-cap regain hint) | `cargo test -p paladin-llm --all-features anthropic` | 30 passed | ✓ PASS |
| WR-01 dedupe (DeepSeek shared redaction) | `cargo test -p paladin-llm --all-features deepseek` | 33 passed | ✓ PASS |
| CR-02 redirect refusal (openai) | `cargo test -p paladin-llm --all-features openai` | 52 passed | ✓ PASS |
| CR-02 redirect refusal end-to-end (anthropic, deepseek, both binaries) | `cargo test -p paladin-ai --all-features anthropic_adapter_test` / `deepseek_adapter_test` | 11 passed (x2 binaries) / 9 passed (x2 binaries) | ✓ PASS |
| WR-02 Permanent short-circuit (unit + integration) | `cargo test -p paladin-ai --all-features --lib "application::services::paladin::paladin_execution_service"` / `cargo test -p paladin-ai --all-features paladin_execution_service` | 27 passed / 21 passed | ✓ PASS |
| Anti-pattern scan on the 5 fix-touched files | `grep -inE "TBD|FIXME|XXX|TODO|HACK|placeholder"` on all touched files | 0 debt-marker hits; 2 pre-existing unrelated `placeholder` comments (vision test doc-comment, pre-724db484, confirmed via `git show 724db484 \| grep -c placeholder` = 0 added) | ✓ PASS |
| Workspace fmt | `cargo fmt --all --check` | exit 0 | ✓ PASS |
| Clippy on touched crates | `cargo clippy -p paladin-llm -p paladin-ai --all-targets --all-features -- -D warnings` | exit 0 | ✓ PASS |

Full-suite evidence (`cargo test --workspace`, `cargo llvm-cov`, `cargo semver-checks`, MSRV, `make security`) was not re-run in this verification per the orchestrator's constraint (disk headroom is 126G free, above the 25G threshold, but the environment note still directs targeted crate/binary-scoped runs for this re-verification). This evidence is independently corroborated by CI run `34051074633` on commit `0e5c106c` (31 jobs green, including API Surface Tracking, Semver Checks, MSRV 1.88, Coverage, Security Audit, License & Dependency Policy — recorded in `25-UAT.md` test 4, resolved) plus `25-REVIEW-FIX.md`'s own one-time full-suite run after all five fix commits landed (1 unrelated pre-existing failure, `cli_isolation::test_cli_feature_is_not_default`, which trips by design under `--all-features` per the environment note and is not a regression). `25-14-SUMMARY.md`'s Gate Evidence table (4413 passed/0 failed, semver clean 11/11, MSRV 1.88 clean, `make security` clean, 89.34% coverage vs 82% floor) remains the base evidence; the five fix commits and the API-surface regeneration commit ran fmt+clippy+targeted-crate-tests per their own commit hooks and are re-verified above by direct re-execution in this session.

### CI-Only Evidence Tiers (per `25-UAT.md`, not re-audited here)

- **Redis node-cache contract suite (25-04 D4, FT-06):** read green on CI run `34051074633` (job includes `redis_node_cache_runs_the_full_contract_suite`, `redis_keys_are_namespaced_by_the_configured_prefix`, `redis_ttl_is_set_on_the_server_not_only_in_the_payload`). Devcontainer has no Docker/Redis; local self-skip is a pre-existing Phase 22 tiering convention, not a gap.
- **Postgres Waypoint contract suite (25-07 D3, FT-02):** read green on the same CI run (`waypoint_with_attempt_history_round_trips`, `failed_waypoint_with_node_error_round_trips`).
- Both tiers are cited from `25-UAT.md` (status: complete, 76/76 pass, 0 open issues) rather than re-run, per the task instructions.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| FT-01 | 25-01, 25-02, 25-05, 25-06, 25-07, 25-14 | Error taxonomy, transience(), structured NodeError, X-10 register | ✓ SATISFIED | Code + re-run tests above; `REQUIREMENTS.md` checkbox `[x]`, status table row `Complete` |
| FT-02 | 25-01, 25-03, 25-07, 25-12, 25-14 | Aegis retry, backoff, predicate gating, per-task Muster retry | ✓ SATISFIED | Code + re-run tests above; `REQUIREMENTS.md` checkbox `[x]`, status table row `Complete` |
| FT-03 | 25-09, 25-12, 25-14 | Nested run/idle timeouts, heartbeat, RunTimeoutExceeded | ✓ SATISFIED | Code + re-run tests above; `REQUIREMENTS.md` checkbox `[x]`, status table row `Complete` |
| FT-04 | 25-03, 25-10, 25-11, 25-12, 25-14 | Route/Absorb/Custom handlers, E2E-3, max_node_visits | ✓ SATISFIED | Code + re-run tests above; `REQUIREMENTS.md` checkbox `[x]`, status table row `Complete` |
| FT-05 | 25-08, 25-14 | FallbackLlmAdapter, served_by | ✓ SATISFIED | Code unmodified by fix commits; `REQUIREMENTS.md` checkbox `[x]`, status table row `Complete` |
| FT-06 | 25-04, 25-13, 25-14 | NodeCachePort, InMemory/Redis, engine cache integration | ✓ SATISFIED | Code unmodified by fix commits; Redis live-server tier CI-green per `25-UAT.md`; `REQUIREMENTS.md` checkbox `[x]`, status table row `Complete` |

**Orphaned requirements:** none — `.planning/REQUIREMENTS.md`'s "Node-Level Fault Tolerance (Doc 04, epic FT)" section lists exactly FT-01..FT-06, and all six are claimed across the 14 plans' `requirements:` frontmatter (re-confirmed this session by grepping every `25-*-PLAN.md`).

**Documentation-tracking gap — RESOLVED.** The previous verification (2026-09-06T03:38:04Z) flagged `REQUIREMENTS.md`'s FT-02..FT-06 checkboxes/status-table rows as stale (`[ ]`/`Pending` despite the phase being fully implemented). Confirmed this session: `REQUIREMENTS.md` lines 132-165 now show all six as `[x]`, and the traceability table (lines 369-374) shows all six rows as `Complete`. Gap closed, no residual documentation lag.

### Anti-Patterns Found

None introduced by the five review-fix commits or the three doc-only commits. Scanned all seven files touched since the previous verification (`anthropic/adapter.rs`, `deepseek/adapter.rs`, `openai/adapter.rs`, `redaction.rs`, `paladin_execution_service.rs`, and the two `tests/unit/llm/*_adapter_test.rs` files) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`placeholder` — the only hits are the identifier `CREDENTIAL_PLACEHOLDER` (not a stub marker) and two pre-existing, unrelated `placeholder` doc-comments in `paladin_execution_service.rs` (a handoff-service fallback-message note and a `#[cfg(feature = "vision")]` test doc-comment), both confirmed present before `724db484` via `git show 724db484 -- <file> | grep -c placeholder` = 0 added lines.

### Human Verification Required

None. This phase is pure backend engine work. The Redis (25-04 D4) and Postgres (25-07 D3) live-server tiers remain the only environment-availability items, and both are already resolved as CI-green per `25-UAT.md` (tests 1 and 3, `result: pass`) — not re-listed here as open human-verification items.

### Gaps Summary

No gaps. This re-verification confirms:

1. All five ROADMAP success criteria remain independently verified against the current tree (`aee5134e`) — none of the engine-layer code (`paladin-battalion`, `paladin-core`, `paladin-storage`, `paladin-ports`) that FT-01 through FT-06 depend on was touched by the intervening commits.
2. All five code-review fix commits (`21e9c989`, `2ea6328b`, `1e50ca3f`, `724db484`, `6b566916`) were independently re-verified by direct code inspection (not `25-REVIEW-FIX.md`'s narrative) and by re-running every test each commit's own verification section cites, in this session, against the current tree — all pass.
3. WR-02's new service-level `Transience::Permanent` retry short-circuit does not regress FT-02's engine-level Aegis retry gating: the two are separate code paths (`paladin_execution_service.rs` application-service retry loop vs. `crates/paladin-battalion/src/engine/retry.rs` engine Aegis loop), confirmed by `git show --stat` on the fix commits (no `paladin-battalion` files touched) and by positive re-runs of both the engine-level backoff/predicate tests and the new/updated service-level tests (`permanent_failure_is_not_retried_by_buffered_retry_sites`, `transient_and_unknown_failures_still_retry_until_max_attempts`).
4. The previous verification's only non-blocking finding (REQUIREMENTS.md checkbox lag for FT-02..FT-06) is resolved — all six requirements now show `[x]`/`Complete`.
5. The public-API baseline regeneration (`0e5c106c`) is purely additive (284 lines added, 2 removed — a net regeneration reformat, not a scope change) and is corroborated by CI run `34051074633` going green on that exact commit, including the previously-failing API Surface Tracking job.
6. `cargo fmt --all --check` and `cargo clippy -p paladin-llm -p paladin-ai --all-targets --all-features -- -D warnings` both exit 0 on the current tree.

---

*Verified: 2026-09-06T23:12:17Z*
*Verifier: Claude (gsd-verifier)*
