---
phase: 57-register-hygiene-close-out
reviewed: 2026-11-05T00:00:00Z
depth: standard
files_reviewed: 37
files_reviewed_list:
  - .cargo/audit.toml
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/src/platform/container/principal.rs
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/src/cadence.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/src/output/run_repository_port.rs
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-storage/src/cadence/redis.rs
  - crates/paladin-storage/src/lib.rs
  - crates/paladin-storage/src/minio.rs
  - crates/paladin-storage/src/node_cache/redis.rs
  - crates/paladin-storage/src/redis_fence.rs
  - crates/paladin-storage/src/run/contract_tests.rs
  - crates/paladin-storage/src/run/in_memory.rs
  - crates/paladin-storage/src/run/postgres.rs
  - crates/paladin-storage/src/run/sqlite.rs
  - crates/paladin-web/CHANGELOG.md
  - crates/paladin-web/openapi.json
  - crates/paladin-web/src/run_controller.rs
  - crates/paladin-web/src/thread_controller.rs
  - docs/src/api-reference/platform-api.md
  - docs/src/deployment-topologies/http-service-host.md
  - docs/src/user-guides/parley-and-chronicle.md
  - scripts/check-review-dispositions.sh
  - src/application/services/paladin/paladin_builder.rs
  - src/application/services/run/http_surface_tests.rs
  - src/application/services/run/submission.rs
  - src/application/services/run/webhook/service.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - src/application/services/treasurer/mod.rs
  - src/application/services/treasurer/tests.rs
  - src/config/treasurer.rs
  - src/infrastructure/web/run_api_wiring.rs
  - tests/integration/file_storage_integration_tests.rs
  - tests/scripts/check-review-dispositions_test.sh
findings:
  critical: 1
  warning: 2
  info: 5
  total: 8
status: issues_found
---

# Phase 57: Code Review Report

**Reviewed:** 2026-11-05
**Depth:** standard
**Files Reviewed:** 37
**Status:** issues_found

## Summary

Review covered the phase diff (`69de7c28..HEAD`) for each listed file, focused on changed hunks. No `cargo` commands were run (a background `cargo test` holds the target lock), so every finding comes from reading the code and git history.

Most of the phase's fixes hold up:

- `owner_run_for_thread` is consistent across the port default, the in-memory override and both SQL overrides.
- The fence marker TTL proof (`L + 30 min <= max(10L, 1 h)`) is correct.
- The Cadence spawn-and-await in `note_failure` is sound.
- Every `{}` of an S3 error in `minio.rs` now goes through `s3_err_summary`, either directly or through `execute_with_retry`. I checked all remaining raw `format!("... {}", e)` sites (lines 188, 492, 521, 544, 577, 640, 649, 681). In each, `e` is already a `FileStorageError` or a credentials error, so no response body can leak.
- Redact-before-truncate is respected in `bounded_run_error`.

One real correctness defect was found in the new WR-04 completion path in `worker.rs`. Two robustness or security-posture issues came up in the treasurer env override and in the disposition gate. The rest is documentation drift and minor items.

## Critical Issues

### CR-01: WR-04 completion/failure "nack and redeliver" loses the run's outcome when only the second write fails

**File:** `src/application/services/run/worker.rs:1814-1866` (completion), `src/application/services/run/worker.rs:2097-2133` (`persist_failure`); test gap at `src/application/services/run/worker_tests.rs` (`agent_completion_whose_outcome_write_fails_emits_no_terminal_event_and_nacks`)

**Issue:** Phase 57 turned the completion path's `?` into an explicit `nack` and documented that "the run is redelivered, so no terminal event is emitted for an outcome that never committed ... the attempt that commits emits the run's one terminal event". That invariant only holds when the first write (`update_status(Running -> Completed)`) fails.

The writes are ordered status first, outcome second. If `update_status` succeeds and `record_outcome` fails, the row is already `Completed` with `output: None`. The message is then nacked. On redelivery, `run_once` (worker.rs:1245-1252) reads the run, sees a terminal status and takes the `_ =>` arm: "A stale message for an already-terminal run: ack and drop". The consequences:

- The run's output is permanently lost.
- No terminal wire event is emitted by any attempt.
- No `completed` webhook is enqueued.

