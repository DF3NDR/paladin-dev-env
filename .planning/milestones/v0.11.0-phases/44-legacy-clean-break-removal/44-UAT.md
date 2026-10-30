---
status: complete
phase: 44-legacy-clean-break-removal
source: [44-01-SUMMARY.md, 44-02-SUMMARY.md, 44-03-SUMMARY.md, 44-04-SUMMARY.md, 44-05-SUMMARY.md, 44-06-SUMMARY.md, 44-07-SUMMARY.md, 44-08-SUMMARY.md, 44-09-SUMMARY.md, 44-10-SUMMARY.md, 44-11-SUMMARY.md, 44-12-SUMMARY.md, 44-13-SUMMARY.md]
started: 2026-10-12T01:34:56Z
updated: 2026-10-12T18:35:42Z
---

## Current Test

[testing complete]

## Tests

### 1. Legacy surfaces are gone from the tree (source-tree guard)
expected: `cargo test --test legacy_removal_guard` prints "3 passed; 0 failed" (legacy_surfaces_are_gone_from_the_tree plus the two positive controls scanner_reports_planted_legacy_uses and scanner_accepts_a_clean_tree). `git grep -nw 'ContinueOnError\|RetryThenContinue' -- '*.rs'` prints nothing and `crates/paladin-battalion/src/retry.rs` no longer exists. The same-named Aegis `RetryPolicy`, Flow `ErrorStrategy` and structured `node_error::NodeError` are untouched and `cargo check --workspace --all-targets --all-features` exits 0.
result: pass

### 2. Campaign is bounded per attempt by Aegis; the whole-run clock is gone
expected: `cargo test -p paladin-battalion --lib -- aegis_attempt campaign_service` passes (10 runner tests, 12 Campaign tests). `cargo test -p paladin-battalion --lib commander_campaign_honours_aegis_per_attempt_timeout_end_to_end` passes: three 600 ms Paladins complete under a 1 s per-attempt bound (a whole-run clock would have fired at 1 s), and a 1.5 s Paladin fails with `BattalionError::Node` carrying `Timeout(Run)`, attempt 1, Transient. Campaign ignores `aegis.on_error` and logs one warn per execution when it is set.
result: pass
reported: "Failure on CI pre-commit" (initial response; gap G-44-2)
resolution: "Re-verified after fix 1e93e1ce: the reported pre-commit CI failure was the end-of-file-fixer hook on three Phase 43 plan files, unrelated to Campaign/Aegis; pre-commit workflow run 38193792540 on fd3d686e concluded success, and the Campaign tests in this checkpoint were not implicated."

### 3. Invalid Aegis policies are rejected at entry, before any Paladin runs
expected: `cargo test -p paladin-battalion --lib -- rejects_invalid_aegis_before_any` passes for Campaign, Formation, Phalanx and Conclave. `cargo test -p paladin-ai-core --lib validate_aegis` reports 5 passed: Route/Custom error handlers, Custom retry predicates and zero durations are rejected; a cache policy is accepted and ignored. `commander_builder_rejects_zero_run_timeout` passes: a `CommanderBuilder` given `run_timeout = 0` fails to build with `CommanderValidation`.
result: pass

### 4. Formation fails fast by default and continues past a failure only under Absorb
expected: `cargo test -p paladin-battalion --lib formation_service` reports 11 passed. With no `on_error` handler a failing Paladin stops the Formation (`BattalionError::Node` on a timeout, the v0.10 `BattalionError::PaladinError` text otherwise). With `on_error = Absorb` the failure is recorded in `node_errors`, the next Paladin receives an empty input, and the run reports `Completed`. `cargo run --example formation_sequential` exits 0 and shows the Absorb run.
result: pass

### 5. Phalanx bounds each attempt, retries Paladins independently and reports typed failures in declaration order
expected: `cargo test -p paladin-battalion --lib phalanx_service` passes, including phalanx_bounds_each_attempt_not_the_run, phalanx_paladins_retry_independently, phalanx_node_errors_follow_declaration_order and phalanx_fail_fast_aggregates_structured_failures. Under `on_error: None` every failure is listed in one `AggregationError` reading "Phalanx execution failed with <n> errors: ..." (a timeout reads `run timeout`); under Absorb the run completes with the failures in `node_errors`. `cargo run --example phalanx_parallel` exits 0 and prints `node_errors`.
result: pass

### 6. Phalanx review fixes: a zero concurrency limit is rejected and no attempt starts after cancellation
expected: `cargo test -p paladin-battalion --lib -- phalanx_rejects_zero_max_concurrency_before_any_paladin_runs phalanx_rejects_a_concurrency_limit_above_the_permit_maximum cancelled_phalanx_does_not_start_queued_paladins test_cancellation_support phalanx_cancellation_during_backoff_stops_retrying` all pass: `with_max_concurrency(0)` is rejected with a named error instead of hanging forever, a limit above `Semaphore::MAX_PERMITS` is rejected instead of panicking, and a cancelled Phalanx returns `Cancelled` without starting the Paladins still queued behind the concurrency limit (commits 229a28a5, cf0be3db, dc55f18e).
result: pass