`persist_failure` has the same shape. `Failed` is written, then `record_outcome` stores the error text. A failing second write nacks, and redelivery drops the message. The error text is never persisted, and no `failed` webhook or `RunFinished` event is emitted.

`persist_agent_halt` already uses the safe order: `record_outcome` first, then `update_status`. The outcome is rewritable while the row is still `Running`. The new completion code and the existing failure code do not follow it. The new test for the outcome-write failure asserts only "no terminal event, one nack" and never checks the row. The sibling status-write test does assert `Running`. The data-loss state is therefore not covered by any assertion.

**Fix:** Write the outcome before the terminal status, as `persist_agent_halt` does. A failed outcome write then leaves the row `Running`, so the redelivery really does re-run the attempt that commits. Apply it to both the completion block and `persist_failure`.

```rust
// completion block: outcome first, status last
self.repository.record_outcome(&run.run_id, RunOutcomeRecord { /* ... */ }).await?;
self.repository
    .update_status(&run.run_id, RunStatus::Running, RunStatus::Completed, chrono::Utc::now())
    .await?;
```

Also assert `repo.get(&run_id).status == RunStatus::Running` in the outcome-failure test. Add an equivalent persist_failure test that fails `record_outcome` and asserts the row stays `Running`.

## Warnings

### WR-01: An empty `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` silently wipes a YAML-configured secret

**File:** `src/config/treasurer.rs:869-875` (test at `:1546-1581`)

**Issue:** The 41-REVIEW WR-05 fix treats an empty env value as "absent" by writing `webhook.secret = Some(v).filter(|s| !s.is_empty())`. Absent should mean "no override", so the YAML value should survive. This code assigns `None`, which discards a valid `secret: "from-yaml"`.

The new test encodes this: it sets the env var to `""` with a YAML secret present and asserts the result is `None`. The test's own doc comment says the goal is to not "shadow a configured secret".

A very common deployment accident triggers this: `APP_..._SECRET=${SECRET:-}` in compose or Helm, with the variable unset. The operator's configured secret is then dropped. Operator allowance notices are signed with the empty key, which any receiver can reproduce. Only a boot `warn!` marks the downgrade, and boot is never refused. The result is a security downgrade that is quiet and depends on the environment.

**Fix:** Treat an empty env value as no override:

```rust
if let Some(webhook) = self.allowance.webhook.as_mut()
    && let Some(v) = read_env::<String>("APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET")
    && !v.is_empty()
{
    webhook.secret = Some(v);
}
```

Update the test to expect `Some("from-yaml")` after an empty env var. Keep `webhook.secret.filter(!empty)` at the wiring layer for a YAML `secret: ""`.

### WR-02: `check-review-dispositions.sh` accepts non-evidence as a "closed" pointer

**File:** `scripts/check-review-dispositions.sh:142-184` (`COMMIT_RE`, `REQ_RE`, `PATH_RE`, `has_pointer`)

**Issue:** The gate exists to fail "looks dispositioned but is not". Its pointer heuristics are loose enough that prose passes:

- `COMMIT_RE` matches any 7-40 character hex-alphabet token containing a digit. A date such as `20261105`, a bare number such as `1234567`, or `deadbee1` all count as a commit id.
- `REQ_RE` accepts any `[A-Z]{2,}-[0-9]+` whose prefix is not WR/CR/IN. That covers `UTF-8`, `SHA-256`, `ADR-0063` and `HTTP-2`, none of which point at evidence.
- `PATH_RE` accepts any `a/b/c` with two or more slashes, so `and/or/else` or `read/write/delete` in prose qualifies.
- `TEST_RE` accepts any lowercase word with two underscores, such as `not_a_thing`.

A closed row such as "fixed on 20261105, see ADR-0063" therefore passes. The regression harness covers only a prose row with no pointer-shaped token at all (`closed_no_pointer`). It has no negative case for these near-misses, so the weakness is untested.

**Fix:** Tighten the patterns and add fixtures for each near-miss:

- Require a commit id to be 7-40 hex characters containing at least one letter and one digit, or require the word "commit" or "sha" adjacent to it.
- Drop `ADR`, `UTF`, `SHA` and `HTTP` from `REQ_RE`, or use an allowlist of real requirement prefixes (`TENANT`, `REG`, `ALLOW`, and so on).
- Require `PATH_RE` candidates to end in a known extension (`.rs`, `.md`, `.toml`, `.sh`, `.json`) or to be backticked.
- Add near-miss fixtures that must fire `CLOSED_NO_POINTER`.

## Info

### IN-01: Doc comment split across the wrong item in `worker.rs`

**File:** `src/application/services/run/worker.rs:139-148`

**Issue:** The new `RUN_ERROR_CHAR_BUDGET` and `bounded_run_error` items were inserted between an existing doc line and its item. The line "The `graph_fingerprint` an agent run's `TraceEvent::RunStarted` carries." (line 139) now sits on top of `RUN_ERROR_CHAR_BUDGET`'s docs. `AGENT_RUN_FINGERPRINT` lost its first sentence and now begins "A legacy `Runnable::Agent` run has no `WarGraph`...". The rendered rustdoc is wrong for both items.

**Fix:** Move line 139 down to sit directly above the `AGENT_RUN_FINGERPRINT` doc block, or fold it into that block's first sentence.

### IN-02: Stale "latest run" wording left behind by the D-02 rename

**File:** `crates/paladin-core/src/platform/container/principal.rs:14` (and the test doc at `:434-436`)

**Issue:** The module-level doc still says "a thread belongs to the tenant of its latest run", while `permits_thread` and every other doc now say "owner run (earliest attributed)". This is the module's headline description of the rule, so the stale text is the one readers hit first. The test-name note at `:434-436` is deliberate and fine.

**Fix:** Change line 14 to "its owner run (its earliest attributed run)".

### IN-03: `AdmissionSettlement::drop` can refund an admission for a run that then commits

**File:** `src/application/services/run/submission.rs:236-262`

**Issue:** On drop, the guard spawns a task that reads `repository.get(run_id)` and abandons the admission if no row exists. If the dropped future was mid-`insert`, the write may still land after that read, particularly on a networked store. The admission is then given back for a run that exists. The effect is a duplicate allowance notice, and the doc comment accepts "a possible second notice over a lost one". This is acceptable by design, but the residual is not listed among the documented windows in `treasurer/mod.rs`, which names only a process crash and a drop inside `admit`.

**Fix:** Add this race to the documented residuals in `treasurer/mod.rs` and in the `AdmissionSettlement` doc. No code change is required.

### IN-04: `s3_error_code` embeds up to 64 characters of server-controlled text, unsanitised

**File:** `crates/paladin-storage/src/minio.rs:107-112` (used by `s3_err_summary`)

**Issue:** The `<Code>` value comes from the S3 response body and is truncated to 64 characters but not filtered. A hostile or buggy endpoint can put newlines or control characters into it. The text lands in `FileStorageError` messages, in `log_operation` output and in health reports (log injection). The 64-character cap and the earlier redaction work make this low risk.

**Fix:** Keep only `[A-Za-z0-9_.-]` characters before the `take(64)`:

```rust
.map(|(code, _)| code.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')).take(64).collect())
```

### IN-05: Fence marker TTL cap reopens stale-writer overwrite for long-lived node-cache entries

**File:** `crates/paladin-storage/src/redis_fence.rs:49-62`, `crates/paladin-storage/src/node_cache/redis.rs:306-309`

**Issue:** The marker is clamped to at most 30 minutes, so that it never outlives the fencing counter. For an entry TTL above 15 minutes, the marker can now expire before the entry it guards. A stale lock holder (token 3) can then overwrite the entry written by a newer holder (token 4) after the marker is gone. The doc (ADR-0058 (g)) argues that the deterministic result makes this a harmless duplicate write, and the new live test covers the counter-expiry side. This is a consciously accepted trade-off rather than a defect. It does mean the fencing guarantee is now bounded to 30 minutes after the last write.

**Fix:** No change needed if ADR-0058 (g) holds. If any node can produce a non-deterministic result, record that exception in the ADR, since the guarantee silently weakens for those entries.

---

_Reviewed: 2026-11-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