### 7. node_errors is the structured NodeError on the wire and in all three heralds
expected: `cargo test -p paladin-ai-core --lib battalion_result` reports 9 passed: each element serialises as `{"node_id","attempt","transience","source"}` in that key order, the v0.10 `{"node_name","error"}` element fails to deserialize, and `[]` or an absent key reads as empty. `cargo test -p paladin-herald --all-features` passes: Markdown renders `**Scout:** attempt 2, Transient: run timeout`, Table renders `  Scout (attempt 2, Transient): run timeout`, JSON carries the four keys with an object-valued `source`, and an `Llm` source line shows neither the provider name nor the HTTP status.
result: pass

### 8. Circuit breaker and Conclave classify failures by transience() only
expected: `cargo test -p paladin-ai --lib circuit_breaker` and `cargo test -p paladin-battalion --lib -- conclave_retry_predicate` pass. A Permanent `LlmFailure` whose message reads "429 rate limit timeout 503 network connection" is neither retried by the Conclave nor counted by the breaker; a Transient `LlmFailure` with an empty message is retried and counted. `git grep -n 'PaladinError::LlmError\|\.is_retryable()\|is_terminal' -- '*.rs'` prints nothing (the remaining `LlmError(String)` variants belong to HandoffError, PromptError and PlanningError, which are out of scope).
result: pass

### 9. Commander has no whole-run clock and a 300 s per-attempt default; the Conclave runs on the engine backoff
expected: `cargo test -p paladin-battalion --lib -- commander conclave_execution_service` passes, including commander_bound_is_not_a_whole_run_clock (two 0.8 s Paladins under a 1 s bound complete in 1.6 s), commander_builder_default_bounds_each_attempt, commander_conclave_bridge_derives_retry_attempts (retry_attempts = max_attempts - 1), commander_maneuver_bridge_defaults_to_fail_fast, conclave_backoff_follows_engine_sequence (500 ms doubling, 60 s cap) and conclave_aggregator_timeout_is_aggregator_failed. The `Commander::execute` rustdoc has a `# Timeouts` section saying the bound is per attempt and that ChainOfCommand, Grove and Council are not bounded.
result: pass

### 10. CLI battalion runs are configured through Aegis with the YAML schema unchanged
expected: `cargo test -p paladin-ai --lib cli` reports 8 passed. `src/application/cli/commands/battalion.rs` has 4 `per_attempt_timeout(` calls and 0 `with_timeout(` calls. `git diff b362d6cb -- src/application/cli/config/battalion_config.rs` touches rustdoc only (no key, type or default changed); that rustdoc and the battalion template comment say "Per-attempt timeout"; `timeout_seconds: 0` in a battalion YAML is still rejected by CLI validation.
result: pass

### 11. The Battalion examples build and run on Aegis
expected: `cargo run --example commander_full_config` exits 0 and prints three runs: fail fast, Absorb (one `node_errors` entry printed with its Display) and retry-then-Absorb succeeding on attempt 3. `cargo run --example` for commander_basic, commander_auto, commander_with_metadata_export, campaign_workflow and conclave_expert_panel each exit 0. `make check-examples` reports 62 of 62 example binaries present. `grep -l 'with_timeout(\|with_error_strategy(\|with_retry_policy(' examples/commander_*.rs examples/formation_sequential.rs examples/phalanx_parallel.rs examples/campaign_workflow.rs examples/conclave_expert_panel.rs` prints nothing (the Maneuver examples' own ManeuverConfig `with_error_strategy` is a separate API and stays).
result: pass

### 12. The mdBook pages teach Aegis and the typed taxonomy
expected: `docs/src/user-guides/orchestration.md` Configuration Reference shows a `with_aegis(Aegis { retry, timeout, on_error, .. })` snippet, a per-pattern table (Formation, Phalanx, Campaign, Conclave, ChainOfCommand/Grove/Council), states that timeouts apply per attempt with no whole-Battalion clock and that `Commander::execute()` can be wrapped in `tokio::time::timeout`, and says the next Paladin after an absorbed failure receives an empty input. `docs/src/getting-started/configuration.md` has no `battalion:` YAML stanza and no `APP_BATTALION_*` line. `docs/src/appendix/conclave-pattern.md` documents the engine backoff (500 ms doubling, 60 s cap) and an `AggregatorFailed` arm with no 2^n table. `make check-doc-config` and `make check-doc-examples` exit 0. `git grep -nw 'ContinueOnError\|RetryThenContinue' -- docs ':!docs/src/api-reference' ':!docs/assets'` prints nothing.
result: pass

### 13. A stored v0.10 BattalionConfig document still loads
expected: `cargo test -p paladin-ai-core --lib -- battalion_config_v0_10_document_deserializes_with_default_aegis` passes: a JSON document carrying the removed `timeout_seconds`, `retry_policy` and `error_strategy` keys deserializes without error with `aegis == Aegis::default()`. `BattalionConfig` has exactly four public fields (name, description, aegis, metadata_output_dir), and `battalion/mod.rs` and `conclave.rs` contain no `serde(alias`, `serde(untagged`, `deny_unknown_fields` or `#[deprecated`.
result: pass

### 14. Every removal is recorded: ADR-0059, MIGRATION rows, semver allowlist and CHANGELOGs
expected: `.planning/decisions/0059-legacy-clean-break-removal.md` is Accepted, carries `## Supersedes` naming ADR-0001 and ADR-0002, says the X-03 supersession is for Phase 44 only, reads Code Conformance `conforms`, and has the (g) checkpoint outcome line dated 2026-10-11; ADR-0001 and ADR-0002 `## Status` read `Superseded`; PROMOTION.md shows next free ADR number 0060. MIGRATION.md 9.1 has rows M-B-06 to M-B-11 and 9.2 has ten `plan 44-11` rows (nine Y, one N for the BattalionResult retype). `./scripts/check-migration-allowlist.sh` reports the allowlist set-equal to the 9.2 register and `make check-gates` exits 0. The root CHANGELOG `[Unreleased]` has a Phase 44 `### Removed` group with six sub-bullets and `### Changed` bullets citing ADR-0059 and the MIGRATION rows; the paladin-core, paladin-battalion and paladin-herald CHANGELOGs each carry their own Phase 44 entries.
result: pass

### 15. The full phase gate is green on the final tree
expected: `cargo fmt --check` is clean; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0; `cargo test --workspace --doc` reports 0 failed; `cargo test --workspace --no-fail-fast` passes every target except the two known pre-phase baseline failures `run_api_wiring::tests::build_run_api_persists_*` (fail identically on baseline 061ee3dc, recorded in deferred-items.md); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` reports "API surface unchanged" (4274 items); `make security` exits 0. CI-only on the phase PR: the 82 percent coverage floor and mdBook rendering are green (neither tool is installable in the sandbox).
result: pass

## Summary

total: 15
passed: 15
issues: 0 (1 reported on test 2, G-44-2, resolved by commit 1e93e1ce and re-verified in CI)
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-44-2
  truth: "Campaign is bounded per attempt by Aegis with the whole-run clock gone, and the phase tree passes the pre-commit CI gate"
  status: resolved
  resolved_by: "commit 1e93e1ce (direct one-byte fix, no gap plan needed)"
  resolved_at: 2026-10-12
  reason: "User reported: Failure on CI pre-commit"
  severity: blocker
  test: 2
  root_cause: "end-of-file-fixer (commit-stage hook) rejects .planning/phases/43-rate-pacing/43-02-PLAN.md, 43-03-PLAN.md and 43-05-PLAN.md, which commit 4ddb048f (Phase 43 planning, 2026-10-08) added without a trailing newline; GSD executor commits bypass commit-stage hooks, so the pre-commit workflow has been red on every push since bc57061b. Not a Phase 44 defect: every other hook (check-toml, check-yaml, large-files, merge-conflict, trailing-whitespace, gitleaks, shellcheck, cargo fmt) passed and cargo clippy --workspace --all-targets --all-features -D warnings exits 0 on HEAD."
  artifacts:
    - path: ".planning/phases/43-rate-pacing/43-02-PLAN.md"
      issue: "no newline at end of file (fixed in 1e93e1ce)"
    - path: ".planning/phases/43-rate-pacing/43-03-PLAN.md"
      issue: "no newline at end of file (fixed in 1e93e1ce)"
    - path: ".planning/phases/43-rate-pacing/43-05-PLAN.md"
      issue: "no newline at end of file (fixed in 1e93e1ce)"
  missing:
    - "Trailing newline on the three files (added in 1e93e1ce; pre-commit run --all-files now exits 0 locally with only clippy skipped, and clippy was verified green separately)"
  debug_session: ""
  evidence: "Local reproduction: pre-commit 4.6.2 run --all-files on 299ff07b -> 'fix end of files' Failed, all other hooks Passed; on 1e93e1ce -> all Passed. CI confirmed: pre-commit workflow run 38193792540 on fd3d686e concluded success (first green run on the branch since 1494e50e, 2026-10-08)."
  preliminary_findings: |
    The `pre-commit` workflow (pre-commit run --all-files) is red on every commit of claude/laughing-dirac-e0h2ax
    from bc57061b (2026-10-08 00:55 UTC, run 37710276127) through HEAD 299ff07b (run 38192977973); the last green
    run on the branch is 1494e50e (run 37706434327). main is green (df24689c, 2026-09-23). The only commit between the
    last green and first red runs is 4ddb048f "docs(43): create phase plan", which added
    .planning/phases/43-rate-pacing/43-02-PLAN.md, 43-03-PLAN.md and 43-05-PLAN.md without a trailing newline;
    the commit-stage `end-of-file-fixer` hook rejects exactly that. The failure therefore predates Phase 44
    (phase base b362d6cb, 2026-10-09) and is not a Campaign/Aegis defect. Locally on HEAD: cargo fmt --check clean.
    Job logs could not be fetched (GitHub serves them from a redirect host this session's gh does not contact);
    the job annotation only records "Process completed with exit code 1" at step "Run pre-commit".
